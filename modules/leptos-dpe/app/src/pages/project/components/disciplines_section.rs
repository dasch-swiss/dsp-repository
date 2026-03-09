use leptos::prelude::*;

use crate::domain::{lang_value, Discipline};

#[component]
pub fn DisciplinesSection(disciplines: Vec<Discipline>) -> impl IntoView {
    (!disciplines.is_empty()).then(|| {
        view! {
            <div>
                <h3 class="text-sm font-semibold text-gray-700 mb-2">"Disciplines"</h3>
                <div class="flex flex-wrap gap-1.5">
                    {disciplines
                        .iter()
                        .map(|d| {
                            let (label, url) = match d {
                                Discipline::Text(map) => {
                                    let text = lang_value(map).cloned().unwrap_or_default();
                                    (text, None)
                                }
                                Discipline::Reference(ref_) => {
                                    let text = ref_
                                        .text
                                        .clone()
                                        .unwrap_or_else(|| ref_.url.clone());
                                    (text, Some(ref_.url.clone()))
                                }
                            };
                            match url {
                                Some(href) => {
                                    view! {
                                        <a href=href>
                                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                                {label}
                                            </span>
                                        </a>
                                    }
                                        .into_any()
                                }
                                None => {
                                    view! {
                                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                            {label}
                                        </span>
                                    }
                                        .into_any()
                                }
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
        .into_any()
    })
}
