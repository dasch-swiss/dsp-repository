//! Actions for `dsp vre sparql query` (plan 035).
//!
//! Deliberately does **not** abstract DSP-API (ADR-0016): the response body
//! is a store-authored document in a store-negotiated media type, and
//! dsp-cli does not interpret it. This is the third no-`Renderer` command
//! (after `dsp docs` and `dsp auth token`, D2) — its output goes to an
//! injected writer via the `run`/`query` seam, mirroring `set_token.rs`'s
//! `run`/`run_from_line`/`run_impl` shape (each layer taking already-read
//! text, never owning IO itself below `run`).
//!
//! dsp-api's own failures (D8) are classified inside `src/client/http.rs`
//! and arrive here as `Err`, already carrying the right `Diagnostic` variant
//! and exit code — this module must never re-implement that table (D8's
//! closing paragraph). The only split this module owns is D7's: a `2xx`
//! relay writes the body to stdout byte-verbatim; anything else is a relayed
//! store rejection, surfaced as `Diagnostic::ServerError` with the store's
//! text sanitised and capped (never interpolated raw — the store's bytes are
//! untrusted prose the moment they leave the byte-exact stdout contract).

use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;

use crate::cli::SparqlQueryArgs;
use crate::client::DspClient;
use crate::config::{AuthCache, Config, resolve_token};
use crate::diagnostic::Diagnostic;

/// D4's `--accept` resolution: an alias table, `/`-passthrough for a raw
/// media type, and a usage error for anything else. `None` defaults to
/// `json` — the store's own no-`Accept` default is XML, which is hostile to
/// the agent audience this tool exists for (D4).
///
/// There is no `none` sentinel (D4, amended 2026-08-07): `reqwest`'s public
/// API cannot express "send no `Accept` header at all", so the flag was
/// dropped rather than faked as `*/*` (not equivalent — live-verified,
/// `*/*` returns JSON here while a genuinely absent `Accept` returns XML).
/// `--accept xml` is the documented way to ask for the store's own default.
fn resolve_accept(arg: Option<&str>) -> Result<String, Diagnostic> {
    let Some(s) = arg else {
        return Ok("application/sparql-results+json".to_string());
    };

    if s.is_empty() {
        return Err(Diagnostic::Usage("--accept must not be empty".into()));
    }

    // A raw media type is forwarded into a header — reject control characters
    // and cap the length so an unvalidated argv value can't rely solely on
    // `http::HeaderValue`'s internal validation (D4's closing warning: without
    // this check a bad value surfaces as Network/Internal, not the Usage
    // error D4 promises).
    if s.contains('/') {
        // `is_control()`, not `is_ascii_control()`: non-ASCII control scalars
        // (e.g. U+0085 NEL) would otherwise pass, making D4's promise
        // conditional on the byte range.
        if s.chars().any(|c| c.is_control()) {
            return Err(Diagnostic::Usage("--accept must not contain control characters".into()));
        }
        if s.chars().count() > 200 {
            return Err(Diagnostic::Usage("--accept is too long (max 200 characters)".into()));
        }
        return Ok(s.to_string());
    }

    match s {
        "json" => Ok("application/sparql-results+json".to_string()),
        "xml" => Ok("application/sparql-results+xml".to_string()),
        "csv" => Ok("text/csv".to_string()),
        "tsv" => Ok("text/tab-separated-values".to_string()),
        "turtle" => Ok("text/turtle".to_string()),
        "ntriples" => Ok("application/n-triples".to_string()),
        "jsonld" => Ok("application/ld+json".to_string()),
        // Echoing argv: sanitise and cap it like every other server- or
        // user-supplied string that becomes prose (the convention
        // `read_query_file` follows for the path). This branch skips the
        // control-char/length guards above, which only cover the `/` arm, so
        // without this an `--accept $'\x1b]0;x\x07'` would reach a terminal
        // verbatim and a multi-megabyte value would be echoed whole.
        other => Err(Diagnostic::Usage(format!(
            "unknown --accept alias '{}'; valid aliases: json, xml, csv, tsv, turtle, \
             ntriples, jsonld — or pass a raw media type containing '/'",
            crate::util::text::sanitise_and_cap(other)
        ))),
    }
}

/// D5's *precedence* errors only — pure, no IO, no client call. The real
/// stdin read and the real `--query-file` read both happen in [`run`], which
/// classifies the three file-read failure modes (D5) before ever calling
/// this; `file_text` therefore arrives already read, and this function has
/// no channel for a file-read error by design.
fn resolve_query(
    args: &SparqlQueryArgs,
    stdin_is_tty: bool,
    stdin_text: Option<String>,
    file_text: Option<String>,
) -> Result<String, Diagnostic> {
    let text = if let Some(q) = &args.query {
        q.clone()
    } else if args.query_file.is_some() {
        // A missing `file_text` here means the caller did not read the file
        // the flag names — a broken internal invariant, not empty user input.
        file_text.ok_or_else(|| Diagnostic::Internal("--query-file was given but its contents were not read".into()))?
    } else if let Some(s) = stdin_text {
        s
    } else if stdin_is_tty {
        return Err(Diagnostic::Usage(
            "no query given: provide --query <text>, --query-file <path>, or pipe the query \
             via stdin — dsp-cli will not wait on an interactive terminal"
                .into(),
        ));
    } else {
        // Should not happen in practice: `run` always attempts a stdin read
        // when neither flag is given and stdin is not a TTY. Treated as the
        // same usage error rather than panicking, since this function has no
        // other way to signal "no source was actually provided".
        return Err(Diagnostic::Usage(
            "no query given: provide --query <text>, --query-file <path>, or pipe the query \
             via stdin"
                .into(),
        ));
    };

    if text.trim().is_empty() {
        return Err(Diagnostic::Usage("the SPARQL query text must not be empty".into()));
    }

    Ok(text)
}

/// Read `--query-file`'s target, classifying all three D5 failure modes as
/// `Diagnostic::Usage` naming the (sanitised) path and never the file's
/// bytes. ⚠️ Never a bare `?` on the `std::fs`/`io` calls here — that would
/// hit the blanket `From<io::Error>` impl and mis-classify as
/// `Diagnostic::Internal` (`src/diagnostic.rs:67-77`).
fn read_query_file(path: &str) -> Result<String, Diagnostic> {
    // D5's sanitise-and-cap requirement reuses the one shared helper (D7),
    // rather than duplicating `resource.rs`'s inline 80-char pattern.
    let safe_path = crate::util::text::sanitise_and_cap(path);

    let p = Path::new(path);
    if p.is_dir() {
        return Err(Diagnostic::Usage(format!(
            "--query-file '{safe_path}' is a directory, not a file"
        )));
    }

    let bytes =
        std::fs::read(p).map_err(|e| Diagnostic::Usage(format!("could not read --query-file '{safe_path}': {e}")))?;

    String::from_utf8(bytes).map_err(|_| Diagnostic::Usage(format!("--query-file '{safe_path}' is not valid UTF-8")))
}

/// Run a raw SPARQL query, owning all real IO: stdin (D5), the
/// `--query-file` read (D5), `DSP_TOKEN` (D11), and stdout (D3).
///
/// Reads stdin only when neither `--query` nor `--query-file` is given, and
/// never when stdin is a terminal — a hang waiting on an interactive
/// terminal is a usage error, not a silent block (D5).
pub fn run(args: &SparqlQueryArgs, cfg: &Config, client: &dyn DspClient) -> Result<(), Diagnostic> {
    let stdin_is_tty = io::stdin().is_terminal();

    let file_text = match &args.query_file {
        Some(path) => Some(read_query_file(path)?),
        None => None,
    };

    let stdin_text = if args.query.is_none() && args.query_file.is_none() && !stdin_is_tty {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| Diagnostic::Usage(format!("could not read query from stdin: {e}")))?;
        Some(buf)
    } else {
        None
    };

    let env_token = std::env::var("DSP_TOKEN").ok();
    let mut out = io::stdout();

    query(
        args,
        cfg,
        client,
        env_token,
        None,
        stdin_is_tty,
        stdin_text,
        file_text,
        &mut out,
    )
}

/// The testable action: resolve token (fail fast, D11) → resolve query text
/// (D5) → resolve `--accept` (D4) → issue the request → D7's status split.
///
/// dsp-api's own failures (D8) already returned `Err` with the right
/// `Diagnostic` variant and exit code from inside `client.sparql_query` —
/// this function's status split therefore only ever sees `Ok(SparqlResponse)`
/// and must NOT re-implement any of D8's table (that knowledge stays in
/// `src/client/`, per ADR-0001).
///
/// - `env_token`/`cache_path`: injectable seams (mirrors `project::run_impl`) so unit tests never
///   touch the real environment or `~/.config/dsp-cli/`.
/// - `stdin_is_tty`/`stdin_text`/`file_text`: already-read input, per D5 — this function performs
///   no IO of its own beyond `out`.
#[allow(clippy::too_many_arguments)]
fn query(
    args: &SparqlQueryArgs,
    cfg: &Config,
    client: &dyn DspClient,
    env_token: Option<String>,
    cache_path: Option<&Path>,
    stdin_is_tty: bool,
    stdin_text: Option<String>,
    file_text: Option<String>,
    out: &mut dyn Write,
) -> Result<(), Diagnostic> {
    // ── 1. Resolve token, fail fast (D11) — BEFORE any HTTP call ────────────
    let env_token_would_win = env_token.as_deref().map(str::trim).map(|s| !s.is_empty()).unwrap_or(false);

    let cache_result = match cache_path {
        Some(p) => AuthCache::load_from(p),
        None => AuthCache::load(),
    };
    let cache = match cache_result {
        Ok(c) => c,
        Err(e) if env_token_would_win => {
            tracing::warn!(
                error = %e,
                "auth cache load failed; DSP_TOKEN is set, falling through to env token"
            );
            AuthCache::default()
        }
        Err(e) => return Err(e),
    };

    let resolved = resolve_token(env_token, &cache, &cfg.server).ok_or_else(|| {
        Diagnostic::AuthRequired(
            "dsp vre sparql query requires a system-administrator token; run `dsp auth login` \
             or set DSP_TOKEN"
                .into(),
        )
    })?;

    // ── 2. Resolve query text and --accept — pure, no IO, no client call ───
    let query_text = resolve_query(args, stdin_is_tty, stdin_text, file_text)?;
    let accept = resolve_accept(args.accept.as_deref())?;

    // ── 3. Issue the request ────────────────────────────────────────────────
    let resp = client.sparql_query(&cfg.server, &resolved.token, &query_text, &accept, args.timeout)?;

    // ── 4. D7's split — the only decision this action owns ─────────────────
    if (200..300).contains(&resp.status) {
        out.write_all(&resp.body)?;
        out.flush()?;
        Ok(())
    } else {
        // The bounded variant: same sanitise-and-cap, but it slices before
        // decoding so a multi-MiB store error is not copied twice to print 200
        // characters. All three body-to-prose sites use this one helper (D7).
        let sanitised = crate::util::text::sanitise_bytes_for_prose(&resp.body);
        Err(Diagnostic::ServerError(format!(
            "the triplestore rejected the query (HTTP {}): {sanitised}",
            resp.status
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;
    use crate::client::sparql::SparqlResponse;
    use crate::model::ProjectRef;

    // ── resolve_accept ───────────────────────────────────────────────────────

    #[test]
    fn resolve_accept_defaults_to_json() {
        assert_eq!(resolve_accept(None).unwrap(), "application/sparql-results+json");
    }

    #[test]
    fn resolve_accept_every_alias() {
        assert_eq!(resolve_accept(Some("json")).unwrap(), "application/sparql-results+json");
        assert_eq!(resolve_accept(Some("xml")).unwrap(), "application/sparql-results+xml");
        assert_eq!(resolve_accept(Some("csv")).unwrap(), "text/csv");
        assert_eq!(resolve_accept(Some("tsv")).unwrap(), "text/tab-separated-values");
        assert_eq!(resolve_accept(Some("turtle")).unwrap(), "text/turtle");
        assert_eq!(resolve_accept(Some("ntriples")).unwrap(), "application/n-triples");
        assert_eq!(resolve_accept(Some("jsonld")).unwrap(), "application/ld+json");
    }

    #[test]
    fn resolve_accept_raw_media_type_is_forwarded_verbatim() {
        assert_eq!(
            resolve_accept(Some("text/csv;q=1, */*;q=0.1")).unwrap(),
            "text/csv;q=1, */*;q=0.1"
        );
    }

    #[test]
    fn resolve_accept_unknown_alias_is_usage_error() {
        let err = resolve_accept(Some("jsonn")).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn resolve_accept_empty_is_usage_error() {
        let err = resolve_accept(Some("")).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn resolve_accept_control_character_is_usage_error() {
        let err = resolve_accept(Some("text/csv\r\nX-Evil: 1")).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn resolve_accept_overlong_raw_type_is_usage_error() {
        let long = format!("text/{}", "x".repeat(300));
        let err = resolve_accept(Some(&long)).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    // ── resolve_query ────────────────────────────────────────────────────────

    fn args_with(query: Option<&str>, query_file: Option<&str>) -> SparqlQueryArgs {
        SparqlQueryArgs {
            server: Some("https://example.org".to_string()),
            query: query.map(str::to_string),
            query_file: query_file.map(str::to_string),
            accept: None,
            timeout: 3600,
        }
    }

    #[test]
    fn resolve_query_from_flag() {
        let args = args_with(Some("SELECT * WHERE { ?s ?p ?o }"), None);
        let text = resolve_query(&args, false, None, None).unwrap();
        assert_eq!(text, "SELECT * WHERE { ?s ?p ?o }");
    }

    #[test]
    fn resolve_query_from_file_text() {
        let args = args_with(None, Some("query.rq"));
        let text = resolve_query(&args, false, None, Some("SELECT * WHERE { ?s ?p ?o }".into())).unwrap();
        assert_eq!(text, "SELECT * WHERE { ?s ?p ?o }");
    }

    #[test]
    fn resolve_query_from_stdin() {
        let args = args_with(None, None);
        let text = resolve_query(&args, false, Some("SELECT * WHERE { ?s ?p ?o }".into()), None).unwrap();
        assert_eq!(text, "SELECT * WHERE { ?s ?p ?o }");
    }

    #[test]
    fn resolve_query_tty_with_no_flag_is_usage_error() {
        let args = args_with(None, None);
        let err = resolve_query(&args, true, None, None).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn resolve_query_empty_is_usage_error() {
        let args = args_with(Some("   "), None);
        let err = resolve_query(&args, false, None, None).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    // ── MockDspClient ────────────────────────────────────────────────────────

    struct MockDspClient {
        sparql_query_result: Option<Result<SparqlResponse, Diagnostic>>,
        sparql_query_calls: RefCell<u32>,
        sparql_query_accept: RefCell<Option<String>>,
    }

    impl MockDspClient {
        fn new() -> Self {
            Self {
                sparql_query_result: None,
                sparql_query_calls: RefCell::new(0),
                sparql_query_accept: RefCell::new(None),
            }
        }

        fn with_sparql_query(mut self, result: Result<SparqlResponse, Diagnostic>) -> Self {
            self.sparql_query_result = Some(result);
            self
        }

        fn calls(&self) -> u32 {
            *self.sparql_query_calls.borrow()
        }

        fn accept(&self) -> Option<String> {
            self.sparql_query_accept.borrow().clone()
        }
    }

    impl DspClient for MockDspClient {
        fn login(
            &self,
            _server: &str,
            _user: &str,
            _password: &str,
        ) -> Result<crate::model::LoginResponse, Diagnostic> {
            unimplemented!("login not used in sparql action tests")
        }

        fn resolve_project(&self, _server: &str, _project: &str) -> Result<ProjectRef, Diagnostic> {
            unimplemented!("resolve_project not used in sparql action tests")
        }

        fn create_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _skip_assets: bool,
            _token: &str,
        ) -> Result<crate::model::CreateDumpOutcome, Diagnostic> {
            unimplemented!("create_project_dump not used in sparql action tests")
        }

        fn get_project_dump_status(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<crate::model::DumpTask, Diagnostic> {
            unimplemented!("get_project_dump_status not used in sparql action tests")
        }

        fn download_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
            _dest: &mut dyn std::io::Write,
        ) -> Result<u64, Diagnostic> {
            unimplemented!("download_project_dump not used in sparql action tests")
        }

        fn delete_project_dump(
            &self,
            _server: &str,
            _project_iri: &str,
            _dump_id: &str,
            _token: &str,
        ) -> Result<(), Diagnostic> {
            unimplemented!("delete_project_dump not used in sparql action tests")
        }

        fn list_projects(&self, _server: &str, _token: Option<&str>) -> Result<Vec<crate::model::Project>, Diagnostic> {
            unimplemented!("list_projects not used in sparql action tests")
        }

        fn describe_project(
            &self,
            _server: &str,
            _project: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ProjectDetail, Diagnostic> {
            unimplemented!("describe_project not used in sparql action tests")
        }

        fn list_data_models(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::DataModel>, Diagnostic> {
            unimplemented!("list_data_models not used in sparql action tests")
        }

        fn describe_data_model(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelDetail, Diagnostic> {
            unimplemented!("describe_data_model not used in sparql action tests")
        }

        fn describe_resource_type(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _resource_type: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourceTypeDetail, Diagnostic> {
            unimplemented!("describe_resource_type not used in sparql action tests")
        }

        fn resource_counts(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<HashMap<String, u64>, Diagnostic> {
            unimplemented!("resource_counts not used in sparql action tests")
        }

        fn data_model_structure(
            &self,
            _server: &str,
            _data_model_iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::DataModelStructure, Diagnostic> {
            unimplemented!("data_model_structure not used in sparql action tests")
        }

        fn list_resources(
            &self,
            _server: &str,
            _project_iri: &str,
            _resource_type_iri: &str,
            _order_by: Option<&str>,
            _page: u32,
            _token: Option<&str>,
        ) -> Result<crate::model::ResourcePage, Diagnostic> {
            unimplemented!("list_resources not used in sparql action tests")
        }

        fn describe_resource(
            &self,
            _server: &str,
            _resource_iri: &str,
            _token: Option<&str>,
            _with_values: bool,
        ) -> Result<crate::model::ResourceDetail, Diagnostic> {
            unimplemented!("describe_resource not used in sparql action tests")
        }

        fn verify_token(&self, _server: &str, _token: &str) -> Result<(), Diagnostic> {
            unimplemented!("verify_token not used in sparql action tests")
        }

        fn list_vocabularies(
            &self,
            _server: &str,
            _project_iri: &str,
            _token: Option<&str>,
        ) -> Result<Vec<crate::model::Vocabulary>, Diagnostic> {
            unimplemented!("list_vocabularies not used in sparql action tests")
        }

        fn describe_vocabulary(
            &self,
            _server: &str,
            _iri: &str,
            _token: Option<&str>,
        ) -> Result<crate::model::VocabularyTree, Diagnostic> {
            unimplemented!("describe_vocabulary not used in sparql action tests")
        }

        fn sparql_query(
            &self,
            _server: &str,
            _token: &str,
            _query: &str,
            accept: &str,
            _timeout_secs: u64,
        ) -> Result<SparqlResponse, Diagnostic> {
            *self.sparql_query_calls.borrow_mut() += 1;
            *self.sparql_query_accept.borrow_mut() = Some(accept.to_string());
            self.sparql_query_result
                .clone()
                .expect("sparql_query_result must be set when sparql_query is called")
        }
    }

    fn cfg() -> Config {
        Config { server: "https://example.org".to_string() }
    }

    fn empty_cache_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── action-level tests ───────────────────────────────────────────────────

    #[test]
    fn success_writes_exact_bytes_to_out() {
        let client = MockDspClient::new().with_sparql_query(Ok(SparqlResponse {
            status: 200,
            content_type: Some("application/sparql-results+json".to_string()),
            body: b"{\"results\":{\"bindings\":[]}}".to_vec(),
        }));
        let args = args_with(Some("SELECT * WHERE { ?s ?p ?o }"), None);
        let dir = empty_cache_dir();
        let cache_path = dir.path().join("auth.toml");
        let mut out = Vec::new();

        query(
            &args,
            &cfg(),
            &client,
            Some("a-token".to_string()),
            Some(&cache_path),
            false,
            None,
            None,
            &mut out,
        )
        .expect("2xx relay must succeed");

        assert_eq!(out, b"{\"results\":{\"bindings\":[]}}");
        assert_eq!(client.calls(), 1);
    }

    #[test]
    fn store_400_maps_to_server_error_and_writes_nothing() {
        let client = MockDspClient::new().with_sparql_query(Ok(SparqlResponse {
            status: 400,
            content_type: Some("text/plain".to_string()),
            // Includes an ANSI escape: the relay path must strip it before the
            // text becomes prose on stderr (D7). Asserting only that "Parse
            // error" survives would not catch an unsanitised relay.
            body: b"Parse error: \x1b]0;pwned\x07line 1, column 1: nonsense".to_vec(),
        }));
        let args = args_with(Some("not a query"), None);
        let dir = empty_cache_dir();
        let cache_path = dir.path().join("auth.toml");
        let mut out = Vec::new();

        let err = query(
            &args,
            &cfg(),
            &client,
            Some("a-token".to_string()),
            Some(&cache_path),
            false,
            None,
            None,
            &mut out,
        )
        .expect_err("a store 400 must be Err");

        match err {
            Diagnostic::ServerError(msg) => {
                assert!(msg.contains("Parse error"), "message must contain the store's text: {msg}");
                assert!(
                    !msg.contains('\u{1b}') && !msg.contains('\u{7}'),
                    "the relay path must strip control characters (D7): {msg:?}"
                );
            }
            other => panic!("expected ServerError, got: {other:?}"),
        }
        assert!(out.is_empty(), "nothing must be written to stdout on a relayed rejection");
    }

    #[test]
    fn no_token_is_auth_required_and_client_is_never_called() {
        let client = MockDspClient::new();
        let args = args_with(Some("SELECT * WHERE { ?s ?p ?o }"), None);
        let dir = empty_cache_dir();
        let cache_path = dir.path().join("auth.toml");
        let mut out = Vec::new();

        let err = query(&args, &cfg(), &client, None, Some(&cache_path), false, None, None, &mut out)
            .expect_err("no token must be Err");

        assert!(matches!(err, Diagnostic::AuthRequired(_)));
        assert_eq!(client.calls(), 0, "the client must never be called without a token");
        assert!(out.is_empty());
    }

    #[test]
    fn accept_alias_reaches_the_client_as_its_media_type() {
        let client = MockDspClient::new().with_sparql_query(Ok(SparqlResponse {
            status: 200,
            content_type: Some("text/csv".to_string()),
            body: b"s,p,o\n".to_vec(),
        }));
        let mut args = args_with(Some("SELECT * WHERE { ?s ?p ?o }"), None);
        args.accept = Some("csv".to_string());
        let dir = empty_cache_dir();
        let cache_path = dir.path().join("auth.toml");
        let mut out = Vec::new();

        query(
            &args,
            &cfg(),
            &client,
            Some("a-token".to_string()),
            Some(&cache_path),
            false,
            None,
            None,
            &mut out,
        )
        .expect("2xx relay must succeed");

        assert_eq!(client.accept(), Some("text/csv".to_string()));
    }
}
