---
name: refactor-rustycore-safely
description: Implement or review behavior-preserving RustyCore restructuring with C++ fidelity and capture-clean validation. Use for module or file splits, moving methods or types, extracting application services, catalogs, repositories, runtime handles, or private substates, shrinking session/handlers/map/player/world-server/data/packet/QA hotspots, changing dependency edges, or promoting a stable module into a crate without intending gameplay or protocol changes. Do not use to hide a behavior change or legacy bug fix inside a refactor; design unclear ownership first with design-rustycore-architecture.
---

# Refactor RustyCore Safely

Execute one small structural slice while preserving runtime ownership, database atomicity, packet
bytes/routing, C++ phase order, and public behavior.
Use small slices as implementation boundaries, not automatic conversation endpoints; follow
AGENTS.md's autonomy, approval and completion rules throughout the authorized task.

## Select review or execution mode

Choose the mode from the user's requested outcome before any mutation:

- **Review mode:** inspect and report only. Do not edit, stage, commit, push, publish, reply to
  reviews, or resolve threads. Report prioritized findings with a narrow file/line range, the
  frozen contract that would be violated, supporting Rust/C++ or capture evidence, and the smallest
  safe correction. Separate patch-caused defects from pre-existing debt. Stop after the findings
  unless the user explicitly asks to implement them.
- **Execution mode:** the user asked to change or refactor code. Follow the implementation workflow
  below and remain within the authorized issue and worktree scope.

If the request is ambiguous between inspection and mutation, use review mode.

## Load the required context

1. Read the repository `AGENTS.md` completely and run its session kickoff.
2. Read `docs/migration/STATE.md` and the decision, risks, and task-relevant sections of any owning
   ADR. For `docs/migration/adr-runtime-tick-ownership.md`, do not load the entire historical
   progress log for a bounded refactor.
3. Read [references/refactor-playbook.md](references/refactor-playbook.md).
   For source/module decomposition, also read `docs/architecture/module-design-guidelines.md`
   completely; it owns the physical budgets, bounded exceptions and semantic/physical done criteria.
4. Inspect the exact C++ source for the moved responsibility and the complete current Rust call
   path before editing.
5. Inspect the issue, branch, worktree, related review history, and existing tests.

If the canonical owner or desired dependency direction is not explicit, pause the affected
mutation and use `$design-rustycore-architecture` to investigate and design. Do not end safe
investigation merely because ownership needs clarification. Preserve any required design approval;
do not request the same approval again for an already approved design and implementation scope.

## Classify the change before editing

Choose exactly one dominant class:

1. **Mechanical relocation:** move code/tests, preserve owner, signatures, ordering, visibility,
   registration, and behavior.
2. **Boundary extraction:** introduce a module/service/port and redirect dependencies without
   changing the canonical state.
3. **Ownership migration:** redirect all writers/readers to one target owner and delete or ledger
   the old mirror.
4. **Behavior correction:** change observable or persistence behavior because C++ parity or a
   proven legacy defect requires it.

Separate structural changes and behavior repairs into distinct commits. Repairs of regressions
introduced by the current authorized slice belong to that slice; do not disguise them as moves.
Pre-existing unrelated behavior changes require separate scope. Within an approved macrodeliverable,
separate relocation, boundary and ownership changes through coherent commits and review checkpoints,
not automatic micro-PRs. Use another issue/PR only for independently deliverable, authorized scope;
do not treat file count or a large internal diff as permission to change the delivery agreement.

## Refactor workflow (execution mode)

### 1. Freeze the contract

List the behavior that must not change:

- opcode, `SessionStatus`, and `PacketProcessing`;
- packet bytes, connection, recipient gates, ordering, and timing boundary;
- C++ validation and phase order;
- SQL statements, transaction boundary, rollback/commit classification, and retry behavior;
- canonical owner, mutation count, lock order, and tick owner;
- public paths and downstream callers;
- positive, negative, concurrency, and failure-path tests.

Add or identify regression tests before moving fragile code.

### 2. Map the complete dependency surface

Use `rg` to find definitions, impl blocks, re-exports, call sites, registrations, tests, feature
gates, SQL statements, locks, channels, and docs. Inspect macros and `inventory::submit!` rather
than assuming a handler is registered because a match arm exists.

### 3. Make the smallest coherent edit

Move one feature family, state group, use case, or adapter seam at a time. Keep implementation
private. Use temporary re-exports only when they reduce unrelated churn, and record their removal
condition.

Do not:

- invent traits merely to split files;
- add `Arc<Mutex<_>>`, `RefCell`, global state, clones, or mirrors to satisfy the borrow checker;
- broaden `pub` visibility for convenience;
- alter serialization or SQL order during a move;
- leave two opcode registration sources;
- introduce unbounded channels or wait while holding a runtime lock.

### 4. Preserve application and persistence ordering

For atomic operations, preserve the established shape:

```text
validate and plan
    → execute transaction
    → classify commit outcome
    → apply canonical runtime state once
    → publish packets/events outside incompatible locks
```

Do not publish success before a durable commit or mutate runtime twice during reconciliation.

### 5. Validate continuously

Run format, diff checks, focused tests, and targeted checks during the edit. Use `PROTOC`
explicitly for protobuf-dependent crates. Run:

```bash
cargo fmt --all -- --check
git diff --check
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo check -p <affected-crate>
./tools/validation-v2 quick --base origin/3.4.3
```

Choose the focused test target from `cargo metadata` instead of assuming every package has a
library:

```bash
# Library target:
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --lib
# Binary target such as world-server or bnet-server:
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --bin <binary>
# Integration-test target:
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --test <target>
```

After committing to a clean HEAD and before an authorized push, run:

```bash
./tools/validation-v2 final --base origin/3.4.3
```

Apply AGENTS.md's capture-diff and runtime-QA triggers plus explicit issue acceptance requirements.
Distinguish regression tests from fresh action-specific captures. A file move is not evidence
that observable behavior stayed equal. Use `validation-v2`, not the retired `full` preflight.

### 6. Audit the finished diff

Verify:

- no code, test, registration, cfg branch, or documentation was accidentally dropped;
- opcode registration tests assert the exact expected set and metadata, not only counts or
  duplicate absence;
- moved tests still exercise the same private behavior or a deliberately narrower contract;
- no new upward crate dependency or public API leak appeared;
- no new state mirror, clone-sync path, task owner, or lock order appeared;
- no unrelated user/worktree changes entered the diff;
- source paths and C++ anchors in docs remain accurate;
- every compatibility re-export or bridge has a deletion condition.
- each completed responsibility meets both the semantic boundary and physical source/test policy;
  the remaining legacy size debt has specific exits, not a blanket Session/aggregate exemption;
- physical before/after counts and the independent logical-owner view are honest about progress;
  moved tests still run, and fixture/support files have not become the replacement monolith.

### 7. Publish only through the repository workflow

Use one issue, one linked branch, and one PR into `3.4.3`; reuse the branch when resuming.
Do not push unless the user asks. After an authorized push, open the PR immediately with
`Closes #<issue>`. Follow AGENTS.md's author-specific validation policy: for exactly `alseif0x`,
require the local final gate and focused evidence; do not wait for a non-skipped remote reviewer
verdict. For other authors, require the configured remote checks and review on the current HEAD.
Address or explicitly defer every actionable review and resolve its thread before merge.
Push or PR creation does not itself authorize merging.

## Stop conditions

Pause the affected mutation and investigate or separate the work when:

- the move reveals an undocumented behavior difference;
- C++ and Rust ownership disagree and the target is not already decided;
- preserving behavior requires a new mirror or cross-layer dependency;
- a test fails for a reason that has not been contrasted with C++;
- a commit mixes structural movement with a gameplay/protocol change instead of separating them;
- the target overlaps unrelated dirty work that cannot be isolated safely.

Continue safe inspection, reproduction, C++/capture comparison and diagnosis. Resume once the
uncertainty is resolved within the approved scope. Ask the user only when resolution requires
new authority, a material scope change, or a choice the evidence cannot settle. If unrelated dirty
work cannot be isolated safely, leave it untouched and report the blocker. Report the evidence
and separate any out-of-scope repair instead of forcing the refactor through.
