use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

#[component]
pub fn InfoCard(children: Children) -> impl IntoView {
    view! {
        <Card variant=CardVariant::Bordered class="w-full bg-gray-50 text-gray-700">
            <CardBody>{children()}</CardBody>
        </Card>
    }
}
