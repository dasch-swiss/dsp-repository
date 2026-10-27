//! Shared pure utilities reused across engine modules.

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, TagEnd};

/// The CommonMark parser options of the **richtext** path: property values
/// (`component::markdown`'s fragments, including the `narrative-for` intros)
/// and [`markdown_to_text`]'s FTS text. Editorial narratives and the heading
/// traversal below parse with `EDITORIAL_MARKDOWN_OPTIONS` instead; extend
/// the shared set here and both stay in sync.
pub const MARKDOWN_OPTIONS: Options = Options::ENABLE_TABLES;

/// The parser options of an **editorial narrative** (`component::editorial`)
/// and of [`headings`]: [`MARKDOWN_OPTIONS`] plus heading attributes, so a
/// `## Title {#id}` heading carries an explicit id (a figure block's section
/// anchor) and the `{#id}` never reaches a derived title or crumb. The two
/// must stay equal, or a title would show the `{#id}` the page does not.
/// Richtext keeps the option off: a `{#id}` there renders as literal text.
pub(crate) const EDITORIAL_MARKDOWN_OPTIONS: Options = MARKDOWN_OPTIONS.union(Options::ENABLE_HEADING_ATTRIBUTES);

/// One heading of a Markdown document, as [`headings`] collects it.
struct Heading {
    level: HeadingLevel,
    /// The trimmed plain text.
    text: String,
    /// The explicit `{#id}`, if any.
    id: Option<String>,
}

/// Collect **every** heading in `md`, in document order — the single
/// traversal every heading helper below ([`first_heading`],
/// [`narrative_contains_h1`], [`page_title_heading`], [`heading_ids`]) is
/// expressed on top of.
///
/// Uses [`EDITORIAL_MARKDOWN_OPTIONS`] (the same parser options as the
/// narrative renderer). Collects `Event::Text` and `Event::Code`
/// payloads (so inline code in a heading is preserved as plain text) until
/// the heading's `End` event; `Event::Html`/`Event::InlineHtml` are never
/// collected, so the result is always plain text, never markup.
///
/// **Empty headings are kept, not filtered.** An ATX heading whose content
/// is only whitespace (`"#  "`) yields an entry with empty text. This is
/// load-bearing for `first_heading`'s contract (an empty *first* heading
/// means `None`, NOT "skip to the next heading") — dropping them here would
/// silently change that behaviour.
fn headings(md: &str) -> Vec<Heading> {
    let parser = Parser::new_ext(md, EDITORIAL_MARKDOWN_OPTIONS);

    let mut out: Vec<Heading> = Vec::new();
    let mut current: Option<Heading> = None;

    for event in parser {
        match event {
            Event::Start(pulldown_cmark::Tag::Heading { level, id, .. }) => {
                current = Some(Heading { level, text: String::new(), id: id.map(|id| id.to_string()) });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut heading) = current.take() {
                    heading.text = heading.text.trim().to_owned();
                    out.push(heading);
                }
            }
            Event::Text(t) => {
                if let Some(heading) = current.as_mut() {
                    heading.text.push_str(&t);
                }
            }
            Event::Code(c) => {
                if let Some(heading) = current.as_mut() {
                    heading.text.push_str(&c);
                }
            }
            _ => {}
        }
    }
    out
}

/// Every explicit heading id (`## Title {#id}`) in `md`, with its heading's
/// level, in document order; duplicates are kept. The boot checks read it:
/// `InvalidHeadingId` over every id, `UnknownFigureSection` for the level.
pub(crate) fn heading_ids(md: &str) -> Vec<(HeadingLevel, String)> {
    headings(md).into_iter().filter_map(|h| h.id.map(|id| (h.level, id))).collect()
}

/// Extract the plain-text content of the **first** heading (any level) in `md`.
///
/// Expressed over [`headings`]. Returns the trimmed text of the first
/// heading in document order, or `None` when `md` has no heading at all
/// **or** when that first heading's text is empty after trimming (an empty
/// first heading is `None`, never a fall-through to a later heading).
///
/// Never returns markup — the result is always a plain `String` suitable for
/// use as an HTML `<title>` text node.
///
/// Deliberately **level-blind**: it cannot answer "is there an `<h1>`" (see
/// [`narrative_contains_h1`]) nor "what is the page's title" on a document
/// that opens with a lower-level heading (see [`page_title_heading`]).
pub(crate) fn first_heading(md: &str) -> Option<String> {
    headings(md).into_iter().next().and_then(|h| non_empty(h.text))
}

/// The text of the **first level-1 heading** in `md`, falling back to
/// `first_heading`'s answer when the document has no (non-empty) `<h1>`.
///
/// **Why this exists.** An editorial narrative may open with a leading
/// level-3 heading that is a *label*, not a section heading — the
/// eyebrow/kicker, rendered by `component::editorial`'s `rewrite_events` as
/// `p.eyebrow.editorial__kicker` at the top of `header.editorial__head`
/// (see that module's "Conventions recognised" list). Incunabula's
/// narratives use it (`### Basler Frühdrucke · 1471–1504` above
/// `# Die Bilderfolgen der Basler Frühdrucke`). Because `first_heading`
/// is level-blind, deriving the page `<title>` and the breadcrumb's
/// page-title segment from it picked the kicker — the Incunabula home
/// `<title>` read "… — Basler Frühdrucke · 1471–1504" and
/// `/incunabula/narrenschiff`'s crumb read "Showcase I" instead of "The
/// Narrenschiff". A label is not a title, so title derivation prefers the
/// `<h1>`.
///
/// Falls back to `first_heading` (rather than `None`) so a document with
/// no `<h1>` at all keeps its previous, still-useful derived title — the
/// only behaviour change is on documents that *do* have an `<h1>` which is
/// not the first heading.
///
/// A `###` directly *after* the h1 is the editorial **subtitle**
/// (`p.editorial__subtitle`); it follows the h1, so the first-`<h1>` rule
/// never reads it.
pub fn page_title_heading(md: &str) -> Option<String> {
    headings(md)
        .into_iter()
        .find(|h| h.level == HeadingLevel::H1)
        .and_then(|h| non_empty(h.text))
        // No usable `<h1>` — defer to the level-blind answer rather than
        // restating its "an empty FIRST heading means None" contract here.
        // Costs a second `headings` traversal, only on this rare path.
        .or_else(|| first_heading(md))
}

/// `Some(text)` iff `text` is non-empty. The shared "an empty heading is no
/// heading" rule of [`first_heading`] and [`page_title_heading`] (the text
/// is already trimmed by [`headings`]).
fn non_empty(text: String) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Return `true` iff `md` contains a level-1 heading **anywhere** in
/// document order (not just first). Used only for the h1-**injection**
/// decision (`page_has_visible_h1` in `dsp-incubator cpe/web`): a narrative
/// whose first heading is level 2 but which contains a level-1 heading further
/// down (e.g. `"## intro\n\n# title"`) already renders a visible `<h1>` via
/// `editorial::render_editorial` (which passes every source heading through as
/// a literal `<hN>` tag) — checking only the *first* heading's level would
/// misclassify such a narrative as headless and inject a SECOND hidden `<h1>`
/// on top of the real one. `first_heading` deliberately discards heading level,
/// so it cannot answer this question — hence this dedicated level-aware helper,
/// expressed over the same `headings` traversal.
///
/// Note this is `true` even for an *empty* level-1 heading (`"#  "`): the
/// renderer emits `<h1></h1>` for it, so a visible h1 element really does
/// exist and injecting another would still double up. This is deliberately
/// unlike `first_heading`/[`page_title_heading`], which reject empty text
/// because they answer a *text* question, not an *element-presence* one.
///
/// Returns `false` when `md` has no level-1 heading at all (including no
/// heading whatsoever).
pub fn narrative_contains_h1(md: &str) -> bool {
    headings(md).iter().any(|h| h.level == HeadingLevel::H1)
}

/// Strip Markdown down to its plain-text visible content, for FTS indexing.
///
/// Parses `md` with [`MARKDOWN_OPTIONS`] (the richtext options: its input is
/// a richtext property value, never a narrative) and concatenates every
/// `Event::Text` and `Event::Code` payload across the whole document (not
/// scoped to a single heading, unlike `first_heading`). `Event::Html` and
/// `Event::InlineHtml` are explicitly dropped, not collected — richtext
/// Markdown is never trusted to inject raw HTML into the plain-text FTS
/// index.
///
/// **Block boundaries get a separating space.** Before the content of a new
/// block-level element (`is_block_tag` — paragraph, heading, list item,
/// block quote, table cell, …), a single space is inserted if the buffer is
/// non-empty and doesn't already end in whitespace (`push_block_separator`).
/// Without this, `"# Title\n\nSome text."` would concatenate the heading's
/// text directly against the following paragraph's text with NO separator
/// (`"TitleSome text."`), fusing two words into one un-matchable FTS token.
/// Inline-level tags (emphasis, links, …) do NOT trigger this — only the
/// block-tag boundary does, so `"Some *emphasized* text."` still collapses
/// to `"Some emphasized text."` (the source's own spaces do the work there),
/// not `"Some  emphasized  text."`.
///
/// Returns the trimmed concatenation. Unlike `first_heading`, this returns a
/// plain `String`, not `Option<String>`: an empty search-index entry for one
/// property is a valid (if unhelpful) outcome, not an error state.
pub fn markdown_to_text(md: &str) -> String {
    let parser = Parser::new_ext(md, MARKDOWN_OPTIONS);

    let mut buf = String::new();
    for event in parser {
        match event {
            Event::Start(ref tag) if is_block_tag(tag) => push_block_separator(&mut buf),
            Event::Text(t) => buf.push_str(&t),
            Event::Code(c) => buf.push_str(&c),
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => {}
        }
    }
    buf.trim().to_owned()
}

/// `true` for every block-level [`pulldown_cmark::Tag`] variant (the ones
/// pulldown-cmark itself groups above its "span-level tags" comment in
/// `Tag`'s definition) — `false` for inline/span-level tags (`Emphasis`,
/// `Strong`, `Link`, …). Used only by [`markdown_to_text`] to decide where a
/// block-boundary separator is needed; inline tags must NOT trigger one (see
/// that function's doc comment).
pub(crate) fn is_block_tag(tag: &pulldown_cmark::Tag<'_>) -> bool {
    use pulldown_cmark::Tag;
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
    )
}

/// Push a single ASCII space onto `buf` iff it is non-empty and does not
/// already end in whitespace. Idempotent — calling it when `buf` is empty or
/// already ends in whitespace is a no-op, so consecutive block boundaries
/// (e.g. a `List` immediately followed by its first `Item`) never produce a
/// doubled space, and a fresh/leading position never gets a stray leading
/// space (the caller still `.trim()`s the final result as a belt-and-suspenders).
fn push_block_separator(buf: &mut String) {
    if !buf.is_empty() && !buf.ends_with(|c: char| c.is_whitespace()) {
        buf.push(' ');
    }
}

/// Convert a schema property name into a human-readable label.
///
/// Splits on `-`, `_`, and space; title-cases each word; joins with a single
/// space. Consecutive separators (e.g. `"a--b"`) produce no empty word in the
/// output. Language-independent — the result is always English-shaped (the
/// property names themselves are English-derived in DaSCH schemas).
///
/// Examples: `"name"` → `"Name"`, `"location-type"` → `"Location Type"`,
/// `"full_text"` → `"Full Text"`, `"a--b__c"` → `"A B C"`.
///
/// `pub` so crates other than `cpe-core` can call it: `labels::resolve_property_label`
/// falls back to it for both `component::resource_detail`'s property `<dt>` labels
/// and `query::reads::ProjectReads::facet_options`'s facet group labels, and the
/// tests of `cpe-components` and `dsp-incubator cpe/web` call it
/// directly.
pub fn humanize_property_name(name: &str) -> String {
    name.split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Split `"home.md"` → `("home", "md")`. Falls back to `(file, "")` if no dot.
///
/// `pub(crate)` so both `validate` and `variant_filename` (which
/// `component::narrative` calls) share the same last-dot semantics without
/// re-implementing the `rfind` logic. The
/// variant-resolution contract (stem = everything before the last dot, ext =
/// everything after) must stay identical between validation and rendering.
pub(crate) fn split_stem_ext(file: &str) -> (String, String) {
    if let Some(dot) = file.rfind('.') {
        (file[..dot].to_owned(), file[dot + 1..].to_owned())
    } else {
        (file.to_owned(), String::new())
    }
}

/// Build the language-variant filename for `file` + `lang`, e.g.
/// `("home.md", "de")` → `Some("home.de.md")`. Returns `None` when `file`
/// has no extension (no variant form is possible — use the file as-is).
///
/// `pub` so both `validate` (the default-language-variant existence
/// probe) and `component::narrative` (render-time variant resolution) build
/// the candidate name with identical last-dot semantics — the naming rule
/// must never drift between validation and rendering.
pub fn variant_filename(file: &str, lang: &str) -> Option<String> {
    let (stem, ext) = split_stem_ext(file);
    if ext.is_empty() {
        None
    } else {
        Some(format!("{stem}.{lang}.{ext}"))
    }
}

/// Percent- (and `+`-) decode a raw query-string VALUE that has already been
/// split out of its `key=value` pair (i.e. this takes just the `value` half,
/// not a full `a=b&c=d` string).
///
/// **Why this exists, and why every other query reader in this engine does
/// NOT decode:** every other query value the engine's renderers read from
/// `RenderContext.current_query` (`sort_param`'s `?sort=` closed vocabulary,
/// facet/list-node ids, `?page=`/`?member=` numeric indices) is a closed-
/// vocabulary identifier that never legitimately contains a space or a
/// reserved URL character, so decoding was never needed. `?q=` (the search-
/// bar's free-text value) is the first genuinely free-text value the engine
/// renders — it must be decoded to display correctly (the input's `value=`
/// attribute, the query chip's text, the no-match message) since a real
/// multi-word query arrives escaped: `+` from the zero-JS
/// `<form method=get>` fallback's `application/x-www-form-urlencoded`
/// submission, or `%20` from the DataStar debounced-live path's
/// `encodeURIComponent`. Mirrors `read_search_query_param`
/// (`dsp-incubator cpe/bin/src/router.rs`) exactly (same `+`-then-percent
/// decode order — a literal `+` means a space per the form-encoding convention,
/// applied BEFORE `%XX` percent-decoding, since raw percent-decoding alone does
/// not treat `+` specially) — this crate keeps its own copy since it cannot
/// depend on the host crate.
///
/// Never panics: falls back to the raw (still `+`-decoded) input on any
/// invalid UTF-8 percent sequence.
pub fn decode_query_value(raw: &str) -> String {
    let plus_decoded = raw.replace('+', " ");
    percent_encoding::percent_decode_str(&plus_decoded)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or(plus_decoded)
}

/// Build the carry-over hidden `(key, value)` pairs for a zero-JS
/// `<form method="get">`: every PAIR of `current_query` (split on `&` and the
/// first `=`, pair-based not a deduped map so a repeated facet key like
/// `role=R_01&role=R_02` round-trips both values) whose decoded key is NOT in
/// `drop`, with key and value each decoded ONCE via [`decode_query_value`].
///
/// **Why decode:** the result is written into `<input type="hidden"
/// name=… value=…>`, and the browser form-encodes those on submit. Writing
/// the raw wire text instead double-encodes it — `a%20b` re-submits as
/// `a%2520b`, `quantum+cat` as `quantum%2Bcat` (a literal `+`). Decoding here
/// makes the GET round trip an identity on the pair's meaning. (Maud escapes
/// the decoded text for the attribute context, so a decoded `"`/`<` is safe.)
///
/// `pub` — shared by `search_bar` (and `facet_panel`'s query chip,
/// via it) and `resource_list::render_order_control`.
pub fn hidden_pairs_dropping(current_query: Option<&str>, drop: &[&str]) -> Vec<(String, String)> {
    current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (k, v) = match pair.find('=') {
                Some(idx) => (&pair[..idx], &pair[idx + 1..]),
                None => (pair, ""),
            };
            let key = decode_query_value(k);
            (!drop.contains(&key.as_str())).then(|| (key, decode_query_value(v)))
        })
        .collect()
}

/// Extract and decode the `q` value from a raw query string — the engine's
/// single canonical "read the search query" accessor, mirroring `sort_param`
/// (`component::resource_list`)'s host-agnostic split-on-`&`/`=` pattern but
/// additionally decoding via [`decode_query_value`] (search's `q` is the
/// first free-text query value the engine reads — see that fn's doc
/// comment). Every call site that needs the live `q` value — the search row
/// itself, the query chip, the no-match message, `SearchSurface::for_query`'s
/// activity test — MUST go through this fn, never re-split `current_query`
/// ad hoc, so decoding can never be forgotten at one call site and not
/// another.
///
/// Returns `""` (never panics) when `q` is absent or has an empty value.
pub fn search_query_param(current_query: Option<&str>) -> String {
    let raw = current_query.unwrap_or("").split('&').find_map(|pair| {
        let (k, v) = match pair.find('=') {
            Some(idx) => (&pair[..idx], &pair[idx + 1..]),
            None => (pair, ""),
        };
        (k == "q" && !v.is_empty()).then_some(v)
    });
    match raw {
        Some(v) => decode_query_value(v),
        None => String::new(),
    }
}

/// Uppercase the first character of `s`, Unicode-aware, leaving the rest of
/// the string untouched. Used to turn a declared-lowercase EN sentence-copy
/// plural noun (e.g. Alice's `plural { en "characters" }`) into a
/// title/breadcrumb-appropriate capitalized form (`"Characters"`) without
/// mutating the underlying declared string.
///
/// Uses `char::to_uppercase` (not `to_ascii_uppercase`) on the first `char`
/// so a non-ASCII first character (e.g. a leading accented letter) is
/// correctly capitalized too. `to_uppercase` can expand to more than one
/// `char` for some scripts (e.g. German `ß` → `SS`); those extra chars are
/// preserved via `.collect::<String>()`, not truncated.
///
/// Returns an empty string unchanged.
pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Substitute `value` for every `{n}` placeholder in a project-authored
/// count template (a `resource-list` composed-gloss `unit=`, an
/// `incoming-links note-label=`). As-is: no plural handling (write the
/// templates plural-neutral). Boot validation guarantees both template kinds
/// contain `{n}` (`GlossUnitMissingPlaceholder`,
/// `IncomingLinksTextMissingPlaceholder`); a template without one renders
/// unchanged.
pub fn substitute_count(template: &str, value: &str) -> String {
    template.replace("{n}", value)
}

// ── JS-context encoders (DataStar `@get` arguments, JS string literals) ───────

/// Escape an arbitrary string for safe interpolation inside a **single-quoted
/// JS string literal** embedded in an HTML attribute (e.g.
/// `document.getElementById('{id}').zoomIn()`) — the opening/closing `'`s
/// are the CALLER's, this returns only the escaped content.
///
/// A full allow-list: `\` and `'` are backslash-escaped, and every other
/// character outside `[A-Za-z0-9_-]` is emitted as a `\uXXXX` escape (via
/// `char::encode_utf16`, so a character outside the BMP correctly becomes a
/// surrogate pair of two `\uXXXX` escapes) — safe
/// regardless of the input's content, not conditional on an assumed id
/// shape: the allow-list, not a `debug_assert!` (a no-op in a release
/// build), is what keeps a resource id from breaking out of these literals.
pub fn escape_js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out
}

/// Escape a string so it can be embedded as a single-quoted JS string
/// literal inside a `data-text`/`data-on`/`data-attr` expression — or
/// inside a literal `<script>…</script>` tag emitted by an `ExecuteScript` SSE
/// patch (the `datastar` crate's `execute_script.rs` wraps the script that
/// way). It serves the breadcrumb's localized chrome strings, the
/// record-surface reactive `href` array of `dsp-incubator cpe/web` and the
/// `_compound` fragment handler's pushState/title `ExecuteScript`. `pub`
/// (re-exported from `dsp-incubator cpe/engine`) because that handler
/// is built in the binary, a separate crate.
///
/// Escapes, in order: `\` and `'` (breaking out of the single-quoted JS
/// string literal itself); `<` as `\x3C` (defeating any `</script>`
/// substring that would otherwise break out of the surrounding `<script>`
/// tag — titles are free-form prose, so this is not merely defensive); and
/// the four JS line-terminator characters (`\n`, `\r`, U+2028 LINE
/// SEPARATOR, U+2029 PARAGRAPH SEPARATOR), each of which terminates a
/// single-quoted JS string literal on its own, independent of quote
/// escaping.
pub fn js_string_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '<' => out.push_str("\\x3C"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

// ── Query-component percent-encoding ──────────────────────────────────────────

/// The characters percent-encoded when composing a query key/value:
/// [`NON_ALPHANUMERIC`] minus RFC 3986's unreserved punctuation (`-`, `_`,
/// `.`, `~`) — the same carve-out `encodeURIComponent` makes for those four
/// characters, kept literal so a copied link stays legible (e.g.
/// `role=R_01`, not `role=R%5F01`). One fixed encode set applied
/// identically to every key and value everywhere is what makes two
/// decoded-then-re-encoded inputs byte-identical; the crate must never
/// grow a second, differently-tuned encode set.
const QUERY_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');

/// Percent-encode one query-string component (a key or a value) via
/// `QUERY_ENCODE_SET`.
///
/// Shared by `share_link`'s canonical-query composer and `component::links`
/// (the `?page=<member-id>` value, a resource id in a route's path segment
/// and an annotation entry's fragment), so all encode with the SAME set —
/// one encode set crate-wide, never a second, differently-tuned one.
pub fn percent_encode_component(s: &str) -> String {
    utf8_percent_encode(s, QUERY_ENCODE_SET).to_string()
}

// ── Free-text search query tokenization (FTS5) ────────────────────────────────

/// Tokenize a raw free-text search query into an FTS5 `MATCH` expression:
/// prefix-match every surviving whitespace-separated token, quoted to
/// neutralize FTS5 query-syntax characters.
///
/// - Splits `q` on whitespace.
/// - Drops any token with no alphanumeric character at all (the proxy for "the `unicode61`
///   tokenizer would strip this token to nothing" — a token like `"("`, `":"`, `"-"`, `"*"`, or a
///   lone `"` has zero alphanumeric content and would otherwise become an empty/malformed phrase).
/// - Doubles every embedded `"` in a surviving token (FTS5 quoted-phrase escaping), wraps it in
///   `"…"`, and appends `*` for a prefix match.
/// - Joins surviving quoted terms with a single space.
///
/// Returns `None` when zero terms survive tokenization — this covers the
/// empty string, whitespace-only input, and punctuation-only input, and is
/// the caller's ("no usable search terms") signal to emit no `MATCH` clause
/// at all rather than a malformed one. This matters at *run time*, not just
/// adversarially: the live search fires on every debounced keystroke, so
/// normal typing (a leading space, an opening paren) transiently sends a
/// query that must not raise a MATCH syntax error against the public
/// `--allow-unauthenticated` service.
///
/// Deliberately does NOT special-case FTS5 boolean keywords (`AND`/`OR`/
/// `NOT`) as terms to drop — quoting every term (making `"and"*` a literal
/// phrase, not a bare operator) is what neutralizes them; excluding them
/// via a stopword list is unnecessary and not implemented here.
///
/// Deliberately does NOT lowercase/normalize case — SQLite's `unicode61`
/// tokenizer folds case at match time, so passing tokens through with their
/// original case is correct.
pub fn fts_match_expr(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split_whitespace()
        .filter(|tok| tok.chars().any(|c| c.is_alphanumeric()))
        .map(|tok| format!("\"{}\"*", tok.replace('"', "\"\"")))
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// Return `true` when a raw `?q=` value has at least one usable search term
/// after tokenization — the single canonical "is this search active"
/// predicate, delegating to `fts_match_expr` rather than duplicating its
/// tokenizer logic.
///
/// This is meant to be shared by **both** layers that need to answer "is
/// there a real query here": the store's page builder (`query::page`,
/// deciding whether to push a `WhereClause::Search` and compute the
/// unfiltered "N of M" count) and the engine's own renderer construction
/// (`SearchSurface::for_query`, deciding whether to show a no-match
/// state). Neither call site should re-derive "active" from raw-string
/// non-emptiness (`!q.is_empty()`) — a whitespace-/punctuation-only `q` must
/// be treated as inactive by both, and routing both through this one
/// function is what keeps that definition from drifting between them.
pub fn is_active_search_query(q: &str) -> bool {
    fts_match_expr(q).is_some()
}

// ── URI safety ────────────────────────────────────────────────────────────────

/// Check whether a `Text` value's content is a safe external URL.
///
/// Allowlist: `https://`, `http://`, or a single-`/`-rooted relative path
/// (never `//` — protocol-relative — nor `/\` — a browser reads a
/// backslash right after a leading slash as another `/`, so `/\evil.example`
/// is protocol-relative in disguise). Blocklist: any value containing an
/// ASCII control character (checked first — the WHATWG URL parser strips
/// tab/CR/LF, so e.g. `/\t/evil.example` would otherwise pass this check and
/// then be followed as `//evil.example`), plus `javascript:`, `data:`,
/// `mailto:`, `#anchor`, and anything else not explicitly allow-listed.
///
/// The one guard for every URI the engine emits as a link or image source:
/// `resource_detail`'s plain-text URL values, `resource_list` and `media`
/// (image URIs), `markdown` (richtext link/image destinations), the showcase
/// components and their validator. Reuse it; never duplicate the allow-list.
pub fn is_safe_external_url(text: &str) -> bool {
    // Reject any control character first, for every arm below.
    if text.bytes().any(|b| b.is_ascii_control()) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    // Reject dangerous schemes first.
    if lower.starts_with("javascript:") || lower.starts_with("data:") {
        return false;
    }
    // Allow known-safe prefixes.
    text.starts_with("https://")
        || text.starts_with("http://")
        || (text.starts_with('/') && !text.starts_with("//") && !text.starts_with("/\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── escape_js_string_literal ────────────────────────────────────────────────

    #[test]
    fn escape_js_string_literal_passes_through_safe_id_chars() {
        assert_eq!(escape_js_string_literal("rv-CH_026"), "rv-CH_026");
    }

    #[test]
    fn escape_js_string_literal_escapes_single_quote_and_backslash() {
        // A hostile/malformed id containing both a single quote (would close
        // the JS string literal early) and a backslash (would escape the
        // next character) — must never appear unescaped in the output.
        let out = escape_js_string_literal(r"rv-a'b\c");
        assert_eq!(out, r"rv-a\'b\\c");
        // The single `'` in the output is legitimately part of the `\'`
        // escape sequence — assert it's ALWAYS backslash-prefixed (never a
        // bare, string-closing quote) rather than asserting no `'` at all.
        assert_eq!(
            out.matches('\'').count(),
            out.matches("\\'").count(),
            "every ' in the output must be part of a \\' escape, got: {out}"
        );
    }

    #[test]
    fn escape_js_string_literal_escapes_other_special_chars_as_unicode_escapes() {
        assert_eq!(escape_js_string_literal("a<b>c"), "a\\u003cb\\u003ec");
        assert_eq!(escape_js_string_literal("a`b"), "a\\u0060b");
    }

    // ── js_string_escape ────────────────────────────────────────────────────

    #[test]
    fn js_string_escape_escapes_quote_and_backslash() {
        assert_eq!(js_string_escape(r"a'b\c"), r"a\'b\\c");
    }

    /// A title/URL sitting inside a literal `<script>` tag must never let a
    /// `</script>` substring through.
    #[test]
    fn js_string_escape_escapes_angle_bracket() {
        assert_eq!(
            js_string_escape("</script><script>alert(1)</script>"),
            r"\x3C/script>\x3Cscript>alert(1)\x3C/script>"
        );
    }

    /// Each JS line-terminator character independently ends a single-quoted
    /// string literal, regardless of quote-escaping — all four must be
    /// escaped.
    #[test]
    fn js_string_escape_escapes_line_terminators() {
        assert_eq!(js_string_escape("a\nb\rc\u{2028}d\u{2029}e"), "a\\nb\\rc\\u2028d\\u2029e");
    }

    #[test]
    fn js_string_escape_leaves_plain_text_untouched() {
        assert_eq!(
            js_string_escape("Alice's Adventures in Wonderland"),
            r"Alice\'s Adventures in Wonderland"
        );
    }

    // ── percent_encode_component ────────────────────────────────────────────────

    #[test]
    fn percent_encode_component_is_stable_across_calls() {
        let a = percent_encode_component("hat & cat");
        let b = percent_encode_component("hat & cat");
        assert_eq!(a, b);
        assert_eq!(a, "hat%20%26%20cat");
    }

    #[test]
    fn percent_encode_component_leaves_unreserved_chars_untouched() {
        // Alphanumerics, `-`, `_`, `.`, `~` (RFC 3986 unreserved) pass through.
        assert_eq!(percent_encode_component("R_01"), "R_01");
        assert_eq!(percent_encode_component("abc123"), "abc123");
        assert_eq!(percent_encode_component("a-b.c~d"), "a-b.c~d");
    }

    #[test]
    fn percent_encode_component_encodes_every_other_byte_as_uppercase_hex() {
        assert_eq!(percent_encode_component("a b/é~#?&"), "a%20b%2F%C3%A9~%23%3F%26");
    }

    // ── fts_match_expr ──────────────────────────────────────────────────────────

    #[test]
    fn fts_match_expr_single_term() {
        assert_eq!(fts_match_expr("rab"), Some("\"rab\"*".to_owned()));
    }

    #[test]
    fn fts_match_expr_multi_term_with_spaces() {
        assert_eq!(fts_match_expr("quantum cat"), Some("\"quantum\"* \"cat\"*".to_owned()));
    }

    #[test]
    fn fts_match_expr_doubles_embedded_quote() {
        assert_eq!(fts_match_expr(r#"ra"b"#), Some("\"ra\"\"b\"*".to_owned()));
    }

    #[test]
    fn fts_match_expr_empty_after_tokenize_corpus_returns_none() {
        for q in ["", " ", "\"", "(", ":", "-", "*"] {
            assert_eq!(fts_match_expr(q), None, "expected None for empty-after-tokenize input {q:?}");
        }
    }

    /// "AND" is a normal alphabetic token — it survives the alphanumeric
    /// filter like any real word and is quoted, NOT dropped as a stopword.
    /// Quoting (not exclusion) is what neutralizes FTS5's boolean keyword.
    #[test]
    fn fts_match_expr_bare_and_is_not_none() {
        let result = fts_match_expr("AND");
        assert!(result.is_some(), "bare word AND must survive as a quoted term, not become None");
        let result = result.expect("checked above");
        assert!(
            result == "\"AND\"*" || result == "\"and\"*",
            "expected a quoted 'and'-shaped term (case-preserving), got {result:?}"
        );
    }

    // ── is_safe_external_url ────────────────────────────────────────────────────

    #[test]
    fn is_safe_external_url_rejects_protocol_relative() {
        assert!(!is_safe_external_url("//evil.example"));
        assert!(!is_safe_external_url("/\\evil.example"));
        assert!(!is_safe_external_url("/\t/evil.example"));
        assert!(!is_safe_external_url("/\n/evil.example"));
        assert!(!is_safe_external_url("https://a\tb"));

        assert!(is_safe_external_url("/p/x"));
        assert!(is_safe_external_url("https://a"));
        assert!(is_safe_external_url("http://a"));
    }

    #[test]
    fn substitute_count_replaces_every_placeholder() {
        assert_eq!(substitute_count("{n} Seiten", "511"), "511 Seiten");
        assert_eq!(substitute_count("{n} / {n}", "3"), "3 / 3");
        assert_eq!(substitute_count("no placeholder", "3"), "no placeholder");
    }

    // ── first_heading tests ───────────────────────────────────────────────────

    #[test]
    fn first_heading_h1_present() {
        assert_eq!(first_heading("# Hello World\n\nSome text."), Some("Hello World".to_owned()));
    }

    #[test]
    fn first_heading_uses_first_heading_when_h2_comes_before_h1() {
        // Even if h2 comes before h1, the first heading in document order wins.
        assert_eq!(first_heading("## Section\n\n# Title"), Some("Section".to_owned()));
    }

    #[test]
    fn first_heading_h3_only() {
        assert_eq!(first_heading("### Deep\n\nSome text."), Some("Deep".to_owned()));
    }

    #[test]
    fn first_heading_no_heading_returns_none() {
        assert_eq!(first_heading("Just some prose.\n\nAnother paragraph."), None);
    }

    #[test]
    fn first_heading_inline_code_in_heading() {
        // `# The \`translate.py\` workflow` → "The translate.py workflow"
        assert_eq!(
            first_heading("# The `translate.py` workflow\n\nBody."),
            Some("The translate.py workflow".to_owned())
        );
    }

    #[test]
    fn first_heading_emphasis_in_heading() {
        // Emphasis is a container tag — the text inside it is collected via Text events.
        assert_eq!(first_heading("# Hello *World*\n\nBody."), Some("Hello World".to_owned()));
    }

    #[test]
    fn first_heading_trims_whitespace() {
        // ATX headings don't normally produce leading/trailing spaces, but verify trim.
        assert_eq!(first_heading("#   Trimmed   \n\nBody."), Some("Trimmed".to_owned()));
    }

    #[test]
    fn first_heading_empty_heading_returns_none() {
        // An ATX heading whose content is only whitespace trims to empty → None.
        // "# " followed by two spaces and a newline: pulldown-cmark emits a
        // heading Start/End pair with no Text events between them, so `buf`
        // stays empty, trims to "", and the function returns None.
        assert_eq!(first_heading("#  \n\nbody"), None);
    }

    // ── narrative_contains_h1 tests ────────────────────────────────────────────

    #[test]
    fn narrative_contains_h1_true_for_leading_h1() {
        assert!(narrative_contains_h1("# Title\n\nBody."));
    }

    /// The h1-injection-decision case: an h1 that is NOT first
    /// (an h2 precedes it) still counts as "this narrative has a visible
    /// h1" — checking only the first heading's level would (wrongly, for
    /// this decision) report `false`.
    #[test]
    fn narrative_contains_h1_true_when_h1_comes_after_h2() {
        assert!(narrative_contains_h1("## intro\n\n# title"));
    }

    #[test]
    fn narrative_contains_h1_false_when_no_h1_anywhere() {
        assert!(!narrative_contains_h1("## Section\n\n### Deep\n\nNo h1 in this document."));
    }

    #[test]
    fn narrative_contains_h1_false_for_no_heading() {
        assert!(!narrative_contains_h1("Just some prose."));
    }

    // ── page_title_heading tests ──────────────────────────────────────────────

    /// The Incunabula shape: a leading `###` kicker (an eyebrow *label*,
    /// rendered by `component::editorial` as `p.eyebrow.editorial__kicker`)
    /// above the real `# ` title. The H1 wins — `first_heading` would
    /// return the kicker.
    #[test]
    fn page_title_heading_prefers_h1_over_leading_kicker() {
        let md = "### Showcase I\n\n# The Narrenschiff\n\nBody.";
        assert_eq!(page_title_heading(md), Some("The Narrenschiff".to_owned()));
        // Contrast with the level-blind helper, which picks the kicker.
        assert_eq!(first_heading(md), Some("Showcase I".to_owned()));
    }

    /// An H1 that is not the first heading still wins (same rule, without
    /// the specific kicker shape).
    #[test]
    fn page_title_heading_prefers_h1_after_h2() {
        assert_eq!(page_title_heading("## Section\n\n# Title"), Some("Title".to_owned()));
    }

    #[test]
    fn page_title_heading_h1_first_is_unchanged() {
        assert_eq!(
            page_title_heading("# Hello World\n\nSome text."),
            Some("Hello World".to_owned())
        );
    }

    /// No H1 anywhere → fall back to `first_heading`'s answer (the first
    /// heading of any level), not `None`.
    #[test]
    fn page_title_heading_falls_back_to_first_heading_without_h1() {
        assert_eq!(page_title_heading("### Deep\n\n## Section\n\nBody."), Some("Deep".to_owned()));
    }

    /// The `### only` case — a single level-3 heading and nothing else.
    #[test]
    fn page_title_heading_h3_only() {
        assert_eq!(page_title_heading("### Deep\n\nSome text."), Some("Deep".to_owned()));
    }

    /// A heading id is markup, never title text.
    #[test]
    fn heading_attribute_never_reaches_a_derived_title() {
        let md = "# The Narrenschiff {#top}\n\n## Strips {#strips}";
        assert_eq!(page_title_heading(md), Some("The Narrenschiff".to_owned()));
        assert_eq!(first_heading("## Strips {#strips}"), Some("Strips".to_owned()));
    }

    #[test]
    fn heading_ids_reports_each_id_with_its_level_and_keeps_duplicates() {
        let md = "# Title {#top}\n\n## Four {#four}\n\n## Plain\n\n### Deep {#deep}\n\n## Again {#four}";
        assert_eq!(
            heading_ids(md),
            vec![
                (HeadingLevel::H1, "top".to_owned()),
                (HeadingLevel::H2, "four".to_owned()),
                (HeadingLevel::H3, "deep".to_owned()),
                (HeadingLevel::H2, "four".to_owned()),
            ]
        );
        assert!(heading_ids("## Plain\n\nNo ids.").is_empty());
    }

    #[test]
    fn page_title_heading_no_heading_returns_none() {
        assert_eq!(page_title_heading("Just some prose."), None);
    }

    /// An empty H1 is not a usable title — fall through to the first
    /// heading's text (here the kicker), mirroring `first_heading`'s
    /// "empty means no heading" rule.
    #[test]
    fn page_title_heading_empty_h1_falls_back() {
        assert_eq!(page_title_heading("### Kicker\n\n#  \n\nBody."), Some("Kicker".to_owned()));
    }

    // ── markdown_to_text tests ─────────────────────────────────────────────────

    #[test]
    fn markdown_to_text_strips_headings_and_emphasis() {
        // The heading and the following paragraph are separate blocks — a
        // separating space is inserted between "Title" and "Some" so the two
        // don't fuse into one un-matchable FTS token ("TitleSome…").
        assert_eq!(
            markdown_to_text("# Title\n\nSome *emphasized* text."),
            "Title Some emphasized text."
        );
    }

    #[test]
    fn markdown_to_text_strips_links_to_visible_text() {
        assert_eq!(
            markdown_to_text("See [Wonderland](https://example.com/wonderland)."),
            "See Wonderland."
        );
    }

    #[test]
    fn markdown_to_text_drops_embedded_html() {
        // A block-level HTML tag starting its own line (blank-line-preceded)
        // is a CommonMark HTML block: pulldown-cmark emits its entire raw
        // content (including the enclosed text) as ONE `Event::Html`, which
        // must be dropped wholesale — not just the tags. "Before" and "After"
        // are each their own paragraph block, so the block-boundary separator
        // still applies between them even though the dropped HTML block sits
        // in between contributing no text of its own.
        assert_eq!(markdown_to_text("Before\n\n<div>evil content</div>\n\nAfter"), "Before After");
    }

    #[test]
    fn markdown_to_text_empty_input_returns_empty_string() {
        assert_eq!(markdown_to_text(""), "");
    }

    #[test]
    fn markdown_to_text_preserves_inline_code() {
        assert_eq!(markdown_to_text("Run `cargo test` now."), "Run cargo test now.");
    }

    // ── split_stem_ext tests ──────────────────────────────────────────────────

    #[test]
    fn simple_extension() {
        assert_eq!(split_stem_ext("home.md"), ("home".to_owned(), "md".to_owned()));
    }

    #[test]
    fn no_dot_returns_empty_ext() {
        assert_eq!(split_stem_ext("readme"), ("readme".to_owned(), String::new()));
    }

    #[test]
    fn multi_dot_splits_at_last_dot() {
        // "notes.intro.md" → stem = "notes.intro", ext = "md"
        assert_eq!(split_stem_ext("notes.intro.md"), ("notes.intro".to_owned(), "md".to_owned()));
    }

    #[test]
    fn lang_variant_file_splits_at_last_dot() {
        // "home.de.md" → stem = "home.de", ext = "md"
        assert_eq!(split_stem_ext("home.de.md"), ("home.de".to_owned(), "md".to_owned()));
    }

    #[test]
    fn empty_string_returns_both_empty() {
        assert_eq!(split_stem_ext(""), (String::new(), String::new()));
    }

    // ── variant_filename tests ────────────────────────────────────────────────

    #[test]
    fn variant_filename_simple_extension() {
        assert_eq!(variant_filename("home.md", "de"), Some("home.de.md".to_owned()));
    }

    #[test]
    fn variant_filename_multi_dot_stem() {
        // "notes.intro.md" + "de" → stem = "notes.intro", ext = "md"
        assert_eq!(variant_filename("notes.intro.md", "de"), Some("notes.intro.de.md".to_owned()));
    }

    #[test]
    fn variant_filename_no_extension_returns_none() {
        assert_eq!(variant_filename("readme", "de"), None);
    }

    // ── decode_query_value / search_query_param tests ─────────────────────────

    #[test]
    fn decode_query_value_percent_decodes_space() {
        assert_eq!(decode_query_value("quantum%20cat"), "quantum cat");
    }

    #[test]
    fn decode_query_value_plus_decodes_space() {
        assert_eq!(decode_query_value("quantum+cat"), "quantum cat");
    }

    #[test]
    fn decode_query_value_passthrough_for_plain_ascii() {
        assert_eq!(decode_query_value("rab"), "rab");
    }

    // ── hidden_pairs_dropping tests ───────────────────────────────────────────

    fn owned(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    #[test]
    fn hidden_pairs_dropping_decodes_value_once() {
        assert_eq!(hidden_pairs_dropping(Some("x=a%20b"), &[]), owned(&[("x", "a b")]));
        assert_eq!(hidden_pairs_dropping(Some("x=a+b"), &[]), owned(&[("x", "a b")]));
        // Decoded exactly once: `%2520` is a literal `%20`, not a space.
        assert_eq!(hidden_pairs_dropping(Some("x=a%2520b"), &[]), owned(&[("x", "a%20b")]));
    }

    #[test]
    fn hidden_pairs_dropping_drops_listed_keys_and_keeps_repeats_in_order() {
        let pairs = hidden_pairs_dropping(Some("role=R_01&page=2&q=cat&role=R_02&flag&&lang=de"), &["q", "page"]);
        assert_eq!(
            pairs,
            owned(&[("role", "R_01"), ("role", "R_02"), ("flag", ""), ("lang", "de")])
        );
    }

    #[test]
    fn hidden_pairs_dropping_matches_drop_list_on_decoded_key() {
        // An encoded key (`p%61ge` == `page`) is still dropped; a UTF-8 key decodes.
        let pairs = hidden_pairs_dropping(Some("p%61ge=3&k%C3%A9y=caf%C3%A9"), &["page"]);
        assert_eq!(pairs, owned(&[("kéy", "café")]));
    }

    #[test]
    fn hidden_pairs_dropping_none_or_empty_query_is_empty() {
        assert!(hidden_pairs_dropping(None, &["q"]).is_empty());
        assert!(hidden_pairs_dropping(Some(""), &["q"]).is_empty());
    }

    /// Facet-value round trip: decoding then the browser's
    /// `application/x-www-form-urlencoded` re-encoding of the hidden input
    /// reproduces the SAME meaning (decodes back to the identical value).
    #[test]
    fn hidden_pairs_dropping_facet_value_round_trips_through_form_encoding() {
        let wire = "role=R%2001&kw=caf%C3%A9+noir";
        for (k, v) in hidden_pairs_dropping(Some(wire), &[]) {
            let resubmitted: String =
                percent_encoding::utf8_percent_encode(&v, percent_encoding::NON_ALPHANUMERIC).to_string();
            let original = wire.split('&').find_map(|p| p.strip_prefix(&format!("{k}="))).unwrap();
            assert_eq!(decode_query_value(&resubmitted), decode_query_value(original));
        }
    }

    #[test]
    fn search_query_param_extracts_and_decodes_q() {
        assert_eq!(search_query_param(Some("q=quantum%20cat&page=2")), "quantum cat");
        assert_eq!(search_query_param(Some("q=quantum+cat")), "quantum cat");
    }

    #[test]
    fn search_query_param_absent_or_empty_returns_empty_string() {
        assert_eq!(search_query_param(None), "");
        assert_eq!(search_query_param(Some("page=2")), "");
        assert_eq!(search_query_param(Some("q=")), "");
    }

    // ── capitalize_first tests ─────────────────────────────────────────────────

    #[test]
    fn capitalize_first_ascii() {
        assert_eq!(capitalize_first("characters"), "Characters");
    }

    #[test]
    fn capitalize_first_already_capitalized() {
        assert_eq!(capitalize_first("Figuren"), "Figuren");
    }

    #[test]
    fn capitalize_first_empty_string() {
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn capitalize_first_non_ascii_first_char() {
        // A leading accented letter is capitalized too (Unicode-aware, not
        // ASCII-only).
        assert_eq!(capitalize_first("émotions"), "Émotions");
    }
}
