---
plan: docs/specs/2026-10-27-editor-dkim-signing/01-feat-editor-dkim-signing-plan.md
target_repo: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dasch-specs
base_commit: 634f94f
branch: specs/dev-7659-editor-dkim-signing
started: 2026-10-27
problem: >
  Prod login mail goes through smtp.unibas.ch:25 (STARTTLS, no auth), which signs nothing and is not in dasch.swiss's SPF. With DMARC p=quarantine; adkim=s; aspf=s, DMARC rests on a signature the editor does not make, so every login mail is quarantined and nothing in the editor notices. The editor must DKIM-sign as d=dasch.swiss with the key and selector ops-deploy#1466 mounts, and refuse to start in PROD with a relay but no signing configuration. The docs and the v1 PRD still describe the Google Workspace relay.
status: complete
---

# Execution Journal: 01-feat-editor-dkim-signing-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dasch-specs | 634f94f | specs/dev-7659-editor-dkim-signing | squash | in-progress | — |
| dsp-repository | 9ca1672b | feature/dev-7659-editor-dkim-sign-login-mails-sent-through-the-unibas-relay | rebase | in-progress | — |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dasch-specs@634f94f | 1 |
| 2 | reviewed | dsp-repository@5bb8d714 | 1 |

## Chunk queue

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dasch-specs | specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md | — | "PRD Open Question 2 … resolution note; bump last_revised" | the relay item records the unibas relay plus editor-side DKIM; last_revised 2026-10-27 | PRD § Infra Dependencies item 2 "SMTP relay" (the plan's "Open Question 2" label is stale) | — |
| 2.1 | dsp-repository | areas/deposit/server/src/config.rs | — | config.rs fields+dkim() accessor; 5 validate rules; smtp_port/smtp_from/mail_daily_cap/Secret comments; 3 existing tests gain DKIM; 5 new test_ tests | PROD+relay without DKIM, half config, bad selector, PEM in path, DKIM without relay all refused; `cargo test -p deposit-server config` green | config.rs:180-200 (smtp fields), :241 (cap), :42 (Secret doc), :425/:441/:449 (validate rules), :523 (smtp_credentials), tests :678/:692/:851; owner: docs/src/editor/operations.md env table | — |
| 2.2 | dsp-repository | docs/src/editor/authentication.md, docs/src/editor/operations.md, areas/deposit/identity/CLAUDE.md, .github/workflows/cloud-run-editor-pull-request.yml, ARCH-MAP.md | — | authentication.md x4, operations.md x6, identity CLAUDE.md, workflow comment, ARCH-MAP rsa | docs describe unibas relay + editor DKIM; no Google relay text left | authentication.md:41,:71,:201-210; operations.md:54,:60,:68,:93,:99; identity CLAUDE.md:17; workflow :103; ARCH-MAP.md:412 | — |
| 2.3 | dsp-repository | areas/deposit/identity/store/Cargo.toml, areas/deposit/identity/store/src/mail.rs, areas/deposit/identity/web/src/lib.rs, areas/deposit/identity/web/src/handlers/login.rs, areas/deposit/server/src/identity.rs | 2.1 | Cargo deps; SmtpSetupError::DkimKey; load_dkim_key; mailer/new dkim arg; message() with Message-ID + sign; describe(); doc comments; 5 mail.rs tests; IdentitySettings.dkim; login.rs None; identity.rs mapping + test | signed message carries one DKIM-Signature d=dasch.swiss c=relaxed/relaxed h=from:to:subject:date:message-id:content-type; PKCS#1 and #8 load; key errors name path only | mail.rs:22-32 mailer, :60-121 SmtpMailer/new, :140-159 send/describe, tests :162-196; lib.rs:32-48, :115; login.rs:2247; identity.rs:10-30; owner: plan Technical Considerations sketch | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dasch-specs | complete | 195f2b5 | PRD Infra Dependencies item 2 gains a resolution note (unibas relay, editor DKIM d=dasch.swiss); last_revised bumped | none |
| 2.2 | dsp-repository | complete | dd12850e | authentication.md, operations.md, deployment.md, identity CLAUDE.md, PR-preview workflow comment and ARCH-MAP move to the unibas relay + editor DKIM | none |
| 2.1 | dsp-repository | complete | 4ab66c01 | EditorConfig gains dkim_selector/dkim_private_key_file, dkim() accessor and five validate rules (PROD+relay needs DKIM, pair, selector syntax, PEM-in-path unechoed, no DKIM without relay); Google comments dropped; 7 new tests | none |
| 2.3 | dsp-repository | complete | 6d09efc5 | identity-store loads PKCS#8/PKCS#1 keys (1024-4096 bits), sets Message-ID and DKIM-signs relaxed/relaxed as the From domain; describe() states signing; IdentitySettings.dkim mapped from config; signing test mutation-checked | none |

## Deferrals

- eng:test-browser (eng.yaml `areas/*/*/web/**`): skipped at intake by the user; Phase 2's identity-web edits render no HTML.

- Leftover (Phase 1 review): v1 plan `03-feat-metadata-editor-v1-plan.md` still specifies the Google relay (`:211`, `:215`, `:464`, `:473`); not rewritten, it is the record this plan reverses.
- Leftover (Phase 1 review): no PRD REQ names editor-side DKIM; adding one is a PRD amendment for the owner.

- Leftover (Phase 2 review, DUNE-001): ARCH-MAP.md identity/server fingerprints not refreshed; run `/dune:dune-map update` separately.
- Leftover (Phase 2 review): RUSTSEC-2023-0071 accepted only in the plan; `deny.toml` has no ignore entry (pre-existing: rsa already in the lockfile via jsonwebtoken, cargo deny in no workflow).

## Deviations

- none

## Side findings

- The plan names the PRD's relay item "Open Question 2"; on dasch-specs origin/main it is Infra Dependencies item 2. Edited by content.
- Phase 1 review fix amended into the PRD commit: 195f2b5 became 322e520 (note trimmed, "no credentials" stated).
- (2.2) `just docs-test` fails on untagged code blocks in git-conventions.md, observability.md and operations.md "The startup comparison", all outside this diff; pre-existing.
- (2.1) Session tightened two new tests that could pass vacuously (selector and PEM tests matched the variable name, which the pair-rule message also contains); PROD-DKIM rule mutation-checked.
- (Phase 2 review) The plan's claim that rsa refuses moduli above 4096 holds for public keys only: `check_public` passes no max size (rsa-0.9.10 src/key.rs:495), so an 8192-bit private key loaded. Fixed with an explicit `bits > 4096` guard; its test generates a 4104-bit key (20-60 s in debug) and is `#[ignore]`d, mutation-checked.
- (Phase 2 review) A key body pasted as the path without PEM lines passed validate and would have been echoed by `DkimKey`; validate now also refuses values over 255 bytes (every RSA >=1024-bit key body is longer), mutation-checked.
- (gate) `just test` inside the sandbox fails in dsp-cli's wiremock tests (local port binding denied); the gate runs unsandboxed.

## Closeout

- root_cause: Prod mail moved from the Google Workspace relay, which DKIM-signed for dasch.swiss, to smtp.unibas.ch, which signs nothing and is outside dasch.swiss's SPF; with DMARC p=quarantine, adkim=s, aspf=s, DMARC could only pass on a signature the editor makes, and it made none.
- investigation: lettre's DkimSigningKey parses PKCS#1 only while OpenSSL 3 writes PKCS#8, so keys go through rsa and are re-encoded. The plan assumed rsa caps private keys at 4096 bits; review found MAX_SIZE applies to public keys only, so the bound is now explicit. A one-line base64 key pasted as the path passed the PEM-marker check and would have been logged by the path-naming error; a length cap closes it. The v1 PRD's relay item was under Infra Dependencies, not Open Questions as the plan said.
- solution: identity-store loads the key (PKCS#8 or PKCS#1, 1024-4096 bits), sets Message-ID in the From domain and signs relaxed/relaxed over From, To, Subject, Date, Message-ID, Content-Type with d= derived from From; describe() states signing. deposit-server adds EDITOR_DKIM_SELECTOR/EDITOR_DKIM_PRIVATE_KEY_FILE and refuses PROD with a relay but no DKIM, half a pair, a non-DNS selector, a key pasted as the path, and DKIM without a relay. Docs move to the unibas relay; the dasch-specs PRD records the reversal.
- prevention: signing test asserts d=, s=, c=, h= and Message-ID (mutation-checked); key-error test asserts path-only rendering per failure reason; one config test per rule; the identity.rs mapping test guards against silently unsigned mail. Anti-pattern: trusting a library constant's name (MAX_SIZE) without reading where it is applied. Not verifiable here: dkim=pass end to end (H2, first prod deploy).
