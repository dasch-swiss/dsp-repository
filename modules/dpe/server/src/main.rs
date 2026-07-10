use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod config;
#[cfg(feature = "dev")]
mod dev_reload;
mod fragments;
mod telemetry_collector;
mod traceparent;
mod view;

/// Shared state for the page handlers: the (optional) Fathom site id and the
/// resolved stylesheet href (unhashed in dev, content-hashed in release).
#[derive(Clone)]
struct AppState {
    fathom_site_id: Option<String>,
    css_href: String,
}

/// Query params for the project detail page: `?tab=` pre-selects the tab.
#[derive(serde::Deserialize, Default)]
struct TabQuery {
    #[serde(default)]
    tab: Option<String>,
}

async fn projects_page_handler(
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

async fn about_page_handler(
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

async fn project_page_handler(
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
async fn not_found(
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
    use axum::response::Redirect;
    use axum::routing::get;
    use axum::Router;
    use init_tracing_opentelemetry::TracingConfig;
    use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
    use tower_http::services::ServeDir;
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

    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

    // Static assets + 404: serve files from the public dir, falling back to the
    // "Page not found." shell.
    let serve_dir = ServeDir::new(&dpe_config.public_dir).not_found_service(get(not_found).with_state(state.clone()));

    let app = Router::new()
        // --- Traced routes (declared BEFORE .layer()) ---
        // Page routes.
        .route("/", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe/projects", get(projects_page_handler))
        .route("/dpe/about", get(about_page_handler))
        .route("/dpe/projects/{id}", get(project_page_handler))
        // OAI-PMH (note: /dpe/oai, not /oai) — XML, must stay unbroken.
        .route("/dpe/oai", get(dpe_api_oai::oai_handler))
        // Datastar SSE + JSON endpoints.
        .route("/dpe/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
        .route("/dpe/projects/search", get(fragments::search_fragment_handler))
        .route("/dpe/api/v2/projects", get(fragments::projects_json_handler))
        .route("/dpe/api/v2/projects/{id}", get(fragments::project_json_handler))
        // Static assets + 404 fallback.
        .fallback_service(serve_dir)
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default());

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
            axum::routing::post(telemetry_collector::collect_handler).layer({
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::key_extractor::SmartIpKeyExtractor;
                use tower_governor::GovernorLayer;

                let governor_conf = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(SmartIpKeyExtractor)
                    .finish()
                    .expect("GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(governor_conf) }
            }),
        )
        .with_state(state);

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
    use std::fs;

    let mut errors: Vec<String> = Vec::new();
    let mut project_count = 0;
    let mut record_count = 0;
    let mut person_count = 0;
    let mut org_count = 0;

    // Validate projects
    let projects_dir = data_dir.join("projects");
    let mut contributor_ids: Vec<String> = Vec::new();
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
                        match serde_json::from_str::<dpe_core::ProjectRaw>(&json) {
                            Ok(raw) => {
                                // Collect contributor IDs for cross-reference checks
                                for attr in &raw.attributions {
                                    contributor_ids.push(attr.contributor.clone());
                                }
                                if let Some(contacts) = &raw.contact_point {
                                    for c in contacts {
                                        contributor_ids.push(c.clone());
                                    }
                                }
                                project_count += 1;
                                // Validate conversion from raw to domain
                                let _project: dpe_core::Project = raw.into();
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
                    Ok(json) => match serde_json::from_str::<Vec<dpe_core::Record>>(&json) {
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
                    Ok(json) => match serde_json::from_str::<dpe_core::Person>(&json) {
                        Ok(p) => {
                            // Guard against project roles drifting into jobTitles.
                            // A role belongs in a project's attributions
                            // (contributorType), not in a person's jobTitles, or
                            // it becomes invisible to the OAI-PMH creator logic.
                            for title in &p.job_titles {
                                if dpe_core::is_role_job_title(title) {
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
                    Ok(json) => match serde_json::from_str::<dpe_core::Organization>(&json) {
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

    // Cross-reference checks: verify contributor IDs resolve to known persons or organizations
    for id in &contributor_ids {
        if !known_person_ids.contains(id) && !known_org_ids.contains(id) {
            errors.push(format!(
                "broken reference: contributor '{id}' not found in persons/ or organizations/"
            ));
        }
    }

    // Report results
    println!(
        "Validated: {} projects, {} records, {} persons, {} organizations",
        project_count, record_count, person_count, org_count
    );

    if errors.is_empty() {
        println!("All data files are valid.");
        ExitCode::SUCCESS
    } else {
        println!("\n{} error(s) found:", errors.len());
        for err in &errors {
            println!("  - {err}");
        }
        ExitCode::FAILURE
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

fn healthcheck(url: &str) -> ExitCode {
    // Only allow localhost URLs to prevent SSRF.
    let allowed_prefixes = ["http://localhost", "http://127.0.0.1", "http://[::1]"];
    if !allowed_prefixes.iter().any(|prefix| url.starts_with(prefix)) {
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
