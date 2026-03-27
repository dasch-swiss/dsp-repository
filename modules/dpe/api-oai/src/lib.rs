//! OAI-PMH 2.0 Data Provider implementation.
//!
//! This crate implements the OAI-PMH 2.0 protocol for exposing Research Projects
//! and Project Clusters as harvestable metadata records.

mod error;
mod handlers;
mod metadata;
mod xml;

pub use handlers::oai_handler;

/// Returns an Axum router for the OAI-PMH endpoint.
pub fn router() -> axum::Router {
    use axum::routing::get;
    axum::Router::new().route("/oai", get(oai_handler))
}
