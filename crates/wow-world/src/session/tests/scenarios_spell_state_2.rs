//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn remove_known_spell_clears_titan_grip_and_penalty_aura_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 774_i32;
    let penalty_spell_id = 49152_i32;
    let player_guid = ObjectGuid::create_player(1, 154);
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
        "RemoveTitanGrip".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.set_can_titan_grip(true, penalty_spell_id as u32);
    });
    session
        .apply_aura(penalty_spell_id, player_guid, 0, 0)
        .expect("represented penalty aura");

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP,
                effect_misc_value_1: penalty_spell_id,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.can_titan_grip(),
                player.titan_grip_penalty_spell_id(),
            )
        }),
        Some((false, 0)),
        "C++ Player::RemoveSpell clears m_canTitanGrip when the removed spell is passive and has SPELL_EFFECT_TITAN_GRIP"
    );
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == penalty_spell_id),
        "C++ RemoveSpell removes m_titanGripPenaltySpellId auras before SetCanTitanGrip(false)"
    );
}
#[test]
fn remove_known_spell_clears_dual_wield_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 775_i32;
    let player_guid = ObjectGuid::create_player(1, 155);
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
        "RemoveDualWield".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
    });

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() }),
        Some(false),
        "C++ Player::RemoveSpell clears m_canDualWield when the removed spell is passive and has SPELL_EFFECT_DUAL_WIELD"
    );
}
#[test]
fn remove_known_spell_records_offhand_auto_unequip_after_losing_dual_wield_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 776_i32;
    let offhand_item_id = 30_001_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_001);
    let player_guid = ObjectGuid::create_player(1, 156);
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
        "RemoveDualWieldOffhand".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
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
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[(
            offhand_item_id,
            InventoryType::WeaponOffhand,
            0,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe as u8,
        )],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield,
            stored_destination: Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)),
            needs_mail_fallback: false,
        }],
        "C++ RemoveSpell calls AutoUnequipOffhandIfNeed after losing dual wield"
    );
    assert!(
        !session
            .inventory_items_like_cpp()
            .contains_key(&EQUIPMENT_SLOT_OFFHAND),
        "C++ RemoveItem removes the item from EQUIPMENT_SLOT_OFFHAND before StoreItem"
    );
    assert_eq!(
        session
            .inventory_items_like_cpp()
            .get(&INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(offhand_guid),
        "represented StoreItem moves the offhand item to the selected backpack slot"
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            (
                player.active_data().inv_slots[EQUIPMENT_SLOT_OFFHAND as usize],
                player.active_data().inv_slots[INVENTORY_SLOT_ITEM_START as usize],
                player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
            )
        }),
        Some((
            ObjectGuid::EMPTY,
            offhand_guid,
            VisibleItemValues::default()
        )),
        "C++ RemoveItem clears offhand InvSlot/VisibleItem and StoreItem sets the backpack InvSlot"
    );
}
#[test]
fn remove_known_spell_honors_offhand_unlearn_config_and_always_allow_flag_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 777_i32;
    let offhand_item_id = 30_002_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_002);
    let player_guid = ObjectGuid::create_player(1, 157);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RemoveDualWieldConfig".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
    });
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[(
            offhand_item_id,
            InventoryType::WeaponOffhand,
            ItemFlags3::AlwaysAllowDualWield as u32,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe as u8,
        )],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert!(
        session
            .represented_auto_unequip_offhand_requests_like_cpp()
            .is_empty(),
        "C++ skips forced offhand unequip for ITEM_FLAG3_ALWAYS_ALLOW_DUAL_WIELD"
    );

    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
    });
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_offhand_check_at_spell_unlearn_like_cpp(false);
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        offhand_item_id,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 0.0,
            price_random_value: 0.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::WeaponOffhand as i8,
        },
    )])));

    session.remove_known_spell_like_cpp(spell_id);

    assert!(
        session
            .represented_auto_unequip_offhand_requests_like_cpp()
            .is_empty(),
        "C++ RemoveSpell skips AutoUnequipOffhandIfNeed when CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN is false"
    );
}
#[test]
fn remove_known_spell_auto_unequip_records_invalid_two_hand_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let mainhand_item_id = 30_003_u32;
    let offhand_item_id = 30_004_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_003);
    let offhand_guid = ObjectGuid::create_item(1, 30_004);
    let player_guid = ObjectGuid::create_player(1, 158);
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
        "InvalidTwoHandOffhand".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(false, 0);
    });
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                mainhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
            (
                offhand_item_id,
                InventoryType::Shield,
                0,
                ItemClass::Armor,
                ItemSubClassArmor::Shield as u8,
            ),
        ],
    );
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                offhand_item_id,
                ItemStatEntry {
                    stats: [
                        (ItemModType::Strength as i8, 7),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                    ],
                    resistances: [0; 7],
                    armor: 0,
                },
            )],
            [
                (
                    mainhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
                (
                    offhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Shield, 0),
                ),
            ],
            [],
        ),
    ));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 705,
        name: "Offhand Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { offhand_item_id } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 21,
            chr_spec_id: 0,
            spell_id: 9021,
            threshold: 1,
            item_set_id: 705,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Shield,
    );
    session.update_inventory_item_object_like_cpp(offhand_guid, |item| {
        item.set_soulbound_tradeable([player_guid]);
        item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        item.set_expiration(300);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    });
    let tradeable_offhand = session.inventory_item_objects_like_cpp()[&offhand_guid].clone();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        let mut duration_item = tradeable_offhand.clone();
        player.add_tradeable_item(&duration_item);
        let _ = player.add_item_durations(&duration_item);
        let _ = player.add_enchantment_duration(
            &mut duration_item,
            EnchantmentSlot::EnhancementTemporary,
            7_000,
        );
    });
    assert!(
        session.inventory_item_objects_like_cpp()[&offhand_guid]
            .has_item_flag2(ItemFieldFlags2::EQUIPPED),
        "fixture starts with the offhand ITEM_FIELD_FLAG2_EQUIPPED bit set"
    );
    assert!(session.record_represented_items_set_item_like_cpp(offhand_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 705,
            spell_entry_id: 21,
            spell_id: 9021,
            threshold: 1,
            apply: true,
        }]
    );

    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .soulbound_tradeable_items()
            .contains(&offhand_guid)),
        Some(true),
        "fixture starts with the offhand in C++ m_itemSoulboundTradeable"
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.item_durations().to_vec(),
            player.enchant_durations().to_vec()
        )),
        Some((
            vec![offhand_guid],
            vec![PlayerEnchantDuration {
                item_guid: offhand_guid,
                slot: EnchantmentSlot::EnhancementTemporary,
                left_duration_ms: 7_000,
            }]
        )),
        "fixture starts with C++ item/enchantment duration refs registered on the player"
    );

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::InvalidTwoHandState,
            stored_destination: Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)),
            needs_mail_fallback: false,
        }],
        "C++ AutoUnequipOffhandIfNeed unequips offhand when the main hand is a 2H weapon without Titan Grip"
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .soulbound_tradeable_items()
            .contains(&offhand_guid)),
        Some(false),
        "C++ RemoveItem removes the item from m_itemSoulboundTradeable before item mod cleanup"
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.item_durations().to_vec(),
            player.enchant_durations().to_vec()
        )),
        Some((Vec::new(), Vec::new())),
        "C++ RemoveItem removes item and enchantment duration refs before item mod cleanup"
    );
    assert!(
        !session.inventory_item_objects_like_cpp()[&offhand_guid]
            .has_item_flag2(ItemFieldFlags2::EQUIPPED),
        "C++ RemoveItem clears ITEM_FIELD_FLAG2_EQUIPPED for equipped top-level slots"
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: offhand_guid,
            slot: EQUIPMENT_SLOT_OFFHAND,
            apply: false,
        }],
        "C++ RemoveItem calls _ApplyItemMods(offhand, false) before clearing the equipment slot"
    );
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp()[1],
        RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 705,
            spell_entry_id: 21,
            spell_id: 9021,
            threshold: 1,
            apply: false,
        },
        "C++ RemoveItem removes item-set bonuses before _ApplyItemMods for equipped top-level slots"
    );
    assert_eq!(
        session.represented_combat_stat_recalculations_like_cpp(),
        &[
            RepresentedCombatStatRecalculationLikeCpp::Expertise {
                attack: WeaponAttackType::OffAttack,
            },
            RepresentedCombatStatRecalculationLikeCpp::Rating {
                combat_rating: CR_ARMOR_PENETRATION_LIKE_CPP,
            },
        ],
        "C++ RemoveItem updates offhand expertise and recalculates armor penetration for weapon/armor slots"
    );
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&offhand_guid)
        .expect("stored backpack item remains a runtime item object");
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        7_000,
        "C++ RemoveEnchantmentDurations writes the remaining duration back onto the item"
    );
    assert_eq!(runtime_item.data().contained_in, player_guid);
    assert_eq!(runtime_item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(runtime_item.slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject
        ],
        "C++ RemoveItem(update=true) + StoreItem(update=true) send player/item values updates; the represented item-mod stat delta is emitted after the offhand leaves equipment"
    );
}
#[test]
fn represented_item_level_area_scaling_activates_on_pvp_rules_aura_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_156_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 30);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_156, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_156, Position::ZERO);
    session
        .visible_auras
        .insert(1, test_visible_aura(1, SPELL_PVP_RULES_ENABLED_LIKE_CPP));

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(130)
    );
}
#[test]
fn represented_pvp_rules_aura_application_recalculates_item_level_scaling_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_158_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 35);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_158, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_158, Position::ZERO);

    assert!(!session.represented_using_pvp_item_levels_like_cpp());
    session
        .apply_aura(
            SPELL_PVP_RULES_ENABLED_LIKE_CPP,
            ObjectGuid::EMPTY,
            30_000,
            0x0000_0001,
        )
        .expect("represented PvP rules aura should apply");

    assert!(
        session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::EnablePvpRules calls UpdateItemLevelAreaBasedScaling after applying SPELL_PVP_RULES_ENABLED"
    );
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(135)
    );
}
#[test]
fn represented_pvp_rules_aura_removal_recalculates_item_level_scaling_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_159_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 40);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_159, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_159, Position::ZERO);
    session
        .apply_aura(
            SPELL_PVP_RULES_ENABLED_LIKE_CPP,
            ObjectGuid::EMPTY,
            30_000,
            0x0000_0001,
        )
        .expect("represented PvP rules aura should apply");
    let slot = session
        .visible_auras
        .values()
        .find_map(|aura| (aura.spell_id == SPELL_PVP_RULES_ENABLED_LIKE_CPP).then_some(aura.slot))
        .expect("PvP rules aura slot");

    session
        .remove_aura(slot)
        .expect("represented PvP rules aura should remove");

    assert!(
        !session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::DisablePvpRules removes SPELL_PVP_RULES_ENABLED and then calls UpdateItemLevelAreaBasedScaling"
    );
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100)
    );
}
#[test]
fn remove_known_spell_auto_unequip_skips_item_mod_remove_for_broken_offhand_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_010_u32;
    let offhand_item_id = 30_011_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_010);
    let offhand_guid = ObjectGuid::create_item(1, 30_011);
    let player_guid = ObjectGuid::create_player(1, 161);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "BrokenOffhandMods".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(false, 0);
    });
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                mainhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
            (
                offhand_item_id,
                InventoryType::Shield,
                0,
                ItemClass::Armor,
                ItemSubClassArmor::Shield as u8,
            ),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Shield,
    );
    session.update_inventory_item_object_like_cpp(offhand_guid, |item| {
        item.set_max_durability(40);
        item.set_durability(0);
    });

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty(),
        "C++ _ApplyItemMods returns without applying/removing mods for broken items"
    );
}
