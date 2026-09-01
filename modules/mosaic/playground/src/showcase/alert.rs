//! Alert showcase.

use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::link::link;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Alert",
        "A bordered, tinted block carrying a message, with an optional bold title. The variant decides both the \
         colour and whether assistive technology is interrupted.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "alert-variants",
                "Variants",
                "Info, Success, Warning and Danger. Only Danger renders role=\"alert\" — see the roles example below.",
                variants(),
            )
        })
        ({
            example(
                "alert-with_title",
                "With a title",
                "A bold title line above the content, for a block that states more than one thing.",
                with_title(),
            )
        })
        ({
            example(
                "alert-with_rich_content",
                "Rich content",
                "The content slot is `impl Render`, so it takes a list, markup, or another tile.",
                with_rich_content(),
            )
        })
        ({
            example(
                "alert-with_link",
                "With a link",
                "The link tile's text-primary-600 clears WCAG 2.1 AA (4.5:1) on all four alert surfaces, so no \
                 dark-surface link variant is needed here.",
                with_link(),
            )
        })
        ({
            example(
                "alert-roles",
                "Announcement",
                "Danger is an assertive live region because it reports a failure the reader has to hear about. \
                 The other three state a consequence and carry no role.",
                roles(),
            )
        })
    }
}

const ALL_VARIANTS: [(AlertVariant, &str); 4] = [
    (AlertVariant::Info, "Drafts are saved automatically as you type."),
    (AlertVariant::Success, "The submission was sent for review."),
    (
        AlertVariant::Warning,
        "This deployment has no mail relay, so codes are shown on screen.",
    ),
    (AlertVariant::Danger, "An account already uses that email address."),
];

fn variants() -> Markup {
    html! {
        div class="flex flex-col gap-3" {
            @for (variant, message) in ALL_VARIANTS { (alert(message).variant(variant)) }
        }
    }
}

fn with_title() -> Markup {
    let body = html! {
        p class="mt-1" {
            "This service has no mail relay and no database that outlives it, so your code is shown here instead \
             of being emailed:"
        }
        p class="font-mono text-2xl tracking-widest mt-2" { "418 526" }
    };
    html! {
        ({
            alert(body)
                .variant(AlertVariant::Warning)
                .title("Development deployment — no mail was sent")
        })
    }
}

fn with_rich_content() -> Markup {
    let body = html! {
        ul class="flex flex-col gap-2 list-disc pl-5" {
            li { "Drafts on 0801, 080C. The work is kept; the last editor becomes unknown." }
            li {
                "A submission awaiting review on 0801. It can still be approved or rejected, but it can no longer \
                 be returned to its depositor for changes."
            }
        }
    };
    html! {
        (alert(body).variant(AlertVariant::Warning).title("What this leaves behind"))
    }
}

fn with_link() -> Markup {
    let body = html! {
        "The shortcode 0801 is not one of yours. "
        (link("See the projects you can edit", "#"))
    };
    html! {
        div class="flex flex-col gap-3" {
            @for (variant, _) in ALL_VARIANTS { (alert(&body).variant(variant)) }
        }
    }
}

fn roles() -> Markup {
    let announced = html! {
        "This one is announced: it carries "
        code class="font-mono" { "role=\"alert\"" }
        "."
    };
    let silent = html! {
        "This one is not: it states a consequence rather than reporting a failure, so it has no role."
    };
    html! {
        div class="flex flex-col gap-3" {
            (alert(announced).variant(AlertVariant::Danger))
            (alert(silent).variant(AlertVariant::Info))
        }
    }
}
