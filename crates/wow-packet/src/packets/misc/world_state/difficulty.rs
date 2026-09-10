//! Difficulty packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

/// C++ `WorldPackets::Misc::SetDungeonDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDungeonDifficulty {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDungeonDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDungeonDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetRaidDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRaidDifficulty {
    pub difficulty_id: i32,
    pub legacy: u8,
}

impl ClientPacket for SetRaidDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetRaidDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_int32()?,
            legacy: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetDifficultyId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDifficultyId {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDifficultyId {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDifficultyId;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Null` for `CMSG_TOGGLE_DIFFICULTY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleDifficulty;

impl ClientPacket for ToggleDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToggleDifficulty;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── DungeonDifficultySet (SMSG 0x26a4) ───────────────────────────────

/// Sets the current dungeon difficulty. Sent BEFORE LoginVerifyWorld.
/// C# sends this via `Player.SendDungeonDifficulty()` during HandlePlayerLogin.
pub struct DungeonDifficultySet {
    pub difficulty_id: i32,
}

impl DungeonDifficultySet {
    /// Normal dungeon difficulty (default for fresh characters).
    pub fn normal() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for DungeonDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetDungeonDifficulty;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
    }
}

// ── RaidDifficultySet (SMSG 0x27ad) ──────────────────────────────────

/// Sets the current raid difficulty.
///
/// C++ `WorldPackets::Misc::RaidDifficultySet::Write`:
/// `int32 DifficultyID` followed by `uint8 Legacy`.
pub struct RaidDifficultySet {
    pub difficulty_id: i32,
    pub legacy: bool,
}

impl ServerPacket for RaidDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::RaidDifficultySet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_uint8(u8::from(self.legacy));
    }
}

/// C++ `WorldPackets::Calendar::SetSavedInstanceExtend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetSavedInstanceExtend {
    pub map_id: i32,
    pub difficulty_id: u32,
    pub extend: bool,
}

impl ClientPacket for SetSavedInstanceExtend {
    // The inspected TrinityCore 3.4.3 opcode table uses the shared unresolved
    // `0xBADD` placeholder for `CMSG_SET_SAVED_INSTANCE_EXTEND`,
    // `CMSG_SET_LOOT_SPECIALIZATION`, and `CMSG_CLEAR_RAID_MARKER`. Rust cannot
    // represent duplicate enum discriminants, so this parser is routed from the
    // existing 0xBADD opcode slot by payload shape in `WorldSession`.
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            map_id: pkt.read_int32()?,
            difficulty_id: pkt.read_uint32()?,
            extend: pkt.read_bit()?,
        })
    }
}
