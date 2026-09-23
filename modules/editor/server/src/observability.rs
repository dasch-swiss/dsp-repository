//! The panic hook and the OTel and Pyroscope init that `serve()` runs first.
//!
//! `serve()` owns the call order: [`install_tracing_panic_hook`] runs before
//! [`init_otel`], since the hook writes to stderr until [`SUBSCRIBER_READY`] flips.

use std::sync::atomic::{AtomicBool, Ordering};

use opentelemetry_sdk::logs::SdkLoggerProvider;

/// Whether a `tracing` subscriber is installed. Before that, `tracing::error!` is a silent
/// no-op, so the panic hook writes to stderr instead.
pub(crate) static SUBSCRIBER_READY: AtomicBool = AtomicBool::new(false);

/// Install a panic hook that emits panics as structured `tracing::error!` events with OTel
/// exception semconv fields, so they reach the same pipeline as the logs.
///
/// Until [`SUBSCRIBER_READY`] is set it delegates to the default stderr hook: a `catch_unwind`
/// around a no-op `tracing::error!` reports success, so without the flag an init panic would
/// exit 101 with an empty log. After that, the stderr hook runs only if emission itself panics.
pub(crate) fn install_tracing_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !SUBSCRIBER_READY.load(Ordering::Acquire) {
            default_hook(info);
            return;
        }

        // Best-effort structured emission.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let message = info.payload_as_str().unwrap_or("<non-string panic payload>");
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            // `Backtrace::capture` respects `RUST_BACKTRACE` / `RUST_LIB_BACKTRACE`.
            let backtrace = std::backtrace::Backtrace::capture().to_string();

            if let Some(loc) = info.location() {
                tracing::error!(
                    exception.r#type = "panic",
                    exception.message = %message,
                    exception.stacktrace = %backtrace,
                    thread.name = %thread_name,
                    code.filepath = %loc.file(),
                    code.lineno = loc.line(),
                    code.column = loc.column(),
                    "thread panicked"
                );
            } else {
                tracing::error!(
                    exception.r#type = "panic",
                    exception.message = %message,
                    exception.stacktrace = %backtrace,
                    thread.name = %thread_name,
                    "thread panicked"
                );
            }
        }));

        // Fall back to the default stderr hook only if the structured emission
        // itself panicked, so the panic is never silently swallowed.
        if result.is_err() {
            default_hook(info);
        }
    }));
}

/// Initialize OTel logs export (OTLP in DEV only, when an endpoint is set; PROD
/// logs to stdout) and the tracing subscriber (no-op export without
/// `OTEL_EXPORTER_OTLP_ENDPOINT`, level from `RUST_LOG`).
///
/// Hold the returned guard for the life of the process; flush and shut down the
/// logger provider (`None` without OTLP log export) on exit.
pub(crate) fn init_otel(
    config: &crate::config::EditorConfig,
) -> (Option<SdkLoggerProvider>, init_tracing_opentelemetry::Guard) {
    use init_tracing_opentelemetry::TracingConfig;
    use opentelemetry_sdk::logs::SdkLogger;
    use tracing_subscriber::layer::SubscriberExt;

    let logger_provider: Option<SdkLoggerProvider> =
        if config.exports_otlp_logs() && std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_tonic()
                .build()
                .expect("failed to build OTLP log exporter");
            Some(SdkLoggerProvider::builder().with_batch_exporter(exporter).build())
        } else {
            None
        };

    let otel_guard = TracingConfig::production()
        .with_otel_tracer_name(env!("CARGO_PKG_NAME"))
        .init_subscriber_ext(|registry| {
            use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
            let otel_logs_layer: Option<OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>> =
                logger_provider.as_ref().map(OpenTelemetryTracingBridge::new);
            registry.with(otel_logs_layer)
        })
        .expect("failed to initialize OpenTelemetry tracing");

    (logger_provider, otel_guard)
}

/// Start Pyroscope profiling if `PYROSCOPE_ENDPOINT` is set.
pub(crate) fn init_pyroscope(
) -> Option<pyroscope::pyroscope::PyroscopeAgent<pyroscope::pyroscope::PyroscopeAgentRunning>> {
    const PROFILING_SAMPLE_RATE: u32 = 100;

    let endpoint = std::env::var("PYROSCOPE_ENDPOINT").ok()?;

    let backend = pyroscope::backend::pprof_backend(
        pyroscope::backend::PprofConfig { sample_rate: PROFILING_SAMPLE_RATE },
        pyroscope::backend::BackendConfig::default(),
    );

    let agent = pyroscope::pyroscope::PyroscopeAgentBuilder::new(
        &endpoint,
        env!("CARGO_PKG_NAME"),
        PROFILING_SAMPLE_RATE,
        "pyroscope-rs", // matches pyroscope crate's PPROFRS_SPY_NAME
        "2.0.0",        // pyroscope crate version (PPROFRS_SPY_VERSION is private)
        backend,
    )
    .tags(vec![("service.namespace", "editor")])
    .build()
    .expect("failed to build Pyroscope agent");

    tracing::info!(endpoint = %endpoint, "Pyroscope profiling enabled");
    Some(agent.start().expect("failed to start Pyroscope agent"))
}
