//! Bounded target prefix, not a generic license to ignore encrypted DB2 data.
//! 70170 acquisition preserves the original header and both section headers.
use super::*;
mod birth_abilities;
pub(in crate::wdc4) mod items;
pub(in crate::wdc4) mod spells;
pub(super) use birth_abilities::validate as validate_birth_abilities;

/// Pinned 70450 Map acquisition: one readable section followed by three
/// TACT-encrypted sections. The latter are skipped, not decrypted or
/// represented as available records.
pub(super) fn validate_forever_map(
    header: &Wdc4Header,
    sections: &[super::format::SectionHeader],
    bytes: usize,
) -> Result<()> {
    ensure!(
        header.format_version == 5
            && header.table_hash == 0xBD84_CD62
            && header._layout_hash == 0xD43A_FAC3
            && header.field_count == 26
            && header.total_field_count == 26
            && header.record_size == 44
            && header.record_count == 81
            && header.flags == 4
            && header.section_count == 4
            && sections.len() == 4,
        "Wrong available Forever Map schema"
    );
    ensure!(
        sections[0]._tact_key_hash == 0 && sections[0].record_count == 73,
        "Wrong available Forever Map readable section"
    );
    for (index, (section, expected_count)) in sections[1..].iter().zip([2, 5, 1]).enumerate() {
        ensure!(
            section._tact_key_hash != 0 && section.record_count == expected_count,
            "Wrong available Forever Map TACT section {}",
            index + 1
        );
    }

    for (index, section) in sections.iter().enumerate() {
        ensure!(
            section.copy_table_count == 0
                && section._relationship_data_size == 0
                && section._offset_map_id_count == 0
                && u64::from(section.id_list_size) == u64::from(section.record_count) * 4,
            "Wrong available Forever Map section metadata {index}"
        );

        let file_offset = usize::try_from(section.file_offset)
            .context("Forever Map section offset does not fit address space")?;
        ensure!(
            file_offset <= bytes,
            "Forever Map section {index} starts out of bounds"
        );
        let record_bytes = usize::try_from(header.record_size)
            .context("Forever Map record size does not fit address space")?
            .checked_mul(
                usize::try_from(section.record_count)
                    .context("Forever Map record count does not fit address space")?,
            )
            .context("Forever Map record data size overflow")?;
        let records_end = file_offset
            .checked_add(record_bytes)
            .context("Forever Map record data offset overflow")?;
        ensure!(
            records_end <= bytes,
            "Forever Map section {index} records out of bounds"
        );
        let strings_end = records_end
            .checked_add(
                usize::try_from(section.string_table_size)
                    .context("Forever Map string table size does not fit address space")?,
            )
            .context("Forever Map string table offset overflow")?;
        ensure!(
            strings_end <= bytes,
            "Forever Map section {index} strings out of bounds"
        );
        let section_end = strings_end
            .checked_add(
                usize::try_from(section.id_list_size)
                    .context("Forever Map ID list size does not fit address space")?,
            )
            .context("Forever Map ID list offset overflow")?;
        ensure!(
            section_end <= bytes,
            "Forever Map section {index} ID list out of bounds"
        );

        if let Some(next) = sections.get(index + 1) {
            ensure!(
                section_end
                    == usize::try_from(next.file_offset)
                        .context("Forever Map next section offset does not fit address space")?,
                "Forever Map section offsets do not chain at section {index}"
            );
        } else {
            ensure!(
                section_end == bytes,
                "Forever Map final section does not end at EOF"
            );
        }
    }
    Ok(())
}

/// Fresh 70170/esES Map acquisition: unchanged 9574-byte prefix, not a
/// complete table. All three excluded sections remain unknown. Ordinary open
/// continues to reject this artifact and encrypted complete files.
pub(super) fn validate_initial_map(
    header: &Wdc4Header,
    sections: &[super::format::SectionHeader],
    bytes: usize,
) -> Result<()> {
    ensure!(
        header.format_version == 5
            && header.table_hash == 0xBD84_CD62
            && header._layout_hash == 0xD43A_FAC3
            && header.field_count == 26
            && header.total_field_count == 26
            && header.record_size == 48
            && header.record_count == 79
            && header.flags == 4
            && header.id_index == 0
            && header._parent_lookup_count == 0
            && header.section_count == 4
            && sections.len() == 4
            && bytes == 9574,
        "Wrong available initial Map schema"
    );
    for (index, section) in sections.iter().enumerate() {
        ensure!(
            (section._tact_key_hash == 0) == (index == 0)
                && section.record_count == [71, 2, 5, 1][index]
                && section.file_offset == [2292, 9574, 9720, 10103][index]
                && section.string_table_size == [3590, 42, 123, 26][index]
                && section.id_list_size == section.record_count * 4
                && section.copy_table_count == 0
                && section._relationship_data_size == 0
                && section._offset_map_id_count == 0,
            "Wrong available initial Map section"
        );
    }
    Ok(())
}

pub(super) fn validate_prefix(
    header: &Wdc4Header,
    sections: &[super::format::SectionHeader],
    bytes: usize,
) -> Result<()> {
    ensure!(
        header.format_version == 5
            && header.table_hash == 0xD2EE_2CA7
            && header._layout_hash == 0x6FC5_281B
            && header.field_count == 19
            && header.total_field_count == 19
            && header.id_index == 3
            && header.flags & 5 == 0
            && header.record_count == 443,
        "Wrong available Achievement schema"
    );
    ensure!(
        sections.len() == 2 && header.section_count == 2,
        "Wrong available Achievement section count"
    );
    let known = &sections[0];
    let unknown = &sections[1];
    ensure!(
        known._tact_key_hash == 0
            && known.record_count == 434
            && unknown._tact_key_hash != 0
            && unknown.record_count == 9
            && known.id_list_size == 0
            && known.copy_table_count == 0
            && unknown.id_list_size == 0
            && unknown.copy_table_count == 0
            && known._offset_map_id_count == 0
            && unknown._offset_map_id_count == 0,
        "Wrong available Achievement section contract"
    );
    let known_end = u64::from(known.file_offset)
        + u64::from(header.record_size) * u64::from(known.record_count)
        + u64::from(known.string_table_size)
        + u64::from(known._relationship_data_size);
    ensure!(
        known_end == bytes as u64 && known_end == u64::from(unknown.file_offset),
        "Truncated or overextended available Achievement prefix"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(data: &mut [u8], at: usize, value: u32) {
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn initial_map_fixture() -> Vec<u8> {
        let mut data = vec![0; 9574];
        word(&mut data, 0, 0x3543_4457);
        word(&mut data, 4, 5);
        word(&mut data, 136, 79);
        word(&mut data, 140, 26);
        word(&mut data, 144, 48);
        word(&mut data, 152, 0xBD84_CD62);
        word(&mut data, 156, 0xD43A_FAC3);
        word(&mut data, 164, 71);
        data[172..174].copy_from_slice(&4u16.to_le_bytes());
        word(&mut data, 176, 26);
        word(&mut data, 188, 26 * 24);
        word(&mut data, 200, 4);
        for i in 0..4 {
            let at = 204 + i * 40;
            word(&mut data, at, if i == 0 { 0 } else { 1 });
            word(&mut data, at + 8, [2292, 9574, 9720, 10103][i]);
            word(&mut data, at + 12, [71, 2, 5, 1][i]);
            word(&mut data, at + 16, [3590, 42, 123, 26][i]);
            word(&mut data, at + 24, [71, 2, 5, 1][i] * 4);
        }
        let ids = 2292 + 71 * 48 + 3590;
        for i in 0..71 {
            word(&mut data, ids + i * 4, i as u32 + 1);
        }
        data
    }

    /// Synthetic Map 70450 structure; the TACT markers and record contents
    /// are fabricated and contain no client data or key material.
    fn forever_map_fixture(readable_sections: usize) -> Vec<u8> {
        let record_counts = [73usize, 2, 5, 1];
        let string_sizes = [2usize, 1, 3, 2];
        let section_headers = WDC5_HEADER_SIZE;
        let field_meta = section_headers + 4 * SECTION_HEADER_SIZE;
        let columns = field_meta + 26 * FIELD_META_SIZE;
        let mut section_offsets = Vec::with_capacity(4);
        let mut id_offsets = Vec::with_capacity(4);
        let mut end = columns + 26 * FIELD_STORAGE_INFO_SIZE;
        for (count, string_size) in record_counts.into_iter().zip(string_sizes) {
            section_offsets.push(end);
            end += count * 44 + string_size;
            id_offsets.push(end);
            end += count * 4;
        }

        let mut data = vec![0; end];
        word(&mut data, 0, 0x3543_4457);
        word(&mut data, 4, 5);
        word(&mut data, 136, 81);
        word(&mut data, 140, 26);
        word(&mut data, 144, 44);
        word(&mut data, 152, 0xBD84_CD62);
        word(&mut data, 156, 0xD43A_FAC3);
        word(&mut data, 160, 1);
        word(&mut data, 164, 81);
        data[172..174].copy_from_slice(&4u16.to_le_bytes());
        word(&mut data, 176, 26);
        word(&mut data, 188, (26 * FIELD_STORAGE_INFO_SIZE) as u32);
        word(&mut data, 200, 4);

        for field in 0..26 {
            let column = columns + field * FIELD_STORAGE_INFO_SIZE;
            data[column + 2..column + 4].copy_from_slice(&32u16.to_le_bytes());
        }

        let mut first_id = 1u32;
        for index in 0..4 {
            let at = section_headers + index * SECTION_HEADER_SIZE;
            let record_count = record_counts[index];
            data[at..at + 8].copy_from_slice(
                &if index == 0 || index < readable_sections {
                    0u64
                } else {
                    index as u64
                }
                .to_le_bytes(),
            );
            word(&mut data, at + 8, section_offsets[index] as u32);
            word(&mut data, at + 12, record_count as u32);
            word(&mut data, at + 16, string_sizes[index] as u32);
            word(&mut data, at + 24, (record_count * 4) as u32);

            for record in 0..record_count {
                word(&mut data, id_offsets[index] + record * 4, first_id);
                first_id += 1;
            }
        }
        data
    }

    #[test]
    fn forever_map_reader_exposes_only_the_validated_readable_prefix() {
        let directory = std::env::temp_dir().join(format!(
            "rustycore-available-forever-map-{}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("synthetic.db2");
        let data = forever_map_fixture(1);
        std::fs::write(&path, &data).unwrap();

        assert!(Wdc4Reader::open(&path).is_err());
        let reader = Wdc4Reader::open_available_forever_map(&path).unwrap();
        assert_eq!(reader.record_count(), 73);
        assert_eq!(reader.total_count(), 73);
        assert_eq!(reader.record_id(0), 1);
        assert_eq!(reader.record_id(72), 73);

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn forever_map_reader_rejects_promoted_sections_bad_counts_and_bad_offsets() {
        let directory = std::env::temp_dir().join(format!(
            "rustycore-available-forever-map-invalid-{}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("synthetic.db2");
        let valid = forever_map_fixture(1);

        for (name, offset, value) in [
            ("second-readable", WDC5_HEADER_SIZE + 40, 0),
            ("wrong-record-count", 136, 80),
            ("out-of-range-offset", WDC5_HEADER_SIZE + 8, u32::MAX),
        ] {
            let mut invalid = valid.clone();
            word(&mut invalid, offset, value);
            std::fs::write(&path, invalid).unwrap();
            assert!(
                Wdc4Reader::open_available_forever_map(&path).is_err(),
                "accepted invalid {name} layout"
            );
        }

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn available_map_keeps_only_known_ids_and_cannot_relax_ordinary_open() {
        let directory =
            std::env::temp_dir().join(format!("rustycore-available-map-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("synthetic.db2");
        std::fs::write(&path, initial_map_fixture()).unwrap();
        assert!(Wdc4Reader::open(&path).is_err());
        assert!(Wdc4Reader::open_available_achievement(&path).is_err());
        let reader = Wdc4Reader::open_available_initial_map(&path).unwrap();
        assert_eq!(reader.total_count(), 71);
        assert_eq!(
            reader.iter_records().map(|(id, _)| id).collect::<Vec<_>>(),
            (1..=71).collect::<Vec<_>>()
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn available_map_rejects_drift_unknown_section_promotion_and_changed_extent() {
        let data = initial_map_fixture();
        let validate = |bytes: &[u8]| {
            let header = parse_header(bytes)?;
            let sections = (0..4)
                .map(|i| parse_section_header(&bytes[204 + i * 40..]))
                .collect::<Vec<_>>();
            validate_initial_map(&header, &sections, bytes.len())
        };
        assert!(validate(&data).is_ok());
        for (offset, value) in [
            (152, 0),
            (156, 0),
            (136, 80),
            (244, 0),
            (252, 9573),
            (256, 3),
            (228, 0),
            (232, 1),
        ] {
            let mut invalid = data.clone();
            word(&mut invalid, offset, value);
            assert!(validate(&invalid).is_err(), "offset {offset}");
        }
        assert!(validate(&data[..data.len() - 1]).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(validate(&extra).is_err());
    }

    /// Synthetic IDs/bytes only; never a copy of Blizzard's table or TACT key.
    fn fixture() -> Vec<u8> {
        let metadata = 204 + 80 + 19 * 4;
        let start = metadata + 19 * 24;
        let end = start + 434 * 4;
        let mut data = vec![0; end];
        word(&mut data, 0, 0x3543_4457);
        word(&mut data, 4, 5);
        word(&mut data, 136, 443);
        word(&mut data, 140, 19);
        word(&mut data, 144, 4);
        word(&mut data, 152, 0xD2EE_2CA7);
        word(&mut data, 156, 0x6FC5_281B);
        data[174..176].copy_from_slice(&3u16.to_le_bytes());
        word(&mut data, 176, 19);
        word(&mut data, 188, 19 * 24);
        word(&mut data, 200, 2);
        word(&mut data, 212, start as u32);
        word(&mut data, 216, 434);
        // Nonzero synthetic unknown-key marker, no actual key identifier.
        word(&mut data, 244, 1);
        word(&mut data, 252, end as u32);
        word(&mut data, 256, 9);
        data[metadata + 3 * 24 + 2..metadata + 3 * 24 + 4].copy_from_slice(&32u16.to_le_bytes());
        for index in 0..434 {
            word(&mut data, start + index * 4, index as u32 + 1);
        }
        data
    }

    #[test]
    fn only_explicit_available_reader_skips_unreadable_section_without_fabricating_ids() {
        let directory = std::env::temp_dir().join(format!(
            "rustycore-available-achievement-{}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("synthetic.db2");
        std::fs::write(&path, fixture()).unwrap();
        assert!(Wdc4Reader::open(&path).is_err());
        let reader = Wdc4Reader::open_available_achievement(&path).unwrap();
        assert_eq!(reader.total_count(), 434);
        assert_eq!(
            reader.iter_records().map(|(id, _)| id).collect::<Vec<_>>(),
            (1..=434).collect::<Vec<_>>()
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn availability_does_not_authorize_wrong_schema_counts_keys_or_truncation() {
        let data = fixture();
        let validate = |data: &[u8]| {
            let header = parse_header(data)?;
            let sections = [
                parse_section_header(&data[204..]),
                parse_section_header(&data[244..]),
            ];
            validate_prefix(&header, &sections, data.len())
        };
        assert!(validate(&data).is_ok());
        for (offset, value) in [
            (152, 0),
            (156, 0),
            (136, 444),
            (216, 433),
            (244, 0),
            (256, 10),
            (200, 1),
        ] {
            let mut invalid = data.clone();
            word(&mut invalid, offset, value);
            // The standalone guard also checks header-declared section count.
            assert!(validate(&invalid).is_err(), "offset {offset}");
        }
        assert!(validate(&data[..data.len() - 1]).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(validate(&extra).is_err());
    }
}
