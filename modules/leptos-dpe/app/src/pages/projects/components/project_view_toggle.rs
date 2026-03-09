use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{AppStore, Icon, List};
use mosaic_tiles::link::Link;

use crate::domain::{ProjectQuery, ProjectView};

#[component]
pub fn ProjectViewToggle() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let current_view = current_query.view();

    // Helper function to build URL with view parameter
    let build_url = |view: ProjectView| {
        let new_query = ProjectQuery {
            view: Some(view),
            search: current_query.search.clone(),
            ongoing: current_query.ongoing,
            finished: current_query.finished,
            page: current_query.page,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let grid_view_url = build_url(ProjectView::Grid);
    let list_view_url = build_url(ProjectView::List);

    let grid_variant = if current_view == ProjectView::Grid {
        ButtonVariant::Soft
    } else {
        ButtonVariant::Ghost
    };

    let list_variant = if current_view == ProjectView::List {
        ButtonVariant::Soft
    } else {
        ButtonVariant::Ghost
    };

    view! {
        <div class="flex gap-1">
            <Link href=grid_view_url as_button=grid_variant aria_label="Grid view">
                <Icon icon=AppStore class="w-5 h-5" />
            </Link>

            <Link href=list_view_url as_button=list_variant aria_label="List view">
                <Icon icon=List class="w-5 h-5" />
            </Link>
        </div>
    }
}
