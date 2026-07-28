use maud::{html, Markup};

const TRUNCATE_THRESHOLD: usize = 500;

/// Expandable description text. For long text, expand/collapse is handled
/// entirely by Datastar signals (`data-signals` + `data-class` + `data-text`),
/// no WASM. Short text renders as a plain paragraph.
pub fn description(text: &str) -> Markup {
    if text.chars().count() > TRUNCATE_THRESHOLD {
        html! {
            div data-signals="{_expanded: false}" {
                p   id="description-text"
                    class="text-lg text-gray-600"
                    data-class="{'line-clamp-4': !$_expanded}"
                { (text) }
                button
                    class="text-primary cursor-pointer mt-2"
                    aria-controls="description-text"
                    aria-expanded="false"
                    data-attr:aria-expanded="$_expanded ? 'true' : 'false'"
                    data-on:click="$_expanded = !$_expanded"
                    data-text="$_expanded ? 'Show less' : 'Show more'"
                { "Show more" }
            }
        }
    } else {
        html! {
            p class="text-lg text-gray-600" { (text) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_a_plain_paragraph() {
        let out = description("short").into_string();
        assert_eq!(out, r#"<p class="text-lg text-gray-600">short</p>"#);
    }

    #[test]
    fn long_text_gets_datastar_expand_toggle() {
        let long = "x".repeat(600);
        let out = description(&long).into_string();
        assert!(out.contains(r#"data-signals="{_expanded: false}""#), "{out}");
        assert!(out.contains(r#"data-on:click="$_expanded = !$_expanded""#), "{out}");
        assert!(out.contains(r#"data-text="$_expanded ? 'Show less' : 'Show more'""#), "{out}");
        assert!(out.contains("Show more"), "{out}");
        // The toggle exposes its state to assistive technology.
        assert!(out.contains(r#"aria-expanded="false""#), "missing initial aria-expanded: {out}");
        assert!(
            out.contains(r#"data-attr:aria-expanded="$_expanded ? 'true' : 'false'""#),
            "{out}"
        );
        assert!(out.contains(r#"aria-controls="description-text""#), "{out}");
    }
}
