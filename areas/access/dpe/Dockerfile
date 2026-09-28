# Packaging-only Dockerfile — the Rust build happens in GH Actions.
# Expects pre-built artifacts in the build context:
#   dpe-server  (static musl binary)
#   public/     (static assets + the content-hashed app.<hash>.css)
#   data/       (JSON data files)

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app

COPY --chmod=755 dpe-server /app/dpe-server
COPY public /app/public
COPY data /app/server/data

ENV RUST_LOG="info"
# Enable backtrace capture for the structured panic hook in dpe-server.
# Without this, `Backtrace::capture()` returns `BacktraceStatus::Disabled`
# and panic logs lose the call site — paired with `debug = "line-tables-only"`
# in the release profile so symbols and file:line make it into the backtrace.
ENV RUST_BACKTRACE="1"
ENV DPE_SITE_ADDR="0.0.0.0:8080"
ENV DPE_ENV="PROD"
ENV DPE_PUBLIC_DIR="/app/public"
ENV DPE_DATA_DIR="/app/server/data"

EXPOSE 8080

ENTRYPOINT ["./dpe-server"]
CMD ["serve"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["./dpe-server", "healthcheck"]
