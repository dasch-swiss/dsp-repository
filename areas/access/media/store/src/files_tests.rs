use std::path::{Path, PathBuf};
use std::{fs, io};

use cpe_ports::{DepositedFiles, FilesError};
use shared_stand_in::{Generation, Provenance, ReadyToken};
use tempfile::TempDir;

use crate::LiveDepositedFiles;

/// The generation [`deposit`] writes.
const GENERATION: &str = "20261120T010203Z-9f3a1c2e";
/// Another generation, for a project that holds two.
const OLDER: &str = "20261119T010155Z-0b1d2e3f";

/// Writes `contents` at `<dir>/<path>`, creating the directories above it.
fn write(dir: &Path, path: &str, contents: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().expect("a fixture path has a parent")).expect("create the fixture directories");
    fs::write(path, contents).expect("write the fixture");
}

/// Writes the Ready token of `shortcode` below `root`, naming `generation`.
fn write_token(root: &Path, shortcode: &str, generation: &str) {
    let token = ReadyToken {
        provenance: Provenance {
            generation: Generation::parse(generation).expect("a generation name"),
            as_of: "2026-11-20T01:02:03Z".to_string(),
            server: "prod".to_string(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: format!("sha256:{}", "0".repeat(64)),
        },
        previous_generation: None,
    };
    write(
        root,
        &format!("{shortcode}/.ready"),
        &String::from_utf8(token.to_bytes()).expect("UTF-8"),
    );
}

/// The directory of `shortcode`'s [`GENERATION`] below `root`.
fn generation(root: &Path, shortcode: &str) -> PathBuf {
    root.join(shortcode).join(GENERATION)
}

/// Deposits `contents` as `name` in `shortcode`'s [`GENERATION`] below `root`, and a token naming
/// it.
fn deposit(root: &Path, shortcode: &str, name: &str, contents: &str) {
    write(&generation(root, shortcode), name, contents);
    write_token(root, shortcode, GENERATION);
}

fn assert_unknown_project(result: Result<Vec<u8>, FilesError>, expected: &str) {
    assert!(
        matches!(&result, Err(FilesError::UnknownProject { shortcode }) if shortcode == expected),
        "{result:?}"
    );
}

fn assert_unknown_file(result: Result<Vec<u8>, FilesError>, expected: &str) {
    assert!(
        matches!(&result, Err(FilesError::UnknownFile { shortcode, name }) if shortcode == "0803" && name == expected),
        "{result:?}"
    );
}

/// The `io::Error` behind an `Unavailable`; panics on any other result.
fn expect_unavailable(result: Result<Vec<u8>, FilesError>) -> io::Error {
    let Err(FilesError::Unavailable(source)) = result else {
        panic!("expected Unavailable, got {result:?}")
    };
    *source.downcast::<io::Error>().expect("the source is an io::Error")
}

#[test]
fn test_file_config_kdl_served_whole() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "remodel {\n}\n");

    let bytes = LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl").expect("served");

    assert_eq!(bytes, b"remodel {\n}\n");
}

#[test]
fn test_file_nested_name_served() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "narrative/home.de.md", "# Willkommen\n");

    let bytes = LiveDepositedFiles::new(dir.path())
        .file("0803", "narrative/home.de.md")
        .expect("served");

    assert_eq!(bytes, b"# Willkommen\n");
}

#[test]
fn test_file_comes_from_the_generation_the_token_names() {
    let dir = TempDir::new().expect("a temporary directory");
    write(&dir.path().join("0803").join(OLDER), "config.kdl", "older");
    write(&generation(dir.path(), "0803"), "config.kdl", "newer");
    write(dir.path(), "0803/config.kdl", "beside the generations");
    write_token(dir.path(), "0803", OLDER);

    let bytes = LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl").expect("served");

    assert_eq!(bytes, b"older");
}

#[test]
fn test_file_shortcode_without_directory_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0804", "config.kdl"), "0804");
}

#[test]
fn test_file_project_without_token_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");
    write(&generation(dir.path(), "0803"), "config.kdl", "");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"), "0803");
}

#[test]
fn test_file_refused_shortcode_unknown_project_even_when_its_project_exists() {
    let root = TempDir::new().expect("a temporary directory");
    let dir = root.path().join("media");
    deposit(&dir, "0ab1", "config.kdl", "inside");
    deposit(root.path(), "beside", "config.kdl", "beside");
    let files = LiveDepositedFiles::new(&dir);

    for shortcode in ["0ab1", "../beside", ".."] {
        assert_unknown_project(files.file(shortcode, "config.kdl"), shortcode);
    }
}

#[test]
fn test_file_refused_shortcode_and_name_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0ab1", "../x"), "0ab1");
}

#[test]
fn test_file_missing_file_in_known_project_unknown_file() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");
    let files = LiveDepositedFiles::new(dir.path());

    assert_unknown_file(files.file("0803", "curation.csv"), "curation.csv");
    assert_unknown_file(files.file("0803", "narrative/home.md"), "narrative/home.md");
}

#[test]
fn test_file_refused_name_unknown_file_even_when_path_exists() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");
    deposit(dir.path(), "0804", "config.kdl", "another project's");
    let generations = format!("../../0804/{GENERATION}/config.kdl");

    for name in ["../0804/config.kdl", generations.as_str()] {
        assert_unknown_file(LiveDepositedFiles::new(dir.path()).file("0803", name), name);
    }
}

/// `.ready` is never served: `is_file_name` refuses a dot-named file, and the token lies outside
/// the generation, so no name reaches it.
#[test]
fn test_file_token_never_served() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");
    write(&generation(dir.path(), "0803"), ".ready", "a copy inside the generation");
    let files = LiveDepositedFiles::new(dir.path());

    for name in [".ready", "../.ready"] {
        assert_unknown_file(files.file("0803", name), name);
    }
}

#[test]
fn test_file_token_naming_missing_generation_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    write(&dir.path().join("0803").join(OLDER), "config.kdl", "");
    write_token(dir.path(), "0803", GENERATION);

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[test]
fn test_file_malformed_token_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");
    write(dir.path(), "0803/.ready", "{}\n");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_generation_directory_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    write(&dir.path().join("0803").join(OLDER), "config.kdl", "");
    write_token(dir.path(), "0803", GENERATION);
    std::os::unix::fs::symlink(dir.path().join("0803").join(OLDER), generation(dir.path(), "0803")).expect("symlink");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

/// The token is refused, and with it the project: `Unavailable`, not `UnknownProject`.
#[cfg(unix)]
#[test]
fn test_file_symlinked_project_directory_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "elsewhere", "config.kdl", "");
    std::os::unix::fs::symlink(dir.path().join("elsewhere"), dir.path().join("0803")).expect("symlink");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_file_unknown_file() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "real.kdl", "");
    let generation = generation(dir.path(), "0803");
    std::os::unix::fs::symlink(generation.join("real.kdl"), generation.join("config.kdl")).expect("symlink");

    assert_unknown_file(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"), "config.kdl");
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_intermediate_directory_unknown_file_even_when_target_holds_file() {
    let root = TempDir::new().expect("a temporary directory");
    let dir = root.path().join("media");
    deposit(&dir, "0803", "config.kdl", "");
    write(root.path(), "elsewhere/home.md", "outside the directory");
    std::os::unix::fs::symlink(root.path().join("elsewhere"), generation(&dir, "0803").join("narrative"))
        .expect("symlink");

    assert_unknown_file(
        LiveDepositedFiles::new(&dir).file("0803", "narrative/home.md"),
        "narrative/home.md",
    );
}

/// The token's reader refuses a file in place of the project directory.
#[test]
fn test_file_regular_file_under_shortcode_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803", "not a directory");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[test]
fn test_file_directory_under_file_name_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl/inner", "");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[cfg(unix)]
#[test]
fn test_file_unreadable_generation_directory_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "");
    let generation = generation(dir.path(), "0803");
    fs::set_permissions(&generation, fs::Permissions::from_mode(0o000)).expect("chmod");
    // Root reads the directory whatever its mode, so the refusal cannot be observed.
    if fs::read_dir(&generation).is_ok() {
        fs::set_permissions(&generation, fs::Permissions::from_mode(0o755)).expect("chmod back");
        return;
    }

    let result = LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl");
    fs::set_permissions(&generation, fs::Permissions::from_mode(0o755)).expect("chmod back");

    assert_eq!(expect_unavailable(result).kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn test_file_changed_file_served_changed() {
    let dir = TempDir::new().expect("a temporary directory");
    deposit(dir.path(), "0803", "config.kdl", "first");
    let files = LiveDepositedFiles::new(dir.path());
    assert_eq!(files.file("0803", "config.kdl").expect("served"), b"first");

    write(&generation(dir.path(), "0803"), "config.kdl", "second");

    assert_eq!(files.file("0803", "config.kdl").expect("served"), b"second");
}
