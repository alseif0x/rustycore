# Migration: Spells / Effects (sub-module)

> **C++ canonical path:** `src/server/game/Spells/SpellEffects.cpp`
> **Rust target crate(s):** `crates/wow-spell/` (módulo `effects`), `crates/wow-spell/src/effects/dispatch.rs`
> **Layer:** L5 sub-module of `spells.md`
> **Status:** ⚠️ represented-partial — `wow-spell` sigue sin motor propio, pero hay efectos representados en `wow-world::WorldSession`
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing — see §13)
> **Last updated:** 2026-06-15

> **DRIFT NOTE (2026-06-15):** la auditoría histórica de §13 describe el estado inicial del módulo
> antes de los efectos representados en `wow-world`. No debe leerse como inventario actual. Para
> estado vivo, usar esta checklist y `docs/migration/current-session-handoff.md`.

> **Parent doc:** [`spells.md`](./spells.md) — overview del motor entero (Spell + SpellInfo + SpellMgr + SpellHistory + Auras combinados, ~44k líneas C++).
> **Related sub-docs:** [`spells-aura.md`](./spells-aura.md), [`spells-cast.md`](./spells-cast.md).
> **Cross-link:** la mayoría de effects opera sobre `Unit`/`Player` (DealDamage, HealBySpell, ModifyPower, TeleportTo, AddAura, …) — ver [`entities-unit.md`](./entities-unit.md) para los entry-points consumidos.

---

## 1. Purpose

El sub-módulo Effects implementa los ~151 **handlers concretos** que ejecutan la consecuencia de cada `SpellEffect` cuando un cast resuelve. Son las funciones que efectivamente "hacen pasar cosas en el mundo": daño, heal, summon, teleport, dispel, knockback, charge, jump, create item, learn spell, enchant, taunt, interrupt, resurrect, leap, modify cooldown, etc. Cada `SpellEffectInfo` dentro de un `SpellInfo` lleva un `Effect: SpellEffectName`, y al castear se invoca el handler correspondiente vía la dispatch table `SpellEffectHandlers[TOTAL_SPELL_EFFECTS]` (función pointer table de 151 entradas).

A diferencia del `Spell` runtime (en `spells-cast.md`) que orquesta el **pipeline** (prepare→cast→finish, target enumeration, miss/hit, packets), este sub-módulo es **pure execution**: cada handler asume que el Spell ya validó cast y enumeró targets, recibe `(unit_target, item_target, gameobj_target, corpse_target, spellEffectInfo, mode)` y aplica el efecto canónico — leyendo `BasePoints`, `MiscValue`, `MiscValueB`, `TriggerSpell`, `RadiusEntry`, `Mechanic` del `SpellEffectInfo`. Los handlers más comunes (`SchoolDamage`, `Heal`, `ApplyAura`) son ~50-200 líneas; los más exóticos (`SummonType`, `OpenLock`, `EnchantItemPerm`, `WeaponDmg`) llegan a 200-400 líneas con casos especiales por spell family.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Spells/SpellEffects.cpp` | 5956 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Spells/SpellEffects.cpp` | 5,956 | **Único archivo del sub-módulo.** Contiene los 151 (`Effect*`) handlers, casi todos como métodos `void Spell::EffectXxx()`. La mayoría son ~30-200 líneas; los XL son `EffectSummonType` (~218 lines), `EffectSchoolDMG` (~57 + auxiliares), `EffectWeaponDmg` (~126 lines), `EffectScriptEffect` (~150 lines), `EffectOpenLock` (~115 lines). Al inicio (~líneas 86-238) está la `SpellEffectHandlers[TOTAL_SPELL_EFFECTS]` dispatch table (function pointer array indexed by `SpellEffectName`) |
| `src/server/game/Spells/SpellDefines.h` | 549 (compartido con resto del módulo Spells) | `enum SpellEffectName` (~151 valores: SCHOOL_DAMAGE=2, DUMMY=3, TELEPORT_UNITS=5, APPLY_AURA=6, ENVIRONMENTAL_DAMAGE=7, POWER_DRAIN=8, HEALTH_LEECH=9, HEAL=10, BIND=11, QUEST_COMPLETE=16, WEAPON_DAMAGE_NOSCHOOL=17, RESURRECT=18, ADD_EXTRA_ATTACKS=19, CREATE_ITEM=24, ENERGIZE=30, SUMMON=28 (renamed), HEAL_PCT=33, DISPEL=38, LANGUAGE=39, DUAL_WIELD=40, SUMMON_PLAYER=44, ACTIVATE_OBJECT=50, JUMP=64, JUMP_DEST=65, …, MAX hasta 270+ con muchos NULL/Unused — `TOTAL_SPELL_EFFECTS` bookend) |
| `src/server/game/Spells/Spell.h` | 994 (compartido) | Declaraciones de los métodos `Effect*` dentro de `class Spell` (líneas ~291-422 son la lista de declaraciones públicas/privadas) — uno por SpellEffectName con código real |
| `src/server/game/Spells/SpellInfo.h` | 625 (compartido) | `class SpellEffectInfo` — el dato estático que cada handler consulta: `Effect: SpellEffectName`, `BasePoints`, `RealPointsPerLevel`, `PointsPerComboPoint`, `DicePerLevel`, `BaseDice`, `DamageMultiplier`, `MiscValue`, `MiscValueB`, `TriggerSpell`, `ImplicitTarget[2]: SpellImplicitTargetInfo`, `Mechanic`, `RadiusEntry`, `MaxRadiusEntry`, `ChainTargets`, `ChainAmplitude`, `ItemType`, `RealPointsPerLevel`, `EffectAttributes` |

**Total Effects sub-module:** ~5,956 líneas (un archivo monolítico, ~150 funciones).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SpellEffectName` | enum uint32 | ~151 variantes — el ID que indexa la dispatch table. Bookend `TOTAL_SPELL_EFFECTS` |
| `SpellEffectHandleMode` | enum | `SPELL_EFFECT_HANDLE_LAUNCH` (cast finish, before target hit), `SPELL_EFFECT_HANDLE_LAUNCH_TARGET`, `SPELL_EFFECT_HANDLE_HIT` (immediate when projectile lands), `SPELL_EFFECT_HANDLE_HIT_TARGET` (per-target hit) — algunos efectos corren en LAUNCH (e.g. `EffectSchoolDMG` para tener daño calculado pre-impact), otros en HIT |
| `SpellEffectHandlerFn` | typedef | `void (Spell::*)()` — punteros a métodos para la dispatch table |
| `SpellEffectHandlers[TOTAL_SPELL_EFFECTS]` | static array | La dispatch table de 151 entries indexed por SpellEffectName. Inicializada en SpellEffects.cpp top |
| `Targets` (m_targets) | struct | `SpellCastTargets` — leído por handlers para conocer item_target, dst position, src position |
| `SpellEffIndex` | enum | EFFECT_0..EFFECT_31 (hasta 32 effect slots por spell, casi siempre solo 3) |
| `damage` | int32 (member) | Damage post-rolls que el handler escribe; el Spell pipeline lo combina al `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` |
| `effectInfo` | SpellEffectInfo const* | Datos estáticos del effect actual (read by handler) |
| `unitTarget` / `itemTarget` / `gameObjTarget` / `corpseTarget` / `destTarget` | members | Los 5 tipos de target que un effect puede tocar |

### Lista canónica de handlers (151 SPELL_EFFECT_*) — agrupados por dominio

**Damage (offensive):**
- `EffectSchoolDMG` (2) — daño directo magic con SpellDamageBonusDone, school separation
- `EffectEnvironmentalDMG` (7) — daño no-evitable (lava, fall, drown)
- `EffectPowerDrain` (8) — drena power del target (SP, mana, energy)
- `EffectHealthLeech` (9) — daño + heal al caster
- `EffectWeaponDmg` (17, 121, 122, 123) — variantes de weapon-based: NOSCHOOL, NORMALIZED, +%, NoDamage
- `EffectAddExtraAttacks` (19) — extra melee swings
- `EffectInstaKill` (1) — kill instantáneo

**Heal:**
- `EffectHeal` (10) — heal directo con SpellHealingBonusDone
- `EffectHealPct` (33) — heal % de max HP
- `EffectHealMechanical` (35) — heal a mechanical pets only
- `EffectHealMaxHealth` (135) — set HP a maxHealth (raise to full)

**Power:**
- `EffectEnergize` (30) — restore power flat
- `EffectEnergizePct` (137) — restore power %
- `EffectPowerBurn` (53) — drain + damage from drained amount

**Aura:**
- `EffectApplyAura` (6) — `Aura::TryRefreshStackOrCreate` (ver `spells-aura.md`)
- `EffectApplyAreaAura*` (35 = friend, 75 = enemy, 64 = pet, 76 = owner, 79 = party, 89 = raid) — 6 variantes by faction filter
- `EffectPersistentAA` (27) — DynamicObject ground area aura

**Movement:**
- `EffectTeleportUnits` (5) — teleport via spell_target_position lookup
- `EffectTeleportUnitsWithVisualLoadingScreen` (272) — variant con loading screen
- `EffectTeleUnitsFaceCaster` (113) — teleport y rotate to face
- `EffectJump` (64) — jump to unit target
- `EffectJumpDest` (145) — jump to dest position
- `EffectLeap` (62) — Blink-style leap forward
- `EffectLeapBack` (147) — Disengage-style leap back
- `EffectKnockBack` (98) — knockback to direction
- `EffectKnockBackDest` (96) — knockback to dest
- `EffectPullTowardsDest` / `EffectPull` — pull
- `EffectCharge` (109) — Charge ability (close gap + stun)
- `EffectChargeDest` (188) — Charge to position
- `EffectJumpCharge` (236) — jump charge mechanic
- `EffectMomentum` — momentum-based push

**Summon:**
- `EffectSummonType` (28) — **el handler XL** (~218 lines). Despacha por `SpellEffectInfo::MiscValueB` (Properties.dbc) a uno de: SUMMON_TYPE_NONE, PET, GUARDIAN, MINION, TOTEM, MINIPET, VEHICLE_FORCED, VEHICLE_FACING, … cada uno con su `SummonsList::Summon*` call
- `EffectSummonPet` (75) — summon pre-existing pet (paladin) o guardian
- `EffectSummonObject` / `EffectSummonObjectWild` (76, 50) — summon GameObject (Sappers, etc.)
- `EffectSummonChangeItem` (155) — replace item with summon
- `EffectSummonPlayer` (109) — summon player to caster (warlock ritual)

**Resurrect:**
- `EffectResurrect` (18) — resurrect dead player target
- `EffectResurrectNew` (94) — resurrect with selectable HP/Mana
- `EffectSelfResurrect` (151) — self resurrect
- `EffectResurrectPet` (113) — pet resurrect

**Dispel / Interrupt:**
- `EffectDispel` (38) — quita auras según DispelMask + chance
- `EffectStealBeneficialBuff` (148) — Spellsteal (Mage)
- `EffectInterruptCast` (32) — cancel target's cast + lock school
- `EffectSurvey` — scanning effect
- `EffectDispelMechanic` (96) — dispel by Mechanic, no DispelType

**Movement modifiers (apply/clear status):**
- `EffectStuck` (140) — unstuck command
- `EffectSanctuary` (50) — drop combat + remove threat
- `EffectDualWield` (51) — grant dual wield proficiency (passive)

**Crowd Control / Status:**
- `EffectTaunt` (152) — force target to attack caster
- `EffectThreat` (24) — modify threat
- `EffectScriptEffect` (77) — **wildcard** — script-only effect (~150 lines — giant switch/case por spellId con boss mechanics, scripted abilities, special-case logic)
- `EffectDummy` (3) — handler-less effect, all logic in scripts
- `EffectDistract` (175) — turn target's facing
- `EffectAddComboPoints` (60) — add combo points (rogue/druid)
- `EffectModifyAuraStacks` (313) — adjust stack count
- `EffectModifyCooldown` / `EffectModifyCooldowns` / `EffectModifyCooldownsByCategory` — adjust spell cooldowns
- `EffectModifySpellCharges` (354) — adjust charges

**Item / Inventory:**
- `EffectCreateItem` (24) — create item in bag
- `EffectCreateItem2` (157) — create item from spec (random suffix)
- `EffectCreateRandomItem` (251) — random loot
- `EffectFeedPet` (101) — pet feed (item destroy + happiness)
- `EffectEnchantItemPerm` (53) — permanent enchant
- `EffectEnchantItemTmp` (54) — temporary enchant
- `EffectEnchantItemPrismatic` (157) — prismatic socket
- `EffectEnchantHeldItem` (115) — quick enchant
- `EffectDisEnchant` (12) — disenchant item
- `EffectMillItem` / `EffectProspecting` — milling/prospecting profession
- `EffectAddItemEnchant` — add enchant via spell

**Quest / Profession:**
- `EffectQuestComplete` (16) — auto-complete quest
- `EffectQuestStart` (134) — start quest from spell
- `EffectQuestRedirect` — redirect quest
- `EffectLearnSpell` (26) — Player learns spell
- `EffectUnlearnSpecialization` (52) — unlearn spec
- `EffectLearnPetSpell` (66) — pet learn
- `EffectLearnSkill` (160) — give skill at level
- `EffectTradeSkill` (47) — open profession trainer/level up
- `EffectProficiency` (45) — grant weapon proficiency
- `EffectUntrainTalents` (88) — clear talents

**Open Lock / GameObject:**
- `EffectOpenLock` (50) — pick lock / chest open / mining / herbalism / fishing
- `EffectOpenLockItem` (50 variant) — open item (lockbox)
- `EffectActivateObject` (107) — activate GO
- `EffectSendEvent` (61) — fire SmartScript event
- `EffectStuck` — port out

**Pet / Charm:**
- `EffectTameCreature` (77) — tame to hunter pet
- `EffectDismissPet` (96) — dismiss
- `EffectAddFarsight` (16) — farsight totem
- `EffectInebriate` (142) — increase drunk state

**Combat mechanics:**
- `EffectParry` (87) — grant parry ability passive
- `EffectBlock` (88) — grant block passive
- `EffectReputation` (103) — modify faction reputation
- `EffectDuel` (52) — challenge to duel

**Glyph / Talent:**
- `EffectApplyGlyph` (170) — apply glyph
- `EffectChangeRaidMarker` — set raid marker

**Misc / Effects:**
- `EffectForceCast` (141) — force target to cast a spell
- `EffectTriggerSpell` (64) — server triggers a sub-spell (no GCD, no power)
- `EffectTriggerMissileSpell` (133) — trigger as missile
- `EffectTriggerRitualOfSummoning` (113) — warlock summoning ritual
- `EffectPlayMovie` (162) — show cinematic
- `EffectPlayScene` (243) — play scene file
- `EffectSendChatMessage` — broadcast chat message
- `EffectGiveHonor` (240) — award honor
- `EffectGrantBattlePetExperience` — battle pet xp
- `EffectGameobjectDamage` — damage GO
- `EffectGameObjectRepair` — repair GO
- `EffectGameobjectSetDestructionState` — change GO state
- `EffectForceDeselect` — clear target on client
- `EffectPickPocket` (102) — rogue pickpocket
- `EffectInebriate` (142) — drunken
- `EffectModifyThreatPercent` — % threat mod

**NULL / Unused:** ~50 entradas a `EffectNULL` o `EffectUnused` (effects deprecated/never-implemented).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Spell::HandleEffects(Unit*, Item*, GameObject*, Corpse*, SpellEffectInfo const&, SpellEffectHandleMode)` | Despacha al handler correcto: `(this->*SpellEffectHandlers[effectInfo.Effect])()`. Set `unitTarget`/`itemTarget`/`gameObjTarget`/`destTarget` antes del call | One of 151 `Effect*` |
| `Spell::HandleLaunchPhase()` | Run effects con mode=LAUNCH (e.g. compute damage during cast, before missile travel) | `HandleEffects(LAUNCH)` |
| `Spell::HandleImmediatePhase()` | Run effects con mode=HIT inmediato (instant spells) | `HandleEffects(HIT)` |
| `Spell::DoAllEffectOnTarget(TargetInfo*)` | Per-target effect application — itera effects bitmask, calls HandleEffects mode=HIT_TARGET | `HandleEffects` |
| `Spell::DoAllEffectOnLaunchTarget(TargetInfo*)` | Per-target effects en LAUNCH | `HandleEffects` |
| `Spell::EffectSchoolDMG(SpellEffIndex)` | Daño directo magic: BasePoints + bonus done/taken, school resist roll, crit roll, write `m_damage` | `Unit::SpellDamageBonusDone`, `Unit::DealSpellDamage`, `SendSpellNonMeleeDamageLog` |
| `Spell::EffectHeal(SpellEffIndex)` | Heal directo: bonus done/taken, crit, overheal log | `Unit::SpellHealingBonusDone`, `Unit::HealBySpell` |
| `Spell::EffectApplyAura(SpellEffIndex)` | Crea/refresh `Aura` en target via `Aura::TryRefreshStackOrCreate` | `Aura::TryRefreshStackOrCreate` |
| `Spell::EffectTeleportUnits(SpellEffIndex)` | Teleport via lookup `SpellTargetPosition` (DB table by spell+effect index) | `Player::TeleportTo`, `SpellMgr::GetSpellTargetPosition` |
| `Spell::EffectSummonType(SpellEffIndex)` | Summon discriminator por `SummonProperties.dbc.Control`/`Title`. Crea Pet, Guardian, Totem, Vehicle, Companion | `Map::SummonCreature`, `Pet::CreateBaseAtTame`, `Player::SummonPet` |
| `Spell::EffectDispel(SpellEffIndex)` | Quita auras del target matching DispelMask con chance roll, send SMSG_SPELL_DISPELL_LOG | `Unit::RemoveAurasByDispel`, `SendSpellDispelLog` |
| `Spell::EffectInterruptCast(SpellEffIndex)` | Cancel current cast on target, lock school for `BasePoints` ms | `Unit::InterruptSpell`, `SpellHistory::LockSpellSchool` |
| `Spell::EffectKnockBack(SpellEffIndex)` | Apply knockback motion: speedXY from BasePoints, speedZ from MiscValue | `MotionMaster::MoveKnockbackFrom`, `Player::SendKnockBack` |
| `Spell::EffectJump(SpellEffIndex)` / `EffectJumpDest` | Jump to target (target as anchor) or to dst position | `MotionMaster::MoveJump`, `Player::SendJump` |
| `Spell::EffectCharge(SpellEffIndex)` / `EffectChargeDest` | Charge: PathGenerator + MoveCharge + apply stun + enter combat | `PathGenerator::CalculatePath`, `MotionMaster::MoveCharge` |
| `Spell::EffectEnergize(SpellEffIndex)` | Power gain (mana/rage/energy/runic). MiscValue = power type | `Unit::ModifyPower`, `SendSpellEnergizeLog` |
| `Spell::EffectPowerDrain(SpellEffIndex)` | Drain power, optionally apply damage = drained * SP coef | `Unit::ModifyPower`, `Unit::DealDamage` |
| `Spell::EffectCreateItem(SpellEffIndex)` / `EffectCreateItem2` | Crea item en inventory; profession crafts | `Player::StoreNewItem`, `Player::SendNewItem` |
| `Spell::EffectEnchantItemPerm(SpellEffIndex)` / `EffectEnchantItemTmp` / `Prismatic` | Apply enchant: lookup `SpellItemEnchantmentEntry`, set on item slot | `Item::SetEnchantment`, `Player::ApplyEnchantment` |
| `Spell::EffectScriptEffect(SpellEffIndex)` | **El wildcard handler** (~150 lines). Giant switch sobre `m_spellInfo->Id` con casos especiales por spell — fallback `ScriptMgr::OnSpellEffectScript` para scripts | Multiple — switch sobre spellId |
| `Spell::EffectDummy(SpellEffIndex)` | Otro wildcard — pure script-driven; SpellMgr::CanSpellTriggerProcOnEvent dispatch + `ScriptMgr::OnSpellEffectDummy` | Script registry only |
| `Spell::EffectTriggerSpell(SpellEffIndex)` | Cast sub-spell (`SpellEffectInfo::TriggerSpell`) sin GCD, sin power, marcando `TRIGGERED_FULL_MASK` | `Unit::CastSpell` con TriggerCastFlags::TRIGGERED_FULL_MASK |
| `Spell::EffectTriggerMissileSpell(SpellEffIndex)` | Trigger sub-spell con missile speed | `Unit::CastSpell` |
| `Spell::EffectForceCast(SpellEffIndex)` | Force target to cast a spell (puppet-cast) | `target->CastSpell` |
| `Spell::EffectQuestComplete(SpellEffIndex)` | Mark quest complete + handle objective | `Player::CompleteQuest` |
| `Spell::EffectLearnSpell(SpellEffIndex)` | Player learns spell ID | `Player::LearnSpell`, `SendNewSpell` |
| `Spell::EffectOpenLock(SpellEffIndex)` | Picks lock / chest open / mining / herbalism / fishing — lookup `LockEntry` by Lock.dbc, check skill, send SMSG_OPEN_LOCK | `LockMgr::CheckLock`, `Player::SendOpenLock`, `Loot::FillLoot` |
| `Spell::EffectResurrect(SpellEffIndex)` | Send SMSG_RESURRECT_REQUEST a player target | `Player::SetResurrectRequestData`, `SendResurrectRequest` |
| `Spell::EffectAddExtraAttacks(SpellEffIndex)` | Set `m_extraAttacks` for melee chain | `Unit::SetExtraAttacks` |
| `Spell::EffectTaunt(SpellEffIndex)` | Force enemy to attack caster | `ThreatManager::TauntApply` |
| `Spell::EffectInstaKill(SpellEffIndex)` | Set HP to 0 | `Unit::DealDamage(Unit::GetHealth())` |
| `Spell::EffectFeedPet(SpellEffIndex)` | Hunter feed pet — destroy item, apply happiness aura | `Item::Destroy`, `Pet::ModifyHappiness` |
| `Spell::EffectTameCreature(SpellEffIndex)` | Convert creature to pet | `Pet::CreateBaseAtTame` |
| `Spell::EffectModifyCooldown(SpellEffIndex)` / `EffectModifyCooldowns` / `EffectModifyCooldownsByCategory` | Adjust cooldowns on caster (e.g. preparation for rogues) | `SpellHistory::ModifyCooldown` |
| `Spell::EffectModifyAuraStacks(SpellEffIndex)` | Adjust stack count of an aura | `Aura::ModStackAmount` |
| `Spell::EffectModifySpellCharges(SpellEffIndex)` | Adjust charge count | `Aura::ModCharges` |
| `Spell::EffectSendEvent(SpellEffIndex)` | Fire SmartScript event with MiscValue | `Map::ScriptsStart` |
| `Spell::EffectSummonObjectWild` (76, 50) | Summon GameObject (sappers, banner) | `Map::SummonGameObject` |
| `Spell::EffectActivateObject` (107) | Trigger GO action | `GameObject::Use` |
| `Spell::EffectPlayMovie` (162) | SMSG_PLAY_MOVIE | `Player::SendCinematicStart` |
| `Spell::EffectGiveHonor` (240) | Award honor points | `Player::ModifyHonorPoints` |
| `Spell::EffectApplyGlyph` (170) | Apply glyph in slot | `Player::ApplyGlyph` |
| `Spell::EffectStuck` (140) | Unstuck client request | `Player::TeleportTo(homebind)` |

---

## 5. Module dependencies

**Depends on:**
- `Spells/Spell` — `Spell::HandleEffects` es el invoker; cada handler corre como método de `Spell` accediendo a `m_caster`, `m_targets`, `m_damage`, `effectInfo`
- `Spells/SpellInfo` — lee `SpellEffectInfo` (BasePoints, MiscValue, MiscValueB, TriggerSpell, RadiusEntry, Mechanic) per effect
- `Spells/Auras` — `EffectApplyAura` invoca `Aura::TryRefreshStackOrCreate` (ver `spells-aura.md`)
- `Spells/SpellMgr` — `GetSpellTargetPosition` (EffectTeleportUnits), `GetSpellInfo` (EffectTriggerSpell sub-spells)
- `Spells/SpellHistory` — `EffectModifyCooldown*`, `EffectInterruptCast` lock school
- `Entities/Unit` — `DealDamage`, `HealBySpell`, `ModifyPower`, `AddAura`, `RemoveAurasDueToSpell`, `RemoveAurasByDispel`, `SetExtraAttacks`, `InterruptSpell`, `SetTargetGUID`, `SpellDamageBonusDone/Taken`, `SpellHealingBonusDone/Taken`
- `Entities/Player` — `TeleportTo` (EffectTeleportUnits), `LearnSpell`, `StoreNewItem` (EffectCreateItem), `CompleteQuest`, `ApplyGlyph`, `ModifyHonorPoints`, `SendNewItem`, `SetResurrectRequestData`, `SendResurrectRequest`, `SendCinematicStart`
- `Entities/Pet` — `EffectSummonPet`, `EffectTameCreature`, `EffectFeedPet`, `EffectDismissPet`, `EffectLearnPetSpell`
- `Entities/GameObject` — `EffectActivateObject`, `EffectSummonObject*`, `EffectGameObjectDamage/Repair`
- `Entities/DynamicObject` — `EffectPersistentAA`
- `Entities/Item` — `EffectEnchantItemPerm/Tmp/Prismatic`, `EffectCreateItem*`, `EffectDisEnchant`, `EffectFeedPet`
- `Movement/MotionMaster` — `MoveJump`, `MoveCharge`, `MoveKnockbackFrom` (Effect Jump/Charge/KnockBack)
- `Pathing/PathGenerator` — `EffectCharge` path computation
- `Combat/ThreatManager` — `EffectTaunt`, `EffectThreat`, `EffectModifyThreatPercent`
- `Loot/LootStore` — `EffectOpenLock` triggers loot rolls
- `Quests/QuestObjective` — `EffectQuestComplete`, `EffectKillCredit`
- `Maps/Map` — `SummonCreature`, `SummonGameObject` (EffectSummon*)
- `Vehicles` — `EffectSummonType` SUMMON_TYPE_VEHICLE
- `Scripting/ScriptMgr` — `OnEffectScript`, `OnEffectDummy`, `OnEffectHit`, `OnEffectLaunch`
- `DataStores` — Lock.dbc (EffectOpenLock), SummonProperties.dbc (EffectSummonType), SpellItemEnchantment.dbc (EffectEnchantItem*), Cinematic.dbc (EffectPlayMovie), GlyphSlot.dbc (EffectApplyGlyph)

**Depended on by:**
- `Spells/Spell::HandleEffects` is the only direct caller — pero through it, casi todos los sistemas del juego (combat, AI, quests, movement) dependen de que estos handlers ejecuten correctamente

---

## 6. SQL / DB queries (if any)

Los handlers en sí **no emiten queries directamente** — leen de `SpellMgr` cachés ya cargados al startup. Los queries relevantes están en SpellMgr (cargados en `LoadSpellTargetPositions`, `LoadSpellChains`, etc.) y consumidos por:

| Statement / Source | Purpose | DB | Used by |
|---|---|---|---|
| `SELECT * FROM spell_target_position` (cargado al startup en `mSpellTargetPositions`) | Destinos teleport | world | `EffectTeleportUnits` |
| `SELECT * FROM spell_loot_template` | Spell-driven loot | world | `EffectOpenLock` |
| `SELECT * FROM page_text` | Para EffectReadBook (legacy) | world | unused in 3.4.3 |

**DB2 stores read by Effects handlers:**

| Store | What it loads | Read by |
|---|---|---|
| `LockStore` | Lock.db2 (Type[8], Index[8], Skill[8]) | `EffectOpenLock` (skill checks per slot) |
| `SummonPropertiesStore` | SummonProperties.db2 (Control, Faction, Title, Slot, Flags) | `EffectSummonType` (discriminator) |
| `SpellItemEnchantmentStore` | SpellItemEnchantment.db2 (Effect[3], EffectAmount[3], EffectArg[3], MinLevel, MaxLevel, ItemVisualID) | `EffectEnchantItemPerm/Tmp/Prismatic/Held` |
| `GlyphSlotStore` | GlyphSlot.db2 (Type, Tooltip) | `EffectApplyGlyph` |
| `GlyphPropertiesStore` | GlyphProperties.db2 (SpellID, GlyphSlotFlags, SpellIconID) | `EffectApplyGlyph` |
| `CinematicSequencesStore` | CinematicSequences.db2 | `EffectPlayMovie` |
| `MovieStore` | Movie.db2 | `EffectPlayMovie` |
| `SceneScriptStore` / `SceneScriptPackageStore` | Scene scripts | `EffectPlaySceneScriptPackage`, `EffectPlayScene` |

---

## 7. Wire-protocol packets (if any)

Los packets que los handlers emiten directamente (después de aplicar el efecto):

| Opcode | Direction | Sent in | Effect that triggers it |
|---|---|---|---|
| `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` | server → client (broadcast) | `SendSpellNonMeleeDamageLog` | `EffectSchoolDMG`, `EffectWeaponDmg`, `EffectHealthLeech` |
| `SMSG_SPELL_HEAL_LOG` | server → client | `SendSpellHealLog` | `EffectHeal`, `EffectHealPct`, `EffectHealMaxHealth` |
| `SMSG_SPELL_ENERGIZE_LOG` | server → client | `SendSpellEnergizeLog` | `EffectEnergize`, `EffectEnergizePct`, `EffectPowerDrain` |
| `SMSG_SPELL_INSTAKILL_LOG` | server → client | manual write | `EffectInstaKill` |
| `SMSG_SPELL_DISPELL_LOG` | server → client | `SendSpellDispelLog` | `EffectDispel`, `EffectStealBeneficialBuff` |
| `SMSG_DISPEL_FAILED` | server → client | manual | `EffectDispel` (resisted) |
| `SMSG_RESURRECT_REQUEST` | server → client (target) | `SendResurrectRequest` | `EffectResurrect` |
| `SMSG_LEARNED_SPELL` | server → client | `SendLearnNewSpell` | `EffectLearnSpell` |
| `SMSG_PLAY_MOVIE` | server → client | manual | `EffectPlayMovie` |
| `SMSG_PLAY_SCENE` | server → client | manual | `EffectPlayScene` |
| `SMSG_NOTIFY_DEST_LOC_SPELL_CAST` | server → client (broadcast) | manual | EffectSummonObject*, dest-targeted effects |
| `SMSG_TELEPORT_REQUEST` (or implicit in Player::TeleportTo) | server → client | `Player::TeleportTo` | `EffectTeleportUnits` |
| `SMSG_MOVE_KNOCK_BACK` | server → client | `Player::SendKnockBack` | `EffectKnockBack`, `EffectKnockBackDest` |
| `SMSG_MOVE_JUMP` | server → client | implicit | `EffectJump`, `EffectJumpDest` |
| `SMSG_CHARGE` (within MoveCharge response chain) | server → client | implicit | `EffectCharge`, `EffectChargeDest` |
| `SMSG_OPEN_LOCK` | server → client | `Player::SendOpenLock` | `EffectOpenLock` |
| `SMSG_TAME_FAILURE` | server → client | manual | `EffectTameCreature` (failure) |
| `SMSG_PET_LEARNED_SPELL` | server → client | manual | `EffectLearnPetSpell` |
| `SMSG_GROUP_REWARD_HONOR` | server → client | manual | `EffectGiveHonor` |
| `SMSG_QUERY_QUEST_REWARD_RESPONSE` (or quest update) | server → client | implicit via `Player::CompleteQuest` | `EffectQuestComplete` |
| `SMSG_INTERRUPT_POWER_REGEN` | server → client | manual | `EffectInterruptCast` (related) |

Los effects que solo emiten state changes (`EffectApplyAura`, `EffectModifyCooldown`, `EffectAddComboPoints`) typically piggyback sobre packets de update genéricos (`SMSG_AURA_UPDATE`, `SMSG_MODIFY_COOLDOWN`, character update fields) — no emiten packet propio.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-spell/src/effects/dispatch.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/spell.rs` | `file` | 1 | 466 | `exists_active` | file exists |
| `crates/wow-data/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-spell/src/lib.rs` — **0 líneas (vacío)**. **Cero handlers de effects**.
- `crates/wow-world/src/handlers/spell.rs` — handler `CMSG_CAST_SPELL` con stub `execute_spell(spell_id, target_guid)` cuyo cuerpo no aplica daño, heal, aura, teleport, ni ningún SpellEffect concreto. Es un placeholder de nombre.
- `crates/wow-packet/src/packets/spell.rs` — POD `CastSpellRequest`, `SpellStartPkt`, `CastFailed`, `SpellTargetData`, `SpellCastVisual`. Ningún log packet de effects (`SMSG_SPELL_NON_MELEE_DAMAGE_LOG`, `SMSG_SPELL_HEAL_LOG`, `SMSG_SPELL_ENERGIZE_LOG`, `SMSG_SPELL_DISPELL_LOG`).
- `crates/wow-data/src/spell_info.rs` (referenciado) — expone `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, `has_cast_time` — pero no `SpellEffectInfo[]`, no `effect: SpellEffectName`, no `BasePoints`, no `MiscValue`, no `TriggerSpell`, no `RadiusEntry`, no `Mechanic`. Sin estos campos no es posible siquiera leer qué effects tiene un spell.

**What's implemented:**
- 0 of ~151 effect handlers.
- 0 dispatch table.
- 0 effect-specific log packets.

**What's missing vs C++:**
1. **No `enum SpellEffectName`** — los ~151 IDs no están definidos en Rust; ni siquiera un enum stub
2. **No `enum SpellEffectHandleMode`** (LAUNCH, LAUNCH_TARGET, HIT, HIT_TARGET)
3. **No `struct SpellEffectInfo`** completo (BasePoints, MiscValue, TriggerSpell, ChainTargets, RadiusEntry, Mechanic, ItemType, EffectAttributes)
4. **No dispatch table** — ni `SpellEffectHandlers[151]` array, ni `match spell_effect { … }`
5. **0 of ~151 handlers**: `EffectSchoolDMG`, `EffectHeal`, `EffectApplyAura`, `EffectTeleportUnits`, `EffectSummonType`, `EffectDispel`, `EffectInterruptCast`, `EffectKnockBack`, `EffectJump`, `EffectCharge`, `EffectEnergize`, `EffectPowerDrain`, `EffectCreateItem`, `EffectEnchantItem*`, `EffectScriptEffect`, `EffectDummy`, `EffectTriggerSpell`, `EffectQuestComplete`, `EffectLearnSpell`, `EffectOpenLock`, `EffectResurrect`, `EffectAddExtraAttacks`, `EffectTaunt`, `EffectInstaKill`, `EffectFeedPet`, `EffectTameCreature`, `EffectModifyCooldown*`, … todos ausentes
6. **No `Spell::HandleEffects`** invoker — no existe la función bisagra
7. **No effect log packets**: ni `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` ni `SMSG_SPELL_HEAL_LOG` ni `SMSG_SPELL_ENERGIZE_LOG` ni `SMSG_SPELL_DISPELL_LOG` ni `SMSG_SPELL_INSTAKILL_LOG`
8. **No script hooks**: `OnEffectHit`, `OnEffectLaunch`, `OnEffectDummy`, `OnEffectScript` no existen
9. **No effect-specific DB2 lookups**: Lock.db2 (OpenLock), SummonProperties.db2 (SummonType), SpellItemEnchantment.db2 (EnchantItem*), GlyphSlot/Properties.db2 (ApplyGlyph) — ninguno cargado
10. **No `m_damage` / `m_healing` aggregation**: en C++ los handlers escriben a estos members del `Spell` y al finalizar `Spell::DoAllEffectOnTarget` se materializan a `Unit::DealDamage` / `Unit::HealBySpell`. RustyCore ni siquiera tiene `Spell` struct (ver `spells-cast.md`).

**Suspicious / likely divergent:**
- El stub `execute_spell(spell_id, target_guid)` en handlers/spell.rs es nombrado como si fuera el dispatch pero su cuerpo (visible o no) no contiene ningún match sobre `SpellEffectName`, ningún branch por effect type. Es plumbing-only.
- Sin `SpellEffectInfo` en `wow_data::SpellInfo`, ni siquiera podríamos leer qué effect aplicar — el tipo `SpellEffect` no existe en el contrato.

**Tests existing:**
- 0 tests de effects (`crates/wow-spell/`, `crates/wow-world/src/handlers/spell.rs`, `crates/wow-packet/`).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SPELLS_EFFECTS.WBS.001** Partir y cerrar la migracion auditada de `game/Spells/SpellEffects.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp`
  Rust target: `crates/wow-spell`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 5956 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerados como `#SPELLS-EFFECTS.N` para referencia desde `MIGRATION_ROADMAP.md`. Complejidad: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#SPELLS-EFFECTS.1** Definir `enum SpellEffectName` con las ~151 variantes (`#[repr(u32)]`) en `crates/wow-spell/src/effects/types.rs`. Marcar `RetailOnly_NNN` los que son post-WoLK (id > 270 ish) (M)
- [ ] **#SPELLS-EFFECTS.2** Definir `enum SpellEffectHandleMode { Launch, LaunchTarget, Hit, HitTarget }` (L)
- [ ] **#SPELLS-EFFECTS.3** Implementar `struct SpellEffectInfo` con todos los campos: effect, base_points, real_points_per_level, points_per_combo, dice_per_level, base_dice, damage_multiplier, misc_value, misc_value_b, trigger_spell, implicit_target_a/b, mechanic, radius_entry, max_radius_entry, chain_targets, chain_amplitude, item_type, real_points_per_level, effect_attributes, scaling_class (cargado de SpellEffect.db2) (H)
- [ ] **#SPELLS-EFFECTS.4** Implementar `Spell::handle_effects(unit_target, item_target, gameobj_target, corpse_target, dest_target, effect_info, mode)` con dispatch — `match effect_info.effect { … }` (M)
- [ ] **#SPELLS-EFFECTS.5** Implementar `Spell::handle_launch_phase()` y `handle_immediate_phase()` y `do_all_effect_on_target(target_info)` (M)
- [ ] **#SPELLS-EFFECTS.6** Implementar `EffectSchoolDMG` con SpellDamageBonusDone/Taken, school resist, crit roll, write `m_damage` (H)
- [ ] **#SPELLS-EFFECTS.7** Implementar `EffectHeal` con SpellHealingBonusDone/Taken, crit, overheal log (M)
- [ ] **#SPELLS-EFFECTS.8** Implementar `EffectApplyAura` → `Aura::TryRefreshStackOrCreate` (depende de `spells-aura.md` task #SPELLS-AURA.20) (L)
- [ ] **#SPELLS-EFFECTS.9** Implementar `EffectApplyAreaAuraEnemy/Friend/Pet/Owner/Party/Raid` variants (6 effects) — usan filtros faction (M)
- [ ] **#SPELLS-EFFECTS.10** Implementar `EffectPersistentAA` con DynamicObject creation + DynObjAura (depende de DynObjAura, ver `spells-aura.md`) (H)
- [ ] **#SPELLS-EFFECTS.11** Implementar `EffectTeleportUnits` con `SpellMgr::get_spell_target_position` lookup + `Player::TeleportTo` (L)
- [ ] **#SPELLS-EFFECTS.12** Implementar `EffectTeleportUnitsWithVisualLoadingScreen` y `EffectTeleUnitsFaceCaster` (L)
- [ ] **#SPELLS-EFFECTS.13** Implementar `EffectEnergize` y `EffectEnergizePct` con `Unit::ModifyPower` + SMSG_SPELL_ENERGIZE_LOG (L)
- [ ] **#SPELLS-EFFECTS.14** Implementar `EffectPowerDrain` y `EffectPowerBurn` (M)
- [ ] **#SPELLS-EFFECTS.15** Implementar `EffectHealthLeech` (damage + heal caster) (L)
- [ ] **#SPELLS-EFFECTS.16** Implementar `EffectHealPct`, `EffectHealMechanical`, `EffectHealMaxHealth` (L)
- [ ] **#SPELLS-EFFECTS.17** Implementar `EffectInstaKill` con SMSG_SPELL_INSTAKILL_LOG (L)
- [ ] **#SPELLS-EFFECTS.18** Implementar `EffectEnvironmentalDMG` (lava, fall, drown — bypass resistance) (L)
- [ ] **#SPELLS-EFFECTS.19** Implementar `EffectAddExtraAttacks` (set m_extraAttacks) (L)
- [ ] **#SPELLS-EFFECTS.20** Implementar `EffectWeaponDmg`, `EffectWeaponDmgNoSchool`, `EffectNormalizedWeaponDmg`, `EffectWeaponDmgPct` (4 variantes) (H)
- [ ] **#SPELLS-EFFECTS.21** Implementar `EffectKnockBack` y `EffectKnockBackDest` con `MotionMaster::move_knockback_from` + SMSG_MOVE_KNOCK_BACK (M)
- [ ] **#SPELLS-EFFECTS.22** Implementar `EffectJump` y `EffectJumpDest` con `MotionMaster::move_jump` (M)
- [ ] **#SPELLS-EFFECTS.23** Implementar `EffectLeap` y `EffectLeapBack` (Blink, Disengage) (M)
- [ ] **#SPELLS-EFFECTS.24** Implementar `EffectCharge` y `EffectChargeDest` con PathGenerator + MoveCharge + stun + enter combat (H — depende de Pathing/PathGenerator)
- [ ] **#SPELLS-EFFECTS.25** Implementar `EffectJumpCharge` y `EffectMomentum` (M)
- [ ] **#SPELLS-EFFECTS.26** Implementar `EffectPull` / `EffectPullTowardsDest` (M)
- [ ] **#SPELLS-EFFECTS.27** Implementar `EffectSummonType` discriminator por SummonProperties.db2 (~218 lines C++; despacha a SUMMON_TYPE_NONE/PET/GUARDIAN/MINION/TOTEM/MINIPET/VEHICLE/...) (XL — splittable per type)
- [ ] **#SPELLS-EFFECTS.28** Implementar `EffectSummonPet` (paladin pre-existing pet, hunter call pet) (M)
- [ ] **#SPELLS-EFFECTS.29** Implementar `EffectSummonObjectWild` y `EffectSummonObject` (sappers, banner, repair bot) — gameobject summon (M)
- [ ] **#SPELLS-EFFECTS.30** Implementar `EffectSummonChangeItem` (replace item slot with summon) (L)
- [ ] **#SPELLS-EFFECTS.31** Implementar `EffectSummonPlayer` (warlock ritual) (L)
- [ ] **#SPELLS-EFFECTS.32** Implementar `EffectResurrect` con SMSG_RESURRECT_REQUEST (L)
- [ ] **#SPELLS-EFFECTS.33** Implementar `EffectResurrectNew` (sel HP/Mana variant) (L)
- [ ] **#SPELLS-EFFECTS.34** Implementar `EffectSelfResurrect` (Reincarnation, Soulstone-self) (L)
- [ ] **#SPELLS-EFFECTS.35** Implementar `EffectResurrectPet` (Hunter/Warlock pet revive) (L)
- [ ] **#SPELLS-EFFECTS.36** Implementar `EffectDispel` con DispelMask + DispelChance + ICD + SMSG_SPELL_DISPELL_LOG (M)
- [ ] **#SPELLS-EFFECTS.37** Implementar `EffectStealBeneficialBuff` (Mage Spellsteal) (M)
- [ ] **#SPELLS-EFFECTS.38** Implementar `EffectInterruptCast` con cancel + lock school + ChannelInterruptFlags check (M)
- [ ] **#SPELLS-EFFECTS.39** Implementar `EffectSanctuary` (drop combat + remove threat) (L)
- [ ] **#SPELLS-EFFECTS.40** Implementar `EffectDualWield` (passive proficiency grant) (L)
- [ ] **#SPELLS-EFFECTS.41** Implementar `EffectTaunt` con `ThreatManager::TauntApply` (L)
- [x] **#SPELLS-EFFECTS.42** Representar `EffectThreat` y `EffectModifyThreatPercent` para caster
  jugador y target criatura (`SPELL_EFFECT_THREAT = 63`,
  `SPELL_EFFECT_MODIFY_THREAT_PERCENT = 125`). C++:
  `SpellEffects.cpp:2864-2880` (`EffectThreat`, `SPELL_EFFECT_HANDLE_HIT_TARGET`, caster unit alive,
  target con threat list, `AddThreat(unitCaster, float(damage), m_spellInfo, true)`) y
  `SpellEffects.cpp:4372-4382` (`EffectModifyThreatPercent`, caster+target, `ModifyThreatByPercent`).
  Rust actualiza threat legacy y espejo canónico de la criatura, con tests de suma, porcentaje y
  caster muerto. Sigue `represented-partial`: no hay `CanHaveThreatList` completo para todos los
  tipos de Unit, ni `ThreatManager` vivo/fanout/lista de amenaza completa, ni validación manual
  servidor/cliente.
- [ ] **#SPELLS-EFFECTS.43** Implementar `EffectAddComboPoints` (L)
- [ ] **#SPELLS-EFFECTS.44** Implementar `EffectModifyAuraStacks`, `EffectModifyCooldown`, `EffectModifyCooldowns`, `EffectModifyCooldownsByCategory`, `EffectModifySpellCharges` (M)
- [ ] **#SPELLS-EFFECTS.45** Implementar `EffectScriptEffect` con switch sobre spellId + ScriptMgr::OnEffectScript fallback (XL — el handler en sí es 150 lines, plus ~50 special cases por boss)
- [ ] **#SPELLS-EFFECTS.46** Implementar `EffectDummy` con ScriptMgr::OnEffectDummy dispatch (M)
- [ ] **#SPELLS-EFFECTS.47** Implementar `EffectTriggerSpell` (force `caster.cast_spell(trigger_id, target, TRIGGERED_FULL_MASK)`) (M)
- [ ] **#SPELLS-EFFECTS.48** Implementar `EffectTriggerMissileSpell` y `EffectTriggerRitualOfSummoning` (M)
- [ ] **#SPELLS-EFFECTS.49** Implementar `EffectForceCast` (target casts as if puppet) (M)
- [ ] **#SPELLS-EFFECTS.50** Implementar `EffectCreateItem` y `EffectCreateItem2` y `EffectCreateRandomItem` (profession crafting) con `Player::store_new_item` + SMSG (M)
- [ ] **#SPELLS-EFFECTS.51** Implementar `EffectEnchantItemPerm`, `EffectEnchantItemTmp`, `EffectEnchantItemPrismatic`, `EffectEnchantHeldItem` con SpellItemEnchantment.db2 lookup (H)
- [ ] **#SPELLS-EFFECTS.52** Implementar `EffectDisEnchant`, `EffectMillItem`, `EffectProspecting` (M)
- [ ] **#SPELLS-EFFECTS.53** Implementar `EffectFeedPet` (consume item, apply happiness aura) (L)
- [ ] **#SPELLS-EFFECTS.54** Implementar `EffectQuestComplete`, `EffectQuestStart`, `EffectQuestRedirect`, `EffectKillCredit*` (M)
- [ ] **#SPELLS-EFFECTS.55** Implementar `EffectLearnSpell`, `EffectUnlearnSpecialization`, `EffectLearnPetSpell`, `EffectLearnSkill` (M)
- [ ] **#SPELLS-EFFECTS.56** Implementar `EffectTradeSkill` y `EffectProficiency` (L)
- [ ] **#SPELLS-EFFECTS.57** Implementar `EffectUntrainTalents` (L)
- [ ] **#SPELLS-EFFECTS.58** Implementar `EffectOpenLock` con Lock.db2 lookup, skill check per slot, SMSG_OPEN_LOCK + Loot::FillLoot (H)
- [ ] **#SPELLS-EFFECTS.59** Implementar `EffectActivateObject` (L)
- [ ] **#SPELLS-EFFECTS.60** Implementar `EffectSendEvent` (SmartScript event fire) (L)
- [ ] **#SPELLS-EFFECTS.61** Implementar `EffectTameCreature` (M)
- [ ] **#SPELLS-EFFECTS.62** Implementar `EffectDismissPet` (L)
- [ ] **#SPELLS-EFFECTS.63** Implementar `EffectAddFarsight` (L)
- [ ] **#SPELLS-EFFECTS.64** Implementar `EffectInebriate` (drunk state) (L)
- [ ] **#SPELLS-EFFECTS.65** Implementar `EffectParry`, `EffectBlock` (passive grant) (L)
- [ ] **#SPELLS-EFFECTS.66** Implementar `EffectReputation` (faction modify) (L)
- [ ] **#SPELLS-EFFECTS.67** Implementar `EffectDuel` (L)
- [x] **#SPELLS-EFFECTS.68** Implementar `EffectStuck` (homebind teleport) (L) — represented-partial:
  `CONFIG_CAST_UNSTUCK`, flight/dead/cooldown gates, homebind teleport, and Hearthstone cooldown
  are covered; full `SpellHistory` duration expiry, exact `KillSelf` death pipeline, and live
  client/manual validation remain open.
- [ ] **#SPELLS-EFFECTS.69** Implementar `EffectApplyGlyph` con GlyphProperties.db2 (M)
- [ ] **#SPELLS-EFFECTS.70** Implementar `EffectPlayMovie`, `EffectPlayScene`, `EffectPlaySceneScriptPackage`, `EffectCreateSceneObject`, `EffectCreatePrivateSceneObject` (M)
- [ ] **#SPELLS-EFFECTS.71** Implementar `EffectGiveHonor`, `EffectGrantBattlePetExperience`, `EffectChangeRaidMarker` (L)
- [ ] **#SPELLS-EFFECTS.72** Implementar `EffectGameobjectDamage`, `EffectGameObjectRepair`, `EffectGameobjectSetDestructionState` (M)
- [ ] **#SPELLS-EFFECTS.73** Implementar `EffectPickPocket` (L)
- [ ] **#SPELLS-EFFECTS.74** Implementar `EffectDistract` (turn target's facing) (L)
- [ ] **#SPELLS-EFFECTS.75** Implementar `EffectForceDeselect`, `EffectSendChatMessage`, `EffectActivateRune`, `EffectCreatePrivateConversation`, `EffectTeleportGraveyard`, `EffectModifyAuraStacks` (M total)
- [ ] **#SPELLS-EFFECTS.76** Implementar log packets: `SMSG_SPELL_NON_MELEE_DAMAGE_LOG`, `SMSG_SPELL_HEAL_LOG`, `SMSG_SPELL_ENERGIZE_LOG`, `SMSG_SPELL_INSTAKILL_LOG`, `SMSG_SPELL_DISPELL_LOG`, `SMSG_DISPEL_FAILED` (M)
- [ ] **#SPELLS-EFFECTS.77** Marcar como `_RetailOnly` (no-op) los effects > id 270 (Dragonflight-only): EffectCreateTraitTreeConfig, EffectChangeActiveCombatTraitConfig, EffectLearnTransmogIllusion, EffectModifySpellCharges (id-dependent), etc. (L)
- [ ] **#SPELLS-EFFECTS.78** Stubbed `EffectNULL` y `EffectUnused` para los ~50 deprecated entries (L)
- [ ] **#SPELLS-EFFECTS.79** Hook `ScriptMgr::on_effect_hit/launch/dummy/script` callbacks en handlers (M)
- [ ] **#SPELLS-EFFECTS.80** Test harness: invocar dispatch table sobre cada effect con dummy SpellInfo, verificar el handler correcto runs (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SPELLS_EFFECTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 1 files / 5956 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp`. Rust target: `crates/wow-packet`, `crates/wow-spell`. | `cargo test -p wow-packet && cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SPELLS_EFFECTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 1 files / 5956 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp`. Rust target: `crates/wow-packet`, `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SPELLS_EFFECTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 1 files / 5956 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp`. Rust target: `crates/wow-packet`, `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SPELLS_EFFECTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 1 files / 5956 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp`. Rust target: `crates/wow-packet`, `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: dispatch table — para cada `SpellEffectName` valid, `Spell::handle_effects` invoca el handler correcto (no crash, no NULL handler)
- [ ] Test: `EffectSchoolDMG` Fireball — caster con SP=500, target sin resist → damage = base + SP*coef + crit roll
- [ ] Test: `EffectSchoolDMG` con target.fire_resist=50% → damage * 0.5
- [ ] Test: `EffectHeal` Renew tick — heal = base + SP*coef * tickCount; overheal log if target.health == max
- [ ] Test: `EffectHeal` crit roll — chance respeta caster.crit_chance + spell.bonus_crit
- [ ] Test: `EffectApplyAura` Renew (139) → `target.has_aura(139) == true`, duration = 15s
- [ ] Test: `EffectTeleportUnits` Hearthstone (8690) — lookup spell_target_position by spell+effect, player teleporta a homebind
- [ ] Test: `EffectEnergize` Innervate — target.power += amount, SMSG_SPELL_ENERGIZE_LOG enviado
- [ ] Test: `EffectPowerDrain` Mana Burn — target.mana -= drain, caster.health no se restaura
- [ ] Test: `EffectInstaKill` — target.health = 0, SMSG_SPELL_INSTAKILL_LOG, kill credit
- [ ] Test: `EffectKnockBack` — target.position move along direction by speedXY/Z, SMSG_MOVE_KNOCK_BACK enviado
- [ ] Test: `EffectJump` — target jumps to caster's position with arc trajectory
- [ ] Test: `EffectCharge` Warrior Charge — caster moves to target via PathGenerator, applies stun aura, enters combat
- [ ] Test: `EffectSummonType` Imp (1860) — discriminator por SummonProperties.MiscValueB → SUMMON_TYPE_PET → creates Pet creature
- [ ] Test: `EffectSummonType` Searing Totem — discriminator → SUMMON_TYPE_TOTEM → totem with slot=1
- [ ] Test: `EffectDispel` Cleanse — target.has_aura(magic_dot), cast Cleanse, dispel succeeds with chance roll
- [ ] Test: `EffectInterruptCast` Pummel — target casteando, cast Pummel → cast cancelled, school locked 4s
- [ ] Test: `EffectCreateItem` profession craft — target.bag has new item with quality/suffix from spell
- [ ] Test: `EffectEnchantItemPerm` Glove enchant — item slot has enchantment, applies stat
- [ ] Test: `EffectLearnSpell` Train new spell — player.known_spells contains new id, SMSG_LEARNED_SPELL
- [ ] Test: `EffectOpenLock` chest — Lock.db2 skill check, success → loot generated, SMSG_OPEN_LOCK
- [ ] Test: `EffectResurrect` — target receives SMSG_RESURRECT_REQUEST with caster info
- [ ] Test: `EffectAddExtraAttacks` Sweeping Strikes — caster.m_extraAttacks = 1, next melee chains
- [ ] Test: `EffectTaunt` — enemy now targets caster, threat reset
- [ ] Test: `EffectTriggerSpell` Holy Shock heal proc — sub-spell casted with TRIGGERED_FULL_MASK, no GCD, no power
- [ ] Test: `EffectScriptEffect` boss-specific spell — ScriptMgr callback fires con (spell, effect_index, target)
- [ ] Test: `EffectModifyCooldown` Preparation — all CDs reset, SMSG_MODIFY_COOLDOWN per spell
- [ ] Test: SUMMON_TYPE_VEHICLE — creates vehicle with VehicleAccessory loaded
- [ ] Test: HandleLaunchPhase vs HandleHitPhase — `EffectSchoolDMG` corre en LAUNCH (damage calc), `EffectKnockBack` en HIT (post-projectile)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 1 files / 5956 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` | `crates/wow-spell/` (módulo `effects`), `crates/wow-spell/src/effects/dispatch.rs` \| ❌ not started — `wow-spell` está en 0 líneas; sin dispatch table, sin handlers |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SPELLS_EFFECTS.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 1 C++ files / 5956 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SPELLS_EFFECTS.DIV.002` | `crates/wow-spell/src/effects/dispatch.rs` (`missing_declared_path`, 0 Rust lines) | 1 C++ files / 5956 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SPELLS_EFFECTS.DIV.003` | `crates/wow-spell/src/lib.rs` (`exists_empty`, 0 Rust lines) | 1 C++ files / 5956 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SPELLS_EFFECTS.DIV.004` | `crates/wow-data/src/spell_info.rs` (`missing_declared_path`, 0 Rust lines) | 1 C++ files / 5956 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`EffectScriptEffect` y `EffectDummy` son catch-all.** Mucha lógica de bosses y abilities específicas vive ahí. El handler en C++ es un giant switch sobre `m_spellInfo->Id` con ~50-100 special cases — al portar, es preferible mover esa lógica a SpellScript registry y dejar el handler genérico delegando a script. Don't replicate the C++ switch literally — usa el DSL.
- **`EffectSummonType` (28) es un effecto, ~10 tipos distintos.** El discriminator real es `SummonProperties.db2` indexed por `m_spellInfo->GetEffect(eff_idx).MiscValueB`. Los tipos: SUMMON_TYPE_NONE=0, SUMMON_TYPE_PET=1, SUMMON_TYPE_GUARDIAN=2, SUMMON_TYPE_MINION=3, SUMMON_TYPE_TOTEM=4, SUMMON_TYPE_MINIPET=5 (companion), SUMMON_TYPE_VEHICLE_FORCED=6, SUMMON_TYPE_VEHICLE_FACING=7, etc. Cada uno tiene factionMask + control flags + lifetime distinto. Bug clásico: tratarlos uniforme.
- **LAUNCH vs HIT mode separation.** Damage spells corren `EffectSchoolDMG` en LAUNCH (computar daño con stats actuales del caster, antes del missile travel). Al impact (HIT), el damage ya está pre-computado. Si haces todo en HIT, el caster puede morir y aún así su daño se aplica con stats post-mortem (broken). Algunos efectos específicos (`EffectKnockBack`) corren en HIT porque dependen de la posición final.
- **`EffectTriggerSpell` evita GCD/power, pero PUEDE seguir CheckCast.** El sub-spell con TRIGGERED_FULL_MASK skipea: power cost, range check, casting time, LOS, items, shapeshift, aura state. MANTIENE: target validity, in_combat checks. Algunos spells abuse esto (Hand of Reckoning bypassea casi todo). Hay variantes `TRIGGERED_DONT_CHECK_GCD`, `TRIGGERED_IGNORE_POWER` etc. para granularidad.
- **`EffectApplyAura` es solo el plumbing.** El handler invoca `Aura::TryRefreshStackOrCreate(create_info)` y termina. Toda la lógica del aura (apply effect, periodic, remove) está en `spells-aura.md`. Es una boundary clara entre los dos sub-módulos.
- **Wire packet ordering matters.** Para spells con multiple effects, el orden es: `SMSG_SPELL_GO` (target list, hit/miss) → effect logs (damage, heal, energize, etc.) → aura updates (`SMSG_AURA_UPDATE`) → cooldown (`SMSG_SPELL_COOLDOWN`). Cliente no muestra el efecto si el orden está mal.
- **`SpellEffectInfo::BasePoints` es un base con scaling.** El amount real es `BasePoints + RealPointsPerLevel * level + roll(BaseDice * DicePerLevel * level)`. Players usan combo points (`PointsPerComboPoint`). En 3.4.3, también hay `SpellScalingEntry` para spec-aware scaling. La fórmula final está en `Spell::CalculateDamage(SpellEffectInfo const&, Unit const* target, float* var)`. **Cualquier handler que retornee un amount** debe pasar por esta fórmula, no leer BasePoints raw.
- **`MiscValue` y `MiscValueB` semantica per-effect.** No es genérico. Para `EffectEnergize`, MiscValue = power type (0=mana, 1=rage, 2=focus, 3=energy, 6=runic). Para `EffectSummonType`, MiscValueB = SummonProperties row index. Para `EffectApplyAura`, MiscValue = misc data del aura (e.g. SchoolMask para SCHOOL_IMMUNITY). Documentar caso por caso.
- **`EffectOpenLock` skill check is multi-slot.** Lock.db2 tiene 8 slots con (Type, Index, Skill). Type=2 ITEM_REQUIRED (key), Type=3 LOCKPICKING/MINING/HERBALISM/etc. (skill check). Cualquier slot OK = unlock. Si Skill > player.skill, fall — but partial skill usable for "you can still try". Bug histórico: chequear solo slot 0.
- **`EffectEnchantItem*` reads SpellItemEnchantment.db2.** El "enchantment" no es el spell, es la entry en `SpellItemEnchantmentEntry`. Cada entry tiene `Effect[3]` (ITEM_ENCHANTMENT_TYPE_*) y `EffectAmount[3]`. Tipos: COMBAT_SPELL=1, DAMAGE=2, EQUIP_SPELL=3, RESISTANCE=4, STAT=5, TOTEM=6, USE_SPELL=7, PRISMATIC_SOCKET=8. Cada uno aplica diferente al equipar.
- **`EffectQuestComplete` doesn't always succeed.** Verifica que el player esté en el quest y que el step sea spell-completable. Sin esto, exploit: cast complete-spell sin tener el quest.
- **`EffectResurrect` SMSG_RESURRECT_REQUEST is a player choice.** Solo manda el request; el resurrect real espera `CMSG_RESURRECT_RESPONSE`. Persiste hasta que el player decida o expire (15s).
- **`m_damage` y `m_healing` aggregation.** En C++ `Spell` carries `m_damage: int32` y `m_healing: int32` que los effect handlers escriben. Al final de `DoAllEffectOnTarget`, se materializa a `Unit::DealDamage(m_damage)` / `Unit::HealBySpell(m_healing)`. Los handlers per-effect NO llaman a Unit directly — escriben al miembro y dejan que el pipeline lo flush. RustyCore debe replicar este patrón para evitar double-apply o partial damage en spells multi-effect.
- **`EffectScriptEffect` retail-only effects.** En 3.4.3 hay menos casos especiales que retail moderno. Cuidado con copy-paste de TC moderno: muchos spell IDs en el switch no existen en WoLK. Filter por `ContentTuningId` o spell IDs known en 3.4.3.
- **EffectAttributes (SpellEffectAttributes).** Algunos effects tienen flags adicionales (`NoImmunity`, `CanTargetUntargetableUnit`, etc.) en `SpellEffectInfo::EffectAttributes`. Verificar antes de hit checks. Bug histórico: `EffectInterruptCast` ignorando immune-magic.
- **Performance: handlers son called múltiples veces per cast (one per (effect, target) tuple).** AoE spell con 3 effects y 10 targets = 30 handler calls. Cada handler debe ser side-effect-only (no allocations en el hot path), o pre-allocate scratch buffers en el `Spell` instance.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `enum SpellEffectName` (151 vals) | `#[repr(u32)] enum SpellEffect { None=0, InstaKill=1, SchoolDamage=2, … }` con `#[non_exhaustive]` | Match es la dispatch table — más idiomática que function pointer array |
| `enum SpellEffectHandleMode` | `enum SpellEffectHandleMode { Launch, LaunchTarget, Hit, HitTarget }` | passed as parámetro a `handle_effects` |
| `class SpellEffectInfo` | `struct SpellEffectInfo` en `crates/wow-spell/src/spell_info/effect_info.rs` | Cargado de SpellEffect.db2 + scaling; immutable post-load |
| `SpellEffectHandlers[151]` (function pointer table) | `match effect_info.effect { SpellEffect::SchoolDamage => self.effect_school_dmg(idx), … }` | match es the table; permite borrows seguros |
| `void Spell::EffectXxx()` | `fn effect_xxx(&mut self, idx: SpellEffIndex)` métodos sobre `Spell` | Same signature pattern |
| `Spell::HandleEffects(unit, item, go, corpse, effInfo, mode)` | `fn handle_effects(&mut self, target_set: &EffectTargets, effect_info: &SpellEffectInfo, mode: SpellEffectHandleMode)` | `EffectTargets { unit: Option<&mut Unit>, item: Option<&mut Item>, gameobj: Option<&mut GameObject>, corpse: Option<&Corpse>, dest: Option<Position> }` |
| `m_damage: int32` (Spell member) | `damage: i32` field en Spell struct | escrito por handlers, leído por finish phase |
| `m_healing: int32` | `healing: i32` field en Spell | idem |
| `m_targets.GetUnitTarget()` | `self.targets.unit_target.as_ref()` | `SpellCastTargets` struct |
| `m_caster->CastSpell(triggerSpell, true)` | `self.caster_mut().cast_spell(trigger_spell, &target, TriggerCastFlags::TRIGGERED_FULL_MASK)` | `TriggerCastFlags` bitflags |
| `Unit::DealSpellDamage` | `Unit::deal_spell_damage(&mut self, target: &mut Unit, school: SpellSchoolMask, amount: i32, crit: bool)` | called from EffectSchoolDMG path |
| `Player::TeleportTo(map, x, y, z, o, options)` | `Player::teleport_to(map: u32, pos: &Position, options: TeleportOptions) -> Result<(), TeleportError>` | called from EffectTeleportUnits |
| `MotionMaster::MoveJump(x, y, z, speedXY, speedZ, id)` | `MotionMaster::move_jump(dest: Position, speed_xy: f32, speed_z: f32, id: u32)` | EffectJump |
| `MotionMaster::MoveCharge(x, y, z, speed, id, generatePath)` | `MotionMaster::move_charge(dest: Position, speed: f32, id: u32, generate_path: bool)` | EffectCharge |
| `SpellMgr::GetSpellTargetPosition(spellId, effIdx)` | `spell_mgr.get_spell_target_position(spell_id, eff_idx) -> Option<&SpellTargetPosition>` | EffectTeleportUnits |
| `LockEntry const* lockInfo = sLockStore.LookupEntry(lockId)` | `lock_store.get(lock_id) -> Option<&LockEntry>` | EffectOpenLock |
| `SummonPropertiesEntry const* properties = sSummonPropertiesStore.LookupEntry(...)` | `summon_properties_store.get(id) -> Option<&SummonPropertiesEntry>` | EffectSummonType |
| `Map::SummonCreature(entry, pos, properties, durationMs, summoner, spellId)` | `Map::summon_creature(entry: u32, pos: &Position, properties: &SummonPropertiesEntry, duration_ms: u32, summoner: &mut Unit, spell_id: u32) -> Option<CreatureHandle>` | — |
| `Player::StoreNewItem(...)` | `Player::store_new_item(item_id: u32, count: u32, randomPropertyId: i32) -> Result<ItemHandle, InventoryError>` | EffectCreateItem |
| `SendSpellNonMeleeDamageLog(SpellNonMeleeDamage*)` | `spell.send_spell_non_melee_damage_log(&damage_log: SpellNonMeleeDamage, broadcaster: &MapBroadcaster)` | broadcast a vecinos |
| `void Spell::EffectScriptEffect(SpellEffIndex)` | `fn effect_script_effect(&mut self, idx: SpellEffIndex) { script_mgr.on_effect_script(self, idx) }` | Delega TODO al script registry, no switch interno |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical source at `/home/server/woltk-trinity-legacy/src/server/game/Spells/SpellEffects.cpp` (5,956 lines, **169 `void Spell::EffectXxx()` definitions** — verified via `grep -c "void Spell::Effect"`), plus the dispatch table declaration `NonDefaultConstructible<SpellEffectHandlerFn> SpellEffectHandlers[TOTAL_SPELL_EFFECTS]` at line 86 (151 entries indexed by `SpellEffectName`), the effect declarations in `Spell.h` lines ~291-422, and `SpellEffectInfo`/`SpellEffectName` in `SpellInfo.h` and `SpellDefines.h`.

**Empty-crate finding — CONFIRMED.** `crates/wow-spell/src/lib.rs` measures **exactly 0 lines** (verified via `wc -l`). The Rust workspace has **no implementation** of: `enum SpellEffectName` (151 variants), `enum SpellEffectHandleMode` (Launch/LaunchTarget/Hit/HitTarget), `struct SpellEffectInfo` proper (BasePoints, MiscValue, MiscValueB, TriggerSpell, RadiusEntry, Mechanic, ItemType, EffectAttributes, ImplicitTargets), the dispatch table (`SpellEffectHandlers[151]` array or equivalent `match`), the bisagra function `Spell::HandleEffects`, and **0 of 151 effect handlers**.

**Effects implemented.** **0 of ~151.** A search for every C++ effect handler name confirms zero analogs in Rust:
- Damage offensive: `EffectSchoolDMG`, `EffectEnvironmentalDMG`, `EffectPowerDrain`, `EffectHealthLeech`, `EffectWeaponDmg` (+ NoSchool/Normalized/Pct variants), `EffectAddExtraAttacks`, `EffectInstaKill` — none.
- Heal: `EffectHeal`, `EffectHealPct`, `EffectHealMechanical`, `EffectHealMaxHealth` — none.
- Power: `EffectEnergize`, `EffectEnergizePct`, `EffectPowerBurn` — none.
- Aura: `EffectApplyAura`, `EffectApplyAreaAura*` (6 variants), `EffectPersistentAA` — none.
- Movement: `EffectTeleportUnits`, `EffectTeleportUnitsWithVisualLoadingScreen`, `EffectTeleUnitsFaceCaster`, `EffectJump`, `EffectJumpDest`, `EffectLeap`, `EffectLeapBack`, `EffectKnockBack`, `EffectKnockBackDest`, `EffectPullTowardsDest`, `EffectPull`, `EffectCharge`, `EffectChargeDest`, `EffectJumpCharge`, `EffectMomentum` — none.
- Summon: `EffectSummonType` (the XL ~218-line discriminator), `EffectSummonPet`, `EffectSummonObject`, `EffectSummonObjectWild`, `EffectSummonChangeItem`, `EffectSummonPlayer` — none.
- Resurrect: `EffectResurrect`, `EffectResurrectNew`, `EffectSelfResurrect`, `EffectResurrectPet` — none.
- Dispel/Interrupt: `EffectDispel`, `EffectStealBeneficialBuff`, `EffectInterruptCast`, `EffectDispelMechanic` — none.
- Status: `EffectStuck`, `EffectThreat`, `EffectModifyThreatPercent` — represented-partial; `EffectSanctuary`, `EffectDualWield`, `EffectTaunt`, `EffectAddComboPoints`, `EffectScriptEffect`, `EffectDummy`, `EffectDistract`, `EffectModifyAuraStacks`, `EffectModifyCooldown`/`Cooldowns`/`CooldownsByCategory`, `EffectModifySpellCharges` — none.
- Item: `EffectCreateItem`, `EffectCreateItem2`, `EffectCreateRandomItem`, `EffectFeedPet`, `EffectEnchantItemPerm`, `EffectEnchantItemTmp`, `EffectEnchantItemPrismatic`, `EffectEnchantHeldItem`, `EffectDisEnchant`, `EffectMillItem`, `EffectProspecting` — none.
- Quest/Profession: `EffectQuestComplete`, `EffectQuestStart`, `EffectQuestRedirect`, `EffectLearnSpell`, `EffectUnlearnSpecialization`, `EffectLearnPetSpell`, `EffectLearnSkill`, `EffectTradeSkill`, `EffectProficiency`, `EffectUntrainTalents` — none.
- OpenLock/GO: `EffectOpenLock`, `EffectActivateObject`, `EffectSendEvent`, `EffectGameobjectDamage`, `EffectGameObjectRepair`, `EffectGameobjectSetDestructionState` — none.
- Pet/Charm: `EffectTameCreature`, `EffectDismissPet`, `EffectAddFarsight`, `EffectInebriate` — none.
- Combat misc: `EffectParry`, `EffectBlock`, `EffectReputation`, `EffectDuel` — none.
- Glyph/Talent: `EffectApplyGlyph` — none.
- Misc: `EffectForceCast`, `EffectTriggerSpell`, `EffectTriggerMissileSpell`, `EffectTriggerRitualOfSummoning`, `EffectPlayMovie`, `EffectPlayScene`, `EffectPlaySceneScriptPackage`, `EffectGiveHonor`, `EffectGrantBattlePetExperience`, `EffectForceDeselect`, `EffectPickPocket`, `EffectModifyAuraStacks` — none.

**Dispatch infrastructure missing.** No `SpellEffect` Rust enum, no `match spell_effect` switch, no `Spell::handle_effects` method (because no `Spell` struct exists either — see `spells-cast.md`). `crates/wow-world/src/handlers/spell.rs` contains a stub `execute_spell(spell_id, target_guid)` whose body — when traced through `session.rs` and the surrounding code — does not branch on `SpellEffect`, does not consult `SpellEffectInfo`, does not invoke any `Effect*` semantic. It is plumbing-only: name without payload.

**Effect-specific log packets missing.** `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` (consumed by SchoolDamage/WeaponDmg/HealthLeech), `SMSG_SPELL_HEAL_LOG` (Heal/HealPct/HealMaxHealth), `SMSG_SPELL_ENERGIZE_LOG` (Energize/EnergizePct/PowerDrain), `SMSG_SPELL_INSTAKILL_LOG` (InstaKill), `SMSG_SPELL_DISPELL_LOG` (Dispel/StealBeneficialBuff), `SMSG_DISPEL_FAILED` — none of these writers exist in `crates/wow-packet/src/packets/spell.rs` or anywhere else. Effects that depend on broadcast logs (visible damage numbers in the client) cannot send them.

**SpellEffectInfo data missing.** The `wow_data::SpellInfo` referenced from the handler exposes `cast_time_ms`, `recovery_time_ms`, `effective_cooldown_ms`, `has_cast_time` only. There is no `effects: Vec<SpellEffectInfo>` field, no `BasePoints`, no `MiscValue`, no `TriggerSpell`, no `Mechanic`, no `RadiusEntry`. Even if a handler existed, it would have nothing to read. The DB2 loader for SpellEffect.db2 (which carries 25+ fields per effect-row) is absent.

**DB2 / DB lookups required by effects.** None of the lookup paths exist:
- `Lock.db2` (EffectOpenLock) — missing
- `SummonProperties.db2` (EffectSummonType discriminator) — missing
- `SpellItemEnchantment.db2` (Effect Enchant*) — missing
- `GlyphSlot.db2` / `GlyphProperties.db2` (EffectApplyGlyph) — missing
- `SpellTargetPosition` SQL (EffectTeleportUnits) — missing
- `Movie.db2` / `CinematicSequences.db2` (EffectPlayMovie) — missing

**Script hooks missing.** `EffectScriptEffect` and `EffectDummy` rely entirely on `ScriptMgr::OnEffectScript`, `OnEffectDummy`, `OnEffectHit`, `OnEffectLaunch` callbacks. These dispatch points do not exist in Rust. Without them, the catch-all effects (used by every boss script and most class abilities with conditional logic) cannot fire any custom code.

**Handler→pipeline coupling missing.** In C++, `Spell::DoAllEffectOnTarget(TargetInfo*)` iterates per-target effect bitmask, calling `HandleEffects` per slot, and at the end materializes `m_damage`/`m_healing` into `Unit::DealDamage`/`Unit::HealBySpell`. The Rust workspace has **no `Spell` struct at all** (see `spells-cast.md` audit), so the orchestration that would consume effect handlers also does not exist. The two are interdependent: effect handlers cannot be implemented and tested without `Spell` to host them, and `Spell` cannot do anything meaningful without effect handlers.

**Worst divergence.** The effects sub-module is the **execution side** of the spell engine — what actually changes game state when a spell resolves. With 0 of 151 handlers and no dispatch infrastructure, **no spell in the game produces any consequence server-side** beyond an acknowledgment packet. The §9 task list (#SPELLS-EFFECTS.1 → #SPELLS-EFFECTS.80) covers ground-up implementation of every handler grouped by domain (damage, heal, summon, item, etc.), the dispatch table, the `SpellEffectInfo` data layer, the effect log packets, and the script hook infrastructure. Multiple individual tasks are XL (`EffectSummonType`, `EffectScriptEffect`, full handler matrix); the full sub-module is comparable in scope to porting all of `SpellEffects.cpp` (~6k C++ lines) and adjacent enum/data/wire support.
