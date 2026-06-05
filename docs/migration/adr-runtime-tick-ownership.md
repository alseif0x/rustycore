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
- 2026-05-31 — Runtime global creature melee range gate `#NEXT.RUNTIME.L3.031ab`: closed the
  bounded range gap in the experimental global legacy creature melee driver. C++ `Unit::DoMeleeAttackIfReady`
  computes an auto-attack error and rejects base/offhand swings before `AttackerStateUpdate` when
  `!IsWithinMeleeRange(victim)`, while `Unit::IsWithinMeleeRangeAt` uses
  `max(attackerCombatReach + victimCombatReach + 4/3, NOMINAL_MELEE_RANGE)`
  (`Unit.cpp:2085-2155`, `Unit.cpp:649-667`). Rust now carries attacker position/combat reach into
  pending global creature swings, checks the canonical victim position/combat reach before mutating
  health, records `melee_range_rejections`, and suppresses victim delivery for out-of-range swings.
  This only affects the gated `GlobalLegacy` owner path. Facing/boundary-radius, LOS,
  non-player victims, and full C++ timer retry semantics remain open.
- 2026-05-31 — Runtime global creature melee facing/boundary gate `#NEXT.RUNTIME.L3.031ac`: closed
  the represented bad-facing gap in the experimental global legacy creature melee driver. C++
  `Unit::DoMeleeAttackIfReady` rejects with `AttackSwingErr::BadFacing` when the victim is outside
  `IsWithinBoundaryRadius` and outside `HasInArc(2*pi/3)`, while `IsWithinBoundaryRadius` delegates
  to `WorldObject::IsWithinDist` and includes attacker/target combat reach
  (`Unit.cpp:2085-2155`, `Unit.cpp:672-678`, `Object.cpp:1057-1083`). Rust now checks the canonical
  victim position, bounding radius, and combat reach before health mutation, records
  `melee_facing_rejections`, and suppresses victim delivery for bad-facing swings. The shared
  boundary helper used by session combat was corrected to include both combat reaches. This still
  leaves LOS, non-player victims, offhand/extra attacks, and exact attack-timer retry semantics open.
- 2026-05-31 — Runtime global creature melee retry timer `#NEXT.RUNTIME.L3.031ad`: closed the
  represented auto-attack error retry gap in the experimental global legacy creature melee driver.
  C++ `Unit::DoMeleeAttackIfReady` sets the base attack timer to `100` when the ready base attack
  fails the auto-attack gate (`NotInRange`/`BadFacing`) and resets the base timer only after the
  successful `AttackerStateUpdate` path (`Unit.cpp:2085-2155`). Rust now records failed global
  creature swings with a 100ms retry cooldown and restores the represented base attack interval
  from `CreatureCreateData::base_attack_time` on successful swings. This prevents per-global-tick
  retry spam while keeping the current bounded AI ownership timer model. Remaining gaps: LOS,
  non-player victims, offhand/extra attacks, and full unification with canonical `Unit` attack
  timers.
- 2026-05-31 — Runtime global creature melee attacker-state gate `#NEXT.RUNTIME.L3.031ae`: closed
  the represented `AttackerStateUpdate` attacker-side rejection gap in the experimental global
  legacy creature melee driver. C++ calls `AttackerStateUpdate` only after range/facing succeeds,
  and that function returns before damage for `UNIT_FLAG_PACIFIED`, `UNIT_STATE_CANNOT_AUTOATTACK`
  when not extra, or `SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES`; the caller still resets the
  base attack timer after the call (`Unit.cpp:2085-2170`). Rust now snapshots
  `Unit::can_attacker_state_update_melee_like_cpp(false)`, suppresses canonical player health
  mutation and victim command delivery when it fails, records `attacker_state_rejections`, and
  treats the swing as consumed rather than a 100ms auto-attack-error retry. Remaining gaps: LOS,
  non-player victims, offhand/extra attacks, exact `CanMelee`/casting/channel gates, and full
  canonical `Unit` attack-timer unification.
- 2026-05-31 — Runtime global creature melee precondition gates `#NEXT.RUNTIME.L3.031af`: closed
  the represented `DoMeleeAttackIfReady` charging/casting precondition gap before ready swings in
  the experimental global legacy creature melee driver. C++ returns before victim lookup and before
  `isAttackReady` while `UNIT_STATE_CHARGING`, and while `UNIT_STATE_CASTING` unless the current
  channeled spell has `SPELL_ATTR5_ALLOW_ACTIONS_DURING_CHANNEL` (`Unit.cpp:2085-2101`). Rust now
  rejects those represented states before queuing a pending swing, records
  `melee_precondition_rejections`, leaves the represented swing timer unchanged, and allows
  channels explicitly marked as allowing actions. `UNIT_STATE_MELEE_ATTACKING` and
  `Creature::CanMelee` remain unrepresented in this transitional global legacy AI path; LOS,
  non-player victims, offhand/extra attacks, and full canonical `Unit` attack timers remain open.
- 2026-05-31 — Runtime global creature melee `CanMelee` gate `#NEXT.RUNTIME.L3.031ag`: closed
  the represented C++ `Creature::CanMelee` precondition gap in the experimental global legacy
  creature melee driver. C++ returns before victim lookup and `isAttackReady` when
  `CREATURE_STATIC_FLAG_NO_MELEE` is set (`Unit.cpp:2085-2101`, `Creature.h:180-181`,
  `CreatureData.h:58`). Rust now carries `creature_template_difficulty.StaticFlags1..8` through
  loaded-grid template resolution into `CreatureTemplateLifecycleRecord` and
  `CreatureLifecycleMetadata`, preserves those flags in map-owned respawn snapshots, exposes
  `Creature::can_melee_like_cpp()`, and rejects `CreatureStaticFlags::NO_MELEE_FLEE` before queuing
  a ready global creature swing. Remaining gaps: LOS, non-player victims, offhand/extra attacks,
  `UNIT_STATE_MELEE_ATTACKING` unification, and full canonical `Unit` attack timers.
- 2026-05-31 — Runtime global creature melee attacking-interrupt aura removal
  `#NEXT.RUNTIME.L3.031ah`: closed the represented C++ `Unit::AttackerStateUpdate` side effect that
  removes attacking-interruptible auras from the attacker on confirmed melee attacks. C++ calls
  `RemoveAurasWithInterruptFlags(SpellAuraInterruptFlags::Attacking)` after the attacker-state/LOS
  gates and before damage (`Unit.cpp:2168-2173`). Rust now removes those represented auras from the
  legacy creature attacker for successful `GlobalLegacy` canonical player hits, reports
  `attacking_interrupt_auras_removed`, and preserves unrelated interrupt flags. The removal happens
  after the canonical health mutation but before swing commit to preserve Rust's legacy/canonical
  lock-ordering invariant; exact pre-damage ordering belongs to the later canonical `Unit` ownership
  unification. Remaining gaps: LOS, non-player victims, offhand/extra attacks,
  `UNIT_STATE_MELEE_ATTACKING` unification, and full canonical `Unit` attack timers.
- 2026-05-31 — Runtime global creature melee represented LOS hook `#NEXT.RUNTIME.L3.031ai`: wired
  the experimental `GlobalLegacy` creature melee apply path to the existing represented
  `WorldObject::IsWithinLOSInMap` hook when the canonical attacker creature exists. C++ rejects
  base/offhand melee inside `Unit::AttackerStateUpdate` when `!IsWithinLOSInMap(victim)` after the
  earlier range/facing and attacker-state gates (`Unit.cpp:2168-2173`, `Object.cpp:1187-1210`).
  Rust now routes that check through `WorldObjectEnvironment`, counts `melee_los_rejections`, and
  consumes rejected swings like the C++ caller's `resetAttackTimer` after `AttackerStateUpdate`
  returns. This is intentionally not claimed as real LOS parity: `SharedCanonicalMapManager` remains
  fixed to `wow_map::MapManager`/`NoopTerrainGridLoader`, whose terrain LOS returns `true`; a
  focused helper test proves false-return delegation, while live false LOS awaits real/injectable
  canonical terrain/VMAP/dynamic-tree LOS. Remaining gaps: non-player victims, offhand/extra
  attacks, `UNIT_STATE_MELEE_ATTACKING` unification, full canonical `Unit` attack timers, and real
  VMAP/dynamic LOS.
- 2026-05-31 — Runtime global creature melee canonical Creature victims `#NEXT.RUNTIME.L3.031aj`:
  closed the player-only victim shortcut in the experimental `GlobalLegacy` creature melee driver
  for canonical Creature victims. C++ `Unit::DoMeleeAttackIfReady`/`AttackerStateUpdate` operates on
  a generic `Unit* victim`; it only branches for `Creature* victimCreature` to set fake-damage hit
  flags before sending the attack state update and applying `DealMeleeDamage` (`Creature.cpp:847`,
  `Unit.cpp:2103-2130`, `Unit.cpp:2211-2227`). Rust now allows creature GUID combat targets, mutates
  canonical creature health via `get_typed_creature_mut`, produces map-visible attack-state and Unit
  values-update events through `RuntimePlan`, and has world-server deliver those events through the
  existing `SendIfVisibleLikeCpp` rail. Player victims continue to use the explicit victim-session
  command because they must mirror local player health. This slice intentionally does not claim full
  `DealMeleeDamage`: Pet victims, absorbs/resists/procs/redirect, fake damage, death cleanup, loot,
  rewards, and exact threat/proc side effects remain open. No C++ bug was found in this area; the
  previous player-only filter was a Rust port gap.
- 2026-05-31 — Runtime global creature melee sparring/fake-damage `#NEXT.RUNTIME.L3.031ak`:
  added the represented `Creature::CalculateDamageForSparring` / `Creature::ShouldFakeDamageFrom`
  branch for canonical Creature victims. C++ first marks `HITINFO_FAKE_DAMAGE` before sending
  `AttackerStateUpdate`, then applies the actual sparring clamp inside `DealDamage`
  (`Unit.cpp:780`, `Unit.cpp:2219-2220`, `Creature.cpp:1726-1765`,
  `UnitDefines.h:468`). Rust now exposes `HIT_INFO_FAKE_DAMAGE`, lets `AttackerStateUpdate` carry
  explicit hit-info flags, clamps canonical creature health at the sparring threshold, and emits a
  fake-damage attack-state update when the victim is already at or below that threshold. Scope stays
  creature-vs-creature: pet victims, full player/charm ownership parity beyond represented control
  state, absorbs/resists/procs/redirect, death cleanup, loot, rewards, and exact threat/proc side
  effects remain open.
- 2026-05-31 — Runtime global creature melee canonical Creature death-state `#NEXT.RUNTIME.L3.031al`:
  closes the immediate health-zero-only death gap for canonical Creature victims. C++ `DealDamage`
  calls `Unit::Kill` when `health <= damageTaken`, and `Creature::setDeathState(JUST_DIED)` promotes
  creatures through corpse/respawn bookkeeping before ending in `CORPSE` (`Unit.cpp:942-1016`,
  `Unit.cpp:10457-10614`, `Creature.cpp:2193-2247`). Rust now routes lethal global creature melee
  through represented AI damage state, then calls `set_death_state_runtime(JustDied, game_time_secs)`
  instead of the existing `mark_ai_dead(now_ms)` shortcut so corpse/respawn times use the same seconds
  scale as C++. This is still not full `Unit::Kill`: rewards, loot, proc hooks, aura/combat cleanup,
  tapper/group handling, and pet notifications remain open.
- 2026-05-31 — Runtime global creature melee legacy Creature victim mirror `#NEXT.RUNTIME.L3.031am`:
  keeps the transient legacy+canonical split from creating two different creature states after a
  global creature-vs-creature hit. C++ has a single `Creature`/`Unit` object, so after
  `DealMeleeDamage` the victim's health/death state cannot diverge between runtimes
  (`Unit.cpp:942-1016`, `Unit.cpp:10457-10614`). Rust now mirrors successful canonical Creature
  victim hits back into the legacy `WorldCreature` when it exists on the same `(map, instance)`,
  including the `JustDied -> Corpse` transition for lethal hits. This is a convergence guard for the
  current dual-model runtime, not a substitute for the final single canonical source of truth.
- 2026-05-31 — Represented creature death CombatStop parity `#NEXT.RUNTIME.L3.031an`:
  closes a shared death-state gap found while auditing `Unit::Kill`. C++ `Unit::setDeathState`
  stores the new death state and, for non-alive states, calls `CombatStop()` before `JUST_DIED`
  side effects (`Unit.cpp:8527-8554`). Rust's represented `Creature::set_death_state_runtime`
  already cleared target/attacking and drove corpse/respawn bookkeeping, but left threat references
  and attacker sets alive. It now clears represented combat/threat/attackers during `JustDied`, so
  all represented creature death paths stop combat before death hooks. This is still partial:
  spell interrupts, vehicle/totem/controlled removal, aura-on-death removal, and full combat packet
  fanout remain open.
- 2026-05-31 — Represented creature death-state field normalization `#NEXT.RUNTIME.L3.031ao`:
  extends the same C++ `Unit::setDeathState(JUST_DIED)` anchor (`Unit.cpp:8552-8567`). Rust now
  clears represented defensive reactives and diminishing-return state, then normalizes health to 0,
  current display power to 0, emote state to `EMOTE_ONESHOT_NONE`, and stand state to
  `UNIT_STAND_STATE_STAND` during represented creature death. This deliberately stays within fields
  already represented by `wow-entities`; spell interruption, vehicle/totem/controlled cleanup,
  death-persistent aura filtering, movement `StopOnDeath`, and ZoneScript/InstanceScript callbacks
  remain separate gaps.
- 2026-05-31 — Represented creature death spell interrupt guard `#NEXT.RUNTIME.L3.031ap`:
  ports the guarded C++ shape `if (IsNonMeleeSpellCast(false)) InterruptNonMeleeSpells(false)`
  from `Unit::setDeathState` (`Unit.cpp:8538-8539`) for represented creature state. Rust now has a
  matching `Unit::is_non_melee_spell_cast_like_cpp` guard so a pure generic instant spell is not
  interrupted just because death cleanup runs, while real generic casts, channeled spells, and
  autorepeat spells follow the existing represented interrupt path. Packet sends, cooldown failure
  side effects, and full spell runtime semantics remain separate spell-system gaps.
- 2026-05-31 — Represented creature respawn corpse-flag cleanup `#NEXT.RUNTIME.L3.031aq`:
  ports the represented subset of C++ `Unit::setDeathState(JUST_RESPAWNED)` /
  `Creature::setDeathState(JUST_RESPAWNED)` (`Unit.cpp:8573-8574`, `Creature.cpp:2279-2284`).
  Rust now clears corpse dynamic flags (`Lootable`/`CanSkin` via `ReplaceAllDynamicFlags(0)`) and
  removes represented `SKINNABLE` / `IN_COMBAT` unit flags during `JustRespawned`. Full
  `ChooseCreatureFlags` template reload, unit flags2/3, melee damage school, addon reload, and
  default motion initialization remain separate respawn gaps.
- 2026-05-31 — Represented creature death active-state cleanup `#NEXT.RUNTIME.L3.031ar`:
  ports the represented subset of C++ `Creature::setDeathState(JUST_DIED)` `setActive(false)`
  (`Creature.cpp:2227`). Rust's death plan already carried `CreatureRuntimeAction::Deactivate`, but
  the represented `WorldObject.active` flag could remain true after `JustDied`; it is now cleared
  directly during `Creature::set_death_state_runtime`. Map active-list removal/fanout remains a
  runtime owner gap outside `wow-entities`.
- 2026-05-31 — Represented creature respawn erasable state cleanup `#NEXT.RUNTIME.L3.031as`:
  ports C++ `Creature::setDeathState(JUST_RESPAWNED)` `ClearUnitState(UNIT_STATE_ALL_ERASABLE)`
  (`Creature.cpp:2262`, `Unit.h:283-296`). Rust now defines the contrasted `UnitState::ALL_ERASABLE`
  mask (`ALL_STATE_SUPPORTED & !IGNORE_PATHFINDING`) and clears it during represented creature
  respawn, preserving `IGNORE_PATHFINDING` like C++.
- 2026-05-31 — Represented creature death live flag/mount cleanup `#NEXT.RUNTIME.L3.031at`:
  ports the live `UnitData` subset of C++ `Creature::setDeathState(JUST_DIED)`
  (`Creature.cpp:2227-2230`). Rust already carried represented runtime actions for clearing NPC
  flags and mount display, but the entity state itself could retain `NpcFlags[0]`, `NpcFlags[1]`,
  and `MountDisplayID` after death. `Creature::set_death_state_runtime(JustDied)` now applies
  `ReplaceAllNpcFlags(0)`, `ReplaceAllNpcFlags2(0)`, and `SetMountDisplayId(0)` to the represented
  `UnitData`. This deliberately does not clear `CreatureAiOwnershipState` template/identity fields:
  C++ clears the live unit update fields on death and reloads/choses template flags on respawn.
- 2026-05-31 — Represented creature death hover/gravity cleanup `#NEXT.RUNTIME.L3.031au`:
  ports the represented movement-flag subset of C++ `Creature::setDeathState(JUST_DIED)`
  (`Creature.cpp:2240-2245`) and the exact flag mutations inside `Unit::SetHover(false,false)` /
  `Unit::SetDisableGravity(false,false)` (`Unit.cpp:12580-12613`, `Unit.cpp:12793-12835`). Rust now
  removes represented `MOVEMENTFLAG_HOVER` and `MOVEMENTFLAG_DISABLE_GRAVITY` during creature
  death while preserving `CAN_FLY`/`FLYING`, which those C++ calls do not clear. The follow-up
  `MoveFall()` spline/fanout remains open because it needs real MotionMaster ground-height/runtime
  ownership rather than just entity-field mutation.
- 2026-05-31 — Represented creature death StopOnDeath wiring `#NEXT.RUNTIME.L3.031av`:
  ports the represented subset of C++ `Unit::setDeathState(JUST_DIED)` movement shutdown
  (`Unit.cpp:8554-8561`, `MotionMaster.cpp:548-566`, `Unit.cpp:9915-9931`, `Unit.cpp:622-625`).
  Rust now skips the movement shutdown for represented vehicle passengers, otherwise calls the
  existing `MotionSubsystem::stop_on_death()`, clears `UNIT_STATE_MOVING`, and finalizes the
  represented move spline when `StopOnDeath` succeeds. This remains entity-local: exact in-world
  `MotionMaster::Clear`/`MoveIdle` packet behavior, spline stop packets, and position update
  fanout remain runtime-owner work.
- 2026-05-31 — Represented creature death vehicle/control cleanup `#NEXT.RUNTIME.L3.031aw`:
  ports the represented local subset of C++ `Unit::setDeathState(non-alive)` cleanup order after
  spell interruption (`Unit.cpp:8534-8546`): `ExitVehicle()`, `UnsummonAllTotems()`,
  `RemoveAllControlled()`, then aura death filtering. Rust now exits the local vehicle subsystem,
  clears represented summon slots, drains represented controlled GUIDs/charmed GUID, and removes
  `UNIT_FLAG_PET_IN_COMBAT` for non-pet creature GUIDs. This does not claim real vehicle aura
  unapply, map lookup/TempSummon unsummon, ObjectAccessor cleanup, controlled target mutation,
  owner slot fanout, or full `RemoveAllAurasOnDeath`.
- 2026-05-31 — Represented creature death aura filtering `#NEXT.RUNTIME.L3.031ax`:
  ports the represented container rule of C++ `Unit::RemoveAllAurasOnDeath`
  (`Unit.cpp:4308-4330`): remove applied and owned auras only when they are neither passive nor
  death-persistent. Rust now stores explicit represented aura death policy metadata in
  `AuraSubsystem`, exposes `remove_all_auras_on_death_like_cpp`, and invokes it during creature
  `JustDied` after vehicle/summon/control cleanup and before the `JUST_DIED` reactive/diminishing
  cleanup. This is still metadata-driven: exact `SpellInfo::IsPassive`,
  `Aura::IsDeathPersistent`, scripts, proc/unapply handlers, packet emission, and full aura runtime
  remain separate gaps.
- 2026-05-31 — Represented creature respawn template flag reload `#NEXT.RUNTIME.L3.031ay`:
  ports the represented local subset of C++ `Creature::setDeathState(JUST_RESPAWNED)`
  `ObjectMgr::ChooseCreatureFlags` / `ReplaceAllNpcFlags*` / `ReplaceAllUnitFlags*`
  (`Creature.cpp:2268-2284`). Rust now preserves `unit_flags2` and `unit_flags3` in
  `CreatureAiOwnershipState` alongside existing npc/unit flags, wires them from represented
  `CreatureCreateData`, adds `UnitFlags3` setters/getters, and reloads represented live
  npc/unit flags during respawn before clearing `IN_COMBAT`. Full `ChooseCreatureFlags` conditions,
  world-event NPC flag overlays, creature-data overrides beyond represented create data, and full
  addon/sparring reload remain open.
- 2026-05-31 — Represented creature respawn melee damage school reload `#NEXT.RUNTIME.L3.031az`:
  ports the bounded `SetMeleeDamageSchool(SpellSchools(cInfo->dmgschool))` path from
  `Creature::UpdateEntry` / `Creature::setDeathState(JUST_RESPAWNED)` (`Creature.cpp:632,2286`,
  `Creature.h:178-179`) and the C++ ObjectMgr invalid-value clamp (`ObjectMgr.cpp:426,1094-1097`).
  Rust now loads represented `creature_template.dmgschool` into the lifecycle template store,
  normalizes invalid schools to normal, carries the value through loaded-grid lifecycle creation,
  applies `m_meleeDamageSchoolMask = 1 << school` during lifecycle create and represented respawn,
  and preserves it through `CreatureCreateData` when respawn reconstruction has the value. Legacy
  visibility/test routes without a `dmgschool` query explicitly seed normal school rather than
  inventing DB-backed data.
- 2026-05-31 — Represented creature respawn spawn-health reload `#NEXT.RUNTIME.L3.031ba`:
  ports the bounded non-pet `SetSpawnHealth()` source used by
  `Creature::setDeathState(JUST_RESPAWNED)` (`Creature.cpp:2251-2255`, `Creature.cpp:1954-1974`).
  The DB-backed loaded-grid resolver already resolves C++ `curhealth`/`curmana`,
  `_regenerateHealth`, classification health scaling, and `NO_HEALTH_REGEN`; Rust now stores that
  represented spawn health/mana in creature lifecycle metadata and uses it during represented
  respawn. Entities without DB-backed lifecycle metadata keep the existing max-health fallback
  rather than inventing missing spawn-row data.
- 2026-05-31 — Represented creature respawn Motion_Initialize fallthrough `#NEXT.RUNTIME.L3.031bb`:
  ports the bounded non-formation / formation-leader fallthrough of
  `Creature::Motion_Initialize()` during respawn (`Creature.cpp:2289`, `Creature.cpp:1046-1060`).
  Rust now reuses the existing represented `aim_initialize_like_cpp()` decision and calls
  `MotionSubsystem::direct_initialize_like_cpp()` when C++ would fall through to
  `MotionMaster::Initialize()`. Represented non-leader formation members intentionally keep their
  previous motion because exact behavior needs live `CreatureGroup::IsFormed` and cross-creature
  `MoveIdle` mutation.
- 2026-05-31 — DB-backed creature sparring health reload `#NEXT.RUNTIME.L3.031bc`:
  ports the bounded data/load path for C++ `ObjectMgr::LoadCreatureTemplateSparring` /
  `GetCreatureTemplateSparringValues` / `Creature::LoadCreaturesSparringHealth`
  (`ObjectMgr.cpp:899-937`, `ObjectMgr.cpp:1468-1471`, `Creature.cpp:2799-2802`) into the
  loaded-grid lifecycle path. Rust now validates `creature_template_sparring` rows against existing
  creature templates and the C++ `0 < pct <= 100` rule, preserves the percentage as `f32` rather
  than the previous lossy `u8`, randomly selects a value through the map-owned RNG seam, and applies
  it to loaded creatures so represented sparring damage/fake-damage uses the DB-backed float
  threshold. Exact addon reload is still a separate gap; `MoveFall()` remains intentionally open
  because the C++ path also needs `IsUnderWater()` plus real MotionMaster terrain/spline execution.
- 2026-05-31 — Represented creature death spell-focus cleanup `#NEXT.RUNTIME.L3.031bd`:
  ports the bounded state cleanup from C++ `Creature::setDeathState(JUST_DIED)`
  `ReleaseSpellFocus(nullptr, false)` and `DoNotReacquireSpellFocusTarget()`
  (`Creature.cpp:2223-2224`, `Creature.cpp:3340-3389`). Rust now represents `_spellFocusInfo`
  locally on `Creature`, preserves the C++ non-pet delayed-reacquire shape for `ReleaseSpellFocus`
  (`withDelay=false` schedules delay `1` before the following cancel), clears represented
  `UNIT_STATE_FOCUSING` for focus spells that use the `AI_DOESNT_FACE_TARGET` path, and death
  cancels any delayed snapback before clearing target. Full `Spell*`/`SpellInfo` validation,
  ObjectAccessor target-facing restore, pet override timing/fanout, and the live spell runtime
  remain separate gaps.
- 2026-05-31 — DB-backed creature template base flags `#NEXT.RUNTIME.L3.031be`:
  closes the bounded base-data half of C++ `Creature::setDeathState(JUST_RESPAWNED)`
  `ObjectMgr::ChooseCreatureFlags` / `ReplaceAllNpcFlags*` / `ReplaceAllUnitFlags*`
  (`Creature.cpp:2268-2284`) by loading `creature_template.npcflag`, `unit_flags`,
  `unit_flags2`, and `unit_flags3` from `ObjectMgr::LoadCreatureTemplate`
  (`ObjectMgr.cpp:349-375`, `ObjectMgr.cpp:403-430`). Rust now carries those fields through
  `CreatureTemplateLifecycleStoreLikeCpp`, `ResolvedCreatureTemplateLikeCpp`, and
  `CreatureTemplateLifecycleRecord`, applies them during lifecycle create, and respawn reloads them
  from represented creature identity. This is not full `ChooseCreatureFlags`: condition evaluation,
  creature-data overrides, addon reload, and the world-event NPC flag overlay from
  `GameEventMgr::GetNPCFlag` (`GameEventMgr.cpp:920-933`) remain explicit follow-up runtime gaps.
- 2026-05-31 — Game-event NPC flag overlay uses template base `#NEXT.RUNTIME.L3.031bf`:
  closes the bounded live-update side of C++ `GameEventMgr::UpdateEventNPCFlags`
  (`GameEventMgr.cpp:1115-1148`) and `GetNPCFlag` (`GameEventMgr.cpp:920-933`). That live route
  updates spawn ids present in `game_event_npcflag`, ORs active-event npcflag records, and ORs
  `creatureTemplate->npcflag` when available; unlike the respawn path (`Creature.cpp:2274-2275`),
  it is not guarded by `CREATURE_FLAG_EXTRA_WORLDEVENT`. Rust `game_event_update_npc_flags_like_cpp`
  now receives the DB-backed lifecycle template store, looks up the spawn template entry, and applies
  `template.npc_flags | active_event_overlay` instead of `0 | active_event_overlay`; the
  `update_npc_flags_template_npcflag_missing` counter now means a true missing template. This does
  not implement full `ChooseCreatureFlags` condition evaluation, creature-data overrides, or the
  respawn-route `flags_extra` gate nuance.
- 2026-05-31 — Loaded-grid creature spawn flag overrides `#NEXT.RUNTIME.L3.031bg`:
  ports the nullable spawn-row source selection inside C++ `ObjectMgr::ChooseCreatureFlags`
  (`ObjectMgr.cpp:1683-1697`) using the nullable fields loaded by `ObjectMgr::LoadCreatures`
  (`ObjectMgr.cpp:2174-2229`, `CreatureData.h:643-646`). Rust now carries optional
  `creature.npcflag` / `unit_flags*` in `CreatureSpawnRuntimeRowLikeCpp` and lets loaded-grid
  lifecycle construction choose `spawn override` before `creature_template` values. This is still
  bounded to the explicit nullable source-selection rule; addon reload and any broader runtime
  flag recomputation remain separate gaps.
- 2026-05-31 — Represented creature addon local fields `#NEXT.RUNTIME.L3.031bh`: starts the
  `Creature::LoadCreaturesAddon` respawn/create gap with the fields already modeled by
  `wow-entities`, contrasted against C++ `Creature::GetCreatureAddon` / `LoadCreaturesAddon`
  (`Creature.cpp:2722-2809`) and `CreatureAddon` (`CreatureData.h:677-694`). Rust now carries an
  optional `CreatureAddonLifecycleRecordLikeCpp` on `CreatureCreateLifecycleRecord`, stores it in
  lifecycle metadata, applies represented mount display, stand state, PvP flags, and non-zero emote
  during lifecycle creation, and reapplies the same represented subset during `JustRespawned`.
  This is not DB-backed addon loading: spawn-vs-template fallback, `creature_addon` /
  `creature_template_addon` stores, path id, auras, visual flags, anim tier/kits, sheath, pet flags,
  shapeshift form, visibility distance override, hover, and validation remain explicit follow-up
  gaps.
- 2026-05-31 — Represented creature addon store fallback `#NEXT.RUNTIME.L3.031bi`: adds the pure
  data-store side for the supported addon subset, contrasted against C++ `ObjectMgr::LoadCreatureAddons`
  / `LoadCreatureTemplateAddons` (`ObjectMgr.cpp:766-897`, `ObjectMgr.cpp:1224-1367`) and
  `Creature::GetCreatureAddon` (`Creature.cpp:2722-2732`). Rust now has
  `CreatureAddonStoreLikeCpp` with spawn-addon-before-template-addon fallback, row existence filters,
  C++-style mount/emote invalidation to zero, stand-state truncation to stand, and full-byte PvP flag
  retention. This is still a dormant data store: live DB loading, movement-type waypoint mutation,
  path id, aura/anim/sheath/visibility validation, and loaded-grid resolver wiring remain open.
- 2026-05-31 — Loaded-grid creature addon resolver wiring `#NEXT.RUNTIME.L3.031bj`: wires the
  supported addon subset through the DB-backed loaded-grid resolver, still contrasted against C++
  `Creature::GetCreatureAddon` (`Creature.cpp:2722-2732`) and `LoadCreaturesAddon`
  (`Creature.cpp:2734-2809`). `build_loaded_grid_creature_inputs_from_db_like_cpp` now accepts a
  `CreatureAddonStoreLikeCpp`, resolves the spawn-addon-before-template-addon record into
  `ResolvedCreatureTemplateLikeCpp`, and `CreatureLoadedGridLifecycleResolverLikeCpp` passes that
  resolved addon into `CreatureCreateLifecycleRecord` so create/respawn lifecycle code applies the
  represented mount/stand/PvP/emote fields. Production still passes an empty store until the next DB
  loading slice; path id, auras, visual flags, anim tier/kits, sheath, visibility distance, hover,
  and waypoint movement mutation remain explicit gaps.
- 2026-05-31 — DB-backed represented creature addon loading `#NEXT.RUNTIME.L3.031bk`: connects the
  represented addon store to live startup. Rust now loads `Emotes.db2`, then loads
  `creature_addon` / `creature_template_addon` with the same selected columns as C++
  `ObjectMgr::LoadCreatureAddons` / `LoadCreatureTemplateAddons` (`ObjectMgr.cpp:766-897`,
  `ObjectMgr.cpp:1224-1367`), filtering missing spawn/template owners and validating represented
  mount/emote/stand fields before the loaded-grid resolver consumes the store. This still only
  covers the represented subset (`mount`, `StandState`, `PvPFlags`, non-zero `emote`); path id
  runtime, auras, visual flags, anim tier/kits, sheath, visibility
  distance, hover, and full validation/reporting are still open.
- 2026-05-31 — Spawn-addon `PathId` movement mutation seam `#NEXT.RUNTIME.L3.031bl`: contrasted
  against C++ `ObjectMgr::LoadCreatureAddons` (`ObjectMgr.cpp:1229-1261`). Rust now retains addon
  `PathId` in `CreatureAddonLifecycleRecordLikeCpp` and applies the spawn-specific C++ load-time
  rule at the loaded-grid movement selection seam: a concrete spawn with DB `WAYPOINT_MOTION_TYPE`
  and a spawn-specific `creature_addon.PathId == 0` is selected as `IDLE_MOTION_TYPE`. Template
  addon rows deliberately do not trigger this mutation, matching C++. Because `wow-entities`
  currently represents only idle movement at the final entity seam, this does not claim waypoint
  runtime support; path execution, waypoint data loading, and full motion generator parity remain
  open.
- 2026-05-31 — Represented addon bytes1/bytes2 fields `#NEXT.RUNTIME.L3.031bm`: contrasted
  against C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2742-2758`) and addon load validation
  (`ObjectMgr.cpp:841-855`, `ObjectMgr.cpp:1311-1325`). Rust now carries and applies represented
  `VisFlags`, `AnimTier`, and `SheathState` through `CreatureAddonLifecycleRecordLikeCpp`,
  `UnitDataValues`, and the unit update bridge. `VisFlags` preserves the full byte like C++;
  `AnimTier >= Max` and `SheathState >= MAX_SHEATH_STATE` normalize to zero like C++. This still
  does not cover hover movement flags, visibility distance override, auras, or
  waypoint path execution.
- 2026-05-31 — Represented addon internal pet/form reset `#NEXT.RUNTIME.L3.031bn`: contrasted
  against C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2758-2761`). Rust now represents the
  fixed internal reset of `PetFlags` to `UNIT_PET_FLAG_NONE` and `ShapeshiftForm` to `FORM_NONE`
  through `UnitDataValues`, the unit update bridge, and addon create/respawn application. These
  fields are deliberately not loaded from DB because C++ treats them as core-internal and forces
  them to zero during addon application.
- 2026-05-31 — Represented addon anim kit create state `#NEXT.RUNTIME.L3.031bo`: contrasted
  against C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2765-2767`), `Unit::Set*AnimKitId`
  (`Unit.cpp:10409-10455`), `Object::BuildCreateUpdateBlockForPlayer` (`Object.cpp:145-152`,
  `Object.cpp:263`, `Object.cpp:420-425`), and addon validation (`ObjectMgr.cpp:865-883`,
  `ObjectMgr.cpp:1335-1353`). Rust now loads/validates addon `aiAnimKit`, `movementAnimKit`,
  and `meleeAnimKit` against `AnimKit.db2`, carries them through `CreatureAddonLifecycleRecordLikeCpp`,
  applies them as represented `Unit` internal state on create/respawn, and writes the C++ create-object
  `AnimKit` payload when any of the three IDs is non-zero. Live `SMSG_SET_*_ANIM_KIT` fanout remains
  a packet/runtime follow-up; visibility distance override, auras, and waypoint path execution
  remain open.
- 2026-05-31 — Represented addon hover movement flag `#NEXT.RUNTIME.L3.031bp`: contrasted against
  C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2747-2753`) and `Creature::CanHover`
  (`Creature.h:127`). Rust now mirrors the bounded addon behavior: when addon application runs and
  `CanHover()` is true (`Ground == Hover` or already hovering), it adds `MOVEMENTFLAG_HOVER` to the
  represented creature runtime movement flags. This intentionally does not call full `Unit::SetHover`,
  because C++ addon loading only calls `AddUnitMovementFlag`; full hover packets/height/anim-tier
  handling remains a separate runtime concern. Visibility distance override, auras, waypoint path
  execution, and live `SMSG_SET_*_ANIM_KIT` fanout remain open.
- 2026-05-31 — Anim kit live packet surface `#NEXT.RUNTIME.L3.031bq`: contrasted against C++
  `WorldPackets::Misc::SetAIAnimKit` / `SetMovementAnimKit` / `SetMeleeAnimKit`
  (`MiscPackets.h:753-783`, `MiscPackets.cpp:615-634`) and `Unit::Set*AnimKitId`
  (`Unit.cpp:10409-10455`). Rust now has the three SMSG packet serializers with the same payload
  shape: raw `ObjectGuid` plus `uint16 AnimKitID`. Runtime fanout from represented `Unit` setters is
  still not wired; this is only the protocol surface needed before that behavior can be closed.
  Visibility distance override, auras, and waypoint path execution remain open.
- 2026-05-31 — Represented addon visibility distance override `#NEXT.RUNTIME.L3.031br`:
  contrasted against C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2769-2771`),
  `WorldObject::SetVisibilityDistanceOverride` (`Object.cpp:952-978`), `WorldObject::GetVisibilityRange`
  (`Object.cpp:1449-1472`), the `VisibilityDistances` table (`Object.cpp:62-70`), and addon load
  validation (`ObjectMgr.cpp:885-889`, `ObjectMgr.cpp:1355-1359`). Rust now loads and normalizes
  addon `visibilityDistanceType`, carries it through `CreatureAddonLifecycleRecordLikeCpp`, stores
  the object-owned override for non-player world objects, applies it during represented creature
  addon create/respawn, and sets the matching `UNIT_FLAG2_LARGE_AOI` / `GIGANTIC_AOI` /
  `INFINITE_AOI` flag for Large/Gigantic/Infinite rows. This is not full runtime visibility
  routing: several session/update/fanout paths still use map visibility range directly, so exact
  client selection parity remains a follow-up runtime gap. Auras and waypoint path execution also
  remain open.
- 2026-05-31 — Represented addon auras `#NEXT.RUNTIME.L3.031bs`: contrasted against C++
  `ObjectMgr::LoadCreatureTemplateAddons` / `LoadCreatureAddons` (`ObjectMgr.cpp:766-897`,
  `ObjectMgr.cpp:1224-1367`) and `Creature::LoadCreaturesAddon` (`Creature.cpp:2777-2793`).
  Rust now reads the `auras` column from both addon tables, tokenizes it like C++, skips malformed
  or missing spell IDs, skips duplicates, skips temporary auras with `GetDuration() > 0`, and keeps
  `SPELL_AURA_CONTROL_VEHICLE` as warn-only rather than rejecting it. Validated aura IDs are carried
  through `CreatureAddonLifecycleRecordLikeCpp` and applied as represented self-cast aura identity on
  create/respawn when the creature does not already have that spell aura. This is still a represented
  `AddAura` subset: real spell effects, aura scripts, procs, visible-aura update-field packet fanout,
  and full spell runtime remain open.
- 2026-05-31 — Creature movement fanout visibility override `#NEXT.RUNTIME.L3.031bt`:
  contrasted against C++ `WorldObject::SendMessageToSet` / `SendMessageToSetInRange`
  (`Object.cpp:1746-1755`) and `WorldObject::GetVisibilityRange` (`Object.cpp:1449-1456`). The
  global legacy creature movement driver now uses the source `WorldCreature` represented visibility
  range when building `RecipientRule::NearbyVisible`, so `visibilityDistanceType` / object-owned
  overrides affect MonsterMove fanout instead of always using the default legacy map radius. This is
  the bounded movement-runtime use of the override; other session visibility scans and non-creature
  fanout paths that still consult map visibility remain separate follow-up gaps.
- 2026-05-31 — Waypoint PathId/default movement bridge `#NEXT.RUNTIME.L3.031bu`: contrasted against
  C++ `Creature::LoadCreaturesAddon` (`Creature.cpp:2773`), `Creature::CreateFromProto`
  (`Creature.cpp:558-560`), `ObjectMgr::LoadCreatureAddons` (`ObjectMgr.cpp:1253-1256`), and
  `WaypointMovementGenerator::DoInitialize` (`WaypointMovementGenerator.cpp:120-129`). Rust now
  preserves nonzero addon `PathId` on represented `Creature` and keeps loaded-grid
  `MovementType=WAYPOINT_MOTION_TYPE` as represented `MovementGeneratorType::Waypoint` after the
  existing spawn-addon `PathId==0` downgrade seam. This does not claim waypoint path DB loading,
  spline launch, timers, `MovementInform`, scripts, formation signaling, or live waypoint execution.
- 2026-05-31 — Creature anim-kit live fanout seam `#NEXT.RUNTIME.L3.031bv`: contrasted against C++
  `Unit::SetAIAnimKitId` / `SetMovementAnimKitId` / `SetMeleeAnimKitId` (`Unit.cpp:10409-10455`),
  `WorldPackets::Misc::Set*AnimKit` (`MiscPackets.h:753-783`, `MiscPackets.cpp:615-634`), and
  `WorldObject::SendMessageToSet` (`Object.cpp:1746-1755`). Rust now exposes a legacy
  map-owned `set_creature_anim_kit_id_like_cpp` seam that validates nonzero IDs through an
  `AnimKitStore` predicate, no-ops unchanged values, mutates represented `Unit` state, updates
  `CreatureCreateData` for late viewers, and returns a `RuntimeEvent::NearbyVisible` with the exact
  SMSG_SET_*_ANIM_KIT bytes. Production callers, canonical sync, scripts/spells, and live
  client/server verification remain open.
- 2026-05-31 — Waypoint path metadata store/load `#NEXT.RUNTIME.L3.031bw`: contrasted against C++
  `WaypointMgr::LoadPaths` / `_LoadPaths` / `_LoadPathNodes` / `LoadPathFromDB` /
  `LoadPathNodesFromDB` (`WaypointManager.cpp:29-129`) and `WaypointDefines.h`. Rust now has
  `WaypointPathStoreLikeCpp`, all-path/all-node DB statements, startup metadata loading before
  creature formations, C++-like X/Y `NormalizeMapCoord`, node orientation/delay preservation, and
  post-load report evidence for empty/backwards-too-short paths. Invalid MoveType rows are skipped
  as an intentional typed-store correction of C++'s suspicious insert-before-return behavior. This
  is not live waypoint execution: no generator launch, spline packet fanout, timers,
  `MovementInform`, scripts, or formation movement are claimed.
- 2026-05-31 — Waypoint default MotionMaster initialization `#NEXT.RUNTIME.L3.031bx`: contrasted
  against C++ `MotionMaster::DirectInitialize` / `InitializeDefault`
  (`MotionMaster.cpp:115-128`, `MotionMaster.cpp:1207-1213`),
  `FactorySelector::SelectMovementGenerator` (`CreatureAISelector.cpp:129-137`), and
  `WaypointMovementGenerator<Creature>` construction (`WaypointMovementGenerator.cpp:33-44`).
  Rust now carries a represented creature `MovementType=WAYPOINT_MOTION_TYPE` into
  `MotionSubsystem` and preserves that selected default across `direct_initialize_like_cpp()`
  instead of replacing it with idle. The represented default keeps C++ normal priority,
  initialization-pending state, and roaming base state. This remains pre-execution scaffolding:
  no waypoint path lookup, `DoInitialize` owner-path fallback, spline launch, timers, AI informs,
  formations, or live waypoint movement are claimed.
- 2026-05-31 — WorldCreature waypoint generator initialization seam `#NEXT.RUNTIME.L3.031by`:
  contrasted against C++ `WaypointMovementGenerator<Creature>::DoInitialize`
  (`WaypointMovementGenerator.cpp:120-148`) and the DB-loaded waypoint constructor
  (`WaypointMovementGenerator.cpp:33-44`). Rust `WorldCreature` now stores an
  `active_waypoint_generator` and can initialize the represented DB-loaded waypoint generator with
  an already loaded path, applying owner default `PathId` through the existing `wow-movement`
  initializer. The path-found branch stores the generator, records the C++ 1000ms initial delay,
  and calls represented `StopMoving`; the missing-path branch stores the generator but does not
  stop the owner, matching the C++ early return. Still open: resolving the path from
  `WaypointPathStoreLikeCpp`, launching `MoveSpline`, advancing timers/nodes, MonsterMove fanout,
  AI waypoint informs, formations/scripts, and live server/client validation.
- 2026-05-31 — WorldCreature waypoint initial launch seam `#NEXT.RUNTIME.L3.031bz`: contrasted
  against C++ `WaypointMovementGenerator<Creature>::DoUpdate` / `StartMove`
  (`WaypointMovementGenerator.cpp:148-222`, `WaypointMovementGenerator.cpp:309-422`). Rust can now
  update the stored represented waypoint generator through the initial 1000ms delay, consume the
  first `Launch` action, apply represented `UNIT_STATE_ROAMING_MOVE`, retain `WaypointStarted`
  evidence from `wow-movement`, and launch a `MoveSplineInit` toward the first waypoint with the
  currently represented facing/walk/velocity/transport flags. This remains a bounded launch seam:
  path/MMAP generation, Land/TakeOff animation tier, formation side effects, arrival timers,
  next-node progression, AI inform dispatch, MonsterMove fanout wiring, and live server/client
  validation remain open.
- 2026-05-31 — WorldCreature waypoint arrival/next-node/path-end side effects
  `#NEXT.RUNTIME.L3.031ca`: contrasted against C++ `WaypointMovementGenerator<Creature>::DoUpdate`,
  `OnArrived`, `MovementInform`, and `StartMove` (`WaypointMovementGenerator.cpp:148-222`,
  `WaypointMovementGenerator.cpp:244-307`, `WaypointMovementGenerator.cpp:309-422`). Rust now
  applies represented side effects for stored waypoint-generator `StopMoving`, `Arrived`, `Launch`,
  and `PathEnded` actions. Arrival clears `UNIT_STATE_ROAMING_MOVE` when C++ would, records
  represented `MovementInform(WAYPOINT_MOTION_TYPE, nodeId)` evidence, honors a node delay before
  launching the next node, and single-node non-repeating paths finalize with home position set to
  the last waypoint target. Same-tick `OnArrived` to `StartMove` chaining is covered by
  `#NEXT.RUNTIME.L3.031cb`. Still open: random wait/end wandering, real
  `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation,
  Land/TakeOff animation tier, formation side effects, MonsterMove fanout wiring, automatic
  `WaypointPathStoreLikeCpp` resolution, and live server/client validation.
- 2026-05-31 — Same-tick waypoint `OnArrived` to `StartMove` chaining `#NEXT.RUNTIME.L3.031cb`:
  contrasted against C++ `WaypointMovementGenerator<Creature>::DoUpdate`
  (`WaypointMovementGenerator.cpp:208-222`) and `StartMove` (`WaypointMovementGenerator.cpp:327-422`).
  Rust `WorldCreature::update_default_waypoint_movement_like_cpp` now applies an `Arrived` action
  and, when no delay/random-wait timer was scheduled, immediately advances the stored generator with
  `diff=0` once more so the next waypoint launch or non-repeating `PathEnded` happens in the same
  update call, matching the C++ `_nextMoveTime.Passed()` branch. Tests cover no-delay next-node
  same-tick launch and single-node same-tick path-end. Still open: random wait/end wandering, real
  `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation, Land/TakeOff animation
  tier, formation side effects, MonsterMove fanout wiring, automatic `WaypointPathStoreLikeCpp`
  resolution, and live server/client validation.
- 2026-05-31 — Waypoint Land/TakeOff animation tier application `#NEXT.RUNTIME.L3.031cc`:
  contrasted against C++ `WaypointMovementGenerator<Creature>::StartMove`
  `init.SetAnimation(AnimTier::Ground/Hover)` (`WaypointMovementGenerator.cpp:391-399`) and
  `AnimTier` values (`UnitDefines.h:63-69`, Ground=0, Hover=2). Rust already represented
  `WaypointLaunchPlan.animation`; `WorldCreature::begin_waypoint_launch_like_cpp` now applies it
  through `MoveSplineInit::set_animation` before launch, so Land waypoints produce anim tier 0 and
  TakeOff waypoints produce anim tier 2. Test covers both branches. Still open: random wait/end
  wandering, real `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation, formation
  side effects, MonsterMove fanout wiring, automatic `WaypointPathStoreLikeCpp` resolution, and live
  server/client validation.
- 2026-05-31 — Waypoint path-end random handoff guard `#NEXT.RUNTIME.L3.031cd`: contrasted against
  C++ `WaypointMovementGenerator<Creature>::OnArrived` random path-end branch
  (`WaypointMovementGenerator.cpp:290-301`) and same-tick `DoUpdate`/`StartMove` flow
  (`WaypointMovementGenerator.cpp:234-244`). Rust `WorldCreature` now carries a represented active
  random-at-path-end handoff, accepts an injectable wait roll for deterministic parity tests, records
  `WaypointRandomAtPathEnd`, and prevents same-tick/next-tick waypoint progression while that
  represented handoff duration is active. This fixes the bridge bug where `move_random_at_path_end`
  with no `_nextMoveTime` could immediately launch the next waypoint. Still open: real
  `MotionMaster::MoveRandom`/`RandomMovementGenerator` bridge at path ends, real
  `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation, formation side effects,
  MonsterMove fanout wiring, automatic `WaypointPathStoreLikeCpp` resolution, and live server/client
  validation.
- 2026-05-31 — Waypoint DB path-store resolver seam `#NEXT.RUNTIME.L3.031ce`: contrasted against
  C++ `WaypointMovementFactory::Create` creating `WaypointMovementGenerator<Creature>(0, true)`
  (`MovementGenerator.cpp:56-64`), `WaypointMovementGenerator<Creature>::DoInitialize` resolving
  `_pathId` from `owner->GetWaypointPathId()` and `sWaypointMgr->GetPath(_pathId)`
  (`WaypointMovementGenerator.cpp:120-148`), `Creature::LoadPath` storing `_waypointPathId`
  (`Creature.h:332-333`), and `WaypointMgr` loading `waypoint_path` / `waypoint_path_node`
  (`WaypointManager.cpp:29-129`). Rust now exposes `Creature::load_path_like_cpp`, a
  `WorldCreature::initialize_default_waypoint_movement_with_path_resolver_like_cpp` seam, and the
  `world-server` helper `initialize_world_creature_default_waypoint_from_store_like_cpp` that
  resolves `WaypointPathStoreLikeCpp` into the existing waypoint initializer. Tests cover direct
  owner-path resolution and store-backed initialization launching the DB node after the C++ 1000ms
  initial delay. Still open: wiring this helper into live DB creature spawn initialization, real
  `MotionMaster::MoveRandom`/`RandomMovementGenerator` bridge at path ends, real
  `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation, formation side effects,
  MonsterMove fanout wiring, and live server/client validation.
- 2026-05-31 — Waypoint path-end random handoff launches active random spline
  `#NEXT.RUNTIME.L3.031cf`: contrasted against C++ `WaypointMovementGenerator<Creature>::OnArrived`
  calling `owner->GetMotionMaster()->MoveRandom(*_wanderDistanceAtPathEnds, waitTime,
  MOTION_SLOT_ACTIVE)` (`WaypointMovementGenerator.cpp:290-301`), `MotionMaster::MoveRandom`
  adding `RandomMovementGenerator<Creature>` (`MotionMaster.cpp:599-604`), and
  `RandomMovementGenerator<Creature>::SetRandomLocation` choosing a destination within
  `_wanderDistance` then launching a spline (`RandomMovementGenerator.cpp:113-190`). Rust now
  starts a represented active random spline from the creature's current path-end position when
  `WaypointRandomAtPathEnd` is emitted, keeps the waypoint generator blocked while the active
  handoff duration remains, and advances that spline during waypoint updates. Still open: full
  `RandomMovementGenerator` state machine/path/LOS retry parity at path ends, MonsterMove fanout for
  this spline, real `WaypointReached`/`WaypointPathEnded` AI dispatch, path/MMAP generation,
  formation side effects, live DB spawn wiring for waypoint initialization, and live server/client
  validation.
- 2026-05-31 — Waypoint movement tick emits `MonsterMove`
  `#NEXT.RUNTIME.L3.031d0`: contrasted against C++ `WaypointMovementGenerator<Creature>::DoUpdate`
  running from the creature update (`WaypointMovementGenerator.cpp:208-244`) and
  `WaypointMovementGenerator<Creature>::StartMove` launching the current node with
  `MoveSplineInit` (`WaypointMovementGenerator.cpp:309-422`). Rust `step_creature_movement_like_cpp`
  no longer ignores `CreatureAiState::WalkingWaypoint`: it advances the represented waypoint
  generator using the caller's diff and returns an `OnMonsterMove` packet when a launch creates a
  new spline, so both the old session tick and the experimental global runtime can use the existing
  `RuntimePlan`/`SendIfVisibleLikeCpp` fanout rail. The global runtime bridge now forwards its real
  map-update interval to the movement body, while `WorldCreature` marks default waypoint movement as
  `WalkingWaypoint` and returns to `Idle` on path end. Still open: live DB spawn wiring for
  `WaypointPathStoreLikeCpp` initialization, full path/MMAP generation for waypoint splines, real
  `WaypointReached`/`WaypointPathEnded` AI dispatch, formation side effects, full
  `RandomMovementGenerator` state machine/path/LOS retry parity at path ends, and live server/client
  validation.
- 2026-05-31 — Session waypoint path resolver injected
  `#NEXT.RUNTIME.L3.031d1`: contrasted against C++ `Creature::LoadCreaturesAddon` copying addon
  `PathId` into `_waypointPathId` (`Creature.cpp:2773`) and
  `WaypointMovementGenerator<Creature>::DoInitialize` resolving `owner->GetWaypointPathId()` through
  `sWaypointMgr->GetPath(_pathId)` (`WaypointMovementGenerator.cpp:120-148`). Rust now exposes a
  `WaypointPathResolverLikeCpp` seam on `WorldSession`; `world-server` injects a resolver backed by
  `CanonicalSpawnMetadataLikeCpp::waypoint_paths_like_cpp`, and session-created legacy
  `WorldCreature::from_canonical` compatibility objects initialize the represented default waypoint
  generator when the canonical creature already carries `MovementType=Waypoint`. This is a wiring
  seam, not the full DB closure. Still open after `#NEXT.RUNTIME.L3.031d2`: internal `wow-map`
  loaded-grid respawn/spawn-group/pool paths still need a `world-server` mirror hook, the direct
  session DB query path still does not hydrate addon `PathId`, AI
  `WaypointReached`/`WaypointPathEnded`, formation side effects, full random path-end generator
  parity, and live server/client validation.
- 2026-05-31 — Loaded-grid game-event Creature mirror into legacy runtime
  `#NEXT.RUNTIME.L3.031d2`: contrasted against C++ `Map::AddToMap<T>` adding the loaded object to
  the single live map runtime (`Map.cpp:530-570`) and the map-owned respawn/add flow
  (`Map.cpp:2191+`). Rust split runtime now mirrors successfully added game-event loaded-grid
  `MapObjectRecord::Creature` records from the canonical `wow_map::Map` into the legacy
  `wow_world::MapManager`, reconstructing `CreatureCreateData` from the canonical creature and
  resolving default waypoint paths through `WaypointPathStoreLikeCpp`. This closes the direct
  `world-server` game-event loaded-grid path needed by the experimental legacy global creature
  tick. Still open: loaded-grid records inserted internally inside `wow-map` respawn/spawn-group/pool
  code cannot mirror yet without an explicit event/hook back to `world-server`; adding a dependency
  from `wow-map` to `wow-world` remains rejected.
- 2026-05-31 — Internal loaded-grid AddToMap records mirrored to legacy runtime
  `#NEXT.RUNTIME.L3.031d3`: contrasted against C++ `Map::DoRespawn` (`Map.cpp:2165-2188`),
  `Map::SpawnGroupSpawn` (`Map.cpp:2356-2395`), `PoolGroup<Creature>::Spawn1Object`
  (`PoolMgr.cpp:355-363`), and `Map::AddToMap<T>` (`Map.cpp:530-570`). Rust preserves the
  dependency boundary: `wow-map` records successful internal loaded-grid primary inserts in summary
  fields instead of depending on `wow-world`; `world-server` consumes those records after map
  mutation and mirrors Creature records into the legacy `MapManager`. This extends `031d2` beyond
  direct game-event spawning to canonical tick respawns, spawn-group condition spawns, pool and
  event-pool loaded-grid execution. Remaining gap: this still mirrors only Creature records into the
  legacy creature runtime; GameObject runtime parity, pre-add trap side effects, and live
  server/client validation remain separate work. Direct session DB addon `PathId` hydration is
  superseded by `031d4`.
- 2026-05-31 — Direct session DB creature query addon `PathId` hydration
  `#NEXT.RUNTIME.L3.031d4`: contrasted against C++ `Creature::LoadFromDB`, `Creature::Create`,
  `Creature::LoadCreaturesAddon`, `ObjectMgr::LoadCreatureAddons`, and
  `WaypointMovementGenerator<Creature>::DoInitialize`. The legacy fallback DB visibility query now
  selects effective spawn movement and resolved addon path (`creature_addon` first,
  `creature_template_addon` fallback), preserving the existing column/parameter prefix. Both direct
  session visibility paths pass those values to creature registration, which sets the represented
  default movement and nonzero waypoint path before building the `WorldCreature`; DB waypoint
  creatures therefore initialize the stored waypoint generator through the injected
  `WaypointPathStoreLikeCpp` resolver. This closes the direct-session gap left after `031d1-031d3`;
  live client validation, full AI waypoint callbacks, path/MMAP, formations, and GameObject runtime
  parity remain open.
- 2026-05-31 — Canonical typed GameObject visibility create fallback
  `#NEXT.RUNTIME.L3.031d5`: contrasted against C++ `Map::AddToMap<T>` (`Map.cpp:530-574`) and
  `GameObject::Create` / `GameObject::AddToWorld` (`GameObject.cpp:899-970`). A GameObject present
  in the live map/grid is visible as a map-owned object; Rust no longer requires
  session-local represented GameObject state before `visible_gameobjects_from_canonical_map_like_cpp`
  can rebuild `GameObjectCreateData` from the canonical typed record. Represented per-player
  despawn state and represented-state overrides still take precedence, and `GameObject` now stores
  the local lifecycle rotation needed for exact create-data reconstruction. Remaining gaps: full
  GameObject runtime/use/scripts/traps/GO AI, complete values-update fanout, dynamic-tree/collision,
  ObjectAccessor parity, and live client/server validation.
- 2026-05-31 — Viewer-dependent GameObject create state `#NEXT.RUNTIME.L3.031d6`: contrasted
  against C++ viewer-dependent `GameObjectData::StateTag` (`ViewerDependentValues.h:323-332`) and
  `GameObject::GetGoStateFor` / `SetGoStateFor` (`GameObject.cpp:3795-3815`). Represented
  GameObject create visibility now rebuilds `GameObjectCreateData.state` from the receiving
  session's effective state: active, unexpired per-player state for that player wins, otherwise the
  shared represented state or Ready fallback is used. This closes the create-packet side of the
  existing represented per-player state rail; viewer-dependent group-loot chest flags and full
  GameObject values-update fanout remain separate gaps.
- 2026-05-31 — Local GameObject state packet `#NEXT.RUNTIME.L3.031d7`: contrasted against C++
  `GameObject::SetGoStateFor` (`GameObject.cpp:3805-3815`) and `GameObjectSetStateLocal::Write`
  (`GameObjectPackets.cpp:82-88`). Rust now serializes `SMSG_GAME_OBJECT_SET_STATE_LOCAL` as raw
  ObjectGuid + uint8 state and sends it from the represented multi-interact goober branch when the
  target player is the current session. This makes the per-player state rail visible immediately
  instead of only influencing later create visibility. Remaining gaps: consumable `DespawnForPlayer`
  packet parity, group-loot chest viewer flags, full GameObject values-update fanout, GO
  scripts/traps/AI, and live client/server validation.
- 2026-06-01 — Per-player GameObject out-of-range removal `#NEXT.RUNTIME.L3.031d8`: contrasted
  against C++ `GameObject::DespawnForPlayer` (`GameObject.cpp:1732-1738`), per-player despawn
  visibility (`GameObject.cpp:2157-2171`), and `Player::UpdateVisibilityOf` / `Object::SendOutOfRangeForPlayer`
  (`Player.cpp:23193-23208`, `Object.cpp:213-245`). The consumable multi-interact goober path now
  mirrors `DespawnForPlayer` for the current session by sending `UpdateObject::out_of_range_objects`
  only when `client_visible_guids_like_cpp` contains the GO, then erasing it from that set. This is
  deliberately not `SMSG_GAMEOBJECT_DESPAWN`, which C++ uses for `GameObject::Delete` /
  `SendGameObjectDespawn` rather than this per-player visibility path. Remaining gaps: group-loot
  chest `OnLootRelease` per-player despawn, group-loot viewer flags, full values-update fanout,
  scripts/traps/GO AI, and live client/server validation.
- 2026-06-04 — GameObject loot-release personal chest visibility gate `#NEXT.RUNTIME.L3.031d9`:
  contrasted against C++ `WorldSession::DoLootRelease` (`LootHandler.cpp:289-320`),
  `GameObject::OnLootRelease` (`GameObject.cpp:3735-3748`), and `Player::UpdateVisibilityOf`
  (`Player.cpp:23193-23208`). Rust's represented `chestPersonalLoot` release path already recorded
  `DespawnForPlayer` state and sent out-of-range for visible chests; it now also mirrors the C++
  `HaveAtClient` guard and skips `UpdateObject::out_of_range_objects` when the GO is absent from
  `client_visible_guids_like_cpp`. Remaining gaps: group-loot viewer flags, full values-update
  fanout, scripts/traps/GO AI, and live client/server validation.
- 2026-06-04 — GM viewer-dependent GameObject dynamic flags `#NEXT.RUNTIME.L3.031da`: contrasted
  against C++ `ViewerDependentValues.h:97-111`. Rust now mirrors the two missing
  `receiver->IsGameMaster()` branches for non-quest-activating chests and goobers by setting
  `GO_DYNFLAG_LO_ACTIVATE` in `represented_gameobject_dynamic_flags_for_player_like_cpp`. Tests
  exercise the real visible values-update path for quest-related-but-not-activating chest/goober
  objects and assert exact `GameObjectData::DynamicFlags` bytes. Remaining gaps: full GameObject
  create/update fanout parity, scripts/traps/GO AI, and live client/server validation.
- 2026-06-04 — GameObject create dynamic flags `#NEXT.RUNTIME.L3.031db`: contrasted against C++
  `Object::BuildCreateUpdateBlockForPlayer` (`Object.cpp:130-172`), `GameObject::BuildValuesCreate`
  (`GameObject.cpp:3914-3923`), `ObjectData::WriteCreate` (`UpdateFields.cpp:38-43`), and
  `ViewerDependentValue<DynamicFlagsTag>` (`ViewerDependentValues.h:60-145`). Rust no longer
  hardcodes GameObject create `ObjectData::DynamicFlags` to zero; `GameObjectCreateData` carries the
  viewer-dependent value and represented/canonical/direct-DB create paths populate it through the
  same dynamic-flags seam used by values updates. The slice also fixes an exposed Rust bug where
  expired per-player GO state could still affect GOOBER/GATHERING dynamic flags. Remaining gaps:
  script/ConditionMgr-rich activation for DB-only create paths beyond represented state, full
  GameObject update fanout, scripts/traps/GO AI, and live client/server validation.
- 2026-06-04 — Represented gathering-node GameObject refresh fanout `#NEXT.RUNTIME.L3.031dc`:
  contrasted against C++ `Player::UpdateVisibleGameobjectsOrSpellClicks` (`Player.cpp:24439-24478`)
  rebuilding per-viewer GameObject dynamic-flag deltas for objects already visible to each player.
  This introduced same-map/instance remote refresh scheduling for represented gathering-node state
  changes while preserving the receiving session as the authority for `HaveAtClient`
  (`client_visible_guids_like_cpp`) and viewer-dependent flags. `031dd` refines this because the
  receiver also needs the changed represented GO state before recomputing. Remaining gaps: this is
  a bounded gathering-node use-state fanout only; full GameObject values-update fanout for chest,
  goober, trap, script, GO AI, map-owned update phases, and live client/server validation remains
  open.
- 2026-06-04 — Authoritative represented gathering-node state sync `#NEXT.RUNTIME.L3.031dd`:
  audit correction to `031dc`. C++ has one shared `GameObject`, so every receiver recomputing
  `UpdateVisibleGameobjectsOrSpellClicks` reads the same mutated GO state; Rust represented GO
  state is still per-session, so a bare remote refresh could recompute from stale local state. Rust
  now sends `SyncGatheringNodeGameobjectStateAndRefreshLikeCpp` with the bounded gathering-node
  state subset, gates it by LoggedIn/map/instance/type on the receiver, applies it to that
  session's represented state, then runs the receiver-authoritative refresh. Remaining gaps:
  broader shared/canonical GameObject state ownership, chest/goober/trap/script/GO AI fanout, and
  live client/server validation.
- 2026-06-04 — Represented chest-open state sync `#NEXT.RUNTIME.L3.031de`: contrasted against C++
  `GameObject::Use` chest activation (`GameObject.cpp:2608-2662`), `GameObject::SetLootState`
  (`GameObject.cpp:3683-3710`), `Player::UpdateVisibleGameobjectsOrSpellClicks`
  (`Player.cpp:24439-24478`), and `GameObject::ActivateToQuest` (`GameObject.cpp:2220-2268`).
  Rust now sends `SyncChestGameobjectStateAndRefreshLikeCpp` after represented chest activation so
  same-map/instance remote sessions receive the bounded shared chest state before recomputing their
  viewer-dependent GameObject flags. The receiver gates by LoggedIn/map/instance/type, applies
  loot-state/unit-guid plus chest loot metadata used by the represented dynamic-flag seam, then
  runs the existing receiver-authoritative refresh. Remaining gaps: chest release, fully-looted and
  restock transitions, broader `UpdateDynamicFlagsForNearbyPlayers` / `UpdateObjectVisibility`,
  personal despawn/delete packets, trap/script/GO AI, canonical shared GameObject ownership, and
  live client/server validation.
- 2026-06-04 — Represented chest loot-release state sync `#NEXT.RUNTIME.L3.031df`: contrasted
  against C++ `WorldSession::DoLootRelease` (`LootHandler.cpp:270-320`), `GameObject::SetLootState`
  (`GameObject.cpp:3683-3710`), `GameObject::OnLootRelease` (`GameObject.cpp:3735-3748`), and
  `Player::UpdateVisibleGameobjectsOrSpellClicks` (`Player.cpp:24439-24478`). Rust now reuses the
  chest state-sync rail after represented loot release mutates chest state, so same-map/instance
  remote sessions receive the bounded chest state before recomputing viewer-dependent flags. This
  covers the release-time shared-state refresh; it does not close the full `GameObject::Update`
  restock tick, fully-looted visibility/update fanout, per-player despawn visibility for other
  viewers, delete/despawn packets, linked traps, scripts/GO AI, canonical shared GameObject
  ownership, or live client/server validation.
- 2026-06-04 — Represented chest restock state sync `#NEXT.RUNTIME.L3.031dg`: contrasted against
  C++ `GameObject::Update` chest restock (`GameObject.cpp:1294-1320`),
  `GameObject::UpdateDynamicFlagsForNearbyPlayers` (`GameObject.cpp:4382-4386`), and
  `Player::UpdateVisibleGameobjectsOrSpellClicks` (`Player.cpp:24439-24478`). Rust `process_pending`
  already represented the local restock mutation; it now queues the bounded chest state sync after
  clearing loot and setting `GO_READY`, so remote same-map/instance viewers refresh from the
  restocked state. Remaining gaps: map-owned `GameObject::Update` ownership, `UpdateObjectVisibility`,
  fully-looted/despawn branches, delete/despawn packets, linked traps, scripts/GO AI, canonical shared
  GameObject ownership, and live client/server validation.
- 2026-06-04 — Gathering-node loot-release direct dynamic-flags update `#NEXT.RUNTIME.L3.031dh`:
  contrasted against C++ `GameObject::OnLootRelease` gathering-node branch (`GameObject.cpp:3735-3762`),
  which applies `SetGoStateFor(GO_STATE_ACTIVE, looter)` and sends a direct dynamic-flags values update
  to that looter. Rust now sends a single-GO `UpdateObject::game_object_values_update` after represented
  gathering-node loot release when the GO is still `HaveAtClient`, using canonical GO access for the
  entry and the represented viewer-dependent dynamic-flags seam. Remaining gaps: remote/state ownership
  beyond the existing `031dd` sync rail, despawn/delete, scripts/GO AI, and live client/server validation.
- 2026-06-04 — Represented goober activation state sync `#NEXT.RUNTIME.L3.031di`: contrasted against
  C++ `GameObject::Use` `GAMEOBJECT_TYPE_GOOBER` shared branch (`GameObject.cpp:2740-2795`) and the
  later autoclose / `GO_JUST_DEACTIVATED` update branches (`GameObject.cpp:1516-1523`,
  `GameObject.cpp:1575-1647`). Rust now sends `SyncGooberGameobjectStateAndRefreshLikeCpp` after the
  non-multi-interact represented goober shared-state mutation so same-map/instance remote viewers apply
  the bounded visual state before `UpdateVisibleGameobjectsOrSpellClicks`. The receiver intentionally
  does not import `cooldown_until` or `goober_use_source`; map-owned autoclose/despawn remains a later
  runtime ownership slice and duplicating those fields per receiver would preserve the current bug.
  Remaining gaps: custom-animation fanout to other viewers, autoclose/JustDeactivated ownership,
  unique-user spell cleanup, ClearLoot/NODESPAWN/delete/despawn branches, scripts/GO AI, canonical
  shared GameObject ownership, and live client/server validation.
- 2026-06-04 — Represented goober custom-animation fanout `#NEXT.RUNTIME.L3.031dj`: contrasted against
  C++ `GameObject::Use` goober `customAnim` branch (`GameObject.cpp:2795-2802`) and
  `GameObject::SendCustomAnim`, which sends `SMSG_GAME_OBJECT_CUSTOM_ANIM` to the visible set
  including the source player (`GameObject.cpp:3413-3419`). Rust now keeps the existing direct self
  send and also enqueues the same packet bytes to same-map/instance remote sessions through
  `SendIfVisibleLikeCpp`, preserving the per-session `HaveAtClient` gate. Remaining gaps:
  autoclose/JustDeactivated ownership, unique-user spell cleanup, ClearLoot/NODESPAWN/delete/despawn
  branches, scripts/GO AI, canonical shared GameObject ownership, and live client/server validation.
- 2026-06-04 — Represented goober autoclose final state sync `#NEXT.RUNTIME.L3.031dk`: contrasted
  against C++ `GameObject::Update` goober cooldown expiry and `GO_JUST_DEACTIVATED` cleanup
  (`GameObject.cpp:1516-1523`, `GameObject.cpp:1575-1628`). Rust already represented the two-step
  local autoclose/cleanup; it now queues the bounded goober state sync after non-consumable cleanup so
  same-map/instance viewers refresh from the final shared `GO_READY` / flag-cleared state. Remaining
  gaps: consumable/despawn-at-action fanout, `SendGameObjectDespawn`, owner/summoned delete paths,
  linked-trap runtime despawn, scripts/GO AI, canonical shared GameObject ownership, and live
  client/server validation.
- 2026-06-04 — Represented goober consumable visual despawn fanout `#NEXT.RUNTIME.L3.031dl`:
  contrasted against C++ `GameObject::Update` `GO_JUST_DEACTIVATED` despawn-at-action branch and
  `GameObject::SendGameObjectDespawn` (`GameObject.cpp:1640-1647`, `GameObject.cpp:1766-1771`).
  Rust now keeps the represented goober consumable self-send and also enqueues the exact same
  `SMSG_GAME_OBJECT_DESPAWN` bytes to same-map/instance remote sessions through `SendIfVisibleLikeCpp`.
  Remaining gaps: generic chest/GO visual despawn fanout, owner/summoned delete paths, linked-trap
  runtime despawn, object removal/list persistence, scripts/GO AI, canonical shared GameObject
  ownership, and live client/server validation.
- 2026-06-04 — Represented generic/chest `GO_JUST_DEACTIVATED` visual despawn fanout
  `#NEXT.RUNTIME.L3.031dm`: contrasted against the same C++ `GameObject::Update`
  despawn-at-action / animation-progress branch and `GameObject::SendGameObjectDespawn`
  (`GameObject.cpp:1640-1647`, `GameObject.cpp:1766-1771`). Rust's represented generic/chest
  `send_despawn_at_action && !delete_after_clear && !schedule_respawn` path now preserves the
  owning-session `SMSG_GAME_OBJECT_DESPAWN` and enqueues the same bytes to same-map/instance remote
  sessions through `SendIfVisibleLikeCpp`. Remaining gaps: owner/summoned delete paths,
  scheduled-respawn/remove-list despawn, linked-trap runtime despawn, object removal/list
  persistence, scripts/GO AI, canonical shared GameObject ownership, and live client/server
  validation.
- 2026-06-04 — Represented delete-path `GameObjectDespawn` fanout `#NEXT.RUNTIME.L3.031dn`:
  contrasted against C++ `GameObject::Delete`, which calls `SendGameObjectDespawn()` before
  reset/remove-list handling, and `GameObject::SendGameObjectDespawn`, which sends the despawn
  packet to the visible set (`GameObject.cpp:1743-1753`, `GameObject.cpp:1766-1771`). Rust's
  represented delete helper now fanouts only the `SMSG_GAME_OBJECT_DESPAWN` through
  `SendIfVisibleLikeCpp`, while preserving the owning-session `UpdateObject::destroy_objects`
  packet as-is. Remaining gaps: full remove-list/object lifecycle, owner/summoned delete semantics,
  linked-trap runtime despawn by GUID, scripts/GO AI, canonical shared GameObject ownership, and
  live client/server validation.
- 2026-06-04 — Represented linked-trap GUID bridge `#NEXT.RUNTIME.L3.031do`: contrasted against
  C++ linked-trap ownership (`GameObject::GetLinkedTrap` returns the runtime GUID stored in
  `m_linkedTrap`, `GameObject.cpp:3909-3912`; `RemoveFromWorld` and `GO_JUST_DEACTIVATED` call
  `GetLinkedTrap()->DespawnOrUnsummon()`, `GameObject.cpp:939-941`, `GameObject.cpp:1577-1579`).
  Rust already had canonical loaded-grid `linked_trap_guid`, but represented session state only
  carried `linked_trap_entry`. The represented state and same-map sync commands now preserve
  `linked_trap_guid` when canonical data has it. Remaining gap: actually applying
  `DespawnOrUnsummon`/delete by GUID in represented linked-trap paths, plus full remove-list/object
  lifecycle and live client/server validation.
- 2026-06-04 — Represented linked-trap despawn by GUID `#NEXT.RUNTIME.L3.031dp`: contrasted
  against C++ `GO_JUST_DEACTIVATED` linked-trap cleanup (`GameObject.cpp:1577-1579`) and
  `GameObject::RemoveFromWorld` linked-trap cleanup (`GameObject.cpp:939-941`). Rust now uses the
  bridged `linked_trap_guid` to run the represented delete/despawn packet path for linked traps in
  generic and goober `GO_JUST_DEACTIVATED` cleanup, while preserving the existing entry-based
  evidence effects. Remaining gaps: remove-from-world/object-store lifecycle parity,
  `DespawnOrUnsummon` scheduler nuances, scripts/GO AI, canonical shared GameObject ownership, and
  live client/server validation.
- 2026-06-04 — Canonical linked-trap delete queues remove-list `#NEXT.RUNTIME.L3.031dq`: contrasted
  against C++ `GO_JUST_DEACTIVATED` linked-trap cleanup (`GameObject.cpp:1577-1579`),
  `GameObject::DespawnOrUnsummon(0)` (`GameObject.cpp:1711-1729`), `GameObject::Delete`
  (`GameObject.cpp:1740-1764`), and `Map::AddObjectToRemoveList` / drain
  (`Map.cpp:2547-2555`, `Map.cpp:2574-2646`). Canonical `wow-map::Map::update_game_object_like_cpp`
  now routes linked traps through a bounded `gameobject_delete_like_cpp` helper and queues the trap
  on the represented remove-list instead of physically extracting it during the update tick. Focused
  tests assert the trap remains in `map_objects` after update, is `GO_NOT_READY`/ready-state reset,
  and is removed only when `remove_all_objects_in_remove_list_like_cpp` drains. Remaining gaps:
  general `RemoveFromWorld` linked-trap scheduler parity, same-pass drain of newly queued objects,
  PoolMgr/DB/capture-point packet runtime, scripts/GO AI, ObjectAccessor/session fanout, and live
  client/server validation.
- 2026-06-04 — Represented delayed-despawn linked-trap cleanup `#NEXT.RUNTIME.L3.031dr`: contrasted
  against C++ delayed `GameObject::DespawnOrUnsummon` expiry into immediate `Delete`
  (`GameObject.cpp:1711-1729`, `GameObject.cpp:1740-1764`) plus the later linked-trap
  `RemoveFromWorld` cleanup (`GameObject.cpp:939-941`). The represented `despawn_delay_until`
  expiry path now snapshots `linked_trap_guid` and runs the existing represented linked-trap
  delete/despawn helper before deleting the owner, so delayed gathering/goober-style despawns do not
  leave a represented linked trap visible. Focused test coverage extends
  `process_pending_expires_gameobject_despawn_delay_like_cpp_update` with linked-trap state and
  fanout assertions. Remaining gaps: real map-owned remove-list lifecycle from the session runtime,
  `m_despawnRespawnTime`/`SaveRespawnTime`/DB/PoolMgr nuances, capture-point packet runtime,
  scripts/GO AI, ObjectAccessor/session fanout beyond represented packets, and live client/server
  validation.
- 2026-06-04 — Canonical remove-list same-pass linked-trap drain `#NEXT.RUNTIME.L3.031ds`:
  contrasted against C++ `Map::RemoveAllObjectsInRemoveList`, which uses
  `while (!i_objectsToRemove.empty())` (`Map.cpp:2574-2646`), and `GameObject::RemoveFromWorld`,
  which calls `GetLinkedTrap()->DespawnOrUnsummon()` before object-store removal
  (`GameObject.cpp:926-948`). Rust now processes `objects_to_remove` until empty instead of
  draining a snapshot, and canonical `GameObject::RemoveFromWorld` linked-trap cleanup routes the
  trap through `gameobject_delete_like_cpp`/remove-list rather than direct physical removal. Focused
  tests assert direct `remove_from_map_like_cpp` leaves linked traps queued, while
  `remove_all_objects_in_remove_list_like_cpp` removes both owner and newly queued trap in the same
  call. Remaining gaps: full PoolMgr/DB/capture-point packet runtime, real ObjectAccessor/session
  fanout, scripts/GO AI, and live client/server validation.
- 2026-06-04 — Capture-point delete packet serialization `#NEXT.RUNTIME.L3.031dt`: contrasted
  against C++ `GameObject::Delete`, which sends `WorldPackets::Battleground::CapturePointRemoved`
  for `GAMEOBJECT_TYPE_CAPTURE_POINT` before `SendGameObjectDespawn` (`GameObject.cpp:1740-1764`),
  and `CapturePointRemoved::Write`, whose payload is only `CapturePointGUID`
  (`BattlegroundPackets.cpp:436-439`). Rust now has a `wow-packet` `CapturePointRemoved` packet that
  writes exactly opcode placeholder `0xBADD` plus the raw GUID. Because the legacy C++ opcode table
  gives both update and remove capture-point packets the same placeholder, Rust reuses
  `ServerOpcodes::UpdateCapturePoint` intentionally and keeps the payload type distinct. Remaining
  gaps: connecting canonical `GameObjectDeleteOutcomeLikeCpp.capture_point_packet_represented` to
  `SendIfVisibleLikeCpp`/session fanout, confirming the real client opcode if/when available,
  PoolMgr/DB delete branches, and live client/server validation.
- 2026-06-04 — Pooled `GameObject::Delete` compatibility branch `#NEXT.RUNTIME.L3.031du`:
  contrasted against C++ `GameObject::Delete`, where `m_respawnCompatibilityMode && poolid` calls
  `sPoolMgr->UpdatePool<GameObject>(..., poolid, GetSpawnId())` instead of `AddObjectToRemoveList`
  (`GameObject.cpp:1759-1763`), and against `PoolMgr::UpdatePool<T>` dispatch
  (`PoolMgr.cpp:891-905`). Rust now has a bounded map-owned
  `Map::gameobject_delete_with_pool_update_like_cpp` helper: the non-pooled/default branch still
  queues the remove-list, while the pooled compatibility branch consumes the existing
  `PoolMgrLikeCpp::update_pool_plan_like_cpp` plan, mutates map-owned `SpawnedPoolDataLikeCpp`, and
  applies only already-represented safe map-local side effects. Focused tests assert pooled delete
  updates PoolMgr state without remove-list and non-compatibility delete keeps remove-list behavior.
  Remaining gaps: startup/runtime wiring of this helper from live delete call sites, DB save/delete
  effects, DB-backed entity fabrication, capture-point/session fanout, and live client/server
  validation.
- 2026-06-04 — Pool-aware `GameObject::Update` delete seam `#NEXT.RUNTIME.L3.031dv`: contrasted
  against the `GameObject::Update` `GO_JUST_DEACTIVATED` branches that call `Delete()` for
  owner/spell-created expired objects (`GameObject.cpp:1625-1636`) and the subsequent pooled
  `GameObject::Delete` branch (`GameObject.cpp:1759-1763`). Rust now keeps
  `update_game_object_like_cpp` / `update_game_objects_like_cpp` as no-metadata wrappers, adds
  pool-aware variants that pass `SpawnStore` + `PoolMgrLikeCpp`, and routes update-time `Delete()`
  consumption through the pooled helper when metadata is supplied. `GameObjectUpdateStatusLikeCpp`
  now distinguishes `DespawnPoolUpdated` from direct remove-list queueing, so tests do not falsely
  report `AddObjectToRemoveList` when C++ would call `PoolMgr::UpdatePool`. Focused tests cover an
  owner-created expired pooled GameObject reaching `DespawnPoolUpdated` without direct remove-list,
  while existing no-metadata update/delete tests preserve prior behavior. Remaining gaps:
  `ManagedMap`/`MapManager`/`world-server` metadata wiring, DB save/delete effects, DB-backed
  entity fabrication, capture-point/session fanout, and live client/server validation.
- 2026-06-05 — Canonical map manager pool metadata wiring `#NEXT.RUNTIME.L3.031dw`: follow-up to
  `031dv`, still anchored to C++ `GameObject::Update` owner/spell-created `Delete()` branches
  (`GameObject.cpp:1625-1636`), pooled `GameObject::Delete` (`GameObject.cpp:1759-1763`), and
  `PoolMgr::UpdatePool<T>` (`PoolMgr.cpp:891-905`). Rust now carries `SpawnStore` +
  `PoolMgrLikeCpp` through `ManagedMap::update_with_pool_update_context`,
  `MapManager::update_with_pool_update_context`, and the synchronous `MapUpdater` schedule path,
  while preserving no-metadata `update()` wrappers. `world-server`'s canonical map update tick now
  supplies `CanonicalSpawnMetadataLikeCpp::spawn_store()` and `pool_mgr_like_cpp()`, making the
  represented pooled GameObject update/delete branch reachable from the real canonical loop.
  Focused manager tests cover both the old no-metadata remove-list path and the pool-aware
  `DespawnPoolUpdated` path. Remaining gaps: full PoolMgr live despawn/fabrication is not yet
  represented in the manager path (pool-data can advance while the old live record remains), DB
  save/delete effects, capture-point/session fanout, and live client/server validation.
- 2026-06-05 — Loaded-grid PoolMgr GameObject delete/update side effects `#NEXT.RUNTIME.L3.031dx`:
  follow-up to `031dw`, contrasted against C++ `GameObject::Delete` pooled branch
  (`GameObject.cpp:1759-1763`), `PoolMgr::UpdatePool<T>` (`PoolMgr.cpp:891-905`),
  `PoolGroup<GameObject>::SpawnObject` / loaded-grid `Spawn1Object` (`PoolMgr.cpp:289-379`),
  `Despawn1Object<GameObject>` (`PoolMgr.cpp:241`), and `Map::AddToMap` (`Map.cpp:530-570`).
  Rust now preserves the previous no-loader safe-map-action wrappers and adds loaded-grid-record
  variants for `gameobject_delete_with_pool_update_like_cpp` and
  `update_game_object(s)_with_pool_update_like_cpp`. Update-time delete uses the same optional
  loader path, so `GameObject::Update -> Delete -> PoolMgr::UpdatePool<GameObject>` can load a
  replacement `MapObjectRecord::GameObject` on a loaded grid instead of only advancing
  `SpawnedPoolDataLikeCpp`. Focused tests cover both direct delete and the real update path.
  Remaining gaps: `MapManager`/`world-server` still need to pass the DB-backed GameObject
  loaded-grid loader into these variants; DB save/delete effects, capture-point/session fanout,
  full object lifecycle side effects, and live client/server validation remain open.
- 2026-06-05 — Canonical manager/world-server GameObject loaded-grid pool-update loader wiring
  `#NEXT.RUNTIME.L3.031dy`: follow-up to `031dx`, still anchored to C++ `GameObject::Delete`
  pooled branch (`GameObject.cpp:1759-1763`), `PoolMgr::UpdatePool<T>` (`PoolMgr.cpp:891-905`),
  `PoolGroup<GameObject>::Spawn1Object` (`PoolMgr.cpp:368-379`), and `Map::AddToMap`
  (`Map.cpp:530-570`). Rust now exposes `ManagedMap`/`MapManager`/`MapUpdater` variants that
  accept a loaded-grid record loader and `world-server` passes the existing DB-backed
  `build_loaded_grid_gameobject_respawn_record_like_cpp` from the canonical map update tick.
  Focused manager coverage proves the live manager path can materialize a replacement pooled
  GameObject on a loaded grid, and now verifies the trigger is removed through represented
  `DespawnOne -> remove_from_map_like_cpp` when its spawn-id index is populated before `AddToMap`,
  matching the C++ `Despawn1Object<GameObject>` lookup by spawn id. The old no-loader context still
  advances pool data without materializing the replacement. Remaining gaps: DB save/delete effects,
  capture-point/session fanout, broader `GameObject::Delete` side effects outside this bounded
  seam, full object lifecycle side effects, and live client/server validation remain open.
- 2026-06-05 — Canonical `GameObject::Delete` capture-point/despawn packet delivery
  `#NEXT.RUNTIME.L3.031dz`: contrasted against C++ `GameObject::Delete` (`GameObject.cpp:1740-1764`),
  `SendGameObjectDespawn` (`GameObject.cpp:1766-1771`), `GameObjectDespawn::Write`
  (`GameObjectPackets.cpp:30-34`), and `CapturePointRemoved::Write`
  (`BattlegroundPackets.cpp:436-439`). Rust now records delete-time `CapturePointRemoved` GUIDs in
  `GameObjectsUpdateSummaryLikeCpp`, propagates `Delete()->SendGameObjectDespawn()` through the
  existing visual-despawn summary, and has `WorldSession::process_pending` deliver capture-point
  removal before visual despawn. Because `MapManager::update` may drain `DelayedUpdate` before the
  session consumes the summary, deleted GO routing falls back to represented `HaveAtClient` when the
  canonical record has already been physically removed. Follow-up audit `#NEXT.RUNTIME.L3.031e0`
  confirmed that both the live legacy C++ opcode table and archived TrinityCore source leave
  `SMSG_CAPTURE_POINT_REMOVED` as `0xBADD`/`UNKNOWN_OPCODE`; Rust therefore keeps the distinct packet
  type and GUID-only payload without inventing a client opcode. Remaining gaps: exact
  ObjectAccessor/cell traversal, DB save/delete effects, and live client/server validation.
- 2026-06-05 — Capture-point removed opcode audit `#NEXT.RUNTIME.L3.031e0`: contrasted
  `SMSG_CAPTURE_POINT_REMOVED` and `SMSG_UPDATE_CAPTURE_POINT` against legacy C++
  `Opcodes.h:1920/2152` plus archived TrinityCore `Opcodes.h:1779/1916`. Both sources leave these
  battleground packets on placeholder/unknown opcodes. Rust's `CapturePointRemoved` serializer keeps
  a separate packet type and the C++ GUID-only `Write()` payload, but intentionally reuses the only
  available `ServerOpcodes::UpdateCapturePoint = 0xBADD` placeholder because Rust enums cannot expose
  duplicate discriminants. This closes the "confirm non-placeholder opcode" TODO as unavailable from
  current C++ sources; a real opcode would require client-sniff/opcode-table research outside the
  port source-of-truth.
- 2026-06-05 — Map-owned `RemoveFromOwner` local owner cleanup `#NEXT.RUNTIME.L3.031e1`: contrasted
  C++ `GameObject::Delete -> RemoveFromOwner` (`GameObject.cpp:1740-1764`, `GameObject.cpp:880-897`)
  and `Unit::RemoveGameObject(GameObject*, false)` (`Unit.cpp:5213-5245`) plus `Unit.h` storage
  (`m_ObjectSlot`, `m_gameObj`). Rust now represents the local owner state needed by that call:
  `ControlSubsystem` carries `gameobject_slots` and `owned_gameobjects`, and map-owned
  `gameobject_remove_from_owner_like_cpp` clears the GO owner, removes the GO from the owner list,
  and clears the first matching object slot for Player/Creature/Pet owners. This also fixes the
  map-local Unit resolver to include Player, matching C++ `ObjectAccessor::GetUnit`. Remaining gaps:
  `RemoveAurasDueToSpell`, cooldown event emission, `CreatureAI::SummonedGameobjectDespawn`,
  automatic owner-list population at all creation call sites, DB effects, and live client/server
  validation.
- 2026-06-05 — Map-owned `RemoveFromOwner` aura cleanup `#NEXT.RUNTIME.L3.031e2`: contrasted C++
  `Unit::RemoveGameObject(GameObject*, false)` spell branch (`Unit.cpp:5229-5233`) and
  `Unit::RemoveAurasDueToSpell` filtering (`Unit.cpp:3806-3819`). Rust now adds a bounded
  `AuraSubsystem::remove_auras_due_to_spell_like_cpp` helper and invokes it from map-owned
  `gameobject_remove_from_owner_like_cpp` when the resolved owner exists and the GameObject has a
  non-zero spell id. This records the branch and removes represented applied aura refs matching
  spell id / optional caster / required effect mask plus their matching represented owned-aura base
  refs when present. Scope boundary: this is not full C++
  `Aura::Remove`; aura scripts/procs/update-field packet fanout, cooldown event emission,
  `CreatureAI::SummonedGameobjectDespawn`, creation-time owner-list population, DB effects, and live
  client/server validation remain open.
- 2026-06-05 — Map-owned `RemoveFromOwner` creature-AI despawn callback boundary
  `#NEXT.RUNTIME.L3.031e3`: contrasted C++ `Unit::RemoveGameObject(GameObject*, false)` AI callback
  branch (`Unit.cpp:5241-5244`) and the virtual callback declaration (`CreatureAI.h:118-119`). Rust
  now records the represented `SummonedGameobjectDespawn` dispatch boundary for Creature/Pet owners
  whose `AiSubsystem` is enabled, incrementing a local evidence counter. This intentionally does not
  execute script AI or SmartAI behavior; cooldown event emission, creation-time owner-list
  population, DB effects, real script callback runtime, and live client/server validation remain
  open.
- 2026-06-05 — Map-owned `Unit::AddGameObject` owner-link boundary
  `#NEXT.RUNTIME.L3.031e4`: contrasted C++ `Unit::AddGameObject(GameObject*)` (`Unit.cpp:5192-5209`),
  `WorldObject::SummonGameObject` (`Object.cpp:2067-2090`), slot summon setup
  (`SpellEffects.cpp:3541-3597`), and transmitted GO special cases (`SpellEffects.cpp:4441-4512`).
  Rust now exposes `gameobject_add_to_owner_like_cpp` as a map-owned, session-free representation of
  the `AddGameObject` guard: only Unit-like owners and ownerless typed GameObjects register
  `owned_gameobjects`, set `CreatedBy`, and record the Creature/Pet `JustSummonedGameobject` callback
  boundary when AI is enabled. Slot assignment remains caller-specific because C++
  `Unit::AddGameObject` does not write `m_ObjectSlot`; `Spell::EffectSummonObject` writes the slot
  after `Map::AddToMap`. Cooldown start remains blocked, not implemented: Rust lacks a runtime
  equivalent of `SpellInfo::IsCooldownStartedOnEvent()` over both spell attributes and category
  flags, plus Trinity-equivalent category-aware `SpellHistory::StartCooldown` semantics.
- 2026-06-05 — Map-owned `EffectSummonObject` owner-slot tail boundary
  `#NEXT.RUNTIME.L3.031e5`: contrasted C++ `Spell::EffectSummonObject` slot handling
  (`SpellEffects.cpp:3541-3597`). Rust now exposes `gameobject_add_to_owner_slot_like_cpp`, which
  reuses the `Unit::AddGameObject` owner-link representation and then writes the requested
  `m_ObjectSlot[slot]` equivalent only when the owner-link guard succeeds. This is intentionally the
  post-create tail only: it does not clear/delete the previous slot occupant, create the
  GameObject, compute spell duration/location, inherit phase, call `Map::AddToMap`, execute scripts,
  or send packets.
- 2026-06-05 — Map-owned `Unit::RemoveGameObject(spellid, del)` boundary
  `#NEXT.RUNTIME.L3.031e6`: contrasted C++ spell-id overload (`Unit.cpp:5253-5274`) and its
  channeled-spell cancellation caller (`Spell.cpp:3621-3625`). Rust now exposes
  `unit_remove_gameobjects_by_spell_like_cpp`, filtering the owner `owned_gameobjects` list by spell
  id (`0` means all), clearing each matched GameObject owner, optionally setting respawn time to zero
  and routing through the represented `GameObject::Delete` seam, then erasing matched owner-list
  entries. The outcome deliberately reports no slot cleanup, aura cleanup, cooldown event, or
  CreatureAI despawn callback because this C++ overload does not perform those pointer-overload side
  effects. Tests preserve the C++ quirk that `m_ObjectSlot` can remain pointing at a removed
  spell-owned GameObject.
- 2026-06-05 — Map-owned `EffectSummonObject` old-slot cleanup boundary
  `#NEXT.RUNTIME.L3.031e7`: contrasted C++ `Spell::EffectSummonObject` pre-create cleanup
  (`SpellEffects.cpp:3548-3563`) and pointer-overload `Unit::RemoveGameObject(GameObject*, true)`
  (`Unit.cpp:5213-5251`). Rust now exposes
  `gameobject_prepare_owner_slot_for_summon_like_cpp`, which reads the owner's represented
  `m_ObjectSlot[slot]`, nulls the old GameObject spell id for exact recasts before owner removal,
  routes through the pointer-overload owner cleanup, sets respawn time to zero, queues represented
  `GameObject::Delete`, and clears the slot even when the referenced GameObject is missing. Tests
  cover the C++ recast aura-preservation branch, different-spell aura cleanup, and missing-GUID slot
  clear. Scope remains pre-create only: no new GameObject creation, phase inheritance,
  `Map::AddToMap`, final slot write, scripts, packets, DB, or cooldown event emission.
- 2026-06-05 — Map-owned `EffectSummonObject` create/add/slot body boundary
  `#NEXT.RUNTIME.L3.031e8`: contrasted C++ `Spell::EffectSummonObject` create body
  (`SpellEffects.cpp:3565-3597`) and `GameObject::Create` (`GameObject.cpp:179-229`). Rust now
  exposes `gameobject_summon_object_for_owner_slot_like_cpp`, which consumes an already-resolved
  GameObject template, destination and duration, generates a map-owned GameObject GUID, creates a
  ready dynamic spell-owned GameObject, copies caster faction/level, applies respawn seconds and
  spell id, performs the represented `Unit::AddGameObject` owner link before `Map::AddToMap`, then
  writes the represented owner slot after AddToMap. Tests cover the successful create/add/slot path
  and the missing-owner guard without low-GUID consumption. Scope remains bounded: no DB/template
  lookup, spell target resolution/GetClosePoint, real phase inheritance, scripts, packets, or
  cooldown event emission.
- 2026-06-05 — Map-owned `WorldObject::SummonGameObject` generic boundary
  `#NEXT.RUNTIME.L3.031e9`: contrasted C++ `WorldObject::SummonGameObject` (`Object.cpp:2067-2090`),
  `GOSummonType` (`ObjectDefines.h:81-85`) and `GameObject::CreateGameObject`
  (`GameObject.cpp:1187-1200`). Rust now exposes `GameObjectSummonTypeLikeCpp` and
  `world_object_summon_gameobject_like_cpp`, consuming an already-resolved template, position and
  respawn seconds, requiring the summoner to be in world before low-GUID consumption, creating a
  ready dynamic GameObject, representing the pending phase-inherit boundary, linking owner before
  `Map::AddToMap` only for Player or Unit + `TimedOrCorpseDespawn`, and otherwise forcing
  `spawned_by_default=false`. Tests cover the player owner branch, Unit + `TimedDespawn` non-owner
  branch, and not-in-world/no-low-guid-consumption. Scope remains bounded: no DB/template lookup,
  spell target resolution/GetClosePoint, real phase inheritance, scripts, packets, linked traps, or
  spell execute logs.
- 2026-06-05 — `WorldObject::SummonGameObject` coordinate-overload position resolver
  `#NEXT.RUNTIME.L3.031ea`: contrasted C++ coordinate overload (`Object.cpp:2096-2105`),
  `GetClosePoint` (`Object.cpp:3405-3408`) and `GetNearPoint2D/GetNearPoint`
  (`Object.cpp:3341-3398`). Rust now exposes
  `world_object_summon_gameobject_position_from_coords_like_cpp`, preserving explicit coordinates
  and resolving the `x=y=z=0` fallback from summoner position/orientation with effective distance
  `2 * combatReach`, matching the `GetClosePoint(..., GetCombatReach())` plus `searcher=nullptr`
  `GetNearPoint2D` composition. It applies map-coordinate normalization and explicitly reports that
  height correction, collision detection and LOS fallback search are still runtime gaps. Tests cover
  explicit coordinates plus X/Y-axis close-point fallback.
- 2026-06-05 — Map-owned `EffectSummonObjectWild` body boundary
  `#NEXT.RUNTIME.L3.031eb`: contrasted C++ `Spell::EffectSummonObjectWild`
  (`SpellEffects.cpp:2937-2986`) and `GameObject::GetLinkedTrap` (`GameObject.cpp:3909-3912`).
  Rust now exposes `spell_effect_summon_object_wild_like_cpp`, consuming an already-resolved caster,
  template, destination, spell id and duration, creating a ready dynamic spell GameObject without
  owner linkage, setting respawn seconds and spell id, and representing the execute-log boundary
  when `Map::AddToMap` succeeds. It records the FlagDrop player branch but does not claim
  Battleground `SetDroppedFlagGUID`; linked-trap phase/respawn/spell/log side effects remain
  unrepresented until Rust has real linked-trap creation/resolution for this path. Tests cover
  normal wild summon, FlagDrop branch evidence, and missing-caster/no-low-guid-consumption.
- 2026-06-05 — DB-backed GameObject template lifecycle bridge for spell-created GOs
  `#NEXT.RUNTIME.L3.031ec`: contrasted C++ `ObjectMgr::LoadGameObjectTemplate`
  (`ObjectMgr.cpp:7552-7610`), `LoadGameObjectTemplateAddons` (`ObjectMgr.cpp:7770-7854`) and
  `GameObject::CreateGameObject` (`GameObject.cpp:1187-1200`). Rust now exposes
  `wow_data::gameobject_template_lifecycle_record_like_cpp`, converting
  `GameObjectTemplateLifecycleRecordLikeCpp` plus optional template addon into the
  `wow_entities::GameObjectTemplateLifecycleRecord` consumed by spell-created dynamic GO helpers.
  The bridge applies addon faction/flags/worldEffect/animKit, maps `ContentTuningId` to the
  represented GameObject level, preserves template data/name/display/size, and intentionally does
  not apply spawn overrides because `CreateGameObject` for spell-created GOs has no DB spawn row.
  Tests cover with/without addon. Scope remains bounded: no spell handler wiring, target/duration
  resolution, map mutation, linked-trap creation, scripts, or packets.
- 2026-06-05 — Session wiring for spell-created GameObject template lifecycle store
  `#NEXT.RUNTIME.L3.031ed`: contrasted C++ `Spell::EffectSummonObjectWild`
  (`SpellEffects.cpp:2937-2986`) and `Spell::EffectSummonObject`
  (`SpellEffects.cpp:3541-3597`): both ultimately call `GameObject::CreateGameObject` with
  `effectInfo->MiscValue`, so live callers need the DB-backed template/addon lifecycle store.
  Rust now carries `GameObjectTemplateLifecycleStoreLikeCpp` through `SessionResources` into
  `WorldSession`, with a setter/getter and test coverage proving session callers can retrieve a
  stored template and convert it through `gameobject_template_lifecycle_record_like_cpp`. Scope
  remains bounded: no effect-handler wiring, target/fallback resolution, map mutation, packets,
  linked traps, or owner-slot cleanup.
- 2026-06-05 — Dormant session resolver for `EffectSummonObjectWild`
  `#NEXT.RUNTIME.L3.031ee`: contrasted C++ `Spell::EffectSummonObjectWild`
  (`SpellEffects.cpp:2937-2986`), `SpellInfo::CalcDuration` (`SpellInfo.cpp:3894-3910`) and
  `GameObject::CreateGameObject` (`GameObject.cpp:1187-1200`). Rust adds spell-effect constants
  for wild/slot summons and `WorldSession::apply_effect_summon_object_wild_like_cpp`, which is
  deliberately not connected to the live spell loop yet. The resolver validates effect 76, resolves
  `effect_misc_value_1` through the session's DB-backed GameObject template lifecycle store,
  converts template+addon, computes duration through `SpellMiscStore`/`SpellDurationStore`, requires
  an explicit destination, locates the caster's canonical map, and delegates to the map-owned
  `spell_effect_summon_object_wild_like_cpp` body. Tests cover non-wild no-op, missing template,
  missing destination, and explicit-destination creation with respawn/spell id. Scope remains
  bounded: no live handler wiring, no no-destination fallback/focusObject orientation, no real phase
  inheritance, linked trap, battleground flag mutation, packets, or scripts.
- 2026-06-05 — No-destination fallback for dormant `EffectSummonObjectWild`
  `#NEXT.RUNTIME.L3.031ef`: contrasted C++ `Spell::EffectSummonObjectWild`
  (`SpellEffects.cpp:2946-2954`), `DEFAULT_PLAYER_BOUNDING_RADIUS` (`ObjectDefines.h:39`) and
  `WorldObject::GetClosePoint` / `GetNearPoint` (`Object.cpp:3341-3408`). Rust now exposes
  `DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP` and
  `spell_effect_summon_object_wild_position_like_cpp`: explicit destinations still win, while a
  missing destination resolves to the C++ close-point fallback at caster combat reach plus
  `DEFAULT_PLAYER_BOUNDING_RADIUS`, with map-coordinate normalization evidence. The dormant session
  resolver uses this caster fallback instead of rejecting missing `dst`, and tests prove it creates
  the canonical spell GameObject at that fallback position. Scope remains bounded: focusObject
  selection/orientation is not fully represented beyond the caster fallback, and height correction,
  collision/LOS/terrain search, phase inheritance, linked traps, battleground state, packets/scripts,
  and live spell-loop wiring remain open. The C++ `DEFAULT_PLAYER_BOUNDING_RADIUS` comment is
  suspicious (`correctly?`), but this slice ports the legacy behavior literally before any separate
  bug-fix decision.
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
