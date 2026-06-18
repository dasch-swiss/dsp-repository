use maud::{html, Markup};
use mosaic_tiles::icon::{icon, IconSearch};

/// Keyboard navigation for the search-results dropdown.
/// Uses native JS (not Datastar) because it manipulates DOM classes on
/// server-rendered result items.
const SEARCH_KEYBOARD_NAV: &str = "\
var results = document.getElementById('search-results');\
if (!results) return;\
var items = results.querySelectorAll('a');\
if (items.length === 0) return;\
var active = results.querySelector('.bg-neutral-100');\
var idx = active ? Array.from(items).indexOf(active) : -1;\
if (event.key === 'ArrowDown') {\
  event.preventDefault();\
  idx = Math.min(idx + 1, items.length - 1);\
} else if (event.key === 'ArrowUp') {\
  event.preventDefault();\
  idx = Math.max(idx - 1, 0);\
} else if (event.key === 'Enter' && idx >= 0) {\
  event.preventDefault();\
  window.location.href = items[idx].getAttribute('href');\
  return;\
} else if (event.key === 'Escape') {\
  results.style.display = 'none';\
  return;\
} else {\
  return;\
}\
items.forEach(function(el) {\
  el.classList.remove('bg-neutral-100');\
  el.classList.add('hover:bg-neutral-100');\
});\
if (idx >= 0 && items[idx]) {\
  items[idx].classList.add('bg-neutral-100');\
  items[idx].classList.remove('hover:bg-neutral-100');\
}";

/// Project search input with Datastar-driven autocomplete.
///
/// `data-bind:search` two-way-binds the `search` signal; `data-on:input`
/// (debounced) triggers `@get`. The server reads the `search` signal and
/// returns results as a PatchElements SSE event. Keyboard navigation uses
/// native JS via `onkeydown`. Without JavaScript it falls back to a standard
/// GET form submission.
pub fn project_search_input() -> Markup {
    html! {
        form method="get" action="/dpe/projects" {
            div class="relative flex-1" data-signals="{_focused: false}" {
                label class="flex w-full items-center gap-2 rounded-md border border-neutral-300 bg-white px-3 py-2 focus-within:border-primary-600 focus-within:ring-1 focus-within:ring-primary-600" {
                    (icon(IconSearch, "w-4 h-4 opacity-50 shrink-0"))
                    input type="search" name="search" placeholder="Search projects..." class="grow bg-transparent outline-none"
                          role="combobox" aria-autocomplete="list" aria-controls="search-results"
                          aria-expanded="false"
                          data-attr:aria-expanded="$_focused && $search.length > 0 ? 'true' : 'false'"
                          data-bind:search
                          "data-on:input__debounce.300ms"="@get('/dpe/projects/search')"
                          data-on:focus="$_focused = true"
                          "data-on:blur__debounce.200ms"="$_focused = false"
                          onkeydown=(SEARCH_KEYBOARD_NAV);
                }

                div id="search-results" role="listbox" aria-label="Search results"
                    data-show="$_focused && $search.length > 0"
                    class="absolute top-full left-0 right-0 mt-1 bg-white border border-neutral-200 rounded-lg shadow-lg z-[100] p-2" {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_form_with_get_fallback() {
        let out = project_search_input().into_string();
        assert!(out.contains(r#"<form method="get" action="/dpe/projects">"#), "{out}");
        assert!(out.contains(r#"name="search""#), "{out}");
        assert!(out.contains(r#"role="combobox""#), "{out}");
    }

    #[test]
    fn preserves_datastar_bindings_verbatim() {
        let out = project_search_input().into_string();
        assert!(out.contains("data-bind:search"), "{out}");
        // Dotted attribute name survives as a literal name (not an escaped value).
        assert!(
            out.contains(r#"data-on:input__debounce.300ms="@get('/dpe/projects/search')""#),
            "{out}"
        );
        assert!(out.contains(r#"data-on:focus="$_focused = true""#), "{out}");
        assert!(out.contains("data-on:blur__debounce.200ms"), "{out}");
        assert!(out.contains(r#"data-signals="{_focused: false}""#), "{out}");
    }

    #[test]
    fn renders_results_listbox() {
        let out = project_search_input().into_string();
        assert!(out.contains(r#"id="search-results""#), "{out}");
        assert!(out.contains(r#"role="listbox""#), "{out}");
        // Maud HTML-escapes literal attribute values (`&`→`&amp;`, `>`→`&gt;`);
        // the browser decodes them back when Datastar reads the expression.
        assert!(
            out.contains(r#"data-show="$_focused &amp;&amp; $search.length &gt; 0""#),
            "{out}"
        );
    }
}
