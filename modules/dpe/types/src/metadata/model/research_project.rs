use std::fmt::{Display, Formatter};

use regex::Regex;
use serde::Deserialize;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCluster {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchProject {
    pub shortcode: Shortcode,
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Hash, Eq, Deserialize)]
pub struct Shortcode(String);
impl Shortcode {
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl Display for Shortcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Shortcode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_uppercase();
        Shortcode::try_from(value)
    }
}

impl TryFrom<String> for Shortcode {
    type Error = AppError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let regex: Regex = Regex::new(r"^[A-F0-9]{4}$").expect("Valid regex");
        let value = value.to_uppercase();
        if !regex.is_match(&value) {
            Err(AppError::Msg("Shortcode must be a 4 character hexadecimal string"))
        } else {
            Ok(Shortcode(value))
        }
    }
}
#[test]
fn test_try_from_shortcode() {
    assert!(Shortcode::try_from("000F".to_string()).is_ok());
    assert!(Shortcode::try_from("12345".to_string()).is_err());
    assert!(Shortcode::try_from("000G".to_string()).is_err());
}
