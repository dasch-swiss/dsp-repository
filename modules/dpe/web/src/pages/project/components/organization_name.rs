use maud::{html, Markup};

/// Renders an organization name from the in-process org cache, by ID.
pub fn organization_name(organization_id: &str) -> Markup {
    match dpe_core::load_organization(organization_id) {
        Some(org) => html! { span class="font-semibold" { (org.name) } },
        None => html! { span class="italic text-base-content/70" { "Organization not found" } },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_organization_renders_not_found() {
        // No org cache is populated in the unit-test environment.
        let out = organization_name("organization-does-not-exist").into_string();
        assert!(out.contains("Organization not found"), "{out}");
    }
}
