//! Entity update bridge regression scenarios, part 1 of 2.
//!
//! Moved out of the entity_update_bridge.rs root under #662; every test is unchanged.

use super::*;

#[test]
fn bridges_active_player_money_update_from_entity_mask() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();
    player.set_money(123_456);
    player.set_watched_faction_index_like_cpp(42);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_ACTIVE_PLAYER
    );
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_COINAGE_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT
    ));
    assert_eq!(active.coinage, 123_456);
    assert_eq!(active.watched_faction_index, 42);
}

#[test]
fn bridges_active_player_explored_zones_update_from_entity_mask_like_cpp() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();

    assert!(player.add_explored_zones_like_cpp(9, u64::MAX));

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_ACTIVE_PLAYER
    );
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT + 9
    ));
    assert_eq!(active.explored_zones[9], u64::MAX);
}

#[test]
fn bridges_scaling_player_level_delta_with_cpp_section_mask() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();
    player.set_scaling_player_level_delta_like_cpp(-1);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_ACTIVE_PLAYER
    );
    let active = packet_update.active_player_data.unwrap();
    assert_eq!(active.scaling_player_level_delta, -1);
    assert_eq!(active.active_player_data_mask[0], 1);
    assert_eq!(active.active_player_data_mask[1], 0);
    assert_eq!(
        active.active_player_data_mask[2],
        (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT - 64))
            | (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT - 64))
    );
    assert!(
        active.active_player_data_mask[3..]
            .iter()
            .all(|block| *block == 0)
    );
}

#[test]
fn bridges_isolated_rest_info_delta_without_player_flags_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();
    player.prepare_rest_info_values_update_like_cpp(0, 123, 1, 0x07);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_ACTIVE_PLAYER
    );
    assert!(packet_update.object_data.is_none());
    assert!(packet_update.unit_data.is_none());
    assert!(
        packet_update
            .player_data_mask
            .iter()
            .all(|block| *block == 0)
    );
    let active = packet_update.active_player_data.unwrap();
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT
    ));
    assert_eq!(active.rest_info[0].rest_info_mask, 0x07);
    assert_eq!(active.rest_info[0].threshold, 123);
    assert_eq!(active.rest_info[0].state_id, 1);
}

#[test]
fn rest_bonus_combined_set_marks_both_nested_fields_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_xp_rest_info_like_cpp(123, 0);
    let threshold_only = player_values_update_to_packet(&player.values_update(true)).unwrap();
    assert_eq!(
        threshold_only.active_player_data.unwrap().rest_info[0].rest_info_mask,
        0x07
    );

    player.clear_data_changes();
    player.set_xp_rest_info_like_cpp(123, 1);
    let state_only = player_values_update_to_packet(&player.values_update(true)).unwrap();
    assert_eq!(
        state_only.active_player_data.unwrap().rest_info[0].rest_info_mask,
        0x07
    );
}

#[test]
fn bridges_player_honor_fields_with_cpp_nested_masks() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    player.set_honor_level_like_cpp(3);
    player.set_honor_like_cpp(1_234);
    player.set_honor_next_level_like_cpp(8_800);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_HONOR_LEVEL_BIT
    ));
    assert_eq!(packet_update.honor_level, 3);

    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_HONOR_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT
    ));
    assert_eq!(active.honor, 1_234);
    assert_eq!(active.honor_next_level, 8_800);
}

#[test]
fn bridges_player_inebriation_field_like_cpp() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    player.set_inebriation_like_cpp(67);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        wow_entities::PLAYER_DATA_INEBRIATION_BIT
    ));
    assert_eq!(packet_update.inebriation, 67);
}

#[test]
fn bridges_player_title_update_from_entity_mask_like_cpp() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    player.set_chosen_title_like_cpp(42);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PLAYER_TITLE_BIT
    ));
    assert_eq!(packet_update.player_title, 42);
    assert!(packet_update.active_player_data.is_none());
}

#[test]
fn bridges_active_player_heirloom_dynamic_fields_like_cpp() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    player.add_heirloom_like_cpp(44_000, 0x03);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT
    ));
    assert_eq!(active.heirlooms, vec![44_000]);
    assert_eq!(active.heirlooms_update_mask, Some(vec![1]));
    assert_eq!(active.heirloom_flags, vec![0x03]);
    assert_eq!(active.heirloom_flags_update_mask, Some(vec![1]));
}

#[test]
fn bridges_active_player_toys_dynamic_field_like_cpp() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    player.add_toy_like_cpp(30_000);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_TOYS_BIT
    ));
    assert_eq!(active.toys, vec![30_000]);
    assert_eq!(active.toys_update_mask, Some(vec![1]));
}

#[test]
fn bridges_active_player_transmog_dynamic_field_like_cpp() {
    let mut player = Player::new(Some(7), true);
    player.clear_data_changes();

    let slot = player.add_transmog_block_like_cpp(0);
    assert!(player.add_transmog_flag_like_cpp(slot, 1 << 5));
    player.add_conditional_transmog_like_cpp(65);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();

    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_TRANSMOG_BIT
    ));
    assert_eq!(active.transmog, vec![1 << 5]);
    assert_eq!(active.transmog_update_mask, Some(vec![1]));
    assert!(mask_has(
        &active.active_player_data_mask,
        wow_entities::ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT
    ));
    assert_eq!(active.conditional_transmog, vec![65]);
    assert_eq!(active.conditional_transmog_update_mask, Some(vec![1]));
}

#[test]
fn bridges_quest_completed_bitmap_from_active_player_data_like_cpp() {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_uniq(0x65));
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.as_ref().unwrap();

    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT
    ));
    assert!(mask_has(
        &active.active_player_data_mask,
        ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT + 1
    ));
    assert_eq!(active.quest_completed[1], 1);

    let update_object = player_values_update_to_update_object(player.guid(), 571, &update).unwrap();
    assert!(!update_object.to_bytes().is_empty());
}

#[test]
fn bridges_player_and_active_player_values_without_unit_bits() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();
    player.set_player_flag(0x20);
    player.set_visible_item_slot(
        0,
        Some(VisibleItemValues {
            item_id: 25,
            item_appearance_mod_id: 3,
            item_visual: 4,
        }),
    );
    player.set_money(42);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        (1 << TYPEID_PLAYER) | (1 << TYPEID_ACTIVE_PLAYER)
    );
    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_FLAGS_BIT
    ));
    assert_eq!(packet_update.player_flags, 0x20);
    assert_eq!(packet_update.visible_items[0].visible_item_mask, 0x0F);
    assert_eq!(packet_update.visible_items[0].item_id, 25);
    assert_eq!(
        packet_update.active_player_data.as_ref().unwrap().coinage,
        42
    );
}

#[test]
fn bridges_unit_emote_state_from_player_values_update_like_cpp() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();

    player.unit_mut().set_emote_state_like_cpp(10);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();

    assert!(mask_has(&unit.unit_data_mask, UNIT_DATA_MODS_PARENT_BIT));
    assert!(mask_has(&unit.unit_data_mask, UNIT_DATA_EMOTE_STATE_BIT));
    assert_eq!(unit.emote_state, 10);
}

#[test]
fn bridges_unit_stand_state_with_generated_cpp_parent_mask() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();

    player
        .unit_mut()
        .set_stand_state_like_cpp(wow_constants::UnitStandStateType::Sit);

    let update = player.values_update(false);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();

    assert!(mask_has(
        &unit.unit_data_mask,
        UNIT_DATA_STAND_STATE_PARENT_BIT
    ));
    assert!(mask_has(&unit.unit_data_mask, UNIT_DATA_STAND_STATE_BIT));
    assert!(!mask_has(&unit.unit_data_mask, UNIT_DATA_PARENT_BIT));
    assert_eq!(
        unit.stand_state,
        wow_constants::UnitStandStateType::Sit as u8
    );
}

#[test]
fn bridges_player_party_type_update_like_cpp() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();

    assert!(player.set_party_type_like_cpp(0, wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP));

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();

    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARTY_TYPE_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        PLAYER_DATA_PARTY_TYPE_FIRST_BIT
    ));
    assert_eq!(
        packet_update.party_type[0],
        wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP
    );
    assert_eq!(
        packet_update.party_type[1],
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );
}

#[test]
fn builds_update_object_from_entity_player_values_update() {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_uniq(0x42));
    player.clear_data_changes();
    player.set_money(1234);

    let update = player.values_update(true);
    let packet = player_values_update_to_update_object(player.guid(), 571, &update).unwrap();
    let bytes = packet.to_bytes();

    assert!(!bytes.is_empty());
    assert!(
        bytes
            .windows(1234u64.to_le_bytes().len())
            .any(|window| window == 1234u64.to_le_bytes())
    );
}

#[test]
fn bridges_player_object_data_like_cpp_values_prefix() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();
    player.unit_mut().world_mut().object_mut().set_entry(42);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let object_data = packet_update.object_data.unwrap();

    assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_OBJECT);
    assert_eq!(object_data.changed_object_type_mask, 1 << TYPEID_OBJECT);
    assert_eq!(object_data.entry_id, 42);
}

#[test]
fn bridges_unit_object_and_unit_values() {
    let mut unit = wow_entities::Unit::new(true);
    unit.world_mut().object_mut().set_entry(99);
    unit.set_max_health(123);
    unit.set_health(123);
    unit.set_stand_state_like_cpp(wow_constants::UnitStandStateType::Sit);
    unit.replace_all_vis_flags_like_cpp(0x12);
    unit.set_anim_tier_like_cpp(2);
    unit.set_sheath_like_cpp(wow_constants::SheathState::Ranged);
    unit.set_pvp_flag_like_cpp(wow_constants::UnitPvpFlags::PVP);
    unit.replace_all_pet_flags_like_cpp(0x03);
    unit.set_shapeshift_form_like_cpp(wow_constants::ShapeShiftForm::CatForm);
    unit.set_npc_flags_like_cpp(0x40);
    unit.set_npc_flags2_like_cpp(0x1);
    unit.set_hover_height_like_cpp(1.25);

    let update = unit.values_update();
    let packet_update = unit_values_update_to_packet(&update).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        (1 << TYPEID_OBJECT) | (1 << TYPEID_UNIT)
    );
    assert_eq!(packet_update.object_data.unwrap().entry_id, 99);
    assert!(mask_has(
        &packet_update.unit_data_mask,
        UNIT_DATA_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        UNIT_DATA_HEALTH_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        UNIT_DATA_STAND_STATE_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_VIS_FLAGS_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_ANIM_TIER_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_SHEATHE_STATE_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_PVP_FLAGS_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_PET_FLAGS_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_SHAPESHIFT_FORM_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_NPC_FLAGS_PARENT_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1
    ));
    assert!(mask_has(
        &packet_update.unit_data_mask,
        wow_entities::UNIT_DATA_HOVER_HEIGHT_BIT
    ));
    assert_eq!(packet_update.health, 123);
    assert_eq!(packet_update.npc_flags, [0x40, 0x1]);
    assert_eq!(packet_update.hover_height, 1.25);
    assert_eq!(
        packet_update.stand_state,
        wow_constants::UnitStandStateType::Sit as u8
    );
    assert_eq!(packet_update.vis_flags, 0x12);
    assert_eq!(packet_update.anim_tier, 2);
    assert_eq!(
        packet_update.sheathe_state,
        wow_constants::SheathState::Ranged as u8
    );
    assert_eq!(
        packet_update.pvp_flags,
        wow_constants::UnitPvpFlags::PVP.bits()
    );
    assert_eq!(packet_update.pet_flags, 0x03);
    assert_eq!(
        packet_update.shapeshift_form,
        wow_constants::ShapeShiftForm::CatForm as u8
    );
}

#[test]
fn bridges_item_object_only_and_item_data() {
    let mut item = Item::new(0);
    item.object_mut().set_entry(6948);

    let object_update = item_values_update_to_packet(&item.values_update()).unwrap();
    assert_eq!(object_update.changed_object_type_mask, 1 << TYPEID_OBJECT);
    assert_eq!(object_update.object_data.unwrap().entry_id, 6948);
    assert_eq!(object_update.item_data_mask, 0);

    item.clear_item_data_changes();
    item.object_mut().clear_update_mask(false);
    item.set_count(5);

    let item_update = item_values_update_to_packet(&item.values_update()).unwrap();
    assert_eq!(item_update.changed_object_type_mask, 1 << TYPEID_ITEM);
    assert!(mask_has_u64(
        item_update.item_data_mask,
        ITEM_DATA_STACK_COUNT_BIT
    ));
    assert_eq!(item_update.stack_count, 5);
}

#[test]
fn bridges_bag_container_values_with_item_base() {
    let mut bag = Bag::new(0);
    bag.item_mut().object_mut().set_entry(4242);
    bag.set_bag_size(16);

    let packet_update = bag_values_update_to_packet(&bag.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        (1 << TYPEID_OBJECT) | (1 << TYPEID_CONTAINER)
    );
    assert_eq!(packet_update.object_data.unwrap().entry_id, 4242);
    assert!(mask_has_u64(
        packet_update.container_data_mask,
        CONTAINER_DATA_NUM_SLOTS_BIT
    ));
    assert_eq!(packet_update.num_slots, 16);
}

#[test]
fn bridges_game_object_values_with_object_prefix() {
    let mut go = GameObject::new();
    go.world_mut().object_mut().set_entry(1001);
    go.set_display_id(22);
    let owner = wow_core::ObjectGuid::create_player(1, 42);
    go.set_created_by(owner);

    let packet_update = game_object_values_update_to_packet(&go.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        (1 << TYPEID_OBJECT) | (1 << TYPEID_GAME_OBJECT)
    );
    assert_eq!(packet_update.object_data.unwrap().entry_id, 1001);
    assert!(mask_has_u64(
        packet_update.game_object_data_mask as u64,
        GAME_OBJECT_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.game_object_data_mask as u64,
        GAME_OBJECT_DATA_DISPLAY_ID_BIT
    ));
    assert!(mask_has_u64(
        packet_update.game_object_data_mask as u64,
        GAME_OBJECT_DATA_CREATED_BY_BIT
    ));
    assert_eq!(packet_update.display_id, 22);
    assert_eq!(packet_update.created_by, owner);
}

#[test]
fn bridges_dynamic_object_values_with_cpp_field_order_data() {
    let mut dyn_object = DynamicObject::new(true);
    dyn_object.set_radius(7.5);

    let packet_update =
        dynamic_object_values_update_to_packet(&dyn_object.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_DYNAMIC_OBJECT
    );
    assert!(mask_has_u64(
        packet_update.dynamic_object_data_mask as u64,
        DYNAMIC_OBJECT_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.dynamic_object_data_mask as u64,
        DYNAMIC_OBJECT_DATA_RADIUS_BIT
    ));
    assert_eq!(packet_update.radius, 7.5);
}

#[test]
fn bridges_corpse_values_and_items_mask() {
    let mut corpse = Corpse::new(CorpseType::Bones);
    corpse.set_display_id(123);

    let packet_update = corpse_values_update_to_packet(&corpse.values_update()).unwrap();

    assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_CORPSE);
    assert!(mask_has_u64(
        packet_update.corpse_data_mask as u64,
        CORPSE_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.corpse_data_mask as u64,
        CORPSE_DATA_DISPLAY_ID_BIT
    ));
    assert_eq!(packet_update.display_id, 123);
}

#[test]
fn corpse_create_and_values_preserve_cpp_customizations() {
    let mut corpse = Corpse::new(CorpseType::ResurrectablePve);
    corpse.set_customizations(vec![
        CorpseCustomizationChoice {
            option_id: 101,
            choice_id: 201,
        },
        CorpseCustomizationChoice {
            option_id: 102,
            choice_id: 202,
        },
    ]);

    let create = corpse_create_data_from_entity_like_cpp(&corpse);
    assert_eq!(
        create.customizations,
        vec![
            ChrCustomizationChoiceValuesUpdate {
                option_id: 101,
                choice_id: 201,
            },
            ChrCustomizationChoiceValuesUpdate {
                option_id: 102,
                choice_id: 202,
            },
        ]
    );

    let packet_update = corpse_values_update_to_packet(&corpse.values_update()).unwrap();
    assert!(mask_has_u64(
        packet_update.corpse_data_mask as u64,
        CORPSE_DATA_CUSTOMIZATIONS_BIT
    ));
    assert_eq!(packet_update.customizations, create.customizations);
    assert!(packet_update.customizations_update_mask.is_none());
}

#[test]
fn bridges_area_trigger_values_with_nested_full_masks() {
    let mut area_trigger = wow_entities::AreaTrigger::new();
    area_trigger.set_duration(4000);
    area_trigger.set_override_scale_constant(2.0);

    let packet_update =
        area_trigger_values_update_to_packet(&area_trigger.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_AREA_TRIGGER
    );
    assert!(mask_has_u64(
        packet_update.area_trigger_data_mask as u64,
        AREA_TRIGGER_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.area_trigger_data_mask as u64,
        AREA_TRIGGER_DATA_DURATION_BIT
    ));
    assert_eq!(packet_update.duration, 4000);
    assert_eq!(packet_update.override_scale_curve.scale_curve_mask, 0x0F);
}

#[test]
fn bridges_scene_object_values() {
    let mut scene = SceneObject::new();
    scene.set_script_package_id(77);

    let packet_update = scene_object_values_update_to_packet(&scene.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_SCENE_OBJECT
    );
    assert!(mask_has_u64(
        packet_update.scene_object_data_mask as u64,
        SCENE_OBJECT_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.scene_object_data_mask as u64,
        SCENE_OBJECT_DATA_SCRIPT_PACKAGE_ID_BIT
    ));
    assert_eq!(packet_update.script_package_id, 77);
}

#[test]
fn bridges_conversation_values_with_full_actor_mask() {
    let mut conversation = wow_entities::Conversation::new();
    conversation.set_last_line_end_time(1234);
    conversation.add_actor_world_object(9, 0, ObjectGuid::new(1, 55));

    let packet_update =
        conversation_values_update_to_packet(&conversation.values_update()).unwrap();

    assert_eq!(
        packet_update.changed_object_type_mask,
        1 << TYPEID_CONVERSATION
    );
    assert!(mask_has_u64(
        packet_update.conversation_data_mask as u64,
        CONVERSATION_DATA_PARENT_BIT
    ));
    assert!(mask_has_u64(
        packet_update.conversation_data_mask as u64,
        CONVERSATION_DATA_LAST_LINE_END_TIME_BIT
    ));
    assert_eq!(packet_update.last_line_end_time, 1234);
    assert_eq!(packet_update.actors.len(), 1);
    assert!(packet_update.actor_update_mask.is_none());
}

#[test]
fn conversation_create_uses_receiver_locale_timings_like_cpp() {
    let mut conversation = wow_entities::Conversation::new();
    conversation.set_lines(vec![
        ConversationLine {
            conversation_line_id: 7,
            start_time: 100,
            ..Default::default()
        },
        ConversationLine {
            conversation_line_id: 8,
            start_time: 300,
            ..Default::default()
        },
    ]);
    conversation.set_last_line_end_time(9_999);
    conversation.set_line_start_time(6, 7, 250);
    conversation.set_last_line_end_time_for_locale(6, 1_500);

    let create = conversation_create_data_from_entity_like_cpp(&conversation, "esES");

    assert_eq!(create.lines[0].start_time, 250);
    assert_eq!(
        create.lines[1].start_time, 300,
        "C++ falls back to the line's base StartTime when no localized slot exists"
    );
    assert_eq!(create.last_line_end_time, 1_500);
}

#[test]
fn bridges_unit_virtual_items_from_player_values_update() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();
    player.unit_mut().set_virtual_item(
        2,
        Some(VisibleItemValues {
            item_id: 25,
            item_appearance_mod_id: 3,
            item_visual: 4,
        }),
    );

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();

    assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_UNIT);
    assert!(mask_has(
        &unit.unit_data_mask,
        UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT
    ));
    assert!(mask_has(
        &unit.unit_data_mask,
        UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 2
    ));
    assert_eq!(unit.virtual_items[2].visible_item_mask, 0x0F);
    assert_eq!(unit.virtual_items[2].item_id, 25);
    assert_eq!(unit.virtual_items[2].appearance_mod_id, 3);
    assert_eq!(unit.virtual_items[2].item_visual, 4);
}
