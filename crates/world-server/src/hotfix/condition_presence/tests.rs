use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use wow_persistence::{
    ConditionPresenceLoadOutcome, ConditionPresencePersistencePort, ConditionPresenceRows,
    PersistenceFutureLikeCpp,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Db2FixtureDirectory(PathBuf);

impl Db2FixtureDirectory {
    fn new(with_files: bool) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rustycore-world-condition-presence-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        if with_files {
            let locale = path.join("dbc/enUS");
            fs::create_dir_all(&locale).expect("create DB2 fixture directory");
            write_empty_wdc4(&locale.join("PlayerCondition.db2"), 0x1111);
            write_empty_wdc4(&locale.join("UnitCondition.db2"), 0x2222);
        }
        Self(path)
    }

    fn data_dir(&self) -> &str {
        self.0.to_str().expect("temporary path is UTF-8")
    }
}

impl Drop for Db2FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct StubPort {
    outcome: ConditionPresenceLoadOutcome,
    calls: AtomicUsize,
}

impl StubPort {
    fn new(outcome: ConditionPresenceLoadOutcome) -> Self {
        Self {
            outcome,
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl ConditionPresencePersistencePort for StubPort {
    fn load_condition_presence_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ConditionPresenceLoadOutcome> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.outcome.clone()
        })
    }
}

fn write_empty_wdc4(path: &Path, table_hash: u32) {
    const HEADER_SIZE: usize = 72;
    const SECTION_HEADER_SIZE: usize = 40;
    let mut bytes = vec![0; HEADER_SIZE + SECTION_HEADER_SIZE];
    put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
    put_u32(&mut bytes, 20, table_hash);
    put_u16(&mut bytes, 42, u16::MAX);
    put_u32(&mut bytes, 68, 1);
    put_u32(
        &mut bytes,
        HEADER_SIZE + 8,
        (HEADER_SIZE + SECTION_HEADER_SIZE) as u32,
    );
    fs::write(path, bytes).expect("write empty WDC4 fixture");
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[tokio::test]
async fn composition_publishes_both_id_sets_from_the_typed_port() {
    let files = Db2FixtureDirectory::new(true);
    let persistence = StubPort::new(ConditionPresenceLoadOutcome::Loaded(
        ConditionPresenceRows {
            player_condition_ids: vec![0, 7],
            unit_condition_ids: vec![u32::MAX],
        },
    ));
    let catalog = load_condition_presence_catalog(
        files.data_dir(),
        "enUS",
        &persistence,
        &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .await
    .expect("compose complete condition catalog");

    assert_eq!(persistence.calls(), 1);
    assert!(catalog.contains_player_condition(0));
    assert!(catalog.contains_player_condition(7));
    assert!(!catalog.contains_player_condition(u32::MAX));
    assert!(catalog.contains_unit_condition(u32::MAX));
    assert!(!catalog.contains_unit_condition(7));
}

#[tokio::test]
async fn typed_persistence_failure_returns_no_catalog() {
    let files = Db2FixtureDirectory::new(true);
    let persistence = StubPort::new(ConditionPresenceLoadOutcome::Failed {
        reason: "condition query failed".into(),
    });
    let error = load_condition_presence_catalog(
        files.data_dir(),
        "enUS",
        &persistence,
        &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .await
    .err()
    .expect("failed SQL load must not publish partial file-only presence");

    assert_eq!(error.to_string(), "condition query failed");
    assert_eq!(persistence.calls(), 1);
}

#[tokio::test]
async fn successful_empty_catalog_is_known_while_missing_files_fail_before_sql() {
    let files = Db2FixtureDirectory::new(true);
    let empty = StubPort::new(ConditionPresenceLoadOutcome::Loaded(
        ConditionPresenceRows::default(),
    ));
    let catalog = load_condition_presence_catalog(
        files.data_dir(),
        "enUS",
        &empty,
        &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .await
    .expect("empty successful load is a known catalog");
    assert!(!catalog.contains_player_condition(0));
    assert!(!catalog.contains_unit_condition(0));
    assert_eq!(empty.calls(), 1);

    let missing_files = Db2FixtureDirectory::new(false);
    let untouched = StubPort::new(ConditionPresenceLoadOutcome::Loaded(
        ConditionPresenceRows::default(),
    ));
    assert!(
        load_condition_presence_catalog(
            missing_files.data_dir(),
            "enUS",
            &untouched,
            &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .await
        .is_err()
    );
    assert_eq!(untouched.calls(), 0);
}
