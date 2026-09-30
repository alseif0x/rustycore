//! Shared opaque persistence fixtures for the recovery families.

pub(super) use super::support::*;
pub(super) use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
pub(super) use wow_loot::OwnedLootAuthorityLifecycle;
pub(super) use wow_persistence::PersistenceOutcomeLikeCpp;
pub(super) use wow_world::session::mailbox::KickLikeCppCommand;
pub(super) use wow_world::test_fixtures::loot::represented_loot_entry_for_test as represented_loot_entry;
pub(super) use wow_world::test_fixtures::loot::two_sessions_with_recovery_loot_for_test as two_sessions_with_authoritative_creature_loot_like_cpp;
pub(super) use wow_world::test_fixtures::loot::{
    LootCompletion, LootPersistenceError, apply_loot_completions_for_test,
    begin_loot_persistence_for_test, disconnect_loot_cleanup_for_test,
    disconnect_loot_save_for_test, loot_recovery_authority_for_test, loot_recovery_cache_for_test,
    loot_recovery_cache_values_for_test, prepare_loot_item_fanout_for_test,
    spawn_loot_claim_worker_for_test, spawn_loot_item_worker_for_test,
    wait_for_loot_persistence_for_test,
};

pub(super) fn install_active_item_loot_completion_fixture_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    owner_guid: ObjectGuid,
    coins: u32,
) {
    assert!(owner_guid.is_item());
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(session, owner_guid);
    set_loot_for_test(
        session,
        owner_guid,
        CreatureLoot {
            loot_guid: owner_guid,
            coins,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );
}

pub(super) fn authoritative_test_loot_like_cpp(coins: u32, with_item: bool) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count: u8::from(with_item),
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: with_item
            .then(|| LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            })
            .into_iter()
            .collect(),
        looted_by_player: false,
    }
}
