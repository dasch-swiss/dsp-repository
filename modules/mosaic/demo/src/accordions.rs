use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn AccordionExamples() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <section>
                <h2 class="text-2xl font-bold mb-4">"Accordion Component Examples"</h2>

                <div class="space-y-6">
                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Basic Accordion"</h3>
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
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Accordion with Default Open Item"</h3>
                        <Accordion>
                            <AccordionItem title="Getting Started".to_string() default_open=true>
                                <p class="text-gray-700 mb-2">
                                    "To get started with Leptos, you'll need to have Rust installed. Follow these steps:"
                                </p>
                                <ol class="list-decimal list-inside text-gray-700 space-y-1 ml-4">
                                    <li>"Install Rust from rustup.rs"</li>
                                    <li>"Install cargo-leptos: cargo install cargo-leptos"</li>
                                    <li>"Create a new project: cargo leptos new my-app"</li>
                                    <li>"Run the development server: cargo leptos watch"</li>
                                </ol>
                            </AccordionItem>
                            <AccordionItem title="Project Structure".to_string()>
                                <p class="text-gray-700">
                                    "A typical Leptos project includes src/app.rs for components, src/lib.rs for shared code, and Cargo.toml for dependencies."
                                </p>
                            </AccordionItem>
                            <AccordionItem title="Deployment".to_string()>
                                <p class="text-gray-700">
                                    "Leptos apps can be deployed to various platforms including traditional servers, serverless functions, and static hosting."
                                </p>
                            </AccordionItem>
                        </Accordion>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Accordion with Rich Content"</h3>
                        <Accordion>
                            <AccordionItem title="Features".to_string()>
                                <div class="text-gray-700 space-y-2">
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
                                <div class="text-gray-700 space-y-2">
                                    <p>"Leptos is designed for exceptional performance:"</p>
                                    <div class="bg-gray-50 p-4 rounded-md mt-2">
                                        <p class="font-mono text-sm">"Bundle size: ~50KB minified"</p>
                                        <p class="font-mono text-sm">"Initial load: < 100ms"</p>
                                        <p class="font-mono text-sm">"Reactivity overhead: ~1μs"</p>
                                    </div>
                                </div>
                            </AccordionItem>
                            <AccordionItem title="Community".to_string()>
                                <div class="text-gray-700">
                                    <p class="mb-3">"Join the growing Leptos community:"</p>
                                    <div class="flex flex-wrap gap-2">
                                        <span class="px-3 py-1 bg-indigo-100 text-indigo-800 rounded-md text-sm">"Discord"</span>
                                        <span class="px-3 py-1 bg-indigo-100 text-indigo-800 rounded-md text-sm">"GitHub"</span>
                                        <span class="px-3 py-1 bg-indigo-100 text-indigo-800 rounded-md text-sm">"Reddit"</span>
                                        <span class="px-3 py-1 bg-indigo-100 text-indigo-800 rounded-md text-sm">"Twitter"</span>
                                    </div>
                                </div>
                            </AccordionItem>
                        </Accordion>
                    </div>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"FAQ Example"</h3>
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
            </section>
        </div>
    }
}
