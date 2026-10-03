use super::{SpellPrefix, contract};
use crate::wdc4::{Wdc4Header, creation::SpellTable, format::SectionHeader};

fn fixture(table: SpellTable, c: SpellPrefix) -> (Wdc4Header, Vec<SectionHeader>) {
    let schema = table.schema();
    let header = Wdc4Header {
        format_version: 5,
        record_count: c.records,
        field_count: schema.base.fields as u32,
        record_size: c.record_bytes,
        string_table_size: c.strings,
        table_hash: schema.base.hash,
        _layout_hash: schema.base.layout,
        min_id: 0,
        max_id: 1024,
        _locale: schema.locale_mask,
        flags: schema.flags,
        id_index: schema.base.id.unwrap_or(0) as u16,
        total_field_count: schema.base.fields as u32,
        _packed_data_offset: 0,
        _lookup_column_count: 0,
        _parent_lookup_count: schema.parent_lookups,
        field_storage_info_size: schema.base.fields as u32 * 24,
        common_data_size: 0,
        pallet_data_size: 0,
        section_count: c.sections,
    };
    let sections = (0..c.sections)
        .map(|i| {
            let records = if i == 0 {
                c.known
            } else if i == 1 {
                c.records - c.known
            } else {
                0
            };
            SectionHeader {
                _tact_key_hash: if i == 0 { 0 } else { 1 }, // synthetic marker, not a key
                file_offset: if i == 0 { c.first } else { c.bytes + i - 1 },
                record_count: records,
                string_table_size: if i == 0 { c.strings } else { 0 },
                _offset_records_end: 0,
                id_list_size: if schema.base.id.is_none() {
                    records * 4
                } else {
                    0
                },
                _relationship_data_size: if schema.parent_lookups == 1 {
                    12 + records * 8
                } else {
                    0
                },
                _offset_map_id_count: 0,
                copy_table_count: if i == 0 { c.copies } else { 0 },
            }
        })
        .collect();
    (header, sections)
}

#[test]
fn only_twenty_four_target_spell_tables_have_prefix_admission() {
    let mut count = 0;
    for table in SpellTable::ALL {
        if let Some(c) = contract(table) {
            let (header, sections) = fixture(table, c);
            c.validate(table, &header, &sections, c.bytes as usize)
                .unwrap();
            assert!(c.unknown() > 0);
            assert!(
                c.validate(table, &header, &sections, c.bytes as usize - 1)
                    .is_err()
            );
            assert!(
                c.validate(table, &header, &sections, c.bytes as usize + 1)
                    .is_err()
            );
            count += 1;
        }
    }
    assert_eq!(count, 24);
    assert!(contract(SpellTable::SpellShapeshift).is_none());
    assert!(contract(SpellTable::SpellLearnSpell).is_none());
    assert!(contract(SpellTable::Talent).is_none());
    assert!(contract(SpellTable::LiquidType).is_none());
}

#[test]
fn spell_prefixes_reject_schema_locale_key_count_and_boundary_drift() {
    for table in SpellTable::ALL {
        let Some(c) = contract(table) else {
            continue;
        };
        for mutation in 0..13 {
            let (mut header, mut sections) = fixture(table, c);
            match mutation {
                0 => header.table_hash ^= 1,
                1 => header._layout_hash ^= 1,
                2 => header._locale ^= 1,
                3 => header.flags ^= 1,
                4 => header.record_size ^= 1,
                5 => header._parent_lookup_count ^= 1,
                6 => sections[0]._tact_key_hash = 1,
                7 => sections[1]._tact_key_hash = 0,
                8 => sections[1].file_offset -= 1,
                9 => sections[0].copy_table_count += 1,
                10 => sections[0].id_list_size ^= 4,
                11 => sections[0]._relationship_data_size ^= 1,
                12 => sections[1].record_count += 1,
                _ => unreachable!(),
            }
            assert!(
                c.validate(table, &header, &sections, c.bytes as usize)
                    .is_err(),
                "{} drift {}",
                table.name(),
                mutation
            );
        }
    }
}
