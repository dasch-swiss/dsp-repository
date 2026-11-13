//! What CPE reads from a request before a project handles it: the language
//! and appearance preferences, the query-string and cookie readers, and the
//! public origin that absolute links are built from.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

/// Resolved by the handler against the project's declared languages.
#[derive(Clone, Debug)]
pub(crate) struct RequestedLang {
    /// Ordered preference list (lowercased primary subtags, region stripped).
    pub prefs: Vec<String>,
    /// `true` when the preference came from an explicit `?lang=` query param.
    /// The handler sets `Set-Cookie` only in this case.
    pub from_query: bool,
}

/// `value: None` means no recognised appearance signal; the renderer then uses light. CPE does
/// not follow the OS `prefers-color-scheme`: light is the CSS baseline and the header toggle is
/// the only switch to dark.
#[derive(Clone, Debug)]
pub(crate) struct RequestedAppearance {
    pub value: Option<cpe_core::theme::Appearance>,
    /// `true` when the preference came from an explicit `?appearance=` query param; the handler
    /// manages the `appearance` cookie only in this case.
    pub from_query: bool,
}

pub(crate) fn normalize_lang(tag: &str) -> String {
    let primary = match tag.find('-') {
        Some(idx) => &tag[..idx],
        None => tag,
    };
    primary.trim().to_lowercase()
}

/// Stores the language and appearance preferences in the request's extensions. Never answers
/// 400: unknown or garbage values give `None`. Sets no cookie: a handler does, once it has
/// resolved the preference against its project.
pub(crate) async fn resolve_request_prefs(mut req: Request, next: Next) -> Response {
    let (prefs, lang_from_query) = extract_lang_prefs(req.headers(), req.uri().query());
    if let Some(top) = prefs.first() {
        tracing::Span::current().record("requested_lang", top.as_str());
    }
    req.extensions_mut()
        .insert(RequestedLang { prefs, from_query: lang_from_query });

    let appearance = extract_appearance(req.headers(), req.uri().query());
    req.extensions_mut().insert(appearance);

    next.run(req).await
}

pub(crate) async fn report_matched_route(
    State(base): State<cpe_core::url::Base>,
    req: Request,
    next: Next,
) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| crate::MatchedRoute(base.request_path(matched.as_str())));
    let mut response = next.run(req).await;
    if let Some(route) = route {
        response.extensions_mut().insert(route);
    }
    response
}

pub(crate) fn extract_lang_prefs(headers: &HeaderMap, query: Option<&str>) -> (Vec<String>, bool) {
    if let Some(lang_q) = query_lang(query) {
        if !lang_q.is_empty() {
            let normalized = normalize_lang(&lang_q);
            if !normalized.is_empty() {
                return (vec![normalized], true);
            }
        }
    }

    let mut prefs: Vec<String> = Vec::new();

    if let Some(cookie_val) = cookie_lang(headers) {
        let normalized = normalize_lang(&cookie_val);
        if !normalized.is_empty() {
            prefs.push(normalized);
        }
    }

    if let Some(al) = headers.get(axum::http::header::ACCEPT_LANGUAGE) {
        if let Ok(al_str) = al.to_str() {
            for entry in al_str.split(',') {
                let tag = match entry.find(';') {
                    Some(idx) => &entry[..idx],
                    None => entry,
                };
                let normalized = normalize_lang(tag);
                if !normalized.is_empty() {
                    prefs.push(normalized);
                }
            }
        }
    }

    (prefs, false)
}

pub(crate) fn extract_appearance(headers: &HeaderMap, query: Option<&str>) -> RequestedAppearance {
    if let Some(raw) = query_appearance(query) {
        let value = cpe_core::theme::Appearance::parse_config_str(&raw);
        return RequestedAppearance { value, from_query: true };
    }

    if let Some(raw) = cookie_appearance(headers) {
        let value = cpe_core::theme::Appearance::parse_config_str(&raw);
        return RequestedAppearance { value, from_query: false };
    }

    RequestedAppearance { value: None, from_query: false }
}

pub(crate) fn query_pairs(query: Option<&str>) -> impl Iterator<Item = (&str, &str)> {
    query.into_iter().flat_map(|q| q.split('&')).map(|pair| match pair.find('=') {
        Some(idx) => (&pair[..idx], &pair[idx + 1..]),
        None => (pair, ""),
    })
}

fn query_appearance(query: Option<&str>) -> Option<String> {
    query_pairs(query).find(|(k, _)| *k == "appearance").map(|(_, v)| v.to_owned())
}

fn cookie_appearance(headers: &HeaderMap) -> Option<String> {
    let cookie_hdr = headers.get(axum::http::header::COOKIE)?;
    let cookie_str = cookie_hdr.to_str().ok()?;
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("appearance=") {
            return Some(val.to_owned());
        }
    }
    None
}

fn query_lang(query: Option<&str>) -> Option<String> {
    // URL-decoding the minimal subset (%XX) is not needed for language codes
    // (ASCII letters only), so the raw value is returned as-is.
    query_pairs(query).find(|(k, _)| *k == "lang").map(|(_, v)| v.to_owned())
}

/// The 1-based `?img=` index, like `?member=` and the `.pg` pager; 1 when absent, zero or
/// unparseable. Callers clamp it against the media-item count.
pub(crate) fn query_img(query: Option<&str>) -> usize {
    query_pairs(query)
        .find(|(k, _)| *k == "img")
        .map(|(_, v)| v.parse::<usize>().unwrap_or(1))
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

/// The record viewer's thumb-strip fold on a full load: closed only for `?strip=0`. Folding it
/// later is the client-side `$stripOpen` signal, never a URL write
/// (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`).
pub(crate) fn query_strip(query: Option<&str>) -> bool {
    query_pairs(query)
        .find(|(k, _)| *k == "strip")
        .map(|(_, v)| v != "0")
        .unwrap_or(true)
}

fn cookie_lang(headers: &HeaderMap) -> Option<String> {
    let cookie_hdr = headers.get(axum::http::header::COOKIE)?;
    let cookie_str = cookie_hdr.to_str().ok()?;
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("lang=") {
            return Some(val.to_owned());
        }
    }
    None
}

/// Where a request's public origin comes from: the configured one, else each request's own
/// headers. One origin resolved at start would be wrong for a preview deployed under another
/// host with the same environment.
#[derive(Clone, Debug)]
pub(crate) struct OriginConfig {
    override_origin: Option<Arc<str>>,
}

/// The port in the origin of a request that carries no `Host` header. CPE
/// does not know the port its host listens on, and 80 makes that origin say
/// no more than `http://localhost`.
const NO_HOST_PORT: u16 = 80;

impl OriginConfig {
    pub(crate) fn new(public_origin: Option<&str>) -> Self {
        Self {
            override_origin: public_origin.filter(|origin| !origin.trim().is_empty()).map(Arc::from),
        }
    }

    #[cfg(test)]
    pub(crate) fn fixed(origin: &str) -> Self {
        Self::new(Some(origin))
    }

    pub(crate) fn resolve(&self, headers: &HeaderMap) -> String {
        match &self.override_origin {
            Some(origin) => origin.to_string(),
            None => derive_request_origin(headers, NO_HOST_PORT),
        }
    }
}

/// The origin of one request from its `X-Forwarded-Proto` and `Host`. Both are a client's to
/// send and the origin starts the share links' `href`: a scheme other than `https` is `http`,
/// and a `Host` that [`crate::config::is_origin_host`] refuses is treated as absent.
pub(crate) fn derive_request_origin(headers: &HeaderMap, port: u16) -> String {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|host| crate::config::is_origin_host(host));
    let Some(host) = host else {
        return format!("http://localhost:{port}");
    };
    let sent = headers.get("x-forwarded-proto").and_then(|value| value.to_str().ok());
    let scheme = if sent.is_some_and(|sent| sent.eq_ignore_ascii_case("https")) {
        "https"
    } else {
        "http"
    };
    format!("{scheme}://{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_headers() -> HeaderMap {
        HeaderMap::new()
    }

    fn headers_with_cookie(val: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::COOKIE, axum::http::HeaderValue::from_str(val).unwrap());
        h
    }

    fn headers_with_accept_lang(val: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            axum::http::HeaderValue::from_str(val).unwrap(),
        );
        h
    }

    #[test]
    fn normalize_strips_region_subtag() {
        assert_eq!(normalize_lang("en-US"), "en");
        assert_eq!(normalize_lang("de-AT"), "de");
    }

    #[test]
    fn normalize_lowercases() {
        assert_eq!(normalize_lang("DE"), "de");
        assert_eq!(normalize_lang("EN"), "en");
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_lang(" fr "), "fr");
    }

    #[test]
    fn normalize_plain_code_unchanged() {
        assert_eq!(normalize_lang("en"), "en");
        assert_eq!(normalize_lang("de"), "de");
    }

    #[test]
    fn query_lang_de_sets_from_query_true() {
        let (prefs, from_query) = extract_lang_prefs(&empty_headers(), Some("lang=de"));
        assert!(from_query, "?lang= present should set from_query=true");
        assert_eq!(prefs, vec!["de".to_owned()]);
    }

    #[test]
    fn query_lang_region_stripped() {
        let (prefs, from_query) = extract_lang_prefs(&empty_headers(), Some("lang=en-US"));
        assert!(from_query);
        assert_eq!(prefs, vec!["en".to_owned()]);
    }

    #[test]
    fn query_lang_empty_value_not_from_query() {
        let (prefs, from_query) = extract_lang_prefs(&empty_headers(), Some("lang="));
        assert!(
            !from_query,
            "empty ?lang= should NOT set from_query=true; got from_query={from_query}, prefs={prefs:?}"
        );
    }

    #[test]
    fn no_query_lang_not_from_query() {
        let (prefs, from_query) = extract_lang_prefs(&empty_headers(), None);
        assert!(!from_query);
        assert!(prefs.is_empty());
    }

    #[test]
    fn cookie_lang_used_when_no_query() {
        let headers = headers_with_cookie("lang=de");
        let (prefs, from_query) = extract_lang_prefs(&headers, None);
        assert!(!from_query);
        assert_eq!(
            prefs.first().map(String::as_str),
            Some("de"),
            "cookie lang=de should appear first in prefs"
        );
    }

    #[test]
    fn cookie_lang_with_other_cookies() {
        let headers = headers_with_cookie("session=abc; lang=de; foo=bar");
        let (prefs, _) = extract_lang_prefs(&headers, None);
        assert_eq!(
            prefs.first().map(String::as_str),
            Some("de"),
            "lang cookie found among other cookies"
        );
    }

    #[test]
    fn accept_language_source_order_region_stripped() {
        let headers = headers_with_accept_lang("en-US,de;q=0.9");
        let (prefs, from_query) = extract_lang_prefs(&headers, None);
        assert!(!from_query);
        assert_eq!(
            prefs,
            vec!["en".to_owned(), "de".to_owned()],
            "Accept-Language should be source-order region-stripped"
        );
    }

    #[test]
    fn accept_language_appended_after_cookie() {
        let mut headers = headers_with_cookie("lang=de");
        headers.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            axum::http::HeaderValue::from_str("en-US,fr;q=0.8").unwrap(),
        );
        let (prefs, _) = extract_lang_prefs(&headers, None);
        assert_eq!(
            prefs,
            vec!["de".to_owned(), "en".to_owned(), "fr".to_owned()],
            "cookie first, then Accept-Language; got: {prefs:?}"
        );
    }

    #[test]
    fn query_lang_does_not_include_cookie_or_accept() {
        let mut headers = headers_with_cookie("lang=de");
        headers.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            axum::http::HeaderValue::from_str("fr").unwrap(),
        );
        let (prefs, from_query) = extract_lang_prefs(&headers, Some("lang=it"));
        assert!(from_query);
        assert_eq!(prefs, vec!["it".to_owned()]);
    }

    fn headers_with_appearance_cookie(val: &str) -> HeaderMap {
        let cookie = format!("appearance={val}");
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::COOKIE, axum::http::HeaderValue::from_str(&cookie).unwrap());
        h
    }

    #[test]
    fn appearance_query_dark_sets_from_query_true_and_parses() {
        let result = extract_appearance(&empty_headers(), Some("appearance=dark"));
        assert!(result.from_query, "?appearance= present must set from_query=true");
        assert_eq!(
            result.value,
            Some(cpe_core::theme::Appearance::Dark),
            "?appearance=dark must parse to Dark"
        );
    }

    #[test]
    fn appearance_query_light_parses() {
        let result = extract_appearance(&empty_headers(), Some("appearance=light"));
        assert!(result.from_query);
        assert_eq!(result.value, Some(cpe_core::theme::Appearance::Light));
    }

    #[test]
    fn appearance_query_unknown_value_gives_none_but_from_query_true() {
        // The handler sets a cookie only for from_query && value.is_some(), so the bad input is
        // dropped.
        let result = extract_appearance(&empty_headers(), Some("appearance=solarized"));
        assert!(result.from_query, "from_query must be true when ?appearance= key is present");
        assert_eq!(result.value, None, "unknown appearance must produce None");
    }

    #[test]
    fn appearance_adversarial_query_value_is_ignored() {
        // An attacker might try to inject cookie attributes via ?appearance=.
        let result = extract_appearance(&empty_headers(), Some("appearance=dark%3BDomain%3Devil"));
        // Note: URL decoding is NOT done here (we operate on the raw query string),
        // so "dark%3BDomain%3Devil" is treated as an unrecognised literal → None.
        assert_eq!(
            result.value, None,
            "adversarial ?appearance= value must not parse (no injection)"
        );
        assert!(result.from_query);
    }

    #[test]
    fn appearance_cookie_dark_used_when_no_query() {
        let headers = headers_with_appearance_cookie("dark");
        let result = extract_appearance(&headers, None);
        assert!(!result.from_query, "cookie source must not set from_query");
        assert_eq!(result.value, Some(cpe_core::theme::Appearance::Dark));
    }

    #[test]
    fn appearance_cookie_light_used_when_no_query() {
        let headers = headers_with_appearance_cookie("light");
        let result = extract_appearance(&headers, None);
        assert!(!result.from_query);
        assert_eq!(result.value, Some(cpe_core::theme::Appearance::Light));
    }

    #[test]
    fn appearance_cookie_unknown_value_returns_none() {
        let headers = headers_with_appearance_cookie("invalid");
        let result = extract_appearance(&headers, None);
        assert!(!result.from_query);
        assert_eq!(result.value, None);
    }

    #[test]
    fn appearance_cookie_with_other_cookies() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_str("lang=de; appearance=dark; session=abc").unwrap(),
        );
        let result = extract_appearance(&h, None);
        assert_eq!(
            result.value,
            Some(cpe_core::theme::Appearance::Dark),
            "appearance cookie found among other cookies"
        );
    }

    #[test]
    fn appearance_absent_returns_none_not_from_query() {
        let result = extract_appearance(&empty_headers(), None);
        assert!(!result.from_query);
        assert_eq!(result.value, None);
    }

    #[test]
    fn appearance_query_takes_precedence_over_cookie() {
        let headers = headers_with_appearance_cookie("dark");
        let result = extract_appearance(&headers, Some("appearance=light"));
        assert!(result.from_query);
        assert_eq!(result.value, Some(cpe_core::theme::Appearance::Light));
    }

    #[test]
    fn query_img_absent_returns_one() {
        assert_eq!(query_img(None), 1);
        assert_eq!(query_img(Some("lang=de")), 1);
    }

    #[test]
    fn query_img_parses_valid_index() {
        assert_eq!(query_img(Some("img=2")), 2);
        assert_eq!(query_img(Some("lang=de&img=5")), 5);
        assert_eq!(query_img(Some("img=1&lang=en")), 1);
    }

    #[test]
    fn query_img_zero_falls_back_to_one() {
        // `img=0` is out of the 1-based vocabulary — treat it like absent.
        assert_eq!(query_img(Some("img=0&lang=en")), 1);
    }

    #[test]
    fn query_strip_absent_defaults_open() {
        assert!(query_strip(None));
        assert!(query_strip(Some("lang=de")));
    }

    #[test]
    fn query_strip_zero_is_closed() {
        assert!(!query_strip(Some("strip=0")));
    }

    #[test]
    fn query_strip_one_is_open() {
        assert!(query_strip(Some("strip=1")));
    }

    #[test]
    fn query_img_unparseable_defaults_to_one() {
        assert_eq!(query_img(Some("img=bad")), 1);
        assert_eq!(query_img(Some("img=-1")), 1); // usize parse fails for negative
        assert_eq!(query_img(Some("img=")), 1);
    }

    #[test]
    fn query_pairs_splits_and_parses_ampersand_and_equals() {
        let pairs: Vec<(&str, &str)> = query_pairs(Some("a=1&b=2")).collect();
        assert_eq!(pairs, vec![("a", "1"), ("b", "2")]);
    }

    #[test]
    fn query_pairs_bare_key_with_no_equals_yields_empty_value() {
        let pairs: Vec<(&str, &str)> = query_pairs(Some("bare&x=1")).collect();
        assert_eq!(pairs, vec![("bare", ""), ("x", "1")]);
    }

    #[test]
    fn query_pairs_none_query_yields_empty_iterator() {
        let pairs: Vec<(&str, &str)> = query_pairs(None).collect();
        assert_eq!(pairs, Vec::<(&str, &str)>::new());
    }

    #[test]
    fn derive_request_origin_uses_forwarded_proto_and_host_when_both_present() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "cpe-pr-42-pbjdzenira-oa.a.run.app".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        assert_eq!(
            derive_request_origin(&headers, 8080),
            "https://cpe-pr-42-pbjdzenira-oa.a.run.app"
        );
    }

    #[test]
    fn derive_request_origin_defaults_proto_to_http_when_missing() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "localhost:8080".parse().unwrap());
        assert_eq!(derive_request_origin(&headers, 8080), "http://localhost:8080");
    }

    #[test]
    fn derive_request_origin_falls_back_to_localhost_when_host_missing() {
        let headers = HeaderMap::new();
        assert_eq!(derive_request_origin(&headers, 3000), "http://localhost:3000");
    }

    #[test]
    fn derive_request_origin_takes_no_scheme_but_http_and_https() {
        for (sent, used) in [
            ("https", "https"),
            ("HTTPS", "https"),
            ("http", "http"),
            ("javascript", "http"),
            ("data:text/html,x", "http"),
            ("https, http", "http"),
            ("", "http"),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert("host", "preview.example.test".parse().unwrap());
            headers.insert("x-forwarded-proto", sent.parse().unwrap());
            assert_eq!(
                derive_request_origin(&headers, 80),
                format!("{used}://preview.example.test"),
                "{sent:?}"
            );
        }
    }

    #[test]
    fn derive_request_origin_takes_a_host_only_under_the_configured_origins_rule() {
        for good in ["preview.example.test", "localhost:8080", "[::1]:3000", "127.0.0.1"] {
            let mut headers = HeaderMap::new();
            headers.insert("host", good.parse().unwrap());
            assert_eq!(derive_request_origin(&headers, 80), format!("http://{good}"), "{good:?}");
        }
        let long = "a".repeat(256);
        for bad in [
            "evil.example/path",
            "user@evil.example",
            "evil.example\\@good.example",
            "a b",
            "x\"onmouseover=\"y",
            "evil.example?q",
            "evil.example#f",
            ":8080",
            "",
            long.as_str(),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert("host", bad.parse().unwrap());
            headers.insert("x-forwarded-proto", "https".parse().unwrap());
            assert_eq!(
                derive_request_origin(&headers, 80),
                "http://localhost:80",
                "{bad:?} is answered as a request without a Host"
            );
        }
    }

    #[test]
    fn origin_config_fixed_override_wins_over_any_headers() {
        let cfg = OriginConfig::fixed("https://cpe-pbjdzenira-oa.a.run.app");
        let mut headers = HeaderMap::new();
        headers.insert("host", "some-other-host.example".parse().unwrap());
        headers.insert("x-forwarded-proto", "http".parse().unwrap());
        assert_eq!(cfg.resolve(&headers), "https://cpe-pbjdzenira-oa.a.run.app");
    }

    #[test]
    fn origin_config_without_an_origin_derives_it_from_each_request() {
        for unset in [None, Some(""), Some("   ")] {
            let cfg = OriginConfig::new(unset);
            let mut headers = HeaderMap::new();
            headers.insert(axum::http::header::HOST, "preview.example.test".parse().unwrap());
            headers.insert("x-forwarded-proto", "https".parse().unwrap());
            assert_eq!(cfg.resolve(&headers), "https://preview.example.test", "{unset:?}");
            assert_eq!(cfg.resolve(&HeaderMap::new()), "http://localhost:80", "{unset:?}");
        }
    }
}
