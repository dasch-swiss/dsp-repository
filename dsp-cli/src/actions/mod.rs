//! Action layer — layer 2 of ADR-0008.
//!
//! Each action function orchestrates a `DspClient` and a `Renderer` to fulfil
//! one CLI verb. Action functions are the unit of action-level testing
//! (T2 seam from ADR-0009): inject a `MockDspClient` and a capturing
//! renderer, assert against the output.
//!
//! The directory shape mirrors the noun-group hierarchy: `vre::project`,
//! `vre::data_model`, `vre::resource_type`.

pub mod auth;
pub mod auth_state;
pub mod docs;
pub mod vre;
