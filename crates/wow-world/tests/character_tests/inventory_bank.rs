use super::*;
use super::item_fixture_rows::{basic_item_record, inventory_sparse_template};
use wow_world::test_fixtures::make_inventory_bank_session_for_test as make_bank_slot_session;

fn install_bank_move_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    max_stack_size: i32,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([basic_item_record(
        entry_id,
        ItemClass::Miscellaneous as u8,
        0,
        InventoryType::NonEquip as i8,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry_id,
        ItemSparseTemplateEntry {
            stackable: max_stack_size,
            ..inventory_sparse_template(InventoryType::NonEquip as i8)
        },
    )])));
}

fn insert_bank_move_test_item(
    session: &mut WorldSession,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    count: u32,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    insert_inventory_item_for_test(
        session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let item = make_inventory_item_object_for_test(
        session,
        item_guid,
        entry_id,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    insert_inventory_item_object_for_test(session, item);
    item_guid
}

fn attach_stat_update_player_with_mana(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
) {
    let identity = loaded_player_identity_for_test(session);
    install_canonical_player_owner_for_test(
        session,
        u32::from(identity.0),
        0,
    );
    set_loaded_player_identity_like_cpp(session, 
        identity.0, identity.1, identity.2, identity.3, identity.4,
    );
    assert!(configure_inventory_player_vitals_for_test(
        session,
        player_guid,
        (100, 100, PowerType::Mana, current_mana, max_mana, 0),
    ));
}

#[test]
fn bank_move_plan_selects_first_personal_bank_slot_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 700, 10);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 700, 7_001, 3);

    let plan = inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::bank(),
        )
        .expect("source item")
        .expect("valid bank plan");

    assert_eq!(plan.source.guid, source_guid);
    assert!(plan.existing_updates.is_empty());
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 3))
    );
}

#[test]
fn bank_move_plan_merges_then_moves_one_remainder_stack_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 701, 10);
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 701, 7_011, 5);
    let existing_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        701,
        7_012,
        8,
    );

    let plan = inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::bank(),
        )
        .expect("source item")
        .expect("valid bank plan");

    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, existing_guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START + 1,
            3,
        ))
    );
}

#[test]
fn bank_move_plan_can_merge_and_leave_remainder_in_source_slot_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 705, 10);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        705,
        7_051,
        5,
    );
    let merge_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START + 1,
        705,
        7_052,
        8,
    );

    let plan = inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::bank(),
        )
        .expect("source item")
        .expect("valid consolidation plan");

    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, merge_guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 3))
    );
}

#[test]
fn bank_move_plan_reports_bank_full_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 706, 1);
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 706, 7_061, 1);
    for (index, slot) in
        (wow_entities::BANK_SLOT_ITEM_START..wow_entities::BANK_SLOT_ITEM_END).enumerate()
    {
        insert_bank_move_test_item(&mut session, slot, 706, 7_100 + index as u64, 1);
    }

    assert!(matches!(
        inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::bank(),
        ),
        Some(Err(InventoryResult::BankFull))
    ));
}

#[test]
fn autostore_bank_move_plan_returns_item_to_backpack_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 702, 1);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        702,
        7_021,
        1,
    );

    let plan = inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::inventory(),
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 1))
    );
}

#[test]
fn bank_merge_refreshes_destination_enchant_timer_without_item_duration_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(2);
    install_bank_move_item_fixture(&mut session, 708, 10);
    attach_stat_update_player_with_mana(&mut session, ObjectGuid::create_player(1, 42), 0, 0);
    let destination_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        708,
        7_081,
        8,
    );
    set_inventory_item_expiration_and_temporary_enchantment_for_test(
        &mut session,
        destination_guid,
        300,
        (940, 12_000, 1),
    );
    let mut tracked_item = inventory_item_objects_for_test(&session)[&destination_guid].clone();
    mutate_canonical_player_for_test(&session, |player| {
            player.add_enchantment_duration(
                &mut tracked_item,
                EnchantmentSlot::EnhancementTemporary,
                7_000,
            )
        })
        .expect("canonical player");

    inventory_refresh_enchantment_durations_for_test(&mut session, destination_guid);

    let mut packet = WorldPacket::from_bytes(
        &send_rx
            .try_recv()
            .expect("destination enchantment duration update"),
    );
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::ItemEnchantTimeUpdate as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), destination_guid);
    assert_eq!(packet.read_uint32().unwrap(), 12);
    assert_eq!(
        packet.read_uint32().unwrap(),
        EnchantmentSlot::EnhancementTemporary as u32
    );
    assert_eq!(
        packet.read_packed_guid().unwrap(),
        session.player_guid().unwrap()
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ merge branch refreshes AddEnchantmentDurations but does not emit AddItemDurations"
    );
}

#[test]
fn mainhand_bank_remove_clears_and_persists_weapon_only_enchant_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    attach_stat_update_player_with_mana(&mut session, ObjectGuid::create_player(1, 42), 0, 0);
    install_bank_move_item_fixture(&mut session, 707, 1);
    let item_guid =
        insert_bank_move_test_item(&mut session, wow_entities::EQUIPMENT_SLOT_MAINHAND, 707, 7_071, 1);
    let enchantment_entry = |id, flags| wow_data::SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
        effect_scaling_points: [0.0; 3],
        item_visual: 0,
        flags,
        required_skill_id: 0,
        required_skill_rank: 0,
        item_level: 1,
        charges: 0,
        effect: [wow_constants::ItemEnchantmentType::None as u8; 3],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    };
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([
            enchantment_entry(930, wow_constants::SpellItemEnchantmentFlags::MAINHAND_ONLY),
            enchantment_entry(931, wow_constants::SpellItemEnchantmentFlags::empty()),
            enchantment_entry(
                932,
                wow_constants::SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB,
            ),
        ]),
    ));
    set_equipped_inventory_item_enchantments_for_test(
        &mut session,
        item_guid,
        (930, 4_000, 2),
        (931, 3_000, 1),
        (932, 2_000, 3),
        (999, 1_000, 4),
    );
    let mut timed_item = inventory_item_object_for_test(&session, item_guid)
        .expect("canonical Player inventory item");
    mutate_canonical_player_for_test(&session, |player| {
            player.add_enchantment_duration(
                &mut timed_item,
                EnchantmentSlot::EnhancementTemporary,
                1_500,
            )
        })
        .expect("canonical player");

    let (persisted, cleared) = inventory_enchantment_persistence_for_test(&session, item_guid, true)
        .expect("main-hand-only enchantment");
    assert_eq!(cleared, vec![EnchantmentSlot::EnhancementPermanent]);
    let fields: Vec<_> = persisted.split_whitespace().collect();
    assert_eq!(&fields[0..3], &["0", "0", "0"]);
    assert_eq!(&fields[3..6], &["931", "1500", "1"]);
    assert_eq!(&fields[24..27], &["0", "0", "0"]);
    assert_eq!(&fields[27..30], &["0", "0", "0"]);

    let _ = inventory_remove_side_effects_for_test(&mut session, 
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_MAINHAND,
        item_guid,
        &cleared,
    );
    let item = inventory_item_object_for_test(&session, item_guid)
        .expect("canonical Player inventory item");
    assert!(!item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        0
    );
    let packet_update = inventory_storage_fields_update_for_test(&item, true, true, &cleared)
        .expect("item values update");
    let expected_mask = (1_u64 << wow_entities::ITEM_DATA_PARENT_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_CONTAINED_IN_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT)
        | (1_u64
            << (wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT
                + EnchantmentSlot::EnhancementPermanent as usize));
    assert_eq!(packet_update.item_data_mask, expected_mask);
    assert_eq!(packet_update.dynamic_flags2, 0);
    assert_eq!(
        packet_update.enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        0
    );
    assert_eq!(
        inventory_combat_recalculations_for_test(&session),
        &[
            InventoryCombatRecalculationForTest::expertise(wow_constants::WeaponAttackType::BaseAttack),
            InventoryCombatRecalculationForTest::rating(24),
        ]
    );
}

#[test]
fn committed_bank_relocation_updates_runtime_only_after_explicit_apply() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 703, 10);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 703, 7_031, 4);

    assert_eq!(
        wow_world::test_fixtures::get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        inventory_non_bank_count_for_test(&session, 703),
        Some(4)
    );
    assert!(inventory_apply_relocation_for_test(&mut session, 
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        INVENTORY_SLOT_BAG_0,
        wow_entities::BANK_SLOT_ITEM_START,
        4,
    ));
    assert!(
        wow_world::test_fixtures::get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .is_none()
    );
    assert_eq!(
        wow_world::test_fixtures::get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        inventory_non_bank_count_for_test(&session, 703),
        Some(0)
    );
}

#[test]
fn autostore_full_merge_reports_destination_stack_total_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 709, 10);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        709,
        7_091,
        2,
    );
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 709, 7_092, 8);

    let plan = inventory_storage_move_for_test(&session, 
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetForTest::inventory(),
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert!(plan.moved_destination.is_none());
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(bank_store_item_added_quest_count_for_test(&plan), 10);
}
