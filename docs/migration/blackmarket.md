# Migration: BlackMarket

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/` + `src/server/game/Handlers/BlackMarketHandler.cpp` + `src/server/game/Server/Packets/BlackMarketPackets.{h,cpp}`
> **Rust target crate(s):** *none yet* — would live as `crates/wow-world/src/blackmarket/` or new `wow-blackmarket` crate. Depends on Mail (`wow-world` mail handlers), `wow-database` (4 prepared statements + 2 tables), `wow-packet` (4 SMSG + 2 CMSG opcodes already enumerated).
> **Layer:** L7 (game systems, optional content)
> **Status:** ❌ not started — **intentionally unported** (post-MoP content); audit confirms 0 lines + dead enum variants
> **Audited vs C++:** ✅ complete
> **Audited vs Rust impl:** ✅ 2026-05-01
> **Last updated:** 2026-06-07
> **WoLK 3.4.3 relevance:** ❌ **Post-WoLK content (Mists of Pandaria 5.4, October 2013).** The Black Market Auction House was added in patch 5.4 and does **not** exist on a vanilla 3.3.5 client. The fact that this file is in the WotLK Classic branch is purely because TrinityCore's WotLK Classic fork was created from the modern (BfA-era) main branch and never deleted MoP+ subsystems. The 3.4.3.54261 client should never send `CMSG_BLACK_MARKET_OPEN` or `..._BID_ON_ITEM` — and the corresponding opcodes (already defined in our `wow-constants/src/opcodes.rs`) are vestigial. **Do not implement for the WoLK Classic relaunch.** Document for completeness, then either no-op or hard-reject the opcodes.

## 2026-06-07 scope freeze

BlackMarket is frozen under the same rule as BattlePets:

- Do not select BlackMarket implementation slices for WotLK progress.
- Do not count BlackMarket work toward the honest WotLK completion percentage.
- Preserve the C++ coverage, packet/opcode inventory, and docs for future MoP+/modern-client targets.
- For the current WotLK target, any BlackMarket opcode/config surface should stay dormant, no-op, or explicitly rejected unless the target version changes.

---

## 1. Purpose

Server-driven auction house in which "rare/cosmetic" items appear on a special NPC vendor at random, with a bid-based timer and a winner-takes-all payout via mail. Unlike the regular AH, players never list items here — listings are picked from the `blackmarket_template` table by weighted roll, capped at `BlackMarket.MaxAuctions`. Each entry has a starting `MinBid`, a duration (typically 24-48h), bidding extends time by 30 min if under 30 min remain, and the winner gets the item via mail with subject `<itemId>:0:1:<marketId>:<quantity>`.

Hard-coded WoW economy constants: 5% min bid increment (`GetMinIncrement = currentBid/20`), max bid 1,000,000 gold (`BMAH_MAX_BID`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/BlackMarket/BlackMarketMgr.cpp` | 521 | `prefix` |
| `game/BlackMarket/BlackMarketMgr.h` | 161 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/BlackMarket/BlackMarketMgr.h` | 161 | `BlackMarketMgr` singleton, `BlackMarketEntry` (live auction), `BlackMarketTemplate` (DB-loaded catalog row), `BlackMarketError` enum, `BMAHMailAuctionAnswers`, `BMAH_MAX_BID = 1000000 * GOLD`. |
| `src/server/game/BlackMarket/BlackMarketMgr.cpp` | 521 | Full implementation: `LoadTemplates` / `LoadAuctions` from DB, `Update`/`RefreshAuctions` (the daily/scheduled refresh that picks new entries via `roll_chance_f` and `Trinity::Containers::RandomResize`), `BuildItemsResponse` (stuffs `BlackMarketRequestItemsResult`), `SendAuctionWonMail`/`SendAuctionOutbidMail`, `BlackMarketEntry::PlaceBid` (mutates state, writes to DB inline). |
| `src/server/game/Handlers/BlackMarketHandler.cpp` (NOT in this folder, but inseparable) | ~110 | `WorldSession::HandleBlackMarketOpen`, `HandleBlackMarketRequestItems`, `HandleBlackMarketBidOnItem` — the only client-facing surface. |
| `src/server/game/Server/Packets/BlackMarketPackets.h` (NOT here, but inseparable) | ~80 | `BlackMarketOpen` (CMSG), `BlackMarketRequestItems` (CMSG), `BlackMarketBidOnItem` (CMSG), `BlackMarketRequestItemsResult` (SMSG, contains `BlackMarketItem[]`), `BlackMarketOutbid` (SMSG), `BlackMarketWon` (SMSG), `BlackMarketBidOnItemResult` (SMSG with `BlackMarketError`). |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `BlackMarketMgr` | singleton | Owns `_templates: unordered_map<int32, BlackMarketTemplate*>` (catalog) and `_auctions: unordered_map<int32, BlackMarketEntry*>` (live), plus `_lastUpdate: time_t` (the world-time anchor for `_secondsRemaining` arithmetic). |
| `BlackMarketTemplate` | struct | `MarketID`, `SellerNPC`, `Quantity`, `MinBid: u64`, `Duration: time_t`, `Chance: f32`, `Item: ItemInstance` (itemId + bonusListIDs). Validates `SellerNPC` + `ItemID` exist on load. |
| `BlackMarketEntry` | class | One live auction: `_marketId`, `_currentBid: u64`, `_numBids`, `_bidder: ObjectGuid::LowType`, `_secondsRemaining: u32` (relative to `_lastUpdate`), `_mailSent: bool`. Owns DB save/load. |
| `BlackMarketError` | enum int32 | Wire result for `BlackMarketBidOnItemResult`: `OK=0, ITEM_NOT_FOUND=1, ALREADY_BID=2, HIGHER_BID=4, DATABASE_ERROR=6, NOT_ENOUGH_MONEY=7, RESTRICTED_ACCOUNT_TRIAL=9`. |
| `BMAHMailAuctionAnswers` | enum | `OUTBID=0, WON=1` — embedded in mail subject. |

Constants: `BMAH_MAX_BID = 1000000 * GOLD = 1e10` copper; `MIN_INCREMENT = floor(currentBid/20) - (floor(currentBid/20) % GOLD)` — i.e. 5% rounded *down* to whole gold; if `< 30 min` remain, place-bid extends by 30 min.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `BlackMarketMgr::LoadTemplates()` | One inline `SELECT marketId, sellerNpc, itemEntry, quantity, minBid, duration, chance, bonusListIDs FROM blackmarket_template`. Validates each row's seller and item via `sObjectMgr`. | `WorldDatabase`, `sObjectMgr` |
| `BlackMarketMgr::LoadAuctions()` | Prepared `CHAR_SEL_BLACKMARKET_AUCTIONS` against `character` DB. Sets `_lastUpdate = GameTime::GetGameTime()` *before* loading so each entry's `_secondsRemaining` is computed from the same anchor. Drops completed entries. | `CharacterDatabase`, `BlackMarketEntry::LoadFromDB` |
| `BlackMarketMgr::Update(updateTime)` | Walks `_auctions`; if completed and bidder set, sends won-mail; if `updateTime`, calls `entry->Update(now)` to decrement `_secondsRemaining` by `(now - _lastUpdate)`, then sets `_lastUpdate = now`. | `SendAuctionWonMail`, `BlackMarketEntry::Update` |
| `BlackMarketMgr::RefreshAuctions()` | The "daily refresh": delete all completed entries, then for each template not currently live, roll `roll_chance_f(template->Chance)`, collect into a list, `RandomResize` to `BlackMarket.MaxAuctions`, instantiate `BlackMarketEntry`, persist via `SaveToDB`. Calls `Update(true)` at the end. | `Trinity::Containers::RandomResize`, `BlackMarketEntry::SaveToDB` |
| `BlackMarketMgr::IsEnabled()` | Reads `CONFIG_BLACKMARKET_ENABLED` from `sWorld`. Used by the handler before responding to client. | `sWorld` |
| `BlackMarketMgr::BuildItemsResponse(packet, player)` | Fills `BlackMarketRequestItemsResult.Items[]` — one entry per live auction. `MinIncrement = 1` if no bids yet, else `entry->GetMinIncrement()`. `HighBid = (bidder == player.guid.counter)`. | `BlackMarketEntry`, `BlackMarketTemplate` |
| `BlackMarketMgr::SendAuctionWonMail(entry, trans)` | Creates the won item via `Item::CreateItem(itemId, qty, ItemContext::Black_Market)`, persists, GM-trade-logs if applicable, sends `SMSG_BLACK_MARKET_WON` + mail with subject `BuildAuctionMailSubject(WON)` and body `<sellerNPC>:<currentBid>`. | `Item::CreateItem`, `MailDraft::SendMailTo`, `WorldSession::SendBlackMarketWonNotification` |
| `BlackMarketMgr::SendAuctionOutbidMail(entry, trans)` | Sends `SMSG_BLACK_MARKET_OUTBID` to the previous bidder if connected, mail-refunds the previous `_currentBid` (yes — full refund of their last bid), call **before** updating `_bidder`. | `MailDraft::AddMoney`, `WorldSession::SendBlackMarketOutbidNotification` |
| `BlackMarketEntry::Update(newTimeOfUpdate)` | `_secondsRemaining -= (newTimeOfUpdate - sBlackMarketMgr->GetLastUpdate())`. Note: signed underflow if `_secondsRemaining` got below the elapsed delta — uses `uint32` so the wrap is the bug-by-design that `IsCompleted` catches with `<= 0` (signed comparison). |
| `BlackMarketEntry::GetSecondsRemaining()` | `_secondsRemaining - (now - _lastUpdate)` — recomputed every call relative to the manager's anchor. |
| `BlackMarketEntry::ValidateBid(bid)` | `bid > _currentBid && bid >= _currentBid + GetMinIncrement() && bid < BMAH_MAX_BID`. |
| `BlackMarketEntry::PlaceBid(bid, player, trans)` | Modifies `_currentBid` / `_numBids`/ `_bidder`, **adds 30 min** if `< 30 min` remained, deducts `bid` copper from player, `UPDATE` via `CHAR_UPD_BLACKMARKET_AUCTIONS`, then triggers `sBlackMarketMgr->Update(true)`. **Note: the outbid-mail to the previous bidder is sent by the handler, NOT by this method.** |
| `BlackMarketEntry::BuildAuctionMailSubject(response)` | `"<itemId>:0:<response>:<marketId>:<quantity>"` — the client parses this for the in-mail item display. Don't change the format. |

---

## 5. Module dependencies

**Depends on:**
- `AuctionHouseMgr` — *no direct dependency* (Black Market is parallel infra), but the mail subject/body convention is similar.
- `Mail` — `MailDraft::SendMailTo`, `MailDraft::AddItem`/`AddMoney`. Heavy reliance on mail infra.
- `Item` — `Item::CreateItem(id, qty, ItemContext::Black_Market)`. The `ItemContext::Black_Market = 15` value matters for client display.
- `ObjectMgr` — template validation (creature & item).
- `CharacterCache` — `GetCharacterAccountIdByGuid`, `GetCharacterNameByGuid` for offline-bidder lookup.
- `World` — `CONFIG_BLACKMARKET_ENABLED`, `CONFIG_BLACKMARKET_MAXAUCTIONS`.
- `WorldDatabase` — `blackmarket_template` table.
- `CharacterDatabase` — `blackmarket_auctions` table + 4 prepared statements.
- `Player::ModifyMoney`, `Player::GetSession`, RBAC for `RBAC_PERM_LOG_GM_TRADE`.

**Depended on by:**
- `WorldSession::HandleBlackMarketOpen` — opens the AH UI when interacting with seller NPC.
- `WorldSession::HandleBlackMarketRequestItems` — sends the listings.
- `WorldSession::HandleBlackMarketBidOnItem` — places a bid.
- `WorldSession::SendBlackMarketWonNotification` / `SendBlackMarketOutbidNotification` — toast popups.
- A `RefreshAuctions` cron tick somewhere in `World::Update` (configurable interval, typically once per realm reset).

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| inline `SELECT marketId, sellerNpc, itemEntry, quantity, minBid, duration, chance, bonusListIDs FROM blackmarket_template` | Catalog load. | world |
| `CHAR_SEL_BLACKMARKET_AUCTIONS` | Live auctions load. | character |
| `CHAR_INS_BLACKMARKET_AUCTIONS` | Insert new live entry. | character |
| `CHAR_UPD_BLACKMARKET_AUCTIONS` | Update bid/expiration. | character |
| `CHAR_DEL_BLACKMARKET_AUCTIONS` | Delete completed/invalid. | character |

DB tables: `blackmarket_template` (world), `blackmarket_auctions` (character). **Neither table exists in 3.3.5 / classic-WotLK schemas — confirmed.** Stock TC `world` & `characters` SQL dumps for the WotLK Classic branch include them with schema; vanilla 3.3.5a TrinityCore (the LK reference) does not.

DB2: the `Item.ItemBonus` field references `ItemBonus.db2` (also MoP+ data). The 3.4.3 client does not send `ItemBonus` in regular `ItemInstance` reads at all.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_BLACK_MARKET_OPEN` (0x352B in our table) | client → server | `WorldSession::HandleBlackMarketOpen` — sent when interacting with seller NPC. |
| `CMSG_BLACK_MARKET_REQUEST_ITEMS` | client → server | `HandleBlackMarketRequestItems` — refresh button. |
| `CMSG_BLACK_MARKET_BID_ON_ITEM` | client → server | `HandleBlackMarketBidOnItem`. |
| `SMSG_BLACK_MARKET_REQUEST_ITEMS_RESULT` (0x2627) | server → client | `BlackMarketMgr::BuildItemsResponse` |
| `SMSG_BLACK_MARKET_BID_ON_ITEM_RESULT` (0x2628) | server → client | After `ValidateBid`/`PlaceBid`, returns `BlackMarketError`. |
| `SMSG_BLACK_MARKET_OUTBID` (0x2629) | server → client | To the previous bidder when outbid. |
| `SMSG_BLACK_MARKET_WON` (0x262A) | server → client | To winner when auction ends. |

All four SMSG opcodes are already listed in `crates/wow-constants/src/opcodes.rs` (lines 804-807). The two CMSGs are not — `CMSG_BLACK_MARKET_OPEN` at 0x352B is in the file but the request/bid CMSGs aren't. Not a concern for this doc since the recommendation is to NOT implement BMAH on the WoLK Classic relaunch.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/blackmarket` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-blackmarket` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *None.* No `wow-blackmarket` crate, no `blackmarket` module, no DB statements registered, no handlers.
- The four SMSG opcodes are defined in `crates/wow-constants/src/opcodes.rs` (~lines 804-807) — these are **dead enum variants**.
- `ItemContext::Black_Market = 15` is in `crates/wow-constants/src/item.rs` (line 291). Also dead.

**What's implemented:**
- Nothing on the wire path; nothing on the storage path; no handlers; no manager; no scheduled refresh.

**What's missing vs C++:**
- Everything. But also: this is an item we should **deliberately leave missing** for the WoLK Classic 3.4.3 target.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — there's nothing to diverge from.
- If the 3.4.3 client never sends these opcodes (it shouldn't), we have free choice between (a) no-op stubs that log on receipt, (b) hard disconnect on receipt as malformed, (c) gate the SMSG path entirely so the server can never originate them. Recommendation: **(a)** — log+drop, since a third-party private-server client mod might attempt them.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#BLACKMARKET.WBS.001** Partir y cerrar la migracion auditada de `game/BlackMarket/BlackMarketMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`
  Rust target: `crates/wow-blackmarket`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 521 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BLACKMARKET.WBS.002** Cerrar la migracion auditada de `game/BlackMarket/BlackMarketMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h`
  Rust target: `crates/wow-blackmarket`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

**Top-level decision:** all sub-tasks below are **deprioritized to "tier-4 / never"** unless we explicitly decide to enable BMAH on the relaunch. If the answer is "never", #BLACKMARKET.0 is sufficient.

- [ ] **#BLACKMARKET.0** Add a no-op handler that logs + drops `CMSG_BLACK_MARKET_OPEN`/`_REQUEST_ITEMS`/`_BID_ON_ITEM`. Clarifies for future readers that BMAH is intentionally unported. Mark the four SMSG opcodes with `#[allow(dead_code)]` or move to a `legacy_disabled` module. (L)
- [ ] **#BLACKMARKET.1** *(if implementing)* Confirm the 3.4.3 client actually accepts the WotLK Classic re-encoded packet shape. The TC packet code under `BlackMarketPackets.cpp` was authored against 9.x-era opcodes; the WotLK-Classic fork rewrote the wire serialization. Verify before any other work. (M)
- [ ] **#BLACKMARKET.2** *(if implementing)* Port `BlackMarketTemplate` + `BlackMarketEntry` POD as plain Rust structs in a new `wow-blackmarket` crate. Keep `MarketID: i32` (yes, signed, matches DB). (L)
- [ ] **#BLACKMARKET.3** *(if implementing)* Register the four character-DB prepared statements (`CHAR_SEL/INS/UPD/DEL_BLACKMARKET_AUCTIONS`) and the inline world-DB `blackmarket_template` SELECT. (M)
- [ ] **#BLACKMARKET.4** *(if implementing)* Port `BlackMarketMgr::LoadTemplates` + `LoadAuctions` with the same "set _lastUpdate before loading" timing — many subtle correctness bugs hide there. (M)
- [ ] **#BLACKMARKET.5** *(if implementing)* Port `RefreshAuctions` with the per-template `roll_chance_f` and `RandomResize(maxAuctions)`. (M)
- [ ] **#BLACKMARKET.6** *(if implementing)* Port `Update(updateTime)` + `BlackMarketEntry::Update` (decrement-by-elapsed); test for the underflow-by-design behavior with a rolled-clock fixture. (M)
- [ ] **#BLACKMARKET.7** *(if implementing)* Port `BuildItemsResponse` packet writer; add unit tests against a captured 5.4-era reference packet. (M)
- [ ] **#BLACKMARKET.8** *(if implementing)* Port handlers `HandleBlackMarketOpen`/`Request`/`BidOnItem` with `IsEnabled` gate, GM-trade log, money deduction, mail flow. (H)
- [ ] **#BLACKMARKET.9** *(if implementing)* Port `SendAuctionWonMail` / `SendAuctionOutbidMail`, including the `Item::CreateItem(..., ItemContext::Black_Market)` flag. (M)
- [ ] **#BLACKMARKET.10** *(if implementing)* Schedule `RefreshAuctions` on a configurable cron via the world tick — TC ties it to "next reset" by default. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#BLACKMARKET.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 682 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-database && cargo test -p wow-packet && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#BLACKMARKET.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 682 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#BLACKMARKET.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 682 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#BLACKMARKET.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 682 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

(All optional unless implementation is decided to proceed.)

- [ ] Test: `BlackMarketEntry::GetMinIncrement()` rounds 5%-of-current down to whole gold. (`current=99g50s → minIncrement=4g`; `current=200g → minIncrement=10g`; `current=23g → minIncrement=1g`.)
- [ ] Test: `ValidateBid(bid)` rejects `bid <= _currentBid`, rejects `bid < _currentBid + minIncrement`, rejects `bid >= BMAH_MAX_BID = 1e10`.
- [ ] Test: `PlaceBid` extends `_secondsRemaining` by 30 min iff `< 30 min` remained; otherwise leaves it unchanged.
- [ ] Test: `Update(updateTime=true)` decrements `_secondsRemaining` by `(now - lastUpdate)` exactly; second consecutive call with same `now` is a no-op.
- [ ] Test: `RefreshAuctions` never re-creates a still-live MarketID (i.e. it skips templates with `GetAuctionByID(id)`).
- [ ] Test: `RefreshAuctions` honors `RandomResize(maxAuctions)` cap.
- [ ] Test: `SendAuctionWonMail` sets `_mailSent = true` and is idempotent on second call.
- [ ] Test: `SendAuctionOutbidMail` mail body contains the **previous** `_currentBid` (refund), not the new one.
- [ ] Test: Mail subject format is exactly `<itemId>:0:<response>:<marketId>:<quantity>` with literal colons.
- [ ] Test (negative): `IsEnabled = false` → handlers reply with appropriate error / drop without DB writes.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 2 files / 682 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h` | *none yet* — would live as `crates/wow-world/src/blackmarket/` or new `wow-blackmarket` crate. Depends on Mail (`wow-world` mail handlers), `wow-database` (4 prepared statements + 2 tables), `wow-packet` (4 SMSG + 2 CMSG opcodes already enumerated). \| ❌ not started — **intentionally unported** (post-MoP content); audit confirms 0 lines + dead enum variants |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#BLACKMARKET.DIV.001` | `crates/wow-world/src/blackmarket` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 682 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#BLACKMARKET.DIV.002` | `crates/wow-blackmarket` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 682 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BlackMarket/BlackMarketMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **MoP content in a WotLK Classic codebase.** The single most important note: this is **5.4 Pandaria** infra ported back into the modern TC main branch and inherited by the Wrath Classic fork. The 3.4.3 client should never originate these CMSGs. If it does, treat as suspect — likely a modded client.
- **`_secondsRemaining` is `uint32` but compared as signed.** `IsCompleted() return GetSecondsRemaining() <= 0;` returns a `uint32` from `GetSecondsRemaining`, but the `<= 0` triggers signed-int promotion in C++. Underflow wraps to a huge positive value, and `<= 0` becomes false — so a wrapped entry "looks alive forever" until the next `Update` recomputes. Watch for this; in Rust, prefer `i64` end-time math.
- **`_lastUpdate` anchor is shared.** Every `BlackMarketEntry`'s `GetSecondsRemaining` is computed against the manager's `_lastUpdate`. If you split the manager across threads, treat `_lastUpdate` as the synchronization point.
- **Outbid mail is sent before mutation.** TC documents this: `SendAuctionOutbidMail` is called *before* `PlaceBid` updates `_bidder`, because the mail goes to the *previous* bidder. The handler is responsible for ordering, not `PlaceBid` itself. Easy to get wrong on a port.
- **Mail subject parsing.** The client parses the subject string for inline display. Format `<itemId>:0:<response>:<marketId>:<quantity>` — the literal `:0:` middle is not a typo, it's reserved (probably a stack delta in MoP that ended up unused).
- **`BMAH_MAX_BID = 1000000 * GOLD`** is 10^10 copper, well within `u64` but past `i32`. Use `u64` end-to-end. The C++ code mixes `int64_t` for `Player::ModifyMoney(-bid)` and `uint64_t` everywhere else; mind the negation.
- **`Item::CreateItem(..., ItemContext::Black_Market = 15)`.** The context enum value is checked client-side for some UI hints; don't substitute another context.
- **Five-percent increment rounding.** `(current/20) - (current/20) % GOLD` — i.e., compute increment, then strip remainder mod 1g (10000 copper). Don't simplify to `floor(current * 0.05)`.
- **Performance.** Catalog (`_templates`) is small (<200 entries on retail). Live auctions are also small (≤ `BlackMarket.MaxAuctions`, typically 12). Linear iteration is fine.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class BlackMarketMgr` (singleton) | `pub struct BlackMarketMgr` + `OnceLock<RwLock<BlackMarketMgr>>` | `RwLock` because `RefreshAuctions` mutates while readers (`BuildItemsResponse`) want shared. |
| `unordered_map<int32, BlackMarketTemplate*> _templates` | `HashMap<i32, BlackMarketTemplate>` | Templates are immutable post-load — store by value, no `Box`. |
| `unordered_map<int32, BlackMarketEntry*> _auctions` | `HashMap<i32, BlackMarketEntry>` | Mutable; consider `DashMap` if reads contend with bidder updates. |
| `time_t _lastUpdate` | `i64` (epoch seconds) or `chrono::DateTime<Utc>` | Mind sign for the `_secondsRemaining` math; `i64` is simpler. |
| `BlackMarketTemplate::Item: WorldPackets::Item::ItemInstance` | `wow_packet::ItemInstance` (whatever the existing struct is) | Reuse the AuctionHouse-side ItemInstance. |
| `class BlackMarketEntry` | `pub struct BlackMarketEntry` with methods | No inheritance, no virtuals. |
| `ObjectGuid::LowType _bidder` (uint64) | `u64` (raw counter) | Wrap in `Option<NonZeroU64>` if you want compile-time "no bidder" distinction. |
| `enum BlackMarketError : int32` | `#[repr(i32)] pub enum BlackMarketError { ... }` | Match wire size exactly. |
| `MailDraft(...).AddItem(item).SendMailTo(...)` | Existing mail builder in `wow-world::mail` | Reuse, do not duplicate. |
| `roll_chance_f(template->Chance)` | `rng.gen::<f32>() * 100.0 < template.chance` | Match TC semantics — Chance is 0..100 not 0..1. |
| `Trinity::Containers::RandomResize(list, n)` | `wow_collections::random_resize` (or shuffle + `.truncate(n)`) | Worth a small helper; used in many TC subsystems. |
| `CHAR_*_BLACKMARKET_AUCTIONS` | New entries in `wow-database::statements::character` | Four prepared statements + the world-DB inline SELECT. |
| `Item::CreateItem(id, qty, ItemContext::Black_Market)` | Existing item factory in `wow-world::entities::item` | Pass context as `ItemContext::BlackMarket` enum variant. |
| `Player::ModifyMoney(-bid)` | `wow_world::session::with_player(|p| p.modify_money(-bid as i64))` | Beware sign cast. |
| `sBlackMarketMgr` | `wow_blackmarket::mgr()` returning `&'static RwLock<BlackMarketMgr>` | Or via dependency injection from world tick. |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines `BlackMarketMgr` Rust impl | ✅ | `grep -rn "BlackMarket\|black_market\|BMAH" crates/wow-world crates/wow-handler → 0` (no struct, no global, no handler) |
| Dead enum variants exist in opcodes | ✅ | 5 hits, all in `crates/wow-constants/src/opcodes.rs`: `BlackMarketOpen` (CMSG `0x352b`), `BlackMarketBidOnItemResult` `0x2628`, `BlackMarketOutbid` `0x2629`, `BlackMarketRequestItemsResult` `0x2627`, `BlackMarketWon` `0x262a` (SMSG). Plus item-class `BlackMarket = 15` at `item.rs:291`. Doc says 4 SMSG + 2 CMSG; reality is 4 SMSG + **1 CMSG** (only `BlackMarketOpen`; `BidOnItem` CMSG is missing). |
| No DB tables / prepared statements | ✅ | grep `blackmarket_template\|blackmarket_auctions` in `crates/wow-database` → 0 |
| WoLK 3.4.3 client never sends these | ✅ confirmed by spec | MoP 5.4 feature; doc's recommendation to "no-op or hard-reject" is correct |

**Silent-hang risk:** none. The 3.4.3 client doesn't have a Black Market UI. The CMSG `0x352b` will never arrive. The dead variants in `opcodes.rs` are vestigial constants only — leaving them as dead code is fine, or remove for hygiene.

---

*Template version: 1.0 (2026-05-01).* If this gets implemented, flip Status & update Last updated; otherwise leave as a permanent "intentionally unported, here's why" entry.
