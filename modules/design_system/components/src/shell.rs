use maud::{html, PreEscaped};

pub struct ShellNav {}

pub fn shell() -> String {
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
    .into_string()
}

fn shell_nav() -> PreEscaped<String> {
    html! {
        // TODO: implement
        div {"placeholder"}
    }
}

fn search_icon_button() -> PreEscaped<String> {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" {
            "🔍"
        }
    }
}

fn theme_toggle() -> PreEscaped<String> {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" {
            "🌙"
        }
    }
}
