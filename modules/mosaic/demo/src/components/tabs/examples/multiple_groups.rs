use leptos::prelude::*;
use mosaic_tiles::tabs::*;

#[component]
pub fn MultipleGroupsExample() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <div>
                <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">"Navigation Tabs"</h4>
                <Tabs>
                    <Tab
                        name="nav-tabs"
                        value="dashboard"
                        label="Dashboard"
                        checked=true
                    >
                        <p>"Dashboard content - Overview of your account and activities."</p>
                    </Tab>
                    <Tab
                        name="nav-tabs"
                        value="projects"
                        label="Projects"
                    >
                        <p>"Projects content - List of all your projects and their status."</p>
                    </Tab>
                    <Tab
                        name="nav-tabs"
                        value="settings"
                        label="Settings"
                    >
                        <p>"Settings content - Configure your preferences and account settings."</p>
                    </Tab>
                </Tabs>
            </div>

            <div>
                <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">"Content Tabs"</h4>
                <Tabs>
                    <Tab
                        name="content-tabs"
                        value="overview"
                        label="Overview"
                        checked=true
                    >
                        <p>"Overview section - High-level summary of the content."</p>
                    </Tab>
                    <Tab
                        name="content-tabs"
                        value="details"
                        label="Details"
                    >
                        <p>"Details section - In-depth information and specifications."</p>
                    </Tab>
                </Tabs>
            </div>
        </div>
    }
}
