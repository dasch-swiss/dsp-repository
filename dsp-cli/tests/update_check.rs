// `reqwest::blocking` is safe alongside `#[tokio::test]` (which wiremock
// requires) provided the blocking client is constructed, used, and dropped
// entirely on a plain OS thread — not on the tokio runtime's thread pool.
// We achieve this with `std::thread::spawn` + `JoinHandle::join`: the
// blocking reqwest runtime lives and dies on its own OS thread, so it never
// tries to drop a Tokio runtime from within an async context (which would
// panic). Do not "fix" this by moving the `fetch_latest` call back into the
// async body without also dropping the blocking runtime on a non-async
// thread.
//
// `fetch_latest` takes an explicit `url: &str` parameter (not derived from
// `SPARSE_INDEX_URL`), so these tests point it directly at a wiremock server
// — no need to mock the exact `/ds/p-/dsp-cli` sparse-index path segments.

use dsp_cli::update::fetch_latest;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A realistic newline-delimited crates.io sparse-index body: one JSON
/// object per line, unknown fields present (`name`, `cksum`) alongside the
/// two fields `fetch_latest`/`parse_latest_stable` actually read.
const INDEX_BODY: &str =
    "{\"name\":\"dsp-cli\",\"vers\":\"0.1.0\",\"yanked\":false,\"cksum\":\"aaa\"}
{\"name\":\"dsp-cli\",\"vers\":\"0.1.1\",\"yanked\":false,\"cksum\":\"bbb\"}
{\"name\":\"dsp-cli\",\"vers\":\"0.1.2\",\"yanked\":false,\"cksum\":\"ccc\"}
";

const INDEX_BODY_HIGHEST_YANKED: &str = "{\"name\":\"dsp-cli\",\"vers\":\"0.1.0\",\"yanked\":false}
{\"name\":\"dsp-cli\",\"vers\":\"0.1.1\",\"yanked\":false}
{\"name\":\"dsp-cli\",\"vers\":\"0.1.2\",\"yanked\":true}
";

#[tokio::test]
async fn realistic_index_body_returns_highest_stable_version() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_string(INDEX_BODY))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("fetch_latest should succeed"),
        Some(semver::Version::parse("0.1.2").unwrap())
    );
}

/// The update-check client must send the User-Agent in the PLAIN form
/// `dsp-cli/<version>` — no `(github.com/dasch-swiss/dsp-incubator)` suffix
/// (see plan 033). The mock only matches on that exact header value, so a
/// successful `Ok(...)` result proves the plain form was actually sent.
#[tokio::test]
async fn user_agent_is_sent_in_plain_form() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .and(header(
            "user-agent",
            format!("dsp-cli/{}", env!("CARGO_PKG_VERSION")).as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(INDEX_BODY))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("fetch_latest should succeed with the plain User-Agent form"),
        Some(semver::Version::parse("0.1.2").unwrap())
    );
}

#[tokio::test]
async fn yanked_highest_entry_falls_back_to_next_highest_stable() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_string(INDEX_BODY_HIGHEST_YANKED))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("fetch_latest should succeed"),
        Some(semver::Version::parse("0.1.1").unwrap())
    );
}

#[tokio::test]
async fn not_found_returns_ok_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("404 must be Ok(None), not an error"),
        None,
        "a 404 (unpublished crate name) must yield Ok(None), not an error"
    );
}

#[tokio::test]
async fn server_error_returns_ok_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("500 must be Ok(None), not an error"),
        None,
        "a 5xx server error must yield Ok(None), not an error"
    );
}

#[tokio::test]
async fn garbage_body_returns_ok_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ds/p-/dsp-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not an index at all"))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/ds/p-/dsp-cli", server.uri());
    let result = std::thread::spawn(move || fetch_latest(&url))
        .join()
        .expect("blocking thread should not panic");

    assert_eq!(
        result.expect("garbage body must be Ok(None), not an error"),
        None,
        "a 200 response with an unparseable body must yield Ok(None), \
         mirroring parse_latest_stable's own garbage-body behaviour"
    );
}
