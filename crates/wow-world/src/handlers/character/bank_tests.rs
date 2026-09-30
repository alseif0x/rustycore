use crate::handlers::character::bank_test_support::{
    insert_binder_creature, make_bank_slot_session,
};
use crate::session::{AuraApplication, InventoryItem, RepresentedAuraEffectLikeCpp, WorldSession};
use crate::test_fixtures::{
    get_inventory_item_by_pos_for_test, insert_inventory_item_for_test,
    insert_inventory_item_object_for_test, make_inventory_item_object_for_test,
    player_gold_for_test, player_interaction_source_guid_for_test,
    player_interaction_trainer_id_for_test, set_failing_player_inventory_persistence_port_for_test,
    set_player_trainer_interaction_for_test, CollectionLoadPortLikeCpp,
};
use wow_constants::unit::NPCFlags1;
use wow_constants::{ItemBondingType, ItemClass, ItemContext, InventoryType, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::ObjectGuid;
use std::sync::{Arc, Mutex};
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::item::InvUpdate;
use wow_packet::packets::misc::{
    AutoBankItem, AutoStoreBankItem, BuyBankSlot, ChangeBankBagSlotFlag,
};
use wow_packet::WorldPacket;
use wow_data::item::stats::{ItemSparseTemplateEntry, ItemStatsStore};
use wow_data::item::ItemRecord;

fn install_bank_move_item_fixture(session: &mut WorldSession, entry_id: u32, max_stack_size: i32) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([ItemRecord {
        id: entry_id,
        class_id: ItemClass::Miscellaneous as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry_id,
        ItemSparseTemplateEntry {
            flags: [0; 4],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0; 2],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
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

fn insert_bank_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(wow_core::Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn seed_bank_test_feign_death(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
        })
        .expect("canonical player");
    session.visible_auras.insert(
        slot,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

fn bank_test_player_has_died_state(session: &mut WorldSession) -> bool {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

fn drain_bank_test_server_opcodes(
    send_rx: &flume::Receiver<Vec<u8>>,
) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

#[tokio::test]
async fn buy_bank_slot_buys_next_slot_and_spends_money_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 1);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 1);
    assert_eq!(player_gold_for_test(&session), 50);
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
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
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
    assert_eq!(player_gold_for_test(&session), 150);
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
    insert_binder_creature(&canonical, creature, NPCFlags1::QUEST_GIVER.bits());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: creature })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(player_gold_for_test(&session), 150);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn buy_bank_slot_rejects_missing_price_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 3);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_bank_bag_slot_count_like_cpp(2);

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 2);
    assert_eq!(player_gold_for_test(&session), 150);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn autostore_missing_bank_item_does_not_record_unapplied_move_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 41);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());

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

    // Both positions have no source item, so this preserves the no-op scenario but does not
    // independently prove the preceding CanUseBank guard.
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

#[tokio::test]
async fn change_bank_bag_slot_flag_toggles_flag_after_banker_activation_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(16));
    assert!(send_rx.try_recv().is_ok(), "flag update should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: false,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(
        send_rx.try_recv().is_ok(),
        "flag clear update should be sent"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_without_current_bank_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(1);

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_invalid_slot_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 5);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 7,
            flag: 4,
            enabled: true,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(6), Some(0));
}

#[tokio::test]
async fn banker_activate_removes_feign_after_validation_before_open_like_cpp() {
    const FEIGN_SLOT: u8 = 17;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 37);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    set_player_trainer_interaction_for_test(&mut session, banker, 77);
    seed_bank_test_feign_death(&mut session, FEIGN_SLOT);

    session.handle_banker_activate(Hello { unit: banker }).await;

    assert_eq!(
        drain_bank_test_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::NpcInteractionOpenResult,
        ],
        "C++ removes feign death before SendShowBank"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!bank_test_player_has_died_state(&mut session));
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(banker)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
}

#[tokio::test]
async fn banker_activate_invalid_source_preserves_feign_and_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 18;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let invalid_banker =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 38);
    let active_source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 39);
    insert_binder_creature(&canonical, invalid_banker, NPCFlags1::VENDOR.bits());
    set_player_trainer_interaction_for_test(&mut session, active_source, 77);
    seed_bank_test_feign_death(&mut session, FEIGN_SLOT);

    session
        .handle_banker_activate(Hello {
            unit: invalid_banker,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(bank_test_player_has_died_state(&mut session));
    assert!(
        session.player_trainer_interaction_matches_like_cpp(active_source, 77),
        "invalid banker must return before fake-death removal and SendShowBank"
    );
}

#[tokio::test]
async fn autobank_item_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 40);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 704, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 704, 7_041, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(banker)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(wow_entities::INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: wow_entities::INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            wow_entities::INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid),
        "runtime must not move when no character database can commit the plan"
    );
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn autobank_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 42);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 708, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 708, 7_081, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    set_failing_player_inventory_persistence_port_for_test(&mut session);
    assert!(session.represented_can_use_current_bank_like_cpp());
    let precommit_plan = session
        .plan_inventory_storage_move_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            wow_entities::NULL_BAG,
            wow_entities::NULL_SLOT,
            crate::handlers::character::inventory_plan::InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid bank destination");
    assert_eq!(
        precommit_plan.moved_destination,
        Some((
            wow_entities::INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            1,
        ))
    );

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(wow_entities::INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: wow_entities::INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            wow_entities::INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid),
        "a failed SQL commit must not expose the planned bank location"
    );
    assert!(get_inventory_item_by_pos_for_test(
        &session,
        wow_entities::INVENTORY_SLOT_BAG_0,
        wow_entities::BANK_SLOT_ITEM_START,
    )
    .is_none());
    assert_eq!(session.represented_non_bank_item_count_like_cpp(708), Some(1));
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());

    let error = send_rx
        .try_recv()
        .expect("commit failure should send an equipment error");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        ServerOpcodes::InventoryChangeFailure as u16,
    );
    assert!(send_rx.try_recv().is_err());
}
