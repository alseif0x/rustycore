use super::*;

#[test]
fn can_add_item_appearance_applies_can_use_item_learning_effect_gate_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    let player_guid = ObjectGuid::create_player(1, 92);
    fixture.install_player(
        player_guid,
        "TransmogLearningEffectTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
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
                ItemQuality::Uncommon,
                [0, 0, 0, 0],
                0,
            ),
        ],
    );
    // C++ `Player::CanUseItem` learning-effect pair (`Player.cpp:11110-11113`):
    // effect 0 is `SPELL_EFFECT_LEARN_SPELL` (483) and effect 1 is the taught
    // spell.
    fixture.effects = Some(Arc::new(ItemEffectStore::from_entries([
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
    fixture.install_identity(571, 1, 1, 80, 0);
    fixture
        .player
        .add_weapon_proficiency_like_cpp(1 << (ItemSubClassWeapon::Sword as u32));
    fixture
        .player
        .replace_known_spell_ids_like_cpp(vec![12_345]);

    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(
        &fixture, 65
    ));
    assert!(
        !PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 66),
        "an already known learned spell blocks the item"
    );
    assert!(
        PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 67),
        "an unknown learned spell leaves the item usable"
    );
}
