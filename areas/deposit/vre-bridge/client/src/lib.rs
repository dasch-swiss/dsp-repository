//! The VRE bridge's DSP-API client (`areas/deposit/ADR-0005`, role `client`): the service
//! identity's tokens, the project lookup, the change check, the export lifecycle, the dump's
//! download and private unpacking, and anonymous file downloads. The service identity's token
//! contract and the export endpoints' answers are DSP-API's (DEV-7936), provisional until it ships.
//!
//! Every error is safe to log as it is: none carries a token, a key, a URL or a response body.
//! The binary installs a rustls crypto provider before [`Server::new`], which fails without one.

#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unreachable)
)]

mod dump;
mod error;
mod identity;
mod server;

pub use dump::{private_work_dir, unpack, DumpDir, DUMP_DIR_PREFIX};
pub use error::{ClientError, FileError};
pub use identity::ServiceIdentity;
pub use server::{Changes, DeleteOutcome, DumpZip, ExportStatus, ExportTrigger, Server, Timeouts};
