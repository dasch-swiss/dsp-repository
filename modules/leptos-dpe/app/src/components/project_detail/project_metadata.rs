use crate::components::project_detail::publication_year::PublicationYear;
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
            <h3 class="text-base font-semibold mb-3">"Project Details"</h3>
            <div class="space-y-2">
                <PublicationYear year=data_publication_year />
                {(!url.is_empty())
                    .then(|| {
                        view! {
                            <div class="text-sm">
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
