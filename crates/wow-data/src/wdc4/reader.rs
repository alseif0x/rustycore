//! Reader packets.
//!
//! Separated from wdc4.rs under #691.

use super::*;

#[derive(Clone, Copy)]
enum AvailablePrefix {
    Achievement,
    InitialMap,
    BirthAbilities,
    Item(super::available::items::ItemPrefix),
    Spell(super::creation::SpellTable),
}

// ── Reader ───────────────────────────────────────────────────────────

/// Parsed WDC4 or bounded regular WDC5 file ready for field access.
pub struct Wdc4Reader {
    pub(super) header: Wdc4Header,
    pub(super) field_info: Vec<FieldStorageInfo>,
    /// Per-field pallet data: field_index → Vec<u32>
    pub(super) pallet_data: Vec<Vec<u32>>,
    /// Per-field common data: field_index → HashMap<record_id, u32>
    pub(super) common_data: Vec<HashMap<u32, u32>>,
    /// Concatenated record data bytes (all sections).
    pub(super) record_data: Vec<u8>,
    /// Record ID for each record index (from id_list or inline).
    pub(super) record_ids: Vec<u32>,
    /// Copy table: (new_id, source_id) pairs.
    pub(super) copy_table: Vec<(u32, u32)>,
    /// Map from record_id → record_index for fast lookup.
    pub(super) id_to_index: HashMap<u32, usize>,
    /// Parent/relationship id by record index, when present in WDC4 relationship data.
    pub(super) relationship_ids: Vec<Option<u32>>,
    /// For offset-map files: byte offset of each record within record_data.
    /// Empty for non-offset-map files (fixed-size records use record_idx * record_size).
    pub(super) record_offsets: Vec<usize>,
    /// Byte size of each record (variable for offset-map, uniform for fixed-size).
    pub(super) record_sizes: Vec<usize>,
    /// Per-section string tables, used by non-localized string fields.
    pub(super) string_tables: Vec<Vec<u8>>,
    /// String table index for each direct record.
    pub(super) record_string_table_indices: Vec<Option<usize>>,
}

impl Wdc4Reader {
    /// Open WDC4 or bounded regular WDC5. Full target-schema checks belong to
    /// the typed table consumer, not a generic header's declared hash alone.
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_inner(path, None)
    }

    /// Pinned build-70170 Achievement acquisition: retain only its readable
    /// prefix. C++ DB2FileLoader::LoadTableData skips unknown TACT sections;
    /// no inaccessible bytes/IDs are replaced with zeros or declared present.
    pub(crate) fn open_available_achievement(path: &Path) -> Result<Self> {
        Self::open_inner(path, Some(AvailablePrefix::Achievement))
    }

    pub(crate) fn open_available_initial_map(path: &Path) -> Result<Self> {
        Self::open_inner(path, Some(AvailablePrefix::InitialMap))
    }

    pub(crate) fn open_available_birth_abilities(path: &Path) -> Result<Self> {
        Self::open_inner(path, Some(AvailablePrefix::BirthAbilities))
    }

    pub(in crate::wdc4) fn open_available_item(
        path: &Path,
        table: super::available::items::ItemPrefix,
    ) -> Result<Self> {
        Self::open_inner(path, Some(AvailablePrefix::Item(table)))
    }

    fn open_inner(path: &Path, available: Option<AvailablePrefix>) -> Result<Self> {
        Self::open_bounded_inner(path, available, None)
    }

    pub(in crate::wdc4) fn open_spell_table(path: &Path) -> Result<Self> {
        Self::open_bounded_inner(path, None, Some(4 * 1024 * 1024))
    }

    pub(in crate::wdc4) fn open_available_spell(
        path: &Path,
        table: super::creation::SpellTable,
    ) -> Result<Self> {
        Self::open_bounded_inner(
            path,
            Some(AvailablePrefix::Spell(table)),
            Some(4 * 1024 * 1024),
        )
    }

    fn open_bounded_inner(
        path: &Path,
        available: Option<AvailablePrefix>,
        byte_limit: Option<usize>,
    ) -> Result<Self> {
        let byte_limit = match available {
            Some(AvailablePrefix::Item(table)) => Some(table.byte_limit()),
            _ => byte_limit,
        };
        let data = if let Some(limit) = byte_limit {
            use std::io::Read;
            let file =
                std::fs::File::open(path).context("Cannot open private target item prefix")?;
            let mut bytes = Vec::new();
            file.take(limit as u64 + 1)
                .read_to_end(&mut bytes)
                .context("Cannot read bounded target item prefix")?;
            ensure!(bytes.len() <= limit, "Target DB2 read exceeds bound");
            bytes
        } else {
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?
        };

        ensure!(
            data.len() >= WDC4_HEADER_SIZE,
            "file too small for DB2 header"
        );

        let header = parse_header(&data)?;
        debug!(
            "WDC{}: records={}, fields={}, record_size={}, sections={}, table_hash=0x{:08X}",
            header.format_version,
            header.record_count,
            header.field_count,
            header.record_size,
            header.section_count,
            header.table_hash
        );

        ensure!(
            header.format_version != 5 || header.flags & 0x01 == 0,
            "WDC5 sparse files are not supported by the regular reader"
        );

        let header_size = match header.format_version {
            4 => WDC4_HEADER_SIZE,
            5 => WDC5_HEADER_SIZE,
            version => bail!("unsupported DB2 format version {version}"),
        };
        let section_count = header.section_count as usize;
        let section_bytes = section_count
            .checked_mul(SECTION_HEADER_SIZE)
            .context("WDC section header size overflow")?;
        ensure!(
            header_size
                .checked_add(section_bytes)
                .is_some_and(|end| end <= data.len()),
            "truncated section headers"
        );

        // Parse section headers
        let mut offset = header_size;
        let mut sections = Vec::with_capacity(section_count);
        for _ in 0..section_count {
            ensure!(
                offset + SECTION_HEADER_SIZE <= data.len(),
                "truncated section header"
            );
            sections.push(parse_section_header(&data[offset..]));
            offset += SECTION_HEADER_SIZE;
        }

        if header.format_version == 5 {
            if let Some(prefix) = available {
                match prefix {
                    AvailablePrefix::Achievement => {
                        super::available::validate_prefix(&header, &sections, data.len())?
                    }
                    AvailablePrefix::InitialMap => {
                        super::available::validate_initial_map(&header, &sections, data.len())?
                    }
                    AvailablePrefix::BirthAbilities => {
                        super::available::validate_birth_abilities(&header, &sections, data.len())?
                    }
                    AvailablePrefix::Item(table) => {
                        table.validate(&header, &sections, data.len())?
                    }
                    AvailablePrefix::Spell(table) => {
                        super::available::spells::contract(table)
                            .context("Not an admitted spell prefix")?
                            .validate(table, &header, &sections, data.len())?;
                    }
                }
            } else {
                ensure!(
                    sections.iter().all(|section| section._tact_key_hash == 0),
                    "WDC5 TACT/encrypted sections are not supported"
                );
            }
            ensure!(
                sections
                    .iter()
                    .map(|section| u64::from(section.record_count))
                    .sum::<u64>()
                    == u64::from(header.record_count),
                "WDC5 section record counts do not match the header"
            );
        }
        ensure!(
            available.is_none() || header.format_version == 5,
            "Available target prefix requires WDC5"
        );

        let has_no_records = header.record_count == 0
            && sections
                .iter()
                .all(|section| section.record_count == 0 && section.copy_table_count == 0);
        if has_no_records && header.field_storage_info_size == 0 {
            return Ok(Self {
                header,
                field_info: Vec::new(),
                pallet_data: Vec::new(),
                common_data: Vec::new(),
                record_data: Vec::new(),
                record_ids: Vec::new(),
                copy_table: Vec::new(),
                id_to_index: HashMap::new(),
                relationship_ids: Vec::new(),
                record_offsets: Vec::new(),
                record_sizes: Vec::new(),
                string_tables: Vec::new(),
                record_string_table_indices: Vec::new(),
            });
        }

        // Parse field meta (unused directly — column metadata has the offsets
        // and compression data needed by this reader). WDC5 deliberately uses
        // FieldCount for this 4-byte table and TotalFieldCount for columns;
        // taking their maximum here would shift the following metadata.
        let field_meta_count = header.field_count as usize;
        let field_meta_bytes = field_meta_count
            .checked_mul(FIELD_META_SIZE)
            .context("DB2 field metadata size overflow")?;
        let field_meta_end = offset
            .checked_add(field_meta_bytes)
            .context("DB2 field metadata offset overflow")?;
        ensure!(field_meta_end <= data.len(), "truncated field metadata");
        offset = field_meta_end;

        // Parse column metadata. WDC4 historically used one storage record per
        // max(field_count,total_field_count); WDC5's count is exact and its
        // byte size must describe complete 24-byte columns.
        ensure!(
            header.field_storage_info_size as usize % FIELD_STORAGE_INFO_SIZE == 0,
            "DB2 column metadata size is not divisible by 24"
        );
        let fsi_count = header.field_storage_info_size as usize / FIELD_STORAGE_INFO_SIZE;
        let expected_fsi_count = if header.format_version == 5 {
            header.total_field_count as usize
        } else {
            header.total_field_count.max(header.field_count) as usize
        };
        ensure!(
            fsi_count == expected_fsi_count,
            "column metadata count ({fsi_count}) != expected field count ({expected_fsi_count})"
        );
        let storage_end = offset
            .checked_add(header.field_storage_info_size as usize)
            .context("DB2 column metadata offset overflow")?;
        ensure!(storage_end <= data.len(), "truncated column metadata");
        let mut field_info = Vec::with_capacity(fsi_count);
        for _ in 0..fsi_count {
            ensure!(
                offset + FIELD_STORAGE_INFO_SIZE <= data.len(),
                "truncated column metadata"
            );
            field_info.push(parse_field_storage_info(&data[offset..])?);
            offset += FIELD_STORAGE_INFO_SIZE;
        }

        // Parse pallet data
        let pallet_end = offset
            .checked_add(header.pallet_data_size as usize)
            .context("DB2 pallet data offset overflow")?;
        ensure!(pallet_end <= data.len(), "truncated pallet data");
        let pallet_raw = &data[offset..pallet_end];
        let pallet_data = split_pallet_data(pallet_raw, &field_info);
        offset = pallet_end;

        // Parse common data
        let common_end = offset
            .checked_add(header.common_data_size as usize)
            .context("DB2 common data offset overflow")?;
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
        let mut relationship_ids: Vec<Option<u32>> = Vec::new();
        let mut string_tables: Vec<Vec<u8>> = Vec::new();
        let mut record_string_table_indices: Vec<Option<usize>> = Vec::new();

        if header.format_version == 5 && header.record_count > 0 {
            ensure!(header.record_size > 0, "WDC5 records have zero record size");
        }

        for (si, sec) in sections.iter().enumerate() {
            if available.is_some() && sec._tact_key_hash != 0 {
                continue;
            }
            if sec.record_count == 0 && sec.copy_table_count == 0 {
                continue;
            }

            let sec_offset = sec.file_offset as usize;
            let base_data_len = record_data.len();

            // Determine record data bounds and post-record cursor
            let mut section_string_table_index = None;
            let after_records = if has_offset_map {
                // Offset-map: records are variable-length, end at offset_records_end
                let rec_end = sec._offset_records_end as usize;
                ensure!(
                    rec_end <= data.len(),
                    "section {si} record data truncated (offset_map)"
                );
                ensure!(
                    sec_offset <= rec_end,
                    "section {si} offset-map range reversed"
                );
                record_data.extend_from_slice(&data[sec_offset..rec_end]);
                if sec.string_table_size > 0 {
                    let string_end = rec_end
                        .checked_add(sec.string_table_size as usize)
                        .context("section string table offset overflow")?;
                    ensure!(
                        string_end <= data.len(),
                        "section {si} string table truncated (offset_map)"
                    );
                    string_tables.push(data[rec_end..string_end].to_vec());
                    section_string_table_index = Some(string_tables.len() - 1);
                }
                rec_end
            } else {
                // Fixed-size records
                let rec_bytes = (sec.record_count as usize)
                    .checked_mul(record_size)
                    .context("section record data size overflow")?;
                let rec_end = sec_offset
                    .checked_add(rec_bytes)
                    .context("section record data offset overflow")?;
                ensure!(
                    sec_offset <= data.len(),
                    "section {si} record offset out of bounds"
                );
                ensure!(rec_end <= data.len(), "section {si} record data truncated");
                record_data.extend_from_slice(&data[sec_offset..rec_end]);
                let string_end = rec_end
                    .checked_add(sec.string_table_size as usize)
                    .context("section string table offset overflow")?;
                ensure!(
                    string_end <= data.len(),
                    "section {si} string table truncated"
                );
                if sec.string_table_size > 0 {
                    string_tables.push(data[rec_end..string_end].to_vec());
                    section_string_table_index = Some(string_tables.len() - 1);
                }
                string_end
            };

            // ID list
            let id_list_end = after_records
                .checked_add(sec.id_list_size as usize)
                .context("section id list offset overflow")?;
            if header.format_version == 5 && has_id_list {
                ensure!(
                    u64::from(sec.id_list_size) == u64::from(sec.record_count) * 4,
                    "WDC5 external id list must contain exactly one id per record"
                );
            }
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
                    // Fixed-record WDC4 files without an external ID list can
                    // carry their IDs in the field selected by `id_index`.
                    // Keep one placeholder per record until every section's
                    // bytes and offsets have been assembled, then decode that
                    // inline field before Common-compressed values use the IDs
                    // as lookup keys.
                    if header.id_index != u16::MAX {
                        record_ids.push(0);
                    } else {
                        record_ids.push(header.min_id + base_idx as u32 + i);
                    }
                }
            }

            let mut cursor = id_list_end;

            // Copy table
            if sec.copy_table_count > 0 {
                let copy_bytes = (sec.copy_table_count as usize)
                    .checked_mul(8)
                    .context("section copy table size overflow")?;
                let copy_end = cursor
                    .checked_add(copy_bytes)
                    .context("section copy table offset overflow")?;
                ensure!(copy_end <= data.len(), "section {si} copy_table truncated");
                for i in 0..sec.copy_table_count as usize {
                    let co = cursor + i * 8;
                    copy_table.push((read_u32_le(&data, co), read_u32_le(&data, co + 4)));
                }
                cursor = copy_end;
            }

            if !has_offset_map && sec._relationship_data_size > 0 {
                let rel_end = cursor
                    .checked_add(sec._relationship_data_size as usize)
                    .context("section relationship data offset overflow")?;
                ensure!(
                    rel_end <= data.len(),
                    "section {si} relationship_data truncated"
                );
                merge_relationship_data(
                    &data[cursor..rel_end],
                    base_data_len / record_size,
                    &mut relationship_ids,
                )?;
                cursor = rel_end;
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
                let om_bytes = om_count
                    .checked_mul(6)
                    .context("section offset map size overflow")?;
                let om_end = cursor
                    .checked_add(om_bytes)
                    .context("section offset map offset overflow")?;
                ensure!(om_end <= data.len(), "section {si} offset_map truncated");

                let mut om_entries: Vec<(u32, u16)> = Vec::with_capacity(om_count);
                for i in 0..om_count {
                    let om_off = cursor + i * 6;
                    let file_off = read_u32_le(&data, om_off);
                    let rec_sz = read_u16_le(&data, om_off + 4);
                    om_entries.push((file_off, rec_sz));
                }
                cursor = om_end;

                if sec._relationship_data_size > 0 {
                    let rel_end = cursor
                        .checked_add(sec._relationship_data_size as usize)
                        .context("section relationship data offset overflow")?;
                    ensure!(
                        rel_end <= data.len(),
                        "section {si} relationship_data truncated"
                    );
                    merge_relationship_data(
                        &data[cursor..rel_end],
                        base_data_len / record_size.max(1),
                        &mut relationship_ids,
                    )?;
                    cursor = rel_end;
                }

                // Parse offset map ID list
                let om_id_bytes = om_count
                    .checked_mul(4)
                    .context("section offset map id-list size overflow")?;
                let om_id_end = cursor
                    .checked_add(om_id_bytes)
                    .context("section offset map id-list offset overflow")?;
                ensure!(
                    om_id_end <= data.len(),
                    "section {si} offset_map_id_list truncated"
                );

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
                            let data_relative =
                                (file_off as usize).saturating_sub(sec_offset) + base_data_len;
                            record_offsets.push(data_relative);
                            record_sizes.push(rec_sz as usize);
                        } else {
                            // Shouldn't happen for valid data
                            record_offsets.push(0);
                            record_sizes.push(0);
                        }
                        record_string_table_indices.push(section_string_table_index);
                    }
                } else {
                    // No id_list — build both lists from offset map
                    for i in 0..om_count {
                        let om_id = read_u32_le(&data, cursor - om_id_bytes + i * 4);
                        let (file_off, rec_sz) = om_entries[i];
                        if rec_sz > 0 {
                            record_ids.push(om_id);
                            let data_relative =
                                (file_off as usize).saturating_sub(sec_offset) + base_data_len;
                            record_offsets.push(data_relative);
                            record_sizes.push(rec_sz as usize);
                            record_string_table_indices.push(section_string_table_index);
                        }
                    }
                }
            } else if !has_offset_map {
                // Fixed-size records: offsets are sequential
                for i in 0..sec.record_count as usize {
                    record_offsets.push(base_data_len + i * record_size);
                    record_sizes.push(record_size);
                    record_string_table_indices.push(section_string_table_index);
                }
            }

            trace!(
                "  section {si}: {} records, {} copies, id_list={}, offset_map={}",
                sec.record_count, sec.copy_table_count, sec.id_list_size, sec._offset_map_id_count
            );
        }

        if header.format_version == 5
            && header.record_count > 0
            && !has_id_list
            && header.id_index == u16::MAX
        {
            bail!("WDC5 regular file has neither an external id list nor an inline id field");
        }

        if !has_id_list && !has_offset_map && header.id_index != u16::MAX {
            let id_field = usize::from(header.id_index);
            ensure!(
                id_field < field_info.len(),
                "inline id field {id_field} is outside {} DB2 fields",
                field_info.len()
            );
            if header.format_version == 5 {
                let info = &field_info[id_field];
                let field_end = usize::from(info.field_offset_bits)
                    .checked_add(usize::from(info.field_size_bits))
                    .context("WDC5 inline id bit range overflow")?;
                ensure!(
                    info.field_size_bits > 0 && info.field_size_bits <= 32,
                    "WDC5 inline id field must contain 1..=32 bits"
                );
                ensure!(
                    field_end <= record_size.saturating_mul(8),
                    "WDC5 inline id field exceeds the record bit width"
                );
            }
            ensure!(
                record_ids.len() == record_offsets.len(),
                "inline id record/offset count mismatch ({} != {})",
                record_ids.len(),
                record_offsets.len()
            );
            for (record_idx, record_id) in record_ids.iter_mut().enumerate() {
                *record_id = read_inline_record_id(
                    &record_data,
                    &record_offsets,
                    record_size,
                    &field_info,
                    &pallet_data,
                    record_idx,
                    id_field,
                )?;
            }
        }

        // Build id→index map
        let mut id_to_index = HashMap::with_capacity(record_ids.len());
        for (idx, &id) in record_ids.iter().enumerate() {
            id_to_index.insert(id, idx);
        }

        debug!(
            "WDC{}: loaded {} records + {} copies = {} total, {} pallet fields, offset_map={}",
            header.format_version,
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
            relationship_ids,
            record_offsets,
            record_sizes,
            string_tables,
            record_string_table_indices,
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
        let info = &self.field_info[field];
        let raw = self.read_field(record_idx, field);
        if info.compression == CompressionType::BitpackedSigned {
            sign_extend(raw, u32::from(info.field_size_bits))
        } else {
            raw as i32
        }
    }

    /// Read a field as f32 from a record index.
    pub fn get_field_f32(&self, record_idx: usize, field: usize) -> f32 {
        f32::from_bits(self.read_field(record_idx, field))
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

    /// Read a non-localized string field from a record index.
    pub fn get_field_string(&self, record_idx: usize, field: usize) -> String {
        let offset = self.get_field_u32(record_idx, field) as usize;
        let Some(Some(table_idx)) = self.record_string_table_indices.get(record_idx) else {
            return String::new();
        };
        let Some(table) = self.string_tables.get(*table_idx) else {
            return String::new();
        };
        if offset >= table.len() {
            return String::new();
        }

        let end = table[offset..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|pos| offset + pos)
            .unwrap_or(table.len());
        String::from_utf8_lossy(&table[offset..end]).into_owned()
    }

    /// Read an explicitly uncompressed byte array without zero fallbacks.
    /// Target typed consumers must reject unsupported column representations
    /// instead of manufacturing key/data bytes through permissive getters.
    pub fn get_fixed_u8_array<const N: usize>(
        &self,
        record_idx: usize,
        field: usize,
    ) -> Result<[u8; N]> {
        let info = self
            .field_info
            .get(field)
            .context("DB2 array field absent")?;
        ensure!(
            info.compression == CompressionType::None,
            "DB2 byte array compression is unsupported"
        );
        ensure!(
            usize::from(info.field_size_bits)
                == N.checked_mul(8).context("DB2 array size overflow")?,
            "DB2 byte array width does not match typed schema"
        );
        ensure!(
            info.field_offset_bits % 8 == 0,
            "DB2 byte array is not byte aligned"
        );
        let start = usize::from(info.field_offset_bits / 8);
        let end = start.checked_add(N).context("DB2 array offset overflow")?;
        let record = self
            .record_bytes(record_idx)
            .context("DB2 array record absent")?;
        let bytes = record
            .get(start..end)
            .context("DB2 byte array exceeds record")?;
        Ok(bytes.try_into().expect("checked fixed-array length"))
    }

    /// Read an element from an array field. This legacy permissive accessor
    /// is not the typed byte-array validation contract above.
    pub fn get_array_element(
        &self,
        record_idx: usize,
        field: usize,
        array_index: usize,
        element_bits: usize,
    ) -> u32 {
        let info = &self.field_info[field];
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };

        match info.compression {
            CompressionType::None
            | CompressionType::Bitpacked
            | CompressionType::BitpackedSigned => {
                let bit_offset = info.field_offset_bits as usize + array_index * element_bits;
                read_bits(&self.record_data, record_start, bit_offset, element_bits)
            }
            CompressionType::PalletArray => {
                let index = read_bits(
                    &self.record_data,
                    record_start,
                    info.field_offset_bits as usize,
                    info.field_size_bits as usize,
                ) as usize;
                let cardinality = info.val3.max(1) as usize;
                self.pallet_data
                    .get(field)
                    .and_then(|p| p.get(index * cardinality + array_index))
                    .copied()
                    .unwrap_or(0)
            }
            CompressionType::Pallet | CompressionType::Common => self.read_field(record_idx, field),
        }
    }

    /// Read an array element as i16 (for short[] arrays like StatModifierBonusAmount).
    pub fn get_array_i16(&self, record_idx: usize, field: usize, array_index: usize) -> i16 {
        self.get_array_element(record_idx, field, array_index, 16) as i16
    }

    /// Read an array element as i32.
    pub fn get_array_i32(&self, record_idx: usize, field: usize, array_index: usize) -> i32 {
        let info = &self.field_info[field];
        let element_bits = 32;
        let bit_offset = info.field_offset_bits as usize + array_index * element_bits;
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };
        sign_extend(
            read_bits(&self.record_data, record_start, bit_offset, element_bits),
            32,
        )
    }

    /// Read an array element as i64.
    pub fn get_array_i64(&self, record_idx: usize, field: usize, array_index: usize) -> i64 {
        let info = &self.field_info[field];
        let element_bits = 64;
        let bit_offset = info.field_offset_bits as usize + array_index * element_bits;
        let record_start = if !self.record_offsets.is_empty() {
            self.record_offsets[record_idx]
        } else {
            record_idx * self.header.record_size as usize
        };
        let lo = read_bits(&self.record_data, record_start, bit_offset, 32) as u64;
        let hi = read_bits(&self.record_data, record_start, bit_offset + 32, 32) as u64;
        ((hi << 32) | lo) as i64
    }

    /// Read an array element as u16 (for ushort[] arrays like QuestXP::Difficulty).
    pub fn get_array_u16(&self, record_idx: usize, field: usize, array_index: usize) -> u16 {
        self.get_array_element(record_idx, field, array_index, 16) as u16
    }

    /// Read an array element as i8 (for sbyte[] arrays like StatModifierBonusStat).
    pub fn get_array_i8(&self, record_idx: usize, field: usize, array_index: usize) -> i8 {
        self.get_array_element(record_idx, field, array_index, 8) as i8
    }

    /// Number of fields in this DB2 file.
    pub fn field_count(&self) -> usize {
        self.field_info.len()
    }

    /// Header metadata is distinct from the parsed column count in WDC5.
    pub fn declared_field_count(&self) -> u32 {
        self.header.field_count
    }
    pub fn parent_lookup_count(&self) -> u32 {
        self.header._parent_lookup_count
    }

    /// Return the DB2 wire format version (4 for WDC4, 5 for WDC5).
    pub fn format_version(&self) -> u32 {
        self.header.format_version
    }

    /// Return the inline ID column, or `None` when IDs are supplied by a
    /// section ID table (including the on-disk -1 sentinel).
    pub fn inline_id_field(&self) -> Option<usize> {
        if self.header.flags & 0x04 != 0 || self.header.id_index == u16::MAX {
            None
        } else {
            Some(usize::from(self.header.id_index))
        }
    }

    /// Get the record index for a given record ID.
    pub fn get_record_index(&self, record_id: u32) -> Option<usize> {
        self.id_to_index.get(&record_id).copied()
    }

    /// Return the WDC4 relationship/parent id for a record index when the table has one.
    pub fn get_relationship_id(&self, record_idx: usize) -> Option<u32> {
        self.relationship_ids
            .get(record_idx)
            .and_then(|relationship_id| *relationship_id)
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

    /// Return the schema/layout hash from the DB2 header.
    pub fn layout_hash(&self) -> u32 {
        self.header._layout_hash
    }

    /// Iterate over all records including copies: yields (record_id, record_index).
    ///
    /// For copy table entries, the record_index points to the source record data.
    pub fn iter_records(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        let direct = self
            .record_ids
            .iter()
            .enumerate()
            .map(|(idx, &id)| (id, idx));
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
            CompressionType::None
            | CompressionType::Bitpacked
            | CompressionType::BitpackedSigned => read_bits(
                &self.record_data,
                record_start,
                info.field_offset_bits as usize,
                info.field_size_bits as usize,
            ),
            CompressionType::Pallet => {
                let index = read_bits(
                    &self.record_data,
                    record_start,
                    info.field_offset_bits as usize,
                    info.field_size_bits as usize,
                ) as usize;
                self.pallet_data
                    .get(field)
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
                self.pallet_data
                    .get(field)
                    .and_then(|p| p.get(index * cardinality))
                    .copied()
                    .unwrap_or(0)
            }
            CompressionType::Common => {
                let record_id = self.record_ids.get(record_idx).copied().unwrap_or(0);
                self.common_data
                    .get(field)
                    .and_then(|m| m.get(&record_id))
                    .copied()
                    .unwrap_or(info.val1) // val1 = default_value for Common
            }
        }
    }
}

#[test]
fn test_find_record_58268() {
    let path = std::path::Path::new("/home/server/woltk-server-core/Data/dbc/esES/ItemSparse.db2");
    if !path.exists() {
        return;
    }
    let reader = Wdc4Reader::open(path).expect("failed to parse");
    let found = reader.iter_records().any(|(id, _)| id == 58268);
    eprintln!("Record 58268 in ItemSparse: {found}");
    // Also check nearby
    let nearby: Vec<u32> = reader
        .iter_records()
        .map(|(id, _)| id)
        .filter(|&id| id >= 58260 && id <= 58280)
        .collect();
    eprintln!("Records 58260-58280: {:?}", {
        let mut v = nearby;
        v.sort();
        v
    });
}
