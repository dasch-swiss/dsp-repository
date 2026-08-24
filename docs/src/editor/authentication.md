# Editor Authentication

Login is an email one-time code. This page records the design, and — first — the standards deviation it rests on, because OWASP ASVS 6.1.1 and 6.3.3 require a deviation and its rationale to be written down. Without this page the design reads as compliant when it is not.

## Accepted deviation: email as an authentication mechanism

**Two standards prohibit this outright.**

NIST SP 800-63B-4 §3.1.3.1: *"Email SHALL NOT be used for out-of-band authentication because it may be vulnerable to: Access using only a password; Interception in transit or at intermediate mail servers; Rerouting attacks…"*

OWASP ASVS 5.0 V6.6: *"Unsafe out-of-band authentication mechanisms such as e-mail and VOIP are not permitted."*

**The carve-out does not cover us.** NIST exempts codes sent to *verify an address* and codes used for *account recovery*. This is neither: it is the login.

**Why it was accepted anyway.** The editor serves roughly thirty people — RDU staff and depositing project teams at institutions across Switzerland. The alternatives were weighed and each costs more than it buys at this size: passwords need a reset channel, which is email again, plus storage, rotation and breach handling; WebAuthn needs enrolment and a recovery path for a lost authenticator, for users who touch the tool a few times a year; federated identity has no single provider these institutions share. What is being protected is project metadata that is public once approved, held for at most one review cycle, in a service that holds no credential to anything else — it cannot write to `dsp-repository`, which is why the collection endpoint exists.

**What the deviation actually exposes.** Anyone who can read a user's mailbox, or intercept mail in transit or at a relay, can obtain a live code. Browser binding (below) removes most of the value of holding one, but it does not remove all of it, and it does nothing against an attacker who also controls a browser the user will act in.

**When to revisit.** If the editor gains write access to anything outside itself, holds unpublished data of real sensitivity, or grows past a population where enrolment is a conversation rather than a project, this decision should be reopened rather than inherited.

## The flow

```text
POST /login        address → a code is sent, and the browser is bound to it
GET  /login/code   the code form
POST /login/code   code → session
POST /logout       session deleted, cookie cleared
```

There is no resend endpoint. Asking again is another `POST /login`, governed by the same cooldown, which keeps the number of endpoints that send mail at one.

### What each control defends

| Control | What it stops |
|---------|---------------|
| Identical response for every outcome (REQ-6.2) | Learning which addresses have accounts |
| Browser binding (`login_codes.browser_token`) | A code read from the mailbox by someone else |
| Three strikes per code (REQ-6.4) | Guessing against one code |
| Account counter surviving resend | Guessing across codes — see below |
| Single use (`consumed_at`) | Replay |
| Global daily send cap | Exhausting the shared relay quota, which locks out everyone (partially — see below) |
| Per-IP rate limit | One host driving either endpoint flat out |
| Fresh session id on login | Fixation |
| `Sec-Fetch-Site: same-origin` | CSRF from any other `*.dasch.swiss` host |

### Anti-enumeration is a property of the response, not a branch

Every `POST /login` answers with the same status and the same `Location`, whether the address is known, unknown, inside its cooldown, throttled, or blocked by the daily cap. Only the mail differs, and only the address's owner sees that.

The part that is easy to get wrong is the cookie, and the first version of this got it wrong. It set a fresh binding only when a code had been issued *or* the browser presented none, reasoning that a browser already holding a binding should be left alone. But the presented cookie is attacker-supplied — any non-empty value is accepted, because at that point it is just a token to look up — so a request carrying an invented cookie was answered with `Set-Cookie` for an address with an account and without one for an address without. One request per address, no timing needed.

The rule is therefore simpler and absolute: **a freshly minted token is returned on every request**, whatever the outcome. When no code was issued, a binding the browser already holds is *moved* onto the new token, so the code it owns stays spendable; when it holds nothing, the new token binds to nothing. The move is authorised by holding the old token, so it cannot be used to acquire the binding of a code on its way to somebody else. Every branch produces a byte-identical response.

**A known limit: timing.** The response is identical; the work behind it is not. A request for an address with an account stores a row and hands a message to the relay, where one for an address without returns almost immediately. In production that is tens to hundreds of milliseconds, which is measurable over a network — so an attacker with a list of candidate addresses can still separate them, REQ-6.2's identical *response* notwithstanding.

This is not closed here, and it is stated rather than left implied because a page like this one is worthless if it overclaims. The fix is to hand the send to a background task so the response returns before the relay is contacted, which collapses the difference to a single database write — at the cost of a window in which a process stopped between storing a code and sending it leaves a user waiting out a cooldown for a code that was never sent. That trade deserves to be made deliberately rather than as a side effect. Until it is: the per-IP limit bounds probing, the population is about thirty people at largely guessable addresses, and what enumeration buys is knowing which of a small set of guessable addresses has an account.

The same reasoning is why a failed send is not reported to the user. REQ-6.9 asks for a generic failure report; REQ-6.2 requires the response not to vary. Where they conflict, REQ-6.2 wins: telling the user "we could not send your code" tells an attacker the address exists. The failure is reported in the log instead.

### Browser binding, and what it is not

`POST /login` sets `__Host-editor_login`, an opaque 256-bit token stored on the code row. Only a browser presenting that token can spend the code. It is set for **every** request, including for addresses with no account, where it binds to nothing — otherwise its presence would itself be the enumeration oracle.

This defends interception, which is NIST's stated objection. It does **not** defend attacker-initiated social engineering: an attacker who starts the login holds the binding, so talking a victim into reading a code out still works. Nothing here claims otherwise.

An outstanding code's binding is never handed to a second browser. A browser inside the cooldown that does not already hold the binding cannot verify, and waits the cooldown out. That is deliberate: returning the live binding to whoever posts the address would let anyone acquire the binding for a code already on its way to its owner.

### What the send cap does not yet do

Two gaps, both tracked in [DEV-7023](https://linear.app/dasch/issue/DEV-7023/editor-make-the-login-code-send-cap-countable-and-cap-it-per-account) and both deliberate rather than overlooked.

The cap is **global only**. With a 60-second cooldown one address can be sent 1,440 codes a day against a default cap of 500, so a single attacker with a single known address can exhaust the shared budget in about eight hours and stop everyone — RDU included — from signing in. A per-account cap closes it.

The cap also **counts rows rather than sends**. `count_issued_since` counts live `login_codes` rows, which is an inference: codes rolled back after a failed delivery vanish (correctly, nothing was sent) and a user's unspent codes are deleted when they sign in (incorrectly, those were mailed). An append-only send counter is the honest form.

The code just spent is deliberately **not** deleted on sign-in, which an earlier version did. That row is the resend cooldown's only anchor — REQ-6.5 measures from the last code *issued* — so deleting it let a user sign in and be sent another code immediately, and made every completed sign-in invisible to the cap.

### Throttling, and why it is time-based

REQ-6.4's three strikes are per *code*, so each resend hands out a fresh budget. At a sixty-second cooldown that is about 4,320 guesses a day against one address, which is roughly a 12% chance of hitting a six-digit code within a month. NIST SP 800-63B-4 addresses this directly: *"Generating a new authentication secret SHALL NOT reset the failed authentication count."*

So the counter lives on the account (`users.failed_logins`) and survives invalidation and resend. It clears only on a successful authentication — which means an account at the cap can never clear it by succeeding, because it cannot succeed. A latch would therefore be a permanent lock needing an unlock control that does not exist. Instead `users.failed_login_at` records the most recent failure and the account is refused for `EDITOR_LOGIN_LOCKOUT_SECS` after it.

The counter also **decays over that window**. Without the decay it is a ratchet: once an account has reached the cap, a single wrong entry after each lockout expires re-locks it for another full window, so an attacker keeps any address they know is registered permanently locked out for one request per window — and mails its owner an unrequested code each time. A failure whose predecessor has aged out starts the count at one, so re-locking costs a fresh budget. NIST is not in the way: it requires that generating a new *secret* not reset the count, and says nothing about an elapsed throttling window.

The three strikes are claimed atomically, in the same statement that increments them (`UPDATE … WHERE attempts < 3`). Reading the count and incrementing it afterwards leaves a window in which every simultaneous submission passes the check — twenty parallel guesses against one code cost three strikes' worth of budget and got twenty comparisons, which is the guessing defence for a sub-20-bit secret failing open.

A throttled account is told nothing about it. The message for every code-entry failure is the same, because a token that resolves to a live code only exists for an address that has an account — so "too many attempts" would confirm the address to anyone willing to spend ten guesses. The cost is real: a throttled user sees only "that code is not valid". RDU diagnoses it from the auth log and from the per-account "last code issued at", never from an address in a log.

### When a correct code buys nothing

Consuming the code and creating the session are two writes, and the second can fail — a busy writer, a pool checkout timeout. Left alone that spends the code for nothing: the user is told it was invalid, re-entering it fails, and the cooldown refuses them another, so an error that was never theirs locks them out for up to a cooldown. The code is therefore reopened when session creation fails. Nobody was authenticated, so it costs nothing.

### Expired rows

Codes and sessions are deleted wherever the flow trips over them, but a code nobody ever entered is tripped over by nothing. An hourly sweep removes both, because otherwise every six-digit code ever issued stays in the table in plaintext for the life of the database.

### Cookies

Two, both `__Host-` prefixed, `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/`:

- `__Host-editor_login` — the pre-auth binding, living as long as the code (ten minutes).
- `__Host-editor_session` — the session, living the absolute session lifetime.

`SameSite=Lax` rather than `Strict` per REQ-6.3: `Strict` drops the cookie on the first navigation *into* the editor from a link in mail or chat, so the user would arrive signed out for no security gain that `Sec-Fetch-Site` does not already provide.

The `__Host-` prefix stops a sibling host *setting* a cookie of the same name that ours could not be told apart from. It does not stop a sibling host *triggering a request that carries* the real one — see [Architecture](./architecture.md#relationship-to-dpe) for why the editor has its own origin, and the CSRF middleware for what closes that.

### CSRF

Every non-`GET`/`HEAD` request must carry `Sec-Fetch-Site: same-origin`, and fails closed on `same-site`, `cross-site`, `none` and an absent header.

`SameSite` cannot do this job: it is scoped to the registrable domain, so a request from any other `*.dasch.swiss` host counts as same-site and carries the cookie. `Sec-Fetch-Site` distinguishes same-origin from same-site, is a forbidden request header so page script cannot spoof it, and is named by ASVS 3.5.3. `Datastar-Request` is not a substitute: the no-JavaScript path is a plain form post, which does not send it.

The consequence of failing closed on absent is that a browser too old to send `Sec-Fetch-*` cannot use the editor. That is accepted: treating absent as permissive would hand over the bypass, because a cross-site post from such a browser is indistinguishable from a legitimate one.

`GET` and `HEAD` are exempt, because a navigation from anywhere is a `GET`. That is only sound while no `GET` handler changes state, which is why every state-changing route is a `POST` and the router tests assert each refuses `GET`. The one deliberate exception is the session's idle-timeout touch, which advances `last_seen_at` at most once a minute: it carries nothing the requester supplied, is idempotent, and writes only the requester's own row, so there is nothing for a cross-site request to achieve by triggering it.

### Code generation

Six digits from a CSPRNG over the full `000000..=999999` range, sampled by rejection rather than by a modulus — `rng % 1_000_000` biases toward low values unless the generator's range is an exact multiple, which is the classic modulo bug and hands an attacker a better-than-uniform guessing order. Compared in constant time, so the time a comparison takes cannot reveal how long a shared prefix is.

Six digits is ≈19.93 bits, marginally under ASVS 6.5.4's 20-bit floor. Accepted rather than overlooked: the code lives ten minutes, tolerates three wrong entries, and sits behind the account counter and the per-IP limit. Seven digits would clear the floor and cost every user a keystroke on a control they use often.

The lifetime is ten minutes and is **not configurable**. NIST §3.1.3.2 (*"the authentication SHALL be considered invalid unless completed within 10 minutes"*) and ASVS 6.5.5 both cap it there, so the only thing a setting could express is a violation of both.

Codes are stored unhashed, deliberately: a code lives ten minutes, and anyone able to read the `login_codes` table already holds `sessions`.

## Mail

`lettre` over STARTTLS to the Google Workspace relay, sending as `noreply@dasch.swiss`. A university relay cannot send as `dasch.swiss`, and a login code arriving from `dasch.unibas.ch` reads as a phishing attempt.

### Relay prerequisites (not the application's job, but they block delivery)

- **DKIM** — Google DKIM-signs relayed mail with the envelope sender's domain *only if that domain has DKIM enabled in the Workspace Admin Console*. Set up for `dasch.swiss`.
- **SPF** — remains the sending domain's responsibility; `dasch.swiss` must include Google. Set up.
- **Quota** — the relay allows 10,000 recipients a day, and whether that is shared with other senders determines where `EDITOR_MAIL_DAILY_CAP` should sit. The default of 500 is well below the ceiling either way.

### No address ever reaches a log (REQ-6.10)

The claim is precisely "an account holder's address", not "any address": `EDITOR_SMTP_FROM` is logged once at startup, and it is a service mailbox nobody signs in with. There is also one channel this does not close — see [Observability](./observability.md) on `url.query`.

Two things make that hold rather than merely be intended:

- Every instrumented function is `#[tracing::instrument(skip_all)]`, so no argument becomes a span field by default. The correlation id is the account's own UUID, which is already opaque and not derived from the address.
- `MailError` carries a **classification and an SMTP status code, never the relay's reply**. A relay's reply routinely quotes the recipient — `550 5.1.1 <someone@example.org> User unknown` is the canonical shape — so logging a transport error verbatim would write addresses into the log pipeline on exactly the paths nobody exercises.

The one place in the service that stringifies a `lettre` error verbatim is the relay's *setup*, and it is safe only because it runs before any envelope exists — there is nothing for a reply to quote yet. That reasoning is written at the call site, because it stops holding the moment anything routes a connected transport's error through it.

A test drives the unknown-address, issued, wrong-code, signed-in, malformed and relay-refused paths under a capturing subscriber and asserts on span fields as well as events.

### When there is no relay, and when the relay is broken

With `EDITOR_SMTP_HOST` unset, codes are written to the log and the service stays usable (REQ-6.8). That is the development transport, the PR-preview transport, and the break-glass for a broken relay.

**`EDITOR_ENV=PROD` with no relay is refused at startup.** That combination is the dangerous one precisely because nothing goes wrong: the service behaves normally and every login code for the life of the deployment sits in the log pipeline. Development, `just dev-editor`, `just run-docker-editor` and the PR preview all run as `DEV`, where the console transport is the point.

A **configured** relay that fails is different. By default the code is rolled back — the row and, with it, the cooldown, so the user is not left waiting out a cooldown for a code that never arrived — and the response is unchanged. Setting `EDITOR_SMTP_BREAK_GLASS=true` instead keeps the code alive and writes it to the log.

That is off by default on purpose. A transient relay error — a rate limit, a TLS blip — would otherwise write a **live credential** into a log pipeline that retains it for weeks and is readable by everyone with log access. Turning it on is an incident response for a relay broken long enough to lock people out, and the failure log line names the variable so the remedy is found from the error itself. Turn it off again once the relay is back.

## Preview safety

The Cloud Run PR preview is `--allow-unauthenticated`, so `/login` is publicly reachable the moment it deploys. The preview leaves `EDITOR_SMTP_*` unset, so REQ-6.8's console transport applies: codes go to Cloud Run logs, no mail leaves the Workspace relay, and the shared daily quota is untouched. There is deliberately **no** dev-only "show the code on screen" affordance — read it from the logs.

Gating the preview URL to GitHub organisation members is not available: Cloud Run IAM gates on *Google* identity, not GitHub membership, and the two do not connect. The workflow's same-repo and non-dependabot conditions gate who can *trigger* a deploy, not who can reach the URL.
