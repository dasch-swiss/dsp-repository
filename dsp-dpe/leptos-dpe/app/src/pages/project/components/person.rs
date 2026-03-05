use leptos::prelude::*;

use crate::domain::get_person;

#[component]
pub fn Person(person_id: String) -> impl IntoView {
    let person_resource = Resource::new(move || person_id.clone(), |id| async move { get_person(id).await });

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

                        view! { <div class="font-medium">{full_name}</div> }
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
