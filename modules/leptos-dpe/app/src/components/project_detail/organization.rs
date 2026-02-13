use leptos::prelude::*;

use crate::components::UrlBadge;
use crate::domain::get_organization;

#[component]
pub fn Organization(organization_id: String) -> impl IntoView {
    let organization_resource =
        Resource::new(move || organization_id.clone(), |id| async move { get_organization(id).await });

    view! {
        <Suspense>
            {move || {
                let organization_opt = organization_resource
                    .get()
                    .and_then(|result| result.ok())
                    .flatten();
                match organization_opt {
                    Some(org) => {

                        view! {
                            <div class="collapse collapse-arrow">
                                <input type="checkbox" />
                                <div class="collapse-title font-semibold min-h-0 after:start-0 after:end-auto ps-6 pe-0">
                                    {org.name.clone()}
                                </div>
                                <div class="collapse-content ps-6 pe-0 pt-2 pb-0">
                                    <div class="flex flex-col gap-2">
                                        {org
                                            .address
                                            .as_ref()
                                            .map(|addr| {
                                                view! {
                                                    <div class="text-sm text-base-content/70">
                                                        {format!(
                                                            "{}, {} {}",
                                                            addr.street,
                                                            addr.postal_code,
                                                            addr.locality,
                                                        )}
                                                    </div>
                                                }
                                            })} <div class="flex flex-wrap gap-2 pt-2">
                                            <UrlBadge
                                                url=org.url.clone()
                                                url_type="Website".to_string()
                                            />
                                            {org
                                                .email
                                                .as_ref()
                                                .map(|email| {
                                                    view! {
                                                        <UrlBadge
                                                            url=format!("mailto:{}", email)
                                                            url_type="Email".to_string()
                                                        />
                                                    }
                                                })}
                                        </div>
                                        {(!org.same_as.is_empty())
                                            .then(|| {
                                                view! {
                                                    <div class="flex flex-wrap gap-2 mt-2">
                                                        {org
                                                            .same_as
                                                            .into_iter()
                                                            .map(|ref_| {
                                                                view! {
                                                                    <a
                                                                        href=ref_.url.clone()
                                                                        class="badge badge-outline badge-sm"
                                                                        rel="noopener noreferrer"
                                                                    >
                                                                        {ref_.type_.clone()}
                                                                    </a>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </div>
                                                }
                                            })}
                                    </div>
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                    None => {
                        view! {
                            <div class="collapse collapse-arrow">
                                <input type="checkbox" />
                                <div class="collapse-title font-semibold min-h-0 after:start-0 after:end-auto ps-6 pe-0">
                                    <span class="italic text-base-content/70">
                                        "Organization not found"
                                    </span>
                                </div>
                                <div class="collapse-content ps-6 pe-0 pt-2 pb-0">
                                    <span></span>
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                }
            }}
        </Suspense>
    }
}
