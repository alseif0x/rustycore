# Migration: Server/Packets (per-domain message structs)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Server/Packets/`
> **Rust target crate(s):** `crates/wow-packet/src/packets/`
> **Layer:** L1 (cross-cutting reference; sits on top of `shared-packets.md` L0 ByteBuffer/WorldPacket)
> **Status:** ⚠️ partial — Rust covers ~20 domains vs ~70 TC `*Packets.cpp` files (mainline post-WotLK includes Legion+/BfA/Shadowlands domains absent in 3.4.3)
> **Audited vs C++:** ⚠️ partial — file/line-count comparison done, struct-by-struct opcode parity not yet exhaustive
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-domain packet message structs. While `shared-packets.md` (L0) covers the wire-format primitives (`ByteBuffer`, `WorldPacket`, bit-packing), this catalogue covers the **typed packet bodies**: structs that wrap a single opcode plus its payload and serialize via `ServerPacket::write` / deserialize via `ClientPacket::read`.

In TrinityCore mainline (the legacy clone here is post-WotLK and includes expansion-only domains) this is split into ~70 `*Packets.cpp/.h` pairs under `src/server/game/Server/Packets/`, one per gameplay domain. RustyCore mirrors the same idea under `crates/wow-packet/src/packets/` but with **only ~20 domain modules**, and several of them collapse multiple TC domains into one (notably `misc.rs` at 2613 LOC, 145 structs — it absorbs the contents of TC `MiscPackets.cpp`, `ClientConfigPackets.cpp`, `SystemPackets.cpp`, parts of `WorldStatePackets.cpp`, `TimeSync`, etc.).

The mismatch is partially an artifact of WotLK 3.4.3 not needing many of the post-WotLK domains (Garrison, BattlePet, BlackMarket, Artifact, Azerite, Scenario, Trait, Transmog, VoidStorage, Toy, Token, Collection, Adventure*, Mythic+, Perks, Refer-A-Friend, Warden) — but it also reflects genuine gaps for domains that **do** exist in 3.4.3 (Calendar, AuctionHouse, Mail, Guild, BG, Channel, Petitions, Pet, Tax, Trade, Vehicle, CombatLog, Inspect-as-separate-domain, GameObject, Talent, Totem, Duel, EquipmentSet, Instance, Skill, Ticket).

---

## 2. C++ canonical files

Inventory under `/home/server/woltk-trinity-legacy/src/server/game/Server/Packets/`. 75 `*.cpp` files including the `*Common.cpp` shared-types modules and `PacketUtilities.cpp`. **Total ≈ 18 177 LOC** of `.cpp` (headers add roughly 2× more in struct definitions).

| C++ file | LOC | Purpose / scope |
|---|---|---|
| `AccountPackets.cpp` | 29 | Account data times, server time |
| `AchievementPackets.cpp` | 251 | Criteria update, achievement earned, guild achievements |
| `AddonPackets.cpp` | 41 | Addon info / prefix registration |
| `AdventureJournalPackets.cpp` | — | (Legion+) — n/a 3.4.3 |
| `AdventureMapPackets.cpp` | 29 | (Legion+) — n/a 3.4.3 |
| `AreaTriggerPackets.cpp` | 101 | Area trigger entered/left |
| `ArenaTeamPackets.cpp` | 82 | Arena team query / roster / stats |
| `ArtifactPackets.cpp` | 71 | (Legion) — n/a 3.4.3 |
| `AuctionHousePackets.cpp` | 772 | List/place/bid/cancel; ~14 opcodes |
| `AuthenticationPackets.cpp` | 366 | AuthChallenge, AuthSession, AuthResponse, ConnectTo |
| `AzeritePackets.cpp` | 71 | (BfA) — n/a 3.4.3 |
| `BankPackets.cpp` | 63 | BuyBankSlot, BankerActivate |
| `BattlegroundPackets.cpp` | 523 | BG join/leave/score, queue status |
| `BattlenetPackets.cpp` | 90 | BNet request/response wrapping |
| `BattlePayPackets.cpp` | 25 | (cash shop) — n/a 3.4.3 |
| `BattlePetPackets.cpp` | 210 | (MoP+) — n/a 3.4.3 |
| `BlackMarketPackets.cpp` | 108 | (MoP+) — n/a 3.4.3 |
| `CalendarPackets.cpp` | 512 | Events, RSVPs, invites |
| `ChannelPackets.cpp` | 196 | Join/leave/list/mute/silence |
| `CharacterPackets.cpp` | 727 | Enum/create/delete/login/customize/rename |
| `ChatPackets.cpp` | 358 | Say/yell/whisper/emote/addon |
| `ClientConfigPackets.cpp` | 76 | LFG list, account data ack |
| `CollectionPackets.cpp` | 25 | (WoD+) — n/a 3.4.3 |
| `CombatLogPacketsCommon.cpp` | 195 | Shared combat-log structs (SpellLogEffect, etc.) |
| `CombatLogPackets.cpp` | 488 | SpellNonMeleeDamageLog, EnvironmentalDamage, PeriodicAuraLog |
| `CombatPackets.cpp` | 166 | AttackStart/Stop, AttackerStateUpdate (legacy) |
| `CraftingPacketsCommon.cpp` | 62 | (DF+) — n/a 3.4.3 |
| `DuelPackets.cpp` | 79 | DuelRequested, DuelComplete, DuelInBounds |
| `EquipmentSetPackets.cpp` | 132 | Save/use/delete equipment set |
| `EventPackets.cpp` | 28 | UpdateGameEvent (calendar holidays) |
| `GameObjectPackets.cpp` | 103 | GameObjectActivateAnimKit, GameObjectCustomAnim |
| `GarrisonPackets.cpp` | 484 | (WoD+) — n/a 3.4.3 |
| `GuildPackets.cpp` | 1047 | Roster/bank/rank/MOTD/petition |
| `HotfixPackets.cpp` | 127 | (MoP+ DB2 hotfix) — partial-applicable to 3.4.3 hotfix push |
| `InspectPackets.cpp` | 247 | InspectResult, InspectPVP, InspectTalent |
| `InstancePackets.cpp` | 168 | InstanceInfo, InstanceLockResponse, RaidGroupOnly |
| `ItemPacketsCommon.cpp` | 267 | Shared item bonuses, modifiers, sockets |
| `ItemPackets.cpp` | 363 | Inv-change-failure, item-push, item-cooldown |
| `LFGPacketsCommon.cpp` | 42 | Shared LFG structs |
| `LFGPackets.cpp` | 497 | LFG queue/teleport/join/leave |
| `LootPackets.cpp` | 248 | LootResponse, LootRelease, MasterLoot |
| `MailPackets.cpp` | 305 | List, body, send, take, delete |
| `MiscPackets.cpp` | 832 | Tutorial/repop/binder/weather/zone/gossip-misc |
| `MovementPackets.cpp` | 1097 | Move, transfer, teleport-ack, fall, swim, transport |
| `MythicPlusPacketsCommon.cpp` | 130 | (Legion+) — n/a 3.4.3 |
| `NPCPackets.cpp` | 291 | NPCText, ListInventory, TrainerList, GossipPOI |
| `PacketUtilities.cpp` | 62 | Helpers (Hyperlinks, regex validators) |
| `PartyPackets.cpp` | 790 | Group invite/leave/role/loot/leader |
| `PerksProgramPacketsCommon.cpp` | 38 | (DF+) — n/a 3.4.3 |
| `PetitionPackets.cpp` | 192 | Guild/arena charter sign |
| `PetPackets.cpp` | 206 | PetActionFeedback, PetSpells, PetMode |
| `QueryPackets.cpp` | 534 | Creature/GO/quest/NPCText/realm-name query |
| `QuestPackets.cpp` | 823 | Accept/complete/abandon/log-update |
| `ReferAFriendPackets.cpp` | 30 | (RAF) — n/a 3.4.3 |
| `ReputationPackets.cpp` | 70 | InitialFactions, SetFactionVisible/AtWar |
| `ScenarioPackets.cpp` | 136 | (MoP+) — n/a 3.4.3 |
| `ScenePackets.cpp` | 56 | (WoD+) — n/a 3.4.3 |
| `SocialPackets.cpp` | 148 | FriendList/IgnoreList/who/whois |
| `SpellPackets.cpp` | 1042 | SpellGo, AuraUpdate (alt path), CastFailed, SendKnownSpells |
| `SystemPackets.cpp` | 278 | FeatureSystemStatus, GameTime, UrlInfo |
| `TalentPackets.cpp` | 179 | Talent learn, glyph, spec switch |
| `TaxiPackets.cpp` | 78 | TaxiNodeStatusQuery, ShowTaxiNodes, ActivateTaxi |
| `TicketPackets.cpp` | 392 | GMTicket / bug-report / suggest |
| `TokenPackets.cpp` | 55 | (WoD+) — n/a 3.4.3 |
| `TotemPackets.cpp` | 46 | TotemCreated, TotemDestroyed |
| `ToyPackets.cpp` | 57 | (WoD+) — n/a 3.4.3 |
| `TradePackets.cpp` | 138 | TradeStatus, TradeUpdated, TradeAccepted |
| `TraitPacketsCommon.cpp` | 137 | (DF+) — n/a 3.4.3 |
| `TraitPackets.cpp` | 68 | (DF+) — n/a 3.4.3 |
| `TransmogrificationPackets.cpp` | 53 | (Cata+) — n/a 3.4.3 |
| `VehiclePackets.cpp` | 69 | Move-vehicle, vehicle-seat-switch |
| `VoidStoragePackets.cpp` | 106 | (Cata+) — n/a 3.4.3 |
| `WardenPackets.cpp` | 29 | Warden challenge/response |
| `WhoPackets.cpp` | 135 | /who request + result |
| `WorldStatePackets.cpp` | 52 | UpdateWorldState, InitWorldStates |
| **Total cpp** | **18 177** | **75 files (incl. 6 `*Common` + `PacketUtilities`)** |

WotLK 3.4.3 applicability: **~50 of 75** TC domains apply. The 25 marked `n/a 3.4.3` are post-WotLK content (Legion artifact / BfA azerite / SL covenant / DF traits / WoD garrison-toy-token-collection / MoP scenario-blackmarket-battlepet / Cata transmog-voidstorage / DF crafting-perks).

---

## 3. Rust packet domain inventory (`crates/wow-packet/src/packets/`)

20 modules listed in `mod.rs`. Total ≈ 11 794 LOC; 392 `pub struct` declarations.

| Rust file | LOC | `pub struct` count | Notes |
|---|---|---|---|
| `auth.rs` | 804 | 31 | AuthChallenge, AuthSession, AuthResponse, EnableEncryption, ConnectToFailed |
| `aura.rs` | 123 | 3 | `AuraData`, `AuraUpdate` (likely thin) |
| `battlenet.rs` | 223 | 4 | `BattlenetRequest`, BNet response wrapping |
| `character.rs` | 550 | 20 | Enum / create / delete / customize / rename / login |
| `character_packets.rs` | 10 | 1 | **Stub file** — almost empty, likely accidental dup |
| `chat.rs` | 351 | 16 | Say / yell / whisper / channel / addon |
| `combat.rs` | 192 | 14 | AttackStart, AttackStop; **also** holds combat-log (SpellNonMeleeDamageLog) — fuses TC `CombatPackets` + `CombatLogPackets` |
| `gossip.rs` | 326 | 14 | Gossip Hello / select / POI / NPCText (fuses parts of TC `NPCPackets` + `MiscPackets`) |
| `inspect.rs` | 120 | 3 | Inspect basics; PVP / talent variants likely missing |
| `item.rs` | 395 | 13 | Item-push, inv-change-failure |
| `loot.rs` | 210 | 16 | Loot response / release / item / money |
| `misc.rs` | **2613** | **145** | **Catch-all**: tutorial, weather, zone, hotfix, time-sync, account-data, system-feature, world-state, gametime, area-trigger, durability — fuses TC `MiscPackets` + `ClientConfigPackets` + `SystemPackets` + `WorldStatePackets` + `EventPackets` + `AccountPackets` + `AreaTriggerPackets` + `HotfixPackets` |
| `movement.rs` | 461 | 15 | MOVE_HEARTBEAT family, transfer-pending, teleport-ack — much smaller than TC's 1097 LOC; advanced transport / falling likely stubbed |
| `party.rs` | 302 | 17 | Group invite / leave / role / loot rules |
| `query.rs` | 597 | 22 | Creature / GO / quest / NPCText / player-name |
| `quest.rs` | 603 | 22 | Accept / complete / abandon / log-update |
| `social.rs` | 117 | 5 | Friend / ignore — `WhoPackets` content not present here |
| `spell.rs` | 466 | 15 | Cast / fail / aura / cooldown |
| `trainer.rs` | 233 | 12 | Trainer list / buy-spell (TC routes this through `NPCPackets`) |
| `update.rs` | 3072 | 9 | UpdateObject super-packet (`CreatureCreateData`, `PlayerCreateData`); the 3K LOC is encoder logic, not new packet types |
| **Total** | **11 794** | **392 structs** | 20 files (one almost-empty: `character_packets.rs`) |

---

## 4. Domain-by-domain mapping (TC → Rust → coverage)

Coverage rubric: ✅ ≥80% of TC domain structs implemented • ⚠️ 30–80% • 🚧 1–30% (handful of structs only) • ❌ 0% (no Rust file or content) • 🔵 n/a (post-WotLK; intentionally skipped for 3.4.3).

### 4.1 Domains present in both (paired)

| Domain | TC file (LOC) | Rust file (LOC, structs) | Coverage |
|---|---|---|---|
| Authentication | `AuthenticationPackets.cpp` (366) | `auth.rs` (804, 31) | ✅ Rust line-count exceeds TC because it includes BNet handshake glue + EnableEncryption + ResumeComms |
| Battle.net | `BattlenetPackets.cpp` (90) | `battlenet.rs` (223, 4) | ⚠️ Rust thicker on framing, but only 4 structs vs ~5–6 TC opcodes |
| Character | `CharacterPackets.cpp` (727) | `character.rs` (550) + `character_packets.rs` (10) | ⚠️ ~75% — 20 structs vs estimated 25–30; rename/customize/factionchange may be partial. `character_packets.rs` is a 10-line orphan |
| Chat | `ChatPackets.cpp` (358) | `chat.rs` (351, 16) | ✅ Sizes match closely; addon-message + 3.4.3 channels covered |
| Channel (chat-channels) | `ChannelPackets.cpp` (196) | folded into `chat.rs` | ⚠️ likely partial — TC splits join/leave/list/mute/silence into a separate file; no dedicated Rust module |
| Combat | `CombatPackets.cpp` (166) + `CombatLogPackets.cpp` (488) + `CombatLogPacketsCommon.cpp` (195) | `combat.rs` (192, 14) | ⚠️ 192 vs 849 combined — combat-log is largely missing; only attack-start/stop and basic damage events present |
| Gossip / NPC | `NPCPackets.cpp` (291) | `gossip.rs` (326, 14) | ⚠️ Gossip core OK; vendor list / trainer split into `trainer.rs`; stable / banker still likely missing |
| Inspect | `InspectPackets.cpp` (247) | `inspect.rs` (120, 3) | 🚧 ~30% — only base inspect, no PvP/talent variants |
| Item | `ItemPackets.cpp` (363) + `ItemPacketsCommon.cpp` (267) | `item.rs` (395, 13) | ⚠️ ~50% — push/use/cooldown OK; bonuses/sockets/modifiers thin |
| Loot | `LootPackets.cpp` (248) | `loot.rs` (210, 16) | ✅ Close parity |
| Misc bag (multi-domain) | `MiscPackets.cpp` (832) + `ClientConfigPackets.cpp` (76) + `SystemPackets.cpp` (278) + `EventPackets.cpp` (28) + `AccountPackets.cpp` (29) + `WorldStatePackets.cpp` (52) + `HotfixPackets.cpp` (127) + `AreaTriggerPackets.cpp` (101) | `misc.rs` (2613, 145) | ✅ TC totals ~1523 LOC, Rust 2613 — includes time-sync, auth-data ack, weather, durability, world-state, tutorial. Volume suggests broad coverage; quality not yet audited |
| Movement | `MovementPackets.cpp` (1097) | `movement.rs` (461, 15) | ⚠️ ~40% — basic MOVE_* heartbeat covered; transport / vehicle move / fall-land detailed responses likely partial |
| Group / Party | `PartyPackets.cpp` (790) | `party.rs` (302, 17) | ⚠️ ~40% — invite/leave/loot/role-check OK; raid-target-update, ready-check details may be thin |
| Quest | `QuestPackets.cpp` (823) | `quest.rs` (603, 22) | ⚠️ ~70% — accept/complete/log-update OK; quest-giver-status array, POI, push-to-party may be partial |
| Query | `QueryPackets.cpp` (534) | `query.rs` (597, 22) | ✅ Rust slightly larger; creature/GO/quest/NPCText/realm-name all listed |
| Social (friends/ignore) | `SocialPackets.cpp` (148) | `social.rs` (117, 5) | ⚠️ ~75% friends/ignore; **`WhoPackets.cpp` (135) has no Rust counterpart** |
| Spell | `SpellPackets.cpp` (1042) | `spell.rs` (466, 15) + `aura.rs` (123, 3) | ⚠️ ~55% — cast / fail / cooldown / aura-update yes; spell-go bit-packed body, channel-update, missile-cancel, 3.4.3 SMSG_SPELL_DELAYED likely partial |
| Trainer | (folded in `NPCPackets.cpp`) | `trainer.rs` (233, 12) | ✅ Rust extracted as own module |
| Update (object update) | (cross-cuts; structs in `UpdateData.h` not under Server/Packets/) | `update.rs` (3072, 9) | ✅ Bit-mask field encoding for `UpdateObject`; 9 wrapper structs over a giant encoder |

### 4.2 Domains in TC but **NOT in Rust** (3.4.3-applicable)

These are gaps that DO need addressing for 3.4.3:

| TC domain | LOC | Why it matters in 3.4.3 |
|---|---|---|
| `AchievementPackets.cpp` | 251 | Criteria-update, achievement-earned, guild-achievement-update — **needed** for the existing `wow-achievement` crate |
| `AddonPackets.cpp` | 41 | Addon prefix register / addon-message routing |
| `ArenaTeamPackets.cpp` | 82 | Arena rated PvP — needed for `wow-pvp` |
| `AuctionHousePackets.cpp` | 772 | AH browse/bid/sell — major missing domain |
| `BankPackets.cpp` | 63 | BuyBankSlot, banker activate |
| `BattlegroundPackets.cpp` | 523 | BG queue/score — needed for `wow-pvp` BG path |
| `CalendarPackets.cpp` | 512 | Calendar events — major missing domain (3.4.3 has it) |
| `ChannelPackets.cpp` | 196 | Standalone chat-channel ops (currently fused into `chat.rs`?) |
| `DuelPackets.cpp` | 79 | DuelRequested / DuelComplete — duel handler exists, packet structs probably ad-hoc |
| `EquipmentSetPackets.cpp` | 132 | EquipmentSet save/use — 3.4.3 supports |
| `GameObjectPackets.cpp` | 103 | GO custom-anim / activate-anim-kit |
| `GuildPackets.cpp` | 1047 | **Largest gap** — guild roster/bank/rank/MOTD; `wow-world` has guild handler but no packet module |
| `InstancePackets.cpp` | 168 | InstanceInfo, RaidGroupOnly, lock-response |
| `LFGPackets.cpp` (+ Common 42) | 539 | LFG/dungeonfinder — needed |
| `MailPackets.cpp` | 305 | Mail list/body/send/take — **major missing** |
| `PetPackets.cpp` | 206 | PetSpells, PetMode, PetActionFeedback |
| `PetitionPackets.cpp` | 192 | Charter sign for guild/arena |
| `ReputationPackets.cpp` | 70 | InitialFactions, SetFactionVisible |
| `TalentPackets.cpp` | 179 | Talent-learn, glyph-add, spec-switch |
| `TaxiPackets.cpp` | 78 | ShowTaxiNodes, ActivateTaxi |
| `TicketPackets.cpp` | 392 | GMTicket / bug-report |
| `TotemPackets.cpp` | 46 | TotemCreated / TotemDestroyed |
| `TradePackets.cpp` | 138 | Trade begin / accept / item / money |
| `VehiclePackets.cpp` | 69 | Vehicle seat / move |
| `WardenPackets.cpp` | 29 | Warden challenge — anti-cheat |
| `WhoPackets.cpp` | 135 | /who request and result rows |

**Total missing-but-needed: 26 domains, ≈ 6 376 C++ LOC.**

### 4.3 Domains intentionally skipped (post-WotLK)

🔵 25 TC files for content that didn't exist in 3.4.3.54261:

`AdventureJournalPackets.cpp`, `AdventureMapPackets.cpp`, `ArtifactPackets.cpp`, `AzeritePackets.cpp`, `BattlePayPackets.cpp`, `BattlePetPackets.cpp`, `BlackMarketPackets.cpp`, `CollectionPackets.cpp`, `CraftingPacketsCommon.cpp`, `GarrisonPackets.cpp`, `MythicPlusPacketsCommon.cpp`, `PerksProgramPacketsCommon.cpp`, `ReferAFriendPackets.cpp`, `ScenarioPackets.cpp`, `ScenePackets.cpp`, `TokenPackets.cpp`, `ToyPackets.cpp`, `TraitPacketsCommon.cpp`, `TraitPackets.cpp`, `TransmogrificationPackets.cpp` (Cata transmog feature, debatable for 3.4.3), `VoidStoragePackets.cpp` (Cata feature). Plus `AccountPackets`-style time-sync if absorbed elsewhere. **Verdict: do NOT migrate** — explicitly mark as out-of-scope in roadmap.

(Some are debatable: HotfixPackets is technically MoP+ but RustyCore already has a hotfix path via `HotfixBlobCache` so partial structs exist in `misc.rs`.)

---

## 5. Module dependencies

**Depends on:**
- `crates/wow-packet/src/world_packet.rs` — `WorldPacket`, bit-packing primitives (read/write_bit, flush_bits, packed GUID).
- `crates/wow-constants` — `ClientOpcodes`, `ServerOpcodes` enums (every packet struct sets `const OPCODE: ServerOpcodes = ServerOpcodes::X`).
- `crates/wow-core` — `ObjectGuid`, `Position`.

**Depended on by:**
- `crates/wow-handler` — dispatch table, calls `ClientPacket::read` per opcode.
- `crates/wow-world/src/handlers/*.rs` — every handler module imports specific packet types from `wow_packet::packets::*`.
- `crates/wow-world/src/session.rs` — sends server packets via `send_packet` / `send_tx`.

---

## 6. SQL / DB queries

None. This is pure wire-format. Some packet bodies *carry* DB IDs (creature entry, item entry, quest ID) but the packet module itself does not query.

---

## 7. Wire-protocol packets

This *is* the wire-protocol-packets module. See §4 for full mapping.

Convention in Rust:
- Server-to-client struct implements `ServerPacket` trait with `const OPCODE: ServerOpcodes = ...; fn write(&self, pkt: &mut WorldPacket);`.
- Client-to-server struct implements `ClientPacket` trait with `fn read(pkt: &mut WorldPacket) -> Result<Self, _>;`.
- Both use bit-flags via `WorldPacket::write_bit / read_bit / flush_bits` for 4.0+-style wire bodies (3.4.3.54261 client backports many of those).

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore/crates/wow-packet/src/packets/`:** 20 files, 11 794 LOC, **392 `pub struct` declarations** total.

**What's implemented (covers ~50% of 3.4.3-applicable opcodes):**
- Auth handshake fully (`auth.rs`, 31 structs)
- Misc/system/world-state/tutorial/area-trigger/account-data via the catch-all `misc.rs` (145 structs)
- Character enum/create/login basics (`character.rs`)
- Chat/say/yell/whisper/addon (`chat.rs`)
- Loot (`loot.rs`), Trainer (`trainer.rs`)
- Quest log + accept/complete (`quest.rs`)
- Query family (`query.rs`)
- Object update encoder (`update.rs`, 3072 LOC — the heavy hitter)
- Spell cast/fail/aura skeletons (`spell.rs` + `aura.rs`)
- Movement heartbeat / teleport-ack basics (`movement.rs`)
- Party invite/leave/role (`party.rs`)
- Friends/ignore (`social.rs`)
- Item push/use/cooldown (`item.rs`)
- Combat attack-start/stop (`combat.rs`)
- BNet framing (`battlenet.rs`)

**What's missing vs C++ (3.4.3-relevant, prioritized by handler dependency):**
1. Guild — 1047 LOC, no Rust file
2. AuctionHouse — 772 LOC, no Rust file
3. Battleground — 523 LOC, no Rust file
4. Calendar — 512 LOC, no Rust file
5. LFG — 539 LOC combined, no Rust file
6. Mail — 305 LOC, no Rust file
7. Achievement — 251 LOC, no Rust file (despite a `wow-achievement` crate existing)
8. Inspect details (PvP/talent variants) — only 30% of the TC file
9. Combat-log (SpellNonMeleeDamageLog, PeriodicAuraLog) — only ~40% of TC combined
10. Movement transport / vehicle / detailed fall — ~40% of TC `MovementPackets.cpp`
11. Spell-go bit-packed body, channel-update, talent learn — ~55% of `SpellPackets.cpp` + 0% of `TalentPackets.cpp`
12. Pet (PetSpells, PetMode), Petition, Trade, Vehicle, Bank, Reputation, Taxi, Totem, Duel, EquipmentSet, GameObject, Instance, Channel-standalone, Who, Ticket, ArenaTeam, Warden, Addon — none have a dedicated Rust module.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `character_packets.rs` (10 LOC, 1 struct) — looks like an accidental dup or scaffolding remnant of `character.rs`. Either delete or merge.
- `misc.rs` at 2613 LOC / 145 structs is a maintainability concern — multiple TC domains fused. Plan: split into `system.rs`, `world_state.rs`, `area_trigger.rs`, `time_sync.rs` once handler migration stabilizes.
- `combat.rs` at 192 LOC is suspiciously small given TC has 849 LOC across `CombatPackets` + `CombatLogPackets*` — confirms combat-log is largely absent (matches `wow-combat` crate's known partial state).
- `inspect.rs` 3 structs vs TC ~10–15 — confirms `InspectHandler.cpp` (152 LOC) in C++ writes packet variants that have no Rust struct.
- `update.rs` at 3072 LOC for only 9 structs is correct: most volume is bit-mask field encoders, not packet count.

**Tests existing:** Each Rust packet module has unit tests for the structs it owns (sample: `aura.rs` has `test_aura_update_write`). **0** tests exist for the missing-domain files (because the files don't exist).

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** <1h, **M** 1–4h, **H** 4–12h, **XL** >12h split.

### Tier 1 — high-impact missing domains (block live-server features)

- [ ] **#PKD.1** Create `crates/wow-packet/src/packets/guild.rs` with structs for guild roster / invite / promote / demote / motd / rank / bank-deposit / bank-withdraw (target: ≈25 structs, ~700 LOC). (XL — split into guild-core, guild-bank, guild-rank if needed)
- [ ] **#PKD.2** Create `auction_house.rs` — list-items, place-bid, sell-item, cancel-auction, owner-list, replicate-items (~20 structs). (H)
- [ ] **#PKD.3** Create `battleground.rs` — BG join/queue, status, leave-bg, score-update, init-world-states-arena. (H)
- [ ] **#PKD.4** Create `calendar.rs` — calendar-get-events, add-event, RSVP, invite, remove. (H)
- [ ] **#PKD.5** Create `lfg.rs` (incl. LFGPacketsCommon) — join-LFG, role-check-update, dungeon-finder-pop, teleport. (H)
- [ ] **#PKD.6** Create `mail.rs` — list, body, send, take-money, take-item, mark-read, return, delete. (M-H)
- [ ] **#PKD.7** Create `achievement.rs` — criteria-update, achievement-earned, all-achievement-data, guild-criteria-update. (M)

### Tier 2 — feature-completeness (often handler exists, packet inline)

- [ ] **#PKD.8** Create `talent.rs` — learn-talent, glyph-add, spec-switch, learn-talents-in-primary-tree. (M)
- [ ] **#PKD.9** Create `pet.rs` — pet-spells, pet-mode, pet-action-feedback, pet-rename. (M)
- [ ] **#PKD.10** Create `trade.rs` — trade-status, trade-updated, trade-accepted, trade-money. (M)
- [ ] **#PKD.11** Create `bank.rs` — buy-bank-slot, banker-activate. (L)
- [ ] **#PKD.12** Create `taxi.rs` — show-taxi-nodes, activate-taxi, taxi-node-status. (L)
- [ ] **#PKD.13** Create `vehicle.rs` — move-set-vehicle-rec-id, vehicle-seat-switch. (L)
- [ ] **#PKD.14** Create `petition.rs` — charter sign / show / decline / turn-in. (M)
- [ ] **#PKD.15** Create `equipment_set.rs` — save/use/delete. (L)
- [ ] **#PKD.16** Create `who.rs` — who-request, who-result. (L)
- [ ] **#PKD.17** Create `channel.rs` — extract from `chat.rs` if currently fused; join/leave/list/mute. (M)
- [ ] **#PKD.18** Create `ticket.rs` — GMTicket / bug-report. (M)
- [ ] **#PKD.19** Create `totem.rs`, `duel.rs`, `reputation.rs`, `instance.rs`, `arena_team.rs`, `addon.rs`, `game_object.rs`, `warden.rs`. (each L)

### Tier 3 — fill out partial domains

- [ ] **#PKD.20** Extend `inspect.rs` with PVP-inspect, talent-inspect (target +5 structs). (M)
- [ ] **#PKD.21** Extend `combat.rs` to absorb `CombatLogPackets` content: SpellNonMeleeDamageLog, EnvironmentalDamage, PeriodicAuraLog, SpellMissLog. **Or** split into `combat_log.rs`. (H)
- [ ] **#PKD.22** Extend `movement.rs` with transport-related packets, vehicle move-set, swim-start/stop, fall-land (target +10 structs, ≈ +600 LOC). (H)
- [ ] **#PKD.23** Extend `spell.rs` with full spell-go bit-packed body, channel-start/update/cancel, set-flat-spell-modifier, send-known-spells, learn-spell. (H)

### Tier 4 — hygiene / refactor

- [ ] **#PKD.24** Decide fate of `character_packets.rs` (10 LOC, 1 struct): merge into `character.rs` or delete. (L)
- [ ] **#PKD.25** Split `misc.rs` (2613 LOC, 145 structs) into `system.rs`, `world_state.rs`, `area_trigger.rs`, `time_sync.rs`, `account_data.rs`, `event.rs`. Pure refactor — gated behind handler-migration completion to avoid churn. (XL)
- [ ] **#PKD.26** Audit struct-by-struct parity per domain (compare each `pub struct` in a Rust file against the TC `class XYZ : public ServerPacket` list in the corresponding header). Fill the per-domain coverage % with hard numbers. (XL — one M task per domain)

---

## 10. Regression tests to write

For every domain pair where a Rust struct exists, validate byte-for-byte parity with a captured TC packet sample (when possible) **or** with a hand-computed expected buffer:

- [ ] Test: `auth.rs` SMSG_AUTH_RESPONSE byte equality vs known-good captured buffer.
- [ ] Test: `update.rs` `UpdateObject` for a creature-create-data — exact byte-for-byte against TC reference.
- [ ] Test: `quest.rs` SMSG_QUEST_GIVER_QUEST_DETAILS reward arrays.
- [ ] Test: `query.rs` SMSG_QUERY_CREATURE_RESPONSE non-localized strings.
- [ ] Test: `chat.rs` SMSG_MESSAGECHAT for SAY / WHISPER / CHANNEL.
- [ ] Test: bit-packing roundtrip — every struct that uses `write_bit/flush_bits` must read back identically (property test via `proptest` or QuickCheck).
- [ ] Test (per Tier 1 new module): when added, include at least one round-trip + one captured-buffer test.

---

## 11. Notes / gotchas

- **Two-step dispatch (AGENTS.md):** adding a packet struct alone is not enough — the opcode must also have a `match` arm AND `inventory::submit!` registration in the dispatcher. Forgetting the `submit!` silently drops the opcode. Document this in each new packet module's doc-comment.
- **`use wow_packet::ClientPacket;`** must be imported explicitly in each handler module decoding a packet. The trait does not auto-import. (Handlers, not packet modules, but the pattern arises whenever a new `ClientPacket` impl is added here.)
- **`Position` fields are `.x .y .z .orientation`** — never `.o`. Recurring mistake when writing position into a packet.
- **3.4.3.54261 is a backport client.** Many opcodes look like Cata/MoP IDs but with WotLK semantics. Resolve uncertainty against the exact C++ packet class/registry or an identified client capture; no other implementation is a layout authority.
- **`*_stubs.rs` from AGENTS.md** does NOT live under `crates/wow-packet/src/packets/`. It lives in `crates/wow-world/_attic/stubs.rs.txt` (a `.txt` rename so cargo skips it) and only stubs handlers — NOT packet structs. The packet crate has zero stub files of its own; partial coverage shows up as missing-file rather than as `*_stubs.rs`.
- **`character_packets.rs` (10 LOC) is the only smell** of stub-ness inside `wow-packet`. Treat it as #PKD.24.
- **Bit-packing convention:** TC writes bit fields LSB-first within a byte then flushes; the Rust impl in `world_packet.rs` mirrors that. Any new packet body using bits must call `flush_bits()` before any byte-aligned write that follows, or the buffer will desync.
- **Update.rs is special.** Don't be fooled by the 3072 LOC — it is one packet (`SMSG_UPDATE_OBJECT`) with a giant field-mask encoder. New domains should NOT live inside it.

---

## 12. C++ → Rust mapping (high-level)

| C++ pattern | Rust equivalent | Notes |
|---|---|---|
| `class Foo final : public ServerPacket { ByteBuffer Write() override; }` | `pub struct Foo { ... } impl ServerPacket for Foo { const OPCODE: ServerOpcodes = ...; fn write(&self, pkt: &mut WorldPacket); }` | Rust uses trait + assoc-const instead of vtable |
| `class Bar final : public ClientPacket { void Read() override; }` | `pub struct Bar { ... } impl ClientPacket for Bar { fn read(pkt: &mut WorldPacket) -> Result<Self, _>; }` | Result-based error propagation |
| TC `_worldPacket << X << Y;` (operator<< chain) | Rust `pkt.write_u32(self.x); pkt.write_u32(self.y);` (or `WriteInto`) | Explicit calls; no operator overloading |
| TC `_worldPacket.WriteBits(value, N)` | Rust `pkt.write_bits(value, N)` from `world_packet.rs` | Same LSB-first semantics |
| TC `_worldPacket.FlushBits()` | Rust `pkt.flush_bits()` | Required before next byte-aligned write |
| TC inline struct in `.h` then encode in `.cpp` | Rust struct + `impl` in same `.rs` | Tighter; tests inline `#[cfg(test)]` |
| TC `Optional<T>` in packet | Rust `Option<T>` with explicit bit-flag for presence | Match TC encoding exactly |
| TC `std::vector<T>` in packet | Rust `Vec<T>` preceded by length-prefix u32/u16 (per TC's WriteBits) | Match TC's prefix size, often `WriteBits(n, K)` for some K |
| TC ObjectGuid packed via `<<` | Rust `pkt.write_packed_guid(self.guid)` | 8-byte mask + variable bytes; same as TC `::WriteAsPacked` |
| TC `class FooPackets.cpp` (one domain) | Rust `crates/wow-packet/src/packets/foo.rs` (one domain) | Mostly 1:1; RustyCore fuses 8 TC files into `misc.rs` |

---

## 13. Audit (vs C++) — performed 2026-05-01

**Method:**
1. `ls /home/server/woltk-trinity-legacy/src/server/game/Server/Packets/` → 75 `*.cpp` files (with paired `*.h`).
2. `wc -l` on all `*.cpp` → total **18 177 LOC** of C++ packet implementation.
3. `ls /home/server/rustycore/crates/wow-packet/src/packets/` → 20 `*.rs` files + `mod.rs`.
4. `wc -l` on all Rust `*.rs` → total **11 794 LOC**, of which `update.rs` alone is 3 072 LOC and `misc.rs` is 2 613 LOC.
5. `grep -c "^pub struct\|^impl ServerPacket\|^impl ClientPacket"` per Rust file → **392 struct declarations** total across the 20 modules.
6. `find crates -name "*stubs*"` → only `crates/wow-world/_attic/stubs.rs.txt`. **No stub files in `wow-packet`.** The AGENTS.md mention of `*_stubs.rs` refers to the world crate, not the packet crate.
7. Mapped each TC file to (a) its 3.4.3 applicability, (b) the Rust file containing equivalent content (or `none`), (c) coverage tier.

**Top-line findings:**

| Metric | TC | RustyCore | Δ |
|---|---|---|---|
| Domain `.cpp/.rs` files | 75 (incl. 6 `*Common` + utilities) | 20 (incl. mod.rs orphan `character_packets.rs`) | **−55 files** |
| 3.4.3-applicable domains | ~50 | 20 (with 8 fused into `misc.rs`) | **~30 domains effectively missing or partial** |
| Total LOC | 18 177 | 11 794 | −6 383 (but Rust includes 3072 LOC of `update.rs` encoder absent in TC's per-domain split) |
| Avg LOC per Rust domain (excl. `update.rs`+`misc.rs`) | n/a | ≈ 340 | reasonable size |

**Per-domain delta highlights (LOC C++ → LOC Rust):**

| Domain | C++ | Rust | Delta indicates |
|---|---|---|---|
| Guild | 1047 | **0** | Hard miss — major feature absent |
| Movement | 1097 | 461 | ~58% missing |
| Spell + CombatLog (related) | 1042+488 | 466+192 | ~57% missing on combat-log side |
| AuctionHouse | 772 | **0** | Hard miss |
| Calendar | 512 | **0** | Hard miss |
| Battleground | 523 | **0** | Hard miss |
| LFG | 539 | **0** | Hard miss |
| Misc family (8 TC files) | 1523 | 2613 | Inverted — Rust includes more, but it's fused (refactor needed) |
| Update | n/a as separate file in TC | 3072 | Rust extracted UpdateData as packet domain |
| Inspect | 247 | 120 | ~50% missing |
| Item | 363+267 | 395 | ~37% missing (Common content thin) |
| Character | 727 | 550+10 | ~25% missing |

**Verdict:** Status badge is correct as ⚠️ partial. The 26 missing 3.4.3-applicable TC domains (≈ 6 376 LOC) are all listed in §4.2 with assigned migration sub-tasks (#PKD.1–#PKD.19, plus extensions #PKD.20–#PKD.23 for partial domains). Hygiene tasks #PKD.24–#PKD.26 cover the orphan `character_packets.rs`, the oversized `misc.rs`, and the per-struct parity audit.

**Open questions for next pass:**
- Confirm whether `ChannelPackets` content lives inside `chat.rs` (fused) or is genuinely missing (current evidence: 16 structs in `chat.rs` is plausibly inclusive of channel ops — needs grep audit on `Channel*` symbol names).
- Confirm coverage of `AddonPackets.cpp` (41 LOC, mostly addon-prefix registration) — likely fused into `chat.rs` or `auth.rs`.
- Confirm `TransmogrificationPackets.cpp` and `VoidStoragePackets.cpp` 3.4.3 applicability — both are Cata features; some 3.4.3.54261 backport clients have them, some don't.

---

*Template version: 1.0 (2026-05-01).* Coverage will move to ✅ once Tier 1 (#PKD.1–#PKD.7) is complete and §13 audit is re-run with hard struct-by-struct numbers per #PKD.26.
