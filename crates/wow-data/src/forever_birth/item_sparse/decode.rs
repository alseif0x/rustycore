//! Source sparse offsets ignore DB2FieldEntry::Offset and compression columns:
//! walk variable inline strings + physical primitive arrays in metadata order.
use super::{SparseItemRecord, SparseItemStrings};
use anyhow::{Context, Result, ensure};

pub(super) const fn count(field: usize) -> usize {
    match field {
        14..=16 => 10,
        21 | 42 => 2,
        27 => 5,
        55 => 3,
        _ => 1,
    }
}
pub(super) const fn width(field: usize) -> usize {
    match field {
        0..=34 => 4,
        35..=51 => 2,
        _ => 1,
    }
}

pub(super) fn record(id: u32, bytes: &[u8]) -> Result<SparseItemRecord> {
    ensure!(
        bytes.len() <= u16::MAX as usize,
        "Sparse catalog record exceeds uint16 size"
    );
    // Private codec scratch, not retained domain state or a public data bag.
    let mut cells = [[0u32; 10]; 68];
    let mut strings: [Vec<u8>; 5] = std::array::from_fn(|_| Vec::new());
    let mut offset = 0usize;
    for (field, values) in cells.iter_mut().enumerate() {
        for value in values.iter_mut().take(count(field)) {
            if field < 5 {
                let tail = bytes
                    .get(offset..)
                    .context("Sparse string outside record")?;
                let length = tail
                    .iter()
                    .position(|byte| *byte == 0)
                    .context("Unterminated sparse item string")?;
                strings[field] = tail[..length].to_vec();
                offset += length + 1;
            } else {
                let size = width(field);
                let end = offset
                    .checked_add(size)
                    .context("Sparse item offset overflow")?;
                let source = bytes
                    .get(offset..end)
                    .context("Truncated sparse item numeric cell")?;
                let mut encoded = [0u8; 4];
                encoded[..size].copy_from_slice(source);
                *value = u32::from_le_bytes(encoded);
                offset = end;
            }
        }
    }
    // Source reads its fields without requiring consumption of catalog padding.
    // Our bounded slice prevents the source's unchecked cross-record reads.
    ensure!(offset <= bytes.len(), "Sparse item fields exceed record");
    let v = |field: usize| cells[field][0];
    let f = |field| f32::from_bits(v(field));
    Ok(SparseItemRecord {
        id,
        strings: SparseItemStrings::for_locale(6, strings),
        expansion: v(5) as i32,
        damage_variance: f(6),
        limit_category: v(7) as i32,
        duration: v(8),
        quality_modifier: f(9),
        bag_family: v(10),
        start_quest: v(11) as i32,
        language: v(12) as i32,
        item_range: f(13),
        socket_percentage: cells[14].map(f32::from_bits),
        stat_percent: cells[15].map(|n| n as i32),
        stat_bonus: cells[16].map(|n| n as i32),
        stackable: v(17) as i32,
        max_count: v(18) as i32,
        min_reputation: v(19) as i32,
        required_ability: v(20),
        allowable_race: u64::from(cells[21][0]) | (u64::from(cells[21][1]) << 32),
        sell_price: v(22),
        buy_price: v(23),
        vendor_stack: v(24),
        price_variance: f(25),
        price_random: f(26),
        flags: std::array::from_fn(|i| cells[27][i] as i32),
        faction_related: v(28) as i32,
        modified_crafting_reagent: v(29) as i32,
        content_tuning: v(30) as i32,
        player_level_curve: v(31) as i32,
        item_level_offset_curve: v(32) as i32,
        item_level_offset: v(33) as i32,
        squish_era: v(34) as i32,
        name_description: v(35) as u16,
        transmog_holiday: v(36) as u16,
        holiday: v(37) as u16,
        gem_properties: v(38) as u16,
        socket_enchantment: v(39) as u16,
        totem_category: v(40) as u16,
        instance_bound: v(41) as u16,
        zone_bound: [cells[42][0] as u16, cells[42][1] as u16],
        item_set: v(43) as u16,
        lock: v(44) as u16,
        page: v(45) as u16,
        delay: v(46) as u16,
        min_faction: v(47) as u16,
        required_skill_rank: v(48) as u16,
        required_skill: v(49) as u16,
        item_level: v(50) as u16,
        allowable_class: v(51) as i16,
        artifact: v(52) as u8,
        spell_weight: v(53) as u8,
        spell_weight_category: v(54) as u8,
        socket_type: std::array::from_fn(|i| cells[55][i] as u8),
        sheathe: v(56) as u8,
        material: v(57) as u8,
        page_material: v(58) as u8,
        bonding: v(59) as u8,
        damage_type: v(60) as u8,
        container_slots: v(61) as u8,
        required_pvp_medal: v(62) as u8,
        required_pvp_rank: v(63) as i8,
        required_level: v(64) as i8,
        inventory_type: v(65) as i8,
        quality: v(66) as i8,
        ammunition: v(67) as u8,
    })
}
