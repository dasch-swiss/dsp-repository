//! An operator's refresh of project 0803 over its shipped inputs as they are committed, against
//! wiremock with a synthetic dump: the `media` generation holds those inputs byte for byte and the
//! rights record, and both Ready tokens record [`INPUTS_DIGEST_0803`].

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use shared_stand_in::{ReadyToken, TOKEN_FILE};
use vre_bridge_server::{RawSettings, VreBridge, METER_SCOPE};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// The digest of `inputs/0803/`, as `vre_bridge_core::inputs_digest` computes it. The committed
/// fixture tokens record it too, which `just check-stand-in-fixture` reads from this line.
const INPUTS_DIGEST_0803: &str = "sha256:260b4ee50905eaf727df4b6d00dc167a0aad004cf01dac2102f8a4efdaae2d9e";

/// The shipped inputs, copied at their repository paths: runfiles are symbolic links, which the
/// digest refuses.
const INPUTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_inputs/areas/deposit/vre-bridge/inputs");
const KEY: &str = include_str!("../../client/testdata/ed25519-private.pem");

const IRI: &str = "http://rdfh.ch/projects/0803";
const ENCODED_IRI: &str = "http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0803";
const ONTO: &str = "http://www.knora.org/ontology/0803/incunabula";
const DATA: &str = "http://www.knora.org/data/0803/incunabula";

/// The rights record the bridge writes into every `media` generation: each of its files, the
/// shipped inputs included, is public and unclassified.
const RIGHTS: &str = r#"{"format":"access-rights","version":1,"default":{"access_rights":"public","data_classification":"unclassified"}}"#;

/// A dump zip with one public Page.
fn dump() -> Vec<u8> {
    let page = "http://rdfh.ch/0803/ZEBRA";
    let data = format!(
        "<{page}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{ONTO}#Page> <{DATA}> .\n\
         <{page}> <http://www.w3.org/2000/01/rdf-schema#label> \"a page\" <{DATA}> .\n\
         <{page}> <http://www.knora.org/ontology/knora-base#isDeleted> \"false\"^^<http://www.w3.org/2001/XMLSchema#boolean> <{DATA}> .\n\
         <{page}> <http://www.knora.org/ontology/knora-base#hasPermissions> \"V knora-admin:UnknownUser\" <{DATA}> .\n"
    );
    let ontology = format!(
        "<{ONTO}#Page> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> <{ONTO}> .\n"
    );
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, content) in [("data/rdf/data.nq", &data), ("data/rdf/ontology-0.nq", &ontology)] {
        writer.start_file(name, SimpleFileOptions::default()).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// Answers each request with `response`, and checks when `dsp` is dropped that exactly one came.
async fn mount(dsp: &MockServer, http_method: &str, at: String, response: ResponseTemplate) {
    Mock::given(method(http_method))
        .and(path(at))
        .respond_with(response)
        .expect(1)
        .mount(dsp)
        .await;
}

/// DSP-API for one forced refresh of 0803: the lookup, the change check and an export that
/// completes at once and is deleted.
async fn dsp_api() -> MockServer {
    let dsp = MockServer::start().await;
    let json = |body| ResponseTemplate::new(200).set_body_json(body);
    let export = |suffix: &str| format!("/v3/projects/{ENCODED_IRI}/exports{suffix}");
    let project = serde_json::json!({"project": {"id": IRI, "shortcode": "0803"}});
    mount(&dsp, "GET", "/admin/projects/shortcode/0803".to_owned(), json(project)).await;
    let changes = serde_json::json!({"resources": [], "count": 0, "asOf": "2026-11-20T01:00:05Z"});
    mount(&dsp, "GET", "/v2/resources/info".to_owned(), json(changes)).await;
    let started = serde_json::json!({"id": "exp1", "status": "in_progress"});
    mount(&dsp, "POST", export(""), ResponseTemplate::new(202).set_body_json(started)).await;
    let completed = serde_json::json!({"id": "exp1", "status": "completed"});
    mount(&dsp, "GET", export("/exp1"), json(completed)).await;
    mount(
        &dsp,
        "GET",
        export("/exp1/download"),
        ResponseTemplate::new(200).set_body_bytes(dump()),
    )
    .await;
    mount(&dsp, "DELETE", export("/exp1"), ResponseTemplate::new(204)).await;
    dsp
}

fn temp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Every file below `dir`, as relative paths with `/` separators, sorted.
fn files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_str().unwrap());
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

fn read_token(root: &Path) -> ReadyToken {
    ReadyToken::parse(&std::fs::read(root.join("0803").join(TOKEN_FILE)).unwrap()).unwrap()
}

#[tokio::test]
async fn test_refresh_of_0803_writes_its_shipped_inputs_and_their_digest() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dsp = dsp_api().await;
    let (sync, media, work, lock) = (temp(), temp(), temp(), temp());
    let display = |path: &Path| Some(path.display().to_string());
    let key_file = lock.path().join("signing-key.pem");
    std::fs::write(&key_file, KEY).unwrap();
    let raw = RawSettings {
        sync_dir: display(sync.path()),
        media_dir: display(media.path()),
        lock_file: display(&lock.path().join("lock")),
        allowlist: Some("0803=prod".to_owned()),
        servers: Some(format!("prod={}", dsp.uri())),
        key_ids: Some("prod=test-kid".to_owned()),
        signing_key_files: Some(format!("prod={}", key_file.display()).into()),
        inputs_dir: Some(INPUTS_DIR.to_owned()),
        work_dir: display(work.path()),
        ..RawSettings::default()
    };
    let bridge = VreBridge::from_raw(raw, &opentelemetry::global::meter(METER_SCOPE));
    assert!(bridge.is_active(), "{:?}", bridge.inactive_reason());

    assert_eq!(bridge.operator_refresh("0803").await, ExitCode::from(0));

    let token = read_token(sync.path());
    assert_eq!(token, read_token(media.path()));
    assert_eq!(
        token.provenance.inputs_digest, INPUTS_DIGEST_0803,
        "inputs/0803 changed: pin the new digest in INPUTS_DIGEST_0803 and write it into \
         areas/access/media/data/0803/.ready and areas/access/sync/data/0803/.ready, \
         whose generation `just check-stand-in-fixture` holds equal to inputs/0803"
    );
    let inputs = Path::new(INPUTS_DIR).join("0803");
    let generation = media.path().join("0803").join(token.provenance.generation.as_str());
    let mut expected = files(&inputs);
    assert!(expected.contains(&"curation.csv".to_owned()), "{expected:?}");
    expected.push("access-rights.json".to_owned());
    expected.sort();
    assert_eq!(files(&generation), expected);
    for file in files(&inputs) {
        let (shipped, written) = (inputs.join(&file), generation.join(&file));
        assert!(
            std::fs::read(&written).unwrap() == std::fs::read(&shipped).unwrap(),
            "{file} differs"
        );
    }
    assert_eq!(std::fs::read_to_string(generation.join("access-rights.json")).unwrap(), RIGHTS);
}
