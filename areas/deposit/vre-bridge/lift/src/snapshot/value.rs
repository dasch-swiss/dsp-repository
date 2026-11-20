//! Value nodes (§3) and their kinds (§4).

use std::str::FromStr;

use oxigraph::model::{Literal, NamedNode, NamedNodeRef, Term};

use super::index::Entry;
use super::list::LiveLists;
use super::violations::{Rule, Violations};
use super::writer::Snapshot;
use super::{Stats, kinds, mint, vocab};
use crate::model::{self, kb, nn};

/// The kind of a value, decided by its dump value class (§4's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueKind {
    Text,
    Integer,
    Decimal,
    Boolean,
    Uri,
    Date,
    List,
    Color,
    Geometry,
    Interval,
    Time,
    Geoname,
}

impl ValueKind {
    /// §4's table: every dump value class and its kind. `format_doc_tests` holds
    /// it equal to the table in `FORMAT.md`.
    pub(crate) const CLASSES: &[(&str, ValueKind)] = &[
        (kb::TEXT_VALUE, ValueKind::Text),
        (kb::INT_VALUE, ValueKind::Integer),
        (kb::DECIMAL_VALUE, ValueKind::Decimal),
        (kb::BOOLEAN_VALUE, ValueKind::Boolean),
        (kb::URI_VALUE, ValueKind::Uri),
        (kb::DATE_VALUE, ValueKind::Date),
        (kb::LIST_VALUE, ValueKind::List),
        (kb::COLOR_VALUE, ValueKind::Color),
        (kb::GEOM_VALUE, ValueKind::Geometry),
        (kb::INTERVAL_VALUE, ValueKind::Interval),
        (kb::TIME_VALUE, ValueKind::Time),
        (kb::GEONAME_VALUE, ValueKind::Geoname),
    ];

    /// The kind of a dump value class, or `None` when it is not one of §4's.
    pub(crate) fn of_class(class: &str) -> Option<ValueKind> {
        Self::CLASSES.iter().find(|(c, _)| *c == class).map(|(_, kind)| *kind)
    }
}

/// §11: the predicates every value class may carry, besides its kind's own.
const COMMON_PREDICATES: &[&str] = &[
    model::RDF_TYPE,
    kb::VALUE_HAS_UUID,
    kb::VALUE_HAS_ORDER,
    kb::IS_DELETED,
    kb::PREVIOUS_VALUE,
    kb::VALUE_HAS_COMMENT,
    kb::VALUE_CREATION_DATE,
    kb::ATTACHED_TO_USER,
    kb::HAS_PERMISSIONS,
];

const TEXT_PREDICATES: &[&str] = &[
    kb::VALUE_HAS_STRING,
    kb::VALUE_HAS_LANGUAGE,
    kb::VALUE_HAS_MAPPING,
    kb::VALUE_HAS_MAX_STANDOFF_START_INDEX,
    kb::VALUE_HAS_STANDOFF,
    kb::VALUE_HAS_XML,
];

const INTEGER_PREDICATES: &[&str] = &[kb::VALUE_HAS_INTEGER, kb::VALUE_HAS_STRING];

const DATE_PREDICATES: &[&str] = &[
    kb::VALUE_HAS_STRING,
    kb::VALUE_HAS_CALENDAR,
    kb::VALUE_HAS_START_JDN,
    kb::VALUE_HAS_END_JDN,
    kb::VALUE_HAS_START_PRECISION,
    kb::VALUE_HAS_END_PRECISION,
];

/// §4.6: the calendars and precisions a date may name.
const CALENDARS: &[&str] = &["GREGORIAN", "JULIAN"];
const PRECISIONS: &[&str] = &["YEAR", "MONTH", "DAY"];

/// A value node's kind-specific triples: its one `rdf:value` (none for a list
/// value, §4.7), and the kind's extra triples.
struct Content {
    rdf_value: Option<Literal>,
    extra: Vec<(&'static str, Term)>,
}

impl From<Literal> for Content {
    fn from(rdf_value: Literal) -> Content {
        Content { rdf_value: Some(rdf_value), extra: Vec::new() }
    }
}

/// Emit the value's node (§3) and return it, or record every reason it cannot be
/// emitted. The caller writes the edge `<resource> <property> <node>`. `lists`
/// holds the live list nodes, the only ones a list value may name (§4.7).
#[must_use = "the caller writes the edge from the resource to the returned node"]
pub(crate) fn emit(
    property: NamedNodeRef<'_>,
    value: &Entry,
    kind: ValueKind,
    lists: &LiveLists,
    snapshot: &mut Snapshot,
    stats: &mut Stats,
    violations: &mut Violations,
) -> Option<NamedNode> {
    let (content, own_predicates): (Option<Content>, &[&str]) = match kind {
        ValueKind::Text => (text(value, violations), TEXT_PREDICATES),
        ValueKind::Integer => (
            typed(value, kb::VALUE_HAS_INTEGER, vocab::XSD_INTEGER, is_integer, violations).map(Content::from),
            INTEGER_PREDICATES,
        ),
        ValueKind::Decimal => (
            typed(value, kb::VALUE_HAS_DECIMAL, vocab::XSD_DECIMAL, is_decimal, violations).map(Content::from),
            &[kb::VALUE_HAS_DECIMAL, kb::VALUE_HAS_STRING],
        ),
        ValueKind::Boolean => (
            typed(value, kb::VALUE_HAS_BOOLEAN, vocab::XSD_BOOLEAN, is_boolean, violations).map(Content::from),
            &[kb::VALUE_HAS_BOOLEAN, kb::VALUE_HAS_STRING],
        ),
        ValueKind::Uri => (
            retyped(value, kb::VALUE_HAS_URI, model::XSD_ANYURI, vocab::XSD_ANYURI, violations),
            &[kb::VALUE_HAS_URI, kb::VALUE_HAS_STRING],
        ),
        ValueKind::Date => (date(value, violations), DATE_PREDICATES),
        ValueKind::Color => (
            retyped(value, kb::VALUE_HAS_COLOR, model::XSD_STRING, vocab::DAO_COLOR, violations),
            &[kb::VALUE_HAS_COLOR, kb::VALUE_HAS_STRING],
        ),
        ValueKind::Geometry => (
            retyped(
                value,
                kb::VALUE_HAS_GEOMETRY,
                model::XSD_STRING,
                vocab::DAO_GEOMETRY,
                violations,
            ),
            &[kb::VALUE_HAS_GEOMETRY, kb::VALUE_HAS_STRING],
        ),
        ValueKind::List => (
            list_node(value, lists, violations),
            &[kb::VALUE_HAS_LIST_NODE, kb::VALUE_HAS_STRING],
        ),
        ValueKind::Interval | ValueKind::Time | ValueKind::Geoname => {
            *stats.omitted.entry("interval, time or geoname value (§4.10)").or_default() += 1;
            return None;
        }
    };

    let s = value.node.as_str();
    check_predicates(value, own_predicates, violations);
    let uuid = one_literal(value, kb::VALUE_HAS_UUID, violations);
    let node = uuid.and_then(|u| mint::minted_node(vocab::VALUE_PREFIX, u.value(), s, violations));
    let order = order(value, violations);
    let (Some(uuid), Some(node), Some(content), Ok(order)) = (uuid, node, content, order) else {
        return None;
    };

    let n = node.as_ref();
    snapshot.put(n, nn(vocab::RDF_TYPE).as_ref(), nn(vocab::DAO_VALUE).as_ref());
    snapshot.put(
        n,
        nn(vocab::DAO_VALUE_HAS_UUID).as_ref(),
        &Literal::new_simple_literal(uuid.value()),
    );
    snapshot.put(n, nn(vocab::DAO_SOURCE_PROPERTY).as_ref(), property);
    if let Some(rdf_value) = &content.rdf_value {
        snapshot.put(n, nn(vocab::RDF_VALUE).as_ref(), rdf_value);
    }
    for (p, o) in &content.extra {
        snapshot.put(n, nn(p).as_ref(), o);
    }
    if let Some(order) = order {
        snapshot.put(n, nn(vocab::DAO_VALUE_HAS_ORDER).as_ref(), &order);
    }
    *stats.value_nodes_by_kind.entry(kind).or_default() += 1;
    Some(node)
}

/// §7: the `xsd:integer` of a seqnum, an `IntValue` checked against §11's list
/// like any other; it mints no node, so its UUID is not read.
pub(crate) fn seqnum(value: &Entry, violations: &mut Violations) -> Option<Literal> {
    check_predicates(value, INTEGER_PREDICATES, violations);
    typed(value, kb::VALUE_HAS_INTEGER, vocab::XSD_INTEGER, is_integer, violations)
}

/// §11: every predicate of the value is common to all classes or its kind's own.
fn check_predicates(value: &Entry, own: &[&str], violations: &mut Violations) {
    for (p, _) in &value.triples {
        let p = p.as_str();
        if !COMMON_PREDICATES.contains(&p) && !own.contains(&p) {
            violations.record(Rule::UnknownValuePredicate, value.node.as_str(), p);
        }
    }
}

/// §4.1: `kb:valueHasString` without its U+001E, tagged when the dump records a
/// language, and `dao:valueHasXml` when the dump records `kb:valueHasXml`.
fn text(value: &Entry, violations: &mut Violations) -> Option<Content> {
    let text = one_literal(value, kb::VALUE_HAS_STRING, violations);
    let language = at_most_one_literal(value, kb::VALUE_HAS_LANGUAGE, violations);
    let xml = xml(value, violations);
    let text = without_u001e(text?);
    let rdf_value = match language.ok()? {
        None => Literal::new_simple_literal(text),
        Some(tag) => mint::lang_literal(&text, tag.value(), value.node.as_str(), violations)?,
    };
    let extra = match xml.ok()? {
        Some(xml) => vec![(vocab::DAO_VALUE_HAS_XML, Literal::new_simple_literal(without_u001e(xml)).into())],
        None => Vec::new(),
    };
    Some(Content { rdf_value: Some(rdf_value), extra })
}

/// §4.1, d.51: a text's lexical with every U+001E removed.
fn without_u001e(literal: &Literal) -> String {
    literal.value().replace('\u{1E}', "")
}

/// §4.1: the text value's one plain-string `kb:valueHasXml`, if any. Standoff is
/// never read; that the value has some, or a mapping, only makes the XML required.
fn xml<'a>(value: &'a Entry, violations: &mut Violations) -> Result<Option<&'a Literal>, ()> {
    let s = value.node.as_str();
    match at_most_one_literal(value, kb::VALUE_HAS_XML, violations)? {
        Some(xml) if is_plain_string(xml) => Ok(Some(xml)),
        Some(_) => {
            violations.record(Rule::InvalidLiteral, s, kb::VALUE_HAS_XML);
            Err(())
        }
        None => {
            let markup = [kb::VALUE_HAS_STANDOFF, kb::VALUE_HAS_MAPPING];
            if value.triples.iter().any(|(p, _)| markup.contains(&p.as_str())) {
                violations.record(Rule::MissingTextXml, s, kb::VALUE_HAS_XML);
                return Err(());
            }
            Ok(None)
        }
    }
}

/// §4.6: the DSP lexical typed `dao:date`, with its calendar, JDNs and precisions.
fn date(value: &Entry, violations: &mut Violations) -> Option<Content> {
    let lexical = one_literal(value, kb::VALUE_HAS_STRING, violations);
    let calendar = word(value, kb::VALUE_HAS_CALENDAR, CALENDARS, Rule::UnknownCalendar, violations);
    let start_precision = word(
        value,
        kb::VALUE_HAS_START_PRECISION,
        PRECISIONS,
        Rule::UnknownPrecision,
        violations,
    );
    let end_precision = word(
        value,
        kb::VALUE_HAS_END_PRECISION,
        PRECISIONS,
        Rule::UnknownPrecision,
        violations,
    );
    let start = jdn(value, kb::VALUE_HAS_START_JDN, violations);
    let end = jdn(value, kb::VALUE_HAS_END_JDN, violations);
    let (
        Some(lexical),
        Some(calendar),
        Some(start_precision),
        Some(end_precision),
        Some((start, start_day)),
        Some((end, end_day)),
    ) = (lexical, calendar, start_precision, end_precision, start, end)
    else {
        return None;
    };
    if start_day > end_day {
        violations.record(Rule::InvertedDate, value.node.as_str(), "");
        return None;
    }
    let integer = |l: &Literal| -> Term { integer_literal(l).into() };
    let word = |w: &str| -> Term { Literal::new_simple_literal(w).into() };
    Some(Content {
        rdf_value: Some(Literal::new_typed_literal(lexical.value(), nn(vocab::DAO_DATE))),
        extra: vec![
            (vocab::DAO_DATE_CALENDAR, word(calendar)),
            (vocab::DAO_DATE_START_JDN, integer(start)),
            (vocab::DAO_DATE_END_JDN, integer(end)),
            (vocab::DAO_DATE_START_PRECISION, word(start_precision)),
            (vocab::DAO_DATE_END_PRECISION, word(end_precision)),
        ],
    })
}

/// The value's one `p` literal when its lexical is one of `words`; `rule` names
/// any other word.
fn word<'a>(value: &'a Entry, p: &str, words: &[&str], rule: Rule, violations: &mut Violations) -> Option<&'a str> {
    let word = one_literal(value, p, violations)?.value();
    if !words.contains(&word) {
        violations.record(rule, value.node.as_str(), word);
        return None;
    }
    Some(word)
}

/// The value's one `p` literal, an `xsd:integer` Julian Day Number, with its
/// number. A JDN outside `i64` is reported as invalid, never wrapped.
fn jdn<'a>(value: &'a Entry, p: &str, violations: &mut Violations) -> Option<(&'a Literal, i64)> {
    let literal = one_literal(value, p, violations)?;
    let day = xsd_integer_as::<i64>(literal);
    if day.is_none() {
        violations.record(Rule::InvalidLiteral, value.node.as_str(), p);
    }
    Some((literal, day?))
}

/// §4.7: `dao:sourceListNode` naming the value's one `kb:valueHasListNode`, which
/// must be a live list node, and no `rdf:value`.
fn list_node(value: &Entry, lists: &LiveLists, violations: &mut Violations) -> Option<Content> {
    let (s, p) = (value.node.as_str(), kb::VALUE_HAS_LIST_NODE);
    let rule = match kinds::objects(value, p).as_slice() {
        [Term::NamedNode(node)] if lists.contains(node.as_str()) => {
            return Some(Content {
                rdf_value: None,
                extra: vec![(vocab::DAO_SOURCE_LIST_NODE, node.clone().into())],
            });
        }
        [Term::NamedNode(node)] => {
            violations.record(Rule::ListValueTarget, s, node.as_str());
            return None;
        }
        [] => Rule::MissingSourceLiteral,
        [_] => Rule::InvalidLiteral,
        _ => Rule::RepeatedSourceLiteral,
    };
    violations.record(rule, s, p);
    None
}

/// The lexical of the value's one `source` literal, verbatim, retyped from the
/// `from` datatype the dump must give it to `to`. A language tag or another
/// datatype would be lost in the retyping, so it is reported instead.
fn retyped(value: &Entry, source: &str, from: &str, to: &str, violations: &mut Violations) -> Option<Content> {
    let literal = one_literal(value, source, violations)?;
    if literal.datatype().as_str() != from {
        violations.record(Rule::InvalidLiteral, value.node.as_str(), source);
        return None;
    }
    Some(Literal::new_typed_literal(literal.value(), nn(to)).into())
}

/// `"<lexical>"^^datatype` from the value's one `source` literal, which the dump
/// must already type `datatype` with a lexical `valid` accepts. The store has
/// already put an integer, decimal or boolean lexical in canonical form (§1).
fn typed(
    value: &Entry,
    source: &str,
    datatype: &str,
    valid: fn(&str) -> bool,
    violations: &mut Violations,
) -> Option<Literal> {
    let literal = one_literal(value, source, violations)?;
    if literal.datatype().as_str() != datatype || !valid(literal.value()) {
        violations.record(Rule::InvalidLiteral, value.node.as_str(), source);
        return None;
    }
    Some(Literal::new_typed_literal(literal.value(), nn(datatype)))
}

/// §5: `dao:valueHasOrder` when the dump records `kb:valueHasOrder`; `Err` after
/// recording why it cannot be carried.
fn order(value: &Entry, violations: &mut Violations) -> Result<Option<Literal>, ()> {
    let source = kb::VALUE_HAS_ORDER;
    let Some(order) = at_most_one_literal(value, source, violations)? else {
        return Ok(None);
    };
    if !is_xsd_integer(order) {
        violations.record(Rule::InvalidLiteral, value.node.as_str(), source);
        return Err(());
    }
    Ok(Some(integer_literal(order)))
}

/// The value's one literal object of `p`, or `None` after recording why not.
pub(crate) fn one_literal<'a>(value: &'a Entry, p: &str, violations: &mut Violations) -> Option<&'a Literal> {
    match at_most_one_literal(value, p, violations) {
        Ok(Some(literal)) => Some(literal),
        Ok(None) => {
            violations.record(Rule::MissingSourceLiteral, value.node.as_str(), p);
            None
        }
        Err(()) => None,
    }
}

/// The value's literal object of `p` if it has one; `Err` after recording a
/// repeated or non-literal object.
pub(crate) fn at_most_one_literal<'a>(
    value: &'a Entry,
    p: &str,
    violations: &mut Violations,
) -> Result<Option<&'a Literal>, ()> {
    let rule = match kinds::objects(value, p).as_slice() {
        [] => return Ok(None),
        [Term::Literal(literal)] => return Ok(Some(literal)),
        [_] => Rule::InvalidLiteral,
        _ => Rule::RepeatedSourceLiteral,
    };
    violations.record(rule, value.node.as_str(), p);
    Err(())
}

/// A literal the dump types `xsd:integer` with a valid lexical.
pub(crate) fn is_xsd_integer(literal: &Literal) -> bool {
    literal.datatype().as_str() == model::XSD_INTEGER && is_integer(literal.value())
}

/// The number of an [`is_xsd_integer`] literal, when it fits `T`.
pub(crate) fn xsd_integer_as<T: FromStr>(literal: &Literal) -> Option<T> {
    if is_xsd_integer(literal) {
        literal.value().parse().ok()
    } else {
        None
    }
}

/// The literal's lexical, typed `xsd:integer`.
pub(crate) fn integer_literal(literal: &Literal) -> Literal {
    Literal::new_typed_literal(literal.value(), nn(vocab::XSD_INTEGER))
}

/// A plain string: `xsd:string`, no language tag.
pub(crate) fn is_plain_string(literal: &Literal) -> bool {
    literal.language().is_none() && literal.datatype().as_str() == model::XSD_STRING
}

/// An `xsd:integer` lexical: an optional sign, then digits.
fn is_integer(lexical: &str) -> bool {
    let digits = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// An `xsd:decimal` lexical: an optional sign, digits with at most one `.`, at
/// least one digit.
fn is_decimal(lexical: &str) -> bool {
    let unsigned = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    !(whole.is_empty() && fraction.is_empty()) && digits(whole) && digits(fraction)
}

/// An `xsd:boolean` lexical: `true`, `false`, `1` or `0`.
fn is_boolean(lexical: &str) -> bool {
    matches!(lexical, "true" | "false" | "1" | "0")
}
