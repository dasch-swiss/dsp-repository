//! Test support helpers shared by integration tests in `tests/`.
//!
//! `MockDspClient` is the T2 test seam from dsp-cli/ADR-0008 (internal architecture)
//! and the canned-response client used by layer-2 action tests per dsp-cli/ADR-0009
//! (testing strategy). Production code wires up the real HTTP `DspClient`
//! impl; action-level integration tests wire up `MockDspClient` instead.
//!
//! `SharedBuf` is a `Write`-impl wrapping a shared `Rc<RefCell<Vec<u8>>>`,
//! used by snapshot tests to capture renderer output. It uses `Rc<RefCell>`
//! (single-threaded) rather than `Arc<Mutex>` — integration test binaries are
//! single-threaded and `RefCell` has no lock overhead. The `src/render/`
//! unit-test copies use `Arc<Mutex>` (different variant, different home).
//!
//! ## State model — two tiers
//!
//! Most responses are **single-shot**: each field is an `Option<Result<…>>`
//! that is cloned and returned once. The poll progression is inherently
//! **multi-step**: `get_project_dump_status` replays a canned sequence using
//! `RefCell<VecDeque<…>>`, popping the front per call and **panicking on
//! exhaustion** so an over-eager poll loop fails loudly rather than looping
//! forever. All access is single-threaded (owned by one test); no `Arc` or
//! `Mutex` is needed.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::rc::Rc;

// ── SharedBuf ─────────────────────────────────────────────────────────────────

/// A `Write` impl wrapping a shared `Vec<u8>` so integration tests can read
/// captured renderer output after a render call.
///
/// Uses `Rc<RefCell<…>>` (single-threaded); NOT the `Arc<Mutex<…>>` variant
/// in `src/render/test_support` — the two homes are forced by the crate
/// boundary and differ intentionally.
pub struct SharedBuf(pub Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

/// Create a shared buffer and a `SharedBuf` writer pointing to it.
///
/// Returns `(handle, writer)` — pass `writer` to the renderer constructor,
/// then call `buf_to_string(&handle)` after rendering.
#[allow(dead_code)]
pub fn shared_buf() -> (Rc<RefCell<Vec<u8>>>, SharedBuf) {
    let buf = Rc::new(RefCell::new(Vec::<u8>::new()));
    let writer = SharedBuf(Rc::clone(&buf));
    (buf, writer)
}

/// Decode the captured bytes in `buf` as UTF-8.
///
/// Panics if the renderer emitted non-UTF-8 bytes (should never happen for
/// any renderer in this crate).
#[allow(dead_code)]
pub fn buf_to_string(buf: &Rc<RefCell<Vec<u8>>>) -> String {
    String::from_utf8(buf.borrow().clone()).expect("renderer output must be valid UTF-8")
}

use dsp_cli::client::DspClient;
use dsp_cli::client::sparql::SparqlResponse;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{
    CreateDumpOutcome, DataModel, DataModelDetail, DataModelStructure, DumpTask, LoginResponse, Project, ProjectDetail,
    ProjectRef, ResourceDetail, ResourcePage, ResourceTypeDetail, Vocabulary, VocabularyTree,
};

/// A recorded call to `list_projects`, capturing the arguments passed.
///
/// Stored by [`MockDspClient`] so Step 5 action tests can assert that the
/// correct `server` and `token` arguments were forwarded by the action layer.
/// The `token` field is `Option<String>` — `None` means the action called
/// `list_projects` with `token = None` (anonymous path).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListProjectsCall {
    pub server: String,
    pub token: Option<String>,
}

/// Test double for the [`DspClient`] trait.
///
/// Build a canned-response mock using the `with_*` builder methods, then pass
/// `&mock` wherever an action expects a `&dyn DspClient`.
// `allow(dead_code)`: fields and builders are consumed by future step tests
// (Steps 4–5 wiremock tests, Step 8 action tests). Suppress the warning rather
// than silently removing this shared fixture.
#[allow(dead_code)]
pub struct MockDspClient {
    pub login_result: Option<Result<LoginResponse, Diagnostic>>,
    pub resolve_project_result: Option<Result<ProjectRef, Diagnostic>>,
    pub create_dump_result: Option<Result<CreateDumpOutcome, Diagnostic>>,
    /// Sequence returned by successive `get_project_dump_status` calls.
    /// Each call pops the front entry. Panics if the sequence is exhausted
    /// before the poll loop finishes.
    pub poll_sequence: RefCell<VecDeque<Result<DumpTask, Diagnostic>>>,
    pub download_bytes: Option<Vec<u8>>,
    pub delete_result: Option<Result<(), Diagnostic>>,
    pub verify_token_result: Option<Result<(), Diagnostic>>,
    /// Canned response returned by [`DspClient::list_projects`].
    pub list_projects_result: Option<Result<Vec<Project>, Diagnostic>>,
    /// Every call to `list_projects` appends a [`ListProjectsCall`] here so
    /// tests can assert on the exact arguments (server, token) that the action
    /// layer forwarded — the "008 lesson" that verifying argument pass-through
    /// requires real call-record assertions, not just result checks.
    pub list_projects_calls: RefCell<Vec<ListProjectsCall>>,
    /// Canned response returned by [`DspClient::describe_project`].
    pub describe_project_result: Option<Result<ProjectDetail, Diagnostic>>,
    /// Canned response returned by [`DspClient::list_data_models`].
    pub list_data_models_result: Option<Result<Vec<DataModel>, Diagnostic>>,
    /// Canned response returned by [`DspClient::describe_data_model`].
    pub describe_data_model_result: Option<Result<DataModelDetail, Diagnostic>>,
    /// Canned response returned by [`DspClient::describe_resource_type`].
    pub describe_resource_type_result: Option<Result<ResourceTypeDetail, Diagnostic>>,
    /// Token captured from the most recent [`DspClient::describe_resource_type`] call.
    pub describe_resource_type_token: RefCell<Option<Option<String>>>,
    /// Canned response returned by [`DspClient::data_model_structure`].
    pub data_model_structure_result: Option<Result<DataModelStructure, Diagnostic>>,
    /// Canned response returned by [`DspClient::list_resources`].
    pub list_resources_result: Option<Result<ResourcePage, Diagnostic>>,
    /// Canned response returned by [`DspClient::describe_resource`].
    pub describe_resource_result: Option<Result<ResourceDetail, Diagnostic>>,
    /// Canned response returned by [`DspClient::list_vocabularies`].
    pub list_vocabularies_result: Option<Result<Vec<Vocabulary>, Diagnostic>>,
    /// Canned response returned by [`DspClient::describe_vocabulary`].
    pub describe_vocabulary_result: Option<Result<VocabularyTree, Diagnostic>>,
    /// Canned response returned by [`DspClient::sparql_query`].
    pub sparql_query_result: Option<Result<SparqlResponse, Diagnostic>>,
}

#[allow(dead_code)]
impl MockDspClient {
    /// Build a fresh mock with no canned responses configured.
    ///
    /// Calling trait methods on this instance will panic — configure a result
    /// with the appropriate builder before use.
    pub fn new() -> Self {
        Self {
            login_result: None,
            resolve_project_result: None,
            create_dump_result: None,
            poll_sequence: RefCell::new(VecDeque::new()),
            download_bytes: None,
            delete_result: None,
            verify_token_result: None,
            list_projects_result: None,
            list_projects_calls: RefCell::new(Vec::new()),
            describe_project_result: None,
            list_data_models_result: None,
            describe_data_model_result: None,
            describe_resource_type_result: None,
            describe_resource_type_token: RefCell::new(None),
            data_model_structure_result: None,
            list_resources_result: None,
            describe_resource_result: None,
            list_vocabularies_result: None,
            describe_vocabulary_result: None,
            sparql_query_result: None,
        }
    }

    /// Build a mock preconfigured to return `result` from [`DspClient::login`].
    pub fn with_login_result(result: Result<LoginResponse, Diagnostic>) -> Self {
        Self { login_result: Some(result), ..Self::new() }
    }

    /// Set the result returned by [`DspClient::resolve_project`].
    pub fn with_resolve_project(mut self, result: Result<ProjectRef, Diagnostic>) -> Self {
        self.resolve_project_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::create_project_dump`].
    ///
    /// Use `with_create_dump(Ok(CreateDumpOutcome::Created(task)))` for the
    /// "fresh dump triggered" path, or `with_create_exists(id)` as a shorthand
    /// for the "existing dump found" path.
    pub fn with_create_dump(mut self, result: Result<CreateDumpOutcome, Diagnostic>) -> Self {
        self.create_dump_result = Some(result);
        self
    }

    /// Shorthand for `with_create_dump(Ok(CreateDumpOutcome::Exists { id }))`.
    pub fn with_create_exists(mut self, id: impl Into<String>) -> Self {
        self.create_dump_result = Some(Ok(CreateDumpOutcome::Exists { id: id.into() }));
        self
    }

    /// Shorthand for `with_create_dump(Ok(CreateDumpOutcome::ExistsForOtherProject { id,
    /// project_iri }))`.
    pub fn with_create_exists_other_project(mut self, id: impl Into<String>, project_iri: impl Into<String>) -> Self {
        self.create_dump_result = Some(Ok(CreateDumpOutcome::ExistsForOtherProject {
            id: id.into(),
            project_iri: project_iri.into(),
        }));
        self
    }

    /// Set the poll sequence replayed by successive
    /// [`DspClient::get_project_dump_status`] calls.
    ///
    /// Each call pops the front of the sequence. If the sequence is exhausted
    /// before the poll loop finishes the mock panics loudly.
    pub fn with_poll_sequence(mut self, sequence: Vec<Result<DumpTask, Diagnostic>>) -> Self {
        self.poll_sequence = RefCell::new(VecDeque::from(sequence));
        self
    }

    /// Set the bytes written to `dest` by [`DspClient::download_project_dump`].
    ///
    /// The mock writes these bytes to `dest` and returns their length.
    pub fn with_download_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.download_bytes = Some(bytes);
        self
    }

    /// Set the result returned by [`DspClient::delete_project_dump`].
    pub fn with_delete_result(mut self, result: Result<(), Diagnostic>) -> Self {
        self.delete_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::verify_token`].
    pub fn with_verify_token(mut self, result: Result<(), Diagnostic>) -> Self {
        self.verify_token_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::list_projects`].
    pub fn with_list_projects(mut self, result: Result<Vec<Project>, Diagnostic>) -> Self {
        self.list_projects_result = Some(result);
        self
    }

    /// Return a snapshot of the calls recorded to `list_projects`.
    ///
    /// Each entry records the `server` and `token` arguments the action layer
    /// forwarded, so tests can assert that authentication is propagated correctly.
    pub fn list_projects_calls(&self) -> Vec<ListProjectsCall> {
        self.list_projects_calls.borrow().clone()
    }

    /// Set the result returned by [`DspClient::describe_project`].
    pub fn with_describe_project(mut self, result: Result<ProjectDetail, Diagnostic>) -> Self {
        self.describe_project_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::list_data_models`].
    pub fn with_list_data_models(mut self, result: Result<Vec<DataModel>, Diagnostic>) -> Self {
        self.list_data_models_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::describe_data_model`].
    pub fn with_describe_data_model(mut self, result: Result<DataModelDetail, Diagnostic>) -> Self {
        self.describe_data_model_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::describe_resource_type`].
    pub fn with_describe_resource_type(mut self, result: Result<ResourceTypeDetail, Diagnostic>) -> Self {
        self.describe_resource_type_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::data_model_structure`].
    pub fn with_data_model_structure(mut self, result: Result<DataModelStructure, Diagnostic>) -> Self {
        self.data_model_structure_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::list_resources`].
    pub fn with_list_resources(mut self, result: Result<ResourcePage, Diagnostic>) -> Self {
        self.list_resources_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::describe_resource`].
    pub fn with_describe_resource(mut self, result: Result<ResourceDetail, Diagnostic>) -> Self {
        self.describe_resource_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::list_vocabularies`].
    pub fn with_list_vocabularies(mut self, result: Result<Vec<Vocabulary>, Diagnostic>) -> Self {
        self.list_vocabularies_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::describe_vocabulary`].
    pub fn with_describe_vocabulary(mut self, result: Result<VocabularyTree, Diagnostic>) -> Self {
        self.describe_vocabulary_result = Some(result);
        self
    }

    /// Set the result returned by [`DspClient::sparql_query`].
    pub fn with_sparql_query(mut self, result: Result<SparqlResponse, Diagnostic>) -> Self {
        self.sparql_query_result = Some(result);
        self
    }
}

impl Default for MockDspClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DspClient for MockDspClient {
    fn login(&self, _server: &str, _user: &str, _password: &str) -> Result<LoginResponse, Diagnostic> {
        // `expect` is acceptable in test support code — the panic message
        // points the test author at the builder they forgot to call.
        self.login_result
            .clone()
            .expect("MockDspClient::login_result was not configured; use MockDspClient::with_login_result(...)")
    }

    fn resolve_project(&self, _server: &str, _project: &str) -> Result<ProjectRef, Diagnostic> {
        self.resolve_project_result.clone().expect(
            "MockDspClient::resolve_project_result was not configured; use MockDspClient::with_resolve_project(...)",
        )
    }

    fn create_project_dump(
        &self,
        _server: &str,
        _project_iri: &str,
        _skip_assets: bool,
        _token: &str,
    ) -> Result<CreateDumpOutcome, Diagnostic> {
        self.create_dump_result
            .clone()
            .expect("MockDspClient::create_dump_result was not configured; use MockDspClient::with_create_dump(...)")
    }

    fn get_project_dump_status(
        &self,
        _server: &str,
        _project_iri: &str,
        _dump_id: &str,
        _token: &str,
    ) -> Result<DumpTask, Diagnostic> {
        self.poll_sequence.borrow_mut().pop_front().expect(
            "MockDspClient::poll_sequence was exhausted; add more entries via MockDspClient::with_poll_sequence(...)",
        )
    }

    fn download_project_dump(
        &self,
        _server: &str,
        _project_iri: &str,
        _dump_id: &str,
        _token: &str,
        dest: &mut dyn std::io::Write,
    ) -> Result<u64, Diagnostic> {
        let bytes = self
            .download_bytes
            .as_ref()
            .expect("MockDspClient::download_bytes was not configured; use MockDspClient::with_download_bytes(...)");
        dest.write_all(bytes)
            .map_err(|e| Diagnostic::Internal(format!("mock download write failed: {e}")))?;
        Ok(bytes.len() as u64)
    }

    fn delete_project_dump(
        &self,
        _server: &str,
        _project_iri: &str,
        _dump_id: &str,
        _token: &str,
    ) -> Result<(), Diagnostic> {
        self.delete_result
            .clone()
            .expect("MockDspClient::delete_result was not configured; use MockDspClient::with_delete_result(...)")
    }

    fn list_projects(&self, server: &str, token: Option<&str>) -> Result<Vec<Project>, Diagnostic> {
        // Record the call so Step 5 action tests can assert on the forwarded arguments.
        self.list_projects_calls
            .borrow_mut()
            .push(ListProjectsCall { server: server.to_string(), token: token.map(str::to_string) });
        self.list_projects_result.clone().expect(
            "MockDspClient::list_projects_result was not configured; use MockDspClient::with_list_projects(...)",
        )
    }

    fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
        self.verify_token_result
            .clone()
            .expect("MockDspClient::verify_token_result was not configured; use MockDspClient::with_verify_token(...)")
    }

    fn describe_project(
        &self,
        _server: &str,
        _project: &str,
        _token: Option<&str>,
    ) -> Result<ProjectDetail, Diagnostic> {
        self.describe_project_result.clone().expect(
            "MockDspClient::describe_project_result was not configured; use MockDspClient::with_describe_project(...)",
        )
    }

    fn list_data_models(
        &self,
        _server: &str,
        _project_iri: &str,
        _token: Option<&str>,
    ) -> Result<Vec<DataModel>, Diagnostic> {
        self.list_data_models_result.clone().expect(
            "MockDspClient::list_data_models_result was not configured; use MockDspClient::with_list_data_models(...)",
        )
    }

    fn describe_data_model(
        &self,
        _server: &str,
        _data_model_iri: &str,
        _token: Option<&str>,
    ) -> Result<DataModelDetail, Diagnostic> {
        self.describe_data_model_result
            .clone()
            .expect("MockDspClient::describe_data_model_result was not configured; use MockDspClient::with_describe_data_model(...)")
    }

    fn describe_resource_type(
        &self,
        _server: &str,
        _data_model_iri: &str,
        _resource_type: &str,
        token: Option<&str>,
    ) -> Result<ResourceTypeDetail, Diagnostic> {
        // Capture the token for integration-test assertion.
        *self.describe_resource_type_token.borrow_mut() = Some(token.map(str::to_owned));
        self.describe_resource_type_result
            .clone()
            .expect("MockDspClient::describe_resource_type_result was not configured; use MockDspClient::with_describe_resource_type(...)")
    }

    fn data_model_structure(
        &self,
        _server: &str,
        _data_model_iri: &str,
        _token: Option<&str>,
    ) -> Result<DataModelStructure, Diagnostic> {
        self.data_model_structure_result
            .clone()
            .expect("MockDspClient::data_model_structure_result was not configured; use MockDspClient::with_data_model_structure(...)")
    }

    fn list_resources(
        &self,
        _server: &str,
        _project_iri: &str,
        _resource_type_iri: &str,
        _order_by: Option<&str>,
        _page: u32,
        _token: Option<&str>,
    ) -> Result<ResourcePage, Diagnostic> {
        self.list_resources_result.clone().expect(
            "MockDspClient::list_resources_result was not configured; use MockDspClient::with_list_resources(...)",
        )
    }

    fn describe_resource(
        &self,
        _server: &str,
        _resource_iri: &str,
        _token: Option<&str>,
        _with_values: bool,
    ) -> Result<ResourceDetail, Diagnostic> {
        self.describe_resource_result
            .clone()
            .expect("MockDspClient::describe_resource_result was not configured; use MockDspClient::with_describe_resource(...)")
    }

    fn resource_counts(
        &self,
        _server: &str,
        _project_iri: &str,
        _token: Option<&str>,
    ) -> Result<std::collections::HashMap<String, u64>, Diagnostic> {
        Ok(std::collections::HashMap::new())
    }

    fn list_vocabularies(
        &self,
        _server: &str,
        _project_iri: &str,
        _token: Option<&str>,
    ) -> Result<Vec<Vocabulary>, Diagnostic> {
        self.list_vocabularies_result
            .clone()
            .expect("MockDspClient::list_vocabularies_result was not configured; use MockDspClient::with_list_vocabularies(...)")
    }

    fn describe_vocabulary(
        &self,
        _server: &str,
        _iri: &str,
        _token: Option<&str>,
    ) -> Result<VocabularyTree, Diagnostic> {
        self.describe_vocabulary_result
            .clone()
            .expect("MockDspClient::describe_vocabulary_result was not configured; use MockDspClient::with_describe_vocabulary(...)")
    }

    fn sparql_query(
        &self,
        _server: &str,
        _token: &str,
        _query: &str,
        _accept: &str,
        _timeout_secs: u64,
    ) -> Result<SparqlResponse, Diagnostic> {
        self.sparql_query_result
            .clone()
            .expect("MockDspClient::sparql_query_result was not configured; use MockDspClient::with_sparql_query(...)")
    }
}
