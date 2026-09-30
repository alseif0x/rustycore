//! Original roll criteria and generation-lifetime setup, backed by real authority.
pub(super) use super::recovery_support::*;
pub(super) use super::money_support::recv_packet_with_opcode;
pub(super) use wow_loot::{LOOT_METHOD_GROUP_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP};
pub(super) use wow_world::test_fixtures::loot::{LootCriterionExpectation, loot_criterion_for_test, loot_criteria_empty_for_test, loot_roll_observation_for_test, set_loot_roll_deadline_for_test, tick_loot_rolls_for_test, loot_opened_cache_generation_for_test};
use std::collections::HashMap;

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
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(&mut session, owner_guid, generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid));
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;
    while send_rx.try_recv().is_ok() {}
    while candidate_rx.try_recv().is_ok() {}

    let state = loot_roll_observation_for_test(&session, loot_object, 0)
        .expect("first loot generation should start the group roll");
    assert_eq!(state.owner_guid(), owner_guid);
    assert_eq!(
        state.authority_generation(),
        loot_opened_cache_generation_for_test(&session, owner_guid)
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
    let authority = loot_recovery_authority_for_test(session, owner_guid)
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
