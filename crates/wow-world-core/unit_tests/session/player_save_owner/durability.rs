use super::*;
use wow_constants::ItemUpdateState;
use wow_core::ObjectGuid;

fn item(id: i64, owner: ObjectGuid, state: ItemUpdateState) -> Item {
    let mut item = Item::default();
    item.object_mut().create(ObjectGuid::create_item(1, id));
    item.set_owner_guid(owner);
    item.set_max_durability(60);
    item.set_durability(34);
    item.set_count(7);
    item.force_state(state);
    item
}

#[test]
fn partial_durability_requires_existing_changed_owned_identity_without_full_authority() {
    let owner = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(Some(42), false);
    player.unit_mut().world_mut().object_mut().create(owner);
    let runtime = player.inventory_runtime_mut_like_cpp();
    for (id, state) in [
        (30, ItemUpdateState::Changed),
        (20, ItemUpdateState::New),
        (40, ItemUpdateState::Removed),
        (50, ItemUpdateState::Unchanged),
    ] {
        let mut item = item(id, owner, state);
        if state == ItemUpdateState::New {
            item.set_state(ItemUpdateState::Changed);
            assert_eq!(item.update_state(), ItemUpdateState::New);
        }
        runtime.store_item_object_like_cpp(item);
    }
    runtime.store_item_object_like_cpp(item(
        60,
        ObjectGuid::create_player(1, 99),
        ItemUpdateState::Changed,
    ));
    let mut traded = item(70, owner, ItemUpdateState::Changed);
    traded.set_in_trade(true);
    runtime.store_item_object_like_cpp(traded);
    let buyback = item(80, owner, ItemUpdateState::Changed);
    let buyback_guid = buyback.object().guid();
    runtime.store_item_object_like_cpp(buyback);
    runtime.store_buyback_item_in_slot_like_cpp(
        94,
        wow_entities::PlayerInventoryItem {
            guid: buyback_guid,
            entry_id: 2121,
            db_guid: 80,
            inventory_type: None,
        },
    );
    runtime.item_objects_mut().insert(
        ObjectGuid::create_item(1, 90),
        item(91, owner, ItemUpdateState::Changed),
    );
    let mut empty = item(92, owner, ItemUpdateState::Changed);
    empty.object_mut().create(ObjectGuid::EMPTY);
    runtime.store_item_object_like_cpp(empty);
    runtime.store_item_object_like_cpp(item(0, owner, ItemUpdateState::Changed));
    let mut wrong_type = item(93, owner, ItemUpdateState::Changed);
    wrong_type
        .object_mut()
        .create(ObjectGuid::create_player(1, 93));
    runtime.store_item_object_like_cpp(wrong_type);
    let mut nondurable = item(94, owner, ItemUpdateState::Changed);
    nondurable.set_max_durability(0);
    runtime.store_item_object_like_cpp(nondurable);
    assert!(!runtime.equipment_inventory_authority_complete_like_cpp());
    assert_eq!(
        capture_durability(&player),
        Some(vec![PlayerItemDurabilitySave {
            item_guid: 30,
            durability: 34,
        }])
    );
    let current =
        &player.inventory_runtime_like_cpp().item_objects()[&ObjectGuid::create_item(1, 30)];
    assert_eq!(current.update_state(), ItemUpdateState::Changed);
    assert_eq!(current.count(), 7);
}
