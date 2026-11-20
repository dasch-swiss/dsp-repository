use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use shared_stand_in::TOKEN_FILE;

use crate::test_support::{
    commit, generation, generations, names, place, subtrees, token, token_bytes, token_with, GEN_A, GEN_B, GEN_C, SC,
};
use crate::{Subtree, Subtrees};

const ORPHAN: &str = "20261117T000000Z-000000ff";

fn allowlist(shortcodes: &[&str]) -> Vec<String> {
    shortcodes.iter().map(|&shortcode| shortcode.to_owned()).collect()
}

/// Adds the directory `<root>/<shortcode>/<name>` with one file in it.
fn seed_generation(root: &Path, shortcode: &str, name: &str) {
    let dir = root.join(shortcode).join(name);
    fs::create_dir_all(&dir).expect("create the generation");
    fs::write(dir.join("snapshot.nq"), b"x").expect("write");
}

fn both(subtrees: &Subtrees) -> [&Path; 2] {
    [subtrees.root(Subtree::Sync), subtrees.root(Subtree::Media)]
}

#[test]
fn test_recover_diverged_media_token_pointed_back_and_orphans_deleted() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, Some(GEN_A)));
    // A refresh that replaced the media token and died before the sync token.
    let media = subtrees.root(Subtree::Media);
    place(media, SC, &token_with(GEN_C, Some(GEN_B), "stage"));
    seed_generation(subtrees.root(Subtree::Sync), SC, GEN_C);
    for root in both(&subtrees) {
        seed_generation(root, SC, ORPHAN);
        for temp in [".ready.x.tmp", ".ready.x.rollback.tmp", ".ready.x.recover.tmp"] {
            fs::write(root.join(SC).join(temp), b"{}").expect("write a stale temp token");
        }
    }

    let report = subtrees.recover(&allowlist(&[SC]));

    assert_eq!(report.repointed, allowlist(&[SC]));
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(token_bytes(media, SC), token_bytes(subtrees.root(Subtree::Sync), SC));
    assert_eq!(token_bytes(media, SC), Some(token(GEN_B, Some(GEN_A)).to_bytes()));
    for root in both(&subtrees) {
        let entries: BTreeSet<String> = fs::read_dir(root.join(SC))
            .expect("list")
            .map(|entry| entry.expect("entry").file_name().into_string().expect("UTF-8"))
            .collect();
        assert_eq!(entries, names(&[GEN_A, GEN_B, TOKEN_FILE]), "{}", root.display());
    }
}

#[test]
fn test_recover_lone_media_token_and_its_generation_left_alone() {
    let (_dir, subtrees) = subtrees();
    let media = subtrees.root(Subtree::Media);
    place(media, SC, &token(GEN_A, None));
    seed_generation(subtrees.root(Subtree::Sync), SC, GEN_A);
    let before = token_bytes(media, SC);

    let report = subtrees.recover(&allowlist(&[SC]));

    assert!(report.repointed.is_empty());
    assert_eq!(token_bytes(media, SC), before);
    assert_eq!(token_bytes(subtrees.root(Subtree::Sync), SC), None);
    for root in both(&subtrees) {
        assert_eq!(generations(root, SC), names(&[GEN_A]), "{}", root.display());
    }
}

/// Every generation of a project whose `sync` token cannot be read stays in place.
fn assert_unreadable_sync_token_keeps_every_generation(break_token: impl FnOnce(&Path)) {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, Some(GEN_A)));
    for root in both(&subtrees) {
        seed_generation(root, SC, ORPHAN);
    }
    break_token(&subtrees.root(Subtree::Sync).join(SC).join(TOKEN_FILE));

    let report = subtrees.recover(&allowlist(&[SC]));

    assert_eq!(report.skipped, allowlist(&[SC]));
    assert_eq!(report.deleted_generations, 0);
    for root in both(&subtrees) {
        assert_eq!(generations(root, SC), names(&[GEN_A, GEN_B, ORPHAN]), "{}", root.display());
    }
}

#[test]
fn test_recover_token_replaced_by_a_directory_keeps_every_generation() {
    assert_unreadable_sync_token_keeps_every_generation(|token| {
        fs::remove_file(token).expect("remove the token");
        fs::create_dir(token).expect("a directory where the token was");
    });
}

#[test]
fn test_recover_token_of_a_later_version_keeps_every_generation() {
    assert_unreadable_sync_token_keeps_every_generation(|token| {
        let bytes = fs::read_to_string(token).expect("read the token");
        fs::write(token, bytes.replace(r#""token_version":1"#, r#""token_version":2"#)).expect("write");
    });
}

#[test]
fn test_recover_token_naming_a_missing_generation_keeps_its_previous_generation() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, Some(GEN_A)));
    for root in both(&subtrees) {
        fs::remove_dir_all(root.join(SC).join(GEN_B)).expect("remove the current generation");
        seed_generation(root, SC, ORPHAN);
    }

    let report = subtrees.recover(&allowlist(&[SC]));

    assert!(report.skipped.is_empty());
    for root in both(&subtrees) {
        assert_eq!(generations(root, SC), names(&[GEN_A]), "{}", root.display());
    }
}

#[test]
fn test_recover_project_without_tokens_deletes_its_generations() {
    let (_dir, subtrees) = subtrees();
    for root in both(&subtrees) {
        seed_generation(root, SC, GEN_A);
    }
    subtrees.recover_project(SC).expect("recover");
    for root in both(&subtrees) {
        assert_eq!(generations(root, SC), BTreeSet::new(), "{}", root.display());
    }
}

/// `0803` listed with tokens, `0804` unlisted with tokens, `0805` and `0806` unlisted without,
/// and `lost+found` in both roots.
fn removal_fixture() -> (tempfile::TempDir, Subtrees) {
    let (dir, subtrees) = subtrees();
    commit(&subtrees, "0803", &token(GEN_A, None));
    commit(&subtrees, "0804", &token(GEN_A, None));
    for root in both(&subtrees) {
        seed_generation(root, "0805", GEN_A);
        seed_generation(root, "0806", GEN_A);
        fs::create_dir(root.join("lost+found")).expect("create lost+found");
    }
    (dir, subtrees)
}

#[test]
fn test_remove_unlisted_empty_allowlist_does_nothing() {
    let (_dir, subtrees) = removal_fixture();
    let before = tree(&subtrees);
    let report = subtrees.remove_unlisted(&[], &names(&["0805"])).expect("remove");
    assert!(report.withdrawn.is_empty() && report.deleted.is_empty());
    assert_eq!(tree(&subtrees), before);
}

#[test]
fn test_remove_unlisted_withdraws_tokens_keeps_data_and_deletes_tokenless_directories() {
    let (_dir, subtrees) = removal_fixture();

    let report = subtrees
        .remove_unlisted(&allowlist(&["0803"]), &names(&["0805"]))
        .expect("remove");

    assert_eq!(report.withdrawn, allowlist(&["0804"]));
    assert_eq!(report.deleted, allowlist(&["0805"]));
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    for root in both(&subtrees) {
        assert_eq!(token_bytes(root, "0804"), None, "{}", root.display());
        assert_eq!(generations(root, "0804"), names(&[GEN_A]), "{}", root.display());
        assert!(!root.join("0805").exists(), "{}", root.display());
        assert!(root.join("0806").is_dir(), "tokenless now, but not at the run's start");
        assert!(root.join("lost+found").is_dir());
        assert!(token_bytes(root, "0803").is_some());
    }
}

#[test]
fn test_remove_unlisted_sync_token_goes_before_media_token() {
    let (_dir, subtrees) = removal_fixture();
    // A media token that cannot be removed: what went before it is gone, what comes after is not.
    let media_token = subtrees.root(Subtree::Media).join("0804").join(TOKEN_FILE);
    fs::remove_file(&media_token).expect("remove");
    fs::create_dir(&media_token).expect("a directory where the token was");
    fs::write(media_token.join("x"), b"x").expect("write");

    let report = subtrees
        .remove_unlisted(&allowlist(&["0803"]), &BTreeSet::new())
        .expect("remove");

    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, "0804");
    assert_eq!(token_bytes(subtrees.root(Subtree::Sync), "0804"), None);
}

/// Every path below both roots, with the bytes of each file.
fn tree(subtrees: &Subtrees) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(path: &Path, found: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let metadata = fs::symlink_metadata(path).expect("metadata");
        if metadata.is_dir() {
            found.insert(path.to_owned(), None);
            for entry in fs::read_dir(path).expect("list") {
                walk(&entry.expect("entry").path(), found);
            }
        } else {
            found.insert(path.to_owned(), Some(fs::read(path).expect("read")));
        }
    }
    let mut found = BTreeMap::new();
    for root in both(subtrees) {
        walk(root, &mut found);
    }
    found
}

#[test]
fn test_every_operation_writes_only_below_the_allowlisted_shortcode() {
    let (dir, subtrees) = subtrees();
    for root in both(&subtrees) {
        fs::create_dir(root.join("lost+found")).expect("create");
        fs::write(root.join("PROVENANCE"), b"p").expect("write");
        fs::write(root.join("0803-arks.txt"), b"a").expect("write");
        seed_generation(root, "0804", GEN_A);
    }
    let inputs = dir.path().join("inputs");
    fs::create_dir_all(inputs.join("narrative")).expect("create");
    fs::write(inputs.join("narrative/home.md"), b"x").expect("write");
    let work = dir.path().join("work");
    fs::create_dir_all(work.join("vre-bridge-dump-x")).expect("create");
    let outside = |subtrees: &Subtrees| {
        let mut found = tree(subtrees);
        found.retain(|path, _| !both(subtrees).iter().any(|root| path.starts_with(root.join(SC))));
        found
    };
    let before = outside(&subtrees);
    let confined = |step: &str| assert_eq!(outside(&subtrees), before, "after {step}");
    let listed = allowlist(&[SC]);

    commit(&subtrees, SC, &token(GEN_A, None));
    confined("a first commit");
    let mut writer = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    confined("begin");
    writer.write(Subtree::Media, "narrative/note.md", b"x").expect("write");
    writer.copy_tree(Subtree::Media, &inputs).expect("copy");
    writer.write_rights_record().expect("write the rights record");
    confined("writing");
    writer.commit(&token(GEN_B, Some(GEN_A))).expect("commit");
    confined("commit");
    drop(subtrees.begin(SC, generation(GEN_C)).expect("begin"));
    confined("a dropped writer");
    let report = subtrees.recover(&listed);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    confined("recovery");
    let tokenless = subtrees.tokenless_projects();
    assert!(tokenless.contains("0804"));
    subtrees.remove_unlisted(&listed, &BTreeSet::new()).expect("remove");
    confined("removal");
    let lock = crate::Lock::try_acquire(&dir.path().join("vre-bridge.lock"))
        .expect("open")
        .expect("free");
    lock.record_nightly(chrono::NaiveDate::from_ymd_opt(2026, 11, 20).expect("a date"))
        .expect("record");
    crate::sweep_work_dir(&work, "vre-bridge-dump-").expect("sweep");
    confined("the lock, its record and the sweep");
    let beside: BTreeSet<String> = fs::read_dir(dir.path())
        .expect("list")
        .map(|entry| entry.expect("entry").file_name().into_string().expect("UTF-8"))
        .collect();
    assert_eq!(
        beside,
        names(&[
            "inputs",
            "media",
            "sync",
            "vre-bridge.lock",
            "vre-bridge.lock.nightly",
            "work"
        ]),
        "nothing but the lock and its record is written beside the subtrees"
    );
    assert_eq!(fs::read_dir(&work).expect("list").count(), 0);
}
