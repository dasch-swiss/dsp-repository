use leptos::prelude::*;
use leptos_meta::Style;

use crate::CSS;

#[component]
pub fn ThemeProvider(
    // The children
    children: Children,
) -> impl IntoView {
    view! {
        <Style id="mosaic">{CSS}</Style>
        {children()}
    }
}
