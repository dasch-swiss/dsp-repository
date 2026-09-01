use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod config;
#[cfg(feature = "dev")]
mod dev_reload;
pub(crate) mod downloads;
pub(crate) mod fragments;
mod page_url;
mod router;
mod traceparent;
mod view;

/// Shared state for the page handlers: the (optional) Fathom site id and the
/// resolved stylesheet href (unhashed in dev, content-hashed in release).
#[derive(Clone)]
pub(crate) struct AppState {
    fathom_site_id: Option<String>,
    css_href: String,
}

/// Query params for the project detail page: `?tab=` pre-selects the tab.
#[derive(serde::Deserialize, Default)]
struct TabQuery {
    #[serde(default)]
    tab: Option<String>,
}

pub(crate) async fn projects_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(query): axum::extract::Query<dpe_web::domain::ProjectQuery>,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    let content = dpe_web::pages::projects_page(&query);
    axum::response::Html(
        view::page(
            "DaSCH Metadata Browser Projects Overview",
            tp.as_deref(),
            &state.css_href,
            state.fathom_site_id.as_deref(),
            content,
        )
        .into_string(),
    )
}

pub(crate) async fn about_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    let content = dpe_web::pages::about_page();
    axum::response::Html(
        view::page(
            "DaSCH Metadata Browser — About",
            tp.as_deref(),
            &state.css_href,
            state.fathom_site_id.as_deref(),
            content,
        )
        .into_string(),
    )
}

pub(crate) async fn project_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(tab): axum::extract::Query<TabQuery>,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    // Fall back to "overview" for a missing or unrecognized tab, mirroring the
    // validation the SSE fragment handler applies against VALID_TABS.
    let active_tab = tab
        .tab
        .as_deref()
        .filter(|t| dpe_core::project::VALID_TABS.contains(t))
        .unwrap_or("overview");
    let content = dpe_web::pages::project_page(&id, active_tab);
    // Prefer the project's display name for the document title (falls back to the
    // shortcode when the project can't be resolved). `project_by_shortcode` reads
    // from the in-memory project cache, so this is not an extra load.
    let title = dpe_core::project_cache::project_by_shortcode(&id)
        .map(|p| format!("{} — DaSCH Metadata Browser", p.name))
        .unwrap_or_else(|| format!("Project {id} — DaSCH Metadata Browser"));
    axum::response::Html(
        view::page(&title, tp.as_deref(), &state.css_href, state.fathom_site_id.as_deref(), content).into_string(),
    )
}

/// 404 fallback (after `ServeDir` finds no matching static file): the app shell
/// with a "Page not found." body.
pub(crate) async fn not_found(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let tp = traceparent::extract_traceparent();
    let content = maud::html! {
        "Page not found."
    };
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::response::Html(
            view::page(
                "DaSCH Metadata Browser — Page Not Found",
                tp.as_deref(),
                &state.css_href,
                state.fathom_site_id.as_deref(),
                content,
            )
            .into_string(),
        ),
    )
}

/// Resolve the stylesheet href. Dev serves the unhashed `app.css` at a fixed
/// path; release builds emit a content-hashed `app.<hash>.css`, whose name is
/// discovered from the assets directory at startup.
fn resolve_css_href(public_dir: &std::path::Path) -> String {
    if cfg!(debug_assertions) {
        return "/assets/app.css".to_string();
    }
    let assets = public_dir.join("assets");
    discover_hashed_css(&assets).unwrap_or_else(|| "/assets/app.css".to_string())
}

/// Scan `assets_dir` for a content-hashed `app.<hash>.css` and return its
/// `/assets/…` href if one is present. Kept separate from [`resolve_css_href`]
/// (which gates on `debug_assertions`) so the discovery logic is unit-testable
/// under `cargo test`.
fn discover_hashed_css(assets_dir: &std::path::Path) -> Option<String> {
    let entries = std::fs::read_dir(assets_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("app.") && name.ends_with(".css") && name != "app.css" {
            return Some(format!("/assets/{name}"));
        }
    }
    None
}

#[cfg(test)]
mod css_href_tests {
    use super::discover_hashed_css;

    #[test]
    fn discovers_content_hashed_stylesheet() {
        let dir = std::env::temp_dir().join(format!("dpe_css_discover_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.abc123.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir).as_deref(), Some("/assets/app.abc123.css"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_unhashed_stylesheet_and_returns_none() {
        let dir = std::env::temp_dir().join(format!("dpe_css_none_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_dir_returns_none() {
        let dir = std::env::temp_dir().join(format!("dpe_css_missing_{}", std::process::id()));
        assert_eq!(discover_hashed_css(&dir), None);
    }
}

#[derive(Parser)]
#[command(name = "dpe-server", about = "DaSCH Discovery and Presentation Environment")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve,
    /// Validate all data files under the given data directory
    Validate {
        /// Path to the data directory containing projects/, persons/, organizations/, records/
        data_dir: PathBuf,
    },
    /// Check if the server is healthy (for Docker HEALTHCHECK)
    Healthcheck {
        #[arg(long, default_value = "http://localhost:8080/healthz")]
        url: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No subcommand: print help and exit
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
        Some(Commands::Serve) => serve(),
        Some(Commands::Validate { data_dir }) => validate(data_dir),
        Some(Commands::Healthcheck { url }) => healthcheck(&url),
    }
}

#[tokio::main]
async fn serve() -> ExitCode {
    use axum::http::StatusCode;
    use axum::routing::get;
    use init_tracing_opentelemetry::TracingConfig;
    use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
    use tracing_subscriber::layer::SubscriberExt;

    // Route panics through tracing so they appear as structured Grafana logs
    // alongside normal traces (with location, thread, backtrace). Installed
    // before OTel init so a panic during init is also captured (it falls
    // back to the default stderr hook until a subscriber is registered).
    install_tracing_panic_hook();

    // Export logs via OTLP only when DPE_ENV=DEV (local dev) and an OTLP
    // endpoint is configured. Production (DPE_ENV=PROD) logs to stdout only.
    // Default to "DEV" when unset.
    let logger_provider: Option<SdkLoggerProvider> = if std::env::var("DPE_ENV").as_deref().unwrap_or("DEV") == "DEV"
        && std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
    {
        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .build()
            .expect("failed to build OTLP log exporter");
        Some(SdkLoggerProvider::builder().with_batch_exporter(exporter).build())
    } else {
        None
    };

    // Initialize OpenTelemetry tracing subscriber.
    // Reads OTEL_* env vars automatically. Falls back to no-op export when
    // OTEL_EXPORTER_OTLP_ENDPOINT is not set (safe for local development).
    // Log level is controlled via RUST_LOG.
    let _otel_guard = TracingConfig::production()
        .with_otel_tracer_name(env!("CARGO_PKG_NAME"))
        .init_subscriber_ext(|registry| {
            use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
            let otel_logs_layer: Option<OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>> =
                logger_provider.as_ref().map(OpenTelemetryTracingBridge::new);
            registry.with(otel_logs_layer)
        })
        .expect("failed to initialize OpenTelemetry tracing");

    // Start Pyroscope continuous profiling agent (CPU sampling).
    // Only active when PYROSCOPE_ENDPOINT is set.
    const PROFILING_SAMPLE_RATE: u32 = 100;

    let _pyroscope_agent = if let Ok(endpoint) = std::env::var("PYROSCOPE_ENDPOINT") {
        let backend = pyroscope::backend::pprof_backend(
            pyroscope::backend::PprofConfig { sample_rate: PROFILING_SAMPLE_RATE },
            pyroscope::backend::BackendConfig::default(),
        );

        let agent = pyroscope::pyroscope::PyroscopeAgentBuilder::new(
            &endpoint,
            env!("CARGO_PKG_NAME"),
            PROFILING_SAMPLE_RATE,
            "pyroscope-rs", // matches pyroscope crate's PPROFRS_SPY_NAME
            "2.0.0",        // pyroscope crate version (PPROFRS_SPY_VERSION is private)
            backend,
        )
        .tags(vec![("service.namespace", "dpe")])
        .build()
        .expect("failed to build Pyroscope agent");

        tracing::info!(endpoint = %endpoint, "Pyroscope profiling enabled");
        Some(agent.start().expect("failed to start Pyroscope agent"))
    } else {
        None
    };

    // Load DPE-specific configuration (defaults → dpe.toml → DPE_* env vars)
    let dpe_config = config::DpeConfig::load().expect("failed to load DPE configuration");
    tracing::info!(data_dir = %dpe_config.data_dir.display(), "DPE configuration loaded");

    if let Some(ref site_id) = dpe_config.fathom_site_id {
        tracing::info!(fathom_site_id = %site_id, "Fathom Analytics enabled");
    }

    // Set data directory for dpe-core (thread-safe OnceLock, no env mutation)
    dpe_core::set_data_dir(dpe_config.data_dir.to_str().expect("data_dir path must be valid UTF-8"));

    // Set the public OAI-PMH base URL (thread-safe OnceLock), emitted as baseURL / <request>.
    dpe_api_oai::set_base_url(&dpe_config.oai_base_url);
    tracing::info!(oai_base_url = %dpe_config.oai_base_url, "OAI-PMH base URL set");

    // Set placeholder visibility flag for dpe-core
    dpe_core::set_show_placeholder_values(dpe_config.show_placeholder_values);
    if dpe_config.show_placeholder_values {
        tracing::info!("Placeholder values (MISSING/CALCULATED) will be shown in the UI");
    }

    tokio::task::spawn_blocking(dpe_core::record_cache::all_records);

    // Listen address: DPE_SITE_ADDR → default 127.0.0.1:4000.
    let addr: std::net::SocketAddr = std::env::var("DPE_SITE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid site address (DPE_SITE_ADDR)");

    let state = AppState {
        fathom_site_id: dpe_config.fathom_site_id.clone(),
        css_href: resolve_css_href(&dpe_config.public_dir),
    };

    // Traced routes, incl. the rate-limited /dpe/oai (limiter scoped to that route).
    let app = router::build_router(state, &dpe_config.public_dir, router::oai_router(&dpe_config));

    // Dev-only browser live-reload (`dev` feature): wraps the page/static
    // routes declared above; the untraced routes below stay outside it.
    #[cfg(feature = "dev")]
    let app = dev_reload::apply(app, &dpe_config.public_dir);

    let app = app
        // --- Untraced routes ---
        // Routes declared AFTER .layer() calls are NOT wrapped by those layers.
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route(
            "/telemetry/collect",
            // "dpe" names the OTel instrumentation scope (`dpe.browser`), which
            // the dashboards filter on — do not change it.
            platform_telemetry::collector::collect_route("dpe", page_url::normalize_page_url).layer({
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::GovernorLayer;

                use crate::router::RightmostXffKeyExtractor;

                let governor_conf = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(RightmostXffKeyExtractor)
                    .finish()
                    .expect("GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(governor_conf) }
            }),
        );

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server exited with error");

    // Stop Pyroscope agent and flush OTel data. Both perform blocking I/O
    // (condvar waits, thread joins, HTTP uploads, OTLP flush). Run via
    // spawn_blocking to avoid deadlocking the Tokio runtime.
    tokio::task::spawn_blocking(move || {
        if let Some(agent) = _pyroscope_agent {
            if let Ok(ready) = agent.stop() {
                ready.shutdown();
            }
        }
        // Flush OTel logs before dropping the trace/metrics guard — log records
        // may reference trace context that becomes invalid after guard drop.
        if let Some(provider) = logger_provider {
            let _ = provider.force_flush();
            let _ = provider.shutdown();
        }
        drop(_otel_guard);
    })
    .await
    .ok();

    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, flushing telemetry");
}

fn validate(data_dir: PathBuf) -> ExitCode {
    let report = collect_validation_errors(&data_dir);

    println!(
        "Validated: {} projects, {} records, {} persons, {} organizations",
        report.project_count, report.record_count, report.person_count, report.org_count
    );

    if report.errors.is_empty() {
        println!("All data files are valid.");
        ExitCode::SUCCESS
    } else {
        println!("\n{} error(s) found:", report.errors.len());
        for err in &report.errors {
            println!("  - {err}");
        }
        ExitCode::FAILURE
    }
}

/// Counts and errors gathered by [`collect_validation_errors`]. Factored out of
/// `validate` so the validation logic is testable without capturing a process
/// exit code.
struct ValidationReport {
    project_count: usize,
    record_count: usize,
    person_count: usize,
    org_count: usize,
    errors: Vec<String>,
}

fn collect_validation_errors(data_dir: &std::path::Path) -> ValidationReport {
    use std::fs;

    let mut errors: Vec<String> = Vec::new();
    let mut project_count = 0;
    let mut record_count = 0;
    let mut person_count = 0;
    let mut org_count = 0;

    // Validate projects
    let projects_dir = data_dir.join("projects");
    // Every contributor id every project references, cross-referenced against
    // `persons/` and `organizations/` once the whole corpus has been read.
    let mut contributor_refs: Vec<platform_metadata::ContributorRef> = Vec::new();
    // Temporal-coverage resolution: the same ChronOntology period cache and
    // offline enrichment table the OAI-PMH `every_committed_temporal_coverage_resolves`
    // test loads, so the two can never disagree about what counts as resolved.
    let temporal_periods = platform_metadata::chronontology::load_from(data_dir);
    let temporal_enrichment = platform_metadata::temporal_enrichment::load_from(data_dir);
    // Each distinct offending value is reported once for the whole corpus: an
    // unenriched period name shared by twenty projects is one thing to fix, not
    // twenty. Keyed on the value rather than the project, so the first file to
    // carry it is the one named — and on the member too, so a value that is
    // wrong in two different members is still reported for each. A finding
    // carrying no value is per project and is never folded away.
    let mut reported_values: std::collections::HashSet<(&str, String)> = std::collections::HashSet::new();
    if projects_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => {
                        // Parsing is the one project rule the shared checker cannot
                        // hold: it takes a `&ProjectRaw`, so it is downstream of this.
                        match serde_json::from_str::<platform_metadata::ProjectRaw>(&json) {
                            Ok(raw) => {
                                project_count += 1;
                                contributor_refs.extend(platform_metadata::contributor_refs(&raw));

                                for finding in
                                    platform_metadata::check_project(&raw, &temporal_periods, &temporal_enrichment)
                                {
                                    if let Some(value) = &finding.value {
                                        if !reported_values.insert((finding.field, value.clone())) {
                                            continue;
                                        }
                                    }
                                    // The checker's message carries no file prefix, so
                                    // that a per-field consumer is not handed a path it
                                    // has no use for. This is the consumer that wants
                                    // one.
                                    errors.push(format!("{filename}: {}", finding.message));
                                }
                            }
                            Err(e) => errors.push(format!("{filename}: {e}")),
                        }
                    }
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    } else {
        errors.push(format!("projects directory not found: {}", projects_dir.display()));
    }

    // Validate records
    let records_dir = data_dir.join("records");
    if records_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&records_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<Vec<platform_metadata::Record>>(&json) {
                        Ok(recs) => record_count += recs.len(),
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Validate persons
    let persons_dir = data_dir.join("persons");
    let mut known_person_ids = std::collections::HashSet::new();
    if persons_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&persons_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<platform_metadata::Person>(&json) {
                        Ok(p) => {
                            // Guard against project roles drifting into jobTitles.
                            // A role belongs in a project's attributions
                            // (contributorType), not in a person's jobTitles, or
                            // it becomes invisible to the OAI-PMH creator logic.
                            for title in &p.job_titles {
                                if platform_metadata::is_role_job_title(title) {
                                    errors.push(format!(
                                        "{filename}: jobTitle '{title}' on {} is a project role; \
                                         move it to the project's attributions (contributorType)",
                                        p.id
                                    ));
                                }
                            }
                            known_person_ids.insert(p.id.clone());
                            person_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Validate organizations
    let orgs_dir = data_dir.join("organizations");
    let mut known_org_ids = std::collections::HashSet::new();
    if orgs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&orgs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<platform_metadata::Organization>(&json) {
                        Ok(o) => {
                            known_org_ids.insert(o.id.clone());
                            org_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Cross-reference checks: verify contributor IDs resolve to known persons or
    // organizations. Which ids a project references is the shared checker's
    // question; what counts as known is this corpus's, and the editor answers it
    // differently — published entities plus its own pending proposals.
    for reference in &contributor_refs {
        let id = &reference.id;
        if !known_person_ids.contains(id) && !known_org_ids.contains(id) {
            errors.push(format!(
                "broken reference: contributor '{id}' not found in persons/ or organizations/"
            ));
        }
    }

    ValidationReport { project_count, record_count, person_count, org_count, errors }
}

#[cfg(test)]
mod validate_tests {
    use super::collect_validation_errors;

    /// A minimal `ProjectRaw`, valid except for its `temporalCoverage` and
    /// `attributions`, which the caller supplies as raw JSON array literals.
    fn project_json(temporal_coverage: &str, attributions: &str) -> String {
        format!(
            r#"{{
                "id": "0000", "pid": "MISSING", "name": "Test Project", "shortcode": "0000",
                "officialName": "Test Project", "status": "Finished", "shortDescription": "test",
                "description": {{}}, "startDate": "MISSING", "endDate": "MISSING",
                "howToCite": "test", "accessRights": {{ "accessRights": "Full Open Access" }},
                "legalInfo": [], "keywords": [], "disciplines": [],
                "temporalCoverage": {temporal_coverage}, "spatialCoverage": [], "attributions": {attributions},
                "funding": "No funding"
            }}"#
        )
    }

    /// Writes `project_json(temporal_coverage)` as the sole project file under a
    /// fresh temp data dir (with an empty `projects/`), plus an optional
    /// `temporal-coverage-enrichment.json` at the data dir root, and returns the
    /// collected validation errors.
    fn validate_with(temporal_coverage: &str, enrichment_json: Option<&str>) -> Vec<String> {
        // An atomic counter (not e.g. the JSON literal's length) guarantees a
        // distinct dir per call even if two callers happen to pass same-length
        // literals — `cargo test` runs test fns concurrently, so a collision
        // would let one test's cleanup race another's still-in-progress write.
        static CALL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let call_id = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("dpe_validate_temporal_{}_{call_id}", std::process::id()));
        let projects_dir = dir.join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();
        std::fs::write(projects_dir.join("0000_test.json"), project_json(temporal_coverage, "[]")).unwrap();
        if let Some(enrichment) = enrichment_json {
            std::fs::write(dir.join("temporal-coverage-enrichment.json"), enrichment).unwrap();
        }

        let report = collect_validation_errors(&dir);
        std::fs::remove_dir_all(&dir).ok();
        report.errors
    }

    #[test]
    fn flags_temporal_coverage_with_no_resolved_date() {
        let errors = validate_with(r#"[{"en": "Mysterious Era"}]"#, None);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Mysterious Era") && e.contains("no resolved date")),
            "expected an unresolved temporalCoverage error, got: {errors:?}"
        );
    }

    #[test]
    fn accepts_temporal_coverage_resolved_via_enrichment() {
        let errors = validate_with(
            r#"[{"en": "Early Christianity"}]"#,
            Some(
                r#"{"Early Christianity": {"date": "0030/0451", "original_name": "Early Christianity", "source": "llm"}}"#,
            ),
        );
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn accepts_temporal_coverage_explicitly_marked_unresolved() {
        let errors = validate_with(
            r#"[{"en": "Swiss"}]"#,
            Some(r#"{"Swiss": {"date": null, "original_name": "Swiss", "source": "unresolved"}}"#),
        );
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    /// A data directory assembled for one test.
    ///
    /// [`validate_with`] covers the temporal rule over a single project file;
    /// the tests below pin the exact *wording* of every error
    /// [`collect_validation_errors`] emits, so each needs a different corner of
    /// the corpus populated — a person, an organization, a records file, or no
    /// `projects/` at all.
    ///
    /// The wording is a contract in two directions: `just validate-data` is read
    /// by a human deciding what to fix, and `modules/dpe/CLAUDE.md` quotes the
    /// temporal-coverage message as the instruction for adding an enrichment
    /// row. Snapshotting the recipe's output does not protect it — the committed
    /// corpus is valid, so that output is two lines and exercises no error
    /// branch at all.
    struct Fixture {
        dir: std::path::PathBuf,
    }

    impl Fixture {
        /// A data dir holding an empty `projects/`. Every rule except
        /// [`reports_a_missing_projects_directory`] wants it: absent, its own
        /// error joins the list and the assertion under test is no longer about
        /// one rule.
        fn new() -> Self {
            let fixture = Self::bare();
            std::fs::create_dir_all(fixture.dir.join("projects")).unwrap();
            fixture
        }

        /// A data dir with nothing in it.
        fn bare() -> Self {
            // Same reasoning as `validate_with`: an atomic counter rather than
            // anything derived from the contents, so two same-shaped fixtures
            // cannot collide and let one test's cleanup race another's write.
            static CALL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let call_id = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("dpe_validate_wording_{}_{call_id}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        /// Writes `contents` at `relative`, creating parent directories.
        fn with(self, relative: &str, contents: &str) -> Self {
            let path = self.dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
            self
        }

        /// The path `collect_validation_errors` names in an error. Errors are
        /// keyed by the full path, not the bare filename, so a test cannot
        /// assemble the expected message without it.
        fn path_of(&self, relative: &str) -> String {
            self.dir.join(relative).display().to_string()
        }

        fn report(&self) -> super::ValidationReport {
            collect_validation_errors(&self.dir)
        }

        fn errors(&self) -> Vec<String> {
            self.report().errors
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).ok();
        }
    }

    /// A minimal `Record`. `pid` deserializes from the ARK URL string, not an
    /// object, so it cannot be assembled field by field.
    fn record_json(record_id: &str) -> String {
        format!(
            r#"{{
                "id": "http://rdfh.ch/0000/{record_id}",
                "pid": "https://ark.dasch.swiss/ark:/72163/1/0000/{record_id}",
                "label": {{ "en": "Record {record_id}" }},
                "accessRights": "Full Open Access",
                "legalInfo": {{
                    "license": {{ "licenseIdentifier": "public domain", "licenseDate": "2023-01-01",
                                  "licenseURI": "https://creativecommons.org/publicdomain/zero/1.0/" }},
                    "copyrightHolder": "DaSCH",
                    "authorship": ["DaSCH"]
                }}
            }}"#
        )
    }

    /// A person, valid except for the `jobTitles` the caller supplies.
    fn person_json(id: &str, job_titles: &str) -> String {
        format!(
            r#"{{
                "id": "{id}", "givenNames": ["Ada"], "familyNames": ["Lovelace"],
                "jobTitles": {job_titles}
            }}"#
        )
    }

    #[test]
    fn unresolved_temporal_coverage_message_is_unchanged() {
        let fixture =
            Fixture::new().with("projects/0000_test.json", &project_json(r#"[{"en": "Mysterious Era"}]"#, "[]"));
        assert_eq!(
            fixture.errors(),
            vec![format!(
                "{}: temporalCoverage 'Mysterious Era' has no resolved date \
                 (add a W3CDTF range to temporal-coverage-enrichment.json, \
                 or mark source=\"unresolved\" if not a time period)",
                fixture.path_of("projects/0000_test.json")
            )]
        );
    }

    #[test]
    fn role_job_title_message_is_unchanged() {
        let fixture = Fixture::new().with("persons/ada.json", &person_json("ada", r#"["Project Leader"]"#));
        assert_eq!(
            fixture.errors(),
            vec![format!(
                "{}: jobTitle 'Project Leader' on ada is a project role; \
                 move it to the project's attributions (contributorType)",
                fixture.path_of("persons/ada.json")
            )]
        );
    }

    #[test]
    fn broken_contributor_reference_message_is_unchanged() {
        let fixture = Fixture::new().with(
            "projects/0000_test.json",
            &project_json("[]", r#"[{"contributor": "ghost", "contributorType": ["Project Leader"]}]"#),
        );
        assert_eq!(
            fixture.errors(),
            vec!["broken reference: contributor 'ghost' not found in persons/ or organizations/".to_string()]
        );
    }

    #[test]
    fn a_contributor_resolves_against_organizations_as_well_as_persons() {
        let fixture = Fixture::new()
            .with(
                "projects/0000_test.json",
                &project_json("[]", r#"[{"contributor": "unibas", "contributorType": ["Project Leader"]}]"#),
            )
            .with(
                "organizations/unibas.json",
                r#"{"id": "unibas", "name": "University of Basel", "url": "https://unibas.ch"}"#,
            );
        assert!(fixture.errors().is_empty(), "expected no errors, got: {:?}", fixture.errors());
    }

    #[test]
    fn reports_a_missing_projects_directory() {
        let fixture = Fixture::bare();
        assert_eq!(
            fixture.errors(),
            vec![format!("projects directory not found: {}", fixture.path_of("projects"))]
        );
    }

    #[test]
    fn a_malformed_file_is_reported_against_its_own_path() {
        let fixture = Fixture::new()
            .with("projects/0000_test.json", "{ not json")
            .with("records/0000.json", "{ not json")
            .with("persons/ada.json", "{ not json")
            .with("organizations/unibas.json", "{ not json");
        let errors = fixture.errors();
        assert_eq!(errors.len(), 4, "expected one error per malformed file, got: {errors:?}");
        // Each error is `"{path}: {serde message}"`. The prefix is ours and is
        // asserted; serde's own wording is not, so a serde bump cannot fail this
        // test for a reason that has nothing to do with the rules.
        for relative in [
            "projects/0000_test.json",
            "records/0000.json",
            "persons/ada.json",
            "organizations/unibas.json",
        ] {
            let prefix = format!("{}: ", fixture.path_of(relative));
            assert!(
                errors.iter().any(|e| e.starts_with(&prefix)),
                "expected an error prefixed {prefix:?}, got: {errors:?}"
            );
        }
    }

    #[test]
    fn a_name_resolved_in_one_project_no_longer_masks_a_gap_in_another() {
        // Two projects share the coverage name "Trajanic": one carries it as a
        // ChronOntology reference that resolves, the other as unenriched free
        // text, which is a genuine gap.
        //
        // The corpus-wide de-duplication used to be keyed on every *named* entry
        // rather than every *reported* one, so reading the resolving project
        // first marked the name checked and the real gap went unreported —
        // `validate` printed "All data files are valid." and exited 0. Reported
        // values are now what de-duplicates, so the gap is reported once no
        // matter which file is read first.
        let fixture = Fixture::new()
            .with(
                "projects/0000_resolves.json",
                &project_json(
                    r#"[{"type": "Chronontology",
                         "url": "https://chronontology.dainst.org/period/0vGXxVln724L",
                         "text": "Trajanic"}]"#,
                    "[]",
                ),
            )
            .with("projects/0001_gap.json", &project_json(r#"[{"en": "Trajanic"}]"#, "[]"))
            .with(
                "chronontology-periods.json",
                r#"{"0vGXxVln724L": {"hasTimespan": [{"begin": {"at": "98"}, "end": {"at": "117"}}]}}"#,
            );
        let errors = fixture.errors();
        assert_eq!(errors.len(), 1, "expected the gap reported exactly once, got: {errors:?}");
        assert_eq!(
            errors[0],
            format!(
                "{}: temporalCoverage 'Trajanic' has no resolved date \
                 (add a W3CDTF range to temporal-coverage-enrichment.json, \
                 or mark source=\"unresolved\" if not a time period)",
                fixture.path_of("projects/0001_gap.json")
            )
        );
    }

    #[test]
    fn counts_feed_the_validated_summary_line() {
        let fixture = Fixture::new()
            .with(
                "projects/0000_test.json",
                &project_json("[]", r#"[{"contributor": "ada", "contributorType": ["Project Leader"]}]"#),
            )
            .with("persons/ada.json", &person_json("ada", "[]"))
            .with(
                "organizations/unibas.json",
                r#"{"id": "unibas", "name": "University of Basel", "url": "https://unibas.ch"}"#,
            )
            // Records are counted per entry, not per file: the summary line says
            // "50994 records" over 85 files.
            .with("records/0000.json", "[]")
            .with(
                "records/0001.json",
                &format!("[{}, {}]", record_json("one"), record_json("two")),
            );
        let report = fixture.report();
        assert!(report.errors.is_empty(), "expected no errors, got: {:?}", report.errors);
        assert_eq!(report.project_count, 1);
        assert_eq!(report.record_count, 2);
        assert_eq!(report.person_count, 1);
        assert_eq!(report.org_count, 1);
    }
}

/// Install a panic hook that emits panics as structured `tracing::error!`
/// events, so production panics are captured by the same OTel pipeline as
/// the rest of the logs. Field names follow the OTel semconv for exceptions
/// (`exception.message`, `exception.stacktrace`, `exception.r#type`,
/// `thread.name`) so panic events share Grafana Sift / Loki query surface
/// with `RecordException` events emitted by instrumented spans.
///
/// The default stderr hook is only invoked as a fallback when the structured
/// emission itself panics (e.g. OTel exporter in a degraded state). Under
/// normal operation each panic produces exactly one log line — no duplicate
/// stderr backtrace.
fn install_tracing_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort structured emission.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let message = info.payload_as_str().unwrap_or("<non-string panic payload>");
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            // `Backtrace::capture` respects `RUST_BACKTRACE` / `RUST_LIB_BACKTRACE`.
            // Operators can opt in with `RUST_BACKTRACE=1` at incident time.
            let backtrace = std::backtrace::Backtrace::capture().to_string();

            if let Some(loc) = info.location() {
                tracing::error!(
                    exception.r#type = "panic",
                    exception.message = %message,
                    exception.stacktrace = %backtrace,
                    thread.name = %thread_name,
                    code.filepath = %loc.file(),
                    code.lineno = loc.line(),
                    code.column = loc.column(),
                    "thread panicked"
                );
            } else {
                tracing::error!(
                    exception.r#type = "panic",
                    exception.message = %message,
                    exception.stacktrace = %backtrace,
                    thread.name = %thread_name,
                    "thread panicked"
                );
            }
        }));

        // Fall back to the default stderr hook only if the structured emission
        // itself panicked, so the panic is never silently swallowed.
        if result.is_err() {
            default_hook(info);
        }
    }));
}

/// Whether `url` is one the healthcheck may call. The URL comes from a CLI
/// flag the Docker `HEALTHCHECK` supplies, so restricting it to loopback stops
/// a mistyped or tampered value turning the container into an SSRF probe.
///
/// The host must be followed by a port, a path, or end of string. A bare
/// `starts_with` is not enough: `http://localhost.evil.com/healthz` and
/// `http://localhost@evil.com/healthz` both begin with `http://localhost` while
/// addressing `evil.com` — in the second case `localhost` is userinfo, not the
/// host. Either would send the probe off-box and, on a 200, report the container
/// healthy while the real server is down.
///
/// Separate from [`healthcheck`] so the rule is testable without issuing a
/// request.
fn is_allowed_healthcheck_url(url: &str) -> bool {
    ["http://localhost", "http://127.0.0.1", "http://[::1]"].iter().any(|prefix| {
        url.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('/'))
    })
}

fn healthcheck(url: &str) -> ExitCode {
    if !is_allowed_healthcheck_url(url) {
        eprintln!("healthcheck: only localhost URLs are allowed, got: {url}");
        return ExitCode::FAILURE;
    }

    let agent: ureq::Agent = ureq::config::Config::builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    match agent.get(url).call() {
        Ok(response) => {
            if response.status() == 200 {
                ExitCode::SUCCESS
            } else {
                eprintln!("healthcheck: unexpected status {}", response.status());
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("healthcheck: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod healthcheck_tests {
    use super::*;

    #[test]
    fn healthcheck_allows_only_loopback_urls() {
        assert!(is_allowed_healthcheck_url("http://localhost:8080/healthz"));
        assert!(is_allowed_healthcheck_url("http://127.0.0.1:8080/healthz"));
        assert!(is_allowed_healthcheck_url("http://[::1]:8080/healthz"));
        // Port and path are both optional.
        assert!(is_allowed_healthcheck_url("http://localhost/healthz"));
        assert!(is_allowed_healthcheck_url("http://localhost"));

        assert!(!is_allowed_healthcheck_url("http://example.com/healthz"));
        // Lookalike hosts: each begins with an allowed prefix but addresses
        // somewhere else, so the host must end at a `:`, a `/`, or end of string.
        assert!(!is_allowed_healthcheck_url("http://localhost.evil.com/healthz"));
        assert!(!is_allowed_healthcheck_url("http://127.0.0.1.evil.com/healthz"));
        // `localhost` here is userinfo — the actual host is evil.com.
        assert!(!is_allowed_healthcheck_url("http://localhost@evil.com/healthz"));
        assert!(!is_allowed_healthcheck_url("http://evil.com/?x=http://localhost"));
        // Scheme is part of the prefix, so https is rejected too: the probe
        // talks to the process inside its own container, never over TLS.
        assert!(!is_allowed_healthcheck_url("https://localhost/healthz"));
    }
}
