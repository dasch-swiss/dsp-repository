use leptos::prelude::*;
use leptos::{component, view, IntoView};
use mosaic_tiles::icon::{Data, Document, Icon, Info, People};

use crate::components::project_detail::attributions_section::AttributionsSection;
use crate::components::project_detail::dataset_overview_section::DatasetOverviewSection;
use crate::components::project_detail::lang_utils::lang_map_to_views;
use crate::components::project_detail::publication_tab::PublicationTab;
use crate::domain::{Attribution, Project};

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    let abstracts = lang_map_to_views(&proj.abstract_text.clone().unwrap_or_default());
    let publications = proj.publications.clone();

    view! {
        <div class="tabs tabs-lift">
            <label class="tab">
                <Icon icon=Info class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" checked="checked" />
                Dataset Overview
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <DatasetOverviewSection proj=proj />
            </div>

            <label class="tab">
                <Icon icon=Data class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" />
                Data
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">Tab content 2</div>

        <label class="tab">
                <Icon icon=Document class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" />
                Publications
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <PublicationTab abstracts=abstracts publications=publications />
            </div>


            <label class="tab">
                <Icon icon=People class="h-6 text-neutral-400" />

                <input type="radio" name="my_tabs" />
                Contributors
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <AttributionsSection attributions=attributions />
            </div>
        </div>
    }
}
