//! Button showcase.
//!
//! The old `interactive` example relied on Leptos signals (a live counter); it
//! is dropped along with the islands runtime. The remaining examples are static.

use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonProps, ButtonType, ButtonVariant};
use mosaic_tiles::icon::{icon, CopyPaste, IconChevronRight, IconSearch, Info, LinkExternal, Mail};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("Button", "A clickable button component with multiple variants and states.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "button-variants",
                "Button Variants",
                "Available button styles: Primary, Secondary, Outline, Ghost, and Soft",
                variants(),
            )
        })
        ({
            example(
                "button-types",
                "Button Types",
                "HTML button types: Button, Submit, and Reset",
                types(),
            )
        })
        ({
            example(
                "button-disabled",
                "Disabled State",
                "Buttons in disabled state",
                disabled(),
            )
        })
        ({
            example(
                "button-with_icons",
                "Buttons with Icons",
                "Buttons combined with icon components",
                with_icons(),
            )
        })
    }
}

/// Shorthand: a button with a variant and label.
fn btn(variant: ButtonVariant, label: Markup) -> Markup {
    button(ButtonProps { variant, ..Default::default() }, label)
}

fn variants() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            ({
                btn(
                    ButtonVariant::Primary,
                    html! {
                        "Primary Button"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Secondary,
                    html! {
                        "Secondary Button"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Outline,
                    html! {
                        "Outline Button"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Ghost,
                    html! {
                        "Ghost Button"
                    },
                )
            })
            ({
                btn(
                    ButtonVariant::Soft,
                    html! {
                        "Soft Button"
                    },
                )
            })
        }
    }
}

fn types() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            ({
                button(
                    ButtonProps {
                        button_type: ButtonType::Button,
                        ..Default::default()
                    },
                    html! {
                        "Button"
                    },
                )
            })
            ({
                button(
                    ButtonProps {
                        button_type: ButtonType::Submit,
                        ..Default::default()
                    },
                    html! {
                        "Submit"
                    },
                )
            })
            ({
                button(
                    ButtonProps {
                        button_type: ButtonType::Reset,
                        ..Default::default()
                    },
                    html! {
                        "Reset"
                    },
                )
            })
        }
    }
}

fn disabled() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            ({
                button(
                    ButtonProps {
                        variant: ButtonVariant::Primary,
                        disabled: true,
                        ..Default::default()
                    },
                    html! {
                        "Disabled Primary"
                    },
                )
            })
            ({
                button(
                    ButtonProps {
                        variant: ButtonVariant::Secondary,
                        disabled: true,
                        ..Default::default()
                    },
                    html! {
                        "Disabled Secondary"
                    },
                )
            })
        }
    }
}

fn with_icons() -> Markup {
    html! {
        div class="space-y-6" {
            div {
                h4 class="text-base font-semibold mb-2" { "Icon + Text" }
                div class="flex gap-4 items-center" {
                    ({
                        btn(
                            ButtonVariant::Primary,
                            html! {
                                (icon(IconSearch, "w-4 h-4 inline mr-2")) "Search"
                            },
                        )
                    })
                    ({
                        btn(
                            ButtonVariant::Secondary,
                            html! {
                                (icon(Mail, "w-4 h-4 inline mr-2")) "Send Email"
                            },
                        )
                    })
                    ({
                        btn(
                            ButtonVariant::Primary,
                            html! {
                                "Visit Docs"(icon(LinkExternal, "w-4 h-4 inline ml-2"))
                            },
                        )
                    })
                }
            }
            div {
                h4 class="text-base font-semibold mb-2" { "Icon Only" }
                div class="flex gap-4 items-center" {
                    ({
                        btn(
                            ButtonVariant::Secondary,
                            html! {
                                (icon(CopyPaste, "w-4 h-4"))
                            },
                        )
                    })
                    ({
                        btn(
                            ButtonVariant::Secondary,
                            html! {
                                (icon(Info, "w-4 h-4"))
                            },
                        )
                    })
                    ({
                        btn(
                            ButtonVariant::Primary,
                            html! {
                                (icon(IconChevronRight, "w-4 h-4"))
                            },
                        )
                    })
                }
            }
        }
    }
}
