//! The library reads nothing but the snapshot it is given: no adapter, no file, no environment,
//! no clock. Cargo holds only part of this: the adapter's crate is a dev-dependency, which a
//! test module of the library could still name, and the standard library is always there. A
//! search of the sources for the strings of [`BINARY_ONLY`] holds it, as far as a search can.

use std::fs;
use std::path::Path;

/// The crate's sources.
const SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// What no source of the library may hold, in code or in a comment: the adapter's crate; two ways
/// to read a file, and the modules of the standard library that reach outside the process or
/// read its clock; the path of the folder of the one project that has a hook (a caller names a
/// project by its folder name).
/// An `include_str!` is not among them: it reads when the crate is compiled.
const BINARY_ONLY: [&str; 9] = [
    "sync_store",
    "read_to_string",
    "File::open",
    "std::fs",
    "std::env",
    "std::net",
    "std::process",
    "std::time",
    "projects/incunabula",
];

/// (path below [`SRC`] with `/` between its parts, text) of every `.rs` file in `dir` and
/// below it.
fn sources(dir: &Path, found: &mut Vec<(String, String)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|error| panic!("{}: {error}", dir.display())).path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let below = path.strip_prefix(SRC).expect("the file is below src/");
            let parts: Vec<_> = below.components().map(|part| part.as_os_str().to_string_lossy()).collect();
            let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            found.push((parts.join("/"), text));
        }
    }
}

/// The names a `use` tree names directly below its root: of `fs::File` that is `fs`, of
/// `{fs as files, io::{self, Read}}` they are `fs` and `io`.
fn first_segments(tree: &str) -> Vec<&str> {
    let tree = tree.trim();
    let items = match tree.strip_prefix('{') {
        Some(group) => {
            let group = group.strip_suffix('}').unwrap_or(group);
            let mut depth = 0_usize;
            let split = group.split(move |c: char| {
                match c {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                c == ',' && depth == 0
            });
            split.collect()
        }
        None => vec![tree],
    };
    fn first(item: &str) -> &str {
        let item = item.trim();
        let is_name = |c: char| c.is_alphanumeric() || c == '_';
        &item[..item.find(|c| !is_name(c)).unwrap_or(item.len())]
    }
    items.into_iter().map(first).collect()
}

/// The path of every `use` in `text`, without a leading `::`: what stands between `use ` and
/// the `;`.
fn use_paths(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("use ").map(|(at, _)| {
        let statement = &text[at + "use ".len()..];
        let statement = &statement[..statement.find(';').unwrap_or(statement.len())];
        let statement = statement.trim();
        statement.strip_prefix("::").unwrap_or(statement)
    })
}

/// The names `text` gives the standard library besides `std`: what a `use` renames it to, as
/// `use std as base;` and `use std::{self as base};` do.
fn std_aliases(text: &str) -> Vec<&str> {
    fn renamed<'a>(item: &'a str, from: &str) -> Option<&'a str> {
        let rest = item.trim().strip_prefix(from)?;
        let name = rest.trim_start().strip_prefix("as ")?.trim();
        (rest.starts_with(char::is_whitespace) && !name.is_empty()).then_some(name)
    }
    let mut aliases = Vec::new();
    for path in use_paths(text) {
        aliases.extend(renamed(path, "std"));
        if let Some(group) = path.strip_prefix("std::{") {
            let items = group.strip_suffix('}').unwrap_or(group).split(',');
            aliases.extend(items.filter_map(|item| renamed(item, "self")));
        }
    }
    aliases
}

/// Whether `text` names `root::module` as a path, and not as the end of a longer name or the
/// start of one.
fn names_path(text: &str, root: &str, module: &str) -> bool {
    let path = format!("{root}::{module}");
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(&path).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + path.len()..].chars().next();
        !before.is_some_and(is_name) && !after.is_some_and(is_name)
    })
}

/// Whether `text` names `module` directly below the standard library, however it is spelt: in a
/// `use` alone, in a group or under another name, or below another name a `use` gave `std`.
fn uses_std_module(text: &str, module: &str) -> bool {
    let aliases = std_aliases(text);
    let roots = ["std"].into_iter().chain(aliases.iter().copied());
    let in_a_use = use_paths(text).any(|path| {
        roots.clone().any(|root| {
            let tree = path.strip_prefix(root).and_then(|rest| rest.strip_prefix("::"));
            tree.is_some_and(|tree| first_segments(tree).contains(&module))
        })
    });
    in_a_use || aliases.iter().any(|alias| names_path(text, alias, module))
}

/// (file, string) for every one of [`BINARY_ONLY`] that a file of `sources` holds. A module of
/// the standard library is held also where a `use` names it in a group or under another name,
/// which the module's path as one string does not show.
fn held<'a>(sources: &[&'a (String, String)]) -> Vec<(&'a str, &'static str)> {
    let mut held = Vec::new();
    for (file, text) in sources {
        for string in BINARY_ONLY {
            let used = string.strip_prefix("std::").is_some_and(|module| uses_std_module(text, module));
            if used || text.contains(string) {
                held.push((file.as_str(), string));
            }
        }
    }
    held.sort();
    held
}

#[test]
fn test_library_sources_hold_nothing_that_only_the_binary_may() {
    let mut all = Vec::new();
    sources(Path::new(SRC), &mut all);
    let library: Vec<_> = all.iter().collect();

    let files: Vec<&str> = library.iter().map(|(file, _)| file.as_str()).collect();
    assert!(files.contains(&"projects/incunabula/catalogue.rs"), "{files:?}");
    assert_eq!(
        held(&library),
        [],
        "the library reads only its snapshot: read in the caller and pass the result in"
    );
}

/// The mapping's modules import the generic ones, so a generic module imports none of the
/// mapping's: what both need stands below both.
#[test]
fn test_generic_modules_import_nothing_of_the_mapping() {
    let mut all = Vec::new();
    sources(Path::new(SRC), &mut all);
    let generic: Vec<_> = all.iter().filter(|(file, _)| file.starts_with("generic/")).collect();
    assert!(generic.len() > 1, "the generic modules are searched");

    let importing: Vec<&str> = generic
        .iter()
        .filter(|(_, text)| text.contains("crate::mapping"))
        .map(|(file, _)| file.as_str())
        .collect();
    assert_eq!(importing, Vec::<&str>::new());
}

/// Where the hook modules are, as a path below [`SRC`].
const HOOK_MODULES: &str = "projects/";

/// What a hook module may name of its own crate: the seam, the rule names, the builder's
/// helpers that read nothing but their arguments, and the other files of its project.
const SEAM: [&str; 7] = [
    "crate::hook::",
    "crate::error::Rule",
    "crate::generic::text::",
    "crate::generic::slug::",
    "crate::generic::order::",
    "crate::generic::dates::",
    "crate::projects::",
];

/// Where a `use` of a hook module may start: the standard library and its own crate. The
/// port's types and the mapping's are not a hook's to name.
const USE_ROOTS: [&str; 4] = ["std::", "crate::", "super::", "self::"];

/// What `text`, the source of a hook module, holds beyond the seam: each path into its crate
/// that is none of [`SEAM`], wherever it stands; each `use` that starts outside [`USE_ROOTS`];
/// and the archive's ontology host, which only the project's `remodel` block names.
fn beyond_the_seam(text: &str) -> Vec<String> {
    let is_path = |c: char| c.is_alphanumeric() || c == '_' || c == ':';
    let mut found = Vec::new();
    for (at, _) in text.match_indices("crate::") {
        let path = &text[at..];
        if !SEAM.iter().any(|allowed| path.starts_with(allowed)) {
            let end = path.find(|c| !is_path(c)).unwrap_or(path.len());
            found.push(path[..end].to_string());
        }
    }
    for line in text.lines() {
        let line = line.trim_start();
        let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
        let Some(path) = line.strip_prefix("use ") else {
            continue;
        };
        if !USE_ROOTS.iter().any(|root| path.starts_with(root)) {
            let end = path.find(|c| !is_path(c)).unwrap_or(path.len());
            found.push(path[..end].to_string());
        }
    }
    if text.contains("knora.org") {
        found.push("knora.org".to_string());
    }
    found
}

/// A hook reaches the draft only through the seam, and names no IRI of the archive's ontology.
/// The compiler holds the first for the draft's fields; this search holds it for the rest of
/// the crate, as far as a search can.
#[test]
fn test_hook_modules_hold_nothing_beyond_the_seam() {
    let mut all = Vec::new();
    sources(Path::new(SRC), &mut all);
    let hook_modules = all.iter().filter(|(file, _)| file.starts_with(HOOK_MODULES));

    for (file, text) in hook_modules {
        assert_eq!(beyond_the_seam(text), Vec::<String>::new(), "{file}");
    }
}

/// The control: a text within the seam holds nothing beyond it, and each kind of step beyond
/// it is found.
#[test]
fn test_beyond_the_seam_finds_a_path_a_use_and_the_ontology_host() {
    let within = "\
use std::collections::BTreeMap;
use crate::error::Rule;
use crate::generic::text::py_strip;
use crate::hook::{Draft, Mapped, Refusals};
use super::sides;
pub(crate) use self::scans::drop_blank;
";
    let beyond = "\
use cpe_ports::Resource;
use crate::draft::Draft;
use crate::{hook::Draft, store::Store};
fn names() { crate::mapping::values::names(node); }
const PAGE: &str = \"http://www.knora.org/ontology/0803/incunabula#Page\";
";

    assert_eq!(beyond_the_seam(within), Vec::<String>::new());
    assert_eq!(
        beyond_the_seam(beyond),
        [
            "crate::draft::Draft",
            "crate::",
            "crate::mapping::values::names",
            "cpe_ports::Resource",
            "knora.org"
        ]
    );
}

/// The nouns of the one project that has a hook: its name and its classes.
const PROJECT_NOUNS: [&str; 6] = ["incunabula", "Incunabula", "Book", "Page", "Band", "BorderStrip"];

/// Where a project's nouns may stand, as paths below [`SRC`]: the hook modules, the table of
/// hooks, and the rule names, some of which carry a class.
const NOT_GENERIC: [&str; 3] = [HOOK_MODULES, "hooks.rs", "error.rs"];

/// `text` up to its first test module: a line that is `#[cfg(test)]` alone, followed by a
/// `mod`. The attribute anywhere else, in a comment or on another item, hides nothing.
fn above_tests(text: &str) -> &str {
    let mut at = 0;
    let mut lines = text.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let opens_module = lines.peek().is_some_and(|next| next.trim_start().starts_with("mod "));
        if line.trim() == "#[cfg(test)]" && opens_module {
            return &text[..at];
        }
        at += line.len();
    }
    text
}

/// (line, noun) for every one of [`PROJECT_NOUNS`] that `text` holds as a whole word, in code
/// or in a comment. A word is letters, digits and underscores; the search minds the case.
fn project_nouns(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for word in line.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if let Some(noun) = PROJECT_NOUNS.iter().find(|noun| **noun == word) {
                found.push((index + 1, *noun));
            }
        }
    }
    found
}

/// Generic code names no project: what a project is called and what its classes are called
/// are the mapping's to say, and the hook's.
#[test]
fn test_generic_sources_name_no_project_above_their_tests() {
    let mut all = Vec::new();
    sources(Path::new(SRC), &mut all);
    let generic: Vec<_> = all
        .iter()
        .filter(|(file, _)| !NOT_GENERIC.iter().any(|place| file.starts_with(place)))
        .collect();

    let files: Vec<&str> = generic.iter().map(|(file, _)| file.as_str()).collect();
    for searched in ["lib.rs", "sql.rs", "generic/text.rs"] {
        assert!(files.contains(&searched), "{searched} is searched: {files:?}");
    }
    for (file, text) in generic {
        let text = above_tests(text);
        assert_eq!(
            project_nouns(text),
            [],
            "{file}: a project's name or class in generic code, by line; it belongs in the \
             project's remodel block or its hook module"
        );
    }
}

/// The control: each noun is found as a whole word, in code and in a comment; a longer word,
/// another case and a test module are not searched; the test attribute in a comment or on a
/// function cuts nothing.
#[test]
fn test_project_nouns_finds_whole_words_above_the_tests() {
    let sample = "\
// A Page of the incunabula project.
const CLASS: &str = \"BorderStrip\";
fn band(book: Book) -> Band { Incunabula::of(book) }
struct Pages; struct PageRef; fn page_of() {} // Books, Bands, BorderStrips, incunabula_x, sub_Page #[cfg(test)]
#[cfg(test)]
fn last(page: src::Page) {}
#[cfg(test)]
mod tests { struct Book; }
";

    let above = above_tests(sample);
    assert_eq!(
        project_nouns(above),
        [
            (1, "Page"),
            (1, "incunabula"),
            (2, "BorderStrip"),
            (3, "Book"),
            (3, "Band"),
            (3, "Incunabula"),
            (6, "Page"),
        ]
    );
}

/// The control of the search for [`BINARY_ONLY`]: each string is found in a text that has it.
#[test]
fn test_held_finds_every_string_of_the_list() {
    let sample = ("sample.rs".to_string(), BINARY_ONLY.join("\n"));

    let found: Vec<&str> = held(&[&sample]).into_iter().map(|(_, string)| string).collect();

    let mut all = BINARY_ONLY;
    all.sort_unstable();
    assert_eq!(found, all);
}

/// The control of the modules a `use` names without spelling their path: in a group, under
/// another name, behind a leading `::`, and below another name a `use` gave `std` itself. A
/// module of another root and a longer name are not found.
#[test]
fn test_held_finds_a_module_of_the_standard_library_named_in_a_group_or_under_another_name() {
    let cases: [(&str, &[&str]); 9] = [
        ("use std as base;\nfn read() { base::fs::read(\"x\"); }\n", &["std::fs"]),
        (
            "use std::{self as base};\nfn now() { base::time::Instant::now(); }\n",
            &["std::time"],
        ),
        ("use ::std as base;\nuse base::{env, net as wire};\n", &["std::env", "std::net"]),
        (
            "use std as base;\nfn show() { database::fs(); base::fsync(); base::fmt::format(); }\n",
            &[],
        ),
        ("use std::{fs as files};\n", &["std::fs"]),
        (
            "use std::{\n    collections::BTreeMap,\n    env,\n    time::{Duration, Instant},\n};\n",
            &["std::env", "std::time"],
        ),
        ("pub(crate) use ::std::{net as wire, process};\n", &["std::net", "std::process"]),
        ("use std::{io::{self, Read}, fs::{self, File}};\n", &["std::fs"]),
        ("use std::{collections::BTreeMap, fmt, timeout};\nuse crate::{fs, env};\n", &[]),
    ];

    for (text, expected) in cases {
        let sample = ("sample.rs".to_string(), text.to_string());
        let found: Vec<&str> = held(&[&sample]).into_iter().map(|(_, string)| string).collect();
        assert_eq!(found, expected, "{text:?}");
    }
}
