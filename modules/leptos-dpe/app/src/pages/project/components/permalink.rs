use leptos::prelude::*;

use super::copy_button::CopyButton;
use super::info_card::InfoCard;

#[component]
pub fn Permalink(permalink: String) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h3 class="dpe-subtitle">"Permalink"</h3>
            <InfoCard>
                <div class="flex items-center justify-between gap-3">
                    <a href=permalink.clone() class="text-primary break-all flex-1">
                        {permalink.clone()}
                    </a>
                    <CopyButton text=permalink />
                </div>
            </InfoCard>
        </div>
    }
}
