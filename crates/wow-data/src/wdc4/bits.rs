//! Bits packets.
//!
//! Separated from wdc4.rs under #691.

use super::*;

// ── Parsing helpers ──────────────────────────────────────────────────

pub(super) fn read_u16_le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

pub(super) fn read_u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

pub(super) fn read_u64_le(data: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
        data[off + 4],
        data[off + 5],
        data[off + 6],
        data[off + 7],
    ])
}

pub(super) fn read_inline_record_id(
    record_data: &[u8],
    record_offsets: &[usize],
    record_size: usize,
    field_info: &[FieldStorageInfo],
    pallet_data: &[Vec<u32>],
    record_idx: usize,
    field: usize,
) -> Result<u32> {
    let info = &field_info[field];
    let record_start = record_offsets
        .get(record_idx)
        .copied()
        .unwrap_or(record_idx * record_size);

    match info.compression {
        CompressionType::None | CompressionType::Bitpacked | CompressionType::BitpackedSigned => {
            Ok(read_bits(
                record_data,
                record_start,
                info.field_offset_bits as usize,
                info.field_size_bits as usize,
            ))
        }
        CompressionType::Pallet => {
            let index = read_bits(
                record_data,
                record_start,
                info.field_offset_bits as usize,
                info.field_size_bits as usize,
            ) as usize;
            pallet_data
                .get(field)
                .and_then(|pallet| pallet.get(index))
                .copied()
                .with_context(|| {
                    format!(
                        "inline id field {field} pallet index {index} is outside its value table"
                    )
                })
        }
        CompressionType::PalletArray => {
            let index = read_bits(
                record_data,
                record_start,
                info.field_offset_bits as usize,
                info.field_size_bits as usize,
            ) as usize;
            let cardinality = info.val3.max(1) as usize;
            pallet_data
                .get(field)
                .and_then(|pallet| pallet.get(index * cardinality))
                .copied()
                .with_context(|| {
                    format!(
                        "inline id field {field} pallet-array index {index} is outside its value table"
                    )
                })
        }
        CompressionType::Common => {
            bail!("inline WDC4 id field {field} cannot use Common compression")
        }
    }
}

/// Split the concatenated pallet data blob into per-field Vec<u32>.
pub(super) fn split_pallet_data(raw: &[u8], fields: &[FieldStorageInfo]) -> Vec<Vec<u32>> {
    let mut result = Vec::with_capacity(fields.len());
    let mut offset = 0usize;

    for info in fields {
        if matches!(
            info.compression,
            CompressionType::Pallet | CompressionType::PalletArray
        ) {
            let size = info.additional_data_size as usize;
            let count = size / 4;
            let mut values = Vec::with_capacity(count);
            for i in 0..count {
                let o = offset + i * 4;
                if o + 4 <= raw.len() {
                    values.push(read_u32_le(raw, o));
                }
            }
            result.push(values);
            offset += size;
        } else {
            result.push(Vec::new());
        }
    }
    result
}

/// Split the concatenated common data blob into per-field HashMap<record_id, u32>.
pub(super) fn split_common_data(raw: &[u8], fields: &[FieldStorageInfo]) -> Vec<HashMap<u32, u32>> {
    let mut result = Vec::with_capacity(fields.len());
    let mut offset = 0usize;

    for info in fields {
        if info.compression == CompressionType::Common {
            let size = info.additional_data_size as usize;
            // Common data format: repeated (record_id: u32, value: u32)
            let count = size / 8;
            let mut map = HashMap::with_capacity(count);
            for i in 0..count {
                let o = offset + i * 8;
                if o + 8 <= raw.len() {
                    let record_id = read_u32_le(raw, o);
                    let value = read_u32_le(raw, o + 4);
                    map.insert(record_id, value);
                }
            }
            result.push(map);
            offset += size;
        } else {
            result.push(HashMap::new());
        }
    }
    result
}

pub(super) fn merge_relationship_data(
    raw: &[u8],
    base_record_index: usize,
    relationship_ids: &mut Vec<Option<u32>>,
) -> Result<()> {
    if raw.is_empty() {
        return Ok(());
    }

    ensure!(raw.len() >= 12, "relationship_data too small");
    let count = read_u32_le(raw, 0) as usize;
    let entries_start = 12usize;
    ensure!(
        raw.len() >= entries_start + count * 8,
        "relationship_data truncated"
    );

    for i in 0..count {
        let entry_offset = entries_start + i * 8;
        let relationship_id = read_u32_le(raw, entry_offset);
        let record_index = base_record_index + read_u32_le(raw, entry_offset + 4) as usize;
        if relationship_ids.len() <= record_index {
            relationship_ids.resize(record_index + 1, None);
        }
        relationship_ids[record_index] = Some(relationship_id);
    }

    Ok(())
}

/// Read `bit_count` bits from `record_data` starting at byte offset `record_start`
/// plus `bit_offset` bits within the record.
pub(super) fn read_bits(
    record_data: &[u8],
    record_start: usize,
    bit_offset: usize,
    bit_count: usize,
) -> u32 {
    if bit_count == 0 || bit_count > 32 {
        return 0;
    }

    let abs_bit = record_start * 8 + bit_offset;
    let byte_start = abs_bit / 8;
    let bit_start = abs_bit % 8;

    // Read enough bytes to cover all bits we need
    let bytes_needed = (bit_start + bit_count + 7) / 8;
    let mut val: u64 = 0;
    for i in 0..bytes_needed.min(8) {
        let idx = byte_start + i;
        if idx < record_data.len() {
            val |= u64::from(record_data[idx]) << (i * 8);
        }
    }

    // Shift right to skip the starting bits, then mask
    let shifted = val >> bit_start;
    let mask = if bit_count >= 32 {
        u32::MAX
    } else {
        (1u32 << bit_count) - 1
    };
    (shifted as u32) & mask
}

/// Sign-extend a value from `bits` width to i32.
pub(super) fn sign_extend(value: u32, bits: u32) -> i32 {
    if bits == 0 || bits >= 32 {
        return value as i32;
    }
    let sign_bit = 1u32 << (bits - 1);
    if (value & sign_bit) != 0 {
        // Set all high bits
        let mask = !((1u32 << bits) - 1);
        (value | mask) as i32
    } else {
        value as i32
    }
}
