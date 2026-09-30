use super::*;

#[test]
fn can_add_item_appearance_represented_applies_cpp_gates() {
    let mut fixture = AdmissionFixture::default();
    let player_guid = ObjectGuid::create_player(1, 79);
    fixture.install_player(
        player_guid,
        "TransmogCanAddTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    fixture.install_identity(571, 1, 1, 80, 0);
    fixture
        .player
        .add_weapon_proficiency_like_cpp(1 << (ItemSubClassWeapon::Sword as u32));
    fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
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
    ])));
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

    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 65
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 66
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 67
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 68
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 69
    ));
    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 70
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 71
    ));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 72
    ));
    // The canonical Player owns the collection; the session mirror is only the
    // handle-less fixture path.
    assert!(fixture.add_appearance(65).is_some());
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 65
    ));
}
