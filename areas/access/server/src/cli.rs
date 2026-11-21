//! The `access-server` CLI: argument parsing and the `healthcheck` subcommand.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "access-server", about = "DaSCH Discovery and Presentation Environment")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Start the web server
    Serve,
    /// Validate DPE's data files, then every CPE project, and fail if either fails
    ///
    /// CPE's projects are read as `serve` reads them (`CPE_PROJECTS_DIR`, `SYNC_DATA_DIR`,
    /// `MEDIA_DATA_DIR`), whether or not `CPE_BASE` is set. Every project's report is printed; a
    /// project that fails to load fails the run, and one with no Ready token is a warning.
    Validate {
        /// Path to DPE's data directory containing projects/, persons/, organizations/, records/
        data_dir: PathBuf,
    },
    /// Check if the server is healthy (for Docker HEALTHCHECK)
    Healthcheck {
        #[arg(long, default_value = "http://localhost:8080/healthz")]
        url: String,
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
