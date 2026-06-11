# Migration: Chat

> **C++ canonical path:** `src/server/game/Chat/` + `src/server/game/Handlers/ChatHandler.cpp` + `src/server/game/Handlers/ChannelHandler.cpp`
> **Rust target crate(s):** `crates/wow-chat/`, `crates/wow-world/src/handlers/chat.rs`, `crates/wow-packet/src/packets/chat.rs`
> **Layer:** L6
> **Status:** ⚠️ partial (~36.5% — say/yell/whisper/emote work; Party/Raid/RaidWarning/InstanceChat now use group membership routing; group addon Party/Raid/InstanceChat routing represented with config-backed `AddonChannel` gate; addon prefix register/unregister filtering now mirrors C++ softcap/filter-flag behavior; addon whisper represented with target lookup, level/silence/team gates, and receiver-side prefix filtering; targeted addon payload parsing plus direct Whisper/Party/Raid/InstanceChat handler paths represented with logged-language handling, but live runtime dispatch is blocked until the real opcode mapping replaces the duplicated `0xBADD` placeholder; `ValidateMessage` newline/control gates and config-backed duplicate-space collapse, `LANG_UNIVERSAL` client-cheat rejection, unknown-language rejection, offline-whisper `SMSG_CHAT_PLAYER_NOTFOUND`, `SMSG_CHAT_PLAYER_AMBIGUOUS` wire layout, `SMSG_CHAT_RESTRICTED` wire layout, `SMSG_PRINT_NOTIFICATION` wire layout, `SMSG_CHAT_SERVER_MESSAGE` wire layout, `SMSG_DEFENSE_MESSAGE` wire layout, config-backed Say/Yell/Emote/Whisper level gates, config-backed party raid warnings, config-backed invalid-link kick for represented chat handlers, whisper AFK/DND auto-replies, AFK/DND default auto-reply fallbacks, `GM_SILENCE_AURA`, and hyperlink shape/control-sequence rejection represented; ignored-report, filtered-report C++ stub, and AFK/DND status toggles represented; Guild/Officer, channels, targeted channel addon routing, semantic hyperlink validation, full `LanguageMgr` still missing)
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
- **CharacterDatabase** — `channels` table (custom channel persistence) + `character_social` (social/ignore data; C++ uses ignore checks in channels/invites/LFG, but current `CHAT_MSG_WHISPER` contrast does not show a receiver-ignore drop in `ChatHandler`/`Player::Whisper`)
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
| Direct `SELECT` of `character_social` flags | Future social/channel ignore gates where C++ calls `PlayerSocial::HasIgnore`; do not invent a whisper drop without a C++ call-site | character |

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
| `crates/wow-chat` | `crate_dir` | 3 | 279 | `exists_active` | crate exists; first-stage hyperlink parser and validation helpers active |
| `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 1360 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/chat.rs` | `file` | 1 | 351 | `exists_active` | file exists |
| `crates/wow-chat/src/lib.rs` + `src/hyperlinks.rs` + `src/validation.rs` | `file` | 3 | 279 | `exists_active` | first-stage hyperlink parser and validation helpers active |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-chat/src/lib.rs` + `crates/wow-chat/src/{hyperlinks,validation}.rs` — 279 lines — structural `Hyperlinks::CheckAllLinks` parser/gate plus `ValidateMessage` newline/control-char logic; semantic per-tag validators still missing
- `crates/wow-world/src/handlers/chat.rs` — 1360 lines — covers ~34% of `Handlers/ChatHandler.cpp`
- `crates/wow-packet/src/packets/chat.rs` — 351 lines — `ChatMessage`, `ChatMessageWhisper`, `ChatMessageEmote`, `ChatPkt`, `EmoteMessage`, `STextEmote`, `CTextEmote`, `EmoteClient`, `ChatMsg` enum

**What's implemented:**
- `CMSG_CHAT_MESSAGE_SAY` / `_YELL` — proximity broadcast within range (25y say/emote, 300y yell).
- `CMSG_CHAT_MESSAGE_PARTY` / `_RAID` / `_RAID_WARNING` / `_INSTANCE_CHAT` — route through `GroupRegistry` + `PlayerRegistry`, with C++-style subgroup filtering for party, leader message variants, raid-only gates, config-backed `PartyRaidWarnings`, and raid-warning leader/assistant gates.
- `CMSG_CHAT_MESSAGE_GUILD` — intentionally no-op until `GuildRegistry`/`Guild::BroadcastToGuild` is ported; this avoids the previous proximity leak but is still missing guild delivery.
- `ValidateMessage` first-stage chat validation — normal chat, whisper, chat-emote, AFK, and DND now reject empty/oversized normal chat, reject leading newline/control chars, truncate at newline/carriage return, and honor `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` / `ChatFakeMessagePreventing` for duplicate-space collapse.
- Say/Yell/chat-emote now mirror C++ `Player::IsAlive()` cheating gates and config-backed `ChatLevelReq.Say/Yell/Emote`; whisper applies config-backed `ChatLevelReq.Whisper` with the C++ GM sender exemption. `ChatLevelReq.Channel` is loaded into `SessionResources` but remains dormant until `ChannelMgr` exists.
- `Hyperlinks::CheckAllLinks` first-stage shape/control validation — `wow_chat::hyperlinks::check_all_links_shape_like_cpp` rejects illegal `|` control sequences, malformed `|c...|H...|h[...]|h|r` links, and unknown link tags before normal chat delivery; represented chat handlers now honor `ChatStrictLinkChecking.Kick` / `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` by disconnecting on that invalid-link gate when enabled.
- `CMSG_CHAT_MESSAGE_WHISPER` — name lookup via `PlayerRegistry`; online target receives `Whisper` and sender receives `WhisperInform`; offline/missing target now receives C++ `SMSG_CHAT_PLAYER_NOTFOUND` instead of a false delivery inform; AFK/DND targets send the represented C++ `LANG_PLAYER_AFK`/`LANG_PLAYER_DND` system auto-reply to the sender.
- `CMSG_CHAT_MESSAGE_EMOTE` — broadcasts `CHAT_MSG_EMOTE` packet at 25y range.
- `CMSG_EMOTE` — parsed and logged; no emote-state-machine update on `Unit`.
- `CMSG_SEND_TEXT_EMOTE` — pass-through: emits `STextEmote` + `EmoteMessage` with raw client-supplied EmoteId (no EmotesText.db2 lookup, no anim-id resolution, no sound).
- `ChatMsg` Rust enum exists in `wow-packet::packets::chat` (subset — needs full audit count).

**What's missing vs C++:**
- **Channels system entirely absent** — no `Channel`, no `ChannelMgr`, no DB persistence, no `CMSG_CHAT_JOIN_CHANNEL`, no `LEAVE`, no `COMMAND`/`PLAYER_COMMAND`, no `PASSWORD`, no `DECLINE_INVITE`, no `SMSG_CHANNEL_NOTIFY*` family.
- **Built-in channels** (Trade, General, LookingForGroup, GuildRecruitment, LocalDefense) — none auto-joined on zone change.
- **Custom user channels** — no creation/destruction, no `channels` table read/write.
- **Moderator/Owner/Banlist** — no `MEMBER_FLAG_*` enforcement, no `KickOrBan`, no `SetMode`, no `SetOwner`, no `Announce` toggle.
- **Languages** — full `LanguageMgr` not ported; speech is never scrambled and addon lang (183/184) never validated beyond known-language existence. The first C++ gates rejecting client-sent `LANG_UNIVERSAL` for non-emote chat and rejecting unknown language ids are represented.
- **Hyperlinks** — shape/control-sequence validation and config-backed invalid-link kick are represented for normal chat, whisper/chat-emote, AFK, and DND, but full C++ semantic validation (`ValidateLinkInfo`: item/spell/quest/achievement/text/color/store lookups), packet-parser hyperlink exception kick, and mail/other hyperlink gates remain incomplete.
- **Addon messages** — `CMSG_CHAT_REGISTER_ADDON_PREFIXES` and `CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES` now mirror C++ receiver-side addon-prefix filtering, including the softcap flag behavior: unregister clears prefixes but does not disable filtering. `CMSG_CHAT_ADDON_MESSAGE` honors the config-backed `AddonChannel` gate and routes Party/Raid/InstanceChat addon payloads through group membership and receiver-side addon-prefix filtering. `CMSG_CHAT_ADDON_MESSAGE_WHISPER` now routes through named targets, applies represented silence/whisper-level/same-team gates where C++ does, and uses receiver-side addon-prefix filtering. The `CMSG_CHAT_ADDON_MESSAGE_TARGETED` C++ payload layout and reusable direct handler path are represented for Whisper/Party/Raid/InstanceChat, including `LANG_ADDON_LOGGED`, but live opcode dispatch is intentionally not registered yet because the inspected C++ 3.4.3 source marks it as the duplicated unresolved `0xBADD` placeholder. Guild/Officer/Channel addon routing and targeted channel/name lookup remain absent.
- **AFK/DND** — `CMSG_CHAT_MESSAGE_AFK/DND` now toggles canonical `PLAYER_FLAGS_AFK/DND`, stores represented auto-reply text, applies first-stage `ValidateMessage`/hyperlink shape gates with config-backed invalid-link kick, rejects represented `GM_SILENCE_AURA=1852`, uses represented English `LANG_PLAYER_AFK_DEFAULT`/`LANG_PLAYER_DND_DEFAULT` fallback text, and whisper delivery now sends represented AFK/DND auto-replies. Guild away event, script hook, DB-backed localized default strings, and battleground side-effect remain missing.
- **Whisper offline queue** — no fallback, no `BN_WHISPER_PLAYER_OFFLINE`.
- **Cross-realm whisper resolution (VirtualRealmAddress)** — partially wired (`virtual_realm_address` field passed) and `SMSG_CHAT_PLAYER_AMBIGUOUS` wire layout exists, but no name-disambiguation flow sends it yet.
- **Guild / Officer routing** — `GuildRegistry`/`Guild::BroadcastToGuild` and `BroadcastToOfficer` are still absent. Guild currently drops rather than proximity-leaking.
- **Group routing residuals** — Party/Raid/RaidWarning/InstanceChat now use group membership routing, but battleground original-group/BG-group selection, full `LanguageMgr` validation/scrambling, and remaining chat-validation gates remain missing.
- **Chat commands (`.gm`, `!command`)** — no `ChatCommand` registry, no command parser, no security-level enforcement.
- **Profanity / spam filter** — absent. `CMSG_CHAT_REPORT_FILTERED` is represented as the current C++ no-op/debug stub, but no actual spam-reporting backend exists. `GM_SILENCE_AURA=1852` is now represented for normal chat, chat-emote, AFK/DND, and whisper-to-non-GM gates, but the exact localized notification packet/string is still not ported.
- **`SMSG_CHAT_PLAYER_AMBIGUOUS`, `SMSG_PRINT_NOTIFICATION`, `SMSG_CHAT_SERVER_MESSAGE`, `SMSG_CHAT_RESTRICTED`, `SMSG_DEFENSE_MESSAGE`** — wire layouts exist where noted, but the higher-level call sites are still partial or absent; defense message send/localization path remains absent.

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
- [x] **#CHAT.3a** Fix Party/Raid/RaidWarning/InstanceChat routing — use `GroupRegistry`/`PlayerRegistry` membership delivery instead of proximity; preserve party subgroup filtering, raid-only gates, config-backed `PartyRaidWarnings`, leader variants, and raid-warning leader/assistant gate. Complejidad: **M**
- [ ] **#CHAT.3b** Port Guild/Officer routing — add `GuildRegistry`/`Guild::BroadcastToGuild`/`BroadcastToOfficer`; keep delivery out of proximity routing. Complejidad: **M**
- [ ] **#CHAT.3c** Finish group-chat residuals — battleground original-group/BG-group selection, full `LanguageMgr` validation/scrambling, and remaining C++ chat-validation gates. Complejidad: **M**
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
- [x] **#CHAT.14b1** Port first-stage chat `ValidateMessage` gates — length/empty checks for normal chat, newline truncation/rejection, ASCII-control rejection except tab, applied to normal chat, whisper, chat-emote, AFK, and DND. Complejidad: **M**
- [x] **#CHAT.14b2a** Port represented `GM_SILENCE_AURA=1852` chat gates — non-whisper chat/emote/AFK/DND are rejected, and whispers are rejected unless the receiver is a GM, matching `ChatHandler.cpp` first-stage behavior. Complejidad: **S**
- [x] **#CHAT.14b2c** Port C++ first-stage language cheat gate — client-sent `LANG_UNIVERSAL` is rejected for non-emote chat/whisper before delivery. Complejidad: **S**
- [x] **#CHAT.14b2d** Port C++ unknown-language existence gate — normal chat and whisper reject language ids absent from `LanguageMgr::LoadLanguages` / `SharedDefines.h` before delivery; full skill/aura/scrambling logic remains in `#CHAT.17`. Complejidad: **S**
- [x] **#CHAT.14b2e** Port C++ Say/Yell/Emote level gates — config-backed `ChatLevelReq.Say/Yell/Emote` rejects represented senders below the configured level before delivery. Complejidad: **S**
- [x] **#CHAT.14b2i** Port C++ Whisper level gate — config-backed `ChatLevelReq.Whisper` rejects non-GM represented senders below the configured level before delivery; GM sender exemption is represented. Complejidad: **S**
- [x] **#CHAT.14b2f** Port C++ whisper AFK/DND auto-replies — `Player::Whisper` sends represented `LANG_PLAYER_AFK`/`LANG_PLAYER_DND` system text to the sender after `WHISPER_INFORM` when the target is AFK/DND. Complejidad: **S**
- [x] **#CHAT.14b2g** Port represented AFK/DND default auto-reply fallbacks — empty AFK/DND text now stores C++ English `LANG_PLAYER_AFK_DEFAULT` / `LANG_PLAYER_DND_DEFAULT` fallback strings (`Away from Keyboard`, `Do not Disturb`); DB-backed localization remains future work. Complejidad: **S**
- [x] **#CHAT.14b2h** Port config-backed `ChatFakeMessagePreventing` duplicate-space collapse — `world-server` reads `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING`, sessions carry it, and `ValidateMessage` collapses subsequent spaces only when enabled, matching C++ `sWorld->getBoolConfig`. Complejidad: **S**
- [x] **#CHAT.14b2j** Port config-backed invalid-link kick for represented chat handlers — `world-server` reads `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK`, sessions carry it, and invalid first-stage hyperlink/control sequences call `WorldSession::kick` for normal chat, whisper, chat-emote, AFK, and DND when enabled. Complejidad: **S**
- [x] **#CHAT.14b2k** Represent C++ `SMSG_CHAT_RESTRICTED` wire packet and `ChatRestrictionType` reason enum. Behavior call sites remain pending with the full mute/throttle/profanity system. Complejidad: **S**
- [x] **#CHAT.14b2l** Represent C++ `SMSG_PRINT_NOTIFICATION` and `SMSG_CHAT_SERVER_MESSAGE` wire packets. Behavior call sites remain pending. Complejidad: **S**
- [ ] **#CHAT.14b2b** Complete remaining C++ AFK/DND/chat side effects: packet-parser hyperlink exception kick, localized `LANG_GM_SILENCE` notification, guild away event, script hook, DB-backed localized defaults, and battleground leave. Complejidad: **M**
- [x] **#CHAT.15** Implement `CMSG_CHAT_REPORT_IGNORED` — inform sender they were ignored. Complejidad: **L**
- [x] **#CHAT.15a** Implement `CMSG_CHAT_REPORT_FILTERED` as the current C++ stub — empty packet reader, logged no-op handler, no spam-reporting backend invented. Complejidad: **S**
- [x] **#CHAT.15b** Represent C++ `SMSG_CHAT_PLAYER_AMBIGUOUS` wire packet. Cross-realm/name-disambiguation behavior remains pending. Complejidad: **S**
- [x] **#CHAT.15c** Represent C++ `SMSG_DEFENSE_MESSAGE` wire packet. OutdoorPvP/Battlefield localized send path remains pending. Complejidad: **S**
- [x] **#CHAT.16a** Implement group addon routing for `CMSG_CHAT_ADDON_MESSAGE` Party/Raid/InstanceChat — build `SMSG_CHAT` with real addon prefix, `LANG_ADDON`/`LANG_ADDON_LOGGED`, config-backed `AddonChannel` gate, no sender echo, party subgroup filtering, and receiver-side `IsAddonRegistered(prefix)` gate. Complejidad: **M**
- [x] **#CHAT.16a1** Fix `CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES` filter semantics — clear registered prefixes without disabling the C++ `_filterAddonMessages` softcap/filter flag, so receiver-side addon filtering remains active until the next valid registration packet. Complejidad: **S**
- [ ] **#CHAT.16b** Implement Guild/Officer/Channel addon routing once `GuildRegistry`/`ChannelMgr` exist. Complejidad: **M**
- [x] **#CHAT.16c1** Implement `CMSG_CHAT_ADDON_MESSAGE_WHISPER` — parse the C++ packet layout, find the named online target, apply represented GM-silence, config-backed whisper-level, and same-team gates, then enqueue addon whisper delivery through receiver-side `IsAddonRegistered(prefix)` filtering. Complejidad: **M**
- [x] **#CHAT.16c2a** Represent `CMSG_CHAT_ADDON_MESSAGE_TARGETED` payload layout and reusable direct handler path for Whisper/Party/Raid/InstanceChat addon messages, including `LANG_ADDON_LOGGED` preservation. Live runtime dispatch remains pending because C++ currently exposes this packet as duplicated placeholder opcode `0xBADD`. Complejidad: **M**
- [ ] **#CHAT.16c2b** Resolve/register the real targeted addon opcode or introduce a safe unresolved-placeholder opcode model; then implement remaining targeted addon channel/name routing plus cross-realm/RBAC two-side resolution. Complejidad: **M**
- [ ] **#CHAT.17** Port `LanguageMgr` — load `Languages.db2`+`LanguageWords.db2`, scramble text for unknown-language listeners. Complejidad: **H**
- [x] **#CHAT.18a** Port first-stage `Hyperlinks::CheckAllLinks` shape/control validation — reject illegal `|` controls, malformed link envelopes, uppercase/invalid color hex, and unknown link tags before normal chat delivery. Complejidad: **M**
- [ ] **#CHAT.18b** Port semantic hyperlink validators (`ValidateLinkInfo`) for core WotLK tags: item, quest, spell, achievement, enchant, trade, talent, glyph/journal-compatible tags where present. Complejidad: **XL** (split per-tag)
- [ ] **#CHAT.18c** Apply hyperlink validation to AFK/DND, mail subject/body, and other C++ `ValidateHyperlinksAndMaybeKick` call sites. Complejidad: **M**
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
- [x] Test: Whisper to offline/missing name produces `SMSG_CHAT_PLAYER_NOTFOUND` with the target name like C++ `SendChatPlayerNotfoundNotice`.
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
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 21 files / 7293 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `crates/wow-chat/`, `crates/wow-world/src/handlers/chat.rs`, `crates/wow-packet/src/packets/chat.rs` \| ⚠️ partial (~36.5% — say/yell/whisper/emote work; Party/Raid/RaidWarning/InstanceChat and group addon routing are represented with config-backed `AddonChannel`; addon whisper represented; targeted addon payload parsing and direct Whisper/Party/Raid/InstanceChat handler paths represented but not runtime-registered until real opcode resolution; `ValidateMessage` including config-backed duplicate-space collapse, config-backed Say/Yell/Emote/Whisper level gates, config-backed party raid warnings, config-backed invalid-link kick, `LANG_UNIVERSAL` rejection, unknown-language rejection, offline-whisper `SMSG_CHAT_PLAYER_NOTFOUND`, whisper AFK/DND auto-replies, AFK/DND default auto-reply fallbacks, `GM_SILENCE_AURA`, and hyperlink shape/control validation represented; ignored-report and AFK/DND status toggles represented; Guild/Officer, channels, targeted channel addon routing, semantic hyperlink validation, full `LanguageMgr` still missing) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#CHAT.DIV.001` | `crates/wow-chat` (`partial`, 279 Rust lines) | 21 C++ files / 7293 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `partial` | Rust target now contains first-stage hyperlink parsing and `ValidateMessage`; Channel/ChannelMgr/LanguageMgr and semantic hyperlink validators remain absent. |
| `#CHAT.DIV.002` | `crates/wow-chat/src/lib.rs` + `src/hyperlinks.rs` + `src/validation.rs` (`partial`, 279 Rust lines) | 21 C++ files / 7293 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Chat/Channels/Channel.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Chat.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Chat/Hyperlinks.cpp` | `partial` | First-stage `CheckAllLinks` structural parser and `ValidateMessage` represented; per-tag DB/DB2 semantic validation remains open. |

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

*Template version: 1.0 (2026-05-01).* Status: ⚠️ partial — ~36.5% of C++ behaviour. The former proximity confidentiality bug is fixed for Party/Raid/RaidWarning/InstanceChat; group addon routing is represented for Party/Raid/InstanceChat with the config-backed `AddonChannel` gate; addon whisper is runtime-registered through receiver-side prefix filtering; targeted addon parsing and direct Whisper/Party/Raid/InstanceChat handler paths are represented but not runtime-registered until real opcode resolution; first-stage `ValidateMessage` and hyperlink shape/control validation are represented; Guild/Officer remain undelivered until the guild model is ported.

---

## 13. Audit (2026-05-01)

Side-by-side audit of `crates/wow-chat` + `crates/wow-world/src/handlers/chat.rs` vs `src/server/game/Handlers/ChatHandler.cpp` + `src/server/game/Chat/Channels/`.

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
| `HandleChatMessageSay/Yell/Emote` | ✅ proximity broadcast for Say/Yell/Emote; ✅ dead/level-0 players rejected for Say/Yell/chat-emote like C++ default config |  partial |
| `HandleChatMessageParty/Raid/RaidWarning/InstanceChat` | ✅ registered, ✅ group-routed via `GroupRegistry`/`PlayerRegistry`; ✅ client-sent `LANG_UNIVERSAL` and unknown languages rejected; ✅ `PartyRaidWarnings` represented; residual BG original-group/BG-group and full `LanguageMgr` gates pending | partial |
| `HandleChatMessageGuild/Officer` | Guild registered but drops pending `GuildRegistry`; Officer not represented | missing |
| `HandleChatMessageWhisper` | ✅ `chat.rs` via `player_registry` name lookup; online delivery + sender inform; offline/missing target sends `SMSG_CHAT_PLAYER_NOTFOUND`; AFK/DND target sends represented system auto-reply |  partial |
| `HandleChatMessageAFK/DND` | ✅ registered; toggles canonical AFK/DND flags + represented auto-reply |  partial |
| `HandleChatAddonMessage` (3 variants) | `CMSG_CHAT_ADDON_MESSAGE` registered; Party/Raid/InstanceChat group routing + prefix gate represented; `CMSG_CHAT_ADDON_MESSAGE_WHISPER` represented; `CMSG_CHAT_ADDON_MESSAGE_TARGETED` layout/direct handler path represented for Whisper/Party/Raid/InstanceChat but not runtime-registered until the real opcode is resolved; Guild/Officer/Channel/targeted-channel remain missing | partial |
| `HandleChatIgnoredOpcode` / `HandleChatReportFiltered` | ✅ `CMSG_CHAT_REPORT_IGNORED` sends `CHAT_MSG_IGNORED` to ignored player; ✅ `CMSG_CHAT_REPORT_FILTERED` represented as C++ empty-packet logged stub |  partial |
| `HandleEmoteOpcode` | ✅ `chat.rs:297-301` (logs only, no `Unit::SetEmoteState`) | stub |
| `HandleTextEmoteOpcode` | ✅ `chat.rs:313-358` but no `EmotesText.db2` lookup, no `Player::HandleEmoteCommand` chain |  partial |
| `HandleJoinChannel/LeaveChannel/Command/PlayerCommand/Password/DeclineInvite` | ❌ all unregistered (channel system absent) |  |
| `Channel`, `ChannelMgr`, `LanguageMgr`, `Hyperlinks::CheckAllLinks`, `ChatCommand` | `Hyperlinks::CheckAllLinks` shape/control gate represented; Channel/ChannelMgr/LanguageMgr/ChatCommand absent; semantic hyperlink validators absent | partial |

### Other observed bugs

- `chat.rs:280` — `Emote` chat sets `language: 0` unconditionally (`LANG_UNIVERSAL`), but Trinity uses `LANG_UNIVERSAL=0` only as a literal — fine here.
- `chat.rs:208-212` — whisper target lookup is O(N) iter over the entire `PlayerRegistry` per whisper (`reg.iter().find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name))`). At realm scale this is a per-message linear scan.
- Hyperlinks no longer pass completely unchecked for normal chat: illegal control sequences, malformed envelopes, invalid color hex, and unknown tags are rejected, and the represented chat handlers honor config-backed kick on that invalid-link gate. Fake-but-structurally-valid item/spell/etc. payloads still require C++ `ValidateLinkInfo` semantic validators.
- `ValidateMessage` newline/control-char gates and config-backed duplicate-space collapse are represented for normal chat, whisper, chat-emote, AFK and DND.
- `ChatLevelReq.Say/Yell/Emote/Whisper` are config-backed; `ChatLevelReq.Channel` is parsed into the session resource snapshot but not behaviorally consumed until the channel system is ported.
- `GM_SILENCE_AURA = 1852` is now checked via represented visible auras for normal chat, chat-emote, AFK/DND, and whisper-to-non-GM. The remaining gap is notification fidelity (`LANG_GM_SILENCE`) and any path outside represented chat handlers.
- AFK/DND now toggles canonical flags, applies first-stage validation gates, stores represented auto-reply text including English C++ default fallback strings, and whisper delivery sends represented AFK/DND auto-replies, but C++ guild/script/DB-localized-default/battleground paths are still absent.
- Earlier notes claimed a missing whisper ignore-list gate, but current C++ contrast found no `PlayerSocial::HasIgnore` call in `WorldSession::HandleChatMessage(... CHAT_MSG_WHISPER ...)` or `Player::Whisper`; do not add a Rust-only whisper drop unless a real C++ call-site is identified.

### Channels system

Entirely absent. The Trinity `Channel.cpp` (1026 lines), `ChannelMgr.cpp` (287 lines), and `ChannelAppenders.h` (476 lines) have **zero** Rust equivalent. No `CHAT_MSG_CHANNEL` handling, no `SMSG_CHANNEL_NOTIFY` family, no `channels` table read/write, no built-in channel auto-join on zone change. Players cannot create or use Trade/General/LFG/custom channels at all.

**Verdict:** the flagged proximity-broadcast routing bug was real and dangerous. It is now fixed for Party/Raid/RaidWarning/InstanceChat; Guild no longer leaks by proximity but remains undelivered until the guild model exists. Approximately 34% of `ChatHandler.cpp` is ported; channels are 0%. Semantic hyperlink validation, language scrambling, AFK/DND side effects beyond status flags/first-stage validation, guild/channel/targeted addon routing, and the `.gm` command parser are still incomplete.
