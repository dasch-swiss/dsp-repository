//! Every URL path CPE writes: the base it is mounted at, and under it the
//! pages, the shared static files and a project's own files.
//!
//! The router does not know where it is mounted (a host nests it), so a
//! handler sees a path without the base, and everything written into a
//! response is built here from the configured [`Base`]. No other module joins
//! a base, a project id or one of the reserved segments into a path: a link
//! built elsewhere leaves the mount.
//!
//! The layout (ADR-0007):
//!
//! | What | Path |
//! |------|------|
//! | A project's page | `<base>/<project id><route>` |
//! | The shared static files, for a project's pages | `<base>/<project id>/_static/…` |
//! | The same files, for a page without a project | `<base>/_static/…` |
//! | A project's own files | `<base>/<project id>/_assets/…` |
//!
//! The first segment of a project's route never starts with `_`
//! (`DiagnosticKind::ReservedRoute`), which keeps its pages off the two
//! reserved segments.

use std::borrow::Cow;

/// The segment the shared static files are served under.
const STATIC_SEGMENT: &str = "_static";

/// The segment a project's own files are served under.
const ASSETS_SEGMENT: &str = "_assets";

/// The path CPE is mounted at: empty at the root of the host, else a path
/// that starts with `/` and does not end with one (`/cpe`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Base(String);

/// The base of a mount at the root of the host.
pub static ROOT: Base = Base(String::new());

/// Why a value cannot be a [`Base`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BaseError {
    #[error("must be empty or start with `/` (got {0:?})")]
    NoLeadingSlash(String),
    #[error("must not end with `/` (got {0:?})")]
    TrailingSlash(String),
    #[error("must be segments of letters, digits, `-`, `_`, `.` and `~`, each after one `/` (got {0:?})")]
    Segment(String),
}

impl Base {
    /// `value` as a base. It is written into every link, redirect and cookie
    /// as it is, so nothing that needs escaping there is accepted: a segment
    /// holds unreserved characters only, and none is empty, `.` or `..`.
    /// An empty segment would make `//host`, which a browser reads as another
    /// origin.
    pub fn parse(value: &str) -> Result<Self, BaseError> {
        if value.is_empty() {
            return Ok(Self::default());
        }
        let Some(segments) = value.strip_prefix('/') else {
            return Err(BaseError::NoLeadingSlash(value.to_owned()));
        };
        if value.ends_with('/') {
            return Err(BaseError::TrailingSlash(value.to_owned()));
        }
        let unreserved = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~');
        let usable = |segment: &str| !matches!(segment, "" | "." | "..") && segment.chars().all(unreserved);
        if !segments.split('/').all(usable) {
            return Err(BaseError::Segment(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    /// The base as it is written: empty, or `/cpe`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path CPE's own start is linked at, `<base>/`.
    pub fn home(&self) -> String {
        format!("{}/", self.0)
    }

    /// The `Path` of CPE's cookies: the base, and `/` at a root mount, so
    /// that they reach every project and nothing outside CPE.
    pub fn cookie_path(&self) -> &str {
        if self.0.is_empty() {
            "/"
        } else {
            &self.0
        }
    }

    /// The public path of a request, from the path its handler sees, which is
    /// the part below the mount. The current path of a page, both redirects
    /// and the share links are built from this. `path` keeps its query, if it
    /// has one.
    pub fn request_path(&self, path: &str) -> String {
        format!("{}{path}", self.0)
    }

    /// A link destination written in a project's content, as it is written
    /// into the page. A destination that starts with exactly one `/` names a
    /// path of CPE and gets the base in front. Anything else is left as it
    /// is: `//host/…`, a URL with a scheme, a fragment and a relative link.
    pub fn content_link<'a>(&self, destination: &'a str) -> Cow<'a, str> {
        if self.0.is_empty() || !destination.starts_with('/') || destination.starts_with("//") {
            Cow::Borrowed(destination)
        } else {
            Cow::Owned(self.request_path(destination))
        }
    }

    /// Where a page without a project loads the shared static files from.
    pub fn static_dir(&self) -> StaticDir {
        StaticDir(format!("{}/{STATIC_SEGMENT}", self.0))
    }

    /// The URLs of the project `id`.
    pub fn project<'a>(&'a self, id: &'a str) -> ProjectUrls<'a> {
        ProjectUrls { base: self, id }
    }
}

/// The URLs of one project. This is the one place that turns a project id
/// into the project's segment of a path.
#[derive(Debug, Clone, Copy)]
pub struct ProjectUrls<'a> {
    base: &'a Base,
    id: &'a str,
}

impl ProjectUrls<'_> {
    /// CPE's own start, `<base>/`: where a project's pages link out of the
    /// project.
    pub fn site_home(&self) -> String {
        self.base.home()
    }

    /// The project's start page, `<base>/<project id>/`.
    pub fn home(&self) -> String {
        format!("{}/{}/", self.base.0, self.id)
    }

    /// The page at `route`, a route of the project's config (`/books`,
    /// `/books/b1`); it starts with `/`. A debug build panics on one that
    /// does not: it would be joined to the project id without a separator.
    pub fn page(&self, route: &str) -> String {
        debug_assert!(route.starts_with('/'), "a route starts with `/` (got {route:?})");
        format!("{}/{}{route}", self.base.0, self.id)
    }

    /// Where the project's pages load the shared static files from.
    pub fn static_dir(&self) -> StaticDir {
        StaticDir(format!("{}/{}/{STATIC_SEGMENT}", self.base.0, self.id))
    }

    /// One of the project's own files, by its path in the project folder
    /// (`assets/cover.webp`), which does not start with `/`. A debug build
    /// panics on one that does: the URL would hold an empty segment.
    pub fn asset(&self, path: &str) -> String {
        debug_assert!(
            !path.starts_with('/'),
            "a path in the project folder is relative (got {path:?})"
        );
        format!("{}/{}/{ASSETS_SEGMENT}/{path}", self.base.0, self.id)
    }
}

/// A directory the shared static files are served from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticDir(String);

impl StaticDir {
    /// The file `name` of the static table (`cpe/tokens.css`), or a
    /// directory of it (`image-viewer`).
    pub fn file(&self, name: &str) -> String {
        format!("{}/{name}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cpe() -> Base {
        Base::parse("/cpe").expect("/cpe is a base")
    }

    #[test]
    fn a_base_is_empty_or_a_path_without_a_trailing_slash() {
        for good in ["", "/cpe", "/a/b", "/cpe-2", "/c_p.e~"] {
            assert_eq!(Base::parse(good).map(|base| base.as_str().to_owned()), Ok(good.to_owned()));
        }
        assert_eq!(Base::parse(""), Ok(ROOT.clone()));
    }

    #[test]
    fn a_base_without_a_leading_slash_or_with_a_trailing_one_is_refused() {
        assert_eq!(Base::parse("cpe"), Err(BaseError::NoLeadingSlash("cpe".to_owned())));
        assert_eq!(
            Base::parse("https://evil.example"),
            Err(BaseError::NoLeadingSlash("https://evil.example".to_owned()))
        );
        assert_eq!(Base::parse("/cpe/"), Err(BaseError::TrailingSlash("/cpe/".to_owned())));
        assert_eq!(Base::parse("/"), Err(BaseError::TrailingSlash("/".to_owned())));
    }

    /// The base is written into links, `Location` and `Set-Cookie` unescaped.
    #[test]
    fn a_base_that_would_need_escaping_or_names_another_origin_is_refused() {
        for bad in [
            "//evil.example",
            "/cpe//x",
            "/a/../b",
            "/.",
            "/c pe",
            "/cpe;x",
            "/cpe?x",
            "/cpe#x",
            "/cpe\"",
            "/cp\\e",
            "/cpe\r\nSet-Cookie: x",
            "/%2f",
            "/cpé",
        ] {
            assert_eq!(Base::parse(bad), Err(BaseError::Segment(bad.to_owned())), "{bad:?}");
        }
    }

    #[test]
    fn at_the_root_every_url_is_the_bare_path() {
        let project = ROOT.project("incunabula");
        assert_eq!(ROOT.home(), "/");
        assert_eq!(ROOT.cookie_path(), "/");
        assert_eq!(project.home(), "/incunabula/");
        assert_eq!(project.page("/books"), "/incunabula/books");
        assert_eq!(project.page("/books/b1"), "/incunabula/books/b1");
        assert_eq!(
            project.static_dir().file("cpe/tokens.css"),
            "/incunabula/_static/cpe/tokens.css"
        );
        assert_eq!(project.asset("assets/cover.webp"), "/incunabula/_assets/assets/cover.webp");
        assert_eq!(ROOT.static_dir().file("datastar.js"), "/_static/datastar.js");
        assert_eq!(ROOT.request_path("/incunabula/books"), "/incunabula/books");
    }

    #[test]
    fn under_a_base_every_url_starts_with_it() {
        let base = cpe();
        let project = base.project("incunabula");
        assert_eq!(base.home(), "/cpe/");
        assert_eq!(base.cookie_path(), "/cpe");
        assert_eq!(project.home(), "/cpe/incunabula/");
        assert_eq!(project.page("/books"), "/cpe/incunabula/books");
        assert_eq!(
            project.static_dir().file("cpe/tokens.css"),
            "/cpe/incunabula/_static/cpe/tokens.css"
        );
        assert_eq!(
            project.static_dir().file("image-viewer"),
            "/cpe/incunabula/_static/image-viewer"
        );
        assert_eq!(project.asset("assets/cover.webp"), "/cpe/incunabula/_assets/assets/cover.webp");
        assert_eq!(base.static_dir().file("datastar.js"), "/cpe/_static/datastar.js");
    }

    #[test]
    fn the_project_segment_is_the_project_id() {
        assert_eq!(
            cpe().project("alice-in-daschland").page("/characters"),
            "/cpe/alice-in-daschland/characters"
        );
        assert_eq!(ROOT.project("p2").home(), "/p2/");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a route starts with `/`")]
    fn a_route_without_a_leading_slash_is_refused_in_a_debug_build() {
        let _ = cpe().project("incunabula").page("books");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a path in the project folder is relative")]
    fn an_asset_path_with_a_leading_slash_is_refused_in_a_debug_build() {
        let _ = cpe().project("incunabula").asset("/assets/cover.webp");
    }

    /// A fragment endpoint is a page's route with a segment after it.
    #[test]
    fn a_fragment_url_is_the_page_url_and_its_segment() {
        assert_eq!(
            cpe().project("incunabula").page("/books/_facets"),
            "/cpe/incunabula/books/_facets"
        );
        assert_eq!(cpe().request_path("/incunabula/books/_facets"), "/cpe/incunabula/books/_facets");
    }

    /// The path a share link is built from: the origin goes in front of it.
    #[test]
    fn a_request_path_keeps_its_query() {
        assert_eq!(
            cpe().request_path("/incunabula/books?page=2&lang=de"),
            "/cpe/incunabula/books?page=2&lang=de"
        );
        assert_eq!(ROOT.request_path("/incunabula/books?page=2"), "/incunabula/books?page=2");
        assert_eq!(
            format!("https://repository.dasch.swiss{}", cpe().request_path("/incunabula/books/b1")),
            "https://repository.dasch.swiss/cpe/incunabula/books/b1"
        );
    }

    #[test]
    fn a_content_link_with_one_leading_slash_gets_the_base() {
        let base = cpe();
        assert_eq!(base.content_link("/incunabula/books"), "/cpe/incunabula/books");
        assert_eq!(
            base.content_link("/incunabula/books?page=2#x"),
            "/cpe/incunabula/books?page=2#x"
        );
        assert_eq!(base.content_link("/"), "/cpe/");
        assert_eq!(ROOT.content_link("/incunabula/books"), "/incunabula/books");
    }

    #[test]
    fn any_other_content_link_is_left_as_it_is() {
        let base = cpe();
        for kept in [
            "//evil.example/x",
            "https://example.org/a",
            "http://example.org",
            "mailto:a@b.example",
            "#section",
            "notes.html",
            "../up",
            "?page=2",
            "",
        ] {
            assert_eq!(base.content_link(kept), kept);
            assert_eq!(ROOT.content_link(kept), kept);
        }
    }
}
