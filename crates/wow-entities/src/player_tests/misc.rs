//! Misc scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_lifecycle_create_initializes_representable_state_as_clean_baseline() {
    let record = lifecycle_create_record();
    let player = Player::create_from_lifecycle(Some(9), true, record.clone(), &StubPowerResolver);

    assert_eq!(player.guid(), record.guid);
    assert_eq!(player.session_id(), Some(9));
    assert_eq!(player.unit().world().name(), "Lifecycle");
    assert_eq!(player.unit().world().map_id(), record.map_id);
    assert_eq!(player.unit().world().position(), record.position);
    assert_eq!(player.unit().world().object().scale(), 1.0);
    assert_eq!(player.unit().data().race, record.race);
    assert_eq!(player.unit().data().class_id, record.class_id);
    assert_eq!(player.unit().data().player_class_id, record.class_id);
    assert_eq!(player.unit().data().sex, Gender::Female as u8);
    assert_eq!(player.unit().data().level, i32::from(record.level));
    assert_eq!(player.unit().data().faction_template, 35);
    assert_eq!(player.unit().data().display_id, 1234);
    assert_eq!(player.unit().data().native_display_id, 1234);
    assert_eq!(player.unit().data().display_power, PowerType::Mana as u8);
    assert_eq!(player.unit().data().max_health, record.max_health);
    assert_eq!(player.unit().data().health, record.health);
    assert_eq!(player.active_data().xp, record.xp);
    assert_eq!(player.active_data().coinage, record.money);
    assert_eq!(
        player.active_data().num_backpack_slots,
        record.inventory_slot_count
    );
    assert_eq!(player.data().num_bank_slots, record.bank_bag_slot_count);
    assert_eq!(player.data().player_flags, record.player_flags);
    assert_eq!(player.data().player_flags_ex, record.player_flags_ex);
    assert_eq!(player.extra_flags(), record.extra_flags);
    assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(player.get_power(PowerType::Mana), 400);
    assert_eq!(player.get_max_power(PowerType::Mana), 900);
    assert_eq!(player.get_power_index(PowerType::Energy), Some(3));
    assert_eq!(player.get_power(PowerType::Energy), 40);
    assert_eq!(player.get_power_index(PowerType::Focus), None);
    assert_eq!(player.get_power(PowerType::Focus), 0);
    assert_eq!(
        player.lifecycle_metadata(),
        PlayerLifecycleMetadata {
            account_id: None,
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: None,
        }
    );
    assert_player_lifecycle_is_clean(&player);
}
#[test]
fn action_buttons_are_sorted_player_owned_state_like_cpp() {
    let mut player = Player::new(None, false);

    assert!(player.set_action_button_like_cpp(7, 12_345, 0x80));
    assert!(player.set_action_button_like_cpp(2, 635, 0));
    assert_eq!(
        player.action_button_like_cpp(7),
        Some(12_345 | (0x80 << 24))
    );
    assert_eq!(
        player
            .gameplay_state()
            .action_buttons
            .iter()
            .map(|button| button.button)
            .collect::<Vec<_>>(),
        vec![2, 7]
    );

    assert!(player.set_action_button_like_cpp(7, 0, 0));
    assert_eq!(player.action_button_like_cpp(7), Some(0));
    assert_eq!(player.action_buttons_snapshot_like_cpp()[2], 635);
}
#[test]
fn native_trait_config_lifecycle_preserves_raw_headers_and_resets_invalid_authority() {
    let mut state = PlayerSpellRuntimeState::default();
    state.known_spells = vec![10];
    state.trait_definition_ids.insert(10, 20);
    state.trait_definition_ids_complete = true;
    state.override_spells.insert(10, BTreeSet::from([30]));
    assert!(state.complete_trait_config_load_like_cpp(vec![(1, -1, -2, -3)], false));
    assert_eq!(
        state.trait_config_rows,
        BTreeMap::from([(1, (-1, -2, -3).into())])
    );
    assert!(state.trait_config_rows_complete && state.trait_entry_rows_complete);
    assert!(!state.trait_entry_rows_empty);
    assert!(state.trait_definition_ids_complete);
    assert!(!state.complete_trait_config_load_like_cpp(vec![(1, 1, 62, 4), (1, 2, 0, 0)], true));
    assert!(state.trait_definition_ids.is_empty() && state.trait_config_rows.is_empty());
    assert!(
        !state.trait_definition_ids_complete
            && !state.trait_config_rows_complete
            && !state.trait_entry_rows_complete
            && !state.trait_entry_rows_empty
    );
    assert_eq!(state.known_spells, vec![10]);
    assert_eq!(
        state.override_spells,
        BTreeMap::from([(10, BTreeSet::from([30]))])
    );
    assert!(state.complete_trait_config_load_like_cpp(vec![], true));
    assert!(state.trait_entry_rows_empty && state.trait_config_rows_complete);
    state.begin_trait_config_load_like_cpp();
    assert!(!state.trait_entry_rows_empty && !state.trait_config_rows_complete);
}
#[test]
fn prepared_acquisition_validates_then_installs_both_families_preserving_source_evidence() {
    let spell = PlayerKnownSpellRecord {
        spell_id: 10,
        state: PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: true,
        dependent: true,
    };
    let skill = PlayerSkillRecord {
        skill_line_id: 333,
        current_value: 0,
        max_value: 0,
        step: 0,
        profession_slot: -1,
        state: PlayerSkillLoadState::Deleted,
    };
    assert!(
        PreparedPlayerSpellAcquisitionLikeCpp::try_new(
            [spell.clone(), spell.clone()],
            [],
            [],
            vec![],
            0,
            BTreeSet::new()
        )
        .is_none()
    );
    assert!(
        PreparedPlayerSpellAcquisitionLikeCpp::try_new(
            [spell.clone()],
            [],
            [],
            vec![(333, skill.clone()), (333, skill.clone())],
            2,
            BTreeSet::new()
        )
        .is_none()
    );
    let prepared = PreparedPlayerSpellAcquisitionLikeCpp::try_new(
        [spell.clone()],
        [(10, 400)],
        [(50, 10), (50, 10)],
        vec![(333, skill.clone())],
        1,
        BTreeSet::from([333]),
    )
    .unwrap();
    let mut player = Player::new(None, false);
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.fallback_rows.insert(10, spell.clone());
    runtime.trait_config_rows.insert(777, (1, 62, 4).into());
    runtime.trait_config_rows_complete = true;
    player.replace_spell_runtime_like_cpp(runtime.clone());
    player.apply_prepared_spell_acquisition_like_cpp(prepared);
    let applied = player.spell_runtime_like_cpp();
    assert_eq!(applied.rows, BTreeMap::from([(10, spell)]));
    assert_eq!(applied.known_spells, vec![10]);
    assert_eq!(applied.dependent_known_spells, BTreeSet::from([10]));
    assert_eq!(applied.favorite_known_spells, BTreeSet::from([10]));
    assert_eq!(applied.trait_definition_ids, BTreeMap::from([(10, 400)]));
    assert_eq!(
        applied.override_spells,
        BTreeMap::from([(50, BTreeSet::from([10]))])
    );
    assert_eq!(applied.fallback_rows, runtime.fallback_rows);
    assert_eq!(applied.trait_config_rows, runtime.trait_config_rows);
    assert!(applied.trait_config_rows_complete && applied.rows_loaded && applied.rows_complete);
    assert_eq!(player.skill_records_like_cpp(), &[skill]);
    assert!(player.skill_records_loaded_like_cpp() && player.skill_records_complete_like_cpp());
    assert_eq!(player.occupied_skill_slots_like_cpp(), Some(1));
    assert_eq!(
        player.non_durable_skill_tombstones_like_cpp(),
        &BTreeSet::from([333])
    );
}
#[test]
fn player_owns_create_form_and_specialization_state_like_cpp() {
    let mut player = Player::new(Some(7), false);

    player.set_create_mode_like_cpp(1);
    player.set_shapeshift_form_id_like_cpp(5);
    player.set_loot_specialization_id_like_cpp(65);
    player.set_primary_specialization(66);

    assert_eq!(player.create_mode_like_cpp(), 1);
    assert_eq!(player.shapeshift_form_id_like_cpp(), 5);
    assert_eq!(player.loot_specialization_id_like_cpp(), 65);
    assert_eq!(player.primary_specialization_id_like_cpp(), 66);
}
#[test]
fn player_owns_interaction_provenance_and_gossip_menu_like_cpp() {
    let mut player = Player::new(Some(7), false);
    let source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1);
    let other = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 101, 1);
    let option = crate::PlayerGossipOptionLikeCpp {
        gossip_option_id: 7,
        menu_id: 11,
        order_index: 2,
        option_npc: 3,
        action_menu_id: 13,
    };

    player.set_trainer_interaction_like_cpp(source, 77);
    player.replace_gossip_options_like_cpp(vec![option.clone()]);
    assert!(
        player
            .interaction_data_like_cpp()
            .trainer_matches(source, 77)
    );
    assert_eq!(player.gossip_options_like_cpp(), &[option]);

    assert!(!player.reset_interaction_if_source_like_cpp(other));
    assert_eq!(player.interaction_data_like_cpp().source_guid, source);
    player.set_interaction_source_like_cpp(other);
    assert_eq!(player.interaction_data_like_cpp().trainer_id, 0);
    assert_eq!(player.interaction_data_like_cpp().player_choice_id, 0);

    player.clear_gossip_options_like_cpp();
    player.reset_interaction_data_like_cpp();
    assert!(player.gossip_options_like_cpp().is_empty());
    assert_eq!(
        player.interaction_data_like_cpp(),
        &crate::PlayerInteractionDataLikeCpp::default()
    );
}
#[test]
fn player_gameplay_default_state_is_empty_and_attached_to_new_player() {
    let player = Player::new(None, false);

    assert!(player.gameplay_state().is_empty());
    assert!(player.gameplay_state().quests.statuses.is_empty());
    assert!(player.gameplay_state().rest.logout_time.is_none());
}
#[test]
fn player_gameplay_rest_and_taxi_destination_round_trip() {
    let mut player = Player::new(None, false);
    let state = player_gameplay_sample_state();
    let expected_taxi = state.taxi.clone();
    let expected_rest = state.rest.clone();

    player.apply_gameplay_state_from_load(PlayerGameplayLoadRecord { state });

    assert_eq!(player.gameplay_state().taxi, expected_taxi);
    let taxi = &player.gameplay_state().taxi;
    assert_eq!(taxi.taxi_destination_like_cpp(), Some(2));
    assert_eq!(taxi.destinations_like_cpp(), [1, 2, 3]);
    assert!(taxi.is_taximask_node_known_like_cpp(1));
    assert_eq!(taxi.known_node_mask_text_like_cpp(), Some("3 128"));
    assert_eq!(player.gameplay_state().rest, expected_rest);
    assert_eq!(player.gameplay_state().rest.rest_bonus, 1.5);
    assert!(player.gameplay_state().rest.logout_was_resting);
}
#[test]
fn player_constructor_matches_cpp_base_state() {
    let player = Player::new(Some(42), false);

    assert_eq!(player.unit().world().object().type_id(), TypeId::Player);
    assert_eq!(
        player.unit().world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER
    );
    assert_eq!(player.session_id(), Some(42));
    assert_eq!(player.hit_chances(), (7.5, 7.5, 15.0));
    assert_eq!(player.ingame_time(), 0);
    assert_eq!(player.shared_quest_id(), 0);
    assert_eq!(player.extra_flags(), 0);
    assert!(!player.is_game_master_like_cpp());
    assert_eq!(player.team(), TEAM_OTHER);
    assert!(player.is_active());
    assert!(player.controlled_by_player());
    assert!(player.accept_whispers());
    assert_eq!(
        player.data().visible_items,
        [VisibleItemValues::default(); EQUIPMENT_SLOT_END as usize]
    );
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
#[test]
fn game_master_flag_matches_cpp_extra_flag() {
    let mut player = Player::new(Some(42), false);

    player.set_game_master_like_cpp(true);
    assert!(player.is_game_master_like_cpp());
    assert_eq!(
        player.extra_flags() & PLAYER_EXTRA_GM_ON,
        PLAYER_EXTRA_GM_ON
    );

    player.set_game_master_like_cpp(false);
    assert!(!player.is_game_master_like_cpp());
    assert_eq!(player.extra_flags() & PLAYER_EXTRA_GM_ON, 0);
}
#[test]
fn can_filter_whispers_permission_keeps_constructor_accept_flag_false() {
    let player = Player::new(None, true);
    assert!(!player.accept_whispers());
}
#[test]
fn player_position_classifiers_match_cpp_static_helpers() {
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT));
    assert!(!is_inventory_pos(NULL_BAG, NULL_SLOT));
    assert!(is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START
    ));
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_START, 0));
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START));
    assert!(is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    ));
    assert!(!is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    ));
    assert!(is_inventory_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_START,
        5
    )));

    assert!(is_equipment_pos(INVENTORY_SLOT_BAG_0, 0));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        PROFESSION_SLOT_START
    ));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    ));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        REAGENT_BAG_SLOT_START
    ));
    assert!(!is_equipment_pos(INVENTORY_SLOT_BAG_START, 0));
    assert!(is_equipment_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    )));

    assert!(is_bank_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START));
    assert!(is_bank_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_BAG_START));
    assert!(is_bank_pos(BANK_SLOT_BAG_START, 0));
    assert!(!is_bank_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START
    ));
    assert!(is_bank_packed_pos(make_item_pos(BANK_SLOT_BAG_START, 2)));

    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    )));
    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_BAG_START
    )));
    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        REAGENT_BAG_SLOT_START
    )));
    assert!(!is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_START, 0)));

    assert!(is_child_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    ));
    assert!(is_child_equipment_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    )));
    assert!(!is_child_equipment_pos(
        INVENTORY_SLOT_BAG_START,
        CHILD_EQUIPMENT_SLOT_START
    ));
}
#[test]
fn player_identity_setters_mark_cpp_unit_and_playerdata_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_race_class_gender(1, 2, Gender::Female);
    player.set_selection(ObjectGuid::new(7, 11));

    assert_eq!(player.unit().data().race, 1);
    assert_eq!(player.unit().data().class_id, 2);
    assert_eq!(player.unit().data().player_class_id, 2);
    assert_eq!(player.unit().data().sex, Gender::Female as u8);
    assert_eq!(player.data().native_sex, Gender::Female as u8);
    assert_eq!(player.unit().data().target, ObjectGuid::new(7, 11));
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_NATIVE_SEX_BIT)
    );
}
#[test]
fn set_inebriation_matches_cpp_clamp_and_marks_playerdata_bit() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_inebriation_like_cpp(55);

    assert_eq!(player.inebriation_like_cpp(), 55);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_INEBRIATION_BIT)
    );

    player.clear_data_changes();
    player.set_inebriation_like_cpp(150);

    assert_eq!(player.inebriation_like_cpp(), 100);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_INEBRIATION_BIT)
    );
}
#[test]
fn values_update_splits_player_and_active_player_for_receiver() {
    let mut player = Player::new(None, false);

    player.set_player_flag(0x20);
    player.set_money(50);

    let other_view = player.values_update(false);
    assert!(other_view.has_data());
    assert_eq!(other_view.changed_object_type_mask, 1 << TYPEID_PLAYER);
    assert!(other_view.player_data.is_some());
    assert!(other_view.active_player_data.is_none());

    let self_view = player.values_update(true);
    assert_eq!(
        self_view.changed_object_type_mask,
        (1 << TYPEID_PLAYER) | (1 << TYPEID_ACTIVE_PLAYER)
    );
    assert!(self_view.active_player_data.is_some());
}
#[test]
fn add_heirloom_marks_active_player_heirlooms_and_flags_dynamic_fields_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    assert_eq!(player.add_heirloom_like_cpp(44_000, 0x03), 0);
    assert_eq!(player.heirlooms_like_cpp(), &[44_000]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x03]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![1]));
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![1])
    );

    assert_eq!(player.add_heirloom_like_cpp(44_001, 0x04), 1);
    assert_eq!(player.heirlooms_like_cpp(), &[44_000, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x03, 0x04]);
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![3]));
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![3])
    );
}
#[test]
fn set_heirloom_flags_marks_only_heirloom_flags_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.add_heirloom_like_cpp(44_000, 0x01);
    player.add_heirloom_like_cpp(44_001, 0x02);
    player.clear_active_player_data_changes();
    player.active_data.heirlooms_update_mask = None;
    player.active_data.heirloom_flags_update_mask = None;

    assert!(player.set_heirloom_flags_like_cpp(1, 0x06));
    assert!(!player.set_heirloom_flags_like_cpp(2, 0x08));

    assert_eq!(player.heirlooms_like_cpp(), &[44_000, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x01, 0x06]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        !player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, None);
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![0b10])
    );
}
#[test]
fn set_heirloom_marks_only_heirlooms_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.add_heirloom_like_cpp(44_000, 0x01);
    player.add_heirloom_like_cpp(44_001, 0x02);
    player.clear_active_player_data_changes();
    player.active_data.heirlooms_update_mask = None;
    player.active_data.heirloom_flags_update_mask = None;

    assert!(player.set_heirloom_like_cpp(0, 44_002));
    assert!(!player.set_heirloom_like_cpp(2, 44_003));

    assert_eq!(player.heirlooms_like_cpp(), &[44_002, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x01, 0x02]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        !player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![1]));
    assert_eq!(player.active_data().heirloom_flags_update_mask, None);
}
#[test]
fn add_toy_marks_active_player_toys_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    assert_eq!(player.add_toy_like_cpp(30_000), 0);
    assert_eq!(player.toys_like_cpp(), &[30_000]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_TOYS_BIT)
    );
    assert_eq!(player.active_data().toys_update_mask, Some(vec![1]));

    assert_eq!(player.add_toy_like_cpp(30_001), 1);
    assert_eq!(player.toys_like_cpp(), &[30_000, 30_001]);
    assert_eq!(player.active_data().toys_update_mask, Some(vec![3]));
}
#[test]
fn player_buyback_slots_follow_cpp_current_slot_and_masks() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    let first = ObjectGuid::create_item(1, 1000);
    let second = ObjectGuid::create_item(1, 1001);

    let first_slot = player.add_item_to_buyback_slot(first, 123, 456);
    assert_eq!(first_slot, BUYBACK_SLOT_START);
    assert_eq!(
        player.inventory().current_buyback_slot,
        BUYBACK_SLOT_START + 1
    );
    assert_eq!(player.get_item_from_buyback_slot(first_slot), Some(first));
    assert_eq!(player.active_data().buyback_price[0], 123);
    assert_eq!(player.active_data().buyback_timestamp[0], 456);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT)
    );

    let second_slot = player.add_item_to_buyback_slot(second, 200, 500);
    assert_eq!(second_slot, BUYBACK_SLOT_START + 1);
    assert_eq!(
        player.remove_item_from_buyback_slot(first_slot),
        Some(first)
    );
    assert_eq!(player.get_item_from_buyback_slot(first_slot), None);
    assert_eq!(
        player.active_data().inv_slots[first_slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);
}
#[test]
fn enchantment_time_update_removes_missing_or_zero_enchantments_like_cpp() {
    let mut player = Player::new(None, false);
    let mut missing = item_with_guid_entry(1240, 7400);
    let mut zero = item_with_guid_entry(1241, 7401);
    player.add_enchantment_duration(&mut missing, EnchantmentSlot::EnhancementSocket, 2_000);
    player.add_enchantment_duration(&mut zero, EnchantmentSlot::EnhancementSocket2, 3_000);

    assert_eq!(
        player.update_enchant_time(
            &[PlayerEnchantDurationItemRef::new(
                zero.object().guid(),
                EnchantmentSlot::EnhancementSocket2,
                0,
            )],
            100,
        ),
        vec![
            UpdateEnchantTimeAction::RemoveMissingEnchantment {
                item_guid: missing.object().guid(),
                slot: EnchantmentSlot::EnhancementSocket,
            },
            UpdateEnchantTimeAction::RemoveMissingEnchantment {
                item_guid: zero.object().guid(),
                slot: EnchantmentSlot::EnhancementSocket2,
            },
        ]
    );
    assert!(player.enchant_durations().is_empty());
}
#[test]
fn apply_enchantment_effect_actions_match_cpp_stat_resistance_and_broken_skip() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12493, 7467);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[
                ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Resistance, 17, 2),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    31,
                    ItemModType::Strength as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(2),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 17,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 31,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );

    item.set_max_durability(100);
    item.set_durability(0);
    assert!(
        player
            .apply_enchantment_effect_actions(
                &item,
                None,
                EnchantmentSlot::EnhancementTemporary,
                true,
                &[ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    31,
                    ItemModType::Strength as u32,
                )],
            )
            .is_empty()
    );
}
#[test]
fn apply_enchantment_effect_actions_resolve_cpp_random_suffix_amounts() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12495, 7469);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    item.set_random_properties_id(-77);
    item.set_property_seed(12_345);

    let random_suffix = ApplyEnchantmentRandomSuffixRef::new(
        77,
        [901, 900, 902, 0, 0],
        [1_000, 2_000, 3_000, 0, 0],
    );

    assert_eq!(
        player.apply_enchantment_effect_actions_for_enchantment(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            900,
            Some(random_suffix),
            true,
            &[
                ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Resistance, 0, 2),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    0,
                    ItemModType::Strength as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(2),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 2_469,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 2_469,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );

    item.set_random_properties_id(-78);
    assert_eq!(
        player.apply_enchantment_effect_actions_for_enchantment(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            900,
            Some(random_suffix),
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Stat,
                0,
                ItemModType::Strength as u32,
            )],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 0,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );
}
