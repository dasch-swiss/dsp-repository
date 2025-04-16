use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Message(String),
    Msg(&'static str),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Message(msg) => write!(f, "{}", msg),
            AppError::Msg(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}
