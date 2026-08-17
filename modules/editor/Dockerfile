# Packaging-only Dockerfile — the Rust build happens in GH Actions.
# Expects pre-built artifacts in the build context:
#   editor-server  (static musl binary)
#   public/        (static assets + the content-hashed app.<hash>.css)
#   data/          (the published project/person/organization set)

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app

COPY --chmod=755 editor-server /app/editor-server
COPY public /app/public
COPY data /app/server/data

ENV RUST_LOG="info"
# Enable backtrace capture for the structured panic hook in editor-server.
# Without this, `Backtrace::capture()` returns `BacktraceStatus::Disabled`
# and panic logs lose the call site — paired with `debug = "line-tables-only"`
# in the release profile so symbols and file:line make it into the backtrace.
ENV RUST_BACKTRACE="1"
ENV EDITOR_SITE_ADDR="0.0.0.0:8080"
ENV EDITOR_ENV="PROD"
ENV EDITOR_PUBLIC_DIR="/app/public"
ENV EDITOR_DATA_DIR="/app/server/data"

EXPOSE 8080

ENTRYPOINT ["./editor-server"]
CMD ["serve"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["./editor-server", "healthcheck"]

# The `:nonroot` tag runs as uid 65532 (distroless NONROOT), not 65534 (nobody).
# This matters once SQLite lands: the mounted data *directory* must be writable
# by 65532, and chowning it to 65534 produces `unable to open database file`,
# which reads like a wrong mount path rather than a permissions problem.
