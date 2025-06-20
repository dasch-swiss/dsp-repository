// TODO: work in progress
// TODO: add accessibility features (navigation landmarks, keyboard support)
use maud::{html, Markup};

pub struct ShellNav {}

pub fn shell() -> Markup {
    html! {
        header .dsp-shell-header role="banner" {
            div .dsp-shell-header__header-left {
                a href="/" {
                    img .dsp-shell-header__logo src="/assets/logo.png" alt="DaSCH Logo";
                }
                div .dsp-shell-header__divider {}
                (shell_nav())
            }
            div .dsp-shell-header__header-actions {
                // TODO: get search bar to work
                (search_icon_button())
                (theme_toggle())
            }
        }
    }
}

fn shell_nav() -> Markup {
    html! {
        // TODO: implement
        div {"placeholder"}
    }
}

fn search_icon_button() -> Markup {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" {
            "🔍"
        }
    }
}

fn theme_toggle() -> Markup {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" {
            "🌙"
        }
    }
}
