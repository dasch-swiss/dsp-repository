use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, Export, Help};
use mosaic_tiles::link::link;

pub fn header_links() -> Markup {
    let help_label = html! {
        (icon(Help, "w-5 h-5"))
        "Help"
    };
    let deposit_label = html! {
        "Deposit Data at DaSCH"
        (icon(Export, "w-5 h-5"))
    };
    html! {
        ul class="flex items-center gap-4" {
            li { (link(help_label, "/dpe/about").as_button(ButtonVariant::Ghost)) }
            li {
                ({
                    link(deposit_label, "https://dasch.swiss")
                        .as_button(ButtonVariant::Primary)
                        .external()
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_help_link_to_about() {
        let out = header_links().into_string();
        assert!(out.contains(r#"href="/dpe/about""#), "{out}");
        assert!(out.contains("Help"), "{out}");
        // Ghost button variant styling.
        assert!(out.contains("btn btn-ghost"), "{out}");
    }

    #[test]
    fn renders_deposit_link_external() {
        let out = header_links().into_string();
        assert!(out.contains(r#"href="https://dasch.swiss""#), "{out}");
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains("Deposit Data at DaSCH"), "{out}");
        assert!(out.contains("btn btn-primary"), "{out}");
    }
}
