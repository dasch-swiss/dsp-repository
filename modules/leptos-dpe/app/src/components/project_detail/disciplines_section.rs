use leptos::prelude::*;

use crate::domain::Discipline;

#[component]
pub fn DisciplinesSection(disciplines: Vec<Discipline>) -> impl IntoView {
    (!disciplines.is_empty()).then(|| {
        view! {
            <div
                id="disciplines"
                class="bg-base-100 p-6 rounded-lg scroll-mt-52"
            >
                <h3 class="text-xl font-bold mb-3">"Disciplines"</h3>
                <ul class="list-disc list-inside space-y-1">
                    {disciplines
                        .iter()
                        .map(|d| match d {
                            Discipline::Text(map) => {
                                view! {
                                    <li>
                                        <a
                                            href="http://www.snf.ch/SiteCollectionDocuments/allg_disziplinenliste.pdf"
                                            class="link link-primary"
                                        >
                                            {map
                                                .iter()
                                                .map(|(lang, text)| {
                                                    format!("{} ({})", text, lang)
                                                })
                                                .collect::<Vec<_>>()
                                                .join(" / ")}
                                        </a>
                                    </li>
                                }
                                    .into_any()
                            }
                            Discipline::Reference(ref_) => {
                                view! {
                                    <li>
                                        <a
                                            href=ref_.url.clone()
                                            class="link link-primary"
                                        >
                                            {ref_
                                                .text
                                                .clone()
                                                .unwrap_or_else(|| ref_.url.clone())}
                                        </a>
                                        <span class="text-sm text-base-content/70 ml-2">
                                            "(" {ref_.type_.clone()} ")"
                                        </span>
                                    </li>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()}
                </ul>
            </div>
        }
        .into_any()
    })
}
