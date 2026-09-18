//! Machine-readable metadata for a project landing page: the embedded JSON-LD,
//! the Dublin Core meta tags, and the Signposting links, as head markup and as
//! a `Link` header.
//!
//! **Every identifier, URL and header value here derives from the resolved
//! project's own `shortcode` and `pid`, never from the request path.** Axum
//! percent-decodes the path segment before a handler sees it, so that segment is
//! attacker-controlled; the shortcode lookup is case-insensitive, so
//! `/dpe/projects/080c` and `/dpe/projects/080C` resolve to one project and
//! produce byte-identical output. Free text — a name, a title, a description —
//! never reaches a header at all.

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use maud::{html, Markup, PreEscaped};
use shared_fair::{
    project_to_datacite, project_to_datacite_json, project_to_dublin_core_meta, project_to_link_set,
    project_to_schema_org, representation_to_link_set, script_safe_json, LinkSet, ProjectGraph, ResolveContext,
    SchemaOrgOptions, UrlLayout,
};
use shared_metadata::{ProjectRaw, Record};

use crate::AppState;

/// `hasPart` entries the embedded block carries.
///
/// One constant for two limits that must agree: the graph is built from at most
/// this many records, so a project with 27,026 of them does not allocate a
/// `PartRef` per record to emit a hundred, and the writer caps at the same
/// number, so the cap holds however the graph was built. The complete list is
/// harvestable from the OAI set `project:{shortcode}`, and the standalone
/// JSON-LD representation serves it uncapped.
const HAS_PART_CAP: usize = 100;

/// The two OAI records that describe a project, and the media type of the
/// envelope `GetRecord` returns them in.
const OAI_PREFIXES: [&str; 2] = ["oai_datacite", "oai_dc"];
const OAI_MEDIA_TYPE: &str = "application/xml";

/// The machine-readable representations served beside a landing page, as
/// `(media type, path suffix)`.
///
/// One table, read once by [`url_layout`]. Everything downstream comes from
/// `UrlLayout.representations`: the representation half of the `describedby`
/// links and, through `UrlLayout::candidates`, the whole `Accept` candidate
/// list. So adding a row here links a representation and makes it negotiable in
/// one edit, and the route table in `router.rs` is the only other place a
/// suffix appears.
///
/// Not the reverse: `describedby` also carries the two OAI records, which are
/// deliberately never candidates. A harvester is pointed at them; it is never
/// redirected to them.
const REPRESENTATIONS: [(&str, &str); 2] = [
    ("application/ld+json", "metadata.jsonld"),
    ("application/vnd.datacite.datacite+json", "metadata.datacite.json"),
];

/// Which row of [`REPRESENTATIONS`] each handler serves.
const JSON_LD: usize = 0;
const DATACITE_JSON: usize = 1;

/// Errors on the representation routes are `text/plain` with an empty body.
/// A client that asked for JSON-LD is a machine, and the content type is how it
/// learns that what came back is not the document it asked for — a bare status
/// carrying the representation's own type would invite a parse.
const PLAIN_TEXT: &str = "text/plain; charset=utf-8";

/// The head markup and response headers for a project's landing page, or
/// `None` when no project answers to `shortcode`.
///
/// An unresolved project gets nothing: no JSON-LD, no `Link` header, no
/// redirect. The always-200 "Project Not Found" body it already renders is left
/// exactly as it is.
pub(crate) fn head_extras_for_project(shortcode: &str, state: &AppState) -> Option<(Markup, HeaderMap)> {
    let raw = dpe_core::project_cache::project_raw_by_shortcode(shortcode)?;
    // The canonical shortcode, not the path segment, so the record lookup and
    // the URLs below cannot be steered by how the visitor spelled it.
    let records = dpe_core::record_cache::records_for_shortcode(&raw.shortcode);
    Some(render(raw, records.iter().copied().take(HAS_PART_CAP), state))
}

/// Renders one project's metadata. Separate from the lookup so that it can be
/// tested against a project built in the test rather than the process-global
/// cache.
///
/// Synchronous, and called before any `.await`: `ContributorLookup` carries no
/// `Sync` bound, so a `ResolveContext` cannot be held across an await point.
/// Everything it borrows is created and dropped inside this call.
fn render<'a>(
    raw: &ProjectRaw,
    records: impl IntoIterator<Item = &'a Record>,
    state: &AppState,
) -> (Markup, HeaderMap) {
    let graph = build_graph(raw, records);

    let urls = url_layout(raw, state);
    let json = script_safe_json(&project_to_schema_org(
        &graph,
        &urls,
        SchemaOrgOptions { has_part_cap: Some(HAS_PART_CAP) },
    ));
    let dublin_core = project_to_dublin_core_meta(&graph);
    let links = project_to_link_set(&graph, &urls);

    let markup = html! {
        @for (name, content) in &dublin_core {
            meta name=(name) content=(content);
        }
        @for link in &links {
            link rel=(link.rel) href=(link.href) type=[link.media_type.as_deref()];
        }
        // The one sanctioned `PreEscaped` site in this crate. Maud escapes text
        // inside `script {}`, which would corrupt the JSON; `script_safe_json`
        // has already neutralised `<`, `>` and `&`, so nothing here can close
        // the element or open a comment. A test greps all of `src/` to pin that
        // this is the only one.
        script type="application/ld+json" { (PreEscaped(json)) }
    };

    (markup, link_header(&links, &graph.shortcode))
}

/// One project's resolved graph, over the records the caller chose to
/// materialise.
///
/// The three call sites differ in nothing else: the landing page passes the
/// first [`HAS_PART_CAP`] records, the JSON-LD representation passes every one
/// of them, and DataCite passes none. `ProjectGraph::build` returns an owned
/// value, so the resolve context — whose `ContributorLookup` carries no `Sync`
/// bound — is created and dropped inside this call and can reach no await
/// point.
fn build_graph<'a>(raw: &ProjectRaw, records: impl IntoIterator<Item = &'a Record>) -> ProjectGraph {
    let (lookup, periods, enriched) = dpe_core::resolve_inputs();
    ProjectGraph::build(raw, &ResolveContext::new(lookup, periods, enriched), records)
}

/// The `Link` header for a link set, or an empty map when the HTTP layer will
/// not take it.
///
/// Every target is a URI assembled from the graph's canonical fields and the
/// configured base URLs, so a rejection means a bug or bad data — most likely a
/// control character in a recorded PID — and not an injection attempt that got
/// this far. It is logged rather than panicked, because a persistent failure is
/// a data-quality problem someone has to see, and the page itself is fine
/// without the header: the same links are in the head.
fn link_header(links: &LinkSet, shortcode: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    match HeaderValue::from_str(&links.to_header_string()) {
        Ok(value) => {
            headers.insert(header::LINK, value);
        }
        Err(error) => {
            tracing::warn!(shortcode, %error, "Link header rejected by the HTTP layer and dropped");
        }
    }
    headers
}

/// The project's schema.org graph as a standalone JSON-LD document.
///
/// Uncapped, unlike the block embedded in the page: `hasPart` lists every
/// record. For the largest committed project that is a few megabytes, which is
/// why this route sits behind the per-IP limiter from its first day
/// (`router.rs`).
pub(crate) async fn project_json_ld_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    representation(&id, &state, JSON_LD, |raw, urls| {
        let records = dpe_core::record_cache::records_for_shortcode(&raw.shortcode);
        let graph = build_graph(raw, records.iter().copied());
        project_to_schema_org(&graph, urls, SchemaOrgOptions { has_part_cap: None })
    })
    .await
}

/// The project's DataCite record as kernel-4 JSON, for a harvester that wants
/// bare DataCite rather than the OAI envelope the `describedby` links to.
pub(crate) async fn project_datacite_json_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    representation(&id, &state, DATACITE_JSON, |raw, _urls| {
        // No records: DataCite carries no part list, so building one would cost
        // a `PartRef` per record for nothing.
        let graph = build_graph(raw, std::iter::empty());
        project_to_datacite_json(&project_to_datacite(&graph))
    })
    .await
}

/// One representation response: the document `write` produces, served at the
/// media type its own row in [`REPRESENTATIONS`] names, with a `describes` link
/// back to the landing page.
///
/// 400 for a shortcode that could not name a project and 404 for one that names
/// none, mirroring `project_json_handler` in `fragments.rs`. The landing page's
/// own always-200 behaviour is deliberately not mirrored: that is a page a
/// person reads, and this is a document a machine parses.
///
/// `write` runs on a blocking thread. Building the graph and serialising it is
/// CPU-bound with nothing to await — up to 27,026 `PartRef`s and a few
/// megabytes of JSON — and on a runtime worker it stalls every other request
/// that worker is driving, `/healthz` included. Nothing it borrows reaches an
/// await point: the project is a `&'static` cache reference, and the resolve
/// context lives and dies inside `build_graph`, inside the closure.
async fn representation(
    id: &str,
    state: &AppState,
    row: usize,
    write: impl FnOnce(&'static ProjectRaw, &UrlLayout) -> serde_json::Value + Send + 'static,
) -> Response {
    if !shared_metadata::project::is_valid_shortcode(id) {
        return (StatusCode::BAD_REQUEST, plain_text()).into_response();
    }
    let Some(raw) = dpe_core::project_cache::project_raw_by_shortcode(id) else {
        return (StatusCode::NOT_FOUND, plain_text()).into_response();
    };

    // `describedby`'s other half, built before the work leaves the runtime.
    let urls = url_layout(raw, state);
    let mut headers = link_header(&representation_to_link_set(&urls), &raw.shortcode);

    let write_body = move || serde_json::to_string(&write(raw, &urls)).expect("a Value should serialise");
    let body = match tokio::task::spawn_blocking(write_body).await {
        Ok(body) => body,
        Err(error) => {
            // A panic in the writer, or a runtime shutting down. Either way the
            // rest of the site is fine, so this request gets a 500 rather than
            // taking its connection task down with it.
            tracing::error!(shortcode = %raw.shortcode, %error, "writing a representation failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, plain_text()).into_response();
        }
    };

    // `axum::Json` would hardcode `application/json`. Both documents are served
    // at their own registered type, read from the same row of `REPRESENTATIONS`
    // that the `describedby` link and the `Accept` candidate come from, or a
    // client that negotiated for one would not recognise what it got.
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(REPRESENTATIONS[row].0));
    (StatusCode::OK, headers, body).into_response()
}

fn plain_text() -> [(header::HeaderName, HeaderValue); 1] {
    [(header::CONTENT_TYPE, HeaderValue::from_static(PLAIN_TEXT))]
}

/// Where this deployment publishes the project.
///
/// The site's URLs come from `DPE_PUBLIC_BASE_URL` and the OAI ones from
/// `DPE_OAI_BASE_URL`. Neither is derived from the other: the OAI endpoint
/// advertises its own base URL, and on DEV it answers on a different host.
fn url_layout(raw: &ProjectRaw, state: &AppState) -> UrlLayout {
    let shortcode = &raw.shortcode;
    // Percent-encoded, which is what stops a recorded PID carrying an `&` or a
    // newline from rewriting the URL around it. The HTTP layer's own rejection
    // of control characters is a backstop, not the defence: the `<link>`
    // elements in the head never pass through it.
    let oai_identifier = dpe_api_oai::project_oai_identifier(raw);
    let identifier = urlencoding::encode(&oai_identifier);
    UrlLayout {
        landing: format!("{}/dpe/projects/{shortcode}", state.public_base_url),
        catalog: format!("{}/dpe/projects", state.public_base_url),
        representations: REPRESENTATIONS
            .iter()
            .map(|(media_type, suffix)| {
                (
                    media_type.to_string(),
                    format!("{}/dpe/projects/{shortcode}/{suffix}", state.public_base_url),
                )
            })
            .collect(),
        oai_records: OAI_PREFIXES
            .iter()
            .map(|prefix| {
                (
                    OAI_MEDIA_TYPE.to_string(),
                    format!(
                        "{}?verb=GetRecord&identifier={identifier}&metadataPrefix={prefix}",
                        state.oai_base_url
                    ),
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::{parse_link_header, test_state, ParsedLink, NO_PUBLIC_DIR};

    fn project() -> ProjectRaw {
        serde_json::from_str(include_str!("testdata/metadata-fixture-project.json")).expect("the fixture should parse")
    }

    fn rendered(raw: &ProjectRaw) -> (String, HeaderMap) {
        let (markup, headers) = render(raw, std::iter::empty(), &test_state());
        (markup.into_string(), headers)
    }

    fn header_links(headers: &HeaderMap) -> Vec<ParsedLink> {
        parse_link_header(headers.get(header::LINK).expect("a Link header").to_str().expect("ascii"))
    }

    #[test]
    fn the_head_carries_one_json_ld_script_the_dc_tags_and_the_links() {
        let (html, _) = rendered(&project());
        assert_eq!(html.matches(r#"<script type="application/ld+json">"#).count(), 1, "{html}");
        assert!(html.contains(r#"<meta name="DC.title" content="Rural Land Use"#), "{html}");
        assert!(html.contains(r#"<meta name="DC.accessRights""#), "{html}");
        assert!(html.contains(r#"rel="cite-as""#), "{html}");
    }

    /// The `<link>` elements and the `Link` header are one set rendered twice,
    /// so they must agree relation for relation. Maud escapes `&` in an
    /// attribute value and the header does not, which is the one difference.
    #[test]
    fn the_link_elements_and_the_link_header_are_the_same_set() {
        let (html, headers) = rendered(&project());
        let mut from_html: Vec<ParsedLink> = html
            .match_indices("<link ")
            .map(|(at, _)| {
                let element = &html[at..html[at..].find('>').expect("a closed element") + at];
                ParsedLink {
                    rel: attribute(element, "rel").expect("a rel"),
                    href: attribute(element, "href").expect("a href").replace("&amp;", "&"),
                    media_type: attribute(element, "type"),
                }
            })
            .collect();
        let mut from_header = header_links(&headers);
        from_html.sort();
        from_header.sort();
        assert_eq!(from_html, from_header);
    }

    fn attribute(element: &str, name: &str) -> Option<String> {
        let needle = format!(" {name}=\"");
        let start = element.find(&needle)? + needle.len();
        let end = start + element[start..].find('"')?;
        Some(element[start..end].to_string())
    }

    /// The two representations this deployment serves, off the site's base URL,
    /// then the two OAI records, off the OAI endpoint's own. The two base URLs
    /// differ in `test_state`, as they do on DEV, so neither can be passing by
    /// being derived from the other.
    #[test]
    fn the_describedby_targets_are_the_representations_and_the_oai_records() {
        let (_, headers) = rendered(&project());
        let described: Vec<String> = header_links(&headers)
            .into_iter()
            .filter(|link| link.rel == "describedby")
            .map(|link| link.href)
            .collect();
        let identifier = "oai%3Adasch.swiss%3Aark%3A%2F72163%2F1%2F0001";
        assert_eq!(
            described,
            vec![
                "https://example.test/dpe/projects/0001/metadata.jsonld".to_string(),
                "https://example.test/dpe/projects/0001/metadata.datacite.json".to_string(),
                format!("https://oai.example.test/dpe/oai?verb=GetRecord&identifier={identifier}&metadataPrefix=oai_datacite"),
                format!("https://oai.example.test/dpe/oai?verb=GetRecord&identifier={identifier}&metadataPrefix=oai_dc"),
            ]
        );
    }

    /// A description that tries to close the script element, and a PID that
    /// tries to inject a second `Link` entry. Neither survives.
    #[test]
    fn hostile_text_cannot_escape_the_script_or_the_query_string() {
        let mut raw = project();
        raw.description = shared_metadata::Multilingual::from([(
            "en".to_string(),
            "</script><script>alert(1)</script> and <!--".to_string(),
        )]);
        let (html, _) = rendered(&raw);
        assert_eq!(html.matches("<script").count(), 1, "{html}");
        assert!(html.contains("\\u003c/script"), "{html}");

        let mut raw = project();
        raw.pid = "https://ark.dasch.swiss/ark:/72163/1/0001&x=1".to_string();
        let (_, headers) = rendered(&raw);
        // The OAI targets specifically: the representation URLs are built from
        // the shortcode and carry no PID at all.
        let described = header_links(&headers)
            .into_iter()
            .find(|link| link.rel == "describedby" && link.href.contains("GetRecord"))
            .expect("a describedby link to an OAI record");
        assert!(described.href.contains("%26x%3D1"), "{}", described.href);
        assert!(!described.href.contains("&x=1"), "{}", described.href);
    }

    /// A control character in a recorded PID is data the writers pass through
    /// unchanged. The HTTP layer refuses it; the header goes, the page stays.
    #[test]
    fn a_header_value_the_http_layer_refuses_is_dropped_not_panicked() {
        let mut raw = project();
        raw.pid = "https://ark.dasch.swiss/ark:/72163/1/0001\r\nX-Injected: 1".to_string();
        let (html, headers) = rendered(&raw);
        assert!(headers.get(header::LINK).is_none(), "{headers:?}");
        assert!(html.contains(r#"<script type="application/ld+json">"#), "{html}");
    }

    /// The only place this crate may hand Maud pre-escaped markup is the
    /// JSON-LD splice above, whose one permitted input is `script_safe_json`.
    ///
    /// The whole of `src/`, not just this file: the site the rule actually
    /// worries about is the search-query echo in `fragments.rs`, and a grep
    /// scoped to `metadata.rs` could never have seen it.
    #[test]
    fn this_crate_splices_pre_escaped_markup_exactly_once() {
        // Built at runtime, or this literal would count as another site.
        let needle = ["PreEscaped", "("].concat();
        let mut sites: Vec<(String, usize)> = Vec::new();
        for path in rust_sources(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
            let source = std::fs::read_to_string(&path).expect("a readable source file");
            let count = source.matches(&needle).count();
            if count > 0 {
                sites.push((path.display().to_string(), count));
            }
        }
        assert_eq!(sites.len(), 1, "{sites:?}");
        assert!(sites[0].0.ends_with("metadata.rs"), "{sites:?}");
        assert_eq!(sites[0].1, 1, "{sites:?}");
    }

    fn rust_sources(dir: std::path::PathBuf) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("a readable source directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_sources(path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
        files
    }

    /// The corpus, not a fixture: the casing rule is about the shortcode index
    /// and the project cache agreeing, which only the real data exercises.
    /// `test_state` is what points `dpe-core` at it.
    ///
    /// The rate-limited sub-router is built with a passthrough limiter: these
    /// tests are about what the routes answer, and that the limiter is wired to
    /// exactly these routes is `router.rs`'s own test. A real `GovernorLayer`
    /// here would make every assertion depend on how fast the suite runs.
    fn corpus_app() -> axum::Router {
        crate::router::build_router(
            test_state(),
            NO_PUBLIC_DIR.as_ref(),
            crate::router::rate_limited_router_with(tower::layer::util::Identity::new()),
        )
    }

    async fn get(uri: &str) -> (StatusCode, HeaderMap, String) {
        let request = Request::builder().uri(uri).body(Body::empty()).expect("a request");
        let response = corpus_app().oneshot(request).await.expect("a response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.expect("a body").to_bytes();
        (status, headers, String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn head(uri: &str) -> (StatusCode, HeaderMap, String) {
        let request = Request::builder()
            .method("HEAD")
            .uri(uri)
            .body(Body::empty())
            .expect("a request");
        let response = corpus_app().oneshot(request).await.expect("a response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.expect("a body").to_bytes();
        (status, headers, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn a_landing_page_carries_the_metadata_and_the_link_header() {
        let (status, headers, body) = get("/dpe/projects/0862").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.matches(r#"<script type="application/ld+json">"#).count(), 1);
        assert!(body.contains(r#"<meta name="DC.identifier""#), "no DC tags");
        let links = header_links(&headers);
        assert_eq!(links.iter().filter(|l| l.rel == "cite-as").count(), 1);
        assert_eq!(links.iter().filter(|l| l.rel == "type").count(), 2);
        assert!(
            links.iter().filter(|l| l.rel == "describedby").all(|l| l.media_type.is_some()),
            "{links:?}"
        );
        assert!(links.iter().filter(|l| l.rel == "describedby").count() >= 2, "{links:?}");
        // Scoped to the head this module writes: whether the *rendered page*
        // shows a placeholder is `DPE_SHOW_PLACEHOLDER_VALUES`'s business and
        // predates this work.
        let head = &body[..body.find("</head>").expect("a head")];
        assert!(!head.contains("MISSING") && !head.contains("CALCULATED"), "{head}");
    }

    #[tokio::test]
    async fn shortcode_casing_changes_nothing() {
        let (_, lower_headers, lower_body) = get("/dpe/projects/081c").await;
        let (_, upper_headers, upper_body) = get("/dpe/projects/081C").await;
        assert_eq!(lower_headers.get(header::LINK), upper_headers.get(header::LINK));
        let json_ld = |body: &str| {
            let at = body.find(r#"<script type="application/ld+json">"#).expect("the script");
            body[at..body[at..].find("</script>").expect("a closed script") + at].to_string()
        };
        assert_eq!(json_ld(&lower_body), json_ld(&upper_body));
    }

    #[tokio::test]
    async fn head_and_get_answer_with_the_same_headers_and_no_body() {
        let (get_status, get_headers, _) = get("/dpe/projects/0862").await;
        let (head_status, head_headers, head_body) = head("/dpe/projects/0862").await;
        assert_eq!(get_status, head_status);
        assert!(head_body.is_empty(), "{head_body}");
        assert_eq!(get_headers.get(header::LINK), head_headers.get(header::LINK));
        assert_eq!(get_headers.get(header::CONTENT_TYPE), head_headers.get(header::CONTENT_TYPE));
    }

    #[tokio::test]
    async fn an_unknown_shortcode_carries_no_metadata_at_all() {
        let (status, headers, body) = get("/dpe/projects/zzzz").await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers.get(header::LINK).is_none(), "{headers:?}");
        assert!(!body.contains("application/ld+json"), "{body}");
        assert!(!body.contains("DC.title"), "{body}");
    }

    // --- the machine-readable representations ---

    fn content_type(headers: &HeaderMap) -> &str {
        headers
            .get(header::CONTENT_TYPE)
            .expect("a Content-Type")
            .to_str()
            .expect("ascii")
    }

    /// 0803, not 0862: only three committed projects have records at all, and
    /// the point of this route is the part list the embedded block caps.
    #[tokio::test]
    async fn the_json_ld_representation_is_the_uncapped_graph() {
        let (status, headers, body) = get("/dpe/projects/0803/metadata.jsonld").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type(&headers), REPRESENTATIONS[JSON_LD].0);

        let doc: serde_json::Value = serde_json::from_str(&body).expect("the body should be JSON");
        assert_eq!(doc["@type"], "Dataset", "{}", &body[..200.min(body.len())]);
        assert_eq!(
            doc["url"], "https://example.test/dpe/projects/0803",
            "the landing page, from the configured base URL"
        );
        // The whole set, where the embedded block stops at HAS_PART_CAP.
        let parts = doc["hasPart"].as_array().map_or(0, Vec::len);
        let records = dpe_core::record_cache::records_for_shortcode("0803").len();
        assert!(records > HAS_PART_CAP, "0803 should have more records than the cap");
        assert_eq!(parts, records, "hasPart should be uncapped here");

        // The embedded block on the same project's page is the capped one.
        let (_, _, page) = get("/dpe/projects/0803").await;
        let at = page.find(r#"<script type="application/ld+json">"#).expect("the script");
        let embedded = &page[at..page[at..].find("</script>").expect("a closed script") + at];
        let embedded: serde_json::Value =
            serde_json::from_str(embedded.split_once('>').expect("the open tag").1).expect("the block should be JSON");
        assert_eq!(
            embedded["hasPart"].as_array().map_or(0, Vec::len),
            HAS_PART_CAP,
            "the embedded block should still be capped"
        );
    }

    #[tokio::test]
    async fn the_datacite_representation_is_a_datacite_document() {
        let (status, headers, body) = get("/dpe/projects/0862/metadata.datacite.json").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type(&headers), REPRESENTATIONS[DATACITE_JSON].0);

        let doc: serde_json::Value = serde_json::from_str(&body).expect("the body should be JSON");
        assert_eq!(doc["identifiers"][0]["identifierType"], "ARK", "{doc}");
        assert_eq!(doc["types"]["resourceTypeGeneral"], "Project", "{doc}");
        assert_eq!(doc["schemaVersion"], "http://datacite.org/schema/kernel-4", "{doc}");
    }

    /// The other half of the page's `describedby`: each representation says
    /// which page it is a representation of.
    #[tokio::test]
    async fn every_representation_describes_its_landing_page() {
        for (_, suffix) in REPRESENTATIONS {
            let (_, headers, _) = get(&format!("/dpe/projects/0862/{suffix}")).await;
            assert_eq!(
                header_links(&headers),
                vec![ParsedLink {
                    rel: "describes".to_string(),
                    href: "https://example.test/dpe/projects/0862".to_string(),
                    media_type: None,
                }],
                "{suffix}"
            );
        }
    }

    /// The page links exactly the representations it serves, at exactly the
    /// media types it serves them at — both sides read the one table.
    #[tokio::test]
    async fn the_page_links_every_representation_it_serves() {
        let (_, headers, _) = get("/dpe/projects/0862").await;
        let described: Vec<(String, Option<String>)> = header_links(&headers)
            .into_iter()
            .filter(|link| link.rel == "describedby")
            .map(|link| (link.href, link.media_type))
            .collect();
        for (media_type, suffix) in REPRESENTATIONS {
            let url = format!("https://example.test/dpe/projects/0862/{suffix}");
            assert!(
                described.contains(&(url.clone(), Some(media_type.to_string()))),
                "{url} at {media_type} is not linked: {described:?}"
            );
            let (status, headers, _) = get(&format!("/dpe/projects/0862/{suffix}")).await;
            assert_eq!(status, StatusCode::OK, "{suffix}");
            assert_eq!(content_type(&headers), media_type, "{suffix}");
        }
    }

    /// A machine asked for a document; an HTML error page it cannot parse would
    /// be worse than nothing.
    #[tokio::test]
    async fn a_bad_or_unknown_shortcode_is_answered_in_plain_text() {
        for (segment, expected) in [
            ("no-such-shortcode!", StatusCode::BAD_REQUEST),
            ("%22", StatusCode::BAD_REQUEST),
            ("thisistoolongtobeashortcode", StatusCode::BAD_REQUEST),
            ("zzzz", StatusCode::NOT_FOUND),
        ] {
            for (_, suffix) in REPRESENTATIONS {
                let (status, headers, body) = get(&format!("/dpe/projects/{segment}/{suffix}")).await;
                assert_eq!(status, expected, "{segment}/{suffix}");
                assert_eq!(content_type(&headers), PLAIN_TEXT, "{segment}/{suffix}");
                assert!(body.is_empty(), "{segment}/{suffix}: {body}");
            }
        }
    }

    #[tokio::test]
    async fn shortcode_casing_changes_no_representation() {
        for (_, suffix) in REPRESENTATIONS {
            let (_, lower_headers, lower) = get(&format!("/dpe/projects/081c/{suffix}")).await;
            let (_, upper_headers, upper) = get(&format!("/dpe/projects/081C/{suffix}")).await;
            assert_eq!(lower, upper, "{suffix}");
            assert_eq!(lower_headers.get(header::LINK), upper_headers.get(header::LINK), "{suffix}");
        }
    }

    /// The path segment reaches the handler percent-decoded, so these are the
    /// characters that would end a header value or open a new one. The project
    /// does not resolve, so nothing is emitted and nothing panics.
    #[tokio::test]
    async fn a_header_injecting_shortcode_is_answered_without_a_link_header() {
        for segment in ["%0D%0AX-Injected:%201", "%22", ";", "%3E", "0862%0D%0A"] {
            let (status, headers, body) = get(&format!("/dpe/projects/{segment}")).await;
            assert_eq!(status, StatusCode::OK, "{segment}");
            assert!(headers.get(header::LINK).is_none(), "{segment}: {headers:?}");
            assert!(!body.contains("application/ld+json"), "{segment}");
        }
    }
}
