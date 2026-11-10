use maud::{html, Markup};

/// The danger-tint card of a confirmation before something is destroyed, holding what it destroys
/// or, inline on a form, the whole question.
pub fn impact_card(body: Markup) -> Markup {
    html! {
        div class="flex flex-col gap-1 rounded-md border border-danger-200 bg-danger-50 p-4" {
            (body)
        }
    }
}
