use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::card::{Card, CardBody, CardFooter, CardHeader, CardVariant};
use mosaic_tiles::icon::*;

#[component]
pub fn CardExamples() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <section>
                <h2 class="text-2xl font-bold mb-4">"Card Component Examples"</h2>

                <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                    // Simple card
                    <Card>
                        <CardBody>
                            <h3 class="text-lg font-semibold mb-2">"Simple Card"</h3>
                            <p class="text-gray-600">
                                "This is a basic card with default styling."
                            </p>
                        </CardBody>
                    </Card>

                    // Bordered card
                    <Card variant=CardVariant::Bordered>
                        <CardBody>
                            <h3 class="text-lg font-semibold mb-2">"Bordered Card"</h3>
                            <p class="text-gray-600">"This card has a visible border."</p>
                        </CardBody>
                    </Card>

                    // Elevated card
                    <Card variant=CardVariant::Elevated>
                        <CardBody>
                            <h3 class="text-lg font-semibold mb-2">"Elevated Card"</h3>
                            <p class="text-gray-600">
                                "This card has a shadow to create elevation."
                            </p>
                        </CardBody>
                    </Card>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Card with Header and Footer"</h3>

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
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Interactive Card"</h3>

                <div class="max-w-md">
                    <Card variant=CardVariant::Elevated>
                        <CardHeader>
                            <h3 class="text-lg font-semibold">"User Profile"</h3>
                        </CardHeader>
                        <CardBody>
                            <div class="space-y-4">
                                <div>
                                    <label class="block text-sm font-medium mb-1">"Name"</label>
                                    <p class="text-gray-700">"Jane Doe"</p>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium mb-1">"Email"</label>
                                    <p class="text-gray-700">"jane@example.com"</p>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium mb-1">"Role"</label>
                                    <p class="text-gray-700">"Administrator"</p>
                                </div>
                            </div>
                        </CardBody>
                        <CardFooter>
                            <div class="text-sm text-gray-500">"Last updated: 2026-01-22"</div>
                        </CardFooter>
                    </Card>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Interactive Hover Card"</h3>

                <div class="max-w-md">
                    <Card variant=CardVariant::AutoHover>
                        <CardHeader>
                            <h3 class="text-lg font-semibold">"Hover Me"</h3>
                            <p class="text-sm text-gray-500">"Transitions on hover"</p>
                        </CardHeader>
                        <CardBody>
                            <p class="text-gray-700">
                                "This card uses the AutoHover variant which starts with a bordered "
                                "style and smoothly transitions to an elevated appearance when you "
                                "hover over it."
                            </p>
                        </CardBody>
                        <CardFooter>
                            <div class="text-sm text-gray-500">
                                "Hover to see the elevation effect"
                            </div>
                        </CardFooter>
                    </Card>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Cards with Images"</h3>

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
                            <p class="text-gray-600">
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
                            <p class="text-gray-600 text-sm">
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
                            <p class="text-sm text-gray-500">"Published: Jan 22, 2026"</p>
                        </CardHeader>
                        <CardBody>
                            <p class="text-gray-600">
                                "Combine images with headers and body content for rich card layouts."
                            </p>
                        </CardBody>
                    </Card>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Cards with Icons"</h3>

                <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                    // Info card with icon
                    <Card variant=CardVariant::Bordered>
                        <CardBody>
                            <div class="flex items-start gap-3">
                                <div class="p-2 bg-blue-100 rounded-lg">
                                    <Icon icon=Info class="w-5 h-5 text-blue-600" />
                                </div>
                                <div class="flex-1">
                                    <h3 class="text-lg font-semibold mb-2">"Information"</h3>
                                    <p class="text-gray-600 text-sm">
                                        "This card uses an icon to indicate informational content."
                                    </p>
                                </div>
                            </div>
                        </CardBody>
                    </Card>

                    // Mail card with icon
                    <Card variant=CardVariant::Elevated>
                        <CardHeader>
                            <div class="flex items-center gap-2">
                                <Icon icon=Mail class="w-5 h-5 text-gray-600" />
                                <h3 class="text-lg font-semibold">"Messages"</h3>
                            </div>
                        </CardHeader>
                        <CardBody>
                            <p class="text-gray-600">
                                "You have 3 new messages waiting for your review."
                            </p>
                        </CardBody>
                        <CardFooter>
                            <Button variant=ButtonVariant::Primary>
                                "View Messages"
                                <Icon icon=IconChevronRight class="w-4 h-4 inline ml-1" />
                            </Button>
                        </CardFooter>
                    </Card>

                    // Action card with external link icon
                    <Card variant=CardVariant::Bordered>
                        <CardBody>
                            <h3 class="text-lg font-semibold mb-2">"Documentation"</h3>
                            <p class="text-gray-600 text-sm mb-4">
                                "Learn more about our API and integration guides."
                            </p>
                            <a
                                href="#"
                                class="inline-flex items-center gap-1 text-indigo-600 hover:text-indigo-700 text-sm font-medium"
                            >
                                "Read the docs"
                                <Icon icon=LinkExternal class="w-4 h-4" />
                            </a>
                        </CardBody>
                    </Card>
                </div>
            </section>
        </div>
    }
}
