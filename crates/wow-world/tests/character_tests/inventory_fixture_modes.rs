//! Feature-build boundary cases for the explicit ownerless inventory mode.
use super::*;
use super::inventory_runtime::make_session;

#[test]
fn ownerless_inventory_snapshots_are_disabled_by_default() {
    let (mut session, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    assert!(inventory_player_snapshot_for_test(&session).is_none());
    assert_eq!(inventory_capacities_for_test(&session), (None, None));
    assert_eq!(set_inventory_capacities_for_test(&mut session, 24, 2), (false, false));
    assert_eq!(inventory_capacities_for_test(&session), (None, None));
    assert!(!inventory_player_handle_present_for_test(&session));
}

#[test]
fn ownerless_inventory_snapshots_require_player_guid_even_when_enabled() {
    let (mut session, _) = make_session();
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    assert!(session.player_guid().is_none());
    assert!(inventory_player_snapshot_for_test(&session).is_none());
}

#[test]
fn ownerless_inventory_snapshots_reject_an_unresolvable_existing_handle() {
    let (mut session, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    assert!(inventory_player_handle_present_for_test(&session));
    assert!(inventory_player_snapshot_for_test(&session).is_none());
    assert_eq!(inventory_capacities_for_test(&session), (None, None));
    assert_eq!(set_inventory_capacities_for_test(&mut session, 24, 2), (false, false));
    assert!(inventory_player_snapshot_for_test(&session).is_none());
}

#[test]
fn ownerless_inventory_snapshots_never_override_the_canonical_player() {
    let (mut session, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 0, 0);
    mutate_canonical_player_for_test(&session, |player| {
        player.set_inventory_slot_count(24);
        player.set_bank_bag_slot_count(3);
    }).expect("canonical Player fixture");
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    let snapshot = inventory_player_snapshot_for_test(&session).expect("canonical Player snapshot");
    assert!(inventory_player_handle_present_for_test(&session));
    assert_eq!(snapshot.guid(), guid);
    assert_eq!(snapshot.inventory_slot_count(), 24);
    assert_eq!(snapshot.bank_bag_slot_count(), 3);
    assert_eq!(inventory_capacities_for_test(&session), (Some(24), Some(3)));
    assert_eq!(mutate_canonical_player_for_test(&session, |player| (
        player.inventory_slot_count(), player.bank_bag_slot_count(),
    )), Some((24, 3)));
}

#[test]
fn ownerless_inventory_snapshots_reconstruct_existing_slots_only_after_opt_in() {
    let (mut session, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 73);
    session.set_player_guid(Some(guid));
    insert_inventory_item_for_test(&mut session, INVENTORY_SLOT_ITEM_START, InventoryItem {
        guid: item_guid, entry_id: 100, db_guid: 73, inventory_type: None,
    });
    assert!(inventory_player_snapshot_for_test(&session).is_none());
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    assert_eq!(inventory_capacities_for_test(&session), (Some(INVENTORY_DEFAULT_SIZE), Some(0)));
    assert_eq!(set_inventory_capacities_for_test(&mut session, 24, 2), (true, true));
    let snapshot = inventory_player_snapshot_for_test(&session).expect("explicit ownerless snapshot");
    assert_eq!(snapshot.guid(), guid);
    assert_eq!(snapshot.inventory_slot_count(), 24);
    assert_eq!(snapshot.bank_bag_slot_count(), 2);
    assert_eq!(snapshot.top_level_item_guid(INVENTORY_SLOT_ITEM_START), Some(item_guid));
    assert!(!inventory_player_handle_present_for_test(&session));
}
