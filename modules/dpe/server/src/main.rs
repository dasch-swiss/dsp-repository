use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod config;
mod fragments;
mod telemetry_collector;
mod traceparent;

#[derive(Parser)]
#[command(name = "dpe", about = "DaSCH Discovery and Presentation Environment")]
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
    use axum::Router;
    use dpe_web::*;
    use init_tracing_opentelemetry::TracingConfig;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
    use tracing_subscriber::layer::SubscriberExt;

    // Export logs via OTLP only when LEPTOS_ENV=DEV (local dev) and an OTLP
    // endpoint is configured. Production (LEPTOS_ENV=PROD) logs to stdout only.
    // Default to "DEV" when unset, matching leptos_config's own default.
    let logger_provider: Option<SdkLoggerProvider> = if std::env::var("LEPTOS_ENV").as_deref().unwrap_or("DEV") == "DEV"
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

    // Set placeholder visibility flag for dpe-core
    dpe_core::set_show_placeholder_values(dpe_config.show_placeholder_values);
    if dpe_config.show_placeholder_values {
        tracing::info!("Placeholder values (MISSING/CALCULATED) will be shown in the UI");
    }

    // Load Leptos configuration from Cargo.toml metadata
    let conf =
        get_configuration(None).expect("Leptos configuration missing — check Cargo.toml [package.metadata.leptos]");
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    let fathom_site_id = dpe_config.fathom_site_id.clone();

    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

    let app = Router::new()
        // --- Traced routes ---
        // Routes declared BEFORE .layer() calls are wrapped by those layers.
        .route("/dpe/oai", get(dpe_api_oai::oai_handler))
        .route("/dpe/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
        .route("/dpe/projects/search", get(fragments::search_fragment_handler))
        .route("/dpe/api/v1/projects", get(fragments::projects_json_handler))
        .route("/dpe/api/v1/projects/{id}", get(fragments::project_json_handler))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            let fathom_site_id = fathom_site_id.clone();
            move || {
                let traceparent = traceparent::extract_traceparent();
                shell(leptos_options.clone(), fathom_site_id.clone(), traceparent)
            }
        })
        .fallback(leptos_axum::file_and_error_handler({
            let fathom_site_id = fathom_site_id.clone();
            move |options| shell(options, fathom_site_id.clone(), None)
        }))
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
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
        .with_state(leptos_options);

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
