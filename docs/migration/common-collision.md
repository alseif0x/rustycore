# Migration: Collision (VMAP / MMAP / DynamicTree)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/common/Collision/`
> **Rust target crate(s):** `crates/wow-recastdetour/` (currently empty FFI scaffold), prospective new `crates/wow-collision/`
> **Layer:** L1 (infrastructure, sits under L3 Maps; consumed by L4 Spell/Combat/AI)
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete — 2026-05-01 audit; zero Rust collision code, all five sub-systems (BIH, RegularGrid, DynamicMapTree, StaticMapTree, VMapManager2/MMapManager) absent
> **Last updated:** 2026-05-01

---

## 1. Purpose

The `Collision/` module is TrinityCore's **runtime** geometry layer. It loads pre-baked WoW client geometry (`.vmtree` / `.vmtile` per map; `.mmap` / `.mmtile` for navmesh) and answers three questions every tick: *is point A in line of sight of point B?*, *what Z does the world have at (x, y)?*, and *if I shoot a ray from A toward B, where does it hit static + dynamic geometry?*. The system is composed of (a) a static spatial index per map (`StaticMapTree` containing a BIH of `ModelInstance`s pointing at shared `WorldModel`s), (b) a dynamic spatial index per map (`DynamicMapTree` = `RegularGrid2D<GameObjectModel, BIHWrap>` for currently-spawned doors/destructibles), and (c) the management layer (`VMapManager2`, `MMapManager`) that owns load/unload lifecycle keyed by mapId+(x,y) tile coords. **This is not build-time tooling**: every cast spell, every fall-damage check, every `Creature::IsWithinLOS`, every `Player::JumpTo`, every projectile ballistic trajectory, and every `MoveSplineInit::MoveTo` ground-snap runs through this module on the live server.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/DetourMemoryFunctions.h` | 34 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/common/Collision/BoundingIntervalHierarchy.h` | 394 | `BIH` template class: build / `intersectRay` / `intersectPoint` / serialization (Sunflow-derived BVH packed in `std::vector<uint32>`) |
| `src/common/Collision/BoundingIntervalHierarchy.cpp` | 309 | `BIH::buildHierarchy`, `subdivide`, `BuildStats::printStats`, `writeToFile`/`readFromFile` |
| `src/common/Collision/BoundingIntervalHierarchyWrapper.h` | 117 | `BIHWrap<T>`: bridge between `RegularGrid2D` cell node and `BIH` (lazy rebuild on insert/remove) |
| `src/common/Collision/RegularGrid.h` | 235 | `RegularGrid2D<T, Node>` template: 64×64 voxel grid, world↔cell coord math, `intersectRay` DDA traversal, `intersectZAllignedRay` fast-path |
| `src/common/Collision/VMapDefinitions.h` | 35 | `VMAP_MAGIC = 'VMAP'`, version constants, file-format magics |
| `src/common/Collision/DynamicTree.h` | 63 | `DynamicMapTree` public surface: `isInLineOfSight`, `getIntersectionTime`, `getObjectHitPos`, `getHeight`, `getAreaInfo`, `getAreaAndLiquidData`, `insert`/`remove`/`balance`/`update` |
| `src/common/Collision/DynamicTree.cpp` | 303 | `DynTreeImpl: RegularGrid2D<GameObjectModel, BIHWrap<GameObjectModel>>`, intersection callbacks (`DynamicTreeIntersectionCallback`, `…AreaInfoCallback`, `…LocationInfoCallback`), 200-ms rebalance timer |
| `src/common/Collision/Maps/MapTree.h` | 119 | `StaticMapTree` (BIH over `ModelInstance` array, `loadedTileMap`, `loadedSpawnMap` ref-counts), `LocationInfo`, `GroupLocationInfo`, `AreaInfo` POD |
| `src/common/Collision/Maps/MapTree.cpp` | 513 | `StaticMapTree::InitMap` (parse `.vmtree`), `LoadMapTile`/`UnloadMapTile` (parse `.vmtile`, ref-count spawns), `isInLineOfSight`, `getObjectHitPos`, `getHeight` (Z-aligned ray from +10), `GetLocationInfo`, `getIntersectionTime` (private; called by ray ops) |
| `src/common/Collision/Maps/MapDefines.h` | 165 | `map_fileheader`, `ZLiquidStatus`, `LiquidData`, `PositionFullTerrainStatus` (return type for combined area+liquid query), liquid header flag enums |
| `src/common/Collision/Maps/MapDefines.cpp` | 24 | `MapMagic = 'MAPS'`, `MapVersionMagic = 10`, `MapAreaMagic`, `MapHeightMagic`, `MapLiquidMagic` global constants |
| `src/common/Collision/Maps/MMapDefines.h` | 72 | `MMAP_MAGIC = 'MMAP'`, `MMAP_VERSION = 15`, `MmapTileHeader` (20 bytes, `static_assert`'d), `NavArea`/`NavTerrainFlag` enums (NAV_GROUND, NAV_WATER, NAV_MAGMA_SLIME, …) |
| `src/common/Collision/Models/ModelIgnoreFlags.h` | 34 | `enum class ModelIgnoreFlags : uint32 { Nothing=0, M2=1 }` (skip M2 doodads in spell LOS) |
| `src/common/Collision/Models/ModelInstance.h` | 89 | `ModelMinimalData` (flags/adtId/ID/iPos/iScale/iBound), `ModelSpawn` (+iRot, +name, file-IO), `ModelInstance` (+iInvRot/iInvScale/iModel pointer) |
| `src/common/Collision/Models/ModelInstance.cpp` | 225 | `ModelInstance::intersectRay` (transform ray to model space, delegate to `WorldModel::IntersectRay`), `intersectPoint`, `GetLocationInfo`, `GetLiquidLevel`, `ModelSpawn::readFromFile`/`writeToFile` |
| `src/common/Collision/Models/WorldModel.h` | 131 | `MeshTriangle`, `WmoLiquid` (height grid + flags), `GroupModel` (per-WMO-group BIH over triangles, optional liquid), `WorldModel` (BIH over `GroupModel`s, name, RootWMOID, Flags) |
| `src/common/Collision/Models/WorldModel.cpp` | 641 | `GroupModel::IntersectRay` (per-triangle Möller–Trumbore via BIH), `IsInsideObject` (downward-cast hit count parity), `GetLiquidLevel`, `WorldModel::IntersectRay`/`IntersectPoint`/`GetLocationInfo`, `readFile`/`writeFile` (`.wmo`/`.m2` baked binary) |
| `src/common/Collision/Models/GameObjectModel.h` | 102 | `GameObjectModelOwnerBase` interface (back-edge to `GameObject`: GetDisplayId, GetPosition, GetRotation, GetScale, IsInPhase, IsSpawned), `GameObjectModel` collision wrapper |
| `src/common/Collision/Models/GameObjectModel.cpp` | 300 | `GameObjectModel::Create` (lookup `GameObjectDisplayInfoEntry`, acquire `WorldModel`), `intersectRay` (phase + collision-enabled gates), `intersectPoint`, `GetLocationInfo`, `GetLiquidLevel`, `UpdatePosition` (re-bound after rotation/scale change), `LoadGameObjectModelList` |
| `src/common/Collision/Management/IVMapManager.h` | 125 | Abstract base: `loadMap`/`unloadMap`/`existsMap`, `isInLineOfSight`, `getHeight`, `getObjectHitPos`, `getAreaInfo`, `GetLiquidLevel`, `getAreaAndLiquidData`, `LoadResult` enum, `AreaAndLiquidData` POD, `VMAP_INVALID_HEIGHT = -100000.0f`, `VMAP_INVALID_HEIGHT_VALUE = -200000.0f` |
| `src/common/Collision/Management/VMapManager2.h` | 132 | `VMapManager2 : IVMapManager`, `iLoadedModelFiles : ModelFileMap` (refcounted `WorldModel*` cache), `iInstanceMapTrees : InstanceTreeMap` (one `StaticMapTree*` per mapId), `iParentMapData` (instance→continent fallback), `LoadedModelFilesLock : std::mutex`, `GetLiquidFlagsPtr`/`IsVMAPDisabledForPtr` injectable callbacks, `DisableTypes` bitmask (AREAFLAG / HEIGHT / LOS / LIQUIDSTATUS) |
| `src/common/Collision/Management/VMapManager2.cpp` | 382 | `loadMap` (instantiate StaticMapTree + LoadMapTile), `unloadMap` (variants by tile / by map), `acquireModelInstance`/`releaseModelInstance` (refcounted lazy load of `.wmo`/`.m2` baked files), all `is*/get*` queries forward to `StaticMapTree`, parent-map-id fallback for instances |
| `src/common/Collision/Management/VMapFactory.h` | 39 | Singleton holder: `createOrGetVMapManager()` returns global `VMapManager2*` |
| `src/common/Collision/Management/VMapFactory.cpp` | 41 | Singleton implementation |
| `src/common/Collision/Management/MMapManager.h` | 91 | `MMapData` (owns `dtNavMesh*`, `loadedTileRefs : MMapTileSet`, `navMeshQueries : NavMeshQuerySet` — *one query per (mapId, instanceId)* due to Detour thread-unsafety), `MMapManager` singleton: `loadMap`/`loadMapInstance`/`unloadMap`/`unloadMapInstance`, `GetNavMeshQuery`, `GetNavMesh` |
| `src/common/Collision/Management/MMapManager.cpp` | 361 | `loadMapData` (parse `mmaps/<mapId>.mmap`), `loadMap` (per-tile `.mmtile`, `dtNavMesh::addTile`), `unloadMap*` (reverse refcount), `GetNavMeshQuery` lazy-creates `dtNavMeshQuery` per instance, `MMAP_VERSION` validation against `MmapTileHeader` |
| `src/common/Collision/Management/MMapFactory.h` | 44 | `MMapFactory::createOrGetMMapManager()` singleton; `IsPathfindingEnabled(uint32 mapId)` config gate |
| `src/common/Collision/Management/MMapFactory.cpp` | 42 | Singleton + DB2 disable-list lookup |

**Total C++ lines in Collision/:** ~5,128 (.h + .cpp).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `BIH` | class | Bounding Interval Hierarchy: BVH-like spatial index packed into `std::vector<uint32>` (axis bits + offset bits in the high 3 bits of node[0]). Templated `build<BoundsFunc, PrimArray>`, `intersectRay<RayCallback>`, `intersectPoint<IsectCallback>`. Stack-based traversal, `MAX_STACK_SIZE = 64`. |
| `BIH::buildData` | struct | Build-time scratch: `indices[]`, `primBound[]`, `numPrims`, `maxPrims`. |
| `BIH::StackNode` | struct | Traversal stack frame: `node`, `tnear`, `tfar`. |
| `BIH::BuildStats` | class | Optional build stats: `numNodes`, `numLeaves`, `sumDepth`, `numLeavesN[6]`, `numBVH2`. |
| `AABound` | struct | `G3D::Vector3 lo, hi` (used during `subdivide`). |
| `BIHWrap<T>` | template class | `RegularGrid2D` Node: lazy `BIH::build` over a `std::vector<T const*>` (rebuilt when `unbalanced`). |
| `RegularGrid2D<T, Node, NodeCreator, BoundsFunc, PositionFunc>` | template class | 64×64 sparse grid (`Node* nodes[64][64]`), `HGRID_MAP_SIZE = 533.3333 * 64`, `CELL_SIZE = HGRID_MAP_SIZE/64`, `MemberTable : unordered_multimap<T*, Node*>`, DDA `intersectRay`. |
| `RegularGrid2D::Cell` | struct | `{ int x, y }` with `ComputeCell(fx, fy) = (fx/CELL_SIZE + 32, fy/CELL_SIZE + 32)`. |
| `DynamicMapTree` | class | Public façade for the per-map dynamic tree of `GameObjectModel`s. Pimpl over `DynTreeImpl`. |
| `DynTreeImpl` | struct (cpp) | `RegularGrid2D<GameObjectModel, BIHWrap<GameObjectModel>>` + `TimeTracker rebalance_timer`, `int unbalanced_times`, `CHECK_TREE_PERIOD = 200`. |
| `DynamicTreeIntersectionCallback` | struct (cpp) | Forwards `(Ray, GameObjectModel, distance)` to `obj.intersectRay(…, phaseShift, …)`. |
| `DynamicTreeAreaInfoCallback` | struct (cpp) | Forwards point query to `obj.intersectPoint(…)` with `phaseShift`. |
| `DynamicTreeLocationInfoCallback` | struct (cpp) | Returns `LocationInfo` + `hitModel*` for combined area+liquid. |
| `StaticMapTree` | class | One per loaded map: `BIH iTree`, `ModelInstance* iTreeValues`, `iLoadedTiles : unordered_map<uint32 packTileID, bool>`, `iLoadedSpawns : unordered_map<uint32, uint32>` (refcount), `iLoadedPrimaryTiles : vector<pair<i32,i32>>`. Non-copyable. |
| `StaticMapTree::TileFileOpenResult` | struct | `{ Name, FILE*, UsedMapId }` (parent-map fallback for instances). |
| `LocationInfo` | struct (VMAP::) | `int32 rootId`, `ModelInstance const* hitInstance`, `GroupModel const* hitModel`, `float ground_Z`. |
| `GroupLocationInfo` | struct (VMAP::) | `GroupModel const* hitModel`, `int32 rootId`. |
| `AreaInfo` | struct (VMAP::) | `bool result`, `float ground_Z`, `uint32 flags`, `int32 adtId`, `int32 rootId`, `int32 groupId`. |
| `MeshTriangle` | class (VMAP::) | `uint32 idx0, idx1, idx2` into `WorldModel::vertices`. |
| `WmoLiquid` | class (VMAP::) | `iTilesX × iTilesY` height grid + `iFlags[]` byte mask, `iCorner` lower corner, `iType` liquid type id; `GetLiquidHeight(pos, &h)`. |
| `GroupModel` | class (VMAP::) | One WMO group: `iBound : AABox`, `iMogpFlags` (0x8 outdoor, 0x2000 indoor), `iGroupWMOID`, `vertices`, `triangles`, `BIH meshTree`, optional `WmoLiquid* iLiquid`. |
| `WorldModel` | class (VMAP::) | One source asset (M2 or WMO): `Flags`, `RootWMOID`, `vector<GroupModel> groupModels`, `BIH groupTree`, `name`. Loaded via `readFile`. |
| `ModelMinimalData` | struct (VMAP::) | Common spawn-record header: `flags : uint8`, `adtId : uint8`, `ID : uint32`, `iPos`, `iScale`, `iBound`. |
| `ModelSpawn : ModelMinimalData` | struct | + `iRot : Vector3`, `name : string`, file IO. Stored in `.vmtile`. |
| `ModelInstance : ModelMinimalData` | class | + `iInvRot : Matrix3`, `iInvScale`, `WorldModel* iModel`. The actual entry in `BIH iTree`. |
| `ModelFlags` | enum (VMAP::) | `MOD_M2 = 1`, `MOD_HAS_BOUND = 1<<1`, `MOD_PARENT_SPAWN = 1<<2`. |
| `ModelIgnoreFlags` | enum class : uint32 | `Nothing = 0`, `M2 = 1` (skip doodads for spell LOS — unit LOS includes them). |
| `GameObjectModelOwnerBase` | abstract class | Owner interface: `IsSpawned`, `GetDisplayId`, `GetNameSetId`, `IsInPhase(PhaseShift)`, `GetPosition`, `GetRotation : Quat`, `GetScale`, `DebugVisualizeCorner`. |
| `GameObjectModel` | class | Wraps a `WorldModel*` with a placement (iPos/iInvRot/iInvScale), bounds, owner backref, `_collisionEnabled`, `isWmo`. Member of `DynTreeImpl`. |
| `IVMapManager` | abstract class | LOS/height/area/liquid query interface; consumer-facing API surface for the rest of the engine. |
| `VMapManager2 : IVMapManager` | class | Concrete: `iLoadedModelFiles`, `iInstanceMapTrees`, `iParentMapData`, `LoadedModelFilesLock` mutex, function-pointer hooks `GetLiquidFlagsPtr` / `IsVMAPDisabledForPtr`. |
| `ManagedModel` | class (.cpp internal) | Refcount wrapper around `WorldModel*` for `iLoadedModelFiles`. |
| `LoadResult` | enum class : uint8 | `Success`, `FileNotFound`, `VersionMismatch`, `ReadFromFileFailed`, `DisabledInConfig`. |
| `DisableTypes` | enum | `VMAP_DISABLE_AREAFLAG=1`, `_HEIGHT=2`, `_LOS=4`, `_LIQUIDSTATUS=8` (bitmask read from `disables` DB table). |
| `AreaAndLiquidData` | struct (VMAP::) | Combined-query return: `floorZ`, `Optional<AreaInfo>`, `Optional<LiquidInfo>`. |
| `AreaAndLiquidData::AreaInfo` / `::LiquidInfo` | struct | const-fielded payloads. |
| `MmapTileHeader` | struct (POD, 20 bytes, `static_assert`'d) | `mmapMagic = 'MMAP'`, `dtVersion = DT_NAVMESH_VERSION`, `mmapVersion = 15`, `size`, `usesLiquids`, `padding[3]`. Binary-stable on disk. |
| `NavArea` | enum | `NAV_AREA_EMPTY=0`, `NAV_AREA_GROUND=11`, `NAV_AREA_GROUND_STEEP=10`, `NAV_AREA_WATER=9`, `NAV_AREA_MAGMA_SLIME=8`. |
| `NavTerrainFlag` | enum | Bit-shifted complement of NavArea (used in Detour `dtQueryFilter`). |
| `MMapData` | struct (MMAP::) | Owns `dtNavMesh*`, `loadedTileRefs : unordered_map<uint32 packed, dtTileRef>`, `navMeshQueries : unordered_map<pair<mapId,instanceId>, dtNavMeshQuery*>`. |
| `MMapManager` | singleton class | `loadedMMaps : MMapDataSet`, `loadedTiles`, `parentMapData`, `thread_safe_environment`. |
| `ZLiquidStatus` | enum (MapDefines.h) | `LIQUID_MAP_NO_WATER=0`, `_ABOVE_WATER=1`, `_WATER_WALK=2`, `_IN_WATER=4`, `_UNDER_WATER=8`, `_OCEAN_FLOOR=0x10` — used by `Map::GetLiquidStatus`. |
| `LiquidData` | struct | `type_flags`, `entry`, `level`, `depth_level`. |
| `PositionFullTerrainStatus` | struct | `areaId`, `floorZ`, `outdoors`, `liquidStatus`, `Optional<AreaInfo>`, `Optional<LiquidData>` — the per-position summary `Map::GetFullTerrainStatusForPosition` returns. |
| `map_fileheader` | struct (POD) | ADT-tile baked file header: magic + version + per-section offsets/sizes (area / height / liquid / holes). |

---

## 4. Critical public methods / functions

### 4.1 BIH (`BoundingIntervalHierarchy.h`)

| Symbol | Purpose | Calls into |
|---|---|---|
| `BIH::build<BoundsFunc, PrimArray>(prims, getBounds, leafSize, printStats)` | Build BVH from `prims[]`. Computes per-prim AABB, calls `buildHierarchy`, fills `tree` + `objects`. Sunflow algorithm. | `subdivide`, `BuildStats::printStats` |
| `BIH::intersectRay<RayCallback>(ray, cb, &maxDist, stopAtFirst)` | Stack-based BVH traversal. Bit-trick: encodes axis (3 bits) + BVH2 flag (1 bit) + offset (28 bits) in `tree[node]`. Uses `floatToRawIntBits` reinterpret. | `RayCallback::operator()` (per-leaf prim) |
| `BIH::intersectPoint<IsectCallback>(p, cb)` | Same traversal but Z-aligned for area/floor queries. | `IsectCallback::operator()` |
| `BIH::primCount() const` | `objects.size()`. | — |
| `BIH::writeToFile(FILE*) / readFromFile(FILE*)` | Binary serialization (used by `WorldModel::writeFile`/`readFile`). | — |

### 4.2 RegularGrid2D / DynamicMapTree

| Symbol | Purpose | Calls into |
|---|---|---|
| `RegularGrid2D::insert(T const& v)` | Compute cell range from `BoundsFunc::getBounds`, insert into all overlapping cells, register in `MemberTable`. | `Node::insert`, `Cell::ComputeCell` |
| `RegularGrid2D::remove(T const& v)` | Walk `MemberTable` equal-range, remove from each cell, erase membership. | `Node::remove` |
| `RegularGrid2D::intersectRay(ray, cb, &maxDist, end)` | DDA traversal between origin-cell and end-cell, calling `Node::intersectRay` per visited cell. | `Node::intersectRay` |
| `RegularGrid2D::intersectZAllignedRay(ray, cb, &maxDist)` | Single-cell fast path for `(0,0,-1)` rays (used by `getHeight`). | `Node::intersectRay` |
| `DynamicMapTree::isInLineOfSight(start, end, phaseShift)` | Build ray, return `!callback.didHit()`. | `DynTreeImpl::intersectRay` |
| `DynamicMapTree::getIntersectionTime(ray, end, phaseShift, &maxDist)` | Returns first hit distance. | `DynTreeImpl::intersectRay` |
| `DynamicMapTree::getObjectHitPos(start, end, &resultHitPos, modifyDist, phaseShift)` | Hit-point with optional ±`modifyDist` adjustment along ray; assertion that distance is finite to prevent NaN-induced infinite loops in BIH. | `getIntersectionTime` |
| `DynamicMapTree::getHeight(x, y, z, maxSearchDist, phaseShift)` | Z-down ray, returns `v.z - maxSearchDist` on hit, `-finf` on miss. | `intersectZAllignedRay` |
| `DynamicMapTree::getAreaInfo(x, y, &z, phaseShift, &flags, &adtId, &rootId, &groupId)` | Point query via `intersectPoint`. | `DynamicTreeAreaInfoCallback` |
| `DynamicMapTree::getAreaAndLiquidData(x, y, z, phaseShift, reqLiquidType, &data)` | Combined area+liquid lookup; consults `VMapManager2::GetLiquidFlagsPtr` for type filtering. | `DynamicTreeLocationInfoCallback`, `VMapFactory::createOrGetVMapManager` |
| `DynamicMapTree::insert/remove/contains(GameObjectModel const&)` | Lifecycle of dynamic geometry. | `DynTreeImpl::insert/remove` |
| `DynamicMapTree::balance()` | Force rebuild of all dirty `BIHWrap` nodes. | `RegularGrid2D::balance` |
| `DynamicMapTree::update(uint32 diff)` | Throttled rebalance every 200 ms if `unbalanced_times > 0`. | `balance` |

### 4.3 StaticMapTree (`Maps/MapTree.h`)

| Symbol | Purpose | Calls into |
|---|---|---|
| `StaticMapTree::CanLoadMap(basePath, mapID, tileX, tileY, vm)` (static) | Probe `<mapID>_<tileX>_<tileY>.vmtile` exists + version-matches; respects `iParentMapData`. | `OpenMapTileFile` |
| `StaticMapTree::InitMap(fname)` | Open `<mapID>.vmtree`, allocate `iTreeValues : ModelInstance[N]`, deserialize root BIH (only the **structure** — instances themselves are tile-loaded). | `BIH::readFromFile` |
| `StaticMapTree::LoadMapTile(tileX, tileY, vm)` | Read `.vmtile` → list of `(spawnIdx, ModelSpawn)`; for each new spawn, `vm->acquireModelInstance(name)`, populate `iTreeValues[treeIdx]`; refcount `iLoadedSpawns`. | `VMapManager2::acquireModelInstance` |
| `StaticMapTree::UnloadMapTile(tileX, tileY, vm)` | Decrement refcount, call `vm->releaseModelInstance` once a spawn drops to zero. Sets `ModelInstance::setUnloaded()`. | `VMapManager2::releaseModelInstance` |
| `StaticMapTree::isInLineOfSight(p1, p2, ignoreFlags)` | Build ray, traverse `iTree.intersectRay` with a stop-at-first callback that calls `ModelInstance::intersectRay`. | `BIH::intersectRay`, `ModelInstance::intersectRay` |
| `StaticMapTree::getObjectHitPos(p1, p2, &resultHitPos, modifyDist)` | Same as DynamicMapTree's variant but on static geometry. | `getIntersectionTime` (private) |
| `StaticMapTree::getHeight(pPos, maxSearchDist)` | Z-down ray from `pPos + (0,0,maxSearchDist*0.1)` (or similar; see .cpp), returns floor z or `-finf`. | `BIH::intersectRay` |
| `StaticMapTree::getAreaInfo(pos, &flags, &adtId, &rootId, &groupId)` | `intersectPoint` over BIH; downward IsInsideObject test per leaf. | `BIH::intersectPoint`, `ModelInstance::intersectPoint` |
| `StaticMapTree::GetLocationInfo(pos, &info)` | Combined area + liquid model lookup. | `ModelInstance::GetLocationInfo` |
| `StaticMapTree::numLoadedTiles()` | `iLoadedTiles.size()`. | — |
| `StaticMapTree::packTileID(x,y)` (static) | `(x<<16) \| y` (note: `unpackTileID` masks `& 0xFF` — see notes §11). | — |

### 4.4 ModelInstance / WorldModel / GroupModel

| Symbol | Purpose | Calls into |
|---|---|---|
| `ModelInstance::intersectRay(ray, &maxDist, stopAtFirst, ignoreFlags)` | Skip if `MOD_M2 && ignoreFlags::M2`. Transform world ray → model space (translate by `-iPos`, rotate by `iInvRot`, scale by `iInvScale`), call `iModel->IntersectRay`, scale dist back. | `WorldModel::IntersectRay` |
| `ModelInstance::intersectPoint(p, info)` | Check `iBound.contains(p)`, transform to model space, `iModel->IntersectPoint` (downward cast inside-test). Updates `info.ground_Z` etc. | `WorldModel::IntersectPoint` |
| `ModelInstance::GetLocationInfo(p, info)` | Returns deepest `GroupModel*` containing `p` (for liquid lookup). | `WorldModel::GetLocationInfo` |
| `ModelInstance::GetLiquidLevel(p, info, &liqHeight)` | Forward to `info.hitModel->GetLiquidLevel`. | `GroupModel::GetLiquidLevel` |
| `ModelSpawn::readFromFile / writeToFile` | `.vmtile` per-spawn serialization. | — |
| `WorldModel::IntersectRay(ray, &dist, stopAtFirst, ignoreFlags)` | `groupTree.intersectRay` over `groupModels`, each delegating to `GroupModel::IntersectRay`. | `BIH::intersectRay`, `GroupModel::IntersectRay` |
| `WorldModel::IntersectPoint(p, down, &dist, info)` | Downward ray inside-test; `GroupModel::IsInsideObject` parity count. | `GroupModel::IsInsideObject` |
| `WorldModel::GetLocationInfo(p, down, &dist, &info)` | Find tightest WMO group containing `p`. | `GroupModel::IsInsideObject` |
| `WorldModel::readFile/writeFile` | `.wmo`/`.m2` baked binary (BIH + meshes + liquid). | `BIH::readFromFile/writeToFile`, `GroupModel::readFromFile`, `WmoLiquid::readFromFile` |
| `GroupModel::IntersectRay(ray, &dist, stopAtFirst)` | `meshTree.intersectRay` over triangles; per-triangle Möller–Trumbore in callback. | `BIH::intersectRay` |
| `GroupModel::IsInsideObject(pos, down, &z_dist)` | Downward ray vs triangle mesh; parity count. | per-triangle test |
| `GroupModel::GetLiquidLevel(pos, &liqHeight)` | Bilinear interp over `WmoLiquid`'s height grid. | `WmoLiquid::GetLiquidHeight` |

### 4.5 GameObjectModel

| Symbol | Purpose | Calls into |
|---|---|---|
| `GameObjectModel::Create(owner, dataPath)` (static) | Lookup `GameObjectDisplayInfo`, derive `.wmo`/`.m2` name, `vm->acquireModelInstance`, fill `iPos`/`iInvRot`/`iScale`/`iBound`, mark `_collisionEnabled = true`. | `VMapManager2::acquireModelInstance` |
| `GameObjectModel::intersectRay(ray, &maxDist, stopAtFirst, phaseShift, ignoreFlags)` | Skip if `!_collisionEnabled \|\| !owner->IsInPhase(phaseShift) \|\| !owner->IsSpawned()`; otherwise transform-and-delegate to `WorldModel`. | `WorldModel::IntersectRay` |
| `GameObjectModel::intersectPoint(p, info, phaseShift)` | Phase + bounds gate; forward to `WorldModel::IntersectPoint`. | `WorldModel::IntersectPoint` |
| `GameObjectModel::GetLocationInfo(p, info, phaseShift)` | Phase gate; forward. | `WorldModel::GetLocationInfo` |
| `GameObjectModel::UpdatePosition()` | Re-pull `owner->GetPosition()/GetRotation()/GetScale()`, recompute `iInvRot`/`iInvScale`/`iBound`. Called from `GameObject::SetPosition` etc. | — |
| `GameObjectModel::enableCollision(bool)` / `isCollisionEnabled()` | Runtime toggle (e.g., transport doors, scripted destructibles). | — |
| `LoadGameObjectModelList(dataPath)` | Read `vmaps/GameObjectModels.dtree` listing displayId → `.wmo`/`.m2` filename. | — |

### 4.6 VMapManager2 (consumer-facing API)

| Symbol | Purpose | Calls into |
|---|---|---|
| `VMapManager2::InitializeThreadUnsafe(mapData)` | Pre-startup: populate `iParentMapData` (instance map → parent continent) so parallel loads don't race the `unordered_map`. After this, `thread_safe_environment = false` is enforced. | — |
| `VMapManager2::loadMap(basePath, mapId, x, y)` | Lazy-instantiate `StaticMapTree` for `mapId`; call `tree->LoadMapTile(x, y, this)`. | `StaticMapTree::InitMap`, `LoadMapTile` |
| `VMapManager2::unloadMap(mapId, x, y)` / `unloadMap(mapId)` | Decrement / fully unload. Drops empty trees. | `StaticMapTree::UnloadMapTile`, dtor |
| `VMapManager2::existsMap(basePath, mapId, x, y)` | Used by `Map::EnsureGridLoaded` to know if vmap is even available before requesting load. | `StaticMapTree::CanLoadMap` |
| `VMapManager2::isInLineOfSight(mapId, x1, y1, z1, x2, y2, z2, ignoreFlags)` | If LOS calc disabled or mapId disabled → return `true` (permissive). Otherwise look up `iInstanceMapTrees[mapId]`, fall back to parent map, call `tree->isInLineOfSight`. **This is the LOS that Spell/Combat/AI/Vision call.** | `StaticMapTree::isInLineOfSight` |
| `VMapManager2::getObjectHitPos(mapId, x1,…,z2, &rx, &ry, &rz, modifyDist)` | Used by spell projectile clamping (`Spell::SelectImplicitTrajTargets`), follow-paths, GM teleport-no-clip. | `StaticMapTree::getObjectHitPos` |
| `VMapManager2::getHeight(mapId, x, y, z, maxSearchDist)` | Z resolver for `Map::GetHeight`. Returns `VMAP_INVALID_HEIGHT_VALUE = -200000.0f` on miss. | `StaticMapTree::getHeight` |
| `VMapManager2::getAreaInfo(mapId, x, y, &z, &flags, &adtId, &rootId, &groupId)` | Indoor/outdoor + WMO area override resolution. | `StaticMapTree::getAreaInfo` |
| `VMapManager2::GetLiquidLevel(mapId, x, y, z, reqType, &level, &floor, &type, &mogpFlags)` | WMO-internal liquid (vs ADT liquid in `TerrainInfo`). | `StaticMapTree::GetLocationInfo` |
| `VMapManager2::getAreaAndLiquidData(mapId, x, y, z, reqType, &data)` | Combined query (single tree traversal). | `StaticMapTree::GetLocationInfo` |
| `VMapManager2::acquireModelInstance(basePath, filename, flags)` | Refcounted lazy load of `.wmo`/`.m2`. Mutex-protected `iLoadedModelFiles`. | `WorldModel::readFile` |
| `VMapManager2::releaseModelInstance(filename)` | Decrement; delete on zero. | — |
| `VMapManager2::convertPositionToInternalRep(x, y, z) const` | World coords → vmap internal frame (Z-up vs Z-down convention swap; see notes §11). | — |
| `VMapManager2::getParentMapId(mapId) const` | Returns instance→continent fallback for vmap reuse (e.g., MapID 559 → 530 Outland). | — |
| `IVMapManager::setEnableLineOfSightCalc(bool)` / `setEnableHeightCalc(bool)` | Global kill-switches read from `vmap.enableLOS` / `vmap.enableHeight` config. | — |

### 4.7 MMapManager (consumer-facing API)

| Symbol | Purpose | Calls into |
|---|---|---|
| `MMapManager::InitializeThreadUnsafe(mapData)` | Pre-startup map metadata seed. | — |
| `MMapManager::loadMap(basePath, mapId, x, y)` | Lazy `loadMapData` (parses `<mapId>.mmap` master); for tile, parse `<mapId>_xx_yy.mmtile`, validate `MmapTileHeader`, `dtNavMesh::addTile`. | `loadMapData`, `dtNavMesh::addTile` |
| `MMapManager::loadMapInstance(basePath, meshMapId, instanceMapId, instanceId)` | Distinct: per (mapId, instanceId) it lazy-creates a `dtNavMeshQuery` (Detour query objects are not thread-safe; one per instance). | `dtAllocNavMeshQuery`, `dtNavMeshQuery::init` |
| `MMapManager::unloadMap(mapId, x, y)` / `unloadMap(mapId)` / `unloadMapInstance` | Reverse of load; `dtNavMesh::removeTile`, `dtFreeNavMesh`, free queries. | — |
| `MMapManager::GetNavMeshQuery(meshMapId, instanceMapId, instanceId)` | **Per-instance, NOT thread-safe.** Used by `PathGenerator::BuildPath`. | — |
| `MMapManager::GetNavMesh(mapId)` | Read-only navmesh access (used by `PathGenerator` filter setup). | — |
| `MMapFactory::IsPathfindingEnabled(mapId)` | Disable list lookup (`disables` table type 7). | `DisableMgr::IsDisabledFor` |

---

## 5. Module dependencies

**Depends on:**
- **`G3D-lib`** (`Vector3`, `AABox`, `Ray`, `Matrix3`, `Quat`, `fuzzyGt`, `fuzzyNe`, `finf`) — used everywhere as the math primitive.
- **`Recast/Detour`** (external library, vendored) — `dtNavMesh`, `dtNavMeshQuery`, `dtTileRef`, `DT_NAVMESH_VERSION`. Only `MMapManager.{h,cpp}` and `PathGenerator` depend on it.
- **`Define.h` (TC common)** — `TC_COMMON_API`, integer typedefs.
- **`Errors.h`** — `ASSERT` (used to trap NaN distances in `getObjectHitPos`).
- **`Timer.h`** — `TimeTracker` (200-ms rebalance throttle in `DynTreeImpl`).
- **`Hash.h`** — `pair<uint32, uint32>` hash specialization for `NavMeshQuerySet`.
- **`Optional.h`** — wraps `AreaInfo`/`LiquidInfo` in `AreaAndLiquidData`.
- **`PhaseShift`** (forward-decl from `src/server/game/Phasing/`) — passed through `DynamicMapTree` queries; only `GameObjectModel::intersectRay` actually uses it (via `owner->IsInPhase`).
- **DBC/DB2 stores** — `GameObjectDisplayInfoEntry` (looked up in `GameObjectModel::initialize`), `LiquidTypeEntry` (via `GetLiquidFlagsPtr` callback). Read indirectly via injected function pointer.
- **`disables` DB table** — checked by `IsVMAPDisabledForPtr` and `MMapFactory::IsPathfindingEnabled`.

**Depended on by:**
- **`src/server/game/Maps/Map.cpp`** — `Map::IsInLineOfSight`, `Map::GetHeight`, `Map::GetAreaInfo`, `Map::GetFullTerrainStatusForPosition`, `Map::EnsureGridLoaded` (calls `vm->loadMap`).
- **`src/server/game/Maps/TerrainInfo.cpp`** — wraps `VMapManager2` getters for ADT-tile + vmap composition.
- **`src/server/game/Movement/PathGenerator.cpp`** — `MMapManager::GetNavMeshQuery`, `dtNavMesh` filters, polygon raycast.
- **`src/server/game/Entities/Object/Object.cpp`** — `WorldObject::IsWithinLOS`, `WorldObject::IsWithinLOSInMap` (calls `_map->isInLineOfSight`).
- **`src/server/game/Entities/Unit/Unit.cpp`** — visibility checks, `Unit::SetVisible`.
- **`src/server/game/Spells/Spell.cpp`** — `Spell::CheckRange` LOS gate, `Spell::SelectImplicitTrajTargets` (`getObjectHitPos`), `Spell::SelectEffectTargetsByLineOfSight`.
- **`src/server/game/Spells/SpellEffects.cpp`** — projectile path validation.
- **`src/server/game/Movement/MovementGenerators/`** — `FollowMovementGenerator`, `ChaseMovementGenerator`, fleeing, point movement; all consume `getHeight` / `PathGenerator`.
- **`src/server/game/AI/CoreAI/`** — `IsInLOSForSpell`, `IsHostileTo` LOS gate.
- **`src/server/game/Entities/GameObject/GameObject.cpp`** — `GameObject::SetGoState`, `EnableCollision`, transport movement; mutate `GameObjectModel`.
- **`src/server/game/Maps/MapInstanced.cpp`** — instance-id propagation to `MMapManager::loadMapInstance`.

---

## 6. SQL / DB queries (if any)

The collision module itself **does not emit SQL queries**. It receives all geometry from baked binary files (`.vmtree`, `.vmtile`, `.wmo`, `.m2`, `.mmap`, `.mmtile`) produced by the offline `vmap4extractor` + `vmap4assembler` + `mmaps_generator` pipeline. The only DB interactions are indirect:

| Source | Purpose | DB |
|---|---|---|
| `disables` table (type 6 = vmap, type 7 = mmap, type 8 = LOS, etc.) | Per-mapId disable flags consumed via `VMapManager2::IsVMAPDisabledForPtr` and `MMapFactory::IsPathfindingEnabled`. | world |
| `gameobject_template` / `gameobject` | Read by `GameObject` ctor; values feed `GameObjectModel::Create` (displayId → model file). | world |

**DBC/DB2 stores read indirectly:**

| Store | What it loads | Read by |
|---|---|---|
| `GameObjectDisplayInfo.db2` | DisplayId → model filename, bounds | `GameObjectModel::initialize` |
| `LiquidType.db2` | Liquid type id → flags (water / lava / slime) | `VMapManager2::GetLiquidFlagsPtr` callback |
| `Map.db2` | Map id → instance/parent metadata (used to build `iParentMapData`) | `VMapManager2::InitializeThreadUnsafe` (caller fills the map) |

---

## 7. Wire-protocol packets (if any)

The collision module is purely server-internal — **no opcodes originate or terminate here**. It is consumed by other systems whose packets are observable:

| Opcode | Direction | Triggers / Consumes Collision |
|---|---|---|
| `SMSG_MISSILE_TRAJECTORY_COLLISION` (0x318d) | server → client | Sent when `Spell::SelectImplicitTrajTargets` finds early `getObjectHitPos` hit. |
| `CMSG_NOTIFY_MISSILE_TRAJECTORY_COLLISION` (0x26aa) | client → server | Client reports its own predicted collision; server cross-checks against vmap. |
| `SMSG_MOVE_SPLINE_DISABLE_COLLISION` / `_ENABLE_COLLISION` | server → client | Mirrors `GameObjectModel::enableCollision` for transports/doors. |
| `SMSG_MOVE_SET_COLLISION_HEIGHT` / `CMSG_MOVE_SET_COLLISION_HEIGHT_ACK` | bidirectional | Per-unit collision capsule height (mounting, transformation auras); not vmap-related but uses adjacent constants. |

Plus every spell, melee, and movement packet implicitly depends on collision having returned the right answer.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-recastdetour` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-collision` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-recastdetour/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-map/src/lib.rs` | `file` | 1 | 70 | `exists_active` | file exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**

- `crates/wow-recastdetour/src/lib.rs` — **0 bytes**. The crate exists with a `Cargo.toml` listing only `thiserror`; there is no FFI to Recast/Detour, no vmap loader, no BIH, no `DynamicMapTree`, no `StaticMapTree`, no `WorldModel`, nothing.
- No `crates/wow-collision/` exists.
- `crates/wow-map/src/lib.rs` — empty (per the prior grids.md audit).

**What's implemented:** Nothing. Searching the entire `crates/` tree for `vmap`, `VMAP`, `BIH`, `BoundingInterval`, `LineOfSight`, `isInLineOfSight`, `WorldModel`, `ModelInstance`, `GameObjectModel` (collision sense), `MapTree`, `MMapManager`, `dtNavMesh`, `dtNavMeshQuery`, `RegularGrid2D`, `RecastDetour`, `Recast`, `Detour` returns no implementation hits — only opcode-name string matches in `wow-constants/src/opcodes.rs` (e.g. `MissileTrajectoryCollision = 0x318d`, `MoveCollisionDisableAck`, `MoveSetCollisionHeight`) and one `LineOfSight = 71` enum constant in `wow-constants/src/spell.rs` (the spell-failure reason code, not a check). The `MOVEMENTFLAG_DISABLE_COLLISION = 0x20000000` bit is defined in `wow-constants/src/movement.rs:44` but is not consulted anywhere.

**What's missing vs C++:** All ~5,128 lines. Specifically:
- No spatial index of any kind (no BIH, no BVH, no kd-tree, no grid → BVH wrapper).
- No vmap file format parser (`.vmtree`, `.vmtile`, `.wmo`, `.m2`).
- No mmap file format parser (`.mmap`, `.mmtile`).
- No Recast/Detour binding (the `wow-recastdetour` crate is a 0-byte stub).
- No `IsInLineOfSight` callable from `wow-spell`, `wow-combat`, `wow-ai` — these crates currently must succeed every LOS check vacuously.
- No `getHeight` Z-resolver — falls back to whatever the Z the client sent in the movement packet, with no server-side cap.
- No `getObjectHitPos` projectile-path clamp — `Spell::SelectImplicitTrajTargets`-equivalent code in `wow-spell` cannot exist correctly.
- No `DynamicMapTree::insert` for spawned `GameObject`s — doors, destructibles, transports do not occlude.
- No phase-aware filtering — `PhaseShift` handling is also absent.
- No `acquireModelInstance` refcounted geometry cache — even if loaders were added, every spawn would reload its WMO from disk.

**Suspicious / likely divergent (hypotheses pre-audit):** Not applicable — there is no Rust code to diverge.

**Tests existing:** Zero. `cargo test -p wow-recastdetour` builds (because the crate compiles an empty `lib.rs`) and runs zero tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#COMMON_COLLISION.WBS.001** Cerrar la migracion auditada de `shared/DetourMemoryFunctions.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h`
  Rust target: `crates/wow-recastdetour`, `crates/wow-collision`, `crates/wow-spell`, `crates/wow-combat`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, split before scheduling).

- [ ] **#COLLISION.1** Create `crates/wow-collision/` skeleton (Cargo.toml, lib.rs with module stubs `bih`, `regular_grid`, `dynamic_tree`, `static_tree`, `models`, `manager`). Wire into workspace `Cargo.toml`. (L)
- [ ] **#COLLISION.2** Port `BoundingIntervalHierarchy.h` BIH packed-tree layout — `Vec<u32>` + `Vec<u32>` indices, `AABox` from `glam` or `wow-math`. Implement `build` over a generic primitive iterator. (H)
- [ ] **#COLLISION.3** Port `BIH::intersectRay` traversal (the bit-trick stack-based loop, including BVH2 nodes). Add `intersectPoint` for area queries. Match C++ `floatToRawIntBits` via `f32::to_bits`. (H)
- [ ] **#COLLISION.4** Port `BIH::writeToFile` / `readFromFile` so existing `.vmtree` / `.wmo` baked files load byte-identically. Endian-fix-up if needed. (M)
- [ ] **#COLLISION.5** Port `RegularGrid2D` (64×64 sparse cell grid, `HGRID_MAP_SIZE = 533.3333 * 64`, `Cell::ComputeCell` with center-32 offset). DDA `intersectRay` traversal between origin-cell and end-cell. (M)
- [ ] **#COLLISION.6** Port `BIHWrap<T>` (lazy rebuild adaptor: dirty flag + `Vec<*const T>` + lazy `BIH::build`). (M)
- [ ] **#COLLISION.7** Port `MeshTriangle`, `WmoLiquid` (height grid + flags), `GroupModel` (per-WMO-group BIH over triangles). Per-triangle Möller–Trumbore intersection callback. (H)
- [ ] **#COLLISION.8** Port `WorldModel` (BIH over `GroupModel`, `IntersectRay`, `IntersectPoint`, `GetLocationInfo`, `readFile`/`writeFile`). Validate `.wmo`/`.m2` baked binaries parse correctly. (H)
- [ ] **#COLLISION.9** Port `ModelMinimalData` / `ModelSpawn` / `ModelInstance` including ray world→model transform via `Matrix3` inverse rotation + scale. `MOD_M2` / `MOD_HAS_BOUND` / `MOD_PARENT_SPAWN` flags. (M)
- [ ] **#COLLISION.10** Port `StaticMapTree::InitMap` (parse `.vmtree`), `LoadMapTile`/`UnloadMapTile` (parse `.vmtile`, ref-count `iLoadedSpawns`). Match `packTileID = (x<<16) | y` exactly (note the asymmetric `unpackTileID` — see §11). (H)
- [ ] **#COLLISION.11** Port `StaticMapTree::isInLineOfSight`, `getObjectHitPos`, `getHeight`, `getAreaInfo`, `GetLocationInfo` query surface. (M)
- [ ] **#COLLISION.12** Port `ModelIgnoreFlags` enum and thread it through ray callbacks (used by spell LOS to skip M2 doodads). (L)
- [ ] **#COLLISION.13** Port `IVMapManager` trait + `LoadResult` enum + `AreaAndLiquidData` struct. Define `setEnableLineOfSightCalc` / `setEnableHeightCalc` global gates wired to `WorldServer.conf`. (M)
- [ ] **#COLLISION.14** Port `VMapManager2` concrete: `Arc<RwLock<HashMap<u32, Arc<StaticMapTree>>>>` for `iInstanceMapTrees`, `Arc<Mutex<HashMap<String, ManagedModel>>>` for refcounted `iLoadedModelFiles`. `iParentMapData` for instance→continent fallback. Inject `GetLiquidFlagsFn` / `IsVMAPDisabledForFn` as boxed closures. (H)
- [ ] **#COLLISION.15** Port `GameObjectModelOwnerBase` as a Rust trait; implement for an ECS-friendly view of `WorldGameObject`. `UpdatePosition` recomputation. (M)
- [ ] **#COLLISION.16** Port `GameObjectModel` placement type + `enableCollision` runtime toggle + phase gate. (M)
- [ ] **#COLLISION.17** Port `DynTreeImpl` (`RegularGrid2D<GameObjectModel, BIHWrap<GameObjectModel>>`) + `DynamicMapTree` façade with the three intersection callbacks. 200-ms rebalance timer. (H)
- [ ] **#COLLISION.18** Wire `Map`/`MapInstance` to call `VMapManager2::loadMap(mapId, x, y)` from the existing tile-load path; `unloadMap` on idle eviction. (M)
- [ ] **#COLLISION.19** Wire `Spell` cast / `Unit::IsWithinLOS` / `Unit::IsWithinLOSInMap` / `Creature::CanCreatureAttack` to `VMapManager2::isInLineOfSight` + `DynamicMapTree::isInLineOfSight` chain. **All currently-vacuous LOS checks become real here**; expect behavior changes (and bug discoveries) the moment this lands. (M)
- [ ] **#COLLISION.20** Wire spell projectile clamp to `getObjectHitPos` for `SPELL_TARGET_DEST_TRAJ` and missile-trajectory effects; emit `SMSG_MISSILE_TRAJECTORY_COLLISION` accordingly. (M)
- [ ] **#COLLISION.21** Wire `Map::GetHeight` / `Player::UpdateGroundPositionZ` / fall-damage to `VMapManager2::getHeight` + `DynamicMapTree::getHeight`. (M)
- [ ] **#COLLISION.22** Decide Recast/Detour binding strategy for **#COLLISION.MM**: either (a) FFI to vendored upstream `recastnavigation/recastnavigation` C++ via `cc` crate + `bindgen` (pragmatic), or (b) pure-Rust port (massive undertaking, unrealistic). Document the decision. (M for decision; H–XL for execution.)
- [ ] **#COLLISION.MM.1** Implement chosen Detour binding; expose `dtNavMesh`, `dtNavMeshQuery`, `dtTileRef`. (XL — split per submodule.)
- [ ] **#COLLISION.MM.2** Port `MmapTileHeader` (20 bytes, padding-stable) + `NavArea` / `NavTerrainFlag` enums. Validate `MMAP_VERSION = 15` matches existing `mmaps_generator` output we already have on disk. (L)
- [ ] **#COLLISION.MM.3** Port `MMapManager` singleton — `loadedMMaps : DashMap<u32, MMapData>`, `MMapData { nav_mesh: Box<dtNavMesh>, loaded_tile_refs, nav_mesh_queries }`. **One `dtNavMeshQuery` per (mapId, instanceId) — Detour is not thread-safe.** (H)
- [ ] **#COLLISION.MM.4** Port `MMapFactory::IsPathfindingEnabled(mapId)` + `disables` table type-7 lookup. (L)
- [ ] **#COLLISION.MM.5** Port `PathGenerator` (movement-side consumer of MMapManager) — out of scope for collision crate but blocked on this. (XL — separate doc.)
- [ ] **#COLLISION.23** Add config keys: `vmap.enableLOS` / `vmap.enableHeight` / `vmap.enableIndoorCheck` / `vmap.disabled_maps` to `WorldServer.conf` parsing, mirroring TC. (L)
- [ ] **#COLLISION.24** Add `Map::isOutdoors` / `Map::GetAreaInfo` / `Map::GetFullTerrainStatusForPosition` consumer surface in `wow-map`. (M)
- [ ] **#COLLISION.25** Port `convertPositionToInternalRep` (vmap internal coord frame is rotated/swapped vs world frame) — this trips up every fresh attempt. Match TrinityCore's transform exactly. (L but **mandatory** before any LOS works.)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#COMMON_COLLISION.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 1 files / 34 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-recastdetour`, `crates/wow-spell`. | `cargo test -p wow-ai && cargo test -p wow-combat && cargo test -p wow-recastdetour` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#COMMON_COLLISION.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 1 files / 34 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-recastdetour`, `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#COMMON_COLLISION.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 1 files / 34 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-recastdetour`, `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#COMMON_COLLISION.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 1 files / 34 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-recastdetour`, `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Every test below must be added to `crates/wow-collision/tests/` once the crate exists. Each maps directly to an invariant the C++ implementation maintains.

- [ ] Test: `BIH::build` over a fixed primitive set produces a binary-identical packed `tree: Vec<u32>` to the C++ reference (capture once from a known `.wmo`).
- [ ] Test: `BIH::intersectRay` returns the same hit distance (within 1e-5 ulps) as C++ for a curated ray battery against a sample BIH.
- [ ] Test: `RegularGrid2D::Cell::ComputeCell(0.0, 0.0) == (32, 32)` (center-of-map invariant).
- [ ] Test: `RegularGrid2D::Cell::ComputeCell(-17066.66, -17066.66) == (0, 0)` (lower corner invariant; `-MAX_NUMBER_OF_CELLS / 2 * SIZE_OF_GRIDS`).
- [ ] Test: `intersectRay` DDA visits every cell along a 45° ray exactly once (no skips, no doubles).
- [ ] Test: `StaticMapTree::packTileID(x, y) == (x << 16) | y` for `x, y` in `0..64`.
- [ ] Test: `StaticMapTree::LoadMapTile` is idempotent — loading then unloading the same tile leaves `iLoadedSpawns` empty and `iLoadedTiles[tile] == false`.
- [ ] Test: `StaticMapTree::LoadMapTile` then `LoadMapTile` of an overlapping tile-set increments `iLoadedSpawns` for shared spawn ids without re-allocating `WorldModel`.
- [ ] Test: `VMapManager2::acquireModelInstance` then `releaseModelInstance` round-trip leaves `iLoadedModelFiles` empty.
- [ ] Test: `VMapManager2::isInLineOfSight` returns `true` when `setEnableLineOfSightCalc(false)` (permissive-when-disabled invariant — admin escape hatch).
- [ ] Test: `VMapManager2::isInLineOfSight` returns the configured `IsVMAPDisabledFor(mapId, VMAP_DISABLE_LOS)` answer when set.
- [ ] Test: `VMapManager2::getHeight` returns `VMAP_INVALID_HEIGHT_VALUE = -200000.0f` (not `-finf`, not `-100000`) on miss.
- [ ] Test: `getHeight` against a known WMO interior returns the same Z (within 1e-3) as the C++ reference for ten sampled `(map, x, y, z)`.
- [ ] Test: `ModelInstance::intersectRay` with `MOD_M2` flag set and `ignoreFlags = ModelIgnoreFlags::M2` returns false even if geometric intersection exists (spell LOS skips doodads).
- [ ] Test: `DynamicMapTree::update(diff)` only rebalances when `unbalanced_times > 0` and `rebalance_timer.Passed()` (200-ms throttle).
- [ ] Test: `DynamicMapTree::insert` then `remove` of the same `GameObjectModel` leaves `contains` returning `false` and `MemberTable` empty.
- [ ] Test: `DynamicMapTree::isInLineOfSight` honors `PhaseShift` — model in different phase does not block.
- [ ] Test: `DynamicMapTree::getObjectHitPos` with `start == end` does not produce NaN (the `< 1e-10f` guard in C++ must be preserved or the BIH traversal will spin).
- [ ] Test: `DynamicMapTree::getHeight` with `maxSearchDist = 50` and a known dynamic platform at z=10 returns the platform z, not the static floor.
- [ ] Test: `MMapManager::GetNavMeshQuery(mapId, instanceMapId, instanceId)` returns distinct `dtNavMeshQuery` pointers for two distinct `(instanceMapId, instanceId)` pairs (thread-safety contract).
- [ ] Test: `MmapTileHeader` serializes to **exactly** 20 bytes (`static_assert` from C++).
- [ ] Test: A representative `.mmtile` from `mmaps_generator` parses with `mmapMagic == 0x4d4d4150`, `mmapVersion == 15`, `dtVersion == DT_NAVMESH_VERSION`.
- [ ] Test: `VMapManager2::convertPositionToInternalRep` round-trips with `convertPositionFromInternalRep` (or whatever inverse is added) to within float epsilon — this catches the Z-up/Z-down swap mistake that kills every fresh attempt.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 1 files / 34 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `crates/wow-recastdetour/` (currently empty FFI scaffold), prospective new `crates/wow-collision/` \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#COMMON_COLLISION.DIV.001` | `crates/wow-recastdetour` (`exists_empty`, 0 Rust lines) | 1 C++ files / 34 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#COMMON_COLLISION.DIV.002` | `crates/wow-collision` (`missing_declared_path`, 0 Rust lines) | 1 C++ files / 34 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#COMMON_COLLISION.DIV.003` | `crates/wow-recastdetour/src/lib.rs` (`exists_empty`, 0 Rust lines) | 1 C++ files / 34 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#COMMON_COLLISION.DIV.004` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 1 C++ files / 34 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#COMMON_COLLISION.DIV.005` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 1 C++ files / 34 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DetourMemoryFunctions.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **The `unpackTileID` bug is real.** `MapTree.h:82`: `unpackTileID(uint32 ID, …) { tileX = ID >> 16; tileY = ID & 0xFF; }` — note the mask is `& 0xFF`, not `& 0xFFFF`. This means tileY is silently truncated to [0, 255] on unpack, but pack uses no mask. `tileX` and `tileY` are both legitimately in [0, 63], so the bug doesn't fire in practice, but **do not "fix" it on port** — preserve the asymmetry, because if a future map ever produces tileY ≥ 256 the divergence would matter, and we want the Rust port to be byte-identical first, behaviorally-correct second.
- **`floatToRawIntBits` / `intBitsToFloat`.** BIH packs floats as `uint32` inside its `tree` vector to keep the storage homogeneous. Port via `f32::to_bits` / `f32::from_bits` — but watch out for sign-bit usage in `BIH::intersectRay`: `offsetFront[i] = floatToRawIntBits(dir[i]) >> 31` extracts the sign of the ray direction to pick which child to visit first. Subnormals and `±0.0` must be handled identically to C++ (test against negative zero specifically).
- **NaN-induces-infinite-loop assertion.** `DynamicMapTree::getObjectHitPos:207`: `ASSERT(maxDist < std::numeric_limits<float>::max())`. Without this guard, BIH ray traversal hangs. The Rust port must either keep the assertion (with `wow-logging` LOG-and-bail) or refuse the call with `Err(NotFinite)`.
- **Coordinate frame swap.** The vmap files are baked in a **left-handed** Y-up frame (M2/WMO native), but the world is right-handed Z-up. `VMapManager2::convertPositionToInternalRep` and `ModelInstance::intersectRay`'s `iInvRot` together compose the swap. Many initial ports get this wrong and ship with LOS that is *correct in the X axis but inverted in Y*; symptoms are "LOS works in Stormwind but is flipped in Orgrimmar". Consult `vmap4assembler` source for the canonical transform.
- **Refcount semantics in `iLoadedModelFiles`.** `acquireModelInstance` returns a `WorldModel*` and increments refcount; `releaseModelInstance(filename)` decrements. A `WorldModel` is shared across every `ModelInstance` that names it, including across maps. Don't make this per-map.
- **`StaticMapTree::iTreeValues` is a raw pointer to a heap array sized at `InitMap` time.** Spawns are populated lazily by `LoadMapTile`. An entry can be valid (tile loaded) or stale (`iModel == nullptr` after `setUnloaded()`). Rust port should likely use `Vec<Option<ModelInstance>>` indexed by `treeIdx`, but BIH's leaf payload contract is "an integer index into `objects`" — preserve that.
- **`PhaseShift` is currently absent from RustyCore.** All phase checks in `GameObjectModel::intersectRay` must be threaded through some `PhaseShift` shim or temporarily stubbed to "always in phase 1". Until phasing exists, dynamic-collision phase filtering is a no-op (this matches a fresh shard before any phasing content is enabled).
- **`VMAP_DISABLE_*` is per-map, not global.** A common mistake is to wire only the global `setEnableLineOfSightCalc` and ignore the per-map `IsVMAPDisabledFor(mapId, VMAP_DISABLE_LOS)`. Some maps (e.g. seasonal arenas, GM islands) intentionally disable LOS to avoid bad vmap data. The two gates must compose.
- **MMAP version drift.** `MMAP_VERSION = 15` in the C++ source. The `mmaps_generator` tool stamps generated `.mmtile` headers with this version. If RustyCore ports to a different Detour version, regenerate mmaps; do not "loosen" the version check in `MMapManager::loadMap` — that's how navmesh corruption ships to production.
- **`DynamicMapTree` uses `RegularGrid2D` — *not* the same grid as `Map`'s 64×64 grid system.** It happens to share `MAX_NUMBER_OF_GRIDS = 64` and `HGRID_MAP_SIZE = 533.3333 * 64`, but the cell origin and the population mechanism are completely independent. Don't fuse them. (See `grids.md` audit for what RustyCore's current "grid" thinks it is — `wow-collision` must not depend on it.)
- **`DynTreeImpl::balance()` is non-cheap.** It rebuilds *every* dirty `BIHWrap` cell, which can be hundreds of cells with thousands of GameObjects (raid instances at peak). The 200-ms throttle is load-bearing; call sites must not bypass it.
- **`getHeight` returns `-G3D::finf()` from `DynamicMapTree` on miss but `VMAP_INVALID_HEIGHT_VALUE = -200000.0f` from `VMapManager2`.** These are different sentinels and the consumer code in `Map::GetHeight` distinguishes them. Match exactly.
- **`AreaAndLiquidData::AreaInfo` and `VMAP::AreaInfo` are different types** (the latter is the legacy single-query result; the former is the combined-query payload). Both must exist in the Rust port; do not collapse.
- **Liquid type filtering via injected callback (`GetLiquidFlagsPtr`).** This indirection exists so the `common/Collision/` library doesn't depend on game DBC stores. Preserve it: the Rust API should accept a `Box<dyn Fn(u32) -> u32>` or generic, not hardcode a DB2 lookup inside the collision crate.
- **No baked-data vendoring decision yet.** RustyCore needs `.vmtree` / `.vmtile` / `.mmap` / `.mmtile` on disk to test any of this. Generating them requires running `vmap4extractor` against a 3.4.3 client install. Currently no plan exists for fixture data; first integration test will need a tiny synthetic map.
- **Performance hotspot.** In TC, `Map::IsInLineOfSight` is the single hottest call site in the world server during raids (hundreds of LOS queries per second per active grid). Any Rust port that allocates per-call (e.g. `Vec` for the traversal stack) will regress. The C++ uses a stack-allocated `StackNode stack[MAX_STACK_SIZE = 64]` — the Rust equivalent is `[MaybeUninit<StackNode>; 64]` or just a fixed array.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class BIH` | `pub struct Bih { tree: Vec<u32>, objects: Vec<u32>, bounds: AaBox }` (`crates/wow-collision/src/bih.rs`) | Keep packed-`Vec<u32>` layout; do **not** convert to enum nodes. |
| `BIH::build<BoundsFunc, PrimArray>` | `impl Bih { pub fn build<I, F>(prims: I, get_bounds: F, leaf_size: u32) where I: ExactSizeIterator, F: Fn(&I::Item) -> AaBox }` | Generic over iterator, not template. |
| `BIH::intersectRay<RayCallback>` | `pub fn intersect_ray<F>(&self, ray: &Ray, mut cb: F, max_dist: &mut f32, stop_at_first: bool) where F: FnMut(&Ray, u32, &mut f32, bool) -> bool` | Closure replaces functor. |
| `BIH::StackNode stack[64]` | `let mut stack: [StackNode; 64]; let mut top: usize = 0;` | Fixed array, no allocation. |
| `floatToRawIntBits(f) / intBitsToFloat(i)` | `f32::to_bits(f) / f32::from_bits(i)` | 1:1. |
| `G3D::Vector3` | `glam::Vec3` (or `wow_math::Vec3`) | Pick one and use everywhere. |
| `G3D::AABox` | `pub struct AaBox { lo: Vec3, hi: Vec3 }` | Add `contains`, `merge`, `intersect_ray` methods. |
| `G3D::Ray` | `pub struct Ray { origin: Vec3, direction: Vec3, inv_dir: Vec3 }` | Precompute `inv_dir` at construction (needed by BIH). |
| `G3D::Matrix3` | `glam::Mat3` | For `iInvRot`. |
| `G3D::Quat` | `glam::Quat` | For `GetRotation`. |
| `RegularGrid2D<T, Node>` | `pub struct RegularGrid2D<T, N> { nodes: Box<[[Option<N>; 64]; 64]>, member_table: HashMap<*const T, Vec<(u8,u8)>> }` | `Box` to avoid 8KB stack allocation. Avoid raw pointers if possible — use `T: Hash + Eq` keys. |
| `BIHWrap<T>` | `pub struct BihWrap<T> { items: Vec<T>, bih: Bih, dirty: bool }` | Lazy rebuild on `intersect_ray` if dirty. |
| `class WorldModel` | `pub struct WorldModel { flags: u32, root_wmo_id: u32, group_models: Vec<GroupModel>, group_tree: Bih, name: String }` | One per loaded `.wmo`/`.m2`. |
| `class GroupModel` | `pub struct GroupModel { bound: AaBox, mogp_flags: u32, group_wmo_id: u32, vertices: Vec<Vec3>, triangles: Vec<MeshTriangle>, mesh_tree: Bih, liquid: Option<WmoLiquid> }` | — |
| `class WmoLiquid` | `pub struct WmoLiquid { tiles_x: u32, tiles_y: u32, corner: Vec3, ty: u32, height: Vec<f32>, flags: Vec<u8> }` | Heap vectors, not raw `float*`. |
| `class ModelInstance` | `pub struct ModelInstance { minimal: ModelMinimalData, inv_rot: Mat3, inv_scale: f32, model: Option<Arc<WorldModel>> }` | `Arc<WorldModel>` for refcounted sharing. |
| `class GameObjectModel` | `pub struct GameObjectModel { collision_enabled: bool, bound: AaBox, inv_rot: Mat3, pos: Vec3, inv_scale: f32, scale: f32, model: Option<Arc<WorldModel>>, owner: Box<dyn GameObjectModelOwner>, is_wmo: bool }` | — |
| `class GameObjectModelOwnerBase` (abstract) | `pub trait GameObjectModelOwner` | Methods: `is_spawned`, `display_id`, `name_set_id`, `is_in_phase`, `position`, `rotation`, `scale`. |
| `class StaticMapTree` | `pub struct StaticMapTree { map_id: u32, tree: Bih, tree_values: Vec<Option<ModelInstance>>, spawn_indices: HashMap<u32, u32>, loaded_tiles: HashMap<u32, bool>, loaded_primary_tiles: Vec<(i32, i32)>, loaded_spawns: HashMap<u32, u32>, base_path: PathBuf }` | `Vec<Option<…>>` over raw heap array. |
| `class DynamicMapTree` (pimpl) | `pub struct DynamicMapTree { grid: RegularGrid2D<GameObjectModel, BihWrap<GameObjectModel>>, rebalance_timer: Duration, unbalanced_times: u32 }` | No pimpl; just be private. |
| `class IVMapManager` (abstract) | `pub trait VMapManager: Send + Sync` | Methods listed in §4.6. |
| `class VMapManager2 : IVMapManager` | `pub struct VMapManager2 { loaded_model_files: Mutex<HashMap<String, Arc<ManagedModel>>>, instance_map_trees: RwLock<HashMap<u32, Arc<StaticMapTree>>>, parent_map_data: HashMap<u32, u32>, get_liquid_flags: Box<dyn Fn(u32) -> u32 + Send + Sync>, is_vmap_disabled_for: Box<dyn Fn(u32, u8) -> bool + Send + Sync>, enable_los: AtomicBool, enable_height: AtomicBool }` | `Arc<RwLock<…>>` instead of raw `std::mutex`. |
| `class ManagedModel` | `pub struct ManagedModel { model: Arc<WorldModel>, refcount: AtomicU32 }` | Or just `Arc<WorldModel>` and rely on `Arc::strong_count` — but be careful, `strong_count` is racy; explicit `AtomicU32` is safer. |
| `class MMapManager` | `pub struct MMapManager { loaded_mmaps: DashMap<u32, MMapData>, loaded_tiles: AtomicU32, parent_map_data: HashMap<u32, u32>, thread_safe_environment: AtomicBool }` | `DashMap` for sharded concurrency. |
| `struct MMapData` | `pub struct MMapData { nav_mesh: NavMeshHandle /* FFI */, loaded_tile_refs: HashMap<u32, DtTileRef>, nav_mesh_queries: HashMap<(u32,u32), NavMeshQueryHandle> }` | FFI handles wrap raw `*mut` from Detour. |
| `enum class LoadResult : uint8` | `#[repr(u8)] pub enum LoadResult { Success, FileNotFound, VersionMismatch, ReadFromFileFailed, DisabledInConfig }` | — |
| `enum class ModelIgnoreFlags : uint32` | `bitflags! { pub struct ModelIgnoreFlags: u32 { const NOTHING = 0; const M2 = 1; } }` | — |
| `enum NavArea / NavTerrainFlag` | `#[repr(u8)] pub enum NavArea { Empty=0, MagmaSlime=8, Water=9, GroundSteep=10, Ground=11 }` | — |
| `struct MmapTileHeader` (POD, 20 bytes) | `#[repr(C, packed)] pub struct MmapTileHeader { pub mmap_magic: u32, pub dt_version: u32, pub mmap_version: u32, pub size: u32, pub uses_liquids: u8, pub padding: [u8; 3] }` (`const _: () = assert!(size_of::<MmapTileHeader>() == 20);`) | Match C++ `static_assert`. |
| `VMAP_INVALID_HEIGHT_VALUE = -200000.0f` | `pub const VMAP_INVALID_HEIGHT_VALUE: f32 = -200_000.0;` | Match exact value. |
| `MMAP_MAGIC = 0x4d4d4150 / MMAP_VERSION = 15` | `pub const MMAP_MAGIC: u32 = 0x4d4d_4150; pub const MMAP_VERSION: u32 = 15;` | — |
| `class TimeTracker` | `std::time::Instant` + duration arithmetic | TC's `TimeTracker` is just a "millis remaining" countdown; trivial to replace. |
| `std::mutex / std::unordered_map` | `parking_lot::Mutex / dashmap::DashMap` | Per `AGENTS.md` workspace convention. |
| `dtNavMesh / dtNavMeshQuery / dtTileRef` (Detour) | FFI via `bindgen`; `unsafe` newtype wrappers; `Send` impl gated on Detour's actual thread-safety contract (mesh = Sync, query = !Sync) | Per-`(mapId, instanceId)` query is NOT Send across threads. Enforce in the type system. |

---

## 13. Audit (2026-05-01)

Audited C++ tree: `/home/server/woltk-trinity-legacy/src/common/Collision/` — 23 files, **5,128 total lines** across `BoundingIntervalHierarchy.{h:394,cpp:309}`, `BoundingIntervalHierarchyWrapper.h:117`, `RegularGrid.h:235`, `VMapDefinitions.h:35`, `DynamicTree.{h:63,cpp:303}`, `Maps/{MapTree.{h:119,cpp:513},MapDefines.{h:165,cpp:24},MMapDefines.h:72}`, `Models/{ModelIgnoreFlags.h:34,ModelInstance.{h:89,cpp:225},WorldModel.{h:131,cpp:641},GameObjectModel.{h:102,cpp:300}}`, `Management/{IVMapManager.h:125,VMapManager2.{h:132,cpp:382},VMapFactory.{h:39,cpp:41},MMapManager.{h:91,cpp:361},MMapFactory.{h:44,cpp:42}}`. Audited Rust tree: full search of `/home/server/rustycore/crates/` for `vmap` / `VMAP` / `BIH` / `BoundingInterval` / `LineOfSight` / `WorldModel` / `ModelInstance` / `MapTree` / `MMapManager` / `dtNavMesh` / `RegularGrid2D` / `GameObjectModel` (collision sense) / `Recast` / `Detour` / `IsInLineOfSight` / `getObjectHitPos` / `getHeight` (vmap sense) / `acquireModelInstance` / `getAreaInfo`. **Zero implementation hits.** The only matches are: opcode-name strings in `crates/wow-constants/src/opcodes.rs` (`MissileTrajectoryCollision`, `MoveCollisionDisableAck`, `MoveSetCollisionHeight`, `NotifyMissileTrajectoryCollision`, `MoveSplineDisableCollision`, etc.), the `LineOfSight = 71` spell-failure constant in `crates/wow-constants/src/spell.rs:162`, the `MOVEMENTFLAG_DISABLE_COLLISION = 0x20000000` bit in `crates/wow-constants/src/movement.rs:44`, two unrelated `creature.rs` flag bits, two cryptography "no Ed25519 collisions" string constants in `crates/wow-crypto/src/ed25519ctx.rs`, and a comment "Area Trigger system — collision detection and teleportation" in `crates/wow-data/src/area_trigger.rs:6`. None of these constitute collision logic. The `crates/wow-recastdetour/` crate exists but `src/lib.rs` is **0 bytes** and `Cargo.toml` declares only `thiserror` as a dependency; no FFI, no bindings, no scaffolding beyond the empty file.

### 13.1 Coverage table

| C++ symbol (file:line) | Rust equivalent | Status |
|---|---|---|
| `class BIH` (BoundingIntervalHierarchy.h:65) | None | ❌ |
| `BIH::build` (.h:79) | None | ❌ |
| `BIH::intersectRay` (.h:116) | None | ❌ |
| `BIH::intersectPoint` (.h:254) | None | ❌ |
| `BIH::writeToFile/readFromFile` (.h:331-332) | None | ❌ |
| `BIHWrap<T>` (BoundingIntervalHierarchyWrapper.h:117 entire file) | None | ❌ |
| `RegularGrid2D<T, Node>` (RegularGrid.h:39) | None | ❌ |
| `RegularGrid2D::insert/remove/balance` (RegularGrid.h:67/84/92) | None | ❌ |
| `RegularGrid2D::intersectRay` (DDA, RegularGrid.h:134) | None | ❌ |
| `RegularGrid2D::intersectZAllignedRay` (RegularGrid.h:222) | None | ❌ |
| `Cell::ComputeCell` (RegularGrid.h:111) — center-32 offset | None | ❌ |
| `class DynamicMapTree` (DynamicTree.h:38) | None | ❌ |
| `DynamicMapTree::isInLineOfSight` (DynamicTree.h:47) | None | ❌ |
| `DynamicMapTree::getObjectHitPos` (DynamicTree.h:49) | None | ❌ |
| `DynamicMapTree::getHeight` (DynamicTree.h:51) | None | ❌ |
| `DynamicMapTree::getAreaAndLiquidData` (DynamicTree.h:53) | None | ❌ |
| `DynamicMapTree::insert/remove/balance/update` (DynamicTree.h:55-60) | None | ❌ |
| `DynTreeImpl` (DynamicTree.cpp:61) — 200ms rebalance throttle | None | ❌ |
| `DynamicTreeIntersectionCallback` (DynamicTree.cpp:140) | None | ❌ |
| `class StaticMapTree` (MapTree.h:48) | None | ❌ |
| `StaticMapTree::InitMap` (MapTree.h:94) | None | ❌ |
| `StaticMapTree::LoadMapTile/UnloadMapTile` (MapTree.h:96-97) | None | ❌ |
| `StaticMapTree::isInLineOfSight` (MapTree.h:88) | None | ❌ |
| `StaticMapTree::getObjectHitPos` (MapTree.h:89) | None | ❌ |
| `StaticMapTree::getHeight` (MapTree.h:90) | None | ❌ |
| `StaticMapTree::getAreaInfo / GetLocationInfo` (MapTree.h:91-92) | None | ❌ |
| `StaticMapTree::CanLoadMap` (MapTree.h:83) | None | ❌ |
| `StaticMapTree::packTileID/unpackTileID` (MapTree.h:81-82) | None | ❌ |
| `LocationInfo` / `GroupLocationInfo` / `AreaInfo` (MapTree.h:33-46, 106-115) | None | ❌ |
| `class WorldModel` (WorldModel.h:106) | None | ❌ |
| `WorldModel::IntersectRay/IntersectPoint/GetLocationInfo/readFile/writeFile` (WorldModel.h:114-118) | None | ❌ |
| `class GroupModel` (WorldModel.h:73) | None | ❌ |
| `GroupModel::IntersectRay / IsInsideObject / GetLiquidLevel` (WorldModel.h:85-87) | None | ❌ |
| `class WmoLiquid` (WorldModel.h:47) | None | ❌ |
| `class MeshTriangle` (WorldModel.h:36) | None | ❌ |
| `ModelMinimalData` / `ModelSpawn` (ModelInstance.h:42, 59) | None | ❌ |
| `class ModelInstance` (ModelInstance.h:70) | None | ❌ |
| `ModelInstance::intersectRay/intersectPoint/GetLocationInfo/GetLiquidLevel` (ModelInstance.h:76-79) | None | ❌ |
| `ModelFlags` enum — MOD_M2/MOD_HAS_BOUND/MOD_PARENT_SPAWN (ModelInstance.h:35-40) | None | ❌ |
| `enum class ModelIgnoreFlags : uint32` (ModelIgnoreFlags.h:34 entire file) | None | ❌ |
| `class GameObjectModel` (GameObjectModel.h:61) | None | ❌ |
| `class GameObjectModelOwnerBase` (GameObjectModel.h:46) | None | ❌ |
| `GameObjectModel::Create/intersectRay/UpdatePosition/enableCollision` (GameObjectModel.h:77-84) | None | ❌ |
| `LoadGameObjectModelList` (GameObjectModel.h:100) | None | ❌ |
| `class IVMapManager` (IVMapManager.h:68) abstract | None | ❌ |
| `enum class LoadResult : uint8` (IVMapManager.h:34) | None | ❌ |
| `struct AreaAndLiquidData` (IVMapManager.h:46) | None | ❌ |
| `VMAP_INVALID_HEIGHT = -100000.0f / VMAP_INVALID_HEIGHT_VALUE = -200000.0f` (IVMapManager.h:43-44) | None | ❌ |
| `class VMapManager2 : IVMapManager` (VMapManager2.h:65) | None | ❌ |
| `VMapManager2::loadMap/unloadMap/existsMap` (VMapManager2.h:91-95, 118) | None | ❌ |
| `VMapManager2::isInLineOfSight` (VMapManager2.h:97) | None | ❌ |
| `VMapManager2::getObjectHitPos` (VMapManager2.h:101) | None | ❌ |
| `VMapManager2::getHeight` (VMapManager2.h:102) | None | ❌ |
| `VMapManager2::getAreaInfo / GetLiquidLevel / getAreaAndLiquidData` (VMapManager2.h:106-108) | None | ❌ |
| `VMapManager2::acquireModelInstance / releaseModelInstance` (VMapManager2.h:110-111) | None | ❌ |
| `VMapManager2::convertPositionToInternalRep` (VMapManager2.h:83) | None | ❌ |
| `VMapManager2::getParentMapId` (VMapManager2.h:122) | None | ❌ |
| `enum DisableTypes` — VMAP_DISABLE_AREAFLAG/HEIGHT/LOS/LIQUIDSTATUS (VMapManager2.h:57-63) | None | ❌ |
| `iLoadedModelFiles : ModelFileMap` + mutex (VMapManager2.h:69, 74) | None | ❌ |
| `iInstanceMapTrees : InstanceTreeMap` (VMapManager2.h:70) | None | ❌ |
| `iParentMapData` instance→continent fallback (VMapManager2.h:71) | None | ❌ |
| `GetLiquidFlagsFn / IsVMAPDisabledForFn` injectable callbacks (VMapManager2.h:124-128) | None | ❌ |
| `VMapFactory::createOrGetVMapManager()` singleton (VMapFactory.h:39) | None | ❌ |
| `MMAP_MAGIC = 0x4d4d4150 / MMAP_VERSION = 15` (MMapDefines.h:24-25) | None | ❌ |
| `struct MmapTileHeader` 20-byte POD (MMapDefines.h:27) | None | ❌ |
| `enum NavArea / NavTerrainFlag` (MMapDefines.h:49, 63) | None | ❌ |
| `struct MMapData` (MMapManager.h:36) | None | ❌ |
| `class MMapManager` (MMapManager.h:59) | None | ❌ |
| `MMapManager::loadMap/loadMapInstance/unloadMap*` (MMapManager.h:66-70) | None | ❌ |
| `MMapManager::GetNavMeshQuery (per-instance, NOT thread-safe)` (MMapManager.h:73) | None | ❌ |
| `MMapManager::GetNavMesh` (MMapManager.h:74) | None | ❌ |
| `MMapFactory::IsPathfindingEnabled` (MMapFactory.h) | None | ❌ |
| `MMapFactory::createOrGetMMapManager()` singleton (MMapFactory.h) | None | ❌ |
| `struct map_fileheader` ADT-tile baked header (MapDefines.h:38) | None | ❌ |
| `enum ZLiquidStatus` (MapDefines.h:124) | None | ❌ |
| `struct LiquidData` (MapDefines.h:137) | None | ❌ |
| `struct PositionFullTerrainStatus` (MapDefines.h:145) | None | ❌ |
| `MapMagic = 'MAPS' / MapVersionMagic = 10` constants (MapDefines.cpp:24) | None | ❌ |

### 13.2 Critical divergences

There are no divergences to enumerate, because there is no Rust code. The audit instead surfaces the **operational implications** of the absent module.

1. **Line-of-sight is silently bypassed.** Every spell that should LOS-fail (out-of-LOS spells like `Polymorph`, every ranged hostile, every targeted heal across a wall) currently has no server-side gate. `wow-spell` and `wow-combat` either accept all targets unconditionally or fail-open on a stub. **Exploit:** any client capable of crafting a `CMSG_CAST_SPELL` with a target GUID can hit through walls at unbounded range. This is the single largest cheat surface in the current server.
2. **Server-authoritative Z is absent.** With no `getHeight`, the server cannot reject a movement packet whose Z is implausible (player on top of an Ironforge tower they cannot reach by jumping). **Exploit:** wall-climb / ceiling-walk via crafted `MSG_MOVE_*` packets — server has no "what *should* the floor be here?" reference to compare against.
3. **No projectile-trajectory clamping.** `Spell::SelectImplicitTrajTargets` cannot exist correctly without `getObjectHitPos`. Skill shots (`Polymorph`, `Charge`-with-obstacle, missile-trajectory abilities) can pass through geometry. **Exploit:** charge-through-walls, line-shot through pillars, `SMSG_MISSILE_TRAJECTORY_COLLISION` is never emitted because nothing computes it.
4. **GameObjects don't occlude.** Closed doors, drawbridges, transports, destructibles all pretend to be air to LOS/projectile/path queries. **Gameplay break:** instances with door-progression mechanics are trivially bypassable; transports are uncollideable.
5. **No fall-damage validation.** Without a Z-resolver, the server cannot detect when a player has fallen further than the Z they sent suggests (or, equivalently, has *not* fallen but claims they did to teleport). **Exploit:** zero-fall-damage haxxoring, fall-then-no-damage on jump scripts.
6. **No pathfinding for AI.** `wow-ai` cannot path around obstacles because `MMapManager` is absent. Creatures will use straight-line movement and get stuck on terrain. (Distinct exploit from above: this is a *gameplay quality* hole, not a cheat surface.)
7. **No phase-aware geometry.** Even when collision lands, until `PhaseShift` exists, dynamic geometry from differently-phased GameObjects will block LOS for everyone, breaking phased questlines.
8. **Recast/Detour binding decision is unmade.** Most realistic paths port the C++ Detour library via FFI (`cc` + `bindgen`) rather than rewriting it in Rust — Detour is ~20K lines of math-heavy navmesh code with subtle numerical contracts. The `crates/wow-recastdetour/` crate name suggests the original plan was FFI; that decision needs to be ratified and executed before any pathfinding consumer (`wow-ai`, movement generators) can land.

### 13.3 Verdict

❌ **not started — confirmed.** The collision module is the largest single missing infrastructure piece in RustyCore by *consequence*: every L4 game-logic crate (`wow-spell`, `wow-combat`, `wow-ai`, `wow-loot` AOE, `wow-pvp`) silently relies on its absence to "succeed" their LOS / range / target checks, and the resulting permissiveness is the dominant exploit class on this codebase. Recommend the migration roadmap promote `#COLLISION.*` ahead of any further L4 feature work, with the **minimum viable first-cut** being **#COLLISION.1-14, #COLLISION.18-19** (BIH + StaticMapTree + VMapManager2 + Map wiring + `IsInLineOfSight` plumbing) — about 30-40 hours of focused work — to remove the wall-hack exploit class. Dynamic geometry (#COLLISION.15-17), liquid (#COLLISION.7 fully), and MMAP/pathfinding (#COLLISION.MM.*) can land in subsequent waves. The empty `crates/wow-recastdetour/` should be left as the placeholder for the future MMAP work and not be confused with a starting point for vmaps; vmaps belong in a new `crates/wow-collision/` crate per #COLLISION.1.

---

*Template version: 1.0 (2026-05-01).* Revision: initial complete audit — ❌ not started, all 5,128 C++ lines unmigrated.
