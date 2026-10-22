# Top-level command shape: areas and meta-groups

`dsp-cli`'s top level distinguishes **areas** of the DaSCH Service Platform (operational scope) from **meta-groups** (cross-cutting concerns like authentication).
Each area is a top-level command group; meta-groups also live at the top level rather than being nested inside an area.

```
dsp auth ...    # meta — authentication management; applies to all areas
dsp docs ...    # meta — embedded end-user documentation (dsp-cli/ADR-0010)
dsp vre ...     # area — Virtual Research Environment
dsp repo ...    # area — Repository (reserved)
```

## Rationale

Auth tokens are **server-keyed, not area-keyed**: the same token authenticates a user across VRE and Repository operations once they target the same server.
Nesting `auth` inside `vre` would falsely imply VRE-scoped auth and would force an awkward duplication when `dsp repo` is added.
Top-level `dsp auth` matches the standard pattern (`gh auth`, `aws configure`, `kubectl config`).

## Considered Options

- **`dsp vre auth ...`** (auth scoped under each area). Rejected — falsely implies area-scoped tokens; doesn't generalise when a second area arrives.
- **No `auth` commands; rely solely on env vars / external token provisioning.** Rejected — leaves humans without a built-in login flow and forces
  every user to learn DSP-API's login endpoint by hand.
- **Further meta-groups beside `auth` and `docs`**, e.g. `dsp config` for non-secret settings (see the config-file item in
  [dsp-cli/ADR-0007](0007-auth-and-environments.md)) and `dsp completion` for shell completions. *Deferred*, not rejected:
  each would sit at the top level without restructuring, and gets its own record when it is built.

## Consequences

- The top level has three groups: `dsp auth`, `dsp docs` and `dsp vre`.
- `dsp repo` is reserved so the shape does not have to be renegotiated when the Repository area arrives.

Not yet in code: the `dsp repo` area does not exist; `TopLevel` in `src/cli/mod.rs` has only `Auth`, `Vre` and `Docs` (not scheduled).
Enforced by: the top-level help snapshot (`help_top`) in `dsp-cli/tests/cli.rs`, which pins the top-level command groups (**static-analysis**);
the rest by **review** (the `TopLevel` clap enum is the only place top-level groups are declared).
