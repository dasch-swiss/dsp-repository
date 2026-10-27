//! Identity's settings from [`EditorConfig`].

use std::time::Duration;

use identity_web::{IdentitySettings, SmtpPassword};

use crate::config::EditorConfig;

impl EditorConfig {
    pub(crate) fn identity_settings(&self, css_href: String) -> IdentitySettings {
        IdentitySettings {
            css_href,
            login_cooldown: Duration::from_secs(self.login_cooldown_secs),
            login_max_failed: self.login_max_failed,
            login_lockout: Duration::from_secs(self.login_lockout_secs),
            mail_daily_cap: self.mail_daily_cap,
            mail_account_daily_cap: self.mail_account_daily_cap,
            session_absolute: Duration::from_secs(self.session_absolute_secs),
            session_idle: Duration::from_secs(self.session_idle_secs),
            smtp_break_glass: self.smtp_break_glass,
            smtp_host: self.smtp_host.clone(),
            smtp_port: self.smtp_port,
            smtp_credentials: self
                .smtp_credentials()
                .map(|(username, password)| (username.to_string(), SmtpPassword::new(password.to_string()))),
            dkim: self
                .dkim()
                .map(|(selector, key_file)| (selector.to_string(), key_file.to_path_buf())),
            smtp_from: self.smtp_from.clone(),
            rdu_addresses: self.rdu_addresses(),
            reveal_login_code: self.reveals_login_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_auth_config_reads_every_knob_from_the_editor_config() {
        let config = EditorConfig::default();
        let auth = config.identity_settings(String::new()).auth();
        assert_eq!(auth.cooldown, Duration::from_secs(config.login_cooldown_secs));
        assert_eq!(auth.max_failed, config.login_max_failed);
        assert_eq!(auth.lockout, Duration::from_secs(config.login_lockout_secs));
        assert_eq!(auth.daily_cap, config.mail_daily_cap);
        assert_eq!(auth.account_daily_cap, config.mail_account_daily_cap);
        assert_eq!(auth.session_absolute, Duration::from_secs(config.session_absolute_secs));
        assert_eq!(auth.session_idle, Duration::from_secs(config.session_idle_secs));
        assert!(!auth.break_glass, "the break-glass log fallback must be off unless asked for");
    }

    #[test]
    fn test_a_configured_dkim_selector_and_key_reach_the_settings() {
        let config = EditorConfig {
            dkim_selector: Some("dasch-deposit-prod-2026".to_string()),
            dkim_private_key_file: Some(PathBuf::from("/run/secrets/dkim_private_key.pem")),
            ..EditorConfig::default()
        };
        assert_eq!(
            config.identity_settings(String::new()).dkim,
            Some((
                "dasch-deposit-prod-2026".to_string(),
                PathBuf::from("/run/secrets/dkim_private_key.pem")
            ))
        );
    }
}
