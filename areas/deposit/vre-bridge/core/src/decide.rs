//! Whether a project's refresh is forced, or asks the change check from which cutoff.

use std::fmt;

use chrono::{DateTime, TimeDelta, Utc};
use shared_stand_in::ReadyToken;
use vre_bridge_lift::{INTERIM_FORMAT_VERSION, TRANSFORMATION_VERSION};

use crate::settings::AllowlistEntry;

/// How far before T the change check starts.
const CUTOFF_MARGIN: TimeDelta = TimeDelta::seconds(60);

/// What the running bridge writes into a token; a token that records anything else is refreshed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
    pub interim_format_version: u32,
    pub transformation_version: u32,
    /// The project's shipped-input digest, as [`crate::inputs_digest`] computes it.
    pub inputs_digest: String,
}

impl Running {
    /// The running bridge's versions, with the project's shipped-input digest.
    #[must_use]
    pub fn new(inputs_digest: String) -> Self {
        Self {
            interim_format_version: INTERIM_FORMAT_VERSION,
            transformation_version: TRANSFORMATION_VERSION,
            inputs_digest,
        }
    }
}

/// Why a refresh is forced, whatever the change check would report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshReason {
    /// A subtree has no usable token: it is missing or broken, or the `sync` token's T cannot be
    /// read.
    MissingToken,
    /// A token names another server than the project's allowlist entry.
    OtherServer,
    /// A token records another interim format version than the running bridge's.
    InterimFormatVersion,
    /// A token records another transformation version than the running bridge's.
    TransformationVersion,
    /// A token records another shipped-input digest than the project's inputs have.
    InputsDigest,
    /// An operator asked.
    Operator,
}

impl RefreshReason {
    /// The reason as one word, for a log line.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingToken => "missing-token",
            Self::OtherServer => "other-server",
            Self::InterimFormatVersion => "interim-format-version",
            Self::TransformationVersion => "transformation-version",
            Self::InputsDigest => "inputs-digest",
            Self::Operator => "operator",
        }
    }
}

impl fmt::Display for RefreshReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a refresh does first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Refresh whatever the change check reports.
    Forced(RefreshReason),
    /// Ask the change check for resources changed at or after `cutoff`, T − 60 s at T's full
    /// precision; refresh only if it lists any.
    CheckChanges { cutoff: DateTime<Utc> },
}

/// What a refresh of `entry`'s project does first, given its tokens.
///
/// The caller passes `None` for a token that is missing or broken, so both count as missing.
/// Both tokens are compared with `running` and `entry`'s server; T is the `sync` token's.
#[must_use]
pub fn decide(
    sync: Option<&ReadyToken>,
    media: Option<&ReadyToken>,
    entry: &AllowlistEntry,
    running: &Running,
    operator: bool,
) -> Decision {
    let (Some(sync), Some(media)) = (sync, media) else {
        return Decision::Forced(RefreshReason::MissingToken);
    };
    for token in [sync, media] {
        let p = &token.provenance;
        let reason = if p.server != entry.server.as_str() {
            RefreshReason::OtherServer
        } else if p.interim_format_version != running.interim_format_version {
            RefreshReason::InterimFormatVersion
        } else if p.transformation_version != running.transformation_version {
            RefreshReason::TransformationVersion
        } else if p.inputs_digest != running.inputs_digest {
            RefreshReason::InputsDigest
        } else {
            continue;
        };
        return Decision::Forced(reason);
    }
    let cutoff = DateTime::parse_from_rfc3339(&sync.provenance.as_of)
        .ok()
        .and_then(|t| t.with_timezone(&Utc).checked_sub_signed(CUTOFF_MARGIN));
    match (operator, cutoff) {
        (true, _) => Decision::Forced(RefreshReason::Operator),
        (false, None) => Decision::Forced(RefreshReason::MissingToken),
        (false, Some(cutoff)) => Decision::CheckChanges { cutoff },
    }
}
