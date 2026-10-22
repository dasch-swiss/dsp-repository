//! `deposit-shell`: the Deposit Area's document shell (`areas/deposit/ADR-0004`). It depends on
//! no capability crate (ADR-0003), so it returns `Markup` and each caller
//! turns it into a response.
//!
//! The routes it links are string literals, a coupling to the capabilities' route tables that
//! no compiler checks: the 403 page links `/projects`, the area's home route; the header links
//! `/` and posts to `/logout`; [`sign_in::login_url`] builds `/login`, and
//! [`sign_in::safe_next`] refuses `/login` and `/login/…` as destinations. The document loads
//! `/logo.svg`, `/vendor/datastar.js` and `/telemetry.js`, which the server serves.

mod components;
pub mod pages;
pub mod sign_in;
pub mod view;

/// What a depositor is told if they find their way to an RDU-only page.
///
/// Deliberately says nothing about what the page holds. Unlike the project 403,
/// where the reader supplied the shortcode themselves, there is nothing here the
/// reader already knows.
pub const RDU_ONLY: &str = "This page is available to RDU members only.";
