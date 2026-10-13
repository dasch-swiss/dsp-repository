# AI Agent Guide for Identity

Guidance for the Deposit Area's identity capability; the editor's is in `areas/deposit/editor/CLAUDE.md`, and project-wide guidance in the top-level `CLAUDE.md`. Vocabulary: `areas/deposit/identity/CONTEXT.md`.

## Where identity lives

`identity-core` (`core/`) holds identity's domain: `User`, `Role`, `Session` and `LoginCode`, the repository traits and their `IdentityRepositories` bundle, the login rules (`AuthConfig`, `locked_out`, `CODE_TTL`, `SEND_WINDOW`), the session rules (`begin`, `current`, `end`), the cookie names and the `Cookie`-header scan, code and token generation, and the `Mailer` trait. It has no Axum, Maud or database dependency, and it depends on no `editor-*` crate; `editor-core` does not depend on it either (ADR-0003: capabilities share no core). Nothing checks that yet, so it holds by review.

A new account, session, login-code or mail type goes in `identity-core`, never in `editor-core`. When an editor crate needs a fact about an account (a display name, a role), `editor-server` hands it over as a plain value — an id, a `&str`, a small struct of strings, as `editor-web/src/pages/depositors.rs` takes — so the editor crates never name an `identity_core` type.

The login and depositor handlers, the `Authenticated`/`Rdu` extractors, the `Set-Cookie` glue, the SQLite implementation and the mail transports are still in `editor-server`. `docs/src/editor/architecture.md` ("Divergence from the target architecture") lists what is left to move, and `docs/src/editor/authentication.md` describes the flow and the control behind each step.

## Common Pitfalls

### Configuration refuses unsafe combinations at startup

`EditorConfig::validate` (`editor-server/src/config.rs`; `EditorConfig::auth_config` builds `AuthConfig` from the same config) stops the process rather than starting in a state that locks users out or leaks. Two that bite in test setups: `EDITOR_ENV=PROD` requires `EDITOR_SMTP_HOST` (a console mailer in production is a standing credential leak), and `EDITOR_LOGIN_COOLDOWN_SECS` must be **at least 1** — it cannot be set to zero.

With `EDITOR_SMTP_HOST` unset the console transport writes login codes to the log, which is how development, the PR preview and the E2E suite sign in.
