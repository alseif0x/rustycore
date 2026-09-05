# Migration: Battlegrounds + Arenas

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/`
> **Rust target crate(s):** `crates/wow-pvp/` (currently empty scaffold — `Cargo.toml` only, no `lib.rs` content)
> **Layer:** L7
> **Status:** ❌ not started
> **Audited vs C++:** ✅ audited 2026-05-01 (❌ confirmed)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Manages all instanced PvP — Battlegrounds (10v10, 15v15, 40v40 zone-based with capture flags / bases / vehicles / siege) and Arenas (2v2/3v3/5v5, both rated with persistent teams and unrated skirmishes). Owns the queue / matchmaking / bracketing pipeline (`BattlegroundQueue`), the per-match runtime (`Battleground` + per-zone subclass), the persistent rated-team registry (`ArenaTeam` / `ArenaTeamMgr`), and the global lifecycle / template / world-state coordinator (`BattlegroundMgr`).

In WoLK 3.4.3 there are 6 battlegrounds (AB, AV, EotS, IoC, SotA, WSG) plus arena maps (BE = Blade's Edge, DS = Dalaran Sewers, NA = Nagrand, RL = Ruins of Lordaeron, RV = Ring of Valor) and a few placeholder/cataclysm-era ones (BFG, TP) that remain stubbed. Each BG zone has its own ~500–1500-line script with capture-point logic, flag carrier tracking, NPC reinforcements, vehicle mounts (IoC), gate destruction (SotA), and end-condition arithmetic.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Battlegrounds/Arena.cpp` | 289 | `prefix` |
| `game/Battlegrounds/Arena.h` | 78 | `prefix` |
| `game/Battlegrounds/ArenaScore.cpp` | 66 | `prefix` |
| `game/Battlegrounds/ArenaScore.h` | 58 | `prefix` |
| `game/Battlegrounds/ArenaTeam.cpp` | 834 | `prefix` |
| `game/Battlegrounds/ArenaTeam.h` | 192 | `prefix` |
| `game/Battlegrounds/ArenaTeamMgr.cpp` | 134 | `prefix` |
| `game/Battlegrounds/ArenaTeamMgr.h` | 55 | `prefix` |
| `game/Battlegrounds/Battleground.cpp` | 1883 | `prefix` |
| `game/Battlegrounds/Battleground.h` | 604 | `prefix` |
| `game/Battlegrounds/BattlegroundMgr.cpp` | 754 | `prefix` |
| `game/Battlegrounds/BattlegroundMgr.h` | 196 | `prefix` |
| `game/Battlegrounds/BattlegroundQueue.cpp` | 1089 | `prefix` |
| `game/Battlegrounds/BattlegroundQueue.h` | 184 | `prefix` |
| `game/Battlegrounds/BattlegroundScore.cpp` | 74 | `prefix` |
| `game/Battlegrounds/BattlegroundScore.h` | 96 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundAB.cpp` | 529 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundAB.h` | 305 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundAV.cpp` | 1483 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundAV.h` | 1734 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundBE.cpp` | 99 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundBE.h` | 69 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundBFG.cpp` | 26 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundBFG.h` | 73 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundDS.cpp` | 167 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundDS.h` | 113 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundEY.cpp` | 533 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundEY.h` | 359 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundIC.cpp` | 913 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundIC.h` | 1056 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundNA.cpp` | 96 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundNA.h` | 68 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundRL.cpp` | 94 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundRL.h` | 64 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundRV.cpp` | 165 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundRV.h` | 110 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundSA.cpp` | 1036 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundSA.h` | 710 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundTP.cpp` | 26 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundTP.h` | 73 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundWS.cpp` | 580 | `prefix` |
| `game/Battlegrounds/Zones/BattlegroundWS.h` | 247 | `prefix` |
| `game/Battlegrounds/enuminfo_ArenaTeam.cpp` | 67 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Battlegrounds/Battleground.h` | 604 | Base class + enums (status, sounds, spells, time intervals, broadcast text ids, player position constants) |
| `src/server/game/Battlegrounds/Battleground.cpp` | 1883 | Update loop (`_ProcessJoin/_ProcessProgress/_ProcessLeave`), player lifecycle, scoring, end-game rewards, raid auto-grouping, player-position broadcast, prematureWinner |
| `src/server/game/Battlegrounds/BattlegroundMgr.h` | 196 | `BattlegroundMgr` singleton, `BattlegroundData`, `BattlegroundTemplate`, BattleMastersMap |
| `src/server/game/Battlegrounds/BattlegroundMgr.cpp` | 754 | Template loader, instance allocation, queue update scheduler, holiday/weekend logic, `BuildBattlegroundStatus*` packet builders |
| `src/server/game/Battlegrounds/BattlegroundQueue.h` | 184 | `BattlegroundQueue`, `GroupQueueInfo`, `PlayerQueueInfo`, `SelectionPool`, BG queue events |
| `src/server/game/Battlegrounds/BattlegroundQueue.cpp` | 1089 | Add/remove group, premade-vs-normal matching, faction balancing, average wait time tracking, rated arena MMR matching, `BGQueueInviteEvent` / `BGQueueRemoveEvent` |
| `src/server/game/Battlegrounds/BattlegroundScore.h/.cpp` | 96/74 | Per-player score abstract (kills, deaths, honor, damage, healing); subclasses extend with BG-specific stats |
| `src/server/game/Battlegrounds/Arena.h/.cpp` | 78/289 | `Arena` subclass of `Battleground`; arena-specific reward calc + MMR adjustment + `BuildPvPLogDataPacket` overrides |
| `src/server/game/Battlegrounds/ArenaScore.h/.cpp` | 58/66 | Arena-specific score type |
| `src/server/game/Battlegrounds/ArenaTeam.h` | 192 | `ArenaTeam`, `ArenaTeamMember`, `ArenaTeamStats`, error / event enums |
| `src/server/game/Battlegrounds/ArenaTeam.cpp` | 834 | DB persistence, ELO/MMR math (`GetMatchmakerRatingMod`, `WonAgainst`, `LostAgainst`, `GetChanceAgainst`), member add/remove, weekly stats reset |
| `src/server/game/Battlegrounds/ArenaTeamMgr.h/.cpp` | 55/134 | Global registry: `LoadArenaTeams`, `GetArenaTeamById/ByName/ByCaptain`, ID generator |
| `src/server/game/Battlegrounds/Zones/BattlegroundAB.cpp/.h` | 529 / 305 | Arathi Basin — 5 capture nodes, resource accumulation race to 1600 |
| `src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp/.h` | 1483 / 1734 | Alterac Valley — 40v40, towers, captains, mines, lieutenants, generals; biggest BG by far |
| `src/server/game/Battlegrounds/Zones/BattlegroundBE.cpp/.h` | 99 / 69 | Blade's Edge arena |
| `src/server/game/Battlegrounds/Zones/BattlegroundBFG.cpp/.h` | 26 / 73 | Battle for Gilneas — Cata stub (3.4.3 fork carries the file) |
| `src/server/game/Battlegrounds/Zones/BattlegroundDS.cpp/.h` | 167 / 113 | Dalaran Sewers arena (water hazard + pipes) |
| `src/server/game/Battlegrounds/Zones/BattlegroundEY.cpp/.h` | 533 / 359 | Eye of the Storm — 4 capture points + flag mid → cap point |
| `src/server/game/Battlegrounds/Zones/BattlegroundIC.cpp/.h` | 913 / 1056 | Isle of Conquest — vehicles, siege engines, keep + airship |
| `src/server/game/Battlegrounds/Zones/BattlegroundNA.cpp/.h` | 96 / 68 | Nagrand arena |
| `src/server/game/Battlegrounds/Zones/BattlegroundRL.cpp/.h` | 94 / 64 | Ruins of Lordaeron arena |
| `src/server/game/Battlegrounds/Zones/BattlegroundRV.cpp/.h` | 165 / 110 | Ring of Valor arena (elevators, fire pillars) |
| `src/server/game/Battlegrounds/Zones/BattlegroundSA.cpp/.h` | 1036 / 710 | Strand of the Ancients — attack/defend gates, demolishers, relic |
| `src/server/game/Battlegrounds/Zones/BattlegroundTP.cpp/.h` | 26 / 73 | Twin Peaks — Cata stub |
| `src/server/game/Battlegrounds/Zones/BattlegroundWS.cpp/.h` | 580 / 247 | Warsong Gulch — flag capture, 3 caps to win |

Approx total: **~6,650 lines core + ~10,730 zone-script lines = ~17,400 lines**.

---

## 3. Classes / Structs / Enums

### Core

| Symbol | Kind | Purpose |
|---|---|---|
| `Battleground` | class (extends `ZoneScript`) | Base for all matches; owns players, scores, raid groups, status, timers |
| `Arena` | class (extends `Battleground`) | Arena specialization (rated/skirmish, MMR bookkeeping) |
| `BattlegroundMgr` | singleton | Templates, instance store, queue map, BattleMasters table |
| `BattlegroundQueue` | class | Per-`BattlegroundQueueTypeId` queue with bracket × side × {premade,normal} dim |
| `BattlegroundQueue::SelectionPool` | inner class | Greedy invite assembler |
| `BattlegroundData` | struct | `m_Battlegrounds` (instanceId → BG) + per-bracket client visible IDs |
| `BattlegroundTemplate` | struct | Template loaded from `battleground_template`: type id, start locations, weight, scriptId, BattlemasterListEntry, min/max levels & players |
| `BattlegroundPlayer` | struct | Per-player state in BG: `OfflineRemoveTime`, `Team`, `Mercenary`, `queueTypeId` |
| `BattlegroundObjectInfo` | struct | `GameObject*` + timer + spellid (for buff respawn etc.) |
| `BattlegroundScore` | struct (abstract) | Polymorphic per-BG score, builds packet on demand |
| `GroupQueueInfo` | struct | Queued group: members, team, joinTime, MMR, OpponentRating, IsInvitedToBGInstanceGUID |
| `PlayerQueueInfo` | struct | Per-player back-ref to GroupQueueInfo + LastOnlineTime |
| `BGQueueInviteEvent` | class (`BasicEvent`) | Reminds player after 1 min; can re-invite |
| `BGQueueRemoveEvent` | class (`BasicEvent`) | Auto-remove after 1m20s no answer |

### Arena

| Symbol | Kind | Purpose |
|---|---|---|
| `ArenaTeam` | class | Persistent rated team (2v2, 3v3, 5v5) — DB-backed |
| `ArenaTeamMember` | struct | guid, name, class, weekGames/Wins, seasonGames/Wins, PersonalRating, MMR |
| `ArenaTeamStats` | struct | Rating, week stats, season stats, Rank |
| `ArenaTeamMgr` | singleton | Global team registry by id/name/captain; `GenerateArenaTeamId` |
| `ArenaScore` | class | Arena-specific scoring (rating change, team total) |

### Enums (most-used)

| Symbol | Kind | Purpose |
|---|---|---|
| `BattlegroundStatus` | enum | `NONE=0, WAIT_QUEUE=1, WAIT_JOIN=2, IN_PROGRESS=3, WAIT_LEAVE=4` |
| `BattlegroundQueueIdType` | enum class | `Battleground=0, Arena=1, Wargame=2, Cheat=3, ArenaSkirmish=4` |
| `BattlegroundQueueGroupTypes` | enum | `PREMADE_ALLIANCE=0, PREMADE_HORDE=1, NORMAL_ALLIANCE=2, NORMAL_HORDE=3` |
| `BattlegroundQueueInvitationType` | enum | `NO_BALANCE=0, BALANCED=1, EVEN=2` |
| `BattlegroundStartTimeIntervals` | enum | `2M=120000, 1M=60000, 30S=30000, 15S=15000, NONE=0` (ms) |
| `BattlegroundTimeIntervals` | enum | `CHECK_PLAYER_POSITION=1000`, `RESURRECTION=30000`, `INVITATION_REMIND=20000`, `INVITE_ACCEPT_WAIT=90000`, `TIME_AUTOCLOSE=120000`, etc. |
| `BattlegroundPointCaptureStatus` | enum class | `AllianceControlled, AllianceCapturing, Neutral, HordeCapturing, HordeControlled` |
| `BattlegroundTypeId` | enum (in `SharedDefines.h`) | `BATTLEGROUND_AV=1, WS=2, AB=3, EY=...`, etc. |
| `BattlegroundBracketId` | enum (in `DBCEnums.h`) | Index into per-level brackets (`MAX_BATTLEGROUND_BRACKETS = 9`) |
| `BattlegroundQueueTypeId` | struct | `(uint16 BattlemasterListId, IdType, bool rated, uint8 teamSize)` packed key |
| `ArenaTeamTypes` | enum | `2v2=2, 3v3=3, 5v5=5` |
| `ArenaTeamCommandErrors` | enum | Big set of `ERR_ARENA_TEAM_*` returned to client |
| `BattlegroundSpells` | enum | Spell ids (resurrection, preparation 32727/44521, recently-dropped flags 42792/50326/50327, mercenary, honorable defender) |
| `EncounterFrameType` *(no, lives in instances)* | — | n/a |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `BattlegroundMgr::Update(diff)` | Tick all queues, run `m_QueueUpdateScheduler`, fire next rated arena update, gen new instances | `BattlegroundQueue::BattlegroundQueueUpdate`, `CreateNewBattleground` |
| `BattlegroundMgr::CreateNewBattleground(queueId, bracket)` | Allocate a new instance from template; clone subclass via `Battleground(template)`; assign instanceID | `GetBattlegroundTemplateByTypeId`, `CreateClientVisibleInstanceId` |
| `BattlegroundMgr::SendToBattleground(player, instanceId, type)` | Teleport into the instance map | `Map::TeleportTo` |
| `BattlegroundMgr::LoadBattlegroundTemplates()` | Load `battleground_template`, link to `BattlemasterList.dbc`, set start locs from `WorldSafeLocs.dbc` | `WorldDatabase`, DBC stores |
| `BattlegroundMgr::LoadBattleMastersEntry()` | Load `battlemaster_entry` mapping creature entry → bgTypeId | `WorldDatabase` |
| `BattlegroundMgr::BuildBattlegroundStatus*` (5 variants) | Build `SMSG_BATTLEFIELD_STATUS_*` packet sets | `WorldPackets::Battleground::*` |
| `BattlegroundMgr::SendBattlegroundList(player, guid, type)` | Send list of joinable BGs to client (battlemaster gossip) | `WorldPackets::Battleground::BattlefieldList` |
| `BattlegroundMgr::ScheduleQueueUpdate(mmr, queueId, bracket)` | Defers queue tick | append to `m_QueueUpdateScheduler` |
| `BattlegroundQueue::AddGroup(leader, group, team, bracketEntry, isPremade, rating, mmr, arenaTeamId)` | Insert into `m_QueuedGroups[bracket][groupType]`; set up offline tracker | constructs `GroupQueueInfo` |
| `BattlegroundQueue::RemovePlayer(guid, decreaseInvitedCount)` | Remove from m_QueuedPlayers + `GroupQueueInfo::Players`; cleanup empty groups | various |
| `BattlegroundQueue::BattlegroundQueueUpdate(diff, bracket, minRating)` | Heart of matchmaking: try CheckPremade, then CheckNormal, then CheckSkirmishForSameFaction; build SelectionPool; invite | `InviteGroupToBG`, `FillPlayersToBG` |
| `BattlegroundQueue::CheckPremadeMatch / _Normal / _SkirmishForSameFaction` | 3 matching strategies | enumerates groups |
| `BattlegroundQueue::FillPlayersToBG(bg, bracket)` | Top-up an existing in-progress BG with queued solos | invokes `Battleground::AddPlayer` indirectly |
| `BattlegroundQueue::IsPlayerInvited(guid, instance, removeTime)` | Used by `BGQueueInviteEvent::Execute` to abort if state changed | hash-map lookup |
| `BattlegroundQueue::GetAverageQueueWaitTime(ginfo, bracket)` | Rolling avg over last 10 invites | ring buffer `m_WaitTimes` |
| `Battleground::Update(diff)` | Top-level match tick; calls 5 sub-stages | `_ProcessOfflineQueue`, `_CheckSafePositions`, `_ProcessJoin/Progress/Leave`, `_ProcessPlayerPositionBroadcast`, `PostUpdateImpl` |
| `Battleground::AddPlayer(player, queueId)` | Subclass-implemented: teleports to start location, applies preparation buffs, pushes onto raid | `AddOrSetPlayerToCorrectBgGroup`, `Group::AddMember` |
| `Battleground::RemovePlayerAtLeave(guid, transport, sendPacket)` | Tears down player state; sends `SMSG_BATTLEFIELD_STATUS_NONE` if intentional leave | `BattlegroundQueue::RemovePlayer` |
| `Battleground::EndBattleground(winner)` | Award honor / arena points / marks; play sound; flip status to `WAIT_LEAVE`; schedule auto-close | `RewardHonor`, `ArenaTeam::MemberWon/Lost` |
| `Battleground::StartBattleground()` | Flip status `WAIT_JOIN → IN_PROGRESS`; open doors via `StartingEventOpenDoors`; remove preparation auras | subclass override `StartingEventOpenDoors` |
| `Battleground::SendPacketTo{Team,All}` | Broadcast helpers | iterate `m_Players` |
| `Battleground::UpdateWorldState(id, value, hidden)` | Push WS to all players | `Player::SendDirectMessage` |
| `Battleground::HandleAreaTrigger(player, trigger, entered)` | Subclass override; e.g. flag pick-up triggers in WSG, gate triggers in SotA | per-zone |
| `Battleground::HandleKillPlayer / KillUnit` | Update score, drop flag if carrier, award honor | `UpdatePlayerScore`, `EventPlayerDroppedFlag` |
| `Battleground::EventPlayerDroppedFlag / ClickedOnFlag` | Subclass-implemented for capture BGs | per-zone |
| `Battleground::AddObject / AddCreature / AddSpiritGuide / DelObject / DelCreature` | Per-instance world spawn helpers | `Map::Add*` |
| `Battleground::DoorOpen / DoorClose` | Toggle GO state by type | iterate `BgObjects` |
| `Battleground::GetPrematureWinner()` | Side with more players when other side abandons | per-zone override |
| `Battleground::AddPlayerPosition / RemovePlayerPosition` | Tracks flag carrier markers for `SMSG_BATTLEGROUND_PLAYER_POSITIONS` | vector |
| `Battleground::CastSpellOnTeam / RemoveAuraOnTeam / RewardHonorToTeam / RewardReputationToTeam` | Bulk team hooks | iterate |
| `Arena::EndBattleground(winner)` | Override: compute MMR delta + apply to ArenaTeams; send win/loss | `ArenaTeam::WonAgainst/LostAgainst` |
| `ArenaTeam::Create(captain, type, name, colors)` | Insert into `arena_team` and `arena_team_member` rows | CharacterDatabase |
| `ArenaTeam::AddMember(guid)` | Validate, push member, save | CharacterDatabase |
| `ArenaTeam::DelMember(guid, cleanDb)` | Remove member; auto-disband if empty | `Disband` |
| `ArenaTeam::Disband(session?)` | Close team; refund nothing; clean rows | `BroadcastEvent`, `CharacterDatabase` |
| `ArenaTeam::FinishWeek()` | Reset weekly counters | bool ret |
| `ArenaTeam::MemberWon / MemberLost / OfflineMemberLost` | Apply rating change to member + team stats | math via `WonAgainst` / `LostAgainst` |
| `ArenaTeam::WonAgainst / LostAgainst` | ELO-style rating change | uses `GetChanceAgainst` |
| `ArenaTeam::GetMatchmakerRatingMod / GetRatingMod` | Per-game rating delta calc | exponent table |
| `ArenaTeamMgr::LoadArenaTeams()` | Bulk load on startup | CharacterDatabase |
| `ArenaTeamMgr::GenerateArenaTeamId()` | Allocate next id | counter |

---

## 5. Module dependencies

**Depends on:**
- `Maps` — `BattlegroundMap` is the runtime parent (subclass of `Map`); `Battleground::SetBgMap` couples them. `MapManager::CreateBattlegroundMap` allocates.
- `Groups` — `Battleground::AddOrSetPlayerToCorrectBgGroup`, `SetBgRaid`. The match generates an in-memory raid group that exists only for the BG lifetime.
- `Player` — full lifecycle: `Player::TeleportTo`, `RemoveAura`, `CastSpell`, `SendDirectMessage`. Also `Player::CanJoinToBattleground`.
- `WorldSession` — opcode handlers (Section 7).
- `DataStores` — `BattlemasterListEntry`, `MapEntry`, `WorldSafeLocsEntry`, `PVPDifficultyEntry` (per-bracket level limits), `MapDifficultyEntry`.
- `Database (CharacterDatabase + WorldDatabase)` — see Section 6.
- `Achievement / Criteria` — many BG-specific criteria (e.g. flag captures, K/D ratios, speed-runs).
- `LFG` — none directly; BGs use their own queue, but LFG status flags are checked to prevent double-queue.
- `World/Time` — `GameTime::GetGameTime()` for `JoinTime`, `OfflineRemoveTime`.
- `EventProcessor` — `BGQueueInviteEvent` and `BGQueueRemoveEvent` schedule on the per-queue `m_events`.
- `WorldStateMgr` — `UpdateWorldState` calls integrate.
- `ZoneScript` parent — generic `OnCreatureCreate`, `ProcessEvent`, `GetData`/`SetData`.

**Depended on by:**
- `WorldSession::HandleBattlemaster*` opcodes — every queue/leave/port/list opcode calls into `BattlegroundMgr` and/or `BattlegroundQueue`.
- `Player::Update` — checks BG status & invites timeouts.
- `Group` — checks `GetBattlegroundQueueId` before disband.
- All ~12 zone scripts (`BattlegroundAB`, `_AV`, `_BE`, ... `_WS`).
- GM commands `.bg`, `.arena`, `.bf` (battlefield uses similar plumbing).
- `OutdoorPvP` partially mirrors patterns.

---

## 6. SQL / DB queries (if any)

### CharacterDatabase tables

| Table | Schema highlights | Purpose |
|---|---|---|
| `arena_team` | `arenaTeamId PK`, `name`, `captainGuid`, `type`, `rating`, `seasonGames`, `seasonWins`, `weekGames`, `weekWins`, `rank`, `backgroundColor`, `emblemStyle`, `emblemColor`, `borderStyle`, `borderColor` | Persistent team registry |
| `arena_team_member` | `(arenaTeamId, guid) PK`, `weekGames`, `weekWins`, `seasonGames`, `seasonWins`, `personalRating`, `matchMakerRating` | Member roster + per-member rating |
| `character_arena_stats` | `(guid, slot)`, `matchMakerRating` | MMR tracked even outside teams |

### WorldDatabase tables

| Table | Schema highlights | Purpose |
|---|---|---|
| `battleground_template` | `ID`, `MinPlayersPerTeam`, `MaxPlayersPerTeam`, `MinLvl`, `MaxLvl`, `AllianceStartLoc`, `AllianceStartO`, `HordeStartLoc`, `HordeStartO`, `StartMaxDist`, `Weight`, `ScriptName` | Per-BG-type config; loaded by `LoadBattlegroundTemplates` |
| `battleground_door` (referenced in some BGs, e.g. SotA) | gate spawn data | Per-BG door layout |
| `battlemaster_entry` | `(entry, bg_template)` | Maps creature entry → `BattlegroundTypeId` so battlemaster gossip works |
| `game_event_arena_seasons` | `eventEntry`, `season` | Season holiday rotations |

### Prepared statements (CharacterDatabase, names from `CharacterDatabase.cpp`)

| Statement | Purpose |
|---|---|
| `CHAR_SEL_ARENA_TEAMS` / `_ARENA_TEAM_MEMBERS` | Bulk load on startup |
| `CHAR_INS_ARENA_TEAM` | Insert new team |
| `CHAR_DEL_ARENA_TEAM` / `_ARENA_TEAM_MEMBERS_BY_TEAM` | Disband cleanup |
| `CHAR_INS_ARENA_TEAM_MEMBER` / `CHAR_DEL_ARENA_TEAM_MEMBER` | Roster |
| `CHAR_UPD_ARENA_TEAM_STATS` | Save team rating + week/season counters |
| `CHAR_UPD_ARENA_TEAM_MEMBER` | Save member rating + counters |
| `CHAR_UPD_PERSONAL_ARENA_RATING` | Personal rating |
| `CHAR_UPD_ARENA_TEAM_CAPTAIN` | Captain swap |
| `CHAR_UPD_ARENA_TEAM_NAME` | Rename |
| `CHAR_REP_CHARACTER_ARENA_STATS` | Off-team MMR persistence |

### DBC/DB2 stores read

| Store | What it loads | Read by |
|---|---|---|
| `BattlemasterListStorage` | BattlemasterList.dbc | `BattlegroundTemplate::BattlemasterEntry` |
| `MapStorage` | Map.db2 | `BattlegroundMap` instances |
| `MapDifficultyStorage` | MapDifficulty.db2 | n/a |
| `PVPDifficultyStorage` | PVPDifficulty.dbc | per-bracket level → `BracketId` |
| `WorldSafeLocsStorage` | WorldSafeLocs.dbc | `BattlegroundTemplate::StartLocation`, graveyards |
| `HolidaysStorage` | Holidays.dbc | `BGTypeToWeekendHolidayId`, `IsBGWeekend` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_BATTLEMASTER_HELLO` | client → server | `WorldSession::HandleBattlemasterHelloOpcode` → `BattlegroundMgr::SendBattlegroundList` |
| `CMSG_BATTLEMASTER_JOIN` | client → server | `HandleBattlemasterJoinOpcode` → `BattlegroundQueue::AddGroup` |
| `CMSG_BATTLEMASTER_JOIN_ARENA` | client → server | `HandleBattlemasterJoinArena` (rated arena) |
| `CMSG_BATTLEMASTER_JOIN_SKIRMISH` | client → server | `HandleBattlemasterJoinSkirmish` |
| `CMSG_BATTLEFIELD_LIST` | client → server | `HandleBattlefieldListOpcode` → `BattlegroundMgr::SendBattlegroundList` |
| `CMSG_BATTLEFIELD_PORT` | client → server | `HandleBattleFieldPortOpcode` → enter or leave queue based on flag |
| `CMSG_BATTLEFIELD_LEAVE` | client → server | `HandleBattlefieldLeaveOpcode` → `RemovePlayerAtLeave` |
| `CMSG_REQUEST_BATTLEFIELD_STATUS` | client → server | `HandleRequestBattlefieldStatusOpcode` → `BuildBattlegroundStatus*` for each slot |
| `CMSG_AREA_SPIRIT_HEALER_QUERY` | client → server | `HandleAreaSpiritHealerQueryOpcode` → time until next AoE rez |
| `CMSG_AREA_SPIRIT_HEALER_QUEUE` | client → server | `HandleAreaSpiritHealerQueueOpcode` → opt in for next 30s rez |
| `CMSG_REPORT_PVP_PLAYER_AFK` | client → server | `HandleReportPvPAFK` → strikes against player; auto-kick |
| `CMSG_PVP_LOG_DATA` (sometimes prefixed `CMSG_INSPECT_*`) | client → server | Trigger end-of-match data fetch |
| `CMSG_BATTLEMASTER_JOIN_BRAWL` | client → server | `Handle_NULL` (Cataclysm+; not used in 3.4.3) |
| `CMSG_BATTLEMASTER_JOIN_RATED_SOLO_SHUFFLE` | client → server | `Handle_NULL` (modern) |
| `SMSG_BATTLEFIELD_LIST` | server → client | Joinable BGs at this battlemaster + level bracket |
| `SMSG_BATTLEFIELD_STATUS_NONE` | server → client | Slot empty (quit / kicked) |
| `SMSG_BATTLEFIELD_STATUS_QUEUED` | server → client | In queue with avg-wait |
| `SMSG_BATTLEFIELD_STATUS_NEED_CONFIRMATION` | server → client | Invitation sent (90s timer) |
| `SMSG_BATTLEFIELD_STATUS_ACTIVE` | server → client | Player is in-match |
| `SMSG_BATTLEFIELD_STATUS_FAILED` | server → client | Error (already in queue, deserter, level mismatch) |
| `SMSG_BATTLEFIELD_STATUS_GROUP_PROPOSAL_FAILED` | server → client | Some member failed checks |
| `SMSG_BATTLEFIELD_STATUS_WAIT_FOR_GROUPS` | server → client | (modern) waiting on assemble |
| `SMSG_BATTLEFIELD_PORT_DENIED` | server → client | Port denied after invite (rare) |
| `SMSG_PVP_LOG_DATA` | server → client | End-of-match scoreboard (kills/deaths/honor + per-BG bonuses) |
| `SMSG_BATTLEGROUND_PLAYER_POSITIONS` | server → client | Flag carrier nameplate icons |
| `SMSG_BATTLEGROUND_PLAYER_JOINED / _LEFT` | server → client | Mid-match roster delta |
| `SMSG_REQUEST_PVP_REWARDS_RESPONSE` | server → client | Daily / weekly reward preview |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-pvp` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-pvp/Cargo.toml` | `file` | 1 | 10 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-pvp/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `crates/wow-maps` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-pvp/Cargo.toml` — declares deps on `wow-core`, `wow-constants`. Empty package.
- `crates/wow-pvp/src/lib.rs` — empty file (0 lines).
- *No* handlers in `crates/wow-world/src/handlers/` for any battlemaster / battlefield / area-spirit-healer opcode.
- *No* `BattlegroundMap` subclass in `crates/wow-maps/`.

**What's implemented:**
- Nothing. Scaffold exists but `lib.rs` is empty; no types, no consts.

**What's missing vs C++:**
- Absolutely everything — see Section 9 (37 sub-tasks, mostly H/XL).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — there is no Rust code to diverge yet. The risk is design-time: the Rust model must NOT replicate the C++ `Battleground*` / `Arena*` / `BattlefieldStatus*` packet split mistake (5 different opcodes for what is logically one status enum). Use a single Rust `BattlefieldStatus` enum + serializer.

**Tests existing:**
- 0 tests anywhere.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#BATTLEGROUNDS.WBS.001** Cerrar la migracion auditada de `game/Battlegrounds/Arena.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Arena.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.002** Cerrar la migracion auditada de `game/Battlegrounds/Arena.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Arena.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.003** Cerrar la migracion auditada de `game/Battlegrounds/ArenaScore.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaScore.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.004** Cerrar la migracion auditada de `game/Battlegrounds/ArenaScore.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaScore.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.005** Partir y cerrar la migracion auditada de `game/Battlegrounds/ArenaTeam.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaTeam.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 834 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.006** Cerrar la migracion auditada de `game/Battlegrounds/ArenaTeam.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaTeam.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.007** Cerrar la migracion auditada de `game/Battlegrounds/ArenaTeamMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaTeamMgr.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.008** Cerrar la migracion auditada de `game/Battlegrounds/ArenaTeamMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/ArenaTeamMgr.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.009** Partir y cerrar la migracion auditada de `game/Battlegrounds/Battleground.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1883 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.010** Partir y cerrar la migracion auditada de `game/Battlegrounds/Battleground.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 604 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.011** Partir y cerrar la migracion auditada de `game/Battlegrounds/BattlegroundMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundMgr.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 754 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.012** Cerrar la migracion auditada de `game/Battlegrounds/BattlegroundMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundMgr.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.013** Partir y cerrar la migracion auditada de `game/Battlegrounds/BattlegroundQueue.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundQueue.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1089 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.014** Cerrar la migracion auditada de `game/Battlegrounds/BattlegroundQueue.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundQueue.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.015** Cerrar la migracion auditada de `game/Battlegrounds/BattlegroundScore.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundScore.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.016** Cerrar la migracion auditada de `game/Battlegrounds/BattlegroundScore.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/BattlegroundScore.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.017** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundAB.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAB.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 529 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.018** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundAB.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAB.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.019** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundAV.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1483 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.020** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundAV.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1734 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.021** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundBE.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundBE.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.022** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundBE.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundBE.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.023** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundBFG.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundBFG.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.024** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundBFG.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundBFG.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.025** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundDS.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundDS.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.026** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundDS.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundDS.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.027** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundEY.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundEY.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 533 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.028** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundEY.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundEY.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.029** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundIC.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundIC.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 913 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.030** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundIC.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundIC.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1056 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.031** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundNA.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundNA.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.032** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundNA.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundNA.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.033** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundRL.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundRL.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.034** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundRL.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundRL.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.035** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundRV.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundRV.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.036** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundRV.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundRV.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.037** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundSA.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundSA.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1036 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.038** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundSA.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundSA.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 710 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.039** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundTP.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundTP.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.040** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundTP.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundTP.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.041** Partir y cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundWS.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundWS.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 580 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.042** Cerrar la migracion auditada de `game/Battlegrounds/Zones/BattlegroundWS.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundWS.h`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEGROUNDS.WBS.043** Cerrar la migracion auditada de `game/Battlegrounds/enuminfo_ArenaTeam.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/enuminfo_ArenaTeam.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world/src/handlers`, `crates/wow-maps`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#BG.1** Populate `crates/wow-pvp/src/lib.rs` w/ module skeleton (`mod battleground; mod queue; mod mgr; mod arena; mod arena_team; mod templates; mod scores; mod zones;`) (L)
- [ ] **#BG.2** Port all enums (`BattlegroundStatus`, `BattlegroundQueueIdType`, `BattlegroundQueueGroupTypes`, `BattlegroundPointCaptureStatus`, `BattlegroundStartTimeIntervals`, `ArenaTeamTypes`) into `wow-constants` (L)
- [ ] **#BG.3** Port `BattlegroundQueueTypeId` packed key struct + `pack/unpack` (L)
- [ ] **#BG.4** Port `BattlegroundTemplate`, `BattlegroundData`, `BattlegroundPlayer`, `BattlegroundObjectInfo` (M)
- [ ] **#BG.5** Implement `BattlegroundMgr` singleton w/ `bgDataStore: HashMap<BattlegroundTypeId, BattlegroundData>`, `m_BattlegroundQueues: HashMap<BattlegroundQueueTypeId, BattlegroundQueue>`, BattleMastersMap (H)
- [ ] **#BG.6** Implement `LoadBattlegroundTemplates` from `battleground_template` table — async query + DBC join (M)
- [ ] **#BG.7** Implement `LoadBattleMastersEntry` + `CheckBattleMasters` (validates each entry's creature exists) (M)
- [ ] **#BG.8** Implement `CreateClientVisibleInstanceId` + per-bracket id generator (L)
- [ ] **#BG.9** Implement `CreateNewBattleground` factory dispatching on `BattlegroundTypeId` (M)
- [ ] **#BG.10** Implement `BattlegroundMgr::Update(diff)` — process queue scheduler, fire rated-arena update on interval (M)
- [ ] **#BG.11** Implement `SendToBattleground` + holiday-weekend logic + `IsRandomBattleground` + `IsBGWeekend` (M)
- [ ] **#BG.12** Implement `BattlegroundQueue` core + `m_QueuedGroups[BRACKETS][4]` + `m_QueuedPlayers` (H)
- [ ] **#BG.13** Implement `AddGroup` w/ `GroupQueueInfo` ctor and `PlayerQueueInfo` back-refs (M)
- [ ] **#BG.14** Implement `RemovePlayer` w/ proper invited-count decrement & empty-group cleanup (M)
- [ ] **#BG.15** Implement `BattlegroundQueueUpdate` + `CheckPremadeMatch` + `CheckNormalMatch` + `CheckSkirmishForSameFaction` + `SelectionPool` (XL — split into 4 sub-PRs)
- [ ] **#BG.16** Implement `FillPlayersToBG` (top-up running BG) (M)
- [ ] **#BG.17** Implement `InviteGroupToBG` + schedule `BGQueueInviteEvent` / `BGQueueRemoveEvent` on `m_events` (M)
- [ ] **#BG.18** Implement avg-wait-time ring buffer (`m_WaitTimes`) (L)
- [ ] **#BG.19** Implement `Battleground` base — fields, `SetStatus`, `Start/EndBattleground`, raid auto-formation (H)
- [ ] **#BG.20** Implement `Battleground::Update` + 5 sub-stages (`_ProcessOfflineQueue`, `_CheckSafePositions`, `_ProcessJoin`, `_ProcessProgress`, `_ProcessLeave`, `_ProcessPlayerPositionBroadcast`) (H)
- [ ] **#BG.21** Implement starting countdown (4-step start delays + `BG_TEXT_*` broadcast text) + opening doors (M)
- [ ] **#BG.22** Implement `AddPlayer / RemovePlayerAtLeave` w/ teleport, prep-aura, raid join (H)
- [ ] **#BG.23** Implement `BattlegroundScore` polymorphic struct + `BuildPvPLogDataPacket` → `SMSG_PVP_LOG_DATA` (M)
- [ ] **#BG.24** Implement `RewardHonorToTeam`, `RewardReputationToTeam`, `CastSpellOnTeam`, `RemoveAuraOnTeam` (M)
- [ ] **#BG.25** Implement `UpdateWorldState`, `SendPacketToTeam/All`, `PlaySoundToTeam/All` (M)
- [ ] **#BG.26** Implement `AddObject / AddCreature / AddSpiritGuide / DelObject / DelCreature / DoorOpen / DoorClose` (M)
- [ ] **#BG.27** Implement `HandleAreaTrigger / HandleKillPlayer / HandleKillUnit` virtual dispatch hooks (M)
- [ ] **#BG.28** Implement `Player::CanJoinToBattleground` (deserter aura check, level bracket, BG marks limit) (M)
- [ ] **#BG.29** Port `Arena` subclass + arena-specific `EndBattleground` MMR delta application (H)
- [ ] **#BG.30** Port `ArenaTeam` w/ all DB CRUD (Create, AddMember, DelMember, Disband, SetCaptain, SetName, SaveToDB) (H)
- [ ] **#BG.31** Port ELO math: `GetChanceAgainst`, `WonAgainst`, `LostAgainst`, `GetMatchmakerRatingMod`, `GetRatingMod` (M)
- [ ] **#BG.32** Port `ArenaTeam::FinishWeek`, `FinishGame`, `MemberWon/Lost/OfflineMemberLost` (M)
- [ ] **#BG.33** Port `ArenaTeamMgr` w/ load/get-by-{id,name,captain}, ID generator (M)
- [ ] **#BG.34** Implement opcode handlers in `wow-world/src/handlers/`: `CMSG_BATTLEMASTER_HELLO`, `CMSG_BATTLEMASTER_JOIN`, `CMSG_BATTLEMASTER_JOIN_ARENA`, `CMSG_BATTLEMASTER_JOIN_SKIRMISH`, `CMSG_BATTLEFIELD_LIST`, `CMSG_BATTLEFIELD_PORT`, `CMSG_BATTLEFIELD_LEAVE`, `CMSG_REQUEST_BATTLEFIELD_STATUS`, `CMSG_AREA_SPIRIT_HEALER_QUERY`, `CMSG_AREA_SPIRIT_HEALER_QUEUE`, `CMSG_REPORT_PVP_PLAYER_AFK` (XL — split per opcode)
- [ ] **#BG.35** Implement `SMSG_BATTLEFIELD_*` packet builders (NONE, QUEUED, NEED_CONFIRMATION, ACTIVE, FAILED) and `SMSG_BATTLEFIELD_LIST` (M)
- [ ] **#BG.36** Implement `BattlegroundMap` (subclass of `Map`) — depends on `maps.md` work; provide hooks `OnPlayerEnter`, `OnPlayerLeave`, `Update` (M)
- [ ] **#BG.37** Implement Warsong Gulch (`zones/wsg.rs`) — flag carry, capture limit 3 (H)
- [ ] **#BG.38** Implement Arathi Basin (`zones/ab.rs`) — 5 cap nodes, resource race to 1600 (H)
- [ ] **#BG.39** Implement Eye of the Storm (`zones/eots.rs`) — flag-mid + 4 cap points (H)
- [ ] **#BG.40** Implement Strand of the Ancients (`zones/sa.rs`) — gate destruction, demolisher vehicles, attack/defend rotation, relic timer (XL — split into 3 sub-PRs)
- [ ] **#BG.41** Implement Isle of Conquest (`zones/ic.rs`) — vehicles, hangar, docks, keep, airships (XL — split per zone area)
- [ ] **#BG.42** Implement Alterac Valley (`zones/av.rs`) — 40v40, 7+ towers, captains, mines, NPCs (XL — split into 5+ sub-PRs)
- [ ] **#BG.43** Implement arena maps (Blade's Edge, Dalaran Sewers, Nagrand, Ruins of Lordaeron, Ring of Valor) (XL — 5 separate small modules)
- [ ] **#BG.44** Wire `Battleground::EndBattleground` → `Arena::EndBattleground` → `ArenaTeam::MemberWon/Lost` for rated matches (M)
- [ ] **#BG.45** Implement deserter debuff application on flee + "join again in 15 min" client message (M)
- [ ] **#BG.46** GM commands: `.bg`, `.arena`, `.arenateam` (rename, add, remove, disband) (M)
- [ ] **#BG.47** Audit XPathAdmin / XPath gossip flow that triggers `CMSG_BATTLEMASTER_HELLO` (gossip menu wiring) (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#BATTLEGROUNDS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 43 files / 17381 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-pvp`. | `cargo test -p wow-constants && cargo test -p wow-core && cargo test -p wow-pvp` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#BATTLEGROUNDS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 43 files / 17381 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-pvp`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#BATTLEGROUNDS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 43 files / 17381 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-pvp`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#BATTLEGROUNDS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 43 files / 17381 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-pvp`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `BattlegroundQueue::AddGroup` for a 5-man premade puts entries in `PREMADE_*` slots, while solo adds go to `NORMAL_*`.
- [ ] Test: `CheckPremadeMatch` succeeds only when both factions have premades within `PremadeMatchTime` window AND cardinality matches.
- [ ] Test: `CheckNormalMatch` produces an `N+1 vs N` invite under `BG_QUEUE_INVITATION_TYPE_BALANCED` policy.
- [ ] Test: `CheckSkirmishForSameFaction` fires only for unrated arena queues with `BattlegroundQueueIdType::ArenaSkirmish`.
- [ ] Test: `BGQueueInviteEvent` fires at +60s from add and re-sends `STATUS_NEED_CONFIRMATION`.
- [ ] Test: `BGQueueRemoveEvent` fires at +1m20s from invite if no port; player removed.
- [ ] Test: `Battleground::AddPlayer` teleports to `template.StartLocation[teamId]`, applies preparation aura 44521 (BG) or 32727 (arena).
- [ ] Test: `StartBattleground` opens doors and removes preparation auras after final countdown step.
- [ ] Test: `RewardHonorToTeam` honor calc honors `BG_AWARD_ARENA_POINTS_MIN_LEVEL` floor and `IsBGWeekend` multiplier.
- [ ] Test: `EndBattleground(winner)` flips status to `WAIT_LEAVE`, sets `m_EndTime = TIME_AUTOCLOSE_BATTLEGROUND` (120000 ms).
- [ ] Test: `RemovePlayerAtLeave` when status `IN_PROGRESS` applies deserter aura; when `WAIT_LEAVE` does NOT apply deserter.
- [ ] Test: Round-trip `BattlegroundTemplate` from DB, ensure `MaxStartDistSq = (StartMaxDist)^2`.
- [ ] Test: ArenaTeam ELO — losing a 1500-vs-1500 match yields exactly the C++ value of `LostAgainst(1500, 1500, &delta)` (golden-vector).
- [ ] Test: `ArenaTeam::FinishWeek` zeros `weekGames` / `weekWins` for team and all members.
- [ ] Test: `ArenaTeam::Disband` deletes both `arena_team` and `arena_team_member` rows in one transaction.
- [ ] Test: `MemberWon` applies personal-rating delta plus team-rating delta independently.
- [ ] Test: `BattlegroundMgr::CreateNewBattleground` emits client-visible instance ID in expected range for the bracket.
- [ ] Test: `Battleground::HandleAreaTrigger` for the WSG flag-room trigger fires `EventPlayerClickedOnFlag` only for the opposing team.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 43 files / 17381 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp` | `crates/wow-pvp/` (currently empty scaffold — `Cargo.toml` only, no `lib.rs` content) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#BATTLEGROUNDS.DIV.001` | `crates/wow-pvp` (`exists_empty`, 0 Rust lines) | 43 C++ files / 17381 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#BATTLEGROUNDS.DIV.002` | `crates/wow-pvp/src/lib.rs` (`exists_empty`, 0 Rust lines) | 43 C++ files / 17381 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#BATTLEGROUNDS.DIV.003` | `crates/wow-maps` (`missing_declared_path`, 0 Rust lines) | 43 C++ files / 17381 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Battleground.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlegrounds/Zones/BattlegroundAV.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **The C++ `BattlegroundQueueTypeId` is a packed 64-bit struct.** Do NOT serialize/deserialize as struct-by-struct; serialize as the same packed `(BMListId<<x | type<<y | rated<<z | teamSize)` integer wherever it crosses the wire / DB.
- **Bracket math.** `MAX_BATTLEGROUND_BRACKETS = 9` corresponds to level windows from `PVPDifficulty.dbc`. Different BGs have different bracket counts in 3.4.3; a sub-9 BG must skip allocations for unused brackets — easy to off-by-one.
- **Invite count vs invited+queued count.** `m_InvitedAlliance` / `m_InvitedHorde` are decremented by `RemovePlayerAtLeave`. If you invite a 2-man group and one accepts, one declines, you must decrement only on actual decline not on invite issuance — bug in early TC.
- **Mercenary contracts** (3.x retail back-port — entries `SPELL_MERCENARY_CONTRACT_HORDE = 193472`, `_ALLIANCE = 193475`) let players queue as the opposite faction. Affects `BattlegroundPlayer::Mercenary`. Not active in classic 3.4.3 unless a custom flag is set; document but disable by default.
- **`SPELL_PREPARATION = 44521`** removes mana cost / increases regen during BG warm-up. `SPELL_ARENA_PREPARATION = 32727` is the arena variant. Use **32727 not 32728** — comment in code explicitly says 32728 is wrong.
- **`SPELL_RECENTLY_DROPPED_*_FLAG`** auras (`42792 / 50326 / 50327`) make a flag *unselectable* for 5s after drop, preventing instant re-grab cheese.
- **Auto-close after end.** `m_EndTime = 120000` ms; players who don't manually leave are teleported out at 0 by `_ProcessLeave`. Do NOT delete the BG instance until everyone has left or `m_EndTime` is exhausted, or you'll crash on dangling player ptrs.
- **`m_OfflineQueue`** is a deque of guids whose `OfflineRemoveTime != 0`. After `MAX_OFFLINE_TIME = 300s`, the player is purged. Logout in-match doesn't immediately remove; that's intentional so players can reconnect.
- **`Group* m_BgRaids[2]`** — the BG creates a synthetic raid for each team (so loot/heal/buff group mechanics work). This Group is *not* persisted; do not call `Group::SaveToDB`. Tear down at `EndBattleground`.
- **`PVPDifficultyEntry`** holds `(MapID, RangeIndex, MinLevel, MaxLevel)`. Used to bracket players in queue. Different from `MapDifficulty`.
- **Queue update scheduler** is throttled — `m_QueueUpdateScheduler` runs at most every `BATTLEGROUND_OBJECTIVE_UPDATE_INTERVAL = 1000ms`, not every server tick. Same for player-position broadcast (5000 ms via `PLAYER_POSITION_UPDATE_INTERVAL`).
- **`SMSG_BATTLEFIELD_STATUS_*` are 5 separate opcodes**, not 1 with a tag. Do not collapse them in the Rust port — clients expect distinct opcodes.
- **Rated arena MMR** is stored in `arena_team_member.matchMakerRating` AND `character_arena_stats.matchMakerRating` (the latter survives team disband). When a team disbands, retain MMR via `character_arena_stats`; new team load merges these.
- **`ArenaTeam::IsFighting()`** returns true when ANY member is in an arena instance — used to block disband / member kick mid-match.
- **`ArenaTeamCommandErrors` numbering** matches client's expected error packet payload bytes; do not re-number.
- **Premature win** (`Battleground::GetPrematureWinner`) — when a BG has too few players (defection), the side with more players wins. Subclasses override (e.g. AB checks resource lead).
- **`AddPlayer` is called during `_ProcessJoin`**, NOT directly from the queue invite. Queue invite sends `STATUS_NEED_CONFIRMATION`; only after `CMSG_BATTLEFIELD_PORT` does the BG actually call `AddPlayer`.
- **AV is special**: 40v40 means the bracket lookup, the spawn count, and the score broadcast are all O(N²) in many places. Watch for hotspots when porting.
- **BFG.cpp / TP.cpp are stubs** in the 3.4.3 fork (26 lines each) — do not translate the stub; just leave the `BattlegroundType` registered as unsupported.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Battleground : public ZoneScript` | `pub trait Battleground: ZoneScript` + `pub struct BattlegroundBase` (data) | Trait for virtual hooks; struct for shared state |
| `class Arena : public Battleground` | `pub struct Arena { base: BattlegroundBase, .. }` impl `Battleground` | Composition over inheritance |
| Per-zone subclasses (`BattlegroundWS`, etc.) | One module per zone in `crates/wow-pvp/src/zones/` | Explicit dispatch via factory function |
| `BattlegroundMgr` singleton | `pub struct BattlegroundMgr` + `OnceLock<BattlegroundMgr>` | `sBattlegroundMgr` → `wow_pvp::mgr()` |
| `BattlegroundQueue` | `pub struct BattlegroundQueue` per `BattlegroundQueueTypeId` in `DashMap` | Queue ops wrapped in `parking_lot::Mutex` |
| `std::map<ObjectGuid, BattlegroundPlayer>` | `HashMap<ObjectGuid, BattlegroundPlayer>` | `m_Players` |
| `std::list<GroupQueueInfo*>` | `LinkedList<GroupQueueInfo>` or `IndexedSlab` | `Vec` is fine for small N; do NOT use `LinkedList` for hot path |
| `std::deque<ObjectGuid> m_OfflineQueue` | `VecDeque<ObjectGuid>` | Direct map |
| `EventProcessor m_events` | `wow-shared-events::EventProcessor` (existing) | Schedule timed events |
| `BasicEvent` subclasses (`BGQueueInviteEvent`, `BGQueueRemoveEvent`) | `Box<dyn TimedEvent>` w/ `Execute(now, p_time)` | Same shape |
| `class ArenaTeam` + DB save | `pub struct ArenaTeam` + `save_to_db(tx) -> Result<()>` | Async DB |
| `MemberList = std::list<ArenaTeamMember>` | `Vec<ArenaTeamMember>` | size ≤ 10 |
| `WorldPackets::Battleground::*` | `wow-shared-packets::battleground::*` (NEW — must port) | Strict structs |
| `EnumUtils: DESCRIBE THIS` macros | `strum::Display` derive | C++ macros are codegen for clients of the enum |
| `static BattlegroundMgr* instance()` | `pub fn instance() -> &'static BattlegroundMgr` | Use `OnceLock::get_or_init` |
| `uint32 ScheduleQueueUpdate` (loose vec) | `Vec<ScheduledQueueUpdate>` w/ dedupe via `==` | Same shape |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

❌ confirmado. Auditado contra `/home/server/rustycore/crates/`.

**Hallazgos clave:**
- `crates/wow-pvp/src/lib.rs` está literalmente vacío (`wc -l` = 0 líneas). `Cargo.toml` declara solo `wow-core` y `wow-constants` — sin `wow-database`, sin `wow-network`, sin nada que pudiera persistir o transmitir BG state.
- Constants existen en `wow-constants/src/opcodes.rs`: `BattlemasterHello=0x32b1`, `BattlemasterJoin=0x3520`, `BattlemasterJoinArena=0x3521`, `BattlemasterJoinSkirmish=0x3522`, `BattlefieldList=0x3181`, `BattlefieldPort=0x3525`, `BattlefieldLeave=0x3175`, `RequestBattlefieldStatus=0x35dd`, `AreaSpiritHealerQuery=0x34b0`, `AreaSpiritHealerQueue=0x34b1`. Server-bound: `BattlefieldList`, `BattlefieldStatusActive/Failed/NeedConfirmation/None/Queued/WaitForGroups`, `BattlefieldPortDenied`. Las constantes están — los handlers no.
- Búsqueda de registros (`opcode: ClientOpcodes::Battlemaster*` o `Battlefield{List,Port,Leave}` o `AreaSpiritHealer*` o `ReportPvpAfk`) en `wow-world/src/handlers/` y `wow-handler/src/`: **0 resultados**. **Único registrado: `RequestBattlefieldStatus`** en `handlers/misc.rs:191` → handler vacío `handle_request_battlefield_status` en L598 (`{}`).
- Conteo de handlers BG registrados confirmado por la doc: **1 stub vacío de 11 esperados.** Los otros 10 opcodes (HELLO, JOIN, JOIN_ARENA, JOIN_SKIRMISH, LIST, PORT, LEAVE, AREA_SPIRIT_HEALER_QUERY, AREA_SPIRIT_HEALER_QUEUE, REPORT_PVP_PLAYER_AFK) **no están registrados en absoluto** — caen en el path `handle_unknown_packet`.
- `handlers/character.rs:2972` confirma además: gossip `option_npc=9 (Battlemaster)` solo emite `info!("Battlemaster interaction (stub)")` y retorna sin enviar packet alguno (no `BattlefieldList`, no `BattlemasterListEntry` lookup). El cliente click-derecho sobre un BattleMaster NPC abre menú gossip y luego se queda en blanco.

**Riesgo de UI hang silencioso (real, ya activo):**
- ⚠️ **CMSG_BATTLEMASTER_HELLO**: el `option_npc=9` en gossip select va al stub log-only — *no responde*. Cliente espera lista de BGs joinable; al no llegar, el menú se queda con la opción "I would like to go to the battleground" inerte. **Workaround actual: ninguno**. Test path: hablar con un Battlemaster NPC.
- ⚠️ **CMSG_BATTLEMASTER_JOIN / JOIN_ARENA / JOIN_SKIRMISH**: opcodes registrados como constantes, NO dispatched. Si el cliente los envía (forzar via macro / addon), caen en error log "unknown opcode". Inofensivo en el flujo normal porque `HELLO` ya bloquea.
- ⚠️ **CMSG_REQUEST_BATTLEFIELD_STATUS** está stubbed (silent ack). El cliente lo envía periódicamente para refrescar la mini-ventana de queue status. Sin respuesta → la mini-ventana muestra "—" forever pero NO bloquea login ni char screen. Bajo riesgo, sin embargo descarta la posibilidad de mostrar "Queue: position 1 of N" cuando se implemente.

**Acción:** dejar `❌ not started` en el badge. Recomendación táctica si se necesita unstuck el menú de Battlemaster antes de la migración completa: stub `handle_battlemaster_hello` que envíe `SMSG_BATTLEFIELD_LIST` con `count=0` (lista vacía) y cierre el gossip con un mensaje "Battlegrounds offline".
