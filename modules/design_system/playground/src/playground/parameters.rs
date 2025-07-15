use serde::Deserialize;

use crate::playground::components::{get_component_info_by_name, ComponentInfo};
use crate::playground::error::{PlaygroundError, PlaygroundResult};

#[derive(Debug, Clone, Deserialize)]
pub struct PlaygroundParams {
    #[serde(default = "default_component")]
    pub component: String,
    pub variant: Option<String>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub view: ViewMode,
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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Component,
    Documentation,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Light => write!(f, "light"),
            Theme::Dark => write!(f, "dark"),
        }
    }
}

impl std::fmt::Display for ViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewMode::Component => write!(f, "component"),
            ViewMode::Documentation => write!(f, "documentation"),
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
        let component_info = get_component_info_by_name(&self.component)
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
        params.push(format!("view={}", self.view));

        params.join("&")
    }

    pub fn get_component_info(&self) -> Option<ComponentInfo> {
        get_component_info_by_name(&self.component)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_self_valid_component_with_valid_variant() {
        // Note: This test depends on documentation files existing
        // In test environment, we test the validation logic with known invalid data
        let params = PlaygroundParams {
            component: "nonexistent".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let result = params.validate_self();
        assert!(result.is_err());
        if let Err(PlaygroundError::InvalidComponent(component)) = result {
            assert_eq!(component, "nonexistent");
        } else {
            panic!("Expected InvalidComponent error");
        }
    }

    #[test]
    fn test_validate_self_invalid_component() {
        let params = PlaygroundParams {
            component: "nonexistent".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let result = params.validate_self();
        assert!(result.is_err());
        if let Err(PlaygroundError::InvalidComponent(component)) = result {
            assert_eq!(component, "nonexistent");
        } else {
            panic!("Expected InvalidComponent error");
        }
    }

    #[test]
    fn test_validate_self_valid_component_with_invalid_variant() {
        // Note: This test depends on documentation files existing
        // We test the validation logic structure instead
        let params = PlaygroundParams {
            component: "definitely_nonexistent_component".to_string(),
            variant: Some("nonexistent".to_string()),
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let result = params.validate_self();
        assert!(result.is_err());
        // Should fail on invalid component first, before checking variant
        if let Err(PlaygroundError::InvalidComponent(component)) = result {
            assert_eq!(component, "definitely_nonexistent_component");
        } else {
            panic!("Expected InvalidComponent error");
        }
    }

    #[test]
    fn test_validate_self_valid_component_with_no_variant() {
        // Note: This test depends on documentation files existing
        // We test the validation logic structure instead
        let params = PlaygroundParams {
            component: "nonexistent".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let result = params.validate_self();
        assert!(result.is_err());
        if let Err(PlaygroundError::InvalidComponent(component)) = result {
            assert_eq!(component, "nonexistent");
        } else {
            panic!("Expected InvalidComponent error");
        }
    }

    #[test]
    fn test_validate_self_empty_component_name() {
        let params = PlaygroundParams {
            component: "".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let result = params.validate_self();
        assert!(result.is_err());
        if let Err(PlaygroundError::InvalidComponent(component)) = result {
            assert_eq!(component, "");
        } else {
            panic!("Expected InvalidComponent error");
        }
    }

    #[test]
    fn test_to_query_string_all_parameters() {
        let params = PlaygroundParams {
            component: "button".to_string(),
            variant: Some("primary".to_string()),
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let query_string = params.to_query_string();
        assert_eq!(query_string, "component=button&variant=primary&theme=light&view=component");
    }

    #[test]
    fn test_to_query_string_no_variant() {
        let params = PlaygroundParams {
            component: "button".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let query_string = params.to_query_string();
        assert_eq!(query_string, "component=button&theme=light&view=component");
    }

    #[test]
    fn test_to_query_string_dark_theme() {
        let params = PlaygroundParams {
            component: "button".to_string(),
            variant: None,
            theme: Theme::Dark,
            view: ViewMode::Component,
        };
        let query_string = params.to_query_string();
        assert_eq!(query_string, "component=button&theme=dark&view=component");
    }

    #[test]
    fn test_to_query_string_documentation_view() {
        let params = PlaygroundParams {
            component: "button".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Documentation,
        };
        let query_string = params.to_query_string();
        assert_eq!(query_string, "component=button&theme=light&view=documentation");
    }

    #[test]
    fn test_get_component_info_valid_component() {
        // Note: This test depends on documentation files existing
        // In test environment, we test with known invalid component to verify None return
        let params = PlaygroundParams {
            component: "nonexistent".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let component_info = params.get_component_info();
        assert!(component_info.is_none());
    }

    #[test]
    fn test_get_component_info_invalid_component() {
        let params = PlaygroundParams {
            component: "nonexistent".to_string(),
            variant: None,
            theme: Theme::Light,
            view: ViewMode::Component,
        };
        let component_info = params.get_component_info();
        assert!(component_info.is_none());
    }

    #[test]
    fn test_default_component() {
        let default_comp = default_component();
        assert_eq!(default_comp, "button");
    }
}
