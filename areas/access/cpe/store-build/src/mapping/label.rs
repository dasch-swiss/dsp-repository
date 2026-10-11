//! The label rule. A resource's label is checked once, before anything reads it, and only when
//! its class reads it.

use cpe_core::remodel::{PropertySource, RemodelClass, TextRule};
use cpe_ports::Resource;

use crate::error::{BuildError, Rule};
use crate::generic::curation::{Curation, Found};
use crate::generic::text::{self, Limits, Sentinel};

/// Whether a property of `class` reads the label of `resource` as a text: a property whose
/// source is the label, unless a curated text stands in for it, and a property that leaves out
/// the value equal to the label. Such a reader needs a label that is not empty; an id made from
/// the label does not, and reports a label that slugs to nothing by its own rule.
pub(crate) fn is_read_as_text(resource: &Resource, class: &RemodelClass, curation: &Curation<'_>) -> bool {
    class.properties.iter().any(|property| match &property.source {
        PropertySource::Label { curated_key: None } => true,
        // The refusals of the key are the property's to report, when it is mapped.
        PropertySource::Label { curated_key: Some(key) } => {
            let iri = resource.iri.as_str();
            curation.at_most_one(iri, key.as_str(), &mut Vec::new()) == Found::Absent
        }
        PropertySource::Text { rule, .. } => match rule {
            TextRule::ExcludeLabel => true,
            TextRule::Plain | TextRule::CuratedOverride(_) => false,
        },
        PropertySource::Uri { .. } | PropertySource::Date { .. } | PropertySource::Link { .. } => false,
    })
}

/// What a label is held to: no address on a class that takes none (`no_addresses`), no
/// sentinel, and, for a reader that needs a text (`as_text`), not empty once stripped.
pub(crate) fn limits(no_addresses: bool, sentinel: Option<&str>, as_text: bool) -> Limits<'_> {
    Limits {
        not_empty: as_text,
        no_addresses,
        sentinel: sentinel.map(Sentinel::anywhere),
    }
}

/// Refuses the label of `resource` when it holds an address on a class that takes none, when it
/// holds the sentinel, or, for a reader that needs a text (`as_text`), when it is empty once
/// stripped; in that order, and once. Returns whether it was refused.
pub(crate) fn refuse(
    resource: &Resource,
    class: &RemodelClass,
    sentinel: Option<&str>,
    as_text: bool,
    errors: &mut Vec<BuildError>,
) -> bool {
    let label = resource.label.as_str();
    let limits = limits(class.no_addresses, sentinel, as_text);
    let Some(rule) = text::offence(label, limits) else {
        return false;
    };
    let detail = if rule == Rule::EmptyValue { label } else { "label" };
    errors.push(BuildError::refused(rule, resource.iri.as_str(), detail));
    true
}
