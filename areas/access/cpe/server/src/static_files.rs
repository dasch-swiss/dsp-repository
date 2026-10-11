//! CPE's shared static files, compiled into the binary and served by lookup.
//!
//! The table holds the seven stylesheets of `cpe-components`' `style/` and the
//! files of `areas/access/cpe/public/`. A file added to one of those folders
//! needs a row here; `every_file_of_the_three_folders_has_a_row` fails until
//! it has one. `shell.css` refers to `dasch-fingerprint.svg` by a relative
//! URL, so the stylesheets and the images share the `cpe/` prefix.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::router::content_type;

/// `(path under /_static/, bytes)`.
static FILES: &[(&str, &[u8])] = &[
    ("cpe/editorial.css", include_bytes!("../../components/style/editorial.css")),
    ("cpe/grid.css", include_bytes!("../../components/style/grid.css")),
    ("cpe/incunabula.css", include_bytes!("../../components/style/incunabula.css")),
    ("cpe/media.css", include_bytes!("../../components/style/media.css")),
    ("cpe/record.css", include_bytes!("../../components/style/record.css")),
    ("cpe/shell.css", include_bytes!("../../components/style/shell.css")),
    ("cpe/tokens.css", include_bytes!("../../components/style/tokens.css")),
    (
        "cpe/dasch-fingerprint.svg",
        include_bytes!("../../public/dasch-fingerprint.svg"),
    ),
    (
        "cpe/dasch-logo-colour.svg",
        include_bytes!("../../public/dasch-logo-colour.svg"),
    ),
    ("cpe/dasch-logo-white.svg", include_bytes!("../../public/dasch-logo-white.svg")),
    ("datastar.js", include_bytes!("../../public/vendor/datastar.js")),
];

/// Answer `/_static/{*path}`: the file of that exact name, or 404.
pub(crate) fn serve(path: &str) -> Response {
    match FILES.iter().find(|(name, _)| *name == path) {
        Some((name, bytes)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type(name)),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            *bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
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
        crate::router::build(
            &[],
            &[],
            &AssetInventory::default(),
            OriginConfig::fixed("http://localhost:8080"),
        )
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("hand-crafted test request is well-formed"),
        )
        .await
        .expect("oneshot against an in-process router never fails")
    }

    /// The three folders the table is written from, with the prefix their
    /// files are served under. `README.md` documents `public/vendor/` and is
    /// not served.
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
            let row = FILES.iter().find(|(row, _)| row == name);
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
            ("datastar.js", "text/javascript"),
        ] {
            let response = serve(path);
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(header_value(&response, header::CONTENT_TYPE), expected, "{path}");
            assert_eq!(header_value(&response, header::X_CONTENT_TYPE_OPTIONS), "nosniff", "{path}");
        }
    }

    #[tokio::test]
    async fn serve_static_returns_file_for_known_path() {
        let response = serve("cpe/tokens.css");
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
            assert_eq!(serve(path).status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[test]
    fn serve_static_rejects_dotfiles() {
        assert_eq!(serve(".gitkeep").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serve_static_returns_404_for_missing_file() {
        for path in ["nonexistent.bin", "README.md", "cpe/README.md", "tokens.css", ""] {
            assert_eq!(serve(path).status(), StatusCode::NOT_FOUND, "{path:?}");
        }
    }

    #[tokio::test]
    async fn static_route_serves_the_stylesheets_beside_the_public_files() {
        for (uri, expected) in [
            ("/_static/cpe/shell.css", "text/css; charset=utf-8"),
            ("/_static/cpe/dasch-fingerprint.svg", "image/svg+xml"),
            ("/_static/datastar.js", "text/javascript"),
        ] {
            let response = get(uri).await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(header_value(&response, header::CONTENT_TYPE), expected, "{uri}");
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
