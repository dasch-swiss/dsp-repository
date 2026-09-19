//! Shared value-rendering surface (D3 of plan 027).
//!
//! `render_value_content` and `build_value_rows` are consumed by `prose` and
//! (from a later step) the three tabular renderers, so both produce
//! identical value strings from one place. This module also holds the
//! relocated date-formatting helpers, which only `render_value_content` uses.

use crate::model::{DatePoint, DateValue, FieldValues, ResourceDetail, ValueContent, ValueType};

/// Renders the RAW display string for one value.
///
/// Mirrors, arm-for-arm, the per-`ValueContent` display logic that used to be
/// inlined in `prose.rs::resource_describe`.
///
/// SECURITY: this returns the raw display string with **no control-character
/// handling**. Any caller rendering to a terminal MUST sanitise first —
/// `prose` applies one outer `strip_control_chars` to the whole rendered
/// string; the tabular renderers feed this string into `render_table`, whose
/// `QuoteMode::apply` → `replace_control_chars` chokepoint neutralises every
/// cell. Reintroducing an unsanitised print path is the 2026-06-19 tsv-bug
/// class (the ADR-0003 control-char amendment).
pub(crate) fn render_value_content(value: &ValueContent) -> String {
    match value {
        ValueContent::Text(s) => s.clone(),
        ValueContent::Integer(n) => n.to_string(),
        ValueContent::Decimal(s) => s.clone(),
        ValueContent::Boolean(b) => b.to_string(),
        ValueContent::Date(dv) => format_date_value(dv),
        ValueContent::Time(s) => s.clone(),
        ValueContent::Uri(s) => s.clone(),
        ValueContent::Color(s) => s.clone(),
        ValueContent::Geoname(s) => s.clone(),
        ValueContent::VocabularyItem { node_iri, label } => match label.as_deref() {
            Some(lbl) => lbl.to_string(),
            None => node_iri.clone(),
        },
        ValueContent::Link {
            target_iri,
            target_label,
        } => match target_label.as_deref() {
            Some(lbl) => format!("\u{2192} {lbl} [{target_iri}]"),
            None => format!("\u{2192} {target_iri}"),
        },
        ValueContent::File(fv) => {
            // StillImage with both dimensions known inserts a `(WxH)` marker between
            // filename and url; every other case (including StillImage without full
            // dimensions) falls back to the same plain "filename url" shape.
            let dims = match fv.value_type {
                ValueType::StillImage => match (fv.width, fv.height) {
                    (Some(w), Some(h)) => Some((w, h)),
                    _ => None,
                },
                _ => None,
            };
            match dims {
                Some((w, h)) => format!("{} ({w}\u{d7}{h}) {}", fv.filename, fv.url),
                None => format!("{} {}", fv.filename, fv.url),
            }
        }
        ValueContent::Raw { text, .. } => text.clone(),
    }
}

/// Builds one row per value, iterating fields in order then each field's
/// values in order.
///
/// Each row is a 7-element `Vec<String>` in the column order
/// `[label, iri, field, field_label, value_type, value, comment]`. No
/// sanitisation here — the tabular engine's `render_table` chokepoint handles
/// it (see the `SECURITY` note on `render_value_content`).
pub(crate) fn build_value_rows(label: &str, iri: &str, fields: &[FieldValues]) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for field in fields {
        for value in &field.values {
            rows.push(vec![
                label.to_string(),
                iri.to_string(),
                field.name.clone(),
                field.label.as_deref().unwrap_or("").to_string(),
                value.content.value_type_token().to_string(),
                render_value_content(&value.content),
                value.comment.as_deref().unwrap_or("").to_string(),
            ]);
        }
    }
    rows
}

/// Builds the single metadata-mode row for `resource describe` (no `--values`).
///
/// The 10-column `RESOURCE_DESCRIBE_COLUMNS` order:
/// `label, iri, resource_type, ark_url, creation_date, last_modified,
/// attached_project, owner, visibility, your_access`. Optional fields render
/// as an empty string when `None` — mirrors the pre-extraction inline blocks
/// in `lines`/`csv`/`tsv` byte-for-byte; shared here so the three tabular
/// renderers' metadata mode can't drift from one another.
pub(crate) fn build_metadata_row(detail: &ResourceDetail) -> Vec<String> {
    vec![
        detail.label.clone(),
        detail.iri.clone(),
        detail.resource_type.clone(),
        detail.ark_url.as_deref().unwrap_or("").to_string(),
        detail.creation_date.as_deref().unwrap_or("").to_string(),
        detail.last_modified.as_deref().unwrap_or("").to_string(),
        detail.attached_project.as_deref().unwrap_or("").to_string(),
        detail.owner.as_deref().unwrap_or("").to_string(),
        detail
            .visibility
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        detail
            .your_access
            .as_ref()
            .map(|a| a.as_str())
            .unwrap_or("")
            .to_string(),
    ]
}

/// Format a single `DatePoint` to a human-readable string.
///
/// Precision follows the fields present: year only, year-month, or
/// year-month-day. Era is appended when present. Returns an empty string when
/// `year` is `None` (graceful degradation — never panics).
fn format_date_point(p: &DatePoint) -> String {
    let year = match p.year {
        Some(y) => y,
        None => return String::new(),
    };
    let mut s = match (p.month, p.day) {
        (Some(m), Some(d)) => format!("{year:04}-{m:02}-{d:02}"),
        (Some(m), None) => format!("{year:04}-{m:02}"),
        _ => format!("{year}"),
    };
    if let Some(era) = p.era.as_deref() {
        s.push(' ');
        s.push_str(era);
    }
    s
}

/// Format a `DateValue` as a human-readable string.
///
/// Collapses a range to a point when `start == end` or when only one endpoint
/// has a year. Format:
/// `<start> – <end> (<CALENDAR>)` for a full range (both endpoints non-empty);
/// `<point> (<CALENDAR>)` for a single point or a one-sided range;
/// `""` when both endpoints format to empty.
fn format_date_value(dv: &DateValue) -> String {
    let start_str = format_date_point(&dv.start);
    let end_str = format_date_point(&dv.end);

    match (start_str.is_empty(), end_str.is_empty()) {
        // Both empty — nothing useful to show.
        (true, true) => String::new(),
        // Only start is present (or start == end) — render as a single point.
        (false, true) => format!("{} ({})", start_str, dv.calendar),
        // Only end is present — render as a single point.
        (true, false) => format!("{} ({})", end_str, dv.calendar),
        // Both non-empty: collapse equal endpoints to a point; otherwise range.
        (false, false) => {
            if start_str == end_str {
                format!("{} ({})", start_str, dv.calendar)
            } else {
                format!("{} \u{2013} {} ({})", start_str, end_str, dv.calendar)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileValue;

    // ── render_value_content: scalars ─────────────────────────────────────

    #[test]
    fn render_value_content_text() {
        assert_eq!(
            render_value_content(&ValueContent::Text("hello world".into())),
            "hello world"
        );
    }

    #[test]
    fn render_value_content_integer() {
        assert_eq!(render_value_content(&ValueContent::Integer(42)), "42");
    }

    #[test]
    fn render_value_content_decimal() {
        assert_eq!(
            render_value_content(&ValueContent::Decimal("3.14".into())),
            "3.14"
        );
    }

    #[test]
    fn render_value_content_boolean() {
        assert_eq!(render_value_content(&ValueContent::Boolean(true)), "true");
        assert_eq!(render_value_content(&ValueContent::Boolean(false)), "false");
    }

    #[test]
    fn render_value_content_time() {
        assert_eq!(
            render_value_content(&ValueContent::Time("2021-01-01T00:00:00Z".into())),
            "2021-01-01T00:00:00Z"
        );
    }

    #[test]
    fn render_value_content_uri() {
        assert_eq!(
            render_value_content(&ValueContent::Uri("https://example.com".into())),
            "https://example.com"
        );
    }

    #[test]
    fn render_value_content_color() {
        assert_eq!(
            render_value_content(&ValueContent::Color("#ff0000".into())),
            "#ff0000"
        );
    }

    #[test]
    fn render_value_content_geoname() {
        assert_eq!(
            render_value_content(&ValueContent::Geoname("2661552".into())),
            "2661552"
        );
    }

    // ── render_value_content: date ────────────────────────────────────────

    #[test]
    fn render_value_content_date() {
        let pt = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: pt.clone(),
            end: pt,
        };
        assert_eq!(
            render_value_content(&ValueContent::Date(dv)),
            "1489 CE (GREGORIAN)"
        );
    }

    // ── render_value_content: vocabulary-item ───────────────────────────────

    #[test]
    fn render_value_content_vocabulary_item_with_label() {
        let v = ValueContent::VocabularyItem {
            node_iri: "http://rdfh.ch/lists/0001/abc".into(),
            label: Some("Red".into()),
        };
        assert_eq!(render_value_content(&v), "Red");
    }

    #[test]
    fn render_value_content_vocabulary_item_without_label() {
        let v = ValueContent::VocabularyItem {
            node_iri: "http://rdfh.ch/lists/0001/abc".into(),
            label: None,
        };
        assert_eq!(render_value_content(&v), "http://rdfh.ch/lists/0001/abc");
    }

    // ── render_value_content: link ────────────────────────────────────────

    #[test]
    fn render_value_content_link_with_label() {
        let v = ValueContent::Link {
            target_iri: "http://rdfh.ch/0803/book1".into(),
            target_label: Some("Incunabula Book".into()),
        };
        assert_eq!(
            render_value_content(&v),
            "\u{2192} Incunabula Book [http://rdfh.ch/0803/book1]"
        );
    }

    #[test]
    fn render_value_content_link_without_label() {
        let v = ValueContent::Link {
            target_iri: "http://rdfh.ch/0803/book2".into(),
            target_label: None,
        };
        assert_eq!(
            render_value_content(&v),
            "\u{2192} http://rdfh.ch/0803/book2"
        );
    }

    // ── render_value_content: file (split per kind) ───────────────────────

    #[test]
    fn render_value_content_file_still_image_with_dims() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::StillImage,
            filename: "image.jp2".into(),
            url: "https://iiif.example.com/image.jp2/full/max/0/default.jpg".into(),
            width: Some(1200),
            height: Some(800),
        });
        assert_eq!(
            render_value_content(&v),
            "image.jp2 (1200\u{d7}800) https://iiif.example.com/image.jp2/full/max/0/default.jpg"
        );
    }

    #[test]
    fn render_value_content_file_still_image_without_dims() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::StillImage,
            filename: "image.jp2".into(),
            url: "https://iiif.example.com/image.jp2".into(),
            width: None,
            height: None,
        });
        assert_eq!(
            render_value_content(&v),
            "image.jp2 https://iiif.example.com/image.jp2"
        );
    }

    #[test]
    fn render_value_content_file_moving_image() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::MovingImage,
            filename: "clip.mp4".into(),
            url: "https://example.com/clip.mp4".into(),
            width: None,
            height: None,
        });
        assert_eq!(
            render_value_content(&v),
            "clip.mp4 https://example.com/clip.mp4"
        );
    }

    #[test]
    fn render_value_content_file_audio() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::Audio,
            filename: "track.mp3".into(),
            url: "https://example.com/track.mp3".into(),
            width: None,
            height: None,
        });
        assert_eq!(
            render_value_content(&v),
            "track.mp3 https://example.com/track.mp3"
        );
    }

    #[test]
    fn render_value_content_file_document() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::Document,
            filename: "report.pdf".into(),
            url: "https://example.com/report.pdf".into(),
            width: None,
            height: None,
        });
        assert_eq!(
            render_value_content(&v),
            "report.pdf https://example.com/report.pdf"
        );
    }

    #[test]
    fn render_value_content_file_archive() {
        let v = ValueContent::File(FileValue {
            value_type: ValueType::Archive,
            filename: "bundle.zip".into(),
            url: "https://example.com/bundle.zip".into(),
            width: None,
            height: None,
        });
        assert_eq!(
            render_value_content(&v),
            "bundle.zip https://example.com/bundle.zip"
        );
    }

    // ── render_value_content: raw fallback ────────────────────────────────

    #[test]
    fn render_value_content_raw() {
        let v = ValueContent::Raw {
            value_type: "interval".into(),
            text: "1.0 - 2.0".into(),
        };
        assert_eq!(render_value_content(&v), "1.0 - 2.0");
    }

    // ── build_value_rows ───────────────────────────────────────────────────

    #[test]
    fn build_value_rows_two_fields_one_multi_valued() {
        let fields = vec![
            FieldValues {
                name: "hasTitle".into(),
                label: Some("Title".into()),
                values: vec![ValueContent::Text("The Book".into()).into()],
            },
            FieldValues {
                name: "hasKeyword".into(),
                label: None,
                values: vec![
                    ValueContent::Text("history".into()).into(),
                    ValueContent::Text("incunabula".into()).into(),
                ],
            },
        ];
        let rows = build_value_rows("My Book", "http://rdfh.ch/0803/book1", &fields);

        assert_eq!(rows.len(), 3, "one row per value across both fields");

        assert_eq!(
            rows[0],
            vec![
                "My Book".to_string(),
                "http://rdfh.ch/0803/book1".to_string(),
                "hasTitle".to_string(),
                "Title".to_string(),
                "text".to_string(),
                "The Book".to_string(),
                "".to_string(),
            ]
        );
        assert_eq!(
            rows[1],
            vec![
                "My Book".to_string(),
                "http://rdfh.ch/0803/book1".to_string(),
                "hasKeyword".to_string(),
                "".to_string(),
                "text".to_string(),
                "history".to_string(),
                "".to_string(),
            ]
        );
        assert_eq!(
            rows[2],
            vec![
                "My Book".to_string(),
                "http://rdfh.ch/0803/book1".to_string(),
                "hasKeyword".to_string(),
                "".to_string(),
                "text".to_string(),
                "incunabula".to_string(),
                "".to_string(),
            ]
        );
    }

    #[test]
    fn build_value_rows_carries_comment_in_seventh_cell() {
        use crate::model::Value;

        let fields = vec![FieldValues {
            name: "hasTranscription".into(),
            label: Some("Transcription".into()),
            values: vec![Value {
                content: ValueContent::Text("teh book".into()),
                comment: Some("reading uncertain".into()),
            }],
        }];
        let rows = build_value_rows("My Book", "http://rdfh.ch/0803/book1", &fields);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            vec![
                "My Book".to_string(),
                "http://rdfh.ch/0803/book1".to_string(),
                "hasTranscription".to_string(),
                "Transcription".to_string(),
                "text".to_string(),
                "teh book".to_string(),
                "reading uncertain".to_string(),
            ]
        );
    }

    // ── build_metadata_row ──────────────────────────────────────────────────

    #[test]
    fn build_metadata_row_all_fields_present() {
        use crate::model::{ResourceAccess, ResourceDetail, ResourceVisibility};

        let detail = ResourceDetail {
            label: "My Book".into(),
            iri: "http://rdfh.ch/0803/book1".into(),
            resource_type: "Book".into(),
            ark_url: Some("https://ark.example.org/00000/1/book1".into()),
            creation_date: Some("2021-01-01T00:00:00Z".into()),
            last_modified: Some("2021-06-01T00:00:00Z".into()),
            attached_project: Some("http://rdfh.ch/projects/0803".into()),
            owner: Some("http://rdfh.ch/users/abc".into()),
            visibility: Some(ResourceVisibility::Public),
            your_access: Some(ResourceAccess::View),
            values: None,
        };

        assert_eq!(
            build_metadata_row(&detail),
            vec![
                "My Book".to_string(),
                "http://rdfh.ch/0803/book1".to_string(),
                "Book".to_string(),
                "https://ark.example.org/00000/1/book1".to_string(),
                "2021-01-01T00:00:00Z".to_string(),
                "2021-06-01T00:00:00Z".to_string(),
                "http://rdfh.ch/projects/0803".to_string(),
                "http://rdfh.ch/users/abc".to_string(),
                "public".to_string(),
                "view".to_string(),
            ]
        );
    }

    #[test]
    fn build_metadata_row_optional_fields_absent() {
        use crate::model::ResourceDetail;

        let detail = ResourceDetail {
            label: "Minimal".into(),
            iri: "http://rdfh.ch/0803/minimal".into(),
            resource_type: "Page".into(),
            ark_url: None,
            creation_date: None,
            last_modified: None,
            attached_project: None,
            owner: None,
            visibility: None,
            your_access: None,
            values: None,
        };

        assert_eq!(
            build_metadata_row(&detail),
            vec![
                "Minimal".to_string(),
                "http://rdfh.ch/0803/minimal".to_string(),
                "Page".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ]
        );
    }

    // ── date formatting helper tests (relocated from prose.rs) ────────────

    #[test]
    fn format_date_point_year_only() {
        let p = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: None,
        };
        assert_eq!(format_date_point(&p), "1489");
    }

    #[test]
    fn format_date_point_year_month() {
        let p = DatePoint {
            year: Some(1456),
            month: Some(3),
            day: None,
            era: Some("CE".into()),
        };
        assert_eq!(format_date_point(&p), "1456-03 CE");
    }

    #[test]
    fn format_date_point_full_date() {
        let p = DatePoint {
            year: Some(1456),
            month: Some(3),
            day: Some(14),
            era: Some("CE".into()),
        };
        assert_eq!(format_date_point(&p), "1456-03-14 CE");
    }

    #[test]
    fn format_date_point_none_year_returns_empty() {
        let p = DatePoint {
            year: None,
            month: Some(3),
            day: Some(14),
            era: Some("CE".into()),
        };
        assert_eq!(format_date_point(&p), "");
    }

    #[test]
    fn format_date_value_single_point() {
        let pt = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: pt.clone(),
            end: pt,
        };
        assert_eq!(format_date_value(&dv), "1489 CE (GREGORIAN)");
    }

    #[test]
    fn format_date_value_range() {
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: Some(1490),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
        };
        assert_eq!(
            format_date_value(&dv),
            "1489 CE \u{2013} 1490 CE (GREGORIAN)"
        );
    }

    #[test]
    fn format_date_value_julian_full_day() {
        let dv = DateValue {
            calendar: "JULIAN".into(),
            start: DatePoint {
                year: Some(1456),
                month: Some(3),
                day: Some(14),
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: Some(1456),
                month: Some(3),
                day: Some(14),
                era: Some("CE".into()),
            },
        };
        assert_eq!(format_date_value(&dv), "1456-03-14 CE (JULIAN)");
    }

    #[test]
    fn format_date_value_none_year_returns_empty() {
        let pt = DatePoint {
            year: None,
            month: None,
            day: None,
            era: None,
        };
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: pt.clone(),
            end: pt,
        };
        assert_eq!(format_date_value(&dv), "");
    }

    /// One-sided date: start has a year, end has `year: None`.
    /// The formatter must emit the start point only — no trailing dash or empty half.
    #[test]
    fn format_date_value_one_sided_start_only() {
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: None,
                month: None,
                day: None,
                era: None,
            },
        };
        // Must render the non-empty endpoint as a single point — no dash, no empty half.
        assert_eq!(format_date_value(&dv), "1489 CE (GREGORIAN)");
    }

    /// Symmetric case: end has a year, start has `year: None`.
    #[test]
    fn format_date_value_one_sided_end_only() {
        let dv = DateValue {
            calendar: "JULIAN".into(),
            start: DatePoint {
                year: None,
                month: None,
                day: None,
                era: None,
            },
            end: DatePoint {
                year: Some(1490),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
        };
        assert_eq!(format_date_value(&dv), "1490 CE (JULIAN)");
    }
}
