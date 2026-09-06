# Migration: Scripts — Ulduar

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/`
> **Rust target crate(s):** `crates/wow-scripts/` (content layer; engine in `wow-script`)
> **Layer:** L8 (content / boss scripts; depends on L7 dispatch + AI base + spell-aura + vehicle)
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete (file inventory + per-boss enum/phase audit, 2026-05-01)
> **Last updated:** 2026-05-01

Cross-links: [scripts.md](scripts.md), [scripting.md](scripting.md), [ai.md](ai.md) (`BossAI` / `VehicleAI`), [spells.md](spells.md). The `_attic/STATUS_MIGRATION.md` brief in `crates/wow-world/_attic/` lists the dispatch stubs a content port plugs into.

The sibling `Northrend/Ulduar/HallsOfLightning/` and `HallsOfStone/` directories are 5-mans, but the C++ tree places them under the same `scripts/Northrend/Ulduar/` prefix. R2 keeps them covered here until a later split creates dedicated dungeon-content docs; they must not disappear silently.

---

## 1. Purpose

Ulduar is the WotLK 25-/10-man engineering raid (`map_id = 603`). 14 encounters across 4 wings: vehicle-combat opening (Flame Leviathan), Antechamber (Razorscale, Ignis, XT-002), Assembly of Iron, Iron Council, Kologarn, Auriaya, then the four "Keepers" (Hodir, Thorim, Freya, Mimiron, each with hard-mode triggers), then descent (General Vezax, Yogg-Saron with up-to-4-keeper participation), then the optional one-shot Algalon-the-Observer with a per-week 1h timer. The directory is the canonical reference for vehicle-as-boss scripting (Flame Leviathan), watcher-buff aggregation (Hodir), keeper-stack scaling (Yogg-Saron), and per-encounter hard-mode unlocks via loot-mode bitflags (`LOOT_MODE_HARD_MODE_1..4`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `scripts/Northrend/Ulduar/HallsOfLightning/boss_general_bjarngrim.cpp` | 499 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfLightning/boss_ionar.cpp` | 358 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfLightning/boss_loken.cpp` | 222 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfLightning/boss_volkhan.cpp` | 496 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfLightning/halls_of_lightning.h` | 86 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfLightning/instance_halls_of_lightning.cpp` | 126 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/boss_krystallus.cpp` | 183 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/boss_maiden_of_grief.cpp` | 137 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/boss_sjonnir.cpp` | 490 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.cpp` | 726 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.h` | 82 | `prefix` |
| `scripts/Northrend/Ulduar/HallsOfStone/instance_halls_of_stone.cpp` | 141 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_algalon_the_observer.cpp` | 1227 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_assembly_of_iron.cpp` | 870 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_auriaya.cpp` | 671 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_flame_leviathan.cpp` | 1804 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_freya.cpp` | 1694 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_general_vezax.cpp` | 600 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_hodir.cpp` | 1092 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_ignis.cpp` | 508 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_kologarn.cpp` | 674 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp` | 2791 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_razorscale.cpp` | 1733 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | 2121 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_xt002.cpp` | 1002 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp` | 3172 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/instance_ulduar.cpp` | 1009 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/ulduar.cpp` | 80 | `prefix` |
| `scripts/Northrend/Ulduar/Ulduar/ulduar.h` | 526 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/scripts/Northrend/Ulduar/Ulduar/ulduar.h` | 19 581 chars (~470 lines) | Shared header: data-slot enum, NPC/GO IDs, achievement keys (`DATA_UNBROKEN = 29052906`), Algalon despawn world-state |
| `src/server/scripts/Northrend/Ulduar/Ulduar/ulduar.cpp` | ~70 lines | Script-loader entrypoint registering all boss + spell scripts |
| `src/server/scripts/Northrend/Ulduar/Ulduar/instance_ulduar.cpp` | ~1300 lines | `InstanceMapScript` — encounter state, Algalon weekly summon timer, sigil door state, keeper-count for Yogg, Freya elder lifecycle, Mimiron tram + computer state, hard-mode counters for Iron Council |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_flame_leviathan.cpp` | 1841 lines (64 461 bytes) | Vehicle-only fight: 4 towers (Storms/Flames/Frost/Life) gate hard-mode loot bits. `boss_flame_leviathan_seat`, defense-cannon turret, `_overload_device`, `_safety_container`. Pursuit fixate, Searing Flame, Pyrite ammo |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_razorscale.cpp` | 1750 lines (57 960 bytes) | 5 phases: NONE / COMBAT / GROUND / AIR / PERMA_GROUND. Harpoons (4 turrets) ground her, fuse-armor stack tank-swap |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_ignis.cpp` | ~600 lines | Slag pot grip, Scorch flame patches, Construct adds → Brittle in slag pot → Shattered. Flame Jets |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_xt002.cpp` | ~900 lines | 2 phases (P1 + Heart phase via submerge). Searing Light + Gravity Bomb adds, Tympanic Tantrum, Heart of the Deconstructor → hard-mode unlock if heart killed before re-emerge |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_assembly_of_iron.cpp` | ~1050 lines | 3 sub-bosses, `DATA_PHASE_3` = "all 3 dead" achievement. Steelbreaker (high physical hits, Fusion Punch), Runemaster Molgeim (rune of power buff disc, summoner), Stormcaller Brundir (overload, lightning whirl, lightning tendrils) |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_kologarn.cpp` | ~770 lines | Two-arm encounter: left/right arm with separate HP, Eye Beam roving laser, Stone Grip, Shockwave, arm respawn, Disarmed achievement |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_auriaya.cpp` | ~620 lines | Sentinel intro phase, Sanguine Swarm, Sonic Screech tank cleave, Defender of the Brood (sanctuary), Crazy Cat Lady achievement |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_hodir.cpp` | ~1100 lines | Friendly-NPC buff system (priest/druid/mage/shaman/dk/warrior — 4 random per pull). Frozen Blows, Flash Freeze, Icicles, Biting Cold, Toasty Fires (mage / fire-mage), Storm Cloud (shaman). Hard-mode = kill before any helper dies. Cheese The Freeze achievement |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | 2151 lines (76 523 bytes) | 2 phases: arena gauntlet + open boss. PHASE_NULL / PHASE_1 / PHASE_2. Pre-boss arena waves (warbringer / commoner / death-knight / acolyte / dark-rune-evoker / champion / mercenary / captain), gauntlet rooms (Runic Colossus + Rune Giant), Sif RP, hard-mode = kill within 3min of intro |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_freya.cpp` | 1722 lines (61 118 bytes) | 3 elders (Brightleaf / Stonebark / Ironbranch). Each alive elder = +1 buff for Freya. Adds: Detonating Lasher, Ancient Conservator, Storm Lasher, Snaplasher, Ancient Water Spirit. 0/1/2/3 elders = 4 different difficulty tiers (loot-mode bits). Knock On Wood / Getting Back to Nature achievements |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp` | 3132 lines (111 464 bytes) | 4 forms: Leviathan MK-II, VX-001, Aerial Command Unit, V-07-TR-0N (combined). `PHASE_LEVIATHAN_MK_II`, `PHASE_VX_001`, `PHASE_AERIAL_COMMAND_UNIT`, `PHASE_VOL7RON`. Self-destruct hard-mode, Computer/Tram NPC, Plasma Blast, Frost Bomb, Heat Wave, P3 Magnetic Core |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_general_vezax.cpp` | ~600 lines | Mark of the Faceless, Searing Flames, Surge of Darkness, Saronite Vapors. Hard-mode = let Saronite Animus spawn (don't kill all vapors), then kill animus + Vezax. Unloads `LOOT_MODE_HARD_MODE_1` |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp` | 3325 lines (122 311 bytes, **largest single boss in WotLK**) | 4 fight phases (PHASE_ONE Sara RP, PHASE_TRANSFORM, PHASE_TWO Tentacles + Brain portals, PHASE_THREE Yogg himself). Sanity stack debuff `SPELL_SANITY = 63050`, Insane stack `SPELL_INSANE = 63120`, sanity well objects, illusions (Stormwind / Ice Crown / Chamber of Aspects), hard-mode = N keepers active (0/1/2/3/4 → 5 difficulty tiers), Drive Me Crazy achievement |
| `src/server/scripts/Northrend/Ulduar/Ulduar/boss_algalon_the_observer.cpp` | ~1140 lines | Optional, weekly. Per-week 1h timer summon. Phase-Punch (4-stack tank death), Cosmic Smash, Living Constellation + Constellation Phase, Black Hole + Worm Hole, Big Bang phase RP, Phase 2 at 20% with Ascending Constellation. He Feeds On Your Tears achievement (no deaths). Unbroken (`DATA_UNBROKEN = 29052906`) gates Starcaller / The Astral Walker title |

(`boss_ignis.cpp`, `boss_general_vezax.cpp`, `boss_kologarn.cpp`, `boss_auriaya.cpp` line counts paraphrased from byte size + sampling; section-13 audit lists them more precisely.)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `instance_ulduar::instance_ulduar_InstanceMapScript` | class | 14-encounter state, Algalon weekly counters, Yogg keeper count, Freya elder count, Mimiron sub-form data, Iron Council `DATA_PHASE_3` flag |
| `boss_flame_leviathan` + 4 helper classes | class | Main vehicle + cannon turret + overload device + safety container + seat AI |
| `boss_<name>AI` (14× principal) | struct | One per encounter |
| `boss_steelbreaker` / `boss_runemaster_molgeim` / `boss_stormcaller_brundir` | sub-bosses | Three "Iron Council" / "Assembly of Iron" members |
| `boss_elder_brightleaf` / `boss_elder_stonebark` / `boss_elder_ironbranch` | sub-bosses | Freya pre-encounter elders |
| `boss_leviathan_mk_ii` / `boss_vx_001` / `boss_aerial_command_unit` | sub-bosses | Mimiron forms (each is its own NPC entry) |
| `boss_voice_of_yogg_saron` / `boss_sara` / `boss_brain_of_yogg_saron` | sub-bosses | Yogg-Saron's three NPCs (Voice = Phase 1, Sara = transform, Brain = Phase 2 portal target) |
| `npc_freya_ys` / `npc_hodir_ys` / `npc_thorim_ys` / `npc_mimiron_ys` | helpers | Yogg-Saron keeper instances; their presence = +1 to `DATA_KEEPERS_COUNT` |
| `npc_living_constellation` / `npc_black_hole` / `npc_worm_hole` / `npc_unleashed_dark_matter` | helpers | Algalon adds |
| `npc_demolisher` / `npc_siege_engine` / `npc_chopper` | vehicles | Flame Leviathan player vehicles |
| `Spells` / `Events` / `Phases` / `Actions` (per-boss enums) | local | Per-boss script state (audited in section 13) |
| `LOOT_MODE_DEFAULT / HARD_MODE_1..4` | bitflags | 5 loot tiers gated on hard-mode predicates (Leviathan towers, XT heart, Hodir helper-deaths, Mimiron self-destruct, Vezax animus, Freya elders alive, Yogg keepers active, Thorim 3min timer, Algalon special) |
| `DATA_UNBROKEN = 29052906` | enum constant | Composite achievement key (2905 + 2906) for Algalon kill without deaths |

Yogg-Saron-specific spells (verified):

- `SPELL_SANITY = 63050`, `SPELL_SANITY_PERIODIC = 63786`, `SPELL_INSANE = 63120`, `SPELL_INSANE_PERIODIC = 64554`, `SPELL_SANITY_WELL = 64169`, `SPELL_SANITY_WELL_VISUAL = 63288`, `SPELL_SANITY_WELL_SUMMON = 64170`, `SPELL_PHASE_2_TRANSFORM = 65157`, `SPELL_PHASE_3_TRANSFORM = 63895`, `SPELL_KEEPER_ACTIVE = 62647`, `SPELL_SIMPLE_TELEPORT_KEEPERS = 12980`.

Flame-Leviathan tower buffs (verified):

- `SPELL_BUFF_TOWER_OF_STORMS = 65076`, `SPELL_BUFF_TOWER_OF_FLAMES = 65075`, `SPELL_BUFF_TOWER_OF_FR0ST = 65077`, `SPELL_BUFF_TOWER_OF_LIFE = 64482`. Towers map to `GO_TOWER_OF_STORMS = 194377`, `GO_TOWER_OF_FLAMES = 194371`, `GO_TOWER_OF_FROST = 194370`, `GO_TOWER_OF_LIFE = 194375`.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `BossAI::JustEngagedWith` (override per boss) | Open-pull state, schedule events, faction-buff fan-out | `events.ScheduleEvent`, `instance->SetBossState` |
| `BossAI::Reset` | Drop summons, reset sub-state. Razorscale resets harpoons, Mimiron resets to MK-II form, Yogg resets sanity stacks on every player | per-boss |
| `BossAI::DamageTaken` (Razorscale, Mimiron, XT, Yogg, Vezax, Algalon) | Phase transitions on HP%; Vezax 33% adds spawn | `events.SetPhase` |
| `BossAI::DoAction(int32)` | Hard-mode triggers — every boss has at least one `ACTION_START_HARD_MODE` / `DO_ACTIVATE_HARD_MODE` | various |
| `boss_flame_leviathan::Reset` | Restore `ActiveTowersCount = 4`, reset 4 tower bools, reset loot mode | `me->SetLootMode` |
| `boss_flame_leviathan::DoAction(ACTION_START_HARD_MODE)` | Mark all 4 towers as not-yet-killed → enables `LOOT_MODE_HARD_MODE_1..4` | `me->SetLootMode(DEFAULT \| HARD_MODE_1..4)` |
| `boss_flame_leviathan_seat::PassengerBoarded` | Vehicle hand-off: cannon seat ↔ pilot seat | `Vehicle::AddPassenger` / `RemovePassenger` |
| `boss_voice_of_yogg_saron::DoAction(ACTION_PHASE_TRANSFORM/TWO/THREE)` | Yogg phase machine driver | `events.SetPhase`, summon tentacles, trigger Brain portals |
| `boss_brain_of_yogg_saron::DoAction(ACTION_INDUCE_MADNESS)` | Wipe trigger when Brain is reached and Sara not killed | `instance->SetBossState(FAIL)` |
| `boss_brain_of_yogg_saron::DoAction(ACTION_TENTACLE_KILLED)` | Brain HP step on tentacle kill (tracking via summon list size) | `me->ModifyHealth` |
| `instance_ulduar::SetData(DATA_KEEPERS_COUNT, ...)` | Increment keeper count when each freya_ys / hodir_ys / thorim_ys / mimiron_ys spawns | data slot |
| `instance_ulduar::GetData(DATA_KEEPERS_COUNT)` | Yogg AI reads this to gate hard-mode (`< 4 in 25-man = no hardmode`) | — |
| `instance_ulduar::GetData(DATA_UNBROKEN)` | Algalon achievement check (no deaths in fight) | — |
| `boss_thorim::DoAction(ACTION_START_HARD_MODE)` | Set `DATA_THORIM_HARDMODE = 1`, scale boss HP, replace Sif | instance-data |
| `boss_hodir::Talk(SAY_HARD_MODE_FAILED)` | Emit RP line when first helper dies (hard-mode lost) | creature_text |
| `boss_freya::elderCount` access | Read alive-elder count to pick `summonSpell[difficulty][elderCount]` row | per-boss |
| `boss_mimiron::DoAction(DO_ACTIVATE_HARD_MODE)` | Self-destruct button activation; sets `_hardMode` flag on instance | — |
| `boss_general_vezax::JustSummoned(Saronite Animus)` | Adds `LOOT_MODE_HARD_MODE_1` if animus reaches arena | `me->AddLootMode` |
| `SpellScript`s | ~40 in Ulduar across all bosses | `wow-spell` registry |

---

## 5. Module dependencies

**Depends on:**
- `wow-ai` — `BossAI`, `ScriptedAI`, `PassiveAI` (Brain of Yogg-Saron uses `PassiveAI`), `EventMap`, `SummonList`. See [ai.md](ai.md).
- `wow-spell` — `SpellScript` / `AuraScript`, target selectors, proc hooks (Algalon Phase Punch is a tank-swap mechanic via aura stack count). See [spells.md](spells.md).
- `wow-combat` — Threat list, `Attack`, `DoCastVictim`.
- `wow-world` — `Vehicle` integration is a **hard pre-req** for Flame Leviathan, Razorscale harpoons, Hodir Toasty-Fires GO interaction, Mimiron tram, Algalon constellation. The current `wow-world` has no vehicle host — see entities-vehicle.md.
- `wow-data` — `Map.db2` (map 603), `MapDifficulty.db2`, `VehicleSeat.db2`, `Spell*.db2`, `Achievement.db2`.
- `wow-database` — `creature`, `gameobject`, `creature_template_addon` (vehicle seats), `instance_template`, `instance_encounters`, `gameobject_addon` for tower GOs.
- `wow-loot` — `LOOT_MODE_*` bitflag system. Multiple Ulduar bosses are *the* reference implementation for this.
- `wow-script` (engine) — `ScriptMgr`, `RegisterCreatureAI`, inventory-based registration. See [scripts.md](scripts.md).
- `wow-achievement` — Many Ulduar achievement criteria are script-driven (Crazy Cat Lady on `JustDied`, Knock On Wood on `JustSummoned`, Drive Me Crazy on aura-application, He Feeds On Your Tears on `JustDied` + zero-deaths).

**Depended on by:**
- `world-server` binary loads instance via map manager.
- The Algalon weekly summon timer hooks into the realm-level event scheduler — a per-week tick the realm DB persists.

---

## 6. SQL / DB queries

Like ICC, scripts mutate runtime state; SQL touches happen via the engine.

| Source | Purpose | DB |
|---|---|---|
| `instance_template` row map 603 | Bind `instance_ulduar` script | world |
| `instance_encounters` map 603 | 14 boss slots → DBC encounter IDs | world |
| `creature` / `gameobject` rows | All NPC + tower + sigil-door + tram + Algalon platform spawns | world |
| `creature_template_addon` | Vehicle seats: demolisher (4 seats), siege engine (4 seats), chopper (2 seats), Razorscale harpoon turret, Mimiron MK-II / VX-001 / ACU forms, Algalon constellation | world |
| `creature_text` | All `Talk()` lines (Hodir hard-mode-failed, Algalon Big Bang RP, Yogg Sara Phase 1 mood lines) | world |
| `spawn_group_template` | Pre-Thorim arena gauntlet waves | world |
| `script_waypoint` | Razorscale harpoon-aim waypoints, Algalon entry RP path | world |
| `pool_template` (Algalon weekly state) | Algalon Despawn world-state `WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131` | world |
| `achievement_criteria_data` for ~30 Ulduar achievements | `He Feeds On Your Tears`, `Knock On Wood`, `Drive Me Crazy`, `Crazy Cat Lady`, `Cheese The Freeze`, `Disarmed`, `Set Up Us The Bomb`, `Lose Your Illusion`, `Heartbreaker` | character / world |

DBC/DB2 stores:

| Store | What it loads | Read by |
|---|---|---|
| `Map.db2` | Map 603 metadata | `MapManager` |
| `Spell*.db2` | ~250 spells these scripts touch | `SpellMgr` |
| `Vehicle.db2` / `VehicleSeat.db2` | Demolisher, siege, chopper, MK-II, VX-001, ACU, harpoon turret, Algalon constellation, Razorscale, gunship analogues | `Vehicle::Initialize` |
| `Achievement.db2` + `Criteria.db2` | All Ulduar metas + Glory of the Ulduar Raider | `AchievementMgr` |

---

## 7. Wire-protocol packets (if any)

Same opcodes as ICC; nothing new is defined.

| Opcode | Direction | Sent in |
|---|---|---|
| `SMSG_INSTANCE_ENCOUNTER_*` (engage/disengage frames) | s→c | Every `BossAI::JustEngagedWith` / `JustDied` |
| `SMSG_UPDATE_WORLD_STATE` | s→c | Algalon despawn timer (`WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131`), Flame Leviathan tower count, Mimiron self-destruct timer |
| `SMSG_SPELL_GO` / `SMSG_SPELL_START` | s→c | Every scripted DoCast |
| `SMSG_AURA_UPDATE` | s→c | Sanity stacks (Yogg), Phase Punch stacks (Algalon), Fusion Punch stack (Steelbreaker), Permeating Chill (Iron Council) |
| `SMSG_VEHICLE_BOARD` / `SMSG_PLAYER_VEHICLE_DATA` / `SMSG_CONTROL_VEHICLE` | s↔c | Flame Leviathan vehicles, Mimiron forms, Razorscale harpoons, Algalon constellation seats |
| `SMSG_MOVE_SPLINE_*` | s→c | Razorscale dive, Mimiron VX-001 transform, Algalon Cosmic Smash projectile, Living Constellation drift |
| `SMSG_RAID_BOSS_EMOTE` | s→c | "Hodir conjures a glaring storm cloud!", "Razorscale reaches the ground!", etc. |
| `SMSG_PLAY_MUSIC` | s→c | Algalon RP, Mimiron transform, Yogg Phase transitions |

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
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-scripts/src/lib.rs` — **0 bytes**.
- `crates/wow-script/src/lib.rs` — **0 bytes**.
- No instance script, no boss AI, no Ulduar NPC ID anywhere. Verified: `grep -r 'NPC_VOICE_OF_YOGG\|DATA_FLAME_LEVIATHAN\|FlameLeviathan\|Mimiron\|Algalon\|YoggSaron\|boss_freya' crates/` produces zero hits.

**What's implemented:**
- Nothing. Ulduar content is **0% ported**.

**What's missing vs C++:**
- Full encounter set (14 bosses + 3 elders + 4 keepers + ~30 helper NPCs + ~40 spell scripts + Mimiron's 4 forms).
- Vehicle integration (mandatory pre-req for Flame Leviathan, Razorscale, Mimiron, Algalon).
- Loot-mode bitflag system for hard-mode loot tables.
- Algalon weekly summon timer + 1h despawn world-state.
- Watcher buff system (Hodir helpers) — needs friendly-NPC AI that buffs raid members based on their class spec.
- Keeper gauntlet (Yogg-Saron) — needs cross-instance state sync (which keepers were saved during their respective fights).
- Sanity stacks debuff system (Yogg-Saron) with periodic ticking, well-aura cleanse, illusion-room map transitions.
- Achievement criteria fan-out: ~30 distinct script-driven criteria.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — empty crate. Pre-port suspicions for the future:
  - Yogg-Saron Sanity is *not* a normal aura — it's a stack debuff that ticks via a periodic spell on each player and is applied/removed by ~10 different sources. The aura system in `wow-spell` must support `Aura::ModStackAmount(int32, AuraRemoveMode)` faithfully or this fight is wrong.
  - Flame Leviathan tower mechanic relies on `LOOT_MODE_HARD_MODE_*` bits being **removed** as towers die, not added. A naive port that adds bits as conditions improve will give the wrong loot.
  - Mimiron's 4 forms are 4 different `Creature` rows linked by instance data; the boss "frame" in the UI has to migrate across NPCs.
  - Algalon's per-week timer is a realm-level scheduler, not an instance-local timer.
  - Vehicle seats for Flame Leviathan use unusual passive seat configurations (cannon vs pilot) — must round-trip through `Vehicle.db2` exactly.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SCRIPTS_ULDUAR.WBS.001** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/boss_general_bjarngrim.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/boss_general_bjarngrim.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.002** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/boss_ionar.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/boss_ionar.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.003** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/boss_loken.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/boss_loken.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.004** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/boss_volkhan.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/boss_volkhan.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.005** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/halls_of_lightning.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/halls_of_lightning.h`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.006** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfLightning/instance_halls_of_lightning.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfLightning/instance_halls_of_lightning.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.007** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/boss_krystallus.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/boss_krystallus.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.008** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/boss_maiden_of_grief.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/boss_maiden_of_grief.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.009** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/boss_sjonnir.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/boss_sjonnir.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.010** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 726 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.011** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/halls_of_stone.h`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.012** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/HallsOfStone/instance_halls_of_stone.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/HallsOfStone/instance_halls_of_stone.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.013** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_algalon_the_observer.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_algalon_the_observer.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1227 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.014** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_assembly_of_iron.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_assembly_of_iron.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 870 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.015** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_auriaya.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_auriaya.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 671 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.016** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_flame_leviathan.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_flame_leviathan.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1804 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.017** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_freya.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_freya.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1694 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.018** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_general_vezax.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_general_vezax.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 600 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.019** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_hodir.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_hodir.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1092 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.020** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_ignis.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_ignis.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 508 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.021** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_kologarn.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_kologarn.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 674 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.022** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2791 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.023** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_razorscale.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_razorscale.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1733 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.024** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2121 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.025** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_xt002.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_xt002.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1002 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.026** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3172 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.027** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/instance_ulduar.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/instance_ulduar.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1009 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.028** Cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/ulduar.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/ulduar.cpp`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTS_ULDUAR.WBS.029** Partir y cerrar la migracion auditada de `scripts/Northrend/Ulduar/Ulduar/ulduar.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/ulduar.h`
  Rust target: `crates/wow-scripts`, `crates/wow-script`, `crates/wow-spell`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 526 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Pre-requisites (shared with [scripts-icc.md](scripts-icc.md) #ICC.0a..0e):

- [ ] **#ULD.0a** Vehicle integration in `wow-world`: `Vehicle` struct, seat allocation, `AddPassenger`/`RemovePassenger`, control-vehicle opcode flow (XL — blocks Flame Leviathan, Razorscale, Mimiron, Algalon)
- [ ] **#ULD.0b** Loot-mode bitflag system in `wow-loot`: `LootMode { DEFAULT, HARD_MODE_1, HARD_MODE_2, HARD_MODE_3, HARD_MODE_4 }`, per-creature add/remove (M)
- [ ] **#ULD.0c** Realm-level weekly scheduler: register a tick that resets Algalon-summonable state every Tuesday-reset + 1h Algalon despawn timer (M)

Per-boss (~12 each; complexity scales with line count):

**Flame Leviathan (1841 lines, XL — split)**
- [ ] **#ULD.FL.1** Vehicle host AI skeleton + chase/pursue mechanic (`SPELL_PURSUED`) (H)
- [ ] **#ULD.FL.2** Tower predicate state: 4 bools (`towerOfStorms/Flames/Frost/Life`), `ActiveTowersCount`, populated from instance pre-pull (M)
- [ ] **#ULD.FL.3** Tower buff cast on engage: each alive tower → `SPELL_BUFF_TOWER_OF_*` on Leviathan (M)
- [ ] **#ULD.FL.4** Hard-mode action: `ACTION_START_HARD_MODE` adds `LOOT_MODE_HARD_MODE_1..4`, also re-applies all 4 tower buffs (M)
- [ ] **#ULD.FL.5** Loot-mode strip on tower-killed-during-fight: at 4/3/2/1 active towers, remove `HARD_MODE_4/3/2/1` respectively (M)
- [ ] **#ULD.FL.6** `boss_flame_leviathan_seat` cannon/pilot dual-seat AI (M)
- [ ] **#ULD.FL.7** `boss_flame_leviathan_defense_cannon` + `_defense_turret` AOE NPCs (M)
- [ ] **#ULD.FL.8** `_overload_device` + `_safety_container` interactable mechanics (M)
- [ ] **#ULD.FL.9** Player vehicle scripts: demolisher pyrite cannon, siege-engine ramp, chopper aggro generator (H)
- [ ] **#ULD.FL.10** Searing Flame, Pyrite Pursued chase, EMP fire (M)
- [ ] **#ULD.FL.11** Battery enrage at 10min (L)
- [ ] **#ULD.FL.12** `Orbit-uary` / `Three Car Garage` / `A Quick Shave` / `Take Out Those Turrets` / `Unbroken` achievements wired (M)

**Razorscale (1750 lines, H)**
- [ ] **#ULD.RZ.1..12** 5-phase machine, harpoon turret vehicle, fuse-armor stack, devouring flame, fireball volley, ground/perma-ground transition. (~12 sub-tasks each M-H)

**Ignis (~600 lines, M)** — `#ULD.IG.1..12` (slag-pot cycle, scorch patches, construct adds, brittle, shattered, jets)

**XT-002 (~900 lines, H)** — `#ULD.XT.1..12`
- 2 phases (P1 / Heart submerge), Searing Light + Gravity Bomb adds, Tympanic Tantrum, Heart of the Deconstructor hard-mode unlock (M each, 1 H for heart-phase loot-mode toggle)

**Assembly of Iron / Iron Council (~1050 lines, H)** — `#ULD.AI.1..12`
- 3 sub-bosses, kill-order-dependent buffs (Steelbreaker last = phase 3), Fusion Punch / Rune of Power / Lightning Tendrils, `DATA_PHASE_3` flag, I Choose You achievement

**Kologarn (~770 lines, M)** — `#ULD.KG.1..12`
- Two-arm separate HP, eye beam roving, stone grip, shockwave, arm respawn, Disarmed achievement

**Auriaya (~620 lines, M)** — `#ULD.AU.1..12`
- Sentinel intro, Sanguine Swarm, Sonic Screech, Defender of the Brood, Crazy Cat Lady

**Hodir (~1100 lines, H)** — `#ULD.HD.1..12`
- Friendly-NPC pool (priest/druid/mage/shaman/dk/warrior — 4 random), Frozen Blows enrage, Flash Freeze (room-wide AoE → must be in toasty fire), Icicles spawn pattern, Biting Cold periodic, Storm Cloud + Toasty Fire interactable GOs (helper-cast). Hard-mode = no helper deaths, achievements: Cheese The Freeze, I Have The Coolest Friends, I Could Say That This Cache Was Rare

**Thorim (2151 lines, XL — split)**
- [ ] **#ULD.TH.1..6** Arena phase: gauntlet waves through twin chambers, Runic Colossus + Rune Giant mini-encounters, Sif RP, lever activation (each M-H)
- [ ] **#ULD.TH.7..12** Open boss phase: Charged Sphere (lightning orbs for tank-DPS), Stormhammer, Lightning Charge, Berserk, hard-mode triggered by reaching boss < 3min, Lose Your Illusion / I'll Take You All On / Siffed achievements (each M-H)

**Freya (1722 lines, XL — split)**
- [ ] **#ULD.FR.1..4** 3 elders pre-encounter (`boss_elder_brightleaf` / `_stonebark` / `_ironbranch`) — each H, killable in any combo (M)
- [ ] **#ULD.FR.5..8** Freya main fight: 3 add waves rotating (Detonating Lasher, Ancient Conservator, Storm Lasher / Snaplasher / Ancient Water Spirit), Sun Beam, Eonar's Gift heal (M-H each)
- [ ] **#ULD.FR.9..12** Hard-mode tier (3 elders alive = 3-add-wave with all add types same time) + scaled spells via `summonSpell[difficulty][elderCount]` table; achievements: Knock On Wood, Knock Knock Knock On Wood, Getting Back to Nature

**Mimiron (3132 lines, XL — biggest split, 4 sub-fights)**
- [ ] **#ULD.MM.1** Computer NPC + tram interactable + button GO (`DO_ACTIVATE_HARD_MODE`) (M)
- [ ] **#ULD.MM.2..5** Phase 1 — Leviathan MK-II form: Plasma Blast tank-swap, Napalm Shell, Shock Blast, Proximity Mines (M-H)
- [ ] **#ULD.MM.6..8** Phase 2 — VX-001 form: Heat Wave AoE, Spinning Up rotational laser, Rocket Strike, Frost Bomb (H)
- [ ] **#ULD.MM.9..10** Phase 3 — Aerial Command Unit: Magnetic Core swap-to-ground, Plasma Ball periodic (M)
- [ ] **#ULD.MM.11** Phase 4 — V-07-TR-0N combined form (all 3 sub-bosses up simultaneously) (H)
- [ ] **#ULD.MM.12** Self-destruct hard-mode: 9-minute room-wide nuke timer if button pressed; achievements Set Up Us The Bomb, Firefighter (M)

**General Vezax (~600 lines, M)** — `#ULD.GV.1..12`
- Mark of the Faceless, Searing Flames, Surge of Darkness, Saronite Vapors at 33%, Saronite Animus hard-mode (don't kill all vapors → animus spawns at 30%), achievements: Smell Saronite And Die, Two Lights In The Darkness

**Yogg-Saron (3325 lines, XL — biggest single split)**
- [ ] **#ULD.YS.1** Sanity-stack debuff system (`SPELL_SANITY = 63050` periodic apply, `SPELL_INSANE = 63120` on 0 stacks, sanity well GOs cleanse) (XL)
- [ ] **#ULD.YS.2** Phase 1 — Voice + Sara: Guardian of Yogg-Saron summons, Sara's Anger / Blessing / Fervor mood-tracking, mind-control on Sara aggro (H)
- [ ] **#ULD.YS.3** Phase 1 wipe trigger: `boss_voice_of_yogg_saron::WHISPER_VOICE_PHASE_1_WIPE` (M)
- [ ] **#ULD.YS.4** Transform RP: `SPELL_PHASE_2_TRANSFORM = 65157`, 4 timed events (M)
- [ ] **#ULD.YS.5** Phase 2 — Tentacles: Corruptor (slime cone), Constrictor (grip), Crusher (ground-pound), summon scheduling (H)
- [ ] **#ULD.YS.6** Phase 2 — Brain portals: 3 illusion rooms (Stormwind / Ice Crown / Chamber of Aspects), in-portal mind-rape mechanic, brain HP only ticks down on tentacle kills inside portal (XL)
- [ ] **#ULD.YS.7** Phase 3 transform: `SPELL_PHASE_3_TRANSFORM = 63895`, Immortal Guardian summons (H)
- [ ] **#ULD.YS.8** Phase 3 — Yogg-Saron himself: Lunatic Gaze, Shadow Beacon, Empowering Shadows (H)
- [ ] **#ULD.YS.9** Keeper-count hard-mode tiers: 0/1/2/3/4 keepers active = 5 difficulty modes; `DATA_KEEPERS_COUNT` consumed by Yogg AI to pick scaled values (H)
- [ ] **#ULD.YS.10** Per-keeper buff fan-out: Freya (Resilience of Nature), Hodir (Storm Power), Thorim (Fury of the Storm), Mimiron (Speed of Invention) (M)
- [ ] **#ULD.YS.11** Sanity Wells GO interactable (`npc_sanity_well`) — heal sanity on click (M)
- [ ] **#ULD.YS.12** Achievements: Drive Me Crazy, He's Not Getting Any Older, In His House He Waits Dreaming, They're All Dead (M)

**Algalon (~1140 lines, H)**
- [ ] **#ULD.AL.1** Weekly summon timer + 1h despawn world-state (`WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131`) (M, depends on #ULD.0c)
- [ ] **#ULD.AL.2** Sigil-door progression (`DATA_SIGILDOOR_01..03`), Universe-floor-globe state (M)
- [ ] **#ULD.AL.3** Brann Bronzebeard intro RP (`DATA_BRANN_BRONZEBEARD_ALG`) + Lore Keeper of Norgannon (M)
- [ ] **#ULD.AL.4** Phase Punch: 4-stack tank death, tank-swap requirement (M)
- [ ] **#ULD.AL.5** Cosmic Smash projectile mechanic (M)
- [ ] **#ULD.AL.6** Living Constellation + Constellation Phase swap (`SPELL_CONSTELLATION_PHASE_TRIGGER = 65508`, `SPELL_CONSTELLATION_PHASE_EFFECT = 65509`) (H)
- [ ] **#ULD.AL.7** Black Hole + Worm Hole spawn pair (M)
- [ ] **#ULD.AL.8** Big Bang phase RP `SAY_ALGALON_PHASE_TWO` — escape into worm hole or die (H)
- [ ] **#ULD.AL.9** Phase 2 at 20%: Ascending Constellation enrage (M)
- [ ] **#ULD.AL.10** `DATA_GIFT_OF_THE_OBSERVER` + `DATA_AZEROTH` post-fight RP, Gift of the Observer item drop (M)
- [ ] **#ULD.AL.11** `DATA_UNBROKEN = 29052906` achievement: composite key for Starcaller / Astral Walker title (no deaths in fight) (M)
- [ ] **#ULD.AL.12** Despawn-on-1h-elapsed RP (`SAY_ALGALON_DESPAWN`) (L)

**Instance / shared content**
- [ ] **#ULD.INST.1** `instance_ulduar` script — 14 boss slots, ~40 auxiliary data slots (H)
- [ ] **#ULD.INST.2** Algalon weekly summonable state, sigil-door progression (M)
- [ ] **#ULD.INST.3** Mimiron sub-form GUID tracking (`DATA_LEVIATHAN_MK_II / VX_001 / AERIAL_COMMAND_UNIT / COMPUTER`) (M)
- [ ] **#ULD.INST.4** Yogg keeper-count + sub-keeper GUIDs (`DATA_FREYA_YS / HODIR_YS / THORIM_YS / MIMIRON_YS`) (M)
- [ ] **#ULD.INST.5** Iron Council `DATA_PHASE_3` flag + kill-order tracking (L)
- [ ] **#ULD.INST.6** Thorim `DATA_THORIM_HARDMODE` + arena waves (M)
- [ ] **#ULD.INST.7** Achievement-flag fan-out + per-encounter clear on Reset (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SCRIPTS_ULDUAR.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 29 files / 25120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`. Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-spell`. | `cargo test -p wow-script && cargo test -p wow-scripts && cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SCRIPTS_ULDUAR.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 29 files / 25120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`. Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-spell`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SCRIPTS_ULDUAR.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 29 files / 25120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`. Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-spell`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SCRIPTS_ULDUAR.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 29 files / 25120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp`. Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-spell`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: Flame Leviathan loot mode bits drop in the right order as towers die mid-fight (4 towers active = `HARD_MODE_4` removed when first dies, etc.)
- [ ] Test: Razorscale phase machine cycles `NONE → COMBAT → AIR → GROUND → AIR → ... → PERMA_GROUND` deterministically; PERMA_GROUND is one-way
- [ ] Test: XT-002 Heart submerge → Heart killed → re-emerge with hard-mode loot bits set; Heart NOT killed in time → re-emerge without bits
- [ ] Test: Iron Council kill-order detection: `DATA_PHASE_3` flag is set iff Steelbreaker is the last to die
- [ ] Test: Hodir Frozen Blows enrage triggers exactly when first helper dies; `SAY_HARD_MODE_FAILED` plays exactly once
- [ ] Test: Hodir Toasty Fire / Storm Cloud GOs are interactable only when alive
- [ ] Test: Thorim hard-mode triggers iff arena → boss transition happens within 3min of pull
- [ ] Test: Freya `summonSpell[difficulty][elderCount]` table correctly indexes 0..3 elders × 2 difficulty (10/25)
- [ ] Test: Mimiron form chain MK-II → VX-001 → ACU → V-07-TR-0N transitions only at scripted HP thresholds; encounter frame migrates across the 4 NPCs
- [ ] Test: Vezax Saronite Animus only spawns if at least one Vapor was left alive at 30%; killing animus + Vezax sets `LOOT_MODE_HARD_MODE_1`
- [ ] Test: Yogg-Saron sanity stack defaults to 100 in 25-man / 100 in 10-man, ticks down 1 per `SPELL_SANITY_PERIODIC` apply, reaches 0 → `SPELL_INSANE` mind-controls the player
- [ ] Test: Yogg `DATA_KEEPERS_COUNT` is the count of `npc_*_ys` GUIDs alive at pull, never updates mid-fight
- [ ] Test: Yogg-Saron Phase 2 brain portal spawns 3 illusion-room versions (Stormwind / Ice Crown / Chamber of Aspects) each pull with random pick — but always 3 distinct
- [ ] Test: Algalon weekly summon timer resets on Tuesday-reset realm-tick; 1h despawn world-state matches `WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131`
- [ ] Test: Algalon Phase Punch tank death at 4 stacks with no taunt-swap reproduces TC behavior
- [ ] Test: `DATA_UNBROKEN = 29052906` achievement only awards if zero deaths during Algalon fight (not during instance wipes)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 29 files / 25120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `crates/wow-scripts/` (content layer; engine in `wow-script`) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SCRIPTS_ULDUAR.DIV.001` | `crates/wow-scripts` (`exists_empty`, 0 Rust lines) | 29 C++ files / 25120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTS_ULDUAR.DIV.002` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 29 C++ files / 25120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTS_ULDUAR.DIV.003` | `crates/wow-scripts/src/lib.rs` (`exists_empty`, 0 Rust lines) | 29 C++ files / 25120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SCRIPTS_ULDUAR.DIV.004` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 29 C++ files / 25120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SCRIPTS_ULDUAR.DIV.005` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 29 C++ files / 25120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_yogg_saron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_mimiron.cpp`, `/home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/boss_thorim.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Vehicle dependency**: Flame Leviathan, Razorscale (harpoon turrets), Mimiron (4 forms are 4 NPCs swapped in), Algalon (constellation seats), and the Hodir Toasty Fire / Storm Cloud objects all require the `wow-world` Vehicle subsystem. None of Ulduar should be merged before vehicles land. (#ULD.0a is the gate.)
- **Loot-mode subtlety**: bits get *removed* as conditions degrade in Flame Leviathan; many other bosses *add* bits when conditions improve (Vezax animus, Mimiron self-destruct, Yogg keeper count). Don't conflate the two patterns.
- **Yogg-Saron Sanity is the most complex aura mechanic in WotLK**. Stack value reaches 100 default and decays. Multiple sources reduce stacks: looking at Crusher, getting hit by Lunatic Gaze, being in tentacle eye-contact range, brain portal mistakes, Insane on 0. Sanity Wells (objects) restore. Aura model in `wow-spell` must support negative stack-mods + per-source attribution.
- **Hodir helper class roster**: 4 of (priest_pred / druid_velanoor / mage_kar / shaman_thoeril / dk_thuella / warrior_baleog). Their AI casts class-appropriate buffs on raid members. This is the only fight in WotLK with friendly-NPC class-fantasy AI; do not skip it.
- **Mimiron's 4 forms are 4 distinct creature entries**. When VX-001 phase begins, MK-II is invisible/passive, VX-001 spawns. Encounter frame must migrate. Reset must despawn all 4. Achievements track which form did damage.
- **Algalon's despawn timer**: server-side, 1 hour from first engage. World-state `4131` is what the client renders. Realm-level tracking required (a server restart inside the timer must persist).
- **Thorim arena trash gauntlet** is part of the boss script, not separate trash. Gauntlet creature templates live in the boss file and are non-trivial (~8 different mob types with class-fantasy spells).
- **Freya `summonSpell[difficulty][elderCount]`** table at `boss_freya.cpp:~620` is the canonical reference for encounter-scaling in Ulduar. Don't hand-rewrite — port verbatim.
- **Unbroken (`DATA_UNBROKEN = 29052906`)** is a composite achievement key encoding `2905` + `2906` (10 / 25 versions). Custom encoding scheme — do not treat as a regular achievement ID.
- **Yogg-Saron is the largest single boss file in WotLK at 3325 lines**. Splitting it into ≥4 sub-modules (Voice / Sara / Tentacles / Brain) before porting is mandatory.
- **`Is25ManRaid()` switches between fundamentally different difficulty rows** in many Ulduar tables; the Freya table indices and Yogg keeper-count ceilings differ by raid size. Cache per-instance.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class boss_flame_leviathan : public CreatureScript` + inner `boss_flame_leviathanAI : BossAI` | `pub struct BossFlameLeviathanAI { ... }` impl `BossAI + VehicleAI` | Composition over inheritance |
| `me->SetLootMode(LOOT_MODE_DEFAULT \| LOOT_MODE_HARD_MODE_1)` | `self.creature.set_loot_mode(LootMode::DEFAULT \| LootMode::HARD_MODE_1)` | bitflags crate |
| `me->RemoveLootMode(LOOT_MODE_HARD_MODE_4)` | `self.creature.remove_loot_mode(LootMode::HARD_MODE_4)` | — |
| `Vehicle::AddPassenger(player, seat)` | `vehicle.add_passenger(&player, seat)` | requires #ULD.0a |
| `events.SetPhase(PHASE_LEVIATHAN_MK_II)` | `events.set_phase(MimironPhase::LeviathanMkIi)` | typed phase enum per boss |
| `instance->GetData(DATA_KEEPERS_COUNT)` | `instance.get(UlduarData::KeepersCount)` returns `u32` | typed slot enum |
| `Aura::ModStackAmount(int32, AuraRemoveMode)` (Sanity reduction) | `aura.mod_stack_amount(delta: i32, mode: AuraRemoveMode)` | core aura API |
| `class npc_living_constellation : public ScriptedAI` | `struct NpcLivingConstellationAI { ... }` impl `ScriptedAI` | — |
| `RegisterCreatureAI(boss_yogg_saron)` | `inventory::submit!(CreatureAiEntry::new::<BossYoggSaronAI>(NPC_YOGG_SARON))` | Inventory dispatch |
| `class boss_voice_of_yogg_saron` (separate NPC, drives Phase 1) | Separate `struct BossVoiceOfYoggSaronAI` registered to `NPC_VOICE_OF_YOGG_SARON` | Yogg's phase machine spans 3 NPCs |
| `DoAction(ACTION_PHASE_TRANSFORM)` | `enum YoggAction { PhaseTransform, PhaseTwo, PhaseThree, ... }` + `fn do_action(&mut self, a: YoggAction)` | typed actions |
| `LOOT_MODE_DEFAULT \| HARD_MODE_1..4` (Flame Leviathan) | `LootMode::all()` then `.remove()` per tower kill | Reverse pattern: start max, strip on degrade |
| `WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131` | `WorldState::AlgalonDespawnTimer = 4131` | broadcast via SMSG_UPDATE_WORLD_STATE |
| `SAY_HARD_MODE_FAILED` (Hodir) | `enum HodirText { HardModeFailed = 6, ... }` mapped to `creature_text` rows | — |
| `me->CastSpell(nullptr, summonSpell[difficulty][elderCount], true)` (Freya scaled adds) | 2D const table `SUMMON_SPELL: [[SpellId; 4]; 2]` indexed by `(diff, elders)` | port verbatim |

---

## 13. Audit (vs C++ source, 2026-05-01)

- File inventory in section 2 cross-checked against `ls -la /home/server/woltk-trinity-legacy/src/server/scripts/Northrend/Ulduar/Ulduar/`. 18 files (16 cpp + 1 h + 1 ulduar.cpp loader) confirmed. Top byte-size files: `boss_yogg_saron.cpp` (122 311 bytes), `boss_mimiron.cpp` (111 464 bytes), `boss_thorim.cpp` (76 523 bytes), `boss_flame_leviathan.cpp` (64 461 bytes), `boss_freya.cpp` (61 118 bytes), `boss_razorscale.cpp` (57 960 bytes), `boss_algalon_the_observer.cpp` (42 076 bytes), `boss_assembly_of_iron.cpp` (34 656 bytes).
- 14-encounter `DATA_*` slot enum (0..13) verified against `ulduar.h:37-50`. Sub-slots (`DATA_BRIGHTLEAF / IRONBRANCH / STONEBARK = 14..16`, `DATA_LEVIATHAN_MK_II / VX_001 / AERIAL_COMMAND_UNIT / COMPUTER = 417..420`, `DATA_VOICE_OF_YOGG_SARON / SARA / BRAIN / FREYA_YS / HODIR_YS / THORIM_YS / MIMIRON_YS = 427..433`, `DATA_KEEPERS_COUNT = 436`, `DATA_DRIVE_ME_CRAZY = 435`, `DATA_ALGALON_SUMMON_STATE = 439`, sigil-door / universe-floor slots = 440..449, `DATA_UNBROKEN = 29052906`) all confirmed against the header.
- Phase-enum audit per boss:
  - Flame Leviathan: events-only, no phase enum (vehicle-state machine via `EVENT_PURSUE`, etc.)
  - Razorscale: 5-state phase enum confirmed at `boss_razorscale.cpp:226-230`
  - XT-002: `PHASE_1` + `PHASE_HEART` confirmed at lines 96-97
  - Auriaya: `PHASE_NONE` + `PHASE_COMBAT` confirmed at lines 108-109
  - Thorim: `PHASE_NULL / PHASE_1 / PHASE_2` confirmed at lines 90-94
  - Algalon: `PHASE_NORMAL / PHASE_ROLE_PLAY / PHASE_BIG_BANG` confirmed at lines 173-175
  - Mimiron: 4-phase form chain `PHASE_LEVIATHAN_MK_II / VX_001 / AERIAL_COMMAND_UNIT / VOL7RON` confirmed at lines 312-315
  - Yogg-Saron: 4-phase machine `PHASE_ONE / PHASE_TRANSFORM / PHASE_TWO / PHASE_THREE` confirmed at lines 283-286
- Hard-mode actions confirmed:
  - Flame Leviathan `ACTION_START_HARD_MODE` at line 174 + 550-557 → adds `LOOT_MODE_HARD_MODE_1..4`
  - XT-002 `ACTION_ENTER_HARD_MODE` at line 102 + line 204
  - Thorim `ACTION_START_HARD_MODE` at line 273
  - Mimiron `DO_ACTIVATE_HARD_MODE` at line 302 + line 417
  - Vezax animus path adds `LOOT_MODE_HARD_MODE_1` at line 205
- Yogg sanity spell IDs: `SPELL_SANITY = 63050`, `SPELL_SANITY_PERIODIC = 63786`, `SPELL_INSANE = 63120`, `SPELL_INSANE_PERIODIC = 64554`, `SPELL_SANITY_WELL = 64169`, `SPELL_SANITY_WELL_VISUAL = 63288`, `SPELL_SANITY_WELL_SUMMON = 64170` — all confirmed at `boss_yogg_saron.cpp:114-216`. `SanityReduction` SpellScript class confirmed at line 1985.
- Yogg keeper handling: `for (uint8 i = DATA_FREYA_YS; i <= DATA_MIMIRON_YS; ++i)` keeper iteration confirmed at lines 514, 967. `_instance->GetData(DATA_KEEPERS_COUNT)` consumed at lines 938, 1024.
- Flame Leviathan tower spell IDs confirmed at `boss_flame_leviathan.cpp:70-73` and tower GO IDs at lines 108-111.
- Algalon `WORLD_STATE_ALGALON_DESPAWN_TIMER = 4131` confirmed in `ulduar.h:472`. `DATA_UNBROKEN = 29052906` confirmed at line 479.
- Hodir `SAY_HARD_MODE_FAILED = 6` confirmed at `boss_hodir.cpp:46`.
- Freya `summonSpell[difficulty][elderCount]` table indexing confirmed at `boss_freya.cpp:624`.
- Iron Council `DATA_PHASE_3 = 1` flag confirmed at line 132.
- Freya elder classes confirmed: `boss_elder_brightleaf` (line 727), `boss_elder_stonebark` (line 828), `boss_elder_ironbranch` (line 935), all registered at lines 1674-1676.
- Mimiron sub-forms confirmed: `boss_leviathan_mk_ii` at line 688, plus form-phase guards in UpdateAI at lines 714, 723, 763.
- Rust state confirmed empty: `wc -l crates/wow-scripts/src/lib.rs` = 0; `wc -l crates/wow-script/src/lib.rs` = 0. No Ulduar identifier appears anywhere under `crates/`.

**Audit verdict:** ✅ complete. Doc fidelity to C++ canonical source is high; Rust state is "empty stub crate" with zero ported content. No discrepancies between this doc and the C++ tree as of 2026-05-01.

---

*Template version: 1.0 (2026-05-01).*
