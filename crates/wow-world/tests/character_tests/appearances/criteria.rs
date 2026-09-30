use super::*;

#[test]
fn add_item_appearance_records_transmog_criteria_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    enable_appearance_criteria_diagnostics_for_test(&mut session);
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 78);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    attach_appearance_player_for_test(
        &mut session,
        player_guid,
        "TransmogCriteriaTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    );
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries_and_sets(
        [
            TransmogSetItemEntry {
                id: 1,
                transmog_set_id: 70,
                item_modified_appearance_id: 65,
                flags: 0,
            },
            TransmogSetItemEntry {
                id: 2,
                transmog_set_id: 70,
                item_modified_appearance_id: 96,
                flags: 0,
            },
        ],
        [TransmogSetEntry {
            id: 70,
            name: "criterion set".to_string(),
            class_mask: 0,
            tracking_quest_id: 0,
            flags: 0,
            transmog_set_group_id: 7000,
            item_name_description_id: 0,
            parent_transmog_set_id: 0,
            expansion_id: 0,
            ui_order: 0,
        }],
    )));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 96,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 777,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Head as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 778,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());
    appearance_seed_temporary_provider_for_test(&session, 96, HashSet::from([ObjectGuid::create_item(1, 902)]));

    session
        .add_item_appearance_like_cpp(65)
        .expect("first permanent piece should update transmog state");
    assert_eq!(
        appearance_criteria_events_for_test(&session),
        vec![RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
            equipment_slot: EQUIPMENT_SLOT_HEAD as u32,
            item_modified_appearance_id: 65,
        }]
    );

    clear_appearance_criteria_events_for_test(&mut session);
    session
        .add_item_appearance_like_cpp(96)
        .expect("second permanent piece should complete the set");
    assert_eq!(
        appearance_criteria_events_for_test(&session),
        vec![
            RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                equipment_slot: EQUIPMENT_SLOT_CHEST as u32,
                item_modified_appearance_id: 96,
            },
            RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                transmog_set_group_id: 7000,
            },
        ]
    );
}
