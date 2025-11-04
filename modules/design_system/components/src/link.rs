use maud::{html, Markup};

const BASE_CLASSES: &str =
    "text-indigo-900 hover:text-indigo-600 visited:text-indigo-600 font-medium cursor-pointer no-underline dark:text-indigo-400 dark:hover:text-indigo-300 dark:visited:text-indigo-300";

#[derive(Debug, Clone)]
pub enum LinkTarget {
    SelfTarget,
    Blank,
    Parent,
    Top,
}

impl LinkTarget {
    fn target(&self) -> &'static str {
        match self {
            LinkTarget::SelfTarget => "_self",
            LinkTarget::Blank => "_blank",
            LinkTarget::Parent => "_parent",
            LinkTarget::Top => "_top",
        }
    }

    fn rel(&self) -> Option<&'static str> {
        match self {
            // Security: prevent window.opener access and tabnabbing attacks
            LinkTarget::Blank => Some("noopener noreferrer"),
            _ => None,
        }
    }
}

pub fn link(text: impl Into<String>, url: impl Into<String>) -> Markup {
    link_with_target(text, url, LinkTarget::SelfTarget)
}

pub fn link_external(text: impl Into<String>, url: impl Into<String>) -> Markup {
    link_with_target(text, url, LinkTarget::Blank)
}

pub fn link_with_target(text: impl Into<String>, url: impl Into<String>, target: LinkTarget) -> Markup {
    link_with_target_and_testid(text, url, target, None)
}

pub fn link_with_target_and_testid(
    text: impl Into<String>,
    url: impl Into<String>,
    target: LinkTarget,
    custom_test_id: Option<&str>,
) -> Markup {
    let text = text.into();
    let url = url.into();
    let test_id = custom_test_id.unwrap_or("link");
    let rel_attr = target.rel();

    html! {
        @if let Some(rel) = rel_attr {
            a href=(url) target=(target.target()) rel=(rel) class=(BASE_CLASSES) data-testid=(test_id) {
                (text)
            }
        } @else {
            a href=(url) target=(target.target()) class=(BASE_CLASSES) data-testid=(test_id) {
                (text)
            }
        }
    }
}
