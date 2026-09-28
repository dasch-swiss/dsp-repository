//! [`Corpus`]'s cache of all records.
//!
//! All records are loaded from disk once on first access and held in memory
//! for the lifetime of the corpus, mirroring the project cache pattern.
use std::collections::HashMap;
use std::time::Instant;

use shared_metadata::Record;

use super::corpus::Corpus;

static BEARER: &str = "Bearer eyJ0eXAiO...";
// update the bearer to download the records locally, if needed

type RecordCache = HashMap<String, Option<(Instant, Vec<Record>)>>;

impl Corpus {
    /// Return a reference to the cached record list, loading it on first call.
    pub fn all_records(&'static self) -> &'static Vec<Record> {
        self.records_cache.get_or_init(|| self.load_all_records())
    }

    /// The records of one project, in the order [`Corpus::all_records`] holds them.
    ///
    /// [`Corpus::all_records`] is one flat vector of every record of every project
    /// (50,994 today), so filtering it per request puts an O(corpus) scan on
    /// anything that needs one project's records — and a landing page needs them
    /// on every visit. The index is built once, in the shape of `project_cache`'s
    /// index and keyed the same way (upper-cased), over references into that same
    /// vector: one pointer per record, no clone, and the order within a project is
    /// the order the flat vector already has, which is what the OAI paging over
    /// `set=project:{shortcode}` depends on.
    pub fn records_for_shortcode(&'static self, shortcode: &str) -> &'static [&'static Record] {
        const EMPTY: &[&Record] = &[];
        self.record_index().get(&shortcode.to_uppercase()).map_or(EMPTY, Vec::as_slice)
    }

    /// Loads the record list and builds the shortcode index, both of which are
    /// otherwise built on the first request that needs them.
    ///
    /// Two caches, so warming `all_records` alone still leaves the first
    /// landing-page or set-filtered OAI request after a deploy paying for a pass
    /// over all 50,994 records. `dpe-server` calls this once at startup, off the
    /// async runtime.
    pub fn warm(&'static self) {
        self.record_index();
    }

    fn record_index(&'static self) -> &'static HashMap<String, Vec<&'static Record>> {
        self.record_index_cache.get_or_init(|| index_by_shortcode(self.all_records()))
    }

    fn records_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.settings.data_dir).join("records")
    }

    fn load_all_records(&self) -> Vec<Record> {
        let mut cache = load_last_fetched(&self.records_dir());

        // localhost cache warmup code (network or filesystem)
        find_records("0803", &self.records_dir(), &mut cache);
        find_records("0868", &self.records_dir(), &mut cache);
        find_records("081C", &self.records_dir(), &mut cache);

        // Ingress, as in `project_cache`: every consumer of a record — the graph
        // builders, the OAI payloads, the file endpoint — reads this vector, so the
        // ARK host is normalised once here. See `crate::ark`.
        let host = self.settings.ark_resolver_base_url.as_deref();
        cache
            .into_values()
            .flatten()
            .flat_map(|(_, records)| records)
            .map(|mut record| {
                crate::ark::normalise_record(&mut record, host);
                record
            })
            .collect()
    }
}

/// Groups records by upper-cased shortcode, keeping each group in input order.
///
/// Separate from the cache so the corpus-wide test can run it over the
/// committed dumps and compare it against the scan it replaces.
pub fn index_by_shortcode(records: &[Record]) -> HashMap<String, Vec<&Record>> {
    let mut index: HashMap<String, Vec<&Record>> = HashMap::new();
    for record in records {
        index.entry(record.pid.shortcode.to_uppercase()).or_default().push(record);
    }
    index
}

fn records_path(records_dir: &std::path::Path, shortcode: &str) -> std::path::PathBuf {
    records_dir.join(format!("{shortcode}-records.json"))
}

fn save_records(records_dir: &std::path::Path, shortcode: &str, body: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(records_dir)?;
    std::fs::write(records_path(records_dir, shortcode), body)
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

fn find_records(shortcode: &str, records_dir: &std::path::Path, cache: &mut RecordCache) -> Vec<Record> {
    if let Some(Some((_, records))) = cache.get(shortcode) {
        return records.clone();
    }

    match fetch_records(shortcode) {
        Err(e) => tracing::error!(shortcode, error = %e, "failed to fetch records"),
        Ok(body) => match serde_json::from_str::<Vec<Record>>(&body) {
            Err(e) => tracing::error!(shortcode, error = %e, "failed to parse fetched records"),
            Ok(records) => {
                let _ = save_records(records_dir, shortcode, &body);
                cache.insert(shortcode.to_string(), Some((Instant::now(), records.clone())));
                return records;
            }
        },
    }

    Vec::new()
}

fn load_last_fetched(records_dir: &std::path::Path) -> RecordCache {
    let mut map = HashMap::new();
    let entries = match std::fs::read_dir(records_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!(dir = %records_dir.display(), error = %e, "failed to read the records directory");
            return map;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(shortcode) = stem.strip_suffix("-records") else {
            continue;
        };
        // Both failures below are loud: a dump that does not load leaves its whole
        // project silently absent from the site, which is indistinguishable from
        // "that project has no records" unless it is logged.
        let body = match std::fs::read_to_string(&path) {
            Ok(body) => body,
            Err(e) => {
                tracing::error!(shortcode, path = %path.display(), error = %e, "failed to read records file");
                continue;
            }
        };
        let records: Vec<Record> = match serde_json::from_str(&body) {
            Ok(records) => records,
            Err(e) => {
                tracing::error!(
                    shortcode,
                    path = %path.display(),
                    error = %e,
                    "failed to parse records file — serving no records for this project"
                );
                continue;
            }
        };
        let ts = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| Instant::now() - t.elapsed().unwrap_or_default());
        map.insert(shortcode.to_string(), ts.map(|t| (t, records)));
    }
    map
}
