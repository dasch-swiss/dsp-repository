use dpe_core::{Page, Project};
use leptos::prelude::*;

#[server]
pub async fn list_type_of_data() -> Result<Vec<String>, ServerFnError> {
    use std::collections::HashSet;

    use dpe_core::all_projects;

    let mut types: HashSet<String> = HashSet::new();
    for project in all_projects() {
        if let Some(t) = &project.type_of_data {
            types.extend(t.iter().cloned());
        }
    }
    let mut result: Vec<String> = types.into_iter().collect();
    result.sort();
    Ok(result)
}

#[server]
pub async fn list_data_languages() -> Result<Vec<(String, String)>, ServerFnError> {
    use std::collections::HashSet;

    use dpe_core::{all_projects, language_display_name};

    let mut codes: HashSet<String> = HashSet::new();
    for project in all_projects() {
        if let Some(langs) = &project.data_language {
            for lang in langs {
                codes.insert(lang.clone());
            }
        }
    }
    let mut result: Vec<(String, String)> = codes
        .into_iter()
        .map(|code| {
            let display = language_display_name(&code).to_string();
            (code, display)
        })
        .collect();
    result.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
#[server]
pub async fn list_projects(
    ongoing: Option<bool>,
    finished: Option<bool>,
    search: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
    type_of_data: Option<String>,
    data_language: Option<String>,
    access_rights: Option<String>,
) -> Result<Page, ServerFnError> {
    use dpe_core::all_projects;

    use super::project::ProjectQuery;

    let query = ProjectQuery {
        ongoing,
        finished,
        search,
        page,
        type_of_data,
        data_language,
        access_rights,
        dialog: None,
    };

    Ok(filter_and_paginate(all_projects(), &query, page_size))
}

// Gated on non-wasm targets to match `dpe_core::all_projects` (the only
// realistic source of the `&[Project]` argument); the previous narrower
// `#[cfg(feature = "ssr")]` gate kept the function from compiling under
// `cargo hack --features default`, even though the body only touches plain
// `dpe_core` types and has nothing SSR-specific.
#[cfg(not(target_arch = "wasm32"))]
pub fn filter_and_paginate(projects: &[Project], query: &super::project::ProjectQuery, page_size: Option<i32>) -> Page {
    use dpe_core::{AccessRightsType, ProjectStatus};

    let items_per_page = page_size.unwrap_or(9).clamp(1, 100) as usize;
    let search_lower = query.search().to_lowercase();
    let type_of_data_filter = query.type_of_data();
    let data_language_filter = query.data_language();
    let access_rights_filter = query.access_rights();

    let filtered_projects: Vec<&Project> = projects
        .iter()
        .filter(|project| {
            // Status filter
            let is_ongoing = project.status == ProjectStatus::Ongoing;
            let is_finished = project.status == ProjectStatus::Finished;
            let status_match = match (query.ongoing, query.finished) {
                (None, None) => true,
                _ => (query.ongoing() && is_ongoing) || (query.finished() && is_finished),
            };

            // Search filter
            let search_match = if search_lower.is_empty() {
                true
            } else {
                project.name.to_lowercase().contains(&search_lower)
                    || project.short_description.to_lowercase().contains(&search_lower)
                    || project.shortcode.to_lowercase().contains(&search_lower)
                    || project.status.as_str().contains(&search_lower)
            };

            // Type of data filter
            let type_match = if type_of_data_filter.is_empty() {
                true
            } else {
                project
                    .type_of_data
                    .as_ref()
                    .map(|types| types.iter().any(|t| type_of_data_filter.contains(t)))
                    .unwrap_or(false)
            };

            // Data language filter
            let language_match = if data_language_filter.is_empty() {
                true
            } else {
                project
                    .data_language
                    .as_ref()
                    .map(|langs| langs.iter().any(|lang| data_language_filter.contains(lang)))
                    .unwrap_or(false)
            };

            // Access rights filter
            let access_rights_match = if access_rights_filter.is_empty() {
                true
            } else {
                let label = match project.access_rights.access_rights {
                    AccessRightsType::FullOpenAccess => "Full Open Access",
                    AccessRightsType::OpenAccessWithRestrictions => "Open Access with Restrictions",
                    AccessRightsType::EmbargoedAccess => "Embargoed Access",
                    AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
                };
                access_rights_filter.iter().any(|f| f == label)
            };

            status_match && search_match && type_match && language_match && access_rights_match
        })
        .collect();

    // Pre-compute lowercase sort keys to avoid O(n log n) String allocations in the comparator.
    let mut keyed: Vec<(String, &Project)> = filtered_projects.iter().map(|p| (p.name.to_lowercase(), *p)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let filtered_projects: Vec<&Project> = keyed.into_iter().map(|(_, p)| p).collect();

    let total_items = filtered_projects.len() as i32;
    let nr_pages = (filtered_projects.len()).div_ceil(items_per_page).max(1) as i32;

    let page_index = (query.page() - 1).max(0) as usize;
    let start_idx = page_index * items_per_page;
    let end_idx = (start_idx + items_per_page).min(filtered_projects.len());

    let items: Vec<Project> = filtered_projects[start_idx..end_idx].iter().map(|p| (*p).clone()).collect();

    Page { items, nr_pages, total_items }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use std::collections::HashMap;

    use dpe_core::{AccessRights, AccessRightsType, Attribution, Funding, Project, ProjectStatus};

    use super::super::project::ProjectQuery;
    use super::filter_and_paginate;

    fn make_project(
        name: &str,
        shortcode: &str,
        status: ProjectStatus,
        type_of_data: Option<Vec<&str>>,
        data_language: Option<Vec<&str>>,
        access_rights: AccessRightsType,
    ) -> Project {
        let data_language = data_language.map(|langs| langs.into_iter().map(|l| l.to_string()).collect());
        Project {
            id: shortcode.to_string(),
            pid: format!("pid-{shortcode}"),
            name: name.to_string(),
            shortcode: shortcode.to_string(),
            official_name: name.to_string(),
            status,
            short_description: format!("desc of {name}"),
            description: HashMap::new(),
            start_date: "2020-01-01".to_string(),
            end_date: "2021-01-01".to_string(),
            url: None,
            secondary_url: None,
            how_to_cite: name.to_string(),
            access_rights: AccessRights { access_rights, embargo_date: None },
            legal_info: vec![],
            data_management_plan: None,
            data_publication_year: None,
            type_of_data: type_of_data.map(|v| v.into_iter().map(str::to_string).collect()),
            data_language,
            clusters: vec![],
            collections: vec![],
            collection_ids: vec![],
            records: None,
            keywords: vec![],
            disciplines: vec![],
            temporal_coverage: vec![],
            spatial_coverage: vec![],
            attributions: vec![Attribution {
                contributor: "person-1".to_string(),
                contributor_type: vec!["Author".to_string()],
            }],
            abstract_text: None,
            contact_point: None,
            publications: None,
            funding: Funding::Text("None".to_string()),
            alternative_names: None,
            documentation_material: None,
            provenance: None,
            additional_material: None,
        }
    }

    fn default_project(name: &str) -> Project {
        make_project(name, name, ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess)
    }

    fn empty_query() -> ProjectQuery {
        ProjectQuery::default()
    }

    // --- status filtering ---

    #[test]
    fn no_status_filter_returns_all() {
        let projects = vec![
            make_project("A", "A", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
            make_project("B", "B", ProjectStatus::Finished, None, None, AccessRightsType::FullOpenAccess),
        ];
        let page = filter_and_paginate(&projects, &empty_query(), None);
        assert_eq!(page.total_items, 2);
    }

    #[test]
    fn ongoing_filter_returns_only_ongoing() {
        let projects = vec![
            make_project("A", "A", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
            make_project("B", "B", ProjectStatus::Finished, None, None, AccessRightsType::FullOpenAccess),
        ];
        let query = ProjectQuery { ongoing: Some(true), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "A");
    }

    #[test]
    fn finished_filter_returns_only_finished() {
        let projects = vec![
            make_project("A", "A", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
            make_project("B", "B", ProjectStatus::Finished, None, None, AccessRightsType::FullOpenAccess),
        ];
        let query = ProjectQuery { finished: Some(true), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "B");
    }

    #[test]
    fn both_status_flags_returns_all() {
        let projects = vec![
            make_project("A", "A", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
            make_project("B", "B", ProjectStatus::Finished, None, None, AccessRightsType::FullOpenAccess),
        ];
        let query = ProjectQuery {
            ongoing: Some(true),
            finished: Some(true),
            ..Default::default()
        };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 2);
    }

    // --- search filtering ---

    #[test]
    fn search_matches_name() {
        let projects = vec![default_project("Alpha"), default_project("Beta")];
        let query = ProjectQuery { search: Some("alph".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "Alpha");
    }

    #[test]
    fn search_is_case_insensitive() {
        let projects = vec![default_project("Alpha")];
        let query = ProjectQuery { search: Some("ALPHA".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
    }

    #[test]
    fn search_matches_shortcode() {
        let projects = vec![
            make_project(
                "Project One",
                "SC01",
                ProjectStatus::Ongoing,
                None,
                None,
                AccessRightsType::FullOpenAccess,
            ),
            make_project(
                "Project Two",
                "SC02",
                ProjectStatus::Ongoing,
                None,
                None,
                AccessRightsType::FullOpenAccess,
            ),
        ];
        let query = ProjectQuery { search: Some("sc01".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].shortcode, "SC01");
    }

    #[test]
    fn search_matches_short_description() {
        let projects = vec![default_project("Alpha")];
        let query = ProjectQuery {
            search: Some("desc of alpha".to_string()),
            ..Default::default()
        };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
    }

    #[test]
    fn search_no_match_returns_empty() {
        let projects = vec![default_project("Alpha"), default_project("Beta")];
        let query = ProjectQuery { search: Some("xyz".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 0);
        assert_eq!(page.items.len(), 0);
    }

    // --- type_of_data filtering ---

    #[test]
    fn type_of_data_filter_matches() {
        let projects = vec![
            make_project(
                "A",
                "A",
                ProjectStatus::Ongoing,
                Some(vec!["Text"]),
                None,
                AccessRightsType::FullOpenAccess,
            ),
            make_project(
                "B",
                "B",
                ProjectStatus::Ongoing,
                Some(vec!["Image"]),
                None,
                AccessRightsType::FullOpenAccess,
            ),
            make_project("C", "C", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
        ];
        let query = ProjectQuery { type_of_data: Some("Text".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "A");
    }

    #[test]
    fn type_of_data_filter_multi_value() {
        let projects = vec![
            make_project(
                "A",
                "A",
                ProjectStatus::Ongoing,
                Some(vec!["Text"]),
                None,
                AccessRightsType::FullOpenAccess,
            ),
            make_project(
                "B",
                "B",
                ProjectStatus::Ongoing,
                Some(vec!["Image"]),
                None,
                AccessRightsType::FullOpenAccess,
            ),
            make_project("C", "C", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
        ];
        let query = ProjectQuery {
            type_of_data: Some("Text,Image".to_string()),
            ..Default::default()
        };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 2);
    }

    // --- data_language filtering ---

    #[test]
    fn data_language_filter_matches() {
        let projects = vec![
            make_project(
                "A",
                "A",
                ProjectStatus::Ongoing,
                None,
                Some(vec!["en"]),
                AccessRightsType::FullOpenAccess,
            ),
            make_project(
                "B",
                "B",
                ProjectStatus::Ongoing,
                None,
                Some(vec!["fr"]),
                AccessRightsType::FullOpenAccess,
            ),
        ];
        let query = ProjectQuery { data_language: Some("en".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "A");
    }

    // --- access_rights filtering ---

    #[test]
    fn access_rights_filter_matches() {
        let projects = vec![
            make_project("A", "A", ProjectStatus::Ongoing, None, None, AccessRightsType::FullOpenAccess),
            make_project(
                "B",
                "B",
                ProjectStatus::Ongoing,
                None,
                None,
                AccessRightsType::OpenAccessWithRestrictions,
            ),
        ];
        let query = ProjectQuery {
            access_rights: Some("Full Open Access".to_string()),
            ..Default::default()
        };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "A");
    }

    // --- sorting ---

    #[test]
    fn results_sorted_alphabetically_case_insensitive() {
        let projects = vec![
            default_project("Zebra"),
            default_project("apple"),
            default_project("Mango"),
        ];
        let page = filter_and_paginate(&projects, &empty_query(), None);
        let names: Vec<&str> = page.items.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["apple", "Mango", "Zebra"]);
    }

    // --- pagination ---

    #[test]
    fn pagination_first_page() {
        let projects: Vec<Project> = (0..20).map(|i| default_project(&format!("P{i:02}"))).collect();
        let page = filter_and_paginate(&projects, &empty_query(), Some(5));
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total_items, 20);
        assert_eq!(page.nr_pages, 4);
    }

    #[test]
    fn pagination_second_page() {
        let projects: Vec<Project> = (0..20).map(|i| default_project(&format!("P{i:02}"))).collect();
        let query = ProjectQuery { page: Some(2), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, Some(5));
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total_items, 20);
    }

    #[test]
    fn pagination_last_partial_page() {
        let projects: Vec<Project> = (0..11).map(|i| default_project(&format!("P{i:02}"))).collect();
        let query = ProjectQuery { page: Some(2), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, Some(5));
        assert_eq!(page.items.len(), 5);
        let query3 = ProjectQuery { page: Some(3), ..Default::default() };
        let page3 = filter_and_paginate(&projects, &query3, Some(5));
        assert_eq!(page3.items.len(), 1);
    }

    #[test]
    fn empty_result_has_one_page() {
        let page = filter_and_paginate(&[], &empty_query(), None);
        assert_eq!(page.total_items, 0);
        assert_eq!(page.nr_pages, 1);
        assert_eq!(page.items.len(), 0);
    }

    // --- ProjectQuery toggle methods ---

    #[test]
    fn with_type_of_data_toggled_adds_value() {
        let q = empty_query().with_type_of_data_toggled("Text");
        assert_eq!(q.type_of_data(), vec!["Text"]);
    }

    #[test]
    fn with_type_of_data_toggled_removes_value() {
        let q = ProjectQuery {
            type_of_data: Some("Text,Image".to_string()),
            ..Default::default()
        };
        let q2 = q.with_type_of_data_toggled("Text");
        assert_eq!(q2.type_of_data(), vec!["Image"]);
    }

    #[test]
    fn with_type_of_data_toggled_resets_page() {
        let q = ProjectQuery { page: Some(3), ..Default::default() };
        let q2 = q.with_type_of_data_toggled("Text");
        assert_eq!(q2.page(), 1);
    }

    #[test]
    fn with_status_toggled_flips_ongoing() {
        let q = empty_query().with_status_toggled("ongoing");
        assert!(q.ongoing());
        let q2 = q.with_status_toggled("ongoing");
        assert!(!q2.ongoing());
    }

    #[test]
    fn with_status_toggled_resets_page() {
        let q = ProjectQuery { page: Some(5), ..Default::default() };
        let q2 = q.with_status_toggled("ongoing");
        assert_eq!(q2.page(), 1);
    }

    // --- to_query_string ---

    #[test]
    fn to_query_string_empty() {
        assert_eq!(empty_query().to_query_string(), "");
    }

    #[test]
    fn to_query_string_with_search() {
        let q = ProjectQuery {
            search: Some("hello world".to_string()),
            ..Default::default()
        };
        let qs = q.to_query_string();
        assert!(qs.contains("search=hello%20world"));
    }

    #[test]
    fn to_query_string_page_1_omitted() {
        let q = ProjectQuery { page: Some(1), ..Default::default() };
        assert_eq!(q.to_query_string(), "");
    }

    #[test]
    fn to_query_string_page_2_included() {
        let q = ProjectQuery { page: Some(2), ..Default::default() };
        let qs = q.to_query_string();
        assert!(qs.contains("page=2"), "got: {qs}");
    }
}

#[server]
pub async fn get_project(shortcode: String) -> Result<Option<Project>, ServerFnError> {
    use std::fs;
    use std::path::PathBuf;

    use dpe_core::{all_projects, get_data_dir, CollectionRef};

    // Look up the base project from the in-memory cache — no disk scan needed.
    let Some(base) = all_projects().iter().find(|p| p.shortcode == shortcode) else {
        return Ok(None);
    };
    let mut project = base.clone();

    // Resolve clusters from the in-memory cache (reverse lookup).
    project.clusters = dpe_core::cluster_cache::all_clusters()
        .iter()
        .filter(|raw| raw.projects.iter().any(|p| p == &shortcode))
        .map(|raw| raw.clone().into_ref())
        .collect();

    // Resolve collection IDs stored on the cached project.
    let data_path = PathBuf::from(get_data_dir());
    let collections_dir = data_path.join("collections");
    project.collections = project
        .collection_ids
        .iter()
        .filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .filter_map(|id| {
            let path = collections_dir.join(format!("{}.json", id));
            fs::read_to_string(&path)
                .ok()
                .and_then(|json: String| serde_json::from_str::<CollectionRef>(&json).ok())
        })
        .collect();

    Ok(Some(project))
}
