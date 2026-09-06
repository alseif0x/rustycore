# Migration: Spells / Cast (sub-module)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Spells/Spell.{h,cpp}` (~10,297 lines combined: 994 + 9,303) + `SpellCastRequest.h` (43)
> **Rust target crate(s):** `crates/wow-spell/` (módulo `cast`), `crates/wow-world/src/handlers/spell.rs` (entry point), `crates/wow-packet/src/packets/spell.rs` (wire)
> **Layer:** L5 sub-module of `spells.md` (Game systems — combat / spells / casting pipeline)
> **Status:** 🔧 broken (rewrite needed) — `wow-spell` está en 0 líneas; el cast handler existente en `wow-world` (288 líneas) implementa solo el ack mínimo (`SMSG_SPELL_START` con visual hardcoded + `SMSG_CAST_FAILED`), sin pipeline, sin `Spell` struct, sin target enumeration, sin `CheckCast`/`CheckPower`/`CheckRange`/`CheckLOS`, sin canal, sin triggered.
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing — see §13)
> **Last updated:** 2026-05-01

> **Parent doc:** [`spells.md`](./spells.md) — overview del motor entero (Spell + SpellInfo + SpellMgr + SpellHistory + Auras combinados, ~44k líneas C++).
> **Related sub-docs:** [`spells-effects.md`](./spells-effects.md) (los ~151 `EffectXxx` handlers que `cast` invoca vía `HandleEffects`), [`spells-aura.md`](./spells-aura.md) (lo que `EffectApplyAura` produce), [`spells-info.md`](./spells-info.md) (datos estáticos consumidos por `Spell`), [`spells-mgr.md`](./spells-mgr.md) (`SpellMgr` registry).
> **Cross-link:** todo cast nace en un `Unit` (`Unit::CastSpell` → `new Spell(caster, …)`) y resuelve sobre otros `Unit`/`Item`/`GameObject` — ver [`entities-unit.md`](./entities-unit.md) para `m_currentSpells[CURRENT_*]`, `Unit::HasUnitState(UNIT_STATE_CASTING)`, `Unit::InterruptSpell`, `Unit::SetCurrentCastSpell`, `Unit::FinishSpell`, `Unit::ProcSkillsAndAuras`.

---

## 1. Purpose

El sub-módulo Cast es el **runtime de un cast en curso**: la clase `Spell` instancia que vive desde `Unit::CastSpell` (CMSG_CAST_SPELL llega) hasta `finish` (cleanup tras resolver), orquestando el pipeline completo `prepare → cast → handle_immediate / handle_delayed → _handle_finish_phase → finish`. Es responsable de **inicializar y validar** el cast (`InitExplicitTargets`, `CheckCast`, `CheckPower`, `CheckItems`, `CheckRange`, `CheckRuneCost`, `CheckCasterAuras`, `CheckMovement`), de **enumerar targets implícitos y explícitos** (~13 selectores `SelectImplicit*Targets` que cubren area/cone/chain/trajectory/line/destination/channel), de **calcular cast time, GCD y power cost** con todos los modifiers, de **temporizar** el cast (cast time bar, channel ticks, delayed projectile travel), de **broadcastear los packets de visualización** (`SMSG_SPELL_START`, `SMSG_SPELL_GO`, `SMSG_CAST_FAILED`, `SMSG_SPELL_FAILURE`, `SMSG_CHANNEL_START`, `SMSG_CHANNEL_UPDATE`, `SMSG_SPELL_INTERRUPT_LOG`), y de **invocar la dispatch table de effects** (`HandleEffects` → uno de los ~151 `EffectXxx` por effect index documentados en [`spells-effects.md`](./spells-effects.md)).

Es el **orquestador**: por sí solo no escribe daño/heal/aura — delega cada effect en `SpellEffects.cpp`. Pero todo lo que rodea esa delegación (target lists, miss/resist/parry/block roll, reflect, absorb, distance/projectile delay, interrupt flags, channel tick scheduling, trigger spell chain con `m_procChainLength` cap, AuraScript / SpellScript hook dispatch, persistence de cooldown via `SpellHistory`, pet cast routing, vehicle/charm cast routing) vive aquí. ~9.3k líneas C++ (`Spell.cpp`) — el archivo más grande de Spells después de `SpellAuraEffects.cpp`.

---

## 2. C++ canonical files

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Spells/Spell.h` | 994 | `class Spell` (caster, targets, fase, timer, daño calculado, miss-info por target, channel state, scripts cargados); `class SpellEvent : public BasicEvent` (tick que avanza cast time / channel); `struct SpellValue` (override per-cast de BasePoints/RadiusMod/Duration/CriticalChance); `struct SpellLogEffect*Params` (PowerDrain, ExtraAttacks, DurabilityDamage, GenericVictim, TradeSkillItem, FeedPet); `enum SpellState` (NULL/PREPARING/CASTING/FINISHED/IDLE/DELAYED); `enum SpellEffectHandleMode` (LAUNCH/LAUNCH_TARGET/HIT/HIT_TARGET); `enum SpellCastFlags` (~32 flags); `enum SpellCastFlagsEx`; `enum SpellCastSource` (PLAYER/NORMAL/ITEM/PASSIVE/PET/AURA/SPELL); `enum SpellRangeFlag` (DEFAULT/MELEE/RANGED); `enum SpellHealPredictionType`; constantes (`SPELL_CHANNEL_UPDATE_INTERVAL = 1000ms`, `MAX_SPELL_RANGE_TOLERANCE = 3.0`, `TRAJECTORY_MISSILE_SIZE = 3.0`, `AOE_DAMAGE_TARGET_CAP = 20`, `SPELL_INTERRUPT_NONPLAYER = 32747`); namespace `Trinity::WorldObjectSpell{Target,Nearby,Area,Cone,Traj,Line}TargetCheck` (functor de filtrado de targets); `using SpellEffectHandlerFn = void(Spell::*)()` |
| `src/server/game/Spells/Spell.cpp` | 9,303 | Implementación completa del pipeline. Constructor (línea 500), destructor (614), 13 selectores de targets (912-2007), `prepare` (3411), `cancel` (3579), `cast`/`_cast` (3633/3650), `handle_immediate`/`handle_delayed` (3964/4031), `_handle_immediate_phase`/`_handle_finish_phase` (4136/4156), `update` (4209), `finish` (4296), `SendSpellStart`/`SendSpellGo`/`SendCastResult`/`SendInterrupted`/`SendChannelStart`/`SendChannelUpdate`/`SendResurrectRequest` (4586-5310), `TakeCastItem`/`TakePower`/`TakeRunePower`/`TakeReagents` (5310-5557), `HandleThreatSpells` (5558), `HandleEffects` (5614 — bisagra a SpellEffects.cpp), `CheckCast` (5640 — la XL ~1050 líneas), `CheckPetCast`/`CheckCasterAuras`/`CheckSpellCancels*` (6692-6928), `CheckArenaAndRatedBattlegroundCastRules` (6929), `CheckMovement` (6960), `CanAutoCast` (6996), `CheckSrc`/`CheckDst` (7052/7058), `LoadScripts` + `CallScript*` (~25 hooks), `TargetInfo::PreprocessTarget`/`DoTargetSpellHit`/`DoDamageAndTriggers`, `DoSpellEffectHit`, `DoTriggersOnSpellHit`, `UpdateChanneledTargetList`, `HandleLaunchPhase`, `DoEffectOnLaunchTarget`, `PreprocessSpellLaunch`, `PreprocessSpellHit`, `prepareDataForTriggerSystem` |
| `src/server/game/Spells/SpellCastRequest.h` | 43 | POD `SpellCastRequest` para la queue `Player::m_pendingSpellCastRequests` (cuando se intenta cast durante GCD se encola para auto-fire al expirar) |

**Total Cast sub-module:** ~10,340 líneas (incluyendo el header).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Spell` | class | **Estado de un cast en curso.** Vida corta: nace en `Unit::CastSpell`, muere en `Spell::finish`. Owns `m_spellValue: SpellValue*`, `m_targets: SpellCastTargets`, `m_powerCost: vector<SpellPowerCost>`, `m_UniqueTargetInfo`/`m_UniqueGOTargetInfo`/`m_UniqueItemInfo`/`m_UniqueCorpseTargetInfo` (per-target hit results), `m_destTargets[MAX_SPELL_EFFECTS]` (per-effect destinations), `m_loadedScripts: vector<SpellScript*>`, `m_hitTriggerSpells`, `_executeLogEffects`. Read-only ref a `SpellInfo const*`. Field `damage: int32` (damage scratch usado por effect handlers) |
| `SpellEvent` | class : BasicEvent | El timer event que `WorldObject::m_Events` despacha cada `update(diff)`. Es lo que hace `Spell::update(difftime)` cada tick mientras el cast vive |
| `SpellValue` | struct | Override per-cast: `EffectBasePoints[MAX_SPELL_EFFECTS]`, `CustomBasePointsMask`, `MaxAffectedTargets`, `RadiusMod`, `AuraStackAmount`, `DurationMul`, `CriticalChance`, `Optional<int32> Duration`. Usado por `SetSpellValue(SpellValueMod, int32)` (e.g. trigger spell con base points custom) |
| `SpellCastTargets` | class | Targets concretos del cast: `m_unitTargetGUID`, `m_itemTargetGUID`, `m_objectTargetGUID`, `m_corpseTargetGUID`, `m_src: SpellDestination`, `m_dst: SpellDestination`, `m_strTarget`, `m_pitch`, `m_speed`, `m_targetMask: uint32` (TargetFlags). Serializado/deserializado wire por `SpellCastTargets::Read(WorldPacket&)` |
| `SpellDestination` | struct | `_position: Position`, `_transportGUID`, `_transportOffset` — origen/destino con soporte de transports |
| `SpellCastRequest` | struct (POD) | Cola de pending casts (en `Player::m_pendingSpellCastRequests`): `CastSpellTargets m_targets`, `uint8 m_castFlags`, `WorldPackets::Spells::SpellCastVisual m_spellVisual`, `int32 m_castItemData`, `ObjectGuid m_castItemGUID`, `ObjectGuid m_originalCastId` |
| `Spell::TargetInfo` | nested struct : TargetInfoBase | Per-Unit-target hit result: `TargetGUID`, `TimeDelay`, `Damage`/`Healing` (computed), `MissCondition: SpellMissInfo`, `ReflectResult`, `IsAlive`, `IsCrit`, `DRGroup: DiminishingGroup`, `AuraDuration`, `AuraBasePoints[MAX_SPELL_EFFECTS]`, `Positive`, `HitAura: UnitAura*`. Tiene `PreprocessTarget`/`DoTargetSpellHit`/`DoDamageAndTriggers` |
| `Spell::GOTargetInfo` | nested struct : TargetInfoBase | Per-GameObject-target |
| `Spell::ItemTargetInfo` | nested struct : TargetInfoBase | Per-Item-target (sharpening stones, scroll targets, enchant targets) |
| `Spell::CorpseTargetInfo` | nested struct : TargetInfoBase | Per-Corpse-target (resurrect, soulstone) |
| `Spell::HitTriggerSpell` | nested struct | Trigger spell on hit: `triggeredSpell: SpellInfo*`, `triggeredByAura: SpellInfo*`, `chance: int32` |
| `SpellState` | enum | NULL=0, PREPARING=1 (after `prepare`, before `cast`), CASTING=2 (during cast time / channel), FINISHED=3, IDLE=4, DELAYED=5 (projectile in flight) |
| `SpellEffectHandleMode` | enum | LAUNCH (cast finish, before targets resolved — for caster-side e.g. SchoolDmg pre-roll), LAUNCH_TARGET (per launch-target), HIT (immediate when projectile lands at AOE position), HIT_TARGET (per-unit-target hit) |
| `SpellCastFlags` | enum bitmask uint32 | NONE=0, PENDING=0x01 (aoe combat log), HAS_TRAJECTORY=0x02, PROJECTILE=0x20, POWER_LEFT_SELF=0x800, ADJUST_MISSILE=0x20000, NO_GCD=0x40000, VISUAL_CHAIN=0x80000, RUNE_LIST=0x200000, IMMUNITY=0x4000000, HEAL_PREDICTION=0x40000000, TRIGGER_PET_COOLDOWN=0x80000000 + ~20 unknown bits — usado en SMSG_SPELL_START/SMSG_SPELL_GO header |
| `SpellCastFlagsEx` | enum bitmask | TRIGGER_COOLDOWN_ON_SPELL_START=0x1, DONT_CONSUME_CHARGES=0x4, DELAY_STARTING_COOLDOWNS=0x10, IGNORE_PET_COOLDOWN=0x100, IGNORE_COOLDOWN=0x200, USE_TOY_SPELL=0x8000 |
| `SpellCastSource` | enum uint8 | PLAYER=2, NORMAL=3, ITEM=4, PASSIVE=7, PET=9, AURA=13, SPELL=16 — quién originó el cast |
| `SpellHealPredictionType` | enum uint8 | TARGET=0, TARGET_AND_CASTER=1, TARGET_AND_BEACON=2, TARGET_PARTY=3 — para healers preview |
| `SpellRangeFlag` | enum | DEFAULT=0, MELEE=1, RANGED=2 (hunter range / ranged weapon) |
| `TriggerCastFlags` | enum bitmask (defined in SpellDefines.h) | NONE, IGNORE_GCD, IGNORE_SPELL_AND_CATEGORY_CD, IGNORE_POWER_AND_REAGENT_COST, IGNORE_CAST_ITEM, IGNORE_AURA_SCALING, IGNORE_CAST_IN_PROGRESS, IGNORE_COMBO_POINTS, CAST_DIRECTLY, IGNORE_AURA_INTERRUPT_FLAGS, IGNORE_SET_FACING, IGNORE_SHAPESHIFT, IGNORE_CASTER_AURASTATE, IGNORE_CASTER_MOUNTED_OR_ON_VEHICLE, IGNORE_CASTER_AURAS, DONT_RESET_PERIODIC_TIMER, DONT_REPORT_CAST_ERROR, FULL_MASK — pasado a constructor |
| `Trinity::WorldObjectSpellTargetCheck` | functor base | Filtro de target por SpellTargetCheckTypes (ENEMY/ALLY/PARTY/RAID/EXOTIC/RAID_DEATH/SUMMONED/THREAT/TAP) + `SpellTargetObjectTypes` (UNIT/CORPSE/ITEM/GAMEOBJECT/PLAYER/CORPSE_ENEMY/CORPSE_ALLY/UNIT_AND_DEST). Subclases: `Nearby`, `Area`, `Cone`, `Traj`, `Line` |
| `SpellEffectHandlerFn` | typedef | `void (Spell::*)()` — el typedef que hace que la dispatch table de SpellEffects.cpp pueda guardar punteros a métodos de Spell |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Spell::Spell(WorldObject* caster, SpellInfo const*, TriggerCastFlags, originalCasterGUID, originalCastId)` | **Constructor**. Inicializa `m_caster`, copia `m_spellInfo`, calcula `m_spellSchoolMask`, `m_attackType`, `m_canReflect`, crea `m_spellValue`, registra en `caster->m_Events` un nuevo `SpellEvent` que será el ticker | `new SpellValue`, `Unit::SetCurrentCastSpell` (más tarde en prepare) |
| `Spell::~Spell()` | Destructor. Limpia `m_loadedScripts`, libera `m_spellValue`, `m_preGeneratedPath`, des-registra targets, marca `_spellEvent` para auto-delete | event cleanup |
| `Spell::prepare(SpellCastTargets const&, AuraEffect const* triggeredByAura)` | **Entry point del pipeline**. Pasos: (1) `LoadScripts` (instancia `SpellScript*` registrados), (2) `InitExplicitTargets` desde `m_targets`, (3) `prepareDataForTriggerSystem` (calcula `m_procAttacker`/`m_procVictim`/`m_procSpellType`), (4) `CallScriptOnPrecastHandler` + `CallScriptBeforeCastHandlers`, (5) `CheckCast(true, …)` — si falla envía `SMSG_CAST_FAILED` y aborta, (6) calcula `m_powerCost` (`SpellInfo::CalcPowerCost`), `m_casttime` (`SpellInfo::CalcCastTime` + auras `MOD_CAST_TIME`/`MOD_CASTING_SPEED`), (7) si instant: `cast(true)` directo; si no: `m_timer = m_casttime`, `m_spellState = PREPARING`, llama `Unit::SetCurrentCastSpell(this)`, envía `SendSpellStart`, llama `TriggerGlobalCooldown` | `LoadScripts`, `InitExplicitTargets`, `CheckCast`, `SpellInfo::CalcPowerCost`, `SpellInfo::CalcCastTime`, `Unit::SetCurrentCastSpell`, `SendSpellStart`, `TriggerGlobalCooldown` |
| `Spell::cast(bool skipCheck)` | Wrapper público. Llama `_cast(skipCheck)` con setup de `SetExecutedCurrently(true)` antifold | `_cast` |
| `Spell::_cast(bool skipCheck)` | **Cast resolution**. Pasos: (1) re-CheckCast (si !skipCheck), (2) `SelectSpellTargets` (~13 selectores implícitos según `SpellEffectInfo::ImplicitTarget[A/B]` por effect), (3) `CallScriptOnCastHandlers`, (4) `TakePower`/`TakeRunePower`/`TakeReagents`/`TakeCastItem`, (5) `HandleLaunchPhase` (effects con `LAUNCH`/`LAUNCH_TARGET` mode), (6) si delayed (proyectil): registrar en `caster->m_Events` para `handle_delayed`; si immediate: `handle_immediate`, (7) `SendSpellGo` con hit/miss list, (8) `SendSpellCooldown`, (9) si channel: `SendChannelStart(duration)`, (10) `CallScriptAfterCastHandlers` | `SelectSpellTargets`, `TakePower`, `HandleLaunchPhase`, `handle_immediate`, `SendSpellGo`, `SendChannelStart` |
| `Spell::handle_immediate()` | Inmediato (no proyectil). Pre-process per-target (`TargetInfo::PreprocessTarget`), `DoProcessTargetContainer<TargetInfo>` (que llama `DoTargetSpellHit` por cada → resuelve effects vía `DoSpellEffectHit` → `HandleEffects`), `DoProcessTargetContainer<GOTargetInfo>`, etc., luego `_handle_immediate_phase` y `_handle_finish_phase` | `DoSpellEffectHit`, `HandleEffects`, `_handle_finish_phase` |
| `Spell::handle_delayed(uint64 t_offset)` | Proyectil en vuelo. Calcula offset travel time, despacha hits que ya llegaron, retorna next delay momento. Llamado repetidamente por SpellEvent | `DoProcessTargetContainer`, `DoSpellEffectHit` |
| `Spell::_handle_immediate_phase()` | Por cada `SpellEffectInfo` en `m_spellInfo->GetEffects()`: si tiene `IsEffect()` y mode `HIT` (no per-target): `HandleEffects(nullptr, nullptr, nullptr, nullptr, effect, SPELL_EFFECT_HANDLE_HIT)` — para effects que corren una vez (e.g. environment/global) | `HandleEffects` |
| `Spell::_handle_finish_phase()` | Procesa: `HandleThreatSpells`, post-cast cleanup, threat/lifesteal aggregation, `prepareDataForTriggerSystem` final | `HandleThreatSpells`, `Unit::ProcSkillsAndAuras` |
| `Spell::HandleEffects(Unit* unitTarget, Item* itemTarget, GameObject* goTarget, Corpse* corpseTarget, SpellEffectInfo const&, SpellEffectHandleMode)` | **Bisagra a SpellEffects.cpp**. Setea `unitTarget`/`itemTarget`/`gameObjTarget`/`m_corpseTarget`/`effectInfo`/`effectHandleMode` como members, calcula `damage = CalculateDamage(effectInfo, unitTarget, &variance)`, dispatch a `SpellEffectHandlers[effectInfo.Effect](this)` (puntero a método). Cubre los 4 modes (LAUNCH, LAUNCH_TARGET, HIT, HIT_TARGET) — algunos effects corren en varios | una de las ~151 `EffectXxx` (ver [`spells-effects.md`](./spells-effects.md)) |
| `Spell::update(uint32 difftime)` | **Tick por SpellEvent**. Estado-machine: PREPARING → resta `m_timer`, si <=0 llama `cast(false)`; CASTING → si channel: chequea `UpdateChanneledTargetList` cada `SPELL_CHANNEL_UPDATE_INTERVAL` (1000ms), envía channel update, si timer<=0 llama `finish`; DELAYED → no-op (handle_delayed corre por su propio event). Verifica también `Unit::HasUnitState(UNIT_STATE_CASTING)`, movimiento (interrupt si `INTERRUPT_FLAG_MOVEMENT`), pushback (cast time se atrasa al recibir daño según `MOD_PUSHBACK_REDUCTION`) | `cast`, `finish`, `cancel` (interrupt), `SendChannelUpdate` |
| `Spell::cancel()` | Aborta el cast actual. Envía `SendInterrupted`, `SendChannelUpdate(0)`, llama `finish(SPELL_FAILED_INTERRUPTED)` | `SendInterrupted`, `finish` |
| `Spell::finish(SpellCastResult result)` | Cleanup terminal. Marca `m_spellState = FINISHED`, llama `Unit::FinishSpell`, des-registra `m_currentSpells[CURRENT_*]`, `SpellHistory::ConsumeCharge`, `CallScriptAfterHitHandlers`, threat/lifesteal final, `m_caster->m_Events` despacha el delete del Spell event después de N segundos para safety | `Unit::FinishSpell`, `SpellHistory::ConsumeCharge` |
| `Spell::CheckCast(bool strict, int32* param1, int32* param2)` | **La función XL — ~1050 líneas**. Validación maestra que devuelve `SpellCastResult`. Verifica (en orden): `Unit::HasUnitState(UNIT_STATE_DIED)`, casting allowed (mounted/on-vehicle/shapeshift compat), `CheckCasterAuras` (silence/pacify/stun/charm/fear/confuse), `CheckRange`, target alive/dead (positive vs negative), `CheckEffectTarget` per effect, `CheckLineOfSight`, `CheckPower`, `CheckRuneCost`, `CheckItems`, `CheckMovement` (cast time interrupted by movement?), `CheckArenaAndRatedBattlegroundCastRules`, faction/PvP, exhaustion (no_combat?), zone/area restrictions (`SpellMgr::GetSpellAreaMapBounds`), `BattlegroundMap::CheckCastSpell`. Cada fallo retorna un `SPELL_FAILED_*` (~250 enum values) con opcionales `param1`/`param2` (e.g. faltante reagent itemId) | `CheckRange`, `CheckPower`, `CheckItems`, `CheckCasterAuras`, `Unit::HasAuraType`, LOS check, condition check |
| `Spell::CheckPower() const` | ¿Suficiente maná/runic/energy/runa/healthcost para el cast? Read `m_powerCost`, compare contra `caster->GetPower(power)` o runas disponibles | `Unit::GetPower`, `Player::CanUseRunes` |
| `Spell::CheckRuneCost() const` | Para DK: ¿runas correctas disponibles según RuneCost.db2? | `Player::GetRunesState`, `RuneCostEntry` lookup |
| `Spell::CheckItems(int32* param1, int32* param2) const` | Reagents (`m_spellInfo->Reagent[8]`/`ReagentCount[8]`), totem requeridos (`m_spellInfo->Totem[2]`), proficiency, item target válido para enchant/disenchant/feed-pet | `Player::HasItemCount`, `Player::HasItemFitToSpellRequirements`, `Player::GetItemByGuid` |
| `Spell::CheckRange(bool strict) const` | Distancia caster↔target dentro `[GetMinMaxRange.first, .second + tolerance]`. `MAX_SPELL_RANGE_TOLERANCE = 3.0` para latency | `Unit::IsWithinDistInMap`, `GetMinMaxRange` |
| `Spell::GetMinMaxRange(bool strict) const` | Lee `SpellRangeEntry` (DBC) — RangeMin/Max para friendly/hostile, plus melee combat range modifier | DBC range entry |
| `Spell::CheckCasterAuras(int32* param1) const` | ¿El caster está silenced/pacified/stun/charm/fear/confuse y el spell no puede ignorarlo? Itera `CheckSpellCancels{Charm,Stun,Silence,Pacify,Fear,Confuse,NoActions}` per AuraType | `Unit::HasAuraType`, `Spell::CheckSpellCancelsAuraEffect` |
| `Spell::CheckMovement() const` | Si tiene cast time y no es `SpellAttr5::USABLE_WHILE_MOVING`, y caster está moving, retorna `SPELL_FAILED_MOVING` | `Unit::isMoving` |
| `Spell::SelectSpellTargets()` | Itera los effects del SpellInfo. Por cada uno mira `SpellEffectInfo::TargetA` / `TargetB` y dispatch a `SelectImplicit*Targets` apropiado: Channel (de canal previo), Nearby (rango free-form), Cone (frontal cone), Area (radio desde caster/dest/target), CasterDest/TargetDest/DestDest (área en posiciones derivadas), CasterObject/TargetObject (single object directo), Chain (chain heal/damage), Traj (trajectory missile), Line (line ground) | 13 selectores siguientes |
| `Spell::SelectImplicitNearbyTargets(...)` | Random nearby objeto según check (single target buscando alguien) | `SearchNearbyTarget`, `WorldObjectSpellNearbyTargetCheck` functor |
| `Spell::SelectImplicitConeTargets(...)` | Frontal cone (Mind Flay, Cone of Cold, Howl of Terror) | `SearchAreaTargets`, `WorldObjectSpellConeTargetCheck` |
| `Spell::SelectImplicitAreaTargets(...)` | Esfera radio R desde origen (Consecration, Blizzard, Hellfire, Holy Nova) | `SearchAreaTargets`, `WorldObjectSpellAreaTargetCheck` |
| `Spell::SelectImplicitChainTargets(...)` | Chain Lightning / Chain Heal — N saltos, prioriza más bajo HP (heal) o más cercano enemigo (damage) | `SearchChainTargets` |
| `Spell::SelectImplicitTrajTargets(...)` | Bombs/grenade trajectories (Mortal Coil, Earthbind grenade) | `WorldObjectSpellTrajTargetCheck` |
| `Spell::SelectImplicitLineTargets(...)` | Line ground (Howl of Terror?) | `WorldObjectSpellLineTargetCheck` |
| `Spell::AddUnitTarget(Unit* target, uint32 effectMask, bool checkIfValid, bool implicit, Position const* losPosition)` | Añade Unit a `m_UniqueTargetInfo` con effectMask, calcula `MissCondition` (`Unit::SpellHitResult` rolls miss/dodge/parry/resist), `ReflectResult`, `IsCrit`, `DRGroup`, registra delay si proyectil | `Unit::SpellHitResult`, `DiminishingGroup` lookup |
| `Spell::AddGOTarget`, `AddItemTarget`, `AddCorpseTarget`, `AddDestTarget` | Análogos para non-Unit targets | — |
| `Spell::TargetInfo::PreprocessTarget(Spell*)` | Pre-cast: si Reflect → swap target a caster; calcula HitAura (creates `UnitAura` ahead of time si effect es ApplyAura, para tener AuraDuration calculado para SMSG_SPELL_GO) | `Aura::TryRefreshStackOrCreate` (si ApplyAura) |
| `Spell::TargetInfo::DoTargetSpellHit(Spell*, SpellEffectInfo const&)` | Per-target dispatch a `Spell::HandleEffects(unit, …, SPELL_EFFECT_HANDLE_HIT_TARGET)` por cada effect del effectMask | `HandleEffects`, `DoSpellEffectHit` |
| `Spell::TargetInfo::DoDamageAndTriggers(Spell*)` | Post-effect: aplica `m_damage`/`m_healing` acumulados via `Unit::DealDamage`/`Unit::HealBySpell`, dispatch a `Unit::ProcSkillsAndAuras`, sends `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` o `SMSG_SPELLHEALLOG` | `Unit::DealDamage`, `Unit::HealBySpell`, `Unit::ProcSkillsAndAuras`, `SendSpellNonMeleeDamageLog`, `SendHealSpellLog` |
| `Spell::CalculateDamage(SpellEffectInfo const&, Unit const* target, float* var)` | Roll de daño: `BasePoints + RandomPoints(DiePerLevel × DiceMax) + BasePointsMod + ComboPoints×PointsPerComboPoint`, aplica auras `MOD_DAMAGE_DONE`/`MOD_DAMAGE_PERCENT_DONE`, crit | random + bonus calc |
| `Spell::SendSpellStart()` | SMSG_SPELL_START — header, caster GUID, cast time, cast flags, m_targets serialize, runes (si DK), ammo, immunity, healPrediction. Broadcast `Map::SendMessageInRange` o sólo a caster | `WorldPacket`, `Map::SendMessageInRange` |
| `Spell::SendSpellGo()` | SMSG_SPELL_GO — header, hit list (GUIDs), miss list (GUIDs+miss reason), cast flags, runes, ammo, target string. Triggea client visual de proyectil | `WorldPacket`, `Map::SendMessageInRange` |
| `Spell::SendCastResult(SpellCastResult, int32* param1, int32* param2)` | SMSG_CAST_FAILED al caster con reason + opcionales params (faltante reagent ID, faltante skill, etc.) | `WorldSession::SendPacket` |
| `Spell::SendInterrupted(uint8 result)` | SMSG_SPELL_FAILURE + SMSG_SPELL_FAILED_OTHER (broadcast) cuando se interrumpe mid-cast | `Map::SendMessageInRange` |
| `Spell::SendChannelStart(uint32 duration)` | SMSG_CHANNEL_START — channel inicio, duration, immunity, healPrediction | `Map::SendMessageInRange` |
| `Spell::SendChannelUpdate(uint32 time)` | SMSG_CHANNEL_UPDATE — tiempo restante de canal (0 = stop) | caster only |
| `Spell::TakePower()` | Resta `m_powerCost` del caster, broadcast `MoveSetActiveMover` o power update vía values, send `SMSG_POWER_UPDATE` | `Unit::ModifyPower`, `Unit::UpdatePowerType` |
| `Spell::TakeRunePower(bool didHit)` | DK: marca runas usadas como `cooldown`, broadcast `SMSG_RUNES_RESTORE_FAILED` si fallaste | `Player::SetRuneCooldown` |
| `Spell::TakeReagents()` | Player only: destruye reagents del inventory si !`SpellAttrEx5::NO_REAGENT_WHILE_PREP` | `Player::DestroyItemCount` |
| `Spell::TriggerGlobalCooldown()` | Setea GCD según `SpellInfo::StartRecoveryCategory` + `StartRecoveryTime`, modificado por haste | `SpellHistory::AddGlobalCooldown` |
| `Spell::HasGlobalCooldown() const` | True si caster está en GCD para esta category | `SpellHistory::HasGlobalCooldown` |
| `Spell::CancelGlobalCooldown()` | Cancela GCD (e.g. cast falló pre-power-take) | `SpellHistory::CancelGlobalCooldown` |
| `Spell::HandleLaunchPhase()` | Para cada effect con `IsEffectLaunch()`: setear `damage = CalculateDamage`, `HandleEffects(SPELL_EFFECT_HANDLE_LAUNCH/LAUNCH_TARGET)`. Caster-side pre-roll | `HandleEffects`, `CalculateDamage` |
| `Spell::DoSpellEffectHit(Unit*, SpellEffectInfo const&, TargetInfo&)` | Per-effect-per-target hit: setea per-target `damage`/`healing`, llama `HandleEffects(unit, …, SPELL_EFFECT_HANDLE_HIT_TARGET)`, acumula resultado en `TargetInfo::Damage`/`Healing` | `HandleEffects` |
| `Spell::DoTriggersOnSpellHit(Unit*)` | Aplica `m_hitTriggerSpells` (cast trigger spells si chance roll succeed) | `Unit::CastSpell` (recursive cast con chain length cap) |
| `Spell::UpdateChanneledTargetList()` | Cada channel update (1s default) re-checks que los targets siguen vivos/in-range; quita los inválidos | `IsValidDeadOrAliveTarget`, `CheckRange` |
| `Spell::PrepareTriggersExecutedOnHit()` | Construye `m_hitTriggerSpells` desde el SpellInfo (`SpellTriggered`/`TriggerSpellId` per effect) + auras pasivas con `MOD_PROC_TRIGGER_SPELL` | `SpellInfo::Effects`, aura iteration |
| `Spell::CallScriptOnPrecastHandler` / `CallScriptBeforeCastHandlers` / `CallScriptOnCastHandlers` / `CallScriptAfterCastHandlers` / `CallScriptCheckCastHandlers` / `CallScriptCalcCastTimeHandlers` / `CallScriptEffectHandlers` / `CallScriptSuccessfulDispel` / `CallScriptBeforeHitHandlers` / `CallScriptOnHitHandlers` / `CallScriptAfterHitHandlers` / `CallScriptCalcCritChanceHandlers` / `CallScriptCalcDamageHandlers` / `CallScriptCalcHealingHandlers` / `CallScriptObjectAreaTargetSelectHandlers` / `CallScriptObjectTargetSelectHandlers` / `CallScriptDestinationTargetSelectHandlers` | ~25 hooks de SpellScript DSL — itera `m_loadedScripts` y delega | SpellScript |
| `Spell::IsTriggered() const` | True si `_triggeredCastFlags != TRIGGERED_NONE` | flag check |
| `Spell::IsChannelActive() const` | True si state CASTING + spellInfo es channeled | — |
| `Spell::IsAutoActionResetSpell() const` | True si spell debería resetear auto-attack timer | spellInfo flag |
| `Spell::IsPositive() const` | True si todos los effects son positivos (buff vs debuff) | `SpellInfo::IsPositiveEffect` |
| `Spell::CanAutoCast(Unit* target)` | Pet AI: ¿puede pet auto-castar este spell sobre este target? | `CheckCast` con flags relajados |
| `Spell::SetSpellValue(SpellValueMod, int32)` | Override of `m_spellValue` field — usado por trigger spells para BasePoints custom | `m_spellValue` write |
| `Spell::CalculateDelayMomentForDst(float launchDelay) const` | Travel time de proyectil desde caster a `m_destTargets[X]._position` según `SpellInfo::Speed` | distance / speed |
| `Spell::IsWithinLOS(WorldObject const* source, WorldObject const* target, bool, ModelIgnoreFlags)` | Wrapper a `Map::isInLineOfSight` con M2/WMO checks | `VMAP::ModelIgnoreFlags` |

**Total surface:** ~120 públicos en `Spell.h`. `CheckCast` por sí sola es ~1050 líneas.

---

## 5. Module dependencies

**Depends on:**
- `SpellInfo` ([`spells-info.md`](./spells-info.md)) — read-only ref via `m_spellInfo`. Cada call a `SpellInfo::CalcCastTime`, `CalcPowerCost`, `CalcDuration`, `GetEffects`, `IsPositiveEffect`, `Reagent[]`, `Totem[]`, `RangeEntry`
- `SpellMgr` ([`spells-mgr.md`](./spells-mgr.md)) — `GetSpellInfo(spellId)` lookup; `GetSpellAreaMapBounds`, `GetSpellChainNode`, `GetSpellTargetPosition`, `GetSpellThreatEntry`, `mSpellLearnSpells`
- `SpellHistory` — para GCD (`HasGlobalCooldown`, `AddGlobalCooldown`, `CancelGlobalCooldown`), cooldowns spell-side y category-side, charges, `SendSpellCooldown`
- `Unit` ([`entities-unit.md`](./entities-unit.md)) — `m_currentSpells[CURRENT_*]` (gen/auto-repeat/melee/channel slot), `SetCurrentCastSpell`, `FinishSpell`, `InterruptSpell`, `HasUnitState(UNIT_STATE_CASTING)`, `IsValidAttackTarget`/`IsValidAssistTarget`, `SpellHitResult` (miss/parry/dodge/block roll), `GetMaxSkillValueForLevel`, `m_Events` (BasicEvent registry), `ProcSkillsAndAuras`, `DealDamage`, `HealBySpell`, `ModifyPower`
- `Aura` ([`spells-aura.md`](./spells-aura.md)) — `Aura::TryRefreshStackOrCreate` (en TargetInfo::PreprocessTarget para EffectApplyAura), `Aura::HandleAuraSpecificMods`
- `SpellEffects.cpp` ([`spells-effects.md`](./spells-effects.md)) — los ~151 `EffectXxx` que `HandleEffects` despacha
- `SpellScript` — `m_loadedScripts: vector<SpellScript*>`, `LoadScripts()` busca en `ScriptMgr` por spellId
- `ConditionMgr` — `WorldObjectSpellTargetCheck` consulta `_condList: ConditionContainer` para filtrado de targets
- `Map` / `MapManager` — `Map::SendMessageInRange` (broadcast SMSG_SPELL_*), `Map::isInLineOfSight` (LOS), `WorldObjectSearcher` patterns
- `MotionMaster` / `PathGenerator` — `EffectJump`/`Charge`/`JumpDest`: `m_preGeneratedPath: unique_ptr<PathGenerator>`
- `WorldPackets::Spells` — `SpellCastData`, `SpellHealPrediction`, `SpellCastVisual` structs serialize
- `BattlegroundMap`, `BattlefieldMgr` — `CheckArenaAndRatedBattlegroundCastRules` consulta a éstos
- `DBCStores` — `SpellRangeEntry`, `SpellCastTimesEntry`, `SpellDurationEntry`, `SpellPowerEntry`, `SpellEffectEntry`, `SkillLineEntry`, `RuneCostEntry`

**Depended on by:**
- `Unit::CastSpell` (overloaded) — todos los entry points de cast public
- `WorldSession::HandleCastSpellOpcode` (player CMSG_CAST_SPELL handler) — crea SpellCastTargets, llama Player::CastSpell
- `WorldSession::HandlePetCastSpellOpcode` (CMSG_PET_CAST_SPELL)
- `WorldSession::HandleUseItemOpcode` (CMSG_USE_ITEM) — items con on-use spell
- `Vehicle::Reset` — vehicle exit casts
- `Pet::Update` — pet auto-cast loop
- `CreatureAI` — `Unit::CastSpell` desde scripts
- `SpellScript::FinishCast(SpellCastResult)`
- `Aura` — `Aura::Update` puede triggerar (`PERIODIC_TRIGGER_SPELL` aura → `Unit::CastSpell`)
- Toda combat / movement / threat / item / quest piping que invoque a `CastSpell`

---

## 6. SQL / DB queries (if any)

`Spell` runtime no emite queries directamente — todas las lookups van a structures pre-cargadas por `SpellMgr` (ver [`spells-mgr.md`](./spells-mgr.md) §6). El runtime sí hace lecturas indirectas via cache:

| Statement / Source | Purpose | DB |
|---|---|---|
| (vía `SpellMgr::mSpellInfoMap`) | Lookup SpellInfo por spellId | world (cargado en startup desde DB2) |
| (vía `SpellMgr::mSpellTargetPositions`) | EffectTeleportUnits destination | world |
| (vía `SpellMgr::mSpellAreaMap`) | Spell area enable/disable por zone/quest | world |
| (vía `SpellMgr::mSpellChains`) | Rank lookup (e.g. cast Lower rank si ya tienes higher) | world |
| (vía `SpellMgr::mSpellThreatMap`) | Threat override per-spell para `HandleThreatSpells` | world |
| (vía `SpellMgr::mSpellProcMap`) | Proc rules para `m_hitTriggerSpells` | world |
| (vía `SpellHistory::SaveCooldownStateBeforeDuel` / `LoadFromDB`) | Persist cooldowns per-character | characters (`character_spell_cooldown`) |

DB2/DBC stores leídas runtime (vía DBC global `sXXXStore`):

| Store | What it loads | Read by |
|---|---|---|
| `SpellRange.db2` | RangeMin/Max for melee/ranged/general | `Spell::GetMinMaxRange` → `CheckRange` |
| `SpellCastTimes.db2` | Base cast time index → ms | `SpellInfo::CalcCastTime` |
| `SpellDuration.db2` | Aura/effect duration per spellLevel | `SpellInfo::CalcDuration`, `Aura::SetMaxDuration` |
| `SpellPower.db2` | PowerType/PowerCost/PowerCostPct/PowerCostPerLevel | `SpellInfo::CalcPowerCost` → `Spell::CheckPower` |
| `SpellEffect.db2` | Cada effect index: effect type, BasePoints, MiscValue, ImplicitTarget, Mechanic, RadiusEntry, ChainTargets | leído al construir SpellInfo |
| `SpellRune.db2` / `RuneCost.db2` | DK rune cost | `Spell::CheckRuneCost`, `TakeRunePower` |
| `SpellShapeshiftForm.db2` | Shapeshift restrictions | `CheckCast` shapeshift logic |
| `SpellRadius.db2` | Effect AOE radius | `SelectImplicitAreaTargets` |
| `SpellMissile.db2` | Speed/launch/trajectory | `CalculateDelayMomentForDst` |
| `Lock.db2` | Ranks for EffectOpenLock | (en SpellEffects, no aquí) |

---

## 7. Wire-protocol packets (if any)

Cast pipeline emite y consume estos opcodes (los principales — Spell.cpp emite >25 SMSG distintos):

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_CAST_SPELL` | C → S | `WorldSession::HandleCastSpellOpcode` → `Player::CastSpell` → `Spell::prepare` |
| `CMSG_PET_CAST_SPELL` | C → S | `WorldSession::HandlePetCastSpellOpcode` |
| `CMSG_USE_ITEM` | C → S | `WorldSession::HandleUseItemOpcode` |
| `CMSG_CANCEL_CAST` | C → S | `WorldSession::HandleCancelCastOpcode` → `Spell::cancel` |
| `CMSG_CANCEL_CHANNELLING` | C → S | `WorldSession::HandleCancelChannellingOpcode` |
| `CMSG_CANCEL_AUTO_REPEAT_SPELL` | C → S | for auto-shot cancel |
| `CMSG_UPDATE_PROJECTILE_POSITION` | C → S | mid-flight projectile sync |
| `CMSG_NO_SPELL_VARIANCE` | C → S | client signals no preview variance |
| `SMSG_SPELL_START` | S → C | `Spell::SendSpellStart` (broadcast in range) — UI shows cast bar |
| `SMSG_SPELL_GO` | S → C | `Spell::SendSpellGo` (broadcast) — projectile fire / instant resolve, includes hit/miss list |
| `SMSG_CAST_FAILED` | S → C | `Spell::SendCastResult` (caster only) — fail reason + params |
| `SMSG_SPELL_FAILURE` | S → C | `Spell::SendInterrupted` (broadcast) — interrupted mid-cast |
| `SMSG_SPELL_FAILED_OTHER` | S → C | broadcast version of failure to others |
| `SMSG_SPELL_DELAYED` | S → C | pushback notification |
| `SMSG_CHANNEL_START` | S → C | `Spell::SendChannelStart` |
| `SMSG_CHANNEL_UPDATE` | S → C | `Spell::SendChannelUpdate` (each tick) |
| `SMSG_SPELL_INTERRUPT_LOG` | S → C | `Spell::SendSpellInterruptLog` (interrupt of OTHER caster's spell) |
| `SMSG_SPELL_EXECUTE_LOG` | S → C | `Spell::SendSpellExecuteLog` — payload con ExecuteLogEffect* (PowerDrain, ExtraAttacks, DurabilityDamage, OpenLock, CreateItem, DestroyItem, SummonObject, UnsummonObject, Resurrect) |
| `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` | S → C | en `TargetInfo::DoDamageAndTriggers` post-effect |
| `SMSG_SPELLHEALLOG` | S → C | en `TargetInfo::DoDamageAndTriggers` post-heal |
| `SMSG_SPELLENERGIZELOG` | S → C | EffectEnergize (cross-ref [`spells-effects.md`](./spells-effects.md)) |
| `SMSG_SPELL_DISPELL_LOG` | S → C | EffectDispel |
| `SMSG_SPELL_COOLDOWN` | S → C | `Spell::SendSpellCooldown` post-cast |
| `SMSG_RESURRECT_REQUEST` | S → C | `Spell::SendResurrectRequest` (target gets resurrect prompt) |
| `SMSG_MOUNT_RESULT` | S → C | `Spell::SendMountResult` |
| `SMSG_PET_CAST_FAILED` | S → C | `Spell::SendPetCastResult` |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 líneas** (verified `wc -l`) — la crate target del módulo está completamente vacía. No hay `Spell` struct, no hay `SpellEvent`, no hay `SpellValue`, no hay `SpellCastTargets`, no hay `SpellState` enum, no hay `SpellEffectHandleMode`, no hay `SpellCastFlags`, no hay `TriggerCastFlags`, no hay `SpellCastSource`, no hay `Trinity::WorldObjectSpell*Check` functor equivalent
- `crates/wow-world/src/handlers/spell.rs` — 288 líneas — registra 3 handlers (`CastSpell`, `CancelCast`, `CancelChannelling`) vía `inventory::submit!`. `handle_cast_spell` parsea `CastSpellRequest`, valida `known_spells` (SET en sesión), valida un cooldown rudimentario (campo del session), si `cast_time > 0` envía `SMSG_SPELL_START` con `SpellCastVisual` hardcoded y guarda `active_spell_cast` en la session, si instant llama un `execute_spell_effects()` interno que sólo loguea
- `crates/wow-packet/src/packets/spell.rs` — 466 líneas — define wire structs: `CastSpellRequest` (read), `SpellTargetData`, `SpellCastVisual` (Read+Write), `SpellStartPkt`, `CastFailed`, `SpellCooldownPkt`, opcodes y un puñado de visualizers básicos
- `crates/wow-data/src/spell.rs` — `SpellInfo` stub con `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, `has_cast_time` — NO tiene `effects: Vec<SpellEffectInfo>`, NO tiene power cost, NO tiene range entries, NO tiene reagents, NO tiene shapeshift restrictions, NO tiene channel duration, NO tiene rune cost, NO tiene proc info

**What's implemented:**
- CMSG_CAST_SPELL parse + ack mínimo (SMSG_SPELL_START + SMSG_CAST_FAILED si unknown spell o on-cooldown rudimentario)
- Persistencia de un `active_spell_cast: Option<ActiveSpellCast>` por session que el tick de session decrementa (cast time visible en client)
- "Cooldown" rudimentario tipo `HashMap<SpellId, Instant>` en session — sin GCD, sin category, sin charges
- CMSG_CANCEL_CAST handler que limpia `active_spell_cast`
- Visual handshake tal que el client cree que algo pasó

**What's missing vs C++:**
- **Toda la clase `Spell`**: constructor, destructor, lifetime management, no existe
- **Pipeline completo** `prepare → cast → handle_immediate / handle_delayed → _handle_finish_phase → finish` — ningún paso implementado más allá del ack
- **`CheckCast`** (la función XL ~1050 líneas en C++): no existe verificación de power, range, LOS, items/reagents, totems, runes, caster auras (silence/pacify/stun), shapeshift, mounted, on-vehicle, zone restrictions, faction, PvP, arena rules, movement
- **Target enumeration**: ninguno de los 13 selectores `SelectImplicit*Targets` existe — sin Cone, Area, Chain, Trajectory, Line, Channel, Nearby, CasterDest/TargetDest/DestDest. Los `WorldObjectSpell{Target,Nearby,Area,Cone,Traj,Line}TargetCheck` functor analogues no existen
- **Miss/hit roll**: `SpellHitResult` (miss/dodge/parry/resist) no existe; no hay `TargetInfo::PreprocessTarget` con reflect / DR group / aura duration; no hay `DoDamageAndTriggers`
- **Power consumption**: `TakePower`, `TakeRunePower`, `TakeReagents`, `TakeCastItem` no existen
- **Channel system**: `SendChannelStart`, `SendChannelUpdate`, `UpdateChanneledTargetList`, `SPELL_CHANNEL_UPDATE_INTERVAL` tick — nada
- **Triggered spell chain**: `m_procChainLength` cap, `_triggeredCastFlags`, `m_hitTriggerSpells` post-hit dispatch — nada
- **GCD**: `TriggerGlobalCooldown`, `HasGlobalCooldown`, `CancelGlobalCooldown` — el "cooldown" actual es ad-hoc per-spell, no diferencia GCD vs spell CD vs category CD
- **Projectile travel time**: `CalculateDelayMomentForDst`, `handle_delayed`, `m_delayStart`/`m_delayMoment`, `IsDelayableNoMore` — nada (todos los hits son instantáneos en Rust hoy)
- **Pushback**: cast time delay al recibir damage según `MOD_PUSHBACK_REDUCTION` — nada (el cast nunca se atrasa)
- **Interrupt flags**: `SPELL_INTERRUPT_FLAG_MOVEMENT`/`PUSH_BACK`/`INTERRUPT`/`AUTOATTACK`/`DAMAGE` evaluation — nada
- **Reflect**: `m_canReflect`, target swap si target tiene reflect aura — nada
- **`SpellEvent` (BasicEvent)**: el ticker que avanza cast time — el equivalente Rust hoy es polling en `WorldSession::tick`, sin event-loop genérico
- **`SpellScript` DSL**: ~25 hooks (`OnPrecast`, `BeforeCast`, `OnCast`, `AfterCast`, `CheckCast`, `CalcCastTime`, `OnEffectHit`/`Launch`/`Apply`/`Remove`, `BeforeHit`/`OnHit`/`AfterHit`, `CalcCritChance`, `CalcDamage`, `CalcHealing`, `ObjectAreaTargetSelect`, `ObjectTargetSelect`, `DestinationTargetSelect`, `SuccessfulDispel`) — ninguno existe
- **`HandleEffects` bisagra a SpellEffects.cpp**: el dispatch table de 151 entries no existe en Rust (cross-ref [`spells-effects.md`](./spells-effects.md))
- **SMSG_SPELL_GO**: el packet con hit/miss list, runes y ammo. Hoy se usa SpellStartPkt como sustituto wire pero es semánticamente distinto (start vs go)
- **`Unit::m_currentSpells[CURRENT_*]`** (slot por tipo: GENERIC, MELEE, AUTO_REPEAT, CHANNELED) — no existe en `Unit`/`Player` Rust
- **Pet cast routing** (CMSG_PET_CAST_SPELL → CheckPetCast) — no existe
- **Item cast routing** (CMSG_USE_ITEM → spell con CastItemEntry/Level) — solo plumbing wire, sin lógica
- **Dispel mechanic resolution** (EffectDispel target picking) — no existe

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- El `SMSG_SPELL_START` actual lleva `cast_time` derivado de `SpellInfo::cast_time_ms` directo — sin aplicar mods de haste/`MOD_CASTING_SPEED`, así que casts hasted no se ven hasted en el client. Probable bug visible: bloodlust/heroism no acelera la barra
- El `effective_cooldown_ms` se aplica como cooldown único; spells con category cooldown (e.g. potions, blessings) van a tener cooldowns mal sincronizados entre los del mismo grupo
- La validación `known_spells.contains(&spell_id)` es necesaria pero insuficiente — hay spells que se castean sin estar en known (auras pasivas que triggerean otros spells, item-on-use, vehicle spells)

**Tests existing:**
- 0 tests en `crates/wow-spell/` (crate vacía)
- ~3 wire-format round-trip tests en `crates/wow-packet/src/packets/spell.rs` (`CastSpellRequest::read`, `SpellStartPkt::write`, `CastFailed::write`)
- 0 integration tests del pipeline cast end-to-end

---

## 9. Migration sub-tasks

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

**Foundation types (en `wow-spell/src/cast/`):**
- [ ] **#SPELLS-CAST.1** Definir `enum SpellState` (NULL/PREPARING/CASTING/FINISHED/IDLE/DELAYED) (L)
- [ ] **#SPELLS-CAST.2** Definir `enum SpellEffectHandleMode` (LAUNCH/LAUNCH_TARGET/HIT/HIT_TARGET) (L)
- [ ] **#SPELLS-CAST.3** Definir `bitflags SpellCastFlags` (~32 flags) y `SpellCastFlagsEx` (L)
- [ ] **#SPELLS-CAST.4** Definir `bitflags TriggerCastFlags` (~18 flags) (L)
- [ ] **#SPELLS-CAST.5** Definir `enum SpellCastSource` (PLAYER/NORMAL/ITEM/PASSIVE/PET/AURA/SPELL) (L)
- [ ] **#SPELLS-CAST.6** Definir `struct SpellValue { effect_base_points, custom_basepoints_mask, max_affected_targets, radius_mod, aura_stack_amount, duration_mul, critical_chance, duration: Option<i32>, … }` con constructor desde `SpellInfo` (M)
- [ ] **#SPELLS-CAST.7** Definir `struct SpellDestination { position: Position, transport_guid: Option<ObjectGuid>, transport_offset: Position }` (L)
- [ ] **#SPELLS-CAST.8** Definir `struct SpellCastTargets { unit_target_guid, item_target_guid, object_target_guid, corpse_target_guid, src: SpellDestination, dst: SpellDestination, str_target, pitch, speed, target_mask: SpellCastTargetFlags }` con `read(&mut WorldPacket)` y `write` (M)
- [ ] **#SPELLS-CAST.9** Definir `struct SpellLogEffect*Params` (PowerDrain, ExtraAttacks, DurabilityDamage, GenericVictim, TradeSkillItem, FeedPet) (L)

**Spell core struct (en `wow-spell/src/cast/spell.rs`):**
- [ ] **#SPELLS-CAST.10** Definir `struct Spell { spell_info: Arc<SpellInfo>, caster_guid, target_guid, cast_id, cast_item_guid, cast_item_entry, cast_item_level, from_client, cast_flags_ex, spell_visual, m_targets: SpellCastTargets, custom_error, applied_mods, casttime, channeled_duration, can_reflect, auto_repeat, runes_state, delay_at_damage_count, delay_start, delay_moment, launch_handled, immediate_handled, executed_currently, m_state: SpellState, timer, triggered_cast_flags, triggered_by_aura_spell, proc_chain_length, … }` (XL — la struct tiene >50 fields)
- [ ] **#SPELLS-CAST.11** Implementar `Spell::new(caster, info, trigger_flags, original_caster_guid, original_cast_id) -> Self` (constructor con calc inicial de school mask, attack type, can_reflect) (M)
- [ ] **#SPELLS-CAST.12** Implementar `Spell::drop` cleanup (ownership ordering — los target infos no deben colgar refs a Spell muerto) (M)

**Pipeline — preparation:**
- [ ] **#SPELLS-CAST.13** `Spell::init_explicit_targets(&mut self, targets: &SpellCastTargets)` (M)
- [ ] **#SPELLS-CAST.14** `Spell::prepare(&mut self, targets: SpellCastTargets, triggered_by_aura: Option<&AuraEffect>) -> SpellCastResult` (H — el orchestrator de fase 1)
- [ ] **#SPELLS-CAST.15** `Spell::prepare_data_for_trigger_system` — calcula `proc_attacker`/`proc_victim`/`proc_spell_type` (M)
- [ ] **#SPELLS-CAST.16** `Spell::send_spell_start` con SMSG_SPELL_START correcto (header, runes, ammo, immunity, healPrediction) (H)

**Pipeline — validation (CheckCast & friends):**
- [ ] **#SPELLS-CAST.17** `Spell::check_cast(strict, &mut param1, &mut param2) -> SpellCastResult` (XL — ~1050 C++ lines, splittear en sub-funciones)
- [ ] **#SPELLS-CAST.18** `Spell::check_caster_auras(&mut param1) -> SpellCastResult` + `check_spell_cancels_{charm,stun,silence,pacify,fear,confuse,no_actions,aura_effect}` (H)
- [ ] **#SPELLS-CAST.19** `Spell::check_range(strict) -> SpellCastResult` + `get_min_max_range(strict) -> (f32, f32)` (M)
- [ ] **#SPELLS-CAST.20** `Spell::check_power() -> SpellCastResult` (lee `m_power_cost: Vec<SpellPowerCost>` y compara contra `caster.get_power(power)`) (M)
- [ ] **#SPELLS-CAST.21** `Spell::check_rune_cost() -> SpellCastResult` (DK only) (M)
- [ ] **#SPELLS-CAST.22** `Spell::check_items(&mut param1, &mut param2) -> SpellCastResult` (reagents, totems, item proficiency, item-target valid) (H)
- [ ] **#SPELLS-CAST.23** `Spell::check_movement() -> SpellCastResult` (M)
- [ ] **#SPELLS-CAST.24** `Spell::check_arena_and_rated_battleground_cast_rules() -> SpellCastResult` (L — defer hasta arenas)
- [ ] **#SPELLS-CAST.25** `Spell::check_pet_cast(target: &Unit) -> SpellCastResult` (M)
- [ ] **#SPELLS-CAST.26** `Spell::check_effect_target(target, effect_info, los_position) -> bool` per Unit/GameObject/Item (M)

**Pipeline — target enumeration:**
- [ ] **#SPELLS-CAST.27** `Spell::select_explicit_targets()` (L)
- [ ] **#SPELLS-CAST.28** `Spell::select_spell_targets()` — orchestrator que despacha a los 13 selectores (M)
- [ ] **#SPELLS-CAST.29** `Spell::select_implicit_channel_targets(effect_info, target_type)` (M)
- [ ] **#SPELLS-CAST.30** `Spell::select_implicit_nearby_targets(effect_info, target_type, target_index, eff_mask)` (M)
- [ ] **#SPELLS-CAST.31** `Spell::select_implicit_cone_targets(...)` (M)
- [ ] **#SPELLS-CAST.32** `Spell::select_implicit_area_targets(...)` (H — más usado, Consecration/Hellfire/Blizzard)
- [ ] **#SPELLS-CAST.33** `Spell::select_implicit_caster_dest_targets`/`target_dest_targets`/`dest_dest_targets` (M ×3)
- [ ] **#SPELLS-CAST.34** `Spell::select_implicit_caster_object_targets`/`target_object_targets` (M ×2)
- [ ] **#SPELLS-CAST.35** `Spell::select_implicit_chain_targets(...)` (Chain Lightning/Heal — H)
- [ ] **#SPELLS-CAST.36** `Spell::select_implicit_traj_targets(...)` (H — geometric trajectory)
- [ ] **#SPELLS-CAST.37** `Spell::select_implicit_line_targets(...)` (M)
- [ ] **#SPELLS-CAST.38** `Spell::select_effect_type_implicit_targets(effect_info)` (M)
- [ ] **#SPELLS-CAST.39** `WorldObjectSpellTargetCheck` (trait + impls Nearby/Area/Cone/Traj/Line) (XL — ~5 functor classes)
- [ ] **#SPELLS-CAST.40** `Spell::search_nearby_target`/`search_area_targets`/`search_chain_targets` (H)
- [ ] **#SPELLS-CAST.41** `Spell::add_unit_target`/`add_go_target`/`add_item_target`/`add_corpse_target`/`add_dest_target` (M)
- [ ] **#SPELLS-CAST.42** `Spell::TargetInfo` + `GOTargetInfo` + `ItemTargetInfo` + `CorpseTargetInfo` con `preprocess_target`/`do_target_spell_hit`/`do_damage_and_triggers` (XL)
- [ ] **#SPELLS-CAST.43** `Spell::preprocess_spell_launch`/`preprocess_spell_hit` con miss/dodge/parry/resist/reflect roll (H)

**Pipeline — cast resolution:**
- [ ] **#SPELLS-CAST.44** `Spell::cast(skip_check)` wrapper + `_cast(skip_check)` body (H)
- [ ] **#SPELLS-CAST.45** `Spell::handle_launch_phase()` + `do_effect_on_launch_target` (M)
- [ ] **#SPELLS-CAST.46** `Spell::handle_immediate()` + `do_process_target_container<T>` (H)
- [ ] **#SPELLS-CAST.47** `Spell::handle_delayed(t_offset) -> u64` (proyectiles) (H)
- [ ] **#SPELLS-CAST.48** `Spell::_handle_immediate_phase()` y `_handle_finish_phase()` (M)
- [ ] **#SPELLS-CAST.49** `Spell::handle_effects(unit, item, go, corpse, effect_info, mode)` — bisagra a effect dispatch table (cross-ref [`spells-effects.md`](./spells-effects.md)) (M)
- [ ] **#SPELLS-CAST.50** `Spell::do_spell_effect_hit(unit, effect_info, target_info)` (M)
- [ ] **#SPELLS-CAST.51** `Spell::do_triggers_on_spell_hit(unit)` + `prepare_triggers_executed_on_hit` + `m_hit_trigger_spells` (H)

**Pipeline — channel + delayed:**
- [ ] **#SPELLS-CAST.52** `Spell::send_channel_start(duration)` SMSG_CHANNEL_START (M)
- [ ] **#SPELLS-CAST.53** `Spell::send_channel_update(time)` SMSG_CHANNEL_UPDATE (L)
- [ ] **#SPELLS-CAST.54** `Spell::update_channeled_target_list() -> bool` (M)
- [ ] **#SPELLS-CAST.55** `Spell::delayed`/`delayed_channel` (pushback) (M)
- [ ] **#SPELLS-CAST.56** `Spell::calculate_delay_moment_for_dst(launch_delay) -> u64` + `recalculate_delay_moment_for_dst` + `update_delay_moment_for_unit_target` (M)

**Pipeline — power / cost / cooldown:**
- [ ] **#SPELLS-CAST.57** `Spell::take_power()` (M)
- [ ] **#SPELLS-CAST.58** `Spell::take_rune_power(did_hit)` (DK) (M)
- [ ] **#SPELLS-CAST.59** `Spell::take_reagents()` (M)
- [ ] **#SPELLS-CAST.60** `Spell::take_cast_item()` (M)
- [ ] **#SPELLS-CAST.61** `Spell::trigger_global_cooldown` + `has_global_cooldown` + `cancel_global_cooldown` (delegados a `SpellHistory`) (M)
- [ ] **#SPELLS-CAST.62** `Spell::send_spell_cooldown` SMSG_SPELL_COOLDOWN post-cast (L)

**Pipeline — finish + cancel:**
- [ ] **#SPELLS-CAST.63** `Spell::cancel()` (M)
- [ ] **#SPELLS-CAST.64** `Spell::finish(result: SpellCastResult)` (M)
- [ ] **#SPELLS-CAST.65** `Spell::send_spell_go()` SMSG_SPELL_GO con hit/miss list, runes, ammo (H)
- [ ] **#SPELLS-CAST.66** `Spell::send_cast_result(result, &mut param1, &mut param2)` SMSG_CAST_FAILED (M)
- [ ] **#SPELLS-CAST.67** `Spell::send_pet_cast_result(result, ...)` SMSG_PET_CAST_FAILED (L)
- [ ] **#SPELLS-CAST.68** `Spell::send_interrupted(result)` SMSG_SPELL_FAILURE + SMSG_SPELL_FAILED_OTHER (M)
- [ ] **#SPELLS-CAST.69** `Spell::send_spell_interrupt_log(victim, spell_id)` SMSG_SPELL_INTERRUPT_LOG (L)
- [ ] **#SPELLS-CAST.70** `Spell::send_mount_result(result)` SMSG_MOUNT_RESULT (L)
- [ ] **#SPELLS-CAST.71** `Spell::send_resurrect_request(target)` SMSG_RESURRECT_REQUEST (L)
- [ ] **#SPELLS-CAST.72** `Spell::send_spell_execute_log()` + `get_execute_log_effect(effect)` + per-type `execute_log_effect_*` builders (M)

**Tick infrastructure (`SpellEvent`):**
- [ ] **#SPELLS-CAST.73** `SpellEvent` struct/trait que avanza `Spell` per tick — integrar con `WorldSession::tick` o un `EventQueue` similar a TC `BasicEvent` (H)
- [ ] **#SPELLS-CAST.74** `Spell::update(diff_ms)` state-machine (PREPARING → CASTING → FINISHED/DELAYED) (H)
- [ ] **#SPELLS-CAST.75** `SPELL_CHANNEL_UPDATE_INTERVAL = 1000ms` ticking + interrupt-on-movement/damage gates (M)

**Script DSL (~25 hooks):**
- [ ] **#SPELLS-CAST.76** `trait SpellScript` con default no-op methods + `inventory::submit!` registry indexed por spell_id (XL)
- [ ] **#SPELLS-CAST.77** `Spell::load_scripts()` + `m_loaded_scripts: Vec<Box<dyn SpellScript>>` (M)
- [ ] **#SPELLS-CAST.78** `call_script_*` helpers (~25 dispatch methods que iteran scripts cargados) (H)
- [ ] **#SPELLS-CAST.79** Migrar al menos los scripts de spells de talent/glyph más usados (clase-base) — itera per-clase (XL para cada una)

**Unit-side integration:**
- [ ] **#SPELLS-CAST.80** En `Unit`/`Player`: `m_current_spells: [Option<Arc<RwLock<Spell>>>; CURRENT_MAX_SPELL]` (slots GENERIC/MELEE/AUTO_REPEAT/CHANNELED) (M)
- [ ] **#SPELLS-CAST.81** `Unit::cast_spell` overloads (spell_id, target, custom_args, trigger_flags) → crea `Spell`, llama `prepare` (M)
- [ ] **#SPELLS-CAST.82** `Unit::set_current_cast_spell` / `finish_spell` / `interrupt_spell(slot, send_auto_repeat_error, complete)` (M)
- [ ] **#SPELLS-CAST.83** `Unit::has_unit_state(UNIT_STATE_CASTING)` integration with movement/damage interrupts (M)

**Routing (handlers):**
- [ ] **#SPELLS-CAST.84** Refactor `wow-world/src/handlers/spell.rs::handle_cast_spell` para construir `SpellCastTargets`, llamar `Player::cast_spell` (en lugar del current ack-only path) (M)
- [ ] **#SPELLS-CAST.85** Implementar `handle_pet_cast_spell` (CMSG_PET_CAST_SPELL) (M)
- [ ] **#SPELLS-CAST.86** Implementar `handle_use_item` (CMSG_USE_ITEM → spell con `cast_item_*`) (M)
- [ ] **#SPELLS-CAST.87** Implementar `handle_cancel_auto_repeat_spell` (L)
- [ ] **#SPELLS-CAST.88** Implementar `handle_update_projectile_position` (M)

---

## 10. Regression tests to write

Tests que demuestren que el comportamiento Rust = comportamiento C++ para invariantes clave.

- [ ] Test: `Spell::prepare` con instant spell + sin recursos → retorna `SPELL_FAILED_NO_POWER`, no consume cooldown, no envía SMSG_SPELL_START
- [ ] Test: `Spell::prepare` con cast time > 0 + recursos OK → state = PREPARING, envía SMSG_SPELL_START, GCD triggered
- [ ] Test: `Spell::cast` instant heal sobre party member → `SMSG_SPELLHEALLOG`, target HP +amount, threat update
- [ ] Test: `Spell::update` con caster moving + spell sin USABLE_WHILE_MOVING → `cancel(SPELL_FAILED_MOVING)`, SMSG_SPELL_FAILURE
- [ ] Test: Interrupt/damage durante cast → pushback timer atrasa `m_timer` con cap `delay_at_damage_count >= 2 → no more delays`
- [ ] Test: Channel spell (Mind Flay) — `SendChannelStart`, ticks cada 1s vía `update_channeled_target_list`, target invalid → tick lo quita
- [ ] Test: AOE area spell (Consecration) — `select_implicit_area_targets` retorna todos los enemigos en radio R, respeta `AOE_DAMAGE_TARGET_CAP = 20`
- [ ] Test: Chain spell (Chain Lightning) — `select_implicit_chain_targets` con N saltos, prioriza más cercano enemigo, decay damage por jump
- [ ] Test: Projectile spell (Frostbolt) — `m_delay_moment` calculado por distancia/speed, `handle_delayed` aplica hit con delay correcto
- [ ] Test: Reflect aura — `TargetInfo::PreprocessTarget` swappea target a caster cuando target tiene reflect, sigue resolution sobre caster
- [ ] Test: Miss/dodge/parry rolls — `SpellHitResult` distribución estadística matchea fórmulas TC para nivel diferencia attacker/defender
- [ ] Test: GCD por category — dos spells con misma `StartRecoveryCategory` no pueden castearse back-to-back; con `NO_GCD` flag sí
- [ ] Test: Triggered spell chain — chain length cap (`m_proc_chain_length >= 3` aborta)
- [ ] Test: `CheckCasterAuras` — silenced caster no puede castar magic schools, pacified no puede physical, stunned no puede nada (excepto USABLE_WHILE_STUNNED)
- [ ] Test: `CheckPower` — cast cuesta 100 mana + caster tiene 90 mana → SPELL_FAILED_NO_POWER; con 100 → succeed; con 99 + `MOD_POWER_COST_MASK` -2% → succeed
- [ ] Test: `CheckRuneCost` — DK Death Strike requiere 1 Frost + 1 Unholy + Death runes substitution rules
- [ ] Test: `CheckItems` — spell con reagent itemId=N + caster sin item → `SPELL_FAILED_REAGENTS` con `param1 = itemId`
- [ ] Test: Cancel cast mid-prepare (CMSG_CANCEL_CAST) → state = FINISHED, no power consumed, no cooldown set, GCD released
- [ ] Test: Cooldown persistence — `SpellHistory::SaveToDB` post-cast, `LoadFromDB` next session restaura cooldowns activos
- [ ] Test: Wire-format SMSG_SPELL_GO — hit list + miss list serialización byte-exact contra capture de TC
- [ ] Test: `SendSpellExecuteLog` con SpellLogEffect (PowerDrain/ExtraAttacks/CreateItem) — payload byte-exact

---

## 11. Notes / gotchas

- **`Spell` lifetime es la peor parte**. En C++ el `Spell` puede sobrevivir al `Unit` caster (si el caster muere mid-projectile-flight, el Spell sigue para resolver el daño en el target). Esto se gestiona con `m_originalCaster: Unit*` cached + `UpdatePointers()` que re-busca via GUID. En Rust esto debe ser `Weak<RwLock<Unit>>` con upgrade-or-skip; **NO** usar refs `&Unit` que no pueden sobrevivir scope.
- **`SpellEvent` doble ownership**: en C++ el `SpellEvent` está en `caster->m_Events`, pero `Spell::~Spell()` marca el event para auto-delete via `m_selfContainer`. Cuidado con drop ordering en Rust — usar `Arc<Mutex<Option<Spell>>>` para que el event y el caster ambos tengan handle.
- **`m_damage` y `m_healing` son scratch fields** que cada effect handler escribe y luego `TargetInfo::DoDamageAndTriggers` consume. Replicar como `Spell::damage_scratch: i32` que se resetea entre effects.
- **`unit_target`/`item_target`/`gameobj_target`/`m_corpse_target`/`destTarget`/`damage`/`variance`/`effect_handle_mode`/`effect_info` son mutables members** que `HandleEffects` setea antes del dispatch — los handlers individuales los leen como contexto. En Rust prefere `EffectContext { unit_target, item_target, …, damage, … }` pasado por argumento, no campos.
- **`MAX_SPELL_RANGE_TOLERANCE = 3.0` yards** se añade al rango max para latency. Sin esto, players con ping alto fallarán casts marginales. Replicar exactamente.
- **`AOE_DAMAGE_TARGET_CAP = 20`** es un límite hard para PvP (Knaak vs Knaak en BG). Aplicar después de selección, no durante.
- **`SPELL_INTERRUPT_NONPLAYER = 32747`** es el spell-ID magic que TC usa para registrar interrupts de NPC sin spell real. Es opaco — replicar como constante.
- **`SpellEffectHandleMode::LAUNCH` vs `HIT`**: muchos effects (e.g. SchoolDmg) corren en LAUNCH para que el daño esté pre-calculado al momento del SMSG_SPELL_GO (cliente lo necesita para el flying number), aunque el efecto real sólo se materializa en HIT_TARGET. **No invertir el orden** o el cliente verá numbers desincronizados.
- **`m_delayAtDamageCount >= 2`** es el cap de pushbacks que un cast puede sufrir. Pegarle a un caster mucho no infinitamente atrasa el cast — solo 2 pushbacks max. Replicar.
- **`Spell::IsTriggered() const` vs `_triggeredCastFlags != TRIGGERED_NONE`** — varios tests dentro del pipeline saltan validación si el spell es triggered. Si haces un trigger spell que NO debería saltar (e.g. eterno como bandage triggers), pasar el flag fragmentado correcto.
- **Channel pushback es DIFERENTE de cast pushback**: channel reduce ticks restantes proporcional al tiempo perdido, no atrasa el end. Misma función `Delayed`/`DelayedChannel` con paths distintos.
- **Reflect aura swap es PRE-effect-application** (en `PreprocessTarget`). No después. Si lo haces después, el spell ya pegó al target.
- **`SpellInfo::Effects` es slice estable** — punteros a `SpellEffectInfo const*` viven mientras viva el SpellInfo (que vive mientras viva SpellMgr, i.e. server lifetime). En Rust `&SpellEffectInfo` con lifetime `'static` o `Arc<SpellInfo>` indexing.
- **AuraScript vs SpellScript**: ambos son `m_loadedScripts` pero distintos types. Un mismo spell-id puede tener N SpellScript + N AuraScript (uno por cast vs uno por aura). El registry debe ir por type.
- **`CheckCast` returna ~250 enum values distintos**. No hay `Result<(), Error>` simple — es `enum SpellCastResult` con casos como `SPELL_FAILED_TARGET_AURASTATE`, `SPELL_FAILED_BAD_TARGETS`, `SPELL_FAILED_NEED_EXOTIC_AMMO`, `SPELL_FAILED_TARGET_IS_PLAYER_CONTROLLED`. **Mantener nombres idénticos** para compatibilidad con SQL `spell_required` overrides + scripts.
- Para 3.4.3 WoLK específicamente: la `m_misc` union (TalentId/GlyphSlot/SpecializationId/GarrFollower/...) tiene casos que **no aplican** a WoLK (Garrison es WoD, Heirloom upgrade es Legion). Filtrar al port.
- **No replicar `EffectIncreaseCurrencyCap`/`EffectGrantBattlePet*`/`EffectLearnTransmog*`/`EffectCreateConversation`/`EffectPlayScene*`/`EffectGiveExperience`/`EffectGiveRestedExperience`/`EffectSendChatMessage`/`EffectModifyCooldownsByCategory`/`EffectChangeBattlePetQuality`/`EffectUpgradeHeirloom`** — son retail-only. Reducción del scope de SpellEffectName a ~145 valores reales para WoLK.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Spell` | `struct Spell` (en `crates/wow-spell/src/cast/spell.rs`) | Sin herencia. `m_caster: Weak<RwLock<dyn WorldObject>>` |
| `Spell* m_currentSpells[CURRENT_MAX]` | `current_spells: [Option<Arc<RwLock<Spell>>>; 4]` en Unit | Slot por type |
| `class SpellEvent : public BasicEvent` | `struct SpellEvent { spell: Weak<RwLock<Spell>> }` con trait `Tick { fn update(&mut self, diff: u32) -> EventOutcome }` | `EventOutcome = Continue \| Drop` |
| `SpellCastTargets m_targets` | `targets: SpellCastTargets` | trivial copy |
| `SpellValue* m_spellValue` | `spell_value: SpellValue` (no Box, vive con Spell) | sin pointer indirection |
| `vector<TargetInfo> m_UniqueTargetInfo` | `unique_target_info: Vec<TargetInfo>` | — |
| `enum SpellState` | `enum SpellState { Null, Preparing, Casting, Finished, Idle, Delayed }` | — |
| `enum SpellCastFlags` | `bitflags! { struct SpellCastFlags: u32 { … } }` | bitflags crate |
| `enum TriggerCastFlags` | `bitflags! { struct TriggerCastFlags: u32 { … } }` | — |
| `enum SpellCastResult` | `#[repr(u32)] enum SpellCastResult { Ok=0, NotKnown=2, … }` | mantener IDs |
| `enum SpellEffectHandleMode` | `enum SpellEffectHandleMode { Launch, LaunchTarget, Hit, HitTarget }` | — |
| `Spell::prepare(SpellCastTargets const&, AuraEffect const*) -> SpellCastResult` | `fn prepare(&mut self, targets: SpellCastTargets, triggered_by_aura: Option<&AuraEffect>) -> SpellCastResult` | retorno explícito (en C++ es void + miembro `SpellCastResult m_castResult`) |
| `void Spell::cast(bool skipCheck)` | `fn cast(&mut self, skip_check: bool)` | — |
| `void Spell::update(uint32 difftime)` | `fn update(&mut self, diff_ms: u32)` | — |
| `void Spell::finish(SpellCastResult)` | `fn finish(&mut self, result: SpellCastResult)` | — |
| `void Spell::HandleEffects(...)` | `fn handle_effects(&mut self, ctx: &mut EffectContext, effect: &SpellEffectInfo, mode: SpellEffectHandleMode)` | `EffectContext { unit_target, item_target, … }` para evitar member-state |
| `Spell::SelectImplicit*Targets(...)` (13 selectores) | métodos individuales `select_implicit_*_targets` | misma firma, returning `()` y mutando `m_unique_target_info` |
| `Trinity::WorldObjectSpellTargetCheck` (functor) | `trait SpellTargetCheck { fn check(&self, target: &dyn WorldObject) -> bool }` con structs Nearby/Area/Cone/Traj/Line | trait objects + composition |
| `SpellEffectHandlerFn = void (Spell::*)()` | `enum SpellEffect { SchoolDamage, Heal, … }` + `fn dispatch(spell: &mut Spell, ctx: &mut EffectContext, effect: SpellEffect)` con match | sin function pointer table — match exhaustivo |
| `class SpellScript` (DSL) | `trait SpellScript { fn on_precast(&mut self, spell: &mut Spell) {} ... }` registrado vía `inventory::submit!` por spell_id | default no-op methods |
| `m_loadedScripts: vector<SpellScript*>` | `loaded_scripts: SmallVec<[Box<dyn SpellScript>; 2]>` | inline storage 2 scripts (común) |
| `ObjectGuid m_originalCasterGUID` + `Unit* m_originalCaster` | `original_caster_guid: ObjectGuid` + `original_caster_cache: Weak<RwLock<Unit>>` que se re-resolve en `update_pointers` | weak ref |
| `WorldPacket SendSpellStart` | `fn send_spell_start(&self) -> Vec<u8>` que el caller broadcastea via `Map::send_message_in_range` | retornar Vec<u8>, no SendPacket inline |
| `m_Events.AddEvent(new SpellEvent(this), m_Events.CalculateTime(...))` | `caster.events.push(SpellEvent { spell: weak.clone() }, deadline_ms)` | event queue por unit |
| `Unit::CastSpell` overloads | `Unit::cast_spell` overloads → builder pattern `CastSpellArgs::new().with_target(t).with_triggered(flags).cast()` | reduce overload explosion |
| `std::any m_customArg` | `custom_arg: Option<Box<dyn Any + Send + Sync>>` | type-erased payload |
| `SpellInfo const* m_spellInfo` | `spell_info: Arc<SpellInfo>` | shared ref |
| `m_powerCost: vector<SpellPowerCost>` | `power_cost: SmallVec<[SpellPowerCost; 2]>` | dual-type (mana+health) común |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Spells/Spell.h` (994 lines) and `Spell.cpp` (9,303 lines, **77 `Spell::` member-function definitions** at top level — verified via `grep -nE "^(SpellCastResult|void|bool|uint64|Spell::Spell|Spell::~Spell) Spell::"`), plus `SpellCastRequest.h` (43 lines), against the Rust workspace at `/home/server/rustycore/crates/`.

**Empty-crate finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` measures **exactly 0 lines** (verified via `wc -l`). Within the cast sub-module specifically, the Rust workspace has **zero implementation** of the central `Spell` struct, `SpellEvent` ticker, `SpellValue`, `SpellCastTargets` proper (only a partial wire reader exists in `wow-packet`), `SpellState`, `SpellEffectHandleMode`, `SpellCastFlags`, `SpellCastFlagsEx`, `TriggerCastFlags`, `SpellCastSource`, `SpellHealPredictionType`, `SpellRangeFlag`, `Trinity::WorldObjectSpell{Target,Nearby,Area,Cone,Traj,Line}TargetCheck` functor analogues, `Spell::TargetInfo`/`GOTargetInfo`/`ItemTargetInfo`/`CorpseTargetInfo` nested structs, `Spell::HitTriggerSpell`. The 10,340 lines of C++ cast machinery map to **zero lines** of Rust engine.

**What exists outside the empty crate.** A partial CMSG_CAST_SPELL handler in `crates/wow-world/src/handlers/spell.rs` (288 lines) — registers 3 inventory entries (`CastSpell`, `CancelCast`, `CancelChannelling`), parses `CastSpellRequest` via `wow_packet::ClientPacket`, validates `known_spells` set membership and a bare cooldown HashMap, and either (a) emits `SMSG_SPELL_START` with a hardcoded `SpellCastVisual` and stores `active_spell_cast: Option<ActiveSpellCast>` in the session for the per-tick countdown, or (b) emits `SMSG_CAST_FAILED` with `reason = 2` (NotKnown). Wire packets in `crates/wow-packet/src/packets/spell.rs` (466 lines) define `CastSpellRequest::read`, `SpellTargetData`, `SpellCastVisual`, `SpellStartPkt::write`, `CastFailed::write`, `SpellCooldownPkt::write`, plus a few visualizer scaffolds. `crates/wow-data/src/spell.rs` exposes a stub `SpellInfo` with only `cast_time_ms`/`recovery_time_ms`/`effective_cooldown_ms`/`has_cast_time` — none of the 25+ fields the cast pipeline reads (`Reagent[]`, `Totem[]`, `RangeEntry`, `PowerCost`, `Effects[]`, `InterruptFlags`, `AuraInterruptFlags`, `ChannelInterruptFlags`, `RuneCost`, `Equipped*Class`/`Subclass`, `RequiresSpellFocus`, `Stances`, `StancesNot`, `SchoolMask`, `Mechanic`, `DispelType`, `SpellFamilyName`, `SpellFamilyFlags[3]`, `Speed`, `LaunchDelay`, `MinDuration`, `MaxAffectedTargets`, `Attributes[15]`, `AttributesEx[14]`).

**Pipeline phases implemented.** **0 of 6.** No `prepare`, no `cast`/`_cast`, no `handle_immediate`, no `handle_delayed`, no `_handle_immediate_phase`/`_handle_finish_phase`, no `update`, no `finish`, no `cancel`. The Rust handler does the cast-time countdown in the session tick path, then "executes" the spell by logging — no orchestration of effect dispatch, no state machine, no SPELL_STATE_* tracking.

**Validation (`CheckCast` family) implemented.** **0 of 12 functions.** No `CheckCast` (the ~1050-line XL function), no `CheckPetCast`, no `CheckPower` (despite `m_powerCost` not existing either), no `CheckRuneCost`, no `CheckItems`, no `CheckRange`, no `GetMinMaxRange`, no `CheckCasterAuras`, no `CheckSpellCancels{Charm,Stun,Silence,Pacify,Fear,Confuse,NoActions,AuraEffect}`, no `CheckArenaAndRatedBattlegroundCastRules`, no `CheckMovement`, no `CheckEffectTarget`/`CheckSrc`/`CheckDst`. The only validation is `known_spells.contains(&spell_id)` and a HashMap cooldown check. Neither resource consumption (mana/runes/reagents/totems), nor range/LOS, nor target validity (alive/dead/positive/negative/faction), nor caster state (silenced/pacified/stunned/charmed/feared/confused/mounted/shapeshifted/on-vehicle), nor zone/area/PvP rules are enforced.

**Target enumeration implemented.** **0 of 13 selectors.** No `SelectImplicitChannelTargets`, `SelectImplicitNearbyTargets`, `SelectImplicitConeTargets`, `SelectImplicitAreaTargets`, `SelectImplicitCasterDestTargets`, `SelectImplicitTargetDestTargets`, `SelectImplicitDestDestTargets`, `SelectImplicitCasterObjectTargets`, `SelectImplicitTargetObjectTargets`, `SelectImplicitChainTargets`, `SelectImplicitTrajTargets`, `SelectImplicitLineTargets`, `SelectEffectTypeImplicitTargets`. No `SelectExplicitTargets`/`SelectSpellTargets` orchestrator. No `WorldObjectSpellTargetCheck` trait, no Nearby/Area/Cone/Traj/Line subclasses, no `SearchNearbyTarget`/`SearchAreaTargets`/`SearchChainTargets` helpers. AOE/cone/chain/trajectory spells cannot identify their targets.

**Hit resolution implemented.** **0.** No `TargetInfo::PreprocessTarget` (reflect handling, DR group calculation, AuraDuration pre-compute), no `DoTargetSpellHit` per-target dispatch, no `DoDamageAndTriggers` post-effect aggregation, no `DoSpellEffectHit`, no `DoTriggersOnSpellHit`, no `PreprocessSpellLaunch`/`PreprocessSpellHit`. No miss/dodge/parry/block/resist roll (`SpellHitResult` does not exist on `Unit`). No reflect mechanic. No diminishing returns group (`DRGroup` enum + tracking).

**Channel system.** Zero. `SendChannelStart`/`SendChannelUpdate`/`UpdateChanneledTargetList`, `SPELL_CHANNEL_UPDATE_INTERVAL = 1000ms`, channel target list pruning, channel pushback (`DelayedChannel`), `Spell::IsChannelActive` — none exist. Mind Flay, Penance, Shadowfury cast, all bind-channel mechanics inert.

**Delayed projectile system.** Zero. `m_delayStart`/`m_delayMoment`, `CalculateDelayMomentForDst`, `RecalculateDelayMomentForDst`, `UpdateDelayMomentForDst`, `UpdateDelayMomentForUnitTarget`, `m_delayAtDamageCount`/`IsDelayableNoMore`, `handle_delayed` per-target landing, `Spell::Delayed` (pushback) — none. Frostbolt, Pyroblast, Hunter's Mark, etc., resolve instantly without travel time.

**GCD system.** Zero proper. `TriggerGlobalCooldown`, `HasGlobalCooldown`, `CancelGlobalCooldown` — none. `SpellHistory` does not exist as a struct. The current `cooldowns: HashMap<SpellId, Instant>` does not differentiate GCD from spell-CD from category-CD; spells with `StartRecoveryCategory` shared (e.g. potions, blessings) are not coordinated; spells with `CAST_FLAG_NO_GCD` (vehicle, charm) are not exempted.

**Power consumption.** Zero. `TakePower`, `TakeRunePower`, `TakeReagents`, `TakeCastItem` — none. The power cost is never deducted from caster — players can spam infinite spells without losing mana. Rune state is never tracked. Reagents are never consumed. Cast-from-item itemcount is never decremented (one-charge items keep firing).

**Triggered spell chain.** Zero. `m_procChainLength` cap (preventing infinite recursive triggers), `_triggeredCastFlags` (TriggerCastFlags bitmask), `m_hitTriggerSpells` post-hit dispatch, `PrepareTriggersExecutedOnHit`, `CanExecuteTriggersOnHit` — none. Procs that cast trigger-spells (Lightning Shield → discharge, Vampiric Touch → mana refund, every weapon enchant) cannot fire.

**SpellScript DSL.** Zero. `LoadScripts`, `m_loadedScripts: vector<SpellScript*>`, the ~25 `CallScript*Handlers` (`OnPrecast`, `BeforeCast`, `OnCast`, `AfterCast`, `CheckCast`, `CalcCastTime`, `BeforeHit`, `OnHit`, `AfterHit`, `EffectHit`/`Launch`/`Apply`/`Remove`, `CalcCritChance`, `CalcDamage`, `CalcHealing`, `ObjectAreaTargetSelect`, `ObjectTargetSelect`, `DestinationTargetSelect`, `SuccessfulDispel`) — none exist. Boss mechanics, talent overrides, glyph effects, and most class-specific spell scripting (~3000+ scripts in `worldserver-scripts/`) cannot be migrated until the trait + dispatch infrastructure exists.

**`SpellEvent` (event-loop ticker).** Zero. The C++ `BasicEvent`/`EventProcessor` machinery on `WorldObject::m_Events` that ticks `Spell::update(diff)` at deadline + arbitrary delay does not exist. The current path piggy-backs on `WorldSession::tick` (per-session polling), which (a) does not work for non-player casters (creatures, GameObjects, DynamicObjects), and (b) cannot schedule arbitrary future events (delayed projectile landings, channel ticks).

**Wire packet surface.** **3 of ~25 implemented.** Only `SMSG_SPELL_START`, `SMSG_CAST_FAILED`, and `SMSG_SPELL_COOLDOWN` writers exist. Missing: `SMSG_SPELL_GO` (the canonical hit/miss list packet — the current path skips it entirely), `SMSG_SPELL_FAILURE` (broadcast interrupt), `SMSG_SPELL_FAILED_OTHER`, `SMSG_SPELL_DELAYED` (pushback), `SMSG_CHANNEL_START`, `SMSG_CHANNEL_UPDATE`, `SMSG_SPELL_INTERRUPT_LOG`, `SMSG_SPELL_EXECUTE_LOG`, `SMSG_RESURRECT_REQUEST`, `SMSG_MOUNT_RESULT`, `SMSG_PET_CAST_FAILED`. Without `SMSG_SPELL_GO`, the client does not see the projectile fire animation or the hit/miss feedback — the visual is broken even when the cast "succeeded."

**Unit-side integration.** Zero. `m_currentSpells[CURRENT_*]` (slot per type GENERIC/MELEE/AUTO_REPEAT/CHANNELED), `Unit::SetCurrentCastSpell`/`FinishSpell`/`InterruptSpell(slot, send_auto_repeat_error, complete)`, `Unit::HasUnitState(UNIT_STATE_CASTING)` proper integration with movement code, `Unit::CastSpell` overloads (~15 in C++ for spell_id/target/args/trigger_flags/custom_args combos) — none exist. The Rust `WorldSession` has neither a `current_spells` array field nor any concept of "the unit is currently casting X." The casting bar in client is decoupled from server-side state.

**Pet/charm/vehicle cast routing.** Zero. `HandlePetCastSpellOpcode`, `CheckPetCast`, `Vehicle::Reset` casts, charm-controlled `CastSpell` paths — none implemented.

**`SpellInfo` data layer.** Critically incomplete (cross-ref [`spells-info.md`](./spells-info.md)). Even if `Spell` existed, the data struct it would read is a 4-field stub. `CheckCast` cannot run without `Reagent[]`/`Totem[]`/`Stances`/`AttributesEx*`/`Equipped*`/`PreventionType`/`InterruptFlags`. `SelectImplicitAreaTargets` cannot run without `RadiusEntry`/`MaxRadiusEntry`. `CheckPower` cannot run without the `SpellPowerEntry` chain. `CheckRange` cannot run without `RangeEntry`. The cast and info sub-modules are blocked on each other.

**Worst divergence.** The cast sub-module is the **orchestrator** of the entire spell engine. With 0 lines of `Spell` struct and 0 of 77 method definitions, **no spell in the game executes server-side semantics** — the engine is a wire-format echo: parse `CMSG_CAST_SPELL`, schedule a fake countdown, send `SMSG_SPELL_START` with hardcoded visual, log "executed" with no consequence. Players can cast unknown spells as easily as known ones (the `known_spells` HashSet is the only gate), pay no mana, ignore range/LOS/silence, hit through reflect, and trigger no procs. AOE casts hit nobody because no target enumeration runs. Boss mechanics — every encounter scripted around a `Spell` triggering, channelling, interrupting, chaining — are dormant. The §9 task list (#SPELLS-CAST.1 → #SPELLS-CAST.88) reflects ground-up greenfield work — equivalent in scope to porting all of `Spell.cpp` (~9.3k C++ lines) to idiomatic Rust with proper ownership (`Arc<RwLock<Spell>>` + `Weak` back-refs to dodge the Spell-outliving-caster problem), an event-loop infrastructure (`SpellEvent` analog of `BasicEvent`), and the SpellScript DSL trait. Multiple individual tasks are XL (`Spell` struct itself, `CheckCast`, `TargetInfo`, the trait registry, the script DSL); the full sub-module is the largest individual blocker for shipping any spell-driven gameplay.
