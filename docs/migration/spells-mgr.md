# Migration: Spells — SpellMgr (loader / static-data registry)

> **C++ canonical path:** `src/server/game/Spells/SpellMgr.{h,cpp}` (~5,855 lines combined: 827 + 5,028)
> **Rust target crate(s):** `crates/wow-spell/src/spell_mgr.rs` (planned), `crates/wow-data/src/spell.rs` (DB2 readers — partial)
> **Layer:** L5 sub-module (Game systems — Spells loader). Parent: `spells.md`. Sibling: `spells-info.md`. Depends downward on `shared-datastores.md` (DB2 readers) and on `wow-database` (SQL).
> **Status:** ❌ not started — the entire singleton is missing in Rust. Audited and confirmed empty 2026-05-01.
> **Audited vs C++:** ✅ complete (see §13)
> **Last updated:** 2026-05-01

> **Cross-links:** see `spells.md` for the engine-level overview, `spells-info.md` for the composed `SpellInfo` consumer of this loader's output, and `shared-datastores.md` for the underlying WDC4 reader + `hotfixes` overlay that `LoadSpellInfoStore` builds upon.

---

## 1. Purpose

`SpellMgr` is the **process-wide singleton** that turns a pile of binary DB2 files plus ~14 sibling `spell_*` SQL tables into the in-memory `SpellInfo` registry the rest of the game core queries every cast. It owns ~17 secondary maps (chains, ranks, required spells, learn-spell graph, target positions, threat overrides, proc rules, group stack rules, area-conditional auras, linked spells, pet auras, enchant proc data, skill-line abilities, pet level-up tables, totem models, …) and is the **only** writer of the global `mSpellInfoMap`. Once startup load is complete it is logically read-only — no spell data is mutated at runtime; hotfixes apply once during boot via the DB2 overlay path.

The class also enforces cross-table invariants: it rejects a `spell_required` row whose two spells are ranks of the same chain, refuses a `serverside_spell_effect` that overlaps an existing DB2 spell, drops `spell_proc` entries that cannot match any spell, and (when `LoadSpellInfoCorrections` runs) hardcodes ~1,500 manual fix-ups for known-broken DB2 records.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Spells/SpellMgr.cpp` | 5028 | `prefix` |
| `game/Spells/SpellMgr.h` | 827 | `prefix` |
| `game/Spells/TraitMgr.cpp` | 752 | `prefix` |
| `game/Spells/TraitMgr.h` | 87 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Spells/SpellMgr.h` | 827 | Singleton class definition; ~30 typedefs (`SpellChainMap`, `SpellRequiredMap`, `SpellLearnSpellMap`, `SpellTargetPositionMap`, `SpellThreatMap`, `SpellPetAuraMap`, `SpellLinkedMap`, `SpellEnchantProcEventMap`, `SpellAreaMap` + 4 secondary indices, `SkillLineAbilityMap`, `PetLevelupSpellMap`, `PetDefaultSpellsMap`, `SpellTotemModelMap`, `SpellSpellGroupMap`, `SpellGroupSpellMap`, `SpellGroupStackMap`, `SameEffectStackMap`); structs `SpellProcEntry`, `SpellThreatEntry`, `SpellTargetPosition`, `SpellChainNode`, `SpellLearnSkillNode`, `SpellLearnSpellNode`, `SpellArea`, `SpellEnchantProcEntry`, `PetAura`, `PetDefaultSpellsEntry`; enums `ProcFlags`, `ProcFlags2`, `ProcFlagsSpellType`, `ProcFlagsSpellPhase`, `ProcFlagsHit`, `ProcAttributes`, `EnchantProcAttributes`, `SpellGroup`, `SpellGroupStackRule`, `SpellLinkedType`, `SpellAreaFlag`, `EffectRadiusIndex`. Also `SpellInfoLoadHelper` — the staging POD that gathers ~20 sibling DB2 records before constructing one `SpellInfo`. |
| `src/server/game/Spells/SpellMgr.cpp` | 5,028 | All loader bodies; 30 `Load*` methods + lookups + validators. |

**SpellInfoLoadHelper** is the linchpin: it has a slot for every sibling DB2 (`AuraOptions`, `AuraRestrictions`, `CastingRequirements`, `Categories`, `ClassOptions`, `Cooldowns`, `Effects[MAX_SPELL_EFFECTS]`, `EquippedItems`, `Interrupts`, `Labels[]`, `Levels`, `Misc`, `Powers[MAX_POWERS_PER_SPELL]`, `Reagents`, `ReagentsCurrency[]`, `Scaling`, `Shapeshift`, `TargetRestrictions`, `Totems`, `Visuals[]`). `LoadSpellInfoStore` fills these from DB2 stores, falls back across difficulties, then constructs one `SpellInfo` per `(SpellNameEntry, Difficulty)` pair.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SpellMgr` | class (singleton via `SpellMgr::instance()`) | Owns all spell static data; declared in 1 file, never copied/moved |
| `SpellInfoLoadHelper` | struct (POD) | Gathered DB2 entries for one `(spellId, difficulty)`; consumed by `SpellInfo` ctor |
| `SpellProcEntry` | struct | One `spell_proc` SQL row mapped to a runtime proc rule (school mask, family flags, ProcFlags overrides, PpmRate, Chance, Cooldown, Charges) |
| `SpellThreatEntry` | struct | `flatMod`/`pctMod`/`apPctMod` from `spell_threat` |
| `SpellTargetPosition` | struct | Map+x/y/z/o destination keyed by `(spellId, effIndex)`; consumed by `SPELL_EFFECT_TELEPORT_UNITS` with `TARGET_DEST_DB` |
| `SpellChainNode` | struct | `prev/next/first/last` `SpellInfo*` plus `rank: uint8` |
| `SpellLearnSkillNode` | struct | Skill bound to a learning spell (skill, step, value, maxvalue) |
| `SpellLearnSpellNode` | struct | Auto-learned secondary spells; carries `Active` / `AutoLearned` flags |
| `SpellArea` | struct | Conditional aura: `(spellId, areaId, questStart, questEnd, auraSpell, raceMask, gender, …)` plus `flags ∈ {AUTOCAST, AUTOREMOVE, IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE}` |
| `SpellEnchantProcEntry` | struct | Per-enchant proc data (chance, ppm, hit-mask, attr-mask) |
| `PetAura` | class | Owner→pet aura mapping table (per pet entry); supports `petEntry == 0` wildcard |
| `PetDefaultSpellsEntry` | struct | `spellid[MAX_CREATURE_SPELL_DATA_SLOT=4]` for pet default ability set |
| `SpellGroup` / `SpellGroupStackRule` | enum | Cross-spell stacking groups (Elixir Battle/Guardian/Unstable/Shattrath + DB-defined ≥ 1000) and rules (`DEFAULT`, `EXCLUSIVE`, `EXCLUSIVE_FROM_SAME_CASTER`, `EXCLUSIVE_SAME_EFFECT`, `EXCLUSIVE_HIGHEST`) |
| `SpellLinkedType` | enum | `CAST` / `HIT` / `AURA` / `REMOVE` — what trigger A→B does |
| `ProcFlags` / `ProcFlags2` / `ProcFlagsSpellType` / `ProcFlagsSpellPhase` / `ProcFlagsHit` / `ProcAttributes` | enum bitmask | The 6 orthogonal proc-condition dimensions |
| `EnchantProcAttributes` | enum | `WHITE_HIT`, `LIMIT_60` |
| `EffectRadiusIndex` | enum | Hardcoded radius IDs (yards) for sanity-checking `SpellRadius.db2` references |
| `SpellAreaFlag` | enum | Bits 0x1/0x2/0x4 for spell-area autocast/remove behavior |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `SpellMgr::instance()` | Static singleton accessor | — |
| `SpellMgr::IsSpellValid(SpellInfo const*, Player*, bool msg)` | Validates a `SpellInfo` is castable: checks effects are real, item refs exist, summons reference real creatures, learn-spell targets resolve | `ObjectMgr::GetItemTemplate`, `sCreatureStorage` |
| `SpellMgr::GetSpellInfo(spellId, Difficulty)` | The hot lookup: read-only into `mSpellInfoMap` (multi-keyed by `(spellName, difficulty)`) | Hash lookup |
| `SpellMgr::AssertSpellInfo(spellId, Difficulty)` | `GetSpellInfo` + `ASSERT(...)` — caller must guarantee existence | — |
| `SpellMgr::ForEachSpellInfo(callback)` | Iterates every loaded `SpellInfo` | — |
| `SpellMgr::ForEachSpellInfoDifficulty(spellId, callback)` | Iterates all difficulty variants of one spellId | — |
| `SpellMgr::GetSpellChainNode(id)` / `GetFirstSpellInChain` / `GetLastSpellInChain` / `GetNextSpellInChain` / `GetPrevSpellInChain` / `GetSpellRank` / `GetSpellWithRank` | Rank traversal | `mSpellChains` |
| `SpellMgr::GetSpellsRequiredForSpellBounds(id)` / `GetSpellsRequiringSpellBounds(id)` / `IsSpellRequiringSpell(a,b)` | `spell_required` graph queries | `mSpellReq`, `mSpellsReqSpell` |
| `SpellMgr::GetSpellLearnSkill(id)` / `GetSpellLearnSpellMapBounds(id)` / `IsSpellLearnSpell` / `IsSpellLearnToSpell(a,b)` | learn-spell traversal | `mSpellLearnSkills`, `mSpellLearnSpells` |
| `SpellMgr::GetSpellTargetPosition(id, effIdx)` | Lookup for `EffectTeleportUnits` with TARGET_DEST_DB | `mSpellTargetPositions` |
| `SpellMgr::GetSpellSpellGroupMapBounds(id)` / `IsSpellMemberOfSpellGroup(id, group)` / `GetSpellGroupSpellMapBounds(group)` / `GetSetOfSpellsInSpellGroup(...)` | Group membership queries | `mSpellSpellGroup`, `mSpellGroupSpell` |
| `SpellMgr::AddSameEffectStackRuleSpellGroups(...)` / `CheckSpellGroupStackRules(a,b)` / `GetSpellGroupStackRule(group)` | Group stacking enforcement | `mSpellGroupStack`, `mSpellSameEffectStack` |
| `SpellMgr::GetSpellProcEntry(spellInfo)` | Returns proc rule for a spell (or fallback by family) | `mSpellProcMap` |
| `SpellMgr::CanSpellTriggerProcOnEvent(procEntry, eventInfo)` (static) | Evaluates whether a proc rule matches a specific `ProcEventInfo`; the central dispatch in `Unit::ProcSkillsAndAuras` | bit AND across 6 mask dimensions |
| `SpellMgr::GetSpellThreatEntry(spellId)` | Per-spell threat override | `mSpellThreatMap` |
| `SpellMgr::GetSkillLineAbilityMapBounds(spellId)` | Spell↔skill-line-ability lookup (for trainers, learnability) | `mSkillLineAbilityMap` |
| `SpellMgr::GetPetAura(spellId, eff)` | Owner→pet aura lookup | `mSpellPetAuraMap` |
| `SpellMgr::GetSpellEnchantProcEvent(enchId)` / `IsArenaAllowedEnchancment(ench)` | Enchant proc data | `mSpellEnchantProcEventMap`, `mEnchantCustomAttr` |
| `SpellMgr::GetSpellLinked(type, spellId)` | Linked-spell chain (cast/hit/aura/remove) | `mSpellLinkedMap` |
| `SpellMgr::GetPetLevelupSpellList(petFamily)` / `GetPetDefaultSpellsEntry(id)` | Pet learning data | `mPetLevelupSpellMap`, `mPetDefaultSpellsMap` |
| `SpellMgr::GetSpellAreaMapBounds(spellId)` / `GetSpellAreaForQuestMapBounds(questId)` / `GetSpellAreaForQuestEndMapBounds(questId)` / `GetSpellAreaForAuraMapBounds(spellId)` / `GetSpellAreaForAreaMapBounds(areaId)` | The 5 secondary indices for `mSpellAreaMap` | All five maps populated by `LoadSpellAreas` |
| `SpellMgr::GetModelForTotem(spellId, race)` | `(spellId, race)` → totem display id | `mSpellTotemModel` |
| **Loaders** (called once at startup, in fixed order — see §11): | | |
| `SpellMgr::LoadSpellInfoStore` | Reads ~20 sibling DB2 stores, runs cross-difficulty fallback, constructs `SpellInfo` per (spellName, difficulty) | DB2 stores listed in §6 |
| `SpellMgr::LoadSpellInfoServerside` | SQL `serverside_spell` + `serverside_spell_effect` — **server-only** spells that don't exist client-side (not the same as `spell_dbc` overlay). Represented row/effect staging exists; live `SpellMgr` insertion is pending. | `WorldDatabase.Query`, `mServersideSpellNames` |
| `SpellMgr::LoadSpellInfoCorrections` | ~1,500 hand-coded `ApplySpellFix({…ids…}, [](SpellInfo* spellInfo){ spellInfo->… = …; })` lambdas; the **single largest function** in the file (~1,540 lines) — patches known-broken DB2 attributes/effects/etc. for individual spells |
| `SpellMgr::LoadSkillLineAbilityMap` | DB2 SkillLineAbility → multimap by spellId | `sSkillLineAbilityStore` |
| `SpellMgr::LoadSpellInfoCustomAttributes` | Reads `spell_custom_attr` SQL overrides, then computes `AttributesCu` bits from analysis of effects/auras/talents/visuals/liquids | `WorldDatabase.Query`, DB2 stores |
| `SpellMgr::LoadSpellInfoSpellSpecificAndAuraState` | Calls `_LoadSpellSpecific` + `_LoadAuraState` on every loaded `SpellInfo` | `SpellInfo::_LoadSpellSpecific` |
| `SpellMgr::LoadSpellInfoDiminishing` | Calls `_LoadSpellDiminishInfo` per spell | `SpellInfo::_LoadSpellDiminishInfo` |
| `SpellMgr::LoadSpellInfoImmunities` | Calls `_LoadImmunityInfo` per effect | `SpellEffectInfo::_LoadImmunityInfo` |
| `SpellMgr::LoadSpellRanks` | DB2 `SkillLineAbilityEntry::SupercedesSpell` → `mSpellChains` (also patches `SpellInfo::ChainEntry`) | `sSkillLineAbilityStore`, `mSpellInfoMap` |
| `SpellMgr::LoadSpellRequired` | `spell_required` SQL → `mSpellReq` + reverse `mSpellsReqSpell` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellLearnSkills` | derived from `mSpellInfoMap` effects → `mSpellLearnSkills` | `SPELL_EFFECT_SKILL`, `SPELL_EFFECT_DUAL_WIELD` |
| `SpellMgr::LoadSpellLearnSpells` | DB2 `SpellLearnSpell` + SQL `spell_learn_spell` (overrides) → `mSpellLearnSpells` | `WorldDatabase.Query`, `sSpellLearnSpellStore` |
| `SpellMgr::LoadSpellTargetPositions` | `spell_target_position` SQL → `mSpellTargetPositions` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellGroups` | `spell_group` SQL → `mSpellSpellGroup` + `mSpellGroupSpell` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellGroupStackRules` | `spell_group_stack_rules` SQL → `mSpellGroupStack` + computed `mSpellSameEffectStack` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellProcs` | `spell_proc` SQL → `mSpellProcMap`; supports negative `spellId` to apply rule to whole rank chain | `WorldDatabase.Query`, `GetFirstSpellInChain` |
| `SpellMgr::LoadSpellThreats` | `spell_threat` SQL → `mSpellThreatMap` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellPetAuras` | `spell_pet_auras` SQL → `mSpellPetAuraMap` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellEnchantProcData` | `spell_enchant_proc_data` SQL → `mSpellEnchantProcEventMap` | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellLinked` | `spell_linked_spell` SQL → `mSpellLinkedMap` keyed by `(SpellLinkedType, spellId)` | `WorldDatabase.Query` |
| `SpellMgr::LoadPetLevelupSpellMap` | DB2 `CreatureFamily` + `SkillLineAbility` join → `mPetLevelupSpellMap`; represented by `PetLevelupSpellStoreLikeCpp` | `sCreatureFamilyStore`, `sSkillLineAbilityStore` |
| `SpellMgr::LoadPetDefaultSpells` | `SpellInfo` summon effects + cached creature templates → `mPetDefaultSpellsMap`; represented by `PetDefaultSpellStoreLikeCpp` | `mSpellInfoMap`, cached creature templates |
| `SpellMgr::LoadSpellAreas` | `spell_area` SQL → `mSpellAreaMap` + 4 secondary indices (`mSpellAreaForQuestMap`, `mSpellAreaForQuestEndMap`, `mSpellAreaForAuraMap`, `mSpellAreaForAreaMap`) | `WorldDatabase.Query` |
| `SpellMgr::LoadPetFamilySpellsStore` | DB2 `SkillLineAbility` + `CreatureFamily` + `SpellLevels` → static `sPetFamilySpellsStore`; represented by `PetFamilySpellStoreLikeCpp` | `sCreatureFamilyStore`, `sSkillLineAbilityStore`, `sSpellLevelsStore`, `mSpellInfoMap` |
| `SpellMgr::LoadSpellTotemModel` | `spell_totem_model` SQL → `mSpellTotemModel` | `WorldDatabase.Query` |

---

## 5. Module dependencies

**Depends on:**
- `wow-data` / `shared-datastores.md` — every DB2 store consumed by `LoadSpellInfoStore` (~20 sibling stores enumerated in §6)
- `wow-database` — `WorldDatabase` connection for ~14 SQL `spell_*` tables; **none** are prepared statements, all are bulk `Query()` calls executed once at boot
- `Database/HotfixDatabase` (transitively) — DB2 readers themselves are overlaid with `hotfixes` schema rows; `SpellMgr` doesn't read hotfixes directly, the DB2 store does
- `wow-spell::SpellInfo` — constructs every `SpellInfo` instance via the `SpellInfoLoadHelper` POD
- `wow-core::Difficulty` enum — keys `mSpellInfoMap` by `(spellId, difficulty)`
- `Conditions` (`ConditionMgr`) — for `SpellArea` `IsFitToRequirements` quest-status checks
- `SharedDefines.h` — `SpellSchool`, `Powers`, `Mechanics`, `Targets`, `SpellEffectName`, `AuraType`
- `wow-logging` — bulk loaders log via `TC_LOG_INFO("server.loading", …)` on success and `TC_LOG_ERROR("sql.sql", …)` on validation failure
- `flag128` (4×u32) — for `SpellFamilyMask` keys in proc and linked tables

**Depended on by:**
- `wow-spell::Spell` (entry points: `GetSpellInfo`, `IsSpellValid`)
- `wow-spell::Aura` (proc dispatch via `GetSpellProcEntry`, stacking via `CheckSpellGroupStackRules`)
- `wow-spell::SpellHistory` (chain queries for cooldown sharing)
- `wow-world::handlers::*` — every handler that touches a spell ID (cast, learn, trainer, dispel, …)
- `wow-combat::ThreatManager` — `GetSpellThreatEntry` overrides on each spell-caused threat increment
- `wow-pet::Pet` — `GetPetAura`, `GetPetLevelupSpellList`, `GetPetDefaultSpellsEntry`
- `wow-quest::QuestMgr` — `GetSpellAreaForQuestMapBounds` for quest-tied spells
- `Player` (entity) — learn/unlearn paths (`GetSpellLearnSpellMapBounds`, `GetSkillLineAbilityMapBounds`)

---

## 6. SQL / DB queries (if any)

All loads are issued via `WorldDatabase.Query("SELECT … FROM …")` — **no prepared statements** (these are one-shot startup loads, not hot path). Persistence of cooldowns/auras lives in `Player` save code in the `character` DB; `SpellMgr` itself never writes.

| Statement / Source | Purpose | DB | Loader |
|---|---|---|---|
| DB2 `SkillLineAbilityEntry::SupercedesSpell` → builds `mSpellChains` | Rank chains derived from skill-line supercession links; the current C++ legacy has no `spell_ranks` SQL load in `LoadSpellRanks` | DB2 | `LoadSpellRanks` |
| `SELECT spell_id, req_spell from spell_required` | Pre-req graph; rejects rows where both ids are in same chain | world | `LoadSpellRequired` |
| `SELECT entry, SpellID, Active FROM spell_learn_spell` | Override for the DB2 `SpellLearnSpell` join | world | `LoadSpellLearnSpells` |
| `SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position` | TeleportUnits destinations; rejects effects whose `Target` isn't `TARGET_DEST_DB` | world | `LoadSpellTargetPositions` |
| `SELECT id, spell_id FROM spell_group` | Group membership; both forward + reverse map | world | `LoadSpellGroups` |
| `SELECT group_id, stack_rule FROM spell_group_stack_rules` | Per-group stacking semantics | world | `LoadSpellGroupStackRules` |
| `SELECT SpellId, SchoolMask, SpellFamilyName, SpellFamilyMask0..3, ProcFlags, ProcFlags2, SpellTypeMask, SpellPhaseMask, HitMask, AttributesMask, DisableEffectsMask, ProcsPerMinute, Chance, Cooldown, Charges FROM spell_proc` | Override / fully-define a spell's proc rule; supports negative SpellId to apply to whole rank chain | world | `LoadSpellProcs` |
| `SELECT entry, flatMod, pctMod, apPctMod FROM spell_threat` | Per-spell threat multiplier override | world | `LoadSpellThreats` |
| `SELECT spell, effectId, pet, aura FROM spell_pet_auras` | Owner aura → pet aura mapping; `removeOnChangePet`/`damage` come from the source spell effect | world | `LoadSpellPetAuras` |
| `SELECT EnchantID, Chance, ProcsPerMinute, HitMask, AttributesMask FROM spell_enchant_proc_data` | Item-enchant proc tuning | world | `LoadSpellEnchantProcData` |
| `SELECT spell_trigger, spell_effect, type FROM spell_linked_spell` | A→B chains by `SpellLinkedType` (CAST/HIT/AURA/REMOVE) | world | `LoadSpellLinked` |
| `SELECT spell, area, quest_start, quest_start_status, quest_end_status, quest_end, aura_spell, racemask, gender, flags FROM spell_area` | Area-conditional auras | world | `LoadSpellAreas` |
| `SELECT SpellID, RaceID, DisplayID from spell_totem_model` | Per-race totem display override | world | `LoadSpellTotemModel` |
| `SELECT entry, attributes FROM spell_custom_attr` | Override `AttributesCu` bits (rare) | world | `LoadSpellInfoCustomAttributes` |
| `SELECT … FROM serverside_spell` | Server-only spells (no client DB2 row) — **this is the modern equivalent of legacy `spell_dbc`**; the user-task brief refers to this overlay | world | `LoadSpellInfoServerside` |
| `SELECT SpellID, EffectIndex, DifficultyID, Effect, EffectAura, …(33 columns)… FROM serverside_spell_effect` | Effect rows for serverside spells; refuses to overlay a real DB2 spell | world | `LoadSpellInfoServerside` |

> **Note on `spell_dbc`:** the user-task brief mentions an SQL `spell_dbc` table acting as a DB2 overlay. In TrinityCore Wrath Classic 3.4.3.54261 the literal `spell_dbc` table name no longer exists — it was renamed and split into `serverside_spell` + `serverside_spell_effect` (handled by `LoadSpellInfoServerside`) for **server-only** spells, and DB2 overlay of **existing** spells is now done by the **hotfixes DB** via `DB2DatabaseLoader::LoadFromDB` (see `shared-datastores.md`), not by a SpellMgr SQL load. The same conceptual role is preserved across the two paths.

**DB2 stores consumed by `LoadSpellInfoStore` (cross-table join):**

| Store | Purpose in the join | Difficulty-keyed? |
|---|---|---|
| `sSpellNameStore` | One row per spellId; the join's primary key. Provides localized `Name` | no (single row per id) |
| `sSpellEffectStore` | Up to `MAX_SPELL_EFFECTS` (32) rows per (spellId, difficulty); base of `SpellEffectInfo[]` | yes |
| `sSpellMiscStore` | The 14-element `Attributes[]` array (= `SpellAttr0..AttributesEx14`), `CastingTimeIndex`, `DurationIndex`, `RangeIndex`, `Speed`, `LaunchDelay`, `SchoolMask`, `SpellIconFileDataID`, `ContentTuningID` | yes |
| `sSpellAuraOptionsStore` | `ProcTypeMask`, `ProcChance`, `ProcCharges`, `ProcCategoryRecovery`, `CumulativeAura` (= StackAmount), `SpellProcsPerMinuteID` | yes |
| `sSpellAuraRestrictionsStore` | `CasterAuraState`, `TargetAuraState`, `Exclude*AuraState`, `*AuraSpell` | yes |
| `sSpellCastingRequirementsStore` | `RequiresSpellFocus`, `FacingCasterFlags`, `RequiredAreasID` | no |
| `sSpellCategoriesStore` | `Category`, `DispelType`, `Mechanic`, `StartRecoveryCategory`, `DefenseType` (=DmgClass), `PreventionType`, `ChargeCategory` | yes |
| `sSpellClassOptionsStore` | `SpellClassSet` (=SpellFamilyName), `SpellClassMask` (flag128) | no |
| `sSpellCooldownsStore` | `RecoveryTime`, `CategoryRecoveryTime`, `StartRecoveryTime` | yes |
| `sSpellEquippedItemsStore` | `EquippedItemClass`, `EquippedItemSubclass`, `EquippedItemInvTypes` | no |
| `sSpellInterruptsStore` | `InterruptFlags`, `AuraInterruptFlags[2]`, `ChannelInterruptFlags[2]` | yes |
| `sSpellLabelStore` | Multi-row `LabelID[]` set | no |
| `sSpellLevelsStore` | `MaxLevel`, `BaseLevel`, `SpellLevel` | yes |
| `sSpellPowerStore` | Up to `MAX_POWERS_PER_SPELL` rows; ordered by `OrderIndex` (or `SpellPowerDifficulty.OrderIndex`) | yes (via `SpellPowerDifficulty`) |
| `sSpellReagentsStore` | `Reagent[8]`, `ReagentCount[8]` | no |
| `sSpellReagentsCurrencyStore` | Multi-row currency reagents (DF+, mostly empty in 3.4.3) | no |
| `sSpellScalingStore` | `MinScalingLevel`, `MaxScalingLevel`, `ScalesFromItemLevel` | no |
| `sSpellShapeshiftStore` | `ShapeshiftMask[2]`, `ShapeshiftExclude[2]` | no |
| `sSpellTargetRestrictionsStore` | `ConeDegrees`, `Width`, `Targets`, `TargetCreatureType`, `MaxTargets` (= MaxAffectedTargets), `MaxTargetLevel` | yes |
| `sSpellTotemsStore` | `RequiredTotemCategoryID[2]`, `Totem[2]` | no |
| `sSpellXSpellVisualStore` | Sorted-by-`CasterPlayerConditionID` visual list | yes |
| `sSpellEffectScalingStore` | (read by `SpellEffectInfo` ctor, not directly by `SpellMgr`) | yes |
| `sSpellRangeStore` / `sSpellRadiusStore` / `sSpellCastTimesStore` / `sSpellDurationStore` | Pointer references stored on `SpellInfo` (lazy lookup through these stores) | no |
| `sSpellProcsPerMinuteStore` + `sSpellProcsPerMinuteModStore` | PpmRate base + per-condition multipliers | no |
| `sSpellPowerDifficultyStore` | Re-keys `SpellPower` rows to `(spellId, difficulty, orderIndex)` | yes |
| `sSpellLearnSpellStore` | DB2 spell-teaches-spell graph (overlaid by `spell_learn_spell` SQL) | represented DB2 store + represented `LoadSpellLearnSpells` merge store; not wired live |
| `sSkillLineAbilityStore` | Read by `LoadSkillLineAbilityMap` for trainer/learn paths; represented by `SkillStore::get_skill_line_ability_map_bounds_like_cpp` | no |
| `sBattlePetSpeciesStore` + `sSummonPropertiesStore` | Cross-walk for `SPELL_EFFECT_SUMMON` rows that point at a battle pet | no |
| `sDifficultyStore` | Drives `FallbackDifficultyID` chain when a (spellId, difficulty) is missing slots | no |

---

## 7. Wire-protocol packets (if any)

`SpellMgr` is a server-internal loader and **emits no opcodes**. The data it loads is consumed by:
- `SMSG_AURA_UPDATE` / `SMSG_PERIODIC_AURA_LOG` (via `Aura` reading `SpellInfo`)
- `SMSG_SPELL_GO` / `SMSG_SPELL_START` / `SMSG_CAST_FAILED` (via `Spell` reading `SpellInfo`)
- `SMSG_SPELL_COOLDOWN` / `SMSG_COOLDOWN_EVENT` (via `SpellHistory` reading chain/category info)
- `SMSG_LEARNED_SPELL` / `SMSG_SUPERCEDED_SPELL` (via `Player` reading learn-spell + chain data)

DB2 overlay rows can also generate `SMSG_DB_REPLY` to clients via `DB2Storage<T>::WriteRecord`, but that is `shared-datastores.md`'s territory.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-spell/src/spell_mgr.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 lines (empty file)** — verified `wc -l = 0`. The crate is registered in the workspace but has no code.
- `crates/wow-data/src/spell.rs` — exists; provides a minimal `SpellInfo`-shaped POD that exposes `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, `has_cast_time`. **Not the SpellInfo type tracked here** — see `spells-info.md`.
- `crates/wow-data/src/wdc4.rs` — generic WDC4 reader (`Wdc4Reader`); shared with all other DB2-backed crates.
- `crates/wow-data/src/hotfix_cache.rs` — `HotfixBlobCache` blob layer (DB-driven hotfixes from the `hotfixes` schema).

**What's implemented:**
- DB2 binary parsing for ~5 of ~325 stores (achievement, area_trigger, item, item_stats, player_stats, quest, quest_xp, skill, spell — partial). The spell DB2 reader **only reads `Spell.db2` itself**, not the 19 other sibling stores that `LoadSpellInfoStore` joins. There is no `SpellInfoLoadHelper` analog.
- Generic hotfix overlay framework exists at the WDC4 reader level.

**What's missing vs C++:**
1. The `SpellMgr` singleton itself — type, instance, public API, and all 30 loader methods.
2. The `SpellInfoLoadHelper` POD and the cross-table join logic in `LoadSpellInfoStore` (the difficulty-fallback walk and per-store merge).
3. Several SQL/DB2 spell loaders are still absent or only represented. `LoadSpellTargetPositions` is represented and wired at world startup (`WorldStatements::SEL_SPELL_TARGET_POSITION`, `SpellTargetPositionStoreLikeCpp`). The rank-chain representation for `LoadSpellRanks` is also built at world startup from `SkillLineAbilityEntry::SupercedesSpell` (`SpellChainStoreLikeCpp`), but it is not yet owned by a live `SpellMgr` or patched into authoritative `SpellInfo::ChainEntry` nodes. `LoadSpellProcs` now builds `SpellProcStoreLikeCpp` at world startup from SQL rows plus represented implicit proc generation using the loaded spell/class/aura/misc/PPM stores and injects it into sessions, but it is not yet owned by a live `SpellMgr` and is not consumed by runtime aura/proc dispatch. `LoadSpellRequired`, `LoadSpellLearnSpells`, `LoadSpellGroups`, `LoadSpellGroupStackRules`, `LoadSkillLineAbilityMap`, `LoadPetLevelupSpellMap`, `LoadPetDefaultSpells`, `LoadPetFamilySpellsStore`, `LoadSpellAreas`, `LoadSpellInfoCustomAttributes` SQL overrides, `LoadSpellThreats`, `LoadSpellPetAuras`, `LoadSpellEnchantProcData`, `LoadSpellLinked`, `LoadSpellTotemModel`, and `LoadSpellInfoServerside` row/effect staging have represented query/store coverage but are not wired into a live `SpellMgr` startup path yet. Live `SpellInfo` ownership/insertion and the derived `AttributesCu` computation pass remain missing.
4. The remaining `LoadSpellInfoServerside` work: mapping represented rows into the future authoritative `SpellInfo` / `mSpellInfoMap` startup path.
5. The `LoadSpellInfoCorrections` ~1,500-entry hand-coded fix-up table (the largest single function in the C++ file at ~1,540 lines).
6. The 17 secondary lookup maps and their accessor APIs (`mSpellChains`, `mSpellReq`, `mSpellsReqSpell`, `mSpellLearnSkills`, `mSpellLearnSpells`, `mSpellTargetPositions`, `mSpellSpellGroup`, `mSpellGroupSpell`, `mSpellGroupStack`, `mSpellSameEffectStack`, `mSpellThreatMap`, `mSpellPetAuraMap`, `mSpellLinkedMap`, `mSpellEnchantProcEventMap`, `mSpellAreaMap` + 4 secondary indices, `mSkillLineAbilityMap`, `mPetLevelupSpellMap`, `mPetDefaultSpellsMap`, `mSpellTotemModel`, `mSpellDifficultySearcherMap`).
7. `IsSpellValid`, `CanSpellTriggerProcOnEvent`, `CheckSpellGroupStackRules`.
8. The bridge into `SpellInfo` mutators that loaders rely on (`SpellInfo::ChainEntry` patched by `LoadSpellRanks`; `_LoadSpellSpecific`/`_LoadAuraState`/`_LoadSpellDiminishInfo`/`_LoadImmunityInfo` driven by per-spell `LoadSpellInfo*` post-processors).

**Suspicious / likely divergent (hipótesis pre-implementación):**
- The eventual Rust `SpellMgr` should be `Arc<SpellMgr>` with a single OnceCell-backed instance — **not** a global mutable singleton mirror of the C++ pattern. All loader methods become async (`async fn load_spell_info_store(&self, ...)` consuming `&Pool<MariaDb>`).
- Difficulty-fallback in `LoadSpellInfoStore` walks `DifficultyEntry::FallbackDifficultyID` to a fixed point — easy to get wrong; needs a regression test fixture against the canonical (spellId, difficulty) tuples C++ produces for known multi-difficulty spells (heroic dungeons, raid normal/heroic/mythic).
- `LoadSpellInfoCorrections` cannot be ported as a single 1,540-line function in idiomatic Rust; it should be split into a `corrections/` module with one file per group of fixes (per-class, per-encounter, generic), each registering with `inventory::submit!` patterns.
- `flag128` (4×u32) keys in `mSpellLinkedMap` and `SpellProcEntry::SpellFamilyMask` need a Rust `Flag128` newtype with `Hash + Eq` derives.

**Tests existing:** **0** in `crates/wow-spell/`. None in `crates/wow-data/src/spell.rs` covering DB2 multi-store join.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SPELLS_MGR.WBS.001** Partir y cerrar la migracion auditada de `game/Spells/SpellMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 5028 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_MGR.WBS.002** Partir y cerrar la migracion auditada de `game/Spells/SpellMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 827 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_MGR.WBS.003** Partir y cerrar la migracion auditada de `game/Spells/TraitMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 752 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_MGR.WBS.004** Cerrar la migracion auditada de `game/Spells/TraitMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for `MIGRATION_ROADMAP.md` cross-reference. Complexity: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#SPELLMGR.1** Scaffold `crates/wow-spell/src/spell_mgr.rs` with the `SpellMgr` struct, `OnceCell`-backed accessor, and a `SpellMgrBuilder` for staged loads (M)
- [ ] **#SPELLMGR.2** Define `SpellInfoLoadHelper` POD with all ~20 sibling-DB2 slot fields + `Effects: [Option<&SpellEffectEntry>; MAX_SPELL_EFFECTS]` (M)
- [ ] **#SPELLMGR.3** Implement `LoadSpellInfoStore` async: walk all 20+ sibling DB2 stores, build `(spellId, difficulty) → SpellInfoLoadHelper` HashMap, run difficulty-fallback walk, construct `SpellInfo` per pair (XL — depends on #SPELLINFO.1 + ~20 DB2 readers in `wow-data`)
- [ ] **#SPELLMGR.4** Implement `LoadSpellInfoServerside` (`serverside_spell` + `serverside_spell_effect` SQL overlay; refuses to overlay an existing real DB2 spell): represented queries exist (`WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT`, `SEL_SERVERSIDE_SPELL`) and represented stores exist (`ServersideSpellEffectStoreLikeCpp`, `ServersideSpellStoreLikeCpp`), including C++ regular-spell rejection, effect-row difficulty/effect/aura/target validation, grouping by `(spellId, difficulty)`, C++-faithful invalid-radius warning without skip, `serverside_spell` 81-column field preservation, `mServersideSpellNames` shape, and C++-faithful lack of main-row difficulty validation. Still pending: live `mSpellInfoMap.emplace(...)`, authoritative `SpellInfo` ownership, and startup wiring. (H)
- [ ] **#SPELLMGR.5** Define `mSpellInfoMap: DashMap<(u32, Difficulty), Arc<SpellInfo>>` + `get_spell_info(spell_id, difficulty)` + `assert_spell_info` + `for_each_spell_info` + `for_each_spell_info_difficulty` (M)
- [ ] **#SPELLMGR.6** Implement `LoadSpellRanks` (DB2 `SkillLineAbilityEntry::SupercedesSpell`): represented `wow-data` store/accessors exist (`SpellChainStoreLikeCpp`) and are now built at world startup from skill-line supercession links, skipping missing spell infos like C++. Still pending: live `SpellMgr` ownership and patching real `SpellInfo::chain_entry` for every difficulty variant. (M)
- [ ] **#SPELLMGR.7** Implement chain accessors: represented `wow-data` accessors exist (`first/last/next/prev/rank/spell_with_rank`), but live `SpellMgr` APIs backed by authoritative `SpellInfo` nodes are still pending. (L)
- [ ] **#SPELLMGR.8** Implement `LoadSpellRequired` (`spell_required` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_REQUIRED`, `SpellRequiredStoreLikeCpp`), including C++ spell/required-spell validation, same-rank-chain rejection through the real startup rank-chain store, duplicate exact-pair skip, and forward+reverse multimaps. World-server startup now builds the store and injects it into sessions. Runtime spell-learning/cast-check consumption is still pending. (M)
- [ ] **#SPELLMGR.9** Implement `LoadSpellLearnSkills`: represented `wow-data` store/loader exists (`SpellLearnSkillStoreLikeCpp`) and mirrors C++ derivation from `mSpellInfoMap` effects (`SPELL_EFFECT_SKILL`, `SPELL_EFFECT_DUAL_WIELD`), first qualifying effect per spell, `DIFFICULTY_NONE` filter, `CalcValue()` seam for skill step, and `GetSpellLearnSkill` accessor. Live startup wiring against authoritative `SpellInfo` is still pending. (L)
- [ ] **#SPELLMGR.10** Implement `LoadSpellLearnSpells` (DB2 `SpellLearnSpell` + SQL `spell_learn_spell` override): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_LEARN_SPELL`, `SpellLearnSpellStoreLikeCpp`), including C++ SQL validation, SQL-table-empty early return, SpellInfo-effect auto-learn pass, SpellLearnSpell.db2 pass, redundant-SQL warnings, `Active`/`AutoLearned` flags and map accessors. Live startup wiring against the authoritative `SpellInfo` cache and runtime learn/unlearn cascade consumption are still pending. (M)
- [ ] **#SPELLMGR.11** Implement `LoadSpellTargetPositions` (`spell_target_position` SQL): represented query/store exists and is wired at world startup (`WorldStatements::SEL_SPELL_TARGET_POSITION`, `SpellTargetPositionStoreLikeCpp`), keyed by `(spell_id, eff_index)`, with C++ map/spell/effect/zero-position validation, nullable orientation fallback, and `TARGET_DEST_DB`/`TARGET_DEST_NEARBY_ENTRY_OR_DB` target validation. Full `SpellMgr` ownership and all runtime consumers remain bounded. (L)
- [ ] **#SPELLMGR.12** Implement `LoadSpellGroups` (`spell_group` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_GROUP`, `SpellGroupStoreLikeCpp`), including C++ core-range rejection, negative nested group references, missing/non-first-rank spell skips, recursive `GetSetOfSpellsInSpellGroup`, first-rank normalization for spell→group lookups, and forward/reverse maps. World-server startup now builds the store and injects it into sessions. Stack-rule/aura runtime consumption is still pending. (M)
- [ ] **#SPELLMGR.13** Implement `LoadSpellGroupStackRules` (`spell_group_stack_rules` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_GROUP_STACK_RULES`, `SpellGroupStackRuleStoreLikeCpp`), including C++ stack-rule validation, missing-group skip, `mSpellGroupStack`, computed `mSpellSameEffectStack`, the hardcoded haste same-effect subgroup, rank-chain recheck, `GetSpellGroupStackRule`, and represented `CheckSpellGroupStackRules`. World-server startup now builds the store from the live `SpellGroupStoreLikeCpp` and injects it into sessions. Aura runtime consumption is still pending. (M)
- [ ] **#SPELLMGR.14** Implement `LoadSpellProcs` (`spell_proc` SQL): represented query/store for explicit SQL rows exists (`WorldStatements::SEL_SPELL_PROC`, `SpellProcStoreLikeCpp`), including 18-column query, `SpellProcEntry`, negative `SpellId` rank-chain expansion, defaults from `SpellInfo`, duplicate handling, primary mask/value validation, represented/session `GetSpellProcEntry` difficulty-fallback lookup, the C++ implicit-proc aura classification table (`isTriggerAura`/`isAlwaysTriggeredAura`/`spellTypeMask`), pure represented implicit proc-entry generation from trigger auras, represented SQL-then-implicit store loading from either implicit sources or `SpellInfo` sources where SQL entries suppress generated defaults, a C++-like adapter from loaded `SpellStore` + DB2 spell stores into proc source `SpellInfo`, and preservation of `SpellEffectInfo::SpellClassMask` plus `DieSides`-aware `CalcValue()` for implicit proc source conversion. World-server startup now builds `SpellProcStoreLikeCpp` from `WorldStatements::SEL_SPELL_PROC` plus the loaded spell/class/aura/misc/PPM/rank-chain stores. Still pending: live `SpellMgr` ownership, full authoritative multi-difficulty `SpellInfo` coverage, and runtime `ProcEventInfo` dispatch. (M)
- [ ] **#SPELLMGR.15** Define `SpellProcEntry` with `flag128 SpellFamilyMask` + `ProcFlagsInit ProcFlags` + 6 mask dimensions (M)
- [x] **#SPELLMGR.16** Represent `CanSpellTriggerProcOnEvent` static dispatcher as
  `wow_data::spell::can_spell_trigger_proc_on_event_like_cpp` (central proc-condition AND
  across ProcFlags, AttributesMask, SchoolMask, SpellFamilyName/Mask, SpellTypeMask,
  SpellPhaseMask and HitMask). C++-contrasted against `SpellMgr.cpp:511-585` and
  `SpellInfo.cpp:1770-1783`; live `Unit::ProcSkillsAndAuras`/aura trigger runtime wiring is
  still pending. (M)
- [ ] **#SPELLMGR.17** Implement `LoadSpellThreats` (`spell_threat` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_THREATS`, `SpellThreatStoreLikeCpp`), including C++ skip for missing spells, duplicate overwrite semantics, and `GetSpellThreatEntry` fallback to first spell in rank chain via callback/session accessor. World-server startup now builds the store and injects it into sessions. Still pending: live `SpellMgr` ownership and spell/threat-runtime consumption. (L)
- [ ] **#SPELLMGR.18** Implement `LoadSkillLineAbilityMap` (DB2): represented index/accessor exists (`SkillStore::get_skill_line_ability_map_bounds_like_cpp`), preserving C++ `mSkillLineAbilityMap` grouping by `SkillLineAbilityEntry::Spell`, insertion order and duplicate multimap rows. Still pending: live `SpellMgr` startup ownership/API wiring. (L)
- [ ] **#SPELLMGR.19** Implement `LoadSpellPetAuras` (`spell_pet_auras` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_PET_AURAS`, `SpellPetAuraStoreLikeCpp`), including C++ key `(spell << 8) + eff`, dummy-effect validation, `petEntry==0` wildcard, and duplicate-key `AddAura` semantics; live startup wiring against the authoritative `SpellInfo` cache is still pending. Before wiring, preserve C++ `SpellEffectInfo::CalcValue()` semantics for the source effect `damage` field, not just raw `EffectBasePoints`. (M)
- [ ] **#SPELLMGR.20** Implement `LoadSpellEnchantProcData` (`spell_enchant_proc_data` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_ENCHANT_PROC_DATA`, `SpellEnchantProcStoreLikeCpp`), including C++ skip for missing `SpellItemEnchantment.db2` rows and duplicate overwrite semantics. World-server startup now builds it against the live `SpellItemEnchantmentStore` and injects it into sessions. Enchant proc runtime consumption is still pending. (L)
- [ ] **#SPELLMGR.21** Implement `LoadSpellLinked` (`spell_linked_spell` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_LINKED`, `SpellLinkedStoreLikeCpp`), including C++ trigger/effect existence checks, signed effect preservation, negative-trigger coercion to REMOVE, invalid-type/self-loop skips, vector push order, and same-base-point warning as non-fatal; live startup wiring and cast/hit/aura/remove runtime consumption are still pending (M)
- [ ] **#SPELLMGR.22** Implement `LoadPetLevelupSpellMap` (DB2 join `CreatureFamily` × `SkillLineAbility`): represented `PetLevelupSpellStoreLikeCpp` exists in `wow-data`, keyed by petFamily and preserving C++ `PetLevelupSpellSet` multimap order by `SpellLevel`, with C++ filters for `AcquireMethod == LEARNED_ON_SKILL_LEARN`, missing `SpellInfo`, and `SpellLevel == 0`. Still pending: live `SpellMgr` startup ownership/API wiring. (M)
- [ ] **#SPELLMGR.23** Implement `LoadPetDefaultSpells`: represented `PetDefaultSpellStoreLikeCpp` exists in `wow-data`, scanning `DIFFICULTY_NONE` summon/summon-pet effects, looking up cached creature templates, copying C++ `PetDefaultSpellsEntry { spellid: [u32; 4] }`, removing spells already covered by `mPetLevelupSpellMap`, and skipping empty results. Still pending: live `SpellMgr` startup ownership/API wiring against authoritative `SpellInfo` and `ObjectMgr` templates. (M)
- [ ] **#SPELLMGR.24** Implement `LoadSpellAreas` (`spell_area` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_AREA`, `SpellAreaStoreLikeCpp`), including C++ row validation, duplicate-similar-requirement skip, area/quest/aura/race/gender checks, autocast-aura-chain rejection, primary map and four secondary indices (`for_quest_start_or_end`, `for_quest_end`, `for_aura`, `for_area`). Still pending: live `SpellMgr` wiring, `SpellInfo` autocast attribute mutation, `SpellArea::IsFitToRequirements` runtime with Player/BG/Battlefield checks, and aura apply/remove integration. (H)
- [ ] **#SPELLMGR.25** Implement `LoadPetFamilySpellsStore`: represented `PetFamilySpellStoreLikeCpp` exists in `wow-data`, preserving C++ `std::map<uint32, std::set<uint32>>` shape and filters (`GetSpellInfo(..., DIFFICULTY_NONE)`, `SpellLevels.DifficultyID == 0 && SpellLevel != 0` skip, `SpellInfo::IsPassive`, creature-family skill-line match, `AcquireMethod == LEARNED_ON_SKILL_LEARN`). Still pending: live `SpellMgr` startup ownership/API wiring and pet passive learning consumption. (L)
- [ ] **#SPELLMGR.26** Implement `LoadSpellTotemModel` (`spell_totem_model` SQL): query + represented `wow-data` store/loader exist (`WorldStatements::SEL_SPELL_TOTEM_MODEL`, `SpellTotemModelStoreLikeCpp`), including C++ spell/race/display validation, duplicate overwrite semantics, and `GetModelForTotem` missing→0; live startup wiring and totem summon display consumption are still pending (L)
- [ ] **#SPELLMGR.27** Implement `LoadSpellInfoCustomAttributes`: query + represented SQL override store exists (`WorldStatements::SEL_SPELL_CUSTOM_ATTR`, `SpellCustomAttributeStoreLikeCpp`), including C++ missing-spell skip, per-difficulty variant application, OR accumulation, and `SPELL_ATTR0_CU_SHARE_DAMAGE` requiring `SPELL_EFFECT_SCHOOL_DAMAGE`. Still pending: live `SpellMgr` wiring and the large derived `AttributesCu` computation pass from effect/aura analysis, enchant procs, talents, visuals, liquids and post-pass binary/crit fixes. (H)
- [ ] **#SPELLMGR.28** Implement `LoadSpellInfoSpellSpecificAndAuraState` (per-spell post-processor: dispatches into `SpellInfo::_load_spell_specific` and `_load_aura_state`) (M)
- [ ] **#SPELLMGR.29** Implement `LoadSpellInfoDiminishing` (per-spell `_load_spell_diminish_info`) (M)
- [ ] **#SPELLMGR.30** Implement `LoadSpellInfoImmunities` (per-effect `_load_immunity_info`) (M)
- [ ] **#SPELLMGR.31** Implement `LoadSpellInfoCorrections` skeleton: split per-class / per-encounter / generic into `crates/wow-spell/src/corrections/`; register fixes via `inventory::submit!`. Initial port covers ~50 highest-impact fixes; rest as ongoing work (XL — splittable per fix-group)
- [ ] **#SPELLMGR.32** Implement `IsSpellValid(&SpellInfo, Option<&Player>) -> Result<(), ValidationError>` matching C++ checks (item refs, summon refs, learn-spell refs) (M)
- [ ] **#SPELLMGR.33** Define startup load-order driver in `world-server` matching C++ `World.cpp:1861-2148` exactly (L — but easy to get wrong; see §11)
- [ ] **#SPELLMGR.34** Add `SpellMgr` benchmark: full boot load on a populated `world` DB; target ≤3s on dev hardware (matches C++ baseline ~1-2s) (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SPELLS_MGR.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 6694 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp`. Rust target: `crates/wow-spell`. | `cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SPELLS_MGR.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 6694 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp`. Rust target: `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SPELLS_MGR.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 6694 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp`. Rust target: `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SPELLS_MGR.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 6694 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp`. Rust target: `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `LoadSpellInfoStore` joins all 20 sibling DB2 stores correctly for a known multi-difficulty spell (e.g. Lich King encounter spell with N+H+M variants); the `Difficulty` fallback chain produces the correct final field values
- [ ] Test: `LoadSpellInfoServerside` rejects rows whose `SpellID` already exists in DB2-loaded spells (matches C++ `TC_LOG_ERROR("…regular spell loaded from file…")`)
- [ ] Test: `LoadSpellRanks` produces `prev/next/first/last/rank` from `SkillLineAbilityEntry::SupercedesSpell` matching C++ for the Frostbolt rank chain (1→3→4→5→6→7)
- [ ] Test: `LoadSpellRequired` rejects a row where `spell_id` and `req_spell` are in the same chain (matches C++ "are ranks of the same spell, entry not needed, skipped")
- [ ] Test: `LoadSpellLearnSpells` deduplicates against DB2 `SpellLearnSpell` (matches C++ "redundant" warning case)
- [ ] Test: `LoadSpellTargetPositions` rejects effects whose `Target` isn't `TARGET_DEST_DB`
- [ ] Test: `LoadSpellProcs` with `SpellId = -id` applies the rule to every spell in the rank chain
- [ ] Test: `LoadSpellGroupStackRules` for an `EXCLUSIVE_SAME_EFFECT` group correctly populates `mSpellSameEffectStack` derived map
- [ ] Test: `LoadSpellAreas` populates all 5 indices (primary + quest-start/end + quest-end + aura + area) for a row with both a quest and an area trigger — represented `wow-data` unit coverage exists; live SpellMgr/runtime coverage pending.
- [x] Test: `CanSpellTriggerProcOnEvent` AND-matches every dimension; a procEntry with `SchoolMask=Fire` rejects an event with `SchoolMask=Frost`
- [ ] Test: `IsSpellValid` returns `Ok` for Pyroblast (spellId 11366); returns `Err` for a spell whose `EffectItemType` references a missing item template
- [ ] Test: `GetSpellInfo(spellId, Difficulty::Heroic)` falls back to `Difficulty::None` when Heroic-keyed row is absent
- [ ] Test: Boot load of full `world` DB completes; `mSpellInfoMap.len() == sSpellNameStore.size()` after `LoadSpellInfoStore`
- [ ] Test: `LoadSpellInfoCorrections` applies a known correction (e.g. setting `Attributes |= SPELL_ATTR0_PASSIVE` on spell X); subsequent `IsPassive()` returns true for that spell
- [ ] Test: Load order — calling `LoadSpellRequired` before `LoadSpellInfoStore` panics or returns an error (matches C++ assumption that `_GetSpellInfo` lookups during validation must work)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 6694 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp` | `crates/wow-spell/src/spell_mgr.rs` (planned), `crates/wow-data/src/spell.rs` (DB2 readers — partial) \| ❌ not started — the entire singleton is missing in Rust. Audited and confirmed empty 2026-05-01. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SPELLS_MGR.DIV.001` | `crates/wow-spell/src/spell_mgr.rs` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6694 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SPELLS_MGR.DIV.002` | `crates/wow-spell/src/lib.rs` (`exists_empty`, 0 Rust lines) | 4 C++ files / 6694 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SPELLS_MGR.DIV.003` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 4 C++ files / 6694 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/TraitMgr.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Load order is not the call order — it is the dependency order.** From `World.cpp:1861-2148`: `LoadSpellInfoStore` → `LoadSpellInfoServerside` → `LoadSpellInfoCorrections` → `LoadSkillLineAbilityMap` → `LoadSpellInfoCustomAttributes` → `LoadSpellInfoDiminishing` → `LoadSpellInfoImmunities` → `LoadPetFamilySpellsStore` → `LoadSpellTotemModel` → (later) `LoadSpellRanks` → `LoadSpellRequired` → `LoadSpellGroups` → `LoadSpellLearnSkills` → `LoadSpellInfoSpellSpecificAndAuraState` (must be after `LoadSpellRanks`) → `LoadSpellLearnSpells` → `LoadSpellProcs` → `LoadSpellThreats` → `LoadSpellGroupStackRules` → `LoadSpellEnchantProcData` → … → `LoadPetLevelupSpellMap` → `LoadPetDefaultSpells` → `LoadSpellAreas` → `LoadSpellPetAuras` → `LoadSpellTargetPositions` → `LoadSpellLinked`. **Never** rearrange — `LoadSpellInfoSpellSpecificAndAuraState` reads chains and `LoadSpellProcs` does negative-id chain lookup. `LoadSpellLearnSkills` itself is derived from `mSpellInfoMap` effects, not rank chains.
- **Difficulty fallback chain (`LoadSpellInfoStore`):** when a `(spellId, difficulty)` slot is missing a sibling DB2 record, the loader walks `DifficultyEntry::FallbackDifficultyID` to fill the gap. The walk continues until the fallback is `DIFFICULTY_NONE`. Each missing field is filled independently — a single record can mix data from multiple difficulty levels. **Visuals do NOT cascade across the full fallback chain** (only fall back to the first difficulty that defines any visual; intentional asymmetry).
- **`LoadSpellInfoCorrections` is the bug-warehouse.** ~1,500 manual `ApplySpellFix({…ids…}, [](SpellInfo* spellInfo){ /* mutate */ })` lambdas. It is the single longest function in TrinityCore. Rust port should split per-system (per-class, per-instance/raid, per-mechanic) and register via `inventory::submit!` — one file per logical group, each compiled-checked. Do not try to port it as a single function — it would not compile.
- **`spell_dbc` is a misnomer in modern TrinityCore.** The user-task brief mentions an `spell_dbc` SQL overlay table; in 3.4.3.54261 this role is split: (a) `serverside_spell` + `serverside_spell_effect` for **server-only** spells via `LoadSpellInfoServerside`, and (b) the **hotfixes DB** `hotfix_data` + per-table `Spell` / `SpellEffect` etc. rows for **overlaying real DB2 spells** via `DB2DatabaseLoader::LoadFromDB` (which `SpellMgr` does not call directly — it happens transparently inside `DB2Storage<T>::Load`). Document both paths; the second one belongs partly in `shared-datastores.md`.
- **`SpellMgr` is logically read-only after load,** but the C++ class is *not* `const` — the loaders mutate `mSpellInfoMap` in place and call `const_cast<SpellInfo&>(...)` from `UnloadSpellInfoImplicitTargetConditionLists`. The Rust port should make this explicit: a `SpellMgrBuilder` is mutable, and calling `.finalize()` returns an `Arc<SpellMgr>` with all maps frozen. No `const_cast`-equivalent at runtime.
- **`mSpellInfoMap` is a multi-keyed container** (TrinityCore-internal `std::unordered_set`-of-`SpellInfo`-like with custom hash on `(Id, Difficulty)`). Rust equivalent should be `DashMap<(u32, Difficulty), Arc<SpellInfo>>` or an indexed Vec with a `(spellId, difficulty)→idx` HashMap.
- **Negative SpellId in `spell_proc` SQL** (legacy convention): `-spellId` = "apply this proc rule to every spell in the same rank chain as `+spellId`". Easy to miss; if the Rust loader ignores the sign, ~30% of proc rules will apply to only the wrong rank.
- **`LoadSpellPetAuras` allows `petEntry == 0` as a wildcard** — the lookup `GetAura(petEntry)` first tries the exact petEntry, then falls back to entry 0. Don't reject 0 as invalid input. The C++ query is `SELECT spell, effectId, pet, aura FROM spell_pet_auras`; `removeOnChangePet` is derived from `TargetA == TARGET_UNIT_PET` and `damage` from `CalcValue()` of the source spell effect.
- **`SpellInfoLoadHelper::Visuals` is presorted by `CasterPlayerConditionID DESC`** with unconditional visuals at the *end*. The lookup `GetSpellXSpellVisualId` walks in order and picks the first matching condition. Sort order is load-time invariant — don't reorder during port.
- **`LoadSpellRanks` is not SQL in this legacy.** Current C++ builds `mSpellChains` from `SkillLineAbilityEntry::SupercedesSpell`, skipping links whose source or target spell info is missing. Older docs/tools may mention `spell_ranks`/`spell_chain`; that is stale for this canonical tree.
- **`SpellChainNode` patches `SpellInfo::ChainEntry` mid-load.** `LoadSpellRanks` creates `SpellChainNode`s but also mutates already-loaded `SpellInfo` objects to set their `ChainEntry` pointer. This is why `LoadSpellInfoSpellSpecificAndAuraState` *must* run after `LoadSpellRanks` — it reads `ChainEntry` to determine spell specificity (e.g. "first rank only" classifications). The current Rust `SpellChainStoreLikeCpp` represents the graph and accessor semantics only; live `SpellInfo::chain_entry` patching is still pending.
- **Bulk `WorldDatabase.Query`, not prepared statements.** All ~14 SQL loaders use raw `Query()` — these are one-shot startup loads, not hot path. The Rust port should use `sqlx::query` (not `query!` macros — column lists are too long and table schemas vary across deployments) and stream rows.
- **No memoization between difficulty variants.** `mSpellInfoMap` stores each `(spellId, difficulty)` independently; a `Heroic` and a `Normal` variant of the same spell are two distinct `SpellInfo` Arc instances even when most fields are equal. This is a deliberate trade — saves a per-field lookup at every cast. Rust port should keep the same shape.
- **Performance:** full load on a populated `world` DB takes ~1-2s in C++ on dev hardware. Rust async load should match within 2× — the bottleneck is DB2 file IO (cached after first run) + SQL deserialization, not CPU.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class SpellMgr` (singleton via `instance()`) | `pub struct SpellMgr { … }` constructed via `SpellMgrBuilder` and stored in `OnceCell<Arc<SpellMgr>>` | Frozen after `.finalize()`; no runtime mutation. |
| `static SpellMgr* SpellMgr::instance()` | `pub fn global() -> &'static Arc<SpellMgr>` | OnceCell-backed; panics if called before init. |
| `mSpellInfoMap` (multi-key set) | `DashMap<(u32, Difficulty), Arc<SpellInfo>>` | Read-only at runtime |
| `mSpellChains: SpellChainMap` | `HashMap<u32, SpellChainNode>` (frozen) | `SpellChainNode` references via `Arc<SpellInfo>` instead of raw pointer |
| `mSpellReq` / `mSpellsReqSpell` (multimap) | two `HashMap<u32, SmallVec<[u32; 4]>>` | Forward + reverse |
| `mSpellLearnSpells` (multimap) | `HashMap<u32, Vec<SpellLearnSpellNode>>` | — |
| `mSpellTargetPositions: map<pair<u32, SpellEffIndex>, SpellTargetPosition>` | `HashMap<(u32, SpellEffIndex), SpellTargetPosition>` | — |
| `mSpellSpellGroup` / `mSpellGroupSpell` | two `HashMap<…, Vec<…>>` (forward + reverse) | `SpellGroup` is `#[repr(u32)]` enum |
| `mSpellGroupStack: map<SpellGroup, SpellGroupStackRule>` | `HashMap<SpellGroup, SpellGroupStackRule>` | — |
| `mSpellAreaMap` + 4 secondary indices | `MultiSpellAreaIndex` newtype wrapping 5 `HashMap<…>` | One forward, 4 reverse — populate together in `LoadSpellAreas` |
| `mSpellLinkedMap: unordered_map<pair<SpellLinkedType, u32>, vector<i32>>` | `HashMap<(SpellLinkedType, u32), SmallVec<[i32; 4]>>` | — |
| `SpellInfoLoadHelper` (POD with raw DB2 pointers) | `struct SpellInfoLoadHelper<'a>` with `Option<&'a SpellEffectEntry>` etc. | Lifetime-bound to the DB2 stores; consumed during `LoadSpellInfoStore` and dropped |
| `SpellProcEntry` | `struct SpellProcEntry` with `flag128: Flag128`, `proc_flags: ProcFlagsInit` (ProcFlags + ProcFlags2 packed), `Cooldown: Duration` | `flag128` = `[u32; 4]` newtype with `Hash + Eq` |
| `enum SpellGroup` | `#[repr(u32)] enum SpellGroup { … }` | Values < 1000 are core; ≥ 1000 are DB-defined |
| `void LoadXxx()` (every loader is `void` returning) | `async fn load_xxx(&mut self, pool: &MariaDbPool) -> Result<()>` | Returns `Result` for SQL errors; logs invalid rows via `tracing::warn!` |
| `WorldDatabase.Query("SELECT … FROM spell_proc")` | `sqlx::query("SELECT … FROM spell_proc").fetch_all(pool).await?` | No `query!` macro — column count is too long and varies per deployment |
| `static bool CanSpellTriggerProcOnEvent(...)` | `pub fn can_spell_trigger_proc_on_event(entry: &SpellProcEntry, event: &ProcEventInfo) -> bool` | Pure function; no `&self` |
| `bool IsSpellValid(SpellInfo const*, Player*, bool msg)` | `fn is_spell_valid(info: &SpellInfo, player: Option<&Player>) -> Result<(), ValidationError>` | `msg` param replaced by `tracing::warn!` |
| `ApplySpellFix({1,2,3}, [](SpellInfo* s){ s->Attributes |= SPELL_ATTR0_PASSIVE; })` | `inventory::submit! { SpellFix { ids: &[1,2,3], apply: |s| s.attributes |= SpellAttr0::Passive } }` registered via `inventory` crate | One module per logical group of fixes |
| `flag128` (4× u32, hashable) | `pub struct Flag128([u32; 4])` with `Hash + Eq + Copy + Debug` | Used as key in proc family masks and linked-spell |

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical at `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellMgr.{h,cpp}` (827 + 5,028 lines = 5,855 total) and `World.cpp:1861-2148` (the master load-order driver) against Rust workspace at `/home/server/rustycore/crates/`.

**Empty-target finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` is 0 bytes. No `SpellMgr`, no `SpellInfoLoadHelper`, no `SpellProcEntry`, no `SpellChainNode`, no `SpellTargetPosition`, no `SpellArea`, no `PetAura`, no `SpellLinkedMap`. The 5,855 lines of C++ singleton + loaders map to **zero lines** of Rust.

**SQL loader coverage.** All 14 `spell_*` SQL tables are unloaded:
- `LoadSpellRanks` / rank chains — represented Rust store/accessors exist in `wow-data` (`SpellChainStoreLikeCpp`) and are derived from `SkillLineAbilityEntry::SupercedesSpell` like current C++; the store is built during world-server startup, but live `SpellMgr` ownership and `SpellInfo::ChainEntry` patching are still absent.
- `spell_required` — represented Rust query/store exists in `wow-data`, is loaded during world-server startup against the real rank-chain store, and is injected into sessions with C++ forward/reverse prerequisite lookups. Spell-learning/cast-check consumption is still absent, so the prerequisite graph is loaded but not fully live in gameplay.
- `spell_learn_spell` — represented query/store exists (`WorldStatements::SEL_SPELL_LEARN_SPELL`, `SpellLearnSpellStoreLikeCpp`), but startup wiring and the `Player::add_spell` learn-cascade runtime are still absent.
- `spell_target_position` — represented loader/store exists and is wired at world startup. It is consumed by bounded represented `TARGET_DEST_DB` / `_OR_DB` paths, but full `SpellMgr` ownership and complete `EffectTeleportUnits` parity remain pending.
- `spell_group` — represented query/store exists (`WorldStatements::SEL_SPELL_GROUP`, `SpellGroupStoreLikeCpp`) with recursive nested-group expansion, is loaded during world-server startup, and is injected into sessions with C++ group/membership accessors. Stack-rule runtime consumption is still pending.
- `spell_group_stack_rules` — represented query/store exists (`WorldStatements::SEL_SPELL_GROUP_STACK_RULES`, `SpellGroupStackRuleStoreLikeCpp`) with C++ validation and same-effect aura inference, is loaded during world-server startup from the live spell-group store, and is injected into sessions with represented stack-rule accessors. Real aura stacking runtime consumption is still absent.
- `spell_proc` — represented explicit SQL-row loader exists (`WorldStatements::SEL_SPELL_PROC`, `SpellProcStoreLikeCpp`) with rank-chain expansion, primary validation, represented difficulty-fallback lookup, C++ implicit-proc aura classification, SQL-then-implicit represented store loading from `SpellInfo` sources, an adapter from loaded DB2 stores to proc source `SpellInfo`, pure implicit proc-entry generation, `SpellEffectInfo` class-mask and `DieSides` preservation for implicit source conversion, and pure `CanSpellTriggerProcOnEvent` matching. World-server startup now builds `SpellProcStoreLikeCpp` from SQL rows plus implicit sources and passes it to sessions. Live `SpellMgr` ownership is absent, authoritative multi-difficulty `SpellInfo` coverage is still incomplete, and the actual proc runtime is still absent.
- `spell_threat` — represented Rust query/store exists in `wow-data`, is loaded during world-server startup, and is injected into sessions with C++ first-rank fallback lookup. Spell/threat runtime consumption is still absent, so per-spell threat overrides are loaded but not yet live in gameplay.
- `spell_pet_auras` — represented Rust query/store exists in `wow-data`, but it is not yet loaded during world-server startup or consumed by owner→pet aura runtime. Owner→pet aura inheritance (Beast Mastery hunter, Demonology warlock) is therefore not live yet.
- `spell_enchant_proc_data` — represented Rust query/store exists in `wow-data`, is loaded during world-server startup against `SpellItemEnchantment.db2`, and is injected into sessions with C++ `GetSpellEnchantProcEvent` lookup semantics. Item-enchant proc runtime consumption is still absent.
- `spell_linked_spell` — represented Rust query/store exists in `wow-data`, but it is not yet loaded during world-server startup or consumed by cast/hit/aura/remove runtime. Spell-A-triggers-Spell-B chains (used heavily by boss scripts) are therefore not live yet.
- `spell_area` — represented Rust query/store exists (`WorldStatements::SEL_SPELL_AREA`, `SpellAreaStoreLikeCpp`) with validation and indices, but it is not yet loaded into a live `SpellMgr` or consumed by aura apply/remove runtime. Zone-conditional auras (sanctum buffs, capital city resting buffs, racial flight masters) remain inactive.
- `spell_totem_model` — represented Rust query/store exists in `wow-data`, but it is not yet loaded during world-server startup or consumed by totem summon runtime. Race-specific totem display (Tauren vs Orc shaman totems) is therefore not live yet.
- `spell_custom_attr` — represented Rust query/store exists (`WorldStatements::SEL_SPELL_CUSTOM_ATTR`, `SpellCustomAttributeStoreLikeCpp`) for SQL-driven `AttributesCu` overrides, but live `SpellMgr` wiring and derived `AttributesCu` computation remain absent.
- `serverside_spell` / `serverside_spell_effect` (the modern `spell_dbc` overlay) — represented Rust query/store coverage exists for row/effect staging and `mServersideSpellNames` shape, but authoritative `SpellInfo` insertion and live startup wiring are still absent. Server-only spells (used by scripts to apply bookkeeping auras with no client visualization) are therefore not live yet.

**DB2 cross-table join coverage.** `LoadSpellInfoStore`'s 20-store join (`SpellName × SpellEffect × SpellMisc × SpellAuraOptions × SpellAuraRestrictions × SpellCastingRequirements × SpellCategories × SpellClassOptions × SpellCooldowns × SpellEquippedItems × SpellInterrupts × SpellLabel × SpellLevels × SpellPower × SpellReagents × SpellReagentsCurrency × SpellScaling × SpellShapeshift × SpellTargetRestrictions × SpellTotems × SpellXSpellVisual + difficulty fallback`) has **zero analog** in `crates/wow-data` — only `Spell.db2` itself is parsed (one of 20 sibling stores; the join is absent).

**`LoadSpellInfoCorrections` coverage.** The single largest function in the C++ source (~1,540 lines, ~1,500 hand-coded `ApplySpellFix` calls) has no Rust counterpart. Every individual correction (e.g. "spell X must have `SPELL_ATTR0_PASSIVE` because the DB2 entry is wrong") is missing. This means even after the loader skeleton is up, every fixed spell will revert to its broken DB2 default until ported one-by-one.

**Proc validation represented, runtime absent.** `CanSpellTriggerProcOnEvent` (the central proc-condition AND across proc flags, attributes, school, family, spell type, phase and hit masks) now exists as a pure represented matcher in `wow-data`. The live proc system remains absent: no `Unit::ProcSkillsAndAuras` integration, no aura trigger dispatch, no proc cooldown/charge consumption, and no live `SpellMgr` ownership path yet.

**Singleton lifecycle absent.** No `SpellMgr::instance()` accessor, no `OnceCell`-backed initializer, no startup sequence in `world-server` calling the 30 loaders in the C++ `World.cpp:1861-2148` order.

**Persistence path note.** `SpellMgr` itself does not write to DB at runtime; its data is loaded once at boot. Persistence of *learned* spells / *active* auras / *cooldowns* lives in `Player` save in the `character` DB and is the responsibility of `spells-info.md` / `spells.md` consumers, not this loader.

**Migration entry point.** The first concrete sub-task in §9 (#SPELLMGR.1) creates `crates/wow-spell/src/spell_mgr.rs` with the struct + `OnceCell` accessor; subsequent sub-tasks fill loaders in dependency order matching `World.cpp:1861-2148`. Any out-of-order loader implementation will produce a runtime where lookups silently return `None` for valid spells — verify the load-order test (#10 last item) before declaring a sub-task done.
