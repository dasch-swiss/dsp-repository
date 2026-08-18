//! Minimal Dataverse-compatible API for external harvesters.
//!
//! EOSC Data Commons harvests our OAI-PMH feed and then, per record, asks a
//! Dataverse Native API for that record's file-level download metadata. Rather
//! than deploy Dataverse, this crate reimplements the two endpoints their
//! pipeline actually calls:
//!
//! - `GET /api/datasets/:persistentId/versions/:latest-published?persistentId={id}` — the file list
//!   for one record ([`versions_handler`]).
//! - `GET /api/access/datafile/{id}` — the bytes for one file ([`datafile_handler`]).
//!
//! Nothing else of Dataverse is implemented, and none of it is authenticated:
//! both endpoints serve published, public metadata only.
//!
//! Most of the response is derived from the record data itself: MIME type,
//! creation date, and the download URL all come from a record's `file`. The
//! numeric `dataFile.id` is derived deterministically from the record IRI, since
//! in this format the id *is* the download address.
//!
//! Three fields — `filename`, `filesize`, and the checksum — have no source in
//! DPE or dsp-api yet and are served as **placeholders** (see
//! [`dpe_core::FilePlaceholders`]); a client verifying a checksum against the
//! downloaded bytes will find a mismatch. They will be supplied by a forthcoming
//! dsp-api file endpoint, which populates `FilePlaceholders` without changing the
//! wire types or handlers. See `docs/src/dpe/dataverse-api.md`.

mod dto;
mod error;
mod handlers;

pub use error::DataverseError;
pub use handlers::{datafile_handler, versions_handler};
