---
title: "feat: DKIM-sign login mails sent through the unibas relay"
type: feat
date: 2026-10-27
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dasch-specs
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dasch-specs
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7659
linear: DEV-7659
linear_project: 48e9fd8122fa
---

# feat: DKIM-sign login mails sent through the unibas relay

## Overview

The editor signs every login mail with DKIM (`d=dasch.swiss`, relaxed/relaxed) using the key and selector ops-deploy#1466 mounts in prod, and refuses to start in `PROD` with a relay but no signing configuration. The editor half of ops-deploy#1466; blocks DEV-7618 (prod deploy).

## Problem Statement / Motivation

Prod mail goes through `smtp.unibas.ch:25` (STARTTLS, no auth), which signs nothing and is not in `dasch.swiss`'s SPF (`~all`, measured with `dig +short TXT dasch.swiss`). DMARC is `p=quarantine; adkim=s; aspf=s` (`dig +short TXT _dmarc.dasch.swiss`), so DMARC rests on the editor's own signature: unsigned or broken-signature mail is quarantined, and nothing in the editor notices. The docs still describe the Google Workspace relay, which signed for us.

**This reverses a prior decision.** The v1 PRD (`dasch-specs/specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md`, Open Question 2) and the v1 plan (`03-feat-metadata-editor-v1-plan.md`, Phase 3) chose the Google Workspace relay: "a Uni relay cannot send as `dasch.swiss`, and a login code arriving from `dasch.unibas.ch` would read as suspicious to a depositor", with Google DKIM-signing relayed mail. What changed: the Google relay does not work for deposit (ops-deploy#1466), and the first reason stops holding once the editor signs itself, because DMARC passes on an aligned `d=dasch.swiss` signature whichever server relays the mail. The second reason is unaffected: `From:` stays `noreply@dasch.swiss`, and nothing a depositor sees names `unibas.ch`. Phase 1 records this in the PRD.

## Proposed Solution

- **identity-store** loads the key at startup (PKCS#8 or PKCS#1, RSA 1024 to 4096 bits), builds a `DkimConfig` with the `From:` domain as `d=`, and signs each message in `SmtpMailer::send` before the transport.
- **deposit-server** adds `dkim_private_key_file` / `dkim_selector` to `EditorConfig` (ops names, unchanged), validates them, and passes them through `IdentitySettings`.
- **Docs** move from the Google relay to the unibas relay plus editor-side DKIM.
- **dasch-specs**: the v1 PRD's open question on the relay asserts the opposite of what ships; it gets a resolution note.

## Technical Considerations

Every claim below about the issue's empirical premises was checked against lettre 0.11.23 and rsa 0.9.10 in `~/.cargo/registry`, `Cargo.lock` on `origin/main` `9ca1672b`, live DNS and OpenSSL 3.6.

- **lettre `dkim` feature** pulls `base64`, `sha2`, `rsa` 0.9, `ed25519-dalek`; `rsa 0.9.10`, `ed25519-dalek 2.2.0`, `pkcs1`, `pkcs8` are already in `Cargo.lock` (via `jsonwebtoken` → dsp-cli). Pure Rust, so the static musl image is unaffected. Licences (MIT/Apache, BSD-3) are in `deny.toml`'s allow list. `check-capability-deps.sh` does not restrict third-party crates for a `store` role.
- **PKCS#8 is required.** `DkimSigningKey::new` parses only PKCS#1 (`RsaPrivateKey::from_pkcs1_pem`, `lettre-0.11.23/src/message/dkim.rs:120`), and `openssl genrsa 2048` on OpenSSL 3.x writes `BEGIN PRIVATE KEY` (PKCS#8, reproduced locally). ops-deploy#1466 says its current keys are PKCS#1; both must load. Parse with `rsa` directly (add `rsa = "0.9"` to identity-store, the same major as lettre's, so no second copy), then hand lettre `to_pkcs1_pem`. Going through `rsa` for both formats gives one path where the size check applies.
- **Key size: 1024 to 4096 bits.** RFC 8301 §3.2: signers MUST use ≥ 1024; a shorter key signs and receivers report `dkim=fail`, the same silent quarantine, so startup refuses it. `rsa` refuses moduli above 4096 (`RsaPublicKey::MAX_SIZE`, `rsa-0.9.10/src/key.rs:174`), so the parse-failure message names that bound too. ops provides 2048.
- **Canonicalization relaxed/relaxed** via `DkimConfig::new` (`default_config` is simple/relaxed, `dkim.rs:164-184`), signing `From`, `To`, `Subject`, `Date`, `Message-ID`, `Content-Type`. lettre adds `Date` at build (`message/mod.rs:416`) but no `Message-ID` unless asked (`message/mod.rs:340`), and the editor sets none today. Google's sender guidelines require one ("Make sure every message includes a valid Message-ID", https://support.google.com/a/answer/81126), and Exchange Online's own signatures sign `Message-ID` and `Content-Type` (Microsoft's DKIM page, https://learn.microsoft.com/en-us/defender-office-365/email-authentication-dkim-configure). So the editor sets `Message-ID: <uuid@{from domain}>` (identity-store already depends on `uuid`) and signs it. Every signed header must be present, and lettre cannot oversign (`dkim.rs:154-155`). Relaxed headers survive Exchange Online re-folding; a body rewrite by Exchange would break `bh=` whatever the canonicalization, which only the post-ship check (H2) can show.
- **`d=` is derived, never configured:** `SmtpMailer.from.email.domain()`. Alignment with `From:` holds by construction, and `adkim=s` needs exactly `dasch.swiss`.
- **No `Debug` on anything that holds the key.** `rsa::RsaPrivateKey` derives `Debug` over `d` and `primes` (`rsa-0.9.10/src/key.rs:37`), and lettre's `DkimConfig` derives `Debug` over it. `SmtpMailer` already has no `Debug` (it holds credentials); keep it that way and say why at the new field.
- **Startup errors name the path, never the content.** The parser's error `Display` is not forwarded: a missing or unreadable file reports `io::ErrorKind`, and an unparseable one reports a fixed sentence. `Identity::start` failures are logged through `tracing::error!` (`areas/deposit/server/src/serve.rs:90`), which is the log pipeline (and OTLP in `DEV`). The path itself is echoed, so `validate` refuses a key-file value containing a newline or `PRIVATE KEY` (PEM pasted where the path belongs) without echoing it; `ConfigError` goes to stderr through `serve.rs:20`.
- **Selector syntax is validated** (RFC 6376 `selector = sub-domain *("." sub-domain)`: non-empty labels of `[A-Za-z0-9-]`). An empty value arrives as `Some("")`, and `docker stack deploy` does not strip quotes from `env_file` values (`ops-deploy/docs/learnings/best-practices/docker-stack-deploy-vs-compose-quirks.md`). figment strips double quotes itself (`figment-0.10.19/src/value/parse.rs:75`) but not single ones, so `''`, `'…'`, `_` or `a..b` would sign with a selector no DNS record answers. The selector is public (it is a DNS name), so the error may echo it. An all-digit selector (`20271027`) is parsed by figment as a number and fails config load loudly at startup; the docs say a selector must contain a non-digit, matching the dated `dasch-deposit-<env>-<year>` scheme.
- **`validate` checks presence, `mailer()` loads the file.** Config tests stay file-free; the key is read once, in `Identity::start`, before the listener binds.
- **Accepted risk: RUSTSEC-2023-0071 (Marvin)** affects every `rsa` 0.9 release and has no fix. It is a timing side channel on private-key operations. An attacker here can trigger a signature only through `POST /login` (per-IP limited, 500/day global cap) and sees only HTTP latency dominated by the SMTP round trip, so it is not exploitable in practice. lettre has no other RSA backend. Ed25519 DKIM (RFC 8463) is not verified by the major receivers on its own (Gmail `dkim=neutral`, Microsoft 365 `dkim=fail` in Red Sift's test, https://redsift.com/blog/ed25519-dkim-support-weak-keys, 2026-04-16), so RSA stays.

Repository bindings (`eng.yaml`): test `just test`, lint `just check`, gate `just check && just test`. Conventions: `CLAUDE.md`, `CONVENTIONS.md`, `areas/deposit/identity/CLAUDE.md`, `areas/deposit/server/CLAUDE.md`, `areas/deposit/editor/CLAUDE.md`. Reviewers for the phase reviews: `eng:review:security-reviewer` (eng.yaml for `areas/deposit/identity/**` and `areas/deposit/server/**`), `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`. eng.yaml also binds `accessibility-reviewer`, `ivan-reviewer` and an `eng:test-browser` offer to `areas/*/*/web/**`; the `identity-web` edits here (a settings field, a call argument) render no HTML, so neither applies.

**Delivery:** one draft PR in `dasch-specs` (Phase 1) and one PR in `dsp-repository` (Phase 2), one commit, with this plan's own commit squashed into it (`docs/src/git-conventions.md`, one-commit cap).

Key loading, in `areas/deposit/identity/store/src/mail.rs` (shape, not final code):

```rust
/// The signing key at `path`, in PKCS#8 (what `openssl genrsa` writes on OpenSSL 3) or PKCS#1.
///
/// No error carries the parser's message or the file's bytes: `Identity::start`'s failure is
/// logged (`serve.rs`), and the log is a pipeline other people read.
fn load_dkim_key(path: &Path) -> Result<DkimSigningKey, SmtpSetupError> {
    use lettre::message::dkim::{DkimSigningAlgorithm, DkimSigningKey};
    use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, LineEnding};
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;

    let refused = |reason: String| SmtpSetupError::DkimKey { path: path.to_path_buf(), reason };
    let pem = std::fs::read_to_string(path).map_err(|e| refused(e.kind().to_string()))?;
    let key = rsa::RsaPrivateKey::from_pkcs8_pem(&pem)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(&pem))
        .map_err(|_| refused("not an RSA private key of at most 4096 bits in PKCS#8 or PKCS#1 PEM".to_string()))?;
    // `n().bits()`, not `size() * 8`: `size()` rounds up to whole bytes, so a 1017-bit modulus
    // would pass as 1024.
    let bits = key.n().bits();
    if bits < 1024 {
        return Err(refused(format!("{bits}-bit key; DKIM needs at least 1024")));
    }
    let pkcs1 = key.to_pkcs1_pem(LineEnding::LF).map_err(|_| refused("could not re-encode the key".to_string()))?;
    DkimSigningKey::new(&pkcs1, DkimSigningAlgorithm::Rsa).map_err(|_| refused("lettre rejected the key".to_string()))
}
```

## Implementation Phases

#### Phase 1: Resolve the PRD's relay question

### dasch-specs

- [x] `specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md` Open Question 2 ("SMTP relay", which asserts a university relay cannot send as `dasch.swiss`): add a resolution note: prod relays through `smtp.unibas.ch:25` (STARTTLS, no auth), and the editor DKIM-signs as `d=dasch.swiss` (DEV-7659, ops-deploy#1466); bump `last_revised`.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Sign login mail, and refuse a production relay without signing

### dsp-repository

- [x] `areas/deposit/identity/store/Cargo.toml`: add `"dkim"` to lettre's features; add `rsa = "0.9"` with a comment (PKCS#8 parsing lettre lacks; same major as lettre's so one copy); add `rand = "0.8"` as a dev-dependency with a comment that `rsa` 0.9 takes `rand_core` 0.6, so it must not be aligned with `identity-core`'s `rand = "0.9"`; add `tempfile = "3"` as a dev-dependency (as `areas/access/server/Cargo.toml:43`; already in `Cargo.lock`).
- [x] `mail.rs`: `SmtpSetupError::DkimKey { path: PathBuf, reason: String }` with `#[error("EDITOR_DKIM_PRIVATE_KEY_FILE {}: {reason}", path.display())]`; one line in the `SmtpSetupError` doc that the DKIM path never forwards a parser's error, unlike the TLS setup case argued at `:95-106`.
- [x] `mail.rs`: `load_dkim_key` as sketched above.
- [x] `mail.rs`: `mailer()` and `SmtpMailer::new` take `dkim: Option<(&str, &Path)>` (selector, key file), mirroring `credentials`; `new` loads the key and builds `DkimConfig::new(selector, domain, key, headers, DkimCanonicalization { header: Relaxed, body: Relaxed })` with `headers = ["From", "To", "Subject", "Date", "Message-ID", "Content-Type"].map(HeaderName::new_from_ascii_str).to_vec()` (`DkimConfig::new` takes `Vec<HeaderName>`, `dkim.rs:187-193`); the `new` doc ("Does not connect", `:79-80`) also says it reads the key file; the field carries a comment that `DkimConfig` is `Debug` over the private key.
- [x] `mail.rs`: extract message building from `send` into `fn message(&self, mail: &Mail) -> Result<Message, MailError>`, which sets `Message-ID: <{uuid v4}@{from domain}>` and signs (`let mut message`, `message.sign(..)`) when a `DkimConfig` is present; `send` calls it.
- [x] `mail.rs`: `describe()` appends `DKIM d=<domain> s=<selector>` or `unsigned`, so the startup "mail transport ready" line shows signing state. `DkimConfig`'s `selector` and `domain` are private (`dkim.rs:149-151`), so `SmtpMailer` keeps the selector beside it; the domain comes from `from`.
- [x] `mail.rs` doc and comments: `SmtpMailer` doc ("The Google Workspace relay") and the 587/`smtp-relay.gmail.com` STARTTLS comment describe a generic relay (prod: `smtp.unibas.ch:25`).
- [x] `mail.rs` tests (generate the key once per test binary with `RsaPrivateKey::new(&mut rand::thread_rng(), 1024)` in a `OnceLock`; write PEMs to `tempfile::NamedTempFile`; async, since the transport needs a reactor to drop): `test_a_pkcs8_key_signs_as_the_from_domain_with_relaxed_canonicalization` asserts `message(...).formatted()` carries exactly one `DKIM-Signature`, with `d=dasch.swiss`, `s=<selector>`, `c=relaxed/relaxed` and `h=from:to:subject:date:message-id:content-type`, and a `Message-ID` ending `@dasch.swiss>`; unfold with `.replace("\r\n ", " ")` first, since lettre folds the header.
- [x] `mail.rs` test: `test_a_pkcs1_key_loads`.
- [x] `mail.rs:173` `test_an_invalid_from_address_stops_startup` and `:186` `test_a_configured_relay_describes_itself_without_credentials`: pass the new `None` DKIM argument.
- [x] `mail.rs` test: `test_an_unusable_key_stops_startup_naming_the_path_never_the_content`: missing file, a fake `BEGIN PRIVATE KEY` PEM containing a marker string, and a 512-bit key each give `SmtpSetupError::DkimKey`; the rendered error contains the path and neither the marker nor `PRIVATE KEY`.
- [x] `mail.rs` test: extend `test_a_configured_relay_describes_itself_without_credentials` (or add one) so a signing mailer's `describe()` names `d=` and `s=`, and an unsigned one says `unsigned`.
- [x] `areas/deposit/identity/web/src/lib.rs`: `IdentitySettings.dkim: Option<(String, PathBuf)>` (selector, key file), passed to `mailer()` in `Identity::start`.
- [x] `areas/deposit/identity/web/src/handlers/login.rs:2247`: the console `mailer(...)` call gets the new `None` argument.
- [x] `areas/deposit/server/src/config.rs`: fields `dkim_private_key_file: Option<PathBuf>` and `dkim_selector: Option<String>` (env `EDITOR_DKIM_PRIVATE_KEY_FILE`, `EDITOR_DKIM_SELECTOR`), default `None`, documented like the SMTP fields; accessor `dkim()` returning both or `None`, like `smtp_credentials()`.
- [x] `config.rs` `validate`: selector and key file set together or not at all (same shape and message style as the username/password rule at `:425`).
- [x] `config.rs` `validate`: a set selector must match RFC 6376 selector syntax; the error echoes the value (public DNS name).
- [x] `config.rs` `validate`: a key-file value containing a newline or `PRIVATE KEY` is refused with a fixed message that does not echo the value.
- [x] `config.rs` `validate`: DKIM configured without `EDITOR_SMTP_HOST` is refused as dead configuration (the console transport ignores it), mirroring the break-glass rule at `:449`.
- [x] `config.rs` `validate`: `env == "PROD"` with `smtp_host` set and no DKIM configuration is refused, naming both variables and the reason (unsigned mail through this relay is quarantined under `dasch.swiss`'s DMARC). Placed after the PROD-requires-a-relay rule (`:441`), with a comment in that rule's register.
- [x] `config.rs` comments: `smtp_port` (drop "what `smtp-relay.gmail.com` speaks"; 587 stays the default, prod sets 25), `smtp_from` (drop the Workspace-DKIM reasoning; the `From:` domain is now the signing domain), `mail_daily_cap` (drop Google's 10,000/day; state the cap bounds abuse of a shared relay whose limit is not published to us).
- [x] `config.rs` tests: `env_env_override_switches_otlp_log_export`, `toml_file_overrides_defaults_and_env_overrides_toml` and `production_with_a_relay_loads` set a selector and key path, since PROD with a relay now requires them.
- [x] New `config.rs` tests follow `CONVENTIONS.md:26` (`test_` prefix), as the file's newer tests do; existing names stay.
- [x] `config.rs` test: `test_production_with_a_relay_but_no_dkim_is_refused` (error names `EDITOR_DKIM_SELECTOR` and `EDITOR_DKIM_PRIVATE_KEY_FILE`).
- [x] `config.rs` test: `test_half_a_dkim_configuration_is_refused`.
- [x] `config.rs` test: `test_a_dkim_selector_that_is_not_a_dns_name_is_refused` (empty, single-quoted `'dasch-deposit-prod-2026'`, `dasch_deposit`, `a..b`).
- [x] `config.rs` test: `test_a_pem_in_the_key_file_variable_is_refused_without_echoing_it` (a PEM block as the value; the error contains neither the marker inside it nor `BEGIN`).
- [x] `config.rs` test: `test_dkim_without_a_relay_is_refused_as_a_misunderstanding`.
- [x] `config.rs:42` `Secret` deserializer doc: drop the "Google Workspace app password" example (the relay has no password now); keep the argument.
- [x] `areas/deposit/server/src/identity.rs`: map `dkim()` into `IdentitySettings.dkim`; a test asserts a configured selector and key path arrive in the settings (a dropped mapping would ship unsigned mail with every other test green).
- [x] `docs/src/editor/authentication.md` "Mail": lettre over STARTTLS to `smtp.unibas.ch`, the editor DKIM-signs as `d=dasch.swiss`; replace the "a university relay cannot send as `dasch.swiss`" reasoning (it can, because the signature carries DMARC).
- [x] `authentication.md` "Relay prerequisites": DKIM is now the editor's (key and selector from ops-deploy, TXT record at `<selector>._domainkey.dasch.swiss`), SPF deliberately does not cover the relay (DMARC rests on DKIM), and the Google 10,000/day quota line is removed.
- [x] `authentication.md`: the control table row (`:41`) and the send-caps paragraph (`:71`) stop citing the Workspace quota.
- [x] `authentication.md`: one paragraph on why startup refuses PROD with a relay and no DKIM, and why key errors name the path only.
- [x] `docs/src/editor/operations.md` env table: `EDITOR_SMTP_PORT` row (default 587, prod 25), `EDITOR_SMTP_FROM` row (its domain is the DKIM `d=`), `EDITOR_MAIL_DAILY_CAP` row (no Google figure).
- [x] `operations.md` env table: new `EDITOR_DKIM_SELECTOR` and `EDITOR_DKIM_PRIVATE_KEY_FILE` rows (PKCS#8 or PKCS#1, RSA 1024 to 4096 bits; selector must contain a non-digit; required in PROD with a relay, refused without one).
- [x] `operations.md` `:54` `EDITOR_ENV` row: add the PROD-requires-DKIM rule next to PROD-requires-relay.
- [x] `operations.md` `:93`: "it waits for a mail relay (DEV-7618)" says the relay is settled and prod waits on DEV-7618's deploy.
- [x] `operations.md` `:99` "Not yet in place, for prod": rewrite to the current state (relay and key provisioned by ops-deploy#1466, no credentials).
- [x] `operations.md`: key rotation in two sentences (publish the new dated selector's TXT record, then switch key and selector, then retire the old record), pointing at ops-deploy#1466 for the procedure; after each rotation, check one received login mail for `dkim=pass`, since a mismatched key signs and starts without error.
- [x] `areas/deposit/identity/CLAUDE.md` "Configuration refuses unsafe combinations at startup": "Two that bite in test setups" (`:17`) becomes three: PROD with `EDITOR_SMTP_HOST` requires `EDITOR_DKIM_SELECTOR` and `EDITOR_DKIM_PRIVATE_KEY_FILE`.
- [x] `.github/workflows/cloud-run-editor-pull-request.yml:103` comment: "shared Workspace relay" becomes the shared relay.
- [x] `ARCH-MAP.md:412`: identity-store's third-party crates gain `rsa`.
- [x] Grep `gmail|Google Workspace|Workspace relay|smtp-relay|10,000` across `areas/deposit`, `docs/src`, `.github` and `ARCH-MAP.md`: no hit left that describes the relay (test fixture host strings may stay).
- [x] Run `just check && just test`; it passes.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

**Phase 1 outcome:** PRD note landed (dasch-specs `322e520`; the relay item sits under Infra Dependencies, not Open Questions). Deferred to Phase 2: none. Left for a follow-up: the v1 plan (`03-feat-metadata-editor-v1-plan.md`) still specifies the Google relay, and no PRD REQ names editor-side DKIM.

**Phase 2 outcome:** signing, config rules and docs landed; the review added an explicit 4096-bit upper bound (rsa enforces `MAX_SIZE` on public keys only) and a length cap on the key-file value. Deferred: none.

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Post on ops-deploy#1466 that `EDITOR_DKIM_PRIVATE_KEY_FILE`, `EDITOR_DKIM_SELECTOR` and `/run/secrets/dkim_private_key.pem` are final as-is, that no domain variable is needed, and that both PKCS#8 and PKCS#1 keys load (the agent drafts the text to the workspace root) | Marcin | after ship | Replies to people are drafted, never posted by the agent |
| H2 | On the first prod deploy (DEV-7618), request a login code to a mailbox outside `dasch.swiss` and confirm `Authentication-Results` shows `dkim=pass` and `dmarc=pass` for `d=dasch.swiss`, and that the STARTTLS certificate of `smtp.unibas.ch` validated against `webpki-roots` (no `Unreachable` in the log) | Marcin | after ship | Needs the prod deployment and a real inbox |
| H3 | Merge the dasch-specs PRD amendment PR | Marcin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] A login mail sent through a configured relay carries a `Message-ID` in the `From:` domain and one `DKIM-Signature` with `d=` equal to `EDITOR_SMTP_FROM`'s domain, `s=` equal to `EDITOR_DKIM_SELECTOR`, `c=relaxed/relaxed`, signing `from:to:subject:date:message-id:content-type`.
- [x] A PKCS#8 and a PKCS#1 RSA key both load.
- [x] A missing, unparseable, under-1024-bit or over-4096-bit key stops startup with an error naming the path and containing no key material.
- [x] `EDITOR_ENV=PROD` with `EDITOR_SMTP_HOST` and without DKIM configuration refuses to start; `DEV` starts without it; half a DKIM configuration, DKIM without a relay, a non-DNS selector or a PEM in the key-file variable is refused in any environment, the last without echoing it.
- [x] The startup "mail transport ready" line states `d=`/`s=` or `unsigned`.
- [x] No doc or comment under `areas/deposit` or `docs/src` describes the Google relay as the editor's relay.

## Dependencies & Risks

- **ops-deploy#1466** provides key, selector and relay settings in prod; it is open, and the editor already ignores unknown `EDITOR_*` keys, so merge order is free. Nothing deploys prod yet (DEV-7618), so the new PROD rule breaks no running deployment: stage runs throwaway with `EDITOR_ENV=DEV` and no DKIM variables (`ops-deploy/deposit.yml:13-33` on `origin/main`), and the PR preview (`cloud-run-editor-pull-request.yml:112`), `just run-docker-editor` (`justfile:752`) and the E2E suite run as `DEV`.
- **`EDITOR_ENV` is compared case-sensitively** (`config.rs:441`): `prod` bypasses every PROD rule, this one included. Pre-existing and out of scope; worth its own issue.
- **DNS:** `dasch-deposit-prod-2026._domainkey.dasch.swiss` is published (`dig +short TXT dasch-deposit-prod-2026._domainkey.dasch.swiss` returns a `v=DKIM1; k=rsa` record), contrary to the issue's 2026-10-26 note. H2 confirms it matches the mounted key.
- **Exchange Online body rewriting** (disclaimer, re-encoding) would break `bh=`; only H2 shows it. Microsoft documents no behaviour for relayed mail that already carries a third-party signature; the one reported breakage (Microsoft Q&A, 2017) concerned multipart MIME boundaries, and login mail is single-part `text/plain`. If it fails, `Authentication-Results` names `body hash did not verify`, and the fix is on the relay side, not canonicalization.
- **Marvin (RUSTSEC-2023-0071):** accepted, see Technical Considerations. `cargo deny` is installed by `just` but runs in no workflow, so no gate changes.

## Success Metrics

- H2's first prod login mail: `dkim=pass`, `dmarc=pass`, `d=dasch.swiss`.
- DMARC aggregate reports (dmarcian, `rua` in `_dmarc.dasch.swiss`) show deposit mail from the unibas relay passing DKIM alignment.

## References

- Issue: [DEV-7659](https://linear.app/dasch/issue/DEV-7659); blocks [DEV-7618](https://linear.app/dasch/issue/DEV-7618); ops side: [ops-deploy#1466](https://github.com/dasch-swiss/ops-deploy/pull/1466).
- Code (origin/main `9ca1672b`): `areas/deposit/identity/store/src/mail.rs` (`mailer`, `SmtpMailer::new`, `send`), `areas/deposit/server/src/config.rs:334` (`validate`), `:425` (credential pair rule), `:441` (PROD requires relay), `:523` (`smtp_credentials`), `areas/deposit/server/src/identity.rs`, `areas/deposit/identity/web/src/lib.rs:32` (`IdentitySettings`), `:115` (`mailer` call).
- lettre 0.11.23 `src/message/dkim.rs` (`DkimSigningKey::new` PKCS#1 only, `DkimConfig::new`, `default_config`); `Message::sign` (`src/message/mod.rs:600`).
- RFC 6376 (DKIM), RFC 8301 (key size), RFC 8463 (Ed25519 DKIM).
- Learnings: `ops-deploy/docs/learnings/best-practices/docker-stack-deploy-vs-compose-quirks.md` (quoted `env_file` values).
- PRD: dasch-specs `specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md` (REQ-6.7, Open Question 2).
