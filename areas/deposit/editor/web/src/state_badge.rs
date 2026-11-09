//! The badge for a project's state, one look per state wherever a state is shown.

use editor_core::status::ProjectState;
use maud::Markup;
use mosaic_tiles::badge::{badge, BadgeSize, BadgeVariant};
use mosaic_tiles::ComponentBuilder;

/// The state's badge, labelled with [`ProjectState::label`]. Exhaustive with no wildcard arm, so a
/// sixth state fails to compile here rather than borrowing another state's look.
pub fn state_badge(state: ProjectState) -> Markup {
    let variant = match state {
        ProjectState::Draft => BadgeVariant::Secondary,
        ProjectState::Submitted => BadgeVariant::Info,
        ProjectState::InReview => BadgeVariant::Primary,
        ProjectState::Approved | ProjectState::Online => BadgeVariant::Success,
    };
    let badge = badge(state.label()).variant(variant).size(BadgeSize::Small).status();
    match state {
        ProjectState::Online => badge.solid().build(),
        _ => badge.build(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_of(state: ProjectState) -> String {
        let out = state_badge(state).into_string();
        let start = out.find(r#"class=""#).expect("class renders") + r#"class=""#.len();
        out[start..start + out[start..].find('"').expect("class closes")].to_string()
    }

    #[test]
    fn test_every_state_has_a_look_of_its_own() {
        let looks: std::collections::HashSet<String> = ProjectState::ALL.into_iter().map(class_of).collect();
        assert_eq!(looks.len(), ProjectState::ALL.len(), "{looks:?}");
    }

    #[test]
    fn test_every_badge_carries_its_state_label() {
        for state in ProjectState::ALL {
            let out = state_badge(state).into_string();
            assert!(out.contains(&format!(">{}</span>", state.label())), "{out}");
        }
    }

    #[test]
    fn test_only_online_is_solid() {
        for state in ProjectState::ALL {
            let solid = class_of(state).contains("badge-solid");
            assert_eq!(solid, state == ProjectState::Online, "{state:?}");
        }
    }
}
