use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, Export, Help};
use mosaic_tiles::link::{link, LinkProps};

pub fn header_links() -> Markup {
    html! {
        ul class="flex items-center gap-4" {
            li {
                (link(
                    LinkProps { href: "/dpe/about", as_button: Some(ButtonVariant::Ghost), ..Default::default() },
                    html! {
                        (icon(Help, "w-5 h-5"))
                        "Help"
                    },
                ))
            }
            li {
                (link(
                    LinkProps {
                        href: "https://dasch.swiss",
                        as_button: Some(ButtonVariant::Primary),
                        target: Some("_blank"),
                        ..Default::default()
                    },
                    html! {
                        "Deposit Data at DaSCH"
                        (icon(Export, "w-5 h-5"))
                    },
                ))
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
