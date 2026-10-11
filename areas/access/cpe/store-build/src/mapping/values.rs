//! The values of a resource under one `property` node of the mapping.
//!
//! A node is mapped in steps, each of which can end it: the curated text that stands in for the
//! source; the source's values, which must all be of the node's kind; their number; the value
//! that repeats the label; the check of each value; the values that are left out. What remains
//! is written as served and in the port's order: no value of the archive is trimmed, and none
//! is moved. Only the curated text that stands in for a source is written stripped.
//!
//! A value of another kind is reported at once. Every other refusal is held on the resource.
//!
//! A property that the mapping's `hook` node lists under `overrides` is checked and thinned
//! only once the hook has run, on the values the hook left: [`after_hook`].

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use cpe_core::remodel::{
    ClassIndex, CuratedKey, EmptyText, HookOverrides, IdPolicy, IdSource, PropertyIndex, PropertySource, Remodel,
    RemodelClass, RemodelProperty, SentinelWhole, SentinelWithin, TextRule, ValueCount,
};
use cpe_ports::{DateValue, Resource, ValueKind};

use crate::draft::{names, Cause, Draft, Value};
use crate::error::{BuildError, Rule};
use crate::generic::curation::{Curation, Found, Text};
use crate::generic::dates;
use crate::generic::kinds::{date_of, link_of, of_kind, text_of, uri_of};
use crate::generic::text::{self, py_strip, without_record_separator, Limits, Sentinel};

/// What a node's mapping reads besides the resource itself.
pub(crate) struct Sources<'a, 'd> {
    pub(crate) mapping: &'a Remodel,
    pub(crate) curation: &'d Curation<'a>,
    pub(crate) draft: &'d Draft<'a>,
}

/// Where a node's refusals go.
pub(crate) struct NodeRefusals<'e> {
    /// A value of a kind the node does not read: reported whatever becomes of the resource.
    pub(crate) kinds: &'e mut Vec<BuildError>,
    /// Every other refusal: held on the resource.
    pub(crate) held: &'e mut Vec<BuildError>,
}

/// Maps the properties of every resource that has an id. A property that was mapped for the id
/// is not mapped again.
pub(crate) fn map<'a>(
    draft: &mut Draft<'a>,
    curation: &Curation<'a>,
    mapping: &'a Remodel,
    errors: &mut Vec<BuildError>,
) {
    let mut outcomes = Vec::new();
    for (entry, class_index, class) in draft.kept() {
        let sources = Sources { mapping, curation, draft };
        let mut given = Vec::new();
        let mut held = Vec::new();
        for (index, node) in class.indexed_properties() {
            if names(node).any(|name| entry.values.contains_key(name)) {
                continue;
            }
            let (kinds_before, held_before) = (errors.len(), held.len());
            let mut refusals = NodeRefusals { kinds: errors, held: &mut held };
            let (resource, label_refused) = (entry.resource, entry.label_refused);
            let placed = Placed { class_index, class, index, node };
            let values = of_node(&sources, resource, label_refused, placed, &mut refusals);
            let outcome = values.ok_or_else(|| {
                let miscounted = |refusal: &BuildError| refusal.rule == Rule::ValueCount;
                if errors.len() > kinds_before {
                    Cause::Kind
                } else if held[held_before..].iter().any(miscounted) {
                    Cause::Count
                } else {
                    Cause::Value
                }
            });
            given.push((node, outcome));
        }
        outcomes.push((entry.resource.iri.as_str(), given, held));
    }
    for (iri, given, held) in outcomes {
        for (node, outcome) in given {
            match outcome {
                Ok(values) => draft.give(iri, names(node), &values),
                Err(cause) => draft.refuse_property(iri, names(node), cause),
            }
        }
        draft.hold(iri, held);
    }
}

/// A `property` node and where it stands: its class, and its index among the class's properties.
#[derive(Clone, Copy)]
pub(crate) struct Placed<'a> {
    pub(crate) class_index: ClassIndex,
    pub(crate) class: &'a RemodelClass,
    pub(crate) index: PropertyIndex,
    pub(crate) node: &'a RemodelProperty,
}

/// Whether the mapping's `hook` node lists the property `property` of `class` under `overrides`.
fn is_overridden(mapping: &Remodel, class: ClassIndex, property: PropertyIndex) -> bool {
    let listed = HookOverrides { class, property };
    let mut hook = mapping.hook.iter();
    hook.any(|hook| hook.overrides.contains(&listed))
}

/// Whether the id of a resource of `class` is made from its property `property`. Such a node is
/// checked where it is mapped, whatever the hook may do with it later: an id is final before
/// the hook runs, and is made of no value the checks would refuse or leave out.
fn makes_the_id(class: &RemodelClass, property: PropertyIndex) -> bool {
    let source = match &class.id {
        IdPolicy::Minted { source, .. } => Some(*source),
        IdPolicy::Curated { fallback, .. } => *fallback,
    };
    source == Some(IdSource::Property(property))
}

/// What follows a hook, for every resource that still gets a row. The links are read as the
/// hook left them, so they are followed up before a property loses its values to a refusal.
pub(crate) fn after_hook<'a>(draft: &mut Draft<'a>, mapping: &'a Remodel, errors: &mut Vec<BuildError>) {
    refuse_links_to_dropped(draft, errors);
    check_overridden(draft, mapping, errors);
}

/// Refuses every link of a kept resource to a resource the hook dropped: the link would point
/// at no row. A target is reported once per linking resource.
fn refuse_links_to_dropped(draft: &Draft<'_>, errors: &mut Vec<BuildError>) {
    for (entry, _, _) in draft.kept() {
        let subject = entry.resource.iri.as_str();
        let linked = entry.values.values().flatten().filter_map(|value| match value {
            Value::Link(target) => Some(*target),
            _ => None,
        });
        let dropped: BTreeSet<&str> = linked
            .filter(|target| {
                let target = draft.get(target);
                target.is_some_and(|target| target.dropped_id().is_some())
            })
            .collect();
        for target in dropped {
            let refusal = BuildError::refused(Rule::LinkTargetWithoutRow, subject, target);
            errors.push(refusal);
        }
    }
}

/// Counts, checks and thins the values of every property the hook may set, as the mapping does
/// for every other property before the hook: a property with more or fewer values than its
/// node takes is refused, the first value that offends refuses the property, and a value that
/// gives no row is left out. They are the hook's values or, where the hook set none, the
/// mapping's.
fn check_overridden<'a>(draft: &mut Draft<'a>, mapping: &'a Remodel, errors: &mut Vec<BuildError>) {
    let mut checked = Vec::new();
    for (entry, class_index, class) in draft.kept() {
        let subject = entry.resource.iri.as_str();
        for (index, node) in class.indexed_properties() {
            if !is_overridden(mapping, class_index, index) {
                continue;
            }
            let Some(given) = names(node).find_map(|name| entry.values.get(name)) else {
                continue;
            };
            // The hook's values are as many as the node takes, like the archive's. The
            // mapping's own were counted as served, before any was left out.
            let set_by_hook = names(node).any(|name| entry.set_by_hook.contains(name));
            let miscounted = match &node.source {
                _ if !set_by_hook => false,
                // A resource has one label, so the property that is told from it has one
                // value: a hook may give another, and neither none nor several.
                PropertySource::Label { .. } => {
                    if given.len() != 1 {
                        errors.push(BuildError::refused(Rule::ValueCount, subject, "label"));
                    }
                    given.len() != 1
                }
                PropertySource::Text { iri, count, .. }
                | PropertySource::Uri { iri, count, .. }
                | PropertySource::Date { iri, count, .. }
                | PropertySource::Link { iri, count, .. } => {
                    counted(*count, given.len(), subject, iri.as_str(), errors).is_none()
                }
            };
            if miscounted {
                checked.push((subject, node, Err(Cause::Count)));
                continue;
            }
            let Some((check, read)) = Check::of(mapping, class, node) else {
                continue;
            };
            let texts = given.iter().filter_map(|value| match value {
                Value::Text(text, lang) => Some((*text, *lang)),
                _ => None,
            });
            let passed = check.passed(texts.collect(), subject, read, errors);
            checked.push((subject, node, passed.ok_or(Cause::Value)));
        }
    }
    for (iri, node, passed) in checked {
        match passed {
            Ok(texts) => {
                let values = texts.into_iter().map(|(text, lang)| Value::Text(text, lang));
                draft.give(iri, names(node), &values.collect::<Vec<_>>());
            }
            Err(cause) => draft.refuse_property(iri, names(node), cause),
        }
    }
}

/// The values `resource` has under the source of `node`, as served: of the node's kind, not
/// counted, not checked, none left out. A value of another kind is not among them, and neither
/// is a date whose bounds cannot be written.
pub(crate) fn served<'a>(resource: &'a Resource, node: &RemodelProperty) -> Vec<Value<'a>> {
    fn those<'a, T>(resource: &'a Resource, property: &str, read: impl Fn(&'a ValueKind) -> Option<T>) -> Vec<T> {
        let values = resource.values.iter();
        let under = values.filter(|value| value.property.as_str() == property);
        under.filter_map(|value| read(&value.kind)).collect()
    }
    let text = |(text, lang)| Value::Text(text, lang);
    match &node.source {
        PropertySource::Label { .. } => vec![Value::Text(resource.label.as_str(), None)],
        PropertySource::Text { iri, .. } => {
            let texts = those(resource, iri.as_str(), text_of);
            texts.into_iter().map(text).collect()
        }
        PropertySource::Uri { iri, .. } => {
            let uris = those(resource, iri.as_str(), uri_of);
            uris.into_iter().map(text).collect()
        }
        PropertySource::Link { iri, .. } => {
            let targets = those(resource, iri.as_str(), link_of);
            targets.into_iter().map(Value::Link).collect()
        }
        PropertySource::Date { iri, .. } => those(resource, iri.as_str(), date_of)
            .into_iter()
            .filter_map(|date| {
                let (start, end) = written_bounds(date).ok()?;
                Some(Value::Date {
                    start,
                    end,
                    precision: date.start.precision,
                    calendar: date.calendar,
                    display: None,
                })
            })
            .collect(),
    }
}

/// The values of `resource` under the node `placed`, a property of the resource's class. `None`
/// when a step refused the node, and when the label it reads was refused (`label_refused`):
/// the node then gives nothing and reports nothing.
pub(crate) fn of_node<'a>(
    sources: &Sources<'a, '_>,
    resource: &'a Resource,
    label_refused: bool,
    placed: Placed<'a>,
    refusals: &mut NodeRefusals<'_>,
) -> Option<Vec<Value<'a>>> {
    let Placed { class_index, class, index, node } = placed;
    let subject = resource.iri.as_str();
    // A property the hook may set is checked once the hook has run: `after_hook`.
    let checked_here = !is_overridden(sources.mapping, class_index, index) || makes_the_id(class, index);
    let check = || {
        let (check, _) = Check::of(sources.mapping, class, node).filter(|_| checked_here)?;
        Some(check)
    };
    match &node.source {
        PropertySource::Label { curated_key } => {
            if let Some(key) = curated_key {
                if let ControlFlow::Break(ended) = stand_in(sources, subject, key, refusals) {
                    return ended;
                }
            }
            if label_refused {
                return None;
            }
            Some(vec![Value::Text(resource.label.as_str(), None)])
        }
        PropertySource::Text { iri, count, rule, .. } => {
            let read = Texts {
                property: iri.as_str(),
                count: *count,
                rule,
                kind: text_of,
                check: check(),
            };
            texts(sources, resource, label_refused, read, refusals)
        }
        // An address is a text with no rule of its own, read from a value of its own kind.
        PropertySource::Uri { iri, count, .. } => {
            let read = Texts {
                property: iri.as_str(),
                count: *count,
                rule: &TextRule::Plain,
                kind: uri_of,
                check: check(),
            };
            texts(sources, resource, label_refused, read, refusals)
        }
        PropertySource::Date { iri, count, display_key } => {
            let property = iri.as_str();
            // A display text is the text of a date: a resource that has no value under the
            // property needs none, and one it has is not read. It is read before the dates,
            // so that it is reported beside them.
            let has_a_value = resource.values.iter().any(|value| value.property.as_str() == property);
            let display = display_key
                .as_ref()
                .filter(|_| has_a_value)
                .map(|key| display(sources, subject, key, refusals.held));
            let served = of_kind(resource, property, refusals.kinds, date_of)?;
            counted(*count, served.len(), subject, property, refusals.held)?;
            let mut bounds = Vec::new();
            for date in served {
                match written_bounds(date) {
                    Ok(written) => bounds.push((date, written)),
                    Err(rule) => {
                        let refusal = BuildError::refused(rule, subject, property);
                        refusals.held.push(refusal);
                        return None;
                    }
                }
            }
            let display = match display {
                Some(None) => return None,
                Some(Some(display)) => Some(display),
                None => None,
            };
            let values = bounds.into_iter().map(|(date, (start, end))| Value::Date {
                start,
                end,
                precision: date.start.precision,
                calendar: date.calendar,
                display,
            });
            Some(values.collect())
        }
        PropertySource::Link { iri, count, target } => {
            let property = iri.as_str();
            let targets = of_kind(resource, property, refusals.kinds, link_of)?;
            counted(*count, targets.len(), subject, property, refusals.held)?;
            let of_target_class = |linked: &&str| {
                let entry = sources.draft.get(linked);
                entry.is_some_and(|entry| entry.is_of(*target))
            };
            if let Some(stray) = targets.iter().find(|linked| !of_target_class(linked)) {
                let refusal = BuildError::refused(Rule::StripLinkTarget, subject, *stray);
                refusals.held.push(refusal);
                return None;
            }
            Some(targets.into_iter().map(Value::Link).collect())
        }
    }
}

/// How the values of a text or an address property are read.
struct Texts<'a> {
    /// The archive's property.
    property: &'a str,
    count: ValueCount,
    rule: &'a TextRule,
    /// Reads a value of the property's kind.
    kind: fn(&ValueKind) -> Option<Text<'_>>,
    /// `None` for a property that is checked once the hook has run.
    check: Option<Check<'a>>,
}

/// The texts of `resource` that `read` describes, in the steps of the module's header: the
/// curated text that stands in, the kind, the count, the value that repeats the label, the
/// check.
fn texts<'a>(
    sources: &Sources<'a, '_>,
    resource: &'a Resource,
    label_refused: bool,
    read: Texts<'a>,
    refusals: &mut NodeRefusals<'_>,
) -> Option<Vec<Value<'a>>> {
    let subject = resource.iri.as_str();
    let property = read.property;
    match read.rule {
        TextRule::CuratedOverride(key) => {
            if let ControlFlow::Break(ended) = stand_in(sources, subject, key, refusals) {
                return ended;
            }
        }
        TextRule::ExcludeLabel if label_refused => return None,
        TextRule::ExcludeLabel | TextRule::Plain => {}
    }
    let mut texts = of_kind(resource, property, refusals.kinds, read.kind)?;
    counted(read.count, texts.len(), subject, property, refusals.held)?;
    if matches!(read.rule, TextRule::ExcludeLabel) {
        let label = resource.label.as_str();
        let Some(repeated) = texts.iter().position(|(text, _)| without_record_separator(text) == label) else {
            let refusal = BuildError::refused(Rule::LabelNotATitle, subject, label);
            refusals.held.push(refusal);
            return None;
        };
        texts.remove(repeated);
    }
    if let Some(check) = &read.check {
        texts = check.passed(texts, subject, property, refusals.held)?;
    }
    let values = texts.into_iter().map(|(text, lang)| Value::Text(text, lang));
    Some(values.collect())
}

/// The curated text that stands in for a node's source: with one, it is the node's value, as
/// the editors wrote it less the whitespace around it, and the source is not read. A refused
/// key ends the node with nothing. Without a text the node goes on to its source.
fn stand_in<'a>(
    sources: &Sources<'a, '_>,
    subject: &str,
    key: &CuratedKey,
    refusals: &mut NodeRefusals<'_>,
) -> ControlFlow<Option<Vec<Value<'a>>>> {
    let found = sources.curation.at_most_one(subject, key.as_str(), refusals.held);
    match found {
        Found::One(text, _) => ControlFlow::Break(Some(vec![Value::Text(text, None)])),
        Found::Refused => ControlFlow::Break(None),
        Found::Absent => ControlFlow::Continue(()),
    }
}

/// The text a reader is shown for a date and its language: one curated text under `key`, in a
/// language of the site, which would never select a text in a language it does not have.
fn display<'a>(
    sources: &Sources<'a, '_>,
    subject: &str,
    key: &CuratedKey,
    held: &mut Vec<BuildError>,
) -> Option<(&'a str, &'a str)> {
    let key = key.as_str();
    let (rule, detail) = match sources.curation.at_most_one(subject, key, held) {
        Found::One(text, Some(lang)) => {
            let languages = &sources.mapping.languages;
            if languages.iter().any(|language| language == lang) {
                return Some((text, lang));
            }
            (Rule::UnknownDateDisplayLanguage, lang)
        }
        Found::One(text, None) => (Rule::DateDisplayWithoutLanguage, text),
        Found::Absent => (Rule::CuratedCount, key),
        Found::Refused => return None,
    };
    held.push(BuildError::refused(rule, subject, detail));
    None
}

/// The two bounds of `date` as the store writes them: each in the date's own calendar, at the
/// precision both share.
fn written_bounds(date: &DateValue) -> Result<(String, String), Rule> {
    let precision = date.start.precision;
    if precision != date.end.precision {
        return Err(Rule::MixedDatePrecision);
    }
    let bounds = [date.start, date.end].map(|bound| dates::day(bound.jdn, date.calendar));
    let [Some(start), Some(end)] = bounds else {
        return Err(Rule::DateOutOfRange);
    };
    Ok((start.text(precision), end.text(precision)))
}

/// Refuses a property that has not as many values as it takes. The values are counted as
/// served, before any is left out.
fn counted(count: ValueCount, served: usize, subject: &str, property: &str, held: &mut Vec<BuildError>) -> Option<()> {
    let served = u64::try_from(served).unwrap_or(u64::MAX);
    let enough = served >= u64::from(count.min());
    let too_many = count.max().is_some_and(|max| served > u64::from(max));
    if enough && !too_many {
        return Some(());
    }
    held.push(BuildError::refused(Rule::ValueCount, subject, property));
    None
}

/// The check of one value of a text or an address.
struct Check<'s> {
    /// What a value is refused for.
    limits: Limits<'s>,
    /// Whether the sentinel alone stands for a value that is not there.
    sentinel_is_absence: bool,
}

impl<'s> Check<'s> {
    /// The check of the texts `node` writes on a resource of `class`, and what a refusal by it
    /// names as read: the archive's property, or the label. `None` for a date and a link,
    /// which write no text.
    ///
    /// An address is what a `uri` is for, so it may hold one on any class, and one that is
    /// empty is no value. A label is held to what every label is: no address where none
    /// belongs, no sentinel, and not empty.
    fn of(mapping: &'s Remodel, class: &RemodelClass, node: &'s RemodelProperty) -> Option<(Self, &'s str)> {
        let check = |no_addresses: bool, empty: EmptyText, whole: SentinelWhole, within: SentinelWithin| {
            let sentinel = mapping.sentinel.as_deref().map(|text| Sentinel {
                text,
                whole: whole == SentinelWhole::Refuse,
                within: within == SentinelWithin::Refuse,
            });
            Check {
                limits: Limits {
                    not_empty: empty == EmptyText::Refuse,
                    no_addresses,
                    sentinel,
                },
                sentinel_is_absence: whole == SentinelWhole::Absent,
            }
        };
        match &node.source {
            PropertySource::Text { iri, empty, sentinel, sentinel_within, .. } => {
                let check = check(class.no_addresses, *empty, *sentinel, *sentinel_within);
                Some((check, iri.as_str()))
            }
            PropertySource::Uri { iri, sentinel, sentinel_within, .. } => {
                let check = check(false, EmptyText::Drop, *sentinel, *sentinel_within);
                Some((check, iri.as_str()))
            }
            PropertySource::Label { .. } => {
                let check = check(
                    class.no_addresses,
                    EmptyText::Refuse,
                    SentinelWhole::Refuse,
                    SentinelWithin::Refuse,
                );
                Some((check, "label"))
            }
            PropertySource::Date { .. } | PropertySource::Link { .. } => None,
        }
    }

    /// Whether `text` gives no row: it is empty once stripped, or it is the sentinel alone
    /// where that stands for a value that is not there.
    fn leaves_out(&self, text: &str) -> bool {
        let stripped = py_strip(text);
        let is_sentinel = |sentinel: Sentinel<'_>| stripped == sentinel.text;
        stripped.is_empty() || (self.sentinel_is_absence && self.limits.sentinel.is_some_and(is_sentinel))
    }

    /// The texts of `served` that give a row, as served. `None` when one of them may not be a
    /// value: the property is refused once, by the first such value in the port's order.
    fn passed<'a>(
        &self,
        served: Vec<Text<'a>>,
        subject: &str,
        property: &str,
        held: &mut Vec<BuildError>,
    ) -> Option<Vec<Text<'a>>> {
        let offence = |(text, _): &Text<'a>| text::offence(text, self.limits);
        if let Some(rule) = served.iter().find_map(offence) {
            held.push(BuildError::refused(rule, subject, property));
            return None;
        }
        let written = served.into_iter().filter(|(text, _)| !self.leaves_out(text));
        Some(written.collect())
    }
}
