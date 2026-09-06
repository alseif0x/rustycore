---
name: refactor-rustycore-safely
description: "Implement or review behavior-preserving RustyCore restructuring: module/file splits, application or persistence boundaries, canonical ownership, dependency edges and earned crates. Use for restructuring Session, handlers, Map, Player, composition, data, packets or QA tooling. Design unclear boundaries with design-rustycore-architecture first; do not hide gameplay/protocol repairs inside a refactor."
---

# Refactor RustyCore Safely

Preserve base-server behavior while improving an approved responsibility boundary.
Use coherent internal slices; follow the authorized macro through acceptance rather than
stopping after each helper. AGENTS.md owns scope, approvals, publication and completion rules.

## Mode and relevant context

- Review-only: inspect and report prioritized findings; no edits, commits or external changes.
  Separate regressions from pre-existing debt, with exact Rust/C++ or capture anchors.
- Execution: implement the requested restructuring inside its approved issue and dirty-tree scope.
  A request to audit/update instructions does not itself authorize production refactoring.
- If inspection versus mutation is materially ambiguous, stay read-only while clarifying.

Read AGENTS.md completely and follow its kickoff. Read the current STATE.md checkpoint,
the active issue and task-relevant decisions/risks in owning ADRs, not their entire historical
logs. Read [references/refactor-playbook.md](references/refactor-playbook.md). For source/module
decomposition, also read docs/architecture/module-design-guidelines.md completely; that document
owns current budgets, bounded exceptions and independent semantic/physical acceptance.

Inspect current callers, tests and exact C++ behavior before moving the responsibility.
If ownership or dependency direction is unresolved, use the architecture skill to investigate
that boundary; keep safe inspection moving. An approved design and implementation request
do not require the same approval again.

## Classify and freeze the change

Choose the dominant structural class for each coherent internal commit:

1. Mechanical relocation: preserve owner, signatures, order, visibility and registrations.
2. Boundary extraction: redirect a real module/application/port dependency without duplicating state.
3. Ownership migration: redirect all relevant readers/writers to one authority and retire the old path.

An intentional behavior repair is a separate change, not a fourth kind of behavior-preserving
refactor. Fix regressions introduced by the authorized slice; do not silently absorb unrelated
legacy defects. Distinguish moves, boundary changes and ownership through reviewable commits,
not automatic new PRs or issues that fragment the approved macro.

Freeze the affected contract: packet bytes/metadata/connection/recipients/order; admission and
C++ phases; SQL/transaction/commit classification and recovery; state incarnation, mutation
count, lock order and clock; public paths and feature/test registrations. Identify regression
evidence before moving fragile code. Pure relocation needs the relevant contract, not an
unrelated new opcode, database or live-client test.

## Map the surface and make a coherent edit

Search definitions, impl blocks, re-exports, all callers, registration macros, feature gates,
tests/fixtures, locks, queues, SQL and dependent docs. Source filenames alone do not identify
the complete logical owner.

Move one complete feature family, use case or adapter seam at a time. Keep private implementation
and narrow inputs/results; compatibility bridges/re-exports require explicit removal conditions.
Do not introduce traits merely for organization, broaden public fields for tests, or add locks,
clones, mutable mirrors, global state or untyped resource bags to silence borrow errors.
Do not alter serialization, transaction order, dispatch uniqueness or queue bounds during a move.

Preserve each operation's actual durability sequence. For commit-before-application operations,
an example is validate/plan -> transaction -> classify commit -> canonical application -> publication.
Deferred saves instead persist a projection of existing canonical state and acknowledge only
the saved incarnation/revisions. Do not force either contract into the other's ordering.
Do not invent persistence for a purely in-memory operation, claim durable success before commit,
or reconcile the same runtime mutation twice. Preserve required async operation fences; do not
carry synchronous map/entity guards across await or perform I/O/delivery under a map lock.

Physical splitting may keep impl WorldSession temporarily, but it does not transfer gameplay
ownership. A valid Player/Map aggregate may span many private files without copied state.
Apply the module-design policy to production, tests and fixtures as well as the logical owner.

## Validate and audit the finished boundary

Use AGENTS.md and the validation-v2 operation guide for the actual commands/profiles:
focused positive/negative tests and affected-crate checks during iteration; explicit production
integration/failure cases for affected owners; clean-HEAD final and exact issue acceptance at
publication. Set PROTOC for protobuf builds and select the real lib/bin/integration target.
Do not rerun exhaustive inventories per helper or treat a library suite as production wiring proof.
The playbook provides change-specific evidence; it does not override proportional validation.

Before accepting a responsibility, verify:

- No logic, cfg branch, test or registration disappeared; moved tests still execute.
- Affected handlers preserve the exact opcode/metadata/call set, not just counts.
- No unjustified public API/upward dependency, state mirror, new writer/clock or lock order appeared.
- All related consumers use the intended owner; superseded access is deleted or explicitly temporary.
- Both semantic boundaries and physical source/test policy are satisfied; no replacement monolith
  in a fixture file or distributed gameplay god object.
- Paths, C++ references, before/after measurements and remaining exceptions match the actual diff.
- Unrelated user/agent work remains untouched and uncommitted.

Capture-diff/live QA follow their actual change triggers and explicit issue acceptance.
Fresh scenario captures are distinct from regression goldens; mocked persistence is not proof of
durable crash recovery. Report the tested SHA, command, result and unproven boundaries.

## Investigate without forcing changes

Pause only the affected mutation when evidence reveals a behavior difference, unresolved owner,
new coupling/mirror requirement, unexplained test failure or inseparable unrelated dirty work.
Continue inspection, reproduction and C++/capture comparison. Resume within existing authority
once resolved; ask only for a material decision/new scope/authority evidence cannot supply.
Reuse the issue branch/PR and follow AGENTS.md for commits, authorized push and merge boundaries.
