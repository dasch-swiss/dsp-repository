use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use leptos_router::{path, StaticSegment};

mod components;
mod domain;
mod pages;

use components::{Footer, Header};
use pages::{AboutPage, ProjectPage, ProjectsPage};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <AutoReload options=options.clone() />
                    <HydrationScripts options islands=true />
                    <MetaTags />
                </head>
                <body>
                    <App />
                </body>
            </html>
        }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos-dpe.css" />

        // sets the document title
        <Title text="DaSCH Metadata Browser Projects Overview" />

        <div class="bg-gray-50 min-h-screen flex flex-col gap-4">
            // content for this welcome page
            <Header />

            <Router>
                <main class="flex-1 max-w-7xl mx-auto px-4 w-full">
                    <Routes fallback=|| "Page not found.".into_view()>
                        <Route
                            path=StaticSegment("")
                            view=|| view! { <Redirect path="/projects" /> }
                        />
                        <Route path=StaticSegment("projects") view=ProjectsPage />
                        <Route path=StaticSegment("about") view=AboutPage />
                        <Route path=path!("projects/:id") view=ProjectPage />
                    </Routes>
                </main>
            </Router>

            <Footer />
        </div>
    }
}
