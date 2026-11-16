use std::collections::HashMap;

use wow_core::ObjectGuid;

use crate::{Item, ItemValuesMapSnapshot, ItemValuesSnapshot};

fn keyed_item(embedded_guid: ObjectGuid, index: usize) -> Item {
    let mut item = Item::new(index as i64);
    item.object_mut().create(embedded_guid);
    item.object_mut().set_entry(20_000 + index as u32);
    item.set_count(index as u32 + 1);
    item.set_durability(index as u32 + 10);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 70_000), 4);
    item.set_slot((index % 17) as u8);
    item
}

fn assert_matches_two_clone_observation(source: &HashMap<ObjectGuid, Item>, round: usize) {
    let old_observation = source.clone().clone();
    let captured = ItemValuesMapSnapshot::capture(source);
    let old_entries = old_observation
        .iter()
        .map(|(&key, item)| (key, ItemValuesSnapshot::capture(item)))
        .collect::<Vec<_>>();
    let captured_entries = captured
        .iter()
        .map(|(&key, item)| (key, item.clone()))
        .collect::<Vec<_>>();
    assert_eq!(old_entries, captured_entries, "clone history {round}");

    for (key, item) in &old_entries {
        assert_eq!(captured.get(key), Some(item), "lookup at history {round}");
    }
    assert_eq!(captured.values().count(), old_entries.len());
}

#[test]
fn capture_matches_old_two_clone_order_across_randomized_map_histories() {
    for round in 0..96usize {
        let count = 1 + round % 97;
        let mut source = HashMap::new();
        source.reserve(count + round % 13);
        let mut inserted = Vec::with_capacity(count);
        for index in 0..count {
            let key = ObjectGuid::create_item(1, 10_000 + (round * 131 + index) as i64);
            let embedded_guid = if index % 7 == 0 {
                ObjectGuid::create_item(1, 90_000 + (round * 97 + index) as i64)
            } else {
                key
            };
            source.insert(key, keyed_item(embedded_guid, index));
            inserted.push(key);
        }

        if round % 2 == 0 {
            let removed = inserted
                .iter()
                .step_by(5)
                .filter_map(|key| source.remove(key).map(|item| (*key, item)))
                .collect::<Vec<_>>();
            for (key, item) in removed.into_iter().rev() {
                source.insert(key, item);
            }
        }
        if round % 3 == 0 {
            source.reserve(round + 31);
        }
        if round % 4 == 0 {
            source.shrink_to_fit();
        }

        assert_matches_two_clone_observation(&source, round);
    }
}

#[test]
fn capture_preserves_original_keys_colliding_slots_and_frozen_raw_facts() {
    let key = ObjectGuid::create_item(1, 1);
    let embedded_guid = ObjectGuid::create_item(1, 2);
    let other_key = ObjectGuid::create_item(1, 3);
    let mut source = HashMap::new();
    let mut first = keyed_item(embedded_guid, 1);
    first.object_mut().set_scale(f32::from_bits(0x7FC0_1234));
    first.object_mut().replace_all_dynamic_flags(0x8123_4567);
    first.set_slot(4);
    let mut second = keyed_item(ObjectGuid::create_item(1, 4), 2);
    second.set_slot(4);
    source.insert(key, first);
    source.insert(other_key, second);

    let captured = ItemValuesMapSnapshot::capture(&source);
    assert!(captured.contains_key(&key));
    assert!(captured.contains_key(&other_key));
    assert!(!captured.contains_key(&embedded_guid));
    let first_snapshot = captured.get(&key).expect("original map key");
    assert_eq!(first_snapshot.object().guid(), embedded_guid);
    assert_eq!(first_snapshot.object().scale().to_bits(), 0x7FC0_1234);
    assert_eq!(first_snapshot.data().durability, 11);
    assert_eq!(
        first_snapshot.item_data_changes_mask(),
        source.get(&key).unwrap().item_data_changes_mask()
    );
    assert!(captured.get(&embedded_guid).is_none());
    assert_eq!(
        captured
            .values()
            .filter(
                |item| item.container_guid() == ObjectGuid::create_item(1, 70_000)
                    && item.container_slot() == 4
            )
            .count(),
        2
    );

    let original_order = captured.iter().map(|(&guid, _)| guid).collect::<Vec<_>>();
    let cloned = captured.clone();
    assert_eq!(
        cloned.iter().map(|(&guid, _)| guid).collect::<Vec<_>>(),
        original_order
    );
    source.get_mut(&key).unwrap().set_count(999);
    source.remove(&other_key);
    assert_eq!(captured.get(&key).unwrap().data().stack_count, 2);
    assert!(captured.get(&other_key).is_some());
    assert_eq!(
        cloned.get(&key).unwrap().object().scale().to_bits(),
        0x7FC0_1234
    );
}

#[test]
fn empty_capture_is_successful_and_clones_with_an_empty_index() {
    let source = HashMap::new();
    let captured = ItemValuesMapSnapshot::capture(&source);
    assert!(captured.is_empty());
    assert_eq!(captured.iter().count(), 0);
    assert_eq!(captured.values().count(), 0);
    assert!(captured.get(&ObjectGuid::create_item(1, 5)).is_none());
    assert!(!captured.contains_key(&ObjectGuid::create_item(1, 5)));

    let cloned = captured.clone();
    assert!(cloned.is_empty());
    assert_eq!(cloned.iter().count(), 0);
}
