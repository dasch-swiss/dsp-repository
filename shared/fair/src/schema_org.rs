//! The schema.org JSON-LD description of a project.
//!
//! Emitted twice: embedded in the landing page's head, and as a standalone
//! representation. The two differ only in how many `hasPart` entries they carry,
//! which is what `SchemaOrgOptions` is for.

use serde_json::{json, Map, Value};

use crate::helpers::{access_rights_to_string, real};
use crate::project_graph::{ProjectAgent, ProjectGraph};
use crate::signposting::UrlLayout;

const PUBLISHER_NAME: &str = "DaSCH";
const PUBLISHER_URL: &str = "https://dasch.swiss";

/// How much of the graph this rendering carries.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemaOrgOptions {
    /// Cap on `hasPart` entries, or `None` for all of them.
    ///
    /// The embedded block caps: a project with 27,026 records would otherwise
    /// put megabytes into every page. The complete list is harvestable from the
    /// OAI set `project:{shortcode}` and served uncapped by the standalone
    /// representation.
    pub has_part_cap: Option<usize>,
}

/// The project as a schema.org `Dataset`, in JSON-LD.
///
/// Keys are inserted in the order they should be emitted and never removed:
/// the workspace turns on `serde_json`'s `preserve_order`, so insertion order
/// *is* emission order, and `Map::remove` would silently re-sort the map.
///
/// Nothing is invented. A value the corpus records as a placeholder, or does
/// not record at all, yields no key — an absent `license` is a truthful
/// statement about the data, and a `"MISSING"` one is not.
pub fn project_to_schema_org(graph: &ProjectGraph, urls: &UrlLayout, opts: SchemaOrgOptions) -> Value {
    let mut root = Map::new();
    root.insert("@context".into(), json!("https://schema.org"));
    root.insert("@type".into(), json!("Dataset"));
    root.insert("@id".into(), json!(graph.ark));
    // A `PropertyValue`, which is the shape F-UJI reads the object identifier
    // out of (`identifier.value` in its schema.org mapping), and which says
    // which scheme the string belongs to.
    root.insert(
        "identifier".into(),
        json!({ "@type": "PropertyValue", "propertyID": "ARK", "value": graph.ark }),
    );

    let (title, alternatives) = graph.titles();
    if let Some(title) = real(&title) {
        root.insert("name".into(), json!(title));
    }
    insert_list(&mut root, "alternateName", alternatives.iter().filter_map(|t| real(t)));

    if let Some(description) = graph.description.as_deref().and_then(real) {
        root.insert("description".into(), json!(description));
    }
    insert_list(&mut root, "keywords", graph.keywords.iter().filter_map(|k| real(k)));

    insert_list(&mut root, "license", graph.license_uris());

    root.insert("isAccessibleForFree".into(), json!(is_accessible_for_free(graph)));
    root.insert("conditionsOfAccess".into(), json!(conditions_of_access(graph)));

    // The raw fact, not `publication_year_with_fallback`: `datePublished` is
    // optional here, so a project that records no usable date gets no key
    // rather than the year DataCite's mandatory field has to invent.
    if let Some(year) = &graph.publication_year {
        root.insert("datePublished".into(), json!(year));
    }

    // The mandatory-creator fallback the graph resolves, so every
    // representation governed by that rule credits the same agents. Dublin Core
    // deliberately reads `creators` raw instead.
    insert_list(&mut root, "creator", graph.creators_with_fallback().iter().map(agent_node));
    insert_list(&mut root, "contributor", graph.contributors.iter().map(agent_node));

    root.insert(
        "publisher".into(),
        json!({ "@type": "Organization", "name": PUBLISHER_NAME, "url": PUBLISHER_URL }),
    );

    insert_list(&mut root, "funder", funder_nodes(graph));
    insert_list(&mut root, "funding", funding_nodes(graph));

    insert_list(&mut root, "spatialCoverage", spatial_nodes(graph));
    insert_list(&mut root, "temporalCoverage", temporal_values(graph));

    insert_list(&mut root, "inLanguage", graph.data_language.iter().filter_map(|l| real(l)));

    root.insert("url".into(), json!(urls.landing));

    insert_list(&mut root, "citation", citation_nodes(graph));

    if let Some(producer) = producer_node(graph) {
        root.insert("producer".into(), producer);
    }

    root.insert(
        "includedInDataCatalog".into(),
        json!({ "@type": "DataCatalog", "@id": urls.catalog, "name": "DaSCH Metadata Browser", "url": urls.catalog }),
    );

    insert_list(&mut root, "hasPart", part_nodes(graph, opts.has_part_cap));

    // Only when the recorded PID differs from the ARK the writers resolved,
    // which is the case a reader cannot otherwise see.
    if let Some(pid) = real(&graph.pid) {
        if pid != graph.ark {
            root.insert("sameAs".into(), json!(pid));
        }
    }

    Value::Object(root)
}

/// JSON safe to splice into an HTML `<script>` element.
///
/// `<`, `>` and `&` become their `\u00XX` escapes. That is still the same JSON
/// — the escapes are inside string literals, and a JSON parser reads them back
/// as the original characters — and it neutralises both `</script>` and `<!--`,
/// the two sequences that move the HTML parser out of script-data state.
/// `serde_json` already escapes quotes and control characters.
///
/// Lives here so every consumer that embeds this JSON gets the same rule, and
/// returns a `String` so the crate needs no HTML templating of its own.
pub fn script_safe_json(value: &Value) -> String {
    let serialised = value.to_string();
    let mut out = String::with_capacity(serialised.len());
    for c in serialised.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            other => out.push(other),
        }
    }
    out
}

/// Inserts one value for a single entry, an array for several, and nothing at
/// all for none — the shape a schema.org consumer expects, and the reason no
/// key here is ever emitted empty.
fn insert_list(root: &mut Map<String, Value>, key: &str, values: impl IntoIterator<Item = impl Into<Value>>) {
    let mut values: Vec<Value> = values.into_iter().map(Into::into).collect();
    match values.len() {
        0 => {}
        1 => {
            root.insert(key.into(), values.remove(0));
        }
        _ => {
            root.insert(key.into(), Value::Array(values));
        }
    }
}

fn is_accessible_for_free(graph: &ProjectGraph) -> bool {
    matches!(graph.access_rights, shared_metadata::AccessRightsType::FullOpenAccess)
}

/// The same vocabulary the human-readable page uses, so the two representations
/// of an access level cannot drift; an embargo adds its date when there is one.
fn conditions_of_access(graph: &ProjectGraph) -> String {
    let text = access_rights_to_string(&graph.access_rights);
    match (&graph.access_rights, graph.embargo_date.as_deref().and_then(real)) {
        (shared_metadata::AccessRightsType::EmbargoedAccess, Some(date)) => format!("{text} until {date}"),
        _ => text.to_string(),
    }
}

fn agent_node(agent: &ProjectAgent) -> Value {
    let mut node = Map::new();
    node.insert(
        "@type".into(),
        json!(match agent.kind {
            crate::graph::AgentKind::Person => "Person",
            crate::graph::AgentKind::Organization => "Organization",
        }),
    );
    if let Some(name) = real(&agent.name) {
        node.insert("name".into(), json!(name));
    }
    if let Some(given) = agent.given_name.as_deref().and_then(real) {
        node.insert("givenName".into(), json!(given));
    }
    if let Some(family) = agent.family_name.as_deref().and_then(real) {
        node.insert("familyName".into(), json!(family));
    }
    insert_list(
        &mut node,
        "identifier",
        agent
            .name_identifiers
            .iter()
            .map(|id| json!({ "@type": "PropertyValue", "propertyID": id.scheme, "value": id.identifier })),
    );
    insert_list(
        &mut node,
        "affiliation",
        agent
            .affiliations
            .iter()
            .filter_map(|a| real(a))
            .map(|name| json!({ "@type": "Organization", "name": name })),
    );
    Value::Object(node)
}

/// One `Organization` per distinct funder name, in grant order.
fn funder_nodes(graph: &ProjectGraph) -> Vec<Value> {
    let mut names: Vec<&str> = Vec::new();
    for grant in &graph.funding {
        for name in &grant.funder_names {
            if let Some(name) = real(name) {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    names
        .into_iter()
        .map(|name| json!({ "@type": "Organization", "name": name }))
        .collect()
}

fn funding_nodes(graph: &ProjectGraph) -> Vec<Value> {
    graph
        .funding
        .iter()
        .filter_map(|grant| {
            let mut node = Map::new();
            node.insert("@type".into(), json!("MonetaryGrant"));
            if let Some(name) = grant.name.as_deref().and_then(real) {
                node.insert("name".into(), json!(name));
            }
            if let Some(number) = grant.number.as_deref().and_then(real) {
                node.insert("identifier".into(), json!(number));
            }
            if let Some(url) = grant.url.as_deref().and_then(real) {
                node.insert("url".into(), json!(url));
            }
            insert_list(
                &mut node,
                "funder",
                grant
                    .funder_names
                    .iter()
                    .filter_map(|n| real(n))
                    .map(|name| json!({ "@type": "Organization", "name": name })),
            );
            // A grant with nothing but its type says nothing.
            (node.len() > 1).then(|| Value::Object(node))
        })
        .collect()
}

fn spatial_nodes(graph: &ProjectGraph) -> Vec<Value> {
    graph
        .spatial_coverage
        .iter()
        .filter_map(|place| {
            let name = place.text.as_deref().and_then(real);
            let url = real(&place.url);
            let mut node = Map::new();
            node.insert("@type".into(), json!("Place"));
            if let Some(name) = name {
                node.insert("name".into(), json!(name));
            }
            if let Some(url) = url {
                node.insert("sameAs".into(), json!(url));
            }
            (node.len() > 1).then(|| Value::Object(node))
        })
        .collect()
}

/// The resolved ISO 8601 interval when there is one, otherwise the entry's
/// name. schema.org's `temporalCoverage` takes either.
fn temporal_values(graph: &ProjectGraph) -> Vec<&str> {
    graph
        .temporal_coverage
        .iter()
        .filter_map(|entry| {
            entry
                .resolution
                .as_ref()
                .and_then(|resolution| real(&resolution.date))
                .or_else(|| entry.name.as_deref().and_then(real))
        })
        .collect()
}

fn citation_nodes(graph: &ProjectGraph) -> Vec<Value> {
    graph
        .publications
        .iter()
        .filter_map(|publication| {
            let text = real(&publication.text)?;
            let mut node = Map::new();
            node.insert("@type".into(), json!("CreativeWork"));
            if let Some(pid) = publication.pid.as_deref().and_then(real) {
                node.insert("@id".into(), json!(pid));
            }
            node.insert("name".into(), json!(text));
            Some(Value::Object(node))
        })
        .collect()
}

/// The research project behind the data.
///
/// schema.org's `producer` is unrelated to the OAIS Producer of the Deposit
/// Area. `member` reads `creators` raw, not the fallback: a research project
/// whose only member is the organizational stand-in would be an invented
/// membership.
fn producer_node(graph: &ProjectGraph) -> Option<Value> {
    let mut node = Map::new();
    node.insert("@type".into(), json!("ResearchProject"));
    let (title, _) = graph.titles();
    if let Some(title) = real(&title) {
        node.insert("name".into(), json!(title));
    }
    if let Some(start) = real(&graph.start_date) {
        node.insert("startDate".into(), json!(start));
    }
    if let Some(end) = real(&graph.end_date) {
        node.insert("endDate".into(), json!(end));
    }
    if let Some(website) = graph.website.as_ref().and_then(|w| real(&w.url)) {
        node.insert("url".into(), json!(website));
    }
    insert_list(&mut node, "member", graph.creators.iter().map(agent_node));
    (node.len() > 1).then(|| Value::Object(node))
}

/// One `Dataset` per record, up to `cap`.
///
/// The record ARKs resolve to metadata landing pages, not downloads, so listing
/// them claims nothing about downloadability whatever the access level.
fn part_nodes(graph: &ProjectGraph, cap: Option<usize>) -> Vec<Value> {
    graph
        .parts
        .iter()
        .take(cap.unwrap_or(usize::MAX))
        .filter_map(|part| {
            let ark = real(&part.ark)?;
            let mut node = Map::new();
            node.insert("@type".into(), json!("Dataset"));
            node.insert("@id".into(), json!(ark));
            if let Some(title) = real(&part.title) {
                node.insert("name".into(), json!(title));
            }
            Some(Value::Object(node))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use shared_metadata::{AccessRights, AccessRightsType, Attribution, Multilingual, ProjectRaw};

    use super::*;
    use crate::test_support::{build, english, legal, project, record};

    fn urls() -> UrlLayout {
        UrlLayout {
            landing: "https://example.test/dpe/projects/0001".to_string(),
            catalog: "https://example.test/dpe/projects".to_string(),
            representations: Vec::new(),
            oai_records: Vec::new(),
        }
    }

    fn render(raw: &ProjectRaw) -> Value {
        project_to_schema_org(&build(raw, &[]), &urls(), SchemaOrgOptions::default())
    }

    #[test]
    fn a_project_is_a_dataset_identified_by_its_ark() {
        let doc = render(&project());
        assert_eq!(doc["@type"], "Dataset");
        assert_eq!(doc["@id"], "https://ark.dasch.swiss/ark:/72163/1/0001");
        assert_eq!(doc["identifier"]["@type"], "PropertyValue");
        assert_eq!(doc["identifier"]["propertyID"], "ARK");
        assert_eq!(doc["identifier"]["value"], "https://ark.dasch.swiss/ark:/72163/1/0001");
        assert_eq!(doc["url"], "https://example.test/dpe/projects/0001");
        assert_eq!(doc["license"], "https://creativecommons.org/licenses/by/4.0/");
    }

    #[test]
    fn several_licenses_become_an_array() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
            ],
            ..project()
        };
        assert_eq!(
            render(&raw)["license"],
            json!([
                "https://creativecommons.org/licenses/by/4.0/",
                "https://creativecommons.org/publicdomain/zero/1.0/"
            ])
        );
    }

    #[test]
    fn a_creator_carries_its_orcid_and_affiliation() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-002".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            ..project()
        };
        let creator = render(&raw)["creator"].clone();
        assert_eq!(creator["@type"], "Person");
        assert_eq!(creator["familyName"], "Dupont");
        assert_eq!(creator["identifier"]["propertyID"], "ORCID");
        assert_eq!(creator["identifier"]["value"], "https://orcid.org/0000-0002-1825-0097");
        assert_eq!(creator["affiliation"]["name"], "Schweizerischer Nationalfonds");
    }

    #[test]
    fn a_project_crediting_nobody_gets_the_dasch_organization() {
        let doc = render(&ProjectRaw { attributions: vec![], ..project() });
        assert_eq!(doc["creator"]["@type"], "Organization");
        assert_eq!(doc["creator"]["name"], "DaSCH");
        // …but not as a member of the research project, which nobody joined.
        assert!(doc["producer"].get("member").is_none(), "{doc}");
    }

    #[test]
    fn the_publisher_is_the_constant_organization() {
        let doc = render(&project());
        assert_eq!(
            doc["publisher"],
            json!({ "@type": "Organization", "name": "DaSCH", "url": "https://dasch.swiss" })
        );
    }

    #[test]
    fn no_placeholder_reaches_the_output() {
        let raw = ProjectRaw {
            name: "MISSING".to_string(),
            official_name: "MISSING".to_string(),
            description: english("MISSING"),
            legal_info: vec![legal("MISSING", "MISSING")],
            ..project()
        };
        let rendered = render(&raw).to_string();
        assert!(!rendered.contains("MISSING"), "{rendered}");
        assert!(!rendered.contains("CALCULATED"), "{rendered}");
    }

    /// `datePublished` is optional in schema.org, so a project that records no
    /// usable date gets no key. DataCite's `publicationYear` is mandatory and
    /// still carries the fallback year, which is why this cannot be resolved
    /// into the graph: the same graph has to answer both ways.
    #[test]
    fn no_usable_date_means_no_date_published_but_datacite_still_gets_its_year() {
        let raw = ProjectRaw {
            data_publication_year: Some("MISSING".to_string()),
            start_date: "MISSING".to_string(),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert!(render(&raw).get("datePublished").is_none(), "{}", render(&raw));
        assert_eq!(crate::project_to_datacite(&graph).publication_year, "2015");
    }

    #[test]
    fn a_recorded_year_reaches_date_published() {
        assert_eq!(render(&project())["datePublished"], "2008");
    }

    /// No project-level download exists, so inventing one for a score is out.
    #[test]
    fn no_distribution_is_emitted() {
        assert!(render(&project()).get("distribution").is_none());
    }

    #[test]
    fn access_rights_drive_both_access_properties() {
        let cases = [
            (AccessRightsType::FullOpenAccess, None, true, "Full Open Access"),
            (
                AccessRightsType::OpenAccessWithRestrictions,
                None,
                false,
                "Open Access with Restrictions",
            ),
            (
                AccessRightsType::EmbargoedAccess,
                Some("2027-01-01"),
                false,
                "Embargoed Access until 2027-01-01",
            ),
            (AccessRightsType::EmbargoedAccess, None, false, "Embargoed Access"),
            (AccessRightsType::MetadataOnlyAccess, None, false, "Metadata only Access"),
        ];
        for (rights, embargo, free, conditions) in cases {
            let raw = ProjectRaw {
                access_rights: AccessRights {
                    access_rights: rights.clone(),
                    embargo_date: embargo.map(str::to_string),
                },
                ..project()
            };
            let doc = render(&raw);
            assert_eq!(doc["isAccessibleForFree"], free, "{rights:?}");
            assert_eq!(doc["conditionsOfAccess"], conditions, "{rights:?}");
        }
    }

    fn many_records(count: usize) -> Vec<shared_metadata::Record> {
        (0..count)
            .map(|i| {
                record(
                    &format!("record-{i:04}"),
                    Multilingual::from([("en".to_string(), format!("Record {i}"))]),
                )
            })
            .collect()
    }

    #[test]
    fn has_part_is_capped_when_a_cap_is_set_and_whole_when_it_is_not() {
        let graph = build(&project(), &many_records(150));
        let capped = project_to_schema_org(&graph, &urls(), SchemaOrgOptions { has_part_cap: Some(100) });
        assert_eq!(capped["hasPart"].as_array().map(Vec::len), Some(100));
        let whole = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(whole["hasPart"].as_array().map(Vec::len), Some(150));
    }

    #[test]
    fn top_level_keys_keep_their_insertion_order() {
        let doc = render(&project());
        let keys: Vec<&str> = doc.as_object().expect("an object").keys().map(String::as_str).collect();
        assert_eq!(&keys[..5], &["@context", "@type", "@id", "identifier", "name"], "{keys:?}");
        let order = |key: &str| keys.iter().position(|k| *k == key);
        assert!(order("name") < order("description"), "{keys:?}");
        assert!(order("creator") < order("publisher"), "{keys:?}");
        assert!(order("url") < order("includedInDataCatalog"), "{keys:?}");
    }

    #[test]
    fn script_safe_json_escapes_the_sequences_that_break_out_of_a_script() {
        let raw = ProjectRaw {
            description: english("</script><script>alert(1)</script> and <!-- and &amp;"),
            ..project()
        };
        let out = script_safe_json(&render(&raw));
        assert!(!out.contains('<'), "{out}");
        assert!(!out.contains('>'), "{out}");
        assert!(!out.contains('&'), "{out}");
        assert!(out.contains("\\u003c/script"), "{out}");
        // Still the same JSON: a parser reads the escapes back as the original text.
        let parsed: Value = serde_json::from_str(&out).expect("escaped output should still parse");
        assert_eq!(parsed["description"], "</script><script>alert(1)</script> and <!-- and &amp;");
    }
}
