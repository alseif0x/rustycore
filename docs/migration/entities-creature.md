# Migration: Entities / Creature

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/`
> **Rust target crate(s):** `crates/wow-world/` (`map_manager.rs::WorldCreature`, `handlers/`), `crates/wow-ai/`, `crates/wow-data/`, `crates/wow-database/` (`world_ext`)
> **Layer:** L4 (sub-modules under `entities.md`)
> **Status:** ⚠️ minimal — `WorldCreature` flat struct exists with HP/aggro/wander; everything template-driven (vendor/trainer/gossip/taxi/summons/static-flags/levelscaling) is absent or stubbed
> **Audited vs C++:** ⚠️ partial (header-level audit, `Creature.cpp` body sampled)
> **Last updated:** 2026-05-01
> **Execution note (2026-06-21):** this status snapshot is stale for current implementation work. Use [`creature-port-no-gaps-plan.md`](creature-port-no-gaps-plan.md) as the active no-gaps Creature completion plan. C++ is the only canonical behavior source for this module.

---

## 1. Purpose

`Creature` is the server-side representation of every non-player NPC: monsters, vendors, trainers, flight masters, quest-givers, summons, totems, pets (via `TempSummon`/`Minion`/`Guardian`/`Pet`), critters, and triggers. It inherits from `Unit` (combat/aura host) plus the `GridObject<Creature>` and `MapObject` mixins, owns spawn metadata (`m_spawnId`, `m_creatureData`, `m_creatureInfo`, `m_creatureDifficulty`), the `CreatureAI*` brain, vendor/trainer/gossip plumbing, loot/respawn timers, formation membership, waypoint paths and sparring/static-flag state. All template data is loaded from `creature_template`/`creature_template_addon`/`creature_difficulty`/`creature` world-DB tables and CDN'd in `ObjectMgr`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Creature/Creature.cpp` | 3568 | `prefix` |
| `game/Entities/Creature/Creature.h` | 547 | `prefix` |
| `game/Entities/Creature/CreatureData.h` | 735 | `prefix` |
| `game/Entities/Creature/CreatureGroups.cpp` | 307 | `prefix` |
| `game/Entities/Creature/CreatureGroups.h` | 101 | `prefix` |
| `game/Entities/Creature/GossipDef.cpp` | 689 | `prefix` |
| `game/Entities/Creature/GossipDef.h` | 281 | `prefix` |
| `game/Entities/Creature/TemporarySummon.cpp` | 584 | `prefix` |
| `game/Entities/Creature/TemporarySummon.h` | 169 | `prefix` |
| `game/Entities/Creature/Trainer.cpp` | 246 | `prefix` |
| `game/Entities/Creature/Trainer.h` | 90 | `prefix` |
| `game/Entities/Creature/enuminfo_CreatureData.cpp` | 154 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Creature/Creature.h` | 547 | `Creature` class, `VendorItemCount`, `AssistDelayEvent`, `ForcedDespawnDelayEvent` |
| `src/server/game/Entities/Creature/Creature.cpp` | 3568 | Lifecycle, AI hooks, spawn/respawn, loot/tap, vendor counts, static flag application, evade/leash, save-to-DB |
| `src/server/game/Entities/Creature/CreatureData.h` | 735 | All POD: `CreatureTemplate`, `CreatureData`, `CreatureAddon`, `CreatureModel`, `CreatureBaseStats`, `CreatureDifficulty`, `EquipmentInfo`, `VendorItem`/`VendorItemData`, `CreatureMovementData`, 8× `CreatureStaticFlags*` enums (>200 flags total), `InhabitTypeValues` |
| `src/server/game/Entities/Creature/enuminfo_CreatureData.cpp` | ~120 | Generated reflection for static-flag enums |
| `src/server/game/Entities/Creature/TemporarySummon.h` | ~150 | `TempSummon`, `Minion`, `Guardian`, `Puppet`; `PetEntry` warlock/DK/shaman pet IDs |
| `src/server/game/Entities/Creature/TemporarySummon.cpp` | ~600 | Timer-based despawn, follow logic, owner GUID resolution, totem-slot allocation, `InitStats`/`InitSummon` |
| `src/server/game/Entities/Creature/CreatureGroups.h/.cpp` | ~250 / ~600 | `CreatureGroup` formation: leader/follower offsets, signal/breakup |
| `src/server/game/Entities/Creature/GossipDef.h` | ~250 | `GossipMenu`, `QuestMenu`, `PlayerMenu`; `GossipOptionNpc` (54 NPC roles), `GossipOptionStatus`, `GossipMenuItem`, `GossipQuestItem` |
| `src/server/game/Entities/Creature/GossipDef.cpp` | ~700 | Gossip packet builders: `SendGossipMenu`, `SendQuestList`, `SendQuestDetails`, `SendQuestReward`, `SendPointOfInterest`, taxi-node menu |
| `src/server/game/Entities/Creature/Trainer.h` | ~90 | `Trainer::Spell` (TrainerSpellID, MoneyCost, ReqLevel, ReqSkillLine), `Trainer` class |
| `src/server/game/Entities/Creature/Trainer.cpp` | ~280 | `SendSpells`, `TeachSpell`, requirement checks (level, skill line, prior spell, money), filter by class |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Creature` | class (Unit + GridObject + MapObject) | The NPC entity itself |
| `CreatureTemplate` | struct | `creature_template` row: name, models, factions, npcflags, AI, family, type, spells, expansion, mechanic immunities, scriptID |
| `CreatureData` | struct (extends `SpawnData`) | `creature` spawn row: spawnpoint, equipmentId, wander_distance, currentwaypoint, curhealth/curmana, npcflag overrides |
| `CreatureAddon` | struct | `creature_addon`: pathId, mount, standState, animTier, sheathState, pvpFlags, emote, anim kits, auras |
| `CreatureModel` | struct | DisplayID + scale + probability (random model picker) |
| `CreatureBaseStats` | struct | Per-(level,class) base HP/mana/armor/AP/RAP/dmg from `creature_classlevelstats` |
| `CreatureDifficulty` | struct | Per-difficulty MinLevel/MaxLevel/HealthModifier/ManaModifier/ArmorModifier/DamageModifier/LootID/Gold/StaticFlags |
| `EquipmentInfo` | struct | `creature_equip_template`: 3× weapon/shield item ids + appearance mod |
| `CreatureMovementData` | struct | Ground/Swim/Flight/Rooted/Random + interaction pause |
| `CreatureStaticFlags{1..8}` | enum (DEFINE_ENUM_FLAG) | ≥200 boolean type-properties (mountable, sessile, bossmob, civilian, immune-pc, etc.) |
| `CreatureStaticFlagsHolder` | struct | Aggregate of all 8 flag fields with `HasFlag`/`ApplyFlag` |
| `VendorItem`, `VendorItemData`, `VendorItemCount` | struct | Vendor catalogue + per-item restock-time + per-buyer cooldown |
| `VendorInventoryReason` | enum class | Reason code for `SMSG_VENDOR_INVENTORY` (None / Empty) |
| `InhabitTypeValues` | enum | Ground / Water / Air / Root / Anywhere bitmask |
| `TempSummon`, `Minion`, `Guardian`, `Puppet` | class hierarchy (extends `Creature`) | Summon-with-timer + owner-follow |
| `TempSummonType` | enum | DEAD_DESPAWN / CORPSE_DESPAWN / TIMED_OOC_DESPAWN / MANUAL_DESPAWN / etc. (in `SharedDefines.h`) |
| `SummonPropertiesEntry` | DBC | `SummonProperties.db2`: slot, faction-bias, type, flags |
| `CreatureGroup` | class | Formation: leader + members + offsets |
| `GossipMenu`, `QuestMenu`, `PlayerMenu` | class trio | Per-session gossip dialog state |
| `GossipMenuItem`, `GossipQuestItem` | struct | One menu line: GossipOptionID, OrderIndex, OptionNpc, OptionText, BoxText, condition |
| `GossipOptionNpc` | enum class (uint8) | 54 NPC roles (None/Vendor/Taxinode/Trainer/SpiritHealer/Binder/Banker/Battlemaster/Auctioneer/TalentMaster/Stablemaster/Mailbox/Transmogrify/...) |
| `GossipOptionStatus` | enum class | Available / Unavailable / Locked / AlreadyComplete |
| `Trainer::Spell` | struct | TrainerSpellID, money cost, req-level, req-skill-line, req-skill-rank, req-ability |
| `Trainer` | class | Spell list + filter rules (per-class trainers only show their class spells) |
| `AssistDelayEvent`, `ForcedDespawnDelayEvent` | class (BasicEvent) | Scheduled creature events |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `Creature::Create(guidlow, map, entry, pos, data, vehId, dynamic)` | Construct + bind to template/data | `CreateFromProto`, `InitEntry`, `LoadCreaturesAddon`, `SelectLevel` |
| `Creature::CreateCreatureFromDB(spawnId, map, addToMap, allowDuplicate)` | Spawn from `creature` row | `LoadFromDB` → `Map::AddToMap` |
| `Creature::Update(uint32 time)` | Per-tick: regen, respawn, evade, combat-pulse, AI, formation, swim flag | `RegenerateHealth`, `Regenerate(power)`, `AI()->UpdateAI`, `SelectVictim`, `RemoveCorpse` |
| `Creature::SelectVictim()` | Threat-list victim pick + leash | `Unit::getThreatManager`, `CanCreatureAttack` |
| `Creature::DespawnOrUnsummon(timeMs, forceRespawn)` | Schedule despawn (or unsummon if TempSummon) | `ForcedDespawn` |
| `Creature::RemoveCorpse(setSpawnTime, destroyForNearbyPlayers)` | After corpse-decay timer, remove + schedule respawn | `Map::AddObjectToRemoveList`, `SetRespawnTime` |
| `Creature::Respawn(force)` | Bring back from despawn | `SetSpawnHealth`, `SelectLevel`, `LoadCreaturesAddon`, `AIM_Initialize` |
| `Creature::SetTappedBy(unit, withGroup)` | Mark as tapped (XP/loot allowed for player or party) | populates `m_tapList` |
| `Creature::AllLootRemovedFromCorpse()` | Trigger corpse-decay timer once fully looted | `SetCorpseDelay` |
| `Creature::GetVendorItems()` / `UpdateVendorItemCurrentCount` | Vendor catalogue + per-buyer counts | `m_vendorItemCounts` |
| `Creature::GetGossipMenuId()` / `SetGossipMenuId` | Resolve gossip menu (from template, addon, or override) | `ObjectMgr::GetGossipMenuItems` |
| `Creature::GetTrainerId()` / `SetTrainerId` | Resolve trainer dataset | `ObjectMgr::GetTrainer` |
| `Creature::isCanInteractWithBattleMaster(player, msg)` | Battlemaster role-check | `BattlegroundMgr` |
| `Creature::CanResetTalents(player)` | Talent-master interaction check | — |
| `Creature::SaveToDB(mapid, spawnDifficulties)` | Persist spawn (GM tooling) | `WorldDatabase` `WORLD_INS_CREATURE` |
| `Creature::DeleteFromDB(spawnId)` | Remove spawn + addon + linked respawn + game-event ties | 8× `WORLD_DEL_*` statements |
| `Creature::SummonGraveyardTeleporter()` | Spawn the SH-graveyard portal | `SummonCreature` |
| `Creature::CallForHelp(radius)` / `CallAssistance()` | Pull faction-friendlies into combat | `AssistDelayEvent` |
| `TempSummon::InitStats(summoner, duration)` | Set timer + visibility-to-summoner | — |
| `TempSummon::UnSummon(msTime)` | Schedule despawn | `RemoveFromWorld` |
| `Trainer::SendSpells(npc, player, locale)` | Build `SMSG_TRAINER_LIST` | `WorldSession::SendPacket` |
| `Trainer::TeachSpell(npc, player, spellId)` | Subtract money + grant spell | `Player::LearnSpell` |
| `PlayerMenu::SendGossipMenu(textId, npcGuid)` | Build/send `SMSG_GOSSIP_MESSAGE` | — |
| `PlayerMenu::SendPointOfInterest(poiId)` | `SMSG_GOSSIP_POI` | — |

---

## 5. Module dependencies

**Depends on:**
- `Unit` (base): combat, threat, auras, stats, movement
- `ObjectMgr` (`CreatureTemplate`, `CreatureData`, `CreatureAddon`, `CreatureBaseStats`, `EquipmentInfo`, `VendorItemData`, `Trainer` lookup tables; loads world DB)
- `Map` / `MapManager`: `AddToMap`, `RemoveFromMap`, grid-cell tracking via `MapObject`
- `CreatureAI` / `ScriptMgr`: brain selection (template `AIName` / `ScriptID`)
- `MovementGenerator` / `MotionMaster` (Random/Waypoint/Idle/Confused/Fleeing)
- `Loot` / `LootMgr` (creature_loot_template, pickpocket, skin)
- `ConditionMgr` (gossip option conditions, vendor PlayerConditionId)
- `Pet`, `TemporarySummon` (subclasses)
- DB2 stores: `CreatureFamily`, `CreatureType`, `CreatureModelData`, `CreatureDisplayInfo`, `Faction`, `FactionTemplate`, `SummonProperties`

**Depended on by:**
- `WorldSession` packet handlers: `HandleNpcHelloOpcode`, `HandleListInventoryOpcode`, `HandleTrainerListOpcode`, `HandleTrainerBuySpellOpcode`, `HandleTaxiNodeStatusQueryOpcode`, `HandleActivateTaxi`, `HandleGossipSelectOption`, `HandleQuestgiverHelloOpcode`, `HandleQuestgiverChooseRewardOpcode`, `HandleAuctionListItems`, `HandleBankerActivate`, etc.
- `Player::PrepareGossipMenu`, `Player::SendPreparedGossip`, `Player::RewardPlayerAndGroupAtKill`
- `Battleground` / `OutdoorPvP` (battlemasters, capture creatures)
- `Spell` (line-of-sight checks, spell-focus)

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `WORLD_DEL_CREATURE` | Delete spawn row | world |
| `WORLD_INS_CREATURE` | Insert spawn row (GM tools, on `SaveToDB`) | world |
| `WORLD_DEL_CREATURE_ADDON` | Drop addon on delete | world |
| `WORLD_DEL_SPAWNGROUP_MEMBER` | Remove from spawn-group | world |
| `WORLD_DEL_GAME_EVENT_CREATURE` | Drop game-event tie | world |
| `WORLD_DEL_GAME_EVENT_MODEL_EQUIP` | Drop event-model-equip tie | world |
| `WORLD_DEL_LINKED_RESPAWN` (×2 directions) | Drop linked-respawn refs | world |
| `WORLD_DEL_LINKED_RESPAWN_MASTER` (×2) | Drop linked-respawn master refs | world |
| `CHAR_DEL_CHARACTER_TUTORIAL_PROGRESS` (transitive on quest-giver) | Quest log housekeeping | character |
| Bulk loads (read at startup by `ObjectMgr`): `creature_template`, `creature_template_addon`, `creature_template_resistance`, `creature_template_spell`, `creature_difficulty`, `creature_classlevelstats`, `creature_addon`, `creature_equip_template`, `creature_model_info`, `creature`, `creature_text`, `creature_summon_groups`, `creature_questender`, `creature_queststarter`, `npc_vendor`, `npc_trainer`, `gossip_menu`, `gossip_menu_option`, `points_of_interest` | World-DB seed | world |

DBC/DB2 stores read by Creature/Gossip/Trainer code:

| Store | What it loads | Read by |
|---|---|---|
| `CreatureFamilyStore` | CreatureFamily.db2 | beast/pet logic |
| `CreatureTypeStore` | CreatureType.db2 | type checks (Beast/Demon/Undead/...) |
| `CreatureModelDataStore` | CreatureModelData.db2 | bounding/combat reach |
| `CreatureDisplayInfoStore` | CreatureDisplayInfo.db2 | model selection |
| `FactionStore`, `FactionTemplateStore` | Faction(Template).db2 | reactions |
| `SummonPropertiesStore` | SummonProperties.db2 | TempSummon classification |
| `PointsOfInterestStore` | (DB-only) | Gossip POI |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_QUERY_CREATURE_RESPONSE` | S → C | `CreatureTemplate::BuildQueryData` |
| `CMSG_QUERY_CREATURE` | C → S | `WorldSession::HandleCreatureQuery` |
| `SMSG_GOSSIP_MESSAGE` | S → C | `PlayerMenu::SendGossipMenu` |
| `CMSG_GOSSIP_HELLO` | C → S | `WorldSession::HandleGossipHelloOpcode` |
| `CMSG_GOSSIP_SELECT_OPTION` | C → S | `WorldSession::HandleGossipSelectOption` |
| `SMSG_GOSSIP_POI` | S → C | `PlayerMenu::SendPointOfInterest` |
| `SMSG_GOSSIP_COMPLETE` | S → C | menu close |
| `SMSG_NPC_TEXT_UPDATE` | S → C | `BroadcastTextStore` lookup |
| `SMSG_LIST_INVENTORY` (a.k.a. `SMSG_VENDOR_INVENTORY`) | S → C | `Creature::GetVendorItems` |
| `CMSG_LIST_INVENTORY` | C → S | `WorldSession::HandleListInventoryOpcode` |
| `CMSG_BUY_ITEM`, `CMSG_BUY_ITEM_IN_SLOT`, `CMSG_SELL_ITEM` | C → S | vendor txn |
| `SMSG_BUY_FAILED`, `SMSG_SELL_RESPONSE` | S → C | vendor txn result |
| `SMSG_TRAINER_LIST` | S → C | `Trainer::SendSpells` |
| `CMSG_TRAINER_LIST` | C → S | open trainer |
| `CMSG_TRAINER_BUY_SPELL` | C → S | `Trainer::TeachSpell` |
| `SMSG_TRAINER_BUY_FAILED`, `SMSG_TRAINER_BUY_SUCCEEDED` | S → C | result |
| `CMSG_TAXI_NODE_STATUS_QUERY` | C → S | flightmaster |
| `SMSG_TAXI_NODE_STATUS` | S → C | flightmaster reply |
| `CMSG_ACTIVATE_TAXI`, `CMSG_ACTIVATE_TAXI_EXPRESS` | C → S | take flight |
| `SMSG_ACTIVATE_TAXI_REPLY` | S → C | flight ack |
| `CMSG_TAXI_QUERY_AVAILABLE_NODES` | C → S | open flight map |
| `SMSG_NEW_TAXI_PATH`, `SMSG_SHOW_TAXI_NODES` | S → C | reveal flight map |
| `SMSG_AI_REACTION` | S → C | `Creature::SendAIReaction` (aggro yell) |
| `SMSG_EMOTE`, `SMSG_TEXT_EMOTE` | S → C | creature_text |
| `SMSG_SPELL_GO`, `SMSG_SPELL_FAILED_OTHER` | S → C | creature spell casts |
| `SMSG_THREAT_UPDATE`, `SMSG_HIGHEST_THREAT_UPDATE`, `SMSG_THREAT_REMOVE`, `SMSG_THREAT_CLEAR` | S → C | threat list mirror |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `crates/wow-database/src/world_ext.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/archived/rustycore_ARCHIVED_20260312`:**
- `crates/wow-world/src/map_manager.rs` — `WorldCreature` flat struct (lines 52-300, ~890 LOC total file). Carries: guid, entry, level, hp/max_hp, position, home_pos, state (Idle/Wandering/Returning/InCombat/Dead), move_target, corpse_despawn_at, npc_flags, unit_flags, aggro_radius, min/max dmg, display_id, faction, respawn_time_secs, wander_radius, combat_target, swing_timer. `MapManager` 64×64 grid. **No** template/difficulty/addon/static-flag concept.
- `crates/wow-world/src/handlers/character.rs` — `send_nearby_creatures`, `send_nearby_gameobjects`. Spawns from world DB row by row via `SEL_CREATURES_IN_RANGE`/`SEL_GAMEOBJECTS_IN_RANGE`. Uses `CreatureCreateData` not `CreatureTemplate`.
- `crates/wow-world/src/handlers/loot.rs` — drives `corpse_despawn_at` setting on full-loot.
- `crates/wow-world/src/handlers/misc.rs`, `handlers/trainer.rs` — minimal trainer/vendor stubs (uses MapManager bridge wrappers).
- `crates/wow-packet/src/packets/update.rs` — `CreatureCreateData` (lines around the ObjectGuid + entry build). `health: i64`, `faction_template: i32`, `npc_flags: u64`. Wire-format mapping is reasonable.
- `crates/wow-database/src/world_ext.rs` — async creature spawn loader.
- `crates/wow-ai/` — minimal AI scaffolding; not yet bound to the creature tick loop.
- `_attic/creature_integration.rs.txt`, `_attic/character_migration.rs.txt`, `_attic/migrated_session_methods.rs.txt` — failed prior attempt (do not reuse mechanically).

**What's implemented:**
- Spawn from `creature` row, basic visibility window, wander between idle and InCombat states, melee swing timer, HP regen on leash, single-victim aggro, corpse-decay timer + respawn after configurable seconds, 3×3-grid visibility, basic threat ping (single target).
- Wire-level `CreatureCreateData` matches client expectations for a generic NPC.
- Two parallel storage paths: `WorldSession::creatures` (legacy, `#[deprecated]`) and `MapManager::WorldCreature` (new, not fully wired yet).

**What's missing vs C++:**
- No `CreatureTemplate` / `CreatureData` / `CreatureAddon` / `CreatureBaseStats` / `CreatureDifficulty` / `CreatureMovementData` / `EquipmentInfo` / `CreatureModel` separation — fields are flattened.
- 0 of 8 `CreatureStaticFlags*` enums (>200 flags). `flags_extra`, `unit_flags2/3`, `MechanicImmuneMask`, `SpellSchoolImmuneMask` absent.
- No `CreatureClassifications` (Normal/Elite/RareElite/WorldBoss/Rare/Trivial/MinusMob).
- No `CreatureAddon` (no mount/standstate/sheathstate/anim-tier/auras-on-spawn/path-id).
- No `LoadEquipment`, `SelectLevel` (level scaling), `UpdateLevelDependantStats`, `SelectWildBattlePetLevel`, `LoadTemplateRoot`, `LoadCreaturesSparringHealth`.
- No formation system (`CreatureGroup`).
- No `TempSummon`/`Minion`/`Guardian`/`Puppet` hierarchy. No `SummonProperties` honoring. No totem-slot mgmt.
- No vendor catalogue (`VendorItemData`) — handler stub only. No per-buyer `VendorItemCount` cooldowns.
- No `Trainer` data structure. No teaching, no requirement checks (level/skill/prior-spell/money).
- No `GossipMenu`/`QuestMenu`/`PlayerMenu`. No `GossipOptionNpc` (54 roles). No POI sender, no SpiritHealer wheel.
- No taxi: no `PlayerTaxi`-driven activation, no `SMSG_TAXI_NODE_STATUS`, no path serialization.
- No `m_tapList` semantics (party-tap, multi-tap, raid-lock-on-death). Loot recipient is single-player only.
- No `SetSpellFocus` / `ReleaseSpellFocus` (caster facing during cast).
- No `CallForHelp` / `CallAssistance` / `AssistDelayEvent`.
- No combat-pulse-zone-into-combat (dungeons).
- No `IsRacialLeader` / `IsCivilian` / `IsTrigger` / `IsGuard` / `IsDungeonBoss` / `isWorldBoss`.
- No `_lastDamagedTime` / evade-on-out-of-leash; current evade is naive distance-from-home.
- No persisting GM-spawned creatures (no `SaveToDB`).
- No `CreatureTextRepeatGroup` (random non-repeating yells).
- No localized name (`CreatureLocale`).
- No DB2: `CreatureFamilyStore`, `CreatureTypeStore`, `CreatureModelDataStore`, `SummonPropertiesStore`.

**Suspicious / likely divergent:**
- Wire `npc_flags: u32` in handler vs `npcflag: u64` in template (NpcFlags2 not represented).
- Faction encoded as `u32` then cast to `i32` in `CreatureCreateData` — sign handling differs between rust and the i32 wire type.
- `level` stored as `u8` but `CreatureBaseStats` uses level as table index up to `MAX_LEVEL` (currently 80 in WoLK 3.4) — fits but no expansion-aware scaling.
- Aggro radius is constant per creature; C++ `GetAttackDistance(target)` modifies by level diff, leader-of-the-pack, sanctuary, gray-mob config — none of that is present.
- Corpse delay is hardcoded; C++ uses template `CorpseDelay` + `CREATURE_STATIC_FLAG_3_EXTENDED_CORPSE_DURATION` / `_3_FOREVER_CORPSE_DURATION`.
- Respawn time is fixed at `respawn_time_secs: 30`; C++ uses `creature.spawntimesecs` per-spawn and supports dynamic respawn ranges.
- No `ReactState` (Aggressive/Defensive/Passive); current creatures all aggro by proximity.

**Tests existing:** 12 tests in `crates/wow-world/src/map_manager.rs` (grid, add/remove, visibility window, aggro). 0 tests for vendor / trainer / gossip / taxi / formation / template loading / static flags / TempSummon.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_CREATURE.WBS.001** Partir y cerrar la migracion auditada de `game/Entities/Creature/Creature.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3568 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.002** Partir y cerrar la migracion auditada de `game/Entities/Creature/Creature.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 547 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.003** Partir y cerrar la migracion auditada de `game/Entities/Creature/CreatureData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 735 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.004** Cerrar la migracion auditada de `game/Entities/Creature/CreatureGroups.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureGroups.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.005** Cerrar la migracion auditada de `game/Entities/Creature/CreatureGroups.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureGroups.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.006** Partir y cerrar la migracion auditada de `game/Entities/Creature/GossipDef.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 689 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.007** Cerrar la migracion auditada de `game/Entities/Creature/GossipDef.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.008** Partir y cerrar la migracion auditada de `game/Entities/Creature/TemporarySummon.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/TemporarySummon.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 584 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.009** Cerrar la migracion auditada de `game/Entities/Creature/TemporarySummon.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/TemporarySummon.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.010** Cerrar la migracion auditada de `game/Entities/Creature/Trainer.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Trainer.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.011** Cerrar la migracion auditada de `game/Entities/Creature/Trainer.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Trainer.h`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CREATURE.WBS.012** Cerrar la migracion auditada de `game/Entities/Creature/enuminfo_CreatureData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/enuminfo_CreatureData.cpp`
  Rust target: `crates/wow-world`, `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#CRE.1** Define `CreatureTemplate` POD in `wow-data` matching the world DB schema; loader from `creature_template` (M)
- [ ] **#CRE.2** Define `CreatureDifficulty` keyed by (entry, difficulty) with HP/mana/armor/dmg modifiers; replace hardcoded scaling (M)
- [ ] **#CRE.3** Define `CreatureBaseStats` per-(level,class) and implement `SelectLevel` + `UpdateLevelDependantStats` (M)
- [ ] **#CRE.4** Define `CreatureAddon` (mount, standState, animTier, sheathState, pvpFlags, emote, animKits, on-spawn auras) and `LoadCreaturesAddon` (L)
- [ ] **#CRE.5** Define all 8× `CreatureStaticFlags{1..8}` enums + `CreatureStaticFlagsHolder` aggregate (L)
- [ ] **#CRE.6** Define `EquipmentInfo` + `LoadEquipment(id, force)` (L)
- [ ] **#CRE.7** Define `CreatureModel` random-picker honoring `Probability` + invisible/visible split (L)
- [ ] **#CRE.8** Implement `ReactState` (Aggressive/Defensive/Passive) and `InitializeReactState` (L)
- [ ] **#CRE.9** Replace flat `WorldCreature` with split: `CreatureSpawn` (per-spawn) + ref to `CreatureTemplate` + ref to `CreatureDifficulty` (H)
- [ ] **#CRE.10** Implement `m_tapList: HashSet<ObjectGuid>` + `SetTappedBy(unit, withGroup)` + `IsTapListNotClearedOnEvade` (M)
- [ ] **#CRE.11** Implement `RemoveCorpse` / `Respawn` / `DespawnOrUnsummon` matching C++ timer semantics (M)
- [ ] **#CRE.12** Implement vendor: `VendorItemData`, `VendorItemCount`, `UpdateVendorItemCurrentCount`, restock timer; wire `SMSG_LIST_INVENTORY` (M)
- [ ] **#CRE.13** Implement `Trainer` struct + `SendSpells` + `TeachSpell` + req checks; wire `SMSG_TRAINER_LIST` / `CMSG_TRAINER_BUY_SPELL` (H)
- [ ] **#CRE.14** Implement `GossipMenu` / `QuestMenu` / `PlayerMenu`; load `gossip_menu` + `gossip_menu_option`; honor `GossipOptionNpc` (H)
- [ ] **#CRE.15** Implement `PointsOfInterest` table + `SMSG_GOSSIP_POI` (L)
- [ ] **#CRE.16** Implement taxi flow: `PlayerTaxi` bitmask, `HandleTaxiNodeStatusQueryOpcode`, `HandleActivateTaxi` (cooperates with Player module) (H)
- [ ] **#CRE.17** Implement `TempSummon` + `Minion` + `Guardian` (Pet is its own crate) — owner GUID, lifetime timer, `SummonPropertiesEntry` honoring (H)
- [ ] **#CRE.18** Implement totem-slot allocation (`FindUsableTotemSlot`) (M)
- [ ] **#CRE.19** Implement `CreatureGroup` formations (leader+offsets) (M)
- [ ] **#CRE.20** Implement `CallForHelp` / `CallAssistance` / `AssistDelayEvent` (M)
- [ ] **#CRE.21** Implement `SetSpellFocus` / `ReleaseSpellFocus` (caster facing) (L)
- [ ] **#CRE.22** Implement `IsImmunedToSpellEffect` honoring `MechanicImmuneMask` + `SpellSchoolImmuneMask` (M)
- [ ] **#CRE.23** Implement `SaveToDB` / `DeleteFromDB` for GM tooling (use 8× `WORLD_DEL_*` statements) (M)
- [ ] **#CRE.24** Implement `CreatureLocale` for non-enUS clients (L)
- [ ] **#CRE.25** Implement `CreatureTextRepeatGroup` (random non-repeating yells) (L)
- [ ] **#CRE.26** Wire `BroadcastText` table + `SMSG_NPC_TEXT_UPDATE` (M)
- [ ] **#CRE.27** Migrate all session methods off the legacy `WorldSession.creatures` HashMap to `MapManager` (M) — already partially done per CLAUDE.md
- [ ] **#CRE.28** Retire `_attic/creature_integration.rs.txt` content once #CRE.9 is in (no-op, just delete) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_CREATURE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 12 files / 7471 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp`. Rust target: `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | `cargo test -p wow-ai && cargo test -p wow-data && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_CREATURE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 12 files / 7471 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp`. Rust target: `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_CREATURE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 12 files / 7471 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp`. Rust target: `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_CREATURE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 12 files / 7471 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp`. Rust target: `crates/wow-ai`, `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `CreatureTemplate` from `creature_template` row produces same npcflag/unit_flags/MechanicImmuneMask as C++ `ObjectMgr::LoadCreatureTemplates`
- [ ] Test: `SelectLevel` for elite at level 70 produces HP within 5% of C++ value (uses `creature_classlevelstats` + `CreatureDifficulty.HealthModifier`)
- [ ] Test: `RemoveCorpse` after `m_corpseDelay` schedules respawn at `now + spawntimesecs`
- [ ] Test: Tapped-by-party allows all party members to loot, exclusively
- [ ] Test: vendor `VendorItem.maxcount > 0 && incrtime > 0` restocks after `incrtime` seconds
- [ ] Test: trainer rejects spell when player level < `Spell::ReqLevel`
- [ ] Test: trainer charges `MoneyCost * (1 - reputation discount)` (C++ `Player::GetReputationPriceDiscount`)
- [ ] Test: Gossip menu option with unmet `Condition` returns `GossipOptionStatus::Unavailable`
- [ ] Test: `TempSummon` with `TEMPSUMMON_TIMED_DESPAWN` despawns exactly at duration
- [ ] Test: `TempSummon` with `TEMPSUMMON_DEAD_DESPAWN` survives indefinitely until killed
- [ ] Test: `CreatureGroup` leader pull triggers all members entering combat
- [ ] Test: `SetReactState(Passive)` → creature does not aggro on proximity, but does counter-attack
- [ ] Test: `CREATURE_STATIC_FLAG_SESSILE` sets MOVEMENTFLAG_ROOT and prevents motion-master change
- [ ] Test: Creature with `flags_extra & CIVILIAN` calls guards but doesn't flee
- [ ] Test: `RefreshCanSwimFlag` toggles `UNIT_FLAG_CAN_SWIM` based on combat state and template

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 12 files / 7471 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp` | `crates/wow-world/` (`map_manager.rs::WorldCreature`, `handlers/`), `crates/wow-ai/`, `crates/wow-data/`, `crates/wow-database/` (`world_ext`) \| ⚠️ minimal — `WorldCreature` flat struct exists with HP/aggro/wander; everything template-driven (vendor/trainer/gossip/taxi/summons/static-flags/levelscaling) is absent or stubbed |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_CREATURE.DIV.001` | `crates/wow-database/src/world_ext.rs` (`missing_declared_path`, 0 Rust lines) | 12 C++ files / 7471 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/CreatureData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/GossipDef.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- The legacy storage `WorldSession::creatures: HashMap<ObjectGuid, CreatureAI>` plus `WorldSession::visible_creatures` is `#[deprecated]` per CLAUDE.md; `MapManager::WorldCreature` is the destination. Bridge wrappers in `map_helpers.rs` (`get_creature`, `with_creature_mut`, `spawn_creature_global`, `get_visible_creatures`) are the only path that should be added to going forward.
- `_attic/creature_integration.rs.txt` was written against `CreatureCreateData` fields that **never existed** (`entry_id`, `position`, `current_hp`, `max_hp`). Real names are `entry`, `health`, `max_health`, with position passed separately. Don't re-introduce attic content mechanically — read `_attic/README.md` first.
- `CreatureClassifications` matters for damage/HP modifiers and is stored on the **template**, not the spawn. Don't put it on `WorldCreature`.
- `m_corpseData` and `m_creatureData->mapId != GetMapId()` is how transports detect creatures spawned on them — easy to get wrong when MapManager learns about transports.
- `IsTemplateRooted()` ⇒ `CREATURE_STATIC_FLAG_SESSILE` ⇒ `creature_template_movement.Rooted = 1`. Three names for the same fact — pick one canonical Rust spelling.
- `m_loot` in C++ is `unique_ptr<Loot>` and `m_personalLoot` is keyed by player GUID for personal-loot dungeons; MoP+ feature but worth modeling now even if WoLK 3.4 only uses `m_loot`.
- `Trainer::Spell::ReqAbility` is an array of 3 in some forks; Trinity's WoLK 3.4 keeps it as 1 + skill-line + skill-rank — verify against `npc_trainer` schema before designing the Rust struct.
- Gossip "Forced" via `CREATURE_STATIC_FLAG_4_FORCE_GOSSIP` overrides quest-giver auto-pickup; easy to miss.
- BroadcastText is the *only* legitimate source of NPC dialog strings since 6.x; don't hardcode strings.
- Static flag 8 (`CreatureStaticFlags8`) didn't exist in 3.3.5 vanilla TC but does in WoLK 3.4 Classic build — confirm against the SQL dump.
- `RegenerateHealth` in combat is gated by `CREATURE_STATIC_FLAG_5_NO_HEALTH_REGEN` AND `_regenerateHealth` AND `IsRegeneratingHealth`. Three gates — easy to short-circuit incorrectly.
- C++ "tap list" cap is `CREATURE_TAPPERS_SOFT_CAP = 5` — soft, not hard.
- C# references in Creature paths are migration bugs, not fallback documentation. Re-audit each one against `/home/server/woltk-trinity-legacy`, fix the Rust behavior if it differs, and remove the C# reference.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Creature : Unit, GridObject<Creature>, MapObject` | `struct WorldCreature` (flat, in `map_manager.rs`) — eventually split into `Creature { spawn: CreatureSpawn, template: &'static CreatureTemplate, difficulty: &'static CreatureDifficulty }` | composition, no inheritance |
| `CreatureTemplate const* m_creatureInfo` | `&'static CreatureTemplate` (interned in `wow-data`) | one alloc, reused across spawns |
| `CreatureData const* m_creatureData` | `Arc<CreatureSpawn>` from `WorldDb` | mutable per-spawn state goes elsewhere |
| `CreatureDifficulty const* m_creatureDifficulty` | `&'static CreatureDifficulty` | DB-backed |
| `CreatureAddon const*` | `Option<&'static CreatureAddon>` | optional |
| `CreatureStaticFlagsHolder _staticFlags` | `bitflags::bitflags!`-generated `CreatureStaticFlags` (split across 8 fields or one `u256`) | wire encoding vs runtime — decide |
| `std::array<std::string_view, 3> m_stringIds` | `[Cow<'static, str>; 3]` | DB-loaded, mostly &'static |
| `Optional<uint32> _trainerId` | `Option<u32>` | direct |
| `uint32 _gossipMenuId` | `u32` (0 = none) | direct |
| `GuidUnorderedSet m_tapList` | `HashSet<ObjectGuid>` | use `ahash` for speed |
| `CreatureTextRepeatGroup m_textRepeat` | `HashMap<u8, SmallVec<[u8;8]>>` | small group ids |
| `MovementGeneratorType m_defaultMovementType` | `enum DefaultMovement` | direct |
| `void Update(uint32 time)` | `fn tick(&mut self, diff_ms: u32, ctx: &mut TickCtx)` | Cf. `MapManager::tick` |
| `class TempSummon : Creature` | `struct TempSummon { creature: WorldCreature, summoner: ObjectGuid, kind: TempSummonType, expires_at: Instant, ... }` | composition |
| `class Minion : TempSummon` | same with `owner: ObjectGuid, follow_angle: f32` | — |
| `class GossipMenu / QuestMenu / PlayerMenu` | `struct GossipMenu { items: Vec<GossipMenuItem> }`, `struct QuestMenu { items: Vec<QuestMenuItem> }`, `struct PlayerMenu { gossip: GossipMenu, quests: QuestMenu, npc: ObjectGuid }` | per-session |
| `enum class GossipOptionNpc : uint8` | `#[repr(u8)] enum GossipOptionNpc` | 54 variants |
| `class Trainer` | `struct Trainer { id: u32, type_: TrainerType, spells: Vec<TrainerSpell>, greeting: String }` | per-trainer |
| `Trainer::Spell` | `struct TrainerSpell { spell_id: u32, money_cost: u32, req_level: u8, req_skill_line: u32, req_skill_rank: u16, req_ability: [u32; 1] }` | matches `npc_trainer` |
| `VendorItemCounts m_vendorItemCounts` | `Vec<VendorItemCount>` | small, linear scan ok |
| `class CreatureGroup` | `struct CreatureGroup { leader: ObjectGuid, members: Vec<(ObjectGuid, FormationOffset)> }` | — |
| `class AssistDelayEvent : BasicEvent` | scheduled via `tokio::time::sleep` + `tx.send(AssistEvent { … })` | event loop, not BasicEvent |
| `WorldDatabasePreparedStatement WORLD_DEL_CREATURE` | `WorldStatements::DEL_CREATURE` enum variant | `wow-database` registry |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Creature` | partial (flat `WorldCreature` only) | `crates/wow-world/src/map_manager.rs:52` | ⚠️ minimal — missing template/difficulty/addon/staticflags split |
| `class TempSummon` | no | — | ❌ missing |
| `class Minion` | no | — | ❌ missing |
| `class Guardian` | no | — | ❌ missing |
| `class Puppet` | no | — | ❌ missing |
| `struct CreatureTemplate` | no | — | ❌ missing |
| `struct CreatureData` (spawn) | partial via `CreatureCreateData` (wire only) | `crates/wow-packet/src/packets/update.rs` | ⚠️ wire-only |
| `struct CreatureAddon` | no | — | ❌ missing |
| `struct CreatureBaseStats` | no | — | ❌ missing |
| `struct CreatureDifficulty` | no | — | ❌ missing |
| `struct EquipmentInfo` | no | — | ❌ missing |
| `struct CreatureModel` | no (display_id flat) | — | ❌ missing |
| `struct CreatureLocale` | no | — | ❌ missing |
| `enum CreatureClassifications` | no | — | ❌ missing |
| `enum CreatureStaticFlags{1..8}` | no | — | ❌ missing (>200 flags) |
| `struct CreatureMovementData` | no | — | ❌ missing |
| `enum InhabitTypeValues` | no | — | ❌ missing |
| `struct VendorItem` / `VendorItemData` | no | — | ❌ missing |
| `struct VendorItemCount` | no | — | ❌ missing |
| `class Trainer` / `Trainer::Spell` | stub only | `crates/wow-world/src/handlers/trainer.rs` | ❌ no data, no req checks |
| `class GossipMenu` / `QuestMenu` / `PlayerMenu` | no | — | ❌ missing |
| `enum class GossipOptionNpc` | no | — | ❌ missing |
| `class CreatureGroup` | no | — | ❌ missing |
| `struct SummonPropertiesEntry` (DB2) | no | — | ❌ missing |
| `Creature::Update` | partial (basic tick) | `map_manager.rs` ticks | ⚠️ no AI hook, no formation, no combat-pulse |
| `Creature::SelectVictim` | no (single combat_target) | — | ❌ missing |
| `Creature::CallForHelp` / `CallAssistance` | no | — | ❌ missing |
| `Creature::SetSpellFocus` / `ReleaseSpellFocus` | no | — | ❌ missing |
| `Creature::SetTappedBy` / `m_tapList` | no | — | ❌ missing |
| `Creature::AllLootRemovedFromCorpse` | yes (timer set in handler) | `crates/wow-world/src/handlers/loot.rs:202` | ✅ behavioral match (minimal) |
| `Creature::RemoveCorpse` | yes (basic) | `crates/wow-world/src/session.rs:~1985` tick loop | ⚠️ no `m_corpseDelay` template wiring |
| `Creature::Respawn` | yes (basic) | session tick | ⚠️ uses fixed `respawn_time_secs` |
| `Creature::SaveToDB` / `DeleteFromDB` | no | — | ❌ missing |
| `WORLD_INS_CREATURE` / `WORLD_DEL_CREATURE` | no | `crates/wow-database/src/statements/world.rs` | ❌ not registered |
| `SEL_CREATURES_IN_RANGE` | yes | `crates/wow-database/src/statements/world.rs` | ✅ present |
| `CMSG_QUERY_CREATURE` / `SMSG_QUERY_CREATURE_RESPONSE` | partial (opcode constants only) | `crates/wow-constants/src/opcodes.rs` | ⚠️ no full query response builder |
| `SMSG_AI_REACTION` | no | — | ❌ missing |
| `SMSG_LIST_INVENTORY` (vendor) | stub | `handlers/misc.rs` | ❌ no real vendor data |
| `SMSG_TRAINER_LIST` / `CMSG_TRAINER_BUY_SPELL` | stub | `handlers/trainer.rs` | ❌ no real trainer data |
| `SMSG_GOSSIP_MESSAGE` / `CMSG_GOSSIP_HELLO` / `CMSG_GOSSIP_SELECT_OPTION` | no | — | ❌ missing |
| `SMSG_GOSSIP_POI` | no | — | ❌ missing |
| Taxi opcodes | no | — | ❌ missing |
| `SMSG_NPC_TEXT_UPDATE` (BroadcastText) | no | — | ❌ missing |

**Verdict:** ⚠️ minimal — `WorldCreature` covers the "I am a hostile mob the player can hit" case at maybe 8% of C++ Creature surface area. Vendor/trainer/gossip/taxi/summons/static-flags/level-scaling/formations/tap-list/spell-focus/template-driven anything is absent. Migration here is the single largest non-Player work item in the entity layer.

---

*Sub-doc of `entities.md`. Template version: 1.0 (2026-05-01).*
