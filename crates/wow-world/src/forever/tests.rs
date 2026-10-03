use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicUsize, Ordering},
};
use wow_persistence::{PersistenceFutureLikeCpp, forever::AccountData};

struct Repository {
    load_error: Option<LoadError>,
    enum_error: Option<LoadError>,
    loads: AtomicUsize,
}

impl Repository {
    fn good() -> Self {
        Self {
            load_error: None,
            enum_error: None,
            loads: AtomicUsize::new(0),
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
    fn enumerate_empty(&self, _: u32) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
        Box::pin(async move { self.enum_error.map_or(Ok(()), Err) })
    }
}

fn session(repository: Arc<dyn SessionRepository>) -> Session {
    Session::after_encryption(
        Identity {
            account_id: 1,
            battlenet_id: 1,
            realm_address: 0x02010001,
            account_expansion: 0,
        },
        repository,
        Arc::new(HotfixBlobCache::new()),
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
        ])
    );
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
        fn enumerate_empty(&self, _: u32) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
            panic!("cancelled initialization must never enumerate")
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
