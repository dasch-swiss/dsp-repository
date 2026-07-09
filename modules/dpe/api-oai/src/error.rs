//! OAI-PMH error codes and error response handling.

/// OAI-PMH protocol error codes as defined in the specification.
#[derive(Debug, Clone)]
pub enum OaiError {
    /// Unrecognized or missing verb argument.
    BadVerb,
    /// Missing required argument, invalid argument, or repeated argument.
    BadArgument(String),
    /// The value of the resumptionToken argument is invalid or expired.
    BadResumptionToken,
    /// The metadata format identified by metadataPrefix is not supported.
    CannotDisseminateFormat,
    /// The identifier does not exist in this repository.
    IdDoesNotExist,
    /// No records match the request criteria.
    NoRecordsMatch,
}

impl OaiError {
    /// Returns the OAI-PMH error code string.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadVerb => "badVerb",
            Self::BadArgument(_) => "badArgument",
            Self::BadResumptionToken => "badResumptionToken",
            Self::CannotDisseminateFormat => "cannotDisseminateFormat",
            Self::IdDoesNotExist => "idDoesNotExist",
            Self::NoRecordsMatch => "noRecordsMatch",
        }
    }

    /// Returns a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            Self::BadVerb => "Illegal OAI verb".to_string(),
            Self::BadArgument(msg) => msg.clone(),
            Self::BadResumptionToken => "The resumptionToken is not supported by this repository".to_string(),
            Self::CannotDisseminateFormat => {
                "The metadata format identified by metadataPrefix is not supported".to_string()
            }
            Self::IdDoesNotExist => "The identifier argument is unknown or illegal in this repository".to_string(),
            Self::NoRecordsMatch => "No records match the request criteria".to_string(),
        }
    }
}
