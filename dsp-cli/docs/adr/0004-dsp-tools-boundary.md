# Boundary between dsp-cli and dsp-tools

`dsp-cli` and `dsp-tools` coexist permanently as separate tools with non-overlapping interaction modes. We do not change anything about `dsp-tools` to accommodate `dsp-cli`,
and `dsp-cli` does not depend on or wrap `dsp-tools` (no shared code, independent reimplementation).
The boundary is drawn on **interaction mode**, not on the data being touched: `dsp-tools` owns file-roundtripping declarative bulk workflows;
`dsp-cli` owns per-command agent-interactive operations.
The two tools can read the same project schema — they package the result differently.

## The rule

- **`dsp-tools` owns** any operation where the input or output is a declarative file in a defined `dsp-tools` format (project JSON, XML data file, Excel inputs,
  roundtrippable backup). Charter: "files in, files out, bulk".
- **`dsp-cli` owns** any operation packaged as "one command → one logical operation → prose or stdout-structured response", suitable for direct AI-agent or
  interactive human use. Charter: "agent mental model".
- Overlap in *data touched* is acceptable. Overlap in *interaction mode* is forbidden. Example: both tools can read a project's data model from the server —
  `dsp-tools get` writes a JSON file; `dsp vre data-model describe` emits prose.
  Same data, different interaction modes, different audiences.

## Considered alternatives

- **R1 — `dsp-cli` eventually replaces `dsp-tools`.** Rejected. Forces a long-term migration plan and ties language/scope decisions to `dsp-tools`' Python heritage.
  We don't want that pressure.
- **R3 — `dsp-cli` wraps `dsp-tools`-the-library** (Python wrapper reusing the dsp-tools API client, models, auth). Rejected — forces Python;
  tight coupling means a change in `dsp-tools` can break `dsp-cli`;
  conflicts with the "files in/out" vs "agent-interactive" split, which is more honest as two separate codebases.
- **B-literal interpretation of the boundary** ("if dsp-tools touches this data, dsp-cli cannot"). Rejected — would gut every read operation in `dsp-cli` because
  `dsp-tools get` already reads project schemas.
  The user's intent ("fits the agent mental model goes to DSP CLI") is the functional reading, not the literal one.
- **R2 + R4 with B-functional** (chosen): orthogonal interaction modes, independent codebases, overlap in data touched is fine.

## Consequences

- `dsp-cli`'s language and runtime are unconstrained by `dsp-tools`' Python — separate ADR to follow.
- Two API clients exist in the DaSCH ecosystem. Bounded maintenance cost; DSP-API isn't rapidly changing.
- `dsp-cli` is **explicitly forbidden** from offering file-roundtripping commands (e.g. "export project to dsp-tools JSON") or bulk-from-file operations.
  If someone proposes one, the answer is "that's dsp-tools' job".
- `dsp-cli` is **explicitly allowed** to expose read/describe/list operations on data that `dsp-tools get` also exposes — different packaging, different audience.
- Future-shaped uncertainty for `dsp-cli`: agent-interactive write operations (e.g. "create one resource") aren't cleanly served by either tool today.
  When they become relevant, this ADR governs the decision: if the natural interaction mode is per-command (not file-driven), it belongs to `dsp-cli`.
- This ADR is the answer to every future "let's just add X to dsp-cli that dsp-tools already does" or "let's just deprecate dsp-tools" —
  the answer is no, and here's the rule that decides it.
