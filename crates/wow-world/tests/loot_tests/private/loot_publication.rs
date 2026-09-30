//! Publication cut, viewer-dependent dirty flags and item-window transitions.

use super::recovery_support::*;
use std::collections::{HashMap, HashSet};
use wow_constants::UnitDynFlags;
use wow_packet::packets::update::{UnitDataValuesDeltaUpdate, ObjectDataValuesUpdate};
use wow_world::test_fixtures::loot::{open_loot_item_window_for_test, loot_release_values_for_test, loot_item_fanout_at_commit_for_test};
use wow_world::test_fixtures::loot::{make_canonical_creature_for_loot_test as make_canonical_creature_for_session, attach_canonical_creature_for_loot_test as attach_canonical_creature};

#[tokio::test]
async fn item_loot_releases_ae_view_and_tracks_multiple_items_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_718);
    let primary_guid = test_creature_guid(61_719);
    let secondary_guid = test_creature_guid(61_720);
    let first_item = ObjectGuid::create_item(1, 61_721);
    let second_item = ObjectGuid::create_item(1, 61_722);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(active_loot_guid_for_test(&session).is_empty());
    assert!(active_loot_view_owners_for_test(&session).contains(&secondary_guid));

    open_loot_item_window_for_test(&mut session, player_guid, first_item)
        .await;
    open_loot_item_window_for_test(&mut session, player_guid, second_item)
        .await;

    assert!(is_active_loot_guid_for_test(&session, first_item));
    assert_eq!(active_loot_view_owners_for_test(&session).len(), 2);
    assert!(active_loot_view_owners_for_test(&session).contains(&first_item));
    assert!(active_loot_view_owners_for_test(&session).contains(&second_item));
    assert!(!active_loot_view_owners_for_test(&session).contains(&secondary_guid));
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "item loot must release the surviving secondary AE viewer first"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}

#[test]
fn durable_item_fanout_uses_precommit_union_exact_commit_cut_like_cpp() {
    let before = ObjectGuid::create_player(1, 41);
    let during = ObjectGuid::create_player(1, 42);
    let after = ObjectGuid::create_player(1, 43);

    let viewers =
        loot_item_fanout_at_commit_for_test(&[before], &[before, during]);

    assert_eq!(viewers, HashSet::from([before, during]));
    assert!(
        !viewers.contains(&after),
        "a later authority sample must never expand the exact commit fanout"
    );
}

#[test]
fn creature_loot_release_dynamic_flags_are_viewer_dependent_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 61);
    let second_player = ObjectGuid::create_player(1, 62);
    let unrelated_player = ObjectGuid::create_player(1, 63);
    let owner_guid = test_creature_guid(19_120);
    let mut creature = make_canonical_creature_for_session(&session, owner_guid);

    let mut consumed_pool = authoritative_test_loot_like_cpp(0, false);
    consumed_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    consumed_pool.allowed_looters = vec![first_player];
    let mut live_pool = authoritative_test_loot_like_cpp(0, true);
    live_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    live_pool.allowed_looters = vec![second_player];
    live_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        creature
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, consumed_pool), (second_player, live_pool),]),
            )
            .installed()
    );
    let authority = creature.loot_authority_like_cpp().clone();
    attach_canonical_creature(&mut session, creature);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.set_player_guid(Some(first_player));

    let update = UnitDataValuesDeltaUpdate {
        object_data: Some(ObjectDataValuesUpdate {
            changed_object_type_mask: 1,
            object_data_mask: 1 << 2,
            entry_id: 0,
            dynamic_flags: UnitDynFlags::Lootable as u32,
            scale: 1.0,
        }),
        ..UnitDataValuesDeltaUpdate::default()
    };
    let dynamic_flags_for = |viewer_guid| {
        loot_release_values_for_test(&session, 
                owner_guid,
                viewer_guid,
                false,
                Some(&authority),
                update.clone(),
            )
            .object_data
            .unwrap()
            .dynamic_flags
    };

    assert_eq!(dynamic_flags_for(first_player), 0);
    assert_eq!(
        dynamic_flags_for(second_player),
        UnitDynFlags::Lootable as u32,
        "one exhausted personal pool must not hide another player's live loot"
    );
    assert_eq!(dynamic_flags_for(unrelated_player), 0);
}
