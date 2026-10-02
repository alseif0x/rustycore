//! Format packets.
//!
//! Separated from wdc4.rs under #691.

use super::*;

const WDC4_MAGIC: u32 = 0x3443_4457; // "WDC4" in little-endian
const WDC5_MAGIC: u32 = 0x3543_4457; // "WDC5" in little-endian

pub(super) const WDC4_HEADER_SIZE: usize = 72;
pub(super) const WDC5_HEADER_SIZE: usize = 204;

pub(super) const SECTION_HEADER_SIZE: usize = 40;

pub(super) const FIELD_META_SIZE: usize = 4;

pub(super) const FIELD_STORAGE_INFO_SIZE: usize = 24;

// ── Compression types ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum CompressionType {
    None = 0,
    Bitpacked = 1,
    Common = 2,
    Pallet = 3,
    PalletArray = 4,
    BitpackedSigned = 5,
}

impl CompressionType {
    pub(super) fn from_u32(v: u32) -> Result<Self> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Bitpacked),
            2 => Ok(Self::Common),
            3 => Ok(Self::Pallet),
            4 => Ok(Self::PalletArray),
            5 => Ok(Self::BitpackedSigned),
            _ => bail!("unknown compression type {v}"),
        }
    }
}

// ── Header structures ────────────────────────────────────────────────

#[derive(Debug)]
pub(super) struct Wdc4Header {
    pub(super) format_version: u32,
    pub(super) record_count: u32,
    pub(super) field_count: u32,
    pub(super) record_size: u32,
    pub(super) string_table_size: u32,
    pub(super) table_hash: u32,
    pub(super) _layout_hash: u32,
    pub(super) min_id: u32,
    pub(super) max_id: u32,
    pub(super) _locale: u32,
    pub(super) flags: u16,
    pub(super) id_index: u16,
    pub(super) total_field_count: u32,
    pub(super) _packed_data_offset: u32,
    pub(super) _lookup_column_count: u32,
    pub(super) _parent_lookup_count: u32,
    pub(super) field_storage_info_size: u32,
    pub(super) common_data_size: u32,
    pub(super) pallet_data_size: u32,
    pub(super) section_count: u32,
}

#[derive(Debug)]
pub(super) struct SectionHeader {
    pub(super) _tact_key_hash: u64,
    pub(super) file_offset: u32,
    pub(super) record_count: u32,
    pub(super) string_table_size: u32,
    pub(super) _offset_records_end: u32,
    pub(super) id_list_size: u32,
    pub(super) _relationship_data_size: u32,
    pub(super) _offset_map_id_count: u32,
    pub(super) copy_table_count: u32,
}

#[derive(Debug, Clone)]
pub(super) struct FieldStorageInfo {
    pub(super) field_offset_bits: u16,
    pub(super) field_size_bits: u16,
    pub(super) additional_data_size: u32,
    pub(super) compression: CompressionType,
    /// Pallet/PalletArray: pallet start offset (cumulative).
    /// Common: default_value.
    /// Bitpacked/None: bitpacking_offset_bits.
    pub(super) val1: u32,
    pub(super) val2: u32,
    pub(super) val3: u32,
}

pub(super) fn parse_header(data: &[u8]) -> Result<Wdc4Header> {
    ensure!(data.len() >= 4, "file too small for DB2 signature");
    let magic = read_u32_le(data, 0);
    match magic {
        WDC4_MAGIC => {
            ensure!(
                data.len() >= WDC4_HEADER_SIZE,
                "file too small for WDC4 header"
            );
            Ok(Wdc4Header {
                format_version: 4,
                record_count: read_u32_le(data, 4),
                field_count: read_u32_le(data, 8),
                record_size: read_u32_le(data, 12),
                string_table_size: read_u32_le(data, 16),
                table_hash: read_u32_le(data, 20),
                _layout_hash: read_u32_le(data, 24),
                min_id: read_u32_le(data, 28),
                max_id: read_u32_le(data, 32),
                _locale: read_u32_le(data, 36),
                flags: read_u16_le(data, 40),
                id_index: read_u16_le(data, 42),
                total_field_count: read_u32_le(data, 44),
                _packed_data_offset: read_u32_le(data, 48),
                _lookup_column_count: read_u32_le(data, 52),
                _parent_lookup_count: 0,
                field_storage_info_size: read_u32_le(data, 56),
                common_data_size: read_u32_le(data, 60),
                pallet_data_size: read_u32_le(data, 64),
                section_count: read_u32_le(data, 68),
            })
        }
        WDC5_MAGIC => {
            ensure!(
                data.len() >= WDC5_HEADER_SIZE,
                "file too small for WDC5 header"
            );
            let version = read_u32_le(data, 4);
            ensure!(version == 5, "unsupported WDC5 version {version}");
            let parent_lookup_count = read_u32_le(data, 184);
            ensure!(
                parent_lookup_count <= 1,
                "unsupported WDC5 parent lookup count {parent_lookup_count}"
            );
            Ok(Wdc4Header {
                format_version: 5,
                record_count: read_u32_le(data, 136),
                field_count: read_u32_le(data, 140),
                record_size: read_u32_le(data, 144),
                string_table_size: read_u32_le(data, 148),
                table_hash: read_u32_le(data, 152),
                _layout_hash: read_u32_le(data, 156),
                min_id: read_u32_le(data, 160),
                max_id: read_u32_le(data, 164),
                _locale: read_u32_le(data, 168),
                flags: read_u16_le(data, 172),
                // WDC5 stores IndexField as signed int16; -1 is represented by
                // the same sentinel used by the WDC4 reader.
                id_index: read_u16_le(data, 174),
                total_field_count: read_u32_le(data, 176),
                _packed_data_offset: read_u32_le(data, 180),
                _lookup_column_count: 0,
                _parent_lookup_count: parent_lookup_count,
                field_storage_info_size: read_u32_le(data, 188),
                common_data_size: read_u32_le(data, 192),
                pallet_data_size: read_u32_le(data, 196),
                section_count: read_u32_le(data, 200),
            })
        }
        _ => bail!("unsupported DB2 signature (magic=0x{magic:08X})"),
    }
}

pub(super) fn parse_section_header(data: &[u8]) -> SectionHeader {
    SectionHeader {
        _tact_key_hash: read_u64_le(data, 0),
        file_offset: read_u32_le(data, 8),
        record_count: read_u32_le(data, 12),
        string_table_size: read_u32_le(data, 16),
        _offset_records_end: read_u32_le(data, 20),
        id_list_size: read_u32_le(data, 24),
        _relationship_data_size: read_u32_le(data, 28),
        _offset_map_id_count: read_u32_le(data, 32),
        copy_table_count: read_u32_le(data, 36),
    }
}

pub(super) fn parse_field_storage_info(data: &[u8]) -> Result<FieldStorageInfo> {
    Ok(FieldStorageInfo {
        field_offset_bits: read_u16_le(data, 0),
        field_size_bits: read_u16_le(data, 2),
        additional_data_size: read_u32_le(data, 4),
        compression: CompressionType::from_u32(read_u32_le(data, 8))?,
        val1: read_u32_le(data, 12),
        val2: read_u32_le(data, 16),
        val3: read_u32_le(data, 20),
    })
}
