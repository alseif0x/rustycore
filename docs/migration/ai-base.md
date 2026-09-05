# Migration: AI — Base classes & stock AIs

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/AI/` (sub-tree `CoreAI/` + the two `CreatureAI.{h,cpp}` files at the root + `ScriptedAI/ScriptedCreature.{h,cpp}` for `EventMap` / `SummonList`)
> **Rust target crate(s):** `crates/wow-ai/`
> **Layer:** L5 — game systems (creature behaviour, base AI traits + stock controllers)
> **Status:** 🔧 broken (rewrite needed) — single concrete `CreatureAI` struct, no trait, no factory, no stock subclasses, no `EventMap`/`SummonList`
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

> **Sub-doc of [`ai.md`](./ai.md).** Companion: [`ai-smartscripts.md`](./ai-smartscripts.md) (data-driven engine). Also see [`scripting.md`](./scripting.md) (registry / `ScriptMgr`) and [`scripts.md`](./scripts.md) (concrete boss/quest scripts that subclass `ScriptedAI`).

---

## 1. Purpose

The "base AI" sub-module is the polymorphic foundation that every NPC controller in TrinityCore inherits from. `UnitAI` is the abstract root used by anything that ticks and selects targets (creatures, charmed players, vehicles); `CreatureAI` adds creature-specific hooks (`JustEngagedWith`, `JustReachedHome`, `MoveInLineOfSight`, `EnterEvadeMode`, …). Concrete stock subclasses cover all common "boring" behaviours so individual mobs do not need bespoke C++: `NullCreatureAI` (no-op), `PassiveAI` / `CritterAI` / `TriggerAI` (don't fight), `CombatAI` / `AggressorAI` (basic auto-attack + spell list rotation), `PetAI` (follow + obey commands), `TotemAI` (cast on cooldown + lifespan), `GuardAI` (city guards calling each other for help), `ReactorAI` (only fights when first attacked), `ScheduledChangeAI` (swap to another AI after a timer for boss phase transitions), `VehicleAI` (vehicle without a passenger doing anything autonomously). The module also ships two helper containers — `EventMap` (timer/event queue used by every boss script) and `SummonList` (track and clean up summons on death) — defined inside `ScriptedAI/ScriptedCreature.{h,cpp}` and consumed by both `ScriptedAI` and `SmartAI`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/AI/CoreAI/AreaTriggerAI.cpp` | 28 | `prefix` |
| `game/AI/CoreAI/AreaTriggerAI.h` | 79 | `prefix` |
| `game/AI/CoreAI/CombatAI.cpp` | 309 | `prefix` |
| `game/AI/CoreAI/CombatAI.h` | 107 | `prefix` |
| `game/AI/CoreAI/GameObjectAI.cpp` | 42 | `prefix` |
| `game/AI/CoreAI/GameObjectAI.h` | 134 | `prefix` |
| `game/AI/CoreAI/GuardAI.cpp` | 69 | `prefix` |
| `game/AI/CoreAI/GuardAI.h` | 37 | `prefix` |
| `game/AI/CoreAI/PassiveAI.cpp` | 119 | `prefix` |
| `game/AI/CoreAI/PassiveAI.h` | 93 | `prefix` |
| `game/AI/CoreAI/PetAI.cpp` | 651 | `prefix` |
| `game/AI/CoreAI/PetAI.h` | 75 | `prefix` |
| `game/AI/CoreAI/ReactorAI.cpp` | 32 | `prefix` |
| `game/AI/CoreAI/ReactorAI.h` | 33 | `prefix` |
| `game/AI/CoreAI/ScheduledChangeAI.cpp` | 22 | `prefix` |
| `game/AI/CoreAI/ScheduledChangeAI.h` | 40 | `prefix` |
| `game/AI/CoreAI/TotemAI.cpp` | 87 | `prefix` |
| `game/AI/CoreAI/TotemAI.h` | 39 | `prefix` |
| `game/AI/CoreAI/UnitAI.cpp` | 401 | `prefix` |
| `game/AI/CoreAI/UnitAI.h` | 182 | `prefix` |
| `game/AI/CoreAI/UnitAICommon.cpp` | 186 | `prefix` |
| `game/AI/CoreAI/UnitAICommon.h` | 117 | `prefix` |
| `game/AI/CoreAI/enuminfo_UnitAICommon.cpp` | 73 | `prefix` |
| `game/AI/PlayerAI/PlayerAI.cpp` | 1307 | `prefix` |
| `game/AI/PlayerAI/PlayerAI.h` | 113 | `prefix` |
| `game/AI/ScriptedAI/ScriptedCreature.cpp` | 714 | `prefix` |
| `game/AI/ScriptedAI/ScriptedCreature.h` | 428 | `prefix` |
| `game/AI/ScriptedAI/ScriptedEscortAI.cpp` | 431 | `prefix` |
| `game/AI/ScriptedAI/ScriptedEscortAI.h` | 100 | `prefix` |
| `game/AI/ScriptedAI/ScriptedFollowerAI.cpp` | 294 | `prefix` |
| `game/AI/ScriptedAI/ScriptedFollowerAI.h` | 73 | `prefix` |
| `game/AI/ScriptedAI/ScriptedGossip.cpp` | 74 | `prefix` |
| `game/AI/ScriptedAI/ScriptedGossip.h` | 103 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/AI/CoreAI/UnitAI.h` | 182 | `class UnitAI` — abstract root; `UpdateAI=0`, `SelectTarget` / `SelectTargetList`, `DoCast*`, `DoSpellAttackIfReady`, hooks (`JustEnteredCombat`, `JustExitedCombat`, `OnDespawn`, `DamageDealt`, `DamageTaken`, `HealReceived`, `HealDone`, `SpellInterrupted`, `OnGameEvent`, `OnCharmed`) |
| `src/server/game/AI/CoreAI/UnitAI.cpp` | 401 | `SelectTarget(s)` impl with threat list + predicates + dist filters; `DoCast` overloads; `AttackStartCaster` |
| `src/server/game/AI/CoreAI/UnitAICommon.h` | 117 | Shared predicates: `NonTankTargetSelector`, `FarthestTargetSelector`, `PowerUsersSelector`, `HealthPctOrderPred` |
| `src/server/game/AI/CoreAI/UnitAICommon.cpp` | 186 | Predicate implementations |
| `src/server/game/AI/CoreAI/enuminfo_UnitAICommon.cpp` | 73 | Generated reflection enum |
| `src/server/game/AI/CreatureAI.h` | 260 | `class CreatureAI : UnitAI` — adds creature hooks (`Reset`, `JustEngagedWith`, `JustDied`, `KilledUnit`, `MoveInLineOfSight`, `TriggerAlert`, `EnterEvadeMode`, `SpellHit`, `SpellHitTarget`, `JustSummoned`, `IsSummonedBy`, `SummonedCreatureDies`, `JustReachedHome`, `ReceiveEmote`, `MovementInform`, `OnHealthDepleted`, `DoZoneInCombat`); `enum EvadeReason` |
| `src/server/game/AI/CreatureAI.cpp` | 464 | `EnterEvadeMode` default flow (`Reset` → `MoveTargetedHome` → `JustReachedHome`); `DoZoneInCombat` (visit grid, `EngageWithTarget`); `TriggerAlert`; `_EnterEvadeMode`; emote/spell defaults |
| `src/server/game/AI/CreatureAIImpl.h` | 102 | Templates for registering AI factories (`FactoryHolder` instantiation) |
| `src/server/game/AI/CreatureAIFactory.h` | 49 | `FactoryHolder<CreatureAI, Creature, std::string>` and `GameObjectAIFactory` registry pattern |
| `src/server/game/AI/CreatureAIRegistry.h` | 25 | Registry singleton declaration |
| `src/server/game/AI/CreatureAIRegistry.cpp` | 63 | Registers every stock AI factory at startup (`RegisterCreatureAI<NullCreatureAI>("NullCreatureAI")`, etc.) |
| `src/server/game/AI/CreatureAISelector.h` | 44 | `selectAI(Creature*)`, `SelectMovementGenerator(Creature*)` declarations |
| `src/server/game/AI/CreatureAISelector.cpp` | 190 | The selection algorithm: Pet → `PetAI`; Totem → `TotemAI`; Vehicle → `VehicleAI`; row in `smart_scripts` → `SmartAI`; else `creature_template.AIName` factory; fallback `NullCreatureAI` |
| `src/server/game/AI/SelectableAI.h` | 45 | Trait base mixin for things that can be picked by `selectAI` |
| `src/server/game/AI/AIException.h` | 35 | Exception type for AI factory load errors |
| `src/server/game/AI/CoreAI/PassiveAI.h` | 93 | `class PassiveAI : CreatureAI`, `class PossessedAI`, `class CritterAI`, `class TriggerAI`, `class NullCreatureAI` — all "do nothing" variants with subtle differences |
| `src/server/game/AI/CoreAI/PassiveAI.cpp` | 119 | Trivial overrides; `CritterAI::DamageTaken` flees; `PossessedAI` redirects target via charmer |
| `src/server/game/AI/CoreAI/CombatAI.h` | 107 | `class AggressorAI : CreatureAI`, `class CombatAI : CreatureAI`, `class CasterAI : CombatAI`, `class ArcherAI : CreatureAI`, `class TurretAI : CreatureAI`, `class VehicleAI : CreatureAI` |
| `src/server/game/AI/CoreAI/CombatAI.cpp` | 309 | `CombatAI::InitSpellList()` reads `creature_template_addon` spell list; `UpdateAI` rotates with `EventMap` |
| `src/server/game/AI/CoreAI/PetAI.h` | 75 | `class PetAI : CreatureAI` + `SpellAttackEntry` cooldown record |
| `src/server/game/AI/CoreAI/PetAI.cpp` | 651 | Stay/follow/attack command dispatch; `Aggressive`/`Defensive`/`Passive` react states; pet-bar spell rotation; charm dispatch; owner aggro inheritance |
| `src/server/game/AI/CoreAI/TotemAI.h` | 39 | `class TotemAI : NullCreatureAI` |
| `src/server/game/AI/CoreAI/TotemAI.cpp` | 87 | Lifespan tracker; cast totem spell on cooldown; despawn on expire |
| `src/server/game/AI/CoreAI/GuardAI.h` | 37 | `class GuardAI : CombatAI` |
| `src/server/game/AI/CoreAI/GuardAI.cpp` | 69 | Calls `CallAssistance` on attack so other guards in 50yd join |
| `src/server/game/AI/CoreAI/ReactorAI.h` | 33 | `class ReactorAI : CombatAI` |
| `src/server/game/AI/CoreAI/ReactorAI.cpp` | 32 | Disables `MoveInLineOfSight` aggro radius; only attacks if attacked |
| `src/server/game/AI/CoreAI/ScheduledChangeAI.h` | 40 | `class ScheduledChangeAI : CreatureAI` |
| `src/server/game/AI/CoreAI/ScheduledChangeAI.cpp` | 22 | Used to swap controller (boss phase transitions); itself a no-op AI until `Creature::AIM_Initialize` rebuilds the real one |
| `src/server/game/AI/CoreAI/AreaTriggerAI.h` | 79 | `class AreaTriggerAI` for client-side area triggers |
| `src/server/game/AI/CoreAI/AreaTriggerAI.cpp` | 28 | Default AreaTrigger AI |
| `src/server/game/AI/CoreAI/GameObjectAI.h` | 134 | `class GameObjectAI` for chests/traps/switches |
| `src/server/game/AI/CoreAI/GameObjectAI.cpp` | 42 | Default GO AI |
| `src/server/game/AI/ScriptedAI/ScriptedCreature.h` (lines defining helpers) | 428 (full file) | `class EventMap` + `class SummonList` declared here (no separate file in TC) |
| `src/server/game/AI/ScriptedAI/ScriptedCreature.cpp` (helper sections) | 714 (full file) | `EventMap::ScheduleEvent / Update / ExecuteEvent / SetPhase / Reset / CancelEvent / RescheduleEvent / DelayEvents`; `SummonList::Summon / DespawnEntry / DespawnAll / DespawnIf / IsAnyCreatureAlive / DoAction / DoZoneInCombat` |

**Sub-module total (CoreAI/ + CreatureAI/* + selector + helpers in ScriptedCreature):** ~3,800 lines.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `UnitAI` | abstract class | Root of AI hierarchy; `Unit* const me`; `UpdateAI=0`; target selection helpers; cast helpers |
| `CreatureAI` | class : UnitAI | Adds creature-specific hooks; default `EnterEvadeMode` flow |
| `NullCreatureAI` | class : CreatureAI | All hooks no-op; baseline fallback when nothing else applies |
| `PassiveAI` | class : CreatureAI | Like Null but stops attacks; reacts only to `EnterEvadeMode` |
| `PossessedAI` | class : CreatureAI | Used when a creature is charmed by a player; reroutes commands via charmer |
| `CritterAI` | class : NullCreatureAI | Rabbits/rats/etc.; `DamageTaken` triggers flee, then despawn |
| `TriggerAI` | class : NullCreatureAI | Invisible visual-only summons; never engage |
| `CombatAI` | class : CreatureAI | Stock combat — auto-attack + `EventMap`-driven rotation of spells from `creature_template_addon.auras` |
| `AggressorAI` | class : CreatureAI | Standard aggro mob, no spells |
| `CasterAI` | class : CombatAI | Caster mob — keeps distance, spell rotation, melee fallback |
| `ArcherAI` | class : CreatureAI | Bow-only ranged turret |
| `TurretAI` | class : CreatureAI | Stationary spell turret |
| `VehicleAI` | class : CreatureAI | Vehicle without active driver — idle until mounted |
| `PetAI` | class : CreatureAI | Player pet behaviour |
| `TotemAI` | class : NullCreatureAI | Totem cast-on-cd + lifespan |
| `GuardAI` | class : CombatAI | City guard with `CallAssistance` |
| `ReactorAI` | class : CombatAI | Only attacks when attacked first |
| `ScheduledChangeAI` | class : CreatureAI | Placeholder during pending AI swap (phase transitions) |
| `AreaTriggerAI` | class | AI for `AreaTrigger` objects |
| `GameObjectAI` | class | AI for `GameObject` (chests/traps/switches) |
| `EventMap` | class | Helper inside `ScriptedAI`: `_eventMap: std::multimap<uint32 deadline, uint64 packed (eventId|group|phase)>`; `_phase: uint8`; `_time: uint32` |
| `SummonList` | class | `GuidList` of active summons owned by a script; iterators + bulk despawn helpers |
| `EvadeReason` | enum | `NoHostiles`, `Boundary`, `SequenceBreak`, `NoPath`, `Other` |
| `SelectTargetMethod` | enum | `Random`, `MaxThreat`, `MinThreat`, `MaxDistance`, `MinDistance` |
| `AISpellInfoType` | struct | Per-spell metadata (target, cooldown, conditions); cached at startup by `UnitAI::FillAISpellInfo` |
| `FactoryHolder<AI, Object, Key>` | template | Generic factory registry; instantiated as `FactoryHolder<CreatureAI, Creature, std::string>` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `UnitAI::UpdateAI(uint32 diff)` | Abstract per-tick driver | (subclass-specific) |
| `UnitAI::AttackStart(Unit* target)` | Set victim + start melee chase | `Unit::Attack`, `MotionMaster::MoveChase` |
| `UnitAI::SelectTarget(method, offset, dist, playerOnly, withTank, aura)` | Pick one target from threat list per criteria | `ThreatManager`, `PrepareTargetListSelection` |
| `UnitAI::SelectTargetList(out, num, method, offset, dist, ...)` | Pick N targets | idem |
| `UnitAI::DoCast(spellId)` / `DoCast(victim, spellId, args)` / `DoCastVictim` / `DoCastSelf` / `DoCastAOE` | Cast helpers checking range/cooldown | `Unit::CastSpell` |
| `UnitAI::DoSpellAttackIfReady(spellId)` | Cast if cooldown ready and in range | `SpellHistory::HasCooldown`, `Unit::CastSpell` |
| `UnitAI::DamageDealt / DamageTaken / HealReceived / HealDone / SpellInterrupted` | Hooks into damage/heal pipelines | (override) |
| `UnitAI::OnCharmed(bool isNew)` | Re-evaluate AI when charm state changes | `Unit::ScheduleAIChange` |
| `CreatureAI::Reset()` | Reset script state on evade/respawn | (subclass override) |
| `CreatureAI::JustEnteredCombat(Unit* who)` | Generic combat entry hook | (override) |
| `CreatureAI::JustEngagedWith(Unit* who)` | Real first-attacker hook (post-threat) | (override) — boss timers start here |
| `CreatureAI::JustDied(Unit* killer)` | Death hook — drop loot, despawn summons, achievement criteria | (override) |
| `CreatureAI::KilledUnit(Unit* victim)` | We just killed something | (override) |
| `CreatureAI::MoveInLineOfSight(Unit* who)` | Aggro detection on entry to visibility range | `IsHostileTo`, `Attack`, `TriggerAlert` |
| `CreatureAI::TriggerAlert(Unit const* who)` | Pre-aggro "?" emote + stand still | `SetReactState`, `SendPlaySpellVisualKit` |
| `CreatureAI::EnterEvadeMode(EvadeReason)` | Leave combat — full reset (HP, auras, summons), `MoveTargetedHome` | `Reset`, `_EnterEvadeMode`, `Creature::SetWalk`, `MotionMaster::MoveTargetedHome` |
| `CreatureAI::SpellHit(WorldObject* caster, SpellInfo const*)` | We were hit | (override) |
| `CreatureAI::SpellHitTarget(WorldObject* target, SpellInfo const*)` | We hit something with a spell | (override) |
| `CreatureAI::JustSummoned(Creature* summon)` | We summoned `summon`; default registers in `SummonList` | (override) |
| `CreatureAI::IsSummonedBy(WorldObject* summoner)` | We were summoned | (override) |
| `CreatureAI::SummonedCreatureDies(Creature*, Unit*)` | One of our summons died | (override) |
| `CreatureAI::JustReachedHome()` | Returned to spawn after evade | (override) |
| `CreatureAI::ReceiveEmote(Player*, uint32 emoteId)` | Player /waved at us etc. | (override) |
| `CreatureAI::MovementInform(uint32 type, uint32 id)` | Movement generator finished | (override) |
| `CreatureAI::DoZoneInCombat(Creature* source = me, float range = 0.0f)` | Pull all hostiles in zone (raid wipe trigger) | grid visitor + `EngageWithTarget` |
| `CombatAI::InitSpellList()` | Load spells from `creature_template_addon` | `sObjectMgr->GetCreatureTemplateAddon` |
| `CombatAI::UpdateAI(diff)` | Spell rotation via `EventMap` | `events.Update`, `events.ExecuteEvent`, `DoCastVictim` |
| `PetAI::UpdateAI(diff)` | Stay/follow/attack command + react state + spell rotation | `MotionMaster::MoveFollow`, `DoSpellAttackIfReady` |
| `PetAI::ReceiveEmote / SpellHit / OwnerAttackedBy / OwnerAttacked` | Owner-aggro inheritance | `Attack`, `Pet::SetIsCommandAttack` |
| `TotemAI::UpdateAI(diff)` | Cast totem spell on cooldown; check lifespan | `DoCast` |
| `GuardAI::JustEnteredCombat(Unit*)` | Call other guards | `Creature::CallAssistance` |
| `ReactorAI::MoveInLineOfSight(Unit*)` | (overridden no-op so no aggro radius) | — |
| `EventMap::ScheduleEvent(eventId, time_ms, group=0, phase=0)` | Insert into deadline-ordered map | — |
| `EventMap::Update(uint32 diff)` | Advance internal `_time` | — |
| `EventMap::ExecuteEvent()` | Pop next ready event in current phase | — |
| `EventMap::SetPhase(uint8 phase)` | Filter events by phase mask | — |
| `EventMap::CancelEvent(uint32 eventId)` / `CancelEventGroup(uint32 group)` | Remove pending entries | — |
| `EventMap::RescheduleEvent(eventId, time, group=0, phase=0)` | Cancel + reschedule | — |
| `EventMap::DelayEvents(uint32 delay)` / `DelayEvents(uint32 delay, uint32 group)` | Push deadlines back (e.g. interrupt cast) | — |
| `EventMap::Reset()` | Clear everything (encounter restart) | — |
| `SummonList::Summon(Creature*)` | Register summon GUID | — |
| `SummonList::DespawnEntry(uint32 entry)` | Despawn all summons of given template | iterate, `Creature::DespawnOrUnsummon` |
| `SummonList::DespawnAll(uint32 timeMs = 0)` | Despawn every tracked summon | — |
| `SummonList::DespawnIf(predicate)` | Conditional despawn | — |
| `SummonList::IsAnyCreatureAlive()` | Encounter-completion check | — |
| `SummonList::DoAction(int32 action, predicate)` | Dispatch `DoAction` to selected summons | `CreatureAI::DoAction` |
| `CreatureAISelector::selectAI(Creature*)` | Decide controller at spawn | check Pet/Totem/Vehicle, then SmartScripts presence, then template `AIName`, fallback `NullCreatureAI` |
| `CreatureAISelector::SelectMovementGenerator(Creature*)` | Pick initial MotionMaster generator | `MovementGeneratorRegistry` |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Unit` — `Unit* me` baseline; threat list, victim, faction, hostility
- `Entities/Creature` — every `CreatureAI` is owned by a `Creature`; uses `me->GetMotionMaster()`, `me->GetThreatManager()`, `me->CastSpell`, `me->GetVictim`, `me->GetCreatureTemplate`, `creature_template_addon`
- `Entities/Player` — gossip, charm, pet ownership
- `Spells` — `Unit::CastSpell`, `SpellInfo`, `SpellHistory` for cooldowns, hit hooks
- `Combat` — `ThreatManager` (drives `SelectTarget`), `DamageInfo`, `HealInfo`, `DamageDealt`/`DamageTaken` hooks
- `Movement` — `MotionMaster::MoveChase`, `MoveTargetedHome`, `MoveFollow`, `MovePoint`, `MoveJump`; `MovementInform` callback
- `Maps` — `Map::VisitNearbyCellsOf` for `DoZoneInCombat` and `CallAssistance`
- `Database` — `creature_template_addon` (CombatAI spell list), `creature_addon` (per-spawn auras/mount/emote), `smart_scripts` presence-check (driven by selector), legacy `creature_ai_template/scripts/summons` (deprecated)
- `Achievements` — `KilledUnit` and `JustDied` feed criteria
- `ScriptMgr` — boss/quest C++ scripts register their factory through the same `FactoryHolder` infra
- `OutdoorPvP` / `Battleground` / `Instance` — `JustDied`/`JustEngagedWith` trigger encounter state changes
- `Loot` — `JustDied` triggers `Creature::SetLootRecipient` consumers
- `Pets` — `PetAI` reads pet stats/talents/spell list

**Depended on by:**
- `Entities/Creature` — `Creature` holds `CreatureAI* i_AI`; `AIM_Initialize` instantiates via `CreatureAISelector::selectAI`
- `World/MapUpdater` — `Map::Update` → `Creature::Update` → `i_AI->UpdateAI(diff)`
- `Combat/Damage pipeline` — calls `i_AI->DamageTaken`/`DamageDealt`/`HealReceived` before applying
- `Spells` — on hit dispatches `i_AI->SpellHit`/`SpellHitTarget`
- `Movement` — `MotionMaster` callbacks `i_AI->MovementInform`
- `Scripting` — boss scripts (`scripts.md`) register their factory in `CreatureAIRegistry`
- `Pets` — `Pet::Initialize` builds `PetAI`
- `SmartScripts` (sibling sub-module) — `SmartAI` itself **is** a `CreatureAI` subclass; relies on this hierarchy

---

## 6. SQL / DB queries (if any)

Base-AI sub-module mostly **consumes** state populated by other modules; it issues few queries directly. Direct reads are limited to the selector/factory boot path:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, AIName FROM creature_template` (column `AIName`) | Selector picks the factory key from this column when no smart_scripts present | world |
| `SELECT entry, ScriptName FROM creature_template` | If non-empty, script registry produces the AI (boss scripts) | world |
| `SELECT entry, path_id, mount_creature_id, bytes1, bytes2, emote, visibility_distance_type, auras FROM creature_template_addon` | `CombatAI::InitSpellList` reads `auras` column for spell rotation | world |
| `SELECT guid, path_id, mount_creature_id, bytes1, bytes2, emote, visibility_distance_type, auras FROM creature_addon` | Per-spawn override (auras applied on spawn, mount, stand state) | world |
| `SELECT id, summonerId, summonerType, groupId, entry, position_x, position_y, position_z, orientation, summonType, summonTime FROM creature_summon_groups` | Used by `Creature::SummonCreatureGroup` consumed by `CreatureAI::JustSummoned` registration in `SummonList` | world |
| `SELECT entry, MoveType, Behavior FROM script_waypoint` | Legacy escort waypoints (deprecated; SmartAI uses `waypoint_data`) | world |
| (presence check only) `SELECT 1 FROM smart_scripts WHERE entryorguid = ? OR entryorguid = -? LIMIT 1` | Conceptual — selector decides whether to instantiate `SmartAI`; in practice the cache is preloaded by `SmartScriptMgr` and the selector queries memory | world |

No DB2/DBC stores are owned by this sub-module.

---

## 7. Wire-protocol packets (if any)

The base AI sub-module **does not own opcodes**. Side effects flow through other modules (Movement, Spells, Chat). Relevant outbound opcodes that base AI causes via its hooks:

| Opcode | Direction | Sent by base AI via |
|---|---|---|
| `SMSG_AI_REACTION` | server → client | `JustEnteredCombat` → cliente shows red name; reaction `AI_REACTION_HOSTILE` |
| `SMSG_PLAY_SPELL_VISUAL_KIT` | server → client | `CreatureAI::TriggerAlert` (the "?" alert above head) |
| `SMSG_EMOTE` / `SMSG_TEXT_EMOTE` | server → client | `Unit::HandleEmoteCommand` invoked from emote helpers |
| `SMSG_ON_MONSTER_MOVE` | server → client | Any `MotionMaster::Move*` triggered by AI (chase, target, point, jump, home) |
| `SMSG_PARTY_KILL_LOG` | server → client | `KilledUnit` broadcast to party |
| `SMSG_THREAT_UPDATE` / `SMSG_HIGHEST_THREAT_UPDATE` | server → client | Caused indirectly via `ThreatManager` updates triggered by AI engagement |
| `CMSG_PET_ACTION` / `CMSG_PET_CAST_SPELL` | client → server | `PetAI` is the consumer (stay/follow/attack/cast dispatch) |
| `CMSG_PET_SET_ACTION` | client → server | `PetAI` reaction-state changes (Aggressive/Defensive/Passive) |
| `CMSG_GOSSIP_HELLO` / `CMSG_GOSSIP_SELECT_OPTION` | client → server | `CreatureAI::ReceiveGossipHello` / `ReceiveGossipSelect` overrides |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-ai/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-ai/src/lib.rs` — **346 lines**, single concrete `struct CreatureAI` and one `enum CreatureState { Idle, WalkingRandom, WalkingWaypoint, InCombat, Dead, Returning }`
- `crates/wow-ai/Cargo.toml` — workspace member
- `crates/wow-script/src/lib.rs` — 0 lines (empty placeholder; relevant for the sibling `ai-smartscripts.md`)
- `crates/wow-world/src/session.rs` and the new `crates/wow-world/src/map_manager.rs` keep a `WorldCreature` value next to combat state — overlapping responsibility with what should be `CreatureAI`

**What's implemented (covers ~5% of base AI):**
- One state machine state set (Idle / WalkingRandom / WalkingWaypoint / InCombat / Dead / Returning)
- Plain methods on the concrete struct: `new`, `try_aggro` (flat distance check, no faction/LOS), `enter_combat`, `reset_combat` (instant full HP + return home), `take_damage`, `die`, `should_respawn`, `respawn`, `movement_finished`, `interpolated_position`, `begin_move`, `finish_move`, `can_swing`, `record_swing`, `roll_damage`, `should_wander`, `pick_wander_destination`, `reset_wander_timer`
- A pseudo-deterministic random based on `Instant::now().subsec_nanos()` (not seeded, not real RNG)

**What's missing vs C++:**
1. **No `trait UnitAI` or `trait CreatureAI`** — single concrete struct cannot represent the polymorphism required for stock subclasses
2. **No factory / registry / selector** — no `CreatureAISelector::select_ai`, no `inventory::submit!` of factories, `creature_template.AIName` and `ScriptName` are both unused
3. **No stock subclasses**: `NullCreatureAI`, `PassiveAI`, `PossessedAI`, `CritterAI`, `TriggerAI`, `CombatAI`, `AggressorAI`, `CasterAI`, `ArcherAI`, `TurretAI`, `VehicleAI`, `PetAI`, `TotemAI`, `GuardAI`, `ReactorAI`, `ScheduledChangeAI`, `AreaTriggerAI`, `GameObjectAI`
4. **No `EventMap`** — boss timer/event queue (used by literally every boss script in C++) — no `schedule_event`, no `execute_event`, no `set_phase`, no group/phase masks
5. **No `SummonList`** — boss summons cannot be tracked or cleaned up on death
6. **No `EvadeReason` enum** — `reset_combat` is a single trivial path
7. **No `EnterEvadeMode` proper flow** — current `reset_combat` instantly heals + teleports; C++ goes `Reset → MoveTargetedHome → JustReachedHome → restore HP/auras/summons`; intermediate hooks are skipped
8. **No `MoveInLineOfSight` + `TriggerAlert` two-phase aggro** — current `try_aggro` does a flat plane distance check; no curiosity "?" alert, no level-diff modifier, no faction/hostility/visibility/stealth gating
9. **No hooks**: `JustEnteredCombat`, `JustEngagedWith`, `JustDied`, `KilledUnit`, `JustReachedHome`, `JustSummoned`, `IsSummonedBy`, `SummonedCreatureDies`, `MovementInform`, `SpellHit`, `SpellHitTarget`, `OnHealthDepleted`, `ReceiveEmote`, `OnGameEvent`, `DamageDealt`, `DamageTaken`, `HealReceived`, `HealDone`, `SpellInterrupted`, `OnCharmed`, `OnDespawn`
10. **No target selection** — no `SelectTarget(method, dist, predicates)` walking the threat list; no `SelectTargetMethod` enum
11. **No `DoSpellAttackIfReady` / `DoCast*`** — spell pipeline integration absent
12. **No `DoZoneInCombat`** — raid pull cannot be modelled
13. **No `CallAssistance` / `CallForHelp`** — guard chains and pack pulls don't exist
14. **No charm/possess swap path** — `OnCharmed` cannot swap controller to `PossessedAI`
15. **No pet command dispatch** (Stay / Follow / Attack / Passive / Defensive / Aggressive)
16. **No totem lifespan / cd-cast loop**
17. **No reactor mode** (always-aggro instead)
18. **No per-template aura application from `creature_template_addon` on spawn**
19. **`creature_template_addon` and `creature_addon` are not loaded by the AI sub-module** — `CombatAI::InitSpellList` has no equivalent
20. **No `creature_summon_groups` integration** — group-summon spawns from boss intros/adds don't work

**Suspicious / likely divergent:**
- `try_aggro` ignores faction, LOS, stealth, level-diff aggro distance
- `reset_combat` heals to full and teleports home in one step; bypasses MoveTargetedHome animation and `JustReachedHome` hook
- `swing_timer_ms` hardcoded 2000 ms; C++ uses `BASE_ATTACK` weapon speed from template — desyncs for mobs with other weapon delays
- Random via `subsec_nanos` is high-correlation between successive calls; can degenerate to identical wander destinations
- `respawn_time_secs` default 30 — far too low; real spawn data is per-spawn (3–7 min for mobs, longer for rares/bosses)

**Tests existing:** 0 tests in `crates/wow-ai/`.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#AI_BASE.WBS.001** Cerrar la migracion auditada de `game/AI/CoreAI/AreaTriggerAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/AreaTriggerAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.002** Cerrar la migracion auditada de `game/AI/CoreAI/AreaTriggerAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/AreaTriggerAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.003** Cerrar la migracion auditada de `game/AI/CoreAI/CombatAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/CombatAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.004** Cerrar la migracion auditada de `game/AI/CoreAI/CombatAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/CombatAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.005** Cerrar la migracion auditada de `game/AI/CoreAI/GameObjectAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/GameObjectAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.006** Cerrar la migracion auditada de `game/AI/CoreAI/GameObjectAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/GameObjectAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.007** Cerrar la migracion auditada de `game/AI/CoreAI/GuardAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/GuardAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.008** Cerrar la migracion auditada de `game/AI/CoreAI/GuardAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/GuardAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.009** Cerrar la migracion auditada de `game/AI/CoreAI/PassiveAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PassiveAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.010** Cerrar la migracion auditada de `game/AI/CoreAI/PassiveAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PassiveAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.011** Partir y cerrar la migracion auditada de `game/AI/CoreAI/PetAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 651 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.012** Cerrar la migracion auditada de `game/AI/CoreAI/PetAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.013** Cerrar la migracion auditada de `game/AI/CoreAI/ReactorAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/ReactorAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.014** Cerrar la migracion auditada de `game/AI/CoreAI/ReactorAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/ReactorAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.015** Cerrar la migracion auditada de `game/AI/CoreAI/ScheduledChangeAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/ScheduledChangeAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.016** Cerrar la migracion auditada de `game/AI/CoreAI/ScheduledChangeAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/ScheduledChangeAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.017** Cerrar la migracion auditada de `game/AI/CoreAI/TotemAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/TotemAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.018** Cerrar la migracion auditada de `game/AI/CoreAI/TotemAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/TotemAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.019** Cerrar la migracion auditada de `game/AI/CoreAI/UnitAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/UnitAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.020** Cerrar la migracion auditada de `game/AI/CoreAI/UnitAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/UnitAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.021** Cerrar la migracion auditada de `game/AI/CoreAI/UnitAICommon.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/UnitAICommon.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.022** Cerrar la migracion auditada de `game/AI/CoreAI/UnitAICommon.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/UnitAICommon.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.023** Cerrar la migracion auditada de `game/AI/CoreAI/enuminfo_UnitAICommon.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/enuminfo_UnitAICommon.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.024** Partir y cerrar la migracion auditada de `game/AI/PlayerAI/PlayerAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1307 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.025** Cerrar la migracion auditada de `game/AI/PlayerAI/PlayerAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.026** Partir y cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedCreature.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 714 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.027** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedCreature.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.028** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedEscortAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedEscortAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.029** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedEscortAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedEscortAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.030** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedFollowerAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedFollowerAI.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.031** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedFollowerAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedFollowerAI.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.032** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedGossip.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedGossip.cpp`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI_BASE.WBS.033** Cerrar la migracion auditada de `game/AI/ScriptedAI/ScriptedGossip.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedGossip.h`
  Rust target: `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** <1h, **M** 1–4h, **H** 4–12h, **XL** >12h (split before tackling).

- [ ] **#AI-BASE.1** Create `crates/wow-ai/src/unit_ai.rs` — `trait UnitAI { fn update_ai(&mut self, diff_ms: u32); fn reset(&mut self) {} fn just_entered_combat(&mut self, who: &Unit) {} fn just_exited_combat(&mut self) {} fn on_despawn(&mut self) {} fn damage_dealt(&mut self, victim, dmg, ty) {} fn damage_taken(&mut self, attacker, dmg, ty, spell) {} fn heal_received(&mut self, by, amount) {} fn heal_done(&mut self, to, amount) {} fn spell_interrupted(&mut self, spell_id, time) {} fn on_charmed(&mut self, is_new) {} fn on_game_event(&mut self, start, event_id) {} }` (M)
- [ ] **#AI-BASE.2** Create `crates/wow-ai/src/creature_ai.rs` — `trait CreatureAI: UnitAI { /* all CreatureAI hooks default no-op */ }` (M)
- [ ] **#AI-BASE.3** `enum EvadeReason { NoHostiles, Boundary, SequenceBreak, NoPath, Other }` (L)
- [ ] **#AI-BASE.4** `enum SelectTargetMethod { Random, MaxThreat, MinThreat, MaxDistance, MinDistance }` (L)
- [ ] **#AI-BASE.5** Move existing `struct CreatureAI` → `struct DefaultCreatureAI` and `impl CreatureAI for DefaultCreatureAI` (backward-compat refactor, no behaviour change) (M)
- [ ] **#AI-BASE.6** Implement `struct NullCreatureAI` impl `CreatureAI` — every method no-op (L)
- [ ] **#AI-BASE.7** Implement `struct PassiveAI`, `struct CritterAI`, `struct TriggerAI`, `struct PossessedAI` — all mostly stubs (M)
- [ ] **#AI-BASE.8** Implement `struct EventMap` in `crates/wow-ai/src/event_map.rs`: `events: BTreeMap<u32 deadline_ms, EventEntry { id: u32, group: u8, phase_mask: u16 }>`, `time_ms: u32`, `phase: u8`; methods `schedule_event`, `update`, `execute_event`, `set_phase`, `cancel_event`, `cancel_event_group`, `reschedule_event`, `delay_events`, `delay_events_in_group`, `reset`, `is_in_phase` (H)
- [ ] **#AI-BASE.9** Implement `struct SummonList` in `crates/wow-ai/src/summon_list.rs`: `Vec<ObjectGuid>`, `summon`, `despawn_all`, `despawn_entry`, `despawn_if`, `is_any_creature_alive`, `do_action` (M)
- [ ] **#AI-BASE.10** Implement `EnterEvadeMode(reason)` proper flow: set evading state → `Reset` → `move_targeted_home()` → on `MovementInform(Home)` callback `JustReachedHome` + restore HP + clear auras + `SummonList::despawn_all` + `ThreatManager::clear` (M)
- [ ] **#AI-BASE.11** Implement `TriggerAlert` (visual "?" + emote + 5s delay before real aggro) inside `MoveInLineOfSight` (L)
- [ ] **#AI-BASE.12** Implement level-diff aggro radius modifier (mob_level vs player_level → ±2yd per level) in `MoveInLineOfSight` (L)
- [ ] **#AI-BASE.13** Implement `DoZoneInCombat(source, range)` — visit grid, `EngageWithTarget` for every hostile in range (M)
- [ ] **#AI-BASE.14** Implement `struct CombatAI` impl `CreatureAI` — read `creature_template_addon.auras`, build `EventMap` rotation, `UpdateAI` ticks (H)
- [ ] **#AI-BASE.15** Implement `struct AggressorAI` impl `CreatureAI` — auto-attack only, no spell list (L)
- [ ] **#AI-BASE.16** Implement `struct CasterAI` impl `CombatAI` — kite distance + spell rotation + melee fallback (M)
- [ ] **#AI-BASE.17** Implement `struct ArcherAI` and `struct TurretAI` impl `CreatureAI` (M)
- [ ] **#AI-BASE.18** Implement `struct VehicleAI` impl `CreatureAI` — passive while empty (M)
- [ ] **#AI-BASE.19** Implement `struct PetAI` impl `CreatureAI` — stay/follow/attack command, Aggressive/Defensive/Passive react state, owner aggro inheritance, pet bar spell rotation (XL — split: command dispatch / react state / spell rotation)
- [ ] **#AI-BASE.20** Implement `struct TotemAI` impl `CreatureAI` — lifespan despawn, cast totem spell on cooldown (M)
- [ ] **#AI-BASE.21** Implement `struct GuardAI` impl `CombatAI` — `CallAssistance` to other guards in 50yd on `JustEnteredCombat` (M)
- [ ] **#AI-BASE.22** Implement `struct ReactorAI` impl `CombatAI` — override `MoveInLineOfSight` to no-op (only attacks if attacked) (L)
- [ ] **#AI-BASE.23** Implement `struct ScheduledChangeAI` impl `CreatureAI` — placeholder until next AI-swap tick (boss phase transitions); `Creature::ScheduleAIChange` API in entities crate (M)
- [ ] **#AI-BASE.24** Implement `struct AreaTriggerAI` and `struct GameObjectAI` traits + default impls (M)
- [ ] **#AI-BASE.25** Implement `CreatureAISelector::select_ai(creature: &Creature) -> Box<dyn CreatureAI>`: Pet/Totem/Vehicle short-circuit; SmartScript presence (depends on `ai-smartscripts.md`); `creature_template.AIName` factory lookup; fallback `NullCreatureAI` (M)
- [ ] **#AI-BASE.26** Implement `inventory::submit!`-based `CreatureAIFactory { name: &str, ctor: fn(&Creature) -> Box<dyn CreatureAI> }` registry; register every stock AI (M)
- [ ] **#AI-BASE.27** Implement `ScriptName` binding glue so boss scripts in `crates/wow-scripts/` (see `scripts.md`) register their factory through the same registry (M)
- [ ] **#AI-BASE.28** Implement `UnitAI::select_target(method, offset, dist, player_only, with_tank, aura)` walking threat list; needs `ThreatManager` from `wow-combat` (M, blocked on combat)
- [ ] **#AI-BASE.29** Implement `UnitAI::do_cast / do_cast_victim / do_cast_self / do_cast_aoe / do_spell_attack_if_ready` (depends on `wow-spell` API) (M)
- [ ] **#AI-BASE.30** Wire all hooks from owners: `Combat::deal_damage` → `ai.damage_taken/damage_dealt`; `Spells::on_hit` → `ai.spell_hit/spell_hit_target`; `Movement::generator_finished` → `ai.movement_inform`; `Creature::die` → `ai.just_died` + `summons.despawn_all`; `Combat::engage` → `ai.just_entered_combat` + first-attacker → `ai.just_engaged_with` (H)
- [ ] **#AI-BASE.31** Implement charm/possess swap: `OnCharmed(true)` swaps controller to `PossessedAI`, preserve original `Box<dyn CreatureAI>` for restore on uncharm (H)
- [ ] **#AI-BASE.32** Replace `subsec_nanos` random with `rand::rngs::StdRng` per-map seeded; thread through wander/aggro/damage rolls (L)
- [ ] **#AI-BASE.33** Hook `creature_template_addon.auras` application on `Creature::Spawn` (M)
- [ ] **#AI-BASE.34** Wire `creature_summon_groups` consumption through `CreatureAI::JustSummoned` → `SummonList::summon` (M)
- [ ] **#AI-BASE.35** Tests: `EventMap` ordering, phase masks, group cancel, delay; `SummonList` despawn flows; `EnterEvadeMode` full restore; `MoveInLineOfSight` level-diff radius; `CreatureAISelector` decision matrix (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#AI_BASE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 33 files / 6592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp`. Rust target: `crates/wow-ai`. | `cargo test -p wow-ai` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#AI_BASE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 33 files / 6592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp`. Rust target: `crates/wow-ai`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#AI_BASE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 33 files / 6592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp`. Rust target: `crates/wow-ai`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#AI_BASE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 33 files / 6592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp`. Rust target: `crates/wow-ai`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `select_target(MaxThreat)` returns the unit at top of threat list
- [ ] Test: `select_target(Random, dist=20)` only returns targets ≤20yd away
- [ ] Test: `select_target(playerOnly=true)` rejects non-player units
- [ ] Test: `select_target(withTank=false)` skips current tank
- [ ] Test: `MoveInLineOfSight` with hostile player in aggro radius schedules `TriggerAlert` first, then aggro after delay
- [ ] Test: `MoveInLineOfSight` ignores friendly faction targets
- [ ] Test: aggro radius increases by ~+2yd per level when `mob_level > player_level + 4`
- [ ] Test: aggro radius is 0 (no auto-aggro) when `mob_level < player_level − 4`
- [ ] Test: `EnterEvadeMode(Boundary)` triggers `Reset`, `MoveTargetedHome`, then on home `JustReachedHome` + restore max HP + clear auras + despawn summons
- [ ] Test: `EnterEvadeMode(NoHostiles)` despawns all `SummonList` entries
- [ ] Test: `DoZoneInCombat(source, 100yd)` adds threat to every hostile inside 100yd
- [ ] Test: `EventMap.schedule_event(1, 5_000)` then `update(5_000)` then `execute_event()` returns `Some(1)`
- [ ] Test: `EventMap.set_phase(2)` filters out events whose phase mask lacks bit 2
- [ ] Test: `EventMap.cancel_event_group(group=3)` removes all entries with that group
- [ ] Test: `EventMap.delay_events(1_000)` shifts every deadline by +1000ms
- [ ] Test: `SummonList.summon(g)` then `despawn_all()` despawns g
- [ ] Test: `SummonList.despawn_entry(entry=12345)` only despawns matching entries
- [ ] Test: `SummonList.is_any_creature_alive()` returns true while at least one summon lives
- [ ] Test: `DoSpellAttackIfReady(spell)` casts iff cooldown ready and target in range
- [ ] Test: `JustDied(killer)` → `SummonList::despawn_all` runs by default
- [ ] Test: `OnCharmed(true)` swaps controller to `PossessedAI`; `OnCharmed(false)` restores original
- [ ] Test: `PetAI` with `command=Stay` does not move when owner moves
- [ ] Test: `PetAI` with `command=Follow` chases owner inside follow distance
- [ ] Test: `PetAI` with `react=Aggressive` engages hostile entering pet aggro radius
- [ ] Test: `PetAI` with `react=Passive` never attacks, even when struck
- [ ] Test: `TotemAI` despawns when `lifespan_ms` elapses
- [ ] Test: `TotemAI` casts totem spell every cooldown
- [ ] Test: `GuardAI::just_entered_combat(who)` causes guards in 50yd to engage `who`
- [ ] Test: `ReactorAI::move_in_line_of_sight` is a no-op (no aggro radius)
- [ ] Test: `CreatureAISelector::select_ai`: Pet → `PetAI`; Totem → `TotemAI`; Vehicle → `VehicleAI`; entry has smart_scripts → `SmartAI`; `AIName='AggressorAI'` → `AggressorAI`; nothing → `NullCreatureAI`
- [ ] Test: `CombatAI::init_spell_list` reads `creature_template_addon.auras` and produces non-empty rotation when present
- [ ] Test: `creature_addon.auras` are applied as auras on `Creature::Spawn`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 33 files / 6592 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp` | `crates/wow-ai/` \| 🔧 broken (rewrite needed) — single concrete `CreatureAI` struct, no trait, no factory, no stock subclasses, no `EventMap`/`SummonList` |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#AI_BASE.DIV.001` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 33 C++ files / 6592 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/PlayerAI/PlayerAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/ScriptedAI/ScriptedCreature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/PetAI.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **`me` semantic.** In TC, every `UnitAI` holds `Unit* const me` (non-owning back-pointer). In Rust this becomes either an `ObjectGuid` look-up through the world state each tick, or a borrowed `&mut Unit` parameter on every method. Storing a raw pointer/`Arc<Unit>` in the AI struct creates ownership cycles with the `Creature` that owns the AI. **Recommended pattern:** AI stores `ObjectGuid` only; `update_ai(&mut self, world: &mut World, diff: u32)` resolves the creature each call.
- **`EnterEvadeMode` vs despawn.** Evading restores HP/auras and goes home — the creature stays spawned. Despawn removes the entity and starts the respawn timer. Boss kills go `JustDied → loot drop → corpse linger → DespawnOrUnsummon` (despawn). Trash that drops aggro through leashing goes `EnterEvadeMode(Boundary) → Reset → MoveTargetedHome → JustReachedHome` (no despawn). Conflating them breaks respawn timers and loot.
- **`JustEnteredCombat` vs `JustEngagedWith`.** The first fires the moment any unit threatens us (including healing threat from a healer outside our LOS). The second fires only once a real attacker is established. Boss timers must use `JustEngagedWith` — using `JustEnteredCombat` causes encounters to start from a heal-pull. (CreatureAI.h:108–111.)
- **Leashing.** When a creature chases a player further than `leashRange` (default ~50yd from spawn), it must `EnterEvadeMode(Boundary)`. Without this, mobs follow players across continents. Each creature stores its leash anchor when entering combat. (CreatureAI.cpp:CheckBoundary.)
- **`SummonList` ownership of GUIDs.** `SummonList` only stores GUIDs; it does not own the `Creature*`. When a summon dies naturally, `JustDied` on the summon does **not** automatically remove it from the boss's `SummonList` — `SummonedCreatureDies` is the hook the boss uses to clean up if it cares. Many scripts skip this and rely on `DespawnAll` at the end. In Rust, prefer holding `Vec<ObjectGuid>` and pruning during `DespawnAll`.
- **Pet command stack.** `Pet` carries `m_savedReactState`, `m_isCommandAttack`, `m_isCommandFollow`. `PetAI::UpdateAI` reads these by priority: command_attack > command_follow > react_state. A proc that re-targets mid-command_attack drops the manual command — known historical bug pattern. (PetAI.cpp:65–160.)
- **Charm switching ownership of the original AI.** `OnCharmed(true)` must swap to `PossessedAI` while preserving the original `Box<dyn CreatureAI>`. Common bug: dropping the original and rebuilding `NullCreatureAI` on uncharm. The Rust port must store the original explicitly (`Option<Box<dyn CreatureAI>> previous_ai`), not rely on a registry round-trip.
- **`AISpellInfo` static cache.** `UnitAI::FillAISpellInfo` builds a static `unordered_map<pair<spellId, difficulty>, AISpellInfoType>` at startup with conditions/cooldowns/targets per spell. Used by `DoCast` and `DoSpellAttackIfReady`. In Rust, an `OnceLock<HashMap<(u32, Difficulty), AISpellInfoType>>` matches the pattern.
- **`EventMap` group/phase semantics.** `group` (u8 1–8) is for cancelling/delaying related events together (e.g. all "fire" abilities). `phase` (u8 1–8) gates execution by `_phase`. Both default to 0 ("any group / always"). Off-by-one in the bitmask is a recurring bug.
- **`ScheduledChangeAI` lifecycle.** Setting `Creature::ScheduleAIChange` does not immediately swap — the next `AIM_Initialize` tick does. During the gap, the placeholder `ScheduledChangeAI` fields all hooks as no-ops. If a phase transition action fires more events between `ScheduleAIChange` and the actual swap, those events are silently dropped.
- **`creature_template_addon.auras` is a space-separated list of spell IDs.** `CombatAI::InitSpellList` parses with whitespace tokenisation. Empty cells produce empty rotations (no error). The parser must skip 0 entries (legacy data has them).
- **Selector order matters.** Pet/Totem/Vehicle short-circuit must run **before** SmartScripts presence check, because a Pet that also has rows in `smart_scripts` should still be `PetAI`, not `SmartAI`. The reference order is `IsPet → IsTotem → IsVehicle → IsControlledByPlayer → smart_scripts → AIName → NullCreatureAI`.
- **WotLK 3.4.3 specifics.** No `AreaTriggerAI` complexity from MoP+ (in 3.4.3 area triggers are mostly server-side scripted via `AreaTriggerScript`, not the `AreaTriggerAI` class which is more developed in later branches). `ScheduledChangeAI` exists but is rarely used in 3.4.3 boss scripts (more common in MoP+ phase transitions).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class UnitAI` (abstract) | `trait UnitAI` in `crates/wow-ai/src/unit_ai.rs` | abstract `update_ai`, default no-ops for hooks |
| `class CreatureAI : UnitAI` | `trait CreatureAI: UnitAI` | sub-trait; default impls for creature hooks |
| `Unit* const me` | `ObjectGuid` field; resolve through `World` each tick | avoid lifetime tangles |
| `CreatureAI* i_AI` (member of Creature) | `Box<dyn CreatureAI>` field on `Creature` | swap on charm/possess |
| `class NullCreatureAI` | `struct NullCreatureAI` impl `CreatureAI` | unit struct |
| `class PassiveAI / CritterAI / TriggerAI / PossessedAI` | `struct PassiveAI / CritterAI / TriggerAI / PossessedAI` impl `CreatureAI` | composition, not inheritance |
| `class CombatAI` | `struct CombatAI { events: EventMap, spell_rotation: Vec<u32> }` impl `CreatureAI` | spell list from addon |
| `class PetAI` | `struct PetAI { command: PetCommandState, react: ReactState, spell_history: Vec<SpellAttackEntry> }` impl `CreatureAI` | enum-driven dispatch |
| `class TotemAI` | `struct TotemAI { spell_id: u32, lifespan_ms: u32, cast_cd_ms: u32, last_cast: u32 }` impl `CreatureAI` | — |
| `class GuardAI` | `struct GuardAI { base: CombatAI }` impl `CreatureAI` | composition forwards to base; calls `CallAssistance` in `just_entered_combat` |
| `class ReactorAI` | `struct ReactorAI { base: CombatAI }` impl `CreatureAI` | overrides `move_in_line_of_sight` no-op |
| `class ScheduledChangeAI` | `struct ScheduledChangeAI` impl `CreatureAI` | placeholder; pairs with `Creature::schedule_ai_change(name)` |
| `class VehicleAI` | `struct VehicleAI` impl `CreatureAI` | — |
| `class EventMap` | `struct EventMap { events: BTreeMap<u32, EventEntry>, time_ms: u32, phase: u8 }` | `BTreeMap` for ordered-by-deadline iteration |
| `class SummonList` | `struct SummonList(Vec<ObjectGuid>)` | plain Vec; `Set<>` not needed since duplicates impossible |
| `enum EvadeReason` | `enum EvadeReason { NoHostiles, Boundary, SequenceBreak, NoPath, Other }` | — |
| `enum SelectTargetMethod` | `enum SelectTargetMethod { Random, MaxThreat, MinThreat, MaxDistance, MinDistance }` | — |
| `FactoryHolder<CreatureAI, Creature, std::string>` | `inventory::submit!(CreatureAIFactory { name: "AggressorAI", ctor: |c| Box::new(AggressorAI::new(c)) })` | `inventory` crate already in workspace |
| `CreatureAISelector::selectAI(c)` | `fn select_ai(c: &Creature) -> Box<dyn CreatureAI>` | match cascade |
| `me->CastSpell(target, spellId, false)` | `world.cast_spell(self.guid, target, spell_id, CastFlags::empty())` | thread `World` through `update_ai` |
| `me->AI()->JustEnteredCombat(who)` | `creature.ai_mut().just_entered_combat(who)` | borrow-checker: drop other borrows first |
| `events.ScheduleEvent(EVENT_FIREBALL, 5s, GROUP_FIRE, PHASE_1)` | `self.events.schedule_event(EventId::Fireball as u32, 5_000, 1, 1)` | u32 event id; group/phase u8 |
| `events.ExecuteEvent()` | `self.events.execute_event() -> Option<u32>` | — |
| `summons.DespawnAll()` | `self.summons.despawn_all(world)` | needs `World` to actually despawn |
| `static AISpellInfo` | `static AI_SPELL_INFO: OnceLock<HashMap<(u32, Difficulty), AISpellInfoType>>` | `FillAISpellInfo` becomes startup init |

---

*Template version: 1.0.* Sub-document of `ai.md`. Last updated 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked the C++ canonical sub-tree at `/home/server/woltk-trinity-legacy/src/server/game/AI/CoreAI/`, `CreatureAI.{h,cpp}`, `CreatureAI{Impl,Factory,Registry,Selector}.*`, `SelectableAI.h`, `AIException.h`, plus the `EventMap`/`SummonList` sections of `ScriptedAI/ScriptedCreature.{h,cpp}`. Compared against `crates/wow-ai/` in `/home/server/rustycore/`.

**File inventory finding.** `EventMap` and `SummonList` are **not** in dedicated files in the TrinityCore tree — they live inside `ScriptedAI/ScriptedCreature.h`/`ScriptedCreature.cpp`. The migration brief in `ai.md` references "EventMap.{h,cpp}" and "SummonList.{h,cpp}" as standalone, which does not match the source layout. This sub-doc reports them as embedded helpers (the table in §2 is annotated). When porting, splitting them into `crates/wow-ai/src/event_map.rs` and `crates/wow-ai/src/summon_list.rs` is appropriate Rust style — that is a deliberate divergence, not a 1:1 layout port.

**Trait surface — none.** `crates/wow-ai/src/lib.rs` (346 lines) defines exactly **one** concrete `struct CreatureAI` and one `enum CreatureState`. There is no `trait UnitAI`, no `trait CreatureAI`, no `Box<dyn ...>` handle anywhere in the workspace, no `OnCharmed` / `JustEnteredCombat` / `JustDied` / `MoveInLineOfSight` / `EnterEvadeMode` / `MovementInform` / `SpellHit` / `JustSummoned` / `JustReachedHome` / `KilledUnit` / `ReceiveEmote` / `OnGameEvent` / `DamageDealt` / `DamageTaken` / `HealReceived` / `HealDone` / `SpellInterrupted` / `OnDespawn` hook. The 21 hook surface of `UnitAI` + `CreatureAI` is **0% covered**.

**Stock subclass coverage — 0/18.** None of `NullCreatureAI`, `PassiveAI`, `PossessedAI`, `CritterAI`, `TriggerAI`, `CombatAI`, `AggressorAI`, `CasterAI`, `ArcherAI`, `TurretAI`, `VehicleAI`, `PetAI`, `TotemAI`, `GuardAI`, `ReactorAI`, `ScheduledChangeAI`, `AreaTriggerAI`, `GameObjectAI` exist in the workspace. Pets, totems, guards, city NPCs, vehicles, summons, charmed creatures and reactor mobs all currently route through the same single `CreatureAI` struct and behave identically (random wander + flat-aggro melee).

**Helper containers — 0/2.** No `EventMap` (so no boss timer/event queue, no group cancel, no phase masks, no `RescheduleEvent`/`DelayEvents`), no `SummonList` (so summons cannot be tracked or cleaned up on death, `SummonedCreatureDies` cannot fire, encounter completion checks impossible). Every C++ boss script uses both — these two are gating dependencies for `ai-smartscripts.md` as well, since `SmartScript::ProcessAction` for `SMART_ACTION_SUMMON_*` requires `SummonList` semantics.

**Selector / factory / `AIName` / `ScriptName` — 0/4.** No `CreatureAISelector::select_ai`, no `FactoryHolder` analogue, no `inventory::submit!` registration of stock AIs, no consumption of `creature_template.AIName` or `creature_template.ScriptName`. Every spawn becomes the same struct regardless of its template. `CreatureAIRegistry` has no Rust counterpart. As a consequence, any future SmartAI work in `ai-smartscripts.md` will still need this sub-module's selector to route the right spawns into it — base AI is a hard prerequisite for smart AI.

**Evade flow — 1-step instead of 4-step.** C++ `EnterEvadeMode` is `Reset → MoveTargetedHome → MovementInform(Home) → JustReachedHome → restore HP/auras/threat`. Rust `reset_combat` collapses this to "set HP=max, set position=home, state=Idle". Skips intermediate hooks, skips animation, skips aura clearing, skips threat clearing, skips summon despawn, skips boundary/leash distinction. There is no `EvadeReason` enum, so leash vs no-hostiles vs sequence-break vs no-path are indistinguishable.

**Aggro detection — flat plane check.** `try_aggro` measures planar distance against a fixed `aggro_radius`. C++ aggregates: faction hostility (`IsHostileTo`), visibility (`CanSeeOrDetect`, including stealth), attack validity (`IsValidAttackTarget`), level-diff radius scaling (±2yd per level, `GetAggroRange`), per-creature alert state (`TriggerAlert` two-phase aggro). The Rust check would aggro a friendly stealthed level-80 player at 10yd from a level-1 critter — every guard rail is missing.

**`creature_template_addon` / `creature_addon` — unread.** `CombatAI::InitSpellList` is the canonical consumer of `creature_template_addon.auras` (space-separated spell ID list); without `CombatAI`, the column is silently ignored. `creature_addon.auras` (per-spawn aura application on `Creature::Spawn`) is also unread. Mounts, stand state, emote state, visibility distance — none flow into the AI at spawn time.

**Worst divergence (within base AI).** **The polymorphism is gone.** TrinityCore's design hinges on `Creature::AIM_Initialize` constructing one of ~18 concrete subclasses based on creature type, template, and SQL data, and on combat/spells/movement modules dispatching ~21 hook methods through a virtual interface. Rust has 0 of these. Even when SmartAI lands (per `ai-smartscripts.md`), it cannot plug into the engine because there is no trait for it to implement and no selector to route spawns to it. **Base AI must be ported before SmartAI can produce any visible behaviour** — they are siblings in source layout but base AI is upstream in the build order. Rough order-of-magnitude: 18 stock AIs × ~50–200 lines each + EventMap + SummonList + selector + factory ≈ 4–6k lines of new Rust to reach C++-equivalent base coverage, before any data-driven content is reachable.
