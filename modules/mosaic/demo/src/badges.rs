use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize, BadgeVariant};

#[component]
pub fn BadgeExamples() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <section>
                <h2 class="text-2xl font-bold mb-4">"Badge Component Examples"</h2>

                <div class="space-y-6">
                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Variants"</h3>
                        <div class="flex flex-wrap gap-3 items-center">
                            <Badge variant=BadgeVariant::Primary>"Primary"</Badge>
                            <Badge variant=BadgeVariant::Secondary>"Secondary"</Badge>
                            <Badge variant=BadgeVariant::Success>"Success"</Badge>
                            <Badge variant=BadgeVariant::Warning>"Warning"</Badge>
                            <Badge variant=BadgeVariant::Danger>"Danger"</Badge>
                            <Badge variant=BadgeVariant::Info>"Info"</Badge>
                        </div>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Sizes"</h3>
                        <div class="flex flex-wrap gap-3 items-center">
                            <Badge size=BadgeSize::Small variant=BadgeVariant::Primary>
                                "Small"
                            </Badge>
                            <Badge size=BadgeSize::Medium variant=BadgeVariant::Primary>
                                "Medium"
                            </Badge>
                            <Badge size=BadgeSize::Large variant=BadgeVariant::Primary>
                                "Large"
                            </Badge>
                        </div>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">
                            "Size Comparison Across Variants"
                        </h3>
                        <div class="space-y-3">
                            <div class="flex flex-wrap gap-3 items-center">
                                <span class="text-sm text-gray-600 w-16">"Small:"</span>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Primary>
                                    "Primary"
                                </Badge>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Secondary>
                                    "Secondary"
                                </Badge>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Success>
                                    "Success"
                                </Badge>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Warning>
                                    "Warning"
                                </Badge>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Danger>
                                    "Danger"
                                </Badge>
                                <Badge size=BadgeSize::Small variant=BadgeVariant::Info>
                                    "Info"
                                </Badge>
                            </div>
                            <div class="flex flex-wrap gap-3 items-center">
                                <span class="text-sm text-gray-600 w-16">"Medium:"</span>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Primary>
                                    "Primary"
                                </Badge>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Secondary>
                                    "Secondary"
                                </Badge>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Success>
                                    "Success"
                                </Badge>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Warning>
                                    "Warning"
                                </Badge>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Danger>
                                    "Danger"
                                </Badge>
                                <Badge size=BadgeSize::Medium variant=BadgeVariant::Info>
                                    "Info"
                                </Badge>
                            </div>
                            <div class="flex flex-wrap gap-3 items-center">
                                <span class="text-sm text-gray-600 w-16">"Large:"</span>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Primary>
                                    "Primary"
                                </Badge>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Secondary>
                                    "Secondary"
                                </Badge>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Success>
                                    "Success"
                                </Badge>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Warning>
                                    "Warning"
                                </Badge>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Danger>
                                    "Danger"
                                </Badge>
                                <Badge size=BadgeSize::Large variant=BadgeVariant::Info>
                                    "Info"
                                </Badge>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Usage Examples"</h3>

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
                            <span class="text-gray-700">"Notifications"</span>
                            <Badge size=BadgeSize::Small variant=BadgeVariant::Danger>
                                "12"
                            </Badge>
                            <span class="text-gray-700 ml-6">"New Messages"</span>
                            <Badge size=BadgeSize::Small variant=BadgeVariant::Info>
                                "3"
                            </Badge>
                            <span class="text-gray-700 ml-6">"Tasks"</span>
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
                                <span class="text-sm text-gray-600">
                                    "Critical task requires immediate attention"
                                </span>
                            </div>
                            <div class="flex items-center gap-3">
                                <Badge variant=BadgeVariant::Warning>"Medium Priority"</Badge>
                                <span class="text-sm text-gray-600">
                                    "Important but not urgent"
                                </span>
                            </div>
                            <div class="flex items-center gap-3">
                                <Badge variant=BadgeVariant::Info>"Low Priority"</Badge>
                                <span class="text-sm text-gray-600">"Can be addressed later"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

        </div>
    }
}
