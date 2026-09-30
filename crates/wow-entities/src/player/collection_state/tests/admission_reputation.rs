use super::*;

#[test]
fn can_add_item_appearance_applies_can_use_item_reputation_gate_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    let player_guid = ObjectGuid::create_player(1, 93);
    fixture.install_player(
        player_guid,
        "TransmogReputationTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    fixture.modified = Some(Arc::new(
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
    install_appearance_test_items(
        &mut fixture,
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
    fixture.stats = Some(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (777, sparse_template_with_reputation_for_test(0, 0)),
                (778, sparse_template_with_reputation_for_test(72, 0)),
                (779, sparse_template_with_reputation_for_test(72, 7)),
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
    fixture.install_identity(571, 1, 1, 80, 0);
    fixture.player.add_weapon_proficiency_like_cpp(1 << (ItemSubClassWeapon::Sword as u32));
    fixture.factions = Some(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 5),
    ])));
    fixture.faction_templates = Some(Arc::new(FactionTemplateStore::from_entries([
        faction_template_entry(35, 72, 0, 0, 0),
        faction_template_entry(1, 1, 0, 0, 0),
    ])));

    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65));
    assert!(
        PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 66),
        "a zero-rank faction requirement is satisfied"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 67),
        "an Exalted requirement is not satisfied by the unloaded reputation"
    );
}


fn sparse_template_with_reputation_for_test(
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
