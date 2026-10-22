# dsp-cli is installed as `dsp` from crates.io, with its agent skill in dasch-claude-plugins

The invoked command is **`dsp`**, not `dsp-cli` (the crate name). It is distributed as a crates.io crate only, on its own version line, and made discoverable to AI agents through a Claude Code skill that is not shipped from this repository.

## Binary name

`dsp` is the user-facing command (`[[bin]] name = "dsp"` in `dsp-cli/Cargo.toml`). The crate and most documentation prose use `dsp-cli`. The distinction matters: every character of friction counts in agent prompts that already carry a lot of context, and "dsp" reads naturally in sentences ("run dsp to list projects", "dsp couldn't reach the server"). There is no collision risk: `dsp-tools` ships as `dsp-tools`, not `dsp`.

## Install

- **From crates.io:** `cargo install dsp-cli`. This is the primary and only distribution channel; there are no prebuilt binaries (dsp-cli/ADR-0015 depends on that).
- **Tracking `main` ahead of a release:** `cargo install --git https://github.com/dasch-swiss/dsp-repository dsp-cli`.
- **From a checkout:** `cargo install --path dsp-cli`. A real artefact fails loudly when a build fails, which a symlinked debug binary does not.

A `dasch-swiss` GitHub team is an owner of the crate on crates.io, so publishing never depends on one person's account. The binary name need not be unique on crates.io.

## Versioning and publishing

`dsp-cli` has its own version line. Its `version` in `dsp-cli/Cargo.toml` is a literal that release-please writes, not `version.workspace = true`, and the workspace version does not apply to it. It is a separate release-please package, with a separate release PR and tags of the form `dsp-cli-v<version>`. Publishing to crates.io runs on a published GitHub release whose tag has that prefix (`.github/workflows/publish-dsp-cli.yml`). The mechanics are in `docs/src/deployment.md`.

The published crate packages an allowlist (`include` in `Cargo.toml`): the sources, `docs/topics/*.md` (compiled in with `include_str!`, dsp-cli/ADR-0010), the README, the licence and the changelog.

## Agent discoverability

The agent-facing Claude Code skill is not shipped from this repository. Its home is `dasch-swiss/dasch-claude-plugins` (`misc:dsp-cli`), distributed through the Claude Code plugin marketplace. The repository has no skill file, no `just install` recipe and no symlink step. The skill is a doorway into the tool; `dsp docs <topic>` (dsp-cli/ADR-0010) is the room, and the skill does not replace it.

## Considered Options

- **Ship the skill from this repository** (symlinked into `~/.claude/skills/`): rejected. The plugins repository is the DaSCH-wide distribution mechanism for agent skills, and two copies in two distribution channels drift.
- **Name the binary `dsp-cli`**: rejected. It adds agent-prompt friction and improves no ergonomics.
- **Mention dsp-cli in each DaSCH project's `CLAUDE.md`**: rejected. It means many files to maintain, and they would go stale.
- **A Homebrew tap or a Nix flake**: deferred, not rejected. Their value only emerges with non-Rust users; that is what would bring them back.
- **Inherit the workspace version**: rejected. Every workspace release would bump dsp-cli with no dsp-cli change, and the published version would carry no dsp-cli semantics.

Enforced by: the binary name `dsp`, by the integration tests that run it as `Command::cargo_bin("dsp")` (e.g. `dsp-cli/tests/cli.rs`,
`dsp-cli/tests/docs.rs`); the separate release-please package and the tag-prefix guard on the publish workflow (**static-analysis**);
`cargo publish -p dsp-cli --dry-run` in the publish workflow; the rest by **review**.
