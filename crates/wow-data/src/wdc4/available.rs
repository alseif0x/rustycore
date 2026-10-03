//! Bounded target prefix, not a generic license to ignore encrypted DB2 data.
//! 70170 acquisition preserves the original header and both section headers.
use super::*;
mod birth_abilities;
pub(in crate::wdc4) mod items;
pub(in crate::wdc4) mod spells;
pub(super) use birth_abilities::validate as validate_birth_abilities;

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
