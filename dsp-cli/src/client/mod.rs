//! Client layer (3a of ADR-0008) — the `DspClient` trait and its HTTP impl.
//!
//! The trait is the primary test seam (T2 from ADR-0009): production wires
//! up the HTTP impl, action tests wire up `MockDspClient`. Translation
//! between DSP-API vocabulary (ontology, class, property) and dsp-cli
//! vocabulary (data-model, resource-type, field) happens here and *only*
//! here — see `docs/dev/domain-language.md`.

mod builtins;
pub mod http;
pub(crate) mod jwt;
pub mod sparql;

pub(crate) use builtins::builtin_data_models;
pub(crate) use builtins::builtin_resource_types;

use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::model::auth::LoginResponse;
use crate::model::{
    CreateDumpOutcome, DataModel, DataModelDetail, DataModelStructure, DumpTask, Project,
    ProjectDetail, ProjectRef, ResourceDetail, ResourcePage, ResourceTypeDetail, Vocabulary,
    VocabularyTree,
};
use sparql::SparqlResponse;

/// The `DspClient` trait. Methods are added as commands need them.
pub trait DspClient {
    /// Authenticate against the DSP-API at `server` with the given credentials.
    ///
    /// Returns a [`LoginResponse`] with the token, resolved user identity, and
    /// optional expiry extracted from the JWT. The caller owns cache persistence.
    fn login(&self, server: &str, user: &str, password: &str) -> Result<LoginResponse, Diagnostic>;

    /// Resolve a project identifier (IRI, shortcode, or shortname) on `server`
    /// to a [`ProjectRef`].
    ///
    /// `project` may be an HTTP(S) IRI, a 4-hex-digit shortcode, or a shortname.
    /// The classifier logic (IRI vs shortcode vs shortname) and URL encoding live
    /// in the HTTP impl. This endpoint is public — no bearer token required.
    fn resolve_project(&self, server: &str, project: &str) -> Result<ProjectRef, Diagnostic>;

    /// Trigger a new server-side project dump on `server` for the project
    /// identified by `project_iri`.
    ///
    /// `project_iri` is URL-encoded inside the implementation before being
    /// inserted into the request URL. `dump_id` values returned from this call
    /// are URL-safe (base64url) and inserted verbatim in subsequent requests.
    ///
    /// Returns a [`CreateDumpOutcome`]:
    /// - `Created(task)` — a fresh dump was triggered; `task.status` is `InProgress`.
    /// - `Exists { id }` — the server reports an existing dump via a conflict response;
    ///   `id` is the existing dump's server-assigned identifier, and the existing dump
    ///   belongs to the same project that was requested. The action decides what to do
    ///   with it (adopt / replace / error).
    /// - `ExistsForOtherProject { id, project_iri }` — an existing dump belongs to a
    ///   **different** project; the DSP-API holds one dump server-wide, and the slot is
    ///   occupied by `project_iri`'s dump. The action must never silently use or destroy
    ///   this dump on behalf of the requested project.
    ///
    /// Other error conditions (auth, not-found, network) propagate as `Err(Diagnostic)`.
    fn create_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        skip_assets: bool,
        token: &str,
    ) -> Result<CreateDumpOutcome, Diagnostic>;

    /// Fetch the current status of an ongoing or completed project dump.
    ///
    /// `dump_id` is URL-safe (base64url) and inserted verbatim into the request
    /// URL. `project_iri` is URL-encoded inside the implementation.
    ///
    /// Returns a [`DumpTask`] with the current `status`. The action poll loop
    /// calls this repeatedly until the status is `Completed` or `Failed`.
    fn get_project_dump_status(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
    ) -> Result<DumpTask, Diagnostic>;

    /// Stream the completed dump archive into `dest`.
    ///
    /// `dump_id` is URL-safe and inserted verbatim. `project_iri` is URL-encoded
    /// inside the implementation. The HTTP impl uses a client without a read
    /// timeout (but retains the connect timeout) so large archives do not time out
    /// mid-stream.
    ///
    /// Returns the number of bytes written on success. The action owns the output
    /// filename — `Content-Disposition` from the server is intentionally ignored.
    fn download_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
        dest: &mut dyn std::io::Write,
    ) -> Result<u64, Diagnostic>;

    /// Delete the server-side dump identified by `dump_id`.
    ///
    /// `dump_id` is URL-safe and inserted verbatim. `project_iri` is URL-encoded
    /// inside the implementation.
    ///
    /// Returns `Ok(())` on success. Fails with `Conflict` if the dump is still
    /// being produced and cannot yet be deleted.
    fn delete_project_dump(
        &self,
        server: &str,
        project_iri: &str,
        dump_id: &str,
        token: &str,
    ) -> Result<(), Diagnostic>;

    /// List all projects on the server.
    ///
    /// Public endpoint; `token` is sent as a bearer when `Some` so an
    /// authenticated caller sees their full set, but a missing token is not an
    /// error. Returns projects in server order (the action layer sorts).
    fn list_projects(&self, server: &str, token: Option<&str>) -> Result<Vec<Project>, Diagnostic>;

    /// Fetch the full detail of a single project (for `project describe`).
    ///
    /// Auth is optional (project metadata is public, ADR-0007); a token is sent
    /// when present, mirroring `list_projects`. `project` may be an IRI,
    /// 4-hex-digit shortcode, or shortname — the classifier logic lives in the
    /// HTTP impl.
    fn describe_project(
        &self,
        server: &str,
        project: &str,
        token: Option<&str>,
    ) -> Result<ProjectDetail, Diagnostic>;

    /// List a project's own data-models (DSP-API "ontologies") via
    /// `GET /v2/ontologies/metadata/{project_iri}`.
    ///
    /// `project_iri` must be a resolved project IRI (the action resolves the
    /// user-supplied identifier via `resolve_project` first). Auth is optional
    /// (the endpoint is public); `token` is sent as a bearer when `Some`. Returns
    /// project data-models in server order (the action sorts). Platform built-ins
    /// are NOT included here — the action appends them when `--include-builtins`
    /// is set (see `builtin_data_models`).
    fn list_data_models(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<Vec<DataModel>, Diagnostic>;

    /// Fetch a single data-model's full content and summarise its child
    /// resource-types, via `GET /v2/ontologies/allentities/{data_model_iri}`.
    ///
    /// `data_model_iri` must be a resolved data-model IRI (the action resolves the
    /// user-supplied name-or-IRI against the project's data-models first). Auth is
    /// optional (the endpoint is public); `token` is sent as a bearer when `Some`.
    /// Returns the data-model identity/label/last-modified plus its resource-types
    /// (sorted by name). `allLanguages` is intentionally NOT requested, so labels
    /// are plain strings.
    fn describe_data_model(
        &self,
        server: &str,
        data_model_iri: &str,
        token: Option<&str>,
    ) -> Result<DataModelDetail, Diagnostic>;

    /// Fetch the full detail of a single resource-type within a data-model, via
    /// `GET /v2/ontologies/allentities/{data_model_iri}`.
    ///
    /// Fetches the data-model's `allentities` response, finds the class whose
    /// local name (case-insensitive) or full IRI matches `resource_type`, and
    /// returns the full [`ResourceTypeDetail`] including all fields (project and
    /// built-in), representation kind, and project superclasses. Cross-data-model
    /// fields are resolved by fetching sibling ontologies as needed (Decision 9).
    ///
    /// Returns `Err(Diagnostic::NotFound(...))` when no class in the queried
    /// ontology's `@graph` matches `resource_type`. The hint message referencing
    /// `resource-type list` is the caller's responsibility (ADR-0001: the ACTION
    /// layer owns the hint in dsp-cli vocabulary; the client owns the wire logic).
    ///
    /// Auth is optional; `token` is forwarded as a bearer when `Some`.
    /// Mirrors the doc style of `describe_data_model`.
    fn describe_resource_type(
        &self,
        server: &str,
        data_model_iri: &str,
        resource_type: &str,
        token: Option<&str>,
    ) -> Result<ResourceTypeDetail, Diagnostic>;

    /// Fetch per-resource-class instance counts for a project, via
    /// `GET {server}/v3/projects/{enc(project_iri)}/resourcesPerOntology`.
    ///
    /// The endpoint groups counts by ontology, but this method flattens the
    /// response into a single `HashMap<String, u64>` keyed by full
    /// resource-class IRI → its instance count (`itemCount`). Flat rather than
    /// grouped because it serves two consumers: `resource-type list` filters
    /// the map down to the target data-model's classes, and `resource-type
    /// describe` picks out one entry.
    ///
    /// **Semantics — read carefully, this differs from [`Self::list_resources`]:**
    /// `itemCount` counts **non-deleted** resources but is **NOT
    /// permission-filtered** — it includes resources the caller may not be
    /// allowed to see (a deliberate server-side performance tradeoff, since
    /// computing exact per-caller visible counts would require evaluating
    /// permissions over every instance). This is a different contract from
    /// `list_resources`, which IS permission-filtered. Callers that surface
    /// this count to a user must disclose that it may over-count relative to
    /// what that user can actually see.
    ///
    /// Auth is optional (mirrors `list_data_models`); `token` is sent as a
    /// bearer when `Some`, omitted when `None` (public endpoint).
    ///
    /// Status mapping:
    /// - `200` → parse and flatten into the returned map.
    /// - `404` → `Err(Diagnostic::NotFound(...))` (project not found).
    /// - `401`/`403` and everything else → the shared `map_unexpected_status`
    ///   mapping (401/403 already map to `AuthRequired` there; no bespoke arm
    ///   needed here).
    ///
    /// NEVER log the token.
    fn resource_counts(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<HashMap<String, u64>, Diagnostic>;

    /// Fetch the relation graph of a single data-model, via
    /// `GET /v2/ontologies/allentities/{data_model_iri}`.
    ///
    /// Returns all directed edges (link relations and inheritance relations)
    /// between resource-types in the data-model, sorted per D6 by
    /// `(source, kind, field, target)`.
    ///
    /// Cross-data-model targets are tagged with `target_data_model =
    /// Some(prefix)`. System targets (knora-api, knora-base, etc.) have
    /// `target_data_model = None` and `is_builtin` set per the asymmetric rules
    /// (see `Relation.is_builtin` docs). The action layer filters `is_builtin`
    /// edges based on `--include-builtins`.
    ///
    /// Only one `allentities` request is made (no sibling fetch — v1 limitation).
    /// Cross-data-model link fields whose property node is absent from this
    /// data-model's graph are silently skipped.
    ///
    /// Auth is optional; `token` is forwarded as a bearer when `Some`.
    fn data_model_structure(
        &self,
        server: &str,
        data_model_iri: &str,
        token: Option<&str>,
    ) -> Result<DataModelStructure, Diagnostic>;

    /// List resource instances of a given resource-type within a project.
    ///
    /// Issues `GET {server}/v2/resources` with:
    /// - query param `resourceClass=<resource_type_iri>` (URL-encoded by reqwest;
    ///   maps to the DSP-API `resourceClass` query parameter — wire name unchanged)
    /// - query param `page=<page>` (zero-based)
    /// - query param `schema=complex` (baked in — never a trait parameter, per D4;
    ///   complex carries per-resource `creationDate`/`lastModificationDate`, which
    ///   `simple` omits — the extra value objects are ignored by the envelope DTO)
    /// - header `x-knora-accept-project: <project_iri>`
    ///
    /// `token` is sent as a bearer when `Some`; omitted when `None` (anonymous
    /// path). Bearer auth is NOT required by the endpoint — omitting it is valid
    /// and returns only publicly-visible resources.
    ///
    /// `may_have_more_results` defaults to `false` when the
    /// `knora-api:mayHaveMoreResults` key is absent from the response (D5 spec).
    ///
    /// `order_by` is the already-resolved complex-schema property IRI (never a bare
    /// field name) — passed to the wire verbatim as `orderByProperty`. `None` omits
    /// the query param; `Some(iri)` appends `orderByProperty=<iri>`.
    fn list_resources(
        &self,
        server: &str,
        project_iri: &str,
        resource_type_iri: &str,
        order_by: Option<&str>,
        page: u32,
        token: Option<&str>,
    ) -> Result<ResourcePage, Diagnostic>;

    /// Fetch the full envelope metadata of a single resource by its IRI.
    ///
    /// Issues `GET {server}/v2/resources/{enc(resource_iri)}?schema=complex`.
    /// `token` is sent as a bearer when `Some`; omitted when `None` (anonymous
    /// path — the endpoint does not require auth, but anonymous callers see only
    /// publicly-visible resources).
    ///
    /// Returns a [`ResourceDetail`] with the resource's label, IRI, resource-type,
    /// optional ARK URL and timestamps, the owning project and user IRIs, and two
    /// translated permission facets (`visibility` + `your_access`).
    ///
    /// When `with_values` is `true`, the implementation **may** issue additional,
    /// deduplicated, non-fatal ontology (`/v2/ontologies/allentities`) and
    /// `/v2/node` fetches to resolve field labels and list-node labels; the method
    /// is therefore **not** always a single round-trip when `with_values == true`.
    /// `ResourceDetail.values` is set to `Some(...)` on success or when the
    /// resource has no value fields.  Mock implementations should set
    /// `values: None` when `with_values == false` and `values: Some(...)` when
    /// `with_values == true`.
    ///
    /// When `with_values` is `false` (the default) the method behaves exactly as
    /// the 8b implementation: a single HTTP request, `values: None`.
    ///
    /// Status mapping:
    /// - `200` → parse and return `ResourceDetail`.
    /// - `404` → `Err(Diagnostic::NotFound(...))`.
    /// - `401`/`403` → `Err(Diagnostic::AuthRequired(...))` with a "log in" hint
    ///   (deliberate: an anonymous caller describing a private resource gets 403,
    ///   and `AuthRequired` with a login hint is the right UX for an auth-optional read).
    /// - Other non-2xx → `Err(Diagnostic::ServerError(...))`.
    /// - Transport failure → `Err(Diagnostic::Network(...))`.
    ///
    /// NEVER log the token.
    fn describe_resource(
        &self,
        server: &str,
        resource_iri: &str,
        token: Option<&str>,
        with_values: bool,
    ) -> Result<ResourceDetail, Diagnostic>;

    /// Probe `server` to confirm that `token` is currently accepted.
    ///
    /// Issues `GET {server}/v2/authentication` with `Authorization: Bearer
    /// <token>`. The server is the authority — no local JWT validation is
    /// performed.
    ///
    /// - `200` (or any 2xx) → `Ok(())`.
    /// - `401` **and** `403` → `Err(Diagnostic::AuthRequired(...))`. Both map
    ///   to the same variant because they share the same user-facing meaning:
    ///   the token is not currently accepted. This is also why an expired token
    ///   surfaces as `AuthRequired` rather than a distinct error kind — the
    ///   server rejects it with `401`, which is handled identically to `403`.
    /// - Any other non-2xx → `Err(Diagnostic::ServerError(...))`.
    /// - Transport failure → `Err(Diagnostic::Network(...))`.
    fn verify_token(&self, server: &str, token: &str) -> Result<(), Diagnostic>;

    /// List a project's vocabularies (DSP-API "lists") via
    /// `GET /admin/lists?projectIri={enc(project_iri)}`.
    ///
    /// `project_iri` must be a resolved project IRI (the action resolves the
    /// user-supplied identifier via `resolve_project` first). Auth is optional
    /// (the endpoint is public); `token` is sent as a bearer when `Some`,
    /// mirroring `list_data_models`. Returns vocabularies in server order (the
    /// action sorts).
    ///
    /// `node_count` and `depth` are always `None` here — the root-listing
    /// endpoint carries neither, and fetching each vocabulary's tree to derive
    /// them is `--count`'s job (an action-layer concern: one extra fetch per
    /// vocabulary, opt-in and disclosed, not baked into every `list` call).
    fn list_vocabularies(
        &self,
        server: &str,
        project_iri: &str,
        token: Option<&str>,
    ) -> Result<Vec<Vocabulary>, Diagnostic>;

    /// Fetch a single vocabulary's full tree, via `GET /admin/lists/{enc(iri)}`.
    ///
    /// `iri` may address either a vocabulary root or one of its nodes — the
    /// response is polymorphic on which key it carries (`list` for a root,
    /// `node` for a node). When it is a root, the tree is built directly from
    /// the response. When it is a node, the response's own subtree payload is
    /// discarded and the implementation resolves upward: it reads
    /// `hasRootNode` from the node response and issues a second
    /// `GET /admin/lists/{enc(hasRootNode)}`, which MUST resolve to the root
    /// shape — one resolution hop only, no retry loop. A failed root fetch
    /// (either call) is a **hard error**, not a degrade: the vocabulary is
    /// what gets rendered, so there is nothing to fall back to.
    ///
    /// Returns a [`VocabularyTree`] with `requested_node` set to `Some(iri)`
    /// when the originally-addressed IRI turned out to be a node, `None` when
    /// it was already a root. Children are position-ordered (the server
    /// already sorts; the implementation re-sorts defensively) and walked
    /// **iteratively** while parsing, since this is the layer closest to
    /// untrusted server input.
    ///
    /// Auth is optional (public endpoint); `token` is sent as a bearer when
    /// `Some`, mirroring `list_data_models`.
    fn describe_vocabulary(
        &self,
        server: &str,
        iri: &str,
        token: Option<&str>,
    ) -> Result<VocabularyTree, Diagnostic>;

    /// Issue a raw SPARQL query against `server`'s underlying triplestore via
    /// `POST /admin/sparql/query`, and relay the store's own response.
    ///
    /// This is the **one** `DspClient` method that returns a relay struct
    /// ([`SparqlResponse`]) rather than a parsed domain model — ADR-0016 (D16).
    /// dsp-cli deliberately does not interpret the response body: the status,
    /// media type and bytes are the store's own, negotiated by `accept`.
    ///
    /// `token: &str`, not `Option<&str>` — unlike most other methods on this
    /// trait, the endpoint is never anonymous (D11): a `SystemAdmin` bearer
    /// token is required and resolved before this is ever called.
    ///
    /// `accept: &str`, not `Option<&str>` (D4, amended 2026-08-07) — there is
    /// no way to send zero `Accept` through `reqwest`'s public API (a
    /// `ClientBuilder` unconditionally seeds `Accept: */*` as a client-level
    /// default header and `execute_request` re-merges it into any request
    /// whose own `Accept` entry is vacant — `reqwest-0.13.4/src/async_impl/
    /// client.rs:284,2617`), so `--accept none` was dropped rather than faked
    /// as `*/*` (which is not equivalent: live-verified 2026-08-07, `*/*`
    /// returns JSON from this store while a genuinely absent `Accept` returns
    /// XML). `--accept xml` is the documented way to get the store's default
    /// serialization explicitly.
    ///
    /// `timeout_secs: u64` (D17) — the per-invocation ceiling on the dedicated
    /// SPARQL HTTP client (`--timeout`, default 3600s). Threaded through here
    /// because the client that carries this timeout is built **per call**, not
    /// once in `HttpDspClient::new()` (`reqwest::blocking::ClientBuilder` has
    /// no `read_timeout`, so a fixed inactivity bound is not available; see the
    /// `sparql_client` doc comment in `src/client/http.rs`).
    fn sparql_query(
        &self,
        server: &str,
        token: &str,
        query: &str,
        accept: &str,
        timeout_secs: u64,
    ) -> Result<SparqlResponse, Diagnostic>;
}
