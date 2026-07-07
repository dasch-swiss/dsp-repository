use leptos::prelude::*;

const TRUNCATE_THRESHOLD: usize = 500;

/// Expandable description text using Datastar signals.
///
/// No WASM needed — expand/collapse is handled by Datastar `data-signals`
/// and `data-class` attributes. The `_expanded` signal toggles the CSS
/// `line-clamp-4` class and button text.
#[component]
pub fn Description(text: String) -> impl IntoView {
    let is_long = text.chars().count() > TRUNCATE_THRESHOLD;

    if is_long {
        view! {
            <div data-signals="{_expanded: false}">
                <p class="text-lg text-gray-600" data-class="{'line-clamp-4': !$_expanded}">
                    {text}
                </p>
                <button
                    class="text-primary cursor-pointer mt-2"
                    data-on:click="$_expanded = !$_expanded"
                    data-text="$_expanded ? 'Show less' : 'Show more'"
                >
                    "Show more"
                </button>
            </div>
        }
        .into_any()
    } else {
        view! { <p class="text-lg text-gray-600">{text}</p> }.into_any()
    }
}
