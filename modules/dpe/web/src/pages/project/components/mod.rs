//! Project page components.
//!
//! ## Direct in-process lookups in `OrganizationName`, `Person`, `AffiliationName`, `EntityName`
//!
//! These four components call `dpe_core::load_person` / `dpe_core::load_organization`
//! synchronously inside the component body, instead of wrapping them in a
//! `Resource::new` + `<Suspense>`. The data already lives in the in-process
//! `OnceLock<HashMap>` caches in `dpe-core`, so adding an async layer added zero
//! benefit and all the lifecycle risk: a typical project page fanned out to 7-11
//! concurrent Resources per request (often the same id resolved 2-3 times),
//! which caused memory spikes and timing-dependent disposal panics in
//! `reactive_graph` under production load.
//!
//! ## wasm32 stubs
//!
//! `dpe_core::load_*` are gated `#[cfg(not(target_arch = "wasm32"))]` (they read
//! from disk-backed caches). DPE renders SSR-only — see
//! `modules/dpe/CLAUDE.md` — but cargo-leptos still compiles `dpe-web` for
//! `wasm32-unknown-unknown` because it is the lib-package, so each affected
//! component pairs its non-wasm body with an inert `#[cfg(target_arch = "wasm32")]`
//! stub that is never actually rendered.

pub mod breadcrumb;
pub mod contributor;
pub mod copy_button;
pub mod description;
pub mod info_card;
pub mod organization_name;
pub mod person;
pub mod project_details;
pub mod project_details_tabs;
pub mod project_header;
pub mod project_sidebar;
pub mod publications_section;
