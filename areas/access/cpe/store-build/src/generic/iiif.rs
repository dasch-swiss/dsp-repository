//! IIIF Image API addresses of an archive asset.

/// The full-size image of `asset` on the server at `base`, which ends in `/`.
///
/// `cpe_core::iiif` derives every other size, and the address of `info.json`, from the
/// `/full/max/` segment of this form.
pub(crate) fn uri(base: &str, asset: &str) -> String {
    format!("{base}{asset}/full/max/0/default.jpg")
}

/// `asset` without its last extension; the whole name where it has none.
pub(crate) fn stem(asset: &str) -> &str {
    asset.rsplit_once('.').map_or(asset, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_is_the_full_size_address_of_the_asset() {
        assert_eq!(
            uri("https://iiif.dasch.swiss/0803/", "109z6wZnSVe-EZnteS9WqWU.jp2"),
            "https://iiif.dasch.swiss/0803/109z6wZnSVe-EZnteS9WqWU.jp2/full/max/0/default.jpg"
        );
    }

    #[test]
    fn test_stem_drops_the_last_extension_only() {
        assert_eq!(stem("109z6wZnSVe-EZnteS9WqWU.jp2"), "109z6wZnSVe-EZnteS9WqWU");
        assert_eq!(stem("a.b.jp2"), "a.b");
        assert_eq!(stem("no-extension"), "no-extension");
        assert_eq!(stem(".jp2"), "");
    }
}
