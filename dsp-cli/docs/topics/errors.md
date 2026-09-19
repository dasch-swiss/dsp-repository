# Errors — exit codes and how to recover

`dsp-cli` is built so a caller can decide what to do next from the **exit code
alone**, without parsing any prose. This topic is the contract.

## Exit codes

| code | meaning | typical cause |
|---|---|---|
| `0` | success | command ran, data emitted |
| `1` | runtime error | network/server failure, malformed response, conflict, local I/O |
| `2` | usage error | bad flag, missing argument, unparseable identifier syntax |
| `3` | authentication required | no token supplied; the token was rejected by the server; or a cached token was locally detected as expired |

These four codes are fixed in v1; a new category would require an ADR amendment, so
you can rely on them. **Branch on the exit code, not on the message text** — the
message is human-oriented and may change.

Note one subtlety: a *named thing that doesn't exist* (a wrong project shortcode, a
missing data-model) is a **runtime** error (exit `1`), not a usage error. Exit `2`
is reserved for malformed *input* (a bad flag or unparseable identifier), matching
clap's own arg-parse behaviour.

## Recovering, by situation

- **exit 3 (auth required):** run `dsp auth login -s <server>` (or supply a token —
  see `dsp docs connecting`), then retry. This is the one case where an agent should
  attempt to authenticate before reporting failure.
- **exit 1, "not found":** the identifier doesn't exist on that server. Re-run the
  matching `list` command to get valid identifiers (see `dsp docs workflows`).
- **exit 1, "conflict":** a server-side resource is busy or already exists — most
  often the single server-wide dump slot is occupied. See
  `dsp vre project dump --help` for `--replace` / `--delete`.
- **exit 1, network/server:** the server is unreachable or returned a 5xx — usually
  transient or an environment problem. Check the server URL and `<server>/health`.
- **exit 1, io:** a local file operation you requested failed (e.g. writing a dump);
  the message names the path. Check permissions and disk space.
- **exit 2 (usage):** re-read the command's `--help`; a flag or argument is wrong.

## The structured error kind

The intended machine-readable contract is a stable `kind` field carried in the JSON
error envelope: `usage`, `auth_required`, `not_found`, `server_error`, `network`,
`conflict`, `io`, `internal`. Adding a kind is additive; renaming one is a breaking
change.

**Behaviour:** under `-j`, the JSON error envelope (with `kind`) is emitted to
**stdout**. Every other format (`prose`, `lines`, `csv`, `tsv`) prints `Error: …` to
**stderr**, as before. `_meta` on an error carries `exit_code` always, and `server`
when it could be resolved (omitted when it couldn't — e.g. when the failure is the
server resolution itself). Still branch on the **exit code** — it remains the
primary stable signal, especially for non-JSON consumers.

See also: `dsp docs output`, `dsp docs connecting`.
