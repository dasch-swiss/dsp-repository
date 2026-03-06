/// Strips the `<responseDate>` line from XML so golden comparisons are stable.
pub fn normalize(xml: &str) -> String {
    xml.lines()
        .filter(|l| !l.trim_start().starts_with("<responseDate>"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Loads a golden file, creating it if absent (first-run mode).
/// Compares and stores the normalized form (without responseDate).
pub fn golden(name: &str, actual: &str) -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/oai/handlers/testdata/golden");
    std::fs::create_dir_all(&dir).expect("create golden dir");
    let path = dir.join(name);
    let normalized = normalize(actual);
    if path.exists() {
        std::fs::read_to_string(&path).expect("read golden file")
    } else {
        std::fs::write(&path, &normalized).expect("write golden file");
        normalized
    }
}

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
