/// Whether `s` is a shortcode the bytes port serves: four hexadecimal digits, upper case.
///
/// An adapter answers every other shortcode with
/// [`FilesError::UnknownProject`](crate::FilesError::UnknownProject) before touching storage. The
/// rule is stricter than `shared_metadata::is_valid_shortcode`, so the five five-character
/// shortcodes (`0801a`–`0801e`, root `CONTEXT.md`) cannot be served through this port. They are
/// refused on purpose: a metadata-only split of BEOL, one DSP project `0801`, so no bucket prefix
/// exists under them.
#[must_use]
pub fn is_shortcode(s: &str) -> bool {
    s.len() == 4 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'F'))
}

/// Whether `name` is a file name the bytes port serves: one or more `/`-separated segments, each
/// non-empty, of `[A-Za-z0-9._-]` and not starting with `.`.
///
/// An adapter answers every other name with
/// [`FilesError::UnknownFile`](crate::FilesError::UnknownFile) before touching storage, so no name
/// climbs out of the project's subtree by its spelling. A file-system adapter must also refuse
/// symlinks, which this rule cannot see.
#[must_use]
pub fn is_file_name(name: &str) -> bool {
    name.split('/').all(|segment| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_shortcode_four_upper_case_hex_digits_accepted() {
        for shortcode in ["0803", "0AB1"] {
            assert!(is_shortcode(shortcode), "{shortcode}");
        }
    }

    #[test]
    fn test_is_shortcode_wrong_length_case_or_digit_refused() {
        for shortcode in ["803", "08030", "0ab1", "ZZZZ", "../x"] {
            assert!(!is_shortcode(shortcode), "{shortcode}");
        }
    }

    #[test]
    fn test_is_file_name_plain_and_nested_names_accepted() {
        for name in ["config.kdl", "curation.csv", "narrative/home.de.md"] {
            assert!(is_file_name(name), "{name}");
        }
    }

    #[test]
    fn test_is_file_name_escaping_or_malformed_names_refused() {
        for name in [
            "",
            "a//b",
            "narrative/",
            "..",
            "narrative/.x",
            "/config.kdl",
            "narrative\\home.md",
            "nüt.md",
        ] {
            assert!(!is_file_name(name), "{name}");
        }
    }
}
