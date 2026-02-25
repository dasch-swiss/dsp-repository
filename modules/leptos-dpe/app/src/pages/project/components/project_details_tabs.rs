use leptos::prelude::*;
use leptos::{component, view, IntoView};
use mosaic_tiles::icon::{Data, Document, Icon, Info, People};

use crate::domain::{Attribution, Project};
use crate::pages::project::components::attributions_section::AttributionsSection;
use crate::pages::project::components::dataset_overview_section::DatasetOverviewSection;
use crate::pages::project::components::publication_tab::PublicationTab;

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    let abstract_en = proj.abstract_text.as_ref().and_then(|m| m.get("en").cloned());
    let publications = proj.publications.clone();

    view! {
        <div class="tabs tabs-lift">
            <label class="tab">
                <Icon icon=Info class="h-6 text-neutral-400 mr-2" />

                <input type="radio" name="my_tabs" checked="checked" />
                Dataset Overview
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <DatasetOverviewSection proj=proj />
            </div>

            <label class="tab">
                <Icon icon=Data class="h-6 text-neutral-400 mr-2" />

                <input type="radio" name="my_tabs" />
                Data
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">TODO</div>

        <label class="tab">
                <Icon icon=Document class="h-6 text-neutral-400 mr-2" />

                <input type="radio" name="my_tabs" />
                Publications
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <PublicationTab abstract_en=abstract_en publications=publications />
            </div>


            <label class="tab">
                <Icon icon=People class="h-6 text-neutral-400 mr-2" />

                <input type="radio" name="my_tabs" />
                Contributors
            </label>
            <div class="tab-content border-base-300 bg-base-100 p-4">
                <AttributionsSection attributions=attributions />
            </div>
        </div>
    }
}
