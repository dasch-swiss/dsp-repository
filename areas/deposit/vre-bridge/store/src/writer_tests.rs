use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{fs, io};

use shared_stand_in::{generation_dir, read_token, ReadyToken};

use crate::test_support::{
    commit, generation, generations, names, place, subtrees, temp_tokens, token, token_bytes, token_with, GEN_A, GEN_B,
    GEN_C, SC,
};
use crate::{StoreError, Subtree, Subtrees};

#[test]
fn test_commit_new_project_readable_in_both_subtrees_with_equal_provenance() {
    let (_dir, subtrees) = subtrees();
    let committed = token(GEN_A, None);

    commit(&subtrees, SC, &committed);

    for subtree in [Subtree::Sync, Subtree::Media] {
        let root = subtrees.root(subtree);
        let read = read_token(root, SC).expect("a valid token").expect("a token");
        assert_eq!(read, committed, "{subtree:?}");
        let dir = generation_dir(root, SC, &read.provenance.generation).expect("a shortcode");
        assert!(dir.is_dir(), "{subtree:?}");
        // The bytes on disk are the canonical encoding and parse back to the token.
        let bytes = token_bytes(root, SC).expect("the token's bytes");
        assert_eq!(bytes, committed.to_bytes());
        assert_eq!(ReadyToken::parse(&bytes), Ok(committed.clone()));
    }
    let sync_dir = generation_dir(subtrees.root(Subtree::Sync), SC, &generation(GEN_A)).expect("a shortcode");
    assert!(sync_dir.join("snapshot.nq").is_file());
}

#[test]
fn test_begin_before_commit_no_token_names_new_generation() {
    let (_dir, subtrees) = subtrees();
    let mut first = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    first.write(Subtree::Sync, "snapshot.nq", b"x").expect("write");
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(read_token(subtrees.root(subtree), SC), Ok(None), "{subtree:?}");
    }
    first.commit(&token(GEN_A, None)).expect("commit");

    let mut second = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    second.write(Subtree::Sync, "snapshot.nq", b"y").expect("write");
    for subtree in [Subtree::Sync, Subtree::Media] {
        let read = read_token(subtrees.root(subtree), SC).expect("valid").expect("a token");
        assert_eq!(read.provenance.generation, generation(GEN_A), "{subtree:?}");
    }
}

#[test]
fn test_commit_between_the_renames_media_names_new_and_sync_old() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    let mut writer = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    let seen = Arc::new(AtomicBool::new(false));
    let (hook_seen, hook_subtrees) = (Arc::clone(&seen), subtrees.clone());
    writer.before_sync_rename = Some(Box::new(move || {
        let generation_in = |subtree| {
            read_token(hook_subtrees.root(subtree), SC)
                .expect("valid")
                .expect("a token")
                .provenance
                .generation
        };
        assert_eq!(generation_in(Subtree::Media), generation(GEN_B));
        assert_eq!(generation_in(Subtree::Sync), generation(GEN_A));
        hook_seen.store(true, Ordering::SeqCst);
        Ok(())
    }));

    writer.commit(&token(GEN_B, Some(GEN_A))).expect("commit");

    assert!(seen.load(Ordering::SeqCst), "the hook ran");
}

fn failing_sync_rename() -> Option<Box<dyn FnOnce() -> io::Result<()> + Send>> {
    Some(Box::new(|| Err(io::Error::other("injected"))))
}

#[test]
fn test_commit_sync_rename_fails_both_tokens_name_old_generation() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    let before = [Subtree::Sync, Subtree::Media].map(|s| token_bytes(subtrees.root(s), SC));
    let mut writer = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    writer.before_sync_rename = failing_sync_rename();

    let result = writer.commit(&token(GEN_B, Some(GEN_A)));

    assert!(matches!(result, Err(StoreError::Io { .. })), "{result:?}");
    let after = [Subtree::Sync, Subtree::Media].map(|s| token_bytes(subtrees.root(s), SC));
    assert_eq!(after, before);
    for subtree in [Subtree::Sync, Subtree::Media] {
        let root = subtrees.root(subtree);
        assert_eq!(generations(root, SC), names(&[GEN_A, GEN_B]), "left for recovery: {subtree:?}");
        assert!(temp_tokens(root, SC).is_empty(), "{subtree:?}");
    }
}

#[test]
fn test_commit_sync_rename_and_restore_fail_recovery_points_media_back() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    let media_project = subtrees.root(Subtree::Media).join(SC);
    // The restore's temporary token cannot be written over a directory.
    fs::create_dir(media_project.join(format!(".ready.{GEN_B}.rollback.tmp"))).expect("block the restore");
    let mut writer = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    writer.before_sync_rename = failing_sync_rename();

    assert!(writer.commit(&token(GEN_B, Some(GEN_A))).is_err());
    let (sync, media) = subtrees.read_tokens(SC);
    assert_ne!(sync, media, "the restore failed");
    fs::remove_dir(media_project.join(format!(".ready.{GEN_B}.rollback.tmp"))).expect("unblock");

    let report = subtrees.recover(&[SC.to_owned()]);

    assert_eq!(report.repointed, [SC.to_owned()]);
    for subtree in [Subtree::Sync, Subtree::Media] {
        let root = subtrees.root(subtree);
        assert_eq!(token_bytes(root, SC), Some(token(GEN_A, None).to_bytes()), "{subtree:?}");
        assert_eq!(generations(root, SC), names(&[GEN_A]), "{subtree:?}");
    }
}

#[test]
fn test_commit_success_leaves_no_temporary_token() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, Some(GEN_A)));
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert!(temp_tokens(subtrees.root(subtree), SC).is_empty(), "{subtree:?}");
    }
}

#[test]
fn test_commit_previous_generation_not_the_replaced_one_keeps_the_replaced_one() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, None));
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(generations(subtrees.root(subtree), SC), names(&[GEN_A, GEN_B]), "{subtree:?}");
    }
}

/// A writer for [`GEN_A`] whose `media` generation holds an `access-rights.json` copied from a
/// shipped-input tree.
fn writer_with_shipped_rights_record(dir: &tempfile::TempDir, subtrees: &Subtrees) -> crate::GenerationWriter {
    let src = dir.path().join("inputs");
    fs::create_dir(&src).expect("create the source");
    fs::write(src.join("access-rights.json"), b"{}").expect("write");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    writer.copy_tree(Subtree::Media, &src).expect("copy");
    writer
}

#[test]
fn test_write_rights_record_after_a_shipped_file_of_its_name_refused() {
    let (dir, subtrees) = subtrees();
    let mut writer = writer_with_shipped_rights_record(&dir, &subtrees);
    assert!(writer.write_rights_record().is_err());
}

#[test]
fn test_copy_tree_shipped_file_named_like_the_rights_record_refused_after_it() {
    let (dir, subtrees) = subtrees();
    let src = dir.path().join("inputs");
    fs::create_dir(&src).expect("create the source");
    fs::write(src.join("access-rights.json"), b"{}").expect("write");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    writer.write_rights_record().expect("write the rights record");
    assert!(writer.copy_tree(Subtree::Media, &src).is_err());
}

#[test]
fn test_commit_sync_rename_fails_on_first_generation_no_token_in_either_subtree() {
    let (_dir, subtrees) = subtrees();
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    writer.before_sync_rename = failing_sync_rename();

    assert!(writer.commit(&token(GEN_A, None)).is_err());

    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(token_bytes(subtrees.root(subtree), SC), None, "{subtree:?}");
    }
}

#[test]
fn test_commit_sync_rename_fails_with_diverged_tokens_media_keeps_its_old_bytes() {
    let (_dir, subtrees) = subtrees();
    place(subtrees.root(Subtree::Sync), SC, &token(GEN_A, None));
    place(subtrees.root(Subtree::Media), SC, &token_with(GEN_B, Some(GEN_A), "stage"));
    let media_before = token_bytes(subtrees.root(Subtree::Media), SC);
    let mut writer = subtrees.begin(SC, generation(GEN_C)).expect("begin");
    writer.before_sync_rename = failing_sync_rename();

    assert!(writer.commit(&token(GEN_C, Some(GEN_A))).is_err());

    assert_eq!(token_bytes(subtrees.root(Subtree::Media), SC), media_before);
    assert_eq!(
        token_bytes(subtrees.root(Subtree::Sync), SC),
        Some(token(GEN_A, None).to_bytes())
    );
}

#[test]
fn test_writer_dropped_or_aborted_leaves_no_new_directory_and_tokens_untouched() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    let before = [Subtree::Sync, Subtree::Media].map(|s| token_bytes(subtrees.root(s), SC));

    let mut dropped = subtrees.begin(SC, generation(GEN_B)).expect("begin");
    dropped.write(Subtree::Media, "narrative/home.md", b"x").expect("write");
    drop(dropped);
    let aborted = subtrees.begin(SC, generation(GEN_C)).expect("begin");
    aborted.abort();

    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(generations(subtrees.root(subtree), SC), names(&[GEN_A]), "{subtree:?}");
    }
    let after = [Subtree::Sync, Subtree::Media].map(|s| token_bytes(subtrees.root(s), SC));
    assert_eq!(after, before);
}

#[test]
fn test_writer_dropped_on_first_generation_removes_the_project_directory() {
    let (_dir, subtrees) = subtrees();
    drop(subtrees.begin(SC, generation(GEN_A)).expect("begin"));
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert!(!subtrees.root(subtree).join(SC).exists(), "{subtree:?}");
    }
}

#[test]
fn test_commit_three_times_last_two_generations_remain() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_B, Some(GEN_A)));
    commit(&subtrees, SC, &token(GEN_C, Some(GEN_B)));
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(generations(subtrees.root(subtree), SC), names(&[GEN_B, GEN_C]), "{subtree:?}");
    }
}

#[test]
fn test_commit_new_generation_with_older_time_last_two_generations_remain() {
    const OLDER: &str = "20261101T000000Z-000000d0";
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    commit(&subtrees, SC, &token(GEN_C, Some(GEN_A)));
    commit(&subtrees, SC, &token(OLDER, Some(GEN_C)));
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(generations(subtrees.root(subtree), SC), names(&[GEN_C, OLDER]), "{subtree:?}");
    }
}

#[test]
fn test_commit_media_generation_holds_the_rights_record_bytes() {
    let (_dir, subtrees) = subtrees();
    commit(&subtrees, SC, &token(GEN_A, None));
    let record = subtrees.root(Subtree::Media).join(SC).join(GEN_A).join("access-rights.json");
    assert_eq!(
        fs::read_to_string(record).expect("the rights record"),
        r#"{"format":"access-rights","version":1,"default":{"access_rights":"public","data_classification":"unclassified"}}"#
    );
}

#[test]
fn test_commit_token_naming_another_generation_refused_tokens_untouched() {
    let (_dir, subtrees) = subtrees();
    let writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    let result = writer.commit(&token(GEN_B, None));
    assert!(matches!(result, Err(StoreError::Internal(_))), "{result:?}");
    for subtree in [Subtree::Sync, Subtree::Media] {
        assert_eq!(token_bytes(subtrees.root(subtree), SC), None, "{subtree:?}");
    }
}

#[test]
fn test_commit_token_readers_refuse_not_written() {
    let (_dir, subtrees) = subtrees();
    let writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    let unreadable = token_with(GEN_A, None, "Not A Server Id");
    let result = writer.commit(&unreadable);
    assert!(matches!(result, Err(StoreError::Internal(_))), "{result:?}");
    assert_eq!(token_bytes(subtrees.root(Subtree::Media), SC), None);
}

#[test]
fn test_begin_existing_generation_refused_and_nothing_created() {
    let (_dir, subtrees) = subtrees();
    fs::create_dir_all(subtrees.root(Subtree::Media).join(SC).join(GEN_A)).expect("seed media");

    let result = subtrees.begin(SC, generation(GEN_A));

    assert!(matches!(result, Err(StoreError::GenerationExists(_))));
    assert!(
        !subtrees.root(Subtree::Sync).join(SC).exists(),
        "sync's new directories are removed"
    );
    assert!(
        subtrees.root(Subtree::Media).join(SC).join(GEN_A).is_dir(),
        "the existing one is kept"
    );
}

#[test]
fn test_begin_name_failing_is_shortcode_refused() {
    let (_dir, subtrees) = subtrees();
    for shortcode in ["../x", "lost+found", "083c"] {
        let result = subtrees.begin(shortcode, generation(GEN_A));
        assert!(matches!(result, Err(StoreError::Shortcode(_))), "{shortcode}");
    }
}

#[test]
fn test_write_name_with_dot_dot_or_leading_dot_refused() {
    let (_dir, subtrees) = subtrees();
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    for name in ["../x", "..", ".ready", "narrative/.hidden", "narrative/../../x", "/abs"] {
        let result = writer.write(Subtree::Media, name, b"x");
        assert!(matches!(result, Err(StoreError::FileName(_))), "{name}");
    }
    writer
        .write(Subtree::Media, "narrative/deep/home.md", b"x")
        .expect("a nested name");
}

#[test]
fn test_write_existing_file_refused() {
    let (_dir, subtrees) = subtrees();
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    writer.write(Subtree::Media, "config.kdl", b"x").expect("first write");
    assert!(writer.write(Subtree::Media, "config.kdl", b"y").is_err());
}

#[test]
fn test_copy_tree_copies_nested_files_and_counts_sizes() {
    let (dir, subtrees) = subtrees();
    let src = dir.path().join("inputs");
    fs::create_dir_all(src.join("narrative")).expect("create the source");
    fs::write(src.join("config.kdl"), b"abc").expect("write");
    fs::write(src.join("narrative/home.md"), b"de").expect("write");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");

    writer.copy_tree(Subtree::Media, &src).expect("copy");

    let generation_root = subtrees.root(Subtree::Media).join(SC).join(GEN_A);
    assert_eq!(fs::read(generation_root.join("config.kdl")).expect("copied"), b"abc");
    assert_eq!(fs::read(generation_root.join("narrative/home.md")).expect("copied"), b"de");
    assert_eq!(writer.sizes().media, 5);
    assert_eq!(writer.sizes().sync, 0);
}

#[test]
fn test_copy_file_copies_under_its_name_and_refuses_a_symlink() {
    let (dir, subtrees) = subtrees();
    let src = dir.path().join("download-0");
    fs::write(&src, b"<TEI/>").expect("write");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&src, &link).expect("symlink");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");

    writer.copy_file(Subtree::Media, "Abc.xml", &src).expect("copy");
    let refused = writer.copy_file(Subtree::Media, "Def.xml", &link);

    let generation_root = subtrees.root(Subtree::Media).join(SC).join(GEN_A);
    assert_eq!(fs::read(generation_root.join("Abc.xml")).expect("copied"), b"<TEI/>");
    assert_eq!(writer.sizes().media, 6);
    assert!(matches!(refused, Err(StoreError::NotRegular(_))), "{refused:?}");
    assert!(!generation_root.join("Def.xml").exists());
}

#[test]
fn test_copy_tree_source_with_symlink_refused_and_nothing_written() {
    let (dir, subtrees) = subtrees();
    let src = dir.path().join("inputs");
    fs::create_dir(&src).expect("create the source");
    fs::write(src.join("config.kdl"), b"abc").expect("write");
    std::os::unix::fs::symlink("/etc/hosts", src.join("linked.txt")).expect("symlink");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");

    let result = writer.copy_tree(Subtree::Media, &src);

    assert!(matches!(result, Err(StoreError::NotRegular(_))), "{result:?}");
    let generation_root = subtrees.root(Subtree::Media).join(SC).join(GEN_A);
    assert_eq!(fs::read_dir(generation_root).expect("list").count(), 0);
}

#[test]
fn test_copy_tree_symlinked_source_root_refused() {
    let (dir, subtrees) = subtrees();
    let real = dir.path().join("real");
    fs::create_dir(&real).expect("create");
    std::os::unix::fs::symlink(&real, dir.path().join("inputs")).expect("symlink");
    let mut writer = subtrees.begin(SC, generation(GEN_A)).expect("begin");
    let result = writer.copy_tree(Subtree::Media, &dir.path().join("inputs"));
    assert!(matches!(result, Err(StoreError::NotRegular(_))), "{result:?}");
}
