use leptos::prelude::*;
#[component]
pub fn UrlBadge(
    url: String,
    #[prop(default = "URL".to_string())] url_type: String,
) -> impl IntoView {
    let tooltip_text = url.strip_prefix("mailto:").unwrap_or(&url).to_string();

    view! {
        <a href=url.clone() class="badge badge-outline break-all tooltip" data-tip=tooltip_text>
            {url_type.clone()}
        </a>
    }
}
