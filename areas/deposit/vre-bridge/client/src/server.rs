use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::{ClientError, FileError, ServiceIdentity};

const DUMP_ZIP: &str = "dump.zip";

const PROJECT_BY_SHORTCODE: &str = "GET /admin/projects/shortcode/{shortcode}";
const RESOURCES_INFO: &str = "GET /v2/resources/info";
const EXPORT_TRIGGER: &str = "POST /v3/projects/{iri}/exports";
const EXPORT_STATUS: &str = "GET /v3/projects/{iri}/exports/{id}";
const EXPORT_DOWNLOAD: &str = "GET /v3/projects/{iri}/exports/{id}/download";
const EXPORT_DELETE: &str = "DELETE /v3/projects/{iri}/exports/{id}";
const FILE_DOWNLOAD: &str = "GET {file}";

/// The client's time limits. `read` bounds each read of a response body; the others bound a whole
/// call: `request` every call but the dump download (`download`) and a file download (`file`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub read: Duration,
    pub request: Duration,
    pub download: Duration,
    pub file: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(30),
            read: Duration::from_secs(120),
            request: Duration::from_secs(120),
            download: Duration::from_secs(3600),
            file: Duration::from_secs(300),
        }
    }
}

/// The answer of the change check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changes {
    /// The server's `asOf`, as sent.
    pub as_of: String,
    /// The number of resources changed at or after the cutoff.
    pub count: usize,
}

/// A downloaded dump: `dump.zip` in the private work directory, and its size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpZip {
    pub path: PathBuf,
    pub size: u64,
}

/// The answer to an export trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportTrigger {
    Started {
        id: String,
    },
    /// `409 export_exists`: the server-wide export slot holds export `id`, which is the service's
    /// own if its status answers, someone else's if the status is `404`.
    Occupied {
        id: String,
    },
    /// Any other `409`: the slot is taken by an export the answer does not name.
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    /// `409 export_in_progress`: dsp-api does not delete a running export.
    StillRunning,
    /// `404`: no such export, or not the service's.
    Gone,
}

/// One DSP-API server, reached with the bridge's service identity.
pub struct Server {
    /// The configured base URL, verbatim: also the tokens' `aud`.
    base: String,
    base_url: Url,
    /// Where anonymous file downloads may come from, beside `base_url`.
    file_bases: Vec<Url>,
    identity: ServiceIdentity,
    client: reqwest::Client,
    timeouts: Timeouts,
}

#[derive(serde::Deserialize)]
struct ProjectEnvelope {
    project: ProjectDto,
}

#[derive(serde::Deserialize)]
struct ProjectDto {
    id: String,
}

#[derive(serde::Deserialize)]
struct ResourcesInfo {
    #[serde(rename = "asOf")]
    as_of: String,
    count: usize,
}

#[derive(serde::Deserialize)]
struct ExportTask {
    id: String,
    status: String,
}

#[derive(serde::Deserialize)]
struct V3Errors {
    #[serde(default)]
    errors: Vec<V3Error>,
}

#[derive(serde::Deserialize)]
struct V3Error {
    code: String,
    #[serde(default)]
    details: std::collections::HashMap<String, serde_json::Value>,
}

impl Server {
    /// Builds the server's client: no redirects followed, no client-wide total timeout (it would
    /// cut the dump download short). Fails if the process has installed no rustls crypto provider,
    /// where reqwest would panic.
    pub fn new(base: &str, identity: ServiceIdentity, timeouts: Timeouts) -> Result<Self, ClientError> {
        let base_url = parse_base(base)?;
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            return Err(ClientError::NoCryptoProvider);
        }
        let client = reqwest::Client::builder()
            .connect_timeout(timeouts.connect)
            .read_timeout(timeouts.read)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ClientError::request("building the HTTP client"))?;
        Ok(Self {
            base: base.to_owned(),
            base_url,
            file_bases: Vec::new(),
            identity,
            client,
            timeouts,
        })
    }

    /// The project's IRI, looked up without credentials.
    #[tracing::instrument(skip_all)]
    pub async fn project_iri(&self, shortcode: &str) -> Result<String, ClientError> {
        let url = self.url(&format!("/admin/projects/shortcode/{}", enc(shortcode)));
        let response = self.send(PROJECT_BY_SHORTCODE, self.client.get(url)).await?;
        let envelope: ProjectEnvelope =
            json(PROJECT_BY_SHORTCODE, expect(PROJECT_BY_SHORTCODE, response, StatusCode::OK)?).await?;
        Ok(envelope.project.id)
    }

    /// The change check, without credentials: the resources of the project changed at or after
    /// `cutoff`, and the server's `asOf`.
    #[tracing::instrument(skip_all)]
    pub async fn changes(&self, project_iri: &str, cutoff: DateTime<Utc>) -> Result<Changes, ClientError> {
        // Always `Z`: a `+` offset in a query string reads as a space.
        let cutoff = cutoff.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string();
        let request = self
            .client
            .get(self.url("/v2/resources/info"))
            .query(&[("cutoff", cutoff)])
            .header("X-Knora-Accept-Project", project_iri);
        let response = self.send(RESOURCES_INFO, request).await?;
        let info: ResourcesInfo = json(RESOURCES_INFO, expect(RESOURCES_INFO, response, StatusCode::OK)?).await?;
        Ok(Changes { as_of: info.as_of, count: info.count })
    }

    /// Triggers an export of the project's data without its assets.
    #[tracing::instrument(skip_all)]
    pub async fn export(&self, project_iri: &str) -> Result<ExportTrigger, ClientError> {
        let mut url = self.url(&format!("/v3/projects/{}/exports", enc(project_iri)));
        url.set_query(Some("skipAssets=true"));
        let request = self.client.post(url).bearer_auth(self.token()?);
        let response = self.send(EXPORT_TRIGGER, request).await?;
        match response.status() {
            StatusCode::ACCEPTED => {
                let task: ExportTask = json(EXPORT_TRIGGER, response).await?;
                if task.id.is_empty() {
                    return Err(ClientError::Body { endpoint: EXPORT_TRIGGER });
                }
                Ok(ExportTrigger::Started { id: task.id })
            }
            StatusCode::CONFLICT => {
                let occupant = json::<V3Errors>(EXPORT_TRIGGER, response)
                    .await
                    .ok()
                    .and_then(|body| detail_id(&body, "export_exists"));
                Ok(match occupant {
                    Some(id) => ExportTrigger::Occupied { id },
                    None => ExportTrigger::Busy,
                })
            }
            status => Err(ClientError::Status { endpoint: EXPORT_TRIGGER, status: status.as_u16() }),
        }
    }

    /// The export's status; `None` when dsp-api answers `404` (no such export, or not the
    /// service's).
    #[tracing::instrument(skip_all)]
    pub async fn export_status(&self, project_iri: &str, id: &str) -> Result<Option<ExportStatus>, ClientError> {
        let request = self.client.get(self.export_url(project_iri, id, "")).bearer_auth(self.token()?);
        let response = self.send(EXPORT_STATUS, request).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let task: ExportTask = json(EXPORT_STATUS, expect(EXPORT_STATUS, response, StatusCode::OK)?).await?;
        let status = match task.status.as_str() {
            "in_progress" => ExportStatus::InProgress,
            "completed" => ExportStatus::Completed,
            "failed" => ExportStatus::Failed,
            _ => return Err(ClientError::Body { endpoint: EXPORT_STATUS }),
        };
        Ok(Some(status))
    }

    /// Polls every `poll` until the export is `completed`; `failed`, a vanished export or `limit`
    /// passing is an error.
    pub async fn wait_completed(
        &self,
        project_iri: &str,
        id: &str,
        poll: Duration,
        limit: Duration,
    ) -> Result<(), ClientError> {
        match self.wait_finished(project_iri, id, poll, limit).await? {
            ExportStatus::Completed => Ok(()),
            ExportStatus::Failed | ExportStatus::InProgress => Err(ClientError::ExportFailed),
        }
    }

    /// Polls every `poll` until the export is no longer `in_progress` and returns its status; a
    /// vanished export or `limit` passing is an error.
    pub async fn wait_finished(
        &self,
        project_iri: &str,
        id: &str,
        poll: Duration,
        limit: Duration,
    ) -> Result<ExportStatus, ClientError> {
        let finished = async {
            loop {
                match self.export_status(project_iri, id).await? {
                    None => return Err(ClientError::ExportGone),
                    Some(ExportStatus::InProgress) => tokio::time::sleep(poll).await,
                    Some(status) => return Ok(status),
                }
            }
        };
        tokio::time::timeout(limit, finished)
            .await
            .map_err(|_| ClientError::TimedOut { what: "the export", limit })?
    }

    /// Downloads the completed export's zip as `dump.zip` into the private work directory `work`;
    /// [`ClientError::TooLarge`] once it would exceed `max_bytes`. A failed download may leave a
    /// partial file there; it goes with the directory.
    #[tracing::instrument(skip_all)]
    pub async fn download(
        &self,
        project_iri: &str,
        id: &str,
        work: &TempDir,
        max_bytes: u64,
    ) -> Result<DumpZip, ClientError> {
        let limit = self.timeouts.download;
        let into = work.path().join(DUMP_ZIP);
        let request = self
            .client
            .get(self.export_url(project_iri, id, "/download"))
            .bearer_auth(self.token()?);
        let download = async {
            let mut response = request.send().await.map_err(ClientError::request(EXPORT_DOWNLOAD))?;
            if response.status() != StatusCode::OK {
                return Err(ClientError::Status {
                    endpoint: EXPORT_DOWNLOAD,
                    status: response.status().as_u16(),
                });
            }
            let expected = response.content_length();
            let too_large = || ClientError::TooLarge { what: "the dump", limit: max_bytes };
            if expected.is_some_and(|expected| expected > max_bytes) {
                return Err(too_large());
            }
            let mut file = tokio::fs::File::create_new(&into)
                .await
                .map_err(ClientError::io("creating the dump file"))?;
            let mut received = 0u64;
            // The workspace's reqwest has no `stream` feature.
            while let Some(chunk) = response.chunk().await.map_err(ClientError::request(EXPORT_DOWNLOAD))? {
                if received + chunk.len() as u64 > max_bytes {
                    return Err(too_large());
                }
                file.write_all(&chunk).await.map_err(ClientError::io("writing the dump file"))?;
                received += chunk.len() as u64;
            }
            file.flush().await.map_err(ClientError::io("writing the dump file"))?;
            match expected {
                Some(expected) if received != expected => {
                    Err(ClientError::ShortBody { endpoint: EXPORT_DOWNLOAD, received, expected })
                }
                _ => Ok(DumpZip { path: into.clone(), size: received }),
            }
        };
        tokio::time::timeout(limit, download)
            .await
            .map_err(|_| ClientError::TimedOut { what: "the dump download", limit })?
    }

    /// Deletes the export. dsp-api refuses while it runs.
    #[tracing::instrument(skip_all)]
    pub async fn delete_export(&self, project_iri: &str, id: &str) -> Result<DeleteOutcome, ClientError> {
        let request = self
            .client
            .delete(self.export_url(project_iri, id, ""))
            .bearer_auth(self.token()?);
        let response = self.send(EXPORT_DELETE, request).await?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(DeleteOutcome::Deleted),
            StatusCode::NOT_FOUND => Ok(DeleteOutcome::Gone),
            StatusCode::CONFLICT => {
                let body = json::<V3Errors>(EXPORT_DELETE, response).await.ok();
                if body.is_some_and(|body| body.errors.iter().any(|e| e.code == "export_in_progress")) {
                    Ok(DeleteOutcome::StillRunning)
                } else {
                    Err(ClientError::Status {
                        endpoint: EXPORT_DELETE,
                        status: StatusCode::CONFLICT.as_u16(),
                    })
                }
            }
            status => Err(ClientError::Status { endpoint: EXPORT_DELETE, status: status.as_u16() }),
        }
    }

    /// Lets [`download_file`](Self::download_file) fetch below `base` too, beside the server's
    /// base URL: the server's IIIF base, held to the same rules as the base URL.
    ///
    /// # Errors
    ///
    /// [`ClientError::BaseUrl`] as [`new`](Self::new).
    pub fn with_file_base(mut self, base: &str) -> Result<Self, ClientError> {
        self.file_bases.push(parse_base(base)?);
        Ok(self)
    }

    /// Downloads a file without credentials into the new file `into`, in the private work
    /// directory, and returns its size. Only a URL below the base URL or a file base is fetched:
    /// any other is [`ClientError::FileUrl`] before a request is made, and a redirect is not
    /// followed. `401` and `403` are `Refused`: the file is not public. Larger than `max_bytes`
    /// is [`ClientError::TooLarge`]. On any error, a file this call created is deleted.
    #[tracing::instrument(skip_all)]
    pub async fn download_file(&self, url: &str, into: &Path, max_bytes: u64) -> Result<u64, FileError> {
        let url = Url::parse(url)
            .ok()
            .filter(|url| self.is_file_url(url))
            .ok_or(FileError::Failed(ClientError::FileUrl))?;
        let limit = self.timeouts.file;
        let mut created = false;
        let fetched = tokio::time::timeout(limit, self.fetch_file(url, into, max_bytes, &mut created))
            .await
            .unwrap_or(Err(FileError::Failed(ClientError::TimedOut { what: "a file download", limit })));
        if fetched.is_err() && created {
            let _ = tokio::fs::remove_file(into).await;
        }
        fetched
    }

    async fn fetch_file(&self, url: Url, into: &Path, max_bytes: u64, created: &mut bool) -> Result<u64, FileError> {
        let failed = |e| FileError::Failed(ClientError::request(FILE_DOWNLOAD)(e));
        let mut response = self.client.get(url).send().await.map_err(failed)?;
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(FileError::Refused(status.as_u16()));
        }
        if status != StatusCode::OK {
            return Err(FileError::Failed(ClientError::Status {
                endpoint: FILE_DOWNLOAD,
                status: status.as_u16(),
            }));
        }
        let too_large = || FileError::Failed(ClientError::TooLarge { what: "a file", limit: max_bytes });
        let expected = response.content_length();
        if expected.is_some_and(|expected| expected > max_bytes) {
            return Err(too_large());
        }
        let io = |what| move |e| FileError::Failed(ClientError::io(what)(e));
        let mut file = tokio::fs::File::create_new(into)
            .await
            .map_err(io("creating a downloaded file"))?;
        *created = true;
        let mut received = 0u64;
        while let Some(chunk) = response.chunk().await.map_err(failed)? {
            if received + chunk.len() as u64 > max_bytes {
                return Err(too_large());
            }
            file.write_all(&chunk).await.map_err(io("writing a downloaded file"))?;
            received += chunk.len() as u64;
        }
        file.flush().await.map_err(io("writing a downloaded file"))?;
        match expected {
            Some(expected) if received != expected => Err(FileError::Failed(ClientError::ShortBody {
                endpoint: FILE_DOWNLOAD,
                received,
                expected,
            })),
            _ => Ok(received),
        }
    }

    /// An http(s) URL without credentials, query or fragment, on the origin of the base URL or a
    /// file base and below its path. An encoded `/`, `\` or `.` is refused too: a server that
    /// decodes it would resolve a path outside the base.
    fn is_file_url(&self, url: &Url) -> bool {
        let below = |base: &Url| {
            url.origin() == base.origin() && url.path().starts_with(&format!("{}/", base.path().trim_end_matches('/')))
        };
        let path = url.path().to_ascii_lowercase();
        matches!(url.scheme(), "http" | "https")
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && !["%2f", "%5c", "%2e"].iter().any(|encoded| path.contains(encoded))
            && std::iter::once(&self.base_url).chain(&self.file_bases).any(below)
    }

    fn token(&self) -> Result<String, ClientError> {
        self.identity.token(&self.base)
    }

    async fn send(&self, endpoint: &'static str, request: reqwest::RequestBuilder) -> Result<Response, ClientError> {
        request
            .timeout(self.timeouts.request)
            .send()
            .await
            .map_err(ClientError::request(endpoint))
    }

    /// `path` below the base URL's own path; its variable segments are already encoded.
    fn url(&self, path: &str) -> Url {
        let mut url = self.base_url.clone();
        url.set_path(&format!("{}{path}", self.base_url.path().trim_end_matches('/')));
        url
    }

    fn export_url(&self, project_iri: &str, id: &str, suffix: &str) -> Url {
        self.url(&format!("/v3/projects/{}/exports/{}{suffix}", enc(project_iri), enc(id)))
    }
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("base", &self.base)
            .field("identity", &self.identity)
            .field("timeouts", &self.timeouts)
            .finish_non_exhaustive()
    }
}

/// An https URL, or an http one on a loopback host, that can be a base, without credentials, query
/// or fragment.
fn parse_base(base: &str) -> Result<Url, ClientError> {
    let url = Url::parse(base).map_err(|_| ClientError::BaseUrl)?;
    if !(url.scheme() == "https" || (url.scheme() == "http" && is_loopback(&url)))
        || url.cannot_be_a_base()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ClientError::BaseUrl);
    }
    Ok(url)
}

/// Whether the base URL's host is `localhost` or a loopback address: only there is plain http a
/// base, for a test server. Plain http to another host would send the service JWT in cleartext.
fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

/// One path segment, every byte but ASCII letters and digits percent-encoded, as dsp-cli sends
/// IRIs.
fn enc(segment: &str) -> String {
    utf8_percent_encode(segment, NON_ALPHANUMERIC).to_string()
}

fn expect(endpoint: &'static str, response: Response, status: StatusCode) -> Result<Response, ClientError> {
    if response.status() == status {
        Ok(response)
    } else {
        Err(ClientError::Status { endpoint, status: response.status().as_u16() })
    }
}

/// Parses the body. A parse error is not passed on: serde's message can quote the body.
async fn json<T: DeserializeOwned>(endpoint: &'static str, response: Response) -> Result<T, ClientError> {
    let body = response.bytes().await.map_err(ClientError::request(endpoint))?;
    serde_json::from_slice(&body).map_err(|_| ClientError::Body { endpoint })
}

fn detail_id(body: &V3Errors, code: &str) -> Option<String> {
    body.errors
        .iter()
        .find(|e| e.code == code)
        .and_then(|e| e.details.get("id"))
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // This test binary never installs a rustls provider; `tests/server.rs` does.
    #[test]
    fn test_new_without_a_crypto_provider_is_an_error_not_a_panic() {
        let identity = ServiceIdentity::new("svc", "kid", include_str!("../testdata/ed25519-private.pem")).unwrap();

        let result = Server::new("https://api.example.test", identity, Timeouts::default());

        assert!(matches!(result, Err(ClientError::NoCryptoProvider)), "{result:?}");
    }
}
