//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn child_equip_plan_rejects_displacement_when_parent_started_equipped_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry = 128;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        entry,
        entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry,
        82,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        entry,
        83,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry,
        84,
        InventoryType::Weapon,
    );

    assert_eq!(
        session.plan_inventory_equip_child_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            parent_guid,
        ),
        Err(InventoryResult::CantSwap)
    );
}
#[tokio::test]
async fn recursive_destroy_commit_failure_keeps_bag_and_children_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 93);
    let child_guid = ObjectGuid::create_item(1, 94);
    session.set_player_guid(Some(player_guid));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 93,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(bag.clone());
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_ITEM_START);
    session.insert_inventory_item_object(child);
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );
    let item = session
        .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
        .expect("bag metadata");

    assert!(
        !session
            .destroy_inventory_full_stack_by_pos_like_cpp(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                item,
                Some(bag),
                "recursive destroy test",
            )
            .await
    );
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&bag_guid)
    );
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&child_guid)
    );
    assert_eq!(
        inventory_failure_result(&send_rx.try_recv().expect("transaction failure packet")),
        InventoryResult::InternalBagError as i32
    );
}
#[tokio::test]
async fn auto_equip_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_auto_equip_item(AutoEquipItem {
            inv_update: InvUpdate { items: Vec::new() },
            pack_slot: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before missing-item handling"
    );
}
#[tokio::test]
async fn auto_store_bag_item_rejects_non_empty_inv_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_auto_store_bag_item(AutoStoreBagItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on non-empty InvUpdate before source-container validation"
    );
}
#[tokio::test]
async fn buy_bank_slot_buys_next_slot_and_spends_money_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 1);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 1);
    assert_eq!(session.player_gold_like_cpp(), 50);
    assert!(
        send_rx.try_recv().is_ok(),
        "bank slot update should be sent"
    );
    assert!(send_rx.try_recv().is_ok(), "money update should be sent");
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}
#[tokio::test]
async fn buy_bank_slot_definite_rollback_keeps_runtime_and_packets_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "fixture rollback".to_owned(),
        },
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "a definite rollback must reopen payout admission"
    );
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}
#[tokio::test]
async fn buy_bank_slot_rejects_non_banker_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 2);
    insert_banker_creature(&canonical, creature, NPCFlags1::QUEST_GIVER.bits());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: creature })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn buy_bank_slot_rejects_missing_price_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 3);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_bank_bag_slot_count_like_cpp(2);

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 2);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn bank_move_plan_selects_first_personal_bank_slot_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 700, 10);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 700, 7_001, 3);

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
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

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
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
    session.update_inventory_item_object_like_cpp(destination_guid, |item| {
        item.set_expiration(300);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 940, 12_000, 1);
    });
    let mut tracked_item = session.inventory_item_objects_like_cpp()[&destination_guid].clone();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut tracked_item,
                EnchantmentSlot::EnhancementTemporary,
                7_000,
            )
        })
        .expect("canonical player");

    session.refresh_inventory_item_enchantment_duration_refs_like_cpp(destination_guid);

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

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
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
        session.plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
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

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 1))
    );
}
#[test]
fn bank_move_quest_removal_follows_opcode_not_direction_like_cpp() {
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetLikeCpp::Bank),
        InventoryStorageQuestChecksLikeCpp::None,
        "C++ AutoStore inventory-to-bank must select no quest check even though its target is Bank"
    );
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetLikeCpp::Inventory),
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    );
}
#[test]
fn legacy_zero_inventory_slots_loads_base_backpack_capacity() {
    assert_eq!(
        loaded_inventory_slot_count_with_legacy_rust_compat(0),
        INVENTORY_DEFAULT_SIZE
    );
    assert_eq!(loaded_inventory_slot_count_with_legacy_rust_compat(24), 24);
}
#[test]
fn inventory_move_quest_checks_only_cross_bank_boundary_like_cpp() {
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, false),
        "ordinary and child inventory relocations must not re-credit quest items"
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, true)
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Bank,
        ),
        (true, false)
    );
}
#[test]
fn mainhand_bank_remove_clears_and_persists_weapon_only_enchant_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    attach_stat_update_player_with_mana(&mut session, ObjectGuid::create_player(1, 42), 0, 0);
    install_bank_move_item_fixture(&mut session, 707, 1);
    let item_guid =
        insert_bank_move_test_item(&mut session, EQUIPMENT_SLOT_MAINHAND, 707, 7_071, 1);
    let enchantment_entry = |id, flags| wow_data::SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
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
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED);
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 930, 4_000, 2);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 931, 3_000, 1);
        item.set_enchantment(EnchantmentSlot::Property0, 932, 2_000, 3);
        item.set_enchantment(EnchantmentSlot::Property1, 999, 1_000, 4);
    });
    let mut timed_item = session
        .resolved_inventory_item_object_like_cpp(item_guid)
        .expect("canonical Player inventory item");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut timed_item,
                EnchantmentSlot::EnhancementTemporary,
                1_500,
            )
        })
        .expect("canonical player");

    let (persisted, cleared) = session
        .inventory_remove_enchantment_persistence_like_cpp(item_guid, true)
        .expect("main-hand-only enchantment");
    assert_eq!(cleared, vec![EnchantmentSlot::EnhancementPermanent]);
    let fields: Vec<_> = persisted.split_whitespace().collect();
    assert_eq!(&fields[0..3], &["0", "0", "0"]);
    assert_eq!(&fields[3..6], &["931", "1500", "1"]);
    assert_eq!(&fields[24..27], &["0", "0", "0"]);
    assert_eq!(&fields[27..30], &["0", "0", "0"]);

    let _ = session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        item_guid,
        &cleared,
    );
    let item = session
        .resolved_inventory_item_object_like_cpp(item_guid)
        .expect("canonical Player inventory item");
    assert!(!item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        0
    );
    let update =
        WorldSession::item_storage_fields_values_update_like_cpp(&item, true, true, &cleared);
    let packet_update = crate::entity_update_bridge::item_values_update_to_packet(&update)
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
        session.represented_combat_stat_recalculations_like_cpp(),
        &[
            crate::session::RepresentedCombatStatRecalculationLikeCpp::Expertise {
                attack: wow_constants::WeaponAttackType::BaseAttack,
            },
            crate::session::RepresentedCombatStatRecalculationLikeCpp::Rating { combat_rating: 24 },
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
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session.represented_non_bank_item_count_like_cpp(703),
        Some(4)
    );
    assert!(session.apply_committed_inventory_item_relocation_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        INVENTORY_SLOT_BAG_0,
        wow_entities::BANK_SLOT_ITEM_START,
        4,
    ));
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session.represented_non_bank_item_count_like_cpp(703),
        Some(0)
    );
}
#[tokio::test]
async fn banker_activate_removes_feign_after_validation_before_open_like_cpp() {
    const FEIGN_SLOT: u8 = 17;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 37);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session.handle_banker_activate(Hello { unit: banker }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::NpcInteractionOpenResult,
        ],
        "C++ removes feign death before SendShowBank"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}
#[tokio::test]
async fn banker_activate_invalid_source_preserves_feign_and_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 18;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let invalid_banker =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 38);
    let active_source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 39);
    insert_banker_creature(&canonical, invalid_banker, NPCFlags1::VENDOR.bits());
    session.set_player_trainer_interaction_like_cpp(active_source, 77);
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_banker_activate(Hello {
            unit: invalid_banker,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(canonical_player_has_died_state_like_cpp(&mut session));
    assert!(
        session.player_trainer_interaction_matches_like_cpp(active_source, 77),
        "invalid banker must return before fake-death removal and SendShowBank"
    );
}
#[tokio::test]
async fn autobank_item_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 40);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 704, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 704, 7_041, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid),
        "runtime must not move when no character database can commit the plan"
    );
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn bank_authorization_reads_the_single_interaction_source_like_cpp() {
    let (mut session, _send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    session.set_player_alive_like_cpp(true);
    session.set_player_faction_template_like_cpp(1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 140);
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 141);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    insert_banker_creature(&canonical, vendor, NPCFlags1::VENDOR.bits());

    session.set_player_trainer_interaction_like_cpp(banker, 77);
    assert!(
        session.represented_can_use_current_bank_like_cpp(),
        "C++ CanUseBank reads SourceGuid and does not impose an interaction kind or TrainerId gate"
    );

    session.set_player_interaction_source_like_cpp(vendor);
    assert!(!session.represented_can_use_current_bank_like_cpp());

    session.set_player_interaction_source_like_cpp(banker);
    assert!(session.represented_can_use_current_bank_like_cpp());
    assert!(session.reset_player_interaction_if_source_like_cpp(banker));
    assert!(!session.represented_can_use_current_bank_like_cpp());
}
#[tokio::test]
async fn autobank_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 42);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 708, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 708, 7_081, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );
    assert!(session.represented_can_use_current_bank_like_cpp());
    let precommit_plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid bank destination");
    assert_eq!(
        precommit_plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 1))
    );

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid),
        "a failed SQL commit must not expose the planned bank location"
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,)
            .is_none()
    );
    assert_eq!(
        session.represented_non_bank_item_count_like_cpp(708),
        Some(1)
    );
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());

    let error = send_rx
        .try_recv()
        .expect("commit failure should send an equipment error");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        wow_constants::ServerOpcodes::InventoryChangeFailure as u16,
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn autostore_missing_bank_item_does_not_record_unapplied_move_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 41);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_autostore_bank_item(AutoStoreBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 39)],
            },
            bag: 255,
            slot: 39,
        })
        .await;

    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn auto_bank_item_rejects_without_current_bank_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(1);

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 19)],
            },
            bag: 255,
            slot: 19,
        })
        .await;
    session
        .handle_autostore_bank_item(AutoStoreBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 39)],
            },
            bag: 255,
            slot: 39,
        })
        .await;

    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}
