//! Snapshot tests for `dsp vre resource describe` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (incunabula:Page from project 0803, all fields present):
//!   exercises the full envelope — label, iri, resource_type, ark_url,
//!   creation_date, last_modified, attached_project, owner, visibility (Public),
//!   your_access (View). Shared by prose, json, lines, csv, tsv cells.
//! - **Absent-fields fixture**: `last_modified = None`, `ark_url = None`. Prose
//!   must omit those lines; tabular must show empty cells.
//! - **Project-members visibility**: `visibility = Some(ProjectMembers)`,
//!   `your_access = Some(Manage)` — exercises a different visibility cell.
//! - **Disclosure variants**: anonymous vs. authenticated `filter_warning`.
//!
//! Determinism: these tests call `Renderer::resource_describe(&detail, &meta)`
//! directly with a hand-built `MetaContext`/`ResourceDetail`. They never go
//! through `run_describe_impl`, which reads real env vars. See
//! `docs/dev/testing-strategy.md` and ADR-0009.
//!
//! Vocabulary guard: no raw DSP-API permission codes (`RV`, `V`, `CR`), no
//! `knora-admin:` group names, no `knora-api:` keys in prose output. The
//! canonical translated tokens (`public`, `view`, `project members only`, etc.)
//! from D1 are the only permission vocabulary that should appear.

use dsp_cli::model::{ResourceAccess, ResourceDetail, ResourceVisibility};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── constants ─────────────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests.
const SERVER: &str = "api.dasch.swiss";

// ── D3 filter_warning strings ─────────────────────────────────────────────────

/// D3 anonymous filter_warning: set on every instance-side read, anonymous path.
const ANON_FILTER_WARNING: &str = "results may be filtered; login to see private resources";

/// D3 authenticated filter_warning: set on every instance-side read, auth path.
const AUTH_FILTER_WARNING: &str = "results limited to your permissions";

// ── MetaContext helpers ───────────────────────────────────────────────────────

fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: Some(ANON_FILTER_WARNING.to_string()),
        count_caveat: None,
        count_cost: None,
    }
}

fn auth_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "authenticated as daisy.duck@dasch.swiss".to_string(),
        filter_warning: Some(AUTH_FILTER_WARNING.to_string()),
        count_caveat: None,
        count_cost: None,
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Main fixture — a realistic incunabula:Page (resource 0803, from the `dev`
/// instance) with all fields populated including last_modified.
///
/// Exercises: label, iri, resource_type, ark_url (Some), creation_date (Some),
/// last_modified (Some), attached_project (Some), owner (Some),
/// visibility (Public), your_access (View).
fn main_detail() -> ResourceDetail {
    ResourceDetail {
        label: "n6r".to_string(),
        iri: "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw".to_string(),
        resource_type: "Page".to_string(),
        ark_url: Some(
            "https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp4SVnGG1DBzFvYErwr".to_string(),
        ),
        creation_date: Some("2011-04-14T07:32:49Z".to_string()),
        last_modified: Some("2024-03-10T15:00:00Z".to_string()),
        attached_project: Some("http://rdfh.ch/projects/3ABR_2i8QYGSIDvmP9mlEw".to_string()),
        owner: Some("http://rdfh.ch/users/IShOmBIGSnO1TXd4-Ty4Sw".to_string()),
        visibility: Some(ResourceVisibility::Public),
        your_access: Some(ResourceAccess::View),
        values: None,
    }
}

/// Absent-fields fixture: `last_modified = None`, `ark_url = None`.
///
/// Exercises omitted-field rendering:
/// - Prose must omit the "ARK:" and "Modified:" lines.
/// - Tabular formats must show empty cells for those columns.
fn absent_fields_detail() -> ResourceDetail {
    ResourceDetail {
        label: "Folio 1r".to_string(),
        iri: "http://rdfh.ch/0803/res-folio-1r".to_string(),
        resource_type: "Page".to_string(),
        ark_url: None,
        creation_date: Some("2023-04-10T08:00:00Z".to_string()),
        last_modified: None, // Never modified.
        attached_project: Some("http://rdfh.ch/projects/0803".to_string()),
        owner: Some("http://rdfh.ch/users/daisy-duck".to_string()),
        visibility: Some(ResourceVisibility::Public),
        your_access: Some(ResourceAccess::View),
        values: None,
    }
}

/// Project-members visibility fixture.
///
/// Exercises the `ProjectMembers` / `Manage` enum arms.
fn project_members_detail() -> ResourceDetail {
    ResourceDetail {
        label: "Private Folio".to_string(),
        iri: "http://rdfh.ch/0803/private-res".to_string(),
        resource_type: "Page".to_string(),
        ark_url: Some("https://ark.stage.dasch.swiss/ark:/72163/1/0803/private".to_string()),
        creation_date: Some("2022-01-10T12:00:00Z".to_string()),
        last_modified: None,
        attached_project: Some("http://rdfh.ch/projects/0803".to_string()),
        owner: Some("http://rdfh.ch/users/project-admin".to_string()),
        visibility: Some(ResourceVisibility::ProjectMembers),
        your_access: Some(ResourceAccess::Manage),
        values: None,
    }
}

/// Public-restricted visibility fixture.
///
/// Exercises the `PublicRestricted` / `RestrictedView` enum arms.
/// Models a resource whose ACL grants `RV` (restricted view) to `UnknownUser`.
fn public_restricted_detail() -> ResourceDetail {
    ResourceDetail {
        label: "Restricted View Folio".to_string(),
        iri: "http://rdfh.ch/0803/restricted-res".to_string(),
        resource_type: "Page".to_string(),
        ark_url: Some("https://ark.stage.dasch.swiss/ark:/72163/1/0803/restricted".to_string()),
        creation_date: Some("2021-06-01T10:00:00Z".to_string()),
        last_modified: None,
        attached_project: Some("http://rdfh.ch/projects/0803".to_string()),
        owner: Some("http://rdfh.ch/users/project-admin".to_string()),
        visibility: Some(ResourceVisibility::PublicRestricted),
        your_access: Some(ResourceAccess::RestrictedView),
        values: None,
    }
}

/// Logged-in-users visibility fixture.
///
/// Exercises the `LoggedInUsers` / `View` enum arms.
/// Models a resource whose ACL grants `V` to `KnownUser` but has no `UnknownUser` entry.
fn logged_in_users_detail() -> ResourceDetail {
    ResourceDetail {
        label: "Logged-In-Only Folio".to_string(),
        iri: "http://rdfh.ch/0803/logged-in-res".to_string(),
        resource_type: "Page".to_string(),
        ark_url: Some("https://ark.stage.dasch.swiss/ark:/72163/1/0803/logged-in".to_string()),
        creation_date: Some("2020-03-15T08:00:00Z".to_string()),
        last_modified: None,
        attached_project: Some("http://rdfh.ch/projects/0803".to_string()),
        owner: Some("http://rdfh.ch/users/project-member".to_string()),
        visibility: Some(ResourceVisibility::LoggedInUsers),
        your_access: Some(ResourceAccess::View),
        values: None,
    }
}

// ── main fixture × 5 formats (anonymous + D3 disclosure) ─────────────────────

/// Prose render of the main fixture (anonymous).
///
/// Locks:
/// - header `Resource: n6r`.
/// - aligned `Type:`, `IRI:`, `ARK:`, `Created:`, `Modified:`, `Project:`,
///   `Owner:`, `Visibility:`, `Your access:` block.
/// - D3 footer with anonymous filter_warning.
/// - No raw permission codes (`RV`, `CR`, etc.) or `knora-admin:` vocab.
#[test]
fn resource_describe_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Structural assertions.
    assert!(
        out.contains("Resource: n6r"),
        "prose must start with 'Resource: n6r'; got:\n{out}"
    );
    assert!(
        out.contains("Type:"),
        "prose must have 'Type:' field; got:\n{out}"
    );
    assert!(
        out.contains("Page"),
        "prose must show resource_type 'Page'; got:\n{out}"
    );
    assert!(
        out.contains("Visibility:"),
        "prose must show 'Visibility:' field; got:\n{out}"
    );
    assert!(
        out.contains("public"),
        "prose must show translated visibility 'public'; got:\n{out}"
    );
    assert!(
        out.contains("Your access:"),
        "prose must show 'Your access:' field; got:\n{out}"
    );
    assert!(
        out.contains("view"),
        "prose must show translated access 'view'; got:\n{out}"
    );
    assert!(
        out.contains(ANON_FILTER_WARNING),
        "prose must contain anonymous filter_warning; got:\n{out}"
    );
    // Vocabulary guard: no raw permission codes.
    assert!(
        !out.contains("knora-admin:"),
        "prose must not contain 'knora-admin:'; got:\n{out}"
    );
    assert!(
        !out.contains("knora-api:"),
        "prose must not contain 'knora-api:'; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the main fixture (anonymous).
///
/// Locks:
/// - `_meta` with `server`, `auth`, `exit_code`, and `note` (D3 filter_warning).
/// - `data` is a single object (not array), ADR-0003.
/// - Keys in deterministic order, `None` → JSON null.
/// - `visibility` = `"public"`, `your_access` = `"view"` (translated tokens).
#[test]
fn resource_describe_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value =
        serde_json::from_str(out.trim()).expect("json must be valid JSON");
    // _meta checks.
    assert!(
        parsed["_meta"]["note"].as_str().is_some(),
        "json _meta must have 'note' for instance-side command; got:\n{out}"
    );
    assert_eq!(
        parsed["_meta"]["note"].as_str().unwrap(),
        ANON_FILTER_WARNING,
        "json _meta.note must equal the anonymous filter_warning"
    );
    // data is an object, not array (ADR-0003 single-object envelope).
    assert!(
        parsed["data"].is_object(),
        "json data must be an object for describe; got:\n{out}"
    );
    let data = &parsed["data"];
    assert_eq!(data["label"].as_str().unwrap(), "n6r");
    assert_eq!(
        data["iri"].as_str().unwrap(),
        "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw"
    );
    assert_eq!(data["resource_type"].as_str().unwrap(), "Page");
    // Translated vocabulary — no raw DSP-API codes.
    assert_eq!(
        data["visibility"].as_str().unwrap(),
        "public",
        "visibility must be the translated token 'public'"
    );
    assert_eq!(
        data["your_access"].as_str().unwrap(),
        "view",
        "your_access must be the translated token 'view'"
    );
    // Vocabulary guard: no raw codes in the JSON string.
    assert!(
        !out.contains("knora-admin:"),
        "json must not contain 'knora-admin:'; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// Lines render of the main fixture (anonymous).
///
/// Locks: tab-separated row on stdout, D3 disclosure on stderr.
#[test]
fn resource_describe_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Disclosure on stderr.
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "lines stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_describe_lines_stdout", stdout);
    insta::assert_snapshot!("resource_describe_lines_stderr", stderr);
}

/// CSV render of the main fixture (anonymous).
///
/// Locks: header `label,iri,resource_type,ark_url,…`, data row, D3 on stderr.
#[test]
fn resource_describe_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stdout.starts_with("label,iri,resource_type,ark_url,creation_date,last_modified,attached_project,owner,visibility,your_access\n"),
        "CSV header must be label,iri,resource_type,…; got:\n{stdout}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "csv stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_describe_csv_stdout", stdout);
    insta::assert_snapshot!("resource_describe_csv_stderr", stderr);
}

/// TSV render of the main fixture (anonymous).
///
/// Locks: header `label\tiri\t…`, data row, D3 on stderr.
#[test]
fn resource_describe_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stdout.starts_with("label\tiri\tresource_type\tark_url\tcreation_date\tlast_modified\tattached_project\towner\tvisibility\tyour_access\n"),
        "TSV header must be label\\tiri\\t…; got:\n{stdout}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "tsv stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_describe_tsv_stdout", stdout);
    insta::assert_snapshot!("resource_describe_tsv_stderr", stderr);
}

// ── D3 disclosure variants: anonymous vs authenticated ────────────────────────

/// Authenticated prose render.
///
/// Locks: authenticated filter_warning in footer; "authenticated as daisy.duck@dasch.swiss".
#[test]
fn resource_describe_disclosure_authenticated_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &auth_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains(AUTH_FILTER_WARNING),
        "authenticated prose must contain the authenticated filter_warning; got:\n{out}"
    );
    assert!(
        out.contains("authenticated as"),
        "authenticated prose footer must say 'authenticated as'; got:\n{out}"
    );
    assert!(
        out.contains("daisy.duck@dasch.swiss"),
        "authenticated prose footer must include the user email; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_disclosure_authenticated_prose", out);
}

/// Authenticated JSON render: `_meta.note` must carry the authenticated warning.
#[test]
fn resource_describe_disclosure_authenticated_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &auth_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert_eq!(
        parsed["_meta"]["note"].as_str().unwrap_or(""),
        AUTH_FILTER_WARNING,
        "authenticated json _meta.note must carry authenticated filter_warning"
    );
    insta::assert_snapshot!("resource_describe_disclosure_authenticated_json", out);
}

// ── Visibility variants: public vs project members only ──────────────────────

/// Prose render with `visibility = ProjectMembers` and `your_access = Manage`.
///
/// Locks translated tokens `"project members only"` and `"manage"` in output.
#[test]
fn resource_describe_prose_project_members_visibility() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&project_members_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("project members only"),
        "prose must show 'project members only' for ProjectMembers visibility; got:\n{out}"
    );
    assert!(
        out.contains("manage"),
        "prose must show 'manage' for Manage access; got:\n{out}"
    );
    // Vocabulary guard: translated tokens only.
    assert!(
        !out.contains("CR"),
        "prose must not contain raw 'CR' permission code; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_prose_project_members", out);
}

/// JSON render with `visibility = ProjectMembers`.
///
/// Locks `"visibility": "project members only"` and `"your_access": "manage"`.
#[test]
fn resource_describe_json_project_members_visibility() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&project_members_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert_eq!(
        parsed["data"]["visibility"].as_str().unwrap(),
        "project members only",
        "visibility must be 'project members only'"
    );
    assert_eq!(
        parsed["data"]["your_access"].as_str().unwrap(),
        "manage",
        "your_access must be 'manage'"
    );
    insta::assert_snapshot!("resource_describe_json_project_members", out);
}

// ── Omitted-field rendering ───────────────────────────────────────────────────

/// Prose render with `last_modified = None`, `ark_url = None`.
///
/// Locks that prose omits the "ARK:" and "Modified:" lines entirely.
#[test]
fn resource_describe_prose_absent_fields() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&absent_fields_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("ARK:"),
        "prose must omit 'ARK:' line when ark_url is None; got:\n{out}"
    );
    assert!(
        !out.contains("Modified:"),
        "prose must omit 'Modified:' line when last_modified is None; got:\n{out}"
    );
    // Other fields must still be present.
    assert!(
        out.contains("Type:"),
        "prose must still show 'Type:' even with absent optional fields; got:\n{out}"
    );
    assert!(
        out.contains("IRI:"),
        "prose must still show 'IRI:'; got:\n{out}"
    );
    assert!(
        out.contains("Created:"),
        "prose must still show 'Created:' when creation_date is present; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_prose_absent_fields", out);
}

/// JSON render with `last_modified = None`, `ark_url = None`.
///
/// Locks that JSON renders `null` for absent optional fields (not omitted).
#[test]
fn resource_describe_json_absent_fields() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&absent_fields_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert!(
        parsed["data"]["ark_url"].is_null(),
        "json must show null for ark_url when None; got:\n{out}"
    );
    assert!(
        parsed["data"]["last_modified"].is_null(),
        "json must show null for last_modified when None; got:\n{out}"
    );
    // Keys must still be present (null, not absent).
    assert!(
        parsed["data"].get("ark_url").is_some(),
        "json must include ark_url key even when None; got:\n{out}"
    );
    assert!(
        parsed["data"].get("last_modified").is_some(),
        "json must include last_modified key even when None; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_json_absent_fields", out);
}

/// CSV render with `last_modified = None`, `ark_url = None`.
///
/// Locks that absent fields render as empty CSV cells (not "null" or omitted).
#[test]
fn resource_describe_csv_absent_fields() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&absent_fields_detail(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    // The header row is present; data row has empty cells for absent fields.
    // We can't easily assert exact column positions without splitting, so
    // assert that "null" does NOT appear (empty string, not the word null).
    assert!(
        !stdout.contains("null"),
        "csv must use empty cells, not 'null', for absent optional fields; got:\n{stdout}"
    );
    insta::assert_snapshot!("resource_describe_csv_absent_fields", stdout);
}

// ── stdout/stderr contract assertions ────────────────────────────────────────

/// Lines: disclosure must land on stderr, NOT stdout.
#[test]
fn resource_describe_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "lines: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "lines: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// CSV: disclosure must land on stderr, NOT stdout.
#[test]
fn resource_describe_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "csv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "csv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// TSV: disclosure must land on stderr, NOT stdout.
#[test]
fn resource_describe_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "tsv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "tsv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

// ── vocabulary guard ──────────────────────────────────────────────────────────

/// Prose output must not contain DSP-API vocabulary leaks.
#[test]
fn resource_describe_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("knora-admin"),
        "prose must not contain 'knora-admin'; got:\n{out}"
    );
    // Note: IRIs are rendered in the IRI/Project/Owner fields, so "ontolog" CAN
    // appear legitimately in IRI values. We only check that the key/label words
    // don't leak DSP-API vocabulary.
    assert!(
        !out.contains("class"),
        "prose must not contain 'class' as DSP-API vocab; got:\n{out}"
    );
    assert!(
        !out.contains("property"),
        "prose must not contain 'property' as DSP-API vocab; got:\n{out}"
    );
}

/// JSON output: no raw permission codes as visibility/access values.
#[test]
fn resource_describe_json_no_raw_permission_codes() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    let visibility = parsed["data"]["visibility"].as_str().unwrap_or("");
    let your_access = parsed["data"]["your_access"].as_str().unwrap_or("");
    // Must be translated tokens, not raw codes.
    assert!(
        !["RV", "V", "M", "D", "CR"].contains(&visibility),
        "visibility must not be a raw permission code; got: {visibility:?}"
    );
    assert!(
        !["RV", "V", "M", "D", "CR"].contains(&your_access),
        "your_access must not be a raw permission code; got: {your_access:?}"
    );
}

// ── Visibility variants: public (restricted view) and logged-in users ────────

/// Prose render with `visibility = PublicRestricted` and `your_access = RestrictedView`.
///
/// Locks translated tokens `"public (restricted view)"` and `"restricted view"` in output.
#[test]
fn resource_describe_prose_public_restricted_visibility() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&public_restricted_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("public (restricted view)"),
        "prose must show 'public (restricted view)' for PublicRestricted visibility; got:\n{out}"
    );
    assert!(
        out.contains("restricted view"),
        "prose must show 'restricted view' for RestrictedView access; got:\n{out}"
    );
    // Vocabulary guard: no raw codes or knora-admin vocab.
    assert!(
        !out.contains("RV"),
        "prose must not contain raw 'RV' permission code; got:\n{out}"
    );
    assert!(
        !out.contains("knora-admin:"),
        "prose must not contain 'knora-admin:'; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_prose_public_restricted", out);
}

/// JSON render with `visibility = PublicRestricted`.
///
/// Locks `"visibility": "public (restricted view)"` and `"your_access": "restricted view"`.
#[test]
fn resource_describe_json_public_restricted_visibility() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&public_restricted_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert_eq!(
        parsed["data"]["visibility"].as_str().unwrap(),
        "public (restricted view)",
        "visibility must be 'public (restricted view)'"
    );
    assert_eq!(
        parsed["data"]["your_access"].as_str().unwrap(),
        "restricted view",
        "your_access must be 'restricted view'"
    );
    // Vocabulary guard.
    assert!(
        !out.contains("knora-admin:"),
        "json must not contain 'knora-admin:'; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_json_public_restricted", out);
}

/// Prose render with `visibility = LoggedInUsers` and `your_access = View`.
///
/// Locks translated token `"logged-in users"` in output.
#[test]
fn resource_describe_prose_logged_in_users_visibility() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&logged_in_users_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("logged-in users"),
        "prose must show 'logged-in users' for LoggedInUsers visibility; got:\n{out}"
    );
    assert!(
        out.contains("view"),
        "prose must show 'view' for View access; got:\n{out}"
    );
    // Vocabulary guard.
    assert!(
        !out.contains("knora-admin:"),
        "prose must not contain 'knora-admin:'; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_prose_logged_in_users", out);
}

/// JSON render with `visibility = LoggedInUsers`.
///
/// Locks `"visibility": "logged-in users"` and `"your_access": "view"`.
#[test]
fn resource_describe_json_logged_in_users_visibility() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_describe(&logged_in_users_detail(), &anon_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
    assert_eq!(
        parsed["data"]["visibility"].as_str().unwrap(),
        "logged-in users",
        "visibility must be 'logged-in users'"
    );
    assert_eq!(
        parsed["data"]["your_access"].as_str().unwrap(),
        "view",
        "your_access must be 'view'"
    );
    // Vocabulary guard.
    assert!(
        !out.contains("knora-admin:"),
        "json must not contain 'knora-admin:'; got:\n{out}"
    );
    insta::assert_snapshot!("resource_describe_json_logged_in_users", out);
}

// ── TSV absent-fields (Fix 4) ────────────────────────────────────────────────

/// TSV render with `last_modified = None`, `ark_url = None`.
///
/// Mirrors the existing CSV absent-fields test: asserts absent fields render as
/// empty TSV cells (not the literal string "null" or anything else).
#[test]
fn resource_describe_tsv_absent_fields() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_describe(&absent_fields_detail(), &anon_meta())
        .unwrap();
    let stdout = buf_to_string(&out_buf);
    // "null" must not appear — absent optional fields must be empty cells.
    assert!(
        !stdout.contains("null"),
        "tsv must use empty cells, not 'null', for absent optional fields; got:\n{stdout}"
    );
    // Header must be present.
    assert!(
        stdout.starts_with("label\tiri\t"),
        "tsv header must start with 'label\\tiri\\t'; got:\n{stdout}"
    );
    insta::assert_snapshot!("resource_describe_tsv_absent_fields", stdout);
}

// ── Prose raw-permission-code guard (Fix 5) ───────────────────────────────────

/// Prose output must not surface raw permission codes as Visibility or Your access values.
///
/// Complements `resource_describe_json_no_raw_permission_codes` for the prose renderer.
/// Uses the main fixture (Public / View) and the project-members fixture (ProjectMembers /
/// Manage) to exercise multiple arms. Asserts that neither `RV`, `V`, `M`, `D`, `CR` appears
/// as a standalone value for these fields, and that `knora-admin:` is absent.
///
/// Guard is targeted: we DO NOT assert the absence of "view" (which appears as a legitimate
/// translated token) — we only check that the raw codes are not present as values.
#[test]
fn resource_describe_prose_no_raw_permission_codes() {
    // Test with main (Public/View) fixture.
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_describe(&main_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // knora-admin: must never appear in prose.
    assert!(
        !out.contains("knora-admin:"),
        "prose (main) must not contain 'knora-admin:'; got:\n{out}"
    );

    // Raw permission codes must not appear as the Visibility or Your access values.
    // We check that "Visibility:" is not immediately followed by a raw code.
    // The canonical translated tokens are "public", "public (restricted view)",
    // "logged-in users", "project members only" — none of which are raw codes.
    for code in &["RV", "CR"] {
        assert!(
            !out.contains(code),
            "prose (main) must not contain raw permission code '{code}'; got:\n{out}"
        );
    }

    // Test with project-members fixture to cover the Manage/CR arm.
    let (buf2, w2) = shared_buf();
    let mut r2 = ProseRenderer::with_writer(w2);
    r2.resource_describe(&project_members_detail(), &anon_meta())
        .unwrap();
    let out2 = buf_to_string(&buf2);

    assert!(
        !out2.contains("knora-admin:"),
        "prose (project-members) must not contain 'knora-admin:'; got:\n{out2}"
    );
    assert!(
        !out2.contains("CR"),
        "prose (project-members) must not contain raw 'CR' code; got:\n{out2}"
    );
    // "manage" must appear (translated token for CR), not "CR".
    assert!(
        out2.contains("manage"),
        "prose (project-members) must contain translated 'manage'; got:\n{out2}"
    );
}
