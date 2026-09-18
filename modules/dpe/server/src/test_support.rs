//! Shared helpers for this crate's tests: the app state the router is built
//! with, and a `Link` header parser so header assertions are about relations
//! rather than substrings.

use crate::AppState;

/// Static assets come from a nonexistent dir: the tests using this target
/// routes, never a real static file, so the `ServeDir` fallback is never
/// exercised.
pub(crate) const NO_PUBLIC_DIR: &str = "nonexistent-test-dir";

/// The app state every test in this crate builds a router or renders a page
/// with.
///
/// It also points `dpe-core` at the committed data, because that is the one
/// call every such test makes. Both directories are process-global `OnceLock`s
/// whose first caller wins, and `cargo test` runs from the package directory,
/// where `dpe-core`'s relative defaults miss: a test that touched a cache
/// before any of them was set would pin the *empty* corpus for the whole
/// binary, and every test needing real data would then fail depending on the
/// order the threads happened to take.
///
/// The two base URLs differ on purpose, as they do on DEV, so a test cannot
/// pass by accident when one is derived from the other.
pub(crate) fn test_state() -> AppState {
    dpe_core::set_data_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    dpe_core::set_public_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../public"));
    AppState {
        fathom_site_id: None,
        css_href: "/assets/app.css".to_string(),
        public_base_url: "https://example.test".to_string(),
        oai_base_url: "https://oai.example.test/dpe/oai".to_string(),
    }
}

/// One entry of a `Link` header.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ParsedLink {
    pub(crate) rel: String,
    pub(crate) href: String,
    pub(crate) media_type: Option<String>,
}

/// Parses a `Link` field value into its entries.
///
/// Hand-rolled and deliberately narrow: it understands exactly what Signposting
/// emits — comma-separated `<uri>; rel="x"; type="y"` entries with quoted
/// parameter values — and nothing else. No relative-URI resolution, no RFC 8187
/// encoding, no unquoted parameters. A full RFC 8288 parser would be a
/// dependency and a second thing to be right about; this asserts on the shape
/// the writer actually produces.
///
/// Splitting on `,` is safe for the same reason: every URI is inside angle
/// brackets and every parameter value inside quotes, and the writer
/// percent-encodes anything else.
pub(crate) fn parse_link_header(value: &str) -> Vec<ParsedLink> {
    value
        .split(',')
        .filter(|entry| !entry.trim().is_empty())
        .map(|entry| {
            let mut parts = entry.split(';').map(str::trim);
            let target = parts.next().unwrap_or_default();
            let href = target
                .strip_prefix('<')
                .and_then(|rest| rest.strip_suffix('>'))
                .unwrap_or_else(|| panic!("link target should be in angle brackets: {target}"))
                .to_string();

            let mut rel = None;
            let mut media_type = None;
            for part in parts {
                let (name, raw) = part
                    .split_once('=')
                    .unwrap_or_else(|| panic!("link parameter should be name=value: {part}"));
                let unquoted = raw.trim_matches('"').to_string();
                match name {
                    "rel" => rel = Some(unquoted),
                    "type" => media_type = Some(unquoted),
                    _ => {}
                }
            }

            ParsedLink {
                rel: rel.unwrap_or_else(|| panic!("link entry should carry a rel: {entry}")),
                href,
                media_type,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shape_the_writer_emits() {
        let parsed = parse_link_header(
            r#"<https://ark.example.test/ark:/1/2>; rel="cite-as", <https://example.test/r.jsonld>; rel="describedby"; type="application/ld+json""#,
        );
        assert_eq!(
            parsed,
            vec![
                ParsedLink {
                    rel: "cite-as".to_string(),
                    href: "https://ark.example.test/ark:/1/2".to_string(),
                    media_type: None,
                },
                ParsedLink {
                    rel: "describedby".to_string(),
                    href: "https://example.test/r.jsonld".to_string(),
                    media_type: Some("application/ld+json".to_string()),
                },
            ]
        );
    }
}
