use super::*;

#[test]
fn send_new_item_plan_maps_entity_fields_to_item_push_result_like_cpp() {
    let plan = send_new_item_plan(SendNewItemDelivery::Direct);
    let packet = crate::session::item_push_result_from_send_new_item_plan(&plan);

    assert_eq!(packet.player_guid, plan.player_guid);
    assert_eq!(packet.item_guid, plan.item_guid);
    assert_eq!(packet.slot, 4);
    assert_eq!(packet.slot_in_bag, 7);
    assert_eq!(packet.quest_log_item_id, 777);
    assert_eq!(packet.quantity, 3);
    assert_eq!(packet.quantity_in_inventory, 9);
    assert_eq!(packet.dungeon_encounter_id, 615);
    assert_eq!(
        packet.display_text,
        ItemPushResultDisplayType::EncounterLoot
    );
    assert!(packet.pushed);
    assert!(!packet.created);
    assert!(!packet.is_bonus_roll);
    assert!(packet.is_encounter_loot);
    assert_eq!(packet.item.item_id, 9001);
    assert_eq!(packet.item.random_properties_seed, 456);
    assert_eq!(packet.item.random_properties_id, -77);
    assert!(packet.item.item_bonus.is_none());
    assert_eq!(
        packet.item.modifications.values,
        vec![ItemMod::new(123, 3), ItemMod::new(25, 5)]
    );
}
