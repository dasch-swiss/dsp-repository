use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::card::*;

#[component]
pub fn WithHeaderFooterExample() -> impl IntoView {
    view! {
        <Card variant=CardVariant::Bordered>
            <CardHeader>
                <h3 class="text-lg font-semibold">"Card Title"</h3>
                <p class="text-sm text-gray-500">"Subtitle information"</p>
            </CardHeader>
            <CardBody>
                <p class="text-gray-700">
                    "This card demonstrates header and footer sections. "
                    "Headers are great for titles and subtitles, while footers "
                    "work well for actions or metadata."
                </p>
            </CardBody>
            <CardFooter>
                <div class="flex gap-2 justify-end">
                    <Button variant=ButtonVariant::Secondary>"Cancel"</Button>
                    <Button variant=ButtonVariant::Primary>"Save"</Button>
                </div>
            </CardFooter>
        </Card>
    }
}
