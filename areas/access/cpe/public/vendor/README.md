# Vendored JavaScript Dependencies

Do not edit these files directly. See update process below.

| File | Package | Version | SHA-256 |
|------|---------|---------|---------|
| `datastar.js` | starfederation/datastar | 1.0.2 | `sha256:2837d87acf6ee0ba8e4e63765926c25a98d63883b02f88be194a86b81d3fd24a` |

## Sources

- datastar: <https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.2/bundles/datastar.js>

The Datastar URL is jsdelivr's **`gh/`** (GitHub) route, not `npm/`. The npm package `@starfederation/datastar` is stale: its latest published version is `1.0.0-beta.11`, so an `npm/` URL for any 1.0.x tag returns 404. Bundles are published only as repository files.

DPE vendors the same package in `areas/access/dpe/public/vendor/datastar.js`. Each service keeps its own copy; update the two together.

## Update process

1. Download the new version from the source URL
2. Replace the file in this directory
3. Update the version and SHA-256 in the table above: `shasum -a 256 <file>`
4. Confirm the table matches the files: `just verify-checksums` (also run by `just check`)
5. Run `cargo test -p cpe-web -p cpe-server`: the page assembly's tests read this file, and the host embeds it
   and compares the bytes it serves with it
