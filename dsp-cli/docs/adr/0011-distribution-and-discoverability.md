# Binary name, local install, and agent discoverability

The invoked command is named **`dsp`**, not `dsp-cli` (the crate name). For the personal-project phase,
the tool is installed locally via `cargo install --path .` and made discoverable to AI agents through a Claude Code skill that ships in the repository
and gets symlinked into `~/.claude/skills/`.
A single `just install` recipe wires both together. Public distribution is via `cargo install --git`; crates.io publishing is
recorded as a decision: publish under the `dasch-swiss` GitHub team as crate owner, starting at 0.1.0.

## Binary name

`dsp` is the user-facing command (set via `[[bin]] name = "dsp"` in Cargo.toml). The crate, the GitHub repo,
and most documentation prose continue to use `dsp-cli` — the distinction matters:
every character of friction matters in agent prompts that already carry a lot of context,
and "dsp" reads naturally in sentences ("run dsp to list projects", "dsp couldn't reach the server").

No collision risk: `dsp-tools` ships as `dsp-tools`, not `dsp`; nothing else common claims the name.

## Local install for the personal phase

```bash
cargo install --path .
```

Lands the binary at `~/.cargo/bin/dsp`. This is the canonical install for the developer's own use.

A `justfile` (or `Makefile`) recipe wraps the install so binary + skill update in one step:

```just
install:
    cargo install --path .
    mkdir -p ~/.claude/skills
    ln -sf {{justfile_directory()}}/skill ~/.claude/skills/dsp-cli
```

Symlink (not copy) of the skill so edits to `skill/SKILL.md` take effect immediately during active development.

## Agent discoverability: the Claude Code skill

A small skill at `<repo>/skill/SKILL.md` declares dsp-cli's existence and the absolute path of its binary. When installed (via the symlink above),
it surfaces in every Claude Code session as a discoverable capability — independent of the calling shell's `PATH`.

The skill answers three questions for any agent that finds it:

1. **When to reach for `dsp`** — DSP interactions, especially project / data-model / resource-type discovery; prefer it over manual DSP-API curl calls.
2. **How to invoke it** — `command -v dsp` falling back to `$HOME/.cargo/bin/dsp`. PATH-independent.
3. **What to do if it's missing** — `cargo install --git https://github.com/dasch-swiss/dsp-incubator dsp-cli --force`.

The skill is itself agent-facing documentation. It is *not* a replacement for `dsp docs <topic>` (ADR-0010) — the skill is a doorway into the tool; `dsp docs` is the room.

## Public distribution

When the project goes public:

- **Primary install path:** `cargo install --git https://github.com/dasch-swiss/dsp-incubator dsp-cli` (or wherever the public repo lives).
  No infrastructure to set up; works for anyone with Rust toolchain.

## Decision: crates.io publishing

Publish `dsp-cli` to crates.io, starting at version 0.1.0. Crate name `dsp-cli` (the binary stays `dsp` — binary names need not
be unique on crates.io); name verified available on crates.io as of 2026-06-18 (the crates.io API returned 404 = unclaimed).

**Owner:** the `dasch-swiss` GitHub team, added as a crate owner — not a personal account. crates.io has no "organization
account" concept: a crate is owned by crates.io users and/or GitHub teams. The first `cargo publish` runs under a
maintainer's GitHub-authenticated crates.io login; `cargo owner --add github:dasch-swiss:<team>` then adds the
`dasch-swiss` team as a co-owner. See ADR-0014 for the publish-first sequencing this decision unblocks.

### Rationale

- **For:** simplest install command for users (`cargo install dsp-cli`); standard Rust ecosystem visibility.
- **Against:** crates.io publishes are irrevocable; name-squatting locks in the project name and crate name forever; ongoing version maintenance obligation;
  limited benefit if `cargo install --git` is good enough.

## Considered alternatives

- **Symlinked debug binary** (`ln -sf target/debug/dsp ~/.cargo/bin/dsp`). Rejected — stale-binary failure mode if a build fails silently;
  the agent invokes the previous binary without warning. `cargo install` produces a real artefact and fails loudly.
- **Wrapper script that builds on each invocation.** Rejected — every command runs a `cargo build` check; adds latency and noise to agent transcripts.
- **Relying on shell PATH inheritance** for agent discoverability. Rejected — agents may launch with a different (more austere) environment than the user's interactive shell;
  aliases and shell config don't help. Absolute path through the skill is robust.
- **Updating each DaSCH project's CLAUDE.md** to mention dsp-cli. Rejected — multiple files to maintain; fragile.
- **Binary name `dsp-cli`** instead of `dsp`. Rejected — agent-prompt friction; nothing about ergonomics is improved by the longer name.
- **Distribute as Homebrew tap / Nix flake.** Deferred — value only emerges with non-Rust users.

## Consequences

- `just install` is the canonical "make my changes available" command during development. The README's first instruction.
- The skill source lives at `<repo>/skill/SKILL.md` and is versioned with the code. Updates to the skill propagate through the symlink without re-running `just install`.
- Other DaSCH project sessions automatically see dsp-cli as available — no per-project CLAUDE.md changes needed.
- Once 0.1.0 is published, the primary install command becomes `cargo install dsp-cli`; `cargo install --git ...` remains the
  fallback for the pre-publish window and for anyone tracking `main` ahead of a release.
- The skill becomes a public-facing artefact that other dsp-cli users (eventually) install alongside the binary.
  Its quality matters: bad skill content misleads every agent that reads it.

## Amendment (2026-07-24) — skill relocated to dasch-claude-plugins

The in-repo skill and its `just install` symlink are **withdrawn**. The agent-facing Claude Code skill is no longer
shipped from this repository; its canonical home is now the `dasch-swiss/dasch-claude-plugins` repo
(`misc/skills/dsp-cli/`, surfaced as the `misc:dsp-cli` skill) and it is distributed via the Claude Code plugin
marketplace rather than a per-repo symlink.

What changed as a result:

- `skill/SKILL.md` deleted from this repo.
- The `just install` recipe was **removed entirely**. It existed only to install the binary *and* symlink the skill;
  with the skill gone it did nothing a plain `cargo install --path . --force` doesn't, so it was dropped rather than kept
  as a thin wrapper. (Anyone who previously ran `just install` has a now-dangling symlink at `~/.claude/skills/dsp-cli` —
  remove it manually.) This retires the "`just install` is the canonical 'make my changes available' command" consequence
  recorded above.
- The crate's `Cargo.toml` `include` allowlist no longer packages the skill.
- The compile-time `skill/SKILL.md` ↔ clap drift-guard test (`src/cli/mod.rs`) was removed with the file.

Why: the plugins repo already carried a diverged, more current copy of the skill, and maintaining two copies in two
distribution channels guaranteed drift. The plugin marketplace is the DaSCH-wide distribution mechanism for agent
skills, so the crate no longer needs to ship or symlink its own. The rest of this ADR (binary name, `cargo install`,
crates.io publishing) stands unchanged; only the skill-shipping/symlink mechanism is withdrawn.
