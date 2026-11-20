use std::fs;

use chrono::{TimeZone, Utc};
use shared_stand_in::{Generation, TokenError, TOKEN_FILE};

use crate::test_support::{commit, names, place, subtrees, token, GEN_A, GEN_B, SC};
use crate::{new_generation_name, Broken, StoreError, Subtree, TokenState};

#[test]
fn test_new_generation_name_every_minted_name_passes_generation_parse() {
    let times = [
        Utc.with_ymd_and_hms(2026, 11, 20, 1, 2, 3).unwrap(),
        Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(9999, 12, 31, 23, 59, 59).unwrap(),
    ];
    for time in times {
        for _ in 0..100 {
            let minted = new_generation_name(&time).expect("a generation name");
            assert_eq!(Generation::parse(minted.as_str()), Some(minted.clone()));
        }
    }
    let minted = new_generation_name(&times[0]).expect("a generation name");
    assert!(minted.as_str().starts_with("20261120T010203Z-"), "{minted}");
}

#[test]
fn test_new_generation_name_two_names_for_one_time_differ() {
    let time = Utc.with_ymd_and_hms(2026, 11, 20, 1, 2, 3).unwrap();
    let minted: std::collections::BTreeSet<_> = (0..20)
        .map(|_| new_generation_name(&time).expect("a name").to_string())
        .collect();
    assert!(minted.len() > 1);
}

#[test]
fn test_new_generation_name_year_past_9999_internal_error() {
    let time = Utc.with_ymd_and_hms(10000, 1, 1, 0, 0, 0).unwrap();
    assert!(matches!(new_generation_name(&time), Err(StoreError::Internal(_))));
}

#[test]
fn test_read_tokens_missing_valid_broken_and_missing_generation() {
    let (_dir, subtrees) = subtrees();
    assert_eq!(subtrees.read_tokens(SC), (TokenState::Missing, TokenState::Missing));

    commit(&subtrees, SC, &token(GEN_A, None));
    let valid = TokenState::Valid(token(GEN_A, None));
    assert_eq!(subtrees.read_tokens(SC), (valid.clone(), valid));

    let naming_missing = token(GEN_B, Some(GEN_A));
    fs::write(
        subtrees.root(Subtree::Sync).join(SC).join(TOKEN_FILE),
        naming_missing.to_bytes(),
    )
    .expect("write");
    fs::write(subtrees.root(Subtree::Media).join(SC).join(TOKEN_FILE), b"{}").expect("write");
    let (sync, media) = subtrees.read_tokens(SC);
    assert_eq!(sync, TokenState::Broken(Broken::MissingGeneration(naming_missing)));
    assert!(
        matches!(media, TokenState::Broken(Broken::Token(TokenError::Invalid(_)))),
        "{media:?}"
    );
}

#[test]
fn test_tokenless_projects_lists_directories_without_a_token_in_either_subtree() {
    let (_dir, subtrees) = subtrees();
    let sync = subtrees.root(Subtree::Sync);
    let media = subtrees.root(Subtree::Media);
    place(sync, "0803", &token(GEN_A, None));
    fs::create_dir(media.join("0803")).expect("create");
    place(media, "0804", &token(GEN_A, None));
    fs::create_dir(sync.join("0804")).expect("create");
    fs::create_dir(sync.join("0805")).expect("create");
    fs::create_dir(media.join("0806")).expect("create");
    fs::create_dir(sync.join("lost+found")).expect("create");

    assert_eq!(subtrees.tokenless_projects(), names(&["0805", "0806"]));
}
