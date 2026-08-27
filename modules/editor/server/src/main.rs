//! `editor-server` — the metadata editor's composition root: configuration,
//! observability setup, router assembly, and the HTTP server.
//!
//! Deliberately a separate service from `dpe-server`: the editor is
//! authenticated and writes state, DPE is public and read-only. They share the
//! `platform-telemetry` beacon contract, `platform-metadata` for the research
//! metadata contract, and `mosaic-tiles` for components — but not a process, an
//! image or an origin.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};

mod accounts;
mod auth;
mod config;
mod csrf;
mod db;
mod depositors;
mod mail;
mod page_url;
mod projects;
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
/// startup, and both the store and the mailer are behind an `Arc`.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Unhashed in dev, content-hashed in release.
    css_href: String,
    /// Behind the ports rather than the concrete [`db::Database`], so a test can
    /// make a chosen storage call fail. Several branches in the auth flow exist
    /// only to handle exactly that, and with a concrete `Database` here none of
    /// them could be driven. Call sites still go through the port that declares
    /// the method, so this widens nothing.
    db: std::sync::Arc<dyn editor_core::repository::Repositories>,
    /// Behind a trait rather than the concrete transport, so a test can watch
    /// what was sent and make sending fail — the failure path is where the code
    /// rollback and the break-glass decision live.
    mailer: std::sync::Arc<dyn mail::Mailer>,
    auth: auth::AuthConfig,
    /// Whether the code-entry screen may show the login code instead of
    /// requiring it to be read out of the log. Resolved once at startup from
    /// [`config::EditorConfig::reveals_login_code`] — never re-derived, so there
    /// is one answer per process and one place that decides it.
    reveal_login_code: bool,
}

/// Render a page inside the document shell.
///
/// The one place a `Markup` becomes a `Response`, so the traceparent meta tag,
/// the resolved stylesheet href and the signed-in header cannot be forgotten by
/// one handler and remembered by the rest.
///
/// `viewer` is the signed-in account or `None`, and it is a `&User` rather than
/// a pre-built [`editor_web::view::Viewer`] so that no call site has to decide
/// which field of an account belongs in a header. That decision is here, once:
/// the **name**, never the address. The header is on every page and in every
/// screenshot of one.
pub(crate) fn render(
    state: &AppState,
    title: &str,
    status: axum::http::StatusCode,
    viewer: Option<&editor_core::records::User>,
    content: maud::Markup,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let traceparent = traceparent::extract_traceparent();
    let viewer = viewer.map(|user| editor_web::view::Viewer { name: &user.name });
    let body = editor_web::view::page(title, traceparent.as_deref(), &state.css_href, viewer, content);
    (status, axum::response::Html(body.into_string())).into_response()
}

/// A 403 rendered as a page (REQ-1.3).
///
/// The status is what the requirement asks for; the page is because a bare 403
/// is a dead end in a browser. Everything that reaches this is authenticated —
/// an unauthenticated request is redirected to login by the extractor — so the
/// header renders signed in and its links are a way out.
pub(crate) fn forbidden(
    state: &AppState,
    user: &editor_core::records::User,
    message: &str,
) -> axum::response::Response {
    render(
        state,
        "No access — DaSCH Metadata Editor",
        axum::http::StatusCode::FORBIDDEN,
        Some(user),
        editor_web::pages::forbidden::forbidden(message),
    )
}

/// `GET /` — a redirect to the project list.
///
/// Public, and the only public route that is not part of signing in. It reads no
/// session and renders nothing: `/projects` decides what this account may see,
/// and redirects to login if there is no account. Answering here instead would
/// mean two places that know what a signed-out visitor gets.
///
/// It exists because the shell's header links here from every page — without the
/// route, the logo on the 404 page led to another 404.
pub(crate) async fn root() -> axum::response::Redirect {
    axum::response::Redirect::to("/projects")
}

/// 404 fallback, reached after `ServeDir` finds no matching static file, by a
/// project path that could never name a project, and by an account id that
/// names none.
///
/// Deliberately renders signed out even for a signed-in reader. It is reached
/// from `ServeDir`'s not-found service, which has no session in hand, and a 404
/// that showed a name on one route and not another would be stranger than one
/// that never does.
pub(crate) async fn not_found(axum::extract::State(state): axum::extract::State<AppState>) -> axum::response::Response {
    let content = maud::html! {
        h1 class="font-display text-2xl mb-2" { "Page not found" }
        p { "The page you asked for does not exist." }
    };
    render(
        &state,
        "Page not found — DaSCH Metadata Editor",
        axum::http::StatusCode::NOT_FOUND,
        None,
        content,
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
        // Behind an `Arc` from here on: `AppState` holds the ports rather than
        // the concrete store, and the cleanup task below needs a handle of its
        // own. One allocation at startup, and every call site is a deref.
        Ok(db) => std::sync::Arc::new(db),
        Err(e) => {
            tracing::error!(error = %e, "failed to open the database");
            return ExitCode::FAILURE;
        }
    };

    // RDU members exist without provisioning (REQ-7.2), so the configured ones
    // are created or promoted on every start. Fatal if it fails: an
    // administrator who cannot exist means nobody can administer the service,
    // and carrying on would hide that until someone tried to sign in.
    match accounts::ensure_rdu(&*db, &config.rdu_addresses(), chrono::Utc::now()).await {
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
    // of the database. Sessions accumulate the same way, and the send log is
    // append-only, so this is the only thing that bounds it at all.
    //
    // Detached rather than awaited on shutdown: it holds no state worth
    // draining, and the next start sweeps whatever a stopped process left.
    {
        const CLEANUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 60);
        let db = std::sync::Arc::clone(&db);
        tokio::spawn(async move {
            use editor_core::repository::{LoginCodeRepository, MailSendRepository, SessionRepository};

            let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
            // The first tick completes immediately; the sweep is for what has
            // aged out, so start one interval in.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let now = chrono::Utc::now();
                // The send log is pruned at the caps' own window, from the same
                // constant the caps measure with. A shorter retention would
                // silently hand back budget that was spent; a longer one would
                // keep rows nothing reads.
                let window_start = now - auth::delta(config::SEND_WINDOW);
                match (
                    LoginCodeRepository::delete_expired(&*db, now).await,
                    SessionRepository::delete_expired(&*db, now).await,
                    MailSendRepository::delete_before(&*db, window_start).await,
                ) {
                    (Ok(codes), Ok(sessions), Ok(sends)) if codes > 0 || sessions > 0 || sends > 0 => {
                        tracing::info!(codes, sessions, sends, "swept expired login codes, sessions and send records");
                    }
                    (Ok(_), Ok(_), Ok(_)) => {}
                    (codes, sessions, sends) => {
                        // Not fatal — nothing depends on the sweep succeeding,
                        // and it runs again in an hour.
                        if let Err(error) = codes {
                            tracing::warn!(error = %error, "could not sweep expired login codes");
                        }
                        if let Err(error) = sessions {
                            tracing::warn!(error = %error, "could not sweep expired sessions");
                        }
                        if let Err(error) = sends {
                            tracing::warn!(error = %error, "could not prune the mail send log");
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

    // Loud, and at `warn`. This deployment puts a live credential on a page, and
    // an operator scanning startup should see that stated rather than infer it
    // from three unset variables.
    if config.reveals_login_code() {
        tracing::warn!(
            "showing login codes on screen: no mail relay, no persistent database, and EDITOR_ENV is not PROD. \
             Intended for the PR preview and local runs. Setting EDITOR_SMTP_HOST or EDITOR_DB_DIR turns it off, \
             and EDITOR_ENV=PROD can never turn it on"
        );
    }

    let state = AppState {
        css_href: resolve_css_href(&config.public_dir),
        db,
        mailer,
        auth: auth::AuthConfig::from(&config),
        reveal_login_code: config.reveals_login_code(),
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
    use axum::response::IntoResponse;

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
        let response = not_found(axum::extract::State(state)).await;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        // A 404 that is a bare status string is a dead end in a browser; it has
        // to arrive inside the shell, with the header's route back.
        let body = test_support::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("DaSCH Metadata Editor"), "{body}");
        assert!(body.contains("Page not found"), "{body}");
    }

    #[tokio::test]
    async fn forbidden_renders_the_page_shell_with_a_403_and_a_way_out() {
        // REQ-1.3 asks for the status; the page is what stops it being a dead
        // end. The reader is signed in, so the shell's header renders their name
        // and its links are a route out.
        let (state, _) = test_support::test_state("forbidden").await;
        let user = editor_core::records::User {
            id: uuid::Uuid::new_v4(),
            email: "a.depositor@example.test".to_string(),
            name: "A Depositor".to_string(),
            role: editor_core::records::Role::Depositor,
            shortcodes: vec!["0801".to_string()],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: chrono::Utc::now(),
        };

        let response = forbidden(&state, &user, "This project is not assigned to your account.");
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        let body = test_support::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("This project is not assigned to your account."), "{body}");
        assert!(body.contains(r#"<a href="/projects""#), "{body}");
        assert!(body.contains("A Depositor"), "{body}");
        // The header shows the name, never the address — it is on every page and
        // in every screenshot of one.
        assert!(!body.contains("a.depositor@example.test"), "{body}");
    }

    #[tokio::test]
    async fn the_root_redirects_to_the_project_list() {
        // One place decides what a signed-out visitor gets, and it is
        // `/projects`. Answering here as well would be a second.
        let response = root().await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(test_support::location(&response).as_deref(), Some("/projects"));
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
