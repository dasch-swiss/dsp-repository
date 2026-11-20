//! `FORMAT.md` under test: its section numbering, its §4 value-kind table against
//! [`ValueKind::CLASSES`], its §15 index against [`vocab::TERMS`] and the
//! emitter's output, its deviation ids against §12, its violation rule names
//! against [`Rule`], and its §14 worked example against the emitter.
//! `FORMAT.md` is normative and pinned by `dsp-repository`: when one of these
//! fails after an intended output change, fix `FORMAT.md` and add a §16 line.

use std::collections::{BTreeMap, BTreeSet};

use oxigraph::io::RdfFormat;
use oxigraph::model::{NamedOrBlankNode, Term};
use oxigraph::store::Store;

use super::link_tests::link;
use super::tests::{Fixture, boolean, integer, onto, plain, run};
use super::value::ValueKind;
use super::violations::Rule;
use super::vocab::TERMS;
use crate::model::{self, kb, nn};

const FORMAT: &str = include_str!("../../FORMAT.md");

/// The namespaces whose terms `FORMAT.md` fixes; §15 lists every one it uses.
const FORMAT_NAMESPACES: &[&str] = &[
    "https://ontology.dasch.swiss/dao#",
    "http://www.w3.org/ns/oa#",
    "http://www.w3.org/2004/02/skos/core#",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2001/XMLSchema#",
];

/// Written without a datatype IRI in N-Quads, so §15 does not list them.
const IMPLICIT_DATATYPES: &[&str] = &[
    "http://www.w3.org/2001/XMLSchema#string",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
];

/// The top-level `## §<n> …` sections, as number → body (up to the next `## `).
fn top_sections() -> BTreeMap<u32, &'static str> {
    FORMAT
        .split("\n## ")
        .skip(1)
        .filter_map(|chunk| {
            let n = chunk.strip_prefix('§')?.split(' ').next()?.parse().ok()?;
            Some((n, chunk))
        })
        .collect()
}

/// The body of the `## <heading>` section, up to the next `## ` heading.
fn section(heading: &str) -> &'static str {
    FORMAT
        .split(&format!("\n## {heading}\n"))
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .unwrap_or_else(|| panic!("FORMAT.md has a `## {heading}` section"))
}

/// The trimmed cells of every body row of the Markdown tables in `body`,
/// header and separator rows excluded.
fn table_rows(body: &'static str) -> Vec<Vec<&'static str>> {
    let rows: Vec<Vec<&str>> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix('|')?.strip_suffix('|'))
        .map(|l| l.split('|').map(str::trim).collect())
        .collect();
    let separator = |cells: &Vec<&str>| cells.iter().all(|c| c.chars().all(|ch| ch == '-'));
    let mut body_rows = Vec::new();
    for (i, cells) in rows.iter().enumerate() {
        let is_header = rows.get(i + 1).is_some_and(separator);
        if !is_header && !separator(cells) {
            body_rows.push(cells.clone());
        }
    }
    body_rows
}

/// §15's rows as IRI → (section, status), enforcing the §15 parse contract.
fn index() -> BTreeMap<&'static str, (&'static str, &'static str)> {
    let mut map = BTreeMap::new();
    for cells in table_rows(section("§15 Vocabulary index")) {
        assert_eq!(cells.len(), 4, "§15 row has not 4 columns: {cells:?}");
        let iri = cells[0]
            .strip_prefix('`')
            .and_then(|c| c.strip_suffix('`'))
            .filter(|c| !c.contains('`'))
            .unwrap_or_else(|| panic!("§15 column 1 is not one backticked IRI: {cells:?}"));
        assert!(map.insert(iri, (cells[1], cells[2])).is_none(), "§15 lists {iri} twice");
    }
    map
}

/// The ids of §12's deviation table, checked to be `D1`…`Dn` in row order.
fn deviation_ids() -> BTreeSet<&'static str> {
    let ids: Vec<&str> = table_rows(section("§12 Deviations from DAO"))
        .into_iter()
        .map(|cells| cells[0])
        .collect();
    let expected: Vec<String> = (1..=ids.len()).map(|n| format!("D{n}")).collect();
    assert_eq!(ids, expected, "§12 ids are not D1…Dn in row order");
    ids.into_iter().collect()
}

#[test]
fn the_top_level_sections_are_numbered_zero_to_sixteen_in_order() {
    let numbers: Vec<u32> = FORMAT
        .lines()
        .filter_map(|l| l.strip_prefix("## §")?.split(' ').next()?.parse().ok())
        .collect();
    assert_eq!(
        numbers,
        (0..=16).collect::<Vec<_>>(),
        "FORMAT.md sections are cited by number from dsp-repository: never renumber one; \
         a new section is appended (extend this range) with a §16 line"
    );
}

#[test]
fn the_vocabulary_index_equals_terms() {
    let doc = index();
    let code: BTreeMap<_, _> = TERMS.iter().map(|(i, s, st)| (*i, (*s, *st))).collect();
    assert_eq!(TERMS.len(), code.len(), "TERMS lists an IRI twice");
    let only_doc: Vec<_> = doc.keys().filter(|i| !code.contains_key(*i)).collect();
    let only_code: Vec<_> = code.keys().filter(|i| !doc.contains_key(*i)).collect();
    assert!(
        only_doc.is_empty() && only_code.is_empty(),
        "only in FORMAT.md §15: {only_doc:?}; only in TERMS: {only_code:?}"
    );
    assert_eq!(doc, code, "§15 and TERMS disagree on a section or status");
}

/// The §4 row a kind belongs to, as (sub-section, kind cell). Exhaustive, so a
/// new kind cannot compile without naming its row.
fn value_kind_row(kind: ValueKind) -> (&'static str, &'static str) {
    match kind {
        ValueKind::Text => ("4.1", "text"),
        ValueKind::Integer => ("4.2", "integer"),
        ValueKind::Decimal => ("4.3", "decimal"),
        ValueKind::Boolean => ("4.4", "boolean"),
        ValueKind::Uri => ("4.5", "URI"),
        ValueKind::Date => ("4.6", "date"),
        ValueKind::List => ("4.7", "list"),
        ValueKind::Color => ("4.8", "colour"),
        ValueKind::Geometry => ("4.9", "geometry"),
        ValueKind::Interval | ValueKind::Time | ValueKind::Geoname => ("4.10", "not carried"),
    }
}

#[test]
fn the_value_kind_table_equals_the_classes_the_emitter_reads() {
    let mut doc = BTreeMap::new();
    let intro = section("§4 Value kinds").split("\n### ").next().unwrap();
    for cells in table_rows(intro) {
        for class in cells[2].split(", ") {
            let local = class.strip_prefix("`kb:").and_then(|c| c.strip_suffix('`')).unwrap_or_else(|| {
                panic!(
                    "§4 column 3 must be backticked kb: class names joined by \", \", \
                         e.g. `kb:IntervalValue`, `kb:TimeValue`: {cells:?}"
                )
            });
            let iri = format!("{}{local}", model::KB);
            assert!(doc.insert(iri, (cells[0], cells[1])).is_none(), "§4 lists {class} twice");
        }
    }
    let code: BTreeMap<String, _> = ValueKind::CLASSES
        .iter()
        .map(|(class, kind)| (class.to_string(), value_kind_row(*kind)))
        .collect();
    assert_eq!(
        doc, code,
        "FORMAT.md §4's table and ValueKind::CLASSES disagree (FORMAT.md is normative: if the \
         change is intended, update §4 and add a §16 line)"
    );
}

/// `vocab.rs`'s source: its consts are not enumerable from code.
const VOCAB: &str = include_str!("vocab.rs");

/// The identifier `s` starts with, after leading whitespace.
fn leading_ident(s: &str) -> &str {
    let s = s.trim_start();
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    &s[..end]
}

#[test]
fn every_vocab_term_const_is_a_terms_row() {
    // Whitespace tokens, not lines, so neither rustfmt's wrapping nor a const's
    // visibility can hide one; comment lines are dropped first.
    let code = VOCAB
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let words: Vec<&str> = code.split_whitespace().collect();
    // The `*_PREFIX` consts are URN templates, not terms (§15's preamble).
    let consts: BTreeSet<&str> = words
        .windows(2)
        .filter(|w| w[0] == "const")
        .map(|w| leading_ident(w[1]))
        .filter(|name| *name != "TERMS" && !name.ends_with("_PREFIX"))
        .collect();
    let list = code
        .split_once("TERMS")
        .and_then(|(_, decl)| decl.split_once('='))
        .and_then(|(_, value)| value.split_once("];"))
        .map(|(list, _)| list)
        .expect("vocab.rs declares TERMS as a `&[ … ];` slice literal");
    let rows: BTreeSet<&str> = list.split('(').skip(1).map(leading_ident).collect();
    assert!(
        !consts.is_empty() && !rows.is_empty(),
        "vocab.rs consts or TERMS rows not found: {consts:?} {rows:?}"
    );
    assert_eq!(
        consts, rows,
        "every term const in vocab.rs needs a TERMS row and a FORMAT.md §15 row"
    );
}

#[test]
fn every_deviation_status_names_a_row_of_the_deviation_table() {
    let ids = deviation_ids();
    for (iri, (_, status)) in &index() {
        assert!(
            *status == "DAO" || ids.contains(status),
            "§15: {iri} has status {status}, neither DAO nor a §12 id"
        );
    }
}

/// The `§` numbers of the top-level sections a [`Rule::section`] string cites,
/// with `§3–§5` expanded and `§4.6` read as §4.
fn cited_sections(cited: &str) -> Vec<u32> {
    let number = |s: &str| -> u32 {
        let s = s.trim().trim_start_matches('§');
        s.split('.')
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("unreadable section {s:?} in {cited:?}"))
    };
    cited
        .split(", ")
        .flat_map(|part| match part.split_once('–') {
            Some((a, b)) => (number(a)..=number(b)).collect::<Vec<_>>(),
            None => vec![number(part)],
        })
        .collect()
}

#[test]
fn every_violation_rule_is_named_in_a_section_its_report_cites() {
    let last = Rule::ALL[Rule::ALL.len() - 1];
    assert!(last.is_last(), "Rule::ALL does not end in the enum's last variant");
    for (i, rule) in Rule::ALL.iter().enumerate() {
        assert_eq!(*rule as usize, i, "Rule::ALL is not in declaration order at {rule:?}");
    }
    let sections = top_sections();
    let mut unnamed = Vec::new();
    for rule in Rule::ALL {
        let name = format!("`{}`", rule.name());
        let cited = cited_sections(rule.section());
        if !cited.iter().any(|n| sections.get(n).is_some_and(|b| b.contains(&name))) {
            unnamed.push(format!("{name} (cited {})", rule.section()));
        }
    }
    assert!(
        unnamed.is_empty(),
        "rules FORMAT.md does not name where the report points; name each there and add a §16 line: {unnamed:#?}"
    );
}

/// The lines of §14's one `nquads` code block.
fn worked_example() -> Vec<&'static str> {
    let body = section("§14 Worked example");
    let block = body
        .split("```nquads\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("§14 has an nquads block");
    block.lines().collect()
}

const BOOK: &str = "http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ";
const PAGE: &str = "http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg";
const PARENT: &str = "http://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg";
const BAND: &str = "http://rdfh.ch/0803/VNdAPbnOXgqJv1PlcMPj6g";
const REGION: &str = "http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w";

/// A live value `uuid` of `class` on `resource` under `property`, recorded at
/// order 0; returns its IRI.
fn value(f: &mut Fixture, resource: &str, property: &str, class: &str, uuid: &str) -> String {
    let iri = format!("{resource}/values/{uuid}");
    f.add(resource, property, nn(&iri));
    f.add(&iri, model::RDF_TYPE, nn(class));
    f.add(&iri, kb::VALUE_HAS_UUID, plain(uuid));
    f.add(&iri, kb::VALUE_HAS_ORDER, integer("0"));
    f.add(&iri, kb::IS_DELETED, boolean("false"));
    iri
}

/// The facts §14 shows, as the 0803 dump records them.
fn worked_example_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.class("Band");
    for (p, sup) in [
        ("isPartOfBook", kb::IS_PART_OF),
        ("hasSeqnum", kb::SEQNUM),
        ("hasBandTypeL", kb::HAS_LINK_TO),
    ] {
        f.onto_triple(&onto(p), model::RDFS_SUBPROPERTYOF, nn(sup));
    }
    f.resource(BOOK, "Book", "Book");
    f.resource(PARENT, "Book", "Parent book");
    f.resource(BAND, "Band", "l_1a");

    let date = value(&mut f, BOOK, &onto("hasPubdate"), kb::DATE_VALUE, "EDo-E58nRcql5glzoMvnYw");
    for (p, o) in [
        (kb::VALUE_HAS_STRING, plain("JULIAN:1492 CE")),
        (kb::VALUE_HAS_CALENDAR, plain("JULIAN")),
        (kb::VALUE_HAS_START_JDN, integer("2266011")),
        (kb::VALUE_HAS_END_JDN, integer("2266376")),
        (kb::VALUE_HAS_START_PRECISION, plain("YEAR")),
        (kb::VALUE_HAS_END_PRECISION, plain("YEAR")),
    ] {
        f.add(&date, p, o);
    }

    f.resource(PAGE, "Page", "b7v");
    link(&mut f, PAGE, &onto("isPartOfBook"), PARENT, "PARTLV");
    link(&mut f, PAGE, &onto("hasBandTypeL"), BAND, "BANDLV");
    let seqnum = value(&mut f, PAGE, &onto("hasSeqnum"), kb::INT_VALUE, "SEQNUM");
    f.add(&seqnum, kb::VALUE_HAS_INTEGER, integer("32"));
    let file = format!("{PAGE}/values/FILE");
    f.add(PAGE, kb::HAS_STILL_IMAGE_FILE_VALUE, nn(&file));
    f.add(&file, model::RDF_TYPE, nn(kb::STILL_IMAGE_FILE_VALUE));
    f.add(&file, kb::VALUE_HAS_UUID, plain("sdSIAoxiS4-7ONN-oeHxjA"));
    f.add(&file, kb::INTERNAL_FILENAME, plain("8F1KTUpKKra-CaFgCYlt8im.jp2"));
    f.add(&file, kb::DIM_X, integer("3297"));
    f.add(&file, kb::DIM_Y, integer("5141"));
    f.add(&file, kb::IS_DELETED, boolean("false"));

    f.add(REGION, model::RDF_TYPE, nn(kb::REGION));
    f.add(REGION, model::RDFS_LABEL, plain("Seitenbezeichnung : b7v"));
    f.add(REGION, kb::IS_DELETED, boolean("false"));
    link(&mut f, REGION, kb::IS_REGION_OF, PAGE, "REGIONLV");
    let color = value(&mut f, REGION, kb::HAS_COLOR, kb::COLOR_VALUE, "MB-Cj5wySn-XGaFWAA1d-w");
    f.add(&color, kb::VALUE_HAS_COLOR, plain("#ff3333"));
    let comment = value(&mut f, REGION, kb::HAS_COMMENT, kb::TEXT_VALUE, "b_NykVy4T3uPwcLFN8PmSg");
    let text = "Derselbe Holzschnitt wird auf Seite c6r der lateinischen Ausgabe des \
                Narrenschiffs verwendet. ";
    f.add(&comment, kb::VALUE_HAS_STRING, plain(text));
    f
}

#[test]
fn the_worked_example_is_sorted_n_quads_the_emitter_writes_for_its_facts() {
    let lines = worked_example();
    for subject in [BOOK, PAGE, REGION, "urn:dsp:representation:", "urn:dsp:value:"] {
        assert!(
            lines.iter().any(|l| l.starts_with(&format!("<{subject}"))),
            "§14 has no line of {subject}"
        );
    }
    let store = Store::new().unwrap();
    store
        .load_from_slice(RdfFormat::NQuads, lines.join("\n").as_bytes())
        .expect("§14 parses as N-Quads");
    for pair in lines.windows(2) {
        assert!(pair[0] < pair[1], "§14 is not strictly ascending at\n{}\n{}", pair[0], pair[1]);
    }

    let (out, _) = run(&worked_example_fixture());
    let emitted: BTreeSet<&str> = out.lines().collect();
    let missing: Vec<_> = lines.iter().filter(|l| !emitted.contains(*l)).collect();
    assert!(
        missing.is_empty(),
        "§14 lines the emitter does not write (FORMAT.md is normative: if the output change is \
         intended, update §14 and add a §16 line): {missing:#?}\n{out}"
    );
    // §7: membership and seqnum appear neither as a link nor as a value node.
    for absent in ["incunabula#isPartOfBook", "incunabula#hasSeqnum", "SEQNUM", "LV>"] {
        assert!(!out.contains(absent), "output contains {absent}:\n{out}");
    }
}

#[test]
fn every_format_term_the_worked_example_emits_is_in_terms() {
    let (out, _) = run(&worked_example_fixture());
    let store = Store::new().unwrap();
    store.load_from_slice(RdfFormat::NQuads, out.as_bytes()).unwrap();
    let mut used = BTreeSet::new();
    for q in store.iter() {
        let q = q.unwrap();
        assert!(matches!(q.subject, NamedOrBlankNode::NamedNode(_)));
        used.insert(q.predicate.as_str().to_string());
        match q.object {
            Term::NamedNode(n) => used.insert(n.as_str().to_string()),
            Term::Literal(l) => used.insert(l.datatype().as_str().to_string()),
            other => panic!("blank node object {other}"),
        };
    }
    let terms: BTreeSet<&str> = TERMS.iter().map(|(iri, _, _)| *iri).collect();
    let unlisted: Vec<_> = used
        .iter()
        .filter(|iri| FORMAT_NAMESPACES.iter().any(|ns| iri.starts_with(ns)))
        .filter(|iri| !IMPLICIT_DATATYPES.contains(&iri.as_str()))
        .filter(|iri| !terms.contains(iri.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "emitted terms missing from vocab::TERMS and FORMAT.md §15: {unlisted:?}"
    );
}
