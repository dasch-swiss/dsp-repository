use leptos::prelude::*;

#[component]
pub fn FilterCheckboxGroup(
    title: &'static str,
    items: Vec<(String, bool, String)>,
) -> impl IntoView {
    view! {
        <div>
            <h5 class="dpe-subtitle">{title}</h5>
            {items.into_iter().map(|(label, checked, href)| {
                view! {
                    <a href=href class="flex items-center gap-2 cursor-pointer hover:opacity-80 py-1">
                        <input
                            type="checkbox"
                            class="checkbox checkbox-sm pointer-events-none"
                            checked=checked
                        />
                        <span class="text-sm">{label}</span>
                    </a>
                }
            }).collect_view()}
        </div>
    }
}
