// TODO: work in progress
// TODO: add accessibility features (navigation landmarks, keyboard support)
use maud::{html, Markup};

pub struct ShellNav {}

pub fn shell() -> Markup {
    shell_with_testid("shell-header")
}

pub fn shell_with_testid(test_id: &str) -> Markup {
    html! {
        header .dsp-shell-header role="banner" data-testid=(test_id) {
            div .dsp-shell-header__header-left {
                a href="/" data-testid=(format!("{}-logo-link", test_id)) {
                    img .dsp-shell-header__logo src="/assets/logo.png" alt="DaSCH Logo" data-testid=(format!("{}-logo", test_id));
                }
                div .dsp-shell-header__divider {}
                (shell_nav_with_testid(&format!("{}-nav", test_id)))
            }
            div .dsp-shell-header__header-actions data-testid=(format!("{}-actions", test_id)) {
                // TODO: get search bar to work
                (search_icon_button_with_testid(&format!("{}-search", test_id)))
                (theme_toggle_with_testid(&format!("{}-theme", test_id)))
            }
        }
    }
}

fn shell_nav_with_testid(test_id: &str) -> Markup {
    html! {
        // TODO: implement
        div data-testid=(test_id) {"placeholder"}
    }
}

fn search_icon_button_with_testid(test_id: &str) -> Markup {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" data-testid=(test_id) {
            "🔍"
        }
    }
}

fn theme_toggle_with_testid(test_id: &str) -> Markup {
    html! {
        // TODO: implement with icon, and get it to do stuff
        button .dsp-shell-header__action-icon disabled="true" data-testid=(test_id) {
            "🌙"
        }
    }
}
