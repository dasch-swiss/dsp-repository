//! Directories and files the store creates are `0755` and `0644` whatever the process's umask.
//! The outer test re-runs this binary under `umask 077` to run the inner, ignored one.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use shared_stand_in::{Generation, Provenance, ReadyToken};
use tempfile::TempDir;
use vre_bridge_store::{Subtree, Subtrees};

#[test]
fn test_created_paths_have_reader_modes_under_umask_077() {
    let binary = std::env::current_exe().expect("this test binary");
    let output = Command::new("sh")
        .arg("-c")
        .arg(r#"umask 077; exec "$0" --exact inner_created_paths_have_reader_modes --ignored"#)
        .arg(binary)
        .output()
        .expect("run the inner test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    // A filter that matched nothing would pass too.
    assert!(stdout.contains("1 passed"), "{stdout}");
}

#[test]
#[ignore = "run by test_created_paths_have_reader_modes_under_umask_077 under umask 077"]
fn inner_created_paths_have_reader_modes() {
    let dir = TempDir::new().expect("a temporary directory");
    let probe = dir.path().join("probe");
    fs::write(&probe, b"x").expect("write");
    assert_eq!(mode(&probe), 0o600, "the umask is 077");

    let sync = dir.path().join("sync");
    let media = dir.path().join("media");
    fs::create_dir(&sync).expect("create");
    fs::create_dir(&media).expect("create");
    let inputs = dir.path().join("inputs");
    fs::create_dir_all(inputs.join("narrative")).expect("create");
    fs::write(inputs.join("narrative/home.md"), b"x").expect("write");
    let subtrees = Subtrees::new(sync.clone(), media.clone());
    let generation = Generation::parse("20261120T010203Z-9f3a1c2e").expect("a generation");
    let mut writer = subtrees.begin("0803", generation.clone()).expect("begin");
    writer.write(Subtree::Sync, "snapshot.nq", b"x").expect("write");
    writer.write(Subtree::Media, "a/b/c.txt", b"x").expect("write");
    writer.copy_tree(Subtree::Media, &inputs).expect("copy");
    writer.write_rights_record().expect("write");
    let token = ReadyToken {
        provenance: Provenance {
            generation,
            as_of: "2026-11-20T01:02:03Z".to_owned(),
            server: "prod".to_owned(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: format!("sha256:{}", "0".repeat(64)),
        },
        previous_generation: None,
    };
    writer.commit(&token).expect("commit");

    let mut checked = 0;
    for root in [&sync, &media] {
        checked += assert_modes(&root.join("0803"));
    }
    assert_eq!(checked, 13, "every path created");
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).expect("metadata").permissions().mode() & 0o777
}

/// Asserts `0755` on `path` and every directory below it and `0644` on every file; returns how
/// many paths it checked.
fn assert_modes(path: &Path) -> usize {
    if fs::symlink_metadata(path).expect("metadata").is_dir() {
        assert_eq!(mode(path), 0o755, "{}", path.display());
        1 + fs::read_dir(path)
            .expect("list")
            .map(|entry| assert_modes(&entry.expect("entry").path()))
            .sum::<usize>()
    } else {
        assert_eq!(mode(path), 0o644, "{}", path.display());
        1
    }
}
