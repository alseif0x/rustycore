# Migration: Game DataStores (DB2Manager singleton + auto-generated DB2 records + GameTables)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/DataStores/`
> **Rust target crate(s):** `crates/wow-data/` (also touches `crates/wow-database/` for hotfix statements)
> **Layer:** L1 (infrastructure — typed game-data tables consumed by every gameplay system)
> **Status:** ⚠️ partial — confirmed via audit 2026-05-01 (raw WDC4 reader OK; **7 hand-rolled tables of ~261**, not 8; no DB2Manager, no hotfix overlay, no GameTables, no M2 cameras)
> **Audited vs C++:** ✅ complete (16 files, ~30 KLoC) — game-side gap reverified 2026-05-01 against `shared-datastores.md` audit (which counted 5 .db2 files at the shared-loader layer; this game-side layer wraps slightly more — they are not the same metric)
> **Last updated:** 2026-05-01

---

## 1. Purpose

This is the **gameplay-facing database of static client data**. The shared layer (`shared/DataStores`, doc: `shared-datastores.md`) provides the binary parser and the templated `DB2Storage<T>` container. The game layer assembles those into:

1. **~261 typed global stores** — one per DB2 table — each declared `extern DB2Storage<XEntry> sXStore` and loaded at world startup. Tables: `sAchievementStore`, `sAreaTableStore`, `sCreatureFamilyStore`, `sFactionStore`, `sItemStore`, `sItemSparseStore`, `sLFGDungeonsStore`, `sMapStore`, `sQuestPackageItemStore`, `sSkillLineStore`, `sSpellStore`, `sTalentStore`, `sTaxiNodesStore`, `sUiMapStore`, etc.
2. **Auto-generated record structs** (`DB2Structure.h`, ~325 structs, 4538 lines) — the in-memory C++ representation of each DB2 row (e.g. `MapEntry { LocalizedString MapName; uint32 ID; uint8 InstanceType; uint16 AreaTableID; … }`).
3. **Auto-generated meta** (`DB2Metadata.h`, ~788 structs, 12067 lines) — the per-table `DB2Meta::Instance` constants used by the binary loader and the hotfix-DB overlay to know each field's type, array size, signedness, and the table's `LayoutHash`.
4. **Auto-generated hotfix-DB load info** (`DB2LoadInfo.h`, ~325 structs, 6357 lines) — binds each table to its `HotfixDatabaseStatements` enum member (the prepared statement that fetches overlay rows).
5. **`DB2Manager`** — the runtime singleton. Owns the load-order, the hotfix data tracking (`hotfix_data`/`hotfix_blob`/`hotfix_optional_data`), client-reply caches, and ~80 derived/indexed accessors (`GetMount`, `GetSkillRaceClassInfo`, `GetTalentsByPosition`, `Map2ZoneCoordinates`, …) on top of the raw stores.
6. **`DBCEnums.h`** — gameplay enums backed by the DB2 numeric values (`Difficulty`, `BattlegroundBracketId`, `ItemQualities`, `Powers`, `MapType`, `LFGDungeonFlags`, …). 2514 lines.
7. **`GameTables`** — sixteen tab-separated **client text files** (not DB2!): per-level scaling curves like `gtBaseMP.txt`, `gtCombatRatings.txt`, `gtSpellScaling.txt`. Loaded by `GameTables.cpp` into `GameTable<GtXEntry>` arrays indexed by level.
8. **`M2Stores`** — `LoadM2Cameras`: parses M2 model files for cinematic camera fly-by data (cinematics opcodes need keyframe positions).
9. **`DB2HotfixGenerator`** — runtime in-memory record patcher (e.g. `db2_hotfixes.cpp` fixes known-broken records on world startup and emits `SMSG_HOTFIX_PUSH` to clients).

The game-side `DataStores` is therefore the *catalog of every numeric/textual constant the WoW client and server share*. Almost every gameplay handler reads from it.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/DataStores/DB2HotfixGenerator.cpp` | 30 | `prefix` |
| `game/DataStores/DB2HotfixGenerator.h` | 70 | `prefix` |
| `game/DataStores/DB2LoadInfo.h` | 6357 | `prefix` |
| `game/DataStores/DB2Metadata.h` | 12067 | `prefix` |
| `game/DataStores/DB2Stores.cpp` | 3104 | `prefix` |
| `game/DataStores/DB2Stores.h` | 516 | `prefix` |
| `game/DataStores/DB2Structure.h` | 4538 | `prefix` |
| `game/DataStores/DBCEnums.h` | 2514 | `prefix` |
| `game/DataStores/GameTables.cpp` | 148 | `prefix` |
| `game/DataStores/GameTables.h` | 415 | `prefix` |
| `game/DataStores/M2Stores.cpp` | 270 | `prefix` |
| `game/DataStores/M2Stores.h` | 35 | `prefix` |
| `game/DataStores/M2Structure.h` | 136 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/DataStores/DB2Stores.h` | 516 | Externs for ~261 `DB2Storage<…Entry>` globals; `DB2Manager` class declaration; helper structs (`HotfixId`, `HotfixRecord`, `ContentTuningLevels`, `ShapeshiftFormModelData`, `TaxiPathBySourceAndDestination`); `TaxiMask` typedef |
| `src/server/game/DataStores/DB2Stores.cpp` | 3104 | All `DB2Storage` definitions; `DB2Manager::LoadStores` (~750 LOAD_DB2 lines); `LoadHotfixData`/`LoadHotfixBlob`/`LoadHotfixOptionalData`; ~80 accessor implementations; secondary indexes (e.g. `_mountsBySpellId`, `_skillLineAbilitiesBySkill`) |
| `src/server/game/DataStores/DB2Structure.h` | 4538 | The auto-generated 325 record structs |
| `src/server/game/DataStores/DB2Metadata.h` | 12067 | The auto-generated 788 meta structs (`XMeta::Instance`) — yes, more meta structs than tables, because some tables have multiple meta variants for different localization layouts |
| `src/server/game/DataStores/DB2LoadInfo.h` | 6357 | The auto-generated 325 `XLoadInfo::Instance` structs |
| `src/server/game/DataStores/DBCEnums.h` | 2514 | All gameplay-side enums: `Difficulty`, `Powers`, `BattlegroundBracketId`, `ItemQualities`, `MapType`, plus `DBCPosition2D`/`DBCPosition3D` POD structs |
| `src/server/game/DataStores/DB2HotfixGenerator.h` | 70 | `DB2HotfixGenerator<T>` template + base class |
| `src/server/game/DataStores/DB2HotfixGenerator.cpp` | 30 | `LogMissingRecord`, `AddClientHotfix` (registers a runtime patch with the client push list) |
| `src/server/game/DataStores/GameTables.h` | 415 | Sixteen `GtXEntry` POD structs + the templated `GameTable<T>` class + `GetRegenGameTableColumnForClass` / `GetSpellScalingColumnForClass` / `GetShieldBlockRegularColumnForQuality` selector functions |
| `src/server/game/DataStores/GameTables.cpp` | 148 | The 16 `sXGameTable` globals, generic `LoadGameTable` (TSV parser), `LoadGameTables` orchestrator |
| `src/server/game/DataStores/M2Stores.h` | 35 | `FlyByCamera { uint32 timeStamp; Position locations; }`; `LoadM2Cameras`; `GetFlyByCameras` |
| `src/server/game/DataStores/M2Stores.cpp` | 270 | M2 file parser (header + camera chunks); per-cinematic-id keyframe vector |
| `src/server/game/DataStores/M2Structure.h` | 136 | M2 binary layout (`M2Header`, `M2Track`, `M2Camera`, animation interpolation tags) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `DB2Manager` | class (Meyers singleton) | Orchestrates DB2 load + hotfix overlay + ~80 accessor helpers |
| `DB2Manager::HotfixId` | struct | `{int32 PushID, uint32 UniqueID}` — composite ordered key for hotfixes |
| `DB2Manager::HotfixRecord` | struct | One row from `hotfix_data`: `TableHash`, `RecordID`, `HotfixId`, `Status`, `AvailableLocalesMask` |
| `DB2Manager::HotfixRecord::Status` | enum class : uint8 | `NotSet=0`, `Valid=1`, `RecordRemoved=2`, `Invalid=3`, `NotPublic=4` |
| `DB2Manager::HotfixOptionalData` | struct | `{Key, Vec<uint8> Data}` for ancillary blobs (BroadcastText TACT keys) |
| `DB2Manager::HotfixPush` | struct | `{Vec<HotfixRecord> Records, AvailableLocalesMask}` — one push group sent to a client |
| `DB2Manager::HotfixContainer` | typedef | `std::map<int32 PushId, HotfixPush>` — the master container |
| `DB2Manager::FriendshipRepReactionSet` | typedef | `std::set<… *, FriendshipRepReactionEntryComparator>` |
| `DB2Manager::MapDifficultyConditionsContainer` | typedef | `std::vector<std::pair<uint32, PlayerConditionEntry const*>>` |
| `DB2Manager::MountTypeXCapabilitySet` | typedef | Comparator-ordered set used by mount lookups |
| `DB2Manager::MountXDisplayContainer` | typedef | `std::vector<MountXDisplayEntry const*>` |
| `XEntry` (×325) | POD struct | One per DB2 table — see `DB2Structure.h`. Examples: `MapEntry`, `ItemEntry`, `ItemSparseEntry`, `SpellEntry` (no, this one is `SpellNameEntry` in WoLK 3.4.3 — Spell.db2 was split in MoP+), `ChrClassesEntry`, `ChrRacesEntry`, `FactionEntry`, `FactionTemplateEntry`, `LFGDungeonsEntry`, `BattlemasterListEntry`, `AchievementEntry`, `CriteriaEntry`, `CriteriaTreeEntry`, `QuestPackageItemEntry`, `TaxiNodesEntry`, `TaxiPathEntry`, `TaxiPathNodeEntry`, `BroadcastTextEntry`, `UiMapEntry`, `UiMapAssignmentEntry`, `WorldMapOverlayEntry`, `LightEntry`, `LiquidTypeEntry`, `MailTemplateEntry`, … |
| `XMeta::Instance` (×788) | constexpr | Per-table layout descriptor — see `DB2Metadata.h` |
| `XLoadInfo::Instance` (×325) | constexpr | Per-table field-name + hotfix-statement bind — see `DB2LoadInfo.h` |
| `GtXEntry` (×16) | POD | Each is a flat `float` array — `GtCombatRatingsEntry { float WeaponSkill, DefenseSkill, … }`. Layout is asserted to match TSV column count via `static_assert`-style runtime check in `LoadGameTable` |
| `GameTable<T>` | template class | Wraps `std::vector<T>` indexed by level; `GetRow(level)`, `GetTableRowCount()` |
| `FlyByCamera` | struct | `{uint32 timeStamp, Position locations}` — one keyframe |
| `M2Header` / `M2Track` / `M2Camera` (in `M2Structure.h`) | POD | M2 binary layout |
| `DBCPosition2D` / `DBCPosition3D` | POD (#pragma pack(1)) | Embedded position fields in many DB2 records |
| `DBCFormer` | enum | Field-type tags (covered in shared doc) |
| `Difficulty` | enum (`DBCEnums.h`) | DB2-backed difficulty IDs — `DIFFICULTY_NORMAL=1`, `DIFFICULTY_HEROIC=2`, `DIFFICULTY_10_N=3`, `DIFFICULTY_25_N=4`, etc. |
| `BattlegroundBracketId` | enum | `BG_BRACKET_ID_FIRST..LAST` |
| `MapType` | enum | `MAP_COMMON`, `MAP_INSTANCE`, `MAP_RAID`, `MAP_BATTLEGROUND`, `MAP_ARENA`, `MAP_SCENARIO` |
| `Powers` | enum | `POWER_HEALTH=-2`, `POWER_MANA=0`, …, `POWER_RUNES=5`, … (used as DB2 lookup key) |
| `ItemQualities` | enum | `ITEM_QUALITY_POOR=0`…`ITEM_QUALITY_HEIRLOOM=7` |
| `LevelLimit` | enum | `DEFAULT_MAX_LEVEL=80`, `MAX_LEVEL=123`, `STRONG_MAX_LEVEL=255` (the Rust port should pin to 80 for WoLK Classic) |
| `DEFINE_DB2_SET_COMPARATOR(s)` | macro | Generates `sComparator` struct with `bool operator()(s const* l, s const* r) const` and `static bool Compare(…)` for `std::set<s const*, sComparator>` use |
| `DB2HotfixGeneratorBase` / `DB2HotfixGenerator<T>` | class | Runtime in-memory record patcher (see shared doc) |

---

## 4. Critical public methods / functions

(Solo los públicos de `DB2Manager` y la maquinaria de carga; demasiado para listarlos todos — top items.)

| Symbol | Purpose | Calls into |
|---|---|---|
| `DB2Manager::Instance()` | Meyers singleton accessor (`#define sDB2Manager DB2Manager::Instance()`) | — |
| `DB2Manager::LoadStores(dataPath, defaultLocale) -> uint32` | Walks `dataPath/dbc/<locale>/` for available locales; for each of ~261 stores, `LoadDB2(availableLocales, errors, _stores, &sStore, db2Path, defaultLocale, GetCppRecordSize)`. `_stores` is a `std::unordered_map<uint32 tableHash, DB2StorageBase*>` populated as a side effect | `DB2StorageBase::Load`, `LoadStringsFrom` |
| `DB2Manager::GetStorage(tableHash)` | Lookup a store by table hash (used when a hotfix references a table generically) | `_stores` map |
| `DB2Manager::LoadHotfixData(localeMask)` | `SELECT Id, UniqueId, TableHash, RecordId, Status FROM hotfix_data ORDER BY Id`. For each row: validate locale availability (cross-check with `_hotfixBlob` for tables not loaded server-side), assemble a `HotfixRecord`, append to `_hotfixData[id].Records`, and assign the current removal decision into `deletedRecords[{tableHash, recordId}]`. After the loop, call `store->EraseRecord(recordId)` only when that key's final assigned status is `RecordRemoved`; a later non-removed push cancels an earlier removal | `HotfixDatabase`, `EraseRecord` |
| `DB2Manager::LoadHotfixBlob(localeMask)` | `SELECT TableHash, RecordId, locale, Blob FROM hotfix_blob ORDER BY TableHash`. Stores raw bytes in `_hotfixBlob[locale][{tableHash,recordId}]`. Used for tables the server doesn't load (locale-specific UI strings, debug-only tables) but still needs to push to clients | `HotfixDatabase` |
| `DB2Manager::LoadHotfixOptionalData(localeMask)` | `SELECT TableHash, RecordId, locale, Key, Data FROM hotfix_optional_data ORDER BY TableHash`. Validates against `_allowedHotfixOptionalData` registry (e.g. only `BroadcastText` may have a `TactKey`). Pushes to `_hotfixOptionalData[locale][{tableHash,recordId}]` | `HotfixDatabase` |
| `DB2Manager::GetHotfixCount()` / `GetHotfixData()` | Used by `WorldSession` to assemble `SMSG_AVAILABLE_HOTFIXES` and `SMSG_HOTFIX_PUSH` | — |
| `DB2Manager::GetHotfixBlobData(tableHash, recordId, locale)` | Reverse-lookup; client requests an unloaded table's record | — |
| `DB2Manager::GetAreasForGroup(areaGroupId)` | DFS over `AreaGroupMember` to collect every leaf area | `sAreaGroupMemberStore` |
| `DB2Manager::IsInArea(objectAreaId, areaId)` | Walk parent chain via `AreaTableEntry::ParentAreaID` | `sAreaTableStore` |
| `DB2Manager::GetBroadcastTextValue(entry, locale, gender, forceGender)` | Per-gender + per-locale broadcast text resolver | `LocalizedString` |
| `DB2Manager::GetClassName(class, locale)` / `GetChrRaceName(race, locale)` | `LocalizedString` lookups via `sChrClassesStore`/`sChrRacesStore` | — |
| `DB2Manager::GetPowerIndexByClass(power, classId)` | `sChrClassesXPowerTypesStore` filter — returns the per-class power slot |
| `DB2Manager::GetCustomiztionChoices(optionId)` (sic — typo perpetuated) | Customization options for character creation/barbershop | `sChrCustomizationOptionStore` |
| `DB2Manager::GetChrModel(race, gender)` / `GetChrSpecializationByIndex(class, index)` / `GetDefaultChrSpecializationForClass(class)` | Char-creation helpers | `sChrModelStore`, `sChrSpecializationStore` |
| `DB2Manager::GetContentTuningData(id, forItem)` | Resolves min/max/target levels for a `ContentTuning` row; powers item-level scaling and quest-level scaling |
| `DB2Manager::GetCurveValueAt(curveId, x)` / `GetCurveXAxisRange(curveId)` | Curve interpolation (linear/cosine/cubic-spline) used everywhere — XP, item scaling, etc. | `sCurveStore`, `sCurvePointStore` |
| `DB2Manager::EvaluateExpectedStat(stat, level, expansion, contentTuningId, class)` | Computes the expected primary stat for a level — used by Item.db2 → ItemSparse delta |
| `DB2Manager::GetFactionTeamList(faction)` / `GetFriendshipRepReactions(id)` / `GetParagonReputation(factionId)` | Reputation system feed | `sFactionStore`, etc. |
| `DB2Manager::GetGlyphBindableSpells(glyphPropertiesId)` / `GetGlyphRequiredSpecs(glyphPropertiesId)` | Glyph validation | `sGlyphBindableSpellStore` |
| `DB2Manager::GetHeirloomByItemId(itemId)` | Heirloom lookup | `sHeirloomStore` |
| `DB2Manager::GetItemDisplayId(itemId, appearanceModId)` | Wardrobe; transmog | `sItemModifiedAppearanceStore` |
| `DB2Manager::GetItemSetSpells(itemSetId)` / `GetItemSpecOverrides(itemId)` | Set bonuses; class-spec drops | `sItemSetSpellStore` |
| `DB2Manager::GetLfgDungeon(mapId, difficulty)` | LFG queue → dungeon resolution | `sLFGDungeonsStore` |
| `DB2Manager::GetDefaultMapDifficulty(mapId, *outDifficulty)` / `GetMapDifficultyData(mapId, difficulty)` / `GetDownscaledMapDifficultyData(mapId, &difficulty)` | Map → difficulty resolution | `sMapDifficultyStore` |
| `DB2Manager::GetMount(spellId)` / `GetMountById(id)` / `GetMountCapabilities(mountType)` / `GetMountDisplays(mountId)` | Mount system | `sMountStore`, `sMountCapabilityStore`, `sMountTypeXCapabilityStore`, and `sMountXDisplayStore` are loaded with hotfix overlays; capability/display indexes match C++ startup grouping |
| `DB2Manager::GetNameGenEntry(race, gender)` / `ValidateName(wname, locale)` | Random-name + name-validation | `sNameGenStore`, `sNamesProfanityStore`, `sNamesReservedStore` |
| `DB2Manager::GetNumTalentsAtLevel(level, class)` | Talent slots per level | builtin table |
| `DB2Manager::GetPhasesForGroup(group)` | Phasing | `sPhaseXPhaseGroupStore` |
| `DB2Manager::GetPowerTypeEntry(power)` / `GetPowerTypeByName(name)` | Power type lookup | `sPowerTypeStore` |
| `DB2Manager::GetBattlegroundBracketByLevel(mapId, level)` / `GetBattlegroundBracketById(mapId, bracketId)` | BG bracket math | `sPVPDifficultyStore` |
| `DB2Manager::GetQuestsForQuestLine(questLineId)` / `GetQuestPackageItems(questPackageID)` | Quest lookups | `sQuestLineXQuestStore` |
| `DB2Manager::GetSkillLinesForParentSkill(parentSkillId)` / `GetSkillLineAbilitiesBySkill(skillId)` / `GetSkillRaceClassInfo(skill, race, class)` | Skill system | `sSkillLineStore`, `sSkillLineAbilityStore`, `sSkillRaceClassInfoStore` |
| `DB2Manager::GetTalentsByPosition(class, tier, column)` | Talent grid | `sTalentStore` |
| `DB2Manager::GetTaxiPath(from, to)` | Flight master pathfinding | `sTaxiPathStore` |
| `DB2Manager::IsToyItem(toy)` | Toy box | `sToyStore` |
| `DB2Manager::GetUiMapPosition(...)` | World coordinates → minimap coordinates | `sUiMapStore`, `sUiMapAssignmentStore` |
| `DB2Manager::Zone2MapCoordinates(areaId, &x, &y)` / `Map2ZoneCoordinates(...)` | Inverse coordinate transforms | `sWorldMapOverlayStore` |
| `DB2Manager::GetWMOAreaTable(rootId, adtId, groupId)` | WMO → area resolution | `sWMOAreaTableStore` |
| `DB2Manager::GetItemEffectsForItemId(itemId)` | Item proc spells | `sItemEffectStore` |
| `DB2Manager::GetScalingStatValuesForLevel(level)` | Heirloom scaling | builtin table |
| `LoadGameTables(dataPath)` (free function in `GameTables.cpp`) | Loads the 16 TSV files in `dataPath/gt/` into the `sXGameTable` globals; aborts on size mismatch | `LoadGameTable<T>` |
| `LoadM2Cameras(dataPath)` / `GetFlyByCameras(cinematicCameraId)` | M2 file walker for cinematics | M2 binary parsing |

---

## 5. Module dependencies

**Depends on:**
- `shared/DataStores/DB2Storage<T>` — the typed container (see `shared-datastores.md`).
- `shared/DataStores/DB2DatabaseLoader` — the hotfix-DB overlay engine.
- `Database/HotfixDatabase` + `HotfixDatabaseStatements` enum — issued ~3 statements per table × 261 tables ≈ 783 statements, plus 3 control-table queries.
- `Common/SharedDefines` — `LocaleConstant`, `Classes`, `Powers`, `Difficulty`.
- `Misc/RaceMask` — for `ChrRaces` / `ChrClasses` filters.
- `Globals/Containers` — `Trinity::Containers::MapGetValuePtr`, `MapEqualRange`.
- `Logging` — `TC_LOG_INFO("server.loading", …)`.
- `Boost::filesystem` — for `directory_iterator` over `dbc/` locale subfolders.

**Depended on by:**
Almost everything in `game/`. A non-exhaustive list of obvious consumers (every one references at least one `s*Store`):
- `Entities/Player/*.cpp` — every Player.{cpp,Stats,Spells,Quests} hits 30-50 stores.
- `Maps/*.cpp` — `sMapStore`, `sMapDifficultyStore`, `sAreaTableStore`.
- `Spells/*.cpp` — `sSpellStore`-equivalent (split across many `Spell*` tables in WoLK 3.4.3).
- `Achievements/*.cpp` — `sAchievementStore`, `sCriteriaStore`, `sCriteriaTreeStore`.
- `Quests/*.cpp` — `sQuestPackageItemStore`, `sQuestLineXQuestStore`.
- `BattleGrounds/*.cpp` — `sBattlemasterListStore`, `sPVPDifficultyStore`.
- `Garrison/*.cpp` — `sGarrBuildingStore`, `sGarrFollowerStore`, `sGarrMissionStore`.
- `Loot/*.cpp` — `sItemStore`, `sItemSparseStore`.
- `Server/WorldSession::HandleDbQueryBulk` — `DB2StorageBase::WriteRecord` for SMSG_DB_REPLY.

---

## 6. SQL / DB queries (if any)

Three "control" tables in the `hotfixes` MariaDB:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT Id, UniqueId, TableHash, RecordId, Status FROM hotfix_data ORDER BY Id` | Master list of all known hotfixes (push-id grouped) | hotfixes |
| `SELECT TableHash, RecordId, locale, Blob FROM hotfix_blob ORDER BY TableHash` | Pre-rendered binary blobs for tables not loaded server-side | hotfixes |
| `SELECT TableHash, RecordId, locale, Key, Data FROM hotfix_optional_data ORDER BY TableHash` | Auxiliary key-blob pairs (BroadcastText TACT keys, etc.) | hotfixes |

Plus the ~783 per-table statements registered through `DB2LoadInfo::Instance.Statement` (see shared doc section 6).

**DBC/DB2 stores (the ~261 game-side stores):**

| Store | What it loads | Read by |
|---|---|---|
| `sAchievementStore` | Achievement.db2 | `Achievement.cpp`, `CriteriaHandler.cpp` |
| `sAreaTableStore` | AreaTable.db2 | `Map.cpp`, `Player::UpdateZone`, `WorldStateMgr` |
| `sBattlemasterListStore` | BattlemasterList.db2 | `BattlegroundMgr.cpp`, `LFG/LFGMgr.cpp` |
| `sBroadcastTextStore` | BroadcastText.db2 | `Creature::FormatBroadcastText`, gossip system |
| `sChrClassesStore` / `sChrRacesStore` / `sChrSpecializationStore` | ChrClasses.db2 / ChrRaces.db2 / ChrSpecialization.db2 | Char-creation, talents |
| `sCreatureFamilyStore` / `sCreatureModelDataStore` / `sCreatureDisplayInfoStore` | Creature*.db2 | `Creature.cpp`, pets |
| `sCurveStore` / `sCurvePointStore` | Curve.db2 | item scaling, XP scaling |
| `sFactionStore` / `sFactionTemplateStore` | Faction.db2 | `Reputation`, `Unit::IsHostileTo` |
| `sItemStore` / `sItemSparseStore` / `sItemEffectStore` | Item.db2, ItemSparse.db2, ItemEffect.db2 | `ItemTemplate.cpp`, every loot/inventory handler |
| `sLFGDungeonsStore` | LFGDungeons.db2 | `LFGMgr.cpp` |
| `sLightStore` / `sLiquidTypeStore` | Light.db2, LiquidType.db2 | terrain, swimming, weather |
| `sLockStore` | Lock.db2 | container/door open mechanic |
| `sMapStore` / `sMapDifficultyStore` | Map.db2, MapDifficulty.db2 | `MapManager.cpp`, instance system |
| `sMountStore` / `sMountTypeXCapabilityStore` / `sMountXDisplayStore` | Mount*.db2 | `Player::SetMount`, mount journal |
| `sPhaseStore` | Phase.db2 | phasing system |
| `sPlayerConditionStore` | PlayerCondition.db2 | condition checks throughout |
| `sQuestPackageItemStore` / `sQuestLineXQuestStore` | QuestPackageItem.db2, QuestLineXQuest.db2 | `Quest*.cpp` |
| `sSkillLineStore` / `sSkillLineAbilityStore` / `sSkillRaceClassInfoStore` | SkillLine*.db2 | skill system, talents |
| `sSpellNameStore` (and 30+ split spell tables) | Spell*.db2 | every spell handler |
| `sTalentStore` / `sPvpTalentStore` | Talent.db2, PvpTalent.db2 | talent grid |
| `sTaxiNodesStore` / `sTaxiPathStore` / `sTaxiPathNodeStore` | Taxi*.db2 | flight masters |
| `sUiMapStore` / `sUiMapAssignmentStore` / `sWorldMapOverlayStore` | UiMap*.db2, WorldMapOverlay.db2 | minimap / world-map overlays |
| `sVehicleStore` / `sVehicleSeatStore` | Vehicle.db2 | vehicle system |
| `sWMOAreaTableStore` | WMOAreaTable.db2 | WMO → area resolution |
| (full list: 261 entries — see `DB2Stores.h:38-260`) | | |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_AVAILABLE_HOTFIXES` | server → client | `WorldSession::HandleEnterWorld` (or similar — TC initial-login flow); body built from `DB2Manager::GetHotfixData` |
| `SMSG_HOTFIX_LIST` | server → client | initial login |
| `SMSG_HOTFIX_PUSH` | server → client | `DB2HotfixGeneratorBase::AddClientHotfix` triggers an enqueue; flushed during world tick |
| `CMSG_HOTFIX_REQUEST` | client → server | initial login (client checksums its DB2 cache; server compares with its known set) |
| `CMSG_DB_QUERY_BULK` | client → server | `WorldSession::HandleDbQueryBulk` |
| `SMSG_DB_REPLY` | server → client | per-record body via `DB2StorageBase::WriteRecord` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `crates/wow-data/src/item_stats.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |
| `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |
| `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/wdc4.rs` — 915 lines — generic WDC4 binary reader (covers shared layer; see `shared-datastores.md`).
- `crates/wow-data/src/hotfix_cache.rs` — pre-loads all `.db2` raw record bytes for the active locale, reads the three C++ hotfix control tables (`hotfix_data`, `hotfix_blob`, `hotfix_optional_data`), and serves `SMSG_DB_REPLY` / `SMSG_HOTFIX_CONNECT` content. This is still not a full `DB2Manager`: it is a blob-serving bridge without typed `Storage<T>` overlays or generated DB2 structs.
- `crates/wow-data/src/item.rs` — 123 lines — `ItemRecord` (6 fields) + `ItemStore` (HashMap loader).
- `crates/wow-data/src/item_stats.rs` — 424 lines — `ItemStatEntry` from ItemSparse.db2 (~30 hand-picked fields out of ~135).
- `crates/wow-data/src/player_stats.rs` — 307 lines — wraps ChrClassUIDisplay-equivalent? No — wraps the `gtChanceToMeleeCrit*.txt` family (a `GameTable`-equivalent for WoLK 3.4.3 base stats per class+level). Hand-rolled.
- `crates/wow-data/src/skill.rs` — 608 lines — SkillLineAbility lookup.
- `crates/wow-data/src/area_trigger.rs` — 312 lines — AreaTrigger.db2 + AreaTriggerTeleport.db2 + the matching DB rows.
- `crates/wow-data/src/spell.rs` — 225 lines — partial Spell.db2 surface (cast time, GCD, cooldown, basic effects).
- `crates/wow-data/src/quest.rs` — 337 lines — Quest tables.
- `crates/wow-data/src/quest_xp.rs` — 116 lines — QuestXP table.
- `crates/wow-database/src/statements/hotfix.rs` — 25 lines — placeholder enum (`_PLACEHOLDER` only).
- world-server `main.rs` calls `wow_data::build_hotfix_blob_cache(&data_dir, &locale)` once at startup.

**What's implemented:**
- Direct binary reads of all `.db2` files for the active locale into a table-hash/record-id blob cache for `CMSG_DB_QUERY_BULK`.
- `hotfix_data`, `hotfix_blob`, and `hotfix_optional_data` control-table reads, plus wire support for `SMSG_AVAILABLE_HOTFIXES`, `CMSG_HOTFIX_REQUEST`, `SMSG_HOTFIX_CONNECT`, and `SMSG_DB_REPLY`.
- Hand-written typed readers still exist for the gameplay stores currently consumed by Rust (Item, ItemSparse, AreaTrigger, Spell-partial, Skill*, Quest, QuestXP, GameTable-style player stats, etc.).

**What's missing vs C++:**
1. **`DB2Manager` singleton** — does not exist. There is no global `sDB2Manager` and no `_stores` registry mapping `table_hash -> &dyn Storage`.
2. **~253 of 261 store equivalents** — Achievement, AreaTable, BattlemasterList, BroadcastText, ChrClasses, ChrRaces, ChrSpecialization, CreatureFamily, CurrencyType, Curve+CurvePoint, Faction, FactionTemplate, GameObjectsEntry, ItemEffect, LFGDungeons, Light, LiquidType, Lock, Map, MapDifficulty, Mount*, Phase, PlayerCondition, QuestPackageItem, SkillRaceClassInfo, SpellEffect, Talent, Taxi*, UiMap*, Vehicle*, WMOAreaTable, … all absent.
3. **Auto-generated record structs** — `DB2Structure.h` (4538 LoC, 325 structs) has no Rust analogue. The 8 hand-written readers cover ~6 of 325 tables.
4. **Auto-generated meta** — `DB2Metadata.h` (12067 LoC, 788 structs) — none.
5. **Auto-generated load info** — `DB2LoadInfo.h` (6357 LoC, 325 structs) — none.
6. **`DBCEnums.h`** — no central place for `Difficulty`, `Powers`, `MapType`, `BattlegroundBracketId`, `LevelLimit`. These are scattered or hard-coded. (`wow-constants` may have a few.)
7. **Hotfix-DB typed overlay** — the three control-table reads exist, but the generated per-DB2 hotfix table statements and typed `Storage<T>` merge path are still missing. Runtime `RecordRemoved` is approximated through blob-cache table presence until real `DB2Manager` storage exists.
8. **`DB2HotfixGenerator`** — runtime in-memory record patcher; no analogue.
9. **`SMSG_HOTFIX_PUSH`** flow — `SMSG_AVAILABLE_HOTFIXES` and `CMSG_HOTFIX_REQUEST`/`SMSG_HOTFIX_CONNECT` are wired; runtime push queue from `DB2HotfixGeneratorBase::AddClientHotfix` is still absent.
10. **Most `DB2Manager` accessor helpers** — `GetCurveValueAt`, `EvaluateExpectedStat`, `GetMapDifficultyData`, `GetTalentsByPosition`, `GetPhasesForGroup`, `GetUiMapPosition`, `Map2ZoneCoordinates`, `GetWMOAreaTable`, `IsInArea`, `GetBroadcastTextValue`, `ValidateName` — none.
11. **GameTables** — sixteen TSV files (`gtBaseMP`, `gtCombatRatings`, `gtSpellScaling`, `gtHpPerSta`, `gtRegen*`, `gtShieldBlockRegular`, …) — `player_stats.rs` covers a subset of one of them; the rest are absent.
12. **M2 cinematic camera parser** — no analogue. Cinematic opcodes will be unable to send fly-by data.
13. **Locale stacking** — single-locale only.
14. **`LayoutHash` validation** — absent at the consumer level (the `Wdc4Reader` exposes `table_hash()` but no per-store enforcement).

**Suspicious / likely divergent:**
- The blob cache for `Item.db2`/`ItemSparse.db2` will serve **file bytes** to the client even after a hotfix is applied to the typed copy. C++ TC re-serializes the patched in-memory record. As soon as hotfix overlay lands, the blob cache must be re-keyed off the typed store — not the raw file.
- The hand-rolled per-table readers use **literal field-index integers** (`get_field_u8(idx, 0)`, `get_field_u8(idx, 1)`, …). The `LayoutHash` from each DB2 file is not pinned anywhere; the next time Blizzard re-shuffles columns, every hard-coded index silently reads the wrong field.
- Spell.db2 in WoLK 3.4.3 is `SpellName.db2` plus 30+ companion tables (`SpellEffect`, `SpellCategories`, `SpellMisc`, `SpellAuraOptions`, …). The `spell.rs` reader handles only one of those — the runtime `SpellInfo` is therefore incomplete vs TC's composed `SpellInfo` (assembled by joining ~30 DB2s in `SpellMgr::LoadSpellInfoStore`).

**Tests existing:** unknown; checking `cargo test -p wow-data --lib` would enumerate. The `wdc4.rs` parser likely has a few decoder unit tests but no per-table integration tests of "DB2 file → typed struct → server semantics".

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#DATASTORES.WBS.001** Cerrar la migracion auditada de `game/DataStores/DB2HotfixGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2HotfixGenerator.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.002** Cerrar la migracion auditada de `game/DataStores/DB2HotfixGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2HotfixGenerator.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.003** Partir y cerrar la migracion auditada de `game/DataStores/DB2LoadInfo.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 6357 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.004** Partir y cerrar la migracion auditada de `game/DataStores/DB2Metadata.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 12067 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.005** Partir y cerrar la migracion auditada de `game/DataStores/DB2Stores.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Stores.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3104 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.006** Partir y cerrar la migracion auditada de `game/DataStores/DB2Stores.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Stores.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 516 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.007** Partir y cerrar la migracion auditada de `game/DataStores/DB2Structure.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4538 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.008** Partir y cerrar la migracion auditada de `game/DataStores/DBCEnums.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DBCEnums.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2514 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.009** Cerrar la migracion auditada de `game/DataStores/GameTables.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/GameTables.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.010** Cerrar la migracion auditada de `game/DataStores/GameTables.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/GameTables.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.011** Cerrar la migracion auditada de `game/DataStores/M2Stores.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/M2Stores.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.012** Cerrar la migracion auditada de `game/DataStores/M2Stores.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/M2Stores.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATASTORES.WBS.013** Cerrar la migracion auditada de `game/DataStores/M2Structure.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/M2Structure.h`
  Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#GDS.1** Define `pub struct DB2Manager` with `OnceLock<Arc<DB2Manager>>` global accessor `db2_manager()`. Inner state: `stores: DashMap<u32 table_hash, Arc<dyn DynStorage>>`, `hotfix_data: HashMap<i32 push_id, HotfixPush>`, `hotfix_blob: [HashMap<(u32, i32), Vec<u8>>; LOCALE_COUNT]`, `hotfix_optional_data: [HashMap<(u32, i32), Vec<HotfixOptionalData>>; LOCALE_COUNT]`. (complexity: **M**)
- [ ] **#GDS.2** Centralize all gameplay enums in `crates/wow-constants/src/dbc_enums.rs` (port `DBCEnums.h`). At minimum: `Difficulty`, `BattlegroundBracketId`, `MapType`, `Powers`, `ItemQualities`, `LevelLimit`, `DBCPosition2D`/`DBCPosition3D`. (complexity: **M**)
- [ ] **#GDS.3** Generate `crates/wow-data/src/generated/structs.rs` from `DB2Structure.h` (325 structs). Split into per-domain modules (`item.rs`, `spell.rs`, `chr.rs`, `map.rs`, `quest.rs`, `loot.rs`, `gear.rs`, `pvp.rs`, `garrison.rs`, `misc.rs`). Use `#[repr(C)]`, `LocalizedString` and `Cstr` typedefs. (complexity: **XL** — split per-domain.)
- [ ] **#GDS.4** Generate `crates/wow-data/src/generated/meta.rs` from `DB2Metadata.h` (788 structs). One `pub static <NAME>_META: TableMeta = …` per table. (complexity: **H**)
- [ ] **#GDS.5** Generate `crates/wow-data/src/generated/load_info.rs` from `DB2LoadInfo.h` (325 structs). One `pub static <NAME>_LOAD_INFO: LoadInfo = …` per table; references both the meta and the `HotfixStatements` enum. (complexity: **H**)
- [ ] **#GDS.6** Define ~261 `pub static <NAME>_STORE: OnceLock<Arc<Storage<XEntry>>>` globals (or expose them via `DB2Manager::store::<XEntry>()`) and the load-order in `DB2Manager::load_stores(data_path, default_locale)`. Mirror `DB2Stores.cpp:593-1100` LOAD_DB2 sequence. (complexity: **XL**, splitter — first cut just the 25 stores actually consumed by current Rust code.)
- [x] **#GDS.7a** Interim `HotfixBlobCache::load_hotfix_data_from_db(locale)` bridge for `hotfix_data`, contrasted with `DB2Stores.cpp:1539-1607`. Remaining full task: typed `DB2Manager::load_hotfix_data(locale_mask)` with real `store.erase_record(record_id)` for `Status::RecordRemoved`. (closed by `5916cbd` + current Hotfix handshake change)
- [x] **#GDS.8a** Interim `HotfixBlobCache::load_hotfix_blobs_from_db(locale)` bridge for `hotfix_blob`, contrasted with `DB2Stores.cpp:1609-1654`. Remaining full task: real `DB2Manager::load_hotfix_blob(locale_mask)` over typed stores. (closed by `5916cbd`)
- [x] **#GDS.9a** Interim `HotfixBlobCache::load_hotfix_optional_data_from_db(locale)` bridge for `hotfix_optional_data` and appending optional blobs to `DBReply`/`HotfixConnect`, contrasted with `DB2Stores.cpp:1661-1736` and `HotfixHandler.cpp`. Remaining full task: enforce the C++ per-table allowlist once typed BroadcastText/TactKey stores exist. (closed by current Hotfix handshake change)
- [x] **#GDS.10a** Wire `SMSG_AVAILABLE_HOTFIXES` and `CMSG_HOTFIX_REQUEST`/`SMSG_HOTFIX_CONNECT` packet builders consuming loaded hotfix data, contrasted with `HotfixHandler.cpp` and `HotfixPackets.cpp`. Remaining full task: `SMSG_HOTFIX_PUSH` runtime queue. (closed by current Hotfix handshake change)
- [ ] **#GDS.7** Implement full typed `DB2Manager::load_hotfix_data(locale_mask)`. Mirror `DB2Stores.cpp:1539-1607`. Includes the post-loop pass that calls `store.erase_record(record_id)` for `Status::RecordRemoved`. (complexity: **M**)
- [ ] **#GDS.8** Implement full typed `DB2Manager::load_hotfix_blob(locale_mask)`. Mirror `DB2Stores.cpp:1609-1654`. (complexity: **L**)
- [ ] **#GDS.9** Implement full typed `DB2Manager::load_hotfix_optional_data(locale_mask)`. Includes the per-table allowlist (BroadcastText → TactKey). Mirror `DB2Stores.cpp:1661-1736`. (complexity: **M**)
- [ ] **#GDS.10** Wire `SMSG_HOTFIX_PUSH` packet builders consuming runtime DB2 hotfix generation queue. (complexity: **M**)
- [ ] **#GDS.11** Implement `DB2HotfixGenerator<T>` — runtime in-memory patcher. API: `DB2Manager::patch::<XEntry>(id, |entry| { entry.field = …; }, notify_client: bool)`. Notify-client routes through `DB2HotfixGeneratorBase::add_client_hotfix(table_hash, id)` which appends to a queue flushed by `WorldSession`. (complexity: **M**)
- [ ] **#GDS.12** Port the curve interpolator: `DB2Manager::curve_value_at(curve_id, x)`. Linear, cosine, cubic-spline modes. Used everywhere — XP, item-level, content tuning. (complexity: **M**)
- [ ] **#GDS.13** Port `DB2Manager::evaluate_expected_stat(stat, level, expansion, content_tuning_id, class)`. Reads `ExpectedStat.db2` + `ExpectedStatMod.db2`. (complexity: **M**)
- [ ] **#GDS.14** Port the ~80 derived helpers (group by domain): items (`item_set_spells`, `item_spec_overrides`, `heirloom_by_item`, `item_display_id`, `default_item_modified_appearance`), maps (`default_map_difficulty`, `map_difficulty_data`, `downscaled_map_difficulty_data`, `lfg_dungeon`, `default_map_light`, `liquid_flags`), chr (`chr_model`, `chr_specialization_by_index`, `default_chr_specialization_for_class`, `customization_choices`, `customization_options`, `power_index_by_class`, `power_type_entry`, `class_name`, `chr_race_name`), reputation (`faction_team_list`, `friendship_rep_reactions`, `paragon_reputation`), mount (`mount`, `mount_by_id`, `mount_capabilities`, `mount_displays`), skills (`skill_lines_for_parent_skill`, `skill_line_abilities_by_skill`, `skill_race_class_info`), pvp (`battleground_bracket_by_level`, `battleground_bracket_by_id`), quest (`quests_for_quest_line`, `quest_package_items`, `quest_unique_bit_flag`), talents (`talents_by_position`, `glyph_bindable_spells`, `glyph_required_specs`, `num_talents_at_level`), areas (`areas_for_group`, `is_in_area`, `phases_for_group`), ui-map (`ui_map_position`, `zone_to_map_coordinates`, `map_to_zone_coordinates`, `wmo_area_table`), broadcast (`broadcast_text_value`, `text_sound_emote_for`), validation (`validate_name`, `name_gen_entry`), misc (`is_toy_item`, `transmog_sets_for_item_modified_appearance`, `transmog_set_items`, `currency_container_for_currency_quantity`, `total_curve_value_at`). (complexity: **XL**, splitter — bucket per consumer.)
- [ ] **#GDS.15** Build secondary indexes that `DB2Manager` populates after `LoadStores`: `_mountsBySpellId`, `_skillLineAbilitiesBySkill`, `_questPackageItems`, `_factionTeamList`, `_glyphBindableSpells`, `_phaseGroups`, `_areaGroupMembers`, `_taxiPathSetBySource`, `_curvePoints`, `_uiMapBounds`, etc. ~30 indexes. Each is built from a `for entry in store.iter()` loop. (complexity: **XL**)
- [ ] **#GDS.16** Port `GameTables`: implement `pub struct GameTable<T>(Vec<T>)` indexed by level. Implement TSV parser (tab-separated, floats, header row). Define the 16 `GtXEntry` structs and globals: `s_artifact_knowledge_multiplier_game_table`, `s_artifact_level_xp_game_table`, `s_barber_shop_cost_base_game_table`, `s_base_mp_game_table`, `s_battle_pet_xp_game_table`, `s_combat_ratings_game_table`, `s_combat_ratings_mult_by_ilvl_game_table`, `s_hp_per_sta_game_table`, `s_item_socket_cost_per_level_game_table`, `s_npc_mana_cost_scaler_game_table`, `s_oct_regen_hp_game_table`, `s_oct_regen_mp_game_table`, `s_regen_hp_per_spt_game_table`, `s_regen_mp_per_spt_game_table`, `s_shield_block_regular_game_table`, `s_spell_scaling_game_table`. (complexity: **H**)
- [ ] **#GDS.17** Port the per-class/per-quality column selectors: `regen_game_table_column_for_class<T>(row, class)`, `spell_scaling_column_for_class(row, class)`, `shield_block_regular_column_for_quality(row, quality)`, `battle_pet_xp_per_level(row)`. (complexity: **L**)
- [ ] **#GDS.18** Port `M2Stores`: `LoadM2Cameras`, `GetFlyByCameras`. Requires an M2 binary parser (covered partly in `M2Structure.h`). (complexity: **M**)
- [ ] **#GDS.19** Audit every existing call site in `wow-world` / `wow-spell` / `wow-combat` / `wow-quest` / `wow-loot` that hits the current 8 hand-rolled stores; switch to `DB2Manager::store::<XEntry>().lookup(id)`. Delete the bespoke modules (`item.rs`, `item_stats.rs`, `skill.rs`, `area_trigger.rs`, `spell.rs`, `quest.rs`, `quest_xp.rs`, `player_stats.rs`). (complexity: **H**)
- [ ] **#GDS.20** Refit `HotfixBlobCache` to consult `Storage<T>::write_record` (post-overlay) rather than reading raw file bytes. After this, the blob cache is no longer "hotfix" misnamed — rename to `Db2BlobCache`. (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#DATASTORES.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 13 files / 30200 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-database`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#DATASTORES.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 13 files / 30200 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#DATASTORES.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 13 files / 30200 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#DATASTORES.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 13 files / 30200 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] `DB2Manager::load_stores` finds all expected DB2 files in `dbc/enUS/`; missing files produce listed errors but don't abort.
- [ ] Locale fallback: if `dbc/frFR/` lacks a file, the `enUS` strings are used (mirror TC behavior — only string columns fall back per-locale).
- [ ] `DB2Manager::load_hotfix_data` with a fixture `hotfix_data` table: a row with `Status=Valid` and `TableHash=Map` patches the in-memory `MapEntry` after `Storage::load_from_db`.
- [ ] A row with `Status=RecordRemoved` causes `MapStore::lookup(removed_id)` to return `None`.
- [ ] `load_hotfix_blob`: a row for a table not loaded server-side is reachable via `DB2Manager::get_hotfix_blob(tableHash, recordId, locale)`.
- [ ] `load_hotfix_optional_data` rejects rows for non-allowed tables (logs error, does not insert).
- [ ] BroadcastText TactKey blob (24 bytes: 8 for keyId + 16 for AES-128) passes the validator; non-24-byte blob is rejected.
- [ ] `curve_value_at(curveId, x)` reproduces TC's interpolation for at least linear, cosine, and Catmull-Rom curves (use a fixture from a known-good TC binary).
- [ ] `get_default_map_difficulty(map_id, &mut difficulty)` returns the same default as TC for at least 5 known maps (instances + battlegrounds).
- [ ] `is_in_area(child_area, parent_area)` walks the parent chain correctly.
- [ ] `GameTable::get_row(level)` for `gtCombatRatings` matches a fixture row at level 80, 60, 1.
- [ ] `regen_game_table_column_for_class(row, CLASS_WARRIOR)` returns `row.warrior` (sanity test for the selector code-gen).
- [ ] `LoadM2Cameras` parses a known cinematic file and yields the right number of keyframes.
- [ ] `SMSG_DB_REPLY` body bytes for `lookup(map_id=0).unwrap()` match TC's output for `MapEntry`.
- [ ] `SMSG_AVAILABLE_HOTFIXES` push-id ordering is by `int32 PushID` ascending (matches `_hotfixData`'s `std::map` iteration order).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 13 files / 30200 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h` | `crates/wow-data/` (also touches `crates/wow-database/` for hotfix statements) \| ⚠️ partial — confirmed via audit 2026-05-01 (raw WDC4 reader OK; **7 hand-rolled tables of ~261**, not 8; no DB2Manager, no hotfix overlay, no GameTables, no M2 cameras) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#DATASTORES.DIV.001` | _none generated_ | 13 C++ files / 30200 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Metadata.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2LoadInfo.h`, `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Structure.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **Spell tables are split**: WoLK 3.4.3 (post-MoP DB2 split) replaces a monolithic `Spell.db2` with `SpellName.db2` + `SpellEffect.db2` + `SpellMisc.db2` + `SpellCategories.db2` + `SpellAuraOptions.db2` + `SpellLevels.db2` + `SpellRange.db2` + `SpellCooldowns.db2` + `SpellEquippedItems.db2` + `SpellInterrupts.db2` + `SpellShapeshift.db2` + `SpellTargetRestrictions.db2` + `SpellAuraRestrictions.db2` + `SpellCastingRequirements.db2` + `SpellCastTimes.db2` + `SpellDuration.db2` + `SpellPower.db2` + `SpellReagents.db2` + `SpellTotems.db2` + `SpellClassOptions.db2` + `SpellAdditionalCastTimes.db2` + `SpellRuneCost.db2` + ~10 more. Server "SpellInfo" is composed by joining these by `SpellNameID`/`SpellID`/`DifficultyID`. **The Rust `spell.rs` reader's `SpellInfo` is therefore at most 1/30th of the truth**.
- **`HotfixId`'s ordering matters**: `(PushID, UniqueID)` lexicographic ordering — `<=>` is defaulted in C++. The Rust `Ord` impl must mirror this exactly because the client expects pushes in this order.
- **`Status::NotPublic` rows**: loaded server-side, but **not** sent to clients in `SMSG_AVAILABLE_HOTFIXES`. Used for staff-only items and unreleased content. The Rust port should respect this status during the push assembly.
- **`Status::Invalid`**: rows that fail validation but are kept in `hotfix_data` for audit. Don't apply.
- **`hotfix_blob` for tables also loaded server-side is an error**: TC logs `"Table hash 0x{:X} points to a loaded DB2 store {}, fill related table instead of hotfix_blob"`. Replicate the warning.
- **Localization fallback at the *string* level (not row level)**: TC walks `LoadStringsFrom` per locale on top of the already-loaded data. If `frFR` is missing the `Item.db2` for one row, that row keeps the `enUS` string for that field while other rows have `frFR`. The hotfix overlay does the same: only string fields are touched by `LoadStringsFromDB`.
- **`DB2HotfixGenerator::ApplyHotfix` casts away const**: the in-memory `T` is normally `T const*` (LookupEntry returns const), but the generator does `const_cast<T*>(entry)` to mutate it. The Rust port should expose mutation through interior mutability (`RwLock<Storage<T>>` or similar) — never replicate the const_cast pattern.
- **GameTables vs DB2**: GameTables are **plain tab-separated text files** under `dataPath/gt/`, *not* DB2 binaries. The TSV column count is asserted equal to `sizeof(T) / sizeof(float)`. For Rust, replicate via a `const COLUMN_COUNT: usize` and a build-time assert (or a test).
- **Talents / pvp talents / spec spells are auto-generated lookup tables**: `_talentsByPosition[class][tier][col] -> Vec<&TalentEntry>`. Pre-built at `LoadStores` end. Don't recompute per query.
- **`UiMap` is a tree** (parent → children); `Map2ZoneCoordinates` walks it. The auto-built `_uiMapBounds` index is critical for performance — never iterate the whole store per query.
- **`sCurveStore` + `sCurvePointStore`**: a curve is stored across **two tables** — Curve.db2 has `id, type` (linear/cubic/cosine/catmull), CurvePoint.db2 has the actual `(x, y)` points keyed by `CurveID`. `GetCurveValueAt` joins them at load time into a sorted `Vec<DBCPosition2D>` per curve.
- **`sChrClassesXPowerTypesStore`** is the only way to know "what powers does class X have, in what slot order" — for Druid this matters because `POWER_RAGE` and `POWER_MANA` and `POWER_ENERGY` are all valid depending on form. Pre-build `_powerIndexByClass` at load.
- **`min_id` ≠ 0 for many tables**: don't naively `[0..max_id]`.
- **WoLK 3.4.3 vs Retail tables**: most tables exist in both, but with different field counts/orders. **The auto-generator must consume the WoLK 3.4.3 client's exact DB2 layout.** A meta generated from a retail TC tree will compile but read garbage.
- **Battle.net account-name validation** (`ValidateName`) requires `sNamesProfanityStore` and `sNamesReservedStore` — these are tiny but *must* be loaded before character creation handlers can reject names.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class DB2Manager` (singleton) | `pub struct DB2Manager` + `pub fn db2_manager() -> &'static Arc<DB2Manager>` | Or pass `Arc<DB2Manager>` through `WorldServer` — preferred over global |
| `extern DB2Storage<XEntry> sXStore` (×261) | `pub static X_STORE: OnceLock<Arc<Storage<XEntry>>>` (×261) | Or accessor via `db2_manager().store::<XEntry>()` |
| `_stores: unordered_map<uint32, DB2StorageBase*>` | `DashMap<u32, Arc<dyn DynStorage>>` | Type-erased, lookup-by-table-hash |
| `DB2Manager::HotfixId` | `#[derive(Eq, Ord)] pub struct HotfixId { push_id: i32, unique_id: u32 }` | Default `Ord` matches C++ `<=>` |
| `DB2Manager::HotfixRecord` | `pub struct HotfixRecord { table_hash: u32, record_id: i32, id: HotfixId, status: HotfixStatus, available_locales_mask: u32 }` | — |
| `DB2Manager::HotfixRecord::Status` (enum class : uint8) | `#[repr(u8)] enum HotfixStatus { NotSet=0, Valid=1, RecordRemoved=2, Invalid=3, NotPublic=4 }` | — |
| `DB2Manager::HotfixContainer (std::map)` | `BTreeMap<i32, HotfixPush>` | Ordered; iteration order matters |
| `DB2Manager::FriendshipRepReactionSet (std::set with comparator)` | `BTreeSet<&'static FriendshipRepReactionEntry>` (custom `Ord` via newtype) | Or `Vec` if cardinality is small |
| `DEFINE_DB2_SET_COMPARATOR(s)` macro | declarative macro `define_db2_set_comparator!(XEntry, |a, b| …)` | Or just a `wrapper` newtype with `Ord` |
| `template GameTable<T>` | `pub struct GameTable<T> { rows: Vec<T> }` | `get_row(level)` returns `Option<&T>` |
| `LoadM2Cameras / GetFlyByCameras` | `pub fn load_m2_cameras(data_path: &Path)` + `pub fn fly_by_cameras(cinematic_camera_id: u32) -> Option<&[FlyByCamera]>` | Backed by static OnceLock |
| `DB2HotfixGenerator<T>::ApplyHotfix` | `Storage<T>::patch(id, |entry: &mut T| { … })` returning bool | Done through interior mutability — no `const_cast` |
| `DB2HotfixGeneratorBase::AddClientHotfix` | enqueue into `DB2Manager::pending_client_pushes: Vec<(u32, i32)>` | Flushed by world tick into `SMSG_HOTFIX_PUSH` |
| `LocalizedString` (in records) | `[Option<&'static str>; LOCALE_COUNT]` or `[Arc<str>; LOCALE_COUNT]` | See shared doc |
| `DBCPosition2D { float X; float Y; }` | `#[repr(C)] struct DbcPosition2D { x: f32, y: f32 }` (in `wow-constants`) | Distinguish from `Position` (which has orientation) |
| `DBCEnums.h` (`Difficulty`, `Powers`, `MapType`, `BattlegroundBracketId`, `LevelLimit`) | `wow-constants/src/dbc_enums.rs` | Already partially scattered; centralize |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Method:** Listed `crates/wow-data/src/` and `crates/wow-data/src/lib.rs`; cross-referenced against the C++ `DataStores/` inventory and the `shared-datastores.md` audit (which already established a 5/325 baseline at the shared-loader layer).

**Findings:**

- `crates/wow-data/src/lib.rs` exports exactly 8 modules: `wdc4` (the parser, shared-layer), `item`, `item_stats`, `hotfix_cache`, `player_stats`, `skill`, `area_trigger`, `spell`. No `quest.rs` / `quest_xp.rs` (the §8 claim of 10 modules was inaccurate at audit time — they were either renamed/removed since the doc draft).
- **Game-side typed tables actually wrapped: 7** (Item, ItemSparse, AreaTrigger+AreaTriggerTeleport, SkillLineAbility, SpellName-partial, plus the GameTable-shaped `player_stats` per-class baseline). The `shared-datastores.md` count of "5 .db2 files parsed" is a stricter metric (one count per .db2 file); this doc's "~8" should be normalized to **7 hand-rolled typed wrappers / ~261 game-side stores ≈ 2.7%**.
- **No `DB2Manager` analogue.** No `OnceLock<Arc<DB2Manager>>`, no `_stores: DashMap<u32, ...>`, no centralized hotfix push container.
- **No hotfix-DB overlay.** `hotfix_cache.rs` (111 LOC) is a *file-bytes* cache for `Item.db2` / `ItemSparse.db2` only — exactly as §8 describes. The misleading-name issue is real: this is **not** the C++ `DB2Manager::LoadHotfixBlob` equivalent.
- **No GameTables.** None of the 16 `gtX.txt` TSV files (`gtCombatRatings`, `gtSpellScaling`, `gtBaseMP`, `gtHpPerSta`, `gtRegen*`, etc.) are loaded. `player_stats.rs` covers only the per-class baseline subset.
- **No M2 cinematic camera parser.** `LoadM2Cameras` / `GetFlyByCameras` absent — cinematic opcodes won't have keyframe data.
- **No DBCEnums centralization.** `Difficulty`, `Powers`, `MapType`, `BattlegroundBracketId`, `LevelLimit` not centralized in `wow-constants`.
- **DB statement enum:** `crates/wow-database/src/statements/hotfix.rs` is a 25-line `_PLACEHOLDER` stub — the ~783 per-table prepared statements are not registered, and the three control tables (`hotfix_data`, `hotfix_blob`, `hotfix_optional_data`) are never queried.

**Cross-reference vs `shared-datastores.md`:** the shared-layer audit found 5 `.db2` files parsed (Item, ItemSparse, SkillLineAbility, AreaTrigger, SpellName — its file count). This game-side layer wraps those into 7 typed accessors plus 1 GameTable shim (`player_stats`), but it does not add the architectural pieces the C++ game layer adds: stores registry, hotfix overlay, accessor helpers, secondary indexes. **The game-side gap is therefore strictly larger than the shared-side gap** — even if every shared-layer .db2 reader worked, the ~80 `DB2Manager::Get*` helpers would still all be missing.

**Status verdict:** ⚠️ partial (no change). Tighten §1 wording: 7 typed wrappers (not 8), ~2.7% of game-side surface, zero of the 80 `DB2Manager::Get*` helpers, zero of the 16 GameTables, zero of the 3 hotfix-DB control tables. Sub-tasks #GDS.1, #GDS.7-9, #GDS.16 are the highest-leverage next steps.
