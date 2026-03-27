use leptos::prelude::*;
use mosaic_tiles::icon::{CopyPaste, Icon};

/// Copy-to-clipboard button.
///
/// Uses a simple `onclick` handler (not Datastar) since the clipboard API
/// and tooltip state are purely client-side with no server interaction needed.
#[component]
pub fn CopyButton(text: String) -> impl IntoView {
    view! {
        <button
            class="btn btn-ghost px-1 py-0.5 text-xs tooltip tooltip-left flex-shrink-0"
            data-tip="Copy"
            data-copy-text=text
            onclick="
            navigator.clipboard.writeText(this.dataset.copyText);
            this.setAttribute('data-tip', 'Copied!');
            var btn = this;
            setTimeout(function() { btn.setAttribute('data-tip', 'Copy'); }, 2000);
            "
        >
            <Icon icon=CopyPaste class="w-4 h-4" />
        </button>
    }
}
