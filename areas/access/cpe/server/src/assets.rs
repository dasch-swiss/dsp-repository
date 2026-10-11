//! The projects' own files (a cover image), read once and served by lookup.
//!
//! A request is answered by looking its project segment and its path up as
//! strings. Neither is ever joined onto a directory, so no request can name a
//! file that discovery did not list under a project's `assets/` folder.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::discovery::DiscoveredProject;
use crate::router::content_type;

/// Sent with every asset. A project may ship an SVG, and an SVG opened at its own URL is a document
/// of this origin: the policy keeps it usable as an image and lets no script in it run.
const ASSET_CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; sandbox";

/// Every served project's asset files, by project id and then by the path a
/// `site.cover-image` value carries (`assets/cover.webp`, with the `assets/`
/// segment). Cheap to clone.
#[derive(Clone, Default)]
pub(crate) struct AssetInventory(Arc<HashMap<String, HashMap<String, Bytes>>>);

impl AssetInventory {
    /// Read the asset files of `projects` from disk.
    pub(crate) fn of(projects: &[DiscoveredProject]) -> Self {
        let by_project = projects
            .iter()
            .map(|project| {
                let files = project
                    .assets()
                    .into_iter()
                    .map(|(path, bytes)| (path, Bytes::from(bytes)))
                    .collect();
                (project.id.clone(), files)
            })
            .collect();
        Self(Arc::new(by_project))
    }

    /// Answer `/_assets/{project_id}/{*path}`: the file, or 404 for anything
    /// that is not a key of the inventory.
    pub(crate) fn serve(&self, project_id: &str, path: &str) -> Response {
        match self.0.get(project_id).and_then(|files| files.get(path)) {
            Some(bytes) => (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, content_type(path)),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    (header::CONTENT_SECURITY_POLICY, ASSET_CSP),
                ],
                bytes.clone(),
            )
                .into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::discovery::discover;
    use crate::request::OriginConfig;

    fn fixture_assets() -> AssetInventory {
        AssetInventory::of(&discover(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery"),
        ))
    }

    fn header_value(response: &Response, name: header::HeaderName) -> &str {
        response
            .headers()
            .get(name)
            .expect("the header is present")
            .to_str()
            .expect("the header is ASCII")
    }

    async fn get(uri: &str) -> Response {
        crate::router::build(&[], &[], &fixture_assets(), OriginConfig::fixed("http://localhost:8080"))
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails")
    }

    #[tokio::test]
    async fn serve_project_asset_returns_file_for_known_path() {
        let response = fixture_assets().serve("alpha", "assets/cover.webp");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_value(&response, header::CONTENT_TYPE), "image/webp");
        assert_eq!(header_value(&response, header::X_CONTENT_TYPE_OPTIONS), "nosniff");
        assert_eq!(header_value(&response, header::CONTENT_SECURITY_POLICY), ASSET_CSP);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let on_disk = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery/alpha/assets/cover.webp"),
        )
        .expect("the fixture file is readable");
        assert_eq!(body.as_ref(), on_disk.as_slice());
    }

    /// The route half of the validator↔route agreement: a nested asset path
    /// is servable, which is why `DiscoveredProject::asset_files` walks
    /// `assets/` recursively. Change both together.
    #[test]
    fn serve_project_asset_returns_file_for_nested_path() {
        let response = fixture_assets().serve("alpha", "assets/covers/nested.webp");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn serve_project_asset_returns_404_for_unknown_project() {
        let response = fixture_assets().serve("no-such-project", "assets/cover.webp");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serve_project_asset_returns_404_for_missing_file() {
        let response = fixture_assets().serve("alpha", "assets/nonexistent.webp");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serve_project_asset_rejects_path_traversal() {
        let assets = fixture_assets();
        for path in [
            "assets/../config.kdl",
            "../alpha/assets/cover.webp",
            "assets/./cover.webp",
        ] {
            assert_eq!(assets.serve("alpha", path).status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    /// `config.kdl` and `data.sql` are in the project folder and not under
    /// `assets/`, so the inventory does not hold them.
    #[test]
    fn serve_project_asset_never_reaches_non_asset_project_files() {
        let assets = fixture_assets();
        for path in ["config.kdl", "data.sql", "assets", ""] {
            assert_eq!(assets.serve("alpha", path).status(), StatusCode::NOT_FOUND, "{path:?}");
        }
    }

    #[tokio::test]
    async fn assets_route_serves_project_cover_image() {
        let response = get("/_assets/alpha/assets/cover.webp").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_value(&response, header::CONTENT_TYPE), "image/webp");
    }

    /// The route decodes percent-escapes before the lookup, so each tail below
    /// reaches the inventory as `..`, a slash, a backslash, a NUL byte or an
    /// absolute path. None is a key. The first request shows that the route
    /// is reached at all.
    #[tokio::test]
    async fn assets_route_answers_404_to_every_tail_that_is_not_an_inventory_key() {
        assert_eq!(get("/_assets/alpha/assets/cover.webp").await.status(), StatusCode::OK);
        for uri in [
            "/_assets/alpha/assets/../config.kdl",
            "/_assets/alpha/../alpha/config.kdl",
            "/_assets/alpha/%2e%2e/alpha/assets/cover.webp",
            "/_assets/alpha/assets/%2e%2e/config.kdl",
            "/_assets/alpha/assets%2f..%2fconfig.kdl",
            "/_assets/alpha/..%2fconfig.kdl",
            "/_assets/alpha/assets%5c..%5cconfig.kdl",
            "/_assets/alpha/assets/..%5cconfig.kdl",
            "/_assets/alpha/assets/cover.webp%00",
            "/_assets/alpha/%00",
            "/_assets/alpha//etc/passwd",
            "/_assets/alpha/%2fetc%2fpasswd",
            "/_assets/alpha/config.kdl",
            "/_assets/%2e%2e/alpha/assets/cover.webp",
            "/_assets/..%2falpha/assets/cover.webp",
            "/_assets/no-such-project/assets/cover.webp",
            "/_assets/ALPHA/assets/cover.webp",
        ] {
            assert_eq!(get(uri).await.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }
}
