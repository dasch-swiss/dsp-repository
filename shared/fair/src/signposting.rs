//! FAIR Signposting: the typed links a published object carries, and the URL
//! layout the consuming service hands in for them to point at.
//!
//! Level 1 of the Signposting profile. The same set is emitted twice — as an
//! HTTP `Link` header and as `<link>` elements in the document head — so it is
//! built once here and rendered twice by the consumer.

use std::borrow::Cow;

use crate::project_graph::ProjectGraph;

/// `rel="type"` targets for a project: the object itself, and the page
/// describing it.
const SCHEMA_ORG_DATASET: &str = "https://schema.org/Dataset";
const SCHEMA_ORG_ABOUT_PAGE: &str = "https://schema.org/AboutPage";

/// `nameIdentifier` scheme that `rel="author"` accepts. Signposting wants an
/// author *identifier*, so a GND or a bare name is not one.
const ORCID: &str = "ORCID";

/// Where a consuming service publishes an object.
///
/// Every URL a writer emits arrives through this type. `shared-fair` holds no
/// route table, no base URL and no path into a service module — which
/// `.github/scripts/check-shared-paths.sh` enforces — so it cannot build one of
/// these itself, and a service that serves the same graph at different URLs
/// (the site and the OAI endpoint are two hosts on DEV) hands in both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UrlLayout {
    /// The object's canonical landing page.
    pub landing: String,
    /// The catalogue the object is listed in, for `includedInDataCatalog`.
    pub catalog: String,
    /// The machine-readable representations served beside the landing page, as
    /// `(media type, URL)`. Filled by the consuming service from its own route
    /// table; empty for a consumer that serves none, as the OAI writers do.
    pub representations: Vec<(String, String)>,
    /// OAI `GetRecord` URLs describing the object, as `(media type, URL)`.
    /// Built from the OAI endpoint's own advertised base URL, never derived
    /// from the site's.
    pub oai_records: Vec<(String, String)>,
}

impl UrlLayout {
    /// The representations a client can negotiate for, in the order they are
    /// offered.
    ///
    /// Every candidate is also a `describedby` target, built from the same
    /// pairs, so a representation can never be negotiable without being linked.
    /// Not the converse: `describedby` additionally carries the OAI records in
    /// [`oai_records`](Self::oai_records), which are deliberately not
    /// negotiable.
    pub fn candidates(&self) -> Vec<Candidate> {
        self.representations
            .iter()
            .map(|(media_type, url)| Candidate { media_type: media_type.clone(), url: url.clone() })
            .collect()
    }
}

/// One negotiable representation: what a client can ask for, and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub media_type: String,
    pub url: String,
}

/// One typed link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub rel: &'static str,
    pub href: String,
    pub media_type: Option<String>,
}

impl Link {
    fn untyped(rel: &'static str, href: impl Into<String>) -> Self {
        Self { rel, href: href.into(), media_type: None }
    }

    fn typed(rel: &'static str, href: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self { rel, href: href.into(), media_type: Some(media_type.into()) }
    }

    /// This link as one RFC 8288 `link-value`.
    fn to_field_value(&self) -> String {
        let href = escape_delimiters(&self.href);
        match self.media_type {
            Some(ref media_type) => format!(r#"<{href}>; rel="{}"; type="{}""#, self.rel, media_type),
            None => format!(r#"<{href}>; rel="{}""#, self.rel),
        }
    }
}

/// Percent-encodes the characters that are field syntax in RFC 8288.
///
/// An href here is built from the graph's canonical identifiers — an ARK, a
/// license URI, an ORCID — and those come out of curator-entered fields. A `>`
/// would end the `URI-Reference`, a `"` would end a parameter value, and `;`
/// and `,` separate parameters and link-values, so a single malformed PID could
/// turn one link into two. `HeaderValue::from_str` catches none of them: it
/// rejects control characters only.
///
/// Applied at serialization, not at construction, so `Link.href` stays the
/// identifier the JSON-LD and the `<link>` elements carry. `<`, `>`, `"` and
/// space may not appear in a URI at all (RFC 3986), so encoding them costs
/// nothing; `;` and `,` are sub-delimiters that none of these identifier
/// schemes uses.
///
/// CR and LF are deliberately left alone: the HTTP layer rejects them, and the
/// `Link` header is then dropped with a warning rather than carrying a value
/// built from a PID nobody can have meant.
fn escape_delimiters(href: &str) -> Cow<'_, str> {
    if !href.contains(['<', '>', '"', ';', ',', ' ']) {
        return Cow::Borrowed(href);
    }
    let mut out = String::with_capacity(href.len());
    for c in href.chars() {
        match c {
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '"' => out.push_str("%22"),
            ';' => out.push_str("%3B"),
            ',' => out.push_str("%2C"),
            ' ' => out.push_str("%20"),
            other => out.push(other),
        }
    }
    Cow::Owned(out)
}

/// The links one published object carries, in emission order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkSet(Vec<Link>);

impl LinkSet {
    /// The set as an RFC 8288 `Link` field value.
    ///
    /// Every href here is a URI built from the graph's canonical identifiers and
    /// the layout, never from a request path and never from free text, so a
    /// value the HTTP layer rejects means a bug or bad data, not an injection.
    /// Those identifiers are still curator-entered, so [`escape_delimiters`]
    /// takes the field syntax out of them before they are joined.
    pub fn to_header_string(&self) -> String {
        self.0.iter().map(Link::to_field_value).collect::<Vec<_>>().join(", ")
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Link> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a LinkSet {
    type Item = &'a Link;
    type IntoIter = std::slice::Iter<'a, Link>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The Signposting link set for a project.
///
/// Cardinalities are the profile's: exactly one `cite-as`, two `type`, one
/// `describedby` per description of the object, at most one `license` (so a
/// project licensed several ways gets none, because the profile has no way to
/// say "these apply together" — the JSON-LD still lists them all), and one
/// `author` per creator identifier.
pub fn project_to_link_set(graph: &ProjectGraph, urls: &UrlLayout) -> LinkSet {
    let mut links = vec![
        Link::untyped("cite-as", &graph.ark),
        Link::untyped("type", SCHEMA_ORG_DATASET),
        Link::untyped("type", SCHEMA_ORG_ABOUT_PAGE),
    ];

    for (media_type, url) in urls.representations.iter().chain(&urls.oai_records) {
        links.push(Link::typed("describedby", url, media_type));
    }

    if let [only] = graph.license_uris().as_slice() {
        links.push(Link::untyped("license", *only));
    }

    // The same creators the JSON-LD names, so the two cannot disagree about
    // who is credited. The `DaSCH` fallback carries no identifier and so adds
    // no link, which is the intent: it stands in for an attribution nobody
    // made.
    for agent in graph.creators_with_fallback().iter() {
        for identifier in &agent.name_identifiers {
            if identifier.scheme == ORCID {
                links.push(Link::untyped("author", &identifier.identifier));
            }
        }
    }

    LinkSet(links)
}

/// The link set a machine-readable representation carries: one `describes`
/// pointing back at the landing page it is a representation of.
///
/// The counterpart of the page's `describedby` links, which is what makes the
/// link graph symmetric between the page and its representations. Built here
/// rather than formatted at the call site so the representation routes go
/// through the same delimiter-guarded serialisation the page's header does.
pub fn representation_to_link_set(urls: &UrlLayout) -> LinkSet {
    LinkSet(vec![Link::untyped("describes", &urls.landing)])
}

#[cfg(test)]
mod tests {
    use shared_metadata::{Attribution, ProjectRaw};

    use super::*;
    use crate::test_support::{build, legal, project};

    fn layout() -> UrlLayout {
        UrlLayout {
            landing: "https://example.test/dpe/projects/0001".to_string(),
            catalog: "https://example.test/dpe/projects".to_string(),
            representations: Vec::new(),
            oai_records: vec![
                (
                    "application/xml".to_string(),
                    "https://oai.example.test/dpe/oai?verb=GetRecord&identifier=oai:dasch.swiss:ark:/72163/1/0001&metadataPrefix=oai_datacite".to_string(),
                ),
                (
                    "application/xml".to_string(),
                    "https://oai.example.test/dpe/oai?verb=GetRecord&identifier=oai:dasch.swiss:ark:/72163/1/0001&metadataPrefix=oai_dc".to_string(),
                ),
            ],
        }
    }

    fn rels<'a>(set: &'a LinkSet, rel: &str) -> Vec<&'a str> {
        set.iter()
            .filter(|link| link.rel == rel)
            .map(|link| link.href.as_str())
            .collect()
    }

    #[test]
    fn cite_as_is_the_ark_exactly_once() {
        let set = project_to_link_set(&build(&project(), &[]), &layout());
        assert_eq!(rels(&set, "cite-as"), vec!["https://ark.dasch.swiss/ark:/72163/1/0001"]);
    }

    #[test]
    fn both_type_links_are_emitted() {
        let set = project_to_link_set(&build(&project(), &[]), &layout());
        assert_eq!(rels(&set, "type"), vec![SCHEMA_ORG_DATASET, SCHEMA_ORG_ABOUT_PAGE]);
    }

    #[test]
    fn every_described_by_carries_its_media_type() {
        let set = project_to_link_set(&build(&project(), &[]), &layout());
        let typed: Vec<_> = set
            .iter()
            .filter(|link| link.rel == "describedby")
            .map(|link| link.media_type.as_deref())
            .collect();
        assert_eq!(typed, vec![Some("application/xml"), Some("application/xml")]);
    }

    #[test]
    fn license_is_dropped_when_the_project_carries_more_than_one() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
            ],
            ..project()
        };
        assert!(rels(&project_to_link_set(&build(&raw, &[]), &layout()), "license").is_empty());
    }

    #[test]
    fn no_license_link_without_a_usable_uri() {
        let raw = ProjectRaw { legal_info: vec![legal("MISSING", "MISSING")], ..project() };
        assert!(rels(&project_to_link_set(&build(&raw, &[]), &layout()), "license").is_empty());
    }

    /// The fallback creator stands in for an attribution nobody made, so it has
    /// no identifier to link to.
    #[test]
    fn the_dasch_fallback_creator_produces_no_author_link() {
        let raw = ProjectRaw { attributions: vec![], ..project() };
        let graph = build(&raw, &[]);
        assert!(graph.creators.is_empty());
        assert!(rels(&project_to_link_set(&graph, &layout()), "author").is_empty());
    }

    #[test]
    fn an_author_link_is_the_creator_orcid() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-002".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            ..project()
        };
        assert_eq!(
            rels(&project_to_link_set(&build(&raw, &[]), &layout()), "author"),
            vec!["https://orcid.org/0000-0002-1825-0097"]
        );
    }

    /// The other half of the page's `describedby`: a representation says which
    /// page it is a representation of, through the same guarded serialisation.
    #[test]
    fn a_representation_describes_its_landing_page() {
        let set = representation_to_link_set(&layout());
        assert_eq!(
            set.to_header_string(),
            r#"<https://example.test/dpe/projects/0001>; rel="describes""#
        );
    }

    #[test]
    fn candidates_come_from_the_representation_pairs() {
        let mut urls = layout();
        urls.representations = vec![(
            "application/ld+json".to_string(),
            "https://example.test/dpe/projects/0001/metadata.jsonld".to_string(),
        )];
        assert_eq!(
            urls.candidates(),
            vec![Candidate {
                media_type: "application/ld+json".to_string(),
                url: "https://example.test/dpe/projects/0001/metadata.jsonld".to_string(),
            }]
        );
    }

    #[test]
    fn the_header_string_is_rfc_8288() {
        let set = LinkSet(vec![
            Link::untyped("cite-as", "https://ark.example.test/ark:/1/2"),
            Link::typed("describedby", "https://example.test/r.jsonld", "application/ld+json"),
        ]);
        assert_eq!(
            set.to_header_string(),
            r#"<https://ark.example.test/ark:/1/2>; rel="cite-as", <https://example.test/r.jsonld>; rel="describedby"; type="application/ld+json""#
        );
    }

    #[test]
    fn a_placeholder_or_duplicate_license_uri_is_not_a_distinct_one() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("MISSING", "MISSING"),
                legal("", ""),
            ],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(
            rels(&project_to_link_set(&graph, &layout()), "license"),
            vec!["https://creativecommons.org/licenses/by/4.0/"]
        );
    }

    /// A PID carrying RFC 8288 field syntax splits one link into several unless
    /// the delimiters are encoded. The HTTP layer would take this value: it
    /// rejects control characters only.
    #[test]
    fn field_syntax_in_an_href_cannot_forge_a_second_link_value() {
        let pid = r#"https://ark.example.test/1>; rel="author", <https://evil.test/"#;
        let raw = ProjectRaw { pid: pid.to_string(), ..project() };
        let graph = build(&raw, &[]);
        // The graph's own identifier is untouched — only the field syntax is.
        assert_eq!(graph.ark, pid);

        let set = project_to_link_set(&graph, &layout());
        let header = set.to_header_string();
        assert!(
            header.starts_with(
                r#"<https://ark.example.test/1%3E%3B%20rel=%22author%22%2C%20%3Chttps://evil.test/>; rel="cite-as""#
            ),
            "{header}"
        );
        // One `<` per link-value, and no smuggled relation.
        assert_eq!(header.matches('<').count(), set.len(), "{header}");
        assert_eq!(header.matches(r#"rel="author""#).count(), 0, "{header}");
    }

    /// CR and LF stay verbatim on purpose: the HTTP layer refuses them and the
    /// consumer drops the whole header, which is the louder outcome.
    #[test]
    fn a_newline_in_an_href_is_left_for_the_http_layer() {
        let raw = ProjectRaw {
            pid: "https://ark.example.test/1\r\nX-Injected: 1".to_string(),
            ..project()
        };
        assert!(project_to_link_set(&build(&raw, &[]), &layout())
            .to_header_string()
            .contains("\r\n"));
    }
}
