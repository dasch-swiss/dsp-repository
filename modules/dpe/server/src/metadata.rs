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
    decide, project_to_datacite, project_to_datacite_json, project_to_dublin_core_meta, project_to_link_set,
    project_to_schema_org, representation_to_link_set, script_safe_json, Decision, LinkSet, PartLimit, ProjectGraph,
    ResolveContext, SchemaOrgOptions, UrlLayout,
};
use shared_metadata::{ProjectRaw, Record};

use crate::AppState;

/// `hasPart` entries the embedded block carries.
///
/// One constant for two limits that must agree: the graph is built from at most
/// this many records, so a project with 27,026 of them does not allocate a
/// `PartRef` per record to emit a hundred, and the writer caps at the same
/// number, so the cap holds however the graph was built. The complete list is
/// harvestable from the OAI set `project:{shortcode}`; the standalone JSON-LD
/// representation carries far more of it, bounded by [`JSON_LD_BYTE_BUDGET`]
/// rather than by a count.
const HAS_PART_CAP: usize = 100;

/// Bytes the standalone JSON-LD representation is held to.
///
/// A count cannot do this job. The same 19,770 parts of project 0868 serialise
/// to 4.74 MB under `ark.dasch.swiss` and 5.25 MB under a preview's longer
/// host, and identifier length, licence URIs, file names and MIME types vary
/// per project besides — so only the bytes actually produced tell us what was
/// produced. `PartLimit::Bytes` measures them.
///
/// **Why a bound at all.** F-UJI truncates any download at 5,000,000 bytes and
/// the truncated fragment is not valid JSON, so at 5.25 MB the document could
/// not be parsed at all and `FsF-I1-01M` scored 1 of 2 on a PR preview. 0868 is
/// a small project by DaSCH standards, so this is the normal case for a project
/// with files, not an edge case.
///
/// **Why 4,000,000.** A 20% margin under that limit, because we control none of
/// the three things that consume it: the host name a deployment serves under
/// (half a megabyte of the difference above is host length alone), the limit a
/// future assessor applies, and what a later field adds to every entry. Not
/// configurable: a second value to keep in step buys nothing here, and the one
/// number is easier to reason about than a range.
const JSON_LD_BYTE_BUDGET: usize = 4_000_000;

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

/// What the landing page route answers for one request.
pub(crate) enum LandingPage {
    /// Render the page, splicing this markup into the head and sending these
    /// headers. Both are empty when no project answers to the shortcode.
    Render(Markup, HeaderMap),
    /// Answer `303 See Other` to this representation.
    Redirect(HeaderValue),
}

/// The landing page's answer for one shortcode and one `Accept` header.
///
/// The `303` is the only thing on this route that varies by header: the page
/// itself is never *rendered* differently by one (ADR-0004), and ADR-0005
/// carves out exactly this redirect.
///
/// An unresolved project gets nothing: no JSON-LD, no `Link` header, and no
/// redirect for any `Accept` value whatsoever — there is no layout, so there
/// are structurally no candidates to redirect to. The always-200 "Project Not
/// Found" body it already renders is left exactly as it is.
pub(crate) fn landing_page(shortcode: &str, accept: Option<&str>, state: &AppState) -> LandingPage {
    let Some(raw) = dpe_core::project_cache::project_raw_by_shortcode(shortcode) else {
        return LandingPage::Render(html! {}, HeaderMap::new());
    };

    // The candidates are `UrlLayout::candidates`, built from the same rows of
    // [`REPRESENTATIONS`] as the representation `describedby` links, so the page
    // cannot redirect somewhere it does not link. The OAI-record `describedby`
    // links are not candidates and are never redirect targets.
    let urls = url_layout(raw, state);
    if let Decision::Redirect(url) = decide(accept, &urls.candidates()) {
        // The same failure `link_header` handles, answered the same way: a
        // configured base URL holding a control character makes the HTTP layer
        // refuse the value. Warn and serve the page, rather than panicking
        // inside `Redirect::to` on the way out.
        match HeaderValue::from_str(&url) {
            Ok(location) => return LandingPage::Redirect(location),
            Err(error) => {
                tracing::warn!(
                    shortcode = %raw.shortcode,
                    %error,
                    "redirect target rejected by the HTTP layer and dropped"
                );
            }
        }
    }

    // The canonical shortcode, not the path segment, so the record lookup and
    // the URLs cannot be steered by how the visitor spelled it.
    let records = dpe_core::record_cache::records_for_shortcode(&raw.shortcode);
    let (markup, headers) = render(raw, records.iter().copied().take(HAS_PART_CAP), state);
    LandingPage::Render(markup, headers)
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
        SchemaOrgOptions { parts: PartLimit::Count(HAS_PART_CAP) },
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
/// Bounded by [`JSON_LD_BYTE_BUDGET`] rather than by the page's count, so it
/// describes as many records as fit and stays a document a consumer can parse.
/// Still a few megabytes, which is why this route sits behind the per-IP
/// limiter from its first day (`router.rs`).
pub(crate) async fn project_json_ld_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    representation(&id, &state, JSON_LD, |raw, urls| {
        let records = dpe_core::record_cache::records_for_shortcode(&raw.shortcode);
        project_json_ld(raw, records.iter().copied(), urls)
    })
    .await
}

/// The JSON-LD representation's document, for one project over the records the
/// caller materialised.
///
/// Named and separate from the handler so a test can serve it the committed
/// corpus without the record cache, which is a process-global keyed on
/// `DPE_DATA_DIR`. It is the function the handler runs, not a lookalike: a
/// budget that stopped being applied has to fail the corpus test too.
fn project_json_ld<'a>(
    raw: &ProjectRaw,
    records: impl IntoIterator<Item = &'a Record>,
    urls: &UrlLayout,
) -> serde_json::Value {
    let graph = build_graph(raw, records);
    project_to_schema_org(&graph, urls, SchemaOrgOptions { parts: PartLimit::Bytes(JSON_LD_BYTE_BUDGET) })
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

    /// **The class, not the instance.** Every byte DPE renders for a committed
    /// project, swept for a production ARK host.
    ///
    /// The sidebar permalink was found by reading the code, which is how the
    /// `sameAs` near-miss was nearly missed: a reader checks the places they
    /// think of. This renders instead — the metadata head, the `Link` header,
    /// the sidebar a person reads, and the JSON API's document — for all 85
    /// committed projects, from a corpus normalised by the **real** ingress
    /// function, so a renderer nobody thought of fails here rather than in a
    /// preview.
    ///
    /// Renderers rather than the router, deliberately: the resolver is a
    /// process-global read when the caches load, so a test cannot serve one
    /// request with it set and another without. `dpe-core`'s
    /// `project_cache::ingress_tests` proves the loader normalises; this proves
    /// that nothing downstream of the loader reintroduces the recorded host.
    mod served_bytes {
        use dpe_core::Project;
        use shared_metadata::ProjectRaw;

        use super::*;

        const PREVIEW: &str = "https://dpe-pr-391-pbjdzenira-oa.a.run.app";
        const RECORDED_ARK_HOST: &str = "ark.dasch.swiss";
        const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/projects");

        /// The committed corpus as the caches would hold it on a deployment
        /// configured with `resolver`.
        fn corpus(resolver: Option<&str>) -> Vec<ProjectRaw> {
            let mut raws: Vec<ProjectRaw> = std::fs::read_dir(COMMITTED)
                .expect("the committed corpus should be readable")
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
                .map(|path| {
                    let json = std::fs::read_to_string(&path).expect("readable");
                    let mut raw: ProjectRaw = serde_json::from_str(&json).expect("parses");
                    // The real ingress rule, not a copy of it: a regression in
                    // `normalise_project` has to fail this sweep too.
                    dpe_core::ark::normalise_project(&mut raw, resolver);
                    raw
                })
                .collect();
            raws.sort_by(|a, b| a.shortcode.cmp(&b.shortcode));
            assert_eq!(raws.len(), 85, "the committed corpus");
            raws
        }

        /// Everything a reader receives for one project: the metadata head, the
        /// `Link` header, the sidebar, and the JSON API's document.
        fn rendered(raw: &ProjectRaw) -> String {
            let state = test_state();
            let (markup, headers) = render(raw, std::iter::empty(), &state);
            let mut out = markup.into_string();
            if let Some(link) = headers.get(header::LINK) {
                out.push_str(link.to_str().expect("ascii"));
            }
            out.push_str(
                &dpe_web::pages::project::components::project_sidebar::project_sidebar(&Project::from(raw.clone()))
                    .into_string(),
            );
            out.push_str(&serde_json::to_string(raw).expect("the wire contract should serialise"));
            out
        }

        /// The recorded strings an ARK may legitimately survive inside.
        ///
        /// Recorded text is *quoted*, not asserted, so it passes through
        /// verbatim — the same carve-out `dpe-api-oai`'s representation sweep
        /// states, for the same reason (ADR-0005, *Nothing is invented for a
        /// score*). Two fields reach a reader: `howToCite`, whose author may
        /// have written the ARK into the sentence, and `url`, the project's
        /// website, which 083D records as its own ARK.
        fn quoted(raw: &ProjectRaw) -> Vec<String> {
            let (website, secondary) = shared_metadata::utils::parse_url_value(raw.url.clone());
            std::iter::once(raw.how_to_cite.clone())
                .chain([website, secondary].into_iter().flatten().map(|site| site.url))
                .filter(|text| text.contains(RECORDED_ARK_HOST))
                .collect()
        }

        /// Maud escapes `&`, and five committed citations carry one ("Nagl, F.
        /// & Gehr, S."), so matching the literal alone would leave their quoted
        /// ARK looking like an asserted one.
        fn html_escaped(text: &str) -> String {
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        }

        fn asserted_only(rendered: &str, raw: &ProjectRaw) -> String {
            let mut out = rendered.to_string();
            for text in quoted(raw) {
                out = out.replace(&text, "[QUOTED]");
                out = out.replace(&html_escaped(&text), "[QUOTED]");
            }
            out
        }

        #[test]
        fn a_configured_resolver_leaves_no_production_ark_in_the_rendered_bytes() {
            let mut substituted = 0usize;
            for raw in corpus(Some(PREVIEW)) {
                let bytes = rendered(&raw);
                assert!(
                    !asserted_only(&bytes, &raw).contains(RECORDED_ARK_HOST),
                    "{}: a production ARK host survives outside quoted text",
                    raw.shortcode
                );
                if bytes.contains(&format!("{PREVIEW}/ark:/")) {
                    substituted += 1;
                }
            }
            // Without this the assertion above would pass on output carrying no
            // ARK at all.
            assert_eq!(substituted, 85, "every project should render a substituted ARK");
        }

        #[test]
        fn with_no_resolver_configured_the_rendered_bytes_carry_the_recorded_ark() {
            for raw in corpus(None) {
                let bytes = rendered(&raw);
                assert!(!bytes.contains("run.app"), "{}: a preview host leaked", raw.shortcode);
                assert!(
                    bytes.contains(&format!("https://{RECORDED_ARK_HOST}/ark:/")),
                    "{}: the recorded ARK should be there",
                    raw.shortcode
                );
            }
        }

        /// The sidebar specifically, since it is what prompted the sweep, and
        /// because it is the proof that the placement is right: it was not
        /// changed at all, and it is correct because the view model it renders
        /// was normalised on the way in.
        #[test]
        fn the_sidebar_permalink_resolves_on_this_deployment() {
            let raw = corpus(Some(PREVIEW))
                .into_iter()
                .find(|raw| raw.shortcode == "0803")
                .expect("0803 is committed");
            let html = dpe_web::pages::project::components::project_sidebar::project_sidebar(&Project::from(raw))
                .into_string();
            assert!(html.contains(&format!(r#"href="{PREVIEW}/ark:/72163/1/0803""#)), "href: {html}");
            assert!(
                html.contains(&format!(r#"data-copy-text="{PREVIEW}/ark:/72163/1/0803""#)),
                "copy text: {html}"
            );
            // The displayed text is the bare identifier either way.
            assert!(html.contains(">ark:/72163/1/0803<"), "display: {html}");
        }
    }

    /// **The bytes `/metadata.jsonld` serves, for every committed project that
    /// has records.**
    ///
    /// Byte-measured and parsed, not counted: what broke was that 5,251,715
    /// bytes of valid JSON arrived at an assessor that stops reading at
    /// 5,000,000, and no assertion about field values or entry counts can see
    /// that. The same instrument as the `served_bytes` sweep above, pointed at
    /// a different risk.
    ///
    /// It runs [`project_json_ld`] — the function the handler runs — over the
    /// committed dumps read from disk, rather than through the router: the
    /// record cache is a process-global keyed on `DPE_DATA_DIR`, and reading
    /// the files is what lets one test cover every project.
    mod byte_budget {
        use super::*;

        const PROJECTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/projects");
        const RECORDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/records");

        /// Every committed project that has a record dump, with the whole dump.
        ///
        /// The whole dump, not a sample: a budget tested against 100 records is
        /// a budget that was never reached.
        fn with_dumps() -> Vec<(ProjectRaw, Vec<Record>)> {
            let mut out: Vec<(ProjectRaw, Vec<Record>)> = std::fs::read_dir(PROJECTS)
                .expect("the committed corpus should be readable")
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
                .filter_map(|path| {
                    let raw: ProjectRaw =
                        serde_json::from_str(&std::fs::read_to_string(&path).expect("readable")).expect("parses");
                    let dump = std::path::Path::new(RECORDS).join(format!("{}-records.json", raw.shortcode));
                    let records: Vec<Record> = match std::fs::read_to_string(&dump) {
                        Ok(json) => serde_json::from_str(&json).expect("a dump should parse"),
                        Err(_) => return None,
                    };
                    Some((raw, records))
                })
                .collect();
            out.sort_by(|(a, _), (b, _)| a.shortcode.cmp(&b.shortcode));
            assert_eq!(out.len(), 3, "the committed record dumps");
            out
        }

        /// What the route writes to the socket for one project: the same
        /// document, through the same serialiser call as `representation`.
        fn served(raw: &ProjectRaw, records: &[Record]) -> String {
            let state = test_state();
            serde_json::to_string(&project_json_ld(raw, records.iter(), &url_layout(raw, &state)))
                .expect("a Value should serialise")
        }

        #[test]
        fn every_committed_project_is_served_under_the_budget_as_valid_json() {
            for (raw, records) in with_dumps() {
                let body = served(&raw, &records);
                assert!(
                    body.len() <= JSON_LD_BYTE_BUDGET,
                    "{}: {} bytes, over the {JSON_LD_BYTE_BUDGET}-byte budget",
                    raw.shortcode,
                    body.len()
                );
                serde_json::from_str::<serde_json::Value>(&body)
                    .unwrap_or_else(|e| panic!("{}: the served document should parse: {e}", raw.shortcode));
            }
            // 0862 has no dump, so it is not in the loop above; it is the
            // project the assertion below proves the budget leaves alone.
            let raw = serde_json::from_str::<ProjectRaw>(
                &std::fs::read_to_string(std::path::Path::new(PROJECTS).join("0862_gotthelf.json")).expect("readable"),
            )
            .expect("parses");
            serde_json::from_str::<serde_json::Value>(&served(&raw, &[])).expect("0862 should parse");
        }

        /// **The budget has to actually bind somewhere, or the test above
        /// proves nothing.** 0868 is the project whose document F-UJI could not
        /// parse, so it is the one that must come back short; 0862 records no
        /// parts at all, so it must come back whole.
        #[test]
        fn the_budget_binds_on_0868_and_not_on_0862() {
            let corpus = with_dumps();
            let (raw, records) = corpus
                .iter()
                .find(|(raw, _)| raw.shortcode == "0868")
                .expect("0868 is committed with a dump");

            let state = test_state();
            let urls = url_layout(raw, &state);
            let graph = build_graph(raw, records.iter());
            let unbounded =
                serde_json::to_string(&project_to_schema_org(&graph, &urls, SchemaOrgOptions::default())).unwrap();
            assert!(
                unbounded.len() > JSON_LD_BYTE_BUDGET,
                "0868 no longer exceeds the budget unbounded ({} bytes), so this measures nothing",
                unbounded.len()
            );

            let bounded = served(raw, records);
            assert!(bounded.len() < unbounded.len(), "0868's document should have been bounded");
            let parts = serde_json::from_str::<serde_json::Value>(&bounded).expect("valid JSON")["hasPart"]
                .as_array()
                .expect("an array")
                .len();
            assert!(parts < graph.parts.len(), "0868 lists {parts} of {}", graph.parts.len());

            // 0862 carries no records at all, so nothing about it is bounded.
            let raw = serde_json::from_str::<ProjectRaw>(
                &std::fs::read_to_string(std::path::Path::new(PROJECTS).join("0862_gotthelf.json")).expect("readable"),
            )
            .expect("parses");
            let urls = url_layout(&raw, &state);
            let whole = serde_json::to_string(&project_to_schema_org(
                &build_graph(&raw, std::iter::empty()),
                &urls,
                SchemaOrgOptions::default(),
            ))
            .unwrap();
            assert_eq!(served(&raw, &[]), whole, "0862 should be served whole");
        }
    }

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

    // --- the one negotiation step ---

    async fn request(method: &str, uri: &str, accept: Option<&str>) -> (StatusCode, HeaderMap, String) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(accept) = accept {
            builder = builder.header(header::ACCEPT, accept);
        }
        let response = corpus_app()
            .oneshot(builder.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.expect("a body").to_bytes();
        (status, headers, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn location(headers: &HeaderMap) -> &str {
        headers.get(header::LOCATION).expect("a Location").to_str().expect("ascii")
    }

    fn vary(headers: &HeaderMap) -> Option<&str> {
        headers.get(header::VARY).map(|value| value.to_str().expect("ascii"))
    }

    #[tokio::test]
    async fn a_harvester_asking_for_a_representation_is_redirected_to_it() {
        // The table, not a copy of it: a new row must be negotiable the moment
        // it is linked, which is the promise `REPRESENTATIONS` makes.
        for (accept, suffix) in REPRESENTATIONS {
            let (status, headers, body) = request("GET", "/dpe/projects/0862", Some(accept)).await;
            assert_eq!(status, StatusCode::SEE_OTHER, "{accept}");
            assert_eq!(
                location(&headers),
                format!("https://example.test/dpe/projects/0862/{suffix}"),
                "{accept}"
            );
            assert!(body.is_empty(), "{accept}: {body}");
        }
    }

    #[tokio::test]
    async fn a_browser_gets_the_page() {
        for accept in [None, Some("text/html,application/xhtml+xml,*/*;q=0.8"), Some("*/*")] {
            let (status, _, body) = request("GET", "/dpe/projects/0862", accept).await;
            assert_eq!(status, StatusCode::OK, "{accept:?}");
            assert!(body.contains(r#"<script type="application/ld+json">"#), "{accept:?}");
        }
    }

    /// Caches must not replay a 303 to a person or the HTML to a harvester, so
    /// every answer from this route says what it varies on.
    #[tokio::test]
    async fn every_landing_page_answer_varies_on_accept() {
        for method in ["GET", "HEAD"] {
            for accept in [None, Some("text/html"), Some("application/ld+json")] {
                let (_, headers, _) = request(method, "/dpe/projects/0862", accept).await;
                assert_eq!(vary(&headers), Some("Accept"), "{method} {accept:?}");
            }
            // Including the page that resolves to no project at all.
            let (_, headers, _) = request(method, "/dpe/projects/zzzz", None).await;
            assert_eq!(vary(&headers), Some("Accept"), "{method} unknown shortcode");
        }
    }

    #[tokio::test]
    async fn the_redirect_target_is_the_canonical_shortcode() {
        let (_, lower, _) = request("GET", "/dpe/projects/081c", Some("application/ld+json")).await;
        let (_, upper, _) = request("GET", "/dpe/projects/081C", Some("application/ld+json")).await;
        assert_eq!(location(&lower), location(&upper));
        assert_eq!(location(&lower), "https://example.test/dpe/projects/081C/metadata.jsonld");
    }

    /// The redirect target is assembled from configuration, not from the
    /// request, so a refusal here means an operator value the HTTP layer will
    /// not carry. Same answer as a refused `Link` header: warn, drop, serve the
    /// page — never a panic on the way out.
    #[test]
    fn a_redirect_target_the_http_layer_refuses_falls_back_to_the_page() {
        let mut state = test_state();
        state.public_base_url = "https://example.test\r\nX-Injected: 1".to_string();
        let answer = landing_page("0862", Some("application/ld+json"), &state);
        assert!(matches!(answer, LandingPage::Render(..)));
    }

    /// No project, no layout, so structurally no candidate to redirect to. The
    /// route never produces a 4xx from `Accept` either.
    #[tokio::test]
    async fn an_unknown_shortcode_never_redirects_whatever_it_was_asked_for() {
        for accept in [
            None,
            Some("application/ld+json"),
            Some("text/html"),
            Some("application/vnd.datacite.datacite+json"),
        ] {
            let (status, headers, body) = request("GET", "/dpe/projects/zzzz", accept).await;
            assert_eq!(status, StatusCode::OK, "{accept:?}");
            assert!(headers.get(header::LOCATION).is_none(), "{accept:?}: {headers:?}");
            assert!(headers.get(header::LINK).is_none(), "{accept:?}: {headers:?}");
            assert!(!body.contains("application/ld+json"), "{accept:?}");
        }
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
