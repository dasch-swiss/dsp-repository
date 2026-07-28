//! Badge showcase.

use maud::{html, Markup, Render};
use mosaic_tiles::badge::{badge, BadgeSize, BadgeVariant};
use mosaic_tiles::icon::{icon, IconChevronUp, Info, Mail};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Badge",
        "A small label component for highlighting status, categories, and metadata.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "badge-variants",
                "Badge Variants",
                "Available badge colors: Primary, Secondary, Success, Warning, Danger, and Info",
                variants(),
            )
        })
        ({
            example(
                "badge-sizes",
                "Badge Sizes",
                "Available sizes: Small, Medium, and Large",
                sizes(),
            )
        })
        ({
            example(
                "badge-usage",
                "Usage Examples",
                "Common use cases: status indicators, categories, counts, and priority levels",
                usage(),
            )
        })
        ({
            example(
                "badge-with_icons",
                "Badges with Icons",
                "Combining badges with icon components",
                with_icons(),
            )
        })
    }
}

/// Shorthand: a badge with the given variant and default size.
fn b(variant: BadgeVariant, content: impl Render) -> impl Render {
    badge(content).variant(variant)
}

/// Shorthand: a badge with explicit variant and size.
fn bs(variant: BadgeVariant, size: BadgeSize, content: impl Render) -> impl Render {
    badge(content).variant(variant).size(size)
}

fn variants() -> Markup {
    html! {
        div class="flex flex-wrap gap-3 items-center" {
            ({
                b(
                    BadgeVariant::Primary,
                    html! {
                        "Primary"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Secondary,
                    html! {
                        "Secondary"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Success,
                    html! {
                        "Success"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Warning,
                    html! {
                        "Warning"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Danger,
                    html! {
                        "Danger"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Info,
                    html! {
                        "Info"
                    },
                )
            })
        }
    }
}

fn sizes() -> Markup {
    html! {
        div class="flex flex-wrap gap-3 items-center" {
            ({
                bs(
                    BadgeVariant::Primary,
                    BadgeSize::Small,
                    html! {
                        "Small"
                    },
                )
            })
            ({
                bs(
                    BadgeVariant::Primary,
                    BadgeSize::Medium,
                    html! {
                        "Medium"
                    },
                )
            })
            ({
                bs(
                    BadgeVariant::Primary,
                    BadgeSize::Large,
                    html! {
                        "Large"
                    },
                )
            })
        }
    }
}

fn usage() -> Markup {
    html! {
        div class="space-y-6" {
            div {
                h4 class="text-base font-semibold mb-2" { "Status Indicators" }
                div class="flex flex-wrap gap-3 items-center" {
                    ({
                        b(
                            BadgeVariant::Success,
                            html! {
                                "Active"
                            },
                        )
                    })
                    ({
                        b(
                            BadgeVariant::Warning,
                            html! {
                                "Pending"
                            },
                        )
                    })
                    ({
                        b(
                            BadgeVariant::Danger,
                            html! {
                                "Inactive"
                            },
                        )
                    })
                    ({
                        b(
                            BadgeVariant::Info,
                            html! {
                                "Draft"
                            },
                        )
                    })
                }
            }
            div {
                h4 class="text-base font-semibold mb-2" { "Categories" }
                div class="flex flex-wrap gap-2 items-center" {
                    ({
                        bs(
                            BadgeVariant::Secondary,
                            BadgeSize::Small,
                            html! {
                                "Technology"
                            },
                        )
                    })
                    ({
                        bs(
                            BadgeVariant::Secondary,
                            BadgeSize::Small,
                            html! {
                                "Design"
                            },
                        )
                    })
                    ({
                        bs(
                            BadgeVariant::Secondary,
                            BadgeSize::Small,
                            html! {
                                "Business"
                            },
                        )
                    })
                    ({
                        bs(
                            BadgeVariant::Secondary,
                            BadgeSize::Small,
                            html! {
                                "Marketing"
                            },
                        )
                    })
                }
            }
            div {
                h4 class="text-base font-semibold mb-2" { "Counts and Metrics" }
                div class="flex flex-wrap gap-3 items-center" {
                    span class="text-neutral-700" { "Notifications" }
                    ({
                        bs(
                            BadgeVariant::Danger,
                            BadgeSize::Small,
                            html! {
                                "12"
                            },
                        )
                    })
                    span class="text-neutral-700 ml-6" { "New Messages" }
                    ({
                        bs(
                            BadgeVariant::Info,
                            BadgeSize::Small,
                            html! {
                                "3"
                            },
                        )
                    })
                    span class="text-neutral-700 ml-6" { "Tasks" }
                    ({
                        bs(
                            BadgeVariant::Warning,
                            BadgeSize::Small,
                            html! {
                                "7"
                            },
                        )
                    })
                }
            }
            div {
                h4 class="text-base font-semibold mb-2" { "Priority Levels" }
                div class="space-y-3" {
                    div class="flex items-center gap-3" {
                        ({
                            b(
                                BadgeVariant::Danger,
                                html! {
                                    "High Priority"
                                },
                            )
                        })
                        span class="text-sm text-neutral-600" {
                            "Critical task requires immediate attention"
                        }
                    }
                    div class="flex items-center gap-3" {
                        ({
                            b(
                                BadgeVariant::Warning,
                                html! {
                                    "Medium Priority"
                                },
                            )
                        })
                        span class="text-sm text-neutral-600" { "Important but not urgent" }
                    }
                    div class="flex items-center gap-3" {
                        ({
                            b(
                                BadgeVariant::Info,
                                html! {
                                    "Low Priority"
                                },
                            )
                        })
                        span class="text-sm text-neutral-600" { "Can be addressed later" }
                    }
                }
            }
        }
    }
}

fn with_icons() -> Markup {
    html! {
        div class="flex flex-wrap gap-3 items-center" {
            ({
                b(
                    BadgeVariant::Info,
                    html! {
                        (icon(Info, "w-3 h-3 inline mr-1")) "New"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Success,
                    html! {
                        (icon(IconChevronUp, "w-3 h-3 inline mr-1")) "Trending"
                    },
                )
            })
            ({
                b(
                    BadgeVariant::Warning,
                    html! {
                        (icon(Mail, "w-3 h-3 inline mr-1")) "5 Messages"
                    },
                )
            })
        }
    }
}
