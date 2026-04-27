#![recursion_limit = "256"]
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use leptos_router::{path, StaticSegment};
use mosaic_tiles::ThemeProvider;

mod components;
pub mod domain;
pub mod pages;

use components::{Footer, Header};
use pages::{AboutPage, ProjectPage, ProjectsPage};

pub fn shell(options: LeptosOptions, fathom_site_id: Option<String>, traceparent: Option<String>) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                {traceparent.map(|tp| view! { <meta name="traceparent" content=tp /> })}
                // Google Fonts: Lora (display) and Lato (body) for design token typography
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
                <link
                    rel="stylesheet"
                    href="https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,300;0,400;0,700;1,400&family=Lora:ital,wght@0,400;0,600;0,700;1,400&display=swap"
                />
                <AutoReload options=options.clone() />
                <MetaTags />
                {fathom_site_id
                    .map(|site_id| {
                        view! {
                            <script
                                src="https://cdn.usefathom.com/script.js"
                                data-site=site_id
                                data-spa="auto"
                                data-excluded-domains="localhost,repository.dev.dasch.swiss,repository.test.dasch.swiss,repository.stage.dasch.swiss"
                                defer
                            ></script>
                        }
                    })}
            </head>
            <body class="font-body">
                <App />
                <script type="module" src="/vendor/datastar.js"></script>
                <script type="module" src="/telemetry.js"></script>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/dpe.css" />

        // sets the document title
        <Title text="DaSCH Metadata Browser Projects Overview" />

        <ThemeProvider>
            <div class="bg-gray-50 min-h-screen flex flex-col gap-4">
                // content for this welcome page
                <Header />

                <Router>
                    <main class="flex-1 dpe-max-layout-width mx-auto px-4 w-full">
                        <Routes fallback=|| "Page not found.".into_view()>
                            <Route
                                path=StaticSegment("")
                                view=|| view! { <Redirect path="/dpe/projects" /> }
                            />
                            <Route path=path!("dpe/projects") view=ProjectsPage />
                            <Route path=path!("dpe/about") view=AboutPage />
                            <Route path=path!("dpe/projects/:id") view=ProjectPage />
                        </Routes>
                    </main>
                </Router>

                <Footer />
            </div>
        </ThemeProvider>
    }
}
