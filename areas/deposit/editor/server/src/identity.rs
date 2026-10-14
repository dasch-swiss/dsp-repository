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
            smtp_from: self.smtp_from.clone(),
            rdu_addresses: self.rdu_addresses(),
            reveal_login_code: self.reveals_login_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_core::records::normalize_shortcode;
    use editor_ports::{Principal, Role};

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

    fn depositor(shortcodes: &[&str]) -> Principal {
        Principal {
            account_id: uuid::Uuid::nil().to_string(),
            display_name: "A".to_string(),
            role: Role::Depositor,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            signed_out_at: std::time::SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn the_assignment_comparison_agrees_with_the_storage_key() {
        // Two expressions of one rule: `may_reach` compares, `normalize_shortcode`
        // keys. If they ever disagreed, a depositor could reach a project whose
        // draft they cannot load — or load someone else's. Checked over the
        // shapes the published set actually contains, mixed case included.
        for (assigned, requested) in [
            ("0801", "0801"),
            ("080C", "080c"),
            ("080c", "080C"),
            ("0801a", "0801A"),
            ("085F", "085f"),
        ] {
            assert!(
                depositor(&[assigned]).may_reach(requested),
                "{assigned} should reach {requested}"
            );
            assert_eq!(
                normalize_shortcode(assigned),
                normalize_shortcode(requested),
                "{assigned} and {requested} must key the same"
            );
        }
        for (assigned, requested) in [("0801", "0803"), ("080C", "080E")] {
            assert!(!depositor(&[assigned]).may_reach(requested));
            assert_ne!(normalize_shortcode(assigned), normalize_shortcode(requested));
        }
    }
}
