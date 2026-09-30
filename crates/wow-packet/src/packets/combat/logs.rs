use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

use crate::ServerPacket;
use crate::world_packet::WorldPacket;

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
