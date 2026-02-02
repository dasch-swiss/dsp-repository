use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn FaqExample() -> impl IntoView {
    view! {
        <Accordion>
            <AccordionItem title="How do I install Leptos?".to_string()>
                <p class="text-gray-700">
                    "You can install Leptos by adding it to your Cargo.toml file or using cargo-leptos for a complete development environment."
                </p>
            </AccordionItem>
            <AccordionItem title="Is Leptos production-ready?".to_string()>
                <p class="text-gray-700">
                    "Yes! Leptos is being used in production by various companies and projects. The API is stabilizing with each release."
                </p>
            </AccordionItem>
            <AccordionItem title="Can I use Leptos with existing JavaScript libraries?".to_string()>
                <p class="text-gray-700">
                    "Yes, through wasm-bindgen you can interoperate with JavaScript libraries. Leptos also provides web-sys bindings for DOM APIs."
                </p>
            </AccordionItem>
            <AccordionItem title="What about SEO?".to_string()>
                <p class="text-gray-700">
                    "Leptos supports server-side rendering, which means your content is fully rendered on the server for excellent SEO performance."
                </p>
            </AccordionItem>
            <AccordionItem title="How does Leptos compare to other frameworks?".to_string()>
                <p class="text-gray-700">
                    "Leptos offers Rust's safety and performance with a developer experience similar to React or Solid.js. It's fully type-safe and has excellent compile-time guarantees."
                </p>
            </AccordionItem>
        </Accordion>
    }
}
