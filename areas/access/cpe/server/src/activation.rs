//! Loading the projects: what [`crate::Cpe::activate`] stores.
//!
//! Every project folder is loaded on its own, and whatever goes wrong with
//! one (a file, the projection, the build, validation, a panic) is logged
//! with its id and leaves that project unavailable: its prefix answers 503,
//! and the others are served. Nothing here returns an error or panics.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Instant;

use cpe_web::project::{LoadError, Project};

use crate::discovery::{self, DiscoveredProject};
use crate::{assets, request, router, CpeConfig, ProjectSummary};

/// What `cpe-core`'s `ReservedProjectId` check is given. The id rule already refuses every id that
/// starts with `_`, and that is what keeps a project off `/_static` and `/_assets`.
const RESERVED_IDS: &[&str] = &["_static"];

/// CPE once its projects are loaded: the router that serves them and the
/// list a host reads.
pub(crate) struct Activated {
    pub(crate) router: axum::Router,
    pub(crate) projects: Vec<ProjectSummary>,
}

/// Every project folder with a store built from `projection`'s snapshot of
/// the shortcode its `remodel` block declares. The projection is asked once
/// per project.
pub(crate) fn from_projection(config: &CpeConfig, projection: &dyn cpe_ports::ArchiveProjection) -> Activated {
    load_all(config, |discovered, config_src| {
        store_sql_from(projection, discovered, config_src)
    })
}

/// Every project folder that holds a `data.sql`, opened from that text.
pub(crate) fn from_sql_folders(config: &CpeConfig) -> Activated {
    load_all(config, |discovered, _| {
        if discovered.has_data_sql() {
            discovered.data_sql().map_err(|error| error.to_string())
        } else {
            Err("it has no data.sql".to_owned())
        }
    })
}

/// Load each project folder with the store text `store_sql` gives for it and
/// its config source.
///
/// "Not ready" is not "not there": a folder that is a project and does not
/// load is kept as unavailable, so that its prefix answers 503 and not 404.
fn load_all(config: &CpeConfig, store_sql: impl Fn(&DiscoveredProject, &str) -> Result<String, String>) -> Activated {
    let mut loaded = Vec::new();
    let mut served = Vec::new();
    let mut unavailable = Vec::new();
    for discovered in discovery::discover(&config.projects_dir) {
        let started = Instant::now();
        // A panic in a hook, the builder or an adapter must stay this project's.
        let outcome = catch_unwind(AssertUnwindSafe(|| load(&discovered, &store_sql)))
            .unwrap_or_else(|payload| Err(format!("loading it panicked: {}", panic_message(payload.as_ref()))));
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        match outcome {
            Ok(project) => {
                tracing::info!(project_id = discovered.id.as_str(), elapsed_ms, "project loaded");
                loaded.push(project);
                served.push(discovered);
            }
            Err(report) => {
                tracing::error!(
                    project_id = discovered.id.as_str(),
                    elapsed_ms,
                    report = report.as_str(),
                    "project does not load; its prefix answers 503"
                );
                unavailable.push(discovered.id);
            }
        }
    }
    tracing::info!(
        projects_dir = %config.projects_dir.display(),
        served = loaded.len(),
        unavailable = unavailable.len(),
        "CPE's projects are loaded"
    );

    let projects = loaded
        .iter()
        .map(|project| ProjectSummary {
            id: project.id().to_owned(),
            title: project.title(&cpe_core::lang::resolve(&[], &project.config().site)).to_owned(),
            url: format!("/{}/", project.id()),
        })
        .collect();
    let router = router::build(
        &loaded,
        &unavailable,
        &assets::AssetInventory::of(&served),
        request::OriginConfig::new(config.public_origin.as_deref()),
    )
    .layer(axum::middleware::from_fn(request::resolve_request_prefs));
    Activated { router, projects }
}

/// What a caught panic said, for the log.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic without a message")
}

/// The store text of one project, built from the projection: the snapshot of
/// the mapping's shortcode, built under the mapping with the hook registered
/// for the folder's name, and rendered as the SQL the store opens.
fn store_sql_from(
    projection: &dyn cpe_ports::ArchiveProjection,
    discovered: &DiscoveredProject,
    config_src: &str,
) -> Result<String, String> {
    let mapping = cpe_core::validate::load_remodel(config_src).map_err(|error| match error {
        cpe_core::validate::ConfigLoadError::Parse(error) => error.to_string(),
        cpe_core::validate::ConfigLoadError::Validation(report) => {
            report.render(config_src, &format!("{}/config.kdl", discovered.id))
        }
    })?;
    let snapshot = projection
        .snapshot(mapping.shortcode.as_str())
        .map_err(|error| error.to_string())?;
    tracing::info!(
        project_id = discovered.id.as_str(),
        shortcode = mapping.shortcode.as_str(),
        resources = snapshot.resources.len(),
        curated_values = snapshot.curation.len(),
        "snapshot read from the archive projection"
    );
    let hook = cpe_store_build::hooks::resolve(&discovered.id);
    let store = cpe_store_build::build(&snapshot, &mapping, hook).map_err(|errors| errors.to_string())?;
    let header = cpe_store_build::sql::header(&discovered.id, &mapping);
    cpe_store_build::sql::to_sql(&store, &mapping, &header).map_err(|error| error.to_string())
}

/// Read one project folder, get its store text and load it. The error is a
/// report ready to log.
fn load(
    discovered: &DiscoveredProject,
    store_sql: &impl Fn(&DiscoveredProject, &str) -> Result<String, String>,
) -> Result<Arc<Project>, String> {
    let config_src = discovered.config_source().map_err(|error| error.to_string())?;
    let narrative_files = discovered.narrative_files();
    let asset_files = discovered.asset_files();
    let narrative_contents = discovered.narrative_contents().map_err(|error| error.to_string())?;
    let data_sql = store_sql(discovered, &config_src)?;
    let ctx = cpe_core::validate::context::ValidationContext {
        project_id: &discovered.id,
        reserved_ids: RESERVED_IDS,
        narrative_files: &narrative_files,
        asset_files: &asset_files,
    };
    let project = match Project::load(&ctx, &config_src, &data_sql, &narrative_contents) {
        Ok(project) => project,
        // A validation report is rendered here, while the config source it points into is at hand.
        Err(LoadError::Validation(report)) => {
            return Err(report.render(&config_src, &format!("{}/config.kdl", discovered.id)));
        }
        Err(other) => return Err(other.to_string()),
    };
    for warning in project.warnings() {
        tracing::warn!(
            project_id = discovered.id.as_str(),
            kind = ?warning.kind,
            message = warning.message.as_str(),
            "validation warning"
        );
    }
    with_routes_that_register(project)
}

/// Refuse a project whose routes axum cannot register. `Router::route` panics on two routes it
/// cannot tell apart (`/books/:id` beside `/books/:name`), and validation does not rule that pair
/// out. The trial registration here runs the same code as the real one and turns the panic into a
/// refused project.
fn with_routes_that_register(project: Project) -> Result<Arc<Project>, String> {
    let project = Arc::new(project);
    catch_unwind(AssertUnwindSafe(|| {
        let _trial = router::build(
            std::slice::from_ref(&project),
            &[],
            &assets::AssetInventory::default(),
            request::OriginConfig::new(None),
        );
    }))
    .map_err(|_| "two of its routes are the same to the router".to_owned())?;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use axum::http::{header, StatusCode};
    use cpe_ports::{
        ArchiveProjection, ClassIri, DataArk, FakeArchiveProjection, ProjectSnapshot, ProjectionError, PropertyIri,
        Resource, ResourceIri, Value, ValueKind, DATA_ARK_PREFIX,
    };
    use sync_store::LiveArchiveProjection;

    use crate::tests::{assert_unavailable, config_for, fixture, get};
    use crate::{Cpe, ProjectSummary};

    /// The fixture project's config, whose `remodel` block declares shortcode `0001`.
    const THINGS: &str = include_str!("../tests/fixtures/activation/things/config.kdl");

    /// A snapshot of project `shortcode` with one Thing, `lamp`, named `name`.
    fn snapshot(shortcode: &str, name: &str) -> ProjectSnapshot {
        ProjectSnapshot {
            shortcode: shortcode.to_owned(),
            resources: vec![Resource {
                iri: ResourceIri(format!("http://rdfh.ch/{shortcode}/lamp")),
                ark: DataArk(format!("{DATA_ARK_PREFIX}{shortcode}/lamp")),
                class: ClassIri("http://www.knora.org/ontology/0001/things#Thing".to_owned()),
                label: name.to_owned(),
                values: vec![Value {
                    property: PropertyIri("http://www.knora.org/ontology/0001/things#hasName".to_owned()),
                    uuid: Some("v1".to_owned()),
                    kind: ValueKind::Text { text: name.to_owned(), lang: None },
                }],
                file: None,
                part_of: Vec::new(),
                seqnum: None,
                annotation: None,
            }],
            list_nodes: Vec::new(),
            curation: Vec::new(),
        }
    }

    /// A projects folder with one project per `(id, config)`.
    fn projects(configs: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("a temporary directory");
        for (id, config) in configs {
            let dir = root.path().join(id);
            std::fs::create_dir(&dir).expect("the project folder is created");
            std::fs::write(dir.join("config.kdl"), config).expect("config.kdl is written");
        }
        root
    }

    /// The fixture config under another shortcode.
    fn things_of(shortcode: &str) -> String {
        let config = THINGS.replace("shortcode \"0001\"", &format!("shortcode \"{shortcode}\""));
        assert_ne!(config, THINGS, "the fixture declares shortcode 0001");
        config
    }

    /// `uri` answered by `router`, which the test took from a `Cpe` earlier.
    async fn get_from(router: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        let request = axum::http::Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .expect("hand-crafted test request is well-formed");
        let response = tower::ServiceExt::oneshot(router.clone(), request)
            .await
            .expect("oneshot against an in-process router never fails");
        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        (parts.status, parts.headers, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn an_activated_project_serves_its_pages_through_a_router_taken_before() {
        let root = projects(&[("things", THINGS)]);
        let cpe = Cpe::new(&config_for(root.path()));
        let before = cpe.router();
        for uri in [
            "/",
            "/things/",
            "/things/things/lamp",
            "/_static/cpe/tokens.css",
            "/nope",
        ] {
            let (status, headers, _) = get_from(&before, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        assert!(cpe.projects().is_empty());

        // On a clone, as a host that moves one into `spawn_blocking` does.
        cpe.clone()
            .activate(&FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")));

        let (status, _, body) = get_from(&before, "/things/").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("Things Fixture"), "{body}");
        let (status, _, body) = get_from(&before, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("Brass lamp"), "{body}");
        let (status, _, _) = get_from(&before, "/things/things/no-such-thing").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _, _) = get_from(&before, "/_static/cpe/tokens.css").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            cpe.projects(),
            vec![ProjectSummary {
                id: "things".to_owned(),
                title: "Things Fixture".to_owned(),
                url: "/things/".to_owned(),
            }]
        );
    }

    /// A projection that panics, as a hook or an adapter with a bug would.
    struct PanickingProjection;

    impl ArchiveProjection for PanickingProjection {
        fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
            if shortcode == "0001" {
                Ok(snapshot("0001", "Brass lamp"))
            } else {
                panic!("the projection panics for {shortcode}")
            }
        }
    }

    /// A fake that also serves `self.1` for shortcode `0004`, whatever shortcode that snapshot
    /// has: the fake alone keys a snapshot by its own.
    struct Mismatching(FakeArchiveProjection, ProjectSnapshot);

    impl ArchiveProjection for Mismatching {
        fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
            if shortcode == "0004" {
                Ok(self.1.clone())
            } else {
                self.0.snapshot(shortcode)
            }
        }
    }

    /// Each cause of a project that does not activate is reported as that cause, so that the
    /// 503s of the test below are not five times one unrelated failure.
    #[test]
    fn a_store_that_cannot_be_built_is_reported_with_its_cause() {
        let delta = std::fs::read_to_string(fixture().join("delta/config.kdl")).expect("the fixture is readable");
        let root = projects(&[("things", THINGS)]);
        let discovered = crate::discovery::discover(root.path()).pop().expect("the folder is a project");
        let mismatched = ProjectSnapshot {
            shortcode: "0001".to_owned(),
            ..snapshot("0004", "Brass lamp")
        };
        let projection = Mismatching(
            FakeArchiveProjection::default()
                .with_project(snapshot("0001", "Brass lamp"))
                .with_unavailable("0003"),
            mismatched,
        );
        let report = |config: &str| super::store_sql_from(&projection, &discovered, config);

        assert!(report(THINGS).is_ok_and(|sql| sql.contains("Brass lamp")));
        for (config, cause) in [
            (things_of("0002"), "holds no project 0002"),
            (things_of("0003"), "is unavailable: simulated outage"),
            (things_of("0004"), "requested project 0004, but the snapshot is of 0001"),
            (delta, "has no 'remodel' block"),
            ("this is { not a project\n".to_owned(), "failed to parse config.kdl"),
        ] {
            let error = report(&config).expect_err("the store is not built");
            assert!(error.contains(cause), "{cause:?} is not in {error:?}");
        }
    }

    #[tokio::test]
    async fn a_project_that_does_not_activate_fails_alone() {
        let unknown = things_of("0002");
        let outage = things_of("0003");
        let other_project = things_of("0004");
        let delta = std::fs::read_to_string(fixture().join("delta/config.kdl")).expect("the fixture is readable");
        let root = projects(&[
            ("things", THINGS),
            ("unknown", &unknown),
            ("outage", &outage),
            ("mismatch", &other_project),
            ("no-remodel", &delta),
            ("broken", "this is { not a project\n"),
        ]);
        let cpe = Cpe::new(&config_for(root.path()));
        let mismatched = ProjectSnapshot {
            shortcode: "0001".to_owned(),
            ..snapshot("0004", "Brass lamp")
        };
        let projection = FakeArchiveProjection::default()
            .with_project(snapshot("0001", "Brass lamp"))
            .with_unavailable("0003");

        cpe.activate(&Mismatching(projection, mismatched));

        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        for id in ["unknown", "outage", "mismatch", "no-remodel", "broken"] {
            for uri in [format!("/{id}"), format!("/{id}/"), format!("/{id}/things/lamp")] {
                let (status, headers, _) = get(&cpe, &uri).await;
                assert_unavailable(status, &headers, &uri);
            }
        }
        for uri in ["/", "/nope", "/nope/things/lamp", "/UNKNOWN/"] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::RETRY_AFTER), "{uri}");
            assert!(body.contains("This address does not name a project."), "{uri}: {body}");
        }
        assert_eq!(cpe.projects().len(), 1);
    }

    #[tokio::test]
    async fn a_panic_while_one_project_activates_stops_neither_activate_nor_the_others() {
        let panicking = things_of("0009");
        let root = projects(&[("panics", &panicking), ("things", THINGS)]);
        let cpe = Cpe::new(&config_for(root.path()));

        cpe.activate(&PanickingProjection);

        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, headers, _) = get(&cpe, "/panics/").await;
        assert_unavailable(status, &headers, "/panics/");
    }

    #[tokio::test]
    async fn with_no_projects_folder_every_path_is_cpes_not_found_once_activated() {
        let cpe = Cpe::new(&config_for(&fixture().join("no-such-folder")));
        let (status, headers, _) = get(&cpe, "/things/").await;
        assert_unavailable(status, &headers, "before activate");

        cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")));

        assert!(cpe.projects().is_empty());
        for uri in ["/", "/things/", "/incunabula/books"] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::RETRY_AFTER), "{uri}");
            assert!(body.contains("This address does not name a project."), "{uri}: {body}");
        }
    }

    #[tokio::test]
    async fn a_refused_configuration_is_not_activated() {
        let root = projects(&[("things", THINGS)]);
        let config = crate::CpeConfig {
            public_origin: Some("https://repository.dasch.swiss/cpe".to_owned()),
            ..config_for(root.path())
        };
        let cpe = Cpe::new(&config);
        cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")));
        for uri in ["/", "/things/", "/_static/cpe/tokens.css"] {
            let (status, headers, _) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }
    }

    /// The store is a read model: it is rebuilt from empty from what the port serves, holds
    /// nothing else, and follows the port (ADR-0008).
    #[tokio::test]
    async fn the_store_is_rebuilt_from_the_port_alone_and_follows_it() {
        let root = projects(&[("things", THINGS)]);
        let uris = ["/things/", "/things/things/lamp"];
        let pages_of = |name: &'static str| {
            let config = config_for(root.path());
            async move {
                let cpe = Cpe::new(&config);
                cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", name)));
                let mut pages = Vec::new();
                for uri in uris {
                    let (status, _, body) = get(&cpe, uri).await;
                    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
                    pages.push(body);
                }
                pages
            }
        };

        let first = pages_of("Brass lamp").await;
        let second = pages_of("Brass lamp").await;
        assert_eq!(first, second, "two builds from one snapshot serve the same pages");
        assert!(first[1].contains("Brass lamp"), "{}", first[1]);

        let changed = pages_of("Copper kettle").await;
        assert!(changed[1].contains("Copper kettle"), "{}", changed[1]);
        assert!(!changed[1].contains("Brass lamp"), "{}", changed[1]);
    }

    #[tokio::test]
    async fn a_second_activation_replaces_what_is_served() {
        let root = projects(&[("things", THINGS)]);
        let cpe = Cpe::new(&config_for(root.path()));
        cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")));
        cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", "Copper kettle")));
        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Copper kettle") && !body.contains("Brass lamp"), "{body}");
    }

    /// The committed project folders, and `sync`'s committed data.
    fn committed_projects() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../projects")
    }

    fn sync_data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sync/data")
    }

    /// Incunabula as the area binary serves it: its committed folder, and its store built from
    /// `sync`'s adapter over the committed 0803 data.
    #[tokio::test]
    async fn incunabula_activates_from_the_committed_data_and_serves_its_pages() {
        let cpe = Cpe::new(&config_for(&committed_projects()));
        cpe.activate(&LiveArchiveProjection::new(sync_data()));

        assert_eq!(
            cpe.projects().iter().map(|project| project.id.as_str()).collect::<Vec<_>>(),
            ["incunabula"]
        );
        for (uri, marker) in INCUNABULA_PAGES {
            let (status, _, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert!(body.contains(marker), "{uri} does not show {marker:?}");
        }
    }

    /// The start page, a book's page and a border strip's record page, each with its heading. The
    /// last two headings are values of the committed data.
    const INCUNABULA_PAGES: [(&str, &str); 3] = [
        ("/incunabula/", "<h1>Die Bilderfolgen der Basler Frühdrucke"),
        (
            "/incunabula/books/bereitung",
            "<h1 class=\"record__title\">Bereitung zu dem Heiligen Sakrament",
        ),
        ("/incunabula/strips/l-10", "<h1 class=\"record__title\">l_10"),
    ];

    /// The length and the SHA-256 of the store text built for Incunabula. They are those of
    /// `dsp-incubator cpe/projects/incunabula/data.sql`, the file the prototype embeds, so the
    /// store opened here holds what the prototype's does. That file is not in this repository.
    ///
    /// The text follows the committed 0803 data, Incunabula's `remodel` block and the builder.
    /// When one of them changes it on purpose, put the two values the failure prints here.
    const INCUNABULA_STORE_BYTES: usize = 5_631_851;
    const INCUNABULA_STORE_SHA256: &str = "6725f22de15d734ca7d8c8ec79bafae3d53cd33181e51ae1587f33ac6784ec49";

    #[test]
    fn incunabulas_store_text_is_the_one_the_prototype_embeds() {
        let discovered = crate::discovery::discover(&committed_projects())
            .into_iter()
            .find(|project| project.id == "incunabula")
            .expect("Incunabula's folder is committed");
        let config = discovered.config_source().expect("Incunabula's config.kdl is readable");
        let sql = super::store_sql_from(&LiveArchiveProjection::new(sync_data()), &discovered, &config)
            .expect("Incunabula's store builds from the committed 0803 data");
        let sha256 = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(sql.as_bytes()));
        assert_eq!((sql.len(), sha256.as_str()), (INCUNABULA_STORE_BYTES, INCUNABULA_STORE_SHA256));
    }

    #[tokio::test]
    async fn without_the_data_directory_incunabula_answers_503_on_its_prefix() {
        let cpe = Cpe::new(&config_for(&committed_projects()));
        cpe.activate(&LiveArchiveProjection::new(sync_data().join("no-such-folder")));

        assert!(cpe.projects().is_empty());
        for uri in ["/incunabula/", "/incunabula/books"] {
            let (status, headers, _) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        let (status, _, _) = get(&cpe, "/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
