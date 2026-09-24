//! `GET /states`: what each state means and how long Online takes.
//!
//! Every label and explanation comes from [`ProjectState`], which the list
//! column and the waiting notice also read, so the three cannot drift. The page
//! iterates `ProjectState::ALL`, so a sixth state cannot be left unexplained.
use editor_core::status::ProjectState;
use maud::{html, Markup};

/// The state-explanation page.
pub fn explanation() -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            h1 class="font-display text-2xl mb-2" { "What the states mean" }
            p class="text-gray-600 mb-6" {
                "Every project you edit moves through these states. You will see the current one beside the \
                 project in your list."
            }
            // Spacing lives on a wrapper, never on the `dl`: overriding a list
            // element's `display` can strip its implicit list role in
            // Safari/VoiceOver (WCAG 1.3.1), and axe reads the DOM rather than
            // WebKit's role mapping, so nothing automated would catch it.
            div class="flex flex-col gap-6" {
                dl {
                    @for state in ProjectState::ALL {
                        div class="mb-6 last:mb-0" {
                            dt class="font-bold" { (state.label()) }
                            dd class="text-gray-600 mt-1" { (state.explanation()) }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_state_is_named_and_explained() {
        // Iterating `ALL` makes this true by construction; this pins that the
        // iteration reaches the rendering.
        let out = explanation().into_string();
        for state in ProjectState::ALL {
            assert!(out.contains(state.label()), "{} is missing from the page", state.label());
            assert!(
                out.contains(state.explanation()),
                "{} has no explanation on the page",
                state.label()
            );
        }
    }

    #[test]
    fn test_the_page_states_the_expected_wait_before_online() {
        // The half nothing else in the codebase would notice the loss of.
        let out = explanation().into_string();
        assert!(
            out.contains("few weeks"),
            "REQ-2.6 requires the expected wait before Online: {out}"
        );
    }
}
