//! Integration tests for `dsp docs` — the embedded documentation command.
//!
//! Topic *bodies* are deliberately NOT snapshotted: they are documentation prose
//! that is edited often, so per-body snapshots would be brittle and carry no
//! output-contract value (plan 018 D5 — a documented exception to ADR-0009's
//! "every (noun, verb, format) cell gets a snapshot"). These tests instead cover
//! the stable contract surface: the topic list, topic resolution across all nine
//! topics, and the not-found / did-you-mean error.

use assert_cmd::Command;

/// Helper: `dsp` with deterministic terminal settings (mirrors `tests/cli.rs`).
fn dsp() -> Command {
    let mut cmd = Command::cargo_bin("dsp").unwrap();
    cmd.env("TERM", "dumb")
        .env("COLUMNS", "100")
        // Belt-and-suspenders: these tests already run non-TTY (piped stderr), so
        // the update-check gate is already closed, but this guarantees no real
        // network call/flakiness even under an unusual terminal setup (plan
        // 031-update-check, Step 6).
        .env("DSP_NO_UPDATE_CHECK", "1");
    cmd
}

/// The catalog, in display order. Kept in lockstep with `TOPICS` in
/// `src/actions/docs.rs`; `docs_lists_all_topics` fails loudly if they drift.
const TOPIC_NAMES: &[&str] = &[
    "dsp-cli",
    "dsp",
    "concepts",
    "identifiers",
    "connecting",
    "output",
    "workflows",
    "errors",
    "dsp-tools",
    "sparql",
];

#[test]
fn docs_lists_all_topics() {
    let output = dsp().arg("docs").assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    for name in TOPIC_NAMES {
        assert!(text.contains(name), "topic list missing '{name}'");
    }
    insta::assert_snapshot!("docs_list", text);
}

#[test]
fn docs_prints_topic_body_to_stdout() {
    let output = dsp().args(["docs", "concepts"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    assert!(text.starts_with("# "), "topic body should be raw markdown");
    assert!(text.contains("data-model"), "concepts should mention data-model");
}

#[test]
fn docs_every_topic_resolves() {
    for name in TOPIC_NAMES {
        dsp().args(["docs", name]).assert().success();
    }
}

#[test]
fn docs_unknown_topic_exits_one_with_suggestion() {
    let assert = dsp().args(["docs", "concept"]).assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("Did you mean 'concepts'?"), "stderr was: {stderr}");
    insta::assert_snapshot!("docs_not_found_with_suggestion", stderr);
}

#[test]
fn docs_far_unknown_topic_exits_one_without_suggestion() {
    let assert = dsp().args(["docs", "xyzzy"]).assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("no documentation topic named 'xyzzy'"));
    assert!(!stderr.contains("Did you mean"), "stderr was: {stderr}");
}

// ── dsp docs -j ───────────────────────────────────────────────────────────────

/// `dsp docs -j` emits a machine-readable JSON topic index and exits 0.
/// Snapshot covers the full output shape (plan 020 step 6 test plan).
#[test]
fn docs_json_flag_emits_topic_index() {
    let output = dsp().args(["docs", "-j"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("docs_json_index", text);
}

/// `_meta` must be the very first key in the JSON output (ADR-0003, plan 020 D4).
/// The assertion is byte-position: no leading whitespace is permitted before
/// `{"_meta"`. `trim_start()` is intentionally absent — that would allow the
/// contract to be silently weakened by leading whitespace or a BOM.
#[test]
fn docs_json_meta_is_first_key() {
    let output = dsp().args(["docs", "-j"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.starts_with(r#"{"_meta""#),
        "_meta must be the first key at byte position 0; got: {text}"
    );
}

/// Every TOPICS entry must appear in the JSON output with its name and summary
/// but without any body content.
#[test]
fn docs_json_contains_all_topics_name_and_summary_no_body() {
    let output = dsp().args(["docs", "-j"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text.trim()).unwrap();

    let data = parsed["data"].as_array().expect("data must be an array");
    assert_eq!(data.len(), TOPIC_NAMES.len(), "data array length must match TOPIC_NAMES");

    for (i, name) in TOPIC_NAMES.iter().enumerate() {
        assert_eq!(data[i]["name"], *name, "name mismatch at index {i}");
        // summary must be non-empty string
        assert!(
            data[i]["summary"].as_str().is_some_and(|s| !s.is_empty()),
            "summary must be non-empty for topic {name}"
        );
        // no body field
        assert!(
            data[i].get("body").is_none(),
            "body must not appear in JSON index for topic {name}"
        );
    }
}

/// `dsp docs <topic> -j` exits 2 (clap-level conflict).
#[test]
fn docs_topic_and_json_conflicts_exits_two() {
    dsp().args(["docs", "concepts", "-j"]).assert().failure().code(2);
}

/// `dsp docs -j --pager` exits 2 (clap-level conflict).
#[test]
fn docs_json_and_pager_conflicts_exits_two() {
    dsp().args(["docs", "-j", "--pager"]).assert().failure().code(2);
}
