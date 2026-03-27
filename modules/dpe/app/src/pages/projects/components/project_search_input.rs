use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconSearch};

/// Keyboard navigation for search results dropdown.
/// Uses native JS (not Datastar) because it manipulates DOM classes
/// on server-rendered result items.
const SEARCH_KEYBOARD_NAV: &str = "\
var results = document.getElementById('search-results');\
if (!results) return;\
var items = results.querySelectorAll('a');\
if (items.length === 0) return;\
var active = results.querySelector('.bg-base-200');\
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
  el.classList.remove('bg-base-200');\
  el.classList.add('hover:bg-base-200');\
});\
if (idx >= 0 && items[idx]) {\
  items[idx].classList.add('bg-base-200');\
  items[idx].classList.remove('hover:bg-base-200');\
}";

/// Project search input with Datastar-driven autocomplete.
///
/// Uses `data-bind:search` for two-way binding and `data-on:input` with
/// debounce to trigger `@get`. The server reads the `search` signal via
/// ReadSignals and returns search results as a PatchElements SSE event.
///
/// Keyboard navigation uses native JS via `onkeydown` (not Datastar) since
/// it needs to manipulate DOM classes on server-rendered result items.
///
/// Falls back to a standard form submission without JavaScript.
#[component]
pub fn ProjectSearchInput() -> impl IntoView {
    view! {
        <form method="get" action="/projects">
            <div class="relative flex-1" data-signals="{_focused: false}">
                <label class="input w-full">
                    <Icon icon=IconSearch class="w-4 h-4 opacity-50 shrink-0" />
                    <input
                        type="search"
                        name="search"
                        placeholder="Search projects..."
                        class="grow"
                        role="combobox"
                        aria-autocomplete="list"
                        aria-controls="search-results"
                        aria-expanded="false"
                        data-attr:aria-expanded="$_focused && $search.length > 0 ? 'true' : 'false'"
                        data-bind:search
                        data-on:input__debounce.300ms="@get('/projects/search')"
                        data-on:focus="$_focused = true"
                        data-on:blur__debounce.200ms="$_focused = false"
                        onkeydown=SEARCH_KEYBOARD_NAV
                    />
                </label>

                <div
                    id="search-results"
                    role="listbox"
                    aria-label="Search results"
                    data-show="$_focused && $search.length > 0"
                    class="absolute top-full left-0 right-0 mt-1 bg-base-100 border border-base-300 rounded-box shadow-lg z-[100] p-2"
                ></div>
            </div>
        </form>
    }
}
