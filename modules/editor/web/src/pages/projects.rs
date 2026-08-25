//! The project list and the per-project page.
//!
//! Both are placeholders for the editing surface, which is the project form's
//! work. What is real here is the **scoping**: the list shows a depositor
//! exactly the shortcodes assigned to them (REQ-1.2), and the per-project page
//! is reached only through the check that answers REQ-1.3's 403 otherwise.
//!
//! The list is not the published project set. The editor cannot read that yet —
//! the data directory is an unconfigured seam until the canonical project reader
//! lands — so a depositor sees their assignments and an RDU member is told why
//! there is no list to show. Naming that is better than rendering an empty table
//! that reads as "there are no projects".

use maud::{html, Markup};

/// `GET /projects` for a depositor: the shortcodes assigned to them.
///
/// An empty assignment set is a real state, not an error — an account created a
/// moment ago has one — so it says who fixes it.
pub fn assigned(shortcodes: &[String]) -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Your projects" }
            @if shortcodes.is_empty() {
                p class="text-gray-600" {
                    "No projects are assigned to your account yet. RDU assigns them; ask them to add yours."
                }
            } @else {
                p class="text-gray-600 mb-6" {
                    "These are the projects you may edit. The editing form is not available yet."
                }
                ul class="flex flex-col gap-2" {
                    @for shortcode in shortcodes {
                        li {
                            a href={ "/projects/" (shortcode) } class="underline font-bold" {
                                (shortcode)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `GET /projects` for an RDU member.
///
/// RDU access is role-based rather than per-project (REQ-4.2), so there is no
/// assignment set to list and the account's own `shortcodes` is empty by design.
/// Every project is reachable by shortcode; the list of which projects exist
/// comes from the published set, which the editor does not read yet.
pub fn rdu_overview() -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Projects" }
            p class="text-gray-600 mb-4" {
                "Your access is role-based rather than per-project, so every project is open to you at "
                code class="font-mono" { "/projects/{shortcode}" }
                ". The list of published projects is not available here yet."
            }
            p {
                a href="/depositors" class="underline" { "Manage depositor accounts" }
            }
        }
    }
}

/// `GET /projects/{shortcode}` — reached only by someone who may.
pub fn project(shortcode: &str) -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Project " (shortcode) }
            p class="text-gray-600 mb-6" {
                "The editing form for this project is not available yet."
            }
            p {
                a href="/projects" class="underline" { "Back to your projects" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn test_the_list_links_each_assigned_shortcode_to_its_project() {
        let out = assigned(&codes(&["0801", "080C"])).into_string();
        assert!(out.contains(r#"<a href="/projects/0801""#), "{out}");
        assert!(out.contains(r#"<a href="/projects/080C""#), "{out}");
    }

    #[test]
    fn test_the_list_shows_nothing_that_was_not_assigned() {
        // The whole point of the page: it is the depositor's own scope
        // (REQ-1.2), not a directory of every project.
        let out = assigned(&codes(&["0801"])).into_string();
        assert!(!out.contains("0803"), "{out}");
    }

    #[test]
    fn test_an_empty_assignment_set_says_who_fixes_it() {
        // A new account has one, so it must not read as an error page.
        let out = assigned(&[]).into_string();
        assert!(out.contains("RDU"), "{out}");
        assert!(!out.contains("<ul"), "{out}");
    }

    #[test]
    fn test_the_rdu_overview_explains_the_absent_list_rather_than_showing_an_empty_one() {
        // An RDU account's `shortcodes` is empty by design (REQ-4.2), so
        // rendering it through `assigned` would tell an administrator they have
        // no projects.
        let out = rdu_overview().into_string();
        assert!(out.contains("role-based"), "{out}");
        assert!(out.contains("/projects/{shortcode}"), "{out}");
    }

    #[test]
    fn test_the_project_page_names_the_project_and_offers_a_route_back() {
        let out = project("0801").into_string();
        assert!(out.contains("Project 0801"), "{out}");
        assert!(out.contains(r#"<a href="/projects""#), "{out}");
    }

    #[test]
    fn test_a_shortcode_is_escaped_wherever_it_is_rendered() {
        // It arrives from a path segment and from a stored assignment. The path
        // is filtered upstream, but a page that depends on that is one route
        // away from being wrong.
        let hostile = "<script>alert(1)</script>";
        for out in [
            project(hostile).into_string(),
            assigned(&codes(&[hostile])).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }
}
