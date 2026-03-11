mod attributions_section;
mod dataset_overview_section;
mod publication_tab;

use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardVariant};
use mosaic_tiles::icon::{Document, Info, People};
use mosaic_tiles::tabs::{Tab, Tabs};

use attributions_section::AttributionsSection;
use crate::domain::{lang_value, Attribution, Project};
use dataset_overview_section::DatasetOverviewSection;
use publication_tab::PublicationTab;

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    let abstract_en = proj.abstract_text.as_ref().and_then(|m| lang_value(m).cloned());
    let publications = proj.publications.clone();
    let has_publications_tab = abstract_en.is_some() || publications.as_ref().map(|p| !p.is_empty()).unwrap_or(false);

    view! {
        <Card variant=CardVariant::Bordered class="dpe-card flex-1 pt-4">
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
                {has_publications_tab
                    .then(|| {
                        view! {
                            <Tab
                                name="project-tabs"
                                value="publications"
                                label="Publications"
                                icon=Document
                            >
                                <PublicationTab abstract_en=abstract_en publications=publications />
                            </Tab>
                        }
                    })}
                <Tab name="project-tabs" value="contributors" label="Contributors" icon=People>
                    <AttributionsSection attributions=attributions />
                </Tab>
            </Tabs>
        </Card>
    }
}
