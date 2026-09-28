//! The panic hook and the OTel and Pyroscope init that `serve()` runs first.
//!
//! `serve()` owns the call order: [`install_tracing_panic_hook`] runs before
//! [`init_otel`], so a panic during init is still captured.

use opentelemetry_sdk::logs::SdkLoggerProvider;

/// Install a panic hook that emits panics as structured `tracing::error!` events with OTel
/// exception semconv fields, so they reach the same pipeline as the logs.
/// The default stderr hook runs only if structured emission itself panics.
pub(crate) fn install_tracing_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort structured emission.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let message = info.payload_as_str().unwrap_or("<non-string panic payload>");
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            // `Backtrace::capture` respects `RUST_BACKTRACE` / `RUST_LIB_BACKTRACE`.
            // Operators can opt in with `RUST_BACKTRACE=1` at incident time.
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

/// Initialize OTel logs export and the OpenTelemetry tracing subscriber.
///
/// Hold the returned guard for the life of the process; flush and shut down the
/// logger provider (`None` without OTLP log export) on exit.
pub(crate) fn init_otel() -> (Option<SdkLoggerProvider>, init_tracing_opentelemetry::Guard) {
    use std::io::IsTerminal;

    use init_tracing_opentelemetry::TracingConfig;
    use opentelemetry_sdk::logs::SdkLogger;
    use tracing_subscriber::layer::SubscriberExt;

    // OTLP log export only when DPE_ENV=DEV (the default) and an endpoint is set; PROD logs to
    // stdout.
    let logger_provider: Option<SdkLoggerProvider> = if std::env::var("DPE_ENV").as_deref().unwrap_or("DEV") == "DEV"
        && std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
    {
        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .build()
            .expect("failed to build OTLP log exporter");
        Some(SdkLoggerProvider::builder().with_batch_exporter(exporter).build())
    } else {
        None
    };

    // OTEL_* env vars configure export (no-op without an endpoint); RUST_LOG sets the level.
    // Pretty output on a terminal, JSON otherwise or with `NO_COLOR`, so Loki keeps parsing it.
    let human_readable = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let tracing_config = if human_readable {
        // `production()` disables line numbers; pretty output is worth much
        // less without them, so switch them back on for this path only.
        TracingConfig::production().with_pretty_format().with_line_numbers(true)
    } else {
        TracingConfig::production()
    };

    let otel_guard = tracing_config
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
    .tags(vec![("service.namespace", "dpe")])
    .build()
    .expect("failed to build Pyroscope agent");

    tracing::info!(endpoint = %endpoint, "Pyroscope profiling enabled");
    Some(agent.start().expect("failed to start Pyroscope agent"))
}
