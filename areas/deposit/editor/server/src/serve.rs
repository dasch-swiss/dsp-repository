//! `serve()`, its startup loaders and `shutdown_signal`.

use std::sync::atomic::Ordering;

use crate::observability::SUBSCRIBER_READY;

/// The temporal-resolution tables, or empty ones without a data directory. Empty refuses
/// every free-text period at submit, the fail-safe direction.
fn load_temporal(data_dir: Option<&std::path::Path>) -> crate::shell::TemporalTables {
    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the temporal-resolution tables are empty, so every free-text temporalCoverage \
             entry reads as unresolvable and no submission carrying one can be made"
        );
        return crate::shell::TemporalTables::default();
    };
    let tables = crate::shell::TemporalTables {
        periods: shared_metadata::chronontology::load_from(data_dir),
        enrichment: shared_metadata::temporal_enrichment::load_from(data_dir),
    };
    tracing::info!(
        periods = tables.periods.len(),
        enrichment = tables.enrichment.len(),
        "temporal-resolution tables loaded"
    );
    tables
}

/// Read the published project set, reporting what did not load. Never fatal: an unset
/// `EDITOR_DATA_DIR` is a configured state, and a malformed file should not take down routes
/// that never read the projects.
fn load_published(data_dir: Option<&std::path::Path>) -> editor_core::published::PublishedProjects {
    use editor_core::published::PublishedProjects;

    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the published project set is empty, so the project list shows nothing and no \
             form can be pre-filled"
        );
        return PublishedProjects::default();
    };
    let (published, errors) = PublishedProjects::load_from(&data_dir.join("projects"));
    // One line per failure, so a bad file is findable by name rather than by
    // subtracting counts.
    for error in &errors {
        tracing::warn!(error = %error, "a published project could not be read");
    }
    if errors.is_empty() {
        tracing::info!(projects = published.len(), "published project set loaded");
    } else {
        tracing::warn!(
            projects = published.len(),
            failed = errors.len(),
            "published project set loaded with failures"
        );
    }
    published
}

/// The agent set, or an empty one without a data directory.
fn load_agents(data_dir: Option<&std::path::Path>) -> editor_core::agents::Agents {
    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the agent set is empty, so every contributor and contact reference renders as a bare id and no submission naming one can be accepted"
        );
        return editor_core::agents::Agents::default();
    };
    let (agents, errors) =
        editor_core::agents::Agents::load_from(&data_dir.join("persons"), &data_dir.join("organizations"));
    for error in &errors {
        tracing::warn!(error = %error, "an agent could not be read");
    }
    if errors.is_empty() {
        tracing::info!(agents = agents.len(), "agent set loaded");
    } else {
        tracing::warn!(agents = agents.len(), failed = errors.len(), "agent set loaded with failures");
    }
    agents
}

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
            eprintln!("editor-server: failed to load configuration: {e}");
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
    // only while a connection to it is open.
    let db = match crate::db::Database::open(config.db_source(), config.db_readers, config.db_busy_timeout()).await {
        // Behind an `Arc`: `AppState` holds the ports, and the cleanup task needs a handle.
        Ok(db) => std::sync::Arc::new(db),
        Err(e) => {
            tracing::error!(error = %e, "failed to open the database");
            return ExitCode::FAILURE;
        }
    };

    // RDU members are created or promoted on every start. Fatal on failure: with no
    // administrator nobody can administer the service.
    match crate::accounts::ensure_rdu(&*db, &config.rdu_addresses(), chrono::Utc::now()).await {
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

    // A misconfigured relay stops the process here rather than at the first login.
    let mailer: std::sync::Arc<dyn crate::mail::Mailer> = match &config.smtp_host {
        Some(host) => {
            match crate::mail::SmtpMailer::new(host, config.smtp_port, config.smtp_credentials(), &config.smtp_from) {
                Ok(mailer) => std::sync::Arc::new(mailer),
                Err(e) => {
                    tracing::error!(error = %e, "failed to configure the SMTP relay");
                    return ExitCode::FAILURE;
                }
            }
        }
        // The PR preview's default: with no relay the service stays
        // usable and codes go to the log.
        None => std::sync::Arc::new(crate::mail::ConsoleMailer),
    };
    tracing::info!(mail.transport = %mailer.describe(), "mail transport ready");

    // The only thing that bounds unused login codes, sessions and the append-only send log.
    // Detached rather than drained on shutdown: the next start sweeps what is left.
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
                // Pruned at the caps' own window: a shorter one would hand back spent budget.
                let window_start = now - crate::auth::delta(crate::config::SEND_WINDOW);
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

    // Loud, at `warn`: this deployment puts a live credential on a page.
    if config.reveals_login_code() {
        tracing::warn!(
            "showing login codes on screen: no mail relay, no persistent database, and EDITOR_ENV is not PROD. \
             Intended for the PR preview and local runs. Setting EDITOR_SMTP_HOST or EDITOR_DB_DIR turns it off, \
             and EDITOR_ENV=PROD can never turn it on"
        );
    }

    let published = std::sync::Arc::new(load_published(config.data_dir.as_deref()));
    let temporal = std::sync::Arc::new(load_temporal(config.data_dir.as_deref()));
    let agents = std::sync::Arc::new(load_agents(config.data_dir.as_deref()));

    // Once per process: the moment a deployment starts is the moment its approved changes are
    // Online. Not fatal, see `reconcile`'s module docs.
    match crate::reconcile::reconcile_published(&*db, &*db, &published, &agents).await {
        // Two spelled-out arms rather than one parameterised call: `tracing`
        // resolves the level at compile time, so it cannot be a variable.
        Ok(summary) if summary.needs_attention() => tracing::warn!(
            projects.online = summary.online,
            projects.waiting = summary.waiting,
            projects.stranded = summary.stranded,
            projects.removed_upstream = summary.removed_upstream,
            records.unreadable = summary.unreadable,
            records.retry_failed = summary.retry_failed,
            proposals.retired = summary.proposals_retired,
            proposals.retire_failed = summary.proposals_retire_failed,
            "compared the published set against local records; some records need an RDU decision"
        ),
        Ok(summary) => tracing::info!(
            projects.online = summary.online,
            projects.waiting = summary.waiting,
            projects.stranded = summary.stranded,
            projects.removed_upstream = summary.removed_upstream,
            records.unreadable = summary.unreadable,
            records.retry_failed = summary.retry_failed,
            proposals.retired = summary.proposals_retired,
            proposals.retire_failed = summary.proposals_retire_failed,
            "compared the published set against local records"
        ),
        Err(e) => tracing::error!(
            error = %e,
            "could not compare the published set against local records; every project keeps its stored state and \
             the comparison is retried on the next start"
        ),
    }

    let state = crate::shell::AppState {
        css_href: crate::assets::resolve_css_href(&config.public_dir),
        db,
        mailer,
        auth: crate::auth::AuthConfig::from(&config),
        reveal_login_code: config.reveals_login_code(),
        collection_token: config.collection_token.clone(),
        published,
        temporal,
        agents,
    };
    let app = crate::router::build_app(state, &config.public_dir);

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
