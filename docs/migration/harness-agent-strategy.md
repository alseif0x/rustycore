# Harness Agent Strategy for the C++ to Rust Migration

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

This document preserves an earlier multi-agent proposal. Its role/checklist
examples may help a bounded task, but it is not the operating strategy until
completion. Use AGENTS.md and the available skills for current coordination,
authority, macro delivery and validation. In particular, the historical
microphase/commit/stop rules below do not add approval or completion gates.

It complements:

- `docs/CPP_TO_RUST_PORTING_METHODOLOGY.md`
- `docs/PI_DEV_PORTING_INSTRUCTIONS.md`
- `docs/migration/migration-test-strategy.md`

The harness is an acceleration tool, not a source of truth. The only source of
truth for behavior is the legacy C++ tree:

```text
/home/server/woltk-trinity-legacy
```

Existing Rust, previous AI work, roadmap checkboxes, tests, comments and bot
behavior are useful evidence, but they are not proof.

## Goals

The harness must help the port finish without hidden gaps.

Primary goals:

- Increase parallel review throughput without losing C++ parity.
- Keep implementation slices small, testable and mergeable.
- Prevent one agent from marking scaffolding as runtime parity.
- Keep the roadmap honest as the source of truth.
- Reduce repeated context loading by assigning stable roles.
- Preserve a clean audit trail for every closed item.

Non-goals:

- Let multiple agents freely edit the same large file.
- Replace C++ contrast with Rust-to-Rust comparison.
- Turn E2E bot success into correctness proof.
- Produce broad refactors while behavior gaps remain.

## Recommended Team

### Principal Coordinator

Usually the main Codex session.

Responsibilities:

- Owns the plan, scope and final decisions.
- Chooses the next migration slice.
- Reads enough C++ personally to verify agent findings.
- Assigns bounded tasks to subagents.
- Integrates patches.
- Resolves conflicts.
- Updates migration docs and inventory rows.
- Runs final validation.
- Reports progress percentage and manual-test readiness to the user.

The coordinator should not delegate the current critical-path decision. If the
next local step depends on an answer, the coordinator usually investigates it
directly.

### C++ Explorer

Recommended reasoning: high.

Mode: read-only.

Responsibilities:

- Inspect only `/home/server/woltk-trinity-legacy`.
- Find exact files, functions and line ranges.
- Follow callees when behavior is not in the top-level handler.
- Extract branch order, state writes, timers, packet sends, callbacks, cleanup
  and early returns.
- Flag legacy bugs or suspicious logic.

Output format:

```text
C++ anchor:
file:line-line
function:

Behavior:
- ...

Early returns:
- ...

Side effects:
- ...

Open questions:
- ...
```

The C++ Explorer must not say whether Rust is complete unless explicitly asked
to compare Rust too.

### Rust Explorer

Recommended reasoning: high.

Mode: read-only.

Responsibilities:

- Inspect current Rust implementation and tests.
- Identify existing fields, bridges, helpers and gaps.
- Identify risky duplicate state between canonical entities and session mirrors.
- Find test names and validation commands.
- Report likely edit locations without editing.

Output format:

```text
Rust locations:
- file:line summary

Current behavior:
- ...

Gap versus claimed scope:
- ...

Likely edit scope:
- ...

Existing tests:
- ...
```

The Rust Explorer must not trust Rust comments or roadmap checkboxes as proof.

### Implementation Worker

Recommended reasoning: high for gameplay/runtime, medium for narrow packet or
doc work.

Mode: write-capable, but only with explicit ownership.

Responsibilities:

- Implement one bounded behavior slice.
- Preserve C++ branch order and side effects.
- Add or update focused tests when assigned.
- Update only the files it owns.
- Leave unrelated dirty files untouched.
- Report changed paths and validation run.

Good assignment:

```text
Implement the represented GAMEOBJECT_TYPE_TRAP GO_READY target activation
branch. Own only crates/wow-world/src/session.rs and the relevant
r8-entities-miniphase docs. Do not edit loot.rs or packet crates.
Contrast against GameObject.cpp lines X-Y. Add a focused test.
```

Bad assignment:

```text
Finish GameObject runtime.
```

### Test Worker

Recommended reasoning: medium or high.

Mode: write-capable only for tests and fixtures.

Responsibilities:

- Add focused tests for a C++ branch.
- Add negative tests for early returns.
- Build fixtures without changing production logic unless explicitly allowed.
- Verify filters actually run tests.
- Prefer behavior assertions over implementation-detail assertions.

The Test Worker is useful when production edits and test edits can be separated
cleanly. In large shared files such as `crates/wow-world/src/session.rs`, a
single writer is often safer.

### Reviewer

Recommended reasoning: high.

Mode: read-only unless explicitly asked to patch.

Responsibilities:

- Review the final diff against the C++ anchors.
- Look for missing branches, fake defaults, stale mirrors and false-positive
  tests.
- Check roadmap wording for overclaiming.
- Verify validation commands match the changed scope.

Review output format:

```text
Findings:
1. severity - file:line - issue

C++ anchors checked:
- ...

Residual gaps:
- ...

Verdict:
accept / partial / reject
```

## Standard Microphase Flow

Every migration microphase should follow this loop.

### 1. Select a Slice

The coordinator chooses a small behavior-complete slice.

Good slice:

```text
GameObject::Update GO_NOT_READY chest restock branch.
```

Bad slice:

```text
All GameObject runtime behavior.
```

Slice criteria:

- Has exact C++ anchor.
- Can be tested.
- Has bounded Rust targets.
- Does not require many unrelated subsystems unless those are already present.
- Can be documented honestly as complete, partial or represented.

### 2. Parallel Investigation

Run C++ Explorer and Rust Explorer in parallel when available.

The coordinator may continue local non-overlapping work, such as reading
roadmap context or checking existing tests, but should not duplicate the exact
assigned exploration.

Minimum evidence before implementation:

- C++ file and line range.
- Relevant C++ branch order.
- Required state writes.
- Required side effects.
- Rust current state and gaps.
- Focused test plan.

### 3. Scope Decision

The coordinator classifies the slice:

- `wire`: packet serialization/deserialization.
- `state`: entity fields, flags, timers, counters.
- `runtime`: live behavior and state transitions.
- `side_effect`: packets, callbacks, spell/aura/combat/threat/movement effects.
- `bridge`: temporary represented behavior until canonical ownership exists.
- `scaffold`: data shape only.

Only the declared scope can be marked complete. A bridge can be complete as a
bridge while still leaving canonical runtime gaps.

### 4. Implementation

Use one writer for files with high conflict risk:

- `crates/wow-world/src/session.rs`
- `crates/wow-world/src/handlers/loot.rs`
- large migration inventory files

Use parallel workers only when write sets are disjoint:

- Worker A: `crates/wow-packet`
- Worker B: `crates/wow-entities`
- Worker C: docs only

Implementation rules:

- Preserve C++ branch order.
- Preserve C++ early returns.
- Preserve timer semantics and default fallbacks.
- Preserve packet shape and side-effect order.
- Use canonical Rust entity state when available.
- Use represented/session mirrors only as explicit temporary bridges.
- Do not invent behavior from Rust tests.
- Do not broaden the slice while editing.

### 5. Tests

Every closed slice needs tests matching risk.

Minimum tests:

- Positive behavior path.
- Negative or early-return path when representable.
- State mutation.
- Side effect or absence of side effect.
- Regression test for any previous false assumption.

For packet work:

- Prefer golden byte assertions.
- Assert opcode and payload order.

For runtime work:

- Assert final state and non-effects.
- Assert timers are armed or cleared.
- Assert represented packet/effect hooks.

For bridge work:

- Assert canonical state wins over fallback when canonical state exists.
- Assert fallback only applies when canonical state is missing.

Test filters must be real. If a command reports `0 tests`, rename tests or
change the filter before documenting it.

### 6. Documentation

Update the relevant migration inventory and module docs in the same slice.

Required doc content:

- New task ID or updated task note.
- C++ anchor paths.
- Rust target paths.
- Acceptance summary.
- Validation command.
- Remaining gaps.

Do not remove remaining gaps just because a represented bridge exists.

### 7. Validation

Focused validation first:

```bash
cargo fmt
cargo test -p <crate> <focused_filter>
```

Integration validation before reporting complete:

```bash
cargo check -p world-server
git diff --check
git status --short --branch
```

Use wider test suites when touching shared crates:

```bash
cargo test -p wow-entities
cargo test -p wow-packet
cargo test -p wow-world <domain_filter>
```

Do not start live servers unless the user explicitly asks.

### 8. Review

For non-trivial changes, use a Reviewer or perform the same review locally.

Review checklist:

- Does every changed behavior have a C++ anchor?
- Did the implementation preserve branch order?
- Are early returns covered?
- Are tests asserting behavior, not only internal convenience?
- Did docs overclaim?
- Are all remaining gaps explicit?
- Is there any stale fallback overriding canonical state?
- Did the change touch unrelated files?

### 9. Commit Policy

The preferred checkpoint is one behavior-complete hito per commit, after all
validation passes.

Commit message shape:

```text
port: mirror gameobject chest restock update
```

Commit body should mention:

- C++ anchor.
- Rust scope.
- Tests run.
- Remaining gaps when important.

Do not commit unrelated dirty changes. If the tree contains prior accumulated
work, either keep developing in the current batch or ask for an explicit
solidification/commit step.

## Role Allocation by Work Type

### Packet and Opcode Work

Recommended team:

- C++ Explorer high.
- Rust Explorer medium.
- Implementation Worker medium.
- Reviewer high.

Best parallelization:

- One worker owns `crates/wow-constants`.
- One worker owns `crates/wow-packet`.
- Coordinator owns integration into `wow-world`.

Required evidence:

- C++ packet class and write/read method.
- Opcode enum source.
- Byte order and packed GUID rules.
- Golden or focused byte test.

### Entity State Work

Recommended team:

- C++ Explorer high.
- Rust Explorer high.
- Implementation Worker high if write set is isolated.
- Reviewer high.

Good worker ownership:

- `crates/wow-entities/src/<entity>.rs`
- matching module tests
- docs row

Coordinator should integrate when state touches `WorldSession` and canonical
map ownership at the same time.

### WorldSession Runtime Bridges

Recommended team:

- C++ Explorer high.
- Rust Explorer high.
- Coordinator implements, or one Implementation Worker high with very narrow
  ownership.
- Reviewer high.

Reason:

`crates/wow-world/src/session.rs` is a conflict-heavy file. Multiple workers
editing it concurrently are usually slower than one careful writer.

Best use of harness here:

- Explorers gather evidence in parallel.
- Reviewer audits the final diff.
- Coordinator writes the patch.

### Loot and GameObject Runtime

Recommended team:

- C++ Explorer high for `GameObject.cpp`, `Loot.cpp`, `LootMgr.cpp`.
- Rust Explorer high for `session.rs`, `handlers/loot.rs`, `wow-entities`.
- Coordinator implements central state machine changes.
- Test Worker high only if tests can be isolated cleanly.

Special rules:

- Always inspect the C++ state machine branch and callees.
- Track `LootState`, `GoState`, flags, cooldowns, restock, despawn and owner.
- Explicitly document represented-vs-canonical ownership.
- Never mark canonical behavior complete if only session-local represented
  behavior exists.

### Spells, Auras and Combat

Recommended team:

- C++ Explorer high or xhigh.
- Rust Explorer high.
- Implementation Worker high only for pure helpers or isolated modules.
- Reviewer high or xhigh.

Special rules:

- Do not fake spell execution with effect logs unless the task is explicitly a
  represented bridge.
- Preserve side-effect ordering.
- Negative tests are mandatory for C++ guard returns.
- Canonical `Unit`/`Player` state must win over session mirrors.

### Database, DB2 and Config

Recommended team:

- C++ Explorer medium/high.
- Rust Explorer medium.
- Implementation Worker medium.
- Test Worker medium.

Required evidence:

- C++ loader or statement.
- Table/field mapping.
- Default and invalid-value behavior.
- Fixture or focused unit test.

### E2E Bot Harness Work

Recommended team:

- C++ Explorer high for client-visible behavior.
- Rust Explorer high for auth/login/world flow.
- Implementation Worker high for server changes.
- Test Worker high for bot scripts and fixtures.

The Rust bot at `/home/cdmonio/projects/wow-test-bot/rust-bot/` should be used
during the port, but only as an E2E regression harness. It is not a correctness
oracle.

Minimum gate before relying on the bot:

```text
BNet auth -> world auth -> CMSG_ENUM_CHARACTERS -> CMSG_PLAYER_LOGIN
```

Each bot failure must be triaged against C++:

- Rust bug.
- Bot assumption mismatch.
- Missing migration task.
- Legacy C++ bug worth fixing in both trees.

## Harness Scheduling Model

### Small Slice

Use no subagents or one C++ Explorer.

Best for:

- One branch in a known function.
- One packet serializer.
- One doc update.
- Simple regression fix.

### Medium Slice

Use:

- C++ Explorer high.
- Rust Explorer high.
- Coordinator implements.
- Optional Reviewer high.

Best for:

- State-machine branch.
- Handler behavior with tests.
- Small canonical bridge.

### Large Slice

Use:

- C++ Explorer high.
- Rust Explorer high.
- Test Worker high.
- Implementation Worker high for isolated files only.
- Reviewer high.

Coordinator must split the slice before implementation.

### Audit Slice

Use:

- Rust Explorer high for diff inventory.
- C++ Explorer high for claimed behavior.
- Reviewer high for final findings.
- Coordinator applies fixes.

Best for reviewing work from another AI or large untrusted commit ranges.

## Concurrency Rules

Use parallel agents when tasks are independent.

Good parallel tasks:

- C++ behavior extraction.
- Rust gap inventory.
- Test fixture research.
- Packet golden-vector research.
- Docs consistency review.

Avoid parallel writers on:

- `crates/wow-world/src/session.rs`
- `crates/wow-world/src/handlers/loot.rs`
- migration TSV inventory files
- any file already being edited by the coordinator

If two workers must edit the same module, split by file or wait.

## Ownership Contract for Workers

Every worker prompt must state:

- Scope.
- Files or modules owned.
- Files forbidden.
- C++ anchors to use.
- Tests to add or run.
- Required final report.

Template:

```text
Task:
Port [specific C++ branch].

C++ anchors:
- /home/server/woltk-trinity-legacy/... lines X-Y

Owned files:
- crates/...

Do not edit:
- ...

Rules:
- Preserve C++ branch order.
- Add focused tests.
- Do not mark roadmap complete beyond this scope.
- Do not revert unrelated changes.

Final report:
- Changed files.
- Behavior implemented.
- Tests run.
- Remaining gaps.
```

## Reviewer Checklist

Reviewer must answer:

- What exact C++ lines prove this behavior?
- Does Rust implement all branches in the stated scope?
- Are unimplemented branches documented as gaps?
- Are C++ early returns tested?
- Are timers and cleanup modeled correctly?
- Are packets or effects emitted in C++ order?
- Is canonical state used where available?
- Does any fallback override canonical `None` or empty state?
- Do tests actually run under the documented filter?
- Does the roadmap overclaim?

## Progress Reporting

The coordinator should report progress as an estimate, not a guarantee.

Recommended categories:

- `overall migration`: whole C++ -> Rust port.
- `current phase`: current roadmap phase.
- `current domain`: e.g. GameObject runtime.
- `current microphase`: current numbered slice.

Progress must not be based only on number of checkboxes. It should consider:

- C++ surface area still unported.
- Runtime depth versus scaffolding.
- Canonical ownership versus session bridges.
- Test quality.
- Manual/E2E readiness.
- Known gaps in docs.

Manual-test readiness should be reported separately:

```text
Manual test point: not yet.
Progress: ~80% overall, ~X% GameObject runtime bridge.
Reason: current changes are represented runtime and unit-tested, but not enough
for a new client-visible scenario beyond the already-proven login smoke.
```

## When to Stop and Solidify

Stop and solidify when:

- A behavior-complete microphase passes validation.
- The dirty tree becomes too large to review comfortably.
- Multiple docs and code files changed across domains.
- User asks for commit/push/merge.
- A manual test point is ready.
- A subagent produced useful work that must be audited before more work lands.

Solidification checklist:

```bash
cargo fmt
cargo check -p world-server
git diff --check
git status --short --branch
git diff --stat
```

Then commit only the intended changes.

## Failure Modes to Avoid

- Treating Rust tests as proof without C++ contrast.
- Closing a roadmap item for a data field with no runtime behavior.
- Having many workers edit `session.rs` at the same time.
- Forgetting C++ early returns.
- Testing only success paths.
- Letting represented bridges masquerade as canonical ownership.
- Using bot harness success as a correctness oracle.
- Overwriting user or unrelated changes.
- Running live servers during normal development without request.
- Reporting a percentage without saying whether it is overall, phase or domain.

## Default Harness Plan Until Completion

For the rest of the migration, use this default structure:

1. Coordinator selects one roadmap gap.
2. C++ Explorer high extracts exact behavior.
3. Rust Explorer high extracts current implementation and gaps.
4. Coordinator defines scope and write ownership.
5. One writer implements the slice.
6. Test Worker adds tests only if write set is independent.
7. Reviewer audits the final diff for non-trivial changes.
8. Coordinator updates docs, validates and reports.
9. Commit/push/merge only when explicitly requested or when solidification is
   part of the agreed workflow.

For `WorldSession`-heavy gameplay slices, prefer:

```text
Explorers in parallel, coordinator writes, reviewer audits.
```

For isolated packet/entity/data slices, prefer:

```text
Explorers in parallel, worker writes isolated module, coordinator integrates.
```

This balance is the most efficient for this repo because it uses the harness for
parallel evidence gathering and review while avoiding costly merge conflicts in
the central runtime files.
