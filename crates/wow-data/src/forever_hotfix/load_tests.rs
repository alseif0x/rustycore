//! Synthetic production-loader coverage for the Forever TactKey baseline.
//!
//! These fixtures contain only artificial bytes.  They exercise the real
//! `ForeverTactKeys::load` path without copying client assets or key material.

use super::{ForeverTactKeys, TACT_KEY_LAYOUT_HASH, TACT_KEY_TABLE_HASH};
use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const WDC5_HEADER_SIZE: usize = 204;
const SECTION_HEADER_SIZE: usize = 40;
const FIELD_META_SIZE: usize = 4;
const FIELD_STORAGE_INFO_SIZE: usize = 24;
const RECORD_SIZE: usize = 16;
const DIRECT_RECORDS: usize = 2;
const COPY_RECORDS: usize = 1;
const SECTION_OFFSET: usize = WDC5_HEADER_SIZE;
const FIELD_META_OFFSET: usize = SECTION_OFFSET + SECTION_HEADER_SIZE;
const COLUMN_OFFSET: usize = FIELD_META_OFFSET + FIELD_META_SIZE;
const RECORD_OFFSET: usize = COLUMN_OFFSET + FIELD_STORAGE_INFO_SIZE;
const RECORD_END: usize = RECORD_OFFSET + DIRECT_RECORDS * RECORD_SIZE;
const ID_LIST_OFFSET: usize = RECORD_END;
const ID_LIST_SIZE: usize = DIRECT_RECORDS * 4;
const COPY_OFFSET: usize = ID_LIST_OFFSET + ID_LIST_SIZE;
const FILE_SIZE: usize = COPY_OFFSET + COPY_RECORDS * 8;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct FixtureDirectory {
    path: PathBuf,
}

impl FixtureDirectory {
    fn create(name: &str) -> Self {
        for _ in 0..64 {
            let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustycore-forever-tact-{}-{serial}-{name}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create synthetic fixture directory: {error}"),
            }
        }
        panic!("could not allocate a unique synthetic fixture directory");
    }

    fn write_db2(&self, data: &[u8]) {
        let path = self.path.join("TactKey.db2");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("create synthetic TactKey.db2 without clobbering");
        file.write_all(data).expect("write synthetic TactKey.db2");
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.path.join("TactKey.db2"));
        let _ = fs::remove_dir(&self.path);
    }
}

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn tact_key_fixture() -> Vec<u8> {
    let mut data = vec![0; FILE_SIZE];

    // WDC5 header.  The target header uses flags=4 and header id_index=0;
    // the external-ID declaration is the typed DB2 metadata, not 0xFFFF here.
    put_u32(&mut data, 0, 0x3543_4457);
    put_u32(&mut data, 4, 5);
    put_u32(&mut data, 136, DIRECT_RECORDS as u32);
    put_u32(&mut data, 140, 1);
    put_u32(&mut data, 144, RECORD_SIZE as u32);
    put_u32(&mut data, 152, TACT_KEY_TABLE_HASH);
    put_u32(&mut data, 156, TACT_KEY_LAYOUT_HASH);
    put_u16(&mut data, 172, 4); // external section ID list, not sparse
    put_u16(&mut data, 174, 0); // native WDC5 header id_index
    put_u32(&mut data, 176, 1);
    put_u32(&mut data, 184, 0); // parent lookup count
    put_u32(&mut data, 188, FIELD_STORAGE_INFO_SIZE as u32);
    put_u32(&mut data, 200, 1);

    // One fixed, unencrypted section with two direct records and one copy.
    put_u64(&mut data, SECTION_OFFSET, 0);
    put_u32(&mut data, SECTION_OFFSET + 8, RECORD_OFFSET as u32);
    put_u32(&mut data, SECTION_OFFSET + 12, DIRECT_RECORDS as u32);
    put_u32(&mut data, SECTION_OFFSET + 20, RECORD_END as u32);
    put_u32(&mut data, SECTION_OFFSET + 24, ID_LIST_SIZE as u32);
    put_u32(&mut data, SECTION_OFFSET + 36, COPY_RECORDS as u32);

    // One uncompressed, byte-aligned 16-byte field.
    put_u16(&mut data, COLUMN_OFFSET, 0);
    put_u16(&mut data, COLUMN_OFFSET + 2, 128);
    put_u32(&mut data, COLUMN_OFFSET + 8, 0); // CompressionType::None

    for (index, value) in [0x10_u8, 0x20].into_iter().enumerate() {
        let start = RECORD_OFFSET + index * RECORD_SIZE;
        data[start..start + RECORD_SIZE].fill(value);
    }
    put_u32(&mut data, ID_LIST_OFFSET, 7);
    put_u32(&mut data, ID_LIST_OFFSET + 4, 8);
    put_u32(&mut data, COPY_OFFSET, 9);
    put_u32(&mut data, COPY_OFFSET + 4, 7);
    data
}

fn load_fixture(name: &str, data: &[u8]) -> Result<ForeverTactKeys> {
    let directory = FixtureDirectory::create(name);
    directory.write_db2(data);
    let result = ForeverTactKeys::load(Path::new(&directory.path));
    drop(directory);
    result
}

fn rejects(name: &str, data: Vec<u8>) {
    assert!(
        load_fixture(name, &data).is_err(),
        "malformed synthetic TactKey fixture was accepted: {name}"
    );
}

#[test]
fn production_loader_reads_external_ids_and_copied_fixed_byte_records() {
    let keys = load_fixture("positive", &tact_key_fixture()).expect("valid TactKey fixture");
    assert_eq!(keys.count(), 3);
    assert_eq!(keys.records.get(&7), Some(&[0x10; 16]));
    assert_eq!(keys.records.get(&8), Some(&[0x20; 16]));
    assert_eq!(keys.records.get(&9), Some(&[0x10; 16]));
}

#[test]
fn production_loader_rejects_wrong_schema_and_version() {
    let mut wrong_table = tact_key_fixture();
    put_u32(&mut wrong_table, 152, 0);
    rejects("wrong-table", wrong_table);

    let mut wrong_layout = tact_key_fixture();
    put_u32(&mut wrong_layout, 156, 0);
    rejects("wrong-layout", wrong_layout);

    let mut wrong_version = tact_key_fixture();
    put_u32(&mut wrong_version, 4, 4);
    rejects("wrong-version", wrong_version);
}

#[test]
fn production_loader_rejects_missing_or_inline_id_source() {
    let mut missing_ids = tact_key_fixture();
    put_u32(&mut missing_ids, SECTION_OFFSET + 24, 4);
    rejects("missing-id-list", missing_ids);

    let mut inline_id = tact_key_fixture();
    put_u16(&mut inline_id, 172, 0);
    put_u16(&mut inline_id, COLUMN_OFFSET + 2, 32);
    rejects("inline-id", inline_id);
}

#[test]
fn production_loader_rejects_truncation_and_unresolved_copies() {
    let mut truncated = tact_key_fixture();
    truncated.truncate(RECORD_OFFSET + 1);
    rejects("truncated-record", truncated);

    let mut unresolved_copy = tact_key_fixture();
    put_u32(&mut unresolved_copy, COPY_OFFSET + 4, 0xFFFF_FFFE);
    rejects("unresolved-copy", unresolved_copy);
}

#[test]
fn production_loader_rejects_unsupported_fixed_byte_representation() {
    let mut bad_width = tact_key_fixture();
    put_u16(&mut bad_width, COLUMN_OFFSET + 2, 64);
    rejects("bad-array-width", bad_width);

    let mut compressed = tact_key_fixture();
    put_u32(&mut compressed, COLUMN_OFFSET + 8, 1); // CompressionType::Bitpacked
    rejects("compressed-array", compressed);
}
