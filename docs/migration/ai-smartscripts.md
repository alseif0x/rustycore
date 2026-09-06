# Migration: AI — SmartScripts (data-driven engine)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/AI/SmartScripts/`
> **Rust target crate(s):** `crates/wow-script/` (engine), consumes traits from `crates/wow-ai/`
> **Layer:** L5 — game systems (data-driven AI interpreter)
> **Status:** ❌ not started — `crates/wow-script/` is **0 lines**; the only trace of SmartScripts in the workspace is one unused SQL prepared-statement constant
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

> **Sub-doc of [`ai.md`](./ai.md).** Companion: [`ai-base.md`](./ai-base.md) (`UnitAI`/`CreatureAI` traits and stock subclasses — hard prerequisite, since `SmartAI` *is* a `CreatureAI`). Also see [`scripting.md`](./scripting.md) (the broader script registry that SmartScripts coexists with) and [`scripts.md`](./scripts.md) (the smaller pool of bespoke C++ boss scripts that SmartScripts replaces for ~95% of NPCs).

---

## 1. Purpose

SmartScripts is TrinityCore's **data-driven AI interpreter**. Instead of writing C++ for every mob and boss, behaviour is encoded as rows in the `smart_scripts` SQL table (~50,000 rows in stock data) — each row pairs an `event_type` (e.g. "health below 30%", "aggro", "spell hit", "out-of-combat LOS", "respawn", "summoned") with an `action_type` (e.g. "talk", "cast spell", "summon creature", "phase change", "play emote", "move to position") and a `target_type` (e.g. "victim", "closest player", "all hostile in range", "stored target list"). Bosses encode multi-phase encounters as sequences of these rows with phase masks, group ids, link chains, chance rolls and event flags. The runtime, `SmartScript`, sits inside a `SmartAI` (which is itself a `CreatureAI` — see `ai-base.md`) and ticks each map update: `OnUpdate(diff)` walks `mEvents`, fires events whose conditions match, calls `ProcessEvent → ProcessAction`, resolves targets via `GetTargets`. **~95% of all NPC behaviour in 3.4.3 is implemented this way** — only marquee raid bosses still get bespoke C++ in `scripts.md`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/AI/SmartScripts/SmartAI.cpp` | 1259 | `prefix` |
| `game/AI/SmartScripts/SmartAI.h` | 354 | `prefix` |
| `game/AI/SmartScripts/SmartScript.cpp` | 4253 | `prefix` |
| `game/AI/SmartScripts/SmartScript.h` | 152 | `prefix` |
| `game/AI/SmartScripts/SmartScriptMgr.cpp` | 2497 | `prefix` |
| `game/AI/SmartScripts/SmartScriptMgr.h` | 1769 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/AI/SmartScripts/SmartAI.h` | 354 | `class SmartAI : CreatureAI` declaration; integrates waypoints, escort flow, follow flow, conversation triggers; `class SmartGameObjectAI : GameObjectAI`; `class SmartAreaTriggerAI : AreaTriggerAI` |
| `src/server/game/AI/SmartScripts/SmartAI.cpp` | 1259 | `UpdateAI` dispatch to `SmartScript::OnUpdate`; waypoint movement state machine (`StartPath`, `PausePath`, `ResumePath`, `EndPath`, `WaypointReached`, `WaypointPathEnded`); escort invoker tracking + max-distance check; gossip dispatch; charm hook; `JustEnteredCombat`/`JustEngagedWith`/`JustDied`/etc. all forward to `SmartScript::ProcessEventsFor` |
| `src/server/game/AI/SmartScripts/SmartScript.h` | 152 | `class SmartScript` interpreter declaration: `mEvents`, `mInstallEvents`, `mTimedActionList`, `mStoredEvents`, `mCounterList`, `_storedTargets`, `mEventPhase`, `mPathId`, `mLastInvoker`, `OnInitialize`, `ProcessEventsFor`, `ProcessEvent`, `ProcessAction`, `ProcessTimedAction`, `GetTargets`, `OnUpdate`, `OnReset`, `IncPhase`/`DecPhase`/`SetPhase`/`IsInPhase`, `StoreTargetList`/`StoreCounter`/`GetCounterValue` |
| `src/server/game/AI/SmartScripts/SmartScript.cpp` | 4253 | The **interpreter giant**. `OnUpdate` walks events; `ProcessEvent` switch over ~80 `SMART_EVENT_*` (health/mana percent gates, aggro/kill/death hooks, range checks, LOS hooks, victim casting checks, target casting checks, group buff checks, achievement criteria, instance state, scene completion, area-trigger entry, quest accept/reward, gossip select, friendly/enemy buff missing, distance to summon, target/owner death, charge end, summoned despawn, etc.); `ProcessAction` switch over ~150 `SMART_ACTION_*` (Talk, SimpleTalk, PlayEmote, SoundPlay, AttackStart/Stop, Cast, AddAura, AreaCast, RandomCast, KillSelf, ForceDespawn, SetReactState, MorphToEntry, MountToEntry, SetUnitFlag, SetVisibility, MoveToPos, RandomMove, MoveOffset, Teleport, Jump, Fly, Run, Walk, SetEventPhase, IncEventPhase, ResetEventPhase, AddItem, RemoveItem, EquipItem, SummonCreature, SummonGameObject, ActivateGameObject, OpenDoor, CloseDoor, ResetGameObject, SetActive, AttackPCFlag, RemoveAuras, ChangeFaction, SetVehicleId, CallForHelp, CallScriptedHelp, ZoneCombat, SendCustomEvent, SetGoFlag, SetGoState, AddGroupTo, SendCreaturePlayPacket, ScriptCreatureFlee, GoSetLootState, GoSetGoState, SendTaxi, ConversationCreate, FleeForAssist, EnableTempGobj, etc.); `GetTargets` switch over ~30 `SMART_TARGET_*` (Self, Victim, HostileSecondAggro, HostileLastAggro, HostileRandom, ActionInvoker, OwnerOrSummoner, ThreatList, ClosestCreature, ClosestGameObject, ClosestPlayer, CreatureRange, GameObjectRange, CreatureGuid, GameObjectGuid, StoredTarget, ClosestEnemy, ClosestFriendly, LootRecipient, VehiclePassenger, etc.); event chance rolling; phase mask gating; flag handling (`NOT_REPEATABLE`, `DEBUG_ONLY`, `DIFFICULTY_*`, `DONT_RESET`, `WHILE_CHARMED`); link chain dispatch; nested-event guard (`MAX_NESTED_EVENTS=10`); `RecalcTimer`/`UpdateTimer`/`InitTimer` for repeating events |
| `src/server/game/AI/SmartScripts/SmartScriptMgr.h` | 1769 | All enum and struct definitions: `enum SMART_EVENT` (~80), `enum SMART_ACTION` (~150), `enum SMARTAI_TARGETS` (~30), `enum SmartScriptType` (Creature, GameObject, AreaTrigger, Event, Gossip, Quest, Spell, Transport, Instance, TimedActionList, Scene, AreaTriggerEntity, AreaTriggerEntityServerSide), `enum SMART_EVENT_PHASE` (1–12 + `_ALWAYS`/`_MAX`/`_COUNT`), `enum SMART_EVENT_PHASE_BITS` (bitmask 1, 2, 4, 8, …, 2048, `_ALL`), `enum SmartEventFlags` (`NOT_REPEATABLE`, `DIFFICULTY_0..3`, `RESERVED_5`, `DEBUG_ONLY`, `DONT_RESET`, `WHILE_CHARMED`), `enum SmartCastFlags` (`INTERRUPT_PREVIOUS`, `TRIGGERED`, `FORCE_CAST`, `NO_COMBAT_MOVE`, `AURA_NOT_PRESENT`, `COMBAT_MOVE`), `struct SmartScriptHolder` (entryOrGuid, source_type, id, link, event/action/target structs, timer state, priority), `struct SmartTarget`, `struct SmartEvent`, `struct SmartAction`, validation helper tables, `SmartPhaseMask[][2]`, `SmartAIEventList`, `SmartAIEventStoredList`, factory of `SmartScriptHolder` from raw SQL columns; `class SmartWaypointMgr` declaration |
| `src/server/game/AI/SmartScripts/SmartScriptMgr.cpp` | 2497 | Singleton `sSmartScriptMgr`: loads `smart_scripts` (full table) into `mEventMap[ScriptType][entryOrGuid] -> SmartAIEventList`; row-by-row column validation with `IsEventValid` (rejects rows whose action_param doesn't match the action's expected schema, whose target_param mismatches the target type, whose link points to non-existent id, whose event_phase_mask exceeds 12 bits, whose chance >100, whose entryOrGuid points to a non-existent template/spawn, etc.); `SmartWaypointMgr` loads `waypoint_data` and `waypoints` (legacy); `LoadSmartAIFromDB(SmartScriptType)` per-type loader; localization for `creature_text` happens in `CreatureTextMgr` (separate module) but is consumed via `SMART_ACTION_TALK`/`SMART_ACTION_SIMPLE_TALK` |
| `src/server/game/AI/SmartScripts/SmartScriptDefines.h` | (small, ~50) | Macro / constant definitions (`SMART_EVENT_PARAM_COUNT=4`, `SMART_ACTION_PARAM_COUNT=7`, `MAX_NESTED_EVENTS=10`) |

**Sub-module total:** ~10,300 lines (4253 cpp + 2497 mgr.cpp + 1769 mgr.h + 1259 SmartAI.cpp + 354 SmartAI.h + 152 SmartScript.h + ~50 defines).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SmartAI` | class : CreatureAI | Runtime engine; owns a `SmartScript`; integrates waypoint/escort/follow state on top of `CreatureAI` hooks |
| `SmartGameObjectAI` | class : GameObjectAI | SmartScript host for `GameObject` (chests, traps, doors with scripted behaviour) |
| `SmartAreaTriggerAI` | class : AreaTriggerAI | SmartScript host for client-side area triggers |
| `SmartScript` | class | The interpreter; holds `mEvents`, `mEventPhase`, `mPathId`, `mLastInvoker`, `mCounterList`, `_storedTargets`, `mNestedEventsCounter` |
| `SmartScriptHolder` | struct | One parsed row from `smart_scripts`: `entryOrGuid: int64` (positive=template, negative=spawn), `source_type: SmartScriptType`, `id: u32`, `link: u32`, `event: SmartEvent`, `action: SmartAction`, `target: SmartTarget`, internal timer state (`timer`, `priority`, `runOnce`, `enableTimed`) |
| `SmartEvent` | struct | event_type + 4 raw u32 params + event_phase_mask + event_chance + event_flags |
| `SmartAction` | struct | action_type + 7 raw u32 params (nested anonymous union per action type) |
| `SmartTarget` | struct | target_type + 4 params + (x, y, z, o) |
| `SmartScriptMgr` | singleton | Loader/validator/cache: `mEventMap[ScriptType][entryOrGuid]` |
| `SmartWaypointMgr` | singleton | `waypoint_data` / `waypoints` cache `mPaths[entryOrPathId] -> WaypointPath` |
| `WaypointPath` | struct | Vec of `WaypointNode { id, x, y, z, orientation, delay, action_id, action_chance }` |
| `enum SmartScriptType` | enum | `Creature=0`, `GameObject=1`, `AreaTrigger=2`, `Event=3`, `Gossip=4`, `Quest=5`, `Spell=6`, `Transport=7`, `Instance=8`, `TimedActionList=9`, `Scene=10`, `AreaTriggerEntity=11`, `AreaTriggerEntityServerSide=12`, `Max=13` |
| `enum SMART_EVENT` | enum (~80) | UPDATE_IC, UPDATE_OOC, HEALTH_PCT, MANA_PCT, AGGRO, KILL, DEATH, EVADE, SPELLHIT, RANGE, OOC_LOS, RESPAWN, TARGET_HEALTH_PCT, VICTIM_CASTING, FRIENDLY_HEALTH, FRIENDLY_IS_CC, FRIENDLY_MISSING_BUFF, SUMMONED_UNIT, TARGET_MANA_PCT, ACCEPTED_QUEST, REWARD_QUEST, REACHED_HOME, RECEIVE_EMOTE, HAS_AURA, TARGET_BUFFED, RESET, IC_LOS, PASSENGER_BOARDED, PASSENGER_REMOVED, CHARMED, CHARMED_TARGET, SPELLHIT_TARGET, DAMAGED, DAMAGED_TARGET, MOVEMENTINFORM, SUMMON_DESPAWNED, CORPSE_REMOVED, AI_INIT, DATA_SET, WAYPOINT_START, WAYPOINT_REACHED, TRANSPORT_ADDPLAYER, TRANSPORT_ADDCREATURE, TRANSPORT_REMOVE_PLAYER, TRANSPORT_RELOCATE, INSTANCE_PLAYER_ENTER, AREATRIGGER_ONTRIGGER, QUEST_ACCEPTED, QUEST_OBJ_COMPLETION, QUEST_COMPLETION, QUEST_REWARDED, QUEST_FAIL, TEXT_OVER, RECEIVE_HEAL, JUST_SUMMONED, WAYPOINT_PAUSED, WAYPOINT_RESUMED, WAYPOINT_STOPPED, WAYPOINT_ENDED, TIMED_EVENT_TRIGGERED, UPDATE, LINK, GOSSIP_SELECT, JUST_CREATED, GOSSIP_HELLO, FOLLOW_COMPLETED, EVENT_PHASE_CHANGE, IS_BEHIND_TARGET, GAME_EVENT_START, GAME_EVENT_END, GO_LOOT_STATE_CHANGED, GO_EVENT_INFORM, ACTION_DONE, ON_SPELLCLICK, FRIENDLY_HEALTH_PCT, DISTANCE_CREATURE, DISTANCE_GAMEOBJECT, COUNTER_SET, SCENE_START, SCENE_TRIGGER, SCENE_CANCEL, SCENE_COMPLETE, SUMMONED_UNIT_DIES, ON_SPELL_CAST, ON_SPELL_FAILED, ON_SPELL_START, ON_DESPAWN, SEND_EVENT_TRIGGER |
| `enum SMART_ACTION` | enum (~150) | NONE, TALK, SET_FACTION, MORPH_TO_ENTRY_OR_MODEL, SOUND, PLAY_EMOTE, FAIL_QUEST, OFFER_QUEST, SET_REACT_STATE, ACTIVATE_GOBJECT, RANDOM_EMOTE, CAST, SUMMON_CREATURE, THREAT_SINGLE_PCT, THREAT_ALL_PCT, CALL_AREAEXPLOREDOREVENTHAPPENS, SET_EMOTE_STATE, SET_UNIT_FLAG, REMOVE_UNIT_FLAG, AUTO_ATTACK, ALLOW_COMBAT_MOVEMENT, SET_EVENT_PHASE, INC_EVENT_PHASE, EVADE, FLEE_FOR_ASSIST, CALL_GROUPEVENTHAPPENS, COMBAT_STOP, REMOVEAURASFROMSPELL, FOLLOW, RANDOM_PHASE, RANDOM_PHASE_RANGE, RESET_GOBJECT, CALL_KILLEDMONSTER, SET_INST_DATA, SET_INST_DATA64, UPDATE_TEMPLATE, DIE, SET_IN_COMBAT_WITH_ZONE, CALL_FOR_HELP, SET_SHEATH, FORCE_DESPAWN, SET_INVINCIBILITY_HP_LEVEL, MOUNT_TO_ENTRY_OR_MODEL, SET_INGAME_PHASE_GROUP, SET_DATA, ATTACK_STOP, SET_VISIBILITY, SET_ACTIVE, ATTACK_START, SUMMON_GO, KILL_UNIT, ACTIVATE_TAXI, WP_START, WP_PAUSE, WP_STOP, ADD_ITEM, REMOVE_ITEM, INSTALL_AI_TEMPLATE, SET_RUN, SET_DISABLE_GRAVITY, SET_SWIM, TELEPORT, SET_COUNTER, STORE_TARGET_LIST, WP_RESUME, SET_ORIENTATION, CREATE_TIMED_EVENT, PLAYMOVIE, MOVE_TO_POS, ENABLE_TEMP_GOBJ, EQUIP, CLOSE_GOSSIP, TRIGGER_TIMED_EVENT, REMOVE_TIMED_EVENT, ADD_AURA, OVERRIDE_SCRIPT_BASE_OBJECT, RESET_SCRIPT_BASE_OBJECT, CALL_SCRIPT_RESET, SET_RANGED_MOVEMENT, CALL_TIMED_ACTIONLIST, SET_NPC_FLAG, ADD_NPC_FLAG, REMOVE_NPC_FLAG, SIMPLE_TALK, SELF_CAST, CROSS_CAST, CALL_RANDOM_TIMED_ACTIONLIST, CALL_RANDOM_RANGE_TIMED_ACTIONLIST, RANDOM_MOVE, SET_UNIT_FIELD_BYTES_1, REMOVE_UNIT_FIELD_BYTES_1, INTERRUPT_SPELL, SEND_GO_CUSTOM_ANIM, SET_DYNAMIC_FLAG, ADD_DYNAMIC_FLAG, REMOVE_DYNAMIC_FLAG, JUMP_TO_POS, SEND_GOSSIP_MENU, GO_SET_LOOT_STATE, SEND_TARGET_TO_TARGET, SET_HOME_POS, SET_HEALTH_REGEN, SET_ROOT, SET_GO_FLAG, ADD_GO_FLAG, REMOVE_GO_FLAG, SUMMON_CREATURE_GROUP, SET_POWER, ADD_POWER, REMOVE_POWER, GAME_EVENT_STOP, GAME_EVENT_START, START_CLOSEST_WAYPOINT, RISE_UP, MOVE_OFFSET, RANDOM_SOUND, SET_CORPSE_DELAY, DISABLE_EVADE, GO_SET_GO_STATE, ADD_THREAT, LOAD_EQUIPMENT, TRIGGER_RANDOM_TIMED_EVENT, REMOVE_ALL_GAMEOBJECTS, PAUSE_MOVEMENT, PLAY_ANIMKIT, SCENE_PLAY, SCENE_CANCEL, SPAWN_SPAWNGROUP, DESPAWN_SPAWNGROUP, RESPAWN_BY_SPAWNID, INVOKER_CAST, PLAY_CINEMATIC, SET_MOVEMENT_SPEED, PLAY_SPELL_VISUAL_KIT, OVERRIDE_LIGHT, OVERRIDE_WEATHER, SET_AI_ANIM_KIT, SET_HOVER, SET_HEALTH_PCT, CREATE_CONVERSATION, SET_IMMUNE_PC, SET_IMMUNE_NPC, SET_UNINTERACTIBLE, ACTIVATE_GAMEOBJECT, ADD_TO_STORED_TARGET_LIST, BECOME_PERSONAL_CLONE_FOR_PLAYER, TRIGGER_GAME_EVENT, DO_ACTION |
| `enum SMARTAI_TARGETS` | enum (~30) | NONE, SELF, VICTIM, HOSTILE_SECOND_AGGRO, HOSTILE_LAST_AGGRO, HOSTILE_RANDOM, HOSTILE_RANDOM_NOT_TOP, ACTION_INVOKER, POSITION, CREATURE_RANGE, CREATURE_GUID, CREATURE_DISTANCE, STORED, GAMEOBJECT_RANGE, GAMEOBJECT_GUID, GAMEOBJECT_DISTANCE, INVOKER_PARTY, PLAYER_RANGE, PLAYER_DISTANCE, CLOSEST_CREATURE, CLOSEST_GAMEOBJECT, CLOSEST_PLAYER, ACTION_INVOKER_VEHICLE, OWNER_OR_SUMMONER, THREAT_LIST, CLOSEST_ENEMY, CLOSEST_FRIENDLY, LOOT_RECIPIENTS, VEHICLE_PASSENGER, FARTHEST |
| `enum SMART_EVENT_PHASE` | enum 0–13 | `_ALWAYS=0`, 1..12 phases, `_MAX=13`, `_COUNT=12` |
| `enum SMART_EVENT_PHASE_BITS` | enum bitmask | bits 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, plus `_ALL` mask |
| `enum SmartEventFlags` | bitmask | `NOT_REPEATABLE=1`, `DIFFICULTY_0=2`, `DIFFICULTY_1=4`, `DIFFICULTY_2=8`, `DIFFICULTY_3=16`, `RESERVED_5=32`, `DEBUG_ONLY=128`, `DONT_RESET=256`, `WHILE_CHARMED=512` |
| `enum SmartCastFlags` | bitmask | `INTERRUPT_PREVIOUS`, `TRIGGERED`, `FORCE_CAST`, `NO_COMBAT_MOVE`, `AURA_NOT_PRESENT`, `COMBAT_MOVE` |
| `EscortState` | enum bitmask | `NONE`, `ESCORTING`, `RETURNING`, `PAUSED`, `MAX` |
| constants | macros | `SMART_EVENT_PARAM_COUNT=4`, `SMART_ACTION_PARAM_COUNT=7`, `MAX_NESTED_EVENTS=10` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `SmartAI::UpdateAI(uint32 diff)` | Per-tick driver | `SmartScript::OnUpdate(diff)`; waypoint state advance |
| `SmartAI::JustEnteredCombat / JustEngagedWith / JustDied / KilledUnit / SpellHit / SpellHitTarget / JustSummoned / SummonedCreatureDies / MoveInLineOfSight / ReceiveEmote / MovementInform / ReceiveSpellCastSuccess / DamageTaken / OnSpellClick / WaypointReached / WaypointPathEnded` | All forward to `SmartScript::ProcessEventsFor(SMART_EVENT_*, …)` | `SmartScript::ProcessEventsFor` |
| `SmartAI::StartPath(run, pathId, repeat, invoker, nodeId)` | Begin waypoint path | `SmartWaypointMgr::GetPath`, `MotionMaster::MovePath` |
| `SmartAI::PausePath(uint32 delay, bool forced)` | Pause WP execution; talks/actions can occupy the slot | timer |
| `SmartAI::ResumePath()` | Resume after pause | `MotionMaster::MovePath` |
| `SmartAI::EndPath(bool fail)` | Finalize path; quest credit on success or despawn | `Player::AreaExploredOrEventHappens`, `Creature::DespawnOrUnsummon` |
| `SmartAI::SetEscortPaused / SetWPPauseTimer` | Escort flow control | — |
| `SmartAI::SetCanCombatMove(bool)` / `SetEvadeDisabled(bool)` | Flags toggled by actions | — |
| `SmartScript::OnInitialize(WorldObject* obj, …)` | Build event list from `SmartScriptMgr` cache | `sSmartScriptMgr->GetScript(entryOrGuid, type)` |
| `SmartScript::OnUpdate(uint32 diff)` | Walk events; tick timers; fire ready events | `UpdateTimer`, `ProcessEvent`, `RecalcTimer` |
| `SmartScript::ProcessEventsFor(SMART_EVENT e, Unit* unit, var0, var1, bvar, spell, gob, varStr)` | Top-level dispatch entry from `SmartAI` hooks | iterates `mEvents`, `ProcessEvent` per match |
| `SmartScript::ProcessEvent(SmartScriptHolder& e, …)` | One row's event match check (~80-arm switch) | `ProcessAction` if matched |
| `SmartScript::ProcessAction(SmartScriptHolder& e, …)` | One row's action execution (~150-arm switch) | varies per action |
| `SmartScript::ProcessTimedAction(e, min, max, …)` | Repeat-with-cooldown handling | `RecalcTimer` |
| `SmartScript::GetTargets(out, e, invoker)` | Resolve `SmartTarget` to `Vec<WorldObject*>` (~30-arm switch) | grid visit, threat list, stored targets |
| `SmartScript::GetWorldObjectsInDist(out, dist)` | Helper for range-based targets | `Cell::VisitGrid` |
| `SmartScript::DoSelectLowestHpFriendly / DoSelectLowestHpPercentFriendly / DoFindFriendlyCC / DoFindFriendlyMissingBuff / DoFindClosestFriendlyInRange` | Friendly target helpers | grid visit + filters |
| `SmartScript::IsUnit / IsPlayer / IsCreature / IsCharmedCreature / IsGameObject` | Static type checks on `WorldObject*` | dynamic_cast |
| `SmartScript::SetPhase(uint32) / IncPhase / DecPhase / IsInPhase` | Phase mutation; events filter by phase mask | — |
| `SmartScript::StoreTargetList(targets, id) / AddToStoredTargetList / GetStoredTargetVector` | Persistent target lists between events | `_storedTargets` map |
| `SmartScript::StoreCounter(id, value, reset) / GetCounterValue` | Boss state counters | `mCounterList` |
| `SmartScript::SetTimedActionList(e, entry, invoker, startFromEventId)` | Switch event list to a `TimedActionList` script | `sSmartScriptMgr->GetScript(entry, TimedActionList)` |
| `SmartScript::OnReset()` | Restart encounter — clear timers, reset phase, restore initial events | `ProcessEventsFor(RESET)` |
| `SmartScript::OnMoveInLineOfSight(Unit*)` | Specifically for `OOC_LOS` / `IC_LOS` events | `ProcessEventsFor` |
| `SmartScript::CreateSmartEvent(e, flags, eParams×5, action, aParams×7, target, tParams×4, phaseMask)` | Static factory: build holder from raw columns | — |
| `SmartScript::CheckTimer(holder)` / `UpdateTimer` / `RecalcTimer` / `InitTimer` | Per-event timer arithmetic for repeating events | — |
| `SmartScriptMgr::LoadSmartAIFromDB(SmartScriptType)` | Load all rows of given script type | `WorldDatabase.Query("SELECT * FROM smart_scripts WHERE source_type = ?")` |
| `SmartScriptMgr::IsEventValid(SmartScriptHolder& e)` | Per-row validation; rejects malformed rows so the runtime never sees bad data | many |
| `SmartScriptMgr::GetScript(entryOrGuid, type)` | Cache lookup at spawn time | `mEventMap[type][entryOrGuid]` |
| `SmartScriptMgr::CombineSmartAI(...)` | Merge template + spawn rows | — |
| `SmartWaypointMgr::LoadFromDB()` | Load `waypoint_data` and `waypoints` | `WorldDatabase.Query` |
| `SmartWaypointMgr::GetPath(entryOrPathId)` | Path lookup | `mPaths` |

---

## 5. Module dependencies

**Depends on:**
- `AI/CoreAI/CreatureAI` (see `ai-base.md`) — `SmartAI : CreatureAI`; **hard prerequisite**, the trait must exist before `SmartAI` can be implemented
- `AI/CoreAI/GameObjectAI` — for `SmartGameObjectAI`
- `AI/CoreAI/AreaTriggerAI` — for `SmartAreaTriggerAI`
- `Entities/Creature` and `Entities/Unit` — `me`, threat list, victim, motion master, faction
- `Entities/GameObject` — for `SmartGameObjectAI` actions (open/close door, set state, set flags)
- `Entities/AreaTrigger` — for area-trigger entry events
- `Spells` — `SMART_ACTION_CAST`, `SELF_CAST`, `INVOKER_CAST`, `CROSS_CAST`, `ADD_AURA`, `REMOVEAURASFROMSPELL`, `INTERRUPT_SPELL`, `PLAY_SPELL_VISUAL_KIT`
- `Combat` — `ThreatManager` (target resolution), `CALL_FOR_HELP`, `SET_IN_COMBAT_WITH_ZONE`, `ATTACK_START/STOP`, threat manipulation actions
- `Movement` — `MotionMaster::MovePath` (waypoints), `MovePoint` (`MOVE_TO_POS`), `MoveJump` (`JUMP_TO_POS`), `MoveOffset`, `WP_START/PAUSE/STOP/RESUME`
- `Maps` — grid visits for range-based targets and `SET_IN_COMBAT_WITH_ZONE`; instance state set/get (`SET_INST_DATA`, `SET_INST_DATA64`)
- `Database` — `smart_scripts` (the table), `creature_text` (talk lines, accessed via `CreatureTextMgr` from a sibling module), `waypoint_data` and `waypoints` (legacy), `creature_summon_groups` (for `SUMMON_CREATURE_GROUP`)
- `Conditions` — every event/action/target can have `Conditions` rows attached gating execution
- `CreatureText` (`Chat` module) — `SMART_ACTION_TALK`, `SIMPLE_TALK` consume `creature_text` rows
- `Quests` — `OFFER_QUEST`, `FAIL_QUEST`, `CALL_AREAEXPLOREDOREVENTHAPPENS`, `CALL_GROUPEVENTHAPPENS`, `CALL_KILLEDMONSTER`
- `Loot` — `GO_SET_LOOT_STATE`, kill-credit hooks
- `Achievements` — `CALL_KILLEDMONSTER` and quest credit feed achievement criteria
- `Gossip` — `CLOSE_GOSSIP`, `SEND_GOSSIP_MENU`, `GOSSIP_HELLO`/`GOSSIP_SELECT` events
- `Scenes` (3.4.3 has limited scenes, mostly for cinematic moments) — `SCENE_PLAY`, `SCENE_CANCEL`
- `GameEvents` — `GAME_EVENT_START` / `_STOP` / `_TRIGGER` events and actions
- `Conversations` (3.4.3 limited) — `CREATE_CONVERSATION`
- `Vehicles` — `SET_VEHICLE_ID`, vehicle passenger boarded/removed events

**Depended on by:**
- `AI/CoreAI/CreatureAISelector` (see `ai-base.md`) — checks for `smart_scripts` rows on a creature template/spawn and instantiates `SmartAI` if any exist
- `Scripts` (`scripts.md`) — bespoke C++ scripts often coexist with SmartScripts on the same encounter (boss with hand-rolled script + add waves driven by SmartScripts)
- Boss content authors / data team — the schema is the tooling target
- `CreatureText` mgr — consumed by SmartScripts at runtime, but does not itself depend on it

---

## 6. SQL / DB queries (if any)

SmartScripts is the heaviest data-driven module in the server. Tables it owns or consumes:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entryorguid, source_type, id, link, event_type, event_phase_mask, event_chance, event_flags, event_param1, event_param2, event_param3, event_param4, event_param5, action_type, action_param1, action_param2, action_param3, action_param4, action_param5, action_param6, action_param7, target_type, target_param1, target_param2, target_param3, target_param4, target_x, target_y, target_z, target_o, comment FROM smart_scripts ORDER BY entryorguid, source_type, id, link` | **The big one** — full table load on startup; every row becomes a `SmartScriptHolder`. ~50,000 rows in stock data, varies by content set | world |
| `SELECT id, point, position_x, position_y, position_z, orientation, delay, action, action_chance, wpguid FROM waypoint_data` | Modern WP paths used by `WP_START` / escort flows | world |
| `SELECT entry, pathid, point, position_x, position_y, position_z, point_comment FROM waypoints` | Legacy paths (still loaded in 3.4.3) | world |
| `SELECT entry, groupid, id, text, type, language, probability, emote, duration, sound, broadcast_text_id, comment FROM creature_text` | Talk lines used by `SMART_ACTION_TALK` / `SIMPLE_TALK` (loaded in `CreatureTextMgr` but consumed by SmartScripts) | world |
| `SELECT * FROM creature_text_locale` | Localization of `creature_text` | world |
| `SELECT id, summonerId, summonerType, groupId, entry, position_x, position_y, position_z, orientation, summonType, summonTime FROM creature_summon_groups` | Used by `SMART_ACTION_SUMMON_CREATURE_GROUP` | world |
| `SELECT * FROM conditions WHERE SourceTypeOrReferenceId = CONDITION_SOURCE_TYPE_SMART_EVENT` | Per-event conditions gating execution | world |
| (presence-only, not loaded as data) `creature_template.AIName='SmartAI'` | Selector signal — see `ai-base.md` | world |

The Rust workspace already has the prepared statement constant **registered but unused**: `crates/wow-database/src/statements/world.rs` defines `SEL_SMART_SCRIPTS` with `SELECT entryorguid, source_type, id, link, event_type, event_phase_mask, event_chance, event_flags, event_param1, event_param2, event_param3, event_param4, event_param5, action_type, action_param1..7, target_type, target_param1..3, target_x, target_y, target_z, target_o FROM smart_scripts ORDER BY entryorguid, source_type, id, link`. **Nothing executes it.**

No DB2/DBC stores belong to SmartScripts directly.

---

## 7. Wire-protocol packets (if any)

SmartScripts does not own opcodes; its actions cause traffic through other modules:

| Opcode | Direction | Sent by SmartScripts via |
|---|---|---|
| `SMSG_CHAT` (`SMSG_MESSAGECHAT`) | server → client | `SMART_ACTION_TALK`, `SIMPLE_TALK` (chat type `MONSTER_SAY` / `YELL` / `EMOTE` / `WHISPER`) |
| `SMSG_TEXT_EMOTE` | server → client | `creature_text` row with `type=TEXTEMOTE` |
| `SMSG_EMOTE` | server → client | `SMART_ACTION_PLAY_EMOTE`, `RANDOM_EMOTE`, `SET_EMOTE_STATE` |
| `SMSG_PLAY_SOUND` / `SMSG_PLAY_MUSIC` | server → client | `SMART_ACTION_SOUND`, `RANDOM_SOUND`, `creature_text.sound` |
| `SMSG_PLAY_SPELL_VISUAL_KIT` | server → client | `SMART_ACTION_PLAY_SPELL_VISUAL_KIT` |
| `SMSG_SPELL_*` (`SMSG_SPELL_GO`, `SMSG_SPELL_START`, `SMSG_SPELL_FAILED`) | server → client | `SMART_ACTION_CAST`, `SELF_CAST`, `INVOKER_CAST`, `CROSS_CAST` (via Spell pipeline) |
| `SMSG_AURA_UPDATE` | server → client | `SMART_ACTION_ADD_AURA`, `REMOVEAURASFROMSPELL` (via Aura pipeline) |
| `SMSG_ON_MONSTER_MOVE` | server → client | `SMART_ACTION_MOVE_TO_POS`, `RANDOM_MOVE`, `MOVE_OFFSET`, `JUMP_TO_POS`, `WP_START`, `TELEPORT` |
| `SMSG_AI_REACTION` | server → client | `ATTACK_START` (via combat pipeline) |
| `SMSG_GAMEOBJECT_CUSTOM_ANIM` | server → client | `SEND_GO_CUSTOM_ANIM` |
| `SMSG_PLAY_MOVIE` | server → client | `PLAY_MOVIE` |
| `SMSG_PLAY_CINEMATIC` | server → client | `PLAY_CINEMATIC` |
| `SMSG_GOSSIP_MESSAGE` / `SMSG_GOSSIP_COMPLETE` | server → client | `SEND_GOSSIP_MENU`, `CLOSE_GOSSIP` |
| `SMSG_QUEST_OFFER` family | server → client | `OFFER_QUEST` |
| `SMSG_INSTANCE_INFO` updates | server → client | `SET_INST_DATA`, `SET_INST_DATA64` (via instance broadcast) |
| `CMSG_AREATRIGGER` | client → server | drives `SMART_EVENT_AREATRIGGER_ONTRIGGER` |
| `CMSG_GOSSIP_HELLO` / `CMSG_GOSSIP_SELECT_OPTION` | client → server | drives `SMART_EVENT_GOSSIP_HELLO` / `GOSSIP_SELECT` |
| `CMSG_QUEST_GIVER_ACCEPT_QUEST` / `CMSG_QUEST_GIVER_CHOOSE_REWARD` | client → server | drives `SMART_EVENT_ACCEPTED_QUEST` / `REWARD_QUEST` / `QUEST_REWARDED` |
| `CMSG_AREA_SPIRIT_HEALER_QUEUE` etc. | client → server | indirectly via `JUST_SUMMONED` / `ON_SPELLCLICK` events |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-script/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `crates/wow-conditions` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-script/src/lib.rs` — **0 lines** (empty placeholder)
- `crates/wow-script/Cargo.toml` — workspace member; package builds but ships nothing
- `crates/wow-ai/src/lib.rs` — 346 lines (the single concrete `CreatureAI` from `ai-base.md`); no `SmartAI`, no `SmartScript`
- `crates/wow-database/src/statements/world.rs:15` — single constant `SEL_SMART_SCRIPTS` defining the load query, **never executed by any consumer**
- No `smart_event.rs`, no `smart_action.rs`, no `smart_target.rs`, no `smart_script_holder.rs`, no `smart_script_mgr.rs`, no `smart_script.rs`, no `smart_ai.rs`, no `smart_waypoint_mgr.rs`
- No tests, no fixtures, no sample data

**What's implemented:** Nothing. The crate is an empty shell.

**What's missing vs C++ (everything):**

1. **Enums.** `SmartScriptType` (13 variants), `SmartEvent` (~80), `SmartAction` (~150), `SmartTarget` (~30), `SmartEventPhase` (1–12 + `_ALWAYS`), `SmartEventPhaseBits` (12-bit mask + `_ALL`), `SmartEventFlags` (`NotRepeatable`, `Difficulty0..3`, `DebugOnly`, `DontReset`, `WhileCharmed`), `SmartCastFlags` (`InterruptPrevious`, `Triggered`, `ForceCast`, `NoCombatMove`, `AuraNotPresent`, `CombatMove`), `EscortState` (`None`, `Escorting`, `Returning`, `Paused`)
2. **Holder.** `struct SmartScriptHolder { entry_or_guid: i64, source_type: SmartScriptType, id: u32, link: u32, event: SmartEventDef, action: SmartActionDef, target: SmartTargetDef, timer: u32, priority: u32, run_once: bool, enable_timed: bool, comment: String }`
3. **Loader.** `SmartScriptMgr` singleton, `load_from_db()`, per-row `is_event_valid` validator (action_param schema vs action_type, target_param schema vs target_type, link existence, phase mask range, chance ≤100, entry/spawn existence, difficulty bit consistency)
4. **Cache.** `mEventMap: HashMap<SmartScriptType, HashMap<i64 entry_or_guid, Vec<SmartScriptHolder>>>` shared via `Arc`
5. **Waypoint loader.** `SmartWaypointMgr`: `waypoint_data` + `waypoints` → `mPaths: HashMap<u32 entry_or_path_id, WaypointPath>`
6. **Runtime `SmartScript`.** Per-instance state: `events: Vec<SmartScriptHolder>`, `current_phase: u8`, `current_phase_mask: u32`, `path_id: u32`, `last_invoker: ObjectGuid`, `counter_list: HashMap<u32,u32>`, `stored_targets: HashMap<u32, Vec<ObjectGuid>>`, `nested_events_counter: u32`, `talk_timer`, `last_text_id`
7. **`on_update(diff)`** event walker: tick `timer` per holder, fire when ready, `recalc_timer(min,max)` for repeating, dispatch `process_event`
8. **`process_events_for(event_kind, …)`** top-level dispatch for hook-driven events
9. **`process_event` switch (~80 arms).** Match each `SmartEvent` variant; check params (HP%, mana%, range, spell id, emote id, target buff id, etc.); roll chance; check phase mask; check `NOT_REPEATABLE` already-fired; if ok, call `process_action`
10. **`process_action` switch (~150 arms).** Each action's effect: cast spell, talk, move, summon, set phase, set data, set faction, set unit flag, despawn, kill, equip, mount, set visibility, set react state, etc.
11. **`get_targets` switch (~30 arms).** Resolve `SmartTarget` to `Vec<WorldObject>`: self, victim, hostile-random, action-invoker, closest-creature-in-range, stored-target-id, position, threat-list, etc.
12. **Phase mutation.** `set_phase(p)`, `inc_phase(p)`, `dec_phase(p)`, `is_in_phase(p)` updating `current_phase_mask` (bit `1 << (phase-1)`)
13. **Stored target lists.** `store_target_list(id, targets)`, `add_to_stored_target_list(id, targets)`, `get_stored_target_vector(id, ref) -> &Vec<ObjectGuid>` for use by later actions resolving `SMART_TARGET_STORED`
14. **Counters.** `store_counter(id, value, reset)`, `get_counter_value(id) -> u32` for boss state machines (e.g. add waves)
15. **TimedActionList.** `set_timed_action_list(holder, entry, invoker, start_from)` swap event list to a parameterised script entry
16. **Reset.** `on_reset()` reprocess `SMART_EVENT_RESET`, restore initial events, clear counters/stored targets per flag rules
17. **`SmartAI`.** `CreatureAI` impl wrapping `SmartScript`; forward every hook (`just_entered_combat`, `just_engaged_with`, `just_died`, `killed_unit`, `spell_hit`, `spell_hit_target`, `just_summoned`, `summoned_creature_dies`, `move_in_line_of_sight`, `receive_emote`, `movement_inform`, `damage_taken`, `on_spell_click`, `waypoint_reached`, `waypoint_path_ended`) to `process_events_for(<right SmartEvent>)`
18. **Waypoint integration.** `start_path(run, path_id, repeat, invoker, node_id)`, `pause_path(delay, forced)`, `resume_path()`, `end_path(fail)`, `waypoint_reached`, `waypoint_path_ended`; tie into `MotionMaster::MovePath` (Movement crate)
19. **Escort flow.** invoker tracking, max-distance check (despawn on player abandon), quest credit on `EndPath(false)`
20. **Follow flow.** `SMART_ACTION_FOLLOW` to escort temporary companions
21. **`SmartGameObjectAI` and `SmartAreaTriggerAI`** parallel runtimes for GameObject and AreaTrigger source types
22. **Conditions integration.** Each event holder may have `Conditions` rows (`SourceTypeOrReferenceId=CONDITION_SOURCE_TYPE_SMART_EVENT`) gating execution; runtime must call into `wow-conditions` (also unimplemented)
23. **CreatureText integration.** `SMART_ACTION_TALK` / `SIMPLE_TALK` reads `creature_text` group_id and emits `SMSG_CHAT` (group probability roll, emote, sound, duration, broadcast_text_id, locale)
24. **Nested-events guard.** `MAX_NESTED_EVENTS=10` check; an action that triggers another event (e.g. cast → spell hit → event) cannot recurse beyond 10 deep, otherwise abort
25. **Event link chains.** `link != 0` points to another holder by `id` within the same `entry_or_guid`; firing the head fires the entire linked tail without separate event matches
26. **Tests.** Zero. No fixtures of `smart_scripts` rows, no golden behaviour traces, no validation tests.

**Suspicious / likely divergent:** N/A — there is nothing to diverge. The whole engine is unbuilt. The single `SEL_SMART_SCRIPTS` constant has no consumer, so even the SQL loading half is dead code.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#AI_SMARTSCRIPTS.WBS.001** Partir y cerrar la migracion auditada de `game/AI/SmartScripts/SmartAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartAI.cpp`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1259 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_SMARTSCRIPTS.WBS.002** Cerrar la migracion auditada de `game/AI/SmartScripts/SmartAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartAI.h`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_SMARTSCRIPTS.WBS.003** Partir y cerrar la migracion auditada de `game/AI/SmartScripts/SmartScript.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4253 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_SMARTSCRIPTS.WBS.004** Cerrar la migracion auditada de `game/AI/SmartScripts/SmartScript.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.h`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_SMARTSCRIPTS.WBS.005** Partir y cerrar la migracion auditada de `game/AI/SmartScripts/SmartScriptMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2497 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_SMARTSCRIPTS.WBS.006** Partir y cerrar la migracion auditada de `game/AI/SmartScripts/SmartScriptMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h`
  Rust target: `crates/wow-script`, `crates/wow-ai`, `crates/wow-conditions`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1769 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** <1h, **M** 1–4h, **H** 4–12h, **XL** >12h (split before tackling). Many of these are XL and **must** be split into smaller PRs in execution.

**Foundation — enums + holder + loader (Wave A):**

- [ ] **#AI-SS.1** `enum SmartScriptType { Creature=0, GameObject, AreaTrigger, Event, Gossip, Quest, Spell, Transport, Instance, TimedActionList, Scene, AreaTriggerEntity, AreaTriggerEntityServerSide, Max }` (L)
- [ ] **#AI-SS.2** `enum SmartEvent` with all ~80 variants in `crates/wow-script/src/smart_event.rs`. Use data-carrying variants (e.g. `HealthPct { hp_min: u8, hp_max: u8, repeat_min: u32, repeat_max: u32 }`) rather than raw u32 params for type-safety. Provide `from_raw(event_type: u32, params: [u32; 5]) -> Result<Self, ParseError>` (XL — split by event family: combat, movement, quest, transport, instance, scene, etc.)
- [ ] **#AI-SS.3** `enum SmartAction` with all ~150 variants in `crates/wow-script/src/smart_action.rs`. Same pattern: data-carrying variants + `from_raw(action_type: u32, params: [u32; 7]) -> Result<Self, ParseError>` (XL — split by action family: talk/sound/emote, cast/aura, movement, summon, faction/flags, instance, gossip, quest, GO state, scene/cinematic, vehicles)
- [ ] **#AI-SS.4** `enum SmartTarget` with all ~30 variants in `crates/wow-script/src/smart_target.rs`. Data-carrying. `from_raw(target_type: u32, params: [u32; 4], xyzo: [f32; 4]) -> Result<Self, ParseError>` (H)
- [ ] **#AI-SS.5** `enum SmartEventPhase` (0..=13), `bitflags! struct SmartEventPhaseBits: u32` (12 bits + `_ALL`), `bitflags! struct SmartEventFlags: u16`, `bitflags! struct SmartCastFlags: u32` (M)
- [ ] **#AI-SS.6** `struct SmartScriptHolder { entry_or_guid: i64, source_type: SmartScriptType, id: u32, link: u32, event: SmartEvent, event_phase_mask: SmartEventPhaseBits, event_chance: u8, event_flags: SmartEventFlags, action: SmartAction, target: SmartTarget, timer: u32, priority: u32, run_once: bool, enable_timed: bool, comment: String }` (M)
- [ ] **#AI-SS.7** SQL loader: implement consumer of the existing `SEL_SMART_SCRIPTS` prepared statement; map each row to `SmartScriptHolder::from_row(...)`; collect into `HashMap<SmartScriptType, HashMap<i64, Vec<SmartScriptHolder>>>` (M)
- [ ] **#AI-SS.8** Per-row validator `is_event_valid(holder) -> Result<(), ValidationError>`: check action params match action variant, target params match target variant, link target exists in same `entry_or_guid`, phase mask ≤ 12 bits, chance ≤ 100, `entry_or_guid` references existing template (positive) or spawn (negative), difficulty bits consistent. Reject rows on validation failure with `tracing::warn!` (H)
- [ ] **#AI-SS.9** `SmartScriptMgr` singleton (`OnceLock<Arc<SmartScriptMgr>>`); `load_from_db(pool)`, `get_script(entry_or_guid, type) -> Option<&[SmartScriptHolder]>`; load all script types in one pass (H)
- [ ] **#AI-SS.10** `SmartWaypointMgr`: load `waypoint_data` and `waypoints`; `get_path(id) -> Option<&WaypointPath>`; `WaypointPath { nodes: Vec<WaypointNode> }`, `WaypointNode { id, x, y, z, o, delay, action_id, action_chance }` (M)
- [ ] **#AI-SS.11** Loader benchmarks + parallel parse with `rayon` (50k rows benefit measurably) (L)

**Runtime — `SmartScript` interpreter (Wave B):**

- [ ] **#AI-SS.12** `struct SmartScript { events: Vec<SmartScriptHolder>, install_events: Vec<SmartScriptHolder>, timed_action_list: Vec<SmartScriptHolder>, stored_events: HashMap<u32, SmartScriptHolder>, last_invoker: ObjectGuid, counter_list: HashMap<u32, u32>, stored_targets: HashMap<u32, Vec<ObjectGuid>>, current_phase: u8, current_phase_mask: SmartEventPhaseBits, path_id: u32, talk_timer: u32, last_text_id: u32, talker_entry: u32, use_text_timer: bool, current_priority: u32, event_sorting_required: bool, nested_events_counter: u32, all_event_flags: SmartEventFlags, script_type: SmartScriptType }` (M)
- [ ] **#AI-SS.13** `SmartScript::on_initialize(obj, …)`: fetch event list from `SmartScriptMgr`; sort by id; init timers (M)
- [ ] **#AI-SS.14** Timer arithmetic: `init_timer`, `update_timer(holder, diff)`, `recalc_timer(holder, min, max)` (L)
- [ ] **#AI-SS.15** `on_update(diff)`: walk events, advance timers, fire ready events (M)
- [ ] **#AI-SS.16** `process_events_for(event_kind, unit, var0, var1, bvar, spell, gob, var_string)` top-level dispatch (M)
- [ ] **#AI-SS.17** `process_event` switch — chunk 1: combat events (UPDATE_IC, UPDATE_OOC, AGGRO, KILL, DEATH, EVADE, DAMAGED, DAMAGED_TARGET, RECEIVE_HEAL, HEALTH_PCT, MANA_PCT, TARGET_HEALTH_PCT, TARGET_MANA_PCT, RANGE, SPELLHIT, SPELLHIT_TARGET, ON_SPELL_CAST, ON_SPELL_FAILED, ON_SPELL_START, VICTIM_CASTING, TARGET_BUFFED, HAS_AURA, IS_BEHIND_TARGET) (H)
- [ ] **#AI-SS.18** `process_event` switch — chunk 2: movement & life-cycle events (RESPAWN, REACHED_HOME, RESET, JUST_CREATED, AI_INIT, ON_DESPAWN, MOVEMENTINFORM, WAYPOINT_START, WAYPOINT_REACHED, WAYPOINT_PAUSED, WAYPOINT_RESUMED, WAYPOINT_STOPPED, WAYPOINT_ENDED, FOLLOW_COMPLETED, OOC_LOS, IC_LOS) (H)
- [ ] **#AI-SS.19** `process_event` switch — chunk 3: summon/charm/passenger events (SUMMONED_UNIT, JUST_SUMMONED, SUMMONED_UNIT_DIES, SUMMON_DESPAWNED, CHARMED, CHARMED_TARGET, PASSENGER_BOARDED, PASSENGER_REMOVED, CORPSE_REMOVED) (M)
- [ ] **#AI-SS.20** `process_event` switch — chunk 4: quest/gossip/scene events (ACCEPTED_QUEST, REWARD_QUEST, QUEST_OBJ_COMPLETION, QUEST_COMPLETION, QUEST_FAIL, QUEST_REWARDED, GOSSIP_HELLO, GOSSIP_SELECT, RECEIVE_EMOTE, TEXT_OVER, ACTION_DONE, DATA_SET, GAME_EVENT_START, GAME_EVENT_END, AREATRIGGER_ONTRIGGER, ON_SPELLCLICK, INSTANCE_PLAYER_ENTER, SCENE_START, SCENE_TRIGGER, SCENE_CANCEL, SCENE_COMPLETE) (H)
- [ ] **#AI-SS.21** `process_event` switch — chunk 5: misc/transport events (TIMED_EVENT_TRIGGERED, UPDATE, LINK, FRIENDLY_HEALTH, FRIENDLY_HEALTH_PCT, FRIENDLY_IS_CC, FRIENDLY_MISSING_BUFF, DISTANCE_CREATURE, DISTANCE_GAMEOBJECT, COUNTER_SET, EVENT_PHASE_CHANGE, GO_LOOT_STATE_CHANGED, GO_EVENT_INFORM, TRANSPORT_*, SEND_EVENT_TRIGGER) (M)
- [ ] **#AI-SS.22** Event chance roll (`event_chance`), `NOT_REPEATABLE` already-fired tracking, `DEBUG_ONLY` skip in release, difficulty bit gating, `WHILE_CHARMED` flag, `DONT_RESET` flag (M)
- [ ] **#AI-SS.23** Phase mask gating (`current_phase_mask & holder.event_phase_mask != 0`) on every event (L)
- [ ] **#AI-SS.24** `link` chain dispatch — when an event fires, walk linked siblings and fire their actions inline (M)
- [ ] **#AI-SS.25** `process_action` switch — chunk 1: talk/sound/emote (TALK, SIMPLE_TALK, SOUND, RANDOM_SOUND, PLAY_EMOTE, RANDOM_EMOTE, SET_EMOTE_STATE, PLAY_MOVIE, PLAY_CINEMATIC, PLAY_ANIMKIT, PLAY_SPELL_VISUAL_KIT, SCENE_PLAY, SCENE_CANCEL) — depends on `wow-chat` `CreatureTextMgr` (H)
- [ ] **#AI-SS.26** `process_action` switch — chunk 2: cast/aura (CAST, SELF_CAST, INVOKER_CAST, CROSS_CAST, ADD_AURA, REMOVEAURASFROMSPELL, INTERRUPT_SPELL, FORCE_DESPAWN_SPELL_CHANNEL) — depends on `wow-spell` (H)
- [ ] **#AI-SS.27** `process_action` switch — chunk 3: movement (MOVE_TO_POS, RANDOM_MOVE, MOVE_OFFSET, JUMP_TO_POS, WP_START, WP_PAUSE, WP_STOP, WP_RESUME, START_CLOSEST_WAYPOINT, TELEPORT, SET_RUN, SET_FLY, SET_SWIM, SET_DISABLE_GRAVITY, SET_ROOT, SET_HOVER, PAUSE_MOVEMENT, ALLOW_COMBAT_MOVEMENT, SET_HOME_POS, SET_ORIENTATION, SET_MOVEMENT_SPEED, SET_RANGED_MOVEMENT, FLEE_FOR_ASSIST, RISE_UP) (H)
- [ ] **#AI-SS.28** `process_action` switch — chunk 4: summon/spawn (SUMMON_CREATURE, SUMMON_GO, SUMMON_CREATURE_GROUP, SPAWN_SPAWNGROUP, DESPAWN_SPAWNGROUP, RESPAWN_BY_SPAWNID, REMOVE_ALL_GAMEOBJECTS) (M)
- [ ] **#AI-SS.29** `process_action` switch — chunk 5: faction/flags/state (SET_FACTION, CHANGE_FACTION, SET_REACT_STATE, SET_UNIT_FLAG, REMOVE_UNIT_FLAG, SET_NPC_FLAG, ADD_NPC_FLAG, REMOVE_NPC_FLAG, SET_DYNAMIC_FLAG, ADD_DYNAMIC_FLAG, REMOVE_DYNAMIC_FLAG, SET_GO_FLAG, ADD_GO_FLAG, REMOVE_GO_FLAG, GO_SET_LOOT_STATE, GO_SET_GO_STATE, ACTIVATE_GAMEOBJECT, ACTIVATE_GOBJECT, RESET_GOBJECT, SET_VISIBILITY, SET_ACTIVE, SET_VEHICLE_ID, SET_UNINTERACTIBLE, SET_IMMUNE_PC, SET_IMMUNE_NPC) (H)
- [ ] **#AI-SS.30** `process_action` switch — chunk 6: combat/threat (ATTACK_START, ATTACK_STOP, AUTO_ATTACK, COMBAT_STOP, EVADE, CALL_FOR_HELP, SET_IN_COMBAT_WITH_ZONE, ZONE_COMBAT, THREAT_SINGLE_PCT, THREAT_ALL_PCT, ADD_THREAT, KILL_UNIT, DIE, SET_HEALTH_PCT, SET_INVINCIBILITY_HP_LEVEL, SET_HEALTH_REGEN, SET_POWER, ADD_POWER, REMOVE_POWER, MORPH_TO_ENTRY_OR_MODEL, MOUNT_TO_ENTRY_OR_MODEL, UPDATE_TEMPLATE, LOAD_EQUIPMENT, EQUIP, SET_SHEATH, SET_AI_ANIM_KIT) (H)
- [ ] **#AI-SS.31** `process_action` switch — chunk 7: phase/event control (SET_EVENT_PHASE, INC_EVENT_PHASE, RESET_EVENT_PHASE, RANDOM_PHASE, RANDOM_PHASE_RANGE, SET_DATA, SET_COUNTER, INSTALL_AI_TEMPLATE, CREATE_TIMED_EVENT, TRIGGER_TIMED_EVENT, REMOVE_TIMED_EVENT, TRIGGER_RANDOM_TIMED_EVENT, CALL_TIMED_ACTIONLIST, CALL_RANDOM_TIMED_ACTIONLIST, CALL_RANDOM_RANGE_TIMED_ACTIONLIST, OVERRIDE_SCRIPT_BASE_OBJECT, RESET_SCRIPT_BASE_OBJECT, CALL_SCRIPT_RESET) (H)
- [ ] **#AI-SS.32** `process_action` switch — chunk 8: quest/instance/items (FAIL_QUEST, OFFER_QUEST, CALL_AREAEXPLOREDOREVENTHAPPENS, CALL_KILLEDMONSTER, CALL_GROUPEVENTHAPPENS, ADD_ITEM, REMOVE_ITEM, ACTIVATE_TAXI, SEND_TAXI, SET_INST_DATA, SET_INST_DATA64, STORE_TARGET_LIST, ADD_TO_STORED_TARGET_LIST, SEND_GOSSIP_MENU, CLOSE_GOSSIP, SEND_TARGET_TO_TARGET, SEND_GO_CUSTOM_ANIM, SEND_CUSTOM_EVENT, GAME_EVENT_START, GAME_EVENT_STOP, TRIGGER_GAME_EVENT, OVERRIDE_LIGHT, OVERRIDE_WEATHER, SET_INGAME_PHASE_GROUP, CREATE_CONVERSATION, BECOME_PERSONAL_CLONE_FOR_PLAYER, ENABLE_TEMP_GOBJ, FOLLOW, DO_ACTION, DISABLE_EVADE, SET_CORPSE_DELAY, SET_UNIT_FIELD_BYTES_1, REMOVE_UNIT_FIELD_BYTES_1) (XL)
- [ ] **#AI-SS.33** `get_targets` — chunk 1: self/victim/threat-list (NONE, SELF, VICTIM, HOSTILE_SECOND_AGGRO, HOSTILE_LAST_AGGRO, HOSTILE_RANDOM, HOSTILE_RANDOM_NOT_TOP, ACTION_INVOKER, ACTION_INVOKER_VEHICLE, OWNER_OR_SUMMONER, THREAT_LIST, LOOT_RECIPIENTS) (M)
- [ ] **#AI-SS.34** `get_targets` — chunk 2: range/distance/closest (POSITION, CREATURE_RANGE, CREATURE_DISTANCE, CREATURE_GUID, GAMEOBJECT_RANGE, GAMEOBJECT_DISTANCE, GAMEOBJECT_GUID, PLAYER_RANGE, PLAYER_DISTANCE, CLOSEST_CREATURE, CLOSEST_GAMEOBJECT, CLOSEST_PLAYER, CLOSEST_ENEMY, CLOSEST_FRIENDLY, FARTHEST) — depends on grid visit (H)
- [ ] **#AI-SS.35** `get_targets` — chunk 3: stored/party/vehicle (STORED, INVOKER_PARTY, VEHICLE_PASSENGER) (M)
- [ ] **#AI-SS.36** Stored target list / counter APIs: `store_target_list`, `add_to_stored_target_list`, `get_stored_target_vector`, `store_counter`, `get_counter_value` (M)
- [ ] **#AI-SS.37** Phase mutation methods: `set_phase`, `inc_phase`, `dec_phase`, `is_in_phase` (L)
- [ ] **#AI-SS.38** TimedActionList: `set_timed_action_list(holder, entry, invoker, start_from_event_id)` swap event list (M)
- [ ] **#AI-SS.39** Reset flow: `on_reset()` reprocess `SMART_EVENT_RESET`, restore initial events (excluding `DONT_RESET`), clear counters/stored targets per flag (M)
- [ ] **#AI-SS.40** Nested-events guard `MAX_NESTED_EVENTS=10`; abort + warn when exceeded (L)

**SmartAI host (Wave C — depends on `ai-base.md` traits being present):**

- [ ] **#AI-SS.41** `struct SmartAI` impl `CreatureAI` from `ai-base.md`: holds `script: SmartScript`, escort state (`escort_state: EscortState`, `escort_invoker: ObjectGuid`, `escort_max_dist: f32`, `escort_quest_id: u32`), waypoint state (`wp_pause_timer: u32`, `wp_repeat: bool`, `wp_run: bool`, `last_wp_id: u32`), `evade_disabled: bool`, `combat_move_allowed: bool` (M)
- [ ] **#AI-SS.42** Forward every CreatureAI hook from `ai-base.md` to `script.process_events_for(<right SmartEvent>)`: `Reset`, `JustEnteredCombat → UPDATE_IC` (no, that's UpdateAI; map to `AGGRO`), `JustEngagedWith → AGGRO`, `JustDied → DEATH`, `KilledUnit → KILL`, `SpellHit → SPELLHIT`, `SpellHitTarget → SPELLHIT_TARGET`, `JustSummoned → JUST_SUMMONED`, `SummonedCreatureDies → SUMMONED_UNIT_DIES`, `SummonedCreatureDespawn → SUMMON_DESPAWNED`, `MoveInLineOfSight → OOC_LOS / IC_LOS`, `ReceiveEmote → RECEIVE_EMOTE`, `MovementInform → MOVEMENTINFORM` + WP-specific events, `DamageTaken → DAMAGED`, `DamageDealt → DAMAGED_TARGET`, `HealReceived → RECEIVE_HEAL`, `OnDespawn → ON_DESPAWN`, `JustReachedHome → REACHED_HOME`, `EnterEvadeMode → EVADE`, `OnCharmed → CHARMED`, `OnGameEvent → GAME_EVENT_START / END`, `JustAppeared → JUST_CREATED + AI_INIT`, `OnSpellClick → ON_SPELLCLICK` (H)
- [ ] **#AI-SS.43** `start_path / pause_path / resume_path / end_path / waypoint_reached / waypoint_path_ended` — wire to `MotionMaster::move_path` from movement crate (M)
- [ ] **#AI-SS.44** Escort flow: track `escort_invoker` GUID, max distance check each tick, on `EndPath(false)` give quest credit, on abandon despawn (M)
- [ ] **#AI-SS.45** Follow flow: `SMART_ACTION_FOLLOW` set `MotionMaster::move_follow` and listen for `FOLLOW_COMPLETED` (M)
- [ ] **#AI-SS.46** `struct SmartGameObjectAI` (parallel host, GameObject source type) (H)
- [ ] **#AI-SS.47** `struct SmartAreaTriggerAI` (parallel host, AreaTrigger source type) (M)

**Integrations (Wave D):**

- [ ] **#AI-SS.48** Wire selector from `ai-base.md`: when `SmartScriptMgr::get_script(creature.entry, Creature)` is non-empty (or for spawn GUID `-creature.spawn_id`), `select_ai` returns `Box::new(SmartAI::new(creature))` (M, blocked on #AI-BASE.25)
- [ ] **#AI-SS.49** `CreatureTextMgr` consumer: load `creature_text` (group_id rows with `text`, `type`, `language`, `probability`, `emote`, `duration`, `sound`, `broadcast_text_id`); `send_chat(creature, group_id, target?)` rolls probability per text id; emit `SMSG_CHAT` + sound + emote (H — depends on `wow-chat`)
- [ ] **#AI-SS.50** `Conditions` integration: per-event Conditions evaluation; depends on `wow-conditions` (M, blocked)
- [ ] **#AI-SS.51** Border with `scripts.md`: register `inventory::submit!` factory `SmartAI` under name `"SmartAI"` so `creature_template.AIName='SmartAI'` resolves (in addition to the data-presence shortcut) (L)

**Tests & validation (Wave E):**

- [ ] **#AI-SS.52** Per-enum `from_raw` round-trip tests (every variant) (M)
- [ ] **#AI-SS.53** Loader fixture: synthetic `smart_scripts` rows for each event type → assert correctly parsed and validated (M)
- [ ] **#AI-SS.54** Validator fixtures: malformed rows → `is_event_valid` returns the right error (L)
- [ ] **#AI-SS.55** Runtime smoke test: 1-row script `AGGRO → TALK group=0 → target=SELF` fires once on engage, exactly once (M)
- [ ] **#AI-SS.56** Phase test: 3-phase boss (HP-pct triggers `INC_EVENT_PHASE`); events tagged phase 1 don't fire in phase 2 (M)
- [ ] **#AI-SS.57** Link-chain test: head event with `link=2`; firing head also fires holder id=2's action (M)
- [ ] **#AI-SS.58** Statistical test for `event_chance=50` (~500/1000 fires) (L)
- [ ] **#AI-SS.59** `NOT_REPEATABLE` flag — fires once across encounter (L)
- [ ] **#AI-SS.60** Stored target list — store on event 1, consume on event 2 → resolves to same units (M)
- [ ] **#AI-SS.61** Counter test — increments and matches `COUNTER_SET` event (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#AI_SMARTSCRIPTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 10284 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h`. Rust target: `crates/wow-ai`, `crates/wow-script`. | `cargo test -p wow-ai && cargo test -p wow-script` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#AI_SMARTSCRIPTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 10284 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h`. Rust target: `crates/wow-ai`, `crates/wow-script`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#AI_SMARTSCRIPTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 10284 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h`. Rust target: `crates/wow-ai`, `crates/wow-script`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#AI_SMARTSCRIPTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 10284 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h`. Rust target: `crates/wow-ai`, `crates/wow-script`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `SmartScriptMgr::load_from_db` loads a fixture of N rows and rejects M malformed rows with the expected `ValidationError` per row
- [ ] Test: `SmartScript.process_event(AGGRO)` fires matching action when boss enters combat
- [ ] Test: `SmartScript.process_event(HEALTH_PCT, 30, 50)` fires when HP enters [30,50]% range
- [ ] Test: `SmartScript.process_event(HEALTH_PCT, 30, 30)` fires only at exactly 30%
- [ ] Test: `SmartScript.process_action(TALK, group_id=0)` emits `SMSG_CHAT` with text from `creature_text` (probability-rolled)
- [ ] Test: `SmartScript.process_action(CAST, spell_id=12345, flags=TRIGGERED)` casts on resolved target with `TRIGGERED` flag
- [ ] Test: `SmartScript.process_action(SUMMON_CREATURE, entry=666)` spawns creature at `target_xyzo`
- [ ] Test: `SmartScript.process_action(SET_EVENT_PHASE, p=2)` sets `current_phase=2` and `current_phase_mask=bit(2)`
- [ ] Test: `SmartScript.process_action(INC_EVENT_PHASE, 1)` increments phase
- [ ] Test: `SmartScript.process_action(STORE_TARGET_LIST, id=7)` stores resolved targets under id 7
- [ ] Test: `SmartScript.process_action(KILL_SELF)` triggers `JustDied` (which is itself a `SMART_EVENT_DEATH`)
- [ ] Test: `SmartScript.process_action(FORCE_DESPAWN, time=0)` despawns the creature immediately
- [ ] Test: `SmartScript.get_targets(VICTIM)` returns `[caster.victim]`
- [ ] Test: `SmartScript.get_targets(CLOSEST_PLAYER, range=50)` returns the single nearest player ≤50yd
- [ ] Test: `SmartScript.get_targets(THREAT_LIST)` returns full threat list ordered by threat
- [ ] Test: `SmartScript.get_targets(STORED, id=7)` returns previously stored list 7
- [ ] Test: `SmartScript.get_targets(HOSTILE_RANDOM_NOT_TOP)` excludes top-threat unit
- [ ] Test: `event_chance=50` fires ~500/1000 with seeded RNG
- [ ] Test: `SmartEventFlags::NotRepeatable` — event fires once per encounter, not again until reset
- [ ] Test: `SmartEventFlags::DontReset` — event survives `OnReset()` clear
- [ ] Test: `SmartEventFlags::DebugOnly` — event skipped in release builds
- [ ] Test: phase mask `0b101` (phases 1 and 3) — fires in phase 1 and 3, skipped in phase 2
- [ ] Test: link chain — head event with `link=5` fires holder id=5's action without re-matching event
- [ ] Test: `MAX_NESTED_EVENTS=10` — recursive event chain aborts at depth 11 with warning
- [ ] Test: `SmartAI::just_engaged_with` triggers `process_events_for(AGGRO)` exactly once
- [ ] Test: `SmartAI::movement_inform(WAYPOINT_MOTION_TYPE, node_id)` triggers `WAYPOINT_REACHED`
- [ ] Test: `SmartAI::start_path` then `pause_path(5_000)` — script paused 5s, then resumes at next node
- [ ] Test: `SmartAI::end_path(fail=false)` gives quest credit to escort invoker
- [ ] Test: `SmartAI::end_path(fail=true)` despawns without credit
- [ ] Test: Selector — creature with rows in `smart_scripts` instantiates `SmartAI`, not `NullCreatureAI`
- [ ] Test: 3-phase boss fixture — full encounter walkthrough hitting events in correct phases produces expected action sequence
- [ ] Test: `creature_template.AIName='SmartAI'` factory route returns `SmartAI` regardless of data presence
- [ ] Test: `SmartAreaTriggerAI` fires `AREATRIGGER_ONTRIGGER` when player enters trigger zone

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 6 files / 10284 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h` | `crates/wow-script/` (engine), consumes traits from `crates/wow-ai/` \| ❌ not started — `crates/wow-script/` is **0 lines**; the only trace of SmartScripts in the workspace is one unused SQL prepared-statement constant |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#AI_SMARTSCRIPTS.DIV.001` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 6 C++ files / 10284 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#AI_SMARTSCRIPTS.DIV.002` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 6 C++ files / 10284 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#AI_SMARTSCRIPTS.DIV.003` | `crates/wow-conditions` (`missing_declared_path`, 0 Rust lines) | 6 C++ files / 10284 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScript.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/SmartScriptMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **The validator is not optional.** Stock `smart_scripts` data ships with malformed rows. C++ `IsEventValid` rejects them with logs at startup, and the runtime never sees them. Skipping validation in Rust **will crash mid-encounter** (out-of-bounds `action_param` indices, null target resolution, division-by-zero on chance, …). Validation must come before runtime exposure.
- **`entry_or_guid` is signed.** Positive = creature_template.entry (script applies to all spawns of that template). Negative = creature.guid (only that specific spawn). The cache map keys on the signed value — do not cast to `u64`.
- **`SmartScriptType` per-row.** A single creature can have rows of `source_type=Creature` (its main script) **and** rows of `source_type=TimedActionList` referenced via `CALL_TIMED_ACTIONLIST`. The `entry_or_guid` of a TimedActionList row is independent (typically 0xFFFF_FFFF + something). Loader must group by `(source_type, entry_or_guid)` separately, not flatten.
- **`event_phase_mask` semantics.** Bit position N corresponds to `SMART_EVENT_PHASE_N` (1-indexed). `phase_mask=5 = bit 0 + bit 2 = phase 1 + phase 3`. There is also a 0 = "always" interpretation, **not** "never". Off-by-one here is a recurring source of bugs.
- **`SMART_EVENT_PHASE` ≠ `PhaseShift`.** The 12 boss phases are completely unrelated to `PhasingHandler`/`PhaseShift` (zone/quest visibility). Sharing the word "phase" is a known footgun.
- **Talk timer (`mTextTimer`).** While a `SMART_ACTION_TALK` is being read by the client (length comes from `creature_text.duration`), the `SmartScript` blocks subsequent talk actions on the same group and emits `SMART_EVENT_TEXT_OVER` when the duration expires. The runtime tracks `mTextTimer`/`mLastTextID`/`mTalkerEntry`/`mUseTextTimer`. If you skip this state, multi-line boss soliloquies overlap on the client.
- **Re-entrancy via `mNestedEventsCounter`.** An action can trigger an event that runs more actions, recursively. C++ handles this on the call stack; `MAX_NESTED_EVENTS=10` is a hard cap. Rust borrow rules forbid recursion through a `&mut SmartScript`, so the implementation must either (a) defer triggered events to a queue and drain after the current action returns, or (b) use `RefCell`. **The deferred-queue approach is preferred** — easier to reason about and avoids `RefCell` borrow panics in nested actions.
- **Concurrency model.** `SmartScript` is single-threaded per map (the map updater drives all creatures sequentially). No locks needed *within* a map, but the `Arc<SmartScriptMgr>` cache is shared across maps and must be immutable post-load. Hot reload would require additional design.
- **`mLastInvoker` lifetime.** Stored as `ObjectGuid` (not pointer) — the invoker can despawn between event and action. Resolution at action time may yield `None`; actions must handle gracefully (log and skip, do not panic).
- **Stored target lists scope.** Per-instance (per `SmartScript`), not global. They survive across events within an encounter and clear on `OnReset()`. A boss can store the tank on engage and reference it on subsequent `process_action` rounds.
- **Counters scope.** Same — per-instance. Useful for "after 3 of these adds die, do X" patterns (`SMART_EVENT_COUNTER_SET`).
- **`creature_text` probability roll.** Each `creature_text` row has a `probability` field (1–100). When `SMART_ACTION_TALK` selects a `group_id` with multiple rows, it weighted-rolls between them. Test boss talks empirically over 1000 fires to validate.
- **`creature_text.broadcast_text_id`** if non-zero overrides the inline `text` column with a localized client-side string. WotLK 3.4.3 doesn't always have the broadcast text resolved on the client; ensure both columns are emitted.
- **`SMART_TARGET_STORED` requires prior `SMART_ACTION_STORE_TARGET_LIST`.** Action ordering matters at runtime — `id` (sort key) and `link` (chain order) define when each holder runs. If a `STORED` target is consumed before a `STORE_TARGET_LIST` runs, you get an empty list. The validator should warn but does not enforce ordering across events (only within a chain).
- **`SMART_ACTION_CAST` with `SmartCastFlags::NoCombatMove`.** Disables combat-chase movement during the cast (caster mob standing still while channeling). Re-enable on cast end via `ALLOW_COMBAT_MOVEMENT`. Easy to forget the re-enable and end up with a permanent statue.
- **`OOC_LOS` vs `IC_LOS`.** Out-of-combat LOS fires when a unit enters LOS while we are not in combat (typical "noticed you" trigger). In-combat LOS fires while we are in combat (rare; e.g. a boss adding a new add to its threat list when it sees them). Confusing them produces sub-pulls during combat.
- **`SMART_EVENT_VICTIM_CASTING`.** Fires when our victim starts casting a spell. Used for interrupt rotations. Param decides which spell IDs (or any). Triggering interrupts is action-side via `SMART_ACTION_INTERRUPT_SPELL`.
- **Difficulty bits.** `SmartEventFlags::Difficulty0..3` filter the event by current map difficulty (Normal/Heroic/10/25). All bits 0 = always. Loader must respect these — failing to filter means heroic-only mechanics fire on normal.
- **WotLK 3.4.3 enum subset.** Stock TC mainline has events/actions added in MoP/WoD/Legion (e.g. `BECOME_PERSONAL_CLONE_FOR_PLAYER`, scenes, scenarios, conversations, area triggers entity). 3.4.3 has a smaller set. If parsing data extracted from mainline TC, filter unknown action/event types at load — do not throw, just skip with a warning. Stick to the action/event IDs that the WotLK Classic client supports.
- **Performance note.** 50k rows × per-row validation can take several seconds in C++. Rust with `rayon` can parallelize the validation pass; the load itself is one sequential SQL query. Expect 100–500ms total in release after parallelization.
- **Hot reload considerations.** Live reloading a single creature's script requires (a) re-running the loader for that `entry_or_guid`, (b) rebuilding the cached `Arc<HashMap>` (or using a per-entry `OnceLock` partition), (c) resetting active `SmartScript` instances on that entry. Not a Wave A concern, but design the loader to allow per-entry refresh later.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class SmartAI : CreatureAI` | `struct SmartAI { script: SmartScript, escort: EscortState, ... }` impl `CreatureAI` | composition; depends on `ai-base.md` |
| `class SmartGameObjectAI : GameObjectAI` | `struct SmartGameObjectAI { script: SmartScript }` impl `GameObjectAI` | parallel host |
| `class SmartAreaTriggerAI : AreaTriggerAI` | `struct SmartAreaTriggerAI { script: SmartScript }` impl `AreaTriggerAI` | parallel host |
| `class SmartScript` | `struct SmartScript { events, install_events, timed_action_list, stored_events, last_invoker, counter_list, stored_targets, current_phase, current_phase_mask, path_id, talk_timer, last_text_id, talker_entry, use_text_timer, current_priority, event_sorting_required, nested_events_counter, all_event_flags, script_type }` | full state |
| `class SmartScriptMgr` (singleton) | `static SMART_SCRIPT_MGR: OnceLock<Arc<SmartScriptMgr>>` | sync init at startup |
| `class SmartWaypointMgr` | `static SMART_WAYPOINT_MGR: OnceLock<Arc<SmartWaypointMgr>>` | — |
| `enum SMART_EVENT` (~80) | `enum SmartEvent { UpdateIc { repeat_min, repeat_max }, HealthPct { hp_min, hp_max, repeat_min, repeat_max }, Aggro, Kill, ... }` | data-carrying variants |
| `enum SMART_ACTION` (~150) | `enum SmartAction { Talk { group_id, duration }, Cast { spell_id, flags: SmartCastFlags, target_index }, AddAura { spell_id, duration }, MoveToPos { x, y, z, transport: bool }, ... }` | data-carrying |
| `enum SMARTAI_TARGETS` (~30) | `enum SmartTarget { None, Self_, Victim, ClosestPlayer { range }, CreatureRange { entry, min_dist, max_dist, max_count }, StoredTargets { id }, ... }` | `Self_` because `self` is reserved |
| `enum SmartScriptType` | `enum SmartScriptType { Creature, GameObject, AreaTrigger, Event, Gossip, Quest, Spell, Transport, Instance, TimedActionList, Scene, AreaTriggerEntity, AreaTriggerEntityServerSide }` | u8 repr |
| `enum SMART_EVENT_PHASE_BITS` | `bitflags! struct SmartEventPhaseBits: u32 { const PHASE_1 = 1; const PHASE_2 = 2; const PHASE_3 = 4; ... const ALL = 0xFFF; }` | bitflags crate |
| `enum SmartEventFlags` | `bitflags! struct SmartEventFlags: u16 { const NOT_REPEATABLE = 1; const DIFFICULTY_0 = 2; const DEBUG_ONLY = 128; const DONT_RESET = 256; const WHILE_CHARMED = 512; }` | — |
| `enum SmartCastFlags` | `bitflags! struct SmartCastFlags: u32 { const INTERRUPT_PREVIOUS = 1; const TRIGGERED = 2; const FORCE_CAST = 4; const NO_COMBAT_MOVE = 8; const AURA_NOT_PRESENT = 16; const COMBAT_MOVE = 32; }` | — |
| `struct SmartScriptHolder` | `struct SmartScriptHolder { entry_or_guid: i64, source_type: SmartScriptType, id: u32, link: u32, event: SmartEvent, event_phase_mask: SmartEventPhaseBits, event_chance: u8, event_flags: SmartEventFlags, action: SmartAction, target: SmartTarget, timer: u32, priority: u32, run_once: bool, enable_timed: bool, comment: String }` | — |
| `mEventMap[type][entryOrGuid] -> SmartAIEventList` | `HashMap<SmartScriptType, HashMap<i64, Vec<SmartScriptHolder>>>` wrapped in `Arc` | shared, immutable post-load |
| `mCounterList: std::unordered_map<uint32, uint32>` | `HashMap<u32, u32>` | — |
| `_storedTargets: ObjectVectorMap` | `HashMap<u32, Vec<ObjectGuid>>` | resolve GUIDs at action time |
| `mLastInvoker: ObjectGuid` | `ObjectGuid` (`Option<ObjectGuid>` if zero-valued) | — |
| `WorldDatabase.Query("SELECT ... FROM smart_scripts ...")` | reuse existing `SEL_SMART_SCRIPTS` constant in `crates/wow-database`; `sqlx::query_as!(SmartScriptRow, ...)` | already declared, never executed |
| `IsEventValid(holder)` | `fn validate(h: &SmartScriptHolder) -> Result<(), ValidationError>` | rich error type |
| `ProcessEventsFor(SMART_EVENT, …)` | `fn process_events_for(&mut self, ev: SmartEventKind, ctx: &mut EventContext)` | `SmartEventKind` = discriminant-only enum for hook dispatch |
| `ProcessEvent(holder, …)` | `fn process_event(&mut self, h: &SmartScriptHolder, ctx: &mut EventContext) -> bool` | returns whether action ran |
| `ProcessAction(holder, …)` | `fn process_action(&mut self, h: &SmartScriptHolder, ctx: &mut EventContext)` | per-action effect |
| `GetTargets(out, holder, invoker)` | `fn get_targets(&self, h: &SmartScriptHolder, invoker: ObjectGuid, world: &World) -> Vec<ObjectGuid>` | GUID-based output |
| nested-event recursion via stack | deferred queue pattern: `pending_events: VecDeque<(SmartEventKind, EventContext)>` drained between top-level `process_event` calls | avoids borrow-checker re-entry |
| `MAX_NESTED_EVENTS = 10` | `const MAX_NESTED_EVENTS: u32 = 10;` | unchanged |
| `SmartCreatureAI::StartPath(run, pathId, repeat, invoker, nodeId)` | `SmartAI::start_path(&mut self, run: bool, path_id: u32, repeat: bool, invoker: ObjectGuid, node_id: u32)` | — |
| `creature_text` consumption | `wow-chat::CreatureTextMgr::send_chat(creature, group_id, target?)` | — |

---

*Template version: 1.0.* Sub-document of `ai.md`. Last updated 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked the C++ canonical sub-tree at `/home/server/woltk-trinity-legacy/src/server/game/AI/SmartScripts/` (`SmartAI.h` 354 lines, `SmartAI.cpp` 1259 lines, `SmartScript.h` 152 lines, `SmartScript.cpp` 4253 lines, `SmartScriptMgr.h` 1769 lines, `SmartScriptMgr.cpp` 2497 lines, plus `SmartScriptDefines.h` constants — total ~10,300 lines) against `crates/wow-script/` and `crates/wow-ai/` in the Rust workspace.

**Crate-emptiness finding — confirmed.** `crates/wow-script/src/lib.rs` is **0 lines** (`wc -l` confirms). The package builds and is wired into the workspace, but it ships no code. There is no `smart_event.rs`, no `smart_action.rs`, no `smart_target.rs`, no `smart_script_holder.rs`, no `smart_script.rs`, no `smart_script_mgr.rs`, no `smart_ai.rs`. There is no event enum, no action enum, no target enum, no flags, no holder struct, no loader, no validator, no cache, no interpreter, no waypoint manager, no `SmartAI` host. **The entire module is unbuilt.**

**Single SQL trace.** The only artifact of SmartScripts in the workspace is one prepared-statement constant `SEL_SMART_SCRIPTS` in `crates/wow-database/src/statements/world.rs:15` that defines the `SELECT entryorguid, source_type, id, link, event_type, event_phase_mask, event_chance, event_flags, event_param1..5, action_type, action_param1..7, target_type, target_param1..3, target_x, target_y, target_z, target_o FROM smart_scripts ORDER BY entryorguid, source_type, id, link` query. **No code in the workspace executes this statement.** Searching the workspace for `SmartScript`, `SmartAI`, `smart_scripts` (case-insensitive) finds zero matches outside this single SQL string.

**Schema coverage — 0%.** None of the ~80 `SMART_EVENT_*` are defined as Rust enums. None of the ~150 `SMART_ACTION_*` are defined. None of the ~30 `SMARTAI_TARGETS` are defined. Phase masks, event flags, cast flags, escort state, script type discriminator — all absent. The `SmartScriptHolder` struct does not exist; there is no Rust type that could even hold a single parsed row of `smart_scripts`.

**Loader coverage — 0%.** No `SmartScriptMgr`, no validation, no cache. The 50,000-row stock dataset cannot be loaded or queried by the runtime. Per-row schema validation (which in C++ rejects ~hundreds of malformed rows in stock TC data) does not exist, so even if the loader were added naively, the runtime would face crashing inputs.

**Runtime coverage — 0%.** No `SmartScript` instance state, no `OnUpdate`, no `ProcessEventsFor`, no `ProcessEvent`, no `ProcessAction`, no `GetTargets`, no phase mutation, no stored targets, no counters, no timed action list, no link chain dispatch, no nested-events guard. The interpreter that drives ~95% of in-game NPC behaviour does not exist in any form.

**Host coverage — 0%.** No `SmartAI`. Combined with the empty `crates/wow-ai/` trait surface (see `ai-base.md`), even if `SmartScript` were ported, there is no `CreatureAI` trait for `SmartAI` to implement and no selector to route spawns to it. **Both gaps must be closed jointly.**

**Auxiliary infrastructure — 0%.** No `SmartWaypointMgr`, no `creature_text` integration through `CreatureTextMgr`, no `Conditions` integration, no `creature_summon_groups` consumption, no waypoint loading, no escort/follow flow.

**Compilation surface.** Because `crates/wow-script/src/lib.rs` is empty rather than declaring stubs, downstream crates have nothing to import. The crate is a pure placeholder reservation in the workspace `Cargo.toml`. There is no even-broken-but-present scaffolding to incrementally fill — Wave A must start from genuinely zero.

**Worst divergence (the headline finding).** **The data-driven content layer that powers ~95% of WoW's mob and boss behaviour is completely disconnected from execution.** TrinityCore's design relies on `smart_scripts` being interpreted at runtime — boss talk lines, phase transitions, ability rotations, summons, waypoint paths, quest objectives, gossip flows, escort encounters all flow through `SmartScript`. RustyCore has the SQL connection ready, has the prepared statement constant declared, has the schema knowledge in this document — but has zero interpreter and no implementation plan beyond what is captured in §9. Even when `wow-spell` and `wow-combat` and the `ai-base.md` trait surface are filled in, **every NPC will play as "stands in place, swings white melee, dies silently"** until the SmartScript engine (rough size: ~10,300 lines C++ → ~5,000–8,000 lines Rust with enum-data variant compression) is ported. This sub-module is the **single biggest content blocker** in the engines layer.

**Realistic order-of-magnitude estimate.** §9 lists 61 numbered sub-tasks across five waves. Many individual items are **XL** (>12h) — particularly the `SmartAction` enum definition (~150 variants × parameter schemas), the `process_action` switch implementation (~150 arms × per-action side effects spread over modules `wow-spell`, `wow-combat`, `wow-chat`, `wow-movement`, `wow-quest`, `wow-instance`, `wow-vehicle`), and per-event-family chunks. Wave A (foundation: enums + holder + loader + validator) is on the order of 1–2 weeks of focused engineering. Wave B (interpreter runtime) is the bulk: 3–6 weeks. Wave C (`SmartAI` host) is gated on `ai-base.md` Wave A landing first. Wave D integrations and Wave E tests are another 1–2 weeks. **Conservatively: 2–3 person-months of focused work to reach behavioural parity with C++ for 3.4.3 content,** assuming Conditions / CreatureText / Spells / Movement support arrives on parallel tracks.
