//! Snapshot tests for plan 020 output-format flags: `--columns`, `--no-header`,
//! `--header-only`, and the composability assertion.
//!
//! These tests exercise the new flag variants on a bounded set of representative
//! (noun, format, flag) cells. They are NOT exhaustive across every noun × flag
//! combination — the engine unit tests in `src/render/table.rs` cover the
//! output-shape contract; these snapshots provide a regression net for the
//! end-to-end rendered paths.
//!
//! ## Cells chosen (per plan 020 test plan)
//!
//! - `projects` csv projected (`--columns shortcode,iri`)
//! - `projects` lines projected (`--columns iri`)
//! - `resource_type_describe` csv projected (`--columns name,iri,value_type`)
//! - csv `--no-header`
//! - csv `--header-only`
//! - csv `--header-only --columns X,Y` composed
//! - tsv projected
//! - unknown-column error: `insta::assert_snapshot!` of the renderer-level
//!   `Diagnostic::Usage` message (renderer knows the valid column set). The
//!   exit-code mapping (`Diagnostic::Usage` → exit 2) is unit-tested in
//!   `src/diagnostic.rs`; this test only pins the message wording.
//!
//! ## Composability assertion
//!
//! The data rows of a headered csv run are byte-identical to the same run with
//! `--no-header`. The fixture intentionally includes a value with a comma
//! (embedded comma in longname triggers RFC-4180 quoting) and uses `--columns`,
//! so the assertion exercises the quoted, projected path.

use dsp_cli::model::{Cardinality, Field, Project, ProjectStatus, ResourceTypeDetail, ValueType};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{HeaderMode, MetaContext, ProjectListView, Renderer, TableOptions};

mod support;
use support::{buf_to_string, shared_buf};

// ── fixtures ──────────────────────────────────────────────────────────────────

const SERVER: &str = "https://api.test.dasch.swiss";

fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

/// Projects fixture with one entry whose longname contains a comma (for CSV
/// quoting coverage), matching the requirement that the composability assertion
/// exercises the quoted, projected path.
fn projects_with_comma_longname() -> ProjectListView {
    let items = vec![
        Project {
            iri: "http://rdfh.ch/projects/Qt8K2mWbT0eHa1cZ".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            status: ProjectStatus::Active,
            data_models: 4,
        },
        Project {
            iri: "http://rdfh.ch/projects/Hn5D0sJwR3uXf7Tb".into(),
            shortcode: "0820".into(),
            shortname: "incunabula".into(),
            // Contains a comma — triggers RFC-4180 quoting in the projected csv path.
            longname: Some("Basel, Early Book Printing".into()),
            status: ProjectStatus::Active,
            data_models: 1,
        },
    ];
    let total = items.len();
    ProjectListView {
        items,
        total,
        filter: None,
    }
}

/// A ResourceTypeDetail fixture for testing `resource-type describe` csv projection.
fn resource_type_detail_fixture() -> ResourceTypeDetail {
    ResourceTypeDetail {
        name: "letter".into(),
        iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
        label: Some("Letter".into()),
        data_model: "beol".into(),
        representation: None,
        super_types: vec![],
        fields: vec![
            Field {
                name: "hasText".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasText".into(),
                value_type: ValueType::Text,
                link_target: None,
                cardinality: Cardinality::One,
                label: Some("Text".into()),
                is_builtin: false,
                data_model: Some("beol".into()),
            },
            Field {
                name: "createdBy".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#createdBy".into(),
                value_type: ValueType::Link,
                link_target: Some("Person".into()),
                cardinality: Cardinality::ZeroOrOne,
                label: Some("Author".into()),
                is_builtin: false,
                data_model: Some("beol".into()),
            },
        ],
        count: None,
    }
}

/// Helper: build `TableOptions` for a column list.
fn col_opts(cols: &[&str]) -> TableOptions {
    TableOptions {
        columns: Some(cols.iter().map(|s| s.to_string()).collect()),
        header: HeaderMode::On,
    }
}

/// Helper: build `TableOptions` for header-only mode.
fn header_only_opts() -> TableOptions {
    TableOptions {
        columns: None,
        header: HeaderMode::Only,
    }
}

/// Helper: build `TableOptions` for no-header mode.
fn no_header_opts() -> TableOptions {
    TableOptions {
        columns: None,
        header: HeaderMode::Off,
    }
}

// ── projects csv projected ────────────────────────────────────────────────────

#[test]
fn projects_csv_columns_shortcode_iri() {
    // --columns shortcode,iri: select a two-column subset, omitting longname etc.
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(col_opts(&["shortcode", "iri"]));
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── projects lines projected ──────────────────────────────────────────────────

#[test]
fn projects_lines_columns_iri() {
    // --columns iri --format lines: bare IRIs for piping.
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w).with_options(col_opts(&["iri"]));
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── resource_type_describe csv projected ─────────────────────────────────────

#[test]
fn resource_type_describe_csv_columns_name_iri_value_type() {
    // --columns name,iri,value_type: guards the D1 "iri now accessible in csv" case.
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(col_opts(&["name", "iri", "value_type"]));
    r.resource_type_describe(&resource_type_detail_fixture(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── csv --no-header ───────────────────────────────────────────────────────────

#[test]
fn projects_csv_no_header() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(no_header_opts());
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── csv --header-only ─────────────────────────────────────────────────────────

#[test]
fn projects_csv_header_only() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(header_only_opts());
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── csv --header-only --columns X,Y composed ─────────────────────────────────

#[test]
fn projects_csv_header_only_with_columns() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(TableOptions {
        columns: Some(vec!["shortcode".to_string(), "longname".to_string()]),
        header: HeaderMode::Only,
    });
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── tsv projected ─────────────────────────────────────────────────────────────

#[test]
fn projects_tsv_columns_shortcode_shortname() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w).with_options(col_opts(&["shortcode", "shortname"]));
    r.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── composability assertion ───────────────────────────────────────────────────
//
// The data rows of a headered csv run (with --columns + a comma-containing
// value) must be byte-identical to the same run with --no-header.
// This validates the `--no-header >> all.csv` concatenation use case.

#[test]
fn csv_no_header_data_rows_byte_equal_headered_data_rows() {
    let opts_with_header = TableOptions {
        columns: Some(vec!["shortcode".to_string(), "longname".to_string()]),
        header: HeaderMode::On,
    };
    let opts_no_header = TableOptions {
        columns: Some(vec!["shortcode".to_string(), "longname".to_string()]),
        header: HeaderMode::Off,
    };

    // Headered run.
    let (buf_h, w_h) = shared_buf();
    let mut r_h = CsvRenderer::with_writer(w_h).with_options(opts_with_header);
    r_h.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    let headered = buf_to_string(&buf_h);

    // No-header run.
    let (buf_n, w_n) = shared_buf();
    let mut r_n = CsvRenderer::with_writer(w_n).with_options(opts_no_header);
    r_n.projects(&projects_with_comma_longname(), &anon_meta())
        .unwrap();
    let no_header = buf_to_string(&buf_n);

    // The headered output starts with the header line; stripping it gives the
    // data rows, which must be byte-identical to the no-header output.
    let first_newline = headered
        .find('\n')
        .expect("headered output must have a newline");
    let data_rows = &headered[first_newline + 1..];

    assert_eq!(
        data_rows,
        no_header.as_str(),
        "data rows of headered csv must be byte-identical to no-header csv"
    );

    // The fixture must contain a comma-quoted value (guards the non-trivial path).
    assert!(
        no_header.contains('"'),
        "fixture must trigger RFC-4180 quoting to exercise the non-trivial path"
    );
}

// ── unknown-column error ──────────────────────────────────────────────────────
//
// Renderer-level: render with columns=Some(["shortcode","nonexistent"]) and
// snapshot the exact Diagnostic::Usage Display text (which includes the
// unknown column name and the valid column list from the per-noun const).
//
// The exit-code mapping (Diagnostic::Usage → exit 2) is unit-tested in
// src/diagnostic.rs::usage_maps_to_exit_code_2; no network-dependent CLI
// test is needed here.

#[test]
fn csv_unknown_column_returns_usage_error() {
    let (_buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(col_opts(&["shortcode", "nonexistent"]));
    let result = r.projects(&projects_with_comma_longname(), &anon_meta());
    let err = result.unwrap_err();
    insta::assert_snapshot!("csv_unknown_column_error", err.to_string());
}
