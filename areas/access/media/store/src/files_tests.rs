use std::path::Path;
use std::{fs, io};

use cpe_ports::{DepositedFiles, FilesError};
use tempfile::TempDir;

use crate::LiveDepositedFiles;

/// Writes `contents` at `<dir>/<path>`, creating the directories above it.
fn write(dir: &Path, path: &str, contents: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().expect("a fixture path has a parent")).expect("create the fixture directories");
    fs::write(path, contents).expect("write the fixture");
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
    write(dir.path(), "0803/config.kdl", "remodel {\n}\n");

    let bytes = LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl").expect("served");

    assert_eq!(bytes, b"remodel {\n}\n");
}

#[test]
fn test_file_nested_name_served() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/narrative/home.de.md", "# Willkommen\n");

    let bytes = LiveDepositedFiles::new(dir.path())
        .file("0803", "narrative/home.de.md")
        .expect("served");

    assert_eq!(bytes, b"# Willkommen\n");
}

#[test]
fn test_file_shortcode_without_directory_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/config.kdl", "");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0804", "config.kdl"), "0804");
}

#[test]
fn test_file_refused_shortcode_unknown_project_even_when_directory_exists() {
    let root = TempDir::new().expect("a temporary directory");
    let dir = root.path().join("media");
    write(&dir, "0ab1/config.kdl", "inside");
    write(root.path(), "beside/config.kdl", "beside");
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
    write(dir.path(), "0803/config.kdl", "");
    let files = LiveDepositedFiles::new(dir.path());

    assert_unknown_file(files.file("0803", "curation.csv"), "curation.csv");
    assert_unknown_file(files.file("0803", "narrative/home.md"), "narrative/home.md");
}

#[test]
fn test_file_refused_name_unknown_file_even_when_path_exists() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/config.kdl", "");
    write(dir.path(), "0804/config.kdl", "another project's");

    assert_unknown_file(
        LiveDepositedFiles::new(dir.path()).file("0803", "../0804/config.kdl"),
        "../0804/config.kdl",
    );
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_project_directory_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "elsewhere/config.kdl", "");
    std::os::unix::fs::symlink(dir.path().join("elsewhere"), dir.path().join("0803")).expect("symlink");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"), "0803");
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_file_unknown_file() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/real.kdl", "");
    std::os::unix::fs::symlink(dir.path().join("0803/real.kdl"), dir.path().join("0803/config.kdl")).expect("symlink");

    assert_unknown_file(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"), "config.kdl");
}

#[cfg(unix)]
#[test]
fn test_file_symlinked_intermediate_directory_unknown_file_even_when_target_holds_file() {
    let root = TempDir::new().expect("a temporary directory");
    let dir = root.path().join("media");
    write(&dir, "0803/config.kdl", "");
    write(root.path(), "elsewhere/home.md", "outside the directory");
    std::os::unix::fs::symlink(root.path().join("elsewhere"), dir.join("0803/narrative")).expect("symlink");

    assert_unknown_file(
        LiveDepositedFiles::new(&dir).file("0803", "narrative/home.md"),
        "narrative/home.md",
    );
}

#[test]
fn test_file_regular_file_under_shortcode_unknown_project() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803", "not a directory");

    assert_unknown_project(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"), "0803");
}

#[test]
fn test_file_directory_under_file_name_unavailable() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/config.kdl/inner", "");

    expect_unavailable(LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl"));
}

#[cfg(unix)]
#[test]
fn test_file_unreadable_project_directory_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/config.kdl", "");
    let project = dir.path().join("0803");
    fs::set_permissions(&project, fs::Permissions::from_mode(0o000)).expect("chmod");
    // Root reads the directory whatever its mode, so the refusal cannot be observed.
    if fs::read_dir(&project).is_ok() {
        fs::set_permissions(&project, fs::Permissions::from_mode(0o755)).expect("chmod back");
        return;
    }

    let result = LiveDepositedFiles::new(dir.path()).file("0803", "config.kdl");
    fs::set_permissions(&project, fs::Permissions::from_mode(0o755)).expect("chmod back");

    assert_eq!(expect_unavailable(result).kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn test_file_changed_file_served_changed() {
    let dir = TempDir::new().expect("a temporary directory");
    write(dir.path(), "0803/config.kdl", "first");
    let files = LiveDepositedFiles::new(dir.path());
    assert_eq!(files.file("0803", "config.kdl").expect("served"), b"first");

    write(dir.path(), "0803/config.kdl", "second");

    assert_eq!(files.file("0803", "config.kdl").expect("served"), b"second");
}
