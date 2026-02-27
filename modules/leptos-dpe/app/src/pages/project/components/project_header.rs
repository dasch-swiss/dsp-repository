use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::icon::{Export, Icon};

use super::description::Description;
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
        <Card variant=CardVariant::Bordered>
            <figure>
                <img
                    class="w-full object-cover"
                    style="height: 200px"
                    src="https://dasch.swiss/projects/0854.webp"
                    alt="Shoes"
                />
            </figure>
            <CardBody>
                <div class="p-8 flex flex-row justify-center">
                    <div class="max-w-3xl space-y-4">
                        <h2 class="font-display text-3xl text-ellipsis">{name}</h2>
                        {(!alternative_names.is_empty())
                            .then(|| {
                                view! {
                                    <p class="text-sm text-gray-600">
                                        <span>"Also known as: "</span>
                                        {alternative_names
                                            .into_iter()
                                            .map(|name| view! { <span>{name}</span> })
                                            .collect_view()}
                                    </p>
                                }
                            })}
                        <div>
                            <Description text=description />
                        </div>

                        <div class="flex gap-4">
                            {url
                                .map(|u| {
                                    let label = u
                                        .text
                                        .clone()
                                        .unwrap_or_else(|| "Text not loaded".to_string());
                                    view! {
                                        <a class="btn btn-primary" href=u.url>
                                            {label}
                                            <Icon icon=Export class="w-5 h-5" />
                                        </a>
                                    }
                                })}
                            {secondary_url
                                .map(|u| {
                                    let label = u
                                        .text
                                        .clone()
                                        .unwrap_or_else(|| "Text not loaded".to_string());
                                    view! {
                                        <a class="btn btn-outline btn-primary" href=u.url>
                                            {label}
                                            <Icon icon=Export class="w-5 h-5" />
                                        </a>
                                    }
                                })}
                        </div>
                    </div>
                </div>
            </CardBody>
        </Card>
    }
}
