use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardVariant};
use mosaic_tiles::icon::{Data, Document, Info, People};
use mosaic_tiles::tabs::{Tab, Tabs};

use crate::domain::{Attribution, Project};
use crate::pages::project::components::attributions_section::AttributionsSection;
use crate::pages::project::components::dataset_overview_section::DatasetOverviewSection;
use crate::pages::project::components::publication_tab::PublicationTab;

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    let abstract_en = proj.abstract_text.as_ref().and_then(|m| m.get("en").cloned());
    let publications = proj.publications.clone();

    view! {
        <Card variant=CardVariant::Bordered class="flex-1 pt-4">
            <Tabs>
                <Tab
                    name="project-tabs"
                    value="overview"
                    label="Dataset Overview"
                    icon=Info
                    checked=true
                >
                    <DatasetOverviewSection proj=proj />
                </Tab>
                <Tab name="project-tabs" value="data" label="Data" icon=Data>
                    "TODO"
                </Tab>
                <Tab name="project-tabs" value="publications" label="Publications" icon=Document>
                    <PublicationTab abstract_en=abstract_en publications=publications />
                </Tab>
                <Tab name="project-tabs" value="contributors" label="Contributors" icon=People>
                    <AttributionsSection attributions=attributions />
                </Tab>
            </Tabs>
        </Card>
    }
}
