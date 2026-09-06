# Migration: Guilds

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Guilds/` + `src/server/game/Handlers/GuildHandler.cpp` + `src/server/game/GuildFinder/GuildFinderMgr.{h,cpp}`
> **Rust target crate(s):** **NONE — entire system missing.** Should be `crates/wow-guild/` (not yet created), `crates/wow-world/src/handlers/guild.rs` (not yet created), `crates/wow-packet/src/packets/guild.rs` (not yet created).
> **Layer:** L6
> **Status:** ❌ not started (0% — no crate, no handler, no packet definitions, no DB schema in Rust)
> **Audited vs C++:** ✅ complete
> **Last updated:** 2026-05-01

---

## 1. Purpose

Manages guilds — persistent player organizations with: name + leader + creation date + emblem (5-component tabard), MOTD/info text, hierarchical ranks (2-10 ranks per guild, each with bitfield of `GuildRankRights`), per-member roster (rank, public note, officer note, last logout, weekly/total reputation+activity), guild bank (8 tabs × 98 slots = 784 slots + bank money up to 100B copper), per-rank bank withdrawal limits (gold/day, slots/day/tab), event log (100 records), bank transaction log (25 records/tab), news feed (250 entries), guild achievements (separate from player achievements), guild reputation/level/perks/rewards (Cata+ feature, vestigial in 3.4.3), guild challenges, GuildFinder, mass invite to calendar events, party-state tracking. Persists to ~15 character-DB tables.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Guilds/Guild.cpp` | 3656 | `prefix` |
| `game/Guilds/Guild.h` | 956 | `prefix` |
| `game/Guilds/GuildMgr.cpp` | 565 | `prefix` |
| `game/Guilds/GuildMgr.h` | 71 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Guilds/Guild.h` | 956 | `Guild` class, nested `Member`, `RankInfo`, `BankTab`, `EmblemInfo`, `LogEntry`, `EventLogEntry`, `BankEventLogEntry`, `NewsLogEntry`, `LogHolder`, `MoveItemData`/`PlayerMoveItemData`/`BankMoveItemData`; all enums |
| `src/server/game/Guilds/Guild.cpp` | 3656 | Full implementation: roster, bank item move, log appenders, rank rights enforcement, news feed, member presence, achievement tracker, recipe-system, broadcasts |
| `src/server/game/Guilds/GuildMgr.h` | 71 | Singleton `GuildMgr` — registry by id/leader/guid/name, ID generator, GuildRewards loader |
| `src/server/game/Guilds/GuildMgr.cpp` | 565 | Bulk `LoadGuilds`, `LoadGuildRewards` (DBC items reward table), `ResetReputationCaps`, `ResetTimes(week)` |
| `src/server/game/GuildFinder/GuildFinderMgr.h` | ~150 | Guild-Finder applications, listing |
| `src/server/game/GuildFinder/GuildFinderMgr.cpp` | ~600 | Per-guild applicant tracking + matchmaking |
| `src/server/game/Handlers/GuildHandler.cpp` | 813 | All `CMSG_GUILD_*` opcodes (60 handlers) + bank ops (auto-store, swap, split, merge across char inv ↔ guild bank) |
| `src/server/game/Server/Packets/GuildPackets.h/.cpp` | ~3000 | Massive packet catalog: `QueryGuildInfo`/`Response`, `GuildRoster`, `GuildBankList`, `GuildEventLogQueryResults`, `GuildPermissionsQueryResults`, `GuildBankLogQueryResults`, `GuildNews`, `GuildSendRankChange`, `GuildEventBankMoneyChanged`, all bank-item-move packets |
| `src/server/game/Handlers/CharacterHandler.cpp` (relevant) | ~30 | On `WORLD_PACKET_HANDLER_LOGIN` — `Guild::SendLoginInfo` if member |
| `src/server/database/Database/Implementation/CharacterDatabase.cpp` (219-290) | ~70 | `CHAR_INS_GUILD`, `CHAR_DEL_GUILD`, `CHAR_INS_GUILD_MEMBER`, `CHAR_INS_GUILD_RANK`, `CHAR_INS_GUILD_BANK_TAB`, `CHAR_INS_GUILD_BANK_ITEM`, `CHAR_INS_GUILD_BANK_RIGHT`, `CHAR_INS_GUILD_BANK_EVENTLOG`, `CHAR_INS_GUILD_EVENTLOG`, `CHAR_UPD_GUILD_MOTD`, `CHAR_UPD_GUILD_INFO`, `CHAR_UPD_GUILD_LEADER`, `CHAR_UPD_GUILD_RANK_*`, `CHAR_UPD_GUILD_EMBLEM_INFO`, `CHAR_UPD_GUILD_BANK_MONEY`, `CHAR_INS_GUILD_NEWS`, `CHAR_INS_GUILD_ACHIEVEMENT(_CRITERIA)`, …+30 more |
| `src/server/game/Server/Protocol/Opcodes.cpp` | — | Wires `STATUS_LOGGEDIN` for all `CMSG_GUILD_*` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Guild` | class | The guild entity — owns members map, ranks vector, 8 BankTabs, EmblemInfo, 3 LogHolders (event/bank-eventlog × 8 tabs + money/news), bank money, MOTD, info text, achievement mgr |
| `Guild::Member` | nested class | Per-member state: GUID, name, level, race, class, gender, accountId, rankId, publicNote, officerNote, zoneId, achievementPoints, totalActivity/weekActivity, totalRep/weekRep, flags (online/AFK/DND/mobile), bankWithdraw[8], bankWithdrawMoney, trackedCriteriaIds |
| `Guild::RankInfo` | nested class | Per-rank: rankId (0=GuildMaster), rankOrder, name, rights bitmask, bankMoneyPerDay, per-tab `GuildBankRightsAndSlots[8]` |
| `Guild::BankTab` | nested class | Per-tab: tabId, name, icon, text (MOTD-style), `Item*[98]` slots |
| `Guild::EmblemInfo` | class | 5-component tabard: style, color, borderStyle, borderColor, backgroundColor |
| `Guild::GuildBankRightsAndSlots` | class | Per-(rank,tab): rights bitfield (`VIEW_TAB`,`PUT_ITEM`,`UPDATE_TEXT`), slot-withdraw-per-day cap |
| `Guild::LogEntry` | abstract class | Base for log records (guildId, logGuid, timestamp) |
| `Guild::EventLogEntry` | nested class | Records: invite/join/promote/demote/uninvite/leave |
| `Guild::BankEventLogEntry` | nested class | Records: deposit/withdraw item or money, move-item, repair-money, buy-slot |
| `Guild::NewsLogEntry` | nested class | Records: guild-achievement, player-achievement, dungeon-encounter, item-looted/crafted/purchased, level-up, create, event |
| `Guild::LogHolder<Entry>` | template class | FIFO ring with max-size cap (25/100/250) + `m_nextGUID` allocator |
| `Guild::MoveItemData` (abstract) + `PlayerMoveItemData` + `BankMoveItemData` | classes | Item-movement validation + execution between char-inventory and guild-bank |
| `GuildMgr` | singleton | Global registry `unordered_map<lowGuid, Guild*>` + `GuildRewards` vector |
| `GuildReward` | struct | Loaded from `guild_rewards` (item, MinGuildRep, RaceMask, Cost, AchievementsRequired) |
| `GuildAchievementMgr` | class | Per-guild achievement tracker (separate from player achievements) |
| `enum GuildRankRights : uint32` | enum | 22 rights bits: GCHATLISTEN/SPEAK, OFFCHATLISTEN/SPEAK, INVITE, REMOVE, ROSTER, PROMOTE, DEMOTE, SETMOTD, EDIT_PUBLIC_NOTE, VIEWOFFNOTE, EOFFNOTE, MODIFY_GUILD_INFO, WITHDRAW_GOLD_LOCK, WITHDRAW_REPAIR, WITHDRAW_GOLD, CREATE_GUILD_EVENT, ALL=0x00DDFFBF |
| `enum GuildBankRights` | enum | VIEW_TAB=0x01, PUT_ITEM=0x02, UPDATE_TEXT=0x04, FULL=-1 |
| `enum GuildCommandType` | enum | 17 types — what command was attempted (for command-result feedback) |
| `enum GuildCommandError` | enum | 30+ error codes (success, internal, already-in-guild, invited, name-invalid, name-exists, leader-leave, permissions, not-allied, rank-too-high/low, withdraw-limit, not-enough-money, bank-full, item-not-found, too-much-money, …) |
| `enum GuildEventLogTypes` | enum | INVITE_PLAYER=1, JOIN_GUILD=2, PROMOTE_PLAYER=3, DEMOTE_PLAYER=4, UNINVITE_PLAYER=5, LEAVE_GUILD=6 |
| `enum GuildBankEventLogTypes` | enum | 10 types (deposit/withdraw item/money, move, repair, buy-slot, cash-flow-deposit) |
| `enum GuildEmblemError` | enum | SUCCESS, INVALID_TABARD_COLORS, NOGUILD, NOTGUILDMASTER, NOTENOUGHMONEY, INVALIDVENDOR |
| `enum GuildMemberFlags` | enum | NONE=0, ONLINE=1, AFK=2, DND=4, MOBILE=8 |
| `enum GuildNews` | enum | 9 types of news entries |
| `enum class GuildRankId : uint8` | enum class | strongly-typed rank id; 0 = GuildMaster |
| `enum class GuildRankOrder : uint8` | enum class | Display-order index (separate from rank id so ranks can be reordered without renumbering) |
| `enum GuildMisc` | enum | constants: BANK_MAX_TABS=8, BANK_MAX_SLOTS=98, RANKS_MIN_COUNT=2, RANKS_MAX_COUNT=10, RANK_NONE=0xFF, MASTER_DETHRONE_INACTIVE_DAYS=90, OLD_MAX_LEVEL=25, BANK_MONEY_LIMIT=10^11 |
| `MinNewsItemLevel = 353` | const | Items below this ilvl don't appear in news feed |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Guild::Create(Player* leader, name)` | Allocate id, create default 5 ranks, assign leader as GuildMaster, INSERT all DB rows | `GuildMgr::GenerateGuildId`, DB transaction |
| `Guild::Disband()` | Tear down: delete all members, ranks, bank tabs, items, logs; DELETE all DB rows; broadcast `SMSG_GUILD_EVENT_DISBANDED` | DB transaction (~12 deletes) |
| `Guild::AddMember(trans, guid, rankId)` | Insert member at given rank (default = lowest non-master rank); broadcast `SMSG_GUILD_EVENT_PLAYER_JOINED`; emit eventlog `JOIN_GUILD` | DB `CHAR_INS_GUILD_MEMBER`, `CHAR_INS_GUILD_EVENTLOG` |
| `Guild::DeleteMember(trans, guid, isDisbanding, isKicked, canDeleteGuild)` | Remove member; if leader & 0 successors → disband (when `canDeleteGuild=true`); else auto-promote highest-rank online member; emit `LEAVE_GUILD` or `UNINVITE_PLAYER` | DB `CHAR_DEL_GUILD_MEMBER` |
| `Guild::ChangeMemberRank(trans, guid, newRank)` | Re-rank a member; broadcast roster update; emit `PROMOTE_PLAYER`/`DEMOTE_PLAYER` | DB `CHAR_UPD_GUILD_MEMBER_RANK` |
| `Guild::HandleInviteMember(session, name)` | Validate invitee (online, not in guild, not invited, faction), send `SMSG_GUILD_INVITE` to target | `ObjectAccessor::FindConnectedPlayerByName` |
| `Guild::HandleAcceptMember(session)` | Validate pending invite, call `AddMember` | DB transaction |
| `Guild::HandleLeaveMember(session)` | Self-leave; if leader and not last → reject with `ERR_GUILD_LEADER_LEAVE`; if last member → `Disband` | `DeleteMember` |
| `Guild::HandleRemoveMember(session, targetGuid)` | Officer kicks; `GR_RIGHT_REMOVE` required; can't kick higher-rank | `DeleteMember(isKicked=true)` |
| `Guild::HandleUpdateMemberRank(session, guid, demote)` | Promote (rankOrder--) or demote (rankOrder++) by one step; rights `GR_RIGHT_PROMOTE`/`DEMOTE` | `ChangeMemberRank` |
| `Guild::HandleSetMemberRank(...)` | Set arbitrary rank (must be lower than setter's own); GM-only mostly | `ChangeMemberRank` |
| `Guild::HandleSetNewGuildMaster(session, name, isSelfPromote)` | Transfer GM if old GM offline 90+ days OR if requestor IS the GM; updates `m_leaderGuid`, swaps ranks | DB `CHAR_UPD_GUILD_LEADER` |
| `Guild::HandleSetMOTD(session, motd)` | `GR_RIGHT_SETMOTD` required; persist + broadcast `SMSG_GUILD_EVENT_MOTD` | DB `CHAR_UPD_GUILD_MOTD` |
| `Guild::HandleSetInfo(session, info)` | `GR_RIGHT_MODIFY_GUILD_INFO` required | DB `CHAR_UPD_GUILD_INFO` |
| `Guild::HandleSetEmblem(session, EmblemInfo)` | GM-only; charge gold (configurable); validate via `EmblemInfo::ValidateEmblemColors`; persist | DB `CHAR_UPD_GUILD_EMBLEM_INFO` |
| `Guild::HandleSetMemberNote(session, note, guid, isPublic)` | Edit pnote (`GR_RIGHT_EDIT_PUBLIC_NOTE`) or offnote (`GR_RIGHT_EOFFNOTE`); 31-char cap | DB `CHAR_UPD_GUILD_MEMBER_PNOTE`/`OFFNOTE` |
| `Guild::HandleAddNewRank(session, name)` | Append rank if total <10; default no rights | DB `CHAR_INS_GUILD_RANK` |
| `Guild::HandleRemoveRank(session, rankOrder)` | Remove rank if total >2 AND no members hold it | DB `CHAR_DEL_GUILD_RANK` |
| `Guild::HandleShiftRank(session, rankOrder, shiftUp)` | Reorder ranks (changes `rankOrder` only, not `rankId`) | DB `CHAR_UPD_GUILD_RANK_ORDER` |
| `Guild::HandleSetRankInfo(...)` | Edit rank: name, rights, money/day, per-tab rights+slots/day | DB `CHAR_UPD_GUILD_RANK_*` + `CHAR_INS_GUILD_BANK_RIGHT` upsert |
| `Guild::HandleBuyBankTab(session, tabId)` | Purchase next tab (cost: 100g, 250g, 500g, 1000g, 2500g, 5000g, 10000g, 25000g for tabs 1..8); emit `BUY_SLOT` log | DB `CHAR_INS_GUILD_BANK_TAB`, money debit |
| `Guild::HandleSetBankTabInfo(...)` | Set tab name + icon | DB `CHAR_UPD_GUILD_BANK_TAB_INFO` |
| `Guild::HandleMemberDepositMoney(session, amount, cashFlow)` | Move gold from char to bank; cap at `BANK_MONEY_LIMIT`; emit `DEPOSIT_MONEY` log | `Player::ModifyMoney`, DB `CHAR_UPD_GUILD_BANK_MONEY` |
| `Guild::HandleMemberWithdrawMoney(session, amount, repair)` | Reverse; check `bankMoneyPerDay` cap per rank, daily reset; emit `WITHDRAW_MONEY`/`REPAIR_MONEY` | `Player::ModifyMoney` |
| `Guild::SwapItems(player, tabId, slot, destTab, destSlot, splitAmount)` | Bank↔Bank item move; permission check on both tabs | `BankMoveItemData` |
| `Guild::SwapItemsWithInventory(player, toChar, tabId, slot, playerBag, playerSlot, splitAmount)` | Bank↔Player inventory move; emit `DEPOSIT_ITEM`/`WITHDRAW_ITEM` log | `PlayerMoveItemData`, `BankMoveItemData` |
| `Guild::HandleRoster(session)` | Send full roster: every member's level, class, race, zone, ranks, notes, last-online, achievement points | `SMSG_GUILD_ROSTER` |
| `Guild::SendBankList(session, tabId, fullUpdate)` | Send full or delta bank tab snapshot | `SMSG_GUILD_BANK_LIST` |
| `Guild::SendEventLog(session)` | 100-record ring | `SMSG_GUILD_EVENT_LOG_QUERY_RESULTS` |
| `Guild::SendBankLog(session, tabId)` | 25-record bank ring | `SMSG_GUILD_BANK_LOG_QUERY_RESULTS` |
| `Guild::SendNewsUpdate(session)` | 250-record news feed | `SMSG_GUILD_NEWS` |
| `Guild::SendPermissions(session)` | Per-rank rights snapshot | `SMSG_GUILD_PERMISSIONS_QUERY_RESULTS` |
| `Guild::SendQueryResponse(session)` | Public guild info: name, ranks, emblem | `SMSG_QUERY_GUILD_INFO_RESPONSE` |
| `Guild::SendLoginInfo(session)` | At login: MOTD + member roster snapshot + permissions + bank tab availability | multiple SMSG |
| `Guild::OnPlayerStatusChange(player, flag, state)` | On AFK/DND/login/logout — broadcast `SMSG_GUILD_EVENT_PRESENCE_CHANGE` to guild | `BroadcastWorker` |
| `Guild::BroadcastToGuild(session, officerOnly, msg, lang)` | Guild/officer chat dispatch; respects `GR_RIGHT_GCHATSPEAK`/`OFFCHATSPEAK` and `GCHATLISTEN`/`OFFCHATLISTEN` | per-member packet |
| `Guild::BroadcastAddonToGuild(...)` | Addon prefix-routed | per-member packet |
| `Guild::HandleGuildPartyRequest(session)` | Reports guild-party state for guild-XP-bonus mechanic | `SMSG_GUILD_PARTY_STATE_RESPONSE` |
| `Guild::MassInviteToEvent(session, minLevel, maxLevel, minRank)` | Calendar bulk-invite | `Calendar::AddInvite` per match |
| `Guild::Validate()` | Post-load sanity check (rank 0 exists, leader is rank 0, ranks 0..n contiguous, bank tabs match purchases) | drops invalid guild |
| `GuildMgr::LoadGuilds()` | Massive multi-pass startup load: guilds → ranks → members → bank tabs → bank items → bank rights → eventlogs → bankeventlogs → news | DB raw queries |
| `GuildMgr::LoadGuildRewards()` | Loads `guild_rewards` table | DB |
| `GuildMgr::ResetTimes(week)` | Daily/weekly reset (per-rank withdraw, weekly rep, weekly activity) | per-Guild reset |
| `WorldSession::HandleGuildQueryOpcode` ... `HandleGuildAddBattlenetFriend` (60 handlers) | Thin routers to `Guild::Handle*` methods | dispatch by `m_guildId` |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Player** — `Player::GetGuildId`, `SetInGuild`, `GetGuildIdInvited`, `GetReputation(Faction::GuildId)`; saved on character
- **Entities/Item** — bank stores `Item*` directly; uses `Item::SaveToDB`/`LoadFromDB` (extended item-instance schema with gems, transmog, …)
- **CharacterDatabase** — 15+ tables: `guild`, `guild_member`, `guild_rank`, `guild_bank_tab`, `guild_bank_item`, `guild_bank_right`, `guild_bank_eventlog`, `guild_eventlog`, `guild_member_withdraw`, `guild_newslog`, `guild_achievement`, `guild_achievement_progress`, `item_instance` (shared), `arena_team` (cross-ref via member account)
- **Globals/ObjectMgr** — `GetPlayerByLowGUID`, `PlayerNameMap` for offline-member roster
- **Globals/ObjectAccessor** — find online players for invite
- **Server/WorldSession** — packet dispatch, status check, account info
- **DBC/DB2** — `ChrRacesStore` (faction check on invite), `GuildPerksSpellStore` (level→perk), `GuildRewardStore`, `ItemTemplate`/`ItemSparse` (bank-item rights validation)
- **Calendar** — for `MassInviteToEvent`
- **AchievementMgr** — for tracking guild-achievement progress (separate `GuildAchievementMgr` instance)
- **GuildFinder** — `GuildFinderMgr` for application/recommendation matching
- **Mail** — disbanded-guild bank items mailed to members
- **PetitionMgr** — petition→guild conversion (signature gathering)
- **Chat / LanguageMgr** — `BroadcastToGuild` uses `LANG_UNIVERSAL` for guild/officer chat
- **Battle.net / WowAccount** — guild reputation lookups, recruit-a-friend bonuses

**Depended on by:**
- **Player.cpp** — `Player::GetGuildId`, `Player::ModifyMoney` (deposit/withdraw), saved with character
- **ChatHandler** — `CMSG_CHAT_MESSAGE_GUILD/OFFICER` routes via `Guild::BroadcastToGuild`
- **Calendar** — guild-event invites
- **AchievementMgr** — `CRITERIA_TYPE_HK_*` and similar guild-related criteria
- **PetitionMgr** — `Petition::ConvertToGuild`
- **Mail** — disband-cleanup
- **LFG / RaidFinder** — guild-group XP bonus detection
- **ArenaTeam** — guild membership snapshot for guild rosters in arena UI
- **GM commands** — `.guild create/info/disband/invite/uninvite/promote/demote/rank/rename`

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_INS_GUILD` | INSERT new guild row | character |
| `CHAR_DEL_GUILD` | DELETE guild on disband | character |
| `CHAR_UPD_GUILD_NAME` | Rename | character |
| `CHAR_UPD_GUILD_MOTD` | Set MOTD | character |
| `CHAR_UPD_GUILD_INFO` | Set info text | character |
| `CHAR_UPD_GUILD_LEADER` | Transfer leader | character |
| `CHAR_UPD_GUILD_EMBLEM_INFO` | Save tabard | character |
| `CHAR_UPD_GUILD_BANK_MONEY` | Bank-money balance | character |
| `CHAR_INS_GUILD_MEMBER` / `CHAR_DEL_GUILD_MEMBER` / `CHAR_DEL_GUILD_MEMBERS` | Member row mgmt | character |
| `CHAR_UPD_GUILD_MEMBER_RANK` / `_PNOTE` / `_OFFNOTE` | Member col updates | character |
| `CHAR_INS_GUILD_RANK` / `CHAR_DEL_GUILD_RANK(S)` / `CHAR_UPD_GUILD_RANK_ORDER`/`_NAME`/`_RIGHTS`/`_BANK_MONEY` | Rank lifecycle | character |
| `CHAR_INS_GUILD_BANK_TAB` / `_DEL_GUILD_BANK_TAB(S)` / `CHAR_UPD_GUILD_BANK_TAB_INFO` / `_TEXT` | Bank-tab mgmt | character |
| `CHAR_INS_GUILD_BANK_ITEM` / `_DEL_GUILD_BANK_ITEM(S)` / `CHAR_SEL_GUILD_BANK_ITEMS` / `_DEL_NONEXISTENT_GUILD_BANK_ITEM` | Bank-item mgmt | character |
| `CHAR_INS_GUILD_BANK_RIGHT` / `_DEL_GUILD_BANK_RIGHTS(_FOR_RANK)` | Per-rank-tab rights upsert | character |
| `CHAR_INS_GUILD_EVENTLOG` / `_DEL_GUILD_EVENTLOG(S)` | Event log ring | character |
| `CHAR_INS_GUILD_BANK_EVENTLOG` / `_DEL_GUILD_BANK_EVENTLOG(S)` | Bank log ring | character |
| `CHAR_INS_GUILD_MEMBER_WITHDRAW(_TABS\|_MONEY)` / `_DEL_GUILD_MEMBER_WITHDRAW` | Daily-reset tracking | character |
| `CHAR_INS_GUILD_NEWS` / `_DEL_GUILD_NEWS_*` | News feed | character |
| `CHAR_INS_GUILD_ACHIEVEMENT` / `_INS_GUILD_ACHIEVEMENT_CRITERIA` / DELs / SELs | Guild-achievements + progress | character |
| `CHAR_SEL_GUILD_MEMBER` / `_EXTENDED` | Cross-ref char→guild | character |

DBC/DB2 stores read:

| Store | What it loads | Read by |
|---|---|---|
| `GuildPerksSpellStore` (deprecated post-Cata) | Guild-level perk spells | `Guild::OnLogin` (auto-cast perks) |
| `GuildColorEmblemStore` / `GuildColorBackgroundStore` / `GuildColorBorderStore` | Tabard colour palette validation | `EmblemInfo::ValidateEmblemColors` |
| `ChrRacesStore` | Faction check on invite | `HandleInviteMember` |
| `ItemTemplate`/`ItemSparse` | Item-validity check at bank deposit | `MoveItemData::CanStore` |
| (world-table) `guild_rewards` | Loadable rewards | `GuildMgr::LoadGuildRewards` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_QUERY_GUILD_INFO` | C→S | `HandleGuildQueryOpcode` |
| `CMSG_GUILD_INVITE_BY_NAME` | C→S | `HandleGuildInviteByName` |
| `CMSG_ACCEPT_GUILD_INVITE` | C→S | `HandleGuildAcceptInvite` |
| `CMSG_GUILD_DECLINE_INVITATION` | C→S | `HandleGuildDeclineInvitation` |
| `CMSG_GUILD_OFFICER_REMOVE_MEMBER` | C→S | `HandleGuildOfficerRemoveMember` |
| `CMSG_GUILD_GET_ROSTER` | C→S | `HandleGuildGetRoster` |
| `CMSG_GUILD_PROMOTE_MEMBER` | C→S | `HandleGuildPromoteMember` |
| `CMSG_GUILD_DEMOTE_MEMBER` | C→S | `HandleGuildDemoteMember` |
| `CMSG_GUILD_ASSIGN_MEMBER_RANK` | C→S | `HandleGuildAssignRank` |
| `CMSG_GUILD_LEAVE` | C→S | `HandleGuildLeave` |
| `CMSG_GUILD_DELETE` | C→S | `HandleGuildDelete` |
| `CMSG_GUILD_UPDATE_MOTD_TEXT` | C→S | `HandleGuildUpdateMotdText` |
| `CMSG_GUILD_SET_MEMBER_NOTE` | C→S | `HandleGuildSetMemberNote` |
| `CMSG_GUILD_GET_RANKS` | C→S | `HandleGuildGetRanks` |
| `CMSG_GUILD_ADD_RANK` | C→S | `HandleGuildAddRank` |
| `CMSG_GUILD_DELETE_RANK` | C→S | `HandleGuildDeleteRank` |
| `CMSG_GUILD_SHIFT_RANK` | C→S | `HandleGuildShiftRank` |
| `CMSG_GUILD_UPDATE_INFO_TEXT` | C→S | `HandleGuildUpdateInfoText` |
| `CMSG_SAVE_GUILD_EMBLEM` | C→S | `HandleSaveGuildEmblem` |
| `CMSG_GUILD_EVENT_LOG_QUERY` | C→S | `HandleGuildEventLogQuery` |
| `CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY` | C→S | `HandleGuildBankMoneyWithdrawn` |
| `CMSG_GUILD_PERMISSIONS_QUERY` | C→S | `HandleGuildPermissionsQuery` |
| `CMSG_GUILD_BANK_ACTIVATE` | C→S | `HandleGuildBankActivate` |
| `CMSG_GUILD_BANK_QUERY_TAB` | C→S | `HandleGuildBankQueryTab` |
| `CMSG_GUILD_BANK_DEPOSIT_MONEY` | C→S | `HandleGuildBankDepositMoney` |
| `CMSG_GUILD_BANK_WITHDRAW_MONEY` | C→S | `HandleGuildBankWithdrawMoney` |
| `CMSG_AUTO_GUILD_BANK_ITEM` | C→S | `HandleAutoGuildBankItem` |
| `CMSG_STORE_GUILD_BANK_ITEM` | C→S | `HandleStoreGuildBankItem` |
| `CMSG_SWAP_ITEM_WITH_GUILD_BANK_ITEM` | C→S | `HandleSwapItemWithGuildBankItem` |
| `CMSG_SWAP_GUILD_BANK_ITEM_WITH_GUILD_BANK_ITEM` | C→S | `HandleSwapGuildBankItemWithGuildBankItem` |
| `CMSG_MOVE_GUILD_BANK_ITEM` | C→S | `HandleMoveGuildBankItem` |
| `CMSG_MERGE_ITEM_WITH_GUILD_BANK_ITEM` | C→S | `HandleMergeItemWithGuildBankItem` |
| `CMSG_SPLIT_ITEM_TO_GUILD_BANK` | C→S | `HandleSplitItemToGuildBank` |
| `CMSG_MERGE_GUILD_BANK_ITEM_WITH_ITEM` | C→S | `HandleMergeGuildBankItemWithItem` |
| `CMSG_SPLIT_GUILD_BANK_ITEM_TO_INVENTORY` | C→S | `HandleSplitGuildBankItemToInventory` |
| `CMSG_AUTO_STORE_GUILD_BANK_ITEM` | C→S | `HandleAutoStoreGuildBankItem` |
| `CMSG_MERGE_GUILD_BANK_ITEM_WITH_GUILD_BANK_ITEM` | C→S | `HandleMergeGuildBankItemWithGuildBankItem` |
| `CMSG_SPLIT_GUILD_BANK_ITEM` | C→S | `HandleSplitGuildBankItem` |
| `CMSG_GUILD_BANK_BUY_TAB` | C→S | `HandleGuildBankBuyTab` |
| `CMSG_GUILD_BANK_UPDATE_TAB` | C→S | `HandleGuildBankUpdateTab` |
| `CMSG_GUILD_BANK_LOG_QUERY` | C→S | `HandleGuildBankLogQuery` |
| `CMSG_GUILD_BANK_TEXT_QUERY` | C→S | `HandleGuildBankTextQuery` |
| `CMSG_GUILD_BANK_SET_TAB_TEXT` | C→S | `HandleGuildBankSetTabText` |
| `CMSG_GUILD_SET_RANK_PERMISSIONS` | C→S | `HandleGuildSetRankPermissions` |
| `CMSG_REQUEST_GUILD_PARTY_STATE` | C→S | `HandleGuildRequestPartyState` |
| `CMSG_GUILD_CHALLENGE_UPDATE_REQUEST` | C→S | `HandleGuildChallengeUpdateRequest` |
| `CMSG_DECLINE_GUILD_INVITES` | C→S | `HandleDeclineGuildInvites` |
| `CMSG_REQUEST_GUILD_REWARDS_LIST` | C→S | `HandleRequestGuildRewardsList` |
| `CMSG_GUILD_QUERY_NEWS` | C→S | `HandleGuildQueryNews` |
| `CMSG_GUILD_NEWS_UPDATE_STICKY` | C→S | `HandleGuildNewsUpdateSticky` |
| `CMSG_GUILD_REPLACE_GUILD_MASTER` | C→S | `HandleGuildReplaceGuildMaster` |
| `CMSG_GUILD_SET_GUILD_MASTER` | C→S | `HandleGuildSetGuildMaster` |
| `CMSG_GUILD_SET_ACHIEVEMENT_TRACKING` | C→S | `HandleGuildSetAchievementTracking` |
| `CMSG_GUILD_GET_ACHIEVEMENT_MEMBERS` | C→S | `HandleGuildGetAchievementMembers` |
| `CMSG_GUILD_AUTO_DECLINE_INVITATION` | C→S | `HandleGuildAutoDeclineInvitation` |
| `CMSG_GUILD_CHANGE_NAME_REQUEST` | C→S | `HandleGuildChangeNameRequest` |
| `CMSG_GUILD_QUERY_RECIPES` / `_MEMBER_RECIPES` / `_MEMBERS_FOR_RECIPE` | C→S | recipe lookup |
| `CMSG_GUILD_ADD_BATTLENET_FRIEND` | C→S | `HandleGuildAddBattlenetFriend` |
| `SMSG_QUERY_GUILD_INFO_RESPONSE` | S→C | `Guild::SendQueryResponse` |
| `SMSG_GUILD_ROSTER` | S→C | `Guild::HandleRoster` |
| `SMSG_GUILD_INVITE` | S→C | `HandleGuildInviteByName` (to target) |
| `SMSG_GUILD_COMMAND_RESULT` | S→C | All op results |
| `SMSG_GUILD_EVENT_*` family (~15 subtypes) | S→C | MOTD / TabardChanged / PlayerJoined / PlayerLeft / Promotion / Demotion / Disbanded / NewLeader / PresenceChange / RanksUpdated / BankMoneyChanged / Renamed | `Send*` methods |
| `SMSG_GUILD_PERMISSIONS_QUERY_RESULTS` | S→C | `Guild::SendPermissions` |
| `SMSG_GUILD_BANK_LIST` | S→C | `Guild::SendBankList` |
| `SMSG_GUILD_BANK_LOG_QUERY_RESULTS` | S→C | `Guild::SendBankLog` |
| `SMSG_GUILD_EVENT_LOG_QUERY_RESULTS` | S→C | `Guild::SendEventLog` |
| `SMSG_GUILD_NEWS` | S→C | `Guild::SendNewsUpdate` |
| `SMSG_GUILD_PARTY_STATE_RESPONSE` | S→C | `HandleGuildPartyRequest` |
| `SMSG_GUILD_REWARDS_LIST` | S→C | `HandleRequestGuildRewardsList` |
| `SMSG_GUILD_RANKS_UPDATE` | S→C | After AddRank/RemoveRank/Shift |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-guild` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/handlers/guild.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-packet/src/packets/guild.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- ❌ **No `crates/wow-guild/`** — crate not created.
- ❌ **No `crates/wow-world/src/handlers/guild.rs`** — handler not created.
- ❌ **No `crates/wow-packet/src/packets/guild.rs`** — packet definitions not created.
- ❌ **No DB schema** for `guild`, `guild_member`, `guild_rank`, `guild_bank_*`, `guild_*_eventlog`, `guild_newslog`, `guild_achievement*` tables in `schemas/character/`.
- Opcode names are pre-registered in `crates/wow-constants/src/opcodes.rs` (CMSG_GUILD_* family), but they have NO handler and arrive at the dispatcher → "unhandled opcode" warning.

**What's implemented:**
- Nothing. **0% complete.** Every CMSG_GUILD_* opcode is silently dropped (or logs an "unhandled" warning).

**What's missing vs C++:**
- **Everything.** Specifically:
  - `Guild` struct + `Member` + `RankInfo` + `BankTab` + `EmblemInfo` + log entries.
  - `GuildMgr` global.
  - Guild creation flow (currently no `.guild create` GM command, no petition→guild path).
  - All 60 `CMSG_GUILD_*` handlers.
  - All `SMSG_GUILD_*` packet builders.
  - 22 rank-rights bits.
  - 8-tab × 98-slot bank.
  - Bank money 100B copper cap.
  - Rank-cost table (100g, 250g, 500g, ...).
  - 2-10 rank dynamic add/remove/reorder.
  - Guild-bank item move (bank↔bank, bank↔inventory, splits, merges, swaps, auto-store).
  - Per-rank `bankMoneyPerDay` + `bankTabSlotsPerDay[8]` daily reset.
  - Event log (100 entries), bank log (25/tab × 9), news (250 entries).
  - Member online/AFK/DND/mobile flag broadcast.
  - Guild chat / officer chat routing (currently `CMSG_CHAT_MESSAGE_GUILD/OFFICER` is mis-routed to proximity broadcast — see `chat.md`).
  - Guild emblem + tabard cost.
  - GuildAchievementMgr (per-guild achievements).
  - Guild reputation (faction guild, weekly/total cap).
  - Guild perks (auto-cast on guild-level-up).
  - Guild rewards list.
  - GuildFinder applications.
  - MassInviteToEvent (calendar integration).
  - DB persistence (15 tables).
  - Disband-cleanup (mail bank items to members on disband).
  - 90-day inactive-leader auto-replace.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — nothing implemented. The "divergence" is total absence.
- Opcodes are pre-allocated in `wow-constants` so dispatcher will error-log when clients send guild ops. Verify they don't crash the session.
- `CMSG_CHAT_MESSAGE_GUILD/OFFICER` currently delivered via proximity broadcast (see chat.md issue) — once Guild module exists, must be re-routed.

**Tests existing:**
- 0 tests. Nothing to test yet.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#GUILDS.WBS.001** Partir y cerrar la migracion auditada de `game/Guilds/Guild.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`
  Rust target: `crates/wow-guild`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3656 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GUILDS.WBS.002** Partir y cerrar la migracion auditada de `game/Guilds/Guild.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`
  Rust target: `crates/wow-guild`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 956 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GUILDS.WBS.003** Partir y cerrar la migracion auditada de `game/Guilds/GuildMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp`
  Rust target: `crates/wow-guild`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 565 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#GUILDS.WBS.004** Cerrar la migracion auditada de `game/Guilds/GuildMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.h`
  Rust target: `crates/wow-guild`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#GUILDS.1** Create new crate `crates/wow-guild` with `Cargo.toml`, register in workspace. Complejidad: **L**
- [ ] **#GUILDS.2** Define enums in `wow-constants`: `GuildRankRights` (u32 bitflags!), `GuildBankRights` (u8 bitflags!), `GuildCommandType` (u8 enum), `GuildCommandError` (u8 enum, ~30 variants), `GuildEventLogTypes`, `GuildBankEventLogTypes`, `GuildEmblemError`, `GuildMemberFlags`, `GuildNews`, `GuildRankId(u8)`, `GuildRankOrder(u8)`. Complejidad: **M**
- [ ] **#GUILDS.3** Define `GuildMisc` constants (`BANK_MAX_TABS=8`, `BANK_MAX_SLOTS=98`, `RANKS_MIN=2`/`MAX=10`, `RANK_NONE=0xFF`, `BANK_MONEY_LIMIT=100_000_000_000`). Complejidad: **L**
- [ ] **#GUILDS.4** Define struct `Guild` with id, name, leader_guid, motd, info, created_date, bank_money, members: HashMap<ObjectGuid, Member>, ranks: Vec<RankInfo>, bank_tabs: [Option<BankTab>; 8], emblem: EmblemInfo. Complejidad: **M**
- [ ] **#GUILDS.5** Define struct `Guild::Member` (mirror C++ exactly). Complejidad: **M**
- [ ] **#GUILDS.6** Define struct `Guild::RankInfo` with id, order, name, rights, bank_money_per_day, bank_tab_rights_and_slots: [GuildBankRightsAndSlots; 8]. Complejidad: **M**
- [ ] **#GUILDS.7** Define struct `Guild::BankTab` (id, name, icon, text, items: [Option<Item>; 98]). Complejidad: **M**
- [ ] **#GUILDS.8** Define struct `EmblemInfo` (5 u32) + `validate_emblem_colors()` against DBC. Complejidad: **L**
- [ ] **#GUILDS.9** Define `LogEntry`, `EventLogEntry`, `BankEventLogEntry`, `NewsLogEntry`, generic `LogHolder<T>(VecDeque, max_records)` with append+evict. Complejidad: **M**
- [ ] **#GUILDS.10** Define struct `GuildMgr` singleton with `DashMap<u32, Arc<RwLock<Guild>>>` + by-name and by-leader indexes; ID generator. Complejidad: **M**
- [ ] **#GUILDS.11** Add character-DB schema migrations for all 15 guild tables. Complejidad: **H**
- [ ] **#GUILDS.12** Implement `GuildMgr::load_guilds()` — multi-pass: guild → ranks → members → bank_tabs → bank_items → bank_rights → eventlog → bankeventlog → news. Complejidad: **XL**
- [ ] **#GUILDS.13** Implement `Guild::create(leader, name)` — allocate id, create 5 default ranks (GuildMaster, Officer, Veteran, Member, Initiate), insert leader, persist, broadcast `EVENT_PLAYER_JOINED`. Complejidad: **H**
- [ ] **#GUILDS.14** Implement `Guild::disband()` — DELETE all DB rows in transaction, mail bank-items to members (special "guild bank disbanded" sender), broadcast `EVENT_DISBANDED`. Complejidad: **H**
- [ ] **#GUILDS.15** Create `crates/wow-packet/src/packets/guild.rs` with `QueryGuildInfoResponse`, `GuildRoster`, `GuildInvite`, `GuildCommandResult`, `GuildEvent*` family (~15 SMSG variants), `GuildPermissionsQueryResults`, `GuildBankList`, `GuildBankLogQueryResults`, `GuildEventLogQueryResults`, `GuildNews`, `GuildRanksUpdate`, `GuildPartyStateResponse`, `GuildRewardsList`. Complejidad: **XL**
- [ ] **#GUILDS.16** Create `crates/wow-world/src/handlers/guild.rs` with all 60 CMSG handlers. Complejidad: **XL** (split — see #GUILDS.17 onwards)
- [ ] **#GUILDS.17** Implement `CMSG_QUERY_GUILD_INFO` + `SMSG_QUERY_GUILD_INFO_RESPONSE`. Complejidad: **L**
- [ ] **#GUILDS.18** Implement `CMSG_GUILD_INVITE_BY_NAME` → SMSG_GUILD_INVITE; pending-invite tracking (`guild_id_invited` field on session). Complejidad: **M**
- [ ] **#GUILDS.19** Implement `CMSG_ACCEPT_GUILD_INVITE` / `CMSG_GUILD_DECLINE_INVITATION`. Complejidad: **M**
- [ ] **#GUILDS.20** Implement `CMSG_GUILD_LEAVE` (incl. leader-with-other-members rejection). Complejidad: **M**
- [ ] **#GUILDS.21** Implement `CMSG_GUILD_DELETE` (last-member-or-GM disband flow). Complejidad: **M**
- [ ] **#GUILDS.22** Implement `CMSG_GUILD_OFFICER_REMOVE_MEMBER` + rights check. Complejidad: **M**
- [ ] **#GUILDS.23** Implement `CMSG_GUILD_PROMOTE_MEMBER` / `_DEMOTE_MEMBER` / `_ASSIGN_MEMBER_RANK`. Complejidad: **M**
- [ ] **#GUILDS.24** Implement `CMSG_GUILD_GET_ROSTER` → `SMSG_GUILD_ROSTER` (full per-member serialisation). Complejidad: **H**
- [ ] **#GUILDS.25** Implement `CMSG_GUILD_UPDATE_MOTD_TEXT` + broadcast `SMSG_GUILD_EVENT_MOTD`. Complejidad: **L**
- [ ] **#GUILDS.26** Implement `CMSG_GUILD_UPDATE_INFO_TEXT`. Complejidad: **L**
- [ ] **#GUILDS.27** Implement `CMSG_GUILD_SET_MEMBER_NOTE` (public + officer flag). Complejidad: **L**
- [ ] **#GUILDS.28** Implement `CMSG_GUILD_GET_RANKS` / `_ADD_RANK` / `_DELETE_RANK` / `_SHIFT_RANK` / `_SET_RANK_PERMISSIONS`. Complejidad: **H**
- [ ] **#GUILDS.29** Implement `CMSG_SAVE_GUILD_EMBLEM` + tabard-cost (configurable) + DBC color validation. Complejidad: **M**
- [ ] **#GUILDS.30** Implement `CMSG_GUILD_PERMISSIONS_QUERY` → `SMSG_GUILD_PERMISSIONS_QUERY_RESULTS`. Complejidad: **M**
- [ ] **#GUILDS.31** Implement `CMSG_GUILD_BANK_ACTIVATE` (open bank UI) + `_QUERY_TAB` → `SMSG_GUILD_BANK_LIST`. Complejidad: **H**
- [ ] **#GUILDS.32** Implement `CMSG_GUILD_BANK_BUY_TAB` (8-step cost ladder, debit money). Complejidad: **M**
- [ ] **#GUILDS.33** Implement `CMSG_GUILD_BANK_UPDATE_TAB` (set name+icon) and `CMSG_GUILD_BANK_SET_TAB_TEXT`. Complejidad: **L**
- [ ] **#GUILDS.34** Implement `CMSG_GUILD_BANK_DEPOSIT_MONEY` / `_WITHDRAW_MONEY` + per-rank daily cap + log appender. Complejidad: **H**
- [ ] **#GUILDS.35** Implement bank-item-move handlers: `CMSG_AUTO_GUILD_BANK_ITEM`, `_STORE_GUILD_BANK_ITEM`, `_SWAP_ITEM_WITH_GUILD_BANK_ITEM`, `_SWAP_GUILD_BANK_ITEM_WITH_GUILD_BANK_ITEM`, `_MOVE_GUILD_BANK_ITEM`, `_MERGE_*`, `_SPLIT_*`, `_AUTO_STORE_GUILD_BANK_ITEM`. Complejidad: **XL** (port `MoveItemData`/`PlayerMoveItemData`/`BankMoveItemData` triad)
- [ ] **#GUILDS.36** Implement `CMSG_GUILD_BANK_LOG_QUERY` → `SMSG_GUILD_BANK_LOG_QUERY_RESULTS` (per-tab + money-tab=100). Complejidad: **M**
- [ ] **#GUILDS.37** Implement `CMSG_GUILD_BANK_TEXT_QUERY` (per-tab freeform text). Complejidad: **L**
- [ ] **#GUILDS.38** Implement `CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY`. Complejidad: **L**
- [ ] **#GUILDS.39** Implement `CMSG_GUILD_EVENT_LOG_QUERY`. Complejidad: **M**
- [ ] **#GUILDS.40** Implement `CMSG_GUILD_QUERY_NEWS` / `_NEWS_UPDATE_STICKY`. Complejidad: **M**
- [ ] **#GUILDS.41** Implement `CMSG_GUILD_REPLACE_GUILD_MASTER` / `_SET_GUILD_MASTER` (90-day inactive rule). Complejidad: **M**
- [ ] **#GUILDS.42** Implement `CMSG_REQUEST_GUILD_PARTY_STATE` (XP-bonus eligibility). Complejidad: **L**
- [ ] **#GUILDS.43** Implement `CMSG_GUILD_CHALLENGE_UPDATE_REQUEST`. Complejidad: **M**
- [ ] **#GUILDS.44** Implement `CMSG_DECLINE_GUILD_INVITES` (auto-decline toggle). Complejidad: **L**
- [ ] **#GUILDS.45** Implement `CMSG_REQUEST_GUILD_REWARDS_LIST`. Complejidad: **L**
- [ ] **#GUILDS.46** Implement `CMSG_GUILD_SET_ACHIEVEMENT_TRACKING` / `_GET_ACHIEVEMENT_MEMBERS`. Complejidad: **M**
- [ ] **#GUILDS.47** Implement `CMSG_GUILD_AUTO_DECLINE_INVITATION`. Complejidad: **L**
- [ ] **#GUILDS.48** Implement `CMSG_GUILD_CHANGE_NAME_REQUEST` (paid rename). Complejidad: **M**
- [ ] **#GUILDS.49** Implement `CMSG_GUILD_QUERY_RECIPES` / `_QUERY_MEMBER_RECIPES` / `_QUERY_MEMBERS_FOR_RECIPE`. Complejidad: **H**
- [ ] **#GUILDS.50** Wire `CMSG_CHAT_MESSAGE_GUILD/OFFICER` to `Guild::broadcast_to_guild` (currently mis-routed to proximity in chat.rs — see chat.md). Complejidad: **M**
- [ ] **#GUILDS.51** Implement member presence broadcast — `OnPlayerLogin/Logout/AFK/DND/Zone` → `SMSG_GUILD_EVENT_PRESENCE_CHANGE`. Complejidad: **M**
- [ ] **#GUILDS.52** Implement daily reset (`reset_times(false)`) — clear `guild_member_withdraw` rows. Complejidad: **L**
- [ ] **#GUILDS.53** Implement weekly reset (`reset_times(true)`) — clear week activity/reputation. Complejidad: **L**
- [ ] **#GUILDS.54** Wire petition→guild conversion (signature-collected petition becomes a Guild). Complejidad: **H** (depends on petition module)
- [ ] **#GUILDS.55** Persist + restore guild-id on character (`Player.guild_id`) save/load. Complejidad: **M**
- [ ] **#GUILDS.56** Implement disband-cleanup mail (bank items mailed to members). Complejidad: **M**
- [ ] **#GUILDS.57** Implement `GuildAchievementMgr` (separate from player achievement mgr). Complejidad: **XL** (depends on achievements module)
- [ ] **#GUILDS.58** Implement guild-finder (`GuildFinderMgr`) applications. Complejidad: **XL** (separate sub-system)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#GUILDS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 5248 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp`. Rust target: `crates/wow-constants`. | `cargo test -p wow-constants` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#GUILDS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 5248 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp`. Rust target: `crates/wow-constants`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#GUILDS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 5248 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp`. Rust target: `crates/wow-constants`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#GUILDS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 5248 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp`. Rust target: `crates/wow-constants`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Guild::create` allocates default 5 ranks with names "Guild Master", "Officer", "Veteran", "Member", "Initiate".
- [ ] Test: GuildMaster cannot leave with members remaining → `ERR_GUILD_LEADER_LEAVE`.
- [ ] Test: Last member leaving → guild auto-disbands.
- [ ] Test: 11th rank addition → `ERR_GUILD_RANKS_LOCKED` (max=10).
- [ ] Test: Removing a rank that has members → `ERR_GUILD_RANK_IN_USE`.
- [ ] Test: Rank count cannot drop below 2.
- [ ] Test: Promote requires `GR_RIGHT_PROMOTE` AND target's rank > promoter's rank+1.
- [ ] Test: Demote cannot demote to rank ≤ demoter's own.
- [ ] Test: 8th bank tab purchase costs exactly 25000g.
- [ ] Test: Daily withdraw cap resets at server-day boundary.
- [ ] Test: Withdrawing past `bankMoneyPerDay` cap → `ERR_GUILD_WITHDRAW_LIMIT`.
- [ ] Test: Bank-money cap enforced at 100,000,000,000 copper.
- [ ] Test: Disbanded guild's bank items appear in members' mailboxes within 24h.
- [ ] Test: Inactive-90-days GM → any member can `CMSG_GUILD_REPLACE_GUILD_MASTER`.
- [ ] Test: Guild emblem with invalid color combo → `ERR_GUILDEMBLEM_INVALID_TABARD_COLORS`.
- [ ] Test: Guild chat respects `GR_RIGHT_GCHATSPEAK`/`GCHATLISTEN` per rank.
- [ ] Test: Member with `WITHDRAW_GOLD_LOCK` cannot withdraw money even if gold/day cap allows.
- [ ] Test: Bank-item swap rolls back on failure (atomic transaction).
- [ ] Test: Event log evicts oldest entry past 100 records.
- [ ] Test: News log evicts oldest entry past 250 records.
- [ ] Test: Bank log evicts oldest past 25 records per tab.
- [ ] Test: Roster shows offline members with their last-online timestamp.
- [ ] Test: Renaming guild updates `guild.name` AND character cache.
- [ ] Test: Cross-faction invite → `ERR_GUILD_NOT_ALLIED`.
- [ ] Test: Inviting a player on someone's ignore list → blocked silently or `ERR_GUILD_IGNORING_YOU_S`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 5248 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp` | **NONE — entire system missing.** Should be `crates/wow-guild/` (not yet created), `crates/wow-world/src/handlers/guild.rs` (not yet created), `crates/wow-packet/src/packets/guild.rs` (not yet created). \| ❌ not started (0% — no crate, no handler, no packet definitions, no DB schema in Rust) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#GUILDS.DIV.001` | `crates/wow-guild` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 5248 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#GUILDS.DIV.002` | `crates/wow-world/src/handlers/guild.rs` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 5248 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#GUILDS.DIV.003` | `crates/wow-packet/src/packets/guild.rs` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 5248 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/Guild.h`, `/home/server/woltk-trinity-legacy/src/server/game/Guilds/GuildMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`Guild` has `HighGuid::Guild`** in 3.4.3 — when sending GUIDs over wire, build via `ObjectGuid::Create<HighGuid::Guild>(m_id)`. Wrong HighGuid silently breaks client UI.
- **Rank 0 IS ALWAYS the Guild Master.** Rank id and rank order are SEPARATE concepts: rankId is the stable database key, rankOrder is the display position. Reordering ranks changes order, NOT id. Bank rights are keyed by `(rankId, tabId)` so reorder is cheap.
- **`GUILD_RANK_NONE = 0xFF`** is the "not in guild" sentinel; do not store in DB.
- **`GUILD_WITHDRAW_MONEY_UNLIMITED = 0xFFFFFFFF`** — for GuildMaster (override `bankMoneyPerDay`).
- **`GUILD_BANK_MONEY_LIMIT = 100,000,000,000`** copper = 10,000,000 gold = 10M gold cap.
- **8 bank tabs cost** (cumulative): 100g, 250g, 500g, 1000g, 2500g, 5000g, 10000g, 25000g.
- **`MinNewsItemLevel = 353`** — items at ilvl < 353 are not posted to news (else news spam).
- **MOTD/info text length** — MOTD ~128 chars, info text ~512 chars; client-side limits.
- **`HandleSetNewGuildMaster` requires** EITHER (a) requester IS current GM (voluntary transfer), OR (b) current GM offline ≥ 90 days (`GUILD_MASTER_DETHRONE_INACTIVE_DAYS`) AND requester has `GR_RIGHT_PROMOTE`.
- **`GR_RIGHT_ALL = 0x00DDFFBF`** — bits not in this mask are ignored (client/server divergence here is a common source of "phantom rights" bugs).
- **`HandleMemberLogout`** is critical — must update `m_logoutTime` so the 90-day GM-replace works; if you forget, GMs are never auto-replaceable.
- **Bank items persistence** — items live in `item_instance` (shared pool) and `guild_bank_item` row points at them by item_guid. Don't double-delete — `CHAR_DEL_GUILD_BANK_ITEMS` only removes the binding rows, NOT the items; items are separately deleted via `Item::DeleteFromDB` if guild disbands.
- **Daily reset semantics** — Trinity uses `WORLD_CONFIG_GUILD_BANK_DAILY_RESET_HOUR` (default 6 AM server time) to reset both money and tab withdraw counters.
- **Guild perks** — Cataclysm-era; in 3.4.3 vestigial. `GuildPerksSpellStore` was deprecated. Implement only if Cata content enabled.
- **Guild reputation** — players can earn rep with their own guild faction (faction id from `guild.faction`). Weekly cap (3500 by default) + total cap (42999).
- **Mass invite to event** uses CalendarMgr — depends on calendar module which is in Batch 5; chain dependency.
- **Bank slot count** is 98 (not 100) — the 99th and 100th slots are reserved for UI metadata. Don't bump to 100.
- **`m_recentInstances`** in Group is unrelated; ignore here.
- **GuildFinder applications** are cross-faction-blocked and respect ignore lists.
- **Petition→Guild conversion**: 4 signatures (Alliance) or 4 (Horde) on a Guild Charter petition unlock `Guild::Create`. Petition module owns the signature flow.
- **Guild challenges** (Cata feature) — 6 types, weekly caps, gold rewards. `GuildChallengeGoldReward[]` and `GuildChallengesMaxCount[]` are constants in `Guild.h`.
- **Achievement tracking** — `GuildAchievementMgr` is per-guild, NOT per-member. Track per-member contribution via `m_trackedCriteriaIds`.
- **`Guild::Validate()`** at load — drops malformed guilds (no rank 0, leader not in members, etc.) — log loudly.
- **Disband during active session** — kicks all online members back to `Player::SetInGuild(0)` and sends `SMSG_GUILD_EVENT_DISBANDED`; their pending guild-bank UI may be stale for a tick.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Guild` | `struct Guild` (en `crates/wow-guild/src/guild.rs` — TBD) | behind `Arc<RwLock<Guild>>` in `GuildMgr` |
| `class Guild::Member` | `struct Member` (nested module) | mirror exactly |
| `class Guild::RankInfo` | `struct RankInfo { id: GuildRankId, order: GuildRankOrder, name: String, rights: GuildRankRights, bank_money_per_day: u32, bank_tab: [GuildBankRightsAndSlots; 8] }` | array, not Vec |
| `class Guild::BankTab` | `struct BankTab { id: u8, name: String, icon: String, text: String, items: [Option<Item>; 98] }` | |
| `class EmblemInfo` | `struct EmblemInfo { style: u32, color: u32, border_style: u32, border_color: u32, background_color: u32 }` | |
| `class GuildBankRightsAndSlots` | `struct GuildBankRightsAndSlots { tab_id: u8, rights: GuildBankRights, slots_per_day: i32 }` | i32 because -1 = unlimited |
| `class Guild::LogHolder<T>` | `struct LogHolder<T> { entries: VecDeque<T>, max_records: u32, next_guid: u32 }` | bounded ring |
| `class Guild::EventLogEntry` / `BankEventLogEntry` / `NewsLogEntry` | `enum LogEntry { Event(EventLogEntry), Bank(BankEventLogEntry), News(NewsLogEntry) }` or 3 separate types | 3 separate is closer to C++ |
| `class GuildMgr` (singleton) | `struct GuildMgr { by_id: DashMap<u32, Arc<RwLock<Guild>>>, by_name: DashMap<String, u32>, by_leader: DashMap<ObjectGuid, u32>, next_id: AtomicU32 }` | `Arc<GuildMgr>` in `WorldContext` |
| `enum GuildRankRights : uint32` | `bitflags! { struct GuildRankRights: u32 { ... } }` | 22 bits |
| `enum GuildBankRights` | `bitflags! { struct GuildBankRights: u8 { VIEW_TAB=1; PUT_ITEM=2; UPDATE_TEXT=4; FULL=0xFF; } }` | |
| `enum GuildCommandError` | `#[repr(u8)] enum GuildCommandError { Success=0, Internal=1, ... }` | ~30 variants |
| `enum class GuildRankId : uint8` | `#[repr(transparent)] struct GuildRankId(u8);` | strongly-typed wrapper |
| `enum class GuildRankOrder : uint8` | `#[repr(transparent)] struct GuildRankOrder(u8);` | |
| `class MoveItemData` (abstract) | `trait MoveItemSource` + `struct PlayerSource` / `struct BankSource` | dynamic dispatch via trait objects OR enum dispatch |
| `Guild::HandleInviteMember(session, name)` | `async fn handle_invite_member(&mut self, session: &mut WorldSession, name: &str) -> Result<()>` | session held mutably |
| `Guild::SaveToDB()` | `async fn save_to_db(&self, db: &Pool<MySql>) -> Result<()>` | giant transaction |
| `Guild::Disband()` | `async fn disband(&self, db: &Pool<MySql>) -> Result<()>` | DELETE cascade |
| `Guild::BroadcastToGuild(...)` | `async fn broadcast_to_guild(&self, sender: ObjectGuid, officer_only: bool, msg: &str, lang: Language)` | iterates members, send via PlayerRegistry |
| `WorldSession::HandleGuild*Opcode(packet)` | `async fn handle_guild_*(&mut self, pkt: WorldPacket)` (en `crates/wow-world/src/handlers/guild.rs`) | inventory-registered |
| `WorldPackets::Guild::QueryGuildInfo` | `struct QueryGuildInfo` (en `crates/wow-packet/src/packets/guild.rs`) | mirror C++ field-for-field |
| `GUILD_BANK_MAX_TABS = 8` | `pub const BANK_MAX_TABS: usize = 8;` | |
| `GUILD_RANKS_MAX_COUNT = 10` | `pub const RANKS_MAX_COUNT: usize = 10;` | |

---

*Template version: 1.0 (2026-05-01).* Status: ❌ NOT STARTED — 0% implemented. Largest single missing module in the L6 batch (~10000 C++ lines, 60 opcodes, 15 DB tables).

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — 0% implemented.** The pre-audit "0%" estimate is exact. No `Guild` type, no `GuildMgr`, no DB schema, no handler module, no packet builders.

**Inventory verified:**
- No `crates/wow-guild/` directory exists (`find` returned empty).
- No `crates/wow-world/src/handlers/guild.rs` (handler file list: `battlenet, character, chat, combat, group, inspect, loot, misc, mod, movement, quest, social, spell, trainer`).
- No `crates/wow-packet/src/packets/guild.rs`. Guild references in `wow-packet` are incidental: `mail.rs` mentions "guild bank" once, `chat.rs` defines the `ChatMsg::Guild` enum value, `update.rs` carries `guild_guid: ObjectGuid::EMPTY`, `inspect.rs` carries `guild_club_member_id: 0`. None of these constitute guild module code.
- No guild SQL statements in `crates/wow-database/src/statements/character.rs` (no `INS_GUILD*`, `SEL_GUILD*`, `DEL_GUILD*`).

**Opcode counting (refining the doc's "~70 GuildHandler functions" claim):**
- `wow-constants/src/opcodes.rs` defines **89 client opcodes whose name starts with `Guild`** plus several more that conceptually belong (e.g. `AcceptGuildInvite`, `DeclineGuildInvites`, `ChatMessageGuild`, `AutoGuildBankItem`, `AutoStoreGuildBankItem`, `SaveGuildEmblem`, `RequestGuildPartyState`, `RequestGuildRewardsList`, `MoveGuildBankItem`, `MergeGuildBankItem*`, `SplitGuildBankItem*`, `SwapItemWithGuildBankItem*`, `StoreGuildBankItem`). Total guild-relevant CMSG surface is **~100 opcodes**, not 60-70 — the doc's count is conservative. Of those, **only 2 stub handlers** are wired (verified): `handle_guild_set_achievement_tracking` and `handle_guild_bank_remaining_withdraw_money_query` in `handlers/misc.rs:595, 605` — both are empty `pub async fn ... (_pkt) {}` bodies that consume the packet and return without sending anything.
- All other ~98 guild CMSG opcodes are dispatched through the session match in `session.rs` and either fall through to "unhandled opcode" warning or are silently dropped. Verified by absence of `inventory::submit!` for any other guild opcode.

**Confirmed bug from doc §8:**
- `CMSG_CHAT_MESSAGE_GUILD/OFFICER` is wired (`session.rs:1724-1725`) but routes into `handle_chat_message(pkt, ChatMsg::Guild)` which falls into the generic chat dispatcher; without a `Guild` to broadcast through, the message is lost or mis-routed (proximity broadcast). Confirmed mis-route per doc claim — fix needed alongside guild module bring-up.

**Largest missing surfaces (confirmed):**
- Entire `Guild` / `Member` / `RankInfo` / `BankTab` / `EmblemInfo` / `LogHolder` type hierarchy.
- All 15 character-DB tables (`guild`, `guild_member`, `guild_rank`, `guild_bank_tab`, `guild_bank_item`, `guild_bank_right`, `guild_bank_eventlog`, `guild_eventlog`, `guild_member_withdraw`, `guild_newslog`, `guild_achievement`, `guild_achievement_progress`, plus `arena_team` cross-ref).
- 8-tab × 98-slot guild bank with item-instance-level move/swap/split/merge logic (the C++ `MoveItemData` triad).
- Guild chat / officer chat broadcast.
- Member presence broadcasts.
- Petition→guild conversion (signature flow, depends on petition module which is also absent).
- Disband-cleanup mail (depends on missing mails module — see mails.md).

**Estimate:** ~10,000 lines of C++ (per doc §2 totals: 956 + 3656 + 71 + 565 + ~150 + ~600 + 813 + ~3000 + ~70 = 9,881) → **largest single missing module in the L6 batch** as the doc footer states. Validated.
