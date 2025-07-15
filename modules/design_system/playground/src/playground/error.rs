use std::fmt;

#[derive(Debug, Clone)]
pub enum PlaygroundError {
    InvalidComponent(String),
    InvalidVariant {
        component: String,
        variant: String,
    },
    #[allow(dead_code)]
    ParameterValidationError(String),
}

impl fmt::Display for PlaygroundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaygroundError::InvalidComponent(component) => {
                write!(f, "Invalid component: '{}'", component)
            }
            PlaygroundError::InvalidVariant { component, variant } => {
                write!(f, "Invalid variant '{}' for component '{}'", variant, component)
            }
            PlaygroundError::ParameterValidationError(msg) => {
                write!(f, "Parameter validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for PlaygroundError {}

pub type PlaygroundResult<T> = Result<T, PlaygroundError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_component_error_display() {
        let error = PlaygroundError::InvalidComponent("nonexistent".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Invalid component: 'nonexistent'");
    }

    #[test]
    fn test_invalid_component_error_empty_string() {
        let error = PlaygroundError::InvalidComponent("".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Invalid component: ''");
    }

    #[test]
    fn test_invalid_variant_error_display() {
        let error = PlaygroundError::InvalidVariant {
            component: "button".to_string(),
            variant: "nonexistent".to_string(),
        };
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Invalid variant 'nonexistent' for component 'button'");
    }

    #[test]
    fn test_invalid_variant_error_empty_strings() {
        let error = PlaygroundError::InvalidVariant { component: "".to_string(), variant: "".to_string() };
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Invalid variant '' for component ''");
    }

    #[test]
    fn test_parameter_validation_error_display() {
        let error = PlaygroundError::ParameterValidationError("Test validation message".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Parameter validation error: Test validation message");
    }

    #[test]
    fn test_parameter_validation_error_empty_message() {
        let error = PlaygroundError::ParameterValidationError("".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Parameter validation error: ");
    }

    #[test]
    fn test_all_error_variants_user_friendly_messages() {
        let invalid_component = PlaygroundError::InvalidComponent("mycomponent".to_string());
        let invalid_variant = PlaygroundError::InvalidVariant {
            component: "button".to_string(),
            variant: "myvariant".to_string(),
        };
        let validation_error = PlaygroundError::ParameterValidationError("Invalid input".to_string());

        // Messages should be user-friendly without technical jargon
        let component_msg = format!("{}", invalid_component);
        let variant_msg = format!("{}", invalid_variant);
        let validation_msg = format!("{}", validation_error);

        assert!(!component_msg.contains("Error"));
        assert!(!variant_msg.contains("Error"));
        assert!(validation_msg.contains("Parameter validation error"));

        // Messages should be specific and helpful
        assert!(component_msg.contains("mycomponent"));
        assert!(variant_msg.contains("button"));
        assert!(variant_msg.contains("myvariant"));
        assert!(validation_msg.contains("Invalid input"));
    }
}
