---
name: orchestrate-rustycore
description: "Coordinate RustyCore development between the GPT-6.1 Sol parent and GPT-6 Luna worker. Use for implementation routing or adapting this workflow, not ordinary factual answers."
---

# RustyCore orchestration

AGENTS.md owns scope, authority, validation cadence and completion. This skill owns
task routing, not another architecture plan. Keep one macrodeliverable and one parent
integrator; no per-worker issues, PRs or mandatory continuation requests.

## Models

The native Codex parent uses **GPT-6.1 Sol** (`gpt-6.1-sol`) at `high` effort. It owns architecture,
decomposition, contracts, coordination, review, Git, integration and final acceptance;
it does not implement. The implementing worker is **GPT-6 Luna** at max reasoning effort,
spawned natively with `model: gpt-6-luna` and `reasoning_effort: max`.
The local, ignored `.codex/config.toml` sets the parent model/effort and the
`[agents]` default subagent model/effort for this checkout.

For Claude-compatible sessions, use `ocx claude`. `.claude/settings.json` selects the
parent alias `ocx-claude-native--gpt-6.1-sol` at high effort, and
`.claude/agents/rustycore-worker.md` selects `ocx-claude-native--gpt-6-luna` at max
effort. These are defaults: start a fresh session after changing them and inspect the
effective session and successful worker spawn before reporting the model or effort that
actually ran. Never infer runtime use from settings alone. Do not automatically fall
back to an old model or another provider; report a spawn or API failure to the parent.

## Sizing the work

Keep worker assignments small and bounded. The parent makes the decisions; the worker
executes them.

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

Start one Luna worker by default. When the user authorizes parallel implementation,
use the available native slots for independent, disjoint write scopes; the current
refactor campaign has that authorization. No worker spawns children. Keep doing
useful parent work (settling consumers, preparing the next unit's contract, reviewing
the previous diff) without duplicating worker edits. Worker concurrency does not
authorize concurrent validation or shared target directories.

- **Heartbeat:** check the worker's output every ~5 minutes. It should be editing
  within ~15 tool calls or ~8 minutes of starting an implementation unit.
- **Stall:** if two heartbeats pass with reads but no edit, stop it and reassign the
  unit smaller or with the missing decision made.
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

Where authorized, implementation checks stay crate-scoped and sequential (`cargo
check -p`, focused `cargo test -p` with `CARGO_BUILD_JOBS=1`). An explicit whole-sweep
instruction defers these worker checks too: write tests and preserve coherent local
work, then execute the single completed-delivery acceptance campaign. The current
refactor checkpoint records that instruction; do not restart feedback checks at a
worker or helper boundary. Completed-delivery
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
