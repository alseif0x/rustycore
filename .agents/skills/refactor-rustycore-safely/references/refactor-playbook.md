# RustyCore safe-refactor playbook

Use this playbook after ownership and dependency direction are decided.

## Slice template

Record before editing:

```text
Issue:
Dominant change class:
C++ semantic owner and anchors:
Current Rust owner:
Target Rust owner:
Observable contract frozen by:
Persistence contract:
Concurrency/lock contract:
Opcode registrations affected:
Public paths affected:
Focused tests:
Capture/live QA requirement:
Explicit non-goals:
Bridge or re-export deletion condition:
```

Reject a slice whose target owner, frozen contract, or non-goals are unclear.

## Mechanical feature split

Use for `handlers/misc.rs`, packet families, QA scenarios, or a cohesive impl block.

1. Identify one complete feature family and all registrations/tests.
2. Create a private module in the same crate.
3. Move definitions and tests without renaming or reordering logic.
4. Keep imports explicit; do not replace compile errors with broad `pub`.
5. Confirm every `(opcode, SessionStatus, PacketProcessing)` tuple is unchanged.
6. Add or update a table-driven test for the exact expected opcode set, metadata, handler names,
   and uniqueness. Counts alone cannot detect replacement by the wrong opcode.
7. Run focused tests, crate tests, quick preflight, then full preflight after commit.

Keep `impl WorldSession` across modules when that preserves C++ handler naming. Do not add
`CharacterHandlers`-style traits solely for organization.

## Composition-root split

Use for `world-server/main.rs` or `bnet-server`.

Extract in this order when seams permit:

1. configuration and CLI parsing;
2. immutable catalog/bootstrap loading;
3. concrete repository/DB writer adapters;
4. session factory;
5. runtime task supervision and routing;
6. realm lifecycle and shutdown.

Keep `main` responsible for construction and supervision. Do not move gameplay ownership into a
binary module merely because it is convenient to wire there.

## Shared-service boundary

Use when replacing `SessionResources` or setter forests.

1. Group immutable catalogs by domain.
2. Group repositories by transaction boundary, not table count.
3. Group runtime handles by owner.
4. Validate mandatory production resources once in bootstrap.
5. Provide an explicit test builder instead of making every production dependency optional.
6. Migrate one resource group and its consumers per slice.
7. Remove the old setters/fields in the same slice when possible.

Do not turn `WorldServices` into an untyped map or allow every use case to depend on the entire
aggregate. Give each use case the smallest typed dependency set.

## Aggregate modularization

Use for `Map`, `Player`, `Unit`, or another legitimate aggregate.

- Keep one aggregate identity and one canonical state.
- Split private substates and impl capabilities by responsibility.
- Preserve invariant enforcement at the aggregate boundary.
- Do not create submanagers that own copies of aggregate state.
- Keep C++ update phase ordering explicit in the top-level method.

Candidate private modules:

```text
map/{objects,grid,respawn,visibility,relocation,transport,scripts,weather}
player/{inventory,progression,quests,skills,social,pvp,update_fields}
session/{identity,transport,lifecycle,time_sync,visibility,dispatch,legacy_bridge}
```

These names are guides, not permission for a bulk move.

## Handler-to-use-case extraction

Extract one complete vertical operation:

```text
decode packet
    → session/status gate
    → command input
    → application use case
    → domain plan/outcome
    → repository transaction
    → canonical mutation
    → packet/event presenter
```

Keep protocol DTOs at the adapter boundary. Keep SQL rows and prepared statements in repository
implementations. Keep pure gameplay decisions out of both.

Add tests at three levels as applicable:

- unit tests for pure rules and negative branches;
- application tests for transaction and canonical mutation ordering;
- packet/capture tests for bytes, connection, routing, and visibility.

## Ownership migration

1. Name the source and target owners.
2. Freeze both single-session and multi-session behavior.
3. Redirect one writer and every dependent reader.
4. Preserve C++ phase order.
5. Delete the old state and sync path immediately, or update the mirror ledger with a precise
   remaining boundary.
6. Prove the mutation happens exactly once.

For runtime work, follow `docs/migration/adr-runtime-tick-ownership.md`. Never enable a global tick
in addition to a session tick for the same responsibility.

## Promote a module to a crate

Promote only after the API is stable:

1. list intended public types/functions;
2. prove dependencies point downward;
3. ensure domain code does not import network, SQL, packet presentation, or binary configuration;
4. move focused tests with the module;
5. add the minimum workspace dependencies and inherit workspace lints;
6. keep adapter conversion in the caller/application layer;
7. inspect `cargo tree -p <new-crate>` and public re-exports.

Do not use a new crate to conceal cyclic domain concepts.

## Verification matrix

| Change surface | Minimum checks |
|---|---|
| Pure private move | format, diff check, focused tests, crate tests |
| Opcode handler move | exact-set registration test, handler tests, packet tests |
| Persistence orchestration | positive/negative/deadlock/commit-unknown tests, DB statement order |
| Map/runtime ownership | one-owner/multi-session tests, lock audit, tick-order tests |
| Packet/presenter move | byte, connection, recipient, order, visibility, capture-diff |
| Public API or crate edge | downstream check/tests, `cargo tree`, visibility/re-export audit |
| QA bot split | scenario JSON fields, CLI compatibility, representative smoke dry run |

Use the repository preflight as the final aggregate gate; its capture-diff harness is mandatory for
every PR. Require fresh scenario capture or live bot QA only when the issue calls for it. Do not
replace focused reasoning with a green build.
