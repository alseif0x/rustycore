# Migration: Entities / Unit

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/`
> **Rust target crate(s):** `crates/wow-world/` (live state, currently `WorldCreature` flat fields), `crates/wow-ai/` (legacy `CreatureAI` doubling as creature body), `crates/wow-combat/` (combat math — partial), `crates/wow-spell/` (auras — partial), `crates/wow-constants/` (`UnitFlags`, `MovementFlags`, `Powers`, `Stats`, `SpellSchools`)
> **Layer:** L4 (sub-modules — rooted under `entities.md`)
> **Status:** 🔧 broken (rewrite needed)
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

**Parent doc:** [`entities.md`](entities.md). **Sibling sub-docs:** [`entities-object.md`](entities-object.md) (the foundation Unit inherits from) · [`entities-vehicle.md`](entities-vehicle.md) (vehicle kit attached to Unit) · [`entities-transport.md`](entities-transport.md). **Cross-refs:** spell/aura mechanics in [`combat.md`](combat.md); future `entities-aura.md` will own the deep aura model.

---

## 1. Purpose

`Unit` is the combat-capable mid-tier in the entity hierarchy: every Player, Creature, Pet, Totem, Guardian, Minion, and TempSummon inherits from it. It owns the *universe* of in-combat state — health, mana/rage/energy/runic-power/focus and all other powers, the eight stat array, school resistances, immunities, the threat list, the combat manager, the aura container, the cast manager (`Spell*` currently being cast), the melee swing timers, the modifier stacks (flat / pct / total flat / total pct), the charm/possess relationship pair, the optional `Vehicle*` kit when the unit IS a vehicle, the optional `TransportBase` ref when the unit is RIDING one, the motion master, the speed array per `UnitMoveType`, and the entire `UnitFlags`/`UnitFlags2`/`UnitFlags3`/`UnitState` bag.

`Unit.cpp` at **13,620 lines** is the second-largest file in TrinityCore (after `Player.cpp`). The bulk is combat math (`DealDamage`, `CalculateMeleeDamage`, `RollMeleeOutcomeAgainst`, spell hit/crit, resilience, armor mitigation), aura lifecycle (`AddAura`, `RemoveAura`, `_ApplyAuraEffect`, periodic ticks), and threat/charm/vehicle housekeeping.

This sub-doc describes the entire `Unit/` directory and the contract the rest of the server has on Unit-level operations. Player-specific extensions live in a future `entities-player.md`; Creature-specific extensions in `entities-creature.md`; Pet/Totem in their own. The aura object model itself (the `Aura` / `AuraEffect` / `AuraApplication` triad) is owned by `Spells/Auras/` and gets its own doc — Unit only describes *how it talks to* that subsystem.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Unit/CharmInfo.cpp` | 283 | `prefix` |
| `game/Entities/Unit/CharmInfo.h` | 157 | `prefix` |
| `game/Entities/Unit/StatSystem.cpp` | 1311 | `prefix` |
| `game/Entities/Unit/Unit.cpp` | 13620 | `prefix` |
| `game/Entities/Unit/Unit.h` | 1953 | `prefix` |
| `game/Entities/Unit/UnitDefines.h` | 526 | `prefix` |
| `game/Entities/Unit/enuminfo_UnitDefines.cpp` | 619 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Unit/Unit.h` | 1953 | `Unit` declaration: stats, combat, threat, movement, auras, charm, vehicle, modifiers |
| `src/server/game/Entities/Unit/Unit.cpp` | 13620 | Combat math, aura application, death/resurrection, threat list, vehicle code, charm transitions |
| `src/server/game/Entities/Unit/UnitDefines.h` | 526 | `UnitFlags(1/2/3)`, `UnitState`, `MovementFlags`, `WeaponAttackType`, `Stats`, `Powers`, `UnitMoveType`, `DamageEffectType`, `VictimState`, `MeleeHitOutcome`, `SpellSchools`, `SpellSchoolMask`, `SpellImmunity`, `Mechanics` |
| `src/server/game/Entities/Unit/StatSystem.cpp` | 1311 | Stat recalc on aura/equip change (Player + Creature paths); `UpdateAttackPower`, `UpdateSpellPower`, `UpdateArmor`, `UpdateMaxHealth`, `UpdateMaxPower` |
| `src/server/game/Entities/Unit/CharmInfo.h` | 157 | `CharmInfo`, `UnitActionBarEntry`, action-bar slot metadata |
| `src/server/game/Entities/Unit/CharmInfo.cpp` | 283 | `CharmInfo` impl |
| `src/server/game/Entities/Unit/enuminfo_UnitDefines.cpp` | 619 | Generated enum reflection |

**Total in Unit/:** ~18,400 lines, of which ~13,600 are `Unit.cpp` alone.

---

## 3. Classes / Structs / Enums

### 3.1 Top-level

| Symbol | Kind | Purpose |
|---|---|---|
| `Unit` | class : `WorldObject` | Combat-capable entity base |
| `DamageInfo` | class | Per-hit damage breakdown (school mask, attack type, absorb, resist, block, original/final) |
| `HealInfo` | class | Per-hit heal breakdown |
| `ProcEventInfo` | class | Aggregates the actor/target/spell/damage/heal context for proc evaluation |
| `CalcDamageInfo` | struct | Melee damage calc workspace (hit info, target state, blocked, absorb) |
| `CleanDamage` | struct | Pre-mitigation damage buffer (used by `DealDamage`) |
| `SpellNonMeleeDamage` | struct | Spell damage event payload |
| `DispelableAura` | class | Aura packaged with a per-roll dispel chance and remaining charges |
| `CharmInfo` | struct | Pet/charm command + reaction state, action bar (10 slots), pet-name |
| `UnitActionBarEntry` | struct | One action-bar slot: spellId or command + active state |
| `AbstractFollower` | struct | Non-formation follower hookup |

### 3.2 Enums (UnitDefines.h)

| Enum | Values | Purpose |
|---|---|---|
| `UnitFlags` (`uint32`) | `SERVER_CONTROLLED`, `NON_ATTACKABLE`, `REMOVE_CLIENT_CONTROL`, `PLAYER_CONTROLLED`, `RENAME`, `PREPARATION`, `IMMUNE_TO_PC`, `IMMUNE_TO_NPC`, `LOOTING`, `PET_IN_COMBAT`, `PVP_ATTACKABLE`, `SILENCED`, `PACIFIED`, `STUNNED`, `IN_COMBAT`, `DISARMED`, `CONFUSED`, `FLEEING`, `POSSESSED`, `NOT_SELECTABLE`, `SKINNABLE`, `MOUNT`, `SHEATHE`, `DISABLE_TURN`, `IMMUNE`, ~32 total | Replicated unit-state bitmask |
| `UnitFlags2` (`uint32`) | `FEIGN_DEATH`, `HIDE_BODY`, `IGNORE_REPUTATION`, `COMPREHEND_LANG`, `MIRROR_IMAGE`, `INSTANTLY_APPEAR_MODEL`, `FORCE_MOVEMENT`, `DISARM_OFFHAND`, `DISABLE_PRED_STATS`, `DISARM_RANGED`, `REGENERATE_POWER`, `RESTRICT_PARTY_INTERACTION`, `PREVENT_SPELL_CLICK`, `INTERACT_WHILE_HOSTILE`, `CANNOT_TURN`, `UNK2`, `PLAY_DEATH_ANIM`, `ALLOW_CHEAT_SPELLS`, ~24 total | Extended unit flags |
| `UnitFlags3` (`uint32`) | `UNK1`, `UNCONSCIOUS_ON_DEATH`, `ALLOW_MOUNTED_COMBAT`, `GARRISON_PET`, `UI_CAN_GET_POSITION`, `AI_OBSTACLE`, `ALTERNATIVE_DEFAULT_LANGUAGE`, ... | Newer (Cataclysm+) flags retained in 3.4.3 client |
| `UnitState` (`uint32`, bitmask) | `DIED`, `MELEE_ATTACKING`, `CHARMED`, `STUNNED`, `ROAMING`, `CHASE`, `FOCUSING`, `FLEEING`, `IN_FLIGHT`, `FOLLOW`, `ROOT`, `CONFUSED`, `DISTRACTED`, `ISOLATED`, `ATTACK_PLAYER`, `CASTING`, `POSSESSED`, `CHARGING`, `JUMPING`, `MOVE`, `ROTATING`, `EVADE`, `ROAMING_MOVE`, `CONFUSED_MOVE`, `FLEEING_MOVE`, `CHASE_MOVE`, `FOLLOW_MOVE`, `IGNORE_PATHFINDING`, `FOLLOW_FORMATION_MOVE` | Server-only state bitmask (NOT replicated) |
| `MovementFlags` (`uint32`) | `FORWARD`, `BACKWARD`, `STRAFE_LEFT`, `STRAFE_RIGHT`, `LEFT`, `RIGHT`, `PITCH_UP`, `PITCH_DOWN`, `WALKING`, `ON_TRANSPORT`, `DISABLE_GRAVITY`, `ROOT`, `JUMPING_OR_FALLING`, `FALLING_FAR`, `PENDING_STOP`, `PENDING_STRAFE_STOP`, `PENDING_FORWARD`, `PENDING_BACKWARD`, `PENDING_STRAFE_LEFT`, `PENDING_STRAFE_RIGHT`, `PENDING_ROOT`, `SWIMMING`, `ASCENDING`, `DESCENDING`, `CAN_FLY`, `FLYING`, `SPLINE_ELEVATION`, `SPLINE_ENABLED`, `WATERWALKING`, `FALLING_SLOW`, `HOVER` | Wire-replicated movement state |
| `Powers` (`uint8`) | `MANA=0`, `RAGE=1`, `FOCUS=2`, `ENERGY=3`, `HAPPINESS=4`, `RUNES=5`, `RUNIC_POWER=6`, `SOUL_SHARDS=7`, `LUNAR_POWER=8`, `HOLY_POWER=9`, `ALTERNATE`, `MAELSTROM`, `CHI`, `INSANITY`, `BURNING_EMBERS`, `DEMONIC_FURY`, `ARCANE_CHARGES`, `FURY`, `PAIN`, `MAX_POWERS=10` (3.4.3 cap), `ALL_POWERS=127`, `HEALTH=-2` | Power resource type |
| `Stats` (`uint8`) | `STRENGTH=0`, `AGILITY=1`, `STAMINA=2`, `INTELLECT=3`, `SPIRIT=4`, `MAX_STATS=5` | Primary stat indices |
| `WeaponAttackType` (`uint8`) | `BASE_ATTACK=0`, `OFF_ATTACK=1`, `RANGED_ATTACK=2`, `MAX_ATTACK=3` | Attack-slot enum |
| `UnitMoveType` (`uint8`) | `WALK=0`, `RUN=1`, `RUN_BACK=2`, `SWIM=3`, `SWIM_BACK=4`, `TURN_RATE=5`, `FLIGHT=6`, `FLIGHT_BACK=7`, `PITCH_RATE=8`, `MAX_MOVE_TYPE=9` | Speed-array index |
| `SpellSchools` (`uint8`) | `NORMAL=0` (physical), `HOLY=1`, `FIRE=2`, `NATURE=3`, `FROST=4`, `SHADOW=5`, `ARCANE=6`, `MAX_SPELL_SCHOOL=7` | Damage type |
| `SpellSchoolMask` (`uint32` bitmask) | `NORMAL=0x01` … `ARCANE=0x40`; `MAGIC = HOLY\|FIRE\|NATURE\|FROST\|SHADOW\|ARCANE`; `ALL = 0x7F` | Bitmask over schools |
| `SpellImmunity` | `EFFECT=0`, `STATE=1`, `SCHOOL=2`, `DAMAGE=3`, `DISPEL=4`, `MECHANIC=5`, `ID=6`, `MAX_SPELL_IMMUNITY=7` | Immunity bucket — keys into `m_spellImmune[MAX_SPELL_IMMUNITY]` |
| `Mechanics` (`uint32`) | `NONE`, `CHARM`, `DISORIENTED`, `DISARM`, `DISTRACT`, `FEAR`, `GRIP`, `ROOT`, `SLOW_ATTACK`, `SILENCE`, `SLEEP`, `SNARE`, `STUN`, `FREEZE`, `KNOCKOUT`, `BLEED`, `BANDAGE`, `POLYMORPH`, `BANISH`, `SHIELD`, `SHACKLE`, `MOUNT`, `INFECTED`, `TURN`, `HORROR`, `INVULNERABILITY`, `INTERRUPT`, `DAZE`, `DISCOVERY`, `IMMUNE_SHIELD`, `SAPPED`, `ENRAGED`, `WOUNDED`, `INFECTED2`, `INFECTED3`, `TAUNTED`, `MAX_MECHANIC=37` | Crowd-control taxonomy |
| `UnitModifierFlatType` | `BASE_VALUE=0`, `BASE_PCT_EXCLUDE_CREATE=1`, `TOTAL_VALUE=2`, `MODIFIER_TYPE_FLAT_END=3` | Modifier slot for flat stack |
| `UnitModifierPctType` | `BASE_PCT=0`, `TOTAL_PCT=1`, `MODIFIER_TYPE_PCT_END=2` | Modifier slot for pct stack |
| `UnitMods` (`uint32`) | `STAT_STRENGTH..STAT_INTELLECT`, `HEALTH`, `MANA..PAIN` (all power types), `ARMOR`, `RESISTANCE_HOLY..ARCANE`, `ATTACK_POWER`, `ATTACK_POWER_RANGED`, `DAMAGE_MAINHAND`, `DAMAGE_OFFHAND`, `DAMAGE_RANGED`, `MAX_UNIT_MODS` | Aggregated stat-mod bucket key |
| `WeaponDamageRange` | `MINDAMAGE`, `MAXDAMAGE` | Index into damage range arrays |
| `DamageEffectType` (`uint8`) | `DIRECT_DAMAGE=0`, `SPELL_DIRECT_DAMAGE=1`, `DOT=2`, `HEAL=3`, `NODAMAGE=4`, `SELF_DAMAGE=5` | `DealDamage` taxonomy |
| `VictimState` | `INTACT`, `HIT`, `DODGE`, `PARRY`, `INTERRUPT`, `BLOCKS`, `EVADES`, `IS_IMMUNE`, `DEFLECTS` | Combat-log target outcome |
| `MeleeHitOutcome` | `MISS`, `DODGE`, `BLOCK`, `PARRY`, `GLANCING`, `CRIT`, `CRUSHING`, `NORMAL`, `BLOCK_CRIT` | Melee roll result |
| `UnitStandStateType` (`uint8`) | `STAND`, `SIT`, `SIT_CHAIR`, `SLEEP`, `SIT_LOW_CHAIR`, `SIT_MEDIUM_CHAIR`, `SIT_HIGH_CHAIR`, `DEAD`, `KNEEL`, `SUBMERGED` | Posture |
| `SheathState` (`uint8`) | `UNARMED`, `MELEE`, `RANGED` | Weapon visual state |
| `UnitVisFlags` (`uint8`) | `INVISIBLE`, `STEALTHED`, `UNTRACKABLE`, `UNK4..6`, `ALL=0xFF` | Visibility bits in `UNIT_FIELD_BYTES_1` |
| `UnitPVPStateFlags` (`uint8`) | `NONE`, `PVP`, `FFA_PVP`, `SANCTUARY`, ... | PvP-state bits |
| `UnitPetFlag` (`uint8`) | `CAN_BE_RENAMED`, `CAN_BE_ABANDONED` | Pet UI flags |
| `AnimTier` (`uint8`) | `Ground`, `Swim`, `Hover`, `Fly`, `Submerged` | Animation tier |
| `CharmType` (`uint8`) | `CONVERT`, `POSSESS`, `CHARM`, `CONVERT_PET`, `AURA` | Charm transition flavor |

### 3.3 Containers held by `Unit`

| Member | Type | Purpose |
|---|---|---|
| `m_attackers` | `AttackerSet` (std::set<Unit*>) | Reverse-pointer set: who is attacking *me* |
| `m_attacking` | `Unit*` | Whom I am currently attacking |
| `m_state` | `uint32` (UnitState bitmask) | Server-only state bitmask |
| `m_unitTypeMask` | `uint32` (UNIT_MASK_*) | Subtype tag (Summon/Guardian/Pet/HunterPet/Totem/Vehicle/...) |
| `m_threatManager` | `ThreatManager` | Threat list + tank order |
| `m_combatManager` | `CombatManager` | Active combat references |
| `m_motion` | `MotionMaster*` | Movement-generator stack |
| `m_charmInfo` | `std::unique_ptr<CharmInfo>` | Pet/charm command bar (when charmed/owned) |
| `m_charmer` | `Unit*` | Who controls me |
| `m_charmed` | `Unit*` | Whom I control |
| `m_vehicleKit` | `Vehicle*` | Vehicle seat manager when I AM a vehicle |
| `m_vehicle` | `Vehicle*` | Vehicle I am riding |
| `m_spellImmune` | `SpellImmuneContainer m_spellImmune[MAX_SPELL_IMMUNITY=7]` | Per-bucket immunity multimaps (Effect/State/School/Damage/Dispel/Mechanic/Id) |
| `m_speed_rate` | `float[MAX_MOVE_TYPE=9]` | Per-move-type speed multipliers |
| `m_baseAttackTime` | `uint32[MAX_ATTACK=3]` | Per-attack-type base swing time |
| `m_attackTimer` | `uint32[MAX_ATTACK=3]` | Per-attack-type remaining swing |
| `m_unitAuras` (`m_appliedAuras`, `m_ownedAuras`) | `AuraApplicationMap`, `AuraMap` | Active auras + applications |
| `m_unitData` | `UpdateField<UF::UnitData>` (the replicated payload) | Tracks all replicated stats/flags |

---

## 4. Critical public methods / functions

### 4.1 Health / power / stats

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::GetHealth()` / `GetMaxHealth()` | Read replicated HP | `m_unitData->Health` / `MaxHealth` |
| `Unit::SetHealth(uint64)` / `SetMaxHealth(uint64)` | Write HP / clamp; flips dirty bit | `SetUpdateFieldValue(UnitData::Health)` |
| `Unit::ModifyHealth(int64) -> int64` | Add/sub clamped; returns delta | `SetHealth` |
| `Unit::IsFullHealth()` / `HealthBelowPct(pct)` / `HealthAbovePct(pct)` / `HealthBelowPctDamaged(pct, dmg)` / `HealthAbovePctHealed(pct, heal)` | Predicates | `CalculatePct` |
| `Unit::GetHealthPct()` | `100 * health / max_health` | — |
| `Unit::CountPctFromMaxHealth(pct)` / `CountPctFromCurHealth(pct)` | Percent-of math used by spells | — |
| `Unit::TriggerOnHealthChangeAuras(oldVal, newVal)` | Fires `SPELL_AURA_PROC_TRIGGER_SPELL` on HP threshold crossings | aura iteration |
| `Unit::SetFullHealth()` | `SetHealth(GetMaxHealth())` | — |
| `Unit::GetPowerType()` / `SetPowerType(Powers, sendUpdate)` | Active power resource | `UpdateDisplayPower` |
| `Unit::GetPower(Powers)` / `GetMaxPower(Powers)` / `GetMinPower(Powers)` | Power read | `m_unitData->Power[]` |
| `Unit::SetPower(Powers, int32, withPowerUpdate)` / `SetMaxPower(Powers, int32)` | Power write | `SetUpdateFieldValue(UnitData::Power)` |
| `Unit::ModifyPower(Powers, int32, withPowerUpdate) -> int32` | Add/sub clamped | `SetPower` |
| `Unit::TriggerOnPowerChangeAuras(power, oldVal, newVal)` | Power-threshold proc | — |
| `Unit::GetStat(Stats)` / `SetStat(Stats, int32)` | Primary stat read/write | `m_unitData->Stats[]` |
| `Unit::GetArmor()` / `SetArmor(int32, int32 bonusVal)` | Armor (resistance NORMAL) | `GetResistance(SPELL_SCHOOL_NORMAL)` |
| `Unit::GetResistance(SpellSchools)` / `GetResistance(SpellSchoolMask)` / `SetResistance(SpellSchools, int32)` | Per-school resistance | `m_unitData->Resistances[]` |
| `Unit::CalculateAverageResistReduction(caster, schoolMask, victim, spellInfo)` (static) | School-resist mitigation factor | — |
| `Unit::HandleStatFlatModifier(UnitMods, UnitModifierFlatType, float, bool apply)` / `HandleStatPctModifier(UnitMods, UnitModifierPctType, float, bool apply)` | Apply/remove flat or pct stat modifier | `UpdateAllStats` cascade |
| `Unit::GetTotalStatValue(Stats stat) const` / `GetTotalAuraModifier(AuraType)` / `GetTotalAuraMultiplier(AuraType)` / `GetMaxPositiveAuraModifier(AuraType)` / `GetMaxNegativeAuraModifier(AuraType)` | Aggregated modifier reads | iterate `m_modAuras[]` |
| `Unit::UpdateAllStats()` (virtual; Player + Creature override) | Recompute all derived stats after a modifier change | `UpdateArmor`, `UpdateMaxHealth`, `UpdateMaxPower`, `UpdateAttackPower`, `UpdateSpellPower`, `UpdateResistances` |

### 4.2 Damage / heal / death

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::DealDamage(attacker, victim, damage, cleanDamage, damagetype, schoolMask, spellProto, durabilityLoss) -> uint32` (static) | **The combat damage entry point.** Applies absorb/resist, modifies victim HP, fires OnDamage auras, calls `Kill` if HP→0 | `DealDamageMods`, `Kill`, `Aura::CallScriptDamageHandlers` |
| `Unit::DealDamageMods(attacker, victim, damage, absorb)` (static) | Pre-mitigation modification (PvP rules, GM toggles) | — |
| `Unit::Kill(attacker, victim, durabilityLoss, skipSettingDeathState)` (static) | Death routine: drops loot, fires script, despawns or sets ghost | `KillRewarder::Reward`, `Creature::SetLootRecipient`, `Player::DurabilityLossAll` |
| `Unit::KillSelf(durabilityLoss, skipSettingDeathState)` | `Unit::Kill(this, this, ...)` | — |
| `Unit::DealHeal(HealInfo&)` (static) | Apply heal, clamp to MaxHealth, fire OnHeal auras | `Aura::CallScriptHealingHandlers`, `ModifyHealth` |
| `Unit::HealBySpell(HealInfo&, critical)` | Spell-driven heal that includes proc + combat-log emit | `DealHeal`, `SendHealSpellLog` |
| `Unit::CalculateMeleeDamage(victim, CalcDamageInfo*, attackType)` | Roll melee outcome, compute damage, populate `CalcDamageInfo` | `RollMeleeOutcomeAgainst`, `MeleeDamageBonusDone/Taken` |
| `Unit::DealMeleeDamage(CalcDamageInfo*, durabilityLoss)` | Apply the populated `CalcDamageInfo` | `DealDamage`, `SendAttackStateUpdate` |
| `Unit::AttackerStateUpdate(victim, attType, extra)` | Per-tick check: if swing timer ready, swing | `CalculateMeleeDamage`, `DealMeleeDamage` |
| `Unit::DoMeleeAttackIfReady()` | Tick callback driving auto-attack | `AttackerStateUpdate(BASE_ATTACK)`, `AttackerStateUpdate(OFF_ATTACK)` |
| `Unit::HandleProcExtraAttackFor(victim, count)` | Extra-attack procs (e.g. Sweeping Strikes) | `AttackerStateUpdate(extra=true)` |
| `Unit::CalculateSpellDamageTaken(damageInfo, damage, spellInfo, attackType, crit, blocked, spell)` | Spell damage post-mitigation calc | absorb/resist auras |
| `Unit::DealSpellDamage(damageInfo, durabilityLoss)` | Apply spell damage payload | `DealDamage` |
| `Unit::ApplyResilience(victim, &damage)` (static) | PvP resilience mitigation | — |
| `Unit::CalculateAOEAvoidance(damage, schoolMask, npcCaster) -> int32` | AoE avoidance modifier | — |
| `Unit::SetDeathState(DeathState)` | Transition Alive→JustDied→Corpse→Dead→Ghost | `RemoveAllAuras`, `SetUnitFlag(UNIT_FLAG_NOT_SELECTABLE)`, `OnDeath` script |

### 4.3 Combat / threat / attack

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::Attack(victim, meleeAttack)` | Begin attacking; sets `m_attacking` | `SendMeleeAttackStart`, `ThreatManager::AddThreat` |
| `Unit::AttackStop()` | Stop attacking | `SendMeleeAttackStop` |
| `Unit::CombatStop(includingCast, mutualPvP)` | Drop combat, clear threat, optionally interrupt cast | `m_combatManager.EndAllCombat`, `m_threatManager.ClearAllThreat` |
| `Unit::CombatStopWithPets(includingCast)` | `CombatStop` for self + minions | iterate `m_Controlled` |
| `Unit::IsInCombat()` / `IsEngaged()` | Combat predicates | `m_combatManager.IsInCombat()` |
| `Unit::EngageWithTarget(who)` | Add to threat list if applicable, otherwise just enter combat | `ThreatManager::AddThreat` or `CombatManager::SetInCombatWith` |
| `Unit::AtTargetAttacked(target, canInitialAggro)` | Reaction to being hit | — |
| `Unit::GetCombatManager()` / `GetThreatManager()` | Subsystem accessors | — |
| `Unit::GetVictim()` / `EnsureVictim()` | Currently-attacked unit | `m_attacking` |
| `Unit::SelectNearbyTarget(exclude, dist) -> Unit*` | Pick a hostile within range | grid query |
| `Unit::SendMeleeAttackStart(victim)` / `SendMeleeAttackStop(victim)` | Wire events | `SMSG_ATTACK_START` / `SMSG_ATTACK_STOP` |
| `Unit::SendAttackStateUpdate(HitInfo, target, swingType, schoolMask, damage, absorb, resist, targetState, blocked)` | Combat-log packet | `SMSG_ATTACKERSTATEUPDATE` |
| `Unit::ValidateAttackersAndOwnTarget()` | Reconcile `m_attackers` / `m_attacking` invariants | — |
| `Unit::StopAttackFaction(faction_id)` | Disengage everyone of a given faction | iterate attackers |
| `Unit::AddUnitState(uint32)` / `HasUnitState` / `ClearUnitState` | Server-only state bitmask | `m_state` |
| `Unit::HasUnitTypeMask(uint32)` / `IsSummon`/`IsGuardian`/`IsPet`/`IsHunterPet`/`IsTotem`/`IsVehicle` | Subtype predicates | `m_unitTypeMask` |

### 4.4 Combat math (rolls + chances)

| Symbol | Purpose |
|---|---|
| `Unit::RollMeleeOutcomeAgainst(victim, attType) -> MeleeHitOutcome` | The big roll: miss → dodge → parry → glancing → block → crit → crushing → normal |
| `Unit::MeleeSpellMissChance(victim, attType, spellInfo) -> float` | Spell miss chance for a melee attack |
| `Unit::MeleeSpellHitResult(victim, spellInfo) -> SpellMissInfo` | Spell hit outcome |
| `Unit::GetUnitDodgeChance(attType, victim)` / `GetUnitParryChance` / `GetUnitBlockChance` / `GetUnitMissChance` | Per-stat avoidance chances |
| `Unit::GetUnitCriticalChanceDone(attType)` / `GetUnitCriticalChanceTaken(attacker, attType, critDone)` / `GetUnitCriticalChanceAgainst(attType, victim)` | Crit chance |
| `Unit::GetMechanicResistChance(spellInfo) -> int32` | Mechanic-based resist |
| `Unit::CanUseAttackType(attackType) -> bool` | Disarm / off-hand availability |
| `Unit::GetWeaponProcChance() / GetPPMProcChance(speed, ppm, spellProto)` | Weapon proc rate |

### 4.5 Auras

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::AddAura(spellId, target) -> Aura*` | High-level aura application | `AddAura(SpellInfo, ...)` |
| `Unit::AddAura(spellInfo, effMask, target) -> Aura*` | Lower-level | `_AddAura(UnitAura*, caster)` |
| `Unit::_AddAura(UnitAura*, caster)` | Insert into `m_ownedAuras` | — |
| `Unit::RemoveAura(spellId, casterGUID, reqEffMask, removeMode)` (overloads: by iterator / by `AuraApplication*` / by `Aura*`) | Aura removal | `_UnapplyAura` |
| `Unit::RemoveAurasByType(AuraType, check, removeMode)` | Bulk-remove by type with predicate | iterate `m_modAuras[]` |
| `Unit::RemoveAllAuras()` / `RemoveAllAurasOnDeath()` / `RemoveAllAurasOnEvade()` | Bulk wipe | — |
| `Unit::GetAuraEffect(spellId, effIndex, casterGUID) -> AuraEffect*` | Look up specific aura effect | — |
| `Unit::HasAura(spellId, casterGUID, itemCasterGUID, reqEffMask) -> bool` | Predicate | — |
| `Unit::HasAuraType(AuraType) -> bool` | Type predicate | — |
| `Unit::GetTotalAuraModifier(AuraType)` / `GetTotalAuraMultiplier(AuraType)` / `GetMaxPositiveAuraModifier(AuraType)` / `GetMaxNegativeAuraModifier(AuraType)` | Aggregate stat-mod reads | — |
| `Unit::GetAppliedAuras()` / `GetOwnedAuras()` | Container access | — |
| `Unit::SendAuraUpdate(slot, remove)` | Replicate to client | `SMSG_AURA_UPDATE` |
| `Unit::ProcSkillsAndAuras(actor, target, typeMaskActor, typeMaskTarget, spellTypeMask, spellPhaseMask, hitMask, spell, damageInfo, healInfo)` (static) | Run proc table | — |
| `Unit::TriggerAurasProcOnEvent(...)` | Iterate proc auras and execute | — |
| `Unit::GetProcAurasTriggeredOnEvent(out, procAuras, eventInfo)` | Collect proc auras matching event | — |

### 4.6 Spell casting

| Symbol | Purpose |
|---|---|
| `Unit::CastSpell(target, spellId, args)` (overloads: `Unit*`, `WorldObject*`, `Position`, `SpellCastTargets`) | Cast wrapper; constructs `Spell*` and calls `prepare` |
| `Unit::CastCustomSpell(...)` | Cast with custom basepoints |
| `Unit::GetCurrentSpell(CurrentSpellTypes) -> Spell*` | Active spell slot (Generic/Melee/AutoRepeat/Channeled) |
| `Unit::HasUnitFlag(UNIT_FLAG_PREPARATION)` / `IsNonMeleeSpellCast(...)` | Cast-state predicates |
| `Unit::InterruptNonMeleeSpells(withDelayed, spellId, withInstant)` | Interrupt active casts |
| `Unit::FinishSpell(CurrentSpellTypes, ok)` | Spell finalize hook |
| `Unit::SpellBaseDamageBonusDone(SpellSchoolMask)` / `SpellBaseHealingBonusDone(SpellSchoolMask)` | Damage/heal bonus contribution |
| `Unit::SpellDamageBonusDone(...)` / `SpellDamageBonusTaken(...)` | Per-cast spell damage modifiers |
| `Unit::SpellHealingBonusDone(...)` / `SpellHealingBonusTaken(...)` | Per-cast spell heal modifiers |
| `Unit::MeleeDamageBonusDone(...)` / `MeleeDamageBonusTaken(...)` | Per-swing melee damage modifiers |
| `Unit::SpellCritChanceDone(...)` / `SpellCritChanceTaken(...)` | Spell crit chance |

### 4.7 Charm / possess

| Symbol | Purpose |
|---|---|
| `Unit::SetCharm(target, apply)` | Attach/detach `m_charmed` link |
| `Unit::SetCharmedBy(charmer, CharmType, aurApp)` | The full charm transition (move into possessor's vehicle/control, swap motion master) |
| `Unit::RemoveCharmedBy(charmer)` | Reverse |
| `Unit::IsCharmed()` / `isPossessed()` / `isPossessedByPlayer()` / `isPossessing()` / `isPossessing(u)` | Predicates |
| `Unit::GetCharmer()` / `GetCharmed()` / `GetCharmerOrOwner()` | Reverse/forward lookups |
| `Unit::GetCharmInfo()` / `InitCharmInfo()` / `DeleteCharmInfo()` | Charm command-bar lifecycle |
| `Unit::RemoveCharmAuras()` | Clean up on charm end |

### 4.8 Vehicle / transport base

| Symbol | Purpose |
|---|---|
| `Unit::SetVehicle(Vehicle*)` | Assign the vehicle kit (when this Unit IS a vehicle) |
| `Unit::GetVehicleKit()` | Accessor |
| `Unit::GetVehicle()` | The vehicle this unit is RIDING |
| `Unit::IsVehicle()` | `m_unitTypeMask & UNIT_MASK_VEHICLE` |
| `Unit::CreateVehicleKit(id, creatureEntry, loading)` | Build the kit from `VehicleEntry` |
| `Unit::RemoveVehicleKit(onRemoveFromWorld)` | Tear down |
| `Unit::SendSetVehicleRecId(uint32)` | `SMSG_SET_VEHICLE_REC_ID` |
| `Unit::EnterVehicle(base, seatId)` | Ride |
| `Unit::ExitVehicle(exitPosition)` | Dismount |
| `Unit::ChangeSeat(seatId, next)` | Move within vehicle |

### 4.9 Movement / speed

| Symbol | Purpose |
|---|---|
| `Unit::GetSpeed(UnitMoveType)` / `GetSpeedRate(UnitMoveType)` | Speed read |
| `Unit::SetSpeed(UnitMoveType, float)` / `SetSpeedRate(UnitMoveType, float)` | Speed write + emit `SMSG_FORCE_*_SPEED_CHANGE` |
| `Unit::UpdateSpeed(UnitMoveType)` | Recompute from base + auras |
| `Unit::GetMotionMaster()` | `MotionMaster*` |
| `Unit::SetWalk(bool)` / `SetDisableGravity(bool)` / `SetSwim(bool)` / `SetCanFly(bool)` / `SetWaterWalking(bool)` / `SetFeatherFall(bool)` / `SetHover(bool)` | Movement-mode toggles + replication |
| `Unit::Mount(mountId, vehicleId, creatureEntry)` / `Dismount()` | Mount lifecycle |
| `Unit::JumpTo(...)` / `KnockbackFrom(...)` | Spline-driven impulse |

### 4.10 Immunities / school masks

| Symbol | Purpose |
|---|---|
| `Unit::ApplySpellImmune(spellId, SpellImmunity, type, apply)` | Add/remove an immunity entry |
| `Unit::GetSchoolImmunityMask() const -> uint32` | OR of all `SCHOOL` immunities |
| `Unit::GetDamageImmunityMask() const -> uint32` | OR of all `DAMAGE` immunities |
| `Unit::GetMechanicImmunityMask() const -> uint64` | OR of all `MECHANIC` immunities |
| `Unit::IsImmunedToSpell(spellInfo, caster, requireImmunityPurgesEffectAttribute)` | Per-spell predicate |
| `Unit::IsImmunedToSpellEffect(spellInfo, eff, caster)` | Per-effect predicate |
| `Unit::IsImmunedToDamage(SpellSchoolMask)` | School/damage |
| `Unit::IsImmunedToDamageOrSchool(spellInfo)` | Combined |

### 4.11 Modifiers / stat aggregation

| Symbol | Purpose |
|---|---|
| `Unit::ApplyStatBuff(Stats, float, bool apply)` | Apply primary stat buff |
| `Unit::ApplyStatPctBuff(Stats, float, bool apply)` | Pct variant |
| `Unit::HandleStatFlatModifier(UnitMods, UnitModifierFlatType, float, bool apply)` | Generic flat |
| `Unit::HandleStatPctModifier(UnitMods, UnitModifierPctType, float, bool apply)` | Generic pct |
| `Unit::GetModifierValue(UnitMods, UnitModifierFlatType)` / `GetPctModifierValue(UnitMods, UnitModifierPctType)` | Read |
| `Unit::GetTotalAuraModValue(UnitMods)` | Aggregated (base + flat + base_pct + total + total_pct) |
| `Unit::UpdateMaxHealth()` (virtual; Player/Creature override) | Recompute from STAMINA + UnitMods |
| `Unit::UpdateMaxPower(Powers)` | Same for powers |
| `Unit::UpdateAttackPower()` / `UpdateAttackPowerAndDamage()` / `UpdateRangedAttackPowerAndDamage()` | AP recompute |
| `Unit::UpdateArmor()` | Armor recompute |
| `Unit::UpdateResistances(uint32 school)` | Per-school recompute |

### 4.12 Replicated flag bags

| Symbol | Purpose |
|---|---|
| `Unit::HasUnitFlag(UnitFlags)` / `SetUnitFlag` / `RemoveUnitFlag` / `ReplaceAllUnitFlags` | Manipulate `UnitData::Flags` |
| `Unit::HasUnitFlag2/3` / `SetUnitFlag2/3` / `RemoveUnitFlag2/3` / `ReplaceAllUnitFlags2/3` | Same for `Flags2` / `Flags3` |
| `Unit::GetEmoteState()` / `SetEmoteState(Emote)` | `UnitData::EmoteState` |
| `Unit::GetSheath()` / `SetSheath(SheathState)` | `UnitData::SheatheState` |
| `Unit::GetFaction()` / `SetFaction(uint32)` | `UnitData::FactionTemplate` |
| `Unit::HasNpcFlag(NPCFlags)` / `SetNpcFlag` / `RemoveNpcFlag` etc. | `UnitData::NpcFlags[0]` |
| `Unit::IsVendor()` / `IsTrainer()` / `IsQuestGiver()` / `IsGossip()` / `IsTaxi()` / `IsBattleMaster()` / `IsBanker()` / `IsInnkeeper()` / `IsSpiritHealer()` / `IsTabardDesigner()` / `IsAuctioner()` / `IsArmorer()` | NPC-flag predicates |
| `Unit::GetStandState()` / `SetStandState(UnitStandStateType, animKitID)` | `UnitData::StandState` |
| `Unit::GetVisFlags()` / `SetVisFlag` / `RemoveVisFlag` / `ReplaceAllVisFlags` | `UnitData::VisFlags` |
| `Unit::GetAnimTier()` / `SetAnimTier(AnimTier, notifyClient)` | `UnitData::AnimTier` |
| `Unit::IsMounted()` / `GetMountDisplayId()` / `SetMountDisplayId(uint32)` | `UnitData::MountDisplayID` + `UNIT_FLAG_MOUNT` |
| `Unit::GetLevel()` / `SetLevel(uint8, sendUpdate)` | `UnitData::Level` |
| `Unit::GetRace()` / `SetRace(uint8)` / `GetClass()` / `SetClass(uint8)` / `GetGender()` / `SetGender(Gender)` | Lineage |

---

## 5. Module dependencies

**Depends on:**
- `Object/` — `Unit : WorldObject : Object`. All `UpdateField<T>` plumbing comes from here.
- `Spells/` — `SpellInfo`, `Spell`, `SpellMgr`. `CastSpell` constructs a `Spell` and calls `prepare`. The aura object model (`Aura`, `AuraEffect`, `AuraApplication`) lives in `Spells/Auras/`.
- `Combat/` — `ThreatManager`, `CombatManager`, `HostileRefMgr`. Members of `Unit`.
- `Movement/` — `MotionMaster`, `MovementGenerator`, `MoveSpline`, `Spline`. `m_motion` is a `MotionMaster*`.
- `AI/` — `UnitAI`, `CreatureAI`, `PlayerAI`. The AI controller is logically a sibling but Unit holds `m_unitAI`.
- `DataStores/` — `ChrClassesEntry`, `ChrRacesEntry`, `FactionTemplateEntry`, `EmotesEntry`, `MountCapabilityEntry`, `PowerTypeEntry`, `VehicleEntry`.
- `Loot/` — `Unit::Kill` triggers loot via `KillRewarder`.
- `Conditions/` — used by spell targeting filters.
- `Networking / Packets` — `SMSG_ATTACK_START/STOP`, `SMSG_ATTACKERSTATEUPDATE`, `SMSG_AURA_UPDATE`, `SMSG_FORCE_*_SPEED_CHANGE`, `SMSG_AI_REACTION`, `SMSG_DURABILITY_DAMAGE_DEATH`, `SMSG_PARTY_KILL_LOG`.

**Depended on by:**
- `Player`, `Creature`, `Pet`, `Guardian`, `Minion`, `TempSummon`, `Totem` — direct subclasses.
- `Vehicle` — holds the riding unit + vehicle kit linkage.
- Every gameplay handler that operates on a target (combat, spell, loot, trade, group, taxi, gossip, vendor, trainer, banker, innkeeper, battlemaster, auctioneer, repair).
- `BattlegroundScore`, `OutdoorPvP`, `Battlefield` — scoring and PvP-state logic.
- `Achievement`, `Quest`, `Reputation` — kill credit, quest credit, faction standing.
- `Scripts` — `OnUnitDeath`, `OnDamageReceived`, `OnHeal`, `OnSpellCast`, etc.

---

## 6. SQL / DB queries (if any)

`Unit` itself does not own persistence — `Player::SaveToDB` and `Creature::SaveToDB` (in subclass docs) handle row writes. Unit consumes:

| Statement | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER_AURAS` | Restore active auras on Player login | character |
| `CHAR_DEL_CHARACTER_AURA` / `CHAR_INS_CHARACTER_AURA` | Player aura persistence | character |
| `CHAR_SEL_CHAR_PET_AURAS` / `CHAR_INS_PET_AURA` | Pet aura persistence | character |

| DBC/DB2 store | What it loads | Read by |
|---|---|---|
| `ChrClassesStore` | ChrClasses.db2 | `UpdateAllStats` (Player + Pet path) |
| `ChrRacesStore` | ChrRaces.db2 | starting stat templates |
| `FactionTemplateStore` | FactionTemplate.db2 | `IsHostileTo`, faction-template resolution |
| `EmotesStore` | Emotes.db2 | `HandleEmoteCommand` |
| `MountCapabilityStore` | MountCapability.db2 | `Mount` / `UpdateMountCapability` |
| `PowerTypeStore` | PowerType.db2 | power init |
| `SpellAuraOptionsStore` / `SpellAuraRestrictionsStore` | aura DBC | `IsImmunedToSpell` |
| `VehicleEntry` (in `DBCStores`) | Vehicle.db2 | `CreateVehicleKit` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_ATTACK_START` | server → client | `Unit::SendMeleeAttackStart` |
| `SMSG_ATTACK_STOP` | server → client | `Unit::SendMeleeAttackStop` |
| `SMSG_ATTACKERSTATEUPDATE` | server → client | `Unit::SendAttackStateUpdate` (combat-log row) |
| `SMSG_AURA_UPDATE` | server → client | `Unit::SendAuraUpdate` |
| `SMSG_AURA_UPDATE_ALL` | server → client | aura full re-sync |
| `SMSG_AI_REACTION` | server → client | `CreatureAI::AttackStart` |
| `SMSG_FORCE_RUN_SPEED_CHANGE`, `SMSG_FORCE_RUN_BACK_SPEED_CHANGE`, `SMSG_FORCE_SWIM_SPEED_CHANGE`, `SMSG_FORCE_SWIM_BACK_SPEED_CHANGE`, `SMSG_FORCE_TURN_RATE_CHANGE`, `SMSG_FORCE_FLIGHT_SPEED_CHANGE`, `SMSG_FORCE_FLIGHT_BACK_SPEED_CHANGE`, `SMSG_FORCE_PITCH_RATE_CHANGE`, `SMSG_FORCE_WALK_SPEED_CHANGE` | server → client | `Unit::SetSpeed` |
| `SMSG_MOVE_KNOCK_BACK` | server → client | `Unit::KnockbackFrom` |
| `SMSG_MOVE_SET_HOVER`, `SMSG_MOVE_UNSET_HOVER`, `SMSG_MOVE_SET_CAN_FLY`, `SMSG_MOVE_UNSET_CAN_FLY`, `SMSG_MOVE_WATER_WALK`, `SMSG_MOVE_LAND_WALK`, `SMSG_MOVE_FEATHER_FALL`, `SMSG_MOVE_NORMAL_FALL`, `SMSG_MOVE_SET_FLIGHT`, `SMSG_MOVE_UNSET_FLIGHT` | server → client | movement-mode toggles |
| `SMSG_DURABILITY_DAMAGE_DEATH` | server → client | `Player::DurabilityLossAll` (called by `Unit::Kill`) |
| `SMSG_PARTY_KILL_LOG` | server → client | `KillRewarder` |
| `SMSG_PARTY_MEMBER_STATS` / `SMSG_PARTY_MEMBER_STATS_FULL` | server → client | `Group::Update` (reads Unit health/power) |
| `SMSG_SET_VEHICLE_REC_ID` | server → client | `Unit::SendSetVehicleRecId` |
| `SMSG_PLAY_SPELL_VISUAL` / `SMSG_PLAY_SPELL_VISUAL_KIT` | server → client | spell visual cues |
| `SMSG_DISMOUNT` | server → client | `Unit::Dismount` |
| `SMSG_EMOTE` | server → client | `Unit::HandleEmoteCommand` |

Note: Unit-replicated state changes (HP, mana, level, flags, auras, faction, emote, stand state, mount, race/class/gender, stats, resistances) all also funnel through `SMSG_UPDATE_OBJECT` via `UnitData` `UpdateField<T>` writes — see `entities-object.md` for the replication primitive.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**

| File | Lines | Coverage of C++ Unit/ |
|---|---|---|
| `crates/wow-world/src/map_manager.rs` | ~890 | `WorldCreature` flat-field record (HP, level, faction, npc/unit flags, aggro radius, damage range, position, home_pos, combat target, swing timer, wander). **No Unit struct, no power array, no stat array, no resistance array, no aura container, no threat manager, no immunity buckets.** |
| `crates/wow-ai/src/lib.rs` | 346 | Legacy `CreatureAI` doubling as creature body. Some HP/state/swing logic that overlaps `WorldCreature`. |
| `crates/wow-world/src/session.rs` | 3138 | **Player Unit-fields scattered as flat session fields**: `player_level`, `player_race`, `player_class`, `player_gender`, `player_position`. No `Unit`/`Player` separation. |
| `crates/wow-combat/` | TBD | Some combat-math scaffolding; not wired into a real `Unit`. |
| `crates/wow-spell/` | TBD | Aura scaffolding; no `AddAura`/`RemoveAura` on a Unit. |
| `crates/wow-constants/` | — | A subset of `UnitFlags`/`MovementFlags`/`Powers`/`Stats`/`SpellSchools` constants. |

**What's implemented:**
- Single-power model: `current_hp` and `max_hp` as `u32` fields on `WorldCreature`.
- `WorldCreature::take_damage(damage: u32) -> bool` at `crates/wow-world/src/map_manager.rs:52` (struct definition) and `:176` (method): saturating-sub HP, returns `true` if died. **This single function is the entirety of `Unit::DealDamage`** (compare to the 13,620-line `Unit.cpp`).
- Basic respawn: `should_respawn()` + `respawn()` reset HP and position.
- `combat_target: Option<ObjectGuid>` + `last_swing` swing timer.
- Damage range `(min_dmg, max_dmg)` with a level-scaling default.
- `aggro_radius: f32`.
- `npc_flags: u32` and `unit_flags: u32` as opaque blobs (no enum binding).
- `state: CreatureState` (Idle / Wander / Combat / Dead) — a coarse subset of `UnitState`.

**What's missing vs C++:**
- **`Unit` struct itself.** No combat-capable mid-tier; `Player` and `Creature` (not even properly modeled — see `entities-object.md`) cannot share Unit-level code.
- **`UpdateField<T>`-tracked state.** Every HP/power/stat/flag write is silent; no dirty bit, no `SMSG_UPDATE_OBJECT` emit, no per-viewer filtering. (Root cause: `entities-object.md` not yet ported.)
- **Power array.** `MANA / RAGE / FOCUS / ENERGY / HAPPINESS / RUNES / RUNIC_POWER / SOUL_SHARDS / LUNAR_POWER / HOLY_POWER` and the Cataclysm+ power types (within 3.4.3 cap of 10) — none.
- **Stat array.** `STRENGTH / AGILITY / STAMINA / INTELLECT / SPIRIT` — not modeled per-unit.
- **School resistances.** `Resistances[7]` (Normal / Holy / Fire / Nature / Frost / Shadow / Arcane) — none. `armor` as a derived field — none.
- **`SpellSchoolMask` mitigation.** `CalculateAverageResistReduction` — none.
- **Modifier system.** `UnitMods`, `UnitModifierFlatType (BASE_VALUE / BASE_PCT_EXCLUDE_CREATE / TOTAL_VALUE)`, `UnitModifierPctType (BASE_PCT / TOTAL_PCT)`, `HandleStatFlatModifier`, `HandleStatPctModifier`, `GetTotalAuraModValue`, `UpdateAllStats` — none.
- **Auras.** No `AddAura` / `RemoveAura` / `RemoveAurasByType` / `HasAura` / `HasAuraType` / `GetTotalAuraModifier` / aura-driven proc table. No `m_appliedAuras` / `m_ownedAuras` containers. No `SMSG_AURA_UPDATE` emission.
- **Threat / combat manager.** `ThreatManager`, `CombatManager`, `HostileRefMgr`, `m_attackers`/`m_attacking` reverse-pointer set. No `EngageWithTarget`, `CombatStop`, `CombatStopWithPets`, `AtTargetAttacked`.
- **Spell casting on Unit.** No `CastSpell` overloads, no `GetCurrentSpell`, no `InterruptNonMeleeSpells`, no `IsNonMeleeSpellCast`. Spell handlers in `crates/wow-world/src/handlers/spell.rs` operate on session fields, not on a Unit.
- **Charm / possess.** `SetCharm`, `SetCharmedBy`, `CharmType`, `m_charmer`/`m_charmed`, `CharmInfo`, `UnitActionBarEntry`, `RemoveCharmAuras`, possessed-by-player predicates — none.
- **Vehicle base.** No `m_vehicleKit`, `m_vehicle`, `CreateVehicleKit`, `EnterVehicle`, `ExitVehicle`, `ChangeSeat`. (Compare `entities-vehicle.md` — also not started.)
- **Immunities.** `m_spellImmune[MAX_SPELL_IMMUNITY=7]` (Effect / State / School / Damage / Dispel / Mechanic / Id buckets), `ApplySpellImmune`, `GetSchoolImmunityMask`, `GetDamageImmunityMask`, `GetMechanicImmunityMask`, `IsImmunedToSpell`, `IsImmunedToSpellEffect`, `IsImmunedToDamage`, `IsImmunedToDamageOrSchool` — none.
- **Mechanics taxonomy.** `MECHANIC_CHARM / FEAR / ROOT / SILENCE / SLEEP / SNARE / STUN / FREEZE / POLYMORPH / BANISH / SHACKLE / KNOCKOUT / DAZE / SAPPED / TAUNT / ...` — not modeled as immunity keys.
- **Death-state machine.** `Alive → JustDied → Corpse → Dead → Ghost` transitions absent; `WorldCreature::is_alive` is a single bool.
- **`Kill` rewards path.** `KillRewarder`, kill-credit broadcast, group-share XP, durability loss — none.
- **Heal path.** `DealHeal`, `HealBySpell`, `HealInfo`, `SMSG_SPELLHEALLOG` — none.
- **Melee math.** `CalculateMeleeDamage`, `RollMeleeOutcomeAgainst`, `MeleeHitOutcome` enum, dodge/parry/block/glancing/crit/crushing — replaced by a single `min_dmg..=max_dmg` random roll with no avoidance.
- **Spell math.** `CalculateSpellDamageTaken`, `MeleeSpellMissChance`, `MeleeSpellHitResult`, `SpellCritChanceDone/Taken`, `MeleeDamageBonusDone/Taken`, `SpellDamageBonusDone/Taken`, `SpellHealingBonusDone/Taken`, `SpellBaseDamageBonusDone`, resilience — none.
- **Procs.** `ProcSkillsAndAuras`, `TriggerAurasProcOnEvent`, `GetProcAurasTriggeredOnEvent`, `ProcEventInfo`, `ProcFlagsHit / ProcFlagsSpellPhase / ProcFlagsSpellType / ProcFlagsInit` — none.
- **Speed array & movement-mode toggles.** Per-`UnitMoveType` speed multipliers, `SMSG_FORCE_*_SPEED_CHANGE`, `SetWalk`/`SetCanFly`/`SetWaterWalking`/`SetHover` toggles — none.
- **Mount lifecycle.** `Mount(mountId, vehicleId, creatureEntry)`, `Dismount`, `MountCapabilityEntry`, `UpdateMountCapability` — none.
- **Stat recompute (`StatSystem.cpp`).** All `UpdateMaxHealth`/`UpdateMaxPower`/`UpdateAttackPower`/`UpdateRangedAttackPowerAndDamage`/`UpdateArmor`/`UpdateResistances` paths — none.
- **`UnitFlags(1/2/3)` and `UnitState` semantics.** Stored as opaque `u32`; no enum binding, no "stunned implies cannot cast" cross-cutting logic.
- **NPC-flag predicates.** `IsVendor`/`IsTrainer`/`IsQuestGiver`/`IsGossip`/`IsTaxi`/`IsBanker`/`IsInnkeeper`/`IsSpiritHealer`/`IsBattleMaster`/`IsAuctioner`/`IsArmorer` — partial in handlers but not on a Unit.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `WorldCreature::take_damage(u32) -> bool` at `crates/wow-world/src/map_manager.rs:176` is a saturating subtract — the C++ `DealDamage` returns the amount dealt (may be < `damage` after absorb/resist) and triggers a long chain of side effects: `DealDamageMods`, on-damage auras, threat update, breakable-CC removal, durability loss, kill credit. None of those happen.
- Damage is `u32` in Rust, `uint64` in 3.4.3 C++ (HP is a 64-bit field on `UnitData`). Boss HP > 4 billion will wrap silently.
- `CreatureAI` (`crates/wow-ai/src/lib.rs`) holds its own copy of HP/state alongside `WorldCreature`'s copy — two sources of truth. The migration to MapManager (per `AGENTS.md`) eliminated the per-session creature copy, but the AI/body conflation is still there.
- Faction is stored as `u32` on `WorldCreature` but `IsHostileTo` is not implemented anywhere — combat targeting in handlers likely treats every creature as hostile to every player.
- `swing_timer_ms` is fixed per-creature; C++ uses `m_baseAttackTime[MAX_ATTACK=3]` per-attack-type with haste modifiers, dual-wield offhand penalty, etc.

**Tests existing:**
- A handful of `WorldCreature` tests in `crates/wow-world/src/map_manager.rs` (~12 tests per `AGENTS.md`).
- `CreatureAI` state-transition tests in `crates/wow-ai/src/lib.rs`.
- **Zero tests** for: power resources, stats, resistances, auras, threat, combat manager, immunity, charm, vehicle, mount, spell casting on Unit, melee outcome rolls, spell hit/crit, modifiers, `UpdateAllStats`, kill rewards, heal — because none of those exist.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_UNIT.WBS.001** Cerrar la migracion auditada de `game/Entities/Unit/CharmInfo.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/CharmInfo.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.002** Cerrar la migracion auditada de `game/Entities/Unit/CharmInfo.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/CharmInfo.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.003** Partir y cerrar la migracion auditada de `game/Entities/Unit/StatSystem.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1311 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.004** Partir y cerrar la migracion auditada de `game/Entities/Unit/Unit.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 13620 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.005** Partir y cerrar la migracion auditada de `game/Entities/Unit/Unit.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1953 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.006** Partir y cerrar la migracion auditada de `game/Entities/Unit/UnitDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/UnitDefines.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 526 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_UNIT.WBS.007** Partir y cerrar la migracion auditada de `game/Entities/Unit/enuminfo_UnitDefines.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/enuminfo_UnitDefines.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-combat`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 619 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered as `UNIT.x` for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h, split). Many of these are **blocked on `entities-object.md`** completing #OBJECT.5 (`Object`), #OBJECT.6 (`WorldObject`), #OBJECT.8 (`UpdateMask`), #OBJECT.9 (`UpdateField<T>`).

- [ ] **#UNIT.1** Port enums from `UnitDefines.h`: `UnitFlags`/`UnitFlags2`/`UnitFlags3` (bitflags), `UnitState`, `MovementFlags`, `Powers` (10 in 3.4.3), `Stats` (5), `WeaponAttackType` (3), `UnitMoveType` (9), `SpellSchools` (7), `SpellSchoolMask`, `SpellImmunity` (7 buckets), `Mechanics` (37), `UnitModifierFlatType` (3), `UnitModifierPctType` (2), `UnitMods`, `WeaponDamageRange`, `DamageEffectType`, `VictimState`, `MeleeHitOutcome`, `UnitStandStateType`, `SheathState`, `UnitVisFlags`, `UnitPVPStateFlags`, `UnitPetFlag`, `AnimTier`, `CharmType`. Place in `wow-constants`. (M)
- [ ] **#UNIT.2** Define `Unit` struct in `crates/wow-entities/src/unit.rs` (PROPOSED): embeds `WorldObject`; adds `unit_data: UnitData` (the `UpdateField<T>`-tracked replicated payload), state bitmask, type mask, attacker set, victim ref, threat manager handle, combat manager handle, motion master handle, charm info, charmer/charmed pair, vehicle kit, ridden vehicle, immunity buckets, speed array, base-attack-time array, attack-timer array, applied/owned aura maps. Blocked on #OBJECT.5/.6. (XL — split: 2a structure, 2b accessors, 2c integration with MapManager)
- [ ] **#UNIT.3** Port `UF::UnitData` (the replicated unit payload, ~150 fields including `Health`, `MaxHealth`, `Power[10]`, `MaxPower[10]`, `Level`, `Race`, `ClassId`, `Sex`, `FactionTemplate`, `Stats[5]`, `Resistances[7]`, `Flags`, `Flags2`, `Flags3`, `EmoteState`, `SheatheState`, `StandState`, `VisFlags`, `AnimTier`, `MountDisplayID`, `CharmedBy`, `Charm`, `NpcFlags[2]`, `BaseAttackTime`, `RangedAttackTime`, `BoundingRadius`, `CombatReach`, `DisplayID`, `NativeDisplayID`, `BaseHealthRegen`, `BaseManaRegen`). Each as `UpdateField<T>` from #OBJECT.9. (XL)
- [ ] **#UNIT.4** Implement HP API: `get_health`, `set_health`, `modify_health`, `get_max_health`, `set_max_health`, `count_pct_from_max_health`, `count_pct_from_cur_health`, `health_below_pct`, `health_above_pct`, `is_full_health`, `set_full_health`. Use `u64` not `u32`. Replace `WorldCreature::take_damage` callers. (M)
- [ ] **#UNIT.5** Implement Power API: `power_array: [UpdateField<i32>; 10]` + `max_power_array: [UpdateField<i32>; 10]`; `get_power(Powers)`, `set_power(Powers, i32, with_power_update: bool)`, `modify_power(Powers, i32) -> i32`, `set_max_power`, `get_power_pct`, `count_pct_from_max_power`, `update_display_power`, `calculate_display_power_type`. Trigger on-power-change auras. (H)
- [ ] **#UNIT.6** Implement Stat array `[UpdateField<i32>; 5]` + `get_stat(Stats)` / `set_stat`; resistance array `[UpdateField<i32>; 7]` + `get_resistance(SpellSchools)` / `set_resistance` / `get_resistance(SpellSchoolMask) -> i32`. Implement `armor` as `get_resistance(SPELL_SCHOOL_NORMAL)`. (M)
- [ ] **#UNIT.7** Implement modifier system: `UnitMods` enum bucket, `UnitModifierFlatType[3]` flat stack, `UnitModifierPctType[2]` pct stack; `handle_stat_flat_modifier(UnitMods, UnitModifierFlatType, f32, apply: bool)`, `handle_stat_pct_modifier`, `get_modifier_value`, `get_pct_modifier_value`, `get_total_aura_mod_value(UnitMods) -> f32`. (H)
- [ ] **#UNIT.8** Implement `update_all_stats` (virtual; trait method specialized for Player vs Creature). Sub-tasks: `update_max_health`, `update_max_power(Powers)`, `update_attack_power`, `update_ranged_attack_power_and_damage`, `update_armor`, `update_resistances(school)`. Match `StatSystem.cpp` semantics. (XL)
- [ ] **#UNIT.9** Implement `unit_flags(1/2/3)` API: `has_unit_flag(UnitFlags) -> bool`, `set_unit_flag`, `remove_unit_flag`, `replace_all_unit_flags`. Mirror for 2 and 3. Wire into `UnitData::Flags(2/3)` `UpdateField<u32>`. (M)
- [ ] **#UNIT.10** Implement `UnitState` (server-only bitmask, NOT replicated): `add_unit_state(u32)`, `has_unit_state`, `clear_unit_state`. Distinct from `UnitData::Flags`. (L)
- [ ] **#UNIT.11** Implement `set_death_state(DeathState)` state machine: `Alive → JustDied → Corpse → Dead → Ghost`. On `JustDied`: remove all auras, set `UNIT_FLAG_NOT_SELECTABLE`, fire `OnDeath` script. (M)
- [ ] **#UNIT.12** Implement `Kill(attacker, victim, durability_loss, skip_setting_death_state)` (free fn or `Unit::kill` static). Calls `set_death_state`, `KillRewarder::reward`, `Creature::set_loot_recipient`, `Player::durability_loss_all`. (H)
- [ ] **#UNIT.13** Implement `DealDamage(attacker, victim, damage: u64, clean_damage, damage_type, school_mask, spell_proto, durability_loss) -> u64` (static). Replace `WorldCreature::take_damage`. Apply absorb/resist auras, modify HP via `modify_health`, fire on-damage auras + threat updates + breakable-CC removal, call `Kill` if HP→0, return amount dealt. (XL)
- [ ] **#UNIT.14** Implement `DealHeal(HealInfo&)` and `HealBySpell(HealInfo&, critical: bool)`. Emit `SMSG_SPELLHEALLOG`. (M)
- [ ] **#UNIT.15** Implement `HealInfo` and `DamageInfo` structs with school-mask, attack-type, original/absorb/resist/block/final fields. (M)
- [ ] **#UNIT.16** Implement `MeleeHitOutcome` roll: port `RollMeleeOutcomeAgainst(victim, attType)` matching the C++ ordering (miss → dodge → parry → glancing → block → crit → crushing → normal). (H)
- [ ] **#UNIT.17** Implement avoidance chances: `get_unit_dodge_chance`, `get_unit_parry_chance`, `get_unit_block_chance`, `get_unit_miss_chance`, `get_unit_critical_chance_done/taken/against`. (H)
- [ ] **#UNIT.18** Implement `CalculateMeleeDamage(victim, &mut CalcDamageInfo, attack_type)` and `DealMeleeDamage(&CalcDamageInfo, durability_loss)`. (H)
- [ ] **#UNIT.19** Implement `AttackerStateUpdate(victim, attType, extra)` and `DoMeleeAttackIfReady`; integrate with creature tick path replacing the current `last_swing` / `swing_timer_ms` ad-hoc loop. (H)
- [ ] **#UNIT.20** Implement spell damage path: `CalculateSpellDamageTaken`, `DealSpellDamage`, `MeleeSpellMissChance`, `MeleeSpellHitResult`, `SpellCritChanceDone/Taken`. (XL)
- [ ] **#UNIT.21** Implement spell/melee bonus modifiers: `MeleeDamageBonusDone/Taken`, `SpellDamageBonusDone/Taken`, `SpellHealingBonusDone/Taken`, `SpellBaseDamageBonusDone/Taken`, `SpellBaseHealingBonusDone`. (XL)
- [ ] **#UNIT.22** Implement resilience: `CanApplyResilience`, `ApplyResilience(victim, &mut damage)`, `GetDamageReduction(damage)`. (M)
- [ ] **#UNIT.23** Implement aura container hookup: `AddAura(spellId, target) -> Option<AuraHandle>`, `AddAura(SpellInfo, effMask, target)`, `RemoveAura(spellId, casterGUID, reqEffMask, removeMode)` overloads, `RemoveAurasByType(AuraType, predicate, removeMode)`, `RemoveAllAuras`, `RemoveAllAurasOnDeath`, `RemoveAllAurasOnEvade`, `HasAura(spellId, casterGUID, itemCasterGUID, reqEffMask)`, `HasAuraType(AuraType)`, `GetAuraEffect(spellId, effIndex, casterGUID)`. The aura **object** model itself stays in `wow-spell` (and gets its own future `entities-aura.md`); Unit only owns the contract. (H)
- [ ] **#UNIT.24** Implement aggregate aura modifier reads: `GetTotalAuraModifier(AuraType)`, `GetTotalAuraMultiplier(AuraType)`, `GetMaxPositiveAuraModifier(AuraType)`, `GetMaxNegativeAuraModifier(AuraType)`. (M)
- [ ] **#UNIT.25** Implement proc system glue: `ProcSkillsAndAuras(actor, target, ...)`, `TriggerAurasProcOnEvent(...)`, `GetProcAurasTriggeredOnEvent(...)`, `ProcEventInfo` struct. (XL)
- [ ] **#UNIT.26** Implement immunity buckets: `m_spell_immune: [HashMap<u32, Vec<u32>>; 7]`, `apply_spell_immune(spellId, SpellImmunity, type, apply: bool)`, `get_school_immunity_mask -> u32`, `get_damage_immunity_mask -> u32`, `get_mechanic_immunity_mask -> u64`, `is_immuned_to_spell`, `is_immuned_to_spell_effect`, `is_immuned_to_damage(SpellSchoolMask)`, `is_immuned_to_damage_or_school(spellInfo)`. (H)
- [ ] **#UNIT.27** Implement threat manager binding on Unit: `get_threat_manager`, `engage_with_target`, `at_target_attacked(target, can_initial_aggro)`, `combat_stop(including_cast, mutual_pvp)`, `combat_stop_with_pets(including_cast)`. (H)
- [ ] **#UNIT.28** Implement attacker bookkeeping: `m_attackers` set, `m_attacking` ref, `attack(victim, melee_attack)`, `attack_stop`, `is_attacking_player`, `validate_attackers_and_own_target`, `stop_attack_faction(faction_id)`. (M)
- [ ] **#UNIT.29** Implement charm system: `set_charm(target, apply)`, `set_charmed_by(charmer, CharmType, aura_application)`, `remove_charmed_by(charmer)`, `is_charmed`, `is_possessed`, `is_possessed_by_player`, `is_possessing(u)`, `get_charmer`, `get_charmed`, `get_charmer_or_owner`, `init_charm_info`, `delete_charm_info`, `remove_charm_auras`, `CharmInfo` + `UnitActionBarEntry` (10 slots). (H)
- [ ] **#UNIT.30** Implement Vehicle base on Unit: `m_vehicle_kit: Option<Vehicle>`, `m_vehicle: Option<VehicleRef>`, `set_vehicle`, `get_vehicle_kit`, `get_vehicle`, `is_vehicle`, `create_vehicle_kit(id, creature_entry, loading)`, `remove_vehicle_kit(on_remove_from_world)`, `enter_vehicle`, `exit_vehicle`, `change_seat(seat_id, next)`, `send_set_vehicle_rec_id`. Cross-ref `entities-vehicle.md`. (H)
- [ ] **#UNIT.31** Implement movement-mode toggles: `set_walk`, `set_disable_gravity`, `set_swim`, `set_can_fly`, `set_water_walking`, `set_feather_fall`, `set_hover` — each emitting the matching `SMSG_MOVE_*` packet. (M)
- [ ] **#UNIT.32** Implement speed array `[f32; 9]`: `get_speed(UnitMoveType)`, `set_speed(UnitMoveType, f32)`, `get_speed_rate`, `set_speed_rate`, `update_speed(UnitMoveType)`. Emit `SMSG_FORCE_*_SPEED_CHANGE`. (M)
- [ ] **#UNIT.33** Implement Mount lifecycle: `mount(mount_id, vehicle_id, creature_entry)`, `dismount`, `is_mounted`, `get_mount_display_id`, `set_mount_display_id`, `update_mount_capability`. (M)
- [ ] **#UNIT.34** Implement spell-cast slots: `current_spells: [Option<SpellHandle>; 4]` (Generic / Melee / AutoRepeat / Channeled); `cast_spell(target, spell_id, args)` overloads, `get_current_spell(CurrentSpellTypes)`, `is_non_melee_spell_cast(...)`, `interrupt_non_melee_spells(with_delayed, spell_id, with_instant)`, `finish_spell(CurrentSpellTypes, ok)`. (XL)
- [ ] **#UNIT.35** Implement NPC-flag predicates: `is_vendor / is_trainer / is_quest_giver / is_gossip / is_taxi / is_battle_master / is_banker / is_innkeeper / is_spirit_healer / is_tabard_designer / is_auctioner / is_armorer / is_critter / is_service_provider / is_wild_battle_pet`. Bind `UnitData::NpcFlags[0..1]` to the `NPCFlags` / `NPCFlags2` bitflags. (L)
- [ ] **#UNIT.36** Implement `m_unit_type_mask` and predicates: `IsSummon`, `IsGuardian`, `IsPet`, `IsHunterPet`, `IsTotem`, `IsVehicle`. (L)
- [ ] **#UNIT.37** Implement faction binding: `get_faction`, `set_faction`, faction-template lookup (DB2), `is_hostile_to(other) -> bool`, `is_friendly_to(other) -> bool`. (M)
- [ ] **#UNIT.38** Implement combat-log emit: `send_attack_state_update(hit_info, target, swing_type, school_mask, damage, absorb, resist, target_state, blocked)`, `send_melee_attack_start`, `send_melee_attack_stop`, `send_aura_update(slot, remove)`, `send_durability_loss`. (M)
- [ ] **#UNIT.39** Migrate `WorldCreature` callers to the new `Unit`. Eliminate `WorldCreature::take_damage`. Eliminate `CreatureAI` HP/state copy (the AI controller becomes a stateless strategy that operates on a `&mut Unit`). (XL)
- [ ] **#UNIT.40** Migrate session-level Player Unit-fields (`player_level`, `player_race`, `player_class`, `player_gender`, `player_position`) into a `Player` struct embedding `Unit`. Cross-ref: future `entities-player.md`. (XL — coordinates with #ENTITIES.13)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_UNIT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 7 files / 18469 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | `cargo test -p wow-ai && cargo test -p wow-combat && cargo test -p wow-constants` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_UNIT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 7 files / 18469 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_UNIT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 7 files / 18469 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_UNIT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 7 files / 18469 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Unit::set_health(MaxHealth + 1)` clamps to `MaxHealth`; `set_health(0)` triggers `set_death_state(JustDied)`.
- [ ] Test: `Unit::modify_health(-1)` from full HP returns `-1`; `modify_health(-MaxHealth)` returns the actual delta and triggers death.
- [ ] Test: `Unit::deal_damage` with `damage > current_health` returns the actual damage dealt (clamped) and calls `Kill` exactly once.
- [ ] Test: `Unit::deal_damage` with `school_mask = SHADOW` against a unit with `SCHOOL` immunity to shadow → returns 0; HP unchanged.
- [ ] Test: `Unit::deal_damage` with `mechanic = STUN` against a unit with `MECHANIC` immunity to stun → no aura applied, no CC triggered.
- [ ] Test: `Unit::heal_by_spell` clamps at `MaxHealth`; over-heal portion does not appear in `SMSG_SPELLHEALLOG`.
- [ ] Test: `Unit::set_power(Powers::Mana, 100)` flips dirty bit on `UnitData::Power[0]`; subsequent `SMSG_UPDATE_OBJECT` emits the new value.
- [ ] Test: `Unit::set_power(Powers::Energy, 200)` clamps to `MaxPower(Energy) = 100`.
- [ ] Test: `Unit::set_stat(Stats::Stamina, X)` followed by `update_all_stats` recomputes `MaxHealth = base + stamina * 10` (or whatever the 3.4.3 formula is).
- [ ] Test: `Unit::set_resistance(SHADOW, R)` followed by spell-damage taken with `SCHOOL_MASK_SHADOW` reduces damage by `CalculateAverageResistReduction`.
- [ ] Test: `Unit::handle_stat_flat_modifier(UNIT_MOD_ATTACK_POWER, BASE_VALUE, +50, apply=true)` then `update_attack_power_and_damage` → AP increased by 50; `apply=false` reverses exactly.
- [ ] Test: `Unit::add_aura(SPELL_BLESSING_OF_KINGS)` + `update_all_stats` → all primary stats up 10%; remove aura → values restored.
- [ ] Test: `Unit::has_aura_type(AURA_PERIODIC_DAMAGE)` true after a DoT applied; aura tick reduces target HP every period.
- [ ] Test: `Unit::remove_auras_by_type(AURA_MOD_DECREASE_SPEED, predicate)` removes only matches; preserves non-matches.
- [ ] Test: `Unit::roll_melee_outcome_against` against a target with 100% dodge always returns `MeleeHitOutcome::Dodge`.
- [ ] Test: `Unit::calculate_melee_damage` against a target with `UNIT_FLAG_NON_ATTACKABLE` early-returns with `Damage = 0`.
- [ ] Test: `Unit::engage_with_target` adds the target to threat list when `can_have_threat_list()`; otherwise sets `IsInCombatWith` only.
- [ ] Test: `Unit::combat_stop(including_cast=true)` interrupts active spell, clears threat, emits `SMSG_ATTACK_STOP`.
- [ ] Test: `Unit::set_charmed_by(charmer, CharmType::Possess)` swaps motion master, sets `UNIT_STATE_POSSESSED`, charmer's `m_charmed` points to victim, `RemoveCharmedBy` reverses exactly.
- [ ] Test: `Unit::create_vehicle_kit(id, creature_entry, loading=false)` populates `m_vehicleKit`; `EnterVehicle` adds passenger; `ExitVehicle` removes; `RemoveVehicleKit` despawns kit and exits all passengers.
- [ ] Test: `Unit::set_speed(Run, 14.0)` emits `SMSG_FORCE_RUN_SPEED_CHANGE` with the new rate.
- [ ] Test: `Unit::mount(mount_id)` sets `UNIT_FLAG_MOUNT` and `MountDisplayID`; `dismount()` clears both.
- [ ] Test: `Unit::set_unit_flag(UNIT_FLAG_STUNNED)` followed by `cast_spell` returns immediately (cast prevented).
- [ ] Test: `Unit::apply_spell_immune(0, SpellImmunity::School, SCHOOL_MASK_FROST, apply=true)` makes `is_immuned_to_damage(SCHOOL_MASK_FROST)` true; `apply=false` reverses.
- [ ] Test: `Unit::is_hostile_to(other)` matches faction-template DBC table for the standard 3.4.3 race/faction matrix (golden table).
- [ ] Test: `Unit::deal_damage` triggering `Kill` invokes `KillRewarder::reward` once with the actual killing-blow attacker (not pet/owner).
- [ ] Test: `Unit::set_death_state(JustDied)` removes all auras except `IsDeathPersistent` ones.
- [ ] Test: `Unit::do_melee_attack_if_ready` swings only when the swing timer has elapsed; consecutive ticks within the timer no-op.
- [ ] Test: Round-trip: `Unit::set_health` then a tick → the next `SMSG_UPDATE_OBJECT` emits the values block with the new Health field present and no other dirty bits.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 7 files / 18469 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp` | `crates/wow-world/` (live state, currently `WorldCreature` flat fields), `crates/wow-ai/` (legacy `CreatureAI` doubling as creature body), `crates/wow-combat/` (combat math — partial), `crates/wow-spell/` (auras — partial), `crates/wow-constants/` (`UnitFlags`, `MovementFlags`, `Powers`, `Stats`, `SpellSchools`) \| 🔧 broken (rewrite needed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_UNIT.DIV.001` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 7 C++ files / 18469 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#ENTITIES_UNIT.DIV.002` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 7 C++ files / 18469 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/StatSystem.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`Unit.cpp` is 13,620 lines.** Do not migrate in one sitting. The natural slices are: stats/power/health (#UNIT.4-.8), flags/state (.9-.10), death/kill/damage (.11-.15), melee math (.16-.19), spell math (.20-.22), auras (.23-.25), immunities (.26), threat/attackers (.27-.28), charm (.29), vehicle (.30), movement/speed/mount (.31-.33), spell-cast slots (.34), NPC predicates (.35-.36), faction (.37), wire (.38).
- **`Unit::DealDamage` is a `static` method.** Both attacker and victim are passed in. Do not put it as a method on `Unit` — make it a free function `unit::deal_damage(attacker: &mut Unit, victim: &mut Unit, ...)`. Same for `Kill`, `DealDamageMods`, `DealHeal`, `ProcSkillsAndAuras`, `ApplyResilience`.
- **HP and power values are 64-bit on 3.4.3 `UnitData`.** The current `WorldCreature::current_hp: u32` will silently wrap on boss encounters. Use `i64` / `u64` consistently.
- **Powers in 3.4.3 cap at `MAX_POWERS = 10`.** Newer power types (`SOUL_SHARDS`, `LUNAR_POWER`, `HOLY_POWER`, `ALTERNATE`, `MAELSTROM`, `CHI`, `INSANITY`, `BURNING_EMBERS`, `DEMONIC_FURY`, `ARCANE_CHARGES`, `FURY`, `PAIN`) exist in the enum (because the field tables are shared) but are NOT used by 3.4.3 classes. The `Power[]` array on the wire is sized for 10; do not over-read.
- **`m_state` (`UnitState`) is server-only**, NOT replicated. It is distinct from `UnitData::Flags` (`UnitFlags`, replicated). Mixing them is a frequent source of bugs — confirm every flag write goes to the right bag. Example: `UNIT_STATE_CASTING` (server-only) vs. `UNIT_FLAG_PREPARATION` (replicated).
- **`UnitFlags2` and `UnitFlags3` are real fields** on 3.4.3 `UnitData` (`Flags2`, `Flags3`). Don't merge them.
- **`m_spellImmune` is keyed by `SpellImmunity` bucket** (7 values: `Effect / State / School / Damage / Dispel / Mechanic / Id`), with a `multimap<type, spellId>` per bucket. `ApplySpellImmune(spellId, bucket, type, apply)` adds or removes an entry. Querying immunity walks the bucket and ORs the keys.
- **`IsImmunedToSpell` checks ALL buckets**, not just `Id`. The standard order is: dispel, school, damage, mechanic, effect, state, id. Each bucket short-circuits if it matches.
- **`m_unitTypeMask` is NOT `TypeID`.** `TypeID` is the `Object`-level discriminant (Player/Creature/GameObject). `m_unitTypeMask` is a Unit-level subtype mask: `UNIT_MASK_NONE / UNIT_MASK_SUMMON / UNIT_MASK_MINION / UNIT_MASK_GUARDIAN / UNIT_MASK_TOTEM / UNIT_MASK_PET / UNIT_MASK_VEHICLE / UNIT_MASK_PUPPET / UNIT_MASK_HUNTER_PET / UNIT_MASK_CONTROLABLE_GUARDIAN`. A Pet has both `TYPEID_UNIT` AND `UNIT_MASK_PET | UNIT_MASK_GUARDIAN | UNIT_MASK_MINION | UNIT_MASK_SUMMON`.
- **Charm transitions are bidirectional and chain.** `SetCharmedBy(charmer, type)` updates `m_charmer` on victim AND `m_charmed` on charmer; the reverse on `RemoveCharmedBy`. Forgetting either side leaves a dangling reference. Possession also swaps the motion master and replaces the AI.
- **Vehicle is BOTH a member of Unit (`m_vehicleKit` / `m_vehicle`) AND a class of its own.** A Unit that IS a vehicle has `m_vehicleKit`; a Unit that is RIDING one has `m_vehicle`. Both can be set on a single Unit (a vehicle riding another vehicle: e.g. drake riding ground transport in Wintergrasp).
- **Speed array reads come from `m_speed_rate * BASE_SPEED[move_type]`**, not from a stored absolute speed. Setting speed sets the rate; the wire packet ALSO carries the rate not the absolute speed.
- **Aura proc evaluation walks `m_modAuras[AuraType]` lists**, not the full aura container. The `m_modAuras` index is rebuilt on `_ApplyAuraEffect` / `_UnapplyAuraEffect`.
- **`UpdateAllStats` is virtual and Player-vs-Creature differ a lot.** Player path reads talent/spec/ratings; Creature path reads `CreatureLevelStats` from DB2. Do not unify them prematurely.
- **`SetDeathState` order matters.** `Alive → JustDied → Corpse → Dead → Ghost`. `JustDied` is the *transition* state where loot is generated and on-death effects fire; `Corpse` is the persisted state. `Dead` is post-corpse-decay. Skipping `JustDied` skips loot.
- **`KillRewarder` is shared between Creature and Player kill paths** but lives under `Player/`. It transitively touches Group, Quest, and Achievement systems — do not migrate it standalone before those exist as stubs.
- **`Unit::Kill` does NOT call `Unit::AttackStop`.** That cleanup happens via `CombatStop` triggered indirectly by death state. If your port short-circuits, attacker references will leak.
- **Periodic aura ticks are NOT in `Unit::Update`.** They run inside `Aura::Update` / `AuraEffect::HandlePeriodic*Aura`. Unit's `Update` only ticks combat manager, motion master, swing timers, regen, and the spell currently being cast.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Unit : WorldObject` | `struct Unit { world_object: WorldObject, unit_data: UnitData, state: UnitState, type_mask: UnitTypeMask, ... }` in `crates/wow-entities/src/unit.rs` (PROPOSED) | Composition; `impl Unit { ... }` for combat methods |
| `UF::UnitData` | `struct UnitData { health: UpdateField<u64>, max_health: UpdateField<u64>, power: UpdateFieldArray<i32, 10>, max_power: UpdateFieldArray<i32, 10>, level: UpdateField<u32>, race: UpdateField<u8>, class_id: UpdateField<u8>, sex: UpdateField<u8>, faction_template: UpdateField<u32>, stats: UpdateFieldArray<i32, 5>, resistances: UpdateFieldArray<i32, 7>, flags: UpdateField<u32>, flags2: UpdateField<u32>, flags3: UpdateField<u32>, ... }` | Generated/hand-ported from `UpdateFields.h` |
| `enum UnitFlags : uint32` | `bitflags! struct UnitFlags : u32` | mirror in `wow-constants` |
| `enum UnitFlags2/3` | `bitflags! struct UnitFlags2 : u32` / `UnitFlags3 : u32` | — |
| `enum UnitState` (bitmask) | `bitflags! struct UnitState : u32` | server-only, NOT replicated |
| `enum MovementFlags` | `bitflags! struct MovementFlags : u32` | — |
| `enum Powers` | `#[repr(i8)] enum Powers { Mana, Rage, Focus, Energy, Happiness, Runes, RunicPower, SoulShards, LunarPower, HolyPower, MaxPowers = 10, AllPowers = 127, Health = -2 }` | `MAX_POWERS = 10` in 3.4.3 |
| `enum Stats` | `#[repr(u8)] enum Stats { Strength=0, Agility, Stamina, Intellect, Spirit, MaxStats=5 }` | — |
| `enum WeaponAttackType` | `#[repr(u8)] enum WeaponAttackType { BaseAttack=0, OffAttack, RangedAttack, MaxAttack=3 }` | — |
| `enum UnitMoveType` | `#[repr(u8)] enum UnitMoveType { Walk=0, Run, RunBack, Swim, SwimBack, TurnRate, Flight, FlightBack, PitchRate, MaxMoveType=9 }` | — |
| `enum SpellSchools` | `#[repr(u8)] enum SpellSchools { Normal=0, Holy, Fire, Nature, Frost, Shadow, Arcane, Max=7 }` | — |
| `enum SpellSchoolMask` | `bitflags! struct SpellSchoolMask : u32` | `NORMAL=0x01 .. ARCANE=0x40`, `MAGIC=0x7E`, `ALL=0x7F` |
| `enum SpellImmunity` | `#[repr(u8)] enum SpellImmunity { Effect=0, State, School, Damage, Dispel, Mechanic, Id, MaxSpellImmunity=7 }` | bucket key |
| `enum Mechanics` | `#[repr(u32)] enum Mechanics { None=0, Charm, ..., MaxMechanic=37 }` | + `bitflags! MechanicMask : u64` for the OR-mask form |
| `class DamageInfo` | `struct DamageInfo { attacker, victim, school_mask, attack_type, damage_type, damage, absorb, resist, block, original_damage }` | — |
| `class HealInfo` | `struct HealInfo { healer, target, school_mask, original_heal, effective_heal, absorb }` | — |
| `class CharmInfo` | `struct CharmInfo { unit: ObjectGuid, command_state, react_state, action_bar: [UnitActionBarEntry; 10], pet_number, pet_name }` | — |
| `struct UnitActionBarEntry` | `struct UnitActionBarEntry { spell_or_command: u32, active_state: u8 }` | — |
| `SpellImmuneContainer m_spellImmune[MAX_SPELL_IMMUNITY]` | `[HashMap<u32, Vec<u32>>; 7]` — outer index is `SpellImmunity` bucket | unordered-multimap → HashMap-of-Vec |
| `ThreatManager m_threatManager` | `ThreatManager` (lives in `wow-combat`) | held by ref or value depending on lifetime story |
| `CombatManager m_combatManager` | `CombatManager` (lives in `wow-combat`) | — |
| `MotionMaster* m_motion` | `Box<MotionMaster>` (lives in `wow-movement`) | — |
| `Vehicle* m_vehicleKit` | `Option<Vehicle>` | inline; cross-ref `entities-vehicle.md` |
| `Vehicle* m_vehicle` | `Option<VehicleHandle>` | weak handle into a vehicle owned by another Unit |
| `AuraApplicationMap m_appliedAuras` | `BTreeMap<u32, Vec<AuraApplicationHandle>>` (spellId → applications) | aura object model owned by `wow-spell`; future `entities-aura.md` |
| `AuraMap m_ownedAuras` | `BTreeMap<u32, AuraHandle>` (spellId → owned aura) | — |
| `Spell* m_currentSpells[CURRENT_MAX_SPELL]` | `[Option<SpellHandle>; 4]` (Generic / Melee / AutoRepeat / Channeled) | — |
| `static uint32 DealDamage(...)` | `pub fn deal_damage(attacker: &mut Unit, victim: &mut Unit, damage: u64, ...) -> u64` (free fn) | static in C++ → free fn in Rust |
| `static void Kill(...)` | `pub fn kill(attacker: &mut Unit, victim: &mut Unit, durability_loss: bool, skip_setting_death_state: bool)` (free fn) | — |
| `static void DealHeal(HealInfo&)` | `pub fn deal_heal(info: &mut HealInfo)` (free fn) | — |
| `void SetHealth(uint64)` | `fn set_health(&mut self, val: u64)` | flips dirty bit on `unit_data.health` |
| `int64 ModifyHealth(int64)` | `fn modify_health(&mut self, val: i64) -> i64` | returns actual delta after clamp |
| `bool IsImmunedToSpell(SpellInfo const*, ...)` | `fn is_immuned_to_spell(&self, spell: &SpellInfo, caster: Option<&Unit>) -> bool` | — |
| `Aura* AddAura(uint32 spellId, Unit* target)` | `fn add_aura(&mut self, spell_id: u32, target: &mut Unit) -> Option<AuraHandle>` | — |
| `void RemoveAura(uint32 spellId, ObjectGuid casterGUID, uint32 reqEffMask, AuraRemoveMode)` | `fn remove_aura(&mut self, spell_id: u32, caster: ObjectGuid, req_eff_mask: u32, mode: AuraRemoveMode)` | — |
| `void SetCharmedBy(Unit*, CharmType, AuraApplication*)` | `fn set_charmed_by(&mut self, charmer: &mut Unit, ty: CharmType, aura_app: Option<&AuraApplication>) -> bool` | — |
| `void SendMeleeAttackStart(Unit*)` | `fn send_melee_attack_start(&self, victim: &Unit, ctx: &SendCtx)` | emit `SMSG_ATTACK_START` |
| `WorldCreature::take_damage(u32) -> bool` (current Rust) | **delete** | replaced by `unit::deal_damage` |

---

## 13. Audit (2026-05-01)

Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/` (`Unit.{h,cpp}`, `UnitDefines.h`, `StatSystem.cpp`, `CharmInfo.{h,cpp}`) against current Rust state in `crates/wow-world/src/map_manager.rs`, `crates/wow-ai/src/lib.rs`, `crates/wow-world/src/session.rs`, `crates/wow-world/src/handlers/combat.rs`, `crates/wow-world/src/handlers/spell.rs`, `crates/wow-combat/`, `crates/wow-spell/`, `crates/wow-constants/`. Verdict: **🔧 broken — no `Unit` exists**. Coverage is single-digit percent and structurally unsound.

### `Unit` class (`Unit.h` + `Unit.cpp` — 1953 + 13620 = ~15,600 lines)
**Coverage in Rust:** ~1%. The single concrete substitution is `WorldCreature::take_damage(damage: u32) -> bool` at `/home/server/rustycore/crates/wow-world/src/map_manager.rs:176` — a 7-line saturating subtract that returns `true` on death. This stands in for the 13,620-line `Unit.cpp` (which includes `DealDamage`, `Kill`, `DealHeal`, `CalculateMeleeDamage`, `CalculateSpellDamageTaken`, `RollMeleeOutcomeAgainst`, `MeleeSpellMissChance`, all aura plumbing, all immunity plumbing, all charm plumbing, all vehicle plumbing, all stat-modifier plumbing).

`WorldCreature` (the closest thing to a Unit-like record) carries: `current_hp: u32`, `max_hp: u32`, `is_alive: bool`, `level: u8`, `faction: u32`, `npc_flags: u32`, `unit_flags: u32`, `aggro_radius: f32`, `min_dmg/max_dmg: u32`, `swing_timer_ms: u64`, `combat_target: Option<ObjectGuid>`. That is the full Unit-state inventory in the current implementation.

### `UnitDefines.h` (526 lines)
**Coverage in Rust:** partial. A subset of `UnitFlags`, `MovementFlags`, `Powers`, `Stats`, `SpellSchools` exists in `wow-constants` but not the full taxonomy: `UnitFlags2`/`UnitFlags3` not modeled, `UnitState` not modeled (no server-only state distinction), `Mechanics` (37 values, the basis of CC immunity) absent, `SpellImmunity` (7 buckets) absent, `MeleeHitOutcome` / `VictimState` absent, `UnitMods` absent, `UnitModifierFlatType` / `UnitModifierPctType` absent, `CharmType` absent.

### `StatSystem.cpp` (1311 lines)
**Coverage in Rust:** **0%**. No `UpdateAllStats`, no `UpdateMaxHealth`, no `UpdateMaxPower`, no `UpdateAttackPower`, no `UpdateRangedAttackPowerAndDamage`, no `UpdateArmor`, no `UpdateResistances`, no `UpdateSpellPower`. There is no per-Unit stat array to recompute *into*. Stat changes from gear/auras/talents have no entry path.

### `CharmInfo.{h,cpp}` (157 + 283 lines)
**Coverage in Rust:** **0%**. No `CharmInfo`, no `UnitActionBarEntry`, no command/reaction state enums, no charm transition logic (`SetCharmedBy` / `RemoveCharmedBy`). Pet command bar is not modeled.

### Aura subsystem contract on Unit
**Coverage in Rust:** **0%**. No `AddAura` / `RemoveAura` / `RemoveAurasByType` / `HasAura` / `HasAuraType` on a Unit. No `m_appliedAuras` / `m_ownedAuras` containers. No `m_modAuras[AuraType]` index. The `wow-spell` crate may have aura-data scaffolding, but there is no `Unit` to attach auras *to*.

### Threat / combat manager / attacker bookkeeping
**Coverage in Rust:** **0%**. No `ThreatManager`, no `CombatManager`, no `m_attackers` reverse-pointer set, no `m_attacking` victim ref, no `EngageWithTarget`, no `CombatStop`, no `AtTargetAttacked`. Combat in handlers/combat.rs is direct HP mutation against `WorldCreature::take_damage` with no threat propagation or attacker validation.

### Immunities (`m_spellImmune[7]`)
**Coverage in Rust:** **0%**. No immunity buckets, no `ApplySpellImmune`, no `GetSchoolImmunityMask`, no `GetMechanicImmunityMask`, no `IsImmunedToSpell`. School/mechanic/dispel/effect/state immunity from auras and creature templates is unenforceable.

### School masks / school resistances / armor mitigation
**Coverage in Rust:** **0%**. No `Resistances[7]` array; armor is not derived. `CalculateAverageResistReduction` does not exist. Spell damage in handlers takes no resist/absorb path.

### Charm / possess / vehicle base
**Coverage in Rust:** **0%**. Cross-ref `entities-vehicle.md` (also ❌ not started).

### Modifier system (`UnitMods`, flat/pct stacks)
**Coverage in Rust:** **0%**. No modifier buckets, no `HandleStatFlatModifier`/`HandleStatPctModifier`, no aggregate `GetTotalAuraModValue`. Aura buffs cannot reach stats.

### Speed array, mount, movement-mode toggles
**Coverage in Rust:** **0%**. No per-`UnitMoveType` speed array; movement handlers use a fixed walk/run speed. No mount lifecycle. No `SetCanFly`/`SetWaterWalking`/`SetHover` toggles. No `SMSG_FORCE_*_SPEED_CHANGE` emission.

### `DealDamage` / `Kill` / `HealBy` taxonomy
**Coverage in Rust:** `WorldCreature::take_damage` only (~7 lines, no school mask, no spell info, no clean damage, no damage type, no proc, no threat, no kill-rewarder). `Kill` not implemented. `HealBySpell` not implemented. `DealHeal` not implemented.

### Verdict
**🔧 broken — must be ported. Blocked on `entities-object.md` shipping the foundation.** Specifically, `Unit` cannot exist without:
1. `Object` + `WorldObject` skeleton (#OBJECT.5 / #OBJECT.6)
2. `UpdateMask` (#OBJECT.8)
3. `UpdateField<T>` (#OBJECT.9)
4. `UF::ObjectData` first vertical slice (#OBJECT.12)
5. `UpdateData` accumulator (#OBJECT.13)

Once those land, the Unit migration order is roughly: enums (#UNIT.1) → struct skeleton (#UNIT.2) → `UnitData` payload (#UNIT.3) → HP/power/stats/resistances (#UNIT.4–.6) → modifiers + `UpdateAllStats` (#UNIT.7–.8) → flag/state APIs (#UNIT.9–.10) → death/kill/damage (#UNIT.11–.15) → melee/spell math (#UNIT.16–.22) → auras (#UNIT.23–.25) → immunities (#UNIT.26) → threat/attackers (#UNIT.27–.28) → charm/vehicle (#UNIT.29–.30) → movement/mount (#UNIT.31–.33) → spell-cast slots (#UNIT.34) → NPC predicates (#UNIT.35–.36) → faction (#UNIT.37) → wire (#UNIT.38) → migration (#UNIT.39–.40).

Until that pipeline lands, **every combat-adjacent feature in RustyCore is held up by a 7-line saturating-sub** at `crates/wow-world/src/map_manager.rs:176`.

---

*Template version: 1.0 (2026-05-01).* Last updated 2026-05-01.
