# Migration: Spells

> **C++ canonical path:** `src/server/game/Spells/` (incluye `Auras/`)
> **Rust target crate(s):** `crates/wow-spell/`, `crates/wow-world/src/handlers/spell.rs`, `crates/wow-packet/src/packets/{spell,aura}.rs`
> **Layer:** L5 (Game systems — combat / spells / auras)
> **Status:** 🔧 broken (rewrite needed) — sólo cast handler básico + cooldown visible; crate `wow-spell` está **vacío**.
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing — see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

El módulo Spells de TrinityCore es el motor que **transforma una intención de hechizo (CMSG_CAST_SPELL) en efectos sobre el mundo**. Implementa todo el ciclo de vida: validación de targets/recursos, fase de cast (instant / cast time / channel), resolución de 151 spell effects (daño, heal, summon, teleport, dispel, polymorph, etc.), aplicación y mantenimiento de auras (buffs/debuffs persistentes con ticks), procs (efectos secundarios disparados por eventos), cooldowns por hechizo + GCD per-school, y resistencia/cap mecánico (miss/dodge/parry/resist). Es el segundo módulo más grande del core (~44k líneas C++) y el más complejo en términos de invariantes de juego — toca casi todos los demás sistemas (combat, movement, threat, AI, items, scripts).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Spells/Spell.cpp` | 9303 | `prefix` |
| `game/Spells/Spell.h` | 994 | `prefix` |
| `game/Spells/SpellCastRequest.h` | 43 | `prefix` |
| `game/Spells/SpellDefines.h` | 549 | `prefix` |
| `game/Spells/SpellHistory.cpp` | 1093 | `prefix` |
| `game/Spells/SpellHistory.h` | 207 | `prefix` |
| `game/Spells/SpellScript.cpp` | 1200 | `prefix` |
| `game/Spells/SpellScript.h` | 2271 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Spells/Spell.h` | 994 | `class Spell` (estado de un cast en curso), `SpellCastTargets`, `SpellEvent`, `SpellValue`, structs de log, public interface |
| `src/server/game/Spells/Spell.cpp` | 9303 | Spell::prepare/cast/finish, `_handle_immediate_phase`, `_handle_finish_phase`, validación (CheckCast, CheckPower, CheckItems, CheckRange), target selection, miss/hit calc, broadcast SMSG_SPELL_START/GO/FAILURE |
| `src/server/game/Spells/SpellEffects.cpp` | 5956 | Implementación de los ~151 `EffectXxx()` (EffectSchoolDMG, EffectHeal, EffectApplyAura, EffectTeleportUnits, EffectSummonType, EffectDispel, EffectInterruptCast, EffectKnockBack, EffectJump, …) dispatch table en `SpellEffectHandlers` |
| `src/server/game/Spells/SpellInfo.h` | 625 | `class SpellInfo` (datos estáticos del DBC + overrides), `SpellEffectInfo`, `SpellImplicitTargetInfo`, getters de attrs/flags/familia |
| `src/server/game/Spells/SpellInfo.cpp` | 5022 | SpellInfo loader (de DB2 + override), `_LoadSpellSpecific`, `IsAffected*`, `IsPositiveEffect`, target check helpers, attribute decoders |
| `src/server/game/Spells/SpellMgr.h` | 827 | `class SpellMgr` (singleton), todos los structs auxiliares: `SpellProcEntry`, `SpellThreatEntry`, `SpellTargetPosition`, `SpellChainNode`, `SpellLearnSpellNode`, `SkillLineAbilityEntry`, `PetAura`, `SpellArea`, etc. |
| `src/server/game/Spells/SpellMgr.cpp` | 5028 | Carga de tablas SQL spell_*, builds de cachés (mSpellInfoMap, mSpellProcMap, mSpellChains, mSpellLearnSpells, mSpellsReqSpell, mSpellTargetPositions, mSkillLineAbilityMap), `IsSpellValid`, `LoadSpellInfoStore` |
| `src/server/game/Spells/SpellHistory.h` | 207 | `class SpellHistory` per-Unit: cooldowns por spellId + por categoría, GCD, charges (multi-stack cd) |
| `src/server/game/Spells/SpellHistory.cpp` | 1093 | StartCooldown, AddCooldown, ConsumeCharge, ResetCooldown, HandleCooldowns, persist a DB (character_spell_cooldown), broadcast SMSG_SPELL_COOLDOWN |
| `src/server/game/Spells/SpellScript.h` | 2271 | Hooks declarativos: `SpellScript`, `AuraScript`, OnEffectHit/Launch, BeforeCast, OnCheckCast, AfterDispel, slots de modificación de daño/heal |
| `src/server/game/Spells/SpellScript.cpp` | 1200 | Implementación del runtime de SpellScript: registración de hooks, dispatch a scripts registrados |
| `src/server/game/Spells/SpellDefines.h` | 549 | Enums maestros: `SpellCastResult` (~250 fail codes), `SpellSchoolMask`, `SpellCastTargetFlags`, `SpellMissInfo`, `SpellInterruptFlags`, `Mechanics`, `DispelType`, `SpellEffectName`, `SpellAttr0..14` |
| `src/server/game/Spells/Auras/SpellAuraDefines.h` | 734 | `AuraType` enum (~280 SPELL_AURA_*), `AuraEffectHandleModes`, `AuraRemoveMode`, `AuraStateType` |
| `src/server/game/Spells/Auras/SpellAuras.h` | 398 | `class Aura` (instancia aplicada), `class UnitAura`/`DynObjAura` (subclases), `class AuraApplication` (per-target), `class AuraScript` |
| `src/server/game/Spells/Auras/SpellAuras.cpp` | 2665 | Lifecycle: `Create`, `_ApplyForTarget`, `_Remove`, `Update` (tick), `RefreshDuration`, stacking rules, broadcast SMSG_AURA_UPDATE, persistence |
| `src/server/game/Spells/Auras/SpellAuraEffects.h` | 410 | `class AuraEffect` (un effect index dentro de un Aura), HandleXxx + tick handlers — uno por AuraType |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | 6342 | Implementación de los ~280 handlers `HandleAuraXxx` (HandlePeriodicDamage, HandleAuraModStat, HandleModConfuse, HandleShapeshift, HandleAuraDummy, HandleSchoolAbsorb, HandleCharm, HandleProcTriggerSpell, …) |
| `src/server/game/Spells/SpellCastRequest.h` | 43 | POD para encolar casts pendientes (cuando estás en GCD pero envías otro) |
| `src/server/game/Spells/TraitMgr.h` / `.cpp` | 87 / 752 | (Retail-only — talents/traits 10.x, no aplica a 3.4.3 puro) |

**Total Spells/ + Auras/:** ~44,506 líneas (incl. headers + comentarios).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Spell` | class | Estado de **un cast en curso** (caster, targets, fase, timer, daño calculado, miss-info por target). Vida corta: nace en `Unit::CastSpell`, muere en `Spell::finish` |
| `SpellInfo` | class | Datos **estáticos** del hechizo cargados de DBC/DB2 (atributos, efectos, costo, rango, casttime, familia, GCD). Una instancia per spellId, compartida |
| `SpellEffectInfo` | class | Sub-struct de SpellInfo: un slot de efecto (Effect, BasePoints, ImplicitTargetA/B, Mechanic, RadiusEntry, TriggerSpell). Hasta 32 effects per spell |
| `SpellImplicitTargetInfo` | class | Decodifica los `Targets[A/B]` (TARGET_UNIT_TARGET_ENEMY, TARGET_DEST_DYNOBJ_ALLY, etc., ~150 valores) |
| `SpellCastTargets` | class | Targets concretos del cast (unit_target, item_target, src_pos, dst_pos, string, glyph) |
| `SpellEvent` | class | `BasicEvent` que hace tick al `Spell` durante cast time / cooldown of channel ticks |
| `SpellValue` | struct | Override per-cast de BasePoints / RadiusMod / Duration (used by triggers) |
| `Aura` | class | **Instancia activa** de una aura (puede afectar 1 o N targets vía AuraApplication). Owns AuraEffect[] |
| `UnitAura` | class | Aura cuyo "owner" es un Unit (la mayoría) |
| `DynObjAura` | class | Aura ligada a un DynamicObject (ej: Consecration, area auras) |
| `AuraApplication` | class | Vínculo per-target de una Aura (slot UI, flags, removeMode). 1 Aura puede tener N AuraApplication si afecta party |
| `AuraEffect` | class | Un effect index dentro de una Aura (ApplyAuraName + amount). Es lo que hace tick periódico (DoT, HoT) |
| `SpellHistory` | class | Per-Unit. Cooldowns spellId→time, categoryCooldowns, GCD, charges, persist |
| `SpellMgr` | class | Singleton. Cache central: `mSpellInfoMap`, `mSpellProcMap`, `mSpellChains`, `mSpellThreatMap`, `mSpellTargetPositions`, `mSkillLineAbilityMap`, `mSpellAreaMap`, etc. |
| `SpellScript` / `AuraScript` | class | Hook DSL para scripts (override damage, add CheckCast, override targets) |
| `SpellProcEntry` | struct | Reglas de proc cargadas de spell_proc (ProcFlags, SpellTypeMask, PpmRate, Chance, Cooldown, Charges) |
| `SpellThreatEntry` | struct | Override de threat from spell_threat (FlatMod, PctMod, ApPctMod) |
| `SpellTargetPosition` | struct | Target dest para EffectTeleportUnits (de spell_target_position) |
| `SpellChainNode` | struct | Rangos de un hechizo (rank, prev, next, first) |
| `SpellLearnSpellNode` | struct | Hechizos auto-learn al aprender otro |
| `SpellArea` | struct | Auras condicionales por zona/área/quest/aura |
| `PetAura` | class | Auras heredadas pet→owner |
| `SpellEffectName` | enum | ~151 `SPELL_EFFECT_*` (SCHOOL_DAMAGE, HEAL, APPLY_AURA, TELEPORT_UNITS, SUMMON, DISPEL, INSTAKILL, JUMP, CHARGE, …) |
| `AuraType` | enum | ~280 `SPELL_AURA_*` (PERIODIC_DAMAGE, MOD_STAT, MOD_INCREASE_HEALTH, SCHOOL_ABSORB, MOD_CONFUSE, MOD_FEAR, MOD_STUN, …) |
| `SpellCastResult` | enum | ~250 fail codes (SPELL_FAILED_NOT_READY, NO_POWER, OUT_OF_RANGE, LINE_OF_SIGHT, …) |
| `SpellMissInfo` | enum | MISS, RESIST, DODGE, PARRY, BLOCK, EVADE, IMMUNE, DEFLECT, ABSORB, REFLECT |
| `Mechanics` | enum | 33 mecánicas (STUN, FEAR, ROOT, SILENCE, DISARM, SLEEP, CHARM, …) |
| `SpellSchoolMask` | enum bitmask | NORMAL, HOLY, FIRE, NATURE, FROST, SHADOW, ARCANE — magic schools |
| `SpellInterruptFlags` | enum bitmask | MOVEMENT, PUSH_BACK, INTERRUPT, AUTOATTACK, DAMAGE — when channel/cast breaks |
| `DispelType` | enum | MAGIC, CURSE, DISEASE, POISON, STEALTH, INVISIBILITY, …; pareado con DispelMask |
| `SpellAttr0..14` | enum bitmask | 15 grupos de 32 bits cada uno = ~450 attribute flags individuales |
| `SpellCustomAttribute` | enum bitmask | Atributos derivados/computados runtime (CHARGE, PICKPOCKET, NO_INITIAL_AGGRO, …) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::CastSpell(...)` (overloaded) | Entry point — crea `Spell`, llama `prepare` | `new Spell()`, `Spell::prepare` |
| `Spell::prepare(SpellCastTargets const&, AuraEffect const* triggeredByAura)` | Inicializa target list, valida, programa SpellEvent, **envía SMSG_SPELL_START** | `CheckCast`, `CheckPower`, `CheckItems`, `SendSpellStart`, `TakePower` (si instant) |
| `Spell::cast(bool skipCheck)` | Ejecuta el cast cuando termina cast time. Loop sobre effects → handle_immediate, **envía SMSG_SPELL_GO** | `_handle_immediate_phase`, `SendSpellGo`, `TakePower`, `TakeReagents`, `SetExecutedCurrently` |
| `Spell::_handle_immediate_phase()` | Para cada effect/target → `HandleEffects(unit, item, gameobj, effect, mode)` | `HandleEffects` → `EffectXxx()` por dispatch |
| `Spell::_handle_finish_phase()` | Procesa AddDelayed, threat, lifesteal | `Unit::DealDamage`, `Unit::DealHeal`, `Unit::ProcSkillsAndAuras` |
| `Spell::HandleEffects(...)` | Dispatch al `EffectXxx()` correcto vía `SpellEffectHandlers[effect]` | Una de las 151 `EffectXxx` |
| `Spell::EffectSchoolDMG(SpellEffIndex)` | Daño directo según school + base points | `Unit::SpellDamageBonusDone`, `Unit::DealSpellDamage`, `SendSpellNonMeleeDamageLog` |
| `Spell::EffectHeal(SpellEffIndex)` | Heal directo | `Unit::SpellHealingBonusDone`, `Unit::HealBySpell` |
| `Spell::EffectApplyAura(SpellEffIndex)` | Crea/refresh `Aura` en target | `Aura::TryRefreshStackOrCreate`, `Aura::_ApplyForTarget` |
| `Spell::EffectTeleportUnits` | Mueve unit a SpellTargetPosition | `Player::TeleportTo` |
| `Spell::EffectSummonType` | Summon pet/totem/temp creature | `SummonsList`, `Unit::SummonCreature` |
| `Spell::EffectDispel` | Quita auras según DispelMask | `Unit::RemoveAurasDueToSpell`, `SendSpellDispelLog` |
| `Spell::CheckCast(bool strict, int32* param1, int32* param2)` | Validación maestra: power/range/los/state/items/aura — devuelve SpellCastResult | `CheckRange`, `CheckPower`, `CheckItems`, `CheckLineOfSight`, `Unit::HasAuraType` |
| `Spell::CheckPower()` | Maná/runic/energy/runas suficientes | `Unit::GetPower`, `Player::CanUseRunes` |
| `Spell::CheckItems()` | Reagents, item proficiency, totems requeridos | `Player::HasItemCount`, `Player::HasItemFitToSpellRequirements` |
| `Spell::CheckRange(bool strict)` | Distancia caster↔target dentro del rango (min/max) | `Unit::IsWithinDistInMap` |
| `Spell::SendSpellStart()` | Construye y envía SMSG_SPELL_START | `WorldPacket`, `Map::SendMessageInRange` |
| `Spell::SendSpellGo()` | SMSG_SPELL_GO con hit/miss list | `WriteSpellGoTargets` |
| `Spell::SendCastResult(SpellCastResult)` | SMSG_CAST_FAILED al caster | `WorldPacket`, `WorldSession::SendPacket` |
| `Spell::finish(bool ok)` | Cleanup: charges, scripts after-cast, m_spellState = SPELL_STATE_FINISHED | `SpellHistory::ConsumeCharge`, `m_caster->FinishSpell` |
| `Spell::cancel()` | Cancela cast/channel actual | `SendChannelUpdate(0)`, `SendInterrupted` |
| `Spell::update(uint32 difftime)` | Tick por SpellEvent (resta cast timer, channel ticks) | `cast` cuando termina, `Unit::HasUnitState(UNIT_STATE_CASTING)` |
| `Aura::TryRefreshStackOrCreate(...)` | Si ya existe stackeable, refresh; si no, crea nueva | `Aura::Create`, `RefreshDuration`, `ModStackAmount` |
| `Aura::Create(AuraCreateInfo&)` | Construye Aura + AuraEffect[] | `new UnitAura/DynObjAura`, `AuraEffect ctor` |
| `Aura::_ApplyForTarget(Unit*, Unit*, AuraApplication*)` | Aplica efectos en target, broadcast SMSG_AURA_UPDATE | `AuraEffect::HandleEffect(true)`, `target->_ApplyAuraEffect` |
| `Aura::Update(uint32, Unit*)` | Tick: resta duration, dispara periodic ticks | `AuraEffect::Update`, `HandlePeriodicXxx` |
| `Aura::Remove(AuraRemoveMode)` | Remueve, dispara handlers de salida | `AuraEffect::HandleEffect(false)`, `target->_RemoveAuraEffect` |
| `Aura::HandleAllEffects(AuraApplication*, mode, bool apply)` | Dispatch por AuraType al `HandleAuraXxx` correcto | `AuraEffect::HandleEffect` → uno de ~280 handlers |
| `AuraEffect::PeriodicTick(AuraApplication*, Unit*)` | Tick handlers (DoT damage, HoT heal, drain power) | `Unit::DealDamage`, `Unit::HealBySpell` |
| `SpellHistory::AddCooldown(spellId, itemId, cooldownEnd, categoryId, categoryEnd, onHold)` | Registra cooldown + opcional category cd | `SendSpellCooldown` |
| `SpellHistory::HasCooldown(spellId, itemId)` | Hot path: chequea durante CheckCast | Hash lookup en `_spellCooldowns` |
| `SpellHistory::StartCooldown(spellInfo, itemId, spell, onHold)` | Cooldown post-cast respetando GCD/category/charges | `AddCooldown`, `ConsumeCharge` |
| `SpellHistory::ConsumeCharge(chargeCategoryId)` | Resta una charge (multi-cast spells like Mind Flay 3 charges) | `_categoryCharges`, broadcast |
| `SpellHistory::HandleCooldowns(spellInfo, ...)` | Aplica cooldowns post-cast (GCD + spell + category) | `StartCooldown`, `AddGlobalCooldown` |
| `SpellHistory::AddGlobalCooldown(spellInfo, durationMs)` | GCD per-school (1.5s base, modificado por haste) | `_globalCooldowns` map |
| `Unit::ProcSkillsAndAuras(...)` | Dispara procs por DamageInfo / HealInfo | `SpellMgr::CanSpellTriggerProcOnEvent`, `Aura::TriggerProcOnEvent` |
| `SpellMgr::AssertSpellExistsInDatabase`, `IsSpellValid`, `GetSpellInfo` | Lookups maestros | `mSpellInfoMap` |
| `SpellMgr::LoadSpellInfoStore` | Build SpellInfo de DB2 + override SQL | DB2 access, `spell_dbc` SQL |
| `SpellMgr::LoadSpellProcs` | spell_proc → mSpellProcMap | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellTargetPositions` | spell_target_position → mSpellTargetPositions | `WorldDatabase.Query` |
| `SpellMgr::LoadSpellRanks` / `LoadSpellLearnSpells` / `LoadSpellRequired` | Dependencias y rangos; ranks derive from `SkillLineAbilityEntry::SupercedesSpell` in the current C++ tree | DB2 + `WorldDatabase.Query` |
| `SpellInfo::GetMaxRange(bool positive, WorldObject const* caster, Spell* spell)` | Range final con mods de caster | `SpellRangeStore`, modifiers |
| `SpellInfo::CheckTarget(WorldObject const* caster, WorldObject const* target, bool implicit)` | Validación target type/faction/aura state | Multiple |
| `SpellInfo::IsPositive` / `IsTargetingArea` / `IsChanneled` / `IsAutoRepeatRangedSpell` | Predicates de uso muy frecuente | Bit reads on `AttributesEx*` |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Unit` — `Unit::CastSpell`, `Unit::DealDamage`, `Unit::HealBySpell`, `Unit::AddAura`, `Unit::HasAuraType` (toda la API spell pasa por Unit)
- `Entities/Player` — `Player::HasItemCount` (reagents), `Player::TeleportTo` (EffectTeleportUnits), `Player::SendNewSpell`, `Player::CanUseRunes`
- `Entities/Pet` — EffectSummonPet, charm, pet-aura inheritance
- `Combat` — `DamageInfo`, `HealInfo`, `Unit::CalculateMeleeDamage`, threat tables (`ThreatManager::AddThreat`)
- `Movement` — EffectKnockBack, EffectJump, EffectCharge, `MotionMaster::MoveJump`, `MovementInfo` for interrupt-on-move
- `Maps` — `Map::VisitNearbyCellsOf` (AoE target search), `Map::isInLineOfSight` (LoS checks)
- `DataStores` — DB2 stores: `SpellNameStore`, `SpellEffectStore`, `SpellCooldownsStore`, `SpellInterruptsStore`, `SpellRangeStore`, `SpellRadiusStore`, `SpellCategoriesStore`, `SpellAuraOptionsStore`, `SpellCastTimesStore`, `SpellPowerStore`, `SpellMiscStore`, `SkillLineAbilityStore`
- `Database (WorldDatabase)` — load spell_*, spell_proc, etc.; rank chains come from DB2 `SkillLineAbilityEntry::SupercedesSpell` in the current C++ tree
- `Database (CharacterDatabase)` — persistencia de auras/cooldowns en `character_aura`, `character_spell_cooldown`
- `Conditions` — `ConditionMgr::IsObjectMeetingNotGroupedConditions` (para condition-based targets)
- `ScriptMgr` — `OnSpellCast`, `OnHit`, `OnApply`, `OnRemove` (SpellScript / AuraScript dispatch)
- `AreaTrigger` / `DynamicObject` — Persistent area auras, ground-targeted spells
- `GridNotifiers` — Object searcher templates para AoE
- `Pathing` (PathGenerator) — para EffectJump / EffectCharge
- `SharedDefines` — SpellSchool, SpellAttr0..14, Powers, Mechanics

**Depended on by:**
- `AI/*` — `UnitAI::DoCast*`, `DoSpellAttackIfReady`, OnSpellHit, OnSpellFailed
- `Combat` — auto-attack triggers ranged spell (auto-shot via Spell)
- `Movement` — slow auras (SPELL_AURA_MOD_DECREASE_SPEED) feed into MovementInfo
- `Pets` — pet abilities cast vía mismo Spell pipeline
- `Quests` — kill credit can be conditional sobre spell cast
- `Loot` — spell-based loot (mining, herbalism via cast)
- `Achievements` — proc-on-cast achievements
- `Group` — heal aggro routing, party buffs
- `Items/Enchant` — proc enchants (SpellEnchantProcEntry)
- `OutdoorPvP` / `Battleground` — flag carrier auras
- `Scripting` — todos los boss scripts disparan/reaccionan a spells

---

## 6. SQL / DB queries (if any)

El módulo emite queries directamente vía `WorldDatabase.Query(...)` (no usa prepared statements para bulk loads de spell data — son loads de inicio de servidor). Persistencia de cooldowns/auras sí usa prepared via Player save.

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM spell_dbc` | Override / inserciones custom de SpellInfo (campos override de DB2) | world |
| `SELECT * FROM spell_proc` | Reglas de proc cargadas a `mSpellProcMap` | world |
| `SELECT spell_id, req_spell FROM spell_required` | Spells que requieren conocer otro antes | world |
| `SELECT entry, SpellID, Active FROM spell_learn_spell` | Spells que enseñan otros automáticamente | world |
| `SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position` | Destinos de TeleportUnits con TARGET_DEST_DB | world |
| `SELECT * FROM spell_threat` | Override de threat (flat/pct/ap_pct) por spellId | world |
| `SELECT * FROM spell_pet_auras` | Auras que pet hereda de owner | world |
| `SELECT * FROM spell_enchant_proc_data` | Procs de enchant (chance, ppm) | world |
| `SELECT * FROM spell_area` | Auras condicionales por zona/area/quest | world |
| `SELECT * FROM spell_group` / `spell_group_stack_rules` | Reglas de stacking entre grupos de spells | world |
| DB2 `SkillLineAbilityEntry::SupercedesSpell` | Cadenas de rangos (`LoadSpellRanks`) | DB2 |
| `SELECT * FROM spell_loot_template` | Loot disparado por cast (rare) | world |
| `SELECT * FROM spell_script_names` | Mapping spellId → C++ ScriptName | world |
| `SELECT entry, attributes FROM spell_custom_attr` | SQL override de `AttributesCu`; la derivación completa de custom attributes ocurre después por análisis de efectos/auras/talentos/visuales | world |
| `SELECT ... FROM serverside_spell_effect` / `SELECT ... FROM serverside_spell` | Server-only spells (`LoadSpellInfoServerside`, modern replacement for old `spell_dbc`) | world |
| `SELECT * FROM spelldifficulty_dbc` | Override de spell por difficulty (heroic/normal/raid) | world |
| `SELECT * FROM spell_linked_spell` | Spell A dispara/cancela Spell B | world |
| `SELECT * FROM character_aura` | Persisted auras al logout (per-character save/load) | character |
| `SELECT * FROM character_spell_cooldown` | Persisted cooldowns | character |
| `INSERT/UPDATE character_spell` | Player learned spells | character |

**DB2 Stores:**

| Store | What it loads | Read by |
|---|---|---|
| `SpellNameStore` | Spell.db2 (Id, Name) | SpellInfo loader |
| `SpellMiscStore` | SpellMisc.db2 (CastTimeIndex, RangeIndex, Speed, Attributes[14], ContentTuningId) | SpellInfo |
| `SpellEffectStore` | SpellEffect.db2 (DifficultyID, EffectIndex, Effect, BasePoints, ImplicitTarget[2], Mechanic, Radius, ChainTargets, TriggerSpell, ScalingClass) | SpellInfo |
| `SpellCooldownsStore` | SpellCooldowns.db2 (RecoveryTime, CategoryRecoveryTime, StartRecoveryTime) | SpellInfo + SpellHistory |
| `SpellInterruptsStore` | SpellInterrupts.db2 (InterruptFlags, AuraInterruptFlags, ChannelInterruptFlags) | Spell, Aura, Unit |
| `SpellCastTimesStore` | SpellCastTimes.db2 (Base, PerLevel, Minimum) | SpellInfo |
| `SpellRangeStore` | SpellRange.db2 (RangeMin[2], RangeMax[2], Flags) | Spell::CheckRange |
| `SpellRadiusStore` | SpellRadius.db2 (Radius, RadiusPerLevel, RadiusMax) | AoE target search |
| `SpellCategoriesStore` | SpellCategories.db2 (DispelType, Mechanic, Attributes) | SpellInfo / dispel |
| `SpellAuraOptionsStore` | SpellAuraOptions.db2 (CumulativeAura, ProcChance, ProcCharges, SpellProcsPerMinuteId) | Aura, SpellProc |
| `SpellPowerStore` | SpellPower.db2 (PowerType, ManaCost, ManaCostPerLevel, PowerPctCost) | Spell::CheckPower |
| `SpellShapeshiftStore` | SpellShapeshift.db2 (ShapeshiftMask, ShapeshiftExclude) | CheckCast |
| `SkillLineAbilityStore` | SkillLineAbility.db2 (SkillLine, Spell, RaceMask, ClassMask) | learning |
| `SpellEffectScalingStore` | SpellEffectScaling.db2 (Coefficient, Variance, ResourceCoefficient) | SpellEffectInfo |
| `SpellLearnSpellStore` | SpellLearnSpell.db2 (SpellID, LearnSpellID) | learning |
| `SpellProcsPerMinuteStore` | SpellProcsPerMinute.db2 (BaseProcRate) | proc rate |
| `SpellTargetRestrictionsStore` | SpellTargetRestrictions.db2 (MaxAffectedTargets, ConeDegrees, Width) | AoE caps |
| `SpellEquippedItemsStore` | SpellEquippedItems.db2 (EquippedItemClass, SubClassMask, InventoryTypeMask) | CheckItems |
| `SpellClassOptionsStore` | SpellClassOptions.db2 (SpellFamilyName, SpellClassMask) | SpellFamilyFlags |
| `SpellLevelsStore` | SpellLevels.db2 (BaseLevel, SpellLevel, MaxLevel) | scaling |
| `SpellReagentsStore` | SpellReagents.db2 (Reagent[8], ReagentCount[8]) | CheckItems |
| `SpellTotemsStore` | SpellTotems.db2 (RequiredTotemCategoryID, Totem[2]) | shaman |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_CAST_SPELL` | client → server | `WorldSession::HandleCastSpellOpcode` → `Unit::CastSpell` |
| `CMSG_CANCEL_CAST` | client → server | `WorldSession::HandleCancelCastOpcode` → `Spell::cancel` |
| `CMSG_CANCEL_CHANNELLING` | client → server | `WorldSession::HandleCancelChanneling` |
| `CMSG_CANCEL_AURA` | client → server | `WorldSession::HandleCancelAuraOpcode` → `Unit::RemoveOwnedAura` |
| `CMSG_CANCEL_AUTO_REPEAT_SPELL` | client → server | Stop auto-shot |
| `CMSG_CANCEL_GROWTH_AURA` | client → server | (legacy) drop self growth |
| `CMSG_TOTEM_DESTROYED` | client → server | Player kills own totem (drops aura too) |
| `CMSG_PET_CAST_SPELL` | client → server | `Pet` casts via owner request |
| `SMSG_SPELL_START` | server → client | `Spell::SendSpellStart` (cast init) |
| `SMSG_SPELL_GO` | server → client | `Spell::SendSpellGo` (resolve, hit/miss list, cast finalizado) |
| `SMSG_SPELL_FAILURE` | server → client (broadcast) | Cast failed visible to nearby |
| `SMSG_SPELL_FAILED_OTHER` | server → client | Variante con razón |
| `SMSG_CAST_FAILED` | server → client (caster only) | `Spell::SendCastResult` (razón legible) |
| `SMSG_PET_CAST_FAILED` | server → client | Pet cast failed |
| `SMSG_SPELL_DELAYED` | server → client | Push-back / delay added to cast |
| `SMSG_SPELL_COOLDOWN` | server → client | `SpellHistory::SendSpellCooldown` |
| `SMSG_COOLDOWN_EVENT` | server → client | Single spell cd event |
| `SMSG_CLEAR_COOLDOWN` | server → client | Reset cooldown remoto |
| `SMSG_MODIFY_COOLDOWN` | server → client | Adjust cooldown remaining |
| `SMSG_AURA_UPDATE` | server → client | `Aura::_ApplyForTarget` / `_Remove` (broadcast a vecinos) |
| `SMSG_AURA_UPDATE_ALL` | server → client | Snapshot completo de auras (login / phase change) |
| `SMSG_PERIODIC_AURA_LOG` | server → client | DoT/HoT tick log |
| `SMSG_SPELL_DISPELL_LOG` | server → client | EffectDispel result |
| `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` | server → client | Daño de spell directo |
| `SMSG_SPELL_HEAL_LOG` | server → client | Heal log |
| `SMSG_SPELL_ENERGIZE_LOG` | server → client | Power gain log |
| `SMSG_SPELL_INSTAKILL_LOG` | server → client | Insta-kill confirm |
| `SMSG_SPELL_MISS_LOG` | server → client | Miss/resist info |
| `SMSG_CHANNEL_START` | server → client | Channel begin |
| `SMSG_CHANNEL_UPDATE` | server → client | Channel time remaining update |
| `SMSG_RESURRECT_REQUEST` | server → client | Resurrect spell offered |
| `SMSG_LEARNED_SPELL` | server → client | Player learned new spell |
| `SMSG_UNLEARNED_SPELLS` | server → client | Spells removed |
| `SMSG_SUPERCEDED_SPELL` | server → client | New rank replaces old |
| `SMSG_PLAY_SPELL_VISUAL_KIT` / `SMSG_PLAY_SPELL_VISUAL` | server → client | Visual-only effects |
| `SMSG_NOTIFY_DEST_LOC_SPELL_CAST` | server → client | Ground-target cast notification |
| `SMSG_DISPEL_FAILED` | server → client | Dispel resisted |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/{spell,aura}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-packet/src/packets/spell.rs` | `file` | 1 | 466 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/aura.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `crates/wow-data/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 líneas (vacío)** — el crate existe pero no tiene contenido
- `crates/wow-world/src/handlers/spell.rs` — ~288 líneas — handler de `CMSG_CAST_SPELL` con cast time básico, cooldown estilo "single timer", parse de targets simplificado
- `crates/wow-packet/src/packets/spell.rs` — ~466 líneas — POD de `CastSpellRequest`, `SpellStartPkt`, `CastFailed`, `SpellTargetData`, `SpellCastVisual`
- `crates/wow-packet/src/packets/aura.rs` — ~123 líneas — `AuraData` + `AuraUpdate` (SMSG_AURA_UPDATE) writer básico
- `crates/wow-data/src/spell_info.rs` (?) — referenciado como `wow_data::SpellInfo` en handler (cast_time_ms, recovery_time_ms, effective_cooldown_ms, has_cast_time)
- `crates/wow-world/src/session.rs` — campos `known_spells: HashSet<u32>`, `last_spell_cast_time: Option<Instant>`, `last_spell_cast_time_per_spell: HashMap<u32, Instant>`, `active_spell_cast: Option<SpellCastState>`, `spell_store: Option<...>`, `SpellCastState` struct

**What's implemented:**
- Parse de `CMSG_CAST_SPELL` → `CastSpellRequest`
- Validación "spell conocido" (HashSet contains)
- Validación "cooldown" muy simplificada: un único `last_spell_cast_time` global (NO es GCD per-school) + `last_spell_cast_time_per_spell` para cd individual
- Si `has_cast_time()` → envía `SMSG_SPELL_START`, guarda `SpellCastState`, espera tick para ejecutar
- Si instant → llama `execute_spell(spell_id, target_guid)` (asumido en otro archivo)
- Envío de `CastFailed` (SMSG_CAST_FAILED) con `reason: 2` (NotKnown) o `10` (NotReady)
- Targeting básico: si `target.unit` no vacío usa Unit flag (0x2), si no auto-target self
- Auras: `AuraData` POD + `AuraUpdate` packet write (sin lógica de aura activa)

**What's missing vs C++:**
1. **No `Spell` class** — no hay objeto `Spell` separado; toda la lógica vive en el handler. C++ tiene un `Spell*` con estado (~9k líneas)
2. **No `SpellInfo` proper** — sólo lo mínimo (cast_time, recovery_time); falta TODO el resto (151 effects, 14 attribute groups, school mask, mechanic, range, radius, power cost, reagents, family flags, scaling, levels, totems, equipped items, target restrictions)
3. **Sólo ~5/151 spell effects** implementados (probablemente SchoolDamage, Heal, ApplyAura, TeleportUnits, summon stub) — las otras ~146 sin nada
4. **No hay GCD per-school** — sólo un timer global; falta `_globalCooldowns` map por SpellSchool
5. **No category cooldowns** — spells en categoría comparten cd; falta
6. **No charges** (multi-cast spells) — falta `_categoryCharges`
7. **No SpellHistory class** — no persistence de cooldowns a DB
8. **No proc system** — `ProcFlags`, `ProcEventInfo`, `Unit::ProcSkillsAndAuras`, `SpellProcEntry`, PpmRate, ICD por aura — todo ausente
9. **No channel real** — `CMSG_CANCEL_CHANNELLING` es stub vacío, no hay channel ticks
10. **No projectile / spell speed** — instant resolve, sin SMSG_MISSILE travel time
11. **No CheckCast completo** — sin range, sin LoS, sin power, sin items/reagents, sin shapeshift, sin aura state, sin caster auras
12. **No SpellMissInfo** — todo asume hit; sin miss/resist/dodge/parry/block/immune
13. **No interrupt-on-move** ni interrupt-on-damage
14. **No AoE target search** — sin VisitNearbyCellsOf con SearchTargets
15. **No AoE caps** (max affected targets) — falta SpellTargetRestrictions
16. **No school separation** — todo daño es genérico
17. **No dispel mechanics** — dispelMask + ICD no resueltos
18. **No SpellMgr vivo** — `spell_required`, `spell_learn_spell`, `spell_threat`, `spell_target_position`, `spell_group`, `spell_group_stack_rules`, `spell_area`, `spell_custom_attr` SQL overrides, `serverside_spell`/`serverside_spell_effect` staging, `LoadSpellRanks`/rank chains y la mitad explícita SQL de `spell_proc` ya tienen stores representados en `wow-data` (target position además está cableado en world startup), pero no existe todavía una carga viva de `SpellMgr`; `spell_proc` sigue incompleto por la generación implícita desde auras, las cadenas de rango aún no parchean `SpellInfo::ChainEntry` real, `spell_area` no se consume en el runtime de auras, falta insertar server-only spells en el `SpellInfo` autoritativo, y falta la derivación completa de `AttributesCu`.
19. **No SpellScript / AuraScript** — sin DSL para scripts
20. **No Aura class real** — `AuraData` es POD para wire only, sin lifecycle (apply, tick, refresh, remove, expire)
21. **No AuraEffect tick** — sin DoT/HoT damage por intervalo
22. **No stacking rules** — `RefreshStackOrCreate` ausente
23. **No persistence** — sin save/load de auras / cooldowns en character DB
24. **No spell.dbc / spell.db2 loader** — el `spell_store` parece un stub

**Suspicious / likely divergent:**
- El handler asume que `effective_cooldown_ms()` es lo correcto a comparar contra `last_spell_cast_time` global, pero C++ trata GCD y spell-specific cd como cosas distintas — habrá bugs cuando GCD termine pero spell cd siga (o viceversa)
- `cast_id` se reusa entre client y server pero C++ asigna server-generated `m_castId` distinto al client request (anti-cheat); RustyCore parece usar el del cliente directamente
- `target_flags` hardcoded a `0x2` (Unit) o `0x2` con self GUID — no respeta los target flags reales del cliente
- El tick del cast (cuando termina cast time → execute_spell) no está visible en este file; probablemente vive en session update loop sin separación clara de fase
- Las auras se envían con el packet writer pero sin estado server-side, lo cual es UI-only y se desincroniza al instante

**Tests existing:**
- `crates/wow-packet/src/packets/aura.rs::test_aura_update_write` — 1 test, sólo serialización
- 0 tests en `crates/wow-spell/`
- 0 tests en `crates/wow-world/src/handlers/spell.rs`

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SPELLS.WBS.001** Partir y cerrar la migracion auditada de `game/Spells/Spell.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 9303 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.002** Partir y cerrar la migracion auditada de `game/Spells/Spell.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 994 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.003** Cerrar la migracion auditada de `game/Spells/SpellCastRequest.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellCastRequest.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.004** Partir y cerrar la migracion auditada de `game/Spells/SpellDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellDefines.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 549 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.005** Partir y cerrar la migracion auditada de `game/Spells/SpellHistory.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellHistory.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1093 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.006** Cerrar la migracion auditada de `game/Spells/SpellHistory.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellHistory.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.007** Partir y cerrar la migracion auditada de `game/Spells/SpellScript.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1200 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS.WBS.008** Partir y cerrar la migracion auditada de `game/Spells/SpellScript.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2271 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerados para referencia desde `MIGRATION_ROADMAP.md`. Complejidad: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#SPELLS.1** Crear `crates/wow-spell/src/lib.rs` con módulos `spell_info`, `spell`, `aura`, `spell_history`, `spell_mgr`, `effects`, `defines` (XL — splitear)
- [ ] **#SPELLS.2** Definir `enum SpellEffect` con las 151 variantes (mapeo a `SpellEffectName` C++) en `defines.rs` (M)
- [ ] **#SPELLS.3** Definir `enum AuraType` con las ~280 variantes (`SPELL_AURA_*`) (M)
- [ ] **#SPELLS.4** Definir `enum SpellCastResult` con los ~250 fail codes (M)
- [ ] **#SPELLS.5** Definir `enum SpellMissInfo`, `enum SpellSchool` + `bitflags SpellSchoolMask`, `enum Mechanics`, `enum DispelType`, `enum SpellInterruptFlags`, `enum AuraStateType` (M)
- [ ] **#SPELLS.6** Definir `bitflags SpellAttr0..SpellAttr14` (15 grupos × 32 bits) — todos los flags atributo (H)
- [ ] **#SPELLS.7** Implementar `struct SpellInfo` completo: Id, name, cast_time_ms, range_min/max, radius_index, school_mask, attributes[15], effects: Vec<SpellEffectInfo>, power: Vec<SpellPower>, totems, reagents, equipped_items_class, family_name, family_flags, scaling, levels, target_restrictions, etc. (H)
- [ ] **#SPELLS.8** Implementar `struct SpellEffectInfo`: effect: SpellEffect, base_points, real_points_per_level, points_per_combo, dice_per_level, implicit_target_a/b: SpellImplicitTargetInfo, mechanic, radius_entry, chain_targets, trigger_spell, scaling_class (M)
- [ ] **#SPELLS.9** Implementar `SpellInfo` predicates: `is_positive`, `is_targeting_area`, `is_channeled`, `is_passive`, `is_ranged`, `has_attr`, `is_affected_by_spell` (M)
- [ ] **#SPELLS.10** Cargar SpellInfo desde DB2 store (Spell.db2 + SpellEffect.db2 + SpellMisc.db2 + SpellCooldowns.db2 + SpellInterrupts.db2 + SpellRange.db2 + SpellRadius.db2 + SpellCategories.db2 + SpellAuraOptions.db2 + SpellPower.db2 + SpellCastTimes.db2 + SpellLevels.db2 + SpellTargetRestrictions.db2 + SpellShapeshift.db2 + SpellEquippedItems.db2 + SpellClassOptions.db2 + SpellReagents.db2 + SpellTotems.db2 + SpellScaling.db2 + SpellEffectScaling.db2) (XL — depende de wow-data)
- [ ] **#SPELLS.11** Crear `SpellMgr` singleton con `mSpellInfoMap: DashMap<u32, Arc<SpellInfo>>`, `get_spell_info`, `is_spell_valid` (M)
- [ ] **#SPELLS.12** Cargar tabla `spell_proc` → `SpellProcEntry` y construir `mSpellProcMap`: query/store explícito representado (`SpellProcStoreLikeCpp`), pendiente generación implícita desde auras, wiring vivo y runtime. (M)
- [ ] **#SPELLS.13** Cargar tabla `spell_target_position` → `mSpellTargetPositions` (L)
- [ ] **#SPELLS.14** Cargar `spell_required` + `spell_learn_spell` + rank chains (`LoadSpellRanks` desde DB2 `SkillLineAbilityEntry::SupercedesSpell`; represented store/accessors exist, live `SpellMgr` wiring pending) (M)
- [ ] **#SPELLS.15** Cargar tabla `spell_threat` (override threat) (L)
- [ ] **#SPELLS.16** Cargar tabla `spell_area` (represented query/store exists with C++ validation and indices; live SpellMgr wiring and aura runtime consumption pending) (M)
- [ ] **#SPELLS.17** Cargar tabla `spell_group` + `spell_group_stack_rules`: ambas tienen query/store representados (`SpellGroupStoreLikeCpp`, `SpellGroupStackRuleStoreLikeCpp`), pero falta wiring al `SpellMgr` vivo y consumo en el runtime real de auras. (M)
- [ ] **#SPELLS.18** Implementar `struct Spell` (estado de cast en curso): caster, targets, spell_info: Arc<SpellInfo>, m_castId, m_castFlags, m_spellState, m_timer, m_channelTargetEffectMask, unit_target_map: HashMap<Guid, TargetInfo> con miss_info (H)
- [ ] **#SPELLS.19** Implementar `Spell::prepare(targets, triggered_by_aura)`: target list build, validate, schedule cast event, send SMSG_SPELL_START si tiene cast time, take power if instant (H)
- [ ] **#SPELLS.20** Implementar `Spell::cast(skip_check)`: handle_immediate_phase + send SMSG_SPELL_GO con hit/miss list (H)
- [ ] **#SPELLS.21** Implementar `Spell::CheckCast` completo: power, range, LoS, items, reagents, shapeshift, aura state, target restrictions, casting_time_index, on_vehicle, etc. (XL)
- [ ] **#SPELLS.22** Implementar `Spell::CheckPower` (mana/rage/energy/runes/runic) (M)
- [ ] **#SPELLS.23** Implementar `Spell::CheckItems` (reagents, item proficiency, totems) (M)
- [ ] **#SPELLS.24** Implementar `Spell::CheckRange(strict)` con min/max range + caster size (M)
- [ ] **#SPELLS.25** Implementar `Spell::CheckLineOfSight` integrado con Maps/VMap (H)
- [ ] **#SPELLS.26** Implementar dispatch table `SpellEffectHandlers[SpellEffect]` (151 entradas) — fila inicial: 30 effects más comunes implementados, resto stub (H)
- [ ] **#SPELLS.27** Implementar `EffectSchoolDMG` con SpellDamageBonusDone/Taken, miss/crit/resist roll, school separation (H)
- [ ] **#SPELLS.28** Implementar `EffectHeal` con SpellHealingBonusDone/Taken, crit, overheal log (M)
- [ ] **#SPELLS.29** Implementar `EffectApplyAura` → crea `Aura` via `Aura::TryRefreshStackOrCreate` (M)
- [ ] **#SPELLS.30** Implementar `EffectTeleportUnits` (lookup `spell_target_position`, `Player::TeleportTo`) (L)
- [ ] **#SPELLS.31** Implementar `EffectSummonType` (totem, pet, temp creature, ground summon, vehicle) (XL — depende de Pet/Totem)
- [ ] **#SPELLS.32** Implementar `EffectDispel` con DispelMask + DispelChance + ICD + send SMSG_SPELL_DISPELL_LOG (M)
- [ ] **#SPELLS.33** Implementar `EffectInterruptCast` → cancel current cast on target, lock school for X ms (M)
- [ ] **#SPELLS.34** Implementar `EffectKnockBack` / `EffectJump` / `EffectJumpDest` / `EffectLeapBack` / `EffectCharge` (depende de Movement) (H)
- [ ] **#SPELLS.35** Implementar `EffectEnergize` (power gain) + SMSG_SPELL_ENERGIZE_LOG (L)
- [ ] **#SPELLS.36** Implementar resto de 100+ EffectXxx en lotes prioridad por uso real (XL — splittable per archetype: damage, heal, summon, item, quest, debug)
- [ ] **#SPELLS.37** Implementar `struct Aura` con Vec<AuraEffect>, owner, caster_guid, m_duration, m_max_duration, m_charges, m_stack_amount, m_applications: HashMap<Guid, AuraApplication> (H)
- [ ] **#SPELLS.38** Implementar `Aura::Create` + `TryRefreshStackOrCreate` + stacking rules por SpellGroup (H)
- [ ] **#SPELLS.39** Implementar `Aura::_ApplyForTarget` / `_Remove` con AuraRemoveMode (Default, Cancel, Death, Dispel, Expire, Interrupt) + broadcast SMSG_AURA_UPDATE (M)
- [ ] **#SPELLS.40** Implementar `Aura::Update(diff_ms)` (resta duration, dispatch periodic ticks) (M)
- [ ] **#SPELLS.41** Implementar `AuraEffect::PeriodicTick` (DoT damage / HoT heal / power drain / power burn) + SMSG_PERIODIC_AURA_LOG (M)
- [ ] **#SPELLS.42** Implementar handlers HandleAuraXxx por AuraType (las 30 más usadas: ModStat, ModDamageDone, ModSpeed, ModConfuse, ModFear, ModRoot, ModStun, ModSilence, SchoolAbsorb, ModIncreaseHealth, ModRegen, …) (H)
- [ ] **#SPELLS.43** Implementar resto de los ~250 handlers HandleAuraXxx en lotes (XL)
- [ ] **#SPELLS.44** Implementar `struct SpellHistory` (per-Unit): `_spellCooldowns: HashMap<u32, CooldownEntry>`, `_categoryCooldowns: HashMap<u32, time>`, `_globalCooldowns: HashMap<SpellSchool, time>`, `_categoryCharges: HashMap<u32, ChargeEntry>` (H)
- [ ] **#SPELLS.45** Implementar `SpellHistory::HasCooldown` / `AddCooldown` / `StartCooldown` / `HandleCooldowns` con GCD per-school + category cd + charges (M)
- [ ] **#SPELLS.46** Implementar `SpellHistory::ConsumeCharge` y `RestoreCharge` (M)
- [ ] **#SPELLS.47** Implementar broadcast `SMSG_SPELL_COOLDOWN`, `SMSG_COOLDOWN_EVENT`, `SMSG_CLEAR_COOLDOWN`, `SMSG_MODIFY_COOLDOWN` (L)
- [ ] **#SPELLS.48** Implementar persistencia: `character_aura` table save/load via Player save loop (M)
- [ ] **#SPELLS.49** Implementar persistencia: `character_spell_cooldown` table save/load (M)
- [ ] **#SPELLS.50** Implementar Proc system: `ProcFlags`, `ProcEventInfo`, `Unit::proc_skills_and_auras`, dispatch a auras con SpellProcEntry matching (H)
- [ ] **#SPELLS.51** Implementar PPM (procs per minute) calculation con weapon speed normalization (M)
- [ ] **#SPELLS.52** Implementar Internal Cooldown (ICD) per-aura para procs (L)
- [ ] **#SPELLS.53** Implementar channel real: `Spell::SendChannelStart` + tick spawning + `SendChannelUpdate` + interrupt on move/damage (H)
- [ ] **#SPELLS.54** Implementar projectile / spell speed: SMSG_SPELL_GO con missile travel, delayed effects on impact (H)
- [ ] **#SPELLS.55** Implementar AoE target search via `Map::VisitNearbyCellsOf` + filtros (`AnyAoETargetUnitInObjectRangeCheck`, `AnyFriendlyUnitInObjectRangeCheck`) (H)
- [ ] **#SPELLS.56** Implementar AoE caps via SpellTargetRestrictions.MaxAffectedTargets (L)
- [ ] **#SPELLS.57** Implementar SpellMissInfo roll (miss / resist / dodge / parry / block / immune / deflect / reflect) integrado con Unit stats (H)
- [ ] **#SPELLS.58** Implementar interrupt-on-movement (CMSG_MOVE_* mientras casteando → cancel cast) (M)
- [ ] **#SPELLS.59** Implementar interrupt-on-damage (DamageTaken → check ChannelInterruptFlags) (M)
- [ ] **#SPELLS.60** Implementar push-back (damage during cast → SMSG_SPELL_DELAYED + extiende cast time) (M)
- [ ] **#SPELLS.61** Implementar `CMSG_CANCEL_CAST` real (cancel actual + restablecer power si refundable) (L)
- [ ] **#SPELLS.62** Implementar `CMSG_CANCEL_CHANNELLING` real (M)
- [ ] **#SPELLS.63** Implementar `CMSG_CANCEL_AURA` (player puede cancelar buffs propios — verifica `SPELL_ATTR0_CANT_CANCEL`) (L)
- [ ] **#SPELLS.64** Implementar `SpellScript` / `AuraScript` DSL en Rust (trait con BeforeCast/OnEffectHit/OnApply/OnRemove/OnDispel hooks) (XL)
- [ ] **#SPELLS.65** Implementar tabla `spell_script_names` → registry de scripts compilados Rust (M)
- [ ] **#SPELLS.66** Implementar `Unit::CastSpell` overloads en `crates/wow-entity/unit.rs` (entry point del módulo) (M)
- [ ] **#SPELLS.67** Refactorizar `handlers/spell.rs` para llamar `Unit::cast_spell` (no contener la lógica) (M)
- [ ] **#SPELLS.68** Migrar `wow_data::SpellInfo` a usar el `wow_spell::SpellInfo` real (deprecar el stub) (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SPELLS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 8 files / 15660 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp`. Rust target: `crates/wow-spell`. | `cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SPELLS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 8 files / 15660 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp`. Rust target: `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SPELLS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 8 files / 15660 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp`. Rust target: `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SPELLS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 8 files / 15660 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp`. Rust target: `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `SpellInfo::has_attr(SpellAttr0::Passive)` parsea correctamente desde DB2 bytes
- [ ] Test: `SpellInfo::is_positive()` matches C++ para spells canónicos (ej. Renew = positive, Curse of Agony = negative)
- [ ] Test: `SpellInfo::get_max_range()` con SpellRange.db2 + caster bonuses == C++
- [ ] Test: `SpellInfo` carga 151 effect slots correctamente para Pyroblast (id 11366) — base_points, mechanic, implicit targets
- [ ] Test: `Spell::prepare` envía exactamente 1 SMSG_SPELL_START con cast_time correcto
- [ ] Test: `Spell::cast` envía 1 SMSG_SPELL_GO con correct hit_count + miss_count
- [ ] Test: `Spell::CheckCast` devuelve `SPELL_FAILED_NO_POWER` si caster.power < cost
- [ ] Test: `Spell::CheckCast` devuelve `SPELL_FAILED_OUT_OF_RANGE` si distance > max_range
- [ ] Test: `Spell::CheckCast` devuelve `SPELL_FAILED_LINE_OF_SIGHT` si vmap LoS bloqueado
- [ ] Test: `EffectSchoolDMG` aplica school resist según SpellSchoolMask::Fire vs target.fire_resist
- [ ] Test: `EffectHeal` respeta crit chance y overheal log
- [ ] Test: `EffectApplyAura` crea Aura, target.has_aura(spell_id) == true
- [ ] Test: `EffectTeleportUnits` mueve player a SpellTargetPosition.x/y/z/o exactos
- [ ] Test: `EffectDispel` quita N auras según DispelMask + DispelChance roll
- [ ] Test: `Aura::TryRefreshStackOrCreate` con stack_amount = 5 ya en target → refresh duration, no crea segunda
- [ ] Test: `Aura::Update` con duration_ms=10000, tick periodic_amplitude=2000 → 5 ticks en 10s
- [ ] Test: `AuraEffect::PeriodicTick` (DoT) aplica daño cada N ms con SpellDamageBonus
- [ ] Test: `Aura::Remove(AuraRemoveMode::Dispel)` envía SMSG_AURA_UPDATE con removed_aura_slots
- [ ] Test: `SpellHistory::HasCooldown` devuelve true durante GCD (1500ms post-cast)
- [ ] Test: `SpellHistory::AddCooldown` con cooldown_end_ms = now+5000 → HasCooldown true por 5s, false después
- [ ] Test: GCD per-school: castar Fire después de Frost no comparte GCD con Frost ni con Holy
- [ ] Test: Category cooldown: Hunter Aspects (cat 21) — cambiar Aspect of the Hawk → Aspect of the Cheetah pone ambos en cd compartido
- [ ] Test: Charges: spell con 3 charges, castar 3x → tercer cast pone categoría en cd hasta que recover
- [ ] Test: Proc system: equipped Spellpower weapon con SpellProcEntry → daño melee triggea Spellpower con PpmRate correcto
- [ ] Test: ICD: aura con InternalCooldown=10s no procea 2 veces dentro de 10s aunque dispare condición 2x
- [ ] Test: Channel: SMSG_CHANNEL_START enviado, ticks llegan cada periodic_amplitude, SMSG_CHANNEL_UPDATE al final
- [ ] Test: Interrupt on move: jugador empieza cast (3s), se mueve a 1s → SMSG_CAST_FAILED con SPELL_FAILED_MOVING
- [ ] Test: Interrupt on damage: target casteando, recibe daño suficiente → cast cancelled si ChannelInterruptFlags::Damage
- [ ] Test: Push-back: damage during cast → cast_time += 500ms (clampa max), SMSG_SPELL_DELAYED enviado
- [ ] Test: AoE cap: spell con MaxAffectedTargets=5, 10 enemigos en range → 5 hits exactos
- [ ] Test: Persistencia: aura con duration 30min se serializa a `character_aura`, al login se restaura con remaining_time correcto

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 8 files / 15660 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp` | `crates/wow-spell/`, `crates/wow-world/src/handlers/spell.rs`, `crates/wow-packet/src/packets/{spell,aura}.rs` \| 🔧 broken (rewrite needed) — sólo cast handler básico + cooldown visible; crate `wow-spell` está **vacío**. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SPELLS.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 8 C++ files / 15660 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SPELLS.DIV.002` | `crates/wow-packet/src/packets/{spell,aura}.rs` (`declared_pattern`, 0 Rust lines) | 8 C++ files / 15660 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |
| `#SPELLS.DIV.003` | `crates/wow-spell/src/lib.rs` (`exists_empty`, 0 Rust lines) | 8 C++ files / 15660 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SPELLS.DIV.004` | `crates/wow-data/src/spell_info.rs` (`missing_declared_path`, 0 Rust lines) | 8 C++ files / 15660 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.h`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellScript.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **GCD per-school no es por-spell:** El GCD agrupa por `SpellSchool` (NORMAL/HOLY/FIRE/...) — un Frost Bolt comparte GCD con Frost Nova pero NO con Holy Light. La implementación actual de RustyCore (single global `last_spell_cast_time`) es trivialmente incorrecta y bloqueará casts legales (Spell.cpp:3475-3520, SpellHistory.cpp:560).
- **Category cooldown ≠ spell cooldown:** Algunos spells comparten un `Category` (Hunter Aspects, Paladin Seals); poner una de un grupo en cd pone TODA la categoría. Hay 3 ejes: GCD, spell cd, category cd; los 3 conviven (SpellHistory.cpp:HandleCooldowns).
- **Charges:** Spells con multi-cast (Mind Flay, Avenger's Shield) usan SpellCategoryRecoveryTime + ChargeRecoveryCategory. Cuando consumes la última charge, la categoría entra en cooldown hasta que se regenera (SpellHistory.cpp:ConsumeCharge).
- **Proc events:** El proc system es donde más bugs históricos hay. ProcFlags (~30 bits) define el evento (DONE_MELEE_AUTO_ATTACK, TAKEN_PERIODIC, …). Cada evento tiene un ProcEventInfo, y CADA aura activa con SpellProcEntry matching dispara su trigger spell. El roll usa Chance OR PpmRate (PPM normalizado por weapon speed). Hay un ICD per-aura via SpellAuraOptions.ProcCharges para evitar que un proc se dispare 100 veces por segundo (SpellMgr.cpp:CanSpellTriggerProcOnEvent, Unit.cpp:ProcSkillsAndAuras).
- **PPM normalization:** PPM real = base_ppm × (weapon_speed / 60). Sin esto, un slow weapon proc al doble. RustyCore tendrá que portar exactamente la fórmula.
- **Dispel mechanics:** `DispelType` no es lo mismo que `DispelMask`. El cast tiene mask de qué types ataca; el aura tiene su DispelType. Además hay `SPELL_AURA_DISPEL_IMMUNITY` que bloquea por type. Dispel también tiene chance per aura, hay ICD entre dispels mass-dispel (Spell.cpp:EffectDispel).
- **School separation:** El daño se aplica school-by-school; un FrostFire bolt (mask=FROST|FIRE) hace daño split. El target resiste cada school separadamente. Absorb shields tienen `SchoolMask` y solo absorben matching schools (SpellAuraEffects.cpp:HandleSchoolAbsorb).
- **Channel interrupt:** `ChannelInterruptFlags` es separado de `InterruptFlags` (que es para casts). Un channel se interrumpe por movimiento, daño taken (con threshold), pushback, jumping, mounting, dismounting. Las flags están en SpellInterrupts.db2 (Spell.cpp:CheckSpellCancel).
- **AoE caps:** SpellTargetRestrictions.MaxAffectedTargets limita N targets simultáneos; pero el algoritmo de selección no es trivial — para friendly heals se prioriza low health, para enemies prioriza closest, depende de `Targets[A/B]` semantics (Spell.cpp:SearchTargets).
- **Target validation:** Hay 150+ implicit target types (TARGET_UNIT_TARGET_ENEMY, TARGET_DEST_DYNOBJ_NONE, TARGET_SRC_CASTER, TARGET_UNIT_DEST_AREA_ENEMY, …). Cada uno tiene su semántica de origen + filtro de quién es válido. Bug clásico: confundir TARGET_DEST_TARGET_ENEMY (toma posición del target) vs TARGET_UNIT_TARGET_ENEMY (toma el unit) (SpellInfo.cpp:_LoadImplicitTargetTypes).
- **Triggered spells stack:** Spell A castea con `TriggerSpell = B`; B se ejecuta sin gastar power, sin aparecer en SMSG_SPELL_GO normal, sin GCD. Mantener `m_triggeredByAuraSpell` para diferenciar (Spell.cpp:Spell ctor).
- **Persistencia race:** Auras se guardan en player save (cada N min). Si server crashea entre tick y save, dura un poco más; OK. Pero cooldowns guardan `cooldown_end_unix_time`, no `remaining_ms` — si el server cambia clock, todo bug (CharacterDatabase saved spell cooldown).
- **Death + auras:** Al morir, auras con `SPELL_ATTR3_DEATH_PERSISTENT` no se quitan; resto sí. AuraRemoveMode debe ser `BY_DEATH`. Bug común: olvidarse de iterar y quitar auras de objetos linked (focus, vehicles, charm).
- **AuraApplication vs Aura:** Una Aura puede afectar 1 o N targets (party buffs, area auras). Cada target tiene un `AuraApplication` (slot UI, flags). Borrar la Aura requiere _Remove de todos los AuraApplications. Olvidar uno = ghost aura visible cliente, no en server (SpellAuras.cpp:Aura::~Aura).
- **Atributos múltiples:** SPELL_ATTR0..14 son 15 grupos; pueden estar en SpellMisc.db2 directamente o en override per-spell. Algunos atributos están duplicados/redefinidos entre versiones de WoW; en 3.4.3 son los listados en SharedDefines.h. Verifica el correcto target version del extractor cuando portas DB2.
- **WotLK 3.4.3 vs retail:** Esta es la versión Wrath Classic. **NO tiene** trait/talent system de Dragonflight — TraitMgr.cpp/.h existe pero está stubbed/legacy y NO debe portarse. Sí tiene glyphs. El sistema de spec-aware spells es talents-based, no traits.
- **Performance hotspots:** El Spell ctor + Spell::prepare está en el hot path (cada cast). Evita allocations dentro de target list build; reusa scratch buffers. C++ usa stack arrays donde puede; Rust debe preferir SmallVec o pre-allocate Vec con capacity (Spell.cpp top of file usa varias arenas).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Spell` | `struct Spell` en `crates/wow-spell/src/spell.rs` | Estado de cast en curso; campo `caster: Arc<RwLock<Unit>>`, `targets: SpellCastTargets`, `spell_info: Arc<SpellInfo>`, `state: SpellState` |
| `class SpellInfo` | `struct SpellInfo` en `crates/wow-spell/src/spell_info.rs` | Compartido vía `Arc<SpellInfo>`; cargado una vez; immutable después de load |
| `class Aura` | `struct Aura` en `crates/wow-spell/src/aura.rs` | `effects: Vec<AuraEffect>`, `applications: HashMap<Guid, AuraApplication>`, `Arc<RwLock<Aura>>` para shared mutable |
| `class AuraApplication` | `struct AuraApplication` | POD per-target del aura (slot, flags, removed_mode) |
| `class AuraEffect` | `struct AuraEffect` | Un effect index dentro de Aura; tick logic, amount, periodic_timer |
| `class SpellHistory` | `struct SpellHistory` (campo en `Unit`) | `cooldowns: HashMap<u32, CooldownEntry>`, `category_cooldowns: HashMap<u32, Instant>`, `gcd: HashMap<SpellSchool, Instant>`, `charges: HashMap<u32, ChargeEntry>` |
| `class SpellMgr` (singleton) | `static SPELL_MGR: OnceLock<SpellMgr>` o pasado como `&SpellMgr` | Carga DB2 + tablas SQL al startup; immutable en runtime |
| `enum SpellEffectName` | `enum SpellEffect` con `#[repr(u32)]` | Valores numericos = ids C++ |
| `enum AuraType` | `enum AuraType` con `#[repr(u32)]` | Idem |
| `enum SpellCastResult` | `enum SpellCastResult` con `#[repr(u32)]` | ~250 variantes; usar `#[non_exhaustive]` |
| `enum SpellAttr0..14` | `bitflags!{}` 15 structs | Una macro `bitflags!` por grupo |
| `SpellEffectHandlers[151]` (función pointer table) | `match spell_effect { ... }` o `[fn(&mut Spell, SpellEffIndex); 151]` | Match es más idiomático y permite borrows seguros |
| `void Spell::Effect*()` | `fn effect_*(&mut self, idx: SpellEffIndex)` | Métodos sobre `Spell` |
| `Aura::TryRefreshStackOrCreate(...)` | `fn try_refresh_stack_or_create(...) -> Arc<RwLock<Aura>>` | Devuelve handle a aura existente o nueva |
| `Aura::Update(uint32, Unit*)` | `fn update(&mut self, diff_ms: u32, owner: &mut Unit)` | Tick periódico |
| `SpellHistory::HasCooldown(uint32, uint32)` | `fn has_cooldown(&self, spell_id: u32, item_id: u32) -> bool` | Hot path; NO RwLock, sólo lectura inmutable |
| `SpellHistory::AddGlobalCooldown(SpellInfo const*, uint32)` | `fn add_global_cooldown(&mut self, info: &SpellInfo, duration: Duration)` | `Instant::now() + duration` |
| `WorldDatabase.Query("SELECT ... FROM spell_proc")` | `sqlx::query_as!(SpellProcRow, "SELECT ... FROM spell_proc").fetch_all(&pool).await` | Async load en startup |
| `m_spellInfoMap[spellId]` | `spell_mgr.get_spell_info(spell_id) -> Option<&Arc<SpellInfo>>` | DashMap o HashMap detrás |
| `Unit::CastSpell(spellId, target)` | `unit.cast_spell(spell_id, target).await -> Result<(), SpellCastResult>` | Async porque puede esperar a CheckLineOfSight |
| `Spell::SendSpellStart()` | `fn send_spell_start(&self, broadcaster: &MapBroadcaster)` | Emite SMSG_SPELL_START a vecinos |
| `Spell::SendCastResult(SpellCastResult)` | `fn send_cast_result(&self, result: SpellCastResult)` | A caster session únicamente |
| `class SpellScript` (DSL hooks) | `trait SpellScript { fn before_cast(&self, ...); fn on_effect_hit(&self, ...); ... }` | Implementado por structs en `crates/wow-scripts/`, registrados via `inventory::submit!` |
| `SpellInfo::HasAttribute(SpellAttr0::CANCELS_AUTO_ATTACK_COMBAT)` | `info.attr0.contains(SpellAttr0::CANCELS_AUTO_ATTACK_COMBAT)` | bitflags `.contains` |

---

*Template version: 1.0 (2026-05-01).* Initial complete audit port.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Spells/` (`Spell.{h,cpp}` ~10k lines, `SpellInfo.{h,cpp}` ~5.6k, `SpellMgr.{h,cpp}` ~5.8k, `SpellEffects.cpp` ~6k, `SpellHistory.{h,cpp}` ~1.3k, `SpellScript.{h,cpp}` ~3.5k) and the Auras subtree (`Auras/SpellAuras.{h,cpp}` ~3k, `Auras/SpellAuraEffects.{h,cpp}` ~6.7k, `Auras/SpellAuraDefines.h`) against the Rust workspace at `/home/server/rustycore/crates/`.

**Empty-crate finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` measures **exactly 0 lines** (verified via `wc -l`). The crate is registered in `Cargo.toml` and listed as a workspace member but contains **no code at all**: no `Spell`, no `SpellInfo`, no `SpellMgr`, no `SpellHistory`, no `Aura`, no `AuraEffect`, no `AuraApplication`, no `SpellScript`, no `SpellCastResult` enum, no `SpellMissInfo`, no `AuraType`, no `SpellEffect` enum, no `SpellSchool/Mask`, no `SpellAttr0..14` bitflags, no `Mechanics`. The 44k+ lines of C++ spell engine map to **zero lines** of Rust engine.

**What exists outside the empty crate.**
- `crates/wow-world/src/handlers/spell.rs` (288 lines) — single `handle_cast_spell` that parses `CastSpellRequest`, checks a `HashSet<u32> known_spells`, checks one global `last_spell_cast_time` plus per-spell `last_spell_cast_time_per_spell: HashMap<u32, Instant>`, sends either `SMSG_SPELL_START` (if cast time > 0) or jumps to a stub `execute_spell()`. `CMSG_CANCEL_CAST` and `CMSG_CANCEL_CHANNELLING` are registered as inplace handlers but their bodies are no-op stubs.
- `crates/wow-packet/src/packets/spell.rs` (~466 lines) — wire structs only (`CastSpellRequest`, `SpellStartPkt`, `CastFailed`, `SpellTargetData`, `SpellCastVisual`).
- `crates/wow-packet/src/packets/aura.rs` (~123 lines) — `AuraData` POD plus `AuraUpdate` writer; one round-trip test. **No server-side aura state at all** — the writer is fed manually by callers, not driven by a real `Aura::Update` tick.

**Spell effects implemented.** **0 of ~151.** A grep for `EffectSchoolDMG`, `EffectHeal`, `EffectApplyAura`, `EffectTeleportUnits`, `effect_*`, `Effect` inside `crates/wow-world/src/handlers/spell.rs` and `crates/wow-spell/` returns empty. There is no dispatch table, no `match spell_effect`, no `SpellEffectHandlers[151]`. The 151-entry switch in C++ `SpellEffects.cpp` (one giant function per `SPELL_EFFECT_*` ID — `SCHOOL_DAMAGE`, `HEAL`, `APPLY_AURA`, `TELEPORT_UNITS`, `SUMMON`, `DISPEL`, `INTERRUPT_CAST`, `KNOCKBACK`, `JUMP`, `CHARGE`, `ENERGIZE`, etc.) has zero analog. The handler's stub `execute_spell(spell_id, target_guid)` is a name only — the body referenced from `session.rs` does not apply damage, healing, auras, teleport, summon, or any other effect; it's plumbing without payload.

**Auras implemented.** **0 of ~280 `AuraType` handlers.** No `HandlePeriodicDamage`, no `HandleAuraModStat`, no `HandleSchoolAbsorb`, no `HandleModConfuse`/`Fear`/`Stun`/`Silence`/`Root`, no `HandleShapeshift`, no `HandleProcTriggerSpell`, no `HandleCharm`. The packet `AuraData` is a wire shape with no lifecycle behind it: no `Aura::Create`, no `_ApplyForTarget`, no `Update` (so DoT/HoT never tick), no `Remove(AuraRemoveMode)`, no stacking via `TryRefreshStackOrCreate`, no persistence to `character_aura`. Sending `AuraUpdate` to a client desynchronizes the moment the aura should expire because there is no server tracker.

**SpellMgr / SpellInfo / SpellHistory.** No live `SpellMgr` startup exists yet. Several represented DB2/SQL stores now exist in `wow-data` (`spell_required`, `spell_learn_spell`, `spell_threat`, `spell_target_position`, `spell_group`, `spell_group_stack_rules`, `spell_area`, `spell_custom_attr` SQL overrides, `serverside_spell`/`serverside_spell_effect` staging, explicit `spell_proc` rows, rank chains via `SkillLineAbilityEntry::SupercedesSpell`, etc.), but implicit proc-entry generation, derived `AttributesCu` computation, live server-only `SpellInfo` insertion, live chain-entry patching, live spell-area runtime consumption, and `spell_script_names` still lack live loader coverage. The minimal `wow_data::SpellInfo` referenced from the handler exposes only a subset of the ~80 fields C++ `SpellInfo` carries.

**Cooldowns / GCD / charges.** A single global `last_spell_cast_time: Option<Instant>` plus per-spell `last_spell_cast_time_per_spell: HashMap<u32, Instant>` substitute for what C++ implements as three orthogonal axes: `_globalCooldowns: map<SpellSchool, time>` (per-school 1.5s GCD modulated by haste), `_categoryCooldowns: map<u32 cat, time>` (Hunter Aspects, Paladin Seals share a category cd), and `_categoryCharges: map<u32 cat, ChargeEntry>` (multi-cast spells like Mind Flay's 3 charges). The Rust collapse-everything-into-one-timer model will block legal casts (e.g. casting Holy Light right after Frost Bolt — different schools, should not share GCD) and allow illegal ones (casting two spells in the same category back-to-back). `SMSG_SPELL_COOLDOWN`, `SMSG_COOLDOWN_EVENT`, `SMSG_CLEAR_COOLDOWN`, `SMSG_MODIFY_COOLDOWN` are not emitted. Cooldown persistence to `character_spell_cooldown` does not exist, so all cooldowns reset at logout.

**Worst divergence.** The handler can be summarized as **"acknowledge a cast, set a per-id timer, send a packet, do nothing"** — there is no execution side at all. None of the consequences a player expects after casting actually occur server-side: the target's HP is not changed, no aura is applied or refreshed, no DoT/HoT ticks, no power is spent (no `CheckPower` / `TakePower`), no range/LoS/items/reagents/shapeshift validation runs, no `SpellMissInfo` roll is performed (every cast is a guaranteed hit by absence of logic), no projectile travel time is honored, no channel ticks are scheduled, no procs trigger, and no GCD-per-school is enforced. Together these mean the spell engine has no simulation behaviour whatsoever — `wow-spell` is the largest single greenfield in the workspace, with §9 tasks spanning #SPELLS.1 → #SPELLS.68 (multiple XL).
