# Migration: Chat

> **C++ canonical path:** `src/server/game/Chat/` + `src/server/game/Handlers/ChatHandler.cpp` + `src/server/game/Handlers/ChannelHandler.cpp`
> **Rust target crate(s):** `crates/wow-chat/` (empty placeholder), `crates/wow-world/src/handlers/chat.rs`, `crates/wow-packet/src/packets/chat.rs`
> **Layer:** L6
> **Status:** ⚠️ partial (~32% — say/yell/whisper/emote work; Party/Raid/RaidWarning/InstanceChat now use group membership routing; group addon Party/Raid/InstanceChat routing represented; ignored-report and AFK/DND status toggles represented; Guild/Officer, channels, targeted addon routing, hyperlinks, languages still missing)
> **Audited vs C++:** ✅ audited 2026-05-01 (§13)
> **Last updated:** 2026-06-11

---

## 1. Purpose

Player-to-player text communication. Covers proximity chat (Say/Yell/Emote), targeted chat (Whisper, Party, Raid, Guild, Officer, InstanceChat), global named channels (Trade, General, LookingForGroup, world-defense, custom user channels), addon binary messages, hyperlink validation, language enforcement (race-restricted speech), AFK/DND auto-replies, and per-target chat-spam reporting (`CMSG_CHAT_REPORT_IGNORED`). Channels also carry their own moderator/owner/ban-list lifecycle persisted to `channels` table.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Chat/Channels/Channel.cpp` | 1026 | `prefix` |
| `game/Chat/Channels/Channel.h` | 271 | `prefix` |
| `game/Chat/Channels/ChannelAppenders.h` | 476 | `prefix` |
| `game/Chat/Channels/ChannelMgr.cpp` | 287 | `prefix` |
| `game/Chat/Channels/ChannelMgr.h` | 68 | `prefix` |
| `game/Chat/Channels/enuminfo_Channel.cpp` | 172 | `prefix` |
| `game/Chat/Chat.cpp` | 795 | `prefix` |
| `game/Chat/Chat.h` | 168 | `prefix` |
| `game/Chat/ChatCommands/ChatCommand.cpp` | 482 | `prefix` |
| `game/Chat/ChatCommands/ChatCommand.h` | 280 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandArgs.cpp` | 136 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandArgs.h` | 338 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandHelpers.cpp` | 30 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandHelpers.h` | 133 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandTags.cpp` | 155 | `prefix` |
| `game/Chat/ChatCommands/ChatCommandTags.h` | 326 | `prefix` |
| `game/Chat/HyperlinkTags.cpp` | 490 | `prefix` |
| `game/Chat/Hyperlinks.cpp` | 730 | `prefix` |
| `game/Chat/Hyperlinks.h` | 549 | `prefix` |
| `game/Chat/LanguageMgr.cpp` | 282 | `prefix` |
| `game/Chat/LanguageMgr.h` | 99 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Chat/Chat.h` | 168 | `ChatHandler` class — global GM/system message helpers + colour codes |
| `src/server/game/Chat/Chat.cpp` | 795 | `ChatHandler` implementation, `BuildChatPacket`, `SendSysMessage`, hyperlink expansion |
| `src/server/game/Chat/Hyperlinks.h` | 549 | Hyperlink tag definitions (item, spell, quest, achievement, journal…) |
| `src/server/game/Chat/Hyperlinks.cpp` | 730 | `Hyperlinks::CheckAllLinks` — validates client-supplied chat hyperlinks against DBC/sObjectMgr |
| `src/server/game/Chat/HyperlinkTags.cpp` | 490 | Per-tag parser + cross-reference lookup |
| `src/server/game/Chat/LanguageMgr.h` | 99 | Language word-substitution mgr |
| `src/server/game/Chat/LanguageMgr.cpp` | 282 | Loads `Languages.dbc` + `LanguageWords.dbc`, scrambles foreign-language text |
| `src/server/game/Chat/Channels/Channel.h` | 271 | `Channel` class, `ChatNotify`, `ChannelFlags`, `ChannelMemberFlags` enums |
| `src/server/game/Chat/Channels/Channel.cpp` | 1026 | Full channel state machine (join/leave/kick/ban/mod/owner/password/announce/list) |
| `src/server/game/Chat/Channels/ChannelMgr.h` | 68 | `ChannelMgr` per-team singleton (Alliance + Horde) |
| `src/server/game/Chat/Channels/ChannelMgr.cpp` | 287 | Channel registry, DBC-built-in channel resolution, persistence load/save |
| `src/server/game/Chat/Channels/ChannelAppenders.h` | 476 | `ChannelNameBuilder<...>` template helpers for building `SMSG_CHANNEL_NOTIFY` packets |
| `src/server/game/Chat/ChatCommands/ChatCommand.{h,cpp}` | ~600 | `.gm`-style chat command registration framework (lambda/templated dispatch) |
| `src/server/game/Chat/ChatCommands/ChatCommandArgs.{h,cpp}` | ~700 | Typed argument parser for chat commands (Hyperlink, Achievement, Quoted, Optional…) |
| `src/server/game/Chat/ChatCommands/ChatCommandTags.{h,cpp}` | ~400 | Tag-types for command params (PlayerIdentifier, AchievementId…) |
| `src/server/game/Handlers/ChatHandler.cpp` | 830 | All `CMSG_CHAT_MESSAGE_*` / `CMSG_EMOTE` / `CMSG_TEXT_EMOTE` / AFK / DND / addon dispatches |
| `src/server/game/Handlers/ChannelHandler.cpp` | 219 | `CMSG_CHAT_*_CHANNEL`, `CMSG_CHANNEL_PASSWORD`, `CMSG_CHANNEL_COMMAND`, `CMSG_CHANNEL_PLAYER_COMMAND` |
| `src/server/game/Miscellaneous/SharedDefines.h` (5877-5949) | ~75 | `enum ChatMsg : int32` (66 values, `CHAT_MSG_ADDON = -1` … `CHAT_MSG_VOICE_TEXT = 0x42`) |
| `src/server/game/Miscellaneous/SharedDefines.h` (1078-1130) | ~55 | `enum Language` (35 values, includes `LANG_UNIVERSAL`, `LANG_ADDON`, `LANG_ADDON_LOGGED`) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ChatHandler` | class | Wraps a `WorldSession*` to send formatted/colored system text and execute `.commands`; holds a `Player*` accessor |
| `Channel` | class | One named channel — owner, password, member map, ban list, moderation flags, dirty-DB tracking |
| `Channel::PlayerInfo` | nested struct | Per-channel-membership flags (Owner/Moderator/Voiced/Muted/Mic-Muted/Custom/Invisible) |
| `ChannelMgr` | class | Per-team (Alliance/Horde) registry of all channels (built-in zone channels + custom user channels) |
| `LanguageMgr` | class | Singleton loading `Languages.dbc` + `LanguageWords.dbc`; performs deterministic text-scrambling for unknown languages |
| `LanguageDesc` | struct | One language entry (id, skillId, race-restriction) |
| `Hyperlinks::HyperlinkInfo` | struct | Parsed tag (kind + colour + payload + display-text) |
| `ChatCommand` | class | Registered chat slash-command (name, security level, handler, sub-commands) |
| `enum ChatMsg : int32` | enum | 66 values — message channel types, including all `CHAT_MSG_*` |
| `enum Language` | enum | 35 values — `LANG_UNIVERSAL=0`, race tongues, `LANG_ADDON=183`, `LANG_ADDON_LOGGED=184` |
| `enum ChatFlags` | enum | bitmask — AFK/DND/GM/COM/DEV/BOSS_SOUND/MOBILE/GUIDE/NEWCOMER/CENSORED |
| `enum ChatNotify : uint8` | enum | 38 values — channel-event subtypes for `SMSG_CHANNEL_NOTIFY` |
| `enum ChannelFlags : uint8` | enum | Custom/Trade/NotLFG/General/City/LFG/Voice |
| `enum ChannelMemberFlags : uint8` | enum | Owner/Moderator/Voiced/Muted/Custom/MicMuted |
| `enum ChatLinkColors : uint32` | enum | Per-link-kind ARGB colours (orange items, blue spells, …) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `WorldSession::HandleChatMessageOpcode(ChatMessage&)` | Dispatches CMSG_CHAT_MESSAGE_SAY/YELL/PARTY/RAID/GUILD/OFFICER/RAID_WARNING/INSTANCE_CHAT to `HandleChatMessage(type, lang, msg)` | `Player::Say/Yell`, `Group::BroadcastPacket`, `Guild::BroadcastToGuild` |
| `WorldSession::HandleChatMessage(ChatMsg, Language, std::string, target, channelGuid)` | Core sender — applies language scrambling, hyperlink validation, profanity, GM mute, builds `ChatPkt`, routes to recipients | `LanguageMgr::Verify`, `Hyperlinks::CheckAllLinks`, `Channel::Say` |
| `WorldSession::HandleChatMessageWhisperOpcode` | Whisper to a named target (cross-realm aware via VirtualRealmAddress) | `ObjectAccessor::FindConnectedPlayerByName`, `ChatHandler::SendSysMessage` |
| `WorldSession::HandleChatAddonMessageOpcode` | Addon binary blob — uses `LANG_ADDON`/`LANG_ADDON_LOGGED`, prefix string, isLogged | `Channel::AddonSay`, `Group::BroadcastAddonMessagePacket` |
| `WorldSession::HandleChatMessageAFKOpcode` / `HandleChatMessageDNDOpcode` | Toggles `PLAYER_FLAGS_AFK`/`DND`, stores auto-reply text | `Player::ToggleAFK/DND` |
| `WorldSession::HandleEmoteOpcode` | Client clears its emote state — server resets `UNIT_FIELD_NPC_EMOTESTATE` | `Unit::SetEmoteState` |
| `WorldSession::HandleTextEmoteOpcode(CTextEmote&)` | Performs `/wave`-style emote — looks up `EmotesText.db2`, broadcasts `STextEmote` + `EmoteMessage` | `EmotesTextStore.LookupEntry`, `Player::HandleEmoteCommand` |
| `WorldSession::HandleChatIgnoredOpcode` | Client reports they're ignoring sender — informs sender's session | `ChatHandler::SendSysMessage` |
| `WorldSession::HandleJoinChannel(JoinChannel&)` | Resolve channelId via `ChatChannelsStore`/`AreaTableStore` zone-restriction, defer to `ChannelMgr::GetChannel(true)`, then `Channel::JoinChannel` | `ChannelMgr::GetChannelForPlayerByNamePart`, `Channel::JoinChannel` |
| `WorldSession::HandleLeaveChannel` | Removes player from channel; possibly destroys empty custom channel | `Channel::LeaveChannel`, `ChannelMgr::LeftChannel` |
| `WorldSession::HandleChannelCommand` | List/Announce/SetOwner/Kick/Mute/etc. (sub-dispatched on `command` enum) | `Channel::List`, `Channel::Announce`, `Channel::KickOrBan`, `Channel::SetOwner` |
| `WorldSession::HandleChannelPlayerCommand` | Two-arg variants: `Kick player`, `Ban player`, `Invite player`, `Owner set`, `ModeratorAdd/Remove`, `MuteAdd/Remove`, `UnBan`, `SilenceAll`, `UnsilenceAll` | `Channel::Kick/Ban/Invite/SetMode` |
| `WorldSession::HandleChannelPassword` | Sets/changes channel password | `Channel::Password` |
| `Channel::JoinChannel(Player*, password)` | Validates password/ban/area/LFG rules, inserts into `_playersStore`, sends `JoinedNotice` + `WhoOwner` if first-joiner | `ChannelMgr::SaveToDB`, `SendToAll<JoinedAppender>` |
| `Channel::Say(guid, text, lang)` | Broadcasts a chat msg to all members; rate-limits via `_nextActivityUpdateTime` if throttled | `Player::isGMVisible`, `SendToAll` |
| `Channel::KickOrBan(player, badname, ban)` | Mod/Owner-only; if `ban==true` adds to `_bannedStore` and persists; unconditionally `LeaveChannel` | `Channel::LeaveChannel`, DB `CHAR_UPD_CHANNEL` |
| `Channel::SetMode(player, target, mod, set)` | Toggles moderator OR mute flag on a member; emits `ModeChange` notice | `SendToAll<ModeChangeAppender>` |
| `Channel::SetOwner(guid, exclaim)` | Transfers ownership; previous owner loses `MEMBER_FLAG_OWNER` | `SendToAll<OwnerChangedAppender>` |
| `Channel::List(player)` | Sends `SMSG_CHANNEL_LIST` listing every member + flags | `SendToOne<ListAppender>` |
| `ChannelMgr::GetChannel(channelId, name, player, notify, zoneEntry)` | Looks up or creates a built-in zone channel; respects `WORLD_CONFIG_PRESERVE_CUSTOM_CHANNELS`/team-split | DB `CHAR_INS_CHANNEL`, `Channel` ctor |
| `ChannelMgr::LoadFromDB()` | At startup, restores persisted custom channels (name, team, password, ban list) | DB `SEL_CHANNELS` |
| `LanguageMgr::Verify(player, lang, msg)` | Checks player can speak language (race/skill); else returns scrambled string | `LanguageWordsStore` |
| `Hyperlinks::CheckAllLinks(msg)` | Walks `\|H...\|h...\|h\|r` sequences, validates payload against game data; rejects unknown/forged links | per-tag `LinkValidators` |
| `ChatHandler::SendSysMessage(string)` | Sends `CHAT_MSG_SYSTEM` to one session in current locale | `BuildChatPacket` |
| `ChatHandler::ParseCommands(text)` | Detects leading `.`/`!`, dispatches to registered `ChatCommand` tree | `ChatCommand::Invoke` |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Player** — sender/recipient identity, `PLAYER_FLAGS_AFK/DND/GM`, race→language map
- **DBC/DB2 stores** — `ChatChannelsStore`, `Languages.db2`, `LanguageWords.db2`, `EmotesText.db2`, `AreaTableStore` for zone-restricted channels
- **CharacterDatabase** — `channels` table (custom channel persistence) + `character_social` (whisper-block via ignore list — read by `ChatHandler` to drop whispers from ignored senders)
- **Globals/ObjectAccessor** — find target Player by name for whispers
- **Groups** — `Group::BroadcastPacket` for party/raid/instance-chat
- **Guilds** — `Guild::BroadcastToGuild`/`BroadcastToOfficer` for guild/officer chat
- **Server/WorldSocket** — `SendPacket` + locale negotiation
- **Hyperlinks** — content validation against ItemTemplate, SpellInfo, AchievementEntry, QuestTemplate, JournalInstanceEntry
- **Conditions** — chat-spam-prevention conditions (CharacterDatabase.world_chat_filter)

**Depended on by:**
- **Player.cpp** — `Player::Say/Yell/TextEmote/Whisper` build packets and dispatch
- **Battleground** — `BG_TEXT_*` system messages routed via `ChatHandler::SendSysMessage`
- **Scripting** — script API exposes `Player::Whisper/Say` to Lua/C++ scripts
- **GM commands** (`.gm visible`, `.silence`, `.mute`) — manipulate Player chat flags
- **AchievementMgr** — broadcasts achievement-earned to guild via `CHAT_MSG_GUILD_ACHIEVEMENT`

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_UPD_CHANNEL` (INSERT…ON DUP KEY UPD) | Save/update custom channel (name, team, announce, ownership, password, banList, lastUsed) | character |
| `CHAR_UPD_CHANNEL_USAGE` | Touch channel `lastUsed` on activity | character |
| `CHAR_UPD_CHANNEL_OWNERSHIP` | Toggle `ownership` for a channel by name | character |
| `CHAR_DEL_CHANNEL` | Delete row when channel destroyed | character |
| `SEL_CHANNELS` (raw query in `ChannelMgr::LoadFromDB`) | Bulk-load persisted channels at startup | character |
| Direct `SELECT` of `character_social` flags | Drop whisper from sender on recipient ignore list | character |

DBC/DB2 stores read:

| Store | What it loads | Read by |
|---|---|---|
| `ChatChannelsStore` (ChatChannels.db2) | Built-in channel templates: id, flags, name pattern, factionId | `ChannelMgr::GetChannel`, `JoinChannel` opcode |
| `LanguagesStore` (Languages.db2) | LangId → name, skillLine | `LanguageMgr::LoadLanguages` |
| `LanguageWordsStore` (LanguageWords.db2) | Per-language pseudo-words for scrambling | `LanguageMgr::Verify` |
| `EmotesTextStore` (EmotesText.db2) | `/wave`,`/dance` → emote-anim ID + sound index | `HandleTextEmoteOpcode` |
| `AreaTableStore` | Zone restriction for built-in channels (LocalDefense, etc.) | `JoinChannel` opcode |
| `ChrRacesStore` | Race → default-language mapping | `LanguageMgr` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_CHAT_MESSAGE_SAY` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_YELL` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_PARTY` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_RAID` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_RAID_WARNING` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_GUILD` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_OFFICER` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_INSTANCE_CHAT` | C→S | `HandleChatMessageOpcode` |
| `CMSG_CHAT_MESSAGE_WHISPER` | C→S | `HandleChatMessageWhisperOpcode` |
| `CMSG_CHAT_MESSAGE_CHANNEL` | C→S | `HandleChatMessageChannelOpcode` |
| `CMSG_CHAT_MESSAGE_EMOTE` | C→S | `HandleChatMessageEmoteOpcode` |
| `CMSG_CHAT_MESSAGE_AFK` | C→S | `HandleChatMessageAFKOpcode` |
| `CMSG_CHAT_MESSAGE_DND` | C→S | `HandleChatMessageDNDOpcode` |
| `CMSG_CHAT_ADDON_MESSAGE` | C→S | `HandleChatAddonMessageOpcode` |
| `CMSG_CHAT_ADDON_MESSAGE_TARGETED` | C→S | `HandleChatAddonMessageTargetedOpcode` |
| `CMSG_CHAT_ADDON_MESSAGE_WHISPER` | C→S | (subset) |
| `CMSG_CHAT_REPORT_IGNORED` | C→S | `HandleChatIgnoredOpcode` |
| `CMSG_EMOTE` | C→S | `HandleEmoteOpcode` |
| `CMSG_SEND_TEXT_EMOTE` | C→S | `HandleTextEmoteOpcode` |
| `CMSG_CHAT_JOIN_CHANNEL` | C→S | `HandleJoinChannel` |
| `CMSG_CHAT_LEAVE_CHANNEL` | C→S | `HandleLeaveChannel` |
| `CMSG_CHAT_CHANNEL_COMMAND` | C→S | `HandleChannelCommand` (Announce/List/SetOwner/Kick…) |
| `CMSG_CHAT_CHANNEL_PLAYER_COMMAND` | C→S | `HandleChannelPlayerCommand` (target-bearing variants) |
| `CMSG_CHAT_CHANNEL_PASSWORD` | C→S | `HandleChannelPassword` |
| `CMSG_CHAT_CHANNEL_DECLINE_INVITE` | C→S | `Channel::DeclineInvite` |
| `SMSG_CHAT` | S→C | `Player::BuildPlayerChat` / `BuildChatPacket` |
| `SMSG_CHANNEL_NOTIFY` | S→C | `ChannelAppenders.h` builders (Joined/Left/ModeChange/PasswordChanged/OwnerChanged/Banned/Kicked…) |
| `SMSG_CHANNEL_NOTIFY_JOINED` | S→C | First-time join notice |
| `SMSG_CHANNEL_NOTIFY_LEFT` | S→C | Left notice |
| `SMSG_CHANNEL_LIST` | S→C | `Channel::List` |
| `SMSG_CHAT_PLAYER_AMBIGUOUS` | S→C | Whisper resolves to multiple cross-realm players |
| `SMSG_DEFENSE_MESSAGE` | S→C | World-defense channel (special routing) |
| `SMSG_TEXT_EMOTE` | S→C | `HandleTextEmoteOpcode` |
| `SMSG_EMOTE` | S→C | `Unit::HandleEmoteCommand` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 413 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/chat.rs` | `file` | 1 | 351 | `exists_active` | file exists |
| `crates/wow-chat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-chat/src/lib.rs` — **0 lines** (empty crate stub; should host `Channel`, `ChannelMgr`, `LanguageMgr`, `Hyperlinks`)
- `crates/wow-world/src/handlers/chat.rs` — 1182 lines — covers ~32% of `Handlers/ChatHandler.cpp`
- `crates/wow-packet/src/packets/chat.rs` — 351 lines — `ChatMessage`, `ChatMessageWhisper`, `ChatMessageEmote`, `ChatPkt`, `EmoteMessage`, `STextEmote`, `CTextEmote`, `EmoteClient`, `ChatMsg` enum

**What's implemented:**
- `CMSG_CHAT_MESSAGE_SAY` / `_YELL` — proximity broadcast within range (25y say/emote, 300y yell).
- `CMSG_CHAT_MESSAGE_PARTY` / `_RAID` / `_RAID_WARNING` / `_INSTANCE_CHAT` — route through `GroupRegistry` + `PlayerRegistry`, with C++-style subgroup filtering for party, leader message variants, raid-only gates, and raid-warning leader/assistant gates.
- `CMSG_CHAT_MESSAGE_GUILD` — intentionally no-op until `GuildRegistry`/`Guild::BroadcastToGuild` is ported; this avoids the previous proximity leak but is still missing guild delivery.
- `CMSG_CHAT_MESSAGE_WHISPER` — name lookup via `PlayerRegistry`; sends `Whisper` to target + `WhisperInform` echo to sender; if offline, only the inform is sent (no offline-friendly whisper queue).
- `CMSG_CHAT_MESSAGE_EMOTE` — broadcasts `CHAT_MSG_EMOTE` packet at 25y range.
- `CMSG_EMOTE` — parsed and logged; no emote-state-machine update on `Unit`.
- `CMSG_SEND_TEXT_EMOTE` — pass-through: emits `STextEmote` + `EmoteMessage` with raw client-supplied EmoteId (no EmotesText.db2 lookup, no anim-id resolution, no sound).
- `ChatMsg` Rust enum exists in `wow-packet::packets::chat` (subset — needs full audit count).

**What's missing vs C++:**
- **Channels system entirely absent** — no `Channel`, no `ChannelMgr`, no DB persistence, no `CMSG_CHAT_JOIN_CHANNEL`, no `LEAVE`, no `COMMAND`/`PLAYER_COMMAND`, no `PASSWORD`, no `DECLINE_INVITE`, no `SMSG_CHANNEL_NOTIFY*` family.
- **Built-in channels** (Trade, General, LookingForGroup, GuildRecruitment, LocalDefense) — none auto-joined on zone change.
- **Custom user channels** — no creation/destruction, no `channels` table read/write.
- **Moderator/Owner/Banlist** — no `MEMBER_FLAG_*` enforcement, no `KickOrBan`, no `SetMode`, no `SetOwner`, no `Announce` toggle.
- **Languages** — `LanguageMgr` not ported; speech is never scrambled, addon lang (183/184) never validated.
- **Hyperlinks** — `Hyperlinks::CheckAllLinks` not ported; client-supplied chat hyperlinks pass through untrusted (forgery vector).
- **Addon messages** — `CMSG_CHAT_ADDON_MESSAGE` now routes Party/Raid/InstanceChat addon payloads through group membership and receiver-side addon-prefix filtering. Guild/Officer/Channel addon routing and targeted/whisper addon packets remain absent.
- **AFK/DND** — `CMSG_CHAT_MESSAGE_AFK/DND` now toggles canonical `PLAYER_FLAGS_AFK/DND` and stores represented auto-reply text, but full C++ validation (`ValidateMessage`, hyperlink kick), `GM_SILENCE_AURA`, guild away event, script hook, localized default strings, battleground side-effect, and auto-reply delivery remain missing.
- **Whisper offline queue** — no fallback, no `BN_WHISPER_PLAYER_OFFLINE`.
- **Cross-realm whisper resolution (VirtualRealmAddress)** — partially wired (`virtual_realm_address` field passed) but no name-disambiguation or `SMSG_CHAT_PLAYER_AMBIGUOUS`.
- **Guild / Officer routing** — `GuildRegistry`/`Guild::BroadcastToGuild` and `BroadcastToOfficer` are still absent. Guild currently drops rather than proximity-leaking.
- **Group routing residuals** — Party/Raid/RaidWarning/InstanceChat now use group membership routing, but battleground original-group selection, non-default `PartyRaidWarnings` config support, language validation, and full chat-validation gates remain missing.
- **Chat commands (`.gm`, `!command`)** — no `ChatCommand` registry, no command parser, no security-level enforcement.
- **Profanity / spam filter / mute aura (GM_SILENCE_AURA 1852)** — no enforcement.
- **`SMSG_CHAT_PLAYER_AMBIGUOUS`, `SMSG_DEFENSE_MESSAGE`** — never sent.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `Guild`/`Officer` delivery remains absent because there is no `GuildRegistry`; current Rust drops guild chat instead of leaking it by proximity. `Party`/`Raid`/`RaidWarning`/`InstanceChat` no longer use proximity routing.
- No GM mute or rate-limit. A scripted client can flood at line speed.
- `ChatPkt.virtual_realm` is filled but receivers may not have `realm` field consumed in client → silent client-side rejection unlikely but worth checking with packet dump.
- Whisper performs case-insensitive name match across the entire registry every call — O(N) on player count; needs a `name → guid` index.
- `RANGE_YELL = 300.0` — Trinity uses `WORLD_CONFIG_LISTEN_RANGE_YELL` (default 300y) so this matches by coincidence, but should be config-driven.

**Tests existing:**
- 0 unit tests for chat handlers in `crates/wow-world` or `crates/wow-chat`.
- Some packet round-trip tests in `crates/wow-packet/src/packets/chat.rs` (need verification).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#CHAT.WBS.001** Partir y cerrar la migracion auditada de `game/Chat/Channels/Channel.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1026 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CHAT.WBS.002** Cerrar la migracion auditada de `game/Chat/Channels/Channel.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.003** Cerrar la migracion auditada de `game/Chat/Channels/ChannelAppenders.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/ChannelAppenders.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.004** Cerrar la migracion auditada de `game/Chat/Channels/ChannelMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/ChannelMgr.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.005** Cerrar la migracion auditada de `game/Chat/Channels/ChannelMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/ChannelMgr.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.006** Cerrar la migracion auditada de `game/Chat/Channels/enuminfo_Channel.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/enuminfo_Channel.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.007** Partir y cerrar la migracion auditada de `game/Chat/Chat.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 795 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CHAT.WBS.008** Cerrar la migracion auditada de `game/Chat/Chat.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.009** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommand.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommand.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.010** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommand.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommand.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.011** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandArgs.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandArgs.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.012** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandArgs.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandArgs.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.013** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandHelpers.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandHelpers.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.014** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandHelpers.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandHelpers.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.015** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandTags.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandTags.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.016** Cerrar la migracion auditada de `game/Chat/ChatCommands/ChatCommandTags.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/ChatCommands/ChatCommandTags.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.017** Cerrar la migracion auditada de `game/Chat/HyperlinkTags.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/HyperlinkTags.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.018** Partir y cerrar la migracion auditada de `game/Chat/Hyperlinks.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 730 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CHAT.WBS.019** Partir y cerrar la migracion auditada de `game/Chat/Hyperlinks.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 549 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#CHAT.WBS.020** Cerrar la migracion auditada de `game/Chat/LanguageMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/LanguageMgr.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CHAT.WBS.021** Cerrar la migracion auditada de `game/Chat/LanguageMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/LanguageMgr.h`
  Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#CHAT.1** Move `ChatMsg` enum to `wow-constants` (full 66-value parity with `SharedDefines.h`). Complejidad: **L**
- [ ] **#CHAT.2** Move `Language` enum + `ChatFlags` enum to `wow-constants`. Complejidad: **L**
- [x] **#CHAT.3a** Fix Party/Raid/RaidWarning/InstanceChat routing — use `GroupRegistry`/`PlayerRegistry` membership delivery instead of proximity; preserve party subgroup filtering, raid-only gates, leader variants, and raid-warning leader/assistant gate. Complejidad: **M**
- [ ] **#CHAT.3b** Port Guild/Officer routing — add `GuildRegistry`/`Guild::BroadcastToGuild`/`BroadcastToOfficer`; keep delivery out of proximity routing. Complejidad: **M**
- [ ] **#CHAT.3c** Finish group-chat residuals — battleground original-group selection, non-default `PartyRaidWarnings` config, language validation, and full C++ chat-validation gates. Complejidad: **M**
- [ ] **#CHAT.4** Implement `wow-chat::Channel` struct (members map, banned set, password, flags, owner GUID, dirty flag). Complejidad: **M**
- [ ] **#CHAT.5** Implement `wow-chat::ChannelMgr` (per-team registries, name-indexed), wire into `WorldContext`. Complejidad: **M**
- [ ] **#CHAT.6** Add `channels` table to character DB schema; load on startup, persist on dirty interval (mirror `CHAR_UPD_CHANNEL` family). Complejidad: **M**
- [ ] **#CHAT.7** Handle `CMSG_CHAT_JOIN_CHANNEL` — resolve built-in channelId via `ChatChannels.db2`, area-restriction check, password check. Complejidad: **H**
- [ ] **#CHAT.8** Handle `CMSG_CHAT_LEAVE_CHANNEL` — remove member, transfer ownership if owner left, destroy channel if empty + non-constant. Complejidad: **M**
- [ ] **#CHAT.9** Handle `CMSG_CHAT_CHANNEL_COMMAND` (List/Announce/Owner/...) — full sub-dispatch. Complejidad: **H**
- [ ] **#CHAT.10** Handle `CMSG_CHAT_CHANNEL_PLAYER_COMMAND` (Kick/Ban/UnBan/Invite/Mod/Mute/SilenceAll/...). Complejidad: **H**
- [ ] **#CHAT.11** Handle `CMSG_CHAT_CHANNEL_PASSWORD`. Complejidad: **L**
- [ ] **#CHAT.12** Build `SMSG_CHANNEL_NOTIFY` family (38 ChatNotify subtypes) in `wow-packet::packets::chat`. Complejidad: **M**
- [ ] **#CHAT.13** Build `SMSG_CHANNEL_LIST` packet + member-flag serialisation. Complejidad: **M**
- [x] **#CHAT.14a** Parse and register `CMSG_CHAT_MESSAGE_AFK`/`_DND`; toggle canonical `PLAYER_FLAGS_AFK/DND`, keep them mutually exclusive, and store represented auto-reply text. Complejidad: **L**
- [ ] **#CHAT.14b** Complete C++ AFK/DND side effects: `ValidateMessage`, `ValidateHyperlinksAndMaybeKick`, `GM_SILENCE_AURA`, guild away event, script hook, localized defaults, battleground leave, and actual auto-reply delivery. Complejidad: **M**
- [x] **#CHAT.15** Implement `CMSG_CHAT_REPORT_IGNORED` — inform sender they were ignored. Complejidad: **L**
- [x] **#CHAT.16a** Implement group addon routing for `CMSG_CHAT_ADDON_MESSAGE` Party/Raid/InstanceChat — build `SMSG_CHAT` with real addon prefix, `LANG_ADDON`/`LANG_ADDON_LOGGED`, no sender echo, party subgroup filtering, and receiver-side `IsAddonRegistered(prefix)` gate. Complejidad: **M**
- [ ] **#CHAT.16b** Implement Guild/Officer/Channel addon routing once `GuildRegistry`/`ChannelMgr` exist. Complejidad: **M**
- [ ] **#CHAT.16c** Implement targeted/whisper addon packet family (`CMSG_CHAT_ADDON_MESSAGE_TARGETED`/whisper) and cross-realm/name resolution. Complejidad: **M**
- [ ] **#CHAT.17** Port `LanguageMgr` — load `Languages.db2`+`LanguageWords.db2`, scramble text for unknown-language listeners. Complejidad: **H**
- [ ] **#CHAT.18** Port `Hyperlinks::CheckAllLinks` — full tag table (`item:`, `quest:`, `spell:`, `achievement:`, `talent:`, `enchant:`, `journal:`, `transmog:`, etc.) — drop msg if any link forged. Complejidad: **XL** (split per-tag)
- [ ] **#CHAT.19** Port `EmotesText.db2` lookup so `/wave` resolves to correct emote-anim-id + sound. Complejidad: **M**
- [ ] **#CHAT.20** Port `ChatCommand` registry + parser (security-level gated `.commands`). Complejidad: **XL** (split per-command-group; minimum: `.help`, `.gps`, `.tele`, `.kick`)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#CHAT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp`. Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-chat && cargo test -p wow-packet && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#CHAT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp`. Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#CHAT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp`. Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#CHAT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp`. Rust target: `crates/wow-chat`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `CHAT_MSG_PARTY` is delivered ONLY to `GroupInfo.members`, never to non-group players within 25y on same map.
- [ ] Test: `CHAT_MSG_GUILD` is delivered ONLY to guild members regardless of map.
- [ ] Test: `CHAT_MSG_YELL` reaches players exactly within 300y, excluding 301y+.
- [ ] Test: Whisper to offline name produces `WhisperInform` with target name echoed back, not a chat-bubble on sender.
- [ ] Test: Joining a `Trade` built-in channel auto-joins all members in same city zone.
- [ ] Test: Channel owner kicking a member produces `CHAT_PLAYER_KICKED_NOTICE` + member receives `CHAT_LEFT_NOTICE`.
- [ ] Test: Ban-list survives channel destruction-and-recreation if persistent.
- [ ] Test: `LANG_ORCISH` text spoken by a Human is scrambled deterministically for Alliance listeners (same seed → same scramble).
- [ ] Test: `CMSG_CHAT_REPORT_IGNORED` triggers `CHAT_MSG_IGNORED` reply to sender.
- [ ] Test: AFK toggle sets `PLAYER_FLAGS_AFK` (0x02) on `UNIT_FIELD_FLAGS` correctly.
- [ ] Test: Addon msg with `LANG_ADDON_LOGGED` (184) is never delivered to non-addon recipients but IS logged for moderation.
- [ ] Test: Hyperlink with item-id NOT present in `ItemTemplate` causes whole chat msg to be dropped.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `crates/wow-chat/` (empty placeholder), `crates/wow-world/src/handlers/chat.rs`, `crates/wow-packet/src/packets/chat.rs` \| ⚠️ partial (~32% — say/yell/whisper/emote work; Party/Raid/RaidWarning/InstanceChat and group addon routing are represented; ignored-report and AFK/DND status toggles represented; Guild/Officer, channels, targeted addon routing, hyperlinks, languages still missing) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#CHAT.DIV.001` | `crates/wow-chat` (`exists_empty`, 0 Rust lines) | 21 C++ files / 7293 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#CHAT.DIV.002` | `crates/wow-chat/src/lib.rs` (`exists_empty`, 0 Rust lines) | 21 C++ files / 7293 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **`CHAT_MSG_ADDON = -1`** — signed int32, NOT a normal `uint8` like the others. Must remain signed in protocol struct or comparisons against `MAX_CHAT_MSG_TYPE` break.
- **`GM_SILENCE_AURA = 1852`** — special spell aura that mutes a player; chat handlers must check `Player::HasAura(1852)` before sending.
- **Addon language IDs** — `LANG_ADDON = 183`, `LANG_ADDON_LOGGED = 184`. The "logged" variant is intentionally captured for moderation; clients send it via `CMSG_CHAT_ADDON_MESSAGE` with `isLogged=true`.
- **Built-in channel zone-restriction** — `LocalDefense` is per-zone; joining requires `Player::GetZoneId()` matches `AreaTableEntry.AreaBit` of the channel's home area. Joining from wrong zone → `CHAT_NOT_IN_AREA_NOTICE`.
- **Channel team split** — Alliance and Horde share name strings but are separate `Channel` instances unless `WORLD_CONFIG_ALLOW_TWO_SIDE_INTERACTION_CHANNEL` is true. RustyCore must replicate or face faction chat-leak bugs.
- **Channel persistence interval** — Trinity persists dirty channels every `WORLD_CONFIG_PRESERVE_CUSTOM_CHANNEL_INTERVAL` minutes (default 5min); not on every change.
- **`ChannelAppenders.h` is template-heavy** — the C++ uses `SendToAll<JoinedAppender>` with each appender being a stateless functor that writes its specific `SMSG_CHANNEL_NOTIFY` payload. In Rust, prefer per-notify-type fn rather than trait-objects.
- **Hyperlink validation is a security boundary** — without it, clients can craft `\|cffffffff\|Hitem:0\|h[Free Legendary]\|h\|r` to spoof item-link tooltips. Trinity drops the entire message on any invalid link; do NOT silently strip.
- **Cross-realm whisper** — `VirtualRealmAddress` must round-trip; if absent, retail clients fall back to local-realm and may misroute.
- **EmotesText.db2 vs Emote.db2** — `/wave` is in `EmotesText.db2` (text-emote table) which maps to an `EmoteId` that is then looked up in `Emote.db2` to get the animation. Two-step lookup; don't conflate.
- **Profanity filter** — Trinity has `WORLD_CONFIG_PROFANITY_FILTER` (CharacterDatabase `wordfilter`). Optional but recommended.
- **`HandleMessagechatOpcode` typo** — kept verbatim in C++ logging since 2009; do not "fix" if grepping log archives.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Channel` | `struct Channel` (en `crates/wow-chat/src/channel.rs` — TBD) | members in `DashMap<ObjectGuid, PlayerInfo>` |
| `class ChannelMgr` (per-team) | `struct ChannelMgr { alliance: DashMap, horde: DashMap }` (en `crates/wow-chat/src/channel_mgr.rs` — TBD) | `Arc<ChannelMgr>` in `WorldContext` |
| `class LanguageMgr` (singleton) | `struct LanguageMgr` (en `crates/wow-chat/src/language.rs` — TBD) | Built once at startup; behind `Arc` |
| `enum ChatMsg : int32` | `#[repr(i32)] enum ChatMsg` (en `wow-constants`) | Currently in `wow-packet`, move up |
| `enum Language` | `#[repr(u32)] enum Language` (en `wow-constants`) | |
| `enum ChatNotify : uint8` | `#[repr(u8)] enum ChatNotify` (en `wow-chat`) | 38 variants |
| `enum ChannelFlags / MemberFlags` | `bitflags! { ... }` (en `wow-chat`) | bitflags crate |
| `Channel::PlayerInfo` | `struct ChannelMember { flags: ChannelMemberFlags, invisible: bool }` | |
| `BannedContainer = GuidUnorderedSet` | `HashSet<ObjectGuid>` or `DashSet` | |
| `void HandleChatMessageOpcode(...)` | `async fn handle_chat_message(&mut self, pkt, msg_type)` | Already exists; needs routing fix |
| `Channel::Say(guid, what, lang)` | `async fn channel_say(&self, sender: ObjectGuid, msg: &str, lang: Language)` | iterates members, sends `ChatPkt` |
| `Hyperlinks::CheckAllLinks` | `pub fn check_all_links(msg: &str, ctx: &WorldContext) -> bool` (en `wow-chat::hyperlinks`) | parser + lookup tables |
| `ChatCommand` tree | `wow-chat::commands::CommandRegistry` | inventory-based registration mirroring `PacketHandlerEntry` |
| `WorldPackets::Channel::JoinChannel` | `crates/wow-packet/src/packets/channel.rs::JoinChannel` (TBD — file not created yet) | new module needed |
| `BuildChatPacket(...)` C++ helper | `ChatPkt::new_for(...)` constructor | Already exists in `wow-packet/chat.rs` |

---

*Template version: 1.0 (2026-05-01).* Status: ⚠️ partial — ~32% of C++ behaviour. The former proximity confidentiality bug is fixed for Party/Raid/RaidWarning/InstanceChat; group addon routing is represented for Party/Raid/InstanceChat; Guild/Officer remain undelivered until the guild model is ported.

---

## 13. Audit (2026-05-01)

Side-by-side audit of `crates/wow-chat/src/lib.rs` (empty) + `crates/wow-world/src/handlers/chat.rs` vs `src/server/game/Handlers/ChatHandler.cpp` + `src/server/game/Chat/Channels/`.

### Flagged divergence — verdict

**Party/Guild/Raid/Officer/InstanceChat broadcast by proximity instead of group/guild membership — PARTIALLY FIXED.**
`crates/wow-world/src/handlers/chat.rs:38-101` registers seven `inventory::submit!` entries (`ChatMessageSay`, `Yell`, `Party`, `Guild`, `Raid`, `RaidWarning`, `InstanceChat`) but examination of `chat.rs:141-184` shows that `handle_chat_message` is the single shared body for all of them and ends with:

```
let range = if msg_type == ChatMsg::Yell { RANGE_YELL } else { RANGE_SAY };
self.broadcast_chat_packet(&chat, range);
```

— a plain `broadcast_raw_packet` (`chat.rs:376-412`) that walked `player_registry()`, filtered by `info.map_id == sender_map`, and applied a 25y (or 300y for yell) distance check. That was a real confidentiality bug: `CHAT_MSG_PARTY` was visible to nearby non-group players and invisible to remote group members, and `CHAT_MSG_GUILD` leaked the same way. The current Rust branch routes `Party`/`Raid`/`RaidWarning`/`InstanceChat` through `GroupRegistry` membership delivery; `Guild` is now dropped until `GuildRegistry` exists, so it no longer leaks but still does not work. C++ contrast: `Player::Say/Yell` is in-range, but `Group::BroadcastPacket` (in `Group.cpp`) iterates `m_memberSlots` and `Guild::BroadcastToGuild` iterates the guild member set; neither uses spatial range.

The seven distinct inventory entries each list their own `handler_name` (`handle_chat_say`, `handle_chat_party`, …) but those names are only labels for the dispatch table — the actual function bodies converge on `handle_chat_message`, which receives `msg_type` and ignores it for routing.

### Coverage matrix

| C++ opcode handler | Rust | Verdict |
|---|---|---|
| `HandleChatMessageSay/Yell/Emote` | ✅ proximity broadcast for Say/Yell/Emote |  partial |
| `HandleChatMessageParty/Raid/RaidWarning/InstanceChat` | ✅ registered, ✅ group-routed via `GroupRegistry`/`PlayerRegistry`; residual BG original-group/config/language gates pending | partial |
| `HandleChatMessageGuild/Officer` | Guild registered but drops pending `GuildRegistry`; Officer not represented | missing |
| `HandleChatMessageWhisper` | ✅ `chat.rs:187-257` via `player_registry` name lookup; offline → inform-only echo |  partial |
| `HandleChatMessageAFK/DND` | ✅ registered; toggles canonical AFK/DND flags + represented auto-reply |  partial |
| `HandleChatAddonMessage` (3 variants) | `CMSG_CHAT_ADDON_MESSAGE` registered; Party/Raid/InstanceChat group routing + prefix gate represented; Guild/Officer/Channel/targeted remain missing | partial |
| `HandleChatIgnoredOpcode` | ✅ `CMSG_CHAT_REPORT_IGNORED` sends `CHAT_MSG_IGNORED` to ignored player |  partial |
| `HandleEmoteOpcode` | ✅ `chat.rs:297-301` (logs only, no `Unit::SetEmoteState`) | stub |
| `HandleTextEmoteOpcode` | ✅ `chat.rs:313-358` but no `EmotesText.db2` lookup, no `Player::HandleEmoteCommand` chain |  partial |
| `HandleJoinChannel/LeaveChannel/Command/PlayerCommand/Password/DeclineInvite` | ❌ all unregistered (channel system absent) |  |
| `Channel`, `ChannelMgr`, `LanguageMgr`, `Hyperlinks::CheckAllLinks`, `ChatCommand` | ❌ none, `crates/wow-chat/src/lib.rs` is 0 bytes |  |

### Other observed bugs

- `chat.rs:280` — `Emote` chat sets `language: 0` unconditionally (`LANG_UNIVERSAL`), but Trinity uses `LANG_UNIVERSAL=0` only as a literal — fine here.
- `chat.rs:208-212` — whisper target lookup is O(N) iter over the entire `PlayerRegistry` per whisper (`reg.iter().find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name))`). At realm scale this is a per-message linear scan.
- Hyperlinks pass through unvalidated — `chat.rs` has no `Hyperlinks::check_all_links` analogue. Fake tooltips trivial to craft.
- No `GM_SILENCE_AURA = 1852` check; muted players can still chat.
- AFK/DND now toggles canonical flags and represented auto-reply text, but C++ guild/script/localized-default/battleground and actual auto-reply delivery paths are still absent.
- No ignore-list cross-check — whispers from blocked senders pass through (see also `social.md` §13).

### Channels system

Entirely absent. The Trinity `Channel.cpp` (1026 lines), `ChannelMgr.cpp` (287 lines), and `ChannelAppenders.h` (476 lines) have **zero** Rust equivalent. No `CHAT_MSG_CHANNEL` handling, no `SMSG_CHANNEL_NOTIFY` family, no `channels` table read/write, no built-in channel auto-join on zone change. Players cannot create or use Trade/General/LFG/custom channels at all.

**Verdict:** the flagged proximity-broadcast routing bug was real and dangerous. It is now fixed for Party/Raid/RaidWarning/InstanceChat; Guild no longer leaks by proximity but remains undelivered until the guild model exists. Approximately 32% of `ChatHandler.cpp` is ported; channels are 0%. Hyperlink validation, language scrambling, AFK/DND side effects beyond status flags, guild/channel/targeted addon routing, and the `.gm` command parser are still incomplete.
