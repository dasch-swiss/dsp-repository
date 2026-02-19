use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn ProjectDetailsTabs() -> impl IntoView {
    view! {
<div class="tabs tabs-border">
  <input type="radio" name="my_tabs_2" class="tab" aria-label="Dataset Overview" />
  <div class="tab-content border-base-300 bg-base-100 p-10">Tab content 1</div>

  <input type="radio" name="my_tabs_2" class="tab" aria-label="Data" checked="checked" />
  <div class="tab-content border-base-300 bg-base-100 p-10">Tab content 2</div>

  <input type="radio" name="my_tabs_2" class="tab" aria-label="Contributors" />
  <div class="tab-content border-base-300 bg-base-100 p-10">Tab content 3</div>
</div>    }
}