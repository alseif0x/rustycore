# Migration: Calendar

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Calendar/` + `src/server/game/Handlers/CalendarHandler.cpp`
> **Rust target crate(s):** `crates/wow-world/` (handlers, CalendarMgr global), `crates/wow-database/` (calendar prepared statements), `crates/wow-packet/` (CalendarPackets), `crates/wow-core/` (calendar enums)
> **Layer:** L7
> **Status:** ⚠️ stub-only (2 noop handlers, audit confirmed)
> **Audited vs C++:** ✅ complete
> **Audited vs Rust impl:** ✅ 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

In-game calendar system: players (and guilds) create events with date/time, type (raid/dungeon/pvp/meeting/heroic/other), texture, title, description, optional lock date and a flags bitmask (guild event, invites locked, all-allowed, without-invites/announcement). Owners invite individual characters or whole-guild and assign moderator/owner ranks, players RSVP (invited/accepted/declined/confirmed/out/standby/signed-up/tentative), and the system mails a notice when an event or invite is removed. The mgr also pushes raid lockout add/remove/update notifications driven by `InstanceLockMgr`. Hard caps: 30 personal events, 100 guild events, 100 invites per event, 5s create-cooldown, 30-day TTL on past events.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Calendar/CalendarMgr.cpp` | 768 | `prefix` |
| `game/Calendar/CalendarMgr.h` | 372 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Calendar/CalendarMgr.h` | 372 | Singleton mgr declaration; `CalendarEvent`, `CalendarInvite` PODs; enums (`CalendarEventType`, `CalendarRepeatType`, `CalendarInviteStatus`, `CalendarError`, `CalendarFlags`, `CalendarModerationRank`, `CalendarSendEventType`, `CalendarLimits`); `CALENDAR_DEFAULT_RESPONSE_TIME = 946684800` (Y2K epoch sentinel) |
| `src/server/game/Calendar/CalendarMgr.cpp` | 768 | Implementation: `LoadFromDB`, free-id deque allocator, AddEvent / RemoveEvent / UpdateEvent, AddInvite / UpdateInvite / RemoveInvite, all `SendCalendar*` builders, `BuildCalendarMailSubject` / `BuildCalendarMailBody`, `RemoveAllPlayerEventsAndInvites`, `RemovePlayerGuildEventsAndSignups`, `DeleteOldEvents`, `GetPlayerNumPending`, `SendPacketToAllEventRelatives` |
| `src/server/game/Handlers/CalendarHandler.cpp` | 575 | All CMSG handlers (Get, GetEvent, AddEvent, UpdateEvent, RemoveEvent, CopyEvent, Invite, EventSignup, RSVP, RemoveInvite, Status, ModeratorStatus, Complain, GetNumPending, CommunityInvite, SetSavedInstanceExtend) and helpers `SendCalendarRaidLockout(Added/Removed/Updated)` |
| `src/server/game/Server/Packets/CalendarPackets.h` / `.cpp` | ~600 | Wire-format Read/Write for CMSG_CALENDAR_* and SMSG_CALENDAR_* (CalendarAddEvent, CalendarUpdateEvent, CalendarInviteRequest, CalendarRSVP, CalendarRemoveInvite, CalendarStatus, CalendarComplain, CalendarSendCalendar, CalendarSendEvent, CalendarInviteAdded, CalendarInviteAlert, CalendarInviteStatus, CalendarRaidLockoutAdded, CalendarRaidLockoutRemoved, CalendarRaidLockoutUpdated, CalendarCommandResult, CalendarSendNumPending, …) |
| `src/server/game/Entities/Player/Player.cpp` (PlayerStorage) | n/a | `Player::SendRaidInfo`, `Player::SendCalendarRaidLockout`, calls into `sCalendarMgr->RemoveAllPlayerEventsAndInvites` on character delete |
| `src/server/game/Instances/InstanceLockMgr.cpp` | n/a | Calls `sCalendarMgr->SendCalendarRaidLockoutUpdated` whenever an `InstanceLock` extend or expiry changes |
| `sql/updates/world/3.3.5/...calendar...sql` | n/a | Holiday/event seeded data — mostly DBC-driven (`Holidays.dbc`) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CalendarMgr` | class (singleton) | Owns global `_events: std::set<CalendarEvent*>` and `_invites: std::map<eventId, vector<CalendarInvite*>>`, free-id deques `_freeEventIds` / `_freeInviteIds`, monotonic counters `_maxEventId` / `_maxInviteId` |
| `CalendarEvent` | struct (heap, non-copyable) | `_eventId`, `_ownerGUID`, `_eventGuildId`, `_eventType`, `_textureId`, `_date`, `_flags`, `_title`, `_description`, `_lockDate`. Helpers `IsGuildEvent`, `IsGuildAnnouncement`, `IsLocked`, `BuildCalendarMailSubject`, `BuildCalendarMailBody` |
| `CalendarInvite` | struct (heap, non-copyable) | `_inviteId`, `_eventId`, `_invitee`, `_senderGUID`, `_responseTime`, `_status`, `_rank`, `_note` |
| `CalendarEventType` | enum | `RAID=0, DUNGEON=1, PVP=2, MEETING=3, OTHER=4, HEROIC=5` |
| `CalendarRepeatType` | enum | `NEVER=0, WEEKLY=1, BIWEEKLY=2, MONTHLY=3` (sent by client; server ignores in 3.3.5 — no actual recurrence engine) |
| `CalendarInviteStatus` | enum | `INVITED=0, ACCEPTED=1, DECLINED=2, CONFIRMED=3, OUT=4, STANDBY=5, SIGNED_UP=6, NOT_SIGNED_UP=7, TENTATIVE=8, REMOVED=9` |
| `CalendarModerationRank` | enum | `PLAYER=0, MODERATOR=1, OWNER=2` |
| `CalendarFlags` | enum (bitmask) | `ALL_ALLOWED=0x001, INVITES_LOCKED=0x010, WITHOUT_INVITES=0x040 (announcement), GUILD_EVENT=0x400` |
| `CalendarSendEventType` | enum | `GET=0, ADD=1, COPY=2` (drives `SMSG_CALENDAR_SEND_EVENT` payload variant) |
| `CalendarError` | enum | 28 codes (see header lines 94-130). Sent via `SMSG_CALENDAR_COMMAND_RESULT` |
| `CalendarMailAnswers` | enum | `EVENT_REMOVED=0`, `INVITE_REMOVED=0x100` — bitflag chosen by mail builder |
| `CalendarLimits` | enum | `MAX_EVENTS=30, MAX_GUILD_EVENTS=100, MAX_INVITES=100, CREATE_EVENT_COOLDOWN=5, OLD_EVENTS_DELETION_TIME=1*MONTH` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CalendarMgr::LoadFromDB()` | Loads `calendar_events` then `calendar_invites`, rebuilds `_events`/`_invites`, initializes `_freeEventIds`/`_freeInviteIds` gaps and `_maxEventId`/`_maxInviteId` | CharacterDB SELECTs; `new CalendarEvent`/`new CalendarInvite` |
| `CalendarMgr::AddEvent(event, sendType)` | Inserts into `_events`, persists via `CHAR_REP_CALENDAR_EVENT`, sends `SMSG_CALENDAR_SEND_EVENT` to owner with `sendType` (Get/Add/Copy) | `UpdateEvent`, `SendCalendarEvent` |
| `CalendarMgr::RemoveEvent(eventId, remover)` | Removes event, mails all invitees (subject/body via `BuildCalendarMail*`), broadcasts `SMSG_CALENDAR_EVENT_REMOVED_ALERT`, deletes `calendar_invites` then `calendar_events` rows in a single transaction, frees event/invite ids | `MailDraft::SendMailTo`, `SendPacketToAllEventRelatives`, `FreeEventId`, `FreeInviteId` |
| `CalendarMgr::UpdateEvent(event)` | Saves modifications via `CHAR_REP_CALENDAR_EVENT` (REPLACE INTO) | DB |
| `CalendarMgr::AddInvite(event, invite, trans)` | Pushes to `_invites[eventId]`, `CHAR_REP_CALENDAR_INVITE`, sends `SMSG_CALENDAR_INVITE_ADDED` to owner relatives + `SMSG_CALENDAR_INVITE_ALERT` to invitee if online | `SendCalendarEventInvite`, `SendCalendarEventInviteAlert` |
| `CalendarMgr::UpdateInvite(invite, trans)` | Persist invite changes via `CHAR_REP_CALENDAR_INVITE` | DB |
| `CalendarMgr::RemoveInvite(inviteId, eventId, remover)` | Removes from `_invites`, `CHAR_DEL_CALENDAR_INVITE`, sends `SMSG_CALENDAR_INVITE_REMOVED` to all relatives + `SMSG_CALENDAR_INVITE_REMOVED_ALERT` to invitee, mails the invitee on event-flagged removal, frees invite id | `MailDraft::SendMailTo` (when `flags & 0x100`), `SendCalendarEventInviteRemove*` |
| `CalendarMgr::RemoveAllPlayerEventsAndInvites(guid)` | Called on character delete: removes every event owned by `guid` + every invite where invitee == `guid` | `RemoveEvent`, `RemoveInvite` |
| `CalendarMgr::RemovePlayerGuildEventsAndSignups(guid, guildId)` | On guild leave: removes player's signups for guild events and any guild events the player owns | `RemoveEvent`, `RemoveInvite` |
| `CalendarMgr::GetEvent(id)` / `GetInvite(id)` | O(n) search of `_events` / `_invites` for matching id | — |
| `CalendarMgr::GetEventsCreatedBy(guid, includeGuild)` | Filters `_events` by owner | — |
| `CalendarMgr::GetPlayerEvents(guid)` | All events where the player has an invite or owns it (or guild event of the player's guild) | `GetPlayerInvites`, `Player::GetGuildId` |
| `CalendarMgr::GetGuildEvents(guildId)` | Events with `_eventGuildId == guildId` | — |
| `CalendarMgr::GetEventInvites(eventId)` | Returns `_invites[eventId]` (empty if none) | — |
| `CalendarMgr::GetPlayerInvites(guid)` | Linear scan of all `_invites` for invitee match | — |
| `CalendarMgr::GetPlayerNumPending(guid)` | Counts invites where status==`INVITED` (non-guild) and `responseTime == CALENDAR_DEFAULT_RESPONSE_TIME` | — |
| `CalendarMgr::GetFreeEventId / FreeEventId(id)` | LIFO id reuse; falls through to `++_maxEventId` when deque empty | — |
| `CalendarMgr::GetFreeInviteId / FreeInviteId(id)` | Same scheme for invites | — |
| `CalendarMgr::DeleteOldEvents()` | Removes events with `_date + 1*MONTH < now` | `RemoveEvent` |
| `CalendarMgr::SendCalendarEvent(guid, event, sendType)` | Builds `SMSG_CALENDAR_SEND_EVENT` (eventId, ownerGUID, title, description, type, repeatType, maxSize, dungeonId, flags, date, lockDate, guildId, list of invites with name/level/online/status/rank/note/responseTime) | `WorldSession::SendPacket` |
| `CalendarMgr::SendCalendarEventInvite(invite)` | `SMSG_CALENDAR_INVITE_ADDED` to event relatives | `SendPacketToAllEventRelatives` |
| `CalendarMgr::SendCalendarEventInviteAlert(event, invite)` | `SMSG_CALENDAR_INVITE_ALERT` direct-to-invitee | session lookup |
| `CalendarMgr::SendCalendarEventInviteRemove(event, invite, flags)` | `SMSG_CALENDAR_INVITE_REMOVED` broadcast + optional mail (flags `0x100`) | mail |
| `CalendarMgr::SendCalendarEventInviteRemoveAlert(guid, event, status)` | `SMSG_CALENDAR_INVITE_REMOVED_ALERT` to invitee | — |
| `CalendarMgr::SendCalendarEventUpdateAlert(event, originalDate)` | `SMSG_CALENDAR_EVENT_UPDATED_ALERT` to relatives | — |
| `CalendarMgr::SendCalendarEventStatus(event, invite)` | `SMSG_CALENDAR_INVITE_STATUS` to relatives | — |
| `CalendarMgr::SendCalendarEventRemovedAlert(event)` | `SMSG_CALENDAR_EVENT_REMOVED_ALERT` to relatives | — |
| `CalendarMgr::SendCalendarEventModeratorStatusAlert(event, invite)` | `SMSG_CALENDAR_MODERATOR_STATUS` (rank change) to relatives | — |
| `CalendarMgr::SendCalendarClearPendingAction(guid)` | `SMSG_CALENDAR_CLEAR_PENDING_ACTION` empty packet to a single player | — |
| `CalendarMgr::SendCalendarCommandResult(guid, err, param)` | `SMSG_CALENDAR_COMMAND_RESULT` (event title + `CalendarError`) | — |
| `CalendarMgr::SendPacketToAllEventRelatives(packet, event)` | Pushes `packet` to event owner + all current invitees that are online (or whole guild for guild events) | `Player::GetSession` |
| `CalendarMgr::GetAllEventRelatives(event)` | Returns the same set as a `std::vector<Player*>` for higher-level use | `ObjectAccessor::FindConnectedPlayer`, `Guild::GetMembers` |
| `CalendarEvent::BuildCalendarMailSubject(remover)` | Builds `"<remover guid hex>:<title>"` mail subject for removed-event/invite notices | — |
| `CalendarEvent::BuildCalendarMailBody(invitee)` | Encodes event date and timezone offset for the client mail UI | `Player::ToTimestamp` |
| `WorldSession::HandleCalendarGetCalendar` | Replies with `SMSG_CALENDAR_SEND_CALENDAR` (player's invites + raid lockouts + saved instances + holidays + character-bound bosses); also dispatches per-event `SMSG_CALENDAR_SEND_EVENT` for guild events | `GetPlayerInvites`, `GetPlayerEvents`, `Player::GetBoundInstances`, `sHolidaysStore` |
| `WorldSession::HandleCalendarAddEvent` | Validates: cooldown (5s), level ≥ 10, title non-empty, date in future. Creates `CalendarEvent`, drains attached invite list (≤100 invitees), `AddEvent(GET)` then per-invitee `AddInvite` | `Player::GetLastCalendarTime`, `CalendarMgr::AddEvent`, `CalendarMgr::AddInvite` |
| `WorldSession::HandleCalendarUpdateEvent` | Owner/moderator only. Updates fields then `UpdateEvent` + `SendCalendarEventUpdateAlert` | `CalendarMgr::UpdateEvent` |
| `WorldSession::HandleCalendarRemoveEvent` | Owner only. `RemoveEvent` (mails all invitees) | mgr |
| `WorldSession::HandleCalendarCopyEvent` | Clones event with new id and new date; copies all invites | mgr |
| `WorldSession::HandleCalendarInvite` | Owner/moderator invites a single player by name (unless event has `WITHOUT_INVITES`/announcement, then forbidden) | `CharacterCache`, `AddInvite` |
| `WorldSession::HandleCalendarEventSignup` | Self-signup for guild events: tentative-or-confirmed | `AddInvite` |
| `WorldSession::HandleCalendarRsvp` | Sets invite status (accept/decline/tentative/confirm), updates `_responseTime`, sends `SMSG_CALENDAR_INVITE_STATUS` | `UpdateInvite` |
| `WorldSession::HandleCalendarEventRemoveInvite` | Self-unsubscribe or owner-kick of one invitee | `RemoveInvite` |
| `WorldSession::HandleCalendarStatus` | Owner/moderator changes another invitee's status (e.g. confirmed→standby) | `UpdateInvite` |
| `WorldSession::HandleCalendarModeratorStatus` | Owner promotes/demotes invitee rank | `UpdateInvite` |
| `WorldSession::HandleCalendarComplain` | No-op stub: server logs CMSG and returns | — |
| `WorldSession::HandleCalendarGetNumPending` | Replies `SMSG_CALENDAR_SEND_NUM_PENDING` with `GetPlayerNumPending(guid)` | mgr |
| `WorldSession::HandleSetSavedInstanceExtend` | Toggles `InstanceLock::ExtendState`; mgr fires `SendCalendarRaidLockoutUpdated` | `InstanceLockMgr` |

---

## 5. Module dependencies

**Depends on:**
- `Mails` — `MailDraft::SendMailTo` to deliver event-removed / invite-removed notices to offline players (via `BuildCalendarMailSubject` / `BuildCalendarMailBody`).
- `ObjectMgr` / `CharacterCache` — name→GUID lookup for invites by character name; resolves invitee level/race/online state for event payload.
- `Guild` — guild-event invitee enumeration (`Guild::GetMembers`), guild-id lookup on the owner.
- `Player` / `WorldSession` — the only delivery target for SMSG packets and the source of CMSG handlers; also for `Player::SendRaidInfo` after `SetSavedInstanceExtend`.
- `Instances::InstanceLockMgr` — drives `SendCalendarRaidLockout(Added|Removed|Updated)`.
- `Database` (Character DB) — 4 prepared statements (see §6).
- `WorldPacket` / Packet builder infra — `WorldPackets::Calendar::*`.
- `DB2/DBC stores` — `Holidays.dbc`, `Difficulty.db2`, `Map.db2` (lockout payload).

**Depended on by:**
- `Player` — `m_lastCalendarInvite`/`m_lastCalendarTime` cooldown trackers; `RemoveAllPlayerEventsAndInvites` on character delete.
- `Guild` — `RemovePlayerGuildEventsAndSignups` on `Guild::DeleteMember`.
- `InstanceSaveMgr` / `InstanceLockMgr` — pushes lockout updates through CalendarMgr to inform clients without round-tripping a calendar fetch.
- `Mail` — only as a sender id source (`MailSender(MAIL_CALENDAR, eventId)`).

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_REP_CALENDAR_EVENT` | `REPLACE INTO calendar_events (EventID, Owner, Title, Description, EventType, TextureID, Date, Flags, LockDate) VALUES (?,?,?,?,?,?,?,?,?)` | character |
| `CHAR_DEL_CALENDAR_EVENT` | `DELETE FROM calendar_events WHERE EventID = ?` | character |
| `CHAR_REP_CALENDAR_INVITE` | `REPLACE INTO calendar_invites (InviteID, EventID, Invitee, Sender, Status, ResponseTime, ModerationRank, Note) VALUES (?,?,?,?,?,?,?,?)` | character |
| `CHAR_DEL_CALENDAR_INVITE` | `DELETE FROM calendar_invites WHERE InviteID = ?` | character |

Boot-load queries (raw SQL inside `LoadFromDB`):
- `SELECT EventID, Owner, Title, Description, EventType, TextureID, Date, Flags, LockDate FROM calendar_events`
- `SELECT InviteID, EventID, Invitee, Sender, Status, ResponseTime, ModerationRank, Note FROM calendar_invites`

DB2/DBC stores read by Calendar:

| Store | What it loads | Read by |
|---|---|---|
| `sHolidaysStore` (Holidays.dbc) | Game holiday schedule (durations, region, looping date table) | `HandleCalendarGetCalendar` to embed holidays in `SMSG_CALENDAR_SEND_CALENDAR` |
| `sMapStore` / `sDifficultyStore` (Map.db2 / Difficulty.db2) | Map name + difficulty for lockout entries | `SendCalendarRaidLockoutAdded/Removed/Updated` |
| `sLFGDungeonsStore` | Indirectly: when client opens calendar dungeon picker | event-add path |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_CALENDAR_GET = 0x3671` | client → server | `WorldSession::HandleCalendarGetCalendar` |
| `CMSG_CALENDAR_GET_EVENT = 0x3672` | client → server | `WorldSession::HandleCalendarGetEvent` |
| `CMSG_CALENDAR_COMMUNITY_INVITE = 0x3673` | client → server | `WorldSession::HandleCalendarCommunityInvite` |
| `CMSG_CALENDAR_INVITE = 0x3674` | client → server | `WorldSession::HandleCalendarInvite` |
| `CMSG_CALENDAR_REMOVE_INVITE = 0x3675` | client → server | `WorldSession::HandleCalendarEventRemoveInvite` |
| `CMSG_CALENDAR_RSVP = 0x3676` | client → server | `WorldSession::HandleCalendarRsvp` |
| `CMSG_CALENDAR_STATUS = 0x3677` | client → server | `WorldSession::HandleCalendarStatus` |
| `CMSG_CALENDAR_MODERATOR_STATUS = 0x3678` | client → server | `WorldSession::HandleCalendarModeratorStatus` |
| `CMSG_CALENDAR_REMOVE_EVENT = 0x3679` | client → server | `WorldSession::HandleCalendarRemoveEvent` |
| `CMSG_CALENDAR_COPY_EVENT = 0x367A` | client → server | `WorldSession::HandleCalendarCopyEvent` |
| `CMSG_CALENDAR_COMPLAIN = 0x367B` | client → server | `WorldSession::HandleCalendarComplain` (stub) |
| `CMSG_CALENDAR_GET_NUM_PENDING = 0x367C` | client → server | `WorldSession::HandleCalendarGetNumPending` |
| `CMSG_CALENDAR_EVENT_SIGN_UP = 0x367D` | client → server | `WorldSession::HandleCalendarEventSignup` |
| `CMSG_CALENDAR_ADD_EVENT = 0x367F` | client → server | `WorldSession::HandleCalendarAddEvent` |
| `CMSG_CALENDAR_UPDATE_EVENT = 0x3680` | client → server | `WorldSession::HandleCalendarUpdateEvent` |
| `CMSG_SET_SAVED_INSTANCE_EXTEND` | client → server | `WorldSession::HandleSetSavedInstanceExtend` |
| `SMSG_CALENDAR_SEND_CALENDAR = 0x268B` | server → client | `HandleCalendarGetCalendar` |
| `SMSG_CALENDAR_SEND_EVENT = 0x268C` | server → client | `CalendarMgr::SendCalendarEvent` |
| `SMSG_CALENDAR_COMMUNITY_INVITE = 0x268D` | server → client | community invite reply |
| `SMSG_CALENDAR_INVITE_ADDED = 0x268E` | server → client | `SendCalendarEventInvite` |
| `SMSG_CALENDAR_INVITE_REMOVED = 0x268F` | server → client | `SendCalendarEventInviteRemove` |
| `SMSG_CALENDAR_INVITE_STATUS = 0x2690` | server → client | `SendCalendarEventStatus` |
| `SMSG_CALENDAR_MODERATOR_STATUS = 0x2691` | server → client | `SendCalendarEventModeratorStatusAlert` |
| `SMSG_CALENDAR_INVITE_ALERT = 0x2692` | server → client | `SendCalendarEventInviteAlert` |
| `SMSG_CALENDAR_INVITE_STATUS_ALERT = 0x2693` | server → client | RSVP propagation |
| `SMSG_CALENDAR_INVITE_REMOVED_ALERT = 0x2694` | server → client | `SendCalendarEventInviteRemoveAlert` |
| `SMSG_CALENDAR_EVENT_REMOVED_ALERT = 0x2695` | server → client | `SendCalendarEventRemovedAlert` |
| `SMSG_CALENDAR_EVENT_UPDATED_ALERT = 0x2696` | server → client | `SendCalendarEventUpdateAlert` |
| `SMSG_CALENDAR_INVITE_NOTES = 0x2697` | server → client | per-invite note read |
| `SMSG_CALENDAR_INVITE_NOTES_ALERT = 0x2698` | server → client | note changed broadcast |
| `SMSG_CALENDAR_RAID_LOCKOUT_ADDED = 0x2699` | server → client | `Player::SendCalendarRaidLockout` (new bind) |
| `SMSG_CALENDAR_RAID_LOCKOUT_REMOVED = 0x269A` | server → client | lockout expiry |
| `SMSG_CALENDAR_RAID_LOCKOUT_UPDATED = 0x269B` | server → client | extend toggled |
| `SMSG_CALENDAR_SEND_NUM_PENDING = 0x269C` | server → client | `HandleCalendarGetNumPending` |
| `SMSG_CALENDAR_CLEAR_PENDING_ACTION = 0x269D` | server → client | after RSVP / accept |
| `SMSG_CALENDAR_COMMAND_RESULT = 0x269E` | server → client | `SendCalendarCommandResult` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-world/src/handlers/misc.rs` — 2 noop stubs (`handle_calendar_get_num_pending`, `handle_calendar_get`) that consume the WorldPacket and return without sending anything.
- `crates/wow-world/src/session.rs` — dispatch entries for the same 2 opcodes (`ClientOpcodes::CalendarGet`, `ClientOpcodes::CalendarGetNumPending`).

**What's implemented:**
- Nothing functional. Two opcode dispatch slots so the server doesn't drop the packets unhandled.

**What's missing vs C++:**
- `CalendarMgr` global singleton.
- `CalendarEvent` / `CalendarInvite` POD structs and persistence layer.
- All 13 remaining CMSG handlers (AddEvent, UpdateEvent, RemoveEvent, CopyEvent, Invite, EventSignup, RSVP, RemoveInvite, Status, ModeratorStatus, Complain, CommunityInvite, GetEvent, SetSavedInstanceExtend).
- All 22 SMSG builders.
- Character DB statements `CHAR_REP_CALENDAR_EVENT`, `CHAR_DEL_CALENDAR_EVENT`, `CHAR_REP_CALENDAR_INVITE`, `CHAR_DEL_CALENDAR_INVITE`.
- `LoadFromDB` boot path.
- `RemoveAllPlayerEventsAndInvites` hook on character delete.
- `RemovePlayerGuildEventsAndSignups` hook on guild leave.
- Cooldown enforcement (`m_lastCalendarTime`).
- Mail integration (event-removed / invite-removed mail to offline targets).
- Holidays (`Holidays.dbc` reader + payload in `SMSG_CALENDAR_SEND_CALENDAR`).
- Raid lockout integration (`SendCalendarRaidLockout*`).
- `DeleteOldEvents` periodic task (run on world tick or on calendar-mgr load).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — module is essentially absent.

**Tests existing:**
- 0 calendar tests. Only `all_achievement_data_empty` exists for AchievementData (unrelated).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#CALENDAR.WBS.001** Partir y cerrar la migracion auditada de `game/Calendar/CalendarMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 768 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CALENDAR.WBS.002** Cerrar la migracion auditada de `game/Calendar/CalendarMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#CAL.1** Add `CalendarEvent` and `CalendarInvite` structs (mirror C++ fields, derive Clone/Debug) in new `crates/wow-world/src/calendar/mod.rs` (L)
- [ ] **#CAL.2** Add enums: `CalendarEventType`, `CalendarRepeatType`, `CalendarInviteStatus`, `CalendarModerationRank`, `CalendarSendEventType`, `CalendarFlags` (bitflags!), `CalendarError` in `crates/wow-core/src/calendar.rs` (L)
- [ ] **#CAL.3** Define constants `CALENDAR_MAX_EVENTS=30`, `CALENDAR_MAX_GUILD_EVENTS=100`, `CALENDAR_MAX_INVITES=100`, `CALENDAR_CREATE_EVENT_COOLDOWN=5`, `CALENDAR_OLD_EVENTS_DELETION_TIME` (30 days), `CALENDAR_DEFAULT_RESPONSE_TIME=946684800` (L)
- [ ] **#CAL.4** Implement `CalendarMgr` singleton via `OnceLock<RwLock<CalendarMgrState>>`; state owns `events: HashMap<u64, CalendarEvent>`, `invites: HashMap<u64, Vec<CalendarInvite>>`, free-id deques, `max_event_id`, `max_invite_id` (M)
- [ ] **#CAL.5** Add 4 prepared statement enums + binding helpers in `crates/wow-database/src/statements/character.rs`: `RepCalendarEvent`, `DelCalendarEvent`, `RepCalendarInvite`, `DelCalendarInvite` (L)
- [ ] **#CAL.6** Implement `CalendarMgr::load_from_db(pool)`: read both tables, populate caches, recompute free-id deques and max-id counters (M)
- [ ] **#CAL.7** Implement `CalendarMgr::add_event(event, send_type)`, `update_event`, `remove_event` with transactional write + frees + broadcast (M)
- [ ] **#CAL.8** Implement `add_invite`, `update_invite`, `remove_invite` (transactional) (M)
- [ ] **#CAL.9** Implement `get_player_invites`, `get_player_events`, `get_guild_events`, `get_events_created_by`, `get_player_num_pending` queries (L)
- [ ] **#CAL.10** Build `SMSG_CALENDAR_SEND_CALENDAR` packet in `crates/wow-packet/src/packets/calendar.rs` including raid lockouts, holidays, saved instances, character-bound bosses (H)
- [ ] **#CAL.11** Build `SMSG_CALENDAR_SEND_EVENT` (3 send-type variants) (M)
- [ ] **#CAL.12** Build SMSG variants `INVITE_ADDED`, `INVITE_ALERT`, `INVITE_REMOVED`, `INVITE_REMOVED_ALERT`, `INVITE_STATUS`, `INVITE_STATUS_ALERT`, `INVITE_NOTES`, `INVITE_NOTES_ALERT`, `MODERATOR_STATUS` (M)
- [ ] **#CAL.13** Build SMSG `EVENT_REMOVED_ALERT`, `EVENT_UPDATED_ALERT`, `SEND_NUM_PENDING`, `CLEAR_PENDING_ACTION`, `COMMAND_RESULT` (M)
- [ ] **#CAL.14** Build SMSG `RAID_LOCKOUT_ADDED`, `RAID_LOCKOUT_REMOVED`, `RAID_LOCKOUT_UPDATED` and tie into `InstanceLockMgr` (M)
- [ ] **#CAL.15** Build SMSG `COMMUNITY_INVITE` reply (L)
- [ ] **#CAL.16** Implement CMSG handler `HandleCalendarGetCalendar` (replace stub `handle_calendar_get`); fetches per-player events, invites, lockouts, holidays (M)
- [ ] **#CAL.17** Implement CMSG `HandleCalendarGetEvent` → `SMSG_CALENDAR_SEND_EVENT(GET)` (L)
- [ ] **#CAL.18** Implement CMSG `HandleCalendarAddEvent` with cooldown check (5s), level≥10, title non-empty, future-date, attached invite list expansion (H)
- [ ] **#CAL.19** Implement CMSG `HandleCalendarUpdateEvent` with rank check (owner/moderator) (M)
- [ ] **#CAL.20** Implement CMSG `HandleCalendarRemoveEvent` (owner-only) including mail-out path (M)
- [ ] **#CAL.21** Implement CMSG `HandleCalendarCopyEvent` (deep clone with new ids, new date) (M)
- [ ] **#CAL.22** Implement CMSG `HandleCalendarInvite` (single-invitee by name, blocked on `WITHOUT_INVITES` flag) (M)
- [ ] **#CAL.23** Implement CMSG `HandleCalendarEventSignup` (self-signup for guild events) (L)
- [ ] **#CAL.24** Implement CMSG `HandleCalendarRsvp` (set status + responseTime + alert) (M)
- [ ] **#CAL.25** Implement CMSG `HandleCalendarEventRemoveInvite` (self-leave or owner-kick) (M)
- [ ] **#CAL.26** Implement CMSG `HandleCalendarStatus` (rank-gated status change of another invitee) (M)
- [ ] **#CAL.27** Implement CMSG `HandleCalendarModeratorStatus` (owner-only rank change) (L)
- [ ] **#CAL.28** Implement CMSG stub `HandleCalendarComplain` (log + drop) (L)
- [ ] **#CAL.29** Replace stub `handle_calendar_get_num_pending` with real `SMSG_CALENDAR_SEND_NUM_PENDING` builder (L)
- [ ] **#CAL.30** Implement CMSG `HandleCalendarCommunityInvite` (resolve guild members, send `SMSG_CALENDAR_COMMUNITY_INVITE`) (M)
- [ ] **#CAL.31** Implement CMSG `HandleSetSavedInstanceExtend` (toggle `InstanceLock::extend_state`, push `SMSG_CALENDAR_RAID_LOCKOUT_UPDATED`) (M)
- [ ] **#CAL.32** Implement `BuildCalendarMailSubject` / `BuildCalendarMailBody` and wire into `MailDraft` for event-removed / invite-removed offline notices (M)
- [ ] **#CAL.33** Wire `Player::on_delete` to call `CalendarMgr::remove_all_player_events_and_invites(guid)` (L)
- [ ] **#CAL.34** Wire `Guild::on_member_remove` to call `CalendarMgr::remove_player_guild_events_and_signups(guid, guild_id)` (L)
- [ ] **#CAL.35** Schedule `delete_old_events` on world tick (e.g. once per game hour) to GC events older than 30 days (L)
- [ ] **#CAL.36** Cooldown tracking: add `last_calendar_create_time: SystemTime` to `Player` session state (L)
- [ ] **#CAL.37** Holidays loader: `Holidays.dbc` reader producing the calendar payload struct (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#CALENDAR.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 1140 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h`. Rust target: `crates/wow-core`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-core && cargo test -p wow-database && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#CALENDAR.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 1140 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h`. Rust target: `crates/wow-core`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#CALENDAR.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 1140 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h`. Rust target: `crates/wow-core`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#CALENDAR.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 1140 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h`. Rust target: `crates/wow-core`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests que demuestren que el comportamiento Rust = comportamiento C++ para invariantes clave.

- [ ] Test: `add_event` then `get_event` returns same struct
- [ ] Test: id allocator reuses freed ids before incrementing `max_event_id` (LIFO)
- [ ] Test: invite limit (101st invite to same event is rejected with `INVITES_EXCEEDED`)
- [ ] Test: per-player event limit (31st personal event rejected with `EVENTS_EXCEEDED`)
- [ ] Test: per-guild event limit (101st guild event rejected with `GUILD_EVENTS_EXCEEDED`)
- [ ] Test: 5s create cooldown rejects second `AddEvent` from same player
- [ ] Test: copy_event yields different `event_id` and different `invite_id`s but identical fields
- [ ] Test: RSVP transitions update `_responseTime` and `_status` and persist via `CHAR_REP_CALENDAR_INVITE`
- [ ] Test: `delete_old_events` removes events with `_date + 30d < now` and frees ids
- [ ] Test: `remove_all_player_events_and_invites` deletes both owned events and inbound invites
- [ ] Test: removed-event mail body matches C++ `BuildCalendarMailBody` byte-exactly
- [ ] Test: round-trip serialize→deserialize of `SMSG_CALENDAR_SEND_EVENT(ADD)` against captured C++ packet
- [ ] Test: announcement event (flag `WITHOUT_INVITES`) rejects `CMSG_CALENDAR_INVITE`
- [ ] Test: guild event auto-lists all guild members in `SendPacketToAllEventRelatives`
- [ ] Test: `GetPlayerNumPending` only counts invites with status==INVITED && responseTime==CALENDAR_DEFAULT_RESPONSE_TIME

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 1140 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h` | `crates/wow-world/` (handlers, CalendarMgr global), `crates/wow-database/` (calendar prepared statements), `crates/wow-packet/` (CalendarPackets), `crates/wow-core/` (calendar enums) \| ⚠️ stub-only (2 noop handlers, audit confirmed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#CALENDAR.DIV.001` | _none generated_ | 2 C++ files / 1140 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Calendar/CalendarMgr.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **Sentinel time `946684800`** (`2000-01-01 00:00:00 UTC`) marks "never responded". Don't replace with `0` — the client checks this exact value to render the "Pending" badge.
- **Repeat types are decorative.** The 3.3.5 server never auto-recreates events, even when the client sends `WEEKLY`/`BIWEEKLY`/`MONTHLY`. Be careful not to reintroduce a "feature" that breaks parity.
- **Free-id deques are LIFO** (`std::deque<uint64>` with `push_back` / `pop_back`). Tests for id reuse depend on this ordering.
- **`SMSG_CALENDAR_SEND_CALENDAR` is enormous** — embeds raid lockouts (`InstanceLock`), saved instances, holidays, and per-event invite snapshots in a single packet. Plan for chunked builder helpers, not one giant function.
- **Mail flag `0x100` (`CALENDAR_INVITE_REMOVED_MAIL_SUBJECT`)** is checked at `*((char*)+8292) & 0x100` in the client; encode the mail subject with that flag set when removing an invite from a moderated event.
- **Cooldown check is per-player, not per-account.** GMs typically bypass via RBAC.
- **`HandleCalendarComplain` is a no-op in TC** but the opcode must still be ack'd; otherwise the client retries.
- **Lockout updates are pushed unsolicited** every time `InstanceLockMgr` mutates. The Rust port must thread that callback (or post a message) through `CalendarMgr` even though Calendar doesn't own instance state.
- **Event `_date` is stored as Unix epoch in seconds** (signed `time_t`). Lock date is also Unix epoch. The client renders local-time and applies its own TZ offset.
- **DB `REPLACE INTO` semantics** — used for both tables to avoid an explicit upsert pattern. If migrating to a Rust SQLx with `INSERT … ON CONFLICT`, replicate the same key set (`EventID` PK, `InviteID` PK).
- **Character delete is asynchronous in TC**; ensure `remove_all_player_events_and_invites` runs *before* `character_achievement` cascade so the mail is queued while the player row still exists in `CharacterCache`.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CalendarMgr` (singleton) | `static CALENDAR: OnceLock<RwLock<CalendarMgrState>>` + `pub fn calendar()` accessor | No inheritance. Use a `Mutex` if access pattern is mostly write |
| `CalendarEvent*` (heap, non-copy) | `CalendarEvent` value stored in `HashMap<u64, CalendarEvent>`; lookups return `Option<&CalendarEvent>` | Non-copy is enforced by ownership in the HashMap |
| `CalendarInvite*` (heap) | `CalendarInvite` stored in `HashMap<u64, Vec<CalendarInvite>>` keyed by event_id | Or `HashMap<u64, CalendarInvite>` keyed by invite_id with secondary index by event |
| `std::set<CalendarEvent*>` | `HashMap<u64, CalendarEvent>` | Iteration order is not used |
| `std::map<uint64, vector<CalendarInvite*>>` | `HashMap<u64, Vec<CalendarInvite>>` | — |
| `std::deque<uint64>` (free-ids) | `VecDeque<u64>` | LIFO via `push_back` / `pop_back` |
| `time_t _date` | `i64` Unix epoch seconds | Use chrono `DateTime<Utc>` only at boundaries |
| `ObjectGuid` | `Guid` (existing) | — |
| `ObjectGuid::LowType` | `u64` | — |
| `CalendarFlags` (uint32 bitmask) | `bitflags! struct CalendarFlags: u32` | — |
| `enum CalendarError` | `#[repr(u32)] enum CalendarError` | — |
| `CharacterDatabaseTransaction` | `&mut sqlx::Transaction<'_, MySql>` (or whatever the DB layer exposes) | — |
| `void CalendarMgr::SendPacketToAllEventRelatives(packet, event)` | `fn broadcast_relatives(state, packet, event)` over `WorldSessions` registry | Use a session registry similar to `WorldSessions` lookup |
| `WorldSession::HandleCalendarRsvp(...)` | `async fn handle_calendar_rsvp(&mut self, pkt: WorldPacket)` in `crates/wow-world/src/handlers/calendar.rs` | New file |
| `MailDraft(MailMessageType::MAIL_CALENDAR, eventId)` | `MailDraft::calendar(event_id)` constructor | Once Mails module migrated |
| `CALENDAR_DEFAULT_RESPONSE_TIME = 946684800` | `pub const CALENDAR_DEFAULT_RESPONSE_TIME: i64 = 946_684_800;` | Y2K sentinel |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines `CalendarMgr` Rust impl | ✅ | `grep -rn "CalendarMgr\|CalendarEvent\|CalendarInvite" crates/ → 0` |
| 35 calendar opcodes in `wow-constants` | ✅ | `grep -c "Calendar" crates/wow-constants/src/opcodes.rs` = 35 |
| Only 2 noop handlers wired | ✅ | `handlers/misc.rs:603` `handle_calendar_get_num_pending(_pkt) {}` and `:614` `handle_calendar_get(_pkt) {}`; dispatched at `session.rs:1808,1841` |
| 33 remaining opcodes silently dropped | ⚠️ | Other 33 (`CalendarAddEvent`, `CalendarInvite`, `CalendarRsvp`, etc.) have NO match arm AND NO inventory submit — fall through to default unhandled-opcode path. Not a silent-hang risk per se because the client won't receive `SMSG_CALENDAR_*`, but UI showing the calendar tab will appear empty/broken. |
| No DB schema/prepared statements | ✅ | grep `calendar_event\|calendar_invite\|CALENDAR_INS` in `crates/wow-database` → 0 |

**Net status:** doc's `❌` was slightly off — there's a 2-handler stub. Reclassified to ⚠️ stub-only.

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.
