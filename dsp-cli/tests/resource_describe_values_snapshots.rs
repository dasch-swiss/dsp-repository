//! Snapshot tests for `dsp vre resource describe --values` — value-rendering layer.
//!
//! Tests call `Renderer::resource_describe` directly with hand-built `ResourceDetail`
//! fixtures (no HTTP). All `insta` baselines are accepted after eyeballing.
//!
//! Coverage rule (ADR-0013): at least one of each rendered category is locked
//! as a public output contract the moment these snapshots are accepted:
//!   text (formatted + unformatted), number (int + decimal), boolean,
//!   date (single-point AND range), time, uri, color, geoname,
//!   link (with target label AND degraded IRI-only),
//!   vocabulary-item (with label AND degraded node-IRI),
//!   file (still-image with W×H locked; plus a non-still document),
//!   raw fallback, multi-value field, degraded (label None) field header,
//!   per-value comment (present under --values in prose/json always; opt-in
//!   `comment` column in tabular via `--columns`).
//!
//! **Negative case**: `values: None` fixtures re-run through prose/json renderers
//! to confirm the output is byte-identical to the existing 8b baselines — no
//! `values` key in JSON (absent, not null). An explicit JSON assertion checks that
//! `data` has no `values` key when `values` is `None`.
//!
//! The tabular stderr note ("values not shown in tabular formats…") is asserted
//! via `RecordingRenderer`-style direct assertions on stderr, NOT via insta
//! snapshots (per plan step 8 note).
//!
//! Vocabulary guard: no raw DSP-API keys or `knora-api:` names in rendered output.

use dsp_cli::model::resource_type::ValueType;
use dsp_cli::model::{
    DatePoint, DateValue, FieldValues, FileValue, ResourceAccess, ResourceDetail,
    ResourceVisibility, Value, ValueContent,
};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{HeaderMode, MetaContext, Renderer, TableOptions};

mod support;
use support::{buf_to_string, shared_buf};

// ── Constants ─────────────────────────────────────────────────────────────────

const SERVER: &str = "api.dasch.swiss";
const ANON_FILTER_WARNING: &str = "results may be filtered; login to see private resources";

fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: Some(ANON_FILTER_WARNING.to_string()),
        count_caveat: None,
        count_cost: None,
    }
}

// ── Base envelope (shared) ────────────────────────────────────────────────────

fn base_envelope(values: Option<Vec<FieldValues>>) -> ResourceDetail {
    ResourceDetail {
        label: "n6r".to_string(),
        iri: "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw".to_string(),
        resource_type: "Page".to_string(),
        ark_url: Some(
            "https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp4SVnGG1DBzFvYErwr".to_string(),
        ),
        creation_date: Some("2011-04-14T07:32:49Z".to_string()),
        last_modified: Some("2024-03-10T15:00:00Z".to_string()),
        attached_project: Some("http://rdfh.ch/projects/3ABR_2i8QYGSIDvmP9mlEw".to_string()),
        owner: Some("http://rdfh.ch/users/IShOmBIGSnO1TXd4-Ty4Sw".to_string()),
        visibility: Some(ResourceVisibility::Public),
        your_access: Some(ResourceAccess::View),
        values,
    }
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// Multi-type fixture A: text (both kinds), integer, decimal, boolean, date single-point,
/// and a multi-value field. Covers the most common scalar types in one resource.
fn fixture_a_scalars() -> ResourceDetail {
    let unformatted_text = FieldValues {
        name: "hasPagenum".to_string(),
        label: Some("Page Number".to_string()),
        values: vec![ValueContent::Text("n6r".to_string()).into()],
    };
    // Formatted text — by the time it reaches the renderer the client has already
    // converted standoff XML to plain text (html_to_text), so the value is plain.
    let formatted_text = FieldValues {
        name: "hasComment".to_string(),
        label: Some("Comment".to_string()),
        values: vec![
            ValueContent::Text("This is a text value with stripped standoff.".to_string()).into(),
        ],
    };
    let integer_field = FieldValues {
        name: "hasSequenceNumber".to_string(),
        label: Some("Sequence Number".to_string()),
        values: vec![ValueContent::Integer(42).into()],
    };
    let decimal_field = FieldValues {
        name: "hasMeasurement".to_string(),
        label: Some("Measurement".to_string()),
        values: vec![ValueContent::Decimal("3.14159".to_string()).into()],
    };
    let bool_field = FieldValues {
        name: "isPublished".to_string(),
        label: Some("Is Published".to_string()),
        values: vec![ValueContent::Boolean(true).into()],
    };
    // Single-point date (start == end)
    let date_point = DatePoint {
        year: Some(1489),
        month: None,
        day: None,
        era: Some("CE".to_string()),
    };
    let date_field = FieldValues {
        name: "hasPublicationDate".to_string(),
        label: Some("Publication Date".to_string()),
        values: vec![
            ValueContent::Date(DateValue {
                calendar: "GREGORIAN".to_string(),
                start: date_point.clone(),
                end: date_point,
            })
            .into(),
        ],
    };
    // Multi-value field: two text values on the same property
    let multi_value_field = FieldValues {
        name: "hasKeyword".to_string(),
        label: Some("Keyword".to_string()),
        values: vec![
            ValueContent::Text("incunabula".to_string()).into(),
            ValueContent::Text("manuscript".to_string()).into(),
        ],
    };
    base_envelope(Some(vec![
        unformatted_text,
        formatted_text,
        integer_field,
        decimal_field,
        bool_field,
        date_field,
        multi_value_field,
    ]))
}

/// Multi-type fixture B: date range, time, uri, color, geoname, raw fallback,
/// and a degraded (label = None) field header.
fn fixture_b_named_scalars_and_raw() -> ResourceDetail {
    // Date range (start != end)
    let date_range = FieldValues {
        name: "hasCopyingDate".to_string(),
        label: Some("Copying Date".to_string()),
        values: vec![
            ValueContent::Date(DateValue {
                calendar: "GREGORIAN".to_string(),
                start: DatePoint {
                    year: Some(1489),
                    month: None,
                    day: None,
                    era: Some("CE".to_string()),
                },
                end: DatePoint {
                    year: Some(1490),
                    month: None,
                    day: None,
                    era: Some("CE".to_string()),
                },
            })
            .into(),
        ],
    };
    let time_field = FieldValues {
        name: "hasTimestamp".to_string(),
        label: Some("Timestamp".to_string()),
        values: vec![ValueContent::Time("2024-03-10T15:00:00Z".to_string()).into()],
    };
    let uri_field = FieldValues {
        name: "hasExternalUri".to_string(),
        label: Some("External URI".to_string()),
        values: vec![ValueContent::Uri("https://www.example.com/resource".to_string()).into()],
    };
    let color_field = FieldValues {
        name: "hasColor".to_string(),
        label: Some("Color".to_string()),
        values: vec![ValueContent::Color("#ff3300".to_string()).into()],
    };
    let geoname_field = FieldValues {
        name: "hasLocation".to_string(),
        label: Some("Location".to_string()),
        values: vec![ValueContent::Geoname("2661552".to_string()).into()],
    };
    // Raw fallback (e.g. interval value-type not in the named set)
    let raw_field = FieldValues {
        name: "hasDuration".to_string(),
        label: Some("Duration".to_string()),
        values: vec![
            ValueContent::Raw {
                value_type: "interval".to_string(),
                text: "PT10S".to_string(),
            }
            .into(),
        ],
    };
    // Degraded field header: label is None → renders as `<name>` only (no parentheses)
    let degraded_label = FieldValues {
        name: "hasStillImageFileValue".to_string(),
        label: None, // built-in field, no ontology fetch
        values: vec![ValueContent::Text("sentinel".to_string()).into()],
    };
    base_envelope(Some(vec![
        date_range,
        time_field,
        uri_field,
        color_field,
        geoname_field,
        raw_field,
        degraded_label,
    ]))
}

/// Multi-type fixture C: link (with label AND degraded IRI-only),
/// vocabulary-item (with label AND degraded node-IRI), still-image file (with W×H),
/// and a non-still document file.
fn fixture_c_links_list_file() -> ResourceDetail {
    // Link with target label populated
    let link_with_label = FieldValues {
        name: "isPartOfBook".to_string(),
        label: Some("Is Part Of Book".to_string()),
        values: vec![
            ValueContent::Link {
                target_iri: "http://rdfh.ch/0803/bookres123".to_string(),
                target_label: Some("Incunabula Testbook".to_string()),
            }
            .into(),
        ],
    };
    // Link with no target label (degraded: IRI only)
    let link_iri_only = FieldValues {
        name: "hasRelatedResource".to_string(),
        label: Some("Related Resource".to_string()),
        values: vec![
            ValueContent::Link {
                target_iri: "http://rdfh.ch/0803/anotherres456".to_string(),
                target_label: None,
            }
            .into(),
        ],
    };
    // List-item with resolved label
    let list_with_label = FieldValues {
        name: "hasBookGenre".to_string(),
        label: Some("Book Genre".to_string()),
        values: vec![
            ValueContent::VocabularyItem {
                node_iri: "http://rdfh.ch/lists/0803/genre-incunabula".to_string(),
                label: Some("Incunabula".to_string()),
            }
            .into(),
        ],
    };
    // List-item degraded to node IRI (label fetch failed)
    let list_degraded = FieldValues {
        name: "hasSubject".to_string(),
        label: Some("Subject".to_string()),
        values: vec![
            ValueContent::VocabularyItem {
                node_iri: "http://rdfh.ch/lists/0803/subject-history".to_string(),
                label: None,
            }
            .into(),
        ],
    };
    // Still-image file: locks the `<filename> (<W>×<H>) <url>` form
    let still_image = FieldValues {
        name: "hasStillImageFileValue".to_string(),
        label: None, // built-in field
        values: vec![
            ValueContent::File(FileValue {
                value_type: ValueType::StillImage,
                filename: "n6r.jp2".to_string(),
                url: "https://iiif.dasch.swiss/0803/n6r.jp2/full/max/0/default.jpg".to_string(),
                width: Some(2048),
                height: Some(3072),
            })
            .into(),
        ],
    };
    // Document file (non-still): no W×H form
    let document = FieldValues {
        name: "hasDocumentFileValue".to_string(),
        label: None, // built-in field
        values: vec![
            ValueContent::File(FileValue {
                value_type: ValueType::Document,
                filename: "transcription.pdf".to_string(),
                url: "https://files.dasch.swiss/0803/transcription.pdf".to_string(),
                width: None,
                height: None,
            })
            .into(),
        ],
    };
    base_envelope(Some(vec![
        link_with_label,
        link_iri_only,
        list_with_label,
        list_degraded,
        still_image,
        document,
    ]))
}

/// Empty-values fixture: `values = Some(vec![])`.
/// Prose must render `Values: (none)`.
fn fixture_empty_values() -> ResourceDetail {
    base_envelope(Some(vec![]))
}

/// Single-field fixture embedding ASCII control chars in BOTH the value (an ESC,
/// as part of an ANSI CSI-ish sequence) and the field label (a tab) — used to
/// prove the tabular `--values` row-builder actually routes through
/// `render_table`'s `QuoteMode::apply` → `replace_control_chars` chokepoint,
/// not just that the chokepoint exists in the abstract.
fn fixture_control_chars() -> ResourceDetail {
    let field = FieldValues {
        name: "hasControlChars".to_string(),
        label: Some("Lab\tel".to_string()),
        values: vec![ValueContent::Text("a\u{1b}b".to_string()).into()],
    };
    base_envelope(Some(vec![field]))
}

/// Single-field fixture with ONE value carrying a `knora-api:valueHasComment`
/// (`Value.comment`) — used to lock the per-value comment baseline across
/// prose, json, and tabular (opt-in `comment` column via `--columns`).
fn fixture_d_with_comment() -> ResourceDetail {
    let field = FieldValues {
        name: "hasTranscription".to_string(),
        label: Some("Transcription".to_string()),
        values: vec![Value {
            content: ValueContent::Text("some transcription".to_string()),
            comment: Some("reading uncertain".to_string()),
        }],
    };
    base_envelope(Some(vec![field]))
}

// ── Prose snapshots — fixture A ───────────────────────────────────────────────

/// Prose render of fixture A (scalars: text, integer, decimal, boolean, date, multi-value).
///
/// Locks:
/// - `Values:` section header appears.
/// - Field headers `<label> (<name>)` for labelled fields.
/// - Integer renders as bare number `42`.
/// - Decimal renders with precision preserved `3.14159`.
/// - Boolean renders as `true`.
/// - Single-point date collapses: `1489 CE (GREGORIAN)`.
/// - Multi-value field shows both values, one per line.
#[test]
fn values_prose_scalars() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&fixture_a_scalars(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Values section present.
    assert!(
        out.contains("Values:"),
        "prose must have 'Values:' section; got:\n{out}"
    );
    // Field header with label.
    assert!(
        out.contains("Page Number (hasPagenum)"),
        "prose must show 'Page Number (hasPagenum)'; got:\n{out}"
    );
    // Integer.
    assert!(
        out.contains("42"),
        "prose must show integer value 42; got:\n{out}"
    );
    // Decimal with precision.
    assert!(
        out.contains("3.14159"),
        "prose must show decimal 3.14159; got:\n{out}"
    );
    // Boolean.
    assert!(
        out.contains("true"),
        "prose must show boolean 'true'; got:\n{out}"
    );
    // Single-point date.
    assert!(
        out.contains("1489 CE (GREGORIAN)"),
        "prose must show collapsed date '1489 CE (GREGORIAN)'; got:\n{out}"
    );
    // Multi-value: both keywords present.
    assert!(
        out.contains("incunabula"),
        "prose must show keyword 'incunabula'; got:\n{out}"
    );
    assert!(
        out.contains("manuscript"),
        "prose must show keyword 'manuscript'; got:\n{out}"
    );
    // No DSP-API vocab.
    assert!(
        !out.contains("knora-api:"),
        "prose must not leak 'knora-api:'; got:\n{out}"
    );
    insta::assert_snapshot!("values_prose_scalars", out);
}

/// Prose render of fixture B (date range, time, uri, color, geoname, raw, degraded header).
///
/// Locks:
/// - Date range renders as `1489 CE – 1490 CE (GREGORIAN)`.
/// - Degraded field header `<name>` alone (no parentheses) when label is None.
/// - Raw fallback renders as the `text` field value.
#[test]
fn values_prose_named_scalars_and_raw() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&fixture_b_named_scalars_and_raw(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Date range (en-dash separator).
    assert!(
        out.contains("1489 CE") && out.contains("1490 CE"),
        "prose must show both ends of date range; got:\n{out}"
    );
    // Named scalar types.
    assert!(
        out.contains("https://www.example.com/resource"),
        "prose must show URI; got:\n{out}"
    );
    assert!(
        out.contains("#ff3300"),
        "prose must show color; got:\n{out}"
    );
    assert!(
        out.contains("2661552"),
        "prose must show geoname code; got:\n{out}"
    );
    // Raw fallback text.
    assert!(
        out.contains("PT10S"),
        "prose must show raw fallback text 'PT10S'; got:\n{out}"
    );
    // Degraded label: `hasStillImageFileValue` alone (no parentheses).
    assert!(
        out.contains("hasStillImageFileValue"),
        "prose must show degraded field name alone; got:\n{out}"
    );
    assert!(
        !out.contains("hasStillImageFileValue ("),
        "prose must NOT show parentheses for degraded label-None field; got:\n{out}"
    );
    insta::assert_snapshot!("values_prose_named_scalars_and_raw", out);
}

/// Prose render of fixture C (links, vocabulary-items, files).
///
/// Locks:
/// - Link with label: `→ <label> [<iri>]`.
/// - Link IRI-only (degraded): `→ <iri>` (no brackets).
/// - List-item with label: label text only.
/// - List-item degraded: node IRI.
/// - Still-image: `<filename> (<W>×<H>) <url>`.
/// - Document: `<filename> <url>` (no dims).
#[test]
fn values_prose_links_list_file() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&fixture_c_links_list_file(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    // Link with label: the → arrow and label+iri.
    assert!(
        out.contains("Incunabula Testbook"),
        "prose must show link target label; got:\n{out}"
    );
    assert!(
        out.contains("[http://rdfh.ch/0803/bookres123]"),
        "prose must show link IRI in brackets; got:\n{out}"
    );
    // Link degraded (no brackets, IRI only).
    assert!(
        out.contains("http://rdfh.ch/0803/anotherres456"),
        "prose must show degraded link IRI; got:\n{out}"
    );
    assert!(
        !out.contains("[http://rdfh.ch/0803/anotherres456]"),
        "prose must NOT wrap degraded link IRI in brackets; got:\n{out}"
    );
    // List-item with label.
    assert!(
        out.contains("Incunabula"),
        "prose must show vocabulary-item label; got:\n{out}"
    );
    // List-item degraded.
    assert!(
        out.contains("http://rdfh.ch/lists/0803/subject-history"),
        "prose must show degraded list node IRI; got:\n{out}"
    );
    // Still-image with W×H dimensions.
    assert!(
        out.contains("n6r.jp2"),
        "prose must show image filename; got:\n{out}"
    );
    assert!(
        out.contains("2048") && out.contains("3072"),
        "prose must show still-image dimensions; got:\n{out}"
    );
    // Document (no dims — just filename url).
    assert!(
        out.contains("transcription.pdf"),
        "prose must show document filename; got:\n{out}"
    );
    insta::assert_snapshot!("values_prose_links_list_file", out);
}

/// Prose render of empty-values fixture.
///
/// Locks: "Values: (none)" when Some(vec![]) is passed.
#[test]
fn values_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&fixture_empty_values(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("Values: (none)"),
        "prose must show 'Values: (none)' for empty fields vec; got:\n{out}"
    );
    insta::assert_snapshot!("values_prose_empty", out);
}

/// Prose render of fixture D (single value carrying a per-value comment).
///
/// Locks:
/// - Field header `Transcription (hasTranscription)`.
/// - Value line shows the value text.
/// - An indented `comment: <text>` line directly follows the value line.
#[test]
fn values_prose_comment() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&fixture_d_with_comment(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        out.contains("Transcription (hasTranscription)"),
        "prose must show 'Transcription (hasTranscription)'; got:\n{out}"
    );
    assert!(
        out.contains("some transcription"),
        "prose must show the value text; got:\n{out}"
    );
    assert!(
        out.contains("comment: reading uncertain"),
        "prose must show the comment line; got:\n{out}"
    );
    insta::assert_snapshot!("values_prose_comment", out);
}

// ── JSON snapshots — fixture A ────────────────────────────────────────────────

/// JSON render of fixture A (scalars).
///
/// Locks:
/// - `values` key present in `data` when `Some`.
/// - `value_type` tokens: `text`, `integer`, `decimal`, `boolean`, `date`.
/// - Date object has `calendar`, `start`, `end` sub-objects.
/// - Multi-value field has an array of 2 value objects.
#[test]
fn values_json_scalars() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&fixture_a_scalars(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");

    // `values` key is present.
    assert!(
        parsed["data"].get("values").is_some(),
        "json data must have 'values' key when Some; got:\n{out}"
    );
    let values = parsed["data"]["values"]
        .as_array()
        .expect("values must be array");
    assert!(
        !values.is_empty(),
        "values array must not be empty; got:\n{out}"
    );

    // Find the integer field.
    let int_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasSequenceNumber"))
        .expect("must have hasSequenceNumber field");
    assert_eq!(
        int_field["values"][0]["value_type"].as_str().unwrap(),
        "integer"
    );
    assert_eq!(int_field["values"][0]["value"].as_i64().unwrap(), 42);

    // Find the date field.
    let date_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasPublicationDate"))
        .expect("must have hasPublicationDate field");
    assert_eq!(
        date_field["values"][0]["value_type"].as_str().unwrap(),
        "date"
    );
    assert_eq!(
        date_field["values"][0]["calendar"].as_str().unwrap(),
        "GREGORIAN"
    );
    assert!(
        date_field["values"][0].get("start").is_some(),
        "date must have 'start' sub-object"
    );
    assert!(
        date_field["values"][0].get("end").is_some(),
        "date must have 'end' sub-object"
    );

    // Multi-value field: 2 values.
    let kw_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasKeyword"))
        .expect("must have hasKeyword field");
    assert_eq!(
        kw_field["values"].as_array().unwrap().len(),
        2,
        "multi-value field must have 2 value objects"
    );

    insta::assert_snapshot!("values_json_scalars", out);
}

/// JSON render of fixture B (named scalars + raw).
///
/// Locks:
/// - `value_type` tokens: `time`, `uri`, `color`, `geoname`, `interval` (raw fallback).
/// - Raw fallback uses `value_type` = lowercased local name and has `text` key.
/// - `field_label` is `null` when label is `None` (degraded field).
#[test]
fn values_json_named_scalars_and_raw() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&fixture_b_named_scalars_and_raw(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    let values = parsed["data"]["values"]
        .as_array()
        .expect("values must be array");

    // Check time token.
    let time_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasTimestamp"))
        .expect("must have hasTimestamp");
    assert_eq!(
        time_field["values"][0]["value_type"].as_str().unwrap(),
        "time"
    );

    // Check uri token.
    let uri_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasExternalUri"))
        .expect("must have hasExternalUri");
    assert_eq!(
        uri_field["values"][0]["value_type"].as_str().unwrap(),
        "uri"
    );

    // Check color token.
    let color_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasColor"))
        .expect("must have hasColor");
    assert_eq!(
        color_field["values"][0]["value_type"].as_str().unwrap(),
        "color"
    );

    // Check geoname token.
    let geo_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasLocation"))
        .expect("must have hasLocation");
    assert_eq!(
        geo_field["values"][0]["value_type"].as_str().unwrap(),
        "geoname"
    );

    // Check raw fallback: value_type = "interval", has `text` key.
    let raw_field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasDuration"))
        .expect("must have hasDuration");
    assert_eq!(
        raw_field["values"][0]["value_type"].as_str().unwrap(),
        "interval"
    );
    assert!(
        raw_field["values"][0].get("text").is_some(),
        "raw must have 'text' key"
    );

    // Degraded field label is null.
    let degraded = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasStillImageFileValue"))
        .expect("must have degraded field");
    assert!(
        degraded["field_label"].is_null(),
        "degraded field (label None) must have null field_label in json; got: {:?}",
        degraded["field_label"]
    );

    insta::assert_snapshot!("values_json_named_scalars_and_raw", out);
}

/// JSON render of fixture C (links, vocabulary-items, files).
///
/// Locks:
/// - Link: `value_type = "link"`, `target_iri`, `target_label` (non-null when present, null when absent).
/// - List-item: `value_type = "vocabulary-item"`, `node_iri`, `label` (non-null/null per case).
/// - Still-image: `value_type = "still-image"`, `filename`, `url`, `width`, `height` non-null.
/// - Document: `value_type = "document"`, `width` and `height` null.
#[test]
fn values_json_links_list_file() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&fixture_c_links_list_file(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    let values = parsed["data"]["values"]
        .as_array()
        .expect("values must be array");

    // Link with label.
    let link = values
        .iter()
        .find(|f| f["field"].as_str() == Some("isPartOfBook"))
        .expect("must have isPartOfBook");
    assert_eq!(link["values"][0]["value_type"].as_str().unwrap(), "link");
    assert_eq!(
        link["values"][0]["target_iri"].as_str().unwrap(),
        "http://rdfh.ch/0803/bookres123"
    );
    assert_eq!(
        link["values"][0]["target_label"].as_str().unwrap(),
        "Incunabula Testbook"
    );

    // Link degraded: target_label is null.
    let link_deg = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasRelatedResource"))
        .expect("must have hasRelatedResource");
    assert!(
        link_deg["values"][0]["target_label"].is_null(),
        "degraded link must have null target_label"
    );

    // List-item with label.
    let list = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasBookGenre"))
        .expect("must have hasBookGenre");
    assert_eq!(
        list["values"][0]["value_type"].as_str().unwrap(),
        "vocabulary-item"
    );
    assert!(list["values"][0]["node_iri"].as_str().is_some());
    assert_eq!(list["values"][0]["label"].as_str().unwrap(), "Incunabula");

    // List-item degraded: label is null.
    let list_deg = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasSubject"))
        .expect("must have hasSubject");
    assert!(
        list_deg["values"][0]["label"].is_null(),
        "degraded vocabulary-item must have null label"
    );

    // Still-image: non-null width + height.
    let img = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasStillImageFileValue"))
        .expect("must have still-image field");
    assert_eq!(
        img["values"][0]["value_type"].as_str().unwrap(),
        "still-image"
    );
    assert_eq!(img["values"][0]["width"].as_u64().unwrap(), 2048);
    assert_eq!(img["values"][0]["height"].as_u64().unwrap(), 3072);

    // Document: null width + height.
    let doc = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasDocumentFileValue"))
        .expect("must have document field");
    assert_eq!(doc["values"][0]["value_type"].as_str().unwrap(), "document");
    assert!(
        doc["values"][0]["width"].is_null(),
        "document width must be null"
    );
    assert!(
        doc["values"][0]["height"].is_null(),
        "document height must be null"
    );

    insta::assert_snapshot!("values_json_links_list_file", out);
}

/// JSON render of fixture D (single value carrying a per-value comment).
///
/// Locks: the value object carries a `"comment"` key with the comment text.
#[test]
fn values_json_comment() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&fixture_d_with_comment(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    let values = parsed["data"]["values"]
        .as_array()
        .expect("values must be array");

    let field = values
        .iter()
        .find(|f| f["field"].as_str() == Some("hasTranscription"))
        .expect("must have hasTranscription field");
    assert_eq!(field["values"][0]["value_type"].as_str().unwrap(), "text");
    assert_eq!(
        field["values"][0]["comment"].as_str().unwrap(),
        "reading uncertain"
    );

    insta::assert_snapshot!("values_json_comment", out);
}

// ── Negative case: values = None produces 8b-identical output ────────────────

/// Prose with `values = None` must NOT include a "Values:" section.
///
/// This guards that 8b callers (which do not pass --values) see byte-identical
/// output. The existing 8b snapshot `resource_describe_prose` already locks this,
/// but this explicit assertion adds a named check so the absence is intentional.
#[test]
fn values_prose_no_values_key_absent() {
    let detail = base_envelope(None);
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&detail, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("Values:"),
        "prose with values=None must NOT contain 'Values:' section; got:\n{out}"
    );
}

/// JSON with `values = None` must NOT have a `values` key in `data`.
///
/// Explicitly asserts the key is absent (not null) — preserving the 8b envelope
/// byte-for-byte for callers that don't pass --values.
#[test]
fn values_json_no_values_key_absent() {
    let detail = base_envelope(None);
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&detail, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert!(
        parsed["data"].get("values").is_none(),
        "json data must NOT have 'values' key when values=None; got:\n{out}"
    );
    // Also assert the raw string doesn't mention "values" in data context.
    // (A "values" key in _meta would be wrong too, but we check data specifically.)
    let data_str = serde_json::to_string(&parsed["data"]).unwrap();
    assert!(
        !data_str.contains("\"values\""),
        "json data serialized must not contain '\"values\"' key when None; got data: {data_str}"
    );
}

// ── Tabular formats: --values renders value rows (ADR-0013 D1/D2) ─────────────
//
// Each format reuses one of the shared multi-type fixtures (a/b/c) for
// breadth across the suite. Stdout is snapshotted in the DEFAULT columns
// (field, field_label, value_type, value — label/iri are opt-in, D2); stderr
// is snapshotted separately and must carry ONLY the ADR-0007 disclosure line
// (the old "values not shown" note is gone).

/// Lines render of fixture A (scalars) with `--values`.
///
/// Locks: one tab-separated value row per value (compact default columns,
/// no header — lines convention); stderr carries only the disclosure line.
#[test]
fn values_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_a_scalars(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    // No stderr note; disclosure only.
    assert!(
        !stderr.contains("values not shown"),
        "lines stderr must not carry the old values note; got: {stderr:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "lines stderr must contain the disclosure filter_warning; got: {stderr:?}"
    );
    // Default columns (no label/iri) — a representative value row is present.
    assert!(
        stdout.contains("hasSequenceNumber\tSequence Number\tinteger\t42\n"),
        "lines stdout must show a default-columns value row; got:\n{stdout}"
    );
    assert!(
        !stdout.contains(&fixture_a_scalars().iri),
        "lines stdout must NOT show the resource iri by default (opt-in only); got:\n{stdout}"
    );
    insta::assert_snapshot!("values_lines_stdout", stdout);
    insta::assert_snapshot!("values_lines_stderr", stderr);
}

/// CSV render of fixture B (named scalars + raw) with `--values`.
///
/// Locks: header `field,field_label,value_type,value` + one row per value;
/// stderr carries only the disclosure line.
#[test]
fn values_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_b_named_scalars_and_raw(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    assert!(
        !stderr.contains("values not shown"),
        "csv stderr must not carry the old values note; got: {stderr:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "csv stderr must contain the disclosure filter_warning; got: {stderr:?}"
    );
    assert!(
        stdout.starts_with("field,field_label,value_type,value\n"),
        "csv values header must be the compact default set (no label/iri); got:\n{stdout}"
    );
    assert!(
        stdout.contains("hasColor,Color,color,#ff3300\n"),
        "csv stdout must show the color value row; got:\n{stdout}"
    );
    insta::assert_snapshot!("values_csv_stdout", stdout);
    insta::assert_snapshot!("values_csv_stderr", stderr);
}

/// TSV render of fixture C (links, vocabulary-items, files) with `--values`.
///
/// Locks: header `field\tfield_label\tvalue_type\tvalue` + one row per value;
/// stderr carries only the disclosure line.
#[test]
fn values_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_c_links_list_file(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    assert!(
        !stderr.contains("values not shown"),
        "tsv stderr must not carry the old values note; got: {stderr:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "tsv stderr must contain the disclosure filter_warning; got: {stderr:?}"
    );
    assert!(
        stdout.starts_with("field\tfield_label\tvalue_type\tvalue\n"),
        "tsv values header must be the compact default set (no label/iri); got:\n{stdout}"
    );
    assert!(
        stdout.contains("hasStillImageFileValue\t\tstill-image\tn6r.jp2 (2048\u{d7}3072) https://iiif.dasch.swiss/0803/n6r.jp2/full/max/0/default.jpg\n"),
        "tsv stdout must show the still-image value row; got:\n{stdout}"
    );
    insta::assert_snapshot!("values_tsv_stdout", stdout);
    insta::assert_snapshot!("values_tsv_stderr", stderr);
}

// ── W1: sanitization end-to-end contract ──────────────────────────────────────

/// Prose STRIPS control chars (incl. ANSI escape sequences) from server-supplied
/// value scalars; JSON keeps them raw (ADR-0003 fidelity).
///
/// A `ValueContent::Text` embedding a raw ANSI escape sequence is built and
/// rendered through both renderers:
///   - Prose: must NOT contain the ESC byte (0x1B); visible text ("danger", "RED",
///     "end") must survive.
///   - JSON: must STILL CONTAIN the raw ESC byte (verbatim server value).
///
/// A regressor that drops the `strip_control_chars` call in prose, OR that
/// wrongly strips in json, will fail this test.
#[test]
fn sanitization_prose_strips_esc_json_keeps_raw() {
    const ESC: char = '\u{1b}';
    // Embed a real ANSI CSI sequence: ESC [ 3 1 m (red) and ESC [ 0 m (reset).
    let danger_value = format!("danger {ESC}[31mRED{ESC}[0m end");

    let fixture = base_envelope(Some(vec![FieldValues {
        name: "hasDangerText".to_string(),
        label: Some("Danger Text".to_string()),
        values: vec![ValueContent::Text(danger_value.clone()).into()],
    }]));

    // ── Prose: ESC must be stripped, visible text must survive ─────────────────
    let (prose_buf, prose_w) = shared_buf();
    let mut prose_r = ProseRenderer::with_writer(prose_w);
    prose_r.resource_describe(&fixture, &anon_meta()).unwrap();
    let prose_out = buf_to_string(&prose_buf);

    assert!(
        !prose_out.contains(ESC),
        "prose must NOT contain ESC (0x1B) — strip_control_chars must be called on value scalars; got:\n{prose_out:?}"
    );
    assert!(
        prose_out.contains("danger"),
        "prose must keep visible text 'danger' after ESC stripping; got:\n{prose_out:?}"
    );
    assert!(
        prose_out.contains("RED"),
        "prose must keep visible text 'RED' after ESC stripping; got:\n{prose_out:?}"
    );
    assert!(
        prose_out.contains("end"),
        "prose must keep visible text 'end' after ESC stripping; got:\n{prose_out:?}"
    );

    // ── JSON: raw ESC must be preserved (ADR-0003 fidelity) ───────────────────
    //
    // The JSON serializer encodes the ESC byte (0x1B) as the JSON unicode
    // escape `` — it is NOT emitted as a raw ESC byte in the JSON text.
    // The fidelity contract is therefore: the JSON output must contain a
    // representation of the ESC character (either the raw byte OR its JSON
    // escape form ``), NOT strip it entirely the way prose does.
    let (json_buf, json_w) = shared_buf();
    let mut json_r = JsonRenderer::with_writer(json_w);
    json_r.resource_describe(&fixture, &anon_meta()).unwrap();
    let json_out = buf_to_string(&json_buf);

    // JSON serializers emit ESC as `` — check for either form.
    let json_has_esc = json_out.contains(ESC) || json_out.contains("\\u001b");
    assert!(
        json_has_esc,
        "json must STILL CONTAIN ESC (raw or as \\u001b) — fidelity contract requires no stripping; got:\n{json_out:?}"
    );
    // Also confirm no DSP-API vocab leaked through.
    assert!(
        !json_out.contains("knora-api:"),
        "json must not leak 'knora-api:' vocab; got:\n{json_out:?}"
    );
}

// ── S3: vocabulary-leak guard in json ──────────────────────────────────────────

/// The vocabulary-leak guard (`!out.contains("knora-api:")`) from the prose
/// snapshot tests must ALSO hold in a json values test.
///
/// This guards ADR-0001: rendered json output must not leak DSP-API vocabulary
/// through a value (e.g. if a Raw fallback accidentally emits the raw CURIE key).
#[test]
fn json_values_no_knora_api_vocab_leak() {
    // Use fixture A (all common scalar types) for breadth.
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&fixture_a_scalars(), &anon_meta())
        .unwrap();
    let json_out = buf_to_string(&buf);

    assert!(
        !json_out.contains("knora-api:"),
        "json output must NOT leak 'knora-api:' DSP-API vocabulary; got:\n{json_out}"
    );
    // Also check fixture C (links, vocabulary-items, files) which is richer.
    let (buf2, w2) = shared_buf();
    let mut r2 = JsonRenderer::with_writer(w2);
    r2.resource_describe(&fixture_c_links_list_file(), &anon_meta())
        .unwrap();
    let json_out2 = buf_to_string(&buf2);

    assert!(
        !json_out2.contains("knora-api:"),
        "json output (fixture C) must NOT leak 'knora-api:' vocabulary; got:\n{json_out2}"
    );
}

// ── Tabular --values: control-char neutralisation end-to-end ──────────────────
//
// Direct assertions (not snapshots — an ESC byte in a .snap file is
// unreadable). Proves THIS call site (the values-mode row builder feeding
// `render_table`) actually routes through the `QuoteMode::apply` →
// `replace_control_chars` chokepoint, closing the gap between "the engine
// neutralises generically" and "this new surface uses the engine".

/// Lines render of the control-char fixture: raw ESC/tab must not survive;
/// the neutralised row (each control char folded to a single space) must.
#[test]
fn tabular_values_lines_neutralises_control_chars() {
    const ESC: char = '\u{1b}';
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.resource_describe(&fixture_control_chars(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        !out.contains(ESC),
        "lines stdout must not contain the raw ESC byte; got:\n{out:?}"
    );
    // The raw embedded tab in the label must not survive as a literal
    // "Lab<TAB>el" fragment (the column-separator tabs are legitimate and
    // expected elsewhere in the row, so we check the specific fragment).
    assert!(
        !out.contains("Lab\tel"),
        "lines stdout must not contain the raw embedded tab from the label; got:\n{out:?}"
    );
    assert!(
        out.contains("hasControlChars\tLab el\ttext\ta b\n"),
        "lines stdout must show the neutralised value row (ESC/tab -> space); got:\n{out:?}"
    );
}

/// CSV render of the control-char fixture: tab is not a CSV delimiter, so a
/// stronger whole-output assertion applies — no raw ESC or tab anywhere.
#[test]
fn tabular_values_csv_neutralises_control_chars() {
    const ESC: char = '\u{1b}';
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.resource_describe(&fixture_control_chars(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        !out.contains(ESC),
        "csv stdout must not contain the raw ESC byte; got:\n{out:?}"
    );
    assert!(
        !out.contains('\t'),
        "csv stdout must not contain any raw tab char (not a csv delimiter); got:\n{out:?}"
    );
    assert!(
        out.contains("hasControlChars,Lab el,text,a b\n"),
        "csv stdout must show the neutralised value row (ESC/tab -> space); got:\n{out:?}"
    );
}

/// TSV render of the control-char fixture: raw ESC/tab must not survive;
/// the neutralised row must.
#[test]
fn tabular_values_tsv_neutralises_control_chars() {
    const ESC: char = '\u{1b}';
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.resource_describe(&fixture_control_chars(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        !out.contains(ESC),
        "tsv stdout must not contain the raw ESC byte; got:\n{out:?}"
    );
    assert!(
        !out.contains("Lab\tel"),
        "tsv stdout must not contain the raw embedded tab from the label; got:\n{out:?}"
    );
    assert!(
        out.contains("hasControlChars\tLab el\ttext\ta b\n"),
        "tsv stdout must show the neutralised value row (ESC/tab -> space); got:\n{out:?}"
    );
}

// ── Tabular --values: empty-values shape (Some(vec![])) ───────────────────────
//
// Spec (plan Test plan section): header row per HeaderMode (csv/tsv when on;
// none for lines), zero data rows, no placeholder row — analogous to
// `resource_types_csv_empty` (src/render/csv.rs). The ADR-0007 disclosure
// line still appears on stderr regardless of how many values there are.

/// CSV: header-only, zero data rows, for `values = Some(vec![])`.
#[test]
fn tabular_values_csv_empty_header_only_no_rows() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_empty_values(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    assert_eq!(
        stdout, "field,field_label,value_type,value\n",
        "csv values header row only, zero data rows, for empty values; got:\n{stdout:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "csv stderr must still carry the disclosure filter_warning when values is empty; got: {stderr:?}"
    );
}

/// TSV: header-only, zero data rows, for `values = Some(vec![])`.
#[test]
fn tabular_values_tsv_empty_header_only_no_rows() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_empty_values(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    assert_eq!(
        stdout, "field\tfield_label\tvalue_type\tvalue\n",
        "tsv values header row only, zero data rows, for empty values; got:\n{stdout:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "tsv stderr must still carry the disclosure filter_warning when values is empty; got: {stderr:?}"
    );
}

/// Lines: no header convention, zero rows -> stdout is entirely empty for
/// `values = Some(vec![])`.
#[test]
fn tabular_values_lines_empty_stdout_is_empty() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_describe(&fixture_empty_values(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);

    assert_eq!(
        stdout, "",
        "lines stdout must be entirely empty (no header, no rows) for empty values; got:\n{stdout:?}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "lines stderr must still carry the disclosure filter_warning when values is empty; got: {stderr:?}"
    );
}

// ── Tabular --values: --columns projection opts resource keys back in ────────
//
// D2: `label`/`iri` are opt-in in values mode. Proves the caller can opt them
// back in via `--columns` and that unlisted columns (here, `field_label`) are
// correctly dropped from the projected header + rows.

/// CSV `--columns label,iri,field,value_type,value`: header is exactly the
/// projected list (in the requested order); resource label/iri (hidden by
/// default) now appear in the data rows; `field_label` (dropped) does not.
#[test]
fn tabular_values_csv_columns_projection_opts_in_resource_keys() {
    let opts = TableOptions {
        columns: Some(
            ["label", "iri", "field", "value_type", "value"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        header: HeaderMode::On,
    };
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(opts);
    let detail = fixture_a_scalars();
    r.resource_describe(&detail, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    assert!(
        out.starts_with("label,iri,field,value_type,value\n"),
        "csv header must be exactly the projected column list, in the requested order; got:\n{out}"
    );
    assert!(
        out.contains(&detail.label),
        "csv data rows must show the resource label once opted in via --columns; got:\n{out}"
    );
    assert!(
        out.contains(&detail.iri),
        "csv data rows must show the resource iri once opted in via --columns; got:\n{out}"
    );
    // field_label ("Sequence Number", the label for hasSequenceNumber) is not
    // in the projection, so it must not appear anywhere in the projected output.
    assert!(
        !out.contains("Sequence Number"),
        "csv output must NOT show field_label when it is not in the --columns projection; got:\n{out}"
    );
}

// ── Tabular --values: --columns opts in the per-value `comment` column ───────

/// CSV `--columns field,value,comment`: the `comment` column is opt-in (not in
/// the default set) and, once projected, carries the per-value comment text.
#[test]
fn values_csv_columns_comment() {
    let opts = TableOptions {
        columns: Some(
            ["field", "value", "comment"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        header: HeaderMode::On,
    };
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w).with_options(opts);
    r.resource_describe(&fixture_d_with_comment(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);

    assert!(
        out.starts_with("field,value,comment\n"),
        "csv header must be exactly the projected column list; got:\n{out}"
    );
    assert!(
        out.contains("hasTranscription,some transcription,reading uncertain\n"),
        "csv data row must show field/value/comment; got:\n{out}"
    );
    insta::assert_snapshot!("values_csv_columns_comment", out);
}
