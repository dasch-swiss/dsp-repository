//! A refresh of the synthetic DaTEI project `0001`, whose shipped inputs map three text files and
//! a still image: the downloads, the file manifest and the refusals.

use super::*;

const SC: &str = "0001";
const KB: &str = "http://www.knora.org/ontology/knora-base#";
const SHIPPED: [(&str, &str); 3] = [
    ("datei.toml", include_str!("../../core/testdata/datei/0001/datei.toml")),
    (
        "file-layout.xml",
        include_str!("../../core/testdata/datei/0001/file-layout.xml"),
    ),
    (
        "file-mapping.json",
        include_str!("../../core/testdata/datei/0001/file-mapping.json"),
    ),
];
/// The mapped text files: resource, internal file name, content.
const TEXTS: [(&str, &str, &str); 3] = [
    ("http://rdfh.ch/0001/TEI", "TeiAbc.xml", "<TEI/>"),
    ("http://rdfh.ch/0001/ODD", "OddDef.odd", "<schemaSpec/>"),
    ("http://rdfh.ch/0001/CSS", "CssGhi.css", "body {}"),
];
const IMAGE: &str = "http://rdfh.ch/0001/IMG";

/// A file value of `class` on `resource`, the resource's only one.
fn file_value(resource: &str, class: &str, name: &str, permissions: &str) -> String {
    let value = format!("{resource}/values/file");
    // The resource's last path segment, padded: a base64url UUID unique in the dump.
    let uuid = format!("{:A<22}", resource.rsplit('/').next().unwrap());
    format!(
        "<{resource}> <{KB}has{class}> <{value}> <{DATA}> .\n\
         <{value}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{KB}{class}> <{DATA}> .\n\
         <{value}> <{KB}valueHasUUID> \"{uuid}\" <{DATA}> .\n\
         <{value}> <{KB}internalFilename> \"{name}\" <{DATA}> .\n\
         <{value}> <{KB}isDeleted> \"false\"^^<http://www.w3.org/2001/XMLSchema#boolean> <{DATA}> .\n\
         <{value}> <{KB}hasPermissions> \"{permissions}\" <{DATA}> .\n"
    )
}

fn still_image(resource: &str, name: &str) -> String {
    let value = format!("{resource}/values/file");
    let integer = |n| format!("\"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer>");
    format!(
        "{}<{value}> <{KB}dimX> {} <{DATA}> .\n<{value}> <{KB}dimY> {} <{DATA}> .\n",
        file_value(resource, "StillImageFileValue", name, PUBLIC),
        integer(640),
        integer(480)
    )
}

/// The project's dump: three public pages with a text file value each and one with a still
/// image, `change` applied to the lines of the resource it names.
fn datei_dump(change: Option<(&str, &dyn Fn(String) -> String)>) -> Vec<u8> {
    let mut data = String::new();
    for (iri, name, _) in TEXTS {
        data.push_str(&resource(iri, file_value(iri, "TextFileValue", name, PUBLIC), change));
    }
    data.push_str(&resource(IMAGE, still_image(IMAGE, "ImgJkl.jp2"), change));
    dump_zip(&data)
}

fn resource(iri: &str, value: String, change: Option<(&str, &dyn Fn(String) -> String)>) -> String {
    let lines = page(iri, "DATEI", PUBLIC) + &value;
    match change {
        Some((changed, f)) if changed == iri => f(lines),
        _ => lines,
    }
}

/// An environment for `0001` with its shipped inputs and the server's IIIF base below `dsp`.
fn datei_env(dsp: &MockServer) -> Env {
    let mut env = Env::one(SC, dsp);
    for server in env.settings.servers.values_mut() {
        server.iiif_base = Some(format!("{}/iiif", dsp.uri()));
    }
    let inputs = env.inputs.path().join(SC);
    std::fs::create_dir(&inputs).unwrap();
    for (name, content) in SHIPPED {
        std::fs::write(inputs.join(name), content).unwrap();
    }
    env
}

fn file_path(name: &str) -> String {
    format!("/iiif/{SC}/{name}/file")
}

/// Each text file downloads as its content, but `refused`, which answers `status`.
async fn files(dsp: &MockServer, refused: Option<(&str, u16)>) {
    for (_, name, content) in TEXTS {
        let response = match refused {
            Some((which, status)) if which == name => ResponseTemplate::new(status),
            _ => ResponseTemplate::new(200).set_body_string(content),
        };
        mount(dsp, "GET", file_path(name), response).await;
    }
}

async fn file_requests(dsp: &MockServer) -> Vec<Request> {
    let requests = dsp.received_requests().await.unwrap();
    requests.into_iter().filter(|r| r.url.path().starts_with("/iiif/")).collect()
}

#[tokio::test]
async fn test_datei_project_stores_its_texts_and_writes_the_manifest_beside_the_shipped_inputs() {
    let dsp = MockServer::start().await;
    forced(&dsp, SC, datei_dump(None)).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    let outcome = env.refresh(SC, true).await;

    assert_eq!(outcome, Ok(Outcome::Refreshed));
    let generation = env.token(env.media.path(), SC).provenance.generation;
    let root = env.media.path().join(SC).join(generation.as_str());
    for (_, name, content) in TEXTS {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), content, "{name}");
    }
    for (name, content) in SHIPPED {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), content, "{name}");
    }
    assert!(root.join(RIGHTS_RECORD_FILE).is_file());
    let iiif = format!("{}/iiif", dsp.uri());
    let expected = format!(
        concat!(
            r#"{{"format":"file-manifest","version":1,"files":["#,
            r#"{{"key":"Briefe/Brief 1 an Hüni.xml","iri":"http://rdfh.ch/0001/TEI","stored":"TeiAbc.xml"}},"#,
            r#"{{"key":"odd/edition.odd","iri":"http://rdfh.ch/0001/ODD","stored":"OddDef.odd"}},"#,
            r#"{{"key":"css/edition.css","iri":"http://rdfh.ch/0001/CSS","stored":"CssGhi.css"}},"#,
            r#"{{"key":"Bilder/Faksimile 1.jpg","iri":"http://rdfh.ch/0001/IMG","iiif":"{iiif}/0001/ImgJkl.jp2"}}]}}"#
        ),
        iiif = iiif
    );
    assert_eq!(std::fs::read_to_string(root.join("file-manifest.json")).unwrap(), expected);
    let downloads = file_requests(&dsp).await;
    assert_eq!(downloads.len(), 3, "the image is not downloaded");
    assert!(!downloads.iter().any(|r| r.headers.contains_key("authorization")));
}

#[tokio::test]
async fn test_datei_project_with_a_text_the_anonymous_user_may_not_download_is_refused() {
    let dsp = MockServer::start().await;
    forced(&dsp, SC, datei_dump(None)).await;
    files(&dsp, Some(("CssGhi.css", 403))).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Refused(RefusalReason::Unauthorized))
        .await;
}

#[tokio::test]
async fn test_datei_project_with_a_failing_download_fails_and_leaves_its_tokens() {
    let dsp = MockServer::start().await;
    forced(&dsp, SC, datei_dump(None)).await;
    files(&dsp, Some(("OddDef.odd", 500))).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Failed(FailureReason::Server)).await;
}

/// The lift drops a superseded file value; so must the mapping's resolution, or the resource
/// would hold two file values.
#[tokio::test]
async fn test_datei_project_with_a_superseded_text_version_stores_the_current_one() {
    let dsp = MockServer::start().await;
    let tei = "http://rdfh.ch/0001/TEI";
    let superseded = |lines: String| {
        let old = format!("{tei}/values/old");
        let old_lines = file_value(tei, "TextFileValue", "TeiOld.xml", PUBLIC).replace("values/file", "values/old");
        // The old version's UUID must differ from the current one's.
        let old_lines = old_lines.replace("TEIAAAA", "TEIOLDA");
        format!("{lines}{old_lines}<{tei}/values/file> <{KB}previousValue> <{old}> <{DATA}> .\n")
    };
    forced(&dsp, SC, datei_dump(Some((tei, &superseded)))).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    assert_eq!(env.refresh(SC, true).await, Ok(Outcome::Refreshed));

    let paths: Vec<_> = file_requests(&dsp).await.iter().map(|r| r.url.path().to_owned()).collect();
    assert!(paths.contains(&file_path("TeiAbc.xml")), "{paths:?}");
    assert!(!paths.contains(&file_path("TeiOld.xml")), "{paths:?}");
}

/// A file value the lift drops as deleted by `"1"`, its other spelling of true.
#[tokio::test]
async fn test_datei_project_mapping_a_resource_whose_file_value_is_deleted_is_refused() {
    let dsp = MockServer::start().await;
    let deleted = |lines: String| {
        lines.replace(
            &format!("values/file> <{KB}isDeleted> \"false\""),
            &format!("values/file> <{KB}isDeleted> \"1\""),
        )
    };
    forced(&dsp, SC, datei_dump(Some(("http://rdfh.ch/0001/CSS", &deleted)))).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Refused(RefusalReason::FileMapping))
        .await;
    assert!(file_requests(&dsp).await.is_empty());
}

#[tokio::test]
async fn test_datei_project_mapping_a_deleted_resource_is_refused() {
    let dsp = MockServer::start().await;
    let deleted = |lines: String| lines.replacen("\"false\"", "\"true\"", 1);
    forced(&dsp, SC, datei_dump(Some(("http://rdfh.ch/0001/ODD", &deleted)))).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Refused(RefusalReason::FileMapping))
        .await;
    assert!(file_requests(&dsp).await.is_empty());
}

#[tokio::test]
async fn test_datei_project_mapping_a_document_is_refused() {
    let dsp = MockServer::start().await;
    let document = |lines: String| lines.replace("TextFileValue", "DocumentFileValue");
    forced(&dsp, SC, datei_dump(Some(("http://rdfh.ch/0001/ODD", &document)))).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Refused(RefusalReason::FileMapping))
        .await;
    assert!(file_requests(&dsp).await.is_empty());
}

#[tokio::test]
async fn test_datei_project_with_a_restricted_view_text_is_refused_by_the_guard_before_any_download() {
    let dsp = MockServer::start().await;
    let restricted = |lines: String| {
        let value_permissions = format!("values/file> <{KB}hasPermissions> \"{PUBLIC}\"");
        lines.replace(
            &value_permissions,
            &format!("values/file> <{KB}hasPermissions> \"RV knora-admin:UnknownUser\""),
        )
    };
    forced(&dsp, SC, datei_dump(Some(("http://rdfh.ch/0001/TEI", &restricted)))).await;
    files(&dsp, None).await;
    let env = datei_env(&dsp);

    env.assert_ends_untouched(SC, Outcome::Refused(RefusalReason::PublicGuard))
        .await;
    assert!(file_requests(&dsp).await.is_empty());
}
