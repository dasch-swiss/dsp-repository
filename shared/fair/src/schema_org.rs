//! The schema.org JSON-LD description of a project.
//!
//! Emitted twice: embedded in the landing page's head, and as a standalone
//! representation. The two differ only in how many records they describe —
//! under `hasPart`, and under `distribution` for those that carry a file —
//! which is what [`PartLimit`] is for. The page bounds that by a count and the
//! representation by the bytes it serialises to; both take one prefix of the
//! same ordered part list.

use serde_json::{json, Map, Value};

use crate::helpers::{access_rights_to_string, real};
use crate::project_graph::{ProjectAgent, ProjectGraph};
use crate::signposting::UrlLayout;

const PUBLISHER_NAME: &str = "DaSCH";
const PUBLISHER_URL: &str = "https://dasch.swiss";

/// The W3C provenance vocabulary, bound in the context so `prov:wasAttributedTo`
/// expands to a real IRI.
///
/// A prefix that is declared and never used produces no triple and expands away,
/// so binding it alone would say nothing. The statement below is what puts the
/// namespace in the graph.
const PROV_NAMESPACE: &str = "http://www.w3.org/ns/prov#";

/// How much of the graph this rendering carries.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemaOrgOptions {
    /// How many of the graph's parts this rendering describes.
    pub parts: PartLimit,
}

/// How far down the graph's part list a rendering goes.
///
/// One prefix, whichever variant sets its length: `hasPart` and `distribution`
/// are always taken over the *same* parts in the *same* order, so the files a
/// document describes are the files of the records it lists. A reader gets a
/// contiguous prefix of the project rather than two lists that stop in
/// different places. What is left out is harvestable from the OAI set
/// `project:{shortcode}`.
#[derive(Clone, Copy, Debug, Default)]
pub enum PartLimit {
    /// Every part the graph carries.
    ///
    /// Only safe where the caller knows the graph is small — a fixture, or a
    /// project whose records were never materialised. Neither served document
    /// uses it: the page counts and the representation measures.
    #[default]
    All,
    /// At most this many parts.
    ///
    /// What the embedded block uses. A project with 27,026 records would
    /// otherwise put megabytes into every page, and one with 7,716 files would
    /// add a `DataDownload` to each of those.
    Count(usize),
    /// As many parts as fit, with the serialised document at or under this many
    /// bytes.
    ///
    /// What the standalone representation uses. A count cannot do this job: the
    /// same 19,770 parts serialise to 4.74 MB under one ARK host and 5.25 MB
    /// under a longer one, and identifier length, licence URIs, file names and
    /// MIME types all vary per project. Only the bytes actually produced are
    /// the thing a downstream limit is applied to, so only they are measured —
    /// see [`within_budget`].
    Bytes(usize),
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
    match opts.parts {
        PartLimit::All => render(graph, urls, None),
        PartLimit::Count(cap) => render(graph, urls, Some(cap)),
        PartLimit::Bytes(budget) => within_budget(graph, urls, budget),
    }
}

/// The longest prefix of the graph's parts whose document fits in `budget`
/// bytes, and the document itself.
///
/// **Measured, never estimated.** The document returned is the very `Value`
/// whose serialisation was measured, so the number checked here and the number
/// served are one number. Adding up per-entry costs would be a second
/// implementation of `serde_json`'s output living beside the real one, and the
/// two would drift the first time a key changed shape.
///
/// Each pass scales the prefix by the ratio the last measurement gives. Treating
/// the document's fixed part as though it scaled too makes that an
/// under-estimate, so the second pass normally fits; `cap - 1` guarantees
/// progress regardless, so the loop terminates for any budget. A budget too
/// small even for a part-less document yields that document rather than
/// nothing: there is no shorter truthful answer, and a truncated one would not
/// be JSON.
fn within_budget(graph: &ProjectGraph, urls: &UrlLayout, budget: usize) -> Value {
    let mut cap = graph.parts.len();
    loop {
        let doc = render(graph, urls, Some(cap));
        let len = serde_json::to_string(&doc).expect("a Value should serialise").len();
        if len <= budget || cap == 0 {
            return doc;
        }
        let scaled = (cap as u128 * budget as u128 / len as u128) as usize;
        cap = scaled.min(cap - 1);
    }
}

fn render(graph: &ProjectGraph, urls: &UrlLayout, cap: Option<usize>) -> Value {
    let mut root = Map::new();
    root.insert("@context".into(), json!(["https://schema.org", { "prov": PROV_NAMESPACE }]));
    root.insert("@type".into(), json!("Dataset"));
    root.insert("@id".into(), json!(graph.ark));
    // Two entries, always, so this is written as an array rather than through
    // `insert_list`: the cardinality is fixed and does not depend on the graph.
    //
    // The `PropertyValue` is the shape F-UJI reads the object identifier out of
    // (`identifier.value` in its schema.org mapping), and it says which scheme
    // the ARK belongs to. The landing page URL sits beside it because FAIR
    // Champion's MetadataIdentifierFound reads `schema:identifier` alone — it
    // does not consider `url`, which already carries the same URL — so an
    // assessor pointed at the page has nothing to match without it.
    root.insert(
        "identifier".into(),
        json!([
            { "@type": "PropertyValue", "propertyID": "ARK", "value": graph.ark },
            urls.landing,
        ]),
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

    // A node object, not a string. schema.org's remote context does not coerce
    // `license` to `@id`, so a bare string parses as a literal and a consumer
    // asking for a licence *resource* finds none — which is what FAIR Champion's
    // LicenseStrong test asks for.
    insert_list(
        &mut root,
        "license",
        graph.license_uris().into_iter().map(|uri| json!({ "@id": uri })),
    );

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
        json!({ "@id": PUBLISHER_URL, "@type": "Organization", "name": PUBLISHER_NAME, "url": PUBLISHER_URL }),
    );

    insert_list(&mut root, "prov:wasAttributedTo", attributed_to(graph));

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

    insert_list(&mut root, "hasPart", part_nodes(graph, cap));
    insert_list(&mut root, "distribution", distribution_nodes(graph, cap));

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

/// The agents this dataset's existence is ascribed to, as references to nodes the
/// graph already carries: DaSCH, which publishes and curates it, and every
/// credited creator that has an IRI of its own.
///
/// References, never copies. Inlining a second copy of a creator that has no
/// `@id` would put a *different* blank node in the graph, and so assert a second,
/// unidentified agent — the opposite of what the statement means. A creator
/// without an ORCID is therefore left out: naming some agents does not deny the
/// others, so the shorter statement is still true.
///
/// Nothing here is new. Both halves restate what `creator` and `publisher`
/// already say, in the vocabulary a consumer asking about provenance reads.
fn attributed_to(graph: &ProjectGraph) -> Vec<Value> {
    let mut nodes = vec![json!({ "@id": PUBLISHER_URL })];
    for agent in graph.creators_with_fallback().iter() {
        if let Some(id) = agent.orcid() {
            nodes.push(json!({ "@id": id }));
        }
    }
    nodes
}

fn agent_node(agent: &ProjectAgent) -> Value {
    let mut node = Map::new();
    // An IRI for the agent, so `prov:wasAttributedTo` can point at this node
    // rather than describe a second one. Absent for an agent with no ORCID,
    // which stays a blank node, as it was.
    if let Some(id) = agent.orcid() {
        node.insert("@id".into(), json!(id));
    }
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

/// One `DataDownload` per listed part whose record carries a publishable file.
///
/// At the root of the graph rather than on the `hasPart` node, because that is
/// where an assessor reads it: F-UJI collects `schema:distribution` off the
/// described object, and FAIR Champion's *DataIdentifierFound* does the same.
///
/// Bounded with `hasPart`, over the same parts in the same order, so a reader
/// sees the files of the records the document lists and not of records it does
/// not. That holds however the bound was set — a count in the page, measured
/// bytes in the representation — because both are one prefix. Which parts carry
/// a file, and which are open enough for it to be named, is
/// [`crate::graph::PartRef`]'s decision, not this writer's.
fn distribution_nodes(graph: &ProjectGraph, cap: Option<usize>) -> Vec<Value> {
    graph
        .parts
        .iter()
        .take(cap.unwrap_or(usize::MAX))
        .filter_map(|part| part.file.as_ref())
        .map(|file| {
            let mut node = Map::new();
            node.insert("@type".into(), json!("DataDownload"));
            node.insert("contentUrl".into(), json!(file.url));
            if let Some(name) = file.file_name.as_deref() {
                node.insert("name".into(), json!(name));
            }
            // Omitted rather than guessed for a file whose export carries no
            // MIME type, which is every file of project 0803.
            if let Some(mime) = file.mime_type.as_deref() {
                node.insert("encodingFormat".into(), json!(mime));
            }
            if let Some(size) = file.file_size {
                node.insert("contentSize".into(), json!(size));
            }
            // The record's own licence, which the corpus records as a different
            // licence from the project's for every file-carrying record of 0868
            // and 0803. Both are reported as they stand; neither is resolved
            // against the other here. A node object for the same reason the
            // root `license` is one — a bare string parses as a literal.
            if let Some(uri) = file.license_uri.as_deref() {
                node.insert("license".into(), json!({ "@id": uri }));
            }
            Value::Object(node)
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
        assert_eq!(doc["identifier"][0]["@type"], "PropertyValue");
        assert_eq!(doc["identifier"][0]["propertyID"], "ARK");
        assert_eq!(doc["identifier"][0]["value"], "https://ark.dasch.swiss/ark:/72163/1/0001");
        assert_eq!(doc["identifier"][1], "https://example.test/dpe/projects/0001");
        assert_eq!(doc["identifier"].as_array().map(Vec::len), Some(2));
        assert_eq!(doc["url"], "https://example.test/dpe/projects/0001");
        assert_eq!(doc["license"], json!({ "@id": "https://creativecommons.org/licenses/by/4.0/" }));
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
                { "@id": "https://creativecommons.org/licenses/by/4.0/" },
                { "@id": "https://creativecommons.org/publicdomain/zero/1.0/" }
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
        // The ORCID is the node's own IRI too, so a statement about the agent can
        // point at this node instead of describing a second one.
        assert_eq!(creator["@id"], "https://orcid.org/0000-0002-1825-0097");
    }

    #[test]
    fn provenance_is_attributed_to_the_publisher_and_the_identified_creators() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-002".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            ..project()
        };
        let doc = render(&raw);
        assert_eq!(doc["@context"][1]["prov"], "http://www.w3.org/ns/prov#");
        assert_eq!(
            doc["prov:wasAttributedTo"],
            json!([
                { "@id": "https://dasch.swiss" },
                { "@id": "https://orcid.org/0000-0002-1825-0097" }
            ])
        );
        // The same agents the graph already names, by the same IRIs, so the
        // statement adds a vocabulary and not a fact.
        assert_eq!(doc["publisher"]["@id"], "https://dasch.swiss");
        assert_eq!(doc["creator"]["@id"], "https://orcid.org/0000-0002-1825-0097");
    }

    #[test]
    fn a_creator_without_an_orcid_is_left_out_of_the_attribution() {
        // `person-001` carries no ORCID, so there is no node for the statement to
        // point at. Attribution to the publisher alone is still true; inlining a
        // copy would assert a second, unidentified agent.
        let doc = render(&project());
        assert!(doc["creator"].get("@id").is_none(), "{doc}");
        assert_eq!(doc["prov:wasAttributedTo"], json!({ "@id": "https://dasch.swiss" }));
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
            json!({
                "@id": "https://dasch.swiss",
                "@type": "Organization",
                "name": "DaSCH",
                "url": "https://dasch.swiss"
            })
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
        let capped = project_to_schema_org(&graph, &urls(), SchemaOrgOptions { parts: PartLimit::Count(100) });
        assert_eq!(capped["hasPart"].as_array().map(Vec::len), Some(100));
        let whole = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(whole["hasPart"].as_array().map(Vec::len), Some(150));
    }

    /// One record with a complete file, as 0868's records carry, licensed
    /// differently from its project, as 0868's also are.
    fn record_with_a_file(id: &str, mime_type: Option<&str>) -> shared_metadata::Record {
        let mut rec = record(id, Multilingual::from([("en".to_string(), format!("Record {id}"))]));
        rec.legal_info.license.license_uri = "https://creativecommons.org/publicdomain/zero/1.0/".to_string();
        rec.file = Some(shared_metadata::RecordFile {
            mime_type: mime_type.map(str::to_string),
            url: format!("https://ingest.dasch.swiss/projects/0001/assets/{id}/original"),
            checksum: Some("9ab438922efe".to_string()),
            file_name: Some(format!("{id}.png")),
            file_size: Some(377_685),
            ..shared_metadata::RecordFile::default()
        });
        rec
    }

    #[test]
    fn a_file_becomes_a_data_download_at_the_root() {
        let graph = build(&project(), &[record_with_a_file("abc", Some("image/png"))]);
        let doc = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(
            doc["distribution"],
            json!({
                "@type": "DataDownload",
                "contentUrl": "https://ingest.dasch.swiss/projects/0001/assets/abc/original",
                "name": "abc.png",
                "encodingFormat": "image/png",
                "contentSize": 377_685,
                "license": { "@id": "https://creativecommons.org/publicdomain/zero/1.0/" },
            })
        );
    }

    /// The file's licence and the project's disagree across the corpus. Both
    /// are reported; neither is resolved against the other.
    #[test]
    fn a_data_download_carries_its_own_license_not_the_projects() {
        let graph = build(&project(), &[record_with_a_file("abc", Some("image/png"))]);
        let doc = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(doc["license"], json!({ "@id": "https://creativecommons.org/licenses/by/4.0/" }));
        assert_eq!(
            doc["distribution"]["license"],
            json!({ "@id": "https://creativecommons.org/publicdomain/zero/1.0/" })
        );
    }

    #[test]
    fn a_file_without_a_mime_type_gets_no_encoding_format() {
        let graph = build(&project(), &[record_with_a_file("abc", None)]);
        let doc = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(
            doc["distribution"]["contentUrl"],
            "https://ingest.dasch.swiss/projects/0001/assets/abc/original"
        );
        assert!(doc["distribution"].get("encodingFormat").is_none(), "{doc}");
    }

    /// A project with nothing to describe describes nothing. Inventing a
    /// download for it would be inventing a fact for a score.
    #[test]
    fn a_project_with_no_file_to_point_at_emits_no_distribution() {
        for records in [Vec::new(), many_records(3)] {
            let doc = project_to_schema_org(&build(&project(), &records), &urls(), SchemaOrgOptions::default());
            assert!(doc.get("distribution").is_none(), "{doc}");
        }
    }

    /// The cap is shared, so the files described are the files of the records
    /// listed. A record beyond the cap contributes neither.
    #[test]
    fn distribution_is_capped_with_has_part_over_the_same_parts() {
        let records: Vec<shared_metadata::Record> = (0..150)
            .map(|i| record_with_a_file(&format!("asset-{i:04}"), Some("image/png")))
            .collect();
        let graph = build(&project(), &records);
        let capped = project_to_schema_org(&graph, &urls(), SchemaOrgOptions { parts: PartLimit::Count(100) });
        assert_eq!(capped["distribution"].as_array().map(Vec::len), Some(100));
        assert_eq!(capped["hasPart"].as_array().map(Vec::len), Some(100));
        let last = &capped["distribution"][99]["contentUrl"];
        assert_eq!(last, "https://ingest.dasch.swiss/projects/0001/assets/asset-0099/original");
        let whole = project_to_schema_org(&graph, &urls(), SchemaOrgOptions::default());
        assert_eq!(whole["distribution"].as_array().map(Vec::len), Some(150));
    }

    /// Only some records of a project carry a file — 4 of 0803's first hundred
    /// do — and the cap counts parts, not files.
    #[test]
    fn a_partly_file_carrying_project_describes_only_the_files_it_has() {
        let mut records = many_records(9);
        records.push(record_with_a_file("abc", Some("image/png")));
        let graph = build(&project(), &records);
        let doc = project_to_schema_org(&graph, &urls(), SchemaOrgOptions { parts: PartLimit::Count(10) });
        assert_eq!(doc["hasPart"].as_array().map(Vec::len), Some(10));
        assert_eq!(doc["distribution"]["@type"], "DataDownload");
    }

    /// Bytes, because bytes are what a downstream limit is applied to. The
    /// document returned is measured as it is serialised, so this asserts on
    /// the same number the caller writes to the socket.
    #[test]
    fn a_byte_budget_bounds_the_serialised_document_and_leaves_it_parseable() {
        let graph = build(&project(), &many_records(2_000));
        let whole = serialised(&graph, PartLimit::All);
        let budget = whole.len() / 2;

        let bounded = serialised(&graph, PartLimit::Bytes(budget));
        assert!(bounded.len() <= budget, "{} bytes, budget {budget}", bounded.len());
        serde_json::from_str::<Value>(&bounded).expect("a bounded document is still valid JSON");
        // Bounding by throwing everything away would satisfy the line above and
        // be useless: the budget is meant to be spent.
        assert!(bounded.len() > budget * 9 / 10, "{} bytes of a {budget} budget", bounded.len());
    }

    /// The same graph and the same budget give the same bytes, every time. A
    /// harvester comparing two fetches must not see a document that moved for
    /// no reason.
    #[test]
    fn a_byte_budget_is_deterministic() {
        let graph = build(&project(), &many_records(2_000));
        let budget = serialised(&graph, PartLimit::All).len() / 3;
        assert_eq!(
            serialised(&graph, PartLimit::Bytes(budget)),
            serialised(&graph, PartLimit::Bytes(budget))
        );
    }

    /// One prefix, not two lists that stop in different places: a budget that
    /// binds still describes the files of the records it lists, and no others.
    #[test]
    fn a_budgeted_document_describes_the_files_of_the_records_it_lists() {
        // Every third record carries a file, as a real project's do.
        let records: Vec<shared_metadata::Record> = (0..900)
            .map(|i| {
                let id = format!("asset-{i:04}");
                if i % 3 == 0 {
                    record_with_a_file(&id, Some("image/png"))
                } else {
                    record(&id, Multilingual::from([("en".to_string(), format!("Record {i}"))]))
                }
            })
            .collect();
        let graph = build(&project(), &records);
        let doc: Value = serde_json::from_str(&serialised(
            &graph,
            PartLimit::Bytes(serialised(&graph, PartLimit::All).len() / 2),
        ))
        .expect("valid JSON");

        let parts = doc["hasPart"].as_array().expect("an array").len();
        assert!(parts < 900, "the budget should have bound, {parts} parts");
        // The listed prefix is 0..parts, so the files described are exactly the
        // file-carrying records inside it — one in three, rounded up.
        assert_eq!(doc["distribution"].as_array().expect("an array").len(), parts.div_ceil(3));
        let last = format!("asset-{:04}", (parts - 1) / 3 * 3);
        assert!(
            doc["distribution"].as_array().expect("an array").last().expect("a last entry")["contentUrl"]
                .as_str()
                .expect("a string")
                .contains(&last),
            "the last file should belong to a listed record"
        );
    }

    #[test]
    fn a_budget_no_document_could_reach_still_yields_valid_json() {
        let graph = build(&project(), &many_records(50));
        let bounded = serialised(&graph, PartLimit::Bytes(1));
        // There is no shorter truthful answer than the project without its
        // parts, and a truncated one would not be JSON — which is the failure
        // this whole bound exists to prevent.
        let doc: Value = serde_json::from_str(&bounded).expect("valid JSON");
        assert!(doc.get("hasPart").is_none(), "{doc}");
        assert_eq!(doc["@type"], "Dataset");
    }

    #[test]
    fn a_budget_the_whole_document_fits_in_changes_nothing() {
        let graph = build(&project(), &many_records(50));
        let whole = serialised(&graph, PartLimit::All);
        assert_eq!(serialised(&graph, PartLimit::Bytes(whole.len())), whole);
    }

    fn serialised(graph: &ProjectGraph, parts: PartLimit) -> String {
        serde_json::to_string(&project_to_schema_org(graph, &urls(), SchemaOrgOptions { parts }))
            .expect("a Value should serialise")
    }

    #[test]
    fn a_restricted_records_file_is_never_advertised() {
        let mut rec = record_with_a_file("abc", Some("image/png"));
        rec.access_rights = "Metadata only Access".to_string();
        let doc = project_to_schema_org(&build(&project(), &[rec]), &urls(), SchemaOrgOptions::default());
        assert!(doc.get("distribution").is_none(), "{doc}");
        assert!(!doc.to_string().contains("ingest."), "{doc}");
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
