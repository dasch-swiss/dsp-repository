# Writing decision records

These rules apply to every decision series in the repository: the root `docs/adr/`,
`areas/<area>/docs/adr/` and `<component>/docs/adr/`. Where a record lives and how it is cited is
decided in ADR-0006 and summarised in `docs/src/decisions.md`. The format is the dune plugin's
MADR 4 "bare" template (`dune:dune-principles` points to it); where the plugin's maintenance
rules differ from this file, this file wins.

## An ADR holds the current decision, never its history

An ADR states what is decided now. Either the code follows it, or the ADR is the target and the
code is moved to match. It never describes a state that no longer holds.

History goes; the future stays. How a decision got here is in git. What is decided but not yet
built, what is known to be coming, what is deliberately left open and what is deferred rather
than rejected is part of the current decision, and stays in the ADR, ticket or not.

- **A changed decision is rewritten in place.** No `## Amendment` sections, no dated notes, no
  "as accepted, the clause read …", no "see the amendment below". The ADR reads as if it had
  been written today, in one piece.
- **A decision that no longer holds is retired**, not marked superseded: its content goes, and its
  file stays as a tombstone (see *Numbering*). If its replacement is another record, the citations
  that rely on the decision move to that record in the same commit.
- **The history is in git.** The commit that rewrites or retires an ADR says in its body what the
  record decided before and why that changed (`docs/src/git-conventions.md` → *History lives in
  git*). Never move that explanation into the ADR.
- **A rejected alternative stays only if it is still worth knowing** — typically because a reader
  would otherwise propose it again. It goes under Considered Options, stated as an option and why
  it loses, not as "we used to do this".
- **No progress logs.** "The Deposit Area half is done", "the move landed on …", "the script has
  landed": what has happened is commit content. A decided order for work still to come is part
  of the decision and stays.

## What is still to come

Forward-looking content is written so a reader can tell it from what is built, in one of three
forms:

- **A decided target the code does not meet yet** — a line directly above `Enforced by:`, one per
  gap:

  ```md
  Not yet in code: <what does not match, in code terms> (DEV-NNNN).
  ```

  Name the Linear issue when one tracks the work, and write `(not scheduled)` when none does. The
  commit that closes the gap deletes the line.
- **A question the record knowingly leaves open** — stated as such where it arises ("The form of
  a presentation ARK is not decided."), so nobody reads silence as a decision.
- **An option deferred rather than rejected** — under Considered Options, marked *deferred*, with
  what would bring it back.

What is not allowed is a statement about the future that goes stale by itself: "this PR adds",
"lands next week", "the second half follows", "temporary until the refactor".

## Format

- No frontmatter. A record on `main` is the decision in force; `status:` and `date:` would only
  repeat or contradict git.
- H1 title stating the decision, then prose: the context, what is decided, and why. Keep it to
  the decision — how it is implemented belongs in `docs/src/`, `ARCH-MAP.md` or `CONTEXT.md`, which
  the ADR may point to.
- Phrase the decision so a violation is describable in code terms (a crate, a path, a
  dependency direction), wherever possible.
- Optional `## Considered Options` and `## Consequences`, only when they add value.
- An `Enforced by:` line naming the check and its level (`structure`, `static-analysis`,
  `review`, `docs-only`).

## Numbering

An ADR number is an identifier, and an identifier is never reused. A series is numbered from
`0001` in order of first acceptance; a new record takes the next number after the highest ever
used, and no record is ever renumbered. Linear issues, learnings, specs, the design repository and
commit messages cite records by number, and each of those citations has to keep naming the record
it meant.

A retired record keeps its file and its file name, so every citation of it still resolves. The
file becomes a tombstone: the H1 title it had, and one `Retired:` sentence that says why, followed
by where what it decided lives now (`See ADR-NNNN.`), or by `No successor.` when nothing replaces
it:

```md
# <the title it had>

Retired: <why, in one sentence>. See <the record that holds the decision now>.
```

A tombstone has no other content, no `Enforced by:` line and no `Not yet in code:` line. What the
record still held moves to the records that hold it now before the record is retired, and the
retiring commit says where. The decision index (`docs/src/decisions.md`) keeps the row, marked
retired.
