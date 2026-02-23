use leptos::prelude::*;
use mosaic_tiles::icon::{Export, Icon};

#[component]
pub fn ProjectHeader(shortcode: String, name: String, description: String) -> impl IntoView {
    view! {
        <div class="card border border-gray-200">
         <figure>
            <img
        class="w-full object-cover"
        style="height: 200px"
                src="https://dasch.swiss/projects/0854.webp"
                alt="Shoes" />
          </figure>
            <div class="card-body">
                <h2 class="card-title text-3xl text-ellipsis">{name}</h2>
            <p class="text-lg mt-4">{description}</p>
        <div class="flex gap-4">
        <a class="btn btn-primary" href=format!("https://app.dasch.swiss/{}", shortcode)>Discover Project Data
                    <Icon icon=Export class="w-5 h-5" />

        </a>

                <a class="btn btn-secondary" href=format!("https://app.dasch.swiss/{}", shortcode)>External Project Website
                    <Icon icon=Export class="w-5 h-5" />

        </a>
        </div>
            </div>

        </div>
    }
}
