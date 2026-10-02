//! Bounded WDC5-regular reader fixtures.
//!
//! These synthetic files exercise the header/metadata and fixed-section
//! contract only. Sparse, TACT/encrypted, and other WDC5 variants are
//! intentionally outside this reader.

use super::*;
use std::fs;
use std::path::PathBuf;

const SYNTHETIC_WDC5_HEADER_SIZE: usize = 204;

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn wdc5_fixture(
    flags: u16,
    id_index: u16,
    tact_id: u64,
    id_list_size: u32,
    copy_count: u32,
) -> Vec<u8> {
    let section_offset = SYNTHETIC_WDC5_HEADER_SIZE;
    let field_meta_offset = section_offset + SECTION_HEADER_SIZE;
    let record_offset = field_meta_offset + FIELD_META_SIZE + FIELD_STORAGE_INFO_SIZE;
    let record_end = record_offset + 4;
    let id_end = record_end + id_list_size as usize;
    let copy_end = id_end + copy_count as usize * 8;
    let mut data = vec![0; copy_end];

    put_u32(&mut data, 0, 0x3543_4457); // WDC5
    put_u32(&mut data, 4, 5);
    put_u32(&mut data, 136, 1); // record count
    put_u32(&mut data, 140, 1); // field metadata count
    put_u32(&mut data, 144, 4); // record size
    put_u32(&mut data, 152, 0x1122_3344); // table hash
    put_u32(&mut data, 156, 0x5566_7788); // layout hash
    put_u32(&mut data, 160, 7);
    put_u32(&mut data, 164, 7);
    put_u16(&mut data, 172, flags);
    put_u16(&mut data, 174, id_index);
    put_u32(&mut data, 176, 1); // total field count
    put_u32(&mut data, 188, FIELD_STORAGE_INFO_SIZE as u32);
    put_u32(&mut data, 200, 1); // section count

    put_u64(&mut data, section_offset, tact_id);
    put_u32(&mut data, section_offset + 8, record_offset as u32);
    put_u32(&mut data, section_offset + 12, 1);
    put_u32(&mut data, section_offset + 24, id_list_size);
    put_u32(&mut data, section_offset + 36, copy_count);

    // The four-byte DB2FieldEntry is intentionally opaque to the reader.
    let column = field_meta_offset + FIELD_META_SIZE;
    put_u16(&mut data, column, 0); // bit offset
    put_u16(&mut data, column + 2, 32); // bit width
    put_u32(&mut data, column + 8, 0); // no compression
    put_u32(&mut data, record_offset, 7);
    if id_list_size == 4 {
        put_u32(&mut data, record_end, 42);
    }
    if copy_count == 1 {
        put_u32(&mut data, id_end, 8); // new id
        put_u32(&mut data, id_end + 4, 42); // source id
    }
    data
}

fn fixture_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rustycore-wdc5-{name}-{}-{}.db2",
        std::process::id(),
        name.len()
    ));
    path
}

fn open_fixture(name: &str, data: &[u8]) -> Result<Wdc4Reader> {
    let directory = fixture_path(name);
    // Own a new directory before writing or removing a fixture. Never overwrite
    // an unrelated predictable /tmp file or follow a pre-existing symlink.
    fs::create_dir(&directory).expect("create new synthetic fixture directory");
    let path = directory.join("fixture.db2");
    fs::write(&path, data).expect("write synthetic WDC5 fixture");
    let result = Wdc4Reader::open(std::path::Path::new(&path));
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(directory);
    result
}

#[test]
fn regular_inline_ids_preserve_schema_hashes_and_version() {
    let reader = open_fixture("inline", &wdc5_fixture(0, 0, 0, 0, 0)).unwrap();

    assert_eq!(reader.format_version(), 5);
    assert_eq!(reader.table_hash(), 0x1122_3344);
    assert_eq!(reader.layout_hash(), 0x5566_7788);
    assert_eq!(reader.inline_id_field(), Some(0));
    assert_eq!(reader.record_count(), 1);
    assert_eq!(reader.record_id(0), 7);
    assert_eq!(reader.get_field_u32(0, 0), 7);
}

#[test]
fn regular_external_ids_and_copy_table_are_loaded() {
    let reader = open_fixture("external", &wdc5_fixture(4, u16::MAX, 0, 4, 1)).unwrap();

    assert_eq!(reader.record_id(0), 42);
    assert_eq!(reader.inline_id_field(), None);
    assert_eq!(reader.total_count(), 2);
    assert_eq!(
        reader.iter_records().collect::<Vec<_>>(),
        vec![(42, 0), (8, 0)]
    );
}

#[test]
fn malformed_wdc5_header_and_regular_variants_fail_closed() {
    let valid = wdc5_fixture(0, 0, 0, 0, 0);
    assert!(parse_header(&valid[..203]).is_err());

    let mut wrong_magic = valid.clone();
    put_u32(&mut wrong_magic, 0, 0xFFFF_FFFF);
    assert!(parse_header(&wrong_magic).is_err());

    let mut wrong_version = valid.clone();
    put_u32(&mut wrong_version, 4, 4);
    assert!(parse_header(&wrong_version).is_err());

    let mut too_many_parents = valid.clone();
    put_u32(&mut too_many_parents, 184, 2);
    assert!(parse_header(&too_many_parents).is_err());

    let mut sparse = valid.clone();
    put_u16(&mut sparse, 172, 1);
    assert!(open_fixture("sparse", &sparse).is_err());

    let tact = wdc5_fixture(0, 0, 1, 0, 0);
    assert!(open_fixture("tact", &tact).is_err());

    let mut empty_tact = tact;
    put_u32(&mut empty_tact, 136, 0);
    put_u32(&mut empty_tact, SYNTHETIC_WDC5_HEADER_SIZE + 12, 0);
    put_u32(&mut empty_tact, 188, 0);
    assert!(open_fixture("empty-tact", &empty_tact).is_err());

    let mut huge_sections = valid.clone();
    put_u32(&mut huge_sections, 200, u32::MAX);
    assert!(open_fixture("huge-sections", &huge_sections).is_err());

    let mut huge_fields = valid.clone();
    put_u32(&mut huge_fields, 140, u32::MAX);
    assert!(open_fixture("huge-fields", &huge_fields).is_err());

    let mut truncated_fields = valid.clone();
    truncated_fields.truncate(SYNTHETIC_WDC5_HEADER_SIZE + SECTION_HEADER_SIZE + 3);
    assert!(open_fixture("field-truncated", &truncated_fields).is_err());

    let mut invalid_columns = valid.clone();
    put_u32(&mut invalid_columns, 188, 25);
    assert!(open_fixture("column-size", &invalid_columns).is_err());

    let invalid_ids = wdc5_fixture(4, u16::MAX, 0, 3, 0);
    assert!(open_fixture("id-list-size", &invalid_ids).is_err());

    let mut invalid_inline = valid;
    put_u16(
        &mut invalid_inline,
        SYNTHETIC_WDC5_HEADER_SIZE + SECTION_HEADER_SIZE + FIELD_META_SIZE + 2,
        33,
    );
    assert!(open_fixture("inline-width", &invalid_inline).is_err());

    let no_id_source = wdc5_fixture(0, u16::MAX, 0, 0, 0);
    assert!(open_fixture("no-id-source", &no_id_source).is_err());
}
