//! CPE's shared static files, compiled into the binary and served by lookup.
//!
//! The table holds the seven stylesheets of `cpe-components`' `style/` and the
//! files of `areas/access/cpe/public/`. A file added to one of those folders
//! needs a row here; `every_file_of_the_three_folders_has_a_row` fails until
//! it has one. `shell.css` refers to `dasch-fingerprint.svg` by a relative
//! URL, so the stylesheets and the images share the `cpe/` prefix.

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use Caching::{Heuristic, Revalidate};

use crate::assets::ASSET_CSP;
use crate::router::content_type;

/// How a browser may cache a file.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Caching {
    /// No `Cache-Control` and no validator: the browser's heuristics decide.
    Heuristic,
    /// `Cache-Control: no-cache` and an `ETag`, so the browser revalidates the file on every use
    /// and a `304` answers an unchanged one. For `cpe.js`, whose functions the HTML calls by
    /// name and signature: a cached old script beside new HTML, or the reverse, breaks the page.
    Revalidate,
}

/// `(path under a static directory, bytes, caching)`.
static FILES: &[(&str, &[u8], Caching)] = &[
    (
        "cpe/editorial.css",
        include_bytes!("../../components/style/editorial.css"),
        Heuristic,
    ),
    ("cpe/grid.css", include_bytes!("../../components/style/grid.css"), Heuristic),
    (
        "cpe/incunabula.css",
        include_bytes!("../../components/style/incunabula.css"),
        Heuristic,
    ),
    ("cpe/media.css", include_bytes!("../../components/style/media.css"), Heuristic),
    ("cpe/record.css", include_bytes!("../../components/style/record.css"), Heuristic),
    ("cpe/shell.css", include_bytes!("../../components/style/shell.css"), Heuristic),
    ("cpe/tokens.css", include_bytes!("../../components/style/tokens.css"), Heuristic),
    (
        "cpe/dasch-fingerprint.svg",
        include_bytes!("../../public/dasch-fingerprint.svg"),
        Heuristic,
    ),
    (
        "cpe/dasch-logo-colour.svg",
        include_bytes!("../../public/dasch-logo-colour.svg"),
        Heuristic,
    ),
    (
        "cpe/dasch-logo-white.svg",
        include_bytes!("../../public/dasch-logo-white.svg"),
        Heuristic,
    ),
    ("cpe/cpe.js", include_bytes!("../../public/cpe.js"), Revalidate),
    ("datastar.js", include_bytes!("../../public/vendor/datastar.js"), Heuristic),
];

/// The `ETag` of each [`Revalidate`] file, computed from its embedded bytes on first use: the
/// first 16 bytes of their SHA-256 in hex, quoted (a strong validator).
static ETAGS: LazyLock<HashMap<&str, String>> = LazyLock::new(|| {
    FILES
        .iter()
        .filter(|(_, _, caching)| *caching == Revalidate)
        .map(|(name, bytes, _)| {
            let hex: String = Sha256::digest(bytes)[..16].iter().map(|b| format!("{b:02x}")).collect();
            (*name, format!("\"{hex}\""))
        })
        .collect()
});

/// Whether the request's `If-None-Match` is `*` or names `tag` (weak comparison, which
/// RFC 9110 prescribes for `If-None-Match`).
fn if_none_match_hits(request: &HeaderMap, tag: &str) -> bool {
    request.get_all(header::IF_NONE_MATCH).iter().any(|value| {
        value.to_str().is_ok_and(|list| {
            list.split(',')
                .map(str::trim)
                .any(|candidate| candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == tag)
        })
    })
}

/// Answer `{*path}` of a static directory (`cpe_core::url::StaticDir`): the file of that exact
/// name, or 404. A [`Revalidate`] file whose `ETag` the request's `If-None-Match` names is
/// answered `304` with no body. An SVG carries [`ASSET_CSP`], as a project's assets do.
pub(crate) fn serve(path: &str, request: &HeaderMap) -> Response {
    let Some((name, bytes, caching)) = FILES.iter().find(|(name, _, _)| *name == path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let headers = [
        (header::CONTENT_TYPE, content_type(name)),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ];
    let mut response = match caching {
        Heuristic => (StatusCode::OK, headers, *bytes).into_response(),
        Revalidate => {
            let tag = ETAGS[name].as_str();
            let validators = [(header::CACHE_CONTROL, "no-cache"), (header::ETAG, tag)];
            if if_none_match_hits(request, tag) {
                (StatusCode::NOT_MODIFIED, validators).into_response()
            } else {
                (StatusCode::OK, headers, validators, *bytes).into_response()
            }
        }
    };
    if name.ends_with(".svg") {
        response
            .headers_mut()
            .insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(ASSET_CSP));
    }
    response
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::assets::AssetInventory;
    use crate::request::OriginConfig;

    fn header_value(response: &Response, name: header::HeaderName) -> &str {
        response
            .headers()
            .get(name)
            .expect("the header is present")
            .to_str()
            .expect("the header is ASCII")
    }

    async fn get(uri: &str) -> Response {
        get_with(uri, None).await
    }

    async fn get_with(uri: &str, if_none_match: Option<&str>) -> Response {
        let mut request = Request::builder().uri(uri);
        if let Some(tag) = if_none_match {
            request = request.header(header::IF_NONE_MATCH, tag);
        }
        crate::router::build(
            &[],
            &[],
            &AssetInventory::default(),
            OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
            &std::sync::Arc::default(),
        )
        .oneshot(request.body(Body::empty()).expect("hand-crafted test request is well-formed"))
        .await
        .expect("oneshot against an in-process router never fails")
    }

    /// The three folders the table is written from, with the prefix their
    /// files are served under. `README.md` documents `public/vendor/` and is
    /// not served, nor is a dotfile such as `public/.biome.json`.
    #[test]
    fn every_file_of_the_three_folders_has_a_row() {
        let cpe = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut expected = Vec::new();
        for (folder, prefix) in [("components/style", "cpe/"), ("public", "cpe/"), ("public/vendor", "")] {
            let entries = std::fs::read_dir(cpe.join(folder)).expect("the folder is readable");
            for entry in entries {
                let entry = entry.expect("the directory entry is readable");
                let name = entry.file_name().into_string().expect("file names are UTF-8");
                if entry.path().is_dir() || name == "README.md" || name.starts_with('.') {
                    continue;
                }
                let bytes = std::fs::read(entry.path()).expect("the file is readable");
                expected.push((format!("{prefix}{name}"), bytes));
            }
        }
        assert!(expected.len() >= 11, "the folders were found empty: {}", expected.len());
        for (name, bytes) in &expected {
            let row = FILES.iter().find(|(row, _, _)| row == name);
            let row =
                row.unwrap_or_else(|| panic!("{name} has no row in FILES; add (\"{name}\", include_bytes!(…)) for it"));
            assert!(row.1 == bytes.as_slice(), "the row for {name} holds another file's bytes");
        }
        assert_eq!(
            FILES.len(),
            expected.len(),
            "FILES has a row for a file that is in none of the folders"
        );
    }

    #[test]
    fn the_content_types_follow_the_extension() {
        for (path, expected) in [
            ("cpe/tokens.css", "text/css; charset=utf-8"),
            ("cpe/dasch-logo-colour.svg", "image/svg+xml"),
            ("cpe/cpe.js", "text/javascript"),
            ("datastar.js", "text/javascript"),
        ] {
            let response = serve(path, &HeaderMap::new());
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(header_value(&response, header::CONTENT_TYPE), expected, "{path}");
            assert_eq!(header_value(&response, header::X_CONTENT_TYPE_OPTIONS), "nosniff", "{path}");
        }
    }

    #[test]
    fn every_svg_and_no_other_file_carries_the_asset_policy() {
        for (path, _, _) in FILES {
            let response = serve(path, &HeaderMap::new());
            let policy = response.headers().get(header::CONTENT_SECURITY_POLICY);
            if path.ends_with(".svg") {
                assert_eq!(policy.map(|value| value.as_bytes()), Some(ASSET_CSP.as_bytes()), "{path}");
            } else {
                assert_eq!(policy, None, "{path}");
            }
        }
    }

    #[tokio::test]
    async fn serve_static_returns_file_for_known_path() {
        let response = serve("cpe/tokens.css", &HeaderMap::new());
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let on_disk = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../components/style/tokens.css"))
            .expect("the stylesheet is readable");
        assert_eq!(body.as_ref(), on_disk.as_slice());
    }

    #[test]
    fn serve_static_rejects_path_traversal() {
        for path in [
            "../escape",
            "cpe/../cpe/tokens.css",
            "cpe/./tokens.css",
            "/cpe/tokens.css",
            "cpe//tokens.css",
        ] {
            assert_eq!(serve(path, &HeaderMap::new()).status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[test]
    fn serve_static_rejects_dotfiles() {
        assert_eq!(serve(".gitkeep", &HeaderMap::new()).status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serve_static_returns_404_for_missing_file() {
        for path in [
            "nonexistent.bin",
            "README.md",
            "cpe/README.md",
            "cpe/.biome.json",
            "tokens.css",
            "",
        ] {
            assert_eq!(serve(path, &HeaderMap::new()).status(), StatusCode::NOT_FOUND, "{path:?}");
        }
    }

    #[tokio::test]
    async fn static_route_serves_the_stylesheets_beside_the_public_files() {
        for (uri, expected) in [
            ("/_static/cpe/shell.css", "text/css; charset=utf-8"),
            ("/_static/cpe/dasch-fingerprint.svg", "image/svg+xml"),
            ("/_static/cpe/cpe.js", "text/javascript"),
            ("/_static/datastar.js", "text/javascript"),
        ] {
            let response = get(uri).await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(header_value(&response, header::CONTENT_TYPE), expected, "{uri}");
        }
    }

    /// `cpe.js` and the HTML that calls it share a contract, so a browser
    /// revalidates the script on every use rather than pairing new HTML with a
    /// heuristically cached old script; the `ETag` keeps that revalidation a
    /// `304` while the script is unchanged.
    #[tokio::test]
    async fn cpe_js_is_revalidated_on_every_use() {
        let response = get("/_static/cpe/cpe.js").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_value(&response, header::CACHE_CONTROL), "no-cache");
        let tag = header_value(&response, header::ETAG).to_owned();
        assert!(tag.starts_with('"') && tag.ends_with('"') && tag.len() > 2, "{tag}");

        for matching in [
            tag.clone(),
            format!("W/{tag}"),
            format!("\"other\", {tag}"),
            "*".to_owned(),
        ] {
            let response = get_with("/_static/cpe/cpe.js", Some(&matching)).await;
            assert_eq!(response.status(), StatusCode::NOT_MODIFIED, "{matching}");
            assert_eq!(header_value(&response, header::ETAG), tag, "{matching}");
            assert_eq!(header_value(&response, header::CACHE_CONTROL), "no-cache", "{matching}");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response body is small and finite in tests");
            assert!(body.is_empty(), "{matching}");
        }

        let stale = get_with("/_static/cpe/cpe.js", Some("\"stale\"")).await;
        assert_eq!(stale.status(), StatusCode::OK);
        let body = axum::body::to_bytes(stale.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        assert_eq!(body.as_ref(), include_bytes!("../../public/cpe.js"));
    }

    /// The other static files keep the caching they had: no `Cache-Control`,
    /// no validator, and an `If-None-Match` is not answered with a `304`.
    #[tokio::test]
    async fn the_other_static_files_carry_no_cache_headers() {
        for uri in [
            "/_static/cpe/shell.css",
            "/_static/cpe/dasch-fingerprint.svg",
            "/_static/datastar.js",
        ] {
            let response = get_with(uri, Some("*")).await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert!(response.headers().get(header::CACHE_CONTROL).is_none(), "{uri}");
            assert!(response.headers().get(header::ETAG).is_none(), "{uri}");
        }
    }

    #[tokio::test]
    async fn static_route_returns_404_for_a_path_outside_the_table() {
        assert_eq!(get("/_static/cpe/tokens.css").await.status(), StatusCode::OK);
        for uri in [
            "/_static/.gitkeep",
            "/_static/cpe/%2e%2e/datastar.js",
            "/_static/cpe%2ftokens.css%00",
            "/_static/vendor/datastar.js",
            "/_static/cpe/nope.css",
        ] {
            assert_eq!(get(uri).await.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }
}
