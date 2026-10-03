//! Bounded target prefix, not a generic license to ignore encrypted DB2 data.
//! 70170 acquisition preserves the original header and both section headers.
use super::*;

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
