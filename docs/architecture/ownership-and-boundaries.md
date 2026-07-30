# RustyCore ownership and dependency boundaries

This document is the executable architecture baseline for the incremental refactor tracked by
issue #133. It records the current owners and intentional mirrors before code is moved. The
machine-readable dependency rules live in
`tools/architecture/dependency-policy.json`; `tools/architecture/check_architecture.py check`
enforces them.

## Decision

RustyCore remains a **modular monolith** while the port is incomplete:

1. preserve one mutable owner for each gameplay concept;
2. split large files into private, cohesive modules inside their current crate first;
3. move a responsibility only after its behavior, ordering, persistence, packet metadata, and
   capture contract are frozen;
4. promote a module to a crate only when its dependency direction and public API are already
   stable;
5. never create a second state mirror, a new `Arc<Mutex<_>>`, or a wider public API merely to make
   a physical extraction compile.

The C++ server is the source of truth for behavior, state ownership, and update ordering. Its
physical layout is not a target: TrinityCore compiles the large `server/game` tree as one library,
and files such as `Player.cpp`, `Unit.cpp`, and `Spell.cpp` remain large themselves. Rust modules
therefore may be smaller and more cohesive while preserving the same semantic owners.

This direction also follows Rust's native boundaries: modules control visibility inside a crate,
while Cargo packages are separate compilation and public-API boundaries. A line count is a signal
for review, not an architecture rule. The checker reports production/test hotspots but never
fails solely because a file crosses an arbitrary size.

## Dependency direction

Every workspace package has one dominant category:

| Category | Intended dependency direction |
|---|---|
| `foundation` | foundation only |
| `domain-runtime` | foundation and domain/runtime |
| `application` | foundation, domain/runtime, and application |
| `adapter-platform` | inward layers and other adapters |
| `composition` | all production layers; owns process wiring |
| `tooling` | may consume production layers but production must not consume tooling |

Three mixed boundary packages have a stricter direct-dependency allowlist:
`wow-network`, `wow-packet`, and `wow-data`. This prevents their current surfaces from growing
while responsibilities are extracted.

The policy's `exceptions` are a **ratchet, not an endorsement**. Each exception:

- describes an edge present in the current Cargo graph;
- names the issue responsible for deciding or removing it;
- permits no neighboring edge by implication;
- becomes an error as soon as the underlying dependency disappears, so obsolete debt cannot stay
  silently allowlisted.

When several ordered slices retire distinct uses of the same Cargo edge, `tracking_issue` names
the final slice that can remove the dependency and the reason lists every intermediate slice.
Closing an earlier slice must not leave an exception pointing at an already completed issue.

A new package, new upward edge, undeclared restricted-package edge, duplicate classification, or
obsolete exception fails the architecture check. A deliberate baseline change must update this
document and the JSON policy in the same reviewed commit.

## Current ownership and mirror ledger

“Writer” means the authority allowed to mutate the state, not every call site that requests a
mutation. “Clock/task” records who advances time-dependent behavior. A mirror row must have an
explicit synchronization direction and retirement condition; there is no implicit
last-writer-wins policy.

| Concept | Current owner and storage | Writers | Readers / delivery | Clock, lifetime, and synchronization | Retirement |
|---|---|---|---|---|---|
| Authenticated world connection | `wow-network::accept` and the connection task | socket/authentication task | `WorldSession` dispatch boundary | one connection; created after authentication and dropped on disconnect | Remains a network responsibility. Issue #134 narrows the constructor input to this boundary. |
| Gameplay session construction aggregate | `wow_network::accept::SessionResources` | `world-server` bootstrap constructs it; accept loop clones the `Arc` | connection setup copies stores, registries, DB adapters, and runtime handles into each `WorldSession` | process lifetime; no independent clock | #134 moves the gameplay aggregate to composition/application code. #136 then extracts a private session factory. |
| Session mailbox and connected-player registry | `wow_network::player_registry::{SessionCommand, PlayerRegistry}`; registry is a `DashMap<ObjectGuid, PlayerBroadcastInfo>` | session login/logout and state publication write the registry; session/global runtime producers enqueue mailbox and durable-rail commands | global routing reads registry snapshots; the owning session consumes its mailbox; chat/movement/combat/loot producers request fanout through these seams | registry lives for the process; mailbox lives for a connection/session task; FIFO command order is observable | #138 moves this application coordination seam out of `wow-network`. |
| Group registry | `wow_network::group_registry::GroupRegistry`, currently a `DashMap<u64, GroupInfo>` | group handlers and group timer/ready-check paths | group handlers, connected-player fanout, world-server ready-check loop | process lifetime; timed group work is driven outside the network listener | #137 moves gameplay group ownership out of `wow-network`. |
| Legacy creature runtime | shared `wow_world::MapManager` behind `Arc<RwLock<_>>` | production `GlobalLegacy` runtime tick plus explicit spawn/respawn bridges; `Session` writer exists only for tests and the diagnostic config override | world handlers, global runtime bridge, visibility/fanout routing | process lifetime; production startup defaults `RuntimeTickOwner` to `GlobalLegacy` and uses the configured map-update interval; session ticks read the shared owner and must skip to prevent double resolution | Retire only method-by-method into the canonical map/entity runtime under `docs/migration/adr-runtime-tick-ownership.md`. |
| Canonical map runtime | `wow_map::MapManager` | canonical global map loop, grid/spawn/respawn paths, explicit selected legacy-result adapters | world-server orchestration and session map/player bridges | process lifetime; canonical loop uses the configured map interval; preserves the C++ `Map::Update` phase order represented by the ADR | Becomes the sole map/entity authority only after every migrated method has parity tests and the corresponding legacy writer is removed. |
| Creature legacy/canonical mirror | canonical loaded-grid records are mirrored into `wow_world::MapManager`; selected lifecycle, movement, aggro, attack-stop, melee, health, and respawn outcomes are explicitly bridged back to canonical state | named bridge functions in `world-server`, including `mirror_loaded_grid_creature_to_legacy_like_cpp` and the `run_legacy_creature_*_and_deliver_once_like_cpp` family | both runtimes and post-lock packet/command delivery | load/respawn synchronization begins canonical → legacy; only explicitly modelled runtime outcomes travel legacy → canonical; delivery occurs after map locks are released | Remove one bridge only when its destination runtime becomes authoritative for that whole transition. Never add a generic bidirectional sync. |
| Represented player gameplay state | mostly fields on `wow_world::WorldSession`; canonical value types and partial state also exist in `wow_entities::Player` | session handlers and session update code | packet builders, persistence helpers, `PlayerRegistry` summaries, canonical snapshot bridges | connection/selected-character lifetime; `canonical_player_entity_snapshot_*_like_cpp` currently rebuilds a `Player` snapshot from represented session fields | #133's later ownership work must migrate one complete responsibility at a time until `Player` is the mutable gameplay owner and `WorldSession` is only the connection/session bridge. |
| Handler registration contract | link-time `inventory::iter<PacketHandlerEntry>` consumed by `wow_handler`/`WorldSession` | static `inventory::submit!` declarations | dispatch table and session update driver | compile/link lifetime; no mutable clock | The distribution across modules may change. The exact opcode set, opcode value, `SessionStatus`, `PacketProcessing`, and handler name are snapshot-guarded and must change deliberately. |

## Non-negotiable runtime invariants

The refactor campaign must preserve:

- exactly one tick owner for each state transition; no dual session/global resolution;
- C++ update ordering, especially session/map phases and creature `Unit` → threat → AI → melee
  sequencing;
- no packet or cross-session command delivery while a map lock is held;
- exact active handler registration metadata: opcode value/name, `SessionStatus`,
  `PacketProcessing`, and handler name;
- persistence mutation order and failure semantics;
- packet bytes, connection choice, and capture-diff behavior unless an intentional compatibility
  deviation is separately approved and documented;
- no new public API, state copy, synchronization primitive, or crate dependency solely for file
  movement.

## Guardrail commands

```bash
python3 tools/architecture/check_architecture.py self-test
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py hotspots --limit 20
./tools/pr-preflight.sh architecture
```

`self-test` proves both a permitted downward edge (`wow-combat → wow-math`) and a rejected upward
edge (`wow-map → wow-network`). `check` evaluates the real locked Cargo workspace, rejects stale
exceptions, and prints informational source hotspots.

The exact handler snapshot is `tools/architecture/world-handler-contract.tsv`. Its Rust test
enumerates the linked `inventory` registry instead of parsing source text, so macro-generated
registrations are included. The existing C++ metadata regression remains the semantic authority;
the new snapshot prevents an unreviewed Rust registration addition, removal, rename, or metadata
change.

## Deliberate baseline updates

Do not regenerate a baseline merely to make CI green.

1. Identify the semantic owner and contrast any behavior-affecting change with C++.
2. Explain why the dependency or handler-contract change is intentional in the issue and PR.
3. Add or update focused positive/negative tests before changing the baseline.
4. For a dependency exception, provide a concrete tracking issue and reason. Remove an exception
   in the same commit that removes its final edge.
5. For a handler snapshot, inspect the exact added/removed/changed row and retain the C++ metadata
   contrast test.
6. Run the architecture self-test, architecture check, focused Rust tests, and full PR preflight.

## Refactor sequence

The child issues of #133 execute in semantic order, regardless of their GitHub creation number:

1. #135 — executable boundary guardrails (this baseline);
2. #134 — remove gameplay `SessionResources` from the listener;
3. #136 — private world-server session factory;
4. #138 — session mailbox/player registry ownership;
5. #137 — group registry ownership;
6. #139 — extract Calendar handlers from `misc.rs`;
7. #140 — extract the `WorldSession` update/dispatch driver.

Each issue is one branch and one PR. The next issue starts only after the current PR is
capture-clean where applicable, all actionable review is resolved, required CI is green on the
current HEAD, and the PR is merged.
