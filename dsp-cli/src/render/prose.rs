//! Prose renderer — rich human-readable output.
//!
//! This is the default renderer (per dsp-cli/ADR-0003). `diagnostic` and the three
//! auth methods produce human-readable one-liners. Per-noun methods arrive
//! with real data: `project_dump` in Phase 3, `projects` (list) in Phase 4;
//! the remaining noun-groups (data-models, etc.) follow in Phase 5. Prose
//! output is irreducibly per-noun — each method is bespoke.

use std::io::{self, Write};

use chrono::{DateTime, Utc};

use crate::diagnostic::Diagnostic;
use crate::model::{DataModelDetail, DataModelStructure, ProjectDetail, VocabularyDetail};
use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
use crate::render::dump::{DumpDeleteOutcome, DumpOutcome};
use crate::render::table::render_prose_footer;
use crate::render::value::render_value_content;
use crate::render::vocabulary::{flatten_vocabulary_detail, join_labels_prose};
use crate::render::{
    DataModelListView, MetaContext, ProjectListView, Renderer, ResourceListPagination, ResourceListView,
    ResourceTypeListView, VocabularyListView,
};
use crate::util::text::{html_to_text, strip_control_chars};

/// Format the expiry clause for a token's prose line.
///
/// Returns one of:
/// - `" Token expires <ts>."` — expiry is known and in the future.
/// - `" Token expired <ts>."` — expiry is known and in the past.
/// - `""` — no expiry information (caller controls how to treat the absent case).
///
/// Note: this helper returns an empty string for `None`; callers that need
/// "expiry unknown" (e.g. the `AuthenticatedViaEnv` arm) must handle `None`
/// themselves.
fn format_expiry_clause(expires_at: Option<DateTime<Utc>>, expired: bool) -> String {
    match expires_at {
        Some(exp) if expired => {
            format!(" Token expired {}.", exp.format("%Y-%m-%d %H:%M UTC"))
        }
        Some(exp) => {
            format!(" Token expires {}.", exp.format("%Y-%m-%d %H:%M UTC"))
        }
        None => String::new(),
    }
}

/// Renders output as rich human-readable prose.
pub struct ProseRenderer {
    out: Box<dyn Write>,
}

impl ProseRenderer {
    /// Creates a renderer writing to stdout.
    ///
    /// Stdout is wrapped in `BrokenPipeWriter` so `dsp ... | head` exits 0
    /// silently instead of surfacing a broken pipe as `Diagnostic::Internal`.
    pub fn new() -> Self {
        Self {
            out: Box::new(crate::util::BrokenPipeWriter::new(io::stdout())),
        }
    }

    /// Creates a renderer writing to an arbitrary `Write` sink (used in tests).
    pub fn with_writer(w: impl Write + 'static) -> Self {
        Self { out: Box::new(w) }
    }
}

impl Default for ProseRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for ProseRenderer {
    fn diagnostic(&mut self, diag: &Diagnostic, _meta: &MetaContext) -> Result<(), Diagnostic> {
        eprintln!("Error: {diag}"); // errors go to stderr for non-JSON formats (dsp-cli/ADR-0012)
        Ok(())
    }

    fn auth_login(&mut self, outcome: &AuthLoginOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        match &outcome.expires_at {
            Some(exp) => writeln!(
                self.out,
                "Logged in to {} as {}. Token expires {}.",
                outcome.server,
                outcome.user,
                exp.format("%Y-%m-%d %H:%M UTC"),
            )?,
            None => writeln!(self.out, "Logged in to {} as {}.", outcome.server, outcome.user,)?,
        }
        Ok(())
    }

    fn auth_status(&mut self, outcome: &AuthStatusOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        match outcome {
            AuthStatusOutcome::LoggedIn { server, user, expires_at, expired } => {
                let user_str = user.as_deref().map(|u| format!(" as {u}")).unwrap_or_default();
                let expiry_str = format_expiry_clause(*expires_at, *expired);
                writeln!(self.out, "Logged in to {server}{user_str}.{expiry_str}")?;
            }
            AuthStatusOutcome::AuthenticatedViaEnv { server, expires_at, expired } => {
                let expiry_str = match expires_at {
                    None => " Token expiry unknown.".to_string(),
                    Some(_) => format_expiry_clause(*expires_at, *expired),
                };
                writeln!(self.out, "Authenticated to {server} via DSP_TOKEN.{expiry_str}")?;
            }
            AuthStatusOutcome::NotLoggedIn { server } => {
                writeln!(self.out, "Not logged in to {server}.")?;
            }
        }
        Ok(())
    }

    fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        if outcome.was_cached {
            writeln!(self.out, "Logged out of {}.", outcome.server)?;
        } else {
            writeln!(self.out, "Not logged in to {} (nothing to remove).", outcome.server)?;
        }
        Ok(())
    }

    fn auth_set_token(&mut self, outcome: &AuthSetTokenOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        let user_clause = outcome.user.as_deref().map(|u| format!(" as {u}")).unwrap_or_default();
        let expiry_clause = match outcome.expires_at {
            Some(exp) => format!(" Token expires {}.", exp.format("%Y-%m-%d %H:%M UTC")),
            None => String::new(),
        };
        writeln!(
            self.out,
            "Cached token for {server}{user_clause}.{expiry_clause}",
            server = outcome.server,
        )?;
        Ok(())
    }

    fn project_dump(&mut self, outcome: &DumpOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        if outcome.reused {
            // Adopt path: announce the existing dump with its creation timestamp
            // if known, then the local file line.
            match outcome.created_at {
                Some(ts) => writeln!(
                    self.out,
                    "Downloaded existing dump (created {}).",
                    ts.format("%Y-%m-%d %H:%M UTC"),
                )?,
                None => writeln!(self.out, "Downloaded existing dump.")?,
            }
        }
        writeln!(self.out, "Wrote {} ({} bytes).", outcome.path.display(), outcome.bytes,)?;
        if outcome.cleaned_up {
            writeln!(self.out, "Cleaned up the server-side dump.")?;
        }
        Ok(())
    }

    fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, _meta: &MetaContext) -> Result<(), Diagnostic> {
        if outcome.deleted {
            writeln!(self.out, "Removed the project's dump.")?;
        } else if let Some(ref note) = outcome.note {
            writeln!(self.out, "{note}")?;
        }
        Ok(())
    }

    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Count line: with or without filter.
        let n = view.items.len();
        match &view.filter {
            None => writeln!(self.out, "Projects on {} ({n}):", meta.server_label)?,
            Some(f) => writeln!(
                self.out,
                "Projects on {} ({n} of {} matching \"{f}\"):",
                meta.server_label, view.total
            )?,
        }

        writeln!(self.out)?;

        // Compute column widths for alignment.
        let sc_w = view.items.iter().map(|p| p.shortcode.len()).max().unwrap_or(0);
        let sn_w = view.items.iter().map(|p| p.shortname.len()).max().unwrap_or(0);
        let ln_w = view
            .items
            .iter()
            .map(|p| p.longname.as_deref().unwrap_or("").len())
            .max()
            .unwrap_or(0);

        for item in &view.items {
            let longname = item.longname.as_deref().unwrap_or("");
            // iri is intentionally omitted from prose (dsp-cli/ADR-0003 / plan Step 3c).
            // The data-models hint is right-appended to the row, per the locked
            // PRD output format. The row is assembled first and trimmed at the
            // end so a project with no longname and no data-models does not
            // leave the column padding behind as trailing whitespace.
            let mut row = format!("  {:<sc_w$}  {:<sn_w$}  {:<ln_w$}", item.shortcode, item.shortname, longname,);
            if item.data_models > 0 {
                let label = if item.data_models == 1 {
                    "data-model"
                } else {
                    "data-models"
                };
                row.push_str(&format!("   \u{b7} {} {label}", item.data_models));
            }
            writeln!(self.out, "{}", row.trim_end())?;
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Header: "Project: <shortname> (<shortcode>)" — always this shape.
        // Short identifiers form the title; the longname becomes a labeled field below.
        writeln!(self.out, "Project: {} ({})", project.shortname, project.shortcode)?;

        // Label/value block — 2-space indent, values aligned to a common column.
        // "  Keywords:   " is the longest label (13 chars incl. colon + spaces).

        // Name (longname) — omit when None.
        if let Some(ln) = &project.longname {
            writeln!(self.out, "  Name:       {ln}")?;
        }

        writeln!(self.out, "  IRI:        {}", project.iri)?;

        // Keywords — omit line when empty.
        if !project.keywords.is_empty() {
            writeln!(self.out, "  Keywords:   {}", project.keywords.join(", "))?;
        }

        // Data-models — always present; no trailing colon/names when n==0.
        let n = project.data_models.len();
        if n == 0 {
            writeln!(self.out, "  Data-models (0)")?;
        } else {
            let names: Vec<&str> = project.data_models.iter().map(|dm| dm.name.as_str()).collect();
            writeln!(self.out, "  Data-models ({n}): {}", names.join(", "))?;
        }

        // Description — each entry is run through html_to_text before rendering
        // (prose-only; JSON keeps the raw value per dsp-cli/ADR-0003). Entries whose
        // value reduces to empty text (e.g. only tags) are dropped, and the whole
        // block — including the `Description:` label — is omitted when nothing
        // visible remains, so the label never dangles with no content under it.
        let descriptions: Vec<(Option<&str>, String)> = project
            .description
            .iter()
            .map(|e| (e.language.as_deref(), html_to_text(&e.value)))
            .filter(|(_, plain)| !plain.is_empty())
            .collect();
        if !descriptions.is_empty() {
            writeln!(self.out, "  Description:")?;
            for (language, plain) in &descriptions {
                // Render each line of the plain-text value with a 4-space base indent.
                // Language tag `[<lang>]` prefixes the first line; continuation lines
                // are indented to align with the first-line text.
                let prefix = match language {
                    Some(lang) => format!("    [{lang}] "),
                    None => "    ".to_string(),
                };
                let continuation_indent = " ".repeat(prefix.len());
                let mut first_line = true;
                for line in plain.lines() {
                    if first_line {
                        writeln!(self.out, "{prefix}{line}")?;
                        first_line = false;
                    } else if line.is_empty() {
                        // Blank separator line between paragraphs: emit it truly
                        // empty, not as a whitespace-only (indented) line.
                        writeln!(self.out)?;
                    } else {
                        writeln!(self.out, "{continuation_indent}{line}")?;
                    }
                }
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Header: "Data-model: {name}"
        writeln!(self.out, "Data-model: {}", detail.name)?;

        // Label/value block — 2-space indent, values aligned to a common column.
        // Longest label key is "Last-modified:" (14 chars incl. colon + 2 trailing spaces = 16
        // wide). Label: omit when None; IRI: always; Last-modified: omit when None.
        if let Some(ref lbl) = detail.label {
            writeln!(self.out, "  Label:          {lbl}")?;
        }
        writeln!(self.out, "  IRI:            {}", detail.iri)?;
        if let Some(ref lm) = detail.last_modified {
            let date = lm.split_once('T').map(|(d, _)| d).unwrap_or(lm.as_str());
            writeln!(self.out, "  Last-modified:  {date}")?;
        }

        // Resource-types summary block.
        let n = detail.resource_types.len();
        if n == 0 {
            writeln!(self.out, "  Resource-types (0)")?;
        } else {
            writeln!(self.out)?;
            writeln!(self.out, "  Resource-types ({n}):")?;
            // Compute max name width for alignment.
            let name_w = detail.resource_types.iter().map(|rt| rt.name.len()).max().unwrap_or(0);
            for rt in &detail.resource_types {
                let label = rt.label.as_deref().unwrap_or("");
                writeln!(self.out, "    {:<name_w$}  {label}", rt.name)?;
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        let n = view.items.len();
        let has_builtins = view.items.iter().any(|d| d.is_builtin);
        let builtin_suffix = if has_builtins { ", incl. built-ins" } else { "" };

        // Header line: with or without filter.
        match &view.filter {
            None => writeln!(self.out, "data-models on {} ({n}){builtin_suffix}:", meta.server_label,)?,
            Some(f) => writeln!(
                self.out,
                "data-models on {} ({n} of {} matching \"{f}\"){builtin_suffix}:",
                meta.server_label, view.total,
            )?,
        }

        writeln!(self.out)?;

        // Compute column widths for alignment.
        let name_w = view.items.iter().map(|d| d.name.len()).max().unwrap_or(0);
        let label_w = view
            .items
            .iter()
            .map(|d| {
                if let Some(ref lbl) = d.label {
                    lbl.len()
                } else if d.is_builtin {
                    "(built-in)".len()
                } else {
                    0
                }
            })
            .max()
            .unwrap_or(0);

        for item in &view.items {
            // label_or_marker: label, else "(built-in)" for builtins, else "".
            let label_or_marker: &str = if let Some(ref lbl) = item.label {
                lbl.as_str()
            } else if item.is_builtin {
                "(built-in)"
            } else {
                ""
            };

            // date: YYYY-MM-DD prefix via split_once('T'), empty when None.
            let date_str: String = match &item.last_modified {
                Some(s) => s.split_once('T').map(|(d, _)| d.to_string()).unwrap_or_else(|| s.clone()),
                None => String::new(),
            };

            writeln!(
                self.out,
                "  {:<name_w$}  {:<label_w$}  {}",
                item.name, label_or_marker, date_str,
            )?;
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        let n = view.items.len();
        let has_builtins = view.items.iter().any(|rt| rt.is_builtin);
        let has_counts = view.items.iter().any(|rt| rt.count.is_some());
        let builtin_suffix = if has_builtins { ", incl. built-ins" } else { "" };

        // Header line: with or without filter.
        match &view.filter {
            None => writeln!(
                self.out,
                "resource-types in {} on {} ({n}){builtin_suffix}:",
                view.data_model, meta.server_label,
            )?,
            Some(f) => writeln!(
                self.out,
                "resource-types in {} on {} ({n} of {} matching \"{f}\"){builtin_suffix}:",
                view.data_model, meta.server_label, view.total,
            )?,
        }

        writeln!(self.out)?;

        // Compute column widths for alignment.
        let name_w = view.items.iter().map(|rt| rt.name.len()).max().unwrap_or(0);
        let label_w = view
            .items
            .iter()
            .map(|rt| rt.label.as_deref().map(|l| l.len()).unwrap_or(0))
            .max()
            .unwrap_or(0);

        if has_counts {
            // Right-aligned COUNT column, shown only when at least one item
            // carries a count (plan 030). label is now padded in both branches
            // so the count column aligns whether or not a row is built-in.
            let count_w = view
                .items
                .iter()
                .map(|rt| rt.count.map(|c| c.to_string()).unwrap_or_default().len())
                .max()
                .unwrap_or(0);
            for item in &view.items {
                let label = item.label.as_deref().unwrap_or("");
                let count_str = item.count.map(|c| c.to_string()).unwrap_or_default();
                if item.is_builtin {
                    writeln!(
                        self.out,
                        "  {:<name_w$}  {:<label_w$}  {:>count_w$}  (built-in)",
                        item.name, label, count_str,
                    )?;
                } else {
                    writeln!(
                        self.out,
                        "  {:<name_w$}  {:<label_w$}  {:>count_w$}",
                        item.name, label, count_str,
                    )?;
                }
            }
        } else {
            for item in &view.items {
                let label = item.label.as_deref().unwrap_or("");
                // Built-in rows get an explicit trailing marker; project rows get nothing.
                if item.is_builtin {
                    writeln!(self.out, "  {:<name_w$}  {:<label_w$}  (built-in)", item.name, label,)?;
                } else {
                    writeln!(self.out, "  {:<name_w$}  {label}", item.name)?;
                }
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn resource_type_describe(
        &mut self,
        detail: &crate::model::ResourceTypeDetail,
        meta: &MetaContext,
    ) -> Result<(), Diagnostic> {
        use crate::model::ValueType;

        // Header: "Resource-type: <name>"
        writeln!(self.out, "Resource-type: {}", detail.name)?;

        // Optional label/extends/representation header block — 2-space indent,
        // values aligned to a common column (longest label key is "Representation:"
        // at 15 chars + 2-space prefix + 2-space gap = col 19).
        if let Some(ref lbl) = detail.label {
            writeln!(self.out, "  Label:          {lbl}")?;
        }
        if !detail.super_types.is_empty() {
            writeln!(self.out, "  Extends:        {}", detail.super_types.join(", "))?;
        }
        if let Some(ref repr) = detail.representation {
            writeln!(self.out, "  Representation: {repr}")?;
        }
        writeln!(self.out, "  IRI:            {}", detail.iri)?;
        writeln!(self.out, "  Data-model:     {}", detail.data_model)?;
        if let Some(count) = detail.count {
            writeln!(self.out, "  Instances:      {count}")?;
        }

        // Fields block.
        let n = detail.fields.len();
        if n == 0 {
            writeln!(self.out, "\n  Fields (0)")?;
        } else {
            writeln!(self.out, "\n  Fields ({n}):")?;

            // Build rendered value-type strings (link → Target or plain value-type).
            let vtype_strs: Vec<String> = detail
                .fields
                .iter()
                .map(|f| match &f.value_type {
                    ValueType::Link => {
                        if let Some(ref tgt) = f.link_target {
                            format!("link \u{2192} {tgt}")
                        } else {
                            "link".to_string()
                        }
                    }
                    vt => vt.to_string(),
                })
                .collect();

            // Compute column widths.
            let name_w = detail.fields.iter().map(|f| f.name.len()).max().unwrap_or(0);
            let vtype_w = vtype_strs.iter().map(|s| s.len()).max().unwrap_or(0);
            let card_w = detail.fields.iter().map(|f| f.cardinality.to_string().len()).max().unwrap_or(0);

            for (field, vtype_str) in detail.fields.iter().zip(vtype_strs.iter()) {
                let label = field.label.as_deref().unwrap_or("");
                let card_str = field.cardinality.to_string();

                // Source tag: append when field's data_model is Some and differs from
                // the resource-type's own data_model.
                let source_tag = if let Some(ref dm) = field.data_model {
                    if dm != &detail.data_model {
                        format!("  [from {dm}]")
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Built-in marker: append after label when is_builtin.
                let builtin_marker = if field.is_builtin { "  (built-in)" } else { "" };

                writeln!(
                    self.out,
                    "    {:<name_w$}  {:<vtype_w$}  {:<card_w$}  {label}{builtin_marker}{source_tag}",
                    field.name, vtype_str, card_str,
                )?;
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn data_model_structure(&mut self, structure: &DataModelStructure, meta: &MetaContext) -> Result<(), Diagnostic> {
        use crate::model::RelationKind;

        // Header: "Structure: <dm>  (<n> relations)"
        let n = structure.relations.len();
        let plural = if n == 1 { "relation" } else { "relations" };
        writeln!(self.out, "Structure: {}  ({n} {plural})", structure.data_model)?;

        if n > 0 {
            writeln!(self.out)?;

            // Compute column widths.
            // source_w: max source name length.
            let source_w = structure.relations.iter().map(|r| r.source.len()).max().unwrap_or(0);

            // target_w: max of (target + optional " [to <dm>]") length.
            let target_w = structure
                .relations
                .iter()
                .map(|r| {
                    let tag_len = match &r.target_data_model {
                        Some(dm) if dm != &structure.data_model => " [to ".len() + dm.len() + "]".len(),
                        _ => 0,
                    };
                    r.target.len() + tag_len
                })
                .max()
                .unwrap_or(0);

            // field_w: max field name length (empty string for inherits).
            let field_w = structure
                .relations
                .iter()
                .map(|r| r.field.as_deref().unwrap_or("").len())
                .max()
                .unwrap_or(0);

            for rel in &structure.relations {
                // Build the target column: target + optional "[to <dm>]" tag.
                let target_col = match &rel.target_data_model {
                    Some(dm) if dm != &structure.data_model => {
                        format!("{} [to {}]", rel.target, dm)
                    }
                    _ => rel.target.clone(),
                };

                // field column: field name for link, empty for inherits.
                let field_col = rel.field.as_deref().unwrap_or("");

                // kind marker.
                let kind_marker = match rel.kind {
                    RelationKind::Link => "[link]",
                    RelationKind::Inherits => "[inherits]",
                };

                writeln!(
                    self.out,
                    "  {:<source_w$}   \u{2192} {:<target_w$}   {:<field_w$}   {kind_marker}",
                    rel.source, target_col, field_col,
                )?;
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        let n = view.items.len();

        // Header line: with or without filter.
        match &view.filter {
            None => writeln!(
                self.out,
                "resources of type {} on {} ({n}):",
                view.resource_type, meta.server_label,
            )?,
            Some(f) => writeln!(
                self.out,
                "resources of type {} on {} ({n} of {} matching \"{f}\"):",
                view.resource_type, meta.server_label, view.total,
            )?,
        }

        if n > 0 {
            writeln!(self.out)?;

            // Sanitise labels before computing the alignment width, so padding
            // stays correct when a label contains control characters (stripping
            // shortens the byte length the `{:<label_w$}` pad relies on).
            let labels: Vec<String> = view.items.iter().map(|r| strip_control_chars(&r.label)).collect();
            let label_w = labels.iter().map(|l| l.len()).max().unwrap_or(0);

            for (item, label) in view.items.iter().zip(&labels) {
                writeln!(self.out, "  {:<label_w$}  {}", label, item.iri,)?;
            }
        }

        // "more results available" hint — only in SinglePage mode when may_have_more.
        if let ResourceListPagination::SinglePage { may_have_more: true, .. } = view.pagination {
            writeln!(self.out)?;
            writeln!(self.out, "  more results available (use --all to fetch all pages)")?;
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn resource_describe(
        &mut self,
        detail: &crate::model::ResourceDetail,
        meta: &MetaContext,
    ) -> Result<(), Diagnostic> {
        // Header: "Resource: <label>"
        writeln!(self.out, "Resource: {}", strip_control_chars(&detail.label))?;

        // Label/value block — 2-space indent, values aligned to a common column.
        // Longest label key is "Your access:" (12 chars) + 2-space prefix + gap.
        // Column alignment: pad to 14 chars for the label (incl. colon).
        // "Your access:  " = 12 + 2 = 14 wide.
        writeln!(self.out, "  Type:         {}", detail.resource_type)?;
        writeln!(self.out, "  IRI:          {}", detail.iri)?;
        if let Some(ref ark) = detail.ark_url {
            writeln!(self.out, "  ARK:          {ark}")?;
        }
        if let Some(ref created) = detail.creation_date {
            writeln!(self.out, "  Created:      {created}")?;
        }
        if let Some(ref modified) = detail.last_modified {
            writeln!(self.out, "  Modified:     {modified}")?;
        }
        if let Some(ref project) = detail.attached_project {
            writeln!(self.out, "  Project:      {project}")?;
        }
        if let Some(ref owner) = detail.owner {
            writeln!(self.out, "  Owner:        {owner}")?;
        }
        if let Some(ref vis) = detail.visibility {
            writeln!(self.out, "  Visibility:   {}", vis.as_str())?;
        }
        if let Some(ref access) = detail.your_access {
            writeln!(self.out, "  Your access:  {}", access.as_str())?;
        }

        // Values section — only when `--values` was set (detail.values is Some).
        if let Some(ref fields) = detail.values {
            writeln!(self.out)?;
            if fields.is_empty() {
                writeln!(self.out, "Values: (none)")?;
            } else {
                writeln!(self.out, "Values:")?;
                for fv in fields {
                    // Field header: "<label> (<name>)" when label is Some, else "<name>".
                    let header = match &fv.label {
                        Some(lbl) => {
                            format!("{} ({})", strip_control_chars(lbl), strip_control_chars(&fv.name))
                        }
                        None => strip_control_chars(&fv.name),
                    };
                    writeln!(self.out, "  {header}")?;

                    for value in &fv.values {
                        let rendered = render_value_content(&value.content);
                        writeln!(self.out, "    {}", strip_control_chars(&rendered))?;
                        if let Some(comment) = &value.comment {
                            writeln!(self.out, "      comment: {}", strip_control_chars(comment))?;
                        }
                    }
                }
            }
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn vocabularies(&mut self, view: &VocabularyListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Header line: with or without filter (mirrors `projects`/`resource_types`).
        let n = view.items.len();
        match &view.filter {
            None => writeln!(self.out, "Vocabularies on {} ({n}):", meta.server_label)?,
            Some(f) => writeln!(
                self.out,
                "Vocabularies on {} ({n} of {} matching \"{f}\"):",
                meta.server_label, view.total
            )?,
        }

        writeln!(self.out)?;

        for item in &view.items {
            let name = strip_control_chars(item.header.name.as_deref().unwrap_or(""));
            let labels = strip_control_chars(&join_labels_prose(&item.header.labels));
            write!(self.out, "  {name}  {labels}")?;
            // `--count` suffix: only when BOTH counted AND this item's per-tree
            // fetch actually succeeded. A failed fetch (`None`) renders no
            // suffix at all — the top-level `meta.count_cost` disclosure
            // already covers the caveat; a per-row placeholder would be a
            // second, redundant disclosure mechanism.
            if view.counted
                && let (Some(nodes), Some(depth)) = (item.node_count, item.depth)
            {
                let node_word = if nodes == 1 { "node" } else { "nodes" };
                let level_word = if depth == 1 { "level" } else { "levels" };
                write!(self.out, "  \u{b7} {nodes} {node_word} \u{b7} {depth} {level_word}")?;
            }
            writeln!(self.out)?;
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }

    fn vocabulary_describe(&mut self, detail: &VocabularyDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        let root = &detail.tree.root;
        let root_name = strip_control_chars(root.name.as_deref().unwrap_or(""));
        let root_labels = strip_control_chars(&join_labels_prose(&root.labels));
        writeln!(self.out, "Vocabulary: {root_name} \u{2014} {root_labels}")?;
        writeln!(self.out, "Root: {}", root.iri)?;

        // Reuse the SAME flattened+filtered row set for both the header
        // line's numbers (subtree-of / requested-node) and the body below —
        // avoids a second flattening pass (plan 034 Step 3).
        let rows = flatten_vocabulary_detail(detail);

        let node_word = if detail.node_count == 1 { "node" } else { "nodes" };
        let level_word = if detail.depth == 1 { "level" } else { "levels" };
        write!(
            self.out,
            "{} {node_word} \u{b7} {} {level_word}",
            detail.node_count, detail.depth
        )?;
        // The `--subtree` header note SUBSUMES the "you asked about" note —
        // never show both (the subtree header already names the node).
        if let Some(target_iri) = &detail.subtree_of
            && let Some(top) = rows.iter().find(|n| &n.header.iri == target_iri)
        {
            write!(self.out, " \u{b7} subtree of {}", top.number)?;
        } else if let Some(requested) = &detail.tree.requested_node
            && let Some(row) = rows.iter().find(|n| &n.header.iri == requested)
        {
            write!(self.out, " \u{2190} you asked about {}", row.number)?;
        }
        writeln!(self.out)?;
        writeln!(self.out)?;

        // Prose indents by depth WITHIN THE RENDERED SET, not by absolute
        // depth: under `--subtree` the branch starts at the left margin even
        // though `number` still reads e.g. `1.1.2.1`.
        let base_depth = rows.first().map(|n| n.depth).unwrap_or(1);
        for node in &rows {
            let indent = "  ".repeat(node.depth.saturating_sub(base_depth) + 1);
            let labels = strip_control_chars(&join_labels_prose(&node.header.labels));
            let marker = if detail.tree.requested_node.as_deref() == Some(node.header.iri.as_str()) {
                "   \u{2190}"
            } else {
                ""
            };
            writeln!(self.out, "{indent}{}  {labels}{marker}", node.number)?;
        }

        render_prose_footer(&mut *self.out, meta)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Cardinality, DataModel, DataModelDetail, DataModelSummary, Field, Project, ProjectDescription, ProjectDetail,
        Representation, ResourceType, ResourceTypeDetail, ResourceTypeSummary, ValueType,
    };
    use crate::render::test_support::{SharedBuf, make_meta};
    use crate::render::{DataModelListView, ResourceTypeListView};

    fn make_fixture() -> Vec<Project> {
        vec![
            Project {
                iri: "http://rdfh.ch/projects/0001".into(),
                shortcode: "0001".into(),
                shortname: "anything".into(),
                longname: Some("Anything Project".into()),
                data_models: 2,
            },
            Project {
                iri: "http://rdfh.ch/projects/0002".into(),
                shortcode: "0002".into(),
                shortname: "images".into(),
                longname: None,
                data_models: 0,
            },
            Project {
                iri: "http://rdfh.ch/projects/0803".into(),
                shortcode: "0803".into(),
                shortname: "daschland".into(),
                longname: Some("=Formula Project".into()),
                data_models: 1,
            },
        ]
    }

    #[test]
    fn projects_prose_no_filter() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = ProjectListView { items: make_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("Projects on https://api.test.dasch.swiss (3):"));
        // Footer on stdout (not stderr)
        assert!(s.contains("[anonymous on https://api.test.dasch.swiss]"));
        // IRI is omitted
        assert!(!s.contains("rdfh.ch"));
        // data_models line for active/2 project
        assert!(s.contains("2 data-models"));
        // singular for 1 data-model
        assert!(s.contains("1 data-model"));
        // no data-models line for 0
        let data_model_lines = s.lines().filter(|l| l.contains("data-model")).count();
        assert_eq!(data_model_lines, 2);
        // None longname → empty (no "None" printed)
        assert!(!s.contains("None"));
    }

    #[test]
    fn projects_prose_with_filter() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let items = vec![make_fixture().remove(0)]; // just the "anything" project
        let view = ProjectListView { items, total: 3, filter: Some("any".to_string()) };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("(1 of 3 matching \"any\")"));
    }

    #[test]
    fn projects_prose_footer_on_stdout() {
        // Prose writes the disclosure footer to stdout (not stderr).
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = ProjectListView { items: vec![], total: 0, filter: None };
        let meta = make_meta("authenticated as alice", "https://api.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("[authenticated as alice on https://api.dasch.swiss]"));
    }

    fn make_beol_detail() -> ProjectDetail {
        ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            description: vec![ProjectDescription {
                value: "<b>BEOL</b> — early modern mathematics.".into(),
                language: Some("en".into()),
            }],
            keywords: vec!["Bernoulli".into(), "Euler".into(), "Mathematics".into()],
            data_models: vec![
                DataModelSummary {
                    name: "beol".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                },
                DataModelSummary {
                    name: "biblio".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                },
                DataModelSummary {
                    name: "leibniz".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/leibniz/v2".into(),
                },
                DataModelSummary {
                    name: "newton".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/newton/v2".into(),
                },
            ],
        }
    }

    #[test]
    fn project_describe_prose_full() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&make_beol_detail(), &meta).unwrap();

        let s = out.string();
        // Header: shortname (shortcode) — new layout B
        assert!(s.contains("Project: beol (0801)"));
        // Longname is now a labeled field, not in the title
        assert!(s.contains("Name:       Bernoulli-Euler Online"));
        // Other label/value lines
        assert!(s.contains("IRI:        http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF"));
        assert!(s.contains("Keywords:   Bernoulli, Euler, Mathematics"));
        // Data-models with count + names
        assert!(s.contains("Data-models (4): beol, biblio, leibniz, newton"));
        // Description: plain text — no raw HTML tags, language prefix present
        assert!(s.contains("[en] BEOL"));
        assert!(!s.contains("<b>"), "description must not contain raw <b> tags");
        assert!(!s.contains("</b>"), "description must not contain raw </b> tags");
        // dsp-cli/ADR-0007 footer
        assert!(s.contains("[anonymous on api.dasch.swiss]"));
    }

    #[test]
    fn project_describe_prose_no_longname() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_beol_detail();
        detail.longname = None;
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Header is shortname (shortcode); no Name: line
        assert!(s.contains("Project: beol (0801)"));
        assert!(!s.contains("Name:"), "Name: line must be absent when longname is None");
    }

    #[test]
    fn project_describe_prose_empty_keywords_and_description() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0000".into(),
            shortcode: "0000".into(),
            shortname: "minimal".into(),
            longname: None,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Keywords line omitted
        assert!(!s.contains("Keywords"));
        // Description block omitted
        assert!(!s.contains("Description"));
        // Data-models (0) with no names
        assert!(s.contains("Data-models (0)"));
        assert!(!s.contains("Data-models (0):"));
    }

    #[test]
    fn project_describe_prose_description_no_language() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0001".into(),
            shortcode: "0001".into(),
            shortname: "test".into(),
            longname: None,
            description: vec![ProjectDescription {
                value: "Plain description without language tag.".into(),
                language: None,
            }],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();
        // No language prefix on description line — value rendered with 4-space indent.
        assert!(s.contains("    Plain description without language tag."));
        // The description line itself must not have a language tag prefix like "[en]".
        let desc_line = s.lines().find(|l| l.contains("Plain description")).unwrap();
        assert!(!desc_line.contains('['));
    }

    /// Two description entries: one with a language tag, one without.
    /// Plan: "Multiple description entries: render each, prefixing with `[<lang>]`
    /// only when `language` is set."
    #[test]
    fn project_describe_prose_two_descriptions_language_and_none() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0002".into(),
            shortcode: "0002".into(),
            shortname: "multilang".into(),
            longname: None,
            description: vec![
                ProjectDescription {
                    value: "English description of this project.".into(),
                    language: Some("en".into()),
                },
                ProjectDescription {
                    value: "Description without a language tag.".into(),
                    language: None,
                },
            ],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();

        // Both entries must appear in the rendered output.
        assert!(
            s.contains("English description of this project."),
            "first description value must appear"
        );
        assert!(
            s.contains("Description without a language tag."),
            "second description value must appear"
        );

        // The languaged entry's value line must be prefixed with "[en] ".
        let en_line = s
            .lines()
            .find(|l| l.contains("English description"))
            .expect("expected a line containing 'English description'");
        assert!(
            en_line.contains("[en] "),
            "languaged entry must carry '[en] ' prefix; got: {en_line:?}"
        );

        // The no-language entry's value line must NOT be prefixed with any "[...]".
        let plain_line = s
            .lines()
            .find(|l| l.contains("Description without a language tag."))
            .expect("expected a line containing 'Description without a language tag.'");
        assert!(
            !plain_line.contains('['),
            "no-language entry must not have a '[...]' prefix; got: {plain_line:?}"
        );
    }

    /// A multi-paragraph description: blank separator lines between paragraphs
    /// must be rendered TRULY empty, not as whitespace-only (indented) lines.
    #[test]
    fn project_describe_prose_blank_lines_are_not_indented() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0003".into(),
            shortcode: "0003".into(),
            shortname: "multipara".into(),
            longname: None,
            description: vec![ProjectDescription {
                value: "First paragraph.\n\nSecond paragraph.".into(),
                language: Some("en".into()),
            }],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Both paragraphs render (the continuation line is indented to align).
        assert!(s.contains("[en] First paragraph."));
        assert!(s.contains("Second paragraph."));
        // No line may be whitespace-only (non-empty but all-whitespace): the blank
        // separator between paragraphs must be a truly empty line.
        let ws_only = s.lines().find(|l| !l.is_empty() && l.trim().is_empty());
        assert!(
            ws_only.is_none(),
            "blank separator lines must be truly empty, not whitespace-only; got:\n{s}"
        );
    }

    /// A description entry whose value reduces to empty text after html_to_text
    /// (e.g. only tags) must NOT leave a dangling `Description:` label.
    #[test]
    fn project_describe_prose_empty_after_html_omits_description_label() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0004".into(),
            shortcode: "0004".into(),
            shortname: "tagsonly".into(),
            longname: None,
            description: vec![ProjectDescription {
                value: "<br/>".into(), // → "" after html_to_text
                language: Some("en".into()),
            }],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(
            !s.contains("Description:"),
            "Description: label must be omitted when no entry has visible text; got:\n{s}"
        );
    }

    fn make_data_model_fixture() -> Vec<DataModel> {
        vec![
            DataModel {
                name: "beol".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                label: Some("The BEOL data-model".into()),
                last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
                is_builtin: false,
            },
            DataModel {
                name: "biblio".into(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                label: None,
                last_modified: Some("2024-01-10T08:00:00.000000Z".into()),
                is_builtin: false,
            },
            DataModel {
                name: "knora-api".into(),
                iri: "http://api.knora.org/ontology/knora-api/v2".into(),
                label: None,
                last_modified: None,
                is_builtin: true,
            },
        ]
    }

    #[test]
    fn data_models_prose_no_filter() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let s = out.string();
        // Header with builtin suffix
        assert!(
            s.contains("data-models on https://api.test.dasch.swiss (3), incl. built-ins:"),
            "header must include count and built-ins suffix; got:\n{s}"
        );
        // dsp-cli/ADR-0007 footer on stdout
        assert!(s.contains("[anonymous on https://api.test.dasch.swiss]"));
        // beol row: label present + date (date portion only)
        assert!(s.contains("beol") && s.contains("The BEOL data-model") && s.contains("2024-05-27"));
        // Full RFC3339 timestamp must NOT appear in prose
        assert!(!s.contains("T13:43:26"), "prose must show only the date portion");
        // biblio row: no label, not builtin, so empty label slot
        assert!(s.contains("biblio"));
        // knora-api row: no label, is_builtin → "(built-in)" marker
        assert!(s.contains("(built-in)"));
        // "None" must never appear literally
        assert!(!s.contains("None"));
    }

    #[test]
    fn data_models_prose_with_filter() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let items = vec![make_data_model_fixture().remove(0)]; // just beol
        let view = DataModelListView { items, total: 3, filter: Some("beol".to_string()) };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("(1 of 3 matching \"beol\")"),
            "filter header must show m of total; got:\n{s}"
        );
        // No builtins in filtered view → no suffix
        assert!(!s.contains("incl. built-ins"));
    }

    /// The `data_models` method must NOT append `, incl. built-ins` to the header
    /// when the view contains no builtins. This is the complement of
    /// `data_models_prose_no_filter` (which uses a fixture with a builtin).
    #[test]
    fn data_models_prose_no_builtins_header_has_no_builtin_suffix() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = DataModelListView {
            items: vec![
                DataModel {
                    name: "beol".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                    label: Some("The BEOL data-model".into()),
                    last_modified: None,
                    is_builtin: false,
                },
                DataModel {
                    name: "biblio".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                    label: None,
                    last_modified: None,
                    is_builtin: false,
                },
            ],
            total: 2,
            filter: None,
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let s = out.string();
        // Header must end with "(2):" — without the ", incl. built-ins" suffix.
        let header_line = s.lines().next().expect("output must have at least one line");
        assert!(
            header_line.ends_with("(2):"),
            "header must end with '(2):' when there are no builtins; got: {header_line:?}"
        );
        assert!(
            !s.contains("incl. built-ins"),
            "header must not contain 'incl. built-ins' when no builtin items are present; \
             got:\n{s}"
        );
    }

    #[test]
    fn data_models_prose_last_modified_no_t_separator() {
        // last_modified without a 'T' separator — fallback to whole string, no panic.
        // The value must contain no uppercase 'T' so split_once('T') returns None.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = DataModelListView {
            items: vec![DataModel {
                name: "odd".into(),
                iri: "http://example.org/odd".into(),
                label: None,
                last_modified: Some("2024-05-27 no-separator".into()),
                is_builtin: false,
            }],
            total: 1,
            filter: None,
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let s = out.string();
        // Whole string falls through when there is no 'T' separator
        assert!(
            s.contains("2024-05-27 no-separator"),
            "expected full string when no 'T' separator; got:\n{s}"
        );
    }

    // ── data_model_describe prose tests ──────────────────────────────────────

    fn make_beol_dm_detail() -> DataModelDetail {
        DataModelDetail {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
            label: Some("The BEOL data-model".into()),
            last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
            resource_types: vec![
                ResourceTypeSummary {
                    name: "Archive".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                    label: Some("Archive".into()),
                },
                ResourceTypeSummary {
                    name: "basicLetter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#basicLetter".into(),
                    label: None,
                },
                ResourceTypeSummary {
                    name: "letter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                    label: Some("Letter".into()),
                },
            ],
        }
    }

    #[test]
    fn data_model_describe_prose_full() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let s = out.string();
        // Header
        assert!(s.contains("Data-model: beol"), "header missing; got:\n{s}");
        // Label line present
        assert!(
            s.contains("Label:          The BEOL data-model"),
            "label line missing; got:\n{s}"
        );
        // IRI line always present
        assert!(
            s.contains("IRI:            http://api.dasch.swiss/ontology/0801/beol/v2"),
            "IRI line missing; got:\n{s}"
        );
        // Last-modified: date prefix only
        assert!(
            s.contains("Last-modified:  2024-05-27"),
            "last-modified line missing; got:\n{s}"
        );
        assert!(
            !s.contains("T13:43"),
            "prose must show only date portion, not full timestamp; got:\n{s}"
        );
        // Resource-types section header
        assert!(s.contains("Resource-types (3):"), "resource-types header missing; got:\n{s}");
        // Resource-type rows
        assert!(s.contains("Archive"), "Archive row missing; got:\n{s}");
        assert!(s.contains("basicLetter"), "basicLetter row missing; got:\n{s}");
        assert!(s.contains("Letter"), "Letter label missing; got:\n{s}");
        // dsp-cli/ADR-0007 footer
        assert!(s.contains("[anonymous on api.dasch.swiss]"), "footer missing; got:\n{s}");
        // "None" must never appear
        assert!(!s.contains("None"), "None must not appear; got:\n{s}");
    }

    #[test]
    fn data_model_describe_prose_no_label_no_last_modified() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "minimal".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2".into(),
            label: None,
            last_modified: None,
            resource_types: vec![ResourceTypeSummary {
                name: "Thing".into(),
                iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#Thing".into(),
                label: None,
            }],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Label line must be absent when None
        assert!(
            !s.contains("Label:"),
            "Label: line must be absent when label is None; got:\n{s}"
        );
        // Last-modified line must be absent when None
        assert!(
            !s.contains("Last-modified:"),
            "Last-modified: line must be absent when last_modified is None; got:\n{s}"
        );
        // IRI still present
        assert!(s.contains("IRI:"), "IRI line must always be present; got:\n{s}");
        // None must not appear literally
        assert!(!s.contains("None"), "None must not appear; got:\n{s}");
    }

    #[test]
    fn data_model_describe_prose_zero_resource_types() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "empty".into(),
            iri: "http://api.dasch.swiss/ontology/9999/empty/v2".into(),
            label: None,
            last_modified: None,
            resource_types: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Zero branch: "Resource-types (0)" without a colon
        assert!(
            s.contains("  Resource-types (0)"),
            "zero resource-types branch missing; got:\n{s}"
        );
        // Must NOT have a colon after "(0)" — that would imply a sub-list follows
        let rt_line = s
            .lines()
            .find(|l| l.contains("Resource-types (0)"))
            .expect("must have resource-types line");
        assert!(
            !rt_line.contains("Resource-types (0):"),
            "zero branch must NOT have a colon; got: {rt_line:?}"
        );
    }

    #[test]
    fn data_model_describe_prose_last_modified_no_t_separator() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "odd".into(),
            iri: "http://api.dasch.swiss/ontology/0000/odd/v2".into(),
            label: None,
            last_modified: Some("2024-05-27 no-separator".into()),
            resource_types: vec![],
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Whole string falls through when there is no 'T' separator
        assert!(
            s.contains("2024-05-27 no-separator"),
            "expected full string when no 'T' separator; got:\n{s}"
        );
    }

    // ── resource_types prose tests ────────────────────────────────────────────

    fn make_resource_type_fixture() -> Vec<ResourceType> {
        vec![
            ResourceType {
                name: "Archive".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                label: Some("Archive".into()),
                is_builtin: false,
                count: None,
            },
            ResourceType {
                name: "letter".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                label: Some("Letter".into()),
                is_builtin: false,
                count: None,
            },
        ]
    }

    #[test]
    fn resource_types_prose_project_only() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: make_resource_type_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let s = out.string();
        // Header: no builtin suffix for project-only list
        assert!(
            s.contains("resource-types in beol on https://api.test.dasch.swiss (2):"),
            "header missing or incorrect; got:\n{s}"
        );
        assert!(
            !s.contains("incl. built-ins"),
            "must not have builtin suffix when no builtins; got:\n{s}"
        );
        // Rows present
        assert!(s.contains("Archive"), "Archive row missing; got:\n{s}");
        assert!(s.contains("letter"), "letter row missing; got:\n{s}");
        assert!(s.contains("Letter"), "Letter label missing; got:\n{s}");
        // No (built-in) marker
        assert!(
            !s.contains("(built-in)"),
            "must not have (built-in) marker on project-only items; got:\n{s}"
        );
        // dsp-cli/ADR-0007 footer on stdout
        assert!(
            s.contains("[anonymous on https://api.test.dasch.swiss]"),
            "footer missing; got:\n{s}"
        );
        // "None" must not appear
        assert!(!s.contains("None"), "None must not appear; got:\n{s}");
    }

    #[test]
    fn resource_types_prose_with_builtins() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut items = make_resource_type_fixture();
        items.push(ResourceType {
            name: "Region".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
            label: Some("Region".into()),
            is_builtin: true,
            count: None,
        });
        let view = ResourceTypeListView { items, total: 3, filter: None, data_model: "beol".into() };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let s = out.string();
        // Header includes builtin suffix
        assert!(
            s.contains("resource-types in beol on https://api.test.dasch.swiss (3), incl. built-ins:"),
            "header with builtin suffix missing; got:\n{s}"
        );
        // Region row must have (built-in) marker
        assert!(s.contains("(built-in)"), "(built-in) marker missing on built-in row; got:\n{s}");
        // Project rows must NOT have (built-in) marker
        let archive_line = s.lines().find(|l| l.contains("Archive")).unwrap();
        assert!(
            !archive_line.contains("(built-in)"),
            "project row must not have (built-in) marker; got: {archive_line:?}"
        );
    }

    #[test]
    fn resource_types_prose_with_filter() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let items = vec![make_resource_type_fixture().remove(0)]; // just Archive
        let view = ResourceTypeListView {
            items,
            total: 3,
            filter: Some("arch".to_string()),
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("(1 of 3 matching \"arch\")"),
            "filter header must show m of total; got:\n{s}"
        );
    }

    #[test]
    fn resource_types_prose_empty() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![],
            total: 0,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let s = out.string();
        // Header still shows data_model name even when empty
        assert!(
            s.contains("resource-types in beol on https://api.test.dasch.swiss (0):"),
            "header must show data_model name even when empty; got:\n{s}"
        );
        // Footer still present
        assert!(
            s.contains("[anonymous on https://api.test.dasch.swiss]"),
            "footer missing; got:\n{s}"
        );
    }

    #[test]
    fn resource_types_prose_with_counts() {
        // plan 030: when at least one item carries a count, a right-aligned
        // count column appears (both project and built-in rows), and the
        // count_caveat is appended to the dsp-cli/ADR-0007 footer suffix.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut items = make_resource_type_fixture();
        items[0].count = Some(42);
        items[1].count = Some(7);
        items.push(ResourceType {
            name: "Region".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
            label: Some("Region".into()),
            is_builtin: true,
            count: None,
        });
        let view = ResourceTypeListView { items, total: 3, filter: None, data_model: "beol".into() };
        let meta = MetaContext {
            server_label: "https://api.test.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts are not permission-filtered".into()),
            count_cost: None,
        };
        renderer.resource_types(&view, &meta).unwrap();

        let s = out.string();
        let archive_line = s.lines().find(|l| l.contains("Archive")).unwrap();
        assert!(
            archive_line.contains("42"),
            "Archive row must show count 42; got: {archive_line:?}"
        );
        let letter_line = s.lines().find(|l| l.contains("letter")).unwrap();
        assert!(letter_line.contains('7'), "letter row must show count 7; got: {letter_line:?}");
        // Built-in row with no count: count cell renders blank, marker still present.
        let region_line = s.lines().find(|l| l.contains("Region")).unwrap();
        assert!(
            region_line.contains("(built-in)"),
            "built-in marker must still be present; got: {region_line:?}"
        );
        // Disclosure footer carries the count_caveat.
        assert!(
            s.contains("counts are not permission-filtered"),
            "footer must carry count_caveat; got:\n{s}"
        );
    }

    // ── resource_type_describe prose tests ───────────────────────────────────

    /// Build a full beol manuscript fixture with representative fields:
    /// a text field (own DM), a link field (own DM), a cross-DM field
    /// (from biblio), a still-image representation, and an Extends.
    fn make_manuscript_detail() -> ResourceTypeDetail {
        ResourceTypeDetail {
            name: "manuscript".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".into(),
            label: Some("Manuscript".into()),
            data_model: "beol".into(),
            representation: Some(Representation::StillImage),
            super_types: vec!["writtenSource".into()],
            fields: vec![
                Field {
                    name: "title".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#title".into(),
                    label: Some("Title".into()),
                    value_type: ValueType::Text,
                    link_target: None,
                    cardinality: Cardinality::OneOrMore,
                    is_builtin: false,
                    data_model: Some("beol".into()),
                },
                Field {
                    name: "hasAuthor".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasAuthor".into(),
                    label: Some("Author".into()),
                    value_type: ValueType::Link,
                    link_target: Some("person".into()),
                    cardinality: Cardinality::ZeroOrMore,
                    is_builtin: false,
                    data_model: Some("beol".into()),
                },
                Field {
                    name: "isPartOfCollection".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2#isPartOfCollection".into(),
                    label: Some("is part of".into()),
                    value_type: ValueType::Link,
                    link_target: Some("Collection".into()),
                    cardinality: Cardinality::ZeroOrMore,
                    is_builtin: false,
                    data_model: Some("biblio".into()),
                },
            ],
            count: None,
        }
    }

    #[test]
    fn resource_type_describe_prose_full() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.dasch.swiss");
        renderer.resource_type_describe(&make_manuscript_detail(), &meta).unwrap();

        let s = out.string();
        // Header
        assert!(s.contains("Resource-type: manuscript"), "header missing; got:\n{s}");
        // Label line present
        assert!(s.contains("Label:          Manuscript"), "label line missing; got:\n{s}");
        // Extends line present (non-empty super_types)
        assert!(s.contains("Extends:        writtenSource"), "Extends line missing; got:\n{s}");
        // Representation line present (Some)
        assert!(
            s.contains("Representation: still-image"),
            "Representation line missing; got:\n{s}"
        );
        // IRI line always present
        assert!(
            s.contains("IRI:            http://api.dasch.swiss/ontology/0801/beol/v2#manuscript"),
            "IRI line missing; got:\n{s}"
        );
        // Data-model line
        assert!(s.contains("Data-model:     beol"), "Data-model line missing; got:\n{s}");
        // Fields header
        assert!(s.contains("Fields (3):"), "Fields header missing; got:\n{s}");
        // Title row (own DM — no source tag)
        let title_line = s.lines().find(|l| l.contains("title")).unwrap();
        assert!(
            title_line.contains("text") && title_line.contains("1-n"),
            "title row incomplete; got: {title_line:?}"
        );
        assert!(
            !title_line.contains("[from"),
            "title row must not have [from ...] tag; got: {title_line:?}"
        );
        // Link field with arrow notation
        let author_line = s.lines().find(|l| l.contains("hasAuthor")).unwrap();
        assert!(
            author_line.contains("\u{2192} person"),
            "link field must show '→ person'; got: {author_line:?}"
        );
        // Cross-DM source tag
        let coll_line = s.lines().find(|l| l.contains("isPartOfCollection")).unwrap();
        assert!(
            coll_line.contains("[from biblio]"),
            "cross-DM field must show [from biblio]; got: {coll_line:?}"
        );
        // dsp-cli/ADR-0007 footer
        assert!(
            s.contains("[anonymous on https://api.dasch.swiss]"),
            "footer missing; got:\n{s}"
        );
        // "None" must never appear
        assert!(!s.contains("None"), "None must not appear; got:\n{s}");
    }

    #[test]
    fn resource_type_describe_prose_with_count() {
        // plan 030: `Instances:` line appears right after `Data-model:` when
        // `detail.count` is Some, and the count_caveat is carried in the footer.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_manuscript_detail();
        detail.count = Some(123);
        let meta = MetaContext {
            server_label: "https://api.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts exclude deleted resources".into()),
            count_cost: None,
        };
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("Instances:      123"),
            "Instances line missing or misaligned; got:\n{s}"
        );
        // Instances line comes right after Data-model line.
        let dm_idx = s.find("Data-model:").expect("Data-model line present");
        let inst_idx = s.find("Instances:").expect("Instances line present");
        assert!(inst_idx > dm_idx, "Instances line must come after Data-model line");
        assert!(
            s.contains("counts exclude deleted resources"),
            "footer must carry count_caveat; got:\n{s}"
        );
    }

    #[test]
    fn resource_type_describe_prose_no_count_no_instances_line() {
        // Regression: count: None (today's only production case) must not
        // emit an Instances line.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.dasch.swiss");
        renderer.resource_type_describe(&make_manuscript_detail(), &meta).unwrap();

        let s = out.string();
        assert!(
            !s.contains("Instances:"),
            "Instances line must be absent when count is None; got:\n{s}"
        );
    }

    #[test]
    fn resource_type_describe_prose_no_label() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_manuscript_detail();
        detail.label = None;
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Label line must be absent
        assert!(
            !s.contains("Label:"),
            "Label: line must be absent when label is None; got:\n{s}"
        );
        // Header still present
        assert!(s.contains("Resource-type: manuscript"), "header missing; got:\n{s}");
        // "None" must not appear
        assert!(!s.contains("None"), "None must not appear; got:\n{s}");
    }

    #[test]
    fn resource_type_describe_prose_zero_fields() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ResourceTypeDetail {
            name: "Empty".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#Empty".into(),
            label: None,
            data_model: "minimal".into(),
            representation: None,
            super_types: vec![],
            fields: vec![],
            count: None,
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Zero branch: "Fields (0)" without colon, no rows
        assert!(s.contains("  Fields (0)"), "zero fields branch missing; got:\n{s}");
        // Must NOT have a colon after "(0)"
        let fields_line = s.lines().find(|l| l.contains("Fields (0)")).expect("must have Fields line");
        assert!(
            !fields_line.contains("Fields (0):"),
            "zero branch must NOT have a colon; got: {fields_line:?}"
        );
        // No Extends or Representation lines
        assert!(!s.contains("Extends:"), "Extends line must be absent; got:\n{s}");
        assert!(!s.contains("Representation:"), "Representation line must be absent; got:\n{s}");
    }

    #[test]
    fn resource_type_describe_prose_no_representation() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_manuscript_detail();
        detail.representation = None;
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Representation line must be absent
        assert!(
            !s.contains("Representation:"),
            "Representation: line must be absent when representation is None; got:\n{s}"
        );
    }

    #[test]
    fn resource_type_describe_prose_include_builtins_marker() {
        // A field with is_builtin=true must get a trailing " (built-in)" marker.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ResourceTypeDetail {
            name: "thing".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#thing".into(),
            label: None,
            data_model: "minimal".into(),
            representation: None,
            super_types: vec![],
            fields: vec![
                Field {
                    name: "hasText".into(),
                    iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#hasText".into(),
                    label: Some("Text content".into()),
                    value_type: ValueType::Text,
                    link_target: None,
                    cardinality: Cardinality::One,
                    is_builtin: false,
                    data_model: Some("minimal".into()),
                },
                Field {
                    name: "arkUrl".into(),
                    iri: "http://api.knora.org/ontology/knora-api/v2#arkUrl".into(),
                    label: Some("ARK URL".into()),
                    value_type: ValueType::Uri,
                    link_target: None,
                    cardinality: Cardinality::One,
                    is_builtin: true,
                    data_model: None,
                },
            ],
            count: None,
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Project field must NOT have (built-in) marker
        let text_line = s.lines().find(|l| l.contains("hasText")).unwrap();
        assert!(
            !text_line.contains("(built-in)"),
            "project field must not have (built-in) marker; got: {text_line:?}"
        );
        // Built-in field must have (built-in) marker
        let ark_line = s.lines().find(|l| l.contains("arkUrl")).unwrap();
        assert!(
            ark_line.contains("(built-in)"),
            "built-in field must have (built-in) marker; got: {ark_line:?}"
        );
    }

    #[test]
    fn resource_type_describe_prose_extends_multiple_supers() {
        // Multiple super_types → joined with ", "
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let detail = ResourceTypeDetail {
            name: "letter".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
            label: Some("Letter".into()),
            data_model: "beol".into(),
            representation: None,
            super_types: vec!["basicLetter".into(), "writtenSource".into()],
            fields: vec![],
            count: None,
        };
        let meta = make_meta("anonymous", "api.dasch.swiss");
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("Extends:        basicLetter, writtenSource"),
            "multiple supers must be joined with ', '; got:\n{s}"
        );
    }

    // ── resource_describe prose values tests ─────────────────────────────────

    use crate::model::{
        DatePoint, DateValue, FieldValues, FileValue, ResourceAccess, ResourceDetail, ResourceVisibility, Value,
        ValueContent,
    };

    fn make_resource_detail_no_values() -> ResourceDetail {
        ResourceDetail {
            label: "Test Resource".into(),
            iri: "http://rdfh.ch/0803/abc123".into(),
            resource_type: "Page".into(),
            ark_url: Some("ark:/72163/1/0803/abc123".into()),
            creation_date: Some("2021-01-01T00:00:00Z".into()),
            last_modified: None,
            attached_project: Some("http://rdfh.ch/projects/0803".into()),
            owner: Some("http://rdfh.ch/users/alice".into()),
            visibility: Some(ResourceVisibility::Public),
            your_access: Some(ResourceAccess::View),
            values: None,
        }
    }

    #[test]
    fn resource_describe_prose_no_values_unchanged() {
        // When values is None, output must be byte-identical to 8b behaviour —
        // no "Values:" section.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&make_resource_detail_no_values(), &meta).unwrap();

        let s = out.string();
        assert!(s.contains("Resource: Test Resource"), "header missing");
        assert!(!s.contains("Values:"), "Values: section must be absent when values is None");
    }

    #[test]
    fn resource_describe_prose_some_empty_values() {
        // When values is Some(vec![]), render "Values: (none)"
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("Values: (none)"),
            "empty values must render as 'Values: (none)'; got:\n{s}"
        );
    }

    #[test]
    fn resource_describe_prose_values_section() {
        // Smoke test: verify Values: section is present and basic field rendering works.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![
            FieldValues {
                name: "hasTitle".into(),
                label: Some("Title".into()),
                values: vec![ValueContent::Text("Incunabula Page".into()).into()],
            },
            FieldValues {
                name: "seqnum".into(),
                label: None, // degraded label — name only, no parens
                values: vec![ValueContent::Integer(42).into()],
            },
        ]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("Values:"), "Values: header missing; got:\n{s}");
        // Field with label: "<label> (<name>)"
        assert!(s.contains("Title (hasTitle)"), "labelled field header format wrong; got:\n{s}");
        assert!(s.contains("Incunabula Page"), "text value missing; got:\n{s}");
        // Field without label: "<name>" only, no parentheses
        let seqnum_line = s.lines().find(|l| l.contains("seqnum")).unwrap();
        assert!(
            !seqnum_line.contains('('),
            "unlabelled field must not have parentheses; got: {seqnum_line:?}"
        );
        assert!(s.contains("42"), "integer value missing; got:\n{s}");
    }

    #[test]
    fn resource_describe_prose_value_with_comment() {
        // A value carrying a `knora-api:valueHasComment` renders an indented
        // "comment: <text>" line directly under the value line.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasTranscription".into(),
            label: Some("Transcription".into()),
            values: vec![Value {
                content: ValueContent::Text("some transcription".into()),
                comment: Some("reading uncertain".into()),
            }],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("some transcription"), "value line missing; got:\n{s}");
        assert!(
            s.contains("      comment: reading uncertain"),
            "comment line missing or mis-indented; got:\n{s}"
        );
        // Comment line must immediately follow the value line.
        let lines: Vec<&str> = s.lines().collect();
        let value_idx = lines.iter().position(|l| l.contains("some transcription")).unwrap();
        assert_eq!(
            lines[value_idx + 1],
            "      comment: reading uncertain",
            "comment line must directly follow the value line; got:\n{s}"
        );
    }

    #[test]
    fn resource_describe_prose_value_without_comment_renders_no_comment_line() {
        // A value with `comment: None` must NOT emit a "comment: " line at all
        // (not an empty one) — the companion negative case to
        // `resource_describe_prose_value_with_comment`.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasTranscription".into(),
            label: Some("Transcription".into()),
            values: vec![ValueContent::Text("some transcription".into()).into()],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("some transcription"), "value line missing; got:\n{s}");
        assert!(
            !s.contains("comment:"),
            "no comment line must be emitted when value.comment is None; got:\n{s}"
        );
    }

    #[test]
    fn resource_describe_prose_link_with_label() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "isPartOf".into(),
            label: Some("Is part of".into()),
            values: vec![
                ValueContent::Link {
                    target_iri: "http://rdfh.ch/0803/book1".into(),
                    target_label: Some("Incunabula Book".into()),
                }
                .into(),
                ValueContent::Link {
                    target_iri: "http://rdfh.ch/0803/book2".into(),
                    target_label: None, // degraded link — no brackets
                }
                .into(),
            ],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Link with label: "→ <label> [<iri>]"
        assert!(
            s.contains("\u{2192} Incunabula Book [http://rdfh.ch/0803/book1]"),
            "link with label rendered incorrectly; got:\n{s}"
        );
        // Link without label: "→ <iri>" (no brackets)
        assert!(
            s.contains("\u{2192} http://rdfh.ch/0803/book2"),
            "degraded link (no label) rendered incorrectly; got:\n{s}"
        );
        // No square brackets around the degraded link
        let degraded_line = s.lines().find(|l| l.contains("http://rdfh.ch/0803/book2")).unwrap();
        assert!(
            !degraded_line.contains('['),
            "degraded link must not have brackets; got: {degraded_line:?}"
        );
    }

    #[test]
    fn resource_describe_prose_file_still_image_with_dims() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasStillImageFileValue".into(),
            label: None,
            values: vec![
                ValueContent::File(FileValue {
                    value_type: ValueType::StillImage,
                    filename: "image.jp2".into(),
                    url: "https://iiif.example.com/image.jp2/full/max/0/default.jpg".into(),
                    width: Some(1200),
                    height: Some(800),
                })
                .into(),
            ],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Still-image: "<filename> (<W>×<H>) <url>"
        assert!(
            s.contains("image.jp2 (1200\u{d7}800) https://iiif.example.com"),
            "still-image with dims rendered incorrectly; got:\n{s}"
        );
    }

    #[test]
    fn resource_describe_prose_date_rendering() {
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        // Single-point date (start == end)
        let pt = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let single = DateValue {
            calendar: "GREGORIAN".into(),
            start: pt.clone(),
            end: pt.clone(),
        };
        // Range date
        let range = DateValue {
            calendar: "GREGORIAN".into(),
            start: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: Some(1490),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
        };
        detail.values = Some(vec![FieldValues {
            name: "hasDate".into(),
            label: Some("Date".into()),
            values: vec![ValueContent::Date(single).into(), ValueContent::Date(range).into()],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        // Single-point date
        assert!(
            s.contains("1489 CE (GREGORIAN)"),
            "single-point date rendered incorrectly; got:\n{s}"
        );
        // Range date with en-dash
        assert!(
            s.contains("1489 CE \u{2013} 1490 CE (GREGORIAN)"),
            "range date rendered incorrectly; got:\n{s}"
        );
    }

    // ── resource label control-char sanitisation (Phase 8.5 #1) ───────────────

    #[test]
    fn resource_describe_prose_strips_control_chars_in_label() {
        // A resource (instance) label carrying control characters must not reach
        // the terminal raw — terminal-injection / display hardening.
        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.label = "Bad\u{1b}[31mLabel\u{7f}".into();
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let s = out.string();
        assert!(
            s.contains("Resource: Bad[31mLabel"),
            "label control chars not stripped; got:\n{s}"
        );
        assert!(
            !s.contains('\u{1b}') && !s.contains('\u{7f}'),
            "control characters leaked into prose output; got:\n{s:?}"
        );
    }

    #[test]
    fn resources_prose_strips_control_chars_in_label() {
        use crate::model::ResourceSummary;

        let out = SharedBuf::new();
        let mut renderer = ProseRenderer::with_writer(out.clone());
        let view = ResourceListView {
            items: vec![ResourceSummary {
                label: "Bad\u{1b}[31mLabel\u{7f}".into(),
                iri: "http://rdfh.ch/0803/abc123".into(),
                ark_url: None,
                creation_date: None,
                last_modified: None,
                resource_type: "Page".into(),
            }],
            total: 1,
            filter: None,
            resource_type: "Page".into(),
            pagination: ResourceListPagination::SinglePage { page: 0, may_have_more: false },
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resources(&view, &meta).unwrap();

        let s = out.string();
        assert!(s.contains("Bad[31mLabel"), "label control chars not stripped; got:\n{s}");
        assert!(
            !s.contains('\u{1b}') && !s.contains('\u{7f}'),
            "control characters leaked into prose output; got:\n{s:?}"
        );
    }
}
