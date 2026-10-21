//! What `Editor::start` does once per process: read the data directory and compare what it
//! holds against the local records. Nothing here is fatal.

use crate::reconcile::reconcile_published;
use crate::shell::TemporalTables;

/// The temporal-resolution tables, or empty ones without a data directory. Empty refuses
/// every free-text period at submit, the fail-safe direction.
pub(crate) fn load_temporal(data_dir: Option<&std::path::Path>) -> TemporalTables {
    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the temporal-resolution tables are empty, so every free-text temporalCoverage \
             entry reads as unresolvable and no submission carrying one can be made"
        );
        return TemporalTables::default();
    };
    let tables = TemporalTables {
        periods: shared_metadata::chronontology::load_from(data_dir),
        enrichment: shared_metadata::temporal_enrichment::load_from(data_dir),
    };
    tracing::info!(
        periods = tables.periods.len(),
        enrichment = tables.enrichment.len(),
        "temporal-resolution tables loaded"
    );
    tables
}

/// Read the published project set, reporting what did not load. Never fatal: an unset
/// `EDITOR_DATA_DIR` is a configured state, and a malformed file should not take down routes
/// that never read the projects.
pub(crate) fn load_published(data_dir: Option<&std::path::Path>) -> editor_core::published::PublishedProjects {
    use editor_core::published::PublishedProjects;

    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the published project set is empty, so the project list shows nothing and no \
             form can be pre-filled"
        );
        return PublishedProjects::default();
    };
    let (published, errors) = PublishedProjects::load_from(&data_dir.join("projects"));
    // One line per failure, so a bad file is findable by name rather than by
    // subtracting counts.
    for error in &errors {
        tracing::warn!(error = %error, "a published project could not be read");
    }
    if errors.is_empty() {
        tracing::info!(projects = published.len(), "published project set loaded");
    } else {
        tracing::warn!(
            projects = published.len(),
            failed = errors.len(),
            "published project set loaded with failures"
        );
    }
    published
}

/// The agent set, or an empty one without a data directory.
pub(crate) fn load_agents(data_dir: Option<&std::path::Path>) -> editor_core::agents::Agents {
    let Some(data_dir) = data_dir else {
        tracing::warn!(
            "no EDITOR_DATA_DIR: the agent set is empty, so every contributor and contact reference renders as a bare id and no submission naming one can be accepted"
        );
        return editor_core::agents::Agents::default();
    };
    let (agents, errors) =
        editor_core::agents::Agents::load_from(&data_dir.join("persons"), &data_dir.join("organizations"));
    for error in &errors {
        tracing::warn!(error = %error, "an agent could not be read");
    }
    if errors.is_empty() {
        tracing::info!(agents = agents.len(), "agent set loaded");
    } else {
        tracing::warn!(agents = agents.len(), failed = errors.len(), "agent set loaded with failures");
    }
    agents
}

/// Once per process: the moment a deployment starts is the moment its approved changes are
/// Online. Not fatal, see `reconcile`'s module docs.
pub(crate) async fn reconcile(
    store: &dyn editor_core::repository::EditorRepositories,
    published: &editor_core::published::PublishedProjects,
    agents: &editor_core::agents::Agents,
) {
    match reconcile_published(store, store, published, agents).await {
        // Two spelled-out arms rather than one parameterised call: `tracing`
        // resolves the level at compile time, so it cannot be a variable.
        Ok(summary) if summary.needs_attention() => tracing::warn!(
            projects.online = summary.online,
            projects.waiting = summary.waiting,
            projects.removed_upstream = summary.removed_upstream,
            records.unreadable = summary.unreadable,
            records.retry_failed = summary.retry_failed,
            proposals.retired = summary.proposals_retired,
            proposals.retire_failed = summary.proposals_retire_failed,
            "compared the published set against local records; some records need an RDU decision"
        ),
        Ok(summary) => tracing::info!(
            projects.online = summary.online,
            projects.waiting = summary.waiting,
            projects.removed_upstream = summary.removed_upstream,
            records.unreadable = summary.unreadable,
            records.retry_failed = summary.retry_failed,
            proposals.retired = summary.proposals_retired,
            proposals.retire_failed = summary.proposals_retire_failed,
            "compared the published set against local records"
        ),
        Err(e) => tracing::error!(
            error = %e,
            "could not compare the published set against local records; every project keeps its stored state and \
             the comparison is retried on the next start"
        ),
    }
}
