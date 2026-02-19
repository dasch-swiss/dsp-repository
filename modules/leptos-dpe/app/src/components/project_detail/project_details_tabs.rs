use leptos::prelude::*;
use leptos::{component, view, IntoView};

use crate::components::project_detail::attributions_section::AttributionsSection;
use crate::domain::Attribution;

#[component]
pub fn ProjectDetailsTabs(attributions: Vec<Attribution>) -> impl IntoView {
    view! {
    <div class="tabs tabs-border">
      <input type="radio" name="my_tabs_2" class="tab" aria-label="Dataset Overview" />
      <div class="tab-content border-base-300 bg-base-100 p-10">Tab content 1</div>

      <input type="radio" name="my_tabs_2" class="tab" aria-label="Data" checked="checked" />
      <div class="tab-content border-base-300 bg-base-100 p-10">Tab content 2</div>

      <input type="radio" name="my_tabs_2" class="tab" aria-label="Contributors" />
      <div class="tab-content border-base-300 bg-base-100 p-10">
        <AttributionsSection attributions=attributions />
      </div>
    </div>    }
}
