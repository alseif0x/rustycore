//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn item_modified_appearance_helpers_use_cpp_lookup_shapes() {
    let (mut session, _, _) = make_session();
    session.set_item_appearance_store(Arc::new(ItemAppearanceStore::from_entries([
        ItemAppearanceEntry {
            id: 1000,
            display_type: 0,
            item_display_info_id: 555,
            default_icon_file_data_id: 0,
            ui_order: 0,
        },
        ItemAppearanceEntry {
            id: 1001,
            display_type: 0,
            item_display_info_id: 777,
            default_icon_file_data_id: 0,
            ui_order: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 10,
                item_id: 100,
                item_appearance_modifier_id: 0,
                item_appearance_id: 1000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 11,
                item_id: 100,
                item_appearance_modifier_id: 2,
                item_appearance_id: 1001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));

    assert_eq!(session.item_modified_appearance_ref(11), Some((100, 2)));
    assert_eq!(session.item_modified_appearance_for_item(100, 2), Some(11));
    assert_eq!(session.item_modified_appearance_for_item(100, 9), Some(10));
    assert_eq!(session.item_modified_appearance_for_item(101, 0), None);
    assert_eq!(session.item_display_id(100, 2), Some(777));
    assert_eq!(session.item_display_id(100, 9), Some(555));
    assert_eq!(session.item_display_id(101, 0), None);
}
#[test]
fn transmog_set_items_helper_uses_cpp_lookup_shape() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 1000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 71,
            item_modified_appearance_id: 2000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 1001,
            flags: 0,
        },
    ])));

    let items = session
        .transmog_set_items_like_cpp(70)
        .expect("transmog set should have indexed items");
    assert_eq!(
        items
            .iter()
            .map(|item| item.item_modified_appearance_id)
            .collect::<Vec<_>>(),
        vec![1000, 1001]
    );
    assert!(session.transmog_set_items_like_cpp(99).is_none());
}
#[test]
fn transmog_set_item_modified_appearances_skip_missing_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 1000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 70,
            item_modified_appearance_id: 1001,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 1001,
            item_id: 777,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));

    let appearances = session.transmog_set_item_modified_appearances_like_cpp(70);
    assert_eq!(appearances.len(), 1);
    assert_eq!(appearances[0].id, 1001);
    assert_eq!(appearances[0].item_id, 777);
    assert!(
        session
            .transmog_set_item_modified_appearances_like_cpp(99)
            .is_empty()
    );
}
#[test]
fn add_item_appearance_resizes_blocks_and_marks_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 71);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_item_appearance_like_cpp(65)
        .expect("represented current player should receive transmog appearance");
    let active = update
        .active_player_data
        .expect("transmog appearance should mark active player data");

    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert_eq!(active.values.transmog, vec![0, 0, 1 << 1]);
    assert_eq!(active.values.transmog_update_mask, Some(vec![0b111]));
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_TRANSMOG_BIT)
    );
    assert!(session.add_item_appearance_like_cpp(65).is_none());
}
#[test]
fn add_item_appearance_for_item_resolves_modified_appearance_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 77);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogItemResolverTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 777,
            item_appearance_modifier_id: 2,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        777,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_item_appearance_for_item_like_cpp(777, 2)
        .expect("represented item appearance should resolve by item/modifier");
    let active = update
        .active_player_data
        .expect("resolved item appearance should mark active player data");

    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert_eq!(active.values.transmog, vec![0, 0, 1 << 1]);
    assert!(
        session
            .add_item_appearance_for_item_like_cpp(777, 2)
            .is_none()
    );
    assert!(
        session
            .add_item_appearance_for_item_like_cpp(778, 2)
            .is_none()
    );
}
#[test]
fn on_item_added_adds_heirloom_and_permanent_appearance_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 86);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 86_777);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "collection item".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [0; 6],
        upgrade_item_bonus_list_id: [0; 6],
    }])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 44_000,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        44_000,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    let mut item = session.make_inventory_item_object(
        item_guid,
        44_000,
        player_guid,
        1,
        0,
        ItemContext::None,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND);

    let updates = session.on_item_added_to_collection_like_cpp(&item);

    assert_eq!(updates.len(), 2);
    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(44_000, 0)]);
    assert!(session.represented_has_item_appearance_like_cpp(65));
    let canonical_fields = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.heirlooms_like_cpp().to_vec(),
                player.heirloom_flags_like_cpp().to_vec(),
                player.transmog_blocks_like_cpp().to_vec(),
            )
        })
        .unwrap();
    assert_eq!(
        canonical_fields,
        (vec![44_000], vec![0], vec![0, 0, 1 << 1])
    );
}
#[test]
fn on_item_added_records_refundable_appearance_as_temporary_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 87);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 87_777);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 96,
            item_id: 44_100,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_100,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        44_100,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    let mut item = session.make_inventory_item_object(
        item_guid,
        44_100,
        player_guid,
        1,
        0,
        ItemContext::Vendor,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE);

    let updates = session.on_item_added_to_collection_like_cpp(&item);

    assert_eq!(updates.len(), 1);
    assert_eq!(session.has_item_appearance_like_cpp(96), (true, true));
    assert!(!session.represented_has_item_appearance_like_cpp(96));
    assert_eq!(
        session.items_providing_temporary_appearance_like_cpp(96),
        HashSet::from([item_guid])
    );
}
#[test]
fn can_add_item_appearance_uses_learned_weapon_proficiency_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogProficiencyTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 66,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Weapon,
                ItemSubClassWeapon::Mace as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    grant_learned_weapon_proficiency_like_cpp(&mut session, 1 << (ItemSubClassWeapon::Mace as u32));

    // C++ `CollectionMgr::CanAddAppearance` reads the learned
    // `Player::GetWeaponProficiency` mask, so the warrior class default is not
    // enough to collect a sword appearance.
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(65),
        "an unlearned sword subclass must be rejected"
    );
    assert!(session.can_add_item_appearance_represented_like_cpp(66));
}

#[test]
fn can_add_item_appearance_applies_can_use_item_template_gates_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 91);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogCanUseItemTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let appearances: Vec<ItemModifiedAppearanceEntry> = (0..7)
        .map(|index: i32| ItemModifiedAppearanceEntry {
            id: (65 + index) as u32,
            item_id: 777 + index,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_000 + index,
            order_index: 0,
            transmog_source_type_enum: 0,
        })
        .collect();
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries(appearances),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, ItemFlags2::InternalItem as u32, 0, 0],
                0,
            ),
            (
                779,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, ItemFlags2::FactionHorde as u32, 0, 0],
                0,
            ),
            (
                780,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                781,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                782,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                783,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    // C++ `Player::CanUseItem(ItemTemplate const*)` (`Player.cpp:11069-11125`)
    // reads the search-name requirement columns for the level, ability and race
    // gates.
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries(
        [
            (777, 0, 0, 0),
            (778, 0, 0, 0),
            (779, 0, 0, 0),
            (780, 81, 0, 0),
            (781, 0, 12_345, 0),
            (782, 0, 54_321, 0),
            (783, 0, 0, 1 << 1),
        ]
        .into_iter()
        .map(
            |(item_id, required_level, required_ability, allowable_race)| ItemSearchNameEntry {
                id: item_id,
                allowable_race,
                display: String::new(),
                overall_quality_id: ItemQuality::Uncommon as u8,
                expansion_id: 0,
                min_faction_id: 0,
                min_reputation: 0,
                allowable_class: 0,
                required_level,
                required_skill: 0,
                required_skill_rank: 0,
                required_ability,
                item_level: 1,
                flags: [0; 4],
            },
        ),
    )));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_known_spells_like_cpp(vec![12_345]);

    assert!(
        session.can_add_item_appearance_represented_like_cpp(65),
        "a plain usable weapon appearance is collectable"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(66),
        "ITEM_FLAG2_INTERNAL_ITEM is rejected"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(67),
        "an opposite-faction item is rejected"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(68),
        "an item above the player level is rejected"
    );
    assert!(
        session.can_add_item_appearance_represented_like_cpp(69),
        "a known required ability passes"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(70),
        "an unknown required ability is rejected"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(71),
        "an item restricted to another race is rejected"
    );
}

/// C++ `ItemSparseEntry` with only the reputation requirement varied; the rest
/// matches `install_transmog_can_add_test_items`.
fn sparse_template_with_reputation_like_cpp(
    required_reputation_faction: u16,
    required_reputation_rank: i32,
) -> ItemSparseTemplateEntry {
    ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank,
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
        required_reputation_faction,
        allowable_class: 0,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::Weapon as i8,
    }
}

#[test]
fn can_add_item_appearance_applies_can_use_item_learning_effect_gate_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 92);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogLearningEffectTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 66,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 67,
                item_id: 779,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_002,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                779,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    // C++ `Player::CanUseItem` learning-effect pair (`Player.cpp:11110-11113`):
    // effect 0 is `SPELL_EFFECT_LEARN_SPELL` (483) and effect 1 is the taught
    // spell.
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: 483,
            chr_specialization_id: 0,
            parent_item_id: 778,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 1,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: 12_345,
            chr_specialization_id: 0,
            parent_item_id: 778,
        },
        ItemEffectEntry {
            id: 3,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: 483,
            chr_specialization_id: 0,
            parent_item_id: 779,
        },
        ItemEffectEntry {
            id: 4,
            legacy_slot_index: 1,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: 54_321,
            chr_specialization_id: 0,
            parent_item_id: 779,
        },
    ])));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_known_spells_like_cpp(vec![12_345]);

    assert!(session.can_add_item_appearance_represented_like_cpp(65));
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(66),
        "an already known learned spell blocks the item"
    );
    assert!(
        session.can_add_item_appearance_represented_like_cpp(67),
        "an unknown learned spell leaves the item usable"
    );
}

#[test]
fn can_add_item_appearance_applies_can_use_item_reputation_gate_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 93);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogReputationTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 66,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 67,
                item_id: 779,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_002,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                779,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (777, sparse_template_with_reputation_like_cpp(0, 0)),
                (778, sparse_template_with_reputation_like_cpp(72, 0)),
                (779, sparse_template_with_reputation_like_cpp(72, 7)),
            ],
            [
                (
                    777,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 1,
                        quality: ItemQuality::Uncommon as i8,
                        inventory_type: InventoryType::Weapon as i8,
                    },
                ),
                (
                    778,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 1,
                        quality: ItemQuality::Uncommon as i8,
                        inventory_type: InventoryType::Weapon as i8,
                    },
                ),
                (
                    779,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 1,
                        quality: ItemQuality::Uncommon as i8,
                        inventory_type: InventoryType::Weapon as i8,
                    },
                ),
            ],
        ),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 5),
    ])));
    session.set_faction_template_store(Arc::new(FactionTemplateStore::from_entries([
        faction_template_entry(35, 72, 0, 0, 0),
        faction_template_entry(1, 1, 0, 0, 0),
    ])));

    assert!(session.can_add_item_appearance_represented_like_cpp(65));
    assert!(
        session.can_add_item_appearance_represented_like_cpp(66),
        "a zero-rank faction requirement is satisfied"
    );
    assert!(
        !session.can_add_item_appearance_represented_like_cpp(67),
        "an Exalted requirement is not satisfied by the unloaded reputation"
    );
}

#[test]
fn can_add_item_appearance_represented_applies_cpp_gates() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 79);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogCanAddTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 66,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 6,
            },
            ItemModifiedAppearanceEntry {
                id: 67,
                item_id: 779,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_002,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 68,
                item_id: 780,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_003,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 69,
                item_id: 781,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_004,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 70,
                item_id: 782,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_005,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 71,
                item_id: 783,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_006,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 72,
                item_id: 784,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_007,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                779,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Artifact,
                [0, 0, 0, 0],
                0,
            ),
            (
                780,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, ItemFlags2::NoSourceForItemVisual as u32, 0, 0],
                0,
            ),
            (
                781,
                ItemClass::Armor,
                ItemSubClassArmor::Miscellaneous as u8,
                InventoryType::Cloak,
                ItemQuality::Normal,
                [0, 0, 0, 0],
                0,
            ),
            (
                782,
                ItemClass::Armor,
                ItemSubClassArmor::Miscellaneous as u8,
                InventoryType::Cloak,
                ItemQuality::Normal,
                [
                    0,
                    ItemFlags2::IgnoreQualityForItemVisualSource as u32,
                    ItemFlags3::ActsAsTransmogHiddenVisualOption as u32,
                    0,
                ],
                0,
            ),
            (
                783,
                ItemClass::Armor,
                ItemSubClassArmor::Cloth as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                784,
                ItemClass::Weapon,
                ItemSubClassWeapon::Thrown as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );

    assert!(session.can_add_item_appearance_represented_like_cpp(65));
    assert!(!session.can_add_item_appearance_represented_like_cpp(66));
    assert!(!session.can_add_item_appearance_represented_like_cpp(67));
    assert!(!session.can_add_item_appearance_represented_like_cpp(68));
    assert!(!session.can_add_item_appearance_represented_like_cpp(69));
    assert!(session.can_add_item_appearance_represented_like_cpp(70));
    assert!(!session.can_add_item_appearance_represented_like_cpp(71));
    assert!(!session.can_add_item_appearance_represented_like_cpp(72));
    // The canonical Player owns the collection; the session mirror is only the
    // handle-less fixture path.
    assert!(session.add_item_appearance_like_cpp(65).is_some());
    assert!(!session.can_add_item_appearance_represented_like_cpp(65));
}
#[test]
fn replay_rewarded_quest_direct_item_appearances_adds_choice_and_fixed_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 78);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RewardedQuestAppearanceTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Weapon,
                ItemSubClassWeapon::Sword as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Armor,
                ItemSubClassArmor::Plate as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    let mut quest = test_quest_template(7_777);
    quest.reward_choice_items[0] = (777, 1);
    quest.reward_choice_items[1] = (0, 1);
    quest.reward_items[0] = 778;
    quest.reward_items[1] = 999;
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .replay_rewarded_quest_direct_item_appearances_like_cpp(&quest)
        .expect("direct rewarded quest appearances should mark the player");
    let active = update
        .active_player_data
        .expect("rewarded quest replay should emit active player data");

    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert!(session.represented_has_item_appearance_like_cpp(96));
    assert_eq!(active.values.transmog, vec![0, 0, 1 << 1, 1]);
    assert!(
        session
            .replay_rewarded_quest_direct_item_appearances_like_cpp(&quest)
            .is_none()
    );
}
#[test]
fn item_spec_class_mask_from_overrides_uses_chr_specialization_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_spec_override_store(Arc::new(ItemSpecOverrideStore::from_entries([
        ItemSpecOverrideEntry {
            id: 1,
            spec_id: 66,
            item_id: 777,
        },
        ItemSpecOverrideEntry {
            id: 2,
            spec_id: 70,
            item_id: 777,
        },
        ItemSpecOverrideEntry {
            id: 3,
            spec_id: 999,
            item_id: 778,
        },
    ])));
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 66,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
        ChrSpecializationEntry {
            id: 70,
            class_id: 3,
            order_index: 0,
            role: 0,
        },
    ])));

    assert_eq!(
        session.item_spec_class_mask_from_overrides_like_cpp(777),
        Some((1 << 1) | (1 << 2))
    );
    assert_eq!(
        session.item_spec_class_mask_from_overrides_like_cpp(778),
        Some(0)
    );
    assert_eq!(
        session.item_spec_class_mask_from_overrides_like_cpp(999),
        None
    );
}
#[test]
fn replay_rewarded_quest_package_item_appearances_uses_item_spec_class_mask_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 79);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RewardedQuestPackageAppearanceTester".to_string(),
        player_position,
        571,
        1,
        2,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 97,
                item_id: 779,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_002,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 98,
                item_id: 780,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9_003,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    install_transmog_can_add_test_items(
        &mut session,
        [
            (
                777,
                ItemClass::Armor,
                ItemSubClassArmor::Plate as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                778,
                ItemClass::Armor,
                ItemSubClassArmor::Plate as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                779,
                ItemClass::Armor,
                ItemSubClassArmor::Plate as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
            (
                780,
                ItemClass::Armor,
                ItemSubClassArmor::Plate as u8,
                InventoryType::Chest,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 66,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
        ChrSpecializationEntry {
            id: 70,
            class_id: 3,
            order_index: 0,
            role: 0,
        },
    ])));
    session.set_item_spec_override_store(Arc::new(ItemSpecOverrideStore::from_entries([
        ItemSpecOverrideEntry {
            id: 1,
            spec_id: 66,
            item_id: 777,
        },
        ItemSpecOverrideEntry {
            id: 2,
            spec_id: 70,
            item_id: 778,
        },
        ItemSpecOverrideEntry {
            id: 3,
            spec_id: 66,
            item_id: 779,
        },
    ])));
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: 44,
            item_id: 777,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
        },
        QuestPackageItemEntry {
            id: 2,
            package_id: 44,
            item_id: 778,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
        },
        QuestPackageItemEntry {
            id: 3,
            package_id: 44,
            item_id: 779,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
        QuestPackageItemEntry {
            id: 4,
            package_id: 44,
            item_id: 780,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
        },
    ])));
    let mut quest = test_quest_template(7_778);
    quest.quest_package_id = 44;
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    assert!(
        session
            .replay_rewarded_quest_item_appearances_like_cpp(&quest)
            .is_some()
    );
    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert!(!session.represented_has_item_appearance_like_cpp(96));
    assert!(!session.represented_has_item_appearance_like_cpp(97));
    assert!(!session.represented_has_item_appearance_like_cpp(98));
}
#[test]
fn has_item_appearance_reports_permanent_before_temporary_like_cpp() {
    let (mut session, _, _) = make_session();

    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));

    session
        .represented_temporary_item_appearances_like_cpp
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 900)]));
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    session.represented_item_appearances_like_cpp.insert(65);
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, false));
}
#[test]
fn account_transmog_update_is_not_sent_while_opcode_is_unresolved_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(65, FavoriteAppearanceStateLikeCpp::Unchanged);

    session.send_favorite_appearances_like_cpp();

    assert_eq!(
        send_rx.try_recv(),
        Err(flume::TryRecvError::Empty),
        "do not send SMSG_ACCOUNT_TRANSMOG_UPDATE while legacy C++ keeps it at NULL_OPCODE/0xBADD"
    );
}
