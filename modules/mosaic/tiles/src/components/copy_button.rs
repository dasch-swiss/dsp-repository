//! Copy-to-clipboard button tile (promoted from DPE).
//!
//! Uses a small inline `onclick` handler (not Datastar): the clipboard API and
//! the tooltip state are purely client-side with no server interaction needed.

use maud::{html, Markup};

use crate::components::icon::{icon, CopyPaste};

/// The inline click handler. Copies `data-copy-text` to the clipboard and
/// flips the tooltip text to confirm.
///
/// Security invariant: this must stay a static literal. Caller input reaches
/// the handler only through the Maud-escaped `data-copy-text` attribute, read
/// at runtime via `this.dataset.copyText` — never interpolate caller content
/// into this string, or it becomes an XSS hole.
const ON_CLICK: &str = "\
try {
navigator.clipboard.writeText(this.dataset.copyText);
this.setAttribute('data-tip', 'Copied!');
var btn = this;
setTimeout(function() { btn.setAttribute('data-tip', 'Copy'); }, 2000);
} catch(e) {
this.setAttribute('data-tip', 'Copy failed');
}";

/// Render a ghost button that copies `text` to the clipboard on click.
pub fn copy_button(text: &str) -> Markup {
    html! {
        button
            class="btn btn-ghost px-1 py-0.5 text-xs tooltip tooltip-left flex-shrink-0"
            aria-label="Copy"
            data-tip="Copy"
            data-copy-text=(text)
            onclick=(ON_CLICK)
        { (icon(CopyPaste, "w-4 h-4")) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_button_with_copy_text_and_icon() {
        let out = copy_button("https://example.org/permalink").into_string();
        assert!(out.contains(r#"data-copy-text="https://example.org/permalink""#), "{out}");
        assert!(out.contains(r#"data-tip="Copy""#), "{out}");
        assert!(out.contains(r#"aria-label="Copy""#), "accessible name missing: {out}");
        assert!(out.contains("navigator.clipboard.writeText"), "onclick handler missing: {out}");
        assert!(out.contains(r#"class="icon w-4 h-4""#), "icon missing: {out}");
    }

    #[test]
    fn escapes_copy_text() {
        let out = copy_button(r#"a"b<c"#).into_string();
        assert!(
            !out.contains(r#"copy-text="a"b<c""#),
            "raw quotes/anglebrackets must be escaped: {out}"
        );
        assert!(out.contains("&quot;") || out.contains("&#34;"), "{out}");
    }
}
