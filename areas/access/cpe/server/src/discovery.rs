//! Project discovery: find the project folders under the projects directory.
//!
//! A direct subfolder is a project when its name passes `cpe-core`'s id rule;
//! the name is the project id and the URL segment. What else makes it one
//! depends on the caller:
//!
//! - [`discover_for_activation`], for `Cpe::activate`: a regular `SHORTCODE` file naming the
//!   project whose deposited files and snapshot the ports serve. Its config and narrative come
//!   through the bytes port, never from the folder.
//! - [`discover_sql_folders`], the legacy path of `Cpe::from_sql_folders`: a `config.kdl`, read
//!   from the folder with its flat `narrative/` and its `data.sql`.
//!
//! Either way `assets/` is walked to any depth. Nothing here follows a
//! symbolic link, and every path read is built from a directory listing,
//! never from a request.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A project folder. The id is the directory name.
#[derive(Debug)]
pub(crate) struct DiscoveredProject {
    pub(crate) id: String,
    dir: PathBuf,
    /// The shortcode of the folder's `SHORTCODE` file; `None` for a folder
    /// [`discover_sql_folders`] found.
    pub(crate) shortcode: Option<String>,
    /// Why the folder, a project, is not loaded: another folder declares its
    /// shortcode.
    pub(crate) refusal: Option<String>,
}

/// A folder [`discover_for_activation`] does not take as a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Skipped {
    pub(crate) folder: String,
    pub(crate) reason: SkipReason,
}

/// Why a folder is not a project to activate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// The folder holds nothing named `SHORTCODE`.
    NoShortcode,
    /// `SHORTCODE` is a symbolic link, a directory or another file that is not
    /// regular.
    ShortcodeNotARegularFile,
    /// `SHORTCODE` cannot be read, or is not UTF-8.
    UnreadableShortcode,
    /// `SHORTCODE`, trimmed, fails `cpe_ports::is_shortcode`.
    MalformedShortcode(String),
    /// The folder's name fails the project id rule.
    NotAProjectId,
}

/// The projects [`discover_for_activation`] found, sorted by id, and the
/// folders it passed over, sorted by name.
#[derive(Debug)]
pub(crate) struct Discovery {
    pub(crate) projects: Vec<DiscoveredProject>,
    pub(crate) skipped: Vec<Skipped>,
}

/// What can go wrong reading the files of a discovered project.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SourcesError {
    #[error("project '{id}': {file} cannot be read: {source}")]
    Unreadable { id: String, file: String, source: std::io::Error },
    #[error("project '{id}': {file} is not valid UTF-8")]
    NotUtf8 { id: String, file: String },
}

/// Whether `path` is a regular file. A symbolic link is not, wherever it
/// points: `symlink_metadata` describes the link itself.
fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Whether `path` is a directory and not a symbolic link to one.
fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

/// The names of the regular files directly in `dir`, sorted. Empty when `dir`
/// is missing or is a symbolic link. A name that is not UTF-8 is left out: no
/// config value could name it.
fn file_names(dir: &Path) -> Vec<String> {
    if !is_real_dir(dir) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        // `DirEntry::file_type` does not follow a symbolic link.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Push every regular file under `dir`, at any depth, into `out` as
/// `<prefix>/<name>`. See [`DiscoveredProject::asset_files`] for why the walk
/// must stay recursive.
fn collect_files_recursively(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let (Ok(kind), Ok(name)) = (entry.file_type(), entry.file_name().into_string()) else {
            continue;
        };
        let relative = format!("{prefix}/{name}");
        if kind.is_file() {
            out.push(relative);
        } else if kind.is_dir() {
            collect_files_recursively(&entry.path(), &relative, out);
        }
    }
}

impl DiscoveredProject {
    fn read_utf8(&self, file: &str) -> Result<String, SourcesError> {
        let bytes = fs::read(self.dir.join(file)).map_err(|source| SourcesError::Unreadable {
            id: self.id.clone(),
            file: file.to_owned(),
            source,
        })?;
        String::from_utf8(bytes).map_err(|_| SourcesError::NotUtf8 { id: self.id.clone(), file: file.to_owned() })
    }

    /// Whether the folder holds a `data.sql`.
    pub(crate) fn has_data_sql(&self) -> bool {
        is_regular_file(&self.dir.join("data.sql"))
    }

    /// The content of `config.kdl`.
    pub(crate) fn config_source(&self) -> Result<String, SourcesError> {
        self.read_utf8("config.kdl")
    }

    /// The content of `data.sql`.
    pub(crate) fn data_sql(&self) -> Result<String, SourcesError> {
        self.read_utf8("data.sql")
    }

    /// `config.kdl` and `data.sql` for the router's tests, whose fixtures
    /// include a folder without a `data.sql`: it yields `""`, which loads as
    /// an empty store.
    #[cfg(test)]
    pub(crate) fn sources(&self) -> Result<(String, String), SourcesError> {
        let data = if self.has_data_sql() {
            self.data_sql()?
        } else {
            String::new()
        };
        Ok((self.config_source()?, data))
    }

    /// The bare file names in the project's `narrative/` folder (e.g.
    /// `["home.de.md", "home.md"]`), sorted. Empty when there is no such
    /// folder. Only direct children: narrative files are flat by convention.
    pub(crate) fn narrative_files(&self) -> Vec<String> {
        file_names(&self.dir.join("narrative"))
    }

    /// The `assets/`-prefixed relative paths of the files under the project's
    /// `assets/` folder (e.g. `["assets/cover.webp"]`), sorted. Empty when
    /// there is no such folder.
    ///
    /// The `assets/` prefix is deliberate (unlike `narrative_files`'s bare
    /// names): a `site.cover-image` value is a path relative to the project
    /// folder INCLUDING the `assets/` segment (`cover-image
    /// "assets/cover.webp"`), and `cpe-core`'s `CoverImageNotFound` check
    /// compares it with this list.
    ///
    /// The walk is **recursive**: a file in a subfolder is listed with its
    /// full relative path (`"assets/covers/x.webp"`). Neither the
    /// `cover-image` path-shape validator nor the `/{id}/_assets/{*path}`
    /// route rejects a nested path, and `CoverImageNotFound` treats this list
    /// as complete, so a flat walk would refuse a nested cover image that the
    /// route serves. Keep it recursive, or the validator and the route
    /// disagree.
    pub(crate) fn asset_files(&self) -> Vec<String> {
        let assets = self.dir.join("assets");
        if !is_real_dir(&assets) {
            return Vec::new();
        }
        let mut files = Vec::new();
        collect_files_recursively(&assets, "assets", &mut files);
        files.sort();
        files
    }

    /// Every file of [`Self::asset_files`] with its bytes. A file that cannot
    /// be read is logged and left out, so a request for it is answered 404.
    pub(crate) fn assets(&self) -> Vec<(String, Vec<u8>)> {
        self.asset_files()
            .into_iter()
            .filter_map(|relative| match fs::read(self.dir.join(&relative)) {
                Ok(bytes) => Some((relative, bytes)),
                Err(error) => {
                    tracing::warn!(project_id = self.id.as_str(), file = relative.as_str(), %error, "asset file cannot be read");
                    None
                }
            })
            .collect()
    }

    /// The UTF-8 content of every file of [`Self::narrative_files`], as
    /// `(bare file name, content)`.
    pub(crate) fn narrative_contents(&self) -> Result<Vec<(String, String)>, SourcesError> {
        self.narrative_files()
            .into_iter()
            .map(|name| {
                let content = self.read_utf8(&format!("narrative/{name}"))?;
                Ok((name, content))
            })
            .collect()
    }
}

/// The real directories directly under `root`, as `(name, path)`. A missing
/// or unreadable `root` holds none, and a name that is not UTF-8 is left out.
fn folders(root: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        // `DirEntry::file_type` does not follow a symbolic link, so a link to
        // a folder is not a project.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
        .collect()
}

/// The shortcode `SHORTCODE` in `dir` declares, trimmed of the white space
/// around it.
fn read_shortcode(dir: &Path) -> Result<String, SkipReason> {
    let path = dir.join("SHORTCODE");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(SkipReason::NoShortcode),
        Err(_) => return Err(SkipReason::UnreadableShortcode),
        Ok(meta) if !meta.is_file() => return Err(SkipReason::ShortcodeNotARegularFile),
        Ok(_) => {}
    }
    let text = fs::read(&path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(SkipReason::UnreadableShortcode)?;
    let shortcode = text.trim();
    if cpe_ports::is_shortcode(shortcode) {
        Ok(shortcode.to_owned())
    } else {
        Err(SkipReason::MalformedShortcode(shortcode.to_owned()))
    }
}

/// The projects under `root` that `Cpe::activate` loads: every folder with a
/// regular `SHORTCODE` file that holds a shortcode, and whose name is a project
/// id. Every other folder is skipped, with the reason.
///
/// Folders that declare one shortcode are each a project, refused with the
/// names of the others: "not ready" is not "not there", so their prefixes
/// answer 503 and not 404.
pub(crate) fn discover_for_activation(root: &Path) -> Discovery {
    let mut projects = Vec::new();
    let mut skipped = Vec::new();
    for (folder, dir) in folders(root) {
        let reason = match read_shortcode(&dir) {
            Ok(_) if !cpe_core::validate::is_valid_project_id(&folder) => SkipReason::NotAProjectId,
            Ok(shortcode) => {
                projects.push(DiscoveredProject { id: folder, dir, shortcode: Some(shortcode), refusal: None });
                continue;
            }
            Err(reason) => reason,
        };
        skipped.push(Skipped { folder, reason });
    }
    projects.sort_by(|a, b| a.id.cmp(&b.id));
    skipped.sort_by(|a, b| a.folder.cmp(&b.folder));

    let mut folders_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for project in &projects {
        let shortcode = project.shortcode.clone().unwrap_or_default();
        folders_of.entry(shortcode).or_default().push(project.id.clone());
    }
    for project in &mut projects {
        let shortcode = project.shortcode.as_deref().unwrap_or_default();
        let others: Vec<&str> = folders_of[shortcode]
            .iter()
            .map(String::as_str)
            .filter(|id| *id != project.id)
            .collect();
        if !others.is_empty() {
            project.refusal = Some(format!(
                "its SHORTCODE {shortcode} is also that of the folder(s) {}; no project is served under a shared \
                 shortcode",
                others.join(", ")
            ));
        }
    }
    Discovery { projects, skipped }
}

/// The projects under `root` that `Cpe::from_sql_folders` loads: every folder
/// with a regular `config.kdl` whose name is a project id, sorted by id.
pub(crate) fn discover_sql_folders(root: &Path) -> Vec<DiscoveredProject> {
    let mut projects: Vec<DiscoveredProject> = folders(root)
        .into_iter()
        .filter_map(|(id, dir)| {
            if !is_regular_file(&dir.join("config.kdl")) {
                return None;
            }
            if !cpe_core::validate::is_valid_project_id(&id) {
                // Such a folder is not a project, so its name is answered 404 and not 503.
                tracing::warn!(
                    folder = id.as_str(),
                    "a folder with a config.kdl has a name that is no project id; it is not served"
                );
                return None;
            }
            Some(DiscoveredProject { id, dir, shortcode: None, refusal: None })
        })
        .collect();
    projects.sort_by(|a, b| a.id.cmp(&b.id));
    projects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery")
    }

    fn ids(projects: &[DiscoveredProject]) -> Vec<&str> {
        projects.iter().map(|p| p.id.as_str()).collect()
    }

    fn project(id: &str) -> DiscoveredProject {
        discover_sql_folders(&fixture())
            .into_iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("the {id} fixture should be discovered"))
    }

    /// A folder with a `config.kdl`, which is all discovery asks of a project.
    fn make_project(root: &Path, name: &str) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("the temporary project folder is created");
        fs::write(dir.join("config.kdl"), "site {}\n").expect("config.kdl is written");
        dir
    }

    #[test]
    fn discovers_dirs_with_config() {
        let projects = discover_sql_folders(&fixture());
        let found = ids(&projects);
        assert!(found.contains(&"alpha"));
        assert!(found.contains(&"gamma"));
    }

    #[test]
    fn skips_dirs_without_config() {
        let projects = discover_sql_folders(&fixture());
        assert!(!ids(&projects).contains(&"beta"), "beta has no config.kdl");
    }

    #[test]
    fn result_is_sorted_by_id() {
        let projects = discover_sql_folders(&fixture());
        let found = ids(&projects);
        let mut sorted = found.clone();
        sorted.sort();
        assert!(found.len() > 1, "the ordering check needs several projects");
        assert_eq!(found, sorted, "ids should be lexicographically ordered");
    }

    #[test]
    fn a_missing_root_holds_no_project() {
        assert!(discover_sql_folders(&fixture().join("no-such-folder")).is_empty());
    }

    #[test]
    fn a_folder_whose_name_fails_the_id_rule_is_not_a_project() {
        let root = tempfile::tempdir().expect("a temporary directory");
        make_project(root.path(), "good-one");
        for name in ["_static", "Upper", ".hidden", "with space", "-leading"] {
            make_project(root.path(), name);
        }
        assert_eq!(ids(&discover_sql_folders(root.path())), vec!["good-one"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_to_a_project_folder_is_not_followed() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let outside = tempfile::tempdir().expect("a second temporary directory");
        let target = make_project(outside.path(), "elsewhere");
        make_project(root.path(), "real");
        std::os::unix::fs::symlink(&target, root.path().join("linked")).expect("the link is created");
        assert_eq!(ids(&discover_sql_folders(root.path())), vec!["real"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_config_does_not_make_a_project() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let real = make_project(root.path(), "real");
        let hollow = root.path().join("hollow");
        fs::create_dir(&hollow).expect("the folder is created");
        std::os::unix::fs::symlink(real.join("config.kdl"), hollow.join("config.kdl")).expect("the link is created");
        assert_eq!(ids(&discover_sql_folders(root.path())), vec!["real"]);
    }

    #[cfg(unix)]
    #[test]
    fn linked_assets_and_narrative_files_are_not_listed() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let secret = root.path().join("secret.txt");
        fs::write(&secret, "secret").expect("the outside file is written");
        let dir = make_project(root.path(), "real");
        for sub in ["assets", "narrative"] {
            fs::create_dir(dir.join(sub)).expect("the subfolder is created");
            fs::write(dir.join(sub).join("kept.md"), "kept").expect("the regular file is written");
            std::os::unix::fs::symlink(&secret, dir.join(sub).join("linked.md")).expect("the file link is created");
        }
        std::os::unix::fs::symlink(root.path(), dir.join("assets").join("up")).expect("the folder link is created");

        let found = discover_sql_folders(root.path());
        let real = &found[0];
        assert_eq!(real.asset_files(), vec!["assets/kept.md".to_owned()]);
        assert_eq!(real.narrative_files(), vec!["kept.md".to_owned()]);
        assert_eq!(real.assets().len(), 1);
    }

    #[test]
    fn sources_reads_config_and_data() {
        let alpha = project("alpha");
        let config = alpha.config_source().expect("alpha's config.kdl is valid UTF-8");
        let data = alpha.data_sql().expect("alpha's data.sql is valid UTF-8");
        assert!(!config.is_empty(), "alpha config.kdl should have content");
        assert!(!data.is_empty(), "alpha data.sql should have content");
    }

    #[test]
    fn a_missing_data_sql_is_reported_and_not_read_as_empty() {
        let gamma = project("gamma");
        assert!(!gamma.has_data_sql());
        assert!(!gamma.config_source().expect("gamma's config.kdl is valid UTF-8").is_empty());
        let error = gamma.data_sql().expect_err("gamma has no data.sql");
        assert!(
            error.to_string().starts_with("project 'gamma': data.sql cannot be read"),
            "{error}"
        );
    }

    #[test]
    fn sources_reports_a_config_that_is_not_utf8() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let dir = make_project(root.path(), "broken");
        fs::write(dir.join("config.kdl"), [0xff, 0xfe]).expect("config.kdl is overwritten");
        let error = discover_sql_folders(root.path())[0]
            .config_source()
            .expect_err("invalid UTF-8 is an error");
        assert_eq!(error.to_string(), "project 'broken': config.kdl is not valid UTF-8");
    }

    #[test]
    fn narrative_files_returns_empty_when_no_narrative_dir() {
        let files = project("alpha").narrative_files();
        assert!(files.is_empty(), "no narrative dir → empty list, got: {files:?}");
    }

    #[test]
    fn asset_files_returns_empty_when_no_assets_dir() {
        let files = project("gamma").asset_files();
        assert!(files.is_empty(), "no assets dir → empty list, got: {files:?}");
    }

    #[test]
    fn asset_files_lists_real_filenames_when_assets_dir_exists() {
        assert_eq!(
            project("alpha").asset_files(),
            vec!["assets/cover.webp".to_owned(), "assets/covers/nested.webp".to_owned()],
            "expected the fixture's asset files in sorted order"
        );
    }

    /// The walk must descend into subfolders of `assets/`; see
    /// [`DiscoveredProject::asset_files`].
    #[test]
    fn asset_files_includes_nested_subdirectory_files() {
        let files = project("alpha").asset_files();
        assert!(
            files.contains(&"assets/covers/nested.webp".to_owned()),
            "nested asset must be listed with its full relative path, got: {files:?}"
        );
    }

    #[test]
    fn assets_reads_the_bytes_of_every_listed_file() {
        let alpha = project("alpha");
        let assets = alpha.assets();
        let names: Vec<&str> = assets.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["assets/cover.webp", "assets/covers/nested.webp"]);
        let on_disk = fs::read(fixture().join("alpha/assets/cover.webp")).expect("the fixture file is readable");
        assert_eq!(assets[0].1, on_disk);
    }

    #[test]
    fn narrative_contents_returns_empty_when_no_narrative_dir() {
        let contents = project("alpha")
            .narrative_contents()
            .expect("no narrative dir should return Ok, not error");
        assert!(contents.is_empty(), "no narrative dir → empty contents, got: {contents:?}");
    }

    #[test]
    fn narrative_contents_reads_each_file_by_its_bare_name() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let dir = make_project(root.path(), "real");
        fs::create_dir(dir.join("narrative")).expect("the narrative folder is created");
        fs::write(dir.join("narrative/home.md"), "# Home\n").expect("the narrative file is written");
        fs::create_dir(dir.join("narrative/nested")).expect("a nested folder is created");
        fs::write(dir.join("narrative/nested/deep.md"), "deep").expect("the nested file is written");

        let contents = discover_sql_folders(root.path())[0]
            .narrative_contents()
            .expect("the narrative is UTF-8");
        assert_eq!(contents, vec![("home.md".to_owned(), "# Home\n".to_owned())]);
    }

    /// A folder whose `SHORTCODE` holds `shortcode`.
    fn shortcode_folder(root: &Path, name: &str, shortcode: &str) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("the temporary project folder is created");
        fs::write(dir.join("SHORTCODE"), shortcode).expect("SHORTCODE is written");
        dir
    }

    fn skipped(folder: &str, reason: SkipReason) -> Skipped {
        Skipped { folder: folder.to_owned(), reason }
    }

    #[test]
    fn discovery_for_activation_finds_a_folder_with_a_shortcode_and_carries_it() {
        let root = tempfile::tempdir().expect("a temporary directory");
        shortcode_folder(root.path(), "things", " 0803\n");
        make_project(root.path(), "legacy");

        let found = discover_for_activation(root.path());
        let projects: Vec<_> = found
            .projects
            .iter()
            .map(|project| (project.id.as_str(), project.shortcode.as_deref(), project.refusal.as_deref()))
            .collect();
        assert_eq!(projects, [("things", Some("0803"), None)]);
        assert_eq!(found.skipped, [skipped("legacy", SkipReason::NoShortcode)]);
        assert_eq!(
            ids(&discover_sql_folders(root.path())),
            ["legacy"],
            "the legacy path reads config.kdl"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discovery_for_activation_skips_a_folder_without_a_shortcode_and_says_why() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let real = shortcode_folder(root.path(), "real", "0001");
        fs::create_dir(root.path().join("none")).expect("the folder is created");
        let linked = root.path().join("linked");
        fs::create_dir(&linked).expect("the folder is created");
        std::os::unix::fs::symlink(real.join("SHORTCODE"), linked.join("SHORTCODE")).expect("the link is created");
        shortcode_folder(root.path(), "short", "803");
        shortcode_folder(root.path(), "lower", "0ab1");
        fs::create_dir_all(root.path().join("folder/SHORTCODE")).expect("a folder named SHORTCODE is created");
        fs::create_dir(root.path().join("bytes")).expect("the folder is created");
        fs::write(root.path().join("bytes/SHORTCODE"), [0xff, 0xfe]).expect("SHORTCODE is written");
        shortcode_folder(root.path(), "Upper", "0002");

        let found = discover_for_activation(root.path());
        assert_eq!(ids(&found.projects), ["real"]);
        assert_eq!(
            found.skipped,
            [
                skipped("Upper", SkipReason::NotAProjectId),
                skipped("bytes", SkipReason::UnreadableShortcode),
                skipped("folder", SkipReason::ShortcodeNotARegularFile),
                skipped("linked", SkipReason::ShortcodeNotARegularFile),
                skipped("lower", SkipReason::MalformedShortcode("0ab1".to_owned())),
                skipped("none", SkipReason::NoShortcode),
                skipped("short", SkipReason::MalformedShortcode("803".to_owned())),
            ]
        );
    }

    #[test]
    fn folders_that_share_a_shortcode_are_each_refused_naming_the_others() {
        let root = tempfile::tempdir().expect("a temporary directory");
        for (name, shortcode) in [
            ("first", "0803"),
            ("second", "0803\n"),
            ("third", "0803"),
            ("other", "0001"),
        ] {
            shortcode_folder(root.path(), name, shortcode);
        }

        let found = discover_for_activation(root.path());
        let refusal = |id: &str| {
            let project = found
                .projects
                .iter()
                .find(|project| project.id == id)
                .expect("the folder is a project");
            project.refusal.clone()
        };
        assert_eq!(refusal("other"), None);
        for (id, others) in [
            ("first", ["second", "third"]),
            ("second", ["first", "third"]),
            ("third", ["first", "second"]),
        ] {
            let refusal = refusal(id).unwrap_or_else(|| panic!("{id} is refused"));
            assert!(refusal.contains("0803"), "{refusal}");
            for other in others {
                assert!(refusal.contains(other), "{id}'s refusal names {other}: {refusal}");
            }
            assert!(
                !refusal.contains(&format!(" {id}")),
                "{id}'s refusal names the others: {refusal}"
            );
        }
    }
}
