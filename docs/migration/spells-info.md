# Migration: Spells — SpellInfo (composed read-only spell descriptor)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Spells/SpellInfo.{h,cpp}` (~5,647 lines combined: 625 + 5,022)
> **Rust target crate(s):** `crates/wow-spell/src/spell_info.rs` (planned), `crates/wow-data/src/spell.rs` (DB2 readers — partial)
> **Layer:** L5 sub-module (Game systems — Spells static data type). Parent: `spells.md`. Sibling: `spells-mgr.md` (the loader that constructs every instance).
> **Status:** ❌ not started — type does not exist; only a tiny stub `wow_data::SpellInfo` (cast_time + cooldown) is in use.
> **Audited vs C++:** ✅ complete (see §13)
> **Last updated:** 2026-05-01

> **Cross-links:** see `spells-mgr.md` for the singleton that constructs `SpellInfo` instances and owns the `mSpellInfoMap`; `spells.md` for the engine that consumes these instances; `shared-datastores.md` for the DB2 readers + hotfix overlay supplying every per-store input.

---

## 1. Purpose

`SpellInfo` is the **read-only, immutable, fully-composed descriptor** of a single (spellId, difficulty) pair after `SpellMgr::LoadSpellInfoStore` has merged ~20 sibling DB2 stores into one record. It is what every cast, aura tick, talent check, dispel roll, threat calculation, range check, line-of-sight check, and proc match reads. The class exposes ~150 const methods (predicates, getters, `Calc*` helpers, `Check*` validators) and 1 mutable bridge (`_LoadSpellSpecific` / `_LoadAuraState` / `_LoadSpellDiminishInfo` / `_LoadImmunityInfo`) called once during boot finalization.

The single most important conceptual fact: **`SpellInfo` is not a 1:1 mirror of any one DB2 file.** It is a **composed type**. `Spell.db2` (= `SpellNameStore`) only carries the spell ID and localized name. Everything else — attributes, effects, range, cooldown, school mask, mechanic, reagents, totems, target restrictions, levels, power costs, scaling, interrupts — comes from one of ~19 other `Spell*.db2` sibling files plus optional difficulty fallback. The 14 `AttributesEx*` 32-bit bitmasks (`SPELL_ATTR0..SPELL_ATTR14`) live in `SpellMisc.db2`. The 32 effect slots (max 32 in DF, 3 used in 3.4.3 WoLK) are individual `SpellEffect.db2` rows joined by `(SpellID, EffectIndex, DifficultyID)`. Power costs are an array of up to `MAX_POWERS_PER_SPELL` entries from `SpellPower.db2` ordered by `OrderIndex`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Spells/SpellInfo.cpp` | 5022 | `prefix` |
| `game/Spells/SpellInfo.h` | 625 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Spells/SpellInfo.h` | 625 | `class SpellInfo` declaration with ~80 public fields, 14 `HasAttribute` overloads, ~150 const methods; nested classes `SpellEffectInfo`, `SpellImplicitTargetInfo`, structs `SpellPowerCost`, `SpellDiminishInfo`; enums `SpellTargetSelectionCategories`, `SpellTargetReferenceTypes`, `SpellTargetObjectTypes`, `SpellTargetCheckTypes`, `SpellTargetDirectionTypes`, `SpellEffectImplicitTargetTypes`, `SpellSpecificType`, `SpellCustomAttributes` |
| `src/server/game/Spells/SpellInfo.cpp` | 5,022 | Two ctors (full and effect-only), 60+ predicates (`IsPositive`, `IsChanneled`, `IsPassive`, `IsRanked`, `IsAffected`, `CanDispelAura`, `CanPierceImmuneAura`, `IsAuraExclusiveBySpecificWith`, …), `Check*` validators, `Calc*` helpers, the giant `_LoadImmunityInfo` (~313 lines), `_LoadSpellDiminishInfo` (~471 lines), `_LoadSpellSpecific` (~188 lines), `_InitializeSpellPositivity` (~39 lines), and the `SpellEffectInfo` ctor + ~30 methods |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SpellInfo` | class | The composed descriptor (this doc's main type). ~80 public fields + ~150 methods. Constructed by `SpellMgr` from `SpellInfoLoadHelper`; immutable after `_Load*` post-processors complete |
| `SpellEffectInfo` | class (nested logically; defined in same file) | One spell-effect slot. Holds `Effect: SpellEffectName`, `ApplyAuraName: AuraType`, `BasePoints`, `RealPointsPerLevel`, `PointsPerResource`, `Amplitude`, `MiscValue`/`MiscValueB`, `Mechanic`, `TargetA: SpellImplicitTargetInfo`, `TargetB`, `TargetARadiusEntry`, `TargetBRadiusEntry`, `ChainTargets`, `DieSides`, `ItemType`, `TriggerSpell`, `SpellClassMask: flag128`, `BonusCoefficientFromAP`, `ImplicitTargetConditions: shared_ptr<vector<Condition>>`, `EffectAttributes`, `Scaling: { Class, Coefficient, Variance, ResourceCoefficient }`, `_immunityInfo: unique_ptr<ImmunityInfo>` |
| `SpellImplicitTargetInfo` | class | Wraps a single `Targets` enum value; classifies it via static `_data: array<StaticData, TOTAL_SPELL_TARGETS>` into `(ObjectType, ReferenceType, SelectionCategory, SelectionCheckType, DirectionType)`. ~150 valid Targets values |
| `SpellPowerCost` | struct | `{ Power: Powers, Amount: int32 }` — output of `CalcPowerCost` |
| `SpellDiminishInfo` | struct | `{ DiminishGroup, DiminishReturnType, DiminishMaxLevel, DiminishDurationLimit }` |
| `SpellEffectInfo::ImmunityInfo` (forward-declared, defined in `.cpp`) | struct | School/Mechanic/Aura/Effect immunity masks computed at load time |
| `SpellEffectInfo::ScalingInfo` | nested struct | Per-effect scaling (Class, Coefficient, Variance, ResourceCoefficient) |
| `SpellInfo::ScalingInfo` | nested struct | Per-spell scaling (`MinScalingLevel`, `MaxScalingLevel`, `ScalesFromItemLevel`) |
| `SpellInfo::SqrtDamageAndHealingDiminishing` | nested struct | `{ MaxTargets, NumNonDiminishedTargets }` for sqrt-AoE damage falloff |
| `SpellTargetSelectionCategories` | enum | `NYI / DEFAULT / CHANNEL / NEARBY / CONE / AREA / TRAJ / LINE` — how to find candidate targets |
| `SpellTargetReferenceTypes` | enum | `NONE / CASTER / TARGET / LAST / SRC / DEST` — reference frame for target selection |
| `SpellTargetObjectTypes` | enum (`uint8`) | `NONE / SRC / DEST / UNIT / UNIT_AND_DEST / GOBJ / GOBJ_ITEM / ITEM / CORPSE / CORPSE_ENEMY / CORPSE_ALLY` |
| `SpellTargetCheckTypes` | enum (`uint8`) | `DEFAULT / ENTRY / ENEMY / ALLY / PARTY / RAID / RAID_CLASS / PASSENGER / SUMMONED / THREAT / TAP` |
| `SpellTargetDirectionTypes` | enum | `NONE / FRONT / BACK / RIGHT / LEFT / FRONT_RIGHT / BACK_RIGHT / BACK_LEFT / FRONT_LEFT / RANDOM / ENTRY` — cone direction |
| `SpellEffectImplicitTargetTypes` | enum | `NONE / EXPLICIT / CASTER` |
| `SpellSpecificType` | enum | Cross-spell exclusivity classes (Seal, Aura, Sting, Curse, Aspect, Tracker, WarlockArmor, MageArmor, ElementalShield, Polymorph, Corruption, Food, Drink, Presence, Charm, Scroll, ArcaneBrilliance, WarriorEnrage, DivineSpirit, Hand, Phase, Bane). Used by `IsAuraExclusiveBySpecificWith` to enforce 1-of-class limits |
| `SpellCustomAttributes` | enum bitmask | 25 bits in `AttributesCu`: `ENCHANT_PROC`, `CONE_BACK`, `CONE_LINE`, `SHARE_DAMAGE`, `NO_INITIAL_THREAT`, `AURA_CC`, `DONT_BREAK_STEALTH`, `CAN_CRIT`, `DIRECT_DAMAGE`, `CHARGE`, `PICKPOCKET`, `IGNORE_ARMOR`, `REQ_TARGET_FACING_CASTER`, `REQ_CASTER_BEHIND_TARGET`, `ALLOW_INFLIGHT_TARGET`, `NEEDS_AMMO_DATA`, `BINARY_SPELL`, `SCHOOLMASK_NORMAL_WITH_MAGIC`, `IS_TALENT`, `AURA_CANNOT_BE_SAVED` (plus 5 deprecated `DO NOT REUSE` slots) |
| `SpellAttr0..14` | enum bitmask (defined in `SharedDefines.h`) | 15 groups × 32 bits = ~450 individual attribute flags; live as `Attributes`/`AttributesEx`/`AttributesEx2..14` `u32` fields on `SpellInfo` |
| `SpellEffectName` | enum (in `SharedDefines.h`) | ~151 `SPELL_EFFECT_*` values; type of `Effect` field on each `SpellEffectInfo` |
| `AuraType` | enum (in `SpellAuraDefines.h`) | ~280 `SPELL_AURA_*` values; type of `ApplyAuraName` field |
| `Mechanics` | enum (in `SharedDefines.h`) | 33 spell mechanics (Stun, Fear, Root, Silence, Disarm, Sleep, Charm, …) — both spell-level (`Mechanic`) and effect-level (`Mechanic` per `SpellEffectInfo`) |
| `DispelType` | enum (in `SharedDefines.h`) | 9 dispel categories (Magic, Curse, Disease, Poison, Stealth, Invisibility, Enrage, …); `Dispel` field is the spell's category, `GetDispelMask` builds a bitmask |
| `SpellSchoolMask` | enum bitmask | 7 schools (Normal/Holy/Fire/Nature/Frost/Shadow/Arcane); the `SchoolMask` field is from `SpellMisc.db2` |
| `SpellInterruptFlags`, `SpellAuraInterruptFlags`, `SpellAuraInterruptFlags2` | enum bitmask | Cast-interrupt + aura-interrupt + 2nd word of aura-interrupt; live as `EnumFlag<…>` typed fields |

---

## 4. Critical public methods / functions

The class has ~150 const methods. Listed by category; only the most-called are fully named.

**Construction (boot only):**

| Symbol | Purpose | Calls into |
|---|---|---|
| `SpellInfo(SpellNameEntry const*, Difficulty, SpellInfoLoadHelper const&)` | Primary ctor — composes from ~20 sibling DB2 records. See §11 | `SpellEffectInfo` ctor (per slot), DB2 store lookups |
| `SpellInfo(SpellNameEntry const*, Difficulty, vector<SpellEffectEntry> const&)` | Effect-only ctor for serverside spells (no `Misc`/`Categories`/etc.) | `SpellEffectInfo` ctor |
| `~SpellInfo()` | Destructor; releases `_immunityInfo` and `ImplicitTargetConditions` | — |
| `_InitializeExplicitTargetMask()` | Computes `ExplicitTargetMask` from each effect's `TargetA`/`TargetB` — what targets the spell explicitly demands | `SpellImplicitTargetInfo::GetExplicitTargetMask` |
| `_InitializeSpellPositivity()` | Walks effects + heuristics to set per-effect `NegativeEffects` bitset and overall `IsPositive` semantics | several predicates |
| `_LoadSpellSpecific()` | Reads the spell to classify into `SpellSpecificType` (188 lines of pattern-matching: family flags, effect types, aura types) | many family-mask checks |
| `_LoadAuraState()` | Sets `_auraState` (the `AuraState` enum) from family flags + attribute flags | family flag analysis |
| `_LoadSpellDiminishInfo()` | Walks 471 lines of pattern-matching to set `_diminishInfo` (DR group, type, max level, duration limit) | family flag analysis |
| `_LoadImmunityInfo()` | Per-effect: builds `_immunityInfo` masks by walking aura-applied effects with `SchoolImmunity`/`MechanicImmunity`/`StateImmunity`/`DispelImmunity` aura types (313 lines) | `SpellEffectInfo::EffectAttributes` |
| `_UnloadImplicitTargetConditionLists()` | Frees per-effect `ImplicitTargetConditions` after Conditions module is shut down | — |

**Predicates (hot path):**

| Symbol | Purpose |
|---|---|
| `HasAttribute(SpellAttr0..14)` (15 overloads) | Bit test on the 15 `Attributes*` fields |
| `HasAttribute(SpellCustomAttributes)` | Bit test on `AttributesCu` |
| `HasEffect(SpellEffectName)` | True if any effect has this `Effect` |
| `HasAura(AuraType)` | True if any effect's `ApplyAuraName` matches |
| `HasAreaAuraEffect()` | True if any effect is `APPLY_AREA_AURA_*` |
| `HasOnlyDamageEffects()` | True if every non-NONE effect is damage class |
| `HasTargetType(Targets)` | True if any effect references this implicit target |
| `HasAnyAuraInterruptFlag()` | `AuraInterruptFlags || AuraInterruptFlags2` |
| `HasAuraInterruptFlag(SpellAuraInterruptFlags)` / `HasAuraInterruptFlag(SpellAuraInterruptFlags2)` | Per-bit |
| `HasChannelInterruptFlag(SpellAuraInterruptFlags)` / `HasChannelInterruptFlag(SpellAuraInterruptFlags2)` | Per-bit |
| `HasInitialAggro()` | Inverse of `SPELL_ATTR1_NO_THREAT` etc. |
| `HasHitDelay()` | `Speed > 0 || LaunchDelay > 0` |
| `HasLabel(uint32)` | `Labels.contains(labelId)` |
| `IsPositive()` / `IsPositiveEffect(uint8)` | Compound predicate considering `NegativeEffects` bitset, target type, family flags |
| `IsPassive()` | `HasAttribute(SPELL_ATTR0_PASSIVE)` |
| `IsAutocastable()` | For pet abilities |
| `IsChanneled()` / `IsMoveAllowedChannel()` | `HasAttribute(SPELL_ATTR1_IS_CHANNELLED)` etc. |
| `IsRanked()` / `GetRank()` / `IsRankOf(other)` / `IsDifferentRankOf(other)` / `IsHighRankOf(other)` | Chain queries via `ChainEntry` |
| `GetFirstRankSpell()` / `GetLastRankSpell()` / `GetNextRankSpell()` / `GetPrevRankSpell()` / `GetAuraRankForLevel(uint8)` | Chain navigation |
| `IsCooldownStartedOnEvent()` | `HasAttribute(SPELL_ATTR0_COOLDOWN_ON_EVENT)` |
| `IsDeathPersistent()` | `HasAttribute(SPELL_ATTR3_DEATH_PERSISTENT)` |
| `IsRequiringDeadTarget()` / `IsAllowingDeadTarget()` | Target-state filters |
| `IsExplicitDiscovery()` | Profession spells revealing recipes |
| `IsLootCrafting()` / `IsProfession()` / `IsPrimaryProfession()` / `IsPrimaryProfessionFirstRank()` / `IsAbilityOfSkillType(skillType)` | Profession classification |
| `IsAffectingArea()` / `IsTargetingArea()` / `NeedsExplicitUnitTarget()` / `NeedsToBeTriggeredByCaster(triggerInfo)` | Target shape predicates |
| `IsStackableWithRanks()` / `IsPassiveStackableWithRanks()` / `IsMultiSlotAura()` / `IsStackableOnOneSlotWithDifferentCasters()` / `IsSingleTarget()` | Stacking semantics |
| `IsAuraExclusiveBySpecificWith(other)` / `IsAuraExclusiveBySpecificPerCasterWith(other)` | `SpellSpecificType` exclusivity check |
| `IsNextMeleeSwingSpell()` / `IsRangedWeaponSpell()` / `IsAutoRepeatRangedSpell()` | Combat classification |
| `IsItemFitToSpellRequirements(Item const*)` | Reagent + equipped item validation |
| `IsAffected(familyName, flag128)` | Spell family + family mask matching (used by talent affecter logic) |
| `IsAffectedBySpellMods()` / `IsAffectedBySpellMod(mod)` | Modifier eligibility |
| `CanBeUsedInCombat(caster)` | Combat-state validation |
| `CanPierceImmuneAura(other)` / `CanDispelAura(other)` | Dispel/immunity interaction |
| `CanSpellProvideImmunityAgainstAura(other)` / `SpellCancelsAuraEffect(AuraEffect const*)` | Reverse-direction immunity check |
| `CanBeInterrupted(WorldObject*, Unit*, bool ignoreImmunity)` | Cast-interrupt validation |
| `CheckTargetCreatureType(Unit const*)` | `TargetCreatureType` mask check |
| `IsGroupBuff()` | Party/raid-wide buff classification |

**Validators (return `SpellCastResult`):**

| Symbol | Purpose |
|---|---|
| `CheckShapeshift(form)` | Stances mask check |
| `CheckLocation(map_id, zone_id, area_id, player)` | Zone/area gating + `RequiredAreasID` |
| `CheckTarget(caster, target, implicit)` | Generic target validity (faction, type, alive/dead, LoS via caller) |
| `CheckExplicitTarget(caster, target, itemTarget)` | Targets the user actually selected match expected types |
| `CheckVehicle(caster)` | Vehicle-based casting restrictions |

**Calc helpers:**

| Symbol | Purpose |
|---|---|
| `CalcDuration(caster)` | Duration after caster mods (haste, talent extends) |
| `GetDuration()` / `GetMaxDuration()` | Raw duration values from `DurationEntry` |
| `GetMaxTicks()` | Number of periodic ticks for DoT/HoT |
| `CalcCastTime(spell)` | Cast time after caster mods |
| `GetRecoveryTime()` | Spell-specific cooldown (not GCD, not category) |
| `CalcPowerCost(powerType, optionalCost, caster, schoolMask, spell)` | Single-power version |
| `CalcPowerCost(SpellPowerEntry, optionalCost, caster, schoolMask, spell)` | Single-power (specific entry) |
| `CalcPowerCost(caster, schoolMask, spell)` | Multi-power version returning `vector<SpellPowerCost>` |
| `CalcProcPPM(caster, itemLevel)` | PpmRate after `ProcPPMMods` evaluation |
| `GetMinRange(positive)` / `GetMaxRange(positive, caster, spell)` | From `RangeEntry` plus mods |
| `GetSchoolMask()` | `SpellSchoolMask(SchoolMask)` |
| `GetAllEffectsMechanicMask()` / `GetEffectMechanicMask(effIndex)` / `GetSpellMechanicMaskByEffectMask(mask)` / `GetEffectMechanic(effIndex)` | Mechanics queries |
| `GetDispelMask()` (instance) / `GetDispelMask(DispelType)` (static) | Dispel category bitmasks |
| `GetExplicitTargetMask()` | Cached from `_InitializeExplicitTargetMask` |
| `GetAuraState()` / `GetSpellSpecific()` | Cached from `_LoadAuraState` / `_LoadSpellSpecific` |
| `GetDiminishingReturnsGroupForSpell()` / `GetDiminishingReturnsGroupType()` / `GetDiminishingReturnsMaxLevel()` / `GetDiminishingReturnsLimitDuration()` | Cached from `_LoadSpellDiminishInfo` |
| `GetAllowedMechanicMask()` / `GetMechanicImmunityMask(caster)` | Cached from `_LoadImmunityInfo` |
| `ApplyAllSpellImmunitiesTo(target, effect, apply)` | Walks `ImmunityInfo` and sets/clears immunities on target |
| `GetCategory()` | `CategoryId` |
| `GetAttackType()` | Maps `EquippedItemSubClassMask` → `WeaponAttackType` |
| `GetSpellXSpellVisualId(caster, viewer)` / `GetSpellVisual(caster, viewer)` | Visual lookup |
| `GetEffects()` (returns `vector<SpellEffectInfo> const&`) / `GetEffect(effIndex)` | Effect array accessors |
| `MeetsFutureSpellPlayerCondition(player)` | UI gating for not-yet-castable spells |

**SpellEffectInfo public methods:**

| Symbol | Purpose |
|---|---|
| `IsEffect()` / `IsEffect(SpellEffectName)` / `IsAura()` / `IsAura(AuraType)` | Type predicates |
| `IsTargetingArea()` / `IsAreaAuraEffect()` / `IsUnitOwnedAuraEffect()` | Target-shape predicates |
| `CalcValue(caster, basePoints, target, variance, castItemId, itemLevel)` | Final per-effect value with bonuses |
| `CalcBaseValue(caster, target, itemId, itemLevel)` | Pre-bonus value with scaling |
| `CalcValueMultiplier(caster, spell)` / `CalcDamageMultiplier(caster, spell)` | Per-effect multipliers |
| `HasRadius(SpellTargetIndex)` / `CalcRadius(caster, targetIndex, spell)` | Radius queries |
| `GetProvidedTargetMask()` / `GetMissingTargetMask(srcSet, destSet, mask)` | Target slot tracking |
| `GetImplicitTargetType()` / `GetUsedTargetObjectType()` / `GetScalingExpectedStat()` | Static-data dispatches |
| `GetImmunityInfo()` | Returns cached `ImmunityInfo*` |

---

## 5. Module dependencies

**Depends on:**
- `wow-data` (DB2 stores: `Spell`, `SpellEffect`, `SpellMisc`, `SpellAuraOptions`, `SpellAuraRestrictions`, `SpellCastingRequirements`, `SpellCategories`, `SpellClassOptions`, `SpellCooldowns`, `SpellEquippedItems`, `SpellInterrupts`, `SpellLabel`, `SpellLevels`, `SpellPower`, `SpellReagents`, `SpellReagentsCurrency`, `SpellScaling`, `SpellShapeshift`, `SpellTargetRestrictions`, `SpellTotems`, `SpellXSpellVisual`, `SpellRange`, `SpellRadius`, `SpellCastTimes`, `SpellDuration`, `SpellProcsPerMinute`, `SpellProcsPerMinuteMod`, `SpellPowerDifficulty`, `SkillLineAbility`, `SummonProperties`, `BattlePetSpecies`)
- `wow-spell::SpellMgr` (constructor caller — `SpellInfo` only ever created by the manager; never standalone)
- `wow-core::Difficulty` enum
- `Conditions` (`Condition` struct in `ImplicitTargetConditions`)
- `wow-spell::AuraEffect` (forward-declared; `SpellCancelsAuraEffect` takes one)
- `Item` / `Player` / `Unit` / `WorldObject` (forward-declared; passed into `Calc*` and `Check*` helpers)
- `flag128` (`SpellClassMask` per effect, `SpellFamilyFlags` per spell)
- `EnumFlag<T>` (typed bit-set wrapper used for `InterruptFlags`, `AuraInterruptFlags`, `AuraInterruptFlags2`, `ChannelInterruptFlags`, `ChannelInterruptFlags2`)
- `SharedDefines.h` (`SpellAttr0..14`, `SpellEffectName`, `AuraType`, `Powers`, `Mechanics`, `DispelType`, `SpellSchoolMask`)
- `DBCEnums.h` / `DB2Structure.h` (the per-store `Entry` POD types)

**Depended on by:**
- `wow-spell::Spell` — every cast reads `SpellInfo` for cast time, range, power, target mask, effects
- `wow-spell::Aura` / `AuraEffect` — periodic tick computation, refresh rules, stacking
- `wow-spell::SpellHistory` — cooldown semantics (RecoveryTime, CategoryRecoveryTime, StartRecoveryCategory, StartRecoveryTime, ChargeCategoryId)
- `wow-spell::SpellMgr` — lookups, validation (`IsSpellValid`)
- `wow-combat::ProcSystem` — `ProcFlags`, `ProcChance`, `ProcCharges`, `CalcProcPPM`
- `wow-combat::DamageInfo` — `SchoolMask`, `DmgClass`, `Mechanic`
- `wow-world::handlers::*` — every cast/learn/dispel/cancel handler
- `wow-pet::Pet` — pet ability casts via owner reading SpellInfo
- `Player` save/load — `IsCooldownStartedOnEvent`, `IsDeathPersistent` (pruning auras at logout)
- All `SpellScript` / `AuraScript` consumers — read SpellInfo to override behavior
- `wow-quest::QuestMgr` — quest-tied auras read `RequiredAreasID`

---

## 6. SQL / DB queries (if any)

`SpellInfo` itself **does not query the DB**. All fields are populated by `SpellMgr` from DB2 stores during `LoadSpellInfoStore`, optionally overlaid by `LoadSpellInfoServerside` (SQL `serverside_spell_effect`) and `LoadSpellInfoCorrections` (hand-coded fixes). Persistence of *active auras* and *cooldowns* lives in the `character` DB and is handled by `Player` save/load, not by `SpellInfo`.

DB2 stores read **directly** by the `SpellInfo` ctor (after `SpellInfoLoadHelper` is filled by `SpellMgr`):

| Store | What `SpellInfo` reads | Field populated |
|---|---|---|
| `SpellNameEntry` | `ID`, `Name` | `Id`, `SpellName` |
| `SpellMiscEntry` | `Attributes[15]`, `CastingTimeIndex`, `DurationIndex`, `RangeIndex`, `Speed`, `LaunchDelay`, `SchoolMask`, `SpellIconFileDataID`, `ActiveIconFileDataID`, `ContentTuningID`, `ShowFutureSpellPlayerConditionID` | `Attributes`, `AttributesEx`..`AttributesEx14`, `CastTimeEntry`, `DurationEntry`, `RangeEntry`, `Speed`, `LaunchDelay`, `SchoolMask`, `IconFileDataId`, `ActiveIconFileDataId`, `ContentTuningId`, `ShowFutureSpellPlayerConditionID` |
| `SpellScalingEntry` | `MinScalingLevel`, `MaxScalingLevel`, `ScalesFromItemLevel` | `Scaling.{...}` |
| `SpellAuraOptionsEntry` | `ProcTypeMask`, `ProcChance`, `ProcCharges`, `ProcCategoryRecovery`, `CumulativeAura`, `SpellProcsPerMinuteID` | `ProcFlags`, `ProcChance`, `ProcCharges`, `ProcCooldown`, `StackAmount`, `ProcBasePPM` (resolved via `sSpellProcsPerMinuteStore`), `ProcPPMMods` (via `sDB2Manager.GetSpellProcsPerMinuteMods`) |
| `SpellAuraRestrictionsEntry` | `CasterAuraState`, `TargetAuraState`, `Exclude*AuraState`, `*AuraSpell` | All 8 `*AuraState` / `*AuraSpell` fields |
| `SpellCastingRequirementsEntry` | `RequiresSpellFocus`, `FacingCasterFlags`, `RequiredAreasID` | Same |
| `SpellCategoriesEntry` | `Category`, `DispelType`, `Mechanic`, `StartRecoveryCategory`, `DefenseType`, `PreventionType`, `ChargeCategory` | `CategoryId`, `Dispel`, `Mechanic`, `StartRecoveryCategory`, `DmgClass`, `PreventionType`, `ChargeCategoryId` |
| `SpellClassOptionsEntry` | `SpellClassSet`, `SpellClassMask` | `SpellFamilyName`, `SpellFamilyFlags` |
| `SpellCooldownsEntry` | `RecoveryTime`, `CategoryRecoveryTime`, `StartRecoveryTime` | Same |
| `SpellEquippedItemsEntry` | `EquippedItemClass`, `EquippedItemSubclass`, `EquippedItemInvTypes` | `EquippedItemClass`, `EquippedItemSubClassMask`, `EquippedItemInventoryTypeMask` |
| `SpellInterruptsEntry` | `InterruptFlags`, `AuraInterruptFlags[2]`, `ChannelInterruptFlags[2]` | `InterruptFlags`, `AuraInterruptFlags`, `AuraInterruptFlags2`, `ChannelInterruptFlags`, `ChannelInterruptFlags2` |
| `SpellLabelEntry[]` | `LabelID` | `Labels: HashSet<u32>` |
| `SpellLevelsEntry` | `MaxLevel`, `BaseLevel`, `SpellLevel` | Same |
| `SpellPowerEntry[]` | Per-power-type cost rows; ordered by `OrderIndex` (or `SpellPowerDifficulty.OrderIndex`) | `PowerCosts: [Option<&SpellPowerEntry>; MAX_POWERS_PER_SPELL]` |
| `SpellReagentsEntry` | `Reagent[8]`, `ReagentCount[8]` | Same |
| `SpellReagentsCurrencyEntry[]` | Multi-row currency reagents | `ReagentsCurrency: Vec<…>` |
| `SpellShapeshiftEntry` | `ShapeshiftMask[2]`, `ShapeshiftExclude[2]` | `Stances`, `StancesNot` (as `u64` via `MAKE_PAIR64`) |
| `SpellTargetRestrictionsEntry` | `ConeDegrees`, `Width`, `Targets`, `TargetCreatureType`, `MaxTargets`, `MaxTargetLevel` | `ConeAngle`, `Width`, `Targets`, `TargetCreatureType`, `MaxAffectedTargets`, `MaxTargetLevel` |
| `SpellTotemsEntry` | `RequiredTotemCategoryID[2]`, `Totem[2]` | `TotemCategory: [u16; 2]`, `Totem: [i32; 2]` |
| `SpellXSpellVisualEntry[]` | Sorted-by-`CasterPlayerConditionID` visual list | `_visuals: SpellVisualVector` |
| `SpellEffectEntry` (per slot) | All ~33 columns per effect | `_effects: Vec<SpellEffectInfo>` |
| `SpellRangeEntry` (lazy via pointer) | `RangeMin[2]`, `RangeMax[2]`, `Flags` | Read at `GetMaxRange` time, not stored |
| `SpellRadiusEntry` (lazy via pointer per effect) | `Radius`, `RadiusPerLevel`, `RadiusMax` | Stored as `TargetARadiusEntry`, `TargetBRadiusEntry` per effect |
| `SpellCastTimesEntry` (lazy via pointer) | `Base`, `PerLevel`, `Minimum` | Stored as `CastTimeEntry`; read at `CalcCastTime` time |
| `SpellDurationEntry` (lazy via pointer) | `Duration`, `DurationPerLevel`, `MaxDuration` | Stored as `DurationEntry` |

**SQL overlays** (read by `SpellMgr`, then merged into `SpellInfo` mutation):
- `serverside_spell_effect` (33 columns) — only for **server-only** spells; cannot overlay an existing DB2 spell
- `spell_custom_attr` — overrides `AttributesCu`
- `LoadSpellInfoCorrections` (hand-coded, no SQL) — ~1,500 manual `ApplySpellFix` lambdas (see `spells-mgr.md` §4)

> **`spell_dbc` legacy note:** the user-task brief mentions an SQL `spell_dbc` overlay for patching DB2 fields. In TrinityCore Wrath Classic 3.4.3.54261 this role is split across `serverside_spell_effect` (server-only spells) + the **hotfixes DB** (overlay of real DB2 spells, applied transparently inside `DB2Storage<T>::Load` — see `shared-datastores.md`). `SpellInfo` itself sees only the final merged data after all overlays.

---

## 7. Wire-protocol packets (if any)

`SpellInfo` is server-internal; it emits no packets. However, every spell-related opcode reads it: `SMSG_SPELL_START` reads `CastTimeEntry`, `SMSG_SPELL_GO` reads `Speed`/`LaunchDelay` for missile travel time, `SMSG_AURA_UPDATE` reads `StackAmount` and `ProcCharges`, `SMSG_PERIODIC_AURA_LOG` reads effect periodic amounts, `SMSG_SPELL_COOLDOWN` reads `RecoveryTime`/`CategoryRecoveryTime`/`StartRecoveryTime`, `SMSG_LEARNED_SPELL` checks `IsAutocastable`/chain. The full list is in `spells.md`.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-spell/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `crates/wow-spell/src/diminish` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-spell/src/immunity` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 lines** (verified `wc -l`). No `SpellInfo` struct, no `SpellEffectInfo`, no `SpellImplicitTargetInfo`, no `SpellPowerCost`, no `SpellDiminishInfo`, no `SpellSpecificType`, no `SpellCustomAttributes`, no `SpellTarget*` enums.
- `crates/wow-data/src/spell.rs` — exposes a tiny stub `SpellInfo` consumed by `crates/wow-world/src/handlers/spell.rs`. Fields confirmed: `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, plus method `has_cast_time()`. No attribute bitmasks, no effect array, no school mask, no mechanic, no range, no power costs, no reagents, no totems, no equipped items, no target restrictions, no levels, no scaling, no interrupts, no proc data.
- `crates/wow-data/src/wdc4.rs` — generic `Wdc4Reader`; can read any DB2 if a struct mapping is provided.
- `crates/wow-data/src/hotfix_cache.rs` — generic hotfix overlay framework.

**What's implemented:**
- `wow_data::SpellInfo` stub provides the bare minimum for the cast handler to compute "is this an instant?" and "is this on cooldown?". Approximately 4 fields out of ~80 on the C++ class.
- DB2 binary parsing for `Spell.db2` itself only (the `SpellNameStore` equivalent — id + name). None of the 19 sibling stores enumerated in §6 have a Rust reader.

**What's missing vs C++:**
1. The whole `SpellInfo` type — fields and methods.
2. The `SpellEffectInfo` nested type. Without it, the per-effect array is impossible.
3. The 15 `Attributes`/`AttributesEx`..`AttributesEx14` `u32` bitmask fields plus `AttributesCu`. Without them, `HasAttribute(SpellAttr0::Passive)` and the ~450 attribute predicates that depend on it cannot exist.
4. `SpellAttr0..SpellAttr14` `bitflags!` definitions (15 separate enum bitmasks of 32 bits each).
5. `SpellCustomAttributes` enum bitmask (25 bits).
6. The full `SpellEffectName` enum (~151 variants) — without it, `HasEffect` is impossible.
7. The full `AuraType` enum (~280 variants) — without it, `HasAura` is impossible.
8. The `Mechanics`/`DispelType`/`SpellSchoolMask`/`SpellInterruptFlags`/`SpellAuraInterruptFlags`/`SpellAuraInterruptFlags2`/`AuraStateType` enums.
9. `SpellImplicitTargetInfo` + the static `_data: array<StaticData, TOTAL_SPELL_TARGETS>` lookup that classifies every `Targets` enum value into `(ObjectType, ReferenceType, SelectionCategory, SelectionCheckType, DirectionType)`.
10. The `Targets` enum (~150 implicit target type values, e.g. `TARGET_UNIT_TARGET_ENEMY`, `TARGET_DEST_DYNOBJ_ALLY`, `TARGET_SRC_CASTER`, `TARGET_DEST_DB`).
11. ~150 `SpellInfo` const methods (predicates, validators, calc helpers, chain navigation).
12. `_LoadSpellSpecific` (188 lines), `_LoadAuraState`, `_LoadSpellDiminishInfo` (471 lines), `_LoadImmunityInfo` (313 lines), `_InitializeExplicitTargetMask`, `_InitializeSpellPositivity` (39 lines).
13. The 2 ctors (full and effect-only) that compose from the ~20 sibling DB2 records.
14. `flag128` newtype for `SpellFamilyFlags` and per-effect `SpellClassMask`.
15. `EnumFlag<T>`-equivalent typed bit-set wrapper for the 5 interrupt flag fields.
16. `SpellPowerCost` / `SpellDiminishInfo` / `ImmunityInfo` structs.
17. The 4 chain-navigation methods (`GetFirstRankSpell`, `GetLastRankSpell`, `GetNextRankSpell`, `GetPrevRankSpell`, `GetAuraRankForLevel`) which require `ChainEntry: Option<Arc<SpellChainNode>>` (lives in `spells-mgr.md`).
18. The `IsAffected(familyName, flag128)` family-flag matcher used by talent affecter logic.
19. `IsItemFitToSpellRequirements(item)` reagent + equipped-item check.
20. `Check*` validators (CheckShapeshift, CheckLocation, CheckTarget, CheckExplicitTarget, CheckVehicle).
21. `CalcPowerCost` (3 overloads), `CalcCastTime`, `CalcDuration`, `CalcProcPPM`, `GetMaxTicks`, `GetMaxRange`/`GetMinRange`, `GetSpellXSpellVisualId`.
22. The `Labels: HashSet<u32>` field + `HasLabel` predicate.
23. `MeetsFutureSpellPlayerCondition(player)` — UI gating predicate.

**Suspicious / likely divergent (hipótesis pre-implementación):**
- `SpellInfo` should be `Arc<SpellInfo>` and shared across all maps and `Spell`/`Aura` instances. **Never** clone; always pass by `&Arc<SpellInfo>`.
- The `Difficulty` keying (one `SpellInfo` per `(spellId, difficulty)`) means a single spellId can have multiple `Arc<SpellInfo>` — be careful: equality is `Arc::ptr_eq`, NOT comparison of `Id` field, when checking "is this the same spell instance".
- `_effects: Vec<SpellEffectInfo>` is reserved at `32` capacity in C++ but in WoLK 3.4.3 only 3 effect slots are used. Use `SmallVec<[SpellEffectInfo; 3]>` to avoid heap allocation per spell load.
- The `EnumFlag<SpellInterruptFlags>` typed wrapper from C++ should be `bitflags!{}` in Rust. Bit-test calls like `info.aura_interrupt_flags.has_flag(...)` translate to `info.aura_interrupt_flags.contains(...)`.
- `flag128` as `[u32; 4]` newtype with custom `Hash`/`Eq` — the C++ `flag128::operator&` is bitwise AND across all 4 words; Rust port should provide `intersects(other: &Flag128) -> bool` and `&` operator.
- `_LoadSpellDiminishInfo` (471 lines of hand-coded family-flag pattern matching) and `_LoadImmunityInfo` (313 lines) cannot be ported as monolithic match expressions. Split into `crates/wow-spell/src/diminish/` and `crates/wow-spell/src/immunity/` with one Rust file per spell-family group.

**Tests existing:** **0** in `crates/wow-spell/`. None covering `SpellInfo` field decoding, attribute predicates, or the multi-store join.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SPELLS_INFO.WBS.001** Partir y cerrar la migracion auditada de `game/Spells/SpellInfo.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 5022 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_INFO.WBS.002** Partir y cerrar la migracion auditada de `game/Spells/SpellInfo.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 625 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for `MIGRATION_ROADMAP.md` cross-reference. Complexity: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#SPELLINFO.1** Create `crates/wow-spell/src/spell_info.rs` with the `SpellInfo` struct skeleton (~80 fields), `Arc`-shareable, all-pub immutable API (M)
- [ ] **#SPELLINFO.2** Define `bitflags! struct SpellAttr0..SpellAttr14` (15 separate 32-bit bitflags; ~450 individual flags) — port from `SharedDefines.h` (H)
- [ ] **#SPELLINFO.3** Define `bitflags! struct SpellCustomAttributes` (25 bits; live `AttributesCu`) — port from `SpellInfo.h:146` (L)
- [ ] **#SPELLINFO.4** Define `enum SpellEffectName` with `#[repr(u32)]` — ~151 variants from `SharedDefines.h` (M)
- [ ] **#SPELLINFO.5** Define `enum AuraType` with `#[repr(u32)]` — ~280 variants from `Auras/SpellAuraDefines.h` (M)
- [ ] **#SPELLINFO.6** Define `enum Mechanics`, `enum DispelType`, `bitflags! SpellSchoolMask`, `bitflags! SpellInterruptFlags`, `bitflags! SpellAuraInterruptFlags`, `bitflags! SpellAuraInterruptFlags2`, `enum AuraStateType` (M)
- [ ] **#SPELLINFO.7** Define `enum Targets` (~150 implicit target type values from `SharedDefines.h`); add a `const STATIC_DATA: [TargetStaticData; TOTAL_SPELL_TARGETS]` table mirroring C++ `SpellImplicitTargetInfo::_data` (H)
- [ ] **#SPELLINFO.8** Define `struct SpellImplicitTargetInfo(Targets)` with all 6 classifier methods (`is_area`, `selection_category`, `reference_type`, `object_type`, `check_type`, `direction_type`) backed by the static data (M)
- [ ] **#SPELLINFO.9** Define `struct SpellEffectInfo` with all ~28 fields + nested `Scaling` + `Option<Arc<ImmunityInfo>>` (H)
- [ ] **#SPELLINFO.10** Implement `SpellEffectInfo::new(spell_info: &SpellInfo, effect: &SpellEffectEntry)` constructor mirroring C++ ctor at `SpellInfo.cpp:407` (M)
- [ ] **#SPELLINFO.11** Implement `SpellEffectInfo` predicates: `is_effect`, `is_aura`, `is_targeting_area`, `is_area_aura_effect`, `is_unit_owned_aura_effect` (L)
- [ ] **#SPELLINFO.12** Implement `SpellEffectInfo::calc_value`, `calc_base_value`, `calc_value_multiplier`, `calc_damage_multiplier`, `has_radius`, `calc_radius` (H)
- [ ] **#SPELLINFO.13** Implement `SpellEffectInfo::get_provided_target_mask`, `get_missing_target_mask`, `get_implicit_target_type`, `get_used_target_object_type`, `get_scaling_expected_stat` (M)
- [ ] **#SPELLINFO.14** Define `Flag128([u32; 4])` newtype with `Hash + Eq + Copy + Debug + bit AND/OR/intersects/all_zero` (L)
- [ ] **#SPELLINFO.15** Define `struct SpellPowerCost { power: Powers, amount: i32 }` and `struct SpellDiminishInfo { group, ret_type, max_level, duration_limit }` (L)
- [ ] **#SPELLINFO.16** Implement `SpellInfo::from_load_helper(spell_name: &SpellNameEntry, difficulty: Difficulty, helper: &SpellInfoLoadHelper)` mirroring C++ `SpellInfo` ctor at `SpellInfo.cpp:1144` — copy fields from each populated DB2 slot (H)
- [ ] **#SPELLINFO.17** Implement `SpellInfo::from_serverside_effects(spell_name: &SpellNameEntry, difficulty: Difficulty, effects: &[SpellEffectEntry])` (effect-only ctor) (M)
- [ ] **#SPELLINFO.18** Implement all 16 `has_attribute` overloads (`has_attribute_0(SpellAttr0) … has_attribute_14(SpellAttr14)` + `has_custom_attribute(SpellCustomAttributes)`) (L — but mechanical)
- [ ] **#SPELLINFO.19** Implement effect/aura search predicates: `has_effect`, `has_aura`, `has_area_aura_effect`, `has_only_damage_effects`, `has_target_type` (L)
- [ ] **#SPELLINFO.20** Implement interrupt-flag predicates (4 of them) (L)
- [ ] **#SPELLINFO.21** Implement classification predicates: `is_passive`, `is_autocastable`, `is_channeled`, `is_move_allowed_channel`, `is_next_melee_swing_spell`, `is_ranged_weapon_spell`, `is_auto_repeat_ranged_spell`, `is_cooldown_started_on_event`, `is_death_persistent`, `is_requiring_dead_target`, `is_allowing_dead_target`, `is_explicit_discovery`, `is_loot_crafting`, `is_profession`, `is_primary_profession`, `is_primary_profession_first_rank`, `is_ability_of_skill_type`, `is_affecting_area`, `is_targeting_area`, `needs_explicit_unit_target` (M — mostly bit tests)
- [ ] **#SPELLINFO.22** Implement stacking predicates: `is_stackable_with_ranks`, `is_passive_stackable_with_ranks`, `is_multi_slot_aura`, `is_stackable_on_one_slot_with_different_casters`, `is_single_target` (L)
- [ ] **#SPELLINFO.23** Implement `is_aura_exclusive_by_specific_with(other)` and `is_aura_exclusive_by_specific_per_caster_with(other)` (M — 32-line C++ each)
- [ ] **#SPELLINFO.24** Implement `is_positive` + per-effect `is_positive_effect(eff_idx)` + `_initialize_spell_positivity` post-processor (M)
- [ ] **#SPELLINFO.25** Implement chain navigation: `is_ranked`, `get_rank`, `get_first_rank_spell`, `get_last_rank_spell`, `get_next_rank_spell`, `get_prev_rank_spell`, `get_aura_rank_for_level`, `is_rank_of`, `is_different_rank_of`, `is_high_rank_of` — all reading `chain_entry: Option<Arc<SpellChainNode>>` populated by `SpellMgr::LoadSpellRanks` (M)
- [ ] **#SPELLINFO.26** Implement `is_item_fit_to_spell_requirements(item: &Item) -> bool` (M)
- [ ] **#SPELLINFO.27** Implement `is_affected(family_name: u32, family_flags: &Flag128) -> bool` for talent affecter logic (L)
- [ ] **#SPELLINFO.28** Implement `is_affected_by_spell_mods` / `is_affected_by_spell_mod(mod)` (M)
- [ ] **#SPELLINFO.29** Implement `can_pierce_immune_aura(other)`, `can_dispel_aura(other)`, `can_spell_provide_immunity_against_aura(other)`, `spell_cancels_aura_effect(aura_eff)`, `can_be_interrupted(interrupt_caster, interrupt_target, ignore_immunity)` (H)
- [ ] **#SPELLINFO.30** Implement validators returning `SpellCastResult`: `check_shapeshift`, `check_location`, `check_target`, `check_explicit_target`, `check_vehicle`, `check_target_creature_type` (H)
- [ ] **#SPELLINFO.31** Implement Calc helpers: `calc_duration`, `get_duration`, `get_max_duration`, `get_max_ticks`, `calc_cast_time`, `get_recovery_time` (M)
- [ ] **#SPELLINFO.32** Implement `calc_power_cost(power_type, optional_cost, caster, school_mask, spell)` (single power, the simpler overload) (M)
- [ ] **#SPELLINFO.33** Implement `calc_power_cost(caster, school_mask, spell)` returning `Vec<SpellPowerCost>` (multi-power) (M)
- [ ] **#SPELLINFO.34** Implement `calc_proc_ppm(caster, item_level)` evaluating `ProcPPMMods` per condition (H)
- [ ] **#SPELLINFO.35** Implement range queries: `get_min_range(positive)`, `get_max_range(positive, caster, spell)` reading `RangeEntry` (M)
- [ ] **#SPELLINFO.36** Implement mechanic queries: `get_school_mask`, `get_all_effects_mechanic_mask`, `get_effect_mechanic_mask(eff_idx)`, `get_spell_mechanic_mask_by_effect_mask(mask)`, `get_effect_mechanic(eff_idx)`, `get_dispel_mask` (instance + static) (M)
- [ ] **#SPELLINFO.37** Implement `_load_spell_specific` (188 lines C++) — set `_spell_specific: SpellSpecificType` from family-flag + effect-type pattern matching. Split into `corrections`/`spell_specific.rs` if too large for one fn (H)
- [ ] **#SPELLINFO.38** Implement `_load_aura_state` — set `_aura_state: AuraStateType` from family flags (M)
- [ ] **#SPELLINFO.39** Implement `_load_spell_diminish_info` (471 lines C++) — split per spell-family into `crates/wow-spell/src/diminish/{warrior,mage,priest,…}.rs`; each file contributes pattern-match arms via a registry (XL)
- [ ] **#SPELLINFO.40** Implement `_load_immunity_info` (313 lines C++) — per-effect; split similar to #SPELLINFO.39 into `crates/wow-spell/src/immunity/` (XL)
- [ ] **#SPELLINFO.41** Implement `apply_all_spell_immunities_to(target, effect, apply)` walking computed `ImmunityInfo` (H)
- [ ] **#SPELLINFO.42** Implement `_initialize_explicit_target_mask` — caches `explicit_target_mask` from each effect's TargetA/TargetB (L)
- [ ] **#SPELLINFO.43** Implement `meets_future_spell_player_condition(player)` UI predicate (L)
- [ ] **#SPELLINFO.44** Implement `get_spell_x_spell_visual_id(caster, viewer)` and `get_spell_visual(caster, viewer)` reading the sorted visuals list (M)
- [ ] **#SPELLINFO.45** Implement `get_attack_type` mapping `EquippedItemSubClassMask` → `WeaponAttackType` (L)
- [ ] **#SPELLINFO.46** Add `Labels: HashSet<u32>` field + `has_label(label_id)` (L)
- [ ] **#SPELLINFO.47** Implement `negative_effects: Bitset<MAX_SPELL_EFFECTS>` field + `is_positive_effect` per-slot read (L)
- [ ] **#SPELLINFO.48** Implement `_unload_implicit_target_condition_lists` (called once at shutdown) (L)
- [ ] **#SPELLINFO.49** Replace the stub `wow_data::SpellInfo` with a re-export of the real type from `wow-spell`; migrate `crates/wow-world/src/handlers/spell.rs` callers (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SPELLS_INFO.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 5647 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h`. Rust target: `crates/wow-spell`. | `cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SPELLS_INFO.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 5647 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h`. Rust target: `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SPELLS_INFO.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 5647 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h`. Rust target: `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SPELLS_INFO.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 5647 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h`. Rust target: `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `SpellInfo` fully composed from a real `Spell.db2` + `SpellMisc.db2` + `SpellEffect.db2[]` for Pyroblast (id 11366) matches C++ field values (school_mask = `Fire`, mechanic = `None`, attributes_ex contains `IS_NEXT_RANK`, effects[0].effect = `SCHOOL_DAMAGE`, effects[0].base_points matches, effects[1].effect = `APPLY_AURA` with `PERIODIC_DAMAGE`)
- [ ] Test: All 16 `has_attribute` overloads correctly bit-test each of the 15 `Attributes*` fields + `AttributesCu`
- [ ] Test: `is_positive` returns true for Renew (139), Power Word: Shield (17), Holy Light (635) and false for Curse of Agony (980), Polymorph (118), Frostbolt (116) — matches `_initialize_spell_positivity`
- [ ] Test: `is_passive` matches `HasAttribute(SpellAttr0::Passive)` for known passive spells (auras innate to a class)
- [ ] Test: `is_channeled` returns true for Mind Flay (15407), Drain Life (689), Arcane Missiles (5143)
- [ ] Test: `get_max_range(positive=false, caster=None, spell=None)` for Frostbolt returns 30 yards (`SpellRange.db2` row 5)
- [ ] Test: `calc_cast_time` for Pyroblast at base = 6000 ms (no caster mods)
- [ ] Test: `calc_power_cost(Mana, optional=false, caster=warlock_lvl70, schoolmask=Shadow, spell=None)` returns the expected mana value matching C++ `CalcPowerCost`
- [ ] Test: `calc_proc_ppm(caster=warrior_with_weapon_speed_3.6, itemLevel=200)` for Windfury returns `base_ppm * 3.6/60` after PpmMods evaluation
- [ ] Test: `get_dispel_mask` for a spell with `Dispel = Magic` returns `1 << DispelType::Magic`
- [ ] Test: `is_aura_exclusive_by_specific_with` for two Hunter Stings (Serpent + Viper) returns true (both `SpellSpecific::Sting`)
- [ ] Test: `can_dispel_aura` for `Dispel Magic` (527) on `Curse of Agony` (980) returns false (Magic mask doesn't include Curse)
- [ ] Test: `can_pierce_immune_aura` for spell with `SPELL_ATTR1_IGNORE_INVULNERABILITY` returns true vs Divine Shield
- [ ] Test: `check_shapeshift(form=Bear)` returns `SPELL_FAILED_NOT_SHAPESHIFT` for a humanoid-only spell
- [ ] Test: `check_location(map=Outland, zone=, area=)` returns `SPELL_FAILED_INCORRECT_AREA` for a spell with `RequiredAreasID` excluding Outland
- [ ] Test: `chain_entry` set via `SpellMgr::LoadSpellRanks` correctly resolves `get_first_rank_spell` for Frostbolt rank 5 → rank 1 (id 116)
- [ ] Test: Multi-power spell (Holy Power + Mana) returns 2 entries from `calc_power_cost(caster, schoolmask, spell)`
- [ ] Test: `get_max_ticks` for a 30s DoT with periodic_amplitude 3000ms returns 10
- [ ] Test: `_load_spell_diminish_info` classifies Polymorph (118) into `DIMINISHING_DISORIENT` group, type `DRTYPE_PLAYER`, max level `DIMINISHING_LEVEL_IMMUNE`
- [ ] Test: `_load_immunity_info` for Divine Shield (642) computes school immunity mask = all schools
- [ ] Test: Two `SpellInfo` instances for the same `spellId` but different `Difficulty` are not `Arc::ptr_eq` and may have different `effects[0].base_points`
- [ ] Test: `is_affected(family_name=Mage, family_flags=[FrostFamily])` returns true for Frostbolt rank 1
- [ ] Test: `has_label(label_id=42)` returns true after parsing a `SpellLabel.db2` row pointing to spellId
- [ ] Test: `serialize_to_smsg_db_reply` round-trips SpellInfo bytes match C++ `WriteRecord` output (cross-check via captured client reply packet)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 5647 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `crates/wow-spell/src/spell_info.rs` (planned), `crates/wow-data/src/spell.rs` (DB2 readers — partial) \| ❌ not started — type does not exist; only a tiny stub `wow_data::SpellInfo` (cast_time + cooldown) is in use. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SPELLS_INFO.DIV.001` | `crates/wow-spell/src/spell_info.rs` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 5647 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SPELLS_INFO.DIV.002` | `crates/wow-spell/src/lib.rs` (`exists_empty`, 0 Rust lines) | 2 C++ files / 5647 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SPELLS_INFO.DIV.003` | `crates/wow-spell/src/diminish` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 5647 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SPELLS_INFO.DIV.004` | `crates/wow-spell/src/immunity` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 5647 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SPELLS_INFO.DIV.005` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 2 C++ files / 5647 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`SpellInfo` is composed, not stored.** No single DB2 file holds a full SpellInfo. `Spell.db2` (the misleadingly-named `SpellNameStore`) only holds id + localized name. Every other field comes from a sibling DB2 joined on `(SpellID[, DifficultyID])`. Beginners often look at `Spell.db2` and conclude RustyCore "has SpellInfo" because it parses that file — it does not. The composition step (`LoadSpellInfoStore`) is what makes a `SpellInfo`. Without the cross-table join + difficulty fallback, the parsed `Spell.db2` rows are nearly useless on their own.
- **15 attribute bitmasks (not 14, not 8).** WoLK 3.4.3.54261 has `Attributes` + `AttributesEx` + `AttributesEx2` … through `AttributesEx14`, totaling **15** 32-bit fields = 480 individual attribute bits. `SpellMisc.db2` provides them as a `int32 Attributes[15]` array. Plus `AttributesCu` (server-derived). The user-task brief mentions "8 32-bit attribute bitmasks" — that's a Vanilla/TBC-era count; **WoLK is 15** for `AttributesEx` series and modern TrinityCore Wrath Classic uses 15. Verify against `SpellMisc.db2` schema in `DB2Structure.h` before settling field count.
- **Per-effect array max in WoLK 3.4.3 is 3, not 32.** `MAX_SPELL_EFFECTS` was raised to 32 in modern DF builds; in WoLK 3.4.3 only 3 effect slots are ever populated. C++ pre-allocates `_effects.reserve(32)` defensively, but in practice `_effects.size() <= 3`. Use `SmallVec<[SpellEffectInfo; 3]>` or fixed `[Option<SpellEffectInfo>; 3]` in Rust to skip heap allocs.
- **Per-effect target slots A/B + per-target radius A/B.** Each effect has 2 implicit target types (`TargetA`, `TargetB`) and 2 radius entries (`TargetARadiusEntry`, `TargetBRadiusEntry`). `TargetA` is the primary; `TargetB` typically chains or extends. Confusing at first because the field names look symmetric; semantics are not.
- **`Targets` enum has ~150 valid values but the static-data table has TOTAL_SPELL_TARGETS slots.** `SpellImplicitTargetInfo::_data` is sized `TOTAL_SPELL_TARGETS` (slightly larger than 150 to allow padding). Many slots are sentinel `(NONE, NONE, NYI, …)` for unimplemented target types. Never index without a bounds check; `assert_spell_info` patterns from `SpellMgr` are a good model.
- **`SpellEffectInfo::ImplicitTargetConditions` is a `shared_ptr<vector<Condition>>`.** Multiple effects across multiple SpellInfos can share the same Condition list (memory optimization for boss scripts that check the same conditions for many adds). Use `Arc<Vec<Condition>>` in Rust. The `_unload_implicit_target_condition_lists` post-shutdown step is needed because `Conditions` module can be torn down before `SpellMgr` clears its maps.
- **`ProcFlags` field on SpellInfo is `ProcFlagsInit`,** which is a packed `(ProcFlags, ProcFlags2)` pair (uint32 + int32). Don't confuse with `SpellProcEntry::ProcFlags` (lives on a separate per-rule struct in `SpellMgr`). The two can override each other: `SpellAuraOptions.ProcTypeMask` populates `SpellInfo::ProcFlags`; `spell_proc.ProcFlags` SQL row populates `SpellProcEntry::ProcFlags` and **overrides** `SpellInfo::ProcFlags` if both are present.
- **`StackAmount` (in C++) = `CumulativeAura` (in DB2 column name).** Don't search for "StackAmount" in DB2 schemas; the DB2 column is `CumulativeAura`. The `SpellInfo` field name preserves Vanilla-era naming.
- **`Stances` and `StancesNot` are `u64` packed from `u32[2]` arrays.** `SpellShapeshift.db2` has `ShapeshiftMask[2]` and `ShapeshiftExclude[2]` (low + high words). C++ uses `MAKE_PAIR64(low, high)` to pack. In Rust: `((high as u64) << 32) | (low as u64)`.
- **`PowerCosts` is an array, not a single value.** In Vanilla→TBC, a spell had a single `manaCost`. In WoLK, `SpellPower.db2` allows up to `MAX_POWERS_PER_SPELL` rows per spell (4 in 3.4.3.54261; 6 in modern DF). Each row has its own `PowerType` (Mana/Rage/Energy/Runic/HolyPower/…) and cost formula. `CalcPowerCost(caster, schoolMask, spell)` returns a `vector<SpellPowerCost>` because some spells (Death Knight Death Strike) cost both runes and runic power.
- **`SpellPowerDifficulty` re-keys SpellPower rows.** For difficulty-keyed spells, `SpellPower.OrderIndex` alone isn't unique — `sSpellPowerDifficultyStore.LookupEntry(power->ID)` re-routes the row to a `(spellId, difficulty, orderIndex)` triple. The Rust port must respect this remapping in `LoadSpellInfoStore`.
- **Difficulty fallback fills slot-by-slot, not record-by-record.** When `(spellId, Mythic)` lacks a `SpellMisc` row, the loader walks `DifficultyEntry::FallbackDifficultyID` (Mythic→Heroic→Normal→None) and fills only the missing field. Different fields can come from different difficulty levels in the same final SpellInfo.
- **`_visuals` does NOT cascade through the full fallback chain.** Visuals fall back only to the *first* difficulty in the chain that defines any visual; further fallbacks are ignored. Asymmetric vs all other fields.
- **`_load_immunity_info` / `_load_spell_diminish_info` are post-construction.** The ctor runs first and sets all DB2-derived fields. Then `SpellMgr::LoadSpellInfoImmunities` and `LoadSpellInfoDiminishing` walk every loaded SpellInfo and call these `_load*` mutator methods to compute derived data. This is the only mutation path after ctor; everything else is `const`.
- **`_load_spell_specific` is a 188-line if/else cascade by family + effect pattern.** Don't try to express it as a single Rust `match`. Split per spell-family into a registry of `fn classify(spell_info: &SpellInfo) -> Option<SpellSpecificType>` functions, return `Some` from the first match.
- **`_load_spell_diminish_info` is 471 lines of similar.** Even worse than `_load_spell_specific`. Same strategy: per-family registry.
- **Diminishing returns is global state, not per-SpellInfo.** `SpellInfo` only carries the *category* (group + type + max level + duration limit). The actual DR tracker (with timers, per-target stacks) lives on `Unit`. Don't conflate the two.
- **`ChainEntry` is `nullptr` for non-ranked spells.** The chain pointer is only set if `SpellMgr::LoadSpellRanks` finds the spell in `mSpellChains`. ~70% of spells are not ranked. All chain-navigation methods must handle `Option<&SpellChainNode>` gracefully.
- **`IsAffected(family, flag128)` is the talent-affecter primitive.** Talents declare "affects spells of family Mage with mask 0x0000_0010_0000_0000" — that mask AND-tests against `SpellInfo::SpellFamilyFlags`. If the test passes, the talent's modifier applies. Bug-prone: the family must match exactly; family `0` (no family) means "affects nothing", not "affects everything".
- **`IsItemFitToSpellRequirements` walks `EquippedItemClass` + `EquippedItemSubClassMask` + `EquippedItemInventoryTypeMask` against the equipped weapon.** A spell can be limited to "only with axes" (subclass mask = 1<<axe) or "only with main-hand items" (inventory type mask = 1<<MAINHAND). Combo logic is AND across all three.
- **Performance:** every cast calls ~10-20 `SpellInfo` predicates. Make all of them `#[inline]` and prefer `bitflags!` over enum matches. The 80-field struct is ~600 bytes; passing by `&Arc<SpellInfo>` is critical. Don't `derive(Clone)` — the type must not be cloned at runtime.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class SpellInfo` | `pub struct SpellInfo` (in `crates/wow-spell/src/spell_info.rs`) | All fields `pub`; immutable after `_load_*` post-processors finish; shared via `Arc<SpellInfo>` |
| `SpellInfo const* SpellMgr::GetSpellInfo(...)` | `&Arc<SpellInfo>` (returned via `Option<&Arc<SpellInfo>>`) | Never clone; never `&mut`. |
| 15 × `uint32 Attributes / AttributesEx / … / AttributesEx14` | 15 × `bitflags! struct SpellAttr0..SpellAttr14` `u32` fields | One `bitflags!` invocation per group |
| `uint32 AttributesCu` | `bitflags! struct SpellCustomAttributes: u32` | 25 bits; populated by `LoadSpellInfoCustomAttributes` |
| `bool HasAttribute(SpellAttr0 attr)` (15 overloads) | `fn has_attribute_0(&self, attr: SpellAttr0) -> bool { self.attributes.contains(attr) }` (15 methods) | `#[inline]` |
| `flag128 SpellFamilyFlags` | `Flag128([u32; 4])` | newtype with `Hash + Eq + Copy`; intersect/AND/OR ops |
| `class SpellEffectInfo` | `struct SpellEffectInfo` (in same file or `effect_info.rs`) | `Vec<SpellEffectInfo>` field on `SpellInfo`; ideally `SmallVec<[SpellEffectInfo; 3]>` |
| `class SpellImplicitTargetInfo` | `struct SpellImplicitTargetInfo(Targets)` | All classifier methods read from `static STATIC_DATA: [TargetStaticData; TOTAL_SPELL_TARGETS]` |
| `enum SpellEffectName` (~151 values) | `#[repr(u32)] enum SpellEffect { … }` with `#[non_exhaustive]` | Numeric values preserved |
| `enum AuraType` (~280 values) | `#[repr(u32)] enum AuraType { … }` with `#[non_exhaustive]` | — |
| `EnumFlag<SpellInterruptFlags>` | `bitflags! struct SpellInterruptFlags: u32` field | `.has_flag(...)` → `.contains(...)` |
| `EnumFlag<SpellAuraInterruptFlags>` / `EnumFlag<SpellAuraInterruptFlags2>` | Two separate `bitflags!` structs | Two fields each for normal + channel variants |
| `array<SpellPowerEntry const*, MAX_POWERS_PER_SPELL>` | `[Option<Arc<SpellPowerEntry>>; MAX_POWERS_PER_SPELL]` | 4 in 3.4.3 |
| `array<int32, MAX_SPELL_TOTEMS>` / `array<uint16, …>` | `[i32; MAX_SPELL_TOTEMS]` / `[u16; MAX_SPELL_TOTEMS]` | 2 in 3.4.3 |
| `array<int32, MAX_SPELL_REAGENTS>` / `array<int16, …>` | `[i32; 8]` / `[i16; 8]` | 8 reagents max |
| `bitset<MAX_SPELL_EFFECTS> NegativeEffects` | `u32` (3 bits used) or small `BitSet` | — |
| `unordered_set<uint32> Labels` | `HashSet<u32>` | — |
| `vector<Condition>` (per effect) | `Arc<Vec<Condition>>` (shared across effects/SpellInfos when conditions are deduped) | Use `Arc::clone` only at construction |
| `void _LoadSpellSpecific()` / `_LoadAuraState()` / `_LoadSpellDiminishInfo()` / `_LoadImmunityInfo()` | `pub(crate) fn _load_*(&mut self)` called by `SpellMgr` post-loaders only | Mark `&mut self`; non-public |
| `SpellCastResult CheckShapeshift(uint32 form)` | `fn check_shapeshift(&self, form: ShapeshiftForm) -> Result<(), SpellCastResult>` | Convert `Ok` ↔ `SPELL_CAST_OK` |
| `Optional<SpellPowerCost> CalcPowerCost(...)` | `fn calc_power_cost(&self, …) -> Option<SpellPowerCost>` | `Optional<T>` → `Option<T>` |
| `vector<SpellPowerCost> CalcPowerCost(...)` | `fn calc_power_cost_all(&self, …) -> Vec<SpellPowerCost>` | Different name to disambiguate from single-power overload |
| `SpellInfo const* GetFirstRankSpell()` | `fn get_first_rank_spell(&self) -> Option<&Arc<SpellInfo>>` | Reads `chain_entry` |
| `bool IsAffected(uint32 familyName, flag128 const& familyFlags)` | `fn is_affected(&self, family_name: u32, family_flags: &Flag128) -> bool` | Pass `Flag128` by reference |
| `void ApplyAllSpellImmunitiesTo(Unit*, SpellEffectInfo const&, bool apply)` | `fn apply_all_spell_immunities_to(&self, target: &mut Unit, effect: &SpellEffectInfo, apply: bool)` | — |
| `LocalizedString const* SpellName` | `Arc<LocalizedString>` (or `&'static LocalizedString` if interned) | Localized name lookup at packet send |
| `SpellChainNode const* ChainEntry` | `Option<Arc<SpellChainNode>>` | Set by `SpellMgr::LoadSpellRanks`; lives in `spells-mgr.md` |
| `bool MeetsFutureSpellPlayerCondition(Player const* player)` | `fn meets_future_spell_player_condition(&self, player: &Player) -> bool` | UI gating |

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical at `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellInfo.{h,cpp}` (625 + 5,022 lines = 5,647 total) against Rust workspace at `/home/server/rustycore/crates/`.

**Empty-target finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` is 0 bytes (verified `wc -l`). No `SpellInfo` struct, no `SpellEffectInfo`, no `SpellImplicitTargetInfo`, no `SpellPowerCost`, no `SpellDiminishInfo`. The 5,647 lines of C++ map to **zero lines** of Rust.

**Stub status of `wow_data::SpellInfo`.** A minimal `SpellInfo` POD exists in `crates/wow-data/src/spell.rs` exposing `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, `has_cast_time()`. **4 fields out of ~80** on the C++ class. Used by `crates/wow-world/src/handlers/spell.rs` to determine "is this an instant cast" and "is this still on per-spell cooldown". Lacks every other dimension.

**Attribute bitmask coverage.** **0 of 15** bitmasks defined. The C++ `Attributes`, `AttributesEx`, `AttributesEx2..AttributesEx14`, and `AttributesCu` (16 total `u32` fields) have no Rust analog. Without these, `HasAttribute(SpellAttr0::Passive)` and the ~480 individual attribute bit predicates that the entire engine depends on cannot exist. `IsPassive()` → broken. `IsChanneled()` → broken. `IsAutoRepeatRangedSpell()` → broken. `IsCooldownStartedOnEvent()` → broken. The rest of `spells.md`'s engine is built on these primitives.

**Effect array coverage.** **0 of 3 effect slots** populated. `SpellEffectInfo` does not exist. The per-effect fields `Effect: SpellEffectName`, `ApplyAuraName: AuraType`, `BasePoints`, `RealPointsPerLevel`, `PointsPerResource`, `Amplitude`, `MiscValue`, `MiscValueB`, `Mechanic`, `TargetA`, `TargetB`, `TargetARadiusEntry`, `TargetBRadiusEntry`, `ChainTargets`, `DieSides`, `ItemType`, `TriggerSpell`, `SpellClassMask: flag128`, `BonusCoefficientFromAP`, `Scaling`, `_immunityInfo` are all absent. Spell.cpp's effect dispatch at C++ `Spell::HandleEffects` cannot proceed because there is nothing to dispatch into.

**Composed-type cross-table join coverage.** The 20-store DB2 join (`SpellName × SpellMisc × SpellEffect × SpellAuraOptions × SpellAuraRestrictions × SpellCastingRequirements × SpellCategories × SpellClassOptions × SpellCooldowns × SpellEquippedItems × SpellInterrupts × SpellLabel × SpellLevels × SpellPower × SpellReagents × SpellReagentsCurrency × SpellScaling × SpellShapeshift × SpellTargetRestrictions × SpellTotems × SpellXSpellVisual` + `SpellRange/SpellRadius/SpellCastTimes/SpellDuration/SpellProcsPerMinute/SpellProcsPerMinuteMod/SpellPowerDifficulty` lazy lookups + difficulty fallback) has **zero analog** in Rust. The composition step that defines `SpellInfo` does not exist — even if all 20 DB2 readers were ported tomorrow, there is no place to merge their output.

**Predicate/method coverage.** **0 of ~150** const methods implemented. Every predicate in `SpellInfo.h` (the `Is*` family ~30 methods, the `Has*` family ~10 methods, the `Calc*` family ~7 methods, the `Check*` family ~5 methods, the `Get*` family ~30 methods, the `Can*` family ~5 methods) is absent.

**Post-construction load coverage.** `_load_spell_specific` (188 lines), `_load_aura_state`, `_load_spell_diminish_info` (471 lines), `_load_immunity_info` (313 lines), `_initialize_explicit_target_mask`, `_initialize_spell_positivity` (39 lines) — **none** ported. The ~1,000 lines of post-construction setup that derive cached classification fields (`_spell_specific`, `_aura_state`, `_diminish_info`, `_allowed_mechanic_mask`, computed per-effect immunity, computed per-spell positivity) are absent.

**Constructor coverage.** Both ctors (`from SpellInfoLoadHelper` and `from effects-only` for serverside spells) are absent. There is no path from parsed DB2 data to a `SpellInfo` instance.

**Enum/type prerequisite coverage.**
- `SpellEffectName` (~151 variants) — **absent**
- `AuraType` (~280 variants) — **absent**
- `Targets` (~150 implicit target type values) — **absent**
- `Mechanics` (33 values) — **absent**
- `DispelType` (9 values) — **absent**
- `SpellSchoolMask` — **absent**
- `SpellInterruptFlags` / `SpellAuraInterruptFlags` / `SpellAuraInterruptFlags2` — **absent**
- `AuraStateType` — **absent**
- `SpellSpecificType` — **absent**
- `SpellCustomAttributes` — **absent**
- `SpellTarget*` classifier enums (5 of them: `Categories`, `Reference`, `Object`, `Check`, `Direction`) — **absent**
- `Flag128` — **absent**

**Persistence note.** `SpellInfo` itself does not persist; it's recomputed at each boot from DB2 + SQL overlays. Persistence of *active auras* and *cooldowns* (which read `SpellInfo` but live elsewhere) is the responsibility of `Player` save/load in the `character` DB and is tracked in `spells.md`.

**Migration entry point.** The first concrete sub-task in §9 (#SPELLINFO.1) creates the struct skeleton; #SPELLINFO.2 adds the 15 attribute `bitflags!` (highest-leverage primitive — every other predicate depends on them); #SPELLINFO.4 + #SPELLINFO.5 add the `SpellEffect` and `AuraType` enums (required by the effect array). Without these three nothing else can compile. After the type prerequisites, #SPELLINFO.16 (the composing ctor) is the unblocker for `SpellMgr::LoadSpellInfoStore` (in `spells-mgr.md` #SPELLMGR.3).
