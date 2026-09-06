# Migration: Mails

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Mails/` + `src/server/game/Handlers/MailHandler.cpp`
> **Rust target crate(s):** `crates/wow-world/` (handlers, session state), `crates/wow-database/` (mail prepared statements), `crates/wow-packet/` (packet types)
> **Layer:** L6
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete
> **Last updated:** 2026-05-01

---

## 1. Purpose

Persistent in-game messaging between characters and NPC senders (auctions, calendar, GM, blackmarket, quest reward mail). Carries text, money, item attachments and Cash-On-Delivery (COD). Mail expires (3 days w/ COD, 30 days normal, 90 days for GM senders, 1 hour for empty auction notices), supports return-to-sender, and applies a 1-hour cross-account delivery delay when items move between accounts. Items in mail are detached from any inventory and stored as standalone `item_instance` rows owned by the receiver GUID.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Mails/Mail.cpp` | 291 | `prefix` |
| `game/Mails/Mail.h` | 217 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Mails/Mail.h` | 217 | `Mail` struct, `MailDraft`, `MailSender`, `MailReceiver`, enums (`MailMessageType`, `MailCheckMask`, `MailStationery`, `MailState`, `MailShowFlags`) |
| `src/server/game/Mails/Mail.cpp` | 291 | `MailDraft::SendMailTo`, `SendReturnToSender`, `prepareItems` (template loot mail), `deleteIncludedItems` |
| `src/server/game/Handlers/MailHandler.cpp` | 676 | All CMSG opcode handlers: `HandleSendMail`, `HandleGetMailList`, `HandleMailMarkAsRead`, `HandleMailDelete`, `HandleMailReturnToSender`, `HandleMailTakeItem`, `HandleMailTakeMoney`, `HandleMailCreateTextItem`, `HandleQueryNextMailTime`, `HandleItemTextQuery`, `CanOpenMailBox` |
| `src/server/game/Server/Packets/MailPackets.h` / `.cpp` | ~600 | Wire-format readers/writers for SMSG_MAIL_LIST_RESULT, SMSG_SEND_MAIL_RESULT, etc. |
| `src/server/game/Entities/Player/Player.cpp` (PlayerStorage section) | ~par. | `Player::AddMail`, `RemoveMail`, `GetMail`, `_LoadMail`, `_SaveMail`, `m_mail`, `m_mailItems`, `unReadMails`, `m_mailsUpdated`, `AddNewMailDeliverTime` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Mail` | struct | Persisted/in-memory mail row: `messageID`, `messageType`, `stationery`, `mailTemplateId`, `sender`, `receiver`, `subject`, `body`, `items` (vec of `MailItemInfo`), `removedItems`, `expire_time`, `deliver_time`, `money`, `COD`, `checked`, `state` |
| `MailItemInfo` | struct | `{ ObjectGuid::LowType item_guid; uint32 item_template; }` |
| `MailDraft` | class | Builder used at send time. Holds `m_items` map, `m_money`, `m_COD`, template id, subject, body. Methods: `AddItem`, `AddMoney`, `AddCOD`, `SendMailTo`, `SendReturnToSender`, `prepareItems`, `deleteIncludedItems` |
| `MailSender` | class | Encapsulates `(MailMessageType, ObjectGuid::LowType senderId, MailStationery)`. Constructed from `Object*`, `Player*`, `CalendarEvent*`, `AuctionHouseObject*`, `BlackMarketEntry*`, raw entry id |
| `MailReceiver` | class | `(Player* receiver, ObjectGuid::LowType receiver_lowguid)`. ASSERTs receiver guid matches. |
| `MailMessageType` | enum | `MAIL_NORMAL=0, MAIL_AUCTION=2, MAIL_CREATURE=3, MAIL_GAMEOBJECT=4, MAIL_CALENDAR=5, MAIL_BLACKMARKET=6, MAIL_COMMERCE_AUCTION=7, MAIL_AUCTION_2=8, MAIL_ARTISANS_CONSORTIUM=9` |
| `MailCheckMask` | enum (bitmask) | `NONE=0, READ=0x01, RETURNED=0x02, COPIED=0x04, COD_PAYMENT=0x08, HAS_BODY=0x10` |
| `MailStationery` | enum | `TEST=1, DEFAULT=41, GM=61, AUCTION=62, VAL=64, CHR=65, ORP=67` (from `Stationery.dbc`) |
| `MailState` | enum | `UNCHANGED=1, CHANGED=2, DELETED=3` (in-memory dirty tracking) |
| `MailShowFlags` | enum | UI hints: `DELETE`, `AUCTION`, `RETURN` |
| `MailResponseType` | enum | `MAIL_SEND, MAIL_MONEY_TAKEN, MAIL_ITEM_TAKEN, MAIL_RETURNED_TO_SENDER, MAIL_DELETED, MAIL_MADE_PERMANENT` |
| `MailResponseResult` | enum | `MAIL_OK=0, MAIL_ERR_*` (recipient-not-found, not-your-team, internal-error, cap-reached, too-many-attachments, …) |

Constants: `MAX_CLIENT_MAIL_ITEMS = 12`, `MAX_MAIL_ITEMS = 16`, `MAIL_BODY_ITEM_TEMPLATE = 8383` (Plain Letter item used by `HandleMailCreateTextItem`).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `MailDraft::SendMailTo(trans, receiver, sender, checked, deliver_delay)` | Final step of any mail send. Generates `mailId`, computes `expire_delay` (3d COD / 30d / 90d GM / 1h auction-empty), inserts `mail` + `mail_items` rows, pushes live `Mail*` to receiver's `m_mail` if connected | `ObjectMgr::GenerateMailID`, `Item::SaveToDB`, `Player::AddMail`, `Player::AddMItem`, `Player::AddNewMailDeliverTime` |
| `MailDraft::SendReturnToSender(sender_acc, sender_guid, receiver_guid, trans)` | Reverse a mail back to original sender. Re-owns items via `CHAR_UPD_ITEM_OWNER` and re-sends with `MAIL_CHECK_MASK_RETURNED` | `CharacterCache::GetCharacterAccountIdByGuid`, `Item::SaveToDB`, `SendMailTo` |
| `MailDraft::prepareItems(receiver, trans)` | Generates items from `mailTemplateId` via `LootTemplates_Mail` (used by quest reward mails, e.g. template 123 grants 100g) | `Loot::FillLoot`, `Item::CreateItem`, `Item::SaveToDB` |
| `WorldSession::HandleSendMail` | Validates: mailbox open, level ≥ `CONFIG_MAIL_LEVEL_REQ`, recipient exists, not self-send, attachments ≤12, money/cod non-negative, recipient has <100 mails, faction match (unless account-bound or GM), recipient level ≥ req. Charges 30c per attachment (or 30c minimum). Delay=1h for cross-account, 0 for guildmates | `CharacterCache`, `Battlenet::AccountMgr::GetIdByGameAccountAsync`, `Item::CanBeTraded`, `IsBoundAccountWide`, `MailDraft::SendMailTo` |
| `WorldSession::HandleGetMailList` | Returns `SMSG_MAIL_LIST_RESULT` with all delivered mails for the player (`deliver_time <= now`) | `Player::GetMail`, item lookup via `m_mailItems` |
| `WorldSession::HandleMailMarkAsRead` | Sets `MAIL_CHECK_MASK_READ` flag, decrements `unReadMails`, marks `MAIL_STATE_CHANGED` | `Player::GetMail` |
| `WorldSession::HandleMailDelete` | Marks mail `MAIL_STATE_DELETED` if no COD attached. COD-mails forbid delete | `Player::GetMail` |
| `WorldSession::HandleMailReturnToSender` | Validates mail not already deleted, deliver_time passed, sender id matches packet. Deletes original rows, inserts new draft via `SendReturnToSender` | `MailDraft::SendReturnToSender` |
| `WorldSession::HandleMailTakeItem` | Moves an attached item into player's inventory. If COD set: charges COD, sends "money received" mail back to sender, sets `MAIL_CHECK_MASK_COD_PAYMENT` | `Player::CanStoreItem`, `Player::StoreItem`, `Player::ModifyMoney`, `MailDraft::SendMailTo` (COD return) |
| `WorldSession::HandleMailTakeMoney` | Adds mail money to player gold (capped at `MAX_MONEY_AMOUNT`), zeroes `m->money`, marks `CHANGED` | `Player::ModifyMoney` |
| `WorldSession::HandleMailCreateTextItem` | Creates a "plain letter" (item entry 8383) carrying the mail body, persists it as a standalone `item_instance` | `Item::CreateItem`, `Player::CanStoreNewItem`, `Player::StoreNewItem` |
| `WorldSession::HandleQueryNextMailTime` | Returns `SMSG_MAIL_QUERY_NEXT_TIME_RESULT` with next undelivered mail time, plus list of pending unread mails | scans `m_mail` |
| `WorldSession::HandleItemTextQuery` | Returns full body text of a previously-read mail letter item | `Player::GetMail` |
| `WorldSession::CanOpenMailBox(guid)` | Validates mailbox source: GameObject `MAILBOX` type, NPC with `UNIT_NPC_FLAG_MAILBOX`, or self (GM perm) | `Player::GetGameObjectIfCanInteractWith`, `GetNPCIfCanInteractWith`, RBAC |
| `Player::AddMail / RemoveMail / GetMail` | In-memory mail container modifiers | — |
| `Player::_LoadMail / _SaveMail` | Serialize `m_mail` to/from `mail` + `mail_items` tables. `_LoadMailedItems` rebuilds `m_mailItems` map | DB |
| `Player::AddNewMailDeliverTime(t)` | Tracks earliest pending deliver to schedule notification | `m_nextMailDelivereTime` |
| `Player::SendMailResult(mailId, action, result, equipErr=0, itemGuid=0, itemCount=0)` | Wraps `SMSG_SEND_MAIL_RESULT` | session |
| `Player::SendNewMail()` | Sends `SMSG_RECEIVED_MAIL` notification when a new delivery becomes available | session |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Player` — `Player::m_mail`, `m_mailItems`, mail-related send helpers, `unReadMails`, `m_nextMailDelivereTime`.
- `Entities/Item` — `Item::SaveToDB`, `DeleteFromDB`, `CreateItem`, `CanBeTraded`, `IsBoundAccountWide`, `IsWrapped`, `SetOwnerGUID`, `SetNotRefundable`.
- `Globals/ObjectMgr` — `GenerateMailID()`, `GetTrinityString`, `GetRepSpilloverTemplate` (no, that's reputation).
- `Cache/CharacterCache` — receiver name → guid → account id resolution for offline targets.
- `Loot/LootMgr` — `LootTemplates_Mail` for `prepareItems` (mail templates from `mail_loot_template`).
- `Calendar/CalendarMgr` — `CalendarEvent` sender constructor.
- `AuctionHouse/AuctionHouseMgr` — `AuctionHouseObject` sender; auction-end notifications.
- `BlackMarket/BlackMarketMgr` — `BlackMarketEntry` sender.
- `Guilds/GuildMgr` — instant delivery between guildmates.
- `World/World` — `CONFIG_MAIL_DELIVERY_DELAY`, `CONFIG_MAIL_LEVEL_REQ`.
- `GameTime` — `GetGameTime()` for `deliver_time`/`expire_time`.
- `Battlenet::AccountMgr` — battle.net account match for cross-account bound items.
- `RBAC` — `RBAC_PERM_COMMAND_MAILBOX`, `RBAC_PERM_TWO_SIDE_INTERACTION_MAIL`, `RBAC_PERM_LOG_GM_TRADE`.
- `Database/CharacterDatabase` — prepared statements (see §6).

**Depended on by:**
- `AuctionHouse` — auction expiration, sale, bid-loss send mail to participants.
- `Calendar` — invite RSVPs sent via mail.
- `BlackMarket` — winning-bid item delivery.
- `Quests` — quest reward mail (template-driven via `prepareItems`).
- `Guild` — `Guild::HandleMemberDepositMoney` may forward via mail in some paths.
- `Battlegrounds` — reward delivery via mail when player offline.
- `Scripts (boss/npc)` — quest hand-ins generating mail.

---

## 6. SQL / DB queries (if any)

DB: `character`. Constants from `CharacterDatabase.cpp`.

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_MAIL` | Load all mail rows for a player on login: `SELECT id, messageType, sender, receiver, subject, body, expire_time, deliver_time, money, cod, checked, stationery, mailTemplateId FROM mail WHERE receiver = ? ORDER BY id DESC` | character |
| `CHAR_SEL_MAILITEMS` | Load all mail items via `mail_items` join `item_instance` for a receiver | character |
| `CHAR_SEL_MAIL_LIST_COUNT` | `SELECT COUNT(id) FROM mail WHERE receiver = ?` | character |
| `CHAR_SEL_MAIL_LIST_INFO` | Mail list with sender/receiver names | character |
| `CHAR_SEL_MAIL_LIST_ITEMS` | `SELECT itemEntry, count FROM item_instance WHERE guid = ?` (for list display) | character |
| `CHAR_SEL_MAIL_COUNT` | Mail count for a receiver (used during `HandleSendMail` to enforce 100-cap) | character |
| `CHAR_INS_MAIL` | Insert new mail row | character |
| `CHAR_DEL_MAIL_BY_ID` | Delete one mail row | character |
| `CHAR_DEL_MAIL` | Delete all mail for player (char delete) | character |
| `CHAR_INS_MAIL_ITEM` | Link an item to a mail | character |
| `CHAR_DEL_MAIL_ITEM` | Detach item from a mail (when taken) | character |
| `CHAR_DEL_MAIL_ITEM_BY_ID` | Delete all items linked to a mail | character |
| `CHAR_DEL_MAIL_ITEMS` | Cleanup all mail items for a deleted character | character |
| `CHAR_UPD_MAIL` | `UPDATE mail SET has_items=?, expire_time=?, deliver_time=?, money=?, cod=?, checked=? WHERE id=?` | character |
| `CHAR_UPD_MAIL_RETURNED` | Returned mail flips sender/receiver, resets cod=0 | character |
| `CHAR_UPD_MAIL_ITEM_RECEIVER` | Re-owner mail items when returning | character |
| `CHAR_UPD_ITEM_OWNER` | Update `item_instance.owner_guid` when item changes hands | character |
| `CHAR_SEL_MAIL_COUNT_ITEM` | Diagnostic count of items by entry across mail | character |
| `CHAR_SEL_MAIL_ITEMS_BY_ENTRY` | GM lookup tool | character |
| `CHAR_INS_ITEM_INSTANCE` | (item module) — used when SaveToDB called for mail-stored items | character |
| `CHAR_DEL_ITEM_INSTANCE` | Delete item instance when mail returns to non-existent sender | character |

DBC/DB2 stores read by mail subsystem:

| Store | What it loads | Read by |
|---|---|---|
| `Stationery.dbc` | Mail stationery records (visual envelope) | client only — values reflected in `MailStationery` enum |
| `MailTemplate.db2` | Mail template id → subject/body localized strings (used by `prepareItems` plus auction/calendar pre-baked text) | `MailDraft::prepareItems` (via `LootTemplates_Mail` for items, template strings via DB2 reader at packet build time) |

Worldserver tables referenced by `LootTemplates_Mail`:

| Table | Purpose |
|---|---|
| `mail_loot_template` | Items granted by a given `mailTemplateId` (loot semantics with chance/reference/groupId) |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_SEND_MAIL` | client → server | `WorldSession::HandleSendMail` |
| `CMSG_MAIL_GET_LIST` | client → server | `WorldSession::HandleGetMailList` |
| `CMSG_MAIL_MARK_AS_READ` | client → server | `WorldSession::HandleMailMarkAsRead` |
| `CMSG_MAIL_DELETE` | client → server | `WorldSession::HandleMailDelete` |
| `CMSG_MAIL_RETURN_TO_SENDER` | client → server | `WorldSession::HandleMailReturnToSender` |
| `CMSG_MAIL_TAKE_ITEM` | client → server | `WorldSession::HandleMailTakeItem` |
| `CMSG_MAIL_TAKE_MONEY` | client → server | `WorldSession::HandleMailTakeMoney` |
| `CMSG_MAIL_CREATE_TEXT_ITEM` | client → server | `WorldSession::HandleMailCreateTextItem` |
| `CMSG_QUERY_NEXT_MAIL_TIME` | client → server | `WorldSession::HandleQueryNextMailTime` |
| `CMSG_ITEM_TEXT_QUERY` | client → server | `WorldSession::HandleItemTextQuery` |
| `SMSG_MAIL_LIST_RESULT` | server → client | response to `MAIL_GET_LIST` |
| `SMSG_SEND_MAIL_RESULT` | server → client | `Player::SendMailResult` (action + result codes) |
| `SMSG_MAIL_QUERY_NEXT_TIME_RESULT` | server → client | response to `QUERY_NEXT_MAIL_TIME` |
| `SMSG_RECEIVED_MAIL` | server → client | `Player::SendNewMail` (notification when delivery time passes) |
| `SMSG_NOTIFY_RECEIVED_MAIL` (alias) | server → client | same path on some builds |
| `SMSG_QUERY_ITEM_TEXT_RESPONSE` | server → client | response to `ITEM_TEXT_QUERY` |

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
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *(none specific to mails)*
- `crates/wow-world/src/handlers/misc.rs` — only stub: `handle_query_next_mail_time` returns "no mail" hard-coded `MailQueryNextTimeResult::no_mail()`.

**What's implemented:**
- Stub for `CMSG_QUERY_NEXT_MAIL_TIME` returning `-1.0` (no mail forever).

**What's missing vs C++:**
- All 9 other CMSG handlers (`SEND_MAIL`, `GET_LIST`, `MARK_AS_READ`, `DELETE`, `RETURN_TO_SENDER`, `TAKE_ITEM`, `TAKE_MONEY`, `CREATE_TEXT_ITEM`, `ITEM_TEXT_QUERY`).
- `Mail`, `MailDraft`, `MailSender`, `MailReceiver` types.
- Persistence: load/save against `mail` and `mail_items` tables — Rust DB layer only has player+inventory statements.
- COD payment loop (charges receiver, mails money back to sender).
- Cross-account 1h delivery delay; guild instant-delivery shortcut.
- Mail-template loot generation for quest reward mail (`MailTemplate.db2` + `mail_loot_template`).
- Plain-letter item creation (entry 8383) for `CREATE_TEXT_ITEM`.
- Mailbox interaction validation (`CanOpenMailBox` over GO/NPC/self).
- Unread mail notification (`SMSG_RECEIVED_MAIL`) and `m_nextMailDelivereTime` scheduling.
- Account / battle.net-account bound item rules on attachment.
- Faction-team mismatch and level-req enforcement on send.
- 30c-per-attachment postage cost.
- 100-mail receiver cap.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The "no mail" stub returns silently for `CMSG_QUERY_NEXT_MAIL_TIME` but there is no handler registration for the other mail opcodes — clients will see opcode-not-handled spam and the mailbox UI will remain frozen.
- No `MAIL_BODY_ITEM_TEMPLATE` constant defined; if mail is later wired up, picking entry 8383 in TrinityCore vs what the WotLK 3.4.3 client expects must be re-verified (the C++ codebase here is multi-version; 3.4.3 client may use a different template id).

**Tests existing:**
- 0 tests in `crates/wow-world` related to mail.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MAILS.WBS.001** Cerrar la migracion auditada de `game/Mails/Mail.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MAILS.WBS.002** Cerrar la migracion auditada de `game/Mails/Mail.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h`
  Rust target: `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.
Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#MAILS.1** Define `Mail`, `MailItemInfo`, `MailState`, `MailMessageType`, `MailCheckMask`, `MailStationery` in `crates/wow-data/src/mail.rs` (L)
- [ ] **#MAILS.2** Define `MailDraft`, `MailSender`, `MailReceiver` builder types in `crates/wow-world/src/mail/` (M)
- [ ] **#MAILS.3** Add prepared statements to `crates/wow-database/src/statements/character.rs` for `INS_MAIL`, `DEL_MAIL_BY_ID`, `INS_MAIL_ITEM`, `DEL_MAIL_ITEM`, `UPD_MAIL`, `UPD_MAIL_RETURNED`, `UPD_MAIL_ITEM_RECEIVER`, `SEL_MAIL`, `SEL_MAILITEMS`, `SEL_MAIL_COUNT`, `UPD_ITEM_OWNER` (M)
- [ ] **#MAILS.4** Implement `Player::load_mail` and `Player::save_mail` (or session-equivalent) — populate `m_mail` Vec<Mail>, `m_mailItems` HashMap<u64, Item>, `unread_mails` counter, `next_mail_delivere_time` (M)
- [ ] **#MAILS.5** Wire packet types `MailPackets.cpp` → `crates/wow-packet/src/packets/mail.rs` (CMSG/SMSG with attachments arrays, COD, stationery, etc.) (H)
- [ ] **#MAILS.6** Implement `MailDraft::send_mail_to(trans, receiver, sender, checked, deliver_delay)` matching C++ expire-delay matrix (3d COD, 30d normal, 90d GM, 1h auction-empty) and inserts `mail` + `mail_items` rows transactionally (H)
- [ ] **#MAILS.7** Implement `MailDraft::send_return_to_sender` including `UPD_ITEM_OWNER` for each attachment and 1-hour cross-account delivery delay (M)
- [ ] **#MAILS.8** Handler `handle_send_mail`: full validation chain — mailbox open, level ≥ `CONFIG_MAIL_LEVEL_REQ`, recipient exists (online + offline async via CharacterCache), not self-send, ≤12 attachments, money/cod ≥ 0, recipient cap 100, faction-team match (unless account-bound or GM), recipient level req, items not wrapped + COD, conjured/expiring items forbidden, account-bound respected; charge 30c × attachments postage; guild instant delivery (XL — split if needed)
- [ ] **#MAILS.9** Handler `handle_get_mail_list` building `SMSG_MAIL_LIST_RESULT` with all delivered mails + their items (M)
- [ ] **#MAILS.10** Handler `handle_mail_mark_as_read` setting `MAIL_CHECK_MASK_READ` and decrementing unread counter (L)
- [ ] **#MAILS.11** Handler `handle_mail_delete` with COD-mail rejection (L)
- [ ] **#MAILS.12** Handler `handle_mail_return_to_sender` (validation + delegate to draft) (M)
- [ ] **#MAILS.13** Handler `handle_mail_take_item` including COD payment loop (charge receiver, send "money received" mail back to sender) (H)
- [ ] **#MAILS.14** Handler `handle_mail_take_money` (cap at MAX_MONEY) (L)
- [ ] **#MAILS.15** Handler `handle_mail_create_text_item` — instantiate item entry 8383 carrying the body, link to mail, persist `item_instance` (M)
- [ ] **#MAILS.16** Handler `handle_query_next_mail_time` — replace stub with real next-deliver scan (L)
- [ ] **#MAILS.17** Handler `handle_item_text_query` returning `SMSG_QUERY_ITEM_TEXT_RESPONSE` (L)
- [ ] **#MAILS.18** Implement `can_open_mail_box(guid)` (GO/NPC/self+RBAC) shared by all mailbox-gated handlers (M)
- [ ] **#MAILS.19** Schedule `SMSG_RECEIVED_MAIL` notification when `deliver_time` passes during session tick / login (M)
- [ ] **#MAILS.20** Mail template loot integration: `mail_loot_template` reader + `MailDraft::prepare_items` (depends on Loot module migration) (H — defer until Loot done)
- [ ] **#MAILS.21** Cross-account delay: hook into Battle.net account resolution so cross-account item mail respects `CONFIG_MAIL_DELIVERY_DELAY` (M)
- [ ] **#MAILS.22** Periodic expire-sweep: when mail past `expire_time` with attachments → return-to-sender, without → delete (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MAILS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 508 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-database && cargo test -p wow-packet && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MAILS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 508 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MAILS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 508 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MAILS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 508 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h`. Rust target: `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: send mail no items, no money, valid recipient → row inserted, `expire_time = deliver + 30d`, `deliver_time = now`.
- [ ] Test: send mail with attachment cross-account → `deliver_time = now + 3600`.
- [ ] Test: send mail with attachment to guildmate → `deliver_time = now` regardless of account.
- [ ] Test: send mail with COD to same-account → `expire_time = deliver + 3d`.
- [ ] Test: send 13 attachments → rejected with `MAIL_ERR_TOO_MANY_ATTACHMENTS`.
- [ ] Test: send mail to self → rejected with `MAIL_ERR_CANNOT_SEND_TO_SELF`.
- [ ] Test: send mail with negative money/cod → rejected, log cheat.
- [ ] Test: receiver has 100 mails → `MAIL_ERR_RECIPIENT_CAP_REACHED`.
- [ ] Test: send conjured item → `MAIL_ERR_EQUIP_ERROR`+`EQUIP_ERR_MAIL_BOUND_ITEM`.
- [ ] Test: COD mail with wrapped item → `MAIL_ERR_CANT_SEND_WRAPPED_COD`.
- [ ] Test: take item from COD mail charges player and mails money to sender; check `MAIL_CHECK_MASK_COD_PAYMENT` set and original mail `cod=0` after.
- [ ] Test: return-to-sender flips sender/receiver, re-owns items, sets `MAIL_CHECK_MASK_RETURNED`.
- [ ] Test: mail-delete on COD mail → `MAIL_ERR_INTERNAL_ERROR` (not allowed).
- [ ] Test: expire sweep: mail with items past `expire_time` → returned to sender; mail without items past `expire_time` → deleted.
- [ ] Test: `MAIL_BODY_ITEM_TEMPLATE` (8383) creation persists item_instance and binds it to mail.
- [ ] Test: GM-sender mail expire = 90d.
- [ ] Test: empty auction-pending mail expire = `CONFIG_MAIL_DELIVERY_DELAY` (3600s default).
- [ ] Test: account-bound item mailable to alt on same account; bnet-account-bound to alt on same bnet.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 508 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h` | `crates/wow-world/` (handlers, session state), `crates/wow-database/` (mail prepared statements), `crates/wow-packet/` (packet types) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MAILS.DIV.001` | _none generated_ | 2 C++ files / 508 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Mails/Mail.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **WotLK 3.4.3 vs modern client**: TrinityCore wotlk_classic still includes references to `ITEM_FLAG_IS_BOUND_TO_ACCOUNT`, `IsBattlenetAccountBound`, paragon, renown, battle pet mail types — many of these are stubs/no-ops on the 3.4.3 client. `MAIL_COMMERCE_AUCTION`, `MAIL_BLACKMARKET`, `MAIL_ARTISANS_CONSORTIUM` are unused in 3.4.3 gameplay. Migrate the enum values for parity but only wire the active paths.
- **Hard-coded postage 30c** (`HandleSendMail`): "price hardcoded in client" comment — the client will not accept any other amount, so do not parameterize.
- **Hard-coded mail template 123** (`MailDraft::prepareItems`): TBC-era quest "The Good News and The Bad News" hardcodes `m_money = 1000000` (100g). Preserve this exact behavior when porting.
- **Item.SaveToDB before AddItem** order: in `HandleSendMail`, items are first removed from inventory (`MoveItemFromInventory`), then `DeleteFromInventoryDB`, then `SetOwnerGUID(receiverGuid)`, then `SaveToDB`. The transaction must commit atomically — partial failure leaves orphan item rows.
- **`MAX_CLIENT_MAIL_ITEMS = 12` vs `MAX_MAIL_ITEMS = 16`**: the client only sends up to 12, but the server can populate up to 16 (used by mail templates that auto-add bonus items).
- **`Mail::sender` is a `LowType` (uint64 part of guid)** — for `MAIL_NORMAL` it's the player's low-guid; for `MAIL_CREATURE`/`MAIL_GAMEOBJECT` it's the entry id; for auction it's `auctionHouseId`. Type-confused storage that the comment in C++ flags as TODO.
- **`m_mail` is a `std::deque` not vector** in `Player.cpp` — push_front semantics for newest-first display in client. Match in Rust with `VecDeque`.
- **Async DB callback chain in HandleSendMail**: when receiver offline, the C++ uses a chained `WithChainingPreparedCallback` to first fetch mail count, then character cache lookup, then bnet account async lookup, then continuation. Rust port must reproduce this with futures or task-spawning to avoid blocking the network thread.
- **`UNIT_NPC_FLAG_MAILBOX` vs `GAMEOBJECT_TYPE_MAILBOX`**: both are valid mailbox sources; do not block one over the other.
- **`CONFIG_MAIL_DELIVERY_DELAY`** default is 3600s (1 hour). Used both for cross-account delivery delay AND for the special "auction empty notice" expire delay — same config knob, two semantics.
- **COD payment back-mail uses `MailSender(MAIL_NORMAL, sender_guid)`** — a synthetic player-typed sender. The original sender will see this mail in their mailbox stamped with their own name.
- **`unReadMails` is uint16** in C++ — overflow only at 65535 mails which cannot happen due to 100-cap, but propagate the type carefully.
- **`MAIL_STATE_DELETED`** is purely in-memory; rows are deleted at next `SaveToDB` flush. If server crashes between flag-and-flush, the mail re-appears at next login — TC accepts this as best-effort.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `struct Mail` | `struct Mail` in `crates/wow-data/src/mail.rs` | All POD; `state` is a transient flag, do not persist |
| `class MailDraft` | `struct MailDraft` (builder) in `crates/wow-world/src/mail/draft.rs` | Use `Result<(), MailError>` instead of silent void return |
| `class MailSender` | `enum MailSender { Player(LowGuid), Creature(u32), GameObject(u32), Auction(u32), Calendar(u32), Blackmarket(u32), Raw(MailMessageType, u64, MailStationery) }` | Replace 6 ctors with a single ADT |
| `class MailReceiver` | `struct MailReceiver { player: Option<Arc<PlayerSession>>, guid_low: LowGuid }` | — |
| `MailItemInfo` | `struct MailItemInfo { item_guid: u64, item_template: u32 }` | — |
| `std::deque<Mail*> Player::m_mail` | `VecDeque<Mail>` on session/player state | `Vec<Box<Mail>>` if heap stability needed |
| `std::map<ObjectGuid::LowType, Item*> Player::m_mailItems` | `HashMap<u64, Arc<Item>>` | items shared with inventory module |
| `MailCheckMask` (bitmask) | `bitflags!` `MailCheckMask` | preserve hex values |
| `void SendMailTo(...)` | `async fn send_mail_to(&mut self, tx: &mut Transaction, ...) -> Result<MailId>` | — |
| `prepareItems(receiver, trans)` | `async fn prepare_items(&mut self, tx, receiver: &Player) -> Result<()>` | depends on Loot |
| `Player::SendMailResult(...)` | `session.send_packet(&SendMailResultPacket{ ... })` | — |
| `sObjectMgr->GenerateMailID()` | atomic `u64` counter in `MailIdAllocator` (inside `World` global) seeded from `MAX(id)` of `mail` at startup | match TC's persistence semantics |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — effectively zero mail code.** The doc's "❌ not started" with a single stub is exact. Only one CMSG handler is wired and it returns a permanent "no mail" sentinel.

**Inventory verified:**
- **No `crates/wow-mail/`** crate.
- **No `crates/wow-world/src/handlers/mail.rs`** (handler file list confirmed: battlenet, character, chat, combat, group, inspect, loot, misc, mod, movement, quest, social, spell, trainer).
- **No `crates/wow-data/src/mail.rs`** type module.
- **No `Mail`, `MailDraft`, `MailSender`, `MailReceiver`** types anywhere — full grep yields zero hits in `crates/` outside test/comment text.
- **No mail SQL statements**: `crates/wow-database/src/statements/character.rs` has no `INS_MAIL`, `SEL_MAIL`, `DEL_MAIL`, `INS_MAIL_ITEM`, `UPD_MAIL`, etc. Verified by grep — only `EMAIL`-related rows in `login.rs` (account email column, unrelated).
- **No `mail_loot_template`** reader in any DB module.

**The single stub:**
- `crates/wow-world/src/handlers/misc.rs:573-581` — `handle_query_next_mail_time` registered at line 92-96 with opcode `QueryNextMailTime`. Implementation is one line: `self.send_packet(&MailQueryNextTimeResult::no_mail());` which sends `next_mail_time = -1.0, count = 0`. Hard-coded "no mail forever" — the player will never receive a `SMSG_RECEIVED_MAIL` notification regardless of true state.
- Packet builder at `crates/wow-packet/src/packets/misc.rs:2027-2051` — `MailQueryNextTimeResult { next_mail_time: f32 }` with `no_mail()` constructor. That is the **only mail-related ServerPacket type** in the whole packet crate.

**Confirmed bug from doc §8:**
- Other 9 mail CMSGs (`SEND_MAIL`, `MAIL_GET_LIST`, `MAIL_MARK_AS_READ`, `MAIL_DELETE`, `MAIL_RETURN_TO_SENDER`, `MAIL_TAKE_ITEM`, `MAIL_TAKE_MONEY`, `MAIL_CREATE_TEXT_ITEM`, `ITEM_TEXT_QUERY`) have **no `inventory::submit!`** registration anywhere. `SendMail` is defined as opcode `0x35fb` in `wow-constants/src/opcodes.rs:560` but no handler exists. The mailbox UI will produce silent "unhandled opcode" warnings on every interaction and remain frozen, exactly as the doc predicted.
- Opcode constants `MailListResult = 0x2756`, `MailQueryNextTimeResult = 0x2757` exist in opcodes.rs but no `MailListResult` packet builder exists. The `AuctionListPendingSalesResult` packet at misc.rs:2000 writes a `Mails.Count = 0` field — that is purely a coincidence of the auction-listing wire format and has nothing to do with the mail subsystem.

**Largest missing surfaces (confirmed):**
- All 9 CMSG handlers + 6 SMSG packet types (`MAIL_LIST_RESULT`, `SEND_MAIL_RESULT`, `RECEIVED_MAIL`, `NOTIFY_RECEIVED_MAIL`, `QUERY_ITEM_TEXT_RESPONSE`).
- Persistence: zero rows of mail SQL exist; the `mail` and `mail_items` tables are not touched by Rust at any point.
- Type system: `Mail`, `MailItemInfo`, `MailDraft`, `MailSender`, `MailReceiver`, `MailMessageType`, `MailCheckMask`, `MailStationery`, `MailState` — none exist.
- COD payment loop, return-to-sender flow, expire-sweep, cross-account 1h delay, guild-instant-delivery shortcut, mail-template loot generation, plain-letter (item 8383) creation, mailbox-source validation (`CanOpenMailBox`).
- Cross-module integrations: auction expire-mail, calendar invite-mail, quest reward-mail, BG offline-reward-mail, blackmarket delivery — all blocked on this module.

**Estimate: <1% complete.** A single dummy SMSG response is the entirety of the implementation.
