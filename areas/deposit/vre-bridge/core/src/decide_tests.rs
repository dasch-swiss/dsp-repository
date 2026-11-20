use shared_stand_in::{Generation, Provenance, ReadyToken};

use crate::{decide, AllowlistEntry, Decision, RefreshReason, Running, ServerId};

const DIGEST: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const OTHER_DIGEST: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

fn running() -> Running {
    Running::new(DIGEST.to_owned())
}

fn entry() -> AllowlistEntry {
    AllowlistEntry {
        shortcode: "0803".to_owned(),
        server: ServerId::parse("prod").unwrap(),
    }
}

/// A token the running bridge would write for `entry()`, with T `as_of`.
fn token(as_of: &str) -> ReadyToken {
    let running = running();
    ReadyToken {
        provenance: Provenance {
            generation: Generation::parse("20261120T010203Z-9f3a1c2e").unwrap(),
            as_of: as_of.to_owned(),
            server: "prod".to_owned(),
            interim_format_version: running.interim_format_version,
            transformation_version: running.transformation_version,
            inputs_digest: running.inputs_digest,
        },
        previous_generation: None,
    }
}

fn current() -> ReadyToken {
    token("2026-11-20T01:02:03.456Z")
}

/// Each token changed by `change`, in turn in `sync` and in `media`, is forced for `reason`.
fn assert_forced_in_either(change: impl Fn(&mut Provenance), reason: RefreshReason) {
    let mut changed = current();
    change(&mut changed.provenance);
    let unchanged = current();
    for (sync, media) in [(&changed, &unchanged), (&unchanged, &changed)] {
        let decision = decide(Some(sync), Some(media), &entry(), &running(), false);
        assert_eq!(decision, Decision::Forced(reason), "sync {sync:?}, media {media:?}");
    }
}

#[test]
fn test_decide_no_token_in_one_subtree_forced() {
    let present = current();
    for (sync, media) in [(None, Some(&present)), (Some(&present), None), (None, None)] {
        let decision = decide(sync, media, &entry(), &running(), false);
        assert_eq!(decision, Decision::Forced(RefreshReason::MissingToken));
    }
}

#[test]
fn test_decide_sync_token_time_unreadable_forced_as_missing() {
    let unreadable = token("yesterday");

    let decision = decide(Some(&unreadable), Some(&current()), &entry(), &running(), false);

    assert_eq!(decision, Decision::Forced(RefreshReason::MissingToken));
}

#[test]
fn test_decide_server_differs_forced() {
    assert_forced_in_either(|p| p.server = "stage".to_owned(), RefreshReason::OtherServer);
}

#[test]
fn test_decide_interim_format_version_differs_forced() {
    assert_forced_in_either(|p| p.interim_format_version += 1, RefreshReason::InterimFormatVersion);
}

#[test]
fn test_decide_transformation_version_differs_forced() {
    assert_forced_in_either(|p| p.transformation_version += 1, RefreshReason::TransformationVersion);
}

#[test]
fn test_decide_inputs_digest_differs_forced() {
    assert_forced_in_either(|p| p.inputs_digest = OTHER_DIGEST.to_owned(), RefreshReason::InputsDigest);
}

#[test]
fn test_decide_operator_asks_forced() {
    let decision = decide(Some(&current()), Some(&current()), &entry(), &running(), true);

    assert_eq!(decision, Decision::Forced(RefreshReason::Operator));
}

#[test]
fn test_decide_operator_asks_with_token_missing_reports_missing_token() {
    let decision = decide(None, Some(&current()), &entry(), &running(), true);

    assert_eq!(decision, Decision::Forced(RefreshReason::MissingToken));
}

#[test]
fn test_decide_current_tokens_check_changes_from_t_minus_60_s() {
    let decision = decide(Some(&current()), Some(&current()), &entry(), &running(), false);

    let Decision::CheckChanges { cutoff } = decision else {
        panic!("{decision:?}")
    };
    assert_eq!(cutoff.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(), "2026-11-20T01:01:03.456Z");
}

#[test]
fn test_decide_t_with_six_fractional_digits_cutoff_keeps_them() {
    let six = token("2026-11-20T00:00:30.123456Z");

    let decision = decide(Some(&six), Some(&current()), &entry(), &running(), false);

    let Decision::CheckChanges { cutoff } = decision else {
        panic!("{decision:?}")
    };
    assert_eq!(
        cutoff.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        "2026-11-19T23:59:30.123456Z"
    );
}
