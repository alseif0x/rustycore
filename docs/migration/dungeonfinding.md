# Migration: DungeonFinding (LFGMgr)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/DungeonFinding/` + `src/server/game/Handlers/LFGHandler.cpp`
> **Rust target crate(s):** `crates/wow-world/src/lfg/` (a crear) + handlers en `crates/wow-world/src/handlers/`
> **Layer:** L7
> **Status:** ❌ not started (stubs vacíos en `handlers/misc.rs`)
> **Audited vs C++:** ✅ audited 2026-05-01 (❌ confirmed — silent UI hang risk)
> **Last updated:** 2026-05-01

---

## 1. Purpose

LFG (*Looking For Group / Dungeon Finder*) automatiza la formación de grupos de 5 jugadores (1 tank + 1 healer + 3 dps) para dungeons aleatorios o específicos. Mantiene colas por queue-id, ejecuta role-checks pre-cola, computa compatibilidad entre tickets, genera proposals con `cancelTime=45s`, teleporta al dungeon, y aplica buffs (`Luck of the Draw 72221`) / debuffs (`Dungeon Deserter 71041` 30 min al abandonar prematuramente, `Dungeon Cooldown 71328` random-roll). El tick global del manager corre cada `LFG_QUEUEUPDATE_INTERVAL = 15s`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/DungeonFinding/LFG.cpp` | 106 | `prefix` |
| `game/DungeonFinding/LFG.h` | 144 | `prefix` |
| `game/DungeonFinding/LFGGroupData.cpp` | 140 | `prefix` |
| `game/DungeonFinding/LFGGroupData.h` | 87 | `prefix` |
| `game/DungeonFinding/LFGList.cpp` | 161 | `prefix` |
| `game/DungeonFinding/LFGList.h` | 68 | `prefix` |
| `game/DungeonFinding/LFGMgr.cpp` | 2241 | `prefix` |
| `game/DungeonFinding/LFGMgr.h` | 509 | `prefix` |
| `game/DungeonFinding/LFGPlayerData.cpp` | 126 | `prefix` |
| `game/DungeonFinding/LFGPlayerData.h` | 81 | `prefix` |
| `game/DungeonFinding/LFGQueue.cpp` | 739 | `prefix` |
| `game/DungeonFinding/LFGQueue.h` | 148 | `prefix` |
| `game/DungeonFinding/LFGScripts.cpp` | 254 | `prefix` |
| `game/DungeonFinding/LFGScripts.h` | 57 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/DungeonFinding/LFG.h` | 144 | Enums core: `LfgRoles`, `LfgState`, `LfgUpdateType`, `LfgLockStatusType`, `LfgAnswer`; typedefs |
| `src/server/game/DungeonFinding/LFG.cpp` | 106 | Helpers `ConcatenateDungeons`, `GetRolesString`, `GetStateString` |
| `src/server/game/DungeonFinding/LFGMgr.h` | 509 | `LFGMgr` singleton, `LfgJoinResult`, `LfgRoleCheck`, `LfgProposal`, `LfgPlayerBoot`, `LFGDungeonData` |
| `src/server/game/DungeonFinding/LFGMgr.cpp` | 2255 | Implementación core (load, join, role-check, proposal, teleport, finish, reward) |
| `src/server/game/DungeonFinding/LFGQueue.h` | 148 | `LFGQueue` per queue-id, `LfgCompatibility`, `LfgQueueData`, wait-time stores |
| `src/server/game/DungeonFinding/LFGQueue.cpp` | 745 | `FindGroups`, `CheckCompatibility`, wait-time avg, AddToQueue/RemoveFromQueue |
| `src/server/game/DungeonFinding/LFGGroupData.h/.cpp` | 87/140 | `LfgGroupData` (group-side state cache) |
| `src/server/game/DungeonFinding/LFGPlayerData.h/.cpp` | 81/126 | `LfgPlayerData` (player-side state cache) |
| `src/server/game/DungeonFinding/LFGScripts.h/.cpp` | 57/254 | `LFGPlayerScript` (OnLogin/OnLogout/OnMapChanged) + `LFGGroupScript` (OnAddMember/OnRemoveMember/OnDisband/OnChangeLeader) |
| `src/server/game/DungeonFinding/LFGList.h/.cpp` | 68/161 | Premade Group Finder (GroupFinder UI) — separado de LFG automático |
| `src/server/game/Handlers/LFGHandler.cpp` | ~950 | Handlers de opcodes CMSG/SMSG y builders de packets |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `LFGMgr` | singleton | Manager global. Posee `QueuesStore`, `LfgDungeonStore`, `RoleChecksStore`, `ProposalsStore`, `BootsStore`, `PlayersStore`, `GroupsStore`, `RewardMapStore` |
| `LFGQueue` | class | Cola por queue-id. `QueueDataStore`, `CompatibleMapStore`, wait-time stores, `currentQueueStore`, `newToQueueStore` |
| `LfgQueueData` | struct | `joinTime`, `tanks`, `healers`, `dps`, `dungeons`, `roles`, `bestCompatible` |
| `LfgRoleCheck` | struct | `cancelTime`, `roles`, `state`, `dungeons`, `rDungeonId`, `leader` (group-level pre-queue) |
| `LfgProposal` | struct | `id`, `dungeonId`, `state`, `group`, `leader`, `cancelTime`, `encounters`, `isNew`, `queues`, `showorder`, `players` (5 jugadores con role + answer) |
| `LfgProposalPlayer` | struct | `role`, `accept` (PENDING/DENY/AGREE), `group` (original guid) |
| `LfgPlayerBoot` | struct | `cancelTime`, `inProgress`, `votes`, `victim`, `reason` (vote-kick) |
| `LfgPlayerData` | class | Per-player cache: `m_State`, `m_OldState`, `m_Roles`, `m_SelectedDungeons`, `m_Team`, `m_Group`, `m_Comment`, `m_Ticket` |
| `LfgGroupData` | class | Per-group cache: `m_State`, `m_OldState`, `m_Dungeon`, `m_Leader`, `m_Players`, `m_KicksLeft`, `m_VoteKickActive` |
| `LfgReward` | struct | `maxLevel`, `firstQuest`, `otherQuest` (random-dungeon rewards) |
| `LfgPlayerRewardData` | struct | `rdungeonEntry`, `sdungeonEntry`, `done`, `quest` (sent en SMSG_LFG_PLAYER_REWARD) |
| `LfgJoinResultData` | struct | `result`, `state`, `lockmap`, `playersMissingRequirement` |
| `LfgUpdateData` | struct | `updateType`, `state`, `dungeons`, `comment` (per SMSG_LFG_UPDATE_STATUS) |
| `LfgQueueStatusData` | struct | `queueId`, `dungeonId`, `waitTime*`, `tanks/healers/dps` |
| `LfgLockInfoData` | struct | `lockStatus`, `requiredItemLevel`, `currentItemLevel` |
| `LFGDungeonData` | struct | `id`, `name`, `map`, `type`, `expansion`, `group`, `minlevel`, `maxlevel`, `difficulty`, `seasonal`, `x/y/z/o`, `requiredItemLevel`, `finalDungeonEncounterId`. `Entry()` = `id | (type << 24)` |
| `LfgRoles` | enum bitmask | `LEADER=0x01`, `TANK=0x02`, `HEALER=0x04`, `DAMAGE=0x08`, `ANY=0x0F` |
| `LfgState` | enum u8 | `NONE=0, ROLECHECK=1, QUEUED=2, PROPOSAL=3, DUNGEON=5, FINISHED_DUNGEON=6, RAIDBROWSER=7` |
| `LfgUpdateType` | enum | 22 valores (JOIN_QUEUE=6, ROLECHECK_FAILED=7, PROPOSAL_BEGIN=15, DUNGEON_FINISHED=26, etc.) |
| `LfgJoinResult` | enum | 21 valores de error (GROUP_FULL=0x1F, NO_SLOTS=0x22, DESERTER_PLAYER=0x28, etc.) |
| `LfgRoleCheckState` | enum | DEFAULT/FINISHED/INITIALITING/MISSING_ROLE/WRONG_ROLES/ABORTED/NO_ROLE (0..6) |
| `LfgTeleportResult` | enum u8 | NONE/DEAD/FALLING/ON_TRANSPORT/EXHAUSTION/NO_RETURN_LOCATION/IMMUNE_TO_SUMMONS |
| `LfgProposalState` | enum | INITIATING/FAILED/SUCCESS |
| `LfgLockStatusType` | enum | INSUFFICIENT_EXPANSION=1, TOO_LOW_LEVEL=2, TOO_HIGH_LEVEL=3, TOO_LOW_GS=4, TOO_HIGH_GS=5, RAID_LOCKED=6, NO_SPEC=14, etc. |
| `LfgAnswer` | enum i8 | PENDING=-1, DENY=0, AGREE=1 |
| `LfgQueueType` | enum | DUNGEON=1, LFR=2, SCENARIO=3, FLEX=4, WORLD_PVP=5, SCHEDULED_PVP=6 |
| `LfgCompatibility` | enum | PENDING / WRONG_GROUP_SIZE / TOO_MUCH_PLAYERS / MULTIPLE_LFG_GROUPS / HAS_IGNORES / NO_ROLES / NO_DUNGEONS / WITH_LESS_PLAYERS / BAD_STATES / MATCH |
| `LfgMgrEnum` | enum | `TIME_ROLECHECK=45s`, `TIME_BOOT=120s`, `TIME_PROPOSAL=45s`, `QUEUEUPDATE_INTERVAL=15000ms`, `SPELL_DUNGEON_COOLDOWN=71328`, `SPELL_DUNGEON_DESERTER=71041`, `SPELL_LUCK_OF_THE_DRAW=72221`, `GROUP_KICK_VOTES_NEEDED=3` |
| `LFGPlayerScript`, `LFGGroupScript` | class | Hooks lifecycle |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `LFGMgr::Update(uint32 diff)` | Tick: avanza role-checks, expira proposals, llama `LFGQueue::FindGroups` cada 15s | `LFGQueue`, `RemoveProposal`, `SendLfgUpdateStatus` |
| `LFGMgr::JoinLfg(Player*, roles, dungeons)` | Validación pre-join, role-check si party, encolar | `selectedRandomLfgDungeon`, `GetCompatibleDungeons`, `LFGQueue::AddToQueue`, `SendLfgJoinResult`, `SendLfgUpdateStatus` |
| `LFGMgr::LeaveLfg(guid, disconnected)` | Sale de cola/proposal/role-check | `LFGQueue::RemoveFromQueue`, `RemoveProposal` |
| `LFGMgr::UpdateRoleCheck(gguid, guid, roles)` | Acumula roles del party; cuando completos → `CheckGroupRoles` y enqueue | `CheckGroupRoles`, `SendLfgRoleCheckUpdate`, `LFGQueue::AddToQueue` |
| `LFGMgr::UpdateProposal(proposalId, guid, accept)` | Acumula respuestas; si todos AGREE → `MakeNewGroup`; si alguien DENY/timeout → `RemoveProposal(FAILED)` | `MakeNewGroup`, `RemoveProposal`, `SendLfgUpdateProposal` |
| `LFGMgr::MakeNewGroup(proposal)` | Forma `Group` LFG, asigna instance binding, inicia teleport | `Group::Create`, `TeleportPlayer`, `SetState(DUNGEON)` |
| `LFGMgr::TeleportPlayer(player, out, fromOpcode)` | Tele in/out a coords del dungeon (`x,y,z,o` de `lfg_dungeon_template`) | `Player::TeleportTo`, `SendLfgTeleportError` |
| `LFGMgr::FinishDungeon(gguid, dungeonId, currMap)` | Marca DUNGEON_FINISHED, otorga `LfgReward` (firstQuest/otherQuest), aplica `SPELL_DUNGEON_COOLDOWN` si fue random | `Player::RewardQuest`, `SetState(FINISHED_DUNGEON)`, `SendLfgPlayerReward` |
| `LFGMgr::OnDungeonEncounterDone(gguid, encounterIds, currMap)` | Track encounters; si match `finalDungeonEncounterId` → `FinishDungeon` | `FinishDungeon` |
| `LFGMgr::InitBoot(gguid, kguid, vguid, reason)` | Inicia vote-kick (3 votes needed, 120s timer) | `SendLfgBootProposalUpdate` |
| `LFGMgr::UpdateBoot(guid, accept)` | Acumula votos; si 3 AGREE → kick via `Group::RemoveMember`; si 3 DENY o timeout → cancel | `Group::RemoveMember`, `DecreaseKicksLeft` |
| `LFGMgr::SetRoles/GetRoles(guid)` | Roles bitmask por player | — |
| `LFGMgr::SetState/GetState(guid)` | Player o group state, dispatch via `IsLfgGroup` | `LfgPlayerData::SetState`, `LfgGroupData::SetState` |
| `LFGMgr::GetSelectedDungeons(guid)` | LfgDungeonSet seleccionado | — |
| `LFGMgr::GetLockedDungeons(guid)` | Dungeons bloqueados con lock-status (level/GS/quest/achievement) | `IsValidDungeonForPlayer` (helper) |
| `LFGMgr::GetRandomAndSeasonalDungeons(level, expansion)` | Lista de random + seasonal disponibles | `LfgDungeonStore` |
| `LFGMgr::GetRandomDungeonReward(dungeon, level)` | Devuelve `LfgReward*` para nivel del player | `RewardMapStore` |
| `LFGMgr::CheckGroupRoles(roles)` | Static: verifica que el bitmap de roles del party puede formar 1T/1H/3D | — |
| `LFGMgr::HasIgnore(g1, g2)` | Static: chequea social ignore-list | `Player::GetSocial()` |
| `LFGMgr::LoadLFGDungeons(reload)` | Lee `LFGDungeons.dbc`, agrega `lfg_dungeon_template` overrides + `lfg_entrances` coords | `sLFGDungeonsStore`, `WorldDatabase` |
| `LFGMgr::LoadRewards()` | Lee `lfg_dungeon_rewards` | `WorldDatabase` |
| `LFGMgr::_LoadFromDB(fields, guid)` | Carga estado LFG persistente desde `character_lfg_data` | — |
| `LFGMgr::_SaveToDB(guid, db_guid)` | Save estado a `character_lfg_data` (CHAR_DEL_LFG_DATA + CHAR_INS_LFG_DATA) | `CharacterDatabase` |
| `LFGMgr::SetupGroupMember(guid, gguid)` | Re-inicializa player tras carga del group desde DB | — |
| `LFGMgr::DumpQueueInfo(full)` | Debug dump (`.lfg queue` GM command) | `LFGQueue::DumpQueueInfo` |
| `LFGQueue::AddToQueue(guid, reAdd)` | Inserta en `newToQueueStore` o `currentQueueStore` (front si reAdd) | — |
| `LFGQueue::RemoveFromQueue(guid)` | Borra de stores y de compatibles | `RemoveFromCompatibles` |
| `LFGQueue::FindGroups()` | Itera new vs current, llama `FindNewGroups` → `CheckCompatibility` recursivo | `CheckCompatibility`, `LFGMgr::AddProposal` |
| `LFGQueue::CheckCompatibility(GuidList)` | Determina `LfgCompatibility` entre subset; si `MATCH` crea proposal | `HasIgnore`, `CheckGroupRoles`, `LFGMgr::AddProposal` |
| `LFGQueue::UpdateWaitTimeAvg/Tank/Healer/Dps` | Rolling-avg de wait time (Welford-style accumulator) | — |

---

## 5. Module dependencies

**Depends on:**
- `Group` — `Group::Create`, `AddMember`, `RemoveMember`, `Disband`, `isLFGGroup`, `SetLfgRoles`
- `Player` — `TeleportTo`, `HasAura`, `RewardQuest`, `GetSocial`, `GetSession`, `GetGroup`
- `Map` / `InstanceMap` — instance binding (`InstanceSave`), encounter tracking
- `WorldStateMgr` — LFG raid weeks (seasonal flag)
- `Spell` — `LFG_SPELL_DUNGEON_DESERTER=71041`, `LFG_SPELL_DUNGEON_COOLDOWN=71328`, `LFG_SPELL_LUCK_OF_THE_DRAW=72221`
- `DBCStores` — `sLFGDungeonsStore`, `sMapStore`, `sAchievementStore` (lock checks)
- `DB2Manager` — `GetMapDifficultyData(map, difficulty)`
- `WorldDatabase` — `lfg_dungeon_template`, `lfg_dungeon_rewards`, `lfg_entrances`
- `CharacterDatabase` — `character_lfg_data` (`CHAR_DEL_LFG_DATA`, `CHAR_INS_LFG_DATA`)
- `ScriptMgr` — `LFGPlayerScript`, `LFGGroupScript` hooks
- `World` — `CONFIG_LFG_OPTIONSMASK` config

**Depended on by:**
- `WorldSession::HandleLfg*` (15+ opcodes)
- `Group::Disband`, `Group::RemoveMember` — invocan `LFGGroupScript::OnRemoveMember`
- `Player::OnLogin/OnLogout/OnMapChanged` — invocan `LFGPlayerScript`
- `BattlegroundMgr` — chequea `IsLfgGroup` para evitar cruzar BG/LFG
- `InstanceScript::SetBossState(_, DONE)` → `OnDungeonEncounterDone`

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT dungeonId, maxLevel, firstQuestId, otherQuestId FROM lfg_dungeon_rewards ORDER BY dungeonId, maxLevel ASC` | Random dungeon rewards por nivel | world |
| `SELECT * FROM lfg_dungeon_template` | Override de DBC (mapId, type, level range) | world |
| `SELECT * FROM lfg_entrances` | Coords (x,y,z,o) de entrada por dungeon | world |
| `CHAR_DEL_LFG_DATA` (`DELETE FROM character_lfg_data WHERE guid = ?`) | Limpia antes de save | character |
| `CHAR_INS_LFG_DATA` (`INSERT INTO character_lfg_data VALUES (?, ?, ?)`) | Persiste dungeon + state | character |
| `SELECT * FROM character_lfg_data WHERE guid = ?` | Carga estado LFG en login | character |

**DBC/DB2 stores leídos:**

| Store | What it loads | Read by |
|---|---|---|
| `sLFGDungeonsStore` | LFGDungeons.dbc (id, name, map, type, expansion, minlevel, maxlevel, difficulty, flags) | `LFGMgr::LoadLFGDungeons` |
| `sMapStore` | Map.dbc (validar `dungeon.map`) | `LoadLFGDungeons` |
| `sAchievementStore` (indirecto via lock checks) | Achievement-required dungeons | `GetCompatibleDungeons` |
| `MapDifficulty` (DB2) | `dbcManager.GetMapDifficultyData(map, diff)` | `LoadLFGDungeons` |

---

## 7. Wire-protocol packets

> Nota: en este fork legacy los opcodes CMSG son `CMSG_DF_*` (Dungeon Finder), no `CMSG_LFG_*`. Los `CMSG_LFG_LIST_*` corresponden al sistema **Premade Group Finder** (LFGList), que es distinto del LFG automático.

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_DF_JOIN` | client → server | `WorldSession::HandleLfgJoinOpcode` |
| `CMSG_DF_LEAVE` | client → server | `WorldSession::HandleLfgLeaveOpcode` |
| `CMSG_DF_PROPOSAL_RESPONSE` | client → server | `WorldSession::HandleLfgProposalResultOpcode` |
| `CMSG_DF_SET_ROLES` | client → server | `WorldSession::HandleLfgSetRolesOpcode` |
| `CMSG_DF_TELEPORT` | client → server | `WorldSession::HandleLfgTeleportOpcode` |
| `CMSG_DF_BOOT_PLAYER_VOTE` | client → server | `WorldSession::HandleLfgSetBootVoteOpcode` |
| `CMSG_DF_GET_SYSTEM_INFO` | client → server | `HandleLfgGetSystemInfoOpcode` (envía locks + rewards iniciales) |
| `CMSG_DF_GET_JOIN_STATUS` | client → server | Ping de estado de cola |
| `SMSG_LFG_PLAYER_INFO` | server → client | `BuildPlayerLockDungeonBlock` |
| `SMSG_LFG_PARTY_INFO` | server → client | Locks de cada miembro del party |
| `SMSG_LFG_UPDATE_STATUS` | server → client | `LFGMgr::SendLfgUpdateStatus` |
| `SMSG_LFG_ROLE_CHOSEN` | server → client | `LFGMgr::SendLfgRoleChosen` |
| `SMSG_LFG_ROLE_CHECK_UPDATE` | server → client | `LFGMgr::SendLfgRoleCheckUpdate` |
| `SMSG_LFG_JOIN_RESULT` | server → client | `LFGMgr::SendLfgJoinResult` |
| `SMSG_LFG_QUEUE_STATUS` | server → client | `LFGMgr::SendLfgQueueStatus` |
| `SMSG_LFG_PLAYER_REWARD` | server → client | En `FinishDungeon` |
| `SMSG_LFG_BOOT_PROPOSAL_UPDATE` | server → client | Vote-kick state |
| `SMSG_LFG_PROPOSAL_UPDATE` | server → client | `LFGMgr::SendLfgUpdateProposal` |
| `SMSG_LFG_DISABLED` | server → client | Cuando `LFG_OPTION_ENABLE_DUNGEON_FINDER` off |
| `SMSG_LFG_OFFER_CONTINUE` | server → client | Re-invitar a continuar dungeon |
| `SMSG_LFG_TELEPORT_DENIED` | server → client | Razón `LfgTeleportResult` |
| (Premade Group Finder, 11 opcodes adicionales) | both | `CMSG_LFG_LIST_JOIN/LEAVE/SEARCH/SET_ROLES/...` ver `LFGList.cpp` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/lfg` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/mod.rs` | `file` | 1 | 20 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *NINGUNO* dedicado a LFG.
- Stubs vacíos en `crates/wow-world/src/handlers/misc.rs`:
  - L601 `pub async fn handle_df_get_system_info(&mut self, _pkt: WorldPacket) {}`
  - L610 `pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: WorldPacket) {}`
  - L611 `pub async fn handle_lfg_list_get_status(&mut self, _pkt: WorldPacket) {}`
- Registry table `crates/wow-world/src/handlers/mod.rs` L221, 302, 311 (mapping opcode → handler vacío).

**What's implemented:**
- Stubs que aceptan + descartan los packets sin enviar respuesta. El cliente queda colgado en "Joining queue..." indefinidamente.

**What's missing vs C++:**
- 100% de la lógica LFG. Concretamente:
  - LFGMgr singleton + ticking 15s
  - Cargas DB: `lfg_dungeon_rewards`, `lfg_dungeon_template`, `lfg_entrances`, `LFGDungeons.dbc`
  - Persistencia `character_lfg_data`
  - Role-check state machine (45s)
  - Compatibility / matchmaking algorithm
  - Proposal state (45s, accept/decline)
  - Teleport in/out a coords del dungeon
  - Reward chain (firstQuest/otherQuest, Luck of the Draw, Dungeon Cooldown)
  - Vote-kick (3 votes, 120s, kicks-left counter)
  - Lock-dungeon evaluation (level, GS, quest, achievement, attunement)
  - Hook chain `LFGPlayerScript`/`LFGGroupScript`
  - Wait-time avg per role
  - Premade Group Finder (LFGList) — 11 opcodes adicionales
  - Builders SMSG_LFG_* completos

**Suspicious / likely divergent:**
- Stubs no logean — un `/df join` desde cliente no aparece ni en debug log.
- `Group::isLFGGroup` probablemente no existe en Rust → instance binding LFG no se distingue de party normal → los players podrían no recibir Luck of the Draw ni teleport-back en wipe.

**Tests existing:**
- 0 tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#DUNGEONFINDING.WBS.001** Cerrar la migracion auditada de `game/DungeonFinding/LFG.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFG.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.002** Cerrar la migracion auditada de `game/DungeonFinding/LFG.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFG.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.003** Cerrar la migracion auditada de `game/DungeonFinding/LFGGroupData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGGroupData.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.004** Cerrar la migracion auditada de `game/DungeonFinding/LFGGroupData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGGroupData.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.005** Cerrar la migracion auditada de `game/DungeonFinding/LFGList.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGList.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.006** Cerrar la migracion auditada de `game/DungeonFinding/LFGList.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGList.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.007** Partir y cerrar la migracion auditada de `game/DungeonFinding/LFGMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2241 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.008** Partir y cerrar la migracion auditada de `game/DungeonFinding/LFGMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 509 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.009** Cerrar la migracion auditada de `game/DungeonFinding/LFGPlayerData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGPlayerData.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.010** Cerrar la migracion auditada de `game/DungeonFinding/LFGPlayerData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGPlayerData.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.011** Partir y cerrar la migracion auditada de `game/DungeonFinding/LFGQueue.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 739 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.012** Cerrar la migracion auditada de `game/DungeonFinding/LFGQueue.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.013** Cerrar la migracion auditada de `game/DungeonFinding/LFGScripts.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGScripts.cpp`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DUNGEONFINDING.WBS.014** Cerrar la migracion auditada de `game/DungeonFinding/LFGScripts.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGScripts.h`
  Rust target: `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Complejidad: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#LFG.1** Crear crate-module `crates/wow-world/src/lfg/mod.rs` con submódulos `mgr`, `queue`, `player_data`, `group_data`, `proposal`, `boot`, `roles`, `reward` (L)
- [ ] **#LFG.2** Definir todos los enums: `LfgState`, `LfgRoles` (bitmask), `LfgUpdateType` (22 variantes), `LfgJoinResult` (21), `LfgRoleCheckState` (7), `LfgTeleportResult`, `LfgProposalState`, `LfgLockStatusType`, `LfgAnswer`, `LfgQueueType`, `LfgCompatibility` (M)
- [ ] **#LFG.3** Constantes `LfgMgrEnum`: TIME_ROLECHECK=45s, TIME_BOOT=120s, TIME_PROPOSAL=45s, QUEUEUPDATE_INTERVAL=15s, SPELL_DUNGEON_COOLDOWN=71328, SPELL_DUNGEON_DESERTER=71041, SPELL_LUCK_OF_THE_DRAW=72221, GROUP_KICK_VOTES_NEEDED=3 (L)
- [ ] **#LFG.4** Structs: `LFGDungeonData`, `LfgReward`, `LfgQueueData`, `LfgRoleCheck`, `LfgProposal`, `LfgProposalPlayer`, `LfgPlayerBoot`, `LfgUpdateData`, `LfgQueueStatusData`, `LfgPlayerRewardData`, `LfgJoinResultData`, `LfgLockInfoData` (M)
- [ ] **#LFG.5** `LfgPlayerData` (per-player cache: state, old_state, roles, selected_dungeons, team, group, comment, ticket) (M)
- [ ] **#LFG.6** `LfgGroupData` (per-group cache: state, old_state, dungeon, leader, players, kicks_left, vote_kick_active) (M)
- [ ] **#LFG.7** `LFGMgr` singleton (`OnceCell<Arc<RwLock<LfgMgr>>>` o `DashMap`-backed) con todos los stores (M)
- [ ] **#LFG.8** Loader `load_lfg_dungeons(reload)` desde `sLFGDungeonsStore` + override `lfg_dungeon_template` + coords `lfg_entrances` (H)
- [ ] **#LFG.9** Loader `load_rewards()` desde `lfg_dungeon_rewards` (M)
- [ ] **#LFG.10** Persistencia: `_save_to_db(guid)` y `_load_from_db(fields, guid)` con prepared statements `CHAR_DEL_LFG_DATA` y `CHAR_INS_LFG_DATA` en CharacterDatabase (M)
- [ ] **#LFG.11** `update(diff)` tick: avanza role-checks (45s), proposals (45s), boots (120s); cada 15s llama `find_groups` por queue (H)
- [ ] **#LFG.12** `LFGQueue` struct con `QueueDataStore`, `CompatibleMapStore`, `current_queue`, `new_to_queue`, wait-time stores por role (M)
- [ ] **#LFG.13** `add_to_queue(guid, re_add)` / `remove_from_queue(guid)` con cleanup en `compatibles` (M)
- [ ] **#LFG.14** `find_groups()` algoritmo: itera `new` cross-product `current`, llama `check_compatibility` recursivo con cache (H)
- [ ] **#LFG.15** `check_compatibility(guidlist)` evalúa: tamaño, ignore-list, roles válidos (`check_group_roles` 1T/1H/3D), states. Devuelve `LfgCompatibility::Match` → `add_proposal` (H)
- [ ] **#LFG.16** `check_group_roles(roles_map)` static: backtracking sobre roles bitmask para asignar 1 tank + 1 healer + 3 dps (H)
- [ ] **#LFG.17** `update_wait_time_{avg,tank,healer,dps}` rolling-avg accumulator por dungeon (M)
- [ ] **#LFG.18** `join_lfg(player, roles, dungeons)`: validaciones (deserter aura 71041, cooldown aura 71328, dungeons válidos, level/GS/quest locks, no en BG/arena), expand RANDOM si len==1, role-check si party, enqueue (XL)
- [ ] **#LFG.19** `leave_lfg(guid, disconnected)`: state-aware cleanup (cancela rolecheck/proposal/queue) (M)
- [ ] **#LFG.20** `update_role_check(gguid, guid, roles)`: acumula, valida con `check_group_roles`, dispara enqueue o `LFG_ROLECHECK_FAILED` (M)
- [ ] **#LFG.21** `update_proposal(proposalId, guid, accept)`: acumula respuestas; all-AGREE → `make_new_group`; DENY o timeout → `remove_proposal(FAILED)` (M)
- [ ] **#LFG.22** `make_new_group(proposal)`: crea Group con flag `LFG`, asigna `InstanceSave`, llama `teleport_player` para cada miembro (H)
- [ ] **#LFG.23** `teleport_player(player, out, from_opcode)`: coords del dungeon (x,y,z,o de `lfg_dungeon_template`), validar dead/falling/transport/exhaustion/no_return, enviar `SMSG_LFG_TELEPORT_DENIED` con razón (M)
- [ ] **#LFG.24** `finish_dungeon(gguid, dungeonId, currMap)`: aplica reward (firstQuest if !done else otherQuest), aura `Dungeon Cooldown 71328` si fue random, set state `FINISHED_DUNGEON`, envía `SMSG_LFG_PLAYER_REWARD` (H)
- [ ] **#LFG.25** `on_dungeon_encounter_done(gguid, encounter_ids, currMap)`: si encuentra `finalDungeonEncounterId` → `finish_dungeon` (M)
- [ ] **#LFG.26** Aplicar `Luck of the Draw 72221` al teleport in si dungeon es RANDOM y party es full LFG (L)
- [ ] **#LFG.27** Aplicar `Dungeon Deserter 71041` (30 min) al `LeaveLfg` mid-dungeon o `Group::Disband` durante DUNGEON state (M)
- [ ] **#LFG.28** `init_boot(gguid, kguid, vguid, reason)` + `update_boot(guid, accept)`: vote-kick, 3 AGREE → `Group::RemoveMember`, decrement `kicks_left` (M)
- [ ] **#LFG.29** `get_locked_dungeons(guid) -> LfgLockMap`: evalúa cada dungeon contra player level, GS, quest req, achievement req, attunement, season (H)
- [ ] **#LFG.30** `get_random_and_seasonal_dungeons(level, expansion)` para system-info packet (M)
- [ ] **#LFG.31** `get_random_dungeon_reward(dungeon, level)`: `multimap` lookup de `RewardMapStore` con upper_bound por nivel (L)
- [ ] **#LFG.32** Helpers `concatenate_dungeons`, `get_roles_string`, `get_state_string` (L)
- [ ] **#LFG.33** Implementar handler `handle_df_join` en `crates/wow-world/src/handlers/lfg.rs` (parse roles + dungeons + comment, llama `LFGMgr::join_lfg`) (M)
- [ ] **#LFG.34** Handler `handle_df_leave` (M)
- [ ] **#LFG.35** Handler `handle_df_proposal_response` (M)
- [ ] **#LFG.36** Handler `handle_df_set_roles` (L)
- [ ] **#LFG.37** Handler `handle_df_teleport` (out only) (L)
- [ ] **#LFG.38** Handler `handle_df_boot_player_vote` (L)
- [ ] **#LFG.39** Handler `handle_df_get_system_info` (reemplazar stub L601 en `misc.rs`): envía locks + random rewards iniciales (M)
- [ ] **#LFG.40** Handler `handle_df_get_join_status` (L)
- [ ] **#LFG.41** Builders SMSG: `lfg_player_info`, `lfg_party_info`, `lfg_update_status`, `lfg_role_chosen`, `lfg_role_check_update`, `lfg_join_result`, `lfg_queue_status`, `lfg_player_reward`, `lfg_boot_proposal_update`, `lfg_proposal_update`, `lfg_disabled`, `lfg_offer_continue`, `lfg_teleport_denied` (XL — splitear en 4 sub-PRs)
- [ ] **#LFG.42** Hook `LFGPlayerScript`: `on_login` (envía info), `on_logout` (`leave_lfg(disconnected=true)`), `on_map_changed` (cancela teleport pending) (M)
- [ ] **#LFG.43** Hook `LFGGroupScript`: `on_add_member`, `on_remove_member`, `on_disband`, `on_change_leader`, `on_invite_member` (M)
- [ ] **#LFG.44** Premade Group Finder (LFGList): 11 opcodes (`CMSG_LFG_LIST_*`) + storage + matching propio — splitear en su propio doc / sub-batch (XL)
- [ ] **#LFG.45** Disable check: `DisableMgr.is_disabled_for(LfgMap, dungeonId)` antes de listar (L)
- [ ] **#LFG.46** Config integration: `CONFIG_LFG_OPTIONSMASK` desde `world.conf` (L)
- [ ] **#LFG.47** GM commands `.lfg queue` (`DumpQueueInfo`), `.lfg clean`, `.lfg options` (M)
- [ ] **#LFG.48** Cross-realm guard: `LFG_JOIN_PARTY_PLAYERS_FROM_DIFFERENT_REALMS=0x24` chequeo (L)
- [ ] **#LFG.49** Seasonal flag: `LFG_FLAG_SEASONAL=0x4` honrar `is_season_active(dungeonId)` (M)
- [ ] **#LFG.50** Telemetry: log `lfg.update`, `lfg.queue`, `lfg.proposal` con tracing spans (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#DUNGEONFINDING.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 14 files / 4861 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#DUNGEONFINDING.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 14 files / 4861 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#DUNGEONFINDING.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 14 files / 4861 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#DUNGEONFINDING.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 14 files / 4861 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `check_group_roles({A:Leader|Tank, B:Healer, C:Dps, D:Dps, E:Dps})` → true, retorna asignación deterministic
- [ ] Test: `check_group_roles({A:Tank, B:Tank, C:Dps, D:Dps, E:Dps})` → false (no healer)
- [ ] Test: player con `roles=Tank|Healer` puede ser asignado a cualquiera de los dos slots
- [ ] Test: `join_lfg` con player en estado `BG` retorna `LFG_JOIN_CANT_USE_DUNGEONS`
- [ ] Test: `join_lfg` con aura `71041` retorna `LFG_JOIN_DESERTER_PLAYER`
- [ ] Test: `join_lfg` con aura `71328` y dungeon RANDOM retorna `LFG_JOIN_RANDOM_COOLDOWN_PLAYER`
- [ ] Test: queue de 5 solo-players compatibles (1T+1H+3D) → genera proposal en 1 tick `find_groups`
- [ ] Test: proposal con 5/5 AGREE → `make_new_group`, todos teleportados a coords del dungeon
- [ ] Test: proposal con 1 DENY → `remove_proposal(FAILED)`, los 4 restantes regresan a queue (re_add=true)
- [ ] Test: proposal timeout 45s sin respuestas → `LFG_PROPOSAL_FAILED`, deserter NO aplicado (sólo si abandonaron mid-dungeon)
- [ ] Test: `finish_dungeon` con dungeon RANDOM otorga firstQuest si !done, otherQuest si done; aplica `LFG_SPELL_DUNGEON_COOLDOWN`
- [ ] Test: `init_boot` + 3 votos AGREE en 60s → kick efectivo via `Group::RemoveMember`
- [ ] Test: `init_boot` + 2 AGREE / 1 DENY → boot cancelado (necesita 3)
- [ ] Test: `init_boot` timeout 120s sin votos → cancel
- [ ] Test: `kicks_left` decrementa de 3 a 0; `init_boot` con 0 retorna error
- [ ] Test: `_load_from_db` restaura state DUNGEON, dungeon-id, group correctamente; `_save_to_db` round-trip idéntico
- [ ] Test: rolling wait-time avg con 100 samples coincide con cálculo Welford ground-truth
- [ ] Test: `get_locked_dungeons` para player level=70 incluye lock `TOO_HIGH_LEVEL` para Ragefire Chasm (15-21)
- [ ] Test: `get_random_dungeon_reward(dungeon=RFD_random, level=80)` devuelve entry con `maxLevel>=80`
- [ ] Test: SMSG_LFG_QUEUE_STATUS bytes coinciden con captura WoW.exe 3.4.3 (parity test)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 14 files / 4861 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h` | `crates/wow-world/src/lfg/` (a crear) + handlers en `crates/wow-world/src/handlers/` \| ❌ not started (stubs vacíos en `handlers/misc.rs`) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#DUNGEONFINDING.DIV.001` | `crates/wow-world/src/lfg` (`missing_declared_path`, 0 Rust lines) | 14 C++ files / 4861 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGQueue.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/DungeonFinding/LFGMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Opcode naming en este fork**: `CMSG_DF_*` (Dungeon Finder) para LFG automático, `CMSG_LFG_LIST_*` para Premade Group Finder (LFGList). Son **dos sistemas distintos**. No confundir.
- `LFG_TANKS_NEEDED=1`, `LFG_HEALERS_NEEDED=1`, `LFG_DPS_NEEDED=3`. Hardcoded; no se puede correr LFG con composición distinta sin patch.
- `LFGDungeonData::Entry()` = `id | (type << 24)`. El cliente espera el **entry packed**, no el id raw. Bug clásico: olvidar el shift.
- `LfgRoleCheckState::LFG_ROLECHECK_INITIALITING` (sí, **typo** intencional en TC para compat cliente).
- Vote-kick necesita `GROUP_KICK_VOTES_NEEDED=3` AGREE; si group tiene <5 miembros (alguien ya salió), basta majority but no menos de 2.
- **Random dungeon expansion**: si `dungeons.size()==1` y es de tipo `LFG_TYPE_RANDOM`, `JoinLfg` lo expande al set completo de dungeons del `group_id` correspondiente (`GetDungeonsByRandom`).
- Cross-realm: `LFG_JOIN_PARTY_PLAYERS_FROM_DIFFERENT_REALMS=0x24` — en este fork single-realm probablemente N/A pero el chequeo debe existir para parity.
- `LFG_QUEUEUPDATE_INTERVAL=15s` es el tick de matchmaking, NO de role-check ni de proposal (ambos usan timers individuales 45s).
- `Luck of the Draw 72221` se aplica al teleport-in y **al revivir** dentro del dungeon LFG; debe re-aplicarse en `LFGPlayerScript::OnMapChanged`.
- `Dungeon Deserter 71041` aplica 30 min; si el group se disuelve naturalmente (no `LeaveLfg` mid-dungeon), no debe aplicar.
- Persistencia: `character_lfg_data` sólo guarda `dungeon` + `state`; el role/comment/dungeon-set se reconstruye en login si state==DUNGEON.
- `LfgQueueType::LFR=2, SCENARIO=3, FLEX=4, WORLD_PVP=5, SCHEDULED_PVP=6` son post-WoLK; en 3.4.x clásico solo DUNGEON=1 está activo (LFR Dragon Soul-style llegó en 4.3, scenarios en MoP).
- `LFGMgr::CheckGroupRoles` es **static** y mutating (modifica `roles_map` con la asignación elegida). Cuidado al portar a Rust (necesita `&mut`).
- `LFGQueue::FindGroups` usa caching agresivo en `CompatibleMapStore` (key = sorted-concat de GUIDs). Sin esto, escala O(n!) en groups grandes.
- C++ tiene un **bug histórico** documentado: `LFG_UPDATETYPE_LEADER_UNK1=1` y `_GROUP_DISBAND_UNK16=18` se disparan a veces sin razón; copiar el comportamiento bit-a-bit para parity, NO "arreglar".

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class LFGMgr` (singleton) | `static LFG_MGR: OnceCell<Arc<RwLock<LfgMgr>>>` | O DashMap-backed para reducir contention |
| `class LFGQueue` | `struct LfgQueue` (per queue-id) | `HashMap<LfgQueueType, LfgQueue>` |
| `LfgQueueContainer = std::map<uint8, LFGQueue>` | `BTreeMap<u8, LfgQueue>` o `HashMap<LfgQueueType, LfgQueue>` | — |
| `LfgPlayerDataContainer = std::map<ObjectGuid, LfgPlayerData>` | `DashMap<Guid, LfgPlayerData>` | Acceso concurrente |
| `LfgGroupDataContainer = std::map<ObjectGuid, LfgGroupData>` | `DashMap<Guid, LfgGroupData>` | — |
| `LfgRewardContainer = std::multimap<uint32, LfgReward const*>` | `HashMap<u32, Vec<LfgReward>>` ordenado por `maxLevel` | upper_bound por level |
| `LfgDungeonSet = std::set<uint32>` | `BTreeSet<u32>` | Set ordenado |
| `LfgRolesMap = std::map<ObjectGuid, uint8>` | `BTreeMap<Guid, u8>` o `HashMap` | — |
| `enum LfgRoles` (bitmask) | `bitflags! struct LfgRoles: u8 { TANK=0x02, HEALER=0x04, DAMAGE=0x08, LEADER=0x01 }` | Use `bitflags` crate |
| `enum LfgState : uint8` | `#[repr(u8)] enum LfgState` | — |
| `enum LfgUpdateType` (22 variantes) | `#[repr(u8)] enum LfgUpdateType` | Mantener valores numéricos |
| `enum LfgJoinResult` (21 variantes) | `#[repr(u8)] enum LfgJoinResult` | — |
| `enum LfgAnswer` (-1/0/1) | `#[repr(i8)] enum LfgAnswer` | — |
| `LfgProposal` con `LfgProposalPlayerContainer players` | `struct LfgProposal` con `BTreeMap<Guid, LfgProposalPlayer>` | — |
| `time_t cancelTime` | `Instant` o `SystemTime` | Usar `tokio::time::Instant` |
| `void Update(uint32 diff)` | `async fn update(&self, diff: Duration)` | Acumular para tick 15s |
| `static bool CheckGroupRoles(LfgRolesMap&)` | `pub fn check_group_roles(roles: &mut HashMap<Guid, LfgRoles>) -> bool` | Backtracking, sí mutates |
| `static bool HasIgnore(g1, g2)` | `pub async fn has_ignore(g1: Guid, g2: Guid) -> bool` | Async porque consulta SocialMgr |
| `void TeleportPlayer(Player*, bool out, bool fromOpcode)` | `async fn teleport_player(player: &Player, out: bool, from_opcode: bool)` | — |
| `WorldPackets::LFG::RideTicket` | `struct RideTicket { id: u32, type_: u8, time: u32, requester: Guid }` | Wire-format exacto |
| `LFG_SPELL_DUNGEON_COOLDOWN` | `pub const LFG_SPELL_DUNGEON_COOLDOWN: u32 = 71328;` | const u32 |
| `sLFGMgr` macro | `lfg::mgr()` o `LfgMgr::instance()` | — |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

❌ confirmado. Auditado contra `/home/server/rustycore/crates/`.

**Hallazgos clave:**
- No existe `crates/wow-world/src/lfg/`. No existe ningún archivo dedicado a LFG en `crates/wow-world/src/handlers/` (no hay `lfg.rs`, no hay `dungeonfinder.rs`).
- Búsqueda de `LFGMgr`, `LFGQueue`, `LfgPlayerData`, `LfgGroupData`, `LfgProposal`, `LfgRoleCheck`, `LfgReward`, `Luck of the Draw`, `72221`, `71041`, `71328`, `LFGDungeons`, `lfg_dungeon_rewards`, `lfg_entrances`, `character_lfg_data` en todo el workspace: **0 resultados**.
- Cero ELO matchmaking, cero algoritmo `check_group_roles` (1T+1H+3D backtracking), cero state machine (NONE/ROLECHECK/QUEUED/PROPOSAL/DUNGEON/FINISHED).
- `crates/wow-constants/src/opcodes.rs` confirma 8 opcodes CMSG_DF_* (`DfJoin=0x360b`, `DfLeave=0x3614`, `DfProposalResponse=0x3609`, `DfSetRoles=0x3617`, `DfTeleport=0x3619`, `DfBootPlayerVote=0x3618`, `DfGetSystemInfo=0x3615`, `DfGetJoinStatus=0x3616`) y 11+ opcodes LfgList* (Premade Group Finder). De los 8 CMSG_DF_*, **solo 2 están registrados** (`DfGetSystemInfo`, `DfGetJoinStatus`) y ambos son stubs no-op (`handlers/misc.rs:601-602`). De los 11 LfgList*, **solo 2 stubs** (`RequestLfgListBlacklist` L610, `LfgListGetStatus` L611). Los otros 6 CMSG_DF_* y 9 LfgList* opcodes **no están registrados** (caen en unknown-opcode log).

**Riesgo de UI hang silencioso (CRÍTICO — el doc lo flag bien):**
- 🔴 **`CMSG_DF_JOIN`**: opcode parseado por el cliente al pulsar "Find Group" en el Dungeon Finder. **No registrado** en RustyCore — cae en unknown-opcode. El cliente espera `SMSG_LFG_JOIN_RESULT` + `SMSG_LFG_UPDATE_STATUS` (con `LFG_UPDATETYPE_JOIN_QUEUE=6`). Sin esto, el botón "Find Group" se queda con la spinner girando *forever*. Confirmado por la doc §8.
- 🔴 **`CMSG_DF_GET_SYSTEM_INFO`** está stubbed (silent ack). El cliente lo envía al abrir el panel LFG y espera `SMSG_LFG_PLAYER_INFO` con la lista de dungeons + locks + random rewards. Sin respuesta, el panel muestra una lista *vacía* — el botón "Find Group" técnicamente está enabled pero al pulsarlo cae en el caso anterior. Riesgo: usuario ve UI vacía sin error → confusión.
- 🟡 **`CMSG_LFG_LIST_JOIN`** (Premade Group Finder, 0x32f0): no registrado. Si el cliente intenta crear un grupo via "Premade", silent drop. UI Premade muestra "Failed to create" eventualmente (timeout ~30s).
- 🟡 **`CMSG_DF_PROPOSAL_RESPONSE`** (0x3609): no registrado, pero solo relevante si llegara una proposal — cosa que no ocurre porque DfJoin no funciona. Cadena rota arriba.

**Acción:** mantener `❌ not started`. Recomendación táctica para evitar el hang visible (si LFG se posterga ≥1 sprint): añadir un stub explícito `handle_df_join` que envíe `SMSG_LFG_JOIN_RESULT` con `result=LFG_JOIN_INTERNAL_ERROR=0x1A` (o `LFG_JOIN_DESERTER_PLAYER=0x28`) — el cliente entonces muestra un toast de error claro en vez de spinner infinito. Mejor todavía: enviar `SMSG_LFG_DISABLED` cuando llegue `CMSG_DF_GET_SYSTEM_INFO` para que el panel salga directamente con el mensaje "Dungeon Finder is currently disabled". Ambos son ~10 líneas y desbloquean el síntoma percibido sin migrar nada del manager.
