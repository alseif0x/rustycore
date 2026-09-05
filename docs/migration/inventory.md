# Migration: Inventory / Player Storage

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Entities/Item/`, `src/server/game/Entities/Player/PlayerStorage.cpp` (logical split of `Player.cpp`), `src/server/game/Handlers/ItemHandler.cpp` + `BankHandler.cpp` + `VoidStorageHandler.cpp`, `src/server/game/Server/Packets/ItemPackets*.cpp`/`BankPackets.cpp`/`VoidStoragePackets.cpp`
> **Rust target crate(s):** `crates/wow-data/` (Item.db2, ItemTemplate, ItemStats), `crates/wow-world/` (handlers, session inventory state), `crates/wow-database/` (item_instance + character_inventory + character_void_storage + character_transmog_outfits + character_equipmentsets prepared statements), `crates/wow-packet/` (item/bank/void packets)
> **Layer:** L4 (item entity) + L6 (player storage logic + handlers) — straddles layers; primary index entry is L6.
> **Status:** ⚠️ partial (base `wow-entities` Item/Bag/Player storage and visible modifier helpers exist; legacy flat `wow-world` inventory bridge remains; no canonical ownership/runtime/DB persistence yet)
> **Audited vs C++:** ✅ complete (re-audit 2026-05-01)
> **Last updated:** 2026-05-07

---

## 1. Purpose

The "inventory" subsystem is the player's complete item storage tree:
- **Equipment** (19 slots): head, neck, shoulders, body (shirt), chest, waist, legs, feet, wrists, hands, finger1/2, trinket1/2, back, mainhand, offhand, ranged, tabard.
- **Equipped bags** (4 slots): bag positions 30-33 hold Bag items that present sub-containers.
- **Backpack** (16-24 slots): default 16 in WoLK, "InventoryPackSlots" 35-58 indices.
- **Bank** (28 main slots + 7 bag slots, must be unlocked progressively).
- **Buyback** (12 slots): vendor sell-back queue.
- **KeyRing** (32 slots, 3.4.3 specific).
- **VoidStorage** (160 slots, post-WoLK; client present in 3.4.3 patched, but unused).
- **Equipment Sets** (saved gear loadouts), **Transmog Outfits**.
- Item lifecycle: instantiation, soulbound, refundable, wrapped/giftwrap, durability, repair, enchantments (perm/temp/socket gems), random properties/suffix, transmog.
- Persistence: items live in `item_instance` + child tables; their inventory position lives in `character_inventory` (or `mail_items`, `character_void_storage`, `auctionhouse`, etc.).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Item/Container/Bag.cpp` | 311 | `prefix` |
| `game/Entities/Item/Container/Bag.h` | 89 | `prefix` |
| `game/Entities/Item/Item.cpp` | 2199 | `prefix` |
| `game/Entities/Item/Item.h` | 378 | `prefix` |
| `game/Entities/Item/ItemDefines.h` | 289 | `prefix` |
| `game/Entities/Item/ItemEnchantmentMgr.cpp` | 204 | `prefix` |
| `game/Entities/Item/ItemEnchantmentMgr.h` | 53 | `prefix` |
| `game/Entities/Item/ItemTemplate.cpp` | 222 | `prefix` |
| `game/Entities/Item/ItemTemplate.h` | 873 | `prefix` |
| `game/Entities/Item/enuminfo_ItemDefines.cpp` | 538 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Entities/Item/Item.h` | 378 | `Item` class + `BonusData` cache + enums |
| `src/server/game/Entities/Item/Item.cpp` | 2199 | All item logic: `Create`, `LoadFromDB`, `SaveToDB`, `DeleteFromDB`, durability, enchantments, refund, soulbound rules |
| `src/server/game/Entities/Item/ItemTemplate.h` | 873 | `ItemTemplate` struct (~150 fields), `ItemBondingType`, `ItemFlags/2/3/4`, `InventoryType`, `ItemQualities`, `ItemSpec` |
| `src/server/game/Entities/Item/ItemTemplate.cpp` | 222 | Helpers: `GetItemSpec*`, level scaling, name lookup |
| `src/server/game/Entities/Item/ItemDefines.h` | 289 | `InventoryResult` (118 codes), `BuyResult`, `SellResult`, `EnchantmentSlot` (13 slots), `ItemModifier` (modern; mostly unused in 3.4.3) |
| `src/server/game/Entities/Item/Container/Bag.h` | 90 | `Bag` class extends `Item` — `m_bagslot[MAX_BAG_SIZE=36]` |
| `src/server/game/Entities/Item/Container/Bag.cpp` | ~250 | Bag-specific load/save, free-slot search |
| `src/server/game/Entities/Item/ItemEnchantmentMgr.h/.cpp` | 53+204 | Random property/suffix roll, gem socket bonus calc |
| `src/server/game/Entities/Player/Player.cpp` (PlayerStorage section) | massive | `CanStoreItem`, `CanEquipItem`, `StoreItem`, `EquipItem`, `MoveItemToInventory`, `MoveItemFromInventory`, `DestroyItem`, `SwapItem`, `SplitItem`, `AutoStoreLoot`, `_LoadInventory`, `_SaveInventory`, `BuyItem`, `SellItem`, `RepairItem`, `DurabilityLoss/Repair`, `BuyBackItem`, `WrapItem`, `OpenItem` |
| `src/server/game/Handlers/ItemHandler.cpp` | 1220 | Most item CMSG handlers (auto-equip, swap, split, destroy, sell, buy, wrap, sort, socket gems, repair, etc.) |
| `src/server/game/Handlers/BankHandler.cpp` | 324 | `HandleBankerActivateOpcode`, `AutoBankItem`, `AutoStoreBankItem`, `BuyBankSlotOpcode`, reagent bank ops |
| `src/server/game/Handlers/VoidStorageHandler.cpp` | 249 | `VoidStorageQuery`, `VoidStorageUnlock`, `VoidStorageTransfer`, `VoidSwapItem` |
| `src/server/game/Server/Packets/ItemPackets.cpp` / `.h` | ~1500 | All client item opcodes' packet types, `SMSG_INVENTORY_CHANGE_FAILURE`, `SMSG_ITEM_PUSH_RESULT`, `SMSG_SELL_RESPONSE`, `SMSG_LOAD_EQUIPMENT_SET`, `SMSG_USE_EQUIPMENT_SET_RESULT`, etc. |
| `src/server/game/Server/Packets/ItemPacketsCommon.cpp` / `.h` | ~400 | Shared `ItemInstance` wire struct, `ItemBonuses`, `ItemEnchantData`, `ItemGemData` |
| `src/server/game/Server/Packets/BankPackets.cpp` / `.h` | ~80 | `BankerActivate`, `AutoBankItem`, `AutoStoreBankItem`, `BuyBankSlot` |
| `src/server/game/Server/Packets/VoidStoragePackets.cpp` / `.h` | ~250 | `VoidStorageContents`, `VoidStorageTransferChanges`, `VoidStorageFailed`, `VoidTransferResult` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Item` | class : Object | An item instance. Has `m_itemData` (UF::ItemData update fields: `Owner`, `ContainedIn`, `Creator`, `GiftCreator`, `StackCount`, `Expiration`, `SpellCharges[5]`, `DynamicFlags`, `DynamicFlags2`, `Enchantment[13]`, `PropertySeed`, `RandomPropertiesID`, `Durability`, `MaxDurability`, `CreatePlayedTime`, `Context`, `ArtifactPowers[]`, `Modifiers`, `BonusListIDs[]`, `ItemAppearanceModID`). `_bonusData` cache for fast template+bonus lookup. `m_state` (NEW/CHANGED/UNCHANGED/REMOVED), `m_slot`, `uQueuePos` (in update queue) |
| `Bag` | class : Item | Container — has `m_bagslot[MAX_BAG_SIZE=36]` array of `Item*`, `m_containerData` (UF::ContainerData) with `NumSlots` and `Slots[36]` (guids) |
| `ItemTemplate` | struct | Static template (from `item_template` table): `BasicData` (Item.db2), `ExtendedData` (ItemSparse.db2 / item_template extras), `Bonding`, `Flags/2/3/4/Custom`, `Quality`, `BuyPrice/SellPrice`, `RequiredLevel/Skill/Faction`, `MaxCount`, `Stackable`, `ContainerSlots`, `Stats`, `Damage[2]`, `Spells[5]`, `Sockets[3]`, `Bonuses`, `RandomProperty/Suffix` |
| `ItemBondingType` | enum | `NO_BIND=0`, `BIND_WHEN_PICKED_UP=1`, `BIND_WHEN_EQUIPPED=2`, `BIND_WHEN_USE=3`, `BIND_QUEST=4` |
| `InventoryType` | enum | 1=Head, 2=Neck, 3=Shoulders, 4=Body, 5=Chest, 6=Waist, 7=Legs, 8=Feet, 9=Wrist, 10=Hands, 11=Finger, 12=Trinket, 13=OneHand, 14=Shield, 15=Ranged, 16=Cloak, 17=TwoHand, 18=Bag, 19=Tabard, 20=Robe, 21=MainHand, 22=OffHand, 23=Holdable, 24=Ammo, 25=Thrown, 26=RangedRight, 27=Quiver, 28=Relic |
| `ItemQualities` | enum | `POOR=0` (grey), `NORMAL=1` (white), `UNCOMMON=2` (green), `RARE=3` (blue), `EPIC=4` (purple), `LEGENDARY=5`, `ARTIFACT=6`, `HEIRLOOM=7`. `MAX_ITEM_QUALITY=8` |
| `ItemFlags` (uint32) | enum | `NO_PICKUP`, `CONJURED`, `OPENABLE`, `HEROIC_TOOLTIP`, `DEPRECATED`, `INDESTRUCTIBLE`, `IS_BOUND_TO_ACCOUNT` (heirloom), `WRAPPER`, `IGNORE_DEFAULT_ARENA_RESTRICTIONS`, `READABLE`, `EVENT_REQUIRED`, `HAS_TEXT`, `NO_DISENCHANT`, `REAL_DURATION`, `NO_REAGENT_COST`, `IS_MILLABLE`, `REPORT_TO_GUILD_CHAT`, `NO_PROGRESSIVE_LOOT` |
| `ItemFieldFlags` (dynamic flags on Item, not template) | enum | `SOULBOUND=0x01`, `UNK1=0x02`, `UNLOCKED=0x04`, `WRAPPED=0x08`, `BOP_TRAEDABLE=0x10` (sic), `READABLE=0x20`, `REFUNDABLE=0x80`, `BOP_TRADEABLE=0x100`, … |
| `EquipmentSlots` | enum | 0..18 (19 slots) — see §1 |
| `ProfessionSlots` | enum | 19..29 (11 slots; modern; in 3.4.3 only first profession tool/gear matters and most are unused) |
| `InventorySlots` | enum | `INVENTORY_SLOT_BAG_START=30`, `END=34` (4 equipped bag slots) |
| `ReagentBagSlots` | enum | 34..35 (1 slot; modern, unused 3.4.3) |
| `InventoryPackSlots` | enum | `INVENTORY_SLOT_ITEM_START=35`, `END=59` (24 slots; 3.4.3 default 16) |
| `BankItemSlots` | enum | 59..87 (28 slots) |
| `BankBagSlots` | enum | 87..94 (7 slots) |
| `BuyBackSlots` | enum | 94..106 (12 slots) |
| `KeyRingSlots` | enum | 106..138 (32 slots; 3.4.3 has keyring; in modern client it's removed) |
| `ChildEquipmentSlots` | enum | 138..141 (3 slots; thrown/ammo/relic linked to equipped) |
| `EquipableSpellSlots` | enum | 211..226 (Dragonflight; n/a 3.4.3) |
| `INVENTORY_SLOT_BAG_0` | `#define 255` | Sentinel: "main inventory" container (the player itself, not an actual bag) |
| `MAX_BAG_SIZE` | `#define 36` | Largest possible bag (modern); WoLK max bag is 22-slot |
| `VoidStorageItem` | struct | `ItemId` (separate id space), `ItemEntry`, `CreatorGuid`, `FixedScalingLevel`, `RandomProperties`, `Context` |
| `VOID_STORAGE_MAX_SLOT` | constant | 160 (defined in `SharedDefines.h`) |
| `EquipmentSetInfo::EquipmentSetData` | struct | Saved gear set: `Guid`, `SetID`, `Name`, `IconName`, `IgnoreMask`, `AssignedSpecIndex`, `Pieces[19]` (item guids), `Type` (Equipment/Transmog), per-piece appearance + enchant for Transmog variant |
| `ItemPosCount` | struct | `{ uint16 pos; uint32 count; }` — output of `CanStoreItem` for stack-storage planning |
| `ItemPosCountVec` | typedef | `std::vector<ItemPosCount>` |
| `EnchantmentSlot` | enum | 13 slots: `PERM=0`, `TEMP=1`, `SOCK1=2`, `SOCK2=3`, `SOCK3=4`, `BONUS=5`, `PRISMATIC=6`, `USE=7`, `PROP_0..4=8..12` |
| `ItemRandomProperties` | struct | `RandomPropertiesID`, `RandomPropertiesSeed` |
| `BonusData` | struct (cache) | Computed item stats with bonuses applied (level, ilvl, item bonuses lookup) |
| `InventoryResult` | enum | 118 EQUIP_ERR codes (see ItemDefines.h) |
| `BuyResult` / `SellResult` | enum | Vendor error codes |
| `ItemContext` | enum | Loot/source context (raid difficulty, quest, vendor, …) used for level-scaling |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Item::Create(guid, itemEntry, context, owner)` | Allocate Item with default fields, set OwnerGUID, StackCount=1, Durability=template.MaxDurability, randomize properties via `ItemEnchantmentMgr::GenerateItemRandomPropertyId`, mark NEW | `sObjectMgr->GetItemTemplate`, `ItemEnchantmentMgr` |
| `Item::LoadFromDB(guid, ownerGuid, fields, entry)` | Build from DB row; restore stack, durability, flags, enchantments, gems, transmog, refund | DB |
| `Item::SaveToDB(trans)` | INSERT/UPDATE `item_instance` per `m_state`; cascade gems/transmog/refund/bopTrade | DB |
| `Item::DeleteFromDB(trans)` | Cascade DELETE `item_instance` + `item_instance_gems` + `item_instance_transmog` + `item_refund_instance` + `item_soulbound_trade_data` | DB |
| `Item::CanBeTraded(mail=false, trade=false)` | False if soulbound (unless BOP-tradeable timer + same group), conjured, refundable | flag checks |
| `Item::SetBinding(true)` | Sets `ITEM_FIELD_FLAG_SOULBOUND`. After AutoStore from loot or after equip if BoE | — |
| `Item::SetState(state, owner)` | Marks NEW/CHANGED/UNCHANGED/REMOVED for `_SaveInventory` flush | — |
| `Item::IsRefundable() / IsBOPTradeable() / IsWrapped() / IsLocked() / IsBroken()` | Bool checks via `m_itemData->DynamicFlags` | — |
| `Item::GetCount()` / `SetCount(n)` | Stack count | — |
| `Item::GetSpellCharges(0..5)` / `SetSpellCharges` | Charges per Use spell | — |
| `Item::GetEnchantmentId(slot)` / `SetEnchantment(slot, id, duration, charges, casterGuid)` | Per-slot enchantment | — |
| `Item::GetVisibleEntry/GetVisibleAppearanceModId/GetVisibleItemVisual` | Resolve transmog/illusion modifiers for equipment display | `ItemModifiedAppearance`, `SpellItemEnchantment` |
| `Item::SetNotRefundable(player)` | Strip refund flag, delete `item_refund_instance` row | DB |
| `Item::AddBonuses(...)` | Push BonusListIDs, recompute `_bonusData` | — |
| `Bag::IsEmpty()` / `GetFreeSlots()` / `GetItemByPos(slot)` | Container access | — |
| `Bag::GetBagSize()` | from `m_containerData->NumSlots` | — |
| `Player::CanStoreItem(bag, slot, dest, count, item, swap, no_space_count_out)` | Compute target position(s) for storing N items; respects stacking, max-count, unique, bag type | Item::CanGoIntoBag, ItemTemplate::IsValidStackSizeForItem |
| `Player::CanStoreItem_InSpecificSlot(...)` / `_InBag(...)` / `_InInventorySlots(...)` | Sub-strategies | — |
| `Player::CanStoreNewItem(...)` | Variant for items that don't yet exist | — |
| `Player::StoreItem(dest, pItem, update)` | Place item into computed positions (may stack), optionally fire SMSG_ITEM_PUSH_RESULT | `RemoveItem`, `SetSlot` |
| `Player::StoreNewItem(dest, itemId, update, itemRandomPropertyId, allowedLooters, context, bonusListIDs, ...)` | Creates Item then StoreItem | `Item::Create`, `StoreItem` |
| `Player::EquipItem(dest, pItem, update)` | Move into equipment slot, apply item-mods (stats, enchant procs), set bind-on-equip, send updates | `_ApplyItemMods`, `RemoveItem` |
| `Player::CanEquipItem(slot, dest, item, swap, not_loading)` | Validate proficiency, level, skill, reputation, spec, not-disarmed, not-stunned, not-in-arena, etc. | `GetSkillValue`, `GetReputationRank`, … |
| `Player::QuickEquipItem(pos, item)` | Fast-path used by spells that auto-equip (e.g. Hand of Justice on use) | — |
| `Player::AutoUnequipChildItem(parent)` | Drops linked thrown/ammo/relic when parent unequipped | — |
| `Player::MoveItemFromInventory(bag, slot, update)` | Drop reference but DO NOT delete (used for trade, mail, sell) | `RemoveItem` |
| `Player::MoveItemToInventory(dest, item, update, in_characterInventoryDB)` | Re-attach an unattached item | `StoreItem` |
| `Player::DestroyItem(bag, slot, update)` | Permanent destroy; cascades to gems/sockets, sends `SMSG_INVENTORY_CHANGE_FAILURE` if blocked | `Item::DeleteFromDB`, RemoveItem |
| `Player::DestroyItemCount(itemId, count, update, unequip_check)` | Destroy N stacks of item by entry | — |
| `Player::SplitItem(srcBag, srcSlot, dstBag, dstSlot, count)` | Split a stack into a new position | StoreItem |
| `Player::SwapItem(src, dst)` | Bidirectional swap with full validation (CanEquip if dst is equip slot, CanStore if storage) | — |
| `Player::AutoStoreLoot(lootid, store_template, mode, broadcast)` | Move all loot items into player inventory | StoreNewItem |
| `Player::AddItem(itemId, count)` | GM/script convenience | StoreNewItem |
| `Player::HasItemCount(itemId, count, inBankAlso)` | Inventory query | — |
| `Player::GetItemCount(itemId, inBankAlso, skipItem=nullptr)` | Total count | — |
| `Player::GetItemByGuid(guid)` | Look up by item guid across all containers | — |
| `Player::GetItemByPos(bag, slot)` | Lookup by position (bag=255 for player inventory, else equipped bag pos) | — |
| `Player::GetWeaponForAttack(attType, useable)` | Returns mainhand/offhand/ranged item based on attack type | GetItemByPos |
| `Player::DurabilityLoss(item, percent)` / `DurabilityLossAll(percent, inventory)` | Damage durability; if reaches 0, broken (no stat) | item update fields |
| `Player::DurabilityRepair(pos, cost_mult, discountMod, guildBank)` / `DurabilityRepairAll(...)` | Repair single/all; charge based on `DurabilityCosts.dbc` | money |
| `Player::DurabilityPointsLoss(item, points)` / `DurabilityPointLossForEquipSlot(slot)` | Per-attack durability damage | — |
| `Player::SendEquipError(msg, item1, item2, itemid=0)` | Wrap `SMSG_INVENTORY_CHANGE_FAILURE` | — |
| `Player::BuyItemFromVendorSlot(...)` | Vendor purchase: validate vendor stock, money/extended cost, slot calc, store item | `StoreNewItem` |
| `Player::SellItem(item, ...)` | Vendor sell: refund if refundable; otherwise add to buyback queue | — |
| `Player::AddItemToBuyBackSlot(item)` / `BuyBackItem(slot)` | Buyback queue | — |
| `Player::WrapItem(...)` | Convert wrap item + target into wrapped gift (sets `ITEM_FIELD_FLAG_WRAPPED`, stores giftCreator + original entry in dynamic data) | — |
| `Player::OpenItem(item, ...)` | Unwrap gift, restore original item; or use openable container (e.g. lockbox) | — |
| `Player::TakeExtendedCost(...)` | Currency / honor / arena point deduction at vendor purchase | — |
| `Player::AddVoidStorageItem(item)` / `DeleteVoidStorageItem(slot)` / `SwapVoidStorageItem(a, b)` / `GetVoidStorageItem(slot)` | Void storage slot management | `_voidStorageItems[160]` |
| `Player::_LoadInventory(result, timeDiff)` | Build full inventory tree from `character_inventory` ⨝ `item_instance` rows | DB |
| `Player::_LoadVoidStorage(result)` | Build void storage from rows | DB |
| `Player::_LoadEquipmentSets(result)` / `_LoadTransmogOutfits(result)` | Equipment set + transmog outfit caches | DB |
| `Player::_SaveInventory(trans)` / `_SaveItem(...)` / `_SaveEquipmentSets(trans)` / `_SaveTransmogOutfits(trans)` / `_SaveVoidStorage(trans)` | Persist deltas | DB |
| `Player::SaveInventoryAndGoldToDB(trans)` | Money + inventory atomic save | DB |
| `Player::ApplyEquipCooldown(item)` | 30s cooldown after auto-equip in combat | — |
| `Player::CheckTitanGripPenalty()` | Two-handed-1H special case | — |
| `Player::TradeCancel(sendback)` | Restore trade items if rejected | — |
| `Player::GetBankBagSlotCount()` / `SetBankBagSlotCount(count)` | Number of bank bag slots purchased | — |
| Handlers `WorldSession::HandleAutoEquipItemOpcode`, `HandleSwapItem`, `HandleSwapInvItemOpcode`, `HandleSplitItemOpcode`, `HandleAutoStoreBagItemOpcode`, `HandleDestroyItemOpcode`, `HandleSellItemOpcode`, `HandleBuybackItem`, `HandleBuyItemOpcode`, `HandleListInventoryOpcode`, `HandleBuyBankSlotOpcode`, `HandleBankerActivateOpcode`, `HandleAutoBankItemOpcode`, `HandleAutoStoreBankItemOpcode`, `HandleWrapItem`, `HandleOpenItemOpcode`, `HandleUseItemOpcode`, `HandleReadItem`, `HandleRepairItemOpcode`, `HandleSocketGems`, `HandleSortBags`, `HandleSortBankBags`, `HandleEquipmentSetSave`, `HandleDeleteEquipmentSet`, `HandleUseEquipmentSet`, `HandleTransmogrifyItems`, `HandleVoidStorageQuery`, `HandleVoidStorageUnlock`, `HandleVoidStorageTransfer`, `HandleVoidSwapItem`, `HandleCancelTempEnchantmentOpcode`, `HandleItemRefund`, `HandleItemTextQuery`, `HandleRemoveNewItem` | (each opcode listed in §7) | (massive cross-call into Player methods) |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Object` — `Item : Object`, `Bag : Item`, update field framework.
- `DataStores` — `Item.db2`, `ItemSparse.db2`, `ItemModifiedAppearance.db2`, `ItemAppearance.db2`, `ItemBonus.db2`, `ItemEffect.db2`, `ItemSearchName.db2`, `ItemSpec.db2`, `ItemSet.db2`, `ItemRandomProperties.db2`, `ItemRandomSuffix.db2`, `ItemEnchantmentTemplate.db2`, `SpellItemEnchantment.db2`, `Lock.db2`, `DurabilityCosts.dbc`, `DurabilityQuality.dbc`.
- `Globals/ObjectMgr` — `GetItemTemplate`, `GetEquipmentSetGuidByID`, item-related multimaps.
- `Spells/SpellMgr` + `Spells/SpellInfo` — Use-item effects, gem socket bonus enchant, lock-pick, item-procs.
- `Combat/Unit` — `_ApplyItemMods` plumbed through `AddStatBuffMod`, weapon damage range, attack speed.
- `Skills` — equip skill check, gem socket bonus reqs, lockpicking.
- `Reputation` — equip rep gating.
- `Mails` — moving items into mail attachments.
- `Trade` — moving items between players.
- `AuctionHouse` — moving items to/from auction.
- `Loot/LootMgr` — `AutoStoreLoot`, item creation context.
- `Quests` — quest item delivery, quest reward items.
- `Guild` — guild bank slots and movement.
- `World` — `CONFIG_DURABILITY_LOSS_*`, `RATE_DURABILITY_LOSS_*`, max stacking config.
- `RBAC` — GM-only flags (e.g. `RBAC_PERM_LOG_GM_TRADE`).
- `Database/CharacterDatabase` — item/inventory/void/transmog/equipset prepared statements.

**Depended on by:**
- Almost everything user-facing: combat (weapon/armor), spells (reagents, ammo), loot, vendor, mail, trade, auction, quests, guild bank, achievements (item-based criteria), warden (anti-cheat item duplication detection).

---

## 6. SQL / DB queries (if any)

DB: `character` (player-owned items + inventory layout) and `world` (`item_template`, `item_loot_template`, `item_enchantment_template`, etc.).

### Character DB (per-player state)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER_INVENTORY` | Massive: `SELECT <item fields>, bag, slot FROM character_inventory ci JOIN item_instance ii ON ci.item = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid WHERE ci.guid = ? ORDER BY (ii.flags & 0x80000) ASC, bag ASC, slot ASC` | character |
| `CHAR_REP_INVENTORY_ITEM` | `REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)` | character |
| `CHAR_DEL_CHAR_INVENTORY_BY_ITEM` | `DELETE FROM character_inventory WHERE item = ?` | character |
| `CHAR_DEL_CHAR_INVENTORY_BY_BAG_SLOT` | `DELETE FROM character_inventory WHERE bag = ? AND slot = ? AND guid = ?` | character |
| `CHAR_DEL_CHAR_INVENTORY` | Cascade on character delete | character |
| `CHAR_INS_ITEM_INSTANCE` | `INSERT INTO item_instance (guid, itemEntry, owner_guid, ...)` | character |
| `CHAR_UPD_ITEM_INSTANCE` | Update durability/charges/flags etc. | character |
| `CHAR_DEL_ITEM_INSTANCE` | `DELETE FROM item_instance WHERE guid = ?` | character |
| `CHAR_DEL_ITEM_INSTANCE_BY_OWNER` | Cascade on character delete | character |
| `CHAR_UPD_ITEM_OWNER` | `UPDATE item_instance SET owner_guid = ? WHERE guid = ?` (mail return-to-sender, faction change) | character |
| `CHAR_INS_ITEM_INSTANCE_GEMS` | `INSERT INTO item_instance_gems (itemGuid, gemItemId1, gemBonuses1, gemContext1, gemItemId2, …, gemContext3) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)` | character |
| `CHAR_DEL_ITEM_INSTANCE_GEMS` | per-itemGuid | character |
| `CHAR_INS_ITEM_INSTANCE_TRANSMOG` | Transmog overrides per equipped piece (`itemModifiedAppearanceAllSpecs`, per-spec, secondary appearance) | character |
| `CHAR_DEL_ITEM_INSTANCE_TRANSMOG` | per-itemGuid | character |
| `CHAR_INS_ITEM_REFUND_INSTANCE` | `(item_guid, player_guid, paidMoney, paidExtendedCost)` for vendor refund | character |
| `CHAR_DEL_ITEM_REFUND_INSTANCE` | When refund expires (2h) or item modified | character |
| `CHAR_INS_ITEM_BOP_TRADE` | BoP-tradeable group window | character |
| `CHAR_DEL_ITEM_BOP_TRADE` | Window expiry | character |
| `CHAR_UPD_CHAR_INVENTORY_FACTION_CHANGE` | Re-map item entries (e.g. faction-specific rep tabards) on faction change | character |
| `CHAR_SEL_CHAR_INVENTORY_COUNT_ITEM` | Count items by entry across player | character |
| `CHAR_SEL_CHAR_VOID_STORAGE` | `SELECT itemId, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context FROM character_void_storage WHERE playerGuid = ?` | character |
| `CHAR_REP_CHAR_VOID_STORAGE_ITEM` | `REPLACE INTO character_void_storage(...) VALUES (...)` | character |
| `CHAR_DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID` | Cascade | character |
| `CHAR_DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT` | per slot | character |
| `CHAR_SEL_CHAR_EQUIPMENT_SETS` | `SELECT setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0..item18 FROM character_equipmentsets WHERE guid = ? ORDER BY setindex` | character |
| `CHAR_INS_EQUIP_SET` / `CHAR_UPD_EQUIP_SET` / `CHAR_DEL_EQUIP_SET` | Saved equipment set CRUD | character |
| `CHAR_DEL_CHAR_EQUIPMENTSETS` | Cascade on character delete | character |
| `CHAR_SEL_CHAR_TRANSMOG_OUTFITS` | `SELECT setguid, setindex, name, iconname, ignore_mask, appearance0..appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ? ORDER BY setindex` | character |
| `CHAR_INS_TRANSMOG_OUTFIT` / `CHAR_UPD_TRANSMOG_OUTFIT` / `CHAR_DEL_TRANSMOG_OUTFIT` | Transmog outfit CRUD | character |
| `CHAR_DEL_CHAR_TRANSMOG_OUTFITS` | Cascade | character |

### World DB (template/static)

| Statement / Table | Purpose | DB |
|---|---|---|
| `item_template` | Static item template (overrides/extends DB2 fields with server-defined sell price, max-count, etc.) | world |
| `item_loot_template`, `disenchant_loot_template`, `prospecting_loot_template`, `milling_loot_template`, `pickpocketing_loot_template` | Loot generators tied to item entries | world |
| `item_enchantment_template` | Random property weights for items | world |
| `item_set_names` | Set bonus name overrides (rare) | world |
| `npc_vendor` | Vendor inventory (item entries the NPC sells, with extended costs) | world |
| `playercreateinfo_item` | Starting items per race/class | world |
| `mail_loot_template` | Mail attachment loot (used by quest reward mail) | world |

### DBC/DB2 stores

| Store | What it loads | Read by |
|---|---|---|
| `Item.db2` (`sItemStore`) | Basic class/subclass/inventory_type/material/sheathe per item id | `ItemTemplate::BasicData` build, reading inventory_type for equip-slot mapping |
| `ItemSparse.db2` | The huge 90+ field record per item | `ItemTemplate::ExtendedData` |
| `ItemAppearance.db2`, `ItemModifiedAppearance.db2` | Transmog appearance data | transmog handler |
| `ItemBonus.db2` + `ItemBonusListLevelDelta.db2` | Bonus list applications (item upgrades, raid difficulty mods) | `Item::AddBonuses` |
| `ItemRandomProperties.db2` (or DBC) / `ItemRandomSuffix.db2` | "of the X" suffix tables | `ItemEnchantmentMgr::GenerateItemRandomPropertyId` |
| `ItemEffect.db2` | Spells triggered/granted by item (Use:, Equip:, Chance on hit:) | spell system |
| `Spell*ItemEnchantment.db2` | Enchant properties (stat bonus, proc spell) | apply enchant logic |
| `Lock.db2` | Lock entries: skill required, key id required, lock difficulty | spell `LOCKPICKING` |
| `DurabilityCosts.dbc` | Per-item-level repair cost base | `DurabilityRepair` |
| `DurabilityQuality.dbc` | Per-item-quality multiplier (Common < Epic) | `DurabilityRepair` |

---

## 7. Wire-protocol packets (if any)

### Client → Server (CMSG)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_AUTO_EQUIP_ITEM` | client → server | `HandleAutoEquipItemOpcode` |
| `CMSG_AUTO_EQUIP_ITEM_SLOT` | client → server | `HandleAutoEquipItemSlotOpcode` |
| `CMSG_AUTO_STORE_BAG_ITEM` | client → server | `HandleAutoStoreBagItemOpcode` |
| `CMSG_AUTOBANK_ITEM` | client → server | `HandleAutoBankItemOpcode` |
| `CMSG_AUTOSTORE_BANK_ITEM` | client → server | `HandleAutoStoreBankItemOpcode` |
| `CMSG_AUTOBANK_REAGENT` / `CMSG_AUTOSTORE_BANK_REAGENT` / `CMSG_BUY_REAGENT_BANK` / `CMSG_DEPOSIT_REAGENT_BANK` / `CMSG_SORT_REAGENT_BANK_BAGS` | client → server | reagent bank handlers (post-WoLK; stub for 3.4.3) |
| `CMSG_SWAP_INV_ITEM` | client → server | `HandleSwapInvItemOpcode` |
| `CMSG_SWAP_ITEM` | client → server | `HandleSwapItem` |
| `CMSG_SPLIT_ITEM` | client → server | `HandleSplitItemOpcode` |
| `CMSG_DESTROY_ITEM` | client → server | `HandleDestroyItemOpcode` |
| `CMSG_USE_ITEM` | client → server | `HandleUseItemOpcode` |
| `CMSG_OPEN_ITEM` | client → server | `HandleOpenItemOpcode` |
| `CMSG_READ_ITEM` | client → server | `HandleReadItem` |
| `CMSG_WRAP_ITEM` | client → server | `HandleWrapItem` |
| `CMSG_REPAIR_ITEM` | client → server | `HandleRepairItemOpcode` |
| `CMSG_SOCKET_GEMS` | client → server | `HandleSocketGems` |
| `CMSG_SORT_BAGS` | client → server | `HandleSortBags` |
| `CMSG_SORT_BANK_BAGS` | client → server | `HandleSortBankBags` |
| `CMSG_BUY_ITEM` | client → server | `HandleBuyItemOpcode` |
| `CMSG_SELL_ITEM` | client → server | `HandleSellItemOpcode` |
| `CMSG_BUY_BACK_ITEM` | client → server | `HandleBuybackItem` |
| `CMSG_LIST_INVENTORY` | client → server | `HandleListInventoryOpcode` (vendor open) |
| `CMSG_BUY_BANK_SLOT` | client → server | `HandleBuyBankSlotOpcode` |
| `CMSG_BANKER_ACTIVATE` | client → server | `HandleBankerActivateOpcode` |
| `CMSG_CANCEL_TEMP_ENCHANTMENT` | client → server | `HandleCancelTempEnchantmentOpcode` |
| `CMSG_ITEM_PURCHASE_REFUND` | client → server | `HandleItemRefund` |
| `CMSG_ITEM_TEXT_QUERY` | client → server | `HandleItemTextQuery` |
| `CMSG_REMOVE_NEW_ITEM` | client → server | `HandleRemoveNewItem` (clear "new" badge) |
| `CMSG_GET_ITEM_PURCHASE_DATA` | client → server | `HandleGetItemPurchaseData` |
| `CMSG_SAVE_EQUIPMENT_SET` | client → server | `HandleEquipmentSetSave` |
| `CMSG_USE_EQUIPMENT_SET` | client → server | `HandleUseEquipmentSet` |
| `CMSG_DELETE_EQUIPMENT_SET` | client → server | `HandleDeleteEquipmentSet` |
| `CMSG_ASSIGN_EQUIPMENT_SET_SPEC` | client → server | `HandleAssignEquipmentSetSpec` |
| `CMSG_TRANSMOGRIFY_ITEMS` | client → server | `HandleTransmogrifyItems` |
| `CMSG_QUERY_VOID_STORAGE` | client → server | `HandleVoidStorageQuery` |
| `CMSG_UNLOCK_VOID_STORAGE` | client → server | `HandleVoidStorageUnlock` |
| `CMSG_VOID_STORAGE_TRANSFER` | client → server | `HandleVoidStorageTransfer` |
| `CMSG_SWAP_VOID_ITEM` | client → server | `HandleVoidSwapItem` |
| `CMSG_CHANGE_BANK_BAG_SLOT_FLAG` | client → server | `HandleChangeBankBagSlotFlag` |

### Server → Client (SMSG)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_INVENTORY_CHANGE_FAILURE` | server → client | `Player::SendEquipError` |
| `SMSG_ITEM_PUSH_RESULT` | server → client | `Player::SendNewItem` (after StoreItem) |
| `SMSG_SELL_RESPONSE` | server → client | `HandleSellItemOpcode` |
| `SMSG_BUY_FAILED` / `SMSG_BUY_SUCCEEDED` | server → client | `HandleBuyItemOpcode` |
| `SMSG_SET_ITEM_PURCHASE_DATA` | server → client | refund metadata after purchase |
| `SMSG_ITEM_TIME_UPDATE` | server → client | refund/expiration ticking |
| `SMSG_ITEM_PURCHASE_REFUND_RESULT` | server → client | refund result |
| `SMSG_LIST_INVENTORY` | server → client | vendor list response |
| `SMSG_ENCHANTMENT_LOG` | server → client | new enchant applied notice |
| `SMSG_ITEM_ENCHANT_TIME_UPDATE` | server → client | enchant duration tick |
| `SMSG_DURABILITY_DAMAGE_DEATH` | server → client | death durability loss |
| `SMSG_LOAD_EQUIPMENT_SET` | server → client | Login: send saved equipment sets |
| `SMSG_EQUIPMENT_SET_ID` | server → client | After save: server-assigned guid |
| `SMSG_USE_EQUIPMENT_SET_RESULT` | server → client | Use equipment set result |
| `SMSG_QUERY_ITEM_TEXT_RESPONSE` | server → client | item text (letters) |
| `SMSG_READ_ITEM_RESULT_OK` / `SMSG_READ_ITEM_RESULT_FAILED` | server → client | book reading |
| `SMSG_SOCKET_GEMS_FAILURE` / `SMSG_SOCKET_GEMS_SUCCESS` | server → client | gem socketing |
| `SMSG_VOID_STORAGE_CONTENTS` | server → client | response to query |
| `SMSG_VOID_STORAGE_FAILED` | server → client | unlock/transfer failure |
| `SMSG_VOID_STORAGE_TRANSFER_CHANGES` | server → client | post-transfer state |
| `SMSG_VOID_TRANSFER_RESULT` | server → client | transfer result |
| `SMSG_VOID_ITEM_SWAP_RESPONSE` | server → client | swap result |
| `SMSG_ACCOUNT_TRANSMOG_UPDATE` | server → client | account-wide transmog collection (post-WoLK) |
| `SMSG_FORCE_RANDOM_TRANSMOG_TOAST` | server → client | (post-WoLK) |

(Total ~45 opcodes — far more than the "35+" minimum requested.)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `crates/wow-data/src/item_stats.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/item.rs` | `file` | 1 | 395 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/item.rs` — 123 lines — minimal `ItemRecord` reader for `Item.db2` (id, class, subclass, material, inventory_type, sheathe). Provides `inventory_type(entry_id)` lookup.
- `crates/wow-data/src/item_stats.rs` — 424 lines — likely ItemSparse.db2 stats / stat-allocation logic (not deeply audited here).
- `crates/wow-packet/src/packets/item.rs` — 395 lines — packet types `SwapInvItem`, `InvUpdate` bit-packed reader, partial `InventoryResult` enum (only 14 codes vs 118 in C++).
- `crates/wow-world/src/handlers/character.rs` — 4611 lines — handles character creation including equipping starting items via `equip_slot_for_inventory_type` (around line 3691); handles `BuyItem` with `INSERT INTO item_instance` + `character_inventory` (around lines 3289-3311) and `SellItem` with `DELETE` cascade (lines 3426-3437). All bag/inventory state lives transiently on the session as a flat 11-slot equipped array (not the full 19 + 4 + 16 + bank model).
- `crates/wow-database/src/statements/character.rs` — has prepared statements: `character_inventory` SELECT/UPDATE/DELETE, `item_instance` INSERT/SELECT-MAX-GUID/DELETE. Equipment sets, transmog, void storage, gems, refund: ABSENT.

**What's implemented:**
- `Item.db2` partial reader.
- 11 equipped slots in session state (head, neck, shoulders, chest, waist, legs, feet, wrists, hands, mainhand, offhand approximately; not the full 19-slot model).
- Vendor `BuyItem` and `SellItem` handlers with single-row insert/delete on `item_instance` + `character_inventory`.
- Auto-equip starting items at character creation by `inventory_type`.
- Partial wire format for `CMSG_SWAP_INV_ITEM` (`SwapInvItem` packet type).

**What's missing vs C++:**
- **Full 211-slot inventory tree**: equipment (19) + profession (11) + bag-slots (4) + reagent-bag (1) + backpack (24) + bank-items (28) + bank-bags (7) + buyback (12) + keyring (32) + child-equipment (3) + equipable-spell (16). Currently 11.
- **`Bag` class**: no representation of equipped bags as containers; cannot store anything in a bag.
- **Bank**: no bank slots, no `HandleBankerActivateOpcode`, no `BuyBankSlot`.
- **Reagent bank**: stub.
- **Buyback**: not implemented; sold items deleted instead of queued.
- **Void Storage**: 160 slots, 4 opcodes, separate id space — entirely absent.
- **Transmog**: outfit save/load/apply, `TRANSMOGRIFY_ITEMS` opcode, `item_instance_transmog` table, `character_transmog_outfits` table — entirely absent.
- **Equipment Sets**: `SAVE_EQUIPMENT_SET`, `USE_EQUIPMENT_SET`, `DELETE_EQUIPMENT_SET`, `ASSIGN_EQUIPMENT_SET_SPEC`, `LOAD_EQUIPMENT_SET` — absent.
- **Durability**: no durability damage on death/PvP/swing, no repair handler.
- **Wrap/Open**: gift wrapping system completely absent.
- **Read Item**: book/letter reading absent.
- **Socket Gems**: gem socketing handler absent (and `item_instance_gems` table not used).
- **Random Properties / Suffix**: looter rewards always have property=0 currently.
- **Refund**: `item_refund_instance` table not used; vendor refund window not implemented.
- **BoP Tradeable**: 2-hour group-trade window absent.
- **Soulbound**: items don't auto-bind on equip / on pickup.
- **Conjured / Expiration**: items with `Expiration` field don't tick down or auto-destroy.
- **Spell Charges**: `SpellCharges[5]` not modeled.
- **Enchantments**: 13 enchantment slots not modeled in items; `SMSG_ENCHANTMENT_LOG` not sent.
- **Item Push Result**: every store/loot only updates state — does not send `SMSG_ITEM_PUSH_RESULT` with new-item glow/notification.
- **Inventory Change Failure**: not all reject paths produce `SMSG_INVENTORY_CHANGE_FAILURE` with the proper error code (only 14 of 118 `InventoryResult` codes defined in Rust).
- **Sort Bags**: client-driven sort opcode unhandled.
- **Item Text Query**: book/letter text retrieval absent.
- **Vendor extended costs**: honor / arena / currency points not deductible at purchase.
- **Auto-store loot full path**: only basic insert; no stack-merging across multiple bags, no fall-back to mail-on-overflow.
- **`Player::CanStoreItem` / `CanEquipItem` full validation chain**: most of the 118 `InventoryResult` checks not performed (level, skill, faction, spec, dual-wield, two-hand-with-shield, unique-equipped, etc.).
- **Repair**: no `DurabilityRepair` cost calc or money deduction.
- **Death durability loss**: `Player::DurabilityLossAll(0.10, true)` on death not applied.
- **Cancel Temp Enchantment**: not implemented.
- **Item.db2 reader**: only 6 fields read; ItemSparse.db2 (the big record) loader status unclear from §8 inspection alone.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `inventory_type` in Item.db2 is signed `i8` (-1 = non-equippable). Rust reads it as `i8` then casts to `u8` — equipping a `-1`-typed item would have wrap-around issues (255 ≠ a valid slot). Confirm during audit.
- `BuyItem` does NOT currently re-stack onto existing items of same entry — every purchase creates a new item_instance row even for stackable items.
- `SellItem` is permanent destroy, not buyback. Selling-then-clicking-buyback will dupe-fail.
- 11-slot session state likely stores items as flat structs without `Item` entity update fields, so any future spell that reads `m_itemData->Durability` / `Enchantment[]` will need a full rewrite.

**Tests existing:**
- `crates/wow-data/src/item.rs::tests::test_load_item_store` — verifies Item.db2 reader for known entries (Thunderfury 19019, Hearthstone 6948).
- No tests for `CanStoreItem`, swap, equip, sell, buy invariants.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#INVENTORY.WBS.001** Cerrar la migracion auditada de `game/Entities/Item/Container/Bag.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Container/Bag.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.002** Cerrar la migracion auditada de `game/Entities/Item/Container/Bag.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Container/Bag.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.003** Partir y cerrar la migracion auditada de `game/Entities/Item/Item.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2199 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.004** Cerrar la migracion auditada de `game/Entities/Item/Item.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.005** Cerrar la migracion auditada de `game/Entities/Item/ItemDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemDefines.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.006** Cerrar la migracion auditada de `game/Entities/Item/ItemEnchantmentMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemEnchantmentMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.007** Cerrar la migracion auditada de `game/Entities/Item/ItemEnchantmentMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemEnchantmentMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.008** Cerrar la migracion auditada de `game/Entities/Item/ItemTemplate.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.009** Partir y cerrar la migracion auditada de `game/Entities/Item/ItemTemplate.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 873 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#INVENTORY.WBS.010** Partir y cerrar la migracion auditada de `game/Entities/Item/enuminfo_ItemDefines.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 538 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.
Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#INV.1** Define complete slot enums in `crates/wow-data/src/inventory_slots.rs`: `EquipmentSlots` (0-18), `ProfessionSlots` (19-29), `InventorySlots` (30-33 bag), `ReagentBagSlots` (34), `InventoryPackSlots` (35-58), `BankItemSlots` (59-86), `BankBagSlots` (87-93), `BuyBackSlots` (94-105), `KeyRingSlots` (106-137), `ChildEquipmentSlots` (138-140); constants `INVENTORY_SLOT_BAG_0=255`, `INVENTORY_DEFAULT_SIZE=16`, `MAX_BAG_SIZE=36`, `VOID_STORAGE_MAX_SLOT=160` (M)
- [ ] **#INV.2** Define full `InventoryResult` enum (all 118 codes from `ItemDefines.h`) replacing the 14-code subset in `crates/wow-packet/src/packets/item.rs` (M)
- [ ] **#INV.3** Define `BuyResult`, `SellResult`, `EnchantmentSlot` (13 slots), `ItemBondingType`, `ItemQualities` (8 ranks), full `ItemFlags`/`ItemFlags2`/`ItemFlags3`/`ItemFlags4` bitflags, `ItemFieldFlags` (dynamic) (M)
- [ ] **#INV.4** Define complete `ItemTemplate` struct mirroring `ItemTemplate.h` (~150 fields). Build from `Item.db2` + `ItemSparse.db2` + worldserver `item_template` table merge. Cache `ItemSpec` lookups (XL — split per category)
- [ ] **#INV.5** Define `Item` entity with full `ItemData` update fields (Owner, ContainedIn, Creator, GiftCreator, StackCount, Expiration, SpellCharges[5], DynamicFlags[1+2], Enchantment[13], PropertySeed, RandomPropertiesID, Durability, MaxDurability, CreatePlayedTime, Context, BonusListIDs, ItemAppearanceModID), state machine NEW/CHANGED/UNCHANGED/REMOVED, queue position (H)
- [ ] **#INV.6** Define `Bag` entity wrapping `Item` with `slots: [Option<Arc<Item>>; 36]`, `num_slots: u8` (template-driven), `m_containerData` update fields (H)
- [ ] **#INV.7** Add prepared statements for missing tables in `crates/wow-database/src/statements/character.rs`: `INS/DEL/SEL_ITEM_INSTANCE_GEMS`, `INS/DEL/SEL_ITEM_INSTANCE_TRANSMOG`, `INS/DEL_ITEM_REFUND_INSTANCE`, `INS/DEL_ITEM_BOP_TRADE`, `SEL/INS/UPD/DEL_EQUIP_SET`, `SEL/INS/UPD/DEL_TRANSMOG_OUTFIT`, `SEL/REP/DEL_CHAR_VOID_STORAGE_ITEM`, `UPD_ITEM_OWNER`, `UPD_CHAR_INVENTORY_FACTION_CHANGE` (M)
- [ ] **#INV.8** Implement `Player::_load_inventory(result, time_diff)` — parses big JOIN query result into bag-slot tree, restores stacks/durability/flags/enchantments/gems/transmog (XL — split per item-attribute group)
- [ ] **#INV.9** Implement `Player::_save_inventory(trans)` — iterate dirty items per `m_state`, INSERT/UPDATE/DELETE atomically; also persist `character_inventory` (bag, slot, item) tuples (H)
- [ ] **#INV.10** Implement `Player::can_store_item(bag, slot, &mut dest, count, item, swap)` matching all 118 `InventoryResult` reject paths — proficiency (skill), level, faction rep, spec, two-hand vs offhand, unique-equipped, max-count, conjured-bag rules, account-bound rules, locked, in-arena, dead, stunned, disarmed, dual-wield, ammo-type-match, quiver-only, bag-special-type, item-max-limit-category (XL — split per category)
- [ ] **#INV.11** Implement `Player::can_equip_item` covering equip-only checks (level scaling, talent requirement, shapeshift, weapon proficiency, two-hand+shield, …) (H)
- [ ] **#INV.12** Implement `Player::store_item(dest, item, update)` and `Player::store_new_item(dest, itemId, update, randomProp, looters, context, bonusList)` — handle stack merge, slot assign, fire `SMSG_ITEM_PUSH_RESULT`, mark soulbound on BoP (H)
- [ ] **#INV.13** Implement `Player::equip_item` — full apply-mods chain (stat bonuses, on-equip spells, set bonuses, weapon visuals), trigger 30s equip cooldown, send updates (H)
- [ ] **#INV.14** Implement `Player::destroy_item(bag, slot, update)` with cascading deletes (gems, transmog, refund, bop-trade) (M)
- [ ] **#INV.15** Implement `Player::split_item(srcBag, srcSlot, dstBag, dstSlot, count)` (M)
- [ ] **#INV.16** Implement `Player::swap_item(src, dst)` with full bidirectional validation (CanEquip/CanStore on each side, two-handed special-cases) (H)
- [ ] **#INV.17** Implement `Player::auto_store_loot(lootid, store_template, mode, broadcast)` for full-bag fallback semantics (overflow → inventory full + send via mail satchel) (M)
- [ ] **#INV.18** Handler `handle_auto_equip_item` (CMSG_AUTO_EQUIP_ITEM) (M)
- [ ] **#INV.19** Handler `handle_auto_equip_item_slot` (CMSG_AUTO_EQUIP_ITEM_SLOT) (L)
- [ ] **#INV.20** Handler `handle_swap_inv_item` (full validation, stack merge) — supersedes the partial implementation (M)
- [ ] **#INV.21** Handler `handle_swap_item` (between bag/non-bag containers) (M)
- [ ] **#INV.22** Handler `handle_split_item` (L)
- [ ] **#INV.23** Handler `handle_auto_store_bag_item` (move-from-equipped-bag) (L)
- [ ] **#INV.24** Handler `handle_destroy_item` (validate not-locked, not-in-trade) (L)
- [ ] **#INV.25** Handler `handle_use_item` — invokes spell cast; charges deduction; consumable destroy on use (depends on Spells module) (M)
- [ ] **#INV.26** Handler `handle_open_item` (lockbox, gift) — calls Lock.db2 spell or gift-unwrap path (M)
- [ ] **#INV.27** Handler `handle_read_item` (`SMSG_READ_ITEM_RESULT_OK/FAILED`) — checks item is `READABLE` and has `PageText` (L)
- [ ] **#INV.28** Handler `handle_wrap_item` — convert wrapping paper (`ITEM_FLAG_WRAPPER`) + target into wrapped gift, store original entry/creator inside wrapped item dynamic data (M)
- [ ] **#INV.29** Handler `handle_repair_item` — single + all variants; cost = `DurabilityCosts.dbc[ilvl] * DurabilityQuality.dbc[quality] * percent_lost`; guild-bank-funded variant (depends on Guild module) (H)
- [ ] **#INV.30** Handler `handle_socket_gems` — validate sockets, gem types, calc `ItemSocketColors`, apply enchant in `SOCK_ENCHANTMENT_SLOT_1..3`, persist `item_instance_gems` row, send `SMSG_SOCKET_GEMS_SUCCESS/FAILURE` (H)
- [ ] **#INV.31** Handler `handle_sort_bags` / `handle_sort_bank_bags` (algorithmic — group by quality/category) (M)
- [ ] **#INV.32** Handler `handle_buy_item` — full version with extended costs (currency/honor/arena), stack merge, vendor stock decrement, refund instance creation (H — supersedes partial impl)
- [ ] **#INV.33** Handler `handle_sell_item` — refund money if refundable, otherwise add to buyback queue with `SetSlot(BUYBACK_SLOT_START + idx)` (M — supersedes partial impl)
- [ ] **#INV.34** Handler `handle_buyback_item` — restore from buyback queue, deduct money (sale price) (M)
- [ ] **#INV.35** Handler `handle_list_inventory` (open vendor) — sends `SMSG_LIST_INVENTORY` with vendor stock (M)
- [ ] **#INV.36** Handler `handle_buy_bank_slot` — purchase next bank bag slot at scaling cost (`BankBagSlotPrices.dbc`) (L)
- [ ] **#INV.37** Handler `handle_banker_activate` (open bank UI) — validates NPC has `UNIT_NPC_FLAG_BANKER` (L)
- [ ] **#INV.38** Handler `handle_auto_bank_item` / `handle_auto_store_bank_item` (M)
- [ ] **#INV.39** Handler `handle_cancel_temp_enchantment` — clear `TEMP_ENCHANTMENT_SLOT`, send updates (L)
- [ ] **#INV.40** Handler `handle_item_refund` — within 2h refund window, refund money, restore stock; outside window: error (M)
- [ ] **#INV.41** Handler `handle_item_text_query` — return text for letter items (links with Mail #MAILS.17) (L)
- [ ] **#INV.42** Handler `handle_remove_new_item` — clear "new item glow" flag (L)
- [ ] **#INV.43** Handler `handle_get_item_purchase_data` — return refund metadata for an item (L)
- [ ] **#INV.44** Equipment Sets full impl: load on login, `handle_save_equipment_set`, `handle_use_equipment_set`, `handle_delete_equipment_set`, `handle_assign_equipment_set_spec`. Persist via `character_equipmentsets`. Send `SMSG_LOAD_EQUIPMENT_SET` post-login, `SMSG_EQUIPMENT_SET_ID` post-save, `SMSG_USE_EQUIPMENT_SET_RESULT` post-use (H)
- [ ] **#INV.45** Transmog Outfits + per-item transmog: load on login, `handle_transmogrify_items` (with per-spec appearance arrays). Persist via `character_transmog_outfits` and per-item `item_instance_transmog`. (H)
- [ ] **#INV.46** Void Storage full impl: `_load_void_storage`, `add_void_storage_item`, `delete_void_storage_item`, `swap_void_storage_item`, separate item-id space. Handlers `handle_void_storage_query`, `_unlock`, `_transfer`, `_swap_item`. Persist via `character_void_storage`. (H)
- [ ] **#INV.47** Durability system: `durability_loss(item, percent)`, `durability_loss_all`, `durability_points_loss`, `durability_repair`, `durability_repair_all`. Hook into death event (`-10%` typical), PvP, melee swings, spell-fail consume. Send `SMSG_DURABILITY_DAMAGE_DEATH` on death loss (M)
- [ ] **#INV.48** Random Property / Suffix system: load `ItemRandomProperties.db2` + `ItemRandomSuffix.db2` + worldserver `item_enchantment_template` weights; roll on item create with `Class+Subclass+Quality+ItemLevel` keys (M)
- [ ] **#INV.49** Enchantment full impl: 13 slots, `Enchantment[slot] = { ID, Duration, Charges }`, apply stat bonuses on equip, expire on duration, send `SMSG_ENCHANTMENT_LOG` and `SMSG_ITEM_ENCHANT_TIME_UPDATE` (H)
- [ ] **#INV.50** Spell charges (`SpellCharges[5]`) for items: decrement on use, destroy at zero if `ITEM_FIELD_FLAG_CONSUMABLE`-equivalent (L)
- [ ] **#INV.51** Item Expiration ticking: `m_itemData->Expiration` decrements; auto-destroy on zero (M)
- [ ] **#INV.52** Soulbound auto-bind: on equip if BoE, on pickup if BoP, on use if BoU (L)
- [ ] **#INV.53** BoP Tradeable window: persist group at pickup → 2h trade-back window via `item_instance_bop_trade` (M)
- [ ] **#INV.54** Send `SMSG_ITEM_PUSH_RESULT` from `Player::send_new_item` after every `store_item` (L)
- [ ] **#INV.55** Send `SMSG_INVENTORY_CHANGE_FAILURE` from `Player::send_equip_error` for every reject path (must include item1, item2 guids and itemId) (L)
- [ ] **#INV.56** Faction-change service: `UPD_CHAR_INVENTORY_FACTION_CHANGE` re-maps faction-specific items (e.g. tabards) (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#INVENTORY.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 10 files / 5156 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#INVENTORY.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 10 files / 5156 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#INVENTORY.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 10 files / 5156 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#INVENTORY.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 10 files / 5156 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `can_store_item` for empty backpack accepts a 1-slot item; rejects when 16 slots full; accepts in equipped 16-slot bag.
- [ ] Test: stack merging: store 5 of an item that stacks to 20, then 17 more → 1 stack of 20 + 1 stack of 2 in next slot.
- [ ] Test: `can_equip_item` rejects with `EQUIP_ERR_CANT_EQUIP_LEVEL_I` if player level < `RequiredLevel`.
- [ ] Test: `can_equip_item` rejects `EQUIP_ERR_CANT_EQUIP_SKILL` if proficiency missing.
- [ ] Test: `can_equip_item` rejects `EQUIP_ERR_CANT_EQUIP_REPUTATION` if rank insufficient.
- [ ] Test: equip BoE item → `ITEM_FIELD_FLAG_SOULBOUND` set after store.
- [ ] Test: pickup BoP from loot → flag set during `auto_store_loot`.
- [ ] Test: swap mainhand 2H weapon while shield in offhand → forces unequip of offhand into bag (or rejects with `EQUIP_ERR_2HANDED_EQUIPPED`).
- [ ] Test: split stack of 10 into (4, 6) — 6 stays, 4 moves; rejects if dest occupied with different itemId.
- [ ] Test: destroy item — soulbound rejected via `EQUIP_ERR_DROP_BOUND_ITEM` only if `INDESTRUCTIBLE` flag absent (most BoP items can drop).
- [ ] Test: durability damage 0% → `Item::IsBroken()` true; `apply_item_mods` skips stat application.
- [ ] Test: repair single item: charges `DurabilityCosts.dbc[ilvl] * DurabilityQuality.dbc[quality] * (1 - cur/max)`.
- [ ] Test: repair all charges sum across all damaged items.
- [ ] Test: death durability loss = 10% (config-driven) on every equipped item; SMSG sent.
- [ ] Test: vendor sell refundable item within 2h → returns money + restores stock; outside 2h → adds to buyback.
- [ ] Test: buyback restores item to inventory at sale price.
- [ ] Test: vendor buy with extended cost (honor/arena/currency) deducts correctly.
- [ ] Test: socket 3 gems of correct color → applies enchant in slots 2-4; record `item_instance_gems` row.
- [ ] Test: socket bonus active when all required colors met; lost when wrong color in any socket.
- [ ] Test: equipment set save → `character_equipmentsets` row; load on login; `use_equipment_set` swaps gear and produces `SMSG_USE_EQUIPMENT_SET_RESULT` per slot.
- [ ] Test: transmog single piece → `item_instance_transmog` row written; visible to other players via inspect.
- [ ] Test: void storage unlock 100g → opens; query returns 0 items initially; transfer in/out persists.
- [ ] Test: wrap item: `ITEM_FLAG_WRAPPER` paper + target → wrapped gift carries original entry; unwrap restores original soulbound state.
- [ ] Test: random property roll on create (entry with non-empty `RandomProperty` field) populates `RandomPropertiesID` and `PropertySeed`.
- [ ] Test: bank buy slot increments `BankBagSlotCount` and deducts scaling cost.
- [ ] Test: keyring slot accepts only `ITEM_CLASS_KEY` items.
- [ ] Test: ammo pouch / quiver — only one equipped at a time (`EQUIP_ERR_ONLY_ONE_QUIVER`).
- [ ] Test: load/save round-trip — equip 19 slots, fill backpack, fill 1 bag, fill bank → save → reload → identical positions and item state.
- [ ] Test: soulbound + wrapped → `EQUIP_ERR_CANT_WRAP_BOUND`.
- [ ] Test: unique-equipped enforcement (e.g. only 1 trinket of category X).
- [ ] Test: max-count limit category (e.g. only 5 of any "PvP currency reward" tier).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 10 files / 5156 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp` | `crates/wow-data/` (Item.db2, ItemTemplate, ItemStats), `crates/wow-world/` (handlers, session inventory state), `crates/wow-database/` (item_instance + character_inventory + character_void_storage + character_transmog_outfits + character_equipmentsets prepared statements), `crates/wow-packet/` (item/bank/void packets) \| ⚠️ partial (flat `HashMap<u8 slot, InventoryItem>` of equipped + backpack only; no `Item` entity, no `Bag` containers, no bank, no void, no transmog, no durability, no wrap, no repair, no buyback queue, no enchant slots) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#INVENTORY.DIV.001` | _none generated_ | 10 C++ files / 5156 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/Item.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/ItemTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/enuminfo_ItemDefines.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **`INVENTORY_SLOT_BAG_0 = 255` sentinel**: when `bag` field is 255, the item is in the player's "main" bag (equipped slots 0-18, profession 19-29, equipped-bag-slots 30-33, backpack 35-58). When `bag` is 30-33 or 87-93, the item is INSIDE that bag (with `slot` 0..bag.size). Confused handling here is the #1 bug source in TC ports.
- **Slot 30..33 dual meaning**: positions 30-33 are themselves **equipment slots** holding `Bag` items, and at the same time they identify "which equipped bag" when used as the `bag` field for sub-items. The `_LoadInventory` query `ORDER BY (ii.flags & 0x80000) ASC, bag ASC, slot ASC` ensures bags are loaded BEFORE their contents — preserve this.
- **`item_instance.flags` bit 0x80000** is `ITEM_FIELD_FLAG_CHILD` (or similar) used to gate the load order. Don't rename without updating the SELECT.
- **`MAX_BAG_SIZE = 36`** is theoretical — WoLK bags max at 22 slots (Embersilk Bag). Keep 36 to match TC array sizing for forward-compat.
- **`InventoryType` value -1** in `Item.db2` means non-equippable. Fixed in `#NEXT.R8.ENTITIES.046`: Rust keeps the signed DB2 boundary and returns no equip slot for negative/zero values, so `-1` cannot wrap to `INVENTORY_SLOT_BAG_0=255`.
- **`character_inventory.bag` column is `BIGINT UNSIGNED`** and stores the GUID of the parent bag (not the slot index). For items in the player's main inventory, `bag = 0` (or NULL on some shards). Match TrinityCore exactly.
- **Stack count** is `u32` in TC (`StackCount` update field) but most items stack to ≤200; mind upper-bound.
- **Refund 2-hour window**: uses `CreatePlayedTime` (in seconds of /played at creation) compared to current /played. Persists across logout — DO NOT use wall-clock.
- **Soulbound trade group** (BoP Tradeable): when a raid drop is picked up, all eligible looters are stored in `item_soulbound_trade_data` with a 2h timestamp. Within 2h, the original looter can trade to another eligible player. Reject after 2h.
- **Wrapped item dynamic data**: wrapped gift stores original entry+creator+random props inside its own `m_itemData->ContainedIn` and similar overloaded fields. Unwrap reverses. Don't store as a JSON blob — match TC's struct overlay.
- **Equipment Set "Pieces[i]"**: stores ITEM GUIDs (low). On `USE_EQUIPMENT_SET`, server iterates 19 slots, finds each piece by guid via `GetItemByGuid`, and queues swaps. If guid is `0`, slot is "ignored". Special value `1` means "remove whatever is equipped". Match exactly.
- **Transmog inheritance**: `ITEM_MODIFIER_TRANSMOG_APPEARANCE_ALL_SPECS` is the fallback; per-spec slots (1..5) override. Inspector packets need to send the per-spec applied to current spec.
- **Void storage item id space is SEPARATE** from `item_instance.guid`. `character_void_storage.itemId` is its own auto-increment/manual sequence. When an item is "transferred to" void storage, the original `item_instance` row is DELETED and a new void row with new itemId is created (so all enchantments/gems are STRIPPED).
- **Void storage transfer cost**: 25g per item-in plus 25g per item-out. Charge BEFORE persisting.
- **Refund instance is auto-cleaned** by maintenance scripts (TC has `clean_orphan_item_instance_refund` event). The refund window check uses the row's existence + `CreatePlayedTime` delta.
- **`UPD_ITEM_OWNER`** is critical for mail return-to-sender, faction change, GUI commands. Always run within a transaction with the corresponding `mail_items` / `character_inventory` updates.
- **Container guid in update field `ContainedIn`** points to the parent bag's guid. For items in player main inventory, it's the player's guid. For items in equipped bag (pos 30-33), it's the bag's guid. Update on every move.
- **Bag load order matters**: load all bags first (so they exist as `Item*` objects), then load their contents in a second pass, OR rely on the `flags & 0x80000 ASC` sort to guarantee bag rows precede content rows.
- **`Item::SetState(ITEM_REMOVED)`** does NOT immediately delete from DB — only flags for next `_SaveInventory` flush. If the player crashes between SetState(REMOVED) and the next save, the item reappears. TC accepts this risk; mirror it.
- **Vendor `ExtendedCost`**: an entry in `ItemExtendedCost.db2` with up to 5 currency requirements + honor + arena + faction-rep gating. Apply ALL or fail.
- **WoLK 3.4.3 specific quirks**:
  - KeyRing exists (slots 106-137); modern WoW removed it but the 3.4.3 client renders the keyring tab.
  - Reagent Bag, ChildEquipment, EquipableSpell slots are present in enum but unused by 3.4.3 client UI.
  - Profession Slots (19-29) — only `MAINHAND/OFFHAND/RANGED` count for 3.4.3 weapon profs; modern profession-tool slots are unused.
  - Heirloom items use `ITEM_FLAG_IS_BOUND_TO_ACCOUNT` (not battle.net account); the wotlk client respects same-account but battle.net bound is a no-op.
- **Locked items**: `ITEM_FIELD_FLAG_UNLOCKED` (note: the absence flag means locked). `IsLocked() = !HasItemFlag(UNLOCKED)`. Used for lockboxes that need a Lockpicking spell or a Skeleton Key.
- **`Item::IsBroken()`** = `MaxDurability > 0 && Durability == 0`. Items with `MaxDurability == 0` (jewelry, cloaks in WoLK) never break.
- **Stack split with right-click drag** uses `CMSG_SPLIT_ITEM` — the client sends source+dest+count, server validates and creates the stack. Don't trust the count blindly: clamp to `min(source.count - 1, dest.maxStack)`.
- **Performance**: `_LoadInventory` is a single big LEFT-JOIN query that returns one row per item — but loading 200+ items creates a LOT of update field allocations. Profile and consider batching the union of `_LoadInventory` + bag content into one pass.
- **Auto-store-loot fall-back to mail**: if loot from a quest/event can't fit in the inventory, TC mails a `MAIL_ITEMINVENTORY_FULL_SATCHEL` (entry 38186 "Satchel of Useful Goods" or similar). Implement only after Mail module is wired (#MAILS.* prerequisites).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Item : Object` | `struct Item` in `crates/wow-world/src/entities/item.rs` | Update fields exposed via typed accessors, not raw arrays |
| `class Bag : Item` | `struct Bag { item: Item, slots: [Option<Arc<Item>>; 36], num_slots: u8 }` | Composition over inheritance; `Bag::as_item()` for shared paths |
| `struct ItemTemplate` (~150 fields) | `struct ItemTemplate` in `crates/wow-data/src/item_template.rs` | Build at startup from DB2 + worldserver merge; immutable thereafter |
| `m_itemData->Owner` etc. | `item.data.owner: ObjectGuid` (typed update field) | Use `dirty_set` helper to track CHANGED state |
| `Player::CanStoreItem(...)` returning `InventoryResult` + writing `ItemPosCountVec` | `fn can_store_item(&self, ...) -> Result<Vec<ItemPosCount>, InventoryResult>` | Idiomatic Result; `InventoryResult` becomes the error type |
| `Player::StoreItem(dest, item, update)` | `fn store_item(&mut self, dest: &[ItemPosCount], item: Arc<Item>, update: bool) -> Arc<Item>` | Returns Arc to placed item (may be a merged stack) |
| `m_buybackitems[12]` | `buyback_slots: [Option<Arc<Item>>; 12]` | — |
| `_voidStorageItems[160]` | `void_storage: [Option<VoidStorageItem>; 160]` | Array preferred; matches TC's 160-element fixed allocation |
| `EquipmentSetInfo::EquipmentSetData` | `struct EquipmentSet { guid: u64, set_id: u32, name: String, icon: String, ignore_mask: u32, spec_index: u8, pieces: [u64; 19], type_: SetType }` | `enum SetType { Equipment, Transmog }` |
| `WorldPackets::Item::SwapInvItem` | already exists in `crates/wow-packet/src/packets/item.rs::SwapInvItem` | extend with full validation |
| `SMSG_INVENTORY_CHANGE_FAILURE` | `pub struct InventoryChangeFailure { result: InventoryResult, item1: ObjectGuid, item2: ObjectGuid, item_id: u32 }` | always 3 args; result code dictates required ones |
| `SMSG_ITEM_PUSH_RESULT` | `pub struct ItemPushResult { player: ObjectGuid, slot: u8, slot_in_bag: u32, displayId: u32, count: u32, … }` | new+pushed item visual |
| `Item::SaveToDB(trans)` | `async fn save_to_db(&self, tx: &mut Transaction) -> Result<()>` | match TC's atomic insert order: item_instance → gems → transmog → refund → bop_trade |
| `sObjectMgr->GetItemTemplate(id)` | `world.item_templates.get(&id) -> Option<&ItemTemplate>` | shared registry |
| `MAX_BAG_SIZE = 36` | `pub const MAX_BAG_SIZE: usize = 36;` | — |
| `INVENTORY_SLOT_BAG_0 = 255` | `pub const INVENTORY_SLOT_BAG_0: u8 = 255;` | — |
| `VOID_STORAGE_MAX_SLOT = 160` | `pub const VOID_STORAGE_MAX_SLOT: usize = 160;` | — |
| `DurabilityCosts.dbc` | `crates/wow-data/src/durability_costs.rs::DurabilityCostsStore` | DB2 reader |
| `Lock.db2` | `crates/wow-data/src/lock.rs::LockStore` | reader for chest/lockbox/door difficulty |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Entities/Item/{Item.h,Item.cpp,ItemTemplate.h,ItemDefines.h,Container/Bag.h}` (378 + 2199 + 873 + 289 + 90 lines) and the inventory section of `Player.cpp` (the `_LoadInventory` / `_SaveInventory` / `CanStoreItem` / `CanEquipItem` / `StoreNewItem` / `EquipItem` / `DestroyItem` / `SwapItem` / `SplitItem` / `BuyItemFromVendorSlot` / `SellItem` / `RepairItem` / `WrapItem` / `OpenItem` / `_LoadEquipmentSets` / `_LoadVoidStorage` / `_LoadTransmogOutfits` cluster) against the current Rust state. Verdict: **⚠️ partial — only flat-slot equipped/backpack tracking; the entire `Item` entity layer and all secondary storage subsystems are absent**.

### What actually exists

- `crates/wow-data/src/item.rs:21 ItemRecord` reads only 6 fields from `Item.db2`: id, class, subclass, material, inventory_type, sheathe. `inventory_type` remains loaded as C++ `int8`; as of `#NEXT.R8.ENTITIES.046`, negative/zero values return `None` for equip mapping and no longer wrap to the `INVENTORY_SLOT_BAG_0=255` sentinel.
- `crates/wow-data/src/item_stats.rs` (424 lines) holds ItemSparse-derived stat allocation logic (not deeply audited here, but doc claims it exists and grep confirms a `PlayerStatsStore` is wired into `WorldSession.player_stats`).
- `crates/wow-entities/src/item.rs` now ports base `Item` state plus modifier storage and pure visible helper precedence (`GetVisibleEntry`, appearance modifier, enchant illusion/permanent visual). `ItemModifiedAppearance` and `SpellItemEnchantment` lookups are still caller-provided bridges because the canonical DB2 stores are not yet implemented in `wow-data`.
- `crates/wow-packet/src/packets/item.rs:19 InventoryResult` enum exposes **14 codes** (Ok, CantEquipLevelI, WrongSlot, BagFull, NotEquippable, CantSwap, SlotEmpty, ItemNotFound, DropBoundItem, NotABag, NotOwner, PlayerDead, InvFull, InternalBagError) vs **118 codes** in C++ `ItemDefines.h::InventoryResult`. ~88% of reject paths are unrepresentable.
- `crates/wow-world/src/session.rs:197` carries `inventory_items: HashMap<u8, InventoryItem>` keyed by slot byte. The `InventoryItem` struct (`session.rs:321`) is just `{ guid: ObjectGuid, entry_id: u32, db_guid: u64, inventory_type: Option<u8> }` — **no stack count, no durability, no enchantments, no charges, no soulbound flag, no creator GUID, no gems, no transmog, no random property, no expiration, no bonus list IDs.** Compared to C++ `Item` which has all of those plus the `UF::ItemData` update fields blob. The Rust struct is the bare minimum to remember "slot N has item X".
- `crates/wow-world/src/handlers/character.rs` does the inventory work directly (no method on `Player` — there is no `Player`):
  - `:1330+` login handler hydrates `inventory_items` from a `character_inventory ⨝ item_instance` SELECT (line `:1425` `inventory_items.insert(slot, InventoryItem {...})`).
  - `:3273` BuyItem finds first free slot, inserts a new `item_instance` row + `character_inventory` tuple (`:3331`).
  - `:3381` SellItem deletes both rows — **permanent destroy, no buyback queue**.
  - `:3534-3557` SwapItem swaps map entries (no validation, no two-handed-with-shield check, no equip cooldown, no apply-mods).
  - Auto-equip starting items at character creation via `equip_slot_for_inventory_type` (~line 3691).
- `crates/wow-database/src/statements/character.rs` has prepared statements only for `character_inventory` SELECT/UPDATE/DELETE and `item_instance` INSERT/SELECT-MAX-GUID/DELETE. **Absent**: `item_instance_gems`, `item_instance_transmog`, `item_refund_instance`, `item_soulbound_trade_data`, `character_void_storage`, `character_equipmentsets`, `character_transmog_outfits`, `UPD_ITEM_OWNER`.

### Slot model — the 211-slot tree

Doc §3 enumerates the full C++ slot map: equipment 0-18 (19 slots) + profession 19-29 (11 slots, mostly unused in 3.4.3) + bag 30-33 (4 equipped bags) + reagent-bag 34 + backpack 35-58 (24 slots, default 16) + bank-items 59-86 (28) + bank-bags 87-93 (7) + buyback 94-105 (12) + keyring 106-137 (32, **3.4.3-specific**, modern-removed) + child-equipment 138-140 (3) + the `INVENTORY_SLOT_BAG_0=255` sentinel + `MAX_BAG_SIZE=36` for sub-bag arrays. Rust: a single `HashMap<u8, InventoryItem>`. **No slot-range enum exists, no validation that slot N belongs to the right range for the operation being performed, no two-level addressing (bag-slot, sub-slot).** The doc's claim "11 equipped slots in session state" understates it slightly — the HashMap is unbounded — but the architectural absence is the same.

### Worst divergence

**There is no `Item` entity.** Every C++ Item lifecycle method (`Create`, `LoadFromDB`, `SaveToDB`, `DeleteFromDB`, `SetBinding`, `SetState(NEW/CHANGED/UNCHANGED/REMOVED)`, `IsRefundable` / `IsBOPTradeable` / `IsWrapped` / `IsLocked` / `IsBroken`, `GetSpellCharges` / `SetSpellCharges`, `GetEnchantmentId(slot)` / `SetEnchantment`, `AddBonuses`, the `BonusData` cache) hangs off a Rust struct that doesn't exist. This propagates: durability ticking is impossible, repair cost can't be computed, enchantments can't be applied or expired, gems can't be socketed, refund 2-hour windows can't be tracked, BoP-tradeable groups can't be recorded, soulbinding can't auto-trigger on equip/pickup/use. Combined with the absence of `Bag` containers, **anything beyond "swap two items in fixed positions" is unimplementable in the current shape**. The auto-equip-starter-items + buy-from-vendor + sell-to-vendor (permanent) + swap path is the entire surface area today; the other ~40 inventory CMSG opcodes (auto-bank, auto-store-bank, banker-activate, buy-bank-slot, void-storage-query/unlock/transfer/swap, transmogrify-items, save-equipment-set, use-equipment-set, repair-item, socket-gems, wrap-item, open-item, read-item, sort-bags, cancel-temp-enchantment, item-purchase-refund, item-text-query, etc.) have no Rust handler. Coverage vs the 211-slot, 35-table-cluster C++ system: ~5%. The 56 sub-tasks in §9 stand essentially unchanged.
