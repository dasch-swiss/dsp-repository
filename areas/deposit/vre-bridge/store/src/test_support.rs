use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use shared_stand_in::{Generation, Provenance, ReadyToken, TOKEN_FILE};
use tempfile::TempDir;

use crate::{Subtree, Subtrees};

pub const SC: &str = "0803";

pub const GEN_A: &str = "20261118T010203Z-0000000a";
pub const GEN_B: &str = "20261119T010203Z-0000000b";
pub const GEN_C: &str = "20261120T010203Z-0000000c";

pub fn generation(name: &str) -> Generation {
    Generation::parse(name).expect("a well-formed generation")
}

/// A token for `current` that replaced `previous`; `server` tells two provenances apart.
pub fn token_with(current: &str, previous: Option<&str>, server: &str) -> ReadyToken {
    ReadyToken {
        provenance: Provenance {
            generation: generation(current),
            as_of: "2026-11-20T01:02:03.456Z".to_owned(),
            server: server.to_owned(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: format!("sha256:{}", "0".repeat(64)),
        },
        previous_generation: previous.map(generation),
    }
}

pub fn token(current: &str, previous: Option<&str>) -> ReadyToken {
    token_with(current, previous, "prod")
}

/// Two empty subtree roots, `sync` and `media`, in a temporary directory.
pub fn subtrees() -> (TempDir, Subtrees) {
    let dir = TempDir::new().expect("a temporary directory");
    let sync = dir.path().join("sync");
    let media = dir.path().join("media");
    fs::create_dir(&sync).expect("create sync");
    fs::create_dir(&media).expect("create media");
    (dir, Subtrees::new(sync, media))
}

/// Writes a generation with a snapshot and the rights record and commits `token` for it.
pub fn commit(subtrees: &Subtrees, shortcode: &str, token: &ReadyToken) {
    let mut writer = subtrees.begin(shortcode, token.provenance.generation.clone()).expect("begin");
    writer
        .write(Subtree::Sync, "snapshot.nq", b"<a> <b> <c> <g> .\n")
        .expect("write the snapshot");
    writer.write_rights_record().expect("write the rights record");
    writer.commit(token).expect("commit");
}

/// The names of the generation directories in `<root>/<shortcode>/`.
pub fn generations(root: &Path, shortcode: &str) -> BTreeSet<String> {
    let Ok(listing) = fs::read_dir(root.join(shortcode)) else {
        return BTreeSet::new();
    };
    listing
        .map(|entry| entry.expect("an entry"))
        .filter(|entry| entry.file_type().expect("its kind").is_dir())
        .map(|entry| entry.file_name().into_string().expect("a UTF-8 name"))
        .filter(|name| Generation::parse(name).is_some())
        .collect()
}

/// The names of the temporary token files, `.ready.*.tmp`, in `<root>/<shortcode>/`.
pub fn temp_tokens(root: &Path, shortcode: &str) -> BTreeSet<String> {
    let Ok(listing) = fs::read_dir(root.join(shortcode)) else {
        return BTreeSet::new();
    };
    listing
        .map(|entry| entry.expect("an entry").file_name().into_string().expect("a UTF-8 name"))
        .filter(|name| name.starts_with(".ready.") && name.ends_with(".tmp"))
        .collect()
}

pub fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|&name| name.to_owned()).collect()
}

/// The token file's bytes, or `None` if there is none.
pub fn token_bytes(root: &Path, shortcode: &str) -> Option<Vec<u8>> {
    fs::read(root.join(shortcode).join(TOKEN_FILE)).ok()
}

/// Places `token` and an empty generation directory for it, as an earlier bridge would have.
pub fn place(root: &Path, shortcode: &str, token: &ReadyToken) {
    let project = root.join(shortcode);
    fs::create_dir_all(project.join(token.provenance.generation.as_str())).expect("create the generation");
    fs::write(project.join(TOKEN_FILE), token.to_bytes()).expect("write the token");
}
