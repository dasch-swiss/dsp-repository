use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="footer sm:footer-horizontal footer-center text-base-content  bg-base-300  p-10">
            <nav class="flex flex-col md:flex-row gap-4">
                <a class="hover:underline" href="https://dasch.swiss/legal-notice">
                    Legal Notice
                </a>
                <a class="hover:underline" href="https://dasch.swiss/privacy-policy">
                    Privacy Policy
                </a>
                <a class="hover:underline" href="https://dasch.swiss/privacy-policy-en">
                    Privacy Policy (English)
                </a>
                <a class="hover:underline" href="https://dasch.swiss/impressum">
                    Impressum
                </a>
                <a class="hover:underline" href="https://dasch.swiss/contact">
                    Contact
                </a>
            </nav>
        </footer>
    }
}
