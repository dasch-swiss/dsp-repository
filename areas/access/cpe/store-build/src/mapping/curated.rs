//! The values of a resource under the `curated` nodes of its class: what the editors authored
//! under a key, as one text, as a node of a list, or as one text in each language of the site.
//!
//! A curated text is written without the whitespace around it, and one that is empty then is
//! no value. Every refusal here is held on the resource.

use cpe_core::remodel::{CuratedForm, Remodel, RemodelCurated};

use crate::draft::{Cause, Draft, Value};
use crate::error::{BuildError, Rule};
use crate::generic::curation::{Curation, Found};
use crate::generic::lists;

/// Maps the `curated` nodes of every resource that has an id.
pub(crate) fn map<'a>(draft: &mut Draft<'a>, curation: &Curation<'a>, mapping: &'a Remodel) {
    let mut outcomes = Vec::new();
    for (entry, _, class) in draft.kept() {
        let iri = entry.resource.iri.as_str();
        let mut given = Vec::new();
        let mut held = Vec::new();
        for node in &class.curated {
            let held_before = held.len();
            let values = of_node(curation, mapping, iri, node, &mut held);
            // A refused node is told from one without a value: what was held says why.
            let cause = held[held_before..].last().map(|refusal| match refusal.rule {
                Rule::CuratedCount => Cause::Count,
                _ => Cause::Value,
            });
            given.push((node.name.as_str(), cause.map_or(Ok(values), Err)));
        }
        outcomes.push((iri, given, held));
    }
    for (iri, given, held) in outcomes {
        for (name, outcome) in given {
            match outcome {
                Ok(values) => draft.give(iri, [name], &values),
                Err(cause) => draft.refuse_property(iri, [name], cause),
            }
        }
        draft.hold(iri, held);
    }
}

/// The values of the resource `iri` under `node`; none when it has none or was refused.
fn of_node<'a>(
    curation: &Curation<'a>,
    mapping: &'a Remodel,
    iri: &str,
    node: &RemodelCurated,
    held: &mut Vec<BuildError>,
) -> Vec<Value<'a>> {
    let key = node.key.as_str();
    let values = match node.form {
        // The language of a single curated text is not read.
        CuratedForm::Text => match curation.at_most_one(iri, key, held) {
            Found::One(text, _) => vec![Value::Text(text, None)],
            Found::Absent => Vec::new(),
            Found::Refused => return Vec::new(),
        },
        CuratedForm::List(list) => match curation.at_most_one(iri, key, held) {
            Found::One(text, _) => {
                let list = mapping.list(list);
                let Some(node) = list.nodes.iter().find(|node| node.slug == text) else {
                    held.push(BuildError::refused(Rule::UnknownListNode, iri, text));
                    return Vec::new();
                };
                vec![Value::ListNode(lists::node_id(list, node))]
            }
            Found::Absent => Vec::new(),
            Found::Refused => return Vec::new(),
        },
        CuratedForm::PerLanguage => {
            let Some(said) = curation.said(iri, key, held) else {
                return Vec::new();
            };
            let languages = &mapping.languages;
            let in_language = |language: &'a String| {
                let language = language.as_str();
                let found = said.iter().find(|(_, lang)| *lang == Some(language));
                found.map(|(text, _)| Value::Text(text, Some(language)))
            };
            let in_each: Vec<Value<'a>> = languages.iter().filter_map(in_language).collect();
            // One text in every language of the site and none beside them, or no text at all.
            let complete = in_each.len() == languages.len() && said.len() == in_each.len();
            if !said.is_empty() && !complete {
                let languages: Vec<&str> = said.iter().map(|(_, lang)| lang.unwrap_or_default()).collect();
                let detail = languages.join(", ");
                held.push(BuildError::refused(Rule::CuratedLanguages, iri, detail));
                return Vec::new();
            }
            in_each
        }
    };
    if values.is_empty() && node.required {
        held.push(BuildError::refused(Rule::CuratedCount, iri, key));
    }
    values
}
