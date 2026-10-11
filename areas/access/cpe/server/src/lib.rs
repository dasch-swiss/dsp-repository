//! `cpe-server`: CPE's configuration, its router and the activation of its projects.
//!
//! This crate is a library only; a composition root owns `main`, the listener
//! and the tracing layers, mounts [`Cpe::router`] and calls [`Cpe::activate`].
//! Nothing here may name a type of `cpe-core`, `cpe-store`, `cpe-store-build`,
//! `cpe-web`, `cpe-components` or `cpe-image-viewer` in a `pub` signature:
//! that boundary is what lets a host depend on this crate and `cpe-ports`
//! alone. `tests::no_pub_item_names_another_cpe_crate` holds it, and relies on
//! this file and `config.rs` importing no CPE crate with `use`.
//!
//! CPE never stops its host. Nothing here fails or panics on what it finds: a
//! configuration that is refused, a projects folder that is missing, a
//! projection that cannot be read and a project that does not load are logged
//! and answered over HTTP (see `unavailable`).
//!
//! A project's store comes from the port and from nowhere else (ADR-0008): `activation` is where
//! it is built, and [`Cpe::activate`] is the one way in that a host is meant to use.

mod activation;
mod assets;
mod config;
mod discovery;
mod request;
mod router;
mod static_files;
mod unavailable;

use std::sync::{Arc, PoisonError, RwLock};

pub use config::CpeConfig;

/// One project CPE serves, as a host needs it to link there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    /// The project id: its folder's name and its URL segment.
    pub id: String,
    /// `site.title`, in the project's default language.
    pub title: String,
    /// The path of the project's start page.
    pub url: String,
}

/// CPE, built from its config. Cheap to clone, and every clone and every
/// router taken from one shares what [`Cpe::activate`] stores.
#[derive(Clone)]
pub struct Cpe {
    config: Arc<CpeConfig>,
    /// `None` until the projects are loaded. A guard is held only to read or
    /// replace the `Arc`, never across an `.await`.
    active: Arc<RwLock<Option<Arc<activation::Activated>>>>,
}

impl Cpe {
    /// CPE with no project loaded: every path is answered 503 until
    /// [`Cpe::activate`] has run. Does no I/O and cannot fail. A
    /// configuration that is refused is logged here, and then `activate`
    /// loads nothing.
    #[must_use]
    pub fn new(config: &CpeConfig) -> Self {
        if let Some(reason) = config.refusal() {
            tracing::error!(
                reason,
                "CPE's configuration is refused and CPE is unavailable: where its router is mounted, every path answers 503"
            );
        } else if config.derives_origin() {
            tracing::info!(
                "CPE_PUBLIC_ORIGIN is not set: the origin of CPE's absolute URLs is derived per request \
                 from X-Forwarded-Proto and Host"
            );
        }
        Self { config: Arc::new(config.clone()), active: Arc::default() }
    }

    /// Load every project folder, each with a store built from
    /// `projection`'s snapshot of its shortcode, and serve them from then on.
    ///
    /// Blocking: it reads the projects folder and asks the projection once
    /// per project, so an async host calls it inside `spawn_blocking`, after
    /// its listener is bound. It returns nothing and never panics. A project
    /// that does not load is logged with its id and answers 503 on its
    /// prefix, and the others are served; a path that names no project is
    /// CPE's not-found. A later call loads everything again and replaces
    /// what is served; if it panics outside a project's own load, what was
    /// served before stays.
    ///
    /// "Never panics" rests on unwinding: a binary built with
    /// `panic = "abort"` loses it.
    pub fn activate(&self, projection: &dyn cpe_ports::ArchiveProjection) {
        if self.config.refusal().is_some() {
            return;
        }
        let config = &self.config;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| activation::from_projection(config, projection)))
        {
            Ok(activated) => self.store(activated),
            Err(payload) => tracing::error!(
                panic = activation::panic_message(payload.as_ref()),
                "CPE's activation panicked; what was served before is still served"
            ),
        }
    }

    /// CPE over the project folders that hold a `data.sql`, each opened from
    /// that SQL text. Blocking: it reads the projects folder.
    ///
    /// For the incubator's host and for tests only. Removable once no project
    /// ships a `data.sql` (DEV-7498).
    ///
    /// A folder without a `data.sql`, and a project that does not load, are
    /// answered 503 on their prefix; the others are served.
    #[doc(hidden)]
    #[must_use]
    pub fn from_sql_folders(config: &CpeConfig) -> Self {
        let cpe = Self::new(config);
        if config.refusal().is_none() {
            cpe.store(activation::from_sql_folders(config));
        }
        cpe
    }

    fn store(&self, activated: activation::Activated) {
        let activated = Some(Arc::new(activated));
        let replaced = std::mem::replace(&mut *self.active.write().unwrap_or_else(PoisonError::into_inner), activated);
        // The projects served before are dropped here, after the guard: requests do not wait for
        // it.
        drop(replaced);
    }

    fn activated(&self) -> Option<Arc<activation::Activated>> {
        self.active.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// CPE's routes: the shared static files, each project's assets and
    /// pages, and CPE's own fallback. It carries no tracing layer.
    ///
    /// The router has one service, a fallback that answers every path and
    /// method: 503 until the projects are loaded, and the loaded projects'
    /// router from then on, whenever this router was taken. Because it has a
    /// fallback of its own, a host that nests it keeps its own fallback for
    /// its own paths only.
    pub fn router(&self) -> axum::Router {
        let cpe = self.clone();
        axum::Router::new().fallback(move |request: axum::extract::Request| {
            let activated = cpe.activated();
            async move {
                match activated {
                    Some(activated) => {
                        // A router's error type is `Infallible`.
                        let Ok(response) = tower::ServiceExt::oneshot(activated.router.clone(), request).await;
                        response
                    }
                    None => unavailable::response(),
                }
            }
        })
    }

    /// The projects CPE serves, sorted by id; empty until they are loaded. A
    /// host that wants a list of them renders it from this.
    #[must_use]
    pub fn projects(&self) -> Vec<ProjectSummary> {
        self.activated().map(|activated| activated.projects.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    use super::*;

    pub(crate) fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery")
    }

    pub(crate) fn config_for(projects_dir: &Path) -> CpeConfig {
        CpeConfig {
            projects_dir: projects_dir.to_owned(),
            ..CpeConfig::default()
        }
    }

    pub(crate) async fn get(cpe: &Cpe, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        let response = cpe
            .router()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        (parts.status, parts.headers, String::from_utf8_lossy(&body).into_owned())
    }

    pub(crate) fn assert_unavailable(status: StatusCode, headers: &axum::http::HeaderMap, what: &str) {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{what}");
        assert!(headers.contains_key(header::RETRY_AFTER), "{what}: Retry-After");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{what}");
    }

    /// A projects folder with a copy of the `delta` fixture, a project whose
    /// config does not validate and a folder without a `data.sql`.
    fn mixed_projects() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("a temporary directory");
        let good = root.path().join("delta");
        std::fs::create_dir(&good).expect("the project folder is created");
        for file in ["config.kdl", "data.sql"] {
            std::fs::copy(fixture().join("delta").join(file), good.join(file)).expect("the fixture file is copied");
        }
        let broken = root.path().join("broken");
        std::fs::create_dir(&broken).expect("the project folder is created");
        std::fs::write(broken.join("config.kdl"), "this is { not a project\n").expect("config.kdl is written");
        std::fs::write(broken.join("data.sql"), "").expect("data.sql is written");
        let no_data = root.path().join("no-data");
        std::fs::create_dir(&no_data).expect("the project folder is created");
        std::fs::copy(fixture().join("gamma/config.kdl"), no_data.join("config.kdl"))
            .expect("the fixture file is copied");
        root
    }

    #[tokio::test]
    async fn before_anything_is_loaded_every_path_and_method_is_answered_503() {
        let cpe = Cpe::new(&config_for(&fixture()));
        assert!(cpe.projects().is_empty());
        for uri in [
            "/",
            "/delta/",
            "/_static/cpe/tokens.css",
            "/_assets/alpha/assets/cover.webp",
            "/nope",
        ] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
            assert!(!body.contains("nope"), "the 503 page names no request path: {body}");
        }
        let post = Request::builder()
            .method("POST")
            .uri("/anything/at/all")
            .body(Body::empty())
            .expect("hand-crafted test request is well-formed");
        let response = cpe
            .router()
            .oneshot(post)
            .await
            .expect("oneshot against an in-process router never fails");
        assert_unavailable(response.status(), response.headers(), "POST");
    }

    #[tokio::test]
    async fn a_loaded_project_is_served_and_a_failed_one_answers_503_on_its_prefix() {
        let root = mixed_projects();
        let cpe = Cpe::from_sql_folders(&config_for(root.path()));

        let (status, _, body) = get(&cpe, "/delta/").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("Delta Fixture"), "{body}");

        for uri in ["/broken", "/broken/", "/broken/any/page", "/no-data/", "/no-data/x?y=1"] {
            let (status, headers, _) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }

        let (status, headers, _) = get(&cpe, "/delta/no-such-page").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!headers.contains_key(header::LOCATION));
    }

    #[tokio::test]
    async fn a_path_that_names_no_project_is_cpes_not_found() {
        let root = mixed_projects();
        let cpe = Cpe::from_sql_folders(&config_for(root.path()));
        for uri in [
            "/",
            "/nope",
            "/nope/deeper",
            "/BROKEN/",
            "/%62roken/",
            "//evil.example/x",
            "/brokenx/",
        ] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
            assert!(body.contains("This address does not name a project."), "{uri}: {body}");
            assert!(!body.contains("evil.example"), "{uri} must not be echoed: {body}");
        }
    }

    #[tokio::test]
    async fn a_missing_projects_folder_leaves_every_path_not_found() {
        let cpe = Cpe::from_sql_folders(&config_for(&fixture().join("no-such-folder")));
        assert!(cpe.projects().is_empty());
        for uri in ["/", "/delta/", "/incunabula/books"] {
            let (status, _, _) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
        let (status, _, _) = get(&cpe, "/_static/cpe/tokens.css").await;
        assert_eq!(status, StatusCode::OK, "the shared static files need no project");
    }

    #[tokio::test]
    async fn a_refused_configuration_answers_503_and_loads_nothing() {
        let config = CpeConfig {
            public_origin: Some("https://repository.dasch.swiss/cpe".to_owned()),
            ..config_for(&fixture())
        };
        for cpe in [Cpe::new(&config), Cpe::from_sql_folders(&config)] {
            assert!(cpe.projects().is_empty());
            for uri in ["/", "/delta/", "/_static/cpe/tokens.css"] {
                let (status, headers, _) = get(&cpe, uri).await;
                assert_unavailable(status, &headers, uri);
            }
        }
    }

    #[test]
    fn projects_lists_the_served_projects_with_their_urls() {
        let root = mixed_projects();
        let cpe = Cpe::from_sql_folders(&config_for(root.path()));
        assert_eq!(
            cpe.projects(),
            vec![ProjectSummary {
                id: "delta".to_owned(),
                title: "Delta Fixture".to_owned(),
                url: "/delta/".to_owned(),
            }]
        );
    }

    #[tokio::test]
    async fn the_request_preferences_reach_the_handlers() {
        let root = mixed_projects();
        let cpe = Cpe::from_sql_folders(&config_for(root.path()));
        let (status, headers, _) = get(&cpe, "/delta/?lang=en").await;
        assert_eq!(status, StatusCode::OK);
        let cookie = headers.get(header::SET_COOKIE).expect("an explicit ?lang= sets the cookie");
        assert!(
            cookie.to_str().expect("the cookie is ASCII").starts_with("lang=en"),
            "{cookie:?}"
        );
    }

    #[tokio::test]
    async fn the_image_viewer_route_is_not_shadowed_by_the_static_table() {
        let cpe = Cpe::from_sql_folders(&config_for(&fixture().join("no-such-folder")));
        let uri = format!("{}/image-viewer.js", cpe_image_viewer::ASSET_PREFIX);
        let (status, _, _) = get(&cpe, &uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
    }

    /// Two pages whose routes axum cannot tell apart. `Router::route` panics on such a pair, so the
    /// project must be refused before its routes are registered.
    #[tokio::test]
    async fn a_project_whose_routes_collide_is_unavailable_and_stops_nothing() {
        let root = mixed_projects();
        let clash = root.path().join("clash");
        std::fs::create_dir(&clash).expect("the project folder is created");
        let config = std::fs::read_to_string(fixture().join("delta/config.kdl")).expect("the fixture is readable");
        let second = "page id=\"other\" route=\"/characters/:name\" {\n    resource-detail id=\"$name\" title-property=\"name\"\n}\n";
        std::fs::write(clash.join("config.kdl"), format!("{config}\n{second}")).expect("config.kdl is written");
        std::fs::copy(fixture().join("delta/data.sql"), clash.join("data.sql")).expect("the fixture file is copied");

        let cpe = Cpe::from_sql_folders(&config_for(root.path()));
        let (status, _, _) = get(&cpe, "/delta/").await;
        assert_eq!(status, StatusCode::OK, "the other project is served");
        let (status, headers, _) = get(&cpe, "/clash/").await;
        assert_unavailable(status, &headers, "/clash/");
    }

    /// The names of the other CPE crates. `cpe_ports` is not among them: its
    /// types are the one thing of CPE a host may see beside this crate's own.
    const OTHER_CPE_CRATES: &[&str] = &[
        "cpe_core",
        "cpe_store",
        "cpe_store_build",
        "cpe_web",
        "cpe_components",
        "cpe_image_viewer",
    ];

    /// `source` up to its test module, without comment lines.
    fn production_lines(source: &str) -> Vec<&str> {
        source
            .lines()
            .take_while(|line| !line.starts_with("#[cfg(test)]"))
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect()
    }

    /// Every `pub` item of a file, each as the text from `pub` to the `{` or
    /// `;` that ends its signature.
    fn pub_signatures(lines: &[&str]) -> Vec<String> {
        let mut signatures = Vec::new();
        let mut current: Option<String> = None;
        for line in lines {
            let trimmed = line.trim();
            if current.is_none() && trimmed.starts_with("pub ") {
                current = Some(String::new());
            }
            if let Some(signature) = current.as_mut() {
                signature.push_str(trimmed);
                signature.push(' ');
                let is_field = !signature.starts_with("pub fn ") && signature.contains(':');
                if trimmed.ends_with('{') || trimmed.ends_with(';') || (is_field && trimmed.ends_with(',')) {
                    signatures.extend(current.take());
                }
            }
        }
        signatures
    }

    /// A text check over the two files that hold every `pub` item of the
    /// crate. It is sound because neither imports another CPE crate with
    /// `use` (asserted below), so a signature can only name such a type by
    /// its full path.
    #[test]
    fn no_pub_item_names_another_cpe_crate() {
        let lib = include_str!("lib.rs");
        let config = include_str!("config.rs");

        let lib_lines = production_lines(lib);
        for line in &lib_lines {
            let line = line.trim();
            assert!(
                !line.starts_with("pub mod "),
                "a public module would publish all its items: {line}"
            );
            assert!(
                !line.starts_with("pub use ") || line == "pub use config::CpeConfig;",
                "a re-export this test does not read: {line}"
            );
        }

        let mut signatures = 0;
        for (file, lines) in [("lib.rs", lib_lines), ("config.rs", production_lines(config))] {
            for line in &lines {
                let line = line.trim();
                let is_use = line.starts_with("use ") || (line.starts_with("pub") && line.contains(" use "));
                assert!(
                    !(is_use && line.contains("cpe_")),
                    "{file} imports a CPE crate, which hides its types from this test: {line}"
                );
                let is_trait_impl = line.starts_with("impl") && line.contains(" for ");
                assert!(
                    !(is_trait_impl && OTHER_CPE_CRATES.iter().any(|name| line.contains(name))),
                    "{file}: a trait impl is public API and names another CPE crate: {line}"
                );
            }
            for signature in pub_signatures(&lines) {
                signatures += 1;
                for name in OTHER_CPE_CRATES {
                    assert!(!signature.contains(name), "{file}: a pub item names {name}: {signature}");
                }
            }
        }
        assert!(signatures > 0, "the pub items were not found");
    }

    #[test]
    fn the_signature_reader_sees_a_forbidden_type() {
        let lines = [
            "impl Cpe {",
            "    pub fn project(",
            "        &self,",
            "    ) -> cpe_web::project::Project {",
            "    }",
            "}",
        ];
        let signatures = pub_signatures(&lines);
        assert_eq!(signatures.len(), 1);
        assert!(signatures[0].contains("cpe_web"), "{signatures:?}");
    }
}
