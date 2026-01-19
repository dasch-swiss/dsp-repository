use leptos::prelude::*;
use std::collections::HashMap;

/// Determines if a language code represents a right-to-left (RTL) language
fn is_rtl_language(lang_code: &str) -> bool {
    matches!(
        lang_code.to_lowercase().as_str(),
        "ar" | "he" | "fa" | "ur" | "yi" | "iw" | "arc" | "dv" | "ku" | "ps"
    )
}

/// Returns the sort priority for a language code.
/// Lower numbers appear first.
fn language_sort_priority(lang_code: &str) -> u8 {
    match lang_code.to_lowercase().as_str() {
        "en" => 0,
        "de" => 1,
        "fr" => 2,
        "it" => 4,
        "rm" => 5,
        _ => 4,
    }
}

#[component]
pub fn LanguageTabs(
    /// The title to display above the tabs
    title: String,
    /// HashMap of language code to arbitrary content
    content: HashMap<String, AnyView>,
) -> impl IntoView {
    if content.is_empty() {
        return view! { <span></span> }.into_any();
    }

    let mut items: Vec<(String, AnyView)> = content.into_iter().collect();
    items.sort_by(|a, b| {
        let priority_a = language_sort_priority(&a.0);
        let priority_b = language_sort_priority(&b.0);

        // First sort by priority
        match priority_a.cmp(&priority_b) {
            std::cmp::Ordering::Equal => {
                // If same priority (both are "other" languages), sort alphabetically
                a.0.cmp(&b.0)
            }
            other => other,
        }
    });

    // Generate a unique name for this tab group based on the title
    let tab_group_name = format!("tabs_{}", title.to_lowercase().replace(' ', "_"));

    view! {
        <div class="bg-base-100 p-6 rounded-lg">
            <h3 class="text-xl font-bold mb-3">{title}</h3>
            <div role="tablist" class="tabs tabs-lift">
                {items
                    .into_iter()
                    .enumerate()
                    .map(|(index, (lang, content_view))| {
                        let lang_display = lang.to_uppercase();
                        let is_first = index == 0;
                        let is_rtl = is_rtl_language(&lang);
                        let group_name = tab_group_name.clone();
                        view! {
                            <label class="tab">
                                <input type="radio" name=group_name checked=is_first />
                                {lang_display.clone()}
                            </label>
                            <div
                                class="tab-content bg-base-100 border-base-300 rounded-box p-6"
                                dir=if is_rtl { "rtl" } else { "ltr" }
                            >
                                {content_view}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
    .into_any()
}
