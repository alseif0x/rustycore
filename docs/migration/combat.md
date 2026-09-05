# Migration: Combat

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Combat/` (+ `src/server/game/Server/Packets/CombatPackets.{h,cpp}`, plus damage helpers spread across `Entities/Unit/Unit.cpp`)
> **Rust target crate(s):** `crates/wow-combat/` (vacío hoy), `crates/wow-packet/src/packets/combat.rs` (packets), `crates/wow-world/src/handlers/combat.rs` (handlers)
> **Layer:** L5 (depende de Entities L4 + Spells L5 + AI L5)
> **Status:** 🔧 broken (rewrite needed) — auto-attack server-driven trivial, sin school/resistance/threat real, sin tabla de outcomes (miss/dodge/parry/block); crate `wow-combat` está **vacío**.
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing — see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Mantiene el **estado de combate** entre unidades y la **resolución de ataques melee/ranged**. Dos sub-sistemas: (1) `CombatManager` rastrea pares-en-combate (PvE + PvP) y arbitra entrada/salida de combate (con suppress flags para vanish/feign-death y timeout PvP de 5s). (2) `ThreatManager` mantiene listas de amenaza por creature (heap ordenado por threat amount + online/suppressed/offline + taunt state) y selecciona el `current victim`. La resolución de daño melee (roll de outcome: miss/dodge/parry/block/glancing/crushing/normal/crit, mitigación por armor, cap por school resistance, durability damage) vive en `Unit::CalcDamageInfo` / `Unit::DealDamage` / `Unit::AttackerStateUpdate` (no en `Combat/`).

---

## 2. C++ canonical files

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Combat/CombatManager.h` | 146 | `CombatReference`, `PvPCombatReference`, `CombatManager` (per-Unit) |
| `src/server/game/Combat/CombatManager.cpp` | 406 | SetInCombatWith, EndCombat, suppress logic, PvP timer, NotifyAICombat |
| `src/server/game/Combat/ThreatManager.h` | 321 | `ThreatReference`, `ThreatManager` API (AddThreat/ScaleThreat/Taunt/Fixate/Redirect) |
| `src/server/game/Combat/ThreatManager.cpp` | 913 | Heap maintenance, victim selection, modifier table, redirect, send-to-clients |
| `src/server/game/Server/Packets/CombatPackets.h` | 240 | AttackSwing/Stop/SetSheathed CMSG + AttackStart/Stop/SwingError/AttackerStateUpdate/AIReaction SMSG |
| `src/server/game/Server/Packets/CombatPackets.cpp` | 166 | Read/Write impls |
| `src/server/game/Handlers/CombatHandler.cpp` | ~150 | `WorldSession::HandleAttackSwingOpcode/AttackStop/SetSheathed` |
| `src/server/game/Entities/Unit/Unit.cpp` (combat sections) | ~2500 (subset) | `AttackerStateUpdate`, `CalculateMeleeDamage`, `RollMeleeOutcomeAgainst`, `DealMeleeDamage`, `MeleeDamageBonus*`, `CalcArmorReducedDamage`, `CalcSpellResistedDamage` |
| `src/server/game/Entities/Unit/Unit.h` (combat decls) | ~3500 (subset) | enums `MeleeHitOutcome`, `WeaponAttackType`, `DamageInfo`, `CalcDamageInfo`, `CleanDamage`, `SpellNonMeleeDamage` |
| `src/server/game/Entities/Player/Player.cpp` (combat) | partial | XP on kill, durability damage on death, party kill log |
| `src/server/game/Loot/...` | — | Hooks on kill (loot generation triggered by combat death) |
| `src/server/game/Quests/QuestDef.cpp` | — | Kill credit on combat death |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CombatReference` | struct | Edge entre dos Units en combate; suppress flag por lado |
| `PvPCombatReference` | struct (extends CombatReference) | PvP-specific: 5s combat timer, refresh-on-action |
| `CombatManager` | class (per Unit) | `_pveRefs`/`_pvpRefs` maps; SetInCombatWith / EndAllCombat / Update |
| `ThreatReference` | class | Edge en threat list: owner ↔ victim, threat amount, taunt state, online state |
| `ThreatManager` | class (per Unit, mostly Creature) | Heap de ThreatReference*, GetCurrentVictim, AddThreat, redirect, fixate |
| `ThreatManager::Heap` | nested class | Min-heap inverso (max-threat first) ordered por `CompareThreatLessThan` |
| `ThreatManager::ThreatListIterator` | nested class | Iterador estable sobre heap |
| `OnlineState` | enum (in ThreatReference) | ONLINE / SUPPRESSED / OFFLINE |
| `TauntState` | enum (in ThreatReference) | TAUNT / NONE / DETAUNT |
| `MeleeHitOutcome` | enum | EVADE/MISS/DODGE/PARRY/BLOCK/GLANCING/CRUSHING/CRIT/NORMAL |
| `WeaponAttackType` | enum | BASE_ATTACK / OFF_ATTACK / RANGED_ATTACK |
| `DamageInfo` | class (Unit.h) | wrapper: damage, school, hit type, absorb/resist, blocked |
| `CalcDamageInfo` | struct | Computed damage + outcome para AttackerStateUpdate |
| `CleanDamage` | struct | absorb/resist subtracted vs raw damage |
| `SpellNonMeleeDamage` | struct | Para `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` |
| `SpellSchool` / `SpellSchoolMask` | enum / bitfield | NORMAL/HOLY/FIRE/NATURE/FROST/SHADOW/ARCANE (mask = 1<<school) |
| `DamageEffectType` | enum | DIRECT_DAMAGE / SPELL_DIRECT_DAMAGE / DOT / HEAL / NODAMAGE / SELF_DAMAGE |
| `HitInfo` | enum (bitfield) | NORMALSWING / CRITICALHIT / MISS / GLANCING / CRUSHING / etc. (used in attacker-state-update flags) |
| `CompareThreatLessThan` | functor | Heap comparator |
| `WorldPackets::Combat::AttackSwing` | packet | CMSG_ATTACK_SWING (target GUID) |
| `WorldPackets::Combat::AttackStop` | packet | CMSG_ATTACK_STOP |
| `WorldPackets::Combat::SetSheathed` | packet | CMSG_SET_SHEATHED |
| `WorldPackets::Combat::AttackStart` | packet | SMSG_ATTACK_START |
| `WorldPackets::Combat::SAttackStop` | packet | SMSG_ATTACK_STOP |
| `WorldPackets::Combat::AttackerStateUpdate` | packet | SMSG_ATTACKER_STATE_UPDATE (gran payload: hit info, damage, blocks, absorbs) |
| `WorldPackets::Combat::AttackSwingError` | packet | SMSG_ATTACK_SWING_ERROR (notarget, deadtarget, etc.) |
| `WorldPackets::Combat::AIReaction` | packet | SMSG_AI_REACTION (mob aggro'd) |
| `WorldPackets::Combat::PartyKillLog` | packet | SMSG_PARTY_KILL_LOG (XP-share log) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CombatManager::Update(uint32 tdiff)` | Tick PvP timers + propagate end | `PvPCombatReference::Update`, `EndCombat` |
| `CombatManager::SetInCombatWith(Unit* who, bool suppressed)` | Crea `CombatReference` y notifica AI | `PutReference`, `NotifyAICombat`, `Unit::SetInCombatState` |
| `CombatManager::IsInCombatWith(ObjectGuid/Unit)` | Lookup en `_pveRefs/_pvpRefs` | hashmap find |
| `CombatManager::EndCombatBeyondRange(range, includingPvP)` | Limpia refs lejanos (evade) | `CombatReference::EndCombat` |
| `CombatManager::SuppressPvPCombat()` | Marca todos PvP refs como suppressed (vanish, etc.) | iterar `_pvpRefs` |
| `CombatManager::EndAllPvECombat()` / `EndAllPvPCombat()` / `EndAllCombat()` | Cleanup completo | iter+EndCombat |
| `CombatManager::RevalidateCombat()` | Re-evalúa cada ref si sigue válido (charm, faction change) | per-ref check |
| `CombatManager::CanBeginCombat(a, b)` (static) | Chequea faction/flags/inmune | `Unit::IsValidAttackTarget` etc. |
| `CombatReference::EndCombat()` | Quita ref de ambos lados, dispara `Unit::ClearInCombat` si último | `CombatManager::PurgeReference` ×2 |
| `CombatReference::SuppressFor(Unit*)` | Marca lado como suppressed | bool flip |
| `PvPCombatReference::Update(tdiff)` | Decrementa timer, true si expiró | timer math |
| `PvPCombatReference::RefreshTimer()` | Reset 5s | timer = 5s |
| `ThreatManager::Initialize()` | Setup _ownerCanHaveThreatList | `CanHaveThreatList(owner)` |
| `ThreatManager::Update(tdiff)` | Re-eval current victim, update temp modifiers, send to clients | `UpdateVictim`, `SendThreatListToClients` |
| `ThreatManager::AddThreat(Unit* target, float amount, SpellInfo*, ignoreModifiers, ignoreRedirects)` | Inserta/incrementa threat ref + crea CombatReference | `CalculateModifiedThreat`, `PutThreatListRef`, `CombatManager::SetInCombatWith` |
| `ThreatManager::ScaleThreat(Unit* target, float factor)` | Multiplica threat actual por factor (0=reset) | heap re-sort |
| `ThreatManager::ModifyThreatByPercent(target, pct)` | Wrapper `ScaleThreat(0.01*(100+pct))` | id |
| `ThreatManager::MatchUnitThreatToHighestThreat(target)` | Set target's threat = max(list) | id |
| `ThreatManager::TauntUpdate()` | Re-evalúa current victim por taunt aura | iter taunts |
| `ThreatManager::ResetAllThreat()` / `ClearAllThreat()` | Reset / wipe | iter+update |
| `ThreatManager::FixateTarget(target)` / `ClearFixate()` | Lock victim hasta clear | direct field |
| `ThreatManager::GetCurrentVictim()` / `GetLastVictim()` | Top-of-heap online target | heap peek |
| `ThreatManager::GetThreat(Unit, includeOffline)` | Lookup threat amount | hashmap find |
| `ThreatManager::ForwardThreatForAssistingMe(assistant, base, spell)` | Asistencia: añade threat a assistant en cada ref | iter+AddThreat |
| `ThreatManager::RemoveMeFromThreatLists()` | Quita owner de TODOS los heaps que lo tengan como victim | iter `_threatenedByMe` |
| `ThreatManager::RegisterRedirectThreat(spellId, victim, pct)` | Misdirect/Tricks of the Trade | redirect map |
| `ThreatManager::EvaluateSuppressed(canExpire)` | Por aura immunity/CC, pasa ONLINE → SUPPRESSED | iter+state |
| `Unit::AttackerStateUpdate(victim, attType, extra)` | Punto de entrada de auto-attack: roll + damage + send packets | `CalculateMeleeDamage`, `DealMeleeDamage`, `SendAttackStateUpdate` |
| `Unit::CalculateMeleeDamage(victim, damage, CalcDamageInfo*, attType)` | Calcula outcome + raw damage | `RollMeleeOutcomeAgainst`, `MeleeDamageBonusDone/Taken`, `CalcArmorReducedDamage` |
| `Unit::RollMeleeOutcomeAgainst(victim, attType)` | Tabla 100-roll: miss/dodge/parry/block/glancing/crushing/crit/normal | `MeleeMissChanceCalc`, `GetUnitDodgeChance`, etc. |
| `Unit::CalcArmorReducedDamage(attacker, victim, damage, spellInfo)` | Armor mitigation por nivel | armor formula |
| `Unit::CalcSpellResistedDamage(...)` | School resistance (binary 25%/50%/75% buckets) | resist formula |
| `Unit::DealMeleeDamage(CalcDamageInfo*, durabilityLoss)` | Aplica daño + procs + durability + AI hooks | `DealDamage`, `ProcDamageAndSpell` |
| `Unit::DealDamage(victim, damage, cleanDamage, damagetype, schoolMask, spellProto, durabilityLoss)` | Genérico: aplica HP loss, dispara Kill, procs | `Kill`, `Player::SendAttackSwingDeadTarget`, etc. |
| `Unit::Kill(victim, durabilityLoss)` | Death pipeline: lootgen, XP, kill credit, party log | `Player::RewardPlayerAndGroupAtKill`, `LootMgr::LoadLoot`, etc. |
| `Unit::SendAttackStateUpdate(CalcDamageInfo*)` | Construye SMSG_ATTACKER_STATE_UPDATE + broadcast | `BroadcastPacket` |
| `Unit::SendMeleeAttackStart(victim)` / `SendMeleeAttackStop(victim)` | SMSG_ATTACK_START/STOP | `SendMessageToSet` |
| `Unit::SetInCombatWith(enemy)` / `ClearInCombat()` | Bridge a CombatManager | `CombatManager::SetInCombatWith` |
| `WorldSession::HandleAttackSwingOpcode(WorldPacket&)` | Player→Unit attack request | `Player::Attack` |
| `WorldSession::HandleAttackStopOpcode(WorldPacket&)` | Stop swing | `Player::AttackStop` |
| `WorldSession::HandleSetSheathedOpcode(WorldPacket&)` | Cosmetic weapon state | `Unit::SetSheath` |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Unit` — `Unit::IsValidAttackTarget`, `Unit::SetInCombatState`, todas las stats (level, armor, weapon damage range, hit/crit/dodge chances).
- `Entities/Creature` — `CreatureAI::JustEnteredCombat`, `EnterEvadeMode`; `CanHaveThreatList` se filtra a creatures non-pet.
- `Entities/Player` — `Player::RewardPlayerAndGroupAtKill`, `Player::DurabilityLossAll`, `Player::SendAttackSwingError`.
- `Spells` — `SpellInfo` modula threat (`AdjustThreatTakers`, `THREAT_MULT`); efectos PROC tras AttackerStateUpdate; school resistance bucket vía `SpellSchoolMask`.
- `Spells/Auras` — taunt aura (`SPELL_AURA_MOD_TAUNT`), threat-mod (`SPELL_AURA_MOD_THREAT`), damage-school resist auras, vanish/feign death (suppress flag).
- `AI/CreatureAI` — `JustEnteredCombat`, `JustExitedCombat`, `EnterEvadeMode`, `AttackStart`, `KilledUnit`, `JustDied`.
- `Maps` / `Grids` — visibility for kill credit + range checks (EndCombatBeyondRange).
- `Loot` — death disparada por combat → `LootMgr::FillLoot`.
- `Quests` — `Player::KilledMonsterCredit` desde `Unit::Kill`.
- `XP` (DBC `XPGainBaseMap`) — desde `RewardPlayerAndGroupAtKill`.
- `Reputation` — kill rewards faction rep (`OnKillReputation`).
- `Battlegrounds` — kill counted en `BattlegroundScore`.
- `Achievements` — `OnKillCreatureType` hook.
- `Pets/Vehicles` — pet damage atribuido al owner para threat redirection.

**Depended on by:**
- `Entities/Unit` — `Unit::Update` llama a `CombatManager::Update` y a `ThreatManager::Update`.
- `Entities/Creature::Update` — usa `GetCurrentVictim()` para set new target.
- `Spells` — los spells crean threat (AddThreat) tras hit; misdirect/tricks redirige.
- `AI` — toda la decision logic se basa en `ThreatManager::GetCurrentVictim`.
- `Battlegrounds`/`Arenas` — usan PvPCombatReference timers para "in combat" UI.

---

## 6. SQL / DB queries (if any)

Combat propio **no emite queries directamente**. Datos relevantes vienen de tablas Unit-side:

| Statement / Source | Purpose | DB |
|---|---|---|
| `creature_template` | base damage range, school, resistances, armor | world |
| `creature_template_resistance` | per-school resistance overrides | world |
| `creature_template_immunity` | auras inmunes (suppresses combat ref) | world |
| `creature_classlevelstats` | base stats por (class, level, expansion) | world |
| `creature_threat_modifier` (raro) | per-creature threat multiplier | world |
| `gameobject_template` | trap/destructible damage values | world |
| `npc_spellclick_spells` | (indirect) abilities que generan combat | world |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `gtCombatRatingsStore` | Combat rating coefficient | Unit melee chance calc |
| `gtRegenHPPerSptStore` / `gtRegenMPPerSptStore` | OOC regen | Unit |
| `gtChanceToMeleeCritStore` | Melee crit base per class+level | Unit::GetUnitMeleeCriticalChance |
| `gtChanceToSpellCritStore` | Spell crit | Unit::SpellCriticalDamageBonus |
| `gtArmorMitigationByLvlStore` (3.4.3) | Armor mitigation cap | CalcArmorReducedDamage |
| `ChrClassesStore` | Class power type | Damage calc |
| `SpellStore` (vía SpellInfo) | School + threat coeff per spell | AddThreat |
| `WeaponSwingTimeStore` | Default weapon speeds | AttackerStateUpdate timer |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_ATTACK_SWING` (0x3255) | C→S | `WorldSession::HandleAttackSwingOpcode` |
| `CMSG_ATTACK_STOP` (0x3256) | C→S | `WorldSession::HandleAttackStopOpcode` |
| `CMSG_SET_SHEATHED` (0x3489) | C→S | `WorldSession::HandleSetSheathedOpcode` |
| `CMSG_DUEL_RESPONSE` (0x34E2) | C→S | (PvP duel flow) |
| `SMSG_ATTACK_START` (0x293D) | S→C | `Unit::SendMeleeAttackStart` (broadcast) |
| `SMSG_ATTACK_STOP` (0x293E) | S→C | `Unit::SendMeleeAttackStop` |
| `SMSG_ATTACK_SWING_ERROR` (0x294C) | S→C | NOTARGET / DEADTARGET / NOTINRANGE / BADFACING / CANTATTACK |
| `SMSG_ATTACK_SWING_LANDED_LOG` (0x294D) | S→C | extended log con CalcDamageInfo |
| `SMSG_ATTACKER_STATE_UPDATE` (0x2952) | S→C | per-swing packet: HitInfo flags, damage, school mask, absorbed, resisted, blocked, rage gain |
| `SMSG_AI_REACTION` (0x26B5) | S→C | "mob noticed you" — emitida en SetInCombatWith |
| `SMSG_PARTY_KILL_LOG` (0x275A) | S→C | XP-share death log |
| `SMSG_DURABILITY_DAMAGE_DEATH` (0x2745) | S→C | item durability tick on death |
| `SMSG_ENVIRONMENTAL_DAMAGE_LOG` (0x2C1E) | S→C | fall/lava/drowning damage log |
| `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` (0x2C2F) | S→C | spell direct damage |
| `SMSG_THREAT_UPDATE` (0x2A30 family) | S→C | per-target threat list update for client UI |
| `SMSG_THREAT_REMOVE` | S→C | victim left list |
| `SMSG_THREAT_CLEAR` | S→C | wipe (evade/death) |
| `SMSG_HIGHEST_THREAT_UPDATE` | S→C | new tank notice |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-combat/src/lib.rs` — **0 líneas** (sólo `Cargo.toml`); placeholder.
- `crates/wow-packet/src/packets/combat.rs` — ~190 líneas — cubre AttackSwing/AttackStop/SetSheathed (CMSG), AttackStart/SAttackStop/AttackerStateUpdate/AttackSwingError (SMSG). ~15% del C++ packet set.
- `crates/wow-world/src/handlers/combat.rs` — 152 líneas — handle_attack_swing / handle_attack_stop / handle_set_sheathed. Trivial state mgmt en `WorldSession` (`combat_target`, `in_combat`, `creatures.enter_combat`).
- `crates/wow-ai/src/lib.rs` — `CreatureState::InCombat` enum + `enter_combat`/`reset_combat` métodos triviales en creature struct.

**What's implemented:**
- Recibir CMSG_ATTACK_SWING → validar target alive → marcar creature `InCombat` + `WorldSession::in_combat = true` → enviar SMSG_ATTACK_START.
- Recibir CMSG_ATTACK_STOP → reset state + SMSG_ATTACK_STOP.
- Recibir CMSG_SET_SHEATHED → log only.
- Auto-attack tick: hay un loop server-side que decrementa HP del target en intervalos fijos (sin tabla de hit outcomes — hit garantizado, daño fijo o random simple).
- Death detection trivial: cuando creature.hp ≤ 0 → mark dead, send SMSG_ATTACK_STOP con `now_dead=true`.

**What's missing vs C++:**
- **`CombatManager` completo** — no hay struct, no hay PvE/PvP refs, no hay suppress flags, no hay PvP timer 5s, no hay `SetInCombatWith`/`EndAllCombat`/`EndCombatBeyondRange`/`RevalidateCombat`.
- **`ThreatManager` completo** — no existe heap, no hay AddThreat, ScaleThreat, taunt, fixate, redirect, online/suppressed/offline state, threat list send-to-client.
- **Roll de outcome melee** — no hay `RollMeleeOutcomeAgainst`. Falta tabla de probabilidades miss/dodge/parry/block/glancing/crushing/crit/normal por nivel del attacker vs defender.
- **Hit chance por skill diff** (level-based weapon skill cap, "yellow attack table" vs "white attack table" en 3.4.3).
- **Armor mitigation** — no hay `CalcArmorReducedDamage` (formula `armor / (armor + 467.5×level - 22167.5)` para WotLK).
- **School resistance** — el campo `SpellSchoolMask` ni siquiera existe en damage; un golpe físico siempre asume "Normal" school. Sin buckets 25%/50%/75%/100% resist.
- **Block / Parry / Dodge** — sin checks de stats, sin animation hookup, sin `SMSG_ATTACK_SWING_PARRIED`/etc.
- **Crit** — sin tabla DBC `gtChanceToMeleeCritStore`, sin scaling por nivel.
- **Glancing/Crushing** — los blows penalizados por skill-cap no se aplican (importante para PvE cap 70+).
- **Mitigation order** — el orden correcto es: Avoidance (miss/dodge/parry) → Block reduction → Resistance/Absorb → Armor mitigation → Crit/Glance multiplier → final. En Rust no se hace ningún paso.
- **Damage absorb shields** — sin lookup de auras tipo `SPELL_AURA_SCHOOL_ABSORB`.
- **Procs on melee** — sin `ProcDamageAndSpell`.
- **`SMSG_ATTACKER_STATE_UPDATE`** — el packet existe en Rust pero no se construye con HitInfo flags reales (cliente verá siempre normal hit).
- **Taunt mechanics** — no hay aura con efecto mod-taunt; no hay forced-target-set.
- **Misdirection / Tricks of the Trade** — sin redirect table.
- **Pet threat redirection** — pet damage no se atribuye al owner para threat.
- **Kill rewards** — no hay XP, no hay quest credit, no hay reputation, no hay loot generación on kill.
- **Durability damage on death** — sin pipe.
- **PvE/PvP detection** — el handler no diferencia.
- **Evade mechanics** — no hay distance check ni leash; creature no vuelve a casa cuando players salen de rango.
- **Combat broadcasting** — solo el atacante recibe SMSG_ATTACK_START; observadores cercanos no reciben broadcast.
- **AI hooks**: no `JustEnteredCombat`/`JustExitedCombat`/`KilledUnit`/`JustDied` invocados.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `WorldSession::combat_target: Option<ObjectGuid>` asume **un solo** combat target — C++ permite múltiples atacantes simultáneos. Cuando se popule MotionMaster + AI, esto será insuficiente.
- `creatures.get_mut(&victim).enter_combat(player_guid)` solo registra al player como atacante; si el creature ya está en combate con otro player, sobreescribe.
- Auto-attack timer probablemente no respeta weapon attack speed (1.5–4.0s típico); si está hardcoded a un intervalo fijo, hits per second estarán mal.
- Damage value posiblemente usa `creature.attack_damage` plano sin variance min/max — C++ usa `min_dmg/max_dmg` ± random.
- Sin handling de `PowerType::Rage` para guerreros: cada hit melee da rage; cada hit recibido también.
- El `is_alive` check en `creatures` no propaga `now_dead=true` correctamente si la death viene de un spell DoT (no implementado).

**Tests existing:**
- 0 tests en `crates/wow-combat/` (crate vacío).
- 0 tests en `crates/wow-packet/src/packets/combat.rs`.
- 0 tests en `crates/wow-world/src/handlers/combat.rs`.

---

## 9. Migration sub-tasks

Numera para referencia desde `MIGRATION_ROADMAP.md` §5.
Complejidad: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#COMBAT.1** Llenar `crates/wow-combat/` con módulos `combat_manager`, `threat_manager`, `damage`, `outcome`, `school`. (L)
- [ ] **#COMBAT.2** `enum SpellSchool` + `SpellSchoolMask` (bitfield) idéntico a C++ (`Normal=0,Holy=1,Fire=2,Nature=3,Frost=4,Shadow=5,Arcane=6`). (L)
- [ ] **#COMBAT.3** `enum MeleeHitOutcome` (Evade/Miss/Dodge/Parry/Block/Glancing/Crushing/Crit/Normal) + `enum WeaponAttackType` (Base/Off/Ranged) + `bitflags HitInfo`. (L)
- [ ] **#COMBAT.4** `struct CombatReference { first, second, is_pvp, suppress_first, suppress_second }` + `PvPCombatReference { timer: u32 }`. (M)
- [ ] **#COMBAT.5** `struct CombatManager { owner, pve_refs: HashMap<Guid, Box<CombatReference>>, pvp_refs: HashMap<Guid, Box<PvPCombatReference>> }` + métodos `set_in_combat_with`, `update`, `end_combat_beyond_range`, `revalidate`, `end_all`. (H)
- [ ] **#COMBAT.6** `static CombatManager::can_begin_combat(a, b)` con todas las checks (faction, gm, immune, dead). (M)
- [ ] **#COMBAT.7** `struct ThreatReference { owner, victim, amount, online_state, taunt_state, temp_modifier }` + comparator. (M)
- [ ] **#COMBAT.8** `struct ThreatManager { owner, heap: BinaryHeap<…>, refs: HashMap<Guid, …>, threatened_by_me: HashMap<…>, current_victim, fixate, redirects: …, update_timer }` + `add_threat`, `scale_threat`, `taunt_update`, `fixate_target`, `register_redirect_threat`. (H)
- [ ] **#COMBAT.9** `ThreatManager::get_current_victim` con tie-break correcto: Online > Suppressed > Offline, luego Taunt > None > Detaunt, luego threat amount. (M)
- [ ] **#COMBAT.10** `CalcDamageInfo` struct: damage[2], school[2], absorb[2], resist[2], blocked, hit_outcome, hit_info_flags, target_state, proc_attacker/proc_victim/proc_ex. (M)
- [ ] **#COMBAT.11** `roll_melee_outcome(attacker, victim, attack_type) -> MeleeHitOutcome` — tabla de roll con miss%, dodge%, parry%, block%, glancing%, crushing%, crit%; usar `gtChanceToMeleeCritStore` DBC. (XL)
- [ ] **#COMBAT.12** `calc_armor_reduced_damage(level, armor, damage) -> u32` con la fórmula 3.4.3 (lookup en `gtArmorMitigationByLvlStore`). (M)
- [ ] **#COMBAT.13** `calc_spell_resisted_damage(school_mask, target_resist, damage) -> (damage, resisted)` con 25/50/75/100 buckets. (M)
- [ ] **#COMBAT.14** `Unit::attacker_state_update` end-to-end: build CalcDamageInfo → roll outcome → mitigate → apply HP → send SMSG_ATTACKER_STATE_UPDATE → procs → AI hook. (XL)
- [ ] **#COMBAT.15** `Unit::deal_damage` genérico (usado por melee + spell + environmental) con kill detection. (H)
- [ ] **#COMBAT.16** `Unit::kill(victim, durability_loss)`: drop loot, give XP, kill credit (quest), reputation, party kill log, durability damage. (H)
- [ ] **#COMBAT.17** SMSG_ATTACKER_STATE_UPDATE writer completo (HitInfo flags, dual-school, absorb/resist/block, rage gain, target_state). (H)
- [ ] **#COMBAT.18** SMSG_ATTACK_SWING_ERROR enum (NotInRange/BadFacing/NotStanding/InvalidAttackTarget/etc.) + dispatch desde validation paths. (M)
- [ ] **#COMBAT.19** SMSG_AI_REACTION + integración con `ThreatManager::add_threat` (primer threat → emit reaction). (M)
- [ ] **#COMBAT.20** SMSG_THREAT_UPDATE / THREAT_REMOVE / THREAT_CLEAR / HIGHEST_THREAT_UPDATE para client UI. (M)
- [ ] **#COMBAT.21** Threat redirection (Misdirection / Tricks of the Trade): `register_redirect_threat(spell_id, victim, pct)` + apply en `add_threat`. (M)
- [ ] **#COMBAT.22** Pet damage attribution: pet hits → owner gana threat en target, pet hits → owner combat ref se crea. (M)
- [ ] **#COMBAT.23** Auto-attack timer correcto: weapon swing time desde Item DBC; main + off-hand independientes; off-hand tiene 19% miss penalty extra. (H)
- [ ] **#COMBAT.24** Rage generation en `attacker_state_update` (formula del 3.4.3: `damage*7.5/2/rage_conversion_value`). (M)
- [ ] **#COMBAT.25** Energy regen + power tick durante combate (cada 2s para rogues, cada 5s OOC). (M)

---

## 10. Regression tests to write

- [ ] Test: `CombatManager::set_in_combat_with(A, B)` crea ref en ambas direcciones; `is_in_combat_with(B)` true en A y viceversa.
- [ ] Test: `EndCombat()` borra ref de ambos lados; cero leaks.
- [ ] Test: `PvPCombatReference` timer expira a los 5000ms (`refresh_timer` lo resetea a 5000).
- [ ] Test: `SuppressFor(A)` hace que `A.has_combat()` ignore esa ref; `B.has_combat()` aún la cuenta.
- [ ] Test: `ThreatManager::add_threat(target=B, amount=100)` + `add_threat(C, 50)` → `get_current_victim() == B`.
- [ ] Test: `ScaleThreat(B, 0.0)` + re-eval → `get_current_victim() == C`.
- [ ] Test: `taunt_update` con aura sobre C → `current_victim == C` aunque B tenga más threat.
- [ ] Test: `fixate_target(B)` → siempre devuelve B mientras fixate activo, ignora threat más alto.
- [ ] Test: `online_state` flow — `evaluate_suppressed(true)` con immunity → ONLINE → SUPPRESSED → vuelve a ONLINE al expirar.
- [ ] Test: `add_threat` crea automáticamente CombatReference si no existía (regla "threat ⇒ combat").
- [ ] Test: `EndCombat` propaga a ThreatManager (limpia todas las refs entre par).
- [ ] Test: `roll_melee_outcome(player_lv70, mob_lv73)` distribución sobre 100k samples coincide con tablas C++ ±0.5%.
- [ ] Test: `calc_armor_reduced_damage(level=70, armor=5000, damage=1000)` matches C++ exact value.
- [ ] Test: `calc_spell_resisted_damage(Fire, target_resist=150, damage=1000, level=70)` → mitigated dentro del rango esperado (dist 25/50/75/100%).
- [ ] Test: school binary resist — víctima inmune (resist ≥ infinite) anula 100% damage en `Holy`.
- [ ] Test: `attacker_state_update` con outcome=Crit → damage = 2× normal; hit_info contiene `CRITICAL_HIT` flag.
- [ ] Test: `attacker_state_update` con outcome=Block → damage reducido por block_value; hit_info contiene `BLOCK`.
- [ ] Test: `kill(victim)` dispara `JustDied` AI hook + party kill log + XP grant (player path).
- [ ] Test: pet damage al target → owner combat ref creada + threat añadido al owner-target list.
- [ ] Test: SMSG_ATTACKER_STATE_UPDATE byte-exact contra captura de cliente (round-trip).

---

## 11. Notes / gotchas

- **WotLK 3.4.3 mitigation order es crítico**: Avoid (miss/dodge/parry) → Block reduction (no full block en 3.4.3, sólo amount) → School resist → Armor → Multipliers (crit/glance/crushing) → Absorb → final HP delta. Saltarse un paso o desordenarlos cambia damage por encima del 5% — bug histórico en muchos servers privados.
- **Crit chance per level**: WotLK usa "Defense Skill" del defender vs "Weapon Skill" del attacker. Cada nivel de mob extra = `0.04*level_diff` reducción de crit del attacker. PvE bosses (level 73 vs lv70 player) tienen `-3*0.04 = -12% crit` agresivo. Si copiamos ciegamente `crit_chance = stat_table[level]` sin level-diff penalty, los crits PvE estarán inflados.
- **Glancing blows** sólo aplican a `attack_type=normal` (auto-attack white) contra mobs con `level > attacker.level` y `attack_type` físico. NO aplican a yellow attacks (abilities). Diferencia visible en damage dummies.
- **Crushing blows** sólo de mobs *contra* players cuando level diff ≥ 4. Players nunca dan crushing.
- **Two attack tables**: "white" (auto-attack) tiene 8 outcomes ordenados (miss/dodge/parry/glancing/block/crit/crushing/normal) con penal de skill diff; "yellow" (abilities) tiene tabla separada (miss/dodge/parry/block/crit/normal). Confundirlas hace que abilities tengan glance, lo cual es incorrecto.
- **Off-hand miss penalty**: dual-wield añade flat 19% miss en off-hand swings (no afectado por hit rating excepto la cap). Easy de olvidar.
- **Threat formula**: `threat_modifier_per_school[7]` por aura `MOD_THREAT`; "Defensive Stance" da +30% threat en physical+holy+nature etc. Implementar el mod por mask, no por single school.
- **Taunt no es threat infinito** — fija `online_state=ONLINE` y `taunt_state=TAUNT`; `match_unit_threat_to_highest` se llama para subir el threat amount al top. Si lo implementas como "set huge number" se rompe `MatchUnitThreatToHighestThreat` futuro.
- **`CanHaveThreatList` filter**: solo Creatures (no Pets, no Players, no Vehicles, no GM-mode) tienen threat list. PVP cobra a `CombatManager` pero no `ThreatManager`.
- **PvP combat timer 5s** — refresh on every `add_threat`, `damage`, ofensive spell. Si dejas de "tap" 5s, sales de combate (raid wipe detection se basa en esto).
- **Suppress flags**: usados por vanish (rogue), feign death (hunter), launched-but-not-landed missiles. Marca el side como suppressed; combat ref existe pero no cuenta para `is_in_combat()`. Se vuelve a "active" si el suppressed lado actúa.
- **CombatReference auto-cleanup**: cuando un Unit se desplaza fuera de `EndCombatBeyondRange` (típicamente 25-50 yards leash), TODAS sus refs PvE se limpian → evade. PvP refs ignoran range (timer-based only).
- **`ThreatReference` heap invalidation**: Cualquier add_threat / scale_threat / online_state change requiere re-heapify. Si iteras `GetSortedThreatList()` y modificas el threat, **iterators invalidan** — en Rust esto será un borrow-check fight; usa `GetModifiableThreatList()` que copia.
- **`ProcDamageAndSpell`** post-hit: hay 30+ procs (auras procean, items, talents). Ojo a ordenar attacker procs antes que victim procs (importante para reflect).
- **Power gain on damage**: warrior gana rage on giving damage AND on taking damage. Otro recurso (energy/runic-power) tiene reglas distintas. Hardcodear sólo para warrior te lo va a estallar al añadir DK.
- **`Unit::Kill`** dispara cascade: `JustDied` AI → loot generation → `RewardPlayerAndGroupAtKill` (XP + reputation + quest credit + battleground score) → durability damage → BG honorable kill counter → corpse spawn. El orden importa: lootgen antes de despawn.
- **Resurrection vs combat state**: revivir a un player NO le saca de combat automáticamente en C++ (el combat manager solo limpia si los refs expiran). Algunas implementaciones erróneas lo hacen.
- **3.4.3 specific**: `MaelstromWeapon` proc, `Vigilance` (transferencia de threat 100% al guardian), `Tricks of the Trade` (transfer 100% threat por 6s) — todos pasan por el redirect system. Cubrirlos en tests.
- **`Unit::AttackerStateUpdate`** se llama desde el game loop CADA vez que `weapon_attack_timer` cae a 0; el handler de `CMSG_ATTACK_SWING` solo cambia el target, no inicia el swing inmediatamente.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CombatManager` (per-Unit) | `struct CombatManager { owner_guid, pve_refs: HashMap<ObjectGuid, Box<CombatReference>>, pvp_refs: HashMap<…> }` | Composición en `Unit` (no herencia) |
| `struct CombatReference` | `struct CombatReference { first: ObjectGuid, second: ObjectGuid, is_pvp: bool, suppress_first: bool, suppress_second: bool }` | Box-owned por CombatManager; refs cruzados por GUID, no pointer |
| `struct PvPCombatReference` | `struct PvPCombatReference { base: CombatReference, timer_ms: u32 }` | composición |
| `class ThreatManager` (per-Unit) | `struct ThreatManager { owner_guid, refs: HashMap<ObjectGuid, ThreatReference>, heap: BinaryHeap<HeapEntry>, threatened_by_me: HashSet<ObjectGuid>, current_victim: Option<ObjectGuid>, fixate: Option<ObjectGuid>, redirects: HashMap<u32, …> }` | Heap entries son `(amount_neg, online_state, taunt_state, guid)` |
| `class ThreatReference` | `struct ThreatReference { owner, victim, amount: f32, online_state: OnlineState, taunt_state: TauntState, temp_modifier: i32 }` | — |
| `enum class OnlineState` | `enum OnlineState { Online, Suppressed, Offline }` | — |
| `enum class TauntState` | `enum TauntState { Detaunt, None, Taunt }` | Orden importa para tie-break |
| `MeleeHitOutcome` | `enum MeleeHitOutcome { Evade, Miss, Dodge, Parry, Block, Glancing, Crushing, Crit, Normal }` | — |
| `WeaponAttackType` | `enum WeaponAttackType { Base, Off, Ranged }` | — |
| `HitInfo` (bitfield) | `bitflags! struct HitInfo: u32` | bits hex idénticos a C++ |
| `SpellSchool` / `SpellSchoolMask` | `#[repr(u8)] enum SpellSchool` + `bitflags! struct SpellSchoolMask: u8` | — |
| `CalcDamageInfo` | `struct CalcDamageInfo { … }` | POD-like |
| `Unit::AttackerStateUpdate` | `fn attacker_state_update(&mut self, victim: &mut Unit, attack_type: WeaponAttackType, extra: bool)` | — |
| `Unit::RollMeleeOutcomeAgainst` | `fn roll_melee_outcome(attacker: &Unit, victim: &Unit, attack_type: WeaponAttackType) -> MeleeHitOutcome` | Stateless puro |
| `Unit::CalcArmorReducedDamage` | `fn calc_armor_reduced_damage(attacker_level: u32, victim_armor: u32, damage: u32, spell: Option<&SpellInfo>) -> u32` | — |
| `Unit::DealDamage` | `fn deal_damage(&mut self, victim: &mut Unit, damage: u32, clean: CleanDamage, dmg_type: DamageType, school_mask: SpellSchoolMask, durability_loss: bool) -> u32` | Devuelve damage real aplicado |
| `Unit::Kill` | `fn kill(&mut self, victim: &mut Unit, durability_loss: bool)` | dispara hooks AI/Loot/XP |
| `std::unordered_map<ObjectGuid, CombatReference*>` | `HashMap<ObjectGuid, Box<CombatReference>>` | sin pointers crudos |
| Heap with `CompareThreatLessThan` | `BinaryHeap<HeapEntry>` con custom Ord | min-heap inverso = max-heap |
| `void Update(uint32 tdiff)` | `fn update(&mut self, diff_ms: u32)` | — |
| `WorldPackets::Combat::AttackSwing` | `struct AttackSwing { victim: ObjectGuid }` (existe) | Ya en `wow-packet` |
| `WorldPackets::Combat::AttackerStateUpdate` | `struct AttackerStateUpdate { … }` (parcial) | Falta poblar HitInfo correctamente |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Combat/` (CombatManager.{h,cpp}, ThreatManager.{h,cpp}) and the damage pipeline embedded in `Entities/Unit/Unit.cpp` (`Unit::AttackerStateUpdate`, `Unit::CalculateMeleeDamage`, `Unit::RollMeleeOutcomeAgainst`, `Unit::DealMeleeDamage`, `Unit::DealDamage`, `Unit::Kill`) against the Rust workspace at `/home/server/rustycore/crates/`.

**Empty-crate finding — CONFIRMED.** `crates/wow-combat/src/lib.rs` measures **exactly 0 lines** (verified via `wc -l`). The `Cargo.toml` exists, the crate is in the workspace member list, but **no code at all** is shipped: no `CombatManager`, no `ThreatManager`, no `CombatReference`, no `PvPCombatReference`, no `MeleeHitOutcome`, no `WeaponAttackType`, no `HitInfo`, no `CalcDamageInfo`, no `SpellSchool`/`SpellSchoolMask`, no `CompareThreatLessThan`. All combat logic that exists in the workspace lives in two side files:
- `crates/wow-packet/src/packets/combat.rs` (~190 lines: AttackSwing/AttackStop/SetSheathed CMSG, AttackStart/SAttackStop/AttackerStateUpdate/AttackSwingError SMSG — wire shapes only, no logic).
- `crates/wow-world/src/handlers/combat.rs` (152 lines: trivial `handle_attack_swing`/`handle_attack_stop`/`handle_set_sheathed` writing directly into `WorldSession.combat_target: Option<ObjectGuid>`).

**Damage pipeline mapping (C++ → Rust).** The C++ chain is `Player::Attack → swing_timer → Unit::AttackerStateUpdate → CalculateMeleeDamage → RollMeleeOutcomeAgainst → MeleeDamageBonusDone/Taken → CalcArmorReducedDamage → DealMeleeDamage → DealDamage → Kill → ProcDamageAndSpell`. **None of those nine functions exist in Rust.** The closest analog is a flat HP decrement inside the legacy `wow-ai::CreatureAI::take_damage` (verified at `crates/wow-ai/src/lib.rs`), which subtracts an unmitigated random damage roll with `subsec_nanos` as RNG, ignoring armor, school, resistance, hit/dodge/parry/block, crit, glancing, crushing, off-hand penalty, and weapon swing time. There is no `SchoolMask`, no `DamageInfo`, no `HitInfo` flag bitfield — the `SMSG_ATTACKER_STATE_UPDATE` writer in `wow-packet` zero-fills its hit-info word.

**ThreatManager / CombatManager.** Both are completely absent. The legacy `WorldSession.combat_target: Option<ObjectGuid>` (single target per player) and `WorldSession.in_combat: bool` (single boolean per session) replace what C++ implements as a per-Unit `CombatManager` with two `unordered_map<ObjectGuid, CombatReference*>` (PvE + PvP) plus a per-Creature `ThreatManager` heap with online/suppressed/offline state, taunt/detaunt tie-break, fixate, redirect (Misdirection / Tricks of the Trade), `MatchUnitThreatToHighestThreat`, and per-tick `Update` for PvP timers and victim re-evaluation. `CanHaveThreatList` filtering does not exist; `EndCombatBeyondRange` (leashing) does not exist; `RevalidateCombat` does not exist; the 5-second PvP timer does not exist. `SMSG_THREAT_UPDATE`/`THREAT_REMOVE`/`THREAT_CLEAR`/`HIGHEST_THREAT_UPDATE` are not emitted, so client threat UI is dark.

**Outcome roll & mitigation order.** C++ enforces a strict `Avoid → Block → Resist → Armor → Multiplier → Absorb → HP delta` order with the well-known WotLK 8-row "white attack table" plus a separate "yellow attack table" for abilities, plus level-diff penalties (the −12% PvE crit hit against +3-level bosses). Rust does **none** of this — every melee swing is a guaranteed normal hit, never miss, never dodge, never parry, never block, never crit, never glance, never crushing.

**Worst divergence.** **The kill pipeline has no rewards.** When `creature.hp ≤ 0` in Rust, the only thing that happens is `creature.is_alive = false` plus a `SMSG_ATTACK_STOP{now_dead: true}`. There is **no XP grant, no quest kill credit, no party kill log (`SMSG_PARTY_KILL_LOG`), no reputation gain, no loot generation hook, no durability damage on player death, and no AI hooks (`JustDied`, `KilledUnit`, `JustExitedCombat`)**. C++ chains all of those through `Unit::Kill → Player::RewardPlayerAndGroupAtKill → LootMgr::FillLoot`. In RustyCore today, killing a quest objective mob produces zero progression: no quest credit is granted, the corpse is not lootable, and no XP is awarded. This is a content-blocking divergence — every PvE gameplay loop (level, quest, gear) is broken until §9 tasks #COMBAT.15–#COMBAT.16 (`Unit::deal_damage` + `Unit::kill`) land, which themselves require the full `wow-combat` crate to exist first.
