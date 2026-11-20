//! Fixture tests for value nodes and their kinds (§3–§5).

use std::collections::BTreeMap;

use oxigraph::model::Literal;

use super::index::Index;
use super::list::LiveLists;
use super::tests::{Fixture, ZEBRA, boolean, build_public, onto, output, report, resource_lines};
use super::value::{self, ValueKind};
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, vocab};
use crate::model::{self, kb, nn};

const GRAPH: &str = "<urn:dsp:project:0803>";

fn line(s: &str, p: &str, o: &str) -> String {
    format!("<{s}> <{p}> {o} {GRAPH} .")
}

fn plain(v: &str) -> Literal {
    Literal::new_simple_literal(v)
}

fn typed(v: &str, datatype: &str) -> Literal {
    Literal::new_typed_literal(v, nn(datatype))
}

fn node(uuid: &str) -> String {
    format!("{}{uuid}", vocab::VALUE_PREFIX)
}

/// Attach a live value of `class` to `resource` under `property`; returns its IRI.
fn value(f: &mut Fixture, resource: &str, property: &str, id: &str, class: &str, uuid: &str) -> String {
    let iri = format!("{resource}/values/{id}");
    f.add(resource, property, nn(&iri));
    f.add(&iri, model::RDF_TYPE, nn(class));
    f.add(&iri, kb::VALUE_HAS_UUID, plain(uuid));
    f.add(&iri, kb::IS_DELETED, boolean("false"));
    iri
}

/// The edge and the §3 node lines of one value without an order.
fn value_lines(resource: &str, property: &str, uuid: &str, rdf_value: &str) -> Vec<String> {
    let n = node(uuid);
    vec![
        line(resource, property, &format!("<{n}>")),
        line(&n, model::RDF_TYPE, &format!("<{}>", vocab::DAO_VALUE)),
        line(&n, vocab::DAO_VALUE_HAS_UUID, &plain(uuid).to_string()),
        line(&n, vocab::DAO_SOURCE_PROPERTY, &format!("<{property}>")),
        line(&n, vocab::RDF_VALUE, rdf_value),
    ]
}

fn file(parts: Vec<Vec<String>>) -> String {
    let mut lines: Vec<String> = parts.into_iter().flatten().collect();
    lines.sort();
    lines.iter().map(|l| format!("{l}\n")).collect()
}

fn page() -> Fixture {
    let mut f = Fixture::new();
    f.resource(ZEBRA, "Page", "b7v");
    f
}

#[test]
fn a_text_value_emits_its_edge_and_node_with_every_u001e_removed() {
    let mut f = page();
    let p = onto("hasDescription");
    let v = value(&mut f, ZEBRA, &p, "T", kb::TEXT_VALUE, "UUID-T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("Anfang\u{1E}Ende \u{1E}\n  "));

    let expected = file(vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(ZEBRA, &p, "UUID-T", &plain("AnfangEnde \n  ").to_string()),
    ]);
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn a_text_value_with_a_language_emits_a_tagged_rdf_value() {
    let mut f = page();
    let p = onto("hasDescription");
    let v = value(&mut f, ZEBRA, &p, "T", kb::TEXT_VALUE, "UUID-T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("Texte"));
    f.add(&v, kb::VALUE_HAS_LANGUAGE, plain("fr"));

    let tagged = Literal::new_language_tagged_literal("Texte", "fr").unwrap();
    let rdf_value = line(&node("UUID-T"), vocab::RDF_VALUE, &tagged.to_string());
    let out = output(&f.dump());
    assert!(out.lines().any(|l| l == rdf_value), "{out}");
}

const MAPPING: &str = "http://rdfh.ch/standoff/mappings/StandardMapping";

/// Give the text value `v` standoff as DSP-API stores it: a mapping, a start
/// index and one root tag with a predicate the format does not know.
fn standoff(f: &mut Fixture, v: &str) {
    let tag = format!("{v}/standoff/1");
    f.add(v, kb::VALUE_HAS_STANDOFF, nn(&tag));
    f.add(v, kb::VALUE_HAS_MAPPING, nn(MAPPING));
    f.add(v, kb::VALUE_HAS_MAX_STANDOFF_START_INDEX, typed("0", model::XSD_INTEGER));
    f.add(
        &tag,
        model::RDF_TYPE,
        nn("http://www.knora.org/ontology/standoff#StandoffRootTag"),
    );
    f.add(&tag, "http://example.org/unknown#standoffPredicate", plain("never read"));
}

#[test]
fn a_formatted_text_value_carries_its_xml_without_u001e_beside_its_plain_string() {
    let mut f = page();
    let p = onto("hasComment");
    let v = value(&mut f, ZEBRA, &p, "T", kb::TEXT_VALUE, "UUID-T");
    standoff(&mut f, &v);
    f.add(&v, kb::VALUE_HAS_STRING, plain("plain\u{1E} text"));
    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<text><p>plain\u{1E} <strong>text</strong></p></text>";
    f.add(&v, kb::VALUE_HAS_XML, plain(xml));

    let carried = plain(&xml.replace('\u{1E}', "")).to_string();
    let expected = file(vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(ZEBRA, &p, "UUID-T", &plain("plain text").to_string()),
        vec![line(&node("UUID-T"), vocab::DAO_VALUE_HAS_XML, &carried)],
    ]);
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn a_text_value_with_standoff_or_a_mapping_but_no_xml_is_missing_text_xml() {
    let mut f = page();
    let p = onto("hasComment");
    let with_standoff = value(&mut f, ZEBRA, &p, "S", kb::TEXT_VALUE, "S");
    f.add(&with_standoff, kb::VALUE_HAS_STRING, plain("x"));
    f.add(
        &with_standoff,
        kb::VALUE_HAS_STANDOFF,
        nn(&format!("{with_standoff}/standoff/1")),
    );
    let with_mapping = value(&mut f, ZEBRA, &p, "M", kb::TEXT_VALUE, "M");
    f.add(&with_mapping, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&with_mapping, kb::VALUE_HAS_MAPPING, nn(MAPPING));
    // A superseded version is dropped before it is checked: only selected values need the XML.
    let old = format!("{ZEBRA}/values/OLD");
    f.add(&old, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&old, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&old, kb::VALUE_HAS_MAPPING, nn(MAPPING));
    f.add(&with_mapping, kb::PREVIOUS_VALUE, nn(&old));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::MissingTextXml, with_mapping, kb::VALUE_HAS_XML.to_string()),
            (Rule::MissingTextXml, with_standoff, kb::VALUE_HAS_XML.to_string()),
        ]
    );
}

#[test]
fn an_xml_that_is_not_one_plain_string_is_a_violation() {
    let mut f = page();
    let p = onto("hasComment");
    let tagged = value(&mut f, ZEBRA, &p, "L", kb::TEXT_VALUE, "L");
    f.add(&tagged, kb::VALUE_HAS_STRING, plain("x"));
    f.add(
        &tagged,
        kb::VALUE_HAS_XML,
        Literal::new_language_tagged_literal("<text>x</text>", "de").unwrap(),
    );
    let twice = value(&mut f, ZEBRA, &p, "R", kb::TEXT_VALUE, "R");
    f.add(&twice, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&twice, kb::VALUE_HAS_XML, plain("<text>x</text>"));
    f.add(&twice, kb::VALUE_HAS_XML, plain("<text>y</text>"));
    let iri = value(&mut f, ZEBRA, &p, "I", kb::TEXT_VALUE, "I");
    f.add(&iri, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&iri, kb::VALUE_HAS_XML, nn("http://example.org/x.xml"));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::RepeatedSourceLiteral, twice, kb::VALUE_HAS_XML.to_string()),
            (Rule::InvalidLiteral, iri, kb::VALUE_HAS_XML.to_string()),
            (Rule::InvalidLiteral, tagged, kb::VALUE_HAS_XML.to_string()),
        ]
    );
}

const CUSTOM_MAPPING: &str = "http://rdfh.ch/projects/0001/mappings/CustomMapping";

/// A page with one text value of each shape DSP-API writes: plain text, text in
/// the standard mapping and text in a custom mapping, each with its XML. With
/// `typed`, each carries the `kb:hasTextValueType` DSP-API writes on it.
fn text_values(typed: bool) -> Fixture {
    let mut f = page();
    let p = onto("hasComment");
    let shapes = [
        ("U", kb::UNFORMATTED_TEXT, None),
        ("F", kb::FORMATTED_TEXT, Some(MAPPING)),
        ("C", kb::CUSTOM_FORMATTED_TEXT, Some(CUSTOM_MAPPING)),
    ];
    for (id, text_type, mapping) in shapes {
        let v = value(&mut f, ZEBRA, &p, id, kb::TEXT_VALUE, id);
        f.add(&v, kb::VALUE_HAS_STRING, plain("plain text"));
        if let Some(mapping) = mapping {
            f.add(&v, kb::VALUE_HAS_STANDOFF, nn(&format!("{v}/standoff/1")));
            f.add(&v, kb::VALUE_HAS_MAPPING, nn(mapping));
            f.add(&v, kb::VALUE_HAS_XML, plain("<text><p>plain text</p></text>"));
        }
        if typed {
            f.add(&v, kb::HAS_TEXT_VALUE_TYPE, nn(text_type));
        }
    }
    f
}

#[test]
fn a_text_value_type_is_read_and_not_emitted() {
    assert_eq!(output(&text_values(true).dump()), output(&text_values(false).dump()));
}

#[test]
fn an_undefined_text_type_leaves_the_text_as_its_other_triples_make_it() {
    let mut plain_text = page();
    let mut typed = page();
    let p = onto("hasComment");
    for (f, text_type) in [(&mut plain_text, None), (&mut typed, Some(kb::UNDEFINED_TEXT_TYPE))] {
        let bare = value(f, ZEBRA, &p, "P", kb::TEXT_VALUE, "P");
        f.add(&bare, kb::VALUE_HAS_STRING, plain("plain text"));
        let marked = value(f, ZEBRA, &p, "X", kb::TEXT_VALUE, "X");
        f.add(&marked, kb::VALUE_HAS_STRING, plain("plain text"));
        standoff(f, &marked);
        f.add(&marked, kb::VALUE_HAS_XML, plain("<text><p>plain text</p></text>"));
        if let Some(text_type) = text_type {
            f.add(&bare, kb::HAS_TEXT_VALUE_TYPE, nn(text_type));
            f.add(&marked, kb::HAS_TEXT_VALUE_TYPE, nn(text_type));
        }
    }
    assert_eq!(output(&typed.dump()), output(&plain_text.dump()));
}

#[test]
fn a_text_value_type_that_is_not_one_known_iri_is_a_violation() {
    let mut f = page();
    let p = onto("hasComment");
    let unknown = value(&mut f, ZEBRA, &p, "U", kb::TEXT_VALUE, "U");
    f.add(&unknown, kb::VALUE_HAS_STRING, plain("x"));
    let other_type = format!("{}RichText", model::KB);
    f.add(&unknown, kb::HAS_TEXT_VALUE_TYPE, nn(&other_type));
    let literal = value(&mut f, ZEBRA, &p, "L", kb::TEXT_VALUE, "L");
    f.add(&literal, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&literal, kb::HAS_TEXT_VALUE_TYPE, plain("UnformattedText"));
    let twice = value(&mut f, ZEBRA, &p, "R", kb::TEXT_VALUE, "R");
    f.add(&twice, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&twice, kb::HAS_TEXT_VALUE_TYPE, nn(kb::UNFORMATTED_TEXT));
    f.add(&twice, kb::HAS_TEXT_VALUE_TYPE, nn(kb::UNDEFINED_TEXT_TYPE));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::RepeatedSourceLiteral, twice, kb::HAS_TEXT_VALUE_TYPE.to_string()),
            (Rule::InvalidLiteral, literal, kb::HAS_TEXT_VALUE_TYPE.to_string()),
            (Rule::UnknownTextType, unknown, other_type),
        ]
    );
}

#[test]
fn a_formatted_text_type_without_xml_is_missing_text_xml_even_without_standoff() {
    let mut f = page();
    let p = onto("hasComment");
    let formatted = value(&mut f, ZEBRA, &p, "F", kb::TEXT_VALUE, "F");
    f.add(&formatted, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&formatted, kb::HAS_TEXT_VALUE_TYPE, nn(kb::FORMATTED_TEXT));
    let custom = value(&mut f, ZEBRA, &p, "C", kb::TEXT_VALUE, "C");
    f.add(&custom, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&custom, kb::HAS_TEXT_VALUE_TYPE, nn(kb::CUSTOM_FORMATTED_TEXT));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::MissingTextXml, custom, kb::VALUE_HAS_XML.to_string()),
            (Rule::MissingTextXml, formatted, kb::VALUE_HAS_XML.to_string()),
        ]
    );
}

#[test]
fn an_unformatted_text_type_with_xml_or_standoff_is_a_text_type_mismatch() {
    let mut f = page();
    let p = onto("hasComment");
    let with_xml = value(&mut f, ZEBRA, &p, "X", kb::TEXT_VALUE, "X");
    f.add(&with_xml, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&with_xml, kb::VALUE_HAS_XML, plain("<text>x</text>"));
    f.add(&with_xml, kb::HAS_TEXT_VALUE_TYPE, nn(kb::UNFORMATTED_TEXT));
    let with_standoff = value(&mut f, ZEBRA, &p, "S", kb::TEXT_VALUE, "S");
    f.add(&with_standoff, kb::VALUE_HAS_STRING, plain("x"));
    standoff(&mut f, &with_standoff);
    f.add(&with_standoff, kb::HAS_TEXT_VALUE_TYPE, nn(kb::UNFORMATTED_TEXT));

    assert_eq!(
        report(&f.dump()),
        vec![
            (Rule::TextTypeMismatch, with_standoff, kb::HAS_TEXT_VALUE_TYPE.to_string()),
            (Rule::TextTypeMismatch, with_xml, kb::HAS_TEXT_VALUE_TYPE.to_string()),
        ]
    );
}

#[test]
fn integer_decimal_boolean_and_uri_values_emit_their_typed_lexicals_verbatim() {
    let mut f = page();
    let kinds = [
        (
            "hasNumber",
            "I",
            kb::INT_VALUE,
            kb::VALUE_HAS_INTEGER,
            typed("42", model::XSD_INTEGER),
        ),
        (
            "hasPrice",
            "D",
            kb::DECIMAL_VALUE,
            kb::VALUE_HAS_DECIMAL,
            typed("1.25", model::XSD_DECIMAL),
        ),
        (
            "hasFlag",
            "B",
            kb::BOOLEAN_VALUE,
            kb::VALUE_HAS_BOOLEAN,
            typed("true", model::XSD_BOOLEAN),
        ),
        (
            "hasUri",
            "U",
            kb::URI_VALUE,
            kb::VALUE_HAS_URI,
            typed("https://www.e-rara.ch/x", model::XSD_ANYURI),
        ),
    ];
    let mut expected = vec![resource_lines(ZEBRA, &onto("Page"), "b7v")];
    for (local, id, class, source, literal) in kinds {
        let p = onto(local);
        let v = value(&mut f, ZEBRA, &p, id, class, id);
        f.add(&v, source, literal.clone());
        f.add(&v, kb::VALUE_HAS_STRING, plain(literal.value()));
        expected.push(value_lines(ZEBRA, &p, id, &literal.to_string()));
    }
    assert_eq!(output(&f.dump()), file(expected));
}

#[test]
fn non_canonical_numeric_lexicals_are_emitted_in_xsd_canonical_form() {
    let mut f = page();
    let kinds = [
        ("I1", kb::INT_VALUE, kb::VALUE_HAS_INTEGER, model::XSD_INTEGER, "+5", "5"),
        ("I2", kb::INT_VALUE, kb::VALUE_HAS_INTEGER, model::XSD_INTEGER, "01", "1"),
        ("D", kb::DECIMAL_VALUE, kb::VALUE_HAS_DECIMAL, model::XSD_DECIMAL, "1.50", "1.5"),
        ("B1", kb::BOOLEAN_VALUE, kb::VALUE_HAS_BOOLEAN, model::XSD_BOOLEAN, "1", "true"),
        ("B0", kb::BOOLEAN_VALUE, kb::VALUE_HAS_BOOLEAN, model::XSD_BOOLEAN, "0", "false"),
    ];
    let mut expected = vec![resource_lines(ZEBRA, &onto("Page"), "b7v")];
    for (id, class, source, datatype, input, canonical) in kinds {
        let p = onto(&format!("has{id}"));
        let v = value(&mut f, ZEBRA, &p, id, class, id);
        f.add(&v, source, typed(input, datatype));
        expected.push(value_lines(ZEBRA, &p, id, &typed(canonical, datatype).to_string()));
    }
    // An order is an `xsd:integer` too (§5).
    let v = format!("{ZEBRA}/values/I1");
    f.add(&v, kb::VALUE_HAS_ORDER, typed("007", model::XSD_INTEGER));
    expected.push(vec![line(
        &node("I1"),
        vocab::DAO_VALUE_HAS_ORDER,
        &typed("7", model::XSD_INTEGER).to_string(),
    )]);
    assert_eq!(output(&f.dump()), file(expected));
}

#[test]
fn a_missing_or_repeated_source_literal_and_a_malformed_language_are_violations() {
    let mut f = page();
    let p = onto("hasDescription");
    let missing = value(&mut f, ZEBRA, &p, "MISSING", kb::TEXT_VALUE, "M");
    let twice = value(&mut f, ZEBRA, &p, "TWICE", kb::TEXT_VALUE, "T");
    f.add(&twice, kb::VALUE_HAS_STRING, plain("one"));
    f.add(&twice, kb::VALUE_HAS_STRING, plain("two"));
    let tagged = value(&mut f, ZEBRA, &p, "TAG", kb::TEXT_VALUE, "L");
    f.add(&tagged, kb::VALUE_HAS_STRING, plain("x"));
    f.add(&tagged, kb::VALUE_HAS_LANGUAGE, plain("de_CH"));
    let int = value(&mut f, ZEBRA, &onto("hasNumber"), "INT", kb::INT_VALUE, "I");
    f.add(&int, kb::VALUE_HAS_INTEGER, typed("forty", model::XSD_INTEGER));

    let mut expected = vec![
        (Rule::MissingSourceLiteral, missing, kb::VALUE_HAS_STRING.to_string()),
        (Rule::RepeatedSourceLiteral, twice, kb::VALUE_HAS_STRING.to_string()),
        (Rule::InvalidLanguageTag, tagged, "de_CH".to_string()),
        (Rule::InvalidLiteral, int, kb::VALUE_HAS_INTEGER.to_string()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn interval_time_and_geoname_values_are_omitted_whole_and_counted() {
    let mut f = page();
    for (id, class) in [
        ("IV", kb::INTERVAL_VALUE),
        ("TV", kb::TIME_VALUE),
        ("GV", kb::GEONAME_VALUE),
    ] {
        let v = value(&mut f, ZEBRA, &onto("hasWhen"), id, class, id);
        // Not checked: §11 lists no predicates for kinds omitted whole.
        f.add(&v, "http://example.org/unlisted", plain("x"));
    }
    let dump = f.dump();
    let mut stats = Stats::default();
    let snapshot = build_public(&dump, &mut stats).unwrap();

    assert_eq!(snapshot.len(), 5);
    assert_eq!(stats.omitted.get("interval, time or geoname value (§4.10)"), Some(&3));
}

#[test]
fn an_unknown_value_class_is_a_violation_naming_it() {
    let mut f = page();
    let class = "http://www.knora.org/ontology/knora-base#FooValue";
    let v = value(&mut f, ZEBRA, &onto("hasFoo"), "F", class, "F");

    assert_eq!(report(&f.dump()), vec![(Rule::UnknownClass, v, class.to_string())]);
}

#[test]
fn a_uuid_that_is_empty_or_not_base64url_is_a_violation_and_mints_no_node() {
    let mut f = page();
    let p = onto("hasDescription");
    let empty = value(&mut f, ZEBRA, &p, "EMPTY", kb::TEXT_VALUE, "");
    let hash = value(&mut f, ZEBRA, &p, "HASH", kb::TEXT_VALUE, "a#b");
    for v in [&empty, &hash] {
        f.add(v, kb::VALUE_HAS_STRING, plain("x"));
    }
    let dump = f.dump();
    assert_eq!(
        report(&dump),
        vec![
            (Rule::InvalidUuid, empty.clone(), String::new()),
            (Rule::InvalidUuid, hash.clone(), "a#b".to_string()),
        ]
    );

    let mut violations = Violations::default();
    let index = Index::build(&dump, &mut violations).unwrap();
    let mut snapshot = Snapshot::new(nn("urn:dsp:project:0803"));
    for v in [&empty, &hash] {
        let entry = index.get(v).unwrap();
        let property = nn(&p);
        let node = value::emit(
            property.as_ref(),
            entry,
            ValueKind::Text,
            &LiveLists::none(),
            &mut snapshot,
            &mut Stats::default(),
            &mut violations,
        );
        assert!(node.is_none());
    }
    assert_eq!(snapshot.len(), 0);
}

#[test]
fn a_value_predicate_outside_its_class_list_is_a_violation_naming_it() {
    let mut f = page();
    let v = value(&mut f, ZEBRA, &onto("hasDescription"), "T", kb::TEXT_VALUE, "T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("x"));
    // Listed in §11, but for IntValue only.
    f.add(&v, kb::VALUE_HAS_INTEGER, typed("1", model::XSD_INTEGER));
    f.add(&v, "http://purl.org/dc/terms/title", plain("x"));

    assert_eq!(
        report(&f.dump()),
        vec![
            (
                Rule::UnknownValuePredicate,
                v.clone(),
                "http://purl.org/dc/terms/title".to_string()
            ),
            (Rule::UnknownValuePredicate, v, kb::VALUE_HAS_INTEGER.to_string()),
        ]
    );
}

#[test]
fn creation_date_permissions_user_and_comment_on_a_value_emit_nothing() {
    let mut f = page();
    let p = onto("hasDescription");
    let v = value(&mut f, ZEBRA, &p, "T", kb::TEXT_VALUE, "T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("x"));
    let date = "http://www.w3.org/2001/XMLSchema#dateTime";
    f.add(&v, kb::VALUE_CREATION_DATE, typed("2011-04-14T07:15:28Z", date));
    f.add(&v, kb::HAS_PERMISSIONS, plain("V knora-admin:UnknownUser"));
    f.add(&v, kb::ATTACHED_TO_USER, nn("http://rdfh.ch/users/U"));
    f.add(&v, kb::VALUE_HAS_COMMENT, plain("a note on the value"));

    let expected = file(vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(ZEBRA, &p, "T", &plain("x").to_string()),
    ]);
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn value_has_order_is_carried_per_uuid_whatever_the_text_or_uuid_order() {
    let mut f = page();
    let p = onto("hasCitation");
    for (id, uuid, text, order) in [
        ("A", "UB", "alpha", "2"),
        ("B", "UC", "zulu", "0"),
        ("C", "UA", "mike", "1"),
    ] {
        let v = value(&mut f, ZEBRA, &p, id, kb::TEXT_VALUE, uuid);
        f.add(&v, kb::VALUE_HAS_STRING, plain(text));
        f.add(&v, kb::VALUE_HAS_ORDER, typed(order, model::XSD_INTEGER));
    }
    let out = output(&f.dump());

    let orders: Vec<&str> = out.lines().filter(|l| l.contains(vocab::DAO_VALUE_HAS_ORDER)).collect();
    let order = |uuid, n| {
        line(
            &node(uuid),
            vocab::DAO_VALUE_HAS_ORDER,
            &typed(n, model::XSD_INTEGER).to_string(),
        )
    };
    assert_eq!(orders, vec![order("UA", "1"), order("UB", "2"), order("UC", "0")]);
}

#[test]
fn a_value_without_value_has_order_emits_no_order() {
    let mut f = page();
    let v = value(&mut f, ZEBRA, &onto("hasDescription"), "T", kb::TEXT_VALUE, "T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("x"));
    let mut g = page();
    let w = value(&mut g, ZEBRA, &onto("hasDescription"), "T", kb::TEXT_VALUE, "T");
    g.add(&w, kb::VALUE_HAS_STRING, plain("x"));
    g.add(&w, kb::VALUE_HAS_ORDER, typed("0", model::XSD_INTEGER));

    assert!(!output(&f.dump()).contains(vocab::DAO_VALUE_HAS_ORDER));
    // The positive canary: the same value with an order 0 carries it.
    assert!(output(&g.dump()).contains(vocab::DAO_VALUE_HAS_ORDER));
}

/// 0803's `incunabula:hasPubdate` of book `70aWaB2kWsuiN6ujYgM0ZQ`, as the dump records it.
fn julian_1492() -> Vec<(&'static str, Literal)> {
    vec![
        (kb::VALUE_HAS_STRING, plain("JULIAN:1492 CE")),
        (kb::VALUE_HAS_CALENDAR, plain("JULIAN")),
        (kb::VALUE_HAS_START_JDN, typed("2266011", model::XSD_INTEGER)),
        (kb::VALUE_HAS_END_JDN, typed("2266376", model::XSD_INTEGER)),
        (kb::VALUE_HAS_START_PRECISION, plain("YEAR")),
        (kb::VALUE_HAS_END_PRECISION, plain("YEAR")),
    ]
}

/// A date value `id` on the page carrying `facts`; returns its IRI.
fn date(f: &mut Fixture, id: &str, facts: &[(&'static str, Literal)]) -> String {
    let v = value(f, ZEBRA, &onto("hasPubdate"), id, kb::DATE_VALUE, id);
    for (p, o) in facts {
        f.add(&v, p, o.clone());
    }
    v
}

/// `facts` with the object of `p` replaced by `o`.
fn with(mut facts: Vec<(&'static str, Literal)>, p: &str, o: Literal) -> Vec<(&'static str, Literal)> {
    facts.iter_mut().filter(|(q, _)| *q == p).for_each(|(_, l)| *l = o.clone());
    facts
}

#[test]
fn a_julian_date_emits_its_lexical_calendar_jdns_and_precisions() {
    let mut f = page();
    date(&mut f, "D", &julian_1492());

    let n = node("D");
    let year = plain("YEAR").to_string();
    let mut expected = vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(
            ZEBRA,
            &onto("hasPubdate"),
            "D",
            &typed("JULIAN:1492 CE", vocab::DAO_DATE).to_string(),
        ),
    ];
    expected.push(vec![
        line(&n, vocab::DAO_DATE_CALENDAR, &plain("JULIAN").to_string()),
        line(&n, vocab::DAO_DATE_START_JDN, &typed("2266011", model::XSD_INTEGER).to_string()),
        line(&n, vocab::DAO_DATE_END_JDN, &typed("2266376", model::XSD_INTEGER).to_string()),
        line(&n, vocab::DAO_DATE_START_PRECISION, &year),
        line(&n, vocab::DAO_DATE_END_PRECISION, &year),
    ]);
    assert_eq!(output(&f.dump()), file(expected));
}

#[test]
fn a_gregorian_date_keeps_distinct_start_and_end_precisions() {
    let mut f = page();
    let facts = with(julian_1492(), kb::VALUE_HAS_CALENDAR, plain("GREGORIAN"));
    let facts = with(facts, kb::VALUE_HAS_START_PRECISION, plain("DAY"));
    let facts = with(facts, kb::VALUE_HAS_END_PRECISION, plain("MONTH"));
    date(&mut f, "D", &facts);

    let out = output(&f.dump());
    let n = node("D");
    for (p, o) in [
        (vocab::DAO_DATE_CALENDAR, "GREGORIAN"),
        (vocab::DAO_DATE_START_PRECISION, "DAY"),
        (vocab::DAO_DATE_END_PRECISION, "MONTH"),
    ] {
        let expected = line(&n, p, &plain(o).to_string());
        assert!(out.lines().any(|l| l == expected), "{expected} missing from\n{out}");
    }
}

#[test]
fn a_date_in_another_calendar_is_a_violation_naming_the_value() {
    let mut f = page();
    let v = date(&mut f, "D", &with(julian_1492(), kb::VALUE_HAS_CALENDAR, plain("ISLAMIC")));

    assert_eq!(report(&f.dump()), vec![(Rule::UnknownCalendar, v, "ISLAMIC".to_string())]);
}

#[test]
fn a_date_missing_any_of_its_six_source_literals_is_a_violation() {
    let mut f = page();
    let mut expected = Vec::new();
    for (i, (missing, _)) in julian_1492().into_iter().enumerate() {
        let facts: Vec<_> = julian_1492().into_iter().filter(|(p, _)| *p != missing).collect();
        let v = date(&mut f, &format!("D{i}"), &facts);
        expected.push((Rule::MissingSourceLiteral, v, missing.to_string()));
    }
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn an_unknown_precision_a_non_integer_jdn_and_an_inverted_date_are_violations() {
    let mut f = page();
    let century = date(&mut f, "P", &with(julian_1492(), kb::VALUE_HAS_END_PRECISION, plain("CENTURY")));
    let fraction = typed("2266011.5", model::XSD_DECIMAL);
    let half_day = date(&mut f, "J", &with(julian_1492(), kb::VALUE_HAS_START_JDN, fraction));
    let later = typed("2266377", model::XSD_INTEGER);
    let inverted = date(&mut f, "I", &with(julian_1492(), kb::VALUE_HAS_START_JDN, later));

    let mut expected = vec![
        (Rule::UnknownPrecision, century, "CENTURY".to_string()),
        (Rule::InvalidLiteral, half_day, kb::VALUE_HAS_START_JDN.to_string()),
        (Rule::InvertedDate, inverted, String::new()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

/// The omission counts of a fixture that builds without violations.
fn omitted(f: &Fixture) -> BTreeMap<&'static str, usize> {
    let mut stats = Stats::default();
    build_public(&f.dump(), &mut stats).unwrap();
    stats.omitted
}

#[test]
fn a_deleted_value_emits_no_edge_and_no_node() {
    let mut f = page();
    let p = onto("hasDescription");
    let v = value(&mut f, ZEBRA, &p, "T", kb::TEXT_VALUE, "UUID-T");
    f.add(&v, kb::VALUE_HAS_STRING, plain("gone"));
    f.add(&v, kb::IS_DELETED, boolean("true"));
    // Unchecked: a dropped value's predicates are not on the closed list's agenda.
    f.add(&v, "http://example.org/unlisted", plain("x"));

    let expected = file(vec![resource_lines(ZEBRA, &onto("Page"), "b7v")]);
    assert_eq!(output(&f.dump()), expected);
    assert_eq!(omitted(&f).get("deleted value"), Some(&1));
}

/// 0803's `-34FYx0jVMGTEi8aItQFxQ` `incunabula:hasPagenum` chain, newest first: only
/// the head carries `kb:valueHasUUID`, and it is the original's, whose IRI ends in it.
/// Returns the head's and the original's IRIs.
fn three_versions(f: &mut Fixture, p: &str) -> (String, String) {
    let uuid = "U_J3GLDmTJuj24vddKrJMA";
    let head = value(f, ZEBRA, p, "8HeOpYC8Tq2BZDFFPHex8A", kb::TEXT_VALUE, uuid);
    let middle = format!("{ZEBRA}/values/aL0mQInVRpq8C9U9aS2tgg");
    let original = format!("{ZEBRA}/values/{uuid}");
    f.add(&head, kb::VALUE_HAS_STRING, plain("a1r; Titelblatt"));
    f.add(&head, kb::VALUE_HAS_ORDER, typed("0", model::XSD_INTEGER));
    f.add(&head, kb::PREVIOUS_VALUE, nn(&middle));
    for (older, text) in [(&middle, "a1r; Titelblatt."), (&original, "a1r; Titelblatt")] {
        f.add(older, model::RDF_TYPE, nn(kb::TEXT_VALUE));
        f.add(older, kb::IS_DELETED, boolean("false"));
        f.add(older, kb::VALUE_HAS_STRING, plain(text));
        f.add(older, kb::VALUE_HAS_ORDER, typed("0", model::XSD_INTEGER));
    }
    f.add(&middle, kb::PREVIOUS_VALUE, nn(&original));
    (head, original)
}

/// The resource and the one current value node of [`three_versions`].
fn current_version(p: &str) -> String {
    let uuid = "U_J3GLDmTJuj24vddKrJMA";
    let order = line(
        &node(uuid),
        vocab::DAO_VALUE_HAS_ORDER,
        &typed("0", model::XSD_INTEGER).to_string(),
    );
    file(vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(ZEBRA, p, uuid, &plain("a1r; Titelblatt").to_string()),
        vec![order],
    ])
}

#[test]
fn a_three_version_chain_emits_one_node_under_the_original_uuid_with_the_current_text() {
    let mut f = page();
    let p = onto("hasPagenum");
    three_versions(&mut f, &p);

    assert_eq!(output(&f.dump()), current_version(&p));
}

#[test]
fn a_superseded_version_still_attached_to_its_resource_is_dropped() {
    let mut f = page();
    let p = onto("hasPagenum");
    let (_, original) = three_versions(&mut f, &p);
    f.add(ZEBRA, &p, nn(&original));

    assert_eq!(output(&f.dump()), current_version(&p));
    assert_eq!(omitted(&f).get("superseded value"), Some(&1));
}

#[test]
fn a_chain_whose_current_head_is_deleted_emits_no_version() {
    let mut f = page();
    let p = onto("hasPagenum");
    let (head, original) = three_versions(&mut f, &p);
    f.add(&head, kb::IS_DELETED, boolean("true"));
    f.add(ZEBRA, &p, nn(&original));

    let expected = file(vec![resource_lines(ZEBRA, &onto("Page"), "b7v")]);
    assert_eq!(output(&f.dump()), expected);
}

#[test]
fn two_live_values_sharing_a_uuid_in_one_resource_or_in_two_are_a_violation() {
    let book = "http://rdfh.ch/0803/BOOK";
    let mut f = page();
    f.resource(book, "Book", "Narrenschiff");
    let p = onto("hasDescription");
    let mut expected = Vec::new();
    for (uuid, holders) in [("SAME", [ZEBRA, ZEBRA]), ("SHARED", [ZEBRA, book])] {
        for (i, resource) in holders.into_iter().enumerate() {
            let v = value(&mut f, resource, &p, &format!("{uuid}{i}"), kb::TEXT_VALUE, uuid);
            f.add(&v, kb::VALUE_HAS_STRING, plain("x"));
            expected.push((Rule::DuplicateUuid, v, node(uuid)));
        }
    }
    // The positive canary: a third, distinct UUID is no violation.
    let v = value(&mut f, ZEBRA, &p, "OWN", kb::TEXT_VALUE, "OWN");
    f.add(&v, kb::VALUE_HAS_STRING, plain("x"));

    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}

#[test]
fn a_live_value_without_a_uuid_is_a_violation() {
    let mut f = page();
    let v = format!("{ZEBRA}/values/T");
    f.add(ZEBRA, &onto("hasDescription"), nn(&v));
    f.add(&v, model::RDF_TYPE, nn(kb::TEXT_VALUE));
    f.add(&v, kb::IS_DELETED, boolean("false"));
    f.add(&v, kb::VALUE_HAS_STRING, plain("x"));

    assert_eq!(
        report(&f.dump()),
        vec![(Rule::MissingSourceLiteral, v, kb::VALUE_HAS_UUID.to_string())]
    );
}

#[test]
fn colour_and_geometry_values_emit_their_lexicals_typed_dao_color_and_dao_geometry() {
    let mut f = page();
    let geometry = r##"{"status":"active","lineColor":"#ff3333","lineWidth":2,"points":[{"x":0.484375,"y":0.6893542757417103}],"type":"rectangle"}"##;
    let color = value(&mut f, ZEBRA, kb::HAS_COLOR, "C", kb::COLOR_VALUE, "C");
    f.add(&color, kb::VALUE_HAS_COLOR, plain("#ff3333"));
    f.add(&color, kb::VALUE_HAS_STRING, plain("#ff3333"));
    let geom = value(&mut f, ZEBRA, kb::HAS_GEOMETRY, "G", kb::GEOM_VALUE, "G");
    f.add(&geom, kb::VALUE_HAS_GEOMETRY, plain(geometry));
    f.add(&geom, kb::VALUE_HAS_STRING, plain(geometry));

    let expected = file(vec![
        resource_lines(ZEBRA, &onto("Page"), "b7v"),
        value_lines(ZEBRA, kb::HAS_COLOR, "C", &typed("#ff3333", vocab::DAO_COLOR).to_string()),
        value_lines(ZEBRA, kb::HAS_GEOMETRY, "G", &typed(geometry, vocab::DAO_GEOMETRY).to_string()),
    ]);
    let out = output(&f.dump());
    assert_eq!(out, expected);
    // Verbatim: the JSON's quotes are escaped on the wire, never rewritten.
    assert!(out.contains(r##"\"lineColor\":\"#ff3333\""##), "{out}");
}

#[test]
fn a_colour_or_geometry_lexical_that_is_not_a_plain_string_is_a_violation() {
    let mut f = page();
    let tagged = value(&mut f, ZEBRA, kb::HAS_COLOR, "C", kb::COLOR_VALUE, "C");
    let red = Literal::new_language_tagged_literal("#ff3333", "en").unwrap();
    f.add(&tagged, kb::VALUE_HAS_COLOR, red);
    let number = value(&mut f, ZEBRA, kb::HAS_GEOMETRY, "G", kb::GEOM_VALUE, "G");
    f.add(&number, kb::VALUE_HAS_GEOMETRY, typed("1", model::XSD_INTEGER));
    // The positive canary: a plain colour is no violation.
    let plain_color = value(&mut f, ZEBRA, kb::HAS_COLOR, "P", kb::COLOR_VALUE, "P");
    f.add(&plain_color, kb::VALUE_HAS_COLOR, plain("#00ff00"));

    let mut expected = vec![
        (Rule::InvalidLiteral, tagged, kb::VALUE_HAS_COLOR.to_string()),
        (Rule::InvalidLiteral, number, kb::VALUE_HAS_GEOMETRY.to_string()),
    ];
    expected.sort();
    assert_eq!(report(&f.dump()), expected);
}
