//! APP delegation and owner-fence cases for the Domain seasonal operation.

use super::*;

#[test]
fn seasonal_admission_bridge_preserves_the_four_original_inputs() {
    for (quest_sort_id, seeded_event, probe_empty_bucket, expected) in [
        (-376, Some(9), false, false),
        (-376, Some(10), false, true),
        (-376, None, true, true),
        (-101, Some(9), false, true),
    ] {
        let (mut session, _, _) = make_session();
        let quest = seasonal_test_quest_template(12_345, quest_sort_id, 9);
        if let Some(event_id) = seeded_event {
            session.seed_seasonal_quest_status_like_cpp(event_id, 12_345, 100);
        }
        assert_eq!(session.can_take_quest(&quest), expected);
        if probe_empty_bucket {
            session.seed_empty_seasonal_event_bucket_like_cpp(9);
            assert!(session.can_take_quest(&quest));
        }
    }
}

fn canonical_seasonal_session() -> (
    WorldSession,
    SharedCanonicalMapManager,
    wow_map::PlayerHandle,
) {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 12_345);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SeasonalOwner".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical seasonal map");
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);
    session.set_seasonal_quest_changed_like_cpp_for_test(true);
    let handle = session.player_handle_like_cpp.expect("canonical seasonal handle");
    (session, canonical, handle)
}

#[test]
fn seasonal_reset_missing_manager_preserves_the_existing_owner_and_reports_missing() {
    let (mut session, canonical, handle) = canonical_seasonal_session();
    let before = canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, |player| player.gameplay_state().quests.clone())
        .expect("original owner");
    // Arrange an absent resource before the operation; no interleaving hook.
    session.canonical_map_manager = None;

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(session.player_handle_like_cpp, Some(handle));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| player.gameplay_state().quests.clone()),
        Some(before)
    );
}

#[test]
fn seasonal_reset_stale_handle_does_not_clear_or_replace_the_current_owner() {
    let (mut session, canonical, old_handle) = canonical_seasonal_session();
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let player_guid = session.player_guid().expect("seasonal Player GUID");
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1002, 98);
    replacement
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(true);
    assert!(replacement.set_quest_completed_bit_like_cpp(65, true));
    let before = replacement.gameplay_state().quests.clone();
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert_eq!(session.player_handle_like_cpp, Some(old_handle));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                (
                    player.gameplay_state().quests.clone(),
                    player.quest_completed_block_like_cpp(1),
                )
            }),
        Some((before, Some(1)))
    );
}
