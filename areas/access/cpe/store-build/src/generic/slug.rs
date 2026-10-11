//! Slugs, which ids are made of.
//!
//! An id is part of a public URL, so this rule must give the same text for the same input for
//! as long as the URLs are to hold.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

use crate::error::{BuildError, Rule};

/// `text` in compatibility decomposition without its combining marks, ASCII letters in lower
/// case, every run of characters outside `[a-z0-9]` as one `-`, and no `-` at either end.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_dash = false;
    for c in text.nfkd().filter(|c| !is_combining_mark(*c)) {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    out
}

/// Whether `id` can stand in a URL path segment unchanged: it is not empty, and it holds only
/// the unreserved characters of RFC 3986, `A-Z a-z 0-9 - . _ ~`.
pub fn is_url_safe(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
}

/// Refuses every id that is not [`is_url_safe`]. An item is the id, then the subject and the
/// detail of its refusal.
pub(crate) fn refuse_unsafe_ids<'s>(
    ids: impl IntoIterator<Item = (&'s str, &'s str, &'s str)>,
    errors: &mut Vec<BuildError>,
) {
    for (id, subject, detail) in ids {
        if !is_url_safe(id) {
            errors.push(BuildError::refused(Rule::IdNotUrlSafe, subject, detail));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url_safe_accepts_the_unreserved_characters() {
        assert!(is_url_safe("A-Za-z0-9-._~"));
        assert!(is_url_safe("bereitung-a1r-2"));
        assert!(is_url_safe("ScanOfThePage"));
    }

    #[test]
    fn test_is_url_safe_refuses_the_empty_id() {
        assert!(!is_url_safe(""));
    }

    #[test]
    fn test_is_url_safe_refuses_a_space_a_slash_and_a_percent_sign() {
        assert!(!is_url_safe("a b"));
        assert!(!is_url_safe("A/b"));
        assert!(!is_url_safe("100%"));
        assert!(!is_url_safe("a%20b"));
    }

    #[test]
    fn test_is_url_safe_refuses_the_other_reserved_characters() {
        for c in ":?#[]@!$&'()*+,;=".chars() {
            assert!(!is_url_safe(&format!("a{c}b")), "{c}");
        }
    }

    #[test]
    fn test_is_url_safe_refuses_a_non_ascii_letter_and_a_control_character() {
        assert!(!is_url_safe("café"));
        assert!(!is_url_safe("\u{3b1}"));
        assert!(!is_url_safe("a\u{ff21}"));
        assert!(!is_url_safe("a\nb"));
        assert!(!is_url_safe("a\0b"));
    }

    #[test]
    fn test_slug_folds_the_real_pagenum_shapes() {
        assert_eq!(slug("[missing]"), "missing");
        assert_eq!(slug("a1r; Titelblatt"), "a1r-titelblatt");
        assert_eq!(slug("a1r, Titelblatt"), "a1r-titelblatt");
        assert_eq!(slug("Einband, Vorderdeckel, aussen"), "einband-vorderdeckel-aussen");
        assert_eq!(slug("3"), "3");
        assert_eq!(slug("  B1R  "), "b1r");
    }

    #[test]
    fn test_slug_folds_diacritics_and_compatibility_forms() {
        assert_eq!(slug("Rückendeckel, außen"), "ruckendeckel-au-en");
        assert_eq!(slug("Vorsatz ﬁ ²"), "vorsatz-fi-2");
        assert_eq!(slug("Café"), "cafe");
    }

    /// U+212A KELVIN SIGN decomposes to `K`; U+03B1 GREEK SMALL LETTER ALPHA to nothing in ASCII.
    #[test]
    fn test_slug_letter_without_an_ascii_decomposition_is_a_separator() {
        assert_eq!(slug("\u{212a}1"), "k1");
        assert_eq!(slug("a\u{3b1}b"), "a-b");
    }

    #[test]
    fn test_slug_pure_punctuation_is_empty() {
        assert_eq!(slug("[ ; ]"), "");
        assert_eq!(slug(""), "");
    }
}
