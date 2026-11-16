use super::*;
use wow_entities::{Item, ItemValuesSnapshot};

fn player_with_item(item_guid: ObjectGuid, entry: u32) -> Player {
    let mut player = seeded_player();
    let mut item = Item::new(0);
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(entry);
    item.set_count(5);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);
    player
}

#[test]
fn selected_item_capture_is_owner_bound_frozen_and_generation_checked() {
    let item_guid = ObjectGuid::create_item(1, 92_001);
    const ITEM_ENTRY: u32 = 71_007;

    for active in [false, true] {
        let player = player_with_item(item_guid, ITEM_ENTRY);
        let (core, manager, handle) = owned_fixture(player, active);
        let access = core.owned_inventory_access_like_cpp();
        let snapshot: ItemValuesSnapshot = access
            .inventory_item_object_snapshot_like_cpp(item_guid)
            .expect("selected item should be captured from its current owner");

        assert_eq!(snapshot.object().guid(), item_guid);
        assert_eq!(snapshot.object().entry(), ITEM_ENTRY);
        assert_eq!(snapshot.data().stack_count, 5);
        assert!(snapshot.item_data_changes_mask().is_any_set());
        assert!(snapshot.object().object_data_changes_mask().is_any_set());
        assert!(
            access
                .inventory_item_object_snapshot_like_cpp(ObjectGuid::create_item(1, 92_002))
                .is_none()
        );

        let frozen = snapshot.clone();
        manager
            .lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |owner| {
                let item = owner
                    .inventory_runtime_mut_like_cpp()
                    .item_objects_mut()
                    .get_mut(&item_guid)
                    .expect("selected item remains in the canonical runtime");
                item.object_mut().set_entry(ITEM_ENTRY + 1);
                item.set_count(9);
                item.clear_item_data_changes();
                item.object_mut().clear_update_mask(false);
            })
            .expect("current owner should remain installed");
        assert_eq!(snapshot, frozen);
        assert_eq!(snapshot.object().entry(), ITEM_ENTRY);
        assert_eq!(snapshot.data().stack_count, 5);
        assert!(snapshot.item_data_changes_mask().is_any_set());
        assert!(snapshot.object().object_data_changes_mask().is_any_set());
    }

    let player = player_with_item(item_guid, ITEM_ENTRY);
    let player_guid = player.guid();
    let (core, manager, handle) = owned_fixture(player, false);
    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .expect("captured owner should retire");

    let replacement = player_with_item(item_guid, ITEM_ENTRY + 2);
    assert_eq!(replacement.guid(), player_guid);
    let replacement_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-GUID replacement should install with a new generation");
    assert_ne!(replacement_handle, handle);
    assert!(
        core.owned_inventory_access_like_cpp()
            .inventory_item_object_snapshot_like_cpp(item_guid)
            .is_none()
    );
}
