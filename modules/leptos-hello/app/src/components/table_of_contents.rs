use leptos::prelude::*;

#[island]
pub fn TableOfContents() -> impl IntoView {
    let node_ref = NodeRef::<leptos::html::Details>::new();
    let (is_open, set_is_open) = signal(false);

    Effect::new(move || {
        use leptos::prelude::window;
        let is_large_screen = window().inner_width().unwrap().as_f64().unwrap() >= 1024.0;
        set_is_open.set(is_large_screen);
    });

    let close_on_click = move |_| {
        use leptos::prelude::window;
        if window().inner_width().unwrap().as_f64().unwrap() < 1024.0 {
            set_is_open.set(false);
        }
    };

    view! {
        <div class="sticky top-0 z-10 bg-base-100 rounded-lg shadow-md">
            <details
                node_ref=node_ref
                class="collapse collapse-arrow lg:collapse-open"
                prop:open=is_open
            >
                <summary class="collapse-title text-xl font-bold">"Table of Contents"</summary>
                <div class="collapse-content">
                    <nav class="grid md:grid-cols-2 lg:grid-cols-4 gap-2" on:click=close_on_click>
                        <a href="#description" class="link link-primary">
                            "Description"
                        </a>
                        <a href="#project-details" class="link link-primary">
                            "Project Details"
                        </a>
                        <a href="#type-of-data" class="link link-primary">
                            "Type of Data"
                        </a>
                        <a href="#keywords" class="link link-primary">
                            "Keywords"
                        </a>
                        <a href="#disciplines" class="link link-primary">
                            "Disciplines"
                        </a>
                        <a href="#temporal-coverage" class="link link-primary">
                            "Temporal Coverage"
                        </a>
                        <a href="#spatial-coverage" class="link link-primary">
                            "Spatial Coverage"
                        </a>
                        <a href="#abstract" class="link link-primary">
                            "Abstract"
                        </a>
                        <a href="#funding" class="link link-primary">
                            "Funding"
                        </a>
                        <a href="#publications" class="link link-primary">
                            "Publications"
                        </a>
                        <a href="#how-to-cite" class="link link-primary">
                            "How to Cite"
                        </a>
                        <a href="#legal-information" class="link link-primary">
                            "Legal Information"
                        </a>
                        <a href="#access-rights" class="link link-primary">
                            "Access Rights"
                        </a>
                        <a href="#data-languages" class="link link-primary">
                            "Data Languages"
                        </a>
                        <a href="#alternative-names" class="link link-primary">
                            "Alternative Names"
                        </a>
                        <a href="#attributions" class="link link-primary">
                            "Attributions"
                        </a>
                    </nav>
                </div>
            </details>
        </div>
    }
}
