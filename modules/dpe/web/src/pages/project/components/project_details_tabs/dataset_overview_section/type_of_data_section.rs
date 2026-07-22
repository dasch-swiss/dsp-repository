use maud::{html, Markup};

use super::CHIP_PRIMARY;

/// "Type of Data" chips. Renders nothing when the field is absent.
pub fn type_of_data_section(type_of_data: Option<&[String]>) -> Markup {
    match type_of_data {
        Some(types) => html! {
            div {
                h3 class="dpe-subtitle" { "Type of Data" }
                div class="flex flex-wrap gap-1.5" {
                    @for t in types {
                        span class=(CHIP_PRIMARY) { (t) }
                    }
                }
            }
        },
        None => html! {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_chips_for_each_type() {
        let types = vec!["Text".to_string(), "Image".to_string()];
        let out = type_of_data_section(Some(&types)).into_string();
        assert!(out.contains("Type of Data"), "{out}");
        assert!(out.contains(">Text</span>"), "{out}");
        assert!(out.contains(">Image</span>"), "{out}");
    }

    #[test]
    fn absent_renders_nothing() {
        assert_eq!(type_of_data_section(None).into_string(), "");
    }
}
