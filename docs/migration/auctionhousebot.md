# Migration: AuctionHouseBot (AHBot)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/`
> **Rust target crate(s):** *none yet* — would live under `crates/wow-world/src/auctionhousebot/` (alongside the future auction crate) or a new `wow-ahbot` crate. Depends on a working `AuctionHouseMgr` (see `auctionhouse.md`), which is also not yet ported.
> **Layer:** L7 (game systems, opt-in fixture; depends on L6 AuctionHouse + L4 DB2 stores)
> **Status:** ❌ not started (audit confirmed 2026-05-01)
> **Audited vs C++:** ✅ complete
> **Audited vs Rust impl:** ✅ 2026-05-01
> **Last updated:** 2026-05-01
> **WoLK 3.4.3 relevance:** ⚠️ **Post-WoLK content from upstream TC.** AHBot was contributed long after vanilla TC and the Wrath Classic fork inherits it as `AuctionHouseBot.Enabled = false` by default; the legacy 3.3.5 server emulators (mangos/SkyFire/etc.) shipped a different, simpler AHBot. None of the bot's auction-creation paths are needed for a "vanilla WoLK PvE realm" — list this as "tier-3, opt-in, after AuctionHouse is fully working." See `worldserver.conf.dist` AHBot section for the live runtime knobs.

---

## 1. Purpose

Server-side bot that populates the three auction houses (Alliance, Horde, Neutral) with synthetic listings and, optionally, places bids/buyouts on real player listings — useful on small/empty realms so the AH never looks dead. The seller half scans every `ItemTemplate`, partitions items by class/quality, and continuously tops up each house up to a configurable target count using owner GUIDs drawn from a single "AHBot" account. The buyer half iterates real auctions and rolls a configurable price-vs-vendor-value chance to bid or buy out.

It is purely a fixture: it never originates new packets and never receives client opcodes. Everything happens via direct `AuctionHouseObject` mutation from the world tick.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/AuctionHouseBot/AuctionHouseBot.cpp` | 559 | `prefix` |
| `game/AuctionHouseBot/AuctionHouseBot.h` | 321 | `prefix` |
| `game/AuctionHouseBot/AuctionHouseBotBuyer.cpp` | 454 | `prefix` |
| `game/AuctionHouseBot/AuctionHouseBotBuyer.h` | 99 | `prefix` |
| `game/AuctionHouseBot/AuctionHouseBotSeller.cpp` | 924 | `prefix` |
| `game/AuctionHouseBot/AuctionHouseBotSeller.h` | 152 | `prefix` |
| `game/AuctionHouseBot/enuminfo_AuctionHouseBot.cpp` | 121 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/AuctionHouseBot/AuctionHouseBot.h` | 321 | `AuctionBotConfig` singleton (~110 config keys via three enum-indexed arrays), `AuctionBotAgent` abstract base, `AuctionHouseBot` holder/dispatcher, `AuctionHouseBotStatusInfoPerType` for `.ahbot` GM commands. Also re-defines `AuctionQuality`/`AuctionHouseType`. |
| `src/server/game/AuctionHouseBot/AuctionHouseBot.cpp` | 559 | Config file parsing (`SetConfig` / `SetConfigMax` / `SetConfigMinMax`), AHBot-character GUID list resolution from `CHAR_SEL_CHARS_BY_ACCOUNT_ID`, `Initialize()`, `Update()` round-robin over `2*MAX_AUCTION_HOUSE_TYPE` operations, `Rebuild(all)`, `PrepareStatusInfos` for `.ahbot status`. |
| `src/server/game/AuctionHouseBot/AuctionHouseBotSeller.h` | 152 | `SellerConfiguration` (per-house min/max time, items per quality×class, price ratios, random-stack ratios), `AuctionBotSeller` class. |
| `src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp` | 924 | `Initialize()` filters every `ItemTemplate` against vendor/loot/include/exclude/bonding rules → builds `_itemPool[quality][class]`. `SetStat()` counts existing AH items per (quality,class), computes "missing", `GetItemsToSell()` rolls weighted picks honoring class priority, `AddNewAuctions()` creates `AuctionPosting`s with random duration/stack/buyout/bid, `SetPricesOfItem()` does deposit-aware pricing. |
| `src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.h` | 99 | `BuyerAuctionEval`, `BuyerItemInfo` (rolling avg per item template), `BuyerConfiguration`, `AuctionBotBuyer` class. |
| `src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp` | 454 | `Update()` per house: `GetItemInformation` aggregates real auctions into `SameItemInfo`, `PrepareListOfEntry` ages out old eval entries, `BuyAndBidItems` re-rolls `RollBuyChance`/`RollBidChance` per eligible auction, `BuyEntry`/`PlaceBidToEntry` mutate `AuctionPosting` and call `SendAuctionWonMail`/`SendAuctionOutbidMail` directly. |
| `src/server/game/AuctionHouseBot/enuminfo_AuctionHouseBot.cpp` | 121 | Auto-generated `EnumUtils` reflection for `AuctionQuality` / `AuctionHouseType`. |

Plus the GM command surface in `src/server/scripts/Commands/cs_ahbot.cpp` (`.ahbot status/items/ratio/rebuild/reload`) — not in the AHBot directory itself, but inseparable from it.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `AuctionBotConfig` | singleton | Reads `AuctionHouseBot.*` keys from `worldserver.conf` via `sConfigMgr`; stores in three flat arrays indexed by `AuctionBotConfigUInt32Values`/`...BoolValues`/`...FloatValues` enums. Owns `_AHBotCharacters: vector<ObjectGuid>` resolved from one account. |
| `AuctionBotAgent` | abstract | Two virtuals: `Initialize()`, `Update(AuctionHouseType)`. Implemented by Seller and Buyer. |
| `AuctionHouseBot` | singleton | Owns `_buyer`, `_seller`, `_operationSelector` (round-robin index 0..5). `Update()` runs *one* successful step per call. |
| `AuctionBotSeller : AuctionBotAgent` | class | Per-house `SellerConfiguration[3]`, `_itemPool[MAX_AUCTION_QUALITY][MAX_ITEM_CLASS]: vector<uint32>` of pre-filtered item ids. |
| `AuctionBotBuyer : AuctionBotAgent` | class | Per-house `BuyerConfiguration[3]`. `_checkInterval` controls how often a single house is re-scanned. |
| `SellerConfiguration` | struct | Item-amount targets per (quality,class), price ratios, random-stack ratios, min/max auction duration. `LastMissedItem` is a roving cursor for fairness. |
| `SellerItemInfo` | POD | `{ AmountOfItems, MissItems }` per (quality,class) — drives "how many to add this cycle". |
| `BuyerConfiguration` | struct | `BuyerEnabled`, `SameItemInfo: map<itemId, BuyerItemInfo>`, `EligibleItems: map<auctionId, BuyerAuctionEval>`. |
| `BuyerItemInfo` | POD | Aggregate stats across all live auctions for the same itemId: bid/buy counts, min prices, totals — used for "how does this auction compare to market". |
| `BuyerAuctionEval` | POD | Per-auction sticky eval state: `LastChecked` (re-roll throttle), `LastExist` (TTL for cleanup). |
| `AuctionHouseBotStatusInfoPerType` | struct | `.ahbot status` output: total items + per-quality counts. |
| `AuctionQuality` | enum | Shadow of `ItemQualities`, capped at 7 (artifact); skips heirloom. |
| `AuctionHouseType` | enum | `NEUTRAL=0, ALLIANCE=1, HORDE=2`. **Note the order mismatch** vs. `AuctionHouseIds[3] = {1,2,6}` (DB2 ids 1=Alliance, 2=Horde, 6=Neutral). |
| `AuctionBotConfigUInt32Values` | enum | Names every numeric config slot — about 100 entries. |
| `AuctionBotConfigBoolValues` | enum | About 30 boolean slots (per-class allow-zero, bind filters, buyer/seller enables). |
| `AuctionBotConfigFloatValues` | enum | 3 floats (chance factor, bid min/max). |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `AuctionBotConfig::Initialize()` | Parse conf, abort if both seller & buyer disabled, resolve `_AHBotCharacters` from `CHAR_SEL_CHARS_BY_ACCOUNT_ID`. | `sConfigMgr`, `CharacterDatabase` |
| `AuctionBotConfig::GetRandChar()` / `GetRandCharExclude(g)` | Pick a random AHBot owner GUID; the exclude variant is used so a bidder ≠ the owner. | `Trinity::Containers::SelectRandomContainerElement` |
| `AuctionBotConfig::IsBotChar(guid)` | True if guid is empty (owner-less) **or** belongs to the AHBot account. Used everywhere to gate "this auction belongs to the bot, mutate it freely". | linear scan of `_AHBotCharacters` |
| `AuctionHouseBot::Initialize()` | Load config; if seller enabled instantiate `AuctionBotSeller` & call its `Initialize()` (heavy: scans every `ItemTemplate`). Same for buyer. | `AuctionBotConfig::Initialize`, agent ctors |
| `AuctionHouseBot::Update()` | Tick from `World::Update`. Round-robin: indexes 0..2 are seller per house, 3..5 are buyer per house. **Only one successful step per outer `Update()`** — bumps the selector regardless. | `_seller->Update`, `_buyer->Update` |
| `AuctionHouseBot::Rebuild(bool all)` | GM command: expire all bot auctions now (`EndTime = GetSystemTime()`), or only those without bids. | `sAuctionMgr->GetAuctionsById`, sets `EndTime` |
| `AuctionHouseBot::PrepareStatusInfos(map&)` | Walk the three houses, count items whose Owner is bot-owned. | `sAuctionMgr` |
| `AuctionBotSeller::Initialize()` | Build `_itemPool` from `sItemStore`. Filters: includeItems, excludeItems, vendor/loot allowlist, bonding flags, allowZero per class, min/max item-level/req-level/skill, plus per-class glyph/mount/tradegood/container item-level windows. Quality `>= MAX_AUCTION_QUALITY` is dropped. | `sObjectMgr->GetItemTemplate`, `sObjectMgr->GetNpcVendorItemList`, `WorldDatabase.PQuery` (UNION of 11 loot tables), `sItemStore` |
| `AuctionBotSeller::SetStat(config)` | Count existing AH items per (quality,class) for this house, compute `MissItems`. | `sAuctionMgr->GetAuctionsById` |
| `AuctionBotSeller::GetItemsToSell(...)` | Roll weighted picks honoring per-class priority sums and per-quality counters; bounded by `ItemsPerCycle.Boost/Normal`. | `urand`, internal weight tables |
| `AuctionBotSeller::AddNewAuctions(config)` | For each picked item: pick template from `_itemPool[q][c]`, roll stack size (`GetStackSizeForItem`), random duration in `[MinTime,MaxTime]` hours, compute `bid`/`buyout` via `SetPricesOfItem`, build `AuctionPosting`, push directly into `AuctionHouseObject`. | `sAuctionMgr`, `Item::CreateItem`, all DB writes via the auction layer |
| `AuctionBotSeller::SetPricesOfItem(...)` | `bid = floor(buyout * frand(BidPriceMin, BidPriceMax))`; buyout = price * quality ratio * class ratio * house ratio / 1e6. Falls back to `SellPrice * 4` if no `BuyPrice`, unless `BuyPrice.Seller=true`. | `urand`/`frand`, `ItemTemplate::GetBuyPrice/GetSellPrice` |
| `AuctionBotBuyer::Update(houseType)` | For each tick where buyer enabled: `GetItemInformation` → `PrepareListOfEntry` → `BuyAndBidItems`. | `sAuctionMgr` |
| `AuctionBotBuyer::GetItemInformation(config)` | Walk all live auctions in the house, skip commodities (TODO partial buys), skip bot-owned, build `SameItemInfo` (rolling avg) and add to `EligibleItems` if no bid or bid is by a player. | iteration of `AuctionHouseObject::GetAuctionsBegin/End` |
| `AuctionBotBuyer::RollBuyChance(info, auction)` | `chance = min(100, 100 ^ (1 + (1 − price/itemValue)/ChanceFactor))`; if there's already a player bidder, ÷5; weight by `GetChanceMultiplier(quality)`. | `frand`, item sell-price lookup, vendor-fallback |
| `AuctionBotBuyer::RollBidChance(info, auction, bidPrice)` | Same shape as buy but uses `MinBid`-derived target. | same |
| `AuctionBotBuyer::BuyEntry(auction, house)` | Sets winning bidder to a random AHBot char (`GetRandCharExclude(owner)`), sets bid amount, expires auction immediately so the owner gets sold-mail. | `AuctionPosting` mutate, mail send via `AuctionHouseMgr::SendAuctionSold` later |
| `AuctionBotBuyer::PlaceBidToEntry(auction, house, bidPrice)` | Outbids previous bidder (sends outbid mail through `AuctionHouseMgr::SendAuctionOutbid`); sets new bidder to a random AHBot char ≠ owner. | same |

---

## 5. Module dependencies

**Depends on:**
- `AuctionHouseMgr` (`AuctionHouse/AuctionHouseMgr.{h,cpp}`) — every AHBot operation mutates a live `AuctionHouseObject`. Without auction support, AHBot is dead code.
- `ObjectMgr` — `GetItemTemplate`, `GetNpcVendorItemList` for the seller's vendor filter.
- `ItemTemplate` (DBC + `item_template`) — class/quality/bonding/buy-sell-price/item-level/req-level/req-skill.
- `World` — `CONFIG_ALLOW_TWO_SIDE_INTERACTION_AUCTION` warning, ticked from `World::Update`.
- `WorldDatabase` — the `creature_loot_template` UNION for the loot allowlist.
- `CharacterDatabase` — `CHAR_SEL_CHARS_BY_ACCOUNT_ID` to enumerate the AHBot account's characters.
- `Mail` (indirectly, via `AuctionHouseMgr::SendAuction*Mail`).
- `Random` (`urand`/`frand`/`roll_chance_f`).
- `GameTime` — auction `EndTime` is wall-clock.

**Depended on by:**
- `cs_ahbot.cpp` (GM commands) — `SetItemsRatio`, `SetItemsAmount`, `Rebuild`, `ReloadAllConfig`, `PrepareStatusInfos`.
- Indirectly `World::SetInitialWorldSettings` → `sAuctionBot->Initialize()`.

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARS_BY_ACCOUNT_ID` (prepared) | Resolve `_AHBotCharacters` from `CONFIG_AHBOT_ACCOUNT_ID`. | character |
| Inline `SELECT item FROM creature_loot_template WHERE Reference = 0 UNION ... 11 loot tables` | Build the loot-only allowlist for seller. **One large UNION query at startup** — can be expensive on a populated world DB. | world |
| `worldserver.conf` (`AuctionHouseBot.*`) | All ~140 runtime knobs. Not a DB query but the dominant config surface. | n/a |

No DB2/DBC store dependency directly — but indirectly via `ItemTemplate` and the `AuctionHouse.db2` ids hardcoded in `AuctionHouseIds[3] = {1,2,6}`.

---

## 7. Wire-protocol packets (if any)

**None.** AHBot is purely server-internal. Buyer-induced sold/outbid/won mail is sent via the standard `AuctionHouseMgr::SendAuction*Mail` paths and arrives at real players via `MailDraft::SendMailTo` — same as a human bidder would trigger.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/auctionhousebot` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-ahbot` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-config` | `crate_dir` | 1 | 397 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *None.* No `wow-ahbot` crate exists. No `AuctionHouseBot` module exists in `wow-world`. No config-loader keys for `AuctionHouseBot.*` in `wow-config`. No GM command surface.

**What's implemented:**
- Nothing.

**What's missing vs C++:**
- Everything: config layer, seller filtering, item pool, buyer chance math, round-robin tick, status-report glue, `.ahbot` GM commands.
- The prerequisite (`AuctionHouseMgr`, the four `AuctionHouseObject`s, `AuctionPosting`, the deposit/cut math, the mail flow) is also unported — see `auctionhouse.md`. **AHBot blocked behind that.**

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — there is nothing to diverge from. When implementation begins, priorities should be:
  - The `_itemPool` filter is *the* thing to test against TC line-by-line; it's where every bug historically lived.
  - The seller's price math has a subtle 1e6 divisor (because percentages are integer 0..10000 with a /100 then /100 cascade); easy to off-by-100x.
  - Bot-owned-detection (`IsBotChar`) returns `true` for empty GUIDs — preserve that (server-spawned auctions count as bot-owned).
  - On `AllowTwoSide.Interaction.Auction = true`, only Neutral should run; the C++ logs a warning but does not enforce.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#AUCTIONHOUSEBOT.WBS.001** Partir y cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBot.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 559 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.002** Cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBot.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.h`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.003** Cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.004** Cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBotBuyer.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.h`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.005** Partir y cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBotSeller.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 924 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.006** Cerrar la migracion auditada de `game/AuctionHouseBot/AuctionHouseBotSeller.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.h`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#AUCTIONHOUSEBOT.WBS.007** Cerrar la migracion auditada de `game/AuctionHouseBot/enuminfo_AuctionHouseBot.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/enuminfo_AuctionHouseBot.cpp`
  Rust target: `crates/wow-ahbot`, `crates/wow-world`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#AHBOT.0** **(BLOCKED)** Wait until `auctionhouse.md` reaches partial parity (at minimum: `AuctionHouseObject`, `AuctionPosting`, expiration tick, deposit math, `SendAuction{Won,Sold,Outbid,Expired}` mail). AHBot cannot be tested in isolation.
- [ ] **#AHBOT.1** Decide crate location: `wow-world::auctionhousebot` submodule vs new `wow-ahbot` crate. New crate preferred so `worldserver` can compile out the bot at zero cost. (L)
- [ ] **#AHBOT.2** Port `AuctionBotConfig`: three flat arrays of u32/bool/f32 indexed by enums; `Initialize()` reading `worldserver.conf` keys; `_AHBotCharacters: Vec<ObjectGuid>` resolved from account id. Use `wow-config` for the conf side. (M)
- [ ] **#AHBOT.3** Port `AuctionBotConfig::SetConfig{,Max,MinMax}` clamping behavior — including the negative-int log-and-reset path. Add unit tests for clamps (M)
- [ ] **#AHBOT.4** Port `AuctionHouseBot` round-robin `Update()` skeleton with the "one successful step per call" semantics and the 0..5 selector. (L)
- [ ] **#AHBOT.5** Port `AuctionBotSeller::Initialize()` item-pool builder. Includes vendor allowlist, loot allowlist (single UNION query), include/exclude string parser, bonding mask, allowZero per class, item-level/req-level/skill windows, per-class mount/glyph/tradegood/container windows. (XL — ~400 lines of branches; split per filter family)
- [ ] **#AHBOT.6** Port `SetStat` (per-house "missing items" math) with the LastMissedItem cursor. (M)
- [ ] **#AHBOT.7** Port `GetItemsToSell` weighted picker honoring class priorities and per-quality budgets, bounded by `ItemsPerCycle.{Boost,Normal}`. (H)
- [ ] **#AHBOT.8** Port `AddNewAuctions` — stack-size roller, duration roller, calling into the (yet-to-be-ported) `AuctionHouseMgr::AddAuction` with bot-owned `Owner`. (H)
- [ ] **#AHBOT.9** Port `SetPricesOfItem` price math; nail the /100 /100 cascade with property tests vs hand-computed examples. (M)
- [ ] **#AHBOT.10** Port `AuctionBotBuyer::GetItemInformation` aggregator (skip commodities, skip bot-owned, rolling avg per itemId). (M)
- [ ] **#AHBOT.11** Port `RollBuyChance` / `RollBidChance` chance-factor math, including the /5 player-bid penalty and `GetChanceMultiplier(quality)`. (M)
- [ ] **#AHBOT.12** Port `BuyEntry` / `PlaceBidToEntry` — invoke the auction layer's outbid/sold mail paths. (M)
- [ ] **#AHBOT.13** Port `PrepareListOfEntry` cleanup of stale `EligibleItems` (TTL via `LastExist`). (L)
- [ ] **#AHBOT.14** GM command surface (`.ahbot status / items / ratio / rebuild / reload`) wired through `wow-script`/chat command framework — **deprioritize until** the AHBot itself works headless. (M)
- [ ] **#AHBOT.15** Document worldserver.conf section equivalents in `WorldServer.conf.example`. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#AUCTIONHOUSEBOT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 7 files / 2630 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`. Rust target: `crates/wow-config`, `crates/wow-world`. | `cargo test -p wow-config && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#AUCTIONHOUSEBOT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 7 files / 2630 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`. Rust target: `crates/wow-config`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#AUCTIONHOUSEBOT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 7 files / 2630 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`. Rust target: `crates/wow-config`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#AUCTIONHOUSEBOT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 7 files / 2630 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp`. Rust target: `crates/wow-config`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests que demuestren que el comportamiento Rust = comportamiento C++ para invariantes clave.

- [ ] Test: `IsBotChar(ObjectGuid::Empty)` returns `true` (so server-spawned items are treated as bot-owned).
- [ ] Test: `IsBotChar(some-real-player-guid)` returns `false`.
- [ ] Test: `GetRandCharExclude(owner)` never returns `owner` even when `_AHBotCharacters` has only one element (returns `Empty` — preserve!).
- [ ] Test: `Update()` with both buyer & seller `nullptr` (disabled) is a no-op and does not advance `_operationSelector`. (Look — current C++ `if (!_buyer && !_seller) return;` is *before* the loop; preserve.)
- [ ] Test: `Update()` advances selector even on unsuccessful step; rolls over at 6.
- [ ] Test: Seller item-pool excludes items with quality `>= MAX_AUCTION_QUALITY` (heirloom+).
- [ ] Test: Seller filters `BIND_NONE`/`PICKUP`/`EQUIP`/`USE`/`QUEST` according to `AuctionHouseBot.Bind.*` flags. Five sub-cases.
- [ ] Test: Pricing math — known item with `BuyPrice=10000c, SellPrice=2500c`, quality ratio 100, class ratio 100, house ratio 100 → buyout matches TC reference. Repeat with non-100 ratios to catch off-by-100 in the cascade.
- [ ] Test: `SetConfigMax` clamps overflow to `maxvalue` and logs an error; default applied for negatives.
- [ ] Test: Buyer `RollBuyChance` with `auction->BuyoutOrUnitPrice == 0` returns `false`.
- [ ] Test: Buyer `RollBuyChance` with player-bidder present multiplies chance by 0.2.
- [ ] Test: `Rebuild(false)` only expires bot auctions with `BidAmount == 0`; `Rebuild(true)` expires all bot auctions; never touches non-bot auctions.
- [ ] Test: AHBot accountid → character resolution: empty list when account has no chars; `_AHBotCharacters.size() == row count` otherwise.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 7 files / 2630 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp` | *none yet* — would live under `crates/wow-world/src/auctionhousebot/` (alongside the future auction crate) or a new `wow-ahbot` crate. Depends on a working `AuctionHouseMgr` (see `auctionhouse.md`), which is also not yet ported. \| ❌ not started (audit confirmed 2026-05-01) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#AUCTIONHOUSEBOT.DIV.001` | `crates/wow-world/src/auctionhousebot` (`missing_declared_path`, 0 Rust lines) | 7 C++ files / 2630 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#AUCTIONHOUSEBOT.DIV.002` | `crates/wow-ahbot` (`missing_declared_path`, 0 Rust lines) | 7 C++ files / 2630 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotSeller.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/AuctionHouseBot/AuctionHouseBotBuyer.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Disabled by default.** TrinityCore ships `AuctionHouseBot.Seller.Enabled = false`, `AuctionHouseBot.Buyer.Enabled = false`. If both are off, `AuctionBotConfig::Initialize()` returns `false` and the bot never instantiates. Most production realms leave it off; treat AHBot as a tier-3 priority for the WoLK Classic relaunch.
- **Auction house id table mismatch.** `AuctionHouseIds[3] = {1,2,6}` is in *enum-order*: `Neutral=0 → id 1`, `Alliance=1 → id 2`, `Horde=2 → id 6`. The DB2 entry numbers are not sequential. Easy to mis-index in Rust.
- **Heirloom + quality skip.** `MAX_AUCTION_QUALITY = 7` skips quality 7 (heirloom). The seller drops them. The buyer's chance multipliers stop at yellow/artifact. Don't try to be clever and add an 8th slot.
- **`_AHBotCharacters` fallback.** When the configured account has 0 chars, `_AHBotCharacters` is empty, `GetRandChar()` returns `ObjectGuid::Empty`, and items get listed as "owner-less". `IsBotChar(empty) == true` re-anchors them as bot-owned. Removing this fallback breaks fresh installs — preserve it.
- **One-success-per-tick is intentional throttling.** Don't be tempted to rewrite `Update()` as a parallel for-loop — the staggering matters when seller fills 1000 items per cycle.
- **Loot-table UNION at startup.** ~11-table UNION can take seconds on a real DB. Run it once at init, cache the result. C++ does the same.
- **`AllowTwoSide.Interaction.Auction` interaction.** When that worldserver flag is `true`, only Neutral house should be populated. The C++ logs a warning but does not enforce. Decide for Rust whether to log-only (compat) or hard-disable Alliance/Horde houses (safer).
- **No localization / no scripts.** Unlike most TC modules there's no `script_name` column and no string localization — everything is config-driven and English-tagged. Keep it that way.
- **Buyer skips commodities.** `if (entry->IsCommodity()) continue;` — partial buys are NYI in TC. We can keep that limitation.
- **WoLK 3.4.3 vs Retail.** This file is from the Wrath Classic branch (which retains the 9.x `AuctionPosting`/`AuctionBucketKey` structures even though most slot fields go unused at level-80 caps). The bot itself doesn't care, but our auction layer must agree on the wire format before AHBot can run.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class AuctionBotConfig` (singleton) | `pub struct AuctionBotConfig` + `OnceLock<AuctionBotConfig>` (or `Arc<RwLock<...>>` if reload is allowed) | Reload requires inner mutability; pick `RwLock`. |
| `_configUint32Values[CONFIG_UINT32_AHBOT_UINT32_COUNT]` | `[u32; UINT32_COUNT]` indexed by `repr(usize)` enum | Avoid `HashMap<&str,u32>` — flat array is the perf win the C++ code goes out of its way to keep. |
| `enum AuctionBotConfigUInt32Values` | `#[repr(usize)] pub enum CfgU32 { … }` with `as usize` indexing | Keep ordinal stability; don't auto-derive. |
| `class AuctionBotAgent` (abstract) | `pub trait AuctionBotAgent { fn initialize(&mut self) -> bool; fn update(&mut self, house: AuctionHouseType) -> bool; }` | Trait, not enum — there are exactly two impls. |
| `class AuctionHouseBot` | `pub struct AuctionHouseBot { seller: Option<Box<AuctionBotSeller>>, buyer: Option<Box<AuctionBotBuyer>>, op_selector: u32 }` | Singletons through `OnceLock` at the worldserver layer. |
| `_itemPool[MAX_AUCTION_QUALITY][MAX_ITEM_CLASS]: vector<uint32>` | `[[Vec<u32>; MAX_ITEM_CLASS as usize]; MAX_AUCTION_QUALITY as usize]` | ~7×16 = 112 vecs; fine. Build once in `initialize`. |
| `SellerConfiguration._itemInfo[q][c]` | `[[SellerItemInfo; MAX_ITEM_CLASS]; MAX_AUCTION_QUALITY]` | Same. |
| `BuyerItemInfoMap (map<u32, BuyerItemInfo>)` | `HashMap<u32, BuyerItemInfo>` | Per-tick rebuild — no need for `DashMap`. |
| `CheckEntryMap (map<u32, BuyerAuctionEval>)` | `HashMap<u32, BuyerAuctionEval>` | Aged via `LastExist` TTL. |
| `Trinity::Containers::SelectRandomContainerElement(vec)` | `vec.choose(&mut rng)` from `rand` crate | Use `rand::SeedableRng` so tests are deterministic. |
| `urand(min, max)` | `rng.gen_range(min..=max)` | Inclusive bounds match TC. |
| `frand(a, b)` | `rng.gen_range(a..=b)` over `f32` | Same. |
| `roll_chance_f(p)` | `rng.gen::<f32>() * 100.0 < p` | Pre-existing helper in `wow-core::random` (likely). |
| `sAuctionBotConfig` (macro) | `wow_ahbot::config()` returning `&'static AuctionBotConfig` | After `OnceLock` init. |
| `AuctionHouseBot::Update()` from `World::Update(diff)` | `world_tick` in `wow-world` calls `wow_ahbot::tick()` if feature enabled | Cargo feature `ahbot` to compile out cleanly. |
| `CHAR_SEL_CHARS_BY_ACCOUNT_ID` | Existing prepared statement in `wow-database::statements::character` | Reuse — already needed by character listing. |
| `WorldDatabase.PQuery(...UNION...)` | `wow-database::statements::world::SEL_AHBOT_LOOT_ITEMS_UNION` (new) | Single registered prepared statement. |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines AHBot Rust impl | ✅ | `grep -rn "AuctionHouseBot\|AhBot\|auctionhousebot" crates/ → 0` |
| No `wow-ahbot` crate | ✅ | `ls crates/` → no entry; only `wow-achievement` & `wow-auction` (also absent) |
| No AHBot opcodes/SQL | ✅ | grep `AHBOT\|ahbot` across `wow-constants`, `wow-database` → 0 |
| Depends on AuctionHouse (also ❌) | ✅ | `auctionhouse.md` audit confirms only hello-stub exists |

**Net status:** absent. As doc notes, AHBot is post-WoLK opt-in fixture; deferring is correct. No silent-stub hazard because no opcodes are wired.

---

*Template version: 1.0 (2026-05-01).* When implementation starts, flip `Status` to `⚠️ partial` and update `Last updated`.
