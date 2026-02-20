use crate::components::project_detail::attributions_section::AttributionsSection;
use crate::components::project_detail::dataset_overview_section::DatasetOverviewSection;
use crate::domain::{Attribution, Project};
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use mosaic_tiles::icon::{Icon, Search};

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    view! {
        <div class="tabs tabs-lift">
            <label class="tab">
                <Icon icon=Search class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" checked="checked" />
                Dataset Overview
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <DatasetOverviewSection proj=proj />
            </div>

            <label class="tab">
                <Icon icon=Search class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" />
                Data
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">Tab content 2</div>

            <label class="tab">
                <Icon icon=Search class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" />
                Contributors
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <AttributionsSection attributions=attributions />
            </div>
        </div>
    }
}
