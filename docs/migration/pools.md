# Migration: Pools

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Pools/` (`PoolMgr`, `QuestPoolMgr`)
> **Rust target crate(s):** `crates/wow-data/` (load `pool_template`, `pool_members`, `quest_pool_template`, `quest_pool_members`, `pool_quest_save`), `crates/wow-world/src/pools/` (the in-memory `PoolMgr` + per-`Map` `SpawnedPoolData`), `crates/wow-database/` (CHAR_INS/DEL_POOL_QUEST_SAVE prepared statements). No dedicated `wow-pools` crate yet.
> **Layer:** L7 (Game systems — depends on Spawn data L4, Maps L4, Creatures/GameObjects L4, Quests L6, ObjectMgr L1, GameEventMgr L7; depended on by Map spawn lifecycle and the daily/weekly/monthly reset scheduler)
> **Status:** ❌ not started — there is no `PoolMgr`, no `QuestPoolMgr`, no pool tables loader, no `SpawnedPoolData` per Map, no `PoolGroup<T>` template specializations. Every DB-defined creature/gameobject in `pool_members` is currently spawned unconditionally (or, more accurately, the spawn pipeline ignores pool membership entirely so all members spawn or none do, depending on how the spawn loader treats the rows).
> **Audited vs C++:** ❌ not audited
> **Audited vs Rust impl:** ✅ 2026-05-01 — see §13
> **Last updated:** 2026-05-01

---

## 1. Purpose

The Pools module is TrinityCore's spawn-rotation engine. A "pool" is a named bag of candidate spawns (creature spawn IDs, gameobject spawn IDs, or other pool IDs for hierarchical pool-of-pools) with a per-pool `MaxLimit` saying how many of them should be alive at any moment. When the world starts up, `PoolMgr` rolls each pool — picks `MaxLimit` members weighted by per-row `chance` (with explicit-chance and equal-chance buckets), spawns them, and remembers the choice in per-`Map` `SpawnedPoolData`. When a member dies and its respawn timer fires, `PoolMgr::UpdatePool` re-rolls the bag and may pick a *different* member next, producing the "rare NPC rotation" behaviour (e.g. Time-Lost Proto-Drake / Vyragosa). Pools are also how seasonal `game_event_pool` rotations work and how world-event-only spawns are gated. The sibling `QuestPoolMgr` does the same thing but for daily/weekly/monthly quests: each tick of the daily/weekly/monthly reset scheduler picks `numActive` quests from each pool's groups of quasi-equivalent quests, and the choice is persisted in `pool_quest_save` so the rotation survives restarts.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Pools/PoolMgr.cpp` | 935 | `prefix` |
| `game/Pools/PoolMgr.h` | 214 | `prefix` |
| `game/Pools/QuestPools.cpp` | 293 | `prefix` |
| `game/Pools/QuestPools.h` | 65 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Pools/PoolMgr.h` | 214 | `PoolTemplateData`, `PoolObject`, `Pool` (empty marker class for "pool-of-pools"), `SpawnedPoolData` (per-Map state), `PoolGroup<T>` template, `PoolMgr` singleton; explicit template specializations for `<Creature>`, `<GameObject>`, `<Pool>` |
| `src/server/game/Pools/PoolMgr.cpp` | 935 | `PoolGroup<T>::AddEntry/CheckPool/SpawnObject/Spawn1Object/ReSpawn1Object/DespawnObject/Despawn1Object/RemoveRespawnTimeFromDB/RemoveOneRelation`; `SpawnedPoolData` template specializations for AddSpawn/RemoveSpawn/IsSpawnedObject; `PoolMgr::LoadFromDB` (pool_template + pool_members×3 types + auto-spawn pools per map); `Initialize`/`SpawnPool`/`DespawnPool`/`UpdatePool`/`InitPoolsForMap`; circular-reference detector for pool-of-pools |
| `src/server/game/Pools/QuestPools.h` | 65 | `QuestPool` struct, `QuestPoolMgr` singleton, `IsQuestPooled`, `IsQuestActive`, `ChangeDailyQuests`/`ChangeWeeklyQuests`/`ChangeMonthlyQuests` |
| `src/server/game/Pools/QuestPools.cpp` | 293 | `RegeneratePool` (random selection algorithm), `SaveToDB`, `LoadFromDB` (the heavy validator that reconciles `quest_pool_members` schema with `pool_quest_save` saved state), `Regenerate` per period |

Out-of-tree touchpoints:
- `src/server/game/Maps/Map.cpp` — owns `std::unique_ptr<SpawnedPoolData> _poolData`, populated by `PoolMgr::InitPoolsForMap` on map load. Map respawn-time helpers consult pool membership before deleting respawn rows.
- `src/server/game/Globals/ObjectMgr.cpp` — `LoadCreatures` / `LoadGameobjects` consult `PoolMgr::IsPartOfAPool` to decide whether a row should be added to the immediate-spawn list or held back.
- `src/server/game/Events/GameEventMgr.cpp` — `game_event_pool` table joins pools to events; only event-active pools spawn during events.
- `src/server/game/Entities/Creature.cpp` and `GameObject.cpp` — on death/destruction, if the spawn is part of a pool, `Map::AddCreatureRespawnTime` / `AddGORespawnTime` ends in `PoolMgr::UpdatePool` instead of a direct respawn.
- `src/server/game/World/World.cpp` — daily/weekly/monthly reset hooks call `sQuestPoolMgr->ChangeDailyQuests()` etc.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `PoolTemplateData` | struct | One row of `pool_template`: `MaxLimit: u32` (how many to keep alive), `MapId: i32` (auto-derived from the first member loaded; `-1` until determined; ASSERT non-negative after load) |
| `PoolObject` | struct | One element of a `PoolGroup`: `guid: u64` (spawn DB GUID **or** child pool ID), `chance: f32` (0..100; `0` = equal-chance bucket, > 0 = explicit-chance bucket) |
| `Pool` | empty class | Type tag used as the third template specialization (`PoolGroup<Pool>`, `IsPartOfAPool<Pool>`) so pool-of-pools can be expressed in the same generic algorithm |
| `SpawnedPoolObjects` | typedef | `set<u64>` — currently-alive creature **or** gameobject spawn GUIDs (the `<Creature>` and `<GameObject>` specializations each get their own set) |
| `SpawnedPoolPools` | typedef | `map<u64 poolId, u32 spawnedCount>` — for pool-of-pools, count how many child slots are filled |
| `SpawnedPoolData` | class (per-Map) | The runtime state: which pool members are currently spawned. Owns `mSpawnedCreatures: set<u64>`, `mSpawnedGameobjects: set<u64>`, `mSpawnedPools: map<poolId, count>`. Non-copyable, non-movable, owned by `Map` |
| `PoolGroup<T>` | template class (specialized for `Creature`, `GameObject`, `Pool`) | Owns `ExplicitlyChanced: vector<PoolObject>` and `EqualChanced: vector<PoolObject>`. The algorithm rolls explicit-chance first; if no explicit-chance hit, picks uniformly from the equal-chance bucket |
| `PoolMgr` | singleton class | Loads pool tables; owns `mPoolTemplate: HashMap<u32, PoolTemplateData>`, `mPoolCreatureGroups: HashMap<u32, PoolGroup<Creature>>`, `mPoolGameobjectGroups: HashMap<u32, PoolGroup<GameObject>>`, `mPoolPoolGroups: HashMap<u32, PoolGroup<Pool>>`, `mCreatureSearchMap: BTreeMap<spawnId, poolId>`, `mGameobjectSearchMap`, `mPoolSearchMap`, `mAutoSpawnPoolsPerMap: HashMap<mapId, vector<poolId>>` |
| `QuestPool` | struct | One quest pool: `poolId: u32`, `numActive: u32`, `members: vector<vector<u32 questId>>` (each inner vector is a *group* of equivalent quests that always come and go together), `activeQuests: HashSet<u32>` |
| `QuestPoolMgr` | singleton class | Owns `_dailyPools: vector<QuestPool>`, `_weeklyPools`, `_monthlyPools`, `_poolLookup: HashMap<questId, *QuestPool>` |
| `SpawnObjectType` | enum | `SPAWN_TYPE_CREATURE`, `SPAWN_TYPE_GAMEOBJECT`, `SPAWN_TYPE_AREATRIGGER` (only the first two are pool-eligible) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `PoolMgr::LoadFromDB()` | The startup entry: load pool_template, then 3× `pool_members WHERE type = N` (0=creature, 1=gameobject, 2=pool), then derive `MapId` from members, validate chances and references, detect circular pool-of-pool refs, populate `mAutoSpawnPoolsPerMap` from `pool_template LEFT JOIN game_event_pool LEFT JOIN pool_members` (skip event-gated and child pools) | World DB queries; per-row validators that consult `sObjectMgr->GetCreatureData` / `GetGameObjectData` |
| `PoolMgr::Initialize()` | Currently the inner part of `LoadFromDB` — sets up the per-map auto-spawn list | — |
| `PoolMgr::InitPoolsForMap(Map*) -> unique_ptr<SpawnedPoolData>` | Called by `Map::Map` constructor: roll every pool registered for this mapId in `mAutoSpawnPoolsPerMap` and produce a fresh `SpawnedPoolData` | `SpawnPool` per pool id |
| `PoolMgr::SpawnPool(SpawnedPoolData&, u32 poolId)` | Top-level untyped spawn: dispatches to `<Pool>`, `<GameObject>`, `<Creature>` specializations in that order | the typed `SpawnPool<T>` |
| `PoolMgr::SpawnPool<T>(SpawnedPoolData&, u32 poolId, u64 trigger)` | Roll the typed `PoolGroup<T>` and call `SpawnObject(spawns, MaxLimit, trigger)`; `trigger != 0` means "we just despawned this guid, prefer not to re-pick it" | `PoolGroup<T>::SpawnObject` |
| `PoolMgr::DespawnPool(SpawnedPoolData&, u32 poolId, bool alwaysDeleteRespawnTime)` | Despawn every alive member of the pool | `PoolGroup<T>::DespawnObject` |
| `PoolMgr::UpdatePool<T>(SpawnedPoolData&, u32 poolId, u64 dbGuidOrPoolId)` | The respawn-callback path: if the pool is a child of a mother pool, re-roll the mother; else re-roll this pool, passing `dbGuidOrPoolId` as trigger so the same member is unlikely to be picked again | `IsPartOfAPool<Pool>`, `SpawnPool<Pool>` or `SpawnPool<T>` |
| `PoolMgr::UpdatePool(SpawnedPoolData&, u32 poolId, SpawnObjectType, u64 spawnId)` | Untyped dispatcher | typed `UpdatePool<T>` |
| `PoolMgr::IsPartOfAPool<T>(u64 dbGuidOrPoolId) -> u32` | Reverse lookup: is this spawn GUID (or child pool ID) a member of any pool? returns the parent pool ID or 0 | search in `mCreatureSearchMap` / `mGameobjectSearchMap` / `mPoolSearchMap` |
| `PoolMgr::IsPartOfAPool(SpawnObjectType, u64 spawnId)` | Untyped dispatcher | typed `IsPartOfAPool<T>` |
| `PoolMgr::IsEmpty(u32 poolId)` | Deep check: returns true if pool has no creature, no gameobject, and no non-empty child pool | `PoolGroup<T>::isEmptyDeepCheck` per type |
| `PoolMgr::CheckPool(u32 poolId)` | Validate that explicit-chance entries sum to ≤ 100 OR there is at least one equal-chance entry | `PoolGroup<T>::CheckPool` |
| `PoolMgr::IsSpawnedObject<T>(SpawnedPoolData&, u64)` | Per-map lookup | `SpawnedPoolData::IsSpawnedObject<T>` |
| `PoolMgr::GetPoolTemplate(u16 poolId) -> *PoolTemplateData` | Direct lookup | — |
| `PoolGroup<T>::AddEntry(PoolObject&, u32 maxentries)` | Insert into either `ExplicitlyChanced` or `EqualChanced` based on `chance > 0` | — |
| `PoolGroup<T>::CheckPool() -> bool` | Sum explicit chances; OK if sum ≤ 100 or `!EqualChanced.empty()` | — |
| `PoolGroup<T>::SpawnObject(SpawnedPoolData&, u32 limit, u64 triggerFrom)` | Until `spawnedCount(poolId) == limit`, pick a member (excluding triggerFrom if non-zero), call `Spawn1Object` | `Trinity::SelectRandomContainerElementIfExists`, `Spawn1Object` |
| `PoolGroup<T>::Spawn1Object(SpawnedPoolData&, PoolObject*)` | Specialization-specific: for Creature/GameObject, call `Map::AddCreatureRespawnTime(0)` style spawn-now; for Pool, recurse into `PoolMgr::SpawnPool<T_inner>` | `Map::SaveCreatureRespawnTime`, `ObjectMgr::AddCreatureToGrid`, etc. |
| `PoolGroup<T>::ReSpawn1Object` | Used when the same guid was triggered to respawn — calls `Spawn1Object` with respawn time | — |
| `PoolGroup<T>::DespawnObject(SpawnedPoolData&, u64 guid=0, bool alwaysDeleteRespawnTime)` | Despawn all alive members (or just the named one); when `guid=0` and `alwaysDeleteRespawnTime`, also drop respawn rows from DB | `Despawn1Object` |
| `PoolGroup<T>::RemoveOneRelation(u32 childPoolId)` | Used by circular-reference cleanup — drop one back-edge | — |
| `PoolGroup<T>::RemoveRespawnTimeFromDB(SpawnedPoolData&, u64 guid)` | Delete the per-spawn respawn time persisted in `creature_respawn` / `gameobject_respawn` | — |
| `SpawnedPoolData::IsSpawnedObject<T>(u64)` | Query for current-spawn membership | set/map lookup |
| `SpawnedPoolData::AddSpawn<T>(u64, u32 poolId)` | On spawn: add to set + bump `mSpawnedPools[poolId]` | — |
| `SpawnedPoolData::RemoveSpawn<T>(u64, u32 poolId)` | On despawn: erase from set + decrement `mSpawnedPools[poolId]` | — |
| `SpawnedPoolData::GetSpawnedObjects(u32 poolId) -> u32` | How many slots of this pool are currently filled | map lookup |
| `QuestPoolMgr::LoadFromDB()` | Load `quest_pool_members` JOIN `quest_pool_template`, slot quests into daily/weekly/monthly buckets per their `IsDaily/IsWeekly/IsMonthly` flag, load `pool_quest_save`, validate every saved active quest belongs to its pool, regenerate any pool whose saved state is incoherent, persist regenerations | World DB + Character DB; `RegeneratePool`, `SaveToDB` |
| `QuestPoolMgr::ChangeDailyQuests()` / `Weekly` / `Monthly` | Called by reset scheduler — `Regenerate(_dailyPools)` etc. | `RegeneratePool` per pool, transactional `SaveToDB` |
| `QuestPoolMgr::FindQuestPool(poolId) -> *QuestPool const` | Linear search across the three vectors — debug-only path | — |
| `QuestPoolMgr::IsQuestPooled(questId) -> bool` | Map lookup | — |
| `QuestPoolMgr::IsQuestActive(questId) -> bool` | If not pooled → true; else check `activeQuests` set | — |
| `RegeneratePool(QuestPool&)` (file-static) | The randomization algorithm: fisher-yates partial shuffle of `members` to pick `numActive` groups, then flatten each group's quests into `activeQuests` | `urand` |
| `SaveToDB(QuestPool const&, transaction)` | DELETE then INSERT — replace the entire pool's persisted rows | `CHAR_DEL_POOL_QUEST_SAVE`, `CHAR_INS_POOL_QUEST_SAVE` |

---

## 5. Module dependencies

**Depends on:**
- `ObjectMgr` — `GetCreatureData(spawnGuid)`, `GetGameObjectData(spawnGuid)`, `GetQuestTemplate(questId)`. Spawn data lookup is the only way to derive `MapId` from member rows.
- `Map` (direct) — `Map::AddCreatureRespawnTime`, `RemoveCreatureRespawnTime`, `SaveCreatureRespawnTime`, `RemoveGORespawnTime`, `SaveGORespawnTime`, `IsGridLoaded`, `RemoveAllObjectsInRemoveList`. The Map owns `SpawnedPoolData`.
- `Creature` / `GameObject` entities — `Creature::SaveRespawnTime`, `Creature::AddObjectToRemoveList`, `Creature::SetVisible`, mirror for GameObject.
- `GameEventMgr` — `game_event_pool` joins; `IsActiveEvent(eventId)` decides whether a pool spawns at startup.
- `Quest module` — `Quest::IsDaily/IsWeekly/IsMonthly` predicates; `Quest::IsDailyOrWeekly` for validation.
- `World DB` (read) — `pool_template`, `pool_members`, `quest_pool_template`, `quest_pool_members`, `game_event_pool`.
- `Character DB` (read+write) — `pool_quest_save`.
- `Trinity::Containers::SelectRandomContainerElementIfExists` and `urand` (uniform RNG) — uses the same RNG facility as the rest of the project (`crates/wow-utils` in Rust terms).
- `Timer.h` — `getMSTime`, `GetMSTimeDiffToNow` for load-time logging.

**Depended on by:**
- `Map::Map` ctor / `Map::Update` — invokes `InitPoolsForMap` and runs spawned-pool maintenance.
- `Creature::SetDeathState` / `GameObject::SetGoState` — when a pooled spawn dies, the respawn callback ends in `PoolMgr::UpdatePool` instead of a direct respawn.
- `ObjectMgr::LoadCreatures` / `LoadGameobjects` — query `IsPartOfAPool` to skip auto-add of pool-managed spawns to the map's static spawn list.
- World reset scheduler (in `World.cpp`'s `Update` tick) — `sQuestPoolMgr->ChangeDailyQuests` / `Weekly` / `Monthly` at the appropriate UTC boundaries.
- `Player::CanTakeQuest` — checks `sQuestPoolMgr->IsQuestActive(questId)` before offering pooled dailies/weeklies.
- `Player::SetDailyQuestStatus` / `SetWeeklyQuestStatus` / `SetMonthlyQuestStatus` — persist completion only against the currently-active pool roll.
- `.gobject` / `.npc` / `.pool` GM commands — admin debug.

---

## 6. SQL / DB queries (if any)

Schema (3.4.3 world DB):

```sql
CREATE TABLE pool_template (
  entry     INT UNSIGNED NOT NULL,
  max_limit INT UNSIGNED NOT NULL DEFAULT 0,
  description VARCHAR(255) DEFAULT NULL,
  PRIMARY KEY (entry)
);

CREATE TABLE pool_members (
  type        TINYINT UNSIGNED NOT NULL,  -- 0 creature, 1 gameobject, 2 pool
  spawnId     BIGINT UNSIGNED NOT NULL,
  poolSpawnId INT UNSIGNED NOT NULL,
  chance      FLOAT NOT NULL DEFAULT 0,
  description VARCHAR(255) DEFAULT '',
  PRIMARY KEY (type, spawnId)
);

CREATE TABLE quest_pool_template (
  poolId    INT UNSIGNED NOT NULL,
  numActive INT UNSIGNED NOT NULL DEFAULT 1,
  description VARCHAR(255) DEFAULT '',
  PRIMARY KEY (poolId)
);

CREATE TABLE quest_pool_members (
  questId    INT UNSIGNED NOT NULL,
  poolId     INT UNSIGNED NOT NULL,
  poolIndex  TINYINT UNSIGNED NOT NULL,
  description VARCHAR(255) DEFAULT '',
  PRIMARY KEY (questId)
);

CREATE TABLE game_event_pool (
  eventEntry SMALLINT NOT NULL,
  pool_entry INT UNSIGNED NOT NULL,
  PRIMARY KEY (eventEntry, pool_entry)
);

-- character DB
CREATE TABLE pool_quest_save (
  pool_id  INT UNSIGNED NOT NULL,
  quest_id INT UNSIGNED NOT NULL,
  PRIMARY KEY (pool_id, quest_id)
);
```

Loaded queries (raw, not prepared statements):

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, max_limit FROM pool_template` | Load templates | world |
| `SELECT spawnId, poolSpawnId, chance FROM pool_members WHERE type = 0` | Creature pool members | world |
| `SELECT spawnId, poolSpawnId, chance FROM pool_members WHERE type = 1` | GameObject pool members | world |
| `SELECT spawnId, poolSpawnId, chance FROM pool_members WHERE type = 2` | Pool-of-pool members | world |
| `SELECT DISTINCT pool_template.entry, pool_members.spawnId, pool_members.poolSpawnId FROM pool_template LEFT JOIN game_event_pool ... LEFT JOIN pool_members ...` | Build `mAutoSpawnPoolsPerMap` (pools that should auto-spawn at world start, i.e. not gated by an event and not a child of another pool) | world |
| `SELECT qpm.questId, qpm.poolId, qpm.poolIndex, qpt.numActive FROM quest_pool_members qpm LEFT JOIN quest_pool_template qpt ON qpm.poolId = qpt.poolId` | Quest pool template + members | world |
| `SELECT pool_id, quest_id FROM pool_quest_save` | Persisted quest-pool active set | character |

Prepared statements (`CharacterDatabase`):

| Statement | Purpose |
|---|---|
| `CHAR_INS_POOL_QUEST_SAVE` (`INSERT INTO pool_quest_save (pool_id, quest_id) VALUES (?, ?)`) | One INSERT per active quest after `RegeneratePool` |
| `CHAR_DEL_POOL_QUEST_SAVE` (`DELETE FROM pool_quest_save WHERE pool_id = ?`) | Wipe before re-insert (and used to clean unknown pool IDs found during load) |

No DB2/DBC stores are involved.

---

## 7. Wire-protocol packets (if any)

The Pools module emits no packets directly. Spawn/despawn events that pools cause are observed by clients via the standard Object Update mechanism (`SMSG_UPDATE_OBJECT` create/destroy blocks) when grids load or when the server explicitly hides/shows objects. Quest pool changes have no client-facing packet either — clients learn about active pooled quests through standard quest gossip / quest-poi opcodes the next time they interact with the relevant questgiver.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-world/src/pools` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-pools` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None for pools directly.

**What's implemented:**
- Nothing.

**What's missing vs C++:**
- `PoolMgr` singleton, `QuestPoolMgr` singleton, `SpawnedPoolData` per-Map state, `PoolGroup<T>` algorithm.
- All five world-DB tables loaders (`pool_template`, `pool_members`×3 types via type column, `quest_pool_template`, `quest_pool_members`, `game_event_pool` join semantics).
- `pool_quest_save` schema migration + prepared statements `CHAR_INS_POOL_QUEST_SAVE`, `CHAR_DEL_POOL_QUEST_SAVE`.
- `Map::pool_data` field; `Map` constructor invocation of `PoolMgr::init_pools_for_map`.
- `IsPartOfAPool<T>` lookups; spawn-time short-circuit in the (also-WIP) creature/gameobject loaders.
- Death/destroy → `UpdatePool` callback wiring.
- Daily/weekly/monthly reset scheduler hooks calling `QuestPoolMgr::Regenerate*`.
- `IsQuestPooled` / `IsQuestActive` integration with the quest-availability path (`Player::CanTakeQuest`).
- Circular-reference detector for pool-of-pool.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Without pools, the rare-NPC rotation feature is gone: every member of a pool spawns simultaneously, breaking the "one of these three rare elites is up at any given time" design.
- Without QuestPools, daily/weekly quest rotation is non-functional — once daily quests are wired (the quest module is partial), all pooled dailies will be available at once. This is *gameplay-breaking* for endgame dailies (Dalaran cooking, fishing, jewelcrafting daily, Argent Tournament etc.).
- The `pool_members` table currently uses a single table with `type` column (0/1/2). Some legacy migration scripts use separate `pool_creature` / `pool_gameobject` / `pool_pool` tables — confirm the SQL dump format the project ships.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#POOLS.WBS.001** Partir y cerrar la migracion auditada de `game/Pools/PoolMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-pools`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 935 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#POOLS.WBS.002** Cerrar la migracion auditada de `game/Pools/PoolMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-pools`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#POOLS.WBS.003** Cerrar la migracion auditada de `game/Pools/QuestPools.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-pools`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#POOLS.WBS.004** Cerrar la migracion auditada de `game/Pools/QuestPools.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-pools`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#POOLS.1** Define `PoolTemplateData { max_limit: u32, map_id: i32 }` in `crates/wow-data/src/pools.rs` (L)
- [ ] **#POOLS.2** Define `PoolObject { guid: u64, chance: f32 }` (L)
- [ ] **#POOLS.3** Define `PoolKind` marker enum or tag types (`Creature`, `GameObject`, `Pool`) so `PoolGroup` can be parameterized (Rust generics + a sealed trait, or a 3-variant enum) (M)
- [ ] **#POOLS.4** Implement `PoolGroup` with `explicitly_chanced: Vec<PoolObject>`, `equal_chanced: Vec<PoolObject>`, `pool_id: u32`; methods `add_entry`, `is_empty`, `is_empty_deep_check`, `check_pool`, `set_pool_id` (M)
- [ ] **#POOLS.5** Implement `PoolGroup::spawn_object(spawns, limit, trigger_from)` algorithm: while `spawned_count(poolId) < limit`, pick weighted-random from explicit-chance bucket if total ≥ rand_chance else uniform from equal-chance, excluding `trigger_from` (H)
- [ ] **#POOLS.6** Implement `PoolGroup::spawn_1_object` for the Creature variant: locate spawn data via ObjectMgr, place on grid, save respawn time to `creature_respawn` (H — depends on creature spawn pipeline)
- [ ] **#POOLS.7** Implement `PoolGroup::spawn_1_object` for the GameObject variant — same as #POOLS.6 for GO (H, parallel)
- [ ] **#POOLS.8** Implement `PoolGroup::spawn_1_object` for the Pool variant — recursive `PoolMgr::spawn_pool` of the child (M)
- [ ] **#POOLS.9** Implement `PoolGroup::despawn_object` and `despawn_1_object` for all three variants, including optional `always_delete_respawn_time` mode that drops persisted respawn rows (H)
- [ ] **#POOLS.10** Implement `PoolGroup::re_spawn_1_object` (used by the trigger path so a re-rolled member that happens to be the same guid still respects respawn timer) (M)
- [ ] **#POOLS.11** Implement `PoolGroup::remove_respawn_time_from_db` and `remove_one_relation` (the circular-reference cleanup helper) (M)
- [ ] **#POOLS.12** Implement `SpawnedPoolData { spawned_creatures: HashSet<u64>, spawned_gameobjects: HashSet<u64>, spawned_pools: HashMap<u64, u32>, owner: &Map }` with `add_spawn`/`remove_spawn`/`is_spawned_object`/`get_spawned_objects` (M)
- [ ] **#POOLS.13** Wire `SpawnedPoolData` ownership into `Map`: a `Map` field `pool_data: Option<SpawnedPoolData>` populated by `PoolMgr::init_pools_for_map` in the Map constructor (M)
- [ ] **#POOLS.14** Implement `PoolMgr::load_from_db` — orchestrate the 4 main queries (pool_template, pool_members×3 by type) and the auto-spawn-per-map join; per-row validators (chance ∈ [0,100], pool_id exists, spawn data exists, mapId consistent) (XL — split: template loader, creature member loader, gameobject member loader, pool-pool member loader, auto-spawn map builder, circular reference detector)
- [ ] **#POOLS.15** Implement circular-reference detector for pool-of-pool: `for each pool, walk parent chain, detect revisits; on detect, RemoveOneRelation + erase from search map` (M)
- [ ] **#POOLS.16** Implement `PoolMgr::is_part_of_a_pool::<T>(id) -> u32` for Creature/GameObject/Pool — read from per-type `SearchMap` (M)
- [ ] **#POOLS.17** Implement `PoolMgr::spawn_pool` (untyped + typed) and `despawn_pool` (M)
- [ ] **#POOLS.18** Implement `PoolMgr::update_pool::<T>(spawns, pool_id, db_guid_or_pool_id)` — the respawn callback path with mother-pool delegation (M)
- [ ] **#POOLS.19** Implement `PoolMgr::init_pools_for_map(map) -> SpawnedPoolData` — for each pool ID in `auto_spawn_pools_per_map[map.id]`, call `spawn_pool` (M)
- [ ] **#POOLS.20** Implement `PoolMgr::is_empty(pool_id)` and `check_pool(pool_id)` deep-checks (L)
- [ ] **#POOLS.21** Wire creature/gameobject loaders in `wow-data` to call `is_part_of_a_pool` and skip the immediate-spawn path for pool-managed rows (M)
- [ ] **#POOLS.22** Wire creature/gameobject `set_death_state` / destroy callbacks to `update_pool` (depends on entity lifecycle being implemented) (M)
- [ ] **#POOLS.23** Define `QuestPool { pool_id: u32, num_active: u32, members: Vec<Vec<u32>>, active_quests: HashSet<u32> }` (L)
- [ ] **#POOLS.24** Add SQL migration for `pool_quest_save` to `crates/wow-database/migrations/character/` (L)
- [ ] **#POOLS.25** Add prepared-statement enum entries `CHAR_INS_POOL_QUEST_SAVE`, `CHAR_DEL_POOL_QUEST_SAVE` (L)
- [ ] **#POOLS.26** Implement `regenerate_pool(&mut QuestPool)` — partial-shuffle algorithm from `RegeneratePool` (M)
- [ ] **#POOLS.27** Implement `save_pool_to_db(QuestPool, transaction)` — DELETE then per-questId INSERT (L)
- [ ] **#POOLS.28** Implement `QuestPoolMgr::load_from_db` — the heavy validator: load `quest_pool_members JOIN quest_pool_template`, slot into daily/weekly/monthly bucket per `quest.is_daily/weekly/monthly`, load `pool_quest_save`, reconcile saved active quests against pool membership, regenerate any incoherent pool, persist (XL — split: template+members loader, saved-state loader, validator, regenerator)
- [ ] **#POOLS.29** Implement `QuestPoolMgr::change_daily_quests` / `change_weekly_quests` / `change_monthly_quests` (each calls `regenerate(_dailyPools)` etc., transactionally) (M)
- [ ] **#POOLS.30** Implement `QuestPoolMgr::find_quest_pool` / `is_quest_pooled` / `is_quest_active` (L)
- [ ] **#POOLS.31** Wire `QuestPoolMgr::change_*` into the daily/weekly/monthly reset scheduler in the world-tick crate (M)
- [ ] **#POOLS.32** Wire `QuestPoolMgr::is_quest_active` into `Player::can_take_quest` (the quest module already has a stub) (L)
- [ ] **#POOLS.33** Wire `game_event_pool` semantics: when `GameEventMgr::start_event(eventId)` fires, `spawn_pool` for each (pool_entry) in that event; on `stop_event`, `despawn_pool` (M, depends on GameEventMgr existing)
- [ ] **#POOLS.34** Documentation cross-link: `pools.md` ↔ `maps.md` (`SpawnedPoolData` ownership), `quests.md` (QuestPool integration), `events.md` (when written; `game_event_pool` join) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#POOLS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 1507 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | `cargo test -p wow-data && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#POOLS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 1507 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#POOLS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 1507 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#POOLS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 1507 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `pool_template` row with `max_limit = 3` and 5 creature members of equal chance — `spawn_pool` selects exactly 3 distinct guids; `spawned_pools[pool_id] == 3`.
- [ ] Test: explicit-chance algorithm — pool with three explicitly-chanced members (50/30/20) and zero equal-chanced — long-run frequencies converge to the configured chances.
- [ ] Test: explicit + equal mixed — explicit total 60, two equal members. Roll < 60 → explicit-bucket pick; roll ≥ 60 → equal-bucket uniform pick.
- [ ] Test: `update_pool` with `trigger_from = G` (the just-died guid) avoids reselecting G when other choices exist; falls back to G if it is the only candidate.
- [ ] Test: pool-of-pool — mother pool with two child pools; spawn_pool on mother spawns the configured `max_limit` of children; each child internally spawns its own `max_limit`.
- [ ] Test: circular reference detection — `pool_members` row that creates `A → B → A` loop is rejected with one back-edge removed.
- [ ] Test: cross-map members rejected — `pool_creature` rows for a single pool referencing creatures on different maps log error and skip the late member.
- [ ] Test: chance out of [0,100] is rejected.
- [ ] Test: invalid spawnId (no `creature` table row) is rejected.
- [ ] Test: `auto_spawn_pools_per_map` excludes pools attached to a `game_event_pool` and excludes pools that are children of another pool.
- [ ] Test: `init_pools_for_map(mapId)` produces a `SpawnedPoolData` whose `spawned_creatures` count equals the sum of `max_limit` across pools registered for that map (assuming every pool has ≥ `max_limit` non-empty members).
- [ ] Test: `is_part_of_a_pool::<Creature>(spawn_guid)` returns the parent pool ID for known members; 0 for non-members.
- [ ] Test: `QuestPoolMgr::regenerate_pool(p)` selects exactly `p.num_active` `members` groups; flattens them into `active_quests` (so a 2-member group contributes 2 quest IDs); previous `active_quests` are wiped first.
- [ ] Test: `QuestPoolMgr::load_from_db` honours `pool_quest_save` when it is fully consistent (no regeneration needed); fully regenerates and re-persists when saved active count ≠ `num_active`.
- [ ] Test: `is_quest_active` returns true for non-pooled quests; honours `active_quests` set for pooled quests.
- [ ] Test: `change_daily_quests` writes new rows to `pool_quest_save` and old rows are gone (atomically per pool).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 1507 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h` | `crates/wow-data/` (load `pool_template`, `pool_members`, `quest_pool_template`, `quest_pool_members`, `pool_quest_save`), `crates/wow-world/src/pools/` (the in-memory `PoolMgr` + per-`Map` `SpawnedPoolData`), `crates/wow-database/` (CHAR_INS/DEL_POOL_QUEST_SAVE prepared statements). No dedicated `wow-pools` crate yet. \| ❌ not started — there is no `PoolMgr`, no `QuestPoolMgr`, no pool tables loader, no `SpawnedPoolData` per Map, no `PoolGroup<T>` template specializations. Every DB-defined creature/gameobject in `pool_members` is currently spawned unconditionally (or, more accurately, the spawn pipeline ignores pool membership entirely so all members spawn or none do, depending on how the spawn loader treats the rows). |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#POOLS.DIV.001` | `crates/wow-world/src/pools` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1507 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#POOLS.DIV.002` | `crates/wow-pools` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1507 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/QuestPools.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Pools/PoolMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- The `pool_template.entry` is the pool ID, not a creature/gameobject entry — easy mistake when reading SQL by hand.
- `MaxLimit` is the number of *currently alive* members the pool keeps, not the bag size. If the pool has 12 members and `MaxLimit = 1`, exactly one is up at any time.
- `MapId = -1` in `PoolTemplateData` means "not yet determined". After load, every non-empty pool MUST have a non-`-1` mapId or the C++ ASSERTs. In Rust, return a hard error and refuse to start the world.
- The chance algorithm: if `chance > 0` the entry goes into `ExplicitlyChanced`; if `chance == 0` (or written as 0 in DB), it goes into `EqualChanced`. Storing all zeros makes the pool a uniform-random bag; mixing explicit and equal is fine. Sum of explicit chances must be ≤ 100 (the missing chance falls through to equal).
- Pool-of-pool serializes via `pool_members WHERE type = 2`. The `spawnId` column is then the *child pool ID* (not a creature/gameobject GUID), and `poolSpawnId` is the *mother pool ID*. The same `pool_members` table thus has 3 distinct conceptual schemas keyed by `type`.
- The `Pool` empty class in C++ exists purely as a type tag for template specialization. In Rust use either generics-with-sealed-trait or a small `enum PoolMemberKind { Creature, GameObject, Pool }`.
- Spawned-pool data lives **per-Map**, not globally. This is required for instances/scenarios where two parallel instances of the same map roll independent picks. The Map ownership is critical.
- `UpdatePool::<Pool>(p, child_id)` is invoked when a *pool-of-pool* child slot opens up. The same code path is reused for "creature in pool just died" because both end up calling `SpawnPool` on the appropriate parent.
- When a pool is part of a `game_event_pool` join, it does NOT auto-spawn at world start. It only spawns when the event activates; same for despawn on event end. The `LEFT JOIN game_event_pool ... WHERE game_event_pool.pool_entry IS NULL` filter is the one that excludes event pools from auto-spawn.
- `PoolMgr::CheckPool` is run *during* load: a pool that fails (explicit sum > 100 with no equal-chance fallback) is logged and excluded from auto-spawn but kept in the maps for `IsPartOfAPool` queries — DO NOT delete it outright, child pools may reference it.
- QuestPools' `members` is a `vector<vector<u32>>` because real daily-quest schemas have *quest groups* — e.g. "any one of (3 fishing dailies in Dalaran)" counts as a single member. When that group is picked, ALL three quests become active simultaneously. This is why `numActive` counts groups, not quests, but `activeQuests` flattens to questIDs.
- The `pool_quest_save` reconciliation in `QuestPoolMgr::LoadFromDB` is one of the trickiest pieces in the codebase. Read the C++ carefully when porting — the algorithm has specific rules about partial saves (a saved active quest from a group implies the entire group is active; mismatches log warnings; an active count != `numActive` triggers a full regenerate). Match the warning messages so admins can correlate logs.
- `RegeneratePool` uses `urand(i, n)` which is `rand` % `(n - i + 1) + i` — partial Fisher-Yates over the front `numActive` elements. Use the project's `wow-utils` RNG to match.
- The `IsQuestActive` shortcut "not pooled → true" is what makes non-pooled dailies/weeklies coexist with pooled ones cleanly; do not "fix" it to return `false` for unknown quests.
- `ChangeDailyQuests` etc. are called by **the world reset scheduler**, not by any per-pool timer. Daily reset is at server-configurable hour (3 AM by default), weekly is on a configured weekday, monthly is on a configured day-of-month. Wire those to the Rust scheduler crate when it's ready.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `struct PoolTemplateData` | `pub struct PoolTemplateData { pub max_limit: u32, pub map_id: Option<u32> }` | `Option` is more honest than the `-1` sentinel |
| `struct PoolObject` | `pub struct PoolObject { pub guid: u64, pub chance: f32 }` | — |
| `class Pool` (empty marker) | `pub enum PoolKind { Creature, GameObject, Pool }` (or use sealed trait + ZST types) | Choose enum for simplicity; generics for performance — algorithm is small enough that monomorphization gain is marginal |
| `template<class T> class PoolGroup` | `pub struct PoolGroup { kind: PoolKind, explicitly_chanced: Vec<PoolObject>, equal_chanced: Vec<PoolObject>, pool_id: u32 }` (with `kind` as runtime tag) | Trade compile-time type-safety for one runtime branch |
| `class SpawnedPoolData` | `pub struct SpawnedPoolData { spawned_creatures: HashSet<u64>, spawned_gameobjects: HashSet<u64>, spawned_pools: HashMap<u64, u32>, owner_map_id: u32 }` | Owner is the Map |
| `class PoolMgr` (singleton) | `pub struct PoolMgr { … }` + `static POOL_MGR: OnceCell<PoolMgr>` | — |
| `unordered_map<u32, PoolGroup<T>>` | `HashMap<u32, PoolGroup>` (one per kind, keyed by pool_id) | — |
| `map<u64, u32> SearchMap` | `BTreeMap<u64, u32>` (sorted iteration is debug-friendly; `HashMap` works too) | — |
| `unordered_map<u32, vector<u32>> mAutoSpawnPoolsPerMap` | `HashMap<u32, Vec<u32>>` | — |
| `unique_ptr<SpawnedPoolData>` (member of `Map`) | `Option<SpawnedPoolData>` field on `Map` | Initialized in Map ctor |
| `Trinity::Containers::SelectRandomContainerElementIfExists` | `wow_utils::random::pick(slice)` (or `rand::seq::IteratorRandom`) | Use the project's seeded RNG so determinism in tests is possible |
| `urand(i, n)` | `wow_utils::random::urand(i, n)` | Inclusive both ends, matches C++ |
| `class QuestPoolMgr` (singleton) | `pub struct QuestPoolMgr { daily: Vec<QuestPool>, weekly: Vec<QuestPool>, monthly: Vec<QuestPool>, lookup: HashMap<u32, *const QuestPool> }` + `OnceCell` | The lookup pointer-into-vector is a borrow problem in Rust — use `(period: enum { Daily, Weekly, Monthly }, index: usize)` as the value instead |
| `struct QuestPool` | `pub struct QuestPool { pub pool_id: u32, pub num_active: u32, pub members: Vec<Vec<u32>>, pub active_quests: HashSet<u32> }` | — |
| `static void RegeneratePool(QuestPool&)` | `fn regenerate_pool(pool: &mut QuestPool, rng: &mut impl Rng)` | Inject RNG for testability |
| `CharacterDatabasePreparedStatement* CHAR_INS_POOL_QUEST_SAVE` | `crate::statements::CharStatement::InsPoolQuestSave` | sqlx prepared statement |
| `Quest::IsDaily/IsWeekly/IsMonthly` | `Quest::is_daily()` / `is_weekly()` / `is_monthly()` (from `wow-data/quests`) | Already on the radar via the Quests migration doc |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines `PoolMgr` Rust impl | ✅ | `grep -rn "PoolMgr\|QuestPoolMgr\|SpawnedPoolData\|PoolGroup" crates/ → 0` |
| 0 references to pool tables | ✅ | `grep -rn "pool_template\|pool_members\|pool_quest_save\|quest_pool_template" crates/ → 0` |
| No `crates/wow-world/src/pools/` directory | ✅ | not present in `ls crates/wow-world/src/` |
| No DB loader / prepared statements | ✅ | grep `pool` in `crates/wow-database/src` → 0; grep in `crates/wow-data/src` → 0 |
| No opcodes (correct — pools are server-internal) | ✅ | n/a, pools never go on the wire |

**Silent-hang risk:** none (no client-visible packets). Behavioural impact: as doc states, every row in `pool_members` is currently spawned unconditionally OR ignored entirely depending on the spawn loader's table list. Need to verify which by checking `wow-database` creature-spawn loader, but that's a separate audit.

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.
