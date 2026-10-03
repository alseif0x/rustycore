//! Exact 70170/esES ability prefix, following 02245dcd LoadTableData::Skip.
//! No generic encrypted-table waiver or promotion of the five unknown rows.
use super::super::{Wdc4Header, format::SectionHeader};
use anyhow::{Result, ensure};

pub(in crate::wdc4) fn validate(
    header: &Wdc4Header,
    sections: &[SectionHeader],
    bytes: usize,
) -> Result<()> {
    ensure!(
        header.format_version == 5
            && header.table_hash == 0xFF44_46F6
            && header._layout_hash == 0x224F_7EA0
            && header.field_count == 18
            && header.total_field_count == 18
            && header.record_size == 20
            && header.record_count == 7838
            && header.flags == 0
            && header.id_index == 2
            && header._parent_lookup_count == 1
            && header.section_count == 6
            && sections.len() == 6
            && bytes == 346918,
        "Wrong available birth-ability schema/extent"
    );
    for (index, section) in sections.iter().enumerate() {
        let records = if index == 0 { 7833 } else { 1 };
        let offset = if index == 0 {
            127580
        } else {
            346918 + (index as u32 - 1) * 40
        };
        ensure!(
            (section._tact_key_hash == 0) == (index == 0)
                && section.file_offset == offset
                && section.record_count == records
                && section.string_table_size == (if index == 0 { 2 } else { 0 })
                && section.id_list_size == 0
                && section._relationship_data_size == 12 + records * 8
                && section._offset_map_id_count == 0
                && section.copy_table_count == 0,
            "Wrong available birth-ability section"
        );
    }
    let known = &sections[0];
    ensure!(
        u64::from(known.file_offset)
            + u64::from(known.record_count) * u64::from(header.record_size)
            + u64::from(known.string_table_size)
            + u64::from(known._relationship_data_size)
            == bytes as u64,
        "Wrong available birth-ability plaintext boundary"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wdc4::{
        Wdc4Reader,
        format::{parse_header, parse_section_header},
    };

    fn word(data: &mut [u8], at: usize, value: u32) {
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    // Exact bounded header/extents but wholly synthetic records/parent IDs.
    // Inline IDs are readable directly; no real client or TACT data is copied.
    fn fixture() -> Vec<u8> {
        let mut data = vec![0; 346918];
        for (at, value) in [
            (0, 0x3543_4457),
            (4, 5),
            (136, 7838),
            (140, 18),
            (144, 20),
            (152, 0xFF44_46F6),
            (156, 0x224F_7EA0),
            (164, 7833),
            (172, 2 << 16),
            (176, 18),
            (184, 1),
            (188, 18 * 24),
            (200, 6),
        ] {
            word(&mut data, at, value);
        }
        for i in 0..6 {
            let at = 204 + i * 40;
            let records = if i == 0 { 7833 } else { 1 };
            word(&mut data, at, if i == 0 { 0 } else { 1 });
            word(
                &mut data,
                at + 8,
                if i == 0 {
                    127580
                } else {
                    346918 + (i as u32 - 1) * 40
                },
            );
            word(&mut data, at + 12, records);
            word(&mut data, at + 16, if i == 0 { 2 } else { 0 });
            word(&mut data, at + 28, 12 + records * 8);
        }
        let columns = 204 + 6 * 40 + 18 * 4;
        // Inline-ID column field 2 has offset zero/32 bits; every other
        // synthetic column also uses None compression, with no data blobs.
        for i in 0..18 {
            word(&mut data, columns + i * 24, 32 << 16);
        }
        for i in 0..7833 {
            word(&mut data, 127580 + i * 20, i as u32 + 1);
        }
        // Parent lookup contains one synthetic entry per plaintext record.
        let parent = 127580 + 7833 * 20 + 2;
        word(&mut data, parent, 7833);
        word(&mut data, parent + 4, 1);
        word(&mut data, parent + 8, 1);
        for i in 0..7833 {
            word(&mut data, parent + 12 + i * 8, 1);
            word(&mut data, parent + 16 + i * 8, i as u32);
        }
        data
    }

    #[test]
    fn bounded_birth_prefix_rejects_hash_parent_key_and_extent_drift() {
        let data = fixture();
        let check = |data: &[u8]| {
            let header = parse_header(data)?;
            let sections = (0..6)
                .map(|i| parse_section_header(&data[204 + i * 40..]))
                .collect::<Vec<_>>();
            validate(&header, &sections, data.len())
        };
        assert!(check(&data).is_ok());
        for at in [
            136, 140, 144, 152, 156, 172, 176, 184, 200, 212, 220, 232, 244, 252, 256, 268, 272,
        ] {
            let mut invalid = data.clone();
            invalid[at] ^= 1;
            assert!(check(&invalid).is_err(), "offset {at}");
        }
        assert!(check(&data[..data.len() - 1]).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(check(&extra).is_err());
    }

    #[test]
    fn only_explicit_birth_reader_retains_plaintext_ids_and_parents() {
        let directory =
            std::env::temp_dir().join(format!("rustycore-birth-prefix-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("synthetic.db2");
        std::fs::write(&path, fixture()).unwrap();
        assert!(Wdc4Reader::open(&path).is_err());
        assert!(Wdc4Reader::open_available_initial_map(&path).is_err());
        assert!(Wdc4Reader::open_available_achievement(&path).is_err());
        let reader = Wdc4Reader::open_available_birth_abilities(&path).unwrap();
        assert_eq!(reader.total_count(), 7833);
        assert_eq!(
            reader.iter_records().map(|(id, _)| id).collect::<Vec<_>>(),
            (1..=7833).collect::<Vec<_>>()
        );
        assert_eq!(reader.get_relationship_id(0), Some(1));
        assert_eq!(reader.get_relationship_id(7832), Some(1));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
