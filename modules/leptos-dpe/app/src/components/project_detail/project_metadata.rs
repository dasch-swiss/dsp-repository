use leptos::prelude::*;

#[component]
pub fn ProjectMetadata(
    start_date: String,
    end_date: String,
    data_publication_year: Option<String>,
    url: Vec<String>,
) -> impl IntoView {
    view! {
        <div
            id="project-details"
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
    }
}
