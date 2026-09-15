# Creature runtime audit — post-#953

**Audit date:** 2026-09-15  
**Rust integration head:** `507f3cfa` (PR #953; implementation candidate `e2ca3df9`)
**Scope:** the remaining #584 C0–C4 boundary around `Map::Update`, Creature
runtime ownership, effect consumption, persistence and publication.

This is the current evidence record for the next macro selection. It is linked
from `PORT_PLAN.md`, `STATE.md` and the #584 checkpoint; it does not replace
the master plan or create an issue per helper.

## C++ contract

The target path is one map-owned operation. `Map::Update` in
`/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:666-815`
updates the dynamic tree, admitted world sessions, respawns and spawn groups,
then visits object families through `Trinity::ObjectUpdater`, updates
transports, publishes `SendObjectUpdates`, and runs scripts, weather, personal
phase, move-list drains and relocation notifications. The visitor calls
`Update(i_timeDiff)` only for in-world objects (`Grids/Notifiers/GridNotifiers.cpp:258-301`).

For a normal Creature, `Creature::Update` is the complete operation at
`Entities/Creature/Creature.cpp:696-900`:

1. `JustAppeared` and vehicle/spirit-healer hooks, then movement flags.
2. Death/corpse/respawn compatibility handling.
3. `Unit::Update(diff)` (spline movement, auras, periodic effects and unit
   timers), followed by threat-manager and spell-focus updates.
4. Evade-boundary checks, dungeon combat pulses, `AIUpdateTick(diff)` and
   `DoMeleeAttackIfReady()`.
5. Death recheck, health/power regeneration and no-path evade accumulation.
6. In `CORPSE`, `Unit::Update`, engaged AI/loot updates and timed corpse removal.

The order matters. A request representation or a timer mutation without the
consumer side effects is not a Creature update.

## Current Rust reality

There are two live representations of one Creature:

| Concern | Current owner | Evidence | Consequence |
| --- | --- | --- | --- |
| Runtime/AI, movement generators, spell schedules, assistance, taunts, RNG and respawn flags | `wow_world::map_manager::WorldCreature` | `crates/wow-world/src/map_manager/mod.rs:70-143` | The global legacy manager is the actual mutable runtime owner. |
| Map visibility, object records, target reads, values and directed publication | `wow_map::Map::entity_world` | `crates/wow-map/src/map/update.rs:340-485` and map publication consumers | Canonical map consumers can observe a second Creature record. |
| Tick selection | `RuntimeTickOwner::GlobalLegacy` in production | `crates/world-server/src/app.rs:5294-5310` | The canonical Creature visitor is set to `ExternalRuntime`. |
| Synchronisation | cloned `Creature` snapshots | `crates/wow-world/src/session/mod.rs:3936-4034` | Health-revision and loot-authority CAS reject stale/conflicting snapshots, but do not make the clone authoritative. |

The production global loop runs player melee, lifecycle/respawn, movement,
aggro, spell selection and creature melee as separate passes
(`crates/world-server/src/runtime/delivery.rs:1563-1645`). The canonical map
loop still executes the rest of its map phases, but its Creature branch returns
an empty summary under `ExternalRuntime`
(`crates/wow-map/src/manager/state_1.rs:538-675`). This prevents a second timer
writer; it is a safety guard, not convergence of the two models.

`WorldCreature::clone` deliberately rebuilds or clears runtime-only state such
as the motion-master selector, chase target and represented active generator
(`map_manager/mod.rs:145-178`). The sync function replaces the canonical map
record only when the health timeline/revision and full health tuple permit it,
then reconciles loot authority and reciprocal threat references
(`session/mod.rs:3956-4034`). Those rules are useful stale-snapshot fences, but
they cannot preserve live AI, generator, RNG or deferred-effect identity across
an arbitrary clone.

The canonical map seam calls `Creature::runtime_update_plan` and records its
actions (`wow-map/src/map/update.rs:348-440`). No production consumer applies
that plan to AI, spells, combat, threat, scripts, values or packet fanout. The
legacy spell pass explicitly skips non-player effects and marks mixed
difficulty/non-instant or unrepresented hooks as unsupported
(`wow-world/src/session/legacy_runtime/creature_spell_tick.rs:8-18,79-117`).
The melee compatibility bridge likewise records unrepresented outcome/proc
work rather than claiming exact C++ parity. This is honest, but it means the
map plan is not an implementation of `Creature::Update`.

## Confirmed residuals

1. **One complete Creature phase is missing.** The six C++ responsibilities
   above are distributed across independent Rust passes and two owners. No
   single operation currently owns the complete read → mutate → outcome →
   publication transaction.
2. **Outcome consumption is incomplete.** Plans are produced, but there is no
   production consumer for the canonical Creature plan. Creature spell effects,
   AI/script callbacks, threat/proc chains, full melee hit outcomes and
   corpse/loot updates remain outside the canonical map consumer.
3. **Phase integration is incomplete.** The global legacy loop and canonical map
   loop share the configured interval but are separate scheduled tasks. The
   `ExternalRuntime` switch prevents double timer advancement; it does not prove
   C++ `Map::Update` order, same-frame visibility or failure/recovery semantics.
4. **Persistence/publication is only partially complete.** Respawn DB mutations
   and several visibility rails are typed and fenced, but complete death,
   corpse, loot, CREATE/DESTROY, aura/value publication and DB/restart/relogin
   behavior still require operation-specific evidence.
5. **The clone bridge is transitional.** Health-revision, incarnation, loot and
   threat fences must remain until the live owner is moved or the bridge is
   retired. Removing them before that point would reintroduce stale writes.

## Decision and selected macro

Do not perform a mass `WorldCreature` → `wow-map` move, add a generic context or
make the canonical plan appear complete by consuming only timer actions. The
selected architecture macro under #584 is:

**C3.1 — one map-owned Creature runtime outcome boundary.**

Its first delivery is a structural contract, not a gameplay shortcut. It must:

- define one map-tick input (`diff`, game time, map incarnation and admitted
  object set) and one typed `CreatureRuntimeOutcome` containing state changes,
  follow-up commands, DB mutations and visibility/publication intents;
- route the existing GlobalLegacy phases through that boundary in the verified
  C++ order, while keeping `ExternalRuntime` as an explicit no-op until the
  canonical consumer is actually complete;
- make the outcome carry incarnation, lifecycle and authority stamps so a
  dropped or delayed delivery cannot mutate a replacement object;
- keep all synchronous map/legacy guards out of async delivery and preserve the
  established canonical → legacy lock order where the bridge still exists;
- add production-linked once-per-tick, phase-order, stale-snapshot,
  death/respawn and dropped-delivery regressions; no timer-only test can accept
  this boundary; and
- list every remaining unrepresented C++ reader/writer before claiming the
  legacy mirror retired.

The macro does **not** implement every AI family, spell effect, script or combat
formula. Those complete behavior deliveries remain in #23/#27/#29/#31/#32/#33/
#34 and must consume the typed outcome rather than create another Creature
writer. After C3.1, select the first complete behavior vertical by dependency:
melee outcome/application (#29 → #31), then aura/proc/effects (#32/#33/#34),
then scripts and the remaining lifecycle/persistence/capture gates. A behavior
macro may touch several crates, but it remains one coherent branch/PR with its
own C++ and live/DB evidence.

## C3.1 implementation evidence — 2026-09-15

The first structural delivery is implemented in `e2ca3df9`:

- `world-server/src/runtime/delivery.rs` now captures one immutable
  `CreatureRuntimeTickInputLikeCpp` per production tick; its private
  `runtime/delivery/creature_boundary.rs` submodule owns the typed input and
  outcome contract. It records the
  measured `diff_ms`, a monotonic loop epoch, game time, map incarnations and
  canonical Creature GUIDs admitted by each loaded-grid `ObjectUpdater` set.
- The production `GlobalLegacy` loop passes that input to one explicit
  `run_legacy_creature_runtime_tick_with_input_and_deliver_once_like_cpp`
  boundary. Its output carries the input identity, actual completed phase
  order, map-incarnation mismatch count, publication events, queued session
  commands and respawn DB mutations produced/submitted. The compatibility
  wrapper remains for existing focused callers and captures epoch zero.
- `wow-map::Map::admitted_creature_guids_like_cpp` exposes only the owned
  ObjectUpdater snapshot, so the boundary does not copy or publish map storage.
  Existing lock order and delivery-outside-guard rules are unchanged.
- `scenarios_13::legacy_creature_global_runtime_task_delivers_lifecycle_movement_and_melee_like_cpp`
  proves the production-linked phase order, two admitted maps, lifecycle
  corpse removal, movement publication and melee application. The negative
  `creature_runtime_boundary_detects_stale_map_incarnation_like_cpp` regression
  proves replacement-map rejection. Commands used were:
  `PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=1 cargo test
  --locked -p world-server
  legacy_creature_global_runtime_task_delivers_lifecycle_movement_and_melee_like_cpp
  --lib` and the corresponding stale-incarnation filter; both passed at the
  candidate SHA. `cargo check -p world-server`, `cargo fmt --all -- --check`
  and `git diff --check` also passed.

This is a structural boundary, not completion of `Creature::Update`. The
legacy owner, per-phase delivery and `ExternalRuntime` fail-closed branch stay
in place. Dropped channel sends, deferred effect consumers, AI/script hooks,
complete melee/proc outcomes and live DB/restart/relogin evidence remain
explicit follow-up gates. The next implementation macro must consume this
envelope instead of adding another Creature writer.

## Acceptance boundary

The legacy owner is not removable when a plan merely exists or when a focused
unit test records an action. Retirement requires a production consumer, no
unaccounted reader or writer, preserved C++ phase/order and lock contracts,
positive/negative/failure tests, exact packet/capture evidence for changed
observable output, and real DB/restart/relogin evidence for durability. Until
those gates pass, the two representations are a documented transitional
boundary and `ExternalRuntime` must remain fail-closed.
