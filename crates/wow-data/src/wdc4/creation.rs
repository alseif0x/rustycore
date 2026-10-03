//! Private checked numeric view for build-70170 creation data, not a new
//! generic permissive getter. Legacy readers retain their existing behavior.
//! 02245dcd DB2FileLoader.cpp:635-696,807-922: packed offset + compression,
//! typed array stride and parent override; copies consume materialized sources.
mod schema;
mod strings;
pub(crate) use schema::CreationTable;

use super::{CompressionType, Wdc4Reader};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, path::Path};

pub(crate) struct CreationDb2 {
    reader: Wdc4Reader,
    table: CreationTable,
    records: BTreeMap<u32, usize>,
}

impl CreationDb2 {
    pub(crate) fn open(directory: &Path, table: CreationTable) -> Result<Self> {
        let schema = table.schema();
        let reader = Wdc4Reader::open(&directory.join(format!("{}.db2", schema.name)))?;
        Self::checked(reader, table)
    }

    fn checked(reader: Wdc4Reader, table: CreationTable) -> Result<Self> {
        let schema = table.schema();
        ensure!(
            reader.format_version() == 5
                && reader.table_hash() == schema.hash
                && reader.layout_hash() == schema.layout
                && reader.field_count() == schema.fields
                && reader.declared_field_count() as usize == schema.fields
                && reader.inline_id_field() == schema.id
                && reader.parent_lookup_count() == u32::from(schema.parent.is_some()),
            "Wrong creation DB2 schema"
        );
        validate_blobs(&reader)?;
        let mut records = BTreeMap::new();
        for (index, &id) in reader.record_ids.iter().enumerate() {
            ensure!(
                reader.record_bytes(index).is_some(),
                "Creation record absent"
            );
            ensure!(
                records.insert(id, index).is_none(),
                "Duplicate creation DB2 ID"
            );
            if let Some(field) = schema.id {
                ensure!(
                    numeric(&reader, index, field, 0, 32, 1)? == id,
                    "Creation inline ID reader mismatch"
                );
            }
        }
        // C++ AutoProduceRecordCopies walks file order. A prior copy can be
        // a source; a missing/forward/cyclic source is not manufactured here.
        for &(new, source) in &reader.copy_table {
            ensure!(source != 0, "Unsupported zero copy source");
            let index = *records.get(&source).context("Unresolved creation copy")?;
            ensure!(
                records.insert(new, index).is_none(),
                "Duplicate creation copy ID"
            );
        }
        // GetMaxId (02245dcd DB2FileLoader.cpp:955-977) includes copy IDs
        // and asserts that the actual maximum is within the header bound.
        ensure!(
            records
                .keys()
                .next_back()
                .is_none_or(|&id| id <= reader.header.max_id),
            "Creation IDs exceed declared maximum"
        );
        ensure!(
            records.len() == reader.total_count(),
            "Incomplete creation IDs"
        );
        Ok(Self {
            reader,
            table,
            records,
        })
    }

    pub(crate) fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.records.keys().copied()
    }

    pub(crate) fn bits(&self, id: u32, field: usize, array_index: usize) -> Result<u32> {
        let index = *self.records.get(&id).context("Creation record ID absent")?;
        let schema = self.table.schema();
        let (bits, count) = self
            .table
            .numeric(field)
            .context("Unported creation numeric field")?;
        ensure!(array_index < count, "Creation array index outside schema");
        if schema.id == Some(field) {
            return Ok(id);
        }
        if schema.parent == Some(field) {
            if let Some(parent) = self.reader.get_relationship_id(index) {
                ensure!(
                    bits == 32 || parent < (1u32 << bits),
                    "Parent field exceeds source type"
                );
                return Ok(parent);
            }
            if field >= schema.fields {
                // Extra parent fields are zero-initialized before FillParentLookup.
                return Ok(0);
            }
        }
        numeric(&self.reader, index, field, array_index, bits, count)
    }
}

fn validate_blobs(reader: &Wdc4Reader) -> Result<()> {
    let mut palette_bytes = 0u64;
    let mut common_bytes = 0u64;
    for (field, info) in reader.field_info.iter().enumerate() {
        match info.compression {
            CompressionType::Pallet | CompressionType::PalletArray => {
                ensure!(
                    info.additional_data_size % 4 == 0
                        && reader.pallet_data.get(field).is_some_and(
                            |data| data.len() == info.additional_data_size as usize / 4
                        ),
                    "Invalid creation palette blob"
                );
                palette_bytes += u64::from(info.additional_data_size);
                if info.compression == CompressionType::PalletArray {
                    ensure!(
                        info.val3 != 0 && (info.additional_data_size / 4) % info.val3 == 0,
                        "Invalid creation palette cardinality"
                    );
                }
            }
            CompressionType::Common => {
                ensure!(
                    info.additional_data_size % 8 == 0
                        && reader.common_data.get(field).is_some_and(
                            |data| data.len() == info.additional_data_size as usize / 8
                        ),
                    "Invalid/duplicate creation common blob"
                );
                common_bytes += u64::from(info.additional_data_size);
            }
            _ => ensure!(
                info.additional_data_size == 0,
                "Unexpected creation field blob"
            ),
        }
    }
    ensure!(
        palette_bytes == u64::from(reader.header.pallet_data_size)
            && common_bytes == u64::from(reader.header.common_data_size),
        "Incomplete creation column blobs"
    );
    Ok(())
}

fn packed(reader: &Wdc4Reader, index: usize, field: usize) -> Result<u32> {
    let info = reader
        .field_info
        .get(field)
        .context("Creation column absent")?;
    let offset = usize::try_from(reader.header._packed_data_offset)?
        .checked_mul(8)
        .and_then(|base| base.checked_add(info.val1 as usize))
        .context("Creation packed offset overflow")?;
    // Source RecordGetPackedValue supports <=64; creation numeric values here
    // are <=32-bit. Larger packed widths remain an explicit unsupported case.
    ensure!(
        (1..=32).contains(&info.val2),
        "Unsupported creation packed width"
    );
    checked_bits(reader, index, offset, info.val2 as usize)
}

fn checked_bits(reader: &Wdc4Reader, index: usize, offset: usize, width: usize) -> Result<u32> {
    let bytes = reader
        .record_bytes(index)
        .context("Creation record absent")?;
    let end = offset
        .checked_add(width)
        .context("Creation bit range overflow")?;
    ensure!(
        width <= 32
            && end
                <= bytes
                    .len()
                    .checked_mul(8)
                    .context("Creation record size overflow")?,
        "Creation field exceeds record"
    );
    let mut value = 0u32;
    for bit in 0..width {
        value |= u32::from((bytes[(offset + bit) / 8] >> ((offset + bit) % 8)) & 1) << bit;
    }
    Ok(value)
}

fn numeric(
    reader: &Wdc4Reader,
    index: usize,
    field: usize,
    array_index: usize,
    bits: usize,
    count: usize,
) -> Result<u32> {
    ensure!(
        array_index < count && matches!(bits, 8 | 16 | 32),
        "Unsupported creation numeric type"
    );
    reader
        .record_bytes(index)
        .context("Creation record absent")?;
    let info = reader
        .field_info
        .get(field)
        .context("Creation column absent")?;
    let value = match info.compression {
        CompressionType::None => {
            ensure!(info.field_offset_bits % 8 == 0, "Unaligned creation field");
            let offset = usize::from(info.field_offset_bits)
                .checked_add(array_index * bits)
                .context("Creation array offset overflow")?;
            checked_bits(reader, index, offset, bits)?
        }
        CompressionType::Bitpacked | CompressionType::BitpackedSigned => {
            ensure!(
                count == 1 && array_index == 0 && info.val2 != 0,
                "Unsupported creation immediate array/zero-width"
            );
            let raw = packed(reader, index, field)?;
            if info.compression == CompressionType::BitpackedSigned {
                super::sign_extend(raw, info.val2) as u32
            } else {
                raw
            }
        }
        CompressionType::Pallet | CompressionType::PalletArray => {
            let cardinality = if info.compression == CompressionType::Pallet {
                ensure!(
                    count == 1 && array_index == 0,
                    "Unsupported creation scalar palette array"
                );
                1
            } else {
                ensure!(
                    info.val3 as usize == count,
                    "Creation palette array schema mismatch"
                );
                count
            };
            let offset = (packed(reader, index, field)? as usize)
                .checked_mul(cardinality)
                .and_then(|base| base.checked_add(array_index))
                .context("Creation palette index overflow")?;
            *reader
                .pallet_data
                .get(field)
                .and_then(|data| data.get(offset))
                .context("Creation palette index absent")?
        }
        CompressionType::Common => {
            ensure!(
                count == 1 && array_index == 0,
                "Unsupported creation common array"
            );
            let id = *reader
                .record_ids
                .get(index)
                .context("Creation source record ID absent")?;
            *reader
                .common_data
                .get(field)
                .context("Creation common column absent")?
                .get(&id)
                .unwrap_or(&info.val1)
        }
    };
    Ok(if bits == 32 {
        value
    } else {
        value & ((1u32 << bits) - 1)
    })
}

#[cfg(test)]
#[path = "creation/tests.rs"]
mod tests;
