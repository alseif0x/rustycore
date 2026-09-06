# ADR — Private entity world behind a single-writer MapRuntime

**Created:** 2026-09-04 · **Reviewed:** 2026-09-05, code `93e4002a` · **Issue:** #578 / parent #133

**Status:** Canonical ownership and selective private `hecs` are selected. Cohesive domain
aggregates are retained. This architectural decision is not production integration or acceptance;
the finite pre-migration conformance proof below now passes its freeze/third-module correctness
and all 320 preregistered aarch64 cost samples. See the
[V2 evidence and remaining production boundaries](../architecture/modularity-conformance-results.md).
Continue to the first real C1/C2 vertical with C0 admission/phase evidence before replication;
the laboratory's provisional cost gates do not establish a production 10 ms frame budget.

**2026-09-05 controlled-lab result:** [the corrected comparison](../architecture/modularity-lab-results.md)
passes its functional/resource gates and supports the feasibility of the selected `hecs` design;
its lower update cost is offset by higher structural churn/transfer cost. The lab knows only two
optional state types and does not prove the independent composition/maintenance or production
lifetime gate below. Production storage remains unchanged. Core Wasm feasibility was evaluated
independently with storage fixed; it does not select or require an ECS backend.

## Decision summary

The world runtime remains an in-process modular monolith; the existing `bnet-server`
authentication boundary stays separate. In-world gameplay state will converge onto
one `MapRuntime` per map instance. The runtime will be the single writer of a private entity world;
sessions, handlers, persistence adapters, and external modules will never receive an ECS entity ID,
a storage/query guard, or mutable access to that world.

The approved direction is the
[modularity and ECS plan](../architecture/modularity-and-ecs-plan.md). External modularity is a
product capability, not a consequence of choosing an entity container. #578 owns runtime,
lifetime, selected-backend conformance and convergence; #99 owns the external extension direction
and SDK contract, with #583 implementing stateful/progression, independent composition and durable
operation/receipt acceptance through real external modules. #583 also delivers the operator-optional
Wasm executor and native/Wasm SDK lifecycle. The isolated shared native/Rust-Wasm/C-Wasm conformance
checkpoint belongs to #578; neither #583 nor its completed production SDK is a dependency of it.
#578's C0–C4 macro remains intact; #153 performs terminal verification after the required #578 and
#583 deliveries.

Selective `hecs` (initial pinned baseline 0.11.1) is chosen for map-local, independently composable
entity/behavior state, not because a synthetic iteration win proves the whole server design.
Keep cohesive Player/Unit/domain aggregates where they enforce complete invariants. Catalogs,
accounts, matchmaking/LFG and durable I/O retain their own owners outside this storage choice.
Retain current production storage until the selected design passes pre-migration conformance and
the affected integration proof. This explicitly replaces the earlier proposal to run another
three-candidate competition before choosing: selection is made now; implementation must still earn
acceptance. An improved aggregate/state registry remains a fallback for a demonstrated obstacle,
not an obligatory parallel implementation or an intermediate dense-arena migration.
This ADR does not require a `wow-ecs` crate, a global ECS, a component per field, or a second copy of
`Player`/`Creature` state. The backend remains private to `wow-map`; a future crate requires a real
dependency boundary, not completion of a diagram.

## Context

The #578 branch has established a generation-checked `PlayerHandle` and transferred represented
gameplay families to the canonical `wow_entities::Player`. Active players live in canonical map
storage; far teleports preserve the same `Player` value in an explicit detached store. This improves
authority for the migrated families, but the transitional access path still serializes operations through
closure-based access to a shared `MapManager`, and canonical map storage is still a heterogeneous
`HashMap<ObjectGuid, MapObjectRecord>` plus derived indexes.

That is a valid convergence bridge, not a completed runtime/API contract:

- it centralizes mutable access but does not make the simulation owner explicit;
- narrow Creature batch projections exist, but complete movement, vitals, aura, threat and
  visibility operations are not thereby integrated under their final owner;
- a large `MapObjectRecord` enum remains the common mutation surface;
- synchronous gameplay can run under the transitional exclusive owner borrow; packet delivery,
  database I/O and `.await` cannot retain that borrow or a manager lock. Nominal storage ownership
  alone does not establish this execution boundary.

These are ownership and API obligations, not proof that `HashMap` cannot support a modular server.

The C++ source remains the behavioral and temporal authority. Its pointer/container layout is not
copied literally, but these facts are binding:

- `WorldSession` has one `_player` pointer and `GetPlayer()` returns it
  (`WorldSession.h:980,1882`); it does not own a flattened second Player model.
- `Map::Update` owns the ordered in-world frame (`Map.cpp:666-813`): dynamic tree, session update,
  respawns, object updates, transports, object-update delivery, scripts, weather, personal phases,
  move lists, relocation notifications, map hook, and metrics.
- `MapManager::Update` waits for all scheduled `Map::Update` work before invoking each
  `Map::DelayedUpdate` (`MapManager.cpp:290-320`). `Map::DelayedUpdate` drains far callbacks, the
  remove list, and grid state (`Map.cpp:2519-2545`).
- During a far teleport the Player exists while it has no current Map. Detached persistent-state
  access remains valid; spatial operations can return `NotActive`. Distinguish detached, stale and
  missing ownership, and never synthesize zero/default gameplay values.

## Target boundary

~~~text
WorldSession                       MapRuntime(map_id, instance_id)
  identity / sockets                 owns execution of admitted phases
  admission / dispatch       cmd --> private EntityWorld
  routing / mailbox                  GUID -> private storage locator
  lifecycle / PlayerHandle  <-- outcome/query snapshot

outside the owner:                 inside the owner:
decode/encode                      ordered C++-like phases
database I/O                       canonical state mutation
packet delivery                    spatial/secondary-index maintenance
await/backpressure                 owned outcomes only
~~~

A single execution owner does not mandate one Tokio task or independent wall clock per map.
Scheduling and barriers must implement the C++ contract; storage selection does not choose the
thread/task topology. Realm-wide responsibilities retain their own explicit owners/coordinators.

### Public identity

- `ObjectGuid` is the stable game/protocol identity.
- `PlayerHandle { guid, generation }` identifies one selected-character incarnation and detects
  replacement/stale sessions.
- Backend IDs such as `hecs::Entity` are private map-local locators. They are never persisted,
  serialized, logged as game identity, stored in `WorldSession`, or exposed through module APIs.
- Secondary indexes contain GUIDs or private entity locators and are derived from the entity world;
  they are not another mutable owner.

### Residence and teleport

`MapManager` owns the lifetime registry. A Player residence is exactly one of:

1. `Detached` — the one Player value is alive in the detached store and is in no entity world;
2. `Active(MapKey)` — the one Player value is in that map runtime's entity world.

Attach/detach moves the value. It never clones a canonical Player, and a failed attach leaves the
value detached. Replacing a character incarnation increments the generation and invalidates every
old handle.

The current implementation transfers one `Box<Player>`. A future component representation may
transfer an owned bundle instead, provided every canonical family and its lifecycle state moves
coherently. Logical Player identity is the invariant, not a permanent Rust struct layout. A
map-local backend entity may be replaced during transfer without replacing the Player incarnation;
`PlayerHandle` generation authority must not silently become backend-generation authority.

### Selective components and aggregates

The migration starts with complete, cohesive families and preserves C++ invariants and existing
represented `Player`/`Creature` behavior. Candidate families include:

- identity/type and world presence;
- transform/map/phase;
- Unit core, vitals, and powers;
- movement;
- combat, auras, and threat;
- Player-only state;
- Creature-only state.

These are candidate boundaries, not a mandatory field-migration list. Components are split when a
real operation or independent composition needs them. Storing `Box<Player>` or `Box<Creature>` as
typed ECS components is a legitimate coarse backend option, but does not make their nested hot
fields contiguous or prove field-level parallelism. A single `MapObjectRecord` component mostly
retains the existing enum surface; measure its actual benefit rather than calling it convergence.

If transform or vitals becomes a component, remove its former mutable representation and preserve
all associated invariants. For example, `Unit::set_health` / `set_max_health`
(`crates/wow-entities/src/unit.rs:2134-2159`) combine death gates, bounds, dirty fields and revision
tracking. A raw component assignment must not bypass them. Derived GUID/spawn/spatial indexes are
allowed; independently writable entity mirrors are not.

### Commands, queries, and outcomes

Handlers adapt packets into typed commands or narrow queries. `MapRuntime` applies them
synchronously inside the owner and returns owned values:

~~~text
decode -> MapCommand -> MapRuntime/EntityWorld -> Vec<MapOutcome>
       -> persistence/delivery adapter -> encode/send
~~~

No outcome borrows a component. Database work, channel backpressure, packet construction requiring
external catalogs, and every `.await` happen after all ECS borrows and map guards have been dropped.
Each cross-entity operation has a coherent transition inside one owner; cross-map/realm work uses
explicit coordinators and idempotent outcomes rather than shared mutable access. Preserve explicit
asynchronous operation fences where persistence requires them; those are not map/entity guards.

Complex extension behavior is not forced into a blind snapshot-to-batch model. #99 may expose
scoped synchronous domain capabilities or equivalent same-phase continuations that return an
action result before the next query/action. No external API receives storage borrows. Per-script
state, reset, persistence, callback ordering and reentry are explicit contracts. An entire callback
is not automatically a rollback transaction: Anomalus changes phase and requests its shield before
a nullable summon (`boss_anomalus.cpp:159-170`); summon failure does not undo the preceding actions.

### Scheduling

ECS storage does not choose gameplay order. The map driver executes the C++ phases, not just their
labels. `Map.cpp:703-755` interleaves each Player update with nearby-cell visits and additional
combat/aura/summon visits. It is not a universal pass over all movement, then all auras, then all
combat. `Unit.cpp:418-434` requires events before spells and then combat-reference updates.

Structural mutation also has multiple barriers: `Map.cpp:2547-2554` marks destruction and performs
initial cleanup immediately, while far callbacks precede deferred switches/removal
(`2519-2530,2574-2646`). An ECS command buffer flushed only at frame end is not an equivalent rule.
Summoning invokes lifecycle callbacks before returning (`Object.cpp:1956-1972`,
`TemporarySummon.cpp:249-264`); evade invokes reset (`CreatureAI.cpp:219-242`). Prove same-phase
ordering and safe reentry, rather than moving those callbacks to the next tick to avoid borrowing.

A scheduler or parallel iteration is considered only within proven-independent work. Preserve
observable packet/publication/queue order and the MapManager barrier before delayed update;
independent clocks or a constant phase-name vector are not that proof.

## Alternatives considered

### Keep `HashMap<ObjectGuid, MapObjectRecord>` as the terminal store

Viable baseline and fallback, not categorically rejected. A private facade, typed operations and
derived indexes can support modular gameplay without an ECS. Retaining the exact enum preserves some dispatch and
layout costs, but does not force broad public mutable access. An improved aggregate plus generic
state registry can implement modular behavior; that open registry is not delivered by today's
store either. Keep aggregates where composition does not justify replacement. The external SDK
must remain independent of the representation.

### Expose a global ECS to sessions and handlers

Rejected. It would turn the ECS into a query-anything service locator, obscure ownership, permit
cross-map coupling, and make async/lock boundaries difficult to enforce.

### `bevy_ecs`

Not selected: no demonstrated requirement for another resources/events/scheduler framework.
Its scheduling, resources, events and change detection offer a broader surface than `hecs`. This is
a scope choice, not a claim that Bevy requires replacing the C++ driver: its scheduling policy would also have to be
chosen explicitly. Revisit only for a concrete unmet requirement, not to prolong the decision.

### `hecs`

Selected private backend for independently composable map-local state. Its typed component
composition, generational locators and borrowing support can serve independently composed behavior
without adding a scheduler alongside the C++-ordered driver. V1 supports feasibility; the expected
reduction in application-specific composition plumbing remains architectural judgment to verify,
not measured maintenance superiority. It need not absorb every entity family, cold aggregate or
catalog, and it does not supply hooks, module authorization, persistence, an SDK or isolation.

### Typed aggregate / library-backed dense or generational store

Credible fallback alternative, not benchmarked against `hecs` by V1. Separate typed collections,
library-backed arenas or dense stores can improve lookup, locality and ownership APIs without
becoming a custom general-purpose ECS.
Using such libraries does not require RustyCore to implement unsafe allocation, archetype movement
or a generic query engine. Prefer existing safe borrowing primitives and maintained libraries;
account for application-specific indexes and composition work on both sides. Do not reject this
alternative on a maintenance burden it does not actually incur. A dense-store migration is not an
automatic prerequisite: use it only if a diagnosed layout/indexing need justifies that fallback.

Veloren is useful evidence for an ECS-backed Rust server, especially its component-oriented state
and retained spatial indexes. It is not a template: RustyCore preserves Trinity's update order and
uses a private command/outcome boundary instead of allowing broad storage fetches from network
handlers.

## Finite pre-migration conformance and production integration gates

The next authorized implementation checkpoint is a bounded conformance proof of the selected
design inside #578, following [the approved plan, section 5](../architecture/modularity-and-ecs-plan.md#5-ecs-decision-now-selective-private-hecs-cohesive-aggregates-retained).
It is not another three-backend competition or completion of #583's production SDK. V1's two known
optional types and enumerated combinations do not prove independent extension. Preserve that
falsification test before migrating authoritative production state:

1. **Freeze the experimental contract:** anchor the private host's operations and callback phases
   to represented C++ paths, distinguish custom behavior, implement two independent modules, and
   record hashes of the frozen host, adapters and contract sources.
2. **Third independently authored module:** add a separate crate with a new state type and lifecycle
   rule. Only dependency declarations, declarative registration and composition may change. No new
   host enum/match arm naming the type, module-specific storage adapter, broad entity borrow or
   exposed hecs/SQL/packet API is allowed. Record central code touched and lifecycle plumbing.
3. **Behavior and lifetime:** exercise zero/mixed modules, coexistence, conflicts, isolated state,
   action -> synchronous callback -> read, nullable action failure and failure after prior effects.
   Include reset/removal, active/detached transfer, failed attach, replacement/stale incarnation
   and versioned snapshot/replay. Reject an outer stale write after a nested state mutation;
   schema versions must not substitute for mutation revisions. Snapshot/replay is a mock here,
   not database durability proof.
4. **Shared execution contract:** run equivalent cases as native Rust, Rust -> Core Wasm and
   C -> Core Wasm, including native/Wasm composition in one host. Reject duplicate executors and
   incompatible versions/capabilities; enforce the approved resource, reentry and failure limits.
   Unsupported executor changes must reject before activation with durable state/history retained,
   not implicitly reset it; supported changes need compatible formats or validated conversion.
   This isolated proof does not depend on delivery of the production SDK or make Wasm mandatory
   for the operator. A Wasm/ABI defect is not evidence against the entity backend.
5. **Predeclared measurements:** fix workloads, populations, repetitions, source hashes and
   CPU/RSS/action/state budgets in a new versioned protocol before measuring. Separate update,
   churn, transfer, dispatch and cold costs; preserve all failed samples. Name date, toolchain,
   architecture and payload limits. V1's provisional resource budgets are not a server SLA.
6. **Conformance verdict:** record pass/fail and remaining production boundaries. This proof now
   **passes** at `c67acbfd` on aarch64; [V2 evidence](../architecture/modularity-conformance-results.md)
   precedes the affected production migration and the #583 SDK. It is not production acceptance,
   a new issue, broad framework or approval per component.

Fix implementation errors and rerun their affected cases. Reopen the backend decision only for a
demonstrated hecs-specific obstacle: unavoidable duplicate authority, failure to support independent
state/lifetime, or unacceptable structural cost after bounded correction. Then compare against the
aggregate plus generic-registry fallback; a dense library is relevant only if layout is the
diagnosed issue. No perpetual candidate competition or permanent second mutable authority.

Passing isolated conformance does not waive these production requirements under C0–C4:

Apply the [explicit reanalysis cadence](../architecture/modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication):
review the first real C1/C2 operation, including its C0 execution contract, before replicating
the pattern. C4 balances the complete #578 acceptance before #583 production integration;
#153 independently audits both merged macros. The later #47/M6.2 review covers the whole port,
not just this storage decision. These checkpoints do not introduce new routine approvals.

1. **Stateful family:** exercise a real domain-shaped behavior with mutable per-creature and
   per-instance state, conditional timers, summon success/failure, callbacks, reset and saved
   encounter progress. Anomalus/Nexus is the C++ reference case
   (`boss_anomalus.cpp:81-181,212-244`, `instance_nexus.cpp:90-147`). Use actual affected domain
   operations where represented; mark missing gameplay explicitly rather than replacing it with
   a toy path and claiming parity.
2. **Second independent composition:** add an optional behavior/policy over the same family and
   exercise coexistence, overlapping access, registration conflict and removal. Specify which
   composition is permitted and which is rejected; zero optional modules must preserve base
   behavior. Prove the selected design's independent composition, not just a second arithmetic loop.
   #578 proves the private owner/composition seams; #583 proves real external consumers under
   #99's SDK contract after its #231/#578 prerequisites.
3. **Lifetime and failure:** move a non-`Clone` representative payload active -> detached -> active;
   preserve valid detached queries, incarnation generation and derived indexes. Cover failed
   attach, replacement, retirement/unload and stale work. If a family splits into components,
   prove complete bundle transfer, not temporary duplicated mutable authority or value equality
   after cloning. Keep save/acknowledgement and I/O-failure obligations from C1.
4. **Operations and callbacks:** exercise an actual multi-entity operation with its reciprocal
   relations and owned outcomes, plus query -> action result -> query behavior. Reject invalid
   admissions without mutation; execution failures follow the operation's explicit partial-effect
   contract, preserving already-completed actions when C++ does. Demonstrate reentry without
   retaining a mutable script/entity borrow across nested callbacks or deferring everything until
   the next tick.
5. **Execution:** trace the invoked production-linked paths for session admission, Player/cell
   updates, spell/aura/combat ordering, spawn/remove callbacks and the MapManager delayed-update
   barrier. Include removal during iteration and a saturated delivery sink. No I/O, delivery or
   `.await` may retain map/component guards. Enforce C0/C3 during each affected writer migration.
6. **Acceptance evidence:** exercise equivalent lookups, iteration, mutation, structural churn,
   transfer and owned-outcome work with real family payloads. Record allocations/RSS, latency and
   implementation/extension complexity, with date, code SHA, toolchain, parameters, samples and
   host architecture. Separate aarch64 from x86_64 evidence and old-Rust equivalence from C++
   contract proof. A checksum or a faster loop alone cannot establish production acceptance.

If a required capability is missing, name its owning C0–C4 work and the affected proof; do not
repeatedly rerun synthetic tests as a substitute. Repeat an experiment only to resolve a named
failure or materially changed workload. A speculative all-entity ECS rewrite or a complete content
port is not the price of implementing this decision.

Before a selected backend's authoritative production cut is accepted, complete the affected family's
non-`cfg(test)` integration and failure tests on the real owner paths, then perform the relevant
capture/live acceptance required by #578. Replace that family's representation rather than adding
a second live store. No production dependency or backend switch is installed by this plan update.

## Historical spike evidence — recorded 2026-09-04

The isolated spike used `hecs 0.11.1` with Rust 1.98.0. Eight focused tests were reported passing on
the hosts below. Their assertions cover the synthetic model, not all requirements of the revised
gate. No production crate depends on `hecs` at the reviewed code `93e4002a`.

Development-host measurement (`aarch64-unknown-linux-gnu`, release build; historical results,
not measurements recomputed by the 2026-09-05 plan update):

| Workload | HashMap baseline | Private `hecs` world | Result |
|---|---:|---:|---|
| 20,000 creatures x 200 updates, 5 samples | 40.76–41.14 ns/entity-update | 6.09–6.25 ns/entity-update | identical checksum |
| 100,000 creatures x 20 updates, 3 samples | 51.08–62.81 ns/entity-update | 7.38–7.88 ns/entity-update | identical checksum |
| 100,000-creature process RSS | 10,252–10,256 KiB | 19,324 KiB | ~9 MiB candidate overhead |

Hosted measurement (`x86_64-unknown-linux-gnu`, Ubuntu 24.04, release build, one isolated sample
per backend; same historical record) passed the same eight tests and produced:

| Workload | HashMap baseline | Private `hecs` world | Result |
|---|---:|---:|---|
| 100,000 creatures x 20 updates | 40.82 ns/entity-update | 4.79 ns/entity-update | identical checksum |
| 100,000-creature process RSS | 10,644 KiB | 19,768 KiB | ~9 MiB candidate overhead |

The x86_64 evidence is preserved in GitHub Actions run
[`33874182319`](https://github.com/alseif0x/rustycore/actions/runs/33874182319). The temporary
workflow used only `contents: read`, pinned checkout by full SHA, and was removed after the run; the
spike does not create a permanent CI gate.

Both synthetic implementations mutate transform/vitals, build owned outcomes, and sort them by GUID;
the measurement is not comparing an outcome-producing path with an empty loop. RSS is whole-process
Linux `/proc/self/status` evidence, not an allocation-exact component accounting. Real Player and
Creature payloads are much larger than the synthetic components, so the percentage must not be
extrapolated directly.

The 2026-09-05 source review bounds that evidence further
(`tools/architecture/map-runtime-spike/src/lib.rs`):

- `detach_player` clones `PlayerState` before despawning. Its equality test does not prove moving
  the same non-cloneable Player payload or rollback after a production attach failure.
- `apply_command(ApplyDamage)` checks the attacker locator but mutates only victim vitals. It does
  not prove reciprocal combat/threat/attacker transitions or coherent multi-entity persistence.
- `frame` returns `CPP_FRAME_PHASES.to_vec()` and runs represented work in `ObjectUpdate`. Comparing
  that vector with the same constant does not execute production phases or a multi-map barrier.
- Tiny synthetic components and whole-process RSS do not establish the layout, memory cost,
  extension complexity or end-to-end speed of real boxed Player/Creature aggregates.

The definitive backend-selection inference originally drawn from this old spike was unsupported.
Today's selective choice rests on the later V1 feasibility evidence plus explicit architectural
judgment; it does not retroactively turn this spike into conformance, production or SDK proof.

### Borrowed APIs: real constraint, not a blanket backend blocker

The production facade and narrower external API are useful progress. However, the earlier claim
that ordinary internal borrowed getters universally require unsafe guard elision or a simultaneous
API rewrite was too broad. Shared-world `hecs` reads use guards; in `hecs 0.11.1`,
[`World::query_one_mut`](https://docs.rs/hecs/0.11.1/hecs/struct.World.html#method.query_one_mut)
under `&mut World` returns query references directly, and `query_disjoint_mut` supports a fixed set
of distinct entities. Safe multi-key borrowing also exists for standard/library maps.

Audit shared-read and exclusive-mutation signatures separately. Internal references scoped to the
owner can remain when safe; sessions and external modules still receive commands, owned queries or
scoped domain capabilities. Removing their broad mutation access is an ownership obligation, not
proof that every internal getter must disappear before evaluating a backend. No unsafe guard
elision or parallel live authority is permitted.

## Preserved implementation checkpoint — 2026-09-04

The numbered steps in this historical account name the original implementation cuts, not the
current execution sequence. The ownership/API advances remain useful under either backend. The
active macro sequence follows this account; current inventories and acceptance are maintained in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md).

Implementation checkpoint (2026-09-04): step 2 is installed in production. `Map` remains the
single owner, while its `HashMap` is private behind the non-`Deref` `EntityWorld` facade. The
generic session-facing `map_object_record`/`ObjectAccessorMapSource` bridge has been removed;
`wow-world` now uses closure-scoped typed reads whose results cannot borrow the backend. The same
cut exposed and removed a recursive `MapManager` mutex acquisition in visibility: GameObjects are
snapshotted while the map is borrowed, then viewer-dependent Player work runs after the guard is
dropped. Remaining shared-read signatures require a backend-aware review; safe internal exclusive
borrows are not categorically blockers, as explained above.

Step 3 is complete without a shadow store. The facade owns the exact GUID lookup for a Creature
transform/vitals/spatial projection and produces stable GUID-ordered, owned batch snapshots for the
canonical Creature update visit. Its context resolver no longer receives `&Creature`; missing and
wrong-kind cell entries retain their explicit skip outcomes and never become zero/default
snapshots. Session/world adapters use that projection or closure-scoped access, and the borrowed
immutable Creature getter is now crate-private so another crate cannot regress across that seam.
The still-public mutable Creature transition API remains an ownership seam to narrow. Historical
step 4 introduces the owner command/outcome path; neither that path nor a specific count of retired
internal getters makes the backend decision automatic.

Steps 4 and the first slice of 5 are now installed. Every `ManagedMap` physically contains one
`MapRuntime`, and that runtime owns the existing `Map`; this is an ownership boundary, not a
decorative actor beside a separately owned map. `MapManager::execute_map_command_like_cpp` is the
temporary synchronous ingress while the existing outer manager mutex remains the synchronization
mechanism. Its result is always an owned `MapCommandOutcomeLikeCpp`, including explicit missing-map,
missing/wrong-kind entity, and same-unit rejection states.

The first complete represented vertical is Creature attack-start and evade combat-stop for Player
or Creature victims. The runtime validates both identities before mutation, applies the reciprocal
combat/threat/attacker transition under one exclusive owner borrow, and returns the post-transition
combat evidence. The global creature loop delivers only commands whose map outcome was applied;
the victim session no longer mutates either participant and is limited to recipient validation,
bounded-directory publication, and packet delivery. This is anchored to C++ `Unit::Attack`
(`Unit.cpp:5645-5745`), `CombatManager::SetInCombatWith` (`CombatManager.cpp:187-228`), and
`Unit::CombatStop` (`Unit.cpp:5802-5821`). It intentionally preserves the already represented Rust
transition rather than claiming full `Unit::Attack`/evade parity: spells, aura interruption, unit
state, AI callbacks, assistance, all-participant cleanup, and packet side effects remain separate
gaps. Public `ManagedMap::map_mut` and other mutable families also remain transitional seams, so
external synchronization has not yet moved into a dedicated runtime task and `hecs` is still not a
production dependency.

Step 6 is also installed without moving generation authority into the entity backend. `MapManager`
continues to own the `PlayerHandle` generation/residence registry and the explicit detached store;
the selected active `MapRuntime` now exclusively performs `AddPlayerToMap`/`RemovePlayerFromMap` on
the one boxed Player value. Rejected attachment returns that same box to the detached owner, and an
already-present active GUID is rejected instead of replacing the map record. Detach prevalidates
that the active record is a Player before removal. This preserves the far-teleport window from C++
`Player::TeleportTo` (`Player.cpp:1415-1455`) and the map container order in
`Map::AddPlayerToMap`/`RemovePlayerFromMap` (`Map.cpp:427-462,907-934`) while keeping stale-handle
rejection in the lifetime coordinator. The outer manager mutex is still the temporary atomicity
boundary across detached storage and a map runtime; no asynchronous actor handoff is claimed yet.

Step 7 work in progress: active Player relocation enters `MapRuntime` through the
generation-checked `MapManager::relocate_player_like_cpp` operation. The map updates the one Player
position and its derived cell membership together; detached and stale handles are rejected by this
active-map operation. Detached login/teleport preparation still changes the detached Player value
through the lifetime coordinator. C++ anchors are `Unit::UpdatePosition`
(`Unit.cpp:12257-12284`) and `Map::PlayerRelocation` (`Map.cpp:1015-1040`). This repairs the prior
direct-coordinate write that left the Player's cell index at its old location; it is not merely
a method relocation.

The grid materialization adapter is now constructed once in `world-server::app`, then borrowed by
movement, embedded spell movement, and login. Session no longer retains its optional resolver or
setter. This capability extraction preserves the existing grid-load call boundaries and the login
failure gate before success packets (`Map::EnsureGridLoadedForActiveObject` / `AddPlayerToMap`,
`Map.cpp:348-363,427-445`). The adapter still bridges canonical and legacy spawn materialization;
moving that work inside the final runtime and removing public mutable map access remain open.

The architecture hotspot and exact Session inventories were reconciled after review in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md). The dependency/ownership
check, architecture self-test and `session-ownership-check check --syntax-only` passed at that
checkpoint. Its historical counts were 292 production fields and 428 test fixtures, plus remaining
ownership debt; consult the current checkpoint for the later reconciliation. Those historical
counts and checks are not terminal #578 acceptance. The exhaustive persistence snapshot was not
recomputed by this update.

Historical focused evidence on the aarch64 development host: `cargo test -p wow-map player_owner --lib`
passes 9 tests, including active cell relocation, detached rejection and stale-generation
rejection. `cargo check -p wow-world --tests` and `cargo check -p world-server --tests` passed
with the explicit repository `PROTOC`; formatting and `git diff --check` also passed.
The `wow-world --lib movement` selection passes 79 tests. The separate
`unavailable_login_grid_aborts_before_success_login_packets_like_cpp` and
`cast_spell_applies_embedded_move_update_like_cpp` regressions each pass, preserving the login
failure publication gate and the spell handler's embedded movement path.
The complete `wow-world --lib` suite also passes: 3,671 passed, zero failed, one ignored.
The complete `wow-map --lib` suite passes: 703 passed, zero failed, one ignored.
`validation-v2 quick --base origin/3.4.3` passes, including the workspace all-targets check
(manifest `20260904T205853.305624Z-2-quick.json`). After also routing same-map residence through
MapRuntime, its cell-crossing regression and the full `wow-world --lib` suite pass again
(3,671 passed, zero failed, one ignored). The release world-server build passes on aarch64.
Clean-HEAD final validation and live/capture acceptance remain separate pending gates.

## Approved continuation — #578 C0–C4, with #99 / #583 extension acceptance

Follow the [approved modularity plan](../architecture/modularity-and-ecs-plan.md) and the current
#578 checkpoint. These are coherent internal checkpoints within one macro, not a micro-PR or
approval per component. First run the finite selected-design conformance proof above. Neither the
selection nor that proof replaces or narrows any C0–C4 exit contract:

Safe same-owner file/test decomposition can precede or accompany conformance: that gate controls
production storage migration, not physical organization. Apply the independent semantic and
physical acceptance in [module design guidelines](../architecture/module-design-guidelines.md)
to each C2 family and the C4 closeout; a cohesive aggregate is not an unlimited file exception.

1. **C0 — Execution contract:** establish admitted residence, owner, exact phase/callback barriers,
   persistence and publication for each affected operation. Define the shared contracts of both
   representative backend/extension cases before enabling another writer; #583 revalidates those
   same expectations against merged #578 rather than first defining them after conformance.
2. **C1 — Player lifetime and persistence:** complete failure/unload/save/logout and acknowledgement
   guarantees around existing generation-checked transfer. Preserve existing persistence fences;
   use this lifetime contract in finite conformance rather than rewriting it for a backend.
3. **C2 — Complete gameplay operations:** finish coherent command/query/outcome families and their
   consumers; retire superseded access/authority. After pre-migration conformance, integrate the
   selected `hecs` representation for the stateful/composable family through the same operation
   contracts, retaining cohesive aggregates. #99 governs the SDK direction and #583 delivers the
   production external-module and optional native/Wasm executor proof; #578 does not wait for
   #583's SDK or a universal extension framework.
4. **C3 — Runtime and delivery completion:** exercise the real admitted paths and C++ barriers,
   lifecycle callbacks, backpressure and shutdown; retire the remaining Session/legacy authority
   and entity bridges. Storage adoption does not complete this block or authorize parallelism.
5. **C4 — Boundary decisions and macro acceptance:** resolve inherited dependency/catalog/dispatch
   decisions, verify no dual authority or backend exposure, and run the required final inventories
   and clean-HEAD validation with affected integration/live/capture evidence. #153 verifies the
   completed architecture after the required #578 and #583 work. Neither the selected backend nor
   a green microbenchmark closes this block; any justified fallback follows the explicit
   hecs-specific reopening rule above rather than resetting a general candidate competition.

Keep optional customization and a port of base C++ behavior distinguishable. Complete the #583
module macro under #99's contract, including zero-module equivalence, conflict handling, state
lifetime, durable operation/receipt evidence, native/Wasm execution and compatible author/operator
install/update/removal. Wasm is delivered as an operator-optional executor under #583, not deferred
behind an M6 date and not made mandatory for native modules. That public API must not depend on
whether the selected family uses ECS components or an aggregate. No new
crate, scheduler or task is required merely to finish this sequence.

## Rollback and observability

Each production cut names owner/writer/phase before and after, its rollback or recovery boundary,
and packet/persistence ordering. Keep structural changes distinguishable from intentional behavior
changes, and preserve coherent internal commits within the macro. Do not promise that a completed
callback or durable state migration can be undone by reversing a field move.

The existing aggregate implementation is an old-Rust regression baseline, not the C++ correctness
oracle. C++/capture evidence governs parity. Validate the selected design in isolated conformance
and affected production tests; any necessary fallback comparison follows the reopening rule above.
Never install a shadow mutable authority in production. Record the selected backend,
owned operation traces, stale-handle rejections and callback/failure outcomes without exposing
backend IDs as game identity. Historical test/benchmark results above are not a new validation run.

## External references

- Veloren ECS overview: <https://book.veloren.net/contributors/developers/ecs.html>
- Veloren codebase structure: <https://book.veloren.net/contributors/developers/codebase-structure.html>
- `hecs`: <https://github.com/Ralith/hecs>
- Bevy ECS: <https://bevyengine.org/learn/quick-start/getting-started/ecs/>
