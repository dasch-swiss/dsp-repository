use dpe_core::models::AuthorityFileReference;
use dpe_core::project::TemporalCoverage;
use maud::{html, Markup};

use crate::components::{placeholder_value, should_render_value};

const CHIP: &str = "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700";

/// Temporal + spatial coverage chips. Reference values that are placeholders
/// render via [`placeholder_value`] (hidden in production); real references link
/// out with a tooltip of the URL. Sections with no renderable values are omitted.
pub fn coverage_section(temporal_coverage: &[TemporalCoverage], spatial_coverage: &[AuthorityFileReference]) -> Markup {
    let temporal: Vec<&TemporalCoverage> = temporal_coverage
        .iter()
        .filter(|t| match t {
            TemporalCoverage::Text(_) => true,
            TemporalCoverage::Reference(r) => should_render_value(&r.url),
        })
        .collect();
    let spatial: Vec<&AuthorityFileReference> =
        spatial_coverage.iter().filter(|s| should_render_value(&s.url)).collect();

    html! {
        @if !temporal.is_empty() {
            div {
                h3 class="dpe-subtitle" { "Temporal Coverage" }
                div class="flex flex-wrap gap-1.5" {
                    @for t in temporal {
                        @match t {
                            TemporalCoverage::Text(map) => {
                                @let label = map
                                    .iter()
                                    .map(|(lang, text)| format!("{text} ({lang})"))
                                    .collect::<Vec<_>>()
                                    .join(" / ");
                                span class=(CHIP) { (label) }
                            }
                            TemporalCoverage::Reference(r) => {
                                @if dpe_core::is_placeholder(&r.url) { (placeholder_value(&r.url)) } @else {
                                    @let label = r.text.clone().unwrap_or_else(|| r.url.clone());
                                    a href=(r.url) class="tooltip" data-tip=(r.url) {
                                        span class=(CHIP) { (label) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        @if !spatial.is_empty() {
            div {
                h3 class="dpe-subtitle" { "Spatial Coverage" }
                div class="flex flex-wrap gap-1.5" {
                    @for s in spatial {
                        @if dpe_core::is_placeholder(&s.url) { (placeholder_value(&s.url)) } @else {
                            @let label = s.text.clone().unwrap_or_else(|| s.url.clone());
                            a href=(s.url) class="tooltip" data-tip=(s.url) {
                                span class=(CHIP) { (label) }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_coverage_links_out_with_tooltip() {
        let temporal = vec![TemporalCoverage::Reference(AuthorityFileReference {
            type_: "URL".to_string(),
            url: "https://chronontology.dainst.org/period/x".to_string(),
            text: Some("Bronze Age".to_string()),
        })];
        let out = coverage_section(&temporal, &[]).into_string();
        assert!(out.contains("Temporal Coverage"), "{out}");
        assert!(out.contains(r#"href="https://chronontology.dainst.org/period/x""#), "{out}");
        assert!(out.contains(r#"data-tip="https://chronontology.dainst.org/period/x""#), "{out}");
        assert!(out.contains("Bronze Age"), "{out}");
    }

    #[test]
    fn spatial_reference_renders() {
        let spatial = vec![AuthorityFileReference {
            type_: "URL".to_string(),
            url: "https://www.geonames.org/1".to_string(),
            text: Some("Rome".to_string()),
        }];
        let out = coverage_section(&[], &spatial).into_string();
        assert!(out.contains("Spatial Coverage"), "{out}");
        assert!(out.contains("Rome"), "{out}");
    }

    #[test]
    fn empty_renders_nothing() {
        assert_eq!(coverage_section(&[], &[]).into_string(), "");
    }
}
