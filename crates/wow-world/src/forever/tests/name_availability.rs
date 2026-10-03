//! Complete registered availability operation, without character mutations.
use super::*;
use name_rules::{NameRuleError, NameRules, Pattern, Patterns};
use std::future::Future;

fn reserved_name_permissions() -> Arc<permissions::DefaultAccountPermissions> {
    Arc::new(permissions::DefaultAccountPermissions::load(
        wow_persistence::forever::permissions::DefaultPermissionRows {
            known: vec![17, 195],
            links: vec![(195, 17)],
            roots: vec![195],
        },
    ))
}

fn request(name: &str) -> Request {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_uint32(0x12345678);
    packet.write_bits(name.len() as u32, 6);
    packet.write_bits(7, 3); // Source ignores these bits and surname validity.
    packet.write_bits(3, 6);
    packet.flush_bits();
    packet.write_string(name);
    packet.write_string("!?!");
    Request {
        opcode: 0x440071,
        payload: packet.into_data(),
    }
}

fn result(output: Vec<Outgoing>, code: u32) {
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].opcode(), 0x46001B);
    let mut expected = 0x12345678_u32.to_le_bytes().to_vec();
    expected.extend_from_slice(&code.to_le_bytes());
    assert_eq!(output[0].payload(), expected);
}

#[tokio::test]
async fn authenticated_lookup_needs_no_enum_and_preserves_sequence_and_normalized_spelling() {
    let catalog = CharacterCatalog::fixture();
    for used in [false, true] {
        let repository = Arc::new(Repository {
            used_name: used,
            ..Repository::good()
        });
        let mut session = session(repository.clone());
        assert!(matches!(
            session.dispatch(&catalog, request("eLuNe")).await,
            Err(SessionError::Phase)
        ));
        assert!(repository.queried_names.lock().unwrap().is_empty());
        session.initialize(&catalog, &policy(), 17).await.unwrap();
        result(
            session
                .dispatch(&catalog, request("eLuNe"))
                .await
                .unwrap()
                .unwrap(),
            if used { 27 } else { 0 },
        );
        assert_eq!(*repository.queried_names.lock().unwrap(), ["Elune"]);
        assert!(!session.enumerated);
        assert!(!session.is_closed());
    }
}

#[tokio::test]
async fn prefix_sql_reserved_and_malformed_requests_do_not_reach_collision_lookup() {
    let catalog = CharacterCatalog::fixture();
    let repository = Arc::new(Repository::good());
    let mut session = session(repository.clone());
    session.name_rules = Arc::new(NameRules::new(
        std::array::from_fn(|_| vec![]),
        vec![],
        ["Elune".into()],
        [],
    ));
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    for (name, code) in [
        ("", 98),
        ("A", 99),
        ("Abcdefghijklm", 100),
        ("Aaab", 107),
        ("eLuNe", 104),
    ] {
        result(
            session
                .dispatch(&catalog, request(name))
                .await
                .unwrap()
                .unwrap(),
            code,
        );
    }
    assert!(repository.queried_names.lock().unwrap().is_empty());
    let mut malformed = request("Elune");
    malformed.payload.pop();
    assert!(matches!(
        session.dispatch(&catalog, malformed).await,
        Err(SessionError::Protocol)
    ));
    assert!(session.is_closed());
    assert!(repository.queried_names.lock().unwrap().is_empty());
}

struct FixedPattern(Result<bool, NameRuleError>);
impl Pattern for FixedPattern {
    fn matches(&self, _: &[u16]) -> Result<bool, NameRuleError> {
        self.0
    }
}

#[tokio::test]
async fn sql_permission_does_not_bypass_db2_rules_or_engine_failures() {
    let catalog = CharacterCatalog::fixture();
    for (local, global, expected) in [
        (Ok(true), Ok(true), Ok(103)),
        (Ok(false), Ok(true), Ok(104)),
        (
            Err(NameRuleError::Engine),
            Ok(false),
            Err(SessionError::NameRules(NameRuleError::Engine)),
        ),
    ] {
        let repository = Arc::new(Repository::good());
        let mut session = session(repository.clone());
        let mut locales: [Patterns; 12] = std::array::from_fn(|_| vec![]);
        locales[6].push(Arc::new(FixedPattern(local)));
        session.name_rules = Arc::new(NameRules::new(
            locales,
            vec![Arc::new(FixedPattern(global))],
            ["Elune".into()],
            [],
        ));
        session.identity.permissions = reserved_name_permissions();
        session.initialize(&catalog, &policy(), 17).await.unwrap();
        match (session.dispatch(&catalog, request("Elune")).await, expected) {
            (Ok(Some(output)), Ok(code)) => result(output, code),
            (Err(error), Err(expected)) => {
                assert_eq!(error, expected);
                assert!(session.is_closed());
            }
            _ => panic!("unexpected name-operation outcome"),
        }
        assert!(repository.queried_names.lock().unwrap().is_empty());
    }
    let repository = Arc::new(Repository::good());
    let mut session = session(repository.clone());
    session.identity.permissions = reserved_name_permissions();
    session.name_rules = Arc::new(NameRules::new(
        std::array::from_fn(|_| vec![]),
        vec![],
        ["Elune".into()],
        [],
    ));
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    result(
        session
            .dispatch(&catalog, request("Elune"))
            .await
            .unwrap()
            .unwrap(),
        0,
    );
    assert_eq!(*repository.queried_names.lock().unwrap(), ["Elune"]);
}

#[tokio::test]
async fn database_failure_closes_without_publishing_an_available_result() {
    let repository = Arc::new(Repository {
        name_error: Some(LoadError::Database),
        ..Repository::good()
    });
    let mut session = session(repository.clone());
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    assert!(matches!(
        session.dispatch(&catalog, request("Elune")).await,
        Err(SessionError::Persistence(LoadError::Database))
    ));
    assert!(session.is_closed());
    assert_eq!(*repository.queried_names.lock().unwrap(), ["Elune"]);
}

#[tokio::test]
async fn cancelled_read_produces_no_result_and_transport_can_close_the_incarnation() {
    struct PendingName(Repository);
    impl SessionRepository for PendingName {
        fn load_account(
            &self,
            account: u32,
            bnet: u32,
            realm: u32,
        ) -> PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>> {
            self.0.load_account(account, bnet, realm)
        }
        fn load_character_selection(
            &self,
            account: u32,
            declined: bool,
        ) -> PersistenceFutureLikeCpp<
            '_,
            Result<wow_persistence::forever::selection::SelectionRows, LoadError>,
        > {
            self.0.load_character_selection(account, declined)
        }
        fn require_recustomization(
            &self,
            account: u32,
            guid: u64,
        ) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
            self.0.require_recustomization(account, guid)
        }
        fn name_in_use<'a>(
            &'a self,
            _: &'a str,
        ) -> PersistenceFutureLikeCpp<'a, Result<bool, LoadError>> {
            Box::pin(std::future::pending())
        }
    }
    let mut session = session(Arc::new(PendingName(Repository::good())));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let mut operation = Box::pin(session.dispatch(&catalog, request("Elune")));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(operation);
    session.close();
    assert!(matches!(
        session.dispatch(&catalog, request("Elune")).await,
        Err(SessionError::Phase)
    ));
}
