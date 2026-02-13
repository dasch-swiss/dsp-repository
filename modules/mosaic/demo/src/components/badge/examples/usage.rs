use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};

#[component]
pub fn UsageExample() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h4 class="text-base font-semibold mb-2">"Status Indicators"</h4>
                <div class="flex flex-wrap gap-3 items-center">
                    <Badge variant=BadgeVariant::Success>"Active"</Badge>
                    <Badge variant=BadgeVariant::Warning>"Pending"</Badge>
                    <Badge variant=BadgeVariant::Danger>"Inactive"</Badge>
                    <Badge variant=BadgeVariant::Info>"Draft"</Badge>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-2">"Categories"</h4>
                <div class="flex flex-wrap gap-2 items-center">
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Secondary>
                        "Technology"
                    </Badge>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Secondary>
                        "Design"
                    </Badge>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Secondary>
                        "Business"
                    </Badge>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Secondary>
                        "Marketing"
                    </Badge>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-2">"Counts and Metrics"</h4>
                <div class="flex flex-wrap gap-3 items-center">
                    <span class="text-neutral-700">"Notifications"</span>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Danger>
                        "12"
                    </Badge>
                    <span class="text-neutral-700 ml-6">"New Messages"</span>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Info>
                        "3"
                    </Badge>
                    <span class="text-neutral-700 ml-6">"Tasks"</span>
                    <Badge size=BadgeSize::Small variant=BadgeVariant::Warning>
                        "7"
                    </Badge>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-2">"Priority Levels"</h4>
                <div class="space-y-3">
                    <div class="flex items-center gap-3">
                        <Badge variant=BadgeVariant::Danger>"High Priority"</Badge>
                        <span class="text-sm text-neutral-600">
                            "Critical task requires immediate attention"
                        </span>
                    </div>
                    <div class="flex items-center gap-3">
                        <Badge variant=BadgeVariant::Warning>"Medium Priority"</Badge>
                        <span class="text-sm text-neutral-600">"Important but not urgent"</span>
                    </div>
                    <div class="flex items-center gap-3">
                        <Badge variant=BadgeVariant::Info>"Low Priority"</Badge>
                        <span class="text-sm text-neutral-600">"Can be addressed later"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}
