//! Original loot quest application cases use the existing Quest fixtures.
use super::quest_support::*;
use super::support::*;
use wow_loot::LootConditionRowLikeCpp;

#[tokio::test]
async fn quest_required_creature_loot_is_not_generated_after_completion_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 8_336;
    let item_id = 20_482;
    let loot_id = 15_274;
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    install_limited_test_item_template(&mut session, item_id, 0);
    install_quest_bound_loot_objective_like_cpp(&mut session, quest_id, item_id, 6, 6);
    mutate_player_quest_gameplay_for_test(&mut session, |quests| {
        quests.statuses_mut_like_cpp().find(|status| status.quest_id == quest_id).unwrap().status = wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP;
    });

    let mut creature_store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: true,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature_store);
    session.set_loot_stores(Arc::new(stores));

    let complete_loot = generate_quest_loot_for_test(&mut session, loot_id, player_guid)
        .await
        .unwrap();
    assert!(
        complete_loot.is_empty(),
        "C++ LootItem::AllowedForPlayer rejects QuestRequired items after HasQuestForItem becomes false"
    );

    mutate_player_quest_gameplay_for_test(&mut session, |quests| {
        let status = quests.statuses_mut_like_cpp().find(|status| status.quest_id == quest_id).unwrap();
        status.status = wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
        status.objective_counts[0] = 5;
    });
    let incomplete_loot = generate_quest_loot_for_test(&mut session, loot_id, player_guid)
        .await
        .unwrap();
    assert_eq!(incomplete_loot.len(), 1);
    assert!(incomplete_loot[0].flags.needs_quest);
}

