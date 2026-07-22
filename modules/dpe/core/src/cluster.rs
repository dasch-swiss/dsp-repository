use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterRef {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClusterRaw {
    pub id: String,
    pub name: String,
    pub description: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub pid: Option<String>,
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
