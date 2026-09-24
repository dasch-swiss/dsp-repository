//! Resolving the stylesheet href served in the page shell.

/// Resolve the stylesheet href. Dev serves the unhashed `app.css` at a fixed
/// path; release builds emit a content-hashed `app.<hash>.css`, whose name is
/// discovered from the assets directory at startup.
pub(crate) fn resolve_css_href(public_dir: &std::path::Path) -> String {
    if cfg!(debug_assertions) {
        return "/assets/app.css".to_string();
    }
    let assets = public_dir.join("assets");
    discover_hashed_css(&assets).unwrap_or_else(|| "/assets/app.css".to_string())
}

/// Scan `assets_dir` for a content-hashed `app.<hash>.css` and return its
/// `/assets/…` href if one is present. Kept separate from [`resolve_css_href`]
/// (which gates on `debug_assertions`) so the discovery logic is unit-testable
/// under `cargo test`.
fn discover_hashed_css(assets_dir: &std::path::Path) -> Option<String> {
    let entries = std::fs::read_dir(assets_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("app.") && name.ends_with(".css") && name != "app.css" {
            return Some(format!("/assets/{name}"));
        }
    }
    None
}

#[cfg(test)]
mod css_href_tests {
    use super::discover_hashed_css;

    #[test]
    fn discovers_content_hashed_stylesheet() {
        let dir = std::env::temp_dir().join(format!("dpe_css_discover_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.abc123.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir).as_deref(), Some("/assets/app.abc123.css"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_unhashed_stylesheet_and_returns_none() {
        let dir = std::env::temp_dir().join(format!("dpe_css_none_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.css"), "").unwrap();

        assert_eq!(discover_hashed_css(&dir), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_dir_returns_none() {
        let dir = std::env::temp_dir().join(format!("dpe_css_missing_{}", std::process::id()));
        assert_eq!(discover_hashed_css(&dir), None);
    }
}
