use leptos::prelude::*;

use crate::domain::get_organization;

#[component]
pub fn OrganizationName(organization_id: String) -> impl IntoView {
    let organization_resource = Resource::new(
        move || organization_id.clone(),
        |id| async move { get_organization(id).await },
    );

    view! {
        <Suspense fallback=move || {
            view! { <span>"Loading..."</span> }
        }>
            {move || {
                let organization_opt = organization_resource
                    .get()
                    .and_then(|result| result.ok())
                    .flatten();
                match organization_opt {
                    Some(org) => view! { <span>{org.name.clone()}</span> }.into_any(),
                    None => {
                        view! {
                            <span class="italic text-base-content/70">
                                "Organization not found"
                            </span>
                        }
                            .into_any()
                    }
                }
            }}
        </Suspense>
    }
}
