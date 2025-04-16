use async_stream::stream;
use core::time::Duration;
use std::sync::Arc;
use axum::extract::State;
use axum::response::IntoResponse;
use datastar::prelude::{MergeFragments, ReadSignals};
use datastar::Sse;
use serde::Deserialize;
use crate::app_state::AppState;

const MESSAGE: &str = "Hello, world!";

#[derive(Deserialize)]
pub struct Signals {
    pub delay: u64,
}

/// GET /hello_world — returns hello world fragments through SSE
pub(crate) async fn hello_world_handler(
    State(_state): State<Arc<AppState>>,
    ReadSignals(signals): ReadSignals<Signals>
) -> impl IntoResponse {
    Sse(stream! {
        for i in 0..MESSAGE.len() {
            yield MergeFragments::new(format!("<div id='message'>{}</div>", &MESSAGE[0..i + 1])).into();
            tokio::time::sleep(Duration::from_millis(signals.delay)).await;
        }
    })
}