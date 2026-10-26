use super::Db2IdStore;
use std::fs;

#[test]
fn complete_loader_rejects_unresolved_copy_sources_without_changing_legacy_load() {
    const HEADER_SIZE: usize = 72;
    const SECTION_HEADER_SIZE: usize = 40;
    let root = std::env::temp_dir().join(format!(
        "rustycore-db2-id-store-copy-{}",
        std::process::id(),
    ));
    let data_dir = root.join("data");
    let locale_dir = data_dir.join("dbc/enUS");
    fs::create_dir_all(&locale_dir).expect("create DB2 fixture directory");

    let data_offset = HEADER_SIZE + SECTION_HEADER_SIZE;
    let mut bytes = vec![0; data_offset + 8];
    put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
    put_u32(&mut bytes, 12, 1);
    put_u32(&mut bytes, 20, 0xABCD_1234);
    put_u16(&mut bytes, 42, u16::MAX);
    put_u32(&mut bytes, 68, 1);
    put_u32(&mut bytes, HEADER_SIZE + 8, data_offset as u32);
    put_u32(&mut bytes, HEADER_SIZE + 36, 1);
    put_u32(&mut bytes, data_offset, 900);
    put_u32(&mut bytes, data_offset + 4, 901);
    fs::write(locale_dir.join("PlayerCondition.db2"), bytes)
        .expect("write unresolved-copy WDC4 fixture");

    let legacy = Db2IdStore::load(
        data_dir.to_str().expect("temporary path is UTF-8"),
        "enUS",
        "PlayerCondition.db2",
    )
    .expect("legacy file-only loader keeps its prior incomplete-copy behavior");
    assert!(legacy.is_empty());

    let complete = Db2IdStore::load_complete(
        data_dir.to_str().expect("temporary path is UTF-8"),
        "enUS",
        "PlayerCondition.db2",
    );
    let error = complete.expect_err("complete loader must reject the unresolved source");
    assert!(error.to_string().contains("unresolved WDC4 copy records"));

    let _ = fs::remove_dir_all(root);
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
