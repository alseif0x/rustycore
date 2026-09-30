//! Explicit directory hydration exercised against the feature-enabled library.

use super::*;

fn seed_canonical_directory_inputs(session: &WorldSession) {
    mutate_canonical_player_for_test(session, |player| {
        let state = player.gameplay_state_mut();
        state.spells.replace_known_spells_and_rows_like_cpp(
            vec![321],
            [(
                321,
                wow_entities::PlayerKnownSpellRecord {
                    spell_id: 321,
                    state: wow_entities::PlayerSpellLoadState::Unchanged,
                    active: true,
                    disabled: false,
                    favorite: false,
                    dependent: false,
                },
            )]
            .into_iter()
            .collect(),
        );
        state.vehicle_seat_flags = Some(7);
        state.vehicle_seat_id = Some(8);
    })
    .expect("canonical directory fixture owner");
}

fn canonical_directory_inputs(session: &WorldSession) -> (Vec<i32>, Option<i32>, Option<u32>) {
    let manager = canonical_map_manager_for_test(session)
        .unwrap()
        .lock()
        .unwrap();
    let state = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(session.player_guid().unwrap())
        .unwrap()
        .gameplay_state();
    (
        state.spells.known_spells_like_cpp().to_vec(),
        state.vehicle_seat_flags,
        state.vehicle_seat_id,
    )
}

#[test]
fn explicit_registration_hydrates_before_missing_registry_guard() {
    let (mut session, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    seed_canonical_directory_inputs(&session);

    register_in_player_registry_production_for_test(&session);
    assert_eq!(
        canonical_directory_inputs(&session),
        (vec![321], Some(7), Some(8))
    );

    register_in_player_registry_for_test(&session);
    assert_eq!(canonical_directory_inputs(&session), (vec![], None, None));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn feature_enabled_production_binding_preserves_canonical_inputs_until_explicit_hydration() {
    for synchronize in [false, true] {
        let (mut session, send_rx) = make_session();
        install_canonical_player_owner_for_test(&mut session, 571, 0);
        let registry = Arc::new(PlayerRegistry::default());
        session.set_player_registry(Arc::clone(&registry));
        register_in_player_registry_production_for_test(&session);
        assert_eq!(registry.fixture_count(), 1);
        seed_canonical_directory_inputs(&session);

        if synchronize {
            sync_player_registry_state_production_for_test(&session);
        } else {
            register_in_player_registry_production_for_test(&session);
        }
        assert_eq!(
            canonical_directory_inputs(&session),
            (vec![321], Some(7), Some(8))
        );

        if synchronize {
            sync_player_registry_state_for_test(&session);
        } else {
            register_in_player_registry_for_test(&session);
        }
        assert_eq!(canonical_directory_inputs(&session), (vec![], None, None));
        assert_eq!(registry.fixture_count(), 1);
        assert!(send_rx.try_recv().is_err());
    }
}

#[test]
fn explicit_registration_hydrates_the_owner_created_during_registration() {
    for explicit_hydration in [false, true] {
        let (mut session, send_rx) = make_session();
        let guid = session.player_guid().unwrap();
        set_loaded_player_name_like_cpp(&mut session, "HydrationPartyMember".into());
        let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
        session.set_player_registry(Arc::clone(&registry));
        add_active_quest_in_slot(&mut session, 7720, 2);

        if explicit_hydration {
            register_in_player_registry_for_test(&session);
        } else {
            register_in_player_registry_production_for_test(&session);
        }
        assert_eq!(registry.fixture_count(), 1);
        let manager = registry.fixture_canonical_map_manager_like_cpp().unwrap();
        let snapshot = registry
            .quest_sharing_snapshot(guid, Some(&manager))
            .unwrap();
        assert_eq!(
            snapshot.active_quest_statuses.get(&7720).copied(),
            explicit_hydration.then_some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        );
        assert!(send_rx.try_recv().is_err());
    }
}

#[tokio::test]
async fn explicit_party_hydration_preserves_prerequisite_packet_bytes_and_recipients() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().unwrap();
    let receiver_guid = ObjectGuid::create_player(1, 7722);
    let quest_id = 7721;
    let store = store_with_sharable_quest_previous(quest_id, -9003);
    let pool = QuestPoolStoreLikeCpp::from_rows_like_cpp(&store, [], []);
    session.set_quest_store(Arc::new(store));
    session.set_quest_pool_store(Arc::new(pool));
    add_active_quest(&mut session, quest_id);
    let (_, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, quest_id).await;

    assert_eq!(
        sender_rx.try_recv().unwrap(),
        wow_packet::packets::quest::QuestPushResultResponse {
            sender_guid: receiver_guid,
            result: QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            quest_title: String::new(),
        }
        .to_bytes()
    );
    assert_eq!(
        receiver_rx.try_recv().unwrap(),
        wow_packet::packets::quest::QuestPushResultResponse {
            sender_guid,
            result: QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            quest_title: "Quest 7721".into(),
        }
        .to_bytes()
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
}
