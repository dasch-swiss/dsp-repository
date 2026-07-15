/// In-process cache for all records.
///
/// All records are loaded from disk once on first access and held in memory
/// for the lifetime of the server process, mirroring the project cache pattern.
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

use super::record::Record;
use super::utils::get_data_dir;

static BEARER: &str = "Bearer eyJ0eXAiO...";
// update the bearer to download the records locally, if needed

type RecordCache = HashMap<String, Option<(Instant, Vec<Record>)>>;

static RECORDS: OnceLock<Vec<Record>> = OnceLock::new();

/// Return a reference to the cached record list, loading it on first call.
pub fn all_records() -> &'static Vec<Record> {
    RECORDS.get_or_init(load_all_records)
}

fn records_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(get_data_dir()).join("records")
}

fn records_path(shortcode: &str) -> std::path::PathBuf {
    records_dir().join(format!("{shortcode}-records.json"))
}

fn save_records(shortcode: &str, body: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(records_dir())?;
    std::fs::write(records_path(shortcode), body)
}

fn fetch_records(shortcode: &str) -> Result<String, ureq::Error> {
    let agent: ureq::Agent = ureq::config::Config::builder().build().into();
    let mut response = agent
        .post("https://api.dev.dasch.swiss/v3/export/resources/oai")
        .header("Authorization", BEARER)
        .send(ureq::SendBody::from_owned_reader(std::io::Cursor::new(
            serde_json::to_vec(&serde_json::json!({"shortcode": shortcode})).unwrap(),
        )))?;
    response.body_mut().read_to_string()
}

fn find_records(shortcode: &str, cache: &mut RecordCache) -> Vec<Record> {
    if let Some(Some((_, records))) = cache.get(shortcode) {
        return records.clone();
    }

    match fetch_records(shortcode) {
        Err(e) => tracing::error!(shortcode, error = %e, "failed to fetch records"),
        Ok(body) => match serde_json::from_str::<Vec<Record>>(&body) {
            Err(e) => tracing::error!(shortcode, error = %e, "failed to parse fetched records"),
            Ok(records) => {
                let _ = save_records(shortcode, &body);
                cache.insert(shortcode.to_string(), Some((Instant::now(), records.clone())));
                return records;
            }
        },
    }

    Vec::new()
}

fn load_last_fetched() -> RecordCache {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(records_dir()) else {
        return map;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(shortcode) = stem.strip_suffix("-records") else {
            continue;
        };
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        let records: Vec<Record> = serde_json::from_str(&body).unwrap_or_default();
        let ts = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| Instant::now() - t.elapsed().unwrap_or_default());
        map.insert(shortcode.to_string(), ts.map(|t| (t, records)));
    }
    map
}

fn load_all_records() -> Vec<Record> {
    let mut cache = load_last_fetched();

    // localhost cache warmup code (network or filesystem)
    find_records("0803", &mut cache);
    find_records("0868", &mut cache);
    find_records("081C", &mut cache);

    cache.into_values().flatten().flat_map(|(_, records)| records).collect()
}
