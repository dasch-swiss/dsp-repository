//! CPE mounted at a base: nothing it answers leaves the mount.
//!
//! The subject is Incunabula as the area binary serves it, activated once
//! with `CPE_BASE=/cpe` and a configured public origin. Its narrative links
//! start with `/incunabula/…`, it has a cover image, and its book page is a
//! compound reader, so every kind of URL CPE writes is on its pages.
//!
//! A request is made as a host that nests the router makes it: the base is
//! removed from the path first, as `axum::Router::nest` does.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use sync_store::LiveArchiveProjection;
use tower::ServiceExt;

use crate::{Cpe, CpeConfig};

const BASE: &str = "/cpe";
const ORIGIN: &str = "https://repository.example";

fn config() -> CpeConfig {
    CpeConfig {
        projects_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("../projects"),
        public_origin: Some(ORIGIN.to_owned()),
        base: BASE.to_owned(),
        ..CpeConfig::default()
    }
}

fn sync_data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sync/data")
}

/// Incunabula under `/cpe`, activated once for every test of this module.
fn cpe() -> &'static Cpe {
    static CPE: OnceLock<Cpe> = OnceLock::new();
    CPE.get_or_init(|| {
        let cpe = Cpe::new(&config());
        cpe.activate(&LiveArchiveProjection::new(sync_data()));
        assert_eq!(cpe.projects().len(), 1, "Incunabula activates from the committed data");
        cpe
    })
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Answer {
    fn header(&self, name: header::HeaderName) -> Option<&str> {
        self.headers.get(name).map(|value| value.to_str().expect("the header is ASCII"))
    }

    fn cookies(&self) -> Vec<&str> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().expect("the cookie is ASCII"))
            .collect()
    }
}

/// The answer to the public URL `url`, which must be under the base. With
/// `datastar`, as Datastar's `@get` asks.
async fn answer(cpe: &Cpe, url: &str, datastar: bool) -> Answer {
    let below = url
        .strip_prefix(BASE)
        .filter(|rest| rest.is_empty() || rest.starts_with(['/', '?']))
        .unwrap_or_else(|| panic!("{url} is not under {BASE}"));
    let below = if below.is_empty() || below.starts_with('?') {
        format!("/{below}")
    } else {
        below.to_owned()
    };
    let mut request = Request::builder().uri(&below);
    if datastar {
        request = request.header("Datastar-Request", "true");
    }
    let response = cpe
        .router()
        .oneshot(
            request
                .body(Body::empty())
                .expect("a URL found in a page is a valid request target"),
        )
        .await
        .expect("oneshot against an in-process router never fails");
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX).await.expect("the body is finite");
    Answer {
        status: parts.status,
        headers: parts.headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}

/// What ends a URL where a page writes one: a quote, a bracket, white space.
fn ends_url(c: char) -> bool {
    matches!(c, '"' | '\'' | '`' | ')' | '<' | '>' | '\\') || c.is_whitespace()
}

/// Every root-absolute path and every URL of the public origin that `text`
/// holds, wherever it stands: an `href`, `src` or `action`, a Datastar
/// `@get('…')`, a CSS `url(…)`, a script's string.
///
/// A path is taken where a `/` follows a quote, an opening bracket or a `=`
/// (an unquoted attribute). That finds more than attributes on purpose: the
/// reader's history URL is a string in a script.
fn urls_in(text: &str) -> Vec<String> {
    let text = text
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&");
    let mut found = Vec::new();
    let mut previous = ' ';
    for (at, c) in text.char_indices() {
        let opens = matches!(previous, '"' | '\'' | '`' | '(' | '=');
        if opens && (c == '/' || text[at..].starts_with(ORIGIN)) {
            let end = text[at..].find(ends_url).map_or(text.len(), |length| at + length);
            found.push(text[at..end].to_owned());
        }
        previous = c;
    }
    found
}

/// Why `url` leaves the mount, if it does. A path that holds the base twice
/// is under the base and wrong all the same: no project here is named `cpe`.
fn leak(url: &str) -> Option<&'static str> {
    let path = match url.strip_prefix(ORIGIN) {
        Some(path) => path,
        None => url,
    };
    if path.starts_with("//") {
        return Some("names another host");
    }
    match path.strip_prefix(BASE) {
        Some(rest)
            if rest
                .strip_prefix(BASE)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/')) =>
        {
            Some("the base is written twice")
        }
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '?', '#']) => None,
        _ if url.starts_with(ORIGIN) => Some("the origin is not followed by the base"),
        _ => Some("a root-absolute path outside the base"),
    }
}

/// The `url(…)` values of a stylesheet. A stylesheet has other strings that
/// start with `/` (a `content: "/"`), and none of them is a URL.
fn urls_in_css(css: &str) -> Vec<String> {
    css.split("url(")
        .skip(1)
        .map(|rest| rest.trim_start_matches(['"', '\'']))
        .map(|rest| rest[..rest.find(ends_url).unwrap_or(rest.len())].to_owned())
        .filter(|url| url.starts_with('/') || url.starts_with(ORIGIN))
        .collect()
}

/// Fail on any URL of `answer` that leaves the mount; return those that stay.
fn urls_under_the_base(what: &str, answer: &Answer) -> Vec<String> {
    let stylesheet = answer
        .header(header::CONTENT_TYPE)
        .is_some_and(|kind| kind.starts_with("text/css"));
    let mut urls = if stylesheet {
        urls_in_css(&answer.body)
    } else {
        urls_in(&answer.body)
    };
    if let Some(location) = answer.header(header::LOCATION) {
        assert!(location.starts_with('/'), "{what}: Location {location:?} is not a path");
        urls.push(location.to_owned());
    }
    for url in &urls {
        if let Some(why) = leak(url) {
            panic!("{what}: {url:?} leaves {BASE} ({why})");
        }
    }
    urls
}

/// The scanner finds what it is meant to find, and `leak` refuses it.
#[test]
fn the_scanner_sees_every_place_a_url_stands() {
    let page = concat!(
        r#"<a href="/x/a">a</a><img src='/x/b'><form action=/x/c>"#,
        r#"<div data-on:click="@get(&#39;/x/d?q=1&amp;r=2&#39;)" style="background: url(/x/e)">"#,
        r#"<script>history.pushState({}, '', '/x/f'); fetch(`/x/g`)</script>"#,
        r#"<a href="https://repository.example/x/h">h</a><a href="//evil.example/i">i</a>"#,
        r#"<a href="/cpe/ok">ok</a><a href="https://repository.example/cpe/ok">ok</a><a href="/cpe">ok</a>"#,
        r##"<a href="https://elsewhere.example/x">x</a><a href="#top">x</a><a href="rel/x">x</a></div>"##,
    );
    let urls = urls_in(page);
    let leaks: Vec<&str> = urls.iter().map(String::as_str).filter(|url| leak(url).is_some()).collect();
    assert_eq!(
        leaks,
        [
            "/x/a",
            "/x/b",
            "/x/c",
            "/x/d?q=1&r=2",
            "/x/e",
            "/x/f",
            "/x/g",
            "https://repository.example/x/h",
            "//evil.example/i"
        ]
    );
    let kept: Vec<&str> = urls.iter().map(String::as_str).filter(|url| leak(url).is_none()).collect();
    assert_eq!(kept, ["/cpe/ok", "https://repository.example/cpe/ok", "/cpe"]);
    assert!(leak("/cpex/a").is_some() && leak("https://repository.example.evil/cpe/a").is_some());
    for twice in [
        "/cpe/cpe",
        "/cpe/cpe/incunabula/",
        "https://repository.example/cpe/cpe/incunabula/books",
    ] {
        assert_eq!(leak(twice), Some("the base is written twice"), "{twice}");
    }
    assert_eq!(leak("/cpe/cpelike/x"), None, "a first segment that only starts like the base");

    let css = r#"a { background: url("/x/a") } b { mask: url(/x/b) } c { content: "/"; src: url(rel.svg) }"#;
    assert_eq!(urls_in_css(css), ["/x/a", "/x/b"]);
}

/// A crawl key: the URLs of one kind share it, and a few of each kind are
/// enough. The id of a record and the value of a query pair are left out.
fn kind(url: &str) -> String {
    let url = url.strip_prefix(ORIGIN).unwrap_or(url);
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let segments: Vec<&str> = path.split('/').collect();
    let shape: Vec<&str> = segments
        .iter()
        .enumerate()
        .map(|(at, segment)| {
            let fixed = at <= 3 || segment.starts_with('_') || segments.contains(&"_static");
            if fixed {
                segment
            } else {
                "*"
            }
        })
        .collect();
    let keys: BTreeSet<&str> = query.split('&').filter_map(|pair| pair.split('=').next()).collect();
    format!("{}?{}", shape.join("/"), keys.into_iter().collect::<Vec<_>>().join("&"))
}

/// What a crawl read.
#[derive(Default)]
struct Crawl {
    /// The URLs requested.
    seen: BTreeSet<String>,
    /// Every URL found in an answer.
    urls: BTreeSet<String>,
    /// The fragment segments answered as an event stream.
    fragments: BTreeSet<String>,
    /// Answers by status and content type.
    answered: BTreeMap<String, usize>,
    /// The URLs answered 404.
    dead: BTreeSet<String>,
}

/// Follow every URL found from `starts` on, a few of each kind, and fail on
/// one that leaves the mount.
async fn crawl(cpe: &Cpe, starts: &[String]) -> Crawl {
    const PER_KIND: usize = 4;
    let mut queue: VecDeque<String> = starts.iter().cloned().collect();
    let mut read = Crawl { seen: queue.iter().cloned().collect(), ..Crawl::default() };
    let mut per_kind: BTreeMap<String, usize> = BTreeMap::new();

    while let Some(url) = queue.pop_front() {
        let last = url.split('?').next().and_then(|path| path.rsplit('/').next()).unwrap_or("");
        let datastar = ["_facets", "_sort", "_search", "_compound"].contains(&last);
        let got = answer(cpe, &url, datastar).await;
        assert!(
            got.status.is_success() || got.status.is_redirection() || got.status == StatusCode::NOT_FOUND,
            "{url}: {}",
            got.status
        );
        if got.status == StatusCode::NOT_FOUND {
            read.dead.insert(url.clone());
        }
        let content_type = got.header(header::CONTENT_TYPE).unwrap_or("").to_owned();
        *read
            .answered
            .entry(format!("{} {content_type}", got.status.as_u16()))
            .or_default() += 1;
        if content_type.starts_with("text/event-stream") {
            assert!(datastar && got.status == StatusCode::OK, "{url}");
            read.fragments.insert(last.to_owned());
        }
        let readable = ["text/html", "text/event-stream", "text/css"]
            .iter()
            .any(|t| content_type.starts_with(t));
        if !readable && !got.status.is_redirection() {
            continue;
        }
        for found in urls_under_the_base(&url, &got) {
            read.urls.insert(found.clone());
            // A URL that a script completes (`…/_search?q=` and the input) is asked for as far as
            // it is written.
            let next = found.strip_prefix(ORIGIN).unwrap_or(&found);
            let next = next.split(['#', '$', '{']).next().unwrap_or(next).to_owned();
            if !read.seen.insert(next.clone()) {
                continue;
            }
            let count = per_kind.entry(kind(&next)).or_default();
            if *count < PER_KIND {
                *count += 1;
                queue.push_back(next);
            }
        }
    }
    read
}

/// Every page and fragment that a crawl from the overview and the start pages
/// reaches, a few of each kind, with both not-found pages and the 503. Incunabula has the
/// narrative, the reader and the sort; the fixture projects have the facet
/// panel and the search bar.
#[tokio::test]
async fn no_url_leaves_the_base() {
    let cpe = cpe();
    let read = crawl(
        cpe,
        &[
            BASE.to_owned(),
            format!("{BASE}/"),
            format!("{BASE}/incunabula/"),
            format!("{BASE}/incunabula"),
        ],
    )
    .await;

    let fixtures = Cpe::from_sql_folders(&CpeConfig { projects_dir: crate::tests::fixture(), ..config() });
    // The fixture projects have no navigation: their pages are named here.
    let mut starts: Vec<String> = fixtures.projects().into_iter().map(|project| project.url).collect();
    assert!(
        starts.len() >= 6 && starts.iter().all(|url| url.starts_with("/cpe/")),
        "{starts:?}"
    );
    for page in [
        "eta/widgets",
        "theta/widgets",
        "zeta/things",
        "epsilon/featured",
        "delta/characters/alice",
    ] {
        starts.push(format!("{BASE}/{page}"));
    }
    let read_fixtures = crawl(&fixtures, &starts).await;
    assert!(
        read_fixtures.seen.len() > 30,
        "the crawl stopped early: {} URLs",
        read_fixtures.seen.len()
    );

    let dead: Vec<&String> = read.dead.iter().chain(&read_fixtures.dead).collect();
    assert!(dead.is_empty(), "a link of a page is dead: {dead:?}");

    // What the crawls must have read for their silence to mean anything.
    let has = |url: &str| read.urls.contains(url);
    assert!(read.seen.len() > 60, "the crawl stopped early: {} URLs", read.seen.len());
    assert_eq!(
        read.fragments.iter().map(String::as_str).collect::<Vec<_>>(),
        ["_compound", "_sort"],
        "Incunabula's fragments are answered as event streams"
    );
    assert_eq!(
        read_fixtures.fragments.iter().map(String::as_str).collect::<Vec<_>>(),
        ["_facets", "_search", "_sort"],
        "the fixtures' fragments are answered as event streams"
    );
    assert!(
        has("/cpe/incunabula/narrenschiff"),
        "a narrative link written `/incunabula/narrenschiff`"
    );
    assert!(has("/cpe/incunabula/_static/cpe/tokens.css"), "a project page's stylesheet");
    assert!(
        has("/cpe/incunabula/_static/image-viewer/image-viewer.js"),
        "the viewer's module"
    );
    assert!(has("/cpe/"), "the CPE crumb");
    assert!(has("/cpe/incunabula/_assets/assets/cover.webp"), "the cover on the overview");
    assert!(has("https://repository.example/cpe/"), "the overview's share link");
    assert!(
        read.urls
            .iter()
            .any(|url| url.starts_with("https://repository.example/cpe/incunabula/books/")),
        "a share link of a book"
    );
    assert!(
        read.urls
            .iter()
            .any(|url| url.starts_with("/cpe/incunabula/books/") && url.contains("?page=")),
        "a reader link"
    );
    assert!(
        read.answered.keys().any(|key| key.starts_with("200 text/css")),
        "{:?}",
        read.answered
    );
    assert!(
        read.answered.keys().any(|key| key.starts_with("200 text/javascript")),
        "{:?}",
        read.answered
    );

    // The pages without a link to them.
    let site_404 = answer(cpe, "/cpe/no-such-project/x", false).await;
    assert_eq!(site_404.status, StatusCode::NOT_FOUND);
    let urls = urls_under_the_base("CPE's not-found", &site_404);
    assert!(urls.contains(&"/cpe/_static/cpe/tokens.css".to_owned()), "{urls:?}");
    assert!(urls.contains(&"/cpe/?appearance=dark".to_owned()), "{urls:?}");

    let project_404 = answer(cpe, "/cpe/incunabula/no-such-page", false).await;
    assert_eq!(project_404.status, StatusCode::NOT_FOUND);
    let urls = urls_under_the_base("the project's not-found", &project_404);
    assert!(urls.contains(&"/cpe/incunabula/_static/cpe/tokens.css".to_owned()), "{urls:?}");
    assert!(urls.contains(&"/cpe/incunabula/books".to_owned()), "{urls:?}");

    let unknown_record = answer(cpe, "/cpe/incunabula/books/no-such-book", false).await;
    assert_eq!(unknown_record.status, StatusCode::NOT_FOUND);
    urls_under_the_base("an unknown record", &unknown_record);

    let not_ready = answer(&Cpe::new(&config()), "/cpe/incunabula/", false).await;
    assert_eq!(not_ready.status, StatusCode::SERVICE_UNAVAILABLE);
    let urls = urls_under_the_base("the 503", &not_ready);
    assert!(urls.is_empty() && references_no_url(&not_ready.body), "{}", not_ready.body);
}

/// Whether a page holds no attribute and no CSS function that names a URL.
fn references_no_url(page: &str) -> bool {
    !["href", "src", "url(", "@import", "action"]
        .iter()
        .any(|reference| page.contains(reference))
}

/// The files a page asks for are where it asks for them, for a project's
/// page and for a page without a project.
#[tokio::test]
async fn the_static_files_and_the_assets_are_served_under_the_base() {
    for url in [
        "/cpe/incunabula/_static/cpe/tokens.css",
        "/cpe/incunabula/_static/cpe/dasch-fingerprint.svg",
        "/cpe/incunabula/_static/datastar.js",
        "/cpe/incunabula/_static/image-viewer/image-viewer.js",
        "/cpe/incunabula/_static/image-viewer/vendor/openseadragon.min.js",
        "/cpe/incunabula/_assets/assets/cover.webp",
        "/cpe/_static/cpe/tokens.css",
        "/cpe/_static/datastar.js",
        "/cpe/_static/image-viewer/image-viewer.js",
    ] {
        assert_eq!(answer(cpe(), url, false).await.status, StatusCode::OK, "{url}");
    }
    for url in [
        "/cpe/_assets/incunabula/assets/cover.webp",
        "/cpe/_assets/assets/cover.webp",
        "/cpe/incunabula/_static/cpe/nope.css",
        "/cpe/incunabula/_assets/config.kdl",
        "/cpe/no-such-project/_static/cpe/tokens.css",
    ] {
        assert_eq!(answer(cpe(), url, false).await.status, StatusCode::NOT_FOUND, "{url}");
    }
}

/// The overview is the same page with and without the slash, and what it
/// writes is under the base: the project's link and cover, its own static
/// files and its share link.
#[tokio::test]
async fn the_overview_is_at_the_base_and_links_under_it() {
    let overview = answer(cpe(), "/cpe/", false).await;
    assert_eq!(overview.status, StatusCode::OK);
    assert_eq!(answer(cpe(), "/cpe", false).await.body, overview.body);
    let body = &overview.body;
    assert!(
        body.contains(r#"<a class="teaser" data-accent="ochre" href="/cpe/incunabula/">"#),
        "{body}"
    );
    assert!(
        body.contains(r#"<img class="teaser__img" src="/cpe/incunabula/_assets/assets/cover.webp" alt="">"#),
        "{body}"
    );
    assert!(body.contains(r#"<span class="teaser__path">/cpe/incunabula/</span>"#), "{body}");
    assert!(body.contains(r#"href="/cpe/_static/cpe/tokens.css""#), "{body}");
    assert!(body.contains(r#"src="/cpe/_static/cpe/dasch-logo-colour.svg""#), "{body}");
    assert!(
        body.contains(r#"href="https://repository.example/cpe/""#),
        "the share link: {body}"
    );
    assert!(
        body.contains(r#"href="/cpe/?appearance=dark""#),
        "the appearance toggle: {body}"
    );
    let urls = urls_under_the_base("the overview", &overview);
    assert!(urls.contains(&"/cpe/incunabula/".to_owned()), "{urls:?}");
}

/// A teaser's texts are chrome strings of the project, in the reader's
/// language where the project declares it.
#[tokio::test]
async fn the_overviews_teaser_is_in_the_readers_language() {
    let english = answer(cpe(), "/cpe/", false).await.body;
    assert!(english.contains("Early Basel prints · 1471–1504"), "{english}");
    assert!(english.contains("Nineteen illustrated early printed books"), "{english}");
    assert!(english.contains("Universitätsbibliothek Basel · CC BY 4.0"), "{english}");

    let german = answer(cpe(), "/cpe/?lang=de", false).await.body;
    assert!(german.contains("Basler Frühdrucke · 1471–1504"), "{german}");
    assert!(german.contains("Neunzehn illustrierte Basler Frühdrucke"), "{german}");

    // French is built in and not one of Incunabula's languages.
    let french = answer(cpe(), "/cpe/?lang=fr", false).await.body;
    assert!(french.contains(r#"<html lang="fr""#), "{french}");
    assert!(french.contains("Early Basel prints · 1471–1504"), "{french}");
    for body in [&english, &german, &french] {
        assert!(!body.contains("site-eyebrow") && !body.contains("site-description"), "{body}");
    }
}

#[tokio::test]
async fn both_redirects_stay_under_the_base() {
    for toggled in ["/cpe/?appearance=dark", "/cpe?appearance=dark"] {
        let overview = answer(cpe(), toggled, false).await;
        assert_eq!(overview.status, StatusCode::SEE_OTHER, "{toggled}");
        assert_eq!(overview.header(header::LOCATION), Some("/cpe/"), "{toggled}");
    }

    let stripped = answer(cpe(), "/cpe/incunabula/books?appearance=dark&sort=name", false).await;
    assert_eq!(stripped.status, StatusCode::SEE_OTHER);
    assert_eq!(stripped.header(header::LOCATION), Some("/cpe/incunabula/books?sort=name"));

    let bare = answer(cpe(), "/cpe/incunabula/?reset", false).await;
    assert_eq!(bare.status, StatusCode::SEE_OTHER);
    assert_eq!(bare.header(header::LOCATION), Some("/cpe/incunabula/"));

    let canonical = answer(cpe(), "/cpe/incunabula/strips?page=99999", false).await;
    assert_eq!(canonical.status, StatusCode::SEE_OTHER);
    let location = canonical.header(header::LOCATION).expect("a canonical redirect has a Location");
    assert!(location.starts_with("/cpe/incunabula/strips?page="), "{location}");
    let landed = answer(cpe(), location, false).await;
    assert_eq!(landed.status, StatusCode::OK, "{location}");
}

#[tokio::test]
async fn both_cookies_are_scoped_to_the_base() {
    let dark = answer(cpe(), "/cpe/incunabula/books?appearance=dark", false).await;
    assert_eq!(dark.cookies(), ["appearance=dark; Path=/cpe; SameSite=Lax"]);

    let light = answer(cpe(), "/cpe/incunabula/books?appearance=light", false).await;
    assert_eq!(light.cookies(), ["appearance=; Path=/cpe; Max-Age=0; SameSite=Lax"]);

    let german = answer(cpe(), "/cpe/incunabula/books?lang=de", false).await;
    assert_eq!(german.status, StatusCode::OK);
    assert_eq!(german.cookies(), ["lang=de; Path=/cpe; SameSite=Lax"]);

    let dark = answer(cpe(), "/cpe/?appearance=dark", false).await;
    assert_eq!(dark.cookies(), ["appearance=dark; Path=/cpe; SameSite=Lax"]);
    let german = answer(cpe(), "/cpe/?lang=de", false).await;
    assert_eq!(german.cookies(), ["lang=de; Path=/cpe; SameSite=Lax"]);
}

/// The fallback answers a path that names another host with a page: no
/// redirect, and nothing of the path in the answer.
#[tokio::test]
async fn a_path_that_names_another_host_is_not_echoed() {
    for url in ["/cpe//evil.example/x", "/cpe/incunabula//evil.example/x"] {
        let got = answer(cpe(), url, false).await;
        assert_eq!(got.status, StatusCode::NOT_FOUND, "{url}");
        assert!(got.header(header::LOCATION).is_none(), "{url}");
        assert!(!got.body.contains("evil.example"), "{url}");
        urls_under_the_base(url, &got);
    }
}

#[tokio::test]
async fn the_base_has_one_source() {
    assert_eq!(cpe().base(), "/cpe");
    assert_eq!(cpe().projects()[0].url, "/cpe/incunabula/");
    assert_eq!(
        Cpe::new(&CpeConfig::default()).base(),
        "",
        "a root mount unless a base is configured"
    );
}

/// A base that is refused is never written: CPE answers 503 everywhere,
/// activated or not, with a page that references no URL, and reports the
/// root as its base.
#[tokio::test]
async fn a_refused_base_is_answered_503_and_never_written() {
    let refused = CpeConfig { base: "cpe/".to_owned(), ..config() };
    let cpe = Cpe::new(&refused);
    cpe.activate(&LiveArchiveProjection::new(sync_data()));
    assert_eq!(cpe.base(), "");
    assert!(cpe.projects().is_empty());
    for uri in ["/", "/incunabula/", "/_static/cpe/tokens.css"] {
        let (status, headers, body) = crate::tests::get(&cpe, uri).await;
        crate::tests::assert_unavailable(status, &headers, uri);
        assert!(!body.contains("cpe/") && references_no_url(&body), "{body}");
    }
}

/// A base of two segments is written whole, once: the page's own links, both
/// cookies and the redirect of a fixture project under `/a/b`.
#[tokio::test]
async fn a_base_of_two_segments_is_written_whole() {
    let config = CpeConfig {
        projects_dir: crate::tests::fixture(),
        base: "/a/b".to_owned(),
        ..config()
    };
    let cpe = Cpe::from_sql_folders(&config);
    assert_eq!(cpe.base(), "/a/b");
    let cookies = |headers: &HeaderMap| -> Vec<String> {
        headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().expect("the cookie is ASCII").to_owned())
            .collect()
    };

    let (status, headers, _) = crate::tests::get(&cpe, "/eta/widgets?appearance=dark&sort=name").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers[header::LOCATION], "/a/b/eta/widgets?sort=name");
    assert_eq!(cookies(&headers), ["appearance=dark; Path=/a/b; SameSite=Lax"]);

    let (status, headers, _) = crate::tests::get(&cpe, "/eta/widgets?appearance=light").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers[header::LOCATION], "/a/b/eta/widgets");
    assert_eq!(cookies(&headers), ["appearance=; Path=/a/b; Max-Age=0; SameSite=Lax"]);

    let (status, headers, body) = crate::tests::get(&cpe, "/eta/widgets?lang=en").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cookies(&headers), ["lang=en; Path=/a/b; SameSite=Lax"]);
    assert!(body.contains(r#"href="/a/b/eta/_static/cpe/tokens.css""#), "{body}");
    assert!(body.contains(r#"href="/a/b/eta/""#), "the project crumb: {body}");
    assert!(
        body.contains("https://repository.example/a/b/eta/widgets"),
        "the share link: {body}"
    );
    assert!(!body.contains("/a/b/a/b"), "{body}");

    let (status, headers, _) = crate::tests::get(&cpe, "/?appearance=dark").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers[header::LOCATION], "/a/b/");
    let (status, _, overview) = crate::tests::get(&cpe, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(overview.contains(r#"href="/a/b/eta/""#), "{overview}");
    assert!(overview.contains(r#"src="/a/b/alpha/_assets/assets/cover.webp""#), "{overview}");
    assert!(overview.contains(r#"href="https://repository.example/a/b/""#), "{overview}");
    let body = format!("{body}{overview}");
    for path in body.split('"').filter(|text| text.starts_with('/') && !text.starts_with("//")) {
        assert!(path.starts_with("/a/b/"), "{path} leaves the base");
    }
}
