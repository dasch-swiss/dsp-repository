//! DaTEI's files: the file mapping a project ships, its resolution against the dump's file values,
//! and the file manifest the bridge writes into the project's `media` generation. Both shapes are
//! DaTEI's (ADR-0009); a change to either is agreed with DaTEI first.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shared_names::is_file_name;

use crate::RESERVED_NAMES;

/// The file mapping's name among a project's shipped inputs.
pub const FILE_MAPPING: &str = "file-mapping.json";

/// The file manifest's name in a `media` generation.
pub const FILE_MANIFEST: &str = "file-manifest.json";

/// DaTEI's file layout, a shipped input no stored file may take even where a project does not
/// ship it; the other names no stored file may take are [`RESERVED_NAMES`].
const FILE_LAYOUT: &str = "file-layout.xml";

const TEXT_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#TextFileValue";
const STILL_IMAGE_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#StillImageFileValue";

/// One entry of the file mapping: DaTEI's logical path, and the IRI of the resource that holds
/// the file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MappingEntry {
    pub key: String,
    pub iri: String,
}

/// A project's file mapping, its entries in their order, each key once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMapping(Vec<MappingEntry>);

/// A file mapping the bridge cannot use: a shipped input's error. A key is quoted; an IRI or the
/// JSON parser's message is not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MappingError {
    #[error("{FILE_MAPPING} is not a JSON object of the file-mapping shape")]
    Json,
    #[error("{FILE_MAPPING} is not of the format `file-mapping`")]
    Format,
    #[error("{FILE_MAPPING} is not of version 1")]
    Version,
    #[error("{FILE_MAPPING}: entry {0} has an empty key or IRI")]
    Empty(usize),
    #[error("{FILE_MAPPING}: the key `{0}` is given twice")]
    DuplicateKey(String),
}

#[derive(Deserialize)]
struct RawMapping {
    format: String,
    version: u64,
    files: Vec<MappingEntry>,
}

impl FileMapping {
    /// The mapping `bytes` hold.
    ///
    /// # Errors
    ///
    /// [`MappingError`] for anything but `{"format":"file-mapping","version":1,"files":[…]}`
    /// with non-empty keys and IRIs, each key once.
    pub fn parse(bytes: &[u8]) -> Result<Self, MappingError> {
        let raw: RawMapping = serde_json::from_slice(bytes).map_err(|_| MappingError::Json)?;
        if raw.format != "file-mapping" {
            return Err(MappingError::Format);
        }
        if raw.version != 1 {
            return Err(MappingError::Version);
        }
        let mut keys = BTreeSet::new();
        for (index, entry) in raw.files.iter().enumerate() {
            if entry.key.is_empty() || entry.iri.is_empty() {
                return Err(MappingError::Empty(index + 1));
            }
            if !keys.insert(entry.key.as_str()) {
                return Err(MappingError::DuplicateKey(entry.key.clone()));
            }
        }
        Ok(Self(raw.files))
    }

    #[must_use]
    pub fn entries(&self) -> &[MappingEntry] {
        &self.0
    }

    /// The mapped IRIs, each once.
    #[must_use]
    pub fn iris(&self) -> BTreeSet<String> {
        self.0.iter().map(|file| file.iri.clone()).collect()
    }
}

/// A current file value of a mapped resource, as the dump records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileValue {
    /// The file value's class IRI.
    pub class: String,
    /// Its `knora-base:internalFilename`, if it has exactly one.
    pub internal_filename: Option<String>,
}

/// What the manifest records for a key: the stored name of a downloaded file, or an image's IIIF
/// URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Stored(String),
    Iiif(String),
}

/// One entry of the file manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub key: String,
    pub iri: String,
    pub target: Target,
}

/// A text file to download without credentials and store under its stored name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub stored: String,
    pub url: String,
}

/// The resolved mapping: the manifest's entries in the mapping's order, and each download once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePlan {
    pub entries: Vec<ManifestEntry>,
    pub downloads: Vec<Download>,
}

/// Why a project's mapped files refuse the project. Each names the mapped IRI; none quotes a
/// stored name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FileRefusal {
    #[error("the mapped resource {0} is not among the dump's current, public resources")]
    NotSelected(String),
    #[error("the mapped resource {0} has no current file value")]
    NoFileValue(String),
    #[error("the mapped resource {0} has more than one current file value")]
    SeveralFileValues(String),
    #[error("the mapped resource {iri} holds a file value of class {class}, neither a text file nor a still image")]
    Class { iri: String, class: String },
    #[error("the file value of the mapped resource {0} has no single internal file name")]
    NoInternalFilename(String),
    #[error("the internal file name of the mapped resource {0} is not a name a project's file may have")]
    Name(String),
    #[error("the internal file name of the mapped resource {0} collides with a shipped input, a reserved name or another stored file")]
    Collision(String),
}

/// Resolves `mapping` for project `shortcode`: each mapped IRI's current file value, from
/// `file_values` (`None` for a resource outside the lift's selection), becomes a download stored
/// under its internal file name (a text file) or an IIIF URL (a still image). Two keys of one IRI
/// share one download. `shipped` holds the relative paths of the project's shipped inputs.
///
/// # Errors
///
/// [`FileRefusal`] for the first mapped IRI that is not resolved to a text file or still image,
/// or whose stored name fails `is_file_name` or collides.
pub fn plan_files(
    mapping: &FileMapping,
    shortcode: &str,
    iiif_base: &str,
    shipped: &[String],
    file_values: impl Fn(&str) -> Option<Vec<FileValue>>,
) -> Result<FilePlan, FileRefusal> {
    let mut names = Names::new(shipped);
    let mut resolved: BTreeMap<&str, Target> = BTreeMap::new();
    let mut plan = FilePlan { entries: Vec::new(), downloads: Vec::new() };
    for MappingEntry { key, iri } in mapping.entries() {
        let target = match resolved.get(iri.as_str()) {
            Some(target) => target.clone(),
            None => {
                let (class, name) = one_file_value(iri, file_values(iri))?;
                let target = match class.as_str() {
                    TEXT_FILE_VALUE => {
                        if !names.take(&name) {
                            return Err(FileRefusal::Collision(iri.clone()));
                        }
                        let url = format!("{iiif_base}/{shortcode}/{name}/file");
                        plan.downloads.push(Download { stored: name.clone(), url });
                        Target::Stored(name)
                    }
                    STILL_IMAGE_FILE_VALUE => Target::Iiif(format!("{iiif_base}/{shortcode}/{name}")),
                    _ => return Err(FileRefusal::Class { iri: iri.clone(), class }),
                };
                resolved.insert(iri, target.clone());
                target
            }
        };
        plan.entries.push(ManifestEntry { key: key.clone(), iri: iri.clone(), target });
    }
    Ok(plan)
}

/// The class and internal file name of the one current file value, which must be a file name.
fn one_file_value(iri: &str, values: Option<Vec<FileValue>>) -> Result<(String, String), FileRefusal> {
    let mut values = values.ok_or_else(|| FileRefusal::NotSelected(iri.to_owned()))?;
    let value = match values.len() {
        0 => return Err(FileRefusal::NoFileValue(iri.to_owned())),
        1 => values.remove(0),
        _ => return Err(FileRefusal::SeveralFileValues(iri.to_owned())),
    };
    let name = value
        .internal_filename
        .ok_or_else(|| FileRefusal::NoInternalFilename(iri.to_owned()))?;
    // A still image's name goes into a URL path: held to the same rule.
    if !is_file_name(&name) {
        return Err(FileRefusal::Name(iri.to_owned()));
    }
    Ok((value.class, name))
}

/// The names a `media` generation's root already holds, compared in ASCII lower case, since a
/// case-insensitive file system would map two spellings onto one file.
struct Names {
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
}

impl Names {
    fn new(shipped: &[String]) -> Self {
        let mut names = Self { files: BTreeSet::new(), dirs: BTreeSet::new() };
        let reserved = RESERVED_NAMES.iter().copied().chain([FILE_LAYOUT]);
        for name in reserved.chain(shipped.iter().map(String::as_str)) {
            names.insert(&name.to_ascii_lowercase());
        }
        names
    }

    /// Takes `name` for a file unless it, or a directory it would need, is taken.
    fn take(&mut self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        let free = !self.files.contains(&name)
            && !self.dirs.contains(&name)
            && ancestors(&name).all(|dir| !self.files.contains(dir));
        if free {
            self.insert(&name);
        }
        free
    }

    fn insert(&mut self, name: &str) {
        self.dirs.extend(ancestors(name).map(str::to_owned));
        self.files.insert(name.to_owned());
    }
}

/// `a`, `a/b` for `a/b/c`.
fn ancestors(name: &str) -> impl Iterator<Item = &str> {
    name.match_indices('/').map(|(at, _)| &name[..at])
}

#[derive(Serialize)]
struct Manifest<'a> {
    format: &'static str,
    version: u32,
    files: Vec<ManifestLine<'a>>,
}

#[derive(Serialize)]
struct ManifestLine<'a> {
    key: &'a str,
    iri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iiif: Option<&'a str>,
}

/// The file manifest's bytes: compact JSON, `{"format":"file-manifest","version":1,"files":[…]}`,
/// each entry `{"key","iri","stored"}` or `{"key","iri","iiif"}`, in the order given.
///
/// # Errors
///
/// `serde_json`'s.
pub fn manifest_bytes(entries: &[ManifestEntry]) -> Result<Vec<u8>, serde_json::Error> {
    let files = entries
        .iter()
        .map(|entry| {
            let (stored, iiif) = match &entry.target {
                Target::Stored(name) => (Some(name.as_str()), None),
                Target::Iiif(url) => (None, Some(url.as_str())),
            };
            ManifestLine { key: &entry.key, iri: &entry.iri, stored, iiif }
        })
        .collect();
    let manifest = Manifest { format: "file-manifest", version: 1, files };
    serde_json::to_vec(&manifest)
}
