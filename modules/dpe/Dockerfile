# Packaging-only Dockerfile — the Rust build happens in GH Actions.
# Expects pre-built artifacts in the build context:
#   dpe-server  (static musl binary)
#   site/       (JS, CSS, static assets)
#   data/       (JSON data files)

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app

COPY --chmod=755 dpe-server /app/dpe-server
COPY site /app/site
COPY data /app/server/data

ENV RUST_LOG="info"
# Enable backtrace capture for the structured panic hook in dpe-server.
# Without this, `Backtrace::capture()` returns `BacktraceStatus::Disabled`
# and panic logs lose the call site — paired with `debug = "line-tables-only"`
# in the release profile so symbols and file:line make it into the backtrace.
ENV RUST_BACKTRACE="1"
ENV LEPTOS_OUTPUT_NAME="dpe"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_ENV="PROD"
ENV DPE_DATA_DIR="/app/server/data"

EXPOSE 8080

ENTRYPOINT ["./dpe-server"]
CMD ["serve"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["./dpe-server", "healthcheck"]
