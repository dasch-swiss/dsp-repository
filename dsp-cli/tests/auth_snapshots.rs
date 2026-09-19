//! Snapshot tests for all auth renderer cells.
//!
//! Drives the renderer methods directly by constructing outcome structs and
//! writing to a `Vec<u8>` sink — no binary spawning, no HTTP calls, deterministic.
//!
//! Coverage:
//! - 45 success cells: 9 scenarios × 5 formats
//!   - `auth_login` (token acquired)
//!   - `auth_status` logged-in (not expired)
//!   - `auth_status` cached token (expired) — D3 headline: `_meta.auth` shows
//!     "authenticated as <user>" while data.state shows "expired"
//!   - `auth_status` env-authenticated (DSP_TOKEN, not expired / active)
//!   - `auth_status` env-authenticated (DSP_TOKEN, expired)
//!   - `auth_status` env-authenticated (DSP_TOKEN, expiry unknown / None)
//!   - `auth_status` not-logged-in
//!   - `auth_logout` was_cached=true
//!   - `auth_logout` was_cached=false
//! - 6 auth_set_token cells: 5 format success cells + 1 minimal (None fields) cell
//!   - `auth_set_token` × {prose, json, lines, csv, tsv}
//!   - `auth_set_token` prose minimal (user=None, expires_at=None)
//! - 6 failure cells (JSON error envelope + prose stderr routing):
//!   - `auth_login` × json × AuthRequired
//!   - `auth_login` × json × Network
//!   - `auth_login` × json × ServerError
//!   - `auth_login` × prose × AuthRequired (stdout must be empty)
//!   - `auth_set_token` × prose × Usage (non-JWT stdin)
//!   - `auth_set_token` × json × AuthRequired (probe-rejected token)
//!
//! All fixtures use a fixed UTC timestamp for determinism (no `Utc::now()`).

use chrono::{TimeZone, Utc};

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::render::auth::{
    AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome,
};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Fixed timestamp used by all test fixtures — keeps snapshots stable across runs.
fn fixed_expires() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 25, 12, 34, 56).unwrap()
}

fn login_outcome() -> AuthLoginOutcome {
    AuthLoginOutcome {
        server: "https://api.example.com".to_string(),
        user: "user@example.com".to_string(),
        expires_at: Some(fixed_expires()),
    }
}

fn login_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated as user@example.com".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn status_logged_in_outcome() -> AuthStatusOutcome {
    AuthStatusOutcome::LoggedIn {
        server: "https://api.example.com".to_string(),
        user: Some("user@example.com".to_string()),
        expires_at: Some(fixed_expires()),
        expired: false,
    }
}

fn status_not_logged_in_outcome() -> AuthStatusOutcome {
    AuthStatusOutcome::NotLoggedIn {
        server: "https://api.example.com".to_string(),
    }
}

fn status_logged_in_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated as user@example.com".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn status_env_outcome() -> AuthStatusOutcome {
    AuthStatusOutcome::AuthenticatedViaEnv {
        server: "https://api.example.com".to_string(),
        expires_at: Some(fixed_expires()),
        expired: false,
    }
}

fn status_env_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated via DSP_TOKEN".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn status_env_expired_outcome() -> AuthStatusOutcome {
    AuthStatusOutcome::AuthenticatedViaEnv {
        server: "https://api.example.com".to_string(),
        expires_at: Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()),
        expired: true,
    }
}

fn status_env_expired_meta() -> MetaContext {
    // _meta.auth uses presence/origin semantics (ADR-0007): an expired env token
    // still reports "authenticated via DSP_TOKEN". The expiry detail lives in
    // the data output (AuthenticatedViaEnv.expired == true).
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated via DSP_TOKEN".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn status_cache_expired_outcome() -> AuthStatusOutcome {
    // D3 headline case: an expired-but-present CACHED token. The `auth status`
    // data output reports the expiry (expired==true, state=="expired") while
    // `_meta.auth` uses presence/origin semantics ("authenticated as <user>").
    AuthStatusOutcome::LoggedIn {
        server: "https://api.example.com".to_string(),
        user: Some("user@example.com".to_string()),
        expires_at: Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()),
        expired: true,
    }
}

fn status_cache_expired_meta() -> MetaContext {
    // _meta.auth uses presence/origin semantics (ADR-0007 / D3): an expired
    // cached token still reports "authenticated as <user>", exactly as the
    // read commands do. The expiry detail lives in the data output
    // (LoggedIn.expired == true, state == "expired").
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated as user@example.com".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn status_env_unknown_outcome() -> AuthStatusOutcome {
    AuthStatusOutcome::AuthenticatedViaEnv {
        server: "https://api.example.com".to_string(),
        expires_at: None,
        expired: false,
    }
}

fn status_env_unknown_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "authenticated via DSP_TOKEN".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn not_logged_in_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn logout_was_cached_outcome() -> AuthLogoutOutcome {
    AuthLogoutOutcome {
        server: "https://api.example.com".to_string(),
        was_cached: true,
    }
}

fn logout_not_cached_outcome() -> AuthLogoutOutcome {
    AuthLogoutOutcome {
        server: "https://api.example.com".to_string(),
        was_cached: false,
    }
}

fn logout_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn failure_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.example.com".to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── success cells: auth_login ─────────────────────────────────────────────────

#[test]
fn auth_login_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_login(&login_outcome(), &login_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_login_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_login(&login_outcome(), &login_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_login_lines() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_login(&login_outcome(), &login_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_login_csv() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_login(&login_outcome(), &login_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_login_tsv() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_login(&login_outcome(), &login_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── success cells: auth_status logged-in ─────────────────────────────────────

#[test]
fn auth_status_prose_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(&status_logged_in_outcome(), &status_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(&status_logged_in_outcome(), &status_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(&status_logged_in_outcome(), &status_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_csv_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(&status_logged_in_outcome(), &status_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_tsv_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(&status_logged_in_outcome(), &status_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── success cells: auth_status env-authenticated (DSP_TOKEN) ─────────────────

#[test]
fn auth_status_prose_env() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(&status_env_outcome(), &status_env_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_env() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(&status_env_outcome(), &status_env_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_env() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(&status_env_outcome(), &status_env_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in lines stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_csv_env() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(&status_env_outcome(), &status_env_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in csv stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_tsv_env() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(&status_env_outcome(), &status_env_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in tsv stdout"
    );
    insta::assert_snapshot!(out);
}

// ── success cells: auth_status env-authenticated (DSP_TOKEN, expired) ─────────

#[test]
fn auth_status_prose_env_expired() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(&status_env_expired_outcome(), &status_env_expired_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_env_expired() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(&status_env_expired_outcome(), &status_env_expired_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_env_expired() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(&status_env_expired_outcome(), &status_env_expired_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in lines stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_csv_env_expired() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(&status_env_expired_outcome(), &status_env_expired_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in csv stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_tsv_env_expired() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(&status_env_expired_outcome(), &status_env_expired_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in tsv stdout"
    );
    insta::assert_snapshot!(out);
}

// ── success cells: auth_status cached token (expired) ────────────────────────
//
// D3 headline case: _meta.auth reports "authenticated as <user>" (presence/origin
// semantics, same as read commands) while data.state reports "expired".

#[test]
fn auth_status_prose_cache_expired() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(
        &status_cache_expired_outcome(),
        &status_cache_expired_meta(),
    )
    .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_cache_expired() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(
        &status_cache_expired_outcome(),
        &status_cache_expired_meta(),
    )
    .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_cache_expired() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(
        &status_cache_expired_outcome(),
        &status_cache_expired_meta(),
    )
    .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_csv_cache_expired() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(
        &status_cache_expired_outcome(),
        &status_cache_expired_meta(),
    )
    .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_tsv_cache_expired() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(
        &status_cache_expired_outcome(),
        &status_cache_expired_meta(),
    )
    .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── success cells: auth_status env-authenticated (DSP_TOKEN, expiry unknown) ──

#[test]
fn auth_status_prose_env_unknown() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(&status_env_unknown_outcome(), &status_env_unknown_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_env_unknown() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(&status_env_unknown_outcome(), &status_env_unknown_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_env_unknown() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(&status_env_unknown_outcome(), &status_env_unknown_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in lines stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_csv_env_unknown() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(&status_env_unknown_outcome(), &status_env_unknown_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in csv stdout"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn auth_status_tsv_env_unknown() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(&status_env_unknown_outcome(), &status_env_unknown_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("DSP_TOKEN"),
        "DSP_TOKEN disclosure must not appear in tsv stdout"
    );
    insta::assert_snapshot!(out);
}

// ── success cells: auth_status not-logged-in ─────────────────────────────────

#[test]
fn auth_status_prose_not_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_status(&status_not_logged_in_outcome(), &not_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_json_not_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_status(&status_not_logged_in_outcome(), &not_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_lines_not_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_status(&status_not_logged_in_outcome(), &not_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_csv_not_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_status(&status_not_logged_in_outcome(), &not_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_status_tsv_not_logged_in() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_status(&status_not_logged_in_outcome(), &not_logged_in_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── success cells: auth_logout was_cached=true ───────────────────────────────

#[test]
fn auth_logout_prose_was_cached() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_logout(&logout_was_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_json_was_cached() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_logout(&logout_was_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_lines_was_cached() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_logout(&logout_was_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_csv_was_cached() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_logout(&logout_was_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_tsv_was_cached() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_logout(&logout_was_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── success cells: auth_logout was_cached=false ──────────────────────────────

#[test]
fn auth_logout_prose_not_cached() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_logout(&logout_not_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_json_not_cached() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_logout(&logout_not_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_lines_not_cached() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_logout(&logout_not_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_csv_not_cached() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_logout(&logout_not_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_logout_tsv_not_cached() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_logout(&logout_not_cached_outcome(), &logout_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── failure cells ─────────────────────────────────────────────────────────────

/// `auth_login` × json × AuthRequired — locks down `kind: auth_required`, exit_code 3.
///
/// Username must NOT appear in the rendered output (ADR-0007 / PRD criterion 7).
#[test]
fn auth_login_json_auth_required() {
    let diag =
        Diagnostic::AuthRequired("Authentication failed on https://api.example.com".to_string());
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &failure_meta()).unwrap();
    let out = buf_to_string(&buf);
    insta::assert_snapshot!(out.clone());
    // Belt-and-braces guard: username must not appear even in a future snapshot change.
    assert!(
        !out.contains("user@example.com"),
        "auth_required output must not contain the username; got: {out}"
    );
}

/// `auth_login` × json × Network — locks down `kind: network`, exit_code 1.
#[test]
fn auth_login_json_network() {
    let diag = Diagnostic::Network("connection refused".to_string());
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &failure_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// `auth_login` × json × ServerError — locks down `kind: server_error`, exit_code 1.
#[test]
fn auth_login_json_server_error() {
    let diag = Diagnostic::ServerError("server returned 500".to_string());
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &failure_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// `auth_login` × prose × AuthRequired — stdout must be empty.
///
/// Prose errors are routed to stderr by `main.rs`'s `eprintln!` handler; the
/// `ProseRenderer::diagnostic` method correctly writes nothing to its writer
/// (stdout). This snapshot locks down that contract.
///
/// Username must NOT appear (ADR-0007 / PRD criterion 7).
#[test]
fn auth_login_prose_auth_required() {
    let diag =
        Diagnostic::AuthRequired("Authentication failed on https://api.example.com".to_string());
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.diagnostic(&diag, &failure_meta()).unwrap();
    let out = buf_to_string(&buf);
    insta::assert_snapshot!(out.clone());
    assert!(
        !out.contains("user@example.com"),
        "prose auth_required stdout must be empty and must not contain the username; got: {out}"
    );
}

// ── fixtures: auth_set_token ──────────────────────────────────────────────────

fn set_token_outcome() -> AuthSetTokenOutcome {
    AuthSetTokenOutcome {
        server: "https://api.dev.dasch.swiss".to_string(),
        user: Some("http://rdfh.ch/users/jDEZpYNRSlaOM7wb55ZQpw".to_string()),
        expires_at: Some(fixed_expires()),
    }
}

fn set_token_outcome_minimal() -> AuthSetTokenOutcome {
    AuthSetTokenOutcome {
        server: "https://api.dev.dasch.swiss".to_string(),
        user: None,
        expires_at: None,
    }
}

// Mirrors the auth_state the `set_token` action actually builds (see
// `actions::auth::set_token::run_impl`): "authenticated as {sub}" when the JWT
// carries a `sub`, using ADR-0007 vocabulary. Keeping the fixture in sync with
// the action is what makes these snapshots a faithful regression detector for
// real command output.
fn set_token_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.dev.dasch.swiss".to_string(),
        auth_state: "authenticated as http://rdfh.ch/users/jDEZpYNRSlaOM7wb55ZQpw".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// The minimal outcome has no `sub`, so the action falls back to "authenticated"
// (cache token with no user, ADR-0007 vocabulary).
fn set_token_meta_minimal() -> MetaContext {
    MetaContext {
        server_label: "https://api.dev.dasch.swiss".to_string(),
        auth_state: "authenticated".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

fn set_token_failure_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.dev.dasch.swiss".to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── success cells: auth_set_token ─────────────────────────────────────────────

#[test]
fn auth_set_token_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome(), &set_token_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_set_token_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome(), &set_token_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_set_token_lines() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome(), &set_token_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_set_token_csv() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome(), &set_token_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn auth_set_token_tsv() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome(), &set_token_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Minimal outcome (user=None, expires_at=None) — exercises the omit-clauses
/// prose branch: should render just `Cached token for <server>.`
#[test]
fn auth_set_token_prose_minimal() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.auth_set_token(&set_token_outcome_minimal(), &set_token_meta_minimal())
        .unwrap();
    let out = buf_to_string(&buf);
    // Guard the conditional rendering: no " as " clause and no expiry sentence.
    assert!(
        !out.contains(" as "),
        "minimal prose must not contain user clause; got: {out}"
    );
    assert!(
        !out.contains("expires"),
        "minimal prose must not contain expiry clause; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── failure cells: auth_set_token ─────────────────────────────────────────────

/// `auth_set_token` × prose × Usage — non-JWT input on stdin.
///
/// Prose errors are routed to stderr; stdout must be empty.
#[test]
fn auth_set_token_prose_usage() {
    let diag = Diagnostic::Usage("input on stdin is not a valid JWT".to_string());
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.diagnostic(&diag, &set_token_failure_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// `auth_set_token` × json × AuthRequired — probe-rejected token.
///
/// Locks down `kind: auth_required`, exit_code 3.
#[test]
fn auth_set_token_json_auth_required() {
    let diag = Diagnostic::AuthRequired(
        "token rejected by https://api.dev.dasch.swiss — it may be expired, revoked, or for a different environment"
            .to_string(),
    );
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &set_token_failure_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}
