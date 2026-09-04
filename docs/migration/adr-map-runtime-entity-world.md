# ADR — Private entity world behind a single-writer MapRuntime

**Date:** 2026-09-04 · **Status:** Accepted for staged implementation · **Issue:** #578 / parent #133

## Decision summary

RustyCore will remain one deployable modular monolith. In-world gameplay state will converge onto
one `MapRuntime` per map instance. The runtime will be the single writer of a private entity world;
sessions, handlers, persistence adapters, and external modules will never receive an ECS entity ID,
a storage/query guard, or mutable access to that world.

The first candidate backend is `hecs`, evaluated in the isolated
`tools/architecture/map-runtime-spike` crate before it is added to any production crate. This ADR
does not authorize a `wow-ecs` crate, a global ECS, or a second copy of `Player`/`Creature` state.
The backend is an implementation detail of `wow-map` and is promoted to a separate crate only if a
later, demonstrated dependency boundary requires it.

## Context

The #578 branch has established a generation-checked `PlayerHandle` and transferred represented
gameplay families to the canonical `wow_entities::Player`. Active players live in canonical map
storage; far teleports preserve the same `Player` value in an explicit detached store. This fixes
authority, but the transitional access path still serializes many session operations through
closure-based access to a shared `MapManager`, and canonical map storage is still a heterogeneous
`HashMap<ObjectGuid, MapObjectRecord>` plus derived indexes.

That is a valid convergence bridge, not the terminal runtime:

- it centralizes mutable access but does not make the simulation owner explicit;
- field-oriented systems cannot batch over movement, vitals, auras, threat, or visibility;
- a large `MapObjectRecord` enum remains the common mutation surface;
- keeping a manager lock across gameplay, packet delivery, persistence, or `.await` would be an
  architectural failure even if the state itself had one nominal owner.

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
- During a far teleport the Player exists while it has no current Map. Failure to resolve an active
  map is therefore `None`/detached, never permission to synthesize zero/default gameplay values.

## Target boundary

~~~text
WorldSession                       MapRuntime(map_id, instance_id)
  identity / sockets                 owns thread/task + clock
  admission / dispatch       cmd --> private EntityWorld
  routing / mailbox                  GUID -> private generational entity index
  lifecycle / PlayerHandle  <-- outcome/query snapshot

outside the owner:                 inside the owner:
decode/encode                      ordered C++-like phases
database I/O                       canonical component mutation
packet delivery                    spatial/secondary-index maintenance
await/backpressure                 owned outcomes only
~~~

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

### Components

Migration starts coarse-grained to preserve C++ invariants and existing `Player`/`Creature`
behavior. Initial families are:

- identity/type and world presence;
- transform/map/phase;
- Unit core, vitals, and powers;
- movement;
- combat, auras, and threat;
- Player-only state;
- Creature-only state.

Components are split only when a real system needs an independent access pattern. A component per
field would recreate the monolith as scheduling and join complexity; a single permanent
`MapObjectRecord` component would merely rename the existing enum.

### Commands, queries, and outcomes

Handlers adapt packets into typed commands or narrow queries. `MapRuntime` applies them
synchronously inside the owner and returns owned values:

~~~text
decode -> MapCommand -> MapRuntime/EntityWorld -> Vec<MapOutcome>
       -> persistence/delivery adapter -> encode/send
~~~

No outcome borrows a component. Database work, channel backpressure, packet construction requiring
external catalogs, and every `.await` happen after all ECS borrows and map guards have been dropped.
Cross-entity transitions are atomic inside one owner; cross-map/realm work uses explicit
coordinators and idempotent outcomes rather than shared mutable access.

### Scheduling

ECS storage does not choose gameplay order. The map driver calls named phases explicitly in the
C++ order. A backend scheduler may be considered only after the phase trace is executable and only
within a phase whose operations are proven independent. Parallel iteration must not change packet,
publication, queue, or deterministic test order.

## Alternatives considered

### Keep `HashMap<ObjectGuid, MapObjectRecord>` as the terminal store

Rejected as the terminal design. It is simple and remains the migration baseline, but it forces
heterogeneous enum dispatch and whole-object mutable access across systems. It does not provide the
borrow separation needed for batched map-owned simulation.

### Expose a global ECS to sessions and handlers

Rejected. It would turn the ECS into a query-anything service locator, obscure ownership, permit
cross-map coupling, and make async/lock boundaries difficult to enforce.

### `bevy_ecs`

Not selected for the first production candidate. Its scheduler, resources, events, change
detection, and ecosystem are valuable for a general game engine, but RustyCore already has a
binding C++ phase order and its own Tokio/server runtime. Adopting the larger scheduling surface
before ownership convergence would add two orchestration models at once.

### `hecs`

Selected for the executable candidate because it is a small, scheduler-free ECS with generational
entity IDs and runtime borrow checks. Its lack of a scheduler is useful here: RustyCore retains an
explicit C++-ordered driver. Selection is conditional on the gate below; the project will not keep
it merely because the spike compiles.

### Custom arena/component store

Kept as the fallback. It gives maximum control and fewer dependencies, but RustyCore would own
generational allocation, borrow/query safety, archetype/component movement, and maintenance. It is
chosen only if the spike exposes a concrete `hecs` limitation or a measured cost that matters.

Veloren is useful evidence for an ECS-backed Rust server, especially its component-oriented state
and retained spatial indexes. It is not a template: RustyCore preserves Trinity's update order and
uses a private command/outcome boundary instead of allowing broad storage fetches from network
handlers.

## Executable gate

`tools/architecture/map-runtime-spike` must prove all of the following before `hecs` enters
`wow-map`:

1. active -> detached -> active moves one Player value and preserves state;
2. replacement invalidates a stale generation;
3. detached/unknown access fails closed and fabricates no defaults;
4. a batch query updates Creature transform and vitals;
5. a Player/Creature combat command returns an owned outcome and delivery can occur after the world
   is mutated again;
6. the phase trace matches the C++ order, including the MapManager barrier before delayed update;
7. GUID lookup and derived indexes survive entity relocation/despawn without exposing backend IDs;
8. the `HashMap` baseline and candidate produce the same deterministic semantic checksum;
9. timing and memory observations are recorded separately for aarch64 development and x86_64 CI.

The microbenchmark is a selection aid, not a parity proof. Correctness and boundary enforcement
win over a small synthetic timing difference.

## Initial gate evidence and backend selection

The isolated spike uses `hecs 0.11.1` with Rust 1.98.0. Eight focused tests prove the lifetime,
stale-handle, detached failure, batch-update, owned-outcome, phase-order, GUID-index, and semantic
checksum properties above. No production crate depends on `hecs` yet.

Development-host measurement (`aarch64-unknown-linux-gnu`, release build):

| Workload | HashMap baseline | Private `hecs` world | Result |
|---|---:|---:|---|
| 20,000 creatures x 200 updates, 5 samples | 40.76–41.14 ns/entity-update | 6.09–6.25 ns/entity-update | identical checksum |
| 100,000 creatures x 20 updates, 3 samples | 51.08–62.81 ns/entity-update | 7.38–7.88 ns/entity-update | identical checksum |
| 100,000-creature process RSS | 10,252–10,256 KiB | 19,324 KiB | ~9 MiB candidate overhead |

Hosted confirmation (`x86_64-unknown-linux-gnu`, Ubuntu 24.04, release build, one isolated sample
per backend) passed the same eight tests and produced:

| Workload | HashMap baseline | Private `hecs` world | Result |
|---|---:|---:|---|
| 100,000 creatures x 20 updates | 40.82 ns/entity-update | 4.79 ns/entity-update | identical checksum |
| 100,000-creature process RSS | 10,644 KiB | 19,768 KiB | ~9 MiB candidate overhead |

The x86_64 evidence is preserved in GitHub Actions run
[`33874182319`](https://github.com/alseif0x/rustycore/actions/runs/33874182319). The temporary
workflow used only `contents: read`, pinned checkout by full SHA, and was removed after the run; the
spike does not create a permanent CI gate.

Both implementations mutate transform/vitals, build the same owned outcomes, and sort them by GUID;
the measurement is not comparing an outcome-producing path with an empty loop. RSS is whole-process
Linux `/proc/self/status` evidence, not an allocation-exact component accounting. Real Player and
Creature payloads are much larger than the synthetic components, so the percentage must not be
extrapolated directly.

Decision: retain `hecs` as the selected entity backend. Its measured iteration benefit and existing
borrow/generation machinery justify the memory cost; the custom-store alternative would make
RustyCore maintain those unsafe-sensitive mechanisms itself. Production equivalence tests remain
mandatory. A materially different real-workload result can reopen the backend choice without
changing the single-writer/API boundary.

The production audit found one integration constraint that the isolated spike intentionally did
not model: `Map::map_object_record`, `ObjectAccessorMapSource`, the typed getters, and internal map
phases return ordinary `&MapObjectRecord` / `&mut MapObjectRecord` references. `hecs` shared-world
access returns borrow guards, so swapping the backend underneath those signatures would require a
wide simultaneous API rewrite or unsafe guard elision. Neither is acceptable. The first production
slice therefore installs a private, non-`Deref` `EntityWorld` facade over the existing `HashMap`.
Callers are then migrated to owned snapshots, typed commands, or closure-scoped access before the
facade changes backend. The old and new stores are never run side by side.

## Migration sequence

Implementation checkpoint (2026-09-04): step 2 is installed in production. `Map` remains the
single owner, while its `HashMap` is private behind the non-`Deref` `EntityWorld` facade. The
generic session-facing `map_object_record`/`ObjectAccessorMapSource` bridge has been removed;
`wow-world` now uses closure-scoped typed reads whose results cannot borrow the backend. The same
cut exposed and removed a recursive `MapManager` mutex acquisition in visibility: GameObjects are
snapshotted while the map is borrowed, then viewer-dependent Player work runs after the guard is
dropped. Typed borrowed getters used by internal map phases still have to be retired before the
facade can switch safely to `hecs`.

Step 3 is complete without a shadow store. The facade owns the exact GUID lookup for a Creature
transform/vitals/spatial projection and produces stable GUID-ordered, owned batch snapshots for the
canonical Creature update visit. Its context resolver no longer receives `&Creature`; missing and
wrong-kind cell entries retain their explicit skip outcomes and never become zero/default
snapshots. Session/world adapters use that projection or closure-scoped access, and the borrowed
immutable Creature getter is now crate-private so another crate cannot regress across that seam.
Internal `wow-map` borrows and the still-public mutable Creature transition API remain explicit
blockers to the backend swap; step 4 introduces the owner command/outcome path that will replace
those mutations before `hecs` becomes the live store.

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

1. Complete the isolated gate and record the decision in this ADR.
2. Introduce a private, non-`Deref` `wow-map::map::entity_world` facade owning the existing
   canonical records; keep the `HashMap` behind it while borrowed-record callers are migrated, with
   no session-facing API and no second store.
3. Move the GUID index and one low-risk batch family (Creature transform/vitals) behind the facade.
4. Add the single-writer `MapRuntime` command/outcome driver while preserving current external
   synchronization until callers migrate. **Complete for the staged synchronous owner.**
5. Route one complete vertical through a typed command/outcome and delete its closure access.
   **First vertical complete: represented Creature attack-start/evade combat-stop; continue with
   the remaining mutable families.**
6. Move Player attach/detach into the runtime without changing `PlayerHandle` semantics.
7. Migrate remaining component families and derived spatial indexes incrementally; delete each old
   representation in the same commit that installs its replacement.
8. Remove shared mutable Map access from sessions, then consider parallel work inside proven-safe
   phases.
9. Promote a crate only if multiple consumers require a stable downward API; otherwise keep the ECS
   private to `wow-map`.

## Rollback and observability

Each production slice keeps the previous public behavior and is independently revertible. A slice
must name owner/writer/clock before and after, preserve packet/persistence ordering, and add phase or
outcome evidence. The `HashMap` implementation remains the comparison oracle until the migrated
family has focused equivalence tests; it is not run as a shadow mutable authority in production.

## External references

- Veloren ECS overview: <https://book.veloren.net/contributors/developers/ecs.html>
- Veloren codebase structure: <https://book.veloren.net/contributors/developers/codebase-structure.html>
- `hecs`: <https://github.com/Ralith/hecs>
- Bevy ECS: <https://bevyengine.org/learn/quick-start/getting-started/ecs/>
