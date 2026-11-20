//! The dump's private unpacking. A dump's `admin.nq` and `permission.nq` hold password hashes and
//! email addresses: only `data/rdf/data.nq` and `data/rdf/ontology-<digits>.nq` ever leave the zip.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io;
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
/// zip's other entries are never read.
pub fn unpack(zip: &Path, into: &TempDir) -> Result<DumpDir, ClientError> {
    let file = File::open(zip).map_err(ClientError::io("opening the dump"))?;
    let mut archive = ZipArchive::new(file).map_err(ClientError::Zip)?;
    // Duplicate names collapse to one entry in the archive's index, which could hide a symlink.
    if archive.has_duplicate_names() {
        return Err(ClientError::DuplicateEntry);
    }

    let mut wanted = Vec::new();
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
            wanted.push((index, name.into_owned()));
        }
    }
    if !wanted.iter().any(|(_, name)| name == DATA) {
        return Err(ClientError::MissingData);
    }

    let root = into.path().join("dump");
    let what = "writing the unpacked dump";
    fs::create_dir_all(root.join("data").join("rdf")).map_err(ClientError::io(what))?;
    for (index, name) in wanted {
        let mut entry = archive.by_index(index).map_err(ClientError::Zip)?;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(&name))
            .map_err(ClientError::io(what))?;
        io::copy(&mut entry, &mut out).map_err(ClientError::io(what))?;
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

        let dump = unpack(&zip, &work).unwrap();

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

            let error = unpack(&zip, &work).unwrap_err();

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

        let error = unpack(&zip, &work).unwrap_err();

        assert!(matches!(error, ClientError::UnsafeEntry { index: 1 }), "{error:?}");
        assert!(files_below(work.path()).is_empty());
    }

    #[test]
    fn test_unpack_without_data_nq_fails() {
        let (_src, zip) = zip_of(&[("data/rdf/ontology-1.nq", "onto")]);
        let work = private_work_dir(&std::env::temp_dir()).unwrap();

        assert!(matches!(unpack(&zip, &work), Err(ClientError::MissingData)));
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
