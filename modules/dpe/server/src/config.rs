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
/// Leptos-specific options are loaded separately via `get_configuration(None)`.
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
    pub oai_base_url: String,
}

impl Default for DpeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("modules/dpe/server/data"),
            public_dir: PathBuf::from("modules/dpe/public"),
            fathom_site_id: None,
            show_placeholder_values: false,
            oai_base_url: "https://repository.dasch.swiss/dpe/oai".to_string(),
        }
    }
}

impl DpeConfig {
    /// Load configuration from defaults → dpe.toml → DPE_* env vars.
    pub fn load() -> Result<Self, Box<figment::Error>> {
        Figment::new()
            .merge(Serialized::defaults(DpeConfig::default()))
            .merge(Toml::file("dpe.toml"))
            .merge(Env::prefixed("DPE_"))
            // Also respect the legacy DATA_DIR env var (used in Dockerfile)
            .merge(Env::raw().only(&["DATA_DIR"]).map(|_| "data_dir".into()))
            .extract()
            .map_err(Box::new)
    }
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
    }

    #[test]
    fn load_with_defaults() {
        // Without any env vars or config file, defaults should work
        let config = DpeConfig::load().expect("default config should load");
        assert!(config.fathom_site_id.is_none());
    }
}
