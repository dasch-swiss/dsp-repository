/// In-process cache of the project cover images present on disk.
///
/// A cover image is optional: images are onboarded per project on request, so at
/// any time some published projects have one and some do not. Presence must
/// therefore be resolved before rendering, not corrected afterwards.
///
/// The directory is scanned once on first access and held for the lifetime of
/// the process, mirroring [`crate::project_cache`]. A file added or removed
/// afterwards is not picked up; the views keep an `onerror` fallback for that.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::utils::get_public_dir;

/// Subdirectory of the public dir holding the per-project covers. `ServeDir` mounts
/// the public dir at the router root, so the URL is this path with a leading slash.
/// Derived rather than declared twice, so a rename cannot leave the two disagreeing.
const IMAGES_SUBDIR: &str = "assets/images";
const COVER_EXTENSION: &str = "webp";

static COVERS: OnceLock<HashSet<String>> = OnceLock::new();

/// The URL of `shortcode`'s cover image, or `None` when the project has none.
///
/// `None` means "render the placeholder instead", not "render a broken image".
pub fn cover_image_url(shortcode: &str) -> Option<String> {
    resolve_cover_url(covers(), shortcode)
}

/// Return a reference to the cached set of cover-image stems, scanning on first call.
fn covers() -> &'static HashSet<String> {
    COVERS.get_or_init(|| scan_covers(&PathBuf::from(get_public_dir()).join(IMAGES_SUBDIR)))
}

/// Build the cover URL from an already-resolved stem set. Separated from the
/// cache lookup so it can be unit-tested with a synthetic set.
///
/// The comparison is deliberately **case-sensitive**: `ServeDir` resolves the
/// URL against a case-sensitive filesystem in the container, so matching
/// scanned names byte-for-byte is what makes a macOS dev box (case-insensitive)
/// agree with production about whether a cover is reachable.
fn resolve_cover_url(covers: &HashSet<String>, shortcode: &str) -> Option<String> {
    covers
        .contains(shortcode)
        .then(|| format!("/{IMAGES_SUBDIR}/{shortcode}.{COVER_EXTENSION}"))
}

/// Collect the file stems of every cover image directly under `images_dir`.
///
/// Names are taken from the directory listing rather than probed with
/// `Path::exists`, so the stem set holds what `ServeDir` can actually serve:
/// including the exact case, and excluding a name that only differs by
/// surrounding whitespace.
fn scan_covers(images_dir: &Path) -> HashSet<String> {
    let Ok(entries) = std::fs::read_dir(images_dir) else {
        tracing::warn!(dir = ?images_dir, "failed to read cover images directory; no covers will render");
        return HashSet::new();
    };

    let mut covers = HashSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(COVER_EXTENSION) {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            covers.insert(stem.to_string());
        }
    }
    tracing::info!(dir = ?images_dir, count = covers.len(), "cover images discovered");
    covers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers_of(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn resolves_the_url_for_a_project_with_a_cover() {
        let covers = covers_of(&["0803", "084D"]);
        assert_eq!(resolve_cover_url(&covers, "0803").as_deref(), Some("/assets/images/0803.webp"));
        assert_eq!(resolve_cover_url(&covers, "084D").as_deref(), Some("/assets/images/084D.webp"));
    }

    #[test]
    fn resolves_to_none_for_a_project_without_one() {
        assert_eq!(resolve_cover_url(&covers_of(&["0803"]), "0843"), None);
        assert_eq!(resolve_cover_url(&HashSet::new(), "0803"), None);
    }

    #[test]
    fn lookup_is_case_sensitive_like_servedir() {
        // `081B.webp` must not satisfy shortcode `081b`: the container's filesystem
        // is case-sensitive, so serving that URL would 404 in production even
        // though a macOS dev box would open the file.
        let covers = covers_of(&["081B"]);
        assert_eq!(resolve_cover_url(&covers, "081B").as_deref(), Some("/assets/images/081B.webp"));
        assert_eq!(resolve_cover_url(&covers, "081b"), None);
    }

    #[test]
    fn scan_collects_webp_stems_and_ignores_everything_else() {
        let dir = std::env::temp_dir().join(format!("dpe_covers_scan_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0803.webp"), "").unwrap();
        std::fs::write(dir.join("084D.webp"), "").unwrap();
        // Not covers: the wrong extension, and a subdirectory of licence badges.
        std::fs::write(dir.join("app.css"), "").unwrap();
        std::fs::create_dir_all(dir.join("cc-licenses")).unwrap();

        let covers = scan_covers(&dir);

        assert_eq!(covers, covers_of(&["0803", "084D"]), "{covers:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_leading_space_in_the_filename_does_not_resolve() {
        // A name differing only by surrounding whitespace must not satisfy the
        // shortcode: it would look present in a directory listing and 404 as a URL.
        let dir = std::env::temp_dir().join(format!("dpe_covers_space_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(" 0118.webp"), "").unwrap();

        let covers = scan_covers(&dir);

        assert_eq!(resolve_cover_url(&covers, "0118"), None, "{covers:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_directory_yields_no_covers_rather_than_panicking() {
        let covers = scan_covers(&PathBuf::from("/nonexistent/dpe/public/assets/images"));
        assert!(covers.is_empty());
    }
}
