use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::card::*;

#[component]
pub fn WithImagesExample() -> impl IntoView {
    view! {
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            // Card with image at top
            <Card variant=CardVariant::Bordered>
                <img
                    src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba?w=400&h=250&fit=crop"
                    alt="Abstract blue and purple gradient"
                    class="w-full h-48 object-cover"
                />
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"Image Card"</h3>
                    <p class="text-neutral-600">
                        "Cards can feature images at the top for visual appeal."
                    </p>
                </CardBody>
            </Card>

            // Card with image and footer
            <Card variant=CardVariant::Elevated>
                <img
                    src="https://images.unsplash.com/photo-1682687220795-796d3f6f7000?w=400&h=250&fit=crop"
                    alt="Colorful abstract art"
                    class="w-full h-48 object-cover"
                />
                <CardBody>
                    <h3 class="text-lg font-semibold mb-2">"Gallery Item"</h3>
                    <p class="text-neutral-600 text-sm">
                        "Perfect for galleries or portfolios with actions."
                    </p>
                </CardBody>
                <CardFooter>
                    <div class="flex gap-2 justify-end">
                        <Button variant=ButtonVariant::Secondary>"Share"</Button>
                        <Button variant=ButtonVariant::Primary>"View"</Button>
                    </div>
                </CardFooter>
            </Card>

            // Card with rounded image and header
            <Card variant=CardVariant::Bordered>
                <img
                    src="https://images.unsplash.com/photo-1682695796497-31a44224d6d6?w=400&h=250&fit=crop"
                    alt="Nature landscape"
                    class="w-full h-48 object-cover rounded-t-lg"
                />
                <CardHeader>
                    <h3 class="text-lg font-semibold">"Featured Article"</h3>
                    <p class="text-sm text-neutral-500">"Published: Jan 22, 2026"</p>
                </CardHeader>
                <CardBody>
                    <p class="text-neutral-600">
                        "Combine images with headers and body content for rich card layouts."
                    </p>
                </CardBody>
            </Card>
        </div>
    }
}
