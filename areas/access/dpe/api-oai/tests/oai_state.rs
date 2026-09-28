//! `OaiState` carries the base URL through axum state rather than a process-global,
//! so two routers in the same test binary can serve different base URLs without
//! interfering with each other — the hazard a `OnceLock` would reintroduce.

use axum::body::Body;
use axum::extract::Request;
use axum::routing::get;
use axum::Router;
use dpe_api_oai::{oai_handler, OaiState};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn router_with_base_url(base_url: &str) -> Router {
    Router::new()
        .route("/dpe/oai", get(oai_handler))
        .with_state(OaiState::new(base_url))
}

async fn identify_body(app: Router) -> String {
    let req = Request::builder().uri("/dpe/oai?verb=Identify").body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn each_router_emits_its_own_base_url_in_request_and_base_url() {
    let a = router_with_base_url("https://a.example/oai");
    let b = router_with_base_url("https://b.example/oai");

    let xml_a = identify_body(a).await;
    let xml_b = identify_body(b).await;

    assert!(xml_a.contains("<baseURL>https://a.example/oai</baseURL>"), "got: {xml_a}");
    assert!(xml_a.contains("\">https://a.example/oai</request>"), "got: {xml_a}");
    assert!(!xml_a.contains("b.example"), "got: {xml_a}");

    assert!(xml_b.contains("<baseURL>https://b.example/oai</baseURL>"), "got: {xml_b}");
    assert!(xml_b.contains("\">https://b.example/oai</request>"), "got: {xml_b}");
    assert!(!xml_b.contains("a.example"), "got: {xml_b}");
}
