use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, Tune};

use super::project_filters_content::ProjectFiltersContent;

#[component]
pub fn MobileFiltersButton(
    status_items: Vec<(String, bool, String)>,
    type_of_data_items: Vec<(String, bool, String)>,
    data_language_items: Vec<(String, bool, String)>,
    access_rights_items: Vec<(String, bool, String)>,
    dialog_open: bool,
    open_dialog_href: String,
    close_dialog_href: String,
) -> impl IntoView {
    view! {
        <a href=open_dialog_href class="btn btn-outline flex items-center gap-2 cursor-pointer">
            <Icon icon=Tune class="w-5 h-5" />
            <span class="text-sm font-medium">"Filters"</span>
        </a>

        {if dialog_open {
            let close_href_backdrop = close_dialog_href.clone();
            view! {
                // Backdrop
                <a href=close_href_backdrop class="fixed inset-0 bg-black/40 z-40 lg:hidden"></a>
                // Panel
                <div class="fixed right-0 top-0 bottom-0 w-full md:w-96 bg-white z-50 overflow-y-auto lg:hidden">
                    <div class="relative p-4">
                        <a
                            href=close_dialog_href
                            class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2 cursor-pointer"
                        >
                            "✕"
                        </a>
                        <ProjectFiltersContent
                            status_items=status_items
                            type_of_data_items=type_of_data_items
                            data_language_items=data_language_items
                            access_rights_items=access_rights_items
                            dialog_open=true
                        />
                    </div>
                </div>
            }
                .into_any()
        } else {
            ().into_any()
        }}
    }
}
