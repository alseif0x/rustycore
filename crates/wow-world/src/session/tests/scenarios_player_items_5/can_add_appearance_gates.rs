use super::*;

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
