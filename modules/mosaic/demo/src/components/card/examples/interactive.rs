use leptos::prelude::*;
use mosaic_tiles::card::*;

#[component]
pub fn InteractiveExample() -> impl IntoView {
    view! {
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
    }
}
