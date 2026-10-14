//! `identity-web`: identity's pages, handlers and routes behind the [`Identity`] facade
//! (`areas/deposit/ADR-0004`). The root names no `identity-core` type: it passes plain
//! [`IdentitySettings`] in and merges the router out.

#[cfg(test)]
mod fixtures;
mod handlers;
mod pages;
mod routes;
mod shell;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod traceparent;

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use identity_core::login::AuthConfig;
/// What the root's configuration check reads: the login code's lifetime and the address shape.
pub use identity_core::login::{is_plausible_address, CODE_TTL};
use identity_core::repository::IdentityRepositories;
use identity_ports::AuthoredWork;

pub use crate::routes::route_paths;
#[cfg(any(test, feature = "test-support"))]
pub use crate::routes::ISSUE_BURST;
use crate::shell::IdentityState;

/// No `Debug`: it holds the RDU members' addresses, and no address may reach a log.
#[derive(Clone)]
pub struct IdentitySettings {
    pub css_href: String,
    pub login_cooldown: Duration,
    pub login_max_failed: u32,
    pub login_lockout: Duration,
    pub mail_daily_cap: u64,
    pub mail_account_daily_cap: u64,
    pub session_absolute: Duration,
    pub session_idle: Duration,
    pub smtp_break_glass: bool,
    /// `None` selects the console transport.
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_credentials: Option<(String, SmtpPassword)>,
    pub smtp_from: String,
    pub rdu_addresses: Vec<String>,
    pub reveal_login_code: bool,
}

impl IdentitySettings {
    #[must_use]
    pub fn auth(&self) -> AuthConfig {
        AuthConfig {
            cooldown: self.login_cooldown,
            max_failed: self.login_max_failed,
            lockout: self.login_lockout,
            daily_cap: self.mail_daily_cap,
            account_daily_cap: self.mail_account_daily_cap,
            session_absolute: self.session_absolute,
            session_idle: self.session_idle,
            break_glass: self.smtp_break_glass,
        }
    }
}

/// The relay password, with a redacted `Debug`.
#[derive(Clone)]
pub struct SmtpPassword(String);

impl SmtpPassword {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Debug for SmtpPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SmtpPassword(redacted)")
    }
}

/// Each is fatal: with no administrator nobody can administer the service, and a misconfigured
/// relay would otherwise fail at the first login.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("failed to create the configured RDU accounts: {0}")]
    Rdu(identity_core::repository::RepositoryError),
    #[error("failed to configure the SMTP relay: {0}")]
    Mail(identity_store::mail::SmtpSetupError),
}

pub struct Identity {
    state: IdentityState,
}

impl Identity {
    /// Create or promote the configured RDU accounts, build the mailer and spawn the hourly
    /// sweep, in that order.
    pub async fn start(
        settings: IdentitySettings,
        store: Arc<dyn IdentityRepositories>,
        authored_work: Arc<dyn AuthoredWork>,
    ) -> Result<Self, StartupError> {
        let changed = handlers::accounts::ensure_rdu(&*store, &settings.rdu_addresses, chrono::Utc::now())
            .await
            .map_err(StartupError::Rdu)?;
        tracing::info!(
            rdu.configured = settings.rdu_addresses.len(),
            rdu.changed = changed,
            "RDU accounts reconciled with configuration"
        );

        let mailer = identity_store::mail::mailer(
            settings.smtp_host.as_deref(),
            settings.smtp_port,
            settings
                .smtp_credentials
                .as_ref()
                .map(|(username, password)| (username.as_str(), password.0.as_str())),
            &settings.smtp_from,
        )
        .map_err(StartupError::Mail)?;
        tracing::info!(mail.transport = %mailer.describe(), "mail transport ready");

        spawn_sweep(Arc::clone(&store));

        Ok(Self {
            state: IdentityState {
                auth: settings.auth(),
                css_href: settings.css_href,
                reveal_login_code: settings.reveal_login_code,
                store,
                authored_work,
                mailer,
            },
        })
    }

    /// Untraced and without CSRF: the root wraps them in its layers.
    pub fn router(&self) -> axum::Router {
        routes::routes()
            .into_iter()
            .fold(axum::Router::new(), |router, (path, handlers)| router.route(path, handlers))
            .with_state(self.state.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Identity {
    /// None of [`Identity::start`]'s startup work, and the login code is never revealed.
    #[must_use]
    pub fn for_test(
        store: Arc<dyn IdentityRepositories>,
        authored_work: Arc<dyn AuthoredWork>,
        mailer: Arc<dyn identity_core::mail::Mailer>,
        auth: AuthConfig,
    ) -> Self {
        Self {
            state: IdentityState {
                css_href: "/assets/app.css".to_string(),
                store,
                authored_work,
                mailer,
                auth,
                reveal_login_code: false,
            },
        }
    }
}

/// The only thing that bounds unused login codes, sessions and the append-only send log.
/// Detached rather than drained on shutdown: the next start sweeps what is left.
fn spawn_sweep(store: Arc<dyn IdentityRepositories>) {
    use identity_core::repository::{LoginCodeRepository, MailSendRepository, SessionRepository};

    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
        // The first tick completes immediately; the sweep is for what has
        // aged out, so start one interval in.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = chrono::Utc::now();
            // Pruned at the caps' own window: a shorter one would hand back spent budget.
            let window_start = now - identity_core::login::delta(identity_core::login::SEND_WINDOW);
            match (
                LoginCodeRepository::delete_expired(&*store, now).await,
                SessionRepository::delete_expired(&*store, now).await,
                MailSendRepository::delete_before(&*store, window_start).await,
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
