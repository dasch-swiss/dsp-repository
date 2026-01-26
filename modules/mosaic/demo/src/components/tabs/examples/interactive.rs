use leptos::prelude::*;
use mosaic_tiles::tabs::*;

use crate::components::counter::Counter;

#[component]
pub fn InteractiveExample() -> impl IntoView {
    view! {
        <Tabs>
            <Tab name="interactive-tabs" value="counter1" label="Counter 1" checked=true>
                <div class="space-y-4">
                    <Counter />
                </div>
            </Tab>
            <Tab name="interactive-tabs" value="counter2" label="Counter 2">
                <div class="space-y-4">
                    <Counter />
                </div>
            </Tab>
            <Tab name="interactive-tabs" value="counter3" label="Counter 3">
                <div class="space-y-4">
                    <Counter />
                </div>
            </Tab>
        </Tabs>
    }
}
