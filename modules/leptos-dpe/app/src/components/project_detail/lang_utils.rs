use std::collections::HashMap;

use leptos::prelude::*;

/// Convert a `HashMap<String, String>` into `HashMap<String, AnyView>`
/// where each value is wrapped in a paragraph.
pub fn lang_map_to_views(map: &HashMap<String, String>) -> HashMap<String, AnyView> {
    map.iter()
        .map(|(lang, text)| (lang.clone(), view! { <p class="leading-relaxed">{text.clone()}</p> }.into_any()))
        .collect()
}

/// Group a `Vec<HashMap<String, String>>` by language key,
/// collecting all values per language, then render each group as badges.
pub fn group_by_language_as_badges(items: &[HashMap<String, String>]) -> HashMap<String, AnyView> {
    let grouped: HashMap<String, Vec<String>> =
        items
            .iter()
            .flat_map(|map| map.iter())
            .fold(HashMap::new(), |mut acc, (lang, text)| {
                acc.entry(lang.clone()).or_default().push(text.clone());
                acc
            });

    grouped
        .into_iter()
        .map(|(lang, values)| {
            (
                lang,
                view! {
                    <div class="flex flex-wrap gap-2">
                        {values
                            .into_iter()
                            .map(|v| {
                                view! { <span class="badge badge-primary">{v}</span> }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any(),
            )
        })
        .collect()
}

/// Group a `Vec<HashMap<String, String>>` by language key,
/// then render each group's values as paragraphs.
pub fn group_by_language_as_paragraphs(items: &[HashMap<String, String>]) -> HashMap<String, AnyView> {
    let grouped: HashMap<String, Vec<String>> =
        items
            .iter()
            .flat_map(|map| map.iter())
            .fold(HashMap::new(), |mut acc, (lang, text)| {
                acc.entry(lang.clone()).or_default().push(text.clone());
                acc
            });

    grouped
        .into_iter()
        .map(|(lang, values)| {
            (
                lang,
                view! {
                    <div class="space-y-2">
                        {values
                            .into_iter()
                            .map(|name| {
                                view! { <p>{name}</p> }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any(),
            )
        })
        .collect()
}
