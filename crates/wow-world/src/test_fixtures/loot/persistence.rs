//! Durable recovery setup sharing the original legacy object authority.

use super::*;

pub use crate::handlers::loot::{
    LootCompletion, LootItemFanout, LootPersistenceError, LootPersistenceGuard,
    LootPersistenceWorker, apply_loot_completions_for_test, begin_loot_persistence_for_test,
    disconnect_loot_cleanup_for_test, disconnect_loot_save_for_test,
    install_loot_recovery_authority_for_test, loot_recovery_authority_for_test,
    loot_recovery_cache_for_test, loot_recovery_cache_values_for_test,
    open_loot_recovery_view_for_test, prepare_loot_item_fanout_for_test,
    reconcile_loot_recovery_cache_for_test, spawn_loot_claim_worker_for_test,
    spawn_loot_item_worker_for_test, wait_for_loot_persistence_for_test,
};

pub fn two_sessions_with_recovery_loot_for_test(
    mut loot: CreatureLoot,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut first, first_rx) = make_session_with_send_capacity(32);
    let (mut second, second_rx) = make_session_with_send_capacity(32);
    let first_guid = ObjectGuid::create_player(1, 42);
    let second_guid = ObjectGuid::create_player(1, 43);
    let owner_guid = test_creature_guid(19_500);
    let shared_map = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));

    first.set_player_guid(Some(first_guid));
    second.set_player_guid(Some(second_guid));
    install_basic_item_template_for_loot_test(&mut first, 25, 0);
    install_basic_item_template_for_loot_test(&mut second, 25, 0);
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    first.set_map_manager(Arc::clone(&shared_map));
    second.set_map_manager(shared_map);
    register_test_creature_for_loot(&mut first, test_creature_for_loot(owner_guid, false));

    loot.loot_guid = represented_loot_object_guid_for_test(owner_guid);
    loot.allowed_looters = vec![first_guid, second_guid];
    for entry in &mut loot.items {
        entry.allowed_looters = vec![first_guid, second_guid];
    }
    install_loot_recovery_authority_for_test(&mut first, owner_guid, first_guid, loot);

    first.set_active_loot_guid(owner_guid);
    open_loot_recovery_view_for_test(&mut first, owner_guid, first_guid);
    assert!(reconcile_loot_recovery_cache_for_test(&mut second, owner_guid, second_guid));
    second.set_active_loot_guid(owner_guid);
    open_loot_recovery_view_for_test(&mut second, owner_guid, second_guid);

    (
        first,
        first_rx,
        second,
        second_rx,
        owner_guid,
        first_guid,
        second_guid,
    )
}

