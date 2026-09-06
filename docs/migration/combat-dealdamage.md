# Migration: Combat — DealDamage pipeline

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Entities/Unit/Unit.cpp` (damage subset: lines ~746-1800, 6592-7800, 10457+) + `src/server/game/Entities/Unit/Unit.h` (lines ~880-1620 — DamageInfo / CalcDamageInfo / CleanDamage / Damage* prototypes)
> **Rust target crate(s):** `crates/wow-combat/` (empty — see §13), `crates/wow-spell/` (school mask + immunity hooks), `crates/wow-world/src/handlers/combat.rs` (entry-point bridge)
> **Layer:** L5 sub-module of `combat.md`. Depends on `entities-unit.md` (L4 — Unit stats, auras), `spells-effects.md` (L5 — SpellInfo, SchoolMask, AuraEffect), `loot.md` (L6 — kill drops), `quests.md` (L6 — kill credit). Sibling of `combat-threat.md`, `combat-manager.md`.
> **Status:** 🔧 broken (rewrite needed) — pipeline does not exist. Single line of code: `creature.hp = creature.hp.saturating_sub(damage)` in `wow-ai`.
> **Audited vs C++:** ✅ complete 2026-05-01 (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Universe-touching damage pipeline. Every HP delta on every Unit — melee swing, spell direct hit, periodic DoT tick, environmental fall — flows through `Unit::DealDamage`. The function performs school-mask resolution, armor reduction, absorb-shield depletion (multiple shields stack), school resistance bucketing, immune checks (`SchoolImmunityList` / `MechanicImmunityList`), spell reflection (Spell Reflection, Magic Reflection), modifier application (Done/Taken bonuses, taken-percent auras, school-power coefficient), and kill detection that funnels into `Unit::Kill` (XP, quest credit, loot, AI hooks). Companion `MeleeDamageBonus*` and `SpellDamageBonus*` apply attacker-side and victim-side multiplicative/additive modifiers before the damage reaches `DealDamage`. `ProcDamageAndSpell` runs after the HP delta to fire on-hit and on-damage proc auras for both attacker and victim.

---

## 2. C++ canonical files

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (range) | Purpose |
|---|---|---|
| `src/server/game/Entities/Unit/Unit.cpp` | 746-770 | `Unit::DealDamageMods` (static): apply DealDamageMods auras + sanity clamp |
| `src/server/game/Entities/Unit/Unit.cpp` | 767-1018 | `Unit::DealDamage` (static): the master function; routes to Kill at end |
| `src/server/game/Entities/Unit/Unit.cpp` | 1100-1340 | `Unit::CalculateMeleeDamage` (uses CalcArmorReducedDamage + CalcAbsorbResist) |
| `src/server/game/Entities/Unit/Unit.cpp` | 1623-1730 | `Unit::CalcArmorReducedDamage` (static): WotLK armor formula |
| `src/server/game/Entities/Unit/Unit.cpp` | 1789-2020 | `Unit::CalcAbsorbResist` (static): school resist + absorb shield depletion |
| `src/server/game/Entities/Unit/Unit.cpp` | 6592-6770 | `Unit::SpellDamageBonusDone`: caster spell-power, school coefficient, talent bonuses |
| `src/server/game/Entities/Unit/Unit.cpp` | 6775-7100 | `Unit::SpellDamageBonusTaken`: victim taken-school auras, vulnerability multipliers |
| `src/server/game/Entities/Unit/Unit.cpp` | 7558-7665 | `Unit::MeleeDamageBonusDone`: attacker AP, weapon damage scaling, MOD_DAMAGE_DONE |
| `src/server/game/Entities/Unit/Unit.cpp` | 7670-7790 | `Unit::MeleeDamageBonusTaken`: victim physical taken auras, MOD_DAMAGE_TAKEN |
| `src/server/game/Entities/Unit/Unit.cpp` | 10457-10640 | `Unit::Kill` (static): the death cascade — JustDied, loot, XP, quest, durability |
| `src/server/game/Entities/Unit/Unit.cpp` | (scattered) | `ProcDamageAndSpell` invocations from CalcDamageInfo + DealDamage paths |
| `src/server/game/Entities/Unit/Unit.h` | 880-980 | `DamageInfo` class (wrapper around damage/absorb/resist/blocked) |
| `src/server/game/Entities/Unit/Unit.h` | 200-260 | `CalcDamageInfo` struct (per-swing computed result) |
| `src/server/game/Entities/Unit/Unit.h` | 270-300 | `CleanDamage` struct (raw damage minus mitigation) |
| `src/server/game/Entities/Unit/Unit.h` | 1564-1610 | Damage prototype declarations |
| `src/server/game/Entities/Unit/Unit.h` | 1700-1760 | `SchoolImmunityList`, `MechanicImmunityList` typedefs + accessors |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | (HandleAuraModSchoolImmunity etc.) | Aura handlers that populate the immunity lists |
| `src/server/game/Spells/Spell.cpp` | (EffectSchoolDMG, EffectWeaponDmg) | Spell side feeders into DealDamage |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `DamageInfo` | class | Wrapper used by `CalcAbsorbResist`: holds attacker, victim, damage, school mask, hit type, absorb, resist, blocked, damage type |
| `CalcDamageInfo` | struct | Computed melee outcome: `Damages[2]` (main + off split for dual school), HitOutCome, HitInfo flags, absorb/resist/blocked per side, ProcAttacker/ProcVictim/ProcEx masks |
| `CleanDamage` | struct | Raw damage pre-mitigation + absorbed + mitigated split, used to back-out when computing rage gain |
| `SpellNonMeleeDamage` | struct | Output for `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` (spell direct damage shape) |
| `DamageEffectType` | enum | `DIRECT_DAMAGE` / `SPELL_DIRECT_DAMAGE` / `DOT` / `HEAL` / `NODAMAGE` / `SELF_DAMAGE` |
| `SpellSchool` | enum | `SPELL_SCHOOL_NORMAL=0` / `HOLY=1` / `FIRE=2` / `NATURE=3` / `FROST=4` / `SHADOW=5` / `ARCANE=6` |
| `SpellSchoolMask` | bitfield | `1 << SpellSchool`; physical = `0x01`, magic = `0x7E`, all = `0x7F` |
| `Mechanics` | enum | 32 mechanic flags (charm, disorient, fear, root, silence, sleep, snare, stun, etc.) consumed by `MechanicImmunityList` |
| `HitInfo` | bitfield | NORMALSWING / CRITICALHIT / MISS / GLANCING / CRUSHING / ABSORB / RESIST / BLOCK / OFFHAND — written into `SMSG_ATTACKER_STATE_UPDATE` |
| `SchoolImmunityList` | typedef `std::multimap<uint32, SpellInfo const*>` | Per-school immunity list keyed by aura cause; populated by `SPELL_AURA_SCHOOL_IMMUNITY` handlers |
| `MechanicImmunityList` | typedef `std::multimap<Mechanics, SpellInfo const*>` | Per-mechanic immunity list; populated by `SPELL_AURA_MECHANIC_IMMUNITY` |
| `AbsorbAuraList` | `std::vector<AuraEffect*>` | Iterated by `CalcAbsorbResist` to deplete shields in priority order |
| `ProcEventInfo` | class | Carries DamageInfo + spell + flags into `ProcDamageAndSpell` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Unit::DealDamage(attacker, victim, damage, cleanDamage, type, schoolMask, spellProto, durabilityLoss)` (static) | Master entry: applies HP delta, fires Kill on lethal | `IsImmunedToDamage`, `Kill`, `Player::DurabilityLossAll`, `ApplyResilience` |
| `Unit::DealDamageMods(attacker, victim, damage, absorb)` (static) | Apply school-immunity check + `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` clamp before absorb pipeline | `IsImmunedToDamage`, aura iter |
| `Unit::CalcAbsorbResist(damageInfo, spell)` (static) | School resistance buckets (25/50/75/100%) + iterate absorb shields in priority order, deplete amount, fire `SPELL_AURA_SCHOOL_ABSORB` handlers, support reflect (`SPELL_AURA_REFLECT_SPELLS_SCHOOL`) | `GetSchoolDamageReductionPercent`, `AbsorbAuraList` iter |
| `Unit::CalcArmorReducedDamage(attacker, victim, damage, spell, attackType, attackerLevel)` (static) | Armor mitigation: `armor / (armor + 467.5*level - 22167.5)` for WotLK level≤80 with `gtArmorMitigationByLvl` DBC fudge | `GetArmor`, `gtArmorMitigationByLvlStore` |
| `Unit::MeleeDamageBonusDone(victim, damage, attType, damagetype, spellProto, mechanic, schoolMask, spell, aurEff)` | Attacker-side: `MOD_DAMAGE_DONE`, `MOD_DAMAGE_PERCENT_DONE`, weapon-school-power coefficient, AP scaling | aura iter, GetAttackPower |
| `Unit::MeleeDamageBonusTaken(attacker, damage, attType, damagetype, spellProto, schoolMask)` | Victim-side: `MOD_DAMAGE_TAKEN`, `MOD_DAMAGE_PERCENT_TAKEN`, school-mask-specific taken auras | aura iter |
| `Unit::SpellDamageBonusDone(victim, spellProto, pdamage, damagetype, effectInfo, stack, spell, aurEff)` | Caster spell-power × coefficient + DoT bonus + talent multipliers | `GetSpellDamage`, `SpellInfo::CalcCastTime`, aura iter |
| `Unit::SpellDamageBonusTaken(caster, spellProto, pdamage, damagetype)` | Victim taken-school multipliers, vulnerability auras (e.g. Curse of Elements) | aura iter |
| `Unit::IsImmunedToDamage(SpellSchoolMask)` | True if any entry in `SchoolImmunityList` covers the mask | mask AND |
| `Unit::IsImmunedToSpell(spellInfo)` / `IsImmunedToSpellEffect` | Spell-level + per-effect immunity check using both lists | both lists |
| `Unit::ApplySpellMod` | Caster talent/aura modifiers feed Spell*BonusDone | — |
| `Unit::ProcDamageAndSpell(actor, target, procAttacker, procVictim, procEx, amount, attackType, spellProto, damageInfo, healInfo)` | Iterate proc-auras on attacker and victim; fire `AuraEffect::HandleProc` for each | proc table iter |
| `Unit::Kill(attacker, victim, durabilityLoss, skipSettingDeathState)` (static) | Death cascade: AI `JustDied` / `KilledUnit`, `Loot::FillLoot`, `Player::RewardPlayerAndGroupAtKill` (XP + faction + quest credit + BG score), `DurabilityLossAll`, `SetDeathState`, corpse spawn, achievement hook, party kill log | massive — see §11 |

---

## 5. Module dependencies

**Depends on:**
- `entities-unit.md` — `Unit::GetArmor`, `GetMaxHealth`, `ModifyHealth`, `IsAlive`, `SetDeathState`, attribute/aura caches.
- `spells-effects.md` — `SpellInfo` (school, mechanic, attributes), `AuraEffect::GetAmount`, proc handlers, `SPELL_AURA_SCHOOL_ABSORB`, `SPELL_AURA_REFLECT_SPELLS_SCHOOL`, `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN`, `SPELL_AURA_MOD_DAMAGE_TAKEN`.
- `combat-threat.md` — every applied damage calls `ThreatManager::AddThreat(attacker, computed_threat)`.
- `combat-manager.md` — every applied damage calls `CombatManager::SetInCombatWith(attacker, victim)`.
- `loot.md` — `Unit::Kill` triggers `LootMgr::FillLoot` for the corpse.
- `quests.md` — `Unit::Kill` triggers `Player::KilledMonsterCredit` for quest objectives.
- `pets.md` — pet damage attribution: pet hits credit owner for threat + combat ref.
- `ai.md` — `Unit::Kill` fires `CreatureAI::JustDied`, `CreatureAI::KilledUnit`, `CreatureAI::EnterEvadeMode` upon victim death.
- `entities.md` (Player) — durability-loss-on-death, party kill log, XP-share group reward.
- `reputation.md` — kill rewards faction rep via `Player::RewardOnKill`.
- `achievements.md` — `OnKillCreatureType` hook fires from `Unit::Kill`.
- `battlegrounds.md` — `BattlegroundScore` updated by `Unit::Kill` honor flow.
- `datastores.md` — DB2: `gtArmorMitigationByLvl`, `gtNPCManaCostScaler`, `XpGainBaseMap`, `gtChanceToMeleeCrit`.

**Depended on by:**
- `combat.md` — parent module summary.
- All damage sources: melee swing (`Unit::AttackerStateUpdate`), spell direct (`Spell::EffectSchoolDMG`), spell weapon (`Spell::EffectWeaponDmg`), DoT tick (`Aura::PeriodicTick`), environmental (`Player::EnvironmentalDamage`), trap (`GameObject::Use`).
- `combat-manager.md` — combat entry depends on a successful damage event.
- `pvp.md` — honorable kill counter feeds from `Unit::Kill`.

---

## 6. SQL / DB queries (if any)

`DealDamage` itself emits no SQL. Inputs are read at unit-load time; outputs flow through systems that own their own DB writes. Relevant inputs:

| Statement / Source | Purpose | DB |
|---|---|---|
| `creature_template.armor_mod` | Armor multiplier per creature | world |
| `creature_template.resistance1..6` | Per-school resistance (Holy/Fire/Nature/Frost/Shadow/Arcane) | world |
| `creature_template_resistance` | Per-difficulty resistance overrides | world |
| `creature_template.dmgschool` | Default attack school for the creature | world |
| `spell_proc` | Server-side proc-event mappings | world |
| `spell_dbc.SchoolMask` (DB2 SpellEffect) | Spell school | hotfixes |
| `character_durability` (write side) | `Player::DurabilityLossAll` writes durability deltas on death | characters |
| `character_quest_status` (write side) | `Player::KilledMonsterCredit` updates objective counts | characters |
| `character_reputation` (write side) | `Player::RewardOnKill` updates faction standings | characters |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `gtArmorMitigationByLvlStore` | Per-level armor cap (3.4.3 fudge above level 70) | `CalcArmorReducedDamage` |
| `gtNPCManaCostScalerStore` | NPC spell mana cost scaling | (indirect — Spell-side) |
| `XpGainBaseMap` (3.4.3 table) | Base XP per (mob_level, player_level) | `Player::RewardPlayerAndGroupAtKill` |
| `SpellStore` / `SpellEffectStore` | School mask, mechanic, coefficients | `SpellDamageBonus*` |
| `gtChanceToMeleeCritStore` | (indirect — used by RollMelee, but absorb path checks crit-mitigation auras) | aura code |

---

## 7. Wire-protocol packets (if any)

`DealDamage` itself doesn't write packets — its callers do. The pipeline produces:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_SPELL_NON_MELEE_DAMAGE_LOG` | S→C | `Unit::SendSpellNonMeleeDamageLog` (spell direct path) |
| `SMSG_PERIODIC_AURA_LOG` | S→C | `Aura::PeriodicTick` (DoT path) |
| `SMSG_SPELL_HEAL_LOG` | S→C | `Unit::SendHealSpellLog` (heal path, mirror of damage) |
| `SMSG_SPELL_ENERGIZE_LOG` | S→C | mana/rage/energy gain log |
| `SMSG_ATTACKER_STATE_UPDATE` | S→C | `Unit::SendAttackStateUpdate` (melee path — see `combat.md` §7) |
| `SMSG_ENVIRONMENTAL_DAMAGE_LOG` | S→C | `Player::EnvironmentalDamage` |
| `SMSG_PARTY_KILL_LOG` | S→C | `Unit::Kill` group XP-share log |
| `SMSG_DURABILITY_DAMAGE_DEATH` | S→C | `Player::DurabilityLossAll` from `Unit::Kill` |
| `SMSG_COMBAT_LOG_MULTIPLE` | S→C | (3.4.3) batched combat log |
| `SMSG_PROC_RESIST` | S→C | `ProcDamageAndSpell` resisted proc notice |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-combat/src/lib.rs` — **0 lines** (empty crate). No `DealDamage`, no `CalcAbsorbResist`, no `CalcArmorReducedDamage`, no `MeleeDamageBonus*`, no `SpellDamageBonus*`, no `SchoolImmunityList`, no `MechanicImmunityList`, no `ProcDamageAndSpell`.
- `crates/wow-ai/src/lib.rs` — `CreatureAI::take_damage` does `self.hp = self.hp.saturating_sub(damage)` with no mitigation.
- `crates/wow-spell/src/lib.rs` — has a stub `SpellSchool` enum but no `SpellSchoolMask` bitflag and no immunity list types.
- `crates/wow-world/src/handlers/combat.rs` — does not invoke any damage pipeline; only manages `WorldSession::combat_target`.

**What's implemented:**
- HP subtraction (`hp -= damage`) with saturating clamp at zero.
- "is_alive" boolean flip when `hp == 0`.
- `SMSG_ATTACK_STOP{now_dead: true}` send when creature dies.

**What's missing vs C++:**
- **`Unit::DealDamage` (the master function)** — does not exist. There is no static damage entry: every caller writes HP directly.
- **`Unit::DealDamageMods`** — not present; pre-absorb school-immunity check skipped.
- **`CalcAbsorbResist`** — entirely absent. Absorb shields (Power Word: Shield, Sacrifice, Mana Shield, Frost Ward, Ice Barrier, Divine Aegis, Spell Reflection) all do nothing. School resistance bucketing (25/50/75/100%) does not run.
- **`CalcArmorReducedDamage`** — armor mitigation skipped; a 5000-armor target takes the same damage as a 0-armor target. WotLK formula `armor / (armor + 467.5*L - 22167.5)` not implemented; `gtArmorMitigationByLvl` DBC unused.
- **`MeleeDamageBonusDone` / `MeleeDamageBonusTaken`** — attacker AP scaling, weapon-power coefficient, `MOD_DAMAGE_DONE_PCT`, `MOD_DAMAGE_TAKEN_PCT` all unapplied.
- **`SpellDamageBonusDone` / `SpellDamageBonusTaken`** — spell power coefficient, talent multipliers, vulnerability auras (Curse of Elements +13%, Earth and Moon +13%) all unapplied.
- **`SchoolImmunityList`** — no map structure exists. Paladin Bubble (Divine Shield = full immunity) does nothing; Ice Block (Mage) does nothing; Anti-Magic Shell (DK) does nothing.
- **`MechanicImmunityList`** — no map structure exists. PvP trinket (mechanic immune to stun/silence/fear/root) does nothing; Berserker Rage (immune to fear/sap/incapacitate) does nothing.
- **`ProcDamageAndSpell`** — proc system absent. Items that proc on hit (Mirror of Truth, Berserking enchant), talents that proc (Sword Spec, Hand of Reckoning), set bonuses, all silent.
- **Reflect** — Spell Reflection (warrior) + Magic Reflection (priest aura) unimplemented.
- **`Unit::Kill`** — does not exist as a function. Death is `creature.is_alive = false` and an `SMSG_ATTACK_STOP`. No XP, no quest credit, no loot, no rep, no BG score, no `JustDied`/`KilledUnit` AI hooks, no party kill log, no durability damage, no corpse spawn, no achievement hook.
- **Damage type routing** — `DamageEffectType` enum absent; cannot distinguish `DIRECT_DAMAGE` vs `DOT` vs `SELF_DAMAGE` (relevant: SELF_DAMAGE bypasses absorb shields).
- **`CleanDamage` / `DamageInfo`** — structs absent; cannot back-out absorbed-vs-resisted-vs-blocked for rage formula.
- **Resilience** — `Unit::ApplyResilience` (PvP damage reduction) absent.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `creature.hp.saturating_sub(damage)` uses `u32` — C++ uses `int32` for damage and supports negative-damage = heal in some paths. Rust shape needs revisit.
- No school routing means a lava-strike physical mob hits paladin under Divine Shield — already off-spec but blockable once `IsImmunedToDamage` exists.
- `is_alive` flag flipped without cascading to AI/Loot/Quest means the dead body is invisible to those subsystems even after they exist.
- Death state is binary (`alive | not`) — C++ has `Alive / JustDied / Corpse / Dead / DeadFalling` for ghost mechanics and resurrection windows.

**Tests existing:**
- 0 tests for damage pipeline (no pipeline exists).

---

## 9. Migration sub-tasks

Numbered for `MIGRATION_ROADMAP.md` §5 reference.
Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#COMBAT-DMG.1** Define `enum SpellSchool` + `bitflags SpellSchoolMask: u8` matching C++ exactly (Normal=1<<0, Holy=1<<1, ..., Arcane=1<<6). (L)
- [ ] **#COMBAT-DMG.2** Define `enum DamageEffectType { DirectDamage, SpellDirectDamage, Dot, Heal, NoDamage, SelfDamage }`. (L)
- [ ] **#COMBAT-DMG.3** Define `struct CleanDamage { absorbed_damage, mitigated_damage, attack_type, hit_outcome }` and `struct DamageInfo { attacker, victim, damage, school_mask, damage_type, attack_type, absorb, resist, blocked }`. (M)
- [ ] **#COMBAT-DMG.4** Define `bitflags HitInfo: u32` with bits hex-identical to C++ (NormalSwing, CriticalHit, Miss, Glancing, Crushing, Absorb, Resist, Block, Offhand, FullAbsorb, PartialAbsorb, FullResist, PartialResist, FullBlock, etc.). (L)
- [ ] **#COMBAT-DMG.5** Define `SchoolImmunityList = HashMap<u32, Vec<u32>>` (cause-spell-id → list of immune school masks) and `MechanicImmunityList = HashMap<Mechanic, Vec<u32>>`. Add accessors `is_immune_to_damage(mask)`, `is_immune_to_mechanic`, `is_immune_to_spell`. (M)
- [ ] **#COMBAT-DMG.6** `fn calc_armor_reduced_damage(attacker_level: u8, victim_armor: u32, damage: u32, spell: Option<&SpellInfo>) -> u32` — implement WotLK formula with `gtArmorMitigationByLvl` DBC lookup. (M)
- [ ] **#COMBAT-DMG.7** `fn calc_school_resist_damage(school_mask, target_resist: u32, attacker_level: u8, damage: u32, rng: &mut Rng) -> (mitigated_damage, resisted_amount)` — 25/50/75/100% bucket roll. (M)
- [ ] **#COMBAT-DMG.8** `fn calc_absorb_resist(damage_info: &mut DamageInfo, spell: Option<&SpellInfo>)` — iterate absorb auras in priority order, deplete `aur_eff.amount`, fire reflect auras, call resist routine. (XL — many aura types)
- [ ] **#COMBAT-DMG.9** `fn melee_damage_bonus_done(attacker, victim, damage, att_type, damage_type, spell, mechanic, school_mask, aur_eff) -> i32` — apply `MOD_DAMAGE_DONE`, `MOD_DAMAGE_PERCENT_DONE`, AP scaling, weapon-school-power. (H)
- [ ] **#COMBAT-DMG.10** `fn melee_damage_bonus_taken(attacker, victim, damage, att_type, damage_type, spell, school_mask) -> i32` — apply `MOD_DAMAGE_TAKEN`, `MOD_DAMAGE_PERCENT_TAKEN`, school-mask-specific taken auras. (M)
- [ ] **#COMBAT-DMG.11** `fn spell_damage_bonus_done(caster, victim, spell_info, p_damage, damage_type, effect_info, stack, spell, aur_eff) -> i32` — spell power × coefficient, DoT bonus, talent multipliers. (H)
- [ ] **#COMBAT-DMG.12** `fn spell_damage_bonus_taken(caster, victim, spell_info, p_damage, damage_type) -> i32` — vulnerability auras (Curse of Elements, Earth and Moon, Misery), school taken multipliers. (M)
- [ ] **#COMBAT-DMG.13** `fn deal_damage_mods(attacker, victim, damage: &mut u32, absorb: &mut u32)` — pre-absorb school-immunity clamp. (L)
- [ ] **#COMBAT-DMG.14** `fn deal_damage(attacker, victim, damage, clean_damage, damage_type, school_mask, spell_proto, durability_loss) -> u32` — master function: apply mods, route to Kill on lethal, return actual damage applied. (XL — central piece)
- [ ] **#COMBAT-DMG.15** `fn kill(attacker, victim, durability_loss, skip_setting_death_state)` — full cascade: `JustDied`/`KilledUnit` AI hooks → `LootMgr::fill_loot` → `Player::reward_player_and_group_at_kill` (XP, faction, quest credit, BG score) → `Player::durability_loss_all` → `SetDeathState` → corpse spawn → achievement hook → SMSG_PARTY_KILL_LOG. (XL)
- [ ] **#COMBAT-DMG.16** `fn proc_damage_and_spell(actor, target, proc_attacker, proc_victim, proc_ex, amount, attack_type, spell_proto, damage_info, heal_info)` — iterate proc auras, fire `AuraEffect::handle_proc`. (XL — depends on aura system)
- [ ] **#COMBAT-DMG.17** Spell Reflection: `SPELL_AURA_REFLECT_SPELLS_SCHOOL` aura handler integrates with `calc_absorb_resist` to swap attacker/victim and re-cast at original caster. (M)
- [ ] **#COMBAT-DMG.18** Resilience: `fn apply_resilience(victim, damage: &mut u32, attack_type, is_crit, damage_type)` — PvP damage reduction by victim's combat rating (Resilience). (M)
- [ ] **#COMBAT-DMG.19** Pet damage attribution: pet hits with `is_pet=true` route threat + combat-ref creation through owner. (M)
- [ ] **#COMBAT-DMG.20** SMSG_SPELL_NON_MELEE_DAMAGE_LOG writer + SMSG_ENVIRONMENTAL_DAMAGE_LOG writer + SMSG_DURABILITY_DAMAGE_DEATH writer + SMSG_PARTY_KILL_LOG writer. (M)

---

## 10. Regression tests to write

- [ ] Test: `calc_armor_reduced_damage(level=70, armor=5000, damage=1000)` matches C++ byte-exact (≈70% of 1000).
- [ ] Test: `calc_armor_reduced_damage(level=80, armor=15000, damage=1000)` returns expected mitigation (~50%).
- [ ] Test: `calc_school_resist_damage(Fire, target_resist=150, level=70, damage=1000)` distribution over 100k samples matches C++ 25/50/75/100% bucket distribution ±0.5%.
- [ ] Test: `is_immune_to_damage(SchoolMask::HOLY)` true with Divine Shield aura applied; deal_damage returns 0.
- [ ] Test: `is_immune_to_mechanic(Mechanic::Stun)` true with PvP trinket effect; subsequent stun aura rejected.
- [ ] Test: `calc_absorb_resist` with PW:Shield (1000 absorb) on victim taking 800 damage → damage=0, absorb=800, shield_remaining=200.
- [ ] Test: `calc_absorb_resist` with PW:Shield (1000) + Divine Aegis (300) stacked, 1500 damage → shields fully consumed, 200 leaks through.
- [ ] Test: Spell Reflection on warrior absorbs Pyroblast and re-casts at mage; mage takes the damage instead.
- [ ] Test: `melee_damage_bonus_done` with attacker AP=1000, white-attack normalized weapon scales correctly.
- [ ] Test: `spell_damage_bonus_taken` with Curse of Elements debuff multiplies arcane/fire/frost/shadow/holy damage by 1.13.
- [ ] Test: `deal_damage` lethal blow triggers `kill` cascade in correct order: `JustDied` AI → loot → XP → quest credit → BG score → durability → corpse → SMSG_PARTY_KILL_LOG.
- [ ] Test: `kill` quest credit: player kills mob with `quest_id=12345 / objective=0` → character_quest_status row updated by 1.
- [ ] Test: `kill` XP grant: lv70 player kills lv70 mob → `Player.xp` increments by `XpGainBaseMap[70][70]`.
- [ ] Test: `kill` party kill log: 3-player party gets SMSG_PARTY_KILL_LOG on each member's session.
- [ ] Test: pet damage attribution — pet hits target → owner CombatReference created + threat added to owner→target list (zero on pet→target).
- [ ] Test: `SELF_DAMAGE` damage type bypasses absorb shields (Hellfire, Bandage interrupt damage).
- [ ] Test: `apply_resilience(victim_with_400_resilience, crit_damage=2000)` reduces by ≈10% (4 resilience = 1% damage reduction at 80).
- [ ] Test: SMSG_ATTACKER_STATE_UPDATE byte-exact round-trip vs captured client packet for {block, partial-absorb, partial-resist} combo.

---

## 11. Notes / gotchas

- **Mitigation order is canonical and load-bearing**: `IsImmunedToDamage` → `Avoid` (miss/dodge/parry — done by `RollMeleeOutcomeAgainst` upstream) → `Block` reduction → `MeleeDamageBonusTaken` (taken-percent) → `Resist` (`CalcAbsorbResist` resist portion) → `Armor` (`CalcArmorReducedDamage`) → `Multiplier` (crit/glance/crushing) → `Absorb` (`CalcAbsorbResist` shield portion) → `Resilience` → `DealDamage` final HP delta. Skip a step or reorder = bug.
- **Absorb shields stack but order matters**: `SPELL_AURA_SCHOOL_ABSORB` auras iterate by stack-order in C++. Ice Barrier on top of Mana Shield on top of Frost Ward = Ice Barrier eats first. Same for PW:Shield + Divine Aegis. Test the order against a real client packet capture.
- **Spell Reflection is a special absorb**: `SPELL_AURA_REFLECT_SPELLS_SCHOOL` triggers within `CalcAbsorbResist`. Returns the spell to the caster *with original damage* — caster does NOT get reflect of their own reflect (no infinite loop).
- **`SELF_DAMAGE` bypasses absorb shields and resilience**: Warlock self-damage (Hellfire) eats the warlock's own shields = wrong. Bandage interrupt damage bypasses shields. The check is `damageType != SELF_DAMAGE`.
- **`MeleeDamageBonusDone` is called for BOTH white and yellow attacks**: don't gate on attack-type. The internal logic checks `spellProto != null` to decide weapon-power coefficient vs flat AP scaling.
- **Crit modifier is applied AFTER `MeleeDamageBonusDone` but BEFORE `MeleeDamageBonusTaken`**: this is why "Mortal Strike" (50% increased crit) and "Defender of Aetheria" (8% reduced crit damage taken) both work correctly — they're at different points in the pipeline.
- **`Unit::Kill` cascade order is critical**: AI `JustDied` runs *before* loot generation (so AI can mutate loot table). XP grant runs *after* loot generation (so quest credit can be checked first). Durability damage is the last visible step (after corpse spawn). Reordering = lost loot or duplicate XP.
- **`Player::RewardPlayerAndGroupAtKill` is the master player-side reward**: it walks the group, computes share-of-XP per member based on level diff, fires `OnKillReputation`, fires `KilledMonsterCredit` for each member, fires achievement hook, fires BG-score update.
- **Resilience formula 3.4.3**: `damage_reduction_pct = combat_rating / RATING_PER_PCT[level]`. At level 80, 95.2 resilience = 1% reduction. Caps at ~100% (theoretical), practical cap ~50%.
- **DoT damage routes through `DealDamage` with `damagetype=DOT`**: but DoTs do NOT go through `MeleeDamageBonusDone` (already applied at cast time when the aura was created; multiplicative re-application is a known historical bug source).
- **`CalcAbsorbResist` resist portion is BINARY for some spells**: `SpellInfo->AttributesEx4 & SPELL_ATTR4_IGNORE_RESISTANCES` skips resist entirely. `AttributesEx4 & SPELL_ATTR4_DAMAGE_DOESNT_BREAK_AURAS` skips proc trigger.
- **Pet damage attribution**: `DealDamage(attacker=pet, victim=target)` increments threat on pet's owner, not the pet. Owner gets the combat ref. Pet does NOT enter combat as a separate target — it's merged with owner combat state.
- **3.4.3 specific**: "Gift of the Wild" (druid party-buff) gives armor → affects `CalcArmorReducedDamage` victim side. "Renewed Hope" (priest 4-set) reduces damage taken 3% — applied in `MeleeDamageBonusTaken` AND `SpellDamageBonusTaken`.
- **`CleanDamage` is needed for rage formula**: warriors gain rage = `damage * 7.5 / 2 / rage_conversion_value` from raw damage *before* mitigation (so even a fully-mitigated swing gives rage). Need `CleanDamage::absorbed_damage + mitigated_damage` to back-compute the unmitigated value.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `static uint32 Unit::DealDamage(...)` | `pub fn deal_damage(&mut WorldState, attacker_guid, victim_guid, damage, ...) -> u32` | Free function in `wow-combat::damage`; takes `&mut WorldState` for access to both Units |
| `static void Unit::DealDamageMods(...)` | `fn deal_damage_mods(...)` (private to `wow-combat::damage`) | — |
| `static void Unit::CalcAbsorbResist(DamageInfo&, Spell*)` | `fn calc_absorb_resist(damage_info: &mut DamageInfo, spell: Option<&SpellInfo>, world: &mut WorldState)` | Mutates DamageInfo + walks aura list via WorldState |
| `static uint32 Unit::CalcArmorReducedDamage(...)` | `fn calc_armor_reduced_damage(attacker_level: u8, victim_armor: u32, damage: u32, spell: Option<&SpellInfo>) -> u32` | Stateless pure function |
| `int32 Unit::MeleeDamageBonusDone(...)` | `fn melee_damage_bonus_done(attacker: &Unit, victim: &Unit, damage: i32, ...) -> i32` | — |
| `int32 Unit::MeleeDamageBonusTaken(...)` | `fn melee_damage_bonus_taken(...)` | — |
| `int32 Unit::SpellDamageBonusDone(...)` | `fn spell_damage_bonus_done(...)` | — |
| `int32 Unit::SpellDamageBonusTaken(...)` | `fn spell_damage_bonus_taken(...)` | — |
| `class DamageInfo` | `struct DamageInfo { ... }` | POD-like; no methods, helper free fns |
| `struct CalcDamageInfo` | `struct CalcDamageInfo { damages: [DamageInfoSide; 2], hit_outcome, hit_info, proc_attacker, proc_victim, proc_ex }` | Two-side split for dual-school weapons |
| `struct CleanDamage` | `struct CleanDamage { absorbed_damage: u32, mitigated_damage: u32, attack_type: WeaponAttackType, hit_outcome: MeleeHitOutcome }` | — |
| `enum DamageEffectType` | `enum DamageEffectType { DirectDamage, SpellDirectDamage, Dot, Heal, NoDamage, SelfDamage }` | — |
| `SpellSchool` / `SpellSchoolMask` | `#[repr(u8)] enum SpellSchool` + `bitflags struct SpellSchoolMask: u8` | Bits identical to C++ |
| `bitfield HitInfo` | `bitflags struct HitInfo: u32` | Hex bits identical |
| `std::multimap<uint32, SpellInfo const*> SchoolImmunityList` | `HashMap<u32, Vec<u32>>` (cause spell id → school masks) | Multimap → `HashMap<K, Vec<V>>` |
| `std::multimap<Mechanics, SpellInfo const*> MechanicImmunityList` | `HashMap<Mechanic, Vec<u32>>` | — |
| `std::vector<AuraEffect*> AbsorbAuraList` | `Vec<AuraEffectId>` (handle, not pointer) | Indexed lookup into world's aura registry |
| `static void Unit::Kill(...)` | `pub fn kill(world: &mut WorldState, attacker_guid, victim_guid, durability_loss: bool, skip_setting_death_state: bool)` | Free function in `wow-combat::kill` |
| `Unit::ProcDamageAndSpell(...)` | `pub fn proc_damage_and_spell(world: &mut WorldState, ...)` | Free function in `wow-combat::proc` |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Audited the C++ damage pipeline at `/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp` (`Unit::DealDamage` line 767, `Unit::DealDamageMods` line 746, `Unit::CalcAbsorbResist` line 1789, `Unit::CalcArmorReducedDamage` line 1623, `Unit::MeleeDamageBonusDone` line 7558, `Unit::MeleeDamageBonusTaken` line 7670, `Unit::SpellDamageBonusDone` line 6592, `Unit::SpellDamageBonusTaken` line 6775, `Unit::Kill` line 10457) plus immunity-list type definitions in `Unit.h` lines 1700-1760, against the Rust workspace at `/home/server/rustycore/crates/wow-combat/`.

**Empty-crate finding — CONFIRMED.** `/home/server/rustycore/crates/wow-combat/src/lib.rs` is **0 lines** (verified via `wc -l`). `Cargo.toml` is present and the crate is declared in the workspace `Cargo.toml` member list, but ships zero code. **None** of the nine C++ damage functions exist in any form: `DealDamage`, `DealDamageMods`, `CalcAbsorbResist`, `CalcArmorReducedDamage`, `MeleeDamageBonusDone`, `MeleeDamageBonusTaken`, `SpellDamageBonusDone`, `SpellDamageBonusTaken`, `ProcDamageAndSpell`. None of the support types exist: `DamageInfo`, `CalcDamageInfo`, `CleanDamage`, `SpellNonMeleeDamage`, `DamageEffectType`, `SchoolImmunityList`, `MechanicImmunityList`, `AbsorbAuraList`, `HitInfo` bitflag.

**Surrogate path.** What runs today is one line in `crates/wow-ai/src/lib.rs`: a `take_damage(amount: u32)` that does `self.hp = self.hp.saturating_sub(amount)`. There is no school routing, no immunity check, no armor reduction, no resist roll, no absorb-shield depletion, no taken-percent aura application, no resilience, no proc trigger, no kill cascade. The `wow-spell` crate has a stub `SpellSchool` enum but no `SpellSchoolMask` bitflag — the school field is effectively unused.

**Kill pipeline is broken.** `Unit::Kill` (line 10457 of `Unit.cpp`) chains: `JustDied` AI hook → `Loot::FillLoot` corpse loot generation → `Player::RewardPlayerAndGroupAtKill` (XP grant via `XpGainBaseMap`, faction reputation via `OnKillReputation`, quest credit via `KilledMonsterCredit`, battleground score update via `BattlegroundScore::UpdateScore`) → `Player::DurabilityLossAll` (10% on player death) → `SetDeathState(JustDied)` → corpse spawn → achievement hook (`OnKillCreatureType`) → SMSG_PARTY_KILL_LOG broadcast to group → SMSG_DURABILITY_DAMAGE_DEATH to dying player. **In RustyCore, none of this runs.** When `creature.hp ≤ 0`, the only effects are `creature.is_alive = false` (a single bool flip in `wow-ai`) and an `SMSG_ATTACK_STOP{now_dead: true}` packet from `crates/wow-world/src/handlers/combat.rs`. **Killing a quest objective mob produces zero progression**: no XP, no quest credit, no loot drops, no faction rep, no AI death hook. Every PvE gameplay loop is content-blocked until #COMBAT-DMG.14 (`deal_damage`) and #COMBAT-DMG.15 (`kill`) ship — and those depend on the entire `wow-combat` crate existing first, which requires #COMBAT-DMG.1 through #COMBAT-DMG.13 as prerequisites. Estimated total work: **80-120 hours** (one quarter of a developer-week, conservatively).

**Worst hidden divergence.** Even *with* the surrogate `take_damage` doing flat HP subtraction, paladin Bubble (`SPELL_AURA_SCHOOL_IMMUNITY` covering all school masks) is silently ignored — the immunity list type does not exist, so there is no place for the aura handler to register. A bubbled paladin takes full damage. Same for Ice Block, Anti-Magic Shell, Divine Shield, Frost Ward, Mana Shield, PW:Shield, Sacrifice — every defensive cooldown in the game is a no-op. This is not a bug to file — it is a missing subsystem.

**Cross-references.** See `combat.md` §8 / §13 for the parent-doc audit (covers the same `wow-combat` 0-line confirmation). See `combat-threat.md` §13 for the `ThreatManager` companion gap. See `combat-manager.md` §13 for the `CombatManager` companion gap. See `entities-unit.md` (when authored) for the broader `Unit` class state.
