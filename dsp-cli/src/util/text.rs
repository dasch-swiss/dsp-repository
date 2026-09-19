//! Text helpers — HTML-to-plain-text conversion and control-character stripping.
//!
//! These helpers are layer-neutral: they have no dependencies on any other
//! `dsp-cli` module and may be imported from any layer (client, render, …).
//! They live here (not in `src/render/`) so the client layer can use
//! `html_to_text` for standoff XML stripping without creating a
//! client → render dependency (ADR-0008).
//!
//! `html_to_text` converts server-supplied HTML description text into a
//! readable plain-text form suitable for terminal output. It is intentionally
//! dep-free: no external HTML parser, no regex — just a plain char scanner.
//!
//! **Behaviour contract** (see tests below):
//! - `<br>`, `<br/>`, `<br />` (case-insensitive) → newline.
//! - `<p>` and `</p>` → newline (block boundary).
//! - `<a href="URL" …>TEXT</a>` → `TEXT (URL)`; if no href, just `TEXT`.
//! - All other tags (`<b>`, `</b>`, `<i>`, `<strong>`, `<em>`, `<ul>`,
//!   `<li>`, unknown) → stripped, inner text kept.
//! - Common HTML entities unescaped: `&amp;` `&lt;` `&gt;` `&quot;`
//!   `&#39;`/`&apos;` `&nbsp;`.
//! - Control chars **except** `\n` and `\t` stripped: the C0 range
//!   (0x00–0x1F), DEL (0x7F) **and the C1 range** (0x80–0x9F). Closes the
//!   terminal/ANSI-injection concern for server-supplied text — C1 matters
//!   because U+009B is the single-character form of `ESC [` (CSI), so an
//!   ASCII-only filter would let a terminal control sequence through.
//! - 3+ consecutive newlines collapsed to exactly 2 (at most one blank line).
//! - Leading/trailing whitespace trimmed from the final result.

/// Convert an HTML string to plain text.
///
/// Dep-free char scanner. Never panics. Returns `String`.
pub(crate) fn html_to_text(s: &str) -> String {
    let after_tags = strip_tags(s);
    let unescaped = unescape_entities(&after_tags);
    let sanitised = strip_control_chars(&unescaped);
    let collapsed = collapse_newlines(&sanitised);
    collapsed.trim().to_string()
}

/// Strip every control character except `\n` (0x0A) and `\t` (0x09): the C0
/// range (0x00–0x1F), DEL (0x7F), and the C1 range (0x80–0x9F). Removes ANSI
/// escape sequences and other terminal-control characters.
///
/// Uses `char::is_control()` (Unicode category `Cc`) rather than
/// `is_ascii_control()`: **U+009B is the single-character form of `ESC [`**
/// (CSI), so an ASCII-only filter leaves a working terminal control
/// introducer in server-supplied text. Found by review, 2026-08-07.
///
/// **Keeps** `\n`/`\t` because it is the *prose* sanitiser: prose values flow
/// freely across lines, where a newline or tab can be legitimate content. For
/// the *tabular* formats (lines/csv/tsv), where `\n`/`\t` break the
/// one-record-per-line / delimited structure, use [`replace_control_chars`].
pub(crate) fn strip_control_chars(s: &str) -> String {
    s.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
        .collect()
}

/// Replace **every** control character — the C0 range (`\x00`–`\x1f`, which
/// includes `\t`, `\n`, `\r`, NUL, ESC, …), DEL (`\x7f`), and the C1 range
/// (`\u{80}`–`\u{9f}`) — with a single space. Uses `char::is_control()`
/// (Unicode category `Cc`); C1 is included for the same reason as in
/// [`strip_control_chars`] (U+009B is CSI). Printable text and all other
/// multibyte Unicode pass through untouched.
///
/// This is the *tabular* sanitiser (lines/csv/tsv cells): it replaces rather
/// than strips (preserving field count / column-width alignment) and, unlike
/// the prose sibling [`strip_control_chars`], it also neutralises `\n`/`\t`
/// because those break the one-record-per-line / delimited structure. One
/// predicate covers both pipeline-breaking (newline, tab) and terminal-control
/// (ESC, DEL) characters without a fragile deny-list. See plan 020 D11 and
/// ADR-0003.
pub(crate) fn replace_control_chars(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Sanitise store/server-authored prose for a diagnostic message: strip
/// control characters (via [`strip_control_chars`], which keeps `\n`/`\t`)
/// and cap to 200 characters, appending `…` if the sanitised text was longer.
///
/// Shared by `src/client/http.rs`'s `classify_sparql_status` (the `500`/`502`/
/// `503`/`504` raw-body fallback) and the `dsp vre sparql query` action's
/// relay path (D7's stderr prose path) — see plan 035 D7's ⚠️ note: it must be
/// exactly one function so both paths sanitise identically: a store error body
/// can arrive under any status, including the `500` the classifier's fallback
/// handles, so sanitising only one path leaves the hole wherever the status
/// happens to land. (An earlier note here claimed Fuseki answers ARQ
/// *evaluation* errors with `500`; live-verified 2026-08-07, it does not — it
/// returns `200` with an unbound result. The rule stands on the general
/// argument.)
///
/// 200 chars (not `strip_control_chars`'s other caller-sites' 80) because ARQ
/// puts line, column and the offending token in the first ~100 characters.
///
/// Counts by `char`, not by byte length — the input is lossy-decoded
/// multibyte text, and a byte-index cap (`&s[..200]`) can panic on a
/// non-boundary split.
/// [`sanitise_and_cap`] for a raw response body.
///
/// Slices to 4 KiB **before** decoding and stripping. `sanitise_and_cap` walks
/// its whole input before taking the first 200 characters, so handing it a
/// 64 MiB store error would materialise two full-size copies to print 200
/// characters — and ADR-0016 deliberately accepts that there is no client-side
/// response ceiling, which makes a multi-MiB body an in-design input rather
/// than a pathological one. 200 sanitised chars are at most ~800 bytes, so the
/// slice can never truncate anything that would have been printed.
///
/// Slicing `&[u8]` at an arbitrary index cannot panic, and `from_utf8_lossy`
/// turns a split multibyte char into a single U+FFFD.
///
/// Use this at **every** site that renders a response body as prose (the
/// classifier's fallback, the action's relay path, the trace preview) so D7's
/// "identical sanitisation on both paths" is literally true.
pub(crate) fn sanitise_bytes_for_prose(body: &[u8]) -> String {
    let head = &body[..body.len().min(4096)];
    sanitise_and_cap(&String::from_utf8_lossy(head))
}

pub(crate) fn sanitise_and_cap(s: &str) -> String {
    let sanitised = strip_control_chars(s);
    let char_count = sanitised.chars().count();
    if char_count > 200 {
        let mut capped: String = sanitised.chars().take(200).collect();
        capped.push('…');
        capped
    } else {
        sanitised
    }
}

// ── tag scanner ───────────────────────────────────────────────────────────────

/// Walk the input char by char, handling tags. Returns a string with tags
/// replaced by their plain-text equivalents.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Find the end of this tag.
            let tag_start = i;
            i += 1; // skip '<'
            let mut j = i;
            // Tags may contain quoted attributes — scan past quoted strings so
            // a '>' inside href="…" doesn't terminate the tag prematurely.
            let mut in_quote: Option<u8> = None;
            while j < len {
                let b = bytes[j];
                match in_quote {
                    Some(q) if b == q => {
                        in_quote = None;
                        j += 1;
                    }
                    Some(_) => {
                        j += 1;
                    }
                    None if b == b'"' || b == b'\'' => {
                        in_quote = Some(b);
                        j += 1;
                    }
                    None if b == b'>' => {
                        j += 1; // include the '>'
                        break;
                    }
                    None => {
                        j += 1;
                    }
                }
            }
            // `tag_start..j` is the complete `<…>` span (or an unclosed '<' if
            // we hit EOF without finding '>').
            let raw_tag = &s[tag_start..j];
            // The inner content (between '<' and '>'), without surrounding angle brackets.
            let inner = if raw_tag.starts_with('<') && raw_tag.ends_with('>') {
                &raw_tag[1..raw_tag.len() - 1]
            } else {
                // Malformed / unclosed tag — emit literally and keep scanning.
                out.push_str(raw_tag);
                i = j;
                continue;
            };

            let tag_name_lower = tag_name_of(inner).to_ascii_lowercase();

            match tag_name_lower.as_str() {
                "br" => out.push('\n'),
                "p" => out.push('\n'),
                "/p" => out.push('\n'),
                "a" => {
                    // Anchor open: extract href, collect inner text up to </a>.
                    let href = extract_href(inner);
                    // Collect the text content up to </a>.
                    let (text, consumed) = collect_until_close_tag(&s[j..], "a");
                    // Unescape the inner text recursively (it may contain tags too,
                    // though that is unusual in DSP descriptions).
                    let plain_text = strip_tags(text);
                    if plain_text.is_empty() {
                        // Empty anchor text — just emit href if present.
                        if let Some(url) = href {
                            out.push_str(url);
                        }
                    } else {
                        out.push_str(&plain_text);
                        if let Some(url) = href {
                            out.push_str(" (");
                            out.push_str(url);
                            out.push(')');
                        }
                    }
                    i = j + consumed;
                    continue;
                }
                _ => {
                    // All other tags: strip, keep inner content (done by
                    // continuing the scan past the tag boundary).
                }
            }

            i = j;
        } else {
            // Regular character — copy directly, advancing by the char's byte
            // length (`i` is always at a char boundary). `chars().next()` on the
            // non-empty `s[i..]` always yields `Some`; the `None` arm is
            // unreachable but avoids `unwrap()` in non-test code (coding-conventions).
            match s[i..].chars().next() {
                Some(ch) => {
                    out.push(ch);
                    i += ch.len_utf8();
                }
                None => break,
            }
        }
    }

    out
}

/// Extract the tag-name portion from `inner` (the string between `<` and `>`).
///
/// Examples:
/// - `"br /"` → `"br"` (whitespace-separated; trailing `/` in its own token ignored)
/// - `"br/"` → `"br"` (self-closing without space; trailing `/` stripped)
/// - `"a href=\"x\""` → `"a"`
/// - `"/p"` → `"/p"`
///
/// Returns a string with the tag name (first token, without any trailing `/`).
fn tag_name_of(inner: &str) -> String {
    let trimmed = inner.trim();
    // First whitespace-delimited token.
    let token = trimmed.split_whitespace().next().unwrap_or("");
    // Strip a trailing `/` from self-closing tags like `br/`.
    token.trim_end_matches('/').to_string()
}

/// Scan forward in `rest` (the text *after* the `<a …>` tag) and collect
/// everything up to (but not including) the matching `</a>` close tag.
///
/// Returns `(text_slice, bytes_consumed)` where `bytes_consumed` is the number
/// of bytes in `rest` that were consumed (including the `</a>` tag itself).
fn collect_until_close_tag<'a>(rest: &'a str, tag: &str) -> (&'a str, usize) {
    let close_needle = format!("</{tag}");
    let lower = rest.to_ascii_lowercase();
    if let Some(pos) = lower.find(&close_needle) {
        // Find end of the close tag (skip to '>').
        let after_name = pos + close_needle.len();
        let close_end = rest[after_name..]
            .find('>')
            .map(|p| after_name + p + 1)
            .unwrap_or(rest.len());
        (&rest[..pos], close_end)
    } else {
        // No close tag — consume the rest.
        (rest, rest.len())
    }
}

/// Extract the `href` attribute value from the content inside an `<a …>` tag.
///
/// Handles both double-quoted and single-quoted values. Returns `None` if no
/// `href` attribute is present.
fn extract_href(inner: &str) -> Option<&str> {
    // Find "href" (case-insensitive).
    let lower = inner.to_ascii_lowercase();
    let href_pos = lower.find("href")?;
    let after_href = inner[href_pos + 4..].trim_start();
    // Expect `=`.
    let rest = after_href.strip_prefix('=')?;
    let rest = rest.trim_start();
    // Expect a quote.
    let (quote_char, value_start) = if let Some(s) = rest.strip_prefix('"') {
        ('"', s)
    } else if let Some(s) = rest.strip_prefix('\'') {
        ('\'', s)
    } else {
        return None;
    };
    let end = value_start.find(quote_char)?;
    Some(&value_start[..end])
}

// ── entity unescaping ─────────────────────────────────────────────────────────

/// Unescape common HTML entities.
fn unescape_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            // Collect until ';' (or give up after 12 chars — max named entity).
            let mut entity = String::new();
            let mut found_semi = false;
            for _ in 0..12 {
                match chars.peek() {
                    Some(&';') => {
                        chars.next();
                        found_semi = true;
                        break;
                    }
                    Some(_) => {
                        // peek() confirmed Some above — next() always yields Some here.
                        if let Some(c) = chars.next() {
                            entity.push(c);
                        }
                    }
                    None => break,
                }
            }
            if found_semi {
                match entity.as_str() {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" | "#39" => out.push('\''),
                    "nbsp" => out.push(' '),
                    _ => {
                        // Unknown entity — emit verbatim.
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                }
            } else {
                // No semicolon found — emit the '&' and whatever we collected.
                out.push('&');
                out.push_str(&entity);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

// ── blank-line collapsing ─────────────────────────────────────────────────────

/// Collapse 3+ consecutive newlines to exactly 2 (at most one blank line).
fn collapse_newlines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut newline_run: usize = 0;

    for ch in s.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
            // 3+ newlines: suppress the extra ones.
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }

    out
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passthrough() {
        let s = "Hello, world!";
        assert_eq!(html_to_text(s), s);
    }

    #[test]
    fn plain_text_trimmed() {
        assert_eq!(html_to_text("  hello  "), "hello");
        assert_eq!(html_to_text("\nhello\n"), "hello");
    }

    #[test]
    fn br_tag_to_newline() {
        assert_eq!(html_to_text("a<br>b"), "a\nb");
        assert_eq!(html_to_text("a<br/>b"), "a\nb");
        assert_eq!(html_to_text("a<br />b"), "a\nb");
        // Case-insensitive
        assert_eq!(html_to_text("a<BR>b"), "a\nb");
        assert_eq!(html_to_text("a<Br />b"), "a\nb");
    }

    #[test]
    fn p_tag_to_newline() {
        assert_eq!(html_to_text("<p>Hello</p>"), "Hello");
        // Two <p> blocks → separated by a blank line (2 newlines each boundary).
        let result = html_to_text("<p>A</p><p>B</p>");
        assert!(result.contains('A'));
        assert!(result.contains('B'));
    }

    #[test]
    fn anchor_with_href() {
        assert_eq!(
            html_to_text("<a href=\"https://example.com\">Click here</a>"),
            "Click here (https://example.com)"
        );
    }

    #[test]
    fn anchor_with_single_quote_href() {
        assert_eq!(
            html_to_text("<a href='https://example.com'>text</a>"),
            "text (https://example.com)"
        );
    }

    #[test]
    fn anchor_without_href() {
        assert_eq!(html_to_text("<a name=\"top\">Section</a>"), "Section");
    }

    #[test]
    fn anchor_empty_text_with_href() {
        assert_eq!(
            html_to_text("<a href=\"https://example.com\"></a>"),
            "https://example.com"
        );
    }

    #[test]
    fn bold_and_italic_stripped() {
        assert_eq!(html_to_text("<b>bold</b>"), "bold");
        assert_eq!(html_to_text("<i>italic</i>"), "italic");
        assert_eq!(html_to_text("<strong>strong</strong>"), "strong");
        assert_eq!(html_to_text("<em>em</em>"), "em");
    }

    #[test]
    fn unknown_tags_stripped() {
        assert_eq!(html_to_text("<ul><li>item</li></ul>"), "item");
        assert_eq!(html_to_text("<span class=\"x\">text</span>"), "text");
    }

    #[test]
    fn entity_unescaping() {
        assert_eq!(html_to_text("a&amp;b"), "a&b");
        assert_eq!(html_to_text("a&lt;b"), "a<b");
        assert_eq!(html_to_text("a&gt;b"), "a>b");
        assert_eq!(html_to_text("say &quot;hello&quot;"), "say \"hello\"");
        assert_eq!(html_to_text("it&apos;s"), "it's");
        assert_eq!(html_to_text("it&#39;s"), "it's");
        // &nbsp; → space (tested in context so trim doesn't eat it)
        assert_eq!(html_to_text("a&nbsp;b"), "a b");
        // Unknown entity left verbatim
        assert_eq!(html_to_text("a&unknown;b"), "a&unknown;b");
    }

    #[test]
    fn control_char_and_ansi_escape_stripped() {
        // ANSI escape sequence: ESC [ 3 1 m  →  ESC is 0x1B (C0)
        let input = "a\u{1b}[31mred\u{1b}[0mb";
        let result = html_to_text(input);
        // ESC (0x1B) must be gone
        assert!(
            !result.contains('\u{1b}'),
            "ESC must be stripped; got: {result:?}"
        );
        // The visible chars remain
        assert!(result.contains('a'));
        assert!(result.contains('b'));
    }

    #[test]
    fn null_byte_stripped() {
        let input = "hel\0lo";
        assert_eq!(html_to_text(input), "hello");
    }

    #[test]
    fn del_byte_stripped() {
        let input = "hel\u{7f}lo";
        assert_eq!(html_to_text(input), "hello");
    }

    #[test]
    fn newline_preserved_tab_preserved() {
        // \n and \t must survive (not stripped as C0)
        assert_eq!(html_to_text("a\nb"), "a\nb");
        assert_eq!(html_to_text("a\tb"), "a\tb");
    }

    #[test]
    fn blank_line_collapsing() {
        // 3 newlines → 2 (at most one blank line)
        assert_eq!(html_to_text("a\n\n\nb"), "a\n\nb");
        // 5 newlines → 2
        assert_eq!(html_to_text("a\n\n\n\n\nb"), "a\n\nb");
        // 2 newlines stay as 2
        assert_eq!(html_to_text("a\n\nb"), "a\n\nb");
    }

    #[test]
    fn beol_metadata_block() {
        // Simulated beol-style metadata block (HTML from the live API).
        let input = concat!(
            "Project Metadata:<br/>",
            "View Metadata (<a href=\"https://meta.dasch.swiss/projects/0801\">",
            "https://meta.dasch.swiss/projects/0801</a>)",
            "<br/><br/>",
            "Dataset License:<br/>",
            "CC BY-NC-SA 4.0 (<a href=\"https://creativecommons.org/licenses/by-nc-sa/4.0/\">",
            "https://creativecommons.org/licenses/by-nc-sa/4.0/</a>)",
            "<br/><br/>",
            "The Bernoulli-Euler Online (BEOL) project is a research platform."
        );
        let result = html_to_text(input);
        // No raw HTML tags remain.
        assert!(!result.contains('<'), "no raw tags; got: {result:?}");
        assert!(!result.contains('>'), "no raw tags; got: {result:?}");
        // Links are rendered as TEXT (URL).
        assert!(
            result.contains(
                "https://meta.dasch.swiss/projects/0801 (https://meta.dasch.swiss/projects/0801)"
            ),
            "link rendered as TEXT (URL); got: {result:?}"
        );
        // Main description text present.
        assert!(
            result.contains("The Bernoulli-Euler Online (BEOL) project"),
            "body text present; got: {result:?}"
        );
    }

    #[test]
    fn simple_bold_description() {
        // The existing beol unit-test fixture value.
        let result = html_to_text("<b>BEOL</b> \u{2014} early modern mathematics.");
        assert_eq!(result, "BEOL \u{2014} early modern mathematics.");
    }

    #[test]
    fn nested_formatting() {
        let result = html_to_text("<p><strong>Title</strong>: text &amp; more</p>");
        assert!(!result.contains('<'), "no tags; got: {result:?}");
        assert!(result.contains("Title"), "title present; got: {result:?}");
        assert!(
            result.contains("text & more"),
            "entity unescaped; got: {result:?}"
        );
    }

    #[test]
    fn href_in_larger_tag_attrs() {
        // href not first attribute
        let result = html_to_text("<a class=\"foo\" href=\"https://x.com\">link text</a>");
        assert_eq!(result, "link text (https://x.com)");
    }

    // ── strip_control_chars tests ─────────────────────────────────────────────

    #[test]
    fn strip_control_chars_keeps_printable() {
        assert_eq!(strip_control_chars("hello world"), "hello world");
    }

    #[test]
    fn strip_control_chars_keeps_newline_and_tab() {
        assert_eq!(strip_control_chars("a\nb"), "a\nb");
        assert_eq!(strip_control_chars("a\tb"), "a\tb");
    }

    #[test]
    fn strip_control_chars_removes_c0_and_del() {
        // NUL
        assert_eq!(strip_control_chars("a\x00b"), "ab");
        // ESC
        assert_eq!(strip_control_chars("a\x1bb"), "ab");
        // DEL
        assert_eq!(strip_control_chars("a\x7fb"), "ab");
    }

    // ── replace_control_chars tests (tabular sanitiser; moved from table.rs) ──

    #[test]
    fn replace_control_chars_plain_string_unchanged() {
        assert_eq!(replace_control_chars("hello world"), "hello world");
    }

    #[test]
    fn replace_control_chars_tab_replaced() {
        assert_eq!(replace_control_chars("a\tb"), "a b");
    }

    #[test]
    fn replace_control_chars_newline_replaced() {
        assert_eq!(replace_control_chars("a\nb"), "a b");
    }

    #[test]
    fn replace_control_chars_cr_replaced() {
        assert_eq!(replace_control_chars("a\rb"), "a b");
    }

    #[test]
    fn replace_control_chars_nul_replaced() {
        assert_eq!(replace_control_chars("a\x00b"), "a b");
    }

    #[test]
    fn replace_control_chars_esc_replaced() {
        assert_eq!(replace_control_chars("a\x1bb"), "a b");
    }

    #[test]
    fn replace_control_chars_del_replaced() {
        assert_eq!(replace_control_chars("a\x7fb"), "a b");
    }

    #[test]
    fn replace_control_chars_multibyte_unicode_unchanged() {
        // Non-ASCII Unicode must pass through untouched.
        assert_eq!(replace_control_chars("héllo wörld"), "héllo wörld");
        assert_eq!(replace_control_chars("日本語"), "日本語");
    }

    #[test]
    fn replace_control_chars_printable_ascii_unchanged() {
        // Space (0x20) and tilde (0x7e) are the boundary printable chars — both
        // must survive unmodified.
        assert_eq!(replace_control_chars(" "), " ");
        assert_eq!(replace_control_chars("~"), "~");
    }

    #[test]
    fn replace_control_chars_multiple_controls() {
        // Each control character independently becomes a space.
        assert_eq!(replace_control_chars("a\t\nb\rc"), "a  b c");
    }

    // ── sanitise_and_cap tests ─────────────────────────────────────────────────

    #[test]
    fn strip_control_chars_strips_c1_csi() {
        // U+009B is the single-character form of `ESC [` — an ASCII-only filter
        // let it through, leaving a working terminal control introducer in
        // server-supplied prose. Found by review 2026-08-07.
        let out = strip_control_chars("before\u{9b}31mafter");
        assert!(!out.contains('\u{9b}'), "C1 CSI must be stripped: {out:?}");
        assert_eq!(out, "before31mafter");
        // U+0085 (NEL) too, while \n and \t still survive.
        assert_eq!(strip_control_chars("a\u{85}b\nc\td"), "ab\nc\td");
    }

    #[test]
    fn sanitise_and_cap_strips_esc() {
        let input = "a\u{1b}[31mred\u{1b}[0mb";
        let result = sanitise_and_cap(input);
        assert!(
            !result.contains('\u{1b}'),
            "ESC must be stripped: {result:?}"
        );
    }

    #[test]
    fn sanitise_and_cap_strips_cr() {
        assert_eq!(sanitise_and_cap("a\rb"), "ab");
    }

    #[test]
    fn sanitise_and_cap_strips_del() {
        assert_eq!(sanitise_and_cap("a\u{7f}b"), "ab");
    }

    #[test]
    fn sanitise_and_cap_keeps_newline_and_tab() {
        // The prose sanitiser keeps \n/\t (multi-line Fuseki parse errors stay
        // readable) — not neutralised the way the tabular sibling would.
        assert_eq!(sanitise_and_cap("line1\nline2\tcol"), "line1\nline2\tcol");
    }

    #[test]
    fn sanitise_and_cap_under_cap_unchanged() {
        let s = "short message";
        assert_eq!(sanitise_and_cap(s), s);
    }

    #[test]
    fn sanitise_and_cap_at_cap_no_ellipsis() {
        let s = "a".repeat(200);
        let result = sanitise_and_cap(&s);
        assert_eq!(result, s);
        assert!(!result.ends_with('…'));
    }

    #[test]
    fn sanitise_and_cap_over_cap_truncates_with_ellipsis() {
        let s = "a".repeat(250);
        let result = sanitise_and_cap(&s);
        assert_eq!(result.chars().count(), 201); // 200 chars + '…'
        assert!(result.ends_with('…'));
        assert_eq!(result.chars().filter(|&c| c == 'a').count(), 200);
    }

    #[test]
    fn sanitise_and_cap_multibyte_boundary_does_not_panic() {
        // 250 multibyte chars (each 3 bytes in UTF-8) — a byte-index cap at
        // 200 would land mid-character and panic; chars()-based capping must
        // not.
        let s = "日".repeat(250);
        let result = sanitise_and_cap(&s);
        assert_eq!(result.chars().count(), 201);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn sanitise_and_cap_removes_control_chars_before_capping() {
        // Control chars are stripped before the cap decision, so the
        // char-count used for the cap is the *post-strip* count, not the raw
        // input length.
        let s = "a\x00".repeat(210); // strips to 210 'a' chars — over the cap
        let result = sanitise_and_cap(&s);
        assert!(!result.contains('\u{0}'));
        assert!(result.ends_with('…'));
        assert_eq!(result.chars().count(), 201);
    }
}
