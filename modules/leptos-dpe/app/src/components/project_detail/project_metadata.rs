use leptos::prelude::*;

use crate::components::project_detail::publication_year::PublicationYear;
use crate::domain::models::AuthorityFileReference;

#[component]
pub fn ProjectMetadata(
    data_publication_year: Option<String>,
    url: Option<AuthorityFileReference>,
    secondary_url: Option<AuthorityFileReference>,
) -> impl IntoView {
    view! {
        <div
            id="project-details"
        >
            <h3 class="text-base font-semibold mb-3">"Project Details"</h3>
            <div class="space-y-2">
                <PublicationYear year=data_publication_year />
                <div class="flex flex-wrap gap-2">
                    {url.map(|u| {
                        let label = u.text.clone().unwrap_or_else(|| u.url.clone());
                        view! {
                            <a href=u.url class="btn btn-primary btn-sm">{label}</a>
                        }
                    })}
                    {secondary_url.map(|u| {
                        let label = u.text.clone().unwrap_or_else(|| u.url.clone());
                        view! {
                            <a href=u.url class="btn btn-outline btn-sm">{label}</a>
                        }
                    })}
                </div>
            </div>
        </div>
    }
}
