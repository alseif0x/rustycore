use super::super::SpellTable;
use crate::wdc4::{CompressionType, FieldStorageInfo, Wdc4Header, Wdc4Reader};
use std::collections::HashMap;

pub(super) fn reader(table: SpellTable) -> Wdc4Reader {
    let schema = table.schema();
    let mut data = Vec::new();
    let mut fields = Vec::new();
    for (field, &(bits, count)) in schema.numeric.iter().take(schema.base.fields).enumerate() {
        let bits = if bits == 0 { 32 } else { bits };
        fields.push(FieldStorageInfo {
            field_offset_bits: (data.len() * 8) as u16,
            field_size_bits: (bits * count) as u16,
            additional_data_size: 0,
            compression: CompressionType::None,
            val1: 0,
            val2: 0,
            val3: 0,
        });
        let start = data.len();
        data.resize(start + bits * count / 8, 0);
        if schema.base.id == Some(field) {
            data[start..start + 4].copy_from_slice(&7u32.to_le_bytes());
        }
    }
    Wdc4Reader {
        header: Wdc4Header {
            format_version: 5,
            record_count: 1,
            field_count: schema.base.fields as u32,
            record_size: data.len() as u32,
            string_table_size: 0,
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
            section_count: 1,
        },
        field_info: fields,
        pallet_data: vec![Vec::new(); schema.base.fields],
        common_data: vec![HashMap::new(); schema.base.fields],
        record_data: data,
        record_ids: vec![7],
        copy_table: Vec::new(),
        id_to_index: HashMap::from([(7, 0)]),
        relationship_ids: vec![None],
        record_offsets: Vec::new(),
        record_sizes: Vec::new(),
        string_tables: Vec::new(),
        record_string_table_indices: vec![None],
    }
}

pub(super) fn cell(
    reader: &mut Wdc4Reader,
    field: usize,
    array_index: usize,
    bits: usize,
    value: u32,
) {
    let at = reader.field_info[field].field_offset_bits as usize / 8 + array_index * bits / 8;
    reader.record_data[at..at + bits / 8].copy_from_slice(&value.to_le_bytes()[..bits / 8]);
}

pub(super) fn word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn empty_file(table: SpellTable) -> Vec<u8> {
    let s = table.schema();
    let mut bytes = vec![0; 204 + s.base.fields * 4];
    word(&mut bytes, 0, 0x35434457);
    word(&mut bytes, 4, 5);
    word(&mut bytes, 140, s.base.fields as u32);
    word(&mut bytes, 152, s.base.hash);
    word(&mut bytes, 156, s.base.layout);
    word(&mut bytes, 168, s.locale_mask);
    word(
        &mut bytes,
        172,
        u32::from(s.flags) | (s.base.id.unwrap_or(0) as u32) << 16,
    );
    word(&mut bytes, 176, s.base.fields as u32);
    word(&mut bytes, 184, s.parent_lookups);
    bytes
}

/// Synthetic single-section file, exercising the public typed batch through
/// the production parser. No client rows or encrypted-section data is used.
pub(super) fn regular_file(table: SpellTable, value: Wdc4Reader) -> Vec<u8> {
    let schema = table.schema();
    let storage = 244 + schema.base.fields * 4;
    let records = storage + schema.base.fields * 24;
    let record_end = records + value.record_data.len();
    let external_id_bytes = if schema.base.id.is_none() { 4 } else { 0 };
    let mut bytes = empty_file(table);
    bytes.resize(record_end + external_id_bytes, 0);
    word(&mut bytes, 136, 1);
    word(&mut bytes, 144, value.header.record_size);
    word(&mut bytes, 164, 1024);
    word(&mut bytes, 188, schema.base.fields as u32 * 24);
    word(&mut bytes, 200, 1);
    word(&mut bytes, 212, records as u32);
    word(&mut bytes, 216, 1);
    word(&mut bytes, 224, record_end as u32);
    word(&mut bytes, 228, external_id_bytes as u32);
    for (index, field) in value.field_info.iter().enumerate() {
        let at = storage + index * 24;
        bytes[at..at + 2].copy_from_slice(&field.field_offset_bits.to_le_bytes());
        bytes[at + 2..at + 4].copy_from_slice(&field.field_size_bits.to_le_bytes());
    }
    bytes[records..record_end].copy_from_slice(&value.record_data);
    if external_id_bytes != 0 {
        word(&mut bytes, record_end, 7);
    }
    // 02245dcd DB2FileLoader.cpp:96-107,1977-1991: one parent
    // lookup header plus a (ParentId, RecordIndex) pair, after external IDs.
    if let Some(parent) = value.relationship_ids[0] {
        assert!(schema.base.parent.is_some());
        let at = bytes.len();
        bytes.resize(at + 20, 0);
        word(&mut bytes, 232, 20);
        word(&mut bytes, at, 1);
        word(&mut bytes, at + 4, parent);
        word(&mut bytes, at + 8, parent);
        word(&mut bytes, at + 12, parent);
        word(&mut bytes, at + 16, 0);
    }
    bytes
}
