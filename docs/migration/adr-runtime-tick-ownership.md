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
    `(map_id, instance_id)`. Dormant (no production caller; `run_creatures_tick` still uses the
    session field). 6 tests; 1074/0. 1 code file.
  - **4A.2b — DONE (`#NEXT.RUNTIME.L3.004`):** repointed `run_creatures_tick` to the map queue via
    session helpers `push_map_respawn_like_cpp`/`drain_ready_map_respawns_like_cpp` (lock only for
    push/drain, released before building packets / `register_world_creature`), removed
    `WorldSession::respawn_queue`. Byte-identical for 1 session (reviewer confirmed the drain logic
    and packet-build loop are unchanged). 3 tests; wow-world 1077/0. **4A.2 complete.**
- **4A.3 (higher risk, gated OFF):** separate legacy creature-tick driver (NOT hooked into the
  canonical loop) that ticks creatures once per map, builds a `RuntimePlan` under the lock, releases
  the lock, resolves recipients, and delivers via `try_send`. Owner stays `Session` in production;
  `GlobalLegacy` only in tests.
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
  - **4A.3b — NEXT:** the separate legacy driver task (gated OFF; spawned only in an integration
    test) that iterates `active_map_keys`, calls the movement helper under the RwLock, releases,
    and delivers `MonsterMove` via `deliver_runtime_plan_like_cpp` (NearbyVisible, try_send). Verify
    two sessions: moved once, fanned out to both.
  - **4A.3c — LATER (own decision):** create/destroy from the global owner — needs a per-session
    creature-visibility scan (the C++ `UpdateVisibilityOf` seam); the current incremental
    `client_visible_guids` model (set mutated on each individual CREATE/DESTROY, ~50 sites) cannot be
    driven from a session-less emitter. NOT bundled with 4A.3a/b.
- **4A.4:** flip `GlobalLegacy` in an integration test only.
- **4B:** production flip + manual client/server verification (first real manual test).

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
  reusable by the future driver. 3 tests; wow-world 1080/0. NEXT: 4A.3b (driver task, gated OFF).

## References

- `crates/wow-world/src/session.rs` — `tick_creatures_sync`, `tick_combat_sync`, creature wrappers; `client_visible_guids_like_cpp` (HashSet, :2312); `process_represented_session_commands_like_cpp` (:12004).
- `crates/wow-network/src/player_registry.rs` — `SessionCommand` enum (:19), `PlayerBroadcastInfo`/`PlayerRegistry`.
- `crates/wow-map/src/manager.rs` — `MapManager::update` / `ManagedMap::update` (mirrors `Map::Update`).
- `crates/world-server/src/main.rs` — both managers + `spawn_canonical_map_update_loop`.
- C++: `World.cpp:2748` (`sMapMgr->Update`), `Map.cpp:666` (`Map::Update` phase order).
- `docs/migration/honest-progress-audit.md`, `crates/wow-world/_attic/README.md` (big-bang lesson).
