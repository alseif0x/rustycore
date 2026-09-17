// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet definitions.

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
    /// Overkill amount (-1 if target is still alive).
    pub over_damage: i32,
    /// C++ `CalcDamageInfo::Blocked`, serialized only for `HITINFO_BLOCK`.
    pub blocked: i32,
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
/// C++ `HITINFO_BLOCK`: the packet then carries `blocked` and the trailing
/// `float Unk` C++ writes for `HITINFO_BLOCK | HITINFO_UNK12`.
pub const HIT_INFO_BLOCK: u32 = 0x0000_2000;
/// C++ `HITINFO_FAKE_DAMAGE`: enables a damage animation even if no damage is done.
pub const HIT_INFO_FAKE_DAMAGE: u32 = 0x0100_0000;

/// C++ `VictimState` (`Unit.h:45-55` in the 3.4.3 target).
pub const VICTIM_STATE_INTACT: u8 = 0;
/// C++ `VICTIMSTATE_HIT`.
pub const VICTIM_STATE_HIT: u8 = 1;
/// C++ `VICTIMSTATE_DODGE`.
pub const VICTIM_STATE_DODGE: u8 = 2;
/// C++ `VICTIMSTATE_PARRY`.
pub const VICTIM_STATE_PARRY: u8 = 3;

impl ServerPacket for AttackerStateUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AttackerStateUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        // C++ builds attackRoundInfo in a separate ByteBuffer.
        let mut info = WorldPacket::new_empty();
        info.write_uint32(self.hit_info);
        info.write_packed_guid(&self.attacker);
        info.write_packed_guid(&self.victim);
        info.write_int32(self.damage);
        info.write_int32(self.damage); // original damage
        info.write_int32(self.over_damage); // over damage (-1 if alive)
        info.write_uint8(0u8); // no SubDmg
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

// ── HealthUpdate (SMSG_HEALTH_UPDATE) ─────────────────────────────

/// Direct owner health update sent by C++ `Unit::ModifyHealth` when damage
/// lowers the health of a player-owned unit.
///
/// C++ anchor: `WorldPackets::Combat::HealthUpdate::Write` writes `Guid`
/// followed by `int64(Health)`.
#[derive(Debug, Clone)]
pub struct HealthUpdate {
    pub guid: ObjectGuid,
    pub health: i64,
}

impl ServerPacket for HealthUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::HealthUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_int64(self.health);
    }
}

// ── PowerUpdate (SMSG_POWER_UPDATE) ──────────────────────────────

/// Direct power update sent by C++ `Unit::SetPower` when an in-world unit's
/// power changes.
///
/// C++ anchor: `WorldPackets::Combat::PowerUpdate::Write`
/// (`CombatPackets.cpp:104-116`) writes the packed `Guid`, a `uint32` count,
/// then for each entry `int32(Power)` followed by `uint8(PowerType)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerUpdate {
    pub guid: ObjectGuid,
    /// `(power value, power type)` entries in `Powers` order.
    pub powers: Vec<(i32, u8)>,
}

impl ServerPacket for PowerUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::PowerUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.powers.len() as u32);
        for (power, power_type) in &self.powers {
            pkt.write_int32(*power);
            pkt.write_uint8(*power_type);
        }
    }
}

// ── SpellInstakillLog (SMSG_SPELL_INSTAKILL_LOG) ─────────────────

/// Combat-log packet emitted by C++ `Spell::EffectInstaKill` before
/// `Unit::Kill`.
///
/// C++ anchor: `WorldPackets::CombatLog::SpellInstakillLog::Write` streams
/// `Target`, `Caster`, then `int32(SpellID)`.
#[derive(Debug, Clone)]
pub struct SpellInstakillLog {
    pub target: ObjectGuid,
    pub caster: ObjectGuid,
    pub spell_id: i32,
}

impl ServerPacket for SpellInstakillLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellInstakillLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target);
        pkt.write_packed_guid(&self.caster);
        pkt.write_int32(self.spell_id);
    }
}

// ── EnvironmentalDamageLog (SMSG_ENVIRONMENTAL_DAMAGE_LOG) ───────

/// Combat-log packet emitted by C++ `Player::EnvironmentalDamage` after
/// applying represented environmental damage.
///
/// C++ anchor: `WorldPackets::CombatLog::EnvironmentalDamageLog::Write`
/// writes `Victim`, `uint8(Type)`, `int32(Amount)`, `int32(Resisted)`,
/// `int32(Absorbed)`, then the empty `CombatLogServerPacket` log-data bit.
#[derive(Debug, Clone)]
pub struct EnvironmentalDamageLog {
    pub victim: ObjectGuid,
    pub damage_type: u8,
    pub amount: i32,
    pub resisted: i32,
    pub absorbed: i32,
}

impl ServerPacket for EnvironmentalDamageLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnvironmentalDamageLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.victim);
        pkt.write_uint8(self.damage_type);
        pkt.write_int32(self.amount);
        pkt.write_int32(self.resisted);
        pkt.write_int32(self.absorbed);
        pkt.write_bit(false); // no LogData
        pkt.flush_bits();
    }
}

// ── PvPCredit (SMSG_PVP_CREDIT) ──────────────────────────────────

/// Direct honor-credit packet emitted by C++ `Spell::EffectGiveHonor` after
/// `Player::AddHonorXP`.
///
/// C++ anchor: `WorldPackets::Combat::PvPCredit::Write` writes
/// `int32(OriginalHonor)`, `int32(Honor)`, `ObjectGuid Target`, then
/// `int32(Rank)`. The C++ `ObjectGuid` stream operator uses the packed GUID
/// format in `ObjectGuid.cpp`.
#[derive(Debug, Clone)]
pub struct PvpCredit {
    pub original_honor: i32,
    pub honor: i32,
    pub target: ObjectGuid,
    pub rank: i32,
}

impl ServerPacket for PvpCredit {
    const OPCODE: ServerOpcodes = ServerOpcodes::PvpCredit;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.original_honor);
        pkt.write_int32(self.honor);
        pkt.write_packed_guid(&self.target);
        pkt.write_int32(self.rank);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test]
    fn attacker_state_update_writes_the_block_fields_like_cpp() {
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            125,
            0x1236,
        );
        let victim = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            126,
            0x1237,
        );
        let bytes = AttackerStateUpdate {
            attacker,
            victim,
            hit_info: HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK,
            damage: 70,
            over_damage: -1,
            blocked: 30,
            victim_state: VICTIM_STATE_HIT,
            school_mask: 1,
            target_level: 80,
            expansion: 2,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::AttackerStateUpdate as u16
        );
        let _ = pkt.read_bit().expect("has_log_data");
        let attack_round_info_size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
        let attack_round_info = pkt
            .read_bytes(attack_round_info_size)
            .expect("attackRoundInfo bytes");
        let mut info = WorldPacket::from_bytes(&attack_round_info);
        assert_eq!(
            info.read_uint32().expect("hitInfo"),
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK
        );
        assert_eq!(info.read_packed_guid().expect("attacker"), attacker);
        assert_eq!(info.read_packed_guid().expect("victim"), victim);
        assert_eq!(info.read_int32().expect("damage"), 70);
        assert_eq!(info.read_int32().expect("original damage"), 70);
        assert_eq!(info.read_int32().expect("over damage"), -1);
        assert_eq!(info.read_uint8().expect("sub damage"), 0);
        assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
        assert_eq!(info.read_uint32().expect("attacker state"), 0);
        assert_eq!(info.read_uint32().expect("melee spell id"), 0);
        // `CombatLogPackets.cpp:373-397`: the blocked amount, then the trailing
        // `float Unk` the same condition writes.
        assert_eq!(info.read_int32().expect("blocked"), 30);
        assert_eq!(info.read_float().expect("unk"), 0.0);
        assert_eq!(info.read_uint8().expect("content tuning type"), 0);
        assert_eq!(info.read_uint8().expect("target level"), 80);
    }

    #[test]
    fn hit_info_and_victim_state_match_cpp_3_4_3_like_cpp() {
        // `UnitDefines.h:440-465` and `Unit.h:45-55` in the 3.4.3 target.
        assert_eq!(HIT_INFO_AFFECTS_VICTIM, 0x0000_0002);
        assert_eq!(HIT_INFO_OFFHAND, 0x0000_0004);
        assert_eq!(HIT_INFO_MISS, 0x0000_0010);
        assert_eq!(HIT_INFO_CRITICAL_HIT, 0x0000_0200);
        assert_eq!(HIT_INFO_GLANCING, 0x0001_0000);
        assert_eq!(HIT_INFO_FAKE_DAMAGE, 0x0100_0000);
        assert_eq!(VICTIM_STATE_INTACT, 0);
        assert_eq!(VICTIM_STATE_HIT, 1);
        assert_eq!(VICTIM_STATE_DODGE, 2);
        assert_eq!(VICTIM_STATE_PARRY, 3);
    }

    fn attacker_state_update_writes_custom_hit_info_like_cpp() {
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            123,
            0x1234,
        );
        let victim = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            124,
            0x1235,
        );
        let bytes = AttackerStateUpdate {
            attacker,
            victim,
            hit_info: HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FAKE_DAMAGE,
            damage: 0,
            over_damage: -1,
            blocked: 0,
            victim_state: VICTIM_STATE_HIT,
            school_mask: 1,
            target_level: 80,
            expansion: 2,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::AttackerStateUpdate as u16
        );
        assert!(
            !pkt.read_bit().expect("has_log_data"),
            "CombatLogServerPacket writes the log-data bit before attackRoundInfo size"
        );
        let attack_round_info_size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
        let attack_round_info = pkt
            .read_bytes(attack_round_info_size)
            .expect("attackRoundInfo bytes");
        let mut info = WorldPacket::from_bytes(&attack_round_info);
        assert_eq!(
            info.read_uint32().expect("hitInfo"),
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FAKE_DAMAGE
        );
        assert_eq!(info.read_packed_guid().expect("attacker"), attacker);
        assert_eq!(info.read_packed_guid().expect("victim"), victim);
        assert_eq!(info.read_int32().expect("damage"), 0);
        assert_eq!(info.read_int32().expect("original damage"), 0);
        assert_eq!(info.read_int32().expect("over damage"), -1);
        assert_eq!(info.read_uint8().expect("sub damage"), 0);
        assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
        assert_eq!(info.read_uint32().expect("attacker state"), 0);
        assert_eq!(info.read_uint32().expect("melee spell id"), 0);
        assert_eq!(info.read_uint8().expect("content tuning type"), 0);
        assert_eq!(info.read_uint8().expect("target level"), 80);
        assert_eq!(info.read_uint8().expect("expansion"), 2);
        assert_eq!(info.read_int16().expect("player level delta"), 0);
        assert_eq!(info.read_int8().expect("target scaling level delta"), 0);
        assert_eq!(info.read_float().expect("player item level"), 0.0);
        assert_eq!(info.read_float().expect("target item level"), 0.0);
        assert_eq!(info.read_uint32().expect("scaling curve"), 0);
        assert_eq!(info.read_uint32().expect("content tuning flags"), 0);
        assert_eq!(info.read_int32().expect("player content tuning id"), 0);
        assert_eq!(info.read_int32().expect("target content tuning id"), 0);
        assert!(
            info.is_empty(),
            "attackRoundInfo must not contain the combat-log bit"
        );
    }

    #[test]
    fn ai_reaction_serializes_guid_then_reaction_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            123,
            0x1234,
        );
        let bytes = AIReaction {
            unit_guid: guid,
            reaction: AiReaction::Alert,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::AiReaction as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("unit guid"), guid);
        assert_eq!(
            pkt.read_uint32().expect("reaction"),
            AiReaction::Alert as u32
        );
        assert!(pkt.is_empty());
    }

    #[test]
    fn health_update_writes_packed_guid_and_i64_health_like_cpp() {
        let guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = HealthUpdate { guid, health: 83 }.to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::HealthUpdate as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("guid"), guid);
        assert_eq!(pkt.read_int64().expect("health"), 83);
        assert!(pkt.is_empty());
    }

    #[test]
    fn power_update_writes_guid_count_and_power_type_pairs_like_cpp() {
        let guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = PowerUpdate {
            guid,
            powers: vec![(4321, 0)],
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::PowerUpdate as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("guid"), guid);
        assert_eq!(pkt.read_uint32().expect("count"), 1);
        assert_eq!(pkt.read_int32().expect("power"), 4321);
        assert_eq!(pkt.read_uint8().expect("power type"), 0);
        assert!(pkt.is_empty());
    }

    #[test]
    fn environmental_damage_log_writes_cpp_shape_without_log_data() {
        let victim = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = EnvironmentalDamageLog {
            victim,
            damage_type: 2,
            amount: 117,
            resisted: 0,
            absorbed: 0,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::EnvironmentalDamageLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("victim"), victim);
        assert_eq!(pkt.read_uint8().expect("type"), 2);
        assert_eq!(pkt.read_int32().expect("amount"), 117);
        assert_eq!(pkt.read_int32().expect("resisted"), 0);
        assert_eq!(pkt.read_int32().expect("absorbed"), 0);
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn pvp_credit_writes_cpp_field_order_like_cpp() {
        let target = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = PvpCredit {
            original_honor: 42,
            honor: 40,
            target,
            rank: 7,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::PvpCredit as u16
        );
        assert_eq!(pkt.read_int32().expect("OriginalHonor"), 42);
        assert_eq!(pkt.read_int32().expect("Honor"), 40);
        assert_eq!(pkt.read_packed_guid().expect("Target"), target);
        assert_eq!(pkt.read_int32().expect("Rank"), 7);
        assert!(pkt.is_empty());
    }

    #[test]
    fn break_target_writes_unit_guid_like_cpp() {
        let unit_guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = BreakTarget { unit_guid }.to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::BreakTarget as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("UnitGUID"), unit_guid);
        assert!(pkt.is_empty());
    }

    #[test]
    fn cancel_combat_writes_empty_cpp_payload_like_cpp() {
        let bytes = CancelCombat.to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::CancelCombat as u16
        );
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_instakill_log_writes_target_caster_and_spell_like_cpp() {
        let target = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            0,
            9_001,
            44,
        );
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let bytes = SpellInstakillLog {
            target,
            caster,
            spell_id: 5_333,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellInstakillLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("target"), target);
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("spell id"), 5_333);
        assert!(pkt.is_empty());
    }
}
