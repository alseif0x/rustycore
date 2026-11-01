//! Spell handler totem scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[tokio::test]
async fn totem_destroyed_despawns_matching_totem_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM + 2;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, true);

    session
        .handle_totem_destroyed(totem_destroyed_packet(2, totem_guid))
        .await;

    assert!(!canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        ObjectGuid::EMPTY
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn totem_destroyed_empty_guid_matches_slot_totem_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM + 1;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, true);

    session
        .handle_totem_destroyed(totem_destroyed_packet(1, ObjectGuid::EMPTY))
        .await;

    assert!(!canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        ObjectGuid::EMPTY
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn totem_destroyed_mismatched_guid_preserves_totem_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, true);
    let other_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 901);

    session
        .handle_totem_destroyed(totem_destroyed_packet(0, other_guid))
        .await;

    assert!(canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        totem_guid
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn totem_destroyed_remote_control_preserves_totem_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, true);
    let controlled_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 902);
    session.set_player_moved_unit_guid_like_cpp(controlled_guid);

    session
        .handle_totem_destroyed(totem_destroyed_packet(0, totem_guid))
        .await;

    assert!(canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        totem_guid
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn totem_destroyed_out_of_range_slot_preserves_totem_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, true);

    session
        .handle_totem_destroyed(totem_destroyed_packet(4, totem_guid))
        .await;

    assert!(canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        totem_guid
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn totem_destroyed_non_totem_creature_preserves_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
    let (player_guid, totem_guid) =
        install_canonical_totem_for_session(&mut session, &canonical, slot, false);

    session
        .handle_totem_destroyed(totem_destroyed_packet(0, totem_guid))
        .await;

    assert!(canonical_creature_exists(&canonical, totem_guid));
    assert_eq!(
        canonical_player_summon_slot(&canonical, player_guid, slot),
        totem_guid
    );
    assert!(send_rx.is_empty());
}
