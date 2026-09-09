//! Skill scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_world_state_update_missing_event_is_explicit_like_cpp() {
    let events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(1));

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(2),
        GameEventWorldStateUpdateOutcomeLikeCpp::MissingEvent { event_id: 2 }
    );
}
#[test]
fn game_event_check_missing_and_zero_occurrence_are_explicit_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 0, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(9, 500),
        GameEventCheckOutcomeLikeCpp::MissingEvent { event_id: 9 }
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 500),
        GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 1 }
    );
}
#[test]
fn canonical_metadata_exposes_game_event_master_metadata_like_cpp() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();
    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_events_like_cpp(events);

    assert_eq!(metadata.game_events_like_cpp().len_like_cpp(), 4);
    assert_eq!(metadata.game_events_like_cpp().iter_like_cpp().count(), 4);
    assert_eq!(
        metadata.game_event_like_cpp(1).map(|event| event.length),
        Some(10)
    );
    assert!(metadata.game_event_like_cpp(4).is_none());
}
#[test]
fn game_event_npc_vendor_cache_missing_bucket_is_explicit_noop_like_cpp() {
    let mut metadata =
        game_event_npc_vendor_metadata_with_records_like_cpp(1, &[(1, 100, 9001, 6000, 2)]);
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    assert!(summary.missing_event_bucket);
    assert_eq!(summary.records_seen, 0);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .len(),
        1
    );
}
