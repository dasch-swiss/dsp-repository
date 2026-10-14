//! `editor-web`: the metadata editor's views, handlers, routes and startup, behind the [`Editor`]
//! facade (`areas/deposit/ADR-0004`). The root names no `editor-core` type: it passes plain
//! [`EditorSettings`] in and merges the router out.
//!
//! The views are plain `fn(...) -> maud::Markup` functions: pages in `pages/`, the form in
//! `form/`. The document they are wrapped in is the area's, in `deposit-shell`.

pub mod entity;
#[cfg(test)]
mod fixtures;
pub mod form;
mod handlers;
mod page_url;
pub mod pages;
mod reconcile;
mod routes;
mod shell;
mod startup;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod traceparent;

use std::fmt;
use std::sync::Arc;

use editor_core::repository::EditorRepositories;
use subtle::ConstantTimeEq;

pub use crate::page_url::normalize_page_url;
pub use crate::routes::{RightmostXffKeyExtractor, COLLECTION_REPORT_PATH};
use crate::shell::EditorState;

#[derive(Clone, Debug)]
pub struct EditorSettings {
    pub css_href: String,
    /// `None` leaves the published set, the temporal tables and the agents empty.
    pub data_dir: Option<std::path::PathBuf>,
    pub collection_token: Option<CollectionToken>,
}

/// The collection-report bearer token, with a redacted `Debug`.
#[derive(Clone)]
pub struct CollectionToken(String);

impl CollectionToken {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Whether a presented token equals this one, in constant time.
    ///
    /// A `==` on strings returns at the first differing byte, so the time it takes
    /// reveals how long a shared prefix is. `ConstantTimeEq` on slices compares every
    /// byte, and reports unequal lengths as a mismatch without comparing.
    pub(crate) fn matches(&self, presented: &str) -> bool {
        presented.as_bytes().ct_eq(self.0.as_bytes()).into()
    }
}

impl fmt::Debug for CollectionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CollectionToken(redacted)")
    }
}

pub struct Editor {
    state: EditorState,
}

impl Editor {
    /// Read the published set, the temporal tables and the agents from the data directory, then
    /// compare the published set against the local records, in that order. None of it is fatal.
    pub async fn start(
        settings: EditorSettings,
        store: Arc<dyn EditorRepositories>,
        authenticator: Arc<dyn editor_ports::Authenticator>,
        directory: Arc<dyn editor_ports::UserDirectory>,
    ) -> Self {
        let published = Arc::new(startup::load_published(settings.data_dir.as_deref()));
        let temporal = Arc::new(startup::load_temporal(settings.data_dir.as_deref()));
        let agents = Arc::new(startup::load_agents(settings.data_dir.as_deref()));

        startup::reconcile(&*store, &published, &agents).await;

        Self {
            state: EditorState {
                css_href: settings.css_href,
                editor: store,
                authenticator,
                directory,
                collection_token: settings.collection_token,
                published,
                temporal,
                agents,
            },
        }
    }

    /// Untraced and without CSRF: the root wraps them in its layers.
    pub fn router(&self) -> axum::Router {
        routes::routes()
            .into_iter()
            .fold(axum::Router::new(), |router, (path, _, handlers)| router.route(path, handlers))
            .with_state(self.state.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Editor {
    /// None of [`Editor::start`]'s work: the committed corpora stand in for the data directory,
    /// and no collection token is configured.
    #[must_use]
    pub fn for_test(
        store: Arc<dyn EditorRepositories>,
        authenticator: Arc<dyn editor_ports::Authenticator>,
        directory: Arc<dyn editor_ports::UserDirectory>,
    ) -> Self {
        Self {
            state: EditorState {
                css_href: "/assets/app.css".to_string(),
                editor: store,
                authenticator,
                directory,
                collection_token: None,
                published: test_support::published_corpus(),
                temporal: test_support::temporal_tables(),
                agents: test_support::agent_corpus(),
            },
        }
    }
}
