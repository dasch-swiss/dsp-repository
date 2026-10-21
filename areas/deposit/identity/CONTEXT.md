# Identity

The Deposit Area's identity capability: who has an account, how they sign in, and which projects each account may reach. Four crates: `identity-core` (`core/`), the account, session and login-code records, the login and session rules, and the repository traits their storage implements; `identity-ports` (`ports/`), `AuthoredWork`, what identity asks of the editor; `identity-store` (`store/`), the SQLite implementation in `identity.sqlite`, the mail transports, and the adapters that answer the editor's ports; and `identity-web` (`web/`), the login and depositor pages, their handlers and routes. The editor's binary starts identity and serves its routes; `docs/src/editor/architecture.md` lists what is left to move. The target shape is `areas/deposit/ADR-0004`.

## Language

### People and access

**Depositor**:
A `users` row with `Role::Depositor`, created by RDU, who may edit the projects assigned to them.
_Avoid_: user (too broad — RDU members are users too), researcher, project team (the people; the depositor is the account).

**RDU**:
The reviewer role: the Research Data Unit member who reviews submissions and administers depositors; `Role::Rdu` when stored, the `Rdu` extractor when authorizing, `Audience::RduOnly` when deciding field visibility.
_Avoid_: admin, reviewer as a role name (a reviewer is what RDU does on a given round).

**Assigned project**:
A `user_shortcodes` row: one project a depositor may reach, matched on the case-folded Shortcode (`editor-ports`' `Actor::assigned_shortcodes` and `Actor::may_reach`, over what `LiveAuthenticator` hands the editor; `Actor` is defined in [`areas/deposit/editor/CONTEXT.md`](../editor/CONTEXT.md)).
_Avoid_: permission, membership, assignment (the row is named for the project it grants, not the act).

**Login code**:
A one-time code mailed to a depositor and bound to a browser token, exchanged for a Session.
_Avoid_: OTP, magic link (it is typed, not clicked), password (the editor has none).

**Session**:
The cookie-backed result of a redeemed Login code; carries the user id only.
