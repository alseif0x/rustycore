//! Equipment sets packets.
//!
//! Separated from character.rs under #689.

use super::*;

// ── WorldServerInfo (SMSG 0x25ad) ───────────────────────────────────

/// C++ `WorldPackets::EquipmentSet::LoadEquipmentSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadEquipmentSet {
    pub sets: Vec<EquipmentSetDataLikeCpp>,
}

impl ServerPacket for LoadEquipmentSet {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadEquipmentSet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sets.len() as u32);

        for set in &self.sets {
            pkt.write_int32(set.set_type);
            pkt.write_uint64(set.guid);
            pkt.write_uint32(set.set_id);
            pkt.write_uint32(set.ignore_mask);

            for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
                pkt.write_guid(&set.pieces[i]);
                pkt.write_int32(set.appearances[i]);
            }

            pkt.write_int32(set.enchants[0]);
            pkt.write_int32(set.enchants[1]);
            pkt.write_int32(set.secondary_shoulder_appearance_id);
            pkt.write_int32(set.secondary_shoulder_slot);
            pkt.write_int32(set.secondary_weapon_appearance_id);
            pkt.write_int32(set.secondary_weapon_slot);

            let has_spec_index = set.assigned_spec_index != -1;
            pkt.write_bit(has_spec_index);
            pkt.write_bits(set.set_name.len() as u32, 8);
            pkt.write_bits(set.set_icon.len() as u32, 9);

            if has_spec_index {
                pkt.write_int32(set.assigned_spec_index);
            }

            pkt.write_string(&set.set_name);
            pkt.write_string(&set.set_icon);
        }
    }
}

/// C++ `WorldPackets::EquipmentSet::EquipmentSetID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentSetId {
    pub guid: u64,
    pub set_type: i32,
    pub set_id: u32,
}

impl ServerPacket for EquipmentSetId {
    const OPCODE: ServerOpcodes = ServerOpcodes::EquipmentSetId;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_int32(self.set_type);
        pkt.write_uint32(self.set_id);
    }
}

// ── SaveEquipmentSet (CMSG 0x3509) ───────────────────────────────────

/// C++ `EquipmentSetInfo::EquipmentSetData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentSetDataLikeCpp {
    pub set_type: i32,
    pub guid: u64,
    pub set_id: u32,
    pub ignore_mask: u32,
    pub pieces: [ObjectGuid; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub enchants: [i32; 2],
    pub secondary_shoulder_appearance_id: i32,
    pub secondary_shoulder_slot: i32,
    pub secondary_weapon_appearance_id: i32,
    pub secondary_weapon_slot: i32,
    pub assigned_spec_index: i32,
    pub set_name: String,
    pub set_icon: String,
}

/// C++ `WorldPackets::EquipmentSet::SaveEquipmentSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEquipmentSet {
    pub set: EquipmentSetDataLikeCpp,
}

impl ClientPacket for SaveEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::SaveEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let set_type = pkt.read_int32()?;
        let guid = pkt.read_uint64()?;
        let set_id = pkt.read_uint32()?;
        let ignore_mask = pkt.read_uint32()?;

        let mut pieces = [ObjectGuid::EMPTY; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        let mut appearances = [0_i32; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            pieces[i] = pkt.read_guid()?;
            appearances[i] = pkt.read_int32()?;
        }

        let enchants = [pkt.read_int32()?, pkt.read_int32()?];
        let secondary_shoulder_appearance_id = pkt.read_int32()?;
        let secondary_shoulder_slot = pkt.read_int32()?;
        let secondary_weapon_appearance_id = pkt.read_int32()?;
        let secondary_weapon_slot = pkt.read_int32()?;

        let has_spec_index = pkt.read_bit()?;
        let set_name_len = pkt.read_bits(8)? as usize;
        let set_icon_len = pkt.read_bits(9)? as usize;
        let assigned_spec_index = if has_spec_index {
            pkt.read_int32()?
        } else {
            -1
        };

        let set_name = pkt.read_string(set_name_len)?;
        let set_icon = pkt.read_string(set_icon_len)?;

        Ok(Self {
            set: EquipmentSetDataLikeCpp {
                set_type,
                guid,
                set_id,
                ignore_mask,
                pieces,
                appearances,
                enchants,
                secondary_shoulder_appearance_id,
                secondary_shoulder_slot,
                secondary_weapon_appearance_id,
                secondary_weapon_slot,
                assigned_spec_index,
                set_name,
                set_icon,
            },
        })
    }
}

// ── AssignEquipmentSetSpec (CMSG 0x3207) ─────────────────────────────

/// C++ `WorldPackets::EquipmentSet::AssignEquipmentSetSpec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignEquipmentSetSpec {
    pub set_id: u32,
    pub spec_index: u32,
}

impl ClientPacket for AssignEquipmentSetSpec {
    const OPCODE: ClientOpcodes = ClientOpcodes::AssignEquipmentSetSpec;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            set_id: pkt.read_uint32()?,
            spec_index: pkt.read_uint32()?,
        })
    }
}

// ── DeleteEquipmentSet (CMSG 0x350a) ─────────────────────────────────

/// C++ `WorldPackets::EquipmentSet::DeleteEquipmentSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteEquipmentSet {
    pub id: u64,
}

impl ClientPacket for DeleteEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::DeleteEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            id: pkt.read_uint64()?,
        })
    }
}

// ── UseEquipmentSet (CMSG 0x3995 / SMSG 0x274f) ──────────────────────

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet::EquipmentSetItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetItemLikeCpp {
    pub item: ObjectGuid,
    pub container_slot: u8,
    pub slot: u8,
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet`.
#[derive(Debug, Clone)]
pub struct UseEquipmentSet {
    pub inv_update: InvUpdate,
    pub items: [UseEquipmentSetItemLikeCpp; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub guid: u64,
}

impl ClientPacket for UseEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(pkt)?;
        let mut items = [UseEquipmentSetItemLikeCpp {
            item: ObjectGuid::EMPTY,
            container_slot: 0,
            slot: 0,
        }; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for item in &mut items {
            item.item = pkt.read_guid()?;
            item.container_slot = pkt.read_uint8()?;
            item.slot = pkt.read_uint8()?;
        }
        let guid = pkt.read_uint64()?;

        Ok(Self {
            inv_update,
            items,
            guid,
        })
    }
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSetResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetResult {
    pub guid: u64,
    pub reason: u8,
}

impl ServerPacket for UseEquipmentSetResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::UseEquipmentSetResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_uint8(self.reason);
    }
}
