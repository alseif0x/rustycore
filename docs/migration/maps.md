# Migration: Maps

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Maps/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-map/`
> **Layer:** L3 (World layer)
> **Status:** 🔧 broken (rewrite needed)
> **Audited vs C++:** ❌ confirmed broken — 2026-05-01 audit; no Map class, no NGrid/Cell hierarchy, no GridState machine, no Map::Update(), no MapManager::update()
> **Last updated:** 2026-05-01

---

## 1. Purpose

The Maps module in TrinityCore manages the world's spatiotemporal state: loading/unloading grids, tracking entities (players, creatures, GameObjects, corpses) within cells, terrain visibility queries, and coordinate transformations. It bridges the gap between a player's in-game position and the Grid/Cell hierarchy that lazily loads object spawns. It is the foundational memory manager for all spatial queries and respawn timers.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Maps/AreaBoundary.cpp` | 107 | `prefix` |
| `game/Maps/AreaBoundary.h` | 167 | `prefix` |
| `game/Maps/GridMap.cpp` | 694 | `prefix` |
| `game/Maps/GridMap.h` | 103 | `prefix` |
| `game/Maps/Map.cpp` | 4014 | `prefix` |
| `game/Maps/Map.h` | 917 | `prefix` |
| `game/Maps/MapManager.cpp` | 461 | `prefix` |
| `game/Maps/MapManager.h` | 183 | `prefix` |
| `game/Maps/MapObject.h` | 60 | `prefix` |
| `game/Maps/MapRefManager.h` | 40 | `prefix` |
| `game/Maps/MapReference.cpp` | 39 | `prefix` |
| `game/Maps/MapReference.h` | 40 | `prefix` |
| `game/Maps/MapScripts.cpp` | 899 | `prefix` |
| `game/Maps/MapUpdater.cpp` | 123 | `prefix` |
| `game/Maps/MapUpdater.h` | 65 | `prefix` |
| `game/Maps/SpawnData.h` | 128 | `prefix` |
| `game/Maps/TerrainMgr.cpp` | 877 | `prefix` |
| `game/Maps/TerrainMgr.h` | 167 | `prefix` |
| `game/Maps/TransportMgr.cpp` | 714 | `prefix` |
| `game/Maps/TransportMgr.h` | 185 | `prefix` |
| `game/Maps/ZoneScript.cpp` | 40 | `prefix` |
| `game/Maps/ZoneScript.h` | 104 | `prefix` |
| `game/Maps/enuminfo_SpawnData.cpp` | 67 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Maps/Map.h` | 917 | Map class definition, grid management interface, player/object tracking |
| `src/server/game/Maps/Map.cpp` | 4014 | Map lifecycle (ctor/dtor), entity add/remove, grid load/unload state machine, Update() loop |
| `src/server/game/Maps/MapManager.h` | 183 | MapManager singleton definition; instance creation, lookup, threading |
| `src/server/game/Maps/MapManager.cpp` | 461 | MapManager implementation: CreateMap, CreateInstance, CreateBattleground, Update(), threading setup |
| `src/server/game/Maps/GridMap.h` | 103 | GridMap wrapper for terrain tile querying (height, liquid, flags) |
| `src/server/game/Maps/GridMap.cpp` | 694 | GridMap impl: height queries, water/liquid detection, area trigger checks |
| `src/server/game/Maps/MapUpdater.h` | 65 | Multithreaded map update executor (thread pool pattern) |
| `src/server/game/Maps/MapUpdater.cpp` | 123 | MapUpdater activation, thread spawning, work distribution |
| `src/server/game/Maps/TerrainMgr.h` | 167 | Terrain manager for VMap/ADT queries, visibility, pathfinding |
| `src/server/game/Maps/TerrainMgr.cpp` | 877 | VMap integration, ADT loading, line-of-sight, height sampling |
| `src/server/game/Maps/TransportMgr.h` | 185 | Static/dynamic transports (boats, elevators) lifecycle |
| `src/server/game/Maps/TransportMgr.cpp` | 714 | Transport spawning, path following, passenger handling |
| `src/server/game/Maps/AreaBoundary.h` | 167 | Area boundary definitions for safe/PvP zone checks |
| `src/server/game/Maps/AreaBoundary.cpp` | 107 | Boundary queries: IsInAreaTrigger, CheckBoundary, area type masks |
| `src/server/game/Maps/MapScripts.cpp` | 899 | Script event hooks: OnCreate, OnDestroy, OnObjectEnter, OnObjectLeave |
| `src/server/game/Maps/MapObject.h` | 60 | Marker base class for objects entering a map (deprecated but present) |
| `src/server/game/Maps/MapReference.h` | 51 | Intrusive reference node for player presence tracking in Map |
| `src/server/game/Maps/MapRefManager.h` | 52 | Manager for MapReference doubly-linked list (player tracking) |
| `src/server/game/Maps/ZoneScript.h` | 104 | Base class for zone/instance scripting hooks |
| `src/server/game/Maps/SpawnData.h` | 128 | Spawn data POD struct (type, entry, position, guids) |
| `src/server/game/Maps/enuminfo_SpawnData.cpp` | 67 | Enum reflection for SpawnData serialization |

**Total C++ lines in Maps/:** ~10,194 (excluding blank/comments)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Map` | class | Main world/instance map container, inherits GridRefManager<NGridType> for player tracking |
| `MapManager` | class | Singleton managing all Map instances (world + instances); handles creation, lookup, threading |
| `MapEntry` | struct | DB2 record (Map.db2) — map ID, instance type, name, flags |
| `MapDifficultyEntry` | struct | DB2 record — difficulty mode per map (normal/heroic/mythic) |
| `TerrainInfo` | class | Terrain data (VMap, ADT, liquid) for a single map instance |
| `GridMap` | class | Single ADT tile terrain queries (height/water sampler) |
| `TransportInfo` | struct | Static/dynamic transport metadata (path, speed, seats) |
| `AreaBoundary` | class | 2D area boundary polygon for zone/PvP checks |
| `RespawnInfo` | struct | Respawn queue entry (type, entry, respawn time, gridId) |
| `ZoneDynamicInfo` | struct | Per-zone runtime state (music, weather, light overrides) |
| `GridInfo` | class | NGrid metadata (load timer, unload lock counter, visibility update timer) |
| `grid_state_t` | enum | Grid lifecycle state: INVALID, ACTIVE, IDLE, REMOVAL |
| `TransferAbortReason` | enum | Failure codes for instance entry attempts (max players, not found, locked, etc.) |
| `GridMapTypeMask` | enum | Bitmask for object type filtering (creature, gameobject, player, etc.) |
| `SpawnData` | struct | Generic spawn metadata (type, entry, position) — used for respawn tracking |
| `InstanceMap` | class | Map subclass for dungeons/raids (derives from Map; adds difficulty, instance lock) |
| `BattlegroundMap` | class | Map subclass for BGs/arenas (derives from Map; minimal scripting) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Map::AddPlayerToMap(Player*, bool)` | Adds player to map, assigns grid, broadcasts SMSG_NEW_WORLD | `Player::InitGroundPositionUpdateTime`, `Map::LoadGrid`, `WorldSession` |
| `Map::RemovePlayerFromMap(Player*, bool)` | Removes player from map, broadcasts SMSG_LOGOUT_COMPLETE | `GridRefManager::unlink` |
| `Map::AddToMap<T>(T*)` | Generic add object to grid + cell | `Object::SetMap`, `Grid::AddWorldObject` or `Grid::AddGridObject` |
| `Map::RemoveFromMap<T>(T*, bool)` | Generic remove object from grid + cell | `Object::RemoveFromWorld`, `Grid` visitor |
| `Map::LoadGrid(float, float)` | Explicitly load a grid cell at coords, fetch spawns from DB | `ObjectGridLoader::LoadN`, `Map::LoadGridForActiveObject` |
| `Map::LoadGridForActiveObject(float, float, WorldObject*)` | Load grid if object has active spawn point (prevents clone loading) | `Map::LoadGrid` |
| `Map::LoadAllCells()` | Preload all grids on map (used for instances) | Called by MapManager on instance create if CONFIG_INSTANCEMAP_LOAD_GRIDS |
| `Map::UnloadGrid(NGridType&, bool)` | Unload a single grid, despawn entities, clear from container | `GridStates::RemovalState::Update`, `Map::UnloadAll` |
| `Map::UnloadAll()` | Unload all grids and remove all objects | Called on map destruction or shutdown |
| `Map::IsGridLoaded(GridCoord)` | Query if a grid is currently loaded | Visibility checks, entity relocation |
| `Map::IsRemovalGrid(float, float)` | Check if grid is in REMOVAL state (about to unload) | Entity movement validation |
| `Map::Update(uint32)` | Per-frame map tick; updates all grids, objects, respawns | MapUpdater, main world loop |
| `Map::PlayerRelocation(Player*, x, y, z, o)` | Player moved; update grid cell, broadcast to nearby | `WorldSession::HandleMovement` |
| `Map::CreatureRelocation(Creature*, x, y, z, a, bool)` | Creature moved; update grid, respawn lock logic | `Creature::Update`, `MotionMaster::OnPathFinished` |
| `Map::GameObjectRelocation(GameObject*, x, y, z, o, bool)` | GameObject moved; update grid | `Transport::UpdatePassengers` |
| `Map::VisitNearbyCellsOf(WorldObject*, gridVisitor, worldVisitor)` | Iterate nearby entities in adjacent cells (9x9 grid) | Spell AoE, threat updates, visibility broadcasts |
| `Map::Visit<T, CONTAINER>(Cell, visitor)` | Visit single cell container (grid or world objects) | GridNotifiers, spell effects |
| `Map::GetVisibilityRange()` | Returns m_VisibleDistance (default ~100 yards) | Client rendering, entity interest |
| `Map::InitVisibilityDistance()` | Compute visibility range per map type | MapManager::InitializeVisibilityDistanceInfo |
| `Map::GetZoneId(PhaseShift, x, y, z)` | Lookup zone ID at position (terrain-based) | Phasing, quest validation |
| `Map::GetAreaId(PhaseShift, x, y, z)` | Lookup area ID at position (finer than zone) | Talent restriction checks |
| `Map::GetHeight(PhaseShift, x, y, z, vmap, maxDist)` | Ground/gameobject floor height at position | Fall damage, mount height |
| `Map::GetWaterLevel(PhaseShift, x, y)` | Water surface height at 2D position | Swimming validation |
| `MapManager::CreateMap(uint32, Player*)` | Create or fetch map instance for player (singleton per map+instance) | World::TransferPlayer |
| `MapManager::FindMap(uint32, uint32)` | Look up existing Map by ID + instance ID | Session::HandleTransferRequest |
| `MapManager::Update(uint32)` | Main world tick; dispatches to all Map::Update() | Main server loop |
| `MapManager::Initialize()` | Init state machine, spawn worker threads | Server startup |

---

## 5. Module dependencies

**Depends on:**
- `Grids` — stores NGridType, grid state machine (GridStates), ObjectGridLoader for cell loading
- `Entities` (Player, Creature, GameObject, DynamicObject, AreaTrigger, Corpse, Pet, Transport) — entities stored in grids + cells; Map is container
- `Database` (WorldDatabase, CharacterDatabase) — loads spawns, respawn times, corpse data from DB
- `Data` (DB2Stores: MapStore, MapDifficultyStore, AreaTriggerStore, LiquidStore) — map metadata, area info
- `TerrainMgr` — terrain queries (height, liquid, vmap, visibility)
- `SharedDefines` — enum definitions (Difficulty, SpawnObjectType, TransferAbortReason)
- `ObjectAccessor` — object lookups by GUID
- `ScriptMgr` — script callbacks (OnObjectEnter, OnObjectLeave, OnDestroy)
- `PhasingHandler` — zone/area visibility based on phase shifts

**Phasing contract:**
- `Map` must own the per-map `MultiPersonalPhaseTracker` equivalent and call its `LoadGrid` / `UnloadGrid` / `Update` from the same grid lifecycle points as C++ `Map.cpp`.
- `ObjectGridLoader` must register personal-phase spawns with the tracker instead of inserting them as globally visible regular grid objects.
- `VisitNearbyCellsOf`, `AddToMap`, relocation and object update broadcasts must consult `PhaseShift::CanSee` through the same visibility chain tracked in `phasing.md #PHASE.26`; DB-spawn session filtering alone is not sufficient.
- `Map::GetAreaId`, `GetZoneId`, height and liquid queries must accept the caller `PhaseShift` and route terrain lookup through `PhasingHandler::GetTerrainMapId` when visible-map terrain swaps are active.

**Depended on by:**
- `World` — creates maps, orchestrates map updates in main loop
- `WorldSession` — player->map transfer, movement packets parsed into Map::PlayerRelocation
- `Creature`, `Player`, `GameObject` — stored in Map grids/cells; all positional queries route through Map
- `TransportMgr` — uses Map::AddToMap/RemoveFromMap for transport spawning
- `OutdoorPvPMgr` — zone-based PvP checks routed through Map area lookups
- `BattlegroundMgr` — creates BattlegroundMaps
- `Instances` / `DungeonFinding` — creates InstanceMaps
- `Spells` / `Combat` — positional queries (nearby enemies, LoS checks)
- `Movement` / `MotionMaster` — relocation calls back to Map when creature moves

---

## 6. SQL / DB queries (if any)

Map does **not** directly emit SQL; all DB I/O is delegated to specialized loaders or managers.

| Source / Context | Purpose | Database |
|---|---|---|
| `ObjectGridLoader::LoadN()` → creature_template, creature, gameobject, gameobject_template | Spawn loading when grid transitions to ACTIVE | world |
| `Map::LoadRespawnTimes()` → creature_respawn, gameobject_respawn | Load respawn timers on map init | character |
| `Map::LoadCorpseData()` → corpse | Load player corpses on map init | character |
| `Map::InitSpawnGroupState()` → spawn_group, spawn_group_template | Load conditional spawn groups | world |
| `TerrainMgr::LoadTerrain()` → (implicit via VMap DLL, ADT binary files) | Terrain/vmap loading | filesystem (not SQL) |

**DB2 Stores accessed:**
| Store | What it loads | Read by |
|---|---|---|
| `MapStore` | Map.db2 (map IDs, names, type, PvP flag, corpse decay time) | MapManager::CreateMap, Map::GetEntry |
| `MapDifficultyStore` | MapDifficulty.db2 (per-map difficulty modes) | Map::GetMapDifficulty, MapManager::CreateInstance |
| `MapDifficultyXConditionStore` | MapDifficultyXCondition.db2 (ordered PlayerCondition gates per map difficulty) | Player::Satisfy transfer-abort difficulty checks |
| `AreaTriggerStore` | AreaTrigger.db2 (trigger points, radius, shape) | Map::GetAreaId, AreaBoundary checks |
| `LiquidStore` (implicit) | Liquid data per ADT tile | GridMap::GetLiquidStatus, Map::IsInWater |

---

## 7. Wire-protocol packets (if any)

Maps module **originates** very few packets; most are sent by Player, Creature, or Transport after Map relocation.

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_NEW_WORLD` | server → client | `Map::AddPlayerToMap` after grid load, broadcasts map ID + position |
| `SMSG_LOGOUT_COMPLETE` | server → client | `Map::RemovePlayerFromMap` after cleanup |
| `SMSG_TRANSFER_PENDING` | server → client | `Map::AddPlayerToMap` (instance enter, phasing transition) |
| `SMSG_TRANSFER_ABORTED` | server → client | `MapManager::CreateMap` (denied due to difficulty/lock/max players) |
| `SMSG_DESTROY_OBJECT` | server → client | `Map::RemoveFromMap` → Object broadcast to nearby |
| `SMSG_CREATE_OBJECT` (legacy) / `SMSG_UPDATE_OBJECT` | server → client | `Map::AddToMap` → broadcast to nearby cells (via VisitNearbyCells) |

The module also **processes** CMSG_MOVE_* packets indirectly via WorldSession → Map::PlayerRelocation.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |
| `crates/wow-map/src/coords.rs` | `file` | 1 | 254 | `exists_active` | file exists |
| `crates/wow-map/src/cell.rs` | `file` | 1 | 234 | `exists_active` | file exists |
| `crates/wow-map/src/lib.rs` | `file` | 1 | 70 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `crates/wow-world/src/map.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-map/src/coords.rs` — coordinate foundation ported from `GridDefines.h`; no Map/NGrid lifecycle yet.
- `crates/wow-map/src/cell.rs` — cell-level coordinate decomposition and typed GUID containers; no Map/NGrid lifecycle yet.
- `crates/wow-map/src/lib.rs` — `MapKey { map_id: u32, instance_id: u32 }`, matching C++ `MapManager::MapKey`.
- `crates/wow-world/src/map_manager.rs` — ~350 lines — **scaffold only**; basic MapManager skeleton, no grid state machine.
- `crates/wow-world/src/map.rs` (if exists) — not found; Map impl missing.
- `crates/wow-world/src/session.rs` (legacy) — 100+ lines — contains `WorldCreature` struct (creature AI state) **not a Map concern** but used in place of proper entity hierarchy

**What's implemented:**
- `GridCoord` struct with `surrounding()` method (basic neighbor grid calculation)
- Constants: `GRID_SIZE = 64.0`, `VISIBILITY_RADIUS = 100.0`, `DEFAULT_GRID_UNLOAD_TIME = 300s`
- `WorldCreature` POD in session (creature entry, HP, position, movement state, combat target) — **should be in Entity layer, not map_manager**
- MapManager singleton stub (no CreateMap, no FindMap logic)
- Bare `GridCoord` distance calculation

**What's missing vs C++:**
1. **No Map class** — no grid container, no entity tracking, no state machine
2. **No grid lifecycle** — no ACTIVE/IDLE/REMOVAL states; no lazy loading/unloading
3. **No ObjectGridLoader** — no spawning of creatures/GameObjects from DB on grid load
4. **No terrain queries** — no height/water/area lookups
5. **No player relocation** — no cell movement tracking or nearby entity broadcasts
6. **No respawn manager** — no respawn timer queue or timers
7. **No grid state machine** — the C++ GridStates (InvalidState, ActiveState, IdleState, RemovalState) classes **not ported**
8. **No visibility tracking** — no VisitNearbyCellsOf or cell-based entity interest
9. **No InstanceMap/BattlegroundMap subclasses** — no difficulty/lock handling
10. **No MapUpdater threading** — maps not updated in parallel threads
11. ~~No coordinate transformations~~ — `wow-map::coords` now has the TrinityCore formulas; still not wired into a real Map/NGrid/Cell model.
12. **Separated creature AI** — WorldCreature is jammed into session.rs instead of properly living in the map grid system

**Suspicious / likely divergent (pre-audit hypothesis from MIGRATION_ROADMAP.md):**
- The C++ model is: Map contains NGrids → NGrid contains Cells → Cell contains entities. Rust version has no hierarchical container model yet.
- WorldCreature in session.rs is a conflation of entity data + AI state; C++ separates Creature (Entity) from CreatureAI (behavior). RustyCore doesn't have proper polymorphic entity hierarchy.
- No lazy-load pattern; C++ grids load spawns on demand (GRID_STATE_INVALID → GRID_STATE_ACTIVE → load via ObjectGridLoader). Rust version has no equivalent.

**Tests existing:**
- 6 coordinate tests, 5 cell tests, and 2 `MapKey` tests in `crates/wow-map`.
- 0 Map lifecycle / NGrid / ObjectGridLoader tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MAPS.WBS.001** Cerrar la migracion auditada de `game/Maps/AreaBoundary.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/AreaBoundary.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.002** Cerrar la migracion auditada de `game/Maps/AreaBoundary.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/AreaBoundary.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.003** Partir y cerrar la migracion auditada de `game/Maps/GridMap.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/GridMap.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 694 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.004** Cerrar la migracion auditada de `game/Maps/GridMap.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/GridMap.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.005** Partir y cerrar la migracion auditada de `game/Maps/Map.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4014 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.006** Partir y cerrar la migracion auditada de `game/Maps/Map.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 917 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.007** Cerrar la migracion auditada de `game/Maps/MapManager.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapManager.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.008** Cerrar la migracion auditada de `game/Maps/MapManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapManager.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.009** Cerrar la migracion auditada de `game/Maps/MapObject.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapObject.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.010** Cerrar la migracion auditada de `game/Maps/MapRefManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapRefManager.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.011** Cerrar la migracion auditada de `game/Maps/MapReference.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapReference.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.012** Cerrar la migracion auditada de `game/Maps/MapReference.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapReference.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.013** Partir y cerrar la migracion auditada de `game/Maps/MapScripts.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 899 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.014** Cerrar la migracion auditada de `game/Maps/MapUpdater.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapUpdater.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.015** Cerrar la migracion auditada de `game/Maps/MapUpdater.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapUpdater.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.016** Cerrar la migracion auditada de `game/Maps/SpawnData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/SpawnData.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.017** Partir y cerrar la migracion auditada de `game/Maps/TerrainMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/TerrainMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 877 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.018** Cerrar la migracion auditada de `game/Maps/TerrainMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/TerrainMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.019** Partir y cerrar la migracion auditada de `game/Maps/TransportMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/TransportMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 714 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MAPS.WBS.020** Cerrar la migracion auditada de `game/Maps/TransportMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/TransportMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.021** Cerrar la migracion auditada de `game/Maps/ZoneScript.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/ZoneScript.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.022** Cerrar la migracion auditada de `game/Maps/ZoneScript.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/ZoneScript.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAPS.WBS.023** Cerrar la migracion auditada de `game/Maps/enuminfo_SpawnData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/enuminfo_SpawnData.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerated for reference in MIGRATION_ROADMAP.md section 5.

Complexity: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, split).

- [ ] **#MAPS.1** Extract Map::GetId(), GetEntry(), GetDifficultyID(), Instanceable(), IsDungeon() etc. into MapProperties struct; build static DB2 query layer (L)
- [ ] **#MAPS.2** Implement GridCoord → NGridType coord mapping (Trinity::ComputeGridCoord reverse, MAX_NUMBER_OF_GRIDS=64, SIZE_OF_GRIDS=533.33) with unit tests (L)
- [ ] **#MAPS.3** Port GridDefines (MAX_NUMBER_OF_CELLS=8, SIZE_OF_GRID_CELL=66.67, TOTAL_NUMBER_OF_CELLS_PER_MAP=512) as constants + cell coord math (M)
- [ ] **#MAPS.4** Implement GridInfo state tracker (timer, unload locks, visibility update periodic) matching C++ GridInfo (M)
- [ ] **#MAPS.5** Implement grid state machine classes: GridState trait, InvalidState, ActiveState, IdleState, RemovalState (matches GridStates.h) (M)
- [ ] **#MAPS.6** Build NGrid container (8x8 cell grid per terrain node) with GetGridType(x,y), SetGridState, GetGridState, isGridObjectDataLoaded (H)
- [ ] **#MAPS.7** Port ObjectGridLoader pattern: load creatures + GameObjects from DB when grid enters ACTIVE state, filtered by cell (H)
- [ ] **#MAPS.8** Implement Map::LoadGrid(x, y) trigger → ObjectGridLoader::LoadN() → spawn loading (M)
- [ ] **#MAPS.9** Implement Map::UnloadGrid(NGrid, force) → despawn entities → unload from DB (M)
- [ ] **#MAPS.10** Implement Map::Update(diff_ms) main loop → iterate all NGrids → state update for each grid (H)
- [ ] **#MAPS.11** Implement MapManager::CreateMap(map_id, player?) → instance lookup or create → return Map or InstanceMap (M)
- [ ] **#MAPS.12** Implement MapManager::FindMap(map_id, instance_id) lookup from DashMap (L)
- [ ] **#MAPS.13** Implement MapManager::Update(diff_ms) → iterate all maps → Map::Update (L)
- [ ] **#MAPS.14** Port Grid<T> generic container (Grid.h template) as Rust generic; TypeMapContainer variant (H)
- [ ] **#MAPS.15** Implement Cell-level entity tracking: AddWorldObject, AddGridObject, RemoveWorldObject, RemoveGridObject (M)
- [ ] **#MAPS.16** Implement Map::AddPlayerToMap(player) → load surrounding grids → broadcast SMSG_NEW_WORLD (M)
- [ ] **#MAPS.17** Implement Map::RemovePlayerFromMap(player) → check if last player → schedule grid unload (M)
- [ ] **#MAPS.18** Implement Map::PlayerRelocation(player, x, y, z, o) → cell change detection → update grid cell → broadcast to nearby (M)
- [ ] **#MAPS.19** Implement Map::CreatureRelocation(creature, x, y, z, a) → cell update + respawn lock check (M)
- [ ] **#MAPS.20** Implement Map::VisitNearbyCellsOf(object, visitor) → 9x9 grid iteration around object (H)
- [ ] **#MAPS.21** Implement RespawnInfo queue (min-heap, respawn times, type/entry/spawnId) matching C++ respawn system (H)
- [ ] **#MAPS.22** Implement InstanceMap subclass (Map + difficulty + instance lock + script context) (M)
- [ ] **#MAPS.23** Implement BattlegroundMap subclass (Map + BG reference) (L)
- [ ] **#MAPS.24** Port TerrainInfo queries (height, water, area, LoS) wrapper around wow-recastdetour or stub (XL — deferred to L5)
- [ ] **#MAPS.25** Refactor WorldCreature from session.rs into crates/wow-entity/creature.rs; remove from session, use entity lookup (H)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MAPS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 23 files / 10194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | `cargo test -p wow-map && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MAPS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 23 files / 10194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MAPS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 23 files / 10194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MAPS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 23 files / 10194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests that demonstrate Rust behavior ≡ C++ behavior for key invariants.

- [ ] Test: GridCoord::surrounding() returns exactly 9 cells in 3x3 around self (including self)
- [ ] Test: GridCoord::distance_squared() matches Manhattan-like grid distance metric
- [ ] Test: coord_to_grid(x, y) → grid_coord matches Trinity::ComputeGridCoord (stochastic test on boundaries: -100, 0, 100, 1000 etc.)
- [ ] Test: GridInfo timer expiry matches C++ TimeTracker logic (reset on state change, update on tick)
- [ ] Test: Grid state transitions: INVALID → no update → ACTIVE (on player enter) → IDLE (on no players) → REMOVAL → unload
- [ ] Test: ObjectGridLoader loads creatures + GameObjects only for the target cell (verify spawn_id ranges)
- [ ] Test: Map::AddPlayerToMap broadcasts SMSG_NEW_WORLD with correct map_id + position
- [ ] Test: Map::PlayerRelocation on cell boundary updates grid cell reference
- [ ] Test: Map::VisitNearbyCellsOf(cell=center) visits exactly 9 cells (8 neighbors + self)
- [ ] Test: RespawnInfo heap ordering (earlier respawn times sorted first) matches C++ CompareRespawnInfo
- [ ] Test: UnloadGrid removes all entities from container (creature, GameObject, etc. counts → 0)
- [ ] Test: MapManager::Update() ticks all maps; verify all grids receive Update(diff) call
- [ ] Test: Concurrent Map::Update() + entity add/remove (no race conditions, ARC/RwLock safety)
- [ ] Test: Instance map cannot unload while player is inside (unload lock)
- [ ] Test: BattlegroundMap unloads when last player leaves (time-based vs explicit)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 23 files / 10194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp` | `crates/wow-world/`, `crates/wow-map/` \| 🔧 broken (rewrite needed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MAPS.DIV.001` | `crates/wow-world/src/map.rs` (`missing_declared_path`, 0 Rust lines) | 23 C++ files / 10194 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.h`, `/home/server/woltk-trinity-legacy/src/server/game/Maps/MapScripts.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

**C++ historical bugs / design quirks:**
- **Grid state machine timing:** The C++ GridInfo::UpdateTimeTracker(diff) is called only by ActiveState and RemovalState, not InvalidState. RustyCore must preserve this asymmetry (GridStates.cpp:28-43).
- **GRID_STATE_REMOVAL race condition:** If a player enters a grid during RemovalState, the unload can be deferred. C++ uses explicit unload locks (`getUnloadLock()`) to prevent despawning active entities (GridInfo.cpp:36-39).
- **Respawn heap performance:** C++ uses `boost::heap::fibonacci_heap` for O(1) peek/pop of next respawn; Rust BinaryHeap is O(log n) — acceptable but slower (Map.cpp:78).
- **VisitNearbyCellsOf asymmetry:** Grid cells are 8x8 per NGrid, but VisitNearbyCellsOf visits in a 9-cell radius around player, not grid boundary. Off-by-one risks exist if boundaries are misaligned (Map.cpp:214).
- **Player interest management:** C++ has subtle visibility "interest" protocol — clients only see creatures/objects in cells nearby. If VisitNearbyCellsOf is off, clients get stale data or pop-in bugs.
- **Respawn timer cleanup:** Respawns are per-cell; orphaned RespawnInfo entries (creature deleted, respawn timer still in heap) can cause memory leaks if cleanup is missed (Map::UnloadGrid → UnloadRespawnInfos).
- **World map vs Instance map:** Both inherit from Map but behave differently:
  - **World maps** unload grids on timeout (default 5 min idle).
  - **Instance maps** keep grids loaded while locked (DungeonEncounter not defeated).
  - **Battleground maps** unload immediately after all players leave.
  This logic is in Map::CanUnload() and MapManager::CreateWorldMap/CreateInstance. Must preserve per-type behavior (MapManager.cpp:71-131).
- **TransferAbortReason enum:** 33 distinct failure codes (TRANSFER_ABORT_*). Not all are actively used in 3.4.3; some are legacy. Document which ones actually send to client (Map.h:83-107).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Map` | `struct Map` in `crates/wow-world/src/map.rs` | Contains Arc<DashMap<GridCoord, NGrid>>, player_refs: Vec<PlayerRef>, owned objects |
| `MapManager` (singleton) | `static MAPMANAGER: OnceLock<MapManager>` in `crates/world-server/src/main.rs` or dedicated module | Singleton pattern using OnceLock or lazy_static |
| `Map::AddPlayerToMap(Player*)` | `fn add_player_to_map(&mut self, player_id: u64) -> Result<Position>` | player_id instead of pointer; return position or error |
| `Map::LoadGrid(float, float)` | `fn load_grid(&mut self, x: f32, y: f32) -> Result<()>` or `async` | Fetches DB, spawns creatures/GameObjects |
| `Map::Update(uint32)` | `fn update(&mut self, diff_ms: u32)` | Iterate NGrids, call state.update(&mut self, &mut grid, diff_ms) |
| `NGridType` (NGrid<8, ...>) | `struct NGrid { cells: [[Cell; 8]; 8], state: GridState, ... }` | 2D array of cells, state enum |
| `Grid<T>` (template) | `struct Cell<T>` / `CellContainer` generic over AllGridObjectTypes | Rust generics instead of C++ templates |
| `GridState` | `enum GridState { Invalid, Active, Idle, Removal }` | Enum instead of virtual class hierarchy |
| `GridInfo` | `struct GridInfo { timer: TimeTracker, unload_locks: u16, vis_timer: PeriodicTimer }` | Same fields, simpler structure |
| `ObjectGridLoader::LoadN()` | `fn load_cell(cell: &mut Cell, grid_id: u32, cx: u8, cy: u8, db: &WorldDb)` | Async/blocking DB query; populate cell |
| `RespawnInfo` | `struct RespawnEntry { spawn_type: SpawnType, entry: u32, respawn_time: SystemTime, grid_id: u32 }` | SystemTime instead of time_t (Unix epoch) |
| `Map::Player* player` | `Arc<RwLock<Player>>` or `player_id: u64` + session lookup | Avoid raw pointers; use Arc or IDs + lookup |
| `Creature*, GameObject*` | `Arc<RwLock<Creature>>` / `Box<GameObject>` | Owned or shared, not raw ptrs |
| `std::unordered_map<ObjectGuid, Creature*>` | `DashMap<ObjectGuid, Arc<RwLock<Creature>>>` | Concurrent hashmap for thread-safe entity lookups |
| `Trinity::ComputeGridCoord(x, y)` | `fn world_to_grid_coord(x: f32, y: f32) -> GridCoord` | Pure function in util module |
| `grid.GetGridState()` | `grid.state` (direct field access) or `grid.get_state()` (method) | Pattern match on enum |
| `void Foo::Update(uint32)` | `fn update(&mut self, diff_ms: u32)` | — |

---

*Template version: 1.0 (2026-05-01).* Revision: initial complete audit port.

---

## 13. Audit (2026-05-01)

Audited C++ tree: `/home/server/woltk-trinity-legacy/src/server/game/Maps/{Map.cpp:4014, Map.h:917, MapManager.cpp:461, MapManager.h:183}`. Audited Rust tree: `/home/server/archived/rustycore_ARCHIVED_20260312/crates/wow-world/src/map_manager.rs:784` (one file; no `map.rs`, no `wow-map/src/lib.rs` content).

### 13.1 Coverage table — Map class

| C++ symbol | Rust equivalent | Status |
|---|---|---|
| `class Map : public GridRefManager<NGridType>` (Map.h:186) | None — only `MapInstance { map_id, instance_id, grids: HashMap<GridCoord, Grid> }` (map_manager.rs:371) | ❌ |
| `Map::Update(uint32 t_diff)` (Map.cpp:666) | **MISSING** — no per-map tick exists anywhere | ❌ |
| `Map::EnsureGridLoaded(Cell)` (Map.h:583, Map.cpp:325) | `MapInstance::get_or_create_grid()` (map_manager.rs:388) — creates blank grid, does **not** load DB spawns | ⚠️ name only |
| `Map::EnsureGridLoadedForActiveObject` (Map.h:584, Map.cpp:348) | None | ❌ |
| `Map::LoadGrid(float, float)` (Map.cpp:417) | None — Rust only has world→grid coord conversion (`world_to_grid_coords` :632) | ❌ |
| `Map::UnloadGrid(NGridType&, bool)` (Map.cpp; declared Map.h:246) | `MapInstance::unload_empty_grids()` :430 / `unload_distant_grids()` :576 — timeout-only, never called | ⚠️ |
| `Map::UnloadAll()` (Map.cpp:1646) | None | ❌ |
| `Map::AddPlayerToMap(Player*, bool)` | `player_enter_grid()` (map_manager.rs:525) — only marks `HashSet<ObjectGuid>`; no `SMSG_NEW_WORLD`, no grid load | ⚠️ |
| `Map::RemovePlayerFromMap(Player*, bool)` (Map.cpp:907) | `player_leave_grid()` :531 | ⚠️ |
| `Map::AddToMap<T>` / `Map::AddToGrid<T>` (Map.cpp:190-242) | `MapInstance::add_creature()` :410 — creatures only; no Player/GO/DynamicObject/AreaTrigger/Corpse | ⚠️ creatures only |
| `Map::RemoveFromMap<T>` (Map.cpp:934) | `MapInstance::remove_creature()` :414 — creatures only | ⚠️ |
| `Map::PlayerRelocation` (Map.cpp:1015) | `player_move()` :538 — only swaps grid membership; no notify, no visibility recalc, no broadcast | ⚠️ |
| `Map::CreatureRelocation` (Map.cpp:1042) | None (creature movement happens inside `WorldSession::tick_*`, not Map) | ❌ |
| `Map::GameObjectRelocation` (Map.cpp:1074) | None | ❌ |
| `Map::DynamicObjectRelocation` (Map.cpp:1103) | None | ❌ |
| `Map::AreaTriggerRelocation` (Map.cpp:1133) | None | ❌ |
| `Map::VisitNearbyCellsOf` (Map.cpp:622) | `MapManager::get_visible_creatures` :549 — 3×3 grid scan, **creatures only**, no visitor pattern, no `WorldTypeMapContainer`/`GridTypeMapContainer` distinction | ⚠️ |
| `Map::ProcessRelocationNotifies` (Map.cpp:830) | None | ❌ |
| `Map::ScriptsProcess()` (Map.h:598; MapScripts.cpp:899L) | None | ❌ |
| `Map::AddObjectToRemoveList(WorldObject*)` (Map.h:345) | None | ❌ |
| `Map::RemoveAllObjectsInRemoveList()` (Map.h:302) | None | ❌ |
| `Map::ResetMarkedCells()` / `resetMarkedCells()` (Map.cpp:693) | None | ❌ |
| `Map::ResetGridExpiry(NGridType&, float)` (Map.h:251) | None — `last_player_time: Instant` (:311) reset on player_enter only | ⚠️ |
| `Map::MoveAllCreaturesInMoveList()` (Map.cpp:1239) | None | ❌ |
| `Map::ProcessRespawns()` (Map.cpp:2191) / `Respawn()` :2025 | None — `WorldCreature::should_respawn` :209 is a per-creature wall-clock check, no heap, no DB persist | ⚠️ stub |
| `Map::CleanupCorpses` / corpse decay | None — `WorldCreature::corpse_despawn_at: Option<Instant>` (:63) field exists, never read | ⚠️ |
| `Map::SendObjectUpdates()` (Map.cpp:1929) | None | ❌ |
| `Map::AddCreatureToMoveList` / move-list family (Map.cpp:1163-1230) | None | ❌ |
| `Map::GetHeight / GetWaterLevel / GetZoneId / GetAreaId` | None — terrain queries unimplemented | ❌ |
| `Map::SetWorldStateValue` (Map.cpp:480) | None | ❌ |
| `Map::SendInitSelf / SendInitTransports` (Map.cpp:1826/1853) | None | ❌ |
| `Map::InitStateMachine() / DeleteStateMachine()` (Map.cpp:124/132) | None — no state machine to init | ❌ |
| `class InstanceMap : public Map` (Map.h:841) | None — `MapInstance` is a **flat** map+instance_id container, not subclass; no instance lock, no `InstanceLock`, no difficulty | ❌ |
| `class BattlegroundMap : public Map` (Map.h:883) | None | ❌ |

### 13.2 Coverage table — MapManager

| C++ symbol | Rust equivalent | Status |
|---|---|---|
| `MapManager::Initialize()` (MapManager.cpp:44) | None | ❌ |
| `MapManager::InitializeVisibilityDistanceInfo()` (MapManager.cpp:54) | None | ❌ |
| `MapManager::Update(uint32 diff)` (MapManager.cpp:293) — schedules `Map::Update` per loaded map via `MapUpdater` thread pool | **MISSING** — no `MapManager::update()` exists; the only world-server tick is `session.update(50)` per-session inside `world-server/src/main.rs:609` | ❌ **breaking divergence** |
| `MapManager::CreateMap(uint32, Player*)` | `get_or_create_map(map_id, instance_id)` (map_manager.rs:466) — no DB2 lookup, no instance creation rules, no transfer-abort | ⚠️ |
| `MapManager::CreateInstance / CreateBattleground` | None | ❌ |
| `MapManager::FindMap(uint32, uint32)` | `get_map(map_id, instance_id)` :476 | ✅ minimal |
| `MapManager::DestroyMap(Map*)` (MapManager.cpp:328) | None — maps live forever in `HashMap<(u16,u32), MapInstance>` | ❌ |
| `MapManager::UnloadAll()` (MapManager.cpp:350) | None | ❌ |
| `MapManager::IsValidMAP / InitInstanceIds / RegisterInstanceId / FreeInstanceId` | None — `instance_id: u32` is a free parameter, no allocator | ❌ |
| `m_updater: MapUpdater` (thread-pool dispatch) | None — no parallel map update | ❌ |

### 13.3 NGrid / Cell architecture

C++ uses a strict 3-tier hierarchy `Map → NGrid<8>[64][64] → Cell[8][8]`, with `MAX_NUMBER_OF_GRIDS=64`, `MAX_NUMBER_OF_CELLS=8`, `SIZE_OF_GRIDS=533.3333f`, `CENTER_GRID_ID=32`, `SIZE_OF_GRID_CELL=66.6667f`, `MAP_RESOLUTION=128` (GridDefines.h:36-57). Each `NGridType` carries `GridInfo` (timer + `i_unloadActiveLockCount: u16` + `i_unloadExplicitLock`) and a `grid_state_t` ∈ {INVALID, ACTIVE, IDLE, REMOVAL} (NGrid.h:30-60). The 4-state machine is virtual-dispatched via `GridState::Update()` overrides (GridStates.cpp:24-65).

Rust replication: **none of this exists**. `GridCoord { x: i16, y: i16 }` (map_manager.rs:21) is unbounded (no [-64..64] clamp, no `CENTER_GRID_ID=32` reorientation — Trinity's `ComputeGridCoord` flips axes via `(MAX_NUMBER_OF_GRIDS-1) - gx`, GridDefines.h:201-203, which Rust ignores). `GRID_SIZE = 64.0` (line 11) is **wrong by 8.33×** vs C++ `SIZE_OF_GRIDS = 533.33` — Rust's "grid" is actually one C++ Cell. There is no NGrid layer, no `GridInfo`, no `GridState` enum, no `unloadActiveLockCount`, no per-grid `TimeTracker`. `Grid` (:307) is a flat `HashMap<ObjectGuid, WorldCreature>` + `HashSet<ObjectGuid>` — no typed `WorldTypeMapContainer`/`GridTypeMapContainer` partition.

### 13.4 Critical divergences

1. **No update loop reaches the map.** C++: `main → World::Update → MapManager::Update → Map::Update → NGrid state.Update → ObjectGridLoader / ObjectUpdater`. Rust: `main → tokio::spawn(start_world_listener)` then per-connection `loop { session.update(50); }` (`world-server/src/main.rs:606-623`). The `SharedMapManager = Arc<RwLock<MapManager>>` (map_manager.rs:616) is mutated from session handlers but **nothing ever calls a tick on it** — grids never expire, respawns never fire from the map side, scripts never process, remove-list never drains.
2. **Grid scale mismatch.** Rust treats 64 yards as one grid (line 11); C++ treats 533.33 yards as one grid and 66.67 yards as one cell. Anything expecting "9-cell visibility ≈ 100 yards" works coincidentally because `VISIBILITY_RADIUS=100` (:14) ≈ 1.5 × Rust GRID_SIZE, but real Trinity visibility (`SIZE_OF_GRID_CELL=66.67` × 9 cells ≈ 600 yards span, capped to per-map `m_VisibleDistance` ~100 y) uses a different math.
3. **No state machine.** The GridStates 4-state lifecycle (Invalid→Active→Idle→Removal, GridStates.cpp:24-65) — the entire purpose of the Grids module — is absent. `Grid::should_unload` (:356) is a single boolean (`empty AND idle>5min`); no INVALID/REMOVAL distinction, no `getUnloadLock()`, no `incUnloadActiveLock` for active spawn points.
4. **No ObjectGridLoader.** When a grid is created in Rust (`get_or_create_grid` :388), nothing queries the world DB for spawns. C++ `ObjectGridLoader::LoadN()` (ObjectGridLoader.cpp:171-198) iterates 8×8 cells and pulls `creature` / `gameobject` / `areatrigger` / corpses for each cell. Creatures in Rust are inserted ad-hoc by handler code, not by grid activation.
5. **Creature is conflated with Map.** `WorldCreature` (map_manager.rs:52-303) embeds `CreatureCreateData` + AI state (`combat_target`, `wander_timer`, `last_swing`, `move_target`) — these belong in `Creature`/`CreatureAI`, not in a Map cell. C++ Map only stores `Creature*`; AI runs via `Creature::Update` invoked by `ObjectUpdater` visitor inside `Map::Update`.
6. **Two-creature-storage-systems coexist.** Per AGENTS.md, `WorldSession.creatures: HashMap<ObjectGuid, CreatureAI>` is the legacy path still used by `handlers/character.rs` and `session.rs`; `MapManager` is the new path used by `handlers/loot.rs`, `handlers/misc.rs`, `handlers/trainer.rs`. Until the legacy path is removed, neither side is authoritative — visibility skew is structural.

### 13.5 Verdict

🔧 **broken — keep status as-is, downgrade audit field to ❌ confirmed**. The `MapManager` module compiles, holds 12 unit tests, and provides a credible-looking façade, but it is missing the four load-bearing C++ structures: (a) `Map::Update()` loop, (b) `MapManager::Update()` dispatcher, (c) NGrid + GridState 4-state machine, (d) ObjectGridLoader DB integration. The world server never ticks the map. Recommended work order: #MAPS.5 → #MAPS.6 → #MAPS.10 → #MAPS.13 (state machine before container before per-map tick before per-manager tick), then #MAPS.7 (`ObjectGridLoader`) once `wow-database` exposes per-cell spawn queries. `_attic/` is irrelevant here — that integration was bridging fields that don't exist on `CreatureCreateData`; this audit's blocker is upstream of that work.
