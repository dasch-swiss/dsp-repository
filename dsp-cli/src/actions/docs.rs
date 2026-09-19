//! Actions for `dsp docs [topic]`.
//!
//! `docs` is the one action with neither a `DspClient` (it reads embedded files,
//! never the network) nor a `Renderer`: its output is raw markdown (a topic body)
//! or a plain prose topic list, both format-agnostic, so the five-format renderer
//! matrix does not apply and `DocsArgs` carries no `--format` flag. Output goes to
//! an injected writer via the `run` / `run_impl` seam (the same testability pattern
//! as `auth::status`). See dsp-cli/ADR-0010.
//!
//! Topics are authored as markdown under `docs/topics/` and embedded at compile
//! time with `include_str!` — a missing file is a compile error.

use std::io::{self, Write};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::cli::DocsArgs;
use crate::diagnostic::Diagnostic;

/// A unit of embedded end-user documentation. See CONTEXT.md ("Topic").
pub struct Topic {
    /// The short name used as `dsp docs <name>`.
    pub name: &'static str,
    /// One-line description shown in the topic list.
    pub summary: &'static str,
    /// The full markdown body, embedded at compile time.
    pub body: &'static str,
}

/// The v1 topic catalog (dsp-cli/ADR-0010, expanded by plan 018). Table order is the
/// display order in the topic list; it follows a first-read progression.
const TOPICS: &[Topic] = &[
    Topic {
        name: "dsp-cli",
        summary: "What this tool is, who it's for, and its design principles.",
        body: include_str!("../../docs/topics/dsp-cli.md"),
    },
    Topic {
        name: "dsp",
        summary: "The DaSCH Service Platform in brief: VRE vs Repository.",
        body: include_str!("../../docs/topics/dsp.md"),
    },
    Topic {
        name: "concepts",
        summary: "The vocabulary dsp-cli speaks (project, data-model, resource-type, …).",
        body: include_str!("../../docs/topics/concepts.md"),
    },
    Topic {
        name: "identifiers",
        summary: "How to name projects, data-models, and resource-types.",
        body: include_str!("../../docs/topics/identifiers.md"),
    },
    Topic {
        name: "connecting",
        summary: "Servers, environments, and authentication.",
        body: include_str!("../../docs/topics/connecting.md"),
    },
    Topic {
        name: "output",
        summary: "Output formats, channels, and the JSON envelope.",
        body: include_str!("../../docs/topics/output.md"),
    },
    Topic {
        name: "workflows",
        summary: "Chaining commands into real tasks.",
        body: include_str!("../../docs/topics/workflows.md"),
    },
    Topic {
        name: "errors",
        summary: "Exit codes and how to recover from failures.",
        body: include_str!("../../docs/topics/errors.md"),
    },
    Topic {
        name: "dsp-tools",
        summary: "When to use dsp-cli versus dsp-tools.",
        body: include_str!("../../docs/topics/dsp-tools.md"),
    },
    Topic {
        name: "sparql",
        summary: "Raw SPARQL passthrough: `dsp vre sparql query`.",
        body: include_str!("../../docs/topics/sparql.md"),
    },
];

/// Display embedded documentation: list topics, or print one topic's body.
///
/// Stdout is wrapped in `BrokenPipeWriter` so `dsp docs ... | head` exits 0
/// silently instead of surfacing a broken pipe as `Diagnostic::Internal`.
pub fn run(args: &DocsArgs) -> Result<(), Diagnostic> {
    let mut out = crate::util::BrokenPipeWriter::new(io::stdout().lock());
    run_impl(args, &mut out)
}

/// Machine-readable JSON topic index: the outer envelope.
///
/// `_meta` is declared first so serde's insertion-order serialisation places it
/// before `data` in the output — a load-bearing ordering guaranteed by the
/// `preserve_order` feature on `serde_json` (dsp-cli/ADR-0003). For `dsp docs -j` there
/// is no server/auth context, so `_meta` is the empty object `{}` (plan 020 D4).
#[derive(Serialize)]
struct DocsJsonEnvelope<'a> {
    _meta: EmptyMeta,
    data: Vec<TopicIndexEntry<'a>>,
}

/// The empty `_meta` block for `dsp docs -j` (no server/auth context applies to
/// embedded documentation). Serialises as `{}`. See plan 020 D4 and dsp-cli/ADR-0003
/// amendment for the empty-`_meta` carve-out.
#[derive(Serialize)]
struct EmptyMeta {}

/// One entry in the JSON topic index: name + summary only (bodies never
/// JSON-wrapped — they are raw markdown surfaced via `dsp docs <topic>`).
#[derive(Serialize)]
struct TopicIndexEntry<'a> {
    name: &'a str,
    summary: &'a str,
}

/// Testable core: writes the topic list or a topic body to `out`. A bad topic
/// name returns `NotFound` (exit 1) with a "did you mean" suggestion; `main.rs`
/// prints that to stderr.
fn run_impl(args: &DocsArgs, out: &mut dyn Write) -> Result<(), Diagnostic> {
    if args.json {
        return write_topic_index_json(out);
    }
    match args.topic.as_deref() {
        None => write_topic_list(out),
        Some(name) => match find_topic(name) {
            Some(topic) => emit(topic.body, args.pager, out),
            None => Err(not_found(name)),
        },
    }
}

/// Emit the JSON topic index: `{"_meta":{},"data":[{"name":"…","summary":"…"},…]}`.
///
/// Uses compact `serde_json::to_string` (same style as `src/render/json.rs`) so
/// `dsp docs -j` output is visually uniform with other `dsp … -j` commands.
/// `_meta` is first because it is the first declared field on `DocsJsonEnvelope`
/// (dsp-cli/ADR-0003, plan 020 D4).
fn write_topic_index_json(out: &mut dyn Write) -> Result<(), Diagnostic> {
    let data: Vec<TopicIndexEntry<'_>> = TOPICS
        .iter()
        .map(|t| TopicIndexEntry { name: t.name, summary: t.summary })
        .collect();
    let envelope = DocsJsonEnvelope { _meta: EmptyMeta {}, data };
    let json =
        serde_json::to_string(&envelope).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?;
    writeln!(out, "{json}")?;
    Ok(())
}

/// Exact-match topic lookup. No partial/prefix matching (dsp-cli/ADR-0010: silent-shadowing
/// risk when topics are added later).
fn find_topic(name: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.name == name)
}

/// Build the not-found diagnostic, appending a "did you mean" when a close topic
/// name exists.
fn not_found(name: &str) -> Diagnostic {
    let suggestion = match suggest(name) {
        Some(s) => format!(" Did you mean '{s}'?"),
        None => String::new(),
    };
    Diagnostic::NotFound(format!(
        "no documentation topic named '{name}'.{suggestion} Run `dsp docs` to see all topics."
    ))
}

/// The closest topic name within an edit-distance threshold, or `None`.
fn suggest(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return None;
    }
    // Threshold: small absolute distance, capped below the input length so a short
    // garbage string doesn't match a long topic name.
    let threshold = 3.min(name.len());
    TOPICS
        .iter()
        .map(|t| (levenshtein(name, t.name), t.name))
        .filter(|(dist, _)| *dist <= threshold)
        .min_by_key(|(dist, _)| *dist)
        .map(|(_, n)| n)
}

/// Classic dynamic-programming Levenshtein edit distance (insert/delete/substitute,
/// cost 1 each). Inlined to avoid a dependency for one small use.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Render the topic list (a successful command: data to stdout, exit 0).
fn write_topic_list(out: &mut dyn Write) -> Result<(), Diagnostic> {
    let width = TOPICS.iter().map(|t| t.name.len()).max().unwrap_or(0);
    writeln!(out, "Available documentation topics:")?;
    writeln!(out)?;
    for t in TOPICS {
        writeln!(out, "  {:<width$}  {}", t.name, t.summary, width = width)?;
    }
    writeln!(out)?;
    writeln!(out, "Run `dsp docs <topic>` to read one.")?;
    Ok(())
}

/// Write a topic body, optionally through a pager. Pager failure (or no pager
/// available) falls back to a direct write — never fatal.
fn emit(content: &str, use_pager: bool, out: &mut dyn Write) -> Result<(), Diagnostic> {
    if use_pager && try_pager(content).is_ok() {
        return Ok(());
    }
    out.write_all(content.as_bytes())?;
    Ok(())
}

/// Pipe `content` through `$PAGER` (default `less`). The pager inherits our stdout,
/// so this bypasses `out` entirely; it is engaged only on the real-binary `--pager`
/// path and is not unit-tested.
fn try_pager(content: &str) -> io::Result<()> {
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
    let mut parts = pager.split_whitespace();
    let program = parts.next().unwrap_or("less");
    let mut child = Command::new(program).args(parts).stdin(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_list() -> String {
        let mut buf: Vec<u8> = Vec::new();
        write_topic_list(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn find_topic_hits_known_name() {
        assert!(find_topic("concepts").is_some());
        assert_eq!(find_topic("concepts").unwrap().name, "concepts");
    }

    #[test]
    fn find_topic_misses_unknown_name() {
        assert!(find_topic("nope").is_none());
    }

    #[test]
    fn find_topic_is_exact_not_prefix() {
        // "concept" must NOT match "concepts" (dsp-cli/ADR-0010: no partial matching).
        assert!(find_topic("concept").is_none());
        assert!(find_topic("dsp-").is_none());
    }

    #[test]
    fn suggest_finds_near_neighbour() {
        assert_eq!(suggest("concept"), Some("concepts")); // distance 1
        assert_eq!(suggest("conecting"), Some("connecting")); // distance 1
        assert_eq!(suggest("error"), Some("errors")); // distance 1
    }

    #[test]
    fn suggest_returns_none_for_far_input() {
        assert_eq!(suggest("xyzzy"), None);
        assert_eq!(suggest(""), None);
    }

    #[test]
    fn levenshtein_basics() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("same", "same"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
    }

    #[test]
    fn not_found_includes_suggestion_when_close() {
        let msg = not_found("concept").to_string();
        assert!(msg.contains("no documentation topic named 'concept'"));
        assert!(msg.contains("Did you mean 'concepts'?"));
        assert!(msg.contains("Run `dsp docs`"));
    }

    #[test]
    fn not_found_omits_suggestion_when_far() {
        let msg = not_found("xyzzy").to_string();
        assert!(msg.contains("no documentation topic named 'xyzzy'"));
        assert!(!msg.contains("Did you mean"));
    }

    #[test]
    fn topic_list_includes_every_topic() {
        let list = render_list();
        for t in TOPICS {
            assert!(list.contains(t.name), "list missing topic {}", t.name);
            assert!(list.contains(t.summary), "list missing summary for {}", t.name);
        }
    }

    #[test]
    fn all_topic_bodies_are_present_and_well_formed() {
        // Smoke test in lieu of brittle per-body snapshots (dsp-cli/ADR-0009 exception, plan
        // 018 D5): every catalogued body is non-empty and starts with an h1 heading.
        for t in TOPICS {
            assert!(!t.body.trim().is_empty(), "empty body for topic {}", t.name);
            assert!(t.body.starts_with("# "), "topic {} body must start with an h1 heading", t.name);
        }
    }

    #[test]
    fn catalog_has_ten_topics_with_unique_names() {
        assert_eq!(TOPICS.len(), 10);
        for (i, t) in TOPICS.iter().enumerate() {
            for other in &TOPICS[i + 1..] {
                assert_ne!(t.name, other.name, "duplicate topic name {}", t.name);
            }
        }
    }

    #[test]
    fn run_impl_no_topic_writes_list() {
        let args = DocsArgs { topic: None, pager: false, json: false };
        let mut buf: Vec<u8> = Vec::new();
        run_impl(&args, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("Available documentation topics:"));
        assert!(out.contains("workflows"));
    }

    #[test]
    fn run_impl_known_topic_writes_body() {
        let args = DocsArgs {
            topic: Some("concepts".to_string()),
            pager: false,
            json: false,
        };
        let mut buf: Vec<u8> = Vec::new();
        run_impl(&args, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.starts_with("# "));
    }

    #[test]
    fn run_impl_unknown_topic_errors() {
        let args = DocsArgs { topic: Some("nope".to_string()), pager: false, json: false };
        let mut buf: Vec<u8> = Vec::new();
        let err = run_impl(&args, &mut buf).unwrap_err();
        assert!(matches!(err, Diagnostic::NotFound(_)));
        assert!(buf.is_empty(), "nothing should be written on error");
    }
}
