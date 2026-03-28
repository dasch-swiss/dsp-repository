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
    /// Default: `modules/dpe/server/data` (for cargo-leptos dev mode).
    /// Production: set via `DPE_DATA_DIR` or `DATA_DIR` env var.
    pub data_dir: PathBuf,
}

impl Default for DpeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("modules/dpe/server/data"),
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
    }

    #[test]
    fn load_with_defaults() {
        // Without any env vars or config file, defaults should work
        let _config = DpeConfig::load().expect("default config should load");
    }
}
