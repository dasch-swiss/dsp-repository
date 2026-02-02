use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <Accordion>
            <AccordionItem title="What is Leptos?".to_string()>
                <p class="text-gray-700">
                    "Leptos is a full-stack, isomorphic Rust web framework leveraging fine-grained reactivity to build declarative user interfaces."
                </p>
            </AccordionItem>
            <AccordionItem title="Why use Rust for web development?".to_string()>
                <p class="text-gray-700">
                    "Rust offers memory safety, performance, and a powerful type system. It compiles to WebAssembly for fast client-side performance and provides excellent tooling."
                </p>
            </AccordionItem>
            <AccordionItem title="What is WebAssembly?".to_string()>
                <p class="text-gray-700">
                    "WebAssembly (Wasm) is a binary instruction format that runs in web browsers at near-native speed. It enables languages like Rust to run on the web."
                </p>
            </AccordionItem>
        </Accordion>
    }
}
