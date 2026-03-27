// Domain types (Project, ProjectRaw, etc.) are now in dpe-core.
// This file retains only ProjectQuery which requires the Leptos Params derive.

pub use dpe_core::project::{
    AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo, License,
    Project, ProjectRaw, ProjectStatus, Publication, TemporalCoverage, ACCESS_RIGHTS_VALUES,
};

use leptos_router::params::Params;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, leptos::Params, PartialEq, Default)]
pub struct ProjectQuery {
    pub ongoing: Option<bool>,
    pub finished: Option<bool>,
    pub search: Option<String>,
    pub page: Option<i32>,
    pub type_of_data: Option<String>,
    pub data_language: Option<String>,
    pub access_rights: Option<String>,
    pub dialog: Option<bool>,
}

impl ProjectQuery {
    pub fn ongoing(&self) -> bool {
        self.ongoing.unwrap_or(false)
    }

    pub fn finished(&self) -> bool {
        self.finished.unwrap_or(false)
    }

    #[cfg(feature = "ssr")]
    pub fn search(&self) -> String {
        self.search.clone().unwrap_or_default()
    }

    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1)
    }

    pub fn type_of_data(&self) -> Vec<String> {
        self.type_of_data
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn data_language(&self) -> Vec<String> {
        self.data_language
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn access_rights(&self) -> Vec<String> {
        self.access_rights
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn with_page(self, page: i32) -> Self {
        Self { page: Some(page), ..self }
    }

    /// Return a new `ProjectQuery` with the given status param toggled, page reset to 1.
    pub fn with_status_toggled(&self, param: &str) -> Self {
        let ongoing = self.ongoing();
        let finished = self.finished();
        Self {
            ongoing: Some(if param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if param == "finished" { !finished } else { finished }),
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Build access rights filter items: `(label, checked, href)` for each access rights value.
    pub fn access_rights_filter_items(&self) -> Vec<(String, bool, String)> {
        ACCESS_RIGHTS_VALUES
            .iter()
            .map(|&v| {
                let checked = self.access_rights().contains(&v.to_string());
                let href = format!("/projects{}", self.with_access_rights_toggled(v).to_query_string());
                (v.to_string(), checked, href)
            })
            .collect()
    }

    /// Build status filter items: `(label, checked, href)` for "Ongoing" and "Finished".
    pub fn status_filter_items(&self) -> Vec<(String, bool, String)> {
        [
            ("ongoing", "Ongoing", self.ongoing()),
            ("finished", "Finished", self.finished()),
        ]
        .iter()
        .map(|(param, label, checked)| {
            let href = format!("/projects{}", self.with_status_toggled(param).to_query_string());
            (label.to_string(), *checked, href)
        })
        .collect()
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `type_of_data`, page reset to 1.
    pub fn with_type_of_data_toggled(&self, value: &str) -> Self {
        let mut selected = self.type_of_data();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: if selected.is_empty() { None } else { Some(selected.join(",")) },
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `data_language`, page reset to 1.
    pub fn with_data_language_toggled(&self, value: &str) -> Self {
        let mut selected = self.data_language();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: if selected.is_empty() { None } else { Some(selected.join(",")) },
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `access_rights`, page reset to 1.
    pub fn with_access_rights_toggled(&self, value: &str) -> Self {
        let mut selected = self.access_rights();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: if selected.is_empty() { None } else { Some(selected.join(",")) },
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `dialog` set to `open`, preserving all other fields.
    pub fn with_dialog(self, open: bool) -> Self {
        Self { dialog: if open { Some(true) } else { None }, ..self }
    }

    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(true) = self.ongoing {
            parts.push("ongoing=true".to_string());
        }
        if let Some(true) = self.finished {
            parts.push("finished=true".to_string());
        }
        if let Some(ref search) = self.search {
            if !search.is_empty() {
                parts.push(format!("search={}", urlencoding::encode(search)));
            }
        }
        if let Some(page) = self.page {
            if page > 1 {
                parts.push(format!("page={}", page));
            }
        }
        if let Some(ref type_of_data) = self.type_of_data {
            if !type_of_data.is_empty() {
                parts.push(format!("type_of_data={}", type_of_data));
            }
        }
        if let Some(ref data_language) = self.data_language {
            if !data_language.is_empty() {
                parts.push(format!("data_language={}", data_language));
            }
        }
        if let Some(ref access_rights) = self.access_rights {
            if !access_rights.is_empty() {
                parts.push(format!("access_rights={}", access_rights));
            }
        }
        if let Some(true) = self.dialog {
            parts.push("dialog=true".to_string());
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }
}
