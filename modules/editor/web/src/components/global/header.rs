use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};

use crate::view::Viewer;

/// The global header: DaSCH logo and the service name, both linking home, plus
/// the signed-in identity and the sign-out control when there is a session.
///
/// Deliberately thinner than DPE's. DPE's header carries public wayfinding
/// (Help, "Deposit Data at DaSCH"); the editor is an authenticated tool whose
/// users arrive knowing why they are here.
///
/// Sign-out is a `<form method="post">`, not a link. A `GET /logout` would be a
/// state-changing GET — which is the one thing the `Sec-Fetch-Site` CSRF control
/// cannot protect, since navigations are exempt from it by necessity. Any page
/// on the internet could then log a user out with an `<img src>`.
///
/// The name is shown, never the address: the header appears on every page and
/// in every screenshot, and the name is what identifies the account to its owner.
pub fn header(viewer: Option<Viewer<'_>>) -> Markup {
    html! {
        div class="bg-white shadow-xs" {
            div class="flex items-center py-2 max-w-[1536px] mx-auto px-4 w-full" {
                a href="/" aria-label="DaSCH Metadata Editor home" {
                    img src="/logo.svg" class="inline h-10 w-10 mr-2" alt="DaSCH logo";
                }

                div class="flex-1" {
                    a class="inline-flex items-center font-bold font-display text-xl" href="/" {
                        "DaSCH Metadata Editor"
                    }
                }

                @if let Some(viewer) = viewer {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let out = header(Some(Viewer { name: "A Depositor" })).into_string();
        assert!(out.contains("A Depositor"), "{out}");
        assert!(out.contains(r#"<form method="post" action="/logout">"#), "{out}");
    }

    #[test]
    fn sign_out_is_never_a_link() {
        // A `GET /logout` is a state-changing GET, the one shape the
        // `Sec-Fetch-Site` control cannot cover — navigations are exempt from it
        // by necessity, so any page could log a user out with an `<img src>`.
        let out = header(Some(Viewer { name: "A Depositor" })).into_string();
        assert!(!out.contains(r#"<a href="/logout""#), "{out}");
    }

    #[test]
    fn the_viewer_name_is_escaped() {
        let out = header(Some(Viewer { name: "<script>alert(1)</script>" })).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
