use super::*;

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
