use components::{footer, header, shell};
use maud::{html, Markup, DOCTYPE};

/// Common page layout wrapper
pub fn page_layout(title: &str, content: Markup) -> Markup {
    // Configure header
    let header_config = header::HeaderConfig {
        company_name: "DSP Repository",
        logo_light_url: "/assets/logo.png",
        logo_dark_url: "/assets/logo.png",
        login_href: "",
    };

    let nav_elements = vec![
        header::NavElement::Item(header::NavItem { label: "Explore Data", href: "/projects" }),
        header::NavElement::Item(header::NavItem { label: "Our Services", href: "/services" }),
        header::NavElement::Item(header::NavItem { label: "Knowledge Hub", href: "/knowledge-hub" }),
        header::NavElement::Item(header::NavItem { label: "About Us", href: "/about-us" }),
        header::NavElement::Item(header::NavItem { label: "FAQ", href: "/faq" }),
        header::NavElement::Item(header::NavItem { label: "Status", href: "/status" }),
    ];

    // Configure footer
    let footer_config = footer::FooterConfig {
        company_name: "DSP Repository",
        description: "A long-term archive for humanities research data.",
        copyright_text: "2024 DSP Repository. All rights reserved.",
        logo_light_url: "/assets/logo.png",
        logo_dark_url: "/assets/logo.png",
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }

                // Tailwind CSS
                script src="https://cdn.tailwindcss.com" {}

                // DataStar
                script type="module" src="https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.0-RC.6/bundles/datastar.js" {}
            }
            body {
                (shell::shell(nav_elements, header_config, footer_config)
                    .with_content(content)
                    .build())
            }
        }
    }
}
