// Live integration test for `dsp vre sparql query` (plan 035).
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// ADR-0009 (testing strategy): live tests are **not** in CI. They require a
// real local DSP-API instance with the SPARQL passthrough enabled — no
// deployed `api.*.dasch.swiss` environment has the route yet (§Verified API
// facts). Missing config causes an early-return skip — never a test failure.
//
// Bring up the local stack per the plan's Step 5 (verified 2026-08-07 — NOT
// `just stack-up`, which fails on a machine without `bazel` on PATH):
//
//   cd ~/Documents/GitHub/dasch-swiss/dsp-api && git pull --ff-only && just init-db-test
//   cd ~/Documents/GitHub/dasch-swiss/dsp-api && \
//     KNORA_WEBAPI_ALLOW_SPARQL_PASSTHROUGH=true nix develop --command bazel run //modules/webapi:app
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL, e.g. "http://localhost:3333"
//   DSP_TOKEN         — a SystemAdmin bearer token (D11: the endpoint is
//                       never anonymous — unlike `live_project_list.rs`,
//                       this file has no anonymous path to fall back to).
//
// NEVER log the token value.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;
use dsp_cli::diagnostic::Diagnostic;

/// The named graph the plan's §Verified API facts records as present in the
/// `init-db-test` fixture data.
const ANYTHING_GRAPH: &str = "http://www.knora.org/data/0001/anything";

/// Read a required environment variable. Returns `None` and emits a skip
/// message if the variable is absent or empty.
fn require_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => {
            eprintln!("skipping live test: {name} not set");
            None
        }
    }
}

/// Collect `(server, token)` or `None` (with a skip message already emitted).
fn require_server_and_token() -> Option<(String, String)> {
    let server_raw = require_env("DSP_TEST_SERVER")?;
    let token = require_env("DSP_TOKEN")?;
    let cfg = Config::resolve(Some(server_raw.trim()))
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");
    Some((cfg.server, token))
}

// ---------------------------------------------------------------------------
// SELECT → parseable SPARQL-JSON
// ---------------------------------------------------------------------------

#[test]
fn live_select_returns_parseable_sparql_json_with_bindings() {
    let Some((server, token)) = require_server_and_token() else {
        return;
    };

    let client = HttpDspClient::new().expect("failed to build HTTP client");
    let query =
        format!("SELECT ?s ?p ?o WHERE {{ GRAPH <{ANYTHING_GRAPH}> {{ ?s ?p ?o }} }} LIMIT 10");

    eprintln!("live test: sparql_query (SELECT, json) on {server}");
    let resp = client
        .sparql_query(
            &server,
            &token,
            &query,
            "application/sparql-results+json",
            3600,
        )
        .expect(
            "sparql_query failed — check DSP_TEST_SERVER, DSP_TOKEN, and that the local \
                 stack has the passthrough enabled",
        );

    assert_eq!(
        resp.status, 200,
        "expected a 2xx relay for a well-formed SELECT"
    );
    // Media type only — the store appends parameters (observed 2026-08-07:
    // `application/sparql-results+json; charset=utf-8`). dsp-cli relays the
    // header without parsing it (D3), so asserting an exact string here would
    // pin a store detail the command deliberately does not interpret.
    let ct = resp.content_type.as_deref().unwrap_or_default();
    assert!(
        ct.starts_with("application/sparql-results+json"),
        "expected a SPARQL-JSON media type, got: {ct}"
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&resp.body).expect("response body must be valid JSON");
    assert!(
        parsed
            .get("results")
            .and_then(|r| r.get("bindings"))
            .is_some(),
        "expected a SPARQL-JSON results.bindings shape, got: {parsed}"
    );

    eprintln!("live test: PASSED — SELECT returned parseable SPARQL-JSON");
}

// ---------------------------------------------------------------------------
// --accept csv → a header row
// ---------------------------------------------------------------------------

#[test]
fn live_accept_csv_returns_a_header_row() {
    let Some((server, token)) = require_server_and_token() else {
        return;
    };

    let client = HttpDspClient::new().expect("failed to build HTTP client");
    let query =
        format!("SELECT ?s ?p ?o WHERE {{ GRAPH <{ANYTHING_GRAPH}> {{ ?s ?p ?o }} }} LIMIT 3");

    eprintln!("live test: sparql_query (SELECT, csv) on {server}");
    let resp = client
        .sparql_query(&server, &token, &query, "text/csv", 3600)
        .expect("sparql_query failed");

    assert_eq!(resp.status, 200);
    let body = String::from_utf8(resp.body).expect("CSV body must be UTF-8");
    let first_line = body.lines().next().unwrap_or_default();
    assert!(
        first_line.contains('s') && first_line.contains('p') && first_line.contains('o'),
        "expected a header row naming the SELECT's variables, got: {first_line}"
    );

    eprintln!("live test: PASSED — --accept csv returned a header row");
}

// ---------------------------------------------------------------------------
// Malformed query → the store's own 4xx, relayed (not an Internal error)
// ---------------------------------------------------------------------------

#[test]
fn live_malformed_query_relays_as_ok_with_a_non_2xx_status() {
    let Some((server, token)) = require_server_and_token() else {
        return;
    };

    let client = HttpDspClient::new().expect("failed to build HTTP client");

    eprintln!("live test: sparql_query (malformed) on {server}");
    let resp = client
        .sparql_query(
            &server,
            &token,
            "SELEKT ?s WHERE { ?s ?p ?o }",
            "application/sparql-results+json",
            3600,
        )
        .expect("a malformed query must relay as Ok(SparqlResponse), not Err (D7)");

    assert!(
        !(200..300).contains(&resp.status),
        "expected a non-2xx store rejection for a malformed query, got status {}",
        resp.status
    );
    assert!(
        !resp.body.is_empty(),
        "expected a non-empty store error body"
    );

    eprintln!(
        "live test: PASSED — malformed query relayed with status {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// 403 (non-admin token) — best-effort, skipped unless a second token is set
// ---------------------------------------------------------------------------

#[test]
fn live_non_admin_token_maps_to_auth_required_403() {
    let Some((server, _admin_token)) = require_server_and_token() else {
        return;
    };
    let Some(non_admin_token) = require_env("DSP_TEST_NON_ADMIN_TOKEN") else {
        return;
    };

    let client = HttpDspClient::new().expect("failed to build HTTP client");

    eprintln!("live test: sparql_query (non-admin token) on {server}");
    let err = client
        .sparql_query(
            &server,
            &non_admin_token,
            "SELECT ?s WHERE { ?s ?p ?o } LIMIT 1",
            "application/sparql-results+json",
            3600,
        )
        .expect_err("a non-SystemAdmin token must be Err(AuthRequired) — a 403, per D8");

    match err {
        Diagnostic::AuthRequired(msg) => {
            assert!(
                msg.contains("system administrator"),
                "403 message must explain the SystemAdmin requirement: {msg}"
            );
        }
        other => panic!("expected AuthRequired, got: {other:?}"),
    }

    eprintln!("live test: PASSED — non-admin token mapped to AuthRequired");
}
