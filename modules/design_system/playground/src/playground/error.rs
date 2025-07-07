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
