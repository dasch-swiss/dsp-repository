use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::icon::{AppStore, Icon, List};

use crate::domain::ProjectQuery;

#[component]
pub fn ProjectViewToggle() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let current_view = current_query.view();

    // Helper function to build URL with view parameter
    let build_url = |view: bool| {
        let new_query = ProjectQuery {
            view: Some(view),
            search: current_query.search.clone(),
            ongoing: current_query.ongoing,
            finished: current_query.finished,
            page: current_query.page,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let grid_view_url = build_url(true);
    let list_view_url = build_url(false);

    let grid_class = if current_view {
        "btn btn-primary btn-sm"
    } else {
        "btn btn-ghost btn-sm"
    };

    let list_class = if !current_view {
        "btn btn-primary btn-sm"
    } else {
        "btn btn-ghost btn-sm"
    };

    view! {
        <div class="flex gap-1">
            <a href=grid_view_url class=grid_class>
                <Icon icon=AppStore class="w-5 h-5" />
            </a>

            <a href=list_view_url class=list_class>
                <Icon icon=List class="w-5 h-5" />
            </a>
        </div>
    }
}
