use leptos::prelude::*;

use crate::domain::models::AuthorityFileReference;
use crate::domain::TemporalCoverage;

#[component]
pub fn CoverageSection(
    temporal_coverage: Vec<TemporalCoverage>,
    spatial_coverage: Vec<AuthorityFileReference>,
) -> impl IntoView {
    view! {
        <div class="grid md:grid-cols-2 gap-6">
            {(!temporal_coverage.is_empty())
                .then(|| {
                    view! {
                        <div
                            id="temporal-coverage"
                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                        >
                            <h3 class="text-xl font-bold mb-3">
                                "Temporal Coverage"
                            </h3>
                            <div class="space-y-2">
                                {temporal_coverage
                                    .iter()
                                    .map(|t| match t {
                                        TemporalCoverage::Text(map) => {
                                            view! {
                                                <div>
                                                    {map
                                                        .iter()
                                                        .map(|(lang, text)| {
                                                            format!("{} ({})", text, lang)
                                                        })
                                                        .collect::<Vec<_>>()
                                                        .join(" / ")}
                                                </div>
                                            }
                                                .into_any()
                                        }
                                        TemporalCoverage::Reference(ref_) => {
                                            view! {
                                                <div>
                                                    <a
                                                        href=ref_.url.clone()
                                                        class="link link-primary"
                                                    >
                                                        {ref_
                                                            .text
                                                            .clone()
                                                            .unwrap_or_else(|| ref_.url.clone())}
                                                    </a>
                                                    <span class="text-sm text-base-content/70 ml-2">
                                                        "(" {ref_.type_.clone()} ")"
                                                    </span>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                        .into_any()
                })}
            {(!spatial_coverage.is_empty())
                .then(|| {
                    view! {
                        <div
                            id="spatial-coverage"
                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                        >
                            <h3 class="text-xl font-bold mb-3">
                                "Spatial Coverage"
                            </h3>
                            <div class="space-y-2">
                                {spatial_coverage
                                    .iter()
                                    .map(|s| {
                                        view! {
                                            <div>
                                                <a
                                                    href=s.url.clone()
                                                    class="link link-primary"
                                                >
                                                    {s
                                                        .text
                                                        .clone()
                                                        .unwrap_or_else(|| s.url.clone())}
                                                </a>
                                                <span class="text-sm text-base-content/70 ml-2">
                                                    "(" {s.type_.clone()} ")"
                                                </span>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                        .into_any()
                })}
        </div>
    }
}
