//! A test binary of its own: no other test can have installed a TLS crypto provider in
//! this process first, so the client below builds only if `HttpDspClient::new` installs one.

use dsp_cli::client::http::HttpDspClient;

#[test]
fn the_dsp_api_client_builds_without_a_provider_installed_by_the_caller() {
    assert!(HttpDspClient::new().is_ok());
}
