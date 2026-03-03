use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn DefaultOpenExample() -> impl IntoView {
    view! {
        <Accordion>
            <AccordionItem title="Getting Started".to_string() default_open=true>
                <p class="text-neutral-700 mb-2">
                    "To get started with Leptos, you'll need to have Rust installed. Follow these steps:"
                </p>
                <ol class="list-decimal list-inside text-neutral-700 space-y-1 ml-4">
                    <li>"Install Rust from rustup.rs"</li>
                    <li>"Install cargo-leptos: cargo install cargo-leptos"</li>
                    <li>"Create a new project: cargo leptos new my-app"</li>
                    <li>"Run the development server: cargo leptos watch"</li>
                </ol>
            </AccordionItem>
            <AccordionItem title="Project Structure".to_string()>
                <p class="text-neutral-700">
                    "A typical Leptos project includes src/app.rs for components, src/lib.rs for shared code, and Cargo.toml for dependencies."
                </p>
            </AccordionItem>
            <AccordionItem title="Deployment".to_string()>
                <p class="text-neutral-700">
                    "Leptos apps can be deployed to various platforms including traditional servers, serverless functions, and static hosting."
                </p>
            </AccordionItem>
        </Accordion>
    }
}
