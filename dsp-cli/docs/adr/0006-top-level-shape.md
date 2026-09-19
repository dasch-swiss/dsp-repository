# Top-level command shape: areas and meta-groups

`dsp-cli`'s top level distinguishes **areas** of the DaSCH Service Platform (operational scope) from **meta-groups** (cross-cutting concerns like authentication).
Each area is a top-level command group; meta-groups also live at the top level rather than being nested inside an area.

```
dsp auth ...    # meta — authentication management; applies to all areas
dsp vre ...     # area — Virtual Research Environment (current scope)
dsp repo ...    # area — Repository (future; not in v1)
```

## Rationale

Auth tokens are **server-keyed, not area-keyed**: the same token authenticates a user across VRE and (future) Repository operations once they target the same server.
Nesting `auth` inside `vre` would falsely imply VRE-scoped auth and would force an awkward duplication if `dsp repo` is added later.
Top-level `dsp auth` matches the standard pattern (`gh auth`, `aws configure`, `kubectl config`).

## Considered alternatives

- **`dsp vre auth ...`** (auth scoped under each area). Rejected — falsely implies area-scoped tokens; doesn't generalise when a second area arrives.
- **No `auth` commands; rely solely on env vars / external token provisioning.** Rejected — leaves humans without a built-in login flow and forces
  every user to learn DSP-API's login endpoint by hand.

## Consequences

- v1 ships two top-level groups: `dsp auth` and `dsp vre`.
- `dsp repo` is reserved but not implemented. The shape is here so we don't have to renegotiate it later.
- Future meta-groups (e.g. `dsp config` for non-secret settings, `dsp completion` for shell completions) can live alongside `auth` without restructuring.
