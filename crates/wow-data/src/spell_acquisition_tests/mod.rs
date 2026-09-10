//! Spell-acquisition regressions.
//!
//! Separated from the spell_acquisition_tests.rs root under #683.

//! Behaviour tests for [`super`].
//!
//! Extracted from `spell_acquisition.rs`, which was 4,540 lines of which
//! 1,668 — 37% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;

fn push_u16_le(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64_le(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn minimal_spell_effect_wdc4() -> Vec<u8> {
    const FIELD_COUNT: u32 = 29;
    const RECORD_SIZE: u32 = FIELD_COUNT * 4;
    const HEADER_SIZE: u32 = 72;
    const SECTION_HEADER_SIZE: u32 = 40;
    const FIELD_META_SIZE: u32 = 4;
    const FIELD_STORAGE_INFO_SIZE: u32 = 24;
    const RECORD_OFFSET: u32 = HEADER_SIZE
        + SECTION_HEADER_SIZE
        + FIELD_COUNT * FIELD_META_SIZE
        + FIELD_COUNT * FIELD_STORAGE_INFO_SIZE;

    let mut bytes = Vec::new();
    push_u32_le(&mut bytes, 0x3443_4457); // WDC4
    push_u32_le(&mut bytes, 1); // record_count
    push_u32_le(&mut bytes, FIELD_COUNT);
    push_u32_le(&mut bytes, RECORD_SIZE);
    push_u32_le(&mut bytes, 0); // string_table_size
    push_u32_le(&mut bytes, 0); // table_hash
    push_u32_le(&mut bytes, 0x6B64_DD7A); // C++ SpellEffectMeta layout
    push_u32_le(&mut bytes, 77);
    push_u32_le(&mut bytes, 77);
    push_u32_le(&mut bytes, 0); // locale
    push_u16_le(&mut bytes, 0x04); // external ID list
    push_u16_le(&mut bytes, u16::MAX); // no inline ID field
    push_u32_le(&mut bytes, FIELD_COUNT);
    push_u32_le(&mut bytes, 0); // packed_data_offset
    push_u32_le(&mut bytes, 0); // lookup_column_count
    push_u32_le(&mut bytes, FIELD_COUNT * FIELD_STORAGE_INFO_SIZE);
    push_u32_le(&mut bytes, 0); // common_data_size
    push_u32_le(&mut bytes, 0); // pallet_data_size
    push_u32_le(&mut bytes, 1); // section_count

    push_u64_le(&mut bytes, 0); // tact_key_hash
    push_u32_le(&mut bytes, RECORD_OFFSET);
    push_u32_le(&mut bytes, 1); // record_count
    push_u32_le(&mut bytes, 0); // string_table_size
    push_u32_le(&mut bytes, RECORD_OFFSET + RECORD_SIZE);
    push_u32_le(&mut bytes, 4); // id_list_size
    push_u32_le(&mut bytes, 0); // relationship_data_size
    push_u32_le(&mut bytes, 0); // offset_map_id_count
    push_u32_le(&mut bytes, 0); // copy_table_count

    bytes.resize(
        bytes.len() + FIELD_COUNT as usize * FIELD_META_SIZE as usize,
        0,
    );
    for field in 0..FIELD_COUNT {
        push_u16_le(&mut bytes, (field * 32) as u16);
        push_u16_le(&mut bytes, 32);
        push_u32_le(&mut bytes, 0); // additional_data_size
        push_u32_le(&mut bytes, 0); // CompressionType::None
        push_u32_le(&mut bytes, 0);
        push_u32_le(&mut bytes, 0);
        push_u32_le(&mut bytes, 0);
    }
    assert_eq!(bytes.len(), RECORD_OFFSET as usize);

    let mut fields = [0_u32; FIELD_COUNT as usize];
    fields[5] = 147; // EffectAura (int16 physical field)
    fields[4] = 1; // EffectAttributes::NoImmunity
    fields[9] = 9;
    fields[10] = 17; // EffectChainTargets
    fields[11] = 11;
    fields[12] = 12_345; // EffectItemType
    fields[13] = 23; // EffectMechanic
    fields[14] = 1.75_f32.to_bits(); // EffectPointsPerResource
    fields[15] = 15.0_f32.to_bits();
    fields[16] = (-2.5_f32).to_bits(); // EffectRealPointsPerLevel
    fields[17] = 17.0_f32.to_bits();
    for field in fields {
        push_u32_le(&mut bytes, field);
    }
    push_u32_le(&mut bytes, 77); // external record ID
    bytes
}

struct SentinelSpellEffectSqlSource {
    raw: [i64; 21],
    f32_bits: [u32; 21],
}

impl SpellEffectSqlFieldSourceLikeCpp for SentinelSpellEffectSqlSource {
    fn raw(&mut self, column: usize, _field: &'static str) -> i64 {
        self.raw[column]
    }

    fn f32_bits(&mut self, column: usize, _field: &'static str) -> u32 {
        self.f32_bits[column]
    }
}

fn effect(
    record_id: u32,
    spell_id: i64,
    difficulty_id: i64,
    effect_index: i64,
    effect_type: i64,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: spell_id,
        difficulty_id_raw: difficulty_id,
        effect_index_raw: effect_index,
        effect_type_raw: effect_type,
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: 0,
        effect_item_type_raw: 0,
        effect_misc_value_raw: [0, 0],
        implicit_target_raw: [0, 0],
    }
}

fn summon(
    record_id: u32,
    spell_id: u32,
    difficulty_id: u8,
    effect_index: u8,
    creature_id: i64,
    properties_id: i64,
) -> SpellAcquisitionEffectLikeCpp {
    let mut row = effect(
        record_id,
        i64::from(spell_id),
        i64::from(difficulty_id),
        i64::from(effect_index),
        i64::from(SPELL_EFFECT_SUMMON_LIKE_CPP),
    );
    row.effect_misc_value_raw = [creature_id, properties_id];
    row
}

fn catalog(
    coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
    rows: EffectiveSpellAcquisitionRowsLikeCpp,
) -> SpellAcquisitionCatalogLikeCpp {
    SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
        coverage,
        rows,
        SpellAcquisitionTableHashesLikeCpp::default(),
        Vec::new(),
    )
}

fn catalog_with_removed(
    coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
    rows: EffectiveSpellAcquisitionRowsLikeCpp,
    removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
) -> SpellAcquisitionCatalogLikeCpp {
    SpellAcquisitionCatalogLikeCpp::from_effective_rows_and_removed_like_cpp(
        coverage,
        rows,
        removed_rows,
        SpellAcquisitionTableHashesLikeCpp::default(),
        Vec::new(),
    )
}

mod scenarios_1;
mod scenarios_2;
