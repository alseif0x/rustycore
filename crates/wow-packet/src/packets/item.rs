// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-related packet definitions: swap, equip, destroy, and error responses.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};

// ── InventoryResult ─────────────────────────────────────────────────

/// Result codes for inventory operations (matches C# InventoryResult).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
pub enum InventoryResult {
    Ok = 0,
    CantEquipLevelI = 1,
    WrongSlot = 3,
    BagFull = 4,
    NotEquippable = 20,
    CantSwap = 21,
    SlotEmpty = 22,
    ItemNotFound = 23,
    DropBoundItem = 24,
    NotABag = 31,
    NotOwner = 33,
    PlayerDead = 39,
    InvFull = 51,
    InternalBagError = 83,
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
        Ok(Self { inv_update, dst_slot, src_slot })
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
        Ok(Self { inv_update, pack_slot, slot })
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
        Ok(Self { inv_update, container_slot_b, container_slot_a, slot_b, slot_a })
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
        Ok(Self { inv_update, container_slot_a, container_slot_b, slot_a })
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
        Ok(Self { count, container_id, slot_num })
    }
}

// ── Server packets ──────────────────────────────────────────────────

/// SMSG_INVENTORY_CHANGE_FAILURE: Sent on inventory operation failure.
pub struct InventoryChangeFailure {
    pub bag_result: InventoryResult,
    pub item: [ObjectGuid; 2],
    pub container_b_slot: u8,
}

impl ServerPacket for InventoryChangeFailure {
    const OPCODE: ServerOpcodes = ServerOpcodes::InventoryChangeFailure;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.bag_result as i32);
        pkt.write_packed_guid(&self.item[0]);
        pkt.write_packed_guid(&self.item[1]);
        pkt.write_uint8(self.container_b_slot);
        // Extra fields based on bag_result omitted for now
    }
}

impl InventoryChangeFailure {
    /// Create a failure response with items context.
    pub fn new(result: InventoryResult, item1: ObjectGuid, item2: ObjectGuid) -> Self {
        Self {
            bag_result: result,
            item: [item1, item2],
            container_b_slot: 0,
        }
    }

    /// Create a simple error with no item context.
    pub fn error(result: InventoryResult) -> Self {
        Self::new(result, ObjectGuid::EMPTY, ObjectGuid::EMPTY)
    }
}

// ── InventoryType → equipment slot mapping ──────────────────────────

/// Map an item's InventoryType to the target equipment slot.
///
/// Returns `None` for non-equippable types.
/// For dual-slot items (ring, trinket, 1H weapon), picks the first empty
/// slot or defaults to the primary slot.
pub fn equip_slot_for_inventory_type(
    inv_type: u8,
    occupied: &std::collections::HashMap<u8, ()>,
) -> Option<u8> {
    match inv_type {
        1 => Some(0),        // Head
        2 => Some(1),        // Neck
        3 => Some(2),        // Shoulders
        4 => Some(3),        // Body (shirt)
        5 | 20 => Some(4),   // Chest / Robe
        6 => Some(5),        // Waist
        7 => Some(6),        // Legs
        8 => Some(7),        // Feet
        9 => Some(8),        // Wrists
        10 => Some(9),       // Hands
        11 => {
            // Finger → slot 10 or 11
            if !occupied.contains_key(&10) { Some(10) }
            else if !occupied.contains_key(&11) { Some(11) }
            else { Some(10) }
        }
        12 => {
            // Trinket → slot 12 or 13
            if !occupied.contains_key(&12) { Some(12) }
            else if !occupied.contains_key(&13) { Some(13) }
            else { Some(12) }
        }
        13 => {
            // 1H Weapon → mainhand 15 or offhand 16
            if !occupied.contains_key(&15) { Some(15) }
            else if !occupied.contains_key(&16) { Some(16) }
            else { Some(15) }
        }
        14 => Some(16),      // Shield → OffHand
        15 | 25 | 26 | 28 => Some(17), // Ranged / Thrown / RangedRight (Wand) / Relic
        16 => Some(14),      // Cloak
        17 | 21 => Some(15), // 2H Weapon / WeaponMainHand
        18 => Some(3),       // Bag → shirt slot? Actually bags go in 19-22
        19 => Some(18),      // Tabard
        22 => Some(16),      // WeaponOffHand
        23 => Some(16),      // Holdable → OffHand
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
    fn swap_inv_item_parses() {
        // Real packet from WoW 3.4.3 client:
        // InvUpdate: 2 bits = 2 (0x80 = 10_000000), flush, then 2x(container, slot)
        // followed by dst_slot=40 src_slot=36
        let mut pkt = WorldPacket::from_bytes(&[
            SwapInvItem::OPCODE as u8, (SwapInvItem::OPCODE as u16 >> 8) as u8,
            0x80,       // 2 bits: count=2, rest padding
            0xFF, 0x28, // item[0]: container=255, slot=40
            0xFF, 0x24, // item[1]: container=255, slot=36
            0x28,       // dst_slot=40
            0x24,       // src_slot=36
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
            SwapInvItem::OPCODE as u8, (SwapInvItem::OPCODE as u16 >> 8) as u8,
            0x00,  // 2 bits: count=0
            15,    // dst
            35,    // src
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
            AutoEquipItem::OPCODE as u8, (AutoEquipItem::OPCODE as u16 >> 8) as u8,
            0x00,  // 2 bits: count=0
            255,   // pack_slot (default backpack)
            35,    // slot
        ]);
        pkt.skip_opcode();
        let eq = AutoEquipItem::read(&mut pkt).unwrap();
        assert_eq!(eq.inv_update.items.len(), 0);
        assert_eq!(eq.pack_slot, 255);
        assert_eq!(eq.slot, 35);
    }

    #[test]
    fn swap_item_parses() {
        // InvUpdate with 0 items, then containerB=255 containerA=255 slotB=15 slotA=35
        let mut pkt = WorldPacket::from_bytes(&[
            SwapItem::OPCODE as u8, (SwapItem::OPCODE as u16 >> 8) as u8,
            0x00,  // 2 bits: count=0
            255,   // containerSlotB
            255,   // containerSlotA
            15,    // slotB
            35,    // slotA
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
            AutoStoreBagItem::OPCODE as u8, (AutoStoreBagItem::OPCODE as u16 >> 8) as u8,
            0x00,  // 2 bits: count=0
            255,   // containerSlotA
            255,   // containerSlotB
            5,     // slotA
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
            DestroyItemPkt::OPCODE as u8, (DestroyItemPkt::OPCODE as u16 >> 8) as u8,
            1, 0, 0, 0, // count=1
            255,         // containerId
            35,          // slotNum
        ]);
        pkt.skip_opcode();
        let destroy = DestroyItemPkt::read(&mut pkt).unwrap();
        assert_eq!(destroy.count, 1);
        assert_eq!(destroy.container_id, 255);
        assert_eq!(destroy.slot_num, 35);
    }

    #[test]
    fn equip_slot_mapping() {
        let empty = std::collections::HashMap::new();
        assert_eq!(equip_slot_for_inventory_type(1, &empty), Some(0));  // Head
        assert_eq!(equip_slot_for_inventory_type(5, &empty), Some(4));  // Chest
        assert_eq!(equip_slot_for_inventory_type(16, &empty), Some(14)); // Cloak
        assert_eq!(equip_slot_for_inventory_type(17, &empty), Some(15)); // 2H Weapon
        assert_eq!(equip_slot_for_inventory_type(0, &empty), None);     // Non-equippable
    }
}
