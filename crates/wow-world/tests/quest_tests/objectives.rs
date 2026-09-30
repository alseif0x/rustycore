//! Quest objective mutation through the handler owner.

use super::*;

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

    let planned = plan_bank_item_quest_persistence_for_test(
        &session,
        item_id as u32,
        0,
        false,
        1,
        1,
    );
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    let planned_quest_id = planned[0].quest_id;

    let changed = apply_quest_item_added_objective_progress_for_test(
        &mut session,
        item_id as u32,
        0,
        1,
    )
    .await;
    assert_eq!(changed, vec![planned_quest_id]);
    assert_eq!(
        player_quest_statuses_for_test(&session)
            .into_iter()
            .flat_map(|status| status.objective_counts)
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

    let planned_statuses = plan_quest_source_item_bound_objective_statuses_for_test(
        &session,
        item_id as u32,
        0,
        1,
    )
        .expect("one bound objective should be planned");
    assert_eq!(planned_statuses.len(), 1);
    assert_eq!(planned_statuses[0].quest_id, early_slot_quest_id);

    let applied = apply_quest_source_item_bound_objective_progress_for_object_for_test(
        &mut session,
        quest_store.as_ref(),
        item_id,
        1,
    )
    .await;
    assert_eq!(applied, vec![(early_slot_quest_id, 1)]);
    assert_eq!(
        player_quest_status_for_test(&session, early_slot_quest_id)
            .expect("early-slot quest status should exist")
            .objective_counts,
        vec![1]
    );
    assert!(
        player_quest_status_for_test(&session, late_slot_quest_id)
            .expect("late-slot quest status should exist")
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

    let changed = apply_quest_item_added_objective_progress_for_test(
        &mut session,
        item_id as u32,
        0,
        1,
    )
    .await;

    assert_eq!(changed, vec![quest_id]);
    assert_eq!(
        player_quest_status_for_test(&session, quest_id)
            .expect("quest status should exist")
            .objective_counts,
        vec![1]
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ suppresses QuestUpdateAddCredit for ITEM objectives"
    );
}

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
        quest_source_item_quest_log_item_id_for_test(&mut session, 1001).await,
        9101
    );
    assert_eq!(
        quest_source_item_quest_log_item_id_for_test(&mut session, 1001).await,
        9101
    );
    assert_eq!(
        quest_source_item_quest_log_item_id_for_test(&mut session, 1002).await,
        0
    );
    assert_eq!(
        quest_source_item_quest_log_item_id_for_test(&mut session, 1002).await,
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

struct ItemTemplateAddonCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<ItemTemplateAddonCatalogRequestLikeCpp>>,
    outcomes:
        std::sync::Mutex<std::collections::VecDeque<ItemTemplateAddonLootMetadataOutcomeLikeCpp>>,
}

impl ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = ItemTemplateAddonLootMetadataOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }
}

impl ItemTemplateAddonCatalogPersistencePortLikeCpp for ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        _request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp> {
        panic!("quest source-item lookup never requests item-addon money")
    }

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one item-addon metadata outcome per uncached request");
        Box::pin(async move { outcome })
    }
}
