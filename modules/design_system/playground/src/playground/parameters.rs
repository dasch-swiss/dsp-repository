use serde::Deserialize;

use crate::playground::components::{get_component_by_name, ComponentInfo};
use crate::playground::error::{PlaygroundError, PlaygroundResult};

#[derive(Debug, Clone, Deserialize)]
pub struct PlaygroundParams {
    #[serde(default = "default_component")]
    pub component: String,
    pub variant: Option<String>,
    #[serde(default)]
    pub theme: Theme,
}

fn default_component() -> String {
    "button".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Light => write!(f, "light"),
            Theme::Dark => write!(f, "dark"),
        }
    }
}

impl PlaygroundParams {
    pub fn validate_self(self) -> PlaygroundResult<Self> {
        self.validate_impl()?;
        Ok(self)
    }

    fn validate_impl(&self) -> PlaygroundResult<()> {
        // Check if component exists
        let component_info = get_component_by_name(&self.component)
            .ok_or_else(|| PlaygroundError::InvalidComponent(self.component.clone()))?;

        // Check if variant exists for this component
        if let Some(variant) = &self.variant {
            let variant_exists = component_info.variants.iter().any(|v| v.value == *variant);

            if !variant_exists {
                return Err(PlaygroundError::InvalidVariant {
                    component: self.component.clone(),
                    variant: variant.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn to_query_string(&self) -> String {
        let mut params = vec![format!("component={}", self.component)];

        if let Some(variant) = &self.variant {
            params.push(format!("variant={}", variant));
        }

        params.push(format!("theme={}", self.theme));

        params.join("&")
    }

    pub fn get_component_info(&self) -> Option<ComponentInfo> {
        get_component_by_name(&self.component)
    }
}
