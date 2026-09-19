// Live integration test for `dsp vre vocabulary list` / `describe` (plan 034).
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//
// Token supply (all optional — both vocabulary endpoints are public):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;

// ---------------------------------------------------------------------------
// Helpers (mirror live_resource_type_list.rs exactly)
// ---------------------------------------------------------------------------

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

/// Read an optional environment variable. Returns `None` silently if absent.
fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test against the geoarch project (shortcode `0838`), a
/// stable DaSCH fixture: lists its vocabularies, describes the `epoch`
/// (Period) vocabulary, and proves D2 (a node-IRI describe returns the WHOLE
/// vocabulary, never just that node's subtree) plus `--subtree`-style branch
/// counting via `VocabularyTree::count_and_depth`.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
#[test]
fn live_vocabulary_list_and_describe_on_geoarch() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim()))
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // Both `list_vocabularies` and `describe_vocabulary` are public endpoints;
    // the token is optional. If present it exercises the authenticated code
    // path, but its absence is not a skip condition. NEVER interpolate the
    // token value in any log or assertion message.
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test: DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test: DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 3. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 4. Resolve the known project (geoarch / shortcode 0838) ──────────────
    // A stable DaSCH fixture project whose vocabulary set (21 roots) and
    // `epoch`/Period vocabulary shape (33 nodes, 2 levels) are verified
    // ground truth as of 2026-07-29/30.
    let project_identifier = "0838";
    eprintln!(
        "live test: resolving project '{}' on {}",
        project_identifier, cfg.server
    );

    let proj = client
        .resolve_project(&cfg.server, project_identifier)
        .expect(
            "resolve_project failed for '0838' (geoarch) — check DSP_TEST_SERVER and network \
         connectivity; if geoarch has been removed from this server, update the test to use a \
         different well-known project shortcode",
        );

    eprintln!(
        "live test: resolved project {} (shortcode {}, shortname {})",
        proj.iri, proj.shortcode, proj.shortname
    );

    // ── 5. list_vocabularies must return exactly 21 roots ────────────────────
    eprintln!(
        "live test: calling list_vocabularies for project IRI {}",
        proj.iri
    );

    let vocabs = client
        .list_vocabularies(&cfg.server, &proj.iri, token_ref)
        .expect("list_vocabularies failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} vocabulary root(s)", vocabs.len());
    assert_eq!(
        vocabs.len(),
        21,
        "geoarch is a stable DaSCH fixture project expected to carry exactly 21 vocabulary \
         roots; if this ever breaks, the vocabulary count changed upstream — update this \
         assertion only after confirming the change is real, not a transient server issue"
    );

    // ── 6. Find "epoch" (case-insensitive name match) and describe it ────────
    let epoch = vocabs
        .iter()
        .find(|v| {
            v.header
                .name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case("epoch"))
        })
        .expect("epoch vocabulary not found in geoarch's vocabulary list");

    eprintln!(
        "live test: found epoch vocabulary (iri: {})",
        epoch.header.iri
    );

    let epoch_tree = client
        .describe_vocabulary(&cfg.server, &epoch.header.iri, token_ref)
        .expect(
            "describe_vocabulary(epoch root iri) failed — check DSP_TEST_SERVER and network \
             connectivity",
        );

    let epoch_stats = epoch_tree.count_and_depth(None);
    eprintln!("live test: epoch tree stats (nodes, depth): {epoch_stats:?}");
    assert_eq!(
        epoch_stats,
        (33, 2),
        "epoch (Period) is a stable DaSCH fixture vocabulary expected to have 33 nodes across \
         2 levels — this crate's depth semantic, where the root's direct children are level 1 \
         (NOT the informal '3 levels' count used loosely elsewhere); if this ever breaks, the \
         vocabulary changed upstream"
    );

    // ── 7. Describe a NON-root node directly — must return the WHOLE vocabulary (D2) ──
    let node_iri = epoch_tree
        .children
        .first()
        .expect("epoch tree must have at least one top-level child")
        .header
        .iri
        .clone();

    eprintln!("live test: describing node IRI directly: {node_iri}");

    let node_tree = client
        .describe_vocabulary(&cfg.server, &node_iri, token_ref)
        .expect(
            "describe_vocabulary(node iri) failed — check DSP_TEST_SERVER and network \
             connectivity",
        );

    assert_eq!(
        node_tree.requested_node,
        Some(node_iri.clone()),
        "describing a node IRI must set requested_node to the addressed node IRI"
    );

    let node_tree_stats = node_tree.count_and_depth(None);
    eprintln!("live test: node-addressed tree stats (nodes, depth): {node_tree_stats:?}");
    assert_eq!(
        node_tree_stats,
        (33, 2),
        "D2: describing a node IRI must return the WHOLE vocabulary, never just that node's \
         subtree — stats must match the root-addressed describe above"
    );

    // ── 8. --subtree-style branch counting on the SAME tree ──────────────────
    let (branch_count, branch_depth) = node_tree.count_and_depth(Some(&node_iri));
    eprintln!(
        "live test: branch stats for node {node_iri}: ({branch_count} nodes, {branch_depth} levels)"
    );
    assert!(
        branch_count < 33,
        "a non-root node's own branch must be smaller than the whole vocabulary (33 nodes); \
         the exact size is data-dependent so it is not hardcoded — got: {branch_count}"
    );
    assert!(
        branch_count >= 1,
        "a node's own branch must include at least the node itself, got: {branch_count}"
    );

    eprintln!("live test: PASSED — vocabulary list/describe invariants hold for geoarch/epoch");
}
