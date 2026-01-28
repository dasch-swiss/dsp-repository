use leptos::prelude::*;

use crate::components::ThemeSwitcher;

#[component]
pub fn HeaderLinks() -> impl IntoView {
    view! {
        <ul class="menu menu-horizontal px-1">
            <li>
                <a href="/projects">"Three columns"</a>
                        </li>
            <li>
                <a href="/projects">"Projects"</a>
            </li>
            <li>
                <a href="/about">"About"</a>
            </li>
            <li><ThemeSwitcher /></li>
        </ul>
    }
}
