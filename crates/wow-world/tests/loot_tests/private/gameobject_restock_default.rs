//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[test]
fn gameobject_loot_release_without_canonical_manager_keeps_represented_restock_fallback_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_135);
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_for_loot_test(&mut session, 
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );

    apply_cached_gameobject_loot_release_for_test(&mut session, loot_guid, player_guid, true, true);

    let state = gameobject_loot_release_snapshot_for_test(&session, loot_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(LootState::NotReady));
    assert_eq!(state.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert!(state.chest_restock_until.is_some());
}

