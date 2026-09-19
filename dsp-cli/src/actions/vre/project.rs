//! Actions for `dsp vre project { list | describe | dump }`.
//!
//! Phase 3 added `dump`; Phase 4 added `list` (the first read command).
//! Phase 5 added `describe` (the first single-object read command).
//! See dsp-cli/ADR-0008.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::actions::auth_state::read_auth_state;
use crate::cli::{ProjectDescribeArgs, ProjectDumpArgs, ProjectListArgs};
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;
use crate::model::{CreateDumpOutcome, DumpStatus};
use crate::render::progress::ProgressReporter;
use crate::render::{DumpDeleteOutcome, DumpEvent, DumpOutcome, MetaContext, ProjectListView, Renderer};

/// List all projects on the DSP server.
///
/// Authentication is optional (public endpoint per PRD AC 2). Reads `DSP_TOKEN`
/// from the environment (env wins over cache per dsp-cli/ADR-0007), and delegates all
/// work to `run_list_impl` with injectable seams for deterministic testing.
pub fn list(
    args: &ProjectListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_list_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `list` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `list` entry point before calling
///   this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in production to use the
///   default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. This differs deliberately from
/// `dump`, which requires auth and propagates errors. For a public endpoint, a
/// corrupt or missing `auth.toml` must still list anonymously (PRD AC 2).
fn run_list_impl(
    args: &ProjectListArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            crate::util::warn_auth_cache_load_failed(&e, "falling back to anonymous for project list");
            AuthCache::default()
        }
    };

    // ── 2. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 3. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 4. Fetch projects ─────────────────────────────────────────────────────
    let mut projects = client.list_projects(&cfg.server, token)?;

    // ── 5. Capture total BEFORE filtering ────────────────────────────────────
    let total = projects.len();

    // ── 6. Apply --filter (case-insensitive substring) ────────────────────────
    if let Some(ref f) = args.filter {
        let lower = f.to_lowercase();
        projects.retain(|p| {
            p.shortcode.to_lowercase().contains(&lower)
                || p.shortname.to_lowercase().contains(&lower)
                || p.longname.as_deref().unwrap_or("").to_lowercase().contains(&lower)
        });
    }

    // ── 7. Sort surviving projects ascending by shortcode ─────────────────────
    projects.sort_by(|a, b| a.shortcode.cmp(&b.shortcode));

    // ── 8. Build view + meta and render ──────────────────────────────────────
    let view = ProjectListView { items: projects, total, filter: args.filter.clone() };
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.projects(&view, &meta)
}

/// Describe a single DSP project.
///
/// Authentication is optional (public endpoint per dsp-cli/ADR-0007). Reads `DSP_TOKEN`
/// from the environment (env wins over cache), and delegates all work to
/// `run_describe_impl` with injectable seams for deterministic testing.
pub fn describe(
    args: &ProjectDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    run_describe_impl(args, cfg, client, renderer, env_token, None)
}

/// Internal entry point for `describe` with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `describe` entry point before
///   calling this, so tests never touch process env).
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in production to use the
///   default `~/.config/dsp-cli/auth.toml`.
///
/// **Auth-optional:** a cache-load failure ALWAYS falls back to an empty cache
/// with a `tracing::warn!` — NEVER returns `Err`. Project metadata is public
/// (dsp-cli/ADR-0007), so a corrupt or missing `auth.toml` must still describe anonymously.
fn run_describe_impl(
    args: &ProjectDescribeArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    env_token: Option<String>,
    cache_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    // ── 1. --project required (fail-fast, BEFORE any cache/IO) ───────────────
    let project = args
        .project
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string()))?;

    // ── 2. Load cache (auth-optional: failures fall back to empty cache) ──────
    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) => {
            crate::util::warn_auth_cache_load_failed(&e, "falling back to anonymous for project describe");
            AuthCache::default()
        }
    };

    // ── 3. Resolve token (optional) ───────────────────────────────────────────
    let resolved = resolve_token(env_token, &cache, &cfg.server);
    let token = resolved.as_ref().map(|r| r.token.as_str());

    // ── 4. Build auth-state disclosure string ─────────────────────────────────
    let auth_state = read_auth_state(resolved.as_ref(), &cache, &cfg.server);

    // ── 5. Fetch project detail ───────────────────────────────────────────────
    let detail = client.describe_project(&cfg.server, project, token)?;

    // ── 6. Build meta and render ──────────────────────────────────────────────
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state,
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    renderer.project_describe(&detail, &meta)
}

/// Trigger, poll, download, and optionally clean up a project dump.
///
/// Reads `DSP_TOKEN` from the environment (env wins over cache per dsp-cli/ADR-0007),
/// uses the real system clock for the default output filename, and delegates
/// all work to `run_impl` with injectable seams for deterministic testing.
///
/// Requires a system-administrator token. Returns `Diagnostic::AuthRequired`
/// when no token is available.
pub fn dump(
    args: &ProjectDumpArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), Diagnostic> {
    let env_token = std::env::var("DSP_TOKEN").ok();
    let cwd =
        std::env::current_dir().map_err(|e| Diagnostic::Io(format!("could not determine current directory: {e}")))?;
    run_impl(
        args,
        cfg,
        client,
        renderer,
        reporter,
        env_token,
        &|d| std::thread::sleep(d),
        Utc::now(),
        None,
        &cwd,
    )
}

/// The operational mode for `dsp vre project dump`, derived from clap args.
///
/// Derived as:
/// `if args.delete { Delete } else if args.replace { Replace } else { Default }`.
///
/// Clap enforces that `--replace` and `--delete` are mutually exclusive (Step 16),
/// so the both-true case is impossible at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DumpMode {
    /// Default: adopt an existing dump if present, otherwise create fresh.
    Default,
    /// `--replace`: delete any existing dump and create a fresh one.
    Replace,
    /// `--delete`: remove the existing dump without downloading.
    Delete,
}

/// Internal entry point with injectable seams for testing.
///
/// - `env_token`: the `DSP_TOKEN` env value (read by the public `dump` entry point before calling
///   this, so tests never touch process env).
/// - `sleeper`: a `Fn(Duration)` called between poll attempts; tests use a no-op `|_| {}` so the
///   logical clock advances without real wall-clock time.
/// - `now`: used only to build the default output filename; the poll-loop timeout is an explicit
///   logical accumulator independent of this.
/// - `cache_path`: `Some(path)` in tests to use a temp auth cache; `None` in production to use the
///   default `~/.config/dsp-cli/auth.toml`.
/// - `cwd`: the base directory for the default output path when `--output` is not specified. The
///   public `dump` entry point passes the real process CWD (obtained via
///   `std::env::current_dir()`); tests pass an explicit `TempDir` path so they never mutate
///   process-global state.
#[allow(clippy::too_many_arguments)]
fn run_impl(
    args: &ProjectDumpArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    reporter: &mut dyn ProgressReporter,
    env_token: Option<String>,
    sleeper: &dyn Fn(Duration),
    now: DateTime<Utc>,
    cache_path: Option<&Path>,
    cwd: &Path,
) -> Result<(), Diagnostic> {
    // ── 1. Resolve token (fail fast) ──────────────────────────────────────────
    // dsp-cli/ADR-0007: a non-blank DSP_TOKEN wins over the cache. A corrupt/unreadable
    // auth.toml must not mask the env token — tolerate cache-load failures only
    // when the env token would win (mirrors auth::status).
    let env_token_would_win = env_token.as_deref().map(str::trim).map(|s| !s.is_empty()).unwrap_or(false);

    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) if env_token_would_win => {
            crate::util::warn_auth_cache_load_failed(&e, "DSP_TOKEN is set, falling through to env token");
            AuthCache::default()
        }
        Err(e) => return Err(e),
    };

    let resolved = resolve_token(env_token, &cache, &cfg.server).ok_or_else(|| {
        Diagnostic::AuthRequired(
            "dsp vre project dump requires a system-administrator token; \
run `dsp auth login --server <s>` or set DSP_TOKEN"
                .to_string(),
        )
    })?;
    let token = resolved.token.clone();

    // ── 2. --project required ─────────────────────────────────────────────────
    let project = args
        .project
        .as_deref()
        .ok_or_else(|| Diagnostic::Usage("--project <shortcode|shortname|IRI> is required".to_string()))?;

    // ── 3. Derive mode from args ──────────────────────────────────────────────
    let mode = if args.delete {
        DumpMode::Delete
    } else if args.replace {
        DumpMode::Replace
    } else {
        DumpMode::Default
    };

    // ── 4. Staged overwrite guard (only when downloading) ─────────────────────
    // Delete mode neither downloads nor produces an output file — skip the guard.
    // For Default/Replace: check for an explicit collision BEFORE any server call.
    let explicit_output = if mode != DumpMode::Delete {
        args.output.clone()
    } else {
        None
    };

    if let Some(ref path) = explicit_output
        && path.exists()
        && !args.force
    {
        return Err(Diagnostic::Usage(format!(
            "refusing to overwrite {path}; pass --force",
            path = path.display()
        )));
    }

    // ── 5. Resolve project (always — every mode needs the IRI) ───────────────
    let proj = client.resolve_project(&cfg.server, project)?;

    // ── 6. Compute output path (Download modes only) ──────────────────────────
    let output_path: Option<PathBuf> = if mode != DumpMode::Delete {
        let p = match explicit_output {
            Some(p) => p,
            None => {
                let default = default_output_path(cwd, &proj.shortcode, now);
                if default.exists() && !args.force {
                    return Err(Diagnostic::Usage(format!(
                        "refusing to overwrite {path}; pass --force",
                        path = default.display()
                    )));
                }
                default
            }
        };
        Some(p)
    } else {
        None
    };

    // ── 7. Create (probe) the dump ────────────────────────────────────────────
    let create_outcome = client.create_project_dump(&cfg.server, &proj.iri, args.skip_assets, &token)?;

    // ── 8. Build MetaContext for the final render ─────────────────────────────
    // Use the shared dsp-cli/ADR-0007 helper so _meta.auth uses the same uniform
    // vocabulary as all other commands (presence/origin, not validity).
    let meta = MetaContext {
        server_label: cfg.server.clone(),
        auth_state: read_auth_state(Some(&resolved), &cache, &cfg.server),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };

    // ── 9. Dispatch by mode + create outcome ──────────────────────────────────
    match mode {
        DumpMode::Default => handle_default(
            create_outcome,
            args,
            cfg,
            client,
            renderer,
            reporter,
            &token,
            &proj.iri,
            output_path.ok_or_else(|| Diagnostic::Internal("output_path unexpectedly None in Default mode".into()))?,
            &meta,
            sleeper,
        ),
        DumpMode::Replace => handle_replace(
            create_outcome,
            args,
            cfg,
            client,
            renderer,
            reporter,
            &token,
            &proj.iri,
            args.skip_assets,
            output_path.ok_or_else(|| Diagnostic::Internal("output_path unexpectedly None in Replace mode".into()))?,
            &meta,
            sleeper,
        ),
        DumpMode::Delete => handle_delete(create_outcome, cfg, client, renderer, reporter, &token, &proj.iri, &meta),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Map a `CreateDumpOutcome` from the second `create_project_dump` call in
/// `handle_replace` (after an existing dump was deleted) to `(id, created_at)`.
///
/// Both the `ExistsForOtherProject`-then-discard path and the `Exists`-then-delete
/// path use identical race-handling arms after the re-create call. This helper
/// centralises that logic so the messages stay byte-identical across both paths.
///
/// Returns `Ok((id, created_at))` on `Created`, or a `Conflict` error for the
/// two race variants (`Exists` again, or `ExistsForOtherProject` with a new racer).
fn recreated_dump_ids(
    outcome: CreateDumpOutcome,
) -> Result<(String, Option<chrono::DateTime<chrono::Utc>>), Diagnostic> {
    match outcome {
        CreateDumpOutcome::Created(task2) => Ok((task2.id, task2.created_at)),
        CreateDumpOutcome::Exists { .. } => Err(Diagnostic::Conflict(
            "the dump was recreated before it could be replaced; try again".into(),
        )),
        CreateDumpOutcome::ExistsForOtherProject { project_iri: racer, .. } => Err(Diagnostic::Conflict(format!(
            "the dump slot was claimed by another project ({racer}) \
before this one could be created; try again"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Private mode helpers
// ---------------------------------------------------------------------------

/// Handle `create_project_dump` result in **Default** mode.
///
/// - `Created(task)` → poll → download → optional cleanup → `DumpOutcome{reused:false}`.
/// - `Exists{id}` → fetch status:
///   - `Completed` → Adopting → download → cleanup → `DumpOutcome{reused:true}`.
///   - `InProgress` → Adopting → poll → download → cleanup → `DumpOutcome{reused:true}`.
///   - `Failed` → `Conflict` with hint to use `--replace` or `--delete`.
#[allow(clippy::too_many_arguments)]
fn handle_default(
    create_outcome: CreateDumpOutcome,
    args: &ProjectDumpArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    reporter: &mut dyn ProgressReporter,
    token: &str,
    project_iri: &str,
    output_path: PathBuf,
    meta: &MetaContext,
    sleeper: &dyn Fn(Duration),
) -> Result<(), Diagnostic> {
    match create_outcome {
        CreateDumpOutcome::Created(task) => {
            reporter.report(&DumpEvent::Triggered { id: task.id.clone() })?;
            let created_at = task.created_at;
            let id = task.id;
            poll_until_done(client, cfg, reporter, token, project_iri, &id, args.timeout, sleeper)?;
            reporter.report(&DumpEvent::Downloading)?;
            let bytes = stream_dump_to_path(client, &cfg.server, project_iri, &id, token, &output_path)?;
            let cleaned_up = run_cleanup(args.cleanup, client, &cfg.server, project_iri, &id, token);
            reporter.report(&DumpEvent::Done { bytes })?;
            renderer.project_dump(
                &DumpOutcome {
                    path: output_path,
                    bytes,
                    cleaned_up,
                    reused: false,
                    created_at,
                },
                meta,
            )
        }
        CreateDumpOutcome::ExistsForOtherProject { project_iri: foreign_iri, .. } => {
            Err(Diagnostic::Conflict(format!(
                "no dump exists for the requested project; the server holds a single \
dump and it currently belongs to a different project ({foreign_iri}). Re-run \
with --replace --discard-other-project to discard that dump and create this \
project's, or wait for it to be removed."
            )))
        }
        CreateDumpOutcome::Exists { id } => {
            // Fetch current status to decide what to do.
            let status_task = client.get_project_dump_status(&cfg.server, project_iri, &id, token)?;
            match status_task.status {
                DumpStatus::Failed => Err(Diagnostic::Conflict(format!(
                    "the existing dump failed: {}; re-run with --replace to discard \
and create a fresh one, or --delete to remove it",
                    status_task.error_message.unwrap_or_default()
                ))),
                DumpStatus::Completed => {
                    reporter.report(&DumpEvent::Adopting { id: id.clone() })?;
                    reporter.report(&DumpEvent::Downloading)?;
                    let bytes = stream_dump_to_path(client, &cfg.server, project_iri, &id, token, &output_path)?;
                    let cleaned_up = run_cleanup(args.cleanup, client, &cfg.server, project_iri, &id, token);
                    reporter.report(&DumpEvent::Done { bytes })?;
                    renderer.project_dump(
                        &DumpOutcome {
                            path: output_path,
                            bytes,
                            cleaned_up,
                            reused: true,
                            created_at: status_task.created_at,
                        },
                        meta,
                    )
                }
                DumpStatus::InProgress => {
                    reporter.report(&DumpEvent::Adopting { id: id.clone() })?;
                    let created_at = status_task.created_at;
                    poll_until_done(client, cfg, reporter, token, project_iri, &id, args.timeout, sleeper)?;
                    reporter.report(&DumpEvent::Downloading)?;
                    let bytes = stream_dump_to_path(client, &cfg.server, project_iri, &id, token, &output_path)?;
                    let cleaned_up = run_cleanup(args.cleanup, client, &cfg.server, project_iri, &id, token);
                    reporter.report(&DumpEvent::Done { bytes })?;
                    renderer.project_dump(
                        &DumpOutcome {
                            path: output_path,
                            bytes,
                            cleaned_up,
                            reused: true,
                            created_at,
                        },
                        meta,
                    )
                }
            }
        }
    }
}

/// Handle `create_project_dump` result in **Replace** mode.
///
/// - `Created(task)` → poll → download → `DumpOutcome{reused:false}`.
/// - `Exists{id}`:
///   - `InProgress` → `Conflict` (can't replace in-progress).
///   - `Completed`/`Failed` → Deleting → delete → create again:
///     - `Created(task2)` → poll → download → `DumpOutcome{reused:false}`.
///     - `Exists{..}` again (race) → `Conflict`.
///     - `Err(_)` propagates.
#[allow(clippy::too_many_arguments)]
fn handle_replace(
    create_outcome: CreateDumpOutcome,
    args: &ProjectDumpArgs,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    reporter: &mut dyn ProgressReporter,
    token: &str,
    project_iri: &str,
    skip_assets: bool,
    output_path: PathBuf,
    meta: &MetaContext,
    sleeper: &dyn Fn(Duration),
) -> Result<(), Diagnostic> {
    let (id, created_at) = match create_outcome {
        CreateDumpOutcome::Created(task) => {
            reporter.report(&DumpEvent::Triggered { id: task.id.clone() })?;
            let created_at = task.created_at;
            let id = task.id;
            poll_until_done(client, cfg, reporter, token, project_iri, &id, args.timeout, sleeper)?;
            reporter.report(&DumpEvent::Downloading)?;
            let bytes = stream_dump_to_path(client, &cfg.server, project_iri, &id, token, &output_path)?;
            let cleaned_up = run_cleanup(args.cleanup, client, &cfg.server, project_iri, &id, token);
            reporter.report(&DumpEvent::Done { bytes })?;
            return renderer.project_dump(
                &DumpOutcome {
                    path: output_path,
                    bytes,
                    cleaned_up,
                    reused: false,
                    created_at,
                },
                meta,
            );
        }
        CreateDumpOutcome::ExistsForOtherProject { id: foreign_id, project_iri: foreign_iri } => {
            if !args.discard_other_project {
                return Err(Diagnostic::Conflict(format!(
                    "the server's single dump slot is held by a different project \
({foreign_iri}); re-run with --replace --discard-other-project to discard that \
project's dump and create this one's"
                )));
            }
            // --discard-other-project given: status-check the FOREIGN dump via its OWN iri
            // (intentional — never the requested project's iri).
            let foreign = client.get_project_dump_status(&cfg.server, &foreign_iri, &foreign_id, token)?;
            match foreign.status {
                DumpStatus::InProgress => {
                    return Err(Diagnostic::Conflict(format!(
                        "a dump for a different project ({foreign_iri}) is currently in \
progress; it cannot be discarded until it finishes — wait and retry"
                    )));
                }
                DumpStatus::Completed | DumpStatus::Failed => {
                    reporter.report(&DumpEvent::DiscardingOtherProjectDump {
                        id: foreign_id.clone(),
                        project_iri: foreign_iri.clone(),
                    })?;
                    client.delete_project_dump(&cfg.server, &foreign_iri, &foreign_id, token)?;
                    // Recreate for the REQUESTED project (outer project_iri).
                    let create2 = client.create_project_dump(&cfg.server, project_iri, skip_assets, token)?;
                    recreated_dump_ids(create2)?
                }
            }
        }
        CreateDumpOutcome::Exists { id } => {
            let status_task = client.get_project_dump_status(&cfg.server, project_iri, &id, token)?;
            match status_task.status {
                DumpStatus::InProgress => {
                    return Err(Diagnostic::Conflict(
                        "a dump is already in progress; it cannot be replaced until it finishes".into(),
                    ));
                }
                DumpStatus::Completed | DumpStatus::Failed => {
                    // Delete the existing dump and create a fresh one.
                    reporter.report(&DumpEvent::Deleting { id: id.clone() })?;
                    client.delete_project_dump(&cfg.server, project_iri, &id, token)?;
                    // Re-create — may race with another client.
                    let create2 = client.create_project_dump(&cfg.server, project_iri, skip_assets, token)?;
                    recreated_dump_ids(create2)?
                }
            }
        }
    };

    reporter.report(&DumpEvent::Triggered { id: id.clone() })?;
    poll_until_done(client, cfg, reporter, token, project_iri, &id, args.timeout, sleeper)?;
    reporter.report(&DumpEvent::Downloading)?;
    let bytes = stream_dump_to_path(client, &cfg.server, project_iri, &id, token, &output_path)?;
    let cleaned_up = run_cleanup(args.cleanup, client, &cfg.server, project_iri, &id, token);
    reporter.report(&DumpEvent::Done { bytes })?;
    renderer.project_dump(
        &DumpOutcome {
            path: output_path,
            bytes,
            cleaned_up,
            reused: false,
            created_at,
        },
        meta,
    )
}

/// Handle `create_project_dump` result in **Delete** mode.
///
/// - `Exists{id}`:
///   - `Completed`/`Failed` → Deleting → delete → `project_dump_deleted{deleted:true}`.
///   - `InProgress` → `Conflict`.
/// - `Created(task)` → nothing existed; a probe created a new in-progress dump. Discloses via
///   `ProbeCreated` event, emits `project_dump_deleted{deleted:false}`. Does NOT attempt to delete
///   the in-progress dump (would 409).
#[allow(clippy::too_many_arguments)]
fn handle_delete(
    create_outcome: CreateDumpOutcome,
    cfg: &Config,
    client: &dyn DspClient,
    renderer: &mut dyn Renderer,
    reporter: &mut dyn ProgressReporter,
    token: &str,
    project_iri: &str,
    meta: &MetaContext,
) -> Result<(), Diagnostic> {
    match create_outcome {
        CreateDumpOutcome::ExistsForOtherProject { project_iri: foreign_iri, .. } => renderer.project_dump_deleted(
            &DumpDeleteOutcome {
                deleted: false,
                note: Some(format!(
                    "no dump for the requested project to delete; the server's \
single dump slot is held by a different project ({foreign_iri})"
                )),
            },
            meta,
        ),
        CreateDumpOutcome::Exists { id } => {
            let status_task = client.get_project_dump_status(&cfg.server, project_iri, &id, token)?;
            match status_task.status {
                DumpStatus::InProgress => Err(Diagnostic::Conflict(
                    "the dump is in progress and cannot be deleted until it finishes".into(),
                )),
                DumpStatus::Completed | DumpStatus::Failed => {
                    reporter.report(&DumpEvent::Deleting { id: id.clone() })?;
                    client.delete_project_dump(&cfg.server, project_iri, &id, token)?;
                    renderer.project_dump_deleted(&DumpDeleteOutcome { deleted: true, note: None }, meta)
                }
            }
        }
        CreateDumpOutcome::Created(task) => {
            // Nothing existed; the POST probe created a new in-progress dump.
            // Disclose the side effect explicitly — do not attempt to delete it
            // (it is in_progress and a DELETE would 409).
            let note = format!(
                "no dump existed to delete; a probe created an in-progress dump {} \
that will complete server-side",
                task.id
            );
            reporter.report(&DumpEvent::ProbeCreated { id: task.id.clone() })?;
            tracing::warn!(
                id = %task.id,
                "delete mode: no existing dump found; probe created in-progress dump \
            that will complete server-side"
            );
            renderer.project_dump_deleted(&DumpDeleteOutcome { deleted: false, note: Some(note) }, meta)
        }
    }
}

// ---------------------------------------------------------------------------
// Shared poll loop
// ---------------------------------------------------------------------------

/// Poll `get_project_dump_status` until `Completed`, using a deterministic
/// logical clock and capped exponential backoff.
///
/// Returns `Ok(())` on completion. Returns `Err(ServerError)` on:
/// - `Failed` status from the server.
/// - Timeout (logical elapsed ≥ `timeout_secs`).
#[allow(clippy::too_many_arguments)]
fn poll_until_done(
    client: &dyn DspClient,
    cfg: &Config,
    reporter: &mut dyn ProgressReporter,
    token: &str,
    project_iri: &str,
    dump_id: &str,
    timeout_secs: u64,
    sleeper: &dyn Fn(Duration),
) -> Result<(), Diagnostic> {
    const BASE: Duration = Duration::from_secs(1);
    const CAP: Duration = Duration::from_secs(30);
    let timeout = Duration::from_secs(timeout_secs);
    let mut elapsed = Duration::ZERO;
    let mut delay = BASE;

    loop {
        let t = client.get_project_dump_status(&cfg.server, project_iri, dump_id, token)?;
        match t.status {
            DumpStatus::Completed => return Ok(()),
            DumpStatus::Failed => {
                return Err(Diagnostic::ServerError(format!(
                    "server-side dump failed: {}",
                    t.error_message.unwrap_or_default()
                )));
            }
            DumpStatus::InProgress => {
                // Report BEFORE incrementing elapsed so the first event reads 0s.
                reporter.report(&DumpEvent::Polling {
                    elapsed_secs: elapsed.as_secs(),
                    status: DumpStatus::InProgress,
                })?;
                if elapsed + delay >= timeout {
                    return Err(Diagnostic::ServerError(format!(
                        "dump did not complete within {timeout_secs}s; \
the server-side dump may still be running"
                    )));
                }
                sleeper(delay);
                elapsed += delay;
                delay = (delay * 2).min(CAP);
            }
        }
    }
}

/// Run optional cleanup after a successful download.
///
/// On success returns `true`; on any error logs a warning and returns `false`
/// (non-fatal, exit stays 0).
fn run_cleanup(
    cleanup: bool,
    client: &dyn DspClient,
    server: &str,
    project_iri: &str,
    dump_id: &str,
    token: &str,
) -> bool {
    if !cleanup {
        return false;
    }
    match client.delete_project_dump(server, project_iri, dump_id, token) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, "cleanup failed; dump not deleted from server");
            false
        }
    }
}

/// Stream a completed server-side dump to a local file atomically.
///
/// IMPORTANT: Every `std::io::Error` in this helper must be mapped to
/// `Diagnostic::Io(format!("…{path}…: {e}"))` — NEVER use bare `?` on an
/// `io::Error` here. Bare `?` would hit `From<io::Error> → Diagnostic::Internal`
/// and mis-classify the error as an internal bug instead of a user-visible
/// filesystem failure. See `Diagnostic::Io` for the design rationale.
///
/// Implementation:
/// 1. Create a sibling temp file `<final>.<pid>.partial` with `O_EXCL` so no concurrent writer is
///    clobbered.
/// 2. On Unix, restrict the file to owner-only (`0o600`) — dumps may hold sensitive research data.
/// 3. Stream the download into the temp file.
/// 4. `flush()` + `sync_all()` before rename to avoid truncated archives.
/// 5. `rename(temp, final)` — sibling temp guarantees same-filesystem, no EXDEV.
/// 6. On any error: best-effort `remove_file(temp)` and return the `Io` diagnostic.
///
/// Returns the number of bytes written.
fn stream_dump_to_path(
    client: &dyn DspClient,
    server: &str,
    project_iri: &str,
    dump_id: &str,
    token: &str,
    final_path: &Path,
) -> Result<u64, Diagnostic> {
    // Guard: a path like `/` or an empty path has no file_name component and
    // would silently produce a degenerate temp name. Fail fast instead. Bind
    // the file name here so the invariant is structural (no later unwrap).
    let file_name = final_path
        .file_name()
        .ok_or_else(|| Diagnostic::Usage(format!("invalid --output path: {}", final_path.display())))?;

    let temp_path = {
        let pid = std::process::id();
        let name = format!("{}.{pid}.partial", file_name.to_string_lossy());
        final_path
            .parent()
            .ok_or_else(|| {
                Diagnostic::Io(format!(
                    "cannot determine parent directory of output path {}",
                    final_path.display()
                ))
            })?
            .join(&name)
    };

    // Create the temp file with O_EXCL (create_new = true); on Unix also set
    // mode 0o600 so dumps (which may contain sensitive research data) are
    // owner-readable only — matching the auth.toml 0600 idiom.
    let mut open_opts = std::fs::OpenOptions::new();
    open_opts.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_opts.mode(0o600);
    }

    let mut file = open_opts
        .open(&temp_path)
        .map_err(|e| Diagnostic::Io(format!("failed to create temp file {}: {e}", temp_path.display())))?;

    // Stream download into the temp file.
    let result = client.download_project_dump(server, project_iri, dump_id, token, &mut file);

    let bytes = match result {
        Err(e) => {
            // Best-effort cleanup of the temp file before returning.
            let _ = std::fs::remove_file(&temp_path);
            return Err(e);
        }
        Ok(n) => n,
    };

    // flush() then sync_all() before rename — prevents truncated archives from
    // a buffered or interrupted write.
    file.flush().map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        Diagnostic::Io(format!("failed to flush temp file {}: {e}", temp_path.display()))
    })?;

    file.sync_all().map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        Diagnostic::Io(format!("failed to sync temp file {}: {e}", temp_path.display()))
    })?;

    // Rename temp → final (sibling temp ⇒ same filesystem ⇒ no EXDEV).
    std::fs::rename(&temp_path, final_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        Diagnostic::Io(format!(
            "failed to rename {} to {}: {e}",
            temp_path.display(),
            final_path.display()
        ))
    })?;

    Ok(bytes)
}

/// Compute the default output path for a dump.
///
/// Produces `<base>/<shortcode>-<timestamp>.zip` using the provided base
/// directory, the shortcode of the resolved project, and the injected `now`
/// value (for deterministic tests). When `base` is absolute (the normal case),
/// the result is absolute — no process CWD dependency.
fn default_output_path(base: &Path, shortcode: &str, now: DateTime<Utc>) -> PathBuf {
    base.join(format!("{shortcode}-{}.zip", now.format("%Y%m%dT%H%M%SZ")))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    use super::{default_output_path, run_describe_impl, run_impl, run_list_impl};
    use crate::cli::{FormatArgs, ProjectDescribeArgs, ProjectDumpArgs, ProjectListArgs};
    use crate::client::DspClient;
    use crate::config::auth_cache::ServerEntry;
    use crate::config::{AuthCache, Config};
    use crate::diagnostic::Diagnostic;
    use crate::model::{
        CreateDumpOutcome, DataModelSummary, DumpStatus, DumpTask, Project, ProjectDescription, ProjectDetail,
        ProjectRef, ProjectStatus,
    };
    use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
    use crate::render::progress::ProgressReporter;
    use crate::render::{DumpDeleteOutcome, DumpEvent, DumpOutcome, Format, MetaContext, ProjectListView, Renderer};

    // ── MockDspClient ─────────────────────────────────────────────────────────

    /// Records which client method was called in sequence, for asserting
    /// exact call order in mode-aware orchestration tests.
    ///
    /// `Status(iri)` and `Delete(iri)` carry the `project_iri` argument so tests
    /// can assert that the FOREIGN iri (not the requested project's) was used in
    /// cross-project scenarios.  `Create { project_iri }` similarly locks in the
    /// IRI used for dump creation, closing the mirror gap for recreate calls.
    #[derive(Debug, Clone, PartialEq)]
    enum CallRecord {
        Resolve,
        /// Carries the `project_iri` argument passed to `create_project_dump`,
        /// so tests can assert the recreate call used the REQUESTED project's IRI
        /// and not a foreign IRI.
        Create {
            project_iri: String,
        },
        /// Came from `poll_sequence` (not `status_sequence`).
        Poll,
        Download,
        /// Carries the `project_iri` passed to `delete_project_dump`.
        Delete(String),
        /// Came from `status_sequence`; carries the `project_iri` passed.
        Status(String),
        /// Came from `list_projects`; carries the token passed (for auth assertions).
        ListProjects {
            token: Option<String>,
        },
    }

    /// Local mock client for orchestration tests. Tracks call counts and
    /// call order so tests can assert "no server call was made" for the
    /// fail-fast guard paths, and assert the exact sequence per mode.
    ///
    /// Also records the `skip_assets` value passed to `create_project_dump`.
    /// For `--replace` mode, `create_result` may be used twice — the second
    /// call pops from `create_sequence` if set.
    struct MockDspClient {
        resolve_result: Option<Result<ProjectRef, Diagnostic>>,
        resolve_calls: RefCell<u32>,

        /// The primary create result (first call).
        create_result: Option<Result<CreateDumpOutcome, Diagnostic>>,
        /// If set, the second call to `create_project_dump` pops from this
        /// instead of using `create_result` again.
        create_sequence: RefCell<VecDeque<Result<CreateDumpOutcome, Diagnostic>>>,
        create_calls: RefCell<u32>,
        /// Records the `skip_assets` argument passed on the most recent call to
        /// `create_project_dump`. `None` means the method was never called.
        create_skip_assets: Cell<Option<bool>>,

        // Poll progression: pop-front per call; panics on exhaustion so an
        // over-eager loop fails loudly instead of silently looping forever.
        poll_sequence: RefCell<VecDeque<Result<DumpTask, Diagnostic>>>,
        poll_calls: RefCell<u32>,

        // Status progression: pop-front per call (used for get_project_dump_status
        // in the Exists path). If empty, falls back to poll_sequence.
        status_sequence: RefCell<VecDeque<Result<DumpTask, Diagnostic>>>,

        download_bytes: Option<Vec<u8>>,
        download_error: Option<Diagnostic>,
        download_calls: RefCell<u32>,

        delete_result: Option<Result<(), Diagnostic>>,
        delete_calls: RefCell<u32>,

        /// Canned result for `list_projects`. When `None`, returns `NotImplemented`.
        list_projects_result: Option<Result<Vec<Project>, Diagnostic>>,
        list_projects_calls: RefCell<u32>,
        /// Records the token argument passed to the most recent `list_projects` call.
        /// `None` before any call; `Some(None)` means called with token=None;
        /// `Some(Some(t))` means called with token=Some(t).
        list_projects_token: RefCell<Option<Option<String>>>,

        /// Canned result for `describe_project`. When `None`, returns `NotImplemented`.
        describe_project_result: Option<Result<ProjectDetail, Diagnostic>>,
        /// Records the (project, token) arguments passed to the most recent
        /// `describe_project` call. `None` before any call.
        describe_project_call: RefCell<Option<(String, Option<String>)>>,

        /// Records all client calls in order, for exact-sequence assertions.
        call_log: RefCell<Vec<CallRecord>>,
    }

    impl MockDspClient {
        fn new() -> Self {
            Self {
                resolve_result: None,
                resolve_calls: RefCell::new(0),
                create_result: None,
                create_sequence: RefCell::new(VecDeque::new()),
                create_calls: RefCell::new(0),
                create_skip_assets: Cell::new(None),
                poll_sequence: RefCell::new(VecDeque::new()),
                poll_calls: RefCell::new(0),
                status_sequence: RefCell::new(VecDeque::new()),
                download_bytes: None,
                download_error: None,
                download_calls: RefCell::new(0),
                delete_result: None,
                delete_calls: RefCell::new(0),
                list_projects_result: None,
                list_projects_calls: RefCell::new(0),
                list_projects_token: RefCell::new(None),
                describe_project_result: None,
                describe_project_call: RefCell::new(None),
                call_log: RefCell::new(Vec::new()),
            }
        }

        fn with_resolve_project(mut self, result: Result<ProjectRef, Diagnostic>) -> Self {
            self.resolve_result = Some(result);
            self
        }

        fn with_create_dump(mut self, result: Result<CreateDumpOutcome, Diagnostic>) -> Self {
            self.create_result = Some(result);
            self
        }

        fn with_create_exists(mut self, id: impl Into<String>) -> Self {
            self.create_result = Some(Ok(CreateDumpOutcome::Exists { id: id.into() }));
            self
        }

        fn with_create_exists_other_project(mut self, id: impl Into<String>, project_iri: impl Into<String>) -> Self {
            self.create_result = Some(Ok(CreateDumpOutcome::ExistsForOtherProject {
                id: id.into(),
                project_iri: project_iri.into(),
            }));
            self
        }

        /// Set a second-and-beyond sequence of results for `create_project_dump`
        /// (used in replace tests where create is called twice).
        fn with_create_sequence(
            mut self,
            seq: impl IntoIterator<Item = Result<CreateDumpOutcome, Diagnostic>>,
        ) -> Self {
            self.create_sequence = RefCell::new(seq.into_iter().collect());
            self
        }

        fn with_poll_sequence(mut self, seq: impl IntoIterator<Item = Result<DumpTask, Diagnostic>>) -> Self {
            self.poll_sequence = RefCell::new(seq.into_iter().collect());
            self
        }

        /// Set a status sequence used for `get_project_dump_status` in the
        /// Exists path (first pop from this, then falls back to poll_sequence).
        fn with_status_sequence(mut self, seq: impl IntoIterator<Item = Result<DumpTask, Diagnostic>>) -> Self {
            self.status_sequence = RefCell::new(seq.into_iter().collect());
            self
        }

        fn with_download_bytes(mut self, bytes: Vec<u8>) -> Self {
            self.download_bytes = Some(bytes);
            self
        }

        fn with_download_error(mut self, err: Diagnostic) -> Self {
            self.download_error = Some(err);
            self
        }

        fn with_delete_result(mut self, result: Result<(), Diagnostic>) -> Self {
            self.delete_result = Some(result);
            self
        }

        fn with_list_projects_result(mut self, result: Result<Vec<Project>, Diagnostic>) -> Self {
            self.list_projects_result = Some(result);
            self
        }

        fn with_describe_project_result(mut self, result: Result<ProjectDetail, Diagnostic>) -> Self {
            self.describe_project_result = Some(result);
            self
        }

        fn call_log(&self) -> Vec<CallRecord> {
            self.call_log.borrow().clone()
        }

        /// Return the token argument passed to the most recent `list_projects` call.
        /// Panics if `list_projects` was never called.
        fn list_projects_token(&self) -> Option<String> {
            self.list_projects_token
                .borrow()
                .as_ref()
                .expect("list_projects was not called")
                .clone()
        }

        /// Return the (project, token) arguments passed to the most recent
        /// `describe_project` call. Panics if `describe_project` was never called.
        fn describe_project_call(&self) -> (String, Option<String>) {
            self.describe_project_call
                .borrow()
                .clone()
                .expect("describe_project was not called")
        }

        /// Return `true` if `describe_project` was called at least once.
        fn describe_project_was_called(&self) -> bool {
            self.describe_project_call.borrow().is_some()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in dump tests")
        }

        fn resolve_project(&self, _server: &str, _project: &str) -> Result<ProjectRef, Diagnostic> {
            *self.resolve_calls.borrow_mut() += 1;
            self.call_log.borrow_mut().push(CallRecord::Resolve);
            self.resolve_result
                .clone()
                .expect("resolve_result must be set when resolve_project is called")
        }

        fn create_project_dump(
            &self,
            _server: &str,
            project_iri: &str,
            skip_assets: bool,
            _token: &str,
        ) -> Result<CreateDumpOutcome, Diagnostic> {
            *self.create_calls.borrow_mut() += 1;
            self.call_log
                .borrow_mut()
                .push(CallRecord::Create { project_iri: project_iri.to_string() });
            self.create_skip_assets.set(Some(skip_assets));
            // On the second+ call, pop from create_sequence if available.
            if *self.create_calls.borrow() > 1
                && let Some(result) = self.create_sequence.borrow_mut().pop_front()
            {
                return result;
            }
            self.create_result
                .clone()
                .expect("create_result must be set when create_project_dump is called")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<DumpTask, Diagnostic> {
            // First try the status_sequence (for Exists-path one-shot checks);
            // if empty, fall back to poll_sequence (for the poll loop).
            // The call_log distinguishes: Status(iri) = came from status_sequence,
            // Poll = came from poll_sequence.
            let from_status = self.status_sequence.borrow_mut().pop_front();
            if let Some(result) = from_status {
                self.call_log.borrow_mut().push(CallRecord::Status(project_iri.to_string()));
                return result;
            }
            *self.poll_calls.borrow_mut() += 1;
            self.call_log.borrow_mut().push(CallRecord::Poll);
            self.poll_sequence.borrow_mut().pop_front().expect(
                "poll_sequence exhausted — test bug: provide enough entries or let the logical clock fire first",
            )
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            *self.download_calls.borrow_mut() += 1;
            self.call_log.borrow_mut().push(CallRecord::Download);
            if let Some(ref e) = self.download_error {
                return Err(e.clone());
            }
            let bytes = self.download_bytes.as_deref().unwrap_or(&[]);
            dest.write_all(bytes)
                .map_err(|e| Diagnostic::Internal(format!("mock write error: {e}")))?;
            Ok(bytes.len() as u64)
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            *self.delete_calls.borrow_mut() += 1;
            self.call_log.borrow_mut().push(CallRecord::Delete(project_iri.to_string()));
            self.delete_result
                .clone()
                .expect("delete_result must be set when delete_project_dump is called")
        }

        fn list_projects(&self, _server: &str, token: Option<&str>) -> Result<Vec<crate::model::Project>, Diagnostic> {
            *self.list_projects_calls.borrow_mut() += 1;
            *self.list_projects_token.borrow_mut() = Some(token.map(str::to_owned));
            self.call_log
                .borrow_mut()
                .push(CallRecord::ListProjects { token: token.map(str::to_owned) });
            match &self.list_projects_result {
                Some(r) => r.clone(),
                None => Err(Diagnostic::NotImplemented(
                    "list_projects not configured in MockDspClient".into(),
                )),
            }
        }

        fn describe_project(
            &self,
            _server: &str,
            project: &str,
            token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            *self.describe_project_call.borrow_mut() = Some((project.to_owned(), token.map(str::to_owned)));
            match &self.describe_project_result {
                Some(r) => r.clone(),
                None => Err(Diagnostic::NotImplemented(
                    "describe_project not configured in MockDspClient".into(),
                )),
            }
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::DataModel>, Diagnostic> {
            unimplemented!("list_data_models not used by project tests")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            unimplemented!("describe_data_model not used in project tests")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in project tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in project tests")
        }

        fn list_resources(
            &self,
            _server: &str,
            _project_iri: &str,
            _resource_type_iri: &str,
            _order_by: Option<&str>,
            _page: u32,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourcePage, Diagnostic> {
            unimplemented!("list_resources not used in project tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in project tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used by dump tests")
        }

        fn resource_counts(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<std::collections::HashMap<String, u64>, Diagnostic> {
            Ok(std::collections::HashMap::new())
        }

        fn list_vocabularies(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Vocabulary>, Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn describe_vocabulary(
            &self,
            _server: &str,
            _iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::VocabularyTree, Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn sparql_query(
            &self,
            _server: &str,
            _token: &str,
            _query: &str,
            _accept: &str,
            _timeout_secs: u64,
        ) -> Result<crate::client::sparql::SparqlResponse, Diagnostic> {
            Err(Diagnostic::Internal("not used in this test".into()))
        }
    }

    // ── RecordingRenderer ─────────────────────────────────────────────────────

    struct RecordingRenderer {
        dump_outcome: Option<DumpOutcome>,
        dump_meta: Option<MetaContext>,
        dump_deleted_outcome: Option<DumpDeleteOutcome>,
        dump_deleted_meta: Option<MetaContext>,
        /// Recorded from `projects()` calls.
        projects_view: Option<(Vec<Project>, usize, Option<String>)>,
        projects_meta: Option<MetaContext>,
        /// Recorded from `project_describe()` calls.
        describe_detail: Option<ProjectDetail>,
        describe_meta: Option<MetaContext>,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                dump_outcome: None,
                dump_meta: None,
                dump_deleted_outcome: None,
                dump_deleted_meta: None,
                projects_view: None,
                projects_meta: None,
                describe_detail: None,
                describe_meta: None,
            }
        }
    }

    impl Renderer for RecordingRenderer {
        fn diagnostic(&mut self, _diag: &Diagnostic, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_login(&mut self, _outcome: &AuthLoginOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_status(&mut self, _outcome: &AuthStatusOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_logout(&mut self, _outcome: &AuthLogoutOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn auth_set_token(&mut self, _outcome: &AuthSetTokenOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn project_dump(&mut self, outcome: &DumpOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.dump_outcome = Some(DumpOutcome {
                path: outcome.path.clone(),
                bytes: outcome.bytes,
                cleaned_up: outcome.cleaned_up,
                reused: outcome.reused,
                created_at: outcome.created_at,
            });
            self.dump_meta = Some(meta.clone());
            Ok(())
        }

        fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.dump_deleted_outcome =
                Some(DumpDeleteOutcome { deleted: outcome.deleted, note: outcome.note.clone() });
            self.dump_deleted_meta = Some(meta.clone());
            Ok(())
        }

        fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.projects_view = Some((view.items.clone(), view.total, view.filter.clone()));
            self.projects_meta = Some(meta.clone());
            Ok(())
        }

        fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
            self.describe_detail = Some(project.clone());
            self.describe_meta = Some(meta.clone());
            Ok(())
        }

        fn data_models(
            &mut self,
            _view: &crate::render::DataModelListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn data_model_describe(
            &mut self,
            _detail: &crate::model::DataModelDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_types(
            &mut self,
            _view: &crate::render::ResourceTypeListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_type_describe(
            &mut self,
            _detail: &crate::model::ResourceTypeDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("resource_type_describe not used in project action tests")
        }

        fn data_model_structure(
            &mut self,
            _structure: &crate::model::DataModelStructure,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("data_model_structure not used in project action tests")
        }

        fn resources(
            &mut self,
            _view: &crate::render::ResourceListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn resource_describe(
            &mut self,
            _detail: &crate::model::ResourceDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            Ok(())
        }

        fn vocabularies(
            &mut self,
            _view: &crate::render::VocabularyListView,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }

        fn vocabulary_describe(
            &mut self,
            _detail: &crate::model::VocabularyDetail,
            _meta: &MetaContext,
        ) -> Result<(), Diagnostic> {
            unimplemented!("not exercised by this file's tests")
        }
    }

    // ── RecordingProgressReporter ─────────────────────────────────────────────

    struct RecordingProgressReporter {
        events: Vec<EventRecord>,
    }

    #[derive(Debug, PartialEq)]
    enum EventRecord {
        Triggered(String),
        Polling(u64),
        Downloading,
        Done(u64),
        Adopting(String),
        Deleting(String),
        ProbeCreated(String),
        DiscardingOtherProjectDump { id: String, project_iri: String },
    }

    impl RecordingProgressReporter {
        fn new() -> Self {
            Self { events: Vec::new() }
        }
    }

    impl ProgressReporter for RecordingProgressReporter {
        fn report(&mut self, event: &DumpEvent) -> Result<(), Diagnostic> {
            match event {
                DumpEvent::Triggered { id } => self.events.push(EventRecord::Triggered(id.clone())),
                DumpEvent::Polling { elapsed_secs, .. } => self.events.push(EventRecord::Polling(*elapsed_secs)),
                DumpEvent::Downloading => self.events.push(EventRecord::Downloading),
                DumpEvent::Done { bytes } => self.events.push(EventRecord::Done(*bytes)),
                DumpEvent::Adopting { id } => self.events.push(EventRecord::Adopting(id.clone())),
                DumpEvent::Deleting { id } => self.events.push(EventRecord::Deleting(id.clone())),
                DumpEvent::ProbeCreated { id } => self.events.push(EventRecord::ProbeCreated(id.clone())),
                DumpEvent::DiscardingOtherProjectDump { id, project_iri } => self
                    .events
                    .push(EventRecord::DiscardingOtherProjectDump { id: id.clone(), project_iri: project_iri.clone() }),
            }
            Ok(())
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn fixed_now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 29, 12, 0, 0).unwrap()
    }

    fn make_project_ref() -> ProjectRef {
        ProjectRef {
            iri: "http://rdfh.ch/projects/0001".to_string(),
            shortcode: "0001".to_string(),
            shortname: "anything".to_string(),
        }
    }

    fn make_dump_task(status: DumpStatus) -> DumpTask {
        DumpTask {
            id: "dump-id-42".to_string(),
            status,
            error_message: None,
            created_at: None,
        }
    }

    fn created_task(status: DumpStatus) -> CreateDumpOutcome {
        CreateDumpOutcome::Created(make_dump_task(status))
    }

    fn make_args(dir: &TempDir) -> (ProjectDumpArgs, Config) {
        let args = ProjectDumpArgs {
            server: Some("https://api.test.dasch.swiss".to_string()),
            project: Some("0001".to_string()),
            skip_assets: false,
            output: Some(dir.path().join("out.zip")),
            force: false,
            cleanup: false,
            timeout: 3600,
            replace: false,
            delete: false,
            discard_other_project: false,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        };
        let cfg = Config { server: "https://api.test.dasch.swiss".to_string() };
        (args, cfg)
    }

    fn no_op_sleeper() -> impl Fn(Duration) {
        |_| {}
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn happy_path_resolve_trigger_poll_download() {
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([
                Ok(make_dump_task(DumpStatus::InProgress)),
                Ok(make_dump_task(DumpStatus::Completed)),
            ])
            .with_download_bytes(b"PK fake zip content".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("env-token-abc".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert_eq!(outcome.bytes, 19); // b"PK fake zip content".len()
        assert!(!outcome.cleaned_up);

        // The output file must exist on disk and be non-empty after a happy-path run.
        assert!(outcome.path.exists(), "output file must exist after happy-path download");
        assert!(
            outcome.path.metadata().unwrap().len() > 0,
            "output file must be non-empty after happy-path download"
        );

        // Reporter saw: Triggered, Polling(0), Downloading, Done
        assert_eq!(reporter.events[0], EventRecord::Triggered("dump-id-42".to_string()));
        assert_eq!(reporter.events[1], EventRecord::Polling(0));
        assert_eq!(reporter.events[2], EventRecord::Downloading);
        assert_eq!(reporter.events[3], EventRecord::Done(19));
        assert_eq!(reporter.events.len(), 4);
    }

    #[test]
    fn default_filename_uses_shortcode_and_fixed_timestamp() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        // No explicit --output: default path is <cwd>/0001-20260529T120000Z.zip.
        // The injected cwd seam points at the tempdir so the file lands there
        // and no process-global CWD mutation is needed.
        let args = ProjectDumpArgs {
            server: Some("https://api.test.dasch.swiss".to_string()),
            project: Some("0001".to_string()),
            skip_assets: false,
            output: None,
            force: true, // skip the overwrite guard; we just want to verify the name
            cleanup: false,
            timeout: 3600,
            replace: false,
            delete: false,
            discard_other_project: false,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        };
        let cfg = Config { server: "https://api.test.dasch.swiss".to_string() };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            Some(&cache_path),
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        // The path should be <tempdir>/0001-20260529T120000Z.zip.
        let expected_path = dir.path().join("0001-20260529T120000Z.zip");
        assert_eq!(outcome.path, expected_path);
    }

    #[test]
    fn explicit_output_override_respected() {
        let dir = TempDir::new().unwrap();
        let out_path = dir.path().join("custom.zip");
        let (mut args, cfg) = make_args(&dir);
        args.output = Some(out_path.clone());

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"data".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert_eq!(outcome.path, out_path);
        assert!(out_path.exists());
    }

    #[test]
    fn explicit_output_exists_no_force_returns_usage_before_any_client_call() {
        let dir = TempDir::new().unwrap();
        let out_path = dir.path().join("existing.zip");
        std::fs::write(&out_path, b"existing").unwrap();

        let (mut args, cfg) = make_args(&dir);
        args.output = Some(out_path.clone());
        args.force = false;

        // Client should NOT be called at all — fail-fast before any server call.
        let client = MockDspClient::new();

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::Usage(_)), "expected Usage, got {err:?}");
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "message should mention overwrite refusal: {err}"
        );
        // Assert no client method was called.
        assert_eq!(*client.resolve_calls.borrow(), 0, "resolve_project must not be called");
        assert_eq!(*client.create_calls.borrow(), 0, "create must not be called");
    }

    #[test]
    fn default_output_exists_no_force_returns_usage_after_resolve_before_trigger() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");

        let args = ProjectDumpArgs {
            server: Some("https://api.test.dasch.swiss".to_string()),
            project: Some("0001".to_string()),
            skip_assets: false,
            output: None,
            force: false,
            cleanup: false,
            timeout: 3600,
            replace: false,
            delete: false,
            discard_other_project: false,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        };
        let cfg = Config { server: "https://api.test.dasch.swiss".to_string() };

        let client = MockDspClient::new().with_resolve_project(Ok(make_project_ref()));
        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        // Pre-create the default file inside the tempdir so the overwrite-guard fires.
        // No CWD mutation needed: the injected `cwd` seam points at `dir.path()`.
        let default_path_in_tempdir = dir.path().join("0001-20260529T120000Z.zip");
        std::fs::write(&default_path_in_tempdir, b"existing")
            .expect("must be able to write conflict file into tempdir");

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            Some(&cache_path),
            dir.path(),
        )
        .unwrap_err();

        // Guard fires after resolve but before trigger: unconditional assertions.
        assert!(matches!(err, Diagnostic::Usage(_)), "expected Usage, got {err:?}");
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "message should mention overwrite refusal: {err}"
        );
        assert_eq!(
            *client.resolve_calls.borrow(),
            1,
            "resolve must have been called (default-path guard runs after resolve)"
        );
        assert_eq!(
            *client.create_calls.borrow(),
            0,
            "trigger must NOT have been called (guard fires before trigger)"
        );
    }

    #[test]
    fn missing_token_returns_auth_required_with_no_client_calls() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let (args, cfg) = make_args(&dir);

        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        // No env token, no cache entry → AuthRequired.
        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            None, // no env token
            &no_op_sleeper(),
            fixed_now(),
            Some(&cache_path), // empty cache
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::AuthRequired(_)), "expected AuthRequired, got {err:?}");
        assert_eq!(*client.resolve_calls.borrow(), 0, "resolve must not be called");
        assert_eq!(*client.create_calls.borrow(), 0, "trigger must not be called");
    }

    #[test]
    fn trigger_conflict_propagates() {
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Err(Diagnostic::Conflict(
                "a dump for this project is already in progress or present".to_string(),
            )));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::Conflict(_)), "expected Conflict, got {err:?}");
    }

    #[test]
    fn poll_failed_returns_server_error() {
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let failed_task = DumpTask {
            id: "dump-id-42".to_string(),
            status: DumpStatus::Failed,
            error_message: Some("out of disk space".to_string()),
            created_at: None,
        };
        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(failed_task)]);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::ServerError(_)), "expected ServerError, got {err:?}");
        let msg = err.to_string();
        assert!(
            msg.contains("server-side dump failed"),
            "message should mention dump failure: {msg}"
        );
        assert!(msg.contains("out of disk space"), "message should include error_message: {msg}");
    }

    #[test]
    fn timeout_via_logical_clock_returns_server_error() {
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        // A tiny timeout of 1s. BASE=1s, so after the first in_progress response
        // elapsed=0, delay=1s, elapsed+delay=1s >= timeout=1s → terminate.
        args.timeout = 1;

        // Provide a long enough poll sequence that exhaustion won't fire first.
        let in_progress: Vec<Result<DumpTask, Diagnostic>> =
            (0..100).map(|_| Ok(make_dump_task(DumpStatus::InProgress))).collect();

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence(in_progress);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::ServerError(_)),
            "expected ServerError timeout, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("did not complete"), "message should mention timeout: {msg}");
        assert!(
            msg.contains("may still be running"),
            "message should contain user-visible hint 'may still be running': {msg}"
        );
    }

    #[test]
    fn cleanup_success_sets_cleaned_up_true() {
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.cleanup = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec())
            .with_delete_result(Ok(()));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(outcome.cleaned_up, "cleanup success should set cleaned_up=true");
    }

    #[test]
    fn cleanup_error_keeps_exit_ok_and_cleaned_up_false() {
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.cleanup = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec())
            .with_delete_result(Err(Diagnostic::ServerError("delete failed".to_string())));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        // Should return Ok even when cleanup fails.
        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(!outcome.cleaned_up, "cleanup error should set cleaned_up=false");
    }

    #[test]
    fn download_error_leaves_no_file_at_target_path() {
        let dir = TempDir::new().unwrap();
        let out_path = dir.path().join("should-not-exist.zip");
        let (mut args, cfg) = make_args(&dir);
        args.output = Some(out_path.clone());

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_error(Diagnostic::Network("connection reset".to_string()));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::Network(_)), "expected Network error, got {err:?}");
        assert!(!out_path.exists(), "target file must not exist after download error");
    }

    #[test]
    fn default_output_path_pure_fn() {
        let now = Utc.with_ymd_and_hms(2026, 5, 29, 12, 0, 0).unwrap();
        let base = std::path::Path::new("/tmp/test-base");
        let path = default_output_path(base, "0001", now);
        assert_eq!(path, PathBuf::from("/tmp/test-base/0001-20260529T120000Z.zip"));
    }

    #[test]
    fn auth_state_env_token_reports_authenticated_via_dsp_token() {
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("env-token".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let meta = renderer.dump_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");
    }

    #[test]
    fn auth_state_cache_token_reports_authenticated() {
        use crate::config::AuthCache;
        use crate::config::auth_cache::ServerEntry;

        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let out_path = dir.path().join("out.zip");

        // Put a token in the cache.
        let mut cache = AuthCache::default();
        cache.set_entry(
            "https://api.test.dasch.swiss",
            ServerEntry {
                token: "cache-tok".to_string(),
                user: None,
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let args = ProjectDumpArgs {
            server: Some("https://api.test.dasch.swiss".to_string()),
            project: Some("0001".to_string()),
            skip_assets: false,
            output: Some(out_path),
            force: false,
            cleanup: false,
            timeout: 3600,
            replace: false,
            delete: false,
            discard_other_project: false,
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        };
        let cfg = Config { server: "https://api.test.dasch.swiss".to_string() };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            None, // no env token → falls through to cache
            &no_op_sleeper(),
            fixed_now(),
            Some(&cache_path),
            dir.path(),
        )
        .unwrap();

        // cache entry has no user → "authenticated" (not "authenticated as {user}")
        let meta = renderer.dump_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated");
    }

    #[test]
    fn rename_failure_returns_io_and_temp_cleaned_up() {
        // Point --output at a path whose parent is an existing *directory* so
        // rename succeeds the temp create but fails the rename step (can't
        // rename a file to a path that is an existing directory on most OSes).
        let dir = TempDir::new().unwrap();
        // Create a sub-directory at the target path so rename fails.
        let final_path = dir.path().join("dump_dir");
        std::fs::create_dir(&final_path).unwrap();

        let (mut args, cfg) = make_args(&dir);
        args.output = Some(final_path.clone());
        args.force = true; // skip the exists guard (it's a dir, exists())

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        // Should be Io (not Internal).
        assert!(
            matches!(err, Diagnostic::Io(_)),
            "expected Io error for rename failure, got {err:?}"
        );

        // The .partial temp should have been cleaned up.
        let pid = std::process::id();
        let temp = dir.path().join(format!("dump_dir.{pid}.partial"));
        assert!(!temp.exists(), "temp file should be cleaned up after rename failure");
    }

    // ── Fix 4: download_error Io variant ─────────────────────────────────────

    /// Parameterised helper that verifies a download error propagates and
    /// leaves no file at the target path. Covers both `Network` and `Io` error
    /// variants so both code paths through `stream_dump_to_path` are exercised.
    fn assert_download_error_leaves_no_file(download_error: Diagnostic) {
        let dir = TempDir::new().unwrap();
        let out_path = dir.path().join("should-not-exist.zip");
        let (mut args, cfg) = make_args(&dir);
        args.output = Some(out_path.clone());

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_error(download_error);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        // The error must propagate unchanged and no output file must exist.
        assert!(!err.to_string().is_empty(), "error message must be non-empty");
        assert!(!out_path.exists(), "target file must not exist after download error ({err:?})");
    }

    #[test]
    fn download_io_error_leaves_no_file_at_target_path() {
        assert_download_error_leaves_no_file(Diagnostic::Io(
            "failed to write /tmp/test.zip: no space left on device".to_string(),
        ));
    }

    // ── Fix 6: skip_assets pass-through ──────────────────────────────────────

    #[test]
    fn skip_assets_true_is_passed_through_to_create_project_dump() {
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.skip_assets = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        assert_eq!(
            client.create_skip_assets.get(),
            Some(true),
            "skip_assets=true must be forwarded to create_project_dump"
        );
    }

    #[test]
    fn skip_assets_false_is_passed_through_to_create_project_dump() {
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.skip_assets = false;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zip".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        assert_eq!(
            client.create_skip_assets.get(),
            Some(false),
            "skip_assets=false must be forwarded to create_project_dump"
        );
    }

    // ── Amendment 1: mode-aware orchestration matrix ──────────────────────────

    // --- Default mode ---

    #[test]
    fn default_fresh_created_no_existing_dump() {
        // Default mode + Created → poll → download → reused:false, created_at populated.
        use chrono::TimeZone;
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);
        let ts = Utc.with_ymd_and_hms(2026, 5, 20, 14, 3, 0).unwrap();

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(CreateDumpOutcome::Created(DumpTask {
                id: "dump-id-42".into(),
                status: DumpStatus::InProgress,
                error_message: None,
                created_at: Some(ts),
            })))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"zipdata".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(!outcome.reused, "fresh dump must have reused:false");
        assert_eq!(outcome.created_at, Some(ts), "created_at must be populated from task");
        assert_eq!(outcome.bytes, 7); // b"zipdata".len()
        // Call sequence: Resolve, Create, Status/Poll, Download
        let log = client.call_log();
        assert!(log.contains(&CallRecord::Resolve));
        assert!(log.contains(&CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }));
        assert!(log.contains(&CallRecord::Poll));
        assert!(log.contains(&CallRecord::Download));
    }

    #[test]
    fn default_adopt_completed_existing_dump() {
        // Default mode + Exists{id} → status=Completed → download → reused:true.
        use chrono::TimeZone;
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);
        let ts = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();

        let existing_task = DumpTask {
            id: "existing-id".into(),
            status: DumpStatus::Completed,
            error_message: None,
            created_at: Some(ts),
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("existing-id")
            .with_status_sequence([Ok(existing_task)])
            .with_download_bytes(b"existing".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(outcome.reused, "adopted dump must have reused:true");
        assert_eq!(outcome.created_at, Some(ts), "created_at must come from status");
        // Reporter: Adopting, Downloading, Done (no Triggered, no Polling)
        assert!(
            reporter.events.contains(&EventRecord::Adopting("existing-id".into())),
            "must report Adopting"
        );
        assert!(
            !reporter.events.iter().any(|e| matches!(e, EventRecord::Triggered(_))),
            "must NOT report Triggered when adopting"
        );
        // Call sequence: Resolve, Create, Status, Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log[2], CallRecord::Status("http://rdfh.ch/projects/0001".to_string()));
        assert_eq!(log[3], CallRecord::Download);
    }

    #[test]
    fn default_adopt_in_progress_polls_then_downloads() {
        // Default mode + Exists{id} → status=InProgress → poll → download → reused:true.
        use chrono::TimeZone;
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);
        let ts = Utc.with_ymd_and_hms(2026, 5, 10, 8, 0, 0).unwrap();

        let in_progress_task = DumpTask {
            id: "adopt-ip-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: Some(ts),
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("adopt-ip-id")
            // status call returns in_progress; then poll_sequence has one in_progress
            // tick followed by completed so a Polling event is emitted.
            .with_status_sequence([Ok(in_progress_task)])
            .with_poll_sequence([
                Ok(make_dump_task(DumpStatus::InProgress)),
                Ok(make_dump_task(DumpStatus::Completed)),
            ])
            .with_download_bytes(b"data".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(outcome.reused);
        assert_eq!(outcome.created_at, Some(ts));
        // Full call sequence: Resolve, Create, Status, Poll (in_progress), Poll (completed),
        // Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve, "first call must be Resolve");
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() },
            "second call must be Create"
        );
        assert_eq!(
            log[2],
            CallRecord::Status("http://rdfh.ch/projects/0001".to_string()),
            "third call must be Status"
        );
        assert_eq!(log[3], CallRecord::Poll, "fourth call must be Poll (in_progress)");
        assert_eq!(log[4], CallRecord::Poll, "fifth call must be Poll (completed)");
        assert_eq!(log[5], CallRecord::Download, "sixth call must be Download");
        assert_eq!(log.len(), 6, "must be exactly 6 calls");
        // Adopting must be reported before any Polling event.
        let adopting_idx = reporter
            .events
            .iter()
            .position(|e| matches!(e, EventRecord::Adopting(_)))
            .expect("Adopting event must be present");
        let first_polling_idx = reporter
            .events
            .iter()
            .position(|e| matches!(e, EventRecord::Polling(_)))
            .expect("Polling event must be present");
        assert!(
            adopting_idx < first_polling_idx,
            "Adopting must be reported before the first Polling event"
        );
    }

    #[test]
    fn default_existing_failed_returns_conflict_with_hint() {
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let failed_task = DumpTask {
            id: "fail-id".into(),
            status: DumpStatus::Failed,
            error_message: Some("disk full".into()),
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("fail-id")
            .with_status_sequence([Ok(failed_task)]);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::Conflict(_)),
            "failed existing dump must yield Conflict"
        );
        let msg = err.to_string();
        assert!(msg.contains("existing dump failed"), "message must mention failure: {msg}");
        assert!(msg.contains("disk full"), "message must include server error: {msg}");
        assert!(msg.contains("--replace"), "message must hint at --replace: {msg}");
        assert!(msg.contains("--delete"), "message must hint at --delete: {msg}");
    }

    // --- Replace mode ---

    #[test]
    fn replace_none_existing_creates_fresh() {
        // Replace + Created (no existing) → same as fresh.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"fresh".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(!outcome.reused, "replace with no existing → reused:false");
        // Exactly one create call
        assert_eq!(*client.create_calls.borrow(), 1);
    }

    #[test]
    fn replace_completed_deletes_then_recreates() {
        // Replace + Exists (completed) → status→delete→create2→poll→download.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;

        let existing_task = DumpTask {
            id: "old-id".into(),
            status: DumpStatus::Completed,
            error_message: None,
            created_at: None,
        };
        let task2 = DumpTask {
            id: "new-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("old-id")
            .with_create_sequence([Ok(CreateDumpOutcome::Created(task2))])
            .with_status_sequence([Ok(existing_task)])
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_delete_result(Ok(()))
            .with_download_bytes(b"new".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(!outcome.reused, "replace always produces reused:false");
        // Assert full call order: Resolve, Create(1st), Status, Delete, Create(2nd), Poll, Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve, "first call must be Resolve");
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() },
            "second call must be Create"
        );
        assert_eq!(
            log[2],
            CallRecord::Status("http://rdfh.ch/projects/0001".to_string()),
            "third call must be Status"
        );
        assert_eq!(
            log[3],
            CallRecord::Delete("http://rdfh.ch/projects/0001".to_string()),
            "fourth call must be Delete"
        );
        assert_eq!(
            log[4],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() },
            "fifth call must be Create (2nd)"
        );
        assert_eq!(log[5], CallRecord::Poll, "sixth call must be Poll");
        assert_eq!(log[6], CallRecord::Download, "seventh call must be Download");
        assert_eq!(log.len(), 7, "must be exactly 7 calls");
        // Deleting event must have been reported
        assert!(
            reporter.events.contains(&EventRecord::Deleting("old-id".into())),
            "must report Deleting for the old dump"
        );
        // Triggered must have been reported for the second (new) dump
        assert!(
            reporter.events.contains(&EventRecord::Triggered("new-id".into())),
            "must report Triggered for the new dump; events: {:?}",
            reporter.events
        );
    }

    #[test]
    fn replace_failed_existing_deletes_then_recreates() {
        // Replace + Exists (failed) → same path as completed.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;

        let failed_task = DumpTask {
            id: "failed-old-id".into(),
            status: DumpStatus::Failed,
            error_message: Some("ran out of space".into()),
            created_at: None,
        };
        let task2 = DumpTask {
            id: "new-id-2".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("failed-old-id")
            .with_create_sequence([Ok(CreateDumpOutcome::Created(task2))])
            .with_status_sequence([Ok(failed_task)])
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_delete_result(Ok(()))
            .with_download_bytes(b"new".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        let outcome = renderer.dump_outcome.unwrap();
        assert!(!outcome.reused);
    }

    #[test]
    fn replace_recreate_race_returns_conflict() {
        // Replace + Exists → status→delete→create2 returns Exists again (race).
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;

        let existing_task = DumpTask {
            id: "race-id".into(),
            status: DumpStatus::Completed,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("race-id")
            .with_create_sequence([Ok(CreateDumpOutcome::Exists { id: "race-id-2".into() })])
            .with_status_sequence([Ok(existing_task)])
            .with_delete_result(Ok(()));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::Conflict(_)), "recreate race must yield Conflict");
        let msg = err.to_string();
        assert!(msg.contains("recreated"), "message must mention recreation: {msg}");
    }

    #[test]
    fn replace_in_progress_returns_conflict() {
        // Replace + Exists (in_progress) → cannot replace.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;

        let in_progress_task = DumpTask {
            id: "ip-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("ip-id")
            .with_status_sequence([Ok(in_progress_task)]);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::Conflict(_)),
            "in-progress existing dump must block replace"
        );
        let msg = err.to_string();
        assert!(msg.contains("in progress"), "message must mention in-progress state: {msg}");
        // Delete must NOT have been called.
        assert_eq!(*client.delete_calls.borrow(), 0, "delete must not be called when in-progress");
    }

    // --- Delete mode ---

    #[test]
    fn delete_completed_deletes_without_downloading() {
        // Delete + Exists (completed) → status→delete → project_dump_deleted{deleted:true}.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.delete = true;

        let completed_task = DumpTask {
            id: "del-id".into(),
            status: DumpStatus::Completed,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("del-id")
            .with_status_sequence([Ok(completed_task)])
            .with_delete_result(Ok(()));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        // Must NOT have downloaded.
        assert_eq!(*client.download_calls.borrow(), 0, "delete must not download");
        // Must have called delete.
        assert_eq!(*client.delete_calls.borrow(), 1);
        // project_dump_deleted must have been called with deleted:true.
        let del_outcome = renderer
            .dump_deleted_outcome
            .expect("project_dump_deleted must have been called");
        assert!(del_outcome.deleted);
        assert!(del_outcome.note.is_none());
        // project_dump must NOT have been called.
        assert!(
            renderer.dump_outcome.is_none(),
            "project_dump must not be called in delete mode"
        );
        // Reporter: Deleting{id}
        assert!(reporter.events.contains(&EventRecord::Deleting("del-id".into())));
        // Call sequence: Resolve, Create, Status, Delete — no Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log[2], CallRecord::Status("http://rdfh.ch/projects/0001".to_string()));
        assert_eq!(log[3], CallRecord::Delete("http://rdfh.ch/projects/0001".to_string()));
        assert_eq!(log.len(), 4, "must be exactly 4 calls");
    }

    #[test]
    fn delete_failed_deletes_without_downloading() {
        // Delete + Exists (failed) → same path as completed: status→delete →
        // project_dump_deleted{deleted:true}. Verifies the `Completed | Failed` arm handles Failed
        // identically to Completed.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.delete = true;

        let failed_task = DumpTask {
            id: "del-failed-id".into(),
            status: DumpStatus::Failed,
            error_message: Some("disk full".into()),
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("del-failed-id")
            .with_status_sequence([Ok(failed_task)])
            .with_delete_result(Ok(()));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        // Must NOT have downloaded.
        assert_eq!(
            *client.download_calls.borrow(),
            0,
            "delete must not download even for a failed dump"
        );
        // Must have called delete.
        assert_eq!(*client.delete_calls.borrow(), 1, "delete must be called for a failed dump");
        // project_dump_deleted must have been called with deleted:true.
        let del_outcome = renderer
            .dump_deleted_outcome
            .expect("project_dump_deleted must have been called");
        assert!(del_outcome.deleted, "deleted must be true for failed dump");
        assert!(del_outcome.note.is_none());
        // project_dump must NOT have been called.
        assert!(
            renderer.dump_outcome.is_none(),
            "project_dump must not be called in delete mode"
        );
        // Call sequence: Resolve, Create, Status, Delete — no Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log[2], CallRecord::Status("http://rdfh.ch/projects/0001".to_string()));
        assert_eq!(log[3], CallRecord::Delete("http://rdfh.ch/projects/0001".to_string()));
        assert_eq!(log.len(), 4, "must be exactly 4 calls");
    }

    #[test]
    fn delete_in_progress_returns_conflict() {
        // Delete + Exists (in_progress) → cannot delete.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.delete = true;

        let in_progress_task = DumpTask {
            id: "del-ip-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists("del-ip-id")
            .with_status_sequence([Ok(in_progress_task)]);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, Diagnostic::Conflict(_)), "in-progress dump must block delete");
        let msg = err.to_string();
        assert!(msg.contains("in progress"), "message must mention in-progress: {msg}");
        assert_eq!(*client.delete_calls.borrow(), 0, "delete must not be called");
    }

    #[test]
    fn delete_none_probe_created_reports_probe_and_exits_ok() {
        // Delete + Created (nothing existed; probe created in-progress dump).
        // Must: report ProbeCreated, render project_dump_deleted{deleted:false}, exit Ok.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.delete = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_dump(Ok(created_task(DumpStatus::InProgress)));

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap(); // must succeed (exit 0)

        // project_dump must NOT be called.
        assert!(renderer.dump_outcome.is_none());
        // project_dump_deleted must be called with deleted:false and a note.
        let del_outcome = renderer.dump_deleted_outcome.expect("project_dump_deleted must be called");
        assert!(!del_outcome.deleted, "deleted must be false (probe, not real delete)");
        let note = del_outcome.note.expect("note must be set for probe case");
        assert!(note.contains("dump-id-42"), "note must mention the probe id: {note}");
        // Reporter must have received ProbeCreated.
        assert!(
            reporter.events.contains(&EventRecord::ProbeCreated("dump-id-42".into())),
            "must report ProbeCreated; events: {:?}",
            reporter.events
        );
        // Must NOT have called download or delete.
        assert_eq!(*client.download_calls.borrow(), 0);
        assert_eq!(*client.delete_calls.borrow(), 0);
    }

    // ── ExistsForOtherProject mode arms ──────────────────────────────────────

    /// The IRI of the foreign (other) project used in cross-project guard tests.
    fn foreign_iri() -> &'static str {
        "http://rdfh.ch/projects/0002"
    }

    #[test]
    fn default_exists_for_other_project_returns_conflict_no_server_calls() {
        // Default + ExistsForOtherProject → Conflict immediately; no status/download call.
        let dir = TempDir::new().unwrap();
        let (args, cfg) = make_args(&dir);

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::Conflict(_)),
            "Default + ExistsForOtherProject must yield Conflict, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains(foreign_iri()), "Conflict message must name the foreign IRI: {msg}");
        assert!(
            msg.contains("--replace --discard-other-project"),
            "Conflict message must hint at --replace --discard-other-project: {msg}"
        );
        // No status/download/delete calls (only Resolve + Create).
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log.len(), 2, "must be exactly 2 calls (no status/delete/download)");
    }

    #[test]
    fn replace_exists_for_other_project_without_flag_returns_conflict_no_status_delete() {
        // Replace + ExistsForOtherProject + no --discard-other-project → Conflict, no
        // status/delete.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;
        // discard_other_project remains false (default from make_args).

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::Conflict(_)),
            "Replace + ExistsForOtherProject without flag must yield Conflict, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains(foreign_iri()), "Conflict message must name the foreign IRI: {msg}");
        assert!(
            msg.contains("--replace --discard-other-project"),
            "Conflict message must hint at the flag: {msg}"
        );
        // Only Resolve + Create — no status/delete.
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log.len(), 2, "must be exactly 2 calls (no status/delete)");
    }

    #[test]
    fn replace_exists_for_other_project_with_flag_foreign_completed_discards_and_recreates() {
        // Replace + ExistsForOtherProject + --discard-other-project, foreign Completed →
        // DiscardingOtherProjectDump event, delete(foreign_iri), create(requested_iri), download.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;
        args.discard_other_project = true;

        let foreign_task = DumpTask {
            id: "foreign-dump-id".into(),
            status: DumpStatus::Completed,
            error_message: None,
            created_at: None,
        };
        let new_task = DumpTask {
            id: "new-dump-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri())
            // The status check must use the FOREIGN iri (foreign_task comes from status_sequence).
            .with_status_sequence([Ok(foreign_task)])
            // The create after delete must return Created for the REQUESTED project.
            .with_create_sequence([Ok(CreateDumpOutcome::Created(new_task))])
            .with_delete_result(Ok(()))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"dump-data".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        // DiscardingOtherProjectDump must have been reported.
        assert!(
            reporter.events.contains(&EventRecord::DiscardingOtherProjectDump {
                id: "foreign-dump-id".into(),
                project_iri: foreign_iri().to_string(),
            }),
            "must report DiscardingOtherProjectDump; events: {:?}",
            reporter.events
        );

        // Call log: Resolve, Create(1st ExistsForOtherProject), Status(foreign), Delete(foreign),
        //           Create(2nd for requested), Poll, Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve, "first must be Resolve");
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: make_project_ref().iri },
            "second must be Create with REQUESTED project IRI (initial probe)"
        );
        assert_eq!(
            log[2],
            CallRecord::Status(foreign_iri().to_string()),
            "third must be Status with FOREIGN iri"
        );
        assert_eq!(
            log[3],
            CallRecord::Delete(foreign_iri().to_string()),
            "fourth must be Delete with FOREIGN iri"
        );
        assert_eq!(
            log[4],
            CallRecord::Create { project_iri: make_project_ref().iri },
            "fifth must be Create with REQUESTED project IRI (recreate after discard)"
        );
        assert_eq!(log[5], CallRecord::Poll, "sixth must be Poll");
        assert_eq!(log[6], CallRecord::Download, "seventh must be Download");
        assert_eq!(log.len(), 7, "must be exactly 7 calls");

        // The final dump outcome must exist (we downloaded).
        assert!(
            renderer.dump_outcome.is_some(),
            "project_dump must be called after successful discard+recreate"
        );
    }

    #[test]
    fn replace_exists_for_other_project_with_flag_foreign_failed_discards_and_recreates() {
        // Replace + ExistsForOtherProject + --discard-other-project, foreign Failed →
        // same outcome as Completed: DiscardingOtherProjectDump event, delete(foreign_iri),
        // create(requested_iri), download.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;
        args.discard_other_project = true;

        let foreign_task = DumpTask {
            id: "foreign-dump-id".into(),
            status: DumpStatus::Failed,
            error_message: Some("out of disk space".into()),
            created_at: None,
        };
        let new_task = DumpTask {
            id: "new-dump-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri())
            // The status check must use the FOREIGN iri (foreign_task from status_sequence).
            .with_status_sequence([Ok(foreign_task)])
            // The create after delete must return Created for the REQUESTED project.
            .with_create_sequence([Ok(CreateDumpOutcome::Created(new_task))])
            .with_delete_result(Ok(()))
            .with_poll_sequence([Ok(make_dump_task(DumpStatus::Completed))])
            .with_download_bytes(b"dump-data".to_vec());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap();

        // DiscardingOtherProjectDump must have been reported.
        assert!(
            reporter.events.contains(&EventRecord::DiscardingOtherProjectDump {
                id: "foreign-dump-id".into(),
                project_iri: foreign_iri().to_string(),
            }),
            "must report DiscardingOtherProjectDump; events: {:?}",
            reporter.events
        );

        // Call log: Resolve, Create(1st ExistsForOtherProject), Status(foreign), Delete(foreign),
        //           Create(2nd for requested), Poll, Download
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve, "first must be Resolve");
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: make_project_ref().iri },
            "second must be Create with REQUESTED project IRI (initial probe)"
        );
        assert_eq!(
            log[2],
            CallRecord::Status(foreign_iri().to_string()),
            "third must be Status with FOREIGN iri"
        );
        assert_eq!(
            log[3],
            CallRecord::Delete(foreign_iri().to_string()),
            "fourth must be Delete with FOREIGN iri"
        );
        assert_eq!(
            log[4],
            CallRecord::Create { project_iri: make_project_ref().iri },
            "fifth must be Create with REQUESTED project IRI (recreate after discard)"
        );
        assert_eq!(log[5], CallRecord::Poll, "sixth must be Poll");
        assert_eq!(log[6], CallRecord::Download, "seventh must be Download");
        assert_eq!(log.len(), 7, "must be exactly 7 calls");

        // The final dump outcome must exist (we downloaded).
        assert!(
            renderer.dump_outcome.is_some(),
            "project_dump must be called after successful discard+recreate"
        );
    }

    #[test]
    fn replace_exists_for_other_project_with_flag_foreign_in_progress_returns_conflict() {
        // Replace + ExistsForOtherProject + --discard-other-project, foreign InProgress →
        // Conflict "in progress"; no delete call.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.replace = true;
        args.discard_other_project = true;

        let foreign_task = DumpTask {
            id: "foreign-dump-id".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri())
            .with_status_sequence([Ok(foreign_task)]);

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        let err = run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap_err();

        assert!(
            matches!(err, Diagnostic::Conflict(_)),
            "foreign InProgress must yield Conflict, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("in progress"), "message must mention in progress: {msg}");
        assert!(msg.contains(foreign_iri()), "message must name the foreign IRI: {msg}");
        // Status check was done with FOREIGN iri; no delete.
        let log = client.call_log();
        assert_eq!(
            log[2],
            CallRecord::Status(foreign_iri().to_string()),
            "status must use FOREIGN iri"
        );
        assert_eq!(
            *client.delete_calls.borrow(),
            0,
            "delete must not be called for in-progress foreign dump"
        );
    }

    #[test]
    fn delete_exists_for_other_project_is_noop_no_status_delete_calls() {
        // Delete + ExistsForOtherProject → Ok, project_dump_deleted{deleted:false, note:Some},
        // no status/delete call.
        let dir = TempDir::new().unwrap();
        let (mut args, cfg) = make_args(&dir);
        args.delete = true;

        let client = MockDspClient::new()
            .with_resolve_project(Ok(make_project_ref()))
            .with_create_exists_other_project("foreign-dump-id", foreign_iri());

        let mut renderer = RecordingRenderer::new();
        let mut reporter = RecordingProgressReporter::new();

        run_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            &mut reporter,
            Some("tok".to_string()),
            &no_op_sleeper(),
            fixed_now(),
            None,
            dir.path(),
        )
        .unwrap(); // must succeed (exit 0)

        // project_dump must NOT be called.
        assert!(renderer.dump_outcome.is_none());
        // project_dump_deleted must be called with deleted:false + note.
        let del_outcome = renderer
            .dump_deleted_outcome
            .expect("project_dump_deleted must have been called");
        assert!(!del_outcome.deleted, "deleted must be false for foreign-slot no-op");
        let note = del_outcome.note.expect("note must be set for foreign-slot case");
        assert!(note.contains(foreign_iri()), "note must name the foreign project IRI: {note}");
        // No status/delete calls (only Resolve + Create).
        let log = client.call_log();
        assert_eq!(log[0], CallRecord::Resolve);
        assert_eq!(
            log[1],
            CallRecord::Create { project_iri: "http://rdfh.ch/projects/0001".to_string() }
        );
        assert_eq!(log.len(), 2, "must be exactly 2 calls (no status/delete)");
        assert_eq!(
            *client.delete_calls.borrow(),
            0,
            "delete must not be called for foreign-slot no-op"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // run_list_impl tests
    // ─────────────────────────────────────────────────────────────────────────

    const LIST_SERVER: &str = "https://api.test.dasch.swiss";

    fn make_list_args(filter: Option<&str>) -> ProjectListArgs {
        ProjectListArgs {
            server: Some(LIST_SERVER.to_string()),
            filter: filter.map(str::to_string),
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        }
    }

    fn make_list_cfg() -> Config {
        Config { server: LIST_SERVER.to_string() }
    }

    fn make_project(shortcode: &str, shortname: &str, longname: Option<&str>) -> Project {
        Project {
            iri: format!("http://rdfh.ch/projects/{shortcode}"),
            shortcode: shortcode.to_string(),
            shortname: shortname.to_string(),
            longname: longname.map(str::to_string),
            status: ProjectStatus::Active,
            data_models: 2,
        }
    }

    fn two_project_list() -> Vec<Project> {
        vec![
            make_project("0002", "images", None),
            make_project("0001", "anything", Some("Anything Project")),
        ]
    }

    /// Anonymous: no env token, empty temp cache → auth_state "anonymous",
    /// client called with token=None, full list rendered.
    #[test]
    fn list_anonymous_no_token_no_cache() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(None);
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path)).expect("must succeed anonymously");

        let meta = renderer.projects_meta.unwrap();
        assert_eq!(meta.auth_state, "anonymous");

        let recorded_token = client.list_projects_token();
        assert_eq!(
            recorded_token, None,
            "must call list_projects with token=None when no credentials"
        );

        let (items, total, filter) = renderer.projects_view.unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
        assert!(filter.is_none());
    }

    /// Corrupt/missing cache + no env token → still succeeds anonymously.
    /// Locks the auth-optional fallback in run_list_impl (PRD AC 2).
    #[test]
    fn list_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().unwrap();
        // Write a corrupt (non-TOML) auth.toml to force a parse error.
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"NOT VALID TOML }{").unwrap();

        let args = make_list_args(None);
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        // Must NOT return an error — falls back to anonymous.
        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("corrupt cache must not cause an error for list (auth-optional)");

        let meta = renderer.projects_meta.unwrap();
        assert_eq!(meta.auth_state, "anonymous");
        assert_eq!(client.list_projects_token(), None);
    }

    /// Env token → auth_state "authenticated via DSP_TOKEN",
    /// client called with Some(token).
    #[test]
    fn list_env_token_authenticated_via_dsp_token() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(None);
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            Some("my-env-token".to_string()),
            Some(&cache_path),
        )
        .expect("must succeed with env token");

        let meta = renderer.projects_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");

        // Assert the token was actually forwarded — using a wrong token would fail this.
        let recorded_token = client.list_projects_token();
        assert_eq!(
            recorded_token,
            Some("my-env-token".to_string()),
            "token must be forwarded to list_projects"
        );
    }

    /// Cache token with user → auth_state "authenticated as {user}".
    #[test]
    fn list_cache_token_with_user() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");

        let mut cache = AuthCache::default();
        cache.set_entry(
            LIST_SERVER,
            ServerEntry {
                token: "cache-token-xyz".to_string(),
                user: Some("alice@example.com".to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let args = make_list_args(None);
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("must succeed with cache token");

        let meta = renderer.projects_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated as alice@example.com");

        // Token must be forwarded (not None).
        let recorded_token = client.list_projects_token();
        assert_eq!(
            recorded_token,
            Some("cache-token-xyz".to_string()),
            "cache token must be forwarded to list_projects"
        );
    }

    /// --filter matches a subset case-insensitively.
    /// `total` is pre-filter, shown count is post-filter.
    #[test]
    fn list_filter_matches_subset_case_insensitively() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        // Use a filter that matches "anything" case-insensitively but not "images".
        let args = make_list_args(Some("ANYTH"));
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("filter must not cause an error");

        let (items, total, filter) = renderer.projects_view.unwrap();
        assert_eq!(total, 2, "total must be pre-filter count");
        assert_eq!(items.len(), 1, "only one project matches 'ANYTH'");
        assert_eq!(items[0].shortname, "anything");
        assert_eq!(filter.as_deref(), Some("ANYTH"));
    }

    /// Non-matching filter → empty items list, `total` is still the full count.
    #[test]
    fn list_filter_no_match_returns_empty_items() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(Some("zzz-no-match-zzz"));
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("no-match filter must not be an error");

        let (items, total, _) = renderer.projects_view.unwrap();
        assert_eq!(total, 2, "total must still show pre-filter count");
        assert!(items.is_empty(), "items must be empty when filter matches nothing");
    }

    /// Sort: unsorted mock response → renderer receives shortcode-ascending order.
    #[test]
    fn list_results_sorted_by_shortcode_ascending() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(None);
        let cfg = make_list_cfg();

        // Provide projects in reverse shortcode order.
        let unsorted = vec![
            make_project("0003", "proj-c", None),
            make_project("0001", "proj-a", None),
            make_project("0002", "proj-b", None),
        ];

        let client = MockDspClient::new().with_list_projects_result(Ok(unsorted));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path)).expect("sort must not error");

        let (items, _, _) = renderer.projects_view.unwrap();
        let shortcodes: Vec<&str> = items.iter().map(|p| p.shortcode.as_str()).collect();
        assert_eq!(shortcodes, vec!["0001", "0002", "0003"]);
    }

    /// Filter matches via longname (case-insensitive substring).
    #[test]
    fn list_filter_matches_longname_case_insensitively() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        // Filter "anything project" in upper case — must match longname "Anything Project".
        let args = make_list_args(Some("ANYTHING PROJECT"));
        let cfg = make_list_cfg();

        // two_project_list() has one project with longname "Anything Project".
        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("longname filter must not error");

        let (items, total, _) = renderer.projects_view.unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].shortname, "anything");
    }

    /// Filter matches via shortcode.
    #[test]
    fn list_filter_matches_shortcode() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(Some("0002"));
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("shortcode filter must not error");

        let (items, total, _) = renderer.projects_view.unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].shortcode, "0002");
    }

    /// `Config::resolve(None, _)` with no server yields a Usage error (exit 2).
    /// Covers PRD AC 6 — no-server check is at the dispatch layer.
    #[test]
    fn config_resolve_none_returns_usage_error() {
        let err = crate::config::Config::resolve(None, false).unwrap_err();
        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage diagnostic for missing server, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("--server") || msg.contains("DSP_SERVER"), "{msg}");
    }

    /// Token assertion is real: passing the wrong expected token should fail the test.
    /// This is a compile/logic check — we assert that "wrong-token" != "my-env-token".
    #[test]
    fn list_token_assertion_is_real() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_list_args(None);
        let cfg = make_list_cfg();

        let client = MockDspClient::new().with_list_projects_result(Ok(two_project_list()));
        let mut renderer = RecordingRenderer::new();

        run_list_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            Some("my-env-token".to_string()),
            Some(&cache_path),
        )
        .unwrap();

        let recorded = client.list_projects_token();
        // Verify the correct token was forwarded and that "wrong-token" != "my-env-token"
        assert_eq!(recorded, Some("my-env-token".to_string()));
        assert_ne!(
            recorded,
            Some("wrong-token".to_string()),
            "token assertion must be real: wrong token should not match"
        );
    }

    // run_describe_impl tests
    // ─────────────────────────────────────────────────────────────────────────

    const DESCRIBE_SERVER: &str = "https://api.test.dasch.swiss";

    fn make_describe_args(project: Option<&str>) -> ProjectDescribeArgs {
        ProjectDescribeArgs {
            server: Some(DESCRIBE_SERVER.to_string()),
            project: project.map(str::to_string),
            format: FormatArgs {
                format: Format::Prose,
                json: false,
                lines: false,
                columns: None,
                no_header: false,
                header_only: false,
            },
        }
    }

    fn make_describe_cfg() -> Config {
        Config { server: DESCRIBE_SERVER.to_string() }
    }

    /// Build a realistic `ProjectDetail` fixture (beol-shaped).
    fn make_project_detail() -> ProjectDetail {
        ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".to_string(),
            shortcode: "0801".to_string(),
            shortname: "beol".to_string(),
            longname: Some("Bernoulli-Euler Online".to_string()),
            status: ProjectStatus::Active,
            description: vec![ProjectDescription {
                value: "A project about Bernoulli and Euler.".to_string(),
                language: Some("en".to_string()),
            }],
            keywords: vec!["Bernoulli".to_string(), "Euler".to_string()],
            data_models: vec![
                DataModelSummary {
                    name: "beol".to_string(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".to_string(),
                },
                DataModelSummary {
                    name: "biblio".to_string(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".to_string(),
                },
            ],
        }
    }

    /// Success: mock returns a `ProjectDetail`; renderer records the detail + meta.
    #[test]
    fn describe_success_records_detail_and_meta() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();
        let detail = make_project_detail();

        let client = MockDspClient::new().with_describe_project_result(Ok(detail.clone()));
        let mut renderer = RecordingRenderer::new();

        run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path)).expect("describe must succeed");

        let recorded_detail = renderer.describe_detail.unwrap();
        assert_eq!(recorded_detail, detail, "renderer must receive the exact ProjectDetail");

        let meta = renderer.describe_meta.unwrap();
        assert_eq!(meta.server_label, DESCRIBE_SERVER);
        assert_eq!(meta.auth_state, "anonymous");
        assert!(meta.filter_warning.is_none());
    }

    /// Success: assert the `--project` arg and token were forwarded to the client.
    #[test]
    fn describe_forwards_project_and_token_to_client() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new().with_describe_project_result(Ok(make_project_detail()));
        let mut renderer = RecordingRenderer::new();

        run_describe_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            Some("my-env-token".to_string()),
            Some(&cache_path),
        )
        .expect("describe must succeed");

        let (project_arg, token_arg) = client.describe_project_call();
        assert_eq!(project_arg, "0801", "project argument must be forwarded");
        assert_eq!(
            token_arg,
            Some("my-env-token".to_string()),
            "env token must be forwarded to describe_project"
        );
    }

    /// `not_found`: mock returns `Diagnostic::NotFound`; action propagates the error.
    #[test]
    fn describe_not_found_propagates_error() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(Some("9999"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new()
            .with_describe_project_result(Err(Diagnostic::NotFound("project '9999' not found".to_string())));
        let mut renderer = RecordingRenderer::new();

        let err = run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path)).unwrap_err();

        assert!(matches!(err, Diagnostic::NotFound(_)), "expected NotFound, got {err:?}");
    }

    /// Missing `--project` → `Diagnostic::Usage` with the expected message.
    /// The guard fires BEFORE any cache/IO — no client call must be made.
    #[test]
    fn describe_missing_project_returns_usage_error() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(None); // no --project
        let cfg = make_describe_cfg();

        // No canned result — if describe_project is called, the mock returns NotImplemented.
        let client = MockDspClient::new();
        let mut renderer = RecordingRenderer::new();

        let err = run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path)).unwrap_err();

        assert!(
            matches!(err, Diagnostic::Usage(_)),
            "expected Usage diagnostic for missing --project, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("--project"), "--project must appear in the usage message: {msg}");

        // No server call must have been made (fail-fast guard fires before IO).
        assert!(
            renderer.describe_detail.is_none(),
            "renderer must not be called when --project is missing"
        );
        // The guard fires BEFORE any client call — fail-fast means no IO.
        assert!(
            !client.describe_project_was_called(),
            "client.describe_project must not be called when --project is missing"
        );
    }

    /// Auth-state anonymous: no env token, empty temp cache → "anonymous".
    #[test]
    fn describe_anonymous_no_token_no_cache() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new().with_describe_project_result(Ok(make_project_detail()));
        let mut renderer = RecordingRenderer::new();

        run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("must succeed anonymously");

        let meta = renderer.describe_meta.unwrap();
        assert_eq!(meta.auth_state, "anonymous");

        let (_, token_arg) = client.describe_project_call();
        assert_eq!(token_arg, None, "no token must be forwarded when anonymous");
    }

    /// Auth-state via env token → "authenticated via DSP_TOKEN".
    #[test]
    fn describe_env_token_authenticated_via_dsp_token() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new().with_describe_project_result(Ok(make_project_detail()));
        let mut renderer = RecordingRenderer::new();

        run_describe_impl(
            &args,
            &cfg,
            &client,
            &mut renderer,
            Some("env-token-xyz".to_string()),
            Some(&cache_path),
        )
        .expect("must succeed with env token");

        let meta = renderer.describe_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated via DSP_TOKEN");

        let (_, token_arg) = client.describe_project_call();
        assert_eq!(
            token_arg,
            Some("env-token-xyz".to_string()),
            "env token must be forwarded to describe_project"
        );
    }

    /// Auth-state via cache token with user → "authenticated as <user>".
    #[test]
    fn describe_cache_token_with_user() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");

        let mut cache = AuthCache::default();
        cache.set_entry(
            DESCRIBE_SERVER,
            ServerEntry {
                token: "cache-token-abc".to_string(),
                user: Some("bob@example.com".to_string()),
                acquired_at: None,
                expires_at: None,
            },
        );
        cache.save_to(&cache_path).unwrap();

        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new().with_describe_project_result(Ok(make_project_detail()));
        let mut renderer = RecordingRenderer::new();

        run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("must succeed with cache token");

        let meta = renderer.describe_meta.unwrap();
        assert_eq!(meta.auth_state, "authenticated as bob@example.com");

        let (_, token_arg) = client.describe_project_call();
        assert_eq!(
            token_arg,
            Some("cache-token-abc".to_string()),
            "cache token must be forwarded to describe_project"
        );
    }

    /// Corrupt cache + no env token → still succeeds anonymously (never Err).
    /// Locks the auth-optional fallback in run_describe_impl (dsp-cli/ADR-0007).
    #[test]
    fn describe_corrupt_cache_falls_back_to_anonymous() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join("auth.toml");
        std::fs::write(&cache_path, b"NOT VALID TOML }{").unwrap();

        let args = make_describe_args(Some("0801"));
        let cfg = make_describe_cfg();

        let client = MockDspClient::new().with_describe_project_result(Ok(make_project_detail()));
        let mut renderer = RecordingRenderer::new();

        // Must NOT return an error — falls back to anonymous.
        run_describe_impl(&args, &cfg, &client, &mut renderer, None, Some(&cache_path))
            .expect("corrupt cache must not cause an error for describe (auth-optional)");

        let meta = renderer.describe_meta.unwrap();
        assert_eq!(meta.auth_state, "anonymous");

        let (_, token_arg) = client.describe_project_call();
        assert_eq!(token_arg, None, "no token must be forwarded when anonymous");
    }
}
