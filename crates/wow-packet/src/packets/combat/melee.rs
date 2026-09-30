use wow_constants::creature::AiReaction;
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
        Ok(Self {
            current_sheath_state,
            sheathed,
        })
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

/// C++ `WorldPackets::Combat::CancelCombat` (`SMSG_CANCEL_COMBAT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelCombat;

impl ServerPacket for CancelCombat {
    const OPCODE: ServerOpcodes = ServerOpcodes::CancelCombat;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

/// C++ `WorldPackets::Combat::BreakTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakTarget {
    pub unit_guid: ObjectGuid,
}

impl ServerPacket for BreakTarget {
    const OPCODE: ServerOpcodes = ServerOpcodes::BreakTarget;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit_guid);
    }
}

// ── AIReaction (SMSG_AI_REACTION) ────────────────────────────────

/// Server notifies visible clients of a creature AI reaction.
///
/// C++ anchor: `WorldPackets::Combat::AIReaction::Write` writes `UnitGUID`
/// followed by `Reaction`.
#[derive(Debug, Clone)]
pub struct AIReaction {
    pub unit_guid: ObjectGuid,
    pub reaction: AiReaction,
}

impl ServerPacket for AIReaction {
    const OPCODE: ServerOpcodes = ServerOpcodes::AiReaction;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit_guid);
        pkt.write_uint32(self.reaction as u32);
    }
}

// ── AttackerStateUpdate (SMSG_ATTACKER_STATE_UPDATE) ─────────────

/// Sends melee hit result with damage info to the client.
///
/// This is what makes damage numbers appear on screen.
/// Simplified implementation: normal physical hit, no subdamage breakdown.
///
/// C++ writes the combat-log bit on the outer packet, then size+attackRoundInfo.
#[derive(Debug, Clone)]
pub struct AttackerStateUpdate {
    pub attacker: ObjectGuid,
    pub victim: ObjectGuid,
    /// HitInfo flags written at the start of `attackRoundInfo`.
    pub hit_info: u32,
    /// Total damage dealt.
    pub damage: i32,
    /// C++ `CalcDamageInfo::OriginalDamage`: the post-armour damage before the
    /// outcome switch, serialized as the second `int32` of `attackRoundInfo`.
    pub original_damage: i32,
    /// Overkill amount (-1 if target is still alive).
    pub over_damage: i32,
    /// C++ `CalcDamageInfo::Blocked`, serialized only for `HITINFO_BLOCK`.
    pub blocked: i32,
    /// C++ `CalcDamageInfo::Absorb` (`SubDmg.Absorbed`), serialized only for
    /// `HITINFO_FULL_ABSORB | HITINFO_PARTIAL_ABSORB`.
    pub absorbed: i32,
    /// C++ `VictimState`: 0=intact (miss), 1=hit, 2=dodge, 3=parry,
    /// 4=interrupt, 5=blocks, 6=evades, 7=immune, 8=deflects.
    pub victim_state: u8,
    /// School mask for the hit: 1=physical, 2=holy, etc.
    pub school_mask: i32,
    /// ContentTuning data required by the client.
    pub target_level: u8,
    pub expansion: u8,
}

/// C++ `HitInfo` flags (`UnitDefines.h:440-465` in the 3.4.3 target).
///
/// `HITINFO_NORMALSWING` itself is `0x00000000`; a landed white swing carries
/// `HITINFO_AFFECTS_VICTIM`, which C++ adds to every non-miss outcome
/// (`Unit.cpp:1434-1436`).
pub const HIT_INFO_AFFECTS_VICTIM: u32 = 0x0000_0002;
/// C++ `HITINFO_OFFHAND`.
pub const HIT_INFO_OFFHAND: u32 = 0x0000_0004;
/// C++ `HITINFO_MISS`.
pub const HIT_INFO_MISS: u32 = 0x0000_0010;
/// C++ `HITINFO_CRITICALHIT`.
pub const HIT_INFO_CRITICAL_HIT: u32 = 0x0000_0200;
/// C++ `HITINFO_GLANCING`.
pub const HIT_INFO_GLANCING: u32 = 0x0001_0000;
/// C++ `HITINFO_SWINGNOHITSOUND`, set with a miss when the victim evades.
pub const HIT_INFO_SWING_NO_HIT_SOUND: u32 = 0x0020_0000;
/// C++ `HITINFO_BLOCK`: the packet then carries `blocked` and the trailing
/// `float Unk` C++ writes for `HITINFO_BLOCK | HITINFO_UNK12`.
pub const HIT_INFO_BLOCK: u32 = 0x0000_2000;
/// C++ `HITINFO_CRUSHING` (`UnitDefines.h:461`).
pub const HIT_INFO_CRUSHING: u32 = 0x0002_0000;
/// C++ `HITINFO_FULL_ABSORB`: `CalcAbsorbResist` consumed the whole hit, so the
/// packet carries a zero `Damage` and the `SubDmg.Absorbed` amount
/// (`Unit.cpp:1452-1460`, `CombatLogPackets.cpp:361-362`).
pub const HIT_INFO_FULL_ABSORB: u32 = 0x0000_0020;
/// C++ `HITINFO_PARTIAL_ABSORB`: a school absorb consumed part of the hit.
pub const HIT_INFO_PARTIAL_ABSORB: u32 = 0x0000_0040;
/// C++ `HITINFO_FAKE_DAMAGE`: enables a damage animation even if no damage is done.
pub const HIT_INFO_FAKE_DAMAGE: u32 = 0x0100_0000;
/// C++ `HITINFO_NORMALSWING`: the `0x0` flag the immune path ORs, so a main-hand
/// immune swing publishes a zero `HitInfo` (`UnitDefines.h:440-465`).
pub const HIT_INFO_NORMALSWING: u32 = 0x0;

/// C++ `VictimState` (`Unit.h:45-55` in the 3.4.3 target).
pub const VICTIM_STATE_INTACT: u8 = 0;
/// C++ `VICTIMSTATE_HIT`.
pub const VICTIM_STATE_HIT: u8 = 1;
/// C++ `VICTIMSTATE_DODGE`.
pub const VICTIM_STATE_DODGE: u8 = 2;
/// C++ `VICTIMSTATE_PARRY`.
pub const VICTIM_STATE_PARRY: u8 = 3;
/// C++ `VICTIMSTATE_EVADES`.
pub const VICTIM_STATE_EVADES: u8 = 6;
/// C++ `VICTIMSTATE_IS_IMMUNE`.
pub const VICTIM_STATE_IS_IMMUNE: u8 = 7;

impl ServerPacket for AttackerStateUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackerStateUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        // C++ builds attackRoundInfo in a separate ByteBuffer.
        let mut info = WorldPacket::new_empty();
        info.write_uint32(self.hit_info);
        info.write_packed_guid(&self.attacker);
        info.write_packed_guid(&self.victim);
        info.write_int32(self.damage);
        info.write_int32(self.original_damage);
        info.write_int32(self.over_damage); // over damage (-1 if alive)
        // C++ `Unit::SendAttackStateUpdate(CalcDamageInfo*)` always emplaces
        // `SubDmg` for a melee swing (`Unit.cpp:5473-5479`), and
        // `AttackerStateUpdate::Write` (`CombatLogPackets.cpp:355-365`)
        // serializes the presence byte, the school mask, the float and integer
        // damage and then the absorbed/resisted amounts when their hit-info
        // bits are set. Physical melee never resists: `Unit::CalcSpellResistedDamage`
        // returns zero for a non-magic school mask (`Unit.cpp:2058-2060`), so no
        // `HITINFO_*_RESIST` bit and no `Resisted` field is ever produced here.
        info.write_uint8(1u8); // SubDmg present
        info.write_int32(self.school_mask);
        info.write_float(self.damage as f32);
        info.write_int32(self.damage);
        if self.hit_info & (HIT_INFO_FULL_ABSORB | HIT_INFO_PARTIAL_ABSORB) != 0 {
            info.write_int32(self.absorbed);
        }
        info.write_uint8(self.victim_state);
        info.write_uint32(0u32); // attacker state
        info.write_uint32(0u32); // melee spell id
        if self.hit_info & HIT_INFO_BLOCK != 0 {
            // C++ `AttackerStateUpdate::Write` (`CombatLogPackets.cpp:373-397`)
            // appends `int32(BlockAmount)` and, because the same condition
            // covers `HITINFO_BLOCK | HITINFO_UNK12`, `float(Unk)`; the
            // rage-gain and unk1 blocks between them are never set here.
            info.write_int32(self.blocked);
            info.write_float(0.0f32);
        }

        // ContentTuning.
        info.write_uint8(0u8); // tuning type = none
        info.write_uint8(self.target_level);
        info.write_uint8(self.expansion);
        info.write_int16(0i16); // player_level_delta
        info.write_int8(0i8); // target_scaling_level_delta
        info.write_float(0.0f32); // player_item_level
        info.write_float(0.0f32); // target_item_level
        info.write_uint32(0u32); // scaling_health_item_level_curve_id
        info.write_uint32(0u32); // flags
        info.write_int32(0i32); // player_content_tuning_id
        info.write_int32(0i32); // target_content_tuning_id

        // CombatLogServerPacket::WriteLogDataBit(false), FlushBits(), then attackRoundInfo.
        pkt.write_bit(false);
        pkt.flush_bits();
        let data = info.data().to_vec();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}
