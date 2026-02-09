use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use leptos_router::path;
use leptos_router::StaticSegment;

mod components;
mod domain;
mod pages;

use components::{Footer, NavBar, ProjectCard};
use pages::{AboutPage, ProjectPage, ProjectsPage};
use crate::pages::DataBrowserPage;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <script>
                    {r#"
                    // Apply theme from cookie immediately to prevent flicker
                    // This runs synchronously before page renders
                    (function() {
                      const getCookie = (name) => {
                          const value = `; ${document.cookie}`;
                          const parts = value.split(`; ${name}=`);
                          if (parts.length === 2) return parts.pop().split(';').shift();
                      };
                      const theme = getCookie('theme') || 'dark';
                      document.documentElement.setAttribute('data-theme', theme);
                    })();
                    "#}
                </script>
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

        <div class="min-h-screen flex flex-col">
            // content for this welcome page
            <NavBar />

            <Router>
                <main class="flex-1 max-w-7xl mx-auto px-4 md:px-6 w-full">
                    <Routes fallback=|| "Page not found.".into_view()>
                        <Route
                            path=StaticSegment("")
                            view=|| view! { <Redirect path="/projects" /> }
                        />
                        <Route path=StaticSegment("projects") view=ProjectsPage />
                        <Route path=StaticSegment("about") view=AboutPage />
                        <Route path=path!("projects/:id") view=ProjectPage />
                        <Route path=StaticSegment("data-browser") view=DataBrowserPage />
                        <Route path=path!("data-browser/:id") view=DataBrowserPage />
                    </Routes>
                </main>
            </Router>

            <Footer />
        </div>
    }
}
