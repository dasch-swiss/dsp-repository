// Example usage of the Tabs component
use mosaic_tiles::tabs::*;
use mosaic_tiles::icon::*;

#[component]
pub fn TabsExample() -> impl IntoView {
    view! {
        <Tabs>
            <Tab
                name="my-tabs"
                value="home"
                label="Home"
                icon=IconChevronUp
                checked=true
            >
                <div>
                    <h2>"Home Content"</h2>
                    <p>"This is the home tab content."</p>
                </div>
            </Tab>

            <Tab
                name="my-tabs"
                value="search"
                label="Search"
                icon=IconSearch
            >
                <div>
                    <h2>"Search Content"</h2>
                    <p>"This is the search tab content."</p>
                </div>
            </Tab>

            <Tab
                name="my-tabs"
                value="profile"
                label="Profile"
                icon=IconGitHub
            >
                <div>
                    <h2>"Profile Content"</h2>
                    <p>"This is the profile tab content."</p>
                </div>
            </Tab>
        </Tabs>
    }
}

// Example without icons
#[component]
pub fn SimpleTabsExample() -> impl IntoView {
    view! {
        <Tabs>
            <Tab
                name="simple-tabs"
                value="tab1"
                label="First Tab"
                checked=true
            >
                <p>"Content of first tab"</p>
            </Tab>

            <Tab
                name="simple-tabs"
                value="tab2"
                label="Second Tab"
            >
                <p>"Content of second tab"</p>
            </Tab>

            <Tab
                name="simple-tabs"
                value="tab3"
                label="Third Tab"
            >
                <p>"Content of third tab"</p>
            </Tab>
        </Tabs>
    }
}
