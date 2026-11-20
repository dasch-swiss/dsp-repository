//! What a refresh reports once it has ended: the outcome line and the metrics, meter scope
//! [`METER_SCOPE`]. Every attribute value comes from the allowlist, a shortcode directory of a
//! subtree or a fixed set, so the series stay few.

use std::time::Duration;

use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::KeyValue;
use vre_bridge_core::{Outcome, ServerId};

/// The meter scope of the bridge's instruments.
pub const METER_SCOPE: &str = "vre_bridge";

/// The `server` attribute of a removal, whose allowlist entry is gone: not a server id
/// (`[a-z0-9-]{1,32}`), so it names no configured server.
pub(crate) const NO_SERVER: &str = "(none)";

/// Upper bucket bounds of `vre_bridge.refresh.duration`, in seconds: a second to two hours.
const DURATION_BOUNDS: [f64; 12] = [
    1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1200.0, 1800.0, 3600.0, 7200.0,
];

/// Upper bucket bounds of `vre_bridge.refresh.size`, in bytes: 1 MiB to 64 GiB, by fours.
const SIZE_BOUNDS: [f64; 9] = [
    1_048_576.0,
    4_194_304.0,
    16_777_216.0,
    67_108_864.0,
    268_435_456.0,
    1_073_741_824.0,
    4_294_967_296.0,
    17_179_869_184.0,
    68_719_476_736.0,
];

/// One ended refresh, or one removal. A busy dump slot is no outcome; only giving up on it is
/// reported, as a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshReport {
    pub project: String,
    /// The allowlist entry's server; `None` for [`Outcome::Removed`], whose entry is gone.
    pub server: Option<ServerId>,
    pub outcome: Outcome,
    pub duration: Duration,
    /// Set for [`Outcome::Refreshed`] only.
    pub sizes: Option<RefreshSizes>,
}

/// The bytes a successful refresh handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshSizes {
    /// The downloaded dump zip.
    pub dump: u64,
    /// The interim snapshot written to `sync`.
    pub snapshot: u64,
    /// Everything written to the `media` generation.
    pub media: u64,
}

/// Where a refresh's outcome goes.
pub trait Report: Send + Sync {
    fn refresh(&self, report: &RefreshReport);
}

/// The outcome line alone: info for a success, warning for a refusal, error for a failure.
#[derive(Debug, Clone, Copy, Default)]
pub struct LogReport;

impl Report for LogReport {
    fn refresh(&self, r: &RefreshReport) {
        // `tracing`'s level must be a constant, hence one call per level.
        macro_rules! outcome_line {
            ($level:ident, $message:literal) => {
                tracing::$level!(
                    project = %r.project,
                    server = r.server.as_ref().map(ServerId::as_str),
                    outcome = r.outcome.as_str(),
                    reason = r.outcome.reason(),
                    duration_secs = r.duration.as_secs_f64(),
                    dump_bytes = r.sizes.map(|s| s.dump),
                    snapshot_bytes = r.sizes.map(|s| s.snapshot),
                    media_bytes = r.sizes.map(|s| s.media),
                    $message
                )
            };
        }
        match r.outcome {
            Outcome::Unchanged | Outcome::Refreshed => outcome_line!(info, "refresh ended"),
            Outcome::Refused(_) => outcome_line!(warn, "refresh ended"),
            Outcome::Failed(_) => outcome_line!(error, "refresh ended"),
            Outcome::Removed => outcome_line!(warn, "project withdrawn: it is no longer allowlisted"),
        }
    }
}

/// The outcome line, as [`LogReport`] writes it, and the metrics: the counter
/// `vre_bridge.refresh.outcomes` and the histogram `vre_bridge.refresh.duration` by `project`,
/// `server` and `outcome` (a removal counts with `server` [`NO_SERVER`] and has no duration), and
/// for a refreshed project the histogram `vre_bridge.refresh.size` by `project` and `kind`
/// (`dump`, `snapshot`, `media`).
pub(crate) struct OtelReport {
    outcomes: Counter<u64>,
    duration: Histogram<f64>,
    size: Histogram<u64>,
}

impl OtelReport {
    /// The instruments on `meter`. Made after the process's meter provider is installed, since
    /// instruments made before stay no-ops.
    pub(crate) fn new(meter: &Meter) -> Self {
        Self {
            outcomes: meter
                .u64_counter("vre_bridge.refresh.outcomes")
                .with_description("Refreshes ended, and projects withdrawn, by outcome")
                .build(),
            duration: meter
                .f64_histogram("vre_bridge.refresh.duration")
                .with_description("How long a refresh took, from its start or first try to its end")
                .with_unit("s")
                .with_boundaries(DURATION_BOUNDS.to_vec())
                .build(),
            size: meter
                .u64_histogram("vre_bridge.refresh.size")
                .with_description("What a successful refresh handled: the dump, the snapshot, the media generation")
                .with_unit("By")
                .with_boundaries(SIZE_BOUNDS.to_vec())
                .build(),
        }
    }
}

impl Report for OtelReport {
    fn refresh(&self, r: &RefreshReport) {
        LogReport.refresh(r);
        let project = KeyValue::new("project", r.project.clone());
        let server = r.server.as_ref().map_or(NO_SERVER, ServerId::as_str).to_owned();
        let attributes = [
            project.clone(),
            KeyValue::new("server", server),
            KeyValue::new("outcome", r.outcome.as_str()),
        ];
        self.outcomes.add(1, &attributes);
        if r.outcome != Outcome::Removed {
            self.duration.record(r.duration.as_secs_f64(), &attributes);
        }
        if let Some(sizes) = r.sizes {
            for (kind, bytes) in [
                ("dump", sizes.dump),
                ("snapshot", sizes.snapshot),
                ("media", sizes.media),
            ] {
                self.size.record(bytes, &[project.clone(), KeyValue::new("kind", kind)]);
            }
        }
    }
}

/// The observable gauge `vre_bridge.active` on `meter`: `1` for an active bridge, `0` for an
/// inactive one, so a deployment whose bridge does nothing shows. Its callback stays registered
/// with the meter provider when the handle is dropped.
pub(crate) fn observe_active(meter: &Meter, active: bool) {
    let value = u64::from(active);
    meter
        .u64_observable_gauge("vre_bridge.active")
        .with_description(
            "1 when the VRE bridge is active, 0 when it is not configured or its configuration is invalid",
        )
        .with_callback(move |observer| observer.observe(value, &[]))
        .build();
}
