//! OAI-PMH 2.0 Data Provider implementation.
//!
//! This module implements the OAI-PMH 2.0 protocol for exposing Research Projects
//! and Project Clusters as harvestable metadata records.

mod error;
mod handlers;
mod metadata;
mod xml;

pub use handlers::oai_handler;
