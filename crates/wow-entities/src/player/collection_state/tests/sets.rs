use super::*;

#[test]
fn is_transmog_set_completed_ignores_temporary_and_missing_entries_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.sets = Some(Arc::new(TransmogSetItemStore::from_entries([
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
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 999,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 4,
            transmog_set_id: 71,
            item_modified_appearance_id: 300,
            flags: 0,
        },
    ])));
    fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
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
        ItemModifiedAppearanceEntry {
            id: 300,
            item_id: 779,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9002,
            order_index: 0,
            transmog_source_type_enum: 0,
        },
    ])));
    fixture.items = Some(Arc::new(ItemStore::from_records([
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
        ItemRecord {
            id: 779,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));

    assert!(!fixture.set_complete(99));
    assert!(fixture.set_complete(71));

    fixture
        .player
        .gameplay_state_mut()
        .collections
        .temporary_item_appearances
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 901)]));
    fixture
        .player
        .gameplay_state_mut()
        .collections
        .item_appearances
        .insert(96);
    assert!(!fixture.set_complete(70));

    fixture
        .player
        .gameplay_state_mut()
        .collections
        .item_appearances
        .insert(65);
    assert!(fixture.set_complete(70));
}

#[test]
fn is_transmog_set_completed_keeps_first_completed_slot_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.sets = Some(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 80,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 80,
            item_modified_appearance_id: 96,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 81,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 4,
            transmog_set_id: 81,
            item_modified_appearance_id: 96,
            flags: 0,
        },
    ])));
    fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
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
    ])));
    fixture.items = Some(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 777,
            class_id: 2,
            subclass_id: 7,
            material: 0,
            inventory_type: InventoryType::Weapon as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 778,
            class_id: 2,
            subclass_id: 7,
            material: 0,
            inventory_type: InventoryType::WeaponOffhand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));

    fixture
        .player
        .gameplay_state_mut()
        .collections
        .temporary_item_appearances
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 901)]));
    fixture
        .player
        .gameplay_state_mut()
        .collections
        .item_appearances
        .insert(96);
    assert!(fixture.set_complete(80));

    fixture
        .player
        .gameplay_state_mut()
        .collections
        .item_appearances
        .remove(&96);
    fixture
        .player
        .gameplay_state_mut()
        .collections
        .item_appearances
        .insert(65);
    fixture
        .player
        .gameplay_state_mut()
        .collections
        .temporary_item_appearances
        .insert(96, HashSet::from([ObjectGuid::create_item(1, 902)]));
    assert!(fixture.set_complete(81));
}
