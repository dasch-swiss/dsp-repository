//! The `deposit-server` CLI: argument parsing, and the `healthcheck` and `vre-bridge-refresh`
//! subcommands.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_sdk::metrics::SdkMeterProvider;

#[derive(Parser)]
#[command(name = "deposit-server", about = "DaSCH project metadata editor")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Start the web server
    Serve,
    /// Check if the server is healthy (for Docker HEALTHCHECK)
    Healthcheck {
        #[arg(long, default_value = "http://localhost:8080/healthz")]
        url: String,
    },
    /// Refresh one allowlisted project from its DSP server now, forced, with the deployment's VRE
    /// bridge settings. Exit status: 0 unchanged or refreshed, 2 refused, 1 failed, 75 a nightly
    /// run or another refresh holds the lock (try again once it has ended), 64 the bridge is not
    /// configured, the project is not allowlisted or the configuration does not load
    VreBridgeRefresh {
        /// The project's shortcode, e.g. 0803
        shortcode: String,
    },
}

/// Whether `url` is one the healthcheck may call: loopback only, so a tampered Docker
/// `HEALTHCHECK` flag cannot turn the container into an SSRF probe.
///
/// The host must end at a port, a path or end of string: a bare `starts_with` accepts
/// `http://localhost.evil.com` and `http://localhost@evil.com`.
fn is_allowed_healthcheck_url(url: &str) -> bool {
    ["http://localhost", "http://127.0.0.1", "http://[::1]"].iter().any(|prefix| {
        url.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('/'))
    })
}

pub(crate) fn healthcheck(url: &str) -> std::process::ExitCode {
    if !is_allowed_healthcheck_url(url) {
        eprintln!("healthcheck: only localhost URLs are allowed, got: {url}");
        return std::process::ExitCode::FAILURE;
    }

    let agent: ureq::Agent = ureq::config::Config::builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    match agent.get(url).call() {
        Ok(response) => {
            if response.status() == 200 {
                std::process::ExitCode::SUCCESS
            } else {
                eprintln!("healthcheck: unexpected status {}", response.status());
                std::process::ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("healthcheck: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// The operator's refresh, in its own process beside the server's: the configuration as `serve`
/// loads it, the bridge's lock without waiting, and one forced refresh, whose outcome line is the
/// record. It runs without `init_otel`, so it exports nothing.
pub(crate) fn vre_bridge_refresh(shortcode: &str) -> ExitCode {
    crate::observability::install_tracing_panic_hook();
    crate::observability::install_crypto_provider();

    let config = match crate::config::EditorConfig::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("deposit-server: failed to load configuration: {e}");
            return ExitCode::from(vre_bridge_server::EXIT_UNUSABLE);
        }
    };
    crate::observability::init_plain_logging();

    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "no tokio runtime could be built; nothing is refreshed");
            return ExitCode::FAILURE;
        }
    };
    // A meter provider with no reader: this process's series would collide with the server's.
    let meter = SdkMeterProvider::builder().build().meter(vre_bridge_server::METER_SCOPE);
    let bridge = vre_bridge_server::VreBridge::from_raw(config.vre_bridge_settings(), &meter);
    runtime.block_on(bridge.operator_refresh(shortcode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_serve_and_healthcheck() {
        use clap::CommandFactory;
        Cli::command().debug_assert();

        assert!(matches!(
            Cli::parse_from(["deposit-server", "serve"]).command,
            Some(Commands::Serve)
        ));
        let Some(Commands::Healthcheck { url }) = Cli::parse_from(["deposit-server", "healthcheck"]).command else {
            panic!("expected healthcheck");
        };
        assert_eq!(url, "http://localhost:8080/healthz");
        let Some(Commands::VreBridgeRefresh { shortcode }) =
            Cli::parse_from(["deposit-server", "vre-bridge-refresh", "0803"]).command
        else {
            panic!("expected vre-bridge-refresh");
        };
        assert_eq!(shortcode, "0803");
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
}
