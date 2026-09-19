# dsp-cli vs dsp-tools — which to use

Both tools talk to the same DSP-API, but they do different jobs. The boundary is
the **interaction mode**, not the data touched.

| Use… | when you want to… |
|---|---|
| **dsp-cli** | read or discover, one command at a time, interactively (list/describe projects, data-models, resource-types; download a project dump) |
| **dsp-tools** | drive bulk, declarative work from files (create a project, define a data-model from JSON, bulk-ingest XML/Excel data) |

In short: `dsp-tools` is file-driven and batch; `dsp-cli` is per-command and
interactive. They may touch the same project, but they never duplicate each
other's mode — `dsp-cli` does not do file-roundtripping bulk operations, and it
never will. If your task is "apply this file to the server", that's `dsp-tools`.
If it's "tell me what's there", that's `dsp-cli`.

See also: `dsp docs dsp-cli`, `dsp docs workflows`.
