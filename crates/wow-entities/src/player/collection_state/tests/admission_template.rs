use super::*;

#[test]
fn can_add_item_appearance_applies_can_use_item_template_gates_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    let player_guid = ObjectGuid::create_player(1, 91);
    fixture.install_player(
        player_guid,
        "TransmogCanUseItemTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
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
    fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries(
        appearances,
    )));
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
    fixture.search = Some(Arc::new(ItemSearchNameStore::from_entries(
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
    fixture.install_identity(571, 1, 1, 80, 0);
    fixture
        .player
        .add_weapon_proficiency_like_cpp(1 << (ItemSubClassWeapon::Sword as u32));
    fixture
        .player
        .replace_known_spell_ids_like_cpp(vec![12_345]);

    assert!(
        PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65),
        "a plain usable weapon appearance is collectable"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 66),
        "ITEM_FLAG2_INTERNAL_ITEM is rejected"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 67),
        "an opposite-faction item is rejected"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 68),
        "an item above the player level is rejected"
    );
    assert!(
        PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 69),
        "a known required ability passes"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 70),
        "an unknown required ability is rejected"
    );
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 71),
        "an item restricted to another race is rejected"
    );
}
