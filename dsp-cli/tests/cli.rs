//! Integration tests for the `dsp` binary CLI surface.
//!
//! This file contains two kinds of tests:
//!
//! 1. **Help-text snapshot tests**: Capture `--help` output at every level of the command tree to
//!    detect unintended regressions. For an AI-agent CLI, the `--help` text *is* the contract —
//!    snapshotting every level catches silent vocabulary drift, flag removal, and changed verb
//!    lists. Baselines are accepted once with `cargo insta accept` and reviewed like any diff in
//!    future PRs.
//!
//!    NOTE: `dsp --version` is intentionally NOT snapshotted — every release
//!    bumps the version string, causing unnecessary review noise without
//!    catching anything meaningful.
//!
//!    NOTE: Snapshot drift on version bump is expected for the `help_top`
//!    snapshot (it includes the crate version). Accept the diff after each
//!    `Cargo.toml` version bump.
//!
//! 2. **Dispatch smoke tests**: Verify that dispatch wiring reaches the correct action and that
//!    usage errors surface at exit code 2. After Phase 4, the `dsp vre project list` smoke test
//!    asserts that a missing `--server` yields exit code 2 (Config::resolve is now reached, unlike
//!    the Phase 3 stub).
//!
//! NOTE: `dsp docs connecting` and `dsp docs concepts` are referenced in
//! `--help` "See also:" lines. As of Phase 6 these topics ship as real embedded
//! documentation (`dsp docs <topic>`); the functional behaviour of `dsp docs`
//! itself is exercised in `tests/docs.rs`.

// `support` holds shared test helpers (currently just `MockDspClient`). Rust only
// compiles subdirectory modules under `tests/` when a top-level test file declares
// them; this `mod` line is what makes `tests/support/mod.rs` actually build.
// The `#[allow(unused_imports)]` silences the warning until Phase 3 lands the
// first consumer in `tests/actions/`.
#[allow(unused_imports)]
mod support;

use assert_cmd::Command;
use tempfile::TempDir;

/// Helper: create a `Command` for `dsp` with deterministic terminal settings.
///
/// `max_term_width = 100` in the clap derive is the primary determinism
/// mechanism. `TERM=dumb` and `COLUMNS=100` are belt-and-braces for the
/// test harness; all three are applied concurrently.
fn dsp() -> Command {
    let mut cmd = Command::cargo_bin("dsp").unwrap();
    cmd.env("TERM", "dumb")
        .env("COLUMNS", "100")
        // Belt-and-suspenders: these tests already run non-TTY (piped stderr), so
        // the update-check gate is already closed, but this guarantees no real
        // network call/flakiness even under an unusual terminal setup (plan
        // 031-update-check, Step 6).
        .env("DSP_NO_UPDATE_CHECK", "1")
        // `TERM=dumb` alone does not stop clap from emitting ANSI colour when
        // the caller's environment forces it — `NO_COLOR` and removing both
        // `CLICOLOR` variables closes that gap.
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("CLICOLOR");
    cmd
}

// ── Help-text snapshot tests ─────────────────────────────────────────────────

#[test]
fn help_top() {
    let output = dsp().arg("--help").assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_top", text);
}

#[test]
fn help_no_args() {
    // Clap writes the "missing subcommand" error to *stderr* and exits with
    // code 2 when no subcommand is provided.
    let output = dsp().assert().failure().code(2).get_output().stderr.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_no_args", text);
}

#[test]
fn help_auth() {
    let output = dsp().args(["auth", "--help"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth", text);
}

#[test]
fn help_auth_login() {
    let output = dsp()
        .args(["auth", "login", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth_login", text);
}

#[test]
fn help_auth_status() {
    let output = dsp()
        .args(["auth", "status", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth_status", text);
}

#[test]
fn help_auth_logout() {
    let output = dsp()
        .args(["auth", "logout", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth_logout", text);
}

#[test]
fn help_auth_set_token() {
    let output = dsp()
        .args(["auth", "set-token", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth_set_token", text);
}

#[test]
fn help_auth_token() {
    let output = dsp()
        .args(["auth", "token", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_auth_token", text);
}

#[test]
fn help_vre() {
    let output = dsp().args(["vre", "--help"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre", text);
}

#[test]
fn help_vre_project() {
    let output = dsp()
        .args(["vre", "project", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_project", text);
}

#[test]
fn help_vre_project_list() {
    let output = dsp()
        .args(["vre", "project", "list", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_project_list", text);
}

#[test]
fn help_vre_project_describe() {
    let output = dsp()
        .args(["vre", "project", "describe", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_project_describe", text);
}

#[test]
fn help_vre_project_dump() {
    let output = dsp()
        .args(["vre", "project", "dump", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_project_dump", text);
}

#[test]
fn help_vre_data_model() {
    let output = dsp()
        .args(["vre", "data-model", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_data_model", text);
}

#[test]
fn help_vre_data_model_list() {
    let output = dsp()
        .args(["vre", "data-model", "list", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_data_model_list", text);
}

#[test]
fn help_vre_data_model_describe() {
    let output = dsp()
        .args(["vre", "data-model", "describe", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_data_model_describe", text);
}

#[test]
fn help_vre_data_model_structure() {
    let output = dsp()
        .args(["vre", "data-model", "structure", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_data_model_structure", text);
}

#[test]
fn help_vre_resource_type() {
    let output = dsp()
        .args(["vre", "resource-type", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource_type", text);
}

#[test]
fn help_vre_resource_type_list() {
    let output = dsp()
        .args(["vre", "resource-type", "list", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource_type_list", text);
}

#[test]
fn help_vre_resource_type_describe() {
    let output = dsp()
        .args(["vre", "resource-type", "describe", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource_type_describe", text);
}

#[test]
fn help_vre_resource() {
    let output = dsp()
        .args(["vre", "resource", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource", text);
}

#[test]
fn help_vre_resource_list() {
    let output = dsp()
        .args(["vre", "resource", "list", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource_list", text);
}

#[test]
fn help_vre_resource_describe() {
    let output = dsp()
        .args(["vre", "resource", "describe", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_resource_describe", text);
}

#[test]
fn help_vre_vocabulary() {
    let output = dsp()
        .args(["vre", "vocabulary", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_vocabulary", text);
}

#[test]
fn help_vre_vocabulary_list() {
    let output = dsp()
        .args(["vre", "vocabulary", "list", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_vocabulary_list", text);
}

#[test]
fn help_vre_vocabulary_describe() {
    let output = dsp()
        .args(["vre", "vocabulary", "describe", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_vocabulary_describe", text);
}

#[test]
fn help_vre_sparql() {
    let output = dsp()
        .args(["vre", "sparql", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_sparql", text);
}

#[test]
fn help_vre_sparql_query() {
    let output = dsp()
        .args(["vre", "sparql", "query", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_vre_sparql_query", text);
}

#[test]
fn help_docs() {
    let output = dsp().args(["docs", "--help"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    insta::assert_snapshot!("help_docs", text);
}

// ── Dispatch smoke test ───────────────────────────────────────────────────────

#[test]
fn dispatch_vre_project_list_no_server_exits_usage() {
    // `dsp vre project list` with no --server and no DSP_SERVER env var must
    // fail with exit code 2 (usage). The Config layer requires a server and
    // returns Diagnostic::Usage when none is configured, which maps to exit 2.
    // Confirms Phase 4 dispatch wiring: list now calls Config::resolve, unlike
    // the Phase 3 stub which returned NotImplemented before reaching Config.
    dsp()
        .args(["vre", "project", "list"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dispatch_vre_project_dump_no_server_exits_usage() {
    // `dsp vre project dump` with no --server and no DSP_SERVER env var must
    // fail with exit code 2 (usage). The Config layer requires a server and
    // returns Diagnostic::Usage when none is configured, which maps to exit 2.
    // DSP_SERVER is explicitly removed to guarantee the test environment is clean.
    dsp()
        .args(["vre", "project", "dump"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dump_replace_and_delete_together_exits_usage() {
    // clap enforces conflicts_with at parse time (before server resolution),
    // so --replace --delete must produce exit code 2 regardless of --server.
    dsp()
        .args(["vre", "project", "dump", "--replace", "--delete"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dump_delete_with_output_exits_usage() {
    // --delete conflicts_with_all includes "output"; clap rejects this at parse time.
    dsp()
        .args(["vre", "project", "dump", "--delete", "--output", "/tmp/x.zip"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dump_delete_with_skip_assets_exits_usage() {
    // --delete conflicts_with_all includes "skip_assets"; clap rejects at parse time.
    dsp()
        .args(["vre", "project", "dump", "--delete", "--skip-assets"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dump_delete_with_force_exits_usage() {
    // --delete conflicts_with_all includes "force"; clap rejects at parse time.
    dsp()
        .args(["vre", "project", "dump", "--delete", "--force"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dump_delete_with_cleanup_exits_usage() {
    // --delete conflicts_with_all includes "cleanup"; clap rejects at parse time.
    dsp()
        .args(["vre", "project", "dump", "--delete", "--cleanup"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

// ── plan 020: FormatArgs flag validation CLI-level tests ─────────────────────

#[test]
fn columns_with_default_prose_exits_usage() {
    // --columns with the default format (prose) must exit 2.
    //
    // In lib.rs, the call order is: Config::resolve → table_options → renderer
    // construction → action. A missing --server would fail at Config::resolve
    // (also exit 2, but for the wrong reason). We pass a syntactically-valid
    // server URL so Config::resolve succeeds; the exit-2 here comes from
    // table_options rejecting --columns on a non-tabular format.
    //
    // Regression guard (plan 032): prose-format top-level errors are UNCHANGED —
    // still `Error: {diag}` on stderr, stdout empty.
    let output = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "https://api.example.invalid",
            "--columns",
            "shortcode",
        ])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(
        text.contains("--columns"),
        "--columns must appear in the usage error; got: {text}"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.is_empty(), "expected empty stdout; got: {stdout}");
}

#[test]
fn no_header_with_json_exits_usage() {
    // --no-header -j → resolved format is json → exit 2 from table_options.
    //
    // We pass --server so Config::resolve (called first in lib.rs) succeeds;
    // table_options then rejects --no-header on a non-csv/tsv format.
    //
    // Since plan 032 (top-level error routing), a top-level error under -j emits
    // the dsp-cli/ADR-0012 JSON error envelope to *stdout* (not stderr), with stderr empty.
    let output = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "https://api.example.invalid",
            "--no-header",
            "-j",
        ])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "expected empty stderr; got: {stderr}");
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout did not parse as JSON: {e}; got: {stdout}"));
    assert_eq!(parsed["error"]["kind"], "usage");
    assert_eq!(parsed["_meta"]["exit_code"], 2);
    let message = parsed["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("--no-header"),
        "--no-header must appear in the usage error message; got: {message}"
    );
}

#[test]
fn json_error_envelope_to_stdout_no_server() {
    // `dsp vre project list -j` with no --server and no DSP_SERVER → Config::resolve
    // fails with Diagnostic::Usage before table_options is ever reached. Under -j the
    // JSON error envelope goes to stdout (single NDJSON line), stderr stays empty, and
    // _meta carries no server/auth keys since Config::resolve never produced a server
    // label (main.rs's error-path re-resolution also fails, same as the primary run).
    let output = dsp()
        .args(["vre", "project", "list", "-j"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "expected empty stderr; got: {stderr}");

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected exactly one NDJSON line on stdout; got: {stdout}");
    let parsed: serde_json::Value = serde_json::from_str(lines[0])
        .unwrap_or_else(|e| panic!("stdout line did not parse as JSON: {e}; got: {stdout}"));

    assert_eq!(parsed["error"]["kind"], "usage");
    let message = parsed["error"]["message"].as_str().expect("error.message must be a string");
    assert!(!message.is_empty(), "error.message must be non-empty; got: {parsed}");

    let meta = parsed["_meta"].as_object().expect("_meta must be a JSON object");
    assert!(
        !meta.contains_key("server"),
        "_meta must not have a server key when Config::resolve failed; got: {parsed}"
    );
    assert!(
        !meta.contains_key("auth"),
        "_meta must not have an auth key at the top level (D3); got: {parsed}"
    );
    assert_eq!(parsed["_meta"]["exit_code"], 2);
}

#[test]
fn json_error_envelope_populates_server() {
    // `dsp vre project list -s dev --no-header -j` — `dev` is a resolvable SHORTCUTS
    // entry (src/config/mod.rs), so Config::resolve succeeds without any network call.
    // The usage error here comes from table_options rejecting --no-header on json
    // format, AFTER the server resolved — so _meta.server must be populated.
    let output = dsp()
        .args(["vre", "project", "list", "-s", "dev", "--no-header", "-j"])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout did not parse as JSON: {e}; got: {stdout}"));

    let server = parsed["_meta"]["server"]
        .as_str()
        .expect("_meta.server must be present and a string");
    assert!(!server.is_empty(), "_meta.server must be non-empty");
    assert_eq!(parsed["error"]["kind"], "usage");
    assert_eq!(parsed["_meta"]["exit_code"], 2);
}

#[test]
fn columns_blank_segment_exits_usage() {
    // --columns a,,b has an empty segment → exit 2 from table_options.
    //
    // We pass --server so Config::resolve (called first in lib.rs) succeeds;
    // table_options then rejects the blank segment.
    let stderr = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "https://api.example.invalid",
            "--columns",
            "a,,b",
            "--format",
            "csv",
        ])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(stderr).unwrap();
    assert!(
        text.contains("--columns"),
        "--columns must appear in the usage error; got: {text}"
    );
}

#[test]
fn columns_duplicate_exits_usage() {
    // --columns iri,iri has a duplicate → exit 2 from table_options.
    //
    // We pass --server so Config::resolve (called first in lib.rs) succeeds;
    // table_options then rejects the duplicate column name.
    let stderr = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "https://api.example.invalid",
            "--columns",
            "iri,iri",
            "--format",
            "csv",
        ])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(stderr).unwrap();
    assert!(
        text.contains("--columns"),
        "--columns must appear in the usage error; got: {text}"
    );
}

#[test]
fn resource_list_page_and_all_together_exits_usage() {
    // --page and --all are mutually exclusive (clap conflicts_with).
    // clap enforces conflicts_with at parse time (before server resolution),
    // so this must produce exit code 2 regardless of --server.
    dsp()
        .args([
            "vre",
            "resource",
            "list",
            "--server",
            "https://api.example.invalid",
            "--project",
            "0803",
            "--resource-type",
            "page",
            "--page",
            "1",
            "--all",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn docs_topic_with_json_flag_exits_usage() {
    // `dsp docs output -j` — -j conflicts_with_all includes topic (positional);
    // clap rejects at parse time with exit code 2.
    dsp().args(["docs", "output", "-j"]).assert().failure().code(2);
}

#[test]
fn docs_json_with_pager_exits_usage() {
    // `dsp docs -j --pager` — clap conflicts_with rejects at parse time.
    dsp().args(["docs", "-j", "--pager"]).assert().failure().code(2);
}

// ── sparql query parse tests (plan 035, Step 3) ────────────────────────────────

#[test]
fn sparql_query_parses_with_query_flag() {
    // Parsing succeeds up through Config::resolve — no --server, so it must
    // exit 2 (usage), never a parse-level clap error (exit 2 either way, but
    // proves the flags themselves are accepted).
    dsp()
        .args(["vre", "sparql", "query", "--query", "SELECT * WHERE { ?s ?p ?o }"])
        .env_remove("DSP_SERVER")
        .env_remove("DSP_TOKEN")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn sparql_query_and_query_file_together_exits_usage() {
    // --query and --query-file are mutually exclusive (D5, clap conflicts_with).
    dsp()
        .args([
            "vre",
            "sparql",
            "query",
            "--query",
            "SELECT * WHERE { ?s ?p ?o }",
            "--query-file",
            "query.rq",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn sparql_query_leading_dash_query_file_is_accepted_in_the_space_form() {
    // `allow_hyphen_values` means the SPACE form parses too — verified
    // 2026-08-07. The earlier version of this test asserted only `code(2)`,
    // which a clap parse rejection and a failed file read satisfy equally, so
    // it could not tell them apart and its comment described behaviour that
    // does not happen. Assert on stderr: the value reached the action and the
    // failure is the file read, NOT a clap `unexpected argument`.
    // `--server` must be supplied: `Config::resolve` runs before the file read,
    // so without it the run stops at "no server specified" and never reaches the
    // behaviour under test. The address is never contacted — the file read fails
    // first.
    dsp()
        .args([
            "vre",
            "sparql",
            "query",
            "-s",
            "http://127.0.0.1:1",
            "--query-file",
            "-foo.rq",
        ])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::prelude::PredicateBooleanExt::and(
            predicates::str::contains("could not read --query-file"),
            predicates::prelude::PredicateBooleanExt::not(predicates::str::contains("unexpected argument")),
        ));
}

#[test]
fn sparql_query_dash_equals_query_file_is_accepted_by_the_parser() {
    // The `=` form parses (allow_hyphen_values); the run then fails past
    // parsing (no --server / unreadable file), never as a clap usage error
    // about the flag itself. Exit code 2 either way (Usage), so this only
    // proves the parser accepted the leading-dash value — a parse-level
    // rejection would look identical, so the real proof is that this and
    // the space-form test above both compile and run without a clap panic.
    dsp()
        .args(["vre", "sparql", "query", "--query-file=-foo.rq"])
        .env_remove("DSP_SERVER")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn sparql_query_rejects_format_flag() {
    // D2's regression guard: no `--format`/`-j`/`-l` on this leaf — copied
    // from the `TokenArgs` precedent. Proves `FormatArgs` is NOT flattened.
    dsp()
        .args(["vre", "sparql", "query", "--query", "SELECT * WHERE { ?s ?p ?o }", "-j"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn sparql_query_rejects_columns_flag() {
    // D2's regression guard: no --columns either (no tabular formats exist).
    dsp()
        .args([
            "vre",
            "sparql",
            "query",
            "--query",
            "SELECT * WHERE { ?s ?p ?o }",
            "--columns",
            "status",
        ])
        .assert()
        .failure()
        .code(2);
}

// ── --server scheme validation (5b) ──────────────────────────────────────────
//
// `Config::resolve` refuses a non-local `--server` on plain http:// before any
// network call, in all five output formats. `dsp vre project list` reaches
// Config::resolve first (see `dispatch_vre_project_list_no_server_exits_usage`
// above) and supports every `FormatArgs` shortcut, so it doubles as the cell
// for this refusal the way it already does for the missing-server usage error.

#[test]
fn insecure_http_server_refused_prose() {
    let output = dsp()
        .args(["vre", "project", "list", "--server", "http://api.example.org"])
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "prose refusal must not write to stdout");
    let stderr = String::from_utf8(output.stderr).unwrap();
    insta::assert_snapshot!("insecure_http_refused_prose", stderr);
}

#[test]
fn insecure_http_server_refused_json() {
    let output = dsp()
        .args(["vre", "project", "list", "--server", "http://api.example.org", "-j"])
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.is_empty(),
        "json refusal must go to stdout, not stderr; stderr was: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout did not parse as JSON: {e}; got: {stdout}"));
    assert_eq!(parsed["error"]["kind"], "usage");
    insta::assert_snapshot!("insecure_http_refused_json", stdout);
}

#[test]
fn insecure_http_server_refused_csv() {
    let output = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "http://api.example.org",
            "--format",
            "csv",
        ])
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "csv refusal must not write to stdout");
    let stderr = String::from_utf8(output.stderr).unwrap();
    insta::assert_snapshot!("insecure_http_refused_csv", stderr);
}

#[test]
fn insecure_http_server_refused_tsv() {
    let output = dsp()
        .args([
            "vre",
            "project",
            "list",
            "--server",
            "http://api.example.org",
            "--format",
            "tsv",
        ])
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "tsv refusal must not write to stdout");
    let stderr = String::from_utf8(output.stderr).unwrap();
    insta::assert_snapshot!("insecure_http_refused_tsv", stderr);
}

#[test]
fn insecure_http_server_refused_lines() {
    let output = dsp()
        .args(["vre", "project", "list", "--server", "http://api.example.org", "-l"])
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "lines refusal must not write to stdout");
    let stderr = String::from_utf8(output.stderr).unwrap();
    insta::assert_snapshot!("insecure_http_refused_lines", stderr);
}

// ── --server control-character refusal (1a) ──────────────────────────────────
//
// `dsp auth status` never makes a network call (pure auth-cache read), so it
// exercises Config::resolve's control-character refusal with no live server
// needed. HOME is pointed at a fresh TempDir, same as the
// --allow-insecure-server precedence tests below.

#[test]
fn control_character_in_server_refused_prose() {
    let home = TempDir::new().unwrap();
    let output = dsp()
        .env("HOME", home.path())
        .args(["auth", "status", "--server", "https://x.example/\u{1b}[31m"])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "prose refusal must not write to stdout");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains('\u{1b}'), "ESC leaked into diagnostic: {stderr:?}");
    insta::assert_snapshot!("control_character_in_server_refused_prose", stderr);
}

// ── --allow-insecure-server flag / DSP_ALLOW_INSECURE_SERVER env precedence ──
//
// `dsp auth status` never makes a network call (pure auth-cache read), so it
// exercises the override end-to-end (clap parsing -> Cli::allow_insecure_server
// -> Config::resolve) with no live server needed. HOME is pointed at a fresh
// TempDir per test so these never touch (or depend on) the real
// ~/.config/dsp-cli/auth.toml.

#[test]
fn allow_insecure_flag_permits_insecure_http() {
    let home = TempDir::new().unwrap();
    dsp()
        .env("HOME", home.path())
        .env_remove("DSP_ALLOW_INSECURE_SERVER")
        .args([
            "auth",
            "status",
            "--server",
            "http://api.example.org",
            "--allow-insecure-server",
        ])
        .assert()
        .success();
}

#[test]
fn allow_insecure_env_var_1_permits_insecure_http() {
    // DSP_ALLOW_INSECURE_SERVER=1 alone, no flag — proves the env tier works
    // and that `1` parses (BoolishValueParser; the default clap bool parser
    // accepts only "true"/"false" and would reject "1").
    let home = TempDir::new().unwrap();
    dsp()
        .env("HOME", home.path())
        .env("DSP_ALLOW_INSECURE_SERVER", "1")
        .args(["auth", "status", "--server", "http://api.example.org"])
        .assert()
        .success();
}

#[test]
fn allow_insecure_env_var_0_still_refuses() {
    let home = TempDir::new().unwrap();
    dsp()
        .env("HOME", home.path())
        .env("DSP_ALLOW_INSECURE_SERVER", "0")
        .args(["auth", "status", "--server", "http://api.example.org"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn allow_insecure_flag_wins_over_conflicting_env_var() {
    // Flag-before-env precedence (the tier ordering dsp-cli/ADR-0007 sets for
    // every other setting, applied here too): an explicit
    // --allow-insecure-server overrides a DSP_ALLOW_INSECURE_SERVER=0 that
    // would otherwise refuse.
    let home = TempDir::new().unwrap();
    dsp()
        .env("HOME", home.path())
        .env("DSP_ALLOW_INSECURE_SERVER", "0")
        .args([
            "auth",
            "status",
            "--server",
            "http://api.example.org",
            "--allow-insecure-server",
        ])
        .assert()
        .success();
}
