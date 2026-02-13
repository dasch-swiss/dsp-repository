use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <Title text="Meta DaSCH - About" />
        <div class="min-h-200 py-4">
            <img src="/logo_with_copy.svg" class="w-165 mb-6" />
            <div class="text-2xl font-bold">"DaSCH"</div>
            <div class="text-2xl font-bold">
                "Swiss National Data and Service Center for the Humanities"
            </div>
            <h2 class="text-xl font-bold">"Adresse"</h2>
            <div class="space-y-2">
                <p>"Kornhausgasse 7"<br />"4051 Basel"<br />"Schweiz"</p>
                <p>
                    "E-Mail: "<a href="mailto:info@dasch.swiss" class="link link-primary">
                        "info@dasch.swiss"
                    </a>
                </p>
                <p class="font-semibold">"Vertretungsberechtigte Person(en)"</p>
                <p>"Prof. Dr. Rita Gautschy"</p>
            </div>
        </div>
    }
}
