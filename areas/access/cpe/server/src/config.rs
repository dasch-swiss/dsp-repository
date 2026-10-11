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

    /// The path CPE is mounted at (`CPE_BASE`): empty for a mount at the root
    /// of the host, else a path that starts with `/` and does not end with
    /// one, e.g. `/cpe`. Every link, redirect and cookie CPE writes starts
    /// with it, and a host mounts CPE's router at [`crate::Cpe::base`], so the
    /// two cannot differ.
    #[serde(deserialize_with = "text_from_scalar")]
    pub base: String,

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
            base: String::new(),
            load_error: None,
        }
    }
}

impl CpeConfig {
    /// Reads `cpe.toml` and `CPE_*`. Infallible: a value figment cannot read
    /// gives the defaults and is refused by `Cpe`, whose router answers 503.
    pub fn load() -> Self {
        let figment = Figment::new()
            .merge(Serialized::defaults(CpeConfig::default()))
            .merge(Toml::file("cpe.toml"))
            .merge(Env::prefixed("CPE_").only(&["projects_dir", "public_origin", "base"]));
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
        if let Err(reason) = self.parsed_base() {
            return Some(format!("CPE_BASE {reason}"));
        }
        let origin = self.public_origin.as_deref().filter(|origin| !origin.trim().is_empty())?;
        validate_origin("CPE_PUBLIC_ORIGIN", origin).err()
    }

    /// The base CPE is mounted at. A value that is refused gives the root:
    /// CPE then answers 503 wherever its host mounts it.
    pub(crate) fn base_or_root(&self) -> cpe_core::url::Base {
        self.parsed_base().unwrap_or_default()
    }

    fn parsed_base(&self) -> Result<cpe_core::url::Base, cpe_core::url::BaseError> {
        cpe_core::url::Base::parse(&self.base)
    }
}

/// figment reads an environment value that looks like a number or a boolean as one, and a folder
/// may be named `2024`.
fn path_from_scalar<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    text_from_scalar(deserializer).map(PathBuf::from)
}

/// The value as the text it was written as. A base that figment read as a number is then refused
/// by the base's own rule, with a reason that names `CPE_BASE`.
fn text_from_scalar<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Scalar {
        Text(String),
        Unsigned(u64),
        Signed(i64),
        Bool(bool),
    }
    Ok(match Scalar::deserialize(deserializer)? {
        Scalar::Text(text) => text,
        Scalar::Unsigned(number) => number.to_string(),
        Scalar::Signed(number) => number.to_string(),
        Scalar::Bool(flag) => flag.to_string(),
    })
}

/// The longest host an origin may have, in bytes: the limit of a DNS name.
const MAX_ORIGIN_HOST: usize = 255;

/// Whether `c` may stand in the host of an origin, its port included. Nothing
/// else may: the origin is written into absolute links as it is.
fn is_origin_host_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']')
}

/// Whether `host` (with its port, if it has one) can be the host of an
/// origin. The configured origin and a request's `Host` header are held to
/// this one rule.
pub(crate) fn is_origin_host(host: &str) -> bool {
    !host.is_empty() && !host.starts_with(':') && host.len() <= MAX_ORIGIN_HOST && host.chars().all(is_origin_host_char)
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
    if let Some(bad) = host.chars().find(|c| !is_origin_host_char(*c)) {
        return Err(format!(
            "{variable} must be an origin with no path, query or trailing slash (got {value:?}, found {bad:?})"
        ));
    }
    if host.len() > MAX_ORIGIN_HOST {
        return Err(format!("{variable} has a host longer than {MAX_ORIGIN_HOST} bytes"));
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
        assert_eq!(config.base, "", "CPE is mounted at the root unless a base is configured");
        assert_eq!(config.base_or_root(), cpe_core::url::ROOT);
        assert!(config.refusal().is_none());
    }

    fn with_base(base: &str) -> CpeConfig {
        CpeConfig { base: base.to_owned(), ..CpeConfig::default() }
    }

    #[test]
    fn a_base_is_read_and_accepted() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_BASE", "/cpe");
            let config = CpeConfig::load();
            assert_eq!(config.base, "/cpe");
            assert!(config.refusal().is_none(), "{:?}", config.refusal());
            assert_eq!(config.base_or_root().as_str(), "/cpe");
            Ok(())
        });
        figment::Jail::expect_with(|jail| {
            jail.set_env("CPE_BASE", "");
            let config = CpeConfig::load();
            assert!(config.refusal().is_none(), "{:?}", config.refusal());
            assert_eq!(config.base_or_root(), cpe_core::url::ROOT);
            Ok(())
        });
    }

    #[test]
    fn a_base_without_a_leading_slash_or_with_a_trailing_one_is_refused() {
        for bad in [
            "cpe",
            "cpe/",
            "/cpe/",
            "/",
            "//evil.example",
            "/cpe/../x",
            "https://evil.example",
            "/c pe",
        ] {
            let config = with_base(bad);
            let reason = config.refusal().unwrap_or_else(|| panic!("{bad:?} should be refused"));
            assert!(reason.contains("CPE_BASE"), "the reason names the variable: {reason}");
            assert_eq!(
                config.base_or_root(),
                cpe_core::url::ROOT,
                "{bad:?}: a refused base is never written"
            );
        }
    }

    /// figment reads `2024` as a number. It loads, and the base's own rule refuses it.
    #[test]
    fn a_refused_base_loads_and_is_refused_with_the_variables_name() {
        for bad in ["cpe/", "2024", "true"] {
            figment::Jail::expect_with(|jail| {
                jail.set_env("CPE_BASE", bad);
                let config = CpeConfig::load();
                assert_eq!(config.base, bad);
                let reason = config.refusal().expect("the base is refused");
                assert!(reason.contains("CPE_BASE"), "{reason}");
                Ok(())
            });
        }
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
