use crate::{PlayerGameplayState, PlayerVoidStorageItemLikeCpp};
use wow_core::ObjectGuid;

#[test]
fn unloaded_all_none_void_storage_capacities_are_empty() {
    for capacity in [0, 1, 160] {
        let state = PlayerGameplayState {
            void_storage_items: (0..capacity).map(|_| None).collect(),
            ..Default::default()
        };
        assert!(state.is_empty(), "unloaded all-None capacity {capacity}");
    }
}

#[test]
fn loaded_or_occupied_void_storage_is_not_empty() {
    let mut state = PlayerGameplayState {
        void_storage_items: (0..160).map(|_| None).collect(),
        void_storage_loaded: true,
        ..Default::default()
    };
    assert!(!state.is_empty());
    state.void_storage_loaded = false;
    state.void_storage_items[159] = Some(PlayerVoidStorageItemLikeCpp {
        item_id: 77,
        item_entry: 88,
        creator_guid: ObjectGuid::create_player(1, 99),
        fixed_scaling_level: 60,
        random_properties_id: -12,
        random_properties_seed: 34,
        context: 2,
    });
    assert!(!state.is_empty());
}

#[test]
fn unrelated_quest_and_rest_state_still_prevent_emptiness() {
    let mut state = PlayerGameplayState::default();
    state
        .quests
        .set_pending_share_like_cpp(Some((ObjectGuid::create_player(1, 10), 20)));
    assert!(!state.is_empty());
    state.quests.set_pending_share_like_cpp(None);
    assert!(state.is_empty());
    state.rest.rest_bonus = 1.5;
    assert!(!state.is_empty());
    state.rest.rest_bonus = 0.0;
    assert!(state.is_empty());
}
