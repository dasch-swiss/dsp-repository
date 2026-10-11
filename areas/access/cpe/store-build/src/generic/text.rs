//! Python's `str.strip()`.
//!
//! The builder trims a text as Python's `str.strip()` does: its whitespace is
//! `char::is_whitespace` plus the separators U+001C to U+001F. `str::trim` keeps those four and
//! would change texts of the committed store, so every trim goes through this module. The set
//! is `cpe-core`'s, which holds a mapping's sentinel to it.

use cpe_core::remodel::is_strip_space;

use crate::error::Rule;

pub(crate) fn py_strip(text: &str) -> &str {
    text.trim_matches(is_strip_space)
}

/// `text` without a trailing U+001E and the whitespace around it. A text that ends in
/// whitespace alone is returned as it is: an editor wrote that.
///
/// The whitespace here is `char::is_whitespace`. Do not make it [`py_strip`]'s set: a title
/// is compared with its resource's label after this cut, and with that set the cut would also take
/// a U+001C, U+001D or U+001F that stands before the separator.
pub(crate) fn without_record_separator(text: &str) -> &str {
    let kept = text.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{1e}');
    if text[kept.len()..].contains('\u{1e}') {
        kept
    } else {
        text
    }
}

/// The project's sentinel and where a text may not hold it.
#[derive(Clone, Copy)]
pub(crate) struct Sentinel<'s> {
    pub(crate) text: &'s str,
    /// Whether a text that is the sentinel alone, once stripped, is refused.
    pub(crate) whole: bool,
    /// Whether a longer text that holds the sentinel is refused.
    pub(crate) within: bool,
}

impl<'s> Sentinel<'s> {
    /// The sentinel refused wherever it stands.
    pub(crate) fn anywhere(text: &'s str) -> Self {
        Self { text, whole: true, within: true }
    }
}

/// What a text is held to. Every reader of a text says which of the three it asks for.
#[derive(Clone, Copy)]
pub(crate) struct Limits<'s> {
    /// A text that is empty once stripped is refused.
    pub(crate) not_empty: bool,
    /// A text that holds an address is refused.
    pub(crate) no_addresses: bool,
    /// `None` where no sentinel is declared.
    pub(crate) sentinel: Option<Sentinel<'s>>,
}

/// Why `text` may not be a value under `limits`, if it may not: it is empty once stripped, it
/// holds an address, or it is or holds the sentinel; in that order. A mapping's sentinel is
/// never empty and neither starts nor ends with whitespace, so an empty text holds none.
pub(crate) fn offence(text: &str, limits: Limits<'_>) -> Option<Rule> {
    let stripped = py_strip(text);
    if limits.not_empty && stripped.is_empty() {
        return Some(Rule::EmptyValue);
    }
    if limits.no_addresses && text.contains("http") {
        return Some(Rule::HttpInValue);
    }
    let sentinel = limits.sentinel?;
    let refused = if stripped == sentinel.text {
        sentinel.whole
    } else {
        sentinel.within && text.contains(sentinel.text)
    };
    refused.then_some(Rule::MissingInValue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_without_record_separator_cuts_the_separator_with_its_whitespace_only() {
        let cases = [
            ("Ein Titel.\u{1e}\n        ", "Ein Titel."),
            ("Ein Titel. \u{1e}", "Ein Titel."),
            ("Ein Titel.\n", "Ein Titel.\n"),
            ("Ein\u{1e}Titel", "Ein\u{1e}Titel"),
            ("", ""),
        ];

        for (text, expected) in cases {
            assert_eq!(without_record_separator(text), expected, "{text:?}");
        }
    }

    /// (input, `input.strip()`), the expected values printed by `python3`.
    const STRIPPED: [(&str, &str); 12] = [
        ("  a b  ", "a b"),
        ("\u{1c}a\u{1f}", "a"),
        ("\u{a0}x\u{3000}", "x"),
        ("a\u{200b}", "a\u{200b}"),
        ("\u{200b}x", "\u{200b}x"),
        ("\t\n x \r\n", "x"),
        ("", ""),
        (" \u{1e} ", ""),
        ("x\u{2028}", "x"),
        ("\u{feff}x ", "\u{feff}x"),
        ("\u{1c}\u{1d}\u{1e}\u{1f}", ""),
        ("a\u{1e}", "a"),
    ];

    #[test]
    fn test_py_strip_matches_python() {
        for (input, stripped) in STRIPPED {
            assert_eq!(py_strip(input), stripped, "{input:?}");
        }
    }
}
