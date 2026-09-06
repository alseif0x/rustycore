# pi.dev Porting Instructions

This path is retained for existing references. The shared operating guide is
[AGENTS.md](../AGENTS.md); pi.dev does not have a separate approval, branch, or
completion policy.

Read [STATE.md](migration/STATE.md), [PORT_PLAN.md](migration/PORT_PLAN.md), and the
active issue/checkpoint for the current scope. The
[C++ contrast methodology](CPP_TO_RUST_PORTING_METHODOLOGY.md) is a technical
supplement, not another kickoff or release gate.

- Anchor affected behavior to `/home/server/woltk-trinity-legacy`, or to an
  identified client/server capture when C++ is incomplete or ambiguous. Old Rust
  comments and implementation narratives are not parity proof.
- Work through the authorized capability with focused positive/negative evidence.
  Internal slices and commits do not require separate issues, PRs, or repeated
  "continue" confirmations.
- Preserve the explicit requirement for authorization before starting a server
  or publishing. Reuse approval for its stated scope; a test environment alone
  does not authorize unrelated actions, deployment, push, or merge.
- Use current path-routed validation and explicit issue acceptance, not a fixed
  historical test count or an unconditional workspace/server build per helper.
- Do not use the former `f7eb5dc` base or combat audit target list as current
  status. Determine the relevant reviewed base from current evidence.

Report completed scope, exact checks, remaining boundaries, and local versus
published status. Do not turn represented-item percentages into server completion.

*Workflow redirect refreshed 2026-09-05; no implementation or parity audit implied.*
