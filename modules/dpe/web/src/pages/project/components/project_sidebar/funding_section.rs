use dpe_core::project::{Funding, Grant};
use maud::{html, Markup};
use mosaic_tiles::icon::{icon, LinkExternal};

use super::super::info_card::info_card;
use super::super::organization_name::organization_name;
use crate::components::placeholder_value;

/// A single grant rendered inside an `info_card`: its funders, grant number,
/// name, and an optional external "More info" link.
fn grant_card(grant: &Grant) -> Markup {
    let body = html! {
        div {
            @for (i, funder_id) in grant.funders.iter().enumerate() {
                span {
                    @if i > 0 { ", " }
                    (organization_name(funder_id))
                }
            }
        }
        @if let Some(number) = &grant.number {
            div { "Grant: " (number) }
        }
        @if let Some(name) = &grant.name {
            div { (name) }
        }
        @if let Some(url) = &grant.url {
            a href=(url) class="text-primary items-center gap-1" target="_blank" {
                "More info"
                (icon(LinkExternal, "w-3 h-3"))
            }
        }
    };
    info_card(body)
}

/// The funding block: a list of grant cards, or a free-text funding statement.
pub fn funding_section(funding: &Funding) -> Markup {
    html! {
        div {
            @match funding {
                Funding::Grants(grants) => {
                    div class="space-y-2" {
                        div class="dpe-subtitle" { "Grants" }
                        @for grant in grants { (grant_card(grant)) }
                    }
                }
                Funding::Text(text) => {
                    @if dpe_core::is_placeholder(text) { (placeholder_value(text)) } @else {
                        div class="text-neutral-500" { (text) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::project::Grant;

    use super::*;

    #[test]
    fn grants_render_number_name_and_link() {
        let funding = Funding::Grants(vec![Grant {
            funders: vec!["organization-001".to_string()],
            number: Some("12345".to_string()),
            name: Some("Big Grant".to_string()),
            url: Some("https://example.org/grant".to_string()),
        }]);
        let out = funding_section(&funding).into_string();
        assert!(out.contains("Grants"), "{out}");
        assert!(out.contains("Grant: 12345"), "{out}");
        assert!(out.contains("Big Grant"), "{out}");
        assert!(out.contains(r#"href="https://example.org/grant""#), "{out}");
    }

    #[test]
    fn free_text_funding_renders() {
        let out = funding_section(&Funding::Text("Self-funded".to_string())).into_string();
        assert!(out.contains("Self-funded"), "{out}");
    }
}
