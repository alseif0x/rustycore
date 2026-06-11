# Migration: Social (Friends, Ignore, Inspect)

> **C++ canonical path:** `src/server/game/Handlers/SocialHandler.cpp` + `src/server/game/Entities/Player/SocialMgr.{h,cpp}` + `src/server/game/Globals/ObjectMgr.cpp` (player name cache)
> **Rust target crate(s):** `crates/wow-social/` (empty placeholder), `crates/wow-world/src/handlers/social.rs`, `crates/wow-world/src/handlers/inspect.rs`, `crates/wow-packet/src/packets/social.rs`, `crates/wow-packet/src/packets/inspect.rs`
> **Layer:** L6
> **Status:** ⚠️ partial (~50% — friends list works incl. DB persistence; ignore is missing; mute, account-level ignore, contract, presence updates all missing)
> **Audited vs C++:** ✅ audited 2026-05-01 (§13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Manages each player's per-character contact lists: friends (status broadcast on login/logout/zone-change), ignored players (suppresses chat/whispers), muted players (suppresses voice — placeholder), per-friend notes, and account-level ignore (so blocking one alt blocks the whole account). Also covers `CMSG_INSPECT` (read-only equipment + spec view of another player) and the social contract (ToS) prompt at first login. Persists to `character_social` and is loaded with the character.

---

## 2. C++ canonical files

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Handlers/SocialHandler.cpp` | 184 | All `CMSG_*FRIEND*` / `CMSG_*IGNORE*` / `CMSG_SET_CONTACT_NOTES` / `CMSG_SOCIAL_CONTRACT_REQUEST` opcodes |
| `src/server/game/Entities/Player/SocialMgr.h` | 163 | `PlayerSocial`, `SocialMgr` singleton, `FriendStatus`, `SocialFlag`, `FriendsResult` enums, `FriendInfo` struct |
| `src/server/game/Entities/Player/SocialMgr.cpp` | ~280 | DB-backed friend/ignore list, status broadcast on player login/AFK/DND/zone, per-account ignore enforcement |
| `src/server/game/Server/Packets/SocialPackets.h` | ~250 | Packet structs: `SendContactList`, `ContactList`, `AddFriend`, `DelFriend`, `AddIgnore`, `DelIgnore`, `FriendStatus`, `SetContactNotes`, `SocialContractRequest(Response)` |
| `src/server/game/Server/Packets/SocialPackets.cpp` | ~200 | Wire serialisation |
| `src/server/game/Handlers/InspectHandler.cpp` (folded into ChatHandler / per-version) | ~150 | `CMSG_INSPECT`, `CMSG_QUERY_INSPECT_ACHIEVEMENTS`, `CMSG_INSPECT_PVP` |
| `src/server/game/Server/Packets/InspectPackets.h/.cpp` | ~300 | `Inspect`, `InspectResult`, `InspectGuildData`, `InspectTalentData`, `PVPBracketData` |
| `src/server/game/Globals/ObjectMgr.cpp` (relevant section) | ~200 | `PlayerNameMap` cache (guid→name+race+class+gender+level) used by `SendContactList` to populate names without hitting DB |
| `src/server/game/Server/Protocol/Opcodes.cpp` | — | Wires `STATUS_LOGGEDIN`/`PROCESS_THREADUNSAFE` for all social/inspect opcodes |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `PlayerSocial` | class | Per-player container of friends + ignores; owned by `Player`; `_playerSocialMap: map<ObjectGuid, FriendInfo>` and `_ignoredAccounts: GuidUnorderedSet` |
| `SocialMgr` | class (singleton) | Global registry `map<ObjectGuid, PlayerSocial>`; loads from DB, performs friend-status broadcasts to followers |
| `FriendInfo` | struct | `{WowAccountGuid, FriendStatus, Flags, Area, Level, Class, Note}` — what `SMSG_FRIEND_STATUS` carries |
| `enum FriendStatus` | enum | `OFFLINE=0x00`, `ONLINE=0x01`, `AFK=0x02`, `DND=0x04`, `RAF=0x08` (bitmask combined) |
| `enum SocialFlag` | enum | `FRIEND=0x01`, `IGNORED=0x02`, `MUTED=0x04`, `UNK=0x08`, `ALL=0x07` |
| `enum FriendsResult : uint8` | enum | 29 result codes (DB_ERROR, LIST_FULL, ONLINE, OFFLINE, NOT_FOUND, REMOVED, ADDED_ONLINE/OFFLINE, ALREADY, SELF, ENEMY, IGNORE_*, MUTE_*, AMBIGUOUS, UNK1/2/3, UNKNOWN) |
| `WorldPackets::Social::ContactInfo` | struct | One row in `SMSG_CONTACT_LIST` (GUID, account-guid, vra, native-vra, type-flags, note, status, area, level, class, isMobile bit) |
| `WorldPackets::Inspect::InspectItem` | struct | One equipped item in `SMSG_INSPECT_RESULT` (slot, item-id, modifications, gems) |
| `WorldPackets::Inspect::PVPBracketData` | struct | Per-bracket arena rating |
| `SOCIALMGR_FRIEND_LIMIT` / `SOCIALMGR_IGNORE_LIMIT` | constants | 50 each |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `WorldSession::HandleContactListOpcode(SendContactList&)` | Reads u32 flags (mask of types to return), serialises full social list back as `SMSG_CONTACT_LIST` | `PlayerSocial::SendSocialList` |
| `WorldSession::HandleAddFriendOpcode(AddFriend&)` | Lookup target by name (case-insensitive, current-realm), check enemy-faction, self, list-full, already-friend; insert via `PlayerSocial::AddToSocialList` + `CHAR_INS_CHARACTER_SOCIAL`; emit `SMSG_FRIEND_STATUS` | `ObjectAccessor::FindPlayerByName`, DB |
| `WorldSession::HandleDelFriendOpcode(DelFriend&)` | Remove by GUID; emit `FRIEND_REMOVED`; persist via `CHAR_DEL_CHARACTER_SOCIAL_BY_FRIEND` | `PlayerSocial::RemoveFromSocialList` |
| `WorldSession::HandleAddIgnoreOpcode(AddIgnore&)` | Same flow as AddFriend but with `SOCIAL_FLAG_IGNORED`; ALSO captures the target's `WowAccountGuid` so all of their alts auto-ignore | `PlayerSocial::AddToSocialList` |
| `WorldSession::HandleDelIgnoreOpcode(DelIgnore&)` | Unblock a GUID + recompute `_ignoredAccounts` from remaining ignored entries | `PlayerSocial::RemoveFromSocialList` |
| `WorldSession::HandleSetContactNotesOpcode(SetContactNotes&)` | Set/edit a 48-char note on a friend OR ignore entry | `PlayerSocial::SetFriendNote` |
| `WorldSession::HandleSocialContractRequest(SocialContractRequest&)` | First-login ToS prompt response; sends `SMSG_SOCIAL_CONTRACT_REQUEST_RESPONSE` | session state |
| `PlayerSocial::AddToSocialList(guid, accountGuid, flag)` | Inserts into in-memory map, persists row (or upserts flag bitmask), enforces 50-entry cap | DB `CHAR_REP_CHARACTER_SOCIAL` (REPLACE INTO) |
| `PlayerSocial::RemoveFromSocialList(guid, flag)` | Strips one flag; if remaining flags == 0, deletes the row | DB `CHAR_DEL_CHARACTER_SOCIAL` |
| `PlayerSocial::SetFriendNote(guid, note)` | Stores note; persists | DB `CHAR_UPD_CHARACTER_SOCIAL_NOTE` |
| `PlayerSocial::SendSocialList(Player*, flags)` | Builds `SMSG_CONTACT_LIST` filtered by flags mask; resolves online/offline + area/level/class via `SocialMgr::GetFriendInfo` | `WorldSession::SendPacket` |
| `PlayerSocial::HasFriend(guid)` / `HasIgnore(guid, accountGuid)` | Membership checks | — |
| `SocialMgr::GetFriendInfo(player, friendGUID, FriendInfo&)` | Populates `FriendInfo` from a possibly-online target's `Player*` (status, area, level, class, account guid, RaF) | `World::FindSession`, `Player::GetZoneId` |
| `SocialMgr::SendFriendStatus(player, result, friendGuid, broadcast)` | Sends a single `SMSG_FRIEND_STATUS` to player or to all who friend the player | `BroadcastToFriendListers` |
| `SocialMgr::BroadcastToFriendListers(player, packet)` | Iterates `_socialMap`, finds anyone who lists `player.GUID`, forwards packet | per-session |
| `SocialMgr::LoadFromDB(PreparedQueryResult, guid)` | Builds `PlayerSocial` from `character_social` rows | DB |
| `WorldSession::HandleInspectOpcode(Inspect&)` | Look up target Player; build `SMSG_INSPECT_RESULT` (equipped items, talents, glyphs, achievements, guild, race/class/level) | `Player::GetInventoryItem`, `TalentMgr` |
| `WorldSession::HandleQueryInspectAchievements` | Sends compressed achievement list of inspect target | `Player::GetAchievementMgr` |
| `WorldSession::HandleInspectPVP(InspectPvP&)` | Returns 6-bracket arena rating for inspect target | `ArenaTeam::GetStats` |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Player** — `Player` owns its `PlayerSocial`; status fetched from `Player::GetZoneId`/`GetLevel`/`GetClass`/`isAFK`/`isDND`
- **Globals/ObjectMgr** — `PlayerNameMap` cache (`GetPlayerNameByGUID`) used to populate `ContactInfo.note`/race/class/level without hitting DB on every friend
- **CharacterDatabase** — `character_social` table (cols: `guid`, `friend`, `flags`, `note`, `accountGuid`)
- **Server/WorldSession** — packet send/recv + STATUS_LOGGEDIN gating
- **Globals/ObjectAccessor** — `FindConnectedPlayerByName` for name→GUID resolution on `AddFriend`/`AddIgnore`
- **DBC/DB2** — `ChrRacesStore` (faction → enemy check on `AddFriend`)
- **AchievementMgr** (for inspect-achievements)
- **TalentMgr** (for inspect-talent serialisation)
- **Guild module** (for `InspectGuildData` payload — guild name, level, members count)
- **ArenaTeam** (for inspect-PvP brackets)
- **Items/Inventory** (for visible-item slots + transmog/enchant on inspect)

**Depended on by:**
- **ChatHandler** — drops whispers from senders on the recipient's ignore list (cross-references `PlayerSocial::HasIgnore`)
- **Player::SaveToDB** / load — saves social rows together with character
- **WorldSession::LogoutPlayer** — calls `SocialMgr::RemovePlayerSocial(guid)` and broadcasts `FRIEND_OFFLINE` to followers
- **AccountMgr / battle.net** — reads `WowAccountGuid` for cross-realm ignore propagation

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_REP_CHARACTER_SOCIAL` (REPLACE INTO) | Insert/upsert friend or ignore row (preserves note across flag changes) | character |
| `CHAR_DEL_CHARACTER_SOCIAL` | Remove a single row by `(guid, friend)` | character |
| `CHAR_DEL_CHARACTER_SOCIAL_BY_FRIEND` | Remove all rows where `friend = guid` (used when char deleted) | character |
| `CHAR_UPD_CHARACTER_SOCIAL_FLAGS` | Toggle flag bit without losing other flag/note | character |
| `CHAR_UPD_CHARACTER_SOCIAL_NOTE` | Persist note on friend row | character |
| `CHAR_SEL_CHARACTER_SOCIAL` | Bulk-load all social rows for a character on login | character |
| Direct ad-hoc query in current Rust code (`SELECT CAST(guid AS SIGNED), account, race, class, level, zone FROM characters WHERE name = ?`) | Name→info lookup on AddFriend | character |

DBC/DB2 stores read:

| Store | What it loads | Read by |
|---|---|---|
| `ChrRacesStore` | Race-faction map for enemy-check on `AddFriend` | `HandleAddFriendOpcode` |
| `AreaTableStore` | `area_id` field shown in friend list (resolved → name client-side) | indirect (`Player::GetAreaId`) |

Note: `character_social` schema in 3.4.3 is `(guid, friend, flags, note)` — the `accountGuid` column was added in later patches to support account-level ignore. RustyCore must add it.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_SEND_CONTACT_LIST` | C→S | `HandleContactListOpcode` |
| `CMSG_ADD_FRIEND` | C→S | `HandleAddFriendOpcode` |
| `CMSG_DEL_FRIEND` | C→S | `HandleDelFriendOpcode` |
| `CMSG_ADD_IGNORE` | C→S | `HandleAddIgnoreOpcode` |
| `CMSG_DEL_IGNORE` | C→S | `HandleDelIgnoreOpcode` |
| `CMSG_SET_CONTACT_NOTES` | C→S | `HandleSetContactNotesOpcode` |
| `CMSG_SOCIAL_CONTRACT_REQUEST` | C→S | `HandleSocialContractRequest` |
| `CMSG_INSPECT` | C→S | `HandleInspectOpcode` |
| `CMSG_QUERY_INSPECT_ACHIEVEMENTS` | C→S | `HandleQueryInspectAchievements` |
| `CMSG_INSPECT_PVP` | C→S | `HandleInspectPVP` |
| `SMSG_CONTACT_LIST` | S→C | `PlayerSocial::SendSocialList` |
| `SMSG_FRIEND_STATUS` | S→C | `SocialMgr::SendFriendStatus` (one row) |
| `SMSG_SOCIAL_CONTRACT_REQUEST_RESPONSE` | S→C | `HandleSocialContractRequest` |
| `SMSG_INSPECT_RESULT` | S→C | `HandleInspectOpcode` |
| `SMSG_RESPOND_INSPECT_ACHIEVEMENTS` | S→C | `HandleQueryInspectAchievements` |
| `SMSG_INSPECT_PVP` | S→C | `HandleInspectPVP` |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-social/src/lib.rs` — **0 lines** (empty stub; should host `PlayerSocial`, `SocialMgr`, presence-broadcast logic)
- `crates/wow-world/src/handlers/social.rs` — covers ~55% of `Handlers/SocialHandler.cpp`
- `crates/wow-world/src/handlers/inspect.rs` — 82 lines — covers ~30% of inspect (basic items + race/class/level only)
- `crates/wow-packet/src/packets/social.rs` — `AddIgnore`, `DelIgnore`, `SetContactNotes`, `ContactInfo`, `ContactListPkt`, `FriendStatusPkt`, `FriendsResult`
- `crates/wow-packet/src/packets/inspect.rs` — 120 lines — `InspectItem`, `InspectResult`

**What's implemented:**
- `CMSG_ADD_FRIEND` — name lookup in `characters` table; self-check; already-friend check; 50-entry cap; ORs `SOCIAL_FLAG_FRIEND` into `character_social` while preserving existing ignore/mute flags; replies with `FRIEND_ADDED_ONLINE`/`OFFLINE`/`ALREADY`/`NOT_FOUND`/`SELF`/`LIST_FULL`.
- `CMSG_ADD_IGNORE` — parses C++ `nameLength(9) + AccountGUID + Name`, looks up `characters`, applies self/already/full/not-found gates, and stores per-character `SOCIAL_FLAG_IGNORED` (`flags=2`) in `character_social`. Account-level ignore remains missing.
- `CMSG_DEL_FRIEND` — parses C++ `QualifiedGUID`, clears `SOCIAL_FLAG_FRIEND` (`flags &= ~1`) and deletes the row only if no flags remain; emits `FRIEND_REMOVED`.
- `CMSG_DEL_IGNORE` — parses C++ `QualifiedGUID`, clears `SOCIAL_FLAG_IGNORED` (`flags &= ~2`) and deletes the row only if no flags remain; emits `FRIEND_IGNORE_REMOVED`. Account-level ignored-account recompute remains missing.
- `CMSG_SET_CONTACT_NOTES` — parses C++ `QualifiedGUID + notes_len(10) + Notes`, truncates to the 48-char DB/client limit and updates `character_social.note` for an existing contact; no response packet, matching C++.
- `CMSG_SEND_CONTACT_LIST` — JOINs `character_social` × `characters`; populates `ContactInfo`; also emits `QueryPlayerNamesResponse` (name cache) so client can render names.
- `CMSG_INSPECT` — registry lookup of target's broadcast info; sends `SMSG_INSPECT_RESULT` with target's race, class, level, gender + visible-items array (item_id only).
- `FriendsResult` enum exists with the 28-ish variants (need full audit).

**What's missing vs C++:**
- **Account-level ignore** — `_ignoredAccounts` set + `WowAccountGuid` capture on add — missing entirely. Ignoring an alt does NOT propagate to the other alts.
- **Mute (`SOCIAL_FLAG_MUTED`)** — unimplemented (would need voice-chat stack anyway, but flag persistence is missing).
- **`CMSG_SOCIAL_CONTRACT_REQUEST`** — unhandled. Some retail clients expect a response and stall the EULA prompt.
- **Friend-status presence broadcast** — when a player logs in/out/AFK/DND/zones, `SocialMgr::BroadcastToFriendListers` should push `FRIEND_STATUS_ONLINE/AFK/DND/OFFLINE` with new area/level. Rust does NOT broadcast these → friends never see status changes after initial list-fetch.
- **Enemy-faction check** — `AddFriend` to an enemy is currently allowed; should reply `FRIEND_ENEMY` (0x0A) per Trinity.
- **`character_social.accountGuid` column** — schema does not include it; Rust hard-codes `account_guid: ObjectGuid::EMPTY` everywhere.
- **Inspect-achievements** (`CMSG_QUERY_INSPECT_ACHIEVEMENTS` / `SMSG_RESPOND_INSPECT_ACHIEVEMENTS`) — unhandled.
- **Inspect-PvP** (`CMSG_INSPECT_PVP`) — unhandled.
- **Inspect-result enrichment** — no talent spec, no glyphs, no transmog, no enchant displays, no guild data, no specialization id.
- **Whisper-from-ignored drop** — `chat.rs` does NOT cross-check `PlayerSocial::HasIgnore` because the social state isn't loaded into session. Whispers from an ignored sender still reach the recipient.
- **Cross-realm contact (VirtualRealmAddress, native vs virtual)** — fields are passed through but never differentiated; cross-realm friends won't resolve correctly.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `SELECT … FROM characters WHERE name = ?` is case-sensitive depending on collation; Trinity uses `utf8_general_ci`. Verify schema collation else friend-add-by-typo will fail.
- `ContactListPkt.contacts` is built lazily but `QueryPlayerNamesResponse` is sent unconditionally even if empty list — minor wire bloat.
- `FriendStatusPkt.status: u8` is `1` for online, `0` for offline — does NOT include AFK (0x02)/DND (0x04)/RAF (0x08) bitflag composition.
- `InspectResult` has `target_name` — but C++ never sends the name (client looks it up); spec parity check needed via packet capture.
- Enemy-faction check still missing → opposite-faction AddFriend may succeed without the C++ `FRIEND_ENEMY` gate.
- Inspect handler accesses `entry.visible_items` — assumes the broadcast info is always populated; will crash with `ObjectGuid::EMPTY` if a stale registry entry survives logout.

**Tests existing:**
- 0 unit tests for friends/ignore/inspect in `crates/wow-world` or `crates/wow-social`.
- Some packet round-trip tests in `crates/wow-packet/src/packets/social.rs` (need verification).

---

## 9. Migration sub-tasks

- [ ] **#SOCIAL.1** Complete ignore chat-blocking integration. `CMSG_ADD_IGNORE`/`CMSG_DEL_IGNORE` are represented for per-character `flags=2`, but account-level ignore/chat suppression remain pending. Complejidad: **M**
- [ ] **#SOCIAL.2** Capture `WowAccountGuid` on AddIgnore (JOIN `characters c` ON account → `account` table → derive `accountGuid`); store in new `character_social.accountGuid` column. Complejidad: **M**
- [ ] **#SOCIAL.3** Add migration: `ALTER TABLE character_social ADD COLUMN accountGuid BIGINT UNSIGNED DEFAULT 0`. Complejidad: **L**
- [ ] **#SOCIAL.4** Build `PlayerSocial` in `crates/wow-social` with in-memory `_playerSocialMap` + `_ignoredAccounts`; load on character entry-world; persist diffs on save. Complejidad: **H**
- [x] **#SOCIAL.5** Implement `CMSG_SET_CONTACT_NOTES` — UPDATE `character_social` SET note with C++ 48-char truncation. Complejidad: **L**
- [ ] **#SOCIAL.6** Implement `CMSG_SOCIAL_CONTRACT_REQUEST` — reply with `SMSG_SOCIAL_CONTRACT_REQUEST_RESPONSE { showed_contract: false }` for now. Complejidad: **L**
- [ ] **#SOCIAL.7** Implement friend-status presence broadcast: on `WorldSession::login`, `logout`, `toggle_afk`, `toggle_dnd`, `change_zone`, call `SocialMgr::broadcast_to_friend_listers` to push `SMSG_FRIEND_STATUS`. Complejidad: **H**
- [x] **#SOCIAL.8** Enforce 50-entry caps for `AddFriend` (`SOCIALMGR_FRIEND_LIMIT`) and `AddIgnore` (`SOCIALMGR_IGNORE_LIMIT`). Complejidad: **L**
- [ ] **#SOCIAL.9** Add enemy-faction check on `AddFriend` — reply `FRIEND_ENEMY` (0x0A). Complejidad: **L**
- [ ] **#SOCIAL.10** Cross-reference ignore list in `handle_chat_whisper` and other chat paths — drop msg if recipient ignores sender (or sender's account). Complejidad: **M**
- [ ] **#SOCIAL.11** Enrich `SMSG_INSPECT_RESULT` — add talent-spec, glyphs, item enchants & gems display, transmog appearance, guild snapshot (`InspectGuildData`), specialization id. Complejidad: **H**
- [ ] **#SOCIAL.12** Implement `CMSG_QUERY_INSPECT_ACHIEVEMENTS` + `SMSG_RESPOND_INSPECT_ACHIEVEMENTS` (compressed achievement bitmap). Complejidad: **M**
- [ ] **#SOCIAL.13** Implement `CMSG_INSPECT_PVP` + `SMSG_INSPECT_PVP` (6-bracket arena rating array). Complejidad: **M**
- [ ] **#SOCIAL.14** Compose `FriendStatusPkt.status: u8` as full bitmask (`ONLINE|AFK|DND|RAF`) instead of bool 1/0. Complejidad: **L**

---

## 10. Regression tests to write

- [ ] Test: AddFriend self → replies `FRIEND_SELF` (0x09).
- [ ] Test: AddFriend already-friend → replies `FRIEND_ALREADY` (0x08).
- [ ] Test: AddFriend opposite faction → replies `FRIEND_ENEMY` (0x0A).
- [ ] Test: AddFriend at 50/50 → replies `FRIEND_LIST_FULL` (0x01).
- [ ] Test: AddIgnore alt-of-existing-ignored-account → both alts auto-blocked via `_ignoredAccounts`.
- [ ] Test: Whisper from ignored sender → recipient does NOT receive the chat packet, sender's session receives an inform.
- [ ] Test: Friend logging in → all online players who friend them receive `SMSG_FRIEND_STATUS{ONLINE}` within ~200ms.
- [ ] Test: Friend toggling AFK → followers receive new status with `FRIEND_STATUS_AFK` flag set.
- [ ] Test: SetContactNotes round-trips through DB and returns in next `SendSocialList`.
- [ ] Test: Inspect target's visible-items list matches their equipped items in slots 0..18.
- [ ] Test: Removing last flag (friend AND ignore both 0) DELETEs the row instead of leaving zero-flagged garbage.
- [ ] Test: Character deletion cascades — `CHAR_DEL_CHARACTER_SOCIAL_BY_FRIEND` removes the deleted char from everyone else's lists.

---

## 11. Notes / gotchas

- **`_ignoredAccounts` is a separate set from `_playerSocialMap`** — populated by walking the social map at load time, NOT by querying directly. After a `RemoveFromSocialList(IGNORED)` call, MUST recompute the set or you'll keep ignoring an alt whose char was unblocked.
- **`SOCIAL_FLAG_MUTED = 0x04`** — guessed/legacy; voice-chat is not implemented in 3.4.3 so leave as a no-op flag.
- **`FRIEND_IGNORE_AMBIGUOUS = 0x11`** — only emitted in cross-realm scenarios where multiple chars share a name. Single-realm RustyCore can ignore for now.
- **`FriendStatus` is a bitmask** (`ONLINE|AFK|DND|RAF`) — not an enum. Status of 0x05 means `ONLINE+DND`. Don't `if status == ONLINE` — use `status & ONLINE`.
- **Note column is 48 chars** in retail; longer notes cause client-side display truncation but no server enforcement. Recommend `VARCHAR(48)`.
- **Inspect target must be on the same map** in 3.4.3 (no cross-map inspect). Verify `target.map_id == self.current_map_id` before responding.
- **`SMSG_FRIEND_STATUS` broadcast vs single** — `result == FRIEND_ONLINE/OFFLINE` triggers broadcast to followers; `result == FRIEND_ADDED_*` is a single send to the action-taker only. Mixing them up causes presence storms.
- **VirtualRealmAddress** — for connected realms (3.4.3 supports this in cross-realm zones), set `virtual_realm_address` on contact entries to the friend's home realm; `native_realm_address` to the same value if same realm. Mismatch == cross-realm flag in client.
- **`CHAR_REP_CHARACTER_SOCIAL` (REPLACE INTO)** — `(guid, friend)` is the primary key; flags get OR'd by application logic, not by SQL. Rust now mirrors the flag-preserving behavior for `AddFriend`/`AddIgnore` with `ON DUPLICATE KEY UPDATE flags = flags | ...`.
- **Account-level ignore is essential for trolls**: if not implemented, /ignore is trivially defeated by relogging an alt.
- **Inspect leaks information** — `SMSG_INSPECT_RESULT` reveals talent build, gear, achievements. Some retail PvP servers gate this by `WORLD_CONFIG_ALLOW_INSPECT_OTHER_FACTION`.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class PlayerSocial` | `struct PlayerSocial` (en `crates/wow-social/src/player_social.rs` — TBD) | Owned by `WorldSession` or by character state |
| `class SocialMgr` (singleton) | `struct SocialMgr { sessions: DashMap<ObjectGuid, Arc<RwLock<PlayerSocial>>> }` | `Arc<SocialMgr>` in `WorldContext` |
| `struct FriendInfo` | `struct FriendInfo { account_guid: ObjectGuid, status: FriendStatus, flags: SocialFlags, area: u32, level: u8, class: u8, note: String }` | mirror exactly |
| `enum FriendStatus` (bitmask) | `bitflags! { struct FriendStatus: u8 { OFFLINE=0x00; ONLINE=0x01; AFK=0x02; DND=0x04; RAF=0x08; } }` | use `bitflags` crate |
| `enum SocialFlag` | `bitflags! { struct SocialFlags: u8 { FRIEND=0x01; IGNORED=0x02; MUTED=0x04; UNK=0x08; } }` | |
| `enum FriendsResult : uint8` | `#[repr(u8)] enum FriendsResult` (already in `wow-packet/social.rs`) | Audit count vs C++ (29 variants) |
| `_playerSocialMap: map<ObjectGuid, FriendInfo>` | `HashMap<ObjectGuid, FriendInfo>` | sorted-iter not required |
| `_ignoredAccounts: GuidUnorderedSet` | `HashSet<ObjectGuid>` | rebuilt on add/remove of ignore |
| `PlayerSocial::AddToSocialList(guid, accountGuid, flag)` | `fn add_to_social_list(&mut self, guid: ObjectGuid, account_guid: ObjectGuid, flag: SocialFlags) -> Result<bool>` | returns false if cap hit |
| `SocialMgr::BroadcastToFriendListers(player, packet)` | `async fn broadcast_to_friend_listers(&self, player_guid: ObjectGuid, bytes: Vec<u8>)` | iterates `sessions` |
| `SOCIALMGR_FRIEND_LIMIT` / `IGNORE_LIMIT` | `const FRIEND_LIMIT: usize = 50;` / `IGNORE_LIMIT: usize = 50;` | |
| `WorldPackets::Social::ContactInfo` | `struct ContactInfo { ... }` (already exists, audit fields) | |
| `WorldPackets::Inspect::InspectResult` | `struct InspectResult { ... }` (already exists, needs talents/glyphs/guild) | |

---

*Template version: 1.0 (2026-05-01).* Status: ⚠️ partial — friends ~80%, ignore 0%, inspect ~30%, presence broadcast 0%.

---

## 13. Audit (2026-05-01)

Side-by-side audit of `crates/wow-social/src/lib.rs` (empty) + `crates/wow-world/src/handlers/{social,inspect}.rs` vs `src/server/game/Handlers/SocialHandler.cpp` + `Entities/Player/SocialMgr.{h,cpp}`.

### Flagged divergence — verdict

**`/ignore` does not filter whispers — CONFIRMED.**
Two-part proof:

1. `crates/wow-world/src/handlers/social.rs` now registers per-character `AddIgnore`/`DelIgnore` and `SetContactNotes`, so flag bit 2 (`SOCIAL_FLAG_IGNORED`) can be written/cleared and contact notes can be edited. There is still no loaded `PlayerSocial`, no account-level `_ignoredAccounts`, and no `SocialContractRequest`.
2. `crates/wow-world/src/handlers/chat.rs:187-257` (`handle_chat_whisper`) looks up the target by name in `player_registry()`, builds a `ChatPkt::Whisper`, and unconditionally `tx.send(to_target.to_bytes())` (`chat.rs:227`). There is no membership check against any per-recipient ignore set. The corresponding C++ path (`ChatHandler.cpp` whisper) calls `player->GetSocial()->HasIgnore(senderGuid, senderAccount)` and short-circuits with `WORLD_PACKET_IGNORE_TYPE_*` if true.

Net effect after the represented `CMSG_ADD_IGNORE`/`CMSG_DEL_IGNORE` slices: clicking `Ignore Player` writes the per-character `SOCIAL_FLAG_IGNORED` row to `character_social`, and removing it clears only that flag like C++. `handle_chat_whisper` still does not consult that row and account-level ignore is still unavailable. The originator's ignore list can now persist/display, but chat blocking remains incomplete.

### Coverage matrix

| C++ opcode handler | Rust | Verdict |
|---|---|---|
| `HandleAddFriendOpcode` | ✅ name lookup, self/already/list-full gates, flag-preserving insert/update into `character_social` | partial |
| `HandleDelFriendOpcode` | ✅ `flags &= ~1`, deletes row only when empty | ok |
| `HandleContactListOpcode` | ✅ `social.rs:249-359` (JOIN `character_social` × `characters`, plus `QueryPlayerNamesResponse` for name cache) | ok |
| `HandleAddIgnoreOpcode` | ✅ represented per-character `flags=2`, self/already/full/not-found gates | partial |
| `HandleDelIgnoreOpcode` | ✅ represented per-character `flags &= ~2`, deletes row only when empty | partial |
| `HandleSetContactNotesOpcode` | ✅ updates `character_social.note`, truncates to 48 chars | ok |
| `HandleSocialContractRequest` | ❌ unregistered |  |
| `HandleInspectOpcode` | ✅ `inspect.rs:33-81` (race/class/level/gender + visible-items only) | partial |
| `HandleQueryInspectAchievements` | ❌ unregistered |  |
| `HandleInspectPVP` | ❌ unregistered |  |
| `SocialMgr::BroadcastToFriendListers` (presence updates on login/logout/AFK/DND/zone) | ❌ none | bug |
| `_ignoredAccounts` set + `WowAccountGuid` capture | ❌ none |  |

### Other observed bugs / divergences

- `social.rs` — AddFriend now ORs the friend bit into existing social rows, preserving ignore/mute flags. ✅
- `social.rs` — self-check returns `FriendsResult::Self_` with the target GUID like C++. ✅
- `social.rs:156` — already-friend check returns `Already`. ✅
- `social.rs:104` — `SELECT … FROM characters WHERE name = ?` collation is whatever the schema uses; no explicit `COLLATE utf8mb4_general_ci` clause. Behaviour depends on table default collation.
- `social.rs:128` — `friend_guid = ObjectGuid::create_player(0, friend_guid_raw)` hardcodes realm `0`; cross-realm friends not handled.
- `AddFriend` and `AddIgnore` enforce the C++ 50-entry caps. ✅
- No enemy-faction check — Alliance can friend Horde without `FRIEND_ENEMY (0x0A)`.
- `FriendStatusPkt.status: u8` set to `1` for online / `0` for offline (e.g. `social.rs:190`); never composes the `FriendStatus` bitmask `ONLINE|AFK|DND|RAF`. Followers will never see `AFK`/`DND` flags on a contact, even after AFK/DND is wired (which it isn't yet).
- `account_guid: ObjectGuid::EMPTY` everywhere (`social.rs:90,188,235`). `character_social` schema does not include the `accountGuid` column. Account-level ignore is structurally impossible until both schema and capture are added.
- `inspect.rs:60-78` — only emits `slot + item_id`. No enchant id, no gem ids, no transmog appearance, no talent spec, no glyphs, no guild data, no specialization id, no PvP brackets.
- No `InspectGuildData`, no `InspectTalentData`, no `PVPBracketData`. The four C++ inspect packet structs are stubbed to one (`InspectResult`).
- `crates/wow-social/src/lib.rs` confirmed 0 bytes; no `PlayerSocial`, no `SocialMgr` anywhere in the workspace.

**Verdict:** flagged divergence partly reduced. Friends list works for add/delete/list (~50% of `SocialHandler.cpp`). Per-character `AddIgnore`/`DelIgnore` now write and clear `SOCIAL_FLAG_IGNORED`, but account-level ignore, loaded `PlayerSocial`, and whisper/chat suppression remain open. Inspect is ~25% (basic items + identity); achievements/PvP/talents/glyphs absent. Presence broadcast (`SMSG_FRIEND_STATUS` on login/logout/AFK/zone) is 0%.
