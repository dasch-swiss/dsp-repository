use maud::{html, Markup};

fn banner(prefix: Option<impl Into<String>>, accent: impl Into<String>, suffix: Option<impl Into<String>>) -> Markup {
    html! {
        h1 .dsp-banner role="banner" {
            @if let Some(p) = prefix {
                span .dsp-banner__prefix { (p.into()) }
                br;
            }
            span .dsp-banner__accent { (accent.into()) }
            @if let Some(s) = suffix {
                br;
                span .dsp-banner__suffix { (s.into()) }
            }
        }
    }
}

pub fn accent_only(accent: impl Into<String>) -> Markup {
    banner(None::<&str>, accent, None::<&str>)
}

pub fn with_prefix(prefix: impl Into<String>, accent: impl Into<String>) -> Markup {
    banner(Some(prefix), accent, None::<&str>)
}

pub fn with_suffix(accent: impl Into<String>, suffix: impl Into<String>) -> Markup {
    banner(None::<&str>, accent, Some(suffix))
}

pub fn full(prefix: impl Into<String>, accent: impl Into<String>, suffix: impl Into<String>) -> Markup {
    banner(Some(prefix), accent, Some(suffix))
}
