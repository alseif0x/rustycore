---
name: orchestrate-rustycore
description: "Route RustyCore development work between the Opus parent and its DeepSeek worker (native DeepSeek API through claude-router). Use for implementation coordination or adapting this workflow, not ordinary factual answers."
---

# RustyCore orchestration

AGENTS.md owns scope, authority, validation cadence and completion. This skill owns
task routing, not another architecture plan. Keep one macrodeliverable and one parent
integrator; no per-worker issues, PRs or mandatory continuation requests.

## Models

Only two models take part. Do not call or substitute any other model or provider.

- **Parent: Claude Opus 5.5**, started with `claude-router` in this checkout (parent id
  `codex_router/anthropic/claude-subscription/claude-opus-5.5`, effort `low` from
  `.claude/settings.json`; raise it with `/effort medium` only for a hard architecture,
  lock-order or concurrency decision, then return to `low`). The parent owns
  architecture, decomposition, contracts, coordination, review, Git, integration and
  final acceptance. It does not implement: every code change goes to a worker.
- **Worker: DeepSeek v4.1 flash** on the native DeepSeek API, the
  `.claude/agents/deepseek-worker.md` subagent (`codex_router/anthropic/deepseek/deepseek-v4.1-flash`,
  effort `high`). Never route it through OpenRouter or another reseller.
- **Fallback: `rustycore-worker`** (`.claude/agents/rustycore-worker.md`, Opus inherited,
  `low`) only when a DeepSeek spawn or turn fails at the API level (model error,
  provider outage, quota/balance, rate limit, timeout). State it once and hand over the
  base, current diff and partial work. A slow or poor result is not an API failure.

Plain Anthropic ids such as `claude-opus-5-5` are not served inside `claude-router`;
keep agents on `inherit` or a `codex_router/...` id. Reasoning effort is a real runtime
setting: do not claim a model or effort the session configuration does not confirm.
Record the model that actually ran.

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

Start at most one worker at a time; no worker spawns children. Run it in the
background and keep doing useful parent work (settling consumers, preparing the next
unit's contract, reviewing the previous diff) without duplicating the worker's edits.

- **Heartbeat:** check the worker's output every ~5 minutes. It should be editing
  within ~15 tool calls or ~8 minutes of starting an implementation unit.
- **Stall:** if two heartbeats pass with reads but no edit, stop it and reassign the
  unit smaller or with the missing decision made. A stall is a sizing problem, not a
  reason to use the fallback.
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
`cargo test -p` with `CARGO_BUILD_JOBS=1`), as the worker definition says. Completed-delivery
acceptance follows AGENTS.md: the parent plans it once and may assign the worker as the
exclusive validation executor for the agreed non-live sequence; that assignment does not
include another QA campaign or autonomous repairs. The parent interprets findings and
assigns corrections. Authorized live DB/runtime QA stays with the parent. Reuse valid
evidence for unchanged inputs.

The parent's diff inspection is required; an additional reviewer agent or automated
review request is not. Preserve any explicit external contribution/review requirements.
No automatic review loop, new approval gate, or permission to publish/runtime-write.

Use the existing task/checkpoint for material decisions and final evidence, with a
short handoff in the conversation for active worker/process IDs and remaining work.
On resume reconcile Git and running processes before continuing; do not replay
completed operations. Report actual time/usage/rework when available, otherwise unknown.
New project defaults require a fresh session; inspect effective settings rather than
assuming files changed an existing session.
