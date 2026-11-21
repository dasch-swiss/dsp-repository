//! `cpe-components` — the chrome message catalog and the component renderers.
//!
//! `chrome` is the one catalog of the engine's language-varying UI strings;
//! `component` holds the renderers (and the per-project components) that turn
//! view data into Maud markup.
//!
//! This crate depends on `cpe-core` and `cpe-image-viewer`. Page assembly, which calls these
//! renderers and builds their config carriers, lives elsewhere.
//!
//! `pub` in this crate means visible to CPE's other crates. A composition root sees none of
//! it: `cpe-server`, the crate a composition root depends on, re-exports nothing of this
//! crate. Items marked `#[doc(hidden)]` are for the sibling crates and their tests.

pub mod chrome;
pub mod component;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    /// The `PreEscaped(` call sites of this crate's non-test code, per file below `src/`. Each
    /// comment names the one input its sites may splice.
    const PINNED: &[(&str, usize)] = &[
        // `pulldown-cmark`'s `push_html` output over Markdown with its raw HTML events removed,
        // and the citation sliced from that output, which holds no `<`.
        ("component/editorial.rs", 10),
        // `push_html` output over Markdown with its raw HTML events removed.
        ("component/markdown.rs", 2),
    ];

    /// Adding or removing a site fails this test until `PINNED` changes with it, so every new
    /// site is a reviewed one.
    ///
    /// It counts every line of a file outside its `#[cfg(test)]` modules, code after a test
    /// module included. It does not see a splice written without the call syntax
    /// (`.map(PreEscaped)`), or the sites in the Mosaic tiles, whose output this crate embeds.
    #[test]
    fn pre_escaped_call_sites_match_the_pinned_table() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut sites = BTreeMap::new();
        for path in rust_sources(&src) {
            let relative = path.strip_prefix(&src).expect("a path below src/");
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = std::fs::read_to_string(&path).expect("a readable source file");
            let non_test = strip_test_modules(&source).unwrap_or_else(|error| panic!("{relative}: {error}"));
            let count = non_test.matches("PreEscaped(").count();
            if count > 0 {
                sites.insert(relative, count);
            }
        }
        let pinned: BTreeMap<String, usize> = PINNED.iter().map(|&(file, n)| (file.to_string(), n)).collect();
        assert_eq!(sites, pinned, "a `PreEscaped(` site changed: review it, then change `PINNED`");
    }

    /// `source` without its `#[cfg(test)]` modules, each removed from its attribute through the
    /// `}` that closes it. rustfmt puts both at the start of a line, the `mod` line directly
    /// below the attribute. Any other `#[cfg(test)]`, or a module whose end is not found, is an
    /// error, so a file this cannot cut fails the test instead of being miscounted.
    fn strip_test_modules(source: &str) -> Result<String, String> {
        let mut kept = String::new();
        let mut rest = source;
        while let Some(at) = rest.find("#[cfg(test)]") {
            let line_no = source[..source.len() - rest.len() + at].lines().count() + 1;
            let module = rest[at..].strip_prefix("#[cfg(test)]\n").filter(|module| {
                let mod_line = module.lines().next().unwrap_or_default();
                (at == 0 || rest[..at].ends_with('\n')) && mod_line.starts_with("mod ") && mod_line.ends_with(" {")
            });
            let Some(module) = module else {
                return Err(format!(
                    "line {line_no}: a `#[cfg(test)]` that does not open a module at column 0"
                ));
            };
            let end = module
                .match_indices("\n}")
                .map(|(i, _)| i + 2)
                .find(|&end| matches!(module.as_bytes().get(end), None | Some(b'\n')));
            let Some(end) = end else {
                return Err(format!("line {line_no}: a test module with no closing `}}` at column 0"));
            };
            kept.push_str(&rest[..at]);
            rest = module[end..].strip_prefix('\n').unwrap_or(&module[end..]);
        }
        kept.push_str(rest);
        Ok(kept)
    }

    // The inputs keep their line breaks escaped: a `}` at the start of a line here would end
    // this module for the stripping it tests.
    #[test]
    fn code_after_a_test_module_is_kept() {
        let source = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\nfn b() {}\n";
        assert_eq!(strip_test_modules(source).unwrap(), "fn a() {}\nfn b() {}\n");
    }

    #[test]
    fn two_test_modules_are_both_removed() {
        let source = "fn a() {}\n#[cfg(test)]\nmod one {\n}\nfn b() {}\n#[cfg(test)]\nmod two {\n}\nfn c() {}\n";
        assert_eq!(strip_test_modules(source).unwrap(), "fn a() {}\nfn b() {}\nfn c() {}\n");
    }

    #[test]
    fn a_test_module_at_the_end_of_the_file_is_removed() {
        let source = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}";
        assert_eq!(strip_test_modules(source).unwrap(), "fn a() {}\n");
    }

    #[test]
    fn a_test_item_that_is_not_a_module_is_an_error() {
        assert!(strip_test_modules("#[cfg(test)]\nfn helper() {}\n").is_err());
    }

    #[test]
    fn a_test_module_without_its_closing_brace_is_an_error() {
        assert!(strip_test_modules("#[cfg(test)]\nmod tests {\n    fn t() {}\n").is_err());
    }

    fn rust_sources(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir).expect("a readable source directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_sources(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
        files
    }
}
