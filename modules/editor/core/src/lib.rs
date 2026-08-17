//! `editor-core` — pure domain types for the metadata editor.
//!
//! Mirrors `dpe-core`'s role: framework-free types shared by `editor-web` and
//! `editor-server`, with no Axum, Maud or database dependency.
//!
//! Deliberately empty at this point. The types it will own — the permissive
//! draft representation, the order-preserving multi-language map, and the
//! canonical `ProjectRaw` conversion — land with the draft model. The crate
//! exists from the scaffold so the `server → web → core` dependency direction
//! is established before anything can be written the wrong way round.
