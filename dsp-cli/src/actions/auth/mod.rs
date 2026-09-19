//! Actions for `dsp auth { login | status | logout | set-token | token }`.

pub mod login;
pub mod logout;
pub mod set_token;
pub mod status;
pub mod token;

/// Trim a single trailing line terminator from a string read via `read_line`.
///
/// Pops one trailing `\n` if present, then pops one preceding `\r` only if it
/// directly followed that `\n`. A bare trailing `\r` with no preceding `\n` is
/// **not** stripped — `\r` is a legitimate character in a token or password, so
/// we must not use `trim_end_matches` or strip it unconditionally.
pub(crate) fn trim_line_ending(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::trim_line_ending;

    #[test]
    fn trim_line_ending_removes_trailing_lf() {
        let mut s = "hello\n".to_string();
        trim_line_ending(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn trim_line_ending_removes_trailing_crlf() {
        let mut s = "hello\r\n".to_string();
        trim_line_ending(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn trim_line_ending_leaves_no_terminator_unchanged() {
        let mut s = "hello".to_string();
        trim_line_ending(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn trim_line_ending_leaves_empty_string_unchanged() {
        let mut s = String::new();
        trim_line_ending(&mut s);
        assert_eq!(s, "");
    }

    #[test]
    fn trim_line_ending_does_not_strip_bare_cr_without_lf() {
        // A bare \r (no preceding \n) must NOT be stripped — it may be a
        // legitimate character in a token or password.
        let mut s = "hello\r".to_string();
        trim_line_ending(&mut s);
        assert_eq!(s, "hello\r", "bare \\r must not be removed");
    }
}
