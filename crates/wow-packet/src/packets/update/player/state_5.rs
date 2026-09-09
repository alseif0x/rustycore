//! Player update-value structs state definitions, part 5 of 5.
//!
//! Separated from the player.rs root under #650. Behaviour is preserved.

use super::*;

/// ActivePlayerData VALUES update for the runtime paths currently emitted by
/// RustyCore: InvSlots[141], buyback, coinage and combat stats.
///
/// C++ `UF::ActivePlayerData::WriteUpdate` format:
///   WriteUInt32(blocksMask group 0) — byte-aligned u32 for first 32 blocks
///   WriteBits(blocksMask group 1, 16) — 16 bits for remaining 16 blocks
///   for each active block: WriteBits(block, 32)
///   FlushBits()
///   [second dynamic-mask pass for parent-0 fields 4..19]
///   FlushBits()
///   [field values]
///
/// This writer intentionally does not cover the full 1525-bit
/// ActivePlayerData surface yet. `#026i` tracks the remaining generic writer
/// work: SkillInfo, quest/title/toy/transmog/trait dynamics, research,
/// PVP/rest/profession/bag flags, quest completed and glyph arrays.
///
/// InvSlots: parent=124, elements=125-265. Span multiple blocks.
///
/// ActivePlayerData secondary stats (from stat_changes):
///   Parent 0:            bits 36-37 (expertise)  → block 0 bit 0, block 1 bits 4-5
///   Parent 38:           bits 39-69 (all 31 fields) → block 1 bits 6-31, block 2 bits 0-5
///   ModDamageDonePos[7]: parent=269, bits=277-283 → block 8 bits 13,21-27
///   CombatRatings[32]:   parent=574, bits=575-606 → block 17 bits 30-31, block 18 bits 0-30
///
/// C++ WriteUpdate order for these fields:
/// parent 0 → parent 38 → InvSlots(124) → SpellCrit/ModDamageDone(269)
/// → Buyback(549) → CombatRatings(574).
pub(in crate::packets::update) fn write_active_player_data_values_update(
    buf: &mut WorldPacket,
    inv_slot_changes: &[(u8, ObjectGuid)],
    buyback_changes: &[(u8, u32, i64)],
    stat_changes: Option<&PlayerStatChanges>,
    coinage_change: Option<u64>,
) {
    let mut blocks = [0u32; 48];

    // Coinage: block 0 bit 28 (ActivePlayerData.Coinage = new(0, 28))
    if coinage_change.is_some() {
        blocks[0] |= 1 << 0;
        blocks[0] |= 1 << 28;
    }

    // InvSlots: parent bit 124 = block 3 bit 28
    if !inv_slot_changes.is_empty() {
        blocks[3] |= 1 << 28;
        for &(slot, _) in inv_slot_changes {
            if (slot as u32) >= 141 {
                continue;
            }
            let bit = 125 + slot as u32;
            let block_idx = (bit / 32) as usize;
            let bit_in_block = bit % 32;
            if block_idx < 48 {
                blocks[block_idx] |= 1 << bit_in_block;
            }
        }
    }

    // BuybackPrice[12]: parent bit 549, price bits 550-561, timestamp bits 562-573.
    if !buyback_changes.is_empty() {
        blocks[17] |= 1 << 5;
        for &(slot, _, _) in buyback_changes {
            if !(94..106).contains(&slot) {
                continue;
            }
            let index = u32::from(slot - 94);
            for bit in [550 + index, 562 + index] {
                let block_idx = (bit / 32) as usize;
                let bit_in_block = bit % 32;
                if block_idx < 48 {
                    blocks[block_idx] |= 1 << bit_in_block;
                }
            }
        }
    }

    // Secondary stats from stat_changes
    if stat_changes.is_some() {
        // Parent 0 section: MainhandExpertise(bit 36→b1:4), OffhandExpertise(bit 37→b1:5)
        blocks[0] |= 1 << 0;
        blocks[1] |= (1 << 4) | (1 << 5);

        // Parent 38 section: 30 fields (bits 39-49, 51-69). This represented
        // stats VALUES writer is a narrow runtime path, not the final generic
        // C++ update-field writer. The 3.4.3.54261 client rejects bit 50 in
        // this packet shape; emitting it shifts every following ActivePlayerData
        // field by +4 bytes and desyncs the client's value walk. C++ still
        // defines ShieldBlockCritPercentage, so this is not a schema deletion:
        // the final generic writer must emit exactly the C++ change mask for
        // the object state being updated.
        // parent=38→b1:6, bits 39-63→b1:7-31 EXCEPT bit 50→b1:18, bits 64-69→b2:0-5
        blocks[1] |= 0xFFFB_FFC0; // bits 6-31 except bit 18 (field 50, reserved)
        blocks[2] |= 0x3F; // bits 0-5

        // Parent 269 section (block 8): SpellCritPercentage[7] + ModDamageDonePos[7]
        // parent=269→bit13, SpellCrit[0-6]=270-276→bits14-20, ModDmgPos[0-6]=277-283→bits21-27
        blocks[8] |= (1 << 13) | (0x7F << 14) | (0x7F << 21);

        // CombatRatings[32]: parent bit 574 (block 17 bit 30), CR[0] bit 575 (block 17 bit 31)
        blocks[17] |= (1 << 30) | (1 << 31);
        // CR[1-31]: bits 576-606 → block 18 bits 0-30
        blocks[18] |= 0x7FFF_FFFF;
    }

    // Group masks (which blocks have changes)
    let mut group0: u32 = 0;
    let mut group1: u32 = 0;
    for i in 0..32 {
        if blocks[i] != 0 {
            group0 |= 1 << i;
        }
    }
    for i in 32..48 {
        if blocks[i] != 0 {
            group1 |= 1 << (i - 32);
        }
    }

    // C++ `UF::ActivePlayerData::WriteUpdate`: WriteUInt32 for group 0
    // (byte-aligned), then WriteBits for group 1 (16 bits).
    buf.write_uint32(group0);
    buf.write_bits(group1, 16);

    // Write block masks for blocks with changes
    for i in 0..48 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // First C++ FlushBits point. The supported runtime paths do not emit any
    // early bit payloads here (SortBags/InsertItems/KnownTitles/research).
    buf.flush_bits();

    // Second C++ dynamic-mask pass for parent-0 fields 4..19. Those fields are
    // outside this runtime writer, so no bits are emitted; keep this explicit
    // so future ActivePlayerData work does not collapse the C++ phases.
    buf.flush_bits();

    // Field values in C++ `UF::ActivePlayerData::WriteUpdate` order.

    // Block 0: Coinage (bit 28) — written before all other ActivePlayerData fields.
    // C++ `ActivePlayerData::Coinage` is written in the block-0 field pass.
    if let Some(coinage) = coinage_change {
        buf.write_int64(coinage as i64);
    }

    // Parent 0 section: expertise (bits 36-37) — BEFORE parent 38
    if let Some(sc) = stat_changes {
        buf.write_float(sc.mainhand_expertise); // bit 36: MainhandExpertise
        buf.write_float(sc.offhand_expertise); // bit 37: OffhandExpertise
    }

    // Parent 38 section: 30 fields (bits 39-49, 51-69) in C++ definition order.
    // Field bit 50 (ShieldBlockCritPercentage) is skipped in this represented
    // stats packet shape; see the mask above.
    if let Some(sc) = stat_changes {
        buf.write_float(sc.ranged_expertise); // bit 39: RangedExpertise
        buf.write_float(sc.combat_rating_expertise); // bit 40: CombatRatingExpertise
        buf.write_float(sc.block_pct); // bit 41: BlockPercentage
        buf.write_float(sc.dodge_pct); // bit 42: DodgePercentage
        buf.write_float(sc.dodge_from_attr); // bit 43: DodgePercentageFromAttribute
        buf.write_float(sc.parry_pct); // bit 44: ParryPercentage
        buf.write_float(sc.parry_from_attr); // bit 45: ParryPercentageFromAttribute
        buf.write_float(sc.crit_pct); // bit 46: CritPercentage
        buf.write_float(sc.ranged_crit_pct); // bit 47: RangedCritPercentage
        buf.write_float(sc.offhand_crit_pct); // bit 48: OffhandCritPercentage
        buf.write_int32(sc.shield_block); // bit 49: ShieldBlock
        // bit 50: ShieldBlockCritPercentage — RESERVED in the 54261 client grammar,
        // no property; never masked (see blocks[1] above) and never written here.
        buf.write_float(0.0); // bit 51: Mastery
        buf.write_float(0.0); // bit 52: Speed
        buf.write_float(0.0); // bit 53: Avoidance
        buf.write_float(0.0); // bit 54: Sturdiness
        buf.write_int32(0); // bit 55: Versatility
        buf.write_float(0.0); // bit 56: VersatilityBonus
        buf.write_float(0.0); // bit 57: PvpPowerDamage
        buf.write_float(0.0); // bit 58: PvpPowerHealing
        buf.write_int32(sc.spell_power); // bit 59: ModHealingDonePos
        buf.write_float(sc.mod_healing_pct); // bit 60: ModHealingPercent
        buf.write_float(sc.mod_healing_done_pct); // bit 61: ModHealingDonePercent
        buf.write_float(sc.mod_periodic_healing_pct); // bit 62: ModPeriodicHealingDonePercent
        buf.write_float(sc.mod_spell_power_pct); // bit 63: ModSpellPowerPercent
        buf.write_float(0.0); // bit 64: ModResiliencePercent
        buf.write_float(-1.0); // bit 65: OverrideSpellPowerByAPPercent
        buf.write_float(-1.0); // bit 66: OverrideAPBySpellPowerPercent
        buf.write_int32(0); // bit 67: ModTargetResistance
        buf.write_int32(0); // bit 68: ModTargetPhysicalResistance
        buf.write_uint32(0); // bit 69: LocalFlags
    }

    // Parent 124 section: InvSlots
    for slot in 0..141u8 {
        if let Some(&(_, ref guid)) = inv_slot_changes.iter().find(|&&(s, _)| s == slot) {
            buf.write_packed_guid(guid);
        }
    }

    // Parent 269 section: SpellCritPercentage[7] + ModDamageDonePos[7]
    // C++ interleaves SpellCritPct/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct per school.
    // Both SpellCritPct bits (270-276) and ModDmgDonePos bits (277-283) are set.
    if let Some(sc) = stat_changes {
        for i in 0..7 {
            buf.write_float(sc.spell_crit_pct[i]); // SpellCritPercentage[i]
            if i == 0 {
                buf.write_int32(0); // Physical school: no spell power
            } else {
                buf.write_int32(sc.spell_power); // Magic schools 1-6
            }
            // ModDamageDoneNeg[i] bits 284-290: NOT set → skip
            // ModDamageDonePercent[i] bits 291-297: NOT set → skip
        }
    }

    for slot in 94..106u8 {
        if let Some(&(_, price, timestamp)) = buyback_changes.iter().find(|&&(s, _, _)| s == slot) {
            buf.write_uint32(price);
            buf.write_int64(timestamp);
        }
    }

    // Parent 574 section: CombatRatings[0-31]
    if let Some(sc) = stat_changes {
        for i in 0..32 {
            buf.write_int32(sc.combat_ratings[i]);
        }
    }
}
