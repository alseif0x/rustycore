//! Battle pet journal packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

// ── AccountMountUpdate (SMSG 0x25ae) ─────────────────────────────────

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournal;

impl ClientPacket for BattlePetRequestJournal {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournal;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournalLock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournalLock;

impl ClientPacket for BattlePetRequestJournalLock {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournalLock;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalSlot {
    pub pet_guid: ObjectGuid,
    pub collar_id: u32,
    pub index: u8,
    pub locked: bool,
}

impl BattlePetJournalSlot {
    pub fn locked_empty(index: u8) -> Self {
        Self {
            pet_guid: empty_battle_pet_guid_like_cpp(),
            collar_id: 0,
            index,
            locked: true,
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetOwnerInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalPetOwnerInfo {
    pub guid: ObjectGuid,
    pub player_virtual_realm: u32,
    pub player_native_realm: u32,
}

/// C++ `WorldPackets::BattlePet::BattlePet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournalPet {
    pub guid: ObjectGuid,
    pub species: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub breed: u16,
    pub level: u16,
    pub exp: u16,
    pub flags: u16,
    pub power: u32,
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub quality: u8,
    pub owner_info: Option<BattlePetJournalPetOwnerInfo>,
    pub name: String,
}

impl BattlePetJournalPet {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.species);
        pkt.write_uint32(self.creature_id);
        pkt.write_uint32(self.display_id);
        pkt.write_uint16(self.breed);
        pkt.write_uint16(self.level);
        pkt.write_uint16(self.exp);
        pkt.write_uint16(self.flags);
        pkt.write_uint32(self.power);
        pkt.write_uint32(self.health);
        pkt.write_uint32(self.max_health);
        pkt.write_uint32(self.speed);
        pkt.write_uint8(self.quality);
        pkt.write_bits(self.name.len() as u32, 7);
        pkt.write_bit(self.owner_info.is_some());
        pkt.write_bit(false); // NoRename
        pkt.flush_bits();
        pkt.write_string(&self.name);

        if let Some(owner_info) = self.owner_info {
            pkt.write_packed_guid(&owner_info.guid);
            pkt.write_uint32(owner_info.player_virtual_realm);
            pkt.write_uint32(owner_info.player_native_realm);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetJournal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournal {
    pub trap: u16,
    pub has_journal_lock: bool,
    pub slots: Vec<BattlePetJournalSlot>,
    pub pets: Vec<BattlePetJournalPet>,
}

impl BattlePetJournal {
    pub fn empty_with_default_slots(has_journal_lock: bool) -> Self {
        Self {
            trap: 0,
            has_journal_lock,
            slots: (0..3).map(BattlePetJournalSlot::locked_empty).collect(),
            pets: Vec::new(),
        }
    }
}

impl ServerPacket for BattlePetJournal {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournal;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint16(self.trap);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.has_journal_lock);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetUpdates {
    pub pets: Vec<BattlePetJournalPet>,
    pub pet_added: bool,
}

impl ServerPacket for BattlePetUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.pet_added);
        pkt.flush_bits();

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::PetBattleSlotUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetBattleSlotUpdates {
    pub slots: Vec<BattlePetJournalSlot>,
    pub auto_slotted: bool,
    pub new_slot: bool,
}

impl ServerPacket for PetBattleSlotUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::PetBattleSlotUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_bit(self.new_slot);
        pkt.write_bit(self.auto_slotted);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }
    }
}

/// Tells the client that the battle pet journal lock has been acquired.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockAcquired;

impl ServerPacket for BattlePetJournalLockAcquired {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockAcquired;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

/// Tells the client that the battle pet journal lock was denied.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockDenied;

impl ServerPacket for BattlePetJournalLockDenied {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockDenied;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

// ── TransferAborted (SMSG 0x2703) ───────────────────────────────────

/// C++ `WorldPackets::NPC::RequestStabledPets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestStabledPets {
    pub stable_master: ObjectGuid,
}

impl ClientPacket for RequestStabledPets {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RequestStabledPets;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            stable_master: pkt.read_packed_guid()?,
        })
    }
}
