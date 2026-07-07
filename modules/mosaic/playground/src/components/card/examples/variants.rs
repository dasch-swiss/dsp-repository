use leptos::prelude::*;
use mosaic_tiles::card::*;

#[component]
pub fn VariantsExample() -> impl IntoView {
    view! {
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            // Simple card
            <Card>
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"Simple"</h3>
                    <p class="text-neutral-600">"This is a basic card with default styling."</p>
                </CardBody>
            </Card>

            // Bordered card
            <Card variant=CardVariant::Bordered>
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"Bordered"</h3>
                    <p class="text-neutral-600">"This card has a visible border."</p>
                </CardBody>
            </Card>

            // Elevated card
            <Card variant=CardVariant::Elevated>
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"Elevated"</h3>
                    <p class="text-neutral-600">"This card has a shadow to create elevation."</p>
                </CardBody>
            </Card>

            // AutoHover card
            <Card variant=CardVariant::AutoHover>
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"AutoHover"</h3>
                    <p class="text-neutral-600">
                        "This card starts with a bordered style and smoothly transitions to an elevated appearance when you hover over it."
                    </p>
                </CardBody>
            </Card>
        </div>
    }
}
