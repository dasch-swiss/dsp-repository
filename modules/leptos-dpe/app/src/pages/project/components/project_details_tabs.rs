use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};
use mosaic_tiles::icon::{Data, Document, Icon, Info, People};

use crate::domain::{Attribution, Project};
use crate::pages::project::components::attributions_section::AttributionsSection;
use crate::pages::project::components::dataset_overview_section::DatasetOverviewSection;
use crate::pages::project::components::publication_tab::PublicationTab;

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    let abstract_en = proj.abstract_text.as_ref().and_then(|m| m.get("en").cloned());
    let publications = proj.publications.clone();

    let tab_label_class = "tab gap-1 px-4 pb-2";
    let tab_content_class = "border-base-300 tab-content rounded-none border-0 border-t p-6";
    let icon_class = "h-6 text-neutral-400";

    view! {
        <Card variant=CardVariant::Bordered class="flex-1">
            <CardBody>
                <div class="tabs tabs-border pt-4">
                    <label class=tab_label_class>
                        <Icon icon=Info class=icon_class />

                        <input type="radio" name="my_tabs" checked="checked" />
                        Dataset Overview
                    </label>
                    <div class=tab_content_class>
                        <DatasetOverviewSection proj=proj />
                    </div>

                    <label class=tab_label_class>
                        <Icon icon=Data class=icon_class />

                        <input type="radio" name="my_tabs" />
                        Data
                    </label>
                    <div class=tab_content_class>TODO</div>

                    <label class=tab_label_class>
                        <Icon icon=Document class=icon_class />

                        <input type="radio" name="my_tabs" />
                        Publications
                    </label>
                    <div class=tab_content_class>
                        <PublicationTab abstract_en=abstract_en publications=publications />
                    </div>

                    <label class=tab_label_class>
                        <Icon icon=People class=icon_class />

                        <input type="radio" name="my_tabs" />
                        Contributors
                    </label>
                    <div class=tab_content_class>
                        <AttributionsSection attributions=attributions />
                    </div>
                </div>
            </CardBody>
        </Card>
    }
}
