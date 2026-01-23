use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn AsButtonExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/signup" as_button=true>
                "Sign Up"
            </Link>
            <Link href="/login" as_button=true>
                "Log In"
            </Link>
            <Link href="/get-started" as_button=true>
                "Get Started"
            </Link>
        </div>
    }
}
