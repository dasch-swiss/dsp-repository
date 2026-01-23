use leptos::prelude::*;
use mosaic_tiles::tabs::*;

#[component]
pub fn TabsAnatomy() -> impl IntoView {
    view! {
        <Tabs>
            <Tab
                name="example-tabs"
                value="tab1"
                label="Tab 1"
                checked=true
            >
                "Content for tab 1"
            </Tab>
            <Tab
                name="example-tabs"
                value="tab2"
                label="Tab 2"
            >
                "Content for tab 2"
            </Tab>
        </Tabs>
    }
}
