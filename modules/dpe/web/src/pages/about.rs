use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconArrowLeft};
use mosaic_tiles::link::Link;

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <a class="inline-flex items-center gap-2 text-sm text-primary mb-6" href="/">
            <Icon icon=IconArrowLeft class="w-3 h-3" />
            Back to Projects
        </a>

        <div class="bg-white border border-gray-200 rounded-lg p-8">
            <h1 class="font-display text-3xl font-bold text-gray-900 mb-6">
                "Help & Documentation"
            </h1>
            <div class="space-y-6">
                <section>
                    <h2 class="font-display text-xl font-semibold text-gray-900 mb-3">
                        "About the Metadata Browser"
                    </h2>
                    <p class="text-gray-700 leading-relaxed">
                        "The DaSCH Metadata Browser provides access to comprehensive metadata about humanities research projects archived by DaSCH (Data and Service Center for the Humanities). Browse projects, collections, and clusters to discover research data across various disciplines, time periods, and institutions."
                    </p>
                </section>
                <section>
                    <h2 class="font-display text-xl font-semibold text-gray-900 mb-3">
                        "Searching & Filtering"
                    </h2>
                    <p class="text-gray-700 leading-relaxed mb-3">
                        "Use the search bar to find projects by name, description, or keywords. Combine search with filters to narrow down results:"
                    </p>
                    <ul class="list-disc list-inside space-y-2 text-gray-700 ml-4">
                        <li>"Filter by discipline, time period, or geographic region"</li>
                        <li>"Filter by access rights to find open access projects"</li>
                        <li>"Filter by project status (finished or ongoing)"</li>
                        <li>"Multiple filters within a category use OR logic"</li>
                        <li>"Filters across categories use AND logic"</li>
                    </ul>
                </section>
                <section>
                    <h2 class="font-display text-xl font-semibold text-gray-900 mb-3">
                        "Understanding Access Rights"
                    </h2>
                    <p class="text-gray-700 leading-relaxed mb-3">
                        "Projects are marked with color-coded badges indicating their access level:"
                    </p>
                    <ul class="list-disc list-inside space-y-2 text-gray-700 ml-4">
                        <li>
                            <span class="font-medium text-green-700">"Full Open Access"</span>
                            " - Data is freely available to everyone"
                        </li>
                        <li>
                            <span class="font-medium text-yellow-700">
                                "Open Access with Restrictions"
                            </span>
                            " - Some access limitations apply"
                        </li>
                        <li>
                            <span class="font-medium text-gray-700">"Embargoed Access"</span>
                            " - Data will become available after embargo period"
                        </li>
                        <li>
                            <span class="font-medium text-gray-700">"Metadata only Access"</span>
                            " - Only metadata is publicly available"
                        </li>
                    </ul>
                </section>
                <section>
                    <h2 class="font-display text-xl font-semibold text-gray-900 mb-3">
                        "Need More Help?"
                    </h2>
                    <p class="text-gray-700 leading-relaxed">
                        "For questions about specific projects, data access, or depositing your own data at DaSCH, please visit "
                        <Link href="https://dasch.swiss" target="_blank" rel="noopener noreferrer">
                            "dasch.swiss"
                        </Link> " or contact the DaSCH team directly."
                    </p>
                </section>
            </div>
        </div>
    }
}
