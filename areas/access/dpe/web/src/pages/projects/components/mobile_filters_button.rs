use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Tune};

use super::project_filters::FILTER_PANEL_ID;
use super::project_filters_content::FILTER_FORM_ID;

/// The mobile "Filters" trigger: a submit button of the facet form that adds
/// `dialog=true`, so without JavaScript the page comes back with the panel open. It is a
/// button, not a link, so a crawler finds no `?dialog=true` URL.
pub fn mobile_filters_button(open: bool) -> Markup {
    html! {
        button
            type="submit"
            form=(FILTER_FORM_ID)
            name="dialog"
            value="true"
            aria-controls=(FILTER_PANEL_ID)
            aria-expanded=(if open { "true" } else { "false" })
            class="btn btn-outline flex items-center gap-2 cursor-pointer"
        {
            (icon(Tune, "w-5 h-5"))
            span class="text-sm font-medium" { "Filters" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_a_submit_button_that_opens_the_panel() {
        let out = mobile_filters_button(false).into_string();
        assert!(
            out.contains(r#"<button type="submit" form="project-filters" name="dialog" value="true" aria-controls="project-filters-panel" aria-expanded="false""#),
            "{out}"
        );
        assert!(out.contains("Filters"), "{out}");
        assert!(!out.contains("<a"), "{out}");
        assert!(!out.contains("outline-none"), "{out}");
    }

    #[test]
    fn reflects_an_open_panel() {
        assert!(mobile_filters_button(true).into_string().contains(r#"aria-expanded="true""#));
    }
}
