use dpe_core::Corpus;
use maud::{html, Markup};

/// Renders an organization name from the in-process org cache, by ID.
pub fn organization_name(organization_id: &str, corpus: &'static Corpus) -> Markup {
    match corpus.load_organization(organization_id) {
        Some(org) => html! {
            span class="font-semibold" { (org.name) }
        },
        None => html! {
            span class="italic text-neutral-500" { "Organization not found" }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_corpus;

    #[test]
    fn unknown_organization_renders_not_found() {
        // No project by this id is in the committed test corpus.
        let out = organization_name("organization-does-not-exist", test_corpus()).into_string();
        assert!(out.contains("Organization not found"), "{out}");
    }
}
