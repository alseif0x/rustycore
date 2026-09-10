//! Collections packets.
//!
//! Separated from character.rs under #689.

use super::*;

// ── AllAccountCriteria (SMSG 0x2571) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMount {
    pub spell_id: i32,
    pub flags: u8,
}

/// Account-wide mount collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMountUpdate {
    pub is_full_update: bool,
    pub mounts: Vec<AccountMount>,
}

impl AccountMountUpdate {
    pub fn full(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: true,
            mounts,
        }
    }

    pub fn partial(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: false,
            mounts,
        }
    }

    pub fn empty_full() -> Self {
        Self::full(Vec::new())
    }
}

impl ServerPacket for AccountMountUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountMountUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.write_int32(self.mounts.len() as i32);
        for mount in &self.mounts {
            pkt.write_int32(mount.spell_id);
            pkt.write_bits(u32::from(mount.flags & 0x0f), 4);
        }
        pkt.flush_bits();
    }
}

// ── MountResult (SMSG 0x257b) ───────────────────────────────────────

/// C++ `WorldPackets::Spells::MountResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountResult {
    pub result: i32,
}

impl ServerPacket for MountResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::MountResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.result);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpecial {
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ClientPacket for MountSpecial {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSpecialAnim;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let count = pkt.read_uint32()? as usize;
        let sequence_variation = pkt.read_int32()?;
        let mut spell_visual_kit_ids = Vec::with_capacity(count);
        for _ in 0..count {
            spell_visual_kit_ids.push(pkt.read_int32()?);
        }
        Ok(Self {
            spell_visual_kit_ids,
            sequence_variation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialMountAnim {
    pub unit_guid: ObjectGuid,
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ServerPacket for SpecialMountAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpecialMountAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.spell_visual_kit_ids.len() as u32);
        pkt.write_int32(self.sequence_variation);
        for spell_visual_kit_id in &self.spell_visual_kit_ids {
            pkt.write_int32(*spell_visual_kit_id);
        }
    }
}

// ── AccountHeirloomUpdate (SMSG 0xBADD placeholder) ─────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloom {
    pub item_id: i32,
    pub flags: u32,
}

/// C++ `WorldPackets::Misc::AccountHeirloomUpdate`.
///
/// The archived C++ opcode table uses the shared `0xBADD` placeholder for this
/// packet, so Rust reuses the existing `UpdateCapturePoint` discriminant while
/// keeping a distinct packet type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountHeirloomUpdate {
    pub is_full_update: bool,
    pub unk: i32,
    pub heirlooms: Vec<AccountHeirloom>,
}

impl AccountHeirloomUpdate {
    pub fn full(heirlooms: Vec<AccountHeirloom>) -> Self {
        Self {
            is_full_update: true,
            unk: 0,
            heirlooms,
        }
    }
}

impl ServerPacket for AccountHeirloomUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.unk);
        pkt.write_uint32(self.heirlooms.len() as u32);
        pkt.write_uint32(self.heirlooms.len() as u32);
        for heirloom in &self.heirlooms {
            pkt.write_int32(heirloom.item_id);
        }
        for heirloom in &self.heirlooms {
            pkt.write_uint32(heirloom.flags);
        }
    }
}

// ── MountSetFavorite (CMSG 0x3633) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountSetFavorite {
    pub mount_spell_id: u32,
    pub is_favorite: bool,
}

impl ClientPacket for MountSetFavorite {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSetFavorite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let mount_spell_id = pkt.read_uint32()?;
        let is_favorite = pkt.read_bit()?;
        Ok(Self {
            mount_spell_id,
            is_favorite,
        })
    }
}

// ── AccountToyUpdate (SMSG 0x25b0) ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToy {
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

/// Account-wide toy collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountToyUpdate {
    pub is_full_update: bool,
    pub toys: Vec<AccountToy>,
}

impl AccountToyUpdate {
    pub fn full(toys: Vec<AccountToy>) -> Self {
        Self {
            is_full_update: true,
            toys,
        }
    }
}

impl ServerPacket for AccountToyUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountToyUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        for toy in &self.toys {
            pkt.write_uint32(toy.item_id);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.is_favorite);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.has_fanfare);
        }
        pkt.flush_bits();
    }
}

// ── AddToy (CMSG 0x3299) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddToy {
    pub item_guid: ObjectGuid,
}

impl ClientPacket for AddToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── ToyClearFanfare (CMSG 0x3128) ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyClearFanfare {
    pub item_id: u32,
}

impl ClientPacket for ToyClearFanfare {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToyClearFanfare;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_id: pkt.read_uint32()?,
        })
    }
}

// ── UseToy (CMSG 0x329a) ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct UseToy {
    pub cast: CastSpellRequest,
}

impl ClientPacket for UseToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            cast: CastSpellRequest::read(pkt)?,
        })
    }
}
