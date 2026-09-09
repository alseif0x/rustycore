//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_source_item_addon_port_preserves_lookup_and_zero_cache_like_cpp() {
    let port = ItemTemplateAddonCatalogPortFixtureLikeCpp::new([
        ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(
            wow_persistence::ItemTemplateAddonLootMetadataRowLikeCpp {
                flags_cu: 0x55,
                quest_log_item_id: 9101,
            },
        ),
        ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed {
            reason: "world read failed".into(),
        },
    ]);
    let (mut session, _send_rx) = make_session();
    session.set_item_template_addon_catalog_persistence_port_like_cpp(port.clone());

    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1001)
            .await,
        9101
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1001)
            .await,
        9101
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1002)
            .await,
        0
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1002)
            .await,
        0
    );
    assert_eq!(
        *port.requests.lock().unwrap(),
        [
            ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1001 },
            ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1002 },
        ]
    );
}
#[test]
fn aggregate_item_removal_plan_persists_all_objectives_in_one_status() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_400;
    let first_item_id = 19_900;
    let second_item_id = 19_901;
    let mut quest = quest_template(quest_id);
    quest.objectives = [first_item_id, second_item_id]
        .into_iter()
        .enumerate()
        .map(|(index, item_id)| QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: index as u8,
            storage_index: index as i8,
            object_id: item_id,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![1, 1];

    let removed_entries = [first_item_id as u32, second_item_id as u32];
    let planned = session.plan_item_transfer_quest_persistence_like_cpp(
        &removed_entries,
        &[(first_item_id as u32, 0), (second_item_id as u32, 0)],
        &[],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].quest_id, quest_id);
    assert_eq!(planned[0].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
}
#[test]
fn mixed_item_transfer_quest_plan_applies_withdrawal_after_deposit() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_403;
    let item_id = 19_903;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![1];

    let planned = session.plan_item_transfer_quest_persistence_like_cpp(
        &[item_id as u32],
        &[(item_id as u32, 0)],
        &[(item_id as u32, 0, 1)],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}
#[test]
fn quest_bound_withdrawal_plan_consumes_credit_without_physical_item() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_404;
    let item_id = 19_904;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot(&mut session, quest_id, 0);
    let mut plan = session.begin_item_transfer_quest_persistence_like_cpp(&[], &[]);

    assert!(
        session.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
            &mut plan,
            item_id as u32,
            0,
            1,
        )
    );
    let planned = session.finish_item_transfer_quest_persistence_like_cpp(plan);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}
#[tokio::test]
async fn bank_withdrawal_credits_only_first_matching_bound_item_objective_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let item_id = 19_901;
    let bound_quest = |quest_id: u32| {
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: item_id,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        }];
        quest
    };
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        bound_quest(7_401),
        bound_quest(7_402),
    ])));
    add_active_quest_in_slot(&mut session, 7_401, 0);
    add_active_quest_in_slot(&mut session, 7_402, 1);

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id as u32, 0, false, 1, 1);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    let planned_quest_id = planned[0].quest_id;

    let changed = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id as u32, 0, 1)
        .await;
    assert_eq!(changed, vec![planned_quest_id]);
    assert_eq!(
        session
            .player_quests
            .values()
            .flat_map(|status| status.objective_counts.iter())
            .copied()
            .sum::<i32>(),
        1,
        "C++ UpdateQuestObjectiveProgress breaks after the first credited quest-bound item objective"
    );

    let bytes = send_rx
        .try_recv()
        .expect("single quest-bound item objective should send ItemPushResult");
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn bound_item_durable_plan_and_apply_use_the_same_quest_log_order_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let item_id = 19_950;
    let early_slot_quest_id = 7_452;
    let late_slot_quest_id = 7_451;
    let bound_quest = |quest_id: u32| {
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: item_id,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        }];
        quest
    };
    let quest_store = Arc::new(QuestStore::from_quests_like_cpp([
        bound_quest(late_slot_quest_id),
        bound_quest(early_slot_quest_id),
    ]));
    session.set_quest_store(Arc::clone(&quest_store));
    // Insert the numerically smaller quest first, but give it the later
    // quest-log slot. HashMap bucket/insertion order must affect neither
    // the pre-SQL plan nor the post-COMMIT mutation.
    add_active_quest_in_slot(&mut session, late_slot_quest_id, 9);
    add_active_quest_in_slot(&mut session, early_slot_quest_id, 2);

    let planned = session
        .plan_quest_source_item_bound_objective_persistence_like_cpp(item_id as u32, 0, 1)
        .expect("one bound objective should be planned");
    assert_eq!(planned.statuses.len(), 1);
    assert_eq!(planned.statuses[0].quest_id, early_slot_quest_id);

    let applied = session
        .apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
            quest_store.as_ref(),
            item_id,
            1,
        )
        .await;
    assert_eq!(applied, vec![(early_slot_quest_id, 1)]);
    assert_eq!(
        session.player_quests[&early_slot_quest_id].objective_counts,
        vec![1]
    );
    assert!(
        session.player_quests[&late_slot_quest_id]
            .objective_counts
            .is_empty()
    );
}
#[tokio::test]
async fn bank_withdrawal_item_objective_never_sends_generic_credit_like_cpp() {
    let (mut session, send_rx) = make_session();
    let item_id = 19_902;
    let quest_id = 7_403;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest(&mut session, quest_id);

    let changed = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id as u32, 0, 1)
        .await;

    assert_eq!(changed, vec![quest_id]);
    assert_eq!(session.player_quests[&quest_id].objective_counts, vec![1]);
    assert!(
        send_rx.try_recv().is_err(),
        "C++ suppresses QuestUpdateAddCredit for ITEM objectives"
    );
}
#[test]
fn quest_giver_choose_reward_choice_parser_reads_cpp_wire_item_choice() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_uint32(7001);
    write_cpp_quest_choice_item_like_cpp(
        &mut pkt,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        3,
    );

    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint32().unwrap(), 7001);
    assert_eq!(
        WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap(),
        QuestChoiceItemLikeCpp {
            loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            item_id: 19019,
            quantity: 3,
        }
    );
    assert!(pkt.is_empty());
}
#[test]
fn quest_giver_choose_reward_choice_parser_skips_cpp_item_mods_and_bonus() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 392, 11, 22, &[(7, 1), (8, 2)], Some(&[91, 92]));
    pkt.write_int32(5);

    let choice = WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap();

    assert_eq!(
        choice,
        QuestChoiceItemLikeCpp {
            loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            item_id: 392,
            quantity: 5,
        }
    );
    assert!(pkt.is_empty());
}
#[tokio::test]
async fn quest_giver_choose_reward_rejects_missing_reward_item_template_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7003;
    let reward_item_id = 19019;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_removes_item_objective_before_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7015;
    let required_item_id = 19_028;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: required_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, required_item_id, 20, 0);
    insert_direct_inventory_item(&mut session, player_guid, 23, required_item_id, 5, 9911);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == required_item_id)
        .expect("partial objective item stack should remain");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .map(|item| item.count()),
        Some(3)
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_direct_choice_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7008;
    let reward_item_id = 19_022;
    let limit_category = 44;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9907);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7009;
    let reward_item_id = 19_023;
    let limit_category = 45;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 1;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9908);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7012;
    let reward_item_id = 19_026;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 2;
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("fixed reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(2)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 2);
                assert_eq!(packet.read_int32().unwrap(), 2);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}
#[tokio::test]
async fn quest_reward_item_definite_and_unknown_commit_fail_closed_before_publication_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "fixture rollback".into(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "fixture unknown commit".into(),
        },
    ] {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 70_120;
        let reward_item_id = 19_126;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_items[0] = reward_item_id;
        quest.reward_amounts[0] = 2;
        session.set_player_gold_like_cpp(5);
        install_source_item_template(&mut session, reward_item_id, 20, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let (port, requests) =
            PlayerInventoryPersistencePortFixtureLikeCpp::with_outcomes_like_cpp([
                PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                outcome,
            ]);
        session.set_player_inventory_persistence_port_like_cpp(port);

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert!(
            session
                .inventory_items_like_cpp()
                .values()
                .all(|item| item.entry_id != reward_item_id)
        );
        let requests = requests.lock().unwrap();
        assert!(matches!(
            requests.as_slice(),
            [
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(_),
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(_),
            ]
        ));
    }
}
#[tokio::test]
async fn quest_giver_choose_reward_chosen_item_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7013;
    let reward_item_id = 19_027;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 3);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("chosen reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(3)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 3);
                assert_eq!(packet.read_int32().unwrap(), 3);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}
