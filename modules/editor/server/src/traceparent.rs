use dpe_telemetry::traceparent::is_valid_traceparent;
use opentelemetry::trace::TraceContextExt;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Extract the W3C traceparent from the current OTel span context.
/// Returns None if no valid span context is active.
///
/// Validation is reused from `dpe-telemetry` rather than reimplemented. The
/// crate is not part of the DPE runtime — it reads none of DPE's data or
/// configuration — so sharing it couples the two services on the trace-context
/// format they both have to agree on anyway, not on each other. Its
/// wire-contract modules depend on serde alone; only its `collector` module
/// pulls in axum, opentelemetry, tracing and url.
pub fn extract_traceparent() -> Option<String> {
    let ctx = tracing::Span::current().context();
    let span_ref = ctx.span();
    let sc = span_ref.span_context();
    if sc.is_valid() {
        let tp = format!("00-{}-{}-{:02x}", sc.trace_id(), sc.span_id(), sc.trace_flags().to_u8(),);
        if is_valid_traceparent(&tp) {
            Some(tp)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_traceparent_returns_none_without_span_context() {
        assert!(extract_traceparent().is_none());
    }
}
