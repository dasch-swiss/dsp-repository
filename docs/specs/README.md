# docs/specs — specifications

PRDs, implementation plans, design companions and the execution journals that
`/eng:workflows:work` and `/eng:workflows:work-orchestrate` write while running a
plan, plus design assets, for this repository. Specs targeting dsp-repository
live **here**, never in a central specs repo (see the root `CLAUDE.md`). Treat
them as the source of truth for intended behaviour.

`docs/specs/` sits beside `docs/src/` and is deliberately outside the mdBook
build (`docs/book.toml` has `src = "src"`). Specs are working documents for the
people building the software; the developer documentation in `docs/src/`
describes what shipped.

## Layout

```
docs/specs/
  YYYY-MM-DD-<title-slug>/
    NN-<topic>-PRD.md            # product requirements
    NN-<type>-<topic>-plan.md    # implementation plan (type = feat | fix | refactor)
    NN-<topic>-design.md         # optional UI/UX design companion
    NN-<...>-journal.md          # execution log, written by the work skills, never by a plan
    assets/                      # images/screenshots for this spec (Git LFS)
```

## Folder naming

- One folder **per effort**, `YYYY-MM-DD-<title-slug>/`.
- **Date = the first artifact's creation date.** An effort that spans several
  dates or sessions stays **one folder**, keyed to its first date. (E.g. a plan
  plus its later fix-up plans and journals share one folder.)
- **Slug:** lowercase, `a-z0-9-`, derived from the title.

## File naming

- Files are numbered `NN-…` with a **single per-folder sequence** starting at
  `01`; the next file is `max(NN) + 1`. PRD, plan, design and journals share the
  one sequence (they are ordered by when they were added, not by type).
- The date is **not** repeated in filenames — the folder carries it.

## assets/

- Images and screenshots for a spec go in that spec's `assets/` subfolder, and
  are referenced with a relative path: `![desc](assets/x.png)` or
  `<img src="assets/x.png">`.
- These are **target-state / design** images (e.g. renders of a design canvas or
  annotated mockups). They are plan-scoped: the next spec has its own `assets/`.
- Raster images under `docs/specs/` are stored in **Git LFS** (`.gitattributes`
  at the repo root). The rule is scoped to this folder: the DPE serves images
  from `modules/dpe/public/assets/` straight from the tree, and those must stay
  plain files.

## Plans and PRs

- **One plan is one PR.** A plan's phases are, at most, separate commits inside
  that PR (tick `allow-many-commits` in the PR body when they are). Never plan
  one PR per phase. Steps that can only happen after merge (deploy verification)
  are written as post-merge steps, not as phases needing their own PR.
- **Every phase ends with a reviewer pass.** The plan names the set of
  `eng:review` reviewers relevant to the change once, and each phase's checklist
  ends with running `eng:reviewing` on that phase's diff. Findings are amended
  into the phase's commit before the next phase starts.

## Frontmatter

Each spec carries YAML frontmatter: `title`, `date`, `author`, `status`, and a
`linear:` issue where one tracks it. Plans add `type` (`feat` | `fix` |
`refactor`) and, where one exists, a relative `prd:` path to the PRD they
implement.

```yaml
---
title: "feat: <topic>"
type: feat
date: 2026-09-15
author: "<name>"
status: draft
linear: DEV-0000
prd: ../2026-09-15-<slug>/01-<topic>-PRD.md
---
```
