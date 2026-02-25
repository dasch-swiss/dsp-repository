use std::collections::HashMap;

use leptos::prelude::*;

/// Convert a `HashMap<String, String>` into `HashMap<String, AnyView>`
/// where each value is wrapped in a paragraph.
pub fn lang_map_to_views(map: &HashMap<String, String>) -> HashMap<String, AnyView> {
    map.iter()
        .map(|(lang, text)| (lang.clone(), view! { <p class="leading-relaxed">{text.clone()}</p> }.into_any()))
        .collect()
}
