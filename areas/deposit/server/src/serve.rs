//! `serve()` and `shutdown_signal`.

use std::sync::atomic::Ordering;

use crate::observability::SUBSCRIBER_READY;

#[tokio::main]
pub(crate) async fn serve() -> std::process::ExitCode {
    use std::process::ExitCode;

    // Before OTel init, so a panic during init is captured; the hook writes to stderr until
    // `SUBSCRIBER_READY` is set.
    crate::observability::install_tracing_panic_hook();

    // Before OTel, so the log-export decision reads the layered config. Reported rather than
    // `expect`ed: figment's error names the file, key and expected type.
    let config = match crate::config::EditorConfig::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("deposit-server: failed to load configuration: {e}");
            eprintln!("  checked: code defaults, ./editor.toml, then EDITOR_* environment variables");
            return ExitCode::FAILURE;
        }
    };

    let (logger_provider, _otel_guard) = crate::observability::init_otel(&config);

    // The panic hook emits structured events from here on.
    SUBSCRIBER_READY.store(true, Ordering::Release);

    let _pyroscope_agent = crate::observability::init_pyroscope();

    // Reported as unset rather than as an invented default path.
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

    // Opened before the listener binds, so a bad mount stops the process with a message naming
    // the cause. Held for the life of the process: a shared-cache in-memory database exists
    // only while a connection to it is open. Identity's first, then the editor's.
    let identity_db =
        match identity_store::Database::open(config.identity_db_source(), config.db_readers, config.db_busy_timeout())
            .await
        {
            Ok(db) => std::sync::Arc::new(db),
            Err(e) => {
                tracing::error!(error = %e, "failed to open identity's database");
                return ExitCode::FAILURE;
            }
        };
    let editor_db = match editor_store::Database::open(
        config.editor_db_source(),
        config.db_readers,
        config.db_busy_timeout(),
    )
    .await
    {
        Ok(db) => std::sync::Arc::new(db),
        Err(e) => {
            tracing::error!(error = %e, "failed to open the editor's database");
            return ExitCode::FAILURE;
        }
    };

    let css_href = crate::assets::resolve_css_href(&config.public_dir);
    let identity_settings = config.identity_settings(css_href.clone());
    let auth = identity_settings.auth();
    let identity = match identity_web::Identity::start(
        identity_settings,
        identity_db.clone(),
        std::sync::Arc::new(editor_store::LiveAuthoredWork::new(editor_db.clone())),
    )
    .await
    {
        Ok(identity) => identity,
        Err(e) => {
            tracing::error!(error = %e, "identity could not start");
            return ExitCode::FAILURE;
        }
    };

    let addr: std::net::SocketAddr = config
        .site_addr
        .parse()
        .unwrap_or_else(|e| panic!("invalid site address (EDITOR_SITE_ADDR) {:?}: {e}", config.site_addr));

    // Loud, at `warn`: this deployment puts a live credential on a page.
    if config.reveals_login_code() {
        tracing::warn!(
            "showing login codes on screen: no mail relay, no persistent database, and EDITOR_ENV is not PROD. \
             Intended for the PR preview and local runs. Setting EDITOR_SMTP_HOST or EDITOR_DB_DIR turns it off, \
             and EDITOR_ENV=PROD can never turn it on"
        );
    }

    let editor = editor_web::Editor::start(
        config.editor_settings(css_href.clone()),
        editor_db,
        std::sync::Arc::new(identity_store::LiveAuthenticator::new(identity_db.clone(), auth)),
        std::sync::Arc::new(identity_store::LiveUserDirectory::new(identity_db)),
    )
    .await;
    let app = crate::router::build_app(identity.router(), editor.router(), css_href, &config.public_dir);

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server exited with error");

    // Blocking I/O (thread joins, uploads, OTLP flush): run via spawn_blocking so the
    // Tokio runtime does not deadlock.
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
