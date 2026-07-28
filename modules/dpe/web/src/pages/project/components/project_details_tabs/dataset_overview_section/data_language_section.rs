use maud::{html, Markup};

use super::CHIP_NEUTRAL;

/// "Data Languages" chips. Renders nothing when empty.
pub fn data_language_section(data_languages: &[String]) -> Markup {
    if data_languages.is_empty() {
        return html! {};
    }
    html! {
        div {
            h3 class="dpe-subtitle" { "Data Languages" }
            div class="flex flex-wrap gap-1.5" {
                @for l in data_languages {
                    span class=(CHIP_NEUTRAL) { (l) }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_language_chips() {
        let langs = vec!["English".to_string(), "German".to_string()];
        let out = data_language_section(&langs).into_string();
        assert!(out.contains("Data Languages"), "{out}");
        assert!(out.contains(">English</span>"), "{out}");
    }

    #[test]
    fn empty_renders_nothing() {
        assert_eq!(data_language_section(&[]).into_string(), "");
    }
}
