//! The two login screens: the address form and the code-entry form.
//!
//! Both are plain `<form method="post">` — no Datastar, no JavaScript. Login is
//! the one surface that has to work before anything else does, and a fetch-based
//! submit here would put the whole authentication flow behind a script load.
//!
//! Neither page renders the address the user typed. Not because the markup is a
//! log — REQ-6.10 is about logs and traces — but because it never needs to: the
//! browser is bound to its code by an `HttpOnly` cookie, so nothing has to be
//! carried in a hidden field, and a page with no address on it cannot leak one
//! through a screenshot, a shared URL or a cached response.
//!
//! Both take `next` — where the reader was going when they were sent here. It
//! is rendered into form actions and one link, never into a field the reader can
//! see or edit. The caller has already checked that it is a path inside this
//! service; these pages add HTML escaping on top, so the worst a bad value could
//! do here is produce a link that goes nowhere.
//!
//! The error strings are the caller's, deliberately. Whether a message may say
//! "that address is not registered" is an anti-enumeration decision (REQ-6.2)
//! and belongs with the handler that knows, not with the template.
//!
//! The banner and the two fields are Mosaic tiles. `alert` renders `role="alert"`
//! for its `Danger` variant, so a screen reader announces the message on
//! arrival: these pages re-render on failure, so it is present at load rather
//! than injected.

use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType};
use mosaic_tiles::text_field::{text_field, InputType};

/// `path`, carrying `next` as a query parameter when there is one.
///
/// The value is interpolated rather than encoded, which is sound only because
/// the caller restricts it to unreserved characters — see `safe_next` in
/// `editor-server`. Maud escapes it into the attribute regardless, so a value
/// that slipped through that check cannot break out of the `href`.
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
            @if let Some(message) = error {
                (alert(message).variant(AlertVariant::Danger).class("mb-4"))
            }
            form method="post" action=(with_next("/login", next)) class="flex flex-col gap-4" {
                (email_field())
                div { (button("Send me a code").button_type(ButtonType::Submit)) }
            }
        }
    }
}

/// The login code, shown on the page instead of being sent.
///
/// Rendered only where the caller has established that this deployment has no
/// mail relay, no durable database and is not production — see
/// `EditorConfig::reveals_login_code`. It is styled as a warning rather than a
/// convenience because a page that shows a live credential should not look
/// ordinary, and it names the reason so nobody has to guess whether the
/// deployment is misconfigured or deliberately throwaway.
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
                .class("mb-4")
        })
    }
}

/// The address field, named so the form body reads as a list of fields rather
/// than a wall of builder calls.
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

/// `GET /login/code` — take the six digits.
///
/// The way back is a link to `/login`, not a resend button: a resend needs the
/// address, and the only places to keep it would be a hidden field or a third
/// endpoint. Retyping it costs a legitimate user a few seconds and keeps the
/// address off the page.
///
/// The field is `text_field(...).one_time_code(6)`, which sets
/// `autocomplete="one-time-code"`, `inputmode="numeric"`, the pattern and the
/// length together — see the tile for why those four only work as a set.
pub fn enter_code(next: Option<&str>, error: Option<&str>, revealed: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto py-12" {
            h1 class="font-display text-2xl mb-2" { "Enter your code" }
            p class="text-gray-600 mb-6" {
                "If the address you gave belongs to an account, a six-digit code is on its way to it. Enter the \
                 code below."
            }
            @if let Some(message) = error {
                (alert(message).variant(AlertVariant::Danger).class("mb-4"))
            }
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
        // Login has to work before any script does. A `data-on:` control here
        // would put the whole authentication flow behind a bundle load.
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
        // The default is no reveal. Every deployment that mails a code renders
        // this page, so the block appearing by accident is the failure that
        // matters.
        let out = enter_code(None, None, None).into_string();
        assert!(!out.contains("no mail was sent"), "{out}");
        assert!(!out.contains("alert-warning"), "{out}");
    }

    #[test]
    fn test_a_revealed_code_says_why_it_is_on_the_page() {
        // A page showing a live credential must not look ordinary, and it has to
        // distinguish "deliberately throwaway" from "misconfigured relay".
        let out = enter_code(None, None, Some("482917")).into_string();
        assert!(out.contains("482917"), "{out}");
        assert!(out.contains("no mail relay"), "{out}");
        assert!(out.contains("alert-warning"), "{out}");
    }

    #[test]
    fn test_a_revealed_code_is_escaped() {
        // It comes from the database rather than the request, so this is the
        // second layer rather than the first — but a credential rendered into a
        // page is the last place to rely on someone else's validation.
        let out = enter_code(None, None, Some("<script>alert(1)</script>")).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    #[test]
    fn test_neither_page_carries_an_address_field_to_repost() {
        // The browser is bound to its code by an HttpOnly cookie, so the address
        // never has to be carried forward. A hidden field would put it in the
        // markup, in the back/forward cache and in any screenshot of the page.
        let out = enter_code(None, None, None).into_string();
        assert!(!out.contains(r#"type="hidden""#), "{out}");
        assert!(!out.contains(r#"name="email""#), "{out}");
    }

    #[test]
    fn test_the_way_back_from_the_code_page_is_a_get_link() {
        // Not a resend button: a resend needs the address, and the only places to
        // keep it are a hidden field or a third endpoint.
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
        // It belongs in the action, not in the form body: a visible or editable
        // field invites a reader to change where signing in sends them, and puts
        // one more thing in the markup of the page that must work.
        let out = request_code(Some("/projects/0801"), None).into_string();
        assert!(!out.contains(r#"name="next""#), "{out}");
        assert!(!out.contains(r#"type="hidden""#), "{out}");
    }

    #[test]
    fn test_a_destination_cannot_break_out_of_the_attribute_it_is_rendered_into() {
        // The server validates it before it gets here; this is the second layer,
        // so a widening of that check cannot become an injection in one step.
        let out = request_code(Some(r#"/x" onmouseover="alert(1)"#), None).into_string();
        assert!(!out.contains(r#"onmouseover="alert(1)""#), "{out}");
        assert!(out.contains("&quot;"), "{out}");
    }
}
