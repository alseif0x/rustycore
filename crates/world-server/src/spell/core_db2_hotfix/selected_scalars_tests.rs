use super::*;

#[test]
fn scalar_entry_mapping_preserves_source_payload_widths() {
    let aura_row = SpellAuraOptionsHotfixRow {
        id: u32::MAX,
        difficulty_id: u8::MAX,
        cumulative_aura: u32::MAX,
        proc_category_recovery: i32::MIN,
        proc_chance: u8::MAX,
        proc_charges: -7,
        spell_procs_per_minute_id: u16::MAX,
        proc_type_mask: [i32::MIN, -1],
        spell_id: u32::MAX,
    };
    assert_eq!(
        spell_aura_options_entry(aura_row),
        SpellAuraOptionsEntry {
            id: u32::MAX,
            difficulty_id: u8::MAX,
            cumulative_aura: u32::MAX,
            proc_category_recovery: i32::MIN,
            proc_chance: u8::MAX,
            proc_charges: -7,
            spell_procs_per_minute_id: u16::MAX,
            proc_type_mask: [i32::MIN, -1],
            spell_id: u32::MAX,
        }
    );

    let class_row = SpellClassOptionsHotfixRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        modal_next_spell: u32::MAX,
        spell_class_set: u8::MAX,
        spell_class_mask: [i32::MIN, -1, 0, i32::MAX],
    };
    assert_eq!(
        spell_class_options_entry(class_row),
        SpellClassOptionsEntry {
            id: u32::MAX,
            spell_id: i32::MIN,
            modal_next_spell: u32::MAX,
            spell_class_set: u8::MAX,
            spell_class_mask: [0x8000_0000, u32::MAX, 0, 0x7fff_ffff],
        }
    );

    let label_row = SpellLabelHotfixRow {
        id: u32::MAX,
        label_id: 0x8000_0000,
        spell_id: u32::MAX,
    };
    assert_eq!(
        spell_label_entry(label_row),
        SpellLabelEntry {
            id: u32::MAX,
            label_id: 0x8000_0000,
            spell_id: u32::MAX,
        }
    );

    let levels_row = SpellLevelsHotfixRow {
        id: u32::MAX,
        difficulty_id: u8::MAX,
        base_level: i16::MIN,
        max_level: -1,
        spell_level: -123,
        max_passive_aura_level: u8::MAX,
        spell_id: u32::MAX,
    };
    assert_eq!(
        spell_levels_entry(levels_row),
        SpellLevelsEntry {
            id: u32::MAX,
            difficulty_id: u8::MAX,
            base_level: i16::MIN,
            max_level: -1,
            spell_level: -123,
            max_passive_aura_level: u8::MAX,
            spell_id: u32::MAX,
        }
    );
}

#[test]
fn ppm_entry_mapping_keeps_float_bits_and_flags() {
    let rate = f32::from_bits(0x3F9E_0652);
    let entry = spell_procs_per_minute_entry(SpellProcsPerMinuteHotfixRow {
        id: u32::MAX,
        base_proc_rate: rate,
        flags: u8::MAX,
    });
    assert_eq!(entry.id, u32::MAX);
    assert_eq!(entry.base_proc_rate.to_bits(), rate.to_bits());
    assert_eq!(entry.flags, u8::MAX);
}
