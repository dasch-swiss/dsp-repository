//! `Format` enum and renderer factory.
//!
//! `Format` is the render-layer concept that maps a user-chosen output format
//! to a concrete renderer instance. It is also the `ValueEnum` that clap uses
//! for the `--format` flag on the six vre leaf commands. See ADR-0003 for the
//! output-format specification and ADR-0008 for the renderer-layer placement.

use clap::ValueEnum;

use super::csv::CsvRenderer;
use super::json::JsonRenderer;
use super::lines::LinesRenderer;
use super::progress::{HumanProgress, JsonProgress, ProgressReporter};
use super::prose::ProseRenderer;
use super::tsv::TsvRenderer;
use super::{Renderer, TableOptions};

/// Output format for a command.
///
/// `Prose` is the default (per ADR-0003). The other variants map to the
/// machine-readable formats; their renderers began as Phase 1 stubs (only
/// `diagnostic`) and grow per-noun methods with real data (Phase 3 `dump`,
/// Phase 4 `projects`, Phase 5 onward).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Rich human-readable prose output with contextual hints (default).
    #[value(name = "prose")]
    Prose,

    /// Newline-delimited JSON (one object per line).
    #[value(name = "json")]
    Json,

    /// One identifier per line, for shell pipeline consumption.
    #[value(name = "lines")]
    Lines,

    /// Comma-separated values with a header row.
    #[value(name = "csv")]
    Csv,

    /// Tab-separated values with a header row.
    #[value(name = "tsv")]
    Tsv,
}

impl Format {
    /// Construct the renderer for this format, writing to stdout.
    ///
    /// Returns a heap-allocated `Box<dyn Renderer>`. Per-noun methods grow as
    /// real data lands (Phase 3 `dump`, Phase 4 `projects`, Phase 5 onward).
    /// Use `into_renderer_with_writer` in tests to capture output.
    ///
    /// Uses `TableOptions::default()`. Call [`Format::into_renderer_with_options`]
    /// to supply user-specified column projection or header-control options.
    pub fn into_renderer(self) -> Box<dyn Renderer> {
        self.into_renderer_with_options(TableOptions::default())
    }

    /// Construct the renderer for this format with the given tabular options.
    ///
    /// For `csv`, `tsv`, and `lines`, `opts` controls column projection
    /// (`--columns`) and header emission (`--no-header` / `--header-only`).
    /// For `prose` and `json`, `opts` is ignored — those formats are not column-
    /// structured and do not honour projection or header flags.
    pub fn into_renderer_with_options(self, opts: TableOptions) -> Box<dyn Renderer> {
        match self {
            Format::Prose => Box::new(ProseRenderer::new()),
            Format::Json => Box::new(JsonRenderer::new()),
            Format::Lines => Box::new(LinesRenderer::new().with_options(opts)),
            Format::Csv => Box::new(CsvRenderer::new().with_options(opts)),
            Format::Tsv => Box::new(TsvRenderer::new().with_options(opts)),
        }
    }

    /// Construct the progress reporter for this format, writing to stderr.
    ///
    /// `Format::Json` → `JsonProgress` (NDJSON events on stderr so a JSON
    /// consumer can parse both the final stdout object and the stderr event
    /// stream with the same parser). All other formats → `HumanProgress`
    /// (prose/lines/csv/tsv share human-readable stderr — stderr is non-data
    /// output so the format distinction does not apply there).
    ///
    /// `Format` is `Copy`, so a caller can do:
    /// ```
    /// # use dsp_cli::render::{Format, ProgressReporter};
    /// # let fmt = Format::Prose;
    /// let mut renderer = fmt.into_renderer();
    /// let mut reporter = fmt.into_progress_reporter();
    /// ```
    pub fn into_progress_reporter(self) -> Box<dyn ProgressReporter> {
        match self {
            Format::Json => Box::new(JsonProgress::new()),
            Format::Prose | Format::Lines | Format::Csv | Format::Tsv => Box::new(HumanProgress::new()),
        }
    }
}
