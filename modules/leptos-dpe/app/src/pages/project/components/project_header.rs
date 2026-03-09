use leptos::prelude::*;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::icon::{Export, Icon};
use mosaic_tiles::link::Link;

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
                    alt="Alice from Alice in Wonderland walks through a futuristic arched hall covered in glowing binary code toward a doorway labeled \"DasCHland,\" with plants and computer monitors along the sides."
                />
            </figure>
            <CardBody>
                <div class="p-8 flex flex-row justify-center">
                    <div class="max-w-3xl">
                        <h2 class="font-display text-3xl text-ellipsis">{name}</h2>
                        {(!alternative_names.is_empty())
                            .then(|| {
                                view! {
                                    <p class="mt-1 text-sm text-gray-600">
                                        <span>"Also known as: "</span>
                                        {alternative_names
                                            .into_iter()
                                            .map(|name| view! { <span>{name}</span> })
                                            .collect_view()}
                                    </p>
                                }
                            })}
                        <div class="mt-4">
                            <Description text=description />
                        </div>

                        <div class="mt-6 flex gap-4">
                            {url
                                .map(|u| {
                                    let label = u
                                        .text
                                        .clone()
                                        .unwrap_or_else(|| "Discover Project Data".to_string());
                                    view! {
                                        <Link href=u.url as_button=ButtonVariant::Primary>
                                            {label}
                                            <Icon icon=Export class="w-5 h-5" />
                                        </Link>
                                    }
                                })}
                            {secondary_url
                                .map(|u| {
                                    let label = u
                                        .text
                                        .clone()
                                        .unwrap_or_else(|| "External Project Website".to_string());
                                    view! {
                                        <Link href=u.url as_button=ButtonVariant::Outline>
                                            {label}
                                            <Icon icon=Export class="w-5 h-5" />
                                        </Link>
                                    }
                                })}
                        </div>
                    </div>
                </div>
            </CardBody>
        </Card>
    }
}
