//! Shared group-loot and generation-lifecycle fixtures for loot tests.

use std::collections::HashMap;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_loot::{LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};

use super::{
    broadcast_info, install_cached_test_creature_loot_authority_like_cpp, loot_unit_packet,
    make_session_with_send_capacity, register_test_creature_like_cpp,
    represented_loot_object_guid_like_cpp, test_creature, test_creature_guid,
};
use crate::session::WorldSession;

pub(super) fn install_master_loot_group(
    session: &mut WorldSession,
    master_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
}

pub(super) fn install_group_loot_group(
    session: &mut WorldSession,
    leader_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
}

pub(super) fn generation_guarded_group_loot_like_cpp(
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
        coins: 0,
        unlooted_count: 1,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid, candidate_guid],
        items: vec![LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                blocked: true,
                ..Default::default()
            },
            allowed_looters: vec![player_guid, candidate_guid],
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        }],
        looted_by_player: false,
    }
}

pub(super) async fn open_generation_guarded_group_roll_like_cpp(
    spawn_id: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(spawn_id);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(16);
    let player_registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid),
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    while send_rx.try_recv().is_ok() {}
    while candidate_rx.try_recv().is_ok() {}

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .expect("first loot generation should start the group roll");
    assert_eq!(state.owner_guid, owner_guid);
    assert_eq!(
        state.authority_generation,
        session
            .represented_loot_cache_generations_like_cpp
            .get(&owner_guid)
            .copied()
            .expect("opened loot cache should be generation-tagged")
    );

    (
        session,
        send_rx,
        candidate_rx,
        player_guid,
        candidate_guid,
        owner_guid,
    )
}

pub(super) fn replace_generation_guarded_group_loot_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) -> u64 {
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .expect("test creature should expose its object-owned loot authority");
    let previous_generation = authority.generation_like_cpp();
    let retired_generation = authority.retire_like_cpp();
    let replacement =
        generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid);
    let replacement_generation = authority
        .replace_retired_generation_like_cpp(retired_generation, Some(replacement), HashMap::new())
        .expect("explicit test generation replaces the observed retired lifetime");
    assert!(replacement_generation > previous_generation);
    replacement_generation
}
