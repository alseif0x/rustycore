# Migration: AI

> **C++ canonical path:** `src/server/game/AI/` (incluye `CoreAI/`, `ScriptedAI/`, `SmartScripts/`, `PlayerAI/`)
> **Rust target crate(s):** `crates/wow-ai/`, `crates/wow-script/`, `crates/wow-scripts/`
> **Layer:** L5/L6 (game systems — creature behavior + scripting)
> **Status:** 🔧 broken (rewrite needed) — sólo `CreatureAI` plain con states Idle/Walk/Combat; sin trait, sin SmartAI, sin polimorfismo. Crates `wow-script` y `wow-scripts` están **vacíos**.
> **Audited vs C++:** ✅ audited 2026-05-01 (no SmartAI, see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

El módulo AI de TrinityCore es la capa que **decide qué hacen los NPCs cada tick**. Implementa una jerarquía polimórfica de "AI controllers" que se attachan a `Creature*` y reciben hooks de eventos del motor (`UpdateAI`, `JustEnteredCombat`, `KilledUnit`, `SpellHit`, `MoveInLineOfSight`, etc.). La gran mayoría de mobs/bosses del juego (~95%) se controla con **SmartAI** — un intérprete data-driven de la tabla `smart_scripts` (~50k filas) que mapea `event_type → action_type` con parámetros, sin compilar código C++. El resto se reparte entre subclases especializadas (PetAI, CombatAI, TotemAI, GuardAI, ReactorAI, PassiveAI, ScriptedAI para bosses custom). Es el módulo donde más vive la "personalidad" del contenido del juego, y por tanto el que más cambia con cada parche/expansión sin tocar core.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/AI/AIException.h` | 35 | `prefix` |
| `game/AI/CreatureAI.cpp` | 464 | `prefix` |
| `game/AI/CreatureAI.h` | 260 | `prefix` |
| `game/AI/CreatureAIFactory.h` | 49 | `prefix` |
| `game/AI/CreatureAIImpl.h` | 102 | `prefix` |
| `game/AI/CreatureAIRegistry.cpp` | 63 | `prefix` |
| `game/AI/CreatureAIRegistry.h` | 25 | `prefix` |
| `game/AI/CreatureAISelector.cpp` | 190 | `prefix` |
| `game/AI/CreatureAISelector.h` | 44 | `prefix` |
| `game/AI/GameObjectAIFactory.h` | 47 | `prefix` |
| `game/AI/SelectableAI.h` | 45 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/AI/CreatureAI.h` | 260 | `class CreatureAI : UnitAI` — base de toda creature AI; hooks virtuales (Reset, JustEnteredCombat, JustDied, MoveInLineOfSight, UpdateAI, etc.) |
| `src/server/game/AI/CreatureAI.cpp` | 464 | Implementación: `DoZoneInCombat`, `DoCast*`, `EnterEvadeMode` default, target selection helpers |
| `src/server/game/AI/CreatureAIImpl.h` | 102 | Templates para registrar AI factories (`CreatureAIRegistry`) |
| `src/server/game/AI/CreatureAIFactory.h` | 49 | `FactoryHolder<CreatureAI, Creature, std::string>` registry pattern |
| `src/server/game/AI/CreatureAIRegistry.cpp/.h` | 63 / 25 | Singleton registry de factories (NombreAI → ctor) |
| `src/server/game/AI/CreatureAISelector.cpp/.h` | 190 / 44 | Decide qué AI usar al spawnear (Pet→PetAI, Totem→TotemAI, Guardian→...; fallback a SmartAI si tiene smart_scripts; fallback final a NullCreatureAI) |
| `src/server/game/AI/SelectableAI.h` | 45 | Trait base para AIs registrables |
| `src/server/game/AI/AIException.h` | 35 | Exception type para errores de carga de AI |
| `src/server/game/AI/CoreAI/UnitAI.h` | 182 | `class UnitAI` — abstract base de TODA AI (Unit, no sólo Creature). `UpdateAI=0`, hook events, target selection helpers (`SelectTarget`, `SelectTargetList`, `DoSpellAttackIfReady`) |
| `src/server/game/AI/CoreAI/UnitAI.cpp` | 401 | Implementación de SelectTarget(s) con threat list, predicates, dist filters |
| `src/server/game/AI/CoreAI/UnitAICommon.h/cpp` | 117 / 186 | Predicates compartidos: NonTankTargetSelector, FarthestTargetSelector |
| `src/server/game/AI/CoreAI/CombatAI.h` | 107 | `class CombatAI : CreatureAI` — basic combat (autoattack + spells de creature_template_addon spell list) |
| `src/server/game/AI/CoreAI/CombatAI.cpp` | 309 | UpdateAI loop con event-based timer para casteos |
| `src/server/game/AI/CoreAI/PetAI.h` | 75 | `class PetAI : CreatureAI` — sigue al owner, ataca según owner aggro |
| `src/server/game/AI/CoreAI/PetAI.cpp` | 651 | UpdateAI con stay/follow/aggressive/defensive/passive modes, spells de pet bar, charm dispatch |
| `src/server/game/AI/CoreAI/TotemAI.h/cpp` | 39 / 87 | `class TotemAI : CreatureAI` — totem stationary, casts su spell periódicamente, despawn al expirar |
| `src/server/game/AI/CoreAI/GuardAI.h/cpp` | 37 / 69 | `class GuardAI : CombatAI` — Stormwind/Orgrimmar guards, defienden players amistosos, llamadas de refuerzos |
| `src/server/game/AI/CoreAI/PassiveAI.h` | 93 | `class PassiveAI : CreatureAI`, `class PossessedAI`, `class CritterAI`, `class TriggerAI`, `class NullCreatureAI` — todas variantes "no haces nada" |
| `src/server/game/AI/CoreAI/PassiveAI.cpp` | 119 | Trivial UpdateAI (no-op o solo charm-handling) |
| `src/server/game/AI/CoreAI/ReactorAI.h/cpp` | 33 / 32 | Mob que sólo ataca si attacked first (no aggro radius) |
| `src/server/game/AI/CoreAI/ScheduledChangeAI.h/cpp` | 40 / 22 | AI que se cambia a otra después de N ms (boss phase transitions) |
| `src/server/game/AI/CoreAI/AreaTriggerAI.h/cpp` | 79 / 28 | AI para AreaTrigger objects |
| `src/server/game/AI/CoreAI/GameObjectAI.h/cpp` | 134 / 42 | AI para GameObjects (chests, traps, switches) |
| `src/server/game/AI/CoreAI/enuminfo_UnitAICommon.cpp` | 73 | Reflection enum |
| `src/server/game/AI/ScriptedAI/ScriptedCreature.h` | 428 | `class ScriptedAI : CreatureAI` — base utility class para boss scripts custom (DoCastVictim, DoMeleeAttackIfReady, EventMap helpers) |
| `src/server/game/AI/ScriptedAI/ScriptedCreature.cpp` | 714 | EventMap impl, BossAI, SummonList, target predicates |
| `src/server/game/AI/ScriptedAI/ScriptedEscortAI.h/cpp` | 100 / 431 | `class EscortAI : ScriptedAI` — npcs que escoltan player en quests, walk waypoints, despawn al final |
| `src/server/game/AI/ScriptedAI/ScriptedFollowerAI.h/cpp` | 73 / 294 | `class FollowerAI : ScriptedAI` — sigue a player como guardian temporal en quests |
| `src/server/game/AI/ScriptedAI/ScriptedGossip.h/cpp` | 103 / 74 | Helpers gossip menu para scripted creatures |
| `src/server/game/AI/SmartScripts/SmartAI.h` | 354 | `class SmartAI : CreatureAI` — runtime de SmartScript; integra waypoints + escorting |
| `src/server/game/AI/SmartScripts/SmartAI.cpp` | 1259 | UpdateAI dispatch a SmartScript, path following, escort state, wp pause/resume |
| `src/server/game/AI/SmartScripts/SmartScript.h` | 152 | `class SmartScript` — interpreter; `mEvents: std::vector<SmartScriptHolder>`, `ProcessEvent`, `ProcessAction`, `OnUpdate` |
| `src/server/game/AI/SmartScripts/SmartScript.cpp` | 4253 | El **gigante interpreter**: switch con 158 SMART_ACTION_* + 238 SMART_EVENT_* (bigo de 4k líneas) |
| `src/server/game/AI/SmartScripts/SmartScriptMgr.h` | 1769 | Definiciones: `enum SMART_EVENT` (~80 base), `enum SMART_ACTION` (~150), `SmartScriptHolder`, `SmartTarget`, validation tables |
| `src/server/game/AI/SmartScripts/SmartScriptMgr.cpp` | 2497 | Singleton: carga `smart_scripts` table, valida cada fila, build cachés `mEventMap[entryOrGuid]` |
| `src/server/game/AI/PlayerAI/` | (no listado aquí) | AI para charm/possess de players — minoritario |

**Total AI/:** ~16,780 líneas (incluye headers + comentarios).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `UnitAI` | abstract class | Base de toda AI; abstract `UpdateAI(uint32 diff)`, hooks comunes (DamageDealt, DamageTaken, JustEnteredCombat, OnDespawn, OnCharmed) |
| `CreatureAI` | class : UnitAI | Base para AI de Creature; añade hooks creature-specific (JustDied, KilledUnit, MoveInLineOfSight, JustEngagedWith, JustReachedHome, SpellHit, SpellHitTarget, JustSummoned, IsSummonedBy, ReceiveEmote, EnterEvadeMode) |
| `NullCreatureAI` | class : CreatureAI | "no-op AI" — para creatures sin behavior |
| `PassiveAI` | class : CreatureAI | No reacciona, sólo evade |
| `PossessedAI` | class : CreatureAI | Cuando una criatura está poseída (charmed) |
| `CritterAI` | class : CreatureAI | Conejos, ratas — corren al recibir daño |
| `TriggerAI` | class : NullCreatureAI | NPCs invisibles para triggers (visual-only summons) |
| `CombatAI` | class : CreatureAI | Basic combat: melee + spell list rotation simple |
| `AggressorAI` | class : CombatAI | Auto-aggro standard |
| `PetAI` | class : CreatureAI | Pet behavior (follow, attack on owner aggro, react to commands) |
| `TotemAI` | class : CreatureAI | Totem fijo, cast cd-based, lifespan |
| `GuardAI` | class : CombatAI | NPC guard de ciudades, asistencia entre guards |
| `ReactorAI` | class : CombatAI | Reactivo (no aggro radius, sólo ataca si attacked) |
| `ScheduledChangeAI` | class : CreatureAI | Cambia a otro AI después de timer (boss phases) |
| `ScriptedAI` | class : CreatureAI | Base helpers para scripts custom (boss/quest); `EventMap`, `SummonList`, `DoCastVictim`, `DoCastSelf`, `DoMeleeAttackIfReady` |
| `BossAI` | class : ScriptedAI | Specialized para encounters (instance, achievement criteria) |
| `WorldBossAI` | class : ScriptedAI | World bosses (open-world raid mobs) |
| `EscortAI` | class : ScriptedAI | Escolta quests (waypoint follower) |
| `FollowerAI` | class : ScriptedAI | Quest follower (temporary companion) |
| `SmartAI` | class : CreatureAI | Runtime engine para data-driven SmartScript |
| `SmartScript` | class | Interprete; `std::vector<SmartScriptHolder>`, ProcessEvent / ProcessAction |
| `SmartScriptMgr` | singleton | Carga `smart_scripts` SQL → cache de scripts por entryOrGuid |
| `SmartScriptHolder` | struct | Una fila parseada de smart_scripts: entryOrGuid, eventType, eventParams[4], action, actionParams[7], target, targetParams[3], phase_mask, chance, flags, link |
| `SmartTarget` | struct | TargetType + 3 params (TARGET_NONE, TARGET_SELF, TARGET_VICTIM, TARGET_HOSTILE_LAST_AGGRO, TARGET_CLOSEST_PLAYER, …) |
| `EventMap` | class | Helper en ScriptedAI: schedule eventos con cooldown + group + phase; `ScheduleEvent`, `ExecuteEvent`, `Reset` |
| `SummonList` | class | Lista de summons activos del boss; `Summon`, `DespawnAll`, `DespawnEntry`, `IsAnyBossAlive` |
| `PetAI::SpellAttackEntry` | struct | Lista de spells de pet con cd y priority |
| `EvadeReason` | enum | Razones de EnterEvadeMode (Boundary, NoHostiles, Other, NoPath, …) |
| `SMART_EVENT` | enum (238 incluyendo phases/bits) | Event types de SmartScript: ~80 base (UPDATE_IC/OOC, HEALTH_PCT, MANA_PCT, AGGRO, KILL, DEATH, RANGE, SPELLHIT, RESPAWN, …) |
| `SMART_ACTION` | enum (158) | Action types: ~150 (TALK, CAST, EMOTE, MOVE_TO_POS, START_PATH, PHASE_CHANGE, SUMMON_CREATURE, ADD_AURA, …) |
| `SMART_TARGET` | enum (~30) | Target resolution types |
| `SMART_EVENT_PHASE` | enum (12) | Phase bitmask (bosses con N fases) |
| `SmartEventFlags` | bitmask | DEBUG_ONLY, NOT_REPEATABLE, DIFFICULTY_0..3, RESERVED, … |
| `SmartCastFlags` | bitmask | INTERRUPT_PREVIOUS, TRIGGERED, FORCE_CAST, NO_COMBAT_MOVE |
| `EscortState` | enum bitmask | ESCORT_STATE_NONE, ESCORT_STATE_ESCORTING, RETURNING, PAUSED |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `UnitAI::UpdateAI(uint32 diff)` | **Core tick** — abstract, subclase decide | (varies) |
| `UnitAI::SelectTarget(SelectTargetMethod, position, dist, playerOnly, withTank, aura)` | Encuentra target en threat list según criterio (closest/farthest/random) | `ThreatManager`, target predicates |
| `UnitAI::SelectTargetList(list, num, method, ...)` | N targets simultáneos | idem |
| `UnitAI::DoSpellAttackIfReady(spellId)` | Si cd ready y in range: cast | `Unit::CastSpell`, `SpellHistory::HasCooldown` |
| `UnitAI::AttackStart(Unit*)` | Comienza combat con target — set victim, start melee chase | `Unit::Attack`, `MotionMaster::MoveChase` |
| `CreatureAI::Reset()` | Reset state al evade/respawn (timers, phase, summons) | (override por boss) |
| `CreatureAI::JustEnteredCombat(Unit* who)` | Hook al entrar combate | (override) |
| `CreatureAI::JustEngagedWith(Unit* who)` | Hook al primer aggro real (no sólo "ser amenazado") | (override) |
| `CreatureAI::JustDied(Unit* killer)` | Hook al morir; drop loot, achievement, despawn summons | (override) |
| `CreatureAI::KilledUnit(Unit* victim)` | Hook al matar a algo (jugador, otra creature) | (override) |
| `CreatureAI::MoveInLineOfSight(Unit* who)` | Player/Unit entró en visibility range; check aggro | `IsHostileTo`, `Attack`, ai_factory aggro radius |
| `CreatureAI::TriggerAlert(Unit const* who)` | Pre-aggro alert for stealthed/prowling player | `SendAIReaction(AI_REACTION_ALERT)` + `MotionMaster::MoveDistract(5000ms, angle)` |
| `CreatureAI::EnterEvadeMode(EvadeReason)` | Salir de combat (out of bounds, no hostiles, leashing) | `Reset`, `Creature::SetWalk`, `MotionMaster::MoveTargetedHome`, `SetHealth(maxHealth)` |
| `CreatureAI::SpellHit(WorldObject* caster, SpellInfo const*)` | Hook al ser blanco de spell | (override) |
| `CreatureAI::SpellHitTarget(WorldObject* target, SpellInfo const*)` | Hook al castear y impactar | (override) |
| `CreatureAI::JustSummoned(Creature* summon)` | Hook cuando spawneas algo | guarda en SummonList |
| `CreatureAI::SummonedCreatureDies(Creature*, Unit*)` | Hook cuando summon muere | (override) |
| `CreatureAI::JustReachedHome()` | Hook al volver al spawn post-evade | (override) |
| `CreatureAI::ReceiveEmote(Player*, uint32 emoteId)` | Player hizo /wave a la creature | (override) |
| `CreatureAI::DoZoneInCombat(Creature* creature)` | En instancia, mete en combate a players vivos del mapa, pets controladas y vehículo base | `Map::IsDungeon`, `Map::GetPlayers`, `CombatManager::CanBeginCombat`, `Creature::EngageWithTarget` |
| `CreatureAI::MovementInform(uint32 type, uint32 id)` | Hook cuando termina un movimiento (wp, point, charge) | (override) |
| `PetAI::UpdateAI(diff)` | Pet tick: handle command (stay/follow/attack), spell rotation | `MotionMaster::MoveFollow`, `DoSpellAttackIfReady` |
| `TotemAI::UpdateAI(diff)` | Totem cast cd | `DoSpellAttackIfReady` |
| `CombatAI::InitSpellList()` | Carga spells del template_addon | `sObjectMgr->GetCreatureTemplateAddon` |
| `CombatAI::UpdateAI(diff)` | Spell rotation con `events: EventMap` | `events.Update`, `ExecuteEvent`, `DoCastVictim` |
| `ScriptedAI::DoCastVictim(spellId, triggered)` | Cast a victim actual con flag triggered | `Unit::CastSpell` |
| `ScriptedAI::DoCastSelf(spellId, triggered)` | Cast self | `Unit::CastSpell` |
| `ScriptedAI::DoMeleeAttackIfReady()` | Auto-attack tick | `Unit::AttackerStateUpdate` |
| `EventMap::ScheduleEvent(eventId, time, group=0, phase=0)` | Programa un evento futuro | `_eventMap.insert` |
| `EventMap::ExecuteEvent()` | Pop el siguiente evento ready | retorna eventId |
| `EventMap::SetPhase(uint8)` | Cambia fase del boss; eventos con phase mask filtrados | `_phase` |
| `SmartAI::UpdateAI(diff)` | Llama `mScript.OnUpdate(diff)`; maneja waypoints/escort | `SmartScript::OnUpdate`, `WaypointMovementGenerator` |
| `SmartAI::StartPath(pathId, repeat, invoker, nodeId)` | Comienza waypoint path | `SmartWaypointMgr::GetPath` |
| `SmartAI::PausePath(delay, forced)` | Pausa el wp (talk, action) | timer |
| `SmartAI::EndPath(fail)` | Termina path (despawn / quest credit) | (override) |
| `SmartScript::OnUpdate(uint32 diff, ...)` | Itera mEvents; check si event ready, dispatch ProcessEvent | `ProcessEvent` |
| `SmartScript::ProcessEvent(SmartScriptHolder& e, Unit* unit, uint32 var0, uint32 var1, bool bvar, SpellInfo const* spell, GameObject* gob)` | Switch sobre 80 SMART_EVENT_*: chequea condición | `ProcessAction` if matched |
| `SmartScript::ProcessAction(SmartScriptHolder& e, Unit* unit, ...)` | Switch sobre 150+ SMART_ACTION_* | depende del action: Talk, CastSpell, AddAura, MoveToPos, SummonCreature, RemoveAuras, CallForHelp, ... |
| `SmartScript::GetTargets(SmartScriptHolder const& e, WorldObject* invoker)` | Resuelve SmartTarget → Vec<WorldObject*> | depende de target type |
| `SmartScriptMgr::LoadSmartAIFromDB(SmartScriptType type)` | Carga `smart_scripts` filtered por type | `WorldDatabase.Query` |
| `SmartScriptMgr::IsEventValid(SmartScriptHolder& e)` | Validación columnas (ranges, dependencies) — descarta filas malas | many |
| `CreatureAISelector::selectAI(Creature*)` | Decide AI a instanciar al spawn | check Pet/Totem/Vehicle, then SmartScripts table, then template AIName, fallback NullCreatureAI |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Creature` — `Creature*` es el "owner" de toda CreatureAI; AI accesa `me->GetMotionMaster()`, `me->GetThreatManager()`, `me->CastSpell`, `me->GetVictim`
- `Entities/Unit` — `UnitAI` base; threat list, victim, faction, hostility checks
- `Entities/Player` — para gossip, escort companion checks, charm
- `Spells` — UnitAI llama `Unit::CastSpell`, escucha `SpellHit`, `OnSpellCast`, `OnChannelFinished`
- `Combat` — `ThreatManager`, `DamageInfo`, `HealInfo`, `DamageTaken/Dealt` hooks
- `Movement` — `MotionMaster::MoveChase`, `MovementGenerator`, waypoint paths (`WaypointMovementGenerator`), `MoveFollow`, `MoveTargetedHome`, `MovePoint`, `MoveJump`
- `Maps` — `Map::VisitNearbyCellsOf` (zone-in-combat search), `Cell::VisitGrid` para pet-search
- `Database` — `smart_scripts` (huge), `creature_text`, `waypoint_data`, `waypoint_path`, `creature_template_addon`, `creature_addon`, `gameobject_template_addon` (vía SmartScriptMgr load), `creature_summon_groups`
- `Conditions` — SmartScript actions/events pueden tener Conditions adjuntas
- `ScriptMgr` — registro de C++ scripts (boss scripts implementados con `RegisterCreatureAI(BossAI)`)
- `Achievements` — KilledUnit triggers achievement criteria
- `Gossip` — ScriptedGossip helpers
- `Quests` — EscortAI/FollowerAI completion
- `Pets` — PetAI usa Pet stats, owner aggro, talents
- `OutdoorPvP` / `Battleground` — flag carriers, NPC bosses BG
- `Loot` — JustDied dispara loot generation

**Depended on by:**
- `Creature` — cada Creature tiene `CreatureAI* i_AI` member; `AIM_Initialize` lo crea
- `World/MapUpdater` — Map::Update llama Creature::Update, que llama `i_AI->UpdateAI(diff)`
- `Combat/Damage` — pasa por `i_AI->DamageTaken/Dealt` antes de aplicar
- `Spells` — al impactar, dispatch a `i_AI->SpellHit/SpellHitTarget`
- `Movement` — al terminar movimiento, dispatch a `i_AI->MovementInform`
- `Scripting` — boss scripts `extern "C"` registran su AI factory aquí
- `Pet system` — Pet::Initialize crea PetAI

---

## 6. SQL / DB queries (if any)

Los AIs son **muy** dependientes de SQL — la mayoría del comportamiento está en data, no en código.

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entryorguid, source_type, id, link, event_type, event_phase_mask, event_chance, event_flags, event_param1..5, action_type, action_param1..7, target_type, target_param1..3, target_x, target_y, target_z, target_o, comment FROM smart_scripts` | **La gran tabla** — carga de SmartScript. ~50k filas. Cargada toda al startup en `mEventMap[entryOrGuid]` | world |
| `SELECT entry, groupid, id, text, type, language, probability, emote, duration, sound, broadcast_text_id, comment FROM creature_text` | Líneas de talk de SmartAI ACTION_TALK + ScriptedAI::Talk | world |
| `SELECT id, point, position_x, position_y, position_z, orientation, delay, action, action_chance, wpguid FROM waypoint_data` | Waypoints para SmartAI escort/path | world |
| `SELECT entry, pathid, point, position_x, position_y, position_z, point_comment FROM waypoints` | Waypoints clásicos (legacy) | world |
| `SELECT id, MoveType, Behavior FROM script_waypoint` | Old script waypoints (deprecated, mover) | world |
| `SELECT entry, MoveType, Behavior FROM creature_addon` | Per-spawn config (mount, auras, stand state, emote on spawn) | world |
| `SELECT entry, path_id, mount_creature_id, bytes1, bytes2, emote, visibility_distance_type, auras FROM creature_template_addon` | Per-template config; CombatAI lee la lista de auras/spell list de aquí | world |
| `SELECT * FROM creature_text_locale` | Localización de creature_text | world |
| `SELECT entry, ai_name FROM creature_template` (col `AIName`) | AIName decide AI a usar (CombatAI, AggressorAI, GuardAI, NullCreatureAI, SmartAI…) | world |
| `SELECT entry, ScriptName FROM creature_template` (col `ScriptName`) | Scripted AI name (boss script registrado en C++) | world |
| `SELECT id, summonerId, summonerType, groupId, entry, position_x, position_y, position_z, orientation, summonType, summonTime FROM creature_summon_groups` | Summons por grupo (dragonkin packs, etc.) | world |
| (legacy) `creature_ai_template`, `creature_ai_scripts`, `creature_ai_summons` | EventAI antiguo (deprecated, sustituido por SmartAI). Aún cargado por compatibility en algunos forks | world |

**No DB2 stores específicos del módulo AI** — los stores que afectan AI (CreatureModelInfo, etc.) son de Entities/Creature.

---

## 7. Wire-protocol packets (if any)

El módulo AI **no origina opcodes propios**. Sus efectos llegan al cliente vía:

| Opcode | Direction | Sent by AI via |
|---|---|---|
| `SMSG_EMOTE` | server → client | `Unit::HandleEmoteCommand` (SMART_ACTION_PLAY_EMOTE, ScriptedAI::Talk emote arg) |
| `SMSG_TEXT_EMOTE` | server → client | `/say` etc. (SMART_ACTION_TALK with TEXTEMOTE type) |
| `SMSG_CHAT` (SMSG_MESSAGECHAT) | server → client | NPC says/yells (SMART_ACTION_TALK, ScriptedAI::Talk) — chat type CHAT_MSG_MONSTER_SAY/YELL/EMOTE/WHISPER |
| `SMSG_ON_MONSTER_MOVE` | server → client | Cualquier `MotionMaster::Move*` que arranque AI llama (chase, point, jump, target) |
| `SMSG_PLAY_SOUND` / `SMSG_PLAY_MUSIC` | server → client | SMART_ACTION_PLAY_SOUND, creature_text sound col |
| `SMSG_PLAY_SPELL_VISUAL_KIT` | server → client | TriggerAlert (visual "?" sobre cabeza), SMART_ACTION_PLAY_SPELL_VISUAL_KIT |
| `SMSG_AI_REACTION` | server → client | JustEnteredCombat → cliente muestra red name; AI_REACTION_HOSTILE |
| `SMSG_PARTY_KILL_LOG` (raid kill) | server → client | KilledUnit broadcast a party |
| `SMSG_AURA_UPDATE` | server → client | SMART_ACTION_ADD_AURA, ScriptedAI::DoCast → vía Aura::_ApplyForTarget |
| `SMSG_SPELL_*` (Start/Go/Failed) | server → client | DoCast / DoSpellAttackIfReady → vía Spell pipeline |
| `SMSG_THREAT_UPDATE` / `SMSG_HIGHEST_THREAT_UPDATE` | server → client | Cualquier AddThreat → ThreatManager broadcast |
| `SMSG_ENABLE_BARBER_SHOP` etc. | (no aplica) | — |
| `CMSG_GOSSIP_HELLO` / `CMSG_GOSSIP_SELECT_OPTION` | client → server | ReceiveGossipHello/Select hooks en CreatureAI subclasses |
| `CMSG_PET_ACTION` / `CMSG_PET_CAST_SPELL` | client → server | PetAI command dispatch (stay/follow/attack/cast) |

Resumen: AI consume hooks del engine, no ve sockets directamente. Los efectos (movimiento, casts, talks) ya se opcodifican en otros módulos (Movement, Spells, Chat, GameObject).

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-ai/src/lib.rs` — ~1259 líneas — **un único struct `CreatureAI`** plain (no polimórfico, no trait), con states `Idle/WalkingRandom/WalkingWaypoint/InCombat/Dead/Returning`, selector representado, `SummonListLikeCpp` y tests unitarios
- `crates/wow-script/src/lib.rs` — **0 líneas (vacío)** — crate placeholder
- `crates/wow-scripts/src/lib.rs` — **0 líneas (vacío)** — crate placeholder
- `crates/wow-world/src/session.rs` — contiene legacy `WorldCreature` (ahora siendo refactorizado en MapManager work) que duplica funciones de CreatureAI
- `crates/wow-world/src/handlers/misc.rs` — handler de packets diversos (puede contener emote dispatch que tocaría AI)

**What's implemented:**
- `enum CreatureState { Idle, WalkingRandom, WalkingWaypoint, InCombat, Dead, Returning }`
- `struct CreatureAI` con `guid, entry, home_pos, current_pos, move_target, move_start, move_duration_ms, spline_id, state, wander_timer, wander_delay_ms, hp, max_hp, level, min_dmg, max_dmg, combat_target, last_swing, swing_timer_ms, aggro_radius, wander_radius, is_alive, death_time, respawn_time_secs, corpse_despawn_at, npc_flags, unit_flags, display_id, faction`
- Métodos: `new`, `can_wander`, `try_aggro` (chequea distancia, entra combat), `enter_combat`, `reset_combat` (full HP regen, vuelve a home), `take_damage`, `die`, `should_respawn`, `respawn`, `movement_finished`, `interpolated_position`, `begin_move`, `finish_move`, `can_swing`, `record_swing`, `roll_damage`, `should_wander`, `pick_wander_destination`, `reset_wander_timer`
- Random pseudo-determinístico (subsec_nanos hash) — no es real RNG
- Wandering aleatorio dentro de `wander_radius`
- Aggro radius check + enter combat
- Damage + death + respawn timer (30s default)

**What's missing vs C++:**
1. **NO trait/polimorfismo:** todo es un único struct concreto. C++ tiene jerarquía 8+ AI types — necesita `trait CreatureAI` con métodos virtuales
2. **NO SmartAI:** la pieza más grande (95% del game content) — sin parser, sin runtime, sin loader de `smart_scripts`
3. **NO ScriptedAI runtime:** existe base común `EventMap` y `SummonListLikeCpp`, pero aún no están integrados en `ScriptedAI`/`BossAI` runtime real ni existen helpers `DoCastVictim`/`DoMeleeAttackIfReady`
4. **NO PetAI:** pets tendrán que ser comportamiento custom; sin command dispatch (stay/follow/attack/passive/aggressive/defensive), sin pet bar spell rotation
5. **NO CombatAI:** sin lectura de spell list de `creature_template_addon`, sin spell rotation simple
6. **NO TotemAI:** sin lifespan, sin cast cd
7. **NO GuardAI:** sin asistencia entre guards
8. **NO PassiveAI / NullCreatureAI / TriggerAI** subclases
9. **NO ReactorAI:** la creature actual tiene aggro radius siempre — no hay modo "sólo ataca si attacked"
10. **NO ScheduledChangeAI:** sin transiciones de fase boss
11. **NO EscortAI / FollowerAI:** sin quest companions
12. **NO ThreatManager integration:** combat target es simple Option<ObjectGuid>; sin threat list multi-target
13. **NO EnterEvadeMode runtime:** existe `EvadeReasonLikeCpp` y plan representado de side effects; `reset_combat` sigue siendo trivial (full HP, return home), sin aplicar MotionMaster/auras/threat/leashing por boundary en runtime
14. **NO MoveInLineOfSight con TriggerAlert runtime:** existe plan representado de `TriggerAlert`, pero falta cablearlo desde notifiers/visibility y aplicar `SendAIReaction` + `MoveDistract` reales antes del aggro
15. **NO hooks events:** `JustEnteredCombat`, `KilledUnit`, `SpellHit`, `SpellHitTarget`, `JustDied`, `JustSummoned`, `IsSummonedBy`, `MovementInform`, `JustReachedHome`, `OnHealthDepleted`, `ReceiveEmote`, `OnGameEvent` — todos ausentes
16. **NO factory + selector:** sin `CreatureAISelector::selectAI` que decide qué AI instanciar al spawn
17. **NO registry de AI names:** no se puede mappear `creature_template.AIName='SmartAI'` → instanciar SmartAI
18. **NO ScriptName binding:** boss scripts en `wow-scripts` no se enganchan
19. **NO target selection runtime:** existe selección representada `SelectTargetMethodLikeCpp`/`DefaultTargetSelectorLikeCpp`, pero falta conectarla al `ThreatManager` real y a `UnitAI` polimórfico
20. **NO DoZoneInCombat runtime:** existe plan representado de `CreatureAI::DoZoneInCombat`, pero falta cablearlo al mapa real, `CombatManager::CanBeginCombat` y `Creature::EngageWithTarget`
21. **NO call for help / call assistance:** sin guard chain, sin pack pulls
22. **NO summons cascade runtime:** `SummonListLikeCpp::despawn_all_like_cpp` existe como plan representado; al morir un boss todavía falta wiring real `JustDied -> DespawnAll -> DespawnOrUnsummon`
23. **NO SmartScriptMgr loader:** sin parse de `smart_scripts` SQL
24. **NO creature_text loader:** sin tabla de talks
25. **NO waypoint loader:** sin paths para escort/patrol
26. **NO event_phase_mask:** sin bosses de N fases con eventos filtrados por phase
27. **NO event chance/flags:** sin probabilidad random, sin DEBUG_ONLY/NOT_REPEATABLE/DIFFICULTY flags
28. **Random determinístico flawed:** `pick_wander_destination` usa `subsec_nanos` que es muy poco aleatorio entre llamadas seguidas (puede degenerar en mismo punto)
29. **NO charm/possess support**

**Suspicious / likely divergent:**
- `try_aggro` chequea distancia plana — C++ chequea `IsHostileTo(player) && CanSeeOrDetect(player) && IsValidAttackTarget(player)` antes de aggro; la versión actual entra en combat con cualquier player en rango sin importar faction o stealth
- `reset_combat` cura full HP instantáneo y vuelve home — C++ separa: `EnterEvadeMode` → set state → `MoveTargetedHome` → callback `JustReachedHome` → reset auras + HP. RustyCore salta los hooks intermedios
- `swing_timer_ms` hardcoded 2000ms — C++ usa `BASE_ATTACK` weapon_speed del template; se desincroniza para mobs con weapon delay diferente (1500/2500/3000)
- `aggro_radius` único — C++ ajusta por level diff (mob 5 lvl arriba aggrea desde más lejos; mob 5 lvl abajo no aggrea aunque te le acerques)
- Damage roll vía `subsec_nanos` no respeta `attack_power`, `crit_chance`, `armor` reduction, `level diff` damage modifier
- `respawn_time_secs = 30` default es WAY too low para mobs reales (la mayoría 3-7 min); el data debería leer de spawn data

**Tests existing:**
- 0 tests en `crates/wow-ai/`
- 0 tests en `crates/wow-script/`
- 0 tests en `crates/wow-scripts/`

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#AI.WBS.001** Cerrar la migracion auditada de `game/AI/AIException.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/AIException.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.002** Cerrar la migracion auditada de `game/AI/CreatureAI.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.003** Cerrar la migracion auditada de `game/AI/CreatureAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.004** Cerrar la migracion auditada de `game/AI/CreatureAIFactory.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAIFactory.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.005** Cerrar la migracion auditada de `game/AI/CreatureAIImpl.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAIImpl.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.006** Cerrar la migracion auditada de `game/AI/CreatureAIRegistry.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAIRegistry.cpp`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.007** Cerrar la migracion auditada de `game/AI/CreatureAIRegistry.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAIRegistry.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.008** Cerrar la migracion auditada de `game/AI/CreatureAISelector.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.009** Cerrar la migracion auditada de `game/AI/CreatureAISelector.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.010** Cerrar la migracion auditada de `game/AI/GameObjectAIFactory.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/GameObjectAIFactory.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AI.WBS.011** Cerrar la migracion auditada de `game/AI/SelectableAI.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/SelectableAI.h`
  Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numerados para referencia desde `MIGRATION_ROADMAP.md`. Complejidad: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [ ] **#AI.1** Refactor `CreatureAI` actual a `trait CreatureAI` (en `crates/wow-ai/src/creature_ai.rs`) con métodos virtuales: `update_ai(&mut self, diff_ms: u32)`, `reset(&mut self)`, `just_entered_combat(&mut self, who: &Unit)`, `just_died(&mut self, killer: &Unit)`, `killed_unit(&mut self, victim: &Unit)`, `move_in_line_of_sight(&mut self, who: &Unit)`, `enter_evade_mode(&mut self, reason: EvadeReason)`, `spell_hit(&mut self, caster: &Unit, info: &SpellInfo)`, `just_summoned(&mut self, summon: &Creature)`, `movement_inform(&mut self, ty: u32, id: u32)`, `receive_emote(&mut self, p: &Player, emote_id: u32)` (H)
- [x] **#AI.2** Implementar `EvadeReasonLikeCpp` contrastado contra `CoreAI/UnitAICommon.h` + `enuminfo_UnitAICommon.cpp`: variantes, orden `EnumUtils::ToIndex`, constantes y descripciones. (L)
- [ ] **#AI.3** Mover el struct actual `CreatureAI` a `struct DefaultCreatureAI` (impl trait) — backward compat (M)
- [ ] **#AI.4** Implementar `struct NullCreatureAI` impl `CreatureAI` con todos los métodos no-op (L)
- [ ] **#AI.5** Implementar `struct PassiveAI`, `struct CritterAI`, `struct TriggerAI`, `struct PossessedAI` (M — son mostly stubs)
- [ ] **#AI.6** Implementar `struct CombatAI` que lee spell list de `creature_template_addon` y rota castings simples — depende de `EventMap` (H)
- [x] **#AI.7a** Implementar base común `wow_core::EventMap` contrastada contra `Utilities/EventMap.{h,cpp}`: schedule/update/execute, phase/group bits, repeat, delay, cancel y series. (M)
- [ ] **#AI.7b** Integrar `wow_core::EventMap` en `ScriptedAI`/`CombatAI` runtime real (`events.Update(diff)`, `ExecuteEvent`, casts/side effects). (H)
- [x] **#AI.8a** Implementar base representada `SummonListLikeCpp` contrastada contra `ScriptedAI/ScriptedCreature.{h,cpp}`: almacenamiento GUID list-like con orden/duplicados, `Summon`, `Despawn`, `DespawnAll`, `DespawnEntry`, `RemoveNotExisting`, `HasEntry`, `DoZoneInCombat` y `DoAction(max)` con `RandomResize` como planes de side effect resueltos por vista externa tipo `ObjectAccessor`. (M)
- [ ] **#AI.8b** Integrar `SummonListLikeCpp` en `ScriptedAI`/`BossAI` runtime real: `JustSummoned`, `SummonedCreatureDespawn`, `JustDied -> DespawnAll`, aplicación real de `DespawnOrUnsummon`, `AI()->DoZoneInCombat` y `AI()->DoAction`. (H)
- [ ] **#AI.9** Implementar `struct PetAI` impl CreatureAI: estados `Stay/Follow/Attack/Passive/Aggressive/Defensive`, command dispatch, owner aggro inheritance, spell rotation pet bar (XL)
- [ ] **#AI.10** Implementar `struct TotemAI` impl CreatureAI: lifespan, cd-based cast (M)
- [ ] **#AI.11** Implementar `struct GuardAI` impl CreatureAI (extends CombatAI): asistencia entre guards en mismo zone (M)
- [ ] **#AI.12** Implementar `struct ReactorAI` impl CreatureAI: NO aggro radius, sólo ataca si fue atacado (L)
- [ ] **#AI.13** Implementar `struct ScheduledChangeAI` para boss phase transitions (M)
- [ ] **#AI.14** Implementar `struct ScriptedAI` (base helpers) con utilidades `do_cast_victim(spell_id, triggered)`, `do_cast_self(spell_id)`, `do_melee_attack_if_ready()`, `events: EventMap`, `summons: SummonList` (H)
- [ ] **#AI.15** Implementar `struct EscortAI` impl ScriptedAI: waypoint follow + invoker tracking (H)
- [ ] **#AI.16** Implementar `struct FollowerAI` impl ScriptedAI: temporal companion para quests (M)
- [ ] **#AI.17** Definir `enum SmartEvent` con las ~80 base variants (UpdateIc, UpdateOoc, HealthPct, ManaPct, Aggro, Kill, Death, Respawn, Range, SpellHit, OocLos, FriendlyHealth, …) en `crates/wow-script/src/smart_event.rs` (H)
- [ ] **#AI.18** Definir `enum SmartAction` con las ~150 variants (Talk, CastSpell, Emote, MoveToPos, StartPath, PhaseChange, SummonCreature, AddAura, RemoveAura, KillSelf, Despawn, SetReactState, SetActive, SetEventPhase, …) (XL)
- [ ] **#AI.19** Definir `enum SmartTarget` (~30 variants: None, Self, Victim, HostileLastAggro, ClosestPlayer, ClosestCreature, ClosestGameObject, CreatureRange, GameObjectRange, ActionInvoker, OwnerOrSummoner, …) (M)
- [ ] **#AI.20** Implementar `struct SmartScriptHolder { entry_or_guid: i64, source_type: SmartScriptType, id: u32, link: u32, event: SmartEvent, event_phase_mask: u32, event_chance: u8, event_flags: u32, action: SmartAction, target: SmartTarget, target_xyzo: Position, comment: String }` (M)
- [ ] **#AI.21** Implementar `SmartScriptMgr` singleton: load `smart_scripts` SQL → `mEventMap: HashMap<i64 entryOrGuid, Vec<SmartScriptHolder>>` con validación columna por columna (H)
- [ ] **#AI.22** Implementar `struct SmartScript` runtime: `events: Vec<SmartScriptHolder>`, `current_phase: u8`, `on_update(diff)`, `process_event(holder, unit, ...)` (XL — depende de #AI.18)
- [ ] **#AI.23** Implementar `process_action` switch para los 30 SMART_ACTION_* más usados (Talk, CastSpell, AddAura, MoveToPos, Emote, PlaySound, SetReactState, ForceCombatStop, KillSelf, Despawn, RemoveAura, SetEventPhase, SummonCreature, SetVisibility, MountToCreature, SetEquipment, SetFly, SetRun) (H)
- [ ] **#AI.24** Implementar resto de los ~120 SMART_ACTION_* en lotes (XL — splittable)
- [ ] **#AI.25** Implementar `process_event` switch para los ~80 SMART_EVENT_* (UpdateIc/Ooc, HealthPct, ManaPct, Aggro, Kill, Death, JustSummoned, SpellHit, Range, OocLos, …) (H)
- [ ] **#AI.26** Implementar `get_targets(holder, invoker) -> Vec<Arc<Unit>>` resolviendo SmartTarget → lista concreta (H)
- [ ] **#AI.27** Implementar `struct SmartAI` impl CreatureAI: wraps `SmartScript`, dispatch UpdateAI → script.on_update, all hooks → script.process_event con tipo correcto (H)
- [ ] **#AI.28** Implementar SmartAI waypoint integration: `start_path`, `pause_path`, `resume_path`, `end_path`, `waypoint_reached`, `waypoint_path_ended` (M)
- [ ] **#AI.29** Implementar SmartAI escort flow: invoker tracking, distance check, despawn on quest abandon (M)
- [ ] **#AI.30** Implementar `creature_text` loader → `CreatureTextMgr` con `send_chat(creature, group_id, target?)`; engancha con SMART_ACTION_TALK (M)
- [ ] **#AI.31** Implementar `waypoint_data` / `waypoint_path` loader → `WaypointMgr` con `get_path(path_id) -> &Path` (M)
- [ ] **#AI.32** Implementar `CreatureAISelector::select_ai(creature: &Creature) -> Box<dyn CreatureAI>`: chequea Pet/Totem/Vehicle, busca smart_scripts entry, busca template AIName, fallback NullCreatureAI (M)
- [ ] **#AI.33** Implementar registry `inventory::submit!` para AI factories: `CreatureAIFactory` con name → fn(creature) -> Box<dyn CreatureAI> (M)
- [ ] **#AI.34** Implementar `ScriptName` binding: registrar boss scripts en `crates/wow-scripts/` con `register_creature_ai!("boss_balnazzar", BalnazzarAI)` (M)
- [ ] **#AI.35** Implementar `ThreatManager` proper en `crates/wow-combat/`: `Vec<ThreatRef> { unit_guid, threat_value }` ordenado, `add_threat`, `remove_threat`, `get_top_threat`, `get_threat_list` (H — pertenece a Combat pero AI lo necesita)
- [x] **#AI.36a** Implementar base representada de `UnitAI::SelectTargetList` contrastada contra `CoreAI/UnitAI.{h,cpp}` y `UnitAICommon.{h,cpp}`: `SelectTargetMethod::{Random,MaxThreat,MinThreat,MaxDistance,MinDistance}`, orden current-victim/sorted-threat, orden distancia, offset, `DefaultTargetSelector` (`dist`, `playerOnly`, `withTank`, `aura`) y resize final. (M)
- [ ] **#AI.36b** Wire runtime real de `UnitAI::SelectTarget`/`SelectTargetList`: leer `ThreatManager`, resolver current/last victim, `IsWithinCombatRange`, `HasAura`, player type y selección random real desde el runtime polimórfico. (H)
- [ ] **#AI.37** Implementar hooks completos: `JustEnteredCombat`, `JustEngagedWith`, `KilledUnit`, `SpellHit`, `SpellHitTarget`, `JustSummoned`, `IsSummonedBy`, `MovementInform`, `JustReachedHome`, `OnHealthDepleted`, `ReceiveEmote`, `OnGameEvent`, `JustDied`, `Reset` — wire desde Creature/Spell/Combat/Movement módulos (H)
- [x] **#AI.38a** Implementar plan representado de `CreatureAI::EnterEvadeMode` contrastado contra `CreatureAI.cpp`: early-return si ya evade, rama muerto sólo `EngagementOver`, limpieza de auras/combat/tap/damage/cooldowns/target, movimiento follow-owner vs targeted-home y `Reset` como plan de side effects. (M)
- [ ] **#AI.38b** Wire runtime real de `EnterEvadeMode`: aplicar auras/threat/spell-history/tap, `MotionMaster::MoveTargetedHome`/`MoveFollow`, `UNIT_STATE_EVADE`, callback `JustReachedHome`, HP/auras/summons completos y leash/boundary checks. (H)
- [x] **#AI.39a** Implementar plan representado de `CreatureAI::TriggerAlert` contrastado contra `CreatureAI.cpp`: target player existente, criatura unit/no engaged/no confused-stunned-fleeing-distracted, no civil/passive, hostile y target acceptable; side effects `AI_REACTION_ALERT` + `MoveDistract(5000ms, angle)`. (L)
- [ ] **#AI.39b** Wire runtime real de `TriggerAlert`: llamada desde notifiers/visibility, `SendAIReaction(AI_REACTION_ALERT)`, `MotionMaster::MoveDistract`, orientation real y orden frente a `MoveInLineOfSight`/aggro. (M)
- [x] **#AI.40a** Implementar plan representado de `CreatureAI::DoZoneInCombat` contrastado contra `CreatureAI.cpp`: solo dungeon, no-op sin players, filtra `Player::IsAlive` + `CombatManager::CanBeginCombat`, y planifica `EngageWithTarget` para player, unidades controladas y vehículo base en orden. (M)
- [ ] **#AI.40b** Wire runtime real de `CreatureAI::DoZoneInCombat`: resolver `Map::IsDungeon`/`HavePlayers`/`GetPlayers`, `CombatManager::CanBeginCombat`, aplicar `Creature::EngageWithTarget` real y preservar logging de error en mapas no dungeon. (H)
- [ ] **#AI.41** Refactor random — usar `rand` crate proper (`StdRng` semilla por mapa, no time-based) (L)
- [x] **#AI.42a** Implementar helper representado de `Creature::GetAttackDistance` contrastado contra `Creature.cpp`: `Rate.Creature.Aggro`, base `20 - CombatReach`, level-diff, auras detect/detected range con gate de nivel, cap por max level de expansión y clamps C++ min/max. (L)
- [x] **#AI.42b** Wire runtime básico del aggro radius dinámico: sustituir el radio fijo legacy por `Creature::GetAttackDistance(target) + m_CombatDistance` en el scan global `run_legacy_creature_aggro_tick_once_like_cpp` y en el path legacy `check_creature_aggro`, preservando `aggro_radius <= 0` como desactivador. (M)
- [ ] **#AI.42c** Completar fuentes reales de `GetAttackDistance`: leer `Rate.Creature.Aggro`, `CONFIG_MAX_PLAYER_LEVEL`, max level de expansión del template y aura modifiers `SPELL_AURA_MOD_DETECT_RANGE` / `SPELL_AURA_MOD_DETECTED_RANGE` desde runtime real en todos los callers. (M)
- [ ] **#AI.43** Implementar `creature_template_addon.auras` aplicación al spawn (M)
- [ ] **#AI.44** Implementar charm/possess flow + PossessedAI swap (H)
- [ ] **#AI.45** Tests de regresión unit + smoke tests de SmartScript (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#AI.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 11 files / 1324 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp`. Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`. | `cargo test -p wow-ai && cargo test -p wow-script && cargo test -p wow-scripts` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#AI.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 11 files / 1324 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp`. Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#AI.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 11 files / 1324 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp`. Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#AI.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 11 files / 1324 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp`. Rust target: `crates/wow-ai`, `crates/wow-script`, `crates/wow-scripts`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `CreatureAI::try_aggro` requiere hostility check (un mob friendly no entra combat con player)
- [x] Test: `Creature::GetAttackDistance` representado respeta level-diff, combat reach, clamps, rate, auras detect range y max level de expansión
- [x] Test: runtime global de aggro usa `Creature::GetAttackDistance(target) + m_CombatDistance` en vez de radio fijo
- [ ] Test: `GetAttackDistance` runtime consume rate/config/expansion/aura modifiers reales, no defaults representados
- [ ] Test: `CreatureAI::take_damage` con dmg ≥ hp dispara `JustDied(killer)` exactamente 1 vez
- [ ] Test: `EnterEvadeMode(EvadeReason::Boundary)` programa `MoveTargetedHome`, al llegar dispara `JustReachedHome`, restaura HP a max
- [ ] Test: `EnterEvadeMode(EvadeReason::NoHostiles)` despawnea summons del boss
- [ ] Test: `MoveInLineOfSight` con player en aggro radius dispara `TriggerAlert` antes de `JustEngagedWith` (delay ~5s)
- [ ] Test: `CreatureAI::DoZoneInCombat` en dungeon engancha players vivos, pets controladas y vehículo base; fuera de dungeon solo loguea error
- [ ] Test: `ThreatManager.get_top_threat()` retorna el unit con threat más alta tras varios `add_threat`
- [x] Test: `EventMap.schedule_event(1, 5000ms)` → `update(5000)` → `execute_event()` retorna el evento.
- [x] Test: `EventMap.set_phase(2)` filtra eventos con phase_mask sin bit 2.
- [x] Test: `SummonList.despawn_all()` drena FIFO, ignora GUIDs inexistentes y devuelve el plan de despawn para los summons existentes.
- [ ] Test: `SmartScriptMgr.load_smart_ai_from_db(SmartScriptType::Creature)` parsea N filas válidas, descarta filas con event_type fuera de rango
- [ ] Test: `SmartScript.process_event(SMART_EVENT_AGGRO)` dispara matching action en `JustEnteredCombat`
- [ ] Test: `SmartScript.process_event(SMART_EVENT_HEALTH_PCT, 30, 50)` dispara cuando hp% entra a [30,50]
- [ ] Test: `SmartScript.process_action(SMART_ACTION_TALK, group_id=0)` envía SMSG_CHAT con texto de `creature_text`
- [ ] Test: `SmartScript.process_action(SMART_ACTION_CAST, spell_id=12345)` llama `Unit::CastSpell` con flags correctas
- [ ] Test: `SmartScript.process_action(SMART_ACTION_ADD_AURA)` añade aura al target resuelto
- [ ] Test: `SmartScript.process_action(SMART_ACTION_SUMMON_CREATURE)` spawna creature en target_xyzo
- [ ] Test: `SmartScript.get_targets(SMART_TARGET_VICTIM)` retorna `Vec[caster.victim]`
- [ ] Test: `SmartScript.get_targets(SMART_TARGET_CLOSEST_PLAYER, range=50)` retorna el player más cercano
- [ ] Test: `event_chance=50` dispara ~50% de las veces (test estadístico con N=1000)
- [ ] Test: `event_flags=NOT_REPEATABLE` el evento solo dispara 1 vez
- [ ] Test: `event_phase_mask=4` (phase 3) sólo dispara cuando script está en phase 3
- [ ] Test: `PetAI` con command=Stay no se mueve aunque owner se mueva
- [ ] Test: `PetAI` con command=Follow sigue al owner en chase distance
- [ ] Test: `PetAI` con react=Aggressive ataca a hostiles que entren al pet aggro radius
- [ ] Test: `PetAI` con react=Passive nunca ataca aunque sea atacado
- [ ] Test: `TotemAI` despawnea al expirar lifespan
- [ ] Test: `TotemAI` castea su totem spell según cd
- [ ] Test: `GuardAI` cuando es atacado, llama a guards en 50yd para asistencia
- [ ] Test: `CreatureAISelector.select_ai` con Pet → instancia PetAI; con Totem → TotemAI; con `creature_template.AIName='SmartAI'` → SmartAI; con AIName vacío y entry sin smart_scripts → NullCreatureAI
- [ ] Test: Boss con SmartScript y 3 phases — eventos de phase 1 no disparan en phase 2 ni 3

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 11 files / 1324 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp` | `crates/wow-ai/`, `crates/wow-script/`, `crates/wow-scripts/` \| 🔧 broken (rewrite needed) — sólo `CreatureAI` plain con states Idle/Walk/Combat; sin trait, sin SmartAI, sin polimorfismo. Crates `wow-script` y `wow-scripts` están **vacíos**. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#AI.DIV.001` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 11 C++ files / 1324 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#AI.DIV.002` | `crates/wow-scripts` (`exists_empty`, 0 Rust lines) | 11 C++ files / 1324 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#AI.DIV.003` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 11 C++ files / 1324 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#AI.DIV.004` | `crates/wow-scripts/src/lib.rs` (`exists_empty`, 0 Rust lines) | 11 C++ files / 1324 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAI.h`, `/home/server/woltk-trinity-legacy/src/server/game/AI/CreatureAISelector.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **SmartScript event/action es enorme:** En 3.4.3 hay ~80 SMART_EVENT_* y ~150 SMART_ACTION_*. La tabla `smart_scripts` tiene ~50k filas. Cada `entryOrGuid` (positivo=template, negativo=spawn) puede tener 50+ filas. La validación per-fila es densa: muchos action.params dependen del action_type concreto, así que SmartScriptMgr tiene un switch gigante de validación por separado del intérprete (SmartScriptMgr.cpp:IsEventValid). **No saltarse esta validación** en RustyCore — cargar inválido = crashes en runtime.
- **Leashing:** Cuando un mob persigue al player demasiado lejos (más de `leashRange`, default ~50yd del spawn), debe entrar `EnterEvadeMode(Boundary)`. Sin esto, los mobs te siguen al otro lado del mapa. C++ checa esto en `CreatureAI::CheckBoundary` cada tick (CreatureAI.cpp).
- **Evading vs Despawning:** Evade = vuelve a home con full HP, sigue spawneado. Despawn = se va del mundo, respawn timer empieza. Boss kills → JustDied → loot drop → corpse linger → DespawnOrUnsummon. Mob evade no despawnea. Confundirlos rompe respawns y logs (CreatureAI.cpp:EnterEvadeMode default).
- **Summons cascade:** Cuando un boss muere, sus summons (adds, totems, pets) deben despawnear automáticamente. C++ usa `SummonList::DespawnAll` en `JustDied`. Si ScriptedAI lo olvida → adds quedan vivos forever (ScriptedCreature.cpp:SummonList).
- **JustEngagedWith vs JustEnteredCombat:** Sutil distinción — `JustEnteredCombat` se dispara la primera vez que cualquiera te amenaza (incluso heal threat). `JustEngagedWith` se dispara la primera vez que tienes un real attacker. Para boss timers, usa `JustEngagedWith`. (CreatureAI.h:108-111).
- **Phase masks vs phases:** `SMART_EVENT_PHASE` (1-12) son fases del **boss** para eventos. Es bitmask: phase_mask=5 = bits 0+2 = phase 1 OR phase 3. No confundir con `PhasingHandler`/`PhaseShift` (zones/quests visibility).
- **Event link chain:** Un SMART_EVENT con `link != 0` apunta a otra fila por `id`. Cuando el primer event ejecuta su action, automáticamente ejecuta también la action del linked. Permite cadenas (event A → action 1 → action 2 → action 3) sin generar 3 events. Easy to break si IDs no son únicos por entry. (SmartScriptMgr.cpp validación de link).
- **Action timers/frames:** Algunas acciones tienen timer (SMART_ACTION_WAIT, SMART_ACTION_PAUSE_WAYPOINT). Durante ese timer, el script entero pausa o solo esa fila. Diferenciar bien — pausar todo el script vs pausar wp es distinto.
- **OocLOS vs IcLOS:** `SMART_EVENT_OOC_LOS` dispara cuando un unit entra LOS estando out-of-combat. `SMART_EVENT_IC_LOS` dispara estando in-combat (rare, ej: añadir target adicional al boss). Confundir = mobs aggro durante combat por sub-pulls.
- **EventAI legacy vs SmartAI:** Hay forks que aún tienen `creature_ai_template/scripts/summons` (sistema antiguo EventAI). En 3.4.3 mainline esto está deprecated y todo migrado a SmartAI. RustyCore puede ignorar EventAI completamente.
- **ScriptedEscortAI quest credit:** Al terminar el path (`EndPath`), el escort completa la quest del invoker (`AreaExplored` o quest objective N). Si fail (player se aleja, mata el escortado, etc.), no da credit. Lógica delicada para quest design.
- **Pet command stack:** Pet puede tener `m_savedReactState`, `m_isCommandAttack`, `m_isCommandFollow`. El UpdateAI lee estas flags por prioridad: command_attack > react_state. Si un proc cambia el target en medio de un command_attack, el pet pierde el comando manual. (PetAI.cpp:65-160).
- **Charm switching:** Cuando un Unit es charmed/possessed, su AI cambia a `PossessedAI`. Al expirar charm, vuelve a la AI original. Esto requiere preservar la AI original (no destruirla); typical bug es perderla y crear NullCreatureAI al uncharm. (CreatureAI.cpp:OnCharmed).
- **SmartScript concurrency:** El intérprete es single-threaded por map (todo el map updatea en orden). No hace falta locks pero CUIDADO con `process_action` que llama a `Aura::Apply` que puede llamar a otro `SmartScript::process_event` (chain). En C++ esto funciona por reentrancy del stack; en Rust con borrows estrictos puede dar problemas — necesitarás `RefCell` o defer las acciones a queue post-tick.
- **`me`/`me->GetVictim()` patterns:** ScriptedAI siempre asume `me` (la creature) y `me->GetVictim()` (target actual de combat). Muchos action invocan `me->GetVictim()` que puede ser null entre evade y next aggro — null checks obligatorios.
- **Performance (smart_scripts ~50k filas):** Carga inicial de SmartScriptMgr es lenta (varios segundos en C++). En RustyCore, paralelizar el parse + validation con rayon puede ahorrar mucho startup time. Cache final es immutable, share via `Arc<HashMap<i64, Vec<SmartScriptHolder>>>`.
- **WotLK 3.4.3 specifics:** No tiene los SMART_EVENT_/ACTION_ añadidos en MoP/WoD/Legion. Verifica que los enum values usados sean ≤ los IDs disponibles en 3.4.3 (el header de Wotlk Classic). El extractor de smart_scripts data puede traer IDs nuevos si la fuente DB es mainline TC — filtrar al cargar.
- **Boundary vs leash:** Boundary es área 2D/3D que un boss no debe salir (room). Leash es distancia al spawn. Algunos bosses tienen ambos (Onyxia: boundary del room + leash si escape). Distinct enums: `EvadeReason::Boundary` (left room), `EvadeReason::NoPath` (no hay path back) (CreatureAI.h:EvadeReason).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class UnitAI` (abstract) | `trait UnitAI` en `crates/wow-ai/src/unit_ai.rs` | Métodos abstractos `update_ai`, defaults para hooks |
| `class CreatureAI : UnitAI` | `trait CreatureAI: UnitAI` | Trait subtipo; default impls para hooks no override |
| `CreatureAI* i_AI` (member en Creature) | `Box<dyn CreatureAI>` campo en `Creature` | Owned, swap on charm/possess |
| `class SmartAI : CreatureAI` | `struct SmartAI { script: SmartScript }` impl CreatureAI | Composition over inheritance |
| `class PetAI : CreatureAI` | `struct PetAI { ... }` impl CreatureAI | Idem |
| `class ScriptedAI : CreatureAI` (helpers) | `struct ScriptedAIBase { events: EventMap, summons: SummonList }` + helper trait `ScriptedAIExt` | Composition; bosses concretos contienen ScriptedAIBase |
| `class BossAI : ScriptedAI` | `struct BossAI { base: ScriptedAIBase, instance: Option<Arc<InstanceScript>> }` | — |
| `EventMap` | `struct EventMap { events: BTreeMap<u32 deadline, EventEntry>, phase: u8 }` | BTreeMap for ordered by deadline |
| `SummonList` | `struct SummonListLikeCpp { storage: VecDeque<ObjectGuid> }` | List-like storage; pure plans until ScriptedAI/ObjectAccessor runtime wiring |
| `class SmartScript` | `struct SmartScript { events: Vec<SmartScriptHolder>, current_phase: u8, current_event_phase_mask: u32 }` | — |
| `class SmartScriptMgr` (singleton) | `static SMART_SCRIPT_MGR: OnceLock<SmartScriptMgr>` | Carga inicial sync |
| `enum SMART_EVENT_*` (~80) | `enum SmartEvent { UpdateIc, UpdateOoc, HealthPct{ min, max, cd_min, cd_max }, ... }` | Enum con datos asociados — más type-safe que C++ uint params |
| `enum SMART_ACTION_*` (~150) | `enum SmartAction { Talk{ group_id }, CastSpell{ spell_id, flags }, AddAura{ spell_id, target }, MoveToPos{ x, y, z }, ... }` | Idem |
| `enum SMART_TARGET_*` (~30) | `enum SmartTarget { None, Self, Victim, ClosestPlayer{ range }, ... }` | Idem |
| `WorldDatabase.Query("SELECT * FROM smart_scripts")` | `sqlx::query_as!(SmartScriptRow, "SELECT ... FROM smart_scripts").fetch_all(&pool)` | Async load on startup |
| `CreatureAISelector::selectAI` | `fn select_ai(creature: &Creature) -> Box<dyn CreatureAI>` | Match sobre creature flags + AIName |
| `FactoryHolder<CreatureAI, Creature, std::string>` | `inventory::submit!(CreatureAIFactory { name, ctor })` registry | inventory crate |
| `void Foo::JustDied(Unit*)` | `fn just_died(&mut self, killer: &Unit) { /* default no-op */ }` | trait method con default |
| `Unit::AI()` (returns `UnitAI*`) | `unit.ai() -> &mut dyn UnitAI` | Borrow checker friendly |
| `me->CastSpell(target, spellId, false)` | `self.creature.cast_spell(target, spell_id, false).await` | Async porque pasa por Spell pipeline |
| `me->AI()->JustEnteredCombat(who)` | Dispatch desde Combat module: `creature.ai_mut().just_entered_combat(who)` | Necesita careful borrow handling |
| `events.ScheduleEvent(EVENT_FIREBALL, 5s, GROUP_FIRE, PHASE_1)` | `self.events.schedule_event(EventId::Fireball, 5_000, 1, 1)` | u32 enum value |

---

*Template version: 1.0 (2026-05-01).* Initial complete audit port.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/AI/` (`CreatureAI.{h,cpp}`, `CoreAI/UnitAI.{h,cpp}`, `CoreAI/CombatAI.{h,cpp}`, `CoreAI/PetAI.{h,cpp}`, `CoreAI/TotemAI.{h,cpp}`, `CoreAI/PassiveAI.{h,cpp}`, `CoreAI/GuardAI.{h,cpp}`, `CoreAI/ReactorAI.{h,cpp}`, `CoreAI/ScheduledChangeAI.{h,cpp}`, `ScriptedAI/ScriptedCreature.{h,cpp}`, `ScriptedAI/ScriptedEscortAI.{h,cpp}`, `ScriptedAI/ScriptedFollowerAI.{h,cpp}`, `SmartScripts/SmartAI.{h,cpp}` ~1.6k lines, `SmartScripts/SmartScript.{h,cpp}` ~4.4k, `SmartScripts/SmartScriptMgr.{h,cpp}` ~4.3k, `CreatureAISelector.{h,cpp}`) against the Rust workspace at `/home/server/rustycore/crates/`.

**Empty-crate finding — partial.** `crates/wow-script/src/lib.rs` and `crates/wow-scripts/src/lib.rs` measure **0 lines each** (verified via `wc -l`). The crates are workspace members but ship no code. `crates/wow-ai/src/lib.rs` is **not** empty: ~1259 lines containing a single concrete `struct CreatureAI` (no trait), `CreatureState`, selector helpers, BossAI view and represented `SummonListLikeCpp` foundations. There is no `trait CreatureAI`, no `trait UnitAI`, no `Box<dyn ...>` handle, no `CreatureAISelector` runtime, no `inventory::submit!` factory registry. The 16k+ lines of C++ AI/ subtree still lack the runtime polymorphic AI stack.

**SmartAI presence.** **None.** No `SmartAI`, no `SmartScript`, no `SmartScriptMgr`, no `SmartScriptHolder`, no `SmartTarget`, no `enum SmartEvent`, no `enum SmartAction`, no `process_event`, no `process_action`. The only trace of SmartScripts in the entire repo is a single SQL prepared-statement constant `SEL_SMART_SCRIPTS` in `crates/wow-database/src/statements/world.rs:15` (`SELECT ... FROM smart_scripts ORDER BY entryorguid, source_type, id, link`) that is **never executed by any consumer** — there is no loader code, no parser, no in-memory cache, no `mEventMap: HashMap<i64, Vec<SmartScriptHolder>>`. Since SmartAI is the data-driven engine that runs ~95% of the game's mob and boss content (~50k rows in the `smart_scripts` table — every boss's talk lines, phase transitions, ability rotations, summons, waypoint paths), the absence means **no creature in the game currently has any scripted behavior beyond random wandering and a flat-damage auto-attack**.

**AI subclass coverage.** C++ ships at minimum the following concrete subclasses, all of which are **missing in Rust**: `NullCreatureAI`, `PassiveAI`, `PossessedAI`, `CritterAI`, `TriggerAI`, `CombatAI`, `AggressorAI`, `PetAI`, `TotemAI`, `GuardAI`, `ReactorAI`, `ScheduledChangeAI`, `ScriptedAI`, `BossAI`, `WorldBossAI`, `EscortAI`, `FollowerAI`, `SmartAI`. Rust has **one** struct masquerading as all of them. `EventMap` and `SummonListLikeCpp` now exist as represented foundations, but neither is wired into a real `ScriptedAI`/`BossAI` runtime; there is no `creature_text` loader (so no localized boss talk lines), no `waypoint_data`/`waypoint_path` loader (so no patrol/escort routes).

**Hooks coverage.** C++ defines a wide hook surface — `Reset`, `JustEnteredCombat`, `JustEngagedWith`, `JustDied`, `KilledUnit`, `MoveInLineOfSight`, `TriggerAlert`, `EnterEvadeMode(EvadeReason)`, `SpellHit`, `SpellHitTarget`, `JustSummoned`, `IsSummonedBy`, `SummonedCreatureDies`, `JustReachedHome`, `ReceiveEmote`, `MovementInform`, `OnHealthDepleted`, `OnGameEvent`, `DoZoneInCombat`. Rust now has represented foundations for `EvadeReason`/`EnterEvadeMode` planning, but **dispatches none of these hooks through a real polymorphic AI runtime** — the existing struct only has `try_aggro`, `enter_combat`, `reset_combat`, `take_damage`, `die`, `respawn`, `should_wander` (each called inline from creature ticks). Because there is no hook surface, even when the missing engines (Combat/Spells/Movement) are filled in there is no place for them to call into the AI to inform it of `JustEnteredCombat` / `SpellHit` / `MovementInform` events.

**Selector / ScriptName binding.** C++ `CreatureAISelector::selectAI(Creature*)` decides at spawn time whether to instantiate a Pet/Totem/Vehicle AI, then a SmartAI if the entry has rows in `smart_scripts`, then the AIName from `creature_template`, falling back to `NullCreatureAI`. Rust has no selector and no factory: every spawn becomes the same `CreatureAI` struct. The `creature_template.AIName` and `ScriptName` columns are unused.

**Worst divergence.** **The data-driven content layer is completely disconnected from execution.** TrinityCore's design is that ~95% of NPC behavior is *not* compiled C++ — it lives in the `smart_scripts` SQL table and is interpreted at runtime by `SmartScript::OnUpdate`. Rust has the SQL connection, has the prepared statement constant, has the schema knowledge in this doc — but has zero interpreter, zero `SmartEvent`/`SmartAction`/`SmartTarget` enums, and no plan to wire the `inventory::submit!` registry that boss C++ scripts would use. Even if `wow-spell` and `wow-combat` are filled in tomorrow, **every boss fight will play as "stands in place, swings white melee, dies silently"** until the SmartScript interpreter (an estimated XL spanning §9 tasks #AI.17 → #AI.30, i.e. ~80 SmartEvent variants × ~150 SmartAction variants × ~30 SmartTarget variants × validation × loader × runtime dispatcher) is ported. This is the single biggest "content blocker" in the engines layer — engine work without it produces a server that boots, lets you connect, and shows you mobs that have nothing to say or do.
