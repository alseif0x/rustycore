# Migration: Spells / Auras (sub-module)

> **C++ canonical path:** `src/server/game/Spells/Auras/`
> **Rust target crate(s):** `crates/wow-spell/` (módulo `aura`), `crates/wow-packet/src/packets/aura.rs`
> **Layer:** L5 sub-module of `spells.md` (Game systems — combat / spells / auras)
> **Status:** ❌ not started — `wow-spell` está en 0 líneas; sólo existe `AuraData` POD para wire en `wow-packet/src/packets/aura.rs` sin estado server-side
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing — see §13)
> **Last updated:** 2026-05-01

> **Parent doc:** [`spells.md`](./spells.md) — overview del motor entero (Spell + SpellInfo + SpellMgr + SpellHistory + Auras combinados, ~44k líneas C++).
> **Related sub-docs:** [`spells-effects.md`](./spells-effects.md), [`spells-cast.md`](./spells-cast.md).
> **Cross-link:** este motor de auras se aplica/quita sobre `Unit` — ver [`entities-unit.md`](./entities-unit.md) para el contenedor (`m_appliedAuras`, `m_ownedAuras`, `m_modAuras[AuraType]`, `Unit::AddAura`, `Unit::RemoveAurasDueToSpell`, `Unit::HasAuraType`).

---

## 1. Purpose

El sub-módulo Auras gestiona el **estado persistente de buffs/debuffs** sobre un Unit (o DynamicObject) generado por hechizos. Una `Aura` es una instancia activa con duración, charges, stack count y un vector de `AuraEffect` (uno por effect index). Es responsable del **lifecycle** (apply / refresh / remove / expire), del **tick periódico** (DoT damage, HoT heal, energize, drain, periodic trigger spell), de las **stacking rules** (refresh existing vs create new vs replace per `SpellGroup`), de **dispel** (mark dispellable según `DispelType` + chance), del **proc system** (cuando el aura tiene `SpellProcEntry`, dispara trigger spells por evento), y de la **persistencia** (`character_aura` save/load por logout/login).

Cubre los ~280 `AuraType` handlers en `SpellAuraEffects.cpp` — uno por cada `SPELL_AURA_*` (~190 con código real, el resto stub/NYI). Es el segundo más grande dentro de Spells después del propio `Spell` runtime.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Spells/Auras/SpellAuraDefines.h` | 734 | `prefix` |
| `game/Spells/Auras/SpellAuraEffects.cpp` | 6342 | `prefix` |
| `game/Spells/Auras/SpellAuraEffects.h` | 410 | `prefix` |
| `game/Spells/Auras/SpellAuras.cpp` | 2665 | `prefix` |
| `game/Spells/Auras/SpellAuras.h` | 398 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Spells/Auras/SpellAuraDefines.h` | 734 | `enum AuraType` (~280 SPELL_AURA_*), `enum AuraEffectHandleModes` (DEFAULT/REAL/SEND_FOR_CLIENT/CHANGE_AMOUNT/REAPPLY/STAT/SKILL + masks), `enum AuraRemoveMode` (BY_DEFAULT/INTERRUPT/CANCEL/ENEMY_SPELL/EXPIRE/DEATH), `enum AuraFlags` (NEGATIVE/POSITIVE/PASSIVE/NOCASTER), `enum AuraStateType`, `DAMAGE_ABSORB_TYPE`, `AuraTriggerOnPowerChangeDirection`, `AuraTriggerOnHealthChangeDirection` |
| `src/server/game/Spells/Auras/SpellAuras.h` | 398 | `class AuraApplication` (per-target binding: slot, flags, effectMask, removeMode), `class Aura` (instancia owner-side: spellInfo, m_duration, m_maxDuration, m_procCharges, m_stackAmount, m_applications: ApplicationMap, m_loadedScripts), `class UnitAura` (subclass owner=Unit), `class DynObjAura` (subclass owner=DynamicObject), `struct AuraKey`, `struct AuraLoadEffectInfo`, `struct AuraCreateInfo`, typedef `ApplicationMap`, typedef `AuraEffectVector` |
| `src/server/game/Spells/Auras/SpellAuras.cpp` | 2665 | Lifecycle implementation: `Aura::TryRefreshStackOrCreate`, `Aura::Create`, `_ApplyForTarget`, `_UnapplyForTarget`, `_Remove`, `UpdateTargetMap`, `Update(diff, caster)`, `RefreshDuration`, `RefreshTimers`, `SetCharges`/`ModCharges`/`DropCharge`, `SetStackAmount`/`ModStackAmount`, `HandleAllEffects`, `HandleAuraSpecificMods`, proc plumbing (`AddProcCooldown`, `PrepareProcToTrigger`, `TriggerProcOnEvent`, `CalcProcChance`, `CalcPPMProcChance`), AuraScript dispatch (~30 `CallScript*Handlers`) |
| `src/server/game/Spells/Auras/SpellAuraEffects.h` | 410 | `class AuraEffect` (un effect index dentro de Aura: m_amount, m_baseAmount, m_periodicTimer, m_amplitude, m_tickNumber, m_canBeRecalculated, m_isPeriodic), declaraciones de los ~280 `HandleXxx` handlers + `Apply*` overloads + `PeriodicTick`, `HandleProc`, `HandleShapeshiftBoosts` |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | 6342 | Implementación de los **190 handlers** `void AuraEffect::HandleXxx(AuraApplication const* aurApp, uint8 mode, bool apply) const` — uno por cada AuraType usado: `HandlePeriodicDamage`, `HandlePeriodicHeal`, `HandlePeriodicEnergize`, `HandleAuraModStat`, `HandleAuraModResistance`, `HandleAuraModSpeed`, `HandleAuraModConfuse`, `HandleAuraModFear`, `HandleModRoot`, `HandleModStun`, `HandleAuraModSilence`, `HandleAuraModPacify`, `HandleAuraModDisarm`, `HandleSchoolAbsorb`, `HandleManaShield`, `HandleAuraModShapeshift`, `HandleAuraTransform`, `HandleModInvisibility`/`Detect`, `HandleModStealth`/`Detect`, `HandleAuraMounted`, `HandleFeignDeath`, `HandleAuraGhost`, `HandleSpiritOfRedemption`, `HandlePhase`/`Group`/`AlwaysVisible`, `HandleCharm`, `HandleAuraModPossess`, `HandleProcTriggerSpell`, `HandlePeriodicTriggerSpell`, `HandleAuraDummy`, `HandleAuraTrack*`, `HandleModThreat`, `HandleModTaunt`, `HandleModDetaunt`, `HandleAuraModFixate`, `HandleAuraTransformingFlight`, etc. + `PeriodicTick` (~150-line dispatcher por AuraType para periodic effects), `HandleProc` (proc dispatcher por AuraType) |

**Total Auras/:** ~9,849 líneas (incluyendo headers + comments).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Aura` | abstract class | Instancia activa de una aura — owns `AuraEffectVector`, `ApplicationMap`. Vida: nace en `EffectApplyAura` o `TryCreate`, muere en `_Remove` |
| `UnitAura` | class : Aura | Aura cuyo owner es un `Unit` (la mayoría — buffs, debuffs, transformations, periodic) |
| `DynObjAura` | class : Aura | Aura ligada a un `DynamicObject` (Consecration, Blizzard, Mark of the Wild ground area) |
| `AuraApplication` | class | Vínculo per-target de una `Aura`. 1 Aura puede tener N AuraApplication si afecta una party. Carries: `_target`, `_base: Aura*`, `_slot` (UI slot 0-256), `_flags: uint16` (AFLAG_*), `_effectMask` (qué effects aplicar a este target específico), `_removeMode`, `_needClientUpdate` |
| `AuraEffect` | class | Un effect index (0..MAX_SPELL_EFFECTS) dentro de una Aura. Carries: `m_spellInfo`, `m_effIndex`, `m_amount` (computed amount), `m_baseAmount`, `m_periodicTimer`, `m_amplitude`, `m_tickNumber`, `m_canBeRecalculated`, `m_isPeriodic`. Tiene `HandleEffect(true/false)` que despacha por AuraType y `PeriodicTick` |
| `AuraCreateInfo` | struct | POD que carga `Aura::Create`/`TryRefreshStackOrCreate`: SpellInfo, caster, owner, castDifficulty, effMask, baseAmount, castItemGuid, castItemLevel, isRefresh |
| `AuraKey` | struct | DB primary key: (Caster, Item, SpellId, EffectMask). Para `character_aura` |
| `AuraLoadEffectInfo` | struct | `std::array<int32, MAX_SPELL_EFFECTS> Amounts` + `BaseAmounts` para SetLoadedState |
| `AuraType` | enum uint32 | ~280 valores: NONE=0, BIND_SIGHT=1, MOD_POSSESS=2, PERIODIC_DAMAGE=3, DUMMY=4, MOD_CONFUSE=5, MOD_CHARM=6, MOD_FEAR=7, PERIODIC_HEAL=8, MOD_ATTACKSPEED=9, MOD_THREAT=10, MOD_TAUNT=11, MOD_STUN=12, MOD_DAMAGE_DONE=13, MOD_DAMAGE_TAKEN=14, DAMAGE_SHIELD=15, MOD_STEALTH=16, MOD_INVISIBILITY=18, MOD_RESISTANCE=22, PERIODIC_TRIGGER_SPELL=23, PERIODIC_ENERGIZE=24, MOD_PACIFY=25, MOD_ROOT=26, MOD_SILENCE=27, REFLECT_SPELLS=28, MOD_STAT=29, MOD_INCREASE_SPEED=31, MOD_DECREASE_SPEED=33, MOD_INCREASE_HEALTH=34, MOD_INCREASE_ENERGY=35, MOD_SHAPESHIFT=36, EFFECT_IMMUNITY=37, SCHOOL_IMMUNITY=39, DAMAGE_IMMUNITY=40, DISPEL_IMMUNITY=41, PROC_TRIGGER_SPELL=42, PROC_TRIGGER_DAMAGE=43, MOD_PARRY_PERCENT=49, MOD_DODGE_PERCENT=51, MOD_BLOCK_PERCENT=52, MOD_CRIT_PERCENT=53, PERIODIC_LEECH=53?, MOD_HIT_CHANCE=55, TRANSFORM=56, SCHOOL_ABSORB=69, MANA_SHIELD=66, MOUNTED=78, MOD_DAMAGE_PERCENT_DONE=79, …, hasta SPELL_AURA_544 + TOTAL_AURAS bookend |
| `AuraEffectHandleModes` | enum | Bitmask: DEFAULT=0, REAL=0x01 (apply/remove on unit), SEND_FOR_CLIENT=0x02 (broadcast packet), CHANGE_AMOUNT=0x04 (re-recalc), REAPPLY=0x08 (refresh on existing), STAT=0x10, SKILL=0x20, + composite masks |
| `AuraRemoveMode` | enum | NONE=0, BY_DEFAULT=1 (script remove), BY_INTERRUPT=2 (aura interrupt flag), BY_CANCEL=3 (player cancel), BY_ENEMY_SPELL=4 (dispel/absorb destroy), BY_EXPIRE=5 (duration ended), BY_DEATH=6 |
| `AuraFlags` | enum | AFLAG_NONE, AFLAG_NOCASTER=0x01 (selfcast), AFLAG_POSITIVE=0x100, AFLAG_PASSIVE=0x200, AFLAG_DURATION=0x04, AFLAG_SCALABLE=0x08, AFLAG_NEGATIVE=0x20 |
| `AuraStateType` | enum | DEFENSE=1, HEALTHLESS_20_PERCENT=2, BERSERKING=3, FROZEN=4, JUDGEMENT=5, HUNTER_PARRY=6, WARRIOR_VICTORY_RUSH=7, FAERIE_FIRE=8, HEALTHLESS_35_PERCENT=9, RAID_ENCOUNTER_PERCENT=10, BLEED=11, ENRAGE=18 — usado para `aura_state`-gated spells |
| `DAMAGE_ABSORB_TYPE` | enum | ALL_DAMAGE_ABSORB=-2, ONLY_MAGIC_ABSORB=-1, otherwise SchoolMask |
| `AuraTriggerOnPowerChangeDirection` | enum class | Gain=0, Loss=1 |
| `AuraTriggerOnHealthChangeDirection` | enum class | Above=0, Below=1 |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Aura::TryRefreshStackOrCreate(AuraCreateInfo&, bool updateEffectMask)` | **Entry-point**. Si ya existe aura compatible refresca duration+stack; si no, llama `Create`. Es el path que `Spell::EffectApplyAura` usa | `Aura::TryCreate`, `Aura::Create`, `RefreshDuration`, `ModStackAmount` |
| `Aura::Create(AuraCreateInfo&)` | Construye `UnitAura` o `DynObjAura` según owner type, llama `_InitEffects`, registra en target | `new UnitAura/DynObjAura`, `_InitEffects`, `target->_AddAura` |
| `Aura::_InitEffects(uint32 effMask, Unit* caster, int32 const* baseAmount)` | Crea `AuraEffect[]` por bit en effMask, calcula amount inicial | `new AuraEffect`, `CalcAmount` |
| `Aura::_ApplyForTarget(Unit* target, Unit* caster, AuraApplication*)` | Aplica todos los effect handlers en mode=REAL+SEND, broadcast SMSG_AURA_UPDATE | `AuraApplication::_HandleEffect`, `target->_ApplyAuraEffect`, `SetNeedClientUpdate` |
| `Aura::_UnapplyForTarget(Unit* target, Unit* caster, AuraApplication*)` | Quita all effects mode=REAL+SEND para este target | `AuraApplication::_HandleEffect(false)`, `target->_RemoveAuraEffect` |
| `Aura::_Remove(AuraRemoveMode)` | Borrado completo: itera `m_applications`, llama `_UnapplyForTarget` por cada, marca `m_isRemoved`, dispara AuraScript OnRemove | `_UnapplyForTarget`, `CallScriptAfterEffectRemoveHandlers` |
| `Aura::Update(uint32 diff, Unit* caster)` | **Tick principal**. Resta `m_duration`, llama `m_loadedScripts` update, dispara `AuraEffect::Update` (que llama `PeriodicTick`), expira si duration<=0 | `AuraEffect::Update`, `PeriodicTick`, `IsExpired`, `Remove(AURA_REMOVE_BY_EXPIRE)` |
| `Aura::UpdateTargetMap(Unit* caster, bool apply)` | Cada `UPDATE_TARGET_MAP_INTERVAL` (500ms) recalcula targets (área auras se mueven con el caster) | `FillTargetMap` (virtual UnitAura/DynObjAura), `_ApplyForTarget`, `_UnapplyForTarget` |
| `Aura::SetDuration(int32, bool withMods)` | Cambia duration, opcionalmente aplica DurationMod auras (Stamina aura suele ser 1.5x) | `SetNeedClientUpdateForTargets` |
| `Aura::RefreshDuration(bool withMods)` | Reinicia `m_duration = m_maxDuration`, dispara timer reset | `SetDuration`, `RefreshTimers` |
| `Aura::RefreshTimers(bool resetPeriodic)` | Reset periodic timers (DoT pandemic logic — si <30% restante, resetea fully; si >=30%, suma) | `AuraEffect::ResetPeriodic` |
| `Aura::SetCharges(uint8)` / `ModCharges(int32, AuraRemoveMode)` | Set/modifica `m_procCharges`. Si llega a 0, fuerza `Remove(removeMode)` | `_Remove` |
| `Aura::DropCharge(AuraRemoveMode)` | Equiv a `ModCharges(-1)` | `ModCharges` |
| `Aura::SetStackAmount(uint8)` / `ModStackAmount(int32, removeMode, resetPeriodic)` | Modifica `m_stackAmount`. Llama `RecalculateAmountOfEffects` (multi-stack auras escalan amount) | `RecalculateAmountOfEffects`, `RefreshTimers` |
| `Aura::HandleAllEffects(AuraApplication*, uint8 mode, bool apply)` | Itera AuraEffectVector y llama `HandleEffect` por cada effect del effMask de la application | `AuraEffect::HandleEffect` (despacha al ~190 HandleXxx por AuraType) |
| `Aura::HandleAuraSpecificMods(AuraApplication const*, Unit* caster, bool apply, bool onReapply)` | Hook para mods específicos al apply/remove (modify schools, summon icons, etc.) — código spaghetti per spellId | Many — diversos spellId checks |
| `Aura::CanStackWith(Aura const* existing) const` | Decide si dos auras pueden coexistir en mismo target. Considera: same SpellInfo, same caster, SpellGroup rules, SpellArea, family flags | `SpellMgr::CheckSpellGroupStackRules`, SpellInfo flags |
| `Aura::IsProcOnCooldown(TimePoint now)` / `AddProcCooldown` / `ResetProcCooldown` | ICD per-aura para procs (e.g. 10s ICD evita procs repetidos) | `m_procCooldown` |
| `Aura::PrepareProcToTrigger(AuraApplication*, ProcEventInfo&, TimePoint)` | Pre-roll antes de ejecutar trigger spell del proc | `CallScriptCheckProcHandlers`, `CalcProcChance` |
| `Aura::PrepareProcChargeDrop(SpellProcEntry const*, ProcEventInfo const&)` | Marca charge drop pendiente post-trigger | `m_procCharges` |
| `Aura::ConsumeProcCharges(SpellProcEntry const*)` | Consume charges al success del proc | `ModCharges` |
| `Aura::TriggerProcOnEvent(uint32 procEffectMask, AuraApplication*, ProcEventInfo&)` | Ejecuta el trigger spell del proc | `caster->CastSpell(triggerSpell)`, `CallScriptProcHandlers` |
| `Aura::CalcProcChance(SpellProcEntry const&, ProcEventInfo&)` | Roll de chance: PPM o flat, modificado por aura mods | `CalcPPMProcChance` |
| `Aura::CalcPPMProcChance(Unit* actor)` | PPM normalizado por weapon speed: `chance = ppm * weaponSpeed / 60` | weapon speed lookup |
| `Aura::GetProcEffectMask(AuraApplication*, ProcEventInfo&, TimePoint)` | Bitmask de qué effects realmente proccaron | `CallScriptCheckEffectProcHandlers` |
| `Aura::CalcDispelChance(Unit const* target, bool offensive)` | Chance que un dispel le pegue a esta aura | SpellInfo dispel rates |
| `Aura::IsSingleTarget()` / `IsSingleTargetWith(Aura const*)` | Single-target debuffs (Hunter Mark): solo uno activo per caster | `m_isSingleTarget` |
| `Aura::UnregisterSingleTarget()` | Limpieza al expirar | caster's m_scAuras |
| `Aura::GenerateKey(uint32& recalculateMask) const` | Construye `AuraKey` para persistencia | — |
| `Aura::SetLoadedState(maxDuration, duration, charges, stackAmount, recalcMask, amount[])` | Restore desde DB al login | sets fields |
| `Aura::CanBeSaved() const` | True si esta aura es persistible (no-passive, no-shapeshift, no-vehicle, etc.) | spellInfo flags |
| `Aura::CallScript*` (~30 hooks) | Dispatch a `AuraScript`: OnDispel, AfterEffectApply/Remove, OnEffectPeriodic, EffectCalcAmount, EffectCalcPeriodic, EffectAbsorb, ManaShield, Split, EnterLeaveCombat, CheckProc, PrepareProc, OnProc, EffectProc | `m_loadedScripts` iteration |
| `Aura::IsExpired()` / `IsPermanent()` / `IsPassive()` / `IsDeathPersistent()` / `IsRemovedOnShapeLost()` / `IsArea()` | Predicates frecuentes | flag reads |
| `AuraApplication::_HandleEffect(uint8 effIndex, bool apply)` | Dispatch al effect — llama `aura->GetEffect(effIndex)->HandleEffect(target, mode, apply)` | `AuraEffect::HandleEffect` |
| `AuraApplication::BuildUpdatePacket(AuraInfo&, bool remove)` / `ClientUpdate` | Construye SMSG_AURA_UPDATE entry | wire serialization |
| `AuraApplication::SetRemoveMode(AuraRemoveMode)` / `GetRemoveMode()` | Guarda razón de removal para uso de scripts | — |
| `AuraEffect::HandleEffect(AuraApplication*, uint8 mode, bool apply, AuraEffect const* triggeredBy)` | Despacha al `HandleXxx` correcto vía switch sobre `GetAuraType()` (~190 cases) | Uno de los ~190 HandleXxx |
| `AuraEffect::PeriodicTick(AuraApplication*, Unit* caster) const` | **Tick handler**. Switch por AuraType: PERIODIC_DAMAGE → `target->DealDamage`, PERIODIC_HEAL → `target->HealBySpell`, PERIODIC_ENERGIZE → `target->ModifyPower`, PERIODIC_TRIGGER_SPELL → `caster->CastSpell(triggerSpell, target)`, PERIODIC_LEECH → damage+heal, PERIODIC_DUMMY (script hook) | `Unit::DealDamage`, `Unit::HealBySpell`, `Unit::ModifyPower`, `Unit::CastSpell` |
| `AuraEffect::HandleProc(AuraApplication*, ProcEventInfo&)` | Dispatch para proc handlers por AuraType (PROC_TRIGGER_SPELL, PROC_TRIGGER_DAMAGE) | `caster->CastSpell` |
| `AuraEffect::ApplySpellMod(Unit*, bool apply, AuraEffect const* triggeredBy)` | Apply ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER auras al `m_spellMods` del player | `Player::AddSpellMod`/`RemoveSpellMod` |
| `AuraEffect::HandleShapeshiftBoosts(Unit* target, bool apply) const` | Aplica spells extra al cambiar shapeshift form (Bear gets Stamina, Cat gets Crit, etc.) | `target->CastSpell(boostSpellId)` |
| `AuraEffect::CalculateAmount(Unit* caster) const` | Computa `m_amount` desde basePoints + scaling + caster mods | SpellEffectInfo + `CallScriptEffectCalcAmountHandlers` |
| `AuraEffect::CalculatePeriodic(Unit* caster, bool resetPeriodicTimer, bool load)` | Configura `m_amplitude` (tick interval ms), `m_periodicTimer`, `m_isPeriodic` desde SpellInfo + mods | `CallScriptEffectCalcPeriodicHandlers` |
| `AuraEffect::Update(uint32 diff, Unit* caster)` | Resta `m_periodicTimer`; cuando llega 0 dispara `PeriodicTick` y resetea a `m_amplitude` | `PeriodicTick` |
| `AuraEffect::ChangeAmount(int32 newAmount, bool mark, bool onStackOrReapply, AuraEffect const* triggeredBy)` | Recalculate al stack/refresh; re-handles effect en mode=CHANGE_AMOUNT | `HandleEffect(mode=CHANGE_AMOUNT)` |
| `AuraEffect::HandleXxx(AuraApplication const*, uint8 mode, bool apply) const` | ~190 handlers, uno por AuraType — el grueso de SpellAuraEffects.cpp (~6300 lines) | Engine-wide: stat changes, immunities, shapeshift, transform, mount, fly, water-walk, mod resistance, mod attack speed, mod damage done/taken, school absorb, mana shield, charm, possess, fear, confuse, root, stun, silence, pacify, disarm, threat mods, taunt/detaunt, fixate, periodic damage/heal/energize, periodic trigger spell, periodic leech, dummy (scriptable), absorb, split damage, etc. |

---

## 5. Module dependencies

**Depends on:**
- `Spells/SpellInfo` — toda Aura nace de un `SpellInfo`; lee attributes, effect info, dispel type, family, scaling
- `Spells/Spell` — `Spell::EffectApplyAura` es el único punto que crea auras vía `TryRefreshStackOrCreate`
- `Entities/Unit` — `Unit::AddAura`, `_AddAura`, `_AddAuraEffect`, `_RemoveAuraEffect`, `RemoveOwnedAura`, `m_appliedAuras: ApplicationMap`, `m_ownedAuras: AuraMap`, `m_modAuras: array<AuraType, std::list<AuraEffect*>>` (para `HasAuraType`/`GetTotalAuraModifier`)
- `Entities/DynamicObject` — para DynObjAura subclass
- `Entities/Player` — `m_spellMods` (ADD_FLAT/ADD_PCT_MODIFIER), `RemoveAurasOnEvent`, persistence
- `Combat` — `DamageInfo`, `HealInfo`, `Unit::DealDamage`, `Unit::HealBySpell` (PeriodicTick payload), absorb auras intercept damage events
- `Movement` — speed auras (`MOD_INCREASE/DECREASE_SPEED`) feed `Unit::UpdateSpeed`, charm/possess affect MotionMaster, root removes movement
- `AI/UnitAI` — `OnSpellHit`, `OnAuraApply`, `OnAuraRemove` callbacks
- `Scripting/ScriptMgr` — AuraScript loader, `OnEffectApply`/`OnEffectRemove`/`OnEffectPeriodic`/`OnEffectCalcAmount`/`OnEffectCalcPeriodic`/`OnEffectAbsorb`/`OnEffectManaShield`/`OnEffectSplit`/`OnDispel`/`OnProc`
- `Database (CharacterDatabase)` — persistence: `character_aura`, `character_aura_effect`
- `Database (WorldDatabase)` — `spell_proc`, `spell_group_stack_rules`, `spell_pet_auras`, `spell_area`
- `DataStores` — DB2: SpellAuraOptions (CumulativeAura, ProcChance, ProcCharges, SpellProcsPerMinuteId), SpellAuraRestrictions, SpellInterrupts (AuraInterruptFlags, ChannelInterruptFlags)
- `Spells/SpellMgr` — `mSpellProcMap`, `CheckSpellGroupStackRules`, `GetPetAura`
- `DiminishingReturns` — `DiminishingGroup` for CC auras (fear, stun, root): subsequent applications halved/removed; reset after timeout
- `GridNotifiers` — for area auras to find targets in radius

**Depended on by:**
- `Combat` — auto-attack damage modifiers, crit chance, hit chance, dodge/parry/block all read `m_modAuras`
- `Spells/Spell` — CheckCast verifica caster auras (silence, pacify, stun); SpellEffects consume aura state (e.g., Execute requires HEALTHLESS_20)
- `Entities/Player` — talents are mostly auras applied passively at learn
- `Stats` — `MOD_STAT`, `MOD_RESISTANCE`, `MOD_INCREASE_HEALTH/ENERGY/MANA`, all stat blocks read aura totals
- `Movement` — speed auras, root, slow, snare
- `AI` — fear/charm/confuse/possess auras switch AI to scripted controllers
- `Pets` — pet auras inherited via SpellPetAura
- `Quests` — quest objective spells often apply auras (check via `target->HasAura(spellId)`)
- `Vehicles` — `MOD_VEHICLE` aura puts you on vehicle
- `OutdoorPvP` / `Battleground` — flag-carrier, stealth detection auras
- `Scripts (boss scripts)` — most encounter mechanics are aura applications

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel FROM character_aura WHERE guid = ?` | Restore auras al login | character |
| `SELECT guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount FROM character_aura_effect WHERE guid = ?` | Restore per-effect amounts | character |
| `INSERT INTO character_aura (...) VALUES (...)` | Save al logout | character |
| `INSERT INTO character_aura_effect (...) VALUES (...)` | Save effect amounts | character |
| `DELETE FROM character_aura WHERE guid = ?` | Clear before save | character |
| `SELECT * FROM pet_aura WHERE guid = ?` / `pet_aura_effect` | Pet auras persistence | character |
| `SELECT SpellId, ProcFlags, SpellTypeMask, SpellPhaseMask, HitMask, AttributesMask, ProcsPerMinute, Chance, Cooldown, Charges FROM spell_proc` | `mSpellProcMap` build at startup | world |
| `SELECT spell_id, spell_group, group_stack_rule FROM spell_group_stack_rules` | Stack rules per SpellGroup | world |
| `SELECT spell, pet, aura FROM spell_pet_auras` | Pet inherits owner auras | world |
| `SELECT spell, area, quest_start, quest_start_status, quest_end_status, quest_end, aura_spell, racemask, gender, autocast FROM spell_area` | Auras conditional on zone/quest | world |

**DB2 stores read by Auras:**

| Store | What it loads | Read by |
|---|---|---|
| `SpellAuraOptionsStore` | SpellAuraOptions.db2 (CumulativeAura, ProcChance, ProcCharges, SpellProcsPerMinuteId, DifficultyID) | Aura ctor, proc system |
| `SpellAuraRestrictionsStore` | SpellAuraRestrictions.db2 (CasterAuraState, TargetAuraState, ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell) | Aura applicability checks |
| `SpellInterruptsStore` | SpellInterrupts.db2 (AuraInterruptFlags[2], ChannelInterruptFlags[2]) | aura removal triggers |
| `SpellProcsPerMinuteStore` | SpellProcsPerMinute.db2 (BaseProcRate) | `Aura::CalcPPMProcChance` |
| `SpellProcsPerMinuteModStore` | SpellProcsPerMinuteMod.db2 (Type, Param, Coeff) | PPM mod |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_AURA_UPDATE` | server → client | `Aura::_ApplyForTarget`, `_UnapplyForTarget`, `Aura::SetNeedClientUpdateForTargets`, `AuraApplication::ClientUpdate` |
| `SMSG_AURA_UPDATE_ALL` | server → client | Login / phase-change snapshot of all auras on a unit |
| `SMSG_PERIODIC_AURA_LOG` | server → client | `AuraEffect::PeriodicTick` (DoT/HoT log per tick) |
| `SMSG_SPELL_DISPELL_LOG` | server → client | `Spell::EffectDispel` after removing aura(s) |
| `SMSG_DISPEL_FAILED` | server → client | Dispel resisted |
| `SMSG_PARTY_KILL_LOG` | server → client | (related — death by aura tick credits caster) |
| `CMSG_CANCEL_AURA` | client → server | Player cancels own buff (verifica `SPELL_ATTR0_CANT_CANCEL` y `AFLAG_NOCASTER`) → `Unit::RemoveOwnedAura` |
| `CMSG_CANCEL_GROWTH_AURA` | client → server | Legacy — drop self growth |
| `CMSG_PET_CANCEL_AURA` | client → server | Pet aura cancel |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-packet/src/packets/aura.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 líneas (vacío)**. **No existe ninguna implementación de aura, AuraEffect, AuraApplication, AuraType, AuraRemoveMode, AuraEffectHandleModes**
- `crates/wow-packet/src/packets/aura.rs` — ~123 líneas — `AuraData` POD + `AuraUpdate` writer (SMSG_AURA_UPDATE) — sólo wire shape, sin estado server-side
- `crates/wow-world/src/handlers/spell.rs` — handler `CMSG_CAST_SPELL` no genera ni guarda auras (solo envía SMSG_SPELL_START y stub `execute_spell`)
- `crates/wow-world/src/session.rs` — sin campo `auras` en `WorldSession`; sin campo `m_appliedAuras`/`m_ownedAuras` en player/creature

**What's implemented:**
- Sólo el **wire shape** del packet (`AuraData::write` → struct con slot, spellId, flags, level, charges, durations, points)
- 1 round-trip test en `aura.rs::test_aura_update_write`

**What's missing vs C++:**
1. **Cero clases**: no `Aura`, no `UnitAura`, no `DynObjAura`, no `AuraApplication`, no `AuraEffect`, no `AuraCreateInfo`, no `AuraKey`
2. **Cero enums**: no `AuraType` (~280), no `AuraRemoveMode`, no `AuraEffectHandleModes`, no `AuraFlags`, no `AuraStateType`
3. **Cero handlers**: 0 of ~190 `HandleAuraXxx` implementados (HandlePeriodicDamage, HandleAuraModStat, HandleSchoolAbsorb, etc.)
4. **Cero lifecycle**: no `Create`, no `_ApplyForTarget`, no `_Remove`, no `Update(diff)`, no `RefreshDuration`, no `SetCharges`/`ModCharges`, no `SetStackAmount`/`ModStackAmount`, no `TryRefreshStackOrCreate`, no `HandleAllEffects`, no `CanStackWith`
5. **Cero tick**: `PeriodicTick` ausente — DoT/HoT/PeriodicEnergize/PeriodicTriggerSpell jamás disparan
6. **Cero proc**: `AddProcCooldown`, `IsProcOnCooldown`, `PrepareProcToTrigger`, `TriggerProcOnEvent`, `CalcProcChance`, `CalcPPMProcChance`, `ConsumeProcCharges` — todos ausentes; el sistema entero de procs no existe
7. **Cero stacking**: no SpellGroup stack rules, no single-target tracking, no diminishing returns
8. **Cero AuraScript**: no DSL hooks (OnApply/OnRemove/OnPeriodic/OnAbsorb/OnDispel/OnProc/OnCalcAmount/OnCalcPeriodic)
9. **Cero persistencia**: sin save/load `character_aura` ni `character_aura_effect`
10. **Cero target map update**: no UPDATE_TARGET_MAP_INTERVAL para área auras
11. **Cero shapeshift boosts**: `HandleShapeshiftBoosts` (Bear → Stamina aura) ausente
12. **Cero spell mods**: ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER no aplicados al cast pipeline
13. **Cero mod aura aggregation**: no `m_modAuras: [Vec<&AuraEffect>; TOTAL_AURAS]` en Unit; `Unit::HasAuraType`, `GetTotalAuraModifier`, `GetAuraEffectsByType` no existen
14. **Cero AuraInterruptFlags**: movement/damage/cast no remueven auras (Stealth no se cae al moverse, Sap no se cae al recibir daño)
15. **Cero diminishing returns**: fear/stun/root infinitos sin reduction (broken PvP)
16. **Cero death persistence**: no se respeta `SPELL_ATTR3_DEATH_PERSISTENT`

**Suspicious / likely divergent:**
- `AuraData` se construye manualmente por algún caller (probable handlers/character) sin estar respaldado por una `Aura` viva — cualquier valor (duration, charges, stackAmount) en el packet es ficcional y no decrece en el tiempo
- Sin estado server-side, una vez enviado el packet el cliente "ve" un buff que el server desconoce — bug crítico al validar dispels, cancel, refresh, expire

**Tests existing:**
- 1 test en `crates/wow-packet/src/packets/aura.rs::test_aura_update_write` (sólo serialización)
- 0 tests en `crates/wow-spell/`

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SPELLS_AURA.WBS.001** Partir y cerrar la migracion auditada de `game/Spells/Auras/SpellAuraDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 734 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_AURA.WBS.002** Partir y cerrar la migracion auditada de `game/Spells/Auras/SpellAuraEffects.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 6342 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_AURA.WBS.003** Cerrar la migracion auditada de `game/Spells/Auras/SpellAuraEffects.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SPELLS_AURA.WBS.004** Partir y cerrar la migracion auditada de `game/Spells/Auras/SpellAuras.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2665 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SPELLS_AURA.WBS.005** Cerrar la migracion auditada de `game/Spells/Auras/SpellAuras.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.h`
  Rust target: `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerados como `#SPELLS-AURA.N` para referencia desde `MIGRATION_ROADMAP.md`. Complejidad: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#SPELLS-AURA.1** Definir `enum AuraType` con las ~280 variantes (`#[repr(u32)]`) en `crates/wow-spell/src/aura/types.rs` (M)
- [ ] **#SPELLS-AURA.2** Definir `enum AuraRemoveMode` (NONE, BY_DEFAULT, BY_INTERRUPT, BY_CANCEL, BY_ENEMY_SPELL, BY_EXPIRE, BY_DEATH) (L)
- [ ] **#SPELLS-AURA.3** Definir `bitflags AuraEffectHandleModes` (DEFAULT, REAL, SEND_FOR_CLIENT, CHANGE_AMOUNT, REAPPLY, STAT, SKILL + masks) (L)
- [ ] **#SPELLS-AURA.4** Definir `bitflags AuraFlags` (NOCASTER, POSITIVE, PASSIVE, DURATION, SCALABLE, NEGATIVE) (L)
- [ ] **#SPELLS-AURA.5** Definir `enum AuraStateType` (DEFENSE, HEALTHLESS_20, HEALTHLESS_35, BLEED, ENRAGE, FROZEN, JUDGEMENT, etc.) + bitmask helper (L)
- [ ] **#SPELLS-AURA.6** Crear `struct AuraCreateInfo`: spell_info, caster, owner, cast_difficulty, eff_mask, base_amount, cast_item_guid/id/level, is_refresh (M)
- [ ] **#SPELLS-AURA.7** Crear `struct AuraKey { caster: Guid, item: Guid, spell_id: u32, effect_mask: u32 }` para persistence + ord/eq (L)
- [ ] **#SPELLS-AURA.8** Implementar `struct AuraEffect`: spell_info, eff_index, amount, base_amount, periodic_timer_ms, amplitude_ms, tick_number, can_be_recalculated, is_periodic (M)
- [ ] **#SPELLS-AURA.9** Implementar `AuraEffect::calculate_amount(caster)` (basePoints + scaling + casterMods + script callouts) (M)
- [ ] **#SPELLS-AURA.10** Implementar `AuraEffect::calculate_periodic(caster, reset_periodic_timer)` (amplitude desde SpellInfo + mods) (M)
- [ ] **#SPELLS-AURA.11** Implementar `AuraEffect::update(diff_ms, caster)` con periodic_timer countdown + dispatch a `periodic_tick` cuando hits 0 (M)
- [ ] **#SPELLS-AURA.12** Implementar `AuraEffect::periodic_tick(application, caster)` con switch por AuraType: PERIODIC_DAMAGE → DealDamage, PERIODIC_HEAL → HealBySpell, PERIODIC_ENERGIZE → ModifyPower, PERIODIC_TRIGGER_SPELL → CastSpell, PERIODIC_LEECH (M)
- [ ] **#SPELLS-AURA.13** Implementar `AuraEffect::change_amount(new_amount, mark, on_stack_or_reapply)` para stack/refresh recalc (L)
- [ ] **#SPELLS-AURA.14** Implementar `struct AuraApplication`: target_guid, base_aura_handle, slot: u8, flags: AuraFlags, effects_to_apply: u32, effect_mask: u32, remove_mode, need_client_update (M)
- [ ] **#SPELLS-AURA.15** Implementar `AuraApplication::handle_effect(eff_index, apply)` que dispatcha al `AuraEffect::HandleEffect` (L)
- [ ] **#SPELLS-AURA.16** Implementar `AuraApplication::build_update_packet()` para SMSG_AURA_UPDATE entry (L)
- [ ] **#SPELLS-AURA.17** Implementar `struct Aura` (owner-side): spell_info, cast_id, caster_guid, owner_guid (Unit/DynObject), apply_time, max_duration, duration, time_cla (power-per-sec timer), update_target_map_interval, caster_level, proc_charges, stack_amount, applications: HashMap<Guid, AuraApplication>, is_removed, is_single_target, is_using_charges, drop_event (H)
- [ ] **#SPELLS-AURA.18** Implementar `Aura::create(create_info) -> Arc<RwLock<Aura>>` con UnitAura/DynObjAura discriminator (M)
- [ ] **#SPELLS-AURA.19** Implementar `Aura::_init_effects(eff_mask, caster, base_amount[])` (M)
- [ ] **#SPELLS-AURA.20** Implementar `Aura::try_refresh_stack_or_create(create_info, update_eff_mask) -> Arc<RwLock<Aura>>` con stacking decision (H)
- [ ] **#SPELLS-AURA.21** Implementar `Aura::can_stack_with(other: &Aura)` con SpellGroup rules + same SpellInfo + same caster + family flags (H)
- [ ] **#SPELLS-AURA.22** Implementar `Aura::_apply_for_target(target, caster, application)` con HandleAllEffects + broadcast (M)
- [ ] **#SPELLS-AURA.23** Implementar `Aura::_unapply_for_target(target, caster, application)` (M)
- [ ] **#SPELLS-AURA.24** Implementar `Aura::_remove(remove_mode)` iterando applications + script OnRemove (M)
- [ ] **#SPELLS-AURA.25** Implementar `Aura::update(diff_ms, caster)` con duration countdown + AuraEffect::update + IsExpired → Remove(BY_EXPIRE) (M)
- [ ] **#SPELLS-AURA.26** Implementar `Aura::update_target_map(caster, apply)` con `UPDATE_TARGET_MAP_INTERVAL=500ms` para área auras (H)
- [ ] **#SPELLS-AURA.27** Implementar `Aura::set_duration(duration, with_mods)` + `refresh_duration` (L)
- [ ] **#SPELLS-AURA.28** Implementar `Aura::refresh_timers(reset_periodic)` con pandemic logic (<30% restante = full reset, >=30% = sum) (M)
- [ ] **#SPELLS-AURA.29** Implementar `Aura::set_charges` / `mod_charges` / `drop_charge` / `mod_charges_delayed` / `drop_charge_delayed` (M)
- [ ] **#SPELLS-AURA.30** Implementar `Aura::set_stack_amount` / `mod_stack_amount` con RecalculateAmountOfEffects (M)
- [ ] **#SPELLS-AURA.31** Implementar `Aura::handle_all_effects(application, mode, apply)` (L)
- [ ] **#SPELLS-AURA.32** Implementar `Aura::handle_aura_specific_mods(application, caster, apply, on_reapply)` (modify schools, summon icons, spellId-specific) (H)
- [ ] **#SPELLS-AURA.33** Implementar `Aura::is_expired` / `is_permanent` / `is_passive` / `is_death_persistent` / `is_removed_on_shape_lost` / `is_area` predicates (L)
- [ ] **#SPELLS-AURA.34** Implementar single-target tracking: `is_single_target`, `register_for_single_target_caster`, `unregister_single_target` (M)
- [ ] **#SPELLS-AURA.35** Implementar `Aura::calc_dispel_chance(target, offensive)` (M)
- [ ] **#SPELLS-AURA.36** Implementar `Aura::generate_key()` + `set_loaded_state` para persistence (L)
- [ ] **#SPELLS-AURA.37** Implementar `Aura::can_be_saved()` (filtro de auras persistibles) (L)
- [ ] **#SPELLS-AURA.38** Implementar proc plumbing: `add_proc_cooldown`, `is_proc_on_cooldown`, `reset_proc_cooldown`, `prepare_proc_to_trigger`, `trigger_proc_on_event`, `prepare_proc_charge_drop`, `consume_proc_charges`, `get_proc_effect_mask` (H)
- [ ] **#SPELLS-AURA.39** Implementar `Aura::calc_proc_chance(proc_entry, event_info)` con flat + PPM (M)
- [ ] **#SPELLS-AURA.40** Implementar `Aura::calc_ppm_proc_chance(actor)` con weapon speed normalization (`base_ppm * weapon_speed / 60`) (M)
- [ ] **#SPELLS-AURA.41** Implementar dispatch `AuraEffect::handle_effect` con switch ~190 cases sobre AuraType (XL — splittable in lots) (XL)
- [ ] **#SPELLS-AURA.42** Implementar primer lote (top 30 más usados): HandlePeriodicDamage, HandlePeriodicHeal, HandlePeriodicEnergize, HandlePeriodicTriggerSpell, HandleAuraModStat, HandleAuraModResistance, HandleAuraModSpeed (Inc/Dec), HandleAuraModRoot, HandleAuraModStun, HandleAuraModSilence, HandleAuraModFear, HandleAuraModConfuse, HandleAuraModPacify, HandleAuraModDisarm, HandleSchoolAbsorb, HandleManaShield, HandleAuraModIncreaseHealth, HandleAuraModIncreaseEnergy, HandleAuraModDamageDone/Taken, HandleAuraModCritPercent, HandleAuraModParryPercent, HandleAuraModDodgePercent, HandleAuraModBlockPercent (H)
- [ ] **#SPELLS-AURA.43** Implementar segundo lote: shapeshift/transform/mount/fly handlers (HandleAuraModShapeshift + HandleShapeshiftBoosts, HandleAuraTransform, HandleAuraMounted, HandleAuraAllowFlight, HandleAuraWaterWalk, HandleAuraFeatherFall, HandleAuraHover, HandleWaterBreathing, HandleForceMoveForward) (H)
- [ ] **#SPELLS-AURA.44** Implementar tercer lote: stealth/invisibility/phase (HandleModStealth, HandleModStealthDetect, HandleModStealthLevel, HandleModInvisibility, HandleModInvisibilityDetect, HandlePhase, HandlePhaseGroup, HandlePhaseAlwaysVisible, HandleAuraGhost, HandleSpiritOfRedemption, HandleFeignDeath) (H)
- [ ] **#SPELLS-AURA.45** Implementar cuarto lote: charm/threat/taunt (HandleModConfuse, HandleModFear, HandleModCharm, HandleAuraModPossess, HandleAuraModPossessPet, HandleModThreat, HandleAuraModTotalThreat, HandleModTaunt, HandleModDetaunt, HandleAuraModFixate, HandleModUnattackable) (H)
- [ ] **#SPELLS-AURA.46** Implementar quinto lote: track/skill (HandleAuraTrackCreatures, HandleAuraTrackStealthed, HandleAuraModStalked, HandleAuraUntrackable, HandleAuraModSkill, HandleDetectAmore) (M)
- [ ] **#SPELLS-AURA.47** Implementar sexto lote: proc handlers (HandleProcTriggerSpell, HandleProcTriggerDamage, HandleAuraDummy con proc dispatch, HandleAuraOverrideClassScripts) (H)
- [ ] **#SPELLS-AURA.48** Implementar resto (~120) en lotes prioridad por uso: pacify/silence variants, no-actions, scale/clone, allow-blocking, aura-specific (~XL — splittable per archetype) (XL)
- [ ] **#SPELLS-AURA.49** Implementar `AuraEffect::apply_spell_mod(target, apply, triggered_by)` para SPELL_AURA_ADD_FLAT_MODIFIER y ADD_PCT_MODIFIER (M)
- [ ] **#SPELLS-AURA.50** Implementar `AuraEffect::handle_proc(application, event_info)` proc dispatch (H)
- [ ] **#SPELLS-AURA.51** Implementar `m_modAuras: [Vec<AuraEffectHandle>; TOTAL_AURAS]` en `Unit` para `HasAuraType`, `GetTotalAuraModifier`, `GetTotalAuraModifierByMiscMask`, `GetAuraEffectsByType` (H)
- [ ] **#SPELLS-AURA.52** Implementar `Unit::AddAura(spell_id, target)` overloads (entry-point usado por scripts/AI) (M)
- [ ] **#SPELLS-AURA.53** Implementar `Unit::RemoveAurasDueToSpell(spell_id, casterGuid?, reqEffMask, removeMode)` y `RemoveOwnedAura` (M)
- [ ] **#SPELLS-AURA.54** Implementar AuraInterruptFlags handler — al hacer event (movement/damage/cast/mount/stealth-broken) iterar auras y aplicar Remove(BY_INTERRUPT) si flag matches (H)
- [ ] **#SPELLS-AURA.55** Implementar Diminishing Returns: `DiminishingGroup` enum, `Unit::AddDiminishing`, level decay (1.0 → 0.5 → 0.25 → immune), reset timer 18s post-last-application (H)
- [ ] **#SPELLS-AURA.56** Implementar persistence save: serialize all `CanBeSaved()` auras al `Player::SaveToDB` (sql `character_aura` + `character_aura_effect`) (M)
- [ ] **#SPELLS-AURA.57** Implementar persistence load: `Player::_LoadAuras` desde `character_aura` + `character_aura_effect` con `SetLoadedState` (M)
- [ ] **#SPELLS-AURA.58** Implementar AuraScript trait: `on_effect_apply`, `on_effect_remove`, `on_effect_periodic`, `on_effect_calc_amount`, `on_effect_calc_periodic`, `on_effect_absorb`, `on_dispel`, `on_proc`, `check_proc`, `prepare_proc`, `on_effect_proc`, etc. (XL — DSL design) (XL)
- [ ] **#SPELLS-AURA.59** Implementar `Aura::call_script_*` dispatchers a `m_loadedScripts` (M)
- [ ] **#SPELLS-AURA.60** Implementar `DynObjAura` subclass + `FillTargetMap` para área auras (Consecration, Blizzard) (H)
- [ ] **#SPELLS-AURA.61** Implementar `UnitAura::add_static_application` para non-area auras (L)
- [ ] **#SPELLS-AURA.62** Implementar `CMSG_CANCEL_AURA` real (verifica `SPELL_ATTR0_CANT_CANCEL`, `AFLAG_NOCASTER`, RemoveOwnedAura(BY_CANCEL)) (M)
- [ ] **#SPELLS-AURA.63** Implementar `SMSG_PERIODIC_AURA_LOG` writer + emisión por `PeriodicTick` (L)
- [ ] **#SPELLS-AURA.64** Implementar `SMSG_AURA_UPDATE_ALL` snapshot al login / phase change (M)
- [ ] **#SPELLS-AURA.65** Implementar pet aura inheritance: `SpellPetAura` table + auto-apply al pet summon (M)
- [ ] **#SPELLS-AURA.66** Implementar SpellArea conditional auras (zone/area/quest/aura-required) — periodic check via player tick (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SPELLS_AURA.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 10549 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h`. Rust target: `crates/wow-spell`. | `cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SPELLS_AURA.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 10549 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h`. Rust target: `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SPELLS_AURA.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 10549 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h`. Rust target: `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SPELLS_AURA.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 10549 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h`. Rust target: `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Aura::create` con Renew (spell 139) en target → `target.has_aura(139) == true`, `target.get_aura(139).get_duration() == 15000`
- [ ] Test: `Aura::update(diff=1000)` 15 veces → expira (`is_expired == true`), llama `Remove(BY_EXPIRE)`
- [ ] Test: `AuraEffect::periodic_tick` para Renew tick cada 3000ms — heal recibido == amount + bonus, total 5 ticks
- [ ] Test: `Aura::try_refresh_stack_or_create` con aura ya existente, mismo caster → refresh, no crea segunda; `stack_amount == 1`
- [ ] Test: `Aura::try_refresh_stack_or_create` con cumulative spell (Curse of Agony), 2nd cast → `stack_amount == 2`, `RecalculateAmountOfEffects` corre
- [ ] Test: `Aura::can_stack_with` con dos buffs del mismo SpellGroup STACK_RULE_EXCLUSIVE → false (no stack)
- [ ] Test: `Aura::can_stack_with` Hunter's Mark de caster_A vs caster_B → false (single target debuff overrides)
- [ ] Test: `Aura::set_charges(0)` → Remove called automatically con BY_DEFAULT
- [ ] Test: `Aura::mod_stack_amount(-1)` con stack=1 → Remove
- [ ] Test: `Aura::refresh_timers(reset_periodic=false)` durante último 30% (pandemic) → duration += full max, sin reset tick timer
- [ ] Test: AuraInterruptFlags MOVEMENT — Stealth aura, player move → `Remove(BY_INTERRUPT)` aplicado
- [ ] Test: AuraInterruptFlags TAKE_DAMAGE — Sap, target takes damage → `Remove(BY_INTERRUPT)`
- [ ] Test: Diminishing Returns — Fear cast 4 veces seguidas en mismo target: 1st full duration, 2nd 50%, 3rd 25%, 4th immune
- [ ] Test: DR reset — esperar 18s sin reapply → contador resetea
- [ ] Test: HandleAuraModSpeed (Frost Nova root) → `Unit::is_rooted == true`, speed = 0
- [ ] Test: HandleAuraModSilence — target.has_aura_type(SILENCE) → `Spell::CheckCast` falla con SPELL_FAILED_SILENCED
- [ ] Test: HandleSchoolAbsorb (Power Word: Shield) — target recibe 1000 fire damage con shield 500 → 500 absorbed, shield removed (charges=0 = expire) o reduced
- [ ] Test: HandleAuraModStat (Mark of the Wild) — `target.get_stat(STAT_STRENGTH) += amount`, al unapply → resta exacto
- [ ] Test: HandleAuraModShapeshift (Druid Bear Form) — dispara `HandleShapeshiftBoosts` que aplica auras secundarias (Stamina aura, etc.)
- [ ] Test: PROC_TRIGGER_SPELL con Chance=20% → roll RNG seed fijo, contar successes match expected
- [ ] Test: PPM proc — weapon speed 2.0s, base PPM=4.0 → chance = 4 * 2.0 / 60 = 13.33%
- [ ] Test: Proc ICD — aura con InternalCooldown=10000ms, 2 eventos en 1s → proc dispara 1 vez sólo
- [ ] Test: Persistence save/load — Renew con remaining 8s logout → al login Renew con duration ≈ 8s (con timestamp save)
- [ ] Test: Death persistence — `SPELL_ATTR3_DEATH_PERSISTENT` aura sobrevive a player death; resto se quita con `BY_DEATH`
- [ ] Test: Single-target debuff (Hunter Mark) — caster aplica a A, luego a B → A pierde debuff (single-target tracking via caster m_scAuras)
- [ ] Test: DynObjAura (Consecration) — DynamicObject creado, área aura tickea damage cada N ms a enemigos en radio, expira con duración
- [ ] Test: AuraScript OnEffectApply hook — script registered para spell X, aplicar X → callback dispara con (aura_eff, application, mode)
- [ ] Test: SMSG_AURA_UPDATE — al apply broadcast con slot, spellId, flags, durations correctos
- [ ] Test: SMSG_PERIODIC_AURA_LOG — DoT tick → packet con damage, school, abs/resist match

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 5 files / 10549 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h` | `crates/wow-spell/` (módulo `aura`), `crates/wow-packet/src/packets/aura.rs` \| ❌ not started — `wow-spell` está en 0 líneas; sólo existe `AuraData` POD para wire en `wow-packet/src/packets/aura.rs` sin estado server-side |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SPELLS_AURA.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 5 C++ files / 10549 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SPELLS_AURA.DIV.002` | `crates/wow-spell/src/lib.rs` (`exists_empty`, 0 Rust lines) | 5 C++ files / 10549 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraEffects.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuras.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/SpellAuraDefines.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **Aura vs AuraApplication vs AuraEffect — los 3 niveles son distintos.** `Aura` es la instancia global (1 per cast). `AuraApplication` es per-target (N per Aura si afecta party). `AuraEffect` es per-effect-index (Vec<AuraEffect> dentro de Aura, hasta MAX_SPELL_EFFECTS). Confundir cualquiera = bugs muy serios. Borrar la Aura requiere _Remove de TODAS las AuraApplications + cleanup de los AuraEffect.
- **`m_modAuras` vs `m_appliedAuras`.** En Unit, `m_appliedAuras: ApplicationMap` lista por target todas las auras aplicadas. `m_modAuras: array<list<AuraEffect*>, TOTAL_AURAS>` indexa por AuraType — esto es lo que `HasAuraType(SPELL_AURA_MOD_ROOT)` consulta (O(1) lookup vs O(N) iter). Al apply/remove hay que actualizar AMBOS — bug clásico: olvidar el modAuras update y `IsRooted()` queda mintiendo.
- **Stacking rules son no-triviales.** `CanStackWith`: same SpellInfo + same caster → cumulative (stack++); same SpellInfo + different caster → check `SPELL_ATTR3_STACK_FOR_DIFF_CASTERS`; diferente SpellInfo en mismo SpellGroup → consulta `spell_group_stack_rules.group_stack_rule` (DEFAULT, EXCLUSIVE, EXCLUSIVE_SAME_EFFECT, EXCLUSIVE_SAME_CALLER, EXCLUSIVE_HIGHEST). Si rule=EXCLUSIVE, la nueva remueve la vieja.
- **Pandemic / refresh timer logic.** Al refresh durante último 30% de duration, `RefreshTimers` hace `m_duration = m_maxDuration + remaining` (clampado a 1.3x max). Esto evita lost-rolling. Olvidar = DoT/HoT players nunca optimizan refresh window. Implementación en `Aura::RefreshTimers`.
- **DR (Diminishing Returns).** PvP-critical. `DiminishingGroup` mapea AuraType a categorías (FEAR, STUN, ROOT, DISORIENT, …). Después de 1st application, 2nd dura 50%, 3rd 25%, 4th immune. Reset interno = 18s sin nueva application. Está en `DiminishingReturns.cpp`/`SpellAuras.cpp:Aura::CanApplyResilience`. RustyCore lo va a necesitar para PvP correcto.
- **Proc system es la fuente de más bugs históricos.** `ProcFlags` (~30 bits) define el evento (DONE_MELEE_AUTO_ATTACK, TAKEN_PERIODIC, etc.). Cada Aura activa con `SpellProcEntry` matching dispara su `triggerSpell`. El roll usa Chance OR PpmRate (PPM normalizado por weapon speed). ICD per-aura via `m_procCooldown`. **Charges** drop después del proc, no antes — si lo haces antes y el script previene, perdiste una charge. Ver `Aura::PrepareProcChargeDrop`.
- **PPM normalization formula.** `chance = base_ppm * weapon_speed_seconds / 60.0`. Sin esto, slow weapons proc al doble. Hay también `SpellProcsPerMinuteMod` con coeficientes per spec/race/etc. Bug histórico en TC: PPM con 2H weapons antes de la fix counted como 1H → prock al medio.
- **`AuraType` sigue creciendo.** El enum llega a 544 en TC actual pero muchos huecos (526, 527, 530, 531, 532, 534, 537, 542, 543, 544 = sin nombre, NYI). En 3.4.3 (WotLK) están definidos hasta ~316; los superiores son retail-only. **Al portar, filtra AuraType > 316 como noop o `_RetailOnly`.**
- **`SPELL_AURA_DUMMY` (4) es el wildcard.** Aura sin handler genérico → todo pasa por scripts (`AuraScript::OnEffectApply/Remove`). Si no hay script registrado, la dummy aura no hace nada — pero el cliente la verá. Bug clásico: implementar la dummy en code en lugar de en script registry → fragmenta lógica.
- **`SPELL_AURA_PERIODIC_DUMMY` (226) similar.** Tick handler invoca script callback por spellId — sin script, el tick no hace nada pero igual gasta el timer. Útil para boss mechanics que no caben en handlers genéricos.
- **`SPELL_AURA_PROC_TRIGGER_SPELL` (42) vs `SPELL_AURA_DUMMY` (4) procs.** PROC_TRIGGER_SPELL tiene `triggerSpell` en SpellEffectInfo y dispara automáticamente. DUMMY procs requieren AuraScript o C++ proc handler en SpellMgr's old `mSpellProcEvents` system. WotLK tiene ambos sistemas convivientes — el migration debería preferir el moderno (spell_proc table + SpellProcEntry).
- **Death persistence.** `SPELL_ATTR3_DEATH_PERSISTENT` auras sobreviven al death (Spirit of Redemption, Reincarnation cooldowns, Resurrection sickness). El resto se quita con `Remove(BY_DEATH)`. Olvidar iterar = ghost auras. Ver `Player::ResurrectPlayer`, `Unit::setDeathState`.
- **AuraInterruptFlags vs ChannelInterruptFlags vs InterruptFlags.** Tres conceptos:
  - `InterruptFlags` (en SpellInterrupts.db2) = mientras casteando, qué eventos cancelan el cast.
  - `ChannelInterruptFlags` = mientras channeling, qué eventos cancelan el channel.
  - `AuraInterruptFlags` = mientras la aura está activa, qué eventos la remueven (Stealth → movement break, Sap → take damage, Polymorph → take damage above threshold).
- **`CalcMaxDuration` vs `GetDuration` vs `GetMaxDuration`.** `CalcMaxDuration` consulta DB2 + caster mods + power costs (algunas auras tienen duración dependiente de power gastado). `GetMaxDuration` lee `m_maxDuration` cached. `GetDuration` lee `m_duration` (current remaining). Refresh resetea `m_duration = m_maxDuration`.
- **Time_cla (timer for power per sec).** Algunas auras consumen mana por segundo (Hymn of Hope, channeled spells). `m_timeCla` countdown a 0 → consume `m_periodicCosts` y reset.
- **AuraApplication slot management.** El cliente UI tiene 64 slots (originalmente, ahora más); el server asigna slot al apply. Si overflow → la aura existe sin UI visible. Bug visual común.
- **AuraScript mutation timing.** `OnEffectApply(REAL)` corre antes de que el effect aplique cambios (puede prevent vía `PreventDefaultAction`). `AfterEffectApply` corre después. Confundir = double-apply o no-apply.
- **Performance: PeriodicTick es hot path.** Cada DoT/HoT en cada target en cada tick. C++ usa lista linked + bulk skip de auras no-periodic. Rust debería separar `Vec<&AuraEffect>` solo de los periódicos en el Unit para evitar full scan.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Aura` (abstract) | `struct Aura` en `crates/wow-spell/src/aura/aura.rs` con `enum AuraOwnerKind { Unit, DynObj }` campo en lugar de subclase | Sin herencia; el `FillTargetMap` se vuelve match sobre el kind |
| `class UnitAura : Aura` | mismo `struct Aura` + `kind = AuraOwnerKind::Unit { dr_group: Option<DiminishingGroup>, static_applications: HashMap<Guid, u32> }` | — |
| `class DynObjAura : Aura` | mismo `struct Aura` + `kind = AuraOwnerKind::DynObj { dyn_obj_guid: Guid }` | — |
| `class AuraApplication` | `struct AuraApplication` en `crates/wow-spell/src/aura/application.rs` | Pegado al Unit (`HashMap<u32 spell_id, AuraApplication>` o `Vec` indexed by slot) |
| `class AuraEffect` | `struct AuraEffect` en `crates/wow-spell/src/aura/effect.rs` | Owns su propio `m_amount`/`m_periodicTimer`; vive dentro del `Aura.effects: Vec<AuraEffect>` |
| `enum AuraType` | `#[repr(u32)] enum AuraType { … }` con ~280 variantes; `#[non_exhaustive]` | Filter > 316 as `RetailOnly_NNN` para 3.4.3 build |
| `enum AuraRemoveMode` | `enum AuraRemoveMode { Default, Interrupt, Cancel, EnemySpell, Expire, Death }` | — |
| `enum AuraEffectHandleModes` | `bitflags! { struct AuraEffectHandleModes: u8 { const REAL=0x01; const SEND_FOR_CLIENT=0x02; const CHANGE_AMOUNT=0x04; const REAPPLY=0x08; const STAT=0x10; const SKILL=0x20; } }` | masks como `const REAL_OR_REAPPLY = REAL | REAPPLY` |
| `enum AuraFlags` (AFLAG_*) | `bitflags! AuraFlags { const NOCASTER=0x01; const POSITIVE=0x100; const PASSIVE=0x200; … }` | wire-compatible |
| `enum AuraStateType` | `enum AuraStateType` `#[repr(u8)]` | usado para mask `(1 << (state - 1))` |
| `Aura* TryRefreshStackOrCreate(AuraCreateInfo&)` | `fn try_refresh_stack_or_create(info: AuraCreateInfo, registry: &mut AuraRegistry) -> Arc<RwLock<Aura>>` | `AuraRegistry` per-Unit es el holder principal |
| `void Aura::Update(uint32, Unit*)` | `fn update(&mut self, diff_ms: u32, caster: Option<&Unit>) -> AuraUpdateOutcome` | Outcome = Continue \| Expired \| Removed |
| `void Aura::_Remove(AuraRemoveMode)` | `fn remove(&mut self, mode: AuraRemoveMode)` consume `self` para forzar drop ordering | el target unit reactiva m_modAuras update |
| `Aura::HandleAllEffects(AuraApplication*, mode, apply)` | `fn handle_all_effects(&self, app: &mut AuraApplication, mode: AuraEffectHandleModes, apply: bool, target: &mut Unit, caster: Option<&Unit>)` | — |
| `AuraEffect::PeriodicTick(...)` | `fn periodic_tick(&mut self, app: &AuraApplication, target: &mut Unit, caster: Option<&Unit>)` | `match self.aura_type` switch |
| `AuraEffect::HandleEffect(target, mode, apply, triggeredBy)` | `fn handle_effect(&self, target: &mut Unit, mode: AuraEffectHandleModes, apply: bool, triggered_by: Option<&AuraEffect>)` | giant match sobre AuraType |
| `Aura::TriggerProcOnEvent` | `fn trigger_proc_on_event(&mut self, eff_mask: u32, app: &mut AuraApplication, event: &mut ProcEventInfo)` | dispara `caster.cast_spell(trigger_spell)` |
| `Aura::CalcPPMProcChance(actor)` | `fn calc_ppm_proc_chance(&self, actor: &Unit) -> f32` | `base_ppm * weapon_speed_secs / 60.0` |
| `class AuraScript` (DSL) | `trait AuraScript` en `crates/wow-spell/src/aura/script.rs` con default no-op methods + `inventory::submit!` registry | un script por archivo en `crates/wow-scripts-spell/` |
| `Aura::CallScript*Handlers` | métodos en `Aura` que iteran `self.loaded_scripts: Vec<Box<dyn AuraScript>>` | preserva ordering register |
| `m_modAuras: array<list<AuraEffect*>, TOTAL_AURAS>` | `mod_auras: HashMap<AuraType, SmallVec<[AuraEffectHandle; 4]>>` en Unit | O(1) lookup; `AuraEffectHandle = Weak<RwLock<AuraEffect>>` o slotmap key |
| `Unit::HasAuraType(AuraType)` | `fn has_aura_type(&self, ty: AuraType) -> bool` | `mod_auras.get(&ty).map_or(false, \|v\| !v.is_empty())` |
| `Unit::GetTotalAuraModifier(AuraType)` | `fn get_total_aura_modifier(&self, ty: AuraType) -> i32` | sum por type |
| `Unit::AddAura(spellId, target)` | `fn add_aura(&mut self, spell_id: u32, target: &mut Unit) -> Option<Arc<RwLock<Aura>>>` | conveniencia (looks up SpellInfo + create) |
| `WorldDatabase.Query("SELECT … FROM spell_proc")` | `sqlx::query_as!(SpellProcRow, …).fetch_all(&pool).await` | Async load en startup, populates `SpellMgr.spell_procs: DashMap<u32, SpellProcEntry>` |
| `CharacterDatabase load character_aura` | async load en `Player::load_auras` durante login | persistence flow detallado en handlers/auth |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Spells/Auras/` (`SpellAuraDefines.h` 734 lines, `SpellAuras.h` 398 lines, `SpellAuras.cpp` 2,665 lines, `SpellAuraEffects.h` 410 lines, `SpellAuraEffects.cpp` 6,342 lines — total ~9,849 lines including comments) against the Rust workspace at `/home/server/rustycore/crates/`.

**Empty-crate finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` measures **exactly 0 lines** (verified via `wc -l`). Within the aura sub-module specifically, the Rust workspace has **zero implementation** of: `Aura` (struct/class), `UnitAura`, `DynObjAura`, `AuraApplication`, `AuraEffect`, `AuraCreateInfo`, `AuraKey`, `AuraLoadEffectInfo`, `AuraType` enum (~280 variants), `AuraRemoveMode` enum, `AuraEffectHandleModes` bitflags, `AuraFlags` bitflags, `AuraStateType` enum, `DAMAGE_ABSORB_TYPE`, `AuraTriggerOnPowerChangeDirection`, `AuraTriggerOnHealthChangeDirection`. The 9,849 lines of C++ aura engine map to **zero lines** of Rust engine.

**What exists outside the empty crate.** A single file: `crates/wow-packet/src/packets/aura.rs` (~123 lines) defines `AuraData` (POD with slot, spellId, flags, level, charges, durations, points) and an `AuraUpdate` writer that serializes the SMSG_AURA_UPDATE wire shape. There is one round-trip test (`test_aura_update_write`). Crucially, the writer is **fed manually by callers** with arbitrary values — no `Aura::Update` tick produces these values, no `_ApplyForTarget`/`_Remove` decides when to send. Nothing in `wow-world` or `wow-spell` constructs or schedules an aura. Sending the packet to a client makes the UI show a buff that the server has zero awareness of.

**AuraEffect handlers implemented.** **0 of ~190.** The C++ file `SpellAuraEffects.cpp` defines 190 `void AuraEffect::HandleXxx(...)` functions (verified via `grep -c "^void AuraEffect::Handle" = 190`), one per `SPELL_AURA_*` type that has real semantics. None exists in Rust: no `HandlePeriodicDamage`, no `HandlePeriodicHeal`, no `HandlePeriodicEnergize`, no `HandleAuraModStat`, no `HandleAuraModResistance`, no `HandleAuraModSpeed`, no `HandleAuraModRoot`, no `HandleAuraModStun`, no `HandleAuraModSilence`, no `HandleAuraModFear`, no `HandleAuraModConfuse`, no `HandleAuraModPacify`, no `HandleAuraModDisarm`, no `HandleSchoolAbsorb`, no `HandleManaShield`, no `HandleAuraModShapeshift`, no `HandleShapeshiftBoosts`, no `HandleAuraTransform`, no `HandleAuraMounted`, no `HandleAuraAllowFlight`, no `HandleAuraWaterWalk`, no `HandleFeignDeath`, no `HandleAuraGhost`, no `HandleSpiritOfRedemption`, no `HandlePhase`, no `HandleModInvisibility`/`Detect`, no `HandleModStealth`/`Detect`, no `HandleAuraTrack*`, no `HandleProcTriggerSpell`, no `HandlePeriodicTriggerSpell`, no `HandleAuraDummy`, no `HandleCharm`, no `HandleAuraModPossess`, no `HandleModThreat`, no `HandleModTaunt`, no `HandleModDetaunt`, no `HandleAuraModFixate`. The dispatch switch (`switch (GetAuraType())`) over 190 cases has no analog at all.

**Aura lifecycle implemented.** **None.** No `Aura::Create`, no `TryRefreshStackOrCreate`, no `_InitEffects`, no `_ApplyForTarget`, no `_UnapplyForTarget`, no `_Remove`, no `Update(diff, caster)`, no `UpdateTargetMap`, no `RefreshDuration`, no `RefreshTimers` (no pandemic logic), no `SetCharges`/`ModCharges`/`DropCharge`/`ModChargesDelayed`/`DropChargeDelayed`, no `SetStackAmount`/`ModStackAmount`, no `HandleAllEffects`, no `HandleAuraSpecificMods`, no `CanStackWith`, no `CanBeAppliedOn`, no `CalcDispelChance`, no `IsExpired`/`IsPermanent`/`IsPassive`/`IsDeathPersistent`/`IsRemovedOnShapeLost`/`IsArea`. The aura cannot be born, cannot tick, cannot stack, cannot refresh, cannot expire, cannot be dispelled — because it does not exist as a server-side object.

**Periodic tick.** `AuraEffect::PeriodicTick` and `AuraEffect::Update(diff, caster)` are entirely absent. There is no DoT damage, no HoT heal, no PERIODIC_ENERGIZE, no PERIODIC_TRIGGER_SPELL, no PERIODIC_LEECH, no PERIODIC_DUMMY callback. `SMSG_PERIODIC_AURA_LOG` is never emitted because no tick fires.

**Stacking rules.** Zero. No `SpellGroup` loader, no `spell_group_stack_rules` SQL load, no `CanStackWith` decision logic. Two casts of the same DoT will produce duplicate clientside "auras" without the server tracking either, with no exclusivity / refresh / cumulative behavior.

**Dispel.** Zero. `Aura::CalcDispelChance` is missing, `DispelType` enum is missing, `DispelMask` helpers are missing, `SPELL_AURA_DISPEL_IMMUNITY` is missing, `Spell::EffectDispel` is missing (cross-ref `spells-effects.md`). Magic dispel, curse dispel, disease cleanse, poison cleanse — none functional.

**Charge consumption.** Zero. No `m_procCharges`, no `ConsumeProcCharges`, no `DropCharge`, no `PrepareProcChargeDrop`, no proc-cooldown-with-charges semantics. Spells like Lightning Shield (3 charges, drops one per melee hit taken) cannot decrement.

**Proc system.** Runtime is still effectively absent. `SpellProcEntry`/`spell_proc` explicit SQL rows are now represented in `wow-data` (`SpellProcStoreLikeCpp`), but there is no live `mSpellProcMap` startup owner, no implicit aura-generated proc entries, no `Aura::AddProcCooldown`, no `IsProcOnCooldown`, no `PrepareProcToTrigger`, no `TriggerProcOnEvent`, no `CalcProcChance`, no `CalcPPMProcChance`, no `ConsumeProcCharges`, no `GetProcEffectMask`, no PPM-with-weapon-speed-normalization formula, and no ICD per-aura. The proc framework central to gear effects, talents and set bonuses is not active.

**AuraScript / DSL.** Zero. No `AuraScript` trait, no `OnEffectApply`/`OnEffectRemove`/`OnEffectPeriodic`/`OnEffectCalcAmount`/`OnEffectCalcPeriodic`/`OnEffectAbsorb`/`OnEffectManaShield`/`OnEffectSplit`/`OnDispel`/`OnProc`/`CheckProc`/`PrepareProc` hooks, no script registry. Boss mechanics that rely on scripted aura behavior (Lich King's Necrotic Plague, every encounter mechanic encoded as aura) cannot be migrated until the DSL exists.

**Persistence.** Zero. No `character_aura` save/load, no `character_aura_effect` save/load, no `Aura::CanBeSaved` filter, no `GenerateKey`, no `SetLoadedState`. All buffs vanish at logout because there is nothing to save.

**`m_modAuras` aggregation.** Zero. `Unit::HasAuraType`, `GetTotalAuraModifier`, `GetAuraEffectsByType`, `GetTotalAuraModifierByMiscMask` — none exist. Without this, every stat calculation, every immune check, every "is the caster silenced before casting" lookup is broken or simulated as always-false. This is upstream of every other system that consumes aura state.

**Unit-side integration.** Zero. No `m_appliedAuras` ApplicationMap, no `m_ownedAuras`, no `_AddAura`/`_AddAuraEffect`/`_RemoveAuraEffect`, no `RemoveAurasDueToSpell`, no `RemoveOwnedAura`. The `WorldSession`/Unit struct has no field for auras at all.

**Worst divergence.** The aura subsystem is **purely a wire-format echo**: serialize fictional values, push them at the client, the server forgets. Combined with the absence of `Aura::Update`, no buff ever expires server-side, no DoT ever ticks, no proc ever fires, no shapeshift boost cascades, no diminishing returns degrade CC, no AuraInterruptFlag breaks Stealth on movement. The §9 task list (#SPELLS-AURA.1 → #SPELLS-AURA.66) reflects ground-up greenfield work — equivalent in scope to porting all of `SpellAuras.cpp` + `SpellAuraEffects.cpp` (~9k C++ lines) to idiomatic Rust with proper ownership, plus designing the AuraScript DSL trait.
