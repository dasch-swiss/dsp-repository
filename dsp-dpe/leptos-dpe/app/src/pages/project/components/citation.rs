use leptos::prelude::*;

use super::copy_button::CopyButton;
use super::info_card::InfoCard;

#[component]
pub fn Citation(citation: String) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h3 class="dpe-subtitle">"Citation"</h3>
            <InfoCard>
                <div class="flex items-center">
                    <div class="flex-1">{citation.clone()}</div>
                    <CopyButton text=citation />
                </div>
            </InfoCard>
        </div>
    }
}
