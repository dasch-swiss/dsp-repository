//! Shared rendering helpers for the per-format renderers.
//!
//! This module serves three overlapping concerns:
//!
//! **Quoting helpers** (`csv_field`): `csv_field` is the RFC-4180 quoting
//! helper; it lives here (not in csv.rs) so the formula-injection check is
//! applied consistently. ASCII control-char neutralisation for all three
//! tabular formats is `crate::util::text::replace_control_chars` (a sibling of
//! `strip_control_chars`, the prose sanitiser). `render_table_row` is a
//! test-only helper (the tabular renderers moved to the shared engine in plan
//! 020).
//!
//! **ADR-0007 disclosure/footer writers** (`render_table_disclosure`,
//! `render_prose_footer`): the auth-state disclosure line written by every
//! noun method. Tabular formats (lines, csv, tsv) write it to `stderr`;
//! prose/json carry it on stdout — via a footer (`render_prose_footer`) or the
//! `_meta.auth` JSON key respectively — using a different sink than tabular.
//! These helpers centralise the 31-site duplication without adding a new module
//! (accepted trade-off at plan 019 design review, 2026-06-11). Since plan 030
//! they also carry the schema-side `--count` caveat (`MetaContext.count_caveat`)
//! alongside the ADR-0007 `filter_warning`, combined via the private
//! `disclosure_suffix` helper.
//!
//! **Shared table engine** (`render_table`, `TableSpec`, `TableOptions`,
//! `HeaderMode`, `QuoteMode`): a projection/header-control engine used by the
//! csv, tsv, and lines renderers in steps 2–4 of plan 020. Column-set consts
//! (`PROJECTS_COLUMNS`, etc.) live here as the single source of truth for each
//! noun group; they feed the engine's unknown-name error hint and the CLI's
//! `--help` `after_help` text via `crate::render`'s published surface.

use std::io::{self, Write};

use crate::diagnostic::Diagnostic;
use crate::util::text::replace_control_chars;

use super::MetaContext;

/// Escape a CSV field per RFC 4180: wrap in double-quotes if the value
/// contains a comma, double-quote, or newline. Internal double-quotes are
/// escaped by doubling.
///
/// **Formula-injection mitigation (spreadsheet safety):** fields that *begin*
/// with `=`, `+`, `-`, or `@` are also wrapped in quotes. RFC-4180 quoting
/// does not fully prevent spreadsheet applications from evaluating such fields
/// as formulas — a leading `=foo` inside `"=foo"` is still formula-eligible in
/// some apps. We deliberately do **not** prefix-escape (e.g. prefix with `'`)
/// because that would corrupt the data for legitimate consumers. This is an
/// accepted residual risk under the personal-CLI threat model where the user
/// controls the data source. A snapshot fixture locks this behaviour.
pub(crate) fn csv_field(s: &str) -> String {
    let needs_quoting = s.contains(',')
        || s.contains('"')
        || s.contains('\n')
        || matches!(s.chars().next(), Some('=' | '+' | '-' | '@'));
    if needs_quoting {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Write the ADR-0007 auth-state disclosure line to `err` (stderr).
///
/// Tabular formats (lines, csv, tsv) call this once per noun method, writing
/// `[{auth_state} on {server_label}]\n` to their stderr sink. Prose and JSON
/// carry the disclosure on stdout instead — via `render_prose_footer` and the
/// `_meta.auth` key respectively — so this helper is **tabular formats only**.
///
/// Returns `io::Result<()>`. This helper can only fail on IO; that is why it
/// returns `io::Result` rather than the render layer's usual
/// `Result<(), Diagnostic>`. If a future change ever needs to surface a non-IO
/// error here, switch the return type to `Diagnostic` at that point.
pub(crate) fn render_table_disclosure(err: &mut dyn Write, meta: &MetaContext) -> io::Result<()> {
    match disclosure_suffix(meta) {
        None => writeln!(err, "[{} on {}]", meta.auth_state, meta.server_label),
        Some(note) => writeln!(
            err,
            "[{} on {}] — {note}",
            meta.auth_state, meta.server_label
        ),
    }
}

/// Write the ADR-0007 footer (blank line then disclosure) to `out` (stdout).
///
/// Prose renderer calls this once per noun method. The helper owns the
/// preceding blank line, so a prose call site is exactly one line. The
/// disclosure format is `[{auth_state} on {server_label}]\n`, written to
/// stdout (not stderr) — consistent with prose writing all output to a single
/// stream.
///
/// Returns `io::Result<()>`. This helper can only fail on IO; that is why it
/// returns `io::Result` rather than the render layer's usual
/// `Result<(), Diagnostic>`. If a future change ever needs to surface a non-IO
/// error here, switch the return type to `Diagnostic` at that point.
pub(crate) fn render_prose_footer(out: &mut dyn Write, meta: &MetaContext) -> io::Result<()> {
    writeln!(out)?;
    match disclosure_suffix(meta) {
        None => writeln!(out, "[{} on {}]", meta.auth_state, meta.server_label),
        Some(note) => writeln!(
            out,
            "[{} on {}] — {note}",
            meta.auth_state, meta.server_label
        ),
    }
}

/// Combine `filter_warning` (ADR-0007, instance-side), `count_caveat`
/// (schema-side `--count`, plan 030), and `count_cost` (`vocabulary list
/// --count` cost disclosure, plan 034) into one disclosure suffix. `None`
/// when none are set. This is the SAME suffix both `render_table_disclosure`
/// and `render_prose_footer` append — see their docs. `count_cost` is
/// deliberately the LAST element so composition reads `filter_warning;
/// count_caveat; count_cost` when more than one is set.
fn disclosure_suffix(meta: &MetaContext) -> Option<String> {
    let parts: Vec<&str> = [
        meta.filter_warning.as_deref(),
        meta.count_caveat.as_deref(),
        meta.count_cost.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

// ── Header/quote mode types ───────────────────────────────────────────────────

/// Controls which rows are emitted by `render_table`.
///
/// `On` is the unflagged default: a header row is emitted followed by data rows.
/// `Off` suppresses the header entirely; only data rows are written.
/// `Only` emits the header row and no data rows (the action still runs normally —
/// see D3 in the plan 020 decision record).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeaderMode {
    /// Header row + data rows (default).
    #[default]
    On,
    /// Data rows only, no header.
    Off,
    /// Header row only, no data rows.
    Only,
}

/// Per-format quoting strategy, dispatched inside `render_table`.
///
/// All three formats neutralise ASCII control characters via
/// `replace_control_chars` (ADR-0003), so a server-controlled cell can never
/// emit a raw ESC/DEL/etc. to the terminal or corrupt the delimited structure.
/// They differ in separator and additional quoting:
/// - `Csv` → `","`; `replace_control_chars` then RFC-4180 quoting via `csv_field`
/// - `Tsv` → `"\t"`; `replace_control_chars` (also prevents an embedded tab/newline
///   from splitting a column)
/// - `Lines` → `"\t"`; `replace_control_chars`
///
/// No separate separator field exists: the separator is always derived from the
/// mode so callers cannot accidentally mismatch the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuoteMode {
    /// Control-char neutralisation + RFC-4180 quoting + formula-injection
    /// mitigation. Separator: `,`.
    Csv,
    /// Control-char neutralisation (all C0 + DEL → space). Separator: `\t`.
    Tsv,
    /// Control-char neutralisation (all C0 + DEL → space). Separator: `\t`.
    Lines,
}

impl QuoteMode {
    fn sep(self) -> &'static str {
        match self {
            QuoteMode::Csv => ",",
            QuoteMode::Tsv | QuoteMode::Lines => "\t",
        }
    }

    fn apply(self, s: &str) -> String {
        match self {
            // Neutralise control chars first, then RFC-4180 quote. After
            // `replace_control_chars` no `\n` reaches `csv_field`, so its
            // newline-quoting branch is unreachable from this path (kept
            // defensively; `csv_field` is not narrowed — out of scope).
            QuoteMode::Csv => csv_field(&replace_control_chars(s)),
            QuoteMode::Tsv | QuoteMode::Lines => replace_control_chars(s),
        }
    }
}

// ── TableOptions ─────────────────────────────────────────────────────────────

/// Per-invocation tabular options resolved from the CLI flags by
/// `FormatArgs::table_options()` (step 4). Lives here so it can be
/// construction-time validated (syntax only — unknown-name validation
/// happens inside the engine where the per-noun column set is known).
///
/// `Default` gives the unflagged behaviour: all columns, header on.
#[derive(Debug, Default)]
pub struct TableOptions {
    /// User-supplied `--columns` selection, validated for syntax by
    /// `table_options()`: non-empty, no blank segments, no duplicates.
    /// `None` means the flag was not supplied.
    pub columns: Option<Vec<String>>,
    /// Resolved header mode. `On` is the unflagged default.
    pub header: HeaderMode,
}

impl TableOptions {
    /// Borrow the column projection as `Option<Vec<&str>>`, ready to pass to
    /// `TableSpec::projected`.
    ///
    /// Returns `None` when `--columns` was not supplied (engine falls through
    /// to `default_columns` or `all_columns`). Returns `Some(vec)` when the
    /// flag was supplied; each element borrows from `self.columns`.
    pub fn projected(&self) -> Option<Vec<&str>> {
        self.columns
            .as_ref()
            .map(|c| c.iter().map(String::as_str).collect())
    }
}

// ── Per-noun column-set consts ────────────────────────────────────────────────
//
// Single source of truth for each noun group's column set.  These consts are:
//   (a) referenced by `TableSpec.all_columns` in each renderer method body,
//   (b) used by the engine to build the unknown-column error hint (names appear
//       in declaration order, giving a stable, snapshot-stable message), and
//   (c) re-exported through `crate::render`'s published surface for the
//       `after_help` drift-guard tests in the CLI layer (step 5).
//
// Column order follows the CSV header order in csv.rs (the authoritative set).

/// Column set for `project list` and `project describe`.
pub(crate) const PROJECTS_COLUMNS: &[&str] = &[
    "shortcode",
    "shortname",
    "longname",
    "status",
    "data_models",
    "iri",
];

/// Column set for `data-model list`.
pub(crate) const DATA_MODELS_COLUMNS: &[&str] =
    &["name", "iri", "label", "last_modified", "is_builtin"];

/// Column set for `data-model describe`.
pub(crate) const DATA_MODEL_DESCRIBE_COLUMNS: &[&str] =
    &["name", "iri", "label", "last_modified", "resource_types"];

/// Column set for `resource-type list`.
pub(crate) const RESOURCE_TYPES_COLUMNS: &[&str] = &["name", "iri", "label", "is_builtin", "count"];

/// Default (unflagged) columns for `resource-type list` csv/tsv — the
/// original 4-column set, WITHOUT `count`. `count` is still a valid
/// `--columns` name (part of `RESOURCE_TYPES_COLUMNS`) but must never appear
/// unrequested when `--count` was not passed (would be a default-output
/// change — out of scope per plan 030). csv/tsv choose between this and
/// `None` (all 5) dynamically per-call based on whether any item actually
/// carries a count — see their `resource_types` methods.
pub(crate) const RESOURCE_TYPES_DEFAULT_COLUMNS: &[&str] = &["name", "iri", "label", "is_builtin"];

/// Column set for `resource-type describe` (one row per field).
///
/// The full set is 8 columns: `iri` is at position 1 (after `name`), matching
/// the lines renderer's lean default of `["name", "iri"]`. csv/tsv use this set
/// with `default_columns: Some(&["name","value_type","link_target","cardinality",
/// "label","is_builtin","data_model"])` — a 7-column lean default identical to
/// the pre-020 csv/tsv header (byte-identical output for unflagged invocations).
/// The `iri` column is unlocked via `--columns iri` on all three formats.
pub(crate) const RESOURCE_TYPE_DESCRIBE_COLUMNS: &[&str] = &[
    "name",
    "iri",
    "value_type",
    "link_target",
    "cardinality",
    "label",
    "is_builtin",
    "data_model",
];

/// Lean default subset for `resource-type describe` csv/tsv (matches the
/// pre-020 7-column csv/tsv header; `iri` is hidden by default, accessible
/// via `--columns iri`).
pub(crate) const RESOURCE_TYPE_DESCRIBE_DEFAULT_COLUMNS: &[&str] = &[
    "name",
    "value_type",
    "link_target",
    "cardinality",
    "label",
    "is_builtin",
    "data_model",
];

/// Column set for `resource list`.
///
/// Default columns = all (no lean default const). All six columns are shown
/// in every tabular format (matching `project list` / `resource-type list`).
pub(crate) const RESOURCE_LIST_COLUMNS: &[&str] = &[
    "label",
    "iri",
    "ark_url",
    "creation_date",
    "last_modified",
    "resource_type",
];

/// Column set for `resource describe`.
///
/// Default columns = all (no lean default const). All ten columns are shown
/// in every tabular format, mirroring `resource list`. `None` fields render
/// as empty strings in tabular output.
pub(crate) const RESOURCE_DESCRIBE_COLUMNS: &[&str] = &[
    "label",
    "iri",
    "resource_type",
    "ark_url",
    "creation_date",
    "last_modified",
    "attached_project",
    "owner",
    "visibility",
    "your_access",
];

/// Column set for `resource describe --values` (long-format, one row per
/// value). `label`/`iri` are the leading key columns (ADR-0013 option 1).
pub(crate) const RESOURCE_DESCRIBE_VALUES_COLUMNS: &[&str] = &[
    "label",
    "iri",
    "field",
    "field_label",
    "value_type",
    "value",
    "comment",
];

/// Default columns for `resource describe --values` (all three tabular
/// formats). `label`/`iri` are omitted by default — they are constant across
/// every value row of a single-resource describe, so repeating them is pure
/// redundancy; they stay available via `--columns label,iri,…` for callers
/// who want self-contained/greppable rows.
pub(crate) const RESOURCE_DESCRIBE_VALUES_DEFAULT_COLUMNS: &[&str] =
    &["field", "field_label", "value_type", "value"];

/// Column set for `data-model structure`.
pub(crate) const DATA_MODEL_STRUCTURE_COLUMNS: &[&str] =
    &["source", "target", "kind", "field", "target_data_model"];

/// Column set for `auth login`, `auth status`, and `auth set-token`.
pub(crate) const AUTH_LOGIN_COLUMNS: &[&str] = &["server", "user", "expires_at", "state"];

/// Column set for `auth logout`.
pub(crate) const AUTH_LOGOUT_COLUMNS: &[&str] = &["server", "was_cached"];

/// Column set for `project dump`.
pub(crate) const PROJECT_DUMP_COLUMNS: &[&str] = &["path"];

/// Column set for `project dump --delete`.
pub(crate) const PROJECT_DUMP_DELETED_COLUMNS: &[&str] = &["deleted"];

/// Column set for `vocabulary list` (plan 034 D5/D7/D9). One row per
/// vocabulary; one column per language (`en, de, fr, it, rm`, D13 order) plus
/// an untagged slot, for both labels and comments — no preferred-language
/// collapsing (D4). `nodes`/`depth` are populated only under `--count`
/// (`Vocabulary.node_count`/`depth`); they sit outside the default set (see
/// `VOCABULARIES_DEFAULT_COLUMNS`) so they never appear unrequested, mirroring
/// `RESOURCE_TYPES_COLUMNS`'s `count` column.
pub(crate) const VOCABULARIES_COLUMNS: &[&str] = &[
    "name",
    "iri",
    "label_en",
    "label_de",
    "label_fr",
    "label_it",
    "label_rm",
    "label",
    "comment_en",
    "comment_de",
    "comment_fr",
    "comment_it",
    "comment_rm",
    "comment",
    "nodes",
    "depth",
];

/// Default (unflagged) columns for `vocabulary list` csv/tsv — every
/// language/untagged label column, WITHOUT `nodes`/`depth`. csv/tsv choose
/// between this and `VOCABULARIES_COUNTED_DEFAULT_COLUMNS` dynamically
/// per-call based on whether any item actually carries a count (mirroring
/// `RESOURCE_TYPES_DEFAULT_COLUMNS`'s documented behaviour), not merely on
/// `VocabularyListView.counted` — a `--count` run whose every per-tree fetch
/// failed must not emit two permanently blank columns.
pub(crate) const VOCABULARIES_DEFAULT_COLUMNS: &[&str] = &[
    "name", "iri", "label_en", "label_de", "label_fr", "label_it", "label_rm", "label",
];

/// Default columns for `vocabulary list` csv/tsv when at least one item
/// carries a count — `VOCABULARIES_DEFAULT_COLUMNS` plus `nodes`/`depth`. A
/// second const exists (rather than building the default at call time)
/// because `TableSpec::default_columns` is `Option<&'a [&'a str]>` and cannot
/// borrow a locally-built `Vec<&str>`.
pub(crate) const VOCABULARIES_COUNTED_DEFAULT_COLUMNS: &[&str] = &[
    "name", "iri", "label_en", "label_de", "label_fr", "label_it", "label_rm", "label", "nodes",
    "depth",
];

/// Column set for `vocabulary describe` (one row per node, DFS order; plan 034
/// D5/D7/D9/D10/D11). `number` (D10) is the 1-based dotted outline position;
/// `position` stays the raw 0-based DSP value. `path` (D11) is the per-segment
/// language-fallback breadcrumb. `depth` here is the per-node absolute depth
/// column (distinct from `VocabularyDetail.depth`, the branch-relative
/// summary the prose header line renders — see `src/render/vocabulary.rs`).
pub(crate) const VOCABULARY_DESCRIBE_COLUMNS: &[&str] = &[
    "node_iri",
    "number",
    "name",
    "label_en",
    "label_de",
    "label_fr",
    "label_it",
    "label_rm",
    "label",
    "comment_en",
    "comment_de",
    "comment_fr",
    "comment_it",
    "comment_rm",
    "comment",
    "path",
    "position",
    "depth",
    "parent_iri",
];

/// Default (unflagged) columns for `vocabulary describe` csv/tsv — `node_iri`,
/// `number`, plus every language/untagged label column (8 total). No dynamic
/// second default here: unlike `list`, `nodes`/`depth` are describe's
/// summary-line values, not per-row tabular columns in this set.
pub(crate) const VOCABULARY_DESCRIBE_DEFAULT_COLUMNS: &[&str] = &[
    "node_iri", "number", "label_en", "label_de", "label_fr", "label_it", "label_rm", "label",
];

// ── TableSpec and render_table ────────────────────────────────────────────────

/// One table to render, described by named fields.
///
/// ## Column-set precedence (engine contract)
///
/// The effective column set is chosen in this order:
///
/// 1. `projected` — user-supplied `--columns` selection (select AND reorder).
/// 2. `default_columns` — the lean subset used by the lines renderer when no
///    `--columns` flag is given. `Some(&[])` means zero columns (degenerate
///    case — the engine emits nothing for data rows). `None` means "same as
///    `all_columns`".
/// 3. `all_columns` — the full set, used when neither of the above is present.
///
/// This contract is pinned by unit tests in this module.
pub(crate) struct TableSpec<'a> {
    /// Full column set in csv-header declaration order. Used as the valid-name
    /// registry for unknown-column error hints.
    pub all_columns: &'a [&'a str],
    /// Full-width data rows. Each `Vec<String>` must have the same length as
    /// `all_columns`; the engine indexes into it by position.
    pub rows: &'a [Vec<String>],
    /// Lines renderer's lean default subset; `None` = all columns. See
    /// precedence contract above. `Some(&[])` → zero columns.
    pub default_columns: Option<&'a [&'a str]>,
    /// User's `--columns` selection, borrowed from `TableOptions::columns`.
    /// Build with `options.projected()` — the `TableOptions::projected()` helper
    /// returns the `Option<Vec<&str>>` ready to assign here.
    /// `None` = flag not supplied; the engine falls through to `default_columns`
    /// or `all_columns`.
    pub projected: Option<Vec<&'a str>>,
    /// Quoting strategy; also determines the column separator (no separate `sep`
    /// field — the engine derives it to prevent caller mismatches).
    pub quote: QuoteMode,
    /// Effective header mode. The **caller** computes this:
    /// - csv/tsv pass `options.header` directly.
    /// - lines passes `HeaderMode::Off` unconditionally (upstream validation
    ///   prevents the user from setting header flags with lines; the engine
    ///   never sees two authoritative header sources).
    pub header: HeaderMode,
}

/// Render a table to `out` according to `spec`.
///
/// ## Validation (runs before any output is written)
///
/// If `spec.projected` contains a name not in `spec.all_columns`, returns
/// `Diagnostic::Usage` naming the unknown column and listing the valid names
/// in `all_columns` declaration order. Nothing is written to `out` before
/// this check completes — including in `HeaderMode::Only`.
///
/// ## Emission
///
/// - `HeaderMode::On`: header row, then data rows.
/// - `HeaderMode::Off`: data rows only.
/// - `HeaderMode::Only`: header row only (no data rows emitted regardless of
///   `spec.rows`).
///
/// The effective column set is resolved per the precedence contract on
/// [`TableSpec`]. Column cells in data rows are selected and reordered to
/// match the effective column set; quoting is applied per `spec.quote` to
/// data cells. Header cells are written as plain literals (no quoting).
pub(crate) fn render_table(out: &mut dyn Write, spec: &TableSpec<'_>) -> Result<(), Diagnostic> {
    // Invariant: default_columns, when present, must be a subset of all_columns.
    // This is a renderer-layer contract (callers supply the consts); a violation
    // is an internal defect, not a user error.
    debug_assert!(
        spec.default_columns
            .map(|defs| defs.iter().all(|n| spec.all_columns.contains(n)))
            .unwrap_or(true),
        "default_columns must be a subset of all_columns (internal invariant)"
    );

    // Resolve effective column set.
    let effective_columns: &[&str] = if let Some(ref proj) = spec.projected {
        // Validate all projected names before writing anything.
        for name in proj.iter() {
            if !spec.all_columns.contains(name) {
                let valid = spec.all_columns.join(", ");
                return Err(Diagnostic::Usage(format!(
                    "unknown column \"{name}\"; valid columns: {valid}"
                )));
            }
        }
        proj.as_slice()
    } else if let Some(defaults) = spec.default_columns {
        defaults
    } else {
        spec.all_columns
    };

    let sep = spec.quote.sep();

    // Build index map: effective column name → position in all_columns.
    // We use a Vec<usize> aligned to effective_columns for row projection.
    //
    // After the validation above, every name in effective_columns is guaranteed
    // to be in all_columns (projected names are validated above; default_columns
    // and all_columns are renderer-layer consts). `.position(…)` should always
    // succeed here. If it does not, that is an internal invariant breach — return
    // an Internal diagnostic rather than panic (no unwrap/expect in non-test code).
    let col_indices: Vec<usize> = effective_columns
        .iter()
        .map(|name| {
            spec.all_columns
                .iter()
                .position(|c| c == name)
                .ok_or_else(|| {
                    Diagnostic::Internal(format!(
                        "column index missing for \"{name}\" after validation \
                         (all_columns=[{}]); this is a dsp-cli bug",
                        spec.all_columns.join(", ")
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Header row.
    if matches!(spec.header, HeaderMode::On | HeaderMode::Only) {
        let header_line = effective_columns.join(sep);
        writeln!(out, "{header_line}")?;
    }

    // Data rows (skip entirely for HeaderMode::Only).
    if !matches!(spec.header, HeaderMode::Only) {
        for row in spec.rows {
            let mut cells: Vec<String> = Vec::with_capacity(col_indices.len());
            for &idx in &col_indices {
                // Same no-panic policy as the column-index map above: a row
                // narrower than all_columns is an internal defect, not a
                // user error.
                let value = row.get(idx).ok_or_else(|| {
                    Diagnostic::Internal(format!(
                        "row has {} cells, expected {} (column index {idx} out of \
                         range); this is a dsp-cli bug",
                        row.len(),
                        spec.all_columns.len()
                    ))
                })?;
                cells.push(spec.quote.apply(value));
            }
            writeln!(out, "{}", cells.join(sep))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── render_table_row (test-only helper) ───────────────────────────────────
    //
    // render_table_row was a pub(crate) helper used only by the pre-020 CSV/TSV
    // renderer paths. The tabular renderers now route through the shared engine
    // (render_table). This function is preserved here for the tests that pin the
    // quoting and joining behaviour directly (which are still useful as unit
    // coverage for csv_field and replace_control_chars via the join path).

    fn render_table_row(fields: &[&str], sep: &str, quote: impl Fn(&str) -> String) -> String {
        fields
            .iter()
            .map(|f| quote(f))
            .collect::<Vec<_>>()
            .join(sep)
    }

    // ── render_table_disclosure ───────────────────────────────────────────────

    #[test]
    fn table_disclosure_anonymous_prod() {
        let meta = MetaContext {
            auth_state: "anonymous".to_string(),
            server_label: "prod".to_string(),
            filter_warning: None,
            count_caveat: None,
            count_cost: None,
        };
        let mut buf: Vec<u8> = Vec::new();
        render_table_disclosure(&mut buf, &meta).unwrap();
        assert_eq!(buf, b"[anonymous on prod]\n");
    }

    // ── render_prose_footer ───────────────────────────────────────────────────

    #[test]
    fn prose_footer_authenticated_test() {
        let meta = MetaContext {
            auth_state: "authenticated as you@dasch.swiss".to_string(),
            server_label: "test".to_string(),
            filter_warning: None,
            count_caveat: None,
            count_cost: None,
        };
        let mut buf: Vec<u8> = Vec::new();
        render_prose_footer(&mut buf, &meta).unwrap();
        assert_eq!(buf, b"\n[authenticated as you@dasch.swiss on test]\n");
    }

    #[test]
    fn table_disclosure_count_cost_only() {
        let meta = MetaContext {
            auth_state: "anonymous".to_string(),
            server_label: "prod".to_string(),
            filter_warning: None,
            count_caveat: None,
            count_cost: Some("--count costs one extra tree fetch per vocabulary".to_string()),
        };
        let mut buf: Vec<u8> = Vec::new();
        render_table_disclosure(&mut buf, &meta).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert_eq!(
            rendered,
            "[anonymous on prod] \u{2014} --count costs one extra tree fetch per vocabulary\n"
        );
    }

    #[test]
    fn table_disclosure_all_three_join_in_order() {
        let meta = MetaContext {
            auth_state: "anonymous".to_string(),
            server_label: "prod".to_string(),
            filter_warning: Some("filter-warning-text".to_string()),
            count_caveat: Some("count-caveat-text".to_string()),
            count_cost: Some("count-cost-text".to_string()),
        };
        let mut buf: Vec<u8> = Vec::new();
        render_table_disclosure(&mut buf, &meta).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert_eq!(
            rendered,
            "[anonymous on prod] \u{2014} filter-warning-text; count-caveat-text; count-cost-text\n"
        );
    }

    // ── render_table_row ──────────────────────────────────────────────────────

    #[test]
    fn table_row_csv_no_quoting_needed() {
        let result = render_table_row(&["abc", "def", "ghi"], ",", csv_field);
        assert_eq!(result, "abc,def,ghi");
    }

    #[test]
    fn table_row_csv_comma_in_field() {
        let result = render_table_row(&["foo", "bar,baz", "qux"], ",", csv_field);
        assert_eq!(result, r#"foo,"bar,baz",qux"#);
    }

    #[test]
    fn table_row_csv_quote_in_field() {
        let result = render_table_row(&[r#"say "hello""#], ",", csv_field);
        assert_eq!(result, r#""say ""hello""" "#.trim());
    }

    #[test]
    fn table_row_csv_newline_in_field() {
        let result = render_table_row(&["line1\nline2"], ",", csv_field);
        assert_eq!(result, "\"line1\nline2\"");
    }

    #[test]
    fn table_row_identity_tab_separator() {
        let result = render_table_row(&["alpha", "beta", "gamma"], "\t", |s: &str| s.to_string());
        assert_eq!(result, "alpha\tbeta\tgamma");
    }

    #[test]
    fn table_row_identity_comma_separator() {
        // identity closure: no quoting applied, comma not escaped
        let result = render_table_row(&["a,b", "c"], ",", |s: &str| s.to_string());
        assert_eq!(result, "a,b,c");
    }

    #[test]
    fn table_row_empty_fields() {
        let result = render_table_row(&["", "x", ""], "\t", |s: &str| s.to_string());
        assert_eq!(result, "\tx\t");
    }

    // ── csv_field quoting triggers ────────────────────────────────────────────

    #[test]
    fn csv_field_plain_string() {
        assert_eq!(csv_field("hello"), "hello");
    }

    #[test]
    fn csv_field_contains_comma() {
        assert_eq!(csv_field("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_field_contains_quote() {
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn csv_field_contains_newline() {
        assert_eq!(csv_field("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn csv_field_leading_equals_formula_injection() {
        assert_eq!(csv_field("=SUM(A1:A10)"), "\"=SUM(A1:A10)\"");
    }

    #[test]
    fn csv_field_leading_plus_formula_injection() {
        assert_eq!(csv_field("+1"), "\"+1\"");
    }

    #[test]
    fn csv_field_leading_minus_formula_injection() {
        assert_eq!(csv_field("-1"), "\"-1\"");
    }

    #[test]
    fn csv_field_leading_at_formula_injection() {
        assert_eq!(csv_field("@SUM"), "\"@SUM\"");
    }

    #[test]
    fn csv_field_not_leading_equals() {
        // `=` in the middle is not a formula trigger
        assert_eq!(csv_field("a=b"), "a=b");
    }

    #[test]
    fn csv_field_empty_string() {
        assert_eq!(csv_field(""), "");
    }

    // (control-char sanitiser tests moved to `crate::util::text` as
    //  `replace_control_chars_*` when the helper was relocated there.)

    // ── render_table: header modes ────────────────────────────────────────────

    fn make_spec<'a>(
        all: &'a [&'a str],
        rows: &'a [Vec<String>],
        projected: Option<Vec<&'a str>>,
        defaults: Option<&'a [&'a str]>,
        quote: QuoteMode,
        header: HeaderMode,
    ) -> TableSpec<'a> {
        TableSpec {
            all_columns: all,
            rows,
            default_columns: defaults,
            projected,
            quote,
            header,
        }
    }

    #[test]
    fn header_mode_on_non_empty_rows() {
        let all = &["a", "b"];
        let rows = vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::On),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\tb\n1\t2\n3\t4\n");
    }

    #[test]
    fn header_mode_on_empty_rows() {
        let all = &["x", "y"];
        let rows: Vec<Vec<String>> = vec![];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::On),
        )
        .unwrap();
        // Header only, no data rows.
        assert_eq!(String::from_utf8(buf).unwrap(), "x\ty\n");
    }

    #[test]
    fn header_mode_off_non_empty_rows() {
        let all = &["a", "b"];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Off),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "1\t2\n");
    }

    #[test]
    fn header_mode_off_empty_rows_yields_zero_bytes() {
        let all = &["a", "b"];
        let rows: Vec<Vec<String>> = vec![];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Off),
        )
        .unwrap();
        assert_eq!(buf, b"");
    }

    #[test]
    fn header_mode_only_non_empty_rows() {
        // Only mode: header emitted but data rows suppressed regardless of rows.
        let all = &["a", "b"];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Only),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\tb\n");
    }

    #[test]
    fn header_mode_only_empty_rows() {
        let all = &["a", "b"];
        let rows: Vec<Vec<String>> = vec![];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Only),
        )
        .unwrap();
        // Header row only.
        assert_eq!(String::from_utf8(buf).unwrap(), "a\tb\n");
    }

    // ── render_table: lines mode (QuoteMode::Lines + HeaderMode::Off) ─────────

    #[test]
    fn lines_mode_off_emits_no_header() {
        let all = &["name", "label"];
        let rows = vec![vec!["foo".to_string(), "bar".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Lines, HeaderMode::Off),
        )
        .unwrap();
        // No header, one tab-separated data row.
        assert_eq!(String::from_utf8(buf).unwrap(), "foo\tbar\n");
    }

    #[test]
    fn lines_mode_applies_replace_control_chars() {
        let all = &["name"];
        let rows = vec![vec!["a\tb".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Lines, HeaderMode::Off),
        )
        .unwrap();
        // Tab in value must be replaced by space.
        assert_eq!(String::from_utf8(buf).unwrap(), "a b\n");
    }

    // ── render_table: csv/tsv control-char neutralisation (ADR-0003) ──────────

    #[test]
    fn csv_mode_neutralises_control_chars() {
        let all = &["name"];
        let rows = vec![vec!["a\u{1b}\t\n\u{7f}b".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Csv, HeaderMode::Off),
        )
        .unwrap();
        // ESC, tab, newline, DEL each become a space; nothing left needs quoting.
        assert_eq!(String::from_utf8(buf).unwrap(), "a    b\n");
    }

    #[test]
    fn tsv_mode_neutralises_control_chars() {
        let all = &["name"];
        let rows = vec![vec!["a\u{1b}\u{7f}b".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Off),
        )
        .unwrap();
        // Previously TSV was identity and emitted ESC/DEL raw; now neutralised.
        assert_eq!(String::from_utf8(buf).unwrap(), "a  b\n");
    }

    #[test]
    fn tsv_embedded_tab_does_not_split_column() {
        // Two columns; the first value contains a tab. Previously (identity) the
        // embedded tab created a spurious extra column; now it becomes a space,
        // so the row has exactly one separator tab (between the two columns).
        let all = &["a", "b"];
        let rows = vec![vec!["x\ty".to_string(), "z".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::Off),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "x y\tz\n");
    }

    #[test]
    fn csv_leading_control_then_formula_char_not_quoted() {
        // A leading control char becomes a leading space, so the '=' is no longer
        // first and csv_field's formula-injection guard does not fire. This is
        // SAFE: a leading space defuses spreadsheet formula evaluation. Pinned so
        // the composition's behaviour can't silently regress.
        let all = &["name"];
        let rows = vec![vec!["\u{1b}=SUM(A1)".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Csv, HeaderMode::Off),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), " =SUM(A1)\n");
    }

    // ── render_table: projection select and reorder ──────────────────────────

    #[test]
    fn projection_selects_subset() {
        let all = &["a", "b", "c"];
        let rows = vec![vec!["1".to_string(), "2".to_string(), "3".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["a", "c"]),
                None,
                QuoteMode::Tsv,
                HeaderMode::On,
            ),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\tc\n1\t3\n");
    }

    #[test]
    fn projection_reorders_columns() {
        // User asks for c,a — engine must honour user order, not all_columns order.
        let all = &["a", "b", "c"];
        let rows = vec![vec!["1".to_string(), "2".to_string(), "3".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["c", "a"]),
                None,
                QuoteMode::Tsv,
                HeaderMode::On,
            ),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "c\ta\n3\t1\n");
    }

    #[test]
    fn projection_csv_quoting_applied_to_data_cells() {
        // A projected value that contains a comma must be csv_field-quoted.
        let all = &["name", "note"];
        let rows = vec![vec!["foo".to_string(), "a,b".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["name", "note"]),
                None,
                QuoteMode::Csv,
                HeaderMode::On,
            ),
        )
        .unwrap();
        // Data cell with comma gets quoted; header cells are plain literals.
        assert_eq!(String::from_utf8(buf).unwrap(), "name,note\nfoo,\"a,b\"\n");
    }

    #[test]
    fn projection_header_cells_are_plain_literals() {
        // Header must not be csv_field-quoted even if the column name contained a
        // comma (column names never do in practice, but the engine must not quote).
        let all = &["shortcode", "longname"];
        let rows = vec![vec!["0001".to_string(), "Project One".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Csv, HeaderMode::On),
        )
        .unwrap();
        // Header is plain, data is quoted only if needed.
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "shortcode,longname\n0001,Project One\n"
        );
    }

    // ── render_table: unknown-column error ───────────────────────────────────

    #[test]
    fn unknown_column_returns_usage_error_with_valid_names() {
        let all = &["shortcode", "shortname", "iri"];
        let rows: Vec<Vec<String>> = vec![];
        let mut buf: Vec<u8> = Vec::new();
        let result = render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["shortcode", "xyz"]),
                None,
                QuoteMode::Csv,
                HeaderMode::On,
            ),
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        // Message must name the unknown column.
        assert!(msg.contains("\"xyz\""), "message: {msg}");
        // Message must list valid names.
        assert!(msg.contains("shortcode"), "message: {msg}");
        assert!(msg.contains("shortname"), "message: {msg}");
        assert!(msg.contains("iri"), "message: {msg}");
    }

    #[test]
    fn unknown_column_error_before_any_output() {
        // Nothing must be written to `out` before the error is returned.
        let all = &["a", "b"];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        let result = render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["a", "z"]),
                None,
                QuoteMode::Csv,
                HeaderMode::On,
            ),
        );
        assert!(result.is_err());
        assert_eq!(buf, b"", "output must be empty when validation fails");
    }

    #[test]
    fn unknown_column_error_before_output_with_header_only_mode() {
        // Even HeaderMode::Only must not write a header if validation fails.
        let all = &["a", "b"];
        let rows: Vec<Vec<String>> = vec![];
        let mut buf: Vec<u8> = Vec::new();
        let result = render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["a", "unknown"]),
                None,
                QuoteMode::Csv,
                HeaderMode::Only,
            ),
        );
        assert!(result.is_err());
        assert_eq!(buf, b"", "no header must be emitted before error");
    }

    // ── render_table: column-set precedence ──────────────────────────────────

    #[test]
    fn projected_overrides_default_columns() {
        // default_columns = Some(&["a"]), but projected asks for "b" — projected wins.
        let all = &["a", "b"];
        let rows = vec![vec!["val_a".to_string(), "val_b".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                Some(vec!["b"]),
                Some(&["a"]),
                QuoteMode::Tsv,
                HeaderMode::On,
            ),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "b\nval_b\n");
    }

    #[test]
    fn default_columns_used_when_no_projection() {
        // default_columns = Some(&["a"]) and no projection → only "a" column.
        let all = &["a", "b"];
        let rows = vec![vec!["val_a".to_string(), "val_b".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(
                all,
                &rows,
                None,
                Some(&["a"]),
                QuoteMode::Tsv,
                HeaderMode::On,
            ),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\nval_a\n");
    }

    #[test]
    fn default_columns_none_means_all_columns() {
        // default_columns = None → falls through to all_columns.
        let all = &["a", "b"];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, None, QuoteMode::Tsv, HeaderMode::On),
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "a\tb\n1\t2\n");
    }

    #[test]
    fn default_columns_some_empty_means_zero_columns() {
        // default_columns = Some(&[]) → zero effective columns → empty data lines.
        // HeaderMode::Off so we test the row output path.
        let all = &["a", "b"];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, Some(&[]), QuoteMode::Tsv, HeaderMode::Off),
        )
        .unwrap();
        // Zero columns → each row emits an empty joined string + newline.
        assert_eq!(String::from_utf8(buf).unwrap(), "\n");
    }

    #[test]
    fn default_columns_some_empty_with_header_on() {
        // default_columns = Some(&[]) with HeaderMode::On: the engine emits an
        // empty header line (join of zero columns = "") followed by one empty data
        // line per row (same writeln behaviour as HeaderMode::Off for data rows).
        // This pins the natural engine output; no engine behaviour is changed here.
        let all = &["a", "b"];
        let rows = vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ];
        let mut buf: Vec<u8> = Vec::new();
        render_table(
            &mut buf,
            &make_spec(all, &rows, None, Some(&[]), QuoteMode::Csv, HeaderMode::On),
        )
        .unwrap();
        // Empty header line + two empty data lines (one per row).
        assert_eq!(String::from_utf8(buf).unwrap(), "\n\n\n");
    }
}
