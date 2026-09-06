# Migration: AuctionHouse

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/` + `src/server/game/Handlers/AuctionHouseHandler.cpp` + `src/server/game/Server/Packets/AuctionHousePackets.{h,cpp}`
> **Rust target crate(s):** `crates/wow-world/` (handlers + AuctionHouseMgr global), `crates/wow-database/` (prepared statements), `crates/wow-packet/` (auction packet types)
> **Layer:** L6
> **Status:** ⚠️ effectively not started — only an `AuctionHelloResponse` open-window stub + 3 noop list handlers (audit confirmed)
> **Audited vs C++:** ✅ complete
> **Audited vs Rust impl:** ✅ 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-faction auction house system: lets players list items (12h/24h/48h durations), browse via bucket/search filters, place bids, buyout, cancel listings, and receive items/money via mail when the auction ends. Handles a deposit fee, owner cut from sale, outbid refund, expiration tick, and a "commodity" flow for stackable items where multiple postings of the same item are aggregated and bought by quantity. There are four AuctionHouseObject instances (Alliance, Horde, Neutral, Goblin) so that hostile factions cannot interact with each other's auctions, plus the goblin neutral house for cross-faction trade.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/AuctionHouse/AuctionHouseMgr.cpp` | 1923 | `prefix` |
| `game/AuctionHouse/AuctionHouseMgr.h` | 430 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/AuctionHouse/AuctionHouseMgr.h` | 430 | `AuctionHouseMgr` singleton, `AuctionHouseObject` (per-faction), `AuctionPosting`, `AuctionsBucketKey/Data`, `CommodityQuote`, `AuctionThrottleResult`, enums (`AuctionResult`, `AuctionCommand`, `AuctionMailType`, `AuctionHouseFilterMask`, `AuctionHouseSortOrder`, `AuctionHouseListType`) |
| `src/server/game/AuctionHouse/AuctionHouseMgr.cpp` | 1923 | All auction logic: `LoadAuctions`, `AddAuction`, `RemoveAuction`, `Update` (expiration tick), `BuildList*`, `CreateCommodityQuote`, `BuyCommodity`, `SendAuction{Outbid,Won,Sold,Expired,Removed,Cancelled,Invoice}`, deposit/cut math, mail subject/body builders, throttle |
| `src/server/game/Handlers/AuctionHouseHandler.cpp` | 1124 | All CMSG handlers: Hello, BrowseQuery, ListBuckets, ListItems, ListBidded, ListOwned, PlaceBid, RemoveItem, SellItem, SellCommodity, GetCommodityQuote, ConfirmCommoditiesPurchase, CancelCommoditiesPurchase, ReplicateItems, RequestFavoriteList, SetFavoriteItem, plus legacy WotLK ListBidder/ListOwner/ListPendingSales/ListItems |
| `src/server/game/Server/Packets/AuctionHousePackets.h` | ~700 | All AuctionHouse wire structs: `AuctionItem`, `AuctionBucketKey`, `BucketInfo`, `AuctionSortDef`, `AuctionOwnerNotification`, `AuctionBidderNotification`, plus every CMSG/SMSG packet class |
| `src/server/game/Server/Packets/AuctionHousePackets.cpp` | ~1100 | Read/Write implementations for the above |

DBC dependency: `AuctionHouse.db2` (entries 1=Alliance, 2=Horde, 6=Goblin, 7=Neutral) — provides `factionTemplateID`, `depositRate`, `consignmentRate` per house.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `AuctionHouseMgr` | singleton | Global owner of the four `AuctionHouseObject`s, `_itemsByGuid` (live `Item*`s held in escrow), pending-auction queue per player, replication-id generator, throttle table |
| `AuctionHouseObject` | class | One per faction. Holds `_itemsByAuctionId` (ordered map for replicate cursor), `_buckets` (search index), `_soldItemsById` (kept post-removal for invoice), `_commodityQuotes`, owner/bidder reverse indices |
| `AuctionPosting` | struct | A single live auction: `Id`, `Bucket*`, `Items[]`, `Owner`, `OwnerAccount`, `Bidder`, `MinBid`, `BuyoutOrUnitPrice`, `Deposit`, `BidAmount`, `StartTime`, `EndTime`, `ServerFlags`, `BidderHistory` (for outbid mail) |
| `AuctionsBucketKey` | struct | `(itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId)` — primary search key. WotLK rarely uses pet/suffix |
| `AuctionsBucketData` | struct | Aggregated info per bucket: `MinPrice`, `QualityCounts`, `ItemClass/SubClass`, full localized name, plus list of `Auctions` posted under this key |
| `CommodityQuote` | struct | Per-player aggregated commodity quote: `TotalPrice`, `Quantity`, `ValidTo` (5s window) |
| `AuctionThrottleResult` | struct | `(DelayUntilNext, Throttled)` — anti-DoS for browse/sell |
| `AuctionResult` | enum int8 | Wire result code: `Ok=0, Inventory=1, DatabaseError=2, NotEnoughMoney=3, ItemNotFound=4, HigherBid=5, BidIncrement=7, BidOwn=10, RestrictedAccountTrial=13, HasRestriction=17, AuctionHouseBusy=18, AuctionHouseUnavailable=19, CommodityPurchaseFailed=21, ItemHasQuote=23` |
| `AuctionCommand` | enum int8 | `SellItem=0, Cancel=1, PlaceBid=2` (for `SMSG_AUCTION_COMMAND_RESULT`) |
| `AuctionMailType` | enum int32 | `Outbid=0, Won=1, Sold=2, Expired=3, Removed=4, Cancelled=5, Invoice=6` (mail subject discriminator) |
| `AuctionHouseFilterMask` | enum flag | `UncollectedOnly, UsableOnly, UpgradesOnly, ExactMatch, PoorQuality...ArtifactQuality, LegendaryCraftedItemOnly` |
| `AuctionHouseSortOrder` | enum uint8 | `Price, Name, Level, Bid, Buyout` |
| `AuctionHouseBrowseMode` | enum uint8 | `Search=0, SpecificKeys=1` |
| `AuctionHouseListType` | enum uint8 | `Commodities=1, Items=2` |
| `AuctionPostingServerFlag` | enum flag | `GmLogBuyer=0x1` (for GM transaction logging) |
| `AuctionSearchClassFilters` | struct | `Classes[MAX_ITEM_CLASS]` × `(SubclassMask, InvTypes[])` — class/subclass filter from client |
| `PlayerReplicateThrottleData` | struct | `(Global, Cursor, Tombstone, NextAllowedReplication)` — cursor for bulk-listing replicate |

Constants: `MIN_AUCTION_TIME = 12 * HOUR`, `MAX_FAVORITE_AUCTIONS = 100`, `Browse=500`, `Items=50` (result limits).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `AuctionHouseMgr::LoadAuctions()` | Startup: loads `auctionhouse`, `auction_items`, `auction_bidders` rows; rebuilds `_itemsByGuid`, faction houses, buckets | `CharacterDatabase::Query`, `Item::LoadFromDB`, `AuctionHouseObject::AddAuction` |
| `AuctionHouseMgr::Update()` | Tick all 4 houses + cleanup throttle table | `AuctionHouseObject::Update`, `UpdatePendingAuctions` |
| `AuctionHouseMgr::GetAuctionsMap(factionTemplateId)` | Resolve which house an NPC belongs to (uses `AuctionHouseEntry`) | `GetAuctionHouseEntry` |
| `AuctionHouseMgr::GetItemAuctionDeposit(player, item, time)` | Deposit = vendor sell price × depositRate × (time / 12h). Money-locked if cancel/expire | `ItemTemplate::SellPrice`, `AuctionHouseEntry::DepositRate` |
| `AuctionHouseMgr::GetCommodityAuctionDeposit(item, time, qty)` | Same formula × quantity for commodity stacks | — |
| `AuctionHouseMgr::AddAItem(item)` | Take item out of player inventory, store in `_itemsByGuid` (escrow) | — |
| `AuctionHouseMgr::RemoveAItem(guid, deleteItem)` | Pop item from escrow; optionally delete from DB | `Item::RemoveFromDB` |
| `AuctionHouseMgr::PendingAuctionAdd(player, ahId, auctId, deposit)` | Queue an auction whose money is on hold (offline-safe) | per-player pending list |
| `AuctionHouseMgr::PendingAuctionProcess(player)` | On login: charge deposit if affordable, else cancel pending | `Player::ModifyMoney`, `CHAR_DEL_AUCTION` |
| `AuctionHouseMgr::CheckThrottle(player, addonTainted, command)` | Returns delay+throttled flag (100 queries / period) | `_playerThrottleObjects` |
| `AuctionHouseMgr::GenerateReplicationId()` | Monotonic id for AH replicate cursor | — |
| `AuctionHouseObject::AddAuction(trans, posting)` | Persist new auction; insert into `_itemsByAuctionId`, register bucket, write `auctionhouse`+`auction_items` rows | `CHAR_INS_AUCTION`, `CHAR_INS_AUCTION_ITEMS` |
| `AuctionHouseObject::RemoveAuction(trans, auction)` | Tear down: erase from bucket, owner/bidder indices, `_itemsByAuctionId`; mark items removed; write `CHAR_DEL_AUCTION` | — |
| `AuctionHouseObject::Update()` | Iterate `_itemsByAuctionId`; for each whose `EndTime <= now` either `SendAuctionWon` (had bidder) or `SendAuctionExpired`, then `RemoveAuction` | `SendAuction*`, `RemoveAuction` |
| `AuctionHouseObject::BuildListBuckets(...)` | Filtered/sorted bucket page for browse query | `AuctionsBucketData::BuildBucketInfo`, `Sorter` |
| `AuctionHouseObject::BuildListAuctionItems(... bucketKey ...)` | Listings within one bucket | `AuctionPosting::BuildAuctionItem` |
| `AuctionHouseObject::BuildListBiddedItems(...)` | All postings the player is bidding on | `_playerBidderAuctions` |
| `AuctionHouseObject::BuildListOwnedItems(...)` | All postings player owns | `_playerOwnedAuctions` |
| `AuctionHouseObject::BuildReplicate(...)` | Bulk replicate (legacy "GetAll") with cursor + tombstone | throttle map |
| `AuctionHouseObject::CalculateAuctionHouseCut(bid)` | Owner cut deducted from sale: `bid × consignmentRate` | DB2 |
| `AuctionHouseObject::CreateCommodityQuote(player, itemId, qty)` | Aggregate cheapest postings to fill `qty`; return total + 5s validity | `_buckets` |
| `AuctionHouseObject::BuyCommodity(trans, player, itemId, qty, delay)` | Atomically deduct money, split stacks, mail items+gold to seller(s), schedule outbid/won mails | `Item::CloneItem`, `MailDraft`, `RemoveAuction` |
| `AuctionHouseObject::SendAuctionOutbid(auction, newBidder, newBidAmount, trans)` | Refund prior bidder via mail, send `SMSG_AUCTION_OUTBID_NOTIFICATION` if online | `MailDraft`, session->SendPacket |
| `AuctionHouseObject::SendAuctionWon(auction, bidder, trans)` | Mail items to winner, send won notification | `MailDraft`, `SMSG_AUCTION_WON_NOTIFICATION` |
| `AuctionHouseObject::SendAuctionSold(auction, owner, trans)` | Mail money (bid − cut) to seller, post-delay 1h | `MailDraft` |
| `AuctionHouseObject::SendAuctionExpired(auction, trans)` | Return items to seller via mail (no money); body=`Expired` | `MailDraft` |
| `AuctionHouseObject::SendAuctionRemoved(auction, owner, trans)` | Player-cancel: refund items + active high-bid to bidder, deposit to seller (or kept) | `MailDraft` (×2 if had bidder) |
| `AuctionHouseObject::SendAuctionInvoice(auction, owner, trans)` | Optional invoice mail with breakdown | `MailDraft` |
| `AuctionPosting::BuildAuctionItem(...)` | Wire-pack a posting into `AuctionItem` (with censorship for non-owners) | `Item::BuildItemInstance`, enchant/gem reads |
| `AuctionPosting::CalculateMinIncrement(bid)` | `max(1g, bid × 0.05)` rounded to silver | — |
| `AuctionsBucketKey::ForItem/ForCommodity` | Build search key from `Item*` or `ItemTemplate*` | `Item::GetItemLevel` |
| `WorldSession::HandleAuctionHelloOpcode` | Validates auctioneer NPC is interactable + correct faction → `SMSG_AUCTION_HELLO_RESPONSE` | `Player::GetNPCIfCanInteractWith` |
| `WorldSession::HandleAuctionSellItem` | Validates inventory, faction, deposit ≤ player gold; charges deposit; calls `AuctionHouseMgr::AddAItem` + `AuctionHouseObject::AddAuction`; replies `SMSG_AUCTION_COMMAND_RESULT` | many |
| `WorldSession::HandleAuctionPlaceBid` | Validates bid > current+increment; refund prior bidder via outbid mail; updates `BidAmount`/`Bidder`; if bid==buyout → instant `SendAuctionWon`+`RemoveAuction` | many |
| `WorldSession::HandleAuctionRemoveItem` | Cancel: only owner, only if no bid (or refund bidder); returns items+deposit | `SendAuctionRemoved` |
| `WorldSession::HandleAuctionListBucketsByBucketKeys` / `...BucketKey` / `...ItemID` / `...BiddedItems` / `...OwnedItems` | Wire entry points to `AuctionHouseObject::BuildList*` | throttle, then BuildList* |
| `WorldSession::HandleAuctionReplicateItems` | Bulk replicate with cursor (rate-limited, 1/min) | `BuildReplicate` |
| `WorldSession::HandleAuctionSellCommodity` / `GetCommodityQuote` / `ConfirmCommoditiesPurchase` / `CancelCommoditiesPurchase` | Commodity flow: post stack of stackable, get quote, confirm purchase | `BuyCommodity`, `CreateCommodityQuote` |
| `WorldSession::HandleAuctionSetFavoriteItem` / `RequestFavoriteList` | Per-character favorite bucket list (cap 100), persisted in `character_auction_favorites` (account-bound table) | DB |
| `AuctionHouseMgr::BuildItemAuctionMailSubject(type, auction)` | `"<itemId>:0:<type>:<auctionId>:<itemCount>:<battlePetSpeciesId>:0:0:<context>"` | — |
| `AuctionHouseMgr::BuildAuctionWonMailBody(guid, bid, buyout)` | `"<bidder>:<bid>:<buyout>"` | — |
| `AuctionHouseMgr::BuildAuctionSoldMailBody(guid, bid, buyout, deposit, consignment)` | `"<owner>:<bid>:<buyout>:<deposit>:<consignment>"` | — |

---

## 5. Module dependencies

**Depends on:**
- `Mails` — every auction lifecycle event sends a `MailDraft` (won, sold, expired, outbid, cancel-refund, invoice)
- `Entities/Player` — `Player::ModifyMoney`, inventory removal, item-can-be-traded check, account/realm validation
- `Entities/Item` — items are escrow-owned by `AuctionHouseMgr` while listed; `Item::SaveToDB`, `BuildItemInstance`
- `DB2/AuctionHouse.db2` — faction template → house id, deposit rate, consignment rate
- `Loot` (indirect via items) — N/A directly
- `CharacterDatabase` — prepared statements `CHAR_*_AUCTION*`
- `ObjectMgr` — `GenerateAuctionID`, `GetCreatureTemplate` for auctioneer faction lookup
- `GameTime` — endtime calculations (`GameTime::GetSystemTime()`)
- `Battlenet/AccountMgr` — owner-account guid resolution for cross-account checks
- `RBAC` — restricted-account-trial gating

**Depended on by:**
- `AuctionHouseBot` — reads/writes through `AuctionHouseObject` to fill the market
- `Cheat/GM commands` — `cs_ahbot.cpp`, `cs_auction*.cpp`
- `WorldSession` — opcode dispatch (~25 handlers)
- `Mail expiration` — auction-mail uses `MAIL_AUCTION` type with a 1-hour empty-mail expire if no attachments

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_AUCTIONS` | `SELECT id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags FROM auctionhouse` | character |
| `CHAR_INS_AUCTION` | Insert new auction row | character |
| `CHAR_DEL_AUCTION` | `DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_items ai ON a.id = ai.auctionId LEFT JOIN auction_bidders ab ON a.id = ab.auctionId WHERE a.id = ?` | character |
| `CHAR_UPD_AUCTION_BID` | `UPDATE auctionhouse SET bidder=?, bidAmount=?, serverFlags=? WHERE id=?` | character |
| `CHAR_UPD_AUCTION_EXPIRATION` | `UPDATE auctionhouse SET endTime=? WHERE id=?` | character |
| `CHAR_SEL_AUCTION_ITEMS` | Loads `auction_items` (auctionId,itemGuid) | character |
| `CHAR_INS_AUCTION_ITEMS` | Inserts auction-item link | character |
| `CHAR_SEL_AUCTION_BIDDERS` | Loads `auction_bidders` (auctionId,playerGuid) for outbid history | character |
| `CHAR_INS_AUCTION_BIDDER` | Persists a bidder to history | character |
| `CHAR_SEL_AUCTIONHOUSE_ITEM_BY_ENTRY` | GM lookup: `SELECT ai.itemGuid, c.guid, c.account, c.name FROM auctionhouse ah INNER JOIN auction_items ai ... INNER JOIN characters c ... WHERE ii.itemEntry = ? LIMIT ?` | character |
| `CHAR_SEL_CHARACTER_AUCTION_FAVORITES` | Load per-character favorites | character |
| `CHAR_INS_CHARACTER_AUCTION_FAVORITE` | Add favorite | character |
| `CHAR_DEL_CHARACTER_AUCTION_FAVORITE` | Remove favorite | character |
| `CHAR_INS_MAIL` / `CHAR_INS_MAIL_ITEM` | Indirect: every auction settlement creates a mail row | character |

Tables: `auctionhouse`, `auction_items`, `auction_bidders`, `character_auction_favorites`. Related (settlement output): `mail`, `mail_items`.

| Store | What it loads | Read by |
|---|---|---|
| `AuctionHouseStorage` | `AuctionHouse.db2` | `AuctionHouseMgr::GetAuctionHouseEntry`, deposit/cut math |
| `FactionTemplateStorage` | `FactionTemplate.dbc` | NPC → faction → house id resolution |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_AUCTION_HELLO_REQUEST` | client → server | `HandleAuctionHelloOpcode` |
| `SMSG_AUCTION_HELLO_RESPONSE` | server → client | `HandleAuctionHelloOpcode` |
| `CMSG_AUCTION_BROWSE_QUERY` | client → server | `HandleAuctionBrowseQuery` |
| `CMSG_AUCTION_LIST_BUCKETS_BY_BUCKET_KEYS` | client → server | `HandleAuctionListBucketsByBucketKeys` |
| `SMSG_AUCTION_LIST_BUCKETS_RESULT` | server → client | `BuildListBuckets` |
| `CMSG_AUCTION_LIST_ITEMS_BY_BUCKET_KEY` | client → server | `HandleAuctionListItemsByBucketKey` |
| `CMSG_AUCTION_LIST_ITEMS_BY_ITEM_ID` | client → server | `HandleAuctionListItemsByItemID` |
| `SMSG_AUCTION_LIST_ITEMS_RESULT` | server → client | `BuildListAuctionItems` |
| `CMSG_AUCTION_LIST_OWNED_ITEMS` / `CMSG_AUCTION_LIST_OWNER_ITEMS` (legacy) | client → server | `HandleAuctionListOwnedItems` / `HandleAuctionListOwnerItems` |
| `SMSG_AUCTION_LIST_OWNED_ITEMS_RESULT` / `SMSG_AUCTION_LIST_OWNER_ITEMS_RESULT` | server → client | `BuildListOwnedItems` |
| `CMSG_AUCTION_LIST_BIDDED_ITEMS` / `CMSG_AUCTION_LIST_BIDDER_ITEMS` (legacy) | client → server | `HandleAuctionListBiddedItems` / `HandleAuctionListBidderItems` |
| `SMSG_AUCTION_LIST_BIDDED_ITEMS_RESULT` / `SMSG_AUCTION_LIST_BIDDER_ITEMS_RESULT` | server → client | `BuildListBiddedItems` |
| `CMSG_AUCTION_LIST_PENDING_SALES` | client → server | `HandleAuctionListPendingSales` |
| `SMSG_AUCTION_LIST_PENDING_SALES_RESULT` | server → client | empty in WotLK |
| `CMSG_AUCTION_PLACE_BID` | client → server | `HandleAuctionPlaceBid` |
| `CMSG_AUCTION_REMOVE_ITEM` | client → server | `HandleAuctionRemoveItem` |
| `CMSG_AUCTION_SELL_ITEM` | client → server | `HandleAuctionSellItem` |
| `CMSG_AUCTION_SELL_COMMODITY` | client → server | `HandleAuctionSellCommodity` |
| `CMSG_AUCTION_GET_COMMODITY_QUOTE` | client → server | `HandleAuctionGetCommodityQuote` |
| `CMSG_AUCTION_CONFIRM_COMMODITIES_PURCHASE` | client → server | `HandleAuctionConfirmCommoditiesPurchase` |
| `CMSG_AUCTION_CANCEL_COMMODITIES_PURCHASE` | client → server | `HandleAuctionCancelCommoditiesPurchase` |
| `SMSG_AUCTION_GET_COMMODITY_QUOTE_RESULT` | server → client | `HandleAuctionGetCommodityQuote` |
| `CMSG_AUCTION_REPLICATE_ITEMS` | client → server | `HandleAuctionReplicateItems` |
| `SMSG_AUCTION_REPLICATE_RESPONSE` | server → client | `BuildReplicate` |
| `CMSG_AUCTION_REQUEST_FAVORITE_LIST` | client → server | `HandleAuctionRequestFavoriteList` |
| `SMSG_AUCTION_FAVORITE_LIST` | server → client | response |
| `CMSG_AUCTION_SET_FAVORITE_ITEM` | client → server | `HandleAuctionSetFavoriteItem` |
| `SMSG_AUCTION_COMMAND_RESULT` | server → client | every action result (sell/cancel/bid) |
| `SMSG_AUCTION_OUTBID_NOTIFICATION` | server → client | `SendAuctionOutbid` (online bidder) |
| `SMSG_AUCTION_OWNER_BID_NOTIFICATION` | server → client | first-bid notify |
| `SMSG_AUCTION_WON_NOTIFICATION` | server → client | `SendAuctionWon` (online winner) |
| `SMSG_AUCTION_CLOSED_NOTIFICATION` | server → client | end-of-auction tile refresh |
| `SMSG_AUCTION_DISABLE_NEW_POSTINGS` | server → client | maintenance flag |
| `CMSG_AUCTIONABLE_TOKEN_SELL` / `..._AT_MARKET_PRICE` | client → server | stub, not available in WotLK |
| `SMSG_AUCTIONABLE_TOKEN_SELL_AT_MARKET_PRICE_RESPONSE` / `_SELL_CONFIRM_REQUIRED` / `_AUCTION_SOLD` | server → client | unhandled |

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
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-packet/src/packets/misc.rs` (~lines 1900-2010) — only `AuctionHelloResponse` builder + 3 empty-shell result packets (`AuctionListBidderItemsResult`, `AuctionListOwnerItemsResult`, `AuctionListPendingSalesResult`) — covers ~5% of the wire surface
- `crates/wow-world/src/handlers/misc.rs` — placeholder dispatch entries for `AuctionListBidderItems`, `AuctionListOwnerItems`, `AuctionListPendingSales` (all stub)
- `crates/wow-constants/src/opcodes.rs` — `AuctionHelloRequest/Response` and a handful of legacy opcodes are declared

**What's implemented:**
- Opening packet stub: client sees an open auction window when opcoded, but no listings appear

**What's missing vs C++:**
- `AuctionHouseMgr` global (no faction houses, no escrow, no replication ids, no throttle)
- `AuctionPosting`, `AuctionsBucketKey`, `AuctionsBucketData`, `CommodityQuote` types
- All 25 opcode handlers (sell/bid/cancel/list/browse/commodity/replicate/favorites)
- Deposit/cut formulas, `MIN_AUCTION_TIME`, 12h/24h/48h tiers
- Expiration tick (`AuctionHouseObject::Update`)
- Mail integration for won/sold/expired/outbid/cancelled/invoice
- DB schema bindings: `auctionhouse`, `auction_items`, `auction_bidders`, `character_auction_favorites`
- DB2 `AuctionHouseStorage`
- Pending-auctions queue (offline-safe deposit)
- Commodity flow (post-cataclysm; for WotLK 3.4.x classic this is partially gated but client still emits opcodes)
- Favorite list (per-character cap 100)
- Replicate cursor + 1/min throttle
- GM transaction logging (`AuctionPostingServerFlag::GmLogBuyer`)

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The lone `AuctionHelloResponse::open` always sets `auction_house_id=7` (neutral) — wrong for Alliance/Horde-faction auctioneers; needs DB2 lookup
- No throttle → trivial DoS via spam-browse
- Item escrow not implemented → posting an item leaves it duplicable

**Tests existing:**
- 0 tests under `crates/wow-world/`, `crates/wow-database/` for AH

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#AUCTIONHOUSE.WBS.001** Partir y cerrar la migracion auditada de `game/AuctionHouse/AuctionHouseMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1923 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSE.WBS.002** Cerrar la migracion auditada de `game/AuctionHouse/AuctionHouseMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#AH.1** Define `AuctionResult`, `AuctionCommand`, `AuctionMailType`, `AuctionHouseFilterMask`, `AuctionHouseSortOrder`, `AuctionHouseListType`, `AuctionPostingServerFlag` enums in `crates/wow-constants/` (L)
- [ ] **#AH.2** Define `AuctionsBucketKey` with `ForItem`/`ForCommodity` constructors and `Hash` impl (L)
- [ ] **#AH.3** Define `AuctionPosting` struct with all fields + `is_commodity()`, `total_item_count()`, `calc_min_increment()` (M)
- [ ] **#AH.4** Define `AuctionsBucketData` with quality counts, min price, sort level, locale-aware full-name array (M)
- [ ] **#AH.5** Define `CommodityQuote` + 5s validity logic (L)
- [ ] **#AH.6** Define `AuctionThrottleResult` + `PlayerReplicateThrottleData` (L)
- [ ] **#AH.7** Add prepared-statement IDs `CHAR_SEL_AUCTIONS`, `CHAR_INS_AUCTION`, `CHAR_DEL_AUCTION`, `CHAR_UPD_AUCTION_BID`, `CHAR_UPD_AUCTION_EXPIRATION`, `CHAR_SEL_AUCTION_ITEMS`, `CHAR_INS_AUCTION_ITEMS`, `CHAR_SEL_AUCTION_BIDDERS`, `CHAR_INS_AUCTION_BIDDER`, `CHAR_SEL_AUCTIONHOUSE_ITEM_BY_ENTRY` to `crates/wow-database/` (M)
- [ ] **#AH.8** Add `character_auction_favorites` SELECT/INSERT/DELETE prepared statements (L)
- [ ] **#AH.9** Add `AuctionHouseStorage` (DB2 `AuctionHouse.db2`) loader to `crates/wow-data/`; expose `(faction_template_id → entry, deposit_rate, consignment_rate)` (M)
- [ ] **#AH.10** Implement `AuctionHouseMgr::get_auction_house_entry(faction_template_id)` returning `(entry, house_id)` (L)
- [ ] **#AH.11** Implement `AuctionHouseObject` per-faction (Alliance/Horde/Neutral/Goblin) holding `items_by_auction_id` (BTreeMap for ordered replicate), `buckets` (BTreeMap), `sold_items_by_id`, `commodity_quotes`, `player_owned_auctions`, `player_bidder_auctions` (H)
- [ ] **#AH.12** Implement `AuctionHouseMgr` global with the 4 houses, `items_by_guid` escrow, replication-id generator, throttle table, pending auctions per player (H)
- [ ] **#AH.13** Implement `AuctionHouseMgr::load_auctions()` (startup): query `auctionhouse` + `auction_items` + `auction_bidders`, rebuild structures (M)
- [ ] **#AH.14** Implement `AuctionHouseObject::add_auction(trans, posting)`: INSERT, register bucket, owner/bidder index update (M)
- [ ] **#AH.15** Implement `AuctionHouseObject::remove_auction(trans, &posting)`: tear down indexes + DB delete (M)
- [ ] **#AH.16** Implement `AuctionHouseObject::update()` expiration tick: iterate items_by_auction_id, settle Won/Expired (H)
- [ ] **#AH.17** Implement `AuctionHouseMgr::get_item_auction_deposit(player, item, time)` and `get_commodity_auction_deposit(item, time, qty)` per DB2 rates (M)
- [ ] **#AH.18** Implement `AuctionHouseObject::calculate_auction_house_cut(bid)` per DB2 consignment rate (L)
- [ ] **#AH.19** Implement `AuctionPosting::calculate_min_increment(bid)` = max(1g, 5%) rounded (L)
- [ ] **#AH.20** Implement deposit/refund flow: charge on post, refund on cancel (no bid), forfeit on expire-with-bid (M)
- [ ] **#AH.21** Implement mail subject/body builders: `BuildItemAuctionMailSubject`, `BuildCommodityAuctionMailSubject`, `BuildAuctionMailSubject`, `BuildAuctionWonMailBody`, `BuildAuctionSoldMailBody`, `BuildAuctionInvoiceMailBody` (M)
- [ ] **#AH.22** Implement `send_auction_won/sold/expired/outbid/removed/cancelled_to_bidder/invoice` (mail emissions) — depends on `Mails` migration (H)
- [ ] **#AH.23** Implement `pending_auction_add/process/update_pending_auctions` for offline deposit-affordability gating (M)
- [ ] **#AH.24** Implement `check_throttle(player, addon_tainted, command)` with 100 queries/period rolling window (M)
- [ ] **#AH.25** Implement `generate_replication_id` monotonic counter (L)
- [ ] **#AH.26** Define wire packets in `crates/wow-packet/src/packets/auction.rs`: `AuctionItem`, `AuctionBucketKey` (wire), `BucketInfo`, `AuctionSortDef`, `AuctionOwnerNotification`, `AuctionBidderNotification` (M)
- [ ] **#AH.27** Define request packets (deserialize): `AuctionHelloRequest`, `AuctionBrowseQuery`, `AuctionListBucketsByBucketKeys`, `AuctionListItemsByBucketKey`, `AuctionListItemsByItemID`, `AuctionListOwnedItems`, `AuctionListBiddedItems`, `AuctionPlaceBid`, `AuctionRemoveItem`, `AuctionSellItem`, `AuctionSellCommodity`, `AuctionGetCommodityQuote`, `AuctionConfirmCommoditiesPurchase`, `AuctionCancelCommoditiesPurchase`, `AuctionReplicateItems`, `AuctionRequestFavoriteList`, `AuctionSetFavoriteItem` (H)
- [ ] **#AH.28** Define response packets (serialize): `AuctionHelloResponse` (replace stub w/ DB2-correct house_id), `AuctionListBucketsResult`, `AuctionListItemsResult`, `AuctionListOwnedItemsResult`, `AuctionListBiddedItemsResult`, `AuctionGetCommodityQuoteResult`, `AuctionReplicateResponse`, `AuctionCommandResult`, `AuctionOutbidNotification`, `AuctionWonNotification`, `AuctionOwnerBidNotification`, `AuctionClosedNotification`, `AuctionFavoriteList` (H)
- [ ] **#AH.29** Implement `WorldSession::HandleAuctionHelloOpcode`: NPC-faction → house lookup, throttle, send hello response (L)
- [ ] **#AH.30** Implement `HandleAuctionSellItem` (validation: faction, money, can-be-traded, max-price=`MAX_MONEY_AMOUNT`, escrow item, deposit charge, post) (H)
- [ ] **#AH.31** Implement `HandleAuctionSellCommodity` (commodity stack post) (M)
- [ ] **#AH.32** Implement `HandleAuctionPlaceBid` (validate inc, refund prior, set bidder; if buyout → instant won) (H)
- [ ] **#AH.33** Implement `HandleAuctionRemoveItem` (cancel; refund; mail items+deposit) (M)
- [ ] **#AH.34** Implement `HandleAuctionListBucketsByBucketKeys` + `BuildListBuckets(keys)` variant (M)
- [ ] **#AH.35** Implement `HandleAuctionBrowseQuery` (free-text search variant of BuildListBuckets) (H)
- [ ] **#AH.36** Implement `HandleAuctionListItemsByBucketKey` and `HandleAuctionListItemsByItemID` + `BuildListAuctionItems` (M)
- [ ] **#AH.37** Implement `HandleAuctionListOwnedItems` + `BuildListOwnedItems` (M)
- [ ] **#AH.38** Implement `HandleAuctionListBiddedItems` + `BuildListBiddedItems` (M)
- [ ] **#AH.39** Implement `HandleAuctionGetCommodityQuote` + `CreateCommodityQuote` (M)
- [ ] **#AH.40** Implement `HandleAuctionConfirmCommoditiesPurchase` + `BuyCommodity` (split stacks, mail to all sellers, charge buyer) (XL)
- [ ] **#AH.41** Implement `HandleAuctionCancelCommoditiesPurchase` + `CancelCommodityQuote` (L)
- [ ] **#AH.42** Implement `HandleAuctionReplicateItems` + `BuildReplicate` (cursor, tombstone, 60s throttle) (H)
- [ ] **#AH.43** Implement favorites: `HandleAuctionRequestFavoriteList`, `HandleAuctionSetFavoriteItem` (cap 100, persist) (M)
- [ ] **#AH.44** Stub `HandleAuctionableTokenSell` / `HandleAuctionableTokenSellAtMarketPrice` (WoW Token N/A in WotLK) (L)
- [ ] **#AH.45** Wire `AuctionHouseMgr::update()` into world tick loop (`World::Update`) (L)
- [ ] **#AH.46** Implement `AddAItem`/`RemoveAItem` escrow integration with `Player::Inventory` (M)
- [ ] **#AH.47** Add GM logging via `AuctionPostingServerFlag::GmLogBuyer` (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#AUCTIONHOUSE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 2353 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-database && cargo test -p wow-packet && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#AUCTIONHOUSE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 2353 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#AUCTIONHOUSE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 2353 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#AUCTIONHOUSE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 2353 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: posting an item moves it to escrow (item no longer in player bag, owner GUID unchanged on `item_instance` row)
- [ ] Test: deposit = `vendor_sell_price × deposit_rate × (time / 12h)` — verify per house DB2 rate
- [ ] Test: cancelling an auction with no bidder refunds full deposit + items via mail
- [ ] Test: cancelling with active bidder refunds bidder via outbid mail and seller via removed mail
- [ ] Test: bid below `current_bid + min_increment` rejected with `BidIncrement`
- [ ] Test: bidding own auction rejected with `BidOwn`
- [ ] Test: bid == buyout immediately settles (won mail to bidder, sold mail to owner — minus cut)
- [ ] Test: expiration after 12h with no bid → expired mail returns items, no money transfer
- [ ] Test: expiration with bid → items to bidder, money (minus cut) to owner
- [ ] Test: outbid sends `SMSG_AUCTION_OUTBID_NOTIFICATION` to online prior bidder + refund mail
- [ ] Test: cross-faction NPC interaction blocked (`AuctionHouseUnavailable`)
- [ ] Test: throttle returns `Throttled=true` after 100 queries in period
- [ ] Test: replicate cursor delivers all items, eventually returns tombstone == cursor
- [ ] Test: commodity quote aggregates cheapest postings to fill quantity, valid 5s
- [ ] Test: commodity confirm executes only if quote still valid; expired quote → `ItemHasQuote`
- [ ] Test: pending-auction queue: post while logged-in spending more than gold-on-hand → cancelled at next login
- [ ] Test: favorite list cap 100; 101st insert rejected
- [ ] Test: load roundtrip — INSERT → restart → SELECT_AUCTIONS rebuilds identical state
- [ ] Test: trial-account restriction blocks posting/bidding (`RestrictedAccountTrial`)
- [ ] Test: mail subject format matches `<itemId>:0:<type>:<auctionId>:<count>:<petSpecies>:0:0:<context>`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 2353 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h` | `crates/wow-world/` (handlers + AuctionHouseMgr global), `crates/wow-database/` (prepared statements), `crates/wow-packet/` (auction packet types) \| ⚠️ effectively not started — only an `AuctionHelloResponse` open-window stub + 3 noop list handlers (audit confirmed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#AUCTIONHOUSE.DIV.001` | _none generated_ | 2 C++ files / 2353 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouse/AuctionHouseMgr.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- WotLK 3.4.3 is the "classic" build; **commodities and bucket-search were retrofitted** from retail. The client emits `CMSG_AUCTION_BROWSE_QUERY` regardless, so the server must support it. The legacy opcodes (`AUCTION_LIST_OWNER_ITEMS`, `AUCTION_LIST_BIDDER_ITEMS`, `AUCTION_LIST_PENDING_SALES`, `AUCTION_LIST_ITEMS`) are also still routed in WotLK but receive empty/legacy payloads.
- Four `AuctionHouseObject`s exist: Alliance(1), Horde(2), Goblin(6, neutral mid-tier), Neutral(7). Goblin is rarely used in WotLK 3.4.x but the slot persists.
- `MIN_AUCTION_TIME = 12 * HOUR`. Tiers in client are 12/24/48h. The deposit scales linearly by `(time / 12h)` so 48h costs 4× the base.
- `AuctionPosting::CalculateMinIncrement(bid) = max(1g, bid × 0.05)` rounded down to silver — the `(_currentBid / 20) - ((_currentBid / 20) % SILVER)` pattern matters for client validation; off-by-1c bugs exist historically.
- `_soldItemsById` is kept after `RemoveAuction` because the **invoice mail** (sent ~1h after sale) needs to reference the sold posting; do not free immediately.
- `AuctionHouseMgr::Update` loops faction houses in fixed order — order matters for deterministic replicate IDs across restarts (used by clients to detect changes).
- `AuctionPostingServerFlag::GmLogBuyer` is set when seller is GM-flagged, so when buyer purchases we still log even if buyer is offline (avoids sync DB query).
- `BidderHistory` is needed because outbid mail must go to **all** prior bidders, not just the immediately-previous one (multi-bid edge case in commodity flow).
- The 1-hour mail delivery delay applies to auction mail when items move between accounts (anti-RMT). For same-account auctions delivery is immediate.
- Empty mail (no items, no money) gets a 1-hour expiry (`MAIL_AUCTION` empty-notice) instead of 30 days — consistent with `Mails` module.
- Throttle uses `addonTainted` flag: client addons that drive AH lookups get a stricter rate.
- The `auction_bidders` table exists specifically to persist `BidderHistory` across restarts so outbid refunds survive a crash mid-bid.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `AuctionHouseMgr` (singleton) | `static AUCTION_HOUSE_MGR: OnceCell<AuctionHouseMgr>` in `wow-world::auction::mgr` | Match the `MapManager` global pattern already used |
| `AuctionHouseObject` (per faction) | `struct AuctionHouseObject` (4 instances inside Mgr) | Use `parking_lot::RwLock` for the inner maps |
| `std::map<uint32, AuctionPosting>` | `BTreeMap<u32, AuctionPosting>` | Order matters for replicate cursor |
| `std::unordered_map<AuctionsBucketKey, AuctionsBucketData>` | `HashMap<AuctionsBucketKey, AuctionsBucketData>` | Custom `Hash` per `AuctionsBucketKey::Hash` |
| `std::unordered_multimap<ObjectGuid, uint32>` | `HashMap<ObjectGuid, SmallVec<[u32; 4]>>` | Player→auctionIDs reverse index |
| `AuctionPosting::Items: std::vector<Item*>` | `Vec<ItemHandle>` (escrow ref) | Items live in `AuctionHouseMgr::items_by_guid` |
| `EnumFlag<AuctionPostingServerFlag>` | `bitflags!` | — |
| `std::wstring` (full localized name) | `[String; TOTAL_LOCALES]` | Per-locale lowercased name for prefix search |
| `SystemTimePoint StartTime/EndTime` | `chrono::DateTime<Utc>` | Persisted as `unix_timestamp` |
| `Milliseconds DelayUntilNext` | `std::time::Duration` | — |
| `CharacterDatabaseTransaction` | `wow_database::CharTrans` | Already abstracted |
| `WorldPackets::AuctionHouse::*` | `wow_packet::packets::auction::*` | Mirror the C++ struct layout exactly |
| `void Foo::Update()` | `fn update(&mut self, now: DateTime<Utc>)` | Pass time explicitly for testability |
| `MailDraft::SendMailTo` | `wow_world::mail::MailDraft::send` | Cross-module call |
| `Player::ModifyMoney(int64)` | `wow_world::Player::modify_money(i64) -> Result<()>` | Money cap enforced |
| `sAuctionMgr` macro | `auction_mgr()` accessor returning `&'static AuctionHouseMgr` | — |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines `AuctionHouseMgr` Rust impl | ✅ | `grep -rn "AuctionHouseMgr\|AuctionHouseObject" crates/ → 0` (no struct, no global) |
| Auction packet types absent | ⚠️ partial | Only `AuctionHelloResponse` exists in `crates/wow-packet/src/packets/misc.rs:1907` (returns hardcoded `auction_house_id: 7` neutral) |
| No auction handlers wired | ⚠️ partial | 1 hello + 3 noop list handlers exist: `handle_auction_hello_request` (sends `AuctionHelloResponse`), `handle_auction_list_bidder_items`/`_owner_items`/`_pending_sales` (all `_pkt: WorldPacket {}` body) at `handlers/misc.rs:620-634`, dispatched at `session.rs:1538,1848-1854` |
| 25 auction opcodes in `wow-constants` | ✅ | `grep -c "Auction" crates/wow-constants/src/opcodes.rs` = 25 |
| No DB schema/prepared statements | ✅ | grep `auction` in `crates/wow-database` → 0 hits |

**Net status:** opcode constants + a single "open empty window" packet exist. Clicking an auctioneer NPC opens the UI but every list/post/bid action is silently dropped. Doc's `❌ not started` is accurate at the system level; reclassified to ⚠️ in YAML to flag the small surface area already wired.

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.
