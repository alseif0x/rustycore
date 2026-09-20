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

// ── SpellAbsorbLog (SMSG_SPELL_ABSORB_LOG) ────────────────────────

/// Combat-log packet C++ `Unit::CalcAbsorbResist` sends to the victim for every
/// shield that consumed part of a hit (`Unit.cpp:1876-1889`).
///
/// C++ anchor: `WorldPackets::CombatLog::SpellAbsorbLog::Write`
/// (`CombatLogPackets.cpp:376-397`) writes the attacker and victim packed GUIDs,
/// `int32(AbsorbedSpellID)`, `int32(AbsorbSpellID)`, the absorb aura's caster
/// GUID, `int32(Absorbed)`, `int32(OriginalDamage)`, the empty supporter count,
/// then the packet's `Unk` bit and the `CombatLogServerPacket` log-data bit.
/// `WriteLogData()` only appends to the full-log packet, so the basic packet
/// ends after the flushed bit byte. A white melee swing has no spell of its own,
/// which is why `absorbed_spell_id` is zero for the melee path.
#[derive(Debug, Clone)]
pub struct SpellAbsorbLog {
    pub attacker: ObjectGuid,
    pub victim: ObjectGuid,
    /// C++ `AbsorbedSpellID`: the spell being absorbed (`0` for a white swing).
    pub absorbed_spell_id: i32,
    /// C++ `AbsorbSpellID`: the shield aura's spell.
    pub absorb_spell_id: i32,
    /// C++ `Caster`: the absorb aura's caster.
    pub caster: ObjectGuid,
    pub absorbed: i32,
    pub original_damage: i32,
}

impl ServerPacket for SpellAbsorbLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellAbsorbLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.attacker);
        pkt.write_packed_guid(&self.victim);
        pkt.write_int32(self.absorbed_spell_id);
        pkt.write_int32(self.absorb_spell_id);
        pkt.write_packed_guid(&self.caster);
        pkt.write_int32(self.absorbed);
        pkt.write_int32(self.original_damage);
        // `uint32(Supporters.size())`; `CalcAbsorbResist` leaves the vector empty.
        pkt.write_uint32(0u32);
        // `WriteBit(Unk)` then `WriteLogDataBit()`, both false in the basic
        // packet, flushed together (`CombatLogPackets.cpp:390-394`).
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.flush_bits();
    }
}

// ── SpellNonMeleeDamageLog (SMSG_SPELL_NON_MELEE_DAMAGE_LOG) ──────

/// Combat-log packet C++ `Unit::SendSpellNonMeleeDamageLog` emits for a direct
/// spell hit (`Unit.cpp:5353-5380`).
///
/// C++ anchor: `WorldPackets::CombatLog::SpellNonMeleeDamageLog::Write`
/// (`CombatLogPackets.cpp:92-124`) writes the target (`Me`), caster and cast
/// GUIDs, `int32(SpellID)`, the `SpellCastVisual` (one `SpellXSpellVisualID` in
/// the 3.4.3 branch), `int32(Damage)`, `int32(OriginalDamage)`,
/// `int32(Overkill)`, `uint8(SchoolMask)`, `int32(Absorbed)`,
/// `int32(Resisted)`, `int32(ShieldBlock)`, the empty world-text-viewer and
/// supporter counts, then the bit tail: `Periodic`, `Flags` in seven bits, the
/// false debug-info bit, the basic packet's log-data bit and the content-tuning
/// presence bit. The represented path has no world text viewers, supporters or
/// generated content-tuning parameters, so those stay empty and absent.
#[derive(Debug, Clone)]
pub struct SpellNonMeleeDamageLog {
    /// C++ `Me`: the unit whose combat log this entry belongs to.
    pub target: ObjectGuid,
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    /// C++ `SpellCastVisual::SpellXSpellVisualID`.
    pub visual_id: i32,
    pub damage: i32,
    pub original_damage: i32,
    /// `damage - preHitHealth` when the hit overkilled, else `-1`.
    pub overkill: i32,
    pub school_mask: u8,
    pub absorbed: i32,
    pub resisted: i32,
    pub shield_block: i32,
    /// C++ `Periodic`: false for a direct hit.
    pub periodic: bool,
    /// C++ `CalcDamageInfo`/`SpellNonMeleeDamage::HitInfo`.
    pub flags: i32,
}

impl ServerPacket for SpellNonMeleeDamageLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellNonMeleeDamageLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target);
        pkt.write_packed_guid(&self.caster);
        pkt.write_packed_guid(&self.cast_id);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(self.visual_id);
        pkt.write_int32(self.damage);
        pkt.write_int32(self.original_damage);
        pkt.write_int32(self.overkill);
        pkt.write_uint8(self.school_mask);
        pkt.write_int32(self.absorbed);
        pkt.write_int32(self.resisted);
        pkt.write_int32(self.shield_block);
        // `uint32(WorldTextViewers.size())` and `uint32(Supporters.size())`.
        pkt.write_uint32(0u32);
        pkt.write_uint32(0u32);
        pkt.write_bit(self.periodic);
        pkt.write_bits(self.flags as u32, 7);
        pkt.write_bit(false); // Debug info
        pkt.write_bit(false); // `CombatLogServerPacket::WriteLogDataBit`
        pkt.write_bit(false); // `ContentTuning.has_value()`
        pkt.flush_bits();
    }
}

// ── SpellMissLog (SMSG_SPELL_MISS_LOG) ──────────────────────────

/// One target row in C++ `WorldPackets::CombatLog::SpellMissLog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellMissLogEntry {
    pub victim: ObjectGuid,
    pub miss_reason: u8,
}

/// Standalone spell-miss combat log used by split-damage immunity.
///
/// C++ anchor: `CombatLogPackets.cpp:289-297`; every represented entry omits
/// the optional debug roll pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellMissLog {
    pub spell_id: i32,
    pub caster: ObjectGuid,
    pub entries: Vec<SpellMissLogEntry>,
}

impl ServerPacket for SpellMissLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellMissLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.spell_id);
        pkt.write_guid(&self.caster);
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_guid(&entry.victim);
            pkt.write_uint8(entry.miss_reason);
            pkt.write_bit(false);
            pkt.flush_bits();
        }
    }
}

// ── SpellHealLog (SMSG_SPELL_HEAL_LOG) ────────────────────────────

/// Combat-log packet C++ `Unit::HealBySpell` sends for every spell heal
/// (`Unit.cpp:6538-6563`).
///
/// C++ anchor: `WorldPackets::CombatLog::SpellHealLog::Write`
/// (`CombatLogPackets.cpp:180-212`) writes target and caster packed GUIDs,
/// `int32(SpellID)`, `int32(Health)`, `int32(OriginalHeal)`,
/// `int32(OverHeal)`, `int32(Absorbed)`, the empty supporter count, then the bit
/// tail: `Crit`, the crit-roll-made and crit-roll-needed presence bits, the
/// basic packet's log-data bit and the content-tuning presence bit. The
/// represented path has no supporters, heal absorb, critical roll floats or
/// generated content-tuning parameters, so those stay empty and absent.
#[derive(Debug, Clone)]
pub struct SpellHealLog {
    pub target: ObjectGuid,
    pub caster: ObjectGuid,
    pub spell_id: i32,
    /// C++ `HealInfo::GetHeal()`: the heal before the target's health cap.
    pub health: i32,
    /// C++ `HealInfo::GetOriginalHeal()`.
    pub original_heal: i32,
    /// `health - effective heal`.
    pub over_heal: i32,
    pub absorbed: i32,
    pub crit: bool,
}

impl ServerPacket for SpellHealLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellHealLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target);
        pkt.write_packed_guid(&self.caster);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(self.health);
        pkt.write_int32(self.original_heal);
        pkt.write_int32(self.over_heal);
        pkt.write_int32(self.absorbed);
        // `uint32(Supporters.size())`.
        pkt.write_uint32(0u32);
        pkt.write_bit(self.crit);
        pkt.write_bit(false); // `CritRollMade.has_value()`
        pkt.write_bit(false); // `CritRollNeeded.has_value()`
        pkt.write_bit(false); // `CombatLogServerPacket::WriteLogDataBit`
        pkt.write_bit(false); // `ContentTuning.has_value()`
        pkt.flush_bits();
    }
}

// ── SpellHealAbsorbLog (SMSG_SPELL_HEAL_ABSORB_LOG) ───────────────

/// Combat-log packet C++ `Unit::CalcHealAbsorb` sends for every heal-absorbing
/// aura that consumed part of a heal (`Unit.cpp:2070-2083`).
///
/// C++ anchor: `WorldPackets::CombatLog::SpellHealAbsorbLog::Write`
/// (`CombatLogPackets.cpp:471-486`) writes the target, the absorb aura's caster
/// and the healer packed GUIDs, `int32(AbsorbSpellID)`,
/// `int32(AbsorbedSpellID)`, `int32(Absorbed)`, `int32(OriginalHeal)` and then
/// the content-tuning presence bit. Unlike the damage absorb log this packet is
/// a plain `ServerPacket`, so it has no log-data bit; the represented path has
/// no generated content-tuning parameters, so that bit is false and no
/// parameters follow.
#[derive(Debug, Clone)]
pub struct SpellHealAbsorbLog {
    pub target: ObjectGuid,
    /// C++ `AbsorbCaster`: the heal-absorb aura's caster.
    pub absorb_caster: ObjectGuid,
    pub healer: ObjectGuid,
    pub absorb_spell_id: i32,
    /// C++ `AbsorbedSpellID`: the heal being absorbed (`0` without a spell).
    pub absorbed_spell_id: i32,
    pub absorbed: i32,
    pub original_heal: i32,
}

impl ServerPacket for SpellHealAbsorbLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellHealAbsorbLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target);
        pkt.write_packed_guid(&self.absorb_caster);
        pkt.write_packed_guid(&self.healer);
        pkt.write_int32(self.absorb_spell_id);
        pkt.write_int32(self.absorbed_spell_id);
        pkt.write_int32(self.absorbed);
        pkt.write_int32(self.original_heal);
        pkt.write_bit(false); // `ContentTuning.has_value()`
        pkt.flush_bits();
    }
}

// ── SpellEnergizeLog (SMSG_SPELL_ENERGIZE_LOG) ────────────────────

/// Combat-log packet C++ `Unit::EnergizeBySpell` sends for a power gain or loss
/// (`Unit.cpp:6566-6590`).
///
/// C++ anchor: `WorldPackets::CombatLog::SpellEnergizeLog::Write`
/// (`CombatLogPackets.cpp:214-234`) writes the target and caster packed GUIDs,
/// `int32(SpellID)`, `int32(Type)` (the `Powers` value),
/// `int32(Amount)` (the power actually changed) and `int32(OverEnergize)` (the
/// requested amount the pool could not take), then the basic packet's log-data
/// bit. The represented path has no log data, so no data follows the flushed
/// bit.
#[derive(Debug, Clone)]
pub struct SpellEnergizeLog {
    pub target: ObjectGuid,
    pub caster: ObjectGuid,
    pub spell_id: i32,
    /// C++ `Powers` value of the changed power.
    pub power_type: i32,
    /// C++ `Amount`: the power delta `ModifyPower` actually applied.
    pub amount: i32,
    /// C++ `OverEnergize`: `requested - applied`.
    pub over_energize: i32,
}

impl ServerPacket for SpellEnergizeLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellEnergizeLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.target);
        pkt.write_packed_guid(&self.caster);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(self.power_type);
        pkt.write_int32(self.amount);
        pkt.write_int32(self.over_energize);
        pkt.write_bit(false); // `CombatLogServerPacket::WriteLogDataBit`
        pkt.flush_bits();
    }
}

// ── InterruptPowerRegen (SMSG_INTERRUPT_POWER_REGEN) ──────────────

/// C++ `Player::InterruptPowerRegen` (`Player.cpp:1831-1840`) sends this when
/// `Unit::EnergizeBySpell` fills a power flagged
/// `PowerTypeFlags::UseRegenInterrupt` (`Unit.cpp:6581-6585`).
///
/// C++ anchor: `WorldPackets::Combat::InterruptPowerRegen::Write`
/// (`CombatPackets.cpp:117-122`) writes only `int32(PowerType)` (the `Powers`
/// value).
#[derive(Debug, Clone)]
pub struct InterruptPowerRegen {
    /// C++ `Powers` value whose regeneration is interrupted.
    pub power_type: i32,
}

impl ServerPacket for InterruptPowerRegen {
    const OPCODE: ServerOpcodes = ServerOpcodes::InterruptPowerRegen;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.power_type);
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

// ── SpellExecuteLog (SMSG_SPELL_EXECUTE_LOG) ─────────────────────

/// C++ `SpellLogEffectPowerDrainParams` (`Spell.h:165-171`), produced by
/// `Spell::ExecuteLogEffectTakeTargetPower` (`Spell.cpp:5076-5086`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellLogEffectPowerDrainParams {
    pub victim: ObjectGuid,
    pub points: u32,
    pub power_type: u32,
    pub amplitude: f32,
}

/// C++ `SpellLogEffectExtraAttacksParams` (`Spell.h:173-177`), produced by
/// `Spell::ExecuteLogEffectExtraAttacks` (`Spell.cpp:5088-5095`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogEffectExtraAttacksParams {
    pub victim: ObjectGuid,
    pub num_attacks: u32,
}

/// C++ `SpellLogEffectDurabilityDamageParams` (`Spell.h:179-184`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogEffectDurabilityDamageParams {
    pub victim: ObjectGuid,
    pub item_id: i32,
    pub amount: i32,
}

/// C++ `SpellLogEffectGenericVictimParams` (`Spell.h:186-189`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogEffectGenericVictimParams {
    pub victim: ObjectGuid,
}

/// C++ `SpellLogEffectTradeSkillItemParams` (`Spell.h:191-194`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogEffectTradeSkillItemParams {
    pub item_id: i32,
}

/// C++ `SpellLogEffectFeedPetParams` (`Spell.h:196-199`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogEffectFeedPetParams {
    pub item_id: i32,
}

/// C++ `SpellLogEffect` (`Spell.h:201-213`): one effect of the cast's execute
/// log. C++ models each list as an `Optional` vector and only writes a list
/// when it exists; an empty represented vector is exactly the absent list, so
/// the count is written as zero and no rows follow.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellLogEffect {
    /// C++ `Effect`: the `SpellEffectName` value of the effect.
    pub effect: i32,
    pub power_drain_targets: Vec<SpellLogEffectPowerDrainParams>,
    pub extra_attacks_targets: Vec<SpellLogEffectExtraAttacksParams>,
    pub durability_damage_targets: Vec<SpellLogEffectDurabilityDamageParams>,
    pub generic_victim_targets: Vec<SpellLogEffectGenericVictimParams>,
    pub trade_skill_targets: Vec<SpellLogEffectTradeSkillItemParams>,
    pub feed_pet_targets: Vec<SpellLogEffectFeedPetParams>,
}

/// C++ `Spell::SendSpellExecuteLog` (`Spell.cpp:5048-5060`), sent from
/// `Spell::FinishTargetProcessing` after every effect has resolved.
///
/// C++ anchor: `WorldPackets::CombatLog::SpellExecuteLog::Write`
/// (`CombatLogPackets.cpp:90-155`) writes the caster, `int32(SpellID)`, the
/// effect count, then per effect its id, the six list counts, and each list's
/// rows; the basic packet closes with the log-data bit.
#[derive(Debug, Clone)]
pub struct SpellExecuteLog {
    pub caster: ObjectGuid,
    pub spell_id: i32,
    pub effects: Vec<SpellLogEffect>,
}

impl ServerPacket for SpellExecuteLog {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellExecuteLog;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.caster);
        pkt.write_int32(self.spell_id);
        pkt.write_uint32(self.effects.len() as u32);

        for effect in &self.effects {
            pkt.write_int32(effect.effect);
            pkt.write_uint32(effect.power_drain_targets.len() as u32);
            pkt.write_uint32(effect.extra_attacks_targets.len() as u32);
            pkt.write_uint32(effect.durability_damage_targets.len() as u32);
            pkt.write_uint32(effect.generic_victim_targets.len() as u32);
            pkt.write_uint32(effect.trade_skill_targets.len() as u32);
            pkt.write_uint32(effect.feed_pet_targets.len() as u32);

            for row in &effect.power_drain_targets {
                pkt.write_packed_guid(&row.victim);
                pkt.write_uint32(row.points);
                pkt.write_uint32(row.power_type);
                pkt.write_float(row.amplitude);
            }
            for row in &effect.extra_attacks_targets {
                pkt.write_packed_guid(&row.victim);
                pkt.write_uint32(row.num_attacks);
            }
            for row in &effect.durability_damage_targets {
                pkt.write_packed_guid(&row.victim);
                pkt.write_int32(row.item_id);
                pkt.write_int32(row.amount);
            }
            for row in &effect.generic_victim_targets {
                pkt.write_packed_guid(&row.victim);
            }
            for row in &effect.trade_skill_targets {
                pkt.write_int32(row.item_id);
            }
            for row in &effect.feed_pet_targets {
                pkt.write_int32(row.item_id);
            }
        }

        pkt.write_bit(false); // `CombatLogServerPacket::WriteLogDataBit`
        pkt.flush_bits();
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
            original_damage: 100,
            over_damage: -1,
            blocked: 30,
            absorbed: 0,
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
        assert_eq!(info.read_int32().expect("original damage"), 100);
        assert_eq!(info.read_int32().expect("over damage"), -1);
        // `CombatLogPackets.cpp:355-365`: the presence byte, then the sub-damage
        // struct. This swing carries no absorb/resist bit, so neither amount is
        // serialized.
        assert_eq!(info.read_uint8().expect("sub damage present"), 1);
        assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
        assert_eq!(info.read_float().expect("sub damage float"), 70.0);
        assert_eq!(info.read_int32().expect("sub damage"), 70);
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
        assert_eq!(HIT_INFO_CRUSHING, 0x0002_0000);
        assert_eq!(HIT_INFO_FULL_ABSORB, 0x0000_0020);
        assert_eq!(HIT_INFO_PARTIAL_ABSORB, 0x0000_0040);
        assert_eq!(HIT_INFO_GLANCING, 0x0001_0000);
        assert_eq!(HIT_INFO_FAKE_DAMAGE, 0x0100_0000);
        assert_eq!(VICTIM_STATE_INTACT, 0);
        assert_eq!(VICTIM_STATE_HIT, 1);
        assert_eq!(VICTIM_STATE_DODGE, 2);
        assert_eq!(VICTIM_STATE_PARRY, 3);
    }

    #[test]
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
            original_damage: 0,
            over_damage: -1,
            blocked: 0,
            absorbed: 0,
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
        assert_eq!(info.read_uint8().expect("sub damage present"), 1);
        assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
        assert_eq!(info.read_float().expect("sub damage float"), 0.0);
        assert_eq!(info.read_int32().expect("sub damage"), 0);
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

    /// C++ `AttackerStateUpdate::Write` (`CombatLogPackets.cpp:355-365`): a
    /// school absorb sets `HITINFO_PARTIAL_ABSORB`/`HITINFO_FULL_ABSORB` and the
    /// packet then carries `SubDmg.Absorbed` after the sub-damage integers.
    #[test]
    fn attacker_state_update_writes_absorbed_sub_damage_like_cpp() {
        let guid = |entry: u32, low: i64| {
            ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::Creature,
                0,
                0,
                0,
                0,
                entry,
                low,
            )
        };
        for (hit_info, damage, absorbed) in [
            (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB, 70, 30),
            (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FULL_ABSORB, 0, 100),
        ] {
            let bytes = AttackerStateUpdate {
                attacker: guid(123, 0x1234),
                victim: guid(124, 0x1235),
                hit_info,
                damage,
                original_damage: 100,
                over_damage: -1,
                blocked: 0,
                absorbed,
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
            let size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
            let round_info = pkt.read_bytes(size).expect("attackRoundInfo bytes");
            let mut info = WorldPacket::from_bytes(&round_info);
            assert_eq!(info.read_uint32().expect("hitInfo"), hit_info);
            let _ = info.read_packed_guid().expect("attacker");
            let _ = info.read_packed_guid().expect("victim");
            assert_eq!(info.read_int32().expect("damage"), damage);
            assert_eq!(info.read_int32().expect("original damage"), 100);
            assert_eq!(info.read_int32().expect("over damage"), -1);
            assert_eq!(info.read_uint8().expect("sub damage present"), 1);
            assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
            assert_eq!(info.read_float().expect("sub damage float"), damage as f32);
            assert_eq!(info.read_int32().expect("sub damage"), damage);
            assert_eq!(info.read_int32().expect("absorbed"), absorbed);
            assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
        }
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
    fn spell_absorb_log_writes_cpp_field_order_like_cpp() {
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            123,
            0x1234,
        );
        let victim = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let caster = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
        let bytes = SpellAbsorbLog {
            attacker,
            victim,
            absorbed_spell_id: 0,
            absorb_spell_id: 17_262,
            caster,
            absorbed: 10,
            original_damage: 10,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellAbsorbLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("attacker"), attacker);
        assert_eq!(pkt.read_packed_guid().expect("victim"), victim);
        assert_eq!(pkt.read_int32().expect("absorbed spell id"), 0);
        assert_eq!(pkt.read_int32().expect("absorb spell id"), 17_262);
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("absorbed"), 10);
        assert_eq!(pkt.read_int32().expect("original damage"), 10);
        assert_eq!(pkt.read_uint32().expect("supporters"), 0);
        // `WriteBit(Unk)` then `WriteLogDataBit()`: the basic packet carries two
        // false bits and no log data.
        assert!(!pkt.has_bit().expect("unk"));
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_non_melee_damage_log_writes_cpp_field_order_like_cpp() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let target = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            123,
            0x1234,
        );
        let cast_id = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Cast,
            0,
            1,
            0,
            0,
            456,
            0x5678,
        );
        let bytes = SpellNonMeleeDamageLog {
            target,
            caster,
            cast_id,
            spell_id: 116,
            visual_id: 42,
            damage: 83,
            original_damage: 100,
            overkill: -1,
            school_mask: 4,
            absorbed: 10,
            resisted: 7,
            shield_block: 0,
            periodic: false,
            flags: 0,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellNonMeleeDamageLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("me"), target);
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_packed_guid().expect("cast id"), cast_id);
        assert_eq!(pkt.read_int32().expect("spell id"), 116);
        assert_eq!(pkt.read_int32().expect("visual"), 42);
        assert_eq!(pkt.read_int32().expect("damage"), 83);
        assert_eq!(pkt.read_int32().expect("original damage"), 100);
        assert_eq!(pkt.read_int32().expect("overkill"), -1);
        assert_eq!(pkt.read_uint8().expect("school mask"), 4);
        assert_eq!(pkt.read_int32().expect("absorbed"), 10);
        assert_eq!(pkt.read_int32().expect("resisted"), 7);
        assert_eq!(pkt.read_int32().expect("shield block"), 0);
        assert_eq!(pkt.read_uint32().expect("world text viewers"), 0);
        assert_eq!(pkt.read_uint32().expect("supporters"), 0);
        // Bits: `Periodic`, seven `Flags`, debug info, log data and content
        // tuning, all flushed together.
        assert!(!pkt.has_bit().expect("periodic"));
        assert_eq!(pkt.read_bits(7).expect("flags"), 0);
        assert!(!pkt.has_bit().expect("debug info"));
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(!pkt.has_bit().expect("content tuning"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_heal_log_writes_cpp_field_order_like_cpp() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let target = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
        let bytes = SpellHealLog {
            target,
            caster,
            spell_id: 2_066_001,
            health: 300,
            original_heal: 300,
            over_heal: 40,
            absorbed: 0,
            crit: false,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellHealLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("target"), target);
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("spell id"), 2_066_001);
        assert_eq!(pkt.read_int32().expect("health"), 300);
        assert_eq!(pkt.read_int32().expect("original heal"), 300);
        assert_eq!(pkt.read_int32().expect("over heal"), 40);
        assert_eq!(pkt.read_int32().expect("absorbed"), 0);
        assert_eq!(pkt.read_uint32().expect("supporters"), 0);
        assert!(!pkt.has_bit().expect("crit"));
        assert!(!pkt.has_bit().expect("crit roll made"));
        assert!(!pkt.has_bit().expect("crit roll needed"));
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(!pkt.has_bit().expect("content tuning"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_heal_absorb_log_writes_cpp_field_order_like_cpp() {
        let target = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let absorb_caster = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
        let healer = ObjectGuid::create_player(1, 0x2122_2324_2526_2728);
        let bytes = SpellHealAbsorbLog {
            target,
            absorb_caster,
            healer,
            absorb_spell_id: 17_262,
            absorbed_spell_id: 2_066_001,
            absorbed: 120,
            original_heal: 300,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellHealAbsorbLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("target"), target);
        assert_eq!(
            pkt.read_packed_guid().expect("absorb caster"),
            absorb_caster
        );
        assert_eq!(pkt.read_packed_guid().expect("healer"), healer);
        assert_eq!(pkt.read_int32().expect("absorb spell id"), 17_262);
        assert_eq!(pkt.read_int32().expect("absorbed spell id"), 2_066_001);
        assert_eq!(pkt.read_int32().expect("absorbed"), 120);
        assert_eq!(pkt.read_int32().expect("original heal"), 300);
        assert!(!pkt.has_bit().expect("content tuning"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_energize_log_writes_cpp_field_order_like_cpp() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let target = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
        let bytes = SpellEnergizeLog {
            target,
            caster,
            spell_id: 793,
            power_type: 0,
            amount: 50,
            over_energize: 10,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellEnergizeLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("target"), target);
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("spell id"), 793);
        assert_eq!(pkt.read_int32().expect("power type"), 0);
        assert_eq!(pkt.read_int32().expect("amount"), 50);
        assert_eq!(pkt.read_int32().expect("over energize"), 10);
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_execute_log_writes_cpp_effect_lists() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let victim = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            0,
            9_001,
            44,
        );
        let bytes = SpellExecuteLog {
            caster,
            spell_id: 2_971,
            effects: vec![
                SpellLogEffect {
                    effect: 122, // SPELL_EFFECT_POWER_DRAIN
                    power_drain_targets: vec![SpellLogEffectPowerDrainParams {
                        victim,
                        points: 40,
                        power_type: 0,
                        amplitude: 0.5,
                    }],
                    ..Default::default()
                },
                SpellLogEffect {
                    effect: 16, // SPELL_EFFECT_ADD_EXTRA_ATTACKS
                    extra_attacks_targets: vec![SpellLogEffectExtraAttacksParams {
                        victim,
                        num_attacks: 3,
                    }],
                    ..Default::default()
                },
            ],
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellExecuteLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("spell id"), 2_971);
        assert_eq!(pkt.read_uint32().expect("effect count"), 2);
        // C++ writes each effect's id, the six list counts, then the rows.
        assert_eq!(pkt.read_int32().expect("effect"), 122);
        assert_eq!(pkt.read_uint32().expect("power drain count"), 1);
        for list in 0..5 {
            assert_eq!(
                pkt.read_uint32().expect("empty list count"),
                0,
                "list {list} has no producer in this represented cast"
            );
        }
        assert_eq!(pkt.read_packed_guid().expect("drain victim"), victim);
        assert_eq!(pkt.read_uint32().expect("points"), 40);
        assert_eq!(pkt.read_uint32().expect("power type"), 0);
        assert_eq!(pkt.read_float().expect("amplitude"), 0.5);
        assert_eq!(pkt.read_int32().expect("effect"), 16);
        assert_eq!(pkt.read_uint32().expect("power drain count"), 0);
        assert_eq!(pkt.read_uint32().expect("extra attacks count"), 1);
        for list in 0..4 {
            assert_eq!(
                pkt.read_uint32().expect("empty list count"),
                0,
                "list {list} has no producer in this represented cast"
            );
        }
        assert_eq!(pkt.read_packed_guid().expect("extra victim"), victim);
        assert_eq!(pkt.read_uint32().expect("num attacks"), 3);
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_execute_log_writes_the_durability_generic_trade_and_feed_lists() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let victim = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            0,
            9_001,
            45,
        );
        let bytes = SpellExecuteLog {
            caster,
            spell_id: 4_036,
            effects: vec![SpellLogEffect {
                effect: 25, // SPELL_EFFECT_DURABILITY_DAMAGE
                durability_damage_targets: vec![SpellLogEffectDurabilityDamageParams {
                    victim,
                    item_id: 300,
                    amount: 15,
                }],
                generic_victim_targets: vec![SpellLogEffectGenericVictimParams { victim }],
                trade_skill_targets: vec![SpellLogEffectTradeSkillItemParams { item_id: 1_234 }],
                feed_pet_targets: vec![SpellLogEffectFeedPetParams { item_id: 5_678 }],
                ..Default::default()
            }],
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellExecuteLog as u16
        );
        assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
        assert_eq!(pkt.read_int32().expect("spell id"), 4_036);
        assert_eq!(pkt.read_uint32().expect("effect count"), 1);
        assert_eq!(pkt.read_int32().expect("effect"), 25);
        assert_eq!(pkt.read_uint32().expect("power drain count"), 0);
        assert_eq!(pkt.read_uint32().expect("extra attacks count"), 0);
        assert_eq!(pkt.read_uint32().expect("durability count"), 1);
        assert_eq!(pkt.read_uint32().expect("generic victim count"), 1);
        assert_eq!(pkt.read_uint32().expect("trade skill count"), 1);
        assert_eq!(pkt.read_uint32().expect("feed pet count"), 1);
        // C++ writes the rows in list order after all six counts
        // (`CombatLogPackets.cpp:120-153`).
        assert_eq!(pkt.read_packed_guid().expect("durability victim"), victim);
        assert_eq!(pkt.read_int32().expect("item id"), 300);
        assert_eq!(pkt.read_int32().expect("amount"), 15);
        assert_eq!(pkt.read_packed_guid().expect("generic victim"), victim);
        assert_eq!(pkt.read_int32().expect("trade skill item"), 1_234);
        assert_eq!(pkt.read_int32().expect("feed pet item"), 5_678);
        assert!(!pkt.has_bit().expect("has log data"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn interrupt_power_regen_writes_only_the_power_type_like_cpp() {
        let bytes = InterruptPowerRegen { power_type: 3 }.to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::InterruptPowerRegen as u16
        );
        assert_eq!(
            pkt.read_int32().expect("power type"),
            3,
            "C++ `InterruptPowerRegen::Write` emits `int32(PowerType)` only"
        );
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

    #[test]
    fn spell_miss_log_writes_cpp_field_order_like_cpp() {
        let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
        let victim = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            0,
            0,
            0,
            123,
            0x1234,
        );
        let bytes = SpellMissLog {
            spell_id: 91_364,
            caster,
            entries: vec![SpellMissLogEntry {
                victim,
                miss_reason: 7,
            }],
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::SpellMissLog as u16
        );
        assert_eq!(pkt.read_int32().expect("spell id"), 91_364);
        assert_eq!(pkt.read_guid().expect("caster"), caster);
        assert_eq!(pkt.read_uint32().expect("entry count"), 1);
        assert_eq!(pkt.read_guid().expect("victim"), victim);
        assert_eq!(pkt.read_uint8().expect("miss reason"), 7);
        assert!(!pkt.has_bit().expect("debug absent"));
        assert!(pkt.is_empty());
    }
}
