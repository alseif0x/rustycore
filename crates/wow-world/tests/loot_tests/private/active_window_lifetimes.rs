//! Canonical loot lifetime regressions moved intact from the application owner.
use super::recovery_support::*;
use std::{collections::HashMap, sync::Barrier};
use wow_loot::mark_loot_item_looted_for_player_like_cpp;
use wow_world::test_fixtures::loot::{GameObjectLootWitness, observe_gameobject_loot_for_test, upsert_gameobject_pool_for_test, upsert_observed_gameobject_pool_for_test, release_gameobject_observation_for_test, mutate_loot_gameobject_for_test, release_fishing_hole_for_test, release_loot_owner_for_test, close_retired_loot_views_for_test, refresh_loot_summary_for_test, loot_cache_mut_for_test, share_loot_canonical_map_for_test};
use wow_world::test_fixtures::loot::{
    make_canonical_gameobject_for_loot_test as make_canonical_gameobject_for_session,
    attach_canonical_gameobject_for_loot_test as attach_canonical_gameobject,
    canonical_gameobject_snapshot_for_loot_test as canonical_gameobject_snapshot,
    make_canonical_creature_for_loot_test as make_canonical_creature_for_session,
    attach_canonical_creature_for_loot_test as attach_canonical_creature,
    canonical_creature_snapshot_for_loot_test as canonical_creature_snapshot,
};

#[tokio::test]
async fn stale_release_keeps_replacement_viewer_and_pool_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, true,
        ));
    let authority = loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(13, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    assert!(
        release_loot_owner_for_test(&mut first, owner, first_guid)
            .await
    );

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 13);
    assert!(!snapshot.loot.items[0].taken);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    assert!(!active_loot_view_owners_for_test(&first).contains(&owner));
}

#[test]
fn retired_object_authority_releases_every_session_window_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, true,
        ));
    let authority = loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    authority.retire_like_cpp();
    close_retired_loot_views_for_test(&mut first, first_guid);
    close_retired_loot_views_for_test(&mut second, second_guid);

    for (session, owner_guid) in [(&first, owner), (&second, owner)] {
        assert!(!active_loot_view_owners_for_test(&session).contains(&owner_guid));
        assert!(!has_loot_for_test(&session, owner_guid));
    }
    for rx in [&first_rx, &second_rx] {
        assert_eq!(
            drain_server_opcodes_like_cpp(rx),
            vec![wow_constants::ServerOpcodes::LootRelease as u16]
        );
    }
}

#[tokio::test]
async fn disconnect_after_primary_ae_release_closes_secondary_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_711);
    let primary_guid = test_creature_guid(61_712);
    let secondary_guid = test_creature_guid(61_713);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(active_loot_guid_for_test(&session).is_empty());
    assert!(active_loot_view_owners_for_test(&session).contains(&secondary_guid));

    disconnect_loot_cleanup_for_test(&mut session).await;

    assert!(active_loot_view_owners_for_test(&session).is_empty());
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "logout must remove the secondary AE viewer even after the primary was released"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}
