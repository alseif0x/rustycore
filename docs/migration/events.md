# Migration: GameEvents (holiday & world event scheduler)

> **C++ canonical path:** `src/server/game/Events/` (`GameEventMgr` scheduler + `GameEventSender` trigger pipeline). Distinct from `src/server/scripts/Events/` (the *content* scripts that hook into individual holidays — covered by `scripts.md`).
> **Rust target crate(s):** No dedicated crate yet. Recommend a new `crates/wow-gameevents/` (or fold into `wow-world` if minimal). Cross-cutting: spawn/despawn drives `MapManager`, NPC flags drive `wow-world`, vendor swaps drive `wow-database`'s prepared-statement registry.
> **Layer:** L7 (uses Map/Spawn/Pool L4–L6, Vendor/Quest/WorldState L6, Achievement L7; depended on by Smart-Scripts L7, Quests L6, Vendors L6, Pools L6, the holiday scripts in `scripts/Events/`).
> **Status:** ❌ not started — `GameEventMgr` does not exist in any form in `crates/`. **No** holiday scheduling, **no** Hallow's End / Brewfest / Winter Veil / Lunar Festival cycle, **no** Darkmoon Faire, **no** quest seasonal-availability gating, **no** `IsHolidayActive()` lookup, **no** event-driven creature/gameobject swap (model/equipment/NPC flag changes during holidays), **no** event-driven vendor inventory swap, **no** event-driven world state updates. The `GameEvents::Trigger` pipeline (which fires `event_scripts` on spell `EFFECT_SEND_EVENT`) is also missing — that one cross-cuts with `wow-spell`'s effect dispatcher.
> **Audited vs C++:** ✅ audited 2026-05-01 (status confirmed ❌ — only debug opcodes + 2 DELETE statements wired)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`GameEventMgr` is TrinityCore's wall-clock-driven event scheduler. It loads ~150 game-event rows from `world.game_event`, decides which are currently active based on (`start_time`, `end_time`, `occurence`, `length`, `holiday_id`), then **applies** active events by mutating the live world: spawning event-only creatures and gameobjects (`game_event_creature`, `game_event_gameobject`), swapping creature models/equipment (`game_event_model_equip`), enabling seasonal NPC flags (`game_event_npcflag`), patching vendor inventories (`game_event_npc_vendor`), gating creature/gameobject quests (`game_event_creature_quest`, `game_event_gameobject_quest`, `game_event_seasonal_questrelation`, `game_event_quest_condition`), publishing event-condition world states (`game_event_condition`), activating event-driven pools (`game_event_pool`), and toggling `SMART_EVENT_GAME_EVENT_START/END` SAI hooks. **Unapply** does the inverse on event end. The companion `GameEventSender` namespace is a *different* concept: it fires runtime "events" (i.e. spell-triggered or gameobject-triggered events identified by `event_scripts.id` rows) and drives `ZoneScript::ProcessEvent`, `ScriptMgr::OnEventTrigger`, `GameObjectAI::EventInform`, and player criteria updates. Both share the word "event" but otherwise are unrelated systems.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Events/GameEventMgr.cpp` | 1782 | `prefix` |
| `game/Events/GameEventMgr.h` | 182 | `prefix` |
| `game/Events/GameEventSender.cpp` | 72 | `prefix` |
| `game/Events/GameEventSender.h` | 34 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Events/GameEventMgr.h` | 182 | `GameEventState` enum, `GameEventFinishCondition`, `GameEventQuestToEventConditionNum`, `GameEventConditionMap`, `GameEventData` (one row per `game_event` row), `ModelEquip`, `GameEventMgr` singleton, public helpers `IsHolidayActive(HolidayIds)` and `IsEventActive(uint16)`. |
| `src/server/game/Events/GameEventMgr.cpp` | 1782 | The whole scheduler: `LoadFromDB` (16 sub-queries), `Update` (next-fire computation + state transitions), `StartEvent` / `StopEvent` (incl. `GAMEEVENT_WORLD_*` 5-state machine for "world events" with completion conditions), `ApplyNewEvent` / `UnApplyEvent` (the 7 mutators: `GameEventSpawn`, `ChangeEquipOrModel`, `UpdateEventQuests`, `UpdateWorldStates`, `UpdateEventNPCFlags`, `UpdateEventNPCVendor`, `RunSmartAIScripts`), `HandleQuestComplete` (advance world-event progress when a quest completes), `CheckOneGameEventConditions`, `SaveWorldEventStateToDB`, `SendWorldStateUpdate`, `StartArenaSeason`, `GetNPCFlag` (event-aware NPC flag override). |
| `src/server/game/Events/GameEventSender.h` | 34 | The `GameEvents` namespace: `Trigger(eventId, source, target)`, `TriggerForPlayer`, `TriggerForMap`. |
| `src/server/game/Events/GameEventSender.cpp` | 72 | Implementation: dispatch to `ZoneScript::ProcessEvent`, `sScriptMgr->OnEventTrigger`, `GameObjectAI::EventInform`, criteria updates, `Map::ScriptsStart(sEventScripts, …)`. |

Out-of-tree consumers:
- `src/server/scripts/Events/*.cpp` — the 12 content scripts that hook into specific holidays (brewfest, hallows_end, etc.). See `scripts.md`.
- `src/server/game/AI/SmartScripts/SmartScriptMgr.cpp` — `SMART_EVENT_GAME_EVENT_START` / `_END` triggers.
- `src/server/game/Quests/QuestDef.cpp` and `Player::CanTakeQuest` — seasonal quest gating via `game_event_seasonal_questrelation`.
- `src/server/game/Entities/Creature.cpp` — vendor item filtering via `IsActiveEvent` checks.
- `src/server/game/Spells/SpellEffects.cpp` — `SPELL_EFFECT_SEND_EVENT` calls `GameEvents::Trigger`.
- `src/server/game/Entities/GameObject.cpp` — `GAMEOBJECT_TYPE_GOOBER` `eventID` calls `GameEvents::Trigger`.
- `src/server/game/Misc/Conditions/ConditionMgr.cpp` — `CONDITION_ACTIVE_EVENT` evaluates via `IsEventActive`.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `GameEventState` | enum (6) | `GAMEEVENT_NORMAL` (standard time-windowed events), `GAMEEVENT_WORLD_INACTIVE` (not yet started), `GAMEEVENT_WORLD_CONDITIONS` (currently in condition-matching phase), `GAMEEVENT_WORLD_NEXTPHASE` (conditions met, in `length` countdown), `GAMEEVENT_WORLD_FINISHED` (children started, this one unapplies), `GAMEEVENT_INTERNAL` (never auto-handled in `Update`). |
| `GameEventFinishCondition` | struct | One condition for a "world event": `reqNum` (required), `done` (current), `max_world_state` (worldstate id for max), `done_world_state` (worldstate id for done). |
| `GameEventConditionMap` | typedef | `map<uint32, GameEventFinishCondition>` — keyed by condition id. |
| `GameEventQuestToEventConditionNum` | struct | `event_id`, `condition`, `num` — quest-completion contributes `num` to `condition` of `event_id`. |
| `GameEventData` | struct | One `game_event` row: `start`, `end`, `nextstart`, `occurence`, `length`, `holiday_id`, `holidayStage`, `state`, `conditions`, `prerequisite_events`, `description`, `announce`. `isValid()` = `length > 0 \|\| state > NORMAL`. |
| `ModelEquip` | struct | `modelid`, `modelid_prev`, `equipment_id`, `equipement_id_prev` — for swap during `GAME_EVENT_MODEL_EQUIP`. |
| `GameEventMgr` | singleton class | The scheduler. ~30 private helpers + 12 public methods. |
| `GameEvents` | namespace | The runtime trigger pipeline (`Trigger`, `TriggerForPlayer`, `TriggerForMap`). |
| `HolidayIds` | enum (~50, `HOLIDAY_NONE = 0`, defined in `SharedDefines.h`) | Indexes into Holidays.dbc — joins game_event rows to client-side holiday metadata. |
| `mGameEventCreatureGuids` | `vector<list<LowGuid>>` | Per-event-id, list of creature low GUIDs to spawn/despawn on activate/deactivate. |
| `mGameEventGameobjectGuids` | `vector<list<LowGuid>>` | Same for gameobjects. |
| `mGameEventCreatureQuests` / `mGameEventGameObjectQuests` | `vector<list<QuestRelation>>` | Per-event-id list of `(creatureOrGoEntry, questId)` to enable. |
| `mGameEventVendors` | `vector<unordered_map<entry, vector<VendorItem>>>` | Per-event-id additional vendor inventory per creature entry. |
| `mGameEventModelEquip` | `vector<list<pair<LowGuid, ModelEquip>>>` | Per-event-id model/equipment swaps. |
| `mGameEventNPCFlags` | `vector<list<pair<LowGuid, uint64>>>` | Per-event-id NPC flag overlays. |
| `mGameEventPoolIds` | `vector<list<uint32>>` | Per-event-id list of `pool_template.entry`s to activate. |
| `mQuestToEventConditions` | `map<questId, GameEventQuestToEventConditionNum>` | Reverse index for `HandleQuestComplete`. |
| `m_ActiveEvents` | `set<uint16>` | Hot lookup: which event ids are currently active. |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `GameEventMgr::LoadFromDB()` | One-time load from 16 world+character DB tables (see §6); validates each row, builds in-memory indexes, restores persisted world-event state from `game_event_save` + `game_event_condition_save` | `WorldDatabase.Query` x14, `CharacterDatabase.Query` x2, `sObjectMgr->GetCreatureData`, `sObjectMgr->GetGameObjectData`, `sPoolMgr` |
| `GameEventMgr::StartSystem()` | First-tick bootstrap; sets `isSystemInit=true`, calls `Update()` | `Update` |
| `GameEventMgr::Update()` | Walk every event id; `CheckOneGameEvent` decides if it should be active *now*; reconcile by `StartEvent`/`StopEvent`; for `GAMEEVENT_WORLD_CONDITIONS` events, also evaluate completion conditions; return ms-until-next-check (`min(NextCheck) * 1000`) | `CheckOneGameEvent`, `StartEvent`, `StopEvent`, `CheckOneGameEventConditions`, `NextCheck`, `sWorld->ForceGameEventUpdate` |
| `GameEventMgr::CheckOneGameEvent(uint16 entry) const` | Decide active or not as of `now`: `start ≤ now ≤ end` AND `(now - start) % occurence < length`; world events use state machine instead | std `time(nullptr)`, struct math |
| `GameEventMgr::NextCheck(uint16 entry) const` | Compute seconds until this event's next state transition | calendar arithmetic |
| `GameEventMgr::StartEvent(uint16 event_id, bool overwrite=false)` | Activate; for normal events runs `ApplyNewEvent`; for world events steps the state machine, optionally evaluating conditions | `ApplyNewEvent`, `CheckOneGameEventConditions`, `SaveWorldEventStateToDB`, `sWorld->ForceGameEventUpdate` |
| `GameEventMgr::StopEvent(uint16 event_id, bool overwrite=false)` | Deactivate; `UnApplyEvent` + state cleanup | `UnApplyEvent`, save to DB |
| `GameEventMgr::ApplyNewEvent(uint16)` | The 7-step world mutation: `GameEventSpawn` (creatures + GOs + pools), `ChangeEquipOrModel(true)`, `UpdateEventQuests(true)`, `UpdateWorldStates(true)`, `UpdateEventNPCFlags`, `UpdateEventNPCVendor(true)`, `RunSmartAIScripts(true)` | All seven mutator helpers |
| `GameEventMgr::UnApplyEvent(uint16)` | Inverse: `GameEventUnspawn`, `ChangeEquipOrModel(false)`, `UpdateEventQuests(false)`, `UpdateWorldStates(false)`, `UpdateEventNPCFlags`, `UpdateEventNPCVendor(false)`, `RunSmartAIScripts(false)` | as above |
| `GameEventMgr::GameEventSpawn(int16 event_id)` | Spawn every guid in `mGameEventCreatureGuids[event_id]` and `mGameEventGameobjectGuids[event_id]`; activate event pools | `Map::AddToMap` per spawned object, `sPoolMgr->SpawnPool` |
| `GameEventMgr::GameEventUnspawn(int16 event_id)` | Inverse — remove from map, despawn pools | `Map::RemoveFromMap`, `sPoolMgr->DespawnPool` |
| `GameEventMgr::ChangeEquipOrModel(int16 event_id, bool activate)` | Per affected creature low-guid, set/restore display-id and equipment slot | `Creature::SetDisplayId`, `Creature::LoadEquipment` |
| `GameEventMgr::UpdateEventQuests(uint16, bool activate)` | Toggle which (creature, quest) and (gameobject, quest) pairs are valid; toggle seasonal questrelations; advance quest conditions | `sObjectMgr->GetCreatureQuestRelations`, `Player::CanTakeQuest` indirect |
| `GameEventMgr::UpdateWorldStates(uint16, bool activate)` | Bump worldstate values used by event UI overlays (timers, progress bars) | `WorldStateMgr::SetValue` |
| `GameEventMgr::UpdateEventNPCFlags(uint16)` | For each affected creature guid currently spawned, OR/AND NPC flags (e.g. enables a vendor flag during winter veil) | `Creature::ReplaceAllNpcFlags` |
| `GameEventMgr::UpdateEventNPCVendor(uint16, bool activate)` | Add/remove items from `vendor_template`-derived vendor menus on the fly | `sObjectMgr->AddVendorItem` / `RemoveVendorItem` |
| `GameEventMgr::RunSmartAIScripts(uint16, bool activate)` | Fire `SMART_EVENT_GAME_EVENT_START` / `_END` on every relevant SmartAI | `Map::ScriptsStart` per active map |
| `GameEventMgr::HandleQuestComplete(uint32 quest_id)` | When a player turns in a quest, find any `(quest_id → event_id)` mapping; advance that event's `condition.done`; if conditions met, advance state to `NEXTPHASE` | `mQuestToEventConditions`, `CheckOneGameEventConditions`, `SaveWorldEventStateToDB`, `sWorld->ForceGameEventUpdate` |
| `GameEventMgr::GetNPCFlag(Creature*)` | Compute the effective NPC flag mask for a creature taking active events into account | `mGameEventNPCFlags` map |
| `GameEventMgr::StartArenaSeason()` | Start the season-N event from `game_event_arena_seasons` (where `season = sWorld->getIntConfig(CONFIG_ARENA_SEASON_ID)`) | `WorldDatabase.PQuery` |
| `IsHolidayActive(HolidayIds id)` (free fn) | Any active event with `holiday_id == id`? | scan `m_ActiveEvents` |
| `IsEventActive(uint16 eventId)` (free fn) | Quick `m_ActiveEvents` membership | set lookup |
| `GameEvents::Trigger(eventId, source, target)` | Runtime "send event": fire `ZoneScript::ProcessEvent`, `ScriptMgr::OnEventTrigger`, `GameObjectAI::EventInform`; recurse into `TriggerForPlayer` if source is a player and `TriggerForMap` for the map's `event_scripts` | `ZoneScript::ProcessEvent`, `sScriptMgr->OnEventTrigger`, `GameObjectAI::EventInform`, `Map::ScriptsStart(sEventScripts, …)` |
| `GameEvents::TriggerForPlayer(eventId, player)` | Player-side criteria updates (`PlayerTriggerGameEvent`, `AnyoneTriggerGameEventScenario`) and instance-criteria fail/start | `Player::FailCriteria`, `Player::StartCriteria`, `Player::UpdateCriteria` |
| `GameEvents::TriggerForMap(eventId, map, source, target)` | Run the `event_scripts` table for this map | `Map::ScriptsStart` |

---

## 5. Module dependencies

**Depends on:**
- `Map` / `MapManager` — every spawn/despawn ends in `Map::AddToMap` / `Map::RemoveFromMap`. Currently the new `MapManager` in `wow-world` (see `AGENTS.md`) is the migration target for this side.
- `ObjectMgr` — for creature/gameobject template lookup, vendor item registration, and quest relation tables.
- `PoolMgr` — events activate pools by id (`mGameEventPoolIds`).
- `WorldStateMgr` — events drive worldstate updates for client-visible progress bars.
- `SmartAI` — events fire `SMART_EVENT_GAME_EVENT_START/END`.
- `ConditionMgr` — `CONDITION_ACTIVE_EVENT` calls `IsEventActive`.
- `ScriptMgr` — `GameEvents::Trigger` calls `OnEventTrigger`.
- `Holidays.dbc` — `holiday_id` ↔ client-side holiday metadata.
- `WorldDatabase` (loads), `CharacterDatabase` (persists world-event state).

**Depended on by:**
- `src/server/scripts/Events/*.cpp` — the 12 holiday content scripts call `IsHolidayActive` / `IsEventActive` extensively.
- `Player::CanTakeQuest` — seasonal quest availability.
- `Creature` (NPC flags, vendor inventory, model/equipment).
- `SpellEffects::EffectSendEvent` (`GameEvents::Trigger` from spells).
- `GameObject::SetGoState` / `GameObject::Use` (`GameEvents::Trigger` for goober events).
- `ConditionMgr::isMeetingCondition(CONDITION_ACTIVE_EVENT, …)`.
- `cs_event.cpp` GM commands.
- Achievement criteria types `PlayerTriggerGameEvent`, `AnyoneTriggerGameEventScenario`.

---

## 6. SQL / DB queries (if any)

`GameEventMgr::LoadFromDB` is one of the chattier loaders in TrinityCore. All queries are direct (not prepared except for the persistence side):

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT eventEntry, UNIX_TIMESTAMP(start_time), UNIX_TIMESTAMP(end_time), occurence, length, holiday, holidayStage, description, world_event, announce FROM game_event` | Master event table | world |
| `SELECT eventEntry, prerequisite_event FROM game_event_prerequisite` | DAG of which events block which | world |
| `SELECT guid, eventEntry FROM game_event_creature` | Event-only creature spawns | world |
| `SELECT guid, eventEntry FROM game_event_gameobject` | Event-only gameobject spawns | world |
| `SELECT creature.guid, creature.id, game_event_model_equip.eventEntry, game_event_model_equip.modelid, game_event_model_equip.equipment_id FROM creature JOIN game_event_model_equip` | Model/equipment swap rules | world |
| `SELECT id, quest, eventEntry FROM game_event_creature_quest` | Event-gated creature quest givers | world |
| `SELECT id, quest, eventEntry FROM game_event_gameobject_quest` | Event-gated gameobject quest givers | world |
| `SELECT quest, eventEntry, condition_id, num FROM game_event_quest_condition` | Quest → event-condition contribution map | world |
| `SELECT eventEntry, condition_id, req_num, max_world_state_field, done_world_state_field FROM game_event_condition` | Per-event completion conditions (for "world events") | world |
| `SELECT guid, eventEntry, npcflag FROM game_event_npcflag` | Event-driven NPC flag overlays | world |
| `SELECT questId, eventEntry FROM game_event_seasonal_questrelation` | Seasonal quest availability gating | world |
| `SELECT eventEntry, guid, item, maxcount, incrtime, ExtendedCost, type, BonusListIDs, PlayerConditionId, IgnoreFiltering FROM game_event_npc_vendor ORDER BY guid, slot ASC` | Event-specific vendor items | world |
| `SELECT pool_template.entry, game_event_pool.eventEntry FROM pool_template JOIN game_event_pool ON …` | Event-activated pools | world |
| `SELECT MAX(eventEntry) FROM game_event` | Allocate vector capacity | world |
| `SELECT eventEntry FROM game_event_arena_seasons WHERE season = '{}'` | Map current arena season → event id | world (PQuery) |
| `SELECT eventEntry, state, next_start FROM game_event_save` | Persisted state of world events across restarts | character |
| `SELECT eventEntry, condition_id, done FROM game_event_condition_save` | Persisted progress of world-event conditions | character |

Persistence side (uses prepared statements):

| Prepared statement | Purpose | DB |
|---|---|---|
| `CHAR_DEL_ALL_GAME_EVENT_CONDITION_SAVE` | Wipe condition save table on full reset | character |
| `CHAR_DEL_GAME_EVENT_SAVE` | Delete event save row by event id | character |
| `CHAR_INS_GAME_EVENT_SAVE` | Insert event save row | character |
| `CHAR_DEL_GAME_EVENT_CONDITION_SAVE` | Delete condition save row | character |
| `CHAR_INS_GAME_EVENT_CONDITION_SAVE` | Insert condition save row | character |

DBC stores referenced indirectly:

| Store | What it loads | Read by |
|---|---|---|
| `Holidays.dbc` | Client-side holiday metadata; `game_event.holiday` is an FK into this | `IsHolidayActive` / many holiday scripts |

---

## 7. Wire-protocol packets (if any)

`GameEventMgr` does not own opcodes directly, but its mutations cascade into:

| Opcode | Direction | Sent in |
|---|---|---|
| `SMSG_INIT_WORLD_STATES` (and per-state updates `SMSG_UPDATE_WORLD_STATE`) | server → client | `UpdateWorldStates` and `SendWorldStateUpdate` |
| `SMSG_PLAY_SOUND` | server → client | Triggered by event content scripts (`scripts/Events/*`) on `apply` |
| `SMSG_NPC_TEXT_UPDATE` / `SMSG_GOSSIP_MESSAGE` | server → client | Vendor / quest gossip flips when `UpdateEventNPCFlags` / `UpdateEventNPCVendor` run |
| `SMSG_SPAWN_OBJECT` family (creature/GO updates) | server → client | Cascading from `Map::AddToMap` triggered by `GameEventSpawn` |
| `SMSG_CHAT` (system announce) | server → client | When `game_event.announce = 1`, the start/stop is broadcast |

`GameEvents::Trigger` (the runtime sender, not the scheduler) does not emit packets directly either — it routes through script callbacks that ultimately may.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-gameevents` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- **None.** No `wow-gameevents` crate, no module, no even-stub file. Search results: `crates/wow-constants/src/opcodes.rs` mentions `GameEvent` only in opcode-name string constants; `crates/wow-database/src/statements/world.rs` references it only in passing.

**What's implemented:** Nothing.

**What's missing vs C++:** Everything.
- The `GameEventMgr` singleton.
- Every one of the 16 DB queries.
- All 7 mutator helpers (`GameEventSpawn`, `ChangeEquipOrModel`, `UpdateEventQuests`, `UpdateWorldStates`, `UpdateEventNPCFlags`, `UpdateEventNPCVendor`, `RunSmartAIScripts`).
- The `Update()` tick (the world-server should be calling it once per tick from its main loop).
- `IsHolidayActive` / `IsEventActive` global helpers.
- `HandleQuestComplete` callback wired into the quest turn-in path.
- Persistence (`game_event_save`, `game_event_condition_save`).
- The `GameEvents::Trigger` runtime pipeline (also drives `ScriptMgr::OnEventTrigger` and `event_scripts` table dispatch).

**Suspicious / likely divergent (hypothesis pre-audit):**
- TrinityCore stores absolute UNIX timestamps in `game_event.start_time` and `end_time` for fixed-date events (e.g. Brewfest Sep 20–Oct 6) and uses `occurence`/`length` for recurring weekly events (e.g. Stranglethorn Fishing Extravaganza). The Rust port must not naively recompute by *machine* timezone — TrinityCore uses `time(nullptr)` (UTC seconds since epoch) consistently; preserve that.
- The "world event" state machine (`GAMEEVENT_WORLD_*`) is rare in WoLK content (mostly used for the Operation Gnomeregan / Zalazane's Fall pre-Cataclysm event chains and arena season transitions). It's tempting to drop initially; **don't** — `StartArenaSeason` uses it, and the arena PvP code expects season transitions to flow through this pipeline.
- Many WoLK-era achievements depend on `IsHolidayActive` (e.g. "What a Long, Strange Trip It's Been"). Stubbing the function to "always false" silently breaks dozens of meta-achievements — give it a real implementation early.
- `ApplyNewEvent` mutates spawn lists by **adding to live maps**. With the new `MapManager` in `wow-world`, this needs to go through `MapManager::spawn_creature` and a corresponding `despawn_creature` rather than touching grids directly.
- The `mGameEventCreatureGuids` / `mGameEventGameobjectGuids` are public fields of `GameEventMgr` (the only public mutables on the singleton). They're written by `ObjectMgr` during creature/gameobject load to remember "this guid is event-only, don't spawn it now". Replicating that without breaking `MapManager`'s ownership story takes care.

**Tests existing:** None.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#EVENTS.WBS.001** Partir y cerrar la migracion auditada de `game/Events/GameEventMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`
  Rust target: `crates/wow-gameevents`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1782 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#EVENTS.WBS.002** Cerrar la migracion auditada de `game/Events/GameEventMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`
  Rust target: `crates/wow-gameevents`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#EVENTS.WBS.003** Cerrar la migracion auditada de `game/Events/GameEventSender.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp`
  Rust target: `crates/wow-gameevents`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#EVENTS.WBS.004** Cerrar la migracion auditada de `game/Events/GameEventSender.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.h`
  Rust target: `crates/wow-gameevents`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbering: `#EVENTS.N`. Complexity: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h, split further).

### Phase A — scaffolding & data load

- [ ] **#EVENTS.1** Create `crates/wow-gameevents/` (or a `gameevents` module under `wow-world`); add to workspace; pull in `wow-database`, `wow-core`, `wow-data`, `chrono` (for time math). (L)
- [ ] **#EVENTS.2** Define `pub enum GameEventState`, `pub struct GameEventFinishCondition`, `pub struct GameEventData`, `pub struct ModelEquip`, `pub struct GameEventQuestToEventConditionNum`. Match C++ field order/types (uint32 → u32, uint16 → u16, time_t → i64). (L)
- [ ] **#EVENTS.3** Define `pub struct GameEventMgr` with the 14 internal collections (creature guids, GO guids, model-equip, NPC flags, NPC vendors, creature quests, GO quests, pools, quest-condition reverse map, condition tables, prerequisite map, active-events set, m_game_event vector, isSystemInit). Use `Vec<Vec<T>>` indexed by event id where C++ does — preserves cache layout. (M)
- [ ] **#EVENTS.4** Implement `LoadFromDB` for the master `game_event` query (returns `Vec<GameEventData>`). Use `wow-database`'s direct-query API. (M)
- [ ] **#EVENTS.5** Implement `LoadFromDB` for the remaining 13 world queries. Each gets its own loader function (`load_creatures`, `load_gameobjects`, `load_model_equip`, `load_creature_quests`, …) so they can be unit-tested independently. (H)
- [ ] **#EVENTS.6** Implement persistence-side load: `game_event_save`, `game_event_condition_save` from the **character** DB. Restore world-event state machines. (M)
- [ ] **#EVENTS.7** Implement the 5 prepared statements for state save/restore (`CHAR_INS_GAME_EVENT_SAVE`, …) — register them in `wow-database`'s prepared-statement registry. (M)

### Phase B — scheduler logic

- [ ] **#EVENTS.10** `CheckOneGameEvent(event_id) -> bool` — the time-window arithmetic. **Test against fixed start/end and against recurring `(occurence, length)` events.** (M)
- [ ] **#EVENTS.11** `NextCheck(event_id) -> Duration` — when does this event next change state. (M)
- [ ] **#EVENTS.12** `Update() -> Duration` — the per-tick reconciler. Walks every event id, calls `start_event`/`stop_event`, returns ms-until-next-update. (H)
- [ ] **#EVENTS.13** `StartEvent(event_id, overwrite=false)` — including the world-event state-machine branch (`GAMEEVENT_WORLD_INACTIVE → CONDITIONS → NEXTPHASE → FINISHED`). (H)
- [ ] **#EVENTS.14** `StopEvent(event_id, overwrite=false)`. (M)
- [ ] **#EVENTS.15** `IsEventActive(event_id) -> bool` and `IsHolidayActive(holiday_id) -> bool` free helpers. (L)
- [ ] **#EVENTS.16** `CheckOneGameEventConditions(event_id) -> bool` — all conditions met. (L)

### Phase C — apply/unapply mutators (the heart of the system)

- [ ] **#EVENTS.20** `ApplyNewEvent(event_id)` orchestrator. (L; depends on each mutator)
- [ ] **#EVENTS.21** `UnApplyEvent(event_id)` orchestrator. (L)
- [ ] **#EVENTS.22** `GameEventSpawn(event_id)` — call `MapManager::spawn_creature`/`spawn_gameobject` for every guid in the per-event lists; activate event pools. **Couples with `MapManager` migration in progress.** (H)
- [ ] **#EVENTS.23** `GameEventUnspawn(event_id)` — inverse. (M)
- [ ] **#EVENTS.24** `ChangeEquipOrModel(event_id, activate)` — for every affected creature *currently spawned*, set/restore `display_id` and `equipment_id`. (M)
- [ ] **#EVENTS.25** `UpdateEventQuests(event_id, activate)` — toggle quest-relation tables + seasonal questrelations + quest condition contributions. **Cross-cuts with `wow-quest`.** (H)
- [ ] **#EVENTS.26** `UpdateWorldStates(event_id, activate)` — drive `WorldStateMgr` toggles. **Cross-cuts with worldstate migration.** (M)
- [ ] **#EVENTS.27** `UpdateEventNPCFlags(event_id)` — apply NPC flag overlays to live creatures. (M)
- [ ] **#EVENTS.28** `UpdateEventNPCVendor(event_id, activate)` — add/remove vendor items dynamically. **Cross-cuts with vendor system in `wow-world`.** (M)
- [ ] **#EVENTS.29** `RunSmartAIScripts(event_id, activate)` — `SMART_EVENT_GAME_EVENT_START/END` triggers. **Cross-cuts with `wow-ai/SmartScripts/` (which is itself unstarted).** (M)
- [ ] **#EVENTS.30** `GetNPCFlag(creature) -> u64` — event-aware NPC flag computation, called from `Creature::npc_flags` getter. (M)

### Phase D — quest progression & arena

- [ ] **#EVENTS.40** `HandleQuestComplete(quest_id)` — wire into quest turn-in path; advance world-event conditions; trigger phase transitions. **Cross-cuts with `wow-quest`.** (M)
- [ ] **#EVENTS.41** `SaveWorldEventStateToDB(event_id)` and `SendWorldStateUpdate(player, event_id)` — persistence + per-player worldstate emit on login. (M)
- [ ] **#EVENTS.42** `StartArenaSeason()` — read current season from config, look up `game_event_arena_seasons`, start the matching event. **Cross-cuts with `wow-arena` migration.** (M)
- [ ] **#EVENTS.43** Wire `GameEventMgr::Update()` into the world-server tick loop. Currently no tick wiring exists. (L)

### Phase E — `GameEvents::Trigger` runtime pipeline

- [ ] **#EVENTS.50** `GameEvents::Trigger(event_id, source, target)` — fan out to: `ZoneScript::ProcessEvent` (cross-cuts with zone-script migration), `ScriptMgr::on_event_trigger` (cross-cuts with `scripting.md` #SCRIPTING.35), `GameObjectAI::EventInform`, `Player::TriggerForPlayer`, `Map::TriggerForMap`. (H)
- [ ] **#EVENTS.51** `GameEvents::TriggerForPlayer(event_id, player)` — criteria updates (`PlayerTriggerGameEvent`, `AnyoneTriggerGameEventScenario`), instance criteria fail/start. **Cross-cuts with `wow-achievement`.** (M)
- [ ] **#EVENTS.52** `GameEvents::TriggerForMap(event_id, map, source, target)` — drive `event_scripts` table via `Map::ScriptsStart`. **Cross-cuts with `wow-script` and the `event_scripts` table loader.** (M)
- [ ] **#EVENTS.53** Wire `SPELL_EFFECT_SEND_EVENT` (in `wow-spell`) to call `GameEvents::Trigger`. (L)
- [ ] **#EVENTS.54** Wire `GAMEOBJECT_TYPE_GOOBER` `eventID` (in `wow-world` GameObject Use) to call `GameEvents::Trigger`. (L)

### Phase F — admin & condition integration

- [ ] **#EVENTS.60** `cs_event.cpp` GM command equivalents (`.event start <id>`, `.event stop <id>`, `.event list`, `.event info <id>`). Cross-cuts with `scripts.md` #SCRIPTS.313. (M)
- [ ] **#EVENTS.61** Wire `CONDITION_ACTIVE_EVENT` evaluator in `wow-conditions` to call `IsEventActive`. (L; depends on `#CONDITIONS.*`)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#EVENTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 2070 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp`. Rust target: `crates/wow-database`, `crates/wow-world`. | `cargo test -p wow-database && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#EVENTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 2070 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp`. Rust target: `crates/wow-database`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#EVENTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 2070 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp`. Rust target: `crates/wow-database`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#EVENTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 2070 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp`. Rust target: `crates/wow-database`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `LoadFromDB` correctly populates `m_game_event` from a fixture `game_event` table with 1 fixed-date event + 1 recurring event + 1 world event.
- [ ] Test: `CheckOneGameEvent` returns `true` for `now == start_time`, `false` for `now == end_time + 1`, and the right answer for a recurring event mid-cycle.
- [ ] Test: `NextCheck` for a recurring event at `t = start + length + 1` returns `(occurence - length - 1)` seconds.
- [ ] Test: `IsHolidayActive(HALLOWS_END)` is `true` exactly when an event with `holiday_id == HALLOWS_END` is in `m_active_events`.
- [ ] Test: `StartEvent` followed by `StopEvent` results in zero net spawns / zero net NPC flag overlays / zero net vendor diffs.
- [ ] Test: `ApplyNewEvent` for an event with model-equip rules updates the affected creature's `display_id` to `modelid` and reverts to `modelid_prev` on `UnApplyEvent`.
- [ ] Test: `HandleQuestComplete` for a quest mapped to event condition `(event_id=X, condition_id=1, num=5)` increments `done` by 5, and once `done >= reqNum` the event transitions to `GAMEEVENT_WORLD_NEXTPHASE`.
- [ ] Test: `game_event_save` row written by `SaveWorldEventStateToDB` round-trips correctly through `LoadFromDB` after server restart.
- [ ] Test: `StartArenaSeason` for `season=8` activates the event linked in `game_event_arena_seasons` for that season.
- [ ] Test: `GameEvents::Trigger(eventId=X, source, target)` with an `event_scripts` row matching `id=X` causes the script's actions to run.
- [ ] Test: `GameEvents::TriggerForPlayer` updates `CriteriaType::PlayerTriggerGameEvent` for the source player.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 2070 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp` | No dedicated crate yet. Recommend a new `crates/wow-gameevents/` (or fold into `wow-world` if minimal). Cross-cutting: spawn/despawn drives `MapManager`, NPC flags drive `wow-world`, vendor swaps drive `wow-database`'s prepared-statement registry. \| ❌ not started — `GameEventMgr` does not exist in any form in `crates/`. **No** holiday scheduling, **no** Hallow's End / Brewfest / Winter Veil / Lunar Festival cycle, **no** Darkmoon Faire, **no** quest seasonal-availability gating, **no** `IsHolidayActive()` lookup, **no** event-driven creature/gameobject swap (model/equipment/NPC flag changes during holidays), **no** event-driven vendor inventory swap, **no** event-driven world state updates. The `GameEvents::Trigger` pipeline (which fires `event_scripts` on spell `EFFECT_SEND_EVENT`) is also missing — that one cross-cuts with `wow-spell`'s effect dispatcher. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#EVENTS.DIV.001` | `crates/wow-gameevents` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 2070 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventSender.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Two separate "events" — keep them separate.** `GameEventMgr` (this doc) is the *holiday/world-event scheduler*. `GameEvents::Trigger` (also this doc, but separate API) is the *runtime event-id sender* used by spell `EFFECT_SEND_EVENT` and `GAMEOBJECT_TYPE_GOOBER`. They share neither code path nor data table — `GameEventMgr` reads `game_event*` tables, `GameEvents::Trigger` reads `event_scripts`. C++ TrinityCore puts them in the same directory because they both involve "events", but they are independent systems. Don't merge them in Rust.
- **Time storage matters.** `game_event.start_time DATETIME` is read with `UNIX_TIMESTAMP()` and compared to `time(nullptr)`. In Rust, use `i64` UTC seconds (or `chrono::DateTime<Utc>`) — never local time, never `SystemTime` without explicit UTC conversion.
- The `mGameEventCreatureGuids` / `mGameEventGameobjectGuids` collections are **populated by `ObjectMgr` during creature/gameobject load** (so that creature load can skip event-only spawns "for now") and consumed by `GameEventSpawn`. Two-phase initialization: `ObjectMgr::LoadCreatures` runs first and writes; `GameEventMgr::Initialize` runs second and reads. Preserve this ordering.
- The `mGameEventModelEquip` and `mGameEventNPCFlags` collections only flip state for creatures **currently spawned**. If a creature respawns later in an event window, `Creature::SelectLevel` / `Creature::LoadEquipment` re-checks active events — so `Creature` itself is event-aware. Replicate that polling check in the Rust `Creature` (or via `MapManager::spawn_creature` consulting `GameEventMgr`).
- WoLK 3.4.3 specific: the event id range goes up to ~150. Sizing `Vec` to `max_event_id + 1` is the canonical approach (cheap, dense). Don't use a `HashMap` — straight indexing is hot in `Update()`.
- The `length` field is in **minutes**, but `start_time`/`end_time`/`occurence` are in **seconds**. This is a long-standing TrinityCore footgun; mark the type difference clearly in Rust (e.g. `length: Minutes`, others `Seconds`).
- `game_event.world_event` field stores `state` and is **directly written back** to the world DB on world-event transitions (TC stores world-event state in *both* world and character DBs — world DB row is updated for shared visibility, character DB `game_event_save` for restart resilience). Don't drop the world-DB write.
- `SMART_EVENT_GAME_EVENT_START/_END` (#EVENTS.29) blocks until the SAI engine exists. If `wow-ai` SmartScripts isn't ready, stub `RunSmartAIScripts` to a no-op with a `tracing::warn!` and ship the rest — it's not a correctness blocker for non-SAI content.
- `Holidays.dbc` has a quirky multi-stage structure (`HolidayDuration*`, `HolidayDate*` arrays) used by the client to render the calendar. Server-side `holiday_id` is just an opaque tag; you don't need to parse the DBC structure for `IsHolidayActive` to work — the join is strictly on the integer `holiday_id`.
- `GameEvents::Trigger` is reachable from spell-effect dispatch; an unfortunate misuse is firing `Trigger` with `source=nullptr` AND `target=nullptr`, which `ASSERT`s in C++. The Rust port should `Result<(), TriggerError>` rather than panic.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class GameEventMgr` (singleton) | `pub struct GameEventMgr { … }` + `pub static GAME_EVENT_MGR: OnceLock<RwLock<GameEventMgr>> = …` | Read-mostly; mutated only on `Update`/`HandleQuestComplete`. |
| `enum GameEventState` | `#[repr(u8)] enum GameEventState { Normal, WorldInactive, WorldConditions, WorldNextPhase, WorldFinished, Internal }` | Match `repr(u8)` to the on-disk byte. |
| `struct GameEventData` | `pub struct GameEventData { start: i64, end: i64, next_start: i64, occurence: u32, length_minutes: u32, holiday_id: HolidayId, holiday_stage: u8, state: GameEventState, conditions: HashMap<u32, GameEventFinishCondition>, prerequisite_events: HashSet<u16>, description: String, announce: u8 }` | Use a typed wrapper for `length_minutes` to avoid the seconds-vs-minutes footgun. |
| `vector<list<LowGuid>>` (`mGameEventCreatureGuids`) | `Vec<Vec<u64>>` indexed by event id | Inner `Vec` is fine; insertion is one-shot at load. |
| `vector<unordered_map<entry, vector<VendorItem>>>` (`mGameEventVendors`) | `Vec<HashMap<u32, Vec<VendorItem>>>` | — |
| `set<uint16> m_ActiveEvents` | `BTreeSet<u16>` or `HashSet<u16>` | BTreeSet keeps debug output ordered. |
| `time_t start` / `time(nullptr)` | `i64` UTC seconds via `chrono::Utc::now().timestamp()` | Never local time. |
| `bool IsHolidayActive(HolidayIds)` | `pub fn is_holiday_active(id: HolidayId) -> bool` | Free function delegating to singleton. |
| `bool IsEventActive(uint16)` | `pub fn is_event_active(event_id: u16) -> bool` | Same. |
| `namespace GameEvents { void Trigger(...) }` | `pub mod game_events { pub fn trigger(...) -> Result<(), TriggerError> { … } }` | Replace `ASSERT` with `Result`. |
| `sGameEventMgr->Update()` | `GAME_EVENT_MGR.read().update()` (driven by world-server tick) | Returns `Duration` until next call. |
| `sGameEventMgr->HandleQuestComplete(qid)` | `GAME_EVENT_MGR.write().handle_quest_complete(qid)` | Called from quest turn-in. |
| `WorldDatabase.Query("SELECT … FROM game_event …")` | `wow_database::world::query_game_events()` per loader fn | One fn per loader, all gathered in `LoadFromDB`. |
| `CharacterDatabasePreparedStatement *stmt = CharacterDatabase.GetPreparedStatement(CHAR_INS_GAME_EVENT_SAVE);` | `wow_database::character::stmt::INS_GAME_EVENT_SAVE.execute(&[event_id, state, next_start])` | Mirror prepared-statement registry pattern. |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — `GameEventMgr` and `GameEventSender` are entirely absent.**

Evidence from `grep -rn -i "GameEvent\|HolidayId\|IsHolidayActive\|IsEventActive" crates/ --include='*.rs'`:

- `wow-constants/src/opcodes.rs:237,238,577` — three GM **debug** opcodes (`GameEventDebugDisable=0x31b2`, `GameEventDebugEnable=0x31b1`, `SetGameEventDebugViewState=0x31b9`). No handler — these would be dispatched to "unknown opcode" if anyone ever sent them.
- `wow-database/src/statements/world.rs:62,63,141,211,212` — two `DELETE` prepared statements (`DEL_GAME_EVENT_CREATURE`, `DEL_GAME_EVENT_MODEL_EQUIP`, `DEL_EVENT_GAMEOBJECT`). These appear to be wired into `wow-database`'s `.gobject delete` / creature deletion paths so that removing a spawn cleans up its `game_event_*` row references — purely a referential-integrity cleanup. **They do not load, evaluate, or schedule events.** The 16+ `SELECT` queries listed in §6 are all missing.

Zero hits for `IsHolidayActive`, `IsEventActive`, `HolidayIds`, `GameEventState`, `GAMEEVENT_NORMAL`, `GameEventMgr`, `m_ActiveEvents`, `mGameEventCreatureGuids`, etc.

**Silent-default consequences:**
- `CONDITION_ACTIVE_EVENT` would always evaluate true / false consistent (depending on caller default) — moot because ConditionMgr is also ❌.
- Seasonal quests (`game_event_seasonal_questrelation`) are unfiltered: every holiday quest is takeable year-round. Hallow's End candy buckets, brewfest dailies, all quest helper NPCs would be permanently unavailable (no spawn) **and** permanently available (no gate) simultaneously depending on which side of the system you look at — but in practice the spawns simply don't happen because `game_event_creature` rows are never read.
- Holiday scripts in `crates/wow-scripts/Events/` (also ❌) have nothing to hook into.
- `SPELL_EFFECT_SEND_EVENT` and `GAMEOBJECT_TYPE_GOOBER` event-trigger paths (`GameEvents::Trigger`) silently do nothing.

**Coupling:** Mostly self-contained. Soft-blocked on ConditionMgr (#COND.* — for `CONDITION_ACTIVE_EVENT` and `game_event_quest_condition` evaluation). The state machine itself can be built in isolation. Tractable mid-priority; ICC content does not require it.
