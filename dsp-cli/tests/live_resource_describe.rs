// Live integration test for `dsp vre resource describe`.
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// dsp-cli/ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//   DSP_TEST_PROJECT  — project shortcode (e.g. "0803" for incunabula)
//   DSP_TEST_CLASS_IRI — full class IRI to list resources for (used to obtain
//                       a real resource IRI to feed into describe)
//                       (e.g. "http://api.dasch.swiss/ontology/0803/incunabula/v2#page")
//
// Token supply (optional):
//   DSP_TOKEN         — bearer token; if set, exercises the authenticated path.
//                       NEVER logged or interpolated in failure messages.
//
// This test derives a real resource IRI by listing the first page of resources
// for the given class, then calls `describe_resource` on the first result.
// If the list returns zero resources the describe assertions are skipped
// (eprintln, not a test failure).
//
// D4 hard assertions:
//   - `label`            non-empty (complex always carries rdfs:label)
//   - `resource_type`    non-empty (extracted from @type; degrades to "unknown" not empty)
//   - `iri`              non-empty and starts with "http"
//   - `creation_date`    is Some (complex always carries knora-api:creationDate)
//   - `attached_project` is Some (complex always carries knora-api:attachedToProject)
//   - `owner`            is Some (complex always carries knora-api:attachedToUser)
//
// Report-only (eprintln, never assert):
//   - `ark_url`       — present in complex but may be absent on some servers
//   - `last_modified` — server-side optional (never-modified resource has none)
//   - `visibility`    — derived from the ACL; depends on server permission setup
//   - `your_access`   — derived from the caller's effective permission code
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;

mod common;
use common::{optional_env, require_env};

// ---------------------------------------------------------------------------
// Live test — describe_resource envelope field assertion
// ---------------------------------------------------------------------------

/// End-to-end live test for `dsp vre resource describe` against a real DSP instance.
///
/// Derives a real resource IRI from the first page of `list_resources` (using the
/// same env vars as `live_resource_list`) so no additional env var is needed.
///
/// **D4 hard assertions** (2026-06-17 — command uses `schema=complex`):
/// - `label` must be non-empty.
/// - `resource_type` must be non-empty.
/// - `iri` must be non-empty and start with "http".
/// - `creation_date` must be `Some` — complex always carries `knora-api:creationDate`.
/// - `attached_project` must be `Some` — complex always carries `knora-api:attachedToProject`.
/// - `owner` must be `Some` — complex always carries `knora-api:attachedToUser`.
///
/// Report-only conditions (not a test failure):
/// - `ark_url` — present in complex but surfaced as informational.
/// - `last_modified` — server-side optional (never-modified resource may have none).
/// - `visibility` — derived from ACL; depends on server permission setup.
/// - `your_access` — derived from caller's effective permission; depends on auth.
///
/// Skips cleanly (with `eprintln!`) if required env vars are absent or the
/// resource list is empty.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_resource_describe_envelope_field_assertion() {
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

    // ── 5. List resources (page 0) to obtain a real resource IRI ─────────────
    assert!(
        class_iri.contains("://"),
        "DSP_TEST_CLASS_IRI must be a full IRI (contains '://'); got: {class_iri:?}"
    );

    eprintln!(
        "live test: calling list_resources for class '{}' on project '{}' (page 0) \
         to obtain a resource IRI for describe",
        class_iri, proj.iri
    );

    let page = client
        .list_resources(&cfg.server, &proj.iri, &class_iri, None, 0, token_ref)
        .expect(
            "list_resources failed — check DSP_TEST_SERVER, DSP_TEST_PROJECT, \
             DSP_TEST_CLASS_IRI and network connectivity",
        );

    eprintln!("live test: received {} resource(s) on page 0", page.resources.len());

    if page.resources.is_empty() {
        eprintln!(
            "live test: WARNING — list_resources returned 0 resources for class '{}' \
             in project '{}'. describe assertions skipped (no resource IRI to describe). \
             Check that the project has resources of this type.",
            class_iri, proj.shortname
        );
        return;
    }

    let first_iri = &page.resources[0].iri;
    assert!(
        !first_iri.is_empty(),
        "first resource from list_resources must have a non-empty IRI"
    );

    eprintln!("live test: using resource IRI '{}' for describe call", first_iri);

    // ── 6. Call describe_resource ─────────────────────────────────────────────
    let detail = client.describe_resource(&cfg.server, first_iri, token_ref, false).expect(
        "describe_resource failed — check DSP_TEST_SERVER and network connectivity; \
             resource IRI may be inaccessible to anonymous callers if DSP_TOKEN is not set",
    );

    eprintln!(
        "live test: describe_resource returned — label={:?} iri={} resource_type={} \
         creation_date={:?} attached_project={:?} owner={:?} \
         ark_url={:?} last_modified={:?} visibility={:?} your_access={:?}",
        detail.label,
        detail.iri,
        detail.resource_type,
        detail.creation_date,
        detail.attached_project,
        detail.owner,
        detail.ark_url,
        detail.last_modified,
        detail.visibility,
        detail.your_access,
    );

    // ── 7. D4 hard assertions ─────────────────────────────────────────────────
    //
    // These fields are always present on a real resource when using schema=complex.
    // Failures here indicate a broken DTO extraction or a server regression.

    // -- label (hard assertion) --
    assert!(
        !detail.label.is_empty(),
        "D4 HARD FAIL: `label` is empty on the described resource (iri={:?}, server={:?}). \
         schema=complex always carries rdfs:label. DTO extraction may be broken.",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'label': {:?} ✓", detail.label);

    // -- resource_type (hard assertion) --
    assert!(
        !detail.resource_type.is_empty(),
        "D4 HARD FAIL: `resource_type` is empty on the described resource \
         (iri={:?}, server={:?}). Expected at least the 'unknown' sentinel.",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'resource_type': {} ✓", detail.resource_type);

    // -- iri (hard assertion) --
    assert!(
        !detail.iri.is_empty(),
        "D4 HARD FAIL: `iri` is empty on the described resource (server={:?}). \
         The @id field is load-bearing.",
        cfg.server
    );
    assert!(
        detail.iri.starts_with("http"),
        "D4 HARD FAIL: `iri` does not start with 'http': {:?} (server={:?})",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'iri': {} ✓", detail.iri);

    // -- creation_date (hard assertion — complex always carries it) --
    assert!(
        detail.creation_date.is_some(),
        "D4 HARD FAIL: `creation_date` (knora-api:creationDate) is None on the described \
         resource (iri={:?}, server={:?}). schema=complex always carries creationDate for \
         every resource. DTO extraction for knora-api:creationDate may be broken.",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'creation_date': {:?} ✓", detail.creation_date);

    // -- attached_project (hard assertion — complex always carries it) --
    assert!(
        detail.attached_project.is_some(),
        "D4 HARD FAIL: `attached_project` (knora-api:attachedToProject) is None on the \
         described resource (iri={:?}, server={:?}). schema=complex always carries \
         attachedToProject for every resource. DTO extraction may be broken.",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'attached_project': {:?} ✓", detail.attached_project);

    // -- owner (hard assertion — complex always carries it) --
    assert!(
        detail.owner.is_some(),
        "D4 HARD FAIL: `owner` (knora-api:attachedToUser) is None on the described \
         resource (iri={:?}, server={:?}). schema=complex always carries attachedToUser \
         for every resource. DTO extraction may be broken.",
        detail.iri,
        cfg.server
    );
    eprintln!("live test: D4 field 'owner': {:?} ✓", detail.owner);

    // ── 8. Report-only fields (no assertion) ─────────────────────────────────

    // -- ark_url (report-only) --
    if detail.ark_url.is_some() {
        eprintln!("live test: D4 field 'ark_url': {:?} (present)", detail.ark_url);
    } else {
        eprintln!(
            "live test: D4 NOTE — 'ark_url' is None. This is NOT a test failure — \
             some servers or resource types may not carry an ARK URL."
        );
    }

    // -- last_modified (report-only — server-side optional) --
    if detail.last_modified.is_some() {
        eprintln!("live test: D4 field 'last_modified': {:?} (present)", detail.last_modified);
    } else {
        eprintln!(
            "live test: D4 NOTE — 'last_modified' is None. This is NOT a test failure — \
             a resource that has never been modified has no lastModificationDate \
             (server-side optional)."
        );
    }

    // -- visibility (report-only — depends on server permission setup) --
    eprintln!(
        "live test: D4 field 'visibility' (report-only, derived from ACL): {:?}",
        detail.visibility
    );

    // -- your_access (report-only — depends on caller's auth) --
    // NEVER interpolate the token in any assertion or message.
    eprintln!(
        "live test: D4 field 'your_access' (report-only, derived from caller's permission): {:?}",
        detail.your_access
    );

    eprintln!(
        "live test: PASSED — describe_resource D4 envelope checkpoint complete \
         (schema=complex). label ✓, resource_type ✓, iri ✓, creation_date ✓, \
         attached_project ✓, owner ✓."
    );
}

// ---------------------------------------------------------------------------
// Live test — describe_resource --values (Phase 8c)
// ---------------------------------------------------------------------------

/// End-to-end live test for `dsp vre resource describe --values` against a real
/// DSP instance.
///
/// Uses the known-good `incunabula:Page` resource on `dev`:
///   IRI: `http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw`
///   Project shortcode: 0803
///
/// This test is **not** derived from list_resources — we use the fixed IRI
/// because it is known to carry text, integer, link, and file values (verified
/// live 2026-06-17, plan 022 D4). If the resource is absent on the configured
/// server the test skips with a note.
///
/// **D4 hard assertions** (values present with --values):
/// - `values` is `Some`.
/// - At least one `text` value is present.
/// - At least one `integer` value is present.
/// - At least one `link` value is present.
/// - At least one `still-image` (or other file) value is present.
/// - Every `link` value that has a `target_label` has a non-empty label (the complex schema embeds
///   the target; this live-verifies the parse).
///
/// **Report-only** (never a test failure):
/// - Whether field labels were resolved from the ontology.
/// - Whether vocabulary-item labels were resolved from /v2/node.
/// - Whether standoff text was found (see NOTE below).
///
/// **Standoff / formatted text NOTE:**
/// The `incunabula:Page` carries ONLY unformatted text values; no `textValueAsXml`
/// field is present. The standoff path (`textValueAsXml` → html_to_text) is
/// live-verified by the separate `live_resource_describe_values_standoff` test
/// (against a DaSCH-project resource on `dev` that carries formatted text) and by a
/// wiremock fixture in `tests/resource_describe_values_http.rs`
/// (`standoff_text_xml_is_stripped`).
///
/// **Per-value comment NOTE:**
/// Per-value comments (`knora-api:valueHasComment`) are empirically rare — absent
/// from every sampled 0810-project class and from this `incunabula:Page` resource
/// (per dsp-cli/ADR-0013's empirical note). This test does **not** assert a comment is
/// present. The authoritative parse/render coverage for per-value comments is the
/// wiremock fixture in `tests/resource_describe_values_http.rs`
/// (`value_with_comment_is_parsed_end_to_end`); this live test is best-effort only.
///
/// Skips cleanly (with `eprintln!`) if required env vars are absent or
/// the resource is inaccessible.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_resource_describe_values() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test (values): DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test (values): DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 3. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 4. Call describe_resource with_values=true ────────────────────────────
    //
    // Fixed IRI: the known-good incunabula:Page that carries text, integer, link,
    // and still-image values. This resource is public on dev.
    let fixed_iri = "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw";

    eprintln!(
        "live test (values): calling describe_resource(with_values=true) for IRI {} on {}",
        fixed_iri, cfg.server
    );

    let detail = match client.describe_resource(&cfg.server, fixed_iri, token_ref, true) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "live test (values): SKIP — describe_resource failed ({}). \
                 Resource may be inaccessible on this server or DSP_TEST_SERVER \
                 does not point to a server with the incunabula/0803 project.",
                e
            );
            return;
        }
    };

    // ── 5. D4 hard assertions — values is Some ────────────────────────────────

    let fields = match detail.values {
        Some(f) => f,
        None => {
            panic!(
                "D4 HARD FAIL: `values` is None after describe_resource(with_values=true). \
                 The HTTP client must return Some when with_values=true. \
                 (iri={:?}, server={:?})",
                fixed_iri, cfg.server
            );
        }
    };

    eprintln!("live test (values): received {} field(s)", fields.len());

    for f in &fields {
        eprintln!(
            "live test (values): field '{}' (label={:?}): {} value(s)",
            f.name,
            f.label,
            f.values.len()
        );
        for v in &f.values {
            eprintln!("  value_type={}", v.content.value_type_token());
        }
    }

    // Hard assertions: at least one of the expected value types must be present.
    let all_values: Vec<&dsp_cli::model::ValueContent> =
        fields.iter().flat_map(|f| f.values.iter().map(|v| &v.content)).collect();

    let has_text = all_values.iter().any(|v| matches!(v, dsp_cli::model::ValueContent::Text(_)));
    assert!(
        has_text,
        "D4 HARD FAIL: no `text` value found in --values output. \
         incunabula:Page is known to carry text values. \
         Check the value parsing in src/client/http.rs. \
         Fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
    eprintln!("live test (values): text value present ✓");

    let has_integer = all_values.iter().any(|v| matches!(v, dsp_cli::model::ValueContent::Integer(_)));
    assert!(
        has_integer,
        "D4 HARD FAIL: no `integer` value found in --values output. \
         incunabula:Page is known to carry integer (sequence number) values. \
         Fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
    eprintln!("live test (values): integer value present ✓");

    let has_link = all_values
        .iter()
        .any(|v| matches!(v, dsp_cli::model::ValueContent::Link { .. }));
    assert!(
        has_link,
        "D4 HARD FAIL: no `link` value found in --values output. \
         incunabula:Page is known to carry link values (isPartOfBook). \
         Fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
    eprintln!("live test (values): link value present ✓");

    let has_file = all_values.iter().any(|v| matches!(v, dsp_cli::model::ValueContent::File(_)));
    assert!(
        has_file,
        "D4 HARD FAIL: no `file` value found in --values output. \
         incunabula:Page is known to carry still-image file values. \
         Fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
    eprintln!("live test (values): file value present ✓");

    // Hard assertion: every link with a target_label must be non-empty.
    // (Verifies that the embedded complex-schema target label is correctly extracted.)
    for f in &fields {
        for v in &f.values {
            if let dsp_cli::model::ValueContent::Link { target_label: Some(lbl), target_iri } = &v.content {
                assert!(
                    !lbl.is_empty(),
                    "D4 HARD FAIL: link target_label is present but empty \
                     (target_iri={:?}, field={:?})",
                    target_iri,
                    f.name
                );
                eprintln!("live test (values): link target_label non-empty: {:?} [{}] ✓", lbl, target_iri);
            }
        }
    }

    // ── 6. Report-only: field labels and vocabulary-item labels ────────────────
    // (Never a test failure — depends on permission and ontology availability.)

    let labelled_fields: Vec<&str> = fields.iter().filter(|f| f.label.is_some()).map(|f| f.name.as_str()).collect();
    let unlabelled_fields: Vec<&str> = fields.iter().filter(|f| f.label.is_none()).map(|f| f.name.as_str()).collect();

    eprintln!("live test (values): field labels resolved for: {:?}", labelled_fields);
    eprintln!(
        "live test (values): field labels unresolved (None) for: {:?}",
        unlabelled_fields
    );

    let vocabulary_items_with_labels: Vec<(&str, &str)> = fields
        .iter()
        .flat_map(|f| {
            f.values.iter().filter_map(move |v| match &v.content {
                dsp_cli::model::ValueContent::VocabularyItem { label: Some(lbl), .. } => {
                    Some((f.name.as_str(), lbl.as_str()))
                }
                _ => None,
            })
        })
        .collect();
    eprintln!(
        "live test (values): vocabulary-item labels resolved: {:?}",
        vocabulary_items_with_labels
    );

    // Report-only: whether any value carried a per-value comment
    // (`knora-api:valueHasComment`). Never a test failure — see the
    // "Per-value comment NOTE" on this function's doc comment: comments are
    // empirically rare, and wiremock is the authoritative coverage.
    let commented_values: Vec<(&str, &str)> = fields
        .iter()
        .flat_map(|f| {
            f.values
                .iter()
                .filter_map(move |v| v.comment.as_deref().map(|c| (f.name.as_str(), c)))
        })
        .collect();
    eprintln!(
        "live test (values): values with comment: {} {:?}",
        commented_values.len(),
        commented_values
    );

    // ── 7. Standoff note ──────────────────────────────────────────────────────
    // The incunabula:Page carries unformatted text only. The standoff path
    // (textValueAsXml → html_to_text) is live-verified by
    // `live_resource_describe_values_standoff` below and by the wiremock fixture
    // tests/resource_describe_values_http.rs::standoff_text_xml_is_stripped.
    eprintln!(
        "live test (values): NOTE — standoff path verified separately by \
         live_resource_describe_values_standoff."
    );

    eprintln!(
        "live test (values): PASSED — describe_resource --values D4 checkpoint complete. \
         text ✓, integer ✓, link ✓, file ✓, link target_label non-empty ✓."
    );
}

// ---------------------------------------------------------------------------
// Live test — describe_resource --values, standoff (formatted) text (Phase 8c)
// ---------------------------------------------------------------------------

/// Live-verifies the **standoff** text path (`textValueAsXml` → `html_to_text`)
/// against a real resource that carries formatted text.
///
/// Uses a known public resource in the DaSCH project (0810) on `dev` whose
/// `hasDescription` is a formatted-text value (standoff XML on the wire — verified
/// 2026-06-18). The parse must convert the standoff XML to plain text, so the
/// rendered value must contain NO angle-bracket tags and none of the XML scaffolding
/// (`<?xml`, `<text>`, `<p>`, `<strong>`, …).
///
/// **Hard assertions** (when the resource is reachable):
/// - `values` is `Some`.
/// - At least one `text` value is present.
/// - No `text` value contains `<` or the substring `textValueAsXml` (standoff fully stripped to
///   plain text).
///
/// Skips cleanly (`eprintln!`, not a failure) if the resource is inaccessible —
/// e.g. `DSP_TEST_SERVER` does not point at a server carrying the 0810 project.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_resource_describe_values_standoff() {
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // A public DaSCH-project (0810) resource on dev whose description is formatted
    // (standoff) text. Verified to carry `textValueAsXml` on 2026-06-18.
    let standoff_iri = "http://rdfh.ch/0810/78FBnOYlRf6qb2u9tp58HA";

    eprintln!(
        "live test (standoff): describe_resource(with_values=true) for {} on {}",
        standoff_iri, cfg.server
    );

    let detail = match client.describe_resource(&cfg.server, standoff_iri, token_ref, true) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "live test (standoff): SKIP — describe_resource failed ({}). \
                 DSP_TEST_SERVER may not point at a server carrying the 0810 project.",
                e
            );
            return;
        }
    };

    let fields = match detail.values {
        Some(f) => f,
        None => panic!(
            "D4 HARD FAIL: `values` is None after describe_resource(with_values=true) \
             (iri={standoff_iri:?}, server={:?})",
            cfg.server
        ),
    };

    let text_values: Vec<&str> = fields
        .iter()
        .flat_map(|f| f.values.iter())
        .filter_map(|v| match &v.content {
            dsp_cli::model::ValueContent::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        !text_values.is_empty(),
        "D4 HARD FAIL: no `text` value found on the standoff resource (iri={standoff_iri:?}). \
         Expected a formatted-text description."
    );

    for t in &text_values {
        assert!(
            !t.contains('<'),
            "D4 HARD FAIL: a text value still contains a '<' — standoff XML was NOT \
             stripped to plain text by html_to_text. Value: {t:?}"
        );
        assert!(
            !t.contains("textValueAsXml"),
            "D4 HARD FAIL: a text value contains the raw `textValueAsXml` key — the \
             standoff value object leaked instead of being parsed. Value: {t:?}"
        );
    }

    eprintln!(
        "live test (standoff): PASSED — {} text value(s), all stripped to plain text \
         (no '<' tags). Standoff path live-verified.",
        text_values.len()
    );
}
