use crate::handlers::test_support::world::make_session;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wow_core::ObjectGuid;
use wow_persistence::{
    CharacterEnumerationLoadOutcomeLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
    CharacterEnumerationRequestLikeCpp, CharacterEnumerationRowLikeCpp, PersistenceFutureLikeCpp,
};

struct CharacterEnumerationPortFixtureLikeCpp {
    requests: Mutex<Vec<CharacterEnumerationRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<CharacterEnumerationLoadOutcomeLikeCpp>>,
}

impl CharacterEnumerationPortFixtureLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = CharacterEnumerationLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<CharacterEnumerationRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl CharacterEnumerationPersistencePortLikeCpp for CharacterEnumerationPortFixtureLikeCpp {
    fn load_character_enumeration_like_cpp<'a>(
        &'a self,
        request: CharacterEnumerationRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CharacterEnumerationLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one character-enumeration outcome per request");
        Box::pin(async move { outcome })
    }
}

fn character_enumeration_row_like_cpp() -> CharacterEnumerationRowLikeCpp {
    CharacterEnumerationRowLikeCpp {
        guid_low: 42,
        name: "PortBoundary".to_owned(),
        race: 1,
        class: 1,
        gender: 0,
        level: 20,
        zone: 12,
        map: 0,
        position_x: 1.0,
        position_y: 2.0,
        position_z: 3.0,
        guild_id: 0,
        player_flags: 0,
        at_login_flags: 0,
        pet_entry: 0,
        pet_display_id: 0,
        pet_level: 0,
        equipment_cache: String::new(),
        banned_guid: 0,
        list_slot: 0,
        last_played_time: 100,
        active_talent_group: 0,
        last_login_build: 54261,
        declined_genitive: "PortBoundaryGenitive".to_owned(),
    }
}

#[tokio::test]
async fn character_enumeration_uses_typed_rows_and_keeps_cleanup_best_effort_like_cpp() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
            rows: vec![character_enumeration_row_like_cpp()],
            expired_ban_cleanup_error: Some("best-effort cleanup failed".to_owned()),
        },
    ]);
    let (mut session, send_rx) = make_session();
    session.set_declined_names_used_like_cpp(true);
    session.set_character_enumeration_persistence_port_like_cpp(port.clone());

    session.handle_enum_characters().await;

    assert_eq!(
        port.requests(),
        vec![CharacterEnumerationRequestLikeCpp {
            account_id: 1,
            declined_names_used: true,
        }]
    );
    assert!(session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}

// The port failure outcome is synthetic Rust adapter behavior, not a C++ parity claim.
#[tokio::test]
async fn character_enumeration_query_failure_publishes_failure_and_no_legit_guid() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Failed {
            reason: "query failed".to_owned(),
            expired_ban_cleanup_error: None,
        },
    ]);
    let (mut session, send_rx) = make_session();
    session.set_character_enumeration_persistence_port_like_cpp(port);

    session.handle_enum_characters().await;

    assert!(!session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}
