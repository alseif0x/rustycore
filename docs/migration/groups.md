# Migration: Groups (Party / Raid)

> **C++ canonical path:** `src/server/game/Groups/` + `src/server/game/Handlers/GroupHandler.cpp`
> **Rust target crate(s):** `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
> **Layer:** L6
> **Status:** ⚠️ partial — represented invite/accept/decline/leave plus bounded raid conversion, role flags, loot-method/options, assignments, ready check, role changes and role poll; no full BG/BF/original-group category parity, no timeout tick, no markers/full DB persistence/manual-test-ready runtime
> **Audited vs C++:** ✅ complete
> **Last updated:** 2026-05-01

---

## 1. Purpose

Manages parties (≤5) and raids (≤40 in 8 sub-groups of 5), the membership lifecycle, leadership/assistant/main-tank/main-assist roles, loot distribution rules (FFA/Round-Robin/Master/Group/Need-Before-Greed), loot quality threshold, ready-check polling (35s timer), raid markers (8 world-position pings), target icons (8 boss-marker icons), dungeon/raid difficulty (Normal/Heroic/Mythic, normal/heroic/legacy raid), instance-binding tracking, BG/Battlefield-temporary groups, opt-out-of-loot, sub-group reassignment + swap, role-poll, low-level-raid restrictions, ping-unit/ping-world, group XP scaling, member out-of-range tracking, and DB persistence (`groups`, `group_member`, `group_instance`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Groups/Group.cpp` | 1894 | `prefix` |
| `game/Groups/Group.h` | 427 | `prefix` |
| `game/Groups/GroupInstanceRefManager.h` | 43 | `prefix` |
| `game/Groups/GroupInstanceReference.cpp` | 32 | `prefix` |
| `game/Groups/GroupInstanceReference.h` | 40 | `prefix` |
| `game/Groups/GroupMgr.cpp` | 205 | `prefix` |
| `game/Groups/GroupMgr.h` | 63 | `prefix` |
| `game/Groups/GroupRefManager.h` | 33 | `prefix` |
| `game/Groups/GroupReference.cpp` | 37 | `prefix` |
| `game/Groups/GroupReference.h` | 41 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Groups/Group.h` | 427 | `Group` class def, `MemberSlot`, all enums (`GroupType`, `GroupFlags`, `GroupCategory`, `GroupUpdateFlags`, `GroupMemberOnlineStatus`, `GroupMemberFlags`, `GroupMemberAssignment`, `GroupUpdatePetFlags`, `RaidMarker`, `PingSubjectType`) |
| `src/server/game/Groups/Group.cpp` | 1894 | Full group state machine — invite, add/remove, ready-check timer, raid-marker storage, loot-method, target-icons, instance binding, leader-offline timer, sub-group counter, BG group flag |
| `src/server/game/Groups/GroupMgr.h` | 63 | `GroupMgr` singleton — `GroupContainer` (id→Group*), DB-id allocator |
| `src/server/game/Groups/GroupMgr.cpp` | 205 | `Load/Save` from DB, group ID generator, `GetGroupByDbStoreId`, BG-group registration |
| `src/server/game/Groups/GroupReference.h/.cpp` | ~60 | `GroupReference` — RAII back-ref Player⇄Group via `RefMgr` |
| `src/server/game/Groups/GroupRefManager.h` | 33 | Member ref list head |
| `src/server/game/Groups/GroupInstanceReference.h/.cpp` | ~70 | `GroupInstanceReference` — back-ref Group⇄InstanceMap |
| `src/server/game/Groups/GroupInstanceRefManager.h` | 43 | Instance-ref list head |
| `src/server/game/Handlers/GroupHandler.cpp` | 783 | All `CMSG_PARTY_*` / `CMSG_LEAVE_GROUP` / `CMSG_RANDOM_ROLL` / `CMSG_UPDATE_RAID_TARGET` / `CMSG_CONVERT_RAID` / `CMSG_DO_READY_CHECK` etc. |
| `src/server/game/Server/Packets/PartyPackets.h/.cpp` | ~1500 | All party packet definitions (PartyInviteClient/Server, PartyUpdate, PartyMemberFullState, MinimapPing, RaidMarkerChanged, ReadyCheckStarted, …) |
| `src/server/database/Database/Implementation/CharacterDatabase.cpp` (453-470) | ~20 | `CHAR_INS_GROUP`, `CHAR_INS_GROUP_MEMBER`, `CHAR_DEL_GROUP_MEMBER`, `CHAR_UPD_GROUP_LEADER`, `CHAR_UPD_GROUP_TYPE`, `CHAR_UPD_GROUP_DIFFICULTY`, `CHAR_DEL_GROUP_INSTANCE`, etc. |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Group` | class | The group itself — owns `MemberSlotList m_memberSlots`, invite list, instance refs, ready-check state, raid markers |
| `Group::MemberSlot` | nested struct | `{guid, name, race, class, group(=subgroup id), flags, roles, readyChecked}` |
| `GroupMgr` | singleton | Registry of all live `Group*`, DB-id allocator, BG-group bookkeeping |
| `GroupReference` | class | RAII back-link: when a Player has a group, holds a `GroupReference` in `Group::m_memberMgr` |
| `GroupInstanceReference` | class | Holds a Group's binding to a particular instance map |
| `RaidMarker` | struct | `{Location WorldLocation, ObjectGuid TransportGUID}` — one of 8 raid-marker positions |
| `enum GroupType` | enum | `NONE=0`, `NORMAL=1`, `WORLD_PVP=4` |
| `enum GroupFlags : uint16` | enum | `FAKE_RAID=0x001`, `RAID=0x002`, `LFG_RESTRICTED=0x004`, `LFG=0x008`, `DESTROYED=0x010`, `ONE_PERSON_PARTY=0x020`, `EVERYONE_ASSISTANT=0x040`, `GUILD_GROUP=0x100`, `CROSS_FACTION=0x200`, `RESTRICT_PINGS=0x400` |
| `enum GroupCategory : uint8` | enum | `HOME=0`, `INSTANCE=1`, `MAX=2` |
| `enum GroupMemberFlags` | enum | `ASSISTANT=0x01`, `MAINTANK=0x02`, `MAINASSIST=0x04` |
| `enum GroupMemberAssignment` | enum | `MAINTANK=0`, `MAINASSIST=1` |
| `enum GroupMemberOnlineStatus` | enum | `OFFLINE=0`, `ONLINE=1`, `PVP=2`, `DEAD=4`, `GHOST=8`, `PVP_FFA=10`, `AFK=40`, `DND=80`, `RAF=100`, `VEHICLE=200` (bitmask) |
| `enum GroupUpdateFlags` | enum | 18 fields-update bits: `STATUS`, `POWER_TYPE`, `CUR_HP`, `MAX_HP`, `CUR_POWER`, `MAX_POWER`, `LEVEL`, `ZONE`, `POSITION`, `AURAS`, `PET`, `PHASE`, `VEHICLE_SEAT` |
| `enum GroupUpdatePetFlags` | enum | Pet equivalents of above |
| `enum LootMethod` | enum (in `Loot.h`) | `FREE_FOR_ALL`, `ROUND_ROBIN`, `MASTER_LOOT`, `GROUP_LOOT`, `NEED_BEFORE_GREED`, `PERSONAL_LOOT` |
| `enum RemoveMethod` | enum | `DEFAULT`, `KICK`, `LEAVE`, `KICK_LFG` |
| `enum PingSubjectType : uint8` | enum class | `Attack`, `Warning`, `Assist`, `OnMyWay`, `AlertThreat`, `AlertNotThreat`, `Max` |
| `MAX_GROUP_SIZE = 5`, `MAX_RAID_SIZE = 40`, `MAX_RAID_SUBGROUPS = 8`, `TARGET_ICONS_COUNT = 8`, `RAID_MARKERS_COUNT = 8`, `READYCHECK_DURATION = 35000` | constants | |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Group::Create(Player* leader)` | Initialise empty group with leader; DB insert; allocate `m_dbStoreId` | `GroupMgr::GenerateGroupDbStoreId`, `CHAR_INS_GROUP` |
| `Group::AddInvite(Player*)` / `RemoveInvite` / `RemoveAllInvites` | Pending-invite list (rejected on second invite) | — |
| `Group::AddMember(Player*)` | Insert into `m_memberSlots`, assign sub-group, `LinkMember`, send `SMSG_PARTY_UPDATE` to all + `SMSG_PARTY_MEMBER_FULL_STATE` to new member | DB `CHAR_INS_GROUP_MEMBER`, `BroadcastPacket` |
| `Group::RemoveMember(guid, RemoveMethod, kickerGuid, reason)` | Reverse of AddMember; reassign leader if leader leaves; disband if <2 members | `Disband`, `SelectNewPartyOrRaidLeader` |
| `Group::ChangeLeader(guid)` | Re-elects leader; emits `SMSG_GROUP_NEW_LEADER`; persists | DB `CHAR_UPD_GROUP_LEADER` |
| `Group::SetLootMethod(LootMethod)` | Sets loot method enum + emits `PartyUpdate` | DB `CHAR_UPD_GROUP_LOOT_*` |
| `Group::SetMasterLooterGuid(guid)` / `SetLooterGuid(guid)` | Master-loot designation | DB |
| `Group::SetLootThreshold(quality)` | Min item-quality for group/NBG loot | DB |
| `Group::UpdateLooterGuid(WorldObject*, ifneed)` | Round-robin advance — picks next eligible looter | — |
| `Group::Disband(hideDestroy)` | Tears down group: clears refs, DB delete, sends `SMSG_GROUP_DESTROYED` to each member | DB `CHAR_DEL_GROUP`, `CHAR_DEL_GROUP_MEMBER`, `CHAR_DEL_GROUP_INSTANCE_PERM_BINDING` |
| `Group::SetLfgRoles(guid, uint8 roles)` / `GetLfgRoles` | LFG tank/healer/dps role assignment | LFG mgr |
| `Group::SetEveryoneIsAssistant(bool)` | Toggle `GROUP_FLAG_EVERYONE_ASSISTANT` | — |
| `Group::StartReadyCheck(starterGuid, duration)` | Begin 35s ready-check, broadcast `SMSG_READY_CHECK_STARTED`, init per-member `readyChecked=false` | — |
| `Group::EndReadyCheck()` | Cancel/finish ready-check, broadcast `SMSG_READY_CHECK_COMPLETED` | — |
| `Group::SetMemberReadyCheck(slot, bool ready)` | Record one member's response, broadcast `SMSG_READY_CHECK_RESPONSE` | — |
| `Group::AddRaidMarker(markerId, mapId, x, y, z, transport)` | Place 1 of 8 raid markers; emit `SMSG_RAID_MARKERS_CHANGED` | — |
| `Group::DeleteRaidMarker(markerId)` | Remove marker | — |
| `Group::SendRaidMarkersChanged(session)` | Replay all markers to one or all sessions | `BroadcastWorker` |
| `Group::SetTargetIcon(symbol, target, changedBy)` | Place 1 of 8 target icons (skull, cross, square…) on a unit | `SMSG_RAID_TARGET_UPDATE_SINGLE` |
| `Group::SendTargetIconList(session)` | Replay all target icons on group join / icon list req | — |
| `Group::ConvertToRaid()` / `ConvertToLFG()` / `ConvertToGroup()` | Toggle raid/LFG flags, repartition into sub-groups | DB `CHAR_UPD_GROUP_TYPE` |
| `Group::ChangeMembersGroup(guid, subgroup)` | Move a member to a different raid sub-group (0..7) | `_setMembersGroup` |
| `Group::SwapMembersGroups(g1, g2)` | Exchange two members' subgroups | — |
| `Group::SetDungeonDifficultyID(Difficulty)` / `Raid` / `LegacyRaid` | Per-group difficulty selection; broadcasts to all members; resets old instance binds when toggled | DB `CHAR_UPD_GROUP_DIFFICULTY`, `Map::Reset` |
| `Group::ResetInstances(method, notifyPlayer)` | Force-reset all bound instances; broadcasts | DB `CHAR_DEL_GROUP_INSTANCE_PERM_BINDING` |
| `Group::SetBattlegroundGroup(Battleground*)` | Mark group as transient BG group | — |
| `Group::CanJoinBattlegroundQueue(...)` | Validate eligibility (level range, faction) | `Battleground::GetBgLevels` |
| `Group::BroadcastPacket(packet, ignorePlayersInBGRaid, group, ignoredPlayer)` | Send to all members optionally filtered by sub-group | `WorldSession::SendPacket` |
| `Group::BroadcastAddonMessagePacket(...)` | Same but only to recipients with the matching addon prefix subscribed | — |
| `Group::UpdatePlayerOutOfRange(Player*)` | Mark a member out-of-range so partial `PartyMemberFullState` is sent | — |
| `Group::SelectNewPartyOrRaidLeader()` | Picks best new leader on leader DC: highest-level, then highest-account-id | `ChangeLeader` |
| `Group::Update(uint32 diff)` | Per-tick: ready-check timer, leader-offline timer | `EndReadyCheck`, `Disband` |
| `GroupMgr::AddGroup(Group*)` / `RemoveGroup` | In-memory registry maintenance | — |
| `GroupMgr::LoadGroups()` | At startup: recreate Groups from `groups` table + members from `group_member` | DB |
| `GroupMgr::GetGroupByGUID(guid)` / `GetGroupByDbStoreId(id)` | Lookup | — |
| `WorldSession::HandlePartyInviteOpcode` | Validate (target online, no existing invite, leader has space, faction-cross check), call `Group::AddInvite`, send `SMSG_PARTY_INVITE` to target | `Group::Create`/`AddInvite` |
| `WorldSession::HandlePartyInviteResponseOpcode` | Accept → `Group::AddMember`; Decline → `SMSG_GROUP_DECLINE` to inviter | — |
| `WorldSession::HandlePartyUninviteOpcode` | Leader/assistant kicks a member by guid+reason | `Group::RemoveMember(KICK)` |
| `WorldSession::HandleSetPartyLeaderOpcode` | Promote target to leader | `Group::ChangeLeader` |
| `WorldSession::HandleSetRoleOpcode` | Change LFG role bitmask of self/another | `Group::SetLfgRoles` |
| `WorldSession::HandleLeaveGroupOpcode` | Self leaves; if leader, reassigns | `Group::RemoveMember` |
| `WorldSession::HandleSetLootMethodOpcode` | Leader sets loot method + threshold + master | `Group::SetLootMethod`/`SetMasterLooterGuid`/`SetLootThreshold` |
| `WorldSession::HandleMinimapPingOpcode` | Broadcasts minimap-ping (x,y) to other members | `BroadcastPacket` |
| `WorldSession::HandleRandomRollOpcode` | `/roll min max` — random in range, broadcast | `BroadcastPacket` |
| `WorldSession::HandleUpdateRaidTargetOpcode` | Set a target-icon on a unit | `Group::SetTargetIcon` |
| `WorldSession::HandleConvertRaidOpcode` | Promote party to raid | `Group::ConvertToRaid` |
| `WorldSession::HandleChangeSubGroupOpcode` / `HandleSwapSubGroupsOpcode` | Re-arrange raid sub-groups | `Group::ChangeMembersGroup`/`SwapMembersGroups` |
| `WorldSession::HandleSetAssistantLeaderOpcode` | Toggle assistant flag on a member | `Group::SetGroupMemberFlag` |
| `WorldSession::HandleSetPartyAssignment` | Toggle MainTank/MainAssist | `Group::SetGroupMemberFlag` |
| `WorldSession::HandleDoReadyCheckOpcode` | Leader/assist starts 35s ready-check | `Group::StartReadyCheck` |
| `WorldSession::HandleReadyCheckResponseOpcode` | Member responds yes/no | `Group::SetMemberReadyCheck` |
| `WorldSession::HandleRequestPartyMemberStatsOpcode` | Out-of-range member stats refresh | `Group::UpdatePlayerOutOfRange` |
| `WorldSession::HandleRequestRaidInfoOpcode` | Sends saved-instance list + lockout times | `InstanceSaveMgr::SendRaidInfo` |
| `WorldSession::HandleOptOutOfLootOpcode` | Toggle PLAYER_FLAG_OPT_OUT_OF_LOOT | `Player::SetPassOnGroupLoot` |
| `WorldSession::HandleInitiateRolePoll` | Leader requests role poll | broadcasts |
| `WorldSession::HandleSetEveryoneIsAssistant` | Toggle group flag | `Group::SetEveryoneIsAssistant` |
| `WorldSession::HandleClearRaidMarker` | Remove one or all markers | `Group::DeleteRaidMarker` |
| `WorldSession::HandleSetRestrictPingsToAssistants` | Toggle | `Group::SetRestrictPingsToAssistants` |
| `WorldSession::HandleSendPingUnit` / `HandleSendPingWorldPoint` | Modern raid pings | `BroadcastPacket` |
| `WorldSession::HandleLowLevelRaid1/2` | Low-level raid restrictions toggle (no-op, debug log only) — **represented-complete** | — |
| `WorldSession::HandleSilencePartyTalker` | Voice-mute a member (placeholder) | — |
| `WorldSession::HandleRequestPartyJoinUpdates` | LFG requeue heartbeat | — |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Player** — sender Player & target Player; member-stats fetched from `Player` (level, class, zone, hp, power, position, auras, pet)
- **Entities/Unit** — `Unit::GetGUID`, target-icon validity
- **Maps / InstanceMap** — `Map::GetInstanceId`, instance bind + reset (`InstanceSave`)
- **DBC/DB2** — `MapStore` for difficulty validity per map, `BattlemasterListStore` for BG-group eligibility
- **CharacterDatabase** — `groups`, `group_member`, `group_instance` tables
- **Server/WorldSession** — all packet send/recv
- **Loot** — `LootMethod`, `ItemQualities`, `Roll` machinery (group_loot, NBG)
- **LFG** — `Group::SetLfgRoles`, role-poll, kick-during-LFG
- **Battleground / Battlefield** — `m_bgGroup`, `m_bfGroup` references; `CanJoinBattlegroundQueue`
- **Globals/ObjectAccessor** — `FindConnectedPlayerByName` for invite-by-name
- **Chat** — `Group::BroadcastPacket` is consumed by `CMSG_CHAT_MESSAGE_PARTY/RAID/INSTANCE_CHAT`

**Depended on by:**
- **Player.cpp** — `Player::GetGroup()` is queried everywhere XP-share, loot-eligibility, mob-tag-rules, instance-binding apply
- **Loot system** — group-loot/NBG/master-loot all consult `Group`
- **InstanceSaveMgr** — saves bound to group, not individual players, for raid lockouts
- **Chat** — party/raid/instance-chat msg routing
- **Battleground** — group-queue logic
- **LFG / RaidFinder** — full lifecycle of group creation/disband
- **Achievement** — group-related criteria
- **Quest** — quest-share to party members within range
- **Spell** — group-buffs (Mark of the Wild) range checks via group membership

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_INS_GROUP` | INSERT `groups` row at Group::Create | character |
| `CHAR_INS_GROUP_MEMBER` | INSERT `group_member` (guid, memberGuid, memberFlags, subgroup, roles) | character |
| `CHAR_DEL_GROUP_MEMBER` | DELETE `group_member` WHERE memberGuid=? | character |
| `CHAR_DEL_ALL_GROUP_MEMBER` | DELETE `group_member` WHERE guid=? (group-wide) | character |
| `CHAR_DEL_GROUP` | DELETE `groups` WHERE guid=? | character |
| `CHAR_UPD_GROUP_LEADER` | UPDATE `groups` SET leaderGuid=? | character |
| `CHAR_UPD_GROUP_TYPE` | UPDATE `groups` SET groupType=? | character |
| `CHAR_UPD_GROUP_DIFFICULTY` / `_RAID_DIFFICULTY` / `_LEGACY_RAID_DIFFICULTY` | UPDATE difficulty cols | character |
| `CHAR_UPD_GROUP_MEMBER_FLAG` | UPDATE memberFlags | character |
| `CHAR_UPD_GROUP_MEMBER_SUBGROUP` | UPDATE subgroup | character |
| `CHAR_INS_GROUP_INSTANCE` | INSERT `group_instance` for raid bind | character |
| `CHAR_DEL_GROUP_INSTANCE_PERM_BINDING` | DELETE on reset | character |
| `CHAR_SEL_GROUP_MEMBER` | SELECT guid FROM group_member WHERE memberGuid=? | character |
| `CHAR_SEL_GROUPS` (raw) | Bulk-load all groups at startup | character |

DBC/DB2 stores read:

| Store | What it loads | Read by |
|---|---|---|
| `MapStore` | Per-map allowed difficulties, expansion, max-players | `Group::SetDungeonDifficultyID`, `CanJoinBattlegroundQueue` |
| `DifficultyStore` | Difficulty metadata | `Group::GetDifficultyID(MapEntry*)` |
| `BattlemasterListStore` | BG instance ID + map list | `CanJoinBattlegroundQueue` |
| `LFGDungeonStore` | Dungeon→difficulty mapping | LFG (indirect) |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_PARTY_INVITE` | C→S | `HandlePartyInviteOpcode` |
| `CMSG_PARTY_INVITE_RESPONSE` | C→S | `HandlePartyInviteResponseOpcode` |
| `CMSG_PARTY_UNINVITE` | C→S | `HandlePartyUninviteOpcode` |
| `CMSG_SET_PARTY_LEADER` | C→S | `HandleSetPartyLeaderOpcode` |
| `CMSG_SET_ROLE` | C→S | `HandleSetRoleOpcode` |
| `CMSG_LEAVE_GROUP` | C→S | `HandleLeaveGroupOpcode` |
| `CMSG_SET_LOOT_METHOD` | C→S | `HandleSetLootMethodOpcode` |
| `CMSG_MINIMAP_PING` | C→S | `HandleMinimapPingOpcode` |
| `CMSG_RANDOM_ROLL` | C→S | `HandleRandomRollOpcode` |
| `CMSG_UPDATE_RAID_TARGET` | C→S | `HandleUpdateRaidTargetOpcode` |
| `CMSG_CONVERT_RAID` | C→S | `HandleConvertRaidOpcode` |
| `CMSG_CHANGE_SUB_GROUP` | C→S | `HandleChangeSubGroupOpcode` |
| `CMSG_SWAP_SUB_GROUPS` | C→S | `HandleSwapSubGroupsOpcode` |
| `CMSG_SET_ASSISTANT_LEADER` | C→S | `HandleSetAssistantLeaderOpcode` |
| `CMSG_SET_PARTY_ASSIGNMENT` | C→S | `HandleSetPartyAssignment` |
| `CMSG_DO_READY_CHECK` | C→S | `HandleDoReadyCheckOpcode` |
| `CMSG_READY_CHECK_RESPONSE` | C→S | `HandleReadyCheckResponseOpcode` |
| `CMSG_REQUEST_PARTY_MEMBER_STATS` | C→S | `HandleRequestPartyMemberStatsOpcode` |
| `CMSG_REQUEST_RAID_INFO` | C→S | `HandleRequestRaidInfoOpcode` |
| `CMSG_OPT_OUT_OF_LOOT` | C→S | `HandleOptOutOfLootOpcode` |
| `CMSG_INITIATE_ROLE_POLL` | C→S | `HandleInitiateRolePoll` |
| `CMSG_SET_EVERYONE_IS_ASSISTANT` | C→S | `HandleSetEveryoneIsAssistant` |
| `CMSG_CLEAR_RAID_MARKER` | C→S | `HandleClearRaidMarker` |
| `CMSG_SET_RESTRICT_PINGS_TO_ASSISTANTS` | C→S | `HandleSetRestrictPingsToAssistants` |
| `CMSG_SEND_PING_UNIT` / `CMSG_SEND_PING_WORLD_POINT` | C→S | `HandleSendPingUnit` / `HandleSendPingWorldPoint` |
| `CMSG_REQUEST_PARTY_JOIN_UPDATES` | C→S | LFG heartbeat |
| `SMSG_PARTY_INVITE` | S→C | `HandlePartyInviteOpcode` (to target) |
| `SMSG_PARTY_COMMAND_RESULT` | S→C | All party-action results |
| `SMSG_PARTY_UPDATE` | S→C | `Group::SendUpdate` |
| `SMSG_PARTY_MEMBER_FULL_STATE` / `SMSG_PARTY_MEMBER_PARTIAL_STATE` | S→C | Per-member stats refresh |
| `SMSG_GROUP_DECLINE` | S→C | Decline of invite |
| `SMSG_GROUP_NEW_LEADER` | S→C | `ChangeLeader` |
| `SMSG_GROUP_DESTROYED` | S→C | `Disband` |
| `SMSG_GROUP_UNINVITE` | S→C | Self uninvite |
| `SMSG_RAID_GROUP_ONLY` | S→C | Validation failure |
| `SMSG_RANDOM_ROLL` | S→C | `/roll` broadcast |
| `SMSG_RAID_TARGET_UPDATE_SINGLE` / `_ALL` | S→C | Target-icon |
| `SMSG_RAID_MARKERS_CHANGED` | S→C | `SendRaidMarkersChanged` |
| `SMSG_READY_CHECK_STARTED` | S→C | `StartReadyCheck` |
| `SMSG_READY_CHECK_RESPONSE` | S→C | Per-member response |
| `SMSG_READY_CHECK_COMPLETED` | S→C | `EndReadyCheck` |
| `SMSG_MINIMAP_PING` | S→C | Minimap-ping broadcast |
| `SMSG_ROLE_CHANGED_INFORM` | S→C | LFG role change |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-network/src/group_registry.rs` | `file` | 1 | 53 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/group.rs` | `file` | 1 | 467 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/party.rs` | `file` | 1 | 302 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-network/src/group_registry.rs` — 54 lines — `GroupInfo` struct, `GroupRegistry = DashMap<u64, GroupInfo>`, `PendingInvites = DashMap<ObjectGuid, ObjectGuid>` (target → inviter)
- `crates/wow-world/src/handlers/group.rs` — 467 lines — only invite/accept/decline/leave handlers
- `crates/wow-packet/src/packets/party.rs` — N lines — `PartyInviteServer`, `PartyInviteResponse`, `PartyUpdate`, `PartyPlayerInfo`, `PartyMemberFullState`, `PartyLootSettings`, `PartyDifficultySettings`, `PartyCommandResult`, `GroupDecline`, `GroupDestroyed`, `GroupUninvite`

**What's implemented:**
- `GroupInfo` carries: `group_guid: u64` (allocated by `AtomicU64`), `leader_guid`, `members: Vec<ObjectGuid>` (in join order), `loot_method: u8`, `sequence_num: u32`.
- `CMSG_PARTY_INVITE` — name-resolution against `PlayerRegistry`, self-check, pending-invite check, group-full check (≥5), records pending invite, sends `SMSG_PARTY_INVITE` to target, confirms `SMSG_PARTY_COMMAND_RESULT{OK}` to inviter.
- `CMSG_PARTY_INVITE_RESPONSE` — accept: looks up inviter's group (if any) or creates new `GroupInfo`, adds self; broadcasts `SMSG_PARTY_UPDATE` + `SMSG_PARTY_MEMBER_FULL_STATE` to all members; decline: sends `SMSG_GROUP_DECLINE` to inviter.
- `CMSG_LEAVE_GROUP` — removes self; if remaining <2, dissolves group + sends `SMSG_GROUP_DESTROYED` to last member; reassigns leader if leader-leaves; broadcasts updated `PartyUpdate`.
- `PartyMemberFullState` — fills with hard-coded HP/power placeholders (1000/500), real position, level, race-derived faction-group, class-derived power-type.
- Loot method serialised as `loot_method=0`, threshold=2 (uncommon), loot_master=EMPTY — placeholders.
- Difficulty settings serialised as dungeon=1 (Normal), raid=14 (Normal30), legacy_raid=3 (Normal10) — hard-coded defaults.

**What's missing vs C++:**
- **No DB persistence** — groups are 100% in-memory. Server restart = all groups dissolved. `groups`/`group_member`/`group_instance` tables not read or written.
- **Raid represented-partial** — `CMSG_CONVERT_RAID` is parsed/handled through represented HOME-group state (`#NEXT.R8.ENTITIES.745/#746`), but full raid runtime remains incomplete: BG/BF/original-group category parity, full 8×5 layout/cap semantics, DB persistence, and live-client validation remain open.
- **Roles represented-partial** — `CMSG_SET_ROLE` / `SMSG_ROLE_CHANGED_INFORM` are parsed, dispatched, and fan out through the represented current HOME-group model. Remaining gaps: DB persistence, BG/BF/original-group `PartyIndex` parity, and full live ObjectAccessor/sWorld runtime.
- **Assistant / main-tank / main-assist represented-partial** — `CMSG_SET_ASSISTANT_LEADER` / `CMSG_SET_PARTY_ASSIGNMENT` have represented HOME-group wiring, but full category parity, persistence, and live runtime side effects remain open.
- **Leader change represented-partial** — `CMSG_SET_PARTY_LEADER` is parsed/handled (`#NEXT.R8.ENTITIES.905`), but live Player flags/name/faction, script callbacks, full PartyIndex parity, and exact DB transaction semantics remain open.
- **Kick/uninvite represented-partial** — `CMSG_PARTY_UNINVITE` covers bounded member-kick and pending-invite branches (`#NEXT.R8.ENTITIES.936/#937`), but LFG vote-kick, BG/BF/original-group routing, exact destroy/update side effects, scripts, and rollback parity remain open.
- **Loot method follows this C++ branch** — `CMSG_SET_LOOT_METHOD` is parsed/registered/dispatches (`#NEXT.R8.ENTITIES.231`), but runtime mutation is intentionally a represented no-op because this legacy C++ branch comments out the mutation block.
- **No master looter / round-robin advance** — `UpdateLooterGuid` not implemented, so `looter_guid` is always EMPTY and group looting cannot work.
- **Ready check represented-partial** — `CMSG_DO_READY_CHECK` and `CMSG_READY_CHECK_RESPONSE` parse/dispatch through represented current-group state; `SMSG_READY_CHECK_STARTED/RESPONSE/COMPLETED` writers and connected-member fanout exist, including offline/no-session false response approximation and 35s timer state. Missing: full BG/BF/original-group `PartyIndex` category resolution and real `Group::UpdateReadyCheck` timeout tick loop.
- **Raid markers represented-partial** — `GroupInfo` now owns the 8-slot marker state,
  `SMSG_RAID_MARKERS_CHANGED` serializes active marker entries, join updates replay the
  represented marker state, and `Spell::EffectChangeRaidMarker` can add a marker and fan it
  out to connected HOME-group members. Remaining gaps: `CMSG_CLEAR_RAID_MARKER`, DB persistence,
  original/instance/BG/BF group category parity, and full live runtime/manual validation.
- **Target icons represented-partial** — `CMSG_UPDATE_RAID_TARGET` and target-icon packets are represented (`#NEXT.R8.ENTITIES.793`), but full live raid target storage/category/runtime validation and complete fanout remain open.
- **Difficulty switching represented-partial** — `CMSG_SET_DIFFICULTY_ID`, `CMSG_SET_DUNGEON_DIFFICULTY`, and `CMSG_SET_RAID_DIFFICULTY` now route through represented solo/group difficulty state and reset hooks. Remaining gaps: full live `InstanceMap::Reset`, recent/owned instance parity, BG/BF/original-group exclusions, install/restart, and live client/bot validation.
- **No instance binding** — `m_recentInstances` map absent. Group cannot save/restore raid lockouts.
- **Minimap ping / random roll represented** — `CMSG_MINIMAP_PING` and `CMSG_RANDOM_ROLL` are wired through represented HOME-group state. Remaining gaps: BG/BF/original-group routing, full live `Group::BroadcastPacket` semantics, and live client/bot validation.
- **Member stats refresh represented-partial** — `CMSG_REQUEST_PARTY_MEMBER_STATS` and represented `PartyMemberFullState` snapshots are covered by `#NEXT.R8.ENTITIES.794`; full `Group::UpdatePlayerOutOfRange`, live aura/pet/vehicle runtime, original-group ownership, and manual validation remain open.
- **Raid info represented-partial** — `CMSG_REQUEST_RAID_INFO` is wired through represented instance-lock data, but full `InstanceSaveMgr::SendRaidInfo` parity depends on live instance-save/runtime validation.
- **No group flags** — `GROUP_FLAG_RAID/LFG/CROSS_FACTION/EVERYONE_ASSISTANT/RESTRICT_PINGS` not stored.
- **No BG/BF group support** — `m_bgGroup`, `m_bfGroup` absent.
- **No leader-offline timer** — when a leader DCs, leadership is not auto-transferred after a grace period.
- **`Group::Update(diff)` per-tick** — no equivalent. Without this, ready-check can never time out and leader-offline-timer never fires.
- **Opt-out-of-loot, low-level-raid, restrict-pings, role-poll, silence-talker** — all unhandled.
- **`PartyMemberFullState` hard-codes HP/power 1000/500** — ignores real `Player` stats.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `GroupInfo.group_guid: u64` is just a counter; C++ uses `ObjectGuid` (HighGuid::Group + counter). Wire-protocol field expects an `ObjectGuid`; `PartyUpdate.party_guid: ObjectGuid` is built from `group_guid` somehow — verify that conversion produces a client-acceptable HighGuid::Party.
- `existing_gid` lookup iterates the entire `GroupRegistry` per accept — O(N) scaling.
- `pending_invites` uses target-guid as key, but the inviter creates the entry; if inviter then leaves before target accepts, the target's accept will create a phantom group with a logged-out leader. No invite-expiry timer.
- `add_member` in `GroupInfo` does `if !self.members.contains(&guid)` — O(N) but bounded by 5 (or eventually 40) so acceptable.
- `sequence_num += 1` on every mutation — but client expects monotonically-increasing sequence per packet, and ours wraps to 0 on `u32` overflow only after 4B mutations — fine.
- `PartyDifficultySettings { raid_difficulty_id: 14 }` — value 14 is Normal30 in *retail*; in 3.4.3 valid values are 0=Normal, 1=Heroic only. Wrong client-side dropdown likely shown; verify.
- `class_to_power_type` only handles 4 classes (Warrior/Rogue/DK/everyone-else=Mana). Hunters get power-type 0 (Mana) but should be 2 (Focus / 3.4.3 had Mana for Hunter pre-Cata — actually 3.4.3 Hunter uses Mana so this is OK, but flag for re-check).

**Tests existing:**
- 0 unit tests for `GroupInfo` invariants (add/remove/leader-reelect).
- 0 integration tests for the invite→accept→leave handshake.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#GROUPS.WBS.001** Partir y cerrar la migracion auditada de `game/Groups/Group.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1894 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.002** Cerrar la migracion auditada de `game/Groups/Group.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.003** Cerrar la migracion auditada de `game/Groups/GroupInstanceRefManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupInstanceRefManager.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.004** Cerrar la migracion auditada de `game/Groups/GroupInstanceReference.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupInstanceReference.cpp`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.005** Cerrar la migracion auditada de `game/Groups/GroupInstanceReference.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupInstanceReference.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.006** Cerrar la migracion auditada de `game/Groups/GroupMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.007** Cerrar la migracion auditada de `game/Groups/GroupMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.008** Cerrar la migracion auditada de `game/Groups/GroupRefManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupRefManager.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.009** Cerrar la migracion auditada de `game/Groups/GroupReference.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupReference.cpp`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#GROUPS.WBS.010** Cerrar la migracion auditada de `game/Groups/GroupReference.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupReference.h`
  Rust target: `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

> **Drift note (2026-06-15):** this task list predates the recent R8 group/party slices.
> `CMSG_REQUEST_PARTY_JOIN_UPDATES` is represented-partial under `#NEXT.R8.ENTITIES.793`,
> and `CMSG_REQUEST_PARTY_MEMBER_STATS` / the old hard-coded HP/power placeholders are
> represented-partial under `#NEXT.R8.ENTITIES.794`. Keep the remaining open parts there
> as the source of truth: full vehicle/passenger runtime, full aura runtime,
> original-group/category ownership, DB/install/manual-test-ready.

- [ ] **#GROUPS.1** Replace `GroupInfo.group_guid: u64` with proper `ObjectGuid` (HighGuid::Party); fix all wire serialisations. Complejidad: **M**
- [x] **#GROUPS.2** Represent `CMSG_PARTY_UNINVITE` — bounded member kick and pending-invite removal are covered by `#NEXT.R8.ENTITIES.936/#937`; full LFG/BG/BF/original-group and live side effects remain open. Complejidad: **M**
- [x] **#GROUPS.3** Represent `CMSG_SET_PARTY_LEADER` — explicit represented leader transfer and `SMSG_GROUP_NEW_LEADER` are covered by `#NEXT.R8.ENTITIES.905`; full live side effects remain open. Complejidad: **L**
- [x] **#GROUPS.4** Represent `CMSG_SET_ROLE` — per-member role bitmask (Tank/Healer/DPS) and `SMSG_ROLE_CHANGED_INFORM` fanout are covered in `#NEXT.R8.ENTITIES.789`; persistence/full category parity remain open. Complejidad: **M**
- [x] **#GROUPS.5** Represent `CMSG_SET_ASSISTANT_LEADER` + `CMSG_SET_PARTY_ASSIGNMENT` — represented assistant/main-tank/main-assist flags are covered by `#NEXT.R8.ENTITIES.785/#787`; persistence/category/runtime gaps remain open. Complejidad: **M**
- [x] **#GROUPS.6** Represent `CMSG_SET_LOOT_METHOD` as the legacy C++ branch requires — parser/dispatch plus represented no-op are covered by `#NEXT.R8.ENTITIES.231`; mutable loot settings require a different C++ branch or intentional behavior change. Complejidad: **M**
- [ ] **#GROUPS.7** Implement looter rotation (`UpdateLooterGuid`) — round-robin advance on each loot drop. Complejidad: **M**
- [ ] **#GROUPS.8** Implement `CMSG_DO_READY_CHECK` + `CMSG_READY_CHECK_RESPONSE` + 35s timer in a per-tick `Group::update(diff)`; emit `SMSG_READY_CHECK_STARTED/RESPONSE/COMPLETED`. Complejidad: **H**
- [~] **#GROUPS.9** Represent raid markers — the 8-slot `(map, x, y, z, transport)`
  state, `SMSG_RAID_MARKERS_CHANGED` writer/replay, and represented
  `Spell::EffectChangeRaidMarker` add/fanout path are covered by
  `#NEXT.RUNTIME.L3.031j52`. Still missing: `CMSG_CLEAR_RAID_MARKER`, DB persistence,
  original/instance/BG/BF group category parity, full `Group::SendRaidMarkersChanged`
  session targeting, and live-client/manual validation. Complejidad: **M**
- [x] **#GROUPS.10** Represent target icons — represented `CMSG_UPDATE_RAID_TARGET` / target-icon packets are covered by `#NEXT.R8.ENTITIES.793`; full live raid target storage/category/fanout remains open. Complejidad: **M**
- [x] **#GROUPS.11** Represent `CMSG_CONVERT_RAID` — represented conversion is covered by `#NEXT.R8.ENTITIES.745/#746`; full raid cap/layout/category/runtime parity remains open. Complejidad: **H**
- [ ] **#GROUPS.12** Implement raid sub-groups — `subgroup: u8` per member (0..7), `CMSG_CHANGE_SUB_GROUP`, `CMSG_SWAP_SUB_GROUPS`. Complejidad: **H**
- [x] **#GROUPS.13** Represent `CMSG_SET_DUNGEON_DIFFICULTY` / `CMSG_SET_RAID_DIFFICULTY` legacy opcodes — per-group/solo difficulty and represented reset hooks are covered by `#NEXT.R8.ENTITIES.938`, `#NEXT.R8.ENTITIES.939`, and `#NEXT.R8.ENTITIES.943`; full live instance/runtime parity remains open. Complejidad: **M**
- [ ] **#GROUPS.14** Implement DB persistence — schema for `groups`, `group_member`; load on startup via `GroupMgr::load_groups`; persist on Create/AddMember/Disband. Complejidad: **H**
- [x] **#GROUPS.15** Implement represented `CMSG_MINIMAP_PING` — broadcast `(senderGuid, x, y)` to other represented HOME-group members. Remaining runtime/BG/original-group gaps tracked in R8.
- [x] **#GROUPS.16** Implement represented `CMSG_RANDOM_ROLL` — `/roll min max`, broadcast `SMSG_RANDOM_ROLL` to represented HOME-group members including roller. Remaining runtime/BG/original-group gaps tracked in `#NEXT.R8.ENTITIES.941`.
- [x] **#GROUPS.17** Represent `CMSG_REQUEST_PARTY_MEMBER_STATS` — represented `PartyMemberFullState` snapshots are covered by `#NEXT.R8.ENTITIES.794`; full `UpdatePlayerOutOfRange` live runtime remains open. Complejidad: **M**
- [ ] **#GROUPS.18** Wire `Group::update(diff)` into world-tick loop — process ready-check timer, leader-offline timer, looter advancement. Complejidad: **M**
- [x] **#GROUPS.19** Replace represented hard-coded HP/power placeholders in `PartyMemberFullState` with registry/canonical snapshots under `#NEXT.R8.ENTITIES.794`; full live `UpdatePlayerOutOfRange` remains open. Complejidad: **M**
- [ ] **#GROUPS.20** Add invite-expiry timer (60s in C++) — on expiry, drop pending invite + notify inviter. Complejidad: **M**

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#GROUPS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 10 files / 2815 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#GROUPS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 10 files / 2815 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#GROUPS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 10 files / 2815 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#GROUPS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 10 files / 2815 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: 6th invite to a 5-player party returns `PARTY_RESULT_GROUP_FULL`.
- [ ] Test: Inviting self returns `PARTY_RESULT_BAD_PLAYER_NAME`.
- [ ] Test: Pending invite is dropped if inviter leaves before target responds.
- [ ] Test: Leader leaves → highest-level remaining member becomes leader; tied → highest-account-id wins.
- [ ] Test: Ready-check timer fires after exactly 35s if not all members respond → `SMSG_READY_CHECK_COMPLETED` with `READY_CHECK_FAIL` flag.
- [~] Test: Raid marker placed at slot 3 persists across `SendRaidMarkersChanged` to a
  re-joining session — represented packet/state coverage exists in `#NEXT.RUNTIME.L3.031j52`;
  full reconnect/DB/runtime validation remains open.
- [ ] Test: Target icon (skull) on mob A clears the icon from any previous bearer of skull.
- [ ] Test: Convert-to-raid promotes a 5-player party to raid layout with all in subgroup 0.
- [ ] Test: Kick a member by `RemoveMember(KICK)` triggers `SMSG_GROUP_UNINVITE` to kicked + `PartyUpdate` to remaining.
- [ ] Test: Setting loot method to MASTER_LOOT requires master looter to be a current member; else error.
- [ ] Test: Group dissolves DB row on `Disband` (no orphaned `group_member` rows).
- [ ] Test: Group with 2 members survives a disconnect of leader (paused, not destroyed) and re-elects on `LeaderOfflineTimer` expiry.
- [ ] Test: Sub-group swap of 2 members in a 10-player raid preserves total count.
- [ ] Test: Loot threshold below `ITEM_QUALITY_UNCOMMON` (2) auto-loots; above broadcasts roll.
- [x] Test: `CMSG_RANDOM_ROLL { min: 1, max: 100 }` broadcasts `SMSG_RANDOM_ROLL` with result in `[1, 100]` to represented connected group members.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 10 files / 2815 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp` | `crates/wow-network/src/group_registry.rs`, `crates/wow-world/src/handlers/group.rs`, `crates/wow-packet/src/packets/party.rs` \| ⚠️ partial (~15% — invite, accept, decline, leave only; bounded raid conversion, roles, loot and ready-check represented; no markers/full DB persistence/manual-test-ready runtime) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#GROUPS.DIV.001` | _none generated_ | 10 C++ files / 2815 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/Group.h`, `/home/server/woltk-trinity-legacy/src/server/game/Groups/GroupMgr.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **`READYCHECK_DURATION = 35000`ms** — non-negotiable client-side; do NOT shorten without UI also expecting it.
- **`Group::SetLfgRoles` sends a group update only after mutating an existing member slot** — C++ `Group.cpp:1426-1434` returns before `SendUpdate()` for absent targets. Rust `CMSG_SET_ROLE` must still preserve the handler `RoleChangedInform` fanout for changed old/new roles, but only emit `PartyUpdate` after `set_lfg_roles_like_cpp` reports an existing-slot mutation. Current status: represented-partial; `PartyIndex` remains current-group only, fanout uses `GroupRegistry`/`PlayerRegistry`, and this is not manual-test-ready.
- **Group HighGuid is `HighGuid::Party`** in 3.4.3 (later renamed Group). Distinct from `WowAccount` and `Player`. The wire packets compare GUIDs strictly, so use the right HighGuid.
- **`GROUP_FLAG_FAKE_RAID`** — used for arenas / 5v5 BGs to make a 5-player party render as a "raid" in client UI; cannot be combined with real raid mechanics. Don't auto-enable.
- **`SMSG_PARTY_UPDATE` size**: in retail (3.4.3) sub-groups, MainTank/MainAssist and ally-vs-enemy fields appear. The full packet is large; cache it instead of rebuilding per recipient.
- **Member-list ordering matters** — client uses index in `PartyPlayerInfo` array as the in-game frame slot. If you sort/re-order on every update, frames flicker. Preserve join order.
- **`my_index` field in `PartyUpdate`** is the receiver's index in `player_list` — must be recomputed per recipient (current Rust code does this correctly).
- **Disband-on-disconnect race** — when a leader DCs, the C++ starts a `LeaderOfflineTimer` (default 60s). If they reconnect within window, group resumes. Rust will dissolve immediately on session drop unless this is added.
- **Cross-faction group** (`GROUP_FLAG_CROSS_FACTION`) — a 3.4.3+ feature; needs explicit opt-in by leader for cross-faction members to join.
- **Master-loot eligibility** — only members on same map AND within 100y of the looted corpse get loot rights. Easy to forget when implementing UpdateLooterGuid.
- **Instance bind on raid kill** — `Group::SetRecentInstance(mapId, owner, instanceId)` is called by the instance-script post-kill; saves are per-group not per-player.
- **`m_invitees: set<Player*>`** — raw pointers in C++ that the leader can drop on logout; replicate in Rust via `HashSet<ObjectGuid>` plus session liveness check.
- **`Group::BroadcastPacket(ignorePlayersInBGRaid=true)`** — in BG/arena groups, the same `Group` instance is used for the queue but updates should NOT leak to real-world members; honour this flag.
- **Race-faction-group**: alliance races (1,3,4,7,11) → faction_group=1; horde (2,5,6,8,10) → faction_group=2. Pandaren neutral (24) needs special handling. Rust currently uses `if entry.race <= 5 { 1 } else { 2 }` — incorrect for race=5 (Undead/Horde) which would fall in alliance; FIX.
- **`PartyDifficultySettings.raid_difficulty_id`** — in 3.4.3 valid IDs are 0 (Normal10), 1 (Normal25), 2 (Heroic10), 3 (Heroic25). The hard-coded `14` is a post-4.x value; **client may reject or display garbage**.
- **Member-removal cascade** — `RemoveMember` on a leader of a 2-player party triggers full disband, NOT promotion of the remaining player. Rust dissolves at <2 which matches.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Group` | `struct Group` (en `crates/wow-network/src/group_registry.rs` o nuevo `crates/wow-group/`) | currently `GroupInfo`; promote to full Group struct |
| `Group::MemberSlot` | `struct MemberSlot { guid, name, race, class, subgroup, flags, roles, ready_checked }` | NEEDED — currently only `Vec<ObjectGuid>` |
| `class GroupMgr` (singleton) | `struct GroupMgr` with `Arc<DashMap<ObjectGuid, Arc<RwLock<Group>>>>` | currently `GroupRegistry = DashMap<u64, GroupInfo>` |
| `MemberSlotList = list<MemberSlot>` | `Vec<MemberSlot>` | preserves order |
| `InvitesList = set<Player*>` | `HashSet<ObjectGuid>` | invite-expiry needs separate timer |
| `enum GroupType / GroupFlags / GroupCategory / GroupMemberFlags` | `bitflags!` or `#[repr(u8/u16)] enum` (en `wow-constants`) | `GroupFlags` is u16 bitmask |
| `enum GroupUpdateFlags` | `bitflags! { struct GroupUpdateFlags: u32 { ... } }` | 18 bits |
| `enum LootMethod` | `#[repr(u8)] enum LootMethod` | already implicit as u8 in `GroupInfo` |
| `RaidMarker` | `struct RaidMarker { location: WorldLocation, transport: ObjectGuid }` | |
| `m_targetIcons[8]: ObjectGuid[8]` | `[ObjectGuid; 8]` | |
| `m_markers[8]: unique_ptr<RaidMarker>[8]` | `[Option<RaidMarker>; 8]` | |
| `void HandlePartyInviteOpcode(...)` | `async fn handle_party_invite(...)` | already exists |
| `Group::BroadcastPacket(packet, ignorePlayersInBGRaid, group, ignoredPlayer)` | `async fn broadcast_packet(&self, bytes: Vec<u8>, ignore_bg_raid: bool, subgroup: Option<u8>, ignored: Option<ObjectGuid>)` | use `PlayerRegistry` |
| `READYCHECK_DURATION = 35000` | `const READY_CHECK_DURATION: Duration = Duration::from_secs(35);` | |
| `MAX_GROUP_SIZE/RAID_SIZE/RAID_SUBGROUPS` | `const MAX_GROUP_SIZE: usize = 5;` etc. | |

---

*Template version: 1.0 (2026-05-01).* Status: ⚠️ partial — invite/leave only (~15% of behaviour). Critical gaps: no DB persistence, no roles, no loot rules, no ready check, no raid layout, no target icons, no markers.

---

## 13. Audit (2026-05-01)

**Verdict: ⚠️ partial — historical 2026-05-01 snapshot.** This section is retained as audit
history and is superseded for several represented surfaces by later R8/runtime entries; do not use
its line counts or "only 3 handlers" statement as current truth without re-auditing the code.

**Inventory verified:**
- `crates/wow-network/src/group_registry.rs`: historical snapshot. Later slices added represented
  member slots/subgroups/roles/ready-check/target-icons/raid-markers and many packet paths; use
  the task list above plus `current-session-handoff.md` for current boundaries. Instance binds,
  DB persistence, BG/BF/original-group parity and full live runtime remain open.
- `crates/wow-world/src/handlers/group.rs`: **467 lines** (matches doc). Three `inventory::submit!` registrations exactly: `PartyInvite`, `PartyInviteResponse`, `LeaveGroup`. **Zero** other group opcodes wired.

**Confirmed bugs:**
1. **Faction-group race bug**: doc flagged this; verified at `group.rs:77`: `faction_group: if entry.race <= 5 { 1 } else { 2 }`. Race 5 = Undead (Horde) is mis-classified as Alliance. Should be a race→team table lookup.
2. **HP/power placeholders 1000/500**: confirmed at `group.rs:124-127`. Real Player HP/mana is ignored.
3. **`raid_difficulty_id = 14`**: confirmed at `group.rs:105`. Value 14 is post-Cata; 3.4.3 valid IDs are 0..3. Likely renders garbage in client raid difficulty dropdown.
4. **No invite expiry / phantom group**: confirmed — `pending_invites` has no timer; if inviter logs out before target accepts, `handle_party_invite_response` will still create a group with the (now-offline) inviter as leader.
5. **O(N) group lookup on accept**: confirmed at `group.rs:348-351` (`group_reg.iter().find(...)`).
6. **Group GUID is plain `u64`**: confirmed. C++ uses `ObjectGuid::Create<HighGuid::Party>(counter)`. The wire packet `PartyUpdate.party_guid` is fed this raw `u64`; whether the client interprets it correctly depends on how `ObjectGuid` serialization wraps it (needs separate wire-format check, not done here).
7. **No `Group::Update(diff)` per-tick**: confirmed — there is no tick handler at all. Ready-check timer and leader-offline timer cannot exist without it.

**Largest missing surfaces (confirmed):**
- Remaining group CMSG surfaces still include set leader, set role, raid markers, target icons, convert raid, full raid/original-group routing, opt-out-of-loot, role poll, restrict pings, low-level raid, raid info request, member stats request, and runtime `Group::Update(diff)`. Some formerly listed surfaces now have represented coverage (uninvite, loot method, ready check, sub-groups, difficulty, minimap ping, random roll) but still carry represented/runtime boundaries in R8.
- DB persistence: zero `groups` / `group_member` / `group_instance` reads or writes. `wow-database/src/statements/character.rs` has no group statements.
- `MemberSlot` struct: per-member `subgroup`, `flags` (Assistant/MainTank/MainAssist), `roles`, `readyChecked` all absent.
- Raid (40-cap, 8 sub-groups) entirely absent — cap hard-coded to 5 at `group.rs:249`.
- Loot rules: only `loot_method: u8` field, no master-looter/threshold persistence, no `UpdateLooterGuid` round-robin, no roll machinery.
- Tests: 0 unit tests covering `GroupInfo` (verified — the file has no `#[cfg(test)]` block) and 0 integration tests for invite/accept/leave handshake.

**Refuted nothing.** Every concern raised in §8's "Suspicious / likely divergent" list checks out.
