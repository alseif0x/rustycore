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
- Account-wide instance-enter rate limit (`account_instance_times`).
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
  This is still not full `Map::PlayerCannotEnter`: access requirements, GM
  bypass, raid-group requirement, farm-limit, max-player, in-combat, and the
  broader portal/teleport call-sites remain pending.

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
- `Player::CheckInstanceCount` ya está representado en `WorldSession` con `_instanceResetTimes` por cuenta, `AccountInstancesPerHour`, limpieza de expirados, reentrada a instancia existente y abort `TRANSFER_ABORT_TOO_MANY_INSTANCES = 4` en la posición C++ de `Map::PlayerCannotEnter`. `Map.db2::Flags[1]` se carga como `flags2` para respetar `MapFlags2::IgnoreInstanceFarmLimit`, y una entrada aceptada registra `AddInstanceEnterTime(instanceId, now + HOUR)` como `InstanceMap::AddPlayerToMap`.
- El seam representado de GM bypassa los gates posteriores a dificultad (`MAX_PLAYERS` y lock compatibility) como `Map::PlayerCannotEnter` hace tras `GetDownscaledMapDifficultyData`.

**Límites honestos:**
- `player_count` es el conteo representado del `ManagedMap`; todavía no hay contabilidad separada equivalente a `GetPlayersCountExceptGMs()` para GMs ya presentes dentro del mapa.
- `instance_encounter_in_progress_like_cpp` es un seam representado en `ManagedMap`; falta conectarlo al `InstanceScriptBase::is_encounter_in_progress_like_cpp` real y a la lifecycle `InstanceMap::GetInstanceScript()`.
- El gate de access requirements aún no envía las notificaciones/sysmessages exactas de `Player::Satisfy`, y el requisito de achievement de líder ajeno queda conservador hasta portar carga de `character_achievement` y resolución de líder viva.
- Siguen pendientes la carga/guardado DB real de `account_instance_times`, exactas notificaciones de rechazo de access requirements, y validación live con cliente/bot.
