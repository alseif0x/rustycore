use super::*;
use std::collections::HashMap;

use wow_entities::{Item, ItemValuesMapSnapshot, ItemValuesSnapshot, PlayerInventoryItem};

fn player_with_item_map() -> Player {
    let mut player = seeded_player();
    let raw_key = ObjectGuid::new(0xAA, 0xBB);
    let embedded_guid = ObjectGuid::create_item(1, 92_101);
    let mut item = Item::new(0);
    item.object_mut().create(embedded_guid);
    item.object_mut().set_entry(72_101);
    item.object_mut().set_scale(1.25);
    item.set_count(11);
    item.set_slot(7);
    item.set_container_guid_and_slot(ObjectGuid::new(0, 0xCAFE), 3);
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .insert(raw_key, item);
    let records = player
        .inventory_runtime_mut_like_cpp()
        .inventory_items_mut();
    for slot in [7, 61, 3, 42] {
        let guid = if slot == 7 {
            embedded_guid
        } else {
            ObjectGuid::create_item(1, 92_101 + i64::from(slot))
        };
        records.insert(
            slot,
            PlayerInventoryItem {
                guid,
                entry_id: 72_100 + u32::from(slot),
                db_guid: guid.counter() as u64,
                inventory_type: None,
            },
        );
    }
    let original = records.remove(&7).expect("seeded record should exist");
    records.insert(7, original);
    player
}

fn old_two_clone_observation(
    core: &SessionCore,
) -> (
    Vec<(ObjectGuid, ItemValuesSnapshot)>,
    HashMap<u8, PlayerInventoryItem>,
) {
    core.with_owned_player_like_cpp(|player| {
        let runtime = player.inventory_runtime_like_cpp().clone();
        let old_map = runtime.item_objects().clone();
        let ordered = old_map
            .iter()
            .map(|(&key, item)| (key, ItemValuesSnapshot::capture(item)))
            .collect();
        (ordered, runtime.inventory_items().clone())
    })
    .expect("current Player owner should be readable")
}

#[test]
fn map_and_record_capture_match_old_owner_observation_and_freeze() {
    for active in [false, true] {
        let (core, manager, handle) = owned_fixture(player_with_item_map(), active);
        let (old_items, old_records) = old_two_clone_observation(&core);
        let access = core.owned_inventory_access_like_cpp();
        let values: ItemValuesMapSnapshot = access
            .inventory_item_values_snapshot_like_cpp()
            .expect("current owner should yield an item map");
        let records = access
            .inventory_items_snapshot_like_cpp()
            .expect("current owner should yield inventory records");

        assert_eq!(
            old_items.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            values.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            "direct values capture must preserve the observed HashMap traversal"
        );
        for (key, old) in &old_items {
            assert_eq!(values.get(key), Some(old));
        }
        assert_eq!(records, old_records);
        assert_eq!(
            old_records.keys().collect::<Vec<_>>(),
            records.keys().collect::<Vec<_>>(),
            "record capture must preserve observed slot-map traversal"
        );

        let raw_key = ObjectGuid::new(0xAA, 0xBB);
        assert_ne!(
            values.get(&raw_key).unwrap().object().guid(),
            raw_key,
            "the map key remains independent from the Item's embedded GUID"
        );
        let frozen_values = values.clone();
        let frozen_records = records.clone();
        manager
            .lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |owner| {
                owner
                    .inventory_runtime_mut_like_cpp()
                    .item_objects_mut()
                    .get_mut(&raw_key)
                    .expect("captured map key remains installed")
                    .set_count(19);
                owner
                    .inventory_runtime_mut_like_cpp()
                    .inventory_items_mut()
                    .get_mut(&7)
                    .expect("captured slot record remains installed")
                    .entry_id = 72_102;
            })
            .expect("current Player generation should still resolve");
        assert_eq!(
            values.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            frozen_values.iter().map(|(key, _)| key).collect::<Vec<_>>()
        );
        for (key, frozen) in frozen_values.iter() {
            assert_eq!(values.get(key), Some(frozen));
        }
        assert_eq!(records, frozen_records);
    }
}

#[test]
fn canonical_empty_missing_and_stale_generation_results_stay_distinct() {
    let empty_player = seeded_player();
    let guid = empty_player.guid();
    let (empty_core, manager, handle) = owned_fixture(empty_player, false);
    let access = empty_core.owned_inventory_access_like_cpp();
    assert!(
        access
            .inventory_item_values_snapshot_like_cpp()
            .expect("canonical empty map is present")
            .is_empty()
    );
    assert!(
        access
            .inventory_items_snapshot_like_cpp()
            .expect("canonical empty record map is present")
            .is_empty()
    );

    let no_handle = session_core(guid, None, Some(Arc::clone(&manager)));
    assert!(
        no_handle
            .owned_inventory_access_like_cpp()
            .inventory_item_values_snapshot_like_cpp()
            .is_none()
    );
    assert!(
        no_handle
            .owned_inventory_access_like_cpp()
            .inventory_items_snapshot_like_cpp()
            .is_none()
    );
    let no_manager = session_core(guid, Some(handle), None);
    assert!(
        no_manager
            .owned_inventory_access_like_cpp()
            .inventory_item_values_snapshot_like_cpp()
            .is_none()
    );
    assert!(
        no_manager
            .owned_inventory_access_like_cpp()
            .inventory_items_snapshot_like_cpp()
            .is_none()
    );

    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .expect("original generation should retire");
    let replacement = seeded_player();
    assert_eq!(replacement.guid(), guid);
    let current_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-GUID replacement should receive a new generation");
    assert_ne!(current_handle, handle);
    let current_core = session_core(guid, Some(current_handle), Some(Arc::clone(&manager)));
    assert!(
        current_core
            .owned_inventory_access_like_cpp()
            .inventory_item_values_snapshot_like_cpp()
            .is_some()
    );
    assert!(
        current_core
            .owned_inventory_access_like_cpp()
            .inventory_items_snapshot_like_cpp()
            .is_some()
    );
    assert!(
        empty_core
            .owned_inventory_access_like_cpp()
            .inventory_item_values_snapshot_like_cpp()
            .is_none()
    );
    assert!(
        empty_core
            .owned_inventory_access_like_cpp()
            .inventory_items_snapshot_like_cpp()
            .is_none()
    );
}
