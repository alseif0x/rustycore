# Migration: Conditions

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Conditions/` (`ConditionMgr`, `DisableMgr`)
> **Rust target crate(s):** `crates/wow-data/` (load `conditions` table, store `ConditionContainer` keyed by `(SourceType, SourceGroup, SourceEntry)`), `crates/wow-world/src/conditions/` (the `Meets` evaluator with access to `Player`/`Unit`/`Map`), no dedicated crate yet.
> **Layer:** L7 (Game systems — depends on Entities/Player+Unit L4, Quests L6, Reputation L6, Achievements L7, Map L4, World/DB2 L1; depended on by Phasing L7, Loot L6, Gossip L6, SmartScripts L7, SpellMgr L5, Vendors, Trainers, Graveyards, AreaTriggers, Conversation, GameObjects)
> **Status:** 🟡 in progress — core data shapes, SQL row parsing, full load-time source/type validation, condition grouping/reference semantics, searcher masks, partial `Condition::Meets`, specialized lookup helpers, and the audited `DisableMgr` data/runtime core are implemented. Remaining downstream source-type attachment, evaluator branches, DB2 helpers, full `DisableMgr` consumer wiring, and reload/consumer wiring remain open.
> **Audited vs C++:** ✅ audited 2026-05-01; implementation progress re-checked against C++ continuously during port work.
> **Last updated:** 2026-05-14

---

## 1. Purpose

`ConditionMgr` is TrinityCore's universal "does this player satisfy these requirements?" engine. One `conditions` table row defines a single boolean predicate (e.g. "has aura X", "is in zone Y", "rep rank ≥ Honored with faction F"); rows are bucketed by `SourceType × SourceGroup × SourceEntry × ElseGroup` and OR-of-AND-ed inside a bucket. The same evaluator drives ~30 source types: loot drops, gossip menu visibility, gossip option enable/disable, spell-implicit-target filtering, vehicle entry, NPC vendor item show/hide, spell click events, smart-script branches, terrain swaps, phases, graveyards, area triggers, trainer spells, object-id visibility, spawn groups, and conversation lines. It also exposes static helpers (`IsPlayerMeetingCondition`, `IsMeetingWorldStateExpression`, `IsUnitMeetingCondition`) to evaluate DB2-defined `PlayerCondition`, `WorldStateExpression`, and `UnitCondition` records.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Conditions/ConditionMgr.cpp` | 3921 | `prefix` |
| `game/Conditions/ConditionMgr.h` | 400 | `prefix` |
| `game/Conditions/DisableMgr.cpp` | 407 | `prefix` |
| `game/Conditions/DisableMgr.h` | 72 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Conditions/ConditionMgr.h` | 400 | `ConditionTypes` (~58 entries), `ConditionSourceType` (~33 entries + reference + max), `RelationType`, `InstanceInfo`, `MaxConditionTargets`, `ConditionSourceInfo`, `ConditionId`, `Condition` struct, `ConditionMgr` singleton class, `ConditionsReference` weak-pointer wrapper |
| `src/server/game/Conditions/ConditionMgr.cpp` | 3921 | Massive: `LoadConditions` (the single SQL load), per-source-type validators (`isSourceTypeValid`), per-condition-type validators (`isConditionTypeValid`), the giant `Condition::Meets` switch, `addToLootTemplate` / `addToGossipMenus` / `addToGossipMenuItems` / `addToSpellImplicitTargetConditions` / `addToPhases` / `addToGraveyardData` index builders, `IsPlayerMeetingCondition` (DB2 PlayerCondition.db2), `IsMeetingWorldStateExpression` (DB2), `IsUnitMeetingCondition` (DB2), the static metadata tables `StaticSourceTypeData[]` and `StaticConditionTypeData[]` |
| `src/server/game/Conditions/DisableMgr.h` | 72 | `DisableType` enum (9 values: `SPELL`, `QUEST`, `MAP`, `BATTLEGROUND`, `CRITERIA`, `OUTDOOR_PVP`, `VMAP`, `MMAP`, `LFG_MAP`), `DisableFlags` enum, `IsDisabledFor` helpers; sibling system to ConditionMgr but distinct |
| `src/server/game/Conditions/DisableMgr.cpp` | 407 | `LoadDisables` (loads `disables` table), `IsDisabledFor` per-type implementations |

Out-of-tree consumers (each calls `sConditionMgr` extensively):
- `src/server/game/Loot/LootMgr.cpp` — every loot template item is gated by a `ConditionContainer`.
- `src/server/game/Misc/GossipDef.cpp` and `src/server/game/Handlers/NPCHandler.cpp` — gossip menu/option visibility.
- `src/server/game/Spells/SpellMgr.cpp` — implicit target conditions.
- `src/server/game/AI/SmartScripts/SmartAI.cpp` — `SmartEvent` activation conditions.
- `src/server/game/Phasing/PhasingHandler.cpp` — area phases, terrain swaps.
- `src/server/game/Entities/Vehicle.cpp` — vehicle entry conditions.
- `src/server/game/Entities/Creature.cpp` — vendor item show, trainer spell visibility.
- `src/server/game/Entities/GameObject.cpp` — gameobject visibility / interaction.
- `src/server/game/AreaTrigger/AreaTrigger.cpp` — server-side area trigger filters.
- `src/server/game/Conversation/Conversation.cpp` — conversation line conditions.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Condition` | struct | One row of `conditions`: `SourceType`, `SourceGroup`, `SourceEntry`, `SourceId`, `ElseGroup`, `ConditionType`, `ConditionValue1/2/3`, `ConditionStringValue1`, `ErrorType`, `ErrorTextId`, `ReferenceId`, `ScriptId`, `ConditionTarget` (which target0/1/2 to evaluate against), `NegativeCondition` |
| `ConditionContainer` | typedef | `std::vector<Condition>` — the OR-of-AND grouping is encoded by `ElseGroup` (same `ElseGroup` = AND, different `ElseGroup` = OR) |
| `ConditionsByEntryMap` | typedef | `unordered_map<ConditionId, shared_ptr<ConditionContainer>>` keyed by `(SourceGroup, SourceEntry, SourceId)` |
| `ConditionEntriesByTypeArray` | typedef | `array<ConditionsByEntryMap, CONDITION_SOURCE_TYPE_MAX>` — main storage, indexed by source type |
| `ConditionsReference` | struct | `weak_ptr<ConditionContainer>` wrapper that exposes `Meets(WorldObject*)` for downstream modules to hold without owning |
| `ConditionMgr` | singleton class | `LoadConditions`, the `IsObjectMeet…Conditions` family, validators, `Clean` |
| `ConditionSourceInfo` | struct | `WorldObject* mConditionTargets[3]`, `Map* mConditionMap`, `Condition* mLastFailedCondition` (used to extract `ErrorType/ErrorTextId` for client-facing failure msgs) |
| `ConditionId` | struct | `(SourceGroup, SourceEntry, SourceId)` — hashable key for `ConditionsByEntryMap` |
| `ConditionTypes` | enum (~58 entries, MAX = 59) | `NONE`, `AURA`, `ITEM`, `ITEM_EQUIPPED`, `ZONEID`, `REPUTATION_RANK`, `TEAM`, `SKILL`, `QUESTREWARDED`, `QUESTTAKEN`, `DRUNKENSTATE`, `WORLD_STATE`, `ACTIVE_EVENT`, `INSTANCE_INFO`, `QUEST_NONE`, `CLASS`, `RACE`, `ACHIEVEMENT`, `TITLE`, `SPAWNMASK_DEPRECATED`, `GENDER`, `UNIT_STATE`, `MAPID`, `AREAID`, `CREATURE_TYPE`, `SPELL`, `PHASEID`, `LEVEL`, `QUEST_COMPLETE`, `NEAR_CREATURE`, `NEAR_GAMEOBJECT`, `OBJECT_ENTRY_GUID_LEGACY`, `TYPE_MASK_LEGACY`, `RELATION_TO`, `REACTION_TO`, `DISTANCE_TO`, `ALIVE`, `HP_VAL`, `HP_PCT`, `REALM_ACHIEVEMENT`, `IN_WATER`, `TERRAIN_SWAP`, `STAND_STATE`, `DAILY_QUEST_DONE`, `CHARMED`, `PET_TYPE`, `TAXI`, `QUESTSTATE`, `QUEST_OBJECTIVE_PROGRESS`, `DIFFICULTY_ID`, `GAMEMASTER`, `OBJECT_ENTRY_GUID`, `TYPE_MASK`, `BATTLE_PET_COUNT`, `SCENARIO_STEP`, `SCENE_IN_PROGRESS`, `PLAYER_CONDITION`, `PRIVATE_OBJECT`, `STRING_ID` |
| `ConditionSourceType` | enum (~33 + 2 internal) | `CREATURE_LOOT_TEMPLATE`, `DISENCHANT_LOOT_TEMPLATE`, `FISHING_LOOT_TEMPLATE`, `GAMEOBJECT_LOOT_TEMPLATE`, `ITEM_LOOT_TEMPLATE`, `MAIL_LOOT_TEMPLATE`, `MILLING_LOOT_TEMPLATE`, `PICKPOCKETING_LOOT_TEMPLATE`, `PROSPECTING_LOOT_TEMPLATE`, `REFERENCE_LOOT_TEMPLATE`, `SKINNING_LOOT_TEMPLATE`, `SPELL_LOOT_TEMPLATE`, `SPELL_IMPLICIT_TARGET`, `GOSSIP_MENU`, `GOSSIP_MENU_OPTION`, `CREATURE_TEMPLATE_VEHICLE`, `SPELL`, `SPELL_CLICK_EVENT`, `QUEST_AVAILABLE`, `VEHICLE_SPELL`, `SMART_EVENT`, `NPC_VENDOR`, `SPELL_PROC`, `TERRAIN_SWAP`, `PHASE`, `GRAVEYARD`, `AREATRIGGER`, `CONVERSATION_LINE`, `AREATRIGGER_CLIENT_TRIGGERED`, `TRAINER_SPELL`, `OBJECT_ID_VISIBILITY`, `SPAWN_GROUP`, plus internal `REFERENCE_CONDITION` |
| `RelationType` | enum (6) | `SELF`, `IN_PARTY`, `IN_RAID_OR_PARTY`, `OWNED_BY`, `PASSENGER_OF`, `CREATED_BY` |
| `InstanceInfo` | enum (4) | `DATA`, `GUID_DATA`, `BOSS_STATE`, `DATA64` |
| `MaxConditionTargets` | enum (1) | `MAX_CONDITION_TARGETS = 3` |
| `ConditionTypeInfo` | struct | Static metadata: name + which `ConditionValue1/2/3/StringValue1` slots are meaningful per condition type |
| `DisableType` | enum (9) | `SPELL`, `QUEST`, `MAP`, `BATTLEGROUND`, `CRITERIA`, `OUTDOOR_PVP`, `VMAP`, `MMAP`, `LFG_MAP` |
| `DisableFlags` (per type) | enum bitfields | E.g. `SPELL_DISABLE_PLAYER`, `SPELL_DISABLE_CREATURE`, `SPELL_DISABLE_PET`, `SPELL_DISABLE_DEPRECATED_SPELL`, `SPELL_DISABLE_MAP_ARG`, `SPELL_DISABLE_AREA_ARG`, `SPELL_DISABLE_LOS`; map disable flags include `DISABLE_TYPE_MAP_NORMAL/HEROIC/MAX_DIFFICULTY` etc |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ConditionMgr::LoadConditions(bool isReload)` | Single-pass load: fetch all `conditions` rows, dispatch each to its source-type-specific index builder (loot template attach, gossip menu attach, phase attach, etc.); resolve `ReferenceId` chains; clean stale data on reload | DB query, `addToLootTemplate`, `addToGossipMenus`, `addToGossipMenuItems`, `addToSpellImplicitTargetConditions`, `addToPhases`, `addToGraveyardData`, `isSourceTypeValid`, `isConditionTypeValid` |
| `ConditionMgr::isConditionTypeValid(Condition*)` | Per-type validation: e.g. `CONDITION_AURA` requires `ConditionValue1 = valid spell id` and `ConditionValue2 < MAX_SPELL_EFFECTS` | spell store, item store, faction store, area table, achievement store, etc. |
| `ConditionMgr::isSourceTypeValid(Condition*)` | Per-source-type validation: e.g. `SOURCE_TYPE_GOSSIP_MENU` requires the `(menuId, textId)` pair to exist | `sObjectMgr->GetGossipMenusMapBounds`, etc. |
| `ConditionMgr::IsObjectMeetToConditions(WorldObject const* o, ConditionContainer const&)` | Evaluate a 1-target group; returns true if any `ElseGroup` has all its conditions meet | `IsObjectMeetToConditionList` |
| `ConditionMgr::IsObjectMeetToConditions(WorldObject const* o1, WorldObject const* o2, ConditionContainer const&)` | Same, 2-target | builds `ConditionSourceInfo(o1, o2)`, `IsObjectMeetToConditionList` |
| `ConditionMgr::IsObjectMeetToConditions(ConditionSourceInfo&, ConditionContainer const&)` | Core evaluator: groups conditions by `ElseGroup`, evaluates each group as AND, returns true if any group is fully satisfied (OR across groups) | `Condition::Meets` per row |
| `ConditionMgr::IsObjectMeetingNotGroupedConditions(sourceType, entry, ConditionSourceInfo&)` | For source types that have a single bucket per (sourceGroup=0, sourceEntry=entry) — phases, terrain swaps, area triggers, etc. | indexes into `ConditionStore[sourceType]`, then `IsObjectMeetToConditions` |
| `ConditionMgr::IsObjectMeetingNotGroupedConditions(sourceType, entry, target0, target1?, target2?)` | Convenience overload | builds `ConditionSourceInfo` |
| `ConditionMgr::IsMapMeetingNotGroupedConditions(sourceType, entry, Map*)` | Map-only context (used by terrain swaps when no specific player) | builds map-context `ConditionSourceInfo` |
| `ConditionMgr::HasConditionsForNotGroupedEntry(sourceType, entry)` | Cheap "are there any conditions" lookup, used to skip evaluation for entries that have none | map lookup only |
| `ConditionMgr::IsObjectMeetingSpellClickConditions(creatureId, spellId, clicker, target)` | Special: the `(creatureId, spellId)` pair has a dedicated map (spell click events have their own grouping) | `_spellClickEventConditions` map |
| `ConditionMgr::HasConditionsForSpellClickEvent(creatureId, spellId)` | Same lookup, presence-only | — |
| `ConditionMgr::IsObjectMeetingVehicleSpellConditions(creatureId, spellId, player, vehicle)` | Per-vehicle-spell access | `_vehicleSpellConditions` |
| `ConditionMgr::IsObjectMeetingSmartEventConditions(entryOrGuid, eventId, sourceType, unit, baseObject)` | Smart-script specific (because SmartAI keys are different) | `_smartEventConditions` |
| `ConditionMgr::IsObjectMeetingVendorItemConditions(creatureId, itemId, player, vendor)` | Vendor inventory show/hide | `_npcVendorConditions` |
| `ConditionMgr::IsObjectMeetingTrainerSpellConditions(trainerId, spellId, player)` | Trainer spell visibility | `_trainerSpellConditions` |
| `ConditionMgr::IsObjectMeetingVisibilityByObjectIdConditions(objectType, entry, seer)` | Generic per-entry visibility filter (creature/gameobject) | `_objectVisibilityConditions` |
| `ConditionMgr::GetConditionsForAreaTrigger(areaTriggerId, isServerSide)` | Returns the bucket for a server-side area trigger | `_areaTriggerConditions` |
| `Condition::Meets(ConditionSourceInfo&)` | The big switch over `ConditionType` — every type has its own predicate; updates `mLastFailedCondition` for client error reporting | every condition type's evaluator |
| `Condition::GetSearcherTypeMaskForCondition()` | Returns a TypeMask saying what kinds of WorldObjects this condition could possibly accept, for early-exit | static dispatch by `ConditionType` |
| `ConditionMgr::IsPlayerMeetingCondition(Player const*, PlayerConditionEntry const*)` | Evaluate `PlayerCondition.db2` (huge — race/class/level/aura/quest/skill/reputation/currency/areagroups/honorlevel/spec/raceMask/etc.) — pure function, no `conditions` table involvement | DB2 stores |
| `ConditionMgr::IsMeetingWorldStateExpression(Map*, WorldStateExpressionEntry*)` | Evaluate the byte-coded expression in `WorldStateExpression.db2` (RPN-like) | `WorldStateMgr` |
| `ConditionMgr::IsUnitMeetingCondition(Unit const* a, Unit const* b, UnitConditionEntry*)` | Evaluate `UnitCondition.db2` (relational unit-vs-unit predicates) | unit getters |
| `DisableMgr::IsDisabledFor(DisableType, entry, unit, flags)` | Is this spell/quest/map disabled? | `_disableMap` |
| `DisableMgr::IsPathfindingEnabled(mapId)`, `IsVMAPDisabledFor`, `IsMMAPDisabledFor` | Map-specific overrides | `_disableMap[DISABLE_TYPE_VMAP/MMAP/...]` |

---

## 5. Module dependencies

**Depends on:**
- `Player` / `Unit` / `Item` / `Creature` / `GameObject` / `Map` — every condition predicate reads runtime state from these.
- `World/DB2` — `PlayerCondition.db2`, `WorldStateExpression.db2`, `UnitCondition.db2`, `Achievement.db2`, `Faction.db2`, `Skill.db2`, `Phase.db2`, `AreaTable.db2`, `Map.db2`, `CharTitles.db2`, `BattlePetSpecies.db2`, `ScenarioStep.db2`, `Difficulty.db2`.
- `SpellMgr` — `CONDITION_AURA`, `CONDITION_SPELL`, `CONDITION_SPELL_PROC` evaluators read spell data.
- `ObjectMgr` — `CONDITION_NEAR_CREATURE`, `CONDITION_NEAR_GAMEOBJECT`, `CONDITION_OBJECT_ENTRY_GUID` need entry tables.
- `WorldStateMgr` — `CONDITION_WORLD_STATE` and `WorldStateExpression` evaluation.
- `GameEventMgr` — `CONDITION_ACTIVE_EVENT`.
- `AchievementMgr` — `CONDITION_ACHIEVEMENT`, `CONDITION_REALM_ACHIEVEMENT`.
- `Quest system` — `CONDITION_QUESTREWARDED`, `_QUESTTAKEN`, `_QUEST_NONE`, `_QUEST_COMPLETE`, `_QUESTSTATE`, `_QUEST_OBJECTIVE_PROGRESS`, `_DAILY_QUEST_DONE`.
- `Reputation` — `CONDITION_REPUTATION_RANK`.

**Depended on by:**
- `Phasing` — `CONDITION_SOURCE_TYPE_PHASE` (per area+phase) and `CONDITION_SOURCE_TYPE_TERRAIN_SWAP`.
- `Loot` — every loot template source type (creature, gameobject, fishing, mail, …) gates each loot row.
- `Gossip` — `CONDITION_SOURCE_TYPE_GOSSIP_MENU` and `_OPTION` filter what menus/options are shown.
- `SpellMgr` — `CONDITION_SOURCE_TYPE_SPELL_IMPLICIT_TARGET` filters which targets a spell can affect.
- `SpellClick` — `CONDITION_SOURCE_TYPE_SPELL_CLICK_EVENT` decides if a player can click an NPC.
- `Vehicle` — `CONDITION_SOURCE_TYPE_CREATURE_TEMPLATE_VEHICLE` decides if a player can enter a vehicle creature; `_VEHICLE_SPELL` for which abilities are exposed.
- `SmartScripts` — `CONDITION_SOURCE_TYPE_SMART_EVENT` gates SmartAI events.
- `NPC vendors` — `CONDITION_SOURCE_TYPE_NPC_VENDOR` per-item.
- `Trainers` — `CONDITION_SOURCE_TYPE_TRAINER_SPELL` per-spell.
- `Graveyards` — `CONDITION_SOURCE_TYPE_GRAVEYARD` decides which graveyard a player resurrects to.
- `AreaTriggers` — `_AREATRIGGER` (server-side) + `_AREATRIGGER_CLIENT_TRIGGERED`.
- `Conversation` — `_CONVERSATION_LINE`.
- `Spawn groups` — `_SPAWN_GROUP`.
- `Object visibility` — `_OBJECT_ID_VISIBILITY` (filter creatures/gameobjects by entry).

**Phasing contract:**
- `PhasingHandler::OnAreaChange` needs `CONDITION_SOURCE_TYPE_PHASE` buckets attached to each `PhaseAreaInfo` as a reload-safe `ConditionsReference`/weak handle, mirroring C++ `PhaseRef::AreaConditions`.
- `PhasingHandler::OnMapChange` and `OnConditionChange` need `IsObjectMeetingNotGroupedConditions(CONDITION_SOURCE_TYPE_TERRAIN_SWAP, terrainSwapId, object)` with C++ OR-of-AND `ElseGroup` semantics and `NegativeCondition` support.
- Do not mark phasing lifecycle tasks complete while ConditionMgr still uses injected predicates or "always true" fallbacks; those are test seams only, not C++ parity.

---

## 6. SQL / DB queries (if any)

Only one input table:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT SourceTypeOrReferenceId, SourceGroup, SourceEntry, SourceId, ElseGroup, ConditionTypeOrReference, ConditionTarget, ConditionValue1, ConditionValue2, ConditionValue3, NegativeCondition, ErrorType, ErrorTextId, ScriptName FROM conditions` | Single load, ~tens of thousands of rows in production world DB | world |
| `SELECT … FROM disables` (DisableMgr) | Loads spell/quest/map/bg/criteria/outdoorpvp/vmap/mmap disables | world |

Plus indirect reads of every DB2 store mentioned in §5 — but those are owned by the DataStores module, not Conditions.

DB2 stores read directly by ConditionMgr static helpers:

| Store | What it loads | Read by |
|---|---|---|
| `sPlayerConditionStore` | PlayerCondition.db2 | `IsPlayerMeetingCondition` (~70 sub-checks) |
| `sUnitConditionStore` | UnitCondition.db2 | `IsUnitMeetingCondition` |
| `sWorldStateExpressionStore` | WorldStateExpression.db2 | `IsMeetingWorldStateExpression` |
| `sFactionStore`, `sFactionTemplateStore` | Faction(Template).db2 | `CONDITION_REPUTATION_RANK`, `CONDITION_REACTION_TO` |
| `sAchievementStore` | Achievement.db2 | `CONDITION_ACHIEVEMENT`, `CONDITION_REALM_ACHIEVEMENT` |
| `sCharTitlesStore` | CharTitles.db2 | `CONDITION_TITLE` |
| `sCreatureFamilyStore` | CreatureFamily.db2 | `CONDITION_PET_TYPE` |
| `sBattlePetSpeciesStore` | BattlePetSpecies.db2 | `CONDITION_BATTLE_PET_COUNT` |
| `sDifficultyStore` | Difficulty.db2 | `CONDITION_DIFFICULTY_ID` |

---

## 7. Wire-protocol packets (if any)

ConditionMgr is server-internal — it emits no packets directly. Indirectly, when a `Condition::Meets` returns false and `Condition::ErrorType / ErrorTextId` are set, downstream code (chiefly Quest and Spell systems) packages that into a client-facing error packet (`SMSG_QUEST_GIVER_QUEST_FAILED`, `SMSG_CAST_FAILED`, `SMSG_DISPLAY_GAME_ERROR`). The `ConditionSourceInfo::mLastFailedCondition` field is the carrier.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-world/src/conditions` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-logging` | `crate_dir` | 1 | 464 | `exists_active` | crate exists |
| `crates/wow-logging/src/lib.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/character.rs` | `file` | 1 | 550 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None directly. The `wow-logging` crate has a `LogFilter::Condition` variant (`crates/wow-logging/src/lib.rs:84`) that is currently unused.
- Stub-level dependencies: a few packets (`crates/wow-packet/src/packets/character.rs`, `quest.rs`, `update.rs`) write `0` for `UnlockedConditionalAppearanceCount`, `ConditionalDescriptionText count`, `ContentTuningConditionMask`, `ConditionalTransmog.Size` — none of these are ConditionMgr concerns, they are unrelated DB2-driven optional client fields.

**What's implemented:**
- `crates/wow-constants/src/conditions.rs` defines C++-value-compatible `ConditionType`, `ConditionSourceType`, `RelationType`, `ConditionInstanceInfo`, `MAX_CONDITION_TARGETS`, and `ComparisonType` constants/enums from `ConditionMgr.h` / `Util.h`.
- `crates/wow-data/src/conditions.rs` defines the owned `Condition`, `ConditionId`, and `ConditionContainer` data shapes with C++ constructor defaults and `Condition::isLoaded` parity.
- `crates/wow-data/src/conditions.rs` implements C++ `Condition::ToString`, `CanHaveSourceGroupSet`, `CanHaveSourceIdSet`, and `CanHaveConditionType` helpers for SQL/debug logs and source/type compatibility. Rust intentionally names `CONDITION_PRIVATE_OBJECT` as `Private Object`; the inspected C++ static name table appears to omit that slot.
- `crates/wow-data/src/conditions.rs` implements the C++ `StaticConditionTypeData[]` field-usage metadata, useless-value detection helper, legacy object/type-mask normalization, and the pure `isConditionTypeValid` checks that do not require SpellMgr/ObjectMgr/DB2 stores (including aura effect index, item count, comparison/range masks, relation target selectors, stand state, pet type, and battle-pet count limits). Rust keeps the corrected `Private Object`/`String ID` slots explicit because the inspected legacy table appears shifted around `CONDITION_PRIVATE_OBJECT`; Rust also accepts `CONDITION_STRING_ID` during type validation, matching archived TrinityCore, because the local legacy `ConditionMgr.cpp` appears to omit that no-op validation case by mistake.
- `crates/wow-data/src/conditions.rs` implements C++ `Condition::GetSearcherTypeMaskForCondition` and `ConditionMgr::GetSearcherTypeMaskForConditionList`, including `ElseGroup` AND/OR aggregation and recursive reference-mask expansion.
- `crates/wow-world/src/conditions.rs` defines runtime `ConditionSourceInfo` with 3 target slots, derived map context, and last-failed-condition tracking.
- `crates/wow-world/src/conditions.rs` implements a partial, explicit C++ `Condition::Meets` evaluator for represented state (`NONE`, `MAPID`, `ZONEID`, `AREAID`, object entry/type mask aliases including creature/gameobject spawn-id snapshots, `PRIVATE_OBJECT`, `STRING_ID`, map/server snapshots for `ACTIVE_EVENT`, `INSTANCE_INFO`, `WORLD_STATE`, `DIFFICULTY_ID`, `SCENARIO_STEP`, `AURA`, `CLASS`, `RACE`, `LEVEL`, `ALIVE`, `HP_VAL`, `HP_PCT`, `UNIT_STATE`, `IN_WATER`, `STAND_STATE`, `CHARMED`, `CREATURE_TYPE`, `DISTANCE_TO`, `RELATION_TO`, `REACTION_TO`, `NEAR_CREATURE`, `NEAR_GAMEOBJECT`, inventory/progression snapshots for `ITEM`, `ITEM_EQUIPPED`, `SKILL`, `SPELL`, `ACHIEVEMENT`, `REPUTATION_RANK`, `REALM_ACHIEVEMENT`, `TITLE`, `BATTLE_PET_COUNT`, `SCENE_IN_PROGRESS`, quest snapshots for `QUESTREWARDED`, `QUESTTAKEN`, `QUEST_NONE`, `QUEST_COMPLETE`, `QUESTSTATE`, `QUEST_OBJECTIVE_PROGRESS`, `DAILY_QUEST_DONE`, `TEAM`, `GENDER`, `DRUNKENSTATE`, `GAMEMASTER`, `PET_TYPE`, `TAXI`, `PHASEID`, `TERRAIN_SWAP`) and returns `Unsupported` instead of silently passing unported player/DB2/runtime branches. Unit-only, aura, relation, nearby-object, creature-only, object-only, map/server, inventory/progression, quest, and player-only checks use explicit snapshots until Rust has real `WorldObject -> Unit/Creature/GameObject/Player` views.
- `crates/wow-world/src/conditions.rs` implements C++ `IsObjectMeetToConditions` OR-of-AND `ElseGroup` aggregation and `IsObjectMeetingNotGroupedConditions` lookup with injected per-condition evaluator.
- `crates/wow-world/src/conditions.rs` implements C++ specialized condition lookups for spell-click, vehicle spell, smart event, vendor item, area trigger, trainer spell, and object-id visibility keys.
- `crates/wow-world/src/conditions.rs` implements the runtime loot-condition bridge equivalent to C++ `LootTemplate::LinkConditions` + `LootItem::AllowedForPlayer`: a `LootStoreItemContext` resolves `(SourceType, SourceGroup, SourceEntry)` and evaluates the matching bucket with the looter as target0.
- `wow-data::ConditionEntriesByTypeStore` builds the C++ `SpellsUsedInSpellClickConditions` auxiliary index from `SPELL_CLICK_EVENT` rows whose condition type is `AURA`, and exposes the `IsSpellUsedInSpellClickConditions` query.
- `wow-data::GraveyardStore` ports C++ `ObjectMgr::LoadGraveyardZones`, `FindGraveyardData`, and `ConditionMgr::addToGraveyardData` data attachment with reload-safe `ConditionsReference` handles.
- `wow-data::GossipStore` ports the C++ `GossipMenus` / `GossipMenuItems` condition holders plus `ConditionMgr::addToGossipMenus` and `addToGossipMenuItems` attachment semantics, including the C++ no-error behaviour when a menu exists but no `TextID` matches the condition `SourceEntry`.
- `wow-data::attach_loaded_conditions_like_cpp` mirrors the final attachment pass of C++ `ConditionMgr::LoadConditions` for represented systems: gossip menus/options, spell-click aura spell index, spell implicit target condition handles, phase areas, and graveyards. Spell implicit target conditions are validated/normalized during external validation and attached to the represented `SpellStore` per effect index.
- `handlers/trainer.rs` filters trainer-list spells through the loaded `ConditionMgr` store. Rust intentionally keys this lookup by `CONDITION_SOURCE_TYPE_TRAINER_SPELL`; the inspected legacy and archived C++ `IsObjectMeetingTrainerSpellConditions` implementations search `NPC_VENDOR` despite the dedicated `TRAINER_SPELL` enum/validation path, so Rust keeps the semantically correct source type rather than preserving that legacy bug.
- `wow-database::WorldStatements::SEL_CONDITIONS` plus `wow-data::load_condition_rows_like_cpp` parse the C++ `conditions` table projection, including negative reference rows, reference templates, and C++-equivalent non-fatal useless-data warnings on reference rows/templates; remaining indexing/attachment gaps are tracked under `#COND.20`.
- `wow-data::parse_condition_rows_like_cpp` now applies C++ load-shape/source checks that do not require external stores: allowed `SourceGroup`, allowed `SourceId`, max `ConditionTarget`, internal `REFERENCE_CONDITION` handling, `SPELL_IMPLICIT_TARGET` effect-mask nonzero, `AREATRIGGER` SourceEntry 0/1, `OBJECT_ID_VISIBILITY` object type restrictions, and `ErrorType/ErrorTextId` normalization with warnings.
- `wow-data::apply_external_condition_validation_like_cpp` now performs a startup validation pass against already-loaded stores for represented C++ checks: item/spell/zone/skill/map/phase/quest/objective condition values, loot-template source group/source entry checks, quest-available source entry checks, spell-based source entries, spell implicit target effect-shape normalization, client-triggered area-trigger DB2 entries, terrain-swap map, and phase-area source entries. This intentionally does not claim ObjectMgr-only, Achievement/Faction, PlayerCondition, Scenario/Scene, or runtime SpellImplicitTarget attachment yet.
- `world-server` now loads the C++ `conditions` table during startup, builds a shared `ConditionEntriesByTypeStore`, installs it into a process-wide C++ `sConditionMgr`-style accessor, logs skipped/warning rows, records the represented attachment-pass counts, and passes the store through `SessionResources` into each `WorldSession`. This is startup/store parity only; it does not close external-store validation or every downstream consumer callsite.
- `wow-data::ConditionEntriesByTypeStore` groups parsed rows by `ConditionSourceType` and `ConditionId`, and `ConditionsReference` mirrors the C++ weak-reference holder used by downstream modules across reloads.
- `wow-data::PhaseInfoStore::attach_phase_conditions_like_cpp` ports C++ `ConditionMgr::addToPhases`: `SourceEntry = 0` attaches a phase bucket to every area owned by the phase, non-zero `SourceEntry` attaches only to that concrete area/phase pair, and missing area/phase pairs are reported.
- `wow-data::MountStore` now loads `Mount.db2` plus C++ hotfix rows and builds the C++ `_mountsBySpellId` index. `CollectionMgr::LoadAccountMounts` parity is represented by filtering account mount rows through `GetMount(spellId)`, keeping the accepted account mount flags in session, applying `CMSG_MOUNT_SET_FAVORITE` with the C++ partial `SMSG_ACCOUNT_MOUNT_UPDATE` response, and saving account mount flags on logout. `Mount.PlayerConditionID` has a represented session helper for the C++ `CollectionMgr::AddMount` use/learn gate. `MountCapability.db2`, `MountTypeXCapability.db2`, and `MountXDisplay.db2` are also loaded with hotfix overlays and C++ grouping/order semantics; the C++ taxi preferred-mount display filter is represented by resolving `FlyingMountID`, requiring the source spell, and filtering displays through `PlayerConditionID`. `Vehicle.db2` and `VehicleSeat.db2` now load with hotfix overlays for represented `CreateVehicleKit` validation and C++ seat/usable-seat construction shape, `vehicle_template` rows are loaded for C++ `GetDespawnDelay()` semantics with the 1ms fallback, and `vehicle_accessory`/`vehicle_template_accessory` rows are loaded with C++ GUID-first/template fallback selection for represented vehicle accessory lists. `SpellEffect` now retains the C++ `EffectAura`, `EffectBasePoints`, and `EffectMiscValue1/2` fields needed by `SPELL_AURA_MOUNTED`, and represented mount aura application resolves display candidates from `Mount/MountXDisplay`, handles self-mount hidden display, randomly selects among usable displays, applies/removes the aura, keeps the mount creature `VehicleId` even when MountXDisplay supplies the display, uses creature_template_model fallback when MountXDisplay provides no display, sends the mount display plus `UNIT_FLAG_MOUNT` deltas, validates represented vehicle-kit creation through loaded `Vehicle.db2`, creates an installed `wow_entities::Vehicle` from `VehicleSeat.db2` seat definitions, records the C++ `InstallAllAccessories(false)` template accessory rows for player mounts, exposes represented vehicle despawn delay lookup, uninstalls it on dismount, emits the C++ owner vehicle-rec packets for mount create/remove, emits `SMSG_ON_CANCEL_EXPECTED_RIDE_VEHICLE_AURA` after represented vehicle-kit creation, recalculates `Unit::GetCollisionHeight()` from `CreatureDisplayInfo/CreatureModelData` before owner/broadcast collision-height packets, and emits represented C++ `SMSG_PET_MODE` disable/enable packets when a session pet GUID is known. Session still records mount pet resummon requests until full pet runtime/guid ownership exists. The pure `Unit::GetMountCapability` selection is represented and `WorldSession` now wires it to loaded area/map stores, current area/map, visible auras, known spells, `SKILL_RIDING` loaded from `character_skills`, live movement `SWIMMING`, and represented C++ `ZLiquidStatus` bits. Terrain/grid/vmap still needs to own refreshing that liquid status from map data; exact VehicleKit passenger runtime, accessory TempSummon/HandleSpellClick install, broadcast-to-set behavior, and full pet runtime ownership remain pending.

**What's missing vs C++:**
- Remaining integration of condition validation/indexing into all runtime consumers and reload paths.
- Source-type index builder reload wiring and remaining consumer use sites. Gossip, spell implicit target, phase and graveyard attachment plus a represented final attachment pass exist in `wow-data`; world startup wires those represented attachments. Loot has a runtime bridge from `LootStoreItemContext`, but all production fill callsites still need to pass the ConditionMgr-backed predicate.
- Remaining runtime `Condition::Meets` branches that need real player/unit/map/DB2 state, plus `IsPlayerMeetingCondition`, `IsMeetingWorldStateExpression`, `IsUnitMeetingCondition`, and the remaining `DisableMgr` DB2/consumer wiring.
- Downstream callsites are only partially prepared; Loot/Gossip/Vendors/Trainers/Phasing still need real ConditionMgr integration before they can claim full parity.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Without ConditionMgr, every NPC vendor sells every item to every player; loot tables are unfiltered; quest gossip menus show all options; phases never suppress; spell-click NPCs accept any clicker. This is not "divergent", it is "absent" — once a feature is built that requires conditions, expect immediate-and-loud regressions.
- The L7 batch order matters: ConditionMgr should land *before* Phasing, Loot's quest filtering, gossip menu polish, and SmartScripts.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#CONDITIONS.WBS.001** Partir y cerrar la migracion auditada de `game/Conditions/ConditionMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-logging`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3921 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CONDITIONS.WBS.002** Cerrar la migracion auditada de `game/Conditions/ConditionMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-logging`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CONDITIONS.WBS.003** Cerrar la migracion auditada de `game/Conditions/DisableMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-logging`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CONDITIONS.WBS.004** Cerrar la migracion auditada de `game/Conditions/DisableMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-logging`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [x] **#COND.1** Define `ConditionTypes` enum (~58 variants + `MAX`) in `crates/wow-constants/src/conditions.rs` matching C++ values byte-for-byte (including the deprecated `SPAWNMASK_DEPRECATED = 19` and `OBJECT_ENTRY_GUID_LEGACY = 31` / `TYPE_MASK_LEGACY = 32`) (M)
- [x] **#COND.2** Define `ConditionSourceType` enum (~33 variants + `MAX_DB_ALLOWED` + internal `REFERENCE_CONDITION` + `MAX`) (M)
- [x] **#COND.3** Define auxiliary enums `RelationType` (6), `InstanceInfo` (4), `MaxConditionTargets = 3`, plus `ComparisonType` (referenced by `_LEVEL`, `_HP_VAL`, `_HP_PCT`, `_DISTANCE_TO`, `_BATTLE_PET_COUNT`) (L)
- [x] **#COND.4** Define `Condition` struct in `crates/wow-data/src/conditions.rs` with all 16 fields (`SourceType`, `SourceGroup`, `SourceEntry`, `SourceId`, `ElseGroup`, `ConditionType`, `ConditionTarget`, `ConditionValue1/2/3`, `ConditionStringValue1`, `ErrorType`, `ErrorTextId`, `ReferenceId`, `ScriptId`, `NegativeCondition`) and a `Default` impl (M)
- [x] **#COND.5** Define `ConditionSourceInfo` (3-target array, optional `Map` ref, mutable `LastFailedCondition` slot) (L)
- [x] **#COND.6** Define `ConditionId` `(SourceGroup, SourceEntry, SourceId)` with `Hash`/`Eq` (L)
- [x] **#COND.7** Implement loader for `conditions` table — single SQL query, build `ConditionEntriesByTypeArray` (`[ConditionsByEntryMap; CONDITION_SOURCE_TYPE_MAX]`), validate every row via `is_source_type_valid` + `is_condition_type_valid`, drop invalid rows with `tc_log_error("sql.sql", …)`-equivalent (XL — SQL statement, row parser, self-reference skip, negative reference/template handling, reference/useless-data warnings, shape validation, external store validation, error-field normalization, ConditionStore grouping, and world startup load implemented; source-type attachments/reload remain tracked by `#COND.20`/`#COND.28`)
- [x] **#COND.8** Port `ReferenceId` row semantics — negative `ConditionTypeOrReference` rows keep `ReferenceId`, negative `SourceTypeOrReferenceId` rows are stored under `CONDITION_SOURCE_TYPE_REFERENCE_CONDITION`; C++ does **not** expand them flat during load, it resolves them recursively at evaluation/searcher-mask time (M)
- [x] **#COND.9** Implement `Condition::meets` evaluator — the giant switch over `ConditionType`. Split by category: presence/equality (NONE, ZONEID, AREAID, MAPID, TEAM, CLASS, RACE, GENDER, LEVEL, ALIVE, IN_WATER, GAMEMASTER, CHARMED, TAXI, PRIVATE_OBJECT) (M)
- [x] **#COND.10** `Condition::meets` — inventory and progression (ITEM, ITEM_EQUIPPED, SKILL, SPELL, ACHIEVEMENT, REALM_ACHIEVEMENT, TITLE, BATTLE_PET_COUNT) (M; uses explicit inventory/progression snapshots until the evaluator is wired directly to Player/session state)
- [x] **#COND.11** `Condition::meets` — quest (QUESTREWARDED, QUESTTAKEN, QUEST_NONE, QUEST_COMPLETE, QUESTSTATE, QUEST_OBJECTIVE_PROGRESS, DAILY_QUEST_DONE) (M; uses explicit quest/objective/daily snapshots until the evaluator is wired directly to Player/ObjectMgr state)
- [x] **#COND.12** `Condition::meets` — combat / unit-state (AURA, UNIT_STATE, HP_VAL, HP_PCT, STAND_STATE, DRUNKENSTATE, PET_TYPE, CREATURE_TYPE) (M)
- [x] **#COND.13** `Condition::meets` — relational (RELATION_TO with 6 RelationType variants, REACTION_TO with rank-mask, DISTANCE_TO with ComparisonType, NEAR_CREATURE, NEAR_GAMEOBJECT) (M; relation and nearby object state use explicit snapshots until wired to social/ownership/vehicle/grid systems)
- [x] **#COND.14** `Condition::meets` — world/server (WORLD_STATE, ACTIVE_EVENT, INSTANCE_INFO with C++-handled DATA/BOSS_STATE/DATA64 modes, REPUTATION_RANK, DIFFICULTY_ID) (M; map/server and reputation state use explicit snapshots until wired to WorldStateMgr/GameEventMgr/Map/Player reputation)
- [ ] **#COND.15** `Condition::meets` — phasing/scenario/scene (PHASEID, TERRAIN_SWAP, SCENARIO_STEP, SCENE_IN_PROGRESS, PLAYER_CONDITION) — note: PLAYER_CONDITION delegates to `IsPlayerMeetingCondition` over a DB2 entry (M; partial: `PHASEID`, `TERRAIN_SWAP`, `SCENARIO_STEP`, `SCENE_IN_PROGRESS`; `PLAYER_CONDITION` remains blocked on #COND.23)
- [x] **#COND.16** `Condition::meets` — object identity (OBJECT_ENTRY_GUID, TYPE_MASK, OBJECT_ENTRY_GUID_LEGACY, TYPE_MASK_LEGACY, STRING_ID) (M; object entry/type-mask aliases with explicit creature/gameobject spawn-id snapshots and string-id snapshots implemented)
- [x] **#COND.17** Implement `IsObjectMeetToConditions` family with the OR-of-AND semantics (group by `ElseGroup`, AND inside each, OR across) and `NegativeCondition` flip per row (`ElseGroup` aggregation and reference expansion are implemented; individual `Condition::meets` including `NegativeCondition` remains split across #COND.9-#COND.16) (M)
- [x] **#COND.18** Implement `IsObjectMeetingNotGroupedConditions` (single-bucket lookup → evaluate); used by Phasing, AreaTriggers, Graveyards, ObjectVisibility (M)
- [x] **#COND.19** Implement specialized lookups: `IsObjectMeetingSpellClickConditions(creatureId, spellId, …)`, `IsObjectMeetingVehicleSpellConditions`, `IsObjectMeetingSmartEventConditions`, `IsObjectMeetingVendorItemConditions`, `IsObjectMeetingTrainerSpellConditions`, `IsObjectMeetingVisibilityByObjectIdConditions` — each has its own keyed map built during load (implemented with C++ key parity and target ordering; load-time auxiliary sets such as `SpellsUsedInSpellClickConditions` remain tracked by #COND.20) (H)
- [ ] **#COND.20** Implement source-type index builders: `add_to_loot_template`, `add_to_gossip_menus`, `add_to_gossip_menu_items`, `add_to_spell_implicit_target_conditions`, `add_to_phases`, `add_to_graveyard_data` (H — depends on Loot/Gossip/Phasing/Graveyard data structures already existing or being co-built; partial: `add_to_gossip_menus`, `add_to_gossip_menu_items`, `add_to_spell_implicit_target_conditions`, `add_to_phases`, `add_to_graveyard_data`, represented final attachment pass, loot C++ key/target bridge, startup ConditionMgr store load, startup gossip/spell/phase/graveyard condition attachment diagnostics, `SpellsUsedInSpellClickConditions`, load-time SpellImplicitTarget effect-mask normalization, and production gossip/vendor/trainer condition filtering through the loaded `ConditionMgr` store are ported; reload and remaining production consumer callsite wiring remain open)
- [x] **#COND.21** Implement `is_condition_type_valid` per type — validates `ConditionValue1/2/3/StringValue1` against the relevant store (spell exists, item exists, faction exists, area exists, etc.); log + drop invalid rows (XL — audited against C++ cases and implemented: static field-usage metadata, useless-value load warnings, legacy object/type-mask normalization, pure no-store validation, startup external-store checks for item/spell/zone/skill/map/phase/quest/objective/difficulty/faction/achievement/title/battle-pet/scenario/scene/player-condition/creature-template/gameobject-template/object-guid spawn existence and entry mismatch/active-event/world-state; `CONDITION_MAX` rows are rejected as invalid type like C++; `CONDITION_OBJECT_ENTRY_GUID` flags Value2/3 as useless for PLAYER/CORPSE like C++; `CONDITION_ZONEID` rejects subzones using `AreaTable.Flags1 & AreaFlags::IsSubzone` like C++; `CONDITION_STRING_ID` is accepted as a no-op validation case using archived TrinityCore to correct a local legacy omission)
- [x] **#COND.22** Implement `is_source_type_valid` per source — validates that the `(SourceGroup, SourceEntry)` references something real where C++ does so; gossip menu/menu option and smart event sources intentionally pass source validation and report missing data later during attachment/consumer paths, matching C++ `break` in `isSourceTypeValid`. Audited against all C++ source-type cases: loot-template existence/link target checks, spell implicit target mask/effect-shape normalization, quest-available source entry, NPC vendor item + creature template source, spell-based source entries, trainer source + spell entries, conversation line template entries filtered by `ConversationLine.db2`, vehicle/spell-click creature template sources, object-id visibility creature/gameobject template sources, server-side area-trigger template keys, client-triggered area-trigger DB2 entries, terrain-swap map existence, phase area existence, startup graveyard link validation against `world_safe_locs`/`graveyard_zone`, spawn-group template/non-system validation, and gossip/smart-event no-op source validation are implemented. (H)
- [ ] **#COND.23** Implement `IsPlayerMeetingCondition(player, &PlayerConditionEntry)` — large pure DB2 evaluator (~70 sub-checks: race, class, gender, native gender, power type, skill, language, min level, max level, max factionId, gender, ChrSpecializationID, areaGroup, mapId, teleport access, faction, achievement, lfg status, currency, content tuning, item slot, transmog, raceMask, classMask, prevQuestID, currQuestID, currentCompletedQuestID, spellID, itemID, currencyID, weatherID, ContentTuningID, …). Owns ~600 lines in C++. (XL; partial: `PlayerCondition.db2` real store with the C++ 147-field load layout, `PlayerConditionCompare`, `PlayerConditionLogic`, `ChrSpecializationEntry::OrderIndex/Role`, and a snapshot evaluator for race/class/gender/native gender, power, spec index/role, skills, language bounds, reputation min/max logic, PvP faction/medal/rank, movement flags, weapon subclass, party status, previous/current/current-complete quests, spells, item counts, currency counts, explored areas, auras/stacks, world-state expression, weather, achievements, LFG compare values, area/parent-area checks, expansion/tier gates, phase satisfied gate, quest-kill progress, average item levels, and modifier-tree ids; `Condition::Meets(CONDITION_PLAYER_CONDITION)` now delegates to the DB2 evaluator when `ConditionSourceInfo` carries a real `PlayerConditionStore` plus player-condition context, and remains `Unsupported` otherwise to avoid false positives; `world-server` startup now loads/retains the shared real `PlayerConditionStore` and external condition validation uses that store instead of an ID-only DB2 store; `WorldSession` now receives the shared store, builds a represented live player-condition context for race/class/gender, expansion, area, spells, inventory item counts, currency counts, previous/current/current-complete quest sets, visible auras, party status, mainhand subclass, and item-level averages, and wires that context into gossip, NPC vendor, and trainer SQL condition evaluation; `npc_vendor.PlayerConditionID` now mirrors C++ list/buy behavior by setting `PlayerConditionFailed` only for existing unmet DB2 conditions and blocking buys with `EQUIP_ERR_ITEM_LOCKED`; represented `CMSG_GAME_OBJ_USE` now mirrors `GameObject::MeetsInteractCondition` using the C++ `GameObjectTemplate::GetConditionID1` type/index switch before chest/gathering/fishing interactions; represented taxi helpers now mirror the two audited C++ direct consumers, `TaxiPathGraph::EdgeCost::EvaluateDistance` for `TaxiNodes.ConditionID` and `HandleActivateTaxiOpcode` for `MountXDisplay.PlayerConditionID`, but production activation remains blocked until `CMSG_ACTIVATE_TAXI`, `TaxiNodes/TaxiPath/MountXDisplay` stores, and route graph integration exist; account mount login now sends `battlenet_account_mounts` through `SMSG_ACCOUNT_MOUNT_UPDATE` instead of an always-empty packet, matching C++ collection packet shape; `MapDifficultyXCondition.db2` now loads at startup and represented session logic can report the first failed `MapDifficultyXCondition` id in C++ `OrderIndex` order for transfer-abort difficulty messaging; remaining: direct DB2 `PlayerConditionID` consumers outside SQL `conditions` rows (adventure-map/achievement/spell-visual/mount use gating/etc.), exact language manager resolution, real `DB2Manager::IsInArea` hierarchy data, exact `WowTime` range semantics, forced reputation ranks, real LFG manager values, PhasingHandler bridge, quest objective bridge, and account-wide achievement flag behavior)
- [x] **#COND.24** Implement `IsMeetingWorldStateExpression(map, &WorldStateExpressionEntry)` — RPN-style byte-coded evaluator (operators, immediate values, world-state lookups, function calls). (H; implemented in `wow-data::world_state_expression`: `WorldStateExpression.db2` store load, C++ value/comparison/operator/logic byte parser, world-state lookup context, nested `WSE_FUNCTION_WORLD_STATE_EXPRESSION`, C++ zero-return stubs for unsupported/unknown functions, deterministic `std::mt19937` first-draw parity for `WSE_FUNCTION_MERSENNE_RANDOM`, and startup loading in `world-server`; local legacy's final logic-loop condition is corrected to archived TrinityCore `rpos() >= size()` because the local `<` branch breaks chained logic and can read past end)
- [ ] **#COND.25** Implement `IsUnitMeetingCondition(a, b, &UnitConditionEntry)` (M)
- [x] **#COND.26** Implement `Condition::get_searcher_type_mask_for_condition` — for early-exit during grid searches (M)
- [x] **#COND.27** Implement `Condition::to_string(ext)` — the debug formatter used in `tc_log_error` (L)
- [ ] **#COND.28** Implement `ConditionMgr::clean` and reload semantics — drop and rebuild all index buckets, invalidate `weak_ptr`s in downstream `ConditionsReference` holders (M)
- [x] **#COND.29** Implement `ConditionsReference` weak-pointer wrapper for downstream modules (Phasing, Loot, Gossip) so they can hold a non-owning handle that survives reload safely (M; downstream wiring remains tracked by each source-type index builder task)
- [ ] **#COND.30** Implement `DisableMgr` with all 9 `DisableType` variants, `LoadDisables`, `IsDisabledFor` per type, and the convenience helpers `IsPathfindingEnabled`, `IsVMAPDisabledFor`, `IsMMAPDisabledFor` (H; partial: `crates/wow-data/src/disable_mgr.rs` now ports the C++ enum/flag constants, `disables` SQL row shape, load/parse store, spell map/area params, map type + difficulty flag validation, spell/map/vmap/mmap/quest/bg/outdoorpvp/criteria/LFG runtime semantics, `IsPathfindingEnabled`, `IsVMAPDisabledFor`; `world-server` loads and retains a shared `DisableMgrLikeCpp` after `MapDifficulty.db2`, `SpellStore`, quests, `Criteria.db2`, and `BattlemasterList.db2`, derives the MMAP startup gate from it, `CMSG_CAST_SPELL` now fails disabled player spells with C++ `SPELL_FAILED_SPELL_UNAVAILABLE`, questgiver availability/accept/reward paths honor disabled quests, and teleports to disabled maps are rejected with C++ `SMSG_TRANSFER_ABORTED/TRANSFER_ABORT_MAP_NOT_ALLOWED`; remaining: quest post-load timing parity vs C++ `CheckQuestDisables` and replacing future vmap/LOS/LFG/battleground callsites with this shared store)
- [x] **#COND.31** Wire `sConditionMgr` access pattern (singleton via `OnceCell` / `&'static`) into the world startup sequence so dependents can call it after `LoadConditions` (L; implemented as a process-wide replaceable `Arc<ConditionEntriesByTypeStore>` slot installed after startup load; reload invalidation and downstream weak-reference refresh remain tracked by #COND.28/#COND.20)
- [ ] **#COND.32** Documentation cross-links: `conditions.md` ↔ `phasing.md` (TERRAIN_SWAP, PHASE), `loot.md` (every LOOT_TEMPLATE source), `gossip.md` (when written), `spells.md` (SPELL_IMPLICIT_TARGET, SPELL_PROC, SPELL_CLICK_EVENT) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#CONDITIONS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 4800 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h`. Rust target: `crates/wow-data`, `crates/wow-logging`. | `cargo test -p wow-data && cargo test -p wow-logging` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#CONDITIONS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 4800 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h`. Rust target: `crates/wow-data`, `crates/wow-logging`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#CONDITIONS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 4800 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h`. Rust target: `crates/wow-data`, `crates/wow-logging`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#CONDITIONS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 4800 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h`. Rust target: `crates/wow-data`, `crates/wow-logging`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [x] Test: `conditions` table loader rejects rows with bogus `SourceType` and logs the error. (Parser skip reason covered; logging adapter remains part of final load-service wiring.)
- [ ] Test: `conditions` table loader rejects rows with bogus `(SourceType, SourceEntry)` (e.g. CONDITION_SOURCE_TYPE_NPC_VENDOR with non-existent creatureId).
- [x] Test: `ReferenceId` resolves correctly — negative source/type rows are parsed into reference-template buckets and runtime evaluation recursively expands `ReferenceId` buckets, matching C++ load/evaluation split.
- [ ] Test: OR-of-AND semantics — group A `{X AND Y}`, group B `{Z}`. Pass X+Y but not Z → meets. Pass Z but neither X nor Y → meets. Pass nothing → fail.
- [x] Test: `NegativeCondition = 1` flips the row's truth value before AND-aggregation.
- [ ] Test: each ConditionType evaluator — one positive and one negative sample per ConditionType, fed a synthetic `WorldObject` fixture; for the Player-state types, a synthetic `Player` fixture.
- [ ] Test: `CONDITION_REPUTATION_RANK` with rankMask — Honored set, mask = `1 << REP_HONORED` → meets; mask = `1 << REP_EXALTED` → fails.
- [ ] Test: `CONDITION_QUESTSTATE` with state mask covering INCOMPLETE+COMPLETE — both states pass, NONE fails.
- [ ] Test: `IsObjectMeetingSpellClickConditions` evaluates conditions against the **clicker** (target0) and **clicked target** (target1); swapped roles fail.
- [ ] Test: `IsObjectMeetingNotGroupedConditions(SOURCE_TYPE_PHASE, areaId)` returns true when the area phase's conditions pass for a player in that area.
- [ ] Test: `IsPlayerMeetingCondition` evaluates each PlayerCondition.db2 sub-check in isolation (tabular; partial: compare/logic helpers, race/class/gender, spells/items/currency/auras, party/area snapshot checks, and `Condition::Meets(CONDITION_PLAYER_CONDITION)` delegation/missing-entry behavior).
- [x] Test: `IsMeetingWorldStateExpression` evaluates core C++ bytecode behavior: constants, world-state lookup, comparison, arithmetic, division-by-zero, chained AND/OR/XOR logic, nested expression function, and Mersenne random first draw.
- [ ] Test: ConditionMgr reload — load v1, reload v2, downstream `ConditionsReference` holders see the v2 buckets; v1 buckets dropped.
- [x] Test: `DisableMgr::IsDisabledFor(SPELL, spellId, unit)` honours per-type flags and map/area params for player contexts; see `cargo test -p wow-data disable_mgr`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 4800 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h` | `crates/wow-data/` (conditions table row parsing, static validation, storage), `crates/wow-world/src/conditions.rs` (runtime condition evaluation and lookup helpers), downstream modules still to wire. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#CONDITIONS.DIV.001` | `crates/wow-world/src/conditions` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 4800 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/DisableMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- The conditions DB schema is a single denormalized table and reuses the same columns for every type, so `ConditionValue1/2/3` mean different things depending on `ConditionType`. The C++ `StaticConditionTypeData[]` table at the bottom of `ConditionMgr.cpp` is the source of truth for "which slots are used" — port it verbatim.
- `ElseGroup = 0` is treated as "default group". Two rows with `ElseGroup = 0` and `ElseGroup = 1` form an OR; two rows with both `ElseGroup = 0` form an AND.
- `ConditionTarget` ∈ {0, 1, 2} selects which entry in `ConditionSourceInfo::mConditionTargets[]` to evaluate against. Most rows use 0 (the primary target). For 2-target source types like `SPELL_CLICK_EVENT`, target 1 is the clicked unit.
- `mLastFailedCondition` is mutated during evaluation and is the *only* way for downstream code to discover which condition caused the failure (for `ErrorType`/`ErrorTextId` reporting). Don't make `Condition::meets` take an immutable `&self` and lose this slot.
- `CONDITION_AURA`'s `ConditionValue2` is the effect index — many old rows incorrectly have it set to 0 even when the spell only has the aura on effect 1. The validator lets these through but evaluation may silently fail. Flag as warning, not error.
- `CONDITION_OBJECT_ENTRY_GUID_LEGACY` and `CONDITION_TYPE_MASK_LEGACY` exist for backward compatibility; new rows should use the non-LEGACY versions. Validator warns but accepts.
- `CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE` is for entries inside `reference_loot_template` (loot rolling references shared across multiple loot tables) — not the same as `ReferenceId` (which is a row-level reference inside the `conditions` table itself). Two distinct concepts, easy to confuse.
- Do not evaluate conditions during DB load — many sub-checks need DB2 stores, the world map, etc. that are not yet ready. The validators verify *referential integrity* only.
- `IsPlayerMeetingCondition` is *also* used by `Vendor` filtering, `QuestGiver` icons, `WorldQuest` availability, `Item.db2`, transmog, and `Conversation` — it is the busiest single function in this module. Performance matters.
- `DisableMgr::IsDisabledFor(MAP, mapId, ...)` is called from `Map::IsBattlegroundOrArena` paths; `_VMAP` and `_MMAP` are called from the pathfinding crate. Path-disable is *map-specific*, not global.
- WoLK 3.4.3 specific: `CONDITION_BATTLE_PET_COUNT` (53), `CONDITION_SCENARIO_STEP` (54), `CONDITION_SCENE_IN_PROGRESS` (55), `CONDITION_PLAYER_CONDITION` (56), `CONDITION_PRIVATE_OBJECT` (57), `CONDITION_STRING_ID` (58) all exist in 3.4.3 even though some are mostly retail-driven. Implement the structural support; runtime hits may be rare.
- The `SourceId` field is *only* used by `CONDITION_SOURCE_TYPE_SMART_EVENT` (to distinguish multiple event entries on the same SmartAI). For every other source type it must be 0 — `ConditionMgr::CanHaveSourceIdSet` enforces this.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `enum ConditionTypes : int` | `#[repr(u32)] enum ConditionType` (in `crates/wow-constants`) | Use `TryFrom<u32>` for DB row parsing; reject unknown values |
| `enum ConditionSourceType` | `#[repr(u32)] enum ConditionSourceType` | Same |
| `struct Condition` | `struct Condition` (in `crates/wow-data`) | All fields owned (no pointers) |
| `typedef vector<Condition> ConditionContainer` | `type ConditionContainer = Vec<Condition>;` | — |
| `unordered_map<ConditionId, shared_ptr<vector<Condition>>>` | `HashMap<ConditionId, Arc<ConditionContainer>>` | `Arc` so downstream `Weak` can survive reload |
| `weak_ptr<ConditionContainer>` (in `ConditionsReference`) | `Weak<ConditionContainer>` | `upgrade()` returns `None` after reload — handle gracefully |
| `array<ConditionsByEntryMap, CONDITION_SOURCE_TYPE_MAX>` | `[HashMap<ConditionId, Arc<ConditionContainer>>; SOURCE_TYPE_MAX]` | Const-array sized at compile time |
| `class ConditionMgr` (singleton) | `pub struct ConditionMgr { … }` + `static CONDITION_MGR: OnceCell<ConditionMgr>` | Same access pattern as other Mgrs in the project |
| `bool Condition::Meets(ConditionSourceInfo&)` | `fn meets(&self, info: &mut ConditionSourceInfo) -> bool` | `&mut` for `mLastFailedCondition` write-back |
| `static bool ConditionMgr::IsPlayerMeetingCondition(Player const*, PlayerConditionEntry const*)` | `fn is_player_meeting_condition(player: &Player, entry: &PlayerConditionEntry) -> bool` | Pure function, no `self` |
| `static bool ConditionMgr::IsMeetingWorldStateExpression(Map const*, WorldStateExpressionEntry const*)` | `wow_data::is_meeting_world_state_expression_like_cpp(expr, &WorldStateExpressionContextLikeCpp)` | Context carries map-derived difficulty/world-state/time/region data; local legacy loop typo corrected to archived TrinityCore semantics |
| `class DisableMgr` (singleton) | `pub struct DisableMgr { … }` similar | — |
| `enum DisableType` | `#[repr(u8)] enum DisableType` | 8 variants |
| `Condition::ConditionStringValue1` (`std::string`) | `String` (or `Option<Box<str>>` to save 24 bytes when empty) | Most rows are empty — boxed str saves a lot |
| Static metadata `StaticSourceTypeData[]` and `StaticConditionTypeData[]` | `const SOURCE_TYPE_DATA: [&'static str; …]`, `const CONDITION_TYPE_DATA: [ConditionTypeInfo; …]` | Port verbatim from C++ — they're the spec |
| Logging via `TC_LOG_ERROR("sql.sql", …)` | `wow_logging::condition_error!("sql.sql: {}", …)` (or generic `log!`) | Use the existing `LogFilter::Condition` |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Historical initial-audit verdict:** ConditionMgr was absent when this plan was opened. That is no longer true: constants, row shapes, SQL projection, parser, grouping, partial evaluator, specialized lookups, static validation, reference semantics, and phase attachment have since been added. The remaining risk is now narrower: any consumer not wired to the new store/evaluator still behaves permissively until its source-type builder/callsite is closed.

Current downstream gaps still capable of silent permissive behaviour:

| Consumer | File | Behaviour right now |
|---|---|---|
| Loot drops | `crates/wow-loot/`, `crates/wow-world/src/conditions.rs` | Loot condition link/reference checking and C++ key/target runtime bridge exist, but all production loot fill callsites still need to pass the ConditionMgr-backed predicate. |
| NPC vendors | `wow-packet/src/packets/misc.rs:1686` | Vendor list and buy paths now evaluate `CONDITION_SOURCE_TYPE_NPC_VENDOR` through the loaded `ConditionMgr` store with player+vendor targets, and evaluate `npc_vendor.PlayerConditionID` through the shared `PlayerConditionStore` for list `PlayerConditionFailed` and buy `ItemLocked` parity. |
| Gossip menus / options | `crates/wow-data/src/gossip.rs`, gossip dispatch in `wow-world/src/handlers/` | C++ gossip condition holders, startup attachment diagnostics and runtime gossip text/option filtering now consult the loaded `ConditionMgr` store while the handler still queries DB on demand. |
| Trainer spells | `handlers/trainer.rs` | Trainer list spell visibility now consults the loaded `ConditionMgr` store using the corrected `TRAINER_SPELL` source type; purchase-side revalidation remains pending. |
| Taxi route / mount display | `crates/wow-world/src/session.rs` | C++ semantics for `TaxiNodes.ConditionID` edge rejection and `MountXDisplay.PlayerConditionID` filtering are captured in represented helpers/tests; production `CMSG_ACTIVATE_TAXI` route/mount integration is still absent. |
| Mount aura display | `crates/wow-data/src/spell.rs`, `crates/wow-data/src/creature_display.rs`, `crates/wow-data/src/vehicle.rs`, `crates/wow-packet/src/packets/pet.rs`, `crates/wow-packet/src/packets/vehicle.rs`, `crates/wow-world/src/session.rs` | `SPELL_AURA_MOUNTED` can now use retained `SpellEffect` misc fields plus Mount/MountXDisplay condition filtering and random usable-display selection to apply/remove represented mount display and `UNIT_FLAG_MOUNT` deltas, with creature_template_model fallback, `Vehicle.db2` validation plus installed `wow_entities::Vehicle` construction from `VehicleSeat.db2` seat definitions, C++ vehicle accessory row load and GUID-first/template fallback selection, owner `SMSG_MOVE_SET_VEHICLE_REC_ID`/`SMSG_SET_VEHICLE_REC_ID` packets plus `SMSG_ON_CANCEL_EXPECTED_RIDE_VEHICLE_AURA` for represented VehicleKit create/remove, `Unit::GetCollisionHeight()` recalculation from `CreatureDisplayInfo/CreatureModelData`, owner `SMSG_MOVE_SET_COLLISION_HEIGHT` plus broadcast-to-set `SMSG_MOVE_UPDATE_COLLISION_HEIGHT` on mount/dismount, represented `SMSG_PET_MODE` disable/enable emission when a session pet GUID is known, and `CMSG_MOVE_SET_VEHICLE_REC_ID_ACK` movement-info validation; exact VehicleKit passenger runtime, accessory TempSummon/HandleSpellClick install, full pet runtime ownership, and mount pet resummon execution remain pending. |
| Account mounts | `crates/wow-packet/src/packets/misc.rs`, `crates/wow-world/src/handlers/character.rs` | Login now loads `battlenet_account_mounts` by BNet account and sends `SMSG_ACCOUNT_MOUNT_UPDATE` entries; C++ `CollectionMgr::AddMount` spell-learning gate by `Mount.PlayerConditionID` still waits on real mount DB2/collection runtime. |
| Map difficulty conditions | `crates/wow-data/src/map.rs`, `crates/wow-world/src/session.rs` | `MapDifficultyXCondition.db2` is loaded/grouped like C++ and represented logic returns the first unmet condition id; live `Player::Satisfy`/transfer-abort integration remains pending. |
| Spell implicit targets | `wow-data::SpellStore`, future `wow-spell` targeting | C++-style condition handles are now attached per represented spell effect; actual AoE/chained target filtering still waits on the spell target-selection runtime. |
| Phasing area entry | `crates/wow-data/src/phasing.rs`, `crates/wow-world/src/phasing.rs` | Phase-area load and C++ `addToPhases` attachment are ported; global startup/reload wiring and live evaluator integration remain open. |
| Smart-script branches | not yet implemented | N/A. |

**Migration unblock priority:** ConditionMgr is the L7 keystone. Phasing, Loot quest filtering, Gossip polish, Vendors, Trainers all unblock here. Continue closing **#COND.7**, **#COND.20**, **#COND.21**, and **#COND.22** before dependent modules can claim full parity.
