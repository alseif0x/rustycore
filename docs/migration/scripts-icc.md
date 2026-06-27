# Migration: Scripts — Icecrown Citadel (ICC)

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/`
> **Rust target crate(s):** `crates/wow-scripts/` (content layer; the `wow-script` engine crate hosts the runtime)
> **Layer:** L8 (content / boss scripts; sits on top of L7 dispatch + AI base + spell aura framework)
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete (file inventory + per-boss enum/phase audit, 2026-05-01)
> **Last updated:** 2026-05-01

Cross-links: [scripts.md](scripts.md) (engine-level scripting model), [scripting.md](scripting.md), [ai.md](ai.md) (the `BossAI` base used by every encounter), [spells.md](spells.md) (spell-script and aura hooks). The `_attic/` brief in `crates/wow-world/_attic/README.md` is relevant — many of the stub handlers there are the dispatch sites a future ICC content port will need to plug into.

---

## 1. Purpose

Icecrown Citadel is the final WotLK 25-/10-man raid (`map_id = 631`). The scripts directory implements 12 encounters + 1 mini-boss + the instance state machine, raid-wide trash mechanics, gunship vehicle combat, and the Lich King role-play sequences. It is the largest single content directory in the Northrend tree at ~21k LOC across 17 files and is the canonical reference for late-WotLK boss scripting patterns (`SmartScript`-free, hand-written `BossAI`, `EventMap`-driven phase machines, `SpellScript` + `AuraScript` overrides for non-trivial spell mechanics, `ENCOUNTER_FRAME_*` UI sync, dynamic difficulty via `IsHeroic()` / `Is25ManRaid()`, hard-mode unlocks via per-encounter "Strength of Wrynn" / "Hellscream's Warsong" buff stacks).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `scripts/Northrend/IcecrownCitadel/boss_blood_prince_council.cpp` | 1341 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_blood_queen_lana_thel.cpp` | 778 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_deathbringer_saurfang.cpp` | 1257 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_festergut.cpp` | 457 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp` | 2243 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_lady_deathwhisper.cpp` | 1081 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_lord_marrowgar.cpp` | 697 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_professor_putricide.cpp` | 1488 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_rotface.cpp` | 796 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_sindragosa.cpp` | 1526 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_sister_svalna.cpp` | 1487 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp` | 2815 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/boss_valithria_dreamwalker.cpp` | 1284 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/go_icecrown_citadel_teleport.cpp` | 121 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | 1596 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/icecrown_citadel.h` | 578 | `prefix` |
| `scripts/Northrend/IcecrownCitadel/instance_icecrown_citadel.cpp` | 1420 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.h` | 578 | Shared header: spell IDs, NPC IDs, encounter data enum (`DATA_LORD_MARROWGAR` … `DATA_THE_LICH_KING` = 0..11, `EncounterCount = 13`), achievement IDs, teleporter spells, faction-buff stacks |
| `src/server/scripts/Northrend/IcecrownCitadel/instance_icecrown_citadel.cpp` | 1420 | `InstanceMapScript` — boss state machine, teleporter unlock fan-in, gunship faction selection (`DATA_TEAM_IN_INSTANCE`), Putricide table / Sister Svalna door state, achievement fan-out, Sindragosa frostwyrm intro counter |
| `src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | 1596 | Trash + non-boss content: Deathbound Ward (`SPELL_STONEFORM`), Nerub'ar Broodkeeper, weekly quests, Crok Scourgebane gauntlet, Sister Svalna captains (Arnath/Brandon/Grondel/Rupert), Coldflame Jets, teleporter GO triggers |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_lord_marrowgar.cpp` | 697 | Bone Storm phase + bone spike pinning. `SpellScript` for `coldflame` / `bone_spike_graveyard` target selection. "Boned" achievement |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_lady_deathwhisper.cpp` | 1081 | Mana-shield phase 1 → physical phase 2 transition (`PHASE_ONE`/`PHASE_TWO`). Cult Fanatic / Cult Adherent reanimation, Vengeful Shade, Dominate Mind |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp` | 2243 | Two-faction transport battle — Skybreaker (A) vs Orgrim's Hammer (H). Cannon vehicles, teleport portal, boarding adds (`npc_gunship_boarding_add`), ship visit counter (`ACTION_SHIP_VISITS`), Muradin/Saurfang AI |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_deathbringer_saurfang.cpp` | 1257 | Blood Power resource emulation, Mark of the Fallen Champion, Blood Beast adds, Boiling Blood. Pre-fight RP travels with `MovePoints` enum |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_festergut.cpp` | 457 | Single-phase: Pungent Blight after 3 Inhale Blights, Gas Spore, Vile Gas, Gastric Bloat tank stack, Malleable Goo |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_rotface.cpp` | 796 | Slime Spray, Mutated Infection (debuff swap), Big Ooze Combine puddles → Unstable Ooze Explosion. Putricide visit voice lines |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_professor_putricide.cpp` | 1488 | Three-phase: gas-cloud / volatile-ooze chases, slime puddle growth, Unbound Plague (heroic), Mutated Abomination vehicle. Heroic Tear Gas RP |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_blood_prince_council.cpp` | 1341 | 3-prince linked HP pool, Empowered Blood Orb rotation (Keleseth shadow, Taldaram fire balls, Valanar kinetic bombs + shock vortex), Dark Nucleus tanking object |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_blood_queen_lana_thel.cpp` | 778 | Vampiric Bite chain (creates 1 → 2 → 4 vampires), Pact of the Darkfallen, Essence of the Blood Queen, Frenzied Bloodthirst, Air Phase + Bloodbolt Whirl |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_valithria_dreamwalker.cpp` | 1284 | Reverse encounter: heal Valithria to 100%. Suppressors (heal-debuff), Blistering Zombies, Risen Archmages, Dream Portal (Valithria-on-the-vehicle role play state). Defeat = Valithria's Disenchant boss frame |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_sindragosa.cpp` | 1526 | 3-phase frost dragon. Spinestalker + Rimefang frostwyrm intro counter (`DATA_SINDRAGOSA_FROSTWYRMS`). Frost Beacon → Ice Tomb stacking. Mystic Buffet stack debuff. Blistering Cold |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_sister_svalna.cpp` | 1487 | Mini-boss in the gauntlet. Aether Shield reflect, Impaling Spear vehicle, captain rez sequence, Crok yells, gauntlet wave timing |
| `src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp` | 2815 | Largest single boss: 3 active phases + 2 transition role-play phases + Frostmourne sub-instance. Necrotic Plague, Defile (growing AoE), Soul Reaper, Shambling Horror / Drudge Ghoul adds, Val'kyr Shadowguard fly-grab vehicle, Vile Spirits, Frostmourne phase with Terenas Menethil + Spirit Warden, Fury of Frostmourne enrage |
| `src/server/scripts/Northrend/IcecrownCitadel/go_icecrown_citadel_teleport.cpp` | 121 | GameObject teleporter glue calling `instance->SetData(DATA_UPPERSPIRE_TELE_ACT, ...)` |

---

## 3. Classes / Structs / Enums (top-level inventory)

| Symbol | Kind | Purpose |
|---|---|---|
| `instance_icecrown_citadel::instance_icecrown_citadel_InstanceMapScript` | class | Per-instance state holder (`InstanceScript` subclass). Owns 13-slot encounter array, team faction (`_teamInInstance`), Putricide table state, Sister Svalna door, Sindragosa frostwyrm count, achievement flags |
| `boss_<name>AI` (12×) | struct (`BossAI` subclass) | One per encounter. Holds local `EventMap events`, summon group, phase enum |
| `npc_<helper>AI` | struct | Adds, vehicles, role-play actors. Examples: `npc_coldflame`, `npc_bone_spike`, `npc_blood_orb_controller`, `npc_dark_nucleus`, `npc_valkyr_shadowguard`, `npc_terenas_menethil`, `npc_spirit_warden`, `npc_strangulate_vehicle`, `npc_frostmourne_trigger`, `npc_gunship`, `npc_gunship_boarding_add`, `npc_high_overlord_saurfang_igb`, `npc_muradin_bronzebeard_igb`, `npc_zafod_boombox`, `npc_putricide_oozeAI` |
| `spell_<name>` (≈70×) | `SpellScript` / `AuraScript` | Per-spell hook. Notable: `spell_marrowgar_coldflame`, `spell_marrowgar_bone_spike_graveyard`, `spell_putricide_unbound_plague`, `spell_valanar_kinetic_bomb`, `spell_lich_king_defile`, `spell_lich_king_necrotic_plague`, `spell_lich_king_harvest_soul`, `spell_lich_king_summon_into_air` |
| `ICSharedSpells` (enum) | `icecrown_citadel.h` | Cross-encounter spell ID table. Includes `SPELL_BERSERK = 26662`, `SPELL_BERSERK2 = 47008`, `SPELL_HELLSCREAMS_WARSONG = 73822`, `SPELL_STRENGHT_OF_WRYNN = 73828`, `SPELL_REPUTATION_BOSS_KILL = 73843`, Shadowmourne questline spells |
| `ICDataTypes` (enum) | `icecrown_citadel.h` | 47 data slots. 0..11 are encounter states, 12 is Sister Svalna door, 13..46 are auxiliary (achievement flags, captain GUIDs, intro flags, faction buff stack count) |
| `ICCreaturesIds` (enum) | `icecrown_citadel.h` | ~120 NPC entry IDs grouped by encounter |
| `ICTeleporterSpells` (enum) | `icecrown_citadel.h` | 7 teleport spells (`LIGHT_S_HAMMER_TELEPORT = 70781` … `SINDRAGOSA_S_LAIR_TELEPORT = 70861`) gated behind boss-state checks |
| `Phases` (enum, per-boss, varies) | local | Examples: Lady Deathwhisper `PHASE_ONE / PHASE_TWO`; Lich King `PHASE_ONE / PHASE_TWO / PHASE_THREE / PHASE_TRANSITION_*`; Putricide `PHASE_COMBAT_1 / 2 / 3` + transitions; Sindragosa `PHASE_GROUND / PHASE_FLYING` + third stacking phase |
| `Events` (enum, per-boss) | local | Per-boss timed-event IDs handed to `EventMap::ScheduleEvent` |

---

## 4. Critical public methods / functions

The public surface every encounter exposes is dictated by `BossAI` / `ScriptedAI`. The instance script adds its own `Get/SetData` keys.

| Symbol | Purpose | Calls into |
|---|---|---|
| `BossAI::JustEngagedWith(Unit*)` (override per boss) | Set boss state, send `ENCOUNTER_FRAME_ENGAGE`, schedule opening events | `InstanceScript::SetBossState`, `EventMap::ScheduleEvent`, `Talk(SAY_AGGRO)` |
| `BossAI::Reset()` (override) | Clear `EventMap`, despawn summons, restore phase 1, clear achievement flags via `SetData` | `EventMap::Reset`, `SummonList::DespawnAll` |
| `BossAI::JustDied(Unit*)` (override) | Set boss state to `DONE`, `ENCOUNTER_FRAME_DISENGAGE`, drop Shadowmourne quest credit (`SPELL_SHADOWS_FATE`) | `instance->SetBossState`, `_JustDied` helper |
| `BossAI::UpdateAI(uint32 diff)` (override) | Tick `events.Update(diff)`, drain `events.ExecuteEvent()`, dispatch per-event spell casts and movement | per-boss event handlers |
| `BossAI::DamageTaken(Unit*, uint32&)` (Lich King, Lady DW, Saurfang, Putricide, Sindragosa) | Phase transitions on HP%; Saurfang Blood Power gain | `events.SetPhase`, `events.ScheduleEvent` |
| `BossAI::KilledUnit(Unit*)` (Saurfang, Lich King, Festergut, Putricide, Lana'thel) | Quest-credit, Shadowmourne kill counter (`SPELL_UNSATED_CRAVING`), Mark of the Fallen Champion handling | quest system, Shadowmourne aura logic |
| `instance_icecrown_citadel::SetBossState(type, state)` | Gate teleporter unlocks, gunship spawn, Putricide door state, Lich King platform spawn | `InstanceScript::SetBossState`, GameObject state writes |
| `instance_icecrown_citadel::SetData(type, data)` | Set achievement flags (`DATA_BONED_ACHIEVEMENT`, `DATA_OOZE_DANCE_ACHIEVEMENT`, `DATA_NAUSEA_ACHIEVEMENT`, `DATA_ORB_WHISPERER_ACHIEVEMENT`), team selection, Putricide table, frostwyrm count, Coldflame Jets state | per-flag |
| `instance_icecrown_citadel::GetData(type)` | Read team faction, faction-buff stack count, achievement flag, Coldflame Jets state | — |
| `instance_icecrown_citadel::GetGuidData(type)` | Resolve boss/helper GUID by `DATA_*` slot | `_summons` map |
| `instance_icecrown_citadel::OnPlayerEnter(Player*)` | Apply Hellscream/Wrynn buff if Lady DW done and gunship not done (`SPELL_STRENGHT_OF_WRYNN` / `SPELL_HELLSCREAMS_WARSONG`); set faction GO state | `Player::CastSpell`, `DoCastSpellOnPlayer` |
| `npc_gunship::SetGUID(guid, ACTION_SHIP_VISITS)` | Track which players visited the enemy ship (Im On A Boat achievement) | gunship boarding flow |
| `SpellScript::OnEffectHitTarget` (e.g. `spell_marrowgar_coldflame::HandleScriptEffect`) | Cast a chain spell on a script-selected target | `Spell::CastSpell` |

---

## 5. Module dependencies

**Depends on:**
- `wow-ai` — `BossAI` / `ScriptedAI` / `PassiveAI` base classes, `EventMap`, `SummonList`, `MoveInLineOfSight`. See [ai.md](ai.md).
- `wow-spell` — `SpellScript` / `AuraScript` registration, target-selector hooks (`SpellEffectFn`, `AuraEffectApplyFn`), `RegisterSpellScript`. See [spells.md](spells.md).
- `wow-combat` — Threat list reads, `Attack`, `DoCastVictim`, `DoMeleeAttackIfReady`.
- `wow-world` — `InstanceScript` base, `Map::DoOnPlayers`, `ENCOUNTER_FRAME_*` opcodes, `Creature::SummonCreature`. The eventual `MapManager` will own the per-creature state these scripts mutate.
- `wow-data` — `Map.db2` (map ID 631), `CreatureTemplate` rows, `SpellInfo` rows for the ~250 spells these encounters reference.
- `wow-database` — `creature` / `gameobject` / `spawn_group_template` / `instance_template` / `instance_encounters` rows. The instance script reads `creature_template_addon` for vehicle seats.
- `wow-script` (engine) — `ScriptMgr`, `RegisterCreatureAI`, `inventory::submit!` registration. See [scripts.md](scripts.md) and [scripting.md](scripting.md).
- `wow-achievement` — `DATA_*_ACHIEVEMENT` flags fan out to `AchievementMgr::CompletedCriteria`.
- `wow-loot` — Loot mode bits used for hard-mode loot tables (analogous to Ulduar `LOOT_MODE_HARD_MODE_*`).

**Depended on by:**
- `world-server` binary loads the instance via `MapManager` → `Map::AddInstanceScript` (currently absent in Rust).
- Achievement tables (`achievement_criteria_data`) reference these script data slots by name.
- The Shadowmourne questline (an Item-script dependency) reads `SPELL_UNSATED_CRAVING` / `SPELL_SHADOWS_FATE` set by every ICC boss kill.

---

## 6. SQL / DB queries (if any)

The scripts themselves do not emit ad-hoc SQL — they manipulate runtime state. The instance pipeline reads:

| Source | Purpose | DB |
|---|---|---|
| `instance_template` row for map 631 | Bind script name (`instance_icecrown_citadel`), reset times | world |
| `instance_encounters` rows for ICC | Maps encounter `DATA_*` slots → DBC encounter IDs | world |
| `creature` + `spawn_group_template` (groups `SPAWN_GROUP_ALLIANCE_ROS` / `SPAWN_GROUP_HORDE_ROS`) | Faction-conditional Rampart-of-Skulls spawns | world |
| `creature_addon` / `creature_template_addon` | Vehicle seats for Mutated Abomination, Val'kyr, Strangulate vehicle, Bone Spike, gunship cannons | world |
| `gameobject` + `gameobject_template` | Teleporters, Light's Hammer portals, gunship transports, Putricide table, Sigil doors, Frostmourne trigger | world |
| `creature_text` rows for every boss | All `Talk(SAY_*)` lookups | world |
| Achievement criteria rows (3 per faction-buff tier × per boss) | Heroic / no-tank-deaths / quirk achievements | character / world |

DBC/DB2 stores read indirectly via the engine:

| Store | What it loads | Read by |
|---|---|---|
| `Map.db2` | Map 631 metadata | `MapManager::CreateMap` |
| `MapDifficulty.db2` | 10N / 25N / 10H / 25H tuning rows | `Map::GetDifficulty`, `IsHeroic()`, `Is25ManRaid()` |
| `Spell*.db2` | The 250 spells these scripts cast/script | `SpellMgr::GetSpellInfo` |
| `VehicleSeat.db2` | Cannon, abomination, val'kyr, gunship seats | `Vehicle::AddPassenger` |
| `Achievement.db2` + `Criteria*.db2` | Boned, Im On A Boat, Nausea, Orb Whisperer, Been Waiting A Long Time, Neck Deep In Vile, Once Bitten Twice Shy, etc. | `AchievementMgr` |

---

## 7. Wire-protocol packets (if any)

No new opcodes are *defined* by ICC scripts; they emit existing world-server opcodes via helper APIs:

| Opcode | Direction | Sent in |
|---|---|---|
| `SMSG_UPDATE_WORLD_STATE` | server → client | Sindragosa intro count, gunship faction state, Putricide table |
| `SMSG_INSTANCE_ENCOUNTER_GAIN_COMBAT_RESOLUTION` (ENCOUNTER_FRAME_ENGAGE / DISENGAGE / UPDATE_PRIORITY) | server → client | Every `instance->SendEncounterUnit(...)` call — boss frames |
| `SMSG_PLAY_SOUND` / `SMSG_PLAY_MUSIC` | server → client | Phase transitions, Lich King RP, gunship music |
| `SMSG_CONVERSATION_LINE` / `SMSG_MESSAGECHAT` | server → client | All `Talk(SAY_*)` |
| `SMSG_SPELL_GO` / `SMSG_SPELL_START` | server → client | Every scripted `DoCast` |
| `SMSG_AURA_UPDATE` | server → client | Mark of Fallen Champion, Necrotic Plague, Mystic Buffet stacks, Sanity-style stacks (Putricide table heroic) |
| `SMSG_VEHICLE_BOARD` / `SMSG_ON_CANCEL_EXPECTED_RIDE_VEHICLE_AURA` | server → client | Mutated Abomination, gunship cannons, Val'kyr grab, Bone Spike pin |
| `SMSG_MOVE_SPLINE_*` | server → client | `MoveCharge`, Bone Storm spline, Lich King platform return, Val'kyr drag |
| `SMSG_RAID_BOSS_EMOTE` | server → client | "Lord Marrowgar prepares Bone Storm!", "Festergut inhales!", etc. |

A future Rust port writes these via existing serializers in `wow-packet` — no new wire types needed.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-scripts/src/lib.rs` — **0 bytes**. Empty stub crate. `Cargo.toml` has 11 lines, no source.
- `crates/wow-script/src/lib.rs` — **0 bytes**. Empty stub for the script-engine plumbing crate.
- No instance script, no boss AI, no spell script, no `SmartScript` runtime is referenced from `wow-world` or `world-server`.

**What's implemented:**
- Nothing. ICC content is **0% ported**.

**What's missing vs C++:**
- The full encounter set (12 bosses + Sister Svalna + ~22 helper NPC scripts + ~70 spell scripts).
- The instance state machine (`instance_icecrown_citadel.cpp` 1420 lines).
- Hard-mode unlock plumbing (Hellscream's Warsong / Strength of Wrynn buff stacks driven by Lich King kills on the same realm).
- Vehicle integration for gunship cannons, Mutated Abomination, Val'kyr Shadowguard, Bone Spike pin, Strangulate Vehicle, Sister Svalna's Impaling Spear.
- Encounter-frame opcodes (`ENCOUNTER_FRAME_ENGAGE` / `DISENGAGE` / `UPDATE_PRIORITY`) with stable boss-priority ordering for Blood Council and Sindragosa frostwyrms.
- Achievement criterion fan-out (`Boned`, `Im On A Boat`, `Nausea`, `Orb Whisperer`, `Once Bitten Twice Shy`, `Been Waiting A Long Time`, `All You Can Eat`, `The Frostwing Halls Achievements`, `Bane of the Fallen King`).
- Shadowmourne questline hooks (`SPELL_UNSATED_CRAVING`, `SPELL_SHADOWS_FATE`).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — there is no Rust ICC code yet, so divergence is total. Suspicions for a future implementation:
  - `EventMap` semantics (group cancel, phase mask) are subtle; any reimplementation MUST preserve `events.DelayEvents(3s, EVENT_GROUP_SPECIAL)` and `events.RescheduleEvent` invariants Marrowgar relies on for Bone Storm pause/resume.
  - Defile growth (`spell_lich_king_defile`) is non-linear — it scales with each tick that hits a player, not with time. A naive port will break the encounter difficulty.
  - The Lich King's Frostmourne sub-instance is a **separate map** (Frostmourne map 658) that the boss script teleports players into; this requires `MapManager` cross-map teleport which is currently in flight (see `_attic/` brief).

**Tests existing:**
- 0 tests. Whole crate is empty.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SCRIPTS_ICC.WBS.001** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_blood_prince_council.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_blood_prince_council.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1341 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.002** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_blood_queen_lana_thel.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_blood_queen_lana_thel.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 778 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.003** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_deathbringer_saurfang.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_deathbringer_saurfang.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1257 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.004** Cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_festergut.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_festergut.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.005** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2243 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.006** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_lady_deathwhisper.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_lady_deathwhisper.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1081 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.007** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_lord_marrowgar.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_lord_marrowgar.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 697 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.008** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_professor_putricide.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_professor_putricide.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1488 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.009** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_rotface.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_rotface.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 796 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.010** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_sindragosa.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_sindragosa.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1526 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.011** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_sister_svalna.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_sister_svalna.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1487 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.012** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2815 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.013** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/boss_valithria_dreamwalker.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_valithria_dreamwalker.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1284 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.014** Cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/go_icecrown_citadel_teleport.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/go_icecrown_citadel_teleport.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.015** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1596 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.016** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/icecrown_citadel.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.h`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 578 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ICC.WBS.017** Partir y cerrar la migracion auditada de `scripts/Northrend/IcecrownCitadel/instance_icecrown_citadel.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/instance_icecrown_citadel.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-world`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1420 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, split before starting).

Pre-requisite umbrella tasks (must precede any boss):

- [ ] **#ICC.0a** Stand up `wow-script` engine crate: `ScriptMgr`, `CreatureScript` registry, `inventory::submit!` glue, `RegisterCreatureAI` macro analogue, `BossAI` base struct (M)
- [ ] **#ICC.0b** Port `EventMap`: phase mask + group ID + reschedule/delay/cancel semantics, with property-test parity vs C++ `EventMap.cpp` (H)
- [ ] **#ICC.0c** Port `InstanceScript` base: encounter state array, `SetBossState`/`GetBossState`/`SetData`/`GetData`/`GetGuidData`, save/load to `instance` DB (H)
- [ ] **#ICC.0d** Wire `ENCOUNTER_FRAME_*` opcode emitter in `wow-packet` and `Map::SendEncounterUnit` accessor (M)
- [ ] **#ICC.0e** Port `SpellScript` / `AuraScript` hook system: `OnEffectHit`, `OnObjectAreaTargetSelect`, `OnCheckCast`, `OnPeriodicTick` (H — see [spells.md](spells.md))

Per-boss (~12 each, follows `_attic/` skeleton + this doc's section 2 line counts as complexity proxy):

**Lord Marrowgar (697 lines, lowest)**
- [ ] **#ICC.M.1** `boss_lord_marrowgarAI` struct + Reset/EngagedWith/UpdateAI skeleton (M)
- [ ] **#ICC.M.2** EVENT_BONE_SPIKE_GRAVEYARD scheduling and `SPELL_BONE_SPIKE_GRAVEYARD` (69057) script with non-impaled target selector (M)
- [ ] **#ICC.M.3** `npc_bone_spike` add: pin victim with `SPELL_IMPALED` (69065), `SPELL_RIDE_VEHICLE` (46598) seat 0, EVENT_FAIL_BONED 8s timer setting `DATA_BONED_ACHIEVEMENT = false` (M)
- [ ] **#ICC.M.4** EVENT_COLDFLAME / `npc_coldflame`: branched between `SPELL_COLDFLAME_NORMAL` (69140) for non-storm and `SPELL_COLDFLAME_BONE_STORM` (72705) during storm (M)
- [ ] **#ICC.M.5** EVENT_WARN_BONE_STORM → EVENT_BONE_STORM_BEGIN → EVENT_BONE_STORM_MOVE → EVENT_BONE_STORM_END phase chain with 3s delay on `EVENT_GROUP_SPECIAL` (M)
- [ ] **#ICC.M.6** EVENT_ENABLE_BONE_SLICE bone-slice timer; `SPELL_BONE_SLICE` (69055) only outside Bone Storm (L)
- [ ] **#ICC.M.7** EVENT_ENRAGE 10min hard-enrage `SPELL_BERSERK` (26662) (L)
- [ ] **#ICC.M.8** SpellScript `spell_marrowgar_coldflame` + `spell_marrowgar_coldflame_bonestorm` target selection (random non-impaled, set GUID in AI for follow-up) (M)
- [ ] **#ICC.M.9** SpellScript `spell_marrowgar_bone_spike_graveyard` 1/3/3 target selection per difficulty (M)
- [ ] **#ICC.M.10** Achievement: `DATA_BONED_ACHIEVEMENT` flag plumbed through instance + achievement criterion data (L)
- [ ] **#ICC.M.11** Talk lines (SAY_AGGRO, SAY_BONESTORM_*, SAY_BERSERK, SAY_DEATH, SAY_KILL) wired to `creature_text` (L)
- [ ] **#ICC.M.12** Door open on `IN_PROGRESS`, close on `FAIL`, permanent open on `DONE` via instance gate (L)

**Lady Deathwhisper (1081 lines)** — `#ICC.LD.1..12` (M each, +1 H for `Dominate Mind` fear+threat-redirect)
- Phase 1 mana shield (damage routes to mana not HP — needs `DamageTaken` interception), phase 2 transition at mana 0, Cult Fanatic / Cult Adherent reanimation pipeline, Vengeful Shade summon-and-chase, Dominate Mind, Death and Decay, Frostbolt Volley.

**Icecrown Gunship Battle (2243 lines, XL — split)**
- [ ] **#ICC.IGB.1..6** Cannon vehicle integration, ship visit teleport portal, boarding-add NPCs, Muradin/Saurfang AI, dynamic music, faction selection from `DATA_TEAM_IN_INSTANCE` (each H, total XL)
- [ ] **#ICC.IGB.7..12** Cannon overheat + repair, rocket pack jump, mage portal, Im-On-A-Boat achievement counter, hull HP linkage to encounter state, gunship-respawn-on-FAIL (each M-H)

**Deathbringer Saurfang (1257 lines)** — `#ICC.SF.1..12`
- Pre-fight RP (12 movement waypoints), Blood Power resource (custom power, builds on hits), Mark of the Fallen Champion (debuff, kill = wipe achievement gate), Boiling Blood, Blood Beast adds (10/12 in 25), Rune of Blood, Frenzy at 30%, Berserk at 8min.

**Festergut (457 lines, lowest after Marrowgar)** — `#ICC.FG.1..12`
- Inhale Blight (3 stacks → Pungent Blight nuke), Gas Spore, Vile Gas, Gastric Bloat tank stack (10 = death), Malleable Goo, Putricide voice lines from balcony, Nausea / Flu Shot achievement plumbing.

**Rotface (796 lines)** — `#ICC.RF.1..12`
- Slime Spray cone, Mutated Infection swap, Big Ooze Combine puddles → Unstable Ooze Explosion, Ooze Flood from sluices, Putricide voice lines, Dances With Oozes / Ooze Dance achievement.

**Professor Putricide (1488 lines, H/XL)** — `#ICC.PP.1..14`
- Phase 1 Slime Puddle growth, Volatile Ooze adds chasing target, Phase 2 Gas Cloud chase, Mutated Plague stacking, Mutated Abomination vehicle (player-driven), Tear Gas RP between phases, Choking Gas Bomb, Unstable Experiment summon, heroic Unbound Plague swap mechanic, Heroic-only Mutated Strength buff, achievement (Nausea, Flu Shot Shortage, Heroic: From the Bowels of Bowels).

**Blood Prince Council (1341 lines, H)** — `#ICC.BPC.1..12`
- Linked HP across 3 princes, only one "empowered" at a time (rotating via `npc_blood_orb_controller`), Keleseth shadow-lance + dark nucleus tank-debuff stacking, Taldaram ball-of-flame chase + glittering sparks cone, Valanar kinetic bomb falling-balloon mechanic + shock vortex AoE, invocation rotation, Achievement: Orb Whisperer (no shadow lance hits).

**Blood-Queen Lana'thel (778 lines)** — `#ICC.BQ.1..12`
- Vampiric Bite charm-and-multiply chain (1 → 2 → 4 vampires must bite next bloodthirst), Pact of the Darkfallen, Essence of the Blood Queen buff, Frenzied Bloodthirst forced-bite mechanic, Air Phase Bloodbolt Whirl, Twilight Bloodbolt, Achievement: Once Bitten Twice Shy (must be bitten).

**Valithria Dreamwalker (1284 lines, H)** — `#ICC.VD.1..12`
- Reverse encounter — Valithria is HP-up to win, suppressors are healed-debuff stalkers, Blistering Zombies, Risen Archmage, Dream Portal RP teleporting healers into the dream, Manavoid, defeat triggers `_JustDied` only on full heal not damage, integrates with Lich King role play (sets `DATA_VALITHRIA_LICH_KING`).

**Sindragosa (1526 lines, H)** — `#ICC.SI.1..14`
- Spinestalker + Rimefang frostwyrm intro, count tracked via `DATA_SINDRAGOSA_FROSTWYRMS`, ground phase (Frost Breath + Cleave), air phase 35% (Blistering Cold + Ice Tomb beacons), third stacking phase 35% (Mystic Buffet stack debuff + ground spike Frost Bombs), Frost Beacon target selector excludes tank, Achievement: All You Can Eat (no Mystic Buffet stack).

**The Lich King (2815 lines, XL — split aggressively)**
- [ ] **#ICC.LK.1** AI skeleton + 5-phase machine (P1 / Transition1 / P2 / Transition2 / P3) + Frostmourne sub-phase (M)
- [ ] **#ICC.LK.2** Phase 1 adds: Shambling Horror (Enrage), Drudge Ghoul, scheduled timers (M)
- [ ] **#ICC.LK.3** Necrotic Plague: stacking debuff that jumps + grows on jump (`spell_lich_king_necrotic_plague`) (H)
- [ ] **#ICC.LK.4** Infest: AoE periodic, removed if HP > 90% (`spell_lich_king_infest`) (M)
- [ ] **#ICC.LK.5** Shadow Trap (heroic-only summon) (M)
- [ ] **#ICC.LK.6** Transition 1 RP: Remorseless Winter, Raging Spirit summon, Ice Sphere, Pain and Suffering (H)
- [ ] **#ICC.LK.7** Phase 2: Defile growing AoE — must be moved out of (`spell_lich_king_defile`, the size grows per damage tick); Soul Reaper tank-debuff (H)
- [ ] **#ICC.LK.8** Val'kyr Shadowguard: target lift via vehicle, drop into death pit, must be DPS'd to fall faster than the timer; integrates with `npc_strangulate_vehicle` (H)
- [ ] **#ICC.LK.9** Transition 2: Quake (platform shake), more Raging Spirits + Vile Spirits (M)
- [ ] **#ICC.LK.10** Phase 3: Vile Spirits (homing exploding adds) + continued Defile/Soul Reaper, Harvest Soul (heroic: Frostmourne sub-instance map 658) (XL)
- [ ] **#ICC.LK.11** Frostmourne sub-instance: Terenas Menethil + Spirit Warden ghost fight, return teleport `SPELL_HARVEST_SOUL_TELEPORT_BACK = 72597` (H)
- [ ] **#ICC.LK.12** Fury of Frostmourne enrage at 10%, Tirion-Fordring win RP, achievement `Bane of the Fallen King` (heroic 25 with hard-mode buffs maxed) (M)

**Sister Svalna mini-boss (1487 lines)** — `#ICC.SV.1..12`
- Frostwing Halls gauntlet wave timing, Crok Scourgebane + 4 captains AI (Captain Arnath / Brandon / Grondel / Rupert), Aether Shield reflect, Impaling Spear vehicle pin, captain rez sequence, Hellish Stab heroic, achievement Frostwing.

**Instance / shared content**
- [ ] **#ICC.INST.1** `instance_icecrown_citadel` script — 47 data slots, 13 boss slots, faction selection from first player joining (H)
- [ ] **#ICC.INST.2** Teleporter unlock fan-in (`SetTeleporterState` for 7 destinations) (M)
- [ ] **#ICC.INST.3** Faction-buff stack auto-apply on player enter when prerequisites met (M)
- [ ] **#ICC.INST.4** Crok Scourgebane gauntlet trigger + Coldflame Jets state (M)
- [ ] **#ICC.INST.5** Putricide table state, Sister Svalna door, Sigil door state (M)
- [ ] **#ICC.INST.6** Achievement-flag fan-out and per-encounter clear on Reset (M)
- [ ] **#ICC.INST.7** Trash: Deathbound Ward Stoneform, Nerub'ar Broodkeeper waves, weekly quest NPCs (H)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SCRIPTS_ICC.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 17 files / 20965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`. Rust target: `crates/world-server`, `crates/wow-script`, `crates/wow-scripts`, `crates/wow-world`. | `cargo test -p world-server && cargo test -p wow-script && cargo test -p wow-scripts` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SCRIPTS_ICC.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 17 files / 20965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`. Rust target: `crates/world-server`, `crates/wow-script`, `crates/wow-scripts`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SCRIPTS_ICC.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 17 files / 20965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`. Rust target: `crates/world-server`, `crates/wow-script`, `crates/wow-scripts`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SCRIPTS_ICC.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 17 files / 20965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp`. Rust target: `crates/world-server`, `crates/wow-script`, `crates/wow-scripts`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests demonstrating Rust = C++ for invariants the encounters rely on.

- [ ] Test: `EventMap::DelayEvents(3s, EVENT_GROUP_SPECIAL)` postpones only events in that group, leaves Bone-Storm-end timer unaffected (Marrowgar)
- [ ] Test: Bone Spike `SPELL_FAIL_BONED` 8s timer flips `DATA_BONED_ACHIEVEMENT` to `false` exactly once and survives reset
- [ ] Test: Lady Deathwhisper Phase 1→2 transitions only when mana would go negative (damage > current mana), not at HP%
- [ ] Test: Necrotic Plague stack increments by 1 on every jump, not on application; expires resets to 0
- [ ] Test: Defile size growth is proportional to ticks-that-hit, not elapsed time — script-test with deterministic fake clock
- [ ] Test: Val'kyr lift respects `MoveCharge` priority `MOTION_PRIORITY_HIGHEST` so other movement does not preempt
- [ ] Test: Mark of the Fallen Champion + boss-kill = encounter wipe (achievement + reset)
- [ ] Test: Frostmourne sub-instance teleport puts the player on map 658 then `SPELL_HARVEST_SOUL_TELEPORT_BACK` returns them to map 631 same coords
- [ ] Test: Mystic Buffet stack debuff is *not* dispellable; `Aura::ModStackAmount` only via boss spell
- [ ] Test: Sindragosa intro counter (`DATA_SINDRAGOSA_FROSTWYRMS`) is 2 → 0 then opens her door
- [ ] Test: Faction buff stack count matches `world_state` value applied to all players in the raid on enter
- [ ] Test: `instance_encounters` rows resolve to `DATA_*` slots 0..12 in the same order as the C++ enum
- [ ] Test: ICC reset clears all 47 data slots and all 13 boss states atomically
- [ ] Test: Hard-enrage berserks fire at exactly the C++ schedule (Marrowgar 10min, Saurfang 8min, LK soft-enrage at 15min)
- [ ] Test: `ENCOUNTER_FRAME_UPDATE_PRIORITY` for Blood Council assigns priorities 0/1/2 in Keleseth/Taldaram/Valanar order

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 17 files / 20965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | `crates/wow-scripts/` (content layer; the `wow-script` engine crate hosts the runtime) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SCRIPTS_ICC.DIV.001` | `crates/wow-scripts` (`exists_empty`, 0 Rust lines) | 17 C++ files / 20965 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTS_ICC.DIV.002` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 17 C++ files / 20965 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTS_ICC.DIV.003` | `crates/wow-scripts/src/lib.rs` (`exists_empty`, 0 Rust lines) | 17 C++ files / 20965 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SCRIPTS_ICC.DIV.004` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 17 C++ files / 20965 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_the_lich_king.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/boss_icecrown_gunship_battle.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/IcecrownCitadel/icecrown_citadel.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **Two-step dispatch (AGENTS.md)**: every `boss_*AI` and `npc_*AI` must both have a registered `BossAI` factory **and** a `PacketHandlerEntry`-equivalent inventory submission, or it silently dies. The script registry needs the same discipline as the packet dispatcher.
- **`Talk()` lookups** read from `creature_text`. Missing rows → silent no-op, encounter is technically beatable but loses RP/timing cues. Build a lint that scans the SAY_/EMOTE_/WHISPER_ enums against the DB on startup in dev mode.
- **`IsHeroic()` and `Is25ManRaid()`** are read 100s of times per encounter. Cache the difficulty per session, do not re-read `Map::GetDifficulty()` on every event tick.
- **Defile (LK)** has historically been a balance-bug magnet across cores; the canonical TC code in `boss_the_lich_king.cpp:~1900` is correct for 3.4.3. Do not "simplify" the growth math.
- **Gunship encounter uses transports**, which RustyCore's `wow-world` does not yet model (search `_attic/STATUS_MIGRATION.md`). The gunship port is blocked on transport state — schedule `#ICC.IGB.*` after transport infrastructure lands.
- **Sister Svalna's Aether Shield** reflects spells back to caster. Reflection in TC is implemented via `Spell::ReflectEvent`; needs hooks in `wow-spell` proc system before the boss script can use it.
- **Hard-mode unlocks (Hellscream's Warsong / Strength of Wrynn)** are persistent at the realm level — the buff stack count comes from the auth/realm DB, not the instance. Don't model it as instance-local state.
- **Shadowmourne questline credit** (`SPELL_UNSATED_CRAVING` → `SPELL_SHADOWS_FATE`) is applied by every ICC boss `JustDied`. Forgetting this on one boss silently breaks the legendary questline.
- **`_attic/STATUS_MIGRATION.md`** in `crates/wow-world/_attic/` documents that the previous integration attempt did not have a working `BossAI` base — that's the precondition for any of this to compile, hence the umbrella tasks #ICC.0a..0e.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class boss_lord_marrowgar : public CreatureScript` + inner `boss_lord_marrowgarAI : BossAI` | `pub struct BossLordMarrowgarAI { ... }` impl `BossAI` trait | Drop the outer-class wrapper; register via inventory macro |
| `BossAI` base | `trait BossAI: ScriptedAI { ... }` with default tick + summon-list + event-map | Composition over inheritance |
| `EventMap events` | `event_map: EventMap` field | Port wholesale; semantics covered in `#ICC.0b` |
| `events.ScheduleEvent(EVENT_X, 5s, group, phase)` | `events.schedule(EVENT_X, Duration::from_secs(5), group, phase)` | 1:1 |
| `DoCast(target, SPELL_X)` | `self.do_cast(target, SpellId(X))` helper on `ScriptedAI` | Spell casting goes via `wow-spell` |
| `instance->SetBossState(DATA_X, state)` | `instance.set_boss_state(IccData::Marrowgar, EncounterState::InProgress)` | Use a typed data enum, not raw u32 |
| `SpellScript`, `RegisterSpellScript` | `impl SpellScript for SpellMarrowgarColdflame` + `inventory::submit!` | Hooks return `EffectResult` |
| `AuraScript::OnEffectApply` | `fn on_effect_apply(&mut self, aura: &mut Aura, eff: u8) -> AuraResult` | — |
| `me->CastSpell(target, SPELL_RIDE_VEHICLE, true)` | `self.creature.cast_spell(&target, SpellId(46598), CastFlags::TRIGGERED)` | Vehicle integration is a `wow-world` dependency |
| `instance->SendEncounterUnit(ENCOUNTER_FRAME_ENGAGE, me)` | `instance.send_encounter_unit(EncounterFrame::Engage, &self.creature)` | Builds and broadcasts opcode |
| `Talk(SAY_AGGRO)` | `self.talk(MarrowgarText::Aggro)` | Per-boss text enum mapped to `creature_text` |
| `me->RemoveAurasDueToSpell(SPELL_BONE_STORM)` | `self.creature.remove_auras_due_to_spell(SpellId(69076))` | — |
| `me->HasLootMode(LOOT_MODE_*)` (used in Ulduar, ICC has analogue for hard-mode) | `self.creature.has_loot_mode(LootMode::HARD_MODE_1)` | bitflags |
| `_JustDied()` helper | `self.just_died_default()` provided method on trait | Drops summons, sets boss state |
| `class npc_bone_spike : public VehicleAI` | `struct BoneSpikeAI { ... }` impl `VehicleAI` | Vehicle base unimplemented in Rust; blocks Marrowgar |

---

## 13. Audit (vs C++ source, 2026-05-01)

Audit pass performed against the listed canonical files:

- File inventory matches a fresh `ls` of the directory: 17 files, 20 965 lines (counted via `wc -l`). Boss-by-boss line counts in section 2 cross-checked against the same.
- All 12 encounter `DATA_*` slots (0..11) and the Sister Svalna slot (12) verified against `icecrown_citadel.h:73-119`. Auxiliary slots 13..46 enumerated and listed.
- Every boss file's top-of-file `enum Spells` / `enum Events` / `enum Phases` / `enum Actions` was probed; section 3 reflects the actual phase counts found:
  - Marrowgar: events only, no phase enum (storm is sub-state via aura)
  - Lady Deathwhisper: 2 phases (`PHASE_ONE`/`PHASE_TWO`)
  - Saurfang: phases enum + actions enum + misc enum + movepoints enum (all confirmed)
  - Putricide: 3 combat phases
  - Sindragosa: ground / air / 35% stacking phase (third reflected via `_isThirdPhase` bool)
  - Lich King: `PHASE_ONE` / `PHASE_TWO` / `PHASE_THREE` + transitions, plus Frostmourne sub-state. The `PHASE_TWO_THREE` macro alias confirmed at `boss_the_lich_king.cpp:295`
- Hard-mode buff stacks: verified at `instance_icecrown_citadel.cpp:177` — `SPELL_STRENGHT_OF_WRYNN` (alliance) / `SPELL_HELLSCREAMS_WARSONG` (horde) gated on `DATA_LADY_DEATHWHISPER == DONE && DATA_ICECROWN_GUNSHIP_BATTLE != DONE` precondition.
- Faction selection: `DATA_TEAM_IN_INSTANCE` consumed at gunship spawn (lines 962-963), Saurfang event NPC selection (line 358), and player-enter buff (line 177) — all consistent.
- Sindragosa frostwyrm intro: `DATA_SINDRAGOSA_FROSTWYRMS` decrements via `boss_sister_svalna.cpp` and `instance_icecrown_citadel.cpp:429-447`. Verified.
- Im-On-A-Boat counter (`ACTION_SHIP_VISITS = 5`) confirmed at `boss_icecrown_gunship_battle.cpp:214,1825,2204`.
- Boned achievement counter (`DATA_BONED_ACHIEVEMENT`) verified at `boss_lord_marrowgar.cpp:177,471-473`.
- Defile spell IDs (`SPELL_DEFILE = 72762`, `SPELL_DEFILE_AURA = 72743`, `SPELL_DEFILE_GROW = 72756`) verified at `boss_the_lich_king.cpp:113-115`.
- Frostmourne teleport pair (`SPELL_HARVEST_SOUL_TELEPORT = 72546` / `SPELL_HARVEST_SOUL_TELEPORT_BACK = 72597`) verified at lines 139-141 of `boss_the_lich_king.cpp`.
- Rust state confirmed empty: `wc -l crates/wow-scripts/src/lib.rs` = 0; `wc -l crates/wow-script/src/lib.rs` = 0. No grep hits for any ICC NPC ID / spell ID / `DATA_LORD_MARROWGAR` symbol anywhere under `crates/`.

**Audit verdict:** ✅ complete. Doc fidelity to C++ canonical source is high; Rust state is "empty stub crate" with zero ported content. No discrepancies between this doc and the C++ tree as of 2026-05-01.

---

*Template version: 1.0 (2026-05-01).*
