---
name: refactor-rustycore-safely
description: Implement or review behavior-preserving RustyCore restructuring with C++ fidelity and capture-clean validation. Use for module or file splits, moving methods or types, extracting application services, catalogs, repositories, runtime handles, or private substates, shrinking session/handlers/map/player/world-server/data/packet/QA hotspots, changing dependency edges, or promoting a stable module into a crate without intending gameplay or protocol changes. Do not use to hide a behavior change or legacy bug fix inside a refactor; design unclear ownership first with design-rustycore-architecture.
---

# Refactor RustyCore Safely

Execute one small structural slice while preserving runtime ownership, database atomicity, packet
bytes/routing, C++ phase order, and public behavior.

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
4. Inspect the exact C++ source for the moved responsibility and the complete current Rust call
   path before editing.
5. Inspect the issue, branch, worktree, related review history, and existing tests.

If the canonical owner or desired dependency direction is not explicit, stop implementation and
use `$design-rustycore-architecture`.

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

Do not combine class 4 with classes 1–3. Prefer separate PRs for relocation, boundary change, and
ownership migration when the diff would otherwise obscure review.

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
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p <affected-crate>
./tools/pr-preflight.sh quick origin/3.4.3
```

Choose the focused test target from `cargo metadata` instead of assuming every package has a
library:

```bash
# Library target:
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --lib
# Binary target such as world-server or bnet-server:
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --bin <binary>
# Integration-test target:
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p <package> <focused-test> --test <target>
```

After committing to a clean HEAD and before an authorized push, run:

```bash
./tools/pr-preflight.sh full origin/3.4.3
```

Treat the capture-diff harness inside `full` preflight as mandatory for every PR. Run a fresh
action-specific capture or live bot QA when the owning issue requires it. A file move is not
evidence that observable behavior stayed equal.

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

### 7. Publish only through the repository workflow

Use one issue, one linked branch, and one PR into `3.4.3`. Do not push unless the user asks. After
push, open the PR immediately with `Closes #<issue>` and wait for CI plus a clean Codex reviewer
verdict on the current HEAD. Address or explicitly defer every actionable review and resolve its
thread before merge.

## Stop conditions

Stop and separate the work when:

- the move reveals an undocumented behavior difference;
- C++ and Rust ownership disagree and the target is not already decided;
- preserving behavior requires a new mirror or cross-layer dependency;
- a test fails for a reason that has not been contrasted with C++;
- the diff combines structural movement with a gameplay/protocol change;
- the target overlaps unrelated dirty work that cannot be isolated safely.

Report the evidence and propose the smallest next slice instead of forcing the refactor through.
