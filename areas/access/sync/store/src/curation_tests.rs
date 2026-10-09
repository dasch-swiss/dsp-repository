use std::collections::BTreeSet;

use cpe_ports::{CuratedValue, ResourceIri};

use crate::curation::parse;
use crate::{CurationFault, InvalidCuration};

const X: &str = "http://rdfh.ch/0803/xA1b";
const Y: &str = "http://rdfh.ch/0803/yB2c";
const Z: &str = "http://rdfh.ch/0803/zC3d";
/// An IRI outside `known()`.
const U: &str = "http://rdfh.ch/0803/uD4e";

/// The served resources of every test: `X`, `Y` and `Z`, which sort in that order.
fn known() -> BTreeSet<&'static str> {
    BTreeSet::from([X, Y, Z])
}

fn value(resource: &str, key: &str, lang: Option<&str>, text: &str) -> CuratedValue {
    CuratedValue {
        resource: ResourceIri(resource.to_string()),
        key: key.to_string(),
        lang: lang.map(str::to_string),
        text: text.to_string(),
    }
}

fn values_of(input: &str) -> Vec<CuratedValue> {
    parse(input.as_bytes(), &known()).unwrap_or_else(|fault| panic!("{input:?}: {fault}"))
}

fn fault_of(input: impl AsRef<[u8]>) -> CurationFault {
    let input = input.as_ref();
    match parse(input, &known()) {
        Ok(values) => panic!("{:?} is served: {values:?}", String::from_utf8_lossy(input)),
        Err(fault) => fault,
    }
}

fn fault(line: usize, reason: InvalidCuration) -> CurationFault {
    CurationFault { line, reason }
}

/// The fault of a file whose header is `iri,a` and whose line 2 is `X,<cell>`.
fn fault_of_cell(cell: &str) -> CurationFault {
    fault_of(format!("iri,a\n{X},{cell}\n"))
}

// Values

#[test]
fn test_curation_parse_values_returns_them_sorted() {
    let cases = [
        ("iri\n".to_string(), vec![]),
        ("iri".to_string(), vec![]),
        (
            format!("iri,colour,size\n{X},red,big\n"),
            vec![value(X, "colour", None, "red"), value(X, "size", None, "big")],
        ),
        (
            format!("iri,caption@de,caption@en\n{X},Rot,Red"),
            vec![
                value(X, "caption", Some("de"), "Rot"),
                value(X, "caption", Some("en"), "Red"),
            ],
        ),
        (
            format!("iri,size,caption@en,caption,colour\n{Y},s,c,b,r\n{X},big,,plain,\n"),
            vec![
                value(X, "caption", None, "plain"),
                value(X, "size", None, "big"),
                value(Y, "caption", None, "b"),
                value(Y, "caption", Some("en"), "c"),
                value(Y, "colour", None, "r"),
                value(Y, "size", None, "s"),
            ],
        ),
        (format!("iri,\"colour\"\n{X},red\n"), vec![value(X, "colour", None, "red")]),
    ];

    for (input, expected) in cases {
        assert_eq!(values_of(&input), expected, "{input:?}");
    }
}

#[test]
fn test_curation_parse_empty_cells_are_absent() {
    let cases = [
        (format!("iri,a,b\n{X},,v\n"), vec![value(X, "b", None, "v")]),
        (format!("iri,a,b\n{X},\"\",v\n"), vec![value(X, "b", None, "v")]),
        (format!("iri,a,b\n{X},v,\n"), vec![value(X, "a", None, "v")]),
        (format!("iri,a,b,c\n{X},v,,\n"), vec![value(X, "a", None, "v")]),
        (format!("iri,a,b\n{X},,\n"), vec![]),
        // A comment column's cell is never served and may be padded.
        (format!("iri,a,#note,#\n{X},v, kept ,x\n"), vec![value(X, "a", None, "v")]),
    ];

    for (input, expected) in cases {
        assert_eq!(values_of(&input), expected, "{input:?}");
    }
}

#[test]
fn test_curation_parse_text_is_verbatim() {
    let cases = [
        (r#""red, dark""#, "red, dark"),
        (r#""say ""hi""""#, r#"say "hi""#),
        (r#""""""#, r#"""#),
        ("a  b", "a  b"),
        ("März für", "März für"),
        // Inside a value, a zero-width space and a byte-order mark are served as written.
        ("a\u{200b}b", "a\u{200b}b"),
        ("a\u{feff}b", "a\u{feff}b"),
    ];

    for (cell, text) in cases {
        assert_eq!(
            values_of(&format!("iri,a\n{X},{cell}\n")),
            [value(X, "a", None, text)],
            "{cell:?}"
        );
    }
}

// Faults of the file as a whole

#[test]
fn test_curation_parse_invalid_bytes_return_not_utf8() {
    let after_two_lines = [format!("iri\n{X}\n").as_bytes(), &[0xFF]].concat();
    // The carriage return of line 1 is a fault too, but decoding comes first.
    let after_three_lines = [format!("iri,a\r\n{X},v\r\n{Y},v\r\n{Z},").as_bytes(), &[0xFF]].concat();

    assert_eq!(fault_of(after_two_lines), fault(3, InvalidCuration::NotUtf8));
    assert_eq!(fault_of(after_three_lines), fault(4, InvalidCuration::NotUtf8));
}

#[test]
fn test_curation_parse_no_header_returns_missing_header() {
    assert_eq!(fault_of(""), fault(1, InvalidCuration::MissingHeader));
    assert_eq!(fault_of(format!("\n{X},v\n")), fault(1, InvalidCuration::MissingHeader));
}

// Faults of a line's cells

#[test]
fn test_curation_parse_refused_characters_return_control_character() {
    let control = |character| InvalidCuration::ControlCharacter { character };
    let cases = [
        (format!("iri,a\n{X},v\n{Y},v\tw\n"), fault(3, control('\t'))),
        (format!("iri,a\r\n{X},v\r\n"), fault(1, control('\r'))),
        (format!("iri,a\n{X},a\u{85}b"), fault(2, control('\u{85}'))),
        (format!("iri,a\n{X},a\u{2028}b"), fault(2, control('\u{2028}'))),
        (format!("iri,a\n{X},a\u{2029}b"), fault(2, control('\u{2029}'))),
        ("iri,\"a\tb\"\n".to_string(), fault(1, control('\t'))),
        (format!("iri,a,#note\n{X},v,n\t\n"), fault(2, control('\t'))),
        // The quote never closes, but the tab is met first.
        (format!("iri,a\n{X},\"a\tb"), fault(2, control('\t'))),
    ];

    for (input, expected) in cases {
        assert_eq!(fault_of(&input), expected, "{input:?}");
    }
}

#[test]
fn test_curation_parse_misplaced_quotes_return_stray_quote() {
    for cell in [r#"a"b"#, r#" "b""#, r#""a"b"#, r#""a" ,b"#, r#"""x"#] {
        assert_eq!(fault_of_cell(cell), fault(2, InvalidCuration::StrayQuote), "{cell:?}");
    }
}

#[test]
fn test_curation_parse_open_quotes_return_unterminated_quote() {
    let cases = [
        (format!("iri,a\n{X},\"a\"\"\n"), 2),
        (format!("iri,a\n{X},\"\"\"\n"), 2),
        (format!("iri,a\n{X},\"\n"), 2),
        // A quoted cell cannot cross a line: the fault is on the line the quote opens on.
        (format!("iri,a\n{X},v\n{Y},\"one\ntwo\"\n"), 3),
        (format!("iri,a\n{X},\"v"), 2),
        ("iri,\"a\n".to_string(), 1),
    ];

    for (input, line) in cases {
        assert_eq!(fault_of(&input), fault(line, InvalidCuration::UnterminatedQuote), "{input:?}");
    }
}

// Faults of the header

#[test]
fn test_curation_parse_wrong_first_column_returns_first_column_not_iri() {
    for (header, found) in [
        ("id,a", "id"),
        ("\u{feff}iri,a", "\u{feff}iri"),
        ("Teaser,Teaser", "Teaser"),
    ] {
        assert_eq!(
            fault_of(header),
            fault(1, InvalidCuration::FirstColumnNotIri { found: found.to_string() }),
            "{header:?}"
        );
    }
}

#[test]
fn test_curation_parse_bad_column_names_return_malformed_column() {
    let cases = [
        ("iri,Caption", "Caption"),
        ("iri,caption@", "caption@"),
        ("iri,@de", "@de"),
        ("iri,a@b@c", "a@b@c"),
        ("iri,1st", "1st"),
        ("iri,", ""),
        ("iri,caption@de-CH", "caption@de-CH"),
        // The first `Teaser` is the fault; the repeat is never reached.
        ("iri,Teaser,Teaser", "Teaser"),
    ];

    for (header, column) in cases {
        assert_eq!(
            fault_of(header),
            fault(1, InvalidCuration::MalformedColumn { column: column.to_string() }),
            "{header:?}"
        );
    }
}

#[test]
fn test_curation_parse_repeated_columns_return_duplicate_column() {
    let cases = [
        ("iri,slug,slug", "slug"),
        ("iri,caption@de,caption@de", "caption@de"),
        ("iri,#note,#note", "#note"),
        ("iri,iri", "iri"),
    ];

    for (header, column) in cases {
        assert_eq!(
            fault_of(header),
            fault(1, InvalidCuration::DuplicateColumn { column: column.to_string() }),
            "{header:?}"
        );
    }
}

// Faults of a row

#[test]
fn test_curation_parse_empty_lines_return_blank_line() {
    assert_eq!(fault_of("iri\n\n"), fault(2, InvalidCuration::BlankLine));
    assert_eq!(
        fault_of(format!("iri,a\n{X},v\n\n{Y},v\n")),
        fault(3, InvalidCuration::BlankLine)
    );
}

#[test]
fn test_curation_parse_uneven_rows_return_wrong_cell_count() {
    let cases = [
        (format!("iri,a,b\n{X},v\n"), 3, 2),
        (format!("iri,a\n{X},v,w\n"), 2, 3),
        // The padded cell is not reached.
        (format!("iri,a,b\n{X}, v\n"), 3, 2),
    ];

    for (input, expected, found) in cases {
        assert_eq!(
            fault_of(&input),
            fault(2, InvalidCuration::WrongCellCount { expected, found }),
            "{input:?}"
        );
    }
}

#[test]
fn test_curation_parse_rows_for_other_iris_return_unknown_resource() {
    let cases = [
        (format!("{U},v"), U),
        (",v".to_string(), ""),
        // The padded cell is not reached.
        (format!("{U}, v"), U),
    ];

    for (row, iri) in cases {
        assert_eq!(
            fault_of(format!("iri,a\n{row}\n")),
            fault(2, InvalidCuration::UnknownResource { iri: iri.to_string() }),
            "{row:?}"
        );
    }
}

#[test]
fn test_curation_parse_second_row_for_an_iri_returns_duplicate_row() {
    assert_eq!(
        fault_of(format!("iri,a\n{X},v\n{Y},v\n{X},w\n")),
        fault(4, InvalidCuration::DuplicateRow { iri: X.to_string(), first_line: 2 })
    );
}

#[test]
fn test_curation_parse_padded_values_return_padded_value() {
    let padded = |column: &str| fault(2, InvalidCuration::PaddedValue { column: column.to_string() });

    for cell in [" v", "v ", "   ", "v\u{a0}", "\u{200b}v", "v\u{feff}", r#"" a""#] {
        assert_eq!(fault_of_cell(cell), padded("a"), "{cell:?}");
    }
    assert_eq!(fault_of(format!("iri,caption@de\n{X}, v\n")), padded("caption@de"));
}

#[test]
fn test_curation_parse_returns_the_first_fault_only() {
    let cases = [
        (
            format!("iri,A\n{U}, v\n"),
            fault(1, InvalidCuration::MalformedColumn { column: "A".to_string() }),
        ),
        (
            format!("iri,a\n{X}, v\n{U},v\n"),
            fault(2, InvalidCuration::PaddedValue { column: "a".to_string() }),
        ),
        (format!("iri,slug\n{X},a\"b\n{Y},\tc\n"), fault(2, InvalidCuration::StrayQuote)),
        // Within one row, the IRI's faults come before a value's.
        (
            format!("iri,a\n{X},v\n{X}, w\n"),
            fault(3, InvalidCuration::DuplicateRow { iri: X.to_string(), first_line: 2 }),
        ),
        // A misplaced quote comes before the row's cell count and its IRI.
        (format!("iri,a\n{U},v,w\"x\n"), fault(2, InvalidCuration::StrayQuote)),
        (
            format!("iri,a\n{X},v\n{X},w\tx\n"),
            fault(3, InvalidCuration::ControlCharacter { character: '\t' }),
        ),
        // Of two padded values, the leftmost.
        (
            format!("iri,a,b\n{X}, v, w\n"),
            fault(2, InvalidCuration::PaddedValue { column: "a".to_string() }),
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(fault_of(&input), expected, "{input:?}");
    }
}
