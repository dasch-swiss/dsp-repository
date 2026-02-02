use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::card::*;
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
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
                    <p class="text-gray-600">"You have 3 new messages waiting for your review."</p>
                </CardBody>
                <CardFooter>
                    <Button variant=ButtonVariant::Primary>
                        "View Messages" <Icon icon=IconChevronRight class="w-4 h-4 inline ml-1" />
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
    }
}
