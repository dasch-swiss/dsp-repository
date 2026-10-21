//! Loading the projects: what [`crate::Cpe::activate`] stores.
//!
//! Every project folder is loaded on its own, and whatever goes wrong with
//! one (a file, a port, the build, validation, a panic) is logged with its id
//! and leaves that project unavailable: its prefix answers 503, and the others
//! are served. Nothing here returns an error or panics.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Instant;

use cpe_ports::{ArchiveProjection, DepositedFiles, FilesError};
use cpe_store_build::CuratedValues;
use cpe_web::project::{LoadError, Project};

use crate::discovery::{self, DiscoveredProject};
use crate::{assets, request, router, CpeConfig, ProjectSummary};

/// What `cpe-core`'s `ReservedProjectId` check is given. The id rule already refuses every id that
/// starts with `_`, and that is what keeps a project off CPE's own `_static` directory.
const RESERVED_IDS: &[&str] = &["_static"];

/// CPE once its projects are loaded: the router that serves them and lists
/// them at its root, and the list a host reads.
pub(crate) struct Activated {
    pub(crate) router: axum::Router,
    pub(crate) projects: Vec<ProjectSummary>,
}

/// What one project is loaded from, as its source gives it.
struct Sources {
    /// The text of the project's config.
    config: String,
    /// The name the config's diagnostics give its file.
    config_file: String,
    /// The project's narrative files, as `(bare name, content)`.
    narrative: Vec<(String, String)>,
    /// Whether `narrative` is the project's whole narrative, as
    /// `ValidationContext::narrative_files_complete` takes it.
    narrative_complete: bool,
    /// The store's SQL text.
    store_sql: String,
}

/// Every project folder with a `SHORTCODE`, loaded from the two ports: its
/// config, narrative and curation from `files`, and the snapshot of its
/// shortcode from `projection`, each asked once per project.
pub(crate) fn from_ports(
    config: &CpeConfig,
    projection: &dyn ArchiveProjection,
    files: &dyn DepositedFiles,
) -> Activated {
    let discovery = discovery::discover_for_activation(&config.projects_dir);
    for skipped in &discovery.skipped {
        // Such a folder is not a project, so its name is answered 404 and not 503. A folder
        // without a `SHORTCODE` at all is no project folder and is not logged.
        if skipped.reason != discovery::SkipReason::NoShortcode {
            tracing::warn!(
                folder = skipped.folder.as_str(),
                reason = ?skipped.reason,
                "a project folder is not served"
            );
        }
    }
    load_all(config, discovery.projects, |discovered| {
        port_sources(projection, files, discovered)
    })
}

/// Every project folder that holds a `data.sql`, opened from that text.
pub(crate) fn from_sql_folders(config: &CpeConfig) -> Activated {
    load_all(config, discovery::discover_sql_folders(&config.projects_dir), |discovered| {
        let config = discovered.config_source().map_err(|error| error.to_string())?;
        let narrative = discovered.narrative_contents().map_err(|error| error.to_string())?;
        let store_sql = if discovered.has_data_sql() {
            discovered.data_sql().map_err(|error| error.to_string())?
        } else {
            return Err("it has no data.sql".to_owned());
        };
        Ok(Sources {
            config,
            config_file: format!("{}/config.kdl", discovered.id),
            narrative,
            narrative_complete: false,
            store_sql,
        })
    })
}

/// Load each of `discovered` from what `sources` gives for it.
///
/// "Not ready" is not "not there": a folder that is a project and does not
/// load is kept as unavailable, so that its prefix answers 503 and not 404.
fn load_all(
    config: &CpeConfig,
    discovered: Vec<DiscoveredProject>,
    sources: impl Fn(&DiscoveredProject) -> Result<Sources, String>,
) -> Activated {
    let mut loaded = Vec::new();
    let mut served = Vec::new();
    let mut unavailable = Vec::new();
    for discovered in discovered {
        let started = Instant::now();
        // A panic in a hook, the builder or an adapter must stay this project's.
        let outcome = catch_unwind(AssertUnwindSafe(|| load(&discovered, &sources)))
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

    let base = config.base_or_root();
    let projects = loaded
        .iter()
        .map(|project| ProjectSummary {
            id: project.id().to_owned(),
            title: project.title(&cpe_core::lang::resolve(&[], &project.config().site)).to_owned(),
            url: base.project(project.id()).home(),
        })
        .collect();
    let router = router::build(
        &loaded,
        &unavailable,
        &assets::AssetInventory::of(&served),
        request::OriginConfig::new(config.public_origin.as_deref()),
        &base,
    )
    .layer(axum::middleware::from_fn(request::resolve_request_prefs))
    .layer(axum::middleware::from_fn_with_state(
        base.clone(),
        request::report_matched_route,
    ));
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

/// The file `name` of project `shortcode`, `None` when the project holds no such file. Any other
/// failure names the file by its place in `media`, `<shortcode>/<name>`.
fn deposited(files: &dyn DepositedFiles, shortcode: &str, name: &str) -> Result<Option<Vec<u8>>, String> {
    match files.file(shortcode, name) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(FilesError::UnknownFile { .. }) => Ok(None),
        Err(error) => Err(format!("{shortcode}/{name}: {error}")),
    }
}

/// `bytes` of the deposited file `file` as text.
fn utf8(bytes: Vec<u8>, file: &str) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| format!("{file} is not valid UTF-8"))
}

/// One project's sources from the ports. The narrative inventory is complete, so a file the
/// config names and the port does not hold is `NarrativeFileNotFound`.
fn port_sources(
    projection: &dyn ArchiveProjection,
    files: &dyn DepositedFiles,
    discovered: &DiscoveredProject,
) -> Result<Sources, String> {
    if let Some(refusal) = &discovered.refusal {
        return Err(refusal.clone());
    }
    let Some(shortcode) = discovered.shortcode.as_deref() else {
        return Err("its folder declares no shortcode".to_owned());
    };
    let config_file = format!("{shortcode}/config.kdl");
    let config = deposited(files, shortcode, "config.kdl")?
        .ok_or_else(|| format!("{config_file}: project {shortcode} deposited no config.kdl"))?;
    let config = utf8(config, &config_file)?;
    let mapping = cpe_core::validate::load_remodel(&config).map_err(|error| match error {
        cpe_core::validate::ConfigLoadError::Parse(error) => error.to_string(),
        cpe_core::validate::ConfigLoadError::Validation(report) => report.render(&config, &config_file),
    })?;
    if mapping.shortcode.as_str() != shortcode {
        return Err(format!(
            "{config_file} declares shortcode {}, but the folder's SHORTCODE is {shortcode}",
            mapping.shortcode.as_str()
        ));
    }

    let named = cpe_core::validate::named_narratives(&config).map_err(|error| error.to_string())?;
    let mut narrative: Vec<(String, String)> = Vec::new();
    let mut asked: Vec<String> = Vec::new();
    for name in named.candidates() {
        // Validation refuses such a name as `InvalidNarrativePath`; it is never asked for. A name
        // can recur as a variant of another file, and is asked for once.
        if name.contains(['/', '\\']) || asked.contains(&name) {
            continue;
        }
        asked.push(name.clone());
        let file = format!("narrative/{name}");
        if let Some(bytes) = deposited(files, shortcode, &file)? {
            let content = utf8(bytes, &format!("{shortcode}/{file}"))?;
            narrative.push((name, content));
        }
    }

    let curation = deposited(files, shortcode, "curation.csv")?;
    if curation.is_none() && mapping.reads_curation() {
        return Err(format!(
            "{shortcode}/curation.csv: project {shortcode} deposited no curation file, and its remodel block \
             reads curated values"
        ));
    }
    let snapshot = projection.snapshot(shortcode).map_err(|error| error.to_string())?;
    let curated = match curation {
        Some(bytes) => CuratedValues::parse(&bytes, &snapshot)
            .map_err(|fault| format!("{shortcode}/curation.csv is not a valid curation file at {fault}"))?,
        None => CuratedValues::none(),
    };
    tracing::info!(
        project_id = discovered.id.as_str(),
        shortcode,
        resources = snapshot.resources.len(),
        curated_values = curated.len(),
        narrative_files = narrative.len(),
        "read from the archive projection and the deposited files"
    );
    let hook = cpe_store_build::hooks::resolve(&discovered.id);
    let store = cpe_store_build::build(&snapshot, &curated, &mapping, hook).map_err(|errors| errors.to_string())?;
    let header = cpe_store_build::sql::header(&discovered.id, &mapping);
    let store_sql = cpe_store_build::sql::to_sql(&store, &mapping, &header).map_err(|error| error.to_string())?;
    Ok(Sources {
        config,
        config_file,
        narrative,
        narrative_complete: true,
        store_sql,
    })
}

/// Get one project's sources and load it. The error is a report ready to log.
fn load(
    discovered: &DiscoveredProject,
    sources: &impl Fn(&DiscoveredProject) -> Result<Sources, String>,
) -> Result<Arc<Project>, String> {
    let sources = sources(discovered)?;
    let narrative_files: Vec<String> = sources.narrative.iter().map(|(name, _)| name.clone()).collect();
    let asset_files = discovered.asset_files();
    let ctx = cpe_core::validate::context::ValidationContext {
        project_id: &discovered.id,
        reserved_ids: RESERVED_IDS,
        narrative_files: &narrative_files,
        narrative_files_complete: sources.narrative_complete,
        asset_files: &asset_files,
    };
    let project = match Project::load(&ctx, &sources.config, &sources.store_sql, &sources.narrative) {
        Ok(project) => project,
        // A validation report is rendered here, while the config source it points into is at hand.
        Err(LoadError::Validation(report)) => {
            return Err(report.render(&sources.config, &sources.config_file));
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
            &cpe_core::url::ROOT,
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
        ArchiveProjection, ClassIri, DataArk, FakeArchiveProjection, FakeDepositedFiles, ProjectSnapshot,
        ProjectionError, PropertyIri, Resource, ResourceIri, Value, ValueKind, DATA_ARK_PREFIX,
    };
    use media_store::LiveDepositedFiles;
    use sync_store::LiveArchiveProjection;

    use crate::discovery::DiscoveredProject;
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
        }
    }

    /// A projects folder with one project per `(id, shortcode)`, each a folder holding its
    /// `SHORTCODE`.
    fn projects(projects: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("a temporary directory");
        for (id, shortcode) in projects {
            let dir = root.path().join(id);
            std::fs::create_dir(&dir).expect("the project folder is created");
            std::fs::write(dir.join("SHORTCODE"), format!("{shortcode}\n")).expect("SHORTCODE is written");
        }
        root
    }

    /// Deposited files holding each `(shortcode, config)` as that project's `config.kdl`.
    fn configs(configs: &[(&str, &str)]) -> FakeDepositedFiles {
        configs
            .iter()
            .fold(FakeDepositedFiles::default(), |files, (shortcode, config)| {
                files.with_file(shortcode, "config.kdl", *config)
            })
    }

    /// The one project of a projects folder whose folder `things` declares `shortcode`, as
    /// activation discovers it.
    fn discovered(shortcode: &str) -> (tempfile::TempDir, DiscoveredProject) {
        let root = projects(&[("things", shortcode)]);
        let project = crate::discovery::discover_for_activation(root.path())
            .projects
            .pop()
            .expect("the folder is a project");
        (root, project)
    }

    /// The fixture config under another shortcode.
    fn things_of(shortcode: &str) -> String {
        let config = THINGS.replace("shortcode \"0001\"", &format!("shortcode \"{shortcode}\""));
        assert_ne!(config, THINGS, "the fixture declares shortcode 0001");
        config
    }

    /// The fixture config under `shortcode`, with `body` as its home page's body.
    fn things_with_home(shortcode: &str, body: &str) -> String {
        let things = if shortcode == "0001" {
            THINGS.to_owned()
        } else {
            things_of(shortcode)
        };
        let config = things.replace(
            "page id=\"home\" route=\"/\"\n",
            &format!("page id=\"home\" route=\"/\" {{\n{body}\n}}\n"),
        );
        assert!(config.contains(body), "the fixture has a home page without a body");
        config
    }

    /// The fixture config under `shortcode`, with a home page that shows `home.md`.
    fn things_with_narrative(shortcode: &str) -> String {
        things_with_home(shortcode, "    narrative file=\"home.md\"")
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

    /// The route template the answer to `uri` reports, if it reports one.
    async fn matched_route(router: &axum::Router, uri: &str) -> Option<String> {
        let request = axum::http::Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .expect("hand-crafted test request is well-formed");
        let response = tower::ServiceExt::oneshot(router.clone(), request)
            .await
            .expect("oneshot against an in-process router never fails");
        response
            .extensions()
            .get::<crate::MatchedRoute>()
            .map(|route| route.as_str().to_owned())
    }

    #[tokio::test]
    async fn an_activated_project_serves_its_pages_through_a_router_taken_before() {
        let root = projects(&[("things", "0001")]);
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
        cpe.clone().activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &configs(&[("0001", THINGS)]),
        );

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
            matched_route(&before, "/things/things/lamp").await.as_deref(),
            Some("/things/things/{id}")
        );
        assert_eq!(
            matched_route(&before, "/things/things/no-such-thing").await.as_deref(),
            Some("/things/things/{id}")
        );
        assert_eq!(matched_route(&before, "/").await.as_deref(), Some("/"));
        assert_eq!(matched_route(&before, "/things/no-such-page").await, None);
        assert_eq!(matched_route(&before, "/no-such-project/").await, None);
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

    /// The projection of the cases below: `0001` and `0005` to `0018` are served, `0003` is
    /// unavailable, and `0004` serves a snapshot of `0001`.
    fn failing_projection() -> Mismatching {
        let mismatched = ProjectSnapshot {
            shortcode: "0001".to_owned(),
            ..snapshot("0004", "Brass lamp")
        };
        let served = (5..=18).fold(
            FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            |projection, number| projection.with_project(snapshot(&format!("{number:04}"), "Brass lamp")),
        );
        Mismatching(served.with_unavailable("0003"), mismatched)
    }

    /// Each project that does not activate, as `(folder, SHORTCODE)`, the deposited files that
    /// make it fail, and the cause its report names.
    fn failing_projects() -> (Vec<(&'static str, &'static str, &'static str)>, FakeDepositedFiles) {
        let delta = std::fs::read_to_string(fixture().join("delta/config.kdl")).expect("the fixture is readable");
        let reads_curation = things_of("0016").replace(
            "        id from=\"iri\"\n",
            "        id from=\"iri\"\n        curated \"note\" key=\"note\" type=\"text\"\n",
        );
        assert_ne!(reads_curation, things_of("0016"), "the fixture's class has an id node");
        let files = configs(&[
            ("0001", THINGS),
            ("0002", &things_of("0002")),
            ("0003", &things_of("0003")),
            ("0004", &things_of("0004")),
            ("0005", &delta),
            ("0006", "this is { not a project\n"),
            ("0012", &things_with_narrative("0012")),
            ("0014", &things_with_narrative("0014")),
            ("0015", &things_with_narrative("0015")),
            ("0016", &reads_curation),
            ("0017", THINGS),
            ("0018", &things_of("0018")),
        ])
        .with_file("0010", "curation.csv", "iri\n")
        .with_unavailable_file("0012", "narrative/home.en.md")
        .with_unavailable_file("0018", "curation.csv")
        .with_file("0013", "config.kdl", [0xff, 0xfe])
        .with_file("0014", "narrative/home.md", [0xff, 0xfe]);
        let cases = vec![
            ("unknown", "0002", "holds no project 0002"),
            ("outage", "0003", "is unavailable: simulated outage"),
            ("mismatch", "0004", "requested project 0004, but the snapshot is of 0001"),
            ("no-remodel", "0005", "has no 'remodel' block"),
            ("broken", "0006", "failed to parse config.kdl"),
            ("no-config", "0010", "0010/config.kdl: project 0010 deposited no config.kdl"),
            (
                "no-media-project",
                "0011",
                "0011/config.kdl: no deposited files for project \"0011\"",
            ),
            (
                "narrative-outage",
                "0012",
                "0012/narrative/home.en.md: the deposited files are unavailable",
            ),
            ("latin1-config", "0013", "0013/config.kdl is not valid UTF-8"),
            ("latin1-narrative", "0014", "0014/narrative/home.md is not valid UTF-8"),
            (
                "missing-narrative",
                "0015",
                "narrative file 'home.md' not found among the project's narrative files",
            ),
            (
                "no-curation",
                "0016",
                "0016/curation.csv: project 0016 deposited no curation file, and its remodel block reads curated \
                 values",
            ),
            (
                "other-shortcode",
                "0017",
                "0017/config.kdl declares shortcode 0001, but the folder's SHORTCODE is 0017",
            ),
            (
                "curation-outage",
                "0018",
                "0018/curation.csv: the deposited files are unavailable",
            ),
        ];
        (cases, files)
    }

    /// Each cause of a project that does not activate is reported as that cause, so that the
    /// 503s of the test below are not many times one unrelated failure.
    #[test]
    fn a_project_that_cannot_be_loaded_is_reported_with_its_cause() {
        let projection = failing_projection();
        let (cases, files) = failing_projects();
        let (_root, things) = discovered("0001");
        let sources = |discovered: &DiscoveredProject| super::port_sources(&projection, &files, discovered);
        assert!(super::load(&things, &sources).is_ok(), "the fixture loads");

        for (_, shortcode, cause) in cases {
            let (_root, project) = discovered(shortcode);
            let error = super::load(&project, &sources).expect_err("the project does not load");
            assert!(error.contains(cause), "{shortcode}: {cause:?} is not in {error:?}");
        }
    }

    #[tokio::test]
    async fn a_project_that_does_not_activate_fails_alone() {
        let (cases, files) = failing_projects();
        let mut folders = vec![("things", "0001")];
        folders.extend(cases.iter().map(|(id, shortcode, _)| (*id, *shortcode)));
        let root = projects(&folders);
        let cpe = Cpe::new(&config_for(root.path()));

        cpe.activate(&failing_projection(), &files);

        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        for (id, _, _) in &cases {
            for uri in [format!("/{id}"), format!("/{id}/"), format!("/{id}/things/lamp")] {
                let (status, headers, _) = get(&cpe, &uri).await;
                assert_unavailable(status, &headers, &uri);
            }
        }
        for uri in ["/nope", "/nope/things/lamp", "/UNKNOWN/"] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::RETRY_AFTER), "{uri}");
            assert!(body.contains("This address does not name a project."), "{uri}: {body}");
        }
        assert_eq!(cpe.projects().len(), 1);
        let (status, _, overview) = get(&cpe, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            overview.matches("class=\"teaser\"").count(),
            1,
            "the overview lists the one served project: {overview}"
        );
        assert!(overview.contains("href=\"/things/\""), "{overview}");
    }

    #[tokio::test]
    async fn folders_that_share_a_shortcode_both_answer_503_and_nothing_is_fetched() {
        let root = projects(&[("first", "0001"), ("second", "0001")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let files = configs(&[("0001", THINGS)]);

        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &files,
        );

        for uri in ["/first/", "/second/", "/first/things/lamp"] {
            let (status, headers, _) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        assert!(cpe.projects().is_empty());
        assert_eq!(files.requests(), [] as [(String, String); 0]);
    }

    /// The names are the base files the config names and their variants, in the site's
    /// languages (`en`, `de`); a name holding a path separator is never asked for.
    #[test]
    fn activation_asks_the_files_port_for_the_config_the_named_narrative_and_the_curation_only() {
        let config = things_with_home(
            "0001",
            "    narrative file=\"home.md\"\n    narrative file=\"../up.md\"\n    narrative file=\"a\\\\b.md\"",
        )
        .replace(
            "        show-properties \"name\"\n",
            "        show-properties \"name\"\n        narrative-for \"lamp\" file=\"lamp.md\"\n        \
             narrative-for \"x\" file=\"sub/x.md\"\n",
        );
        assert!(config.contains("narrative-for \"lamp\""), "the fixture has a resource-detail");
        let files = configs(&[("0001", &config)]).with_file("0001", "narrative/home.en.md", "# Home\n");
        let (_root, project) = discovered("0001");
        let projection = FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp"));

        let _ = super::port_sources(&projection, &files, &project).expect("the store builds");

        let names: Vec<String> = files.requests().into_iter().map(|(_, name)| name).collect();
        assert_eq!(
            names,
            [
                "config.kdl",
                "narrative/home.md",
                "narrative/home.en.md",
                "narrative/home.de.md",
                "narrative/lamp.md",
                "narrative/lamp.en.md",
                "narrative/lamp.de.md",
                "curation.csv",
            ]
        );
        assert!(files.requests().iter().all(|(shortcode, _)| shortcode == "0001"));
    }

    #[tokio::test]
    async fn a_missing_variant_of_a_narrative_falls_back_to_the_default_language() {
        let root = projects(&[("things", "0001")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let files = configs(&[("0001", &things_with_narrative("0001"))]).with_file(
            "0001",
            "narrative/home.en.md",
            "# Lamps\n\nThe English narrative.\n",
        );

        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &files,
        );

        for uri in ["/things/?lang=en", "/things/?lang=de"] {
            let (status, _, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
            assert!(body.contains("The English narrative."), "{uri}: {body}");
        }
    }

    #[tokio::test]
    async fn a_project_without_a_curation_file_whose_mapping_reads_none_is_served() {
        let root = projects(&[("things", "0001")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let files = configs(&[("0001", THINGS)]);

        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &files,
        );

        assert!(files.requests().contains(&("0001".to_owned(), "curation.csv".to_owned())));
        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn a_panic_while_one_project_activates_stops_neither_activate_nor_the_others() {
        let root = projects(&[("panics", "0009"), ("things", "0001")]);
        let cpe = Cpe::new(&config_for(root.path()));

        cpe.activate(
            &PanickingProjection,
            &configs(&[("0001", THINGS), ("0009", &things_of("0009"))]),
        );

        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, headers, _) = get(&cpe, "/panics/").await;
        assert_unavailable(status, &headers, "/panics/");
    }

    #[tokio::test]
    async fn with_no_projects_folder_the_overview_is_empty_and_every_other_path_is_cpes_not_found_once_activated() {
        let cpe = Cpe::new(&config_for(&fixture().join("no-such-folder")));
        let (status, headers, _) = get(&cpe, "/things/").await;
        assert_unavailable(status, &headers, "before activate");

        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &configs(&[("0001", THINGS)]),
        );

        assert!(cpe.projects().is_empty());
        let (status, headers, body) = get(&cpe, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(header::RETRY_AFTER));
        assert!(body.contains("No projects are being presented yet."), "{body}");
        for uri in ["/things/", "/incunabula/books"] {
            let (status, headers, body) = get(&cpe, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::RETRY_AFTER), "{uri}");
            assert!(body.contains("This address does not name a project."), "{uri}: {body}");
        }
    }

    #[tokio::test]
    async fn a_refused_configuration_is_not_activated() {
        let root = projects(&[("things", "0001")]);
        let config = crate::CpeConfig {
            public_origin: Some("https://repository.dasch.swiss/cpe".to_owned()),
            ..config_for(root.path())
        };
        let cpe = Cpe::new(&config);
        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &configs(&[("0001", THINGS)]),
        );
        for uri in ["/", "/things/", "/_static/cpe/tokens.css"] {
            let (status, headers, _) = get(&cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }
    }

    /// The store is a read model: it is rebuilt from empty from what the ports serve, holds
    /// nothing else, and follows each of them (ADR-0008, ADR-0012).
    #[tokio::test]
    async fn the_store_is_rebuilt_from_the_ports_alone_and_follows_each() {
        let root = projects(&[("things", "0001")]);
        let uris = ["/things/", "/things/things/lamp"];
        let pages_of = |name: &'static str, config: String, narrative: &'static str| {
            let config_for = config_for(root.path());
            async move {
                let cpe = Cpe::new(&config_for);
                let files = configs(&[("0001", &config)]).with_file("0001", "narrative/home.en.md", narrative);
                cpe.activate(&FakeArchiveProjection::default().with_project(snapshot("0001", name)), &files);
                let mut pages = Vec::new();
                for uri in uris {
                    let (status, _, body) = get(&cpe, uri).await;
                    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
                    pages.push(body);
                }
                pages
            }
        };
        let config = || things_with_narrative("0001");

        let first = pages_of("Brass lamp", config(), "# Lamps\n\nOf brass.\n").await;
        let second = pages_of("Brass lamp", config(), "# Lamps\n\nOf brass.\n").await;
        assert_eq!(first, second, "two builds from the same ports serve the same pages");
        assert!(first[1].contains("Brass lamp"), "{}", first[1]);
        assert!(first[0].contains("Of brass."), "{}", first[0]);

        let changed = pages_of("Copper kettle", config(), "# Lamps\n\nOf brass.\n").await;
        assert!(changed[1].contains("Copper kettle"), "{}", changed[1]);
        assert!(!changed[1].contains("Brass lamp"), "{}", changed[1]);

        let retitled = config().replace("title \"Things Fixture\"", "title \"Lamps Fixture\"");
        let changed = pages_of("Brass lamp", retitled, "# Lamps\n\nOf brass.\n").await;
        assert!(
            changed[0].contains("Lamps Fixture") && !changed[0].contains("Things Fixture"),
            "{}",
            changed[0]
        );

        let changed = pages_of("Brass lamp", config(), "# Lamps\n\nOf copper.\n").await;
        assert!(
            changed[0].contains("Of copper.") && !changed[0].contains("Of brass."),
            "{}",
            changed[0]
        );
    }

    #[tokio::test]
    async fn a_second_activation_replaces_what_is_served() {
        let root = projects(&[("things", "0001")]);
        let cpe = Cpe::new(&config_for(root.path()));
        let files = configs(&[("0001", THINGS)]);
        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Brass lamp")),
            &files,
        );
        cpe.activate(
            &FakeArchiveProjection::default().with_project(snapshot("0001", "Copper kettle")),
            &files,
        );
        let (status, _, body) = get(&cpe, "/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Copper kettle") && !body.contains("Brass lamp"), "{body}");
    }

    /// The committed project folders, `sync`'s committed data and `media`'s.
    fn committed_projects() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../projects")
    }

    fn sync_data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sync/data")
    }

    fn media_data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../media/data")
    }

    /// Incunabula as the area binary serves it: its committed folder, its store built from
    /// `sync`'s adapter over the committed 0803 data, and its config, narrative and curation read
    /// through `media`'s adapter over the committed deposited files.
    #[tokio::test]
    async fn incunabula_activates_from_the_committed_data_and_serves_its_pages() {
        let cpe = Cpe::new(&config_for(&committed_projects()));
        cpe.activate(&LiveArchiveProjection::new(sync_data()), &LiveDepositedFiles::new(media_data()));

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

    /// The length and the SHA-256 of Incunabula's whole store text, header included. The header
    /// names the two ports the text came through, so it differs from the prototype's `data.sql`;
    /// the text below it is the prototype's, which the body pin below holds.
    ///
    /// The text follows the committed 0803 data, Incunabula's `remodel` block and the builder.
    /// When one of them changes it on purpose, put the two values the failure prints here.
    const INCUNABULA_STORE_BYTES: usize = 5_631_888;
    const INCUNABULA_STORE_SHA256: &str = "36da32247854b09743fd7177fc91c70be2821dd143efa6bf73f49e9c601d0c86";

    /// Incunabula's store text as the area binary builds it from the committed data.
    fn incunabula_store_text() -> String {
        let discovered = crate::discovery::discover_for_activation(&committed_projects())
            .projects
            .into_iter()
            .find(|project| project.id == "incunabula")
            .expect("Incunabula's folder is committed");
        let projection = LiveArchiveProjection::new(sync_data());
        let files = LiveDepositedFiles::new(media_data());
        super::port_sources(&projection, &files, &discovered)
            .expect("Incunabula's store builds from the committed 0803 data")
            .store_sql
    }

    fn sha256(text: &str) -> String {
        format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(text.as_bytes()))
    }

    #[test]
    fn incunabulas_whole_store_text_is_pinned() {
        let sql = incunabula_store_text();
        assert_eq!(
            (sql.len(), sha256(&sql).as_str()),
            (INCUNABULA_STORE_BYTES, INCUNABULA_STORE_SHA256)
        );
    }

    /// The length and the SHA-256 of Incunabula's store text below its header, which ends at the
    /// first blank line. They are those of `dsp-incubator cpe/projects/incunabula/data.sql` at
    /// `d68f214b` below its header, and they change only when the committed 0803 data,
    /// Incunabula's `remodel` block or the builder's tables do: a change to the header alone
    /// leaves them as they are.
    const INCUNABULA_STORE_BODY_BYTES: usize = 5_631_489;
    const INCUNABULA_STORE_BODY_SHA256: &str = "dfd33273fd48b23b64986a2be880d02bdd9a91c7f65c8b1b8b3ef5e27a0f25af";

    #[test]
    fn incunabulas_store_text_below_its_header_is_the_prototypes() {
        let sql = incunabula_store_text();
        let (header, body) = sql.split_once("\n\n").expect("the header ends at a blank line");
        assert!(
            header.lines().all(|line| line.starts_with("-- ")),
            "every header line is a comment: {header}"
        );
        assert_eq!(
            (body.len(), sha256(body).as_str()),
            (INCUNABULA_STORE_BODY_BYTES, INCUNABULA_STORE_BODY_SHA256)
        );
    }

    #[tokio::test]
    async fn without_the_data_directory_incunabula_answers_503_on_its_prefix() {
        let cpe = Cpe::new(&config_for(&committed_projects()));
        cpe.activate(
            &LiveArchiveProjection::new(sync_data().join("no-such-folder")),
            &LiveDepositedFiles::new(media_data()),
        );
        assert_incunabula_answers_503(&cpe).await;
    }

    #[tokio::test]
    async fn without_the_media_directory_incunabula_answers_503_on_its_prefix() {
        let cpe = Cpe::new(&config_for(&committed_projects()));
        cpe.activate(
            &LiveArchiveProjection::new(sync_data()),
            &LiveDepositedFiles::new(media_data().join("no-such-folder")),
        );
        assert_incunabula_answers_503(&cpe).await;
    }

    /// Incunabula answers 503, and the overview does not list it.
    async fn assert_incunabula_answers_503(cpe: &Cpe) {
        assert!(cpe.projects().is_empty());
        for uri in ["/incunabula/", "/incunabula/books"] {
            let (status, headers, _) = get(cpe, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        let (status, _, overview) = get(cpe, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            !overview.contains("class=\"teaser\"") && !overview.contains("incunabula/"),
            "a project that answers 503 is not linked: {overview}"
        );
        let (status, _, _) = get(cpe, "/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
