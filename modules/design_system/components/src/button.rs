use maud::html;

//  TODO: parameterize the button
pub fn button() -> String {
    html! {
        button .dsp-button {
            "Click me!"
        }
    }
    .into_string()
}
