use super::*;
fn word(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn empty(fields: u32, hash: u32, layout: u32) -> Vec<u8> {
    let mut data = vec![0; 204 + fields as usize * 4];
    for (at, v) in [
        (0, 0x3543_4457),
        (4, 5),
        (140, fields),
        (152, hash),
        (156, layout),
        (172, 4),
        (176, fields),
    ] {
        word(&mut data, at, v);
    }
    data
}
fn overrides() -> Vec<u8> {
    let mut data = vec![0; 412];
    for (at, v) in [
        (0, 0x3543_4457),
        (4, 5),
        (136, 9),
        (140, 1),
        (144, 2),
        (152, 0x149A_AE79),
        (156, 0xB292_998C),
        (164, 9),
        (172, 4),
        (176, 1),
        (184, 1),
        (188, 24),
        (200, 1),
        (212, 272),
        (216, 9),
        (220, 2),
        (228, 36),
        (232, 84),
    ] {
        word(&mut data, at, v);
    }
    data[250..252].copy_from_slice(&16u16.to_le_bytes());
    for i in 0..9 {
        data[272 + i * 2..274 + i * 2].copy_from_slice(&u16::MAX.to_le_bytes());
        word(&mut data, 292 + i * 4, (9 - i) as u32);
    }
    word(&mut data, 328, 9);
    for i in 0..9 {
        word(&mut data, 340 + i * 8, u32::MAX);
        word(&mut data, 344 + i * 8, i as u32);
    }
    data
}
#[test]
fn genuine_empty_tables_have_full_metadata_and_extra_parent_is_uint32() {
    let directory =
        std::env::temp_dir().join(format!("forever-item-spec-tables-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let spec = directory.join("ItemSpec.db2");
    let gem = directory.join("GemProperties.db2");
    let relation = directory.join("ItemSpecOverride.db2");
    let valid = empty(6, ITEM_SPEC_HASH, 0x83F3_D113);
    std::fs::write(&spec, &valid).unwrap();
    std::fs::write(&gem, empty(2, GEM_PROPERTIES_HASH, 0x8648_7AD2)).unwrap();
    std::fs::write(&relation, overrides()).unwrap();
    let rows = ItemSpecRecords::load(&directory).unwrap();
    assert!(rows.specs.is_empty() && rows.gems.is_empty());
    assert_eq!(rows.overrides.len(), 9);
    assert_eq!(rows.overrides[0].id, 1);
    assert_eq!(rows.overrides[0].specialization, u16::MAX);
    assert_eq!(rows.overrides[8].item, u32::MAX);
    for at in [136, 144, 148, 152, 156, 176, 188, 192, 196, 200] {
        let mut drift = valid.clone();
        drift[at] ^= 1;
        std::fs::write(&spec, drift).unwrap();
        assert!(ItemSpecRecords::load(&directory).is_err(), "offset {at}");
    }
    std::fs::write(&spec, &valid[..204]).unwrap();
    assert!(ItemSpecRecords::load(&directory).is_err());
    std::fs::write(&spec, &valid[..valid.len() - 1]).unwrap();
    assert!(ItemSpecRecords::load(&directory).is_err());
    let mut extra = valid;
    extra.push(0);
    std::fs::write(&spec, extra).unwrap();
    assert!(ItemSpecRecords::load(&directory).is_err());
    for path in [spec, gem, relation] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();
}
