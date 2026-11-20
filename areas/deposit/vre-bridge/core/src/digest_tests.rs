use std::fs;
use std::path::Path;

use crate::{inputs_digest, DigestError};

/// The digest of `a.txt`, `a/c.md`, `b.txt` below: `.` sorts before `/`, so `a.txt` comes before
/// the directory `a`'s file. Computed apart from this crate.
const FIXED_TREE_DIGEST: &str = "sha256:1af14fe7c1b3d6a5320aa86c7a36eda615d242453ca89ddcc63cea0995c22185";
const EMPTY_SEQUENCE_DIGEST: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The fixed tree, its files created out of sorted order.
fn fixed_tree(dir: &Path) {
    fs::create_dir(dir.join("a")).unwrap();
    fs::write(dir.join("b.txt"), "beta\n").unwrap();
    fs::write(dir.join("a/c.md"), "gamma").unwrap();
    fs::write(dir.join("a.txt"), "alpha").unwrap();
}

#[test]
fn test_inputs_digest_fixed_tree_equals_pinned_value() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());

    assert_eq!(inputs_digest(dir.path()).unwrap(), FIXED_TREE_DIGEST);
}

#[test]
fn test_inputs_digest_one_byte_changed_differs() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    fs::write(dir.path().join("a/c.md"), "gammb").unwrap();

    assert_ne!(inputs_digest(dir.path()).unwrap(), FIXED_TREE_DIGEST);
}

#[test]
fn test_inputs_digest_renamed_file_differs() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    fs::rename(dir.path().join("b.txt"), dir.path().join("d.txt")).unwrap();

    assert_ne!(inputs_digest(dir.path()).unwrap(), FIXED_TREE_DIGEST);
}

#[test]
fn test_inputs_digest_missing_directory_is_empty_sequence_digest() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(inputs_digest(&dir.path().join("0803")).unwrap(), EMPTY_SEQUENCE_DIGEST);
}

#[test]
fn test_inputs_digest_empty_directory_is_empty_sequence_digest() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("narrative")).unwrap();

    assert_eq!(inputs_digest(dir.path()).unwrap(), EMPTY_SEQUENCE_DIGEST);
}

#[test]
fn test_inputs_digest_symlink_below_refused() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    std::os::unix::fs::symlink("a.txt", dir.path().join("a/link.txt")).unwrap();

    assert_eq!(inputs_digest(dir.path()), Err(DigestError::Symlink("a/link.txt".to_owned())));
}

#[test]
fn test_inputs_digest_symlinked_directory_refused() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    let link = dir.path().join("0803");
    std::os::unix::fs::symlink(dir.path(), &link).unwrap();

    assert_eq!(inputs_digest(&link), Err(DigestError::Symlink(".".to_owned())));
}

#[test]
fn test_inputs_digest_reserved_name_refused() {
    for name in ["access-rights.json", "file-manifest.json"] {
        let dir = tempfile::tempdir().unwrap();
        fixed_tree(dir.path());
        fs::write(dir.path().join(name), "{}").unwrap();

        assert_eq!(inputs_digest(dir.path()), Err(DigestError::Reserved(name.to_owned())));
    }
}

#[test]
fn test_inputs_digest_reserved_name_as_directory_or_in_other_case_refused() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    fs::create_dir(dir.path().join("access-rights.json")).unwrap();
    assert_eq!(
        inputs_digest(dir.path()),
        Err(DigestError::Reserved("access-rights.json".to_owned()))
    );

    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    fs::write(dir.path().join("a/File-Manifest.json"), "{}").unwrap();
    assert_eq!(
        inputs_digest(dir.path()),
        Err(DigestError::Reserved("a/File-Manifest.json".to_owned()))
    );
}

#[test]
fn test_inputs_digest_dot_file_refused() {
    let dir = tempfile::tempdir().unwrap();
    fixed_tree(dir.path());
    fs::write(dir.path().join("a/.DS_Store"), "").unwrap();

    assert_eq!(inputs_digest(dir.path()), Err(DigestError::Name("a/.DS_Store".to_owned())));
}
