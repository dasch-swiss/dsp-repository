//! The two login screens: the address form and the code-entry form.
//!
//! Both are plain `<form method="post">`, no Datastar: login has to work before
//! any script does. Neither page renders the address the user typed: the
//! browser is bound to its code by an `HttpOnly` cookie, so nothing has to be
//! carried in a hidden field, and a page with no address on it cannot leak one
//! through a screenshot, a shared URL or a cached response.
//!
//! Both take `next`, rendered into form actions and one link, never into a field
//! the reader can see or edit; the caller has checked it is a path inside this
//! service, and Maud escapes it on top. The error strings are the caller's:
//! whether a message may say "that address is not registered" is an
//! anti-enumeration decision for the handler. The `alert` tile renders
//! `role="alert"` for `Danger`, so a screen reader announces a re-rendered
//! failure on arrival.

use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType};
use mosaic_tiles::text_field::{text_field, InputType};

/// `path`, carrying `next` as a query parameter when there is one. Interpolated
/// rather than encoded: the caller restricts it to unreserved characters (see
/// `safe_next` in `editor-server`), and Maud escapes it into the attribute.
fn with_next(path: &str, next: Option<&str>) -> String {
    match next {
        Some(next) => format!("{path}?next={next}"),
        None => path.to_string(),
    }
}

/// `GET /login` — ask for the address.
pub fn request_code(next: Option<&str>, error: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto py-12" {
            h1 class="font-display text-2xl mb-2" { "Sign in" }
            p class="text-gray-600 mb-6" {
                "Enter your email address and we will send you a six-digit code. The code is valid for ten \
                 minutes and can be used once."
            }
            @if let Some(message) = error { (alert(message).variant(AlertVariant::Danger)) }
            form method="post" action=(with_next("/login", next)) class="flex flex-col gap-4" {
                (email_field())
                div { (button("Send me a code").button_type(ButtonType::Submit)) }
            }
        }
    }
}

/// The login code, shown on the page instead of being sent. Rendered only where
/// `EditorConfig::reveals_login_code` holds: no mail relay, no durable database,
/// not production. Styled as a warning, naming the reason, because a page
/// showing a live credential must not look ordinary.
fn revealed_code(code: &str) -> Markup {
    let body = html! {
        p class="mt-1" {
            "This service has no mail relay and no database that outlives it, so your code is shown here instead \
             of being emailed:"
        }
        p class="font-mono text-2xl tracking-widest mt-2" { (code) }
    };
    html! {
        ({
            alert(body)
                .variant(AlertVariant::Warning)
                .title("Development deployment — no mail was sent")
        })
    }
}

fn email_field() -> Markup {
    html! {
        ({
            text_field("email", "Email address")
                .input_type(InputType::Email)
                .autocomplete("email")
                .autofocus()
                .required()
        })
    }
}

/// The six-digit code field. `one_time_code` is what makes a phone offer the
/// code from the message it arrived in.
fn code_field() -> Markup {
    html! {
        (text_field("code", "Six-digit code").one_time_code(6).autofocus().required())
    }
}

/// `GET /login/code`: take the six digits.
///
/// The way back is a link to `/login`, not a resend button: a resend needs the
/// address, and the only places to keep it would be a hidden field or a third
/// endpoint. `one_time_code(6)` sets `autocomplete`, `inputmode`, the pattern
/// and the length together; the tile says why they only work as a set.
pub fn enter_code(next: Option<&str>, error: Option<&str>, revealed: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto py-12" {
            h1 class="font-display text-2xl mb-2" { "Enter your code" }
            p class="text-gray-600 mb-6" {
                "If the address you gave belongs to an account, a six-digit code is on its way to it. Enter the \
                 code below."
            }
            @if let Some(message) = error { (alert(message).variant(AlertVariant::Danger)) }
            @if let Some(code) = revealed { (revealed_code(code)) }
            form method="post" action=(with_next("/login/code", next)) class="flex flex-col gap-4" {
                (code_field())
                div { (button("Sign in").button_type(ButtonType::Submit)) }
            }
            p class="text-gray-600 mt-6" {
                "Code not arrived, or expired? "
                a href=(with_next("/login", next)) class="underline" { "Start again" }
                "."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_code_posts_the_address_to_login() {
        let out = request_code(None, None).into_string();
        assert!(out.contains(r#"<form method="post" action="/login""#), "{out}");
        assert!(out.contains(r#"name="email""#), "{out}");
        assert!(out.contains(r#"type="email""#), "{out}");
        assert!(out.contains(r#"type="submit""#), "{out}");
    }

    #[test]
    fn test_enter_code_constrains_the_field_to_six_digits() {
        // Not validation — the server does that — but it keeps a mistyped
        // seven-digit paste from costing an attempt against the account counter.
        let out = enter_code(None, None, None).into_string();
        assert!(out.contains(r#"<form method="post" action="/login/code""#), "{out}");
        assert!(out.contains(r#"pattern="[0-9]{6}""#), "{out}");
        assert!(out.contains(r#"maxlength="6""#), "{out}");
        assert!(out.contains(r#"inputmode="numeric""#), "{out}");
        assert!(out.contains(r#"autocomplete="one-time-code""#), "{out}");
    }

    #[test]
    fn test_both_forms_are_plain_posts_without_datastar() {
        for out in [
            request_code(None, None).into_string(),
            enter_code(None, None, None).into_string(),
        ] {
            assert!(!out.contains("data-on"), "{out}");
            assert!(!out.contains("data-bind"), "{out}");
            assert!(out.contains(r#"method="post""#), "{out}");
        }
    }

    #[test]
    fn test_an_error_renders_as_an_alert_and_is_absent_otherwise() {
        let with = request_code(None, Some("Enter a valid email address.")).into_string();
        assert!(with.contains(r#"role="alert""#), "{with}");
        assert!(with.contains("Enter a valid email address."), "{with}");
        assert!(!request_code(None, None).into_string().contains(r#"role="alert""#));
    }

    #[test]
    fn test_an_error_message_is_escaped() {
        let out = enter_code(None, Some("<script>alert(1)</script>"), None).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    #[test]
    fn test_the_code_is_absent_unless_the_caller_passes_one() {
        // Every deployment that mails a code renders this page, so the block
        // appearing by accident is the failure that matters.
        let out = enter_code(None, None, None).into_string();
        assert!(!out.contains("no mail was sent"), "{out}");
        assert!(!out.contains("alert-warning"), "{out}");
    }

    #[test]
    fn test_a_revealed_code_says_why_it_is_on_the_page() {
        let out = enter_code(None, None, Some("482917")).into_string();
        assert!(out.contains("482917"), "{out}");
        assert!(out.contains("no mail relay"), "{out}");
        assert!(out.contains("alert-warning"), "{out}");
    }

    #[test]
    fn test_a_revealed_code_is_escaped() {
        // The code comes from the database, not the request; a credential rendered
        // into a page is still escaped here.
        let out = enter_code(None, None, Some("<script>alert(1)</script>")).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    #[test]
    fn test_neither_page_carries_an_address_field_to_repost() {
        let out = enter_code(None, None, None).into_string();
        assert!(!out.contains(r#"type="hidden""#), "{out}");
        assert!(!out.contains(r#"name="email""#), "{out}");
    }

    #[test]
    fn test_the_way_back_from_the_code_page_is_a_get_link() {
        let out = enter_code(None, None, None).into_string();
        assert!(out.contains(r#"<a href="/login""#), "{out}");
    }

    #[test]
    fn test_a_destination_survives_both_screens_and_the_way_back() {
        // Without it, someone who follows a link into a project and is sent to
        // sign in lands on the root afterwards and has to find their way again.
        let first = request_code(Some("/projects/0801"), None).into_string();
        assert!(first.contains(r#"action="/login?next=/projects/0801""#), "{first}");

        let second = enter_code(Some("/projects/0801"), None, None).into_string();
        assert!(second.contains(r#"action="/login/code?next=/projects/0801""#), "{second}");
        assert!(second.contains(r#"<a href="/login?next=/projects/0801""#), "{second}");
    }

    #[test]
    fn test_the_destination_never_becomes_a_field_the_reader_can_edit() {
        // In the action, not the form body: an editable field invites a reader to
        // change where signing in sends them.
        let out = request_code(Some("/projects/0801"), None).into_string();
        assert!(!out.contains(r#"name="next""#), "{out}");
        assert!(!out.contains(r#"type="hidden""#), "{out}");
    }

    #[test]
    fn test_a_destination_cannot_break_out_of_the_attribute_it_is_rendered_into() {
        // The server validates it first; this is the second layer.
        let out = request_code(Some(r#"/x" onmouseover="alert(1)"#), None).into_string();
        assert!(!out.contains(r#"onmouseover="alert(1)""#), "{out}");
        assert!(out.contains("&quot;"), "{out}");
    }
}
