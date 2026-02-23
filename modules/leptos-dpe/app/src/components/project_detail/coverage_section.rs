use leptos::prelude::*;

use crate::domain::models::AuthorityFileReference;
use crate::domain::TemporalCoverage;

#[component]
pub fn CoverageSection(
    temporal_coverage: Vec<TemporalCoverage>,
    spatial_coverage: Vec<AuthorityFileReference>,
) -> impl IntoView {
    view! {
        <div>
            {(!temporal_coverage.is_empty())
                .then(|| {
                    view! {
                        <div
                            id="temporal-coverage"
                        >
                            <h3 class="text-base font-semibold mb-3">
                                "Temporal Coverage"
                            </h3>
                            <div class="space-y-2 text-sm">
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
                        >
                            <h3 class="text-base font-semibold mb-3">
                                "Spatial Coverage"
                            </h3>
                            <div class="space-y-2 text-sm">
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
