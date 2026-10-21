# AI Agent Guide for Identity

Guidance for the Deposit Area's identity capability; the editor's is in `areas/deposit/editor/CLAUDE.md`, and project-wide guidance in the top-level `CLAUDE.md`. Vocabulary: `areas/deposit/identity/CONTEXT.md`.

## Where identity lives

`identity-core` (`core/`) holds identity's domain: `User`, `Role`, `Session` and `LoginCode`, the repository traits and their `IdentityRepositories` bundle, the login rules (`AuthConfig`, `locked_out`, `CODE_TTL`, `SEND_WINDOW`), the session rules (`begin`, `current`, `end`), the cookie names and the `Cookie`-header scan, code and token generation, and the `Mailer` trait. It has no Axum, Maud or database dependency, and it depends on no `editor-*` crate; `editor-core` does not depend on it either (ADR-0003: capabilities share no core). `.github/scripts/check-capability-deps.sh` (`just check`) checks all three.

A new account, session, login-code or mail type goes in `identity-core`, never in `editor-core`. When an editor crate needs a fact about an account (a display name, a role), it gets it through `editor-ports` (`Actor`, `UserDirectory`) as a plain value — an id, a `&str` — so the editor crates never name an `identity_core` type.

`identity-ports` (`ports/`) is what identity asks of the editor: `AuthoredWork`, the open drafts and submissions the depositor-removal confirmation lists; `editor-store` answers it. `identity-store` (`store/`) is identity's SQLite store, the file `identity.sqlite`, implementing the four repository ports, and holds the mail transports (`SmtpMailer`, `ConsoleMailer`, `mail::mailer`) and the adapters behind `editor-ports`: `LiveAuthenticator`, which calls `session::current` and must stay its only caller on the editor's side, and `LiveUserDirectory`. `identity-web` (`web/`) is identity's HTTP surface: the login and depositor pages (`pages/`), their handlers (`handlers/`; besides them only the route table, `shell.rs` and `lib.rs` use axum), identity's own `Authenticated`/`Rdu` extractors, the `Set-Cookie` glue and the route table, behind the `Identity` facade. `Identity::start` runs the RDU bootstrap, builds the mailer and spawns the hourly sweep; `Identity::router` returns the routes over identity's state, and `deposit-server` merges it under its CSRF and OTel layers. `identity-web` depends on no editor crate: the open work the removal confirmation lists comes through `identity-ports`. `docs/src/editor/architecture.md` describes the crates, and `docs/src/editor/authentication.md` the flow and the control behind each step.

## Common Pitfalls

### Configuration refuses unsafe combinations at startup

`EditorConfig::validate` (`areas/deposit/server/src/config.rs`; `EditorConfig::identity_settings` builds `IdentitySettings` from the same config) stops the process rather than starting in a state that locks users out or leaks. Two that bite in test setups: `EDITOR_ENV=PROD` requires `EDITOR_SMTP_HOST` (a console mailer in production is a standing credential leak), and `EDITOR_LOGIN_COOLDOWN_SECS` must be **at least 1** — it cannot be set to zero.

With `EDITOR_SMTP_HOST` unset the console transport writes login codes to the log, which is how development, the PR preview and the E2E suite sign in.
