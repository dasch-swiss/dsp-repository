//! The editor's two collection endpoints.
//!
//! The wire types are `editor_core`'s own, not a mirror of them: a member added
//! to the payload reaches the collector by recompiling, and the two cannot
//! drift on a shape neither side owns alone.

use std::time::Duration;

use editor_core::collection::{ApprovedRecordView, ApprovedRecordsResponse, CollectionReport};

/// How long one call may take. A collect run handles records in sequence, so a
/// hung editor must not hold the whole run open.
const TIMEOUT: Duration = Duration::from_secs(30);

/// The editor, as the collector talks to it.
pub struct Editor {
    base_url: String,
    /// Presented to the editor and to nothing else. The editor holds no
    /// credential granting write access to any GitHub repository, in either
    /// direction, so this token authenticates here and nowhere else.
    token: String,
    http: reqwest::blocking::Client,
}

impl Editor {
    pub fn new(base_url: &str, token: &str) -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|error| format!("could not build an HTTP client: {error}"))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            http,
        })
    }

    /// Every approved record the editor holds.
    ///
    /// Unauthenticated and unfiltered by contract, so there is no
    /// state-dependent selection here for a stale flag to hide.
    pub fn approved_records(&self) -> Result<Vec<ApprovedRecordView>, String> {
        let url = format!("{}/api/v1/approved-records", self.base_url);
        let response = self
            .http
            .get(&url)
            .send()
            .map_err(|error| format!("GET {url} failed: {error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("GET {url} returned {status}"));
        }

        let body: ApprovedRecordsResponse = response
            .json()
            .map_err(|error| format!("GET {url} returned a body this collector cannot read: {error}"))?;
        Ok(body.records)
    }

    /// Reports one record's outcome.
    ///
    /// One record per call, sent immediately after that record is handled, so a
    /// run that dies halfway still leaves a signal for every record it had
    /// already finished.
    pub fn report(&self, report: &CollectionReport) -> Result<(), String> {
        let url = format!("{}/api/v1/collection-report", self.base_url);
        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(report)
            .send()
            .map_err(|error| format!("POST {url} failed: {error}"))?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        // The body, not just the status: the editor rejects a report naming an
        // unknown or already-discarded record, and which it was is the whole
        // diagnosis.
        let body = response.text().unwrap_or_default();
        Err(format!("POST {url} returned {status}: {}", body.trim()))
    }
}
