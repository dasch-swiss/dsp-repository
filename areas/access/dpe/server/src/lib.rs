//! `dpe-server`: DPE's configuration, router and `validate` logic.
//!
//! This crate is a library only — `access-server` (`areas/access/server`) is
//! the composition root that owns `main`, the OTel layers and the untraced
//! routes. Nothing here may name a type from `dpe-core`, `dpe-web` or
//! `dpe-api-oai` in a `pub` signature: that boundary is what lets
//! `access-server` depend on this crate alone.

mod ark;
mod assets;
mod config;
#[cfg(feature = "dev")]
mod dev_reload;
pub(crate) mod downloads;
pub(crate) mod fragments;
mod metadata;
mod page_url;
mod router;
mod shell;
#[cfg(test)]
pub(crate) mod test_support;
mod traceparent;
mod validate;
mod view;

pub use config::DpeConfig;
pub use page_url::normalize_page_url;
pub use router::RightmostXffKeyExtractor;

/// DPE, built once from its config: the leaked corpus, the OAI-PMH state and
/// everything DPE's router needs. Cheap to clone (`DpeConfig`'s strings and
/// paths, copied into `AppState` and kept for `router()`, plus two `&'static`
/// references), so `access-server` can hand a clone to `spawn_blocking` for
/// [`Dpe::warm`] without an `Arc`.
#[derive(Clone)]
pub struct Dpe {
    state: shell::AppState,
    oai_state: dpe_api_oai::OaiState,
    config: DpeConfig,
}

impl Dpe {
    /// Builds the corpus and DPE's state from its config. The one production
    /// `Box::leak` of a `Corpus` lives here, so the composition root never
    /// names `Corpus`. Each call leaks one corpus: call it once per process
    /// (tests excepted, which leak one per fixture). Does no corpus I/O (the corpus loads lazily;
    /// the only read is resolving the hashed CSS href in `public_dir`), so startup is
    /// not held up by the record load; [`Dpe::warm`] does that.
    pub fn new(config: &DpeConfig) -> Self {
        // The one corpus this process builds, leaked so every handler and view can
        // hold a `&'static Corpus` for the process's lifetime.
        let corpus: &'static dpe_core::Corpus = Box::leak(Box::new(dpe_core::Corpus::new(dpe_core::CorpusSettings {
            data_dir: config.data_dir.to_str().expect("data_dir path must be valid UTF-8").to_string(),
            // The same directory ServeDir serves, so a cover present under it is reachable at
            // its URL. dpe-core scans it to resolve cover presence at render time.
            public_dir: config
                .public_dir
                .to_str()
                .expect("public_dir path must be valid UTF-8")
                .to_string(),
            ark_resolver_base_url: config.ark_resolver_base_url.clone(),
        })));

        // The OAI-PMH base URL, emitted as baseURL / <request>: normalised once here, handed to
        // the OAI router as state, and copied into `AppState.oai_base_url` below.
        let oai_state = dpe_api_oai::OaiState::new(&config.oai_base_url, corpus);
        tracing::info!(oai_base_url = %oai_state.base_url, "OAI-PMH base URL set");

        let state = shell::AppState {
            fathom_site_id: config.fathom_site_id.clone(),
            css_href: assets::resolve_css_href(&config.public_dir),
            public_base_url: config.public_base_url.clone(),
            oai_base_url: oai_state.base_url.clone(),
            ark_resolver_base_url: config.ark_resolver_base_url.clone(),
            show_placeholder_values: config.show_placeholder_values,
            corpus,
        };

        Dpe { state, oai_state, config: config.clone() }
    }

    /// DPE's own routes: pages, fragments, JSON, `/dpe/oai` with its limiter,
    /// `/ark:/` when configured, and the `ServeDir` fallback — un-layered by
    /// OTel; the composition root applies those layers over this router's
    /// output.
    pub fn router(&self) -> axum::Router {
        let rate_limited = router::rate_limited_router(&self.config, self.oai_state.clone());
        let app = router::build_router(self.state.clone(), &self.config.public_dir, rate_limited);

        // Dev-only browser live-reload (`dev` feature): wraps DPE's routes. It watches DPE's
        // public dir, so it belongs here rather than at the composition root; the composition
        // root's OTel layers go on top of this router's output, so this ends up inside them.
        #[cfg(feature = "dev")]
        let app = dev_reload::apply(app, &self.config.public_dir);

        app
    }

    /// Blocking: loads the record cache up front, so the first request does
    /// not pay for it. Run it off the async runtime (`spawn_blocking`).
    pub fn warm(&self) {
        self.state.corpus.warm();
    }
}

/// The `validate` subcommand's logic: checks every data file under `data_dir`
/// and reports counts and errors.
pub fn validate(data_dir: std::path::PathBuf) -> std::process::ExitCode {
    validate::validate(data_dir)
}
