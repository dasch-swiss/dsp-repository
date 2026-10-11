//! The curated overlay: what a project's editors authored about single resources, read from the
//! snapshot's flat list of keyed texts.
//!
//! A key is read where the mapping declares it, for a node of a class or for the project's
//! hook, and the builder reads three of its own on annotations. A value under a key that is
//! read nowhere is refused, and so is one on a resource of a class that does not read its key.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::Remodel;
use cpe_ports::{CuratedValue, ProjectSnapshot};

use crate::draft::{Draft, EntryClass};
use crate::error::{BuildError, Rule};
use crate::generic::text::{self, py_strip, Limits, Sentinel};

// The keys the builder reads itself. They are curated on annotations, which no `class` of a
// mapping can declare.
pub(crate) const KEEP: &str = "keep";
pub(crate) const LANG: &str = "lang";
pub(crate) const NAME: &str = "name";

const ANNOTATION_KEYS: [&str; 2] = [KEEP, LANG];
const ANNOTATION_LINK_KEYS: [&str; 3] = [KEEP, LANG, NAME];

/// A text and its language, where the editors named one.
pub(crate) type Text<'a> = (&'a str, Option<&'a str>);

/// The snapshot's curated values by resource IRI, each resource's in the order of
/// [`CuratedValue`]'s `Ord`: the snapshot's own order is unspecified.
pub(crate) struct Curation<'a> {
    by_resource: BTreeMap<&'a str, Vec<&'a CuratedValue>>,
    /// The mapping's sentinel, which no curated text may hold.
    sentinel: Option<&'a str>,
}

impl<'a> Curation<'a> {
    pub(crate) fn read(snapshot: &'a ProjectSnapshot, sentinel: Option<&'a str>) -> Self {
        let mut by_resource: BTreeMap<&str, Vec<&CuratedValue>> = BTreeMap::new();
        for value in &snapshot.curation {
            by_resource.entry(value.resource.as_str()).or_default().push(value);
        }
        for values in by_resource.values_mut() {
            values.sort();
        }
        Self { by_resource, sentinel }
    }

    /// Whether the editors curated anything about `resource`.
    pub(crate) fn has_any(&self, resource: &str) -> bool {
        self.by_resource.contains_key(resource)
    }

    /// The values of `resource` under `key` as served, in every language.
    pub(crate) fn served(&self, resource: &str, key: &str) -> Vec<&'a CuratedValue> {
        self.by_resource
            .get(resource)
            .into_iter()
            .flatten()
            .filter(|value| value.key == key)
            .copied()
            .collect()
    }

    /// Why `text` may not be a curated text, if it may not: an address would be shown as text,
    /// and the sentinel as what it stands in for. This holds on every class.
    pub(crate) fn offence(&self, text: &str) -> Option<Rule> {
        let limits = Limits {
            not_empty: false,
            no_addresses: true,
            sentinel: self.sentinel.map(Sentinel::anywhere),
        };
        text::offence(text, limits)
    }

    /// The (text, language) of the values of `resource` under `key`, each text without the
    /// whitespace around it. A text that is empty then is no value, as an empty cell of the
    /// editors' table is none. `None` when a value holds an address or the sentinel: the key is
    /// refused once, by the rule of the first such value.
    pub(crate) fn said(&self, resource: &str, key: &str, errors: &mut Vec<BuildError>) -> Option<Vec<Text<'a>>> {
        let values = self.served(resource, key);
        let offence = values.iter().find_map(|value| self.offence(&value.text));
        if let Some(rule) = offence {
            errors.push(BuildError::refused(rule, resource, key));
            return None;
        }
        let said = values
            .into_iter()
            .map(|value| (py_strip(&value.text), value.lang.as_deref()))
            .filter(|(text, _)| !text.is_empty());
        Some(said.collect())
    }

    /// The one value [`said`](Self::said) gives for a key that takes at most one.
    pub(crate) fn at_most_one(&self, resource: &str, key: &str, errors: &mut Vec<BuildError>) -> Found<'a> {
        match self.said(resource, key, errors).as_deref() {
            None => Found::Refused,
            Some([]) => Found::Absent,
            Some([(text, lang)]) => Found::One(text, *lang),
            Some(_) => {
                errors.push(BuildError::refused(Rule::CuratedCount, resource, key));
                Found::Refused
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Found<'a> {
    /// The text and its language.
    One(&'a str, Option<&'a str>),
    Absent,
    /// An error says why.
    Refused,
}

/// Refuses every curated value under a key that neither a class of `mapping` nor the builder
/// reads, and every one on a resource of a class that does not read its key: no reader would
/// see it, and what an editor wrote would be dropped without a word. A key is refused once per
/// resource, whatever its languages. The values of a resource that has no entry in `draft` are
/// passed over: it was refused for its class.
pub(crate) fn refuse_misplaced_keys(
    curation: &Curation<'_>,
    draft: &Draft<'_>,
    mapping: &Remodel,
    errors: &mut Vec<BuildError>,
) {
    for (resource, values) in &curation.by_resource {
        let Some(entry) = draft.get(resource) else {
            continue;
        };
        let keys: BTreeSet<&str> = values.iter().map(|value| value.key.as_str()).collect();
        for key in keys {
            if reads(mapping, entry.class, key) {
                continue;
            }
            let rule = if read_somewhere(mapping, key) {
                Rule::CuratedKeyOnWrongClass
            } else {
                Rule::UnknownCuratedKey
            };
            errors.push(BuildError::refused(rule, resource, key));
        }
    }
}

/// Whether a resource of `class` has `key` read: by a node of its class, or by the project's
/// hook, which the mapping's `hook` node declares the key for.
fn reads(mapping: &Remodel, class: EntryClass<'_>, key: &str) -> bool {
    match class {
        EntryClass::Mapped(index, class) => {
            let own = class.curated_keys();
            let hook_keys = mapping.hook.iter().flat_map(|hook| &hook.keys);
            let mut of_hook = hook_keys
                .filter(|declared| declared.class == index)
                .map(|declared| &declared.key);
            own.iter().any(|declared| declared.as_str() == key) || of_hook.any(|declared| declared.as_str() == key)
        }
        EntryClass::Annotation => ANNOTATION_KEYS.contains(&key),
        EntryClass::AnnotationLink => ANNOTATION_LINK_KEYS.contains(&key),
    }
}

/// Whether `key` is read on some class. The builder's own keys are read only under a mapping
/// with an `annotations` block: without one no resource is an annotation.
fn read_somewhere(mapping: &Remodel, key: &str) -> bool {
    let declared = mapping
        .indexed_classes()
        .any(|(index, class)| reads(mapping, EntryClass::Mapped(index, class), key));
    declared || (mapping.annotations.is_some() && ANNOTATION_LINK_KEYS.contains(&key))
}

#[cfg(test)]
mod tests {
    use cpe_ports::ResourceIri;

    use super::*;

    const RESOURCE: &str = "http://rdfh.ch/0123/a";
    const SENTINEL: &str = "[unbekannt]";

    /// A snapshot of nothing but `curation`, each (key, language, text) on [`RESOURCE`].
    fn snapshot(curation: &[(&str, Option<&str>, &str)]) -> ProjectSnapshot {
        let curated = |&(key, lang, text): &(&str, Option<&str>, &str)| CuratedValue {
            resource: ResourceIri(RESOURCE.to_string()),
            key: key.to_string(),
            lang: lang.map(str::to_string),
            text: text.to_string(),
        };
        ProjectSnapshot {
            shortcode: "0123".to_string(),
            resources: Vec::new(),
            list_nodes: Vec::new(),
            curation: curation.iter().map(curated).collect(),
        }
    }

    fn rules(errors: &[BuildError]) -> Vec<(Rule, &str, Option<&str>)> {
        errors
            .iter()
            .map(|error| (error.rule, error.subject.as_str(), error.detail.as_deref()))
            .collect()
    }

    // U+001F is whitespace to Python and not to `str::trim`.
    #[test]
    fn test_said_strips_each_text_drops_the_empty_ones_and_orders_by_language() {
        let snapshot = snapshot(&[
            ("blurb", Some("en"), " Sermons\u{1f}"),
            ("blurb", Some("de"), "Predigten "),
            ("blurb", Some("fr"), " \u{1f}"),
            ("credit", None, "Sammlung"),
        ]);
        let curation = Curation::read(&snapshot, Some(SENTINEL));
        let mut errors = Vec::new();

        let said = curation.said(RESOURCE, "blurb", &mut errors);

        assert_eq!(said, Some(vec![("Predigten", Some("de")), ("Sermons", Some("en"))]));
        assert_eq!(errors, []);
    }

    /// The German text is the first by language and holds the sentinel; the English one holds
    /// an address. The key is refused once, by the first value's rule.
    #[test]
    fn test_said_refuses_the_key_once_by_the_first_offending_value_in_the_ports_order() {
        let snapshot = snapshot(&[
            ("blurb", Some("en"), "see http://a.example"),
            ("blurb", Some("de"), "[unbekannt] Predigten"),
        ]);
        let curation = Curation::read(&snapshot, Some(SENTINEL));
        let mut errors = Vec::new();

        let said = curation.said(RESOURCE, "blurb", &mut errors);

        assert_eq!(said, None);
        assert_eq!(rules(&errors), [(Rule::MissingInValue, RESOURCE, Some("blurb"))]);
    }

    #[test]
    fn test_said_without_a_sentinel_takes_any_text_but_an_address() {
        let snapshot = snapshot(&[("credit", None, "[unbekannt]"), ("homepage", None, "http://a.example")]);
        let curation = Curation::read(&snapshot, None);
        let mut errors = Vec::new();

        let credit = curation.said(RESOURCE, "credit", &mut errors);
        let homepage = curation.said(RESOURCE, "homepage", &mut errors);

        assert_eq!(credit, Some(vec![("[unbekannt]", None)]));
        assert_eq!(homepage, None);
        assert_eq!(rules(&errors), [(Rule::HttpInValue, RESOURCE, Some("homepage"))]);
    }

    #[test]
    fn test_at_most_one_counts_the_texts_left_once_stripped() {
        let snapshot = snapshot(&[
            ("credit", None, " Sammlung "),
            ("credit", Some("de"), " "),
            ("blurb", Some("de"), "Predigten"),
            ("blurb", Some("en"), "Sermons"),
        ]);
        let curation = Curation::read(&snapshot, Some(SENTINEL));
        let mut errors = Vec::new();

        let credit = curation.at_most_one(RESOURCE, "credit", &mut errors);
        let absent = curation.at_most_one(RESOURCE, "genre", &mut errors);
        assert_eq!(errors, []);
        let blurb = curation.at_most_one(RESOURCE, "blurb", &mut errors);

        assert_eq!(credit, Found::One("Sammlung", None));
        assert_eq!(absent, Found::Absent);
        assert_eq!(blurb, Found::Refused);
        assert_eq!(rules(&errors), [(Rule::CuratedCount, RESOURCE, Some("blurb"))]);
    }
}
