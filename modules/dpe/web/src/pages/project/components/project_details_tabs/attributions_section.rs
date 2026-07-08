use dpe_core::ResolvedContributor;
use maud::{html, Markup};

use super::super::contributor::contributor;

/// The "Contributors" tab panel: a grid of resolved contributors. Renders
/// nothing when there are none.
pub fn attributions_section(contributors: &[ResolvedContributor]) -> Markup {
    if contributors.is_empty() {
        return html! {};
    }
    html! {
        div class="grid md:grid-cols-1 lg:grid-cols-2 gap-2" {
            @for c in contributors { (contributor(c)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_person_contributor;

    #[test]
    fn renders_a_grid_of_contributors() {
        let contributors = vec![sample_person_contributor()];
        let out = attributions_section(&contributors).into_string();
        assert!(out.contains("grid"), "{out}");
        assert!(out.contains("Ada Lovelace"), "{out}");
    }

    #[test]
    fn empty_renders_nothing() {
        assert_eq!(attributions_section(&[]).into_string(), "");
    }
}
