use leptos::prelude::*;
use leptos::{component, view, IntoView};

use crate::components::project_detail::attributions_section::AttributionsSection;
use crate::components::project_detail::dataset_overview_section::DatasetOverviewSection;
use crate::domain::{Attribution, Project};

#[component]
pub fn ProjectDetailsTabs(proj: Project, attributions: Vec<Attribution>) -> impl IntoView {
    view! {
    <div class="tabs tabs-border">
      <input type="radio" name="my_tabs_2" class="tab" aria-label="Dataset Overview" checked="checked" />
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <DatasetOverviewSection proj=proj />
      </div>

      <input type="radio" name="my_tabs_2" class="tab" aria-label="Data" />
      <div class="tab-content border-base-300 bg-base-100 p-4">Tab content 2</div>

      <input type="radio" name="my_tabs_2" class="tab" aria-label="Contributors" />
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <AttributionsSection attributions=attributions />
      </div>
    </div>    }
}
