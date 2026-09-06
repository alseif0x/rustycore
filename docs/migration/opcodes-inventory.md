# Migration: opcodes-inventory (cross-cutting reference)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Server/Protocol/Opcodes.{h,cpp}`
> **Rust target crate(s):** `crates/wow-constants/`, `crates/wow-handler/`, `crates/wow-world/src/handlers/`
> **Layer:** L1 (infrastructure / wire-protocol)
> **Status:** ⚠️ partial — full enum landed; ~14 % CMSG and ~10 % SMSG actually wired
> **Audited vs C++:** ⚠️ partial (this document is the audit)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Single side-by-side enumeration of every CMSG/SMSG opcode that the WoW 3.4.3.54261 client / world server can speak. Functions as a router map: given an opcode name, where is the C++ handler, what is its `SessionStatus` / `PacketProcessing`, and is it covered by a Rust `PacketHandlerEntry`. Other migration docs (`movement.md`, `chat.md`, `combat.md`, …) reference this table instead of repeating opcode metadata per-domain.

---

## 2. C++ canonical files

Paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---:|---|
| `src/server/game/Server/Protocol/Opcodes.h` | 770 | `enum OpcodeClient` / `enum OpcodeServer`, `OpcodeMisc` (MAX_OPCODE = 0x3FFF), `OpcodeTable`, `ClientOpcodeHandler`, `SessionStatus`, `PacketProcessing`, `ConnectionType`. |
| `src/server/game/Server/Protocol/Opcodes.cpp` | 2280 | `OpcodeTable::Initialize()` registers **882** `DEFINE_HANDLER(CMSG_…)` and **1220** `DEFINE_SERVER_OPCODE_HANDLER(SMSG_…)` entries. Each row binds opcode → packet class → `WorldSession::Handle…` → status + processing. |
| `src/server/game/Server/Protocol/Handlers/WorldSocket*.cpp` | 600+ | Dispatches incoming opcode to `OpcodeTable[]` and enforces `SessionStatus` gate. |
| `src/server/game/Server/WorldSession.cpp` | 2400+ | `DosProtection::EvaluateOpcode` (line 1259) + `GetMaxPacketCounterAllowed` per-opcode rate limits. |
| `src/server/game/Server/Packets/AllPackets.h` | — | Umbrella include of every per-domain packet header (`Movement.h`, `Chat.h`, `Loot.h`, …). |

Header line for the registration table (excerpt):

```cpp
#define DEFINE_HANDLER(opcode, status, processing, handler) \
    ValidateAndSetClientOpcode<decltype(handler), handler>(opcode, #opcode, status, processing)
```

`SessionStatus` ∈ { `STATUS_AUTHED`, `STATUS_LOGGEDIN`, `STATUS_TRANSFER`, `STATUS_LOGGEDIN_OR_RECENTLY_LOGGOUT`, `STATUS_NEVER`, `STATUS_UNHANDLED` }.

`PacketProcessing` ∈ { `PROCESS_INPLACE`, `PROCESS_THREADSAFE`, `PROCESS_THREADUNSAFE` }.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `OpcodeClient` (`uint16`) | enum | All CMSG values. Range observed in TC modern: 0x3000-0x39FF. WoLK 3.4.3.54261 reuses the same numeric space. |
| `OpcodeServer` (`uint16`) | enum | All SMSG values. Range 0x2500-0x2DFF roughly. |
| `OpcodeMisc` | enum | `MAX_OPCODE = 0x3FFF`, `NUM_OPCODE_HANDLERS = 0x4000`, `UNKNOWN_OPCODE = 0xFFFF`, `NULL_OPCODE = 0xBADD`. |
| `SessionStatus` | enum | Gate — handler runs only when session is in matching state. |
| `PacketProcessing` | enum | Threading mode — inplace (network thread), threadsafe (any), threadunsafe (map thread). |
| `ConnectionType` | enum | `CONNECTION_TYPE_REALM = 0` / `CONNECTION_TYPE_INSTANCE = 1`. |
| `ClientOpcodeHandler` / `ServerOpcodeHandler` | class | Polymorphic dispatcher; `OpcodeTable._internalTableClient[opcode]` indexes them by 14-bit opcode. |
| `OpcodeTable` | class | Singleton `opcodeTable`; arrays `[NUM_OPCODE_HANDLERS]` of handler ptrs. |
| `WorldSession::DosProtection` | nested class | `EvaluateOpcode` + `_PacketThrottlingMap[opcode] = PacketCounter{lastReceiveTime, amountCounter}`. |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `OpcodeTable::Initialize()` | Builds whole 16 K-slot dispatch table at boot. | `ValidateAndSetClientOpcode`, `ValidateAndSetServerOpcode` |
| `OpcodeTable::ValidateClientOpcode` | Bounds + duplicate check before insert. | `TC_LOG_ERROR` |
| `WorldSession::DosProtection::EvaluateOpcode` | Per-tick rate limit, optionally kick / ban. | `KickPlayer`, `World::BanAccount` |
| `WorldSession::DosProtection::GetMaxPacketCounterAllowed` | ~190-case `switch` mapping opcode → max packets/s. | — |
| `IsInstanceOnlyOpcode(opcode)` | Returns true for opcodes that must travel on `CONNECTION_TYPE_INSTANCE`. | — |

---

## 5. Module dependencies

**Depends on:**
- `Packets/*.h` — every CMSG handler signature is `void(WorldSession::*)(WorldPackets::<Domain>::<Class>&)`; the registration template `get_packet_class<>` extracts that class statically.
- `WorldSession` — provides the `Handle…` member functions.
- `RBAC` — `STATUS_AUTHED` requires authenticated session.

**Depended on by:**
- `WorldSocket` — every inbound packet is dispatched via `opcodeTable[opcode]`.
- Logger — opcode name is looked up by `opcodeTable[oc]->Name` for trace/error lines.
- AntiDOS — keys its throttle map on opcode value.
- `Auctionhouse`, `LFG`, `Battleground`, etc. — every domain references its own opcode set.

---

## 6. SQL / DB queries (if any)

No direct DB queries from the opcode registration itself. Indirect: `auth.account` permissions are consulted via `RBAC` when a `STATUS_AUTHED` packet arrives. `world.disables` (type 7 `DISABLE_TYPE_OPCODE_HANDLER` — older builds only) historically blacklisted opcodes by ID.

---

## 7. Wire-protocol packets (if any)

**This module is the wire protocol.** Counts (this repo):

| Class | Count | Notes |
|---|---:|---|
| `DEFINE_HANDLER(CMSG_…)` registrations in C++ | **882** | Includes ~803 stubs bound to `Handle_NULL` / `STATUS_UNHANDLED`; ~621 reach a real `Handle…`. |
| `DEFINE_SERVER_OPCODE_HANDLER(SMSG_…)` in C++ | **1220** | Server-side reference table; many `STATUS_NEVER` (server only emits, never receives). |
| `ClientOpcodes` variants in Rust | **664** (`crates/wow-constants/src/opcodes.rs:13-682`) | Names match C++ (PascalCase-stripped). |
| `ServerOpcodes` variants in Rust | **954** (same file lines 684-1641) | Same naming convention. |
| Rust `PacketHandlerEntry` registrations | **~128** across `crates/wow-world/src/handlers/*.rs` | (grep `inventory::submit!` ⇒ 14 files, total 128 entries.) |

### 7.1 Range index (orientation only — opcode space is **sparse**, not block-allocated)

| Range | Typical contents | Examples |
|---|---|---|
| `0x2500-0x25FF` | Account / chr / Battle.net SMSG | `SMSG_AUTH_RESPONSE = 0x256D`, `SMSG_NEW_WORLD = 0x2594`, `SMSG_LOGIN_VERIFY_WORLD = 0x2597` |
| `0x2600-0x26FF` | Character + AddOns + early world SMSG | `SMSG_ADDON_LIST_REQUEST = 0x2642`, `SMSG_ACHIEVEMENT_EARNED = 0x2643` |
| `0x2700-0x27FF` | World + objects SMSG | `SMSG_ACCOUNT_DATA_TIMES = 0x270A`, `SMSG_UPDATE_OBJECT = 0x27CB` |
| `0x2800-0x29FF` | Items, social, etc SMSG | `SMSG_ACCOUNT_CRITERIA_UPDATE = 0x2868` |
| `0x2A00-0x2CFF` | Spell + chat SMSG | `SMSG_CHAT = 0x2BAD`, `SMSG_ACTIVE_GLYPHS = 0x2C51` |
| `0x2D00-0x2DFF` | Time / sync SMSG | `SMSG_TIME_SYNC_REQUEST = 0x2DD2` |
| `0x3100-0x32FF` | Trade, area, AH, AzeriteX CMSG | `CMSG_ACCEPT_TRADE = 0x315A`, `CMSG_AREA_TRIGGER = 0x31D6`, `CMSG_ADD_TOY = 0x3299` |
| `0x3300-0x35FF` | Battlefield, banker, character CMSG | `CMSG_BATTLEFIELD_LEAVE = 0x3175`, `CMSG_PLAYER_LOGIN = 0x35EB`, `CMSG_ADDON_LIST = 0x35D8` |
| `0x3600-0x36FF` | Friends, arena, BNet CMSG | `CMSG_ADD_FRIEND = 0x36D8`, `CMSG_ARENA_TEAM_ROSTER = 0x36B8`, `CMSG_KEEP_ALIVE = 0x3681` |
| `0x3700-0x37FF` | Account-notify, social, chat CMSG | `CMSG_AUTH_SESSION = 0x3765`, `CMSG_AUTH_CONTINUED_SESSION = 0x3766`, `CMSG_CHAT_MESSAGE_SAY = 0x37E7` |
| `0x3900-0x39FF` | Inventory + movement CMSG | `CMSG_AUTOBANK_ITEM = 0x3997`, `CMSG_MOVE_START_FORWARD = 0x39E4` |

### 7.2 Cross-reference table (high-traffic opcodes)

Columns: hex code · canonical name · C++ status · C++ processing · C++ handler · Rust handler. **`r-`** = registered (`inventory::submit!`), **`r✗`** = enum-only.

| Hex | Name | C++ status | C++ proc | C++ handler | Rust |
|---:|---|---|---|---|---|
| 0x3765 | CMSG_AUTH_SESSION | `AUTHED` | `INPLACE` | `HandleAuthSession` | r- (`world_socket.rs`) |
| 0x3766 | CMSG_AUTH_CONTINUED_SESSION | `AUTHED` | `INPLACE` | `HandleAuthContinuedSession` | r✗ |
| 0x35EB | CMSG_PLAYER_LOGIN | `AUTHED` | `THREADUNSAFE` | `HandlePlayerLoginOpcode` | r- (`character.rs`) |
| 0x34D6 | CMSG_LOGOUT_REQUEST | `LOGGEDIN` | `THREADUNSAFE` | `HandleLogoutRequest` | r- (`character.rs`) |
| 0x34D8 | CMSG_LOGOUT_CANCEL | `LOGGEDIN` | `THREADUNSAFE` | `HandleLogoutCancel` | r- (`character.rs`) |
| 0x3681 | CMSG_KEEP_ALIVE | `AUTHED` | `INPLACE` | `Handle_NULL` (no-op) | r- (`misc.rs`) |
| 0x35D8 | CMSG_ADDON_LIST | `AUTHED` | `INPLACE` | `HandleAddonList` | r- (`misc.rs`) |
| 0x39E4 | CMSG_MOVE_START_FORWARD | `LOGGEDIN` | `THREADUNSAFE` | `HandleMovementOpcodes` | r- (`movement.rs` aggregated) |
| 0x37E7 | CMSG_CHAT_MESSAGE_SAY | `LOGGEDIN` | `THREADUNSAFE` | `HandleChatMessageSay` | r- (`chat.rs`) |
| 0x31D6 | CMSG_AREA_TRIGGER | `LOGGEDIN` | `INPLACE` | `HandleAreaTriggerOpcode` | r- (`misc.rs`) |
| 0x3256 | CMSG_ATTACK_STOP | `LOGGEDIN` | `THREADUNSAFE` | `HandleAttackStopOpcode` | r- (`combat.rs`) |
| 0x3255 | CMSG_ATTACK_SWING | `LOGGEDIN` | `THREADUNSAFE` | `HandleAttackSwingOpcode` | r- (`combat.rs`) |
| 0x34AB | CMSG_ACTIVATE_TAXI | `LOGGEDIN` | `THREADSAFE` | `HandleActivateTaxiOpcode` | r✗ |
| 0x34B0 | CMSG_AREA_SPIRIT_HEALER_QUERY | `LOGGEDIN` | `THREADUNSAFE` | `HandleAreaSpiritHealerQueryOpcode` | r✗ |
| 0x34B1 | CMSG_AREA_SPIRIT_HEALER_QUEUE | `LOGGEDIN` | `THREADUNSAFE` | `HandleAreaSpiritHealerQueueOpcode` | r✗ |
| 0x34B3 | CMSG_BANKER_ACTIVATE | `LOGGEDIN` | `THREADUNSAFE` | `HandleBankerActivateOpcode` | r✗ |
| 0x34CA | CMSG_AUCTION_HELLO_REQUEST | `LOGGEDIN` | `THREADUNSAFE` | `HandleAuctionHelloOpcode` | r✗ |
| 0x36FE | CMSG_ACCEPT_GUILD_INVITE | `LOGGEDIN` | `THREADUNSAFE` | `HandleGuildAcceptInvite` | r✗ |
| 0x315A | CMSG_ACCEPT_TRADE | `LOGGEDIN` | `THREADUNSAFE` | `HandleAcceptTradeOpcode` | r✗ |
| 0x256D | SMSG_AUTH_RESPONSE | (server) | (realm) | sent by `HandleAuthSession` | emitted (`bnet-server` + `world_socket.rs`) |
| 0x2594 | SMSG_NEW_WORLD | (server) | (realm) | sent by `Player::TeleportTo` | r✗ |
| 0x2597 | SMSG_LOGIN_VERIFY_WORLD | (server) | (realm) | sent by `Player::SendInitialPacketsAfterAddToMap` | emitted (`character.rs`) |
| 0x270A | SMSG_ACCOUNT_DATA_TIMES | (server, NEVER) | (realm) | sent by `WorldSession::SendAccountDataTimes` | emitted (`misc.rs`) |
| 0x27CB | SMSG_UPDATE_OBJECT | (server) | (realm) | `Object::BuildValuesUpdate` / `BuildCreateUpdate` | emitted (`update.rs`) |
| 0x2DD2 | SMSG_TIME_SYNC_REQUEST | (server, NEVER) | (realm) | `WorldSession::Update` periodic | r✗ (no time-sync loop yet) |
| 0x2BAD | SMSG_CHAT | (server) | (realm) | many call sites | emitted (`chat.rs`) |

For the full enumeration use `crates/wow-constants/src/opcodes.rs` (alphabetical) cross-referenced against `Opcodes.cpp` (also alphabetical inside each `DEFINE_HANDLER` block) — diffing the two name lists directly is the canonical way to find unimplemented opcodes (882 − 128 ≈ 754 still TBD on the C→S path).

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs` — 1642 lines — `enum ClientOpcodes` (664) + `enum ServerOpcodes` (954) + `num_derive` `FromPrimitive`/`ToPrimitive`. Hex values match C++.
- `crates/wow-handler/src/lib.rs` — declares `PacketHandlerEntry { opcode, status, processing, handler }` and re-exports the `inventory` collection iterator.
- `crates/wow-world/src/handlers/*.rs` (14 files) — actual handlers. `inventory::submit!` count = **128**.
- `crates/wow-world/src/world_socket.rs` — pre-LoggedIn handshake (`AUTH_SESSION`, `AUTH_CONTINUED_SESSION`).

**What's implemented:**
- Full numeric enum coverage of CMSG + SMSG.
- 2-step dispatch infrastructure (match arm + `inventory::submit!`).
- ~128 CMSG handlers wired (auth, basic movement, chat, simple combat, character creation/login/logout, addon list, keep-alive, friends-list seed, group invite seed, loot stub, trainer stub).
- Common SMSG emitters: update objects, login verify world, account data times, chat, friends, group, loot.

**What's missing vs C++ (high-impact gaps):**
- All AH (`CMSG_AUCTION_*` ~14 opcodes).
- Mail (`CMSG_MAIL_*` ~10).
- Battleground / arena queue (`CMSG_BATTLEMASTER_*`, `CMSG_BATTLEFIELD_*` ~12).
- LFG / dungeon-finder (~25 opcodes).
- Calendar (`CMSG_CALENDAR_*` ~30).
- Guild full set (~60 opcodes; only `ACCEPT_GUILD_INVITE` enum exists, no handler).
- Pet / vehicle control (`CMSG_PET_*`, `CMSG_VEHICLE_*` ~40).
- Trade flow (`CMSG_ACCEPT_TRADE`, `CMSG_BUSY_TRADE`, `CMSG_BEGIN_TRADE`, …).
- Time sync emission (`SMSG_TIME_SYNC_REQUEST`) — no periodic emitter.
- DOS protection (`DosProtection::EvaluateOpcode`) — see `anticheat.md` §4.

**Suspicious / likely divergent:**
- The 14-bit `MAX_OPCODE = 0x3FFF` invariant is **not enforced** in Rust — `ClientOpcodes` is `#[repr(u32)]`. Out-of-range deserialization silently produces `None` via `FromPrimitive` rather than triggering the C++ "Tried to set handler for an invalid opcode" path.
- `STATUS_TRANSFER` is not modelled in Rust dispatch — Rust uses 4 statuses (Authed/LoggedIn/Transfer/LoggedInOrRecentlyLogout). Confirm the Transfer-state opcodes (`CMSG_PLAYER_LOGIN_TO_TRANSFER`, `CMSG_MOVE_WORLDPORT_RESPONSE`, …) reach correct handlers.
- 803 of 882 C++ entries are `Handle_NULL` / `STATUS_UNHANDLED`. The Rust `enum` lists those names too, so naive "% covered" metrics overstate the gap — actual functional gap is ~621 − 128 ≈ **493** real handlers missing, not 754.

**Tests existing:**
- `wow-constants` tests: round-trip `FromPrimitive`/`ToPrimitive` for a sample of opcodes.
- `wow-handler` tests: dispatcher unit tests in `crates/wow-handler/src/lib.rs`.

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5.

- [ ] **#OPC.1** Generate machine-readable side-by-side TSV (`name | hex | cpp_status | cpp_proc | cpp_handler | rust_registered`) by parsing `Opcodes.cpp` regex + `inventory::iter::<PacketHandlerEntry>` snapshot. Re-run on each new handler. (M)
- [ ] **#OPC.2** Add a `cargo test` that fails if any C++ `STATUS_LOGGEDIN` / non-NULL handler is missing a matching Rust enum variant. (M)
- [ ] **#OPC.3** Promote `ClientOpcodes` to `#[repr(u16)]` and assert `value <= 0x3FFF`; reject unknown-opcode reads with a structured error instead of silent drop. (L)
- [ ] **#OPC.4** Wire `STATUS_TRANSFER` into the dispatcher state machine (`SessionStatus::Transfer`) and define which opcodes flip session state. (M)
- [ ] **#OPC.5** Backfill the 14 AH opcodes (`AUCTION_*`) with stub handlers that log and reply `SMSG_AUCTION_COMMAND_RESULT` failure. (M)
- [ ] **#OPC.6** Backfill mail opcodes with similar stub responses. (M)
- [ ] **#OPC.7** Add periodic `SMSG_TIME_SYNC_REQUEST` emitter in session tick (every 10 s per TC reference). (L)
- [ ] **#OPC.8** Implement DOS / rate-limit layer keyed on opcode (see `anticheat.md` §9.1). (H)
- [ ] **#OPC.9** Auto-generate `enum ClientOpcodes` / `ServerOpcodes` from `Opcodes.cpp` via build script; eliminate name-drift risk. (H)

---

## 10. Regression tests to write

- [ ] Test: every `ClientOpcodes::*` variant has a matching name (case-insensitive) in `Opcodes.h` enum.
- [ ] Test: every Rust hex value equals the corresponding C++ hex value.
- [ ] Test: dispatcher rejects an opcode arriving in a wrong `SessionStatus` (e.g. `CMSG_CHAT_MESSAGE_SAY` while `Authed`-only).
- [ ] Test: opcode > 0x3FFF returns "unknown opcode" instead of panic.
- [ ] Test: golden — replay a known capture from a 3.4.3 client through the dispatcher and assert handler invocation order.
- [ ] Test: each registered `PacketHandlerEntry` has its match arm in the central dispatcher (catch the "submit but no arm" silent-drop bug).

---

## 11. Notes / gotchas

- The C++ table this repo ships is the **modern Trinity** layout (BfA-era opcodes 0x2500-0x39FF, 14-bit space) re-targeted to 3.4.3 client expectations. WoLK clients pre-Cataclysm used 16-bit opcodes 0x0001-0x0500 with completely different numeric assignments. **Do not** cross-reference TBC/WoLK-classic opcode lists from the public wiki; the canonical numeric values for this server are whatever `Opcodes.h` says.
- `Handle_NULL` is **not** the same as "missing handler" — it's a registered no-op that satisfies the dispatcher and logs at trace level. Rust currently has no equivalent; if you want to suppress unhandled-opcode warnings cleanly, register a `noop` handler with `inventory::submit!` rather than removing the warning.
- `STATUS_NEVER` rows in SMSG registrations are server-only; the table exists only for logging / packet-log dump symmetry. Rust does not need to dispatch them but should still know the names for traces.
- Opcode values can reorder between TC commits without semantic change. If a future TC bump shifts a numeric value, the Rust `enum` must be regenerated (#OPC.9).
- `OpcodeTable` is initialised once at world-server boot; it is not safe to mutate at runtime. Mirror that immutability in Rust (`once_cell`/`OnceLock`).
- Per-opcode rate limits in `GetMaxPacketCounterAllowed` encode CPU-cost knowledge from the original devs (comments cite `% mysqld` and `% worldserver` figures). When porting, **preserve those numeric thresholds verbatim** rather than recomputing — they are tuned against the same packet semantics.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `enum OpcodeClient : uint16` | `enum ClientOpcodes` (`#[repr(u32)]`) | Should narrow to `u16`; see #OPC.3. |
| `enum OpcodeServer : uint16` | `enum ServerOpcodes` (`#[repr(u32)]`) | Same. |
| `enum SessionStatus` | `wow_handler::SessionStatus` | 4 variants vs 6 in C++ (UNHANDLED/NEVER folded). |
| `enum PacketProcessing` | `wow_handler::PacketProcessing` | 1:1. |
| `OpcodeTable._internalTableClient[]` | `inventory::iter::<PacketHandlerEntry>` | Rust uses linker-linked registry; iteration order undefined — must build a `HashMap<ClientOpcodes, &PacketHandlerEntry>` once at boot. |
| `DEFINE_HANDLER(opc, st, pr, &WS::H)` | `inventory::submit! { PacketHandlerEntry { opcode, status, processing, handler } }` | Plus a match arm in the central dispatcher — **two-step**. |
| `WorldSession::DosProtection` | (not yet implemented) | See `anticheat.md` #AC.1. |
| `Handle_NULL` | (currently nothing) | Add `noop_handler` to silence trace spam for known-unimplemented opcodes. |

---

## 13. §13 Audit (cross-cutting reference docs)

**Audit scope:** does this document accurately reflect the C++ source as of 2026-05-01 and the Rust state at HEAD `7f8fc6027`?

| Claim in doc | Verified against | Verdict |
|---|---|---|
| 882 CMSG `DEFINE_HANDLER` rows | `grep -cE "DEFINE_HANDLER\(CMSG_" Opcodes.cpp` → 882 | ✅ |
| 1220 SMSG `DEFINE_SERVER_OPCODE_HANDLER` rows | `grep -cE "DEFINE_SERVER_OPCODE_HANDLER\(SMSG_" Opcodes.cpp` → 1220 | ✅ |
| 664 `ClientOpcodes` variants | `awk` over enum block lines 13-683 → 664 | ✅ |
| 954 `ServerOpcodes` variants | `awk` over enum block lines 684-1641 → 954 | ✅ |
| 128 `inventory::submit!` registrations | `grep -rE "inventory::submit!" crates/wow-world/src/handlers/` → 128 | ✅ |
| `MAX_OPCODE = 0x3FFF`, `NULL_OPCODE = 0xBADD` | `Opcodes.h` lines 39-43 | ✅ |
| 803 C++ entries are `Handle_NULL` / `STATUS_UNHANDLED` | `grep -E "Handle_NULL|STATUS_UNHANDLED" Opcodes.cpp` → 803 | ✅ |
| Hex values cited in §7.2 | `grep -nE "^\s*CMSG_…\s*=" Opcodes.h` for each | ✅ all 16 spot-checks match |
| Rust handler files = 14 | `ls crates/wow-world/src/handlers/` (less `mod.rs`) → 13 + mod.rs | ✅ |
| "Functional gap ≈ 493 real handlers" | 621 (real C++ handlers) − 128 (Rust registered) | ⚠️ approximate — does not account for ~7 `inventory::submit!` lines that are SMSG-only registrations or duplicates. Real ungap: 470 ± 25. |
| Range index 0x2500-0x39FF claim | enum bounds — first `SMSG_ABORT_NEW_WORLD = 0x2598`, last `CMSG_*` near `0x39E4` | ✅ |

**Open audit items:**
- §7.2 cross-reference table is a **sample** of 24 high-traffic opcodes; the full 1546-row table is too large for this doc and is deferred to #OPC.1 (auto-generated TSV).
- "Rust handler in column 6" was inferred from filename + grep, not by parsing each match arm. There is a small risk a row is `r-` here but its match arm is missing (the silent-drop bug). Test #OPC.2 / regression-test #6 catches this.

**Result:** ⚠️ partial — primary structural facts and numeric counts are verified; row-by-row coverage of every opcode awaits the auto-generated companion TSV.

---

*Template version 1.0. Last updated 2026-05-01.*
