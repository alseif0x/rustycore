use super::*;
use crate::wdc4::{
    Wdc4Reader,
    creation::{CreationDb2, CreationTable},
    format::{parse_header, parse_section_header},
};

fn word(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// Frozen geometry with wholly synthetic IDs/common values/copies/parents.
// No private client row or TACT identifier is embedded in regression data.
fn fixture(prefix: ItemPrefix) -> Vec<u8> {
    let c = prefix.contract();
    let mut data = vec![0; prefix.byte_limit()];
    for (at, value) in [
        (0, 0x3543_4457),
        (4, 5),
        (136, c.counts.iter().sum()),
        (140, c.fields),
        (144, c.record_size),
        (152, c.hash),
        (156, c.layout),
        (164, c.counts[0] + c.copies[0] + 100),
        (172, 4),
        (176, c.fields),
        (184, c.parent),
        (188, c.fields * 24),
        (200, c.counts.len() as u32),
    ] {
        word(&mut data, at, value);
    }
    for i in 0..c.counts.len() {
        let at = 204 + i * 40;
        for (relative, value) in [
            (0, u32::from(i != 0)),
            (8, c.offsets[i]),
            (12, c.counts[i]),
            (16, u32::from(i == 0) * 2),
            (24, c.counts[i] * 4),
            (
                28,
                if c.parent == 0 {
                    0
                } else {
                    12 + c.counts[i] * 8
                },
            ),
            (36, c.copies[i]),
        ] {
            word(&mut data, at + relative, value);
        }
    }
    let columns = 204 + c.counts.len() * 40 + c.fields as usize * 4;
    for i in 0..c.fields as usize {
        // Common compression with no blob: every field has a checked default.
        word(&mut data, columns + i * 24 + 8, 2);
        word(&mut data, columns + i * 24 + 12, 0xFFFF_FEFD);
    }
    let ids = (c.offsets[0] + c.counts[0] * c.record_size + 2) as usize;
    for i in 0..c.counts[0] as usize {
        word(&mut data, ids + i * 4, i as u32 + 1);
    }
    let copies = ids + c.counts[0] as usize * 4;
    for i in 0..c.copies[0] as usize {
        word(&mut data, copies + i * 8, c.counts[0] + i as u32 + 1);
        word(&mut data, copies + i * 8 + 4, 1);
    }
    if c.parent != 0 {
        let parents = copies + c.copies[0] as usize * 8;
        word(&mut data, parents, c.counts[0]);
        for i in 0..c.counts[0] as usize {
            word(&mut data, parents + 12 + i * 8, 0xFEDC_BA98);
            word(&mut data, parents + 16 + i * 8, i as u32);
        }
    }
    data
}

fn check(prefix: ItemPrefix, data: &[u8]) -> Result<()> {
    let header = parse_header(data)?;
    let sections = (0..prefix.contract().counts.len())
        .map(|i| parse_section_header(&data[204 + i * 40..]))
        .collect::<Vec<_>>();
    prefix.validate(&header, &sections, data.len())
}

#[test]
fn item_prefixes_freeze_all_sections_schema_copies_parent_and_extent() {
    for prefix in [ItemPrefix::Basic, ItemPrefix::Effect, ItemPrefix::Relation] {
        let data = fixture(prefix);
        assert!(check(prefix, &data).is_ok());
        for at in [
            136,
            140,
            144,
            152,
            156,
            164 + 8,
            176,
            184,
            188,
            200,
            212,
            216,
            220,
            228,
            232,
            236,
            240,
            244,
            252,
            256,
            260,
            268,
            272,
            276,
            280,
        ] {
            let mut drift = data.clone();
            drift[at] ^= 1;
            assert!(check(prefix, &drift).is_err(), "offset {at}");
        }
        assert!(check(prefix, &data[..data.len() - 1]).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(check(prefix, &extra).is_err());
    }
}

#[test]
fn only_explicit_item_reader_exposes_plaintext_records_copies_and_extra_parent() {
    let directory =
        std::env::temp_dir().join(format!("forever-item-prefixes-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    for (prefix, table, name) in [
        (ItemPrefix::Basic, CreationTable::Item, "Item"),
        (ItemPrefix::Effect, CreationTable::ItemEffect, "ItemEffect"),
        (
            ItemPrefix::Relation,
            CreationTable::ItemEffectRelation,
            "ItemXItemEffect",
        ),
    ] {
        let path = directory.join(format!("{name}.available.db2"));
        std::fs::write(&path, fixture(prefix)).unwrap();
        assert!(Wdc4Reader::open(&path).is_err());
        assert!(Wdc4Reader::open_available_birth_abilities(&path).is_err());
        let reader = CreationDb2::open_item_prefix(&directory, table).unwrap();
        let c = prefix.contract();
        assert_eq!(reader.ids().count(), (c.counts[0] + c.copies[0]) as usize);
        let width = if matches!(prefix, ItemPrefix::Effect) {
            8
        } else {
            32
        };
        assert_eq!(
            reader.bits(1, 0, 0).unwrap(),
            if width == 8 { 253 } else { 0xFFFF_FEFD }
        );
        assert_eq!(reader.bits(1, 0, 1).is_err(), true);
        if c.copies[0] != 0 {
            assert_eq!(
                reader.bits(c.counts[0] + 1, 0, 0).unwrap(),
                reader.bits(1, 0, 0).unwrap()
            );
        } else {
            assert_eq!(reader.bits(1, 1, 0).unwrap(), 0xFEDC_BA98);
            assert_eq!(reader.bits(c.counts[0], 1, 0).unwrap(), 0xFEDC_BA98);
        }
        if matches!(prefix, ItemPrefix::Basic) {
            assert_eq!(reader.bits(1, 3, 0).unwrap() as i8, -3);
            assert_eq!(reader.bits(1, 0, 0).unwrap() as i32, -259);
        }
        if matches!(prefix, ItemPrefix::Effect) {
            assert_eq!(reader.bits(1, 2, 0).unwrap() as i16, -259);
            assert_eq!(reader.bits(1, 5, 0).unwrap() as u16, 65277);
        }
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn item_copy_source_skip_file_order_overwrite_zero_target_and_header_bound() {
    let prefix = ItemPrefix::Basic;
    let c = prefix.contract();
    let mut data = fixture(prefix);
    let at = (c.offsets[0] + c.counts[0] * c.record_size + 2 + c.counts[0] * 4) as usize;
    for (i, (new, source)) in [
        (c.counts[0] + 1, c.counts[0] + 2),
        (c.counts[0] + 2, 1),
        (1, 2),
        (0, 1),
        (c.counts[0] + 3, 0),
    ]
    .into_iter()
    .enumerate()
    {
        word(&mut data, at + i * 8, new);
        word(&mut data, at + i * 8 + 4, source);
    }
    let directory =
        std::env::temp_dir().join(format!("forever-item-copies-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("Item.available.db2");
    std::fs::write(&path, &data).unwrap();
    let reader = CreationDb2::open_item_prefix(&directory, CreationTable::Item).unwrap();
    let ids = reader.ids().collect::<Vec<_>>();
    assert!(ids.contains(&0) && ids.contains(&(c.counts[0] + 2)));
    assert!(!ids.contains(&(c.counts[0] + 1)) && !ids.contains(&(c.counts[0] + 3)));
    word(&mut data, at, u32::MAX);
    std::fs::write(&path, &data).unwrap();
    assert!(CreationDb2::open_item_prefix(&directory, CreationTable::Item).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
