# Migration: Instances (Dungeons / Raids — Lock + Script)

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Instances/`
> **Rust target crate(s):** `crates/wow-instances/`
> **Layer:** L7
> **Status:** 🟡 foundation in progress
> **Audited vs C++:** ✅ audited 2026-05-10 (`InstanceLockMgr` core contrasted; broad module still pending)
> **Last updated:** 2026-06-16

---

## 1. Purpose

The Instances module manages **persistent dungeon and raid state** for TrinityCore: per-character / per-group lockouts, per-instance encounter progress (boss states, doors, minions), difficulty (Normal / Heroic, 10/25 raid sizes), instance ID assignment, and weekly/daily reset timers. It is the bridge between the transient `InstanceMap` (created on demand from `MapManager`) and the durable lock data that survives server restarts and cross-character lookups.

Two distinct subsystems live here:
- **`InstanceLockMgr`** — DB-backed persistence of *who is locked to what*. Tracks `(playerGuid, mapId, lockId)` → `InstanceLock` (with `instanceId`, `expiryTime`, `extended`, `data`, `completedEncountersMask`, `entranceWorldSafeLocId`). Handles ID-based vs lock-id-based binding, shared-state for raid-id locks, lock extension, and reset.
- **`InstanceScript`** — Per-map runtime state machine (boss states `NOT_STARTED|IN_PROGRESS|FAIL|DONE|SPECIAL`, door behaviors, minion linking, dungeon encounter ↔ DBC `DungeonEncounterEntry`, persistent script values, achievement criteria hook). Subclassed by every dungeon/raid script (Naxxramas, Ulduar, ICC, etc.).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Instances/InstanceLockMgr.cpp` | 599 | `prefix` |
| `game/Instances/InstanceLockMgr.h` | 320 | `prefix` |
| `game/Instances/InstanceScript.cpp` | 971 | `prefix` |
| `game/Instances/InstanceScript.h` | 461 | `prefix` |
| `game/Instances/InstanceScriptData.cpp` | 270 | `prefix` |
| `game/Instances/InstanceScriptData.h` | 86 | `prefix` |
| `game/Instances/enuminfo_InstanceScript.cpp` | 76 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Instances/InstanceLockMgr.h` | 320 | `InstanceLock`, `SharedInstanceLock`, `InstanceLockMgr`, `MapDb2Entries`, `InstanceLockUpdateEvent` |
| `src/server/game/Instances/InstanceLockMgr.cpp` | 599 | DB load/save, `CanJoinInstanceLock`, `FindActiveInstanceLock`, `UpdateInstanceLockForPlayer`, `ResetInstanceLocksForPlayer`, `GetNextResetTime` |
| `src/server/game/Instances/InstanceScript.h` | 461 | `InstanceScript` base class, `BossInfo`, `DoorData`, `MinionData`, `ObjectData`, `EncounterState`, `EncounterDoorBehavior`, `PersistentInstanceScriptValue<T>` |
| `src/server/game/Instances/InstanceScript.cpp` | 971 | Boss state machine, door/minion bookkeeping, save-data serialization, world-state push, combat-resurrection, `SendEncounterStart/End`, `UpdateLfgEncounterState` |
| `src/server/game/Instances/InstanceScriptData.h` | 86 | Persistent value variant wrappers + serialization helpers |
| `src/server/game/Instances/InstanceScriptData.cpp` | 270 | rapidjson-based load/save of instance script blobs (boss states + persistent values) |
| `src/server/game/Maps/InstanceMap.cpp` (related) | n/a | `InstanceMap::CreateInstanceData`, glue to `InstanceLockMgr` (covered partially in `maps.md`) |
| `src/server/game/Maps/MapManager.cpp` (related) | n/a | Instance ID allocation (`GenerateInstanceId`), reset scheduling (`InitInstanceIds`, `DoForAllMapsWithMapId`) |

NOTE: `InstanceSaveMgr` no longer exists as a separate class in this WoLK 3.4.3 fork — its responsibilities were absorbed into `InstanceLockMgr` (plus per-instance state in `InstanceMap` / `InstanceScript`). The reset cron is in `MapManager::Update` + `InstanceLockMgr::GetNextResetTime`.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `InstanceLockData` | struct | `Data` (script blob string), `CompletedEncountersMask` (u32), `EntranceWorldSafeLocId` (u32) |
| `InstanceLock` | class | Per-player view of a lock: mapId, difficultyId, instanceId, expiryTime, extended, `_isInUse`, `_isNew` |
| `SharedInstanceLockData` | struct (extends `InstanceLockData`) | Authoritative shared state for ID-based locks (`InstanceId`) |
| `SharedInstanceLock` | class (extends `InstanceLock`) | Holds `shared_ptr<SharedInstanceLockData>`; UI shows player view, real instance reads shared |
| `MapDb2Entries` | struct | Pair of `MapEntry*` + `MapDifficultyEntry*`; provides `GetKey()` → `(mapId, lockId)` and `IsInstanceIdBound()` |
| `InstanceLockKey` | typedef | `pair<u32, u32>` = `(MapDifficultyEntry::MapID, MapDifficultyEntry::LockID)` |
| `InstanceLockUpdateEvent` | struct | New data to be merged into a lock (`InstanceId`, `NewData`, `InstanceCompletedEncountersMask`, `CompletedEncounter*`, `EntranceWorldSafeLocId`) |
| `InstanceLocksStatistics` | struct | `InstanceCount`, `PlayerCount` (for GM commands) |
| `InstanceLockMgr` | singleton | Owns all loaded locks; `Load()`, `CanJoinInstanceLock()`, `FindActiveInstanceLock()`, `UpdateInstanceLockForPlayer()`, `ResetInstanceLocksForPlayer()` |
| `InstanceScript` | class (extends `ZoneScript`) | Per-map script base; owns `bosses`, `doors`, `minions`, `_objectGuids`, persistent values, entrance, combat-res |
| `BossInfo` | struct | `state`, `door[Behavior::Max]` (per-behavior guid sets), `minion`, `boundary`, `DungeonEncounters[4]` |
| `DoorInfo` / `MinionInfo` | struct | Back-reference from GO/creature entry → `BossInfo*` + behavior |
| `DoorData` / `MinionData` / `ObjectData` / `DungeonEncounterData` / `BossBoundaryEntry` | struct | Static load data (compile-time arrays from script files) |
| `EncounterState` | enum | `NOT_STARTED=0, IN_PROGRESS=1, FAIL=2, DONE=3, SPECIAL=4, TO_BE_DECIDED=5` |
| `EncounterDoorBehavior` | enum class | `OpenWhenNotInProgress=0, OpenWhenDone=1, OpenWhenInProgress=2, OpenWhenNotDone=3` |
| `EncounterFrameType` | enum | Wire-frame opcode payloads: ENGAGE, DISENGAGE, UPDATE_PRIORITY, ADD_TIMER, ENABLE/UPDATE/DISABLE_OBJECTIVE, COMBAT_RES_LIMIT |
| `UpdateBossStateSaveDataEvent` / `UpdateAdditionalSaveDataEvent` | struct | Inputs to `InstanceScriptData` JSON serializer |
| `PersistentInstanceScriptValueBase` / `PersistentInstanceScriptValue<T>` | template class | Type-erased `variant<int64,double>` wrapper with auto-notify-on-change |
| `INSTANCE_ID_HIGH_MASK` / `_LFG_MASK` / `_NORMAL_MASK` | constexpr | `0x1F440000`, `0x00000001`, `0x00010000` — bitfields encoded into instance IDs |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `InstanceLockMgr::Load()` | Loads `instance` + `character_instance_lock` rows; reconstructs `_temporaryInstanceLocksByPlayer`, `_instanceLocksByPlayer`, `_instanceLockDataById` | `CharacterDatabase`, `sDB2Manager` |
| `InstanceLockMgr::CanJoinInstanceLock(playerGuid, entries, lock)` | Returns `TransferAbortReason` (NONE / ALREADY_COMPLETED_ENCOUNTER / DIFFICULTY / LOCKED_TO_DIFFERENT_INSTANCE / etc.) | `MapDb2Entries::IsInstanceIdBound`, `InstanceLock::IsExpired` |
| `InstanceLockMgr::FindActiveInstanceLock(playerGuid, entries)` | Resolves owner (player guid or group-leader guid) → active lock; honors `extended` for expired entries | private `FindInstanceLock` |
| `InstanceLockMgr::CreateInstanceLockForNewInstance(...)` | Allocates new (ephemeral) lock pre-encounter; goes into `_temporaryInstanceLocksByPlayer` until first boss kill | `GetNextResetTime` |
| `InstanceLockMgr::UpdateInstanceLockForPlayer(trans, ...)` | Promotes temporary → permanent on encounter; writes `character_instance_lock` row; updates encounters mask | `CharacterDatabase`, `UpdateSharedInstanceLock` |
| `InstanceLockMgr::UpdateSharedInstanceLock(trans, event)` | Writes `instance` row for ID-based locks; mutates `SharedInstanceLockData` | `CharacterDatabase` |
| `InstanceLockMgr::UpdateInstanceLockExtensionForPlayer(...)` | Toggles `extended` flag; returns `(oldExpiry, newExpiry)` (next-week boundary) | `GetNextResetTime` |
| `InstanceLockMgr::ResetInstanceLocksForPlayer(...)` | Removes locks matching filter unless `IsInUse()` (active map exists) | iterates `_instanceLocksByPlayer` |
| `InstanceLockMgr::OnSharedInstanceLockDataDelete(instanceId)` | Cleans `instance` row when last shared ref drops | `CharacterDatabase` |
| `InstanceLockMgr::GetNextResetTime(entries)` | Computes next reset `time_point` from `MapDifficulty.ResetInterval` aligned to weekly/daily reset hour from config | system clock |
| `InstanceScript::Create()` | Fresh-state init (no save data) | `SetBossState` |
| `InstanceScript::Load(data)` | rapidjson parse of header + `bosses[]` states + persistent values | `InstanceScriptData::Load` |
| `InstanceScript::GetSaveData()` | rapidjson serialization | `InstanceScriptData::Save` |
| `InstanceScript::SetBossState(id, state)` | State transition + door/minion/spawn-group updates + LFG progress + `InstanceMap::UpdateInstanceLock` | `UpdateDoorState`, `UpdateMinionState`, `UpdateSpawnGroups`, `SendBossKillCredit`, `UpdateLfgEncounterState` |
| `InstanceScript::IsEncounterInProgress()` | Any boss `IN_PROGRESS`? Used by `Map::CannotEnter` to block ports during pulls | linear scan `bosses` |
| `InstanceScript::OnCreatureCreate/Remove`, `OnGameObjectCreate/Remove` | Auto-bind GO/creature to `BossInfo` via entry → `DoorInfo` / `MinionInfo` / `_creatureInfo` / `_gameObjectInfo` | `AddObject`, `AddDoor`, `AddMinion` |
| `InstanceScript::SendEncounterStart/End` | Sends `SMSG_INSTANCE_ENCOUNTER_START / _END` to all players in map | `Map::DoForAllPlayers` |
| `InstanceScript::SendEncounterUnit(type, unit, prio)` | Sends `SMSG_INSTANCE_ENCOUNTER_FRAME` (engage / disengage / update / objective) | `BroadcastWorker` |
| `InstanceScript::DoUpdateWorldState(id, value)` | World-state push to all players in instance | `Map::DoForAllPlayers` |
| `InstanceScript::InitializeCombatResurrections / Use / Reset` | 25-man raid combat-res charge tracker | timer in `Update` |
| `InstanceScript::UpdateLfgEncounterState(bossInfo)` | Sends LFG dungeon-progress update if all encounters of completion mask done | `LFGMgr` |
| `InstanceScript::TriggerGameEvent(...)` | Override of `ZoneScript`; dispatches event to all players in the map | `GameEventMgr` |

---

## 5. Module dependencies

**Depends on:**
- `Maps` — `InstanceMap` is the runtime parent; `InstanceScript::instance` is an `InstanceMap*`. `MapManager::GenerateInstanceId` allocates the persistent ID.
- `DataStores (DB2)` — `MapEntry`, `MapDifficultyEntry`, `DungeonEncounterEntry`, `WorldSafeLocsEntry` (entrance), `LFGDungeonEntry`.
- `Database (CharacterDatabase)` — tables: `instance`, `character_instance_lock`, `respawn`, `account_instance_times` (account-wide raid IDs).
- `Groups` — `Group::GetLeaderGUID()` is the lock owner for grouped instances; group reset commands (`Group::ResetInstances`).
- `LFG (LFGMgr)` — `UpdateLfgEncounterState` + dungeon-finished bookkeeping.
- `World/Time` — reset hour from `worldserver.conf`, `GameTime::GetGameTime()`.
- `Phasing` — `InstanceScript::UpdatePhasing()` re-evaluates phase conditions on boss state change.
- `Achievement / Criteria` — `DoUpdateCriteria(CriteriaType, m1, m2, unit)` and `CheckAchievementCriteriaMeet`.
- `ZoneScript` (parent class) — generic per-zone hooks (`OnCreatureCreate`, `ProcessEvent`, `GetData`/`SetData`, `GetGuidData`).
- `rapidjson` — for `InstanceScriptData` load/save.

**Depended on by:**
- `Maps` — `InstanceMap` calls `InstanceScript::Update()` every frame, `IsEncounterInProgress` for block-on-enter, `OnPlayerEnter/Leave`.
- `Player` — `Player::SendRaidInfo()`, `Player::ResetInstances()`, `WorldSession::HandleResetInstancesOpcode` use `InstanceLockMgr::GetInstanceLocksForPlayer`.
- `Group` — group lock binding; raid-leader reset commands.
- All ~120 dungeon/raid script files in `src/server/scripts/{Northrend,Outland,EasternKingdoms,...}` subclass `InstanceScript`.
- `LFGMgr` — checks instance encounter-mask completion to award LFG rewards.
- GM commands `.instance unbind`, `.instance reset`, `.instance stats`.

---

## 6. SQL / DB queries (if any)

### CharacterDatabase tables

| Table | Schema highlights | Purpose |
|---|---|---|
| `instance` | `instanceId PK`, `data TEXT`, `completedEncountersMask INT`, `entranceWorldSafeLocId INT` | Authoritative state for ID-based (raid) locks |
| `character_instance_lock` | `(guid, mapId, lockId) PK`, `instanceId`, `difficulty`, `data`, `completedEncountersMask`, `entranceWorldSafeLocId`, `expiryTime BIGINT`, `extended TINYINT` | Per-character lock view |
| `account_instance_times` | per-account hourly limit on instance enter (3.x: max 5 per hour) | `Player::CheckInstanceCount` |
| `respawn` | `(type, spawnId, respawnTime, mapId, instanceId)` | Per-instance creature/GO respawn rows (cleared on reset) |

### Prepared statements (CharacterDatabase, names from `CharacterDatabase.cpp`)

| Statement | Purpose |
|---|---|
| `CHAR_SEL_INSTANCE` | Load all `instance` rows on startup |
| `CHAR_SEL_CHARACTER_INSTANCE_LOCKS` | Load all `character_instance_lock` rows |
| `CHAR_REP_INSTANCE` | REPLACE INTO `instance` |
| `CHAR_DEL_INSTANCE_BY_INSTANCE` | DELETE FROM `instance` WHERE instanceId = ? |
| `CHAR_REP_CHARACTER_INSTANCE_LOCK` | REPLACE INTO `character_instance_lock` |
| `CHAR_DEL_CHARACTER_INSTANCE_LOCK_BY_GUID` / `_BY_GUID_MAP_DIFFICULTY` | Lock removal (player delete, GM unbind) |
| `CHAR_UPD_CHARACTER_INSTANCE_LOCK_EXTEND` | Toggle `extended` flag |
| `CHAR_SEL_RESPAWNS` / `CHAR_REP_RESPAWN` / `CHAR_DEL_RESPAWN` / `CHAR_DEL_ALL_RESPAWNS` | Unified per-instance respawn rows |

### DBC/DB2 stores read

| Store | What it loads | Read by |
|---|---|---|
| `MapStorage` | Map.db2 (`MapEntry`) | `MapDb2Entries` ctor, `IsInstanceIdBound` |
| `MapDifficultyStorage` | MapDifficulty.db2 (`LockID`, `ResetInterval`, `MaxPlayers`) | `GetKey()`, `GetNextResetTime` |
| `DungeonEncounterStorage` | DungeonEncounter.db2 | `LoadDungeonEncounterData`, `GetBossDungeonEncounter` |
| `LFGDungeonsStorage` | LFGDungeons.db2 | `UpdateLfgEncounterState` |
| `WorldSafeLocsStorage` | WorldSafeLocs.db2 | `entranceWorldSafeLocId` resolution |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_RESET_INSTANCES` | client → server | `WorldSession::HandleResetInstancesOpcode` → `Player::ResetInstances` / `Group::ResetInstances` |
| `CMSG_SET_DUNGEON_DIFFICULTY` | client → server | Player ↔ `WorldSession::HandleSetDungeonDifficultyOpcode` (group leader) |
| `CMSG_SET_RAID_DIFFICULTY` | client → server | `HandleSetRaidDifficultyOpcode` |
| `CMSG_INSTANCE_LOCK_RESPONSE` | client → server | Confirm convert-to-permanent on first boss kill |
| `CMSG_REQUEST_RAID_INFO` | client → server | `Player::SendRaidInfo` |
| `CMSG_INSTANCE_INFO` | client → server | (cataclysm+) similar for dungeon list |
| `CMSG_REQUEST_INSTANCE_INFO` | client → server | Build dungeon list (post 4.x) |
| `SMSG_INSTANCE_RESET` | server → client | After successful reset (per-player or per-group) |
| `SMSG_INSTANCE_RESET_FAILED` | server → client | When `IsInUse()` blocks reset |
| `SMSG_INSTANCE_INFO` | server → client | Periodic instance lock list to client (`Player::SendRaidInfo`) |
| `SMSG_RAID_INSTANCE_MESSAGE` | server → client | Welcome / "instance expires in X" / "you are saved" |
| `SMSG_PENDING_RAID_LOCK` | server → client | Convert-to-permanent confirm prompt on entry |
| `SMSG_INSTANCE_ENCOUNTER_ENGAGE_UNIT` / `_DISENGAGE_UNIT` | server → client | `SendEncounterUnit` |
| `SMSG_INSTANCE_ENCOUNTER_START` / `_END` | server → client | `SendEncounterStart`, `SendEncounterEnd` |
| `SMSG_INSTANCE_ENCOUNTER_CHANGE_PRIORITY` | server → client | nameplate priority |
| `SMSG_INSTANCE_ENCOUNTER_TIMER_START` | server → client | Boss soft-enrage timer |
| `SMSG_INSTANCE_ENCOUNTER_OBJECTIVE_START` / `_UPDATE` / `_COMPLETE` | server → client | Obj progress |
| `SMSG_INSTANCE_ENCOUNTER_IN_COMBAT_RESURRECTION` / `_GAIN_COMBAT_RESURRECTION_CHARGE` | server → client | Combat-res tracking |
| `SMSG_BOSS_KILL_CREDIT` (alias `SMSG_INSTANCE_ENCOUNTER_GAIN_KILL_CREDIT` in some versions) | server → client | `SendBossKillCredit(encounterId)` |
| `SMSG_UPDATE_INSTANCE_OWNERSHIP` | server → client | Notify on instance owner change |
| `SMSG_TRANSFER_ABORTED` | server → client | When `CanJoinInstanceLock` returns non-NONE |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-instances` | `crate_dir` | 1 | 1113 | `foundation_active` | Encounter metadata foundation plus in-memory `InstanceLockMgr` core contrasted against C++ |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-instances/src/lib.rs` — foundation started in `#NEXT.R8.ENTITIES.303/#NEXT.R8.ENTITIES.304`: the crate exists and ports the pure C++ encounter metadata path for `MAX_DUNGEON_ENCOUNTERS_PER_BOSS`, `EncounterState`, `DungeonEncounterData`, `BossInfo::GetDungeonEncounterForDifficulty`, `InstanceScript::LoadDungeonEncounterData`, `InstanceScript::GetBossDungeonEncounter(uint32)` and the `BossAI::GetBossId()` branch behind `InstanceScript::GetBossDungeonEncounter(Creature const*)`. `#NEXT.R8.ENTITIES.305` preserves optional represented `BossAI::_bossId` on `wow-ai::CreatureAI`.
- `#NEXT.R8.INSTANCES.001` ports the C++ `InstanceLockMgr` in-memory core: `InstanceLockData`, `InstanceLock`, `SharedInstanceLockData`, `MapDb2Entries::GetKey`, `IsInstanceIdBound`, temporary lock creation, active-lock lookup with extended expired locks, `CanJoinInstanceLock`, temporary-to-permanent promotion, shared-data update, lock extension, reset in-use guard, statistics, `GetNextResetTime`, and C++ instance-id mask constants.
- `#NEXT.R8.INSTANCES.002` adds the C++ character DB statement set for `instance`, `character_instance_lock`, and `account_instance_times`, plus pure Rust load reconstruction from DB row shapes, async DB load glue, prepared-statement builders for the same delete/insert/update operations used by C++ `InstanceLockMgr`, and weak-ref cleanup for unreferenced shared instance data.
- `#NEXT.RUNTIME.L3.031j61` moves the DB2-derived `MapDb2Entries` builder into `wow-instances` as `MapDb2Entries::from_stores_like_cpp`, matching the fields consumed by C++ `MapManager::CreateMap` / `InstanceLockMgr` (`mapId`, `difficultyId`, `lockId`, reset interval, flex-lock flag, encounter-lock flag). `world-server` now delegates to this shared helper instead of owning a private duplicate, which lets the next session-side `CreateMap` work reuse the same logic.
- `#NEXT.RUNTIME.L3.031j62` ports the C++ `DB2Manager::GetDownscaledMapDifficultyData(mapId, difficulty)` foundation: `DifficultyEntry` now carries `FallbackDifficultyID`, `MapDifficultyStore::downscaled_for_map_like_cpp` follows fallback/default semantics and returns the effective difficulty id, and `MapDb2Entries::from_downscaled_stores_like_cpp` builds dungeon `CreateMap` lock inputs with that effective difficulty. Remaining gap: this is still not wired to live dungeon `CreateMap` / active temporary `InstanceLockMgr` side effects.
- `#NEXT.RUNTIME.L3.031j63` adds the `WorldSession::create_map_difficulty_context_like_cpp` bridge that converts the downscaled `MapDb2Entries` into `wow_map::CreateMapDifficultyContext` for the represented dungeon `CreateMap` decision engine. Remaining gap: the live session ensure-map path still skips dungeon maps and active/temporary `InstanceLockMgr` context is not yet supplied.
- `#NEXT.RUNTIME.L3.031j64` adds the `WorldSession::create_map_active_instance_lock_context_like_cpp` bridge for C++ `MapManager::CreateMap` active-lock lookup: group recent-instance owner or solo player owner, downscaled `MapDb2Entries`, represented `InstanceLockMgr::FindActiveInstanceLock`, and conversion into `wow_map::CreateMapInstanceLockContext`. The Rust token represents the C++ `InstanceLock*` identity boundary for encounter-lock conflicts without keying on mutable `instance_id`.
- `#NEXT.RUNTIME.L3.031j65` applies the represented `CreateInstanceLockForNewInstance` side effect into the shared `InstanceLockMgr` temporary-lock map using the same downscaled `MapDb2Entries` bridge.
- `#NEXT.RUNTIME.L3.031j68` wires `ResetSchedule.{Hour,WeekDay}` from the C++ world config into the live-session dungeon `CreateMap` temporary-lock expiry path. Remaining gap: broader DB-loaded/non-session lock paths, persistence/install/restart, bot, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j69` covers the C++ existing-map encounter-lock conflict branch in the live-session dungeon `CreateMap` path and fixes canonical `MapManager::create_map_entry` to register created dungeon/battleground instance ids as allocated, so regenerated ids cannot reuse an occupied map id.
- `#NEXT.RUNTIME.L3.031j70` strengthens `#NEXT.R8.INSTANCES.003` startup fidelity for DB-loaded locks: Rust now mirrors C++ `InstanceLockMgr::Load()` by recording every `character_instance_lock.instanceId` before lock validation, so `MapManager::RegisterInstanceId` still reserves ids from rows that are later skipped because their shared `instance` data is missing.
- `#NEXT.RUNTIME.L3.031j71` wires represented `CMSG_INSTANCE_LOCK_RESPONSE` accept into the shared `InstanceLockMgr` path: pending binds now carry source map/completed-mask context, accept validates the current canonical `InstanceMap` id like C++ `Player::ConfirmPendingBind`, calls `UpdateInstanceLockForPlayer`, and sends `SMSG_INSTANCE_SAVE_CREATED` for newly created player binds. Remaining gap: full `InstanceMap::i_data` save-data and calendar lockout fanout are still pending.
- `#NEXT.RUNTIME.L3.031j72` adds `SMSG_CALENDAR_RAID_LOCKOUT_ADDED` serialization and sends it after represented pending-bind lock creation, matching C++ `WorldSession::SendCalendarRaidLockoutAdded` for the new-lock branch. Remaining gap: broader reset/remove calendar fanout remains pending.
- `#NEXT.RUNTIME.L3.031j73` adds represented `CMSG_SET_SAVED_INSTANCE_EXTEND` support through the shared unresolved C++ `0xBADD` opcode slot, mutates the active player instance-lock extension flag with `UpdateInstanceLockExtensionForPlayer`, and sends `SMSG_CALENDAR_RAID_LOCKOUT_UPDATED` with the C++ field order. Remaining gap: reset/remove calendar fanout remains pending.
- `#NEXT.RUNTIME.L3.031j74` adds `SMSG_CALENDAR_RAID_LOCKOUT_REMOVED` serialization with the C++ field order. Current C++ audit found `WorldSession::SendCalendarRaidLockoutRemoved` but no direct call-site in this fork, so no Rust runtime fanout is invented in this slice.
- `#NEXT.RUNTIME.L3.031j66` applies the represented `SetInstanceLockInstanceId` side effect by mutating the active permanent-or-temporary lock in `InstanceLockMgr`, matching C++ `InstanceLock::SetInstanceId` for encounter-lock conflict regeneration. Remaining gap: lock persistence / live validation remain pending.
- `#NEXT.R8.INSTANCES.003` adds C++-indexed `Map.db2`/`MapDifficulty.db2` readers, wires them into world-server startup/session resources, invokes `InstanceLockMgr::load_from_database_like_cpp()` with real DB2 `MapDb2Entries` resolution, and registers persisted instance ids with both MapManager paths.
- `#NEXT.R8.INSTANCES.004` adds transaction-aware `UpdateInstanceLockForPlayer` and `UpdateSharedInstanceLock` wrappers that mutate the in-memory lock state and append the same C++ delete/insert statement pairs to a caller-owned `SqlTransaction`.
- `#NEXT.R8.INSTANCES.005` adds transaction-aware `UpdateInstanceLockExtensionForPlayer` and `ResetInstanceLocksForPlayer` wrappers that mutate the in-memory lock state and append the same C++ extension/force-expire update statements to a caller-owned `SqlTransaction`.
- `#NEXT.R8.INSTANCES.006` adds C++ packet builders for `SMSG_INSTANCE_RESET` / `SMSG_INSTANCE_RESET_FAILED`, registers/dispatches `CMSG_RESET_INSTANCES`, preserves the C++ guard against resetting while inside an instanceable map, enforces group-leader-only reset, and connects the handler to `InstanceLockMgr::ResetInstanceLocksForPlayer` plus character DB transaction commit.
- `#NEXT.R8.INSTANCES.007` adds C++ packet/read support for `SMSG_PENDING_RAID_LOCK`, `CMSG_INSTANCE_LOCK_RESPONSE`, and `SMSG_INSTANCE_SAVE_CREATED`, plus represented `Player::SetPendingBind` / `ConfirmPendingBind` / `RepopAtGraveyard` session state and handler dispatch for lock accept/decline.
- `#NEXT.R8.INSTANCES.008` corrects the respawn-storage plan against this C++ fork and adds the real `CHAR_SEL_RESPAWNS`, `CHAR_REP_RESPAWN`, `CHAR_DEL_RESPAWN`, and `CHAR_DEL_ALL_RESPAWNS` SQL plus a Rust `CHAR_DEL_ALL_RESPAWNS` builder for `(mapId, instanceId)` purge.
- `#NEXT.R8.INSTANCES.009` adds the C++ `SMSG_RAID_INSTANCE_MESSAGE` packet builder and `RaidInstanceResetWarningType` values (`WARNING_HOURS` through `EXPIRED`) with byte-order/bit-order coverage.
- `#NEXT.R8.INSTANCES.010` adds C++ packet builders for `SMSG_INSTANCE_ENCOUNTER_ENGAGE_UNIT`, `_DISENGAGE_UNIT`, `_CHANGE_PRIORITY`, `_START`, `_END`, `_IN_COMBAT_RESURRECTION`, `_GAIN_COMBAT_RESURRECTION_CHARGE`, and `SMSG_BOSS_KILL`, including packed-guid and empty-payload coverage.
- `#NEXT.R8.INSTANCES.011` adds the pure C++ `InstanceScript::Create`, `InstanceScriptDataReader::Load`, `InstanceScriptDataWriter::FillData/GetString`, and numeric `PersistentInstanceScriptValue` save/load core: header check, boss-state array, transient-state normalization, strict error cases, and compact C++-ordered JSON output.
- `#NEXT.R8.INSTANCES.012` adds pure C++ `InstanceScript` encounter query helpers: `IsEncounterInProgress`, `IsEncounterCompleted`, `IsEncounterCompletedInMaskByBossId`, and `GetEncounterCount`-equivalent boss count behavior.
- `#NEXT.R8.INSTANCES.013` adds pure C++ `InstanceScript::SetBossState` transition planning: `TO_BE_DECIDED` load initialization, unchanged/no-regression guards, alive world-boss-minion DONE guard, combat-res/start/end/player-notify flags, encounter-id derived update-lock/criteria/boss-kill/LFG flags, and door/minion/spawn-group follow-up flag.
- `#NEXT.R8.INSTANCES.014` adds pure C++ combat-resurrection tracker behavior: player-count interval calculation, initialize/reset, timer update, charge gain event, and use-charge event matching the already ported combat-resurrection packets.
- `#NEXT.R8.INSTANCES.015` adds base C++ entrance-location behavior: fixed entrance id, temporary entrance override, `SetEntranceLocation` clearing temporary entrance, and default encounter-lock completed-mask resolver returning no override.
- `#NEXT.R8.INSTANCES.016` adds C++ `InstanceScript` area-trigger completion tracking: mark, reset, and query over an idempotent `unordered_set`-equivalent set.

**What's implemented:**
- `crates/wow-world/src/map_manager.rs` (per the active WIP commits) provides a `MapManager` global stub with placeholder for `GenerateInstanceId` (must verify), but no lock store and no script dispatch. (See WIP commit `f83c48d82`.)
- World/character DB pool layer can run queries; `wow-database` now registers the C++ `instance`, `character_instance_lock`, and `account_instance_times` statements, and world-server startup now invokes the DB2-backed `InstanceLockMgr` load path.

**What's missing vs C++:**
- `InstanceLockMgr` real world/map call-sites that invoke the transaction-aware update helpers.
- `CMSG_RESET_INSTANCES` now reaches `InstanceLockMgr`; represented `Player::m_recentInstances`
  and `Group::m_recentInstances` / `m_ownedInstancesMgr` state plus reset-result erase rules are
  present. `WorldSession` now also builds the canonical `CreateMapPlayerContext` from represented
  player/group state, including solo/group recent instance ids and the C++ `GetDifficultyID(MapEntry*)`
  choice between dungeon, modern raid, and legacy raid difficulty. The session-side
  `CreateMapDecision` side-effect seam now applies represented `SetPlayerRecentInstance` /
  `SetGroupRecentInstance` updates and explicitly reports still-pending lock/BG side effects.
  Remaining gap: the exact live C++ runtime still needs real `InstanceMap` references,
  `InstanceMap::Reset(method)` calls, full dungeon/BG `CreateMap` execution with `InstanceLockMgr`
  creation/update side effects, and reset success/failure packet integration from those live map
  results.
- `SMSG_PENDING_RAID_LOCK` / `CMSG_INSTANCE_LOCK_RESPONSE` protocol and represented pending-bind state are present; creation of the prompt from `InstanceMap::AddPlayerToMap` and real `ConfirmPendingBind` lock creation remain pending with `InstanceMap`.
- Everything below `InstanceScript` — boss-state machine, door/minion linking, encounter packets, persistent values, JSON save blob.
- Reset cron caller in `MapManager::Update`; pure `GetNextResetTime` is now ported/tested in `wow-instances`.
- All wire packets (Section 7).
- Group-leader-as-lock-owner rule.
- Account-wide instance-enter rate limit (`account_instance_times`) DB
  load/save integration remains pending; represented session memory checks are
  already used by live `CreateMap` and teleport preflight paths.
- Per-instance respawn purge on reset (`respawn` rows tagged with `mapId` + `instanceId`).
- LFG progress hook.
- Combat-res tracker.
- All ~120 dungeon/raid scripts (each a separate Trait/struct in Rust).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `MapManager::GenerateInstanceId` was audited against C++: this fork does not OR the `INSTANCE_ID_*_MASK` constants in the allocator; it reserves 0, returns the lowest free sequential id, registers loaded ids, and reuses freed ids. Rust now ports that pure allocator behavior in `wow-world`.
- `#NEXT.RUNTIME.L3.031j75` wires the first live-session `CanJoinInstanceLock`
  call-site for existing canonical dungeon maps: the group/player `CreateMap`
  path now preserves the target map's active lock owner context and rejects a
  player permanently saved to an incompatible instance with `SMSG_TRANSFER_ABORTED`.
- `#NEXT.RUNTIME.L3.031j76` wires the preceding C++ missing-difficulty abort:
  if the dungeon's requested difficulty cannot be downscaled to a
  `MapDifficultyEntry`, Rust now sends `SMSG_TRANSFER_ABORTED` with
  `TRANSFER_ABORT_DIFFICULTY` and does not create/sync the canonical map.
  This is still not full `Player::TeleportTo`: expansion/pet/vehicle/duel/
  transport/BG cleanup, same-map near teleport, `WorldPortAck` fallback, LFG
  teleports, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j42` wires represented `Map::PlayerCannotEnter` into
  `WorldSession::teleport_to` before `SMSG_TRANSFER_PENDING`, matching C++
  `Player::TeleportTo` / `Map::PlayerCannotEnter` ordering. Target dungeon/raid
  teleports now preflight map entry, difficulty, access requirements, GM bypass,
  raid-group requirement, existing-map lock/max-player/in-combat gates, and the
  represented instance-count limit without creating/syncing the canonical target
  map or applying `CreateMap` side effects. Remaining gaps: this is not full
  `Player::TeleportTo`; same-map near teleport, expansion/pet/vehicle/duel/
  transport/BG cleanup, `WorldPortAck` fallback, LFG-specific teleports, bot,
  install/restart, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j43` wires the earlier C++ client-expansion teleport
  gate: if the session expansion is lower than the target `MapEntry::Expansion`,
  Rust now sends `SMSG_TRANSFER_ABORTED` with
  `TRANSFER_ABORT_INSUF_EXPAN_LVL` and the required expansion argument before
  `SMSG_TRANSFER_PENDING`. Remaining gaps: C++ transport passenger removal and
  graveyard repop on this branch, and the broader `Player::TeleportTo`
  cleanup/finalization paths remain pending.
- `#NEXT.RUNTIME.L3.031j77` wires the earlier C++ battleground/arena
  assignment gate in `Player::TeleportTo`: unassigned players targeting a
  battleground/arena map now return silently before expansion checks or
  transfer packets, while a represented assigned battleground player can still
  start the far transfer. Refs: `Player.cpp:1260`, `Player.h:2350`. Remaining
  gap: this uses Rust's represented battleground type seam as the available
  stand-in for C++ `m_bgData.bgInstanceID`; full `BattlegroundMgr`
  assignment/instance ownership and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j78` wires the C++ Death Knight starter-map escape
  guard in the far-teleport branch: a non-GM DK on map 609 without spell 50977
  now receives `TRANSFER_ABORT_UNIQUE_MESSAGE` with argument `1` before entry
  preflight or transfer packets, while a represented DK that knows 50977 can
  proceed. Refs: `Player.cpp:1349-1352`, `SharedDefines.h:148`, `Map.h:93`.
- `#NEXT.RUNTIME.L3.031j79` wires the accepted far-teleport
  `SetSelection(ObjectGuid::Empty)` ordering: Rust clears the represented
  selection after target-map preflight succeeds and before
  `SMSG_TRANSFER_PENDING`, while preflight aborts keep the existing selection.
  Ref: `Player.cpp:1370-1377`.
- `#NEXT.RUNTIME.L3.031j80` wires the accepted far-teleport
  `ResetContestedPvP()` side effects after successful target-map preflight:
  Rust clears `UNIT_STATE_ATTACK_PLAYER`, removes `PLAYER_FLAGS_CONTESTED_PVP`,
  resets the represented contested-PvP timer, and syncs the player registry
  before transfer packets; preflight aborts preserve the state because C++
  returns before `ResetContestedPvP()`. Refs: `Player.cpp:1389-1393`,
  `Player.cpp:20807-20812`, `Player.h:432`.
- `#NEXT.RUNTIME.L3.031j81` wires the accepted far-teleport
  `CombatStop(false, true)` phase for the represented current player before
  `ResetContestedPvP()`: Rust stops the current attack, emits
  `SMSG_ATTACK_STOP` when an attack target existed, emits C++'s empty
  `SMSG_CANCEL_COMBAT`, clears the player's PvE/PvP combat refs, removes
  attacker refs from canonical/legacy creatures or players, clears represented
  combat mirrors, and syncs the player registry. Refs: `Player.cpp:1389-1391`,
  `Unit.cpp:5756-5815`, `Player.cpp:20626-20629`,
  `CombatPackets.h:136-141`. Remaining gaps: same-map near teleport,
  delayed-teleport flags, full BG cleanup,
  pet/vehicle/duel/movement cleanup, bot, install/restart, and live-client
  validation remain pending.
- `#NEXT.RUNTIME.L3.031j82` wires the accepted far-teleport battleground leave
  comparison from C++ `Player::TeleportTo`: if the represented current
  battleground's `GetMapId()` differs from the destination map, Rust records a
  represented `LeaveBattleground(false)` request after `CombatStop()` and
  `ResetContestedPvP()` and before transfer packets. Teleporting into the
  represented battleground's own map does not leave it, matching the C++ join
  note in that branch. Refs: `Player.cpp:1395-1403`. Remaining gap: this is
  still a represented request only; live `BattlegroundMgr`, queue/member
  removal, score/world-state cleanup, and full BG runtime remain pending.
- `#NEXT.RUNTIME.L3.031j83` wires the accepted far-teleport
  `UnsummonPetTemporaryIfAny()` branch for represented pets on map change:
  when the current player has a represented pet GUID, Rust records one
  temporary-pet-unsummon request after the BG leave check and before transfer
  packets; players without represented pets do not request it. Refs:
  `Player.cpp:1414-1416`, `Player.cpp:26256-26266`. Remaining gap: this is the
  existing represented request seam only; live `Pet::RemovePet`,
  `m_temporaryUnsummonedPetNumber`, pet spell persistence, and resummon loading
  remain under full pet runtime work.
- `#NEXT.RUNTIME.L3.031j84` wires the accepted far-teleport
  `RemoveAllDynObjects()` branch for canonical map-owned DynamicObjects whose
  caster is the current player: Rust removes those typed DynamicObject records
  from the current canonical map before transfer packets and reuses the
  canonical `remove_from_map_like_cpp(..., true)` path so aura cleanup,
  caster-unbind cleanup, and Farsight viewpoint cleanup stay centralized.
  DynamicObjects owned by other casters remain on the map. Refs:
  `Player.cpp:1418-1419`, `Unit.cpp:5169-5174`,
  `DynamicObject.cpp:167-171`. Remaining gap: this is still bounded to
  canonical map-owned typed records; exact `Unit::m_dynObj` ordering,
  destroy-packet fanout/ObjectAccessor mirrors, scripts, live DB persistence,
  bot, install/restart, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j85` wires the accepted far-teleport
  `RemoveAllAreaTriggers()` branch for canonical map-owned AreaTriggers whose
  caster is the current player: Rust removes those typed AreaTrigger records
  from the current canonical map after `RemoveAllDynObjects()` and before
  transfer packets, leaving AreaTriggers owned by other casters intact. Refs:
  `Player.cpp:1421-1422`, `Unit.cpp:5347-5351`,
  `AreaTrigger.cpp:366-372`. Remaining gap: this is still bounded to canonical
  typed records; exact `Unit::m_areaTrigger` ordering, destroy-packet fanout,
  ObjectAccessor/session mirrors, AreaTrigger AI/target-list exit callbacks,
  scripts, live DB persistence, bot, install/restart, and live-client
  validation remain pending.
- `#NEXT.RUNTIME.L3.031j86` wires the accepted far-teleport
  non-melee spell interruption branch: after DynamicObjects and AreaTriggers
  are removed and before transfer packets, Rust now clears the represented
  delayed cast state and interrupts the canonical Player Unit's generic,
  channeled, and autorepeat current spell slots while preserving the melee
  slot, matching `IsNonMeleeSpellCast(true)` followed by
  `InterruptNonMeleeSpells(true)`. Preflight aborts still return before this
  cleanup. Refs: `Player.cpp:1424-1428`, `Unit.cpp:3050-3100`,
  `Unit.h:576-583`, `Unit.h:1394-1402`. Remaining gap: Rust now represents the
  `TELE_TO_SPELL` exception, but outgoing spell-interrupt packets, full Spell
  runtime state, bot, install/restart, and live-client validation remain
  pending.
- `#NEXT.RUNTIME.L3.031j87` wires the accepted far-teleport
  `RemoveAurasWithInterruptFlags(Moving | Turning)` branch: after non-melee
  spell interruption and before transfer packets, Rust now removes represented
  visible auras and canonical Player Unit applied auras carrying Trinity's
  `SpellAuraInterruptFlags::Moving` or `Turning`, while preserving unrelated
  auras. Refs: `Player.cpp:1430-1431`, `Unit.cpp:4051-4108`,
  `SpellDefines.h:70-109`. Remaining gap: this is still a bounded
  represented/canonical local cleanup; full aura scripts/procs, remove-mode
  fanout beyond represented `SMSG_AURA_UPDATE`, `CanCastSpellWhileMoving`
  exception handling from `IsInterruptFlagIgnoredForSpell`, channel-interrupt
  edge cases, bot, install/restart, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j88` wires the accepted far-teleport
  transport-server-time cleanup around `SMSG_TRANSFER_PENDING`: immediately
  after sending `TransferPending`, Rust now removes the represented
  `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME` bit and resets the
  represented ActivePlayer `TransportServerTime` to zero; preflight aborts
  still preserve both values because C++ returns before this branch. Refs:
  `Player.cpp:1433-1449`, `Player.h:487`, `Player.h:2774-2787`,
  `MovementHandler.cpp:812-813`. Remaining gap: full update-field propagation,
  bot, install/restart, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j89` adds represented `TeleportToOptions` bits from C++
  `Player.h:790-805` and wires the far-teleport branches that were previously
  unreachable in Rust: `TELE_TO_SPELL` now skips the non-melee spell interrupt
  branch, valid `TELE_TO_SEAMLESS` suppresses `SMSG_TRANSFER_PENDING` and sends
  `SMSG_SUSPEND_TOKEN` with reason `2`, invalid seamless requests fall back to
  normal transfer after the C++ cosmetic-parent-map gate, and
  `PlayerLogout()` suppresses both `TransferPending` and `SuspendToken` without
  clearing transport-server time. Refs: `Player.cpp:1368-1371`,
  `Player.cpp:1424-1469`, `Player.h:790-805`, `WorldSession.h:1034`.
  Remaining gaps: real transport passenger removal, revive-at-teleport,
  not-leave-combat coverage outside same-map near teleport, delayed teleport
  semaphores, install/restart, bot, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j90` starts the C++ same-map/near-teleport branch:
  `SMSG_MOVE_TELEPORT` serialization now mirrors
  `WorldPackets::Movement::MoveTeleport::Write` for the represented
  no-transport/no-vehicle case, same-map `WorldSession::teleport_to` sets the
  near-teleport pending destination instead of entering the far-transfer state,
  sends `MoveTeleport` rather than `TransferPending`, suppresses that packet
  during `PlayerLogout()`, and honors `TELE_TO_NOT_LEAVE_COMBAT` for this
  branch. The dungeon `PlayerCannotEnter` preflight is now kept in the far
  branch, matching C++ `Player::TeleportTo`. Refs: `Player.cpp:1310-1346`,
  `Unit.cpp:12208-12244`, `MovementPackets.h:303-318`,
  `MovementPackets.cpp:705-724`, `MovementHandler.cpp:263-324`.
  Remaining gaps: transport/vehicle payloads, temporary-pet DB
  resummon/stable persistence, full
  `ResurrectPlayer` side effects beyond represented health/powers, delayed far
  teleports/quest-reward activation, install/restart, bot, and live-client
  validation remain pending.
- `#NEXT.RUNTIME.L3.031j91` adds the represented nearby-player fanout half of
  C++ `Unit::SendTeleportPacket`: `SMSG_MOVE_UPDATE_TELEPORT` now serializes
  the C++ `MoveUpdateTeleport::Write` field order for the represented
  no-movement-forces/no-speed-optionals case, and same-map player teleports
  queue that packet to nearby visible sessions through the existing
  `SendIfVisibleLikeCpp` HaveAtClient gate while excluding the moved player.
  This mirrors the C++ player branch in `Unit.cpp:12208-12255`: destination is
  still carried by the moved player's `SMSG_MOVE_TELEPORT`; nearby observers
  receive the represented current `MovementInfo` status, matching the legacy
  `moveUpdateTeleport.Status = &m_movementInfo` shape rather than inventing a
  corrected destination payload. Refs: `Unit.cpp:12208-12255`,
  `MovementPackets.h:319-334`, `MovementPackets.cpp:750-795`.
  Remaining gaps: transport/vehicle offsets and GUID payloads, movement forces,
  optional speed payloads, temporary-pet DB resummon/stable persistence, full
  `ResurrectPlayer` side effects
  beyond represented health/powers, delayed far teleports/quest-reward
  activation, install/restart, bot, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j92` ports the same-map
  `TELE_REVIVE_AT_TELEPORT` branch from C++ `Player::TeleportTo`: a dead
  represented player now runs the local `ResurrectPlayer(0.5f)` state update
  before `CombatStop`, restoring represented/canonical health to 50%, mana to
  50%, energy to 50%, focus to 50%, and rage to 0 while preserving the same-map
  `SMSG_MOVE_TELEPORT` path. The no-flag same-map branch remains a corpse
  teleport, matching the C++ guard. Refs: `Player.h:804`,
  `Player.cpp:1328-1329`, `Player.cpp:4320-4354`.
  Remaining gaps: `SMSG_DEATH_RELEASE_LOC`, area-spirit-healer clearing,
  ghost/night-elf aura removals, water-walk/rooted movement flag packets,
  zone/visibility refresh, outdoor-PvP/BG callbacks, item obtain spell recasts,
  resurrection sickness, temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, delayed
  far teleports/quest-reward activation, install/restart, bot, and live-client
  validation remain pending.
- `#NEXT.RUNTIME.L3.031j93` ports the same-map pet distance gate from C++
  `Player::TeleportTo`: unless `TELE_TO_NOT_UNSUMMON_PET` is set, a represented
  pet with a canonical map record now requests temporary unsummon only when its
  3D distance to the same-map destination exceeds the map visibility range
  (`pet->IsWithinDist3d(x, y, z, GetMap()->GetVisibilityRange())` in C++).
  This preserves the C++ distinction between far teleports, which prepare a
  temporary unsummon unconditionally when a pet exists, and same-map near
  teleports, which only do it for an out-of-range pet. Refs:
  `Player.cpp:1254`, `Player.cpp:1321-1325`, `Object.cpp:1128-1136`,
  `Player.h:801`.
  Remaining gaps: real `ResummonPetTemporaryUnSummonedIfAny` DB load,
  `Pet::SavePetToDB(PET_SAVE_AS_CURRENT)` slot/current-pet persistence,
  transport/vehicle payloads, delayed far teleports/quest-reward activation,
  install/restart, bot, and live-client validation remain pending.
- `#NEXT.RUNTIME.L3.031j94` represents the C++ same-map delayed-teleport
  semaphore path. While `Player::m_bCanDelayTeleport` is represented as active,
  same-map `TeleportTo` now sets `m_bHasDelayedTeleport`, marks the near
  semaphore, stores `m_teleport_dest`/`m_teleport_options`, and returns before
  pet unsummon, revive, combat stop, fall reset, fanout, or
  `SMSG_MOVE_TELEPORT`. `WorldSession::update` opens the represented
  `CanDelayTeleport` window around its player/unit update work, closes it, and
  replays a saved same-map teleport only for alive players, matching the C++
  `Player::Update` delayed-execution guard. Refs: `Player.cpp:933-935`,
  `Player.cpp:1154-1155`, `Player.cpp:1306-1318`, `Player.h:2185-2189`,
  `Player.h:3095-3098`.
  Remaining gaps: temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, install/restart, bot, and live-client validation
  remain pending.
- `#NEXT.RUNTIME.L3.031j95` extends the represented delayed-teleport semaphore
  path to far/cross-map teleports. When the represented
  `Player::m_bCanDelayTeleport` window is active, far `TeleportTo` now stores
  `m_teleport_dest`/`m_teleport_options` and returns before selection clear,
  combat stop, contested-PvP reset, battleground leave, pet unsummon, dynamic
  object/area-trigger cleanup, spell interruption, aura removal,
  `SMSG_TRANSFER_PENDING`, `SMSG_SUSPEND_TOKEN`, or `pending_teleport`. The
  post-update replay runs only for alive players and then starts the normal far
  transfer path. Refs: `Player.cpp:933-935`, `Player.cpp:1154-1155`,
  `Player.cpp:1372-1386`, `Player.cpp:1388-1473`, `Player.h:2185-2189`,
  `Player.h:3095-3098`.
  Remaining gaps: temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, install/restart, bot, and live-client validation
  remain pending.
- `#NEXT.RUNTIME.L3.031j96` represents the `RewardQuest`-scoped delayed-teleport
  window. `reward_represented_quest_like_cpp` now opens the represented
  `Player::m_bCanDelayTeleport` flag on entry, keeps it active through
  represented reward/display spell casts, closes it on every reward abort, and
  closes it again before returning success. Reward-spell evidence records
  whether the C++ delay window was active so tests can prove the timing, not
  only the final state. Refs: `Player.cpp:14635`, `Player.cpp:14830-14870`,
  `Player.cpp:14893`.
  Remaining gaps: temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, install/restart, bot, and live-client validation
  remain pending.
- `#NEXT.RUNTIME.L3.031j97` separates the represented far teleport semaphore
  from `pending_teleport`. Rust now tracks C++ `Player::mSemaphoreTeleport_Far`
  as its own state, sets it on immediate and delayed far `TeleportTo`, clears it
  when same-map near teleport starts, and makes `WorldPortResponse` ignore
  unexpected far-teleport acks unless the semaphore is set. The ack clears the
  semaphore before consuming the stored destination, matching
  `WorldSession::HandleMoveWorldportAck`. Refs: `Player.cpp:203-204`,
  `Player.cpp:1306`, `Player.cpp:1381`, `Player.cpp:1473`,
  `MovementHandler.cpp:53-58`, `Player.h:2184-2189`, `Player.h:3116-3117`.
  Remaining gaps: temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, install/restart, bot, and live-client validation
  remain pending.
- `#NEXT.RUNTIME.L3.031j98` wires represented canonical old-map removal for far
  teleports. Immediate far `TeleportTo` and delayed far replay now remove the
  current player from the canonical old map with `remove_from_map_like_cpp(...,
  false)` after transfer-pending side effects and before storing the pending
  far destination, matching `oldmap->RemovePlayerFromMap(this, false)`. The
  `WorldPortResponse` path now also runs canonical destination map creation/sync
  after relocating the session state, matching the C++ ack phase that removes a
  still-in-world player from the old map and adds it to the destination map.
  Refs: `Player.cpp:1454`, `MovementHandler.cpp:83-88`,
  `MovementHandler.cpp:102-123`.
  Remaining gaps: temporary-pet DB resummon/stable persistence,
  transport/vehicle payloads, install/restart, bot, and live-client validation
  remain pending.
- `#NEXT.RUNTIME.L3.031j99` represents the core state mutation inside
  `Player::UnsummonPetTemporaryIfAny`. When a represented active pet is
  temporarily unsummoned, Rust now reads the canonical pet's `CharmInfo`
  pet-number when the pet is controlled and not temporary, stores represented
  `m_temporaryUnsummonedPetNumber` plus represented `m_oldpetspell`, removes the
  pet from the canonical map with `remove_from_map_like_cpp(..., false)`, and
  clears the represented active pet GUID. Temporary summoned pets are removed
  without storing a pet number, matching the C++ guard. Refs:
  `Player.cpp:26256-26268`, `Player.cpp:20869-20906`,
  `Pet.cpp:468-486`.
  Remaining gaps: real `ResummonPetTemporaryUnSummonedIfAny` DB load,
  `Pet::SavePetToDB(PET_SAVE_AS_CURRENT)` slot/current-pet persistence,
  reagent-return side effects, transport/vehicle payloads, install/restart,
  bot, and live-client validation remain pending.

**Tests existing:**
- `cargo test -p wow-instances -- --nocapture` currently covers 19 focused tests, including C++-contrasted lock key/binding, daily/weekly reset anchors, temporary lock creation, active lock lookup, temp promotion, expired-lock replacement, DB row reconstruction, shared weak-ref cleanup, prepared-statement parameter order, flex-mask join rejection, different-instance rejection, and reset in-use guard.
- `cargo test -p wow-database -- --nocapture` covers the newly registered character DB statements and placeholder counts.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#INSTANCES.WBS.001** Partir y cerrar la migracion auditada de `game/Instances/InstanceLockMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 599 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.002** Cerrar la migracion auditada de `game/Instances/InstanceLockMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.h`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.003** Partir y cerrar la migracion auditada de `game/Instances/InstanceScript.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 971 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.004** Cerrar la migracion auditada de `game/Instances/InstanceScript.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.005** Cerrar la migracion auditada de `game/Instances/InstanceScriptData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScriptData.cpp`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.006** Cerrar la migracion auditada de `game/Instances/InstanceScriptData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScriptData.h`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INSTANCES.WBS.007** Cerrar la migracion auditada de `game/Instances/enuminfo_InstanceScript.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/enuminfo_InstanceScript.cpp`
  Rust target: `crates/wow-instances`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [x] **#INST.1** Create `crates/wow-instances/` crate with `Cargo.toml`; add to workspace (L)
- [x] **#INST.2** Port `InstanceLockData`, `InstanceLock`, `SharedInstanceLockData`, `SharedInstanceLock` structs (L)
- [x] **#INST.3** Port `MapDb2Entries` w/ `GetKey()`, `IsInstanceIdBound()` (M) — Rust uses an explicit DB2-derived value object; direct `wow-data-stores` resolver pending integration.
- [~] **#INST.4** Define `InstanceLockKey = (u32 mapId, u32 lockId)` and `EncounterState`, `EncounterDoorBehavior`, `EncounterFrameType` enums in `wow-constants` (L) — `InstanceLockKey`, `EncounterState`, and instance-id masks are in `wow-instances`; door/frame enums still pending.
- [x] **#INST.5** Implement `InstanceLockMgr` skeleton: temporary/permanent player lock maps plus shared data map (M)
- [x] **#INST.6** Register prepared statements for `instance`, `character_instance_lock` in `wow-database` (SEL/REP/DEL) (M) — also includes C++ `account_instance_times` statements.
- [x] **#INST.7** Implement `InstanceLockMgr::load()` — async load all rows + reconstruct shared map (H) — DB query, row reconstruction, DB2 resolver, startup invocation, shared runtime/session injection, and persisted instance-id registration done.
- [x] **#INST.8** Implement `CanJoinInstanceLock` returning `TransferAbortReason` enum mirror (M)
- [x] **#INST.9** Implement `FindActiveInstanceLock` honoring extended-but-expired (M) — group-leader owner selection remains a caller responsibility until group/map wiring.
- [x] **#INST.10** Implement `CreateInstanceLockForNewInstance` → temporary map (L)
- [~] **#INST.11** Implement `UpdateInstanceLockForPlayer` (promote temp→perm, REPLACE row, transactional) (H) — in-memory promotion/merge plus C++ delete/insert transaction append path done; real gameplay call-sites pending.
- [~] **#INST.12** Implement `UpdateSharedInstanceLock` for raid-id locks (M) — in-memory shared data plus C++ delete/insert transaction append path done; real gameplay call-sites pending.
- [~] **#INST.13** Implement `UpdateInstanceLockExtensionForPlayer` (toggle + recompute expiry) (M) — in-memory behavior plus C++ update statement transaction append path done; real calendar/player call-sites pending.
- [~] **#INST.14** Implement `ResetInstanceLocksForPlayer` w/ in-use guard (M) — in-memory expiry plus C++ force-expire transaction append path done; real reset handler/call-sites pending.
- [x] **#INST.15** Implement `OnSharedInstanceLockDataDelete` cleanup (L) — Rust exposes cleanup builder for unreferenced weak shared data; actual DB execution remains caller responsibility.
- [x] **#INST.16** Implement `GetNextResetTime` from `MapDifficulty.ResetInterval` + reset-hour config (M)
- [x] **#INST.17** Implement instance-id allocator + free-id reuse (M) — contrasted with C++: this fork declares mask constants but `MapManager::GenerateInstanceId` returns sequential ids without OR-ing masks.
- [~] **#INST.18** Implement `account_instance_times` rate-limit (5 per hour) check (M) — player-side `CheckInstanceCount`/`AddInstanceEnterTime` runtime behavior and `AccountInstancesPerHour` config wired; C++ load/delete/insert statements registered; login-load/logout-save DB integration pending.
- [ ] **#INST.19** Port `BossInfo`, `DoorData`, `DoorInfo`, `MinionData`, `MinionInfo`, `ObjectData`, `DungeonEncounterData`, `BossBoundaryEntry` (M)
- [ ] **#INST.20** Define `InstanceScript` trait + default impl struct (`bosses`, `doors`, `minions`, `_creature_info`, `_go_info`, `_object_guids`, `_persistent_values`, `_entrance_id`, `_combat_res_*`) (H)
- [~] **#INST.21** Implement `Create`, `Load(json)`, `GetSaveData()` w/ `serde_json` mirroring `InstanceScriptData.cpp` (header + bosses[] + persistent) (H) — pure C++ JSON reader/writer, transient-state normalization, and numeric persistent values done; `AfterDataLoad`, spawn-group updates, and map call-sites pending.
- [~] **#INST.22** Implement `SetBossState(id, state)` w/ door/minion/spawn-group/LFG hooks + save-trigger + `InstanceMap::UpdateInstanceLock` (H) — pure C++ state-transition guards and side-effect plan done; actual `InstanceMap` execution, criteria, LFG, doors/minions, and DB update call-site pending.
- [ ] **#INST.23** Implement `OnCreatureCreate/Remove` and `OnGameObjectCreate/Remove` w/ `_creature_info` + `_go_info` lookups (M)
- [x] **#INST.24** Implement `IsEncounterInProgress`, `IsEncounterCompleted`, `IsEncounterCompletedInMaskByBossId`, `GetEncounterCount` (L)
- [ ] **#INST.25** Implement `UpdateDoorState`, `UpdateMinionState`, `HandleGameObject`, `DoUseDoorOrButton`, `DoCloseDoorOrButton`, `DoRespawnGameObject` (M)
- [~] **#INST.26** Implement encounter-frame packet senders: `SendEncounterUnit`, `SendEncounterStart`, `SendEncounterEnd` (M) — C++ packet builders done; real `InstanceScript::SendToPlayers` call-sites pending.
- [~] **#INST.27** Implement `SendBossKillCredit` (`SMSG_BOSS_KILL`) (L) — C++ packet builder done; real boss-state call-site pending.
- [ ] **#INST.28** Implement `DoUpdateWorldState`, `DoCastSpellOnPlayers`, `DoRemoveAurasDueToSpellOnPlayers`, `DoUpdateCriteria`, `DoSendNotifyToInstance` (M)
- [x] **#INST.29** Implement combat-resurrection tracker (`InitializeCombatResurrections`, `Use`, `Reset`, `GetCombatResurrectionChargeInterval`, `Update`) (M)
- [x] **#INST.30** Implement entrance-location resolver (`SetEntranceLocation`, `Get`, `ComputeEntranceLocationForCompletedEncounters`) (M)
- [~] **#INST.31** Implement `PersistentInstanceScriptValue<T>` (i64 / f64 variant) w/ change-notify (M) — numeric value registration/save/load done; change-notify event into `InstanceMap::UpdateInstanceLock(UpdateAdditionalSaveDataEvent)` pending.
- [x] **#INST.32** Implement `MarkAreaTriggerDone` / `IsAreaTriggerDone` set tracking (L)
- [ ] **#INST.33** Implement `UpdateLfgEncounterState` integration (depends on `wow-lfg` doc) (M)
- [ ] **#INST.34** Implement `UpdatePhasing` integration (depends on `phasing.md`) (M)
- [~] **#INST.35** Implement `Player::SendRaidInfo` → `SMSG_INSTANCE_INFO` builder (M) — C++ packet layout, empty `CMSG_REQUEST_RAID_INFO` fallback, pure `InstanceLockMgr` raid-info view, session/shared-manager read path, DB2 resolver, and startup population done; gameplay call-sites that create/update/reset real locks still pending.
- [~] **#INST.36** Implement `WorldSession::HandleResetInstancesOpcode` (`CMSG_RESET_INSTANCES`) → call `ResetInstanceLocksForPlayer` or `Group::ResetInstances` (M) — opcode registration/dispatch, packet builders, outside-instance guard, group-leader guard, lock reset transaction, reset notifications, represented player/group recent-instance state, represented owned-instance refs, and reset-result erase rules done; exact live `InstanceMap::Reset` integration and map-result-driven packet fanout pending.
- [~] **#INST.37** Implement `SMSG_INSTANCE_RESET` / `_FAILED` / `SMSG_RAID_INSTANCE_MESSAGE` packet senders (M) — reset/reset-failed/raid-instance-message packet builders and represented reset sends done; real movement welcome and expire broadcast call-sites pending with map transition/`InstanceMap`.
- [~] **#INST.38** Implement `SMSG_PENDING_RAID_LOCK` + `CMSG_INSTANCE_LOCK_RESPONSE` round-trip (M) — packet layouts, handler dispatch, represented pending bind accept/decline state done; real prompt creation from `InstanceMap::AddPlayerToMap` and `ConfirmPendingBind` map binding pending.
- [~] **#INST.39** Implement instance respawn purge on reset (`respawn` rows by `mapId` + `instanceId`) (M) — C++ `CHAR_DEL_ALL_RESPAWNS` SQL and Rust prepared statement builder done; real reset call-site pending with `InstanceMap`.
- [ ] **#INST.40** Wire `InstanceMap::Update(diff)` → `InstanceScript::Update` + combat-res tick (L)
- [x] **#INST.41** Audit & fix `MapManager::GenerateInstanceId` in WIP `map_manager.rs` (L) — contrasted with C++ `MapManager.cpp`: no mask OR is applied by this fork; Rust now reserves 0, registers loaded ids, returns the lowest free sequential id, and reuses freed ids.
- [ ] **#INST.42** Add GM commands `.instance unbind`, `.instance reset`, `.instance stats` (M)
- [~] **#INST.43** Plumb `CanJoinInstanceLock` into the teleport / portal / `MapManager::PlayerCannotEnter` path (H) — live-session canonical dungeon `CreateMap` now applies the C++ missing-`MapDifficulty` `TRANSFER_ABORT_DIFFICULTY` gate and the `InstanceMap::CannotEnter` lock compatibility gate for existing maps; full `Map::PlayerCannotEnter` gates and all portal/teleport call-sites remain pending.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#INSTANCES.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 7 files / 2783 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#INSTANCES.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 7 files / 2783 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#INSTANCES.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 7 files / 2783 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#INSTANCES.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 7 files / 2783 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: lock-id-based vs instance-id-based binding produce different `InstanceLockKey` and different DB persistence shape.
- [ ] Test: `GetNextResetTime` returns stable answer for daily / 3-day / weekly `ResetInterval` regardless of caller wall-clock seconds (anchor on reset-hour).
- [ ] Test: temporary lock (no boss killed) is purged on player logout / map destroy without writing `character_instance_lock`.
- [ ] Test: first boss kill promotes temp → perm and writes both `instance` row (for ID-based) and `character_instance_lock` row.
- [x] Test: `CanJoinInstanceLock` returns `TRANSFER_ABORT_LOCKED_TO_DIFFERENT_INSTANCE` when player has lock to instanceId X but tries to enter Y of same `(mapId, lockId)` — pure `wow-instances` coverage plus `#NEXT.RUNTIME.L3.031j75` live-session canonical dungeon `CreateMap` coverage for the C++ `InstanceMap::CannotEnter` existing-map gate. Refs: `Map.cpp:1808-1811`, `Map.cpp:2836-2869`, `MapManager.cpp:247-279`, `InstanceLockMgr.cpp:188-202`.
- [x] Test: live dungeon entry sends `TRANSFER_ABORT_DIFFICULTY` and creates no canonical map when `GetDownscaledMapDifficultyData` has no `MapDifficultyEntry` — covered by `#NEXT.RUNTIME.L3.031j76`; refs: `Map.cpp:1783-1788`, `Map.h:92`.
- [ ] Test: `extended=true` lets player join a lock that is past `_expiryTime` but not past `effectiveExpiryTime` (= next reset).
- [ ] Test: `ResetInstanceLocksForPlayer` skips locks where `IsInUse()` (active map) and reports them in `locksFailedToReset`.
- [~] Test: `InstanceScript::SetBossState(id, DONE)` flips door states per `EncounterDoorBehavior` and updates `completedEncountersMask` — DONE transition side-effect plan and encounter id resolution covered; real door/minion mutation and DB mask update pending.
- [~] Test: `Load(GetSaveData())` round-trips boss states + persistent values losslessly (JSON stable) — JSON shape, load normalization, persistent numeric load, and C++ error cases covered; full InstanceMap integration pending.
- [x] Test: `IsEncounterInProgress()` returns true iff any boss in `IN_PROGRESS`.
- [x] Test: `account_instance_times` blocks the 6th distinct enter within 1 hour.
- [ ] Test: shared-state instance lock — two players in same group see the same `completedEncountersMask` even if one lock row is older.
- [~] Test: `respawn` rows for `(mapId, instanceId)` are deleted when the instance is reset — statement-builder coverage done; integration through real reset call-site pending.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 7 files / 2783 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h` | `crates/wow-instances/` exists with the #NEXT.R8.ENTITIES.303 encounter-metadata foundation; full module still mostly pending |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#INSTANCES.DIV.001` | `crates/wow-instances` | 7 C++ files / 2783 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceScript.h` | `foundation_started` | Declared Rust target exists as of #NEXT.R8.ENTITIES.303; broad C++ surface remains to port. |

<!-- REFINE.023:END known-divergences -->

- **Instance ID is *not* the same as `mapId`.** `mapId` identifies the dungeon (e.g., 533 = Naxxramas); `instanceId` is a server-allocated global counter that goes into the instance map key alongside `mapId`. The bits encode kind: `INSTANCE_ID_HIGH_MASK = 0x1F440000`, `_LFG_MASK = 0x00000001`, `_NORMAL_MASK = 0x00010000`. Get this wrong and DB rows from prior runs will collide.
- **Lock owner = group leader if grouped, else self.** Crucial — when a player leaves the group, their lock record persists referencing the *old* group-leader's guid as owner, but their own `character_instance_lock.guid` is themselves. The two-table split (`instance` shared + `character_instance_lock` per-player) exists exactly to handle this.
- **Two lock states.** A temporary lock exists from instance-creation until first boss kill; it is *not* persisted. Don't write to DB on every player enter — only on first encounter completion.
- **The `data` blob is rapidjson, not binary.** `InstanceScript::GetSaveData()` produces JSON with shape `{ "Header": "...", "BossStates": [...], "AdditionalData": { ... } }`. Header is set by `SetHeaders()` and used as a versioning string — old script versions write a different header and are rejected on load.
- **`MAX_DUNGEON_ENCOUNTERS_PER_BOSS = 4`** because some bosses have separate `DungeonEncounter` entries per difficulty (10N, 25N, 10H, 25H).
- **Reset cron is in `MapManager::Update`, not `InstanceLockMgr`.** The lock manager only knows *next* reset time per (mapId, lockId); the actual purge of locks + respawn rows + active-instance tear-down is driven by `MapManager`.
- **`character_instance_lock.expiryTime` is unix timestamp (uint64), not a relative offset.** All lock comparisons use absolute wall clock.
- **`account_instance_times`** — Blizzard's anti-grief: a single account can enter at most 5 distinct instance IDs per 1-hour rolling window. This is enforced in `Player::CheckInstanceCount` and queries `account_instance_times`. Easy to forget — leads to "Too many instances" client errors at edge cases.
- **`ASSERT(map->IsDungeon())`** in `InstanceScript` ctor — only instance maps. Battlegrounds and outdoor maps have their own subclasses.
- **Re-entry into in-progress instance.** `IsEncounterInProgress() == true` blocks new players from entering — sends `TRANSFER_ABORT_ZONE_IN_COMBAT`. `OnPlayerEnter` only fires for already-permitted entries.
- **`SendEncounterStart`** must be called *before* the first boss state goes `IN_PROGRESS`, otherwise the client UI does not draw the boss frame. Order matters.
- **Combat-res charges** are 1 by default in 25-man, recharge `interval_ms` on a timer; `_combatResurrectionTimerStarted` only flips on first encounter engage.
- **rapidjson dependency.** In Rust, prefer `serde_json` and a stable schema struct — *do not* duplicate the C++ pointer-walking style.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class InstanceLockMgr` (singleton) | `pub struct InstanceLockMgr` + `OnceLock<InstanceLockMgr>` global | `sInstanceLockMgr` macro → `wow_instances::lock_mgr()` |
| `std::shared_mutex _locksMutex` | `tokio::sync::RwLock<>` or `parking_lot::RwLock` | Reads dominate; writes only on boss kills/resets |
| `std::unordered_map<ObjectGuid, PlayerLockMap>` | `DashMap<ObjectGuid, HashMap<InstanceLockKey, Arc<InstanceLock>>>` | DashMap because access is sharded by playerGuid |
| `std::weak_ptr<SharedInstanceLockData>` | `Weak<SharedInstanceLockData>` | Shared raid lock state ref |
| `std::unique_ptr<InstanceLock>` | `Arc<InstanceLock>` (or `Box` if single-owner) | Multiple readers possible from group members |
| `std::chrono::system_clock::time_point` | `std::time::SystemTime` or `chrono::DateTime<Utc>` | Pick one project-wide |
| `Optional<uint32>` | `Option<u32>` | trivial |
| `std::variant<int64, double>` | `enum PersistentValue { Int(i64), Float(f64) }` | Same shape |
| `class InstanceScript : public ZoneScript` | `pub trait InstanceScript: ZoneScript` + `pub struct InstanceScriptBase` providing default fields | No virtual inheritance; trait + composition |
| `BossInfo::door[Behavior::Max]` (array of GuidSet) | `[HashSet<ObjectGuid>; EncounterDoorBehavior::COUNT]` | Fixed-size array |
| `EncounterState` enum | `#[repr(u8)] enum EncounterState { NotStarted=0, InProgress=1, Fail=2, Done=3, Special=4, ToBeDecided=5 }` | u8 wire-compatible |
| `rapidjson` save/load | `serde_json::Value` + `#[derive(Serialize, Deserialize)]` schema struct | Strict schema with `header` field for versioning |
| `PersistentInstanceScriptValue<T>` | `pub struct PersistentValue<T>` w/ `Drop`-on-script-detach + change-notify channel | Use observer pattern, not raw pointer registry |
| `void Update(uint32 diff)` | `fn update(&mut self, diff: Duration)` | All `uint32` ms → `Duration` |
| `WorldPacket` builders | `wow-shared-packets` packet structs | Already exists in repo |
| Global `INSTANCE_ID_HIGH_MASK` etc. | `pub const INSTANCE_ID_HIGH_MASK: u32 = 0x1F44_0000;` in `wow-constants` | trivial port |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-10)

🟡 parcialmente mitigado. Auditado contra `/home/server/rustycore/crates/` y contrastado con `/home/server/woltk-trinity-legacy/src/server/game/Instances/InstanceLockMgr.cpp/.h`.

**Hallazgos clave:**
- `crates/wow-instances/` existe y contiene las constantes `INSTANCE_ID_HIGH_MASK`, `_LFG_MASK`, `_NORMAL_MASK`; `MapManager::GenerateInstanceId` fue contrastado y en C++ no usa esas máscaras.
- Los 2 únicos call-sites de `add_creature(0,0,0,0,…)` están en tests del propio `map_manager.rs` (líneas 761, 774). Sin caller real, todavía falta conectar el allocator a la creación real de instancias.
- `InstanceLockMgr` análogo: core in-memory creado en `#NEXT.R8.INSTANCES.001`; statements, builders, async load glue y weak-ref cleanup creados en `#NEXT.R8.INSTANCES.002`; shared manager inyectado en world-server/session; startup DB2 resolver/load creado en `#NEXT.R8.INSTANCES.003`; wrappers transaccionales de update/shared creados en `#NEXT.R8.INSTANCES.004`; wrappers transaccionales de extension/reset creados en `#NEXT.R8.INSTANCES.005`; handler representado de reset creado en `#NEXT.R8.INSTANCES.006`; SQL/builder de purge de respawns creado en `#NEXT.R8.INSTANCES.008`; call-sites de gameplay siguen pendientes.
- `CMSG_REQUEST_RAID_INFO` registrado y responde `SMSG_INSTANCE_INFO`; lee el shared `InstanceLockMgr` cuando exista player GUID y resuelve locks cargados con `Map.db2`/`MapDifficulty.db2`, manteniendo fallback vacío si no hay locks. `CMSG_RESET_INSTANCES` registrado y responde `SMSG_INSTANCE_RESET`/`SMSG_INSTANCE_RESET_FAILED` desde locks persistidos representados. `CMSG_INSTANCE_LOCK_RESPONSE` registrado y consume estado pending-bind representado. Builders existentes: `SMSG_PENDING_RAID_LOCK`, `SMSG_INSTANCE_SAVE_CREATED`, `SMSG_RAID_INSTANCE_MESSAGE`, `SMSG_INSTANCE_ENCOUNTER_*` basicos, `SMSG_BOSS_KILL`; falta creación real del pending desde `InstanceMap`.

**Riesgo de UI hang silencioso:**
- `CMSG_REQUEST_RAID_INFO` ya no queda silencioso: responde `SMSG_INSTANCE_INFO` vacío o con locks reales cargados desde DB si existen filas persistidas resolubles. La prueba de cliente puede validar que la pestaña no queda cargando indefinidamente y, con fixture DB, que muestra locks reales.
- Bajo riesgo de UI hang en el resto del flujo: el cliente nunca llega a "set difficulty" o "reset instance" porque el botón "Enter Dungeon" requiere primero un raid info populado.

**Acción:** mantener `🟡 foundation in progress`. Siguiente cierre recomendado: completar el wiring runtime de `#INST.11/#INST.12/#INST.36` antes de persistir nuevos locks generados por gameplay.

## 14. Runtime entry gates (2026-06-16)

🟡 parcialmente mitigado. Auditado contra `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp` y `Map.h`.

**Cerrado en Rust:**
- `MapDifficulty.db2::MaxPlayers` ya se carga desde el campo físico 6 y se propaga por `wow_instances::MapDb2Entries`.
- La entrada a un mapa de instancia canónica existente comprueba `player_count >= max_players` y envía `SMSG_TRANSFER_ABORTED` con `TRANSFER_ABORT_MAX_PLAYERS = 2`, antes del gate de lock compatible, igual que `InstanceMap::CannotEnter`.
- La entrada a una raid canónica existente con encuentro en progreso envía `SMSG_TRANSFER_ABORTED` con `TRANSFER_ABORT_ZONE_IN_COMBAT = 6` para jugadores no-GM que no estén en estado de loading/relog, antes del gate de lock compatible, igual que `InstanceMap::CannotEnter`.
- `Map.db2::ExpansionID` ya se carga en `wow_data::MapEntry`, y la entrada a raids de la expansión actual exige grupo raid salvo GM o `Instance.IgnoreRaid=1`, enviando `SMSG_TRANSFER_ABORTED` con `TRANSFER_ABORT_NEED_GROUP = 11` antes de resolver/crear instancia como `Map::PlayerCannotEnter`.
- `access_requirement` ya se carga desde world DB con las columnas exactas de C++ `ObjectMgr::LoadAccessRequirements`, validando mapa/dificultad y anulando item/quest/achievement inexistentes como Trinity. `WorldSession::ensure_canonical_world_map_for_current_player_like_cpp` ejecuta el gate `Player::Satisfy` en la posición C++: `MapDifficultyXCondition`, `Instance.IgnoreLevel`, nivel min/max, item/item2, quest por facción y achievement representado del jugador actual.
- `MapDifficulty.db2::Message` ya se conserva en `wow_data::MapDifficultyEntry` y el gate representado de `Player::Satisfy` replica la rama C++ que convierte un fallo de access requirement en `TRANSFER_ABORT_DIFFICULTY` cuando el `MapDifficulty` efectivo tiene mensaje localizado, incluso si el fallo concreto es nivel/item/quest/achievement.
- `Player::CheckInstanceCount` ya está representado en `WorldSession` con `_instanceResetTimes` por cuenta, `AccountInstancesPerHour`, limpieza de expirados, reentrada a instancia existente y abort `TRANSFER_ABORT_TOO_MANY_INSTANCES = 4` en la posición C++ de `Map::PlayerCannotEnter`. `Map.db2::Flags[1]` se carga como `flags2` para respetar `MapFlags2::IgnoreInstanceFarmLimit`, y una entrada aceptada registra `AddInstanceEnterTime(instanceId, now + HOUR)` como `InstanceMap::AddPlayerToMap`.
- `_instanceResetTimes` ya se carga desde `account_instance_times` durante login con `CHAR_SEL_ACCOUNT_INSTANCELOCKTIMES`, respetando la semántica `std::map::insert` de C++ (si hubiera duplicados por `instanceId`, gana la primera fila), y se guarda durante `SaveToDB` con `CHAR_DEL_ACCOUNT_INSTANCE_LOCK_TIMES` seguido de `CHAR_INS_ACCOUNT_INSTANCE_LOCK_TIMES` por cada entrada. Se conserva la rama C++ que retorna sin borrar cuando el mapa está vacío; si eso resulta ser stale-row bug, debe corregirse en un bugfix explícito C+++Rust.
- El gate de instancia llena usa ahora `ManagedMap::players_count_except_gms_like_cpp()`: si existen `Player` tipados en el mapa canónico, cuenta solo los no-GM como `Map::GetPlayersCountExceptGMs`; si el mapa aún solo tiene el contador representado legacy, conserva ese fallback hasta que toda la población sea tipada. El snapshot canónico de `WorldSession` propaga el flag GM al `Player` tipado.
- El seam representado de GM bypassa los gates posteriores a dificultad (`MAX_PLAYERS` y lock compatibility) como `Map::PlayerCannotEnter` hace tras `GetDownscaledMapDifficultyData`.
- Los achievements completados del personaje actual se cargan durante login desde `character_achievement` con `CHAR_SEL_CHARACTER_ACHIEVEMENTS`, poblando el set representado que usa el gate `completed_achievement` de `Player::Satisfy`, igual que C++ carga `PLAYER_LOGIN_QUERY_LOAD_ACHIEVEMENTS` antes del resto de criterios de jugador.
- El gate `completed_achievement` de `Player::Satisfy` ya resuelve el líder de grupo conectado mediante `PlayerRegistry` antes de rechazar: si el líder remoto publicó el achievement, la entrada se acepta; si el líder no está conectado/representado, se conserva el fallo conservador equivalente al `ObjectAccessor::FindPlayer` nulo de C++.
- La rama `quest_failed_text` de `Player::Satisfy` ya envía un `SMSG_CHAT` de sistema con el texto literal de `access_requirement.quest_failed_text` antes del `SMSG_TRANSFER_ABORTED`, respetando la prioridad C++ sobre la rama `MapDifficulty::Message`.
- Las ramas `missingItem` y `LevelMin` de `Player::Satisfy` ya envían `SMSG_PRINT_NOTIFICATION` usando `trinity_string` cargado desde world DB (`LANG_LEVEL_MINREQUIRED[_AND_ITEM]`) y el nombre localizado de `ItemSearchName.db2`, antes del `SMSG_TRANSFER_ABORTED` genérico.

**Límites honestos:**
- `instance_encounter_in_progress_like_cpp` es un seam representado en `ManagedMap`; falta conectarlo al `InstanceScriptBase::is_encounter_in_progress_like_cpp` real y a la lifecycle `InstanceMap::GetInstanceScript()`.
- El requisito de achievement de líder ajeno queda cubierto para líderes conectados con snapshot representado; líderes offline/no representados, criterios dinámicos y `AchievementMgr` completo siguen pendientes.
- Siguen pendientes la conexión real de `InstanceScript`, broader portal/teleport call-sites, y validación live con cliente/bot.

## 15. Temporary pet resummon after teleport (2026-06-16)

🟡 parcialmente mitigado. Auditado contra `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`,
`Pet.cpp` y `Handlers/MovementHandler.cpp`.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j100`):**
- `WorldSession` representa una `PetStable` session-local hasta que `Pet::LoadPetFromDB` real quede conectado a `character_pet`.
- `Player::ResummonPetTemporaryUnSummonedIfAny` queda representado para el caso de teleport: retorna sin tocar estado si `m_temporaryUnsummonedPetNumber == 0`, si `IsPetNeedBeTemporaryUnsummoned()` sigue activo, o si ya existe pet activa.
- `IsPetNeedBeTemporaryUnsummoned` cubre los gates ya representados en Rust: no estar en mundo, no estar vivo y `MOVEMENTFLAG_FLYING`.
- En el intento de load, Rust usa `Pet::get_load_pet_info(stable, 0, pet_number, None)` y recrea una pet canónica mínima en el mapa actual con owner, entry, posición, display, nivel, health, mana, react state, specialization y `CharmInfo::pet_number`.
- Igual que C++, si el load representado falla después de pasar los gates previos, `m_temporaryUnsummonedPetNumber` se limpia a `0`; no se queda reintentando infinitamente.
- `HandleMoveWorldportAck` y `HandleMoveTeleportAck` llaman al helper antes de operaciones diferidas, igual que C++.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j101`):**
- El login carga `character_pet` con la statement C++ `CHAR_SEL_CHAR_PETS` ya portada (`id, entry, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization FROM character_pet WHERE owner = ?`) y reconstruye la `PetStable` representada desde DB.
- La conversión replica `Player::_LoadPetStable`: slots activos (`0..5`) a `active_pets`, slots de establo (`5..205`) a `stabled_pets`, `PET_SAVE_NOT_IN_SLOT` a `unslotted_pets`, descartando slots inválidos/deleted.
- Si `summonedPetNumber` de `CHAR_SEL_CHARACTER` apunta a una pet cargada, Rust marca `m_temporaryUnsummonedPetNumber` representado igual que C++ para que el resummon post-teleport pueda usar datos reales de `character_pet` en vez de una stable inyectada por tests.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j102`):**
- El login carga `pet_spell` para el `summonedPetNumber` representado usando la statement C++ `CHAR_SEL_PET_SPELL` (`SELECT spell, active FROM pet_spell WHERE guid = ?`).
- El resummon representado aplica esas filas al `Pet` canónico mínimo con la misma forma de C++ `_LoadSpells`: `addSpell(spell, ActiveStates(active), PETSPELL_UNCHANGED)`. En Rust queda mapeado a `Pet::add_spell(..., PetSpellState::Unchanged, PetSpellType::Normal)`, conservando autocast para `ActiveState::Enabled`.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j103`):**
- El login carga `character_pet_declinedname` para el `summonedPetNumber` representado usando la statement C++ `CHAR_SEL_PET_DECLINED_NAME` (`SELECT genitive, dative, accusative, instrumental, prepositional FROM character_pet_declinedname WHERE owner = ? AND id = ?`).
- `wow_entities::Pet` conserva ahora los cinco casos de declined names y el resummon representado los aplica solo cuando la stable row es `HUNTER_PET`, igual que el branch final de C++ `Pet::LoadPetFromDB`.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j104`):**
- `CharmInfoState` porta el formato de `CharmInfo::InitPetActionBar` / `LoadPetActionBar`: defaults attack/follow/stay, cuatro slots de spell, tres reacciones, parse de 20 tokens `type action` desde `character_pet.abdata`, y packing `MAKE_UNIT_ACTION_BUTTON(action, type)`.
- El resummon representado aplica `PetStableInfo.action_bar` al `CharmInfoState` del pet vivo junto al `pet_number`, cerrando la parte estructural de `Pet::LoadPetFromDB` que llama `m_charmInfo->LoadPetActionBar(petInfo->ActionBar)`.
- Al resummonear un `HUNTER_PET` persistido con `health == 0`, Rust marca el pet representado como `JustDied` y health `0`, igual que el branch C++ `Pet::LoadPetFromDB` que llama `setDeathState(JUST_DIED)`.
- El resummon representado copia `character_pet.exp` al `Pet` vivo y, cuando la tabla `player_xp_for_level` está instalada, calcula `PetNextLevelExperience` como el `InitStatsForLevel` C++ de hunter pets (`XPForLevel(petlevel) * PET_XP_FACTOR`).
- Con `SpellStore`/`SpellMiscStore` instalados, el resummon valida cada botón de spell siguiendo la intención de C++ `CharmInfo::LoadPetActionBar`: spell inexistente → `0/ACT_PASSIVE`; `SpellInfo::IsAutocastable == false` (`SPELL_ATTR0_PASSIVE` o `SPELL_ATTR1_NO_AUTOCAST_AI`) → mismo spell forzado a `ACT_PASSIVE`. Nota honesta: el legacy C++ de este fork empaqueta `ActiveStates << 23`, pero `UNIT_ACTION_BUTTON_TYPE` enmascara `0xFF000000` y pierde el bit bajo; Rust conserva el formato de packing pero recupera el tipo recién cargado para aplicar la validación que el bloque C++ pretende hacer.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j105`):**
- El login carga `pet_spell_cooldown` y `pet_spell_charges` para el `summonedPetNumber` representado con las statements C++ `CHAR_SEL_PET_SPELL_COOLDOWN` (`SELECT spell, time, categoryId, categoryEnd FROM pet_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()`) y `CHAR_SEL_PET_SPELL_CHARGES` (`SELECT categoryId, rechargeStart, rechargeEnd FROM pet_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd`).
- El resummon representado aplica esas filas al `SpellHistory` del pet vivo como `SpellHistory::LoadFromDB<Pet>`: cooldown por spell con `ItemId = 0`, category cooldown enlazada por `categoryId`, y cargas por categoría preservando el orden de DB.
- Si hay `SpellStore` instalado, Rust descarta cooldowns de spells inexistentes igual que `PersistenceHelper<Pet>::ReadCooldown`; sin store instalado, conserva la fila representada para no descartar datos sin autoridad de validación.
- `world-server` carga `SpellCategory.db2`, inyecta `SpellCategoryStore` en cada `WorldSession`, y Rust descarta cargas de categorías inexistentes igual que `PersistenceHelper<Pet>::ReadCharge` usa `sSpellCategoryStore.LookupEntry`.

**Cerrado en Rust (`#NEXT.RUNTIME.L3.031j106`):**
- El login carga `pet_aura` y `pet_aura_effect` para el `summonedPetNumber` representado con las statements C++ `CHAR_SEL_PET_AURA` y `CHAR_SEL_PET_AURA_EFFECT`.
- Igual que `Pet::_LoadAuras`, Rust normaliza caster GUID vacío al GUID vivo del pet recreado, filtra `effectIndex >= MAX_SPELL_EFFECTS`, y aplica la aura al `AuraSubsystem` del pet como `OwnedAuraRef`, `AppliedAuraRef`, `visible_auras` y `VisibleAuraApplicationLikeCpp` con amounts por efecto.
- Si hay `SpellStore` instalado, Rust descarta auras de spells inexistentes igual que `Pet::_LoadAuras` hace con `sSpellMgr->GetSpellInfo`.
- Con `SpellAuraOptionsStore` instalado, `pet_aura.remainCharges` se normaliza como C++ `Pet::_LoadAuras`: si `SpellInfo::ProcCharges` existe y la DB trae `0`, se reemplaza por `ProcCharges`; si el spell no tiene proc charges, se fuerza a `0`. El estado cargado queda preservado en `AuraSubsystem::loaded_aura_states_like_cpp` como metadata representada de `Aura::SetLoadedState`.
- Con `DifficultyStore` instalado, Rust replica el gate C++ `difficulty != DIFFICULTY_NONE && !sDifficultyStore.LookupEntry(difficulty)`: las auras de pet con difficulty no-cero desconocida se descartan.
- `Pet::get_load_pet_info_result_like_cpp` conserva ahora el resultado C++ `nullptr + PET_SAVE_AS_DELETED` cuando no hay selección posible; el wrapper legacy `get_load_pet_info` sigue devolviendo `None` para los call-sites representados existentes.
- `Pet::tick_focus_regen_timer` replica el rollover C++ de `Pet::Update`: conserva el sobrante cuando `diff` supera el timer y resetea a `PET_FOCUS_REGEN_INTERVAL` solo ante lag grande. `Pet::regenerate_focus_like_cpp` porta el cálculo base de `Creature::Regenerate(POWER_FOCUS)` (`24 * RATE_POWER_FOCUS`, multiplicador/aura flat y clamp al max power) como helper puro hasta que exista `Pet::Update` vivo.
- `Pet::give_pet_xp_like_cpp` y `Pet::give_pet_level_like_cpp` portan las decisiones C++ de `Pet::GivePetXP/GivePetLevel`: solo hunter pets vivos reciben XP, capean en `min(CONFIG_MAX_PLAYER_LEVEL, owner->GetLevel())`, arrastran XP restante entre level-ups, resetean XP al alcanzar el cap y devuelven señales representadas para `InitStatsForLevel`/`InitLevelupSpellsForLevel`.
- `Pet::synchronize_level_with_owner_like_cpp` porta `Pet::SynchronizeLevelWithOwner`: `SUMMON_PET` y `HUNTER_PET` llaman a `GivePetLevel(owner->GetLevel())`, mientras el branch default no muta el nivel.
- `Pet::generate_action_bar_data_like_cpp` y `Pet::fill_pet_info_like_cpp` portan la proyección pura de `Pet::GenerateActionBarData` / `Pet::FillPetInfo`: serializan los 10 botones como pares `type action` con espacio final y copian `PetNumber`, entry, display nativo, level, XP, react state forzado/opcional, nombre, `WasRenamed`, health, mana, timestamp, created spell, tipo y especialización al `PetStableInfo` representado.
- `Pet::add_to_world_like_cpp` / `Pet::remove_from_world_like_cpp` portan las fronteras locales C++ de `Pet::AddToWorld` y `Pet::RemoveFromWorld`: registro/desregistro de lookup de pet, ruta `Unit::{Add,Remove}FromWorld`, fases representadas de `AIM_Initialize`/ZoneScript y limpieza de flags transitorios de follow fuera del guard `IsInWorld`.
- `Pet::debug_info_with_guardian_like_cpp` porta la capa C++ de `Pet::GetDebugInfo`: compone el debug heredado de `Guardian` con `PetType` numérico y `PetNumber` de `CharmInfo`, sin sobre-representar aquí el debug completo de Guardian.
- `Pet::set_display_id_like_cpp`, `set_group_update_flag_like_cpp` y `reset_group_update_flag_like_cpp` portan las decisiones C++ de `Pet::SetDisplayId` y flags de grupo: `Guardian::SetDisplayId`, guard `isControlled()`, `GROUP_UPDATE_FLAG_PET_MODEL_ID`, gate `GetOwner()->GetGroup()` y señal representada de `GROUP_UPDATE_FLAG_PET`.
- `Pet::have_in_diet_like_cpp` porta `Pet::HaveInDiet`: rechaza `FoodType == 0`, template/family ausentes, calcula `1 << (FoodType - 1)` y cruza contra `CreatureFamilyEntry::PetFoodMask`.
- `Pet::native_object_scale_like_cpp` porta la decisión C++ de `Pet::GetNativeObjectScale`: solo hunter pets con `CreatureFamilyEntry::MinScale > 0.0` usan la escala familiar, respetan los límites min/max por nivel y conservan la fórmula legacy intermedia; el resto cae a `Guardian::GetNativeObjectScale`.
- `Pet::is_permanent_pet_for_like_cpp` porta el switch C++ de `Pet::IsPermanentPetFor`: hunter pets siempre son permanentes; summon pets solo lo son para warlock+demon, death knight+undead o mage+elemental.
- `Pet::learn_specialization_spells_plan_like_cpp`, `remove_specialization_spells_plan_like_cpp` y `set_specialization_like_cpp` portan la decisión representada de `LearnSpecializationSpells`/`RemoveSpecializationSpells`/`SetSpecialization`: filtran spells inexistentes o sobre el nivel del pet, remueven specs normales y override en orden `0..MAX_SPECIALIZATIONS`, respetan el early return por spec igual, limpian a `0` si la spec no existe y preparan las señales de `CleanupActionBar`, `PetSpellInitialize` y `SMSG_SET_PET_SPECIALIZATION` solo en la rama válida.
- `Pet::add_spell` conserva ahora la máquina mínima C++ de `Pet::addSpell` sobre estados ya representados: rechaza spells existentes no removidos, normaliza el caso load `PETSPELL_UNCHANGED` sin reportar learn, re-agrega `PETSPELL_REMOVED` como `PETSPELL_CHANGED`, y mantiene autocast al cargar `ACT_ENABLED/ACT_DISABLED`.
- `Pet::toggle_autocast_like_cpp` porta `Pet::ToggleAutocast`: no-op para spells no autocastables o ausentes, solo muta `active` cuando el spell entra/sale de `m_autospells`, y marca `PETSPELL_CHANGED` únicamente si el spell no era `PETSPELL_NEW`.
- `unit_action_button_type_like_cpp` conserva el bit bajo del tipo con máscara `0xFF800000`, alineado con TrinityCore archivado; el legacy local usa `0xFF000000` y pierde ese bit (`ACT_ENABLED` se leería como `0xC0`), lo que impediría reconocer action-bar spells en `IsActionBarForSpell`.
- `Pet::cleanup_action_bar_like_cpp` porta `Pet::CleanupActionBar`: recorre los 10 botones, solo procesa entradas de spell (`ACT_DISABLED`, `ACT_ENABLED`, `ACT_PASSIVE`) con `action != 0`, limpia a `0/ACT_PASSIVE` si el pet no conserva el spell y reactiva autocast para slots `ACT_ENABLED` solo cuando el equivalente de `sSpellMgr->GetSpellInfo` confirma el spell.
- `Pet::learn_spell_like_cpp` / `learn_spells_like_cpp` portan la capa C++ de `Pet::learnSpell(s)`: delegan en `addSpell`, solo agregan al paquete los spells aprendidos, el single envía `PetLearnedSpells` y `PetSpellInitialize` solo si hubo learn y `!m_loading`, y el batch conserva la rareza C++ de enviar el paquete tras el loop siempre que `!m_loading`, incluso vacío.
- `Pet::learn_spell_high_rank_like_cpp` porta `Pet::learnSpellHighRank`: llama al seam representado de `learnSpell/addSpell` para el spell actual y sigue `GetNextSpellInChain` hasta `0`, incluso si el rank actual ya estaba aprendido y no produjo cambio.
- `Pet::learn_pet_passives_like_cpp` porta `Pet::LearnPetPassives`: respeta los early returns por `CreatureTemplate` o `CreatureFamilyEntry` ausente, recorre `sPetFamilySpellsStore[family]` como set ordenado y añade cada spell como `PETSPELL_FAMILY`.
- `Pet::init_pet_create_spells_like_cpp` porta el C++ actual de `Pet::InitPetCreateSpells`: `InitPetActionBar`, limpiar `m_spells`, `LearnPetPassives`, señal de `InitLevelupSpellsForLevel` y `CastPetAuras(false)`. Esto corrige el drift de `docs/migration/pets.md`, que todavía describía una ruta antigua de `creature_template_addon.spell1..8`.
- `Pet::learn_pet_talent_like_cpp` conserva el estado real del C++ local de `Pet::LearnPetTalent`: actualmente solo emite el log debug con `talentId` y no muta spells, autocast, puntos de talento ni paquetes.
- `PetAuraLikeCpp`, `cast_pet_aura_like_cpp`, `cast_pet_auras_like_cpp` e `is_pet_aura_like_cpp` portan la decisión representada de `Pet::CastPetAuras`/`CastPetAura`/`IsPetAura`: solo pets permanentes, `removeOnChangePet` elimina el aura del owner y del pet cuando `current=false`, `petEntry=0` actúa como wildcard, y Demonic Knowledge (`35696`) calcula `SPELLVALUE_BASE_POINT0` con `CalculatePct(damage, stamina + intellect)`. Sigue pendiente el loader vivo de `spell_pet_auras` y la ejecución real de `CastSpell`.
- `SpellPetAuraStoreLikeCpp` vive en `wow-data` y junto a `WorldStatements::SEL_SPELL_PET_AURAS` representa `SpellMgr::LoadSpellPetAuras`: query exacta `SELECT spell, effectId, pet, aura FROM spell_pet_auras`, key C++ `(spell << 8) + eff`, validación de fuente dummy/apply-aura-dummy, aura destino existente, wildcard `petEntry=0`, y semántica de duplicado donde una key existente solo hace `AddAura` sin revalidar. Sigue pendiente cablearlo al `SpellMgr` vivo/startup con el `SpellInfo` canónico, preservando `SpellEffectInfo::CalcValue()` para `damage`.
- `SpellThreatStoreLikeCpp` y `WorldStatements::SEL_SPELL_THREATS` representan `SpellMgr::LoadSpellThreats`: query exacta `SELECT entry, flatMod, pctMod, apPctMod FROM spell_threat`, skip de spells inexistentes, overwrite por duplicado, y accessor con fallback a `GetFirstSpellInChain`. Sigue pendiente cargarlo en startup y consumirlo en `ThreatManager`/spell runtime.
- `SpellEnchantProcStoreLikeCpp` y `WorldStatements::SEL_SPELL_ENCHANT_PROC_DATA` representan `SpellMgr::LoadSpellEnchantProcData`: query exacta `SELECT EnchantID, Chance, ProcsPerMinute, HitMask, AttributesMask FROM spell_enchant_proc_data`, skip de encantamientos inexistentes en `SpellItemEnchantment.db2`, overwrite por duplicado y lookup directo por enchant id. Sigue pendiente cargarlo en startup y consumirlo en el runtime de procs de encantamiento.
- `SpellLinkedStoreLikeCpp` y `WorldStatements::SEL_SPELL_LINKED` representan `SpellMgr::LoadSpellLinked`: query exacta `SELECT spell_trigger, spell_effect, type FROM spell_linked_spell`, validación de trigger/effect existentes por `abs(id)`, conservación del signo de `spell_effect`, coerción C++ de trigger negativo a REMOVE, skip de tipo inválido/self-loop no-AURA, orden de `push_back` y warning non-fatal de same-base-point. Sigue pendiente cargarlo en startup y consumirlo en cast/hit/aura/remove.
- `SpellTotemModelStoreLikeCpp` y `WorldStatements::SEL_SPELL_TOTEM_MODEL` representan `SpellMgr::LoadSpellTotemModel`: query exacta `SELECT SpellID, RaceID, DisplayID from spell_totem_model`, validación de spell/race/display existentes, overwrite por duplicado y `GetModelForTotem` con fallback `0`. Sigue pendiente cargarlo en startup y consumirlo al crear tótems.
- `Pet::remove_spell_like_cpp` porta la máquina C++ de `Pet::removeSpell`: missing/removed no-op, `PETSPELL_NEW` se borra, el resto marca `PETSPELL_REMOVED`, limpia autocast, emite señal de `RemoveAurasDueToSpell`, aprende el rank previo si existe y solo limpia el primer slot de action bar de la misma cadena cuando `clear_ab` sigue activo sin rank previo.
- `Pet::unlearn_spell_like_cpp` / `unlearn_spells_like_cpp` portan la capa C++ de `Pet::unlearnSpell(s)`: delegan en `removeSpell`, solo agregan al paquete los spells removidos, el single envía `PetUnlearnedSpells` solo si hubo remove y `!m_loading`, y el batch conserva la rareza C++ de enviar el paquete tras el loop siempre que `!m_loading`, incluso si queda vacío.
- `Pet::remove_plan_like_cpp` porta la frontera C++ de `Pet::Remove`: no duplica `Player::RemovePet`, solo representa la delegación `GetOwner()->RemovePet(this, mode, returnreagent)` con owner, pet, modo de guardado y flag de retorno de reagente.
- `Pet::prepare_save_pet_to_db_like_cpp` porta como plan puro la decisión y el orden transaccional C++ de `Pet::SavePetToDB`: guardas por entry/control/owner-player, skip de hunter pet si hay temporary-unsummoned distinto, conversión warlock/summon a `PET_SAVE_NOT_IN_SLOT`, `_SaveAuras` antes de cualquier limpieza, remapeo `PET_SAVE_AS_CURRENT` al active slot, limpieza de auras para saves no-activos/delete, `_SaveSpells`, `SpellHistory::SaveToDB<Pet>`, commit, y luego ruta save-vs-delete. Conserva explícitamente la rareza C++ de `CHAR_INS_PET.slot`: el insert usa `GetCurrentActivePetIndex().value_or(PET_SAVE_NOT_IN_SLOT)`, no el `mode` ya calculado.
- `Pet::delete_from_db_plan_like_cpp` porta el orden C++ de `Pet::DeleteFromDB`: en una transacción borra `character_pet`, `character_pet_declinedname`, `pet_aura_effect`, `pet_aura`, `pet_spell`, `pet_spell_cooldown` y `pet_spell_charges` por `petNumber`.
- `Pet::save_spells_plan_like_cpp` porta la máquina de estados C++ `_SaveSpells`: `PETSPELL_REMOVED` genera `CHAR_DEL_PET_SPELL_BY_SPELL` y borra el spell del mapa, `PETSPELL_CHANGED` genera delete+insert y vuelve a `UNCHANGED`, `PETSPELL_NEW` genera insert y vuelve a `UNCHANGED`, `PETSPELL_UNCHANGED` no emite nada, y `PETSPELL_FAMILY` se salta antes del switch como en C++.
- `Pet::save_auras_plan_like_cpp` porta el orden de statements de `_SaveAuras`: borra primero `pet_aura_effect` y `pet_aura`, filtra auras no guardables o `IsPetAura`, limpia el caster GUID cuando el caster es el pet vivo, e inserta la fila `pet_aura` seguida de sus filas `pet_aura_effect`.
- `SpellHistory::save_pet_spell_history_plan_like_cpp` porta la fase C++ `GetSpellHistory()->SaveToDB<Pet>`: borra primero `pet_spell_cooldown`, inserta cooldowns no `OnHold` con tiempos `Clock::to_time_t`, borra `pet_spell_charges`, e inserta cada recharge guardando el orden de la cola por categoría.
- `Pet::update_alive_owner_link_like_cpp` porta el branch `ALIVE` de `Pet::Update` que ejecuta `Remove(PET_SAVE_NOT_IN_SLOT, true)` cuando el pet pierde owner/distancia o `owner->GetPetGUID()`, y `Remove(PET_SAVE_NOT_IN_SLOT)` cuando un pet controlado ya no coincide con el GUID del owner; el caso hunter-unlinked conserva la señal del `ASSERT` C++.
- `Pet::update_duration_like_cpp` porta el branch temporal de `Pet::Update`: pets removidas/cargando no avanzan, `m_duration > diff` descuenta tiempo, y al expirar devuelve el `PetSaveMode` C++ (`SUMMON_PET` → `PET_SAVE_NOT_IN_SLOT`, otros tipos → `PET_SAVE_AS_DELETED`) para que el futuro lifecycle llame a `Remove(...)`.
- `Pet::update_corpse_like_cpp` porta la decisión de `Pet::Update` para `CORPSE`: hunter pets mantienen cadáver hasta `m_corpseRemoveTime`; al vencimiento, o para cualquier pet no hunter en corpse, el helper devuelve `PET_SAVE_NOT_IN_SLOT` como el `Remove(...)` C++ pendiente de cablear.
- `Pet::set_death_state_like_cpp` envuelve el override C++ `Pet::setDeathState`: tras delegar en `Creature::setDeathState`, hunter pets en `CORPSE` limpian dynamic flags y `UNIT_FLAG_SKINNABLE`; al quedar `ALIVE` devuelve una señal representada para el futuro `CastPetAuras(true)`.
- `Creature::set_death_state_runtime(JUST_RESPAWNED)` distingue ya el branch C++ `IsPet()`: pets usan `SetFullHealth()` y no ejecutan el bloque `!IsPet()` que recarga flags/dynamic/melee desde template; criaturas no-pet conservan `SetSpawnHealth()` y los resets representados existentes.
- `load_represented_pet_aura_rows_with_timediff_like_cpp` aplica el ajuste offline C++ para auras con `SPELL_ATTR4_AURA_EXPIRES_OFFLINE`: descarta las ya expiradas (`remainTime / 1000 <= timediff`) y resta `timediff * 1000` al resto.

**Límites honestos:**
- No es todavía `Pet::LoadPetFromDB` / `Pet::SavePetToDB` real: aunque `character_pet`, `pet_spell`, action bar, declined names de hunter pet, cooldowns/charges, auras persistidas, el resultado `PET_SAVE_AS_DELETED` del selector, la regeneración base de focus, las decisiones de XP/level, `FillPetInfo`/`GenerateActionBarData`, el plan de decisión de `SavePetToDB`, los planes de `_SaveSpells`/`_SaveAuras`, el plan de `SpellHistory::SaveToDB<Pet>`, owner-link/duración/corpse, el override representado de death-state y el ajuste offline `SPELL_ATTR4_AURA_EXPIRES_OFFLINE` ya alimentan el resummon representado, faltan el ajuste offline para auras negativas (`SpellInfo::IsPositive()` no está modelado todavía), `Aura::TryCreate/CanBeSaved`, happiness exacto, conexión de focus/XP/owner-link/duration/corpse/death-state con el futuro `Pet::Update` vivo, ejecución real de `CastPetAuras(true)`, aplicación completa de save mode al lifecycle, ejecución real de transacciones `Pet::SavePetToDB`/`SpellHistory::SaveToDB<Pet>` y persistencia completa de `character_pet`.
- El gate de vuelo avanzado C++ `MOVEMENTFLAG3_ADV_FLYING` no está cubierto porque no hay campo equivalente representado en `WorldSession`; añadirlo cuando se porte `MovementInfo::flags2/flags3` completo.
- Falta validar live con cliente/bot que la pet reaparece visualmente tras worldport/near teleport y que los paquetes de create/update son suficientes.
