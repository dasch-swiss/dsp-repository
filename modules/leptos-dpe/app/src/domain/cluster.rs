use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterRef {
    pub id: String,
    pub name: String,
    pub description: String,
}
