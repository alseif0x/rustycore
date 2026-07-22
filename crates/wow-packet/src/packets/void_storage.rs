// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage packet codecs.
//!
//! C++ source of truth:
//! `src/server/game/Server/Packets/VoidStoragePackets.{h,cpp}`.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket, packets::item::ItemInstance};

pub const VOID_STORAGE_MAX_DEPOSIT_LIKE_CPP: usize = 9;
pub const VOID_STORAGE_MAX_WITHDRAW_LIKE_CPP: usize = 9;
pub const VOID_STORAGE_MAX_SLOT_LIKE_CPP: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum VoidTransferErrorLikeCpp {
    NoError = 0,
    InternalError1 = 1,
    InternalError2 = 2,
    Full = 3,
    InternalError3 = 4,
    InternalError4 = 5,
    NotEnoughMoney = 6,
    InventoryFull = 7,
    ItemInvalid = 8,
    TransferUnknown = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoidTransferResult {
    pub result: VoidTransferErrorLikeCpp,
}

impl ServerPacket for VoidTransferResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::VoidTransferResult;

    fn write(&self, packet: &mut WorldPacket) {
        packet.write_int32(self.result as i32);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockVoidStorage {
    pub npc: ObjectGuid,
}

impl ClientPacket for UnlockVoidStorage {
    const OPCODE: ClientOpcodes = ClientOpcodes::UnlockVoidStorage;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            npc: packet.read_packed_guid()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryVoidStorage {
    pub npc: ObjectGuid,
}

impl ClientPacket for QueryVoidStorage {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryVoidStorage;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            npc: packet.read_packed_guid()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoidStorageFailed {
    pub reason: u8,
}

impl ServerPacket for VoidStorageFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::VoidStorageFailed;

    fn write(&self, packet: &mut WorldPacket) {
        packet.write_uint8(self.reason);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidItem {
    pub guid: ObjectGuid,
    pub creator: ObjectGuid,
    pub slot: u32,
    pub item: ItemInstance,
}

impl VoidItem {
    fn write(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid(&self.guid);
        packet.write_packed_guid(&self.creator);
        packet.write_uint32(self.slot);
        self.item.write(packet);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoidStorageContents {
    pub items: Vec<VoidItem>,
}

impl ServerPacket for VoidStorageContents {
    const OPCODE: ServerOpcodes = ServerOpcodes::VoidStorageContents;

    fn write(&self, packet: &mut WorldPacket) {
        debug_assert!(self.items.len() <= u8::MAX as usize);
        packet.write_bits(self.items.len() as u32, 8);
        packet.flush_bits();
        for item in &self.items {
            item.write(packet);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidStorageTransfer {
    pub withdrawals: Vec<ObjectGuid>,
    pub deposits: Vec<ObjectGuid>,
    pub npc: ObjectGuid,
}

impl ClientPacket for VoidStorageTransfer {
    const OPCODE: ClientOpcodes = ClientOpcodes::VoidStorageTransfer;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let npc = packet.read_packed_guid()?;
        let deposit_count = packet.read_uint32()? as usize;
        let withdrawal_count = packet.read_uint32()? as usize;
        if deposit_count > VOID_STORAGE_MAX_DEPOSIT_LIKE_CPP {
            return Err(PacketError::InvalidArrayCapacity {
                requested: deposit_count,
                max: VOID_STORAGE_MAX_DEPOSIT_LIKE_CPP,
            });
        }
        if withdrawal_count > VOID_STORAGE_MAX_WITHDRAW_LIKE_CPP {
            return Err(PacketError::InvalidArrayCapacity {
                requested: withdrawal_count,
                max: VOID_STORAGE_MAX_WITHDRAW_LIKE_CPP,
            });
        }

        let deposits = (0..deposit_count)
            .map(|_| packet.read_packed_guid())
            .collect::<Result<Vec<_>, _>>()?;
        let withdrawals = (0..withdrawal_count)
            .map(|_| packet.read_packed_guid())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            withdrawals,
            deposits,
            npc,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoidStorageTransferChanges {
    pub removed_items: Vec<ObjectGuid>,
    pub added_items: Vec<VoidItem>,
}

impl ServerPacket for VoidStorageTransferChanges {
    const OPCODE: ServerOpcodes = ServerOpcodes::VoidStorageTransferChanges;

    fn write(&self, packet: &mut WorldPacket) {
        debug_assert!(self.added_items.len() <= 0x0F);
        debug_assert!(self.removed_items.len() <= 0x0F);
        packet.write_bits(self.added_items.len() as u32, 4);
        packet.write_bits(self.removed_items.len() as u32, 4);
        packet.flush_bits();
        for item in &self.added_items {
            item.write(packet);
        }
        for item in &self.removed_items {
            packet.write_packed_guid(item);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapVoidItem {
    pub npc: ObjectGuid,
    pub void_item_guid: ObjectGuid,
    pub dst_slot: u32,
}

impl ClientPacket for SwapVoidItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SwapVoidItem;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            npc: packet.read_packed_guid()?,
            void_item_guid: packet.read_packed_guid()?,
            dst_slot: packet.read_uint32()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoidItemSwapResponse {
    pub void_item_a: ObjectGuid,
    pub void_item_b: ObjectGuid,
    pub void_item_slot_a: u32,
    pub void_item_slot_b: u32,
}

impl ServerPacket for VoidItemSwapResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::VoidItemSwapResponse;

    fn write(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid(&self.void_item_a);
        packet.write_uint32(self.void_item_slot_a);
        packet.write_packed_guid(&self.void_item_b);
        packet.write_uint32(self.void_item_slot_b);
    }
}

#[cfg(test)]
mod tests {
    use wow_constants::{ItemModifier, ServerOpcodes};

    use super::*;
    use crate::packets::item::{ItemMod, ItemModList};

    fn payload(bytes: &[u8]) -> &[u8] {
        &bytes[2..]
    }

    #[test]
    fn transfer_rejects_cpp_array_capacity_spoofing() {
        let npc =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, 1);
        let mut packet = WorldPacket::new_empty();
        packet.write_packed_guid(&npc);
        packet.write_uint32((VOID_STORAGE_MAX_DEPOSIT_LIKE_CPP + 1) as u32);
        packet.write_uint32(0);
        let bytes = packet.into_data();
        let mut packet = WorldPacket::from_bytes(&bytes);
        assert!(matches!(
            VoidStorageTransfer::read(&mut packet),
            Err(PacketError::InvalidArrayCapacity {
                requested: 10,
                max: 9
            })
        ));
    }

    #[test]
    fn contents_matches_cpp_field_and_bit_order() {
        let guid = ObjectGuid::create_item(1, 41);
        let creator = ObjectGuid::create_player(1, 7);
        let bytes = VoidStorageContents {
            items: vec![VoidItem {
                guid,
                creator,
                slot: 3,
                item: ItemInstance {
                    item_id: 19019,
                    modifications: ItemModList {
                        values: vec![ItemMod::new(80, ItemModifier::TimewalkerLevel as u8)],
                    },
                    ..Default::default()
                },
            }],
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::VoidStorageContents as u16
        );
        let body = payload(&bytes);
        assert_eq!(body[0], 1, "C++ writes the item count as eight bits");
        let mut expected_prefix = WorldPacket::new_empty();
        expected_prefix.write_packed_guid(&guid);
        expected_prefix.write_packed_guid(&creator);
        expected_prefix.write_uint32(3);
        expected_prefix.write_int32(19019);
        let expected_prefix = expected_prefix.into_data();
        assert_eq!(&body[1..1 + expected_prefix.len()], &expected_prefix);
    }

    #[test]
    fn transfer_changes_packs_added_then_removed_counts_and_payloads() {
        let removed = ObjectGuid::create_item(1, 90);
        let bytes = VoidStorageTransferChanges {
            removed_items: vec![removed],
            added_items: vec![VoidItem {
                guid: ObjectGuid::create_item(1, 91),
                creator: ObjectGuid::EMPTY,
                slot: 4,
                item: ItemInstance {
                    item_id: 25,
                    ..Default::default()
                },
            }],
        }
        .to_bytes();

        assert_eq!(payload(&bytes)[0], 0x11);
        let body = payload(&bytes);
        let mut packet = WorldPacket::from_bytes(&body[1..]);
        assert_eq!(
            packet.read_packed_guid().unwrap(),
            ObjectGuid::create_item(1, 91)
        );
        assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(packet.read_uint32().unwrap(), 4);
        assert_eq!(packet.read_int32().unwrap(), 25);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert!(!packet.read_bit().unwrap());
        packet.reset_bits();
        assert_eq!(packet.read_bits(6).unwrap(), 0);
        packet.reset_bits();
        assert_eq!(packet.read_packed_guid().unwrap(), removed);
    }
}
