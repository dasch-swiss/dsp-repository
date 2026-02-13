use leptos::prelude::*;

#[component]
pub fn ProjectMetadata(
    start_date: String,
    end_date: String,
    data_publication_year: Option<String>,
    url: Vec<String>,
    type_of_data: Option<Vec<String>>,
) -> impl IntoView {
    view! {
        <div class="grid md:grid-cols-2 gap-6">
            <div
                id="project-details"
                class="bg-base-100 p-6 rounded-lg scroll-mt-52"
            >
                <h3 class="text-xl font-bold mb-3">"Project Details"</h3>
                <div class="space-y-2">
                    <div>
                        <span class="font-semibold">"Start Date: "</span>
                        {start_date}
                    </div>
                    <div>
                        <span class="font-semibold">"End Date: "</span>
                        {end_date}
                    </div>
                    {data_publication_year
                        .map(|year| {
                            view! {
                                <div>
                                    <span class="font-semibold">"Publication Year: "</span>
                                    {year}
                                </div>
                            }
                        })}
                    {(!url.is_empty())
                        .then(|| {
                            view! {
                                <div>
                                    <span class="font-semibold">"URLs:"</span>
                                    <ul class="list-disc list-inside ml-2">
                                        {url
                                            .iter()
                                            .map(|u| {
                                                view! {
                                                    <li>
                                                        <a
                                                            href=u.clone()
                                                            class="link link-primary"
                                                        >
                                                            {u.clone()}
                                                        </a>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                </div>
                            }
                                .into_any()
                        })}
                </div>
            </div>

            {type_of_data
                .map(|types| {
                    view! {
                        <div
                            id="type-of-data"
                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                        >
                            <h3 class="text-xl font-bold mb-3">"Type of Data"</h3>
                            <div class="flex flex-wrap gap-2">
                                {types
                                    .iter()
                                    .map(|t| {
                                        view! {
                                            <span class="badge badge-primary">
                                                {t.clone()}
                                            </span>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                })}
        </div>
    }
}
