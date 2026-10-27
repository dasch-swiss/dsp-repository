//! Typed builders for Datastar action expressions (`@get(...)`, `@post(...)`).
//!
//! Attribute names (`data-on:click`, ...) stay Maud literals, since Maud takes
//! only literal tokens as attribute names, checked by
//! `.github/scripts/check-datastar-attributes.sh`; this crate types only the
//! action expression that is the attribute's value.
//!
//! [`Get`] and [`Post`] implement [`std::fmt::Display`] and emit raw text.
//! They do not implement `maud::Render` and do not escape HTML: Maud escapes
//! the splice. Signal and JS expressions are deliberately not typed.

use std::fmt;

/// Start a `@get(...)` action for `path`.
pub fn get(path: impl Into<String>) -> Get {
    Get { path: path.into(), tail_js: None, retry_never: false }
}

/// Start a `@post(...)` action for `path`.
pub fn post(path: impl Into<String>) -> Post {
    Post { path: path.into(), submitter_first: false, form: false }
}

/// A `@get(...)` action. Created by [`get`].
#[derive(Debug, Clone)]
pub struct Get {
    path: String,
    tail_js: Option<&'static str>,
    retry_never: bool,
}

impl Get {
    /// Appends a JS expression to the path: `'path' + <expr>`.
    ///
    /// The expression is emitted verbatim and is not encoded, so it is a
    /// `&'static str`: no runtime value can reach it.
    pub fn plus_js(mut self, expr: &'static str) -> Self {
        self.tail_js = Some(expr);
        self
    }

    /// Adds `{retry: 'never', retryMaxCount: 0}`.
    ///
    /// The two keys cannot be split: `retry: 'never'` alone still retries
    /// network failures for minutes, only `retryMaxCount: 0` stops that
    /// (DEV-7366).
    pub fn retry_never(mut self) -> Self {
        self.retry_never = true;
        self
    }
}

impl fmt::Display for Get {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@get('")?;
        write_encoded_path(f, &self.path)?;
        f.write_str("'")?;
        if let Some(tail) = &self.tail_js {
            write!(f, " + {tail}")?;
        }
        if self.retry_never {
            f.write_str(", {retry: 'never', retryMaxCount: 0}")?;
        }
        f.write_str(")")
    }
}

/// A `@post(...)` action. Created by [`post`].
#[derive(Debug, Clone)]
pub struct Post {
    path: String,
    submitter_first: bool,
    form: bool,
}

impl Post {
    /// Prefers the submitter's `formaction` over the path:
    /// `evt.submitter?.formAction || 'path'`.
    ///
    /// Datastar prevents the form's default submit, which drops the
    /// submitter's URL; this keeps a submit button's `formaction` in effect.
    pub fn submitter_first(mut self) -> Self {
        self.submitter_first = true;
        self
    }

    /// Adds `{contentType: 'form'}`.
    pub fn form(mut self) -> Self {
        self.form = true;
        self
    }
}

impl fmt::Display for Post {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("@post(")?;
        if self.submitter_first {
            f.write_str("evt.submitter?.formAction || ")?;
        }
        f.write_str("'")?;
        write_encoded_path(f, &self.path)?;
        f.write_str("'")?;
        if self.form {
            f.write_str(", {contentType: 'form'}")?;
        }
        f.write_str(")")
    }
}

/// Percent-encodes the characters that would end or alter a single-quoted JS
/// string literal: `'`, `` ` `` and `\`. Everything else passes through.
fn write_encoded_path(f: &mut fmt::Formatter<'_>, path: &str) -> fmt::Result {
    for ch in path.chars() {
        match ch {
            '\'' => f.write_str("%27")?,
            '`' => f.write_str("%60")?,
            '\\' => f.write_str("%5C")?,
            _ => fmt::Write::write_char(f, ch)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_plain() {
        assert_eq!(get("/x").to_string(), "@get('/x')");
    }

    #[test]
    fn get_retry_never() {
        assert_eq!(
            get("/x").retry_never().to_string(),
            "@get('/x', {retry: 'never', retryMaxCount: 0})"
        );
    }

    #[test]
    fn get_plus_js() {
        assert_eq!(
            get("/x").plus_js("encodeURIComponent(evt.target.value)").to_string(),
            "@get('/x' + encodeURIComponent(evt.target.value))"
        );
    }

    #[test]
    fn post_form() {
        assert_eq!(post("/x").form().to_string(), "@post('/x', {contentType: 'form'})");
    }

    #[test]
    fn post_submitter_first_form() {
        assert_eq!(
            post("/x").submitter_first().form().to_string(),
            "@post(evt.submitter?.formAction || '/x', {contentType: 'form'})"
        );
    }

    #[test]
    fn path_encodes_only_the_js_string_breakout_characters() {
        assert_eq!(get("/a'b`c\\d").to_string(), "@get('/a%27b%60c%5Cd')");
        assert_eq!(post("/a'b").to_string(), "@post('/a%27b')");
        assert_eq!(get("/x?a=1&b=100%25 ü").to_string(), "@get('/x?a=1&b=100%25 ü')");
    }

    /// Guards against a `Render` impl or pre-escaping that would double- or
    /// un-escape the splice.
    #[test]
    fn maud_escapes_the_splice() {
        let markup = maud::html! {
            a data-on:click=(get("/x?a=1&b=2")) {}
        };
        assert_eq!(markup.into_string(), r#"<a data-on:click="@get('/x?a=1&amp;b=2')"></a>"#);
    }
}
