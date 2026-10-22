//! Synthetic coverage for the pinned 70450 Map prefix and strict Map loading.

use super::*;
use crate::wdc4::Wdc4Reader;
use std::fs;
use std::path::{Path, PathBuf};

const COUNTS: [usize; 4] = [73, 2, 5, 1];
const STRING_SIZES: [usize; 4] = [2, 1, 3, 2];

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Synthetic WDC5 Map layout only; all IDs, payloads and TACT markers are
/// fabricated. `complete` controls whether all four sections are readable.
fn map_fixture(complete: bool) -> Vec<u8> {
    let header_size = 204;
    let section_header_size = 40;
    let field_meta_size = 4;
    let field_storage_info_size = 24;
    let sections_start = header_size;
    let field_meta_start = sections_start + COUNTS.len() * section_header_size;
    let columns_start = field_meta_start + 26 * field_meta_size;
    let mut section_offsets = Vec::with_capacity(COUNTS.len());
    let mut id_offsets = Vec::with_capacity(COUNTS.len());
    let mut end = columns_start + 26 * field_storage_info_size;
    for (record_count, string_size) in COUNTS.into_iter().zip(STRING_SIZES) {
        section_offsets.push(end);
        end += record_count * 44 + string_size;
        id_offsets.push(end);
        end += record_count * 4;
    }

    let mut data = vec![0; end];
    put_u32(&mut data, 0, 0x3543_4457); // WDC5
    put_u32(&mut data, 4, 5);
    put_u32(&mut data, 136, 81);
    put_u32(&mut data, 140, 26);
    put_u32(&mut data, 144, 44);
    put_u32(&mut data, 152, 0xBD84_CD62);
    put_u32(&mut data, 156, 0xD43A_FAC3);
    put_u32(&mut data, 160, 1);
    put_u32(&mut data, 164, 81);
    put_u16(&mut data, 172, 4); // External ID lists.
    put_u16(&mut data, 174, 0);
    put_u32(&mut data, 176, 26);
    put_u32(&mut data, 188, (26 * field_storage_info_size) as u32);
    put_u32(&mut data, 200, 4);

    for field in 0..26 {
        let column = columns_start + field * field_storage_info_size;
        put_u16(&mut data, column + 2, 32);
    }

    let mut first_id = 1u32;
    for index in 0..COUNTS.len() {
        let section = sections_start + index * section_header_size;
        let tact_hash = if complete || index == 0 {
            0
        } else {
            index as u64
        };
        data[section..section + 8].copy_from_slice(&tact_hash.to_le_bytes());
        put_u32(&mut data, section + 8, section_offsets[index] as u32);
        put_u32(&mut data, section + 12, COUNTS[index] as u32);
        put_u32(&mut data, section + 16, STRING_SIZES[index] as u32);
        put_u32(&mut data, section + 24, (COUNTS[index] * 4) as u32);

        for record in 0..COUNTS[index] {
            put_u32(&mut data, id_offsets[index] + record * 4, first_id);
            first_id += 1;
        }
    }
    data
}

fn new_data_dir(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("rustycore-map-v25-{label}-{}", std::process::id()));
    fs::create_dir(&root).expect("create unique synthetic Map directory");
    fs::create_dir_all(root.join("dbc").join("enUS")).unwrap();
    root
}

fn write_map(data_dir: &Path, complete: bool) -> PathBuf {
    let path = data_dir.join("dbc").join("enUS").join("Map.db2");
    fs::write(&path, map_fixture(complete)).expect("write synthetic Map fixture");
    path
}

#[test]
fn map_store_loads_the_v25_readable_prefix_and_skips_tact_rows() {
    let data_dir = new_data_dir("available");
    let path = write_map(&data_dir, false);

    assert!(Wdc4Reader::open(&path).is_err());
    let maps = MapStore::load(data_dir.to_str().unwrap(), "enUS").unwrap();
    assert_eq!(maps.len(), 73);
    assert!(maps.get(1).is_some());
    assert!(maps.get(73).is_some());
    assert!(maps.get(74).is_none());

    fs::remove_dir_all(data_dir).unwrap();
}

#[test]
fn complete_map_without_tact_sections_uses_the_strict_reader() {
    let data_dir = new_data_dir("complete");
    let path = write_map(&data_dir, true);

    let strict = Wdc4Reader::open(&path).unwrap();
    assert_eq!(strict.record_count(), 81);
    let maps = MapStore::load(data_dir.to_str().unwrap(), "enUS").unwrap();
    assert_eq!(maps.len(), 81);
    assert!(maps.get(81).is_some());

    fs::remove_dir_all(data_dir).unwrap();
}
