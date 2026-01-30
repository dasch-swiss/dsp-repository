use leptos::prelude::*;
use mosaic_tiles::tabs::*;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <Tabs>
            <Tab name="basic-tabs" value="home" label="Home" checked=true>
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold">"Home"</h3>
                    <p>
                        "Welcome to the home tab. This is a simple tab example with text content."
                    </p>
                </div>
            </Tab>
            <Tab name="basic-tabs" value="about" label="About">
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold">"About"</h3>
                    <p>"This is the about tab. Each tab can contain any HTML content."</p>
                </div>
            </Tab>
            <Tab name="basic-tabs" value="contact" label="Contact">
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold">"Contact"</h3>
                    <p>
                        "Get in touch through the contact tab. Tab switching is done purely with CSS."
                    </p>
                </div>
            </Tab>
        </Tabs>
    }
}
