// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-related packet definitions: swap, equip, destroy, and error responses.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};

// ── InventoryResult ─────────────────────────────────────────────────

pub use wow_constants::InventoryResult;

// ── Shared item packet structures ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemBonuses {
    pub context: u8,
    pub bonus_list_ids: Vec<i32>,
}

impl ItemBonuses {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.context);
        pkt.write_uint32(self.bonus_list_ids.len() as u32);
        for bonus_id in &self.bonus_list_ids {
            pkt.write_uint32(*bonus_id as u32);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMod {
    pub value: i32,
    pub modifier_type: u8,
}

impl ItemMod {
    pub const fn new(value: i32, modifier_type: u8) -> Self {
        Self {
            value,
            modifier_type,
        }
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.value);
        pkt.write_uint8(self.modifier_type);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemModList {
    pub values: Vec<ItemMod>,
}

impl ItemModList {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.values.len() as u32, 6);
        pkt.flush_bits();
        for item_mod in &self.values {
            item_mod.write(pkt);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemInstance {
    pub item_id: i32,
    pub random_properties_seed: i32,
    pub random_properties_id: i32,
    pub item_bonus: Option<ItemBonuses>,
    pub modifications: ItemModList,
}

impl ItemInstance {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.item_id);
        pkt.write_int32(self.random_properties_seed);
        pkt.write_int32(self.random_properties_id);
        pkt.write_bit(self.item_bonus.is_some());
        pkt.flush_bits();
        self.modifications.write(pkt);
        if let Some(item_bonus) = &self.item_bonus {
            item_bonus.write(pkt);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ItemPushResultDisplayType {
    Hidden = 0,
    Normal = 1,
    EncounterLoot = 2,
    QuestUpdateAddItem = 3,
}

impl Default for ItemPushResultDisplayType {
    fn default() -> Self {
        Self::Hidden
    }
}

/// SMSG_ITEM_PUSH_RESULT.
pub struct ItemPushResult {
    pub player_guid: ObjectGuid,
    pub slot: u8,
    pub slot_in_bag: i32,
    pub item: ItemInstance,
    pub quest_log_item_id: i32,
    pub quantity: i32,
    pub quantity_in_inventory: i32,
    pub dungeon_encounter_id: i32,
    pub battle_pet_species_id: i32,
    pub battle_pet_breed_id: i32,
    pub battle_pet_breed_quality: u32,
    pub battle_pet_level: i32,
    pub item_guid: ObjectGuid,
    pub pushed: bool,
    pub display_text: ItemPushResultDisplayType,
    pub created: bool,
    pub is_bonus_roll: bool,
    pub is_encounter_loot: bool,
}

impl Default for ItemPushResult {
    fn default() -> Self {
        Self {
            player_guid: ObjectGuid::EMPTY,
            slot: 0,
            slot_in_bag: 0,
            item: ItemInstance::default(),
            quest_log_item_id: 0,
            quantity: 0,
            quantity_in_inventory: 0,
            dungeon_encounter_id: 0,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            item_guid: ObjectGuid::EMPTY,
            pushed: false,
            display_text: ItemPushResultDisplayType::Hidden,
            created: false,
            is_bonus_roll: false,
            is_encounter_loot: false,
        }
    }
}

impl ServerPacket for ItemPushResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::ItemPushResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_uint8(self.slot);
        pkt.write_int32(self.slot_in_bag);
        pkt.write_int32(self.quest_log_item_id);
        pkt.write_int32(self.quantity);
        pkt.write_int32(self.quantity_in_inventory);
        pkt.write_int32(self.dungeon_encounter_id);
        pkt.write_int32(self.battle_pet_species_id);
        pkt.write_int32(self.battle_pet_breed_id);
        pkt.write_uint32(self.battle_pet_breed_quality);
        pkt.write_int32(self.battle_pet_level);
        pkt.write_packed_guid(&self.item_guid);
        pkt.write_bit(self.pushed);
        pkt.write_bit(self.created);
        pkt.write_bits(self.display_text as u32, 3);
        pkt.write_bit(self.is_bonus_roll);
        pkt.write_bit(self.is_encounter_loot);
        pkt.flush_bits();
        self.item.write(pkt);
    }
}

/// SMSG_ITEM_TIME_UPDATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTimeUpdate {
    pub item_guid: ObjectGuid,
    pub duration_left: u32,
}

impl ServerPacket for ItemTimeUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::ItemTimeUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.item_guid);
        pkt.write_uint32(self.duration_left);
    }
}

/// SMSG_ITEM_ENCHANT_TIME_UPDATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEnchantTimeUpdate {
    pub owner_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub duration_left: u32,
    pub slot: u32,
}

impl ServerPacket for ItemEnchantTimeUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::ItemEnchantTimeUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.item_guid);
        pkt.write_uint32(self.duration_left);
        pkt.write_uint32(self.slot);
        pkt.write_packed_guid(&self.owner_guid);
    }
}

/// CMSG_GET_ITEM_PURCHASE_DATA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetItemPurchaseData {
    pub item_guid: ObjectGuid,
}

impl ClientPacket for GetItemPurchaseData {
    const OPCODE: ClientOpcodes = ClientOpcodes::GetItemPurchaseData;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            item_guid: packet.read_packed_guid()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemPurchaseRefundItem {
    pub item_id: i32,
    pub item_count: i32,
}

impl ItemPurchaseRefundItem {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.item_id);
        pkt.write_int32(self.item_count);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemPurchaseRefundCurrency {
    pub currency_id: i32,
    pub currency_count: i32,
}

impl ItemPurchaseRefundCurrency {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.currency_id);
        pkt.write_int32(self.currency_count);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemPurchaseContents {
    pub money: u64,
    pub items: [ItemPurchaseRefundItem; 5],
    pub currencies: [ItemPurchaseRefundCurrency; 5],
}

impl ItemPurchaseContents {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.money);
        for item in &self.items {
            item.write(pkt);
        }
        for currency in &self.currencies {
            currency.write(pkt);
        }
    }
}

/// SMSG_SET_ITEM_PURCHASE_DATA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetItemPurchaseData {
    pub item_guid: ObjectGuid,
    pub contents: ItemPurchaseContents,
    pub flags: u32,
    pub purchase_time: u32,
}

impl ServerPacket for SetItemPurchaseData {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetItemPurchaseData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.item_guid);
        self.contents.write(pkt);
        pkt.write_uint32(self.flags);
        pkt.write_uint32(self.purchase_time);
    }
}

/// CMSG_ITEM_PURCHASE_REFUND.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPurchaseRefund {
    pub item_guid: ObjectGuid,
}

impl ClientPacket for ItemPurchaseRefund {
    const OPCODE: ClientOpcodes = ClientOpcodes::ItemPurchaseRefund;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            item_guid: packet.read_packed_guid()?,
        })
    }
}

/// SMSG_ITEM_PURCHASE_REFUND_RESULT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPurchaseRefundResult {
    pub item_guid: ObjectGuid,
    pub result: u8,
    pub contents: Option<ItemPurchaseContents>,
}

impl ServerPacket for ItemPurchaseRefundResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::ItemPurchaseRefundResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.item_guid);
        pkt.write_uint8(self.result);
        pkt.write_bit(self.contents.is_some());
        pkt.flush_bits();
        if let Some(contents) = &self.contents {
            contents.write(pkt);
        }
    }
}

/// SMSG_ITEM_EXPIRE_PURCHASE_REFUND.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemExpirePurchaseRefund {
    pub item_guid: ObjectGuid,
}

impl ServerPacket for ItemExpirePurchaseRefund {
    const OPCODE: ServerOpcodes = ServerOpcodes::ItemExpirePurchaseRefund;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.item_guid);
    }
}

// ── InvUpdate (bit-packed item position list) ──────────────────────

/// Shared structure for client inventory packets.
///
/// Wire format (C# InvUpdate):
/// - [2 bits] count (0-3 items)
/// - [flush to byte boundary]
/// - For each item: [u8 container_slot] [u8 slot]
#[derive(Debug, Clone)]
pub struct InvUpdate {
    pub items: Vec<(u8, u8)>, // (container_slot, slot)
}

impl InvUpdate {
    /// Read an InvUpdate from a packet (matches C# InvUpdate constructor).
    pub fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = packet.read_bits(2)? as usize;
        // ResetBitPos happens automatically when read_uint8 is called
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let container_slot = packet.read_uint8()?;
            let slot = packet.read_uint8()?;
            items.push((container_slot, slot));
        }
        Ok(Self { items })
    }
}

// ── Client packets ──────────────────────────────────────────────────

/// CMSG_SWAP_INV_ITEM: Drag item between two inventory slots.
///
/// Wire format: InvUpdate + DestinationSlot(u8) + SourceSlot(u8).
/// Both slots are absolute InvSlot indices (0-140).
pub struct SwapInvItem {
    pub inv_update: InvUpdate,
    pub dst_slot: u8,
    pub src_slot: u8,
}

impl ClientPacket for SwapInvItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SwapInvItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(packet)?;
        let dst_slot = packet.read_uint8()?;
        let src_slot = packet.read_uint8()?;
        Ok(Self {
            inv_update,
            dst_slot,
            src_slot,
        })
    }
}

/// CMSG_AUTO_EQUIP_ITEM: Right-click to auto-equip an item.
///
/// Wire format: InvUpdate + PackSlot(u8) + Slot(u8).
/// `pack_slot` = 255 means default inventory (equipment + backpack).
/// `slot` = absolute slot index within the container.
pub struct AutoEquipItem {
    pub inv_update: InvUpdate,
    pub pack_slot: u8,
    pub slot: u8,
}

impl ClientPacket for AutoEquipItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoEquipItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(packet)?;
        let pack_slot = packet.read_uint8()?;
        let slot = packet.read_uint8()?;
        Ok(Self {
            inv_update,
            pack_slot,
            slot,
        })
    }
}

/// CMSG_AUTO_EQUIP_ITEM_SLOT: drag an item by GUID into an explicit equipment slot.
///
/// Wire format: InvUpdate + Item(ObjectGuid) + ItemDstSlot(u8).
pub struct AutoEquipItemSlot {
    pub inv_update: InvUpdate,
    pub item: ObjectGuid,
    pub item_dst_slot: u8,
}

impl ClientPacket for AutoEquipItemSlot {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoEquipItemSlot;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(packet)?;
        let item = packet.read_guid()?;
        let item_dst_slot = packet.read_uint8()?;
        Ok(Self {
            inv_update,
            item,
            item_dst_slot,
        })
    }
}

/// CMSG_SWAP_ITEM: Drag item between two container slots (container-aware).
///
/// Wire format: InvUpdate + ContainerSlotB(u8) + ContainerSlotA(u8) + SlotB(u8) + SlotA(u8).
/// ContainerSlot = 255 means player's direct inventory.
pub struct SwapItem {
    pub inv_update: InvUpdate,
    pub container_slot_b: u8,
    pub container_slot_a: u8,
    pub slot_b: u8,
    pub slot_a: u8,
}

impl ClientPacket for SwapItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SwapItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(packet)?;
        let container_slot_b = packet.read_uint8()?;
        let container_slot_a = packet.read_uint8()?;
        let slot_b = packet.read_uint8()?;
        let slot_a = packet.read_uint8()?;
        Ok(Self {
            inv_update,
            container_slot_b,
            container_slot_a,
            slot_b,
            slot_a,
        })
    }
}

/// CMSG_AUTO_STORE_BAG_ITEM: Right-click to store item in bag (unequip).
///
/// Wire format: InvUpdate + ContainerSlotA(u8) + ContainerSlotB(u8) + SlotA(u8).
/// ContainerSlotA = source container (255 for player inventory).
/// ContainerSlotB = destination container (255 for backpack).
/// SlotA = source slot within the container.
pub struct AutoStoreBagItem {
    pub inv_update: InvUpdate,
    pub container_slot_a: u8,
    pub container_slot_b: u8,
    pub slot_a: u8,
}

impl ClientPacket for AutoStoreBagItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoStoreBagItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(packet)?;
        let container_slot_a = packet.read_uint8()?;
        let container_slot_b = packet.read_uint8()?;
        let slot_a = packet.read_uint8()?;
        Ok(Self {
            inv_update,
            container_slot_a,
            container_slot_b,
            slot_a,
        })
    }
}

/// CMSG_DESTROY_ITEM: Delete an item from inventory.
///
/// Wire format: Count(i32) + ContainerId(u8) + SlotNum(u8).
/// No InvUpdate prefix on this one.
pub struct DestroyItemPkt {
    pub count: i32,
    pub container_id: u8,
    pub slot_num: u8,
}

impl ClientPacket for DestroyItemPkt {
    const OPCODE: ClientOpcodes = ClientOpcodes::DestroyItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = packet.read_int32()?;
        let container_id = packet.read_uint8()?;
        let slot_num = packet.read_uint8()?;
        Ok(Self {
            count,
            container_id,
            slot_num,
        })
    }
}

/// CMSG_CANCEL_TEMP_ENCHANTMENT.
///
/// C++ `WorldPackets::Item::CancelTempEnchantment::Read` reads one signed
/// equipment slot index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelTempEnchantment {
    pub slot: i32,
}

impl ClientPacket for CancelTempEnchantment {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelTempEnchantment;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            slot: packet.read_int32()?,
        })
    }
}

// ── Server packets ──────────────────────────────────────────────────

/// SMSG_INVENTORY_CHANGE_FAILURE: Sent on inventory operation failure.
pub struct InventoryChangeFailure {
    pub bag_result: InventoryResult,
    pub item: [ObjectGuid; 2],
    pub container_b_slot: u8,
    pub src_container: ObjectGuid,
    pub dst_container: ObjectGuid,
    pub src_slot: i32,
    pub level: u32,
    pub limit_category: u32,
}

impl ServerPacket for InventoryChangeFailure {
    const OPCODE: ServerOpcodes = ServerOpcodes::InventoryChangeFailure;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.bag_result as i32);
        pkt.write_packed_guid(&self.item[0]);
        pkt.write_packed_guid(&self.item[1]);
        pkt.write_uint8(self.container_b_slot);
        match self.bag_result {
            InventoryResult::CantEquipLevelI | InventoryResult::PurchaseLevelTooLow => {
                pkt.write_uint32(self.level);
            }
            InventoryResult::EventAutoequipBindConfirm => {
                pkt.write_packed_guid(&self.src_container);
                pkt.write_int32(self.src_slot);
                pkt.write_packed_guid(&self.dst_container);
            }
            InventoryResult::ItemMaxLimitCategoryCountExceededIs
            | InventoryResult::ItemMaxLimitCategorySocketedExceededIs
            | InventoryResult::ItemMaxLimitCategoryEquippedExceededIs => {
                pkt.write_uint32(self.limit_category);
            }
            _ => {}
        }
    }
}

impl InventoryChangeFailure {
    /// Create a failure response with items context.
    pub fn new(result: InventoryResult, item1: ObjectGuid, item2: ObjectGuid) -> Self {
        Self {
            bag_result: result,
            item: [item1, item2],
            container_b_slot: 0,
            src_container: ObjectGuid::EMPTY,
            dst_container: ObjectGuid::EMPTY,
            src_slot: 0,
            level: 0,
            limit_category: 0,
        }
    }

    /// Create a simple error with no item context.
    pub fn error(result: InventoryResult) -> Self {
        Self::new(result, ObjectGuid::EMPTY, ObjectGuid::EMPTY)
    }

    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    pub fn with_limit_category(mut self, limit_category: u32) -> Self {
        self.limit_category = limit_category;
        self
    }

    pub fn with_bind_confirm_context(
        mut self,
        src_container: ObjectGuid,
        src_slot: i32,
        dst_container: ObjectGuid,
    ) -> Self {
        self.src_container = src_container;
        self.src_slot = src_slot;
        self.dst_container = dst_container;
        self
    }
}

// ── InventoryType → equipment slot mapping ──────────────────────────

/// Map an item's InventoryType to the target player slot.
///
/// Returns `None` for non-equippable types.
/// For dual-slot items (ring, trinket, 1H weapon), picks the first empty
/// slot or defaults to the primary slot. Bags use the equipped bag slots
/// (`INVENTORY_SLOT_BAG_START..END`, 30..34 in C++ Player.h).
pub fn equip_slot_for_inventory_type(
    inv_type: u8,
    occupied: &std::collections::HashMap<u8, ()>,
) -> Option<u8> {
    fn first_empty(occupied: &std::collections::HashMap<u8, ()>, slots: &[u8], fallback: u8) -> u8 {
        slots
            .iter()
            .copied()
            .find(|slot| !occupied.contains_key(slot))
            .unwrap_or(fallback)
    }

    match inv_type {
        1 => Some(0),                                             // Head
        2 => Some(1),                                             // Neck
        3 => Some(2),                                             // Shoulders
        4 => Some(3),                                             // Body (shirt)
        5 | 20 => Some(4),                                        // Chest / Robe
        6 => Some(5),                                             // Waist
        7 => Some(6),                                             // Legs
        8 => Some(7),                                             // Feet
        9 => Some(8),                                             // Wrists
        10 => Some(9),                                            // Hands
        11 => Some(first_empty(occupied, &[10, 11], 10)),         // Finger
        12 => Some(first_empty(occupied, &[12, 13], 12)),         // Trinket
        13 => Some(first_empty(occupied, &[15, 16], 15)),         // 1H Weapon
        14 => Some(16),                                           // Shield → OffHand
        15 | 25 | 26 | 28 => Some(17), // Ranged / Thrown / RangedRight (Wand) / Relic
        16 => Some(14),                // Cloak
        17 | 21 => Some(15),           // 2H Weapon / WeaponMainHand
        18 => Some(first_empty(occupied, &[30, 31, 32, 33], 30)), // Bag
        19 => Some(18),                // Tabard
        22 => Some(16),                // WeaponOffHand
        23 => Some(16),                // Holdable → OffHand
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_change_failure_serializes() {
        let pkt = InventoryChangeFailure::error(InventoryResult::ItemNotFound);
        let bytes = pkt.to_bytes();
        // opcode(2) + result(4) + guid1(2) + guid2(2) + containerBSlot(1) = 11 bytes
        assert!(bytes.len() >= 11, "Packet too small: {} bytes", bytes.len());
    }

    #[test]
    fn inventory_change_failure_serializes_level_like_cpp_send_equip_error() {
        let pkt = InventoryChangeFailure::error(InventoryResult::CantEquipLevelI).with_level(42);
        let bytes = pkt.to_bytes();

        assert_eq!(&bytes[bytes.len() - 4..], &42u32.to_le_bytes());
    }

    #[test]
    fn inventory_change_failure_serializes_limit_category_like_cpp_send_equip_error() {
        let pkt =
            InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
                .with_limit_category(777);
        let bytes = pkt.to_bytes();

        assert_eq!(&bytes[bytes.len() - 4..], &777u32.to_le_bytes());
    }

    #[test]
    fn inventory_change_failure_serializes_bind_confirm_context_like_cpp() {
        let pkt = InventoryChangeFailure::error(InventoryResult::EventAutoequipBindConfirm)
            .with_bind_confirm_context(ObjectGuid::new(0, 0x0102), 37, ObjectGuid::new(0, 0x0506));
        let bytes = pkt.to_bytes();

        assert_eq!(
            &bytes[bytes.len() - 12..],
            &[0x03, 0x00, 0x02, 0x01, 37, 0, 0, 0, 0x03, 0x00, 0x06, 0x05,]
        );
    }

    #[test]
    fn swap_inv_item_parses() {
        // Real packet from WoW 3.4.3 client:
        // InvUpdate: 2 bits = 2 (0x80 = 10_000000), flush, then 2x(container, slot)
        // followed by dst_slot=40 src_slot=36
        let mut pkt = WorldPacket::from_bytes(&[
            SwapInvItem::OPCODE as u8,
            (SwapInvItem::OPCODE as u16 >> 8) as u8,
            0x80, // 2 bits: count=2, rest padding
            0xFF,
            0x28, // item[0]: container=255, slot=40
            0xFF,
            0x24, // item[1]: container=255, slot=36
            0x28, // dst_slot=40
            0x24, // src_slot=36
        ]);
        pkt.skip_opcode();
        let swap = SwapInvItem::read(&mut pkt).unwrap();
        assert_eq!(swap.inv_update.items.len(), 2);
        assert_eq!(swap.dst_slot, 40);
        assert_eq!(swap.src_slot, 36);
    }

    #[test]
    fn swap_inv_item_parses_zero_inv_update() {
        // InvUpdate with 0 items (bits 00 = 0x00)
        let mut pkt = WorldPacket::from_bytes(&[
            SwapInvItem::OPCODE as u8,
            (SwapInvItem::OPCODE as u16 >> 8) as u8,
            0x00, // 2 bits: count=0
            15,   // dst
            35,   // src
        ]);
        pkt.skip_opcode();
        let swap = SwapInvItem::read(&mut pkt).unwrap();
        assert_eq!(swap.inv_update.items.len(), 0);
        assert_eq!(swap.dst_slot, 15);
        assert_eq!(swap.src_slot, 35);
    }

    #[test]
    fn auto_equip_item_parses() {
        // InvUpdate with 0 items, then pack_slot=255 slot=35
        let mut pkt = WorldPacket::from_bytes(&[
            AutoEquipItem::OPCODE as u8,
            (AutoEquipItem::OPCODE as u16 >> 8) as u8,
            0x00, // 2 bits: count=0
            255,  // pack_slot (default backpack)
            35,   // slot
        ]);
        pkt.skip_opcode();
        let eq = AutoEquipItem::read(&mut pkt).unwrap();
        assert_eq!(eq.inv_update.items.len(), 0);
        assert_eq!(eq.pack_slot, 255);
        assert_eq!(eq.slot, 35);
    }

    #[test]
    fn auto_equip_item_slot_parses_cpp_shape() {
        let item = ObjectGuid::create_item(1, 55);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(1, 2);
        pkt.write_uint8(255);
        pkt.write_uint8(35);
        pkt.write_guid(&item);
        pkt.write_uint8(15);
        pkt.reset_read();

        let eq = AutoEquipItemSlot::read(&mut pkt).unwrap();

        assert_eq!(eq.inv_update.items, vec![(255, 35)]);
        assert_eq!(eq.item, item);
        assert_eq!(eq.item_dst_slot, 15);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn swap_item_parses() {
        // InvUpdate with 0 items, then containerB=255 containerA=255 slotB=15 slotA=35
        let mut pkt = WorldPacket::from_bytes(&[
            SwapItem::OPCODE as u8,
            (SwapItem::OPCODE as u16 >> 8) as u8,
            0x00, // 2 bits: count=0
            255,  // containerSlotB
            255,  // containerSlotA
            15,   // slotB
            35,   // slotA
        ]);
        pkt.skip_opcode();
        let swap = SwapItem::read(&mut pkt).unwrap();
        assert_eq!(swap.inv_update.items.len(), 0);
        assert_eq!(swap.container_slot_b, 255);
        assert_eq!(swap.container_slot_a, 255);
        assert_eq!(swap.slot_b, 15);
        assert_eq!(swap.slot_a, 35);
    }

    #[test]
    fn auto_store_bag_item_parses() {
        // InvUpdate with 0 items, then containerA=255 containerB=255 slotA=5
        let mut pkt = WorldPacket::from_bytes(&[
            AutoStoreBagItem::OPCODE as u8,
            (AutoStoreBagItem::OPCODE as u16 >> 8) as u8,
            0x00, // 2 bits: count=0
            255,  // containerSlotA
            255,  // containerSlotB
            5,    // slotA
        ]);
        pkt.skip_opcode();
        let store = AutoStoreBagItem::read(&mut pkt).unwrap();
        assert_eq!(store.inv_update.items.len(), 0);
        assert_eq!(store.container_slot_a, 255);
        assert_eq!(store.container_slot_b, 255);
        assert_eq!(store.slot_a, 5);
    }

    #[test]
    fn destroy_item_parses() {
        let mut pkt = WorldPacket::from_bytes(&[
            DestroyItemPkt::OPCODE as u8,
            (DestroyItemPkt::OPCODE as u16 >> 8) as u8,
            1,
            0,
            0,
            0,   // count=1
            255, // containerId
            35,  // slotNum
        ]);
        pkt.skip_opcode();
        let destroy = DestroyItemPkt::read(&mut pkt).unwrap();
        assert_eq!(destroy.count, 1);
        assert_eq!(destroy.container_id, 255);
        assert_eq!(destroy.slot_num, 35);
    }

    #[test]
    fn cancel_temp_enchantment_reads_cpp_slot() {
        let mut pkt = WorldPacket::from_bytes(&[
            CancelTempEnchantment::OPCODE as u8,
            (CancelTempEnchantment::OPCODE as u16 >> 8) as u8,
            15,
            0,
            0,
            0,
        ]);
        pkt.skip_opcode();
        let cancel = CancelTempEnchantment::read(&mut pkt).unwrap();
        assert_eq!(cancel.slot, 15);
    }

    #[test]
    fn item_instance_writes_cpp_field_order() {
        let instance = ItemInstance {
            item_id: 0x1122_3344,
            random_properties_seed: -2,
            random_properties_id: 3,
            item_bonus: None,
            modifications: ItemModList::default(),
        };
        let mut pkt = WorldPacket::new_empty();
        instance.write(&mut pkt);

        assert_eq!(
            pkt.data(),
            &[
                0x44, 0x33, 0x22, 0x11, 0xFE, 0xFF, 0xFF, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn item_instance_writes_modifications_before_bonus_like_cpp() {
        let instance = ItemInstance {
            item_id: 10,
            random_properties_seed: 20,
            random_properties_id: -30,
            item_bonus: Some(ItemBonuses {
                context: 4,
                bonus_list_ids: vec![100, 200],
            }),
            modifications: ItemModList {
                values: vec![ItemMod::new(-5, 3), ItemMod::new(7, 4)],
            },
        };
        let mut pkt = WorldPacket::new_empty();
        instance.write(&mut pkt);

        assert_eq!(
            pkt.data(),
            &[
                10, 0, 0, 0, 20, 0, 0, 0, 0xE2, 0xFF, 0xFF, 0xFF, 0x80, 0x08, 0xFB, 0xFF, 0xFF,
                0xFF, 3, 7, 0, 0, 0, 4, 4, 2, 0, 0, 0, 100, 0, 0, 0, 200, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn item_push_result_writes_cpp_order_and_bits() {
        let packet = ItemPushResult {
            player_guid: ObjectGuid::new(0, 0x0102),
            slot: 4,
            slot_in_bag: -1,
            item: ItemInstance {
                item_id: 9001,
                random_properties_seed: 12,
                random_properties_id: -77,
                item_bonus: None,
                modifications: ItemModList::default(),
            },
            quest_log_item_id: 777,
            quantity: 3,
            quantity_in_inventory: 9,
            dungeon_encounter_id: 615,
            battle_pet_species_id: 123,
            battle_pet_breed_id: 188,
            battle_pet_breed_quality: 26,
            battle_pet_level: 25,
            item_guid: ObjectGuid::new(0, 0x0506),
            pushed: true,
            display_text: ItemPushResultDisplayType::EncounterLoot,
            created: false,
            is_bonus_roll: false,
            is_encounter_loot: true,
        };
        let mut pkt = WorldPacket::new_empty();
        packet.write(&mut pkt);

        assert_eq!(
            pkt.data(),
            &[
                0x03, 0x00, 0x02, 0x01, 4, 0xFF, 0xFF, 0xFF, 0xFF, 0x09, 0x03, 0x00, 0x00, 3, 0, 0,
                0, 9, 0, 0, 0, 0x67, 0x02, 0x00, 0x00, 123, 0, 0, 0, 188, 0, 0, 0, 26, 0, 0, 0, 25,
                0, 0, 0, 0x03, 0x00, 0x06, 0x05, 0x92, 0x29, 0x23, 0x00, 0x00, 12, 0, 0, 0, 0xB3,
                0xFF, 0xFF, 0xFF, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn item_time_update_writes_cpp_order() {
        let packet = ItemTimeUpdate {
            item_guid: ObjectGuid::new(0, 0x0102),
            duration_left: 300,
        };
        let mut pkt = WorldPacket::new_empty();
        packet.write(&mut pkt);

        assert_eq!(
            pkt.data(),
            &[0x03, 0x00, 0x02, 0x01, 0x2C, 0x01, 0x00, 0x00,]
        );
    }

    #[test]
    fn item_enchant_time_update_writes_cpp_order() {
        let packet = ItemEnchantTimeUpdate {
            owner_guid: ObjectGuid::new(0, 0x0102),
            item_guid: ObjectGuid::new(0, 0x0506),
            duration_left: 45,
            slot: 2,
        };
        let mut pkt = WorldPacket::new_empty();
        packet.write(&mut pkt);

        assert_eq!(
            pkt.data(),
            &[
                0x03, 0x00, 0x06, 0x05, 45, 0, 0, 0, 2, 0, 0, 0, 0x03, 0x00, 0x02, 0x01,
            ]
        );
    }

    #[test]
    fn item_purchase_data_client_packets_read_cpp_guid() {
        let guid = ObjectGuid::new(0, 0x0102);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        assert_eq!(
            GetItemPurchaseData::read(&mut pkt).unwrap(),
            GetItemPurchaseData { item_guid: guid }
        );

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        assert_eq!(
            ItemPurchaseRefund::read(&mut pkt).unwrap(),
            ItemPurchaseRefund { item_guid: guid }
        );
    }

    #[test]
    fn set_item_purchase_data_writes_cpp_order() {
        let mut contents = ItemPurchaseContents {
            money: 0x0807_0605_0403_0201,
            ..Default::default()
        };
        contents.items[0] = ItemPurchaseRefundItem {
            item_id: 11,
            item_count: 2,
        };
        contents.currencies[0] = ItemPurchaseRefundCurrency {
            currency_id: 390,
            currency_count: 5,
        };
        let packet = SetItemPurchaseData {
            item_guid: ObjectGuid::new(0, 0x0102),
            contents,
            flags: 0xAABB_CCDD,
            purchase_time: 0x1122_3344,
        };

        let mut pkt = WorldPacket::new_empty();
        packet.write(&mut pkt);

        assert_eq!(pkt.read_packed_guid().unwrap(), packet.item_guid);
        assert_eq!(pkt.read_uint64().unwrap(), contents.money);
        assert_eq!(pkt.read_int32().unwrap(), 11);
        assert_eq!(pkt.read_int32().unwrap(), 2);
        for _ in 1..5 {
            assert_eq!(pkt.read_int32().unwrap(), 0);
            assert_eq!(pkt.read_int32().unwrap(), 0);
        }
        assert_eq!(pkt.read_int32().unwrap(), 390);
        assert_eq!(pkt.read_int32().unwrap(), 5);
        for _ in 1..5 {
            assert_eq!(pkt.read_int32().unwrap(), 0);
            assert_eq!(pkt.read_int32().unwrap(), 0);
        }
        assert_eq!(pkt.read_uint32().unwrap(), 0xAABB_CCDD);
        assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
    }

    #[test]
    fn item_purchase_refund_result_writes_optional_contents_bit() {
        let guid = ObjectGuid::new(0, 0x0102);
        let empty = ItemPurchaseRefundResult {
            item_guid: guid,
            result: 10,
            contents: None,
        };
        let mut pkt = WorldPacket::new_empty();
        empty.write(&mut pkt);
        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert_eq!(pkt.read_uint8().unwrap(), 10);
        assert!(!pkt.read_bit().unwrap());

        let ok = ItemPurchaseRefundResult {
            item_guid: guid,
            result: 0,
            contents: Some(ItemPurchaseContents {
                money: 7,
                ..Default::default()
            }),
        };
        let mut pkt = WorldPacket::new_empty();
        ok.write(&mut pkt);
        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert!(pkt.read_bit().unwrap());
        assert_eq!(pkt.read_uint64().unwrap(), 7);
    }

    #[test]
    fn item_expire_purchase_refund_writes_guid_only() {
        let guid = ObjectGuid::new(0, 0x0102);
        let mut pkt = WorldPacket::new_empty();
        ItemExpirePurchaseRefund { item_guid: guid }.write(&mut pkt);
        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert!(pkt.is_empty());
    }

    #[test]
    fn equip_slot_mapping() {
        let empty = std::collections::HashMap::new();
        assert_eq!(equip_slot_for_inventory_type(1, &empty), Some(0)); // Head
        assert_eq!(equip_slot_for_inventory_type(5, &empty), Some(4)); // Chest
        assert_eq!(equip_slot_for_inventory_type(16, &empty), Some(14)); // Cloak
        assert_eq!(equip_slot_for_inventory_type(17, &empty), Some(15)); // 2H Weapon
        assert_eq!(equip_slot_for_inventory_type(18, &empty), Some(30)); // Bag
        assert_eq!(equip_slot_for_inventory_type(0, &empty), None); // Non-equippable
    }

    #[test]
    fn equip_slot_for_bag_uses_cpp_bag_slot_range() {
        let occupied = std::collections::HashMap::from([(30, ()), (31, ()), (32, ())]);

        assert_eq!(equip_slot_for_inventory_type(18, &occupied), Some(33));
    }
}
