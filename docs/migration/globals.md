# Migration: Globals (ObjectMgr / ObjectAccessor / DataStores)

> **C++ canonical path:** `src/server/game/Globals/`
> **Rust target crate(s):** `wow-data` (templates), `wow-database` (loaders), `wow-world` (live registries / `MapManager`), `wow-network` (`PlayerRegistry`)
> **Layer:** L0 (foundation — touched by virtually every other module)
> **Status:** ⚠️ partial — fragmented across 5+ crates; no central `ObjectMgr` analogue
> **Audited vs C++:** ✅ complete (audit 2026-05-01)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The `Globals/` directory hosts the central in-memory caches and lookup services that every other game subsystem reads from. **`ObjectMgr`** is the singleton that loads, validates and stores all static world data (creature/gameobject/item templates, quests, spawns, vendors, trainers, gossip, graveyards, area triggers, instance templates, scripts…) from `world` DB into RAM at server startup. **`ObjectAccessor`** is the live-object lookup layer (GUID → `Player*` / `Creature*` / `GameObject*` / etc.) that wraps a set of per-type `HashMapHolder` registries. The remaining `*DataStore` files (AreaTrigger, CharacterTemplate, Conversation) load specialised data clusters that ObjectMgr does not own.

Together this module is the canonical "is X already known to the server?" entry point. Without it, no other module can resolve an entry id, a spawn id, or a runtime GUID.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Globals/AreaTriggerDataStore.cpp` | 448 | `prefix` |
| `game/Globals/AreaTriggerDataStore.h` | 49 | `prefix` |
| `game/Globals/CharacterTemplateDataStore.cpp` | 118 | `prefix` |
| `game/Globals/CharacterTemplateDataStore.h` | 60 | `prefix` |
| `game/Globals/ConversationDataStore.cpp` | 283 | `prefix` |
| `game/Globals/ConversationDataStore.h` | 114 | `prefix` |
| `game/Globals/ObjectAccessor.cpp` | 309 | `prefix` |
| `game/Globals/ObjectAccessor.h` | 114 | `prefix` |
| `game/Globals/ObjectMgr.cpp` | 11444 | `prefix` |
| `game/Globals/ObjectMgr.h` | 1944 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Globals/ObjectMgr.h` | 1944 | All struct/typedef declarations + `class ObjectMgr` (singleton) public API: ~120 `Load*` methods + ~150 `Get*`/`IsXxx` accessors |
| `src/server/game/Globals/ObjectMgr.cpp` | **11444** | Implementation of every `Load*` (each is a SQL query → in-memory container fill) plus consistency validation, ID generators, GUID generators |
| `src/server/game/Globals/ObjectAccessor.h` | 114 | `template<class T> HashMapHolder` + free functions `GetWorldObject` / `GetCreature` / `GetPlayer` / `FindPlayer` etc. |
| `src/server/game/Globals/ObjectAccessor.cpp` | 309 | Implementation: per-type `HashMapHolder<T>::Insert/Remove/Find`, locked iteration, `SaveAllPlayers()` |
| `src/server/game/Globals/AreaTriggerDataStore.h` | 49 | Singleton interface for area trigger templates / spawns / create properties |
| `src/server/game/Globals/AreaTriggerDataStore.cpp` | 448 | Loads `areatrigger_template`, `areatrigger_create_properties`, action/polygon/spline data |
| `src/server/game/Globals/CharacterTemplateDataStore.h` | 60 | Templates for `chargen` (character_template + character_template_class tables) |
| `src/server/game/Globals/CharacterTemplateDataStore.cpp` | 118 | Loader |
| `src/server/game/Globals/ConversationDataStore.h` | 114 | Conversation templates (NPC dialog scripts) |
| `src/server/game/Globals/ConversationDataStore.cpp` | 283 | Loader |

`GameObjectModel` is referenced by the user prompt but lives at `src/server/game/Collision/Models/GameObjectModel.{h,cpp}` (not under Globals/). It belongs to the **maps/collision** doc; mentioned here because it is one of the few static data caches that ObjectMgr does not own.

---

## 3. Classes / Structs / Enums

(Selected — full list runs to ~80 structs in ObjectMgr.h alone.)

| Symbol | Kind | Purpose |
|---|---|---|
| `ObjectMgr` | class (singleton) | Master cache + loader. Friend class of `PlayerDumpReader`. Non-copyable, non-movable |
| `HashMapHolder<T>` | class template | Per-type GUID→`T*` map with `std::shared_mutex` (used for Player, Transport, etc.) |
| `ObjectAccessor` | namespace | Free functions for lookup; not a class |
| `CreatureData` | struct | Spawn row (`creature` table). Position, displayId, equipment, spawn group, phase mask |
| `GameObjectData` | struct | Spawn row (`gameobject` table) |
| `SpawnData` / `SpawnMetadata` | struct | Polymorphic base for above |
| `CellObjectGuids` | struct | Set of creature/GO/corpse spawn ids per grid cell |
| `InstanceTemplate` | struct | Per-map instance config (script id, parent map) |
| `AreaTriggerStruct` | struct | Teleport area trigger row (target_map/x/y/z/orientation) |
| `AccessRequirement` | struct | Heroic/raid keystone & ilvl requirements |
| `GameTele` | struct | `.tele` GM command destinations |
| `ScriptInfo` | struct | One row of `*_scripts` (event/spell/quest scripts) |
| `PlayerInfo` | struct | Race+class create info (start map/position, items, actions, level info table) |
| `PlayerLevelInfo` | struct | Per-level stat array |
| `PetLevelInfo` | struct | Per-creature-id pet level stats |
| `MailLevelReward` | struct | Race-mask level-up mail reward |
| `RepRewardRate`, `ReputationOnKillEntry`, `RepSpilloverTemplate` | struct | Reputation tuning |
| `PointOfInterest`, `QuestPOIData`, `QuestPOIBlobData` | struct | Map POI markers |
| `GossipMenus`, `GossipMenuItems`, `GossipMenuAddon` | struct | NPC dialog menus |
| `WorldSafeLocsEntry`, `GraveyardData` | struct | Graveyard spawn locations |
| `TempSummonData`, `TempSummonGroupKey` | struct | Pre-defined summon groups |
| `SpawnGroupTemplateData`, `InstanceSpawnGroupInfo` | struct | Spawn group config |
| `SkillTiersEntry` | struct | Profession tier caps |
| `SceneTemplate`, `PlayerChoice`, `PlayerChoiceResponse` | struct | Cinematic / choice-popup data |
| `PhaseInfoStruct`, `PhaseAreaInfo`, `TerrainSwapInfo` | struct | Phasing / terrain swap rules |
| `ScriptNameContainer` | inner class | Bidirectional `name ↔ script_id` registry (interned strings) |
| `QuestRelations`, `QuestRelationsReverse`, `QuestRelationResult` | typedef/struct | Quest giver/ender multimap |
| `QueryDataGroup` | enum flags | Bitmask for `InitializeQueriesData` (CREATURES, GAMEOBJECTS, ITEMS, QUESTS, POIS) |
| `ExtendedPlayerName` | struct | `Name@Realm` parsed pair |
| `BattlenetRpcErrorCode` | enum | (defined in `proto/`, but ObjectMgr returns these too) |

---

## 4. Critical public methods / functions

### `ObjectMgr` — top getters / mutators

| Symbol | Purpose | Calls into |
|---|---|---|
| `instance()` | Singleton accessor | — |
| `GetCreatureTemplate(uint32 entry)` | Resolve creature by id | `_creatureTemplateStore` |
| `GetGameObjectTemplate(uint32 entry)` | Resolve GO by id | `_gameObjectTemplateStore` |
| `GetItemTemplate(uint32 entry)` | Resolve item by id | `_itemTemplateStore` |
| `GetQuestTemplate(uint32 questId)` | Resolve quest by id | `_questTemplates` |
| `GetCreatureData(spawnId)` / `GetGameObjectData(spawnId)` | Spawn row lookup | per-store map |
| `GetCellObjectGuids(map, diff, cellId)` | Grid cell loader uses this | `_mapObjectGuidsStore` |
| `GetClosestGraveyard(loc, team, conditionObj)` | Death/release teleport | `_graveyardStore` + conditions |
| `GetAreaTrigger(triggerId)`, `GetGoBackTrigger(map)`, `GetMapEntranceTrigger(map)` | Area trigger resolution | `_areaTriggerStore` |
| `GetAccessRequirement(map, diff)` | Instance lock check | `_accessRequirementStore` |
| `GetPlayerInfo(race, class)` | Char creation lookup | `_playerInfo[race][class]` |
| `GetPetLevelInfo(creatureId, level)` | Pet stat curve | `_petInfoStore` |
| `GetGossipMenuItemsMapBounds(menuId)` | Render NPC menu | `_gossipMenuItemsStore` |
| `GetVendorItemData(entry)` / `AddVendorItem(...)` | Vendor inventory | `_cacheVendorItemStore` |
| `GeneratePetName(entry)` | Pet auto-name | `_petHalfName0/1` |
| `GenerateAuctionID()` / `GenerateMailID()` / `GeneratePetNumber()` / `GenerateEquipmentSetGuid()` / `GenerateCreatureSpawnId()` / `GenerateGameObjectSpawnId()` / `GetGenerator<HighGuid>()` | Atomic id allocators | `std::atomic` counters / per-HighGuid `ObjectGuidGenerator` |
| `AddCreatureToGrid` / `RemoveCreatureFromGrid` / `AddGameobjectToGrid` / `RemoveGameobjectFromGrid` | Grid-cell spawn registration | `_mapObjectGuidsStore` |
| `IsReservedName(name)` / `IsProfanityName(name)` | Name validation | `_reservedNamesStore` |
| `normalizePlayerName(string&)` (free fn) | UTF-8 first-letter capitalisation | locale tables |
| `LoadTrinityStrings()` | i18n string table | `trinity_string` |
| `SetHighestGuids()` | Initialise generators from DB max | All Generate* counters |
| `InitializeQueriesData(QueryDataGroup mask)` | Pre-build cached query packets for clients | All `*Template` stores |
| `ReturnOrDeleteOldMails(serverUp)` | Cron over mail | `Mail` queries |

### `ObjectMgr::Load*` — major loaders (~120 total in `.cpp`)

Listed in source order from `ObjectMgr.cpp`. Each is a SQL `SELECT` followed by row-by-row population of an in-memory `unordered_map`. Validation logging is per-row.

Creature stack: `LoadCreatureLocales`, `LoadCreatureTemplates`, `LoadCreatureTemplateGossip`, `LoadCreatureTemplateResistances`, `LoadCreatureTemplateSpells`, `LoadCreatureTemplateModels`, `LoadCreatureSummonedData`, `LoadCreatureTemplateAddons`, `LoadCreatureTemplateSparring`, `LoadCreatureTemplateDifficulty`, `LoadCreatureAddons`, `LoadCreatureModelInfo`, `LoadCreatureMovementOverrides`, `LoadEquipmentTemplates`, `LoadCreatureClassLevelStats`, `LoadCreatures`, `LoadLinkedRespawn`, `LoadTempSummons`, `LoadCreatureQuestItems`, `LoadCreatureQuestStarters`, `LoadCreatureQuestEnders`, `LoadCreatureTrainers`.

GameObject stack: `LoadGameObjectLocales`, `LoadGameObjectTemplate`, `LoadGameObjectTemplateAddons`, `LoadGameObjectOverrides`, `LoadGameObjectAddons`, `LoadGameObjects`, `LoadGameObjectQuestItems`, `LoadGameobjectQuestStarters`, `LoadGameobjectQuestEnders`, `LoadGameObjectForQuests`.

Spawn groups: `LoadSpawnGroupTemplates`, `LoadSpawnGroups`, `LoadInstanceSpawnGroups`.

Items: `LoadItemTemplates`, `LoadItemTemplateAddon`, `LoadItemScriptNames`.

Vehicles: `LoadVehicleTemplate`, `LoadVehicleTemplateAccessories`, `LoadVehicleAccessories`, `LoadVehicleSeatAddon`.

Quests: `LoadQuests`, `LoadQuestStartersAndEnders`, `LoadQuestTemplateLocale`, `LoadQuestObjectivesLocale`, `LoadQuestGreetingLocales`, `LoadQuestOfferRewardLocale`, `LoadQuestRequestItemsLocale`, `LoadQuestAreaTriggers`, `LoadQuestGreetings`, `LoadQuestPOI`.

Scripts: `LoadScripts(ScriptsType)`, `LoadEventSet`, `LoadEventScripts`, `LoadSpellScripts`, `LoadSpellScriptNames`, `ValidateSpellScripts`.

Page text / NPC text: `LoadPageTexts`, `LoadPageTextLocales`, `LoadNPCText`.

Triggers: `LoadAreaTriggerTeleports`, `LoadAccessRequirements`, `LoadAreaTriggerScripts`, `LoadTavernAreaTriggers`.

Graveyards / safe locs: `LoadGraveyardZones`, `LoadWorldSafeLocs`.

Player / pet: `LoadPlayerInfo` (race × class × level), `LoadPetLevelInfo`, `LoadExplorationBaseXP`, `LoadPetNames`, `LoadPetNumber`, `LoadFishingBaseSkillLevel`, `LoadSkillTiers`.

Reputation: `LoadReputationRewardRate`, `LoadReputationOnKill`, `LoadReputationSpilloverTemplate`.

POIs: `LoadPointsOfInterest`, `LoadPointOfInterestLocales`.

NPC services: `LoadNPCSpellClickSpells`, `LoadVendors`, `LoadTrainers`, `LoadGossipMenu`, `LoadGossipMenuItems`, `LoadGossipMenuItemsLocales`, `LoadGossipMenuAddon`.

Cross-faction: `LoadFactionChangeAchievements`, `LoadFactionChangeItems`, `LoadFactionChangeQuests`, `LoadFactionChangeReputations`, `LoadFactionChangeSpells`, `LoadFactionChangeTitles`.

Phasing / terrain: `LoadPhases`, `LoadPhaseNames`, `UnloadPhaseConditions`, `LoadTerrainSwapDefaults`, `LoadTerrainWorldMaps`, `LoadAreaPhases`.

Misc: `LoadInstanceTemplate`, `LoadMailLevelRewards`, `LoadGameTele`, `LoadReservedPlayersNames`, `LoadSceneTemplates`, `LoadPlayerChoices`, `LoadPlayerChoicesLocale`, `LoadJumpChargeParams`, `LoadRaceAndClassExpansionRequirements`.

### `ObjectAccessor` — namespace functions

| Symbol | Purpose |
|---|---|
| `GetWorldObject(WorldObject const&, ObjectGuid)` | Generic lookup scoped to a map (uses calling object's map) |
| `GetObjectByTypeMask(WorldObject const&, ObjectGuid, typemask)` | Type-filtered lookup |
| `GetCreature` / `GetPlayer` / `GetGameObject` / `GetUnit` / `GetCorpse` / `GetTransport` / `GetDynamicObject` / `GetAreaTrigger` / `GetSceneObject` / `GetConversation` / `GetPet` | Per-type variants — return only objects in the same map as the reference |
| `GetCreatureOrPetOrVehicle(WorldObject const&, ObjectGuid)` | Combined creature-family lookup |
| `FindPlayer(ObjectGuid)` | Whole-server (any map) — **NOT thread safe** |
| `FindPlayerByName(name)` / `FindPlayerByLowGUID(low)` | Whole-server name lookups |
| `FindConnectedPlayer(...)` | Includes players currently teleporting (no map) |
| `GetPlayers()` | Read-only handle to global player map (caller must hold lock) |
| `AddObject<T>(T*)` / `RemoveObject<T>(T*)` | Templates that route into `HashMapHolder<T>` |
| `SaveAllPlayers()` | Iterates the player map and saves each |

---

## 5. Module dependencies

**Depends on:**
- `wow-database` / DB access — every `Load*` issues a `WorldDatabase.Query(...)`
- `DBC/DB2 stores` — many loaders cross-reference `sCreatureFamilyStore`, `sFactionStore`, `sMapStore`, `sChrRacesStore`, `sChrClassesStore` (loaded by `DB2Stores.cpp`, not ObjectMgr — but ObjectMgr will refuse to load before they're ready)
- `Conditions` (`ConditionMgr`) — used by graveyards, gossip, phasing
- `Spell` definitions — to validate `LoadCreatureTemplateSpells`, `LoadNPCSpellClickSpells`, `LoadSpellScriptNames`
- `Script` system — `ScriptNameContainer` and the `Load*Scripts` family bind named scripts to ids
- `ObjectGuid` / `HighGuid` — for the per-type GUID generators

**Depended on by:**
- **Almost everything.** Specifically: `Map.cpp` (uses `GetCellObjectGuids` to populate grids), `Creature.cpp` & `GameObject.cpp` (template + spawn data resolution), `Player.cpp` (`PlayerInfo`, level info, vendors, trainers, gossip), `QuestObjectiveCriteriaMgr`, `LootMgr`, `PoolMgr`, `BattlegroundMgr`, every `WorldSession::Handle*` for entry-id validation, `ScriptMgr` (script id ↔ name resolution), `World::SetInitialWorldSettings()` (calls every `Load*` in dependency order at startup).

---

## 6. SQL / DB queries (if any)

`ObjectMgr` is **the** principal world-DB consumer. It does **not** use the prepared-statement registry like `Player.cpp` does — most loaders issue ad-hoc `WorldDatabase.Query("SELECT ... FROM ...")` strings directly inside each `Load*` function. Rough table list (one per loader, ~120 tables):

| Table | Loader |
|---|---|
| `creature_template` (+ `_locale`, `_addon`, `_resistance`, `_spell`, `_model`, `_difficulty`, `_gossip`, `_sparring`) | `LoadCreatureTemplate*` |
| `creature` (spawn rows) | `LoadCreatures` |
| `creature_addon`, `creature_movement_override`, `creature_classlevelstats`, `creature_template_addon`, `creature_summoned_data`, `creature_questitem`, `creature_queststarter`, `creature_questender`, `creature_template_difficulty` | per-loader |
| `gameobject_template` (+ `_locale`, `_addon`, `_overrides`) | `LoadGameObject*` |
| `gameobject`, `gameobject_addon`, `gameobject_questitem`, `gameobject_queststarter`, `gameobject_questender` | `LoadGameObject*` |
| `item_template`, `item_template_addon`, `item_script_names` | `LoadItem*` |
| `vehicle_template`, `vehicle_template_accessory`, `vehicle_accessory`, `vehicle_seat_addon` | `LoadVehicle*` |
| `quest_template`, `quest_objectives`, `quest_template_addon`, `quest_template_locale`, `quest_objectives_locale`, `quest_offer_reward`, `quest_request_items`, `quest_poi`, `quest_poi_points`, `quest_greeting`, `quest_greeting_locale`, `areatrigger_involvedrelation` | `LoadQuests*`, `LoadQuestPOI`, etc. |
| `instance_template` | `LoadInstanceTemplate` |
| `areatrigger_teleport`, `areatrigger_tavern`, `areatrigger_scripts` | `LoadAreaTrigger*` |
| `access_requirement` | `LoadAccessRequirements` |
| `graveyard_zone`, `world_safe_locs` | graveyard/safeloc loaders |
| `playercreateinfo` (+ `_action`, `_item`, `_skills`, `_spell_custom`) | `LoadPlayerInfo` |
| `player_levelstats`, `player_classlevelstats`, `player_xp_for_level` | level info |
| `pet_levelstats`, `pet_name_generation`, `pet_name_generation_locale` | pet loaders |
| `mail_level_reward` | `LoadMailLevelRewards` |
| `reputation_reward_rate`, `reputation_spillover_template`, `creature_onkill_reputation` | reputation loaders |
| `points_of_interest`, `points_of_interest_locale` | POI loaders |
| `gossip_menu`, `gossip_menu_option`, `gossip_menu_option_locale`, `gossip_menu_addon` | gossip loaders |
| `npc_vendor`, `npc_trainer`, `creature_default_trainer`, `npc_text`, `npc_spellclick_spells` | NPC services |
| `page_text`, `page_text_locale` | page text |
| `phase_definitions`, `phase_name`, `phase_area`, `terrain_swap_defaults`, `terrain_worldmap` | phasing |
| `scene_template` | `LoadSceneTemplates` |
| `playerchoice`, `playerchoice_response`, `playerchoice_response_reward_*`, `playerchoice_locale` | `LoadPlayerChoices*` |
| `jump_charge_params`, `skill_tiers`, `skill_fishing_base_level`, `exploration_basexp` | misc |
| `event_scripts`, `spell_scripts`, `spell_script_names`, `waypoint_scripts` etc. | `LoadScripts(ScriptsType)`, `LoadSpellScripts`, etc. |
| `game_tele`, `reserved_name`, `string_freeze_*`, `trinity_string` | misc + i18n |
| `playerfactionchange_*` (achievements, items, quests, reputations, spells, titles) | faction-change loaders |
| `game_event_*` (used by `GameEventMgr`, but ObjectMgr loads `event_scripts`) | scripts |

Plus DB2 stores (read by `DB2Stores`, used downstream by ObjectMgr getters): `Map`, `ChrRaces`, `ChrClasses`, `Faction`, `FactionTemplate`, `CreatureFamily`, `CreatureType`, `Item`, `SkillRaceClassInfo`, `SkillTiers`, etc.

GUID generators rely on `MAX(guid)` queries from `creature`, `gameobject`, `item_instance`, `mail`, `auction`, `character_pet`, `character_equipmentsets`, `character_void_storage` to set initial high-water marks (`SetHighestGuids()`).

---

## 7. Wire-protocol packets (if any)

`ObjectMgr` does not directly read or send packets, but it caches **pre-built query response packets** for client `CMSG_QUERY_*`:

| Opcode | Direction | Built/cached by |
|---|---|---|
| `SMSG_QUERY_CREATURE_RESPONSE` | server → client | `CreatureTemplate::InitializeQueryData()` (called by `InitializeQueriesData(QUERY_DATA_CREATURES)`) |
| `SMSG_QUERY_GAME_OBJECT_RESPONSE` | server → client | `GameObjectTemplate::InitializeQueryData()` |
| `SMSG_QUERY_ITEM_SINGLE_RESPONSE` | server → client | `ItemTemplate::InitializeQueryData()` |
| `SMSG_QUERY_QUEST_INFO_RESPONSE` | server → client | `Quest::InitializeQueryData()` |
| `SMSG_QUEST_POI_QUERY_RESPONSE` | server → client | `QuestPOIData::InitializeQueryData()` |
| `SMSG_QUERY_NPC_TEXT_RESPONSE` | server → client | via `NpcText` cache |
| `SMSG_QUERY_PAGE_TEXT_RESPONSE` | server → client | via `PageText` cache |

`ObjectAccessor` is purely process-internal.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |
| `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `crates/wow-database/src/statements/{world.rs,character.rs,login.rs,hotfix.rs}` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `crates/wow-network/src/player_registry.rs` | `file` | 1 | 47 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-data/src/{quest,item,player_stats}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |

<!-- REFINE.021:END rust-target-coverage -->

There is **no central `ObjectMgr` analogue** — its responsibilities are split across multiple crates with no unifying registry.

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/quest.rs` — quest template parsing (~templates only, not tied to a DB loader)
- `crates/wow-data/src/quest_xp.rs` — quest XP curves
- `crates/wow-data/src/item.rs` + `item_stats.rs` — item template parsing
- `crates/wow-data/src/spell.rs` — spell lookup
- `crates/wow-data/src/skill.rs` — skill data
- `crates/wow-data/src/area_trigger.rs` — area trigger data (Globals/AreaTriggerDataStore equivalent)
- `crates/wow-data/src/player_stats.rs` — player level info equivalent of `PlayerInfo`/`PlayerLevelInfo`
- `crates/wow-data/src/hotfix_cache.rs` — hotfix blob cache
- `crates/wow-data/src/wdc4.rs` — DB2 reader (not ObjectMgr territory but adjacent)
- `crates/wow-database/src/statements/{world.rs,character.rs,login.rs,hotfix.rs}` — prepared-statement registry + a handful of world-DB loaders
- `crates/wow-network/src/player_registry.rs` — `PlayerRegistry = DashMap<ObjectGuid, PlayerBroadcastInfo>` — partial `HashMapHolder<Player>` analogue
- `crates/wow-world/src/map_manager.rs` — `MapManager` 64×64 grids of `WorldCreature` (the `_creatureDataStore` + grid-cell index combined), 12 tests, ~890 lines, **not yet wired to the live tick path**
- `crates/wow-world/src/handlers/character.rs` — has hard-coded creature spawn-table reads; ObjectMgr loaders are absent

**What's implemented (ObjectMgr surface):**
- Item template lookup (partial)
- Quest template lookup (partial — `wow-data/quest.rs`)
- Player level info / class stats (`player_stats.rs`)
- Area trigger data (`area_trigger.rs`)
- Creature spawn storage (via `MapManager`, but only spawns — no `creature_template` loader)
- Player registry with GUID lookup (`PlayerRegistry`)

**What's implemented (ObjectAccessor surface):**
- `PlayerRegistry::get(guid)` ≈ `ObjectAccessor::FindPlayer`
- `MapManager::get_creature(map, instance, x, y, guid)` ≈ `ObjectAccessor::GetCreature`
- No equivalent of `GetWorldObject` / `GetGameObject` / `GetUnit` / `GetTransport` / `GetDynamicObject` / `GetAreaTrigger` / `GetCorpse` / `GetCreatureOrPetOrVehicle`
- No equivalent of `FindPlayerByName` / `FindPlayerByLowGUID` / `FindConnectedPlayer`
- No `SaveAllPlayers` global save loop

**What's missing vs C++ (high level):**
- ~115 of the ~120 `Load*` functions have no Rust analogue. Critical absentees: `LoadCreatureTemplate*` (server has no creature-template store at all — handlers fabricate stat data), `LoadGameObjectTemplate`, `LoadGameObjects` (no GO spawns), `LoadVendors`, `LoadTrainers`, `LoadGossipMenu*`, `LoadInstanceTemplate`, `LoadAreaTriggerTeleports`, `LoadAccessRequirements`, `LoadGraveyardZones` + `GetClosestGraveyard`, `LoadReputationOnKill`, `LoadNPCSpellClickSpells`, `LoadFactionChange*`, `LoadPhases`, `LoadSpawnGroups*`, `LoadCreatureClassLevelStats`, `LoadEquipmentTemplates`, every `Load*Locale` family (no i18n).
- ID generators (`GenerateAuctionID`, `GenerateMailID`, `GeneratePetNumber`, `GenerateCreatureSpawnId`, `GenerateGameObjectSpawnId`, `GetGenerator<HighGuid>`) are missing — without these, server-allocated GUIDs collide.
- `ScriptNameInternerLikeCpp` exists in `wow-data` as the C++ `ScriptNameContainer` bidirectional name/id core (`""` is id 0; duplicate inserts keep the first DB-bound flag), but the world-DB loaders still do not populate template/script rows from `creature_template.ScriptName`, `gameobject_template.ScriptName`, `spell_script_names`, or `areatrigger_scripts`.
- No `InitializeQueriesData` pre-build of cached query packets ⇒ each `CMSG_QUERY_*` would have to serialize on the fly.
- No `SetHighestGuids` ⇒ post-restart GUIDs may overlap with persisted entities.
- No global `Players` map for whole-server iteration (the `PlayerRegistry` is per-realm but does not iterate broadcasts in a thread-safe `RwLockReadGuard` like `HashMapHolder<Player>` does — it's a `DashMap`, which is fine but the access patterns differ).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `MapManager::get_creature` only resolves by `(map, instance, x, y, guid)` — TC's `ObjectAccessor::GetCreature(WorldObject const&, ObjectGuid)` is `O(1)` from the same map by walking a per-map registry. The Rust API forces the caller to know the cell, which is unusable for opcodes that arrive with only a GUID.
- `PlayerRegistry` does not enforce uniqueness on `(name, realm)` and has no name-folding/normalisation pass — `FindPlayerByName` will likely behave subtly differently.
- Hotfix blob cache and DB2 stores live in `wow-data` but no loader sequencing is documented; ObjectMgr loaders in TC depend on DB2 being ready first.
- `wow-data::quest` parses single quests but there is no `_questTemplates` master container nor `LoadQuests` validation pass (quest start/end relations, prerequisite chains, exclusive groups).
- Faction-change tables (alliance ↔ horde transmute) are entirely absent; any `.character changefaction`-type GM command will desync.

**Tests existing:**
- `crates/wow-world/src/map_manager.rs` — 12 unit tests for grid insert/lookup/move
- `crates/wow-network/src/player_registry.rs` — small unit tests for insert/remove
- `crates/wow-data/src/{quest,item,player_stats}.rs` — parser tests
- No round-trip tests vs C++ reference output for any loader
- No tests of GUID-generator monotonicity

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#GLOBALS.WBS.001** Cerrar la migracion auditada de `game/Globals/AreaTriggerDataStore.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.002** Cerrar la migracion auditada de `game/Globals/AreaTriggerDataStore.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.003** Cerrar la migracion auditada de `game/Globals/CharacterTemplateDataStore.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/CharacterTemplateDataStore.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.004** Cerrar la migracion auditada de `game/Globals/CharacterTemplateDataStore.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/CharacterTemplateDataStore.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.005** Cerrar la migracion auditada de `game/Globals/ConversationDataStore.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ConversationDataStore.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.006** Cerrar la migracion auditada de `game/Globals/ConversationDataStore.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ConversationDataStore.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.007** Cerrar la migracion auditada de `game/Globals/ObjectAccessor.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectAccessor.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.008** Cerrar la migracion auditada de `game/Globals/ObjectAccessor.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectAccessor.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.009** Partir y cerrar la migracion auditada de `game/Globals/ObjectMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 11444 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GLOBALS.WBS.010** Partir y cerrar la migracion auditada de `game/Globals/ObjectMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1944 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

### Phase A — establish a central `ObjectMgr` crate (decision)

- [ ] **#GLOB.1** Decide: introduce a new `wow-globals` crate (singleton `WorldData`) **or** centralise in `wow-data`. Pick one and migrate existing `wow-data` modules under it. (M)
- [ ] **#GLOB.2** Define `WorldData` struct: holds `Arc`s/`DashMap`s for every template store. Construct once at startup; pass `Arc<WorldData>` into sessions. (M)

### Phase B — ObjectAccessor parity (live-object lookup)

- [ ] **#GLOB.3** Add `ObjectAccessor` module in `wow-world` with traits `GetByGuid<T>`. Implement for `Player`, `Creature`, `GameObject`. (M)
- [ ] **#GLOB.4** Extend `MapManager::get_creature_by_guid(guid)` — global-by-guid lookup that walks maps (or maintain a `DashMap<ObjectGuid, (map_id, instance_id, x, y)>` index). (M)
- [ ] **#GLOB.5** Add `find_player_by_name(name)` and `find_player_by_low_guid(low)` to `PlayerRegistry`. (L)
- [ ] **#GLOB.6** Add `find_connected_player(guid)` (covers the teleport-in-progress case where player is not in any map). (L)
- [ ] **#GLOB.7** Implement `save_all_players()` periodic flush. (M)

### Phase C — ID generators

- [ ] **#GLOB.8** Implement `IdGenerators { auction, mail, pet_number, equipment_set, void_storage_item, creature_spawn, gameobject_spawn }` as `AtomicU64`s. (L)
- [ ] **#GLOB.9** Implement per-`HighGuid` `ObjectGuidGenerator`. (M)
- [ ] **#GLOB.10** Implement `set_highest_guids()` — query `MAX()` from each table at startup and seed atomics. (M)

### Phase D — template loaders (each ≈ one C++ `Load*`)

Each `LoadXxx` is a sub-task. Ordered by typical TC startup order (which is itself the dependency order). All M unless noted.

- [ ] **#GLOB.11** `load_trinity_strings` (i18n base). (L)
- [ ] **#GLOB.12** `load_script_names` + `ScriptNameContainer` (interner). Core interner exists in `wow-data` and is built at startup from currently loaded creature/gameobject template script names; `areatrigger_scripts` represented loading exists but waits for an authoritative AreaTrigger store before startup wiring; direct world-DB loader wiring for every C++ script-name source remains pending. (L)
- [ ] **#GLOB.13** `load_instance_template`. (L)
- [ ] **#GLOB.14** `load_creature_class_level_stats` (combat-stat baselines).
- [ ] **#GLOB.15** `load_creature_template` (canonical entry table). (H — wide schema, many validations)
- [ ] **#GLOB.16** `load_creature_template_addons` + `_models` + `_resistances` + `_spells` + `_difficulty` + `_gossip` + `_sparring` + `_summoned_data` (six addons, each L)
- [ ] **#GLOB.17** `load_creature_movement_overrides`.
- [ ] **#GLOB.18** `load_creature_model_info`.
- [ ] **#GLOB.19** `load_equipment_templates`.
- [ ] **#GLOB.20** `load_creature_addons` (per-spawn).
- [ ] **#GLOB.21** `load_creatures` (spawns) → integrate with `MapManager`. (H — links to grid index)
- [ ] **#GLOB.22** `load_linked_respawn`.
- [ ] **#GLOB.23** `load_temp_summons`.
- [ ] **#GLOB.24** `load_game_object_template` (+ `_addons`, `_overrides`). (H)
- [ ] **#GLOB.25** `load_game_object_addons` (per-spawn).
- [ ] **#GLOB.26** `load_game_objects` (spawns) → grid index. (H — needs `MapManager` GO support)
- [ ] **#GLOB.27** `load_spawn_group_templates` + `load_spawn_groups` + `load_instance_spawn_groups`.
- [ ] **#GLOB.28** `load_item_templates` + `_addon` + `_script_names`. (H — schema is wide)
- [ ] **#GLOB.29** `load_vehicle_template` + `_accessories` + `_seat_addon`.
- [ ] **#GLOB.30** `load_pet_level_info` + `load_pet_names` + `load_pet_number`.
- [ ] **#GLOB.31** `load_player_info` (per-race-class create info). (H — many sub-tables: action, item, skills, spell, level stats)
- [ ] **#GLOB.32** `load_quests` + `_starters_and_enders` + locales. (H)
- [ ] **#GLOB.33** `load_quest_poi` + `load_quest_area_triggers` + `load_quest_greetings`.
- [ ] **#GLOB.34** `load_quest_relations` (creature/GO ↔ quest multimap).
- [ ] **#GLOB.35** `load_scripts(ScriptsType)` (multi-table loader for event/spell/waypoint scripts). (H)
- [ ] **#GLOB.36** `load_event_scripts` + `load_spell_scripts` + `load_spell_script_names` + `validate_spell_scripts`.
- [ ] **#GLOB.37** `load_page_texts` + `_locales`.
- [ ] **#GLOB.38** `load_npc_text`.
- [ ] **#GLOB.39** `load_area_trigger_teleports` + `_tavern` + `_scripts` + `_involvedrelation`. The legacy teleport fallback still feeds movement today, but represented C++-faithful `_teleport`/`_tavern`/`_scripts`/quest-involved loaders exist in `wow-data/area_trigger.rs`; startup wiring waits for an authoritative `AreaTriggerStore` instead of the current teleport-only fallback.
- [ ] **#GLOB.40** `load_access_requirements`. `AccessRequirementStoreLikeCpp` is wired at startup with C++-selected columns, map/difficulty row skips, and item/quest/achievement reference clearing; remaining work is runtime consumption parity in all instance-entry paths.
- [ ] **#GLOB.41** `load_graveyard_zones` + `load_world_safe_locs` + `get_closest_graveyard()`. (H — runs `Conditions` at lookup time; partial: `wow-data::WorldSafeLocStore` and `GraveyardStore` are wired at startup, `graveyard_zone` duplicate/missing validation and ConditionMgr attachment exist, and `GetDefaultGraveyard`/`GetClosestGraveyardInZone` no-live-condition-object selection is represented. Remaining: terrain zone lookup, parent-area chain, live `conditionObject`/phase visibility, `MapEntry::CorpseMapID` source, and death/repop runtime consumption.)
- [~] **#GLOB.42** `load_exploration_base_xp`. `ExplorationBaseXpStoreLikeCpp` mirrors C++ `ObjectMgr::LoadExplorationBaseXP` / `GetBaseXP` from world SQL `exploration_basexp` (`level`, `basexp`), including duplicate-level overwrite and missing-level `0` semantics. Remaining: startup wiring and exploration XP runtime consumption.
- [~] **#GLOB.43** `load_fishing_base_skill_level` + `load_skill_tiers`. `FishingBaseSkillStoreLikeCpp` is wired for `skill_fishing_base_level`; `SkillTiersStoreLikeCpp` now mirrors C++ `ObjectMgr::LoadSkillTiers` from world SQL `skill_tiers` (`Value1..Value16`) and exposes `GetSkillTier`/`GetValueForTierIndex` semantics. Remaining: runtime skill/max-rank consumers.
- [~] **#GLOB.44** `load_mail_level_rewards`. `MailLevelRewardStoreLikeCpp` mirrors C++ `ObjectMgr::LoadMailLevelRewards` (`level`, `raceMask`, `mailTemplateId`, `senderEntry`) with `MAX_LEVEL`, playable-race, mail-template and sender-creature validation plus `GetMailLevelReward(level, race)` first-match semantics. Remaining: startup wiring and level-up mail delivery runtime consumption.
- [~] **#GLOB.45** `load_reputation_reward_rate` + `load_reputation_on_kill` + `load_reputation_spillover_template`. (`reputation_reward_rate` reader/validation represented in `#NEXT.R8.ENTITIES.663`; on-kill and spillover templates remain open)
- [~] **#GLOB.46** `load_points_of_interest` + `_locale`. `PointOfInterestStoreLikeCpp` and `PointOfInterestLocaleStoreLikeCpp` mirror C++ `LoadPointsOfInterest` / `LoadPointOfInterestLocales` (`points_of_interest`, `points_of_interest_locale`), including map-coordinate validation, duplicate-ID overwrite, invalid/enUS locale skip, and `GetPointOfInterest` / `GetPointOfInterestLocale` lookup semantics. Remaining: startup wiring and gossip/query packet consumption.
- [~] **#GLOB.47** `load_npc_spell_click_spells`. `NpcSpellClickStoreLikeCpp` mirrors C++ `LoadNPCSpellClickSpells` (`npc_spellclick_spells`) with missing creature/spell validation, invalid-user-type logged-but-loaded semantics, multimap-style bounds lookup, startup wiring, and the C++ post-load `UNIT_NPC_FLAG_SPELLCLICK` removal for templates without spellclick data. Remaining: full runtime coverage for every `Unit::HandleSpellClick`/vehicle/script path.
- [ ] **#GLOB.48** `load_game_object_for_quests`.
- [ ] **#GLOB.49** `load_reserved_players_names` + `load_profanity_names` + `is_reserved_name(...)` + `is_profanity_name(...)`. (L each)
- [ ] **#GLOB.50** `load_game_tele`.
- [ ] **#GLOB.51** `load_gossip_menu` + `load_gossip_menu_items` + `_locales` + `_addon`.
- [ ] **#GLOB.52** `load_vendors` + `add_vendor_item` (online editor support) + persistence.
- [ ] **#GLOB.53** `load_trainers` + `load_creature_trainers`.
- [ ] **#GLOB.54** `load_phases` + `unload_phase_conditions` + `load_terrain_swap_defaults` + `load_terrain_world_maps` + `load_area_phases` + `load_phase_names`.
- [ ] **#GLOB.55** `load_faction_change_*` (achievements, items, quests, reputations, spells, titles). (M total)
- [ ] **#GLOB.56** `load_scene_templates`.
- [ ] **#GLOB.57** `load_player_choices` + `_locale`. (M)
- [ ] **#GLOB.58** `load_jump_charge_params`.
- [ ] **#GLOB.59** `load_creature_quest_items` + `load_game_object_quest_items`.

### Phase E — DataStore companions

- [ ] **#GLOB.60** Port `AreaTriggerDataStore` (template + create_properties + actions/spline/polygon). (H — already partly in `wow-data/area_trigger.rs`)
- [ ] **#GLOB.61** Port `CharacterTemplateDataStore` (chargen presets). (M)
- [ ] **#GLOB.62** Port `ConversationDataStore` (NPC dialog scripting). (H — ties into spell visuals & line timing)

### Phase F — query data caches

- [ ] **#GLOB.63** `initialize_queries_data(mask)` — pre-serialise `QUERY_*_RESPONSE` packets into per-template byte buffers. (H — touches every template type)

### Phase G — utilities

- [ ] **#GLOB.64** `normalize_player_name` (UTF-8 first-letter capitalisation, locale-aware). (M)
- [ ] **#GLOB.65** `extract_extended_player_name` (`Name@Realm`). (L)
- [ ] **#GLOB.66** `return_or_delete_old_mails(server_up: bool)` cron task. (M)
- [ ] **#GLOB.67** `parse_spawn_difficulties` helper. (L)

### Phase H — wiring

- [ ] **#GLOB.68** Boot sequence: invoke loaders in TC order from `world-server` startup. (H — ordering constraints across ~50 loaders)
- [ ] **#GLOB.69** Hot-reload subset: `.reload creature_template`, `.reload gameobject_template`, etc. (M per group)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#GLOBALS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 10 files / 14883 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-network` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#GLOBALS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 10 files / 14883 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#GLOBALS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 10 files / 14883 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#GLOBALS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 10 files / 14883 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: every `Load*` migrated produces the same `len()` as the C# reference (parity check on `wowchad` DB)
- [ ] Test: `find_player(guid)` on a known online char returns the same `PlayerBroadcastInfo` from any map
- [ ] Test: `get_closest_graveyard(loc, team, conditionObj)` returns the same row as TC for a known location/team
- [ ] Test: `generate_creature_spawn_id` after restart never collides with any existing `creature.guid`
- [ ] Test: `generate_auction_id` is monotonic and persists across restarts (uses `MAX(id) + 1`)
- [ ] Test: `is_reserved_name("Arthas")` returns true (or whatever the wowchad seed says)
- [ ] Test: `get_quest_template(101)` returns `Wolf Across the Border` quest, with the correct objectives count
- [ ] Test: `get_creature_template(NPC_ARTHAS)` exposes the right faction template, scale, classification
- [ ] Test: `get_vendor_items(npc)` returns the same item list as the world DB
- [ ] Test: `get_trainer(npc)` returns the same spell list
- [ ] Test: round-trip: load → serialize a `SMSG_QUERY_CREATURE_RESPONSE` matches a TC capture
- [ ] Test: race+class start info — `get_player_info(RACE_HUMAN, CLASS_WARRIOR)` matches TC start position, items and spells
- [ ] Test: name normalisation idempotency — `normalize("ARTHAS")` == `normalize("arthas")` == `"Arthas"`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 10 files / 14883 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp` | `wow-data` (templates), `wow-database` (loaders), `wow-world` (live registries / `MapManager`), `wow-network` (`PlayerRegistry`) \| ⚠️ partial — fragmented across 5+ crates; no central `ObjectMgr` analogue |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#GLOBALS.DIV.001` | `crates/wow-database/src/statements/{world.rs,character.rs,login.rs,hotfix.rs}` (`declared_pattern`, 0 Rust lines) | 10 C++ files / 14883 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |
| `#GLOBALS.DIV.002` | `crates/wow-data/src/{quest,item,player_stats}.rs` (`declared_pattern`, 0 Rust lines) | 10 C++ files / 14883 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |

<!-- REFINE.023:END known-divergences -->

- **`ObjectMgr.cpp` is 11 444 lines.** Do not attempt a big-bang port. Migrate one loader at a time, each behind a feature flag, with the C# reference output kept available for diff. Each loader has its own validation rules; stripping them silently breaks downstream code.
- **Loader ordering matters.** Many loaders read from previous loaders' stores (e.g. `LoadCreatures` references `_creatureTemplateStore`, `LoadVendors` references `_creatureTemplateStore` AND `_itemTemplateStore`). The canonical order is in `World::SetInitialWorldSettings()` in TC (`src/server/game/World/World.cpp`). Replicate that order exactly.
- **DB2 stores must be loaded before ObjectMgr starts.** Map.db2, ChrRaces.db2, ChrClasses.db2, Faction.db2, FactionTemplate.db2, etc. ObjectMgr loaders cross-reference them and will warn-and-skip rows otherwise.
- **`HashMapHolder<Player>` uses `std::shared_mutex`.** TC reads concurrently, writes under exclusive lock. The `PlayerRegistry = DashMap` in Rust is finer-grained but iteration semantics differ — `SaveAllPlayers` in TC takes a global read lock; with DashMap you'd need `iter()` and accept inconsistent snapshots, or replace with `RwLock<HashMap>` if you want the same iteration guarantees.
- **`ObjectAccessor::FindPlayer*` is comment-flagged as "NOT THREAD SAFE".** TC accepts that for cross-map admin commands. In Rust we should make these explicit `try_*` returning `Option<Arc<...>>` to make the lifetime story sound.
- **GUID generators are global atomics initialised from `MAX(guid)`.** Forgetting `set_highest_guids()` ⇒ new spawns reuse persisted GUIDs ⇒ silent data loss in `creature` table on next save.
- **`ScriptNameContainer` is bidirectional.** `script_name → id` and `id → script_name`. The DB stores `ScriptName VARCHAR(64)` for many tables; ObjectMgr resolves them to ints once and downstream code passes ints. If the interner is missing, every script-bound entity (creature_template.AIName, gameobject_template.AIName, areatrigger_scripts.ScriptName, spell_script_names.ScriptName, etc.) silently fails to bind.
- **`InitializeQueriesData(QueryDataGroup mask)`** pre-builds every `SMSG_QUERY_*_RESPONSE` packet in memory at startup. Without it, every quest/creature/item query response is built on the fly per session — measurably slower in busy hubs (Dalaran, Stormwind).
- **Faction-change tables.** TC supports "I want to flip my Alliance Paladin to a Horde Death Knight" — six tables map old → new spell/quest/item/title/achievement/reputation. Easy to forget; if absent, `.character changefaction` GM command produces silently broken characters.
- **`AddVendorItem(persist=true)`** writes back to `npc_vendor`. Online-edited vendors must round-trip. Skipping persistence ⇒ GM edits are lost on restart.
- **`LoadGameObjectForQuests`** is a derived index (`_gameObjectForQuestStore`) computed from quest objectives — it must be rebuilt whenever quests are reloaded, not just on the first GO load.
- **`LoadPhases` builds a `ConditionContainer` per phase area.** The Rust port must thread `wow-conditions` through `wow-data::phase`, otherwise phasing rules collapse to "always visible".
- **Wrath 3.4.3 specifics.** TC's master branch (which this fork tracks) is multi-expansion; some tables (`character_template`, `playerchoice`, `scene_template`, `terrain_swap_*`, `phase_*`) are hard-empty in 3.4.3 wowchad seeds. Don't panic if a loader logs `"loaded 0 rows"` — that's expected for those expansion-locked tables. Audit by running the loader against the live wowchad world DB to confirm.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ObjectMgr` (singleton) | `struct WorldData` (single instance behind `Arc<WorldData>`) in a new `wow-globals` crate | No singleton; pass `Arc` through `WorldSession` and tick contexts |
| `ObjectMgr::instance()` | `WorldData::new(db: &Database).await -> Arc<Self>` | Async because every loader hits SQL |
| `void Foo::LoadXxx()` | `async fn load_xxx(db: &Database, world: &mut WorldData) -> Result<(), LoadError>` | Each is async + bubbles errors instead of asserting |
| `std::unordered_map<Key, T>` (read-mostly) | `HashMap<Key, T>` inside `WorldData` (immutable after load) | Templates loaded once, never mutated at runtime → plain `HashMap`, no lock |
| `std::unordered_map<Key, T>` (mutable at runtime, e.g. spawns, vendors) | `DashMap<Key, T>` | Concurrent without coarse locking |
| `template<class T> HashMapHolder<T>` | per-type `DashMap<ObjectGuid, Arc<RwLock<T>>>` (or your preferred sync wrapper) in `wow-world` | One per object type |
| `namespace ObjectAccessor` | trait `ObjectAccess` with methods `find_player`, `find_creature`, etc. | Implemented for `(Arc<MapManager>, Arc<PlayerRegistry>)` tuple |
| `ObjectAccessor::FindPlayer(guid)` (whole-server) | `PlayerRegistry::get(&guid)` | Already exists |
| `ObjectAccessor::GetCreature(WorldObject const&, guid)` (same map) | `MapManager::find_creature_in_map(map_id, instance_id, guid)` | New method needed: walks all grids of the given map looking for guid |
| `ObjectAccessor::GetWorldObject(...)` | `enum WorldObjectRef { Player(...), Creature(...), GameObject(...), ... }` returning enum | No virtual base class in Rust; use enum |
| `ObjectAccessor::SaveAllPlayers()` | `async fn save_all_players(db: &Database, registry: &PlayerRegistry)` | Use `JoinSet` to parallelise per-player saves |
| `uint32 ObjectMgr::GenerateAuctionID()` | `IdGenerators { auction: AtomicU64 }` + `fn generate_auction_id(&self) -> u64` | Atomic fetch_add |
| `ObjectGuidGenerator<HighGuid>` | `struct GuidGenerator { high: HighGuid, next: AtomicU64 }` | Per-HighGuid map of generators |
| `ScriptNameContainer` | `struct ScriptInterner { name_to_id: HashMap<String, ScriptId>, id_to_name: Vec<String> }` | Built once at load, then immutable |
| `Trinity::IteratorPair<...>` | `impl Iterator<Item = ...>` | Use rust iterators |
| `Optional<T>` | `Option<T>` | — |
| `std::set<uint32> EventContainer` | `HashSet<u32>` | — |
| `Trinity::normalizePlayerName(string&)` | `fn normalize_player_name(&str) -> Option<String>` | UTF-8 aware |
| `BattlenetRpcErrorCode` enum | (already in `wow-proto` — see `proto.md`) | Cross-cutting |
| `friend class PlayerDumpReader` | `pub(crate)` visibility on the struct fields PlayerDumpReader needs | Rust has no `friend` — use module-private fields |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

Verified the section 8 hipótesis against live source.

**ObjectMgr `Load*` count vs Rust loaders.**
- C++ has ~107 `Load*` methods (count from `ObjectMgr.cpp` declarations) plus ~10 more in `AreaTriggerDataStore`/`CharacterTemplateDataStore`/`ConversationDataStore` — section 2's "~120" is correct.
- Rust loaders (greppable with `fn load`): `wow-data/src/spell.rs:99 load`, `quest_xp.rs:33 load`, `item_stats.rs:242 load`, `player_stats.rs:194 load`, `area_trigger.rs:254 load_area_triggers`, `hotfix_cache.rs:40 load_db2`, `skill.rs:81 load`, `quest.rs:198 load_quests`, `item.rs:43 load`. That is **9** distinct top-level loaders covering quest/item/spell/skill/area-trigger/player-stats/hotfix domains. Plus `wow-world/src/handlers/quest.rs:776 load_player_quests` (per-session, not template-load). So **~107–9 ≈ ~98 loaders unported** plus most Locale/Addon variants — section 8's "~115 unported" estimate is in the right ballpark; tighten the doc figure to **~98 of ~107 ObjectMgr `Load*` absent** (still ⚠️ partial; closer to ❌).
- `wow-database/src/statements/world.rs` is a **prepared-statement registry** (86 enum variants — `grep -cE '^\s*[A-Z][A-Z0-9_]*,' = 86`) **not a loader**. Doc already says this; just confirming the registry has slots like `SEL_CREATURE_TEMPLATE`, `SEL_QUEST_TEMPLATE`, `SEL_VENDOR_ITEMS` reserved but no consumer in `wow-data` or `wow-world` outside of the few loaders above.

**Critical: `ObjectAccessor::GetCreature(WorldObject const&, ObjectGuid)` signature.**
- `crates/wow-world/src/map_manager.rs:339` exposes a flat `MapManager::get_creature(guid) -> Option<&WorldCreature>` (search-all) **and** `MapManager::get_creature(map_id, instance_id, x, y, guid)` at `:510`. The flat one effectively is the GUID-only lookup the doc claimed was missing — it walks all maps/grids internally (see also `:343 get_creature_mut`, `:422` per-grid variant).
- **Verdict on the doc claim**: the section 8 "the Rust API forces the caller to know the cell" line is **stale / wrong** — the GUID-only variant exists at `map_manager.rs:339`. The TC equivalent of `GetCreature(WorldObject const&, ObjectGuid)` (same-map scoped) is what's actually missing: there's no method that takes a reference object's `(map_id, instance_id)` and looks up only within that map. Update the doc to: GUID-only walk-all-maps exists; same-map scoped lookup does not.

**Verdict:** ⚠️ partial confirmed. ObjectMgr ≈ 8% ported (9/107 loaders); ObjectAccessor lookup partially exists but lacks same-map scoping, name lookup, low-GUID lookup, save-all. Section 8 "Suspicious" bullet about `get_creature` should be revised — the GUID-only variant is present.
