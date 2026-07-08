//! dpe-web: the DPE view layer.
//!
//! Pages and components are plain `fn(...) -> maud::Markup`. The `dpe-server`
//! binary composes them into full HTML responses (shell, `<head>`, routing) and
//! drives the Datastar SSE fragments. There is no client-side WASM or hydration.

#![recursion_limit = "256"]

pub mod components;
pub mod domain;
pub mod pages;

#[cfg(test)]
pub(crate) mod test_support;
