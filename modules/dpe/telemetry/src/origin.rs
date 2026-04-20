/// Check if a host is an allowed origin for telemetry beacons.
/// Accepts: dasch.swiss, *.dasch.swiss, and localhost.
pub fn is_allowed_origin(host: &str) -> bool {
    if host == "dasch.swiss" {
        return true;
    }
    // Require the character before "dasch.swiss" to be a dot,
    // preventing "evil-dasch.swiss" from matching.
    if let Some(prefix) = host.strip_suffix("dasch.swiss") {
        if prefix.ends_with('.') {
            return true;
        }
    }
    // Localhost: Url::parse strips port, so host is always "localhost".
    host == "localhost"
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn exact_dasch_swiss_accepted() {
        assert!(is_allowed_origin("dasch.swiss"));
    }

    #[test]
    fn subdomains_accepted() {
        assert!(is_allowed_origin("repository.dasch.swiss"));
        assert!(is_allowed_origin("repository.dev.dasch.swiss"));
    }

    #[test]
    fn lookalike_rejected() {
        assert!(!is_allowed_origin("evil-dasch.swiss"));
        assert!(!is_allowed_origin("notdasch.swiss"));
    }

    #[test]
    fn localhost_accepted() {
        assert!(is_allowed_origin("localhost"));
    }

    #[test]
    fn foreign_rejected() {
        assert!(!is_allowed_origin("attacker.com"));
        assert!(!is_allowed_origin("evil-dasch.swiss.attacker.com"));
    }

    #[test]
    fn empty_rejected() {
        assert!(!is_allowed_origin(""));
    }

    // --- Property-based tests ---

    fn dns_label() -> impl Strategy<Value = String> {
        "[a-z0-9][a-z0-9\\-]{0,18}[a-z0-9]"
    }

    fn lookalike_prefix() -> impl Strategy<Value = String> {
        "[a-z0-9][a-z0-9\\-]{0,10}"
    }

    proptest! {
        #[test]
        fn proper_subdomain_always_accepted(label in dns_label()) {
            let host = format!("{label}.dasch.swiss");
            prop_assert!(is_allowed_origin(&host), "should accept: {host}");
        }

        #[test]
        fn nested_subdomain_always_accepted(l1 in dns_label(), l2 in dns_label()) {
            let host = format!("{l1}.{l2}.dasch.swiss");
            prop_assert!(is_allowed_origin(&host), "should accept: {host}");
        }

        #[test]
        fn lookalike_domain_always_rejected(prefix in lookalike_prefix()) {
            let host = format!("{prefix}dasch.swiss");
            prop_assume!(host != "dasch.swiss");
            prop_assert!(!is_allowed_origin(&host), "should reject: {host}");
        }

        #[test]
        fn arbitrary_domain_rejected(domain in "[a-z]{1,10}\\.[a-z]{2,6}") {
            prop_assume!(domain != "dasch.swiss");
            prop_assume!(!domain.ends_with(".dasch.swiss"));
            prop_assume!(domain != "localhost");
            prop_assert!(!is_allowed_origin(&domain), "should reject: {domain}");
        }
    }
}
