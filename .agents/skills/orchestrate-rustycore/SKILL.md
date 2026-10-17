---
name: orchestrate-rustycore
description: "Route RustyCore development work between one parent integrator and bounded workers, per runtime (Claude Code: Opus; DSH Web / OpenCode Go: go-mix; Codex: Sol/Luna). Use for implementation coordination or adapting this workflow, not ordinary factual answers."
---

# RustyCore orchestration

AGENTS.md owns scope, authority, validation cadence and completion. This skill owns
task routing, not another architecture plan. Keep one macrodeliverable and one parent
integrator; no per-worker issues, PRs or mandatory continuation requests.

## Decision authority

Substantive technical decisions are **made inside the agent workflow, never parked on the
human operator**. This is the rule that keeps a round from stalling.

- **Who decides.** Parity and behaviour contracts, intentional departures from the
  reference, the scope and cut points of a slice, ownership boundaries and design choices
  are decided at high effort: in DSH `go-mix`/OpenCode Go the coordinator decides at
  `high`, consulting GPT-6.1 Sol `high` or Claude Opus 5.5 `medium` for a concrete
  uncertainty; in Claude Code the Opus parent itself. If the reviewer route is unavailable,
  state that it failed, decide from the recorded evidence, and label the decision as
  taken without that second opinion.
- **Who executes.** The coordinator/parent owns review, validation, Git, integration and
  the durable record of the decision. It executes what the reviewer decided and reports
  the outcome; it does not return the decision to the human as a question.
- **Reserved to the human.** Only two things: runtime/database/destructive operations
  (a live server, a real client capture, schema or data mutation) and material changes to
  the objective's scope or acceptance criteria. No other gate.
- **When a decision needs a reserved gate.** Record it as a **bounded hold**: the decision
  taken, the exact capture or operation that would close it, and what is forbidden in the
  meantime (for example: behaviour retained, expansion forbidden, no parity claimed). Then
  continue with the rest of the work. A reserved gate blocks that one item, not the round.
- **Never do this.** Do not end a round with a question whose answer is a technical choice
  the reviewer tier can make; do not repeat a request for approval across rounds; do not
  treat "the human has not answered" as a blocker for work that does not touch a reserved
  operation.
## Roles and principles (runtime-neutral)

The roles are the same in every runtime; only the models differ (see **Routing by runtime**).

- **One parent integrator** owns architecture, decomposition, contracts, coordination,
  review, Git, integration and final acceptance. It does not implement routine units.
- **One bounded worker** implements a settled unit, does read-only exploration, or runs
  the agreed final sequence as exclusive validation executor, as assigned.
- **At most one child at a time** (route files state any exception). **No worker spawns
  children.** No overlapping edits between parent and worker; the parent owns Git.
- **One useful collaborator.** Delegate only when the parent has useful independent work
  meanwhile and delegation saves more than it costs; never delegate merely to consume a
  collaborator's quota. Do not duplicate the child's implementation.
- **No silent model substitution.** Use only the routes and fallbacks in the current
  runtime's route file. If a role/model/effort is unavailable, disclose it once and keep
  safe work moving in the parent. **No paid-API fallback** unless the route says so.
- **Actual model/effort.** Workers report the model/effort that actually ran; a
  configured role or default is not evidence that it ran — claims need a successful
  spawn trace.
- **Bounded context.** Avoid full-history forks when they force inherited models or
  expose irrelevant data; do not send secrets or session transcripts to children.
- **The parent inspects the actual diff** and its consumers, not just the summary.

## Routing by runtime

Identify the session's runtime and read only its route file. Each client reads only its
own defaults.

| Runtime | Route | Client defaults |
|---|---|---|
| Claude Code / `ocx claude` (default) | [routes/claude.md](routes/claude.md): Opus parent, Opus `rustycore-worker` | `.claude/settings.json`, `.claude/agents/` |
| DSH Web `go-mix` / OpenCode Go (if the user chooses it) | [routes/go-mix.md](routes/go-mix.md): Go DeepSeek coordinator, Luna `max` worker | DSH profile preset `go-mix`; `opencode.json` |
| Codex | [routes/codex.md](routes/codex.md): Sol parent, `luna_worker` | `.codex/config.toml`, `.codex/agents/` |

## Sizing the work

The worker is productive on small units with compiler feedback and stalls on large,
open-ended ones. The parent makes the decisions, the worker executes them.

- One unit is one behavior or one module: a few files, a change that `cargo check` and
  focused tests can confirm. Split anything larger before assigning it.
- For a restructuring (for example splitting a large file), first assign a read-only
  exploration that returns a map: phases with line ranges, the locals each phase reads
  and writes, the locks it holds, early returns. The parent then fixes the cut points,
  module names and signatures, and assigns one module per implementation task.
- Give the worker a self-contained task: checkout/base, owned paths, objective, exact
  line ranges or symbols, the Rust/C++ anchors, the contract and non-goals, the crate
  and test filter to check with, and the acceptance criteria. Quote the rules that
  apply instead of asking it to read long documents again. No secrets or transcripts.

## Running the worker

In Claude Code, start at most one worker at a time (other routes: see their route file);
no worker spawns children. Run it in the
background and keep doing useful parent work (settling consumers, preparing the next
unit's contract, reviewing the previous diff) without duplicating the worker's edits.

- **Heartbeat:** check the worker's output every ~5 minutes. It should be editing
  within ~15 tool calls or ~8 minutes of starting an implementation unit.
- **Stall:** if two heartbeats pass with reads but no edit, stop it and reassign the
  unit smaller or with the missing decision made. A stall is a sizing problem, not a
  reason to change model or provider.
- **Review every diff** against the contract and its consumers before assigning the
  next unit, not just the summary. Assign corrections as a new small unit.

Within a shared checkout, parent and worker must not edit overlapping files, perform
Git mutations, or change shared generated/policy files concurrently. The parent owns
Git and integration. Worktrees are optional; they do not isolate DBs, processes,
caches or network. These instructions are not a security sandbox; keep unneeded
credentials and live resources out of worker tasks.

Workers return the actual model/effort, base and final diff identity, exact
paths/symbols, the checks they ran with exits, unexecuted checks and concrete blockers.
Claims of model use require a successful spawn trace.

## Acceptance and resumption

Implementation checks stay crate-scoped and sequential (`cargo check -p`, focused
`cargo test -p` with `CARGO_BUILD_JOBS=1`), as the worker definition says. Any explicit
instruction deferring worker checks for a whole sweep (the parked #1233 sweep) does not
apply to #1241 units: their workers run these checks (R3). Completed-delivery
acceptance follows AGENTS.md: the parent plans it once and may assign the worker as the
exclusive validation executor for the agreed non-live sequence; that assignment does not
include another QA campaign or autonomous repairs. The parent interprets findings and
assigns corrections. Authorized live DB/runtime QA stays with the parent. Reuse valid
evidence for unchanged inputs. Per-PR checklist and the no-attribution commit/PR rule:
[develop-rustycore](../develop-rustycore/SKILL.md).

The parent's diff inspection is required; an additional reviewer agent or automated
review request is not. Preserve any explicit external contribution/review requirements.
No automatic review loop, new approval gate, or permission to publish/runtime-write.

Use the existing task/checkpoint for material decisions and final evidence, with a
short handoff in the conversation for active worker/process IDs and remaining work.
On resume reconcile Git and running processes before continuing; do not replay
completed operations. Report actual time/usage/rework when available, otherwise unknown.
New project defaults require a fresh session; inspect effective settings rather than
assuming files changed an existing session.
