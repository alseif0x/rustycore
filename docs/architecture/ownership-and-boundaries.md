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

Three mixed boundary packages have a stricter exact direct-dependency allowlist:
`wow-network`, `wow-packet`, and `wow-data`. This prevents their current surfaces from growing
while responsibilities are extracted. Every listed edge must remain present: once an extraction
removes it, the checker rejects the obsolete allowance so the dependency cannot later be
reintroduced without review.

The inward `foundation`, `domain-runtime`, and `application` categories also have an exact
per-package allowlist for direct third-party `normal` and `build` dependencies. The same external
ratchet protects the deliberately narrow `wow-network`, `wow-packet`, and `wow-data` adapters, so,
for example, raw network code cannot bypass its workspace allowlist by importing `sqlx` directly.
This is deliberately not a global taxonomy of crates.io: Cargo metadata cannot say whether an
arbitrary package is SQL, networking, configuration, process, or runtime infrastructure.
Instead, every new direct external dependency on a protected surface requires a reviewed policy
change. Existing utility libraries such as `rand` and adapter-owned infrastructure such as
`wow-network → tokio` remain explicitly allowed, while current inward `sqlx`/`tokio` leaks are
issue-linked exceptions. Other adapter and composition packages may integrate concrete
infrastructure; the workspace-edge policy still constrains which RustyCore layers they consume.
Development-only dependencies are outside this production boundary. The checker uses Cargo's
resolved package IDs with all features enabled and no platform filter rather than package-name
equality. It validates package/source/version identity for every resolved external edge and
globally rejects duplicate JSON keys, duplicate Cargo package/node/member IDs, and multiple direct
package IDs collapsing to the same package/source/kind identity. On a protected surface, the
allowlist additionally requires the exact canonical crates.io registry source recorded in the
policy; a same-named path, Git, or alternate-registry package is a different edge and fails. Thus
renames, source substitution, malformed or ambiguous metadata, and inactive target-specific
dependencies cannot bypass the boundary. Workspace members themselves must be source-null
`path+…` packages with coherent name/version identity, so registry or Git packages cannot be
mislabelled as internal members. The self-test pins the exact
`cargo metadata --locked --all-features --format-version 1` command, and the checker also fails
closed if Cargo's `workspace.default-members` stops covering every member, because
`--all-features` would no longer prove optional dependencies of omitted packages.

The policy's workspace and external `exceptions` are a **ratchet, not an endorsement**. Each
exception:

- describes an edge present in the current Cargo graph;
- names the issue responsible for deciding or removing it;
- permits no neighboring edge by implication;
- becomes an error as soon as the underlying dependency disappears, so obsolete debt cannot stay
  silently allowlisted.

When several ordered slices retire distinct uses of the same Cargo edge, `tracking_issue` names
the final slice that can remove the dependency and the reason lists every intermediate slice.
Closing an earlier slice must not leave an exception pointing at an already completed issue.

A new package, new upward edge, undeclared restricted-package edge, undeclared direct external
dependency in an inward package, duplicate classification, stale allowed dependency, or obsolete
exception fails the architecture check. A deliberate baseline change must update this document
and the JSON policy in the same reviewed commit.

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
| Effective skill metadata | `wow_data::SkillLineStore` owns final `SkillLine` identity/acquisition fields; `wow_data::SkillStore` owns final `SkillLineAbility` and `SkillRaceClassInfo` rows plus their derived indexes | `world-server` bootstrap composes WDC4 → official SQL → custom SQL → final removals once; no runtime writer | spell loaders and gameplay validation read immutable stores shared with sessions | process lifetime; `SkillLine` is composed first, then dependent rows are filtered and every index is rebuilt from final records in ascending ID order | Retire the specialized acquisition projections only when the general effective DB2 authority carries the same checked payload and coverage states. Never reactivate the raw WDC-only `SkillStore::load` path in production. |
| Effective spell-acquisition metadata | `wow_data::SpellAcquisitionCatalogLikeCpp`, a compact immutable projection of the seven acquisition source families | `world-server` bootstrap composes and publishes one `Arc`; no handler or session mutates it | derived spell-learning loaders now; trainer planning in #164; sessions receive the same `Arc`, not the seven raw stores | process lifetime; exact regular SpellInfo keys seed covered/zero distinction, while server-side keys without validated acquisition payload are explicitly indeterminate | Remove the specialized catalog, or feed it from the general store, once full effective `SpellInfo` payload authority exists. This row does not authorize packet, persistence, spell, skill, money, or battle-pet mutation. |
| Immutable spell-acquisition projection and application | `wow_world::spell_acquisition` owns the pure fixed-point plan plus its validation/transaction/publication boundary; the live player and Character DB remain the runtime/durable owners | planning mutates only a private ordered copy; #158 locks one character row, commits the complete durable result, then replaces runtime mirrors and publishes without another await | #157 consumes ordered primary-profession outcomes; #158 consumes the exact source/result plan and generic player `EffectLearnSpell`; #159 adds trainer charge/wrapper/visual orchestration | one acquisition operation; complete spell/skill/trait/override authority and exact slot occupancy are mandatory. Unknown COMMIT outcomes reconcile all spell/favorite/skill rows before publication; see [the detailed contract](spell-acquisition-plan.md) | Retire the specialized seam only when canonical `Player` methods expose the same atomic dry-run/apply contract. Never reconstruct capacity or criteria from a flat trigger list, sort profession outcomes, infer “no immunity” from missing runtime state, or publish before the durable boundary. |
| Handler registration and dispatch-arm contract | the sole `inventory::collect!(PacketHandlerEntry)` in `wow-handler`, link-time `inventory::iter<PacketHandlerEntry>` consumed by `wow_handler`/`WorldSession`, and the concrete `WorldSession::dispatch_packet` opcode arms | unconditional module-item `inventory::submit!` declarations owned logically by `wow_world::handlers`, plus the dispatcher implementation | dispatch table and session update driver | compile/link lifetime; no mutable clock | The distribution inside `crate::handlers` may change. The exact opcode set, opcode value, `SessionStatus`, `PacketProcessing`, handler name, and presence on both sides of dispatch are guarded and must change deliberately. This proves arm presence, not the semantics of each arm body. The three pre-existing one-sided entries are removal-ratcheted to #142. |

## Non-negotiable runtime invariants

The refactor campaign must preserve:

- exactly one tick owner for each state transition; no dual session/global resolution;
- C++ update ordering, especially session/map phases and creature `Unit` → threat → AI → melee
  sequencing;
- no packet or cross-session command delivery while a map lock is held;
- exact active handler registration metadata and dispatch-arm coverage: opcode value/name,
  `SessionStatus`, `PacketProcessing`, handler name, registration, and concrete opcode arm;
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

`self-test` pins the locked/all-features Cargo metadata command and proves a permitted downward
workspace edge (`wow-combat → wow-math`), a rejected
upward edge (`wow-map → wow-network`), a reviewed domain utility (`wow-map → rand`), concrete SQL
inside its adapter (`wow-database → sqlx`), and rejection of direct SQL, network, configuration,
process, and async-runtime additions to `wow-map`. It also exercises stale workspace exceptions,
external exceptions, external allowlist entries, and the raw-network rejection of direct SQL
(`wow-network → sqlx`) while retaining its reviewed Tokio runtime. Malformed/duplicate JSON and
Cargo identities, canonical/path/Git/alternate-registry origins, both valid Cargo Git-ID forms,
inactive target-specific dependencies, and ambiguous external identities are adversarially
covered. `check` evaluates the real locked Cargo workspace, including direct third-party
`normal`/`build` dependencies, rejects stale policy entries, and prints informational source
hotspots.

The exact handler snapshot is `tools/architecture/world-handler-contract.tsv`. Its Rust test
enumerates the linked `inventory` registry, so macro-generated registrations are included. A
production integration test enumerates the same registry with the library compiled without
`cfg(test)`, preventing test-only submissions from entering the reviewed contract. The four
`wow-handler` registry API tests are also integration tests so their `Ping` submission is not part
of a production library source.

The standalone Rust tool under `tools/architecture/handler-contract-check` parses source without
compiling `wow-world`. Full locked Cargo metadata with all features enabled supplies the reverse
normal-dependency closure of `wow-handler`; every production `lib`/`bin` module tree in that
workspace closure is audited, including optional, renamed, transitive, and target-specific normal
edges. Development/build-only edges are excluded. An unknown dependency kind or a non-workspace
package in that reverse closure fails closed because its production source cannot be proved by the
workspace audit. Independently, every production module graph in all workspace packages is scanned
for definitions, invocations, aliases, exports, and includes capable of emitting, forwarding, or
mounting hidden registration source. The cross-package surface is a closed baseline: exactly the
23 existing declarative exports in `wow-logging`, the six generated protobuf `include!` bodies in
`wow-proto`, and the seven existing non-handler inventory calls in `wow-script` are pinned by
package and source (exported names exactly; include/inventory bodies exactly). Any new export,
include, inventory call, meta-macro, or macro body that can synthesize `mod`, `macro_rules!`,
`PacketHandlerEntry`, an inventory registration path, or one of the audited registration macros
fails. This prevents an upstream workspace crate outside the reverse closure from exporting or
mounting a hidden generator that is later invoked by a handler-linked crate.

Ownership is logical, not a path-prefix convention: only sources mounted as
`wow_world::handlers` or descendants may submit `PacketHandlerEntry`. A file physically below
`src/handlers` but mounted as `crate::shadow` remains outside the owner. The cfg-independent module
walk follows explicit `#[path]` files, requires regular non-symlink `.rs` files inside their package
root, and rejects conditional `cfg_attr(path)`, `#[path]` in/under inline modules, and module
declarations nested in block/item bodies. These closed rules keep the ownership walk and handler
grammar on the same exact source set.

Inside the owner, direct submissions and the six audited one-entry registration macros must be
unconditional, private module items. `#[macro_export]`, reexports, unknown item macros, nested
handler-capable calls, macro-path or metavariable forwarders, aliases of inventory registration
macros, repetitions, and multi-arm generators fail closed. Outside the owner, `include!`,
handler-capable macro definitions/calls, and every `collect`, `submit`, or `__do_submit` inventory
registration-macro path/import/alias are rejected except for the exact non-handler/generated
surfaces pinned above. The only handler-registry exception is exactly one unconditional module-level
`inventory::collect!(PacketHandlerEntry)` at the `wow-handler` production Cargo `lib` target root
(not a file merely named `lib.rs` or a nested conditional module). The checker scans declarative
macro definitions and calls in all workspace production sources, but deliberately does not claim
arbitrary expansion inside third-party crates or external procedural macros; such
source-generation grammar must be made inspectable and added to the guard before use.

The checker also parses the active `dispatch_packet` method and compares its top-level
`ClientOpcodes` patterns with the snapshot; conditional or guarded arms are rejected and the sole
wildcard must be an independent final arm. It permits exactly the three pre-existing
one-sided entries owned by #142 (`TrainerBuySpell`, `MoveSetVehicleRecIdAck`, and
`PartyUninvite`) and fails both on new drift and on an obsolete exception after a repair. This is
a registration-to-arm coverage check; it intentionally does not infer correctness from names
mentioned in an arm body.
The existing C++ metadata regression remains the semantic authority. Together, these guards
prevent an unreviewed registration addition, removal, rename, metadata change, conditionally
compiled registration, or dispatch-arm loss.

## Deliberate baseline updates

Do not regenerate a baseline merely to make CI green.

1. Identify the semantic owner and contrast any behavior-affecting change with C++.
2. Explain why the dependency or handler-contract change is intentional in the issue and PR.
3. Add or update focused positive/negative tests before changing the baseline.
4. For a legitimate external library in an inward package, add only that exact package/kind to
   its allowlist and explain the architectural role. For workspace or infrastructure-debt
   exceptions, provide a concrete tracking issue and reason. Remove an exception or stale
   allowlist entry in the same commit that removes its final edge.
5. For a handler snapshot or dispatch-side exception, inspect the exact added/removed/changed row
   or arm, retain the C++ metadata contrast test, and assign any temporary one-sided wiring to a
   concrete removal issue.
6. Run the architecture self-test, architecture check, focused Rust tests, and full PR preflight.

## Refactor sequence

The child issues of #133 execute in semantic order, regardless of their GitHub creation number:

1. #135 — executable boundary guardrails (this baseline);
2. #163 — compose effective spell-acquisition metadata;
3. #164 — freeze a complete trainer acquisition plan;
4. #157, #158, #159, #160 and #161 — apply the trainer plan in bounded behavioral slices;
5. #142 — reconcile the three pre-existing dispatcher/registration mismatches;
6. #134 — remove gameplay `SessionResources` from the listener;
7. #136 — private world-server session factory;
8. #138 — session mailbox/player registry ownership;
9. #137 — group registry ownership;
10. #139 — extract Calendar handlers from `misc.rs`;
11. #140 — extract the `WorldSession` update/dispatch driver.

Each issue is one branch and one PR. The next issue starts only after the current PR is
capture-clean where applicable, all actionable review is resolved, required CI is green on the
current HEAD, and the PR is merged.
