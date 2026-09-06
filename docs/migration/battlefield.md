# Migration: Battlefield (Wintergrasp — Scheduled Zonal Battle)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/` (base) + `/home/server/woltk-trinity-legacy/src/server/scripts/Battlefield/` (Wintergrasp script)
> **Rust target crate(s):** `crates/wow-battlefield/` (NOT YET CREATED — must be added to workspace)
> **Layer:** L7
> **Status:** ❌ not started
> **Audited vs C++:** ✅ audited 2026-05-01 (❌ confirmed — last PvP to migrate)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Battlefield is TrinityCore's framework for **scheduled, zonal, world-PvP events** — open-world (not instanced) battles that fire on a fixed timer, recruit players from inside a zone or via queue, auto-form raid groups, and resolve with capture mechanics + tower destruction + vehicles. The only Battlefield active in WoLK 3.4.3 is **Wintergrasp** (zone 4197 on map 571 = Northrend), with `BattlefieldTB` (Tol Barad, Cataclysm) and Ashran (Warlords of Draenor) sketched but unused.

Wintergrasp specifically:
- A 3.5-hour idle phase (`m_NoWarBattleTime`, configurable) followed by a ~30-minute battle phase (`m_BattleTime`).
- 15-minute pre-battle "grouping" window: players in the zone are invited to join the queue; queue-joined players from elsewhere are pulled in.
- Defenders and Attackers — `m_DefenderTeam` switches based on previous-battle outcome and persists in `world` table (`battlefield_zone`).
- Capture objectives: 6 workshops (with capture-point control zones), 7 towers (3 attacker, 4 defender keep), gates, and the central Titan Relic.
- Vehicle ecosystem: siege engines (Alliance / Horde), catapults, demolishers, tower cannons; per-team vehicle caps based on workshop control.
- Win conditions: attackers click relic with last gate broken → win; OR timer expires with relic intact → defenders win. Tenacity buff stacks per side imbalance.
- Auto-raid grouping (separate from Group module) with up to 40 per team.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Battlefield/Battlefield.cpp` | 738 | `prefix` |
| `game/Battlefield/Battlefield.h` | 359 | `prefix` |
| `game/Battlefield/BattlefieldMgr.cpp` | 181 | `prefix` |
| `game/Battlefield/BattlefieldMgr.h` | 82 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Battlefield/Battlefield.h` | 359 | `Battlefield` base class + `BfGraveyard` + `BattlefieldControlZoneHandler` + enums (`BattlefieldState`, `BattlefieldObjectiveStates`, `BattlefieldTimers`) |
| `src/server/game/Battlefield/Battlefield.cpp` | 738 | Update loop, player-zone-enter/leave, queue invite/accept, raid group auto-formation, AFK kick, graveyard logic |
| `src/server/game/Battlefield/BattlefieldMgr.h` | 82 | `BattlefieldMgr` singleton; per-map registry of Battlefield instances + zone→Battlefield index |
| `src/server/game/Battlefield/BattlefieldMgr.cpp` | 181 | Init + per-map create/destroy + zone-event router |
| `src/server/scripts/Battlefield/BattlefieldWG.h` | 551 | `BattlefieldWG` + `WintergraspCapturePoint` + `BfWGGameObjectBuilding` + `WintergraspWorkshop` + all WG-specific enums (spells, NPCs, GO, achievements, quests, towers, workshops, graveyards, gossip text) |
| `src/server/scripts/Battlefield/BattlefieldWG.cpp` | 1857 | Wintergrasp setup + battle start/end + tower/wall destruction handlers + vehicle counting + tenacity calc + capture point progress + worldstate updates + reward + phasing |
| `src/server/scripts/Battlefield/battlefield_script_loader.cpp` | 23 | Script loader entry: `AddSC_BattlefieldWG()` |
| `src/server/scripts/Northrend/zone_wintergrasp.cpp` | n/a | Companion creature/GO scripts (defender/attacker NPCs, vehicle scripts) — NOT in scope here but referenced |

Approx total: **~3,790 lines** (Battlefield base + WG specialization).

---

## 3. Classes / Structs / Enums

### Base

| Symbol | Kind | Purpose |
|---|---|---|
| `Battlefield` | class (extends `ZoneScript`) | Base — owns timers, players sets, queue, raids, graveyards, generic data store |
| `BattlefieldControlZoneHandler` | class (extends `ControlZoneHandler`) | Hooks for capture-point progress / contested events |
| `BfGraveyard` | class | Per-team graveyard with spirit guides + control switching |
| `BattlefieldMgr` | singleton | `BattlefieldsMapByMap` + `BattlefieldMapByZone` indexes + `Update(diff)` |
| `BattlefieldState` | enum (int8) | `INACTIVE=0, WARMUP=1, IN_PROGRESS=2` |
| `BattlefieldTypes` | enum | `BATTLEFIELD_WG=1, BATTLEFIELD_TB=2` |
| `BattlefieldIDs` | enum | `BATTLEFIELD_BATTLEID_WG=1, _TB=21, _ASHRAN=24` (used in `SMSG_BATTLEFIELD_MGR_*` packets) |
| `BattlefieldObjectiveStates` | enum | 7 states: `NEUTRAL`, `ALLIANCE`, `HORDE`, `NEUTRAL_*_CHALLENGE`, `*_*_CHALLENGE` |
| `BattlefieldSounds` | enum | `BF_SOUND_HORDE_WINS=8454, _ALLIANCE_WINS=8455, _START=3439` |
| `BattlefieldTimers` | enum | `BATTLEFIELD_OBJECTIVE_UPDATE_INTERVAL=1000` ms |
| `PlayerTimerMap` | typedef | `map<ObjectGuid, time_t>` — invite expiry & kick timers |
| `GraveyardVect` | typedef | `vector<BfGraveyard*>` |

### Wintergrasp-specific

| Symbol | Kind | Purpose |
|---|---|---|
| `BattlefieldWG` | class (extends `Battlefield`) | Wintergrasp specialization |
| `WintergraspCapturePoint` | class (extends `BattlefieldControlZoneHandler`) | Per-workshop capture-point logic |
| `BfWGGameObjectBuilding` | struct | Gate / wall / tower object — state machine `INTACT → DAMAGE → DESTROY` (per-team variants) |
| `WintergraspWorkshop` | struct | A vehicle factory; controlled by team flag; spawns siege engines |
| `BfGraveyardWG` | class (extends `BfGraveyard`) | + GossipTextId per-graveyard |
| `StaticWintergraspTowerInfo` / `StaticWintergraspWorkshopInfo` / `WintergraspObjectPositionData` | struct | Compile-time spawn data tables in `BattlefieldWG.cpp` |
| `WintergraspSpells` | enum | Wartime auras (Recruit/Corporal/Lieutenant), Tenacity (`58549`/`59911`), Tower Control (`62064`), reward spells, phasing spells |
| `WintergraspData` | enum | indexes into `m_Data32`: `DAMAGED_TOWER_DEF/ATT`, `BROKEN_TOWER_DEF/ATT`, `MAX_VEHICLE_A/H`, `VEHICLE_A/H`, `MAX` |
| `WintergraspAchievements` | enum | 16 achievement IDs |
| `WintergraspQuests` | enum | victory + tower destroyed + defend siege quest credits |
| `WGGraveyardId` | enum | 7 graveyard slots |
| `WintergraspNpcs` | enum | guards, spirit guides, siege engines, demolishers, catapults, tower cannons |
| `WintergraspTowerIds` | enum | 7 towers (4 fortress, 3 attacker) |
| `WintergraspWorkshopIds` | enum | 6 workshops |
| `WintergraspGameObjectBuildingType` | enum | `DOOR, TITANRELIC, WALL, DOOR_LAST, KEEP_TOWER, TOWER` |
| `WintergraspGameObjectState` | enum | per-team intact/damage/destroy permutations |
| `WGGossipText` | enum | gossip text IDs for graveyard NPCs |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `BattlefieldMgr::InitBattlefield()` | Read `battlefield_template` table; instantiate per-template global config (currently only WG record is meaningful) | DB, `Battlefield::SetupBattlefield` |
| `BattlefieldMgr::CreateBattlefieldsForMap(map)` | Instantiate per-map Battlefield objects when map is created | factory dispatch on `m_TypeId` |
| `BattlefieldMgr::DestroyBattlefieldsForMap(map)` | Cleanup on map destroy | unique_ptr drop |
| `BattlefieldMgr::HandlePlayerEnterZone(player, zoneId)` | Route to `Battlefield::HandlePlayerEnterZone` | `_battlefieldsByZone` lookup |
| `BattlefieldMgr::HandlePlayerLeaveZone(player, zoneId)` | Route to `Battlefield::HandlePlayerLeaveZone` | same |
| `BattlefieldMgr::IsWorldPvpArea(zoneId)` | Returns true if zone has registered Battlefield | hash lookup |
| `BattlefieldMgr::GetBattlefieldToZoneId / ByBattleId(map, id)` | Resolve | hash lookup |
| `BattlefieldMgr::Update(diff)` | Throttled tick (every `_updateTimer` ms) — calls each Battlefield's `Update` | iterate _battlefieldsByMap |
| `Battlefield::Update(diff)` | Master tick: decrements `m_Timer`, fires phase transitions (INACTIVE→WARMUP→IN_PROGRESS), AFK kick, queue invite reminders | `StartBattle`, `EndBattle`, `KickAfkPlayers`, `InvitePlayersInZoneToQueue` |
| `Battlefield::SetupBattlefield()` | Subclass override: load static spawn data, construct graveyards, capture points, buildings | per-zone (WG: spawn 7 towers, 6 workshops, gates, GO buildings) |
| `Battlefield::StartBattle()` | Phase transition to IN_PROGRESS: invite queue→war, invite zone→war, set timer to `m_BattleTime`, clear/refresh world states | `OnBattleStart` virtual |
| `Battlefield::EndBattle(endByTimer)` | Resolve winner, award rewards, switch defender, set timer to `m_NoWarBattleTime`, untrack vehicles, save state | `OnBattleEnd` virtual |
| `Battlefield::HandlePlayerEnterZone(player, zone)` | Add to `m_players`, apply zone aura, send world-states, if `WARMUP` apply queue invite, if active push into war if accepted | `OnPlayerEnterZone` virtual |
| `Battlefield::HandlePlayerLeaveZone(player, zone)` | Remove from sets, drop war auras | `OnPlayerLeaveZone` virtual |
| `Battlefield::PlayerAcceptInviteToQueue(player)` | Move from `m_InvitedPlayers` → `m_PlayersInQueue` | n/a |
| `Battlefield::PlayerAcceptInviteToWar(player)` | Move into `m_PlayersInWar`, teleport if outside zone, apply tenacity, join raid via `AddOrSetPlayerToCorrectBfGroup` | `OnPlayerJoinWar` |
| `Battlefield::AskToLeaveQueue / PlayerAskToLeave(player)` | Voluntary withdraw | sets cleanup |
| `Battlefield::KickPlayerFromBattlefield(guid)` | Teleport to `KickPosition` | `Player::TeleportTo` |
| `Battlefield::GetFreeBfRaid(teamId)` | Find or create a Group (raid) for that team with a free slot | `Group::Create` |
| `Battlefield::AddOrSetPlayerToCorrectBfGroup(player)` | Place player into `m_Groups[team]` raid; create new if all full | `Group::AddMember` |
| `Battlefield::GetClosestGraveyard(player)` | Resurrection picker honoring control team | iterate `m_GraveyardList` |
| `Battlefield::SpawnCreature / SpawnGameObject` | Helper; auto-tagged with battlefield link | `Map::Add*` |
| `Battlefield::TeamApplyBuff(team, spell, spell2)` | Bulk team aura | iterate players |
| `Battlefield::SendWarning(id, target)` | Server-wide chat warning (e.g. "Wintergrasp battle begins!") | `WorldPackets::Misc::DefenseMessage` |
| `Battlefield::BroadcastPacketToZone/Queue/War` | Targeted broadcast helpers | iterate sets |
| `Battlefield::TeamCastSpell(team, spellId)` | Negative ID = remove aura | iterate |
| `Battlefield::HasPlayer(player)` | Membership check | union of war + zone |
| `BattlefieldWG::SetupBattlefield()` | WG-specific: spawn 7 towers, 6 workshops, 7 graveyards, gates, relic, set defender from DB | base impl + custom |
| `BattlefieldWG::OnBattleStart()` | Spawn relic + turrets, rebuild towers/walls, refresh phasing auras, invite to war, set tenacity initial | `UpdateVehicleCountWG`, `UpdateTenacity` |
| `BattlefieldWG::OnBattleEnd(endByTimer)` | Despawn vehicles, save defender to `world` DB, reward `SPELL_VICTORY_REWARD=56902` (or 58494 defeat), apply Essence of Wintergrasp aura | persistence + reward |
| `BattlefieldWG::OnStartGrouping()` | 15-min pre-battle: invite all players in zone | `InvitePlayersInZoneToQueue` |
| `BattlefieldWG::OnPlayerJoinWar(player)` | Apply rank aura (Recruit/Corporal/Lieutenant per kills), tenacity, port if needed | `PromotePlayer` |
| `BattlefieldWG::OnPlayerLeaveWar / Zone / Enter` | Aura management + worldstate refresh | base impl |
| `BattlefieldWG::OnCreatureCreate / Remove` | Track vehicles in `m_vehicles[team]`, update `BATTLEFIELD_WG_DATA_VEHICLE_*` | `FindAndRemoveVehicleFromList` |
| `BattlefieldWG::OnGameObjectCreate(go)` | Bind GO to building/workshop/relic/door slot | per-entry switch |
| `BattlefieldWG::BrokenWallOrTower(team, building)` | Update achievement criteria, quest credits, world state, possibly trigger relic-interactable | `UpdatedDestroyedTowerCount` |
| `BattlefieldWG::UpdateDamagedTowerCount / UpdatedDestroyedTowerCount(team)` | Track tower damage for reward calc; if 3 attacker towers down, subtract 10 minutes from battle timer | `m_Data32` |
| `BattlefieldWG::HandleKill(killer, victim)` | Apply HK score, possibly promote rank | `HandlePromotion`, `PromotePlayer` |
| `BattlefieldWG::OnUnitDeath(unit)` | Vehicle death cleanup | unhook |
| `BattlefieldWG::PromotePlayer(player)` | Rank-up aura swap (Recruit→Corporal→Lieutenant) | aura cast |
| `BattlefieldWG::UpdateTenacity()` | Compute team imbalance; apply `SPELL_TENACITY=58549` stacks to underdog (and `SPELL_TENACITY_VEHICLE=59911` to their vehicles) | iterate |
| `BattlefieldWG::UpdateVehicleCountWG / UpdateCounterVehicle(init)` | Refresh `MAX_VEHICLE_A/H` based on workshops controlled | per-workshop iteration |
| `BattlefieldWG::FindAndRemoveVehicleFromList(vehicle)` | Linear search in `m_vehicles[team]` | drop guid |
| `BattlefieldWG::DoCompleteOrIncrementAchievement(id, player, n)` | Achievement integration | `AchievementMgr::CompletedCriteria` |
| `BattlefieldWG::GetSpiritGraveyardId(areaId)` | Sub-zone → graveyard index | static table |
| `WintergraspCapturePoint::HandleContestedEventHorde / Alliance` | Workshop being captured by other team | UpdateWorldState, eventually `HandleProgressEvent*` |
| `WintergraspCapturePoint::HandleProgressEventHorde / Alliance` | Workshop change of ownership (final) | spawn vehicles, update tenacity, increment counter |

---

## 5. Module dependencies

**Depends on:**
- `Maps` — `Map* m_Map`; Battlefield is bound to a Northrend map (571). `OnPlayerEnter/Leave` is plumbed via `Map::AddPlayerToMap` hooks.
- `Groups` — `Battlefield::GetFreeBfRaid` and `Group::AddMember`. WG raids are NOT persisted (cleared on `EndBattle`).
- `Player` — `TeleportTo`, `CastSpell`, `RemoveAura`, `SendDirectMessage`.
- `WorldStateMgr` — heavy use of `UpdateWorldState` for client UI (timer, vehicle counts, tower states, workshop ownership).
- `Database (CharacterDatabase + WorldDatabase)` — `battlefield_zone` (defender persistence) + `battlefield_template`.
- `DataStores` — `WorldSafeLocsEntry` (graveyards), `BroadcastTextEntry` (warnings), `MapEntry`.
- `Phasing` — Wintergrasp factory phase shifts (`SPELL_HORDE_CONTROLS_FACTORY_PHASE_SHIFT = 56618`, etc.) — re-evaluated on workshop capture.
- `Vehicles` — siege engines, demolishers, catapults, tower cannons all use `Vehicle` aura mechanics.
- `Achievement / Criteria`.
- `Quests` — `QUEST_VICTORY_WINTERGRASP_A=13181`, `_H=13183`, `_CREDIT_TOWERS_DESTROYED=35074`, `_CREDIT_DEFEND_SIEGE=31284`.
- `OutdoorPvP` (sibling system) — separate; do NOT conflate. Battlefield is for *scheduled* events; `OutdoorPvP` is for continuously-on Hellfire/Halaa-style.
- `ZoneScript` parent — `OnCreatureCreate`, `OnGameObjectCreate`, `ProcessEvent`, `GetData`/`SetData`, `GetData64`/`SetData64`.
- `EventProcessor` (indirectly — for invite timers).
- `ControlZoneHandler` — generic capture-point primitive (lives in `wow-areatrigger` or `wow-gameobject` depending on layout).

**Depended on by:**
- `WorldSession` (no dedicated opcode handlers, but `CMSG_BF_MGR_QUEUE_*` and `CMSG_BF_MGR_ENTRY_*` plumb through).
- `Player::Update` polls warmup invite expiry.
- Northrend zone scripts (`zone_wintergrasp.cpp`).
- GM commands `.bf` (start/stop/timer).

---

## 6. SQL / DB queries (if any)

### WorldDatabase tables

| Table | Schema highlights | Purpose |
|---|---|---|
| `battlefield_template` | `TypeId`, `ZoneId`, `MaxPlayer`, `MinPlayer`, `MinLevel`, `BattleTime` (sec), `NoWarBattleTime` (sec), `RestartAfterCrash` (sec), `TimeForAcceptInvite` (sec), `KickPosition` (`MapId, X, Y, Z, O`) | Static config for each Battlefield (currently only one row, for WG) |
| `battlefield_zone` (referenced) | per-zone pivot back to template | indirection layer |

### CharacterDatabase

| Table | Schema highlights | Purpose |
|---|---|---|
| (none specific) | — | Wintergrasp persists *defender team* via `WorldStates` table (key = a known WS ID for "wg current defender") OR via custom `battlefield_wintergrasp` row in some forks. In the legacy fork, defender persistence is via `CharacterDatabase`'s world state save (WorldStateMgr). |

### Prepared statements

The Battlefield module itself does not register many prepared statements — most state is in-memory. However the *defender team* and the *next start time* survive crashes via:

| Statement | Purpose |
|---|---|
| `WORLD_DEL_GAMEOBJECT_RESPAWN` (indirectly) | Battlefield clears stale GO respawns at start |
| World-state save (via `sWorldStateMgr->SaveAll`) | Persists defender team across restarts |

### DBC/DB2 stores read

| Store | What it loads | Read by |
|---|---|---|
| `MapStorage` | Map.db2 | binding to map 571 |
| `AreaTableStorage` | AreaTable.dbc | zone 4197 + sub-areas for `GetSpiritGraveyardId` |
| `WorldSafeLocsStorage` | WorldSafeLocs.dbc | graveyard positions |
| `BroadcastTextStorage` | BroadcastText.dbc | `SendWarning(id)` |
| `SpellStorage` | Spell.dbc | tenacity / promotion / phase auras |

---

## 7. Wire-protocol packets (if any)

Wintergrasp uses the `BATTLEFIELD_MGR_*` opcode family (which is the same family used for queue UI, NOT the `BATTLEFIELD_STATUS_*` family used by instanced BGs):

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_BATTLEFIELD_MGR_ENTRY_INVITE_RESPONSE` | client → server | Player accepts/declines warmup invite |
| `CMSG_BATTLEFIELD_MGR_QUEUE_INVITE_RESPONSE` | client → server | Player accepts/declines war invite |
| `CMSG_BATTLEFIELD_MGR_QUEUE_REQUEST` | client → server | Manual queue join from outside zone |
| `CMSG_BATTLEFIELD_MGR_EXIT_REQUEST` | client → server | Voluntary leave during battle |
| `SMSG_BATTLEFIELD_MGR_ENTRY_INVITE` | server → client | "Wintergrasp will begin in X — accept invite?" |
| `SMSG_BATTLEFIELD_MGR_ENTERED` | server → client | Confirmation player is now in WG zone-tracked |
| `SMSG_BATTLEFIELD_MGR_QUEUE_INVITE` | server → client | War starts in 60s — accept to teleport in |
| `SMSG_BATTLEFIELD_MGR_QUEUE_REQUEST_RESPONSE` | server → client | Result of queue request |
| `SMSG_BATTLEFIELD_MGR_EJECT_PENDING` | server → client | "You will be teleported out in X seconds" |
| `SMSG_BATTLEFIELD_MGR_EJECTED` | server → client | Battle ended; player ejected |
| `SMSG_BATTLEFIELD_MGR_STATE_CHANGED` | server → client | Phase change (INACTIVE/WARMUP/IN_PROGRESS) update |
| `SMSG_DEFENSE_MESSAGE` (used by `Battlefield::SendWarning`) | server → client | Zone-wide chat warning ("Wintergrasp will begin in 30 minutes!") |
| `SMSG_INIT_WORLD_STATES` (already handled by WorldState system) | server → client | Initial WS dump on zone enter — Battlefield contributes ~30 entries |
| `SMSG_UPDATE_WORLD_STATE` | server → client | Per-second updates: timer, vehicle counts, tower/wall HP states, workshop ownership |
| `SMSG_PLAY_SOUND` | server → client | `BF_SOUND_*` win/start sounds |

Note: there is **no** dedicated opcode handler file for Battlefield in the C++ tree — the `WorldSession::HandleBattlefieldMgr*` handlers live in `Server/Handlers/MiscHandler.cpp` or `BattlefieldHandler.cpp` depending on tree version. In this WoLK 3.4.3 fork they are in `WorldSession.cpp` family.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-battlefield` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `crates/wow-areatrigger` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-gameobject` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *(none)* — no `crates/wow-battlefield/` exists.
- No handlers for any `CMSG_BATTLEFIELD_MGR_*` opcode in `crates/wow-world/src/handlers/`.
- No control-zone primitive in `crates/wow-areatrigger/` or `crates/wow-gameobject/` that could host `BattlefieldControlZoneHandler`.
- No `WorldStateMgr` in repo (referenced by Maps doc — also missing).

**What's implemented:**
- Nothing.

**What's missing vs C++:**
- Everything (Section 9).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The Battlefield system depends on systems that themselves are missing or partial in Rust (WorldStateMgr, ControlZoneHandler, Vehicle aura, Phasing, AchievementMgr). Plan WG implementation as the LAST PvP module — not the first.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#BATTLEFIELD.WBS.001** Partir y cerrar la migracion auditada de `game/Battlefield/Battlefield.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`
  Rust target: `crates/wow-battlefield`, `crates/wow-world/src/handlers`, `crates/wow-areatrigger`, `crates/wow-gameobject`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 738 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEFIELD.WBS.002** Cerrar la migracion auditada de `game/Battlefield/Battlefield.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`
  Rust target: `crates/wow-battlefield`, `crates/wow-world/src/handlers`, `crates/wow-areatrigger`, `crates/wow-gameobject`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEFIELD.WBS.003** Cerrar la migracion auditada de `game/Battlefield/BattlefieldMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp`
  Rust target: `crates/wow-battlefield`, `crates/wow-world/src/handlers`, `crates/wow-areatrigger`, `crates/wow-gameobject`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BATTLEFIELD.WBS.004** Cerrar la migracion auditada de `game/Battlefield/BattlefieldMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.h`
  Rust target: `crates/wow-battlefield`, `crates/wow-world/src/handlers`, `crates/wow-areatrigger`, `crates/wow-gameobject`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#BF.1** Create `crates/wow-battlefield/` crate (Cargo.toml deps: `wow-core`, `wow-constants`, `wow-database`, `wow-data-stores`, `wow-maps`, `wow-groups`); add to workspace (L)
- [ ] **#BF.2** Port enums (`BattlefieldState`, `BattlefieldTypes`, `BattlefieldIDs`, `BattlefieldObjectiveStates`, `BattlefieldSounds`, `BattlefieldTimers`) into `wow-constants` (L)
- [ ] **#BF.3** Port `BfGraveyard` w/ `GiveControlTo`, `Initialize`, `SetSpirit`, `HasNpc`, `GetDistance` (M)
- [ ] **#BF.4** Port `BattlefieldControlZoneHandler` trait (extends generic `ControlZoneHandler`) (M)
- [ ] **#BF.5** Implement `Battlefield` base struct + trait — fields: `m_Timer`, `m_isActive`, `m_DefenderTeam`, `m_TypeId`, `m_BattleId`, `m_ZoneId`, `m_MapId`, `m_BattleTime`, `m_NoWarBattleTime`, `m_StartGroupingTimer`, all player sets, raid sets, graveyards, `m_Data32`, `m_Data64` (H)
- [ ] **#BF.6** Implement `Battlefield::Update(diff)` — phase machine + timer decrement + AFK kick + grouping invite (H)
- [ ] **#BF.7** Implement `StartBattle`, `EndBattle(endByTimer)` w/ virtual hooks (M)
- [ ] **#BF.8** Implement `HandlePlayerEnterZone`, `HandlePlayerLeaveZone` w/ player set bookkeeping + virtual hooks (M)
- [ ] **#BF.9** Implement queue/war invite path: `InvitePlayerToQueue`, `InvitePlayerToWar`, `InvitePlayersInZoneToQueue`, `InvitePlayersInQueueToWar`, `InvitePlayersInZoneToWar` (H)
- [ ] **#BF.10** Implement `PlayerAcceptInviteToQueue / ToWar`, `AskToLeaveQueue`, `PlayerAskToLeave`, `KickPlayerFromBattlefield` (M)
- [ ] **#BF.11** Implement raid auto-formation: `GetFreeBfRaid`, `AddOrSetPlayerToCorrectBfGroup`, `GetGroupPlayer` (M)
- [ ] **#BF.12** Implement graveyard utilities: `GetClosestGraveyard`, `GetGraveyardById`, `SetGraveyardNumber` (M)
- [ ] **#BF.13** Implement broadcast helpers: `BroadcastPacketToZone/Queue/War`, `SendWarning`, `DoPlaySoundToAll`, `TeamApplyBuff`, `TeamCastSpell` (M)
- [ ] **#BF.14** Implement `KickAfkPlayers` (`m_PlayersWillBeKick` timer map) (L)
- [ ] **#BF.15** Implement `HasPlayer`, `GetData/SetData/GetData64/SetData64`, `RegisterZone` (L)
- [ ] **#BF.16** Implement `BattlefieldMgr` singleton + `_battlefieldsByMap` + `_battlefieldsByZone` indexes + `Update(diff)` + `IsWorldPvpArea` + `GetBattlefieldToZoneId / ByBattleId` (M)
- [ ] **#BF.17** Implement `BattlefieldMgr::InitBattlefield` — load `battlefield_template`, register zones (M)
- [ ] **#BF.18** Implement `CreateBattlefieldsForMap / DestroyBattlefieldsForMap` factory dispatch (M)
- [ ] **#BF.19** Port WG enums (spells, NPCs, GO, achievements, quests, towers, workshops, graveyards, buildings, gossip text) — large but straightforward (M)
- [ ] **#BF.20** Implement `BattlefieldWG` struct + `WintergraspWorkshop` + `BfWGGameObjectBuilding` static spawn data (M)
- [ ] **#BF.21** Implement `BattlefieldWG::SetupBattlefield` — spawn 7 towers, 6 workshops, 7 graveyards, gates, relic, set defender from persisted state (H)
- [ ] **#BF.22** Implement `OnBattleStart` — relic spawn, turret spawn, tower/wall rebuild, phasing aura refresh, war invites, tenacity init (H)
- [ ] **#BF.23** Implement `OnBattleEnd(endByTimer)` — vehicle despawn, defender swap save, reward spell distribution (`SPELL_VICTORY_REWARD=56902` / `SPELL_DEFEAT_REWARD=58494`), Essence of Wintergrasp aura, quest credits (H)
- [ ] **#BF.24** Implement `OnStartGrouping` — 15-min pre-battle invite-zone hook (L)
- [ ] **#BF.25** Implement `OnPlayerJoinWar / LeaveWar / EnterZone / LeaveZone` w/ aura management (M)
- [ ] **#BF.26** Implement `OnCreatureCreate / Remove` — vehicle counting (`m_vehicles[team]`, `BATTLEFIELD_WG_DATA_VEHICLE_*`) (M)
- [ ] **#BF.27** Implement `OnGameObjectCreate(go)` — bind GO to building / workshop / relic / door slot via entry switch (M)
- [ ] **#BF.28** Implement `BrokenWallOrTower`, `UpdateDamagedTowerCount`, `UpdatedDestroyedTowerCount` w/ 3-towers-down → −10min battle timer rule (M)
- [ ] **#BF.29** Implement `HandleKill / OnUnitDeath / HandlePromotion / PromotePlayer` w/ rank aura cycle (Recruit→Corporal→Lieutenant) (M)
- [ ] **#BF.30** Implement `UpdateTenacity` — team-imbalance counter w/ `SPELL_TENACITY=58549` + vehicle variant `SPELL_TENACITY_VEHICLE=59911` (M)
- [ ] **#BF.31** Implement `UpdateVehicleCountWG` / `UpdateCounterVehicle` based on workshop ownership (M)
- [ ] **#BF.32** Implement `WintergraspCapturePoint::HandleContestedEvent*` and `HandleProgressEvent*` for workshop control swaps + factory phasing spell cast (M)
- [ ] **#BF.33** Implement `DoCompleteOrIncrementAchievement` integration (depends on `achievements.md`) (M)
- [ ] **#BF.34** Implement world-state push for all WG UI (timer, towers, walls, workshops, vehicle counts) — 30+ keys (M)
- [ ] **#BF.35** Implement opcode handlers in `wow-world/src/handlers/`: `CMSG_BATTLEFIELD_MGR_ENTRY_INVITE_RESPONSE`, `CMSG_BATTLEFIELD_MGR_QUEUE_INVITE_RESPONSE`, `CMSG_BATTLEFIELD_MGR_QUEUE_REQUEST`, `CMSG_BATTLEFIELD_MGR_EXIT_REQUEST` (M)
- [ ] **#BF.36** Implement packet builders for `SMSG_BATTLEFIELD_MGR_ENTRY_INVITE / QUEUE_INVITE / ENTERED / EJECT_PENDING / EJECTED / STATE_CHANGED / QUEUE_REQUEST_RESPONSE` (M)
- [ ] **#BF.37** Persist defender team across restart (`WorldStateMgr` integration or dedicated `battlefield_wintergrasp` row) (M)
- [ ] **#BF.38** Persist next-start timer across restart (so a crash mid-warmup resumes correctly within `RestartAfterCrash` window) (M)
- [ ] **#BF.39** Wire `BattlefieldMgr::Update` into the world tick loop (depends on `world.md`) (L)
- [ ] **#BF.40** GM commands: `.bf` (status, start, stop, switch, timer) (M)
- [ ] **#BF.41** Audit/refactor: ensure Battlefield does NOT collide with the Battleground status-packet family — they are different opcode subspaces (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#BATTLEFIELD.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 1360 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#BATTLEFIELD.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 1360 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#BATTLEFIELD.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 1360 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#BATTLEFIELD.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 1360 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Battlefield::Update` correctly transitions `INACTIVE → WARMUP` at exactly `m_Timer == m_StartGroupingTimer`.
- [ ] Test: `WARMUP → IN_PROGRESS` transition fires `StartBattle` and resets `m_Timer = m_BattleTime`.
- [ ] Test: `IN_PROGRESS → INACTIVE` on `m_Timer == 0` fires `EndBattle(endByTimer=true)` with current defender as winner.
- [ ] Test: WG attacker clicking relic with `m_isRelicInteractible == true` triggers `EndBattle(endByTimer=false)` with attackers as winner.
- [ ] Test: 3 attacker towers destroyed → battle timer drops by 600s (10 min).
- [ ] Test: `BattlefieldWG::OnBattleEnd` swaps defender team and persists it (defender role flip).
- [ ] Test: Tenacity aura is applied to underdog team only when player count delta ≥ threshold.
- [ ] Test: Workshop capture by attackers spawns siege engines (`NPC_WINTERGRASP_SIEGE_ENGINE_HORDE` or `_ALLIANCE`).
- [ ] Test: Workshop capture phase-shift aura swap (`SPELL_HORDE_CONTROLS_FACTORY_PHASE_SHIFT=56618` or `_ALLIANCE_=56617`) re-evaluated on workshop change.
- [ ] Test: Player rank promotion: 1 HK → Recruit (`37795`), 5 HKs → Corporal (`33280`), 25 HKs → Lieutenant (`55629`) auras cycled correctly.
- [ ] Test: AFK kick at `m_uiKickAfkPlayersTimer == 0` for a player flagged AFK in the war set.
- [ ] Test: Player declining war invite is teleported to `KickPosition` + receives no rewards.
- [ ] Test: Auto-raid: 41st joining player on a side gets a NEW raid Group (not added to a full one).
- [ ] Test: `GetClosestGraveyard` honors `m_ControlTeam` (cannot res at enemy-controlled GY).
- [ ] Test: After `EndBattle`, all `m_vehicles[*]` are despawned and `m_PlayersInWar` is cleared.
- [ ] Test: `RestartAfterCrash` window — server crash 5 minutes into battle resumes battle if restarted within `RestartAfterCrash` seconds; otherwise treats as ended.
- [ ] Test: Reward spells `SPELL_VICTORY_REWARD=56902` cast on every winner; `SPELL_DEFEAT_REWARD=58494` on losers.
- [ ] Test: Quest credit `QUEST_CREDIT_TOWERS_DESTROYED=35074` granted to all attackers in zone when an attacker tower is destroyed.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 1360 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp` | `crates/wow-battlefield/` (NOT YET CREATED — must be added to workspace) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#BATTLEFIELD.DIV.001` | `crates/wow-battlefield` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1360 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#BATTLEFIELD.DIV.002` | `crates/wow-areatrigger` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1360 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#BATTLEFIELD.DIV.003` | `crates/wow-gameobject` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1360 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/Battlefield.h`, `/home/server/woltk-trinity-legacy/src/server/game/Battlefield/BattlefieldMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Battlefield is NOT instanced.** Unlike `Battleground`, players don't change maps — they stay on Northrend (map 571). All state lives in the open world. This means: GO respawn rows in `gameobject_respawn` need `instanceId=0` filtering (or you'll wipe BG GO rows when resetting WG state).
- **Defender persists across restarts.** This is the single most important persistence requirement. If you skip it, the defender team alternates incorrectly after every server boot.
- **`m_NoWarBattleTime` is typically 12600s (3.5 hours)** in retail; configurable per-server. `m_BattleTime` is typically 1800s (30 min). The 15-min grouping window is hardcoded as `m_StartGroupingTimer` ≈ 900s.
- **WG zone is 4197**, sub-areas have their own area IDs that resolve to specific graveyards via `GetSpiritGraveyardId(areaId)`. Don't shortcut the lookup.
- **Phasing on factory control** is implemented as 4 distinct auras (`56617`, `56618`, `55773`, `55774`) cast on every player in zone. They add specific phase masks (16, 32, 64, 128). When workshops change hands, ALL players in zone need their auras refreshed — single forgetful pass leaks ghost vehicles into wrong phase.
- **Tenacity has a vehicle variant.** Both `SPELL_TENACITY=58549` (player) and `SPELL_TENACITY_VEHICLE=59911` (mounted) must be applied/refreshed when a player mounts/dismounts a siege engine during war. Forgetting this gives unmounting players a perma-stack of tenacity.
- **`Tower Control` spell `62064`** is applied to *attackers* while *attacker* towers stand and reduces tower-cannon damage taken. When all attacker towers fall it must be removed — easy to miss since it's a buff for *attackers*, not the team that controls the tower.
- **Relic interactibility.** `m_isRelicInteractible` is FALSE until the last fortress door (`BATTLEFIELD_WG_OBJECTTYPE_DOOR_LAST`) is broken. Setting it true earlier = match insta-loss for defenders.
- **Vehicle cap.** `BATTLEFIELD_WG_DATA_MAX_VEHICLE_A/H` is recomputed from controlled workshops × per-workshop slot count. New vehicles refuse to spawn past the cap. Cap is 0 when team controls 0 workshops, meaning attackers without any workshops cannot spawn ANY vehicle.
- **`SPELL_ESSENCE_OF_WINTERGRASP=58045`** is applied zone-wide *to the winners* AFTER battle ends; it grants +33% mount speed and other utility for the next 4 hours. It must be removed at next battle start.
- **`SPELL_WINTERGRASP_RESTRICTED_FLIGHT_AREA=91604`** is what blocks flying mounts during battle. Must be cast on every player on entering zone during war.
- **`SPELL_VICTORY_AURA=60044`** is a brief glow on winners; cosmetic, skip if porting essentials only.
- **Auto-promote ranks** are *strictly* HK-based (kills count). Different from BG K/D ratio. Look at `HandlePromotion` precondition: kill count thresholds 1, 5, 25.
- **`KickPosition`** is read from `battlefield_template.KickPosition*`. WG default sends to Dalaran via `SPELL_TELEPORT_DALARAN=53360`. Don't hardcode coords.
- **Ashran (`BATTLEFIELD_BATTLEID_ASHRAN=24`)** is a WoD concept. Do not implement; `BattlefieldTB` (Tol Barad) is also Cataclysm. Stick to WG only for 3.4.3.
- **Battlefield uses `ControlZoneHandler`**, a generic system that some other modules also use (Outdoor PvP, scenarios). Do NOT duplicate the primitive in `wow-battlefield` — put it in `wow-areatrigger` or `wow-gameobject` and have all callers depend on it.
- **`GuidUnorderedSet`** in C++ is `std::unordered_set<ObjectGuid>`; in Rust use `FxHashSet<ObjectGuid>` for speed (small N, no DoS surface here).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Battlefield : public ZoneScript` | `pub trait Battlefield: ZoneScript` + `pub struct BattlefieldBase` | Trait for virtuals; struct for shared state |
| `class BattlefieldWG : public Battlefield` | `pub struct BattlefieldWg { base: BattlefieldBase, .. }` | Composition |
| `class BfGraveyard` | `pub struct BfGraveyard` w/ `pub trait BfGraveyardLike` for WG override | WG uses subtype `BfGraveyardWG` for gossip text |
| `class BattlefieldControlZoneHandler` | `pub trait BattlefieldControlZoneHandler: ControlZoneHandler` | Trait |
| `BattlefieldMgr` singleton | `pub struct BattlefieldMgr` + `OnceLock<BattlefieldMgr>` | `sBattlefieldMgr` → `wow_battlefield::mgr()` |
| `unordered_map<Map const*, vector<unique_ptr<Battlefield>>>` | `DashMap<MapId, Vec<Box<dyn Battlefield>>>` | Use MapId not raw map ptr — Rust borrow checker friendlier |
| `unordered_map<pair<Map const*, u32>, Battlefield*>` | `DashMap<(MapId, u32 /*zoneId*/), BattlefieldRef>` | `BattlefieldRef` = handle into Vec |
| `GuidUnorderedSet m_players[2]` | `[FxHashSet<ObjectGuid>; 2]` | Indexed by `TeamId` (0=Alliance, 1=Horde) |
| `PlayerTimerMap = map<ObjectGuid, time_t>` | `HashMap<ObjectGuid, SystemTime>` or `BTreeMap` if iteration-by-time matters | Pick chrono crate consistently |
| `vector<u64> m_Data64` / `vector<u32> m_Data32` | `Vec<u64>` / `Vec<u32>` | Index by enum variant cast to usize |
| `BfWGGameObjectBuilding` (state machine) | `pub struct BfWgBuilding { state: BuildingState, .. }` w/ enum `BuildingState` | wire-compatible discriminant |
| `Static*Info` tables | `static const TOWERS: [TowerInfo; 7] = [..]` | compile-time arrays |
| `WintergraspSpells` enum | `pub mod wg_spells { pub const RECRUIT: u32 = 37795; .. }` | constants module |
| `ProcessEvent(WorldObject*, u32, WorldObject*)` | `fn process_event(&mut self, target: &WorldObject, event_id: u32, invoker: Option<&WorldObject>)` | direct port |
| `WorldPacket SMSG_BATTLEFIELD_MGR_*` | `wow-shared-packets::battlefield_mgr::*` (NEW) | Strict structs |
| `Vehicle` aura mechanics | `wow-vehicles` crate (likely partial) | Coordinate w/ vehicles.md |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

❌ confirmado. Auditado contra `/home/server/rustycore/crates/`.

**Hallazgos clave:**
- No existe `crates/wow-battlefield/`. No existe `crates/wow-vehicles/` ni `crates/wow-worldstate/` ni `crates/wow-areatrigger/` con `ControlZoneHandler`. Todas las dependencies que el doc lista como precondición (WorldStateMgr, ControlZoneHandler, Vehicle aura, Phasing, AchievementMgr) están ausentes.
- `crates/wow-achievement/` existe como crate pero solo con scaffolding mínimo (no `AchievementMgr` real con `CompletedCriteria`).
- Búsqueda de identifiers `Wintergrasp`, `BattlefieldMgr`, `Bf*Mgr*`, `BfWGGameObjectBuilding`, `WintergraspWorkshop`, `m_DefenderTeam`, `OnBattleStart`, `OnBattleEnd` en todo el workspace: **0 resultados**.
- Búsqueda de los opcodes `BfMgr*` o `BattlefieldMgr*` (entry-invite, queue-invite, queue-request, exit-request, ejected, eject-pending, state-changed) en `wow-constants/src/opcodes.rs`: **0 nombres con esos prefijos**. Las constantes ni siquiera están aún en el catálogo de opcodes — habrá que añadirlas en el batch correspondiente. Solo aparecen los del subspace `Battlefield*` de BGs (que es otra familia, ver §11 del propio doc).
- `crates/wow-world/src/handlers/` no contiene ningún archivo `battlefield.rs` ni stub para los 4 CMSG (`ENTRY_INVITE_RESPONSE`, `QUEUE_INVITE_RESPONSE`, `QUEUE_REQUEST`, `EXIT_REQUEST`).

**Verificación de dependencias (el doc dice "should be last PvP migrated" — confirmado):**
- WorldStateMgr: ❌ ausente. Battlefield necesita ~30 worldstate IDs propios + persistencia del defender team.
- ControlZoneHandler: ❌ ausente. Wintergrasp usa el primitivo de captura genérico para los 6 workshops.
- Vehicle (auras 56933 driving, etc.): ❌ ausente. Sin esto, demolishers/siege engines/catapults son spawns inertes.
- Phasing (auras 56617, 56618, 55773, 55774 + máscaras 16/32/64/128): ❌ ausente. Sin esto, factory phase shift no funciona y los vehicles spawneados serían visibles para todos sin distinción de team.
- AchievementMgr: ⚠️ scaffold-only. Wintergrasp completa 16 logros distintos.

**Riesgo de UI hang silencioso:**
- 🟢 **No hay riesgo activo** porque no existe la zona Wintergrasp jugable. Los players nunca llegarán al área 4197 con contenido funcional, así que el cliente nunca enviará `CMSG_BATTLEFIELD_MGR_*` opcodes. Dropear estos en el dispatcher sin handler es no-op visible.
- ⚠️ Si se entra al área 4197 (zone teleport via GM) con el sistema dormido, el cliente queda en zone con HUD genérico — no hay UI element esperando confirmación, así que no hay hang. Riesgo cosmético only.

**Acción:** dejar `❌ not started` y mantenerlo al final del orden de migración PvP (después de #BG, #OPVP, y de WorldStateMgr+Vehicle+Phasing). Documentar en MIGRATION_ROADMAP que Battlefield es **gated by**: maps.md, instances.md, vehicles, phasing, worldstatemgr, achievements. Hasta entonces nadie debe abrir un PR `wow-battlefield`.
