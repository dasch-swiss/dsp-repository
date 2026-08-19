//! Application configuration.
//!
//! Layered configuration following Twelve-Factor App Factor III, mirroring
//! `DpeConfig`:
//! 1. Defaults (in code)
//! 2. Config file (optional `editor.toml`)
//! 3. Environment variables (`EDITOR_*` prefix, override all)

use std::path::PathBuf;

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

/// Metadata editor configuration.
///
/// Loaded from defaults → `editor.toml` (optional) → `EDITOR_*` env vars.
#[derive(Debug, Deserialize, Serialize)]
pub struct EditorConfig {
    /// Address the HTTP server binds to. Default `127.0.0.1:4100` — deliberately
    /// not DPE's 4000, so both services can run locally at the same time.
    /// Production: `0.0.0.0:8080` via `EDITOR_SITE_ADDR`.
    pub site_addr: String,

    /// Directory served as static assets by `ServeDir` (vendored JS, the
    /// telemetry beacon, and the compiled `app.css`). Default
    /// `modules/editor/public`, resolved relative to the working directory,
    /// which is the workspace root under `just dev-editor`. Set via
    /// `EDITOR_PUBLIC_DIR`.
    pub public_dir: PathBuf,

    /// Directory holding the published project/person/organization set baked
    /// into the image, set via `EDITOR_DATA_DIR`. Deliberately has **no
    /// default**: the only plausible one is a relative path into DPE's tree,
    /// which the editor does not own, and which would hide the dependency from
    /// the code that reads records. Every environment that reads them sets the
    /// variable — the image to `/app/server/data`, `just dev-editor` to DPE's
    /// checked-out data. `None` therefore means "no data directory
    /// configured", and is reported as such at startup.
    pub data_dir: Option<PathBuf>,

    /// Deployment environment, `DEV` or `PROD`. `DEV` additionally exports logs
    /// over OTLP when an endpoint is configured; `PROD` logs to stdout only.
    /// Set via `EDITOR_ENV`.
    pub env: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            site_addr: "127.0.0.1:4100".to_string(),
            public_dir: PathBuf::from("modules/editor/public"),
            data_dir: None,
            env: "DEV".to_string(),
        }
    }
}

impl EditorConfig {
    /// Load configuration from defaults → `editor.toml` → `EDITOR_*` env vars.
    pub fn load() -> Result<Self, Box<figment::Error>> {
        Figment::new()
            .merge(Serialized::defaults(EditorConfig::default()))
            .merge(Toml::file("editor.toml"))
            .merge(Env::prefixed("EDITOR_"))
            .extract()
            .map_err(Box::new)
    }

    /// Whether logs should additionally be exported over OTLP. True only in
    /// `DEV`; production logs to stdout and is scraped from there.
    pub fn exports_otlp_logs(&self) -> bool {
        self.env == "DEV"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let config = EditorConfig::default();
        assert_eq!(config.site_addr, "127.0.0.1:4100");
        assert_eq!(config.public_dir, PathBuf::from("modules/editor/public"));
        assert_eq!(config.data_dir, None);
        assert_eq!(config.env, "DEV");
    }

    #[test]
    fn default_site_addr_parses_and_differs_from_dpe() {
        // The default is a bind address, and it must not collide with DPE's
        // 4000 — both run locally during editor development.
        let addr: std::net::SocketAddr = EditorConfig::default().site_addr.parse().expect("default must parse");
        assert_ne!(addr.port(), 4000);
    }

    #[test]
    fn site_addr_env_override() {
        // Jail isolates env + cwd so EDITOR_* overrides are tested without
        // touching the real environment.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SITE_ADDR", "0.0.0.0:8080");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.site_addr, "0.0.0.0:8080");
            Ok(())
        });
    }

    #[test]
    fn public_dir_env_override() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_PUBLIC_DIR", "/app/public");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.public_dir, PathBuf::from("/app/public"));
            Ok(())
        });
    }

    #[test]
    fn data_dir_env_override() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_DATA_DIR", "/app/server/data");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.data_dir, Some(PathBuf::from("/app/server/data")));
            Ok(())
        });
    }

    #[test]
    fn data_dir_is_unset_without_the_env_var() {
        // No default on purpose. The only plausible one is a relative path into
        // DPE's tree (`modules/dpe/server/data`), which the editor does not own;
        // baking it in would let a records reader silently resolve another
        // module's directory instead of failing on an unconfigured seam.
        figment::Jail::expect_with(|_| {
            let config = EditorConfig::load().expect("config should load without EDITOR_DATA_DIR");
            assert_eq!(config.data_dir, None);
            Ok(())
        });
    }

    #[test]
    fn env_env_override_switches_otlp_log_export() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_ENV", "PROD");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.env, "PROD");
            assert!(!config.exports_otlp_logs());
            Ok(())
        });
    }

    #[test]
    fn toml_file_overrides_defaults_and_env_overrides_toml() {
        // Pins the layer order: editor.toml beats the code defaults, EDITOR_*
        // beats editor.toml.
        figment::Jail::expect_with(|jail| {
            jail.create_file("editor.toml", "site_addr = \"127.0.0.1:4200\"\nenv = \"PROD\"\n")?;
            let from_file = EditorConfig::load().expect("config should load");
            assert_eq!(from_file.site_addr, "127.0.0.1:4200");
            assert_eq!(from_file.env, "PROD");

            jail.set_env("EDITOR_SITE_ADDR", "127.0.0.1:4300");
            let from_env = EditorConfig::load().expect("config should load");
            assert_eq!(from_env.site_addr, "127.0.0.1:4300");
            Ok(())
        });
    }

    #[test]
    fn load_with_defaults() {
        // Without any env vars or config file, defaults should work.
        figment::Jail::expect_with(|_| {
            let config = EditorConfig::load().expect("default config should load");
            assert_eq!(config.env, "DEV");
            assert!(config.exports_otlp_logs());
            Ok(())
        });
    }
}
