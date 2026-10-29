use super::*;
use wow_constants::ItemUpdateState;
use wow_entities::{
    ITEM_DATA_DURABILITY_BIT, ITEM_DATA_PARENT_BIT, ITEM_DATA_STACK_COUNT_BIT, Item,
    ItemObjectUpdateLikeCpp, TYPEID_ITEM,
};

#[test]
fn map_captures_only_current_owned_durability_and_consumes_once() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 4_450_192);
    let item_guid = ObjectGuid::create_item(1, 4_450_193);
    let mut player = Player::new(Some(7), false);
    player.unit_mut().world_mut().object_mut().create(owner);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.set_owner_guid(owner);
    item.set_count(7);
    item.set_max_durability(60);
    item.set_durability(40);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);
    player.clear_data_changes();
    for durability in [34, 19] {
        player.with_inventory_runtime_mut_like_cpp(|runtime| {
            assert!(runtime.apply_item_object_updates_like_cpp(
                item_guid,
                &[
                    ItemObjectUpdateLikeCpp::SetDurability(durability),
                    ItemObjectUpdateLikeCpp::SetState(ItemUpdateState::Changed),
                    ItemObjectUpdateLikeCpp::QueueDurabilityUpdate,
                ]
            ));
        });
    }
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let first = map.send_object_updates_like_cpp();
    assert_eq!(first.player_values_updates.len(), 1);
    let captured = &first.player_values_updates[0];
    assert!(
        !captured.values_update.has_data(),
        "item-only publication needs no actor delta"
    );
    assert_eq!(captured.owned_item_durability_updates.len(), 1);
    let (guid, update) = &captured.owned_item_durability_updates[0];
    assert_eq!(*guid, item_guid);
    assert_eq!(update.changed_object_type_mask, 1 << TYPEID_ITEM);
    assert!(update.object_data.is_none());
    let data = update.item_data.as_ref().unwrap();
    for bit in 0..wow_entities::ITEM_DATA_BITS {
        assert_eq!(
            data.mask.is_set(bit),
            [ITEM_DATA_PARENT_BIT, ITEM_DATA_DURABILITY_BIT].contains(&bit)
        );
    }
    assert_eq!(data.values.durability, 19);
    assert_eq!(
        data.values.stack_count, 0,
        "the snapshot carries durability only"
    );
    let live = map.get_typed_player_mut(owner).unwrap();
    let item = &live.inventory_runtime_like_cpp().item_objects()[&item_guid];
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_STACK_COUNT_BIT),
        "durability publication must not consume unrelated Item dirty leaves"
    );
    assert!(
        map.send_object_updates_like_cpp()
            .player_values_updates
            .is_empty()
    );

    let live = map.get_typed_player_mut(owner).unwrap();
    live.with_inventory_runtime_mut_like_cpp(|runtime| {
        runtime.apply_item_object_updates_like_cpp(
            item_guid,
            &[ItemObjectUpdateLikeCpp::QueueDurabilityUpdate],
        );
        runtime
            .item_objects_mut()
            .get_mut(&item_guid)
            .unwrap()
            .set_owner_guid(ObjectGuid::create_player(1, 999));
    });
    assert!(
        map.send_object_updates_like_cpp()
            .player_values_updates
            .is_empty(),
        "an item moved to another owner cannot be redirected at capture"
    );
}
