//! `editor-server` — the metadata editor's composition root: configuration,
//! observability setup, router assembly, and the HTTP server.
//!
//! Deliberately a separate service from `dpe-server`: the editor is
//! authenticated and writes state, DPE is public and read-only. They share the
//! `platform-telemetry` beacon contract, and will share `mosaic-tiles` for components
//! and `dpe-core` for the data contract — but not a process, an image or an
//! origin.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};

mod accounts;
mod auth;
mod config;
mod csrf;
mod db;
mod mail;
mod page_url;
mod router;
#[cfg(test)]
mod test_support;
mod traceparent;

/// Whether a `tracing` subscriber is installed and will actually record events.
///
/// The panic hook needs this because `tracing::error!` with no subscriber is a
/// silent no-op, not an error — so before init, structured emission writes the
/// panic precisely nowhere. See [`install_tracing_panic_hook`].
static SUBSCRIBER_READY: AtomicBool = AtomicBool::new(false);

/// Shared state for the handlers.
///
/// Cheap to clone: the stylesheet href is a short string resolved once at
/// startup, `Database` is a pair of pool handles, and the mailer is behind an
/// `Arc` because a transport holds a connection pool of its own.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Unhashed in dev, content-hashed in release.
    css_href: String,
    db: db::Database,
    /// Behind a trait rather than the concrete transport, so a test can watch
    /// what was sent and make sending fail — the failure path is where the code
    /// rollback and the break-glass decision live.
    mailer: std::sync::Arc<dyn mail::Mailer>,
    auth: auth::AuthConfig,
}

/// `GET /` — the service root.
///
/// Exists now because the page shell's header links here from every page: without
/// a route, clicking the logo on the 404 page produced another 404, so every
/// navigation the shell offered was a dead end. Once the project list lands this
/// becomes the redirect to `/projects` that the URL scheme specifies.
///
/// Signed out it offers the way in, because otherwise `/login` is reachable only
/// by typing it.
pub(crate) async fn root(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    let name = auth::session::current(&state.db, &state.auth, &headers, chrono::Utc::now())
        .await
        .map(|user| user.name);
    let content = maud::html! {
        h1 class="font-display text-2xl mb-2" { "DaSCH Metadata Editor" }
        p { "Editing your project metadata is not available yet. This service is being built." }
        @if name.is_none() {
            p class="mt-4" {
                a href="/login" class="underline" { "Sign in" }
            }
        }
    };
    let viewer = name.as_deref().map(|name| editor_web::view::Viewer { name });
    axum::response::Html(
        editor_web::view::page("DaSCH Metadata Editor", tp.as_deref(), &state.css_href, viewer, content).into_string(),
    )
}

/// 404 fallback, reached after `ServeDir` finds no matching static file.
pub(crate) async fn not_found(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let tp = traceparent::extract_traceparent();
    let content = maud::html! {
        h1 class="font-display text-2xl mb-2" { "Page not found" }
        p { "The page you asked for does not exist." }
    };
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::response::Html(
            editor_web::view::page(
                "Page not found — DaSCH Metadata Editor",
                tp.as_deref(),
                &state.css_href,
                None,
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
    discover_hashed_css(&public_dir.join("assets")).unwrap_or_else(|| "/assets/app.css".to_string())
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

#[derive(Parser)]
#[command(name = "editor-server", about = "DaSCH project metadata editor")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve,
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
        Some(Commands::Healthcheck { url }) => healthcheck(&url),
    }
}

#[tokio::main]
async fn serve() -> ExitCode {
    use init_tracing_opentelemetry::TracingConfig;
    use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
    use tracing_subscriber::layer::SubscriberExt;

    // Route panics through tracing so they appear as structured Grafana logs
    // alongside normal traces. Installed before OTel init so a panic during init
    // is also captured — it writes to stderr until `SUBSCRIBER_READY` is set.
    install_tracing_panic_hook();

    // Load configuration before OTel so the log-export decision reads the same
    // layered config as everything else, rather than a raw env lookup.
    //
    // Reported explicitly rather than via `expect`: this is the likeliest startup
    // failure (a typo in `editor.toml` or an `EDITOR_*` value of the wrong type),
    // it happens before any subscriber exists, and figment's error names the file,
    // key and expected type — which is exactly what the operator needs and what a
    // panic message would bury.
    let config = match config::EditorConfig::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("editor-server: failed to load configuration: {e}");
            eprintln!("  checked: code defaults, ./editor.toml, then EDITOR_* environment variables");
            return ExitCode::FAILURE;
        }
    };

    // Export logs via OTLP only in DEV, and only when an endpoint is
    // configured. PROD logs to stdout and is scraped from there.
    let logger_provider: Option<SdkLoggerProvider> =
        if config.exports_otlp_logs() && std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_tonic()
                .build()
                .expect("failed to build OTLP log exporter");
            Some(SdkLoggerProvider::builder().with_batch_exporter(exporter).build())
        } else {
            None
        };

    // Initialize the OpenTelemetry tracing subscriber. Reads OTEL_* env vars
    // automatically and falls back to no-op export when
    // OTEL_EXPORTER_OTLP_ENDPOINT is unset, so local development needs no
    // collector. Log level is controlled via RUST_LOG.
    let _otel_guard = TracingConfig::production()
        .with_otel_tracer_name(env!("CARGO_PKG_NAME"))
        .init_subscriber_ext(|registry| {
            use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
            let otel_logs_layer: Option<OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>> =
                logger_provider.as_ref().map(OpenTelemetryTracingBridge::new);
            registry.with(otel_logs_layer)
        })
        .expect("failed to initialize OpenTelemetry tracing");

    // From here on the panic hook can emit structured events; before this point
    // it wrote to stderr, because `tracing::error!` without a subscriber is a
    // silent no-op.
    SUBSCRIBER_READY.store(true, Ordering::Release);

    // Continuous CPU profiling, active only when PYROSCOPE_ENDPOINT is set.
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
        .tags(vec![("service.namespace", "editor")])
        .build()
        .expect("failed to build Pyroscope agent");

        tracing::info!(endpoint = %endpoint, "Pyroscope profiling enabled");
        Some(agent.start().expect("failed to start Pyroscope agent"))
    } else {
        None
    };

    // An unset data directory is a legitimate state today (nothing reads
    // records yet), so report it rather than failing — but report it as unset,
    // not as some path the editor invented.
    let data_dir = config
        .data_dir
        .as_deref()
        .map_or_else(|| "<unset>".to_string(), |path| path.display().to_string());
    let db_dir = config
        .db_dir
        .as_deref()
        .map_or_else(|| "<unset, in-memory>".to_string(), |path| path.display().to_string());
    tracing::info!(
        env = %config.env,
        public_dir = %config.public_dir.display(),
        data_dir = %data_dir,
        db_dir = %db_dir,
        smtp_host = %config.smtp_host.as_deref().unwrap_or("<unset, console fallback>"),
        "editor configuration loaded"
    );

    // Persistence. Opened before the listener binds, so a bad mount or a
    // wrong-uid directory stops the process with a message naming the cause
    // rather than answering requests that fail one query at a time.
    //
    // Reported rather than panicked for the same reason as the config load: the
    // likely failures here are operational (a missing or wrongly-owned mount),
    // and `DbError`'s messages already say what to do about them, which a panic
    // would bury under a backtrace.
    //
    // Held for the life of the process, which the in-memory variant needs: a
    // shared-cache in-memory database exists only while a connection to it is
    // open.
    let db = match db::Database::open(config.db_source(), config.db_readers, config.db_busy_timeout()).await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(error = %e, "failed to open the database");
            return ExitCode::FAILURE;
        }
    };

    // RDU members exist without provisioning (REQ-7.2), so the configured ones
    // are created or promoted on every start. Fatal if it fails: an
    // administrator who cannot exist means nobody can administer the service,
    // and carrying on would hide that until someone tried to sign in.
    match accounts::ensure_rdu(&db, &config.rdu_addresses(), chrono::Utc::now()).await {
        Ok(changed) => tracing::info!(
            rdu.configured = config.rdu_addresses().len(),
            rdu.changed = changed,
            "RDU accounts reconciled with configuration"
        ),
        Err(e) => {
            tracing::error!(error = %e, "failed to create the configured RDU accounts");
            return ExitCode::FAILURE;
        }
    }

    // The mail transport. A relay that is misconfigured stops the process here
    // rather than at the first login: an unparseable `EDITOR_SMTP_FROM` makes
    // every send fail, and learning that from a user report is the expensive way.
    let mailer: std::sync::Arc<dyn mail::Mailer> = match &config.smtp_host {
        Some(host) => {
            match mail::SmtpMailer::new(host, config.smtp_port, config.smtp_credentials(), &config.smtp_from) {
                Ok(mailer) => std::sync::Arc::new(mailer),
                Err(e) => {
                    tracing::error!(error = %e, "failed to configure the SMTP relay");
                    return ExitCode::FAILURE;
                }
            }
        }
        // REQ-6.8, and the PR preview's default: with no relay the service stays
        // usable and codes go to the log.
        None => std::sync::Arc::new(mail::ConsoleMailer),
    };
    tracing::info!(mail.transport = %mailer.describe(), "mail transport ready");

    // Expired rows are deleted wherever the flow trips over them, but a code
    // nobody ever entered is tripped over by nothing: without a sweep, every
    // six-digit code ever issued stays in the table, in plaintext, for the life
    // of the database. Sessions accumulate the same way.
    //
    // Detached rather than awaited on shutdown: it holds no state worth
    // draining, and the next start sweeps whatever a stopped process left.
    {
        const CLEANUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 60);
        let db = db.clone();
        tokio::spawn(async move {
            use editor_core::repository::{LoginCodeRepository, SessionRepository};

            let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
            // The first tick completes immediately; the sweep is for what has
            // aged out, so start one interval in.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let now = chrono::Utc::now();
                match (
                    LoginCodeRepository::delete_expired(&db, now).await,
                    SessionRepository::delete_expired(&db, now).await,
                ) {
                    (Ok(codes), Ok(sessions)) if codes > 0 || sessions > 0 => {
                        tracing::info!(codes, sessions, "swept expired login codes and sessions");
                    }
                    (Ok(_), Ok(_)) => {}
                    (codes, sessions) => {
                        // Not fatal — nothing depends on the sweep succeeding,
                        // and it runs again in an hour.
                        if let Err(error) = codes {
                            tracing::warn!(error = %error, "could not sweep expired login codes");
                        }
                        if let Err(error) = sessions {
                            tracing::warn!(error = %error, "could not sweep expired sessions");
                        }
                    }
                }
            }
        });
    }

    let addr: std::net::SocketAddr = config
        .site_addr
        .parse()
        .unwrap_or_else(|e| panic!("invalid site address (EDITOR_SITE_ADDR) {:?}: {e}", config.site_addr));

    let state = AppState {
        css_href: resolve_css_href(&config.public_dir),
        db,
        mailer,
        auth: auth::AuthConfig::from(&config),
    };
    let app = router::build_app(state, &config.public_dir);

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server exited with error");

    // Stop Pyroscope and flush OTel. Both perform blocking I/O (condvar waits,
    // thread joins, HTTP uploads, OTLP flush), so they run via spawn_blocking
    // to avoid deadlocking the Tokio runtime.
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

/// Install a panic hook that emits panics as structured `tracing::error!`
/// events, so production panics are captured by the same OTel pipeline as the
/// rest of the logs. Field names follow the OTel semconv for exceptions
/// (`exception.message`, `exception.stacktrace`, `exception.r#type`,
/// `thread.name`) so panic events share Grafana Sift / Loki query surface with
/// `RecordException` events emitted by instrumented spans.
///
/// Until [`SUBSCRIBER_READY`] is set, the hook delegates to the default stderr
/// hook instead. This is load-bearing, not belt-and-braces: `tracing::error!`
/// with no registered subscriber is a **silent no-op**, and it does not panic, so
/// a `catch_unwind` around it reports success. Without the flag, every panic
/// between hook installation and subscriber init — including the OTel exporter
/// and subscriber `expect`s below — would produce no output whatsoever, and the
/// process would exit 101 with an empty log.
///
/// Once the subscriber is up, the stderr hook is only reached if the structured
/// emission itself panics (e.g. a degraded exporter), so a panic produces exactly
/// one log line and never a duplicate backtrace.
fn install_tracing_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !SUBSCRIBER_READY.load(Ordering::Acquire) {
            default_hook(info);
            return;
        }

        // Best-effort structured emission.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let message = info.payload_as_str().unwrap_or("<non-string panic payload>");
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            // `Backtrace::capture` respects `RUST_BACKTRACE` / `RUST_LIB_BACKTRACE`.
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
mod tests {
    use super::*;

    #[test]
    fn cli_parses_serve_and_healthcheck() {
        use clap::CommandFactory;
        Cli::command().debug_assert();

        assert!(matches!(
            Cli::parse_from(["editor-server", "serve"]).command,
            Some(Commands::Serve)
        ));
        let Some(Commands::Healthcheck { url }) = Cli::parse_from(["editor-server", "healthcheck"]).command else {
            panic!("expected healthcheck");
        };
        assert_eq!(url, "http://localhost:8080/healthz");
    }

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

    #[tokio::test]
    async fn not_found_renders_the_page_shell_with_a_404() {
        let (state, _) = test_support::test_state("not-found").await;
        let (status, body) = not_found(axum::extract::State(state)).await;
        assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
        // A 404 that is a bare status string is a dead end in a browser; it has
        // to arrive inside the shell, with the header's route back.
        assert!(body.0.starts_with("<!DOCTYPE html>"), "{}", body.0);
        assert!(body.0.contains("DaSCH Metadata Editor"), "{}", body.0);
        assert!(body.0.contains("Page not found"), "{}", body.0);
    }

    #[test]
    fn discovers_content_hashed_stylesheet() {
        let dir = std::env::temp_dir().join(format!("editor_css_discover_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.abc123.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir).as_deref(), Some("/assets/app.abc123.css"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_unhashed_stylesheet_and_returns_none() {
        // `just css-editor` (dev) leaves an unhashed app.css behind. Release
        // discovery must not latch onto it, or a stale dev build would be
        // served with no cache-busting.
        let dir = std::env::temp_dir().join(format!("editor_css_none_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_assets_dir_returns_none() {
        let dir = std::env::temp_dir().join(format!("editor_css_missing_{}", std::process::id()));
        assert_eq!(discover_hashed_css(&dir), None);
    }
}
