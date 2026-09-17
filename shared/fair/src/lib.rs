//! The FAIR exposure engine: one resolved graph per published object, and a
//! writer per representation that reads it.
//!
//! Nothing here knows where a page is rendered. The crate has no routes, no
//! Maud and no Axum: a writer returns a `String` or a `serde_json::Value`, and
//! the consuming service turns that into a response. It holds no path into a
//! service module either, which `.github/scripts/check-shared-paths.sh`
//! enforces — corpus-wide tests over the committed data stay in `dpe-api-oai`,
//! beside the data they read.
