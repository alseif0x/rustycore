---
name: orchestrate-rustycore
description: "Route RustyCore development work between Astra, Luna and Spark when a bounded independent task benefits from delegation. Use for implementation coordination or adapting this workflow, not ordinary factual answers."
---

# RustyCore orchestration

AGENTS.md owns scope, authority, validation cadence and completion. This skill owns
task routing, not another architecture plan. Delegation is explicitly requested for
useful independent work, not for every task. Keep one macrodeliverable and one parent
integrator; no per-worker issues, PRs or mandatory continuation requests.

## Routing

MiMo is outside the active workflow: do not call it or use its credentials. Its
local experimental profile must not replace the main project configuration.

- Parent Astra: `low` default; own architecture, decomposition, coordination,
  integration and final acceptance. Delegate routine bounded implementation to Luna;
  implement directly for trivial, inseparable or critical work. Use `high` for ambiguous
  architecture, ownership, concurrency, persistence and protocol work. Reasoning effort is a real
  runtime setting, not a promise in prose: use an available effort control, or give
  a bounded hard task to `astra_specialist`. Do not claim the parent switched when
  it did not. A session can also be launched with
  `codex -c model_reasoning_effort='"high"'` when high-effort parent work is needed.
- `luna_worker`: `gpt-5.6-luna`, `max`; implement a complete bounded responsibility
  with decided contracts and assigned files, including its tests and consumers.
  This is the normal implementation collaborator, not just an escalation from Spark.
- `spark_helper`: `gpt-5.3-codex-spark`, `medium`; choose one explicit mode:
  targeted read-only exploration, specified mechanical transformation, or execution
  of the parent's final-check sequence. Small diffs are not automatically mechanical.
  Moving code is mechanical only after owner, destination, visibility and behavior
  have been decided. Never ask Spark to redesign Session or establish whole-port parity.

Use the actual available model/effort and record it in the handoff. Custom TOML roles
apply only on clients that load them; otherwise pass explicit supported model/effort
and the role instructions to the available delegation tool. Keep context bounded;
avoid full-history forks when they force inherited models or expose irrelevant data.
If a role/model/effort is unavailable, disclose that once and keep safe work moving
in the parent. Do not silently substitute models or use a paid API fallback. A quota
or unsupported optional role is not a reason to abandon the delivery.

## One useful collaborator

Start with at most one child at a time. No worker spawns children. The parent must
have useful independent work; execute directly if delegation costs more than it saves.
Once a nontrivial implementation unit has a clear contract, independent file ownership
and useful concurrent parent work, assign it to Luna with a real spawn call rather than
merely describing delegation and doing it all in Astra. The parent can settle other
consumers, prepare integration or inspect a separate boundary, but must not duplicate
the child's implementation. File count alone does not require delegation. Keep tiny,
tightly coupled or unavailable-model tasks local; briefly identify the reason when a
substantial delivery stays root-only. This is not a user approval checkpoint.
Give the child the checkout/base, owned paths, objective, relevant Rust/C++ anchors,
contract/non-goals, acceptance criteria and whether checks are deferred or assigned.
Do not send secrets or full session transcripts.

Within a shared checkout, parent and child must not edit overlapping files, perform
Git mutations, or change shared generated/policy files concurrently. The parent owns
Git and integration; serialize overlapping work. Worktrees are optional when useful,
not a compulsory handoff step; they do not isolate DBs, processes, caches or network.
Permissions may inherit live session overrides. These role instructions are not a
security sandbox; keep unneeded credentials and live resources out of worker tasks.

Workers return the actual model/effort, base and final diff identity, exact paths/symbols,
changes, unexecuted tests and concrete blockers. Claims of model use require a successful
spawn trace; a configured role or default is not evidence that it ran.
The parent inspects the actual diff and consumers, not just the summary. Resolve
routine uncertainty locally; escalate on evidence, not a fixed attempt counter or
a compulsory Spark -> Luna -> Astra chain. Preserve useful work when changing owners.

## Acceptance and resumption

Follow AGENTS.md's complete-implementation-first cadence and exclusive validation
owner. Spark can execute the agreed final sequence; it cannot create another QA
campaign or repair failures autonomously. The parent interprets findings and assigns
corrections, then reruns affected evidence on the combined candidate as required.
Delegate only non-live checks to these roles; authorized live DB/runtime QA stays with
the parent under AGENTS.md. Do not send a worker a task its role explicitly forbids.
Freeze inputs while that sequence runs. Reuse valid evidence for unchanged inputs;
do not rerun successful commands merely because ownership passed between agents.
Do not delegate a single shell command merely to consume Spark quota.

The parent's diff inspection is required; an additional reviewer agent or automated
review request is not. Preserve any explicit external contribution/review requirements.
No automatic review loop, new approval gate, or permission to publish/runtime-write.

Use the existing task/checkpoint for material decisions and final evidence, with a
short handoff in the conversation for active child/process IDs and remaining work.
Do not create a competing orchestration ledger for every helper. On resume reconcile
Git and running processes before continuing; do not replay completed operations.
Report actual time/usage/rework when available, otherwise unknown. This initial
profile is not benchmark-proven and does not enforce a hard CPU/RAM/token budget.

Configuration shape checked against
[Codex subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents).
The Astra-orchestrator/Luna-executor topology is also used by
[donvito's template](https://github.com/donvito/codex-astra-luna-orchestrator);
this project deliberately omits its broad mandatory-delegation triggers and staged
tester/reviewer pipeline. Spark is a selective helper, not a required first hop.
New project defaults require a fresh session and a trusted project configuration;
inspect effective settings rather than assuming files changed an existing session.
