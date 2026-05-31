# ADR — Live-runtime tick ownership and convergence

**Date:** 2026-05-29 · **Status:** Accepted · **Base commit:** `6671dee`

This ADR fixes a clean starting point for converging RustyCore onto a real live
runtime. It supersedes stale runtime claims in `MIGRATION_ROADMAP.md` / `_INDEX.md`
(their L3 status snapshots predate the canonical map work and have drifted).

## Context (verified)

Three world models coexist today (characterized + regression-anchored in
`#NEXT.R8.ENTITIES.764`):

1. **Legacy `wow_world::MapManager`** (`Arc<RwLock<…>>`, `crates/world-server/src/main.rs`).
   Shared across sessions. Runs creature **AI/combat** via per-session ticks
   (`tick_creatures_sync`, `tick_combat_sync` in `crates/wow-world/src/session.rs`).
   **No clock of its own** — advances only when a logged-in session ticks it.
2. **Canonical `wow_map::MapManager`** (`crates/wow-map/src/manager.rs`).
   Owns the global tick loop (`spawn_canonical_map_update_loop`, ~10ms) and a faithful
   `Map::Update` phase structure, but its creature update uses
   `CreatureRuntimeUpdateContext::default()` and **does not dispatch real AI/combat**
   (no `match` executes `AiUpdateTick`/`MeleeAttackIfReady`).
3. **Global world loop** — ticks only the canonical manager (2), never the legacy (1).

The old `WorldSession.creatures: HashMap` field no longer exists; do not build on it.

### C++ structural truth (verified against legacy)

- `World::Update` → `sMapMgr->Update(diff)` — `World.cpp:2748`.
- `Map::Update(t_diff)` — `Map.cpp:666` — runs, in order:
  1. `_dynamicTree.update`
  2. **worldsessions for existing players** (`session->Update(t_diff, updater)`)
  3. **respawns** (`ProcessRespawns`, `UpdateSpawnGroupConditions`)
  4. **`Trinity::ObjectUpdater`** over active cells — this is where creatures/pets/active
     objects update (AI/combat)
  5. then `SendObjectUpdates`, scripts, weather, move-list drains, relocation notifies.

So in C++ the **global map tick owns the creature/AI/combat update**, and player
sessions are updated as an **earlier phase of the same map tick** — not the other way
around. The Rust legacy model (each session owns creature AI/combat) is **structurally
inverted** from C++. The canonical `wow_map::MapManager` already mirrors the C++
`Map::Update` phase order, so it is the correct **structural destination**.

## Decision

1. **Single tick-owner invariant.** A creature/combat tick is owned by exactly one of:
   the session OR the global runtime — **never both**. The deadly bug to avoid is a global
   tick that *adds to* the per-session tick (double resolution). Introduce an explicit
   owner (e.g. `RuntimeTickOwner::{ Session, GlobalLegacy }`).
2. **Legacy is the transitional behavior engine; canonical is the structural destination.**
   Do not consolidate everything onto `wow_map::MapManager` at once (it has the clock and
   structure but not real AI/combat — porting that surface in one shot repeats the `_attic/`
   big-bang that died with 176 compile errors). Keep legacy running the behavior, move it
   under a global clock, then migrate the source of truth method-by-method.
3. **Migrate ownership/fanout before logic.** Get "who ticks" and "who sends packets to which
   sessions" right before moving gameplay resolution.
4. **Track separation.** `#NEXT.R8.ENTITIES.*` is the represented-logic mini-phase. The live
   runtime convergence is **L3/L4 of `MIGRATION_ROADMAP.md`** and must use roadmap
   phase/module IDs, not the `R8.ENTITIES` namespace.

## Refined sequence (supersedes the earlier handoff roadmap order)

1. ✅ Characterize the split — `#NEXT.R8.ENTITIES.764` (tests only).
2. **This ADR** — clean starting point; minimal reconciliation of roadmap/_INDEX drift.
3. **Infra, no behavior change:** add the `RuntimeTickOwner` guard; extract the bodies of
   `tick_creatures_sync`/`tick_combat_sync` into reusable helpers driven by a
   `PacketSink`/`RuntimeOutput`, callable by either a session or the global runtime. Default
   stays `Session`. Add a regression test proving a creature is ticked **once** with two
   sessions on the same map.
4. **First behavior change:** global legacy tick owner with session creature/combat ticks
   **disabled** for that responsibility (NOT global tick in addition to session tick).
5. Per-map session registry + creature-move/object-update fanout from the global tick.
6. Move combat resolution to the global owner (resolve once, not per session).
7. Migrate the source of truth toward `wow_map::MapManager`, method by method; retire legacy.
8. Real `SendObjectUpdates`, scripts, weather, threat, remaining fanout.

## Risks to respect

- **Double resolution** — two sessions on one map advancing the same state twice. Mitigated by
  the single-owner invariant (step 3) before any global behavior tick (step 4).
- **C++ phase order** — the global tick must respect `Map::Update`'s phase sequence
  (sessions → respawns → ObjectUpdater → SendObjectUpdates → scripts/weather/relocation), not
  an ad-hoc order.
- **No packets under map lock** — the global tick must not send session packets while holding a
  map `RwLock`/`Mutex`. Build packet plans inside the lock, send outside it.
- **Locks in Tokio** — `std::sync::RwLock`/`Mutex` are acceptable only for short sections with
  no `.await`. Heavy simulation belongs on a dedicated task/thread or computes plans outside
  the lock.
- **Single source of truth** — while legacy and canonical coexist, every mutation needs a single
  owner or explicit sync.
- **Backpressure** — a full session channel must not block the global tick under a lock.
- **Unload / active grids** — C++ does not update everything always; it respects maps, loaded
  grids, players, and active non-players. Do not globally tick idle/unloaded content.

## Consequences

- The next production slice is **infrastructure** (`RuntimeTickOwner` + extract-to-helper, no
  behavior change), not gameplay.
- Progress metric note: this convergence advances the live-runtime axis (~0% today, per
  `honest-progress-audit.md`), not the `R8.ENTITIES` inventory count.

## Slice 4 subdivision (validated with Codex)

Step 4 ("first behavior change") is too large for one slice and is split. **Combat stays out of
Slice 4** — `WorldSession::run_combat_tick` is per-PLAYER (the player's auto-attack swing,
`session.rs` ~20417/20427), not per-creature, so it belongs to **Slice 6** ("move combat to the
global owner"), not 4A. Slice 4A is limited to the **creature (AI/movement/respawn) tick**.

**Visibility amendment (Codex, mandatory):** `PlayerBroadcastInfo` (`map_id`, `position`,
`is_in_world`) is enough for **candidate routing only**, NOT for C++-faithful final delivery.
`MessageDistDeliverer` (Object.cpp:1746-1764, GridNotifiersImpl.h:43-46) also filters `InSamePhase`,
2D/3D distance per `required3dDist`, **`HaveAtClient(source)`**, shared vision/seer/vehicle,
`own_team_only`, `skipped_receiver`. Decision: do NOT duplicate visibility/phase in the registry.
Final delivery uses `SessionCommand::SendIfVisibleLikeCpp { source_guid, packet_bytes }` and **each
session applies its own `client_visible_guids_like_cpp`** (the per-session `HashSet<ObjectGuid>` =
HaveAtClient gate). `client_visible_guids` stays per-session (not moved to the map).

Sub-slices (each compiles, suite green, no production behavior change until the flip in 4B):
- **4A.1a — DONE (`#NEXT.RUNTIME.L3.002`, 4ab11af):** addressable types in `map_manager.rs`
  (`RecipientRule`/`RuntimeEvent`/`RuntimePlan`, `RuntimeOutput::into_owning_session_plan`,
  `MapManager::active_map_keys`). Pure types, gated OFF, 1062/0.
- **4A.1b — DONE (`#NEXT.RUNTIME.L3.003`):** `SessionCommand::SendIfVisibleLikeCpp { source_guid,
  map_id, instance_id, packet_bytes }` + per-session visibility gate in `handlers/loot.rs`
  (gates in order: LoggedIn, map_id, instance_id, `client_visible_guids_like_cpp.contains` =
  HaveAtClient; mirrors `SendVisibleObjectValuesUpdate`) + `resolve_runtime_event_candidates_like_cpp`
  / `deliver_runtime_plan_like_cpp` in world-server (try_send, non-blocking, candidates cloned out
  of the DashMap before sending). Confirmed Codex refinements: (1) `instance_id` added to
  `PlayerBroadcastInfo` and the command, filled from the canonical map key (fallback 0) and filtered
  everywhere — avoids cross-instance bleed; (2) `required_3d` honored (2D vs 3D distance) in
  `NearbyVisible`; (3) `SelfOnly` is NOT broadcast (skipped with `self_only_skipped`, no guessing of
  owning session); `ExplicitPlayer` reads the target's map_id/instance_id from the registry entry so
  the session map gate accepts it. Dormant in production (no caller until 4A.3).
- **4A.2 (split, Codex-validated):** move `respawn_queue` from `WorldSession` to the map (world
  state). **CRITICAL framing: this fixes respawn-queue OWNERSHIP, NOT multi-session delivery** —
  while `run_creatures_tick` stays session-local, only the draining session sees the respawn CREATE;
  real fanout is 4A.3/4B. Behavior: byte-identical with 1 session; with N sessions it replaces a
  mis-modeled per-session queue (latent bug) with a single map queue — a bugfix, not a regression,
  NOT gated. `client_visible_guids` stays per-session. NOT fused with the canonical
  `wow_map RespawnStoreLikeCpp` (by SpawnId/DB — that's step 7). `instance_id=0` legacy-path limit.
  - **4A.2a — DONE (`#NEXT.RUNTIME.L3.004`):** `PendingRespawn` moved to `map_manager.rs`;
    `MapInstance.respawn_queue: Vec<PendingRespawn>` + `push_respawn`/`drain_ready_respawns`
    (ready in insertion order, non-ready stay)/`respawn_queue_len`; delegates on `MapManager` by
    `(map_id, instance_id)`. At this sub-slice it was dormant (`run_creatures_tick` still used the
    session field); 4A.2b repointed the tick to this map-owned queue. 6 tests; 1074/0. 1 code file.
  - **4A.2b — DONE (`#NEXT.RUNTIME.L3.004`):** repointed `run_creatures_tick` to the map queue via
    session helpers `push_map_respawn_like_cpp`/`drain_ready_map_respawns_like_cpp` (lock only for
    push/drain, released before building packets / `register_world_creature`), removed
    `WorldSession::respawn_queue`. Byte-identical for 1 session (reviewer confirmed the drain logic
    and packet-build loop are unchanged). 3 tests; wow-world 1077/0. **4A.2 complete.**
- **4A.3 (higher risk, gated by owner/config):** separate legacy creature-tick driver (NOT hooked
  into the canonical loop) that ticks creatures once per map, builds a `RuntimePlan` under the lock,
  releases the lock, resolves recipients, and delivers via `try_send`. Owner stays `Session` by
  default; `GlobalLegacy` is used in tests and by the explicit 4B.1 experimental flag.
  - **Design decisions (orchestrator call, conservative + C++-faithful; revisable):**
    Q1 create/destroy — the global driver does **MOVEMENT ONLY**; create/destroy stay per-session,
    which is the C++ `Player::UpdateVisibilityOf` model (per-player visibility update). NOT Q2 (no
    mutating the receiver's set). Q2 npc_flags per-viewer — moot, happens at the per-session CREATE.
    Q3 canonical ECS sync — stays in the existing `mutate_world_creature` per-creature path; the
    driver's lock-ordering (simulate under RwLock → release → sync canonical under Mutex) is 4A.3b's
    concern. Q4 active grids — deferred (the legacy map only holds spawned creatures).
  - **4A.3a — DONE (`#NEXT.RUNTIME.L3.005`):** extracted the per-creature movement step from
    `run_creatures_tick`'s closure into a session-free free function `step_creature_movement_like_cpp`
    (returns the `MonsterMove` bytes; the tick delegates to it). Byte-identical, reusable by the
    future driver. 3 tests; wow-world 1080/0; warning count unchanged vs baseline. 1 code file.
  - **4A.3b — DONE (`#NEXT.RUNTIME.L3.006`):** single-shot legacy global movement driver body,
    gated by `RuntimeTickOwner::GlobalLegacy`. At this sub-slice it had no loop and no production
    caller; 4B.1 later wires it behind an experimental flag. It iterates
    `active_map_keys`, mutates movement under the legacy RwLock, collects canonical sync snapshots
    and `RuntimePlan` events, releases the legacy lock, syncs canonical state under the canonical
    Mutex, then returns the plan for delivery outside all map locks. `world-server` has a private
    bridge that runs the single-shot and delivers via `deliver_runtime_plan_like_cpp` for tests only.
    Verified: default `Session` owner is no-op; `GlobalLegacy` moves once, emits `NearbyVisible`
    `OnMonsterMove`, fans out to two candidate sessions, rejects wrong-map candidates, and keeps the
    canonical typed creature in sync. No task/loop yet.
  - **4A.3c — DONE (`#NEXT.RUNTIME.L3.008`..`#NEXT.RUNTIME.L3.010`):** create/destroy/respawn
    visibility from the global owner.
    C++ contrast: `Creature::Update` (`Creature.cpp:696`) is not movement-only; it also drives corpse
    removal / respawn-side state before `Map::Update` reaches the object updater.
    `SendIfVisibleLikeCpp` is intentionally **not** enough for CREATE: unseen objects fail the
    HaveAtClient gate and would never enter `client_visible_guids_like_cpp`. Implementation is
    dormant by default and only reached by the 4B.1 experimental flag: (1)
    `SessionCommand::RefreshVisibleWorldCreaturesLikeCpp { map_id,
    instance_id }`, gated by LoggedIn/map/instance, calls `force_update_visibility_like_cpp()` so the
    session computes CREATE/DESTROY and mutates its own visible set; (2) `world-server` can enqueue
    that command to all sessions on a map instance via
    `deliver_refresh_visible_world_creatures_like_cpp` using `try_send`, with no map locks held; (3)
    `run_legacy_creature_lifecycle_tick_once_like_cpp` processes corpse despawn + map respawn queue
    once under `GlobalLegacy`, removes/inserts canonical creatures after releasing the legacy lock,
    and returns the map instances that need a refresh fanout.
- **4A.4 — DONE (`#NEXT.RUNTIME.L3.007`, `#NEXT.RUNTIME.L3.012`):** gated task integration in tests
  only. One tokio test flips `GlobalLegacy`, runs the single-shot movement bridge from a spawned task
  via `spawn_blocking`, and proves fanout/canonical-sync through the task boundary. A second combined
  bridge runs lifecycle refresh first, then movement delivery from the same task boundary, proving the
  dormant-by-default global creature runtime can process corpse removal visibility refresh and
  MonsterMove fanout together. Production now has a caller only through the 4B.1 experimental flag and
  still defaults to `Session`.
- **4B.1 — DONE (`#NEXT.RUNTIME.L3.013`):** production wiring exists behind an experimental config
  flag, default off. `RustyCore.LegacyCreatureGlobalRuntime = 0` keeps production behavior unchanged.
  When explicitly enabled, startup flips the legacy map owner to `GlobalLegacy` and spawns the global
  creature runtime loop at the C++ `MapUpdateInterval` cadence. The loop uses `spawn_blocking` and the
  same combined lifecycle+movement bridge proven by tests.
- **4B.2:** manual client/server verification with the experimental flag enabled. Not marked
  `manual-test-ready` until the server is actually run and a client observes the realm behavior.
  `tick_combat_sync` remains session-owned because C++ `Player::Update` calls
  `DoMeleeAttackIfReady()` before map `ObjectUpdater`.
- **4C.1 — DONE (`#NEXT.RUNTIME.L3.014`):** dormant victim-session delivery rail for future global
  creature melee. C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()` from the map
  object update phase, so the map-owned driver must resolve each creature swing once and then deliver
  one result to the victim session. Added
  `SessionCommand::ApplyCreatureMeleeDamageLikeCpp { attacker_guid, victim_guid, map_id,
  instance_id, damage, over_damage, target_level, victim_health_after }`; the session gates LoggedIn,
  own victim GUID, map, instance, and `client_visible_guids_like_cpp.contains(attacker_guid)`, then
  sets final player health and sends one `AttackerStateUpdate`. No production caller yet; this is
  infrastructure for the real creature-combat driver.
- **4C.2 — DONE (`#NEXT.RUNTIME.L3.015`):** dormant single-shot legacy creature melee driver body.
  It gates on `RuntimeTickOwner::GlobalLegacy`, scans map-owned creatures for ready player-victim
  swings, releases the legacy map lock, applies damage to the canonical player under the canonical
  mutex, then returns `ApplyCreatureMeleeDamageLikeCpp` commands for delivery outside all map locks.
  No production caller yet; the next slice must add world-server delivery and then decide whether the
  experimental runtime loop includes melee before manual 4B/4C validation.
- **4C.3 — DONE (`#NEXT.RUNTIME.L3.016`):** world-server delivery for the dormant creature melee
  driver. `deliver_creature_melee_damage_commands_like_cpp` routes each already-resolved command to
  the exact victim session via `PlayerRegistry`, gates `is_in_world` + map + instance, and enqueues
  `SessionCommand::ApplyCreatureMeleeDamageLikeCpp` with `try_send` outside map locks. The combined
  experimental runtime bridge now runs lifecycle, movement, then creature melee. The session command
  no longer writes canonical health again; canonical health is owned by the global driver, while the
  session mirrors final represented health and sends `AttackerStateUpdate`.
- **4C.4 — DONE (`#NEXT.RUNTIME.L3.017`):** transitional global creature aggro rail. C++ contrast:
  `CreatureAI::MoveInLineOfSight` checks `Creature::CanStartAttack` and engages the target, then
  `Unit::SendMeleeAttackStart` notifies the client. Rust now snapshots in-world players from
  `PlayerRegistry`, lets the legacy map owner run the existing represented `try_aggro` radius model
  once, and routes `SessionCommand::CreatureAttackStartLikeCpp` to the exact victim session. This is
  intentionally not full AI parity yet: faction/LOS/gray-aggro checks remain later fidelity work.

## Progress log (runtime slices)

- 2026-05-29 — Slice 3 `#NEXT.RUNTIME.L3.001` (3308647): `RuntimeTickOwner` infra + extract
  `run_*_tick` + guard. No behavior change.
- 2026-05-30 — Slice 4A.1a `#NEXT.RUNTIME.L3.002` (4ab11af): addressable types. No behavior change.
- 2026-05-30 — Slice 4A.1b `#NEXT.RUNTIME.L3.003`: SendIfVisibleLikeCpp command + per-session
  visibility gate + candidate routing/delivery (try_send). instance_id/required_3d/SelfOnly
  refinements integrated. Dormant in production. wow-world 1068/0, world-server 266/0, wow-network 14/0.
- 2026-05-30 — Slice 4A.2a `#NEXT.RUNTIME.L3.004`: PendingRespawn -> map_manager.rs +
  MapInstance/MapManager respawn-queue API (dormant). 6 tests; wow-world 1074/0. Fixes ownership,
  not delivery.
- 2026-05-30 — Slice 4A.2b `#NEXT.RUNTIME.L3.004`: run_creatures_tick repointed to the map queue;
  WorldSession::respawn_queue removed. Byte-identical 1 session. 3 tests; wow-world 1077/0.
  **4A.2 complete.**
- 2026-05-30 — Slice 4A.3a `#NEXT.RUNTIME.L3.005`: extracted session-free
  `step_creature_movement_like_cpp` (movement step) from run_creatures_tick; byte-identical,
  reusable by the future driver. 3 tests; wow-world 1080/0.
- 2026-05-30 — Slice 4A.3b `#NEXT.RUNTIME.L3.006`: single-shot global legacy creature movement
  driver, explicit legacy->canonical lock ordering, canonical sync helpers with explicit
  `(map_id, instance_id)`, and a world-server bridge to `deliver_runtime_plan_like_cpp`. At this
  slice there was no production caller/loop; 4B.1 later wires it behind an experimental flag.
  Verification: wow-world 1084/0, world-server targeted bridge test 1/0, `cargo check -p
  world-server`, fmt/check/diff-check.
- 2026-05-30 — Slice 4A.4 `#NEXT.RUNTIME.L3.007`: test-only task/flip path. `GlobalLegacy` is enabled
  only inside the integration test; production remains `Session` and has no global legacy task.
  Verification: world-server targeted task test 1/0.
- 2026-05-30 — Slice 4A.3c.1 `#NEXT.RUNTIME.L3.008`: dormant refresh-visibility command for future
  global create/destroy/respawn delivery. Adds
  `SessionCommand::RefreshVisibleWorldCreaturesLikeCpp { map_id, instance_id }`; the session accepts
  only LoggedIn same-map same-instance commands and forces its own visibility pass, preserving
  per-session `client_visible_guids_like_cpp` ownership. Verification: wow-network targeted 1/0,
  wow-world targeted refresh tests 2/0.
- 2026-05-30 — Slice 4A.3c.2 `#NEXT.RUNTIME.L3.009`: dormant world-server refresh fanout helper.
  `deliver_refresh_visible_world_creatures_like_cpp(map_id, instance_id, registry)` filters
  `is_in_world` + map + instance, clones candidates out of `PlayerRegistry`, then enqueues
  `RefreshVisibleWorldCreaturesLikeCpp` with `try_send`. At this slice there was no production
  caller; 4B.1 later wires it behind an experimental flag. Verification: world-server targeted
  refresh fanout tests 2/0.
- 2026-05-30 — Slice 4A.3c.3 `#NEXT.RUNTIME.L3.010`: dormant global lifecycle driver.
  `run_legacy_creature_lifecycle_tick_once_like_cpp` gates on `GlobalLegacy`, removes elapsed corpses,
  pushes map-owned respawns, drains ready respawns, preserves captured phase shift across session-free
  respawn, syncs canonical remove/insert outside the legacy lock, and returns refresh map keys.
  `world-server` bridge fans those keys out through the refresh command rail. Verification:
  wow-world targeted lifecycle tests 3/0, world-server targeted lifecycle bridge test 1/0. NEXT:
-  combine movement+lifecycle under one test-only task before any production flip.
- 2026-05-30 — Runtime guard correction `#NEXT.RUNTIME.L3.011`: `GlobalLegacy` now suppresses only
  the session creature tick. Player combat stays session-owned, matching C++ `Player::Update`
  (`DoMeleeAttackIfReady`) and avoiding an invalid production flip that would freeze auto-attack.
- 2026-05-30 — Slice 4A.4 combined bridge `#NEXT.RUNTIME.L3.012`: added a
  `world-server` single-shot bridge that runs global lifecycle refresh and movement delivery from one
  `spawn_blocking` task in tests only. Verification: one task removes an expired corpse, enqueues
  `RefreshVisibleWorldCreaturesLikeCpp` to same-map sessions, moves a live creature once, enqueues
  `SendIfVisibleLikeCpp` MonsterMove to the same candidates, rejects wrong-map sessions, and keeps
  canonical movement state synced. 4B.1 later reuses this bridge behind an experimental flag.
- 2026-05-30 — Slice 4B.1 wiring `#NEXT.RUNTIME.L3.013`: added
  `spawn_legacy_creature_runtime_update_loop_like_cpp`, wired from `main` behind
  `RustyCore.LegacyCreatureGlobalRuntime` (default `0`). If enabled, startup sets
  `RuntimeTickOwner::GlobalLegacy`; otherwise the server still defaults to session-owned creature
  ticks. The loop uses the same C++ `CONFIG_INTERVAL_MAPUPDATE` cadence and runs the combined
  lifecycle+movement bridge in `spawn_blocking`. Verification: `cargo check -p world-server`.
- 2026-05-30 — Runtime creature-combat rail `#NEXT.RUNTIME.L3.014`: added dormant
  `ApplyCreatureMeleeDamageLikeCpp` session command for map-owned creature melee results. Verification:
  wow-network targeted command test 1/0, wow-world targeted victim-session delivery tests 2/0.
- 2026-05-30 — Runtime creature-combat driver body `#NEXT.RUNTIME.L3.015`: added
  `run_legacy_creature_melee_tick_once_like_cpp` with explicit legacy->canonical lock ordering and
  no production caller. Verification: wow-world targeted melee driver tests 2/0.
- 2026-05-30 — Runtime creature-combat world-server delivery `#NEXT.RUNTIME.L3.016`: added
  explicit victim-session delivery for global creature melee commands, integrated it into the
  experimental combined runtime bridge, and removed the stale-overwrite-prone canonical write from
  the session handler. Verification: world-server targeted delivery tests 3/0 and bridge test 1/0.
- 2026-05-30 — Runtime creature-aggro rail `#NEXT.RUNTIME.L3.017`: added
  `CreatureAttackStartLikeCpp` session command, a map-owned global aggro scan over player-registry
  snapshots, world-server delivery, and combined bridge coverage. The represented model is still
  radius-only; C++ `CanStartAttack` fidelity remains open.
- 2026-05-30 — Runtime creature-aggro bridge proof `#NEXT.RUNTIME.L3.018`: added a positive
  combined-runtime bridge test that starts creature combat from the map-owned aggro scan and delivers
  one `CreatureAttackStartLikeCpp` command to the victim session without producing movement or melee
  side effects in the same controlled tick. This keeps 4C.4 honest: scan, delivery, and bridge wiring
  are covered; full C++ `CanStartAttack` parity is still a later AI-fidelity slice.
- 2026-05-30 — Runtime creature-aggro react-state gate `#NEXT.RUNTIME.L3.019`: ported the first
  `CreatureAI::MoveInLineOfSight` gate from C++: normal proximity aggro now requires
  `REACT_AGGRESSIVE` before entering combat. This closes the passive/defensive false-aggro gap in
  the represented `try_ai_aggro` helper and the global legacy aggro driver. Remaining
  `CanStartAttack` fidelity still includes faction, immunity, z-distance, gray aggro, and LOS.
- 2026-05-30 — Runtime creature-aggro no-radius gate `#NEXT.RUNTIME.L3.020`: moved the existing
  session-side `aggro_radius <= 0` guard into `Creature::try_ai_aggro`, so the global map-owned
  aggro driver cannot start combat for Rust's represented non-aggro/neutral spawns (for example
  faction 35 mapped to radius 0). This is not full `Creature::CanStartAttack`; it closes the
  immediate divergence introduced by calling the helper directly from the global driver. Remaining
  `CanStartAttack` fidelity still includes faction/neutrality as first-class data, immunity,
  z-distance, gray aggro, and LOS.
- 2026-05-30 — Runtime creature-aggro immune-to-PC gate `#NEXT.RUNTIME.L3.021`: ported the direct
  C++ `Creature::CanStartAttack` check that rejects `IsImmuneToPC()` when the target is
  `UNIT_FLAG_PLAYER_CONTROLLED`. The global aggro scan currently only targets player-registry
  candidates, so this gate is represented by `UnitFlags::IMMUNE_TO_PC` on the creature. Remaining
  `CanStartAttack` fidelity still includes faction/neutrality as first-class data, target
  attackability, z-distance, gray aggro, and LOS.
- 2026-05-30 — Runtime creature-aggro civilian gate `#NEXT.RUNTIME.L3.022`: represented C++
  `Creature::IsCivilian()` (`CREATURE_FLAG_EXTRA_CIVILIAN`) in `Creature::try_ai_aggro` and the
  global legacy aggro driver. This closes the entity/runtime gate when `flags_extra` is present.
  The simple runtime spawn loader still needs first-class propagation of template `flags_extra`
  into map-owned creatures; until then this gate is only as complete as the data path feeding it.
  Remaining `CanStartAttack` fidelity still includes faction/neutrality as first-class data, target
  attackability, z-distance, gray aggro, and LOS.
- 2026-05-30 — Runtime respawn metadata preservation `#NEXT.RUNTIME.L3.023`: map-owned
  `PendingRespawn` now preserves creature `flags_extra` and restores it through
  `world_creature_from_pending_respawn_like_cpp`, so C++ gates such as `IsCivilian()` do not vanish
  after global lifecycle despawn/respawn. The DB spawn SELECT still needs a separate audited slice
  to feed template `flags_extra` into the simple runtime loader.
- 2026-05-30 — Runtime spawn `flags_extra` propagation `#NEXT.RUNTIME.L3.024`: the simple
  `SEL_CREATURES_IN_RANGE` loader now reads `creature_template.flags_extra` and passes it into
  `register_world_creature_with_flags_extra_like_cpp`, so represented runtime creatures created from
  visible DB spawns retain C++ template gates such as `CREATURE_FLAG_EXTRA_CIVILIAN`. This is still
  not full faction/LOS/gray-aggro parity; it only closes the data path needed by the civilian gate.
- 2026-05-30 — Runtime creature neutral-faction aggro radius `#NEXT.RUNTIME.L3.025`: replaced the
  simple loader's `faction == 35` no-aggro shortcut with a C++-shaped
  `WorldObject::IsNeutralToAll` helper using `FactionTemplateStore` plus `FactionStore`
  reputation-index semantics. The previous faction-35 behavior remains only as transitional
  compatibility when DB2 faction stores are absent. This closes the direct neutral-to-all gate for
  DB-backed runtime spawns; full `_IsTargetAcceptable`, hostile reaction, z-distance, gray aggro,
  and LOS parity remain open.
- 2026-05-30 — Runtime creature-aggro living-target prefilter `#NEXT.RUNTIME.L3.026`: the
  world-server snapshot that feeds the global legacy creature aggro scan now requires player
  candidates to be both `is_in_world` and `is_alive`. C++ `_IsTargetAcceptable` rejects
  `UNIT_STATE_DIED` targets (except feign-death detection branches not represented by this
  player-registry snapshot), so this removes a false aggro candidate class before the map-owned
  scan runs. Remaining targetability/hostility, z-distance, gray aggro, and LOS parity remain open.
- 2026-05-30 — Runtime creature-aggro live-victim delivery gate `#NEXT.RUNTIME.L3.027`: the
  `CreatureAttackStartLikeCpp` delivery rail now rechecks `is_alive` on the victim registry entry
  before enqueueing the session command. This closes the small stale-window where a player could be
  alive during the candidate snapshot but dead before delivery; C++ would no longer accept that
  target through `_IsTargetAcceptable`.
- 2026-05-30 — Runtime creature-aggro dead-victim session guard `#NEXT.RUNTIME.L3.028`: the
  victim-session handler for `CreatureAttackStartLikeCpp` now rejects commands if the represented
  player is no longer alive before mutating combat state or sending `AttackStart`. This mirrors the
  same C++ `_IsTargetAcceptable` dead-target rejection at the final command-consumption boundary.
- 2026-05-30 — Runtime creature-aggro NoGrayAggro gate `#NEXT.RUNTIME.L3.029`: the global legacy
  aggro scan now receives player `level` plus the session-published C++
  `Trinity::XP::GetGrayLevel` snapshot and applies `Creature::CheckNoGrayAggroConfig` using
  `CONFIG_NO_GRAY_AGGRO_ABOVE/BELOW` from the loaded world config. This keeps script-adjusted
  gray-level state on the session side instead of recomputing an incomplete value in the map driver.
  Remaining `CanStartAttack` fidelity gaps: full targetability/hostility, z-distance/accessibility,
  and LOS.
- 2026-05-30 — Runtime creature-aggro player targetability gate `#NEXT.RUNTIME.L3.030`: the player
  registry now publishes `unit_flags`, `unit_state`, and `is_game_master` snapshots, and the global
  legacy aggro scan rejects C++-untargetable player candidates before NoGrayAggro/radius engagement.
  Covered C++ anchors: `Unit::isTargetableForAttack(false)` for non-attackable/uninteractible/GM
  targets, `Creature::_IsTargetAcceptable` for `UNIT_STATE_DIED`, and
  `WorldObject::IsValidAttackTarget` for untargetable/taxi/immune-to-NPC player flags. Remaining
  `CanStartAttack` fidelity gaps: full hostility/reputation relation, z-distance/accessibility, and
  LOS.
- 2026-05-30 — Runtime creature-aggro relation snapshot wiring `#NEXT.RUNTIME.L3.031a`: the live
  server now loads `FactionTemplate.db2` into `SessionResources`, sessions receive the
  `FactionTemplateStore`, and `PlayerRegistry` publishes player faction-template id, reputation
  flags, forced-reputation ids, forced-rank values, `UNIT_FLAG2_IGNORE_REPUTATION`, and
  contested-PvP state. This is the data prerequisite for porting the C++
  `_IsTargetAcceptable`/`WorldObject::IsValidAttackTarget` hostility/reputation gates into the
  global legacy aggro scan. No aggro behavior is flipped in this slice. Remaining `CanStartAttack`
  fidelity gaps: applying the hostility/reputation gate, z-distance/accessibility, and LOS.
- 2026-05-30 — Runtime creature-aggro hostility/reputation gate `#NEXT.RUNTIME.L3.031b`: the global
  legacy aggro scan now consumes those relation snapshots plus the loaded `FactionTemplateStore` /
  `FactionStore` and rejects friendly templates, non-`AtWar` reputation states, and represented
  non-hostile reputation ranks before mutating creature combat state. Unknown/missing relation data
  is counted with `hostility_unrepresented` and rejected as neutral rather than allowed to fall
  through to radius aggro, matching the C++ no-template `REP_NEUTRAL` path. It reuses
  `FactionTemplateEntry` / `FactionEntry` helpers instead of duplicating DB2 relation semantics.
  Covered C++ anchors: `Creature::_IsTargetAcceptable`,
  `WorldObject::IsValidAttackTarget`, and `WorldObject::GetReactionTo` for the NPC-vs-player
  hostility/reputation path, including forced reaction ranks and `UNIT_FLAG2_IGNORE_REPUTATION`.
  Remaining `CanStartAttack` fidelity gaps: z-distance/accessibility, visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro z-distance gate `#NEXT.RUNTIME.L3.031c`: represented the
  vertical gate from `Creature::CanStartAttack` before radius engagement in both session-owned and
  global legacy aggro paths. Rust now snapshots player combat reach into `PlayerBroadcastInfo`,
  carries it through `LegacyCreatureAggroCandidateLikeCpp`, and applies the C++ formula
  `max(0, abs(dz) - ownCombatReach - targetCombatReach) <= CREATURE_Z_ATTACK_RANGE(3) +
  m_CombatDistance`. Covered C++ anchors: `Creature.h` `CREATURE_Z_ATTACK_RANGE`,
  `Object::GetDistanceZ`, and `Creature::CanStartAttack`. Remaining `CanStartAttack` fidelity gaps:
  represented `CanFly()` exemption, accessibility/visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro `CanFly()` z-exemption `#NEXT.RUNTIME.L3.031d`: represented
  the C++ `Creature::CanFly()` side of the z-distance branch for template/spawn movement data. Rust
  now loads `creature_template_movement.Flight`, applies `creature_movement_override.Flight` where a
  concrete spawn is available, normalizes invalid flight values to `None`, carries the value through
  loaded-grid/runtime registration/respawn, and bypasses the aggro z-distance reject when
  `Flight != None` (`DisableGravity` and `CanFly`). Covered C++ anchors:
  `Creature.h::CanFly`, `CreatureData.h::CreatureMovementData::IsFlightAllowed`,
  `ObjectMgr::LoadCreatureTemplates`, `ObjectMgr::LoadCreatureMovementOverrides`, and
  `Creature::CanStartAttack`. Remaining `CanStartAttack` fidelity gaps: dynamic `IsFlying()`
  movement flags for creature runtime state, accessibility/visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro dynamic flying half `#NEXT.RUNTIME.L3.031e`: represented the
  second half of C++ `Creature::CanFly()` by adding creature runtime movement flags and making
  `Unit::IsFlying()` true only for `MOVEMENTFLAG_FLYING | MOVEMENTFLAG_DISABLE_GRAVITY`, not bare
  `MOVEMENTFLAG_CAN_FLY`. The z-distance reject now matches the C++ predicate
  `GetMovementTemplate().IsFlightAllowed() || IsFlying()` for represented data. Covered C++ anchors:
  `Creature.h::CanFly`, `Unit.h::IsFlying`, `Unit::SetDisableGravity`, and `Unit::SetCanFly`.
  Remaining `CanStartAttack` fidelity gaps: producers for all dynamic creature movement flag
  transitions, accessibility/visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro home leash `#NEXT.RUNTIME.L3.031f`: represented the
  `CanCreatureAttack` home-distance leash for global legacy aggro. Rust now rejects candidates
  beyond `min(GetMap()->GetVisibilityRange(), SIZE_OF_GRID_CELL * 2) + combat reaches` from the
  creature home position before entering combat. It also preserves the C++ distinction that
  `GetMovementTemplate().IsFlightAllowed()` uses 2D home distance, while a dynamically flying
  creature with no flight template still uses 3D home distance. Covered C++ anchors:
  `Creature::CanCreatureAttack`, `Map::GetVisibilityRange`, and `GridDefines.h::SIZE_OF_GRID_CELL`.
  Remaining `CanStartAttack` fidelity gaps: exact map visibility range source for every map,
  accessibility/visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro in-flight targetability gate `#NEXT.RUNTIME.L3.031g`:
  represented the C++ `UNIT_STATE_UNATTACKABLE` rejection for player candidates in the global
  legacy aggro scan. In Trinity 3.3.5 `UNIT_STATE_UNATTACKABLE` is `UNIT_STATE_IN_FLIGHT`, so Rust
  now rejects in-flight players before gray-aggro/radius engagement, alongside the existing dead,
  GM, non-attackable, taxi, and NPC-immune targetability gates. Covered C++ anchors:
  `WorldObject::IsValidAttackTarget`, `Unit::isTargetableForAttack(false)`, and
  `Unit.h::UNIT_STATE_UNATTACKABLE`. Remaining `CanStartAttack` fidelity gaps: exact map visibility
  range source for every map, accessibility/visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro map visibility range source `#NEXT.RUNTIME.L3.031h`:
  the legacy global aggro home leash now uses the C++ visibility-distance categories instead of a
  hardcoded `100.0`: continents, instances, battlegrounds, and arenas are loaded from
  `Visibility.Distance.*`, clamped like `World.cpp`, and selected via `Map.db2` instance type before
  applying `min(GetMap()->GetVisibilityRange(), SIZE_OF_GRID_CELL * 2)`. This fixes the too-short
  home leash on represented BG/arena/instance map entries. Covered C++ anchors:
  `Map::GetVisibilityRange`, `Map::InitVisibilityDistance`, `InstanceMap::InitVisibilityDistance`,
  `BattlegroundMap::InitVisibilityDistance`, and `World.cpp` visibility config loading. Remaining
  gaps: the canonical `wow_map::Map::visible_distance` constructor still defaults to `100.0`
  outside this legacy aggro config path; `CanCreatureAttack` dungeon/owner/taunt bypasses,
  accessibility/visibility/detection, and LOS remain separate runtime fidelity work.
- 2026-05-30 — Runtime creature-aggro dungeon leash bypass `#NEXT.RUNTIME.L3.031i`: represented the
  C++ `Creature::CanCreatureAttack` branch where non-player-owned creatures on dungeon maps skip the
  home-distance leash. Rust selects dungeon maps from `Map.db2` via `MapEntry::is_dungeon()` and lets
  those represented unowned legacy world creatures continue to radius aggro without a home-range
  rejection. Covered C++ anchors: `Creature::CanCreatureAttack`, `Map::IsDungeon`, and
  `MapEntry::IsDungeon`. Remaining gaps: represented player charmer/owner state for creatures,
  recent-damage/taunt bypass, accessibility, AI-specific attack gates, evade states,
  visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro owner leash guard `#NEXT.RUNTIME.L3.031j`: represented the C++
  `Creature::CanCreatureAttack` distinction between non-player-owned creatures and creatures with a
  player charmer/owner. The dungeon leash bypass now applies only to creatures whose
  `GetCharmerOrOwnerGUID().IsPlayer()` equivalent is false; if any charmer/owner exists and this
  transitional global scan lacks the owner position needed for C++ `victim->IsWithinDist(owner,
  dist)`, Rust fails closed and counts `owner_position_unrepresented` instead of falling back to the
  home position. Remaining gaps: owner-position routing for player/non-player owned creatures,
  recent-damage/taunt bypass, accessibility, AI-specific attack gates, evade states,
  visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro recent-damage/taunt leash bypass
  `#NEXT.RUNTIME.L3.031k`: represented the C++ non-player-owned, non-world-boss
  `CanCreatureAttack` branch that skips home leash when `_lastDamagedTime > GameTime::GetGameTime()`
  or `HasAuraType(SPELL_AURA_MOD_TAUNT)`. Rust now stores creature AI `last_damaged_time` as a
  C++-style absolute `GameTime` seconds expiry (`now + MAX_AGGRO_RESET_TIME`), records it only for
  non-lethal positive damage to non-player-owned creatures, clears it on combat reset/respawn,
  exposes `Creature::is_world_boss_like_cpp()` from `CreatureTypeFlags::BOSS_MOB`, and adds
  `SPELL_AURA_MOD_TAUNT = 11`. Remaining gaps: owner-position routing for owned creatures, exact
  damage-type exclusions for DoTs/damage shields, accessibility, AI-specific attack gates, evade
  states, visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro attacker evade gate `#NEXT.RUNTIME.L3.031l`: represented the
  C++ `Creature::CanCreatureAttack` branch that rejects when the attacking creature
  `IsInEvadeMode()`. The global legacy aggro scan now checks `Creature::is_in_evade_mode_like_cpp()`
  before leash/NoGray/aggro start and counts `attacker_evade_rejections`. Remaining gaps:
  victim-creature evade for non-player aggro targets, accessibility, AI-specific attack gates,
  visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro player-owner leash `#NEXT.RUNTIME.L3.031m`: represented the
  C++ `GetCharmerOrOwner()` leash center for the safe subset where the owner is a player already
  present in the active aggro candidate snapshots on the same map/instance. The check uses 3D
  distance from victim to owner and includes victim + owner combat reach, matching
  `WorldObject::IsWithinDist` defaults, including the strict `< dist * dist` edge from
  `Position::IsInDist`. Missing, non-player, or cross-map owners still fail closed as
  `OwnerPositionUnrepresented`. Remaining gaps: victim-creature evade for non-player aggro targets,
  non-player owner leash centers, accessibility, AI-specific attack gates, visibility/detection,
  and LOS.
- 2026-05-30 — Runtime creature-aggro home leash strict edge `#NEXT.RUNTIME.L3.031n`: aligned the
  normal home-position leash branch with C++ `Position::IsInDist` / `IsInDist2d` strict `< dist *
  dist` semantics for both ground 3D and flight-template 2D checks. This closes the exact-edge case
  where Rust's generic `Position::is_within_dist` helper was inclusive. Remaining gaps:
  victim-creature evade for non-player aggro targets, non-player owner leash centers,
  accessibility, AI-specific attack gates, visibility/detection, and LOS.
- 2026-05-30 — Runtime creature-aggro accessibility gate `#NEXT.RUNTIME.L3.031o`: represented C++
  `Unit::isInAccessiblePlaceFor(Creature const*)` before radius engagement. Rust now carries
  creature `Ground`/`Swim` movement-template state alongside `Flight`, publishes player liquid status
  through `PlayerRegistry`, and rejects water targets unless the creature can enter water, or land
  targets unless the creature can walk or fly. Covered C++ anchors: `CreatureMovementData`,
  `Creature::CanWalk`, `Creature::CanEnterWater`, `Unit::CanSwim`, and
  `Unit::isInAccessiblePlaceFor`. Remaining gaps: victim-creature evade for non-player aggro
  targets, non-player owner leash centers, AI-specific attack gates, visibility/detection, and LOS.
- 2026-05-31 — Runtime creature-aggro represented detection gate `#NEXT.RUNTIME.L3.031p`: wired the
  global legacy aggro scan through the represented `Unit::CanSeeOrDetect` port before hostility,
  leash, NoGrayAggro, and radius engagement. Candidate snapshots now carry canonical player
  `PhaseShift` and `UnitVisibilityDetectionStateLikeCpp` plus an explicit represented bit; the
  global bridge hydrates them from the canonical map manager before taking the legacy map lock. If
  the canonical visibility snapshot is missing, Rust fails closed and counts
  `visibility_unrepresented`. The scan builds transient in-world seer and target units on the active
  `(map_id, instance_id)` and rejects invisible/stealthed/phase-hidden represented players unless
  the creature can detect them. Covered C++ anchors:
  `CreatureUnitRelocationWorker`, `WorldObject::CanSeeOrDetect`, and
  `WorldObject::IsValidAttackTarget`. Remaining gaps: real VMAP-backed LOS, AI-specific
  `CanAIAttack`, sightless/alert behavior, victim-creature evade for non-player aggro targets, and
  non-player owner/victim creature edge cases.
- 2026-05-31 — Runtime creature-aggro sightless gate `#NEXT.RUNTIME.L3.031q`: represented the
  `CreatureUnitRelocationWorker` early return when the creature has `UNIT_STATE_SIGHTLESS`
  (`UNIT_STATE_LOST_CONTROL | UNIT_STATE_EVADE`). Rust now exposes the composed
  `UnitState::SIGHTLESS` constant and the map-owned global aggro scan skips sightless creatures
  before candidate-specific visibility/hostility/leash/radius work, counting
  `sightless_creatures_skipped`. C++ anchors: `GridNotifiers.cpp:124-132` and `Unit.h:292`.
  Remaining gaps: alert/prowl behavior, real VMAP-backed LOS, AI-specific `CanAIAttack`,
  victim-creature evade for non-player aggro targets, and non-player owner/victim creature edge
  cases.
- 2026-05-31 — Runtime creature-aggro stealth alert/distract `#NEXT.RUNTIME.L3.031r`: represented
  the `CreatureUnitRelocationWorker` fallback branch for stealthed/prowling players that fail the
  normal `CanSeeOrDetect(..., checkAlert=false)` aggro visibility gate but pass
  `CanSeeOrDetect(..., checkAlert=true)`. Rust now keeps those players out of combat start, applies
  the C++ `CreatureAI::TriggerAlert` gates (non-engaged, non-controlled, non-civilian, non-passive,
  hostile/targetable player), and starts the map-owned `MoveDistract(5000ms, angle)` movement side
  effect. The `SendAIReaction(AI_REACTION_ALERT)` sound packet remains an explicit packet/fanout gap.
  C++ anchors: `GridNotifiers.cpp:127-132` and `CreatureAI.cpp:140-159`. Alert reaction packet
  delivery is closed by `#NEXT.RUNTIME.L3.031s`; remaining gaps: real VMAP-backed LOS, AI-specific
  `CanAIAttack`, victim-creature evade for non-player aggro targets, and non-player owner/victim
  creature edge cases.
- 2026-05-31 — Runtime creature-aggro AI reaction packet/fanout `#NEXT.RUNTIME.L3.031s`: closed
  the explicit `SendAIReaction(AI_REACTION_ALERT)` packet gap for the represented stealth-alert
  branch. C++ `WorldPackets::Combat::AIReaction::Write` writes `UnitGUID` then `Reaction`, and
  `Creature::SendAIReaction` sends it with `SendMessageToSet(packet.Write(), true)`. Rust now has
  `wow_packet::packets::combat::AIReaction` for `SMSG_AI_REACTION`, returns alert packets as a
  `RuntimePlan` from the global legacy aggro scan, addresses them with `MapBroadcastVisible`, and
  lets the existing `SendIfVisibleLikeCpp` rail apply the per-session HaveAtClient gate outside map
  locks. C++ anchors: `CombatPackets.cpp:89-94`, `CombatPackets.h:125-134`, and
  `Creature.cpp:2506-2515`. Remaining gaps: real VMAP-backed LOS, AI-specific `CanAIAttack`,
  victim-creature evade for non-player aggro targets, non-player owner/victim creature edge cases,
  and live-client validation of the alert reaction sound/visual.
- 2026-05-31 — Runtime creature AI identity data `#NEXT.RUNTIME.L3.031t`: carried template
  `AIName` and `ScriptName` through the live creature lifecycle path as a prerequisite for real
  `CanAIAttack` / AI selector work. C++ `ObjectMgr::LoadCreatureTemplates` selects `AIName` and
  `ScriptName`; `LoadCreatureTemplate` stores `AIName` directly and resolves `ScriptName` via
  `GetScriptId` (`ObjectMgr.cpp:349-400`, `ObjectMgr.cpp:430-482`). Rust now loads both columns in
  `CreatureTemplateLifecycleStoreLikeCpp`, propagates them through `ResolvedCreatureTemplateLikeCpp`
  and `CreatureTemplateLifecycleRecord`, and stores them in `CreatureLifecycleMetadata`. This slice
  deliberately does not instantiate AI factories, SmartAI, BossAI boundary checks, TurretAI range
  gates, or `CanAIAttack`; those remain behavior slices now backed by real template identity data.
- 2026-05-31 — Runtime respawn AI identity continuity `#NEXT.RUNTIME.L3.031u`: preserved that
  identity across the map-owned respawn queue. `PendingRespawn` now captures `ai_name` and
  `script_name` from the despawned `WorldCreature`, and
  `world_creature_from_pending_respawn_like_cpp` restores them into `CreatureLifecycleMetadata`.
  This is still data continuity only; AI factory selection and `CanAIAttack` behavior remain open.
- 2026-05-31 — Runtime creature AI selector evidence `#NEXT.RUNTIME.L3.031v`: represented the pure
  C++ `FactorySelector::SelectAI` decision in `wow-ai` without changing live behavior. Rust now
  models the pet override, script-created AI before `AIName`, registered `AIName` lookup
  (`SmartAI` included), and fallback permit selection with C++ `ObjectRegistry` `std::map` /
  `std::max_element` tie behavior. This matters for cases like trigger-without-spell selecting
  `NullCreatureAI`, and equal permit values being resolved by lexicographic AIName order rather
  than hand-written priority. C++ anchors: `CreatureAISelector.cpp:59-101`,
  `CreatureAIRegistry.cpp:34-54`, `ObjectRegistry.h:31,70`, `CreatureAI.h:44-49`, and stock
  permit implementations in `CoreAI`. This remains selector evidence only: no AI object
  instantiation, script/SmartAI execution, `CanAIAttack` wiring, or live creature behavior change.
- 2026-05-31 — Runtime creature AI attack / LOS dispatch evidence `#NEXT.RUNTIME.L3.031w`:
  represented the pure `AI()->CanAIAttack(target)` decision and whether a selected AI reaches the
  base `CreatureAI::MoveInLineOfSight` auto-aggro path in `wow-ai`, without wiring it to global
  aggro yet. C++ `Creature::CanCreatureAttack` calls `AI()->CanAIAttack` after target validity and
  accessibility; the default inherited `UnitAI::CanAIAttack` returns true, while represented
  overrides are `TurretAI` range/min-range and script-proven `BossAI` boundary membership. The
  relocation path dispatches `MoveInLineOfSight_Safe`, so AIs with empty LOS overrides must be
  suppressed before base auto-aggro. C++ anchors: `GridNotifiers.cpp:122-130`,
  `CreatureAI.cpp:113-127`, `Creature.cpp:2671-2683`, `UnitAI.h:57`, `CombatAI.cpp:203-210`,
  `ScriptedCreature.cpp:631-634`, and empty LOS overrides in `PassiveAI.h`, `ReactorAI.h`,
  `PetAI.h`, `TotemAI.h`, `CombatAI.h` (`VehicleAI`), and `ScheduledChangeAI.h`. Wiring remains
  blocked on clean runtime facts for script-creatable AI registry, true unit-type masks for
  totem/guardian/controlable guardian, spell range hydration for TurretAI, and boss boundary data.
- 2026-05-31 — Runtime creature unit-type mask fact `#NEXT.RUNTIME.L3.031x`: added represented
  C++ `UnitTypeMask` storage/helpers to `Creature` so later selector wiring can distinguish
  `IsTotem`, `IsGuardian`, `HasUnitTypeMask(UNIT_MASK_CONTROLABLE_GUARDIAN)`, and `IsVehicle`
  without guessing from template type. Successful represented `CreateVehicleKit` now sets
  `UNIT_MASK_VEHICLE`; a missing DB2 vehicle entry keeps the mask clear, matching the existing
  bounded seam. C++ anchors: `Unit.h:334-346`, `Unit.h:723-730`, `Totem.cpp:29-31`,
  `Pet.cpp:53-59`, `TemporarySummon.cpp:513-516`, and `Unit.cpp:11306-11326`. This is still a
  data prerequisite only; summon/totem/guardian spell paths and live selector/aggro wiring remain
  open.
- 2026-05-31 — Runtime global aggro AI selector gates `#NEXT.RUNTIME.L3.031y`: wired the
  represented selector/LOS/`CanAIAttack` decisions into the experimental global legacy aggro scan.
  The scan now derives selector facts from `WorldCreature`, applies C++ empty `MoveInLineOfSight`
  suppression before base auto-aggro, fails closed/counts non-pet `ScriptName` because the
  script-creatable AI registry is not represented yet, and fails closed/counts `TurretAI` until
  spell range hydration can provide its min/max range gate. C++ anchors:
  `GridNotifiers.cpp:122-130`, `CreatureAI.cpp:113-127`, `Creature.cpp:2095-2128`,
  `Creature.cpp:2671-2684`, `CreatureAISelector.cpp:83-101`, `CombatAI.cpp:203-210`, and empty LOS
  overrides in the stock AI headers. This is a real runtime fidelity improvement but still only in
  the gated `GlobalLegacy` owner path; script AI factories, SmartAI execution, Turret range
  hydration, and BossAI boundary data remain open.
- 2026-05-31 — Runtime global aggro TurretAI spell range hydration `#NEXT.RUNTIME.L3.031z`: closed
  the bounded `TurretAI` range gap in the gated global legacy aggro scan. C++ `TurretAI::TurretAI`
  resolves `m_spells[0]` through spell info, stores `_minimumRange = GetMinRange(false)` and
  `m_CombatDistance = GetMaxRange(false)`, and `CanAIAttack` rejects targets outside max range or
  inside a non-zero minimum range (`CombatAI.cpp:190-210`). Rust now passes the real
  `SpellMiscStore` and `SpellRangeStore` through `LegacyCreatureAggroConfigLikeCpp`, wires those
  stores from `world-server`, resolves `creature.spells()[0] -> SpellMisc.range_index ->
  SpellRange`, and feeds the represented `creature_ai_can_attack_like_cpp` TurretAI inputs. Missing
  spell/range data still fails closed and counts `ai_can_attack_unrepresented`. This removes the
  previous blanket TurretAI-unrepresented behavior only for the experimental `GlobalLegacy` owner
  path; script AI factories, SmartAI execution, BossAI boundary data, and real VMAP LOS remain open.
- 2026-05-31 — Runtime global aggro non-player owner leash center `#NEXT.RUNTIME.L3.031aa`: closed
  the bounded owner-position gap for map-owned creature owners. C++ `Creature::CanCreatureAttack`
  skips the home-position leash center when `GetCharmerOrOwner()` exists and instead checks
  `victim->IsWithinDist(owner, dist)`, with `WorldObject::IsWithinDist` adding both combat reaches
  (`Creature.cpp:2671-2721`, `Object.cpp:1062-1078`, `Object.cpp:1148-1151`). Rust now snapshots
  active map-owned creatures on the same `(map_id, instance_id)` alongside player candidates before
  the global aggro scan mutates creatures, so creature-owned creatures can use their non-player
  owner as the leash center. Missing/cross-map owners still fail closed as
  `owner_position_unrepresented`. This only affects the experimental `GlobalLegacy` owner path;
  victim-creature aggro targets, full non-player owner routing outside the active map, BossAI
  boundary data, and real VMAP LOS remain open.
- 2026-05-30 — Runtime loop smoke `#NEXT.RUNTIME.L3.032`: added 4B.2a coverage for the real
  experimental production loop wrapper `spawn_legacy_creature_runtime_update_loop_like_cpp`. The
  test flips the legacy owner to `GlobalLegacy`, runs the loop with a 1ms interval, observes a real
  `SendIfVisibleLikeCpp` `OnMonsterMove` command through `PlayerRegistry`, verifies canonical
  creature sync, and aborts the forever task. Production remains default-off; startup logs include
  the map-update interval when `RustyCore.LegacyCreatureGlobalRuntime` is enabled. This advances
  manual-test readiness but does not mark 4B.2 complete until the server is actually run with a
  client.

## References

- `crates/wow-world/src/session.rs` — `tick_creatures_sync`, `tick_combat_sync`, creature wrappers; `client_visible_guids_like_cpp` (HashSet, :2312); `process_represented_session_commands_like_cpp` (:12004).
- `crates/wow-network/src/player_registry.rs` — `SessionCommand` enum (:19), `PlayerBroadcastInfo`/`PlayerRegistry`.
- `crates/wow-map/src/manager.rs` — `MapManager::update` / `ManagedMap::update` (mirrors `Map::Update`).
- `crates/world-server/src/main.rs` — both managers + `spawn_canonical_map_update_loop`.
- C++: `World.cpp:2748` (`sMapMgr->Update`), `Map.cpp:666` (`Map::Update` phase order).
- `docs/migration/honest-progress-audit.md`, `crates/wow-world/_attic/README.md` (big-bang lesson).
