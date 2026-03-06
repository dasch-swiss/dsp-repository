pub fn validate_against_schema(xml: &str) {
    let xsd_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/oai/handlers/testdata/schemas/validate.xsd");

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    std::io::Write::write_all(&mut tmp, xml.as_bytes()).expect("write temp file");

    let output = std::process::Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(xsd_path)
        .arg(tmp.path())
        .output()
        .expect("xmllint must be available");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Schema validation failed:\n{}", stderr);
    }
}
