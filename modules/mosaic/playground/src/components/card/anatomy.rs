use leptos::prelude::*;
use mosaic_tiles::card::*;

#[component]
pub fn CardAnatomy() -> impl IntoView {
    view! {
        <Card>
            <CardHeader />
            <CardBody />
            <CardFooter />
        </Card>
    }
}
