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
pub async fn list_data_languages() -> Result<Vec<String>, ServerFnError> {
    use std::collections::HashSet;

    use dpe_core::{all_projects, lang_value};

    let mut languages: HashSet<String> = HashSet::new();
    for project in all_projects() {
        if let Some(langs) = &project.data_language {
            for lang_map in langs {
                if let Some(val) = lang_value(lang_map) {
                    languages.insert(val.clone());
                }
            }
        }
    }
    let mut result: Vec<String> = languages.into_iter().collect();
    result.sort();
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
    use super::project::ProjectQuery;
    use dpe_core::all_projects;

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

#[cfg(feature = "ssr")]
pub fn filter_and_paginate(projects: &[Project], query: &super::project::ProjectQuery, page_size: Option<i32>) -> Page {
    use dpe_core::{AccessRightsType, ProjectStatus, lang_value};

    let items_per_page = page_size.unwrap_or(9).clamp(1, 100) as usize;
    let search_lower = query.search().to_lowercase();
    let type_of_data_filter = query.type_of_data();
    let data_language_filter = query.data_language();
    let access_rights_filter = query.access_rights();

    let mut filtered_projects: Vec<&Project> = projects
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
                    .map(|langs| {
                        langs.iter().any(|lang_map| {
                            lang_value(lang_map).map(|v| data_language_filter.contains(v)).unwrap_or(false)
                        })
                    })
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

    filtered_projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

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
        let data_language = data_language.map(|langs| {
            langs
                .into_iter()
                .map(|l| {
                    let mut m = HashMap::new();
                    m.insert("en".to_string(), l.to_string());
                    m
                })
                .collect()
        });
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
    fn search_matches_name() {
        let projects = vec![default_project("Alpha"), default_project("Beta")];
        let query = ProjectQuery { search: Some("alph".to_string()), ..Default::default() };
        let page = filter_and_paginate(&projects, &query, None);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.items[0].name, "Alpha");
    }

    #[test]
    fn pagination_first_page() {
        let projects: Vec<Project> = (0..20).map(|i| default_project(&format!("P{i:02}"))).collect();
        let page = filter_and_paginate(&projects, &empty_query(), Some(5));
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total_items, 20);
        assert_eq!(page.nr_pages, 4);
    }

    #[test]
    fn empty_result_has_one_page() {
        let page = filter_and_paginate(&[], &empty_query(), None);
        assert_eq!(page.total_items, 0);
        assert_eq!(page.nr_pages, 1);
        assert_eq!(page.items.len(), 0);
    }

    #[test]
    fn to_query_string_empty() {
        assert_eq!(empty_query().to_query_string(), "");
    }
}

#[server]
pub async fn get_project(shortcode: String) -> Result<Option<Project>, ServerFnError> {
    use std::fs;
    use std::path::PathBuf;

    use dpe_core::cluster::ClusterRaw;
    use dpe_core::{CollectionRef, all_projects, get_data_dir};

    // Look up the base project from the in-memory cache — no disk scan needed.
    let Some(base) = all_projects().iter().find(|p| p.shortcode == shortcode) else {
        return Ok(None);
    };
    let mut project = base.clone();

    let data_path = PathBuf::from(get_data_dir());

    // Resolve clusters by reverse lookup through the clusters directory.
    let clusters_dir = data_path.join("clusters");
    project.clusters = fs::read_dir(&clusters_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry: std::fs::DirEntry| {
            let path = entry.path();
            if path.extension().and_then(|e: &std::ffi::OsStr| e.to_str()) != Some("json") {
                return None;
            }
            let json = fs::read_to_string(&path).ok()?;
            let raw: ClusterRaw = serde_json::from_str(&json).ok()?;
            if raw.projects.iter().any(|p| p == &shortcode) {
                Some(raw.into_ref())
            } else {
                None
            }
        })
        .collect();

    // Resolve collection IDs stored on the cached project.
    let collections_dir = data_path.join("collections");
    project.collections = project
        .collection_ids
        .iter()
        .filter_map(|id| {
            let path = collections_dir.join(format!("{}.json", id));
            fs::read_to_string(&path)
                .ok()
                .and_then(|json: String| serde_json::from_str::<CollectionRef>(&json).ok())
        })
        .collect();

    Ok(Some(project))
}
