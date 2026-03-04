// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Generic WDC4 (DB2) file parser.
//!
//! Supports the six compression types used in WoW 3.4.3 client data files:
//! None, Immediate (Bitpacked), SignedImmediate, Pallet, PalletArray, Common.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use tracing::{debug, trace};

// ── Constants ────────────────────────────────────────────────────────

const WDC4_MAGIC: u32 = 0x3443_4457; // "WDC4" in little-endian
const HEADER_SIZE: usize = 72;
const SECTION_HEADER_SIZE: usize = 40;
const FIELD_META_SIZE: usize = 4;
const FIELD_STORAGE_INFO_SIZE: usize = 24;

// ── Compression types ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum CompressionType {
    None = 0,
    Bitpacked = 1,
    Common = 2,
    Pallet = 3,
    PalletArray = 4,
    BitpackedSigned = 5,
}

impl CompressionType {
    fn from_u32(v: u32) -> Result<Self> {
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
struct Wdc4Header {
    record_count: u32,
    field_count: u32,
    record_size: u32,
    string_table_size: u32,
    table_hash: u32,
    _layout_hash: u32,
    min_id: u32,
    max_id: u32,
    _locale: u32,
    flags: u16,
    _id_index: u16,
    total_field_count: u32,
    _packed_data_offset: u32,
    _lookup_column_count: u32,
    field_storage_info_size: u32,
    common_data_size: u32,
    pallet_data_size: u32,
    section_count: u32,
}

#[derive(Debug)]
struct SectionHeader {
    _tact_key_hash: u64,
    file_offset: u32,
    record_count: u32,
    string_table_size: u32,
    _offset_records_end: u32,
    id_list_size: u32,
    _relationship_data_size: u32,
    _offset_map_id_count: u32,
    copy_table_count: u32,
}

#[derive(Debug, Clone)]
struct FieldStorageInfo {
    field_offset_bits: u16,
    field_size_bits: u16,
    additional_data_size: u32,
    compression: CompressionType,
    /// Pallet/PalletArray: pallet start offset (cumulative).
    /// Common: default_value.
    /// Bitpacked/None: bitpacking_offset_bits.
    val1: u32,
    val2: u32,
    val3: u32,
}

// ── Reader ───────────────────────────────────────────────────────────

/// Parsed WDC4 file ready for field access.
pub struct Wdc4Reader {
    header: Wdc4Header,
    field_info: Vec<FieldStorageInfo>,
    /// Per-field pallet data: field_index → Vec<u32>
    pallet_data: Vec<Vec<u32>>,
    /// Per-field common data: field_index → HashMap<record_id, u32>
    common_data: Vec<HashMap<u32, u32>>,
    /// Concatenated record data bytes (all sections).
    record_data: Vec<u8>,
    /// Record ID for each record index (from id_list or inline).
    record_ids: Vec<u32>,
    /// Copy table: (new_id, source_id) pairs.
    copy_table: Vec<(u32, u32)>,
    /// Map from record_id → record_index for fast lookup.
    id_to_index: HashMap<u32, usize>,
    /// For offset-map files: byte offset of each record within record_data.
    /// Empty for non-offset-map files (fixed-size records use record_idx * record_size).
    record_offsets: Vec<usize>,
    /// Byte size of each record (variable for offset-map, uniform for fixed-size).
    record_sizes: Vec<usize>,
}

impl Wdc4Reader {
    /// Open and parse a WDC4 file.
    pub fn open(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        ensure!(data.len() >= HEADER_SIZE, "file too small for WDC4 header");

        let header = parse_header(&data)?;
        debug!(
            "WDC4: records={}, fields={}, record_size={}, sections={}, table_hash=0x{:08X}",
            header.record_count, header.field_count, header.record_size,
            header.section_count, header.table_hash
        );

        let section_count = header.section_count as usize;
        let field_count = header.total_field_count.max(header.field_count) as usize;

        // Parse section headers
        let mut offset = HEADER_SIZE;
        let mut sections = Vec::with_capacity(section_count);
        for _ in 0..section_count {
            ensure!(offset + SECTION_HEADER_SIZE <= data.len(), "truncated section header");
            sections.push(parse_section_header(&data[offset..]));
            offset += SECTION_HEADER_SIZE;
        }

        // Parse field meta (unused directly — field_storage_info has all we need)
        let _field_meta_end = offset + field_count * FIELD_META_SIZE;
        offset = _field_meta_end;

        // Parse field storage info
        let fsi_count = header.field_storage_info_size as usize / FIELD_STORAGE_INFO_SIZE;
        ensure!(fsi_count == field_count, "field_storage_info count ({fsi_count}) != field_count ({field_count})");
        let mut field_info = Vec::with_capacity(fsi_count);
        for _ in 0..fsi_count {
            ensure!(offset + FIELD_STORAGE_INFO_SIZE <= data.len(), "truncated field_storage_info");
            field_info.push(parse_field_storage_info(&data[offset..])?);
            offset += FIELD_STORAGE_INFO_SIZE;
        }

        // Parse pallet data
        let pallet_end = offset + header.pallet_data_size as usize;
        ensure!(pallet_end <= data.len(), "truncated pallet data");
        let pallet_raw = &data[offset..pallet_end];
        let pallet_data = split_pallet_data(pallet_raw, &field_info);
        offset = pallet_end;

        // Parse common data
        let common_end = offset + header.common_data_size as usize;
        ensure!(common_end <= data.len(), "truncated common data");
        let common_raw = &data[offset..common_end];
        let common_data = split_common_data(common_raw, &field_info);
        offset = common_end;

        // Now read section data
        let has_id_list = (header.flags & 0x04) != 0;
        let has_offset_map = (header.flags & 0x01) != 0;
        let record_size = header.record_size as usize;

        let mut record_data = Vec::new();
        let mut record_ids = Vec::new();
        let mut copy_table = Vec::new();
        let mut record_offsets: Vec<usize> = Vec::new();
        let mut record_sizes: Vec<usize> = Vec::new();

        for (si, sec) in sections.iter().enumerate() {
            if sec.record_count == 0 && sec.copy_table_count == 0 {
                continue;
            }

            let sec_offset = sec.file_offset as usize;
            let base_data_len = record_data.len();

            // Determine record data bounds and post-record cursor
            let after_records = if has_offset_map {
                // Offset-map: records are variable-length, end at offset_records_end
                let rec_end = sec._offset_records_end as usize;
                ensure!(rec_end <= data.len(), "section {si} record data truncated (offset_map)");
                record_data.extend_from_slice(&data[sec_offset..rec_end]);
                rec_end
            } else {
                // Fixed-size records
                let rec_bytes = sec.record_count as usize * record_size;
                let rec_end = sec_offset + rec_bytes;
                ensure!(rec_end <= data.len(), "section {si} record data truncated");
                record_data.extend_from_slice(&data[sec_offset..rec_end]);
                rec_end + sec.string_table_size as usize
            };

            // ID list
            let id_list_end = after_records + sec.id_list_size as usize;
            if has_id_list && sec.id_list_size > 0 {
                ensure!(id_list_end <= data.len(), "section {si} id_list truncated");
                let id_count = sec.id_list_size as usize / 4;
                for i in 0..id_count {
                    let id_off = after_records + i * 4;
                    record_ids.push(read_u32_le(&data, id_off));
                }
            } else if !has_offset_map {
                let base_idx = base_data_len / record_size;
                for i in 0..sec.record_count {
                    record_ids.push(header.min_id + base_idx as u32 + i);
                }
            }

            let mut cursor = id_list_end;

            // Copy table
            if sec.copy_table_count > 0 {
                let copy_bytes = sec.copy_table_count as usize * 8;
                let copy_end = cursor + copy_bytes;
                ensure!(copy_end <= data.len(), "section {si} copy_table truncated");
                for i in 0..sec.copy_table_count as usize {
                    let co = cursor + i * 8;
                    copy_table.push((read_u32_le(&data, co), read_u32_le(&data, co + 4)));
                }
                cursor = copy_end;
            }

            // Offset map entries + offset map ID list (only for offset-map files)
            //
            // WDC4 offset-map section layout after copy_table:
            //   1. offset_map_entries: offset_map_id_count × 6 bytes (u32 offset, u16 size)
            //   2. relationship_data: _relationship_data_size bytes
            //   3. offset_map_id_list: offset_map_id_count × 4 bytes (u32 record_id)
            //
            // offset_map_entries[i] and offset_map_id_list[i] correspond 1:1.
            // offset_map_entries with size=0 indicate non-existent records.
            if has_offset_map && sec._offset_map_id_count > 0 {
                let om_count = sec._offset_map_id_count as usize;

                // Parse offset map entries
                let om_bytes = om_count * 6;
                let om_end = cursor + om_bytes;
                ensure!(om_end <= data.len(), "section {si} offset_map truncated");

                let mut om_entries: Vec<(u32, u16)> = Vec::with_capacity(om_count);
                for i in 0..om_count {
                    let om_off = cursor + i * 6;
                    let file_off = read_u32_le(&data, om_off);
                    let rec_sz = read_u16_le(&data, om_off + 4);
                    om_entries.push((file_off, rec_sz));
                }
                cursor = om_end;

                // Skip relationship data
                cursor += sec._relationship_data_size as usize;

                // Parse offset map ID list
                let om_id_bytes = om_count * 4;
                let om_id_end = cursor + om_id_bytes;
                ensure!(om_id_end <= data.len(), "section {si} offset_map_id_list truncated");

                // Build per-ID offset+size mapping, then populate record_ids + record_offsets
                // using the ID list order (which matches the actual record order in data)
                let mut id_to_om_info: HashMap<u32, (u32, u16)> = HashMap::with_capacity(om_count);
                for i in 0..om_count {
                    let om_id = read_u32_le(&data, cursor + i * 4);
                    let (file_off, rec_sz) = om_entries[i];
                    if rec_sz > 0 {
                        id_to_om_info.insert(om_id, (file_off, rec_sz));
                    }
                }
                cursor = om_id_end;

                // If the ID list was already loaded, use it to set up offsets
                // in the correct order. Otherwise, build from offset map.
                if record_ids.len() > record_offsets.len() {
                    // IDs were loaded from id_list — match each to its offset+size
                    let start_idx = record_offsets.len();
                    for idx in start_idx..record_ids.len() {
                        let id = record_ids[idx];
                        if let Some(&(file_off, rec_sz)) = id_to_om_info.get(&id) {
                            let data_relative = (file_off as usize)
                                .saturating_sub(sec_offset) + base_data_len;
                            record_offsets.push(data_relative);
                            record_sizes.push(rec_sz as usize);
                        } else {
                            // Shouldn't happen for valid data
                            record_offsets.push(0);
                            record_sizes.push(0);
                        }
                    }
                } else {
                    // No id_list — build both lists from offset map
                    for i in 0..om_count {
                        let om_id = read_u32_le(&data, cursor - om_id_bytes + i * 4);
                        let (file_off, rec_sz) = om_entries[i];
                        if rec_sz > 0 {
                            record_ids.push(om_id);
                            let data_relative = (file_off as usize)
                                .saturating_sub(sec_offset) + base_data_len;
                            record_offsets.push(data_relative);
                            record_sizes.push(rec_sz as usize);
                        }
                    }
                }
            } else if !has_offset_map {
                // Fixed-size records: offsets are sequential
                for i in 0..sec.record_count as usize {
                    record_offsets.push(base_data_len + i * record_size);
                    record_sizes.push(record_size);
                }
            }

            trace!(
                "  section {si}: {} records, {} copies, id_list={}, offset_map={}",
                sec.record_count, sec.copy_table_count, sec.id_list_size,
                sec._offset_map_id_count
            );
        }

        // Build id→index map
        let mut id_to_index = HashMap::with_capacity(record_ids.len());
        for (idx, &id) in record_ids.iter().enumerate() {
            id_to_index.insert(id, idx);
        }

        debug!(
            "WDC4: loaded {} records + {} copies = {} total, {} pallet fields, offset_map={}",
            record_ids.len(),
            copy_table.len(),
            record_ids.len() + copy_table.len(),
            pallet_data.iter().filter(|p| !p.is_empty()).count(),
            has_offset_map,
        );

        Ok(Self {
            header,
            field_info,
            pallet_data,
            common_data,
            record_data,
            record_ids,
            copy_table,
            id_to_index,
            record_offsets,
            record_sizes,
        })
    }

    /// Total number of unique records (excluding copies).
    pub fn record_count(&self) -> usize {
        self.record_ids.len()
    }

    /// Total number of accessible records (including copies).
    pub fn total_count(&self) -> usize {
        self.record_ids.len() + self.copy_table.len()
    }

    /// Get the record ID for a given record index.
    pub fn record_id(&self, record_idx: usize) -> u32 {
        self.record_ids[record_idx]
    }

    /// Read a field as u32 from a record index.
    pub fn get_field_u32(&self, record_idx: usize, field: usize) -> u32 {
        self.read_field(record_idx, field)
    }

    /// Read a field as i32 from a record index.
    pub fn get_field_i32(&self, record_idx: usize, field: usize) -> i32 {
        self.read_field(record_idx, field) as i32
    }

    /// Read a field as u8 from a record index.
    pub fn get_field_u8(&self, record_idx: usize, field: usize) -> u8 {
        self.read_field(record_idx, field) as u8
    }

    /// Read a field as u16 from a record index.
    pub fn get_field_u16(&self, record_idx: usize, field: usize) -> u16 {
        self.read_field(record_idx, field) as u16
    }

    /// Read a field as i16 from a record index (sign-extended if compression supports it).
    pub fn get_field_i16(&self, record_idx: usize, field: usize) -> i16 {
        let info = &self.field_info[field];
        if info.compression == CompressionType::BitpackedSigned {
            let raw = self.read_field(record_idx, field);
            let bits = info.field_size_bits as u32;
            sign_extend(raw, bits) as i16
        } else {
            self.read_field(record_idx, field) as i16
        }
    }

    /// Read a field as i8 from a record index (sign-extended if compression supports it).
    pub fn get_field_i8(&self, record_idx: usize, field: usize) -> i8 {
        let info = &self.field_info[field];
        if info.compression == CompressionType::BitpackedSigned {
            let raw = self.read_field(record_idx, field);
            let bits = info.field_size_bits as u32;
            sign_extend(raw, bits) as i8
        } else {
            self.read_field(record_idx, field) as i8
        }
    }

    /// Read a field as i64 from a record index.
    ///
    /// For 64-bit fields (e.g. RaceMask), reads two 32-bit halves from the
    /// record data and combines them into a single i64.
    pub fn get_field_i64(&self, record_idx: usize, field: usize) -> i64 {
        let info = &self.field_info[field];
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };

        let bit_offset = info.field_offset_bits as usize;
        let lo = read_bits(&self.record_data, record_start, bit_offset, 32) as u64;
        let hi = read_bits(&self.record_data, record_start, bit_offset + 32, 32) as u64;
        ((hi << 32) | lo) as i64
    }

    /// Read an element from an array field.
    ///
    /// Array fields in WDC4 are stored as a single field with
    /// `field_size_bits = element_count * element_bits`. This method reads
    /// a single element at `array_index` within the field.
    pub fn get_array_element(&self, record_idx: usize, field: usize, array_index: usize, element_bits: usize) -> u32 {
        let info = &self.field_info[field];
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };
        let bit_offset = info.field_offset_bits as usize + array_index * element_bits;
        read_bits(&self.record_data, record_start, bit_offset, element_bits)
    }

    /// Read an array element as i16 (for short[] arrays like StatModifierBonusAmount).
    pub fn get_array_i16(&self, record_idx: usize, field: usize, array_index: usize) -> i16 {
        self.get_array_element(record_idx, field, array_index, 16) as i16
    }

    /// Read an array element as i8 (for sbyte[] arrays like StatModifierBonusStat).
    pub fn get_array_i8(&self, record_idx: usize, field: usize, array_index: usize) -> i8 {
        self.get_array_element(record_idx, field, array_index, 8) as i8
    }

    /// Number of fields in this DB2 file.
    pub fn field_count(&self) -> usize {
        self.field_info.len()
    }

    /// Get the record index for a given record ID.
    pub fn get_record_index(&self, record_id: u32) -> Option<usize> {
        self.id_to_index.get(&record_id).copied()
    }

    /// Debug: describe a field's compression and bit layout.
    pub fn field_info_debug(&self, field: usize) -> String {
        if field >= self.field_info.len() {
            return format!("field {field} out of range");
        }
        let info = &self.field_info[field];
        format!(
            "field[{field}]: offset={}bits, size={}bits, compression={:?}",
            info.field_offset_bits, info.field_size_bits, info.compression,
        )
    }

    /// Get raw bytes for a record by index.
    ///
    /// For offset-map files (variable-length records), this returns the
    /// exact bytes for that record. For fixed-size records, returns
    /// `record_size` bytes.
    pub fn record_bytes(&self, record_idx: usize) -> Option<&[u8]> {
        if record_idx >= self.record_ids.len() {
            return None;
        }
        let start = if record_idx < self.record_offsets.len() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };
        let size = if record_idx < self.record_sizes.len() {
            self.record_sizes[record_idx]
        } else {
            self.header.record_size as usize
        };
        if size == 0 || start + size > self.record_data.len() {
            return None;
        }
        Some(&self.record_data[start..start + size])
    }

    /// Return the table hash from the DB2 header.
    pub fn table_hash(&self) -> u32 {
        self.header.table_hash
    }

    /// Iterate over all records including copies: yields (record_id, record_index).
    ///
    /// For copy table entries, the record_index points to the source record data.
    pub fn iter_records(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        let direct = self.record_ids.iter().enumerate().map(|(idx, &id)| (id, idx));
        let copies = self.copy_table.iter().filter_map(|&(new_id, source_id)| {
            self.id_to_index.get(&source_id).map(|&idx| (new_id, idx))
        });
        direct.chain(copies)
    }

    // ── Internal ─────────────────────────────────────────────────────

    fn read_field(&self, record_idx: usize, field: usize) -> u32 {
        let info = &self.field_info[field];
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };

        match info.compression {
            CompressionType::None | CompressionType::Bitpacked | CompressionType::BitpackedSigned => {
                read_bits(
                    &self.record_data,
                    record_start,
                    info.field_offset_bits as usize,
                    info.field_size_bits as usize,
                )
            }
            CompressionType::Pallet => {
                let index = read_bits(
                    &self.record_data,
                    record_start,
                    info.field_offset_bits as usize,
                    info.field_size_bits as usize,
                ) as usize;
                self.pallet_data.get(field)
                    .and_then(|p| p.get(index))
                    .copied()
                    .unwrap_or(0)
            }
            CompressionType::PalletArray => {
                let index = read_bits(
                    &self.record_data,
                    record_start,
                    info.field_offset_bits as usize,
                    info.field_size_bits as usize,
                ) as usize;
                let cardinality = info.val3.max(1) as usize;
                self.pallet_data.get(field)
                    .and_then(|p| p.get(index * cardinality))
                    .copied()
                    .unwrap_or(0)
            }
            CompressionType::Common => {
                let record_id = self.record_ids.get(record_idx).copied().unwrap_or(0);
                self.common_data.get(field)
                    .and_then(|m| m.get(&record_id))
                    .copied()
                    .unwrap_or(info.val1) // val1 = default_value for Common
            }
        }
    }
}

// ── Parsing helpers ──────────────────────────────────────────────────

fn read_u16_le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn read_u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn read_u64_le(data: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([
        data[off], data[off + 1], data[off + 2], data[off + 3],
        data[off + 4], data[off + 5], data[off + 6], data[off + 7],
    ])
}

fn parse_header(data: &[u8]) -> Result<Wdc4Header> {
    let magic = read_u32_le(data, 0);
    ensure!(magic == WDC4_MAGIC, "not a WDC4 file (magic=0x{magic:08X})");

    Ok(Wdc4Header {
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
        _id_index: read_u16_le(data, 42),
        total_field_count: read_u32_le(data, 44),
        _packed_data_offset: read_u32_le(data, 48),
        _lookup_column_count: read_u32_le(data, 52),
        field_storage_info_size: read_u32_le(data, 56),
        common_data_size: read_u32_le(data, 60),
        pallet_data_size: read_u32_le(data, 64),
        section_count: read_u32_le(data, 68),
    })
}

fn parse_section_header(data: &[u8]) -> SectionHeader {
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

fn parse_field_storage_info(data: &[u8]) -> Result<FieldStorageInfo> {
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

/// Split the concatenated pallet data blob into per-field Vec<u32>.
fn split_pallet_data(raw: &[u8], fields: &[FieldStorageInfo]) -> Vec<Vec<u32>> {
    let mut result = Vec::with_capacity(fields.len());
    let mut offset = 0usize;

    for info in fields {
        if matches!(info.compression, CompressionType::Pallet | CompressionType::PalletArray) {
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
fn split_common_data(raw: &[u8], fields: &[FieldStorageInfo]) -> Vec<HashMap<u32, u32>> {
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

/// Read `bit_count` bits from `record_data` starting at byte offset `record_start`
/// plus `bit_offset` bits within the record.
fn read_bits(record_data: &[u8], record_start: usize, bit_offset: usize, bit_count: usize) -> u32 {
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
    let mask = if bit_count >= 32 { u32::MAX } else { (1u32 << bit_count) - 1 };
    (shifted as u32) & mask
}

/// Sign-extend a value from `bits` width to i32.
fn sign_extend(value: u32, bits: u32) -> i32 {
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

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits_simple() {
        // Byte 0 = 0b1010_0101
        let data = vec![0xA5];
        // Read 4 bits from bit 0: should be 0b0101 = 5
        assert_eq!(read_bits(&data, 0, 0, 4), 5);
        // Read 4 bits from bit 4: should be 0b1010 = 10
        assert_eq!(read_bits(&data, 0, 4, 4), 10);
        // Read 8 bits from bit 0: should be 0xA5
        assert_eq!(read_bits(&data, 0, 0, 8), 0xA5);
    }

    #[test]
    fn test_read_bits_cross_byte() {
        let data = vec![0xFF, 0x00];
        // Read 4 bits starting at bit 6: crosses byte boundary
        // Byte 0 bits 6-7 = 11, Byte 1 bits 0-1 = 00 → 0b0011 = 3
        assert_eq!(read_bits(&data, 0, 6, 4), 3);
    }

    #[test]
    fn test_read_bits_multi_byte() {
        let data = vec![0x12, 0x34, 0x56];
        // Read 16 bits from bit 0
        assert_eq!(read_bits(&data, 0, 0, 16), 0x3412);
    }

    #[test]
    fn test_sign_extend() {
        // 5-bit value 0b11111 = 31 → sign-extended = -1
        assert_eq!(sign_extend(0x1F, 5), -1);
        // 5-bit value 0b01111 = 15 → positive = 15
        assert_eq!(sign_extend(0x0F, 5), 15);
        // 8-bit value 0xFF → -1
        assert_eq!(sign_extend(0xFF, 8), -1);
        // 8-bit value 0x7F → 127
        assert_eq!(sign_extend(0x7F, 8), 127);
    }

    #[test]
    fn test_compression_type_from_u32() {
        assert_eq!(CompressionType::from_u32(0).unwrap(), CompressionType::None);
        assert_eq!(CompressionType::from_u32(3).unwrap(), CompressionType::Pallet);
        assert!(CompressionType::from_u32(99).is_err());
    }

    // Integration test: parse real Item.db2 if available
    #[test]
    fn test_parse_item_db2() {
        let path = std::path::Path::new("/home/server/woltk-server-core/Data/dbc/esES/Item.db2");
        if !path.exists() {
            eprintln!("Skipping test: Item.db2 not found");
            return;
        }

        let reader = Wdc4Reader::open(path).expect("failed to parse Item.db2");
        assert!(reader.record_count() > 0, "should have records");
        assert!(reader.total_count() > 20000, "expected >20k total items");

        // Verify a known item: Thunderfury (entry 19019)
        // class_id=2 (Weapon), subclass_id=7 (Swords), inventory_type=13 (Weapon/One-Hand)
        let mut found_any = false;
        for (id, idx) in reader.iter_records() {
            if id == 19019 {
                let class_id = reader.get_field_u8(idx, 0);
                let subclass_id = reader.get_field_u8(idx, 1);
                let inv_type = reader.get_field_i8(idx, 3);
                assert_eq!(class_id, 2, "Thunderfury class should be 2 (Weapon)");
                assert_eq!(subclass_id, 7, "Thunderfury subclass should be 7 (Sword)");
                assert_eq!(inv_type, 13, "Thunderfury inv_type should be 13 (One-Hand)");
                found_any = true;
                break;
            }
        }
        // At minimum, verify we can read fields without panicking
        if !found_any {
            let (id, idx) = reader.iter_records().next().unwrap();
            let _ = reader.get_field_u8(idx, 0);
            let _ = reader.get_field_i8(idx, 3);
            eprintln!("Thunderfury not found, first record id={id}");
        }
    }

    /// Diagnostic test: probe ItemSparse.db2 field layout to find stat modifier fields.
    #[test]
    fn test_probe_item_sparse_db2() {
        let path = std::path::Path::new(
            "/home/server/woltk-server-core/Data/dbc/esES/ItemSparse.db2",
        );
        if !path.exists() {
            eprintln!("Skipping test: ItemSparse.db2 not found");
            return;
        }

        let reader = Wdc4Reader::open(path).expect("failed to parse ItemSparse.db2");
        eprintln!("ItemSparse: {} records, {} total, {} fields",
            reader.record_count(), reader.total_count(), reader.field_count());

        // Print all field info with byte offsets
        let mut prev_end_bits = 0u32;
        for i in 0..reader.field_count() {
            let info = &reader.field_info[i];
            let start_byte = info.field_offset_bits / 8;
            let end_byte = (info.field_offset_bits + info.field_size_bits + 7) / 8;
            let gap = if info.field_offset_bits as u32 > prev_end_bits {
                format!(" GAP={}bits", info.field_offset_bits as u32 - prev_end_bits)
            } else { String::new() };
            eprintln!(
                "  f[{:2}] byte {:3}..{:3} ({:4}bits) {:?}{}",
                i, start_byte, end_byte, info.field_size_bits, info.compression, gap
            );
            prev_end_bits = info.field_offset_bits as u32 + info.field_size_bits as u32;
        }
        eprintln!("Total record bit-width: {prev_end_bits} ({} bytes)", prev_end_bits / 8);

        // f[53] = StatModifierBonusAmount[10] (i16[10], 160 bits)
        // f[65] = _statModifierBonusStat[10] (i8[10], 80 bits)
        // f[45] = ItemLevel (u16)
        // f[69] = _inventoryType (i8)
        // Verify with known items
        eprintln!("record_ids={}, record_offsets={}", reader.record_ids.len(), reader.record_offsets.len());
        for &check_id in &[6948u32, 49623, 19364, 19019] {
            if let Some(idx) = reader.get_record_index(check_id) {
                let offset = reader.record_offsets[idx];
                let item_level = reader.get_field_u16(idx, 45);
                let inv_type = reader.get_field_i8(idx, 69);
                eprintln!("\n=== Item {check_id} (idx={idx}, offset={offset}, iLvl={item_level}, invType={inv_type}) ===");
                for i in 0..10 {
                    let stat_type = reader.get_array_i8(idx, 65, i);
                    let stat_amount = reader.get_array_i16(idx, 53, i);
                    if stat_type != 0 || stat_amount != 0 {
                        eprintln!("  slot[{i}]: type={stat_type:3}, amount={stat_amount:5}");
                    }
                }
            }
        }
    }
}

#[test]
fn test_find_record_58268() {
    let path = std::path::Path::new(
        "/home/server/woltk-server-core/Data/dbc/esES/ItemSparse.db2",
    );
    if !path.exists() { return; }
    let reader = Wdc4Reader::open(path).expect("failed to parse");
    let found = reader.iter_records().any(|(id, _)| id == 58268);
    eprintln!("Record 58268 in ItemSparse: {found}");
    // Also check nearby
    let nearby: Vec<u32> = reader.iter_records()
        .map(|(id, _)| id)
        .filter(|&id| id >= 58260 && id <= 58280)
        .collect();
    eprintln!("Records 58260-58280: {:?}", {let mut v=nearby; v.sort(); v});
}
