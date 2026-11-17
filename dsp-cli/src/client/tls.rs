//! How this crate starts an HTTP client.

/// A `reqwest` client builder, with `ring` installed as the process's TLS crypto provider.
///
/// reqwest is built with `rustls-no-provider`, so `build()` panics when no provider is
/// installed; every client in this crate starts here. An `Err` from `install_default`
/// means a provider is installed already, which is all a client needs.
pub(crate) fn client_builder() -> reqwest::blocking::ClientBuilder {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::blocking::Client::builder()
}
