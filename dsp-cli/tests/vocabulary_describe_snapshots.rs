//! Snapshot tests for `dsp vre vocabulary describe` — one per (fixture, format)
//! cell.
//!
//! Fixture philosophy: ONE hand-built tree, `period_like_tree()`, loosely modeled
//! on the real geoarch "Period" vocabulary (small enough to read in a diff — 6
//! nodes, not the real 33), deliberately covering every edge case the plan calls
//! out:
//! - `prehistory` (level 1): 4 languages (en/de/fr/it), no comment, has children.
//! - `stoneAge` (level 2, under `prehistory`): 2 languages (en/de), HAS a comment, has a child —
//!   this is the node used for the node-IRI and `--subtree` cases.
//! - `neolithic` (level 3, under `stoneAge`): 1 language (en only), a leaf — reaches 3 levels of
//!   nesting so DFS `number` is `1.1.1`.
//! - `bronzeAge` (level 2, under `prehistory`): an UNTAGGED label (`language: None`), a leaf — used
//!   for the `--subtree`-on-a-leaf case.
//! - `antiquity` (level 1): `name: None`, a DUPLICATE-tag label (two entries both `language:
//!   Some("en")` — `localized_column` takes the first), a leaf.
//! - `modernPeriod` (level 1): 1 language (en only), a leaf.
//!
//! Required cases:
//! 1. Whole-vocabulary describe (`subtree_of: None`, `requested_node: None`) — all 5 formats.
//! 2. Node-IRI describe (`requested_node: Some(stoneAge)`, `subtree_of: None`) — prose (trailing
//!    `←` marker + header note) and json (`data.requested_node`).
//! 3. `--subtree` describe (`subtree_of: Some(prehistory)`, `requested_node` set to the same iri,
//!    `node_count`/`depth` recomputed via `count_and_depth`) — prose (narrowed branch + `· subtree
//!    of <number>` header) and json (`data.subtree_of`).
//! 4. `--subtree` on a leaf (`bronzeAge`) — prose only.
//!
//! Determinism: these tests call `Renderer::vocabulary_describe(&detail, &meta)`
//! **directly** with a hand-built `MetaContext` / `VocabularyDetail`. They never go
//! through `run_describe_impl`, which reads the real `DSP_TOKEN` env var.
//! See `docs/dev/testing-strategy.md`, ADR-0009, and
//! `docs/design/plans/034-vre-vocabulary/implementation-plan.md`.

use dsp_cli::model::{LocalizedText, VocabularyDetail, VocabularyHeader, VocabularyNode, VocabularyTree};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_describe_impl`
/// sets from `cfg.server`.
const SERVER: &str = "api.dasch.swiss";

// ── MetaContext helper ────────────────────────────────────────────────────────

/// Anonymous `MetaContext` — mirrors what `run_describe_impl` builds when no
/// token is present. `count_cost` is always `None` for `describe` — plan 034's
/// `--count` cost disclosure only applies to `vocabulary list`.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── fixture ───────────────────────────────────────────────────────────────────

fn text(value: &str, language: Option<&str>) -> LocalizedText {
    LocalizedText {
        value: value.to_string(),
        language: language.map(str::to_string),
    }
}

fn header(iri: &str, name: Option<&str>, labels: Vec<LocalizedText>, comments: Vec<LocalizedText>) -> VocabularyHeader {
    VocabularyHeader {
        iri: iri.to_string(),
        name: name.map(str::to_string),
        labels,
        comments,
    }
}

fn node(header: VocabularyHeader, position: i32, children: Vec<VocabularyNode>) -> VocabularyNode {
    VocabularyNode { header, position, children }
}

const NEOLITHIC_IRI: &str = "http://rdfh.ch/lists/0838/neolithic";
const STONE_AGE_IRI: &str = "http://rdfh.ch/lists/0838/stoneAge";
const BRONZE_AGE_IRI: &str = "http://rdfh.ch/lists/0838/bronzeAge";
const PREHISTORY_IRI: &str = "http://rdfh.ch/lists/0838/prehistory";
const ANTIQUITY_IRI: &str = "http://rdfh.ch/lists/0838/antiquity";
const MODERN_PERIOD_IRI: &str = "http://rdfh.ch/lists/0838/modernPeriod";

/// Builds:
/// ```text
/// epoch (root)
///   1   prehistory (en/de/fr/it)
///     1.1   stoneAge (en/de, has a comment)
///       1.1.1 neolithic (en only)            (leaf)
///     1.2   bronzeAge (untagged label only)  (leaf)
///   2   antiquity (name: None, duplicate-tag en label)  (leaf)
///   3   modernPeriod (en only)               (leaf)
/// ```
/// 6 real nodes; deepest level is 3 (prehistory -> stoneAge -> neolithic).
fn period_like_tree() -> VocabularyTree {
    let neolithic = node(
        header(NEOLITHIC_IRI, Some("neolithic"), vec![text("Neolithic", Some("en"))], vec![]),
        0,
        vec![],
    );
    let stone_age = node(
        header(
            STONE_AGE_IRI,
            Some("stoneAge"),
            vec![text("Stone Age", Some("en")), text("Steinzeit", Some("de"))],
            vec![text(
                "Includes Paleolithic, Mesolithic, and Neolithic sub-periods.",
                Some("en"),
            )],
        ),
        0,
        vec![neolithic],
    );
    let bronze_age = node(
        header(BRONZE_AGE_IRI, Some("bronzeAge"), vec![text("Bronze Age", None)], vec![]),
        1,
        vec![],
    );
    let prehistory = node(
        header(
            PREHISTORY_IRI,
            Some("prehistory"),
            vec![
                text("Prehistory", Some("en")),
                text("Vorgeschichte", Some("de")),
                text("Préhistoire", Some("fr")),
                text("Preistoria", Some("it")),
            ],
            vec![],
        ),
        0,
        vec![stone_age, bronze_age],
    );
    let antiquity = node(
        header(
            ANTIQUITY_IRI,
            None,
            vec![text("Antiquity", Some("en")), text("Classical Antiquity", Some("en"))],
            vec![],
        ),
        1,
        vec![],
    );
    let modern_period = node(
        header(
            MODERN_PERIOD_IRI,
            Some("modernPeriod"),
            vec![text("Modern Period", Some("en"))],
            vec![],
        ),
        2,
        vec![],
    );
    VocabularyTree {
        root: header(
            "http://rdfh.ch/lists/0838/epoch",
            Some("epoch"),
            vec![text("Period", Some("en")), text("Epoche", Some("de"))],
            vec![],
        ),
        children: vec![prehistory, antiquity, modern_period],
        project_iri: "http://rdfh.ch/projects/0838".to_string(),
        requested_node: None,
    }
}

/// Case 1 — whole-vocabulary describe: `subtree_of: None`, `requested_node: None`.
fn whole_detail() -> VocabularyDetail {
    let tree = period_like_tree();
    let (node_count, depth) = tree.count_and_depth(None);
    VocabularyDetail { tree, subtree_of: None, node_count, depth }
}

/// Case 2 — node-IRI describe: whole vocabulary renders, `stoneAge` is merely
/// MARKED via `requested_node`. `subtree_of` stays `None`.
fn node_iri_detail() -> VocabularyDetail {
    let mut tree = period_like_tree();
    tree.requested_node = Some(STONE_AGE_IRI.to_string());
    let (node_count, depth) = tree.count_and_depth(None);
    VocabularyDetail { tree, subtree_of: None, node_count, depth }
}

/// Case 3 — `--subtree` describe on `prehistory` (a level-1 node with
/// children). `requested_node` is set to the same iri (mirrors the action
/// always setting it when the address is a node); `node_count`/`depth` are
/// branch-relative, computed via `count_and_depth(Some(iri))`.
fn subtree_detail() -> VocabularyDetail {
    let mut tree = period_like_tree();
    let iri = PREHISTORY_IRI.to_string();
    tree.requested_node = Some(iri.clone());
    let (node_count, depth) = tree.count_and_depth(Some(&iri));
    VocabularyDetail { tree, subtree_of: Some(iri), node_count, depth }
}

/// Case 4 — `--subtree` on a leaf (`bronzeAge`). D14b: the addressed leaf is
/// included as the top of its own one-node branch, so `count_and_depth` yields
/// `(1, 1)`.
fn leaf_subtree_detail() -> VocabularyDetail {
    let mut tree = period_like_tree();
    let iri = BRONZE_AGE_IRI.to_string();
    tree.requested_node = Some(iri.clone());
    let (node_count, depth) = tree.count_and_depth(Some(&iri));
    VocabularyDetail { tree, subtree_of: Some(iri), node_count, depth }
}

// ── case 1: whole-vocabulary describe × 5 formats ────────────────────────────

/// Prose render of the whole-vocabulary fixture. Locks the header line
/// (name/labels), the `Root:` IRI line, the `N nodes · M levels` summary with
/// no subtree/requested-node note, the DFS-numbered + indented node list
/// (D13 label-join order, D11 path derivation not directly visible here but
/// exercised via the number/indent), and the ADR-0007 footer.
#[test]
fn vocabulary_describe_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabulary_describe(&whole_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the whole-vocabulary fixture. Locks the `_meta`-first
/// envelope, `data.nodes`/`data.depth` ALWAYS present (unlike `list`'s
/// `Option`), the `"children"` array key (not `"nodes"`), and
/// `requested_node`/`subtree_of` OMITTED (not null) when not applicable.
#[test]
fn vocabulary_describe_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.vocabulary_describe(&whole_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("whole-vocabulary json must be valid JSON");
    assert!(
        parsed["data"].get("requested_node").is_none(),
        "requested_node must be omitted when not applicable; got:\n{out}"
    );
    assert!(
        parsed["data"].get("subtree_of").is_none(),
        "subtree_of must be omitted when not applicable; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// Lines render of the whole-vocabulary fixture. Locks one row per node:
/// `node_iri\tnumber`, no header, disclosure on stderr.
#[test]
fn vocabulary_describe_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.vocabulary_describe(&whole_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!("vocabulary_describe_lines_stdout", buf_to_string(&out_buf));
    insta::assert_snapshot!("vocabulary_describe_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the whole-vocabulary fixture. Locks the 8-column describe
/// default header (`node_iri,number,label_en,label_de,label_fr,label_it,
/// label_rm,label` — note: no `name` column in the default, unlike the full
/// column set).
#[test]
fn vocabulary_describe_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.vocabulary_describe(&whole_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("node_iri,number,label_en,label_de,label_fr,label_it,label_rm,label\n"),
        "CSV header must be the 8-column describe default; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_describe_csv_stdout", stdout);
    insta::assert_snapshot!("vocabulary_describe_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the whole-vocabulary fixture. Same columns as CSV but
/// tab-separated and unquoted.
#[test]
fn vocabulary_describe_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.vocabulary_describe(&whole_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("node_iri\tnumber\tlabel_en\tlabel_de\tlabel_fr\tlabel_it\tlabel_rm\tlabel\n"),
        "TSV header must be the 8-column describe default; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_describe_tsv_stdout", stdout);
    insta::assert_snapshot!("vocabulary_describe_tsv_stderr", buf_to_string(&err_buf));
}

// ── case 2: node-IRI describe (requested_node, no subtree) ──────────────────

/// Prose render of the node-IRI fixture. Locks the whole vocabulary still
/// rendering (all 6 nodes present), the header's "you asked about 1.1" note
/// (`stoneAge`'s DFS number), and the trailing `←` marker on `stoneAge`'s own
/// row (and nowhere else).
#[test]
fn vocabulary_describe_prose_requested_node() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabulary_describe(&node_iri_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("you asked about 1.1"),
        "prose must show the requested-node header note naming stoneAge's number 1.1; got:\n{out}"
    );
    let stone_age_line = out
        .lines()
        .find(|l| l.contains("Stone Age"))
        .expect("stoneAge row must be present");
    assert!(
        stone_age_line.contains('\u{2190}'),
        "stoneAge's own row must carry the ← marker; got: {stone_age_line:?}"
    );
    let neolithic_line = out
        .lines()
        .find(|l| l.contains("Neolithic"))
        .expect("neolithic row must be present");
    assert!(
        !neolithic_line.contains('\u{2190}'),
        "neolithic must NOT carry the ← marker; got: {neolithic_line:?}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the node-IRI fixture. Locks `data.requested_node` present
/// (the stoneAge iri) and `data.subtree_of` absent — the whole tree's node
/// count is unaffected (marking, not narrowing).
#[test]
fn vocabulary_describe_json_requested_node() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.vocabulary_describe(&node_iri_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("requested-node json must be valid JSON");
    assert_eq!(
        parsed["data"]["requested_node"], STONE_AGE_IRI,
        "data.requested_node must be stoneAge's iri; got: {}",
        parsed["data"]["requested_node"]
    );
    assert!(
        parsed["data"].get("subtree_of").is_none(),
        "subtree_of must be omitted for a plain node-IRI describe; got:\n{out}"
    );
    assert_eq!(
        parsed["data"]["nodes"], 6,
        "whole-vocabulary node count must be unaffected by marking; got: {}",
        parsed["data"]["nodes"]
    );
    insta::assert_snapshot!(out);
}

// ── case 3: --subtree describe (prehistory branch) ───────────────────────────

/// Prose render of the `--subtree` fixture. Locks the narrowed branch (only
/// `prehistory`, `stoneAge`, `neolithic`, `bronzeAge` — `antiquity` and
/// `modernPeriod` are gone), the `· subtree of 1` header note (prehistory's
/// absolute number), and that absolute numbers (`1.1`, `1.1.1`) are unchanged
/// from the whole-tree view.
#[test]
fn vocabulary_describe_prose_subtree() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabulary_describe(&subtree_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("subtree of 1"),
        "prose must show the '· subtree of 1' header note; got:\n{out}"
    );
    assert!(
        out.contains("1.1") && out.contains("1.1.1"),
        "absolute numbers must be unchanged from the whole-tree view; got:\n{out}"
    );
    assert!(
        !out.contains("Antiquity") && !out.contains("Modern Period"),
        "branches outside prehistory must not render; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the `--subtree` fixture. Locks `data.subtree_of` and
/// `data.requested_node` both present (same iri), and `data.nodes`/`data.depth`
/// narrowed to the branch (4 nodes, 3 levels — branch-relative, matches
/// `count_and_depth(Some(prehistory))`).
#[test]
fn vocabulary_describe_json_subtree() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.vocabulary_describe(&subtree_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("subtree json must be valid JSON");
    assert_eq!(parsed["data"]["subtree_of"], PREHISTORY_IRI);
    assert_eq!(parsed["data"]["requested_node"], PREHISTORY_IRI);
    assert_eq!(
        parsed["data"]["nodes"], 4,
        "subtree node count must be branch-relative (4); got: {}",
        parsed["data"]["nodes"]
    );
    assert_eq!(
        parsed["data"]["depth"], 3,
        "subtree depth must be branch-relative (3); got: {}",
        parsed["data"]["depth"]
    );
    insta::assert_snapshot!(out);
}

// ── case 4: --subtree on a leaf (bronzeAge) ──────────────────────────────────

/// Prose render of the leaf-subtree fixture. D14b: the addressed leaf is
/// included as its own one-node branch — exactly one node line renders, and
/// the header shows whatever `count_and_depth(Some(leaf))` actually produces
/// (locked by the snapshot, not hardcoded here).
#[test]
fn vocabulary_describe_prose_subtree_leaf() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabulary_describe(&leaf_subtree_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert_eq!(
        leaf_subtree_detail().node_count,
        1,
        "sanity: bronzeAge's own branch must be exactly 1 node"
    );
    assert_eq!(
        leaf_subtree_detail().depth,
        1,
        "sanity: bronzeAge's own branch must be exactly 1 level"
    );
    insta::assert_snapshot!(out);
}
