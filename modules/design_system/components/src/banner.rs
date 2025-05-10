use maud::html;

// TODO: Into String instead of String
pub fn banner(prefix: Option<String>, accent: String, suffix: Option<String>) -> String {
    html! {
        h1 .dsp-banner role="banner" {
            @if let Some(p) = prefix {
                span .dsp-banner__prefix { (p) }
                br;
            }
            span .dsp-banner__accent { (accent) }
            @if let Some(s) = suffix {
                br;
                span .dsp-banner__suffix { (s) }
            }
        }
    }
    .into_string()
}
