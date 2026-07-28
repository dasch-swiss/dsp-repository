//! OAI-PMH resumption tokens (flow control / paging).
//!
//! List verbs (`ListRecords`, `ListIdentifiers`) return at most [`DEFAULT_PAGE_SIZE`]
//! items per response. When more items remain, the response carries a
//! `<resumptionToken>` that the harvester passes back verbatim to fetch the next
//! page. The token is opaque to harvesters: it encodes the original request's
//! filter arguments plus the offset into the deterministic result list, so a
//! resumed request reproduces the exact same list and continues where it left off.
//!
//! The encoding is a base64url string (no padding) of a compact pipe-separated
//! record: `metadataPrefix|from|until|set|offset`. Empty optional fields are the
//! empty string. The token carries its own state, so harvesters cannot
//! meaningfully parse it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use crate::error::OaiError;

/// Default number of items per page in production. OAI-PMH imposes no fixed page
/// size; 100 keeps individual responses modest. Overridable per call (tests use a
/// small value so paging can be exercised without large fixtures), or via the
/// `DPE_OAI_PAGE_SIZE` env var (see [`page_size`]).
pub const DEFAULT_PAGE_SIZE: usize = 100;

pub fn page_size() -> usize {
    resolve_page_size(std::env::var("DPE_OAI_PAGE_SIZE").ok().as_deref())
}

/// Parses a page size from an optional raw value, ignoring anything that is not a
/// positive integer. Pure helper so the precedence is unit-testable without
/// touching the process-global env.
fn resolve_page_size(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_PAGE_SIZE)
}

/// The decoded state carried by a resumption token: the filter arguments of the
/// original request and the offset of the next item to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumptionCursor {
    pub metadata_prefix: String,
    pub from: Option<String>,
    pub until: Option<String>,
    pub set: Option<String>,
    pub offset: usize,
}

impl ResumptionCursor {
    /// Encodes the cursor into an opaque base64url token.
    pub fn encode(&self) -> String {
        let raw = format!(
            "{}|{}|{}|{}|{}",
            self.metadata_prefix,
            self.from.as_deref().unwrap_or(""),
            self.until.as_deref().unwrap_or(""),
            self.set.as_deref().unwrap_or(""),
            self.offset,
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    /// Decodes an opaque token back into a cursor.
    ///
    /// Returns [`OaiError::BadResumptionToken`] if the token is not valid
    /// base64url, is not UTF-8, has the wrong number of fields, carries an empty
    /// metadataPrefix, or has a non-numeric offset.
    pub fn decode(token: &str) -> Result<Self, OaiError> {
        let bytes = URL_SAFE_NO_PAD.decode(token).map_err(|_| OaiError::BadResumptionToken)?;
        let raw = String::from_utf8(bytes).map_err(|_| OaiError::BadResumptionToken)?;

        // Exactly five fields; the last is the offset. Any other shape is invalid.
        let fields: Vec<&str> = raw.split('|').collect();
        if fields.len() != 5 {
            return Err(OaiError::BadResumptionToken);
        }

        let metadata_prefix = fields[0];
        if metadata_prefix.is_empty() {
            return Err(OaiError::BadResumptionToken);
        }
        let offset: usize = fields[4].parse().map_err(|_| OaiError::BadResumptionToken)?;

        let empty_to_none = |s: &str| (!s.is_empty()).then(|| s.to_string());

        Ok(Self {
            metadata_prefix: metadata_prefix.to_string(),
            from: empty_to_none(fields[1]),
            until: empty_to_none(fields[2]),
            set: empty_to_none(fields[3]),
            offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_parses_a_valid_value() {
        assert_eq!(resolve_page_size(Some("250")), 250);
    }

    #[test]
    fn page_size_falls_back_when_absent_empty_zero_or_garbage() {
        assert_eq!(resolve_page_size(None), DEFAULT_PAGE_SIZE);
        assert_eq!(resolve_page_size(Some("")), DEFAULT_PAGE_SIZE);
        assert_eq!(resolve_page_size(Some("0")), DEFAULT_PAGE_SIZE);
        assert_eq!(resolve_page_size(Some("-5")), DEFAULT_PAGE_SIZE);
        assert_eq!(resolve_page_size(Some("lots")), DEFAULT_PAGE_SIZE);
    }

    fn cursor(offset: usize) -> ResumptionCursor {
        ResumptionCursor {
            metadata_prefix: "oai_dc".to_string(),
            from: Some("2020-01-01".to_string()),
            until: None,
            set: Some("project:0803".to_string()),
            offset,
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let original = cursor(42);
        let token = original.encode();
        let decoded = ResumptionCursor::decode(&token).expect("valid token");
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_all_optionals_absent() {
        let original = ResumptionCursor {
            metadata_prefix: "oai_datacite".to_string(),
            from: None,
            until: None,
            set: None,
            offset: 0,
        };
        let decoded = ResumptionCursor::decode(&original.encode()).expect("valid token");
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_all_optionals_present() {
        let original = ResumptionCursor {
            metadata_prefix: "oai_dc".to_string(),
            from: Some("2020-01-01".to_string()),
            until: Some("2021-12-31".to_string()),
            set: Some("cluster:cluster-001".to_string()),
            offset: 12345,
        };
        let decoded = ResumptionCursor::decode(&original.encode()).expect("valid token");
        assert_eq!(decoded, original);
    }

    #[test]
    fn token_is_opaque_and_url_safe() {
        // No pipe/plus/slash/equals leak through: it does not look like the raw
        // pipe-separated form and is safe in a URL query without escaping.
        let token = cursor(7).encode();
        assert!(!token.contains('|'));
        assert!(!token.contains('+'));
        assert!(!token.contains('/'));
        assert!(!token.contains('='));
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(ResumptionCursor::decode("not a token!!!").is_err());
    }

    #[test]
    fn decode_rejects_wrong_field_count() {
        // base64url of "oai_dc|only|three" — too few fields.
        let token = URL_SAFE_NO_PAD.encode(b"oai_dc|only|three");
        assert!(ResumptionCursor::decode(&token).is_err());
    }

    #[test]
    fn decode_rejects_non_numeric_offset() {
        let token = URL_SAFE_NO_PAD.encode(b"oai_dc||||notanumber");
        assert!(ResumptionCursor::decode(&token).is_err());
    }

    #[test]
    fn decode_rejects_empty_metadata_prefix() {
        let token = URL_SAFE_NO_PAD.encode(b"||||0");
        assert!(ResumptionCursor::decode(&token).is_err());
    }
}
