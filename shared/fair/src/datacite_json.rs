//! The DataCite record as kernel-4 JSON.
//!
//! The same [`DataCiteRecord`] the XML writer serialises, in DataCite's own
//! JSON shape: property names are the kernel's camel-case ones, `identifier`
//! plus `identifierType` become one `identifiers` entry, and `resourceType`
//! plus `resourceTypeGeneral` become the `types` object. Nothing is mapped
//! twice — the record is written from the graph by [`crate::project_to_datacite`],
//! so this writer and the XML one can only disagree about shape, never about
//! content.
//!
//! DataCite's JSON schema declares `additionalProperties: false` and types every
//! member, so a property nobody recorded is **absent**, never `null` and never
//! an empty array. [`put`] is what enforces that at every call site.

use serde_json::{Map, Value};

use crate::types::{DataCiteContributor, DataCiteCreator, DataCiteNameIdentifier, DataCiteRecord};

/// The kernel the document conforms to. The schema pins this exact string, and
/// it is the kernel-4 namespace for every 4.x version — 4.6 included.
const SCHEMA_VERSION: &str = "http://datacite.org/schema/kernel-4";

/// The DataCite record as a kernel-4 JSON document.
///
/// Key order is the kernel's reading order — identifiers, types, creators,
/// titles, publisher, publicationYear, then the recommended and optional
/// properties — which `serde_json`'s workspace-pinned `preserve_order` turns
/// into emission order.
pub fn project_to_datacite_json(record: &DataCiteRecord) -> Value {
    let mut doc = Map::new();

    put(
        &mut doc,
        "identifiers",
        Value::Array(vec![object([
            ("identifier", Value::String(record.identifier.clone())),
            ("identifierType", Value::String(record.identifier_type.clone())),
        ])]),
    );
    put(
        &mut doc,
        "types",
        object([
            ("resourceType", Value::String(record.resource_type.clone())),
            ("resourceTypeGeneral", Value::String(record.resource_type_general.clone())),
        ]),
    );
    put(
        &mut doc,
        "creators",
        Value::Array(record.creators.iter().map(creator).collect()),
    );
    put(
        &mut doc,
        "titles",
        Value::Array(
            record
                .titles
                .iter()
                .map(|title| {
                    object([
                        ("title", Value::String(title.title.clone())),
                        ("titleType", optional(&title.title_type)),
                        ("lang", optional(&title.lang)),
                    ])
                })
                .collect(),
        ),
    );
    put(&mut doc, "publisher", Value::String(record.publisher.clone()));
    // Mandatory in DataCite, which is why the record's own field is filled
    // through `ProjectGraph::publication_year_with_fallback`. Read verbatim
    // here: re-deriving it would be a second rule to keep in step.
    put(&mut doc, "publicationYear", Value::String(record.publication_year.clone()));
    put(
        &mut doc,
        "subjects",
        Value::Array(
            record
                .subjects
                .iter()
                .map(|subject| {
                    object([
                        ("subject", Value::String(subject.subject.clone())),
                        ("subjectScheme", optional(&subject.subject_scheme)),
                        ("schemeUri", optional(&subject.scheme_uri)),
                        ("lang", optional(&subject.lang)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut doc,
        "contributors",
        Value::Array(record.contributors.iter().map(contributor).collect()),
    );
    put(
        &mut doc,
        "descriptions",
        Value::Array(
            record
                .descriptions
                .iter()
                .map(|description| {
                    object([
                        ("description", Value::String(description.description.clone())),
                        ("descriptionType", Value::String(description.description_type.clone())),
                        ("lang", optional(&description.lang)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut doc,
        "dates",
        Value::Array(
            record
                .dates
                .iter()
                .map(|date| {
                    object([
                        ("date", Value::String(date.date.clone())),
                        ("dateType", Value::String(date.date_type.clone())),
                        ("dateInformation", optional(&date.date_information)),
                    ])
                })
                .collect(),
        ),
    );
    put(&mut doc, "language", optional(&record.language));
    put(
        &mut doc,
        "relatedIdentifiers",
        Value::Array(
            record
                .related_identifiers
                .iter()
                .map(|related| {
                    object([
                        ("relatedIdentifier", Value::String(related.identifier.clone())),
                        ("relatedIdentifierType", Value::String(related.related_identifier_type.clone())),
                        ("relationType", Value::String(related.relation_type.clone())),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut doc,
        "rightsList",
        Value::Array(
            record
                .rights_list
                .iter()
                .map(|rights| {
                    object([
                        ("rights", Value::String(rights.rights.clone())),
                        ("rightsUri", optional(&rights.rights_uri)),
                        ("rightsIdentifier", optional(&rights.rights_identifier)),
                        ("rightsIdentifierScheme", optional(&rights.rights_identifier_scheme)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut doc,
        "geoLocations",
        Value::Array(
            record
                .geo_locations
                .iter()
                .map(|location| object([("geoLocationPlace", Value::String(location.geo_location_place.clone()))]))
                .collect(),
        ),
    );
    put(
        &mut doc,
        "fundingReferences",
        Value::Array(
            record
                .funding_references
                .iter()
                .map(|funding| {
                    object([
                        ("funderName", Value::String(funding.funder_name.clone())),
                        ("awardNumber", optional(&funding.award_number)),
                        ("awardUri", optional(&funding.award_uri)),
                        ("awardTitle", optional(&funding.award_title)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut doc,
        "formats",
        Value::Array(record.formats.iter().cloned().map(Value::String).collect()),
    );
    put(&mut doc, "schemaVersion", Value::String(SCHEMA_VERSION.to_string()));

    Value::Object(doc)
}

fn creator(creator: &DataCiteCreator) -> Value {
    object([
        ("name", Value::String(creator.name.clone())),
        ("nameType", optional(&creator.name_type)),
        ("givenName", optional(&creator.given_name)),
        ("familyName", optional(&creator.family_name)),
        ("nameIdentifiers", name_identifiers(&creator.name_identifiers)),
        ("affiliation", affiliations(&creator.affiliations)),
    ])
}

fn contributor(contributor: &DataCiteContributor) -> Value {
    object([
        ("name", Value::String(contributor.name.clone())),
        ("nameType", optional(&contributor.name_type)),
        ("givenName", optional(&contributor.given_name)),
        ("familyName", optional(&contributor.family_name)),
        ("nameIdentifiers", name_identifiers(&contributor.name_identifiers)),
        ("affiliation", affiliations(&contributor.affiliations)),
        ("contributorType", Value::String(contributor.contributor_type.clone())),
    ])
}

fn name_identifiers(identifiers: &[DataCiteNameIdentifier]) -> Value {
    Value::Array(
        identifiers
            .iter()
            .map(|identifier| {
                object([
                    ("nameIdentifier", Value::String(identifier.identifier.clone())),
                    ("nameIdentifierScheme", Value::String(identifier.scheme.clone())),
                    ("schemeUri", optional(&identifier.scheme_uri)),
                ])
            })
            .collect(),
    )
}

/// DataCite JSON names this member `affiliation`, singular, and takes objects
/// where the record holds plain names.
fn affiliations(names: &[String]) -> Value {
    Value::Array(
        names
            .iter()
            .map(|name| object([("name", Value::String(name.clone()))]))
            .collect(),
    )
}

fn optional(value: &Option<String>) -> Value {
    match value {
        Some(text) => Value::String(text.clone()),
        None => Value::Null,
    }
}

/// An object built from the members that carry something.
fn object<const N: usize>(members: [(&str, Value); N]) -> Value {
    let mut map = Map::new();
    for (key, value) in members {
        put(&mut map, key, value);
    }
    Value::Object(map)
}

/// Inserts `key` only when `value` carries something.
///
/// Absence is the only way DataCite JSON says "not recorded": the schema is
/// `additionalProperties: false` and types every member, so `null` and `[]` are
/// both wrong where the XML writer would simply emit no element.
///
/// An **empty string is a recorded value**, not an absence, and is emitted.
/// Two properties in the record carry one: a name-only `Coverage` date, whose
/// `date` is empty because the entry resolved to a period name and nothing
/// else, and the deliberate empty `rightsURI`. The XML writer emits an empty
/// element for both, and `date` is mandatory where it appears, so dropping the
/// member would be neither parity nor valid. Only `optional` produces a `Null`,
/// and that is what a property nobody recorded looks like.
fn put(map: &mut Map<String, Value>, key: &str, value: Value) {
    let empty = match &value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(members) => members.is_empty(),
        _ => false,
    };
    if !empty {
        map.insert(key.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use shared_metadata::{Attribution, ProjectRaw};

    use super::*;
    use crate::datacite::project_to_datacite;
    use crate::test_support::{build, legal, project};

    fn document(raw: &ProjectRaw) -> Value {
        project_to_datacite_json(&project_to_datacite(&build(raw, &[])))
    }

    #[test]
    fn the_identifier_is_the_ark() {
        let doc = document(&project());
        assert_eq!(doc["identifiers"][0]["identifierType"], "ARK");
        assert_eq!(doc["identifiers"][0]["identifier"], "https://ark.dasch.swiss/ark:/72163/1/0001");
    }

    /// The JSON and the XML are two shapes of one record, so the properties a
    /// harvester reads must be the same values in both.
    #[test]
    fn the_title_creators_and_rights_are_the_records_own() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-002".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            legal_info: vec![legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/")],
            ..project()
        };
        let record = project_to_datacite(&build(&raw, &[]));
        let doc = project_to_datacite_json(&record);

        assert_eq!(doc["titles"][0]["title"], record.titles[0].title.as_str());
        let names: Vec<&str> = doc["creators"]
            .as_array()
            .expect("creators")
            .iter()
            .map(|creator| creator["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, record.creators.iter().map(|c| c.name.as_str()).collect::<Vec<_>>());
        assert_eq!(
            doc["rightsList"][0]["rightsUri"],
            "https://creativecommons.org/licenses/by/4.0/"
        );
        assert_eq!(doc["rightsList"][0]["rightsIdentifierScheme"], "SPDX");
    }

    #[test]
    fn a_creator_orcid_becomes_a_name_identifier() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-002".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            ..project()
        };
        let doc = document(&raw);
        let identifier = &doc["creators"][0]["nameIdentifiers"][0];
        assert_eq!(identifier["nameIdentifier"], "https://orcid.org/0000-0002-1825-0097");
        assert_eq!(identifier["nameIdentifierScheme"], "ORCID");
    }

    /// `publicationYear` is mandatory, and the record carries the graph's
    /// fallback rule already applied.
    #[test]
    fn the_publication_year_is_the_records_own_string() {
        let record = project_to_datacite(&build(&project(), &[]));
        let doc = project_to_datacite_json(&record);
        assert_eq!(doc["publicationYear"], record.publication_year.as_str());
        assert!(doc["publicationYear"].is_string(), "{doc}");
    }

    /// A property nobody recorded is absent. `null` and `[]` both fail the
    /// schema, and both would read as "recorded as nothing".
    #[test]
    fn nothing_unrecorded_is_emitted_as_null_or_an_empty_array() {
        let doc = document(&project());
        let members = doc.as_object().expect("an object");
        assert!(!members.contains_key("relatedIdentifiers"), "{doc}");
        assert!(!members.contains_key("formats"), "{doc}");
        for (key, value) in members {
            assert!(!value.is_null(), "{key} is null");
            if let Some(items) = value.as_array() {
                assert!(!items.is_empty(), "{key} is an empty array");
            }
        }
        let creator = &doc["creators"][0];
        assert!(
            creator.get("givenName").is_none() || creator["givenName"].is_string(),
            "{creator}"
        );
        assert!(creator.get("affiliation").is_none(), "{creator}");
    }

    /// A `temporalCoverage` entry that resolved to a period name and no date
    /// carries an empty `date`, and the XML writer emits an empty element for
    /// it. `date` is mandatory wherever a date appears, so the member must be
    /// there — an empty string is a recorded value, not an absence.
    #[test]
    fn a_name_only_coverage_date_keeps_its_empty_date_member() {
        let record = DataCiteRecord {
            dates: vec![crate::types::DataCiteDate {
                date: String::new(),
                date_type: "Coverage".to_string(),
                date_information: Some("Trajanic".to_string()),
            }],
            ..Default::default()
        };
        let date = &project_to_datacite_json(&record)["dates"][0];
        assert_eq!(date["date"], "", "{date}");
        assert_eq!(date["dateInformation"], "Trajanic", "{date}");
    }

    /// The kernel-4 reading order, which `preserve_order` makes emission order.
    #[test]
    fn the_mandatory_properties_come_first_and_in_the_kernels_order() {
        let doc = document(&project());
        let keys: Vec<&str> = doc.as_object().expect("an object").keys().map(String::as_str).collect();
        assert_eq!(
            &keys[..6],
            &[
                "identifiers",
                "types",
                "creators",
                "titles",
                "publisher",
                "publicationYear"
            ],
            "{keys:?}"
        );
        assert_eq!(keys.last(), Some(&"schemaVersion"), "{keys:?}");
    }

    #[test]
    fn the_resource_type_is_the_projects() {
        let doc = document(&project());
        assert_eq!(doc["types"]["resourceType"], "Research Project");
        assert_eq!(doc["types"]["resourceTypeGeneral"], "Project");
        assert_eq!(doc["schemaVersion"], SCHEMA_VERSION);
    }
}
