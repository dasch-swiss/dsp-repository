//! CPE's runtime configuration.
//!
//! Loaded with figment, in order of increasing priority: the defaults in
//! `Default`, an optional `cpe.toml` in the working directory, and `CPE_*`
//! environment variables.
//!
//! Loading never fails. CPE is one capability of the area binary, and a wrong
//! `CPE_*` value must not stop the others: a configuration that cannot be used
//! is kept with its reason, and `Cpe`'s router then answers 503 on every path.

use std::path::PathBuf;

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CpeConfig {
    /// The folder whose subfolders are the projects (`CPE_PROJECTS_DIR`).
    /// Relative to the working directory; the default fits a run from the
    /// repository root.
    #[serde(deserialize_with = "path_from_scalar")]
    pub projects_dir: PathBuf,

    /// The origin CPE's absolute links start with, e.g.
    /// `https://repository.dasch.swiss` (`CPE_PUBLIC_ORIGIN`). A scheme and a
    /// host, with no path, query or trailing slash. Unset or blank: each
    /// request's origin is derived from its `X-Forwarded-Proto` and `Host`
    /// headers, so a preview deployment reports its own host. `Cpe::new`
    /// logs once that it is derived.
    pub public_origin: Option<String>,

    /// Why the files and variables could not be read, when [`CpeConfig::load`]
    /// fell back to the defaults. Never read from a file or a variable.
    #[serde(skip)]
    pub(crate) load_error: Option<String>,
}

impl Default for CpeConfig {
    fn default() -> Self {
        Self {
            projects_dir: PathBuf::from("areas/access/cpe/projects"),
            public_origin: None,
            load_error: None,
        }
    }
}

impl CpeConfig {
    /// Reads `cpe.toml` and `CPE_*`. Infallible: a value figment cannot read
    /// gives the defaults with [`CpeConfig::load_error`] set.
    pub fn load() -> Self {
        let figment = Figment::new()
            .merge(Serialized::defaults(CpeConfig::default()))
            .merge(Toml::file("cpe.toml"))
            .merge(Env::prefixed("CPE_").only(&["projects_dir", "public_origin"]));
        match figment.extract::<Self>() {
            Ok(mut config) => {
                config.public_origin = config.public_origin.filter(|origin| !origin.trim().is_empty());
                config
            }
            Err(error) => Self {
                load_error: Some(format!("CPE's configuration (cpe.toml, CPE_*) cannot be read: {error}")),
                ..Self::default()
            },
        }
    }

    /// Whether no origin is configured, so that each request's own headers
    /// give the origin of its absolute URLs.
    pub(crate) fn derives_origin(&self) -> bool {
        self.public_origin.as_deref().is_none_or(|origin| origin.trim().is_empty())
    }

    /// Why CPE cannot be served with this configuration, in a sentence that
    /// names the variable; `None` when it can.
    pub(crate) fn refusal(&self) -> Option<String> {
        if let Some(error) = &self.load_error {
            return Some(error.clone());
        }
        let origin = self.public_origin.as_deref().filter(|origin| !origin.trim().is_empty())?;
        validate_origin("CPE_PUBLIC_ORIGIN", origin).err()
    }
}

/// figment reads an environment value that looks like a number or a boolean as one, and a folder
/// may be named `2024`.
fn path_from_scalar<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Scalar {
        Text(PathBuf),
        Unsigned(u64),
        Signed(i64),
        Bool(bool),
    }
    Ok(match Scalar::deserialize(deserializer)? {
        Scalar::Text(path) => path,
        Scalar::Unsigned(number) => PathBuf::from(number.to_string()),
        Scalar::Signed(number) => PathBuf::from(number.to_string()),
        Scalar::Bool(flag) => PathBuf::from(flag.to_string()),
    })
}

/// `value` must be an origin: `http://` or `https://`, a host, and nothing
/// after it. A path would be repeated in every absolute link built from it.
fn validate_origin(variable: &str, value: &str) -> Result<(), String> {
    let host = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| format!("{variable} must start with http:// or https:// (got {value:?})"))?;
    if host.is_empty() || host.starts_with(':') {
        return Err(format!("{variable} has no host (got {value:?})"));
    }
    if let Some(bad) = host
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']')))
    {
        return Err(format!(
            "{variable} must be an origin with no path, query or trailing slash (got {value:?}, found {bad:?})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_origin(origin: &str) -> CpeConfig {
        CpeConfig {
            public_origin: Some(origin.to_owned()),
            ..CpeConfig::default()
        }
    }

    #[test]
    fn the_defaults_fit_a_run_from_the_repository_root() {
        let config = CpeConfig::default();
        assert_eq!(config.projects_dir, PathBuf::from("areas/access/cpe/projects"));
        assert!(config.public_origin.is_none());
        assert!(config.refusal().is_none());
    }

    #[test]
    fn an_origin_is_accepted() {
        for good in [
            "https://repository.dasch.swiss",
            "http://localhost:4000",
            "https://cpe-pr-1-x.a.run.app",
        ] {
            assert!(with_origin(good).refusal().is_none(), "{good} should be accepted");
        }
    }

    #[test]
    fn a_public_origin_with_anything_past_the_host_is_refused() {
        for bad in [
            "https://repository.dasch.swiss/",
            "https://repository.dasch.swiss/cpe",
            "https://repository.dasch.swiss?x=1",
            "https://repository.dasch.swiss#f",
            "https://repository.dasch.swiss evil.example",
            "https://user@evil.example",
            "https://a\\b",
            "https://a\u{0}",
            "https://:8080",
            "ftp://repository.dasch.swiss",
            "repository.dasch.swiss",
            "//evil.example",
            "https://",
        ] {
            let reason = with_origin(bad).refusal().unwrap_or_else(|| panic!("{bad} should be refused"));
            assert!(reason.contains("CPE_PUBLIC_ORIGIN"), "the reason names the variable: {reason}");
        }
    }

    #[test]
    fn a_blank_public_origin_is_unset() {
        assert!(with_origin("  ").refusal().is_none());
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_PUBLIC_ORIGIN", "");
            let config = CpeConfig::load();
            assert!(config.public_origin.is_none());
            assert!(config.refusal().is_none());
            Ok(())
        });
    }

    #[test]
    fn the_variables_are_read() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_PROJECTS_DIR", "/app/cpe/projects");
            jail.set_env("CPE_PUBLIC_ORIGIN", "https://repository.dasch.swiss");
            let config = CpeConfig::load();
            assert_eq!(config.projects_dir, PathBuf::from("/app/cpe/projects"));
            assert_eq!(config.public_origin.as_deref(), Some("https://repository.dasch.swiss"));
            assert!(config.refusal().is_none());
            Ok(())
        });
    }

    /// figment reads `2024` as a number; a folder may be named so.
    #[test]
    fn a_numeric_looking_projects_dir_is_a_path() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_PROJECTS_DIR", "2024");
            let config = CpeConfig::load();
            assert!(config.refusal().is_none(), "{:?}", config.refusal());
            assert_eq!(config.projects_dir, PathBuf::from("2024"));
            Ok(())
        });
    }

    #[test]
    fn an_unrelated_variable_with_the_prefix_is_not_read() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_LOAD_ERROR", "x");
            jail.set_env("CPE_SOMETHING_ELSE", "[1, 2]");
            assert!(CpeConfig::load().refusal().is_none());
            Ok(())
        });
    }

    #[test]
    fn a_public_origin_with_a_path_loads_and_is_refused() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_PUBLIC_ORIGIN", "https://repository.dasch.swiss/cpe");
            let config = CpeConfig::load();
            let reason = config.refusal().expect("an origin with a path is refused");
            assert!(reason.contains("CPE_PUBLIC_ORIGIN"), "{reason}");
            Ok(())
        });
    }

    #[test]
    fn a_file_that_cannot_be_read_gives_the_defaults_and_a_reason() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("cpe.toml", "projects_dir = [1, 2]\n")?;
            let config = CpeConfig::load();
            assert_eq!(config.projects_dir, CpeConfig::default().projects_dir);
            let reason = config.refusal().expect("a wrong type is refused");
            assert!(reason.contains("projects_dir"), "{reason}");
            Ok(())
        });
    }
}
