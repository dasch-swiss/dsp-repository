use std::time::Duration;

/// A failed call. `endpoint` is the endpoint's template, never the URL that was called.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the service identity is not usable: {0}")]
    Identity(&'static str),
    #[error(
        "the server's base URL is not an https URL, or an http one on a loopback host, without credentials, query or fragment"
    )]
    BaseUrl,
    #[error("no rustls crypto provider is installed: the binary installs one before `Server::new`")]
    NoCryptoProvider,
    /// Built only from an error passed through `reqwest::Error::without_url`.
    #[error("{endpoint}: {source}")]
    Request {
        endpoint: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{endpoint}: unexpected status {status}")]
    Status { endpoint: &'static str, status: u16 },
    #[error("{endpoint}: the response body is not the expected JSON")]
    Body { endpoint: &'static str },
    #[error("{endpoint}: the body ended after {received} of {expected} bytes")]
    ShortBody { endpoint: &'static str, received: u64, expected: u64 },
    #[error("the export failed on the server")]
    ExportFailed,
    #[error("the export is no longer on the server")]
    ExportGone,
    #[error("{what} did not finish within {limit:?}")]
    TimedOut { what: &'static str, limit: Duration },
    #[error("{what}: {source}")]
    Io {
        what: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("the private work directory is not mode 0700")]
    WorkDirMode,
    #[error("the dump's zip is unreadable: {0}")]
    Zip(#[source] zip::result::ZipError),
    #[error("the dump's zip holds an absolute path, `..` or a symlink (entry {index})")]
    UnsafeEntry { index: usize },
    #[error("the dump's zip holds two entries of the same name")]
    DuplicateEntry,
    #[error("the dump's zip holds no data/rdf/data.nq")]
    MissingData,
    #[error("{what} is larger than the cap of {limit} bytes")]
    TooLarge { what: &'static str, limit: u64 },
    /// The caller has no `Server` for the server id it asked for.
    #[error("no client is configured for the server")]
    NoServer,
}

impl ClientError {
    pub(crate) fn request(endpoint: &'static str) -> impl FnOnce(reqwest::Error) -> ClientError {
        move |source| ClientError::Request { endpoint, source: source.without_url() }
    }

    pub(crate) fn io(what: &'static str) -> impl FnOnce(std::io::Error) -> ClientError {
        move |source| ClientError::Io { what, source }
    }
}

/// A failed anonymous file download.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// The file server answered `401` or `403`: the file is not public.
    #[error("the file server refused an anonymous download with status {0}")]
    Refused(u16),
    #[error(transparent)]
    Failed(ClientError),
}
