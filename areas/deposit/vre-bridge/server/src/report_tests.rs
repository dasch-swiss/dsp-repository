//! The metrics, read through an `SdkMeterProvider` whose periodic reader pushes to a test
//! exporter on `force_flush`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData, ResourceMetrics};
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
use vre_bridge_core::{FailureReason, Outcome, RawSettings, ServerId};

use crate::report::{OtelReport, METER_SCOPE, NO_SERVER};
use crate::test_support::Dirs;
use crate::{RefreshReport, RefreshSizes, Report, VreBridge};

/// One exported point: the instrument, its attributes, and its value (a histogram's count).
#[derive(Debug, Clone, PartialEq)]
struct Point {
    name: String,
    attributes: BTreeMap<String, String>,
    value: f64,
    /// A histogram's sum.
    sum: Option<f64>,
}

/// Keeps the points of the bridge's scope from the last export.
#[derive(Clone, Default)]
struct Exported(Arc<Mutex<Vec<Point>>>);

impl Exported {
    fn points(&self, name: &str) -> Vec<Point> {
        self.0.lock().unwrap().iter().filter(|p| p.name == name).cloned().collect()
    }
}

fn attributes<'a>(kvs: impl Iterator<Item = &'a opentelemetry::KeyValue>) -> BTreeMap<String, String> {
    kvs.map(|kv| (kv.key.to_string(), kv.value.to_string())).collect()
}

impl PushMetricExporter for Exported {
    fn export(&self, metrics: &ResourceMetrics) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let mut points = Vec::new();
        for scope in metrics.scope_metrics().filter(|s| s.scope().name() == METER_SCOPE) {
            for metric in scope.metrics() {
                let name = metric.name().to_owned();
                let mut point = |attributes, value, sum| {
                    points.push(Point { name: name.clone(), attributes, value, sum });
                };
                match metric.data() {
                    AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                        for p in sum.data_points() {
                            point(attributes(p.attributes()), p.value() as f64, None);
                        }
                    }
                    AggregatedMetrics::U64(MetricData::Gauge(gauge)) => {
                        for p in gauge.data_points() {
                            point(attributes(p.attributes()), p.value() as f64, None);
                        }
                    }
                    AggregatedMetrics::U64(MetricData::Histogram(histogram)) => {
                        for p in histogram.data_points() {
                            point(attributes(p.attributes()), p.count() as f64, Some(p.sum() as f64));
                        }
                    }
                    AggregatedMetrics::F64(MetricData::Histogram(histogram)) => {
                        for p in histogram.data_points() {
                            point(attributes(p.attributes()), p.count() as f64, Some(p.sum()));
                        }
                    }
                    other => panic!("unexpected data for {name}: {other:?}"),
                }
            }
        }
        *self.0.lock().unwrap() = points;
        std::future::ready(Ok(()))
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        Temporality::Cumulative
    }
}

fn provider() -> (SdkMeterProvider, Exported) {
    let exported = Exported::default();
    let reader = PeriodicReader::builder(exported.clone()).build();
    (SdkMeterProvider::builder().with_reader(reader).build(), exported)
}

fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

fn report(project: &str, outcome: Outcome, sizes: Option<RefreshSizes>) -> RefreshReport {
    RefreshReport {
        project: project.to_owned(),
        server: (outcome != Outcome::Removed).then(|| ServerId::parse("prod").unwrap()),
        outcome,
        duration: Duration::from_secs(90),
        sizes,
    }
}

#[test]
fn test_otel_report_refreshed_and_failed_counted_timed_and_sized() {
    let (provider, exported) = provider();
    let report_to = OtelReport::new(&provider.meter(METER_SCOPE));
    let sizes = RefreshSizes { dump: 3_000, snapshot: 2_000, media: 1_000 };
    report_to.refresh(&report("0803", Outcome::Refreshed, Some(sizes)));
    report_to.refresh(&report("083C", Outcome::Failed(FailureReason::Busy), None));
    provider.force_flush().unwrap();

    let refreshed = labels(&[("project", "0803"), ("server", "prod"), ("outcome", "refreshed")]);
    let failed = labels(&[("project", "083C"), ("server", "prod"), ("outcome", "failed")]);
    let mut outcomes = exported.points("vre_bridge.refresh.outcomes");
    outcomes.sort_by(|a, b| a.attributes.cmp(&b.attributes));
    assert_eq!(
        outcomes.iter().map(|p| (&p.attributes, p.value)).collect::<Vec<_>>(),
        [(&failed, 1.0), (&refreshed, 1.0)]
    );

    let durations = exported.points("vre_bridge.refresh.duration");
    assert_eq!(durations.len(), 2, "{durations:?}");
    for p in &durations {
        assert_eq!((p.value, p.sum), (1.0, Some(90.0)), "{p:?}");
    }

    let mut sizes = exported.points("vre_bridge.refresh.size");
    sizes.sort_by(|a, b| a.attributes.cmp(&b.attributes));
    assert_eq!(
        sizes.iter().map(|p| (&p.attributes, p.sum)).collect::<Vec<_>>(),
        [
            (&labels(&[("project", "0803"), ("kind", "dump")]), Some(3_000.0)),
            (&labels(&[("project", "0803"), ("kind", "media")]), Some(1_000.0)),
            (&labels(&[("project", "0803"), ("kind", "snapshot")]), Some(2_000.0)),
        ]
    );
}

#[test]
fn test_otel_report_removed_counted_with_no_server_and_untimed() {
    let (provider, exported) = provider();
    let report_to = OtelReport::new(&provider.meter(METER_SCOPE));
    report_to.refresh(&report("0803", Outcome::Removed, None));
    provider.force_flush().unwrap();

    let outcomes = exported.points("vre_bridge.refresh.outcomes");
    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    assert_eq!(
        outcomes[0].attributes,
        labels(&[("project", "0803"), ("server", NO_SERVER), ("outcome", "removed")])
    );
    assert!(exported.points("vre_bridge.refresh.duration").is_empty());
    assert!(exported.points("vre_bridge.refresh.size").is_empty());
}

fn active_gauge(raw: RawSettings) -> Vec<f64> {
    let (provider, exported) = provider();
    let bridge = VreBridge::from_raw(raw, &provider.meter(METER_SCOPE));
    drop(bridge);
    // After the bridge is gone: the callback outlives the gauge's handle.
    provider.force_flush().unwrap();
    exported.points("vre_bridge.active").iter().map(|p| p.value).collect()
}

#[test]
fn test_active_gauge_one_for_an_active_bridge_zero_for_an_inactive_one() {
    let dirs = Dirs::new();
    assert_eq!(active_gauge(dirs.raw("0803=prod")), [1.0]);
    assert_eq!(active_gauge(RawSettings::default()), [0.0]);
    let invalid = RawSettings {
        time_zone: Some("Mars/Olympus".to_owned()),
        ..dirs.raw("0803=prod")
    };
    assert_eq!(active_gauge(invalid), [0.0]);
}
