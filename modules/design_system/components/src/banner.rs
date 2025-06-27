use maud::{html, Markup};

fn banner(
    prefix: Option<impl Into<String>>,
    accent: impl Into<String>,
    suffix: Option<impl Into<String>>,
    test_id: &str,
) -> Markup {
    html! {
        h1 .dsp-banner role="banner" data-testid=(test_id) {
            @if let Some(p) = prefix {
                span .dsp-banner__prefix data-testid=(format!("{}-prefix", test_id)) { (p.into()) }
                br;
            }
            span .dsp-banner__accent data-testid=(format!("{}-accent", test_id)) { (accent.into()) }
            @if let Some(s) = suffix {
                br;
                span .dsp-banner__suffix data-testid=(format!("{}-suffix", test_id)) { (s.into()) }
            }
        }
    }
}

pub fn accent_only(accent: impl Into<String>) -> Markup {
    accent_only_with_testid(accent, "banner-accent-only")
}

pub fn accent_only_with_testid(accent: impl Into<String>, test_id: &str) -> Markup {
    banner(None::<&str>, accent, None::<&str>, test_id)
}

pub fn with_prefix(prefix: impl Into<String>, accent: impl Into<String>) -> Markup {
    with_prefix_and_testid(prefix, accent, "banner-with-prefix")
}

pub fn with_prefix_and_testid(prefix: impl Into<String>, accent: impl Into<String>, test_id: &str) -> Markup {
    banner(Some(prefix), accent, None::<&str>, test_id)
}

pub fn with_suffix(accent: impl Into<String>, suffix: impl Into<String>) -> Markup {
    with_suffix_and_testid(accent, suffix, "banner-with-suffix")
}

pub fn with_suffix_and_testid(accent: impl Into<String>, suffix: impl Into<String>, test_id: &str) -> Markup {
    banner(None::<&str>, accent, Some(suffix), test_id)
}

pub fn full(prefix: impl Into<String>, accent: impl Into<String>, suffix: impl Into<String>) -> Markup {
    full_with_testid(prefix, accent, suffix, "banner-full")
}

pub fn full_with_testid(
    prefix: impl Into<String>,
    accent: impl Into<String>,
    suffix: impl Into<String>,
    test_id: &str,
) -> Markup {
    banner(Some(prefix), accent, Some(suffix), test_id)
}
