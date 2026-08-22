// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement, transport and taxi packets.

use super::*;

/// C++ `WorldPackets::Misc::FarSight`: one bit toggling seer to current viewpoint/self.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FarSight {
    pub enable: bool,
}

impl ClientPacket for FarSight {
    const OPCODE: ClientOpcodes = ClientOpcodes::FarSight;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable: pkt.read_bit()?,
        })
    }
}

// ── Bank (CMSG 0x3997 / 0x3996 / 0x34B4) ──────────────────────────

/// C++ `WorldPackets::Misc::SetTaxiBenchmarkMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetTaxiBenchmarkMode {
    pub enable: bool,
}

impl ClientPacket for SetTaxiBenchmarkMode {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTaxiBenchmarkMode;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Taxi::ActivateTaxi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateTaxi {
    pub vendor: ObjectGuid,
    pub node: u32,
    pub ground_mount_id: u32,
    pub flying_mount_id: u32,
}

impl ClientPacket for ActivateTaxi {
    const OPCODE: ClientOpcodes = ClientOpcodes::ActivateTaxi;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            vendor: pkt.read_packed_guid()?,
            node: pkt.read_uint32()?,
            ground_mount_id: pkt.read_uint32()?,
            flying_mount_id: pkt.read_uint32()?,
        })
    }
}

pub struct ActivateTaxiReply {
    pub reply: u8,
}

impl ServerPacket for ActivateTaxiReply {
    const OPCODE: ServerOpcodes = ServerOpcodes::ActivateTaxiReply;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.reply), 4);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Misc::SetMovementAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetMovementAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetMovementAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetMovementAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

/// Set game time and speed at login.
pub struct LoginSetTimeSpeed {
    pub server_time: i32,
    pub game_time: i32,
    pub new_speed: f32,
    pub server_time_holiday_offset: i32,
    pub game_time_holiday_offset: i32,
}

impl LoginSetTimeSpeed {
    /// Current time with standard speed (1/24 = real-time game day).
    pub fn now() -> Self {
        let t = wow_core::GameTime::now().to_packed() as i32;
        Self {
            server_time: t,
            game_time: t,
            new_speed: 1.0 / 24.0,
            server_time_holiday_offset: 0,
            game_time_holiday_offset: 0,
        }
    }
}

impl ServerPacket for LoginSetTimeSpeed {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoginSetTimeSpeed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.server_time);
        pkt.write_int32(self.game_time);
        pkt.write_float(self.new_speed);
        pkt.write_int32(self.server_time_holiday_offset);
        pkt.write_int32(self.game_time_holiday_offset);
    }
}

// ── SetupCurrency (SMSG 0x2573) ─────────────────────────────────────

/// Tells the client which unit it controls for movement input.
///
/// **Critical**: Without this packet the client's `m_mover` pointer is null.
/// Any camera/movement processing will dereference null → ACCESS_VIOLATION.
///
/// C++ format: `ObjectGuid` through `operator<<`, which is the packed
/// low/high-mask GUID layout in TrinityCore 3.4.3.
pub struct MoveSetActiveMover {
    pub mover_guid: ObjectGuid,
}

impl ServerPacket for MoveSetActiveMover {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveSetActiveMover;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
    }
}

// ── SetSpellModifier (SMSG 0x2c33 / 0x2c34) ───────────────────────

/// C++ `WorldPackets::Battleground::AreaSpiritHealerQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerQuery {
    pub healer_guid: ObjectGuid,
}

impl ClientPacket for AreaSpiritHealerQuery {
    const OPCODE: wow_constants::ClientOpcodes =
        wow_constants::ClientOpcodes::AreaSpiritHealerQuery;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            healer_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AreaSpiritHealerQueue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerQueue {
    pub healer_guid: ObjectGuid,
}

impl ClientPacket for AreaSpiritHealerQueue {
    const OPCODE: wow_constants::ClientOpcodes =
        wow_constants::ClientOpcodes::AreaSpiritHealerQueue;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            healer_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AreaSpiritHealerTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerTime {
    pub healer_guid: ObjectGuid,
    pub time_left_ms: i32,
}

impl ServerPacket for AreaSpiritHealerTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::AreaSpiritHealerTime;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.healer_guid);
        pkt.write_int32(self.time_left_ms);
    }
}

// ── TaxiNodeStatusPkt (SMSG 0x267C) ─────────────────────────────────────────
/// Response to CMSG_TAXI_NODE_STATUS_QUERY.
/// C# ref: TaxiPackets.TaxiNodeStatusPkt
/// Status bits: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
pub struct TaxiNodeStatusPkt {
    pub unit_guid: wow_core::ObjectGuid,
    /// 2-bit field: 0=None 1=Learned 2=Unlearned 3=NotEligible
    pub status: u8,
}

impl ServerPacket for TaxiNodeStatusPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::TaxiNodeStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit_guid);
        pkt.write_bits(self.status as u32, 2);
        pkt.flush_bits();
    }
}
