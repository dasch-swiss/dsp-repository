# Vendored JavaScript Dependencies

Do not edit these files directly. See update process below.

| File | Package | Version | SHA-256 |
|------|---------|---------|---------|
| `web-vitals-attribution.js` | web-vitals | 5.2.0 | `sha256:580581eec7c7c21eebdf6e4d382389aab3ecf5f42561153d16424b5e35240260` |
| `datastar.js` | @starfederation/datastar | 1.0.0-RC.8 | `sha256:c7f69d2f28ca0d5f4dc9acbdf5cf590bb411d02785c74f86899c611d81c6adcd` |

## Sources

- web-vitals: `npm:web-vitals/dist/web-vitals.attribution.js`
- datastar: `jsdelivr:starfederation/datastar/bundles/datastar.js`

## Update process

1. Download the new version from the source URL
2. Replace the file in this directory
3. Update the version and SHA-256 in the table above: `shasum -a 256 <file>`
4. Verify the application works (`just watch-dpe`, test tab switching and search)
