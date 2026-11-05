use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};

use crate::view::Viewer;

/// The global header, the page's banner landmark: logo and service name linking home, plus the
/// signed-in identity, the sign-out control and the viewer's menu when there is a session.
///
/// The DOM order (logo, title, menu, user block) is the tab order and the visual
/// order at every width; below `md` the menu wraps onto its own row.
///
/// Sign-out is a `<form method="post">`, not a link: a `GET /logout` is a
/// state-changing GET, which the `Sec-Fetch-Site` CSRF control cannot protect
/// since navigations are exempt, so any page could log a user out with an
/// `<img src>`. The name is shown, never the address: the header is on every
/// page and in every screenshot.
pub fn header(viewer: Option<Viewer<'_>>) -> Markup {
    html! {
        header class="bg-white shadow-xs" {
            div class="flex flex-wrap items-center gap-x-4 gap-y-1 py-2 max-w-[1536px] mx-auto px-4 w-full"
            {
                a href="/" aria-label="DaSCH Metadata Editor home" {
                    img src="/logo.svg" class="inline h-10 w-10 mr-2" alt="DaSCH logo";
                }

                div class="flex-1" {
                    a class="inline-flex items-center font-bold font-display text-xl" href="/" {
                        "DaSCH Metadata Editor"
                    }
                }

                @if let Some(viewer) = viewer {
                    nav aria-label="Main" class="w-full md:w-auto" {
                        ul class="flex flex-wrap gap-x-4 gap-y-1" {
                            @for (label, href) in viewer.menu.items() {
                                li {
                                    a   href=(href)
                                        class="font-bold text-gray-700 hover:underline aria-[current=page]:text-black aria-[current=page]:underline"
                                        aria-current=[is_current(href, viewer.path).then_some("page")]
                                    { (label) }
                                }
                            }
                        }
                    }
                    div class="flex items-center gap-3" {
                        span class="text-gray-600" {
                            "Signed in as "
                            span class="font-bold" { (viewer.name) }
                        }
                        form method="post" action="/logout" {
                            ({
                                button("Sign out")
                                    .button_type(ButtonType::Submit)
                                    .variant(ButtonVariant::Ghost)
                            })
                        }
                    }
                }
            }
        }
    }
}

/// Whether `href` is `path` or a path-segment prefix of it, so `/review`
/// matches `/review/0801` but not `/reviewer`.
fn is_current(href: &str, path: &str) -> bool {
    path.strip_prefix(href)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Menu;

    fn viewer(menu: Menu, path: &str) -> Viewer<'_> {
        Viewer { name: "A Depositor", menu, path }
    }

    const RDU: Menu = Menu::Rdu { pending_reviews: Some(3) };

    #[test]
    fn renders_logo_and_home_link() {
        let out = header(None).into_string();
        assert!(out.contains(r#"<img src="/logo.svg""#), "{out}");
        assert!(out.contains(r#"aria-label="DaSCH Metadata Editor home""#), "{out}");
        assert!(out.contains("DaSCH Metadata Editor"), "{out}");
        assert!(out.contains(r#"href="/""#), "{out}");
    }

    #[test]
    fn signed_out_header_offers_no_sign_out_control() {
        let out = header(None).into_string();
        assert!(!out.contains("Sign out"), "{out}");
        assert!(!out.contains("/logout"), "{out}");
    }

    #[test]
    fn signed_in_header_names_the_viewer_and_posts_to_logout() {
        let out = header(Some(viewer(Menu::Depositor, "/projects"))).into_string();
        assert!(out.contains("A Depositor"), "{out}");
        assert!(out.contains(r#"<form method="post" action="/logout">"#), "{out}");
    }

    #[test]
    fn sign_out_is_never_a_link() {
        let out = header(Some(viewer(Menu::Depositor, "/projects"))).into_string();
        assert!(!out.contains(r#"<a href="/logout""#), "{out}");
    }

    #[test]
    fn the_viewer_name_is_escaped() {
        let out = header(Some(Viewer {
            name: "<script>alert(1)</script>",
            menu: Menu::Depositor,
            path: "/",
        }))
        .into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    #[test]
    fn the_rdu_menu_lists_four_items_in_order_with_the_count() {
        let out = header(Some(viewer(RDU, "/projects"))).into_string();
        let at = |needle: &str| out.find(needle).unwrap_or_else(|| panic!("{needle} missing: {out}"));
        let order = [
            at(r#"href="/projects""#),
            at(r#"href="/review""#),
            at(r#"href="/approved""#),
            at(r#"href="/depositors""#),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{out}");
        assert!(out.contains("Review queue (3)"), "{out}");
        assert!(out.contains("Approved records"), "{out}");
        assert!(out.contains("Accounts"), "{out}");
    }

    #[test]
    fn an_unreadable_review_count_renders_no_parentheses() {
        let menu = Menu::Rdu { pending_reviews: None };
        let out = header(Some(viewer(menu, "/projects"))).into_string();
        assert!(out.contains("Review queue<"), "{out}");
        assert!(!out.contains("Review queue ("), "{out}");

        let zero = header(Some(viewer(Menu::Rdu { pending_reviews: Some(0) }, "/"))).into_string();
        assert!(zero.contains("Review queue (0)"), "{zero}");
    }

    #[test]
    fn the_depositor_menu_offers_only_projects() {
        let out = header(Some(viewer(Menu::Depositor, "/projects"))).into_string();
        assert!(out.contains(r#"href="/projects""#), "{out}");
        for rdu_only in ["/review", "/approved", "/depositors"] {
            assert!(!out.contains(rdu_only), "{rdu_only} in {out}");
        }
    }

    #[test]
    fn the_menu_precedes_the_sign_out_control_in_tab_order() {
        let out = header(Some(viewer(RDU, "/projects"))).into_string();
        let sign_out = out.find(r#"action="/logout""#).expect("sign-out form renders");
        let nav = out.find(r#"<nav aria-label="Main""#).expect("menu renders");
        assert!(nav < sign_out, "{out}");
        assert!(!out.contains("md:order-"), "{out}");
    }

    #[test]
    fn no_nav_when_signed_out() {
        let out = header(None).into_string();
        assert!(!out.contains("<nav"), "{out}");
    }

    #[test]
    fn aria_current_marks_only_the_requested_item() {
        let out = header(Some(viewer(RDU, "/review/0801"))).into_string();
        assert_eq!(out.matches("aria-current").count(), 1, "{out}");
        assert!(out.contains(r#"aria-current="page">Review queue (3)"#), "{out}");
    }

    #[test]
    fn current_matches_on_path_segment_boundaries() {
        assert!(is_current("/review", "/review"));
        assert!(is_current("/review", "/review/0801"));
        assert!(is_current("/projects", "/projects/0801/sections/overview"));
        assert!(is_current("/depositors", "/depositors/new"));
        assert!(!is_current("/review", "/reviewer"));
        assert!(!is_current("/projects", "/states"));
        assert!(!is_current("/projects", "/"));
        assert!(!is_current("/projects", ""));
    }
}
