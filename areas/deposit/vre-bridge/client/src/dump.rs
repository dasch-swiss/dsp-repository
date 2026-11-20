//! The dump's private unpacking. A dump's `admin.nq` and `permission.nq` hold password hashes and
//! email addresses: only `data/rdf/data.nq` and `data/rdf/ontology-<digits>.nq` ever leave the zip.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zip::ZipArchive;

use crate::ClientError;

/// The name prefix of every private work directory, so that one left by a killed process can be
/// found and deleted.
pub const DUMP_DIR_PREFIX: &str = "vre-bridge-dump-";

const DATA: &str = "data/rdf/data.nq";

/// An unpacked dump: the directory `vre_bridge_lift::snapshot` reads.
#[derive(Debug)]
pub struct DumpDir {
    path: PathBuf,
}

impl DumpDir {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A new directory `vre-bridge-dump-<random>` below `parent`, mode 0700, deleted with the
/// `TempDir`.
pub fn private_work_dir(parent: &Path) -> Result<TempDir, ClientError> {
    let what = "creating the private work directory";
    let dir = tempfile::Builder::new()
        .prefix(DUMP_DIR_PREFIX)
        .permissions(Permissions::from_mode(0o700))
        .tempdir_in(parent)
        .map_err(ClientError::io(what))?;
    fs::set_permissions(dir.path(), Permissions::from_mode(0o700)).map_err(ClientError::io(what))?;
    let mode = fs::symlink_metadata(dir.path())
        .map_err(ClientError::io(what))?
        .permissions()
        .mode();
    if mode & 0o7777 != 0o700 {
        return Err(ClientError::WorkDirMode);
    }
    Ok(dir)
}

/// Unpacks the dump `zip` into `into`. Every entry name of the central directory is checked first:
/// an absolute path, a `..` component, a symlink or two entries of one name fail the whole dump
/// before anything is written. Then only `data.nq` and the ontologies are extracted, by name; the
/// zip's other entries are never read. More than `max_bytes` extracted, as the entries declare or
/// as they inflate, is [`ClientError::TooLarge`].
pub fn unpack(zip: &Path, into: &TempDir, max_bytes: u64) -> Result<DumpDir, ClientError> {
    let too_large = || ClientError::TooLarge { what: "the unpacked dump", limit: max_bytes };
    let file = File::open(zip).map_err(ClientError::io("opening the dump"))?;
    let mut archive = ZipArchive::new(file).map_err(ClientError::Zip)?;
    // Duplicate names collapse to one entry in the archive's index, which could hide a symlink.
    if archive.has_duplicate_names() {
        return Err(ClientError::DuplicateEntry);
    }

    let mut wanted = Vec::new();
    let mut declared = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index_data(index).map_err(ClientError::Zip)?;
        let name = entry.name().map_err(ClientError::Zip)?;
        let unsafe_name = entry.enclosed_name().is_none()
            || name.starts_with(['/', '\\'])
            || name.split(['/', '\\']).any(|part| part == "..");
        if unsafe_name || entry.is_symlink() {
            return Err(ClientError::UnsafeEntry { index });
        }
        if entry.is_file() && is_extracted(&name) {
            declared = declared.saturating_add(entry.size());
            wanted.push((index, name.into_owned()));
        }
    }
    if !wanted.iter().any(|(_, name)| name == DATA) {
        return Err(ClientError::MissingData);
    }
    if declared > max_bytes {
        return Err(too_large());
    }

    let root = into.path().join("dump");
    let what = "writing the unpacked dump";
    fs::create_dir_all(root.join("data").join("rdf")).map_err(ClientError::io(what))?;
    let mut remaining = max_bytes;
    for (index, name) in wanted {
        let entry = archive.by_index(index).map_err(ClientError::Zip)?;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(&name))
            .map_err(ClientError::io(what))?;
        // A declared size can lie: one byte past what is left tells an entry that inflates further.
        let copied = io::copy(&mut entry.take(remaining.saturating_add(1)), &mut out).map_err(ClientError::io(what))?;
        remaining = remaining.checked_sub(copied).ok_or_else(too_large)?;
    }
    Ok(DumpDir { path: root })
}

fn is_extracted(name: &str) -> bool {
    name == DATA
        || name
            .strip_prefix("data/rdf/ontology-")
            .and_then(|rest| rest.strip_suffix(".nq"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    /// Writes a zip of `(name, content)` entries, deflated as dsp-api's are, and returns its path
    /// in a directory of its own.
    fn zip_of(entries: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dump.zip");
        let mut writer = ZipWriter::new(File::create(&path).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        (dir, path)
    }

    fn files_below(dir: &Path) -> Vec<String> {
        let mut found = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for entry in fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    found.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
                }
            }
        }
        found.sort();
        found
    }

    #[test]
    fn test_unpack_extracts_only_data_and_ontologies() {
        let (_src, zip) = zip_of(&[
            ("bag-info.txt", "External-Identifier: x"),
            ("data/rdf/admin.nq", "secret hashes"),
            ("data/rdf/data.nq", "data"),
            ("data/rdf/ontology-1.nq", "onto"),
            ("data/rdf/ontology-x.nq", "not an ontology file name"),
            ("data/rdf/permission.nq", "permissions"),
        ]);
        let work = private_work_dir(&std::env::temp_dir()).unwrap();

        let dump = unpack(&zip, &work, u64::MAX).unwrap();

        assert_eq!(dump.path(), work.path().join("dump"));
        assert_eq!(
            files_below(work.path()),
            ["dump/data/rdf/data.nq", "dump/data/rdf/ontology-1.nq"]
        );
        assert_eq!(fs::read_to_string(dump.path().join(DATA)).unwrap(), "data");
    }

    #[test]
    fn test_unpack_parent_dir_entry_anywhere_fails_and_extracts_nothing() {
        for evil in [
            "../evil.nq",
            "data/rdf/../../../evil.nq",
            "data/../data/rdf/data.nq",
            "/etc/evil.nq",
        ] {
            let (_src, zip) = zip_of(&[
                ("data/rdf/data.nq", "data"),
                (evil, "x"),
                ("data/rdf/ontology-1.nq", "onto"),
            ]);
            let work = private_work_dir(&std::env::temp_dir()).unwrap();

            let error = unpack(&zip, &work, u64::MAX).unwrap_err();

            assert!(matches!(error, ClientError::UnsafeEntry { index: 1 }), "{evil}: {error:?}");
            assert!(files_below(work.path()).is_empty(), "{evil}");
        }
    }

    #[test]
    fn test_unpack_symlink_entry_fails_and_extracts_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("dump.zip");
        let mut writer = ZipWriter::new(File::create(&zip).unwrap());
        writer.start_file("data/rdf/data.nq", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"data").unwrap();
        writer
            .add_symlink("data/rdf/ontology-1.nq", "/etc/passwd", SimpleFileOptions::default())
            .unwrap();
        writer.finish().unwrap();
        let work = private_work_dir(&std::env::temp_dir()).unwrap();

        let error = unpack(&zip, &work, u64::MAX).unwrap_err();

        assert!(matches!(error, ClientError::UnsafeEntry { index: 1 }), "{error:?}");
        assert!(files_below(work.path()).is_empty());
    }

    #[test]
    fn test_unpack_without_data_nq_fails() {
        let (_src, zip) = zip_of(&[("data/rdf/ontology-1.nq", "onto")]);
        let work = private_work_dir(&std::env::temp_dir()).unwrap();

        assert!(matches!(unpack(&zip, &work, u64::MAX), Err(ClientError::MissingData)));
    }

    #[test]
    fn test_unpack_over_the_cap_fails_before_writing_and_counts_only_extracted_entries() {
        let (_src, zip) = zip_of(&[
            ("data/rdf/admin.nq", "much larger than the cap, never extracted"),
            ("data/rdf/data.nq", "12345"),
            ("data/rdf/ontology-1.nq", "678"),
        ]);

        let work = private_work_dir(&std::env::temp_dir()).unwrap();
        let error = unpack(&zip, &work, 7).unwrap_err();
        assert!(matches!(error, ClientError::TooLarge { limit: 7, .. }), "{error:?}");
        assert!(files_below(work.path()).is_empty());

        let work = private_work_dir(&std::env::temp_dir()).unwrap();
        assert!(unpack(&zip, &work, 8).is_ok());
    }

    #[test]
    fn test_private_work_dir_is_0700_named_by_prefix_and_gone_after_drop() {
        let parent = tempfile::tempdir().unwrap();
        let work = private_work_dir(parent.path()).unwrap();
        let path = work.path().to_path_buf();

        let mode = fs::symlink_metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o7777, 0o700);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("vre-bridge-dump-"), "{name}");
        assert_eq!(path.parent().unwrap(), parent.path());

        fs::write(path.join("dump.zip"), b"zip").unwrap();
        drop(work);
        assert!(!path.exists());
    }
}
