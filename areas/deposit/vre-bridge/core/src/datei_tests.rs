use crate::datei::*;

const MAPPING_0001: &str = include_str!("../testdata/datei/0001/file-mapping.json");
const IIIF: &str = "https://iiif.example.test";
const TEXT: &str = "http://www.knora.org/ontology/knora-base#TextFileValue";
const STILL_IMAGE: &str = "http://www.knora.org/ontology/knora-base#StillImageFileValue";
const DOCUMENT: &str = "http://www.knora.org/ontology/knora-base#DocumentFileValue";

fn mapping(files: &[(&str, &str)]) -> FileMapping {
    let files: Vec<_> = files
        .iter()
        .map(|(key, iri)| serde_json::json!({"key": key, "iri": iri}))
        .collect();
    let json = serde_json::json!({"format": "file-mapping", "version": 1, "files": files});
    FileMapping::parse(json.to_string().as_bytes()).unwrap()
}

fn value(class: &str, name: &str) -> Option<Vec<FileValue>> {
    Some(vec![FileValue {
        class: class.to_owned(),
        internal_filename: Some(name.to_owned()),
    }])
}

/// One mapped text file stored as `name`, beside the shipped inputs `shipped`.
fn plan_one_text(name: &str, shipped: &[&str]) -> Result<FilePlan, FileRefusal> {
    let shipped: Vec<String> = shipped.iter().map(|s| (*s).to_owned()).collect();
    plan_files(&mapping(&[("a.xml", "http://rdfh.ch/0001/A")]), "0001", IIIF, &shipped, |_| {
        value(TEXT, name)
    })
}

// ---------------------------------------------------------------- the mapping

#[test]
fn test_parse_shipped_fixture_keeps_the_entries_in_order() {
    let mapping = FileMapping::parse(MAPPING_0001.as_bytes()).unwrap();

    let keys: Vec<_> = mapping.entries().iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "Briefe/Brief 1 an Hüni.xml",
            "odd/edition.odd",
            "css/edition.css",
            "Bilder/Faksimile 1.jpg"
        ]
    );
    assert_eq!(mapping.entries()[0].iri, "http://rdfh.ch/0001/TEI");
}

#[test]
fn test_parse_another_format_version_shape_or_duplicate_key_refused() {
    let cases: [(&str, MappingError); 6] = [
        (r#"{"format":"file-manifest","version":1,"files":[]}"#, MappingError::Format),
        (r#"{"format":"file-mapping","version":2,"files":[]}"#, MappingError::Version),
        (r#"{"format":"file-mapping","version":1}"#, MappingError::Json),
        (r#"[]"#, MappingError::Json),
        (
            r#"{"format":"file-mapping","version":1,"files":[{"key":"","iri":"http://rdfh.ch/0001/A"}]}"#,
            MappingError::Empty(1),
        ),
        (
            r#"{"format":"file-mapping","version":1,"files":[{"key":"a","iri":"http://rdfh.ch/0001/A"},{"key":"a","iri":"http://rdfh.ch/0001/B"}]}"#,
            MappingError::DuplicateKey("a".to_owned()),
        ),
    ];
    for (json, expected) in cases {
        assert_eq!(FileMapping::parse(json.as_bytes()), Err(expected), "{json}");
    }
}

// ---------------------------------------------------------------- resolution

#[test]
fn test_plan_texts_are_downloaded_and_still_images_get_iiif_urls() {
    let mapping = mapping(&[("t.xml", "http://rdfh.ch/0001/T"), ("i.jpg", "http://rdfh.ch/0001/I")]);

    let plan = plan_files(&mapping, "0001", IIIF, &[], |iri| match iri {
        "http://rdfh.ch/0001/T" => value(TEXT, "Abc.xml"),
        _ => value(STILL_IMAGE, "Def.jp2"),
    })
    .unwrap();

    assert_eq!(
        plan.downloads,
        [Download {
            stored: "Abc.xml".to_owned(),
            url: format!("{IIIF}/0001/Abc.xml/file")
        }]
    );
    let targets: Vec<_> = plan.entries.iter().map(|e| e.target.clone()).collect();
    assert_eq!(
        targets,
        [
            Target::Stored("Abc.xml".to_owned()),
            Target::Iiif(format!("{IIIF}/0001/Def.jp2"))
        ]
    );
}

#[test]
fn test_plan_two_keys_of_one_iri_share_one_download() {
    let mapping = mapping(&[("a.xml", "http://rdfh.ch/0001/T"), ("b.xml", "http://rdfh.ch/0001/T")]);

    let plan = plan_files(&mapping, "0001", IIIF, &[], |_| value(TEXT, "Abc.xml")).unwrap();

    assert_eq!(plan.downloads.len(), 1);
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[1].target, Target::Stored("Abc.xml".to_owned()));
}

#[test]
fn test_plan_unselected_unfiled_ambiguous_or_other_class_refused() {
    let iri = "http://rdfh.ch/0001/A";
    let two = Some(vec![
        FileValue {
            class: TEXT.to_owned(),
            internal_filename: Some("a.xml".to_owned()),
        },
        FileValue {
            class: TEXT.to_owned(),
            internal_filename: Some("b.xml".to_owned()),
        },
    ]);
    let unnamed = Some(vec![FileValue { class: TEXT.to_owned(), internal_filename: None }]);
    let cases: [(Option<Vec<FileValue>>, FileRefusal); 5] = [
        (None, FileRefusal::NotSelected(iri.to_owned())),
        (Some(vec![]), FileRefusal::NoFileValue(iri.to_owned())),
        (two, FileRefusal::SeveralFileValues(iri.to_owned())),
        (unnamed, FileRefusal::NoInternalFilename(iri.to_owned())),
        (
            value(DOCUMENT, "a.pdf"),
            FileRefusal::Class { iri: iri.to_owned(), class: DOCUMENT.to_owned() },
        ),
    ];
    for (values, expected) in cases {
        let result = plan_files(&mapping(&[("a", iri)]), "0001", IIIF, &[], |_| values.clone());
        assert_eq!(result, Err(expected));
    }
}

#[test]
fn test_plan_stored_names_reserved_shipped_or_not_file_names_refused() {
    let iri = "http://rdfh.ch/0001/A".to_owned();
    let shipped = ["datei.toml", "file-mapping.json", "narrative/home.md"];
    let cases = [
        ("file-layout.xml", FileRefusal::Collision(iri.clone())),
        ("file-manifest.json", FileRefusal::Collision(iri.clone())),
        ("access-rights.json", FileRefusal::Collision(iri.clone())),
        ("Access-Rights.JSON", FileRefusal::Collision(iri.clone())),
        ("datei.toml", FileRefusal::Collision(iri.clone())),
        ("narrative", FileRefusal::Collision(iri.clone())),
        ("datei.toml/x.xml", FileRefusal::Collision(iri.clone())),
        (".hidden", FileRefusal::Name(iri.clone())),
        ("a/../b", FileRefusal::Name(iri.clone())),
    ];
    for (name, expected) in cases {
        assert_eq!(plan_one_text(name, &shipped), Err(expected), "{name}");
    }
    assert!(plan_one_text("Abc.xml", &shipped).is_ok());
}

#[test]
fn test_plan_two_iris_with_one_stored_name_refused() {
    let mapping = mapping(&[("a.xml", "http://rdfh.ch/0001/A"), ("b.xml", "http://rdfh.ch/0001/B")]);

    let result = plan_files(&mapping, "0001", IIIF, &[], |_| value(TEXT, "Same.xml"));

    assert_eq!(result, Err(FileRefusal::Collision("http://rdfh.ch/0001/B".to_owned())));
}

// ---------------------------------------------------------------- the manifest

#[test]
fn test_manifest_bytes_for_a_fixed_input_equal_the_literal() {
    let entries = [
        ManifestEntry {
            key: "Briefe/Brief 1 an Hüni.xml".to_owned(),
            iri: "http://rdfh.ch/0001/TEI".to_owned(),
            target: Target::Stored("Abc.xml".to_owned()),
        },
        ManifestEntry {
            key: "Bilder/Faksimile 1.jpg".to_owned(),
            iri: "http://rdfh.ch/0001/IMG".to_owned(),
            target: Target::Iiif("https://iiif.example.test/0001/Def.jp2".to_owned()),
        },
    ];

    let bytes = manifest_bytes(&entries).unwrap();

    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        concat!(
            r#"{"format":"file-manifest","version":1,"files":["#,
            r#"{"key":"Briefe/Brief 1 an Hüni.xml","iri":"http://rdfh.ch/0001/TEI","stored":"Abc.xml"},"#,
            r#"{"key":"Bilder/Faksimile 1.jpg","iri":"http://rdfh.ch/0001/IMG","iiif":"https://iiif.example.test/0001/Def.jp2"}]}"#
        )
    );
}
