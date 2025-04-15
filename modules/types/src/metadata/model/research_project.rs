use regex::Regex;
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

#[derive(Debug, Default, Clone, PartialEq, Hash, Eq)]
pub struct Shortcode(String);
impl Shortcode {
    pub fn as_string(&self) -> String {
        self.0.to_string()
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
