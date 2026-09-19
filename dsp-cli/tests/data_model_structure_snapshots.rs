//! Snapshot tests for `dsp vre data-model structure` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Full-mixed fixture** (beol-like): exercises link relations (in-model, cross-model),
//!   inherits relations (in-model), and a same-DM link with no `[to]` tag. Used for
//!   prose ×4 variants and json full.
//! - **Zero-relations fixture**: empty `relations` vec. Locks prose `(0 relations)` branch
//!   and json `data:[]`.
//! - **Include-builtins fixture**: the full-mixed fixture with additional builtin relations
//!   (system superclass inherits, system link field). The renderer renders them when present
//!   — the action has already filtered if needed.
//! - **Cross-model-heavy fixture**: several cross-DM links to test `[to <dm>]` alignment.
//!
//! Lines ×2, csv ×2, tsv ×2 use the full-mixed fixture (stdout + stderr).
//!
//! Determinism: these tests call `Renderer::data_model_structure(&structure, &meta)`
//! directly with a hand-built `MetaContext`/`DataModelStructure`. They never go through
//! the action layer. Action/auth-resolution logic is covered by action tests (Step 6).
//!
//! ADR-0001 vocabulary guard: no `export`/`class`/`property` in rendered prose.
//! (No IRI fields here at all — local names + dm names only.)

use dsp_cli::model::{DataModelStructure, Relation, RelationKind};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

const SERVER: &str = "https://api.dasch.swiss";

// ── MetaContext helpers ────────────────────────────────────────────────────────

fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Full-mixed fixture (beol-like). Relations:
/// - letter → person via hasSender [link, in-model, no tag]
/// - letter → Book via cites [link, cross-model → biblio, [to biblio]]
/// - manuscript → person via hasAuthor [link, in-model, no tag]
/// - letter → writtenSource [inherits, in-model]
/// - manuscript → writtenSource [inherits, in-model]
///
/// Sorted per D6: (source, kind, field, target).
/// Link < Inherits, so links sort before inherits for same source.
fn full_mixed_structure() -> DataModelStructure {
    DataModelStructure {
        data_model: "beol".to_string(),
        relations: vec![
            Relation {
                source: "letter".to_string(),
                target: "Book".to_string(),
                kind: RelationKind::Link,
                field: Some("cites".to_string()),
                target_data_model: Some("biblio".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "letter".to_string(),
                target: "person".to_string(),
                kind: RelationKind::Link,
                field: Some("hasSender".to_string()),
                target_data_model: Some("beol".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "letter".to_string(),
                target: "writtenSource".to_string(),
                kind: RelationKind::Inherits,
                field: None,
                target_data_model: Some("beol".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "manuscript".to_string(),
                target: "person".to_string(),
                kind: RelationKind::Link,
                field: Some("hasAuthor".to_string()),
                target_data_model: Some("beol".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "manuscript".to_string(),
                target: "writtenSource".to_string(),
                kind: RelationKind::Inherits,
                field: None,
                target_data_model: Some("beol".to_string()),
                is_builtin: false,
            },
        ],
    }
}

/// Zero-relations fixture. Locks the `(0 relations)` prose branch and `data:[]` json.
fn zero_relations_structure() -> DataModelStructure {
    DataModelStructure {
        data_model: "minimal".to_string(),
        relations: vec![],
    }
}

/// Include-builtins fixture — the full-mixed fixture plus:
/// - letter → Resource [inherits, system target, is_builtin=true]
/// - letter → StillImageRepresentation [inherits, system, is_builtin=true]
/// - letter → hasLinkTo target [link, system field, is_builtin=true, target_data_model=None]
///
/// (The action normally filters is_builtin=true unless --include-builtins; the renderer
/// renders whatever it receives, so this fixture exercises the full column set.)
fn include_builtins_structure() -> DataModelStructure {
    let mut s = full_mixed_structure();
    // System inherits edges (is_builtin=true, target_data_model=None because system namespace).
    s.relations.push(Relation {
        source: "letter".to_string(),
        target: "Resource".to_string(),
        kind: RelationKind::Inherits,
        field: None,
        target_data_model: None,
        is_builtin: true,
    });
    s.relations.push(Relation {
        source: "manuscript".to_string(),
        target: "StillImageRepresentation".to_string(),
        kind: RelationKind::Inherits,
        field: None,
        target_data_model: None,
        is_builtin: true,
    });
    // System link field (is_builtin=true; target_data_model=None as system target).
    s.relations.push(Relation {
        source: "letter".to_string(),
        target: "Resource".to_string(),
        kind: RelationKind::Link,
        field: Some("hasLinkTo".to_string()),
        target_data_model: None,
        is_builtin: true,
    });
    s
}

/// Cross-model-heavy fixture. Exercises alignment when many `[to <dm>]` tags differ in width.
fn cross_model_heavy_structure() -> DataModelStructure {
    DataModelStructure {
        data_model: "beol".to_string(),
        relations: vec![
            Relation {
                source: "letter".to_string(),
                target: "Book".to_string(),
                kind: RelationKind::Link,
                field: Some("cites".to_string()),
                target_data_model: Some("biblio".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "letter".to_string(),
                target: "Manuscript".to_string(),
                kind: RelationKind::Link,
                field: Some("refersToManuscript".to_string()),
                target_data_model: Some("leibniz".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "letter".to_string(),
                target: "Edition".to_string(),
                kind: RelationKind::Link,
                field: Some("hasEdition".to_string()),
                target_data_model: Some("beol".to_string()),
                is_builtin: false,
            },
            Relation {
                source: "person".to_string(),
                target: "Institution".to_string(),
                kind: RelationKind::Link,
                field: Some("worksAt".to_string()),
                target_data_model: Some("newton".to_string()),
                is_builtin: false,
            },
        ],
    }
}

// ── prose ×4 ──────────────────────────────────────────────────────────────────

/// Prose render of the full-mixed fixture. Locks:
/// - Header `Structure: beol  (5 relations)`
/// - Blank line before rows
/// - 2-space-indented aligned rows: source → target[ [to <dm>]]   field   [kind]
/// - `→` (U+2192) arrow
/// - `[to biblio]` tag on the cross-model link; absent on in-model rows
/// - `[inherits]` kind marker, empty field column for inherits rows
/// - ADR-0007 footer `[anonymous on https://api.dasch.swiss]`
/// - No `export`/`class`/`property` in rendered text
#[test]
fn data_model_structure_prose_full() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_structure(&full_mixed_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Vocabulary guard — no export/class/property (no IRI lines here, all local names).
    for line in out.lines() {
        let lower = line.to_lowercase();
        assert!(
            !lower.contains("export"),
            "prose must not contain 'export'; offending line: {line:?}\nfull output:\n{out}"
        );
        assert!(
            !lower.contains("class"),
            "prose must not contain 'class'; offending line: {line:?}\nfull output:\n{out}"
        );
        assert!(
            !lower.contains("property"),
            "prose must not contain 'property'; offending line: {line:?}\nfull output:\n{out}"
        );
    }

    // Structural assertions.
    assert!(
        out.contains("Structure: beol"),
        "header must contain 'Structure: beol'; got:\n{out}"
    );
    assert!(
        out.contains("5 relations"),
        "header must contain '5 relations'; got:\n{out}"
    );
    // Cross-model tag present.
    assert!(
        out.contains("[to biblio]"),
        "cross-model link must have [to biblio] tag; got:\n{out}"
    );
    // Arrow present.
    assert!(
        out.contains('\u{2192}'),
        "rows must use → (U+2192) arrow; got:\n{out}"
    );
    // Footer.
    assert!(
        out.contains("[anonymous on https://api.dasch.swiss]"),
        "footer missing; got:\n{out}"
    );
    // [to beol] should NOT appear (same-DM target never tagged).
    assert!(
        !out.contains("[to beol]"),
        "same-DM target must not be tagged [to beol]; got:\n{out}"
    );

    insta::assert_snapshot!(out);
}

/// Prose render of the zero-relations fixture. Locks:
/// - Header `Structure: minimal  (0 relations)` (plural "relations" even when 0)
/// - No relation rows
/// - Blank line before footer
/// - ADR-0007 footer still present
#[test]
fn data_model_structure_prose_zero_relations() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_structure(&zero_relations_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        out.contains("(0 relations)"),
        "zero-relations header must show '(0 relations)'; got:\n{out}"
    );
    assert!(
        out.contains("[anonymous on"),
        "footer must still be present for zero-relations; got:\n{out}"
    );
    // No arrow rows.
    assert!(
        !out.contains('\u{2192}'),
        "zero-relations prose must have no rows; got:\n{out}"
    );

    insta::assert_snapshot!(out);
}

/// Prose render of the include-builtins fixture (relation list WITH builtins present).
/// Locks that the renderer renders all rows it receives (filtering is the action's job).
#[test]
fn data_model_structure_prose_include_builtins() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_structure(&include_builtins_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // All 8 relations must render.
    assert!(
        out.contains("8 relations"),
        "header must show '8 relations' with builtins; got:\n{out}"
    );
    // System types should appear in rows (no IRI expansion — just local names).
    assert!(
        out.contains("Resource"),
        "system super 'Resource' must appear in rows; got:\n{out}"
    );
    assert!(
        out.contains("StillImageRepresentation"),
        "'StillImageRepresentation' must appear in rows; got:\n{out}"
    );

    // Vocabulary guard.
    for line in out.lines() {
        let lower = line.to_lowercase();
        assert!(
            !lower.contains("export"),
            "prose must not contain 'export'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("class"),
            "prose must not contain 'class'; offending line: {line:?}"
        );
        assert!(
            !lower.contains("property"),
            "prose must not contain 'property'; offending line: {line:?}"
        );
    }

    insta::assert_snapshot!(out);
}

/// Prose render of the cross-model-heavy fixture. Locks alignment when multiple
/// `[to <dm>]` tags of different widths are present; same-DM `[to beol]` absent.
#[test]
fn data_model_structure_prose_cross_model_heavy() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_structure(&cross_model_heavy_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        out.contains("[to biblio]"),
        "[to biblio] must appear; got:\n{out}"
    );
    assert!(
        out.contains("[to leibniz]"),
        "[to leibniz] must appear; got:\n{out}"
    );
    assert!(
        out.contains("[to newton]"),
        "[to newton] must appear; got:\n{out}"
    );
    // same-DM target_data_model == "beol" == structure.data_model → no tag.
    assert!(
        !out.contains("[to beol]"),
        "same-DM target must not be tagged [to beol]; got:\n{out}"
    );

    insta::assert_snapshot!(out);
}

// ── json ×2 ───────────────────────────────────────────────────────────────────

/// JSON render of the full-mixed fixture. Locks:
/// - ADR-0003 `{_meta, data:[…]}` envelope
/// - Each element has all 5 keys: source, target, kind, field, target_data_model
/// - `field: null` for inherits relations
/// - `target_data_model: "<baseline_dm>"` for in-model targets (non-system)
/// - `target_data_model: null` for system targets only
/// - `target_data_model: "biblio"` for the cross-model link
/// - `kind: "link"` / `kind: "inherits"` strings
#[test]
fn data_model_structure_json_full() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_model_structure(&full_mixed_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert_eq!(parsed["_meta"]["auth"], "anonymous");
    assert_eq!(parsed["_meta"]["exit_code"], 0);
    let data = parsed["data"].as_array().expect("data must be array");
    assert_eq!(data.len(), 5, "5 relations in full-mixed fixture");

    // First element: letter → Book [link, [to biblio]]
    assert_eq!(data[0]["source"], "letter");
    assert_eq!(data[0]["target"], "Book");
    assert_eq!(data[0]["kind"], "link");
    assert_eq!(data[0]["field"], "cites");
    assert_eq!(data[0]["target_data_model"], "biblio");

    // Third element: letter → writtenSource [inherits]
    assert_eq!(data[2]["source"], "letter");
    assert_eq!(data[2]["target"], "writtenSource");
    assert_eq!(data[2]["kind"], "inherits");
    assert!(
        data[2]["field"].is_null(),
        "inherits field must be null; got: {}",
        data[2]["field"]
    );
    assert_eq!(
        data[2]["target_data_model"], "beol",
        "in-model target_data_model must be the baseline DM name"
    );

    insta::assert_snapshot!(out);
}

/// JSON render of the zero-relations fixture. Locks `data:[]` empty array.
#[test]
fn data_model_structure_json_zero_relations() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_model_structure(&zero_relations_structure(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    let data = parsed["data"].as_array().expect("data must be array");
    assert!(
        data.is_empty(),
        "zero-relations must produce empty data array"
    );

    insta::assert_snapshot!(out);
}

// ── lines ×2 (stdout + stderr) ───────────────────────────────────────────────

/// Lines render of the full-mixed fixture. Locks:
/// - One tab-separated row per relation: source TAB target TAB kind TAB field
/// - field is empty for inherits relations
/// - target_data_model intentionally omitted
/// - No header row
/// - Disclosure on stderr, not stdout
#[test]
fn data_model_structure_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.data_model_structure(&full_mixed_structure(), &anon_meta())
        .unwrap();

    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    // Structural assertions.
    assert!(
        !stdout.starts_with("source"),
        "lines must have no header row"
    );
    // link row: source TAB target TAB kind TAB field
    assert!(
        stdout.contains("letter\tBook\tlink\tcites"),
        "cross-model link row missing; got:\n{stdout}"
    );
    // inherits row: source TAB target TAB kind TAB <empty>
    assert!(
        stdout.contains("letter\twrittenSource\tinherits\t"),
        "inherits row must have empty field; got:\n{stdout}"
    );
    // target_data_model must NOT appear (omitted intentionally).
    assert!(
        !stdout.contains("biblio"),
        "lines format must omit target_data_model; got:\n{stdout}"
    );

    // Every non-disclosure data line must have EXACTLY 4 tab-separated columns
    // (source, target, kind, field) — confirming target_data_model is NOT leaked
    // into the lines format for ANY target (in-model OR cross-model). A regression
    // that appended a 5th column (e.g. the DM name "beol" or "biblio") would be
    // caught here even if the `!contains("biblio")` check above passes for in-model
    // rows.
    for line in stdout.lines() {
        let col_count = line.split('\t').count();
        assert_eq!(
            col_count, 4,
            "every lines data row must have exactly 4 tab-separated columns \
             (source, target, kind, field); got {col_count} in line: {line:?}\nfull stdout:\n{stdout}"
        );
    }

    // Disclosure on stderr.
    assert!(
        stderr.contains("[anonymous on"),
        "disclosure must be on stderr; got:\n{stderr}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "disclosure must NOT be on stdout; got:\n{stdout}"
    );

    // Snapshot stdout.
    insta::assert_snapshot!("data_model_structure_lines_stdout", stdout);
    // Snapshot stderr.
    insta::assert_snapshot!("data_model_structure_lines_stderr", stderr);
}

// ── csv ×2 (stdout + stderr) ─────────────────────────────────────────────────

/// CSV render of the full-mixed fixture. Locks:
/// - Header `source,target,kind,field,target_data_model`
/// - One row per relation; field empty for inherits; target_data_model empty only when None (system)
/// - target_data_model populated for in-model and cross-model links
/// - Disclosure on stderr, not stdout
#[test]
fn data_model_structure_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_model_structure(&full_mixed_structure(), &anon_meta())
        .unwrap();

    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    // Header.
    assert!(
        stdout.starts_with("source,target,kind,field,target_data_model\n"),
        "csv header row missing; got:\n{stdout}"
    );
    // Cross-model link row (with target_data_model).
    assert!(
        stdout.contains("letter,Book,link,cites,biblio"),
        "cross-model link row missing; got:\n{stdout}"
    );
    // In-model link row (target_data_model = baseline DM "beol").
    assert!(
        stdout.contains("letter,person,link,hasSender,beol"),
        "in-model link row missing; got:\n{stdout}"
    );
    // Inherits row (field empty, target_data_model = baseline DM "beol").
    assert!(
        stdout.contains("letter,writtenSource,inherits,,beol"),
        "inherits row must have empty field and in-model target_data_model; got:\n{stdout}"
    );

    // Disclosure on stderr.
    assert!(
        stderr.contains("[anonymous on"),
        "disclosure must be on stderr; got:\n{stderr}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "disclosure must NOT be on stdout; got:\n{stdout}"
    );

    // Snapshot stdout.
    insta::assert_snapshot!("data_model_structure_csv_stdout", stdout);
    // Snapshot stderr.
    insta::assert_snapshot!("data_model_structure_csv_stderr", stderr);
}

// ── tsv ×2 (stdout + stderr) ─────────────────────────────────────────────────

/// TSV render of the full-mixed fixture. Locks:
/// - Header `source\ttarget\tkind\tfield\ttarget_data_model`
/// - One row per relation; field empty for inherits; target_data_model empty only when None (system)
/// - target_data_model populated for in-model and cross-model links
/// - Disclosure on stderr, not stdout
#[test]
fn data_model_structure_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_model_structure(&full_mixed_structure(), &anon_meta())
        .unwrap();

    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    // Header.
    assert!(
        stdout.starts_with("source\ttarget\tkind\tfield\ttarget_data_model\n"),
        "tsv header row missing; got:\n{stdout}"
    );
    // Cross-model link row.
    assert!(
        stdout.contains("letter\tBook\tlink\tcites\tbiblio"),
        "cross-model link row missing; got:\n{stdout}"
    );
    // In-model link row (target_data_model = baseline DM "beol").
    assert!(
        stdout.contains("letter\tperson\tlink\thasSender\tbeol"),
        "in-model link row missing; got:\n{stdout}"
    );
    // Inherits row (field empty, target_data_model = baseline DM "beol").
    assert!(
        stdout.contains("letter\twrittenSource\tinherits\t\tbeol"),
        "inherits row must have empty field and in-model target_data_model; got:\n{stdout}"
    );

    // Disclosure on stderr.
    assert!(
        stderr.contains("[anonymous on"),
        "disclosure must be on stderr; got:\n{stderr}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "disclosure must NOT be on stdout; got:\n{stdout}"
    );

    // Snapshot stdout.
    insta::assert_snapshot!("data_model_structure_tsv_stdout", stdout);
    // Snapshot stderr.
    insta::assert_snapshot!("data_model_structure_tsv_stderr", stderr);
}
