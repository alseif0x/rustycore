use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicUsize, Ordering},
};
use wow_data::{
    HotfixBlobCache,
    forever_hotfix::{ForeverHotfixCatalog, ForeverTactKeys, TACT_KEY_TABLE_HASH},
};
use wow_persistence::{PersistenceFutureLikeCpp, forever::AccountData};
mod item_hotfix;
mod name_availability;
mod selection;
mod templates;

fn hotfix_catalog(mut metadata: HotfixBlobCache) -> ForeverHotfixCatalog {
    // Header/section/column/records/IDs are synthetic. No acquired key bytes.
    let mut bytes = vec![0; 312];
    for (offset, value) in [
        (0, 0x35434457u32),
        (4, 5),
        (136, 2),
        (140, 1),
        (144, 16),
        (152, TACT_KEY_TABLE_HASH),
        (156, 0xCBA490FC),
        (160, 7),
        (164, 8),
        (176, 1),
        (188, 24),
        (200, 1),
        (212, 272),
        (216, 2),
        (228, 8),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[250..252].copy_from_slice(&128u16.to_le_bytes());
    bytes[272..288].fill(0xA5);
    bytes[288..304].fill(0xB6);
    bytes[304..308].copy_from_slice(&7u32.to_le_bytes());
    bytes[308..312].copy_from_slice(&8u32.to_le_bytes());
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "rustycore-forever-tact-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap(); // owns new directory, never overwrite
    let path = directory.join("TactKey.db2");
    std::fs::write(&path, bytes).unwrap();
    let keys = ForeverTactKeys::load(&directory);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
    metadata.register_typed_table(TACT_KEY_TABLE_HASH);
    ForeverHotfixCatalog::new(keys.unwrap(), metadata).unwrap()
}

struct Repository {
    load_error: Option<LoadError>,
    enum_error: Option<LoadError>,
    loads: AtomicUsize,
    name_error: Option<LoadError>,
    used_name: bool,
    queried_names: std::sync::Mutex<Vec<String>>,
    selection: wow_persistence::forever::selection::SelectionRows,
    recustomize_error: Option<LoadError>,
    recustomized: std::sync::Mutex<Vec<u64>>,
}

impl Repository {
    fn good() -> Self {
        Self {
            load_error: None,
            enum_error: None,
            loads: AtomicUsize::new(0),
            name_error: None,
            used_name: false,
            queried_names: Default::default(),
            selection: Default::default(),
            recustomize_error: None,
            recustomized: Default::default(),
        }
    }
}

fn snapshot() -> AccountSnapshot {
    AccountSnapshot {
        account_data: std::array::from_fn(|_| AccountData::default()),
        tutorials: [17; 8],
        instance_release_times: BTreeMap::new(),
        realm_character_counts: BTreeMap::new(),
    }
}

impl SessionRepository for Repository {
    fn load_account(
        &self,
        _: u32,
        _: u32,
        _: u32,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>> {
        Box::pin(async move {
            self.loads.fetch_add(1, Ordering::SeqCst);
            if let Some(error) = self.load_error {
                return Err(error);
            }
            Ok(snapshot())
        })
    }
    fn load_character_selection(
        &self,
        _: u32,
        _: bool,
    ) -> PersistenceFutureLikeCpp<
        '_,
        Result<wow_persistence::forever::selection::SelectionRows, LoadError>,
    > {
        Box::pin(async move {
            self.enum_error
                .map_or_else(|| Ok(self.selection.clone()), Err)
        })
    }
    fn require_recustomization(
        &self,
        _: u32,
        guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
        Box::pin(async move {
            self.recustomized.lock().unwrap().push(guid);
            self.recustomize_error.map_or(Ok(()), Err)
        })
    }
    fn name_in_use<'a>(
        &'a self,
        name: &'a str,
    ) -> PersistenceFutureLikeCpp<'a, Result<bool, LoadError>> {
        Box::pin(async move {
            self.queried_names.lock().unwrap().push(name.into());
            self.name_error.map_or(Ok(self.used_name), Err)
        })
    }
}

fn session(repository: Arc<dyn SessionRepository>) -> Session {
    Session::after_encryption(
        Identity {
            account_id: 1,
            battlenet_id: 1,
            realm_address: 0x02010001,
            account_expansion: 0,
            dbc_locale: 6,
            permissions: Arc::new(permissions::DefaultAccountPermissions::load(
                Default::default(),
            )),
        },
        repository,
        Arc::new(hotfix_catalog(HotfixBlobCache::new())),
        Arc::new(name_rules::NameRules::new(
            std::array::from_fn(|_| vec![]),
            vec![],
            Vec::new(),
            [],
        )),
        name_rules::NamePolicy {
            minimum_units: 2,
            strict_mask: 0,
            creation_charset: 2,
        },
        super::selection::fixture(),
    )
    .unwrap()
}

fn policy() -> InitializationPolicy {
    InitializationPolicy {
        realm_name: "Forever".into(),
        normalized_realm_name: "Forever".into(),
        timezone: "Etc/UTC".into(),
        cache_version: 0,
        content_set: 137,
        max_characters: 200,
        character_templates: Arc::new(
            creation::CharacterTemplates::load(
                Default::default(),
                &wow_data::forever_initialization::InitializationRecords::default()
                    .finish(Default::default(), Default::default(), &Default::default())
                    .unwrap(),
            )
            .unwrap(),
        ),
    }
}

#[test]
fn linked_registry_has_one_exact_metadata_and_call_set() {
    let entries: Vec<_> = inventory::iter::<Entry>.into_iter().collect();
    let rows: BTreeSet<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.opcode,
                format!("{:?}", entry.status),
                format!("{:?}", entry.processing),
                entry.handler_name,
            )
        })
        .collect();
    assert_eq!(entries.len(), rows.len());
    assert_eq!(
        rows,
        BTreeSet::from([
            (
                0x450006,
                "ConnectionEarly".into(),
                "Inplace".into(),
                "forever_ping"
            ),
            (
                0x450007,
                "ConnectionEarly".into(),
                "Inplace".into(),
                "forever_disconnect"
            ),
            (
                0x440014,
                "Authenticated".into(),
                "ThreadUnsafe".into(),
                "forever_enum"
            ),
            (
                0x440011,
                "Authenticated".into(),
                "ThreadUnsafe".into(),
                "forever_hotfix"
            ),
            (
                0x440010,
                "Authenticated".into(),
                "Inplace".into(),
                "forever_db_query"
            ),
            (
                0x440071,
                "Authenticated".into(),
                "ThreadUnsafe".into(),
                "forever_check_name"
            ),
        ])
    );
}

fn bulk_request(table_hash: u32, ids: &[u32]) -> Request {
    let mut payload = table_hash.to_le_bytes().to_vec();
    payload.extend_from_slice(&((ids.len() as u16) << 3).to_be_bytes());
    for id in ids {
        payload.extend_from_slice(&id.to_le_bytes());
    }
    Request {
        opcode: wow_packet::forever::db_query::CLASSIC_QUERY_OPCODE,
        payload,
    }
}

#[tokio::test]
async fn db_bulk_uses_registered_admission_and_typed_data_in_request_order() {
    let mut session = session(Arc::new(Repository::good()));
    let catalog = CharacterCatalog::fixture();
    assert!(matches!(
        session
            .dispatch(&catalog, bulk_request(TACT_KEY_TABLE_HASH, &[7]))
            .await,
        Err(SessionError::Phase)
    ));
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let output = session
        .dispatch(&catalog, bulk_request(TACT_KEY_TABLE_HASH, &[7, 99, 8, 7]))
        .await
        .unwrap()
        .unwrap();
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(output.len(), 4);
    for (reply, id) in output.iter().zip([7u32, 99, 8, 7]) {
        assert_eq!(reply.opcode(), 0x4A0000);
        let bytes = reply.payload();
        assert_eq!(&bytes[..4], &TACT_KEY_TABLE_HASH.to_le_bytes());
        assert_eq!(&bytes[4..8], &id.to_le_bytes());
        let timestamp = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as u64;
        assert!((before..=after).contains(&timestamp));
        if id == 99 {
            assert_eq!(&bytes[12..], &[0x60, 0, 0, 0, 0]);
        } else {
            assert_eq!(&bytes[12..17], &[0x20, 16, 0, 0, 0]);
            assert_eq!(&bytes[17..], &[if id == 7 { 0xA5 } else { 0xB6 }; 16]);
        }
    }
    assert!(!session.is_closed());
}

#[tokio::test]
async fn malformed_bulk_closes_and_unported_table_does_not_masquerade_as_absent_or_use_sql_blob() {
    let catalog = CharacterCatalog::fixture();
    let mut session = session(Arc::new(Repository::good()));
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let mut request = bulk_request(TACT_KEY_TABLE_HASH, &[7]);
    request.payload.pop();
    assert!(matches!(
        session.dispatch(&catalog, request).await,
        Err(SessionError::Protocol)
    ));
    assert!(session.is_closed());

    let mut session = super::tests::session(Arc::new(Repository::good()));
    let mut metadata = HotfixBlobCache::new();
    metadata.insert_hotfix_blob(123, 7, vec![1, 2]);
    session.hotfixes = Arc::new(hotfix_catalog(metadata));
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    assert!(matches!(
        session.dispatch(&catalog, bulk_request(123, &[7])).await,
        Err(SessionError::Protocol)
    ));
    assert!(session.is_closed());
}

#[tokio::test]
async fn known_unported_store_is_an_error_not_a_fake_missing_record() {
    let mut session = session(Arc::new(Repository::good()));
    let mut metadata = HotfixBlobCache::new();
    metadata.register_typed_table(123);
    session.hotfixes = Arc::new(hotfix_catalog(metadata));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    assert!(matches!(
        session.dispatch(&catalog, bulk_request(123, &[7])).await,
        Err(SessionError::Protocol)
    ));
    assert!(session.is_closed());
}

#[tokio::test]
async fn hotfix_connect_prefers_typed_record_then_blob_and_marks_actual_missing_record_removed() {
    let mut metadata = HotfixBlobCache::new();
    metadata.register_typed_table(TACT_KEY_TABLE_HASH);
    metadata.insert_hotfix_blob(TACT_KEY_TABLE_HASH, 99, vec![0xCC, 0xDD]);
    metadata.apply_hotfix_data_rows_like_cpp(
        [
            (1, 1, TACT_KEY_TABLE_HASH, 7, 1),
            (2, 2, TACT_KEY_TABLE_HASH, 99, 1),
            (3, 3, TACT_KEY_TABLE_HASH, 999, 1),
        ],
        "esES",
    );
    let mut session = session(Arc::new(Repository::good()));
    session.hotfixes = Arc::new(hotfix_catalog(metadata));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let mut payload = 70170u32.to_le_bytes().repeat(2);
    payload.extend_from_slice(&3u32.to_le_bytes());
    for id in [1i32, 2, 3] {
        payload.extend_from_slice(&id.to_le_bytes());
    }
    let output = session
        .dispatch(
            &catalog,
            Request {
                opcode: HOTFIX_REQUEST,
                payload,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].opcode(), 0x4A0003);
    let bytes = output[0].payload();
    assert_eq!(&bytes[..4], &3u32.to_le_bytes());
    assert_eq!((bytes[24], bytes[45], bytes[66]), (0x20, 0x20, 0x40));
    assert_eq!(&bytes[67..71], &18u32.to_le_bytes());
    assert_eq!(&bytes[71..87], &[0xA5; 16]);
    assert_eq!(&bytes[87..], &[0xCC, 0xDD]);
}

#[tokio::test]
async fn query_holder_failure_never_publishes_auth_success() {
    for error in [
        LoadError::Database,
        LoadError::InvalidRow,
        LoadError::UnsupportedState,
    ] {
        let mut repository = Repository::good();
        repository.load_error = Some(error);
        let mut session = session(Arc::new(repository));
        let result = session
            .initialize(&CharacterCatalog::fixture(), &policy(), 17)
            .await;
        assert!(matches!(result, Err(SessionError::Persistence(actual)) if actual == error));
        assert!(session.snapshot.is_none());
        assert_eq!(session.phase, Phase::Closed);
    }
}

#[tokio::test]
async fn initialized_snapshot_is_owned_before_the_exact_ordered_batch() {
    let repository = Arc::new(Repository::good());
    let mut session = session(repository.clone());
    let output = session
        .initialize(&CharacterCatalog::fixture(), &policy(), 17)
        .await
        .unwrap();
    assert_eq!(repository.loads.load(Ordering::SeqCst), 1);
    assert_eq!(session.phase, Phase::Authenticated);
    assert_eq!(session.snapshot.as_ref().unwrap().tutorials, [17; 8]);
    assert_eq!(
        output.iter().map(Outgoing::opcode).collect::<Vec<_>>(),
        [
            0x460001, 0x460123, 0x460064, 0x4A000E, 0x4A0001, 0x4601B5, 0x460268, 0x4602B1
        ]
    );
    assert_eq!(&output[0].payload()[..5], &[0, 0, 0, 0, 0x80]);
    assert_eq!(output[6].payload(), &[17, 0, 0, 0].repeat(8));
    assert_eq!(output[7].payload(), &[0x60]); // State=1, source default suppress=true.
    assert!(matches!(
        session
            .initialize(&CharacterCatalog::fixture(), &policy(), 17)
            .await,
        Err(SessionError::Phase)
    ));
}

#[tokio::test]
async fn registry_thunk_and_phase_admission_not_an_opcode_match_run_the_operation() {
    let mut session = session(Arc::new(Repository::good()));
    let catalog = CharacterCatalog::fixture();
    assert!(matches!(
        session
            .dispatch(
                &catalog,
                Request {
                    opcode: ENUM_CHARACTERS,
                    payload: vec![]
                }
            )
            .await,
        Err(SessionError::Phase)
    ));
    let pong = session
        .dispatch(
            &catalog,
            Request {
                opcode: 0x450006,
                payload: vec![1, 2, 3, 4, 9, 0, 0, 0],
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.latency, 9);
    assert_eq!(pong[0].opcode(), 0x4D0009);
    assert_eq!(pong[0].payload(), &[1, 2, 3, 4]);
    let activity = session.character_activity;
    assert!(
        session
            .dispatch(
                &catalog,
                Request {
                    opcode: 0xFFFF_FFFF,
                    payload: vec![]
                }
            )
            .await
            .unwrap()
            .is_none()
    );
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    assert!(session.character_activity >= activity);
    session.character_activity = std::time::Instant::now() - std::time::Duration::from_secs(2);
    let activity = session.character_activity;
    session
        .dispatch(
            &catalog,
            Request {
                opcode: 0x450006,
                payload: vec![0; 8],
            },
        )
        .await
        .unwrap();
    assert_eq!(session.character_activity, activity);
    session
        .dispatch(
            &catalog,
            Request {
                opcode: 0xFFFF_FFFF,
                payload: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(session.character_activity, activity);
    assert!(
        session
            .character_idle_remaining(std::time::Duration::from_secs(1))
            .is_none()
    );
    let output = session
        .dispatch(
            &catalog,
            Request {
                opcode: ENUM_CHARACTERS,
                payload: vec![],
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.iter().map(Outgoing::opcode).collect::<Vec<_>>(),
        [0x460018, 0x460362, 0x460360]
    );
    assert!(session.has_enumerated());
    assert!(session.character_activity > activity);
    assert_eq!(&output[0].payload()[10..14], &1_i32.to_le_bytes());
    assert_eq!(output[1].payload(), &[0; 9]);
    assert_eq!(output[2].payload(), &[0; 4]);
}

#[tokio::test]
async fn malformed_or_failed_enumeration_closes_without_success() {
    for bad_payload in [false, true] {
        let mut repository = Repository::good();
        if !bad_payload {
            repository.enum_error = Some(LoadError::Database);
        }
        let mut session = session(Arc::new(repository));
        let catalog = CharacterCatalog::fixture();
        session.initialize(&catalog, &policy(), 17).await.unwrap();
        assert!(
            session
                .dispatch(
                    &catalog,
                    Request {
                        opcode: ENUM_CHARACTERS,
                        payload: if bad_payload { vec![0] } else { vec![] }
                    }
                )
                .await
                .is_err()
        );
        assert!(!session.enumerated);
        assert_eq!(session.phase, Phase::Closed);
    }
}

#[tokio::test]
async fn invalid_packet_policy_never_returns_a_partial_auth_batch() {
    let mut session = session(Arc::new(Repository::good()));
    let mut policy = policy();
    policy.realm_name = "x".repeat(256);
    assert!(matches!(
        session
            .initialize(&CharacterCatalog::fixture(), &policy, 17)
            .await,
        Err(SessionError::Codec)
    ));
    assert!(session.snapshot.is_none());
    assert_eq!(session.phase, Phase::Closed);
}

#[tokio::test]
async fn cancelled_initialization_cannot_be_restarted_or_admit_character_operations() {
    struct PendingRepository;
    impl SessionRepository for PendingRepository {
        fn load_account(
            &self,
            _: u32,
            _: u32,
            _: u32,
        ) -> PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>> {
            Box::pin(std::future::pending())
        }
        fn load_character_selection(
            &self,
            _: u32,
            _: bool,
        ) -> PersistenceFutureLikeCpp<
            '_,
            Result<wow_persistence::forever::selection::SelectionRows, LoadError>,
        > {
            panic!("cancelled initialization must never enumerate")
        }
        fn require_recustomization(
            &self,
            _: u32,
            _: u64,
        ) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
            panic!("cancelled initialization must never recustomize")
        }
        fn name_in_use<'a>(
            &'a self,
            _: &'a str,
        ) -> PersistenceFutureLikeCpp<'a, Result<bool, LoadError>> {
            panic!("cancelled initialization must never check names")
        }
    }
    let mut session = session(Arc::new(PendingRepository));
    let catalog = CharacterCatalog::fixture();
    let policy = policy();
    let mut pending = Box::pin(session.initialize(&catalog, &policy, 17));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(pending);
    assert_eq!(session.phase, Phase::Initializing);
    assert!(session.snapshot.is_none());
    assert!(matches!(
        session.initialize(&catalog, &policy, 17).await,
        Err(SessionError::Phase)
    ));
    assert!(matches!(
        session
            .dispatch(
                &catalog,
                Request {
                    opcode: ENUM_CHARACTERS,
                    payload: vec![]
                }
            )
            .await,
        Err(SessionError::Phase)
    ));
    session.close();
    assert!(session.is_closed());
}
