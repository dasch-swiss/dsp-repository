//! The VRE servers a refresh talks to, behind a trait so that a test can replace them.

use std::collections::BTreeMap;
use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use vre_bridge_client::{Changes, ClientError, DeleteOutcome, DumpZip, ExportStatus, ExportTrigger, FileError, Server};
use vre_bridge_core::ServerId;

/// DSP-API on each configured VRE server; every call names the allowlist entry's server.
/// The calls are `vre_bridge_client::Server`'s.
#[async_trait]
pub trait Vre: Send + Sync {
    async fn project_iri(&self, server: &ServerId, shortcode: &str) -> Result<String, ClientError>;

    async fn changes(
        &self,
        server: &ServerId,
        project_iri: &str,
        cutoff: DateTime<Utc>,
    ) -> Result<Changes, ClientError>;

    async fn export(&self, server: &ServerId, project_iri: &str) -> Result<ExportTrigger, ClientError>;

    async fn export_status(
        &self,
        server: &ServerId,
        project_iri: &str,
        id: &str,
    ) -> Result<Option<ExportStatus>, ClientError>;

    async fn download(
        &self,
        server: &ServerId,
        project_iri: &str,
        id: &str,
        work: &TempDir,
        max_bytes: u64,
    ) -> Result<DumpZip, ClientError>;

    async fn delete_export(&self, server: &ServerId, project_iri: &str, id: &str)
        -> Result<DeleteOutcome, ClientError>;

    /// An anonymous download of `url`, below one of the server's configured bases, into the new
    /// file `into`; its size.
    async fn download_file(&self, server: &ServerId, url: &str, into: &Path, max_bytes: u64) -> Result<u64, FileError>;
}

fn server<'a>(servers: &'a BTreeMap<ServerId, Server>, id: &ServerId) -> Result<&'a Server, ClientError> {
    servers.get(id).ok_or(ClientError::NoServer)
}

#[async_trait]
impl Vre for BTreeMap<ServerId, Server> {
    async fn project_iri(&self, id: &ServerId, shortcode: &str) -> Result<String, ClientError> {
        server(self, id)?.project_iri(shortcode).await
    }

    async fn changes(&self, id: &ServerId, project_iri: &str, cutoff: DateTime<Utc>) -> Result<Changes, ClientError> {
        server(self, id)?.changes(project_iri, cutoff).await
    }

    async fn export(&self, id: &ServerId, project_iri: &str) -> Result<ExportTrigger, ClientError> {
        server(self, id)?.export(project_iri).await
    }

    async fn export_status(
        &self,
        id: &ServerId,
        project_iri: &str,
        export: &str,
    ) -> Result<Option<ExportStatus>, ClientError> {
        server(self, id)?.export_status(project_iri, export).await
    }

    async fn download(
        &self,
        id: &ServerId,
        project_iri: &str,
        export: &str,
        work: &TempDir,
        max_bytes: u64,
    ) -> Result<DumpZip, ClientError> {
        server(self, id)?.download(project_iri, export, work, max_bytes).await
    }

    async fn delete_export(
        &self,
        id: &ServerId,
        project_iri: &str,
        export: &str,
    ) -> Result<DeleteOutcome, ClientError> {
        server(self, id)?.delete_export(project_iri, export).await
    }

    async fn download_file(&self, id: &ServerId, url: &str, into: &Path, max_bytes: u64) -> Result<u64, FileError> {
        server(self, id)
            .map_err(FileError::Failed)?
            .download_file(url, into, max_bytes)
            .await
    }
}
