// Live integration test for `dsp vre resource list`.
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//   DSP_TEST_PROJECT  — project shortcode (e.g. "0803" for incunabula)
//   DSP_TEST_CLASS_IRI — full class IRI to list resources for
//                       (e.g. "http://api.dasch.swiss/ontology/0803/incunabula/v2#page")
//
// Token supply (optional):
//   DSP_TOKEN         — bearer token; if set, exercises the authenticated path.
//                       NEVER logged or interpolated in failure messages.
//
// D4 checkpoint (updated 2026-06-17): The command now uses `schema=complex`,
// which carries both `knora-api:creationDate` and `knora-api:lastModificationDate`
// (live-verified on `dev` 2026-06-17 against incunabula). `creation_date` is now
// hard-asserted to be present (at least one resource must have it in complex).
// `last_modified` is report-only (a resource that has never been modified has none).
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a required environment variable. Returns `None` and emits a skip
/// message if the variable is absent or empty.
fn require_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => {
            eprintln!("skipping live test: {name} not set");
            None
        }
    }
}

/// Read an optional environment variable. Returns `None` silently if absent.
fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

// ---------------------------------------------------------------------------
// Live test — D4 schema checkpoint
// ---------------------------------------------------------------------------

/// End-to-end live test for `dsp vre resource list` against a real DSP instance.
///
/// **D4 hard assertions** (updated 2026-06-17 — command now uses `schema=complex`):
/// - `iri` (`@id`) must be present (non-empty) on at least one resource.
/// - `label` must be present (non-empty) on at least one resource.
/// - `creation_date` (`knora-api:creationDate`) must be present on at least one resource — hard
///   assertion since `schema=complex` always carries creation dates (live-verified on `dev`
///   2026-06-17).
///
/// Report-only conditions (not a test failure):
/// - `ark_url` (`knora-api:arkUrl`) — present in both schemas; logged as count.
/// - `last_modified` (`knora-api:lastModificationDate`) — present in complex but server-side
///   optional (a never-modified resource has none); logged as count.
///
/// Also exercises the full-IRI bypass path: when `DSP_TEST_CLASS_IRI` is set
/// to a full IRI, the CLI (and this test) use it directly without scanning
/// data-models.
///
/// Skips cleanly (with `eprintln!`) if required env vars are absent.
#[test]
fn live_resource_list_schema_field_assertion() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };
    let project_shortcode = match require_env("DSP_TEST_PROJECT") {
        Some(v) => v,
        None => return,
    };
    let class_iri = match require_env("DSP_TEST_CLASS_IRI") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test: DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test: DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 3. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 4. Resolve the project ────────────────────────────────────────────────
    eprintln!("live test: resolving project '{}' on {}", project_shortcode, cfg.server);

    let proj = client.resolve_project(&cfg.server, &project_shortcode).expect(
        "resolve_project failed — check DSP_TEST_SERVER, DSP_TEST_PROJECT, \
             and network connectivity",
    );

    eprintln!("live test: resolved project '{}' (IRI: {})", proj.shortname, proj.iri);

    // ── 5. Call list_resources — full-IRI bypass path ────────────────────────
    // When DSP_TEST_CLASS_IRI is a full IRI (contains "://"), the production
    // action uses it directly without scanning data-models. This test mirrors
    // that path — D1 full-IRI bypass is exercised here, not the scan path.
    assert!(
        class_iri.contains("://"),
        "DSP_TEST_CLASS_IRI must be a full IRI (contains '://'); got: {class_iri:?}"
    );

    eprintln!(
        "live test: calling list_resources for class '{}' on project '{}' (page 0)",
        class_iri, proj.iri
    );

    let page = client
        .list_resources(&cfg.server, &proj.iri, &class_iri, None, 0, token_ref)
        .expect(
            "list_resources failed — check DSP_TEST_SERVER, DSP_TEST_PROJECT, \
             DSP_TEST_CLASS_IRI and network connectivity",
        );

    eprintln!(
        "live test: received {} resource(s) on page 0 (may_have_more_results: {})",
        page.resources.len(),
        page.may_have_more_results
    );

    // ── 6. D4 schema checkpoint: hard-assert envelope field presence ──────────
    //
    // Updated 2026-06-17: the command now uses `schema=complex`, which carries
    // creation and last-modification dates (live-verified on `dev` 2026-06-17).
    //
    // Hard-fail conditions:
    //   - `iri` absent (empty) for ALL resources: the IRI is load-bearing for the list projection;
    //     if absent the client is broken.
    //   - `label` absent (empty) for ALL resources: labels are the human-readable identifier in the
    //     prose view; if all absent something is wrong.
    //   - `creation_date` absent for ALL resources: complex carries creationDate for every
    //     resource; if all absent the DTO extraction is broken.
    //
    // Report-only conditions (not a test failure):
    //   - `ark_url` absent for all resources: present in both schemas; logged as count.
    //   - `last_modified` absent for all resources: server-side optional (a resource that has never
    //     been modified has none); logged as present/absent count.

    if page.resources.is_empty() {
        // No resources to assert on — skip the field assertions but record the fact.
        eprintln!(
            "live test: WARNING — list_resources returned 0 resources for class '{}' in project '{}'. \
             D4 field assertions skipped (no data to assert on). \
             Check that the project has resources of this type.",
            class_iri, proj.shortname
        );
        // This is not a hard failure — the project may genuinely have no resources
        // of this type. Log it and return.
        return;
    }

    eprintln!(
        "live test: D4 schema checkpoint — inspecting {} resource(s) for envelope field presence \
         (schema=complex, 2026-06-17)",
        page.resources.len()
    );

    // -- IRI (hard assertion) --
    let all_iri_empty = page.resources.iter().all(|r| r.iri.is_empty());
    assert!(
        !all_iri_empty,
        "D4 HARD FAIL: `iri` (from @id) is empty for ALL resources returned by \
         schema=complex. This indicates the HTTP client DTO translation is broken \
         or the API is not returning @id. \
         class_iri={class_iri:?}, project_iri={:?}, server={:?}",
        proj.iri, cfg.server
    );

    let iri_present_count = page.resources.iter().filter(|r| !r.iri.is_empty()).count();
    eprintln!(
        "live test: D4 field 'iri': present on {}/{} resources (schema=complex)",
        iri_present_count,
        page.resources.len()
    );

    // -- label (hard assertion) --
    let all_label_empty = page.resources.iter().all(|r| r.label.is_empty());
    assert!(
        !all_label_empty,
        "D4 HARD FAIL: `label` (from rdfs:label) is empty for ALL resources returned by \
         schema=complex. This indicates the client DTO translation is broken. \
         class_iri={class_iri:?}, project_iri={:?}, server={:?}",
        proj.iri, cfg.server
    );

    let label_present_count = page.resources.iter().filter(|r| !r.label.is_empty()).count();
    eprintln!(
        "live test: D4 field 'label': present on {}/{} resources (schema=complex)",
        label_present_count,
        page.resources.len()
    );

    // -- ark_url (report-only) --
    let ark_present_count = page.resources.iter().filter(|r| r.ark_url.is_some()).count();
    eprintln!(
        "live test: D4 field 'ark_url': present on {}/{} resources (schema=complex)",
        ark_present_count,
        page.resources.len()
    );

    // -- creation_date (hard assertion — complex always carries it) --
    let cd_present_count = page.resources.iter().filter(|r| r.creation_date.is_some()).count();
    assert!(
        cd_present_count > 0,
        "D4 HARD FAIL: `creation_date` (from knora-api:creationDate) is absent from \
         ALL {} resources in schema=complex response. Expected at least one to have it \
         (complex carries creation dates for every resource). This indicates the DTO \
         extraction for knora-api:creationDate is broken. \
         class_iri={class_iri:?}, project_iri={:?}, server={:?}",
        page.resources.len(),
        proj.iri,
        cfg.server
    );
    eprintln!(
        "live test: D4 field 'creation_date': present on {}/{} resources (schema=complex) ✓",
        cd_present_count,
        page.resources.len()
    );

    // -- last_modified (report-only — server-side optional) --
    let lm_present_count = page.resources.iter().filter(|r| r.last_modified.is_some()).count();
    if lm_present_count == 0 {
        eprintln!(
            "live test: D4 NOTE — `last_modified` (from knora-api:lastModificationDate) is absent \
             from ALL {} resources. This is NOT a test failure — a resource that has never been \
             modified has none (server-side optional). \
             Either this class has only freshly-created unmodified resources, or the server returns \
             no lastModificationDate for this resource class.",
            page.resources.len()
        );
    } else {
        eprintln!(
            "live test: D4 field 'last_modified': present on {}/{} resources (schema=complex) ✓",
            lm_present_count,
            page.resources.len()
        );
    }

    // ── 7. Per-resource structural invariants ─────────────────────────────────
    for res in &page.resources {
        assert!(
            !res.iri.is_empty(),
            "every resource must have a non-empty iri; got an empty iri on label={:?}",
            res.label
        );
        assert!(res.iri.starts_with("http"), "resource IRI '{}' must start with 'http'", res.iri);
        // resource_type is always set (fallback to "unknown" if @type missing).
        assert!(
            !res.resource_type.is_empty(),
            "resource_type must not be empty (iri: {})",
            res.iri
        );
        eprintln!(
            "live test: resource label={:?} iri={} ark_url={:?} creation_date={:?} \
             last_modified={:?} type={}",
            res.label, res.iri, res.ark_url, res.creation_date, res.last_modified, res.resource_type
        );
    }

    eprintln!(
        "live test: PASSED — D4 schema checkpoint complete (schema=complex). \
         {} resource(s) examined. label OK, iri OK. \
         ark_url: {}/{}, creation_date: {}/{} ✓, last_modified: {}/{}.",
        page.resources.len(),
        ark_present_count,
        page.resources.len(),
        cd_present_count,
        page.resources.len(),
        lm_present_count,
        page.resources.len()
    );
}

// ---------------------------------------------------------------------------
// Live test — --order-by acceptance checkpoint
// ---------------------------------------------------------------------------

/// End-to-end live test for `--order-by` on `dsp vre resource list`.
///
/// Verifies that passing a property IRI as `orderByProperty` to the DSP-API
/// `GET /v2/resources` endpoint returns a 200 response (i.e. the IRI is
/// accepted as valid by `iriConverter.asPropertyIri` on the server side).
///
/// This proves the complex-schema property IRI (as returned by
/// `describe_resource_type`) slots directly into `orderByProperty` — the key
/// invariant documented in the plan-025 DSP-API contract section.
///
/// **Required env vars:** same as the schema-field test (DSP_TEST_SERVER,
/// DSP_TEST_PROJECT, DSP_TEST_CLASS_IRI).
///
/// **Optional env var:** `DSP_TEST_ORDER_BY_IRI` — a full complex-schema
/// property IRI to pass as `order_by`. If absent, the test skips with a note.
/// The property must be a field of the resource-type identified by
/// DSP_TEST_CLASS_IRI. Example for the known-good incunabula/page target on
/// dev: `http://api.dev.dasch.swiss/ontology/0803/incunabula/v2#seqnum`.
///
/// The test asserts only that the call succeeds (no error returned) — it does
/// NOT assert any particular ordering in the results (the mocked order is
/// deterministic regardless; only the server-side acceptance matters here).
#[test]
fn live_resource_list_order_by_acceptance() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };
    let project_shortcode = match require_env("DSP_TEST_PROJECT") {
        Some(v) => v,
        None => return,
    };
    let class_iri = match require_env("DSP_TEST_CLASS_IRI") {
        Some(v) => v,
        None => return,
    };

    // ── 2. Collect optional order-by IRI ─────────────────────────────────────
    let order_by_iri = match optional_env("DSP_TEST_ORDER_BY_IRI") {
        Some(v) => v,
        None => {
            eprintln!(
                "skipping live_resource_list_order_by_acceptance: \
                 DSP_TEST_ORDER_BY_IRI not set. \
                 Set it to a full property IRI for the class in DSP_TEST_CLASS_IRI \
                 (e.g. for incunabula/page on dev: \
                 http://api.dev.dasch.swiss/ontology/0803/incunabula/v2#seqnum)."
            );
            return;
        }
    };

    // The IRI bypass path requires a full IRI (contains "://").
    assert!(
        order_by_iri.contains("://"),
        "DSP_TEST_ORDER_BY_IRI must be a full property IRI (contains '://'); \
         got: {order_by_iri:?}"
    );

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 3. Resolve optional token ─────────────────────────────────────────────
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test: DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test: DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 4. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 5. Resolve the project ────────────────────────────────────────────────
    eprintln!("live test: resolving project '{}' on {}", project_shortcode, cfg.server);

    let proj = client.resolve_project(&cfg.server, &project_shortcode).expect(
        "resolve_project failed — check DSP_TEST_SERVER, DSP_TEST_PROJECT, \
             and network connectivity",
    );

    eprintln!("live test: resolved project '{}' (IRI: {})", proj.shortname, proj.iri);

    // ── 6. Call list_resources with order_by set ─────────────────────────────
    //
    // Uses the full-IRI bypass path: DSP_TEST_ORDER_BY_IRI is a complex-schema
    // property IRI, passed directly to `orderByProperty` without resolution.
    // This is the production path when the caller already has the IRI.
    eprintln!(
        "live test: calling list_resources with order_by={:?} for class '{}' (page 0)",
        order_by_iri, class_iri
    );

    let result = client.list_resources(&cfg.server, &proj.iri, &class_iri, Some(order_by_iri.as_str()), 0, token_ref);

    // ── 7. Assert acceptance (200, no error) ─────────────────────────────────
    //
    // The key assertion: the DSP-API accepted the property IRI as a valid
    // `orderByProperty` (i.e. `iriConverter.asPropertyIri` did not return
    // BadRequestException / HTTP 400). Any error here indicates either:
    //   - The IRI is malformed or not a complex-schema IRI.
    //   - The IRI does not belong to a property in the server's ontology.
    //   - The server rejected the param for another reason.
    let page = result.expect(
        "list_resources with order_by failed — \
         check DSP_TEST_ORDER_BY_IRI is a valid complex-schema property IRI \
         for the class in DSP_TEST_CLASS_IRI on DSP_TEST_SERVER",
    );

    eprintln!(
        "live test: order_by acceptance PASSED — \
         received {} resource(s) on page 0 with orderByProperty={:?} accepted by server. \
         may_have_more_results: {}.",
        page.resources.len(),
        order_by_iri,
        page.may_have_more_results
    );
}
