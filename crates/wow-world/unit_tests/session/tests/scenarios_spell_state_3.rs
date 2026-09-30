//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn remove_known_spell_auto_unequip_delinks_offhand_when_store_fails_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let offhand_item_id = 30_005_u32;
    let filler_item_id = 30_006_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_005);
    let player_guid = ObjectGuid::create_player(1, 159);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RemoveOffhandFallback".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                offhand_item_id,
                InventoryType::WeaponOffhand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe as u8,
            ),
            (
                filler_item_id,
                InventoryType::NonEquip,
                0,
                ItemClass::Consumable,
                0,
            ),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(false);
        let _ = player.visualize_item(
            EQUIPMENT_SLOT_OFFHAND,
            offhand_guid,
            VisibleItemValues {
                item_id: offhand_item_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            },
        );
    });
    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 40_000 + i64::from(offset));
        equip_represented_test_item_like_cpp(
            &mut session,
            slot,
            guid,
            filler_item_id,
            InventoryType::NonEquip,
        );
    }

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield,
            stored_destination: None,
            needs_mail_fallback: true,
        }],
        "C++ AutoUnequipOffhandIfNeed falls back to MoveItemFromInventory + mail when CanStoreItem fails"
    );
    assert!(
        !session
            .inventory_items_like_cpp()
            .contains_key(&EQUIPMENT_SLOT_OFFHAND),
        "C++ MoveItemFromInventory removes the item from the offhand slot"
    );
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&offhand_guid)
        .expect("mail fallback keeps the standalone item object represented");
    assert_eq!(runtime_item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(runtime_item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(runtime_item.slot(), NULL_SLOT);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            (
                player.active_data().inv_slots[EQUIPMENT_SLOT_OFFHAND as usize],
                player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
            )
        }),
        Some((ObjectGuid::EMPTY, VisibleItemValues::default())),
        "C++ MoveItemFromInventory clears offhand InvSlot/VisibleItem before mail fallback"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject, ServerOpcodes::UpdateObject],
        "C++ MoveItemFromInventory(update=true) sends player and item values updates before the represented mail fallback"
    );
}
#[test]
fn remove_known_spell_auto_unequip_stores_offhand_in_represented_bag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let offhand_item_id = 30_007_u32;
    let filler_item_id = 30_008_u32;
    let bag_item_id = 30_009_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_007);
    let bag_guid = ObjectGuid::create_item(1, 30_009);
    let player_guid = ObjectGuid::create_player(1, 160);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RemoveOffhandBagStore".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                offhand_item_id,
                InventoryType::WeaponOffhand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe as u8,
            ),
            (
                filler_item_id,
                InventoryType::NonEquip,
                0,
                ItemClass::Consumable,
                0,
            ),
            (bag_item_id, InventoryType::Bag, 0, ItemClass::Container, 0),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        bag_guid,
        bag_item_id,
        InventoryType::Bag,
    );
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(false);
        let _ = player.visualize_item(
            EQUIPMENT_SLOT_OFFHAND,
            offhand_guid,
            VisibleItemValues {
                item_id: offhand_item_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            },
        );
        let _ = player.store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid);
        let _ = player.register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4);
    });
    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 41_000 + i64::from(offset));
        equip_represented_test_item_like_cpp(
            &mut session,
            slot,
            guid,
            filler_item_id,
            InventoryType::NonEquip,
        );
    }

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield,
            stored_destination: Some((INVENTORY_SLOT_BAG_START, 0)),
            needs_mail_fallback: false,
        }],
        "C++ StoreItem stores the offhand item inside an equipped bag when backpack slots are full"
    );
    assert!(
        !session
            .inventory_items_like_cpp()
            .contains_key(&EQUIPMENT_SLOT_OFFHAND),
        "C++ RemoveItem removes the direct offhand slot before Bag::StoreItem"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            .map(|item| item.guid),
        Some(offhand_guid),
        "represented Bag::StoreItem lookup resolves the moved offhand item"
    );
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&offhand_guid)
        .expect("stored bag item remains a runtime item object");
    assert_eq!(runtime_item.data().contained_in, bag_guid);
    assert_eq!(runtime_item.container_guid(), bag_guid);
    assert_eq!(runtime_item.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(runtime_item.slot(), 0);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            (
                player.active_data().inv_slots[EQUIPMENT_SLOT_OFFHAND as usize],
                player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
                player
                    .inventory()
                    .bags
                    .get(INVENTORY_SLOT_BAG_START as usize)
                    .and_then(Option::as_ref)
                    .and_then(|bag| bag.item_by_pos(0)),
            )
        }),
        Some((
            ObjectGuid::EMPTY,
            VisibleItemValues::default(),
            Some(offhand_guid)
        )),
        "C++ RemoveItem clears offhand InvSlot/VisibleItem and Bag::StoreItem stores the child item in the equipped bag"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject
        ],
        "C++ RemoveItem(update=true) + Bag::StoreItem(update=true) emit player, item, and bag slot values updates"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_language_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        777,
        wow_data::SKILL_CATEGORY_LANGUAGES_LIKE_CPP,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        50,
        75,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&777),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 777,
            step: 2,
            value: 300,
            max: 75,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ GetSkillRangeType LANGUAGE forces value to 300 but only clamps existing max downward, so max is not raised when it was already below 300"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_level_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        778,
        9,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        80,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&778),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 778,
            step: 2,
            value: 60,
            max: 60,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ LEVEL range uses GetMaxSkillValueForLevel (level * 5) and then clamps current value/max"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_always_max_level_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        779,
        9,
        wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        10,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&779),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 779,
            step: 2,
            value: 60,
            max: 60,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ SKILL_FLAG_ALWAYS_MAX_VALUE sets value to the computed max before SetSkill"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_mono_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        780,
        wow_data::SKILL_CATEGORY_ARMOR_LIKE_CPP,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        10,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&780),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 780,
            step: 2,
            value: 1,
            max: 1,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ MONO range caps the downgraded learned skill to 1"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_rank_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        781,
        9,
        0,
        12,
        wow_data::SkillTiersStoreLikeCpp::from_rows_like_cpp([wow_data::SkillTiersRowLikeCpp {
            id: 12,
            value: [75, 150, 225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }]),
        200,
        225,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&781),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 781,
            step: 2,
            value: 150,
            max: 150,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ RANK range resolves SkillTiers[prevSkill.step - 1] when previous SpellLearnSkill maxvalue is 0"
    );
}
#[test]
fn remove_known_spell_downgrades_learned_skill_without_race_class_info_to_zero_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 12, 0);
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_rank_store_like_cpp(782)));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        test_skill_line_entry_like_cpp(782, 9),
    ])));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty::<wow_data::SkillLineAbilityRecord>(),
            std::iter::empty::<wow_data::SkillRaceClassInfoRecord>(),
        ),
    ));
    session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        782,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 782,
            step: 3,
            value: 80,
            max: 100,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&782),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 782,
            step: 0,
            value: 0,
            max: 0,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Deleted,
        }),
        "C++ leaves new_skill_max_value at 0 when SkillRaceClassInfo is missing, then clamps value/max to 0"
    );
}
#[test]
fn spell_group_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_spell_group_map_bounds_like_cpp(10).is_empty());
    assert!(
        session
            .spell_group_spell_map_bounds_like_cpp(1001)
            .is_empty()
    );
    assert!(!session.is_spell_member_of_spell_group_like_cpp(10, 1001));
    assert!(
        session
            .set_of_spells_in_spell_group_like_cpp(1001)
            .is_empty()
    );
}
#[test]
fn spell_group_queries_expand_nested_and_normalize_ranks_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_group_store(Arc::new(test_spell_group_store_like_cpp()));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 25,
                supercedes_spell_id: 20,
            }],
            |_| true,
        ),
    ));

    assert_eq!(
        session.spell_group_spell_map_bounds_like_cpp(1001),
        &[10, -1002]
    );
    assert_eq!(
        session.set_of_spells_in_spell_group_like_cpp(1001),
        BTreeSet::from([10, 20])
    );
    assert_eq!(
        session.spell_spell_group_map_bounds_like_cpp(25),
        &[1001, 1002]
    );
    assert!(session.is_spell_member_of_spell_group_like_cpp(25, 1002));
    assert!(!session.is_spell_member_of_spell_group_like_cpp(10, 1002));
}
#[test]
fn spell_group_stack_rule_queries_default_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(
        session.spell_group_stack_rule_like_cpp(1001),
        wow_data::SpellGroupStackRuleLikeCpp::Default
    );
    assert!(
        session
            .same_effect_stack_rule_aura_types_like_cpp(1002)
            .is_none()
    );
    assert_eq!(
        session.check_spell_group_stack_rules_like_cpp(10, 20),
        wow_data::SpellGroupStackRuleLikeCpp::Default
    );
}
#[test]
fn spell_group_stack_rule_queries_match_store_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_groups = Arc::new(test_spell_group_store_like_cpp());
    let stack_rules = Arc::new(test_spell_group_stack_rule_store_like_cpp(&spell_groups));
    session.set_spell_group_store(Arc::clone(&spell_groups));
    session.set_spell_group_stack_rule_store(stack_rules);

    assert_eq!(
        session.spell_group_stack_rule_like_cpp(1001),
        wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest
    );
    assert_eq!(
        session.same_effect_stack_rule_aura_types_like_cpp(1002),
        Some(&BTreeSet::from([31]))
    );
    assert_eq!(
        session.check_spell_group_stack_rules_like_cpp(10, 20),
        wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest
    );
}
#[test]
fn spell_linked_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(
        session
            .spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Cast, 10)
            .is_empty()
    );
}
#[test]
fn spell_linked_queries_preserve_signed_effect_order_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_linked_store(Arc::new(test_spell_linked_store_like_cpp()));

    assert_eq!(
        session.spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Cast, 10),
        &[20, -30]
    );
    assert_eq!(
        session.spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Remove, 40),
        &[50]
    );
    assert!(
        session
            .spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Hit, 10)
            .is_empty()
    );
}
#[test]
fn spell_totem_model_queries_return_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.model_for_totem_like_cpp(50, 2), 0);
}
#[test]
fn spell_totem_model_queries_match_spell_and_race_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_totem_model_store(Arc::new(test_spell_totem_model_store_like_cpp()));

    assert_eq!(
        session.model_for_totem_like_cpp(50, 2),
        2000,
        "C++ std::map assignment exposes the last valid duplicate row"
    );
    assert_eq!(session.model_for_totem_like_cpp(50, 8), 3000);
    assert_eq!(session.model_for_totem_like_cpp(50, 3), 0);
    assert_eq!(session.model_for_totem_like_cpp(51, 2), 0);
}
#[test]
fn spell_pet_aura_query_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_aura_like_cpp(77, 2).is_none());
}
#[test]
fn spell_pet_aura_query_matches_spell_effect_key_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_pet_aura_store(Arc::new(test_spell_pet_aura_store_like_cpp()));

    let aura = session.pet_aura_like_cpp(77, 2).expect("pet aura");
    assert!(aura.remove_on_change_pet);
    assert_eq!(aura.damage, 35);
    assert_eq!(aura.aura_for_pet_entry_like_cpp(501), 901);
    assert_eq!(aura.aura_for_pet_entry_like_cpp(502), 900);
    assert!(session.pet_aura_like_cpp(77, 3).is_none());
}
#[test]
fn spell_area_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_area_map_bounds_like_cpp(100).is_empty());
    assert!(
        session
            .spell_area_for_area_map_bounds_like_cpp(10)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_quest_map_bounds_like_cpp(20)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_quest_end_map_bounds_like_cpp(30)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_aura_map_bounds_like_cpp(40)
            .is_empty()
    );
}
#[test]
fn spell_area_queries_match_primary_and_secondary_indices_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_area_store(Arc::new(test_spell_area_store_like_cpp()));

    assert_eq!(session.spell_area_map_bounds_like_cpp(100).len(), 1);
    assert_eq!(session.spell_area_for_area_map_bounds_like_cpp(10).len(), 1);
    assert_eq!(
        session.spell_area_for_quest_map_bounds_like_cpp(20).len(),
        1
    );
    assert_eq!(
        session.spell_area_for_quest_map_bounds_like_cpp(30).len(),
        2
    );
    assert_eq!(
        session
            .spell_area_for_quest_end_map_bounds_like_cpp(30)
            .len(),
        2
    );
    assert_eq!(session.spell_area_for_aura_map_bounds_like_cpp(40).len(), 1);
}
#[test]
fn spell_custom_attributes_return_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 0),
        0
    );
}
#[test]
fn spell_custom_attributes_lookup_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    session
        .set_spell_custom_attribute_store(Arc::new(test_spell_custom_attribute_store_like_cpp()));

    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 0),
        wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP
            | wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 2),
        wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP
            | wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 1),
        0
    );
}
#[test]
fn serverside_spell_lookup_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.serverside_spell_like_cpp(100, 0).is_none());
}
#[test]
fn serverside_spell_lookup_uses_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_serverside_spell_store(Arc::new(test_serverside_spell_store_like_cpp()));

    assert_eq!(
        session
            .serverside_spell_like_cpp(100, 0)
            .unwrap()
            .row
            .spell_name,
        "server spell"
    );
    assert!(session.serverside_spell_like_cpp(100, 1).is_none());
}
#[test]
fn spell_learn_skill_query_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_learn_skill_like_cpp(10).is_none());
    assert_eq!(
        session.spell_learn_skill_lookup_like_cpp(10),
        wow_data::SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}
#[test]
fn spell_learn_skill_query_matches_loaded_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_store_like_cpp()));

    assert_eq!(
        session.spell_learn_skill_like_cpp(10),
        Some(&wow_data::SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
    assert_eq!(
        session.spell_learn_skill_like_cpp(20),
        Some(&wow_data::SpellLearnSkillNodeLikeCpp {
            skill: wow_data::SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
    assert!(session.spell_learn_skill_like_cpp(21).is_none());
    assert_eq!(
        session.spell_learn_skill_lookup_like_cpp(21),
        wow_data::SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}
#[test]
fn spell_learn_spell_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_learn_spell_map_bounds_like_cpp(10).is_empty());
    assert!(!session.is_spell_learn_spell_like_cpp(10));
    assert!(!session.is_spell_learn_to_spell_like_cpp(10, 20));
}
#[test]
fn spell_learn_spell_queries_match_loaded_multimap_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(test_spell_learn_spell_store_like_cpp()));

    assert_eq!(
        session.spell_learn_spell_map_bounds_like_cpp(10),
        &[wow_data::SpellLearnSpellNodeLikeCpp {
            spell: 20,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(session.is_spell_learn_spell_like_cpp(10));
    assert!(session.is_spell_learn_to_spell_like_cpp(10, 20));
    assert!(session.is_spell_learn_to_spell_like_cpp(30, 40));
    assert!(!session.is_spell_learn_to_spell_like_cpp(10, 21));
}
#[test]
fn pet_levelup_spell_list_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_levelup_spell_list_like_cpp(44).is_none());
}
#[test]
fn pet_levelup_spell_list_matches_family_multimap_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_levelup_spell_store(Arc::new(test_pet_levelup_spell_store_like_cpp()));

    let spells = session
        .pet_levelup_spell_list_like_cpp(44)
        .expect("pet levelup spell list");
    assert_eq!(
        spells.iter().collect::<Vec<_>>(),
        vec![(10, 701), (20, 700)]
    );
    assert!(session.pet_levelup_spell_list_like_cpp(45).is_none());
}
#[test]
fn pet_default_spells_entry_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_default_spells_entry_like_cpp(500).is_none());
}
#[test]
fn pet_default_spells_entry_matches_template_spells_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_default_spell_store(Arc::new(test_pet_default_spell_store_like_cpp()));

    let entry = session
        .pet_default_spells_entry_like_cpp(500)
        .expect("pet default spells entry");
    assert_eq!(entry.spellid, [10, 0, 11, 0]);
    assert!(session.pet_default_spells_entry_like_cpp(501).is_none());
}
#[test]
fn pet_family_spells_return_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_family_spells_like_cpp(44).is_none());
}
#[test]
fn pet_family_spells_match_passive_family_store_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_family_spell_store(Arc::new(test_pet_family_spell_store_like_cpp()));

    assert_eq!(session.pet_family_spells_like_cpp(44), Some(vec![800]));
    assert!(session.pet_family_spells_like_cpp(45).is_none());
}
