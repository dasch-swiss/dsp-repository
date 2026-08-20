//! Browser telemetry: the beacon wire contract and the collector endpoint that
//! turns beacons into OTel metrics and structured logs.
//!
//! Shared by every DaSCH service that renders the beacon script, which is why
//! it lives under `modules/platform/` rather than inside a service module —
//! nothing here reads any one service's data or configuration.

pub mod beacon;
pub mod collector;
pub mod origin;
pub mod traceparent;
