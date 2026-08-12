# RustyCore ownership and dependency boundaries

This document is the executable architecture baseline for the incremental refactor tracked by
issue #133. It records the current owners and intentional mirrors before code is moved. The
machine-readable dependency rules live in
`tools/architecture/dependency-policy.json`; `tools/architecture/check_architecture.py check`
enforces them. The checked-in issue ledger
`tools/architecture/architecture-issue-ledger.json` records every architecture epic and
implementation slice, its parents, its prerequisites, and one deterministic topological display
of the audited DAG. External prerequisites such as QA issue #177 are explicit nodes rather than
implicit prose. The checker keeps this document, the ledger, and the JSON policy in agreement
without contacting GitHub; independent nodes may still execute in parallel.

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
for review, not a universal size rule. The checker reports the largest production/test hotspots.
For the explicitly audited paths in `runtime-ownership-ledger.json`, the recorded production,
test, and total counts are independent non-growth ceilings: extraction and reduction pass without
a baseline edit, while growth in any one metric requires an explicit reviewed baseline change.
An audited path cannot disappear or be renamed until its ledger row is explicitly retired or
replaced.

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
The checker enforces that rule through the issue ledger: an exception whose `tracking_issue` is
absent from the ledger, or owned by a completed issue, fails the architecture check. Generic
exceptions keep `tracking_issue` 133 only while the parent genuinely remains open, and the
ledger's `reaudit_issue` (#153) explicitly owns their re-audit and final classification.

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
| Authenticated world connection | `wow-network::accept` and the connection task | socket/authentication task | `WorldSession` dispatch boundary | one connection; created after authentication and dropped on disconnect | Remains a network responsibility. #134 narrowed the listener to transport-owned configuration and authenticated connection outputs. |
| Gameplay session construction aggregate | private `SessionResources` in `crates/world-server/src/session_resources.rs` | `world-server` bootstrap constructs it; the outer session callback captures and clones the `Arc` | `world-server::create_session` copies stores, registries, DB adapters, and runtime handles into each `WorldSession`; the aggregate never enters `wow-network` | process lifetime; no independent clock | #134 moved the aggregate to composition code and retired the direct `wow-network → wow-database` / `wow-network → wow-instances` edges. #136 next extracts a private session factory. |
| Session directory | `wow_network::player_registry::PlayerRegistry`; backing storage is a `DashMap<ObjectGuid, PlayerBroadcastInfo>` | session login/logout and state publication write the directory | global routing, handlers, and gameplay fanout currently read broad snapshots | process lifetime; presence and generation/addressability are Session concerns, while gameplay fields are temporary mirrors | #150 introduces the opaque facade; #192-#195 burn down production consumers; #196 closes storage and fixtures; #138 then relocates the already-opaque directory. |
| Session mailbox and durable rails | `wow_network::player_registry::SessionCommand` plus ordinary and durable payloads in the same network module | session/global runtime and gameplay producers enqueue commands; one session task consumes them | the owning session consumes FIFO commands and publishes acknowledgements | connection/session lifetime; queue identity, capacity, FIFO, incarnation fences, acknowledgements, and shutdown drain are observable | #189 removes durable loot persistence coordination; after #138, #191 relocates the ordinary mailbox protocol and #190 relocates the durable creature-runtime rail. #140 then extracts the pump without changing queue semantics. |
| Group registry and pending invites | `wow_network::group_registry::{GroupRegistry, PendingInvites}`, currently backed by concurrent maps | group handlers and group timer/ready-check paths | group handlers, connected-player fanout, world-server ready-check loop | process lifetime; timed group work is driven outside the network listener | #151 creates opaque facades; #197 and #198 centralize atomic transitions; #199 separates persistence/publication and closes storage; #195 adapts session addressing; #137 finally moves the owner into `wow-social`. |
| Legacy creature runtime | shared `wow_world::MapManager` behind `Arc<RwLock<_>>` | production `GlobalLegacy` runtime tick plus explicit spawn/respawn bridges; `Session` writer exists only for tests and the diagnostic config override | world handlers, global runtime bridge, visibility/fanout routing | process lifetime; production startup defaults `RuntimeTickOwner` to `GlobalLegacy` and uses the configured map-update interval; session ticks read the shared owner and must skip to prevent double resolution | #188 freezes clocks, phases, and bridge behavior. #28 may perform one bounded authority cut; later cuts retire legacy behavior method-by-method under `docs/migration/adr-runtime-tick-ownership.md`. |
| Canonical map runtime | `wow_map::MapManager` | canonical global map loop, grid/spawn/respawn paths, explicit selected legacy-result adapters | world-server orchestration and session map/player bridges | process lifetime; canonical loop uses the configured map interval; preserves the C++ `Map::Update` phase order represented by the ADR | #188 records the current phase trace. It becomes the sole map/entity authority only after each later method cut has parity tests and removes the corresponding legacy writer. |
| Creature legacy/canonical mirror | canonical loaded-grid records are mirrored into `wow_world::MapManager`; selected lifecycle, movement, aggro, attack-stop, melee, health, and respawn outcomes are explicitly bridged back to canonical state | named bridge functions in `world-server`, including `mirror_loaded_grid_creature_to_legacy_like_cpp` and the `run_legacy_creature_*_and_deliver_once_like_cpp` family | both runtimes and post-lock packet/command delivery | load/respawn synchronization begins canonical → legacy; only explicitly modelled runtime outcomes travel legacy → canonical; delivery occurs after map locks are released | #181 inventories every bridge and #188 freezes its phase trace. Remove one bridge only when its destination runtime becomes authoritative for that whole transition; never add generic bidirectional sync. |
| Represented player gameplay state | mostly fields on `wow_world::WorldSession`; canonical value types and partial state also exist in `wow_entities::Player` | session handlers and session update code | packet builders, persistence helpers, `PlayerRegistry` summaries, canonical snapshot bridges | connection/selected-character lifetime; `canonical_player_entity_snapshot_*_like_cpp` currently rebuilds a `Player` snapshot from represented session fields | #181 records field families, writers, mirrors, and cutover owners. After the Session shell lands, #153 materializes one-responsibility cuts until `Player` is the mutable gameplay owner and `WorldSession` is only the connection/session bridge. |
| Concrete persistence access | SQLx/MySQL pools, rows, transactions, prepared statements, and raw SQL currently leak through `wow-world`, data loaders, instances, handlers, and Session resources | application/gameplay code and concrete `wow-database` adapters both assemble durable work | handlers, lifecycle, runtime recovery, and publication paths consume concrete outcomes | statement order, connection affinity, commit/rollback/unknown-commit classification, fences, and publication order are observable | #186 inventories and ratchets leaks; #187 freezes ordered behavior; #200 earns the SQLx-free `wow-persistence` boundary with Player lifecycle; #189 moves durable loot coordination. Epic #169 and post-shell gate #153 own the remaining vertical cuts. |
| Effective skill metadata | `wow_data::SkillLineStore` owns final `SkillLine` identity/acquisition fields; `wow_data::SkillStore` owns final `SkillLineAbility` and `SkillRaceClassInfo` rows plus their derived indexes | `world-server` bootstrap composes WDC4 → official SQL → custom SQL → final removals once; no runtime writer | spell loaders and gameplay validation read immutable stores shared with sessions | process lifetime; `SkillLine` is composed first, then dependent rows are filtered and every index is rebuilt from final records in ascending ID order | Retire the specialized acquisition projections only when the general effective DB2 authority carries the same checked payload and coverage states. Never reactivate the raw WDC-only `SkillStore::load` path in production. |
| Effective spell-acquisition metadata | `wow_data::SpellAcquisitionCatalogLikeCpp`, a compact immutable projection of the seven acquisition source families | `world-server` bootstrap composes and publishes one `Arc`; no handler or session mutates it | derived spell-learning loaders now; trainer planning in #164; sessions receive the same `Arc`, not the seven raw stores | process lifetime; exact regular SpellInfo keys seed covered/zero distinction, while server-side keys without validated acquisition payload are explicitly indeterminate | Remove the specialized catalog, or feed it from the general store, once full effective `SpellInfo` payload authority exists. This row does not authorize packet, persistence, spell, skill, money, or battle-pet mutation. |
| Immutable spell-acquisition projection and application | `wow_world::spell_acquisition` owns the pure fixed-point plan plus its validation/transaction/publication boundary; the live player and Character DB remain the runtime/durable owners | planning mutates only a private ordered copy; #158 locks one character row and commits the complete durable result; #159 extends that same transaction with guarded money and keeps the exclusion through runtime/packet publication | #157 consumes ordered primary-profession outcomes; #158 consumes the exact source/result plan and generic player `EffectLearnSpell`; #159 consumes a startup-audited cast/craft authority plus a fresh player effect mask and owns trainer charge/wrapper/visual orchestration | one acquisition operation; complete spell/skill/trait/override authority, exact slot occupancy and wrapper static/live proofs are mandatory. Unknown COMMIT outcomes reconcile money plus all spell/favorite/skill rows before publication; see [the detailed contract](spell-acquisition-plan.md) | Retire the specialized seam only when canonical `Player` methods expose the same atomic dry-run/apply contract. Never reconstruct capacity or criteria from a flat trigger list, sort profession outcomes, infer “no immunity” from missing runtime state, or publish before the durable boundary. |
| Battle-pet trainer purchase saga | `wow_world::battle_pet_purchase` owns the durable command (`character_battle_pet_purchase`), its state transitions and login recovery; Character DB money stays under the #159 exclusive per-character guard; the pet itself stays under the #160 account owner | the saga is the sole writer of the command table and the sole caller that turns a trainer offer into a #160 add; it spawns no tasks and holds no lock across `.await` other than the pre-existing async money mutex | buy handler adapts request → offer decision → saga; the #160 owner keeps fence/journal-lease/capacity authority; the #163 catalog keeps species-classification authority; the world-DB selection store keeps breed/quality/display authority | one purchase command per 128-bit request key (shared with the #160 receipt); charge+command, publication marker, completion, and refund+flip are each single Character DB transactions; `PetApplied` is derived from the Login DB receipt and a Completed row with a clear `published` marker is owed its publication by recovery | Retire only when a portable cross-pool transaction exists. Never publish before the pet is durable, never refund a durable pet, never recover from in-memory flags, never activate the `TrainerBuySpell` dispatcher arm outside #142. |
| Handler registration and dispatch-arm contract | the sole `inventory::collect!(PacketHandlerEntry)` in `wow-handler`, link-time `inventory::iter<PacketHandlerEntry>` consumed by `wow_handler`/`WorldSession`, and the concrete `WorldSession::dispatch_packet` opcode arms | unconditional module-item `inventory::submit!` declarations owned by the logical `wow-world::crate::handlers` tree, plus one dispatcher owned by `wow-world::crate::session`; both owners and their private descendants are declared in `handler-module-policy.json` | dispatch table and session update driver | compile/link lifetime; no mutable clock | #142 removed the final one-sided entries. #185 makes module ownership independent of physical filenames and fails closed on conditional, missing, duplicate, remounted, malformed, or stale ownership. #139 proves one thin capability, and #152 moves admission/dispatch without altering the exact opcode/metadata/arm contract. The terminal router inversion is re-audited by #153. |

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
cargo +1.88.0 run --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml --bin session-ownership-check -- check
./tools/pr-preflight.sh architecture
```

Ledger schema v2 distinguishes epics from one-PR slices, validates parents and internal/external
prerequisites, rejects unknown dependencies, self-dependencies and cycles, and proves that the
documented sequence is a complete topological ordering of the slices. A closed slice cannot depend
on an open prerequisite. The checked-in state remains an offline reviewed snapshot; the guard does
not contact GitHub or silently rewrite titles, states, or higher baselines.

`handler-module-policy.json` is the offline authority for packet-dispatch and
handler-registration module ownership. Each capability declares one Cargo package, one logical
Rust module root, whether private descendants belong to that root, and an open issue that owns its
next move or terminal re-audit. The architecture checker rejects malformed policy, unknown or
completed issue ownership, duplicate capabilities, and overlapping owner trees. The Rust handler
checker consumes the same file, walks Cargo-declared production module graphs, and locates the sole
concrete `WorldSession::dispatch_packet` by logical mount rather than by filename. External modules,
inline modules, and supported `#[path]` declarations therefore preserve the contract across a
physical split; unresolved paths, production-capable `cfg` ambiguity, remounts, missing or duplicate
dispatchers, registrations outside their declared owner, and unprovable source-generation shapes
fail closed.

`self-test` pins the locked/all-features Cargo metadata command and proves a permitted downward
workspace edge (`wow-combat → wow-math`), a rejected
upward edge (`wow-map → wow-network`), a reviewed domain utility (`wow-map → rand`), concrete SQL
inside its adapter (`wow-database → sqlx`), and rejection of direct SQL, network, configuration,
process, and async-runtime additions to `wow-map`. It also exercises stale workspace exceptions,
external exceptions, external allowlist entries, and the raw-network rejection of direct SQL
(`wow-network → sqlx`) while retaining its reviewed Tokio runtime. Malformed/duplicate JSON and
Cargo identities, canonical/path/Git/alternate-registry origins, both valid Cargo Git-ID forms,
inactive target-specific dependencies, and ambiguous external identities are adversarially
covered. The hotspot classifier self-test proves exact top-level `#[cfg(test)]` item ranges across
multiple modules/items, ignores braces and false attributes in Rust literals/comments, and keeps
production after a test module outside the test range. The ratchet self-test independently rejects
production, test, and total growth and accepts a reduction. `check` evaluates the real locked
Cargo workspace, including direct
third-party `normal`/`build` dependencies, rejects stale policy entries, enforces non-growth for
the curated hotspot paths, and still prints the broader source-hotspot report.

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

The sibling `session-ownership-check` binary reuses that logical module walk but evaluates `cfg`
satisfiability to classify production, test-fixture, dead, and generated-input surfaces. It stores
exact normalized identities rather than line numbers, follows the three transitional registry
aliases through ordinary wrappers/imports/locals/returns, and fails closed when a macro, alias,
glob, `include!`, or malformed `cfg` would make ownership unknowable. Splitting or combining
physical `impl` blocks is therefore neutral; moving or changing an associated item, field, payload,
factory call, or direct registry operation is a reviewed baseline delta.

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
wildcard must be an independent final arm. Registrations and arms must now be exactly equal:
there are no one-sided exceptions, and any future mismatch fails. This is a registration-to-arm
coverage check; it intentionally does not infer correctness from names mentioned in an arm body.
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

## Audited ownership and hotspot evidence

The current baseline was audited on branch `3.4.3` at HEAD `002d3d87`. Production and test lines
are reported separately because moving an inline test module must not masquerade as ownership
progress:

| Hotspot | Production | Tests | Total |
|---|---:|---:|---:|
| `crates/wow-world/src/session.rs` | 71,997 | 94,772 | 166,769 |
| `crates/wow-map/src/map.rs` | 15,247 | 18,413 | 33,660 |
| `crates/wow-world/src/handlers/character.rs` | 20,235 | 10,618 | 30,853 |
| `crates/wow-world/src/handlers/loot.rs` | 13,660 | 16,193 | 29,853 |
| `crates/world-server/src/main.rs` | 15,380 | 12,795 | 28,175 |
| `crates/wow-world/src/handlers/misc.rs` | 7,315 | 11,473 | 18,788 |
| `crates/wow-world/src/handlers/quest.rs` | 8,231 | 10,241 | 18,472 |
| `crates/wow-entities/src/player.rs` | 9,265 | 8,907 | 18,172 |

At the same HEAD, the syntax-aware ratchet records 738 `WorldSession` fields: 727 production and
11 `cfg(test)` fixtures. It also records all 20 logical inherent-impl owners and 3,339 exact
associated-item signatures rather than freezing the number of physical `impl` blocks. Private
composition-side `SessionResources` has 243 fields, of which 186 are optional;
`PlayerBroadcastInfo` has 80 fields; and `SessionCommand` has 37 variants plus 42 transitively
reachable payload types. The factory has 247 `set_*` and one `install_*` call: two setters are
multiline calls that the earlier text-only count missed. The generated-input surface has 44 exact
records, and direct access to `PlayerRegistry`, `GroupRegistry`, or `PendingInvites` is frozen as
685 exact AST rows with multiplicity 705. The workspace-wide persistence inventory adds 309 exact
rows—278 production and 31 test-fixture rows—with multiplicity 366 (326 production and 40 test)
for concrete types, queries, transactions, pool escapes and macro boundaries. The
legacy/canonical inventory contains 71 definition/seam rows, including eight
curated anchors; it deliberately avoids duplicating every caller of an already inventoried typed
helper. `#134` already moved `SessionResources` out of `wow-network`; #136 extracts the factory
without turning the aggregate into another public dependency bag.

The reproducible syntax snapshot is
`tools/architecture/session-ownership-policy.json`; the curated owner/writer/mirror and retirement
mapping is `tools/architecture/runtime-ownership-ledger.json`. The Python guard cross-checks their
exact field/variant memberships, while `session-ownership-check` rejects added, removed, retyped,
re-visibility-scoped, re-owned, generated, factory-wiring, command-payload, broadcast, and direct
registry, persistence, and bridge surfaces. `print-baseline` only writes reviewed JSON to stdout;
it never updates either baseline automatically.

These numbers are diagnostics, not completion criteria. #181 owns reproducible counters and
non-growth rules; each later slice must reduce or retire a named ownership smell rather than
merely moving lines.

## Refactor sequence

The ledger is the offline source of truth for the dependency DAG. The list below is only one
deterministic topological display used to detect documentation drift; it does **not** serialize
independent work. Epics #133 and #169 are parents rather than implementation PRs, and external QA
prerequisite #177 is represented separately in the ledger.

1. #135 — executable boundary guardrails;
2. #143 — interaction provenance;
3. #146 — effective SpellInfo authority;
4. #148 — effective SkillLine authority;
5. #144 — validated trainer load inputs;
6. #156 — primary-profession capacity;
7. #163 — effective spell-acquisition metadata;
8. #164 — deterministic acquisition outcomes;
9. #157 — trainer offer and eligibility decisions;
10. #158 — durable prepared spell learning;
11. #159 — atomic normal trainer teaching;
12. #160 — durable account-atomic battle-pet ownership;
13. #161 — recoverable battle-pet trainer purchase;
14. #142 — dispatcher/registration reconciliation;
15. #154 — audited policy alignment;
16. #134 — listener/SessionResources decoupling;
17. #181 — ownership baseline and submodule ratchets;
18. #185 — module-aware architecture and handler guards;
19. #186 — persistence-leak inventory and ratchet;
20. #188 — world/map clock and bridge trace;
21. #136 — private world-server session factory;
22. #187 — persistence transaction/publication golden;
23. #150 — opaque PlayerRegistry facade in place;
24. #151 — opaque GroupRegistry/PendingInvites facades in place;
25. #139 — Calendar thin-capability pathfinder;
26. #152 — packet admission and dispatch submodules;
27. #200 — Player lifecycle persistence port;
28. #189 — durable loot persistence coordination;
29. #192 — runtime/fanout directory consumer burn-down;
30. #193 — combat/loot directory consumer burn-down;
31. #194 — quest/spell/movement directory consumer burn-down;
32. #197 — atomic invite/create/capacity transitions;
33. #198 — atomic membership/leadership/ready transitions;
34. #199 — Group persistence/publication and storage closure;
35. #195 — social/group session-addressing adapters;
36. #196 — PlayerRegistry storage and fixture closure;
37. #138 — opaque session-directory relocation;
38. #191 — ordinary mailbox-protocol relocation;
39. #137 — encapsulated Group owner move;
40. #190 — durable creature-runtime rail relocation;
41. #140 — ordinary/durable Session command pump;
42. #182 — logical realm/instance routing;
43. #183 — Session-only phase driver;
44. #184 — login/logout lifecycle over persistence ports;
45. #153 — post-shell re-audit and remaining semantic cuts.

The open dependency graph is:

```text
#134 ─► #181
#134 + #181 ─► #136
#181 ─► #185, #186, #188
#181 + #185 ─► #150, #151, #152
#185 ─► #139
#186 ─► #187 ─► #189
#186 + #187 ─► #200

#150 + #188 ─► #192
#150 + #187 ─► #193
#150 + #188 ─► #194
#151 ─► #197 ─► #198
#187 + #198 ─► #199
#150 + #199 ─► #195
#192 + #193 + #194 + #195 ─► #196
#136 + #196 ─► #138 ─► #191
#195 + #199 + #191 ─► #137
#188 + #191 ─► #190

#152 + #191 + #190 ─► #140
#140 + external #177 ─► #182
#182 + #188 ─► #183
#183 + #187 + #200 + external #177 ─► #184
#184 ─► #153
```

Some direct dependencies repeat a transitive evidence gate deliberately; for example #184 names
#187 even though #200 also depends on it. A slice may start when every declared prerequisite is
merged and its own branch is current. Unrelated branches may proceed in parallel, but each issue
still maps to one reviewable PR with focused validation and no behavior-changing extraction hidden
inside a mechanical move.
