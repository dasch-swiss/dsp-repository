//! What CPE reads from a request before a project handles it: the language
//! and appearance preferences, the query-string and cookie readers, and the
//! public origin that absolute links are built from.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

/// Language preference extracted from a request before project dispatch.
///
/// Built by `resolve_request_prefs` middleware and stored in request extensions.
/// The handler calls `cpe_core::lang::resolve(&requested.prefs, site)` to
/// reconcile against a specific project's declared languages.
#[derive(Clone, Debug)]
pub(crate) struct RequestedLang {
    /// Ordered preference list (lowercased primary subtags, region stripped).
    pub prefs: Vec<String>,
    /// `true` when the preference came from an explicit `?lang=` query param.
    /// The handler sets `Set-Cookie` only in this case.
    pub from_query: bool,
}

/// Appearance preference extracted from a request before project dispatch.
///
/// Built by `resolve_request_prefs` middleware (which also resolves appearance) and
/// stored in request extensions alongside `RequestedLang`. Mirrors the
/// `RequestedLang` pattern.
///
/// `value: None` means no recognised appearance signal was supplied (either
/// absent or unparseable); the renderer falls back to the default
/// [`cpe_core::theme::Appearance::Light`]. CPE does not follow the OS
/// `prefers-color-scheme` — light is the CSS baseline and the header toggle is
/// the only switch to dark.
#[derive(Clone, Debug)]
pub(crate) struct RequestedAppearance {
    /// The parsed appearance value, or `None` when absent or unparseable.
    pub value: Option<cpe_core::theme::Appearance>,
    /// `true` when the preference came from an explicit `?appearance=` query param.
    /// The handler manages the `appearance` cookie only in this case: it sets
    /// `appearance=dark` for `Some(Dark)`, clears the cookie for `Some(Light)`
    /// (light is the default), and ignores unparseable input (`None`).
    pub from_query: bool,
}

/// Normalize a language tag for comparison: lowercase, strip the region
/// subtag (everything from the first `-` onward), and trim whitespace.
///
/// Examples: `"en-US"` → `"en"`, `"DE"` → `"de"`, `" fr "` → `"fr"`.
pub(crate) fn normalize_lang(tag: &str) -> String {
    let primary = match tag.find('-') {
        Some(idx) => &tag[..idx],
        None => tag,
    };
    primary.trim().to_lowercase()
}

/// Axum `from_fn` middleware that extracts the request-scoped presentation
/// preferences — language and appearance — and stores them in
/// `req.extensions` as `RequestedLang` and `RequestedAppearance`.
///
/// Language preference order (highest to lowest):
/// 1. `?lang=<code>` query parameter (sets `from_query = true`; only this source is consulted —
///    cookie and Accept-Language are NOT appended).
/// 2. `lang` cookie from the `Cookie` header.
/// 3. `Accept-Language` header entries (source order; `;q=` weights ignored).
///
/// Appearance preference order (highest to lowest):
/// 1. `?appearance=<value>` query parameter (marks the source `from_query`).
/// 2. `appearance` cookie from the `Cookie` header.
///
/// This middleware never returns a 400 — unknown or garbage values simply
/// produce `None`/default. Cookies are NOT set here; handlers set them after
/// resolving against the specific project (lang against its declared
/// languages).
pub(crate) async fn resolve_request_prefs(mut req: Request, next: Next) -> Response {
    let (prefs, lang_from_query) = extract_lang_prefs(req.headers(), req.uri().query());
    // Record the top requested lang on the tracing span (best-effort).
    if let Some(top) = prefs.first() {
        tracing::Span::current().record("requested_lang", top.as_str());
    }
    req.extensions_mut()
        .insert(RequestedLang { prefs, from_query: lang_from_query });

    let appearance = extract_appearance(req.headers(), req.uri().query());
    req.extensions_mut().insert(appearance);

    next.run(req).await
}

/// Puts the route that matched into the response's extensions, as
/// [`crate::MatchedRoute`]. The answer for a path that matches no route gets
/// none.
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

/// Pure extraction helper — testable without building a full `Request`.
///
/// Returns `(prefs, from_query)`.
pub(crate) fn extract_lang_prefs(headers: &HeaderMap, query: Option<&str>) -> (Vec<String>, bool) {
    // ?lang= query parameter
    if let Some(lang_q) = query_lang(query) {
        if !lang_q.is_empty() {
            let normalized = normalize_lang(&lang_q);
            if !normalized.is_empty() {
                return (vec![normalized], true);
            }
        }
    }

    // cookie then Accept-Language
    let mut prefs: Vec<String> = Vec::new();

    // Cookie: lang=<value>
    if let Some(cookie_val) = cookie_lang(headers) {
        let normalized = normalize_lang(&cookie_val);
        if !normalized.is_empty() {
            prefs.push(normalized);
        }
    }

    // Accept-Language: e.g. "en-US,de;q=0.9,fr;q=0.8"
    if let Some(al) = headers.get(axum::http::header::ACCEPT_LANGUAGE) {
        if let Ok(al_str) = al.to_str() {
            for entry in al_str.split(',') {
                // Strip the ";q=…" quality weight, keep only the tag.
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

/// Pure extraction helper for appearance preferences — testable without a full `Request`.
///
/// Preference order (highest to lowest):
/// 1. `?appearance=<value>` query parameter (`from_query = true`).
/// 2. `appearance` cookie from the `Cookie` header.
///
/// Unparseable or absent values produce `value: None` (the renderer then uses
/// the default light scheme). The cookie is never touched here; the handler
/// manages it when `from_query` is true (set `dark`, or clear for `light`).
pub(crate) fn extract_appearance(headers: &HeaderMap, query: Option<&str>) -> RequestedAppearance {
    // ?appearance= query parameter
    if let Some(raw) = query_appearance(query) {
        // The key was present; parse the value.
        let value = cpe_core::theme::Appearance::parse_config_str(&raw);
        return RequestedAppearance { value, from_query: true };
    }

    // appearance cookie
    if let Some(raw) = cookie_appearance(headers) {
        let value = cpe_core::theme::Appearance::parse_config_str(&raw);
        return RequestedAppearance { value, from_query: false };
    }

    // No appearance signal at all.
    RequestedAppearance { value: None, from_query: false }
}

/// Iterate `(key, value)` pairs from a query string, splitting on `&` then the
/// first `=` within each pair (a bare key with no `=` yields `(key, "")`).
///
/// Shared by every query-param reader (`query_appearance`,
/// `query_lang`, `query_img` here; `parse_u32_param`,
/// `query_param_one`, `query_param_all` in `router.rs`) so the query-string
/// walk is written once instead of duplicated per key/shape (single value,
/// all values, numeric parse with a default, …). An absent `query` yields an
/// empty iterator. This is purely a read helper — `router.rs`'s
/// `redirect_without_keys` / `strip_datastar_marker`
/// rebuild a filtered query string (a different concern) and do not use this.
pub(crate) fn query_pairs(query: Option<&str>) -> impl Iterator<Item = (&str, &str)> {
    query.into_iter().flat_map(|q| q.split('&')).map(|pair| match pair.find('=') {
        Some(idx) => (&pair[..idx], &pair[idx + 1..]),
        None => (pair, ""),
    })
}

/// Extract the `appearance` value from the query string.
/// Returns `None` when the key is absent; `Some(raw_value)` when present.
fn query_appearance(query: Option<&str>) -> Option<String> {
    query_pairs(query).find(|(k, _)| *k == "appearance").map(|(_, v)| v.to_owned())
}

/// Extract the `appearance` cookie value from the `Cookie` request header.
/// Returns `None` when absent.
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

/// Extract the `lang` value from the query string (e.g. `?lang=de&foo=bar`).
/// Returns `None` when the key is absent; `Some("")` when present but empty.
fn query_lang(query: Option<&str>) -> Option<String> {
    // URL-decoding the minimal subset (%XX) is not needed for language codes
    // (ASCII letters only), so the raw value is returned as-is.
    query_pairs(query).find(|(k, _)| *k == "lang").map(|(_, v)| v.to_owned())
}

/// Extract the 1-based `img` index from the query string (e.g. `?img=2&lang=de`).
///
/// **1-based**, like `?member=` and the `.pg` pager. Returns the parsed
/// positive integer when the key is present and parseable; defaults to `1`
/// (the first image) when absent, zero, or unparseable (any garbage value is
/// silently treated as the start of the sequence). Callers clamp against the
/// actual media-item count.
pub(crate) fn query_img(query: Option<&str>) -> usize {
    query_pairs(query)
        .find(|(k, _)| *k == "img")
        .map(|(_, v)| v.parse::<usize>().unwrap_or(1))
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

/// Extract the record viewer's thumb-strip fold state from the query string
/// (`?strip=0` closed; absent or any other value → open).
///
/// Open-on-load is the default. The record-viewer's own strip fold is a
/// client-side `$stripOpen` signal (never a URL write,
/// `dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`);
/// `?strip=` is only the initial-load / zero-JS full-reload condition read here.
pub(crate) fn query_strip(query: Option<&str>) -> bool {
    query_pairs(query)
        .find(|(k, _)| *k == "strip")
        .map(|(_, v)| v != "0")
        .unwrap_or(true)
}

/// Extract the `lang` cookie value from the `Cookie` request header.
/// Returns `None` when absent.
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

/// Where a request's public origin comes from.
///
/// A configured origin (`CPE_PUBLIC_ORIGIN`) is used for every request. With
/// none, the origin is derived **per request** from `X-Forwarded-Proto`
/// and `Host` ([`derive_request_origin`], which says what it takes of them), so local
/// development, a preview deployment and the stable service each report
/// their own host with no configuration. One origin resolved at start would
/// be wrong for a preview deployed under another host with the same
/// environment.
///
/// Cheaply `Clone` (`Option<Arc<str>>`), so every handler closure gets its
/// own.
#[derive(Clone, Debug)]
pub(crate) struct OriginConfig {
    override_origin: Option<Arc<str>>,
}

/// The port in the origin of a request that carries no `Host` header. CPE
/// does not know the port its host listens on, and 80 makes that origin say
/// no more than `http://localhost`.
const NO_HOST_PORT: u16 = 80;

impl OriginConfig {
    /// `public_origin` is the configured origin; `None` or a blank value
    /// means "derive per request".
    pub(crate) fn new(public_origin: Option<&str>) -> Self {
        Self {
            override_origin: public_origin.filter(|origin| !origin.trim().is_empty()).map(Arc::from),
        }
    }

    /// A config that answers `origin` for every request, as a configured
    /// origin does.
    #[cfg(test)]
    pub(crate) fn fixed(origin: &str) -> Self {
        Self::new(Some(origin))
    }

    /// Resolve this request's public origin: the configured origin when
    /// there is one; otherwise derive from `headers` via
    /// [`derive_request_origin`].
    ///
    /// No trailing slash is stripped/enforced here — `share_link::
    /// compose_absolute_url` (every consumer) trims a trailing slash
    /// itself, so an origin with or without one is safe either way.
    pub(crate) fn resolve(&self, headers: &HeaderMap) -> String {
        match &self.override_origin {
            Some(origin) => origin.to_string(),
            None => derive_request_origin(headers, NO_HOST_PORT),
        }
    }
}

/// Derive an absolute origin (`scheme://host`, no path, no trailing slash)
/// from one request's own headers: `X-Forwarded-Proto` and `Host`. Cloud Run
/// sets both on every request it proxies, and a local `curl` or browser
/// request always carries `Host`.
///
/// Both headers are a client's to send, and the origin becomes the start of
/// the share links' `href`. So the scheme is `https` or `http` and nothing
/// else (`http` for any other value), and a `Host` that
/// [`crate::config::is_origin_host`] refuses is treated as absent:
/// `http://localhost:<port>`. Never panics.
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

    // Language middleware unit tests

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

    // normalize_lang tests

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

    // extract_lang_prefs: ?lang= query parameter

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

    // extract_lang_prefs: cookie fallback

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

    // extract_lang_prefs: Accept-Language

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
        // cookie lang (de) comes first, then Accept-Language entries
        assert_eq!(
            prefs,
            vec!["de".to_owned(), "en".to_owned(), "fr".to_owned()],
            "cookie first, then Accept-Language; got: {prefs:?}"
        );
    }

    // extract_lang_prefs: ?lang= takes exclusive precedence

    #[test]
    fn query_lang_does_not_include_cookie_or_accept() {
        let mut headers = headers_with_cookie("lang=de");
        headers.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            axum::http::HeaderValue::from_str("fr").unwrap(),
        );
        let (prefs, from_query) = extract_lang_prefs(&headers, Some("lang=it"));
        assert!(from_query);
        // ONLY the query value — cookie and accept-language NOT appended
        assert_eq!(prefs, vec!["it".to_owned()]);
    }

    // Appearance extraction tests

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
        // Unparseable value → None (ignore), but from_query is still true
        // (the key was present). The handler uses from_query && value.is_some()
        // to decide whether to set a cookie, so this silently drops the bad input.
        let result = extract_appearance(&empty_headers(), Some("appearance=solarized"));
        assert!(result.from_query, "from_query must be true when ?appearance= key is present");
        assert_eq!(result.value, None, "unknown appearance must produce None");
    }

    #[test]
    fn appearance_adversarial_query_value_is_ignored() {
        // An attacker might try to inject cookie attributes via ?appearance=.
        // The parse step must reject this and return None — no header injection.
        let result = extract_appearance(&empty_headers(), Some("appearance=dark%3BDomain%3Devil"));
        // Note: URL decoding is NOT done here (we operate on the raw query string),
        // so "dark%3BDomain%3Devil" is treated as an unrecognised literal → None.
        assert_eq!(
            result.value, None,
            "adversarial ?appearance= value must not parse (no injection)"
        );
        // from_query is true (key was present), but value is None → no cookie set.
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
        // Query param wins over cookie: ?appearance=light with Cookie: appearance=dark
        let headers = headers_with_appearance_cookie("dark");
        let result = extract_appearance(&headers, Some("appearance=light"));
        assert!(result.from_query);
        assert_eq!(result.value, Some(cpe_core::theme::Appearance::Light));
    }

    // query_img: ?img= extraction

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

    // query_strip: ?strip= extraction

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

    // query_pairs

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

    // OriginConfig / derive_request_origin

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
