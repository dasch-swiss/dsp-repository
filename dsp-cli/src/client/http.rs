//! HTTP `DspClient` implementation — real `reqwest::blocking` client.
//!
//! ## DSP-API login endpoint (confirmed from AuthenticationEndpointsV2.scala)
//!
//! **URL**: `POST {server}/v2/authentication`
//!
//! **Request body**: JSON with one identifier key (`email`, `username`, or `iri`)
//! plus a `password` key. The CLI auto-detects the identifier type from the
//! `--user` value and sends the matching key: an `http(s)://` prefix is treated
//! as a user IRI, a value containing `@` as an email address, and anything else
//! as a username. See `identifier_key` for the heuristic.
//!
//! Example (email): `{ "email": "<value>", "password": "<password>" }`
//!
//! **Response body (200)**:
//! ```json
//! { "token": "<jwt-string>" }
//! ```
//! Token only — no `user`, no `expires_at` in the response. The user identity
//! is echoed back from the `--user` argument; expiry is extracted from the JWT
//! via `extract_exp`.
//!
//! **Error status codes**: 401 for bad credentials (treat 401 and 403 alike).
//!
//! **Credentials travel in the POST JSON body**, not in the URL or in any header
//! that reqwest's built-in debug logging captures (i.e. not Authorization header).

use std::io::{Read, Write};
use std::time::Duration;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

use std::collections::{HashMap, HashSet};

use crate::client::DspClient;
use crate::client::builtins::builtin_field_value_type;
use crate::client::jwt::extract_exp;
use crate::diagnostic::Diagnostic;
use crate::model::auth::LoginResponse;
use crate::model::resource::{DatePoint, DateValue, FieldValues, FileValue, Value, ValueContent};
use crate::model::{
    Cardinality, CreateDumpOutcome, DataModel, DataModelDetail, DataModelStructure,
    DataModelSummary, DumpStatus, DumpTask, Field, LocalizedText, Project, ProjectDescription,
    ProjectDetail, ProjectRef, ProjectStatus, Relation, RelationKind, Representation,
    ResourceAccess, ResourceDetail, ResourcePage, ResourceSummary, ResourceTypeDetail,
    ResourceTypeSummary, ResourceVisibility, ValueType, Vocabulary, VocabularyHeader,
    VocabularyNode, VocabularyTree,
};

// ---------------------------------------------------------------------------
// Private wire DTOs (boundary translation — ADR-0001)
// ---------------------------------------------------------------------------

/// Private DSP-API wire type for the login response.
///
/// Stays inside this module — the boundary translation to `LoginResponse`
/// (dsp-cli vocabulary) happens below (ADR-0001).
#[derive(serde::Deserialize)]
struct LoginApiResponse {
    token: String,
}

/// DSP-API response envelope for a single-project lookup.
///
/// `GET /admin/projects/shortcode/{sc}` | `/shortname/{n}` | `/iri/{enc-iri}`
/// all return `{ "project": { "id": "…", "shortcode": "…", "shortname": "…", … } }`.
/// Only the fields the CLI needs are extracted here (private, ADR-0001 boundary).
#[derive(serde::Deserialize)]
struct ProjectGetApiResponse {
    project: ProjectApiDto,
}

#[derive(serde::Deserialize)]
struct ProjectApiDto {
    id: String,
    shortcode: String,
    shortname: String,
}

/// DSP-API wire type for a dump/export task status response.
///
/// JSON wire shape: `{ "id": "…", "status": "in_progress"|"completed"|"failed",
/// "errorMessage": "…", "createdAt": "…" }` (camelCase on the wire; optional
/// fields may be absent). This DTO is private to `http.rs` — the translation to
/// `DumpTask` (dsp-cli vocabulary) happens in `into_dump_task`. See ADR-0001.
#[derive(serde::Deserialize)]
struct DataTaskStatusApiResponse {
    id: String,
    status: String,
    #[serde(default, rename = "errorMessage")]
    error_message: Option<String>,
    /// Raw RFC 3339 timestamp string from the wire. Kept as `Option<String>`
    /// (not `Option<DateTime<Utc>>`) so that a present-but-malformed timestamp
    /// does NOT fail the entire body parse — we parse it best-effort in
    /// `into_dump_task`, and fall back to `None` on failure.
    #[serde(default, rename = "createdAt")]
    created_at: Option<String>,
}

/// DSP-API V3 error envelope for 409 conflict responses.
///
/// Shape on the wire:
/// ```json
/// { "errors": [{ "code": "export_exists", "details": { "id": "…", "projectIri": "…" } }] }
/// ```
/// Private to `http.rs` — the `export_exists` accessor is the only path above this module.
#[derive(serde::Deserialize)]
struct V3ErrorBody {
    #[serde(default)]
    errors: Vec<V3ErrorItem>,
}

#[derive(serde::Deserialize)]
struct V3ErrorItem {
    code: String,
    #[serde(default)]
    details: std::collections::HashMap<String, String>,
}

/// One ontology's resource-class instance counts from the resource-counts
/// endpoint.
///
/// `GET /v3/projects/{enc(project_iri)}/resourcesPerOntology` returns a
/// top-level JSON array of these — not wrapped in an envelope object. Only
/// `classesAndCount` is modelled; the sibling `ontology` object (iri/label/
/// comment) is dropped by serde since `resource_counts` flattens across
/// ontologies (ADR-0001: translation stays in this module).
#[derive(serde::Deserialize)]
struct OntologyAndResourceClassesDto {
    #[serde(rename = "classesAndCount", default)]
    classes_and_count: Vec<ClassAndCountDto>,
}

/// One resource-class's instance count within an ontology, from the
/// resource-counts endpoint.
///
/// `itemCount` counts non-deleted resources but is NOT permission-filtered —
/// see the `resource_counts` trait doc for why this differs from
/// `list_resources`.
#[derive(serde::Deserialize)]
struct ClassAndCountDto {
    #[serde(rename = "resourceClass")]
    resource_class: ResourceClassRefDto,
    #[serde(rename = "itemCount")]
    item_count: u64,
}

/// Bare resource-class reference from the resource-counts endpoint — only the
/// IRI is needed.
#[derive(serde::Deserialize)]
struct ResourceClassRefDto {
    iri: String,
}

/// DSP-API response envelope for the project list endpoint.
///
/// `GET /admin/projects` returns `{ "projects": [ … ] }`. Only the fields the
/// CLI needs for `project list` are extracted here (private, ADR-0001 boundary).
#[derive(serde::Deserialize)]
struct ProjectsListApiResponse {
    projects: Vec<ProjectListItemDto>,
}

/// One project item from the `GET /admin/projects` response.
///
/// Only the fields needed for `project list` are modelled — serde ignores the
/// rest (description, keywords, licences, …) by default. See ADR-0001.
#[derive(serde::Deserialize)]
struct ProjectListItemDto {
    id: String,
    shortname: String,
    shortcode: String,
    #[serde(default)]
    longname: Option<String>,
    /// `status` has NO `#[serde(default)]`: a missing `status` field is a
    /// server-contract change and MUST fail parse loudly (→ `ServerError`),
    /// not silently default to `false`. This mirrors the deliberate care in
    /// [`DataTaskStatusApiResponse::status`] and is intentional. Document any
    /// future change to this decision with an ADR amendment.
    status: bool,
    #[serde(default)]
    ontologies: Vec<String>,
}

/// Private DSP-API wire type for the RICH single-project lookup (describe).
///
/// Distinct from `ProjectApiDto` (resolve_project's lean projection) so each
/// caller owns its own parse contract. Boundary-private (ADR-0001).
///
/// Note: this endpoint is the SAME as resolve_project's (`/admin/projects/…`)
/// but describe parses more fields. A separate DTO keeps parse contracts
/// independent and avoids breaking resolve_project fixtures that omit `status`.
#[derive(serde::Deserialize)]
struct ProjectDetailApiResponse {
    project: ProjectDetailApiDto,
}

#[derive(serde::Deserialize)]
struct ProjectDetailApiDto {
    id: String,
    shortcode: String,
    shortname: String,
    #[serde(default)]
    longname: Option<String>,
    /// No `#[serde(default)]`: a missing `status` is a server-contract change
    /// and must fail parse loudly (→ `ServerError`), mirroring `ProjectListItemDto`.
    status: bool,
    #[serde(default)]
    description: Vec<ProjectDescriptionDto>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    ontologies: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ProjectDescriptionDto {
    value: String,
    #[serde(default)]
    language: Option<String>,
}

/// Wire shape of `GET /v2/ontologies/metadata/{iri}`. JSON-LD returns either a
/// `@graph` array (multiple ontologies), a flattened single object (one
/// ontology, NO `@graph`), or `{}` (none). The flattened-single fields are
/// captured at the top level and reconciled in code. (`@graph` and the
/// flattened fields are mutually exclusive in practice; if both ever appear,
/// `@graph` wins — see the reconciliation comment.)
// Note on `#[serde(default)]`: an `Option<T>` field already deserializes a
// MISSING key to `None` without `default`, so it is omitted on the plain
// `Option` fields below. It is kept ONLY on `last_modification_date` as
// belt-and-braces (see `LastModDto` for the precise absent-vs-malformed
// semantics).
#[derive(serde::Deserialize)]
struct OntologyMetadataResponse {
    #[serde(rename = "@graph")]
    graph: Option<Vec<OntologyMetadataDto>>,
    // Flattened single-ontology case (present only when `@graph` is absent):
    #[serde(rename = "@id")]
    id: Option<String>,
    #[serde(rename = "rdfs:label")]
    label: Option<String>,
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<LastModDto>,
}

#[derive(serde::Deserialize)]
struct OntologyMetadataDto {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "rdfs:label")]
    label: Option<String>,
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<LastModDto>,
}

/// `knora-api:lastModificationDate` is `{"@value": "...", "@type": "..."}`.
/// Typed as `Option<LastModDto>` with `#[serde(default)]` on the containing
/// fields: an ABSENT `knora-api:lastModificationDate` key deserializes to
/// `None` (the `Option` default — `#[serde(default)]` is belt-and-braces here).
/// A PRESENT but malformed value (e.g. missing `@value`) will fail the parse
/// and surface as a `ServerError` — it is NOT silently dropped. This is
/// consistent with this codebase's fail-loud-on-contract-violation stance.
#[derive(serde::Deserialize)]
struct LastModDto {
    #[serde(rename = "@value")]
    value: String,
}

/// Wire shape of `GET /v2/ontologies/allentities/{iri}`. A flat JSON-LD doc:
/// the ontology node's fields at top level, a `@graph` of all entities, and a
/// `@context` prefix map. `allLanguages` is off, so labels are plain strings.
#[derive(serde::Deserialize)]
struct OntologyAllEntitiesResponse {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "rdfs:label")]
    label: Option<String>,
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<LastModDto>,
    #[serde(rename = "@graph", default)]
    graph: Vec<OntologyEntityDto>,
    // Prefix → namespace. Typed as Value (NOT String) on purpose: a JSON-LD
    // `@context` may legitimately carry object-valued term definitions
    // (`"term": {"@id": …, "@type": …}`) we don't consume; a `String`-typed map
    // would make serde FAIL the whole parse on such an entry. We extract only the
    // string-valued prefix entries we need. (Live beol context is all strings, but
    // this keeps a richer context from breaking the read — a deliberate
    // robustness-over-strictness call, see Risks.)
    #[serde(rename = "@context", default)]
    context: HashMap<String, serde_json::Value>,
}

/// One `@graph` entity. Only resource-types are consumed; `is_resource_class`
/// is absent on non-resource nodes (→ false via `default`). `label` is a plain
/// string or absent (allLanguages off).
///
/// ADDITIVE extension for `describe_resource_type` (Step 2, task 016): all new
/// fields are `Option` / `#[serde(default)]` so `describe_data_model` (which
/// shares this struct) continues to parse without change — see R1 in the plan.
/// - `sub_class_of`: heterogeneous list of superclass refs + `owl:Restriction`s;
///   typed as `Vec<serde_json::Value>` because the mix of shapes (bare `@id` vs
///   restriction object with integer cardinality) defeats a single `#[derive]`
///   struct (R9).
/// - `object_type`: `knora-api:objectType` → inner `{"@id": "…"}` object.
/// - `is_link_property`: `knora-api:isLinkProperty` (link fields).
/// - `is_link_value_property`: `knora-api:isLinkValueProperty` (reification twin; dropped).
/// - `is_resource_property`: `knora-api:isResourceProperty`.
/// - `gui_order`: `salsah-gui:guiOrder` on restriction nodes (only meaningful
///   inside `sub_class_of` items, but also present on property nodes for some
///   ontologies; captured here for completeness and parsed from `sub_class_of`
///   elements directly in the classifier).
#[derive(serde::Deserialize)]
struct OntologyEntityDto {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "rdfs:label")]
    label: Option<String>,
    #[serde(rename = "knora-api:isResourceClass", default)]
    is_resource_class: bool,
    /// `rdfs:subClassOf` — heterogeneous list of superclass refs and restrictions.
    /// Typed as `Vec<serde_json::Value>` (R9). `#[serde(default)]` so nodes that
    /// lack this field (property nodes, most non-resource-class nodes) parse to
    /// an empty vec rather than failing.
    #[serde(rename = "rdfs:subClassOf", default)]
    sub_class_of: Vec<serde_json::Value>,
    /// `knora-api:objectType` → `{"@id": "…"}`. Present on property nodes to
    /// identify the value type (or link-target resource class).
    #[serde(rename = "knora-api:objectType")]
    object_type: Option<ObjectTypeDto>,
    /// `knora-api:isLinkProperty` — present (true) on link-property nodes.
    #[serde(rename = "knora-api:isLinkProperty", default)]
    is_link_property: bool,
    /// `knora-api:isLinkValueProperty` — present (true) on reification twin nodes.
    /// These are dropped from the field list (Decision 7 / R-twin).
    #[serde(rename = "knora-api:isLinkValueProperty", default)]
    is_link_value_property: bool,
    /// `knora-api:isResourceProperty` — present (true) on resource-property nodes.
    #[serde(rename = "knora-api:isResourceProperty", default)]
    is_resource_property: bool,
}

/// Inner object for `knora-api:objectType: {"@id": "…"}`.
#[derive(serde::Deserialize, Clone)]
struct ObjectTypeDto {
    #[serde(rename = "@id")]
    id: String,
}

/// Borrowed view of an `export_exists` conflict's details.
///
/// Private to `http.rs` — exposing it would leak wire vocabulary ("export",
/// "projectIri") above the client layer, violating ADR-0001.
struct ExportExists<'a> {
    /// `errors[].details.id`, if present.
    id: Option<&'a str>,
    /// `errors[].details.projectIri`, if present.
    project_iri: Option<&'a str>,
}

impl V3ErrorBody {
    /// Extract the `export_exists` conflict details, if present.
    ///
    /// Returns `None` if no error with `code == "export_exists"` is present.
    /// The returned `ExportExists` fields are each `Option` — callers are
    /// responsible for fail-closed handling of absent `id` or `projectIri`.
    fn export_exists(&self) -> Option<ExportExists<'_>> {
        self.errors
            .iter()
            .find(|e| e.code == "export_exists")
            .map(|e| ExportExists {
                id: e.details.get("id").map(String::as_str),
                project_iri: e.details.get("projectIri").map(String::as_str),
            })
    }
}

impl DataTaskStatusApiResponse {
    /// Translate the API wire response to a [`DumpTask`].
    ///
    /// Status string mapping:
    /// - `"in_progress"` → [`DumpStatus::InProgress`]
    /// - `"completed"`   → [`DumpStatus::Completed`]
    /// - `"failed"`      → [`DumpStatus::Failed`]
    /// - anything else   → `ServerError` (unknown status from the server)
    ///
    /// **Single truncation point**: `error_message` is capped to ≤500 chars
    /// here before being stored in `DumpTask.error_message`. The Step 8 poll-loop
    /// `Failed` branch relies on this invariant — do not bypass this method when
    /// constructing `DumpTask` values from server responses.
    fn into_dump_task(self) -> Result<DumpTask, Diagnostic> {
        // Validate the server-supplied id as it enters the domain model, so an
        // invalid id never travels into a DumpTask (defence in depth — the
        // URL-building sites also validate before embedding it).
        validate_dump_id(&self.id)?;

        let status = match self.status.as_str() {
            "in_progress" => DumpStatus::InProgress,
            "completed" => DumpStatus::Completed,
            "failed" => DumpStatus::Failed,
            other => {
                return Err(Diagnostic::ServerError(format!(
                    "server returned unknown dump status: '{other}'"
                )));
            }
        };

        // Truncate error_message to ≤500 chars. Log the TRUNCATED value at
        // TRACE — raw is server-controlled and may be very large; bounding the
        // trace output keeps TRACE logs predictable.
        let error_message = self.error_message.map(|raw| {
            let truncated = if raw.chars().count() > 500 {
                raw.chars().take(500).collect::<String>()
            } else {
                raw
            };
            tracing::trace!("dump task error_message (truncated): {}", truncated);
            truncated
        });

        // Parse created_at best-effort: a present-but-malformed timestamp must
        // NOT fail the whole parse — created_at is display-only, never load-bearing.
        let created_at = self.created_at.and_then(|s| {
            match chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                Err(_) => {
                    tracing::debug!(raw = %s, "dump task createdAt could not be parsed as RFC3339; using None");
                    None
                }
            }
        });

        Ok(DumpTask {
            id: self.id,
            status,
            error_message,
            created_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Vocabulary wire DTOs (`/admin/lists`) — plan 034
// ---------------------------------------------------------------------------

/// Wire shape of `GET /admin/lists?projectIri={enc}`.
#[derive(serde::Deserialize)]
struct ListsListApiResponse {
    lists: Vec<ListSummaryDto>,
}

/// One entry from the `lists` array. Only the fields `list --count`-free
/// projection needs are modelled (`projectIri`/`isRootNode` are ignored by
/// default) — mirrors `ProjectListItemDto`'s "model only what's needed"
/// precedent.
#[derive(serde::Deserialize)]
struct ListSummaryDto {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    labels: Vec<ListLabelDto>,
    #[serde(default)]
    comments: Vec<ListLabelDto>,
}

/// One `{value, language}` label/comment entry, as DSP-API returns for a
/// list or list node. Same shape as [`ProjectDescriptionDto`], deliberately
/// duplicated rather than shared (see plan 034's BACKLOG note).
#[derive(serde::Deserialize, Clone)]
struct ListLabelDto {
    value: String,
    #[serde(default)]
    language: Option<String>,
}

/// `GET /admin/lists/{enc(iri)}` is polymorphic (Verified API facts, plan
/// 034): a root IRI wraps its payload under `list`, a node IRI under `node`.
/// Modelled as `#[serde(untagged)]` over the two struct shapes below —
/// matched by which key is present, NOT the wire's `type` string — so a
/// response carrying neither key fails parse loudly (surfaces as a
/// `serde_json` error, mapped to `Diagnostic::ServerError` by the caller)
/// rather than degrading. Variant names are deliberately `Root` / `Node`
/// and NOT `Root`/`Subtree`: "subtree" already means D14's user-facing
/// filter; reusing it here would reintroduce the conflation D2 cleaned up.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ListGetResponseDto {
    Root(ListRootResponseDto),
    Node(ListNodeGetResponseDto),
}

#[derive(serde::Deserialize)]
struct ListRootResponseDto {
    list: ListRootDto,
}

#[derive(serde::Deserialize)]
struct ListRootDto {
    listinfo: ListInfoDto,
    #[serde(default)]
    children: Vec<ListNodeDto>,
}

/// Root-list metadata (`listinfo`). Carries `projectIri` — the one thing
/// `nodeinfo` (below) does not; that asymmetry is why a node address must
/// resolve upward to the root (D2) before the cross-project guard can run.
#[derive(serde::Deserialize)]
struct ListInfoDto {
    id: String,
    #[serde(rename = "projectIri")]
    project_iri: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    labels: Vec<ListLabelDto>,
    #[serde(default)]
    comments: Vec<ListLabelDto>,
}

#[derive(serde::Deserialize)]
struct ListNodeGetResponseDto {
    node: ListNodeGetDto,
}

/// Only `nodeinfo.hasRootNode` is modelled here — the node response's own
/// `children` (subtree) is discarded per D2, so it is never parsed.
#[derive(serde::Deserialize)]
struct ListNodeGetDto {
    nodeinfo: ListNodeInfoDto,
}

#[derive(serde::Deserialize)]
struct ListNodeInfoDto {
    #[serde(rename = "hasRootNode")]
    has_root_node: String,
}

/// One node in a vocabulary's tree, recursively. `serde_json`'s 128-frame
/// nesting limit bounds recursive DESERIALIZATION of this shape (real data
/// reaches 9 levels); the DTO→domain conversion below (`convert_list_nodes`)
/// walks this tree ITERATIVELY regardless, since it is the layer closest to
/// untrusted server input.
#[derive(serde::Deserialize)]
struct ListNodeDto {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    labels: Vec<ListLabelDto>,
    #[serde(default)]
    comments: Vec<ListLabelDto>,
    position: i32,
    #[serde(default)]
    children: Vec<ListNodeDto>,
}

/// Boundary translation of the wire `{value, language}` shape into
/// [`LocalizedText`] (ADR-0001). No language filtering or preference — all
/// languages are kept (D4).
fn into_localized_texts(dtos: Vec<ListLabelDto>) -> Vec<LocalizedText> {
    dtos.into_iter()
        .map(|d| LocalizedText {
            value: d.value,
            language: d.language,
        })
        .collect()
}

/// Build a [`VocabularyTree`] from a parsed root response.
///
/// `requested_node` is `Some(iri)` when the originally-addressed IRI turned
/// out to be a node (D2's upward resolution), `None` when the root itself
/// was addressed directly.
fn build_vocabulary_tree(list: ListRootDto, requested_node: Option<String>) -> VocabularyTree {
    VocabularyTree {
        root: VocabularyHeader {
            iri: list.listinfo.id,
            name: list.listinfo.name,
            labels: into_localized_texts(list.listinfo.labels),
            comments: into_localized_texts(list.listinfo.comments),
        },
        children: convert_list_nodes(list.children),
        project_iri: list.listinfo.project_iri,
        requested_node,
    }
}

/// One node under construction while `convert_list_nodes` walks the DTO
/// tree — the explicit stack frame that replaces a recursive call.
struct ListNodeConversionFrame {
    header: VocabularyHeader,
    position: i32,
    /// This node's own children, not yet visited, in position order.
    remaining_children: std::collections::VecDeque<ListNodeDto>,
    /// This node's children already converted, in position order.
    converted_children: Vec<VocabularyNode>,
}

/// Convert a DTO tree (as returned by `/admin/lists/{iri}`'s `children`
/// array) into a position-ordered `Vec<VocabularyNode>`, WITHOUT recursion.
///
/// This is the layer closest to untrusted server input, so the walk uses an
/// explicit stack of "frames to finish" instead of a self-recursive helper
/// function — depth is bounded only by available memory, not the Rust call
/// stack. `serde_json` already bounds recursive DESERIALIZATION at 128
/// frames (see [`ListNodeDto`]); this bounds the conversion step too, for
/// the same untrusted-input reason. Siblings are sorted by `position`
/// defensively at every level, even though the server already does.
fn convert_list_nodes(dtos: Vec<ListNodeDto>) -> Vec<VocabularyNode> {
    fn dto_to_frame(dto: ListNodeDto) -> ListNodeConversionFrame {
        let mut children = dto.children;
        children.sort_by_key(|c| c.position);
        ListNodeConversionFrame {
            header: VocabularyHeader {
                iri: dto.id,
                name: dto.name,
                labels: into_localized_texts(dto.labels),
                comments: into_localized_texts(dto.comments),
            },
            position: dto.position,
            remaining_children: children.into(),
            converted_children: Vec::new(),
        }
    }

    let mut top_level = dtos;
    top_level.sort_by_key(|d| d.position);
    let mut top_level: std::collections::VecDeque<ListNodeDto> = top_level.into();

    let mut result: Vec<VocabularyNode> = Vec::new();
    let mut stack: Vec<ListNodeConversionFrame> = Vec::new();

    loop {
        // Descend: pull the next un-visited child from the current frame (or,
        // if the stack is empty, the next top-level sibling).
        let next_dto = match stack.last_mut() {
            Some(frame) => frame.remaining_children.pop_front(),
            None => top_level.pop_front(),
        };

        match next_dto {
            Some(dto) => stack.push(dto_to_frame(dto)),
            None => {
                // The current frame has no more children to visit — it is
                // fully converted. Pop it and attach it to its parent (or to
                // `result` if the stack is now empty).
                match stack.pop() {
                    Some(frame) => {
                        let node = VocabularyNode {
                            header: frame.header,
                            position: frame.position,
                            children: frame.converted_children,
                        };
                        match stack.last_mut() {
                            Some(parent) => parent.converted_children.push(node),
                            None => result.push(node),
                        }
                    }
                    // Stack empty and no more top-level siblings — done.
                    None => break,
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// DSP-API's `POST /v2/authentication` discriminates the user identifier by
/// which JSON key is present. dsp-cli takes one `--user` value and infers the
/// key: an `http(s)://` prefix → user IRI; an `@` → email; otherwise username.
fn identifier_key(user: &str) -> &'static str {
    if user.starts_with("http://") || user.starts_with("https://") {
        "iri"
    } else if user.contains('@') {
        "email"
    } else {
        "username"
    }
}

/// Classifies a `project` string so `resolve_project` can build the right URL.
///
/// Priority:
/// 1. Starts with `http://` or `https://` → `Iri`.
/// 2. Matches `^[0-9A-Fa-f]{4}$` exactly → `Shortcode`.
/// 3. Anything else → `Shortname`.
///
/// **Overlap note**: a 4-hex-letter string (e.g. `beef`) is classified as
/// `Shortcode` even though it could theoretically be a shortname. This is
/// intentional and documented in the plan (risks §6).
enum ProjectIdent<'a> {
    Iri(&'a str),
    Shortcode(&'a str),
    Shortname(&'a str),
}

fn classify(project: &str) -> ProjectIdent<'_> {
    if project.starts_with("http://") || project.starts_with("https://") {
        ProjectIdent::Iri(project)
    } else if project.len() == 4 && project.chars().all(|c| c.is_ascii_hexdigit()) {
        ProjectIdent::Shortcode(project)
    } else {
        ProjectIdent::Shortname(project)
    }
}

/// Percent-encode an IRI for safe insertion as a single URL path segment.
///
/// Uses `NON_ALPHANUMERIC` — encodes every character that is not `[A-Za-z0-9]`,
/// including `/`, `:`, `?`, `#`, `[`, `]`, `@`, and sub-delimiters. Over-encoding
/// the unreserved chars (`-._~`) is harmless; the Tapir server decodes the segment.
fn enc(iri: &str) -> String {
    utf8_percent_encode(iri, NON_ALPHANUMERIC).to_string()
}

/// Maps an unexpected HTTP status to a `Diagnostic`.
///
/// Reused by `resolve_project`, the ontology reads (`fetch_allentities` and the
/// data-model / resource-type endpoints), and the dump methods to keep
/// unexpected-status handling DRY. `401`/`403` map to `AuthRequired` (exit 3,
/// ADR-0012) with a re-authenticate hint — the common case is a cached token
/// that has expired (surfaced by `dsp auth status`), which previously fell
/// through to a bare "unexpected status 401" runtime error. Endpoints needing a
/// tailored auth message (e.g. the dump commands' "system-administrator token"
/// wording) keep their own explicit `401 | 403` arm and never reach this
/// fallback for those statuses; the `login` method retains fully inline handling.
fn map_unexpected_status(status: reqwest::StatusCode, url: &str) -> Diagnostic {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        // ADR-0007: treat 401 and 403 alike. A read refused here is usually a
        // missing or expired cached token, so point the user at re-authentication
        // rather than emitting a bare runtime error.
        Diagnostic::AuthRequired(
            "your token may be missing, expired, or lack permission — run \
             `dsp auth login` to (re)authenticate"
                .into(),
        )
    } else if status.is_server_error() {
        Diagnostic::ServerError(format!("server returned {status} for {url}"))
    } else {
        Diagnostic::ServerError(format!("unexpected status {status} for {url}"))
    }
}

/// Validate that a `dump_id` returned by the server is URL-safe.
///
/// DSP-API returns dump IDs in URL-safe base64 form (`[A-Za-z0-9_-]+`). Before
/// inserting a dump_id verbatim into a URL path segment we verify it matches
/// this shape — a malformed id from a rogue or buggy server must not silently
/// corrupt the URL. Percent-encoding is intentionally NOT applied (that would
/// mangle valid `-`/`_` characters).
fn validate_dump_id(id: &str) -> Result<(), Diagnostic> {
    if id.is_empty()
        || id.len() > 256 // equivalent to char count: the validated charset is ASCII-only ([A-Za-z0-9_-])
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        // Cap the displayed portion to avoid leaking a huge server-controlled string.
        let preview: String = id.chars().take(40).collect();
        let suffix = if id.chars().count() > 40 { "…" } else { "" };
        return Err(Diagnostic::ServerError(format!(
            "server returned an invalid dump id: '{preview}{suffix}'"
        )));
    }
    Ok(())
}

/// Build the URL for a single-project lookup by identifier.
///
/// Returns the URL string only — the conditional-bearer logic and the
/// `is_safe_shortcode` guard stay in their respective callers.
/// Used by both `resolve_project` and `describe_project`.
fn project_lookup_url(base: &str, project: &str) -> String {
    match classify(project) {
        ProjectIdent::Shortcode(code) => {
            format!("{base}/admin/projects/shortcode/{code}")
        }
        ProjectIdent::Shortname(name) => {
            format!("{base}/admin/projects/shortname/{name}")
        }
        ProjectIdent::Iri(iri) => {
            format!("{base}/admin/projects/iri/{}", enc(iri))
        }
    }
}

/// Check whether a project shortcode is a safe filename component.
///
/// DSP shortcodes are 4 hex digits (e.g. `0001`, `ABCD`). This function is
/// generous — it accepts any non-empty ASCII-alphanumeric string up to 32 chars
/// — so it admits the real shortcode space without being fragile. It MUST reject
/// any value containing `/`, `\`, `..`, or an absolute path prefix, because
/// `default_output_path` builds a file path from the shortcode via
/// `PathBuf::join`. A leading `/` or `..` component would escape the intended
/// directory.
fn is_safe_shortcode(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Strip an IRI/CURIE to its local name (after the last `#`, `/`, or `:`).
///
/// rsplit always yields at least one element (even on `""`), so `unwrap_or` is
/// a no-panic guard rather than a live fallback.
fn local_name(id: &str) -> &str {
    id.rsplit(['#', '/', ':']).next().unwrap_or(id)
}

/// Resolve a class `@id` into `(resource-type name, full IRI)`.
///
/// - `name` = the local part: the segment after the last `#`, `/`, or `:`.
/// - `iri`  = CURIE `prefix:local` expanded via `@context` (when `local` is not
///   `//…`, i.e. not a scheme separator, and `prefix` is a known context prefix);
///   otherwise the `@id` verbatim (covers full IRIs, unknown prefixes, no-colon).
///
/// Total — no `unwrap`/`panic`. One code path handles CURIE, full-IRI, and
/// degenerate input without a dedicated `contains("://")` branch (the fallback
/// naturally covers full IRIs).
fn expand_class_id(id: &str, prefixes: &HashMap<String, String>) -> (String, String) {
    let name = local_name(id).to_string();
    let iri = match id.split_once(':') {
        Some((prefix, local)) if !local.starts_with("//") => prefixes
            .get(prefix)
            .map(|ns| format!("{ns}{local}"))
            .unwrap_or_else(|| id.to_string()),
        _ => id.to_string(), // scheme://… , no colon, or unknown prefix
    };
    (name, iri)
}

/// Derive a data-model name from its DSP-API ontology IRI.
///
/// `http://…/ontology/0801/beol/v2` → `beol`. Robust to a trailing slash and a
/// missing `/v2` suffix. Empty input → empty name (server contract trusted; an
/// empty ontology IRI degrades silently to an empty name, treated as benign).
///
/// The `rsplit('/').next()` branch always yields `Some` — `rsplit` on `""` yields
/// one empty element — so the `unwrap_or` is structurally unreachable. It is kept
/// as a no-panic guard.
///
/// `pub(crate)` so `builtins.rs` can call it to assert name↔IRI consistency.
pub(crate) fn data_model_name_from_iri(iri: &str) -> String {
    let t = iri.trim_end_matches('/');
    let t = t.strip_suffix("/v2").unwrap_or(t);
    t.rsplit('/').next().unwrap_or(t).to_string()
}

// ---------------------------------------------------------------------------
// describe_resource_type helpers
// ---------------------------------------------------------------------------

/// System namespace prefixes (Decision 3). A field whose property CURIE prefix
/// is in this set is a built-in field (hidden by default; revealed with
/// `--include-builtins`). Project fields from a sibling data-model have a
/// project-specific prefix and are NOT in this set, so they show by default.
const SYSTEM_PREFIXES: &[&str] = &[
    "knora-api",
    "knora-base",
    "rdf",
    "rdfs",
    "owl",
    "salsah-gui",
    "standoff",
    "xsd",
];

/// `knora-api` file-value property local names that signal the representation kind.
///
/// Matched against `owl:onProperty @id` local names in restrictions. The first
/// match deterministically selects the representation (Decision 5, R8).
const FILE_VALUE_PROPS: &[(&str, Representation)] = &[
    ("hasStillImageFileValue", Representation::StillImage),
    ("hasMovingImageFileValue", Representation::MovingImage),
    ("hasAudioFileValue", Representation::Audio),
    ("hasDocumentFileValue", Representation::Document),
    ("hasArchiveFileValue", Representation::Archive),
    ("hasTextFileValue", Representation::Text),
];

/// Maximum number of distinct sibling ontologies to fetch (Decision 9 / R5).
/// Defends against a pathological or hostile `@context` with many prefixes.
const MAX_SIBLING_FETCHES: usize = 16;

/// Check whether a CURIE prefix belongs to a system (built-in) namespace.
///
/// Returns `true` iff `prefix` is in [`SYSTEM_PREFIXES`]. Used to decide
/// `is_builtin` for a field (Decision 3) and to skip system fields from the
/// sibling-fetch loop (R5).
fn is_system_prefix(prefix: &str) -> bool {
    SYSTEM_PREFIXES.contains(&prefix)
}

/// Map a DSP-API `objectType` local name to a [`ValueType`].
///
/// Named variants for all 16 known types; `Other(kebab)` for anything else
/// (e.g. `GeomValue`, `IntervalValue`, `TextFileValue`). The `Other` string is
/// derived by: strip trailing `Value` (if present), kebab-case by inserting `-`
/// only before an uppercase letter that follows a lowercase (so runs of
/// consecutive uppercase stay together), then lowercase the whole.
///
/// Examples: `TextValue`→`text`, `URIValue`→`uri`, `GeoNameValue`→`geo-name`,
/// `GeomValue`→`geom`. Display-only robustness.
fn map_object_type_to_value_type(local: &str) -> ValueType {
    match local {
        "TextValue" => ValueType::Text,
        "IntValue" => ValueType::Integer,
        "DecimalValue" => ValueType::Decimal,
        "BooleanValue" => ValueType::Boolean,
        "DateValue" => ValueType::Date,
        "TimeValue" => ValueType::Time,
        "UriValue" => ValueType::Uri,
        "ColorValue" => ValueType::Color,
        "GeonameValue" => ValueType::Geoname,
        "ListValue" => ValueType::VocabularyItem,
        "StillImageFileValue" => ValueType::StillImage,
        "MovingImageFileValue" => ValueType::MovingImage,
        "AudioFileValue" => ValueType::Audio,
        "DocumentFileValue" => ValueType::Document,
        "ArchiveFileValue" => ValueType::Archive,
        other => ValueType::Other(object_type_to_kebab(other)),
    }
}

/// Convert an objectType local name to a kebab-cased string for `Other`.
///
/// Algorithm: strip trailing `Value` suffix (if present), then kebab-case
/// by inserting `-` only before an uppercase letter that follows a lowercase
/// (consecutive-uppercase runs stay together — so `URIValue`→`uri`, not
/// `u-r-i`). Then lowercase the whole. See the plan's CamelCase splitter rule.
fn object_type_to_kebab(local: &str) -> String {
    // Strip trailing `Value` suffix if present.
    let base = local.strip_suffix("Value").unwrap_or(local);

    // Insert `-` before an uppercase letter that follows a lowercase letter.
    // Consecutive uppercase sequences (e.g. "URI") are NOT split.
    let mut result = String::with_capacity(base.len() + 4);
    let chars: Vec<char> = base.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() {
            // Insert dash only when the immediately preceding char is lowercase.
            if chars[i - 1].is_lowercase() {
                result.push('-');
            }
        }
        result.push(ch);
    }
    result.to_lowercase()
}

/// Decode an `owl:Restriction` element's cardinality fields into a [`Cardinality`].
///
/// Decision 1 / the decode table: DSP-API only ever emits `owl:cardinality`=1,
/// `owl:minCardinality`∈{0,1}, or `owl:maxCardinality`=1. Any other shape
/// (absent key, value>1) degrades to `ZeroOrMore` with a `tracing::warn!`.
fn decode_cardinality(restriction: &serde_json::Value) -> Cardinality {
    // Helper to read an integer from a serde_json::Value.
    let as_u64 =
        |key: &str| -> Option<u64> { restriction.get(key).and_then(serde_json::Value::as_u64) };

    if let Some(v) = as_u64("owl:cardinality") {
        if v == 1 {
            return Cardinality::One;
        }
        tracing::warn!(
            value = v,
            "owl:cardinality had unexpected value (expected 1); falling back to ZeroOrMore"
        );
        return Cardinality::ZeroOrMore;
    }

    if let Some(v) = as_u64("owl:maxCardinality") {
        if v == 1 {
            return Cardinality::ZeroOrOne;
        }
        tracing::warn!(
            value = v,
            "owl:maxCardinality had unexpected value (expected 1); falling back to ZeroOrMore"
        );
        return Cardinality::ZeroOrMore;
    }

    if let Some(v) = as_u64("owl:minCardinality") {
        return match v {
            0 => Cardinality::ZeroOrMore,
            1 => Cardinality::OneOrMore,
            other => {
                tracing::warn!(
                    value = other,
                    "owl:minCardinality had unexpected value (expected 0 or 1); falling back to ZeroOrMore"
                );
                Cardinality::ZeroOrMore
            }
        };
    }

    tracing::warn!("owl:Restriction has no recognized cardinality key; falling back to ZeroOrMore");
    Cardinality::ZeroOrMore
}

/// Detect the representation kind from the set of restriction `onProperty` local names.
///
/// Matches against [`FILE_VALUE_PROPS`] in order; returns the first hit.
/// Must be called BEFORE filtering built-in fields (the file-value props are
/// `knora-api:` prefixed → `is_builtin = true` → filtered in default mode).
fn detect_representation(restriction_prop_locals: &[&str]) -> Option<Representation> {
    for local in restriction_prop_locals {
        for (file_val_local, repr) in FILE_VALUE_PROPS {
            if local == file_val_local {
                return Some(*repr);
            }
        }
    }
    None
}

/// Extract the CURIE prefix from an `@id` string (the part before the first `:`
/// that is not followed by `//`). Returns `None` for full IRIs or bare names.
fn curie_prefix(id: &str) -> Option<&str> {
    id.split_once(':')
        .filter(|(_, local)| !local.starts_with("//"))
        .map(|(prefix, _)| prefix)
}

// ---------------------------------------------------------------------------
// Resource list DTOs (boundary translation — ADR-0001)
// ---------------------------------------------------------------------------

/// Top-level DTO for `GET /v2/resources` responses.
///
/// The endpoint returns three structural forms of JSON-LD:
///
/// - **Many results**: `{ "@graph": [ { "@id": "…", "@type": "…", … }, … ], "knora-api:mayHaveMoreResults": … }`
/// - **Single result**: `{ "@id": "…", "@type": "…", … }` — no `@graph`, but `@id` IS present
/// - **Empty result**: `{}` — no `@graph`, no `@id`
///
/// The distinction between single and empty is carried by `@id` presence (rev: D3 R3).
/// `graph` handles the many case; the single-node fields (`id`, `type_field`, etc.)
/// handle the one case; absence of both signals empty.
#[derive(serde::Deserialize)]
struct ResourceListDto {
    /// Present for the "many" case: an array of resource nodes.
    #[serde(rename = "@graph", default)]
    graph: Option<Vec<ResourceNodeDto>>,

    /// Present for the "single" case (and absent for empty/many).
    #[serde(rename = "@id", default)]
    id: Option<String>,

    /// `@type` for the single-node case. May be a bare string IRI or an array;
    /// typed as `Value` to match the `node_dto_to_summary` signature uniformly.
    #[serde(rename = "@type", default)]
    type_field: Option<serde_json::Value>,

    /// `rdfs:label` for the single-node case.
    #[serde(rename = "rdfs:label", default)]
    label: Option<serde_json::Value>,

    /// `knora-api:arkUrl` for the single-node case.
    #[serde(rename = "knora-api:arkUrl", default)]
    ark_url: Option<serde_json::Value>,

    /// `knora-api:creationDate` for the single-node case.
    #[serde(rename = "knora-api:creationDate", default)]
    creation_date: Option<serde_json::Value>,

    /// `knora-api:lastModificationDate` for the single-node case.
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<serde_json::Value>,

    /// `knora-api:mayHaveMoreResults` top-level boolean (default false per D5).
    #[serde(rename = "knora-api:mayHaveMoreResults", default)]
    may_have_more_results: bool,
}

/// One node from a resource-list `@graph` array.
///
/// Only the envelope fields the list projection needs are modelled; `serde`
/// ignores the rich value content (ADR-0001 — no DSP-API vocab above the
/// client boundary). All fields except `id` default, so a node without a
/// type or label degrades gracefully.
#[derive(serde::Deserialize)]
struct ResourceNodeDto {
    #[serde(rename = "@id")]
    id: String,

    /// `@type` is an array on the wire; we take the first element.
    #[serde(rename = "@type", default)]
    type_field: Option<serde_json::Value>,

    /// `rdfs:label` can be a string or a language-tagged object.
    #[serde(rename = "rdfs:label", default)]
    label: Option<serde_json::Value>,

    /// `knora-api:arkUrl` — can be a string or an object with `@value`.
    #[serde(rename = "knora-api:arkUrl", default)]
    ark_url: Option<serde_json::Value>,

    /// `knora-api:creationDate` — can be a string or an object with `@value`.
    #[serde(rename = "knora-api:creationDate", default)]
    creation_date: Option<serde_json::Value>,

    /// `knora-api:lastModificationDate` — same shape as `knora-api:creationDate`.
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<serde_json::Value>,
}

/// Extract a plain string from a JSON-LD value that may be a bare string,
/// a language-tagged `{"@value":"…"}` object, or an `{"@id":"…"}` object.
///
/// Returns `None` on anything that cannot be mapped to a string.
fn extract_string_value(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(map) => map
            .get("@value")
            .or_else(|| map.get("@id"))
            .and_then(|inner| inner.as_str())
            .map(str::to_owned),
        _ => None,
    }
}

/// Extract the resource-type name from a `@type` field value.
///
/// `@type` may be:
/// - a bare string IRI/CURIE → take the local name
/// - a JSON array → take the first element's local name
/// - absent → sentinel `"unknown"` (fallback, not an error)
fn extract_resource_type(type_val: Option<&serde_json::Value>) -> String {
    match type_val {
        None => "unknown".to_string(),
        Some(serde_json::Value::String(s)) => local_name(s).to_string(),
        Some(serde_json::Value::Array(arr)) => arr
            .first()
            .and_then(|v| v.as_str())
            .map(|s| local_name(s).to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

/// Translate a `ResourceNodeDto` into a `ResourceSummary`.
fn node_dto_to_summary(
    id: String,
    type_val: Option<&serde_json::Value>,
    label_val: Option<&serde_json::Value>,
    ark_val: Option<&serde_json::Value>,
    creation_val: Option<&serde_json::Value>,
    last_modification_val: Option<&serde_json::Value>,
) -> ResourceSummary {
    let label = label_val.and_then(extract_string_value).unwrap_or_default();
    let resource_type = extract_resource_type(type_val);
    let ark_url = ark_val.and_then(extract_string_value);
    // NOTE (D4, verified live on `dev` 2026-06-17): the command now uses
    // `schema=complex`, which carries both `knora-api:creationDate` and
    // `knora-api:lastModificationDate` (live-verified against incunabula on
    // `dev` 2026-06-17 — `creation_date` now populates). Both are still
    // `Option` because `lastModificationDate` is server-side optional (a
    // resource that has never been modified has none). The simple-vs-complex
    // user-facing terminology is ADR-0013 (Phase 8c) scope.
    let creation_date = creation_val.and_then(extract_string_value);
    let last_modified = last_modification_val.and_then(extract_string_value);
    ResourceSummary {
        label,
        iri: id,
        ark_url,
        creation_date,
        last_modified,
        resource_type,
    }
}

/// Wire DTO for a single-resource `GET /v2/resources/{iri}?schema=complex` response.
///
/// Named envelope fields are parsed directly; the `@context` is captured for CURIE
/// expansion; and the `extra` catch-all captures all remaining keys (field values)
/// for the `with_values` path (ADR-0001 boundary translation).
///
/// Wire-key to domain-field mapping (boundary translation — ADR-0001):
/// - `@id` → `iri`
/// - `@type` → `resource_type` (via `extract_resource_type`)
/// - `rdfs:label` → `label` (via `extract_string_value`)
/// - `knora-api:arkUrl` → `ark_url`
/// - `knora-api:creationDate` → `creation_date`
/// - `knora-api:lastModificationDate` → `last_modified`
/// - `knora-api:attachedToProject` → `attached_project`
/// - `knora-api:attachedToUser` → `owner`
/// - `knora-api:hasPermissions` → raw ACL string → `derive_visibility`
/// - `knora-api:userHasPermission` → raw permission code → `derive_access`
/// - `@context` → prefix map for CURIE expansion (with_values path)
/// - all other keys (field values) → `extra` (with_values path)
#[derive(serde::Deserialize)]
struct ResourceDetailDto {
    #[serde(rename = "@id")]
    id: String,

    /// `@type` is an array on the wire (same as `ResourceNodeDto.type_field`).
    #[serde(rename = "@type", default)]
    type_field: Option<serde_json::Value>,

    /// `rdfs:label` can be a string or a language-tagged object.
    #[serde(rename = "rdfs:label", default)]
    label: Option<serde_json::Value>,

    /// `knora-api:arkUrl` — string or `{"@value": "…"}` object.
    #[serde(rename = "knora-api:arkUrl", default)]
    ark_url: Option<serde_json::Value>,

    /// `knora-api:creationDate` — string or `{"@value": "…"}` object.
    #[serde(rename = "knora-api:creationDate", default)]
    creation_date: Option<serde_json::Value>,

    /// `knora-api:lastModificationDate` — same shape as `creationDate`.
    #[serde(rename = "knora-api:lastModificationDate", default)]
    last_modification_date: Option<serde_json::Value>,

    /// `knora-api:attachedToProject` — IRI of the project; `{"@id": "…"}` on the wire.
    #[serde(rename = "knora-api:attachedToProject", default)]
    attached_to_project: Option<serde_json::Value>,

    /// `knora-api:attachedToUser` — IRI of the user; `{"@id": "…"}` on the wire.
    #[serde(rename = "knora-api:attachedToUser", default)]
    attached_to_user: Option<serde_json::Value>,

    /// `knora-api:hasPermissions` — bare JSON string on the wire (the full ACL).
    /// Not a value object; typed directly as `Option<String>`.
    #[serde(rename = "knora-api:hasPermissions", default)]
    has_permissions: Option<String>,

    /// `knora-api:userHasPermission` — bare JSON string on the wire (the caller's
    /// effective permission code). Not a value object; typed directly as `Option<String>`.
    #[serde(rename = "knora-api:userHasPermission", default)]
    user_has_permission: Option<String>,

    /// JSON-LD `@context` — captured as a `Value` so we can extract the string-valued
    /// prefix→namespace entries for CURIE expansion (with_values path).
    ///
    /// **Must be a named field** — NOT in `extra`. If it fell into the flat catch-all
    /// we'd lose the context prefix map and CURIE expansion would silently fail, leaving
    /// all field labels unresolved.
    #[serde(rename = "@context", default)]
    context: Option<serde_json::Value>,

    /// Catch-all for every key not captured above — primarily field-value entries
    /// (e.g. `incunabula:hasPagenum`, `knora-api:hasStillImageFileValue`) plus any
    /// other server keys we don't model explicitly. Preserved in insertion order by
    /// the `preserve_order` serde_json feature (deterministic field order = server order).
    ///
    /// Named `extra` so the intent is clear at every use-site; the `#[serde(flatten)]`
    /// means it absorbs ALL remaining keys after the above fields are matched.
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

/// Map a DSP permission code to a numeric rank.
///
/// Order is per `Permission.scala` in dsp-api:
/// `RV`(1) < `V`(2) < `M`(6) < `D`(7) < `CR`(8).
/// Unknown codes rank 0 — treated as below `RV`, i.e. no grant.
fn permission_rank(code: &str) -> u8 {
    match code {
        "RV" => 1,
        "V" => 2,
        "M" => 6,
        "D" => 7,
        "CR" => 8,
        _ => 0,
    }
}

/// Derive the caller's access level from the `userHasPermission` code.
///
/// The translation table (D1, Facet B):
/// - `RV` → `RestrictedView`
/// - `V` → `View`
/// - `M` → `Edit`
/// - `D` → `Delete`
/// - `CR` → `Manage`
/// - absent / unknown → `None`
fn derive_access(user_has_permission: &str) -> Option<ResourceAccess> {
    match user_has_permission {
        "RV" => Some(ResourceAccess::RestrictedView),
        "V" => Some(ResourceAccess::View),
        "M" => Some(ResourceAccess::Edit),
        "D" => Some(ResourceAccess::Delete),
        "CR" => Some(ResourceAccess::Manage),
        _ => None,
    }
}

/// Derive the resource's visibility from the `hasPermissions` ACL string.
///
/// Implements the D1 ACL parse algorithm (see implementation plan):
/// 1. Split on `'|'` into entries; for each, `split_once(' ')` → `(code, group_list)`.
/// 2. Split `group_list` on `','`; take each group's local name and match **exactly**
///    against `"UnknownUser"` / `"KnownUser"` — never `.contains()`.
/// 3. Track the max `permission_rank(code)` seen for each world group across all entries.
/// 4. Apply the D1 visibility table.
/// 5. Empty or whitespace-only ACL → `None`.
fn derive_visibility(has_permissions: &str) -> Option<ResourceVisibility> {
    if has_permissions.trim().is_empty() {
        return None;
    }

    let mut unknown_rank: u8 = 0;
    let mut known_rank: u8 = 0;
    let mut parsed_any = false;

    for entry in has_permissions.split('|') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Each entry is "<CODE> <group>[,<group>…]"
        let Some((code, group_list)) = entry.split_once(' ') else {
            // Malformed entry — skip.
            continue;
        };
        parsed_any = true;
        let rank = permission_rank(code);
        for group in group_list.split(',') {
            let group_local = local_name(group.trim());
            if group_local == "UnknownUser" {
                unknown_rank = unknown_rank.max(rank);
            } else if group_local == "KnownUser" {
                known_rank = known_rank.max(rank);
            }
        }
    }

    if !parsed_any {
        return None;
    }

    // D1 table: UnknownUser decides first.
    // Hoist rank constants once to avoid repeated calls and make the ladder
    // stable against future changes to the permission_rank table.
    let v_rank = permission_rank("V");
    let rv_rank = permission_rank("RV");

    if unknown_rank >= v_rank {
        Some(ResourceVisibility::Public)
    } else if unknown_rank >= rv_rank {
        // At this point unknown_rank < v_rank, so >= rv_rank means exactly RV.
        Some(ResourceVisibility::PublicRestricted)
    } else if known_rank >= rv_rank {
        Some(ResourceVisibility::LoggedInUsers)
    } else {
        Some(ResourceVisibility::ProjectMembers)
    }
}

// ---------------------------------------------------------------------------
// HttpDspClient
// ---------------------------------------------------------------------------

/// Real HTTP implementation of `DspClient`, backed by `reqwest::blocking`.
pub struct HttpDspClient {
    /// Default HTTP client: 10 s connect timeout + 30 s overall timeout.
    /// Used by login, resolve_project, create/get/delete dump methods.
    client: reqwest::blocking::Client,
    /// Download-specific HTTP client: 30 s connect timeout, **no overall timeout**.
    /// A bagit-zip archive can be very large; an overall read timeout would kill the
    /// download mid-stream. The connect timeout is retained so a hung server is
    /// still detected at connection time.
    download_client: reqwest::blocking::Client,
}

impl HttpDspClient {
    /// Construct a new client pair with appropriate timeouts.
    ///
    /// - `client`: 10 s connect + 30 s overall (used for all short-lived requests).
    /// - `download_client`: 30 s connect, **no overall timeout** (used only for
    ///   streaming dump archives — they can be large).
    ///
    /// Returns `Err(Diagnostic::Internal(...))` if either reqwest client cannot be
    /// built (rare — only triggered by TLS backend misconfiguration).
    pub fn new() -> Result<Self, Diagnostic> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(crate::util::USER_AGENT)
            .build()
            .map_err(|e| Diagnostic::Internal(format!("failed to build HTTP client: {e}")))?;
        let download_client = reqwest::blocking::Client::builder()
            .connect_timeout(Some(Duration::from_secs(30)))
            .timeout(None)
            .user_agent(crate::util::USER_AGENT)
            .build()
            .map_err(|e| {
                Diagnostic::Internal(format!("failed to build download HTTP client: {e}"))
            })?;
        // No third client built here: `sparql_query`'s client (D17) is built
        // per call, not once here — see its doc comment for why.
        Ok(Self {
            client,
            download_client,
        })
    }

    /// Fetch `GET /v2/ontologies/allentities/{enc(ontology_iri)}` and deserialize.
    ///
    /// Shared by `describe_data_model` and `describe_resource_type` (which needs
    /// both the primary fetch and sibling fetches). Auth is optional; `token` is
    /// forwarded as a bearer when `Some`. NEVER log the token.
    ///
    /// SSRF note (R10): callers must only pass an `ontology_iri` derived from the
    /// user-supplied `--data-model` argument or from the queried ontology's own
    /// `@context`. The host is always the user-supplied `server`.
    fn fetch_allentities(
        &self,
        server: &str,
        ontology_iri: &str,
        token: Option<&str>,
    ) -> Result<OntologyAllEntitiesResponse, Diagnostic> {
        let url = format!(
            "{}/v2/ontologies/allentities/{}",
            server.trim_end_matches('/'),
            enc(ontology_iri)
        );

        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if status.is_success() {
            let resp: OntologyAllEntitiesResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("data-model response could not be parsed: {e}"))
            })?;
            Ok(resp)
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    /// Fetch and parse `GET /admin/lists/{enc(iri)}`.
    ///
    /// Shared by `describe_vocabulary` for both the initial (possibly-node)
    /// address and the second, upward-resolved root fetch (D2). Auth is
    /// optional; `token` is forwarded as a bearer when `Some`. NEVER log the
    /// token.
    fn fetch_list_get(
        &self,
        server: &str,
        iri: &str,
        token: Option<&str>,
    ) -> Result<ListGetResponseDto, Diagnostic> {
        let url = format!("{}/admin/lists/{}", server.trim_end_matches('/'), enc(iri));

        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if status.is_success() {
            response.json::<ListGetResponseDto>().map_err(|e| {
                Diagnostic::ServerError(format!("vocabulary response could not be parsed: {e}"))
            })
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }
}

impl HttpDspClient {
    /// Parse field values from a complex-schema resource response.
    ///
    /// Called from `describe_resource` when `with_values == true`. Iterates the
    /// `extra` map, identifies field entries (those with a `knora-api:*Value`
    /// `@type`), parses each value object into a `ValueContent`, resolves field
    /// labels via project-ontology allentities fetches (deduped), and resolves
    /// list-node labels via `/v2/node` fetches (deduped).
    ///
    /// All label fetch failures degrade gracefully (local name / node IRI fallback)
    /// — they NEVER return `Err`. This is intentional: a label failure must not
    /// abort the describe.
    fn parse_resource_values(
        &self,
        server: &str,
        token: Option<&str>,
        context_val: &Option<serde_json::Value>,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<FieldValues> {
        // ── 1. Build prefix → namespace map from @context ────────────────────────
        let prefixes: HashMap<String, String> = build_prefix_map(context_val);

        // ── 2. Iterate extra, identify field entries ─────────────────────────────
        // Denylist: these keys carry value-class-typed objects but are NOT user fields.
        const DENYLIST: &[&str] = &[
            "knora-api:hasIncomingLinkValue",
            "knora-api:hasStandoffLinkToValue",
            "knora-api:hasStandoffLinkValue", // non-`To` standoff variant (some ontologies)
        ];

        // Collect (key, Vec<value_obj>) for each field.  An entry may be a single
        // value object or an array of value objects.
        let mut field_entries: Vec<(&str, Vec<&serde_json::Value>)> = Vec::new();

        for (key, val) in extra.iter() {
            if DENYLIST.contains(&key.as_str()) {
                continue;
            }

            // Gather the value object(s) for this key.
            let objs: Vec<&serde_json::Value> = match val {
                serde_json::Value::Array(arr) => arr.iter().collect(),
                obj @ serde_json::Value::Object(_) => vec![obj],
                _ => continue, // scalar — not a value field
            };

            if objs.is_empty() {
                continue;
            }

            // A key is a field iff every non-null value object has a knora-api *Value @type.
            // We check only the first one for efficiency (homogeneous arrays).
            let first = match objs.first() {
                Some(v) => v,
                None => continue,
            };
            if !has_value_class_type(first) {
                continue;
            }

            field_entries.push((key.as_str(), objs));
        }

        // ── 3. Parse each value object into ValueContent ─────────────────────────
        // We also record which field keys are link-typed for name derivation (D3).
        struct ParsedField<'a> {
            key: &'a str,
            is_link: bool,
            values: Vec<Value>,
        }

        let mut parsed_fields: Vec<ParsedField> = Vec::new();

        for (key, objs) in &field_entries {
            let mut contents: Vec<Value> = Vec::new();
            let mut any_link = false;

            for obj in objs {
                // Skip DeletedValue objects.
                if get_type_local(obj) == "DeletedValue" {
                    continue;
                }
                let (content, is_link) = parse_value(obj);
                if is_link {
                    any_link = true;
                }
                contents.push(content);
            }

            if contents.is_empty() {
                continue;
            }

            parsed_fields.push(ParsedField {
                key,
                is_link: any_link,
                values: contents,
            });
        }

        // ── 4. Resolve field labels (project ontologies only, deduped) ───────────
        // Collect distinct project ontology IRIs for fields that need labels.
        // knora-api built-ins skip the fetch → label = None.
        let mut ontology_labels: HashMap<String, HashMap<String, String>> = HashMap::new(); // ont_iri → (prop_iri → label)
        let mut fetched_ontologies: HashSet<String> = HashSet::new();

        for pf in &parsed_fields {
            let prefix = curie_prefix(pf.key).unwrap_or("");
            if is_system_prefix(prefix) || prefix.is_empty() {
                continue; // built-in or unknown prefix → skip fetch
            }
            // Expand the CURIE to an ontology IRI (namespace without fragment).
            let namespace = match prefixes.get(prefix) {
                Some(ns) => ns,
                None => continue,
            };
            let ont_iri = namespace.trim_end_matches(['#', '/']).to_string();
            if fetched_ontologies.insert(ont_iri.clone()) {
                // SSRF-safe: the host is always the user-supplied `server`; the ontology
                // IRI is an enc()-encoded path segment (NON_ALPHANUMERIC) and cannot
                // escape the segment or alter the host.
                match self.fetch_allentities(server, &ont_iri, token) {
                    Ok(resp) => {
                        let mut prop_map: HashMap<String, String> = HashMap::new();
                        let ctx_prefixes: HashMap<String, String> = resp
                            .context
                            .iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect();
                        for entity in resp.graph {
                            if let Some(lbl) = entity.label {
                                let (_, iri) = expand_class_id(&entity.id, &ctx_prefixes);
                                prop_map.insert(iri, lbl);
                            }
                        }
                        ontology_labels.insert(ont_iri, prop_map);
                    }
                    Err(e) => {
                        // Non-fatal: warn and continue; affected fields degrade to local name.
                        tracing::warn!(
                            prefix = %prefix,
                            error = %e,
                            "field-label ontology fetch failed; using local name as fallback"
                        );
                    }
                }
            }
        }

        // ── 5. Resolve list-node labels (deduped) ────────────────────────────────
        let mut node_labels: HashMap<String, Option<String>> = HashMap::new();

        // Collect distinct node IRIs.
        for pf in &parsed_fields {
            for v in &pf.values {
                if let ValueContent::VocabularyItem { node_iri, .. } = &v.content {
                    node_labels.entry(node_iri.clone()).or_insert(None);
                }
            }
        }

        // Fetch each node once.
        for (node_iri, label_slot) in node_labels.iter_mut() {
            // SSRF-safe: the host is always the user-supplied `server`; the node IRI
            // is an enc()-encoded path segment (NON_ALPHANUMERIC) and cannot escape
            // the segment or alter the host.
            let url = format!("{}/v2/node/{}", server.trim_end_matches('/'), enc(node_iri));
            let req = self.client.get(&url);
            let req = if let Some(t) = token {
                req.bearer_auth(t)
            } else {
                req
            };
            match req.send() {
                Ok(resp) if resp.status().is_success() => {
                    // Degrade to None on parse failure (consistent with sibling tracing arms).
                    match resp.json::<serde_json::Value>() {
                        Ok(body) => {
                            // `rdfs:label` may be a bare string or a language-tagged object.
                            let lbl = body.get("rdfs:label").and_then(extract_string_value);
                            *label_slot = lbl;
                        }
                        Err(_) => {
                            tracing::debug!(
                                node_iri = %node_iri,
                                "list-node label response could not be parsed as JSON; using node IRI as fallback"
                            );
                        }
                    }
                }
                Ok(resp) => {
                    // Non-2xx: degrade to node IRI.
                    tracing::debug!(
                        node_iri = %node_iri,
                        status = %resp.status(),
                        "list-node label fetch returned non-success; using node IRI as fallback"
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        node_iri = %node_iri,
                        error = %e,
                        "list-node label fetch failed; using node IRI as fallback"
                    );
                }
            }
        }

        // ── 6. Build Vec<FieldValues>, fold labels, preserve server order ─────────
        let mut result: Vec<FieldValues> = Vec::new();

        for pf in parsed_fields {
            // Derive field name (D3): strip `Value` suffix on link-typed fields only.
            let raw_name = local_name(pf.key).to_string();
            let name = if pf.is_link {
                raw_name
                    .strip_suffix("Value")
                    .unwrap_or(&raw_name)
                    .to_string()
            } else {
                raw_name
            };

            // Resolve field label from ontology fetch.
            let label: Option<String> = {
                let prefix = curie_prefix(pf.key).unwrap_or("");
                if is_system_prefix(prefix) || prefix.is_empty() {
                    None
                } else if let Some(ns) = prefixes.get(prefix) {
                    let ont_iri = ns.trim_end_matches(['#', '/']).to_string();
                    let local = local_name(pf.key);
                    let prop_iri = format!("{}{}", ns, local);
                    ontology_labels
                        .get(&ont_iri)
                        .and_then(|m| m.get(&prop_iri).cloned())
                } else {
                    None
                }
            };

            // Fold list-node labels into the VocabularyItem values.
            let values: Vec<Value> = pf
                .values
                .into_iter()
                .map(|v| match v.content {
                    ValueContent::VocabularyItem { node_iri, label: _ } => {
                        let resolved = node_labels.get(&node_iri).cloned().flatten();
                        Value {
                            content: ValueContent::VocabularyItem {
                                node_iri,
                                label: resolved,
                            },
                            comment: v.comment,
                        }
                    }
                    other => Value {
                        content: other,
                        comment: v.comment,
                    },
                })
                .collect();

            result.push(FieldValues {
                name,
                label,
                values,
            });
        }

        result
    }
}

// ---------------------------------------------------------------------------
// Value parsing helpers (pure — no HTTP; tested directly in unit tests)
// ---------------------------------------------------------------------------

/// Return true iff `val` is a JSON object whose `@type` is a `knora-api:*Value`
/// (i.e. its local name ends with `Value` and the prefix is `knora-api`).
/// This is the key discriminant for "is this a value field?" (ADR-0013).
fn has_value_class_type(val: &serde_json::Value) -> bool {
    let type_local = get_type_local(val);
    // Must end with "Value" and not be a bare non-CURIE literal.
    // Additionally, the @type must come from the knora-api namespace.
    type_local.ends_with("Value") && !type_local.is_empty() && {
        // Verify the @type is actually `knora-api:*Value`, not e.g. `xsd:anyURI`.
        let raw_type = val
            .as_object()
            .and_then(|m| m.get("@type"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        raw_type.starts_with("knora-api:")
    }
}

/// Extract the local name of a value object's `@type`.
///
/// Returns `""` if absent or not a string.
fn get_type_local(val: &serde_json::Value) -> &str {
    val.as_object()
        .and_then(|m| m.get("@type"))
        .and_then(|t| t.as_str())
        .map(local_name)
        .unwrap_or("")
}

/// Build a `HashMap<String, String>` prefix→namespace map from a JSON-LD `@context` Value.
///
/// Only string-valued entries are included (object-valued term definitions are
/// skipped, mirroring the pattern in `describe_data_model`). A missing or
/// non-object context yields an empty map (graceful degradation).
fn build_prefix_map(context_val: &Option<serde_json::Value>) -> HashMap<String, String> {
    match context_val {
        Some(serde_json::Value::Object(map)) => map
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        _ => HashMap::new(),
    }
}

/// Parse a single value object into `(ValueContent, is_link_type)`.
///
/// Pure function — no HTTP, no `self`. This is the content-only parser (no
/// per-value comment); [`parse_value`] wraps it to additionally produce a
/// [`Value`]. All parse failures degrade to `Raw`. The `is_link_type` flag is
/// used by the caller for field-name derivation (D3).
fn parse_value_content(obj: &serde_json::Value) -> (ValueContent, bool) {
    let type_local = get_type_local(obj);

    match type_local {
        // ── TextValue ────────────────────────────────────────────────────────────
        "TextValue" => {
            // Presence-based detection (Risk 7 / ADR-0013): if textValueAsXml present
            // → formatted (standoff); else valueAsString.
            let content =
                if let Some(xml) = obj.get("knora-api:textValueAsXml").and_then(|v| v.as_str()) {
                    crate::util::text::html_to_text(xml)
                } else {
                    obj.get("knora-api:valueAsString")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                };
            (ValueContent::Text(content), false)
        }

        // ── IntValue ─────────────────────────────────────────────────────────────
        "IntValue" => {
            let n = obj
                .get("knora-api:intValueAsInt")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (ValueContent::Integer(n), false)
        }

        // ── DecimalValue ─────────────────────────────────────────────────────────
        "DecimalValue" => {
            // `decimalValueAsDecimal` is a typed literal: `{"@value": "3.14", "@type": "xsd:decimal"}`.
            let s = obj
                .get("knora-api:decimalValueAsDecimal")
                .and_then(|v| {
                    // May be a bare string or a {"@value":…} object.
                    if let Some(s) = v.as_str() {
                        Some(s.to_string())
                    } else {
                        v.get("@value").and_then(|i| i.as_str()).map(str::to_owned)
                    }
                })
                .unwrap_or_default();
            (ValueContent::Decimal(s), false)
        }

        // ── BooleanValue ─────────────────────────────────────────────────────────
        "BooleanValue" => {
            let b = obj
                .get("knora-api:booleanValueAsBoolean")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            (ValueContent::Boolean(b), false)
        }

        // ── DateValue ────────────────────────────────────────────────────────────
        "DateValue" => {
            let calendar = obj
                .get("knora-api:dateValueHasCalendar")
                .and_then(|v| v.as_str())
                .unwrap_or("GREGORIAN")
                .to_string();

            let parse_point = |prefix: &str| -> DatePoint {
                let year_key = format!("knora-api:{prefix}Year");
                let month_key = format!("knora-api:{prefix}Month");
                let day_key = format!("knora-api:{prefix}Day");
                let era_key = format!("knora-api:{prefix}Era");

                DatePoint {
                    year: obj
                        .get(year_key.as_str())
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    month: obj
                        .get(month_key.as_str())
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32),
                    day: obj
                        .get(day_key.as_str())
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32),
                    era: obj
                        .get(era_key.as_str())
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                }
            };

            // Check for all required fields: if year is missing on both points, fall
            // back to Raw rather than produce a meaningless date.
            let start = parse_point("dateValueHasStart");
            let end = parse_point("dateValueHasEnd");

            if start.year.is_none() && end.year.is_none() {
                // Degenerate date with no year info — use raw fallback.
                let raw_text = obj
                    .get("knora-api:valueAsString")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                return (
                    ValueContent::Raw {
                        value_type: "date".to_string(),
                        text: raw_text,
                    },
                    false,
                );
            }

            (
                ValueContent::Date(DateValue {
                    calendar,
                    start,
                    end,
                }),
                false,
            )
        }

        // ── TimeValue ────────────────────────────────────────────────────────────
        "TimeValue" => {
            let s = obj
                .get("knora-api:timeValueAsTimeStamp")
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        Some(s.to_string())
                    } else {
                        v.get("@value").and_then(|i| i.as_str()).map(str::to_owned)
                    }
                })
                .unwrap_or_default();
            (ValueContent::Time(s), false)
        }

        // ── UriValue ─────────────────────────────────────────────────────────────
        "UriValue" => {
            let s = obj
                .get("knora-api:uriValueAsUri")
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        Some(s.to_string())
                    } else {
                        v.get("@value").and_then(|i| i.as_str()).map(str::to_owned)
                    }
                })
                .unwrap_or_default();
            (ValueContent::Uri(s), false)
        }

        // ── ColorValue ───────────────────────────────────────────────────────────
        "ColorValue" => {
            let s = obj
                .get("knora-api:colorValueAsColor")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (ValueContent::Color(s), false)
        }

        // ── GeonameValue ─────────────────────────────────────────────────────────
        "GeonameValue" => {
            let s = obj
                .get("knora-api:geonameValueAsGeonameCode")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (ValueContent::Geoname(s), false)
        }

        // ── ListValue ────────────────────────────────────────────────────────────
        "ListValue" => {
            // `listValueAsListNode` → `{"@id": "…"}`.
            let node_iri = obj
                .get("knora-api:listValueAsListNode")
                .and_then(|v| v.get("@id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (
                ValueContent::VocabularyItem {
                    node_iri,
                    label: None, // resolved later by the caller
                },
                false,
            )
        }

        // ── LinkValue ────────────────────────────────────────────────────────────
        "LinkValue" => {
            // Prefer embedded `linkValueHasTarget` (complex schema). Fall back to
            // `linkValueHasTargetIri.@id` when only the IRI is available.
            let (target_iri, target_label) =
                if let Some(target_obj) = obj.get("knora-api:linkValueHasTarget") {
                    let iri = target_obj
                        .get("@id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let lbl = target_obj.get("rdfs:label").and_then(extract_string_value);
                    (iri, lbl)
                } else {
                    let iri = obj
                        .get("knora-api:linkValueHasTargetIri")
                        .and_then(|v| v.get("@id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    (iri, None)
                };
            (
                ValueContent::Link {
                    target_iri,
                    target_label,
                },
                true, // this IS a link
            )
        }

        // ── File values ──────────────────────────────────────────────────────────
        // Match the whole *FileValue family by leading kind (ADR-0013).
        t if t.ends_with("FileValue") => {
            let filename = obj
                .get("knora-api:fileValueHasFilename")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url_str = obj
                .get("knora-api:fileValueAsUrl")
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        Some(s.to_string())
                    } else {
                        v.get("@value").and_then(|i| i.as_str()).map(str::to_owned)
                    }
                })
                .unwrap_or_default();

            // Map leading kind to ValueType.
            let value_type_opt = if t.starts_with("StillImage") {
                Some(ValueType::StillImage)
            } else if t.starts_with("MovingImage") {
                Some(ValueType::MovingImage)
            } else if t.starts_with("Audio") {
                Some(ValueType::Audio)
            } else if t.starts_with("Document") || t.starts_with("Text") {
                // TextFileValue → document (ADR-0013)
                Some(ValueType::Document)
            } else if t.starts_with("Archive") {
                Some(ValueType::Archive)
            } else {
                None // unrecognised *FileValue → raw
            };

            match value_type_opt {
                Some(vt) => {
                    // Still-image: additionally read dimensions.
                    let (width, height) = if vt == ValueType::StillImage {
                        let w = obj
                            .get("knora-api:stillImageFileValueHasDimX")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                        let h = obj
                            .get("knora-api:stillImageFileValueHasDimY")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                        (w, h)
                    } else {
                        (None, None)
                    };
                    (
                        ValueContent::File(FileValue {
                            value_type: vt,
                            filename,
                            url: url_str,
                            width,
                            height,
                        }),
                        false,
                    )
                }
                None => {
                    // Unrecognised *FileValue → raw fallback.
                    let raw_text = obj
                        .get("knora-api:valueAsString")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&filename)
                        .to_string();
                    (
                        ValueContent::Raw {
                            value_type: object_type_to_kebab(t),
                            text: raw_text,
                        },
                        false,
                    )
                }
            }
        }

        // ── Long-tail: any other *Value (IntervalValue, GeomValue, …) ────────────
        other => {
            let value_type = object_type_to_kebab(other);
            // Best-effort text: valueAsString if present; else compact JSON of the
            // value object minus standard metadata keys.
            let raw_text = obj
                .get("knora-api:valueAsString")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| compact_value_text(obj));
            (
                ValueContent::Raw {
                    value_type,
                    text: raw_text,
                },
                false,
            )
        }
    }
}

/// Parse a single value object into `(Value, is_link_type)`.
///
/// Wraps [`parse_value_content`] and additionally reads the optional
/// per-value comment from the sibling `knora-api:valueHasComment` key.
/// Pure function — no HTTP, no `self`.
fn parse_value(obj: &serde_json::Value) -> (Value, bool) {
    let (content, is_link) = parse_value_content(obj);
    let comment = obj
        .get("knora-api:valueHasComment")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned);
    (Value { content, comment }, is_link)
}

/// Standard value-object metadata keys to omit when building the raw fallback text.
const VALUE_META_KEYS: &[&str] = &[
    "@id",
    "@type",
    "knora-api:attachedToUser",
    "knora-api:hasPermissions",
    "knora-api:userHasPermission",
    "knora-api:valueCreationDate",
    "knora-api:valueHasComment",
    "knora-api:isDeleted",
    "knora-api:arkUrl",
    "knora-api:versionArkUrl",
    "knora-api:valueHasUUID",
];

/// Build a compact JSON representation of a value object for the `Raw` fallback.
///
/// Strips standard metadata keys and returns the compact JSON of what remains.
/// If nothing remains (all fields were metadata), returns an empty string.
fn compact_value_text(obj: &serde_json::Value) -> String {
    if let Some(map) = obj.as_object() {
        let filtered: serde_json::Map<String, serde_json::Value> = map
            .iter()
            .filter(|(k, _)| !VALUE_META_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if filtered.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&serde_json::Value::Object(filtered)).unwrap_or_default()
        }
    } else {
        String::new()
    }
}

impl DspClient for HttpDspClient {
    fn login(&self, server: &str, user: &str, password: &str) -> Result<LoginResponse, Diagnostic> {
        let url = format!("{}/v2/authentication", server.trim_end_matches('/'));

        let mut body = serde_json::Map::with_capacity(2);
        body.insert(
            identifier_key(user).to_owned(),
            serde_json::Value::from(user),
        );
        body.insert("password".to_owned(), serde_json::Value::from(password));

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            let api: LoginApiResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("login response could not be parsed: {e}"))
            })?;
            let expires_at = extract_exp(&api.token);
            Ok(LoginResponse {
                token: api.token,
                user: user.to_string(),
                expires_at,
            })
        } else if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            let body = response.text().unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            tracing::trace!("auth failure response body (capped): {}", preview);
            // Username MUST NOT appear in the error message (ADR-0007 / PRD AC 7).
            Err(Diagnostic::AuthRequired(format!(
                "Authentication failed on {server}"
            )))
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(Diagnostic::NotFound(format!(
                "endpoint not found at {url}; check that --server resolves to a DSP-API instance, not just any HTTPS host"
            )))
        } else if status.is_server_error() {
            let body = response.text().unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            tracing::trace!("server error response body (capped): {}", preview);
            Err(Diagnostic::ServerError(format!("server returned {status}")))
        } else {
            Err(Diagnostic::ServerError(format!(
                "unexpected status: {status}"
            )))
        }
    }

    fn resolve_project(&self, server: &str, project: &str) -> Result<ProjectRef, Diagnostic> {
        let base = server.trim_end_matches('/');

        let url = project_lookup_url(base, project);

        // Project lookup endpoints are public — no Authorization header.
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            let api: ProjectGetApiResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("project lookup response could not be parsed: {e}"))
            })?;
            if !is_safe_shortcode(&api.project.shortcode) {
                return Err(Diagnostic::ServerError(
                    "server returned a project with an unexpected shortcode".into(),
                ));
            }
            Ok(ProjectRef {
                iri: api.project.id,
                shortcode: api.project.shortcode,
                shortname: api.project.shortname,
            })
        } else if status == reqwest::StatusCode::NOT_FOUND {
            // Cap a long IRI input at ~80 chars for readability.
            let display_input: String = project.chars().take(80).collect();
            let suffix = if project.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::NotFound(format!(
                "project '{display_input}{suffix}' not found on {server}"
            )))
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn create_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        skip_assets: bool,
        token: &str,
    ) -> Result<CreateDumpOutcome, Diagnostic> {
        let base = server.trim_end_matches('/');
        // DSP-API calls this resource an "export" — the CLI calls it a "dump".
        // The word "export" is confined to this URL and http.rs internals only;
        // the trait and all layers above use "dump" exclusively (ADR-0001).
        // skipAssets is a query parameter: ?skipAssets=true|false
        let url = format!(
            "{base}/v3/projects/{}/exports?skipAssets={skip_assets}",
            enc(project_iri)
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .send()
            .map_err(|e: reqwest::Error| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        match status.as_u16() {
            202 => {
                let api: DataTaskStatusApiResponse = response.json().map_err(|e| {
                    Diagnostic::ServerError(format!(
                        "dump trigger response could not be parsed: {e}"
                    ))
                })?;
                api.into_dump_task().map(CreateDumpOutcome::Created)
            }
            409 => {
                // Parse the conflict body to determine same- vs. cross-project conflict.
                // A 409 with code=="export_exists" carries the occupying dump's id and
                // projectIri. Compare that IRI to the requested project_iri to decide
                // whether to return Exists (same project) or ExistsForOtherProject (different).
                //
                // Guard against serde-parsing an absurdly large conflict body
                // (the body is already buffered by `text()`; reqwest's request
                // timeout bounds the wire read). The `<= 65536` guard only
                // avoids serde-parsing an oversized string.
                let body_text = response.text().unwrap_or_default();
                let error_body: Option<V3ErrorBody> = if body_text.len() <= 65536 {
                    serde_json::from_str(&body_text).ok()
                } else {
                    None
                };
                match error_body.as_ref().and_then(|b| b.export_exists()) {
                    Some(ex) => {
                        // Distinct message from the outer `None` — here the export-exists error
                        // item WAS present but lacked an id (vs. no parseable item at all).
                        let id = ex.id.ok_or_else(|| {
                            Diagnostic::ServerError(
                                "the server's dump-conflict response was missing the dump id"
                                    .into(),
                            )
                        })?;
                        validate_dump_id(id)?;
                        // Both IRIs originate from the same DSP-API instance (the request IRI is
                        // ProjectRef::iri, parsed from a prior server response; the body IRI is
                        // the server's own), so a direct string compare is sound — they are
                        // canonical and identically formed. No normalization needed. If the CLI
                        // ever accepts raw user IRIs here, canonicalize at the input boundary.
                        match ex.project_iri {
                            Some(owner) if owner == project_iri => {
                                Ok(CreateDumpOutcome::Exists { id: id.to_string() })
                            }
                            Some(owner) => Ok(CreateDumpOutcome::ExistsForOtherProject {
                                id: id.to_string(),
                                project_iri: owner.to_string(),
                            }),
                            // FAIL CLOSED — see Decision 1. The field is contractually always
                            // present; its absence is an unexpected response we will not guess on.
                            None => Err(Diagnostic::ServerError(
                                "the server's dump-conflict response did not identify which \
project owns the existing dump; cannot safely proceed"
                                    .into(),
                            )),
                        }
                    }
                    // No `export_exists` error item at all (unparseable / different conflict).
                    None => Err(Diagnostic::ServerError(
                        // ADR-0001: user-facing text — no DSP-API "export" vocabulary
                        "server reported a 409 conflict whose detail could not be parsed".into(),
                    )),
                }
            }
            401 | 403 => Err(Diagnostic::AuthRequired(
                "triggering a project dump requires a system-administrator token".into(),
            )),
            404 => Err(Diagnostic::NotFound(format!("project not found at {url}"))),
            _ => Err(map_unexpected_status(status, &url)),
        }
    }

    fn get_project_dump_status(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
    ) -> Result<DumpTask, Diagnostic> {
        validate_dump_id(dump_id)?;
        let base = server.trim_end_matches('/');
        // dump_id is URL-safe base64 — inserted verbatim (no encoding).
        let url = format!("{base}/v3/projects/{}/exports/{dump_id}", enc(project_iri));

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        match status.as_u16() {
            200 => {
                let api: DataTaskStatusApiResponse = response.json().map_err(|e| {
                    Diagnostic::ServerError(format!(
                        "dump status response could not be parsed: {e}"
                    ))
                })?;
                api.into_dump_task()
            }
            404 => Err(Diagnostic::NotFound(format!(
                "dump '{dump_id}' not found for project at {url}"
            ))),
            401 | 403 => Err(Diagnostic::AuthRequired(
                "fetching dump status requires a system-administrator token".into(),
            )),
            _ => Err(map_unexpected_status(status, &url)),
        }
    }

    fn download_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
        dest: &mut dyn Write,
    ) -> Result<u64, Diagnostic> {
        validate_dump_id(dump_id)?;
        let base = server.trim_end_matches('/');
        // dump_id is URL-safe base64 — inserted verbatim (no encoding).
        let url = format!(
            "{base}/v3/projects/{}/exports/{dump_id}/download",
            enc(project_iri)
        );

        // Use download_client (no overall/read timeout) for potentially large archives.
        let mut response = self
            .download_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        // Check status BEFORE reading the body — avoid streaming a large error body.
        // Content-Disposition is intentionally NOT honoured: the action owns the filename.
        match status.as_u16() {
            200 => {
                // Manual buffered loop so read-side errors (network) and
                // write-side errors (disk full / dest failure) are classified
                // separately — io::copy would attribute both to the same error.
                let mut buf = [0u8; 64 * 1024];
                let mut total: u64 = 0;
                loop {
                    let n = response
                        .read(&mut buf)
                        .map_err(|e| Diagnostic::Network(format!("download interrupted: {e}")))?;
                    if n == 0 {
                        break;
                    }
                    dest.write_all(&buf[..n]).map_err(|e| {
                        Diagnostic::Io(format!("failed to write dump to disk: {e}"))
                    })?;
                    total += n as u64;
                }
                Ok(total)
            }
            409 => Err(Diagnostic::Conflict(
                "dump not ready — still in progress or failed".into(),
            )),
            404 => Err(Diagnostic::NotFound(format!(
                "dump '{dump_id}' not found at {url}"
            ))),
            401 | 403 => Err(Diagnostic::AuthRequired(
                "downloading a project dump requires a system-administrator token".into(),
            )),
            _ => Err(map_unexpected_status(status, &url)),
        }
    }

    fn delete_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
    ) -> Result<(), Diagnostic> {
        validate_dump_id(dump_id)?;
        let base = server.trim_end_matches('/');
        // dump_id is URL-safe base64 — inserted verbatim (no encoding).
        let url = format!("{base}/v3/projects/{}/exports/{dump_id}", enc(project_iri));

        let response = self
            .client
            .delete(&url)
            .bearer_auth(token)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        match status.as_u16() {
            204 => Ok(()),
            409 => Err(Diagnostic::Conflict(
                "dump is still in progress and cannot be deleted yet".into(),
            )),
            404 => Err(Diagnostic::NotFound(format!(
                "dump '{dump_id}' not found at {url}"
            ))),
            401 | 403 => Err(Diagnostic::AuthRequired(
                "deleting a project dump requires a system-administrator token".into(),
            )),
            _ => Err(map_unexpected_status(status, &url)),
        }
    }

    fn list_projects(&self, server: &str, token: Option<&str>) -> Result<Vec<Project>, Diagnostic> {
        let base = server.trim_end_matches('/');
        let url = format!("{base}/admin/projects");

        // Build the request: conditionally add Bearer auth ONLY when a token is
        // provided. When `token` is `None` the request is sent without any
        // Authorization header (public endpoint). Do NOT pass an empty/dummy
        // bearer — that would change request semantics vs. a truly unauthenticated
        // call.
        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            let api: ProjectsListApiResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("projects list response could not be parsed: {e}"))
            })?;
            let projects = api
                .projects
                .into_iter()
                .map(|dto| Project {
                    iri: dto.id,
                    shortcode: dto.shortcode,
                    shortname: dto.shortname,
                    longname: dto.longname,
                    // `status` bool → `ProjectStatus` enum: `true` = active, `false` = inactive.
                    // Confirmed from live data: active research projects have `status: true`;
                    // deprecated/test projects have `status: false`. See ADR-0001.
                    status: if dto.status {
                        ProjectStatus::Active
                    } else {
                        ProjectStatus::Inactive
                    },
                    // `ontologies` is the DSP-API wire name; `data_models` is the dsp-cli
                    // vocabulary (ADR-0001 boundary). The count is all we need here.
                    data_models: dto.ontologies.len(),
                })
                .collect();
            Ok(projects)
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn describe_project(
        &self,
        server: &str,
        project: &str,
        token: Option<&str>,
    ) -> Result<ProjectDetail, Diagnostic> {
        let base = server.trim_end_matches('/');
        let url = project_lookup_url(base, project);

        // Build the request: conditionally add Bearer auth ONLY when a token is
        // provided. When `token` is `None` the request is sent without any
        // Authorization header (public endpoint). Mirrors `list_projects`.
        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            let api: ProjectDetailApiResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("project lookup response could not be parsed: {e}"))
            })?;
            let dto = api.project;

            // Translate `status` bool → enum (true = Active, false = Inactive).
            let project_status = if dto.status {
                ProjectStatus::Active
            } else {
                ProjectStatus::Inactive
            };

            // Translate description Vec, order preserved.
            let description = dto
                .description
                .into_iter()
                .map(|d| ProjectDescription {
                    value: d.value,
                    language: d.language,
                })
                .collect();

            // Translate ontology IRIs → DataModelSummary, sorted by name ascending.
            let mut data_models: Vec<DataModelSummary> = dto
                .ontologies
                .into_iter()
                .map(|iri| {
                    let name = data_model_name_from_iri(&iri);
                    DataModelSummary { name, iri }
                })
                .collect();
            data_models.sort_by(|a, b| a.name.cmp(&b.name));

            Ok(ProjectDetail {
                iri: dto.id,
                shortcode: dto.shortcode,
                shortname: dto.shortname,
                longname: dto.longname,
                status: project_status,
                description,
                keywords: dto.keywords,
                data_models,
            })
        } else if status == reqwest::StatusCode::NOT_FOUND {
            // Cap a long input at ~80 chars for readability, mirroring resolve_project.
            let display_input: String = project.chars().take(80).collect();
            let suffix = if project.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::NotFound(format!(
                "project '{display_input}{suffix}' not found on {server}. Run `dsp vre project list --server {server}` to see available projects."
            )))
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn describe_data_model(
        &self,
        server: &str,
        data_model_iri: &str,
        token: Option<&str>,
    ) -> Result<DataModelDetail, Diagnostic> {
        let resp = self.fetch_allentities(server, data_model_iri, token)?;

        // Build an owned prefix → namespace map BEFORE consuming the graph,
        // so nothing borrows `resp` across the `into_iter()` that moves it.
        // Object-valued context terms are silently skipped — intended (see Risk 9).
        let prefixes: HashMap<String, String> = resp
            .context
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        let mut resource_types: Vec<ResourceTypeSummary> = resp
            .graph
            .into_iter()
            .filter(|dto| dto.is_resource_class)
            .map(|dto| {
                let (name, iri) = expand_class_id(&dto.id, &prefixes);
                ResourceTypeSummary {
                    name,
                    iri,
                    label: dto.label,
                }
            })
            .collect();

        resource_types.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(DataModelDetail {
            name: data_model_name_from_iri(&resp.id),
            iri: resp.id,
            label: resp.label,
            last_modified: resp.last_modification_date.map(|d| d.value),
            resource_types,
        })
    }

    fn data_model_structure(
        &self,
        server: &str,
        data_model_iri: &str,
        token: Option<&str>,
    ) -> Result<DataModelStructure, Diagnostic> {
        // ── 1. Fetch allentities (single fetch — no sibling fetch in v1) ────────
        let resp = self.fetch_allentities(server, data_model_iri, token)?;

        let graph_entities: Vec<OntologyEntityDto> = resp.graph;

        // ── 2. Build property-node lookup ────────────────────────────────────────
        // Partition graph into resource classes and property nodes. Property nodes
        // carry objectType / isLinkProperty / isResourceProperty. Resource classes
        // carry is_resource_class. The two sets are used separately, so we split
        // once rather than cloning. (OntologyEntityDto does not derive Clone.)
        let mut prop_lookup: HashMap<String, OntologyEntityDto> = HashMap::new();
        let mut class_nodes: Vec<OntologyEntityDto> = Vec::new();
        for entity in graph_entities {
            if entity.is_resource_class {
                class_nodes.push(entity);
            } else if entity.object_type.is_some()
                || entity.is_link_property
                || entity.is_resource_property
            {
                prop_lookup.insert(entity.id.clone(), entity);
            }
        }

        // ── 3. Collect relations ─────────────────────────────────────────────────
        let mut relations: Vec<Relation> = Vec::new();

        for class in &class_nodes {
            let source = local_name(&class.id).to_string();

            for element in &class.sub_class_of {
                if let Some(type_val) = element.get("@type")
                    && type_val.as_str() == Some("owl:Restriction")
                {
                    // ── Link edge ────────────────────────────────────────────────
                    let on_prop_id = match element
                        .get("owl:onProperty")
                        .and_then(|v| v.get("@id"))
                        .and_then(serde_json::Value::as_str)
                    {
                        Some(s) => s,
                        None => continue,
                    };

                    // Look up the property node (may be absent for cross-DM props).
                    let node = match prop_lookup.get(on_prop_id) {
                        Some(n) => n,
                        None => continue, // v1 limitation: cross-DM prop absent → skip
                    };

                    // Drop link-value reification twins (…Value properties).
                    if node.is_link_value_property {
                        continue;
                    }

                    // Only link properties produce relation edges.
                    if !node.is_link_property {
                        continue;
                    }

                    // Target: objectType @id local name.
                    let target_id = match node.object_type.as_ref() {
                        Some(ot) => &ot.id,
                        None => continue, // no target — skip
                    };
                    let target = local_name(target_id).to_string();

                    let t_prefix = curie_prefix(target_id).unwrap_or("");
                    let target_data_model = if is_system_prefix(t_prefix) || t_prefix.is_empty() {
                        None
                    } else {
                        Some(t_prefix.to_string())
                    };

                    // is_builtin for link: keyed off the FIELD's CURIE prefix.
                    let field_prefix = curie_prefix(on_prop_id).unwrap_or("");
                    let is_builtin = is_system_prefix(field_prefix);

                    let field = local_name(on_prop_id).to_string();

                    relations.push(Relation {
                        source: source.clone(),
                        target,
                        kind: RelationKind::Link,
                        field: Some(field),
                        target_data_model,
                        is_builtin,
                    });
                } else if let Some(id_val) = element.get("@id").and_then(serde_json::Value::as_str)
                {
                    // ── Inherits edge ────────────────────────────────────────────
                    // Bare {"@id": "..."} entries are superclass refs (skip blank
                    // nodes / owl:Restriction entries which have @type, not @id at
                    // the top level here).
                    let target = local_name(id_val).to_string();

                    let sup_prefix = curie_prefix(id_val).unwrap_or("");
                    let is_builtin = is_system_prefix(sup_prefix);
                    let target_data_model = if is_system_prefix(sup_prefix) || sup_prefix.is_empty()
                    {
                        None
                    } else {
                        Some(sup_prefix.to_string())
                    };

                    relations.push(Relation {
                        source: source.clone(),
                        target,
                        kind: RelationKind::Inherits,
                        field: None,
                        target_data_model,
                        is_builtin,
                    });
                }
            }
        }

        // ── 4. Sort by (source, kind, field, target) — D6 ───────────────────────
        // RelationKind derives Ord with Link < Inherits.
        // Option<String> sorts None < Some (standard Ord).
        relations.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| a.field.cmp(&b.field))
                .then_with(|| a.target.cmp(&b.target))
        });

        // ── 5. Build and return DataModelStructure ───────────────────────────────
        Ok(DataModelStructure {
            data_model: data_model_name_from_iri(data_model_iri),
            relations,
        })
    }

    fn list_resources(
        &self,
        server: &str,
        project_iri: &str,
        resource_type_iri: &str,
        order_by: Option<&str>,
        page: u32,
        token: Option<&str>,
    ) -> Result<ResourcePage, Diagnostic> {
        let base = server.trim_end_matches('/');
        let url = format!("{base}/v2/resources");

        // Build the request with query params via reqwest .query() — NEVER manual
        // string interpolation, which would not URL-encode the resource-type IRI safely.
        // The DSP-API wire parameter name is "resourceClass" (unchanged — stays here at
        // the client boundary, per ADR-0001 vocabulary divergence).
        let mut req = self.client.get(&url).query(&[
            ("resourceClass", resource_type_iri),
            ("page", &page.to_string()),
            ("schema", "complex"),
        ]);
        // `order_by` is the already-resolved complex-schema property IRI; pass verbatim.
        // reqwest .query() is additive and URL-encodes automatically.
        if let Some(prop_iri) = order_by {
            req = req.query(&[("orderByProperty", prop_iri)]);
        }

        // Set x-knora-accept-project header via the fallible HeaderValue path.
        // An IRI containing CRLF or other invalid header bytes is a Usage error
        // (the caller supplied a bad IRI), not an Internal error. No unwrap.
        let header_value = reqwest::header::HeaderValue::from_str(project_iri).map_err(|e| {
            Diagnostic::Usage(format!("project IRI is not a valid HTTP header value: {e}"))
        })?;
        let req = req.header("x-knora-accept-project", header_value);

        // Conditional bearer auth — mirrors list_projects.
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if !status.is_success() {
            return Err(map_unexpected_status(status, &url));
        }

        let dto: ResourceListDto = response.json().map_err(|e| {
            Diagnostic::ServerError(format!("resource list response could not be parsed: {e}"))
        })?;

        let may_have_more_results = dto.may_have_more_results;

        // Distinguish the three JSON-LD forms:
        // 1. @graph present → many results
        // 2. @id present (but no @graph) → single result
        // 3. neither → empty
        let resources: Vec<ResourceSummary> = if let Some(graph) = dto.graph {
            graph
                .into_iter()
                .map(|node| {
                    node_dto_to_summary(
                        node.id,
                        node.type_field.as_ref(),
                        node.label.as_ref(),
                        node.ark_url.as_ref(),
                        node.creation_date.as_ref(),
                        node.last_modification_date.as_ref(),
                    )
                })
                .collect()
        } else if let Some(id) = dto.id {
            // Single result: the top-level fields carry the single node's data.
            vec![node_dto_to_summary(
                id,
                dto.type_field.as_ref(),
                dto.label.as_ref(),
                dto.ark_url.as_ref(),
                dto.creation_date.as_ref(),
                dto.last_modification_date.as_ref(),
            )]
        } else {
            // Empty result.
            vec![]
        };

        Ok(ResourcePage {
            resources,
            may_have_more_results,
        })
    }

    fn describe_resource(
        &self,
        server: &str,
        resource_iri: &str,
        token: Option<&str>,
        with_values: bool,
    ) -> Result<ResourceDetail, Diagnostic> {
        let base = server.trim_end_matches('/');
        // D5: percent-encode the IRI for safe insertion as a single URL path segment.
        let url = format!("{base}/v2/resources/{}", enc(resource_iri));

        // Build request with conditional bearer auth. NEVER log the token.
        let req = self.client.get(&url).query(&[("schema", "complex")]);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if status.is_success() {
            let dto: ResourceDetailDto = response.json().map_err(|e| {
                Diagnostic::ServerError(format!(
                    "resource describe response could not be parsed: {e}"
                ))
            })?;

            // Boundary translation (ADR-0001): wire DTO → domain model.
            let label = dto
                .label
                .as_ref()
                .and_then(extract_string_value)
                .unwrap_or_default();
            let resource_type = extract_resource_type(dto.type_field.as_ref());
            let ark_url = dto.ark_url.as_ref().and_then(extract_string_value);
            let creation_date = dto.creation_date.as_ref().and_then(extract_string_value);
            let last_modified = dto
                .last_modification_date
                .as_ref()
                .and_then(extract_string_value);
            let attached_project = dto
                .attached_to_project
                .as_ref()
                .and_then(extract_string_value);
            let owner = dto.attached_to_user.as_ref().and_then(extract_string_value);
            let visibility = dto.has_permissions.as_deref().and_then(derive_visibility);
            let your_access = dto.user_has_permission.as_deref().and_then(derive_access);

            // When with_values == false: exactly 8b behaviour — values = None, no extra fetches.
            let values = if with_values {
                Some(self.parse_resource_values(server, token, &dto.context, &dto.extra))
            } else {
                None
            };

            Ok(ResourceDetail {
                label,
                iri: dto.id,
                resource_type,
                ark_url,
                creation_date,
                last_modified,
                attached_project,
                owner,
                visibility,
                your_access,
                values,
            })
        } else if status == reqwest::StatusCode::NOT_FOUND {
            // Cap the resource IRI at 80 chars for readability, mirroring resolve_project.
            let display_iri: String = resource_iri.chars().take(80).collect();
            let iri_suffix = if resource_iri.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::NotFound(format!(
                "resource '{display_iri}{iri_suffix}' not found"
            )))
        } else if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            // Deliberate: an anonymous caller describing a private resource gets 403.
            // AuthRequired (exit 3 + login hint) is the right UX for an auth-optional read.
            // NEVER log the token — not in any Diagnostic or tracing call.
            let display_iri: String = resource_iri.chars().take(80).collect();
            let iri_suffix = if resource_iri.chars().count() > 80 {
                "…"
            } else {
                ""
            };
            Err(Diagnostic::AuthRequired(format!(
                "access denied for resource '{display_iri}{iri_suffix}' — log in to view this resource"
            )))
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn verify_token(&self, server: &str, token: &str) -> Result<(), Diagnostic> {
        let url = format!("{}/v2/authentication", server.trim_end_matches('/'));

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            // Drain the response body so the connection can be returned to the pool.
            // NEVER log the token — log the drained body at trace level only.
            let body = response.text().unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            tracing::trace!("verify_token success response body (capped): {}", preview);
            Ok(())
        } else if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            // Drain the response body so pooled connections behave.
            let body = response.text().unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            tracing::trace!("verify_token rejection response body (capped): {}", preview);
            // Token MUST NOT appear in the error message.
            Err(Diagnostic::AuthRequired(format!(
                "token rejected by {server} — it may be expired, revoked, or for a different environment"
            )))
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn list_data_models(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<Vec<DataModel>, Diagnostic> {
        let url = format!(
            "{}/v2/ontologies/metadata/{}",
            server.trim_end_matches('/'),
            enc(project_iri)
        );

        // Build the request: conditionally add Bearer auth ONLY when a token is
        // provided. When `token` is `None` the request is sent without any
        // Authorization header (public endpoint). Mirrors `list_projects`.
        // NEVER log the token — it must not appear in any Diagnostic or tracing call.
        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;

        let status = response.status();

        if status.is_success() {
            let resp: OntologyMetadataResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!("data-models response could not be parsed: {e}"))
            })?;

            // `@graph` present → use it (covers multi AND a server that wraps a single
            // ontology in a length-1 array). Else a flattened top-level `@id` → one
            // ontology. Else `{}` → none. Order matters: never reorder these arms.
            let dtos: Vec<OntologyMetadataDto> = match resp.graph {
                Some(g) => g,
                None => match resp.id {
                    Some(id) => vec![OntologyMetadataDto {
                        id,
                        label: resp.label,
                        last_modification_date: resp.last_modification_date,
                    }],
                    None => vec![],
                },
            };

            let data_models = dtos
                .into_iter()
                .map(|dto| DataModel {
                    name: data_model_name_from_iri(&dto.id),
                    iri: dto.id,
                    label: dto.label,
                    last_modified: dto.last_modification_date.map(|d| d.value),
                    is_builtin: false,
                })
                .collect();

            Ok(data_models)
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn describe_resource_type(
        &self,
        server: &str,
        data_model_iri: &str,
        resource_type: &str,
        token: Option<&str>,
    ) -> Result<ResourceTypeDetail, Diagnostic> {
        // ── 1. Fetch allentities for the queried data-model ───────────────────
        let resp = self.fetch_allentities(server, data_model_iri, token)?;

        // Build prefix → namespace map before consuming resp.graph.
        let prefixes: HashMap<String, String> = resp
            .context
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        // ── 2. Find the target class in this ontology's @graph only ───────────
        // Expand the queried resource_type: check if it looks like a CURIE or full IRI
        // to allow exact IRI matching.
        let queried_id = resp.id;
        let mut graph_entities: Vec<OntologyEntityDto> = resp.graph;

        let target_idx = graph_entities.iter().position(|e| {
            if !e.is_resource_class {
                return false;
            }
            let (type_local, expanded_iri) = expand_class_id(&e.id, &prefixes);
            // Case-insensitive local name match OR exact IRI match.
            type_local.eq_ignore_ascii_case(resource_type) || expanded_iri == resource_type
        });

        let target_idx = match target_idx {
            Some(i) => i,
            None => {
                let display: String = resource_type.chars().take(80).collect();
                let suffix = if resource_type.chars().count() > 80 {
                    "…"
                } else {
                    ""
                };
                return Err(Diagnostic::NotFound(format!(
                    "resource-type '{display}{suffix}' not found in data-model '{}' on {server}",
                    data_model_name_from_iri(data_model_iri)
                )));
            }
        };

        // Extract the target class from the vec (swap_remove is fine — we only need
        // target's fields, and we iterate graph_entities for property nodes separately).
        let target = graph_entities.swap_remove(target_idx);

        // ── 3. Parse rdfs:subClassOf into superclass refs + restrictions ───────
        struct Restriction {
            on_property_id: String,
            cardinality: Cardinality,
            gui_order: u32,
        }

        let mut restrictions: Vec<Restriction> = Vec::new();
        let mut super_type_ids: Vec<String> = Vec::new();
        let mut restriction_prop_locals: Vec<String> = Vec::new();

        for element in &target.sub_class_of {
            if let Some(type_val) = element.get("@type")
                && type_val.as_str() == Some("owl:Restriction")
            {
                // It's a restriction
                let on_prop_id = element
                    .get("owl:onProperty")
                    .and_then(|v| v.get("@id"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();

                if on_prop_id.is_empty() {
                    tracing::warn!("owl:Restriction missing owl:onProperty @id; skipping");
                    continue;
                }

                let cardinality = decode_cardinality(element);
                let gui_order = element
                    .get("salsah-gui:guiOrder")
                    .and_then(serde_json::Value::as_u64)
                    .map(|v| v as u32)
                    .unwrap_or(u32::MAX);

                restriction_prop_locals.push(local_name(&on_prop_id).to_string());

                restrictions.push(Restriction {
                    on_property_id: on_prop_id,
                    cardinality,
                    gui_order,
                });
                continue;
            }
            // Not a restriction — it's a superclass ref: {"@id": "..."}
            if let Some(id_val) = element.get("@id").and_then(serde_json::Value::as_str) {
                super_type_ids.push(id_val.to_string());
            }
        }

        // ── 4. Representation (Decision 5 / R8): from file-value restrictions ─
        let representation = detect_representation(
            &restriction_prop_locals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );

        // ── 5. Build property node lookup from the queried ontology ───────────
        let mut prop_lookup: HashMap<String, OntologyEntityDto> = HashMap::new();
        for entity in graph_entities {
            // Property nodes have an objectType or isResourceProperty/isLinkProperty.
            // Use object_type as the discriminant (property nodes carry it; class nodes don't).
            if entity.object_type.is_some()
                || entity.is_link_property
                || entity.is_resource_property
            {
                prop_lookup.insert(entity.id.clone(), entity);
            }
        }

        // ── 6. Sibling-fetch: resolve missing non-system property nodes ────────
        // Collect restriction onProperty ids whose node is absent AND whose CURIE
        // prefix is not system-namespace.
        //
        // SSRF note (R10): sibling IRIs from the server's @context are used ONLY
        // as the percent-encoded path argument of
        //   `{server}/v2/ontologies/allentities/{enc(sibling_iri)}`
        // The host is always the user-supplied `server` argument. A hostile @context
        // cannot redirect requests or the bearer token to a foreign host.
        let mut missing_prefixes: Vec<String> = Vec::new();
        let mut seen_prefixes: HashSet<String> = HashSet::new();
        for restriction in &restrictions {
            if prop_lookup.contains_key(&restriction.on_property_id) {
                continue;
            }
            let prefix = match curie_prefix(&restriction.on_property_id) {
                Some(p) => p,
                None => continue,
            };
            if is_system_prefix(prefix) {
                continue;
            }
            if seen_prefixes.insert(prefix.to_string()) {
                missing_prefixes.push(prefix.to_string());
            }
        }

        // Resolve sibling IRIs from @context, dedup, cap at MAX_SIBLING_FETCHES.
        let mut fetched_sibling_iris: HashSet<String> = HashSet::new();
        let queried_iri_trimmed = data_model_iri.trim_end_matches(['#', '/']);

        let mut siblings_to_fetch: Vec<String> = Vec::new();
        for prefix in &missing_prefixes {
            let namespace = match prefixes.get(prefix.as_str()) {
                Some(ns) => ns,
                None => {
                    tracing::warn!(
                        prefix = %prefix,
                        "missing @context entry for prefix of cross-DM field; leaving best-effort"
                    );
                    continue;
                }
            };
            let sibling_iri = namespace.trim_end_matches(['#', '/']).to_string();
            if sibling_iri == queried_iri_trimmed {
                // Self-loop: this prefix resolves to the queried DM itself; skip.
                continue;
            }
            if fetched_sibling_iris.insert(sibling_iri.clone()) {
                siblings_to_fetch.push(sibling_iri);
            }
        }

        if siblings_to_fetch.len() > MAX_SIBLING_FETCHES {
            tracing::warn!(
                count = siblings_to_fetch.len(),
                max = MAX_SIBLING_FETCHES,
                "too many sibling ontologies to fetch; capping at MAX_SIBLING_FETCHES"
            );
            siblings_to_fetch.truncate(MAX_SIBLING_FETCHES);
        }

        for sibling_iri in &siblings_to_fetch {
            // SSRF guard: always use same `server`, never the raw IRI as a URL.
            match self.fetch_allentities(server, sibling_iri, token) {
                Ok(sibling_resp) => {
                    for entity in sibling_resp.graph {
                        if entity.object_type.is_some()
                            || entity.is_link_property
                            || entity.is_resource_property
                        {
                            prop_lookup.entry(entity.id.clone()).or_insert(entity);
                        }
                    }
                }
                Err(e) => {
                    // Non-fatal (R5): warn but continue — affected fields degrade.
                    // NEVER log the token or a credential-bearing URL.
                    tracing::warn!(
                        iri = %sibling_iri,
                        error = %e,
                        "sibling ontology fetch failed; affected fields left best-effort"
                    );
                }
            }
        }

        // ── 7. Build Vec<Field> from restrictions + merged lookup ─────────────
        let mut fields: Vec<(u32, Field)> = Vec::new();

        for restriction in &restrictions {
            let prop_id = &restriction.on_property_id;

            // Look up the property node (may be absent for system or failed-fetch fields).
            let node = prop_lookup.get(prop_id.as_str());

            // ── Twin drop (after merges — R-twin) ────────────────────────────
            if let Some(n) = node {
                if n.is_link_value_property {
                    // Authoritative node says it's a reification twin — drop it.
                    continue;
                }
            } else {
                // Node unavailable: apply name heuristic only when the node is missing.
                // If prop_id ends in "Value" and the base name is also a restriction
                // on this class, treat it as a twin and drop.
                let prop_local = local_name(prop_id);
                if let Some(base) = prop_local.strip_suffix("Value") {
                    // Look for a restriction whose local name equals `base` (CURIE match).
                    let base_present = restrictions
                        .iter()
                        .any(|r| local_name(&r.on_property_id) == base);
                    // Also check: `base` must be present as a restriction prop id
                    // (with any prefix, not just same prefix).
                    if base_present {
                        continue;
                    }
                }
            }

            // ── Field attributes ─────────────────────────────────────────────
            let prop_prefix = curie_prefix(prop_id).unwrap_or("");
            let is_builtin = is_system_prefix(prop_prefix);
            let (prop_local, prop_iri) = expand_class_id(prop_id, &prefixes);

            // data_model: system → None, otherwise the CURIE prefix (source DM).
            let field_data_model = if is_builtin {
                None
            } else {
                // Use the CURIE prefix as the source DM name.
                // Even for a failed-fetch field, we know its prefix.
                if prop_prefix.is_empty() {
                    None
                } else {
                    Some(prop_prefix.to_string())
                }
            };

            // value_type + link_target.
            let (value_type, link_target) = if let Some(n) = node {
                if n.is_link_property {
                    // Link property: objectType is the target resource class.
                    let target_name = n
                        .object_type
                        .as_ref()
                        .map(|ot| local_name(&ot.id).to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    (ValueType::Link, Some(target_name))
                } else {
                    let obj_local = n
                        .object_type
                        .as_ref()
                        .map(|ot| local_name(&ot.id))
                        .unwrap_or("");
                    (map_object_type_to_value_type(obj_local), None)
                }
            } else {
                // Node unavailable: try builtin file-value map; else Other/None.
                if is_builtin {
                    if let Some(vt) = builtin_field_value_type(&prop_local) {
                        (vt, None)
                    } else {
                        (ValueType::Other("—".to_string()), None)
                    }
                } else {
                    (ValueType::Other("—".to_string()), None)
                }
            };

            let label = node.and_then(|n| n.label.clone());

            // Check that link_target invariant is maintained.
            debug_assert!(
                (value_type == ValueType::Link) == link_target.is_some(),
                "link_target must be Some iff value_type is Link"
            );

            fields.push((
                restriction.gui_order,
                Field {
                    name: prop_local,
                    iri: prop_iri,
                    label,
                    value_type,
                    link_target,
                    cardinality: restriction.cardinality,
                    is_builtin,
                    data_model: field_data_model,
                },
            ));
        }

        // ── 8. Sort by guiOrder then name ─────────────────────────────────────
        fields.sort_by(|(order_a, field_a), (order_b, field_b)| {
            order_a
                .cmp(order_b)
                .then_with(|| field_a.name.cmp(&field_b.name))
        });
        let sorted_fields: Vec<Field> = fields.into_iter().map(|(_, f)| f).collect();

        // ── 9. super_types: non-system superclass refs ─────────────────────────
        let super_types: Vec<String> = super_type_ids
            .iter()
            .filter(|id| {
                let prefix = curie_prefix(id).unwrap_or("");
                !is_system_prefix(prefix)
            })
            .map(|id| local_name(id).to_string())
            .collect();

        // ── 10. Build ResourceTypeDetail ─────────────────────────────────────
        let (class_name, class_iri) = expand_class_id(&target.id, &prefixes);
        let class_label = target.label;
        let dm_name = data_model_name_from_iri(&queried_id);

        Ok(ResourceTypeDetail {
            name: class_name,
            iri: class_iri,
            label: class_label,
            data_model: dm_name,
            representation,
            super_types,
            fields: sorted_fields,
            count: None,
        })
    }

    fn resource_counts(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<HashMap<String, u64>, Diagnostic> {
        let url = format!(
            "{}/v3/projects/{}/resourcesPerOntology",
            server.trim_end_matches('/'),
            enc(project_iri)
        );

        // Conditionally add Bearer auth ONLY when a token is provided — mirrors
        // `list_data_models`. NEVER log the token.
        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if status.is_success() {
            let entries: Vec<OntologyAndResourceClassesDto> = response.json().map_err(|e| {
                Diagnostic::ServerError(format!(
                    "resource-counts response could not be parsed: {e}"
                ))
            })?;

            let mut counts = HashMap::new();
            for entry in entries {
                for cc in entry.classes_and_count {
                    counts.insert(cc.resource_class.iri, cc.item_count);
                }
            }
            Ok(counts)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(Diagnostic::NotFound(format!("project not found at {url}")))
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn list_vocabularies(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<Vec<Vocabulary>, Diagnostic> {
        let url = format!(
            "{}/admin/lists?projectIri={}",
            server.trim_end_matches('/'),
            enc(project_iri)
        );

        // Conditionally add Bearer auth ONLY when a token is provided — mirrors
        // `list_data_models`. NEVER log the token.
        let req = self.client.get(&url);
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            req
        };

        let response = req.send().map_err(|e| Diagnostic::Network(e.to_string()))?;
        let status = response.status();

        if status.is_success() {
            let resp: ListsListApiResponse = response.json().map_err(|e| {
                Diagnostic::ServerError(format!(
                    "vocabulary list response could not be parsed: {e}"
                ))
            })?;

            Ok(resp
                .lists
                .into_iter()
                .map(|dto| Vocabulary {
                    header: VocabularyHeader {
                        iri: dto.id,
                        name: dto.name,
                        labels: into_localized_texts(dto.labels),
                        comments: into_localized_texts(dto.comments),
                    },
                    // No per-tree fetch here — that's `--count`, an
                    // action-layer concern (see the trait doc comment).
                    node_count: None,
                    depth: None,
                })
                .collect())
        } else {
            Err(map_unexpected_status(status, &url))
        }
    }

    fn describe_vocabulary(
        &self,
        server: &str,
        iri: &str,
        token: Option<&str>,
    ) -> Result<VocabularyTree, Diagnostic> {
        match self.fetch_list_get(server, iri, token)? {
            ListGetResponseDto::Root(root) => Ok(build_vocabulary_tree(root.list, None)),
            ListGetResponseDto::Node(node) => {
                // D2: the addressed IRI is a node, not a root — resolve
                // upward and re-fetch. The subtree payload of THIS response
                // is discarded; the root fetch below carries the full tree.
                let root_iri = node.node.nodeinfo.has_root_node;
                match self.fetch_list_get(server, &root_iri, token)? {
                    ListGetResponseDto::Root(root) => {
                        Ok(build_vocabulary_tree(root.list, Some(iri.to_string())))
                    }
                    // One resolution hop only — no retry loop. A second
                    // node response here is a hard error, not a degrade.
                    ListGetResponseDto::Node(_) => Err(Diagnostic::ServerError(format!(
                        "resolving vocabulary node {iri} to its root ({root_iri}) returned \
                         another node, not a root"
                    ))),
                }
            }
        }
    }

    fn sparql_query(
        &self,
        server: &str,
        token: &str,
        query: &str,
        accept: &str,
        timeout_secs: u64,
    ) -> Result<crate::client::sparql::SparqlResponse, Diagnostic> {
        let url = format!("{}/admin/sparql/query", server.trim_end_matches('/'));

        tracing::debug!(method = "POST", url = %url, "sparql_query: sending request");

        // Built **per call**, not once in `HttpDspClient::new()` alongside
        // `client`/`download_client` (D17, amended 2026-08-07): the timeout is
        // per-invocation (`--timeout`, threaded through as `timeout_secs`), so
        // a client built once at startup could not carry a different bound on
        // every call. `.connect_timeout(10s)` + `.timeout(timeout_secs)`
        // (default 3600) is the settled shape — `read_timeout` (a true
        // inactivity bound) is unavailable: it exists only on
        // `reqwest::async_impl::client::ClientBuilder`
        // (`reqwest-0.13.4/src/async_impl/client.rs:1456`), not on
        // `reqwest::blocking::ClientBuilder`, which this crate is built on.
        // An async client + tokio runtime for this one method was considered
        // and rejected (owner, 2026-08-07): it puts async into a deliberately
        // blocking client layer for a bound the server's own 120s store
        // timeout (relayed as a 504) already provides in practice.
        //
        // Redirects are disabled. `POST /admin/sparql/query` never legitimately
        // redirects, and reqwest's default policy follows up to 10. The bearer
        // token is safe either way (reqwest strips `Authorization` cross-origin),
        // but a 307/308 **replays the request body** — here, the query text,
        // which D19 treats as privacy-sensitive — to a third host. Refusing to
        // follow removes the question instead of reasoning about it.
        let sparql_client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(crate::util::USER_AGENT)
            .build()
            .map_err(|e| {
                Diagnostic::Internal(format!("failed to build SPARQL HTTP client: {e}"))
            })?;

        let req = sparql_client
            .post(&url)
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, "application/sparql-query")
            .header(reqwest::header::ACCEPT, accept)
            .body(query.to_string());

        let response = req.send().map_err(|e| {
            // D17: the message must make a client-side timeout
            // distinguishable from the server's own 504 — reqwest's
            // `is_timeout()` covers both connect and read timeouts, and
            // there is no separate variant for each, so name the client-side
            // origin explicitly rather than leaving a bare `e.to_string()`
            // that could be misread as the server's guardrail.
            if e.is_timeout() {
                Diagnostic::Network(format!(
                    // `url` sanitised for the same reason as the 404 message.
                    "SPARQL request to {} timed out on the client side \
                     after {timeout_secs}s (--timeout) — this is distinct from \
                     the server's own passthrough timeout, which would come \
                     back as an HTTP 504: {e}",
                    crate::util::text::sanitise_and_cap(&url)
                ))
            } else {
                Diagnostic::Network(e.to_string())
            }
        })?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response
            .bytes()
            .map_err(|e| Diagnostic::Network(e.to_string()))?;

        tracing::debug!(
            status = status.as_u16(),
            content_type = content_type.as_deref().unwrap_or(""),
            "sparql_query: received response"
        );

        match classify_sparql_status(status.as_u16(), content_type.as_deref(), &body, &url) {
            SparqlOutcome::DspApiError(diag) => Err(diag),
            SparqlOutcome::Relay => {
                if !status.is_success() {
                    // Built inside the macro: `tracing` evaluates its arguments
                    // only when the callsite is enabled, so at default
                    // verbosity this costs nothing. Computing it eagerly meant
                    // decoding and stripping the body on every non-2xx relay
                    // to produce a string nobody reads.
                    tracing::trace!(
                        "sparql_query: non-2xx relay body preview (capped): {}",
                        crate::util::text::sanitise_bytes_for_prose(&body)
                    );
                }
                Ok(crate::client::sparql::SparqlResponse {
                    status: status.as_u16(),
                    content_type,
                    body: body.to_vec(),
                })
            }
        }
    }
}

/// A dsp-api-typed error's `{"message": …}` body, per D8's Verified API facts.
#[derive(serde::Deserialize)]
struct SparqlErrorBody {
    message: String,
}

/// Outcome of classifying a SPARQL-passthrough response status: either the
/// status is dsp-api's own failure (mapped to a `Diagnostic`), or the
/// response — whatever its status — is the triplestore's own and must be
/// relayed to the caller verbatim (D7: the *action* decides what a non-2xx
/// relay means; the client's job is only to distinguish dsp-api's own
/// failures from a relayed store status).
enum SparqlOutcome {
    DspApiError(Diagnostic),
    Relay,
}

/// Classify a `POST /admin/sparql/query` response status per D8's fixed
/// table. `url` is threaded through (mirroring `map_unexpected_status`)
/// because D9's `404` message must name the server that was queried.
///
/// This endpoint has its **own** classification table, separate from
/// `map_unexpected_status` — reused nowhere else and not reusing it, because
/// SPARQL passthrough's status vocabulary (`413`/`415`/dsp-api's typed
/// `500`-`504` exceptions) has no equivalent in the generic table.
fn classify_sparql_status(
    status: u16,
    content_type: Option<&str>,
    body: &[u8],
    url: &str,
) -> SparqlOutcome {
    match status {
        401 => SparqlOutcome::DspApiError(Diagnostic::AuthRequired(
            "authentication is required — run `dsp auth login`".into(),
        )),
        403 => SparqlOutcome::DspApiError(Diagnostic::AuthRequired(
            "your token is valid but is not a system administrator; \
             re-running `dsp auth login` will not help — the SPARQL \
             passthrough endpoint requires a SystemAdmin account"
                .into(),
        )),
        404 => SparqlOutcome::DspApiError(Diagnostic::NotFound(format!(
            // `url` is sanitised: it is built from `--server`, which may come
            // from a CWD `.env` via dotenvy rather than from the user's own
            // typing, so it is not automatically trustworthy prose.
            "the SPARQL passthrough is not available at {}. Any of these \
             looks identical from here: the endpoint is off on this deployment \
             (it is off by default — allow-sparql-passthrough), the server \
             predates the endpoint, the store's dataset is misconfigured, or \
             --server is wrong.",
            crate::util::text::sanitise_and_cap(url)
        ))),
        413 => SparqlOutcome::DspApiError(Diagnostic::Usage(
            "the SPARQL query text exceeds the server's request-body size \
             limit"
                .into(),
        )),
        415 => SparqlOutcome::DspApiError(Diagnostic::Internal(
            "the server rejected dsp-cli's own Content-Type \
             (application/sparql-query) with 415 — this is either a dsp-cli \
             bug or an unexpected server"
                .into(),
        )),
        500 | 502 | 503 | 504 => {
            let detail = parse_sparql_error_message(content_type, body)
                .unwrap_or_else(|| crate::util::text::sanitise_bytes_for_prose(body));
            // Always name the status. An empty `5xx` body would otherwise yield
            // `ServerError("")`, i.e. a bare `Error: server error:` with no
            // cause and no status; and a non-JSON body would surface as raw
            // store text with no sign it was a 500. The relay path in the
            // action states `(HTTP {status})` for the same reason.
            let message = if detail.trim().is_empty() {
                format!("the server returned HTTP {status} with no usable message")
            } else {
                format!("the server returned HTTP {status}: {detail}")
            };
            SparqlOutcome::DspApiError(Diagnostic::ServerError(message))
        }
        _ => SparqlOutcome::Relay,
    }
}

/// Parse a dsp-api typed-error `{"message": …}` body, if it parses as such.
/// Never assumes the body parses (D8's note on `413`/`415` having no
/// contracted body carries over defensively to the `500`-`504` row too).
///
/// The parsed `message` is sanitised and capped like any other server-supplied
/// text. This gate only proves the body is JSON carrying a `message` key — it
/// cannot prove dsp-api *authored* it. The store, an intermediate proxy, or a
/// hostile `--server` can all produce that shape, and the result is printed as
/// prose to a terminal by `main.rs` (this leaf's `output_format()` is `None`).
/// D7's invariant is "stderr is prose and is sanitised", with no carve-out for
/// a body that happens to parse.
fn parse_sparql_error_message(content_type: Option<&str>, body: &[u8]) -> Option<String> {
    if !content_type.unwrap_or("").starts_with("application/json") {
        return None;
    }
    serde_json::from_slice::<SparqlErrorBody>(body)
        .ok()
        .map(|b| crate::util::text::sanitise_and_cap(&b.message))
}

// ---------------------------------------------------------------------------
// Unit tests for pure helpers (classifier)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // `map_unexpected_status` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn map_unexpected_status_401_403_are_auth_required() {
        // 0.1.1: a read refused with 401 (missing/expired cached token) or 403
        // (permission) must surface as AuthRequired (exit 3) with a
        // re-authenticate hint — not a bare "unexpected status" runtime error.
        for status in [
            reqwest::StatusCode::UNAUTHORIZED,
            reqwest::StatusCode::FORBIDDEN,
        ] {
            let diag = map_unexpected_status(status, "https://example.org/x");
            match diag {
                Diagnostic::AuthRequired(msg) => assert!(
                    msg.contains("dsp auth login"),
                    "auth message should hint at re-authentication: {msg}"
                ),
                other => panic!("expected AuthRequired for {status}, got {other:?}"),
            }
        }
    }

    #[test]
    fn map_unexpected_status_404_and_5xx_stay_server_error() {
        // 404 and 5xx are not auth failures — they remain ServerError (exit 1),
        // preserving the existing contract (cf. the set_token 404 integration test).
        assert!(matches!(
            map_unexpected_status(reqwest::StatusCode::NOT_FOUND, "u"),
            Diagnostic::ServerError(_)
        ));
        assert!(matches!(
            map_unexpected_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "u"),
            Diagnostic::ServerError(_)
        ));
    }

    // ---------------------------------------------------------------------------
    // `identifier_key` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn identifier_key_email_contains_at() {
        assert_eq!(identifier_key("a@b.ch"), "email");
    }

    #[test]
    fn identifier_key_bare_username() {
        assert_eq!(identifier_key("jdoe"), "username");
    }

    #[test]
    fn identifier_key_http_iri() {
        assert_eq!(identifier_key("http://rdfh.ch/users/x"), "iri");
    }

    #[test]
    fn identifier_key_https_iri() {
        assert_eq!(identifier_key("https://rdfh.ch/users/x"), "iri");
    }

    #[test]
    fn identifier_key_iri_with_at_uses_iri_not_email() {
        // IRI prefix is checked before '@'; an '@' inside an IRI must not mis-classify.
        assert_eq!(identifier_key("http://example.org/users/a@b"), "iri");
    }

    #[test]
    fn classify_http_iri() {
        let ident = classify("http://rdfh.ch/projects/0001");
        assert!(
            matches!(ident, ProjectIdent::Iri(_)),
            "http:// prefix should classify as Iri"
        );
    }

    #[test]
    fn classify_https_iri() {
        let ident = classify("https://rdfh.ch/projects/0001");
        assert!(
            matches!(ident, ProjectIdent::Iri(_)),
            "https:// prefix should classify as Iri"
        );
    }

    #[test]
    fn classify_four_digit_hex_shortcode() {
        let ident = classify("0001");
        assert!(
            matches!(ident, ProjectIdent::Shortcode(_)),
            "four hex digits should classify as Shortcode"
        );
    }

    #[test]
    fn classify_four_hex_letter_shortcode() {
        // Documents the shortcode-wins overlap: `beef` is valid hex and exactly
        // 4 chars, so it classifies as Shortcode even if it looks like a shortname.
        // This is intentional (plan risks §6).
        let ident = classify("beef");
        assert!(
            matches!(ident, ProjectIdent::Shortcode(_)),
            "4-hex-letter input 'beef' should classify as Shortcode (documented overlap)"
        );
    }

    #[test]
    fn classify_mixed_case_hex_shortcode() {
        let ident = classify("ABCD");
        assert!(
            matches!(ident, ProjectIdent::Shortcode(_)),
            "upper-case hex digits should classify as Shortcode"
        );
    }

    #[test]
    fn classify_shortname() {
        let ident = classify("incunabula");
        assert!(
            matches!(ident, ProjectIdent::Shortname(_)),
            "alphabetic string longer than 4 chars should classify as Shortname"
        );
    }

    #[test]
    fn classify_five_digit_hex_is_shortname() {
        // 5 hex digits — not exactly 4, so falls through to Shortname.
        let ident = classify("00001");
        assert!(
            matches!(ident, ProjectIdent::Shortname(_)),
            "5-hex-digit string should classify as Shortname, not Shortcode"
        );
    }

    #[test]
    fn classify_three_digit_hex_is_shortname() {
        let ident = classify("001");
        assert!(
            matches!(ident, ProjectIdent::Shortname(_)),
            "3-hex-digit string should classify as Shortname, not Shortcode"
        );
    }

    #[test]
    fn classify_non_hex_four_chars_is_shortname() {
        // 4 chars but contains non-hex ('g') → Shortname.
        let ident = classify("zzzz");
        assert!(
            matches!(ident, ProjectIdent::Shortname(_)),
            "4-char non-hex string should classify as Shortname"
        );
    }

    // ---------------------------------------------------------------------------
    // `validate_dump_id` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_dump_id_valid_accepts() {
        assert!(super::validate_dump_id("abc123").is_ok());
        assert!(super::validate_dump_id("abc-123_XYZ").is_ok());
        // 256-char id is the upper bound — must still be accepted.
        let max_id = "a".repeat(256);
        assert!(
            super::validate_dump_id(&max_id).is_ok(),
            "256-char id must be accepted"
        );
    }

    #[test]
    fn validate_dump_id_empty_is_rejected() {
        let result = super::validate_dump_id("");
        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "empty id must be rejected"
        );
    }

    #[test]
    fn validate_dump_id_too_long_is_rejected() {
        let long_id = "a".repeat(257);
        let result = super::validate_dump_id(&long_id);
        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "257-char id must be rejected"
        );
    }

    #[test]
    fn validate_dump_id_invalid_chars_rejected() {
        let result = super::validate_dump_id("abc/def");
        assert!(
            matches!(result, Err(Diagnostic::ServerError(_))),
            "id with '/' must be rejected"
        );
    }

    // ---------------------------------------------------------------------------
    // `into_dump_task` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn into_dump_task_in_progress() {
        let api = DataTaskStatusApiResponse {
            id: "abc123".into(),
            status: "in_progress".into(),
            error_message: None,
            created_at: None,
        };
        let task = api.into_dump_task().expect("should parse in_progress");
        assert_eq!(task.id, "abc123");
        assert_eq!(task.status, DumpStatus::InProgress);
        assert!(task.error_message.is_none());
        assert!(task.created_at.is_none());
    }

    #[test]
    fn into_dump_task_completed() {
        let api = DataTaskStatusApiResponse {
            id: "done42".into(),
            status: "completed".into(),
            error_message: None,
            created_at: None,
        };
        let task = api.into_dump_task().expect("should parse completed");
        assert_eq!(task.status, DumpStatus::Completed);
    }

    #[test]
    fn into_dump_task_failed_with_message() {
        let api = DataTaskStatusApiResponse {
            id: "fail7".into(),
            status: "failed".into(),
            error_message: Some("disk full".into()),
            created_at: None,
        };
        let task = api.into_dump_task().expect("should parse failed");
        assert_eq!(task.status, DumpStatus::Failed);
        assert_eq!(task.error_message.as_deref(), Some("disk full"));
    }

    #[test]
    fn into_dump_task_unknown_status_is_server_error() {
        let api = DataTaskStatusApiResponse {
            id: "x".into(),
            status: "pending".into(), // not a known status
            error_message: None,
            created_at: None,
        };
        let result = api.into_dump_task();
        assert!(result.is_err(), "unknown status should yield an error");
        assert!(
            matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
            "unknown status should yield ServerError"
        );
    }

    #[test]
    fn into_dump_task_long_error_message_is_truncated() {
        // Build a message that is 501 chars long (just over the 500-char cap).
        let long_msg = "x".repeat(501);
        let api = DataTaskStatusApiResponse {
            id: "trunc".into(),
            status: "failed".into(),
            error_message: Some(long_msg),
            created_at: None,
        };
        let task = api
            .into_dump_task()
            .expect("should parse even with long message");
        let stored = task.error_message.unwrap();
        assert_eq!(
            stored.len(),
            500,
            "error_message must be truncated to ≤500 chars at the client boundary"
        );
    }

    #[test]
    fn into_dump_task_exact_500_chars_not_truncated() {
        // Exactly 500 chars — must pass through unchanged.
        let exact_msg = "y".repeat(500);
        let api = DataTaskStatusApiResponse {
            id: "exact".into(),
            status: "failed".into(),
            error_message: Some(exact_msg.clone()),
            created_at: None,
        };
        let task = api.into_dump_task().expect("should parse");
        assert_eq!(task.error_message.unwrap(), exact_msg);
    }

    // ---------------------------------------------------------------------------
    // `created_at` parsing unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn into_dump_task_valid_created_at_is_parsed() {
        let api = DataTaskStatusApiResponse {
            id: "ts-test".into(),
            status: "completed".into(),
            error_message: None,
            created_at: Some("2026-05-20T14:03:00Z".into()),
        };
        let task = api.into_dump_task().expect("should parse with created_at");
        use chrono::Datelike;
        let ts = task.created_at.expect("created_at should be Some");
        assert_eq!(ts.year(), 2026);
        assert_eq!(ts.month(), 5);
        assert_eq!(ts.day(), 20);
    }

    #[test]
    fn into_dump_task_garbage_created_at_yields_none() {
        let api = DataTaskStatusApiResponse {
            id: "ts-bad".into(),
            status: "in_progress".into(),
            error_message: None,
            created_at: Some("not-a-date!!".into()),
        };
        // Must succeed (garbage timestamp ≠ parse failure for the whole task).
        let task = api
            .into_dump_task()
            .expect("garbage created_at must not fail parse");
        assert!(
            task.created_at.is_none(),
            "garbage created_at must map to None"
        );
    }

    // ---------------------------------------------------------------------------
    // `V3ErrorBody::export_exists` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn export_exists_present_with_both_fields() {
        let body = V3ErrorBody {
            errors: vec![V3ErrorItem {
                code: "export_exists".into(),
                details: [
                    ("id".to_string(), "dGVzdC1pZA".to_string()),
                    (
                        "projectIri".to_string(),
                        "http://rdfh.ch/projects/0001".to_string(),
                    ),
                ]
                .into(),
            }],
        };
        let ex = body.export_exists().expect("export_exists must be Some");
        assert_eq!(ex.id, Some("dGVzdC1pZA"));
        assert_eq!(ex.project_iri, Some("http://rdfh.ch/projects/0001"));
    }

    #[test]
    fn export_exists_wrong_code_returns_none() {
        let body = V3ErrorBody {
            errors: vec![V3ErrorItem {
                code: "some_other_error".into(),
                details: [("id".to_string(), "abc".to_string())].into(),
            }],
        };
        assert!(body.export_exists().is_none(), "wrong code must not match");
    }

    #[test]
    fn export_exists_missing_details_id_returns_some_with_none_id() {
        let body = V3ErrorBody {
            errors: vec![V3ErrorItem {
                code: "export_exists".into(),
                details: [(
                    "projectIri".to_string(),
                    "http://rdfh.ch/projects/0001".to_string(),
                )]
                .into(),
            }],
        };
        // export_exists returns Some (the code matched) but id is None.
        let ex = body
            .export_exists()
            .expect("export_exists must be Some when code matches");
        assert!(ex.id.is_none(), "id must be None when 'id' key is absent");
        assert_eq!(ex.project_iri, Some("http://rdfh.ch/projects/0001"));
    }

    #[test]
    fn export_exists_empty_errors_returns_none() {
        let body = V3ErrorBody { errors: vec![] };
        assert!(body.export_exists().is_none());
    }

    #[test]
    fn export_exists_missing_project_iri_returns_some_with_none_iri() {
        let body = V3ErrorBody {
            errors: vec![V3ErrorItem {
                code: "export_exists".into(),
                details: [("id".to_string(), "abc123".to_string())].into(),
            }],
        };
        let ex = body
            .export_exists()
            .expect("export_exists must be Some when code matches");
        assert_eq!(ex.id, Some("abc123"));
        assert!(
            ex.project_iri.is_none(),
            "project_iri must be None when 'projectIri' key is absent"
        );
    }

    // ---------------------------------------------------------------------------
    // `is_safe_shortcode` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn is_safe_shortcode_valid_hex_shortcode() {
        assert!(
            super::is_safe_shortcode("0001"),
            "4-hex-digit shortcode must be accepted"
        );
        assert!(
            super::is_safe_shortcode("ABCD"),
            "upper-case hex shortcode must be accepted"
        );
        assert!(
            super::is_safe_shortcode("beef"),
            "lower-case hex shortcode must be accepted"
        );
    }

    #[test]
    fn is_safe_shortcode_alphanumeric_within_32_chars_accepted() {
        let long_code = "a".repeat(32);
        assert!(
            super::is_safe_shortcode(&long_code),
            "32-char alphanumeric must be accepted"
        );
    }

    #[test]
    fn is_safe_shortcode_empty_is_rejected() {
        assert!(
            !super::is_safe_shortcode(""),
            "empty shortcode must be rejected"
        );
    }

    #[test]
    fn is_safe_shortcode_too_long_is_rejected() {
        let long_code = "a".repeat(33);
        assert!(
            !super::is_safe_shortcode(&long_code),
            "33-char shortcode must be rejected"
        );
    }

    #[test]
    fn is_safe_shortcode_slash_is_rejected() {
        assert!(
            !super::is_safe_shortcode("ab/cd"),
            "shortcode with '/' must be rejected"
        );
        assert!(
            !super::is_safe_shortcode("/evil"),
            "absolute path shortcode must be rejected"
        );
    }

    #[test]
    fn is_safe_shortcode_dot_dot_is_rejected() {
        assert!(
            !super::is_safe_shortcode("../evil"),
            "path traversal shortcode must be rejected"
        );
        assert!(
            !super::is_safe_shortcode(".."),
            "'..' shortcode must be rejected"
        );
    }

    #[test]
    fn is_safe_shortcode_backslash_is_rejected() {
        assert!(
            !super::is_safe_shortcode("ab\\cd"),
            "shortcode with '\\' must be rejected"
        );
    }

    #[test]
    fn is_safe_shortcode_dot_is_rejected() {
        // A single '.' or mixed dots are not ASCII-alphanumeric.
        assert!(
            !super::is_safe_shortcode("ab.cd"),
            "shortcode with '.' must be rejected"
        );
    }

    #[test]
    fn resolve_project_rejects_unsafe_shortcode() {
        // Verify that the `is_safe_shortcode` guard in `resolve_project` rejects
        // a shortcode containing path-traversal characters. We test `is_safe_shortcode`
        // directly here since the HTTP boundary is the validation point.
        let unsafe_examples = ["../evil", "/abs", "ab/cd", "a\\b", ""];
        for s in &unsafe_examples {
            assert!(
                !super::is_safe_shortcode(s),
                "is_safe_shortcode must reject '{s}' — resolve_project would have returned ServerError for this input"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // `data_model_name_from_iri` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn data_model_name_from_iri_standard_form() {
        // Standard form: http://…/ontology/<code>/<name>/v2 → <name>
        assert_eq!(
            super::data_model_name_from_iri("http://api.dasch.swiss/ontology/0801/beol/v2"),
            "beol"
        );
    }

    #[test]
    fn data_model_name_from_iri_no_v2_suffix() {
        // No /v2 suffix: fall back to last path segment
        assert_eq!(
            super::data_model_name_from_iri("http://api.dasch.swiss/ontology/0801/beol"),
            "beol"
        );
    }

    #[test]
    fn data_model_name_from_iri_trailing_slash() {
        // Trailing slash is stripped before /v2 is checked
        assert_eq!(
            super::data_model_name_from_iri("http://api.dasch.swiss/ontology/0801/beol/v2/"),
            "beol"
        );
    }

    #[test]
    fn data_model_name_from_iri_bare_name() {
        // No slash at all: the whole string is the name
        assert_eq!(super::data_model_name_from_iri("beol"), "beol");
    }

    #[test]
    fn data_model_name_from_iri_empty_string() {
        // Empty input degrades silently to an empty name (benign; server contract trusted)
        assert_eq!(super::data_model_name_from_iri(""), "");
    }

    // ---------------------------------------------------------------------------
    // `expand_class_id` unit tests
    // ---------------------------------------------------------------------------

    fn beol_prefixes() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(
            "beol".to_string(),
            "http://api.dasch.swiss/ontology/0801/beol/v2#".to_string(),
        );
        m
    }

    #[test]
    fn expand_class_id_curie_expands_with_known_prefix() {
        // `beol:Archive` + a `beol` prefix → expanded IRI + name `Archive`
        let (name, iri) = super::expand_class_id("beol:Archive", &beol_prefixes());
        assert_eq!(name, "Archive");
        assert_eq!(iri, "http://api.dasch.swiss/ontology/0801/beol/v2#Archive");
    }

    #[test]
    fn expand_class_id_unknown_prefix_falls_back_to_raw_id() {
        // `urn:uuid:x` with no `urn` prefix in context → iri = raw `@id`, name = `x`
        let (name, iri) = super::expand_class_id("urn:uuid:x", &HashMap::new());
        assert_eq!(name, "x");
        assert_eq!(iri, "urn:uuid:x");
    }

    #[test]
    fn expand_class_id_full_iri_passes_through() {
        // `http://…/v2#Letter` has scheme `://`, so the local starts with `//` and
        // falls through to the passthrough arm. Name = `Letter`, IRI unchanged.
        let (name, iri) = super::expand_class_id(
            "http://api.dasch.swiss/ontology/0801/beol/v2#Letter",
            &beol_prefixes(),
        );
        assert_eq!(name, "Letter");
        assert_eq!(iri, "http://api.dasch.swiss/ontology/0801/beol/v2#Letter");
    }

    #[test]
    fn expand_class_id_no_colon_degenerate() {
        // `bare` has no colon at all → name = `bare`, iri = `bare`
        let (name, iri) = super::expand_class_id("bare", &HashMap::new());
        assert_eq!(name, "bare");
        assert_eq!(iri, "bare");
    }

    // ---------------------------------------------------------------------------
    // `local_name` unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn local_name_hash_iri() {
        assert_eq!(super::local_name("http://example.org/onto#Thing"), "Thing");
    }

    #[test]
    fn local_name_slash_iri() {
        assert_eq!(super::local_name("http://example.org/onto/Thing"), "Thing");
    }

    #[test]
    fn local_name_curie_colon() {
        assert_eq!(super::local_name("incunabula:Page"), "Page");
    }

    #[test]
    fn local_name_bare_name_fallback() {
        assert_eq!(super::local_name("Page"), "Page");
    }

    #[test]
    fn local_name_empty_string() {
        assert_eq!(super::local_name(""), "");
    }

    #[test]
    fn local_name_trailing_separator() {
        // Pins existing inline behaviour: rsplit yields Some("") for "foo#",
        // so the result is "" (the unwrap_or fallback is structurally dead here).
        assert_eq!(super::local_name("foo#"), "");
    }

    // ---------------------------------------------------------------------------
    // `object_type_to_kebab` / `map_object_type_to_value_type` unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn object_type_to_kebab_text_value() {
        assert_eq!(super::object_type_to_kebab("TextValue"), "text");
    }

    #[test]
    fn object_type_to_kebab_geom_value() {
        // "Geom" has no consecutive uppercase → "geom"
        assert_eq!(super::object_type_to_kebab("GeomValue"), "geom");
    }

    #[test]
    fn object_type_to_kebab_geo_name_value() {
        // "GeoName" — "N" follows lowercase "o", so insert "-" before "N"
        assert_eq!(super::object_type_to_kebab("GeoNameValue"), "geo-name");
    }

    #[test]
    fn object_type_to_kebab_uri_value() {
        // "URI" — three consecutive uppercase letters; "R" follows "U" (uppercase)
        // so no dash; "I" follows "R" (uppercase) so no dash → "uri"
        assert_eq!(super::object_type_to_kebab("URIValue"), "uri");
    }

    #[test]
    fn object_type_to_kebab_interval_value() {
        // "Interval" — "n" is lowercase before "I"... no, it's the start. "I" is
        // uppercase at position 0, so no dash. Result: "interval"
        assert_eq!(super::object_type_to_kebab("IntervalValue"), "interval");
    }

    #[test]
    fn object_type_to_kebab_no_value_suffix() {
        // No "Value" suffix — returned as-is after kebab conversion
        assert_eq!(super::object_type_to_kebab("Geom"), "geom");
    }

    #[test]
    fn map_object_type_known_text_value() {
        use crate::model::ValueType;
        assert_eq!(
            super::map_object_type_to_value_type("TextValue"),
            ValueType::Text
        );
    }

    #[test]
    fn map_object_type_known_list_value() {
        use crate::model::ValueType;
        assert_eq!(
            super::map_object_type_to_value_type("ListValue"),
            ValueType::VocabularyItem
        );
    }

    #[test]
    fn map_object_type_other_geom() {
        use crate::model::ValueType;
        // "GeomValue" is not a named variant → Other("geom")
        assert_eq!(
            super::map_object_type_to_value_type("GeomValue"),
            ValueType::Other("geom".to_string())
        );
    }

    #[test]
    fn map_object_type_other_uri_value() {
        use crate::model::ValueType;
        // "URIValue" is not named (the named variant is "UriValue"); kebab → "uri"
        assert_eq!(
            super::map_object_type_to_value_type("URIValue"),
            ValueType::Other("uri".to_string())
        );
    }

    #[test]
    fn map_object_type_other_geo_name_value() {
        use crate::model::ValueType;
        assert_eq!(
            super::map_object_type_to_value_type("GeoNameValue"),
            ValueType::Other("geo-name".to_string())
        );
    }

    // ---------------------------------------------------------------------------
    // `decode_cardinality` unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn decode_cardinality_owl_cardinality_1() {
        use crate::model::Cardinality;
        let v = serde_json::json!({"owl:cardinality": 1});
        assert_eq!(super::decode_cardinality(&v), Cardinality::One);
    }

    #[test]
    fn decode_cardinality_owl_max_cardinality_1() {
        use crate::model::Cardinality;
        let v = serde_json::json!({"owl:maxCardinality": 1});
        assert_eq!(super::decode_cardinality(&v), Cardinality::ZeroOrOne);
    }

    #[test]
    fn decode_cardinality_owl_min_cardinality_0() {
        use crate::model::Cardinality;
        let v = serde_json::json!({"owl:minCardinality": 0});
        assert_eq!(super::decode_cardinality(&v), Cardinality::ZeroOrMore);
    }

    #[test]
    fn decode_cardinality_owl_min_cardinality_1() {
        use crate::model::Cardinality;
        let v = serde_json::json!({"owl:minCardinality": 1});
        assert_eq!(super::decode_cardinality(&v), Cardinality::OneOrMore);
    }

    #[test]
    fn decode_cardinality_fallback_no_key() {
        use crate::model::Cardinality;
        // No recognized cardinality key → ZeroOrMore (defensive fallback)
        let v = serde_json::json!({});
        assert_eq!(super::decode_cardinality(&v), Cardinality::ZeroOrMore);
    }

    #[test]
    fn decode_cardinality_fallback_owl_cardinality_unexpected_value() {
        use crate::model::Cardinality;
        // owl:cardinality=5 is unexpected → ZeroOrMore
        let v = serde_json::json!({"owl:cardinality": 5});
        assert_eq!(super::decode_cardinality(&v), Cardinality::ZeroOrMore);
    }

    #[test]
    fn decode_cardinality_fallback_owl_max_cardinality_gt1() {
        use crate::model::Cardinality;
        // owl:maxCardinality=2 is not a shape DSP emits (only 1 is expected) →
        // defensive fallback: ZeroOrMore.
        let v = serde_json::json!({"owl:maxCardinality": 2});
        assert_eq!(
            super::decode_cardinality(&v),
            Cardinality::ZeroOrMore,
            "owl:maxCardinality=2 must fall back to ZeroOrMore (defensive fallback)"
        );
    }

    #[test]
    fn decode_cardinality_fallback_owl_min_cardinality_gt1() {
        use crate::model::Cardinality;
        // owl:minCardinality=2 is not a shape DSP emits (only 0 or 1 are expected) →
        // defensive fallback: ZeroOrMore.
        let v = serde_json::json!({"owl:minCardinality": 2});
        assert_eq!(
            super::decode_cardinality(&v),
            Cardinality::ZeroOrMore,
            "owl:minCardinality=2 must fall back to ZeroOrMore (defensive fallback)"
        );
    }

    // ---------------------------------------------------------------------------
    // `detect_representation` unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn detect_representation_still_image() {
        use crate::model::Representation;
        let locals = vec!["hasStillImageFileValue"];
        assert_eq!(
            super::detect_representation(&locals),
            Some(Representation::StillImage)
        );
    }

    #[test]
    fn detect_representation_moving_image() {
        use crate::model::Representation;
        let locals = vec!["hasMovingImageFileValue"];
        assert_eq!(
            super::detect_representation(&locals),
            Some(Representation::MovingImage)
        );
    }

    #[test]
    fn detect_representation_audio() {
        use crate::model::Representation;
        let locals = vec!["hasAudioFileValue"];
        assert_eq!(
            super::detect_representation(&locals),
            Some(Representation::Audio)
        );
    }

    #[test]
    fn detect_representation_none_when_absent() {
        // No file-value property in the list → None
        let locals = vec!["hasTitle", "hasAuthor"];
        assert_eq!(super::detect_representation(&locals), None);
    }

    #[test]
    fn detect_representation_takes_first() {
        use crate::model::Representation;
        // Both still-image and document present → first hit wins
        let locals = vec!["hasDocumentFileValue", "hasStillImageFileValue"];
        assert_eq!(
            super::detect_representation(&locals),
            Some(Representation::Document)
        );
    }

    // ---------------------------------------------------------------------------
    // `is_system_prefix` unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn is_system_prefix_knora_api() {
        assert!(super::is_system_prefix("knora-api"));
    }

    #[test]
    fn is_system_prefix_rdf() {
        assert!(super::is_system_prefix("rdf"));
    }

    #[test]
    fn is_system_prefix_project_prefix_is_not_system() {
        assert!(!super::is_system_prefix("incunabula"));
        assert!(!super::is_system_prefix("beol"));
        assert!(!super::is_system_prefix("biblio"));
    }

    // ---------------------------------------------------------------------------
    // `curie_prefix` unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn curie_prefix_returns_prefix_for_curie() {
        assert_eq!(super::curie_prefix("knora-api:arkUrl"), Some("knora-api"));
        assert_eq!(super::curie_prefix("beol:hasTitle"), Some("beol"));
    }

    #[test]
    fn curie_prefix_returns_none_for_full_iri() {
        // http:// starts with "//" after the colon → not a CURIE prefix
        assert_eq!(
            super::curie_prefix("http://api.dasch.swiss/ontology/0801/beol/v2#hasTitle"),
            None
        );
    }

    #[test]
    fn curie_prefix_returns_none_for_no_colon() {
        assert_eq!(super::curie_prefix("hasTitle"), None);
    }

    // ---------------------------------------------------------------------------
    // Sibling-IRI resolution / self-loop / delimiter unit tests (new)
    // ---------------------------------------------------------------------------

    #[test]
    fn sibling_iri_trim_hash_delimiter() {
        // Namespace ending in '#' → sibling IRI without the '#'
        let namespace = "http://api.dasch.swiss/ontology/0801/biblio/v2#";
        let trimmed = namespace.trim_end_matches(['#', '/']);
        assert_eq!(trimmed, "http://api.dasch.swiss/ontology/0801/biblio/v2");
    }

    #[test]
    fn sibling_iri_trim_slash_delimiter() {
        // Namespace ending in '/' → sibling IRI without the '/'
        let namespace = "http://api.dasch.swiss/ontology/0801/biblio/v2/";
        let trimmed = namespace.trim_end_matches(['#', '/']);
        assert_eq!(trimmed, "http://api.dasch.swiss/ontology/0801/biblio/v2");
    }

    #[test]
    fn sibling_iri_self_loop_detected() {
        // When the sibling IRI (trimmed) equals the queried DM IRI (trimmed) → self-loop
        let data_model_iri = "http://api.dasch.swiss/ontology/0801/beol/v2";
        let namespace = "http://api.dasch.swiss/ontology/0801/beol/v2#";
        let sibling_iri = namespace.trim_end_matches(['#', '/']);
        let queried_trimmed = data_model_iri.trim_end_matches(['#', '/']);
        assert_eq!(sibling_iri, queried_trimmed); // self-loop
    }

    #[test]
    fn sibling_iri_different_ontology_is_not_self_loop() {
        let data_model_iri = "http://api.dasch.swiss/ontology/0801/beol/v2";
        let namespace = "http://api.dasch.swiss/ontology/0801/biblio/v2#";
        let sibling_iri = namespace.trim_end_matches(['#', '/']);
        let queried_trimmed = data_model_iri.trim_end_matches(['#', '/']);
        assert_ne!(sibling_iri, queried_trimmed); // not a self-loop
    }

    #[test]
    fn missing_prefix_in_context_is_skipped() {
        // If a CURIE prefix is not in the @context map, no sibling IRI can be derived.
        let prefixes: HashMap<String, String> = HashMap::new();
        let result = prefixes.get("biblio");
        assert!(result.is_none()); // caller skips and warns
    }

    // ---------------------------------------------------------------------------
    // `derive_access` unit tests (D1, Facet B)
    // ---------------------------------------------------------------------------

    #[test]
    fn derive_access_rv() {
        assert_eq!(
            super::derive_access("RV"),
            Some(super::ResourceAccess::RestrictedView)
        );
    }

    #[test]
    fn derive_access_v() {
        assert_eq!(super::derive_access("V"), Some(super::ResourceAccess::View));
    }

    #[test]
    fn derive_access_m() {
        assert_eq!(super::derive_access("M"), Some(super::ResourceAccess::Edit));
    }

    #[test]
    fn derive_access_d() {
        assert_eq!(
            super::derive_access("D"),
            Some(super::ResourceAccess::Delete)
        );
    }

    #[test]
    fn derive_access_cr() {
        assert_eq!(
            super::derive_access("CR"),
            Some(super::ResourceAccess::Manage)
        );
    }

    #[test]
    fn derive_access_unknown_is_none() {
        assert_eq!(super::derive_access("XYZ"), None);
    }

    #[test]
    fn derive_access_empty_is_none() {
        assert_eq!(super::derive_access(""), None);
    }

    // ---------------------------------------------------------------------------
    // `derive_visibility` unit tests (D1 ACL parse algorithm)
    // ---------------------------------------------------------------------------

    #[test]
    fn derive_visibility_public_when_unknown_user_has_view() {
        // Real ACL from incunabula: UnknownUser gets V → public.
        let acl = "CR knora-admin:Creator,knora-admin:ProjectAdmin|V knora-admin:KnownUser,knora-admin:UnknownUser";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::Public)
        );
    }

    #[test]
    fn derive_visibility_public_when_unknown_user_has_cr() {
        // UnknownUser granted CR (>= V) → public.
        let acl = "CR knora-admin:UnknownUser";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::Public)
        );
    }

    #[test]
    fn derive_visibility_public_restricted_when_unknown_user_has_rv() {
        // UnknownUser granted exactly RV → public (restricted view).
        let acl = "RV knora-admin:UnknownUser|CR knora-admin:ProjectAdmin";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::PublicRestricted)
        );
    }

    #[test]
    fn derive_visibility_logged_in_when_known_user_has_rv_unknown_absent() {
        // UnknownUser absent; KnownUser gets RV → logged-in users.
        let acl = "RV knora-admin:KnownUser|CR knora-admin:ProjectAdmin";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::LoggedInUsers)
        );
    }

    #[test]
    fn derive_visibility_logged_in_when_known_user_has_v() {
        // KnownUser ≥ RV (has V) and UnknownUser absent → logged-in users.
        let acl = "V knora-admin:KnownUser|CR knora-admin:ProjectAdmin";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::LoggedInUsers)
        );
    }

    #[test]
    fn derive_visibility_project_members_when_neither_world_group_granted() {
        // Only project-specific groups in ACL → project members only.
        let acl = "CR knora-admin:Creator,knora-admin:ProjectAdmin|M knora-admin:ProjectMember";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::ProjectMembers)
        );
    }

    #[test]
    fn derive_visibility_empty_string_is_none() {
        assert_eq!(super::derive_visibility(""), None);
    }

    #[test]
    fn derive_visibility_whitespace_only_is_none() {
        assert_eq!(super::derive_visibility("   "), None);
    }

    #[test]
    fn derive_visibility_malformed_entry_without_space_is_skipped() {
        // "CRMALFORMED" has no space — skip it; the rest of the ACL may still parse.
        let acl = "CRMALFORMED|CR knora-admin:ProjectAdmin";
        // Only valid entry is CR ProjectAdmin; neither world group granted → ProjectMembers.
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::ProjectMembers)
        );
    }

    #[test]
    fn derive_visibility_unknown_code_ranks_zero_no_implicit_grant() {
        // Unknown code "BOGUS" ranks 0 — even for UnknownUser, no implicit grant.
        let acl = "BOGUS knora-admin:UnknownUser|CR knora-admin:ProjectAdmin";
        // UnknownUser rank = 0 (< RV); KnownUser rank = 0 → ProjectMembers.
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::ProjectMembers)
        );
    }

    #[test]
    fn derive_visibility_same_group_two_entries_max_wins() {
        // UnknownUser appears in two entries: RV and V. Max is V → public.
        let acl = "RV knora-admin:UnknownUser|V knora-admin:UnknownUser";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::Public)
        );
    }

    #[test]
    fn derive_visibility_both_world_groups_unknown_user_decides() {
        // Both UnknownUser (V) and KnownUser (CR) present — UnknownUser's grant decides.
        // UnknownUser ≥ V → public (not logged-in users, even though KnownUser is higher).
        let acl = "V knora-admin:UnknownUser|CR knora-admin:KnownUser";
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::Public)
        );
    }

    #[test]
    fn derive_visibility_super_unknown_user_does_not_match() {
        // A hypothetical "SuperUnknownUser" must NOT be treated as UnknownUser
        // (exact local-name match only, never substring contains).
        let acl = "CR knora-admin:SuperUnknownUser|CR knora-admin:ProjectAdmin";
        // SuperUnknownUser doesn't match → neither world group → ProjectMembers.
        assert_eq!(
            super::derive_visibility(acl),
            Some(super::ResourceVisibility::ProjectMembers)
        );
    }

    #[test]
    fn derive_visibility_all_malformed_entries_no_space_returns_none() {
        // Every entry lacks a space separator (no "<CODE> <group>" shape).
        // `parsed_any` stays false → the function must return None, not
        // fall through to a default visibility.
        let acl = "NOSPACE|ALSONOSPACE|STILLNOSPACE";
        assert_eq!(
            super::derive_visibility(acl),
            None,
            "all-malformed ACL (no space in any entry) must return None"
        );
    }

    // ---------------------------------------------------------------------------
    // Value-type parse matrix unit tests (pure — no HTTP)
    // ---------------------------------------------------------------------------

    use crate::model::ValueType;
    use crate::model::resource::{DatePoint, DateValue, FileValue, ValueContent};

    // ── TextValue ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_text_plain() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Hello world"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Text("Hello world".into()));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_text_standoff_xml_stripped() {
        // textValueAsXml present → html_to_text is applied (standoff path).
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:textValueAsXml": "<p>Hello <b>world</b></p>",
            "knora-api:valueAsString": "This is ignored when xml present"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        // html_to_text strips tags; exact output depends on the util helper.
        assert!(matches!(content, ValueContent::Text(_)));
        assert!(!is_link);
        if let ValueContent::Text(s) = content {
            // Must not contain raw HTML tags.
            assert!(!s.contains('<'), "no raw tags: {s:?}");
            assert!(s.contains("Hello"), "text retained: {s:?}");
        }
    }

    // ── IntValue ─────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_integer() {
        let obj = serde_json::json!({
            "@type": "knora-api:IntValue",
            "knora-api:intValueAsInt": 42
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Integer(42));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_integer_negative() {
        let obj = serde_json::json!({
            "@type": "knora-api:IntValue",
            "knora-api:intValueAsInt": -7
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Integer(-7));
    }

    // ── DecimalValue ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_decimal_object_form() {
        // `{"@value": "3.14159", "@type": "xsd:decimal"}` form.
        let obj = serde_json::json!({
            "@type": "knora-api:DecimalValue",
            "knora-api:decimalValueAsDecimal": {"@value": "3.14159", "@type": "xsd:decimal"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Decimal("3.14159".into()));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_decimal_bare_string_form() {
        let obj = serde_json::json!({
            "@type": "knora-api:DecimalValue",
            "knora-api:decimalValueAsDecimal": "2.71828"
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Decimal("2.71828".into()));
    }

    // ── BooleanValue ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_boolean_true() {
        let obj = serde_json::json!({
            "@type": "knora-api:BooleanValue",
            "knora-api:booleanValueAsBoolean": true
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Boolean(true));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_boolean_false() {
        let obj = serde_json::json!({
            "@type": "knora-api:BooleanValue",
            "knora-api:booleanValueAsBoolean": false
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Boolean(false));
    }

    // ── DateValue ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_date_single_point() {
        // start == end → single-point date (year-only, CE).
        let obj = serde_json::json!({
            "@type": "knora-api:DateValue",
            "knora-api:dateValueHasCalendar": "GREGORIAN",
            "knora-api:dateValueHasStartYear": 1489,
            "knora-api:dateValueHasStartEra": "CE",
            "knora-api:dateValueHasEndYear": 1489,
            "knora-api:dateValueHasEndEra": "CE"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(!is_link);
        let expected = ValueContent::Date(DateValue {
            calendar: "GREGORIAN".into(),
            start: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
        });
        assert_eq!(content, expected);
    }

    #[test]
    fn parse_value_date_range() {
        // start != end → range.
        let obj = serde_json::json!({
            "@type": "knora-api:DateValue",
            "knora-api:dateValueHasCalendar": "GREGORIAN",
            "knora-api:dateValueHasStartYear": 1489,
            "knora-api:dateValueHasStartEra": "CE",
            "knora-api:dateValueHasEndYear": 1490,
            "knora-api:dateValueHasEndEra": "CE"
        });
        let (content, _) = super::parse_value_content(&obj);
        if let ValueContent::Date(dv) = content {
            assert_eq!(dv.start.year, Some(1489));
            assert_eq!(dv.end.year, Some(1490));
            assert_ne!(dv.start, dv.end, "range: start != end");
        } else {
            panic!("expected DateValue, got {content:?}");
        }
    }

    #[test]
    fn parse_value_date_full_day_precision() {
        // Year + month + day + era (full Julian day).
        let obj = serde_json::json!({
            "@type": "knora-api:DateValue",
            "knora-api:dateValueHasCalendar": "JULIAN",
            "knora-api:dateValueHasStartYear": 1456,
            "knora-api:dateValueHasStartMonth": 3,
            "knora-api:dateValueHasStartDay": 14,
            "knora-api:dateValueHasStartEra": "CE",
            "knora-api:dateValueHasEndYear": 1456,
            "knora-api:dateValueHasEndMonth": 3,
            "knora-api:dateValueHasEndDay": 14,
            "knora-api:dateValueHasEndEra": "CE"
        });
        let (content, _) = super::parse_value_content(&obj);
        if let ValueContent::Date(dv) = content {
            assert_eq!(dv.calendar, "JULIAN");
            assert_eq!(dv.start.month, Some(3));
            assert_eq!(dv.start.day, Some(14));
        } else {
            panic!("expected DateValue, got {content:?}");
        }
    }

    #[test]
    fn parse_value_date_no_year_falls_back_to_raw() {
        // A date object with no year on either point → raw fallback.
        let obj = serde_json::json!({
            "@type": "knora-api:DateValue",
            "knora-api:dateValueHasCalendar": "GREGORIAN",
            "knora-api:valueAsString": "some date"
        });
        let (content, _) = super::parse_value_content(&obj);
        assert!(
            matches!(content, ValueContent::Raw { value_type, .. } if value_type == "date"),
            "missing years must degrade to Raw date"
        );
    }

    // ── TimeValue ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_time() {
        let obj = serde_json::json!({
            "@type": "knora-api:TimeValue",
            "knora-api:timeValueAsTimeStamp": {"@value": "2021-01-01T12:00:00Z", "@type": "xsd:dateTimeStamp"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Time("2021-01-01T12:00:00Z".into()));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_time_bare_string() {
        let obj = serde_json::json!({
            "@type": "knora-api:TimeValue",
            "knora-api:timeValueAsTimeStamp": "2022-06-01T00:00:00Z"
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Time("2022-06-01T00:00:00Z".into()));
    }

    // ── UriValue ─────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_uri() {
        let obj = serde_json::json!({
            "@type": "knora-api:UriValue",
            "knora-api:uriValueAsUri": {"@value": "https://example.com", "@type": "xsd:anyURI"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Uri("https://example.com".into()));
        assert!(!is_link);
    }

    // ── ColorValue ───────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_color() {
        let obj = serde_json::json!({
            "@type": "knora-api:ColorValue",
            "knora-api:colorValueAsColor": "#ff0000"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Color("#ff0000".into()));
        assert!(!is_link);
    }

    // ── GeonameValue ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_geoname() {
        let obj = serde_json::json!({
            "@type": "knora-api:GeonameValue",
            "knora-api:geonameValueAsGeonameCode": "2661552"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(content, ValueContent::Geoname("2661552".into()));
        assert!(!is_link);
    }

    // ── ListValue ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_vocabulary_item() {
        let obj = serde_json::json!({
            "@type": "knora-api:ListValue",
            "knora-api:listValueAsListNode": {"@id": "http://rdfh.ch/lists/0001/node1"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert_eq!(
            content,
            ValueContent::VocabularyItem {
                node_iri: "http://rdfh.ch/lists/0001/node1".into(),
                label: None, // resolved later
            }
        );
        assert!(!is_link);
    }

    // ── LinkValue ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_link_with_embedded_target() {
        let obj = serde_json::json!({
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTarget": {
                "@id": "http://rdfh.ch/0803/res1",
                "@type": "incunabula:Book",
                "rdfs:label": "Incunabula Book 1"
            }
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(is_link, "LinkValue must set is_link=true");
        assert_eq!(
            content,
            ValueContent::Link {
                target_iri: "http://rdfh.ch/0803/res1".into(),
                target_label: Some("Incunabula Book 1".into()),
            }
        );
    }

    #[test]
    fn parse_value_link_with_target_iri_only() {
        // `linkValueHasTargetIri` only, no embedded target object.
        let obj = serde_json::json!({
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTargetIri": {"@id": "http://rdfh.ch/0803/res2"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(is_link);
        assert_eq!(
            content,
            ValueContent::Link {
                target_iri: "http://rdfh.ch/0803/res2".into(),
                target_label: None,
            }
        );
    }

    // ── StillImageFileValue ───────────────────────────────────────────────────────

    #[test]
    fn parse_value_still_image_file() {
        let obj = serde_json::json!({
            "@type": "knora-api:StillImageFileValue",
            "knora-api:fileValueHasFilename": "image.jp2",
            "knora-api:fileValueAsUrl": {"@value": "https://iiif.example.com/image.jp2/full/max/0/default.jpg"},
            "knora-api:stillImageFileValueHasDimX": 1200,
            "knora-api:stillImageFileValueHasDimY": 800
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(!is_link);
        assert_eq!(
            content,
            ValueContent::File(FileValue {
                value_type: ValueType::StillImage,
                filename: "image.jp2".into(),
                url: "https://iiif.example.com/image.jp2/full/max/0/default.jpg".into(),
                width: Some(1200),
                height: Some(800),
            })
        );
    }

    #[test]
    fn parse_value_still_image_external_file_value() {
        // StillImageExternalFileValue variant (ADR-0013: StillImage* → still-image).
        let obj = serde_json::json!({
            "@type": "knora-api:StillImageExternalFileValue",
            "knora-api:fileValueHasFilename": "external.jpg",
            "knora-api:fileValueAsUrl": {"@value": "https://iiif.external.com/image.jpg"}
        });
        let (content, _) = super::parse_value_content(&obj);
        if let ValueContent::File(fv) = content {
            assert_eq!(
                fv.value_type,
                ValueType::StillImage,
                "StillImageExternal* → StillImage"
            );
        } else {
            panic!("expected File, got {content:?}");
        }
    }

    // ── MovingImageFileValue ──────────────────────────────────────────────────────

    #[test]
    fn parse_value_moving_image_file() {
        let obj = serde_json::json!({
            "@type": "knora-api:MovingImageFileValue",
            "knora-api:fileValueHasFilename": "video.mp4",
            "knora-api:fileValueAsUrl": {"@value": "https://example.com/video.mp4"}
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(!is_link);
        assert_eq!(
            content,
            ValueContent::File(FileValue {
                value_type: ValueType::MovingImage,
                filename: "video.mp4".into(),
                url: "https://example.com/video.mp4".into(),
                width: None,
                height: None,
            })
        );
    }

    // ── AudioFileValue ────────────────────────────────────────────────────────────

    #[test]
    fn parse_value_audio_file() {
        let obj = serde_json::json!({
            "@type": "knora-api:AudioFileValue",
            "knora-api:fileValueHasFilename": "sound.wav",
            "knora-api:fileValueAsUrl": {"@value": "https://example.com/sound.wav"}
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(
            content,
            ValueContent::File(FileValue {
                value_type: ValueType::Audio,
                filename: "sound.wav".into(),
                url: "https://example.com/sound.wav".into(),
                width: None,
                height: None,
            })
        );
    }

    // ── DocumentFileValue ─────────────────────────────────────────────────────────

    #[test]
    fn parse_value_document_file() {
        let obj = serde_json::json!({
            "@type": "knora-api:DocumentFileValue",
            "knora-api:fileValueHasFilename": "doc.pdf",
            "knora-api:fileValueAsUrl": {"@value": "https://example.com/doc.pdf"}
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(
            content,
            ValueContent::File(FileValue {
                value_type: ValueType::Document,
                filename: "doc.pdf".into(),
                url: "https://example.com/doc.pdf".into(),
                width: None,
                height: None,
            })
        );
    }

    // ── ArchiveFileValue ──────────────────────────────────────────────────────────

    #[test]
    fn parse_value_archive_file() {
        let obj = serde_json::json!({
            "@type": "knora-api:ArchiveFileValue",
            "knora-api:fileValueHasFilename": "data.zip",
            "knora-api:fileValueAsUrl": {"@value": "https://example.com/data.zip"}
        });
        let (content, _) = super::parse_value_content(&obj);
        assert_eq!(
            content,
            ValueContent::File(FileValue {
                value_type: ValueType::Archive,
                filename: "data.zip".into(),
                url: "https://example.com/data.zip".into(),
                width: None,
                height: None,
            })
        );
    }

    // ── TextFileValue (maps to Document per ADR-0013) ─────────────────────────────

    #[test]
    fn parse_value_text_file_value_maps_to_document() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextFileValue",
            "knora-api:fileValueHasFilename": "text.txt",
            "knora-api:fileValueAsUrl": {"@value": "https://example.com/text.txt"}
        });
        let (content, _) = super::parse_value_content(&obj);
        if let ValueContent::File(fv) = content {
            assert_eq!(
                fv.value_type,
                ValueType::Document,
                "TextFileValue → Document"
            );
        } else {
            panic!("expected File, got {content:?}");
        }
    }

    // ── IntervalValue (raw fallback) ──────────────────────────────────────────────

    #[test]
    fn parse_value_interval_raw_fallback() {
        let obj = serde_json::json!({
            "@type": "knora-api:IntervalValue",
            "knora-api:intervalValueHasStart": {"@value": "0.0", "@type": "xsd:decimal"},
            "knora-api:intervalValueHasEnd": {"@value": "10.5", "@type": "xsd:decimal"},
            "knora-api:valueAsString": "0.0 - 10.5"
        });
        let (content, is_link) = super::parse_value_content(&obj);
        assert!(!is_link);
        assert!(
            matches!(content, ValueContent::Raw { ref value_type, .. } if value_type == "interval"),
            "IntervalValue must degrade to Raw with token 'interval'"
        );
        if let ValueContent::Raw { text, .. } = content {
            assert_eq!(text, "0.0 - 10.5");
        }
    }

    #[test]
    fn parse_value_geom_raw_fallback() {
        let obj = serde_json::json!({
            "@type": "knora-api:GeomValue",
            "knora-api:geometryValueAsGeometry": "POINT(1 2)"
        });
        let (content, _) = super::parse_value_content(&obj);
        assert!(
            matches!(content, ValueContent::Raw { value_type, .. } if value_type == "geom"),
            "GeomValue must degrade to Raw with token 'geom'"
        );
    }

    // ── Value wrapper: per-value comment (`knora-api:valueHasComment`) ───────────

    #[test]
    fn parse_value_with_comment() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Hello world",
            "knora-api:valueHasComment": "reading uncertain"
        });
        let (value, is_link) = super::parse_value(&obj);
        assert_eq!(value.content, ValueContent::Text("Hello world".into()));
        assert_eq!(value.comment.as_deref(), Some("reading uncertain"));
        assert!(!is_link);
    }

    #[test]
    fn parse_value_without_comment() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Hello world"
        });
        let (value, is_link) = super::parse_value(&obj);
        assert_eq!(value.content, ValueContent::Text("Hello world".into()));
        assert_eq!(value.comment, None);
        assert!(!is_link);
    }

    #[test]
    fn parse_value_with_empty_comment() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Hello world",
            "knora-api:valueHasComment": ""
        });
        let (value, is_link) = super::parse_value(&obj);
        assert_eq!(value.content, ValueContent::Text("Hello world".into()));
        assert_eq!(value.comment, None);
        assert!(!is_link);
    }

    // ── Field-name derivation (D3) ────────────────────────────────────────────────

    #[test]
    fn parse_value_link_is_link_true() {
        // LinkValue → is_link = true (used by caller to strip "Value" suffix).
        let obj = serde_json::json!({
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTargetIri": {"@id": "http://rdfh.ch/0803/res1"}
        });
        let (_, is_link) = super::parse_value_content(&obj);
        assert!(
            is_link,
            "LinkValue must report is_link=true for name derivation"
        );
    }

    #[test]
    fn field_name_link_strips_value_suffix() {
        // A LinkValue object whose key ends in `Value` → is_link=true → suffix stripped.
        // Uses the real `parse_value` path to determine is_link, then applies the
        // same name-derivation logic the production code uses (D3).
        let key = "incunabula:isPartOfBookValue";
        let link_obj = serde_json::json!({
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTargetIri": {"@id": "http://rdfh.ch/0803/res1"}
        });
        let (_, is_link) = super::parse_value_content(&link_obj);
        assert!(
            is_link,
            "LinkValue must report is_link=true for name derivation"
        );

        let raw_name = super::local_name(key).to_string();
        // is_link = true → strip "Value" suffix (same logic as production code).
        let name = if is_link {
            raw_name
                .strip_suffix("Value")
                .unwrap_or(&raw_name)
                .to_string()
        } else {
            raw_name
        };
        assert_eq!(name, "isPartOfBook");
    }

    #[test]
    fn field_name_non_link_does_not_strip_value_suffix() {
        // A TextValue object whose key ends in `Value` → is_link=false → suffix KEPT.
        // Uses the real `parse_value` path (not an inline re-implementation) to
        // determine is_link, then confirms the production name-derivation preserves
        // the trailing "Value" (D3: only link-typed fields are stripped).
        let key = "incunabula:hasAValue";
        let text_obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "some text"
        });
        let (_, is_link) = super::parse_value_content(&text_obj);
        assert!(!is_link, "TextValue must report is_link=false");

        let raw_name = super::local_name(key).to_string();
        // is_link = false → no stripping (same logic as production code).
        let name = if is_link {
            raw_name
                .strip_suffix("Value")
                .unwrap_or(&raw_name)
                .to_string()
        } else {
            raw_name
        };
        assert_eq!(
            name, "hasAValue",
            "non-link ending in Value must NOT be stripped; is_link={is_link}"
        );
    }

    // ── Field / non-field discrimination (ADR-0013) ───────────────────────────────

    #[test]
    fn has_value_class_type_rejects_xsd_any_uri() {
        // `versionArkUrl` has @type `xsd:anyURI` — NOT a knora-api *Value → excluded.
        let obj = serde_json::json!({
            "@value": "http://ark.dasch.swiss/ark:/…",
            "@type": "xsd:anyURI"
        });
        assert!(
            !super::has_value_class_type(&obj),
            "xsd:anyURI must not pass the value-class test"
        );
    }

    #[test]
    fn has_value_class_type_rejects_scalar() {
        // Bare string value → not an object → not a value field.
        let obj = serde_json::json!("just a string");
        assert!(!super::has_value_class_type(&obj));
    }

    #[test]
    fn has_value_class_type_accepts_text_value() {
        let obj = serde_json::json!({
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "hello"
        });
        assert!(super::has_value_class_type(&obj));
    }

    #[test]
    fn has_value_class_type_accepts_still_image_file_value() {
        let obj = serde_json::json!({
            "@type": "knora-api:StillImageFileValue",
            "knora-api:fileValueHasFilename": "img.jp2"
        });
        assert!(super::has_value_class_type(&obj));
    }

    // ── build_prefix_map ──────────────────────────────────────────────────────────

    #[test]
    fn build_prefix_map_string_entries_only() {
        let ctx = Some(serde_json::json!({
            "incunabula": "http://api.dasch.swiss/ontology/0803/incunabula/v2#",
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            // Object-valued entry — must be skipped.
            "someterm": {"@id": "http://example.com/term", "@type": "@id"}
        }));
        let map = super::build_prefix_map(&ctx);
        assert_eq!(
            map.get("incunabula").map(String::as_str),
            Some("http://api.dasch.swiss/ontology/0803/incunabula/v2#")
        );
        assert_eq!(
            map.get("knora-api").map(String::as_str),
            Some("http://api.knora.org/ontology/knora-api/v2#")
        );
        assert!(
            !map.contains_key("someterm"),
            "object-valued entry must be skipped"
        );
    }

    #[test]
    fn build_prefix_map_empty_when_no_context() {
        let map = super::build_prefix_map(&None);
        assert!(map.is_empty());
    }

    // ── compact_value_text (raw fallback) ─────────────────────────────────────────

    #[test]
    fn compact_value_text_excludes_meta_keys() {
        let obj = serde_json::json!({
            "@id": "http://rdfh.ch/0803/val1",
            "@type": "knora-api:GeomValue",
            "knora-api:geometryValueAsGeometry": "POINT(1 2)"
        });
        let text = super::compact_value_text(&obj);
        // Must include the geometry key, not the metadata keys.
        assert!(
            text.contains("geometryValueAsGeometry"),
            "geometry key present: {text}"
        );
        assert!(!text.contains("@id"), "@id must be excluded: {text}");
        assert!(!text.contains("@type"), "@type must be excluded: {text}");
    }

    #[test]
    fn compact_value_text_all_meta_yields_empty() {
        let obj = serde_json::json!({
            "@id": "http://rdfh.ch/0803/val1",
            "@type": "knora-api:IntervalValue"
        });
        let text = super::compact_value_text(&obj);
        assert!(
            text.is_empty(),
            "all-meta object must yield empty string: {text:?}"
        );
    }

    // ── vocabulary DTO parsing / conversion (plan 034, Step 2) ─────────────────────

    #[test]
    fn list_get_response_root_shape_parses_as_root_variant() {
        // Shape from the Verified API facts: `{"type":"...","list":{"listinfo":{...},"children":[...]}}`.
        // Children deliberately out of order to exercise the defensive sort.
        let json = serde_json::json!({
            "type": "ListGetResponseADM",
            "list": {
                "listinfo": {
                    "id": "http://rdfh.ch/lists/0001/root",
                    "projectIri": "http://rdfh.ch/projects/0001",
                    "name": "root-name",
                    "labels": [
                        {"value": "Root EN", "language": "en"},
                        {"value": "Root DE", "language": "de"}
                    ],
                    "comments": []
                },
                "children": [
                    {"id": "n2", "name": "n2", "labels": [], "comments": [], "position": 1, "children": []},
                    {"id": "n1", "name": "n1", "labels": [], "comments": [], "position": 0, "children": [
                        {"id": "n1a", "name": "n1a", "labels": [], "comments": [], "position": 0, "children": []}
                    ]}
                ]
            }
        });

        let parsed: ListGetResponseDto =
            serde_json::from_value(json).expect("root shape must parse");
        let root = match parsed {
            ListGetResponseDto::Root(root) => root,
            ListGetResponseDto::Node(_) => panic!("expected Root variant, got Node"),
        };

        let tree = build_vocabulary_tree(root.list, None);
        assert_eq!(tree.root.iri, "http://rdfh.ch/lists/0001/root");
        assert_eq!(tree.root.name.as_deref(), Some("root-name"));
        assert_eq!(tree.root.labels.len(), 2, "both languages kept (D4)");
        assert_eq!(tree.project_iri, "http://rdfh.ch/projects/0001");
        assert_eq!(tree.requested_node, None);

        // Defensive sort by position: n1 (position 0) before n2 (position 1),
        // even though the JSON listed n2 first.
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].header.iri, "n1");
        assert_eq!(tree.children[1].header.iri, "n2");
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].header.iri, "n1a");
    }

    #[test]
    fn list_get_response_node_shape_parses_as_node_variant_and_extracts_has_root_node() {
        // Shape from the Verified API facts: `{"type":"...","node":{"nodeinfo":{...,"hasRootNode"},"children":[...]}}`.
        let json = serde_json::json!({
            "type": "ListNodeGetResponseADM",
            "node": {
                "nodeinfo": {
                    "id": "http://rdfh.ch/lists/0001/n1",
                    "name": "n1",
                    "labels": [{"value": "N1", "language": "en"}],
                    "comments": [],
                    "position": 0,
                    "hasRootNode": "http://rdfh.ch/lists/0001/root"
                },
                "children": []
            }
        });

        let parsed: ListGetResponseDto =
            serde_json::from_value(json).expect("node shape must parse");
        match parsed {
            ListGetResponseDto::Node(node) => {
                assert_eq!(
                    node.node.nodeinfo.has_root_node,
                    "http://rdfh.ch/lists/0001/root"
                );
            }
            ListGetResponseDto::Root(_) => panic!("expected Node variant, got Root"),
        }
    }

    #[test]
    fn list_get_response_neither_key_fails_parse() {
        // Neither `list` nor `node` present — must fail parse loudly (the
        // caller maps this to `Diagnostic::ServerError`), not silently pick a
        // default variant.
        let json = serde_json::json!({"type": "SomethingUnexpected", "foo": "bar"});
        let parsed = serde_json::from_value::<ListGetResponseDto>(json);
        assert!(
            parsed.is_err(),
            "a response with neither `list` nor `node` must fail to parse"
        );
    }

    #[test]
    fn into_localized_texts_keeps_all_languages_no_filtering() {
        // D4: no preferred-language collapsing anywhere in this crate.
        let dtos = vec![
            ListLabelDto {
                value: "a".into(),
                language: Some("en".into()),
            },
            ListLabelDto {
                value: "b".into(),
                language: None,
            },
        ];
        let texts = into_localized_texts(dtos);
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0].value, "a");
        assert_eq!(texts[0].language.as_deref(), Some("en"));
        assert_eq!(texts[1].value, "b");
        assert_eq!(texts[1].language, None);
    }

    #[test]
    fn convert_list_nodes_sorts_and_nests_out_of_order_input() {
        // Deliberately out of order at every level, to exercise the
        // defensive-sort + iterative-nesting logic together.
        let leaf_2b1 = ListNodeDto {
            id: "2b1".into(),
            name: None,
            labels: vec![],
            comments: vec![],
            position: 0,
            children: vec![],
        };
        let node_2b = ListNodeDto {
            id: "2b".into(),
            name: None,
            labels: vec![],
            comments: vec![],
            position: 1,
            children: vec![leaf_2b1],
        };
        let node_2a = ListNodeDto {
            id: "2a".into(),
            name: None,
            labels: vec![],
            comments: vec![],
            position: 0,
            children: vec![],
        };
        // node_2's children listed out of position order (2b before 2a).
        let node_2 = ListNodeDto {
            id: "2".into(),
            name: None,
            labels: vec![],
            comments: vec![],
            position: 1,
            children: vec![node_2b, node_2a],
        };
        let node_1 = ListNodeDto {
            id: "1".into(),
            name: None,
            labels: vec![],
            comments: vec![],
            position: 0,
            children: vec![],
        };
        // Top level listed out of position order too (node_2 before node_1).
        let converted = convert_list_nodes(vec![node_2, node_1]);

        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].header.iri, "1");
        assert_eq!(converted[0].position, 0);
        assert_eq!(converted[1].header.iri, "2");
        assert_eq!(converted[1].position, 1);

        let node2_children = &converted[1].children;
        assert_eq!(node2_children.len(), 2);
        assert_eq!(node2_children[0].header.iri, "2a");
        assert_eq!(node2_children[1].header.iri, "2b");
        assert_eq!(node2_children[1].children.len(), 1);
        assert_eq!(node2_children[1].children[0].header.iri, "2b1");
    }

    #[test]
    fn build_vocabulary_tree_sets_requested_node_when_provided() {
        let list = ListRootDto {
            listinfo: ListInfoDto {
                id: "root".into(),
                project_iri: "proj".into(),
                name: Some("Root".into()),
                labels: vec![],
                comments: vec![],
            },
            children: vec![],
        };
        let tree = build_vocabulary_tree(list, Some("node-iri".into()));
        assert_eq!(tree.requested_node.as_deref(), Some("node-iri"));
        assert_eq!(tree.root.iri, "root");
        assert_eq!(tree.project_iri, "proj");
        assert!(tree.children.is_empty());
    }
}
