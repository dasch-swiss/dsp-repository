//! `GET /states`: what each state means and how long Online takes.
//!
//! Every label and explanation comes from [`ProjectState`], which the list
//! column and the waiting notice also read, so the three cannot drift. The page
//! iterates `ProjectState::ALL`, so a sixth state cannot be left unexplained.
use editor_core::status::ProjectState;
use maud::{html, Markup};

use crate::state_badge::state_badge;

/// The state-explanation page.
pub fn explanation() -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            h1 class="font-display font-semibold text-2xl text-neutral-900 mb-2" {
                "What the states mean"
            }
            p class="text-neutral-600 mb-6" {
                "Every project you edit moves through these states. You will see the current one beside the \
                 project in your list."
            }
            (journey())
            // Spacing lives on a wrapper, never on the `dl`: overriding a list
            // element's `display` can strip its implicit list role in
            // Safari/VoiceOver (WCAG 1.3.1), and axe reads the DOM rather than
            // WebKit's role mapping, so nothing automated would catch it.
            div class="flex flex-col gap-6" {
                dl {
                    @for state in ProjectState::ALL {
                        div class="mb-6 last:mb-0" {
                            dt class="font-bold" { (state.label()) }
                            dd class="text-neutral-600 mt-1" { (state.explanation()) }
                        }
                    }
                }
            }
        }
    }
}

/// The states in order as a strip of the list's badges, with the expected wait on Approved.
/// `role="list"` stays: a flex `ol` without markers loses its list role in Safari/VoiceOver.
fn journey() -> Markup {
    html! {
        ol  role="list"
            aria-label="The states in order"
            class="flex flex-wrap items-start gap-x-2 gap-y-3 mb-8"
        {
            @for (index, state) in ProjectState::ALL.into_iter().enumerate() {
                li class="flex items-start gap-2" {
                    @if index > 0 {
                        span aria-hidden="true" class="mt-2.5 w-6 border-t-2 border-neutral-300" {}
                    }
                    span class="flex flex-col items-start gap-1" {
                        (state_badge(state))
                        @if state == ProjectState::Approved {
                            span class="text-xs text-neutral-600" { "usually same working day" }
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
            out.contains("same working day"),
            "REQ-2.6 requires the expected wait before Online: {out}"
        );
    }
}
