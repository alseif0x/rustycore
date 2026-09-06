# Migration: Grids

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Grids/`
> **Rust target crate(s):** `crates/wow-world/` (shared with Maps)
> **Layer:** L3 (World layer, substrate for Maps)
> **Status:** 🔧 broken (coordinate foundation ported; missing core state machine)
> **Audited vs C++:** ❌ confirmed broken — 2026-05-01 audit; flat HashMap "grid" replaces NGrid/Cell hierarchy, no GridState machine, no GridInfo, no ObjectGridLoader, world→grid coords ignore Trinity's [-64..64] center-32 reorientation
> **Last updated:** 2026-05-01

---

## 1. Purpose

Grids is the spatial partitioning system that subdivides a Map into a 64x64 grid of NGrids, each containing an 8x8 array of Cells. Grids implement lazy loading: spawns are loaded only when a grid transitions to ACTIVE state (player or active object nearby), and unloaded when entering REMOVAL state after idling. This architecture enables efficient entity tracking, visibility queries, and memory management across large open worlds.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Grids/Cells/Cell.h` | 123 | `prefix` |
| `game/Grids/Cells/CellImpl.h` | 254 | `prefix` |
| `game/Grids/Dynamic/TypeContainer.h` | 225 | `prefix` |
| `game/Grids/Dynamic/TypeContainerFunctions.h` | 157 | `prefix` |
| `game/Grids/Dynamic/TypeContainerVisitor.h` | 101 | `prefix` |
| `game/Grids/Grid.h` | 142 | `prefix` |
| `game/Grids/GridDefines.h` | 251 | `prefix` |
| `game/Grids/GridLoader.h` | 76 | `prefix` |
| `game/Grids/GridRefManager.h` | 38 | `prefix` |
| `game/Grids/GridReference.h` | 51 | `prefix` |
| `game/Grids/GridStates.cpp` | 65 | `prefix` |
| `game/Grids/GridStates.h` | 56 | `prefix` |
| `game/Grids/NGrid.cpp` | 36 | `prefix` |
| `game/Grids/NGrid.h` | 183 | `prefix` |
| `game/Grids/Notifiers/GridNotifiers.cpp` | 301 | `prefix` |
| `game/Grids/Notifiers/GridNotifiers.h` | 1804 | `prefix` |
| `game/Grids/Notifiers/GridNotifiersImpl.h` | 320 | `prefix` |
| `game/Grids/ObjectGridLoader.cpp` | 283 | `prefix` |
| `game/Grids/ObjectGridLoader.h` | 126 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/src/server/game/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `Grids/GridDefines.h` | 251 | Global grid constants, typedefs, bitmasks for object types |
| `Grids/Grid.h` | 142 | Generic Grid<T> template for cell-level entity containers |
| `Grids/NGrid.h` | 183 | NGrid template wrapper: 8x8 cell grid with state + timers |
| `Grids/GridInfo.h` (part of NGrid.h) | ~50 | GridInfo metadata: unload timers, lock counters, visibility tracker |
| `Grids/GridStates.h` | 56 | GridState abstract base class + InvalidState, ActiveState, IdleState, RemovalState |
| `Grids/GridStates.cpp` | 65 | State machine implementations: Update() logic for each state |
| `Grids/NGrid.cpp` | 36 | NGrid ctor/dtor, minimal impl (mostly inline in .h) |
| `Grids/GridReference.h` | 51 | Intrusive reference node for doubly-linked list of entities in cell |
| `Grids/GridRefManager.h` | 38 | Manager for GridReference nodes (intrusive container) |
| `Grids/GridLoader.h` | 76 | GridLoader<T> template interface for loading spawns into grid |
| `Grids/ObjectGridLoader.h` | 126 | ObjectGridLoader + PersonalPhaseGridLoader + ObjectGridStoper |
| `Grids/ObjectGridLoader.cpp` | 283 | ObjectGridLoader::Visit(CreatureMapType), Visit(GameObjectMapType), LoadN() impl |
| `Grids/Cells/CellImpl.h` | (directory) | Cell visitor implementations (GridNotifiersImpl) |
| `Grids/Dynamic/DynamicTreeImpl.h` | (directory) | Dynamic object spatial tree (visibility queries) |
| `Grids/Notifiers/GridNotifiers.h` | (directory) | Visitor patterns for cell iteration |

**Total C++ lines in Grids/:** ~1,307 (excluding subdirectories)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Grid<ACTIVE_OBJECT, WORLD_OBJECT_TYPES, GRID_OBJECT_TYPES>` | template class | Single cell container; holds world + grid objects via TypeMapContainer |
| `NGrid<N, ACTIVE_OBJECT, WORLD_OBJECT_TYPES, GRID_OBJECT_TYPES>` | template class | NxN grid of Cells; state machine, timers, object data loaded flag |
| `GridInfo` | class | Metadata for NGrid: time tracker, unload lock counters, visibility update timer |
| `GridState` | class (abstract) | Base class for grid lifecycle state machine |
| `InvalidState` | class | Grid state: uninitialized, no load/unload actions |
| `ActiveState` | class | Grid state: active (player nearby), periodic idle check, entities updated |
| `IdleState` | class | Grid state: idle (no players), transition to REMOVAL on timeout |
| `RemovalState` | class | Grid state: scheduled for unload, despawn entities after delay |
| `GridReference<SPECIFIC_OBJECT>` | template struct | Intrusive doubly-linked list node for entity tracking |
| `GridRefManager<TYPE>` | template class | Doubly-linked list container for GridReference nodes |
| `GridLoader<ACTIVE_OBJECT, WORLD_OBJECT_TYPES, GRID_OBJECT_TYPES>` | template class | Abstract loader interface; subclassed by ObjectGridLoader |
| `ObjectGridLoaderBase` | class | Base for creature/GameObject loaders; tracks load counts |
| `ObjectGridLoader` | class | Loads creatures + GameObjects from DB on grid load |
| `PersonalPhaseGridLoader` | class | Loads phase-specific spawns (personal instances) |
| `ObjectGridStoper` | class | Visitor to despawn creatures before grid unload |
| `grid_state_t` | enum | Grid lifecycle: INVALID, ACTIVE, IDLE, REMOVAL (4 states) |
| `GridMapTypeMask` | enum | Bitmask for filtering entity types by mask |
| `AllWorldObjectTypes` | typedef | TYPELIST_4: Player, Creature, Corpse, DynamicObject |
| `AllGridObjectTypes` | typedef | TYPELIST_7: GameObject, Creature, DynamicObject, Corpse, AreaTrigger, SceneObject, Conversation |
| `AllMapStoredObjectTypes` | typedef | TYPELIST_8: All grid + world object types |
| `TypeMapContainer<T>` | template | Heterogeneous container for typed object lists |
| `NGridType` | typedef | NGrid<8, Player, AllWorldObjectTypes, AllGridObjectTypes> (concrete instance) |
| `GridType` | typedef | Grid<Player, AllWorldObjectTypes, AllGridObjectTypes> (concrete instance) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `NGrid::NGrid(id, x, y, expiry, unload)` | Constructor; initializes 8x8 cell array, state INVALID, timers | GridInfo ctor |
| `NGrid::GetGridType(x, y)` | Retrieve Cell at position (x, y) within grid (0-7 range) | Cell methods |
| `NGrid::GetGridState()` | Query current lifecycle state (INVALID/ACTIVE/IDLE/REMOVAL) | State machine |
| `NGrid::SetGridState(state)` | Transition to new state | State machine transition |
| `NGrid::GetGridId()` | Return unique grid ID (map-local) | Entity relocation, respawn tracking |
| `NGrid::getX(), getY()` | Get 2D grid coordinates (0-63 in 64x64 map grid) | Entity relocation |
| `NGrid::isGridObjectDataLoaded()` | Query if spawns have been loaded from DB | ObjectGridLoader |
| `GridInfo::getTimeTracker()` | Query current timer for state expiry | State machine Update() |
| `GridInfo::getUnloadLock()` | Check if unload is blocked (has active locks or explicit lock) | Unload prevention |
| `GridInfo::incUnloadActiveLock(), decUnloadActiveLock()` | Increment/decrement active spawn point locks | Creature spawn, active object tracking |
| `GridInfo::setUnloadExplicitLock(bool)` | Manually lock/unlock grid from unloading | Admin commands, instance locks |
| `GridInfo::ResetTimeTracker(interval)` | Reset timer to interval (called on state transition) | State machine |
| `GridInfo::UpdateTimeTracker(diff)` | Tick timer by diff_ms | GridState::Update() |
| `Grid::AddWorldObject(SPECIFIC_OBJECT*)` | Add world object to cell (Player, Corpse, etc.) | Map::AddToMap |
| `Grid::AddGridObject(SPECIFIC_OBJECT*)` | Add grid object to cell (Creature, GameObject, etc.) | Map::AddToMap |
| `Grid::Visit<T>(visitor)` | Iterate grid or world objects with visitor pattern | Spell AoE, threat updates |
| `Grid::GetWorldObjectCountInGrid<T>()` | Count objects of type T in cell | Activity checks |
| `GridReference<T>::link(mgr, obj)` | Create intrusive link for object in container | GridRefManager |
| `GridRefManager<T>::isEmpty()` | Check if container is empty | Activity checks |
| `ObjectGridLoader::Visit(CreatureMapType&)` | Load creatures from DB for cell; spawn into Map | Grid load transition |
| `ObjectGridLoader::Visit(GameObjectMapType&)` | Load GameObjects from DB for cell; spawn into Map | Grid load transition |
| `ObjectGridLoader::Visit(AreaTriggerMapType&)` | Load area triggers for cell | Grid load transition |
| `ObjectGridLoader::LoadN()` | Main entry point; queries DB and populates cell | Map::LoadGrid |
| `ObjectGridStoper::Visit(CreatureMapType&)` | Stop + despawn all creatures in cell before unload | Grid unload transition |
| `GridState::Update(Map&, NGrid&, GridInfo&, diff)` | Per-frame state machine tick (pure virtual) | Map::Update loop |
| `InvalidState::Update()` | No-op; grid is uninitialized | State machine |
| `ActiveState::Update()` | Check player presence; if none, transition to IDLE; else reset expiry timer | GridStates.cpp:28-44 |
| `IdleState::Update()` | Transition to REMOVAL after delay | GridStates.cpp:47-51 |
| `RemovalState::Update()` | Unload grid if no unload locks remain | GridStates.cpp:54-64 |

---

## 5. Module dependencies

**Depends on:**
- `Maps` — NGrid is contained by Map via GridRefManager; lifecycle driven by Map::Update()
- `Entities` (Creature, GameObject, AreaTrigger, Corpse, Player) — entities stored in Cells within NGrid
- `Database` (WorldDatabase) — ObjectGridLoader queries creature, gameobject tables
- `Data` (creature_template, gameobject_template DB2/cache) — spawn metadata
- `Cell` (implicit via Grid.h) — Cell visitor pattern
- `GridNotifiers` — visitor implementations for spell AoE, threat, visibility

**Depended on by:**
- `Maps` — Map uses NGrid as primary spatial container
- `Entities` — entities belong to Cells, which belong to NGrids
- `Movement` — relocation triggers cell updates
- `Spells` / `Combat` — AoE checks iterate nearby cells via VisitNearbyCellsOf
- `Scripts` — grid state hooks (OnGridLoad, OnGridUnload)
- `Phasing` — personal phase grids use PersonalPhaseGridLoader

---

## 6. SQL / DB queries (if any)

Grids **does not** directly emit SQL; all queries routed through ObjectGridLoader.

| Source / Context | Query Pattern | Purpose | Database |
|---|---|---|---|
| `ObjectGridLoader::Visit(CreatureMapType&)` | `SELECT creature FROM world.creature WHERE map=? AND grid_id=? AND cell_x=? AND cell_y=?` | Load spawns when grid activates | world |
| `ObjectGridLoader::Visit(GameObjectMapType&)` | `SELECT gameobject FROM world.gameobject WHERE map=? AND grid_id=? AND cell_x=? AND cell_y=?` | Load objects when grid activates | world |
| `ObjectGridLoader::Visit(AreaTriggerMapType&)` | `SELECT areatrigger FROM world.areatrigger WHERE map=? AND grid_id=?` | Load triggers when grid activates | world |
| (implicit) Template loading | (DB2 queries via DataStores) | creature_template, gameobject_template | world |

**No DB2 stores directly accessed by Grids** (filtered/loaded by ObjectMgr before ObjectGridLoader).

---

## 7. Wire-protocol packets (if any)

Grids module **does not originate** any packets directly. All object creation/destruction packets are sent by individual entities or Map::VisitNearbyCellsOf broadcasts.

| Opcode | Direction | Context |
|---|---|---|
| `SMSG_CREATE_OBJECT` / `SMSG_UPDATE_OBJECT` | server → client | Sent when entity spawns in cell (via creature/GameObject handlers) |
| `SMSG_DESTROY_OBJECT` | server → client | Sent when entity despawns in cell (grid unload or entity removal) |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-map/src/coords.rs` | `file` | 1 | 254 | `exists_active` | file exists |
| `crates/wow-map/src/cell.rs` | `file` | 1 | 234 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-map/src/coords.rs` — TrinityCore grid/cell constants and `ComputeGridCoord` / `ComputeCellCoord` equivalents.
- `crates/wow-map/src/cell.rs` — first-pass cell container with TrinityCore grid/cell decomposition and separate world/grid object GUID sets.
- `crates/wow-world/src/map_manager.rs` — legacy scaffold only; still not the canonical Map/NGrid/Cell implementation.
- No dedicated `grid.rs` or grid state machine file yet.

**What's implemented:**
- `CoordPair<LIMIT>`, `GridCoord`, `CellCoord`.
- Constants from `GridDefines.h`: `MAX_NUMBER_OF_GRIDS=64`, `MAX_NUMBER_OF_CELLS=8`, `SIZE_OF_GRIDS=533.3333`, `SIZE_OF_GRID_CELL=66.6666...`, `TOTAL_NUMBER_OF_CELLS_PER_MAP=512`.
- `compute_grid_coord`, `compute_grid_coord_simple`, `compute_cell_coord`, `compute_cell_coord_with_offset`, `cell_to_grid_local`, map-coordinate validation/clamping.
- `Cell`, `CellArea`, `WorldObjectGuids`, `GridObjectGuids` with separate typed GUID containers matching the C++ split between `i_objects` and `i_container`.

**What's missing vs C++:**
1. **No NGrid<> template/struct** — no 8x8 cell array container
2. **No GridInfo** — no state timer, unload locks, visibility tracker
3. **No GridState state machine** — no InvalidState, ActiveState, IdleState, RemovalState
4. **No Grid<T> generic container** — cell-level GUID storage exists, but no entity refs or visitor machinery yet
5. **No GridReference / GridRefManager** — no intrusive list tracking for entities
6. **No ObjectGridLoader** — no DB spawn loading on grid activation
7. ~~No coordinate conversions~~ — first-pass world→grid/cell conversions now exist in `wow-map::coords`; not yet wired into Map/NGrid lifecycle.
8. **No grid lifecycle** — no ACTIVE/IDLE/REMOVAL transitions, no lazy loading/unloading
9. **No visibility tracking** — no NGrid::VisitAllGrids or cell iteration patterns
10. **No PersonalPhaseGridLoader** — no phase-specific grid loading

**Suspicious / likely divergent:**
- WorldCreature in session.rs is entity-like data but belongs in grid cells; architecture doesn't support hierarchical entity storage yet.
- No separation between "world objects" (Player, Corpse, DynamicObject) and "grid objects" (Creature, GameObject, AreaTrigger, Conversation).
- No lazy-load pattern: static spawning vs. dynamic on-demand loading not implemented.

**Tests existing:**
- 6 tests for grid/cell coordinate math in `crates/wow-map/src/coords.rs`.
- 5 tests for cell decomposition and typed GUID storage in `crates/wow-map/src/cell.rs`.
- 0 tests for state transitions or entity loading.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#GRIDS.WBS.001** Cerrar la migracion auditada de `game/Grids/Cells/Cell.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Cells/Cell.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.002** Cerrar la migracion auditada de `game/Grids/Cells/CellImpl.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Cells/CellImpl.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.003** Cerrar la migracion auditada de `game/Grids/Dynamic/TypeContainer.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Dynamic/TypeContainer.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.004** Cerrar la migracion auditada de `game/Grids/Dynamic/TypeContainerFunctions.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Dynamic/TypeContainerFunctions.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.005** Cerrar la migracion auditada de `game/Grids/Dynamic/TypeContainerVisitor.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Dynamic/TypeContainerVisitor.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.006** Cerrar la migracion auditada de `game/Grids/Grid.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Grid.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.007** Cerrar la migracion auditada de `game/Grids/GridDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridDefines.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.008** Cerrar la migracion auditada de `game/Grids/GridLoader.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridLoader.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.009** Cerrar la migracion auditada de `game/Grids/GridRefManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridRefManager.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.010** Cerrar la migracion auditada de `game/Grids/GridReference.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridReference.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.011** Cerrar la migracion auditada de `game/Grids/GridStates.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridStates.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.012** Cerrar la migracion auditada de `game/Grids/GridStates.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/GridStates.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.013** Cerrar la migracion auditada de `game/Grids/NGrid.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/NGrid.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.014** Cerrar la migracion auditada de `game/Grids/NGrid.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/NGrid.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.015** Cerrar la migracion auditada de `game/Grids/Notifiers/GridNotifiers.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.016** Partir y cerrar la migracion auditada de `game/Grids/Notifiers/GridNotifiers.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1804 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.017** Cerrar la migracion auditada de `game/Grids/Notifiers/GridNotifiersImpl.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.018** Cerrar la migracion auditada de `game/Grids/ObjectGridLoader.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/ObjectGridLoader.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GRIDS.WBS.019** Cerrar la migracion auditada de `game/Grids/ObjectGridLoader.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/ObjectGridLoader.h`
  Rust target: `crates/wow-world`, `crates/wow-map`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerated for reference in MIGRATION_ROADMAP.md section 5.

Complexity: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, split).

- [x] **#GRIDS.1** Port GridDefines constants: MAX_NUMBER_OF_GRIDS=64, MAX_NUMBER_OF_CELLS=8, SIZE_OF_GRIDS=533.33, SIZE_OF_GRID_CELL=66.67 (L)
- [x] **#GRIDS.2** Implement world-to-grid coordinate conversion: world_x,y → grid_idx (Trinity::ComputeGridCoord equivalent) with unit tests (L)
- [x] **#GRIDS.3** Implement world-to-cell coordinate conversion within grid: world_x,y → cell_x,cell_y (0-7 range) (L)
- [ ] **#GRIDS.4** Enhance GridCoord: add `cell_from_world(x,y)`, boundary checking, GridCoord↔u32 serialization (M) — partially covered by `Cell::from_world`, `Cell::cell_coord`, and `CoordPair::get_id`; reverse/id helpers still pending.
- [ ] **#GRIDS.5** Implement GridInfo struct: timer (TimeTracker), unload_locks (u16), vis_timer (PeriodicTimer), methods inc/dec locks (M)
- [ ] **#GRIDS.6** Implement GridState enum + Update trait: { Invalid, Active, Idle, Removal } each with update_state(&mut self, grid, info, diff) (M)
- [ ] **#GRIDS.7** Implement InvalidState::update() → no-op (L)
- [ ] **#GRIDS.8** Implement ActiveState::update() → periodic idle check; if no players + no active objects, transition to IDLE (M)
- [ ] **#GRIDS.9** Implement IdleState::update() → transition to REMOVAL after delay (L)
- [ ] **#GRIDS.10** Implement RemovalState::update() → if no unload locks, call map.unload_grid(self) (M)
- [ ] **#GRIDS.11** Build TypeMapContainer generic: holds heterogeneous object types (Creature, GameObject, etc.) in separate Vec/DashMap (H)
- [ ] **#GRIDS.12** Implement Cell<T> generic struct: containers for grid_objects + world_objects, add/remove/visit methods (H)
- [ ] **#GRIDS.13** Implement NGrid<T> struct: cells: [[Cell<T>; 8]; 8], state, info, grid_id, x, y; getters/setters (H)
- [ ] **#GRIDS.14** Implement NGrid::GetGridType(x, y) bounds-checked getter for Cell at index (M)
- [ ] **#GRIDS.15** Implement NGrid::SetGridState(state) with state transition logging (L)
- [ ] **#GRIDS.16** Implement GridRefManager trait/impl for intrusive reference tracking (entity→cell link) (H)
- [ ] **#GRIDS.17** Implement ObjectGridLoaderBase: load counts (creatures, gameobjects, areaTriggers) tracking (M)
- [ ] **#GRIDS.18** Implement ObjectGridLoader::Visit(CreatureMapType) → query DB, create Creature, add to cell (M)
- [ ] **#GRIDS.19** Implement ObjectGridLoader::Visit(GameObjectMapType) → query DB, create GameObject, add to cell (M)
- [ ] **#GRIDS.20** Implement ObjectGridLoader::Visit(AreaTriggerMapType) → query DB, create AreaTrigger, add to cell (L)
- [ ] **#GRIDS.21** Implement ObjectGridLoader::LoadN() entry point → iterate cells, call Visit for each type (M)
- [ ] **#GRIDS.22** Implement ObjectGridStoper visitor → despawn all creatures in cell before unload (M)
- [ ] **#GRIDS.23** Implement Cell::Visit<Visitor>() pattern → iterate entities matching type, call visitor for each (H)
- [ ] **#GRIDS.24** Implement NGrid::VisitAllGrids(visitor) → iterate all 64 cells, call visitor.Visit(cell) (M)
- [ ] **#GRIDS.25** Integration: wire NGrid state machine into Map::Update() loop (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#GRIDS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 19 files / 4592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | `cargo test -p wow-map && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#GRIDS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 19 files / 4592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#GRIDS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 19 files / 4592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#GRIDS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 19 files / 4592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp`. Rust target: `crates/wow-map`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests demonstrating Rust behavior ≡ C++ behavior for grid lifecycle and loading.

- [ ] Test: world_x,y → GridCoord matches Trinity::ComputeGridCoord for 100+ random points
- [ ] Test: GridCoord → cell_x,cell_y conversion for all boundaries (0-7 range, wrapping)
- [ ] Test: GridCoord::surrounding() returns 9 cells (verified each is adjacent or center)
- [ ] Test: GridInfo::inc/dec_unload_lock() maintains u16 counter correctly
- [ ] Test: GridInfo::getUnloadLock() returns true iff locks > 0 or explicit_lock == true
- [ ] Test: GridState transition sequence: Invalid → Active (player enter) → Idle (player leave + timeout) → Removal → Unload
- [ ] Test: ActiveState::update() triggers transition to Idle only if no players AND no active objects
- [ ] Test: RemovalState::update() calls map.unload_grid() only if getUnloadLock() == false
- [ ] Test: ObjectGridLoader populates cell with correct creature count (verify spawn_id ranges)
- [ ] Test: ObjectGridLoader loads only creatures in target cell (cell_x, cell_y bounds)
- [ ] Test: ObjectGridStoper despawns all creatures from cell (verify count → 0)
- [ ] Test: Cell::Visit iterates exactly N entities of type T
- [ ] Test: NGrid::GetGridType(x, y) returns correct Cell for x,y in [0,7] range
- [ ] Test: NGrid::VisitAllGrids calls visitor.Visit(cell) exactly 64 times
- [ ] Test: Multiple concurrent NGrid state transitions (no race conditions, ARC/RwLock safety)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 19 files / 4592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp` | `crates/wow-world/` (shared with Maps) \| 🔧 broken (coordinate foundation ported; missing core state machine) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#GRIDS.DIV.001` | _none generated_ | 19 C++ files / 4592 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiersImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Grids/Notifiers/GridNotifiers.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

**C++ historical bugs / design quirks:**

- **Grid state timing asymmetry (GridStates.cpp:28-44):** ActiveState calls `info.UpdateTimeTracker(diff)` and checks expiry; IdleState and RemovalState do not call Update on InvalidState. This timing model must be preserved in Rust.
- **Unload lock increment logic (NGrid.h:36-39):** `incUnloadActiveLock()` is called when a creature with an active spawn point enters grid. Decrement must match increment 1:1; off-by-one causes memory leaks or premature despawning.
- **VisibilityNotifyPeriod (NGrid.h:28):** Grid only updates visibility/player notifications once per `DEFAULT_VISIBILITY_NOTIFY_PERIOD` (1000ms) to reduce network spam. Rust must throttle similarly.
- **Respawn timer ownership:** RespawnInfo entries are tied to grid lifecycle; if grid unloads before respawn fires, entry must be persisted to character DB. C++ uses ObjectMgr respawn queues; Rust must replicate.
- **Cell boundary rounding:** C++ SIZE_OF_GRID_CELL = 66.67f (SIZE_OF_GRIDS / MAX_NUMBER_OF_CELLS). Float divisions can cause off-by-one errors near cell boundaries. Recommend fixed-point or int arithmetic.
- **PersonalPhaseGridLoader complexity:** Loads spawns based on phase visibility for personal instances. Requires second loader class; not used in open world (only instances).
- **GridReference intrusive linking:** C++ uses boost intrusive pointers. Rust must use Arc<RwLock<T>> or custom intrusive structures; order matters for cleanup (link before add, unlink after remove).
- **Creature respawn state:** When creature despawns (grid unload), respawn timer is saved to DB (table: creature_respawn). Rust must mimic; track respawn time per creature_id+map_id.
- **GridLoader template specialization:** C++ Grid.h has explicit template instantiation for concrete NGridType. Rust may use static dispatch or dynamic dispatch; must match performance.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `Grid<A, W, G>` (template) | `struct Cell<T>` | Generic cell container; TypeMapContainer for heterogeneous types |
| `NGrid<N, A, W, G>` | `struct NGrid { cells: [[Cell; 8]; 8], ... }` | Fixed 8x8 array, state machine, timers |
| `GridInfo` | `struct GridInfo { timer: TimeTracker, unload_locks: u16, vis_timer: PeriodicTimer }` | Metadata for NGrid |
| `GridState` | `enum GridState { Invalid, Active, Idle, Removal }` | State machine (not virtual class hierarchy) |
| `GridState::Update(&Map, &NGrid, &GridInfo, uint32)` | `fn update_state(state: &mut GridState, grid: &mut NGrid, map: &Map, diff: u32)` | Pattern-matched enum dispatch |
| `InvalidState`, `ActiveState`, `IdleState`, `RemovalState` | Enum variants with match arms | No subclasses needed in Rust |
| `GridReference<T>` | `Arc<RwLock<T>>` or intrusive `LinkedListNode<T>` | Ownership tracking for entities |
| `GridRefManager<T>` | `Vec<Arc<RwLock<T>>>` or custom intrusive list | Container for GridReference nodes |
| `ObjectGridLoaderBase` | `struct ObjectGridLoaderBase { cell, grid, map, counts }` | Base data structure (no inheritance) |
| `ObjectGridLoader` | `impl ObjectGridLoader { fn visit_creatures(...), visit_gameobjects(...), load_n(...) }` | Methods, not virtual inheritance |
| `ObjectGridStoper` | `struct ObjectGridStoper; impl Visitor for ObjectGridStoper { fn visit_creatures(...) }` | Visitor pattern via trait impl |
| `typedef AllWorldObjectTypes` | `type AllWorldObjectTypes = (Player, Creature, Corpse, DynamicObject);` | Tuple or enum variant aggregate |
| `typedef AllGridObjectTypes` | `type AllGridObjectTypes = (GameObject, Creature, ..., Conversation);` | — |
| `grid.GetGridState()` | `grid.state` or `grid.get_state()` | Direct field or method |
| `grid.SetGridState(state)` | `grid.state = state;` | Direct assignment or setter |
| `info.getUnloadLock()` | `info.unload_locked()` → `self.unload_locks > 0 \|\| self.explicit_lock` | Boolean method |
| `Trinity::ComputeGridCoord(x, y)` | `fn world_to_grid(x: f32, y: f32) -> GridCoord` | Pure function in util |
| `NGrid::GetGridType(x, y)` | `ngrid.cells[x as usize][y as usize]` | Direct indexing with bounds |
| `Cell::AddWorldObject(obj)` | `cell.world_objects.insert(obj)` | TypeMapContainer or Vec insert |
| `Cell::Visit(visitor)` | `for obj in cell.world_objects.iter() { visitor.visit(obj) }` | Trait pattern matching |

---

*Template version: 1.0 (2026-05-01).* Revision: initial complete audit port.

---

## 13. Audit (2026-05-01)

Audited C++ tree: `/home/server/woltk-trinity-legacy/src/server/game/Grids/{GridDefines.h:251, NGrid.h:183, GridStates.h:56, GridStates.cpp:65, ObjectGridLoader.{h:126,cpp:283}, GridReference.h:51, GridRefManager.h:38, GridLoader.h:76, Grid.h:142}`. Audited Rust tree: grid code lives entirely inside `crates/wow-world/src/map_manager.rs`. There is no `grid.rs`, no `ngrid.rs`, no `cell.rs`, no `grid_state.rs`, and `crates/wow-map/src/lib.rs` is empty.

### 13.1 Coverage table

| C++ symbol (file:line) | Rust equivalent | Status |
|---|---|---|
| `MAX_NUMBER_OF_GRIDS = 64` (GridDefines.h:38) | None — `GridCoord { x: i16, y: i16 }` is unbounded (map_manager.rs:21) | ❌ |
| `MAX_NUMBER_OF_CELLS = 8` (GridDefines.h:36) | None — Rust has no Cell layer at all | ❌ |
| `SIZE_OF_GRIDS = 533.3333f` (GridDefines.h:40) | `GRID_SIZE = 64.0` (map_manager.rs:11) — **8.33× too small**; numerically equals C++ `SIZE_OF_GRID_CELL` (66.67), suggesting "grid" was confused with "cell" | ❌ wrong scale |
| `CENTER_GRID_ID = MAX_NUMBER_OF_GRIDS/2 = 32` (GridDefines.h:41) | None — no recentering | ❌ |
| `CENTER_GRID_OFFSET = SIZE_OF_GRIDS/2` (GridDefines.h:43) | None | ❌ |
| `SIZE_OF_GRID_CELL = 66.67f` (GridDefines.h:48) | None | ❌ |
| `CENTER_GRID_CELL_ID = 256` (GridDefines.h:50) | None | ❌ |
| `MAP_RESOLUTION = 128` (GridDefines.h:55) | None | ❌ |
| `MAP_SIZE = 34133.33` (GridDefines.h:57) | None | ❌ |
| `Trinity::ComputeGridCoord(x, y)` (GridDefines.h:194-204) — flips axes: `gx = (int)(CENTER_GRID_ID - x/SIZE_OF_GRIDS)`, returns `((MAX_NUMBER_OF_GRIDS-1) - gx, ...)` | `world_to_grid_x(x: f32) -> (x/64.0).floor() as i16` (map_manager.rs:620) — **no axis flip, no centering, no MAX-1 reorient** | ❌ wrong math |
| `Trinity::ComputeCellCoord(x, y)` (GridDefines.h:206) | None | ❌ |
| `class GridInfo` (NGrid.h:30-51) — `i_timer: TimeTracker`, `vis_Update: PeriodicTimer`, `i_unloadActiveLockCount: uint16`, `i_unloadExplicitLock: bool` | None — `Grid::last_player_time: Instant` (map_manager.rs:311) is the only timing field | ❌ |
| `enum grid_state_t { INVALID, ACTIVE, IDLE, REMOVAL }` (NGrid.h:53-60) | None — no state field on `Grid` (map_manager.rs:307) | ❌ |
| `class GridState` virtual base + `InvalidState/ActiveState/IdleState/RemovalState` (GridStates.h:26-55) | None | ❌ |
| `InvalidState::Update()` no-op (GridStates.cpp:24) | None | ❌ |
| `ActiveState::Update()` — calls `info.UpdateTimeTracker(diff)`, on expiry checks `GetWorldObjectCountInNGrid<Player>() && ActiveObjectsNearGrid()`, runs `ObjectGridStoper`, transitions to IDLE (GridStates.cpp:28-44) | None | ❌ |
| `IdleState::Update()` — `ResetGridExpiry`, transition to REMOVAL (GridStates.cpp:47-51) | None | ❌ |
| `RemovalState::Update()` — if `!info.getUnloadLock()` and timer passed, `map.UnloadGrid()` (GridStates.cpp:54-64) | None — `Grid::should_unload` :356 is a binary `empty + idle>timeout`, no lock concept | ⚠️ |
| `template<N, ACTIVE_OBJECT, WORLD_OBJECT_TYPES, GRID_OBJECT_TYPES> class NGrid` (NGrid.h:62-182) | None | ❌ |
| `NGrid::GetGridType(x,y)` returning `Grid<...>&` (NGrid.h:78) | None — Rust `Grid` is itself the leaf, no nested cell array | ❌ |
| `NGrid::SetGridState / GetGridState` (NGrid.h:91-92) | None | ❌ |
| `NGrid::isGridObjectDataLoaded() / setGridObjectDataLoaded()` (NGrid.h:100-101) | `Grid::loaded: bool` (map_manager.rs:312) — set to `true` at construction and never modified | ⚠️ vestigial |
| `NGrid::VisitAllGrids<T,TT>(visitor)` / `VisitGrid(x,y,visitor)` (NGrid.h:135-148) | None — visitor pattern absent | ❌ |
| `NGrid::GetWorldObjectCountInNGrid<T>()` (NGrid.h:163) | None — no typed counts | ❌ |
| `Grid<A, W, G>` template (Grid.h:142) — heterogeneous `TypeMapContainer` of `WORLD_OBJECT_TYPES` (Player/Creature/Corpse/DynamicObject) and `GRID_OBJECT_TYPES` (GameObject/Creature/AreaTrigger/SceneObject/Conversation) | `Grid { creatures: HashMap<ObjectGuid, WorldCreature>, player_guids: HashSet<ObjectGuid> }` (:307) — only creatures + player GUIDs; missing Corpse, DynamicObject, GameObject, AreaTrigger, SceneObject, Conversation | ❌ |
| `GridReference<T>` intrusive list node (GridReference.h:51) | None — Rust uses `HashMap<ObjectGuid, T>` (no intrusive backref from object → cell) | ❌ |
| `GridRefManager<T>` (GridRefManager.h:38) | None | ❌ |
| `GridLoader<...>` template (GridLoader.h:76) | None | ❌ |
| `class ObjectGridLoaderBase` (ObjectGridLoader.h:29) — `i_creatures, i_gameObjects, i_corpses, i_areaTriggers` counts | None | ❌ |
| `ObjectGridLoader::Visit(CreatureMapType&)` (ObjectGridLoader.cpp:138) | None | ❌ |
| `ObjectGridLoader::Visit(GameObjectMapType&)` (ObjectGridLoader.cpp:131) | None | ❌ |
| `ObjectGridLoader::Visit(AreaTriggerMapType&)` (ObjectGridLoader.cpp:145) | None | ❌ |
| `ObjectGridLoader::LoadN()` (ObjectGridLoader.cpp:171) — iterates 8×8 cells, dispatches per-type loader | None | ❌ |
| `ObjectGridStoper::Visit(CreatureMapType&)` (ObjectGridLoader.cpp:248) — pre-IDLE: stop combat, drop dynobjects/areatriggers | None | ❌ |
| `ObjectGridUnloader::Visit<T>` (ObjectGridLoader.cpp:232) — pre-REMOVAL: cleanup-before-delete | `MapInstance::unload_empty_grids()` (map_manager.rs:430) just removes the HashMap entry; nothing calls cleanup, nothing despawns | ⚠️ |
| `PersonalPhaseGridLoader` (ObjectGridLoader.h:73, .cpp:200-230) | None | ❌ |
| `DEFAULT_VISIBILITY_NOTIFY_PERIOD = 1000` (NGrid.h:28) | None — Rust has no visibility-notify throttle; `get_visible_creatures` (:549) recomputes from scratch every call | ❌ |

### 13.2 Critical divergences

1. **Coordinate system is incompatible.** C++ stores grids in `CoordPair<MAX_NUMBER_OF_GRIDS=64>` (GridDefines.h:177) with origin re-centered: `gx = (MAX_NUMBER_OF_GRIDS-1) - (int)(CENTER_GRID_ID - world_x/SIZE_OF_GRIDS)` (GridDefines.h:201-203). World coord `(0, 0)` lands at grid `(31 or 32, 31 or 32)` — the map center. RustyCore's `world_to_grid_x = (x/64).floor() as i16` (map_manager.rs:620) puts world `(0,0)` at grid `(0,0)` and lets x/y go negative without bounds; the Trinity DB `creature.position_x` columns will hash to entirely different grids than the C++ server uses, breaking any future ObjectGridLoader port unless coords are renormalized.
2. **Grid scale 8.33× off.** `GRID_SIZE = 64.0f` (map_manager.rs:11) numerically matches `SIZE_OF_GRID_CELL = 66.67f` (GridDefines.h:48), not `SIZE_OF_GRIDS = 533.33f` (GridDefines.h:40). What Rust calls a "grid" is what C++ calls a "cell", and there is no enclosing 64-grid super-structure. A Trinity `creature.grid_id` (1..4096 across the 64×64 grid lattice) cannot be looked up.
3. **No state machine = no lifecycle.** The four-state lazy-load model (Invalid → first activate via `EnsureGridLoaded` → load DB spawns → Active → Idle on no-players-near → Removal → unload after expiry) is the entire reason Grids exists as a module. Rust collapses it to `loaded: bool` (always `true`) and `should_unload = empty AND idle>300s` (:356). Active spawn-point locks (`i_unloadActiveLockCount`), explicit locks (`i_unloadExplicitLock`), and `ResetGridExpiry(grid, 0.1f)` are all absent. Lookup in map_manager.rs returns no result for the keywords `GridState`, `GridInfo`, `unload_lock`, `Active`, `Idle`, `Invalid`, `Removal`.
4. **"Load triggers when player enters" semantics are nonexistent.** C++ `Map::EnsureGridLoaded(Cell)` (Map.cpp:325) creates the NGrid, runs `ObjectGridLoader::LoadN`, marks `setGridObjectDataLoaded(true)` so subsequent enters don't re-query DB. Rust `MapInstance::get_or_create_grid` (:388) just `HashMap::insert(GridCoord, Grid::new())` with empty creature/player maps — there is no DB query, no per-cell spawn fetch, no idempotent flag. The "stays in grid" half of the semantic doesn't exist either: `ResetGridExpiry` has no Rust counterpart.
5. **No type-segregated containers.** C++ separates `WORLD_OBJECT_TYPES = TYPELIST_4(Player, Creature, Corpse, DynamicObject)` from `GRID_OBJECT_TYPES = TYPELIST_7(GameObject, Creature, DynamicObject, Corpse, AreaTrigger, SceneObject, Conversation)` (GridDefines.h). Rust stores only `WorldCreature` keyed by GUID + a `HashSet<ObjectGuid>` of player GUIDs. GameObjects, AreaTriggers, DynamicObjects, Corpses, SceneObjects, Conversations have no home; the dispatcher silently drops them.
6. **No intrusive object→grid backref.** C++ `GridReference<T>` (GridReference.h:51) lets a `Creature*` know which `NGrid` it lives in for O(1) relocation/removal. Rust must scan every grid's HashMap to find a creature by GUID (only `MapInstance::get_creature(x, y, guid)` :422 with explicit (x,y) hint works in O(1)).

### 13.3 Verdict

🔧 **broken — confirmed.** This module is structurally a single flat HashMap of "tiles" wearing the name `Grid`. The four canonical sub-systems of the C++ Grids module — coordinate math (`Compute*`), `NGrid<8>` template, `GridInfo`+`GridState` 4-state machine, `ObjectGridLoader` DB loader — are all missing. None of the existing code can be incrementally upgraded; coordinates and scale are wrong, so even creature persistence keys won't line up with the world DB. Recommend treating tasks #GRIDS.1-3 (constants + ComputeGridCoord/ComputeCellCoord) as **a hard prerequisite** and rewriting before #GRIDS.5+. Keep the existing `Grid`/`MapInstance` types only as a temporary integration shim while the real `NGrid` lands; do not extend them.
