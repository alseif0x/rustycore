// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet definitions.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── AttackSwing (CMSG_ATTACK_SWING) ──────────────────────────────

/// Client requests to start attacking a target.
#[derive(Debug, Clone)]
pub struct AttackSwing {
    pub victim: ObjectGuid,
}

impl ClientPacket for AttackSwing {
    const OPCODE: ClientOpcodes = ClientOpcodes::AttackSwing;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let victim = pkt.read_packed_guid()?;
        Ok(Self { victim })
    }
}

// ── AttackStop (CMSG_ATTACK_STOP) ────────────────────────────────

/// Client requests to stop attacking.
#[derive(Debug, Clone)]
pub struct AttackStop;

impl ClientPacket for AttackStop {
    const OPCODE: ClientOpcodes = ClientOpcodes::AttackStop;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── SetSheathed (CMSG_SET_SHEATHED) ──────────────────────────────

/// Client changes weapon sheathe state.
#[derive(Debug, Clone)]
pub struct SetSheathed {
    pub current_sheath_state: i32,
    pub sheathed: bool,
}

impl ClientPacket for SetSheathed {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetSheathed;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let current_sheath_state = pkt.read_int32()?;
        let sheathed = pkt.has_bit()?;
        Ok(Self { current_sheath_state, sheathed })
    }
}

// ── AttackStart (SMSG_ATTACK_START) ──────────────────────────────

/// Server notifies client that combat has started.
#[derive(Debug, Clone)]
pub struct AttackStart {
    pub attacker: ObjectGuid,
    pub victim: ObjectGuid,
}

impl ServerPacket for AttackStart {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackStart;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.attacker);
        pkt.write_packed_guid(&self.victim);
    }
}

// ── SAttackStop (SMSG_ATTACK_STOP) ───────────────────────────────

/// Server notifies client that combat has stopped.
#[derive(Debug, Clone)]
pub struct SAttackStop {
    pub attacker: ObjectGuid,
    pub victim: ObjectGuid,
    pub now_dead: bool,
}

impl ServerPacket for SAttackStop {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackStop;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.attacker);
        pkt.write_packed_guid(&self.victim);
        pkt.write_bit(self.now_dead);
        pkt.flush_bits();
    }
}

// ── AttackerStateUpdate (SMSG_ATTACKER_STATE_UPDATE) ─────────────

/// Sends melee hit result with damage info to the client.
///
/// This is what makes damage numbers appear on screen.
/// Simplified implementation: normal physical hit, no subdamage breakdown.
///
/// C# format: attackRoundInfo is written to a sub-buffer, then size+bytes appended.
#[derive(Debug, Clone)]
pub struct AttackerStateUpdate {
    pub attacker: ObjectGuid,
    pub victim: ObjectGuid,
    /// Total damage dealt.
    pub damage: i32,
    /// Overkill amount (-1 if target is still alive).
    pub over_damage: i32,
    /// Victim state: 0=none, 1=hit, 2=miss, 3=dodge, 4=parry, 5=interrupt,
    ///               6=blocks, 7=evades, 8=immune, 9=deflect, 10=absorb
    pub victim_state: u8,
    /// School mask for the hit: 1=physical, 2=holy, etc.
    pub school_mask: i32,
    /// ContentTuning data required by the client.
    pub target_level: u8,
    pub expansion: u8,
}

/// HitInfo flags (uint in C#).
const HIT_INFO_NORMAL_SWING: u32 = 0x0000_0002;
/// VictimState: normal hit.
pub const VICTIM_STATE_HIT: u8 = 1;

impl ServerPacket for AttackerStateUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackerStateUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        // Build the attackRoundInfo sub-buffer (C# does this to a separate WorldPacket)
        let mut info = WorldPacket::new_empty();
        info.write_uint32(HIT_INFO_NORMAL_SWING);
        info.write_packed_guid(&self.attacker);
        info.write_packed_guid(&self.victim);
        info.write_int32(self.damage);
        info.write_int32(self.damage); // original damage
        info.write_int32(self.over_damage); // over damage (-1 if alive)
        info.write_uint8(0u8); // no SubDmg
        info.write_uint8(self.victim_state);
        info.write_uint32(0u32); // attacker state
        info.write_uint32(0u32); // melee spell id

        // ContentTuning (10 fields as in C#)
        info.write_uint8(0u8); // tuning type = none
        info.write_uint8(self.target_level);
        info.write_uint8(self.expansion);
        info.write_int16(0i16); // player_level_delta
        info.write_int8(0i8);   // target_scaling_level_delta
        info.write_float(0.0f32); // player_item_level
        info.write_float(0.0f32); // target_item_level
        info.write_uint32(0u32); // scaling_health_item_level_curve_id
        info.write_uint32(0u32); // flags
        info.write_int32(0i32);  // player_content_tuning_id
        info.write_int32(0i32);  // target_content_tuning_id

        // WriteLogDataBit + FlushBits (CombatLogServerPacket base)
        info.write_bit(false); // has_log_data
        info.flush_bits();

        // The outer packet: u32 size + bytes
        let data = info.data().to_vec();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

// ── UnitHealthUpdate — VALUES packet wrapper ──────────────────────
// (These are sent via UpdateObject::unit_values_update, not a separate packet type)

// ── AttackSwingError (SMSG_ATTACK_SWING_ERROR) ───────────────────

#[derive(Debug, Clone)]
pub struct AttackSwingError {
    pub reason: u8,
}

impl ServerPacket for AttackSwingError {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackSwingError;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.reason as u32, 3);
        pkt.flush_bits();
    }
}
