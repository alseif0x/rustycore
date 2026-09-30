use super::*;

#[test]
fn can_add_item_appearance_uses_learned_weapon_proficiency_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    let player_guid = ObjectGuid::create_player(1, 90);
    fixture.install_player(
        player_guid,
        "TransmogProficiencyTester".to_string(),
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
                ItemSubClassWeapon::Mace as u8,
                InventoryType::Weapon,
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    fixture.install_identity(571, 1, 1, 80, 0);
    fixture.player.add_weapon_proficiency_like_cpp(1 << (ItemSubClassWeapon::Mace as u32));

    // C++ `CollectionMgr::CanAddAppearance` reads the learned
    // `Player::GetWeaponProficiency` mask, so the warrior class default is not
    // enough to collect a sword appearance.
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65),
        "an unlearned sword subclass must be rejected"
    );
    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 66));
}
