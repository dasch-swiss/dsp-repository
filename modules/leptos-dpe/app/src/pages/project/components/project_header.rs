use leptos::prelude::*;
use mosaic_tiles::icon::{Export, Icon};

use crate::domain::models::AuthorityFileReference;

#[component]
pub fn ProjectHeader(
    name: String,
    description: String,
    alternative_names: Vec<String>,
    url: Option<AuthorityFileReference>,
    secondary_url: Option<AuthorityFileReference>,
) -> impl IntoView {
    view! {
        <div class="card border border-gray-200">
            <figure>
                <img
                    class="w-full object-cover"
                    style="height: 200px"
                    src="https://dasch.swiss/projects/0854.webp"
                    alt="Shoes"
                />
            </figure>
            <div class="card-body">
                <h2 class="card-title text-3xl text-ellipsis">{name}</h2>
                <p class="text-lg mt-4">{description}</p>
                {(!alternative_names.is_empty()).then(|| view! {
                    <p class="text-sm text-gray-600">
                        <span>"Also known as: "</span>
                        {alternative_names.into_iter().map(|name| view! { <span>{name}</span> }).collect_view()}
                    </p>
                })}
                <div class="flex gap-4">
                    {url.map(|u| {
                        let label = u.text.clone().unwrap_or_else(|| u.url.clone());
                        view! {
                            <a class="btn btn-primary" href=u.url>
                                {label}
                                <Icon icon=Export class="w-5 h-5" />
                            </a>
                        }
                    })}
                    {secondary_url.map(|u| {
                        let label = u.text.clone().unwrap_or_else(|| u.url.clone());
                        view! {
                            <a class="btn btn-secondary" href=u.url>
                                {label}
                                <Icon icon=Export class="w-5 h-5" />
                            </a>
                        }
                    })}
                </div>
            </div>
        </div>
    }
}
