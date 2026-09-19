// Live integration test for the update check's crates.io sparse-index fetch
// (`dsp_cli::update::fetch_latest`).
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
// or specifically:
//   cargo test --features live --test live_update_check
//
// The `just test-live` recipe runs the former.
//
// dsp-cli/ADR-0009 (testing strategy): live tests are layer 5 and are **not** in CI.
//
// Unlike the other live tests in this file's siblings (e.g.
// `live_project_list.rs`, `live_data_model_list.rs`), this test needs NO
// environment variables — it hits the public crates.io sparse index directly
// (`https://index.crates.io/ds/p-/dsp-cli`), not a DSP-API instance, so there
// is no `DSP_TEST_SERVER`/`DSP_TOKEN` to supply and no skip path to wire up.
#![cfg(feature = "live")]

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test: fetch the real crates.io sparse index for `dsp-cli`
/// and assert that a published, stable version is returned.
///
/// `dsp-cli` has been published to crates.io since 0.1.0 (see CHANGELOG.md),
/// so a real fetch must return at least that version.
#[test]
#[ignore = "hits crates.io; excluded from just dsp-cli-test-live, run with cargo nextest run -p dsp-cli --features live --run-ignored only --test live_update_check"]
fn live_fetch_latest_returns_a_published_stable_version() {
    eprintln!("live test: calling fetch_latest against the real crates.io sparse index");

    let v = dsp_cli::update::fetch_latest("https://index.crates.io/ds/p-/dsp-cli")
        .expect("fetch_latest failed — check network connectivity to index.crates.io")
        .expect("expected a published stable version for dsp-cli, got None");

    eprintln!("live test: got version {v}");

    assert!(
        v >= semver::Version::parse("0.1.0").unwrap(),
        "expected the latest published dsp-cli version to be >= 0.1.0, got {v}"
    );
}
