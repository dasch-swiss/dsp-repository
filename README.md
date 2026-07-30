# DSP-Repository

Monorepo for the DSP Repository — the long-term archive for humanities research data. Contains the Discovery and Presentation Environment (DPE) and the Mosaic component library (design system).

## Documentation

Full documentation: [dasch-swiss.github.io/dsp-repository](https://dasch-swiss.github.io/dsp-repository/)

Or serve locally:

```bash
just install-requirements
just docs-serve              # http://localhost:3000
```

## Quick Start

```bash
just install-requirements    # Install all dependencies
just dev                     # Run DPE at http://127.0.0.1:4000
just watch-mosaic-playground # Run Mosaic playground
```

## Refreshing the record data

The record dumps in `modules/dpe/server/data/records/` are tracked in git and
refreshed by hand after an API deployment changes the exported metadata. The
token is not stored in the repo — set it yourself, then run the recipe:

```bash
export bearer="Bearer eyJ..."   # a token for api.dasch.swiss
just fetch-records
just validate-data              # check the refreshed dumps
```

To track another project, add its shortcode to `RECORD_SHORTCODES` at the top of
the `justfile`.

## How should I write my commits?

We use [Conventional Commit messages](https://www.conventionalcommits.org/), and
**every commit needs both a type and a scope** — `type(scope): subject`. The most
common:

* `fix(scope):` — a bug fix, a [SemVer](https://semver.org/) patch.
* `feat(scope):` — a new feature, a SemVer minor.
* `feat(scope)!:`, `fix(scope)!:`, etc. — a breaking change (the `!`), a SemVer major.

While the version is below 1.0.0 the scheme shifts down one place — breaking
changes bump the minor, everything else the patch — so the version still says
whether a release broke anything.

A PR lands as **one commit** by default. A CI gate (`commitlint-rs`) enforces the
type, the mandatory scope, and the one-commit cap; run it yourself with
`just commit-lint`. The full type list, the scope vocabulary, what `fix:` means,
and how to opt out of the cap all live in one place —
[docs/src/git-conventions.md](docs/src/git-conventions.md).

## License

Apache 2.0
