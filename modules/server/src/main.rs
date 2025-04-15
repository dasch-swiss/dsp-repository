use core::error::Error;
use core::time::Duration;

use async_stream::stream;
use axum::extract::{Form, Query};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use datastar::prelude::{MergeFragments, ReadSignals};
use datastar::Sse;
use html_api::calculator::{CashFlowRow, DcfForm, DcfTableContext};
use serde::Deserialize;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", get(index))
        .route("/hello-world", get(hello_world))
        .route("/calculator", get(calculator_page))
        .route("/calculator/styles.css", get(calculator_style_css))
        .route("/calculator/calculate", get(handle_calculate));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3333")
        .await
        .unwrap();

    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn index() -> Html<String> {
    Html(html_api::home::index())
}

const MESSAGE: &str = "Hello, world!";

#[derive(Deserialize)]
pub struct Signals {
    pub delay: u64,
}

async fn hello_world(ReadSignals(signals): ReadSignals<Signals>) -> impl IntoResponse {
    Sse(stream! {
        for i in 0..MESSAGE.len() {
            yield MergeFragments::new(format!("<div id='message'>{}</div>", &MESSAGE[0..i + 1])).into();
            tokio::time::sleep(Duration::from_millis(signals.delay)).await;
        }
    })
}

async fn calculator_page() -> Html<String> {
    Html(html_api::calculator::index())
}

async fn calculator_style_css() -> Response {
    (
        [("Content-Type", "text/css")],
        html_api::calculator::style(),
    )
        .into_response()
}

async fn handle_calculate(Query(form): Query<DcfForm>) -> impl IntoResponse {
    let ctx = compute_dcf_table(
        form.fcf,
        form.growth,
        form.discount,
        form.terminal,
        form.years,
    );

    let res = html_api::calculator::result_table(&ctx);
    Sse(stream! {
        yield MergeFragments::new(format!("<div id='intrinsic_value' class='value-display'>Intrinsic Value: ${}</div>", ctx.total_intrinsic_value)).into();
        yield MergeFragments::new(res.clone()).into()
    })
}

pub fn compute_dcf_table(
    fcf: f64,
    growth: f64,
    discount: f64,
    terminal: f64,
    years: u32,
) -> DcfTableContext {
    let mut rows = Vec::new();
    let mut total = 0.0;

    for year in 1..=years {
        let projected_fcf = fcf * (1.0 + growth).powi(year as i32);
        let discounted = projected_fcf / (1.0 + discount).powi(year as i32);

        rows.push(CashFlowRow {
            year: year.to_string(),
            fcf: projected_fcf,
            discounted,
        });

        total += discounted;
    }

    // Terminal calculation
    let last_fcf = fcf * (1.0 + growth).powi(years as i32);
    let terminal_value = last_fcf * (1.0 + terminal) / (discount - terminal);
    let discounted_terminal = terminal_value / (1.0 + discount).powi(years as i32);

    rows.push(CashFlowRow {
        year: "Terminal".to_string(),
        fcf: terminal_value,
        discounted: discounted_terminal,
    });

    total += discounted_terminal;

    DcfTableContext {
        rows,
        total_intrinsic_value: total,
    }
}
