//! How a refresh ended. A refusal is what the project causes, its data or its missing shipped
//! inputs; a failure is everything else (the transport, any other answer, the bridge itself).
//! Either leaves the project's tokens, and so what readers see, as they were.

/// How a refresh, or a removal, ended for one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The change check listed nothing; the tokens were left as they were.
    Unchanged,
    /// A new generation was written and both tokens name it.
    Refreshed,
    Refused(RefusalReason),
    Failed(FailureReason),
    /// The project left the allowlist and its tokens were removed. Only the nightly run's removal
    /// reports it, never a refresh.
    Removed,
}

/// Why the project was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// The project has no shipped-inputs directory, `<INPUTS_DIR>/<SHORTCODE>`.
    InputsMissing,
    /// The public guard found something the anonymous user may not view.
    PublicGuard,
    /// The dump breaks the interim format's closed world.
    ClosedWorld,
    /// The server answered `401` or `403`, to the export or to an anonymous file download.
    Unauthorized,
    /// A file the project's file mapping names is not a current, public text file or still
    /// image of the dump, or its stored name is not usable.
    FileMapping,
}

/// Why the refresh could not finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// The server's dump slot stayed busy with another caller's export.
    Busy,
    /// The server could not be reached, or the connection broke.
    Unreachable,
    /// A step exceeded its time limit.
    Timeout,
    /// The server answered something the bridge does not expect.
    Server,
    /// The server's export ended as failed.
    Export,
    /// The dump could not be read: a damaged zip, an entry the unpacking refuses, or a size cap.
    Dump,
    /// The project's shipped inputs are not usable.
    Inputs,
    /// Writing the generation or replacing the tokens failed.
    Store,
    /// The bridge itself failed: a panic or a broken invariant.
    Internal,
}

impl Outcome {
    /// The operator's refresh's exit status: `0` refreshed or unchanged, `2` refused, `1` failed.
    /// `Removed` never comes from a refresh and maps to `1` if it does.
    #[must_use]
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Unchanged | Self::Refreshed => 0,
            Self::Refused(_) => 2,
            Self::Failed(_) | Self::Removed => 1,
        }
    }

    /// The outcome as one word, for a log line and the outcome metric's label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Refreshed => "refreshed",
            Self::Refused(_) => "refused",
            Self::Failed(_) => "failed",
            Self::Removed => "removed",
        }
    }

    /// The reason of a refusal or failure, as one word, for a log line.
    #[must_use]
    pub fn reason(self) -> Option<&'static str> {
        match self {
            Self::Refused(why) => Some(why.as_str()),
            Self::Failed(why) => Some(why.as_str()),
            Self::Unchanged | Self::Refreshed | Self::Removed => None,
        }
    }
}

impl RefusalReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputsMissing => "inputs-missing",
            Self::PublicGuard => "public-guard",
            Self::ClosedWorld => "closed-world",
            Self::Unauthorized => "unauthorized",
            Self::FileMapping => "file-mapping",
        }
    }
}

impl FailureReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Unreachable => "unreachable",
            Self::Timeout => "timeout",
            Self::Server => "server",
            Self::Export => "export",
            Self::Dump => "dump",
            Self::Inputs => "inputs",
            Self::Store => "store",
            Self::Internal => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_each_outcome_maps_to_its_status() {
        let cases = [
            (Outcome::Unchanged, 0),
            (Outcome::Refreshed, 0),
            (Outcome::Refused(RefusalReason::PublicGuard), 2),
            (Outcome::Failed(FailureReason::Busy), 1),
            (Outcome::Removed, 1),
        ];
        for (outcome, code) in cases {
            assert_eq!(outcome.exit_code(), code, "{outcome:?}");
        }
    }

    /// The words are the outcome metric's label values, which alerts are defined on.
    #[test]
    fn test_as_str_and_reason_each_outcome_pinned() {
        let cases = [
            (Outcome::Unchanged, "unchanged", None),
            (Outcome::Refreshed, "refreshed", None),
            (Outcome::Removed, "removed", None),
            (
                Outcome::Refused(RefusalReason::InputsMissing),
                "refused",
                Some("inputs-missing"),
            ),
            (Outcome::Refused(RefusalReason::PublicGuard), "refused", Some("public-guard")),
            (Outcome::Refused(RefusalReason::ClosedWorld), "refused", Some("closed-world")),
            (Outcome::Refused(RefusalReason::Unauthorized), "refused", Some("unauthorized")),
            (Outcome::Refused(RefusalReason::FileMapping), "refused", Some("file-mapping")),
            (Outcome::Failed(FailureReason::Busy), "failed", Some("busy")),
            (Outcome::Failed(FailureReason::Unreachable), "failed", Some("unreachable")),
            (Outcome::Failed(FailureReason::Timeout), "failed", Some("timeout")),
            (Outcome::Failed(FailureReason::Server), "failed", Some("server")),
            (Outcome::Failed(FailureReason::Export), "failed", Some("export")),
            (Outcome::Failed(FailureReason::Dump), "failed", Some("dump")),
            (Outcome::Failed(FailureReason::Inputs), "failed", Some("inputs")),
            (Outcome::Failed(FailureReason::Store), "failed", Some("store")),
            (Outcome::Failed(FailureReason::Internal), "failed", Some("internal")),
        ];
        for (outcome, word, reason) in cases {
            assert_eq!((outcome.as_str(), outcome.reason()), (word, reason), "{outcome:?}");
        }
    }
}
