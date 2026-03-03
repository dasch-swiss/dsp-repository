use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::link::Link;

#[component]
pub fn LinkListSection(title: String, items: Vec<String>, #[prop(default = false)] as_links: bool) -> impl IntoView {
    view! {
        <Card variant=CardVariant::Bordered>
            <CardBody>
                <h3 class="text-base font-semibold mb-3">{title}</h3>
                <ul class="list-disc list-inside text-sm">
                    {items
                        .iter()
                        .map(|item| {
                            if as_links {
                                let href = item.clone();
                                let text = item.clone();
                                view! {
                                    <li>
                                        <Link href=href>{text}</Link>
                                    </li>
                                }
                                    .into_any()
                            } else {
                                view! { <li>{item.clone()}</li> }.into_any()
                            }
                        })
                        .collect_view()}
                </ul>
            </CardBody>
        </Card>
    }
}
