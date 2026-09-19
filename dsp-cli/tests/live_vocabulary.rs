// Live integration test for `dsp vre vocabulary list` / `describe` (plan 034).
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just dsp-cli-test-live` recipe runs exactly this command.
//
// dsp-cli/ADR-0009 (testing strategy): live tests are **not** in the default
// suite. They require environment variables pointing at a reachable DSP stack,
// and missing config causes an early-return skip — never a test failure. The
// `dsp-cli-drift` workflow runs them in CI against the pinned stack in
// `dsp-cli/ci/stack/`, with `DSP_LIVE_STRICT=1` so a skip there is a failure.
//
// Required environment variables:
//   DSP_TEST_SERVER        — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//   DSP_TEST_VOCAB_PROJECT — project identifier (shortcode or IRI) whose vocabularies this test
//                            exercises. In CI this is dsp-api's `anything` fixture project
//                            ("0001").
//
// Token supply (all optional — both vocabulary endpoints are public):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use std::env;

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;
use dsp_cli::model::VocabularyNode;

mod common;
use common::{optional_env, require_env};

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test against a configurable project (`DSP_TEST_VOCAB_PROJECT`): lists its
/// vocabularies, describes the first one the server returns, and proves D2 (a node-IRI describe
/// returns the WHOLE vocabulary, never just that node's subtree) plus `--subtree`-style branch
/// counting via `VocabularyTree::count_and_depth`.
///
/// Assertions are STRUCTURAL, not exact counts — fixture data (dsp-api's `anything` project in
/// CI) does not carry production ground truth, so this checks shape (at least one vocabulary
/// root, a tree of depth >= 2, every node labeled) rather than specific numbers.
///
/// Skips cleanly (with an `eprintln!`) if `DSP_TEST_SERVER` or `DSP_TEST_VOCAB_PROJECT` is
/// absent, or if the named project cannot be resolved. Never fails due to missing config — only
/// due to real errors. Set `DSP_LIVE_STRICT=1` to turn any of those skips into a panic.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_vocabulary_list_and_describe() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    let project_identifier = match require_env("DSP_TEST_VOCAB_PROJECT") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

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

    // ── 4. Resolve the configured project ─────────────────────────────────────
    eprintln!("live test: resolving project '{}' on {}", project_identifier, cfg.server);

    let proj = match client.resolve_project(&cfg.server, &project_identifier) {
        Ok(p) => p,
        Err(e) => {
            if env::var("DSP_LIVE_STRICT").as_deref() == Ok("1") {
                panic!("DSP_LIVE_STRICT=1: project '{project_identifier}' could not be resolved: {e}");
            }
            eprintln!("skipping live test: project '{project_identifier}' could not be resolved: {e}");
            return;
        }
    };

    eprintln!(
        "live test: resolved project {} (shortcode {}, shortname {})",
        proj.iri, proj.shortcode, proj.shortname
    );

    // ── 5. list_vocabularies must return at least one root ────────────────────
    eprintln!("live test: calling list_vocabularies for project IRI {}", proj.iri);

    let vocabs = client
        .list_vocabularies(&cfg.server, &proj.iri, token_ref)
        .expect("list_vocabularies failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} vocabulary root(s)", vocabs.len());
    assert!(
        !vocabs.is_empty(),
        "project '{project_identifier}' is expected to carry at least one vocabulary root"
    );

    // ── 6. Describe roots until one is nested deeply enough to exercise D2 ───
    // DSP-API does not specify the order of the vocabulary list, and a project
    // may legitimately hold flat vocabularies beside nested ones — dsp-api's
    // own `anything` fixture has both (`testList`'s children are all leaves,
    // `treeList`'s are not). Taking whichever root came back first would make
    // this test report drift that is really just list ordering, so scan for a
    // root of depth >= 2 and assert on that one.
    let mut chosen: Option<(String, dsp_cli::model::VocabularyTree, usize, usize)> = None;
    for vocab in &vocabs {
        let tree = client
            .describe_vocabulary(&cfg.server, &vocab.header.iri, token_ref)
            .expect("describe_vocabulary(vocabulary root iri) failed — check DSP_TEST_SERVER and network connectivity");

        let (count, depth) = tree.count_and_depth(None);
        eprintln!(
            "live test: vocabulary {} stats (nodes, depth): ({count}, {depth})",
            vocab.header.iri
        );

        // Every vocabulary the project holds must be fully labelled, not just
        // the one that ends up carrying the D2 assertions.
        assert_all_nodes_labeled(&tree.children);

        if depth >= 2 {
            chosen = Some((vocab.header.iri.clone(), tree, count, depth));
            break;
        }
    }

    let (chosen_iri, vocab_tree, whole_count, whole_depth) = chosen.expect(
        "no vocabulary in this project is a tree of depth >= 2 — this crate's depth semantic, \
         where the root's direct children are level 1 (NOT the informal '3 levels' count used \
         loosely elsewhere); the D2 invariant below needs a non-root node to address, so point \
         DSP_TEST_VOCAB_PROJECT at a project with at least one nested vocabulary",
    );

    eprintln!("live test: exercising D2 against vocabulary {chosen_iri} ({whole_count} nodes, depth {whole_depth})");

    // ── 7. Describe a NON-root node directly — must return the WHOLE vocabulary (D2) ──
    let node_iri = vocab_tree
        .children
        .first()
        .expect("vocabulary tree must have at least one top-level child")
        .header
        .iri
        .clone();

    eprintln!("live test: describing node IRI directly: {node_iri}");

    let node_tree = client
        .describe_vocabulary(&cfg.server, &node_iri, token_ref)
        .expect("describe_vocabulary(node iri) failed — check DSP_TEST_SERVER and network connectivity");

    assert_eq!(
        node_tree.requested_node,
        Some(node_iri.clone()),
        "describing a node IRI must set requested_node to the addressed node IRI"
    );

    let node_tree_stats = node_tree.count_and_depth(None);
    eprintln!("live test: node-addressed tree stats (nodes, depth): {node_tree_stats:?}");
    assert_eq!(
        node_tree_stats,
        (whole_count, whole_depth),
        "D2: describing a node IRI must return the WHOLE vocabulary, never just that node's \
         subtree — stats must match the root-addressed describe above"
    );

    // ── 8. --subtree-style branch counting on the SAME tree ──────────────────
    let (branch_count, branch_depth) = node_tree.count_and_depth(Some(&node_iri));
    eprintln!("live test: branch stats for node {node_iri}: ({branch_count} nodes, {branch_depth} levels)");
    assert!(
        branch_count < whole_count,
        "a non-root node's own branch must be smaller than the whole vocabulary ({whole_count} \
         nodes); the exact size is data-dependent so it is not hardcoded — got: {branch_count}"
    );
    assert!(
        branch_count >= 1,
        "a node's own branch must include at least the node itself, got: {branch_count}"
    );

    eprintln!("live test: PASSED — vocabulary list/describe invariants hold");
}

/// Recursively assert that every node in `nodes`, and all of their descendants, carries at least
/// one label. Fixture data carries no guaranteed label content beyond "some label exists", so
/// this walks the whole tree rather than checking only the root's immediate children.
fn assert_all_nodes_labeled(nodes: &[VocabularyNode]) {
    for node in nodes {
        assert!(
            !node.header.labels.is_empty(),
            "every vocabulary node must carry a label — node {} has none",
            node.header.iri
        );
        assert_all_nodes_labeled(&node.children);
    }
}
