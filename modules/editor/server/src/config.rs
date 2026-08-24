//! Application configuration.
//!
//! Layered configuration following Twelve-Factor App Factor III, mirroring
//! `DpeConfig`:
//! 1. Defaults (in code)
//! 2. Config file (optional `editor.toml`)
//! 3. Environment variables (`EDITOR_*` prefix, override all)

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

/// How long a login code lives.
///
/// A constant, not a setting. NIST SP 800-63B-4 §3.1.3.2 ("the authentication
/// SHALL be considered invalid unless completed within 10 minutes") and OWASP
/// ASVS 6.5.5 both cap an out-of-band code at ten minutes, so the only thing a
/// knob here could express is a violation of both.
pub const CODE_TTL: Duration = Duration::from_secs(600);

/// A configured value that must never reach a log.
///
/// The `Debug` implementation is the point: `EditorConfig` derives `Debug`, and
/// without this a single `{:?}` anywhere — a panic message, a startup dump, a
/// future `tracing::debug!` — would print the relay password.
///
/// `Serialize` is required because figment seeds its defaults by serializing
/// `EditorConfig::default()`, where this is `None`. Nothing else serializes the
/// config, and nothing should.
#[derive(Clone, Serialize)]
pub struct Secret(String);

impl<'de> Deserialize<'de> for Secret {
    /// Hand-written, and load-bearing.
    ///
    /// figment magic-parses environment values, so `EDITOR_SMTP_PASSWORD=1234567890`
    /// arrives as a number. A derived `Deserialize` on this newtype would reject
    /// it — and figment's type-mismatch error prints the value it found, straight
    /// to stderr through `main`'s config-load report:
    ///
    /// ```text
    /// invalid type: found unsigned int `1234567890`, expected a string
    /// for key "SMTP_PASSWORD" in `EDITOR_` environment variable(s)
    /// ```
    ///
    /// [`Secret`]'s redacting [`fmt::Debug`] cannot help with that: the value
    /// leaks while it is being turned *into* a `Secret`, upstream of the type.
    /// Accepting every scalar shape and stringifying it removes the failure, and
    /// with it the message.
    ///
    /// One edge worth knowing: a password figment reads as a *number* is
    /// stringified back from the parsed value, so a leading zero (`0755`) or a
    /// trailing one (`1.10`) is not preserved. It fails closed — the relay
    /// rejects the credential — but choose a password that is not purely
    /// numeric. A Google Workspace app password is sixteen lowercase letters, so
    /// the intended one cannot hit this.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AnyScalar;

        impl serde::de::Visitor<'_> for AnyScalar {
            type Value = Secret;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // Deliberately says nothing about what was found.
                f.write_str("a secret value")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Secret, E> {
                Ok(Secret(value.to_string()))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Secret, E> {
                Ok(Secret(value))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Secret, E> {
                Ok(Secret(value.to_string()))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Secret, E> {
                Ok(Secret(value.to_string()))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Secret, E> {
                Ok(Secret(value.to_string()))
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Secret, E> {
                Ok(Secret(value.to_string()))
            }
        }

        deserializer.deserialize_any(AnyScalar)
    }
}

impl Secret {
    /// The plaintext. Named so that every use is greppable.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(redacted)")
    }
}

/// Why the configuration could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A value of the wrong type, or a malformed `editor.toml`.
    #[error(transparent)]
    Load(#[from] Box<figment::Error>),

    /// Every value parsed, but the combination is not usable. Separate from
    /// [`Self::Load`] because figment cannot express a rule that spans two keys,
    /// and those are the rules that silently lock people out.
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

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

    /// Directory holding the SQLite database, set via `EDITOR_DB_DIR`.
    ///
    /// **No default, and unset means in-memory** — not a path. That is the
    /// preview-safety default: the Cloud Run PR preview has no mounted volume
    /// and is publicly reachable, so a publicly reachable preview must not be
    /// able to accumulate state. It also means tests and `just dev-editor` need
    /// no volume.
    ///
    /// Production sets it to the mount Infra provides. It names the
    /// **directory**, never the database file: SQLite creates `-wal` and `-shm`
    /// siblings next to the file, which is impossible if the file itself is the
    /// mount point. Startup writes and removes a probe file in it, so a
    /// wrong-uid or root-owned mount fails with a message that says so.
    pub db_dir: Option<PathBuf>,

    /// Size of the reader connection pool, set via `EDITOR_DB_READERS`.
    ///
    /// The writer pool is always one connection: SQLite permits a single writer
    /// at a time, so a second would move the queue from the pool into SQLite.
    pub db_readers: usize,

    /// SQLite `busy_timeout` in milliseconds, set via `EDITOR_DB_BUSY_TIMEOUT_MS`.
    ///
    /// Applied per connection, because that is the only place it has effect. It
    /// is a backstop rather than the primary defence: writes serialise in the
    /// pool and open `BEGIN IMMEDIATE`, so a reader waiting on a checkpoint is
    /// the case it actually covers.
    pub db_busy_timeout_ms: u64,

    /// SMTP relay host, set via `EDITOR_SMTP_HOST`.
    ///
    /// **No default, and unset means the console transport** (REQ-6.8): codes
    /// are written to the log and the service stays usable. That is the
    /// development and PR-preview default — the Cloud Run preview is publicly
    /// reachable, so it must not be able to send mail through the shared relay
    /// or spend its daily quota.
    pub smtp_host: Option<String>,

    /// SMTP port, set via `EDITOR_SMTP_PORT`. 587 is submission with STARTTLS,
    /// which is what `smtp-relay.gmail.com` speaks.
    pub smtp_port: u16,

    /// SMTP username, set via `EDITOR_SMTP_USERNAME`.
    pub smtp_username: Option<String>,

    /// SMTP password, set via `EDITOR_SMTP_PASSWORD`.
    pub smtp_password: Option<Secret>,

    /// Envelope sender, set via `EDITOR_SMTP_FROM`.
    ///
    /// `noreply@dasch.swiss` rather than a university address: a login code
    /// arriving from `dasch.unibas.ch` reads as a phishing attempt to a
    /// depositor, and Google DKIM-signs relayed mail with the envelope sender's
    /// domain only if that domain has DKIM enabled in the Workspace Admin
    /// Console.
    pub smtp_from: String,

    /// Whether a **configured but failing** relay falls back to writing the code
    /// to the log, set via `EDITOR_SMTP_BREAK_GLASS`.
    ///
    /// Off by default, deliberately. On, a transient relay error — a rate limit,
    /// a TLS blip — writes a live login code into the log pipeline, where it is
    /// retained for weeks and readable by everyone with log access. Off, a failed
    /// send rolls the code back and reports a generic failure.
    ///
    /// It exists because a relay that is broken for hours otherwise locks out
    /// every user including RDU. Turning it on is an incident response, and the
    /// failure log line names it so the remedy is found from the error itself.
    pub smtp_break_glass: bool,

    /// Seconds before another code may be sent to the same address (REQ-6.5),
    /// set via `EDITOR_LOGIN_COOLDOWN_SECS`.
    ///
    /// Validated to be shorter than [`CODE_TTL`]: a cooldown longer than the
    /// code's life means three wrong entries leave a legitimate user with a dead
    /// code and no way to ask for another, for the difference between the two.
    pub login_cooldown_secs: u64,

    /// Consecutive failed authentications an account tolerates before it is
    /// throttled, set via `EDITOR_LOGIN_MAX_FAILED`.
    ///
    /// Ten for this population, far below NIST SP 800-63B-4's ceiling of 100.
    /// The counter is per **account** and survives code invalidation and
    /// resend — a per-code counter hands out a fresh budget on every resend,
    /// which at a 60-second cooldown is roughly 4,320 guesses a day against one
    /// address.
    pub login_max_failed: u32,

    /// How long the throttle lasts once the account hits the cap, set via
    /// `EDITOR_LOGIN_LOCKOUT_SECS`.
    ///
    /// Time-based rather than permanent because the counter clears only on a
    /// successful authentication, which a locked-out account cannot perform. A
    /// permanent lock would need an unlock control that does not exist.
    pub login_lockout_secs: u64,

    /// The most codes that may be sent across **all** users in 24 hours, set via
    /// `EDITOR_MAIL_DAILY_CAP`.
    ///
    /// The Google Workspace relay allows 10,000 recipients a day and the quota
    /// is shared with other senders, so this sits well below it. Without the cap,
    /// an attacker looping resend across the known addresses exhausts the quota
    /// and locks out every user, RDU included — a login denial of service that
    /// costs nothing to mount.
    pub mail_daily_cap: u64,

    /// Absolute session lifetime in seconds, set via
    /// `EDITOR_SESSION_ABSOLUTE_SECS`. Set at creation and never extended.
    pub session_absolute_secs: u64,

    /// Idle session timeout in seconds, set via `EDITOR_SESSION_IDLE_SECS`.
    pub session_idle_secs: u64,

    /// Comma-separated addresses that always have an RDU account (REQ-7.2), set
    /// via `EDITOR_RDU_EMAILS`.
    ///
    /// A single string rather than a `Vec<String>` because that is what an
    /// environment variable is; splitting it here keeps the parsing in one
    /// tested place instead of relying on how a config library happens to coerce
    /// a list out of an env value.
    ///
    /// Unset is legitimate — a fresh checkout and the PR preview both run with
    /// no accounts at all.
    pub rdu_emails: Option<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            site_addr: "127.0.0.1:4100".to_string(),
            public_dir: PathBuf::from("modules/editor/public"),
            data_dir: None,
            env: "DEV".to_string(),
            db_dir: None,
            db_readers: 4,
            db_busy_timeout_ms: 5_000,
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "noreply@dasch.swiss".to_string(),
            smtp_break_glass: false,
            login_cooldown_secs: 60,
            login_max_failed: 10,
            login_lockout_secs: 900,
            mail_daily_cap: 500,
            // Twelve hours: long enough that a working day needs one sign-in,
            // short enough that a session left open overnight is gone by morning.
            session_absolute_secs: 12 * 60 * 60,
            // Two hours idle. The editor is used in long sittings on one form,
            // so a shorter timeout would sign people out mid-edit.
            session_idle_secs: 2 * 60 * 60,
            rdu_emails: None,
        }
    }
}

impl EditorConfig {
    /// Load configuration from defaults → `editor.toml` → `EDITOR_*` env vars,
    /// and validate the rules that span more than one key.
    ///
    /// Validation is inside `load` rather than a `validate()` a caller has to
    /// remember: a separate call is one refactor away from being dropped, and
    /// what it guards — a cooldown that outlives the code it gates — locks
    /// people out silently rather than failing loudly.
    pub fn load() -> Result<Self, ConfigError> {
        let config: Self = Figment::new()
            .merge(Serialized::defaults(EditorConfig::default()))
            .merge(Toml::file("editor.toml"))
            .merge(Env::prefixed("EDITOR_"))
            .extract()
            .map_err(Box::new)?;
        config.validate()?;
        Ok(config)
    }

    /// The rules figment cannot express, each one a way to lock users out
    /// without any single value looking wrong.
    fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |message: String| Err(ConfigError::Invalid(message));

        if self.login_cooldown_secs == 0 {
            return invalid(
                "EDITOR_LOGIN_COOLDOWN_SECS must be at least 1 — a zero cooldown lets a loop send one mail per \
                 request and exhaust the relay quota"
                    .to_string(),
            );
        }
        if self.login_cooldown_secs >= CODE_TTL.as_secs() {
            return invalid(format!(
                "EDITOR_LOGIN_COOLDOWN_SECS ({}) must be shorter than the {}-second code lifetime: three wrong \
                 entries invalidate a code, and a cooldown that outlives it leaves a legitimate user with no way to \
                 ask for another for the difference",
                self.login_cooldown_secs,
                CODE_TTL.as_secs()
            ));
        }
        if self.login_max_failed == 0 {
            return invalid(
                "EDITOR_LOGIN_MAX_FAILED must be at least 1 — zero locks out every account on its first attempt"
                    .to_string(),
            );
        }
        if self.login_max_failed > 100 {
            return invalid(format!(
                "EDITOR_LOGIN_MAX_FAILED ({}) is above NIST SP 800-63B-4's ceiling of 100 consecutive failed \
                 attempts per account",
                self.login_max_failed
            ));
        }
        if self.login_lockout_secs == 0 {
            return invalid(
                "EDITOR_LOGIN_LOCKOUT_SECS must be at least 1 — a zero lockout makes the failure counter decorative"
                    .to_string(),
            );
        }
        if self.mail_daily_cap == 0 {
            return invalid(
                "EDITOR_MAIL_DAILY_CAP must be at least 1 — zero refuses every code and no one can sign in".to_string(),
            );
        }
        if self.session_absolute_secs == 0 || self.session_idle_secs == 0 {
            return invalid(
                "EDITOR_SESSION_ABSOLUTE_SECS and EDITOR_SESSION_IDLE_SECS must both be at least 1 — zero expires \
                 every session at the moment it is created"
                    .to_string(),
            );
        }
        // Upper bounds, because the deadline arithmetic is not total.
        // `chrono`'s `DateTime + TimeDelta` panics on overflow, and a duration
        // large enough to reach that is accepted by every other rule here — so
        // an absurd value validates, boots, and panics on the first sign-in.
        // A year is far beyond any sane session and nowhere near the edge.
        const MAX_DURATION_SECS: u64 = 365 * 24 * 60 * 60;
        for (name, value) in [
            ("EDITOR_SESSION_ABSOLUTE_SECS", self.session_absolute_secs),
            ("EDITOR_SESSION_IDLE_SECS", self.session_idle_secs),
            ("EDITOR_LOGIN_LOCKOUT_SECS", self.login_lockout_secs),
        ] {
            if value > MAX_DURATION_SECS {
                return invalid(format!(
                    "{name} ({value}) is longer than a year, which is past anything the deadline arithmetic can \
                     represent safely"
                ));
            }
        }
        if self.session_idle_secs > self.session_absolute_secs {
            return invalid(format!(
                "EDITOR_SESSION_IDLE_SECS ({}) is longer than EDITOR_SESSION_ABSOLUTE_SECS ({}), so the idle timeout \
                 can never be reached and is dead configuration",
                self.session_idle_secs, self.session_absolute_secs
            ));
        }
        // Half a credential authenticates as nobody, and the relay's rejection
        // is indistinguishable from a wrong password.
        if self.smtp_username.is_some() != self.smtp_password.is_some() {
            return invalid(
                "EDITOR_SMTP_USERNAME and EDITOR_SMTP_PASSWORD must be set together — one without the other \
                 authenticates as nobody"
                    .to_string(),
            );
        }
        // The dangerous combination, and the reason it needs a guard rather than a
        // warning: with no relay every code is written to the log, and the
        // service otherwise behaves perfectly normally. Nothing fails, nobody
        // notices, and every login code for the life of the deployment is
        // retained in the log pipeline for anyone with log access to read.
        //
        // Development, `just dev-editor`, `just run-docker-editor` and the Cloud
        // Run PR preview all run as `DEV`, where the console transport is the
        // point (REQ-6.8). Only `PROD` is refused.
        if self.env == "PROD" && self.smtp_host.is_none() {
            return invalid(
                "EDITOR_ENV=PROD requires EDITOR_SMTP_HOST. Without a relay every login code is written to the log \
                 instead of being sent (REQ-6.8), which is correct for development and a standing credential leak in \
                 production. Set the relay, or set EDITOR_ENV=DEV if this is not a production deployment"
                    .to_string(),
            );
        }
        if self.smtp_break_glass && self.smtp_host.is_none() {
            return invalid(
                "EDITOR_SMTP_BREAK_GLASS has no effect without EDITOR_SMTP_HOST — with no relay configured, every \
                 code already goes to the log (REQ-6.8)"
                    .to_string(),
            );
        }
        // The position, never the value. This message reaches stderr through
        // `main`'s config-load report, which is container stderr and therefore
        // the log pipeline — and an address in a log is exactly what REQ-6.10
        // forbids. The operator holds the configuration, so an index is just as
        // actionable.
        for (index, address) in self.rdu_addresses().iter().enumerate() {
            if !crate::auth::is_plausible_address(address) {
                return invalid(format!(
                    "EDITOR_RDU_EMAILS entry {} cannot be an email address. Every entry becomes an account that \
                     administers the service, so a typo here is an administrator who can never sign in (the value is \
                     not repeated here: it would put an address in the log)",
                    index + 1
                ));
            }
        }
        Ok(())
    }

    /// The configured RDU addresses, split and trimmed.
    pub fn rdu_addresses(&self) -> Vec<String> {
        self.rdu_emails
            .iter()
            .flat_map(|list| list.split(','))
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// The SMTP credentials, present only when both halves are.
    pub fn smtp_credentials(&self) -> Option<(&str, &str)> {
        match (&self.smtp_username, &self.smtp_password) {
            (Some(username), Some(password)) => Some((username.as_str(), password.expose())),
            _ => None,
        }
    }

    /// Whether logs should additionally be exported over OTLP. True only in
    /// `DEV`; production logs to stdout and is scraped from there.
    pub fn exports_otlp_logs(&self) -> bool {
        self.env == "DEV"
    }

    /// Where the database lives: the configured directory, or a named in-memory
    /// database when `EDITOR_DB_DIR` is unset.
    ///
    /// The in-memory name is fixed rather than random, because a shared-cache
    /// in-memory database is scoped to the process and there is one per process.
    /// It is never bare `:memory:` — see `db::Source::Memory`.
    pub fn db_source(&self) -> crate::db::Source {
        match &self.db_dir {
            Some(dir) => crate::db::Source::Directory(dir.clone()),
            None => crate::db::Source::Memory("editor".to_string()),
        }
    }

    /// The configured `busy_timeout`.
    pub fn db_busy_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.db_busy_timeout_ms)
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
        assert_eq!(config.db_dir, None);
        assert_eq!(config.db_readers, 4);
        assert_eq!(config.db_busy_timeout_ms, 5_000);
    }

    #[test]
    fn database_defaults_to_in_memory_so_a_preview_cannot_accumulate_state() {
        // The Cloud Run PR preview has no mounted volume and is publicly
        // reachable. If the default were a path, the preview would persist login
        // codes, sessions and drafts in its ephemeral filesystem for as long as
        // the revision lived.
        figment::Jail::expect_with(|_| {
            let config = EditorConfig::load().expect("config should load");
            assert!(matches!(config.db_source(), crate::db::Source::Memory(_)));
            Ok(())
        });
    }

    #[test]
    fn db_dir_env_override_selects_a_file_database() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_DB_DIR", "/data/editor");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.db_dir, Some(PathBuf::from("/data/editor")));
            assert_eq!(config.db_source(), crate::db::Source::Directory(PathBuf::from("/data/editor")));
            Ok(())
        });
    }

    #[test]
    fn db_pool_and_timeout_env_overrides() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_DB_READERS", "8");
            jail.set_env("EDITOR_DB_BUSY_TIMEOUT_MS", "2500");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.db_readers, 8);
            assert_eq!(config.db_busy_timeout(), std::time::Duration::from_millis(2500));
            Ok(())
        });
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
            // A relay, because PROD without one is refused — see
            // `production_without_a_relay_is_refused`.
            jail.set_env("EDITOR_SMTP_HOST", "smtp-relay.gmail.com");
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
            jail.create_file(
                "editor.toml",
                "site_addr = \"127.0.0.1:4200\"\nenv = \"PROD\"\nsmtp_host = \"smtp-relay.gmail.com\"\n",
            )?;
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
    fn rdu_addresses_split_trim_and_drop_blanks() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_RDU_EMAILS", " a@dasch.swiss , b@dasch.swiss ,, ");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(
                config.rdu_addresses(),
                vec!["a@dasch.swiss".to_string(), "b@dasch.swiss".to_string()]
            );
            Ok(())
        });
    }

    #[test]
    fn no_rdu_addresses_is_a_legitimate_state() {
        // A fresh checkout and the PR preview both run with no accounts.
        figment::Jail::expect_with(|_| {
            assert!(EditorConfig::load().expect("config should load").rdu_addresses().is_empty());
            Ok(())
        });
    }

    #[test]
    fn a_malformed_rdu_address_stops_startup() {
        // Every entry becomes an account that administers the service, so a typo
        // is an administrator who can never sign in — and the failure would
        // otherwise appear as "my code never arrives".
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_RDU_EMAILS", "a@dasch.swiss,not-an-address");
            let error = EditorConfig::load().expect_err("a malformed address must be refused");
            let rendered = error.to_string();
            assert!(rendered.contains("EDITOR_RDU_EMAILS"), "{rendered}");
            assert!(rendered.contains("entry 2"), "the position must be named: {rendered}");
            assert!(
                !rendered.contains("not-an-address") && !rendered.contains("dasch.swiss"),
                "no configured address may appear in a message that reaches stderr: {rendered}"
            );
            Ok(())
        });
    }

    #[test]
    fn a_cooldown_at_or_over_the_code_lifetime_is_refused() {
        // Three wrong entries invalidate a code; a cooldown that outlives it
        // leaves a legitimate user with no way to ask for another.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_LOGIN_COOLDOWN_SECS", CODE_TTL.as_secs().to_string());
            let error = EditorConfig::load().expect_err("the cooldown must be shorter than the code lifetime");
            assert!(error.to_string().contains("shorter than"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn half_a_relay_credential_is_refused() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SMTP_HOST", "smtp.example.test");
            jail.set_env("EDITOR_SMTP_USERNAME", "user");
            let error = EditorConfig::load().expect_err("a username without a password must be refused");
            assert!(error.to_string().contains("must be set together"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn an_absurd_duration_is_refused_before_it_can_overflow_a_deadline() {
        // `chrono`'s `DateTime + TimeDelta` panics on overflow, so a value this
        // large would validate, boot, and panic on the first sign-in.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SESSION_ABSOLUTE_SECS", "9000000000000");
            let error = EditorConfig::load().expect_err("an absurd session lifetime must be refused");
            assert!(error.to_string().contains("longer than a year"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn an_idle_timeout_longer_than_the_absolute_expiry_is_refused() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SESSION_ABSOLUTE_SECS", "3600");
            jail.set_env("EDITOR_SESSION_IDLE_SECS", "7200");
            let error = EditorConfig::load().expect_err("dead configuration must be refused");
            assert!(error.to_string().contains("dead configuration"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn production_without_a_relay_is_refused() {
        // The failure this prevents is silent: with no relay the service works
        // normally and writes every login code to the log.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_ENV", "PROD");
            let error = EditorConfig::load().expect_err("production without a relay must be refused");
            assert!(error.to_string().contains("EDITOR_SMTP_HOST"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn production_with_a_relay_loads() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_ENV", "PROD");
            jail.set_env("EDITOR_SMTP_HOST", "smtp-relay.gmail.com");
            let config = EditorConfig::load().expect("production with a relay should load");
            assert!(!config.exports_otlp_logs());
            Ok(())
        });
    }

    #[test]
    fn development_without_a_relay_is_the_normal_case() {
        // `just dev-editor`, the test suite and the PR preview all run this way.
        figment::Jail::expect_with(|_| {
            let config = EditorConfig::load().expect("development without a relay must load");
            assert_eq!(config.env, "DEV");
            assert_eq!(config.smtp_host, None);
            Ok(())
        });
    }

    #[test]
    fn break_glass_without_a_relay_is_refused_as_a_misunderstanding() {
        // With no relay every code already goes to the log (REQ-6.8), so the
        // variable would look like it was doing something and not be.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SMTP_BREAK_GLASS", "true");
            let error = EditorConfig::load().expect_err("break-glass without a relay must be refused");
            assert!(error.to_string().contains("no effect"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn the_relay_password_never_appears_in_a_debug_rendering() {
        // `EditorConfig` derives `Debug`, so one `{:?}` anywhere — a panic
        // message, a future debug log — would otherwise print the credential.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SMTP_HOST", "smtp.example.test");
            jail.set_env("EDITOR_SMTP_USERNAME", "user");
            jail.set_env("EDITOR_SMTP_PASSWORD", "hunter2");
            let config = EditorConfig::load().expect("config should load");
            assert_eq!(config.smtp_credentials(), Some(("user", "hunter2")));
            let rendered = format!("{config:?}");
            assert!(!rendered.contains("hunter2"), "{rendered}");
            assert!(rendered.contains("redacted"), "{rendered}");
            Ok(())
        });
    }

    #[test]
    fn a_relay_password_never_reaches_the_error_message_whatever_shape_it_takes() {
        // figment magic-parses environment values, so a password that looks like
        // a number, a float or a boolean does not arrive as a string. A derived
        // `Deserialize` rejected those, and figment's type-mismatch error prints
        // the value it found — to stderr, through `main`'s config-load report.
        // The redacting `Debug` cannot help: the leak is upstream of the type.
        for password in ["1234567890", "9876.54", "true", "false", "-42", "0755"] {
            figment::Jail::expect_with(|jail| {
                jail.set_env("EDITOR_SMTP_HOST", "smtp.example.test");
                jail.set_env("EDITOR_SMTP_USERNAME", "user");
                jail.set_env("EDITOR_SMTP_PASSWORD", password);
                // The two places it could surface: the error figment builds when
                // the value does not fit the type, and any rendering of the
                // stored secret. Not the whole config dump — that contains
                // unrelated `false`s and would only test the test.
                let rendered = match EditorConfig::load() {
                    Ok(config) => format!("{:?}", config.smtp_password),
                    Err(e) => format!("{e}"),
                };
                assert!(
                    !rendered.contains(password),
                    "the password {password:?} reached a rendered string: {rendered}"
                );
                Ok(())
            });
        }
    }

    #[test]
    fn a_numeric_relay_password_is_still_usable() {
        // Not only silent — correct. Rejecting it would have been a startup
        // failure for a legitimate password.
        figment::Jail::expect_with(|jail| {
            jail.set_env("EDITOR_SMTP_HOST", "smtp.example.test");
            jail.set_env("EDITOR_SMTP_USERNAME", "user");
            jail.set_env("EDITOR_SMTP_PASSWORD", "1234567890");
            let config = EditorConfig::load().expect("a numeric password must load");
            assert_eq!(config.smtp_credentials(), Some(("user", "1234567890")));
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
