use leptos::prelude::*;

use crate::components::{Organization, UrlBadge};
use crate::domain::get_person;

#[component]
pub fn Person(person_id: String) -> impl IntoView {
    let person_resource = Resource::new(
        move || person_id.clone(),
        |id| async move { get_person(id).await },
    );

    view! {
        <Suspense>
            {move || {
                let person_opt = person_resource.get().and_then(|result| result.ok()).flatten();
                match person_opt {
                    Some(person) => {
                        let full_name = format!(
                            "{} {}",
                            person.given_names.join(" "),
                            person.family_names.join(" "),
                        );

                        view! {
                            <div class="flex flex-col gap-1">
                                <div class="font-semibold">{full_name}</div>
                                {(!person.job_titles.is_empty())
                                    .then(|| {
                                        view! {
                                            <div class="text-sm text-base-content/70">
                                                {person.job_titles.join(", ")}
                                            </div>
                                        }
                                            .into_any()
                                    })}
                                {person
                                    .email
                                    .as_ref()
                                    .map(|email| {
                                        view! {
                                            <a
                                                href=format!("mailto:{}", email)
                                                class="text-sm link link-primary"
                                            >
                                                {email.clone()}
                                            </a>
                                        }
                                    })}
                                {(!person.same_as.is_empty())
                                    .then(|| {
                                        view! {
                                            <div class="flex flex-wrap gap-2 mt-2">
                                                {person
                                                    .same_as
                                                    .into_iter()
                                                    .map(|ref_| {
                                                        view! {
                                                            <UrlBadge
                                                                url=ref_.url.clone()
                                                                url_type=ref_.type_.clone()
                                                            />
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        }
                                            .into_any()
                                    })}
                                {(!person.affiliations.is_empty())
                                    .then(|| {
                                        view! {
                                            <div class="flex flex-col gap-3 mt-2">
                                                {person
                                                    .affiliations
                                                    .into_iter()
                                                    .map(|org_id| {
                                                        view! { <Organization organization_id=org_id /> }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        }
                                            .into_any()
                                    })}
                            </div>
                        }
                            .into_any()
                    }
                    None => {
                        view! { <div class="italic text-base-content/70">"Person not found"</div> }
                            .into_any()
                    }
                }
            }}
        </Suspense>
    }
}
