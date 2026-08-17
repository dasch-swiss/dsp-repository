//! Browser telemetry: the beacon wire contract and the collector endpoint that
//! turns beacons into OTel metrics and structured logs.
//!
//! Shared by every DaSCH service that renders the beacon script. The `dpe-`
//! prefix is historical — DPE was the first consumer; the crate is not DPE-
//! specific and nothing in it reads DPE's data or configuration.

pub mod beacon;
pub mod collector;
pub mod origin;
pub mod page_url;
pub mod traceparent;
