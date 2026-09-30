//! Shared canonical loot-authority fixtures for packet and lifecycle scenarios.

use std::sync::{Arc, RwLock};
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
    LootResponse,
};

use super::{
    install_limited_test_item_template, loot_type_for_client_like_cpp,
    make_session_with_send_capacity, register_test_creature_like_cpp,
    represented_loot_object_guid_like_cpp, represented_loot_response_items_like_cpp, test_creature,
    test_creature_guid,
};
use crate::handlers::loot::rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp;
use crate::session::WorldSession;

pub(super) fn insert_allowed_coin_loot_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    coins: u32,
) {
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    if session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .is_some()
    {
        install_cached_test_creature_loot_authority_like_cpp(session, owner_guid, player_guid);
    }
}

/// Legacy packet tests construct the result of `Unit::Kill` directly.
/// Install that fixture into the creature before exercising CMSG_LOOT_UNIT
/// so the request remains a pure read/reconciliation path, like C++.
pub(super) fn install_cached_test_creature_loot_authority_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    scope_player: ObjectGuid,
) {
    if let Some(loot) = session.loot_table.get_mut(&owner_guid) {
        // The fixtures describe the already-filtered post-FillLoot item
        // set. Rebuild only its derived counters; adding looters here
        // would erase negative eligibility cases the fixture represents.
        rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
    }
    session
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player)
        .expect("the kill-time loot fixture must install into its creature authority");
}

pub(super) fn two_sessions_with_authoritative_creature_loot_like_cpp(
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
    install_limited_test_item_template(&mut first, 25, 0);
    install_limited_test_item_template(&mut second, 25, 0);
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    first.set_map_manager(Arc::clone(&shared_map));
    second.set_map_manager(shared_map);
    register_test_creature_like_cpp(&mut first, test_creature(owner_guid, false));

    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![first_guid, second_guid];
    for entry in &mut loot.items {
        entry.allowed_looters = vec![first_guid, second_guid];
    }
    first.loot_table.insert(owner_guid, loot);
    first
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, first_guid)
        .unwrap();

    first.set_active_loot_guid(owner_guid);
    let first_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &first.loot_table[&owner_guid],
        first_guid,
    );
    first.represented_on_loot_opened_like_cpp(owner_guid, first_guid, first_response);
    assert!(second.reconcile_represented_loot_cache_like_cpp(owner_guid, second_guid));
    second.set_active_loot_guid(owner_guid);
    let second_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &second.loot_table[&owner_guid],
        second_guid,
    );
    second.represented_on_loot_opened_like_cpp(owner_guid, second_guid, second_response);

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

pub(super) fn authoritative_test_loot_response_like_cpp(
    owner_guid: ObjectGuid,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> LootResponse {
    LootResponse {
        owner: owner_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    }
}

pub(super) fn represented_disenchant_test_outputs_like_cpp(
    winner_guid: ObjectGuid,
    item_id: u32,
) -> Vec<LootEntry> {
    (0..2)
        .map(|loot_list_id| LootEntry {
            loot_list_id,
            item_id,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                ..Default::default()
            },
            allowed_looters: vec![winner_guid],
            roll_winner: winner_guid,
            ffa_looted_by: Vec::new(),
            taken: false,
        })
        .collect()
}
