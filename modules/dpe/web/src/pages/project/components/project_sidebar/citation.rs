use leptos::prelude::*;

use super::super::copy_button::CopyButton;
use super::super::info_card::InfoCard;

#[component]
pub fn Citation(citation: String) -> impl IntoView {
    view! {
        <h3 class="dpe-subtitle">"Citation"</h3>
        <InfoCard>
            <div class="flex items-center">
                <div class="flex-1">{citation.clone()}</div>
                <CopyButton text=citation />
            </div>
        </InfoCard>
    }
}
