//! The reading side of the stand-in contract: where a project's data lies in a stand-in subtree,
//! and the Ready token that says which of its generations is current.
//!
//! A subtree root holds `<root>/<shortcode>/`, and in it the Ready token [`TOKEN_FILE`] and one
//! directory per generation. A reader reads the token, then reads only below the generation it
//! names; a project without a token is unknown. Writing a generation or a token is the VRE
//! bridge's, never this crate's.
//!
//! A symbolic link found where a project directory or a token belongs is refused; whatever is read
//! below a generation's directory must be refused the same way by its reader. Nothing assumes more
//! of the file system than that a rename within one directory is atomic: the subtrees may be NFS.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use serde::{Deserialize, Serialize};
use shared_names::is_shortcode;

/// The Ready token's file name in a project's directory. Dot-named, so `is_file_name` never
/// serves it.
pub const TOKEN_FILE: &str = ".ready";

/// The only token format version this crate parses and encodes.
pub const TOKEN_VERSION: u32 = 1;

/// A token is a few hundred bytes; a larger file is refused unread.
const TOKEN_MAX_BYTES: u64 = 64 * 1024;

/// A generation's directory name: `YYYYMMDDTHHMMSSZ-xxxxxxxx`, a UTC time and 8 lowercase hex
/// digits. Only its shape is checked; generations are never compared for order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generation(String);

impl Generation {
    /// The generation `name` spells, or `None` if it does not have the generation's shape.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let b = name.as_bytes();
        let well_formed = b.len() == 25
            && b[..8].iter().all(u8::is_ascii_digit)
            && b[8] == b'T'
            && b[9..15].iter().all(u8::is_ascii_digit)
            && b[15] == b'Z'
            && b[16] == b'-'
            && b[17..].iter().all(is_lower_hex);
        well_formed.then(|| Self(name.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Generation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a project's current generation was made from. The `sync` and `media` tokens of one
/// project hold equal provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub generation: Generation,
    /// T, the time the source's data is as of, as the writer wrote it (RFC 3339, UTC).
    pub as_of: String,
    /// The allowlist's server id the data came from: `[a-z0-9-]{1,32}`.
    pub server: String,
    pub interim_format_version: u32,
    pub transformation_version: u32,
    /// `sha256:` and 64 lowercase hex digits.
    pub inputs_digest: String,
}

/// A project's Ready token: its provenance, and the generation the token replaced, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyToken {
    pub provenance: Provenance,
    pub previous_generation: Option<Generation>,
}

/// Why a project's token could not be taken as its Ready token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    /// The project directory or the token is a symlink or the wrong kind of file
    /// (`InvalidInput`), or reading it failed.
    #[error("the Ready token cannot be read: {0}")]
    Unreadable(io::ErrorKind),
    #[error("the Ready token is invalid: {0}")]
    Invalid(InvalidToken),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidToken {
    /// Not a JSON object with every field of the token's version.
    #[error("not a Ready token: {0}")]
    Malformed(String),
    #[error("token version {0}, not {TOKEN_VERSION}")]
    TokenVersion(u64),
    #[error("a generation does not have the generation's shape")]
    Generation,
    #[error("the server id is not [a-z0-9-]{{1,32}}")]
    Server,
    #[error("the inputs digest is not sha256: and 64 lowercase hex digits")]
    InputsDigest,
}

/// The token as it is encoded. Field order is the canonical order: serde writes a struct's fields
/// in declaration order, so reordering them changes the contract.
#[derive(Serialize, Deserialize)]
struct Encoded {
    token_version: u64,
    generation: String,
    // Required, though `null` for a first generation: serde would default a missing `Option` key.
    #[serde(deserialize_with = "Option::deserialize")]
    previous_generation: Option<String>,
    as_of: String,
    server: String,
    interim_format_version: u32,
    transformation_version: u32,
    inputs_digest: String,
}

/// Read first, so a token of another version is refused as that, whatever its other fields.
#[derive(Deserialize)]
struct EncodedVersion {
    token_version: u64,
}

impl ReadyToken {
    /// The canonical encoding: compact UTF-8 JSON, fields in a fixed order, one trailing newline.
    /// A token's bytes are compared to tell whether it changed.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let p = &self.provenance;
        let encoded = Encoded {
            token_version: u64::from(TOKEN_VERSION),
            generation: p.generation.to_string(),
            previous_generation: self.previous_generation.as_ref().map(Generation::to_string),
            as_of: p.as_of.clone(),
            server: p.server.clone(),
            interim_format_version: p.interim_format_version,
            transformation_version: p.transformation_version,
            inputs_digest: p.inputs_digest.clone(),
        };
        let mut bytes = serde_json::to_vec(&encoded).expect("a struct of strings and integers serializes");
        bytes.push(b'\n');
        bytes
    }

    /// A token from its encoding, in any JSON layout. Fields this version does not know are
    /// ignored.
    ///
    /// # Errors
    ///
    /// [`TokenError::Invalid`] for bytes that are not a token of [`TOKEN_VERSION`], a generation
    /// or server id of the wrong shape, or an inputs digest not `sha256:` and 64 lowercase hex.
    pub fn parse(bytes: &[u8]) -> Result<Self, TokenError> {
        use InvalidToken as Why;
        let malformed = |e: serde_json::Error| TokenError::Invalid(Why::Malformed(e.to_string()));
        let version: EncodedVersion = serde_json::from_slice(bytes).map_err(malformed)?;
        if version.token_version != u64::from(TOKEN_VERSION) {
            return Err(TokenError::Invalid(Why::TokenVersion(version.token_version)));
        }
        let encoded: Encoded = serde_json::from_slice(bytes).map_err(malformed)?;
        let generation = |name: &str| Generation::parse(name).ok_or(TokenError::Invalid(Why::Generation));
        let current = generation(&encoded.generation)?;
        let previous_generation = encoded.previous_generation.as_deref().map(generation).transpose()?;
        if !is_server_id(&encoded.server) {
            return Err(TokenError::Invalid(Why::Server));
        }
        if !is_inputs_digest(&encoded.inputs_digest) {
            return Err(TokenError::Invalid(Why::InputsDigest));
        }
        Ok(Self {
            provenance: Provenance {
                generation: current,
                as_of: encoded.as_of,
                server: encoded.server,
                interim_format_version: encoded.interim_format_version,
                transformation_version: encoded.transformation_version,
                inputs_digest: encoded.inputs_digest,
            },
            previous_generation,
        })
    }
}

fn is_lower_hex(b: &u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f')
}

fn is_server_id(s: &str) -> bool {
    (1..=32).contains(&s.len()) && s.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}

fn is_inputs_digest(s: &str) -> bool {
    s.strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.as_bytes().iter().all(is_lower_hex))
}

/// The Ready token of `shortcode` in the subtree at `root`.
///
/// `Ok(None)` when the project is unknown: `shortcode` fails `is_shortcode` (answered without
/// touching the file system), or `<root>/<shortcode>` or its token does not exist.
///
/// # Errors
///
/// [`TokenError::Unreadable`] when the project directory or the token is a symlink or the wrong
/// kind of file, or reading either fails; [`TokenError::Invalid`] when the token's bytes are not a
/// valid token ([`ReadyToken::parse`]).
pub fn read_token(root: &Path, shortcode: &str) -> Result<Option<ReadyToken>, TokenError> {
    if !is_shortcode(shortcode) {
        return Ok(None);
    }
    let project = root.join(shortcode);
    match file_type(&project)? {
        None => return Ok(None),
        Some(kind) if kind.is_dir() => {}
        Some(_) => return Err(TokenError::Unreadable(io::ErrorKind::InvalidInput)),
    }
    let token = project.join(TOKEN_FILE);
    match file_type(&token)? {
        None => return Ok(None),
        Some(kind) if kind.is_file() => {}
        Some(_) => return Err(TokenError::Unreadable(io::ErrorKind::InvalidInput)),
    }
    match read_capped(&token) {
        Ok(bytes) if bytes.len() as u64 > TOKEN_MAX_BYTES => Err(TokenError::Invalid(InvalidToken::Malformed(
            format!("larger than {TOKEN_MAX_BYTES} bytes"),
        ))),
        Ok(bytes) => ReadyToken::parse(&bytes).map(Some),
        // Removed since it was found: the project is being withdrawn.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TokenError::Unreadable(e.kind())),
    }
}

/// The file's bytes, at most [`TOKEN_MAX_BYTES`] + 1 of them, so an oversized file is told apart.
fn read_capped(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(TOKEN_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// What `path` is, without following a symlink; `None` if nothing is there.
fn file_type(path: &Path) -> Result<Option<fs::FileType>, TokenError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata.file_type())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TokenError::Unreadable(e.kind())),
    }
}

/// The directory of `shortcode`'s `generation` in the subtree at `root`, or `None` if `shortcode`
/// fails `is_shortcode`. Neither its existence nor its kind is checked: a reader refuses a
/// symlink at it and below it itself.
#[must_use]
pub fn generation_dir(root: &Path, shortcode: &str, generation: &Generation) -> Option<PathBuf> {
    is_shortcode(shortcode).then(|| root.join(shortcode).join(generation.as_str()))
}

/// The shortcodes the subtree at `root` knows, sorted: the names of the real directories directly
/// below `root` that pass `is_shortcode` and hold a regular [`TOKEN_FILE`]. Whether the token is
/// valid is not checked; a token whose kind cannot be read, or an entry removed while the root is
/// listed, counts as absent.
///
/// # Errors
///
/// When `root` cannot be listed.
pub fn known_projects(root: &Path) -> io::Result<Vec<String>> {
    let mut known = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !is_shortcode(&name) {
            continue;
        }
        // `DirEntry::file_type` does not follow a symlink.
        let is_dir = match entry.file_type() {
            Ok(kind) => kind.is_dir(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => return Err(e),
        };
        if !is_dir {
            continue;
        }
        let holds_token = fs::symlink_metadata(entry.path().join(TOKEN_FILE)).is_ok_and(|m| m.is_file());
        if holds_token {
            known.push(name);
        }
    }
    known.sort_unstable();
    Ok(known)
}

#[cfg(test)]
mod stand_in_tests;
