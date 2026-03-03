use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn RichContentExample() -> impl IntoView {
    view! {
        <Accordion>
            <AccordionItem title="Features".to_string()>
                <div class="text-neutral-700 space-y-2">
                    <p class="font-semibold">"Key Features:"</p>
                    <ul class="list-disc list-inside ml-4 space-y-1">
                        <li>"Fine-grained reactivity"</li>
                        <li>"Server-side rendering"</li>
                        <li>"Hydration"</li>
                        <li>"Routing"</li>
                        <li>"Full-stack capabilities"</li>
                    </ul>
                </div>
            </AccordionItem>
            <AccordionItem title="Performance".to_string()>
                <div class="text-neutral-700 space-y-2">
                    <p>"Leptos is designed for exceptional performance:"</p>
                    <div class="bg-neutral-50 p-4 rounded-md mt-2">
                        <p class="font-mono text-sm">"Bundle size: ~50KB minified"</p>
                        <p class="font-mono text-sm">"Initial load: < 100ms"</p>
                        <p class="font-mono text-sm">"Reactivity overhead: ~1μs"</p>
                    </div>
                </div>
            </AccordionItem>
            <AccordionItem title="Community".to_string()>
                <div class="text-neutral-700">
                    <p class="mb-3">"Join the growing Leptos community:"</p>
                    <div class="flex flex-wrap gap-2">
                        <span class="px-3 py-1 bg-primary-100 text-primary-800 rounded-md text-sm">
                            "Discord"
                        </span>
                        <span class="px-3 py-1 bg-primary-100 text-primary-800 rounded-md text-sm">
                            "GitHub"
                        </span>
                        <span class="px-3 py-1 bg-primary-100 text-primary-800 rounded-md text-sm">
                            "Reddit"
                        </span>
                        <span class="px-3 py-1 bg-primary-100 text-primary-800 rounded-md text-sm">
                            "Twitter"
                        </span>
                    </div>
                </div>
            </AccordionItem>
        </Accordion>
    }
}
