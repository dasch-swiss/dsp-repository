use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::StaticSegment;
use mosaic_tiles::ThemeProvider;

use crate::accordions::AccordionExamples;
use crate::badges::BadgeExamples;
use crate::buttons::ButtonExamples;
use crate::cards::CardExamples;
use crate::counter::Counter;
use crate::icons::IconExamples;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
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
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/mosaic-demo.css" />

        // sets the document title
        <Title text="Welcome to the Mosaic component library demo" />

        // content for this welcome page
        <ThemeProvider>
            <Router>
                <main class="min-h-screen bg-gray-50">
                    <nav class="bg-white border-b border-gray-200">
                        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                            <div class="flex gap-8 h-16 items-center">
                                <A href="/" attr:class="text-gray-700 hover:text-gray-900">
                                    "Home"
                                </A>
                                <A
                                    href="/accordions"
                                    attr:class="text-gray-700 hover:text-gray-900"
                                >
                                    "Accordions"
                                </A>
                                <A href="/badges" attr:class="text-gray-700 hover:text-gray-900">
                                    "Badges"
                                </A>
                                <A href="/buttons" attr:class="text-gray-700 hover:text-gray-900">
                                    "Buttons"
                                </A>
                                <A href="/cards" attr:class="text-gray-700 hover:text-gray-900">
                                    "Cards"
                                </A>
                                <A href="/icons" attr:class="text-gray-700 hover:text-gray-900">
                                    "Icons"
                                </A>
                            </div>
                        </div>
                    </nav>
                    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                        <Routes fallback=|| "Page not found.".into_view()>
                            <Route path=StaticSegment("") view=HomePage />
                            <Route path=StaticSegment("accordions") view=AccordionsPage />
                            <Route path=StaticSegment("badges") view=BadgesPage />
                            <Route path=StaticSegment("buttons") view=ButtonsPage />
                            <Route path=StaticSegment("cards") view=CardsPage />
                            <Route path=StaticSegment("icons") view=IconsPage />
                        </Routes>
                    </div>
                </main>
            </Router>
        </ThemeProvider>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1 class="text-3xl font-bold mb-6">"Welcome to Leptos!"</h1>
        <Counter />
    }
}

/// Renders the accordions demo page.
#[component]
fn AccordionsPage() -> impl IntoView {
    view! { <AccordionExamples /> }
}

/// Renders the badges demo page.
#[component]
fn BadgesPage() -> impl IntoView {
    view! { <BadgeExamples /> }
}

/// Renders the buttons demo page.
#[component]
fn ButtonsPage() -> impl IntoView {
    view! { <ButtonExamples /> }
}

/// Renders the cards demo page.
#[component]
fn CardsPage() -> impl IntoView {
    view! { <CardExamples /> }
}

/// Renders the icons demo page.
#[component]
fn IconsPage() -> impl IntoView {
    view! { <IconExamples /> }
}
