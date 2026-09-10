//! Vendor packets.
//!
//! Separated from character.rs under #689.

use super::*;

// ── SuspendToken (SMSG 0x25a8) ───────────────────────────────────────

/// One item in the vendor's inventory list.
/// C++ `WorldPackets::NPC::VendorItem` (`Server/Packets/NPCPackets.cpp:132-150`).
#[derive(Debug, Clone)]
pub struct VendorItem {
    pub muid: i32, // slot/muid index
    pub item_id: i32,
    pub item_type: i32, // 1 = item, 2 = currency
    pub quantity: i32,  // max stack on vendor (-1 = unlimited)
    pub price: u64,     // buy price (copper)
    pub durability: i32,
    pub stack_count: i32, // VendorStackCount from item_sparse
    pub extended_cost: i32,
    pub player_condition_failed: i32,
    pub locked: bool,
    pub do_not_filter: bool,
    pub refundable: bool,
}

/// SMSG_VENDOR_INVENTORY — list of items a vendor is selling.
/// C++ `WorldPackets::NPC::VendorInventory::Write`.
pub struct VendorInventory {
    pub vendor_guid: ObjectGuid,
    pub reason: u8, // 0 = ok, non-0 = error (no items etc)
    pub items: Vec<VendorItem>,
}

impl ServerPacket for VendorInventory {
    const OPCODE: ServerOpcodes = ServerOpcodes::VendorInventory;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.items.len() as i32);

        for (i, item) in self.items.iter().enumerate() {
            pkt.write_uint64(item.price);
            pkt.write_int32(item.muid);
            pkt.write_int32(item.item_type);
            pkt.write_int32(item.durability);
            pkt.write_int32(item.stack_count);
            pkt.write_int32(item.quantity);
            pkt.write_int32(item.extended_cost);
            pkt.write_int32(item.player_condition_failed);
            // 3 bits: Locked, DoNotFilterOnVendor, Refundable
            pkt.write_bit(item.locked);
            pkt.write_bit(item.do_not_filter);
            pkt.write_bit(item.refundable);
            pkt.flush_bits();
            // ItemInstance inline:
            //   ItemID (i32), RandomPropertiesSeed (i32), RandomPropertiesID (i32)
            //   bit(ItemBonus != null) = false, FlushBits
            //   ItemModList: WriteBits(0, 6) + FlushBits  (no mods)
            pkt.write_int32(item.item_id);
            pkt.write_int32(0i32); // RandomPropertiesSeed
            pkt.write_int32(0i32); // RandomPropertiesID
            pkt.write_bit(false); // has ItemBonus = false
            pkt.flush_bits();
            pkt.write_bits(0u32, 6); // ItemModList count = 0
            pkt.flush_bits();
            // no ItemMod entries, no ItemBonus
            let _ = i; // suppress unused
        }
    }
}

/// CMSG_BUY_ITEM — client wants to buy an item from a vendor.
/// C++ `WorldPackets::Item::BuyItem::Read`.
#[derive(Debug)]
pub struct BuyItem {
    pub vendor_guid: ObjectGuid,
    pub container_guid: ObjectGuid,
    pub quantity: i32,
    pub muid: i32,
    pub slot: i32,
    pub item_type: i32,
    pub item_id: i32,
}

impl ClientPacket for BuyItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let container_guid = pkt.read_packed_guid()?;
        let quantity = pkt.read_int32()?;
        let muid = pkt.read_int32()?;
        let slot = pkt.read_int32()?;
        let item_type = pkt.read_int32()?;
        // ItemInstance.Read: ItemID, RandomPropertiesSeed, RandomPropertiesID, bit(hasBonus), FlushBits, ItemModList
        let item_id = pkt.read_int32()?;
        let _seed = pkt.read_int32()?;
        let _rand_prop = pkt.read_int32()?;
        let has_bonus = pkt.read_bit()?;
        pkt.reset_bits();
        let mod_count = pkt.read_bits(6)? as u32;
        for _ in 0..mod_count {
            let _val = pkt.read_int32()?;
            let _ty = pkt.read_uint8()?;
        }
        if has_bonus {
            // ItemBonuses: Context (u8) + BonusListIDs count + entries
            let _ctx = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bid = pkt.read_uint16()?;
            }
        }
        Ok(Self {
            vendor_guid,
            container_guid,
            quantity,
            muid,
            slot,
            item_type,
            item_id,
        })
    }
}

/// CMSG_BUY_BACK_ITEM — client buys back an item from a vendor buyback slot.
/// C++: WorldPackets::Item::BuyBackItem
#[derive(Debug)]
pub struct BuyBackItem {
    pub vendor_guid: ObjectGuid,
    pub slot: u32,
}

impl ClientPacket for BuyBackItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyBackItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let slot = pkt.read_uint32()?;
        Ok(Self { vendor_guid, slot })
    }
}

/// SMSG_BUY_SUCCEEDED — item bought successfully.
/// C++ `WorldPackets::Item::BuySucceeded::Write`.
pub struct BuySucceeded {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub new_quantity: i32,
    pub quantity_bought: i32,
}

impl ServerPacket for BuySucceeded {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuySucceeded;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_int32(self.new_quantity);
        pkt.write_int32(self.quantity_bought);
    }
}

/// SMSG_BUY_FAILED — buy failed with reason code.
/// C++ `WorldPackets::Item::BuyFailed::Write`.
pub struct BuyFailed {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub reason: BuyResult,
}

impl ServerPacket for BuyFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuyFailed;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_uint8(self.reason as u8);
    }
}

/// CMSG_SELL_ITEM — client wants to sell an item to a vendor.
/// C++ `WorldPackets::Item::SellItem::Read`.
#[derive(Debug)]
pub struct SellItem {
    pub vendor_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub amount: i32,
}

impl ClientPacket for SellItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::SellItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let amount = pkt.read_int32()?;
        Ok(Self {
            vendor_guid,
            item_guid,
            amount,
        })
    }
}

/// CMSG_REPAIR_ITEM — client repairs one item or all items at a repair NPC.
/// C++: WorldPackets::Item::RepairItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub npc_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub use_guild_bank: bool,
}

impl ClientPacket for RepairItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RepairItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let npc_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let use_guild_bank = pkt.read_bit()?;
        Ok(Self {
            npc_guid,
            item_guid,
            use_guild_bank,
        })
    }
}

/// SMSG_SELL_RESPONSE — result of a sell operation.
/// C++ `WorldPackets::Item::SellResponse::Write`
/// (`Server/Packets/ItemPackets.cpp:238-247`).
pub struct SellResponse {
    pub vendor_guid: ObjectGuid,
    pub item_guids: Vec<ObjectGuid>,
    pub reason: i32,
}

impl ServerPacket for SellResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::SellResponse;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint32(self.item_guids.len() as u32);
        pkt.write_int32(self.reason);
        for item_guid in &self.item_guids {
            pkt.write_packed_guid(item_guid);
        }
    }
}

impl SellResponse {
    pub fn error(vendor_guid: ObjectGuid, item_guid: ObjectGuid, reason: SellResult) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: reason as i32,
        }
    }

    pub fn success(vendor_guid: ObjectGuid, item_guid: ObjectGuid) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: 0,
        }
    }
}
