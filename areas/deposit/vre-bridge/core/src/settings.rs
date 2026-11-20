//! The bridge's settings: [`RawSettings`] as the configuration loader reads them, one optional
//! string per environment variable, and [`Settings::parse`], which checks them all at once. Numbers
//! arrive as text too, so a malformed one is a configuration error of the bridge, which leaves the
//! bridge inactive, and never stops the loader.
//!
//! A list variable is `key=value,…`: entries split on `,`, trimmed, empty ones dropped, each split
//! on its first `=`. An empty or blank string counts as unset, so a deployment may set a variable
//! to nothing. A [`SettingsError`] names the variable and never holds a signing key: it quotes no
//! value and no key longer than a server id, and names an entry of
//! `EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES` by its position only, so neither a key pasted where its
//! path belongs nor a key pasted into another variable is quoted.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::NaiveTime;
use shared_names::is_shortcode;
use shared_stand_in::is_server_id;

const SYNC_DIR: &str = "EDITOR_VRE_BRIDGE_SYNC_DIR";
const MEDIA_DIR: &str = "EDITOR_VRE_BRIDGE_MEDIA_DIR";
const LOCK_FILE: &str = "EDITOR_VRE_BRIDGE_LOCK_FILE";
const ALLOWLIST: &str = "EDITOR_VRE_BRIDGE_ALLOWLIST";
const SERVERS: &str = "EDITOR_VRE_BRIDGE_SERVERS";
const IIIF_BASES: &str = "EDITOR_VRE_BRIDGE_IIIF_BASES";
const KEY_IDS: &str = "EDITOR_VRE_BRIDGE_KEY_IDS";
const SIGNING_KEY_FILES: &str = "EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES";
const NIGHTLY_AT: &str = "EDITOR_VRE_BRIDGE_NIGHTLY_AT";
const EXPORT_POLL_SECS: &str = "EDITOR_VRE_BRIDGE_EXPORT_POLL_SECS";
const REQUEST_TIMEOUT_SECS: &str = "EDITOR_VRE_BRIDGE_REQUEST_TIMEOUT_SECS";
const EXPORT_TIMEOUT_SECS: &str = "EDITOR_VRE_BRIDGE_EXPORT_TIMEOUT_SECS";
const DOWNLOAD_TIMEOUT_SECS: &str = "EDITOR_VRE_BRIDGE_DOWNLOAD_TIMEOUT_SECS";
const FILE_TIMEOUT_SECS: &str = "EDITOR_VRE_BRIDGE_FILE_TIMEOUT_SECS";
const MAX_DUMP_BYTES: &str = "EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES";
const MAX_UNPACKED_BYTES: &str = "EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES";
const MAX_FILE_BYTES: &str = "EDITOR_VRE_BRIDGE_MAX_FILE_BYTES";
const DUMP_RETRIES: &str = "EDITOR_VRE_BRIDGE_DUMP_RETRIES";
const DUMP_RETRY_SECS: &str = "EDITOR_VRE_BRIDGE_DUMP_RETRY_SECS";

const GIB: u64 = 1024 * 1024 * 1024;

/// The settings as the configuration loader reads them: one field per `EDITOR_VRE_BRIDGE_*`
/// variable, `None` when unset. [`Settings::parse`] applies the defaults.
#[derive(Debug, Clone, Default)]
pub struct RawSettings {
    pub sync_dir: Option<String>,
    pub media_dir: Option<String>,
    pub lock_file: Option<String>,
    pub allowlist: Option<String>,
    pub servers: Option<String>,
    pub iiif_bases: Option<String>,
    pub key_ids: Option<String>,
    pub signing_key_files: Option<SigningKeyFiles>,
    pub service_name: Option<String>,
    pub inputs_dir: Option<String>,
    pub work_dir: Option<String>,
    pub nightly_at: Option<String>,
    pub time_zone: Option<String>,
    pub dump_retries: Option<String>,
    pub dump_retry_secs: Option<String>,
    pub request_timeout_secs: Option<String>,
    pub export_timeout_secs: Option<String>,
    pub export_poll_secs: Option<String>,
    pub download_timeout_secs: Option<String>,
    pub file_timeout_secs: Option<String>,
    pub max_dump_bytes: Option<String>,
    pub max_unpacked_bytes: Option<String>,
    pub max_file_bytes: Option<String>,
}

/// `EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES` as read: `id=<absolute path of a PEM PKCS#8 Ed25519
/// private key>,…`. Its `Debug` shows nothing, since a key may be pasted where a path belongs.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SigningKeyFiles(String);

impl From<String> for SigningKeyFiles {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl fmt::Debug for SigningKeyFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SigningKeyFiles(<redacted>)")
    }
}

/// A VRE server's id in the configuration and in the Ready token: `[a-z0-9-]{1,32}`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ServerId(String);

impl ServerId {
    /// The server id `s` spells, or `None` if it is not `[a-z0-9-]{1,32}`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        is_server_id(s).then(|| Self(s.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A project the bridge refreshes, and the VRE server it is refreshed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowlistEntry {
    pub shortcode: String,
    pub server: ServerId,
}

/// The allowlist, sorted by shortcode, each shortcode once. An empty allowlist refreshes nothing
/// and removes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Allowlist(Vec<AllowlistEntry>);

impl Allowlist {
    #[must_use]
    pub fn entries(&self) -> &[AllowlistEntry] {
        &self.0
    }
}

/// What the bridge needs to reach one VRE server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    /// The base URL as configured, without a trailing `/`; verbatim, it is also the JWT's `aud`.
    pub base_url: String,
    /// Where the server's IIIF images are served, if configured.
    pub iiif_base: Option<String>,
    pub key_id: String,
    /// The signing key's file, an absolute path; the bridge reads it at start.
    pub signing_key_file: PathBuf,
}

/// The bridge's settings, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub sync_dir: PathBuf,
    pub media_dir: PathBuf,
    /// On local disk; the nightly-run record is `<lock_file>.nightly`.
    pub lock_file: PathBuf,
    pub allowlist: Allowlist,
    /// The servers the allowlist uses, each with its key id and signing key file. A configured
    /// server no allowlist entry names is not kept.
    pub servers: BTreeMap<ServerId, ServerSettings>,
    /// The JWT's `iss` and `sub`; dsp-api tells the bridge's own exports by it.
    pub service_name: String,
    pub inputs_dir: PathBuf,
    /// The parent of the private dump directories.
    pub work_dir: PathBuf,
    /// The nightly run's local time.
    pub nightly_at: NaiveTime,
    /// An IANA zone name, unchecked here: whoever schedules the nightly run resolves it.
    pub time_zone: String,
    pub dump_retries: u64,
    pub dump_retry: Duration,
    pub request_timeout: Duration,
    pub export_timeout: Duration,
    pub export_poll: Duration,
    pub download_timeout: Duration,
    pub file_timeout: Duration,
    /// The largest dump zip the bridge downloads.
    pub max_dump_bytes: u64,
    /// The most a dump may unpack to.
    pub max_unpacked_bytes: u64,
    /// The largest single file the bridge downloads.
    pub max_file_bytes: u64,
}

/// An entry of a list variable, as an error names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRef {
    /// The entry's key, quoted.
    Key(String),
    /// The entry's 1-based position among the non-empty entries: in a list of secrets, or for a
    /// key too long to be a shortcode or server id, which may be a key pasted into the wrong
    /// variable.
    Position(usize),
}

/// The longest key an error quotes: a server id's length.
const QUOTABLE: usize = 32;

impl EntryRef {
    fn new(key: &str, position: usize, secret: bool) -> Self {
        if secret || key.len() > QUOTABLE {
            Self::Position(position)
        } else {
            Self::Key(key.to_owned())
        }
    }
}

impl fmt::Display for EntryRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(key) => write!(f, "`{key}`"),
            Self::Position(n) => write!(f, "entry {n}"),
        }
    }
}

/// A configuration error. It names the variable and never holds a signing key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("{var} is not set, but an output location is")]
    Missing { var: &'static str },
    #[error("{var}: entry {position} has no `=`")]
    NoEquals { var: &'static str, position: usize },
    #[error("{var}: entry {position} has an empty key or value")]
    Empty { var: &'static str, position: usize },
    #[error("{var}: {entry} is given twice")]
    Duplicate { var: &'static str, entry: EntryRef },
    #[error("{var}: {entry} is not a shortcode (four upper-case hexadecimal digits)")]
    Shortcode { var: &'static str, entry: EntryRef },
    #[error("{var}: {entry} is not a server id ([a-z0-9-]{{1,32}})")]
    ServerId { var: &'static str, entry: EntryRef },
    #[error("{var}: {entry} names a server {SERVERS} does not list")]
    UnknownServer { var: &'static str, entry: EntryRef },
    #[error("{var} has no entry for server `{server}`, which the allowlist uses")]
    NoEntry { var: &'static str, server: ServerId },
    #[error("{var}: the path of {entry} is not absolute")]
    RelativePath { var: &'static str, entry: EntryRef },
    #[error("{var}: the URL of {entry} must be https (http only to a loopback host) with a host and no user, query, fragment, space or trailing /")]
    Url { var: &'static str, entry: EntryRef },
    #[error("{var} is not a time HH:MM")]
    Time { var: &'static str },
    #[error("{var} must be greater than 0")]
    Zero { var: &'static str },
    #[error("{var} is not a whole number")]
    Number { var: &'static str },
    #[error("{IIIF_BASES} has no entry for server `{server}`, from which project {shortcode} ships a file mapping")]
    NoIiifBase { server: ServerId, shortcode: String },
}

impl Settings {
    /// The settings `raw` holds, with defaults for what it leaves unset.
    ///
    /// `Ok(None)` when neither output location is set: the bridge does nothing, which keeps an
    /// image run without them (a public preview) inert.
    ///
    /// # Errors
    ///
    /// The first configuration error found; the bridge then stays inactive.
    pub fn parse(raw: RawSettings) -> Result<Option<Self>, SettingsError> {
        let (sync_dir, media_dir) = match (given(&raw.sync_dir), given(&raw.media_dir)) {
            (None, None) => return Ok(None),
            (Some(sync), Some(media)) => (PathBuf::from(sync), PathBuf::from(media)),
            (None, Some(_)) => return Err(SettingsError::Missing { var: SYNC_DIR }),
            (Some(_), None) => return Err(SettingsError::Missing { var: MEDIA_DIR }),
        };
        let lock_file = PathBuf::from(given(&raw.lock_file).ok_or(SettingsError::Missing { var: LOCK_FILE })?);

        let allowlist = parse_allowlist(given(&raw.allowlist))?;
        let servers = parse_servers(&raw, &allowlist)?;

        let nightly_at =
            parse_hh_mm(given(&raw.nightly_at).unwrap_or("02:00")).ok_or(SettingsError::Time { var: NIGHTLY_AT })?;

        Ok(Some(Self {
            sync_dir,
            media_dir,
            lock_file,
            allowlist,
            servers,
            service_name: given(&raw.service_name).unwrap_or("dsp-repository").to_owned(),
            inputs_dir: PathBuf::from(given(&raw.inputs_dir).unwrap_or("areas/deposit/vre-bridge/inputs")),
            work_dir: given(&raw.work_dir).map_or_else(std::env::temp_dir, PathBuf::from),
            nightly_at,
            time_zone: given(&raw.time_zone).unwrap_or("Europe/Zurich").to_owned(),
            dump_retries: number(&raw.dump_retries, 3, DUMP_RETRIES)?,
            dump_retry: Duration::from_secs(number(&raw.dump_retry_secs, 1200, DUMP_RETRY_SECS)?),
            request_timeout: secs(&raw.request_timeout_secs, 120, REQUEST_TIMEOUT_SECS)?,
            export_timeout: secs(&raw.export_timeout_secs, 3600, EXPORT_TIMEOUT_SECS)?,
            export_poll: secs(&raw.export_poll_secs, 15, EXPORT_POLL_SECS)?,
            download_timeout: secs(&raw.download_timeout_secs, 3600, DOWNLOAD_TIMEOUT_SECS)?,
            file_timeout: secs(&raw.file_timeout_secs, 300, FILE_TIMEOUT_SECS)?,
            max_dump_bytes: positive(&raw.max_dump_bytes, GIB / 4, MAX_DUMP_BYTES)?,
            max_unpacked_bytes: positive(&raw.max_unpacked_bytes, GIB / 2, MAX_UNPACKED_BYTES)?,
            max_file_bytes: positive(&raw.max_file_bytes, GIB / 2, MAX_FILE_BYTES)?,
        }))
    }

    /// Requires an IIIF base for every server from which an allowlisted project ships a file
    /// mapping; `ships_mapping` tells by shortcode, from the shipped inputs.
    ///
    /// # Errors
    ///
    /// [`SettingsError::NoIiifBase`] for the first such project whose server has none.
    pub fn check_iiif_bases(&self, ships_mapping: impl Fn(&str) -> bool) -> Result<(), SettingsError> {
        for entry in self.allowlist.entries() {
            let has_base = self.servers.get(&entry.server).is_some_and(|s| s.iiif_base.is_some());
            if !has_base && ships_mapping(&entry.shortcode) {
                return Err(SettingsError::NoIiifBase {
                    server: entry.server.clone(),
                    shortcode: entry.shortcode.clone(),
                });
            }
        }
        Ok(())
    }
}

/// The value, trimmed, unless it is unset or blank.
fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// The number `value` spells, `default` when it is unset or blank. The value is never quoted.
fn number(value: &Option<String>, default: u64, var: &'static str) -> Result<u64, SettingsError> {
    given(value).map_or(Ok(default), |text| text.parse().map_err(|_| SettingsError::Number { var }))
}

fn positive(value: &Option<String>, default: u64, var: &'static str) -> Result<u64, SettingsError> {
    match number(value, default, var)? {
        0 => Err(SettingsError::Zero { var }),
        n => Ok(n),
    }
}

fn secs(value: &Option<String>, default: u64, var: &'static str) -> Result<Duration, SettingsError> {
    positive(value, default, var).map(Duration::from_secs)
}

/// `HH:MM`, two digits each, 00:00 to 23:59.
fn parse_hh_mm(text: &str) -> Option<NaiveTime> {
    let (hours, minutes) = text.split_once(':')?;
    let two_digits = |s: &str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
    if !two_digits(hours) || !two_digits(minutes) {
        return None;
    }
    NaiveTime::from_hms_opt(hours.parse().ok()?, minutes.parse().ok()?, 0)
}

/// The entries of a list variable as `(key, value)`, keys unique. With `secret`, an error names
/// an entry by its position, never by its text; a value is never quoted.
fn parse_list<'a>(
    var: &'static str,
    text: Option<&'a str>,
    secret: bool,
) -> Result<Vec<(&'a str, &'a str)>, SettingsError> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let entry_texts = text.unwrap_or("").split(',').map(str::trim).filter(|e| !e.is_empty());
    for (index, entry) in entry_texts.enumerate() {
        let position = index + 1;
        let (key, value) = entry.split_once('=').ok_or(SettingsError::NoEquals { var, position })?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || value.is_empty() {
            return Err(SettingsError::Empty { var, position });
        }
        if !seen.insert(key) {
            return Err(SettingsError::Duplicate { var, entry: EntryRef::new(key, position, secret) });
        }
        entries.push((key, value));
    }
    Ok(entries)
}

fn parse_allowlist(text: Option<&str>) -> Result<Allowlist, SettingsError> {
    let mut entries = Vec::new();
    for (index, (shortcode, server)) in parse_list(ALLOWLIST, text, false)?.into_iter().enumerate() {
        if !is_shortcode(shortcode) {
            let entry = EntryRef::new(shortcode, index + 1, false);
            return Err(SettingsError::Shortcode { var: ALLOWLIST, entry });
        }
        let server = ServerId::parse(server)
            .ok_or_else(|| SettingsError::ServerId { var: ALLOWLIST, entry: EntryRef::Key(shortcode.to_owned()) })?;
        entries.push(AllowlistEntry { shortcode: shortcode.to_owned(), server });
    }
    entries.sort_by(|a, b| a.shortcode.cmp(&b.shortcode));
    Ok(Allowlist(entries))
}

/// A list variable keyed by server id, each id one `SERVERS` lists (unless `var` is `SERVERS`).
fn by_server<'a>(
    var: &'static str,
    text: Option<&'a str>,
    secret: bool,
    known: Option<&BTreeMap<ServerId, &str>>,
) -> Result<BTreeMap<ServerId, &'a str>, SettingsError> {
    let mut map = BTreeMap::new();
    for (index, (key, value)) in parse_list(var, text, secret)?.into_iter().enumerate() {
        let entry = || EntryRef::new(key, index + 1, secret);
        let id = ServerId::parse(key).ok_or_else(|| SettingsError::ServerId { var, entry: entry() })?;
        if known.is_some_and(|known| !known.contains_key(&id)) {
            return Err(SettingsError::UnknownServer { var, entry: entry() });
        }
        map.insert(id, value);
    }
    Ok(map)
}

/// `https://`, or `http://` to a loopback host (a test server), a host, and nothing that could
/// carry a credential: the URL is the JWT's `aud` verbatim and is shown in the `Debug` of
/// `Settings`. Plain http to another host would send the service JWT in cleartext.
fn is_base_url(url: &str) -> bool {
    let (rest, plain) = match url.strip_prefix("https://") {
        Some(rest) => (rest, false),
        None => match url.strip_prefix("http://") {
            Some(rest) => (rest, true),
            None => return false,
        },
    };
    let host = rest.split('/').next().unwrap_or_default();
    !host.is_empty()
        && (!plain || is_loopback(host))
        && !rest.ends_with('/')
        && !rest
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control() || matches!(b, b'@' | b'?' | b'#'))
}

/// `localhost` or a loopback address, with or without a port.
fn is_loopback(host_port: &str) -> bool {
    let host = match host_port.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => host_port.split(':').next().unwrap_or_default(),
    };
    host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn parse_servers(
    raw: &RawSettings,
    allowlist: &Allowlist,
) -> Result<BTreeMap<ServerId, ServerSettings>, SettingsError> {
    let urls = by_server(SERVERS, given(&raw.servers), false, None)?;
    let iiif_bases = by_server(IIIF_BASES, given(&raw.iiif_bases), false, Some(&urls))?;
    let key_ids = by_server(KEY_IDS, given(&raw.key_ids), false, Some(&urls))?;
    let key_file_text = raw.signing_key_files.as_ref().map(|files| files.0.as_str());
    let key_files = by_server(SIGNING_KEY_FILES, key_file_text, true, Some(&urls))?;
    // A key pasted where its path belongs is refused here, as a relative path.
    let entries = parse_list(SIGNING_KEY_FILES, key_file_text, true)?;
    if let Some(index) = entries.iter().position(|(_, path)| !Path::new(path).is_absolute()) {
        let entry = EntryRef::Position(index + 1);
        return Err(SettingsError::RelativePath { var: SIGNING_KEY_FILES, entry });
    }
    for (var, map) in [(SERVERS, &urls), (IIIF_BASES, &iiif_bases)] {
        if let Some(id) = map.iter().find_map(|(id, url)| (!is_base_url(url)).then_some(id)) {
            return Err(SettingsError::Url { var, entry: EntryRef::Key(id.to_string()) });
        }
    }

    let mut servers = BTreeMap::new();
    for entry in allowlist.entries() {
        let id = &entry.server;
        if servers.contains_key(id) {
            continue;
        }
        let no_entry = |var| SettingsError::NoEntry { var, server: id.clone() };
        let base_url = urls.get(id).ok_or_else(|| no_entry(SERVERS))?;
        let key_id = key_ids.get(id).ok_or_else(|| no_entry(KEY_IDS))?;
        let signing_key_file = key_files.get(id).ok_or_else(|| no_entry(SIGNING_KEY_FILES))?;
        servers.insert(
            id.clone(),
            ServerSettings {
                base_url: (*base_url).to_owned(),
                iiif_base: iiif_bases.get(id).map(|url| (*url).to_owned()),
                key_id: (*key_id).to_owned(),
                signing_key_file: PathBuf::from(signing_key_file),
            },
        );
    }
    Ok(servers)
}
