//! Snapshot tests for `dsp vre resource-type describe` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (beol-like manuscript): exercises the full feature set — label, Extends,
//!   still-image Representation, IRI, own-DM fields (text, link), and cross-DM fields (link + uri
//!   from biblio). Shared by prose, json, lines, csv, tsv cells.
//! - **Empty fixture**: zero fields. Tests the prose `Fields (0)` branch.
//! - **Include-builtins fixture**: adds `is_builtin=true` fields (arkUrl, hasStillImageFileValue)
//!   to the main fixture to exercise the `(built-in)` marker.
//! - **Degraded-field fixture**: a field with `ValueType::Other("—")` and cross-DM `data_model:
//!   Some("biblio")`, testing best-effort render and `[from biblio]` tag.
//! - **not_found json**: `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`
//!   — locks the action-built NotFound envelope rendered by the generic `diagnostic` method.
//!
//! Determinism: these tests call `Renderer::resource_type_describe(&detail, &meta)`
//! **directly** with a hand-built `MetaContext` / `ResourceTypeDetail`. They never go
//! through `run_describe_impl`, which reads the real `DSP_TOKEN` env var. Action /
//! auth-resolution logic is covered by the in-module action tests; these layer-4
//! snapshot tests cover rendering only.
//! See `docs/src/dsp-cli/testing-strategy.md` and learning from plan 010.
//!
//! dsp-cli/ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` in this
//! file or any .snap it generates. (IRI strings contain "/ontology/" as data —
//! the documented exception per review-guidelines.md.)

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{Cardinality, Field, Representation, ResourceTypeDetail, ValueType};
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
const SERVER: &str = "https://api.dasch.swiss";

// ── MetaContext helpers ────────────────────────────────────────────────────────

/// Anonymous `MetaContext` — mirrors what `run_describe_impl` builds when no
/// token is present.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

/// Anonymous `MetaContext` with `count_caveat` set — mirrors what
/// `run_describe_impl` builds when `--count` is passed. Wording is copied
/// verbatim from `COUNT_CAVEAT` in `src/actions/vre/resource_type.rs` so the
/// two stay in sync.
fn count_caveat_meta() -> MetaContext {
    MetaContext {
        count_caveat: Some(
            "counts include resources you may not be permitted to see and exclude deleted resources.".to_string(),
        ),
        ..anon_meta()
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Main fixture — a beol-like manuscript resource-type exercising:
/// - `label: Some`, `representation: Some(StillImage)`, `super_types: ["writtenSource"]`
/// - Own-DM text field (title, 1-n)
/// - Own-DM link field (hasAuthor → person, 0-n)
/// - Own-DM text field (hasText, 0-1)
/// - Cross-DM link field from biblio (isPartOfCollection → Collection, 0-n)  [from biblio]
/// - Cross-DM uri field from biblio (hasURI, 0-n)  [from biblio]
///
/// Real beol/biblio IRIs grounded in the live API.
/// Fields sorted as the HTTP client would deliver them (guiOrder → name).
fn manuscript_detail() -> ResourceTypeDetail {
    ResourceTypeDetail {
        name: "manuscript".to_string(),
        iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".to_string(),
        label: Some("Manuscript".to_string()),
        data_model: "beol".to_string(),
        representation: Some(Representation::StillImage),
        super_types: vec!["writtenSource".to_string()],
        fields: vec![
            Field {
                name: "title".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#title".to_string(),
                label: Some("Title".to_string()),
                value_type: ValueType::Text,
                link_target: None,
                cardinality: Cardinality::OneOrMore,
                is_builtin: false,
                data_model: Some("beol".to_string()),
            },
            Field {
                name: "hasAuthor".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasAuthor".to_string(),
                label: Some("Author".to_string()),
                value_type: ValueType::Link,
                link_target: Some("person".to_string()),
                cardinality: Cardinality::ZeroOrMore,
                is_builtin: false,
                data_model: Some("beol".to_string()),
            },
            Field {
                name: "hasText".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasText".to_string(),
                label: Some("Text".to_string()),
                value_type: ValueType::Text,
                link_target: None,
                cardinality: Cardinality::ZeroOrOne,
                is_builtin: false,
                data_model: Some("beol".to_string()),
            },
            Field {
                name: "isPartOfCollection".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2#isPartOfCollection".to_string(),
                label: Some("is part of".to_string()),
                value_type: ValueType::Link,
                link_target: Some("Collection".to_string()),
                cardinality: Cardinality::ZeroOrMore,
                is_builtin: false,
                data_model: Some("biblio".to_string()),
            },
            Field {
                name: "hasURI".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2#hasURI".to_string(),
                label: Some("URI".to_string()),
                value_type: ValueType::Uri,
                link_target: None,
                cardinality: Cardinality::ZeroOrMore,
                is_builtin: false,
                data_model: Some("biblio".to_string()),
            },
        ],
        count: None,
    }
}

/// Main fixture with `--count` merged in — the same manuscript detail as
/// `manuscript_detail()`, but with `count: Some(1893)` (a realistic instance
/// count for an `incunabula:page`-like resource-type, per plan 030's worked
/// example).
fn manuscript_detail_with_count() -> ResourceTypeDetail {
    let mut detail = manuscript_detail();
    detail.count = Some(1893);
    detail
}

/// Empty fixture — zero fields, no label, no representation, no super_types.
/// Locks the prose `Fields (0)` branch, json empty fields array.
fn empty_detail() -> ResourceTypeDetail {
    ResourceTypeDetail {
        name: "EmptyThing".to_string(),
        iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#EmptyThing".to_string(),
        label: None,
        data_model: "minimal".to_string(),
        representation: None,
        super_types: vec![],
        fields: vec![],
        count: None,
    }
}

/// Include-builtins fixture — the main manuscript fixture plus two built-in fields.
/// Exercises the `(built-in)` marker in prose and the `is_builtin: true` JSON/tabular columns.
fn manuscript_with_builtins_detail() -> ResourceTypeDetail {
    let mut detail = manuscript_detail();
    detail.fields.push(Field {
        name: "arkUrl".to_string(),
        iri: "http://api.knora.org/ontology/knora-api/v2#arkUrl".to_string(),
        label: None,
        value_type: ValueType::Uri,
        link_target: None,
        cardinality: Cardinality::One,
        is_builtin: true,
        data_model: None,
    });
    detail.fields.push(Field {
        name: "hasStillImageFileValue".to_string(),
        iri: "http://api.knora.org/ontology/knora-api/v2#hasStillImageFileValue".to_string(),
        label: None,
        value_type: ValueType::StillImage,
        link_target: None,
        cardinality: Cardinality::One,
        is_builtin: true,
        data_model: None,
    });
    detail
}

/// Degraded-field fixture — a cross-DM field left best-effort (value_type Other("—"),
/// label None, data_model Some("biblio")). Exercises the `[from biblio]` tag still
/// appearing even when value-type is unknown.
fn degraded_field_detail() -> ResourceTypeDetail {
    ResourceTypeDetail {
        name: "manuscript".to_string(),
        iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".to_string(),
        label: Some("Manuscript".to_string()),
        data_model: "beol".to_string(),
        representation: None,
        super_types: vec!["writtenSource".to_string()],
        fields: vec![
            Field {
                name: "title".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#title".to_string(),
                label: Some("Title".to_string()),
                value_type: ValueType::Text,
                link_target: None,
                cardinality: Cardinality::OneOrMore,
                is_builtin: false,
                data_model: Some("beol".to_string()),
            },
            Field {
                // best-effort cross-DM field: sibling fetch failed → value_type Other("—"),
                // label None, but data_model still Some("biblio") so source tag renders.
                name: "unknownBiblioField".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2#unknownBiblioField".to_string(),
                label: None,
                value_type: ValueType::Other("—".to_string()),
                link_target: None,
                cardinality: Cardinality::ZeroOrMore,
                is_builtin: false,
                data_model: Some("biblio".to_string()),
            },
        ],
        count: None,
    }
}

// ── main fixture × 5 formats ──────────────────────────────────────────────────

/// Prose render of the main fixture. Locks:
/// - Header `Resource-type: manuscript`
/// - Label/Extends/Representation/IRI/Data-model block (values aligned)
/// - `Fields (5):` sub-list with aligned `name  value-type  cardinality  label[ source-tag]`
/// - Link field with `→ person` arrow notation
/// - Cross-DM fields with `[from biblio]` tag, own-DM fields without
/// - dsp-cli/ADR-0007 footer `[anonymous on https://api.dasch.swiss]`
///
/// dsp-cli/ADR-0001 vocabulary guard: IRI lines containing "/ontology/" are the documented
/// exception.
#[test]
fn resource_type_describe_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Vocabulary guard (014 learning) — skip lines that contain an IRI field.
    for line in out.lines() {
        let lower = line.to_lowercase();
        // IRI-value exception: lines with "iri:" in them may contain "/ontology/".
        if lower.contains("iri:") {
            continue;
        }
        assert!(
            !lower.contains("export"),
            "prose must not contain 'export'; offending line: {line:?}\nfull output:\n{out}"
        );
        assert!(
            !lower.contains("class"),
            "prose must not contain 'class'; offending line: {line:?}\nfull output:\n{out}"
        );
        // "ontolog" prefix catches both "ontology" and "ontologies"; allowed only in IRI lines.
        assert!(
            !lower.contains("ontolog"),
            "prose must not contain 'ontolog'; offending line: {line:?}\nfull output:\n{out}"
        );
    }

    insta::assert_snapshot!(out);
}

/// JSON render of the main fixture. Locks:
/// - dsp-cli/ADR-0003 single-object `data` envelope with `_meta` first
/// - Key order: name, iri, label, data_model, representation, super_types, fields
/// - `representation: "still-image"` string
/// - `super_types: ["writtenSource"]` array
/// - `fields` array with per-field objects (name, iri, label, value_type, link_target, cardinality,
///   is_builtin, data_model)
/// - link field has `value_type: "link"` and `link_target: "person"`
/// - cross-DM fields carry `data_model: "biblio"`; own-DM fields carry `data_model: "beol"`
#[test]
fn resource_type_describe_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture. Locks:
/// - One row per field: name TAB iri (no header)
/// - Disclosure on stderr, not stdout
#[test]
fn resource_type_describe_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (data rows only).
    insta::assert_snapshot!("resource_type_describe_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_describe_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture. Locks:
/// - Header `name,value_type,link_target,cardinality,label,is_builtin,data_model`
/// - One data row per field; link fields have link_target populated
/// - Cross-DM fields have data_model="biblio"
/// - Disclosure on stderr, not stdout
#[test]
fn resource_type_describe_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_describe_csv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_describe_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture. Same column shape as CSV but tab-separated
/// and unquoted. Disclosure on stderr.
#[test]
fn resource_type_describe_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_describe_tsv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_describe_tsv_stderr", buf_to_string(&err_buf));
}

// ── main fixture with `--count` (plan 030) ────────────────────────────────────

/// Prose render of the main fixture with `--count`. Locks the `Instances:`
/// line landing right after `Data-model:` (the load-bearing placement decision
/// from the renderer step — see `src/render/prose.rs`'s `resource_type_describe`)
/// and the count_caveat appended to the dsp-cli/ADR-0007 footer.
#[test]
fn resource_type_describe_prose_with_count() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_type_describe(&manuscript_detail_with_count(), &count_caveat_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Structural assertion: Instances: must immediately follow Data-model:.
    let lines: Vec<&str> = out.lines().collect();
    let data_model_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("Data-model:"))
        .expect("Data-model: line must be present");
    assert!(
        lines[data_model_idx + 1].trim_start().starts_with("Instances:"),
        "Instances: line must immediately follow Data-model:; got:\n{out}"
    );
    assert!(
        out.contains("Instances:      1893"),
        "Instances line must show count 1893; got:\n{out}"
    );
    assert!(
        out.contains("counts include resources you may not be permitted to see"),
        "footer must carry count_caveat; got:\n{out}"
    );

    insta::assert_snapshot!(out);
}

/// JSON render of the main fixture with `--count`. Locks `data.count == 1893`
/// and `_meta.note` carrying the count_caveat text.
#[test]
fn resource_type_describe_json_with_count() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_type_describe(&manuscript_detail_with_count(), &count_caveat_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("with-count json must be valid JSON");
    assert_eq!(
        parsed["data"]["count"], 1893,
        "data.count must be 1893; got: {}",
        parsed["data"]["count"]
    );
    let note = parsed["_meta"]["note"]
        .as_str()
        .expect("_meta.note must be present and a string");
    assert!(!note.is_empty(), "_meta.note must be non-empty");

    insta::assert_snapshot!(out);
}

/// Lines/csv/tsv `resource-type describe` STDOUT is UNCHANGED by `--count`
/// (per plan 030: count is a resource-type-level scalar, no column fits the
/// per-field row shape). This test snapshots ONLY the stderr disclosure note
/// (which now carries the count_caveat) and asserts stdout is byte-identical
/// to the unflagged `resource_type_describe_lines` test's stdout — no
/// redundant stdout snapshot.
#[test]
fn resource_type_describe_lines_stderr_with_count() {
    let (unflagged_out_buf, unflagged_out_w) = shared_buf();
    let (unflagged_err_buf, unflagged_err_w) = shared_buf();
    let mut unflagged = LinesRenderer::with_writers(unflagged_out_w, unflagged_err_w);
    unflagged.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let _ = unflagged_err_buf; // only stdout is compared here

    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail_with_count(), &count_caveat_meta())
        .unwrap();

    assert_eq!(
        buf_to_string(&out_buf),
        buf_to_string(&unflagged_out_buf),
        "stdout must be byte-identical to the unflagged resource_type_describe_lines test"
    );
    insta::assert_snapshot!("resource_type_describe_lines_stderr_with_count", buf_to_string(&err_buf));
}

/// CSV `resource-type describe` STDOUT is unchanged by `--count` (see the
/// lines test above for rationale). Snapshots only the stderr disclosure note.
#[test]
fn resource_type_describe_csv_stderr_with_count() {
    let (unflagged_out_buf, unflagged_out_w) = shared_buf();
    let (unflagged_err_buf, unflagged_err_w) = shared_buf();
    let mut unflagged = CsvRenderer::with_writers(unflagged_out_w, unflagged_err_w);
    unflagged.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let _ = unflagged_err_buf;

    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail_with_count(), &count_caveat_meta())
        .unwrap();

    assert_eq!(
        buf_to_string(&out_buf),
        buf_to_string(&unflagged_out_buf),
        "stdout must be byte-identical to the unflagged resource_type_describe_csv test"
    );
    insta::assert_snapshot!("resource_type_describe_csv_stderr_with_count", buf_to_string(&err_buf));
}

/// TSV `resource-type describe` STDOUT is unchanged by `--count` (see the
/// lines test above for rationale). Snapshots only the stderr disclosure note.
#[test]
fn resource_type_describe_tsv_stderr_with_count() {
    let (unflagged_out_buf, unflagged_out_w) = shared_buf();
    let (unflagged_err_buf, unflagged_err_w) = shared_buf();
    let mut unflagged = TsvRenderer::with_writers(unflagged_out_w, unflagged_err_w);
    unflagged.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let _ = unflagged_err_buf;

    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail_with_count(), &count_caveat_meta())
        .unwrap();

    assert_eq!(
        buf_to_string(&out_buf),
        buf_to_string(&unflagged_out_buf),
        "stdout must be byte-identical to the unflagged resource_type_describe_tsv test"
    );
    insta::assert_snapshot!("resource_type_describe_tsv_stderr_with_count", buf_to_string(&err_buf));
}

// ── empty fixture ─────────────────────────────────────────────────────────────

/// Prose render of the empty fixture. Locks:
/// - `Label:` line omitted when None
/// - `Extends:` line omitted when super_types is empty
/// - `Representation:` line omitted when None
/// - `  Fields (0)` with NO trailing colon or rows
/// - dsp-cli/ADR-0007 footer still present
#[test]
fn resource_type_describe_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_type_describe(&empty_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting.
    assert!(out.contains("Fields (0)"), "prose empty must show 'Fields (0)'; got:\n{out}");
    assert!(
        !out.contains("Fields (0):"),
        "prose empty must NOT have trailing colon on 'Fields (0)'; got:\n{out}"
    );
    assert!(
        !out.contains("Label:"),
        "prose empty must omit Label: when label is None; got:\n{out}"
    );
    assert!(
        !out.contains("Extends:"),
        "prose empty must omit Extends: when super_types is empty; got:\n{out}"
    );
    assert!(
        !out.contains("Representation:"),
        "prose empty must omit Representation: when representation is None; got:\n{out}"
    );
    assert!(
        out.contains("[anonymous on"),
        "prose empty must still have disclosure footer; got:\n{out}"
    );

    // Vocabulary guard.
    for line in out.lines() {
        let lower = line.to_lowercase();
        if lower.contains("iri:") {
            continue;
        }
        assert!(
            !lower.contains("export"),
            "prose empty must not contain 'export'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("class"),
            "prose empty must not contain 'class'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("ontolog"),
            "prose empty must not contain 'ontolog'; offending line: {line:?}"
        );
    }

    insta::assert_snapshot!(out);
}

// ── include-builtins fixture ──────────────────────────────────────────────────

/// Prose render of the include-builtins fixture. Locks:
/// - Project fields render without `(built-in)` marker
/// - Built-in fields (`arkUrl`, `hasStillImageFileValue`) render with trailing `(built-in)` marker
/// - Source tag is absent for built-in fields (data_model is None)
#[test]
fn resource_type_describe_prose_include_builtins() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_type_describe(&manuscript_with_builtins_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions.
    assert!(
        out.contains("(built-in)"),
        "must render built-in marker for is_builtin=true fields; got:\n{out}"
    );
    // Project field must NOT have (built-in) marker.
    let title_line = out.lines().find(|l| l.contains("title")).unwrap();
    assert!(
        !title_line.contains("(built-in)"),
        "project field must not have (built-in) marker; got: {title_line:?}"
    );
    // arkUrl line must have (built-in) marker.
    let ark_line = out.lines().find(|l| l.contains("arkUrl")).unwrap();
    assert!(
        ark_line.contains("(built-in)"),
        "arkUrl must have (built-in) marker; got: {ark_line:?}"
    );

    // Vocabulary guard.
    for line in out.lines() {
        let lower = line.to_lowercase();
        if lower.contains("iri:") {
            continue;
        }
        assert!(
            !lower.contains("export"),
            "prose include-builtins must not contain 'export'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("class"),
            "prose include-builtins must not contain 'class'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("ontolog"),
            "prose include-builtins must not contain 'ontolog'; offending line: {line:?}"
        );
    }

    insta::assert_snapshot!(out);
}

// ── degraded-field fixture ────────────────────────────────────────────────────

/// Prose render of the degraded-field fixture. Locks:
/// - A field with `ValueType::Other("—")` renders the em-dash as value-type
/// - The `[from biblio]` source tag still appears (data_model is known from CURIE prefix)
/// - The label column is empty (label: None)
#[test]
fn resource_type_describe_prose_degraded_field() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_type_describe(&degraded_field_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions.
    // The cross-DM degraded field must carry [from biblio] tag.
    let degraded_line = out.lines().find(|l| l.contains("unknownBiblioField")).unwrap();
    assert!(
        degraded_line.contains("[from biblio]"),
        "degraded cross-DM field must still show [from biblio]; got: {degraded_line:?}"
    );
    // Own-DM field must NOT have source tag.
    let title_line = out.lines().find(|l| l.contains("title")).unwrap();
    assert!(
        !title_line.contains("[from"),
        "own-DM field must not have [from ...] tag; got: {title_line:?}"
    );

    // Vocabulary guard.
    for line in out.lines() {
        let lower = line.to_lowercase();
        if lower.contains("iri:") {
            continue;
        }
        assert!(
            !lower.contains("export"),
            "prose degraded must not contain 'export'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("class"),
            "prose degraded must not contain 'class'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("ontolog"),
            "prose degraded must not contain 'ontolog'; offending line: {line:?}"
        );
    }

    insta::assert_snapshot!(out);
}

// ── not_found JSON error envelope ─────────────────────────────────────────────

/// Snapshot of the `not_found` JSON error envelope produced by
/// `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`.
///
/// This locks the dsp-cli/ADR-0012 JSON envelope shape — the error path for when the
/// `--resource-type` name/IRI does not match any resource-type in the data-model.
/// The action builds the `NotFound` message with a recovery hint and propagates it
/// via `?`; `main.rs` calls `renderer.diagnostic(…)`.
#[test]
fn resource_type_describe_json_not_found() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let diag = Diagnostic::NotFound(
        "resource-type 'xyz' not found in data-model 'beol' on https://api.dasch.swiss. \
         Run `dsp vre resource-type list --project 0801 --data-model beol \
         --server https://api.dasch.swiss` to see available resource-types."
            .to_string(),
    );
    r.diagnostic(&diag, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("not_found json must be valid JSON");
    assert_eq!(
        parsed["error"]["kind"], "not_found",
        "error envelope must have kind='not_found'; got: {}",
        parsed["error"]["kind"]
    );
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("dsp vre resource-type list"),
        "error message must include the recovery hint; got: {}",
        parsed["error"]["message"]
    );

    insta::assert_snapshot!(out);
}

// ── stderr/stdout contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn resource_type_describe_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "lines: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "lines: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// CSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn resource_type_describe_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "csv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "csv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// TSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn resource_type_describe_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_type_describe(&manuscript_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "tsv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "tsv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}
