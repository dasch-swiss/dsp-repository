//! Application configuration.
//!
//! Layered configuration following Twelve-Factor App Factor III:
//! 1. Defaults (in code)
//! 2. Config file (optional `dpe.toml`)
//! 3. Environment variables (`DPE_*` prefix, override all)

use std::path::PathBuf;

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

/// DPE application configuration.
///
/// Loaded from defaults → `dpe.toml` (optional) → `DPE_*` env vars.
#[derive(Debug, Deserialize, Serialize)]
pub struct DpeConfig {
    /// Directory containing project/record JSON data files.
    /// Default: `modules/dpe/server/data` (resolved relative to the working
    /// directory, which is the workspace root under `just dev` / the e2e runner).
    /// Production: set via `DPE_DATA_DIR` or `DATA_DIR` env var.
    pub data_dir: PathBuf,

    /// Directory served as static assets by `ServeDir` (favicon, logo, vendored
    /// JS, project images, and the compiled `app.css`). Default:
    /// `modules/dpe/public`. Set via `DPE_PUBLIC_DIR`.
    pub public_dir: PathBuf,

    /// Fathom Analytics site ID. If set, the tracking script is injected into the HTML shell.
    /// Not a secret (visible in page source). Set via `DPE_FATHOM_SITE_ID`.
    pub fathom_site_id: Option<String>,

    /// Whether to display placeholder values ("MISSING", "CALCULATED") in the UI.
    /// When false (default/production), placeholders are hidden entirely.
    /// When true (DEV/STAGE), they are shown styled in red for QA visibility.
    /// Set via `DPE_SHOW_PLACEHOLDER_VALUES`.
    pub show_placeholder_values: bool,

    /// Public base URL at which the OAI-PMH endpoint is reachable. Emitted as the
    /// OAI-PMH `baseURL` and echoed in every `<request>` element, so it must match the
    /// URL harvesters actually use (e.g. `https://repository.dasch.swiss/dpe/oai` in
    /// production, `https://api.dev.dasch.swiss/dpe/oai` on DEV). Set via `DPE_OAI_BASE_URL`.
    ///
    /// Not origin-validated the way [`DpeConfig::public_base_url`] is: this one
    /// legitimately carries a path (`/dpe/oai`), which that check exists to reject.
    pub oai_base_url: String,

    /// OAI-PMH rate limit: seconds per request once burst is spent. Set via
    /// `DPE_OAI_RATE_LIMIT_PER_SECOND`.
    pub oai_rate_limit_per_second: u64,

    /// OAI-PMH rate limit: back-to-back requests allowed per IP. Set via
    /// `DPE_OAI_RATE_LIMIT_BURST`.
    pub oai_rate_limit_burst: u32,

    /// Public base URL at which the site itself is reachable, used to build the
    /// landing-page and machine-readable-representation URLs that the embedded
    /// metadata and the `Link` header carry. Origin only, no trailing slash.
    ///
    /// Independent of [`DpeConfig::oai_base_url`] and never derived from it: the
    /// OAI endpoint advertises its own `baseURL`, which on DEV lives on a
    /// different host. Set via `DPE_PUBLIC_BASE_URL`.
    pub public_base_url: String,
}

impl Default for DpeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("modules/dpe/server/data"),
            public_dir: PathBuf::from("modules/dpe/public"),
            fathom_site_id: None,
            show_placeholder_values: false,
            oai_base_url: "https://repository.dasch.swiss/dpe/oai".to_string(),
            oai_rate_limit_per_second: 1,
            oai_rate_limit_burst: 60,
            public_base_url: "https://repository.dasch.swiss".to_string(),
        }
    }
}

impl DpeConfig {
    /// Load configuration from defaults → dpe.toml → DPE_* env vars.
    pub fn load() -> Result<Self, Box<figment::Error>> {
        let config: Self = Figment::new()
            .merge(Serialized::defaults(DpeConfig::default()))
            .merge(Toml::file("dpe.toml"))
            .merge(Env::prefixed("DPE_"))
            // Also respect the legacy DATA_DIR env var (used in Dockerfile)
            .merge(Env::raw().only(&["DATA_DIR"]).map(|_| "data_dir".into()))
            .extract()
            .map_err(Box::new)?;
        validate_public_base_url(&config.public_base_url).map_err(|message| Box::new(figment::Error::from(message)))?;
        Ok(config)
    }
}

/// An origin and nothing else: `http` or `https`, a non-empty host, and no
/// path, query, fragment or trailing slash.
///
/// Every identifier the landing page emits is this value followed by a path the
/// code builds, so a value carrying its own path or slash would silently
/// produce doubled separators in an ARK-adjacent URL. Startup is the right
/// place to refuse it; a hand-rolled check is enough for a rule this narrow and
/// keeps the `url` crate out of the dependency list.
fn validate_public_base_url(value: &str) -> Result<(), String> {
    let host = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| format!("DPE_PUBLIC_BASE_URL must start with http:// or https:// (got {value:?})"))?;
    if host.is_empty() {
        return Err(format!("DPE_PUBLIC_BASE_URL has no host (got {value:?})"));
    }
    if let Some(bad) = host.chars().find(|c| matches!(c, '/' | '?' | '#')) {
        return Err(format!(
            "DPE_PUBLIC_BASE_URL must be an origin with no path, query or trailing slash (got {value:?}, found {bad:?})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let config = DpeConfig::default();
        assert_eq!(config.data_dir, PathBuf::from("modules/dpe/server/data"));
        assert_eq!(config.public_dir, PathBuf::from("modules/dpe/public"));
        assert!(config.fathom_site_id.is_none());
        assert!(!config.show_placeholder_values);
        assert_eq!(config.oai_base_url, "https://repository.dasch.swiss/dpe/oai");
        assert_eq!(config.oai_rate_limit_per_second, 1);
        assert_eq!(config.oai_rate_limit_burst, 60);
        assert_eq!(config.public_base_url, "https://repository.dasch.swiss");
    }

    #[test]
    fn public_base_url_accepts_an_origin() {
        assert!(validate_public_base_url("https://repository.dasch.swiss").is_ok());
        assert!(validate_public_base_url("http://host.docker.internal:4000").is_ok());
    }

    #[test]
    fn public_base_url_rejects_anything_past_the_origin() {
        for bad in [
            "https://repository.dasch.swiss/",
            "https://repository.dasch.swiss/dpe",
            "https://repository.dasch.swiss?x=1",
            "https://repository.dasch.swiss#f",
            "ftp://repository.dasch.swiss",
            "repository.dasch.swiss",
            "https://",
        ] {
            assert!(validate_public_base_url(bad).is_err(), "{bad} should be rejected");
        }
    }

    // The rule is tested on `validate_public_base_url` rather than through
    // `DpeConfig::load`: `figment::Jail` sets the variable in the *process*
    // environment, so a jailed test that made `load` fail would make it fail
    // for every other test loading a config at the same moment.

    #[test]
    fn oai_rate_limit_burst_env_override() {
        // Jail isolates env + cwd so DPE_* overrides are tested without touching the real
        // environment.
        figment::Jail::expect_with(|jail| {
            jail.set_env("DPE_OAI_RATE_LIMIT_BURST", 5);
            let config = DpeConfig::load().expect("config should load");
            assert_eq!(config.oai_rate_limit_burst, 5);
            Ok(())
        });
    }

    #[test]
    fn load_with_defaults() {
        let config = DpeConfig::load().expect("default config should load");
        assert!(config.fathom_site_id.is_none());
    }
}
