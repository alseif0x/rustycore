# RustyCore architecture decision rules

Use these rules as constraints, not as a substitute for inspecting the current HEAD and exact C++
behavior.

## Target dependency direction

Dependencies should point inward:

```text
composition roots
    world-server, bnet-server
        |
adapters
    network, SQL/DB2 loaders, packet presentation, scripts, recast
        |
application
    session lifecycle, commands, use cases, transactions, event orchestration
        |
runtime and domain
    map, entities, movement, combat, spell, AI, loot, social, PvP
        |
foundation
    core, constants, math, collections
```

The composition root may know concrete adapters. Domain/runtime must not know sockets, SQL rows,
configuration files, or client packet publication.

Current crates may mix these roles. Treat the diagram as a migration direction, not permission for
a big-bang rewrite.

## Canonical ownership map

| Responsibility | Target owner |
|---|---|
| TCP/TLS, crypto, framing, socket queues | `wow-network` |
| Packet wire DTOs and serialization | `wow-packet` |
| Opcode adaptation and session lifecycle | `wow-world::session` / packet handlers |
| Cross-domain use cases and transaction orchestration | `wow-world::application` |
| Active object storage, grids, tick, respawn, visibility phases | canonical `wow-map` runtime |
| Player, Creature, Item state and local invariants | `wow-entities` |
| Mutable Unit combat/threat relations, timers, attackers, and current victim | `wow-entities::Unit` / `CombatSubsystem` |
| Pure combat/threat calculations over explicit inputs | `wow-combat` after its API stabilizes |
| Pure spell validation/effect resolution | `wow-spell` after its API stabilizes |
| Loot generation and authority rules | `wow-loot` |
| Immutable game catalogs | typed catalog modules in `wow-data` |
| SQL/DB2 loading and repository implementations | adapters over `wow-database` |
| Startup, configuration, task supervision, shutdown | `world-server` / `bnet-server` |
| QA scenarios and reporting | `tools/wow-test-bot` scenario modules |

Use C++ ownership as a semantic cross-check:

- `WorldSession`: account/session, sockets, queues, time sync, login/logout, opcode adaptation.
- `Player`: character-owned inventory, quests, skills, reputation, collections, and persistent
  gameplay state.
- `Unit`: mutable combat references, auras, threat lists, attackers, timers, control, and unit
  state.
- `Spell`: one cast's validation and effect execution.
- `Map`: objects, grid, respawn, global tick, visibility, and update phase order.
- `ObjectMgr` and stores: global immutable/shared data, not copies per session.

Do not reproduce C++ header/source layout mechanically.

Keep mutable combat/threat state in the canonical `Unit`. Let a future `wow-combat` crate compute
deterministic policies or outcomes over explicit snapshots; do not let it become a second runtime
owner.

## Module or crate

Create or retain a private module when:

- ownership or API is still changing;
- code changes with its caller;
- extraction would introduce cyclic concepts or broad re-exports;
- the only benefit is a smaller file.

Promote a module to a crate only when all are true:

- it has one stable semantic owner;
- it exposes a small command/query/model API;
- dependencies point only toward foundation or lower domain contracts;
- it has meaningful focused tests or build isolation;
- at least one consumer benefits from the boundary;
- moving it does not require infrastructure dependencies in the domain.

Do not populate `wow-combat`, `wow-spell`, `wow-pvp`, `wow-social`, `wow-achievement`, or
`wow-ecs` merely because the directories exist.

## Port or trait

Introduce a trait when:

- multiple real adapters exist; or
- a deterministic fake is needed to test an application use case; or
- dependency inversion cannot be achieved with a concrete value or function.

Typical useful ports include repositories, clock, RNG, packet/event sink, navigation, and world
runtime handle. Avoid one-method traits that only wrap an internal call.

## Task, lock, or channel

Use an owner task/channel when one task must serialize state or I/O. Make capacity, overload
behavior, ordering, shutdown, and durable-versus-best-effort semantics explicit.

Use a synchronous lock only for short sections with no `.await`. Never send packets or perform SQL
while holding a map lock. Avoid adding `Arc<Mutex<_>>` to escape an unclear owner.

Do not parallelize map simulation until each mutable state has one owner. Preserve the C++
session → respawn → object update → publication phase order.

## Mirror ledger

For every transitional mirror, record:

```text
State:
Canonical owner:
Temporary mirror:
Writers:
Readers:
Sync direction:
Conflict rule:
Why the mirror still exists:
Test that guards it:
Deletion condition:
Owning issue:
```

Reject a new mirror without all fields.

## Public API budget

- Keep modules private by default.
- Re-export only stable inter-crate contracts.
- Prefer command, query, outcome, and event types over exposing internal maps or locks.
- Do not widen visibility to move tests; keep unit tests beside private code or add a narrow
  behavior-level seam.
- Treat every `pub` item as a compatibility and coupling cost.

## Architecture slice rules

1. Assign the owner.
2. Freeze observable behavior with tests/captures.
3. Move code without changing ownership.
4. Introduce the target boundary.
5. Redirect one writer and all related readers.
6. Delete the superseded mirror or document its precise remaining boundary.
7. Only then change behavior or concurrency.

Keep structural changes and behavior repairs in distinct commits. Repairs of regressions
introduced by the current authorized slice belong to that slice. Pre-existing unrelated behavior
changes require separate scope; use a separate issue/PR when independently deliverable.
