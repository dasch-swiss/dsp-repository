use leptos::prelude::*;

use crate::domain::Discipline;

#[component]
pub fn DisciplinesSection(disciplines: Vec<Discipline>) -> impl IntoView {
    (!disciplines.is_empty()).then(|| {
        view! {
            <div id="disciplines">
                <h3 class="text-xl font-bold mb-3">"Disciplines"</h3>
                <div class="flex flex-wrap gap-2">
                    {disciplines
                        .iter()
                        .map(|d| {
                            let (label, url) = match d {
                                Discipline::Text(map) => {
                                    let text = map
                                        .get("en")
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            map.values().next().cloned().unwrap_or_default()
                                        });
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
                                Some(href) => view! {
                                    <a href=href class="badge badge-primary cursor-pointer">
                                        {label}
                                    </a>
                                }
                                .into_any(),
                                None => view! {
                                    <span class="badge badge-primary">{label}</span>
                                }
                                .into_any(),
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
        .into_any()
    })
}
