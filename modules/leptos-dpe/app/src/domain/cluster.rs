use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterRef {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ClusterRaw {
    pub id: String,
    pub name: String,
    pub description: HashMap<String, String>,
    #[serde(default)]
    pub projects: Vec<String>,
}

impl ClusterRaw {
    pub fn into_ref(self) -> ClusterRef {
        let description = self
            .description
            .get("en")
            .or_else(|| self.description.values().next())
            .cloned()
            .unwrap_or_default();
        ClusterRef { id: self.id, name: self.name, description }
    }
}
