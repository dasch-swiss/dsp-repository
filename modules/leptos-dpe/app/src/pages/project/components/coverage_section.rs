use leptos::prelude::*;

use crate::domain::models::AuthorityFileReference;
use crate::domain::TemporalCoverage;

#[component]
pub fn CoverageSection(
    temporal_coverage: Vec<TemporalCoverage>,
    spatial_coverage: Vec<AuthorityFileReference>,
) -> impl IntoView {
    view! {
        {(!temporal_coverage.is_empty())
            .then(|| {
                view! {
                    <div id="temporal-coverage">
                        <h3 class="text-sm font-semibold text-gray-700 mb-2">"Temporal Coverage"</h3>
                        <div class="flex flex-wrap gap-1.5">
                            {temporal_coverage
                                .iter()
                                .map(|t| match t {
                                    TemporalCoverage::Text(map) => {
                                        let label = map
                                            .iter()
                                            .map(|(lang, text)| format!("{} ({})", text, lang))
                                            .collect::<Vec<_>>()
                                            .join(" / ");
                                        view! {
                                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                                {label}
                                            </span>
                                        }
                                            .into_any()
                                    }
                                    TemporalCoverage::Reference(ref_) => {
                                        let label = ref_
                                            .text
                                            .clone()
                                            .unwrap_or_else(|| ref_.url.clone());
                                        view! {
                                            <a
                                                href=ref_.url.clone()
                                                class="tooltip"
                                                data-tip=ref_.url.clone()
                                            >
                                                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                                    {label}
                                                </span>
                                            </a>
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
                    <div id="spatial-coverage">
                        <h3 class="text-sm font-semibold text-gray-700 mb-2">"Spatial Coverage"</h3>
                        <div class="flex flex-wrap gap-1.5">
                            {spatial_coverage
                                .iter()
                                .map(|s| {
                                    let label = s.text.clone().unwrap_or_else(|| s.url.clone());
                                    view! {
                                        <a
                                            href=s.url.clone()
                                            class="tooltip"
                                            data-tip=s.url.clone()
                                        >
                                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                                {label}
                                            </span>
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                }
                    .into_any()
            })}
    }
}
