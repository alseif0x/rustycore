use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Db2FixtureDirectory(PathBuf);

impl Db2FixtureDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rustycore-condition-presence-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(path.join("dbc/enUS")).expect("create DB2 fixture directory");
        Self(path)
    }

    fn data_dir(&self) -> &str {
        self.0.to_str().expect("temporary path is UTF-8")
    }

    fn write(&self, filename: &str, table_hash: u32, ids: &[u32]) {
        write_wdc4(&self.0.join("dbc/enUS").join(filename), table_hash, ids);
    }
}

impl Drop for Db2FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_wdc4(path: &Path, table_hash: u32, ids: &[u32]) {
    const HEADER_SIZE: usize = 72;
    const SECTION_HEADER_SIZE: usize = 40;
    let data_offset = HEADER_SIZE + SECTION_HEADER_SIZE;
    let mut bytes = vec![0; data_offset + ids.len() * 4];
    put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
    put_u32(&mut bytes, 4, ids.len() as u32);
    put_u32(&mut bytes, 20, table_hash);
    put_u16(&mut bytes, 40, if ids.is_empty() { 0 } else { 0x04 });
    put_u16(&mut bytes, 42, u16::MAX);
    put_u32(&mut bytes, 68, 1);

    let section = HEADER_SIZE;
    put_u32(&mut bytes, section + 8, data_offset as u32);
    put_u32(&mut bytes, section + 12, ids.len() as u32);
    put_u32(&mut bytes, section + 24, (ids.len() * 4) as u32);
    for (index, id) in ids.iter().enumerate() {
        put_u32(&mut bytes, data_offset + index * 4, *id);
    }
    fs::write(path, bytes).expect("write WDC4 fixture");
}

#[test]
fn condition_presence_unions_file_and_sql_ids_then_applies_table_scoped_final_removals() {
    let fixtures = Db2FixtureDirectory::new();
    let player_hash = 0x1357_2468;
    let unit_hash = 0x2468_1357;
    fixtures.write("PlayerCondition.db2", player_hash, &[0, 5, 0x8000_0001]);
    fixtures.write("UnitCondition.db2", unit_hash, &[7, 0x8000_0001]);

    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (player_hash, 9, 2),
        (player_hash, 5, 2),
        (player_hash, 5, 1),
        (player_hash, 7, 2),
        (player_hash, -1, 2),
        (unit_hash, 0x8000_0001_u32 as i32, 2),
        (unit_hash, 0x8000_0001_u32 as i32, 1),
    ]);
    let catalog = ConditionPresenceCatalogFiles::load(fixtures.data_dir(), "enUS")
        .expect("load complete condition files")
        .apply_hotfix_ids([0, 5, 9, 9, u32::MAX], [0, 7, 8, 0x8000_0001], &removals)
        .expect("apply effective hotfix presence");

    assert!(catalog.contains_player_condition(0));
    assert!(catalog.contains_player_condition(5));
    assert!(!catalog.contains_player_condition(9));
    assert!(catalog.contains_player_condition(0x8000_0001));
    assert!(!catalog.contains_player_condition(u32::MAX));
    assert!(!catalog.contains_player_condition(7));
    assert!(catalog.contains_unit_condition(0));
    assert!(catalog.contains_unit_condition(7));
    assert!(catalog.contains_unit_condition(8));
    assert!(catalog.contains_unit_condition(0x8000_0001));
    assert_eq!(catalog.player_conditions.len(), 3);
    assert_eq!(catalog.unit_conditions.len(), 4);
}

#[test]
fn empty_success_is_known_empty_while_missing_files_are_unavailable() {
    let empty_files = Db2FixtureDirectory::new();
    empty_files.write("PlayerCondition.db2", 0x1111, &[]);
    empty_files.write("UnitCondition.db2", 0x2222, &[]);
    let catalog = ConditionPresenceCatalogFiles::load(empty_files.data_dir(), "enUS")
        .expect("empty DB2 files are complete")
        .apply_hotfix_ids([], [], &Db2HotfixRemovalStoreLikeCpp::default())
        .expect("empty hotfix result is known");

    assert!(!catalog.contains_player_condition(0));
    assert!(!catalog.contains_unit_condition(0));

    let unavailable = Db2FixtureDirectory::new();
    assert!(ConditionPresenceCatalogFiles::load(unavailable.data_dir(), "enUS").is_err());
}

#[test]
fn effective_id_constructor_distinguishes_empty_from_id_zero() {
    let empty = ConditionPresenceCatalog::from_effective_ids([], []);
    assert!(!empty.contains_player_condition(0));
    assert!(!empty.contains_unit_condition(0));

    let zero = ConditionPresenceCatalog::from_effective_ids([0], [0]);
    assert!(zero.contains_player_condition(0));
    assert!(zero.contains_unit_condition(0));
}

#[test]
#[ignore = "requires the installed 3.4.3 DB2 data directory"]
fn condition_presence_installed_files() {
    let data_dir = std::env::var("RUSTYCORE_CONDITION_DATA_DIR")
        .expect("set RUSTYCORE_CONDITION_DATA_DIR to the installed data directory");
    let files = ConditionPresenceCatalogFiles::load(&data_dir, "enUS")
        .expect("load complete installed PlayerCondition and UnitCondition files");

    println!(
        "complete file phase: PlayerCondition rows={} ID0={}; UnitCondition rows={} ID0={}",
        files.player_conditions.len(),
        files.player_conditions.contains(0),
        files.unit_conditions.len(),
        files.unit_conditions.contains(0),
    );
}
