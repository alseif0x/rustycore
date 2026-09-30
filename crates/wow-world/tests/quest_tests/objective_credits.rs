//! Wiring at the canonical Player owner; packet delivery and persistence stay here.

use super::*;
use wow_world::test_fixtures::{
    PlayerQuestPersistencePortFixtureLikeCpp, insert_player_quest_gameplay_status_for_test,
    player_quest_status_requests_for_test,
};
use wow_world::test_fixtures::loot::{
    basic_quest_template_for_loot_test, make_session_with_send_capacity,
};
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_constants::quest::{
    QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, QUEST_OBJECTIVE_CURRENCY_LIKE_CPP,
    QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP, QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
    QUEST_OBJECTIVE_MONEY_LIKE_CPP, QUEST_STATUS_COMPLETE_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};
use wow_data::quest::{QuestObjective, QuestStore, QuestTemplate};
use wow_packet::ServerPacket;
use wow_packet::packets::quest::{QuestUpdateAddCredit, QuestUpdateAddCreditSimple};
use wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp;

fn objective(quest_id: u32, obj_type: u8, index: i8, amount: i32) -> QuestObjective {
    QuestObjective {
        id: quest_id * 10, quest_id, obj_type, order: 0, storage_index: index,
        object_id: 42, amount, flags: 0, flags2: 0, progress_bar_weight: 0.0,
        description: String::new(),
    }
}

fn credit_quest(obj_type: u8, index: i8, amount: i32) -> QuestTemplate {
    let mut quest = basic_quest_template_for_loot_test(7104);
    let credit = objective(quest.id, obj_type, index, amount);
    let mut blocker = objective(quest.id, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 1, 999);
    blocker.id += 1;
    blocker.object_id = 99;
    quest.objectives = vec![credit.clone(), credit, blocker];
    quest
}

fn make_owner(
    quest: QuestTemplate,
    status: u8,
    counts: Vec<i32>,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<PlayerQuestPersistencePortFixtureLikeCpp>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571, instance_type: wow_data::map::MAP_COMMON, expansion_id: 0,
            parent_map_id: -1, cosmetic_parent_map_id: -1, flags1: 0, flags2: 0,
        },
    ])));
    attach_player_controller_for_test(&mut session, 
        ObjectGuid::create_player(1, 7104), "QuestCreditOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0), 571, 1, 1, 80, 0,
    );
    ensure_world_map_for_current_player_for_test(&mut session).expect("world map");
    let quest_id = quest.id;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(&mut session, quest_id,
        wow_entities::PlayerQuestStatusRecord {
            quest_id, status, explored: false, accept_time_secs: 12, end_time_secs: 34,
            objective_counts: counts, slot: 0,
        },
    );
    let persistence = Arc::new(PlayerQuestPersistencePortFixtureLikeCpp::default());
    session.set_player_quest_persistence_port_like_cpp(persistence.clone());
    (session, send_rx, persistence, canonical)
}

#[tokio::test]
async fn duplicate_value_credits_write_and_publish_in_order_then_save_once() {
    let (mut session, packets, persistence, _) = make_owner(
        credit_quest(QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0, 2),
        QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0, 0],
    );
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    credit_quest_value_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, 1, ObjectGuid::EMPTY,
    ).await;
    for count in [1, 2] {
        assert_eq!(packets.try_recv().unwrap(), QuestUpdateAddCredit {
            victim_guid: ObjectGuid::EMPTY, quest_id: 7104, object_id: 42,
            count, required: 2, objective_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
        }.to_bytes());
    }
    assert!(packets.try_recv().is_err());
    let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
    assert_eq!(state.statuses_like_cpp()[&7104].objective_counts, vec![2, 0]);
    assert_eq!(state.statuses_like_cpp()[&7104].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let saved = player_quest_status_requests_for_test(&persistence);
    assert_eq!(saved.len(), 1, "the existing save phase deduplicates quest IDs after both writers");
    let PlayerQuestStatusPersistenceRequestLikeCpp::Save { owner_guid, status } = &saved[0] else {
        panic!("credit saves the active status");
    };
    assert_eq!(*owner_guid, 7104);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(status.objectives.iter().map(|row| (row.objective_index, row.count))
        .collect::<Vec<_>>(), vec![(0, 2), (0, 2)],
        "the existing persistence projection keeps duplicate definitions and omits zero counts");
}

#[tokio::test]
async fn capped_value_noops_still_enter_writer_and_resize_without_save_or_packet() {
    let (mut session, packets, persistence, _) = make_owner(
        credit_quest(QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 3, 0),
        QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![],
    );
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    credit_quest_value_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, 1, ObjectGuid::EMPTY,
    ).await;
    assert_eq!(player_quest_gameplay_snapshot_for_test(&session).unwrap()
        .statuses_like_cpp()[&7104].objective_counts, vec![0, 0, 0, 0]);
    assert!(player_quest_status_requests_for_test(&persistence).is_empty());
    assert!(packets.try_recv().is_err());
}

#[tokio::test]
async fn value_credit_keeps_saturation_and_existing_u16_publication_casts() {
    let (mut session, packets, persistence, _) = make_owner(
        credit_quest(QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0, i32::MAX),
        QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![i32::MAX - 1, 0],
    );
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    credit_quest_value_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, 100, ObjectGuid::EMPTY,
    ).await;
    assert_eq!(player_quest_gameplay_snapshot_for_test(&session).unwrap()
        .statuses_like_cpp()[&7104].objective_counts, vec![i32::MAX, 0]);
    assert_eq!(packets.try_recv().unwrap(), QuestUpdateAddCredit {
        victim_guid: ObjectGuid::EMPTY, quest_id: 7104, object_id: 42,
        count: i32::MAX as u16, required: i32::MAX as u16,
        objective_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP,
    }.to_bytes());
    assert!(packets.try_recv().is_err(), "the duplicate's capped writer returns no credit");
    assert_eq!(player_quest_status_requests_for_test(&persistence).len(), 1);
}

#[tokio::test]
async fn duplicate_flag_credits_publish_when_already_set_and_save_only_on_change() {
    let (mut session, packets, persistence, _) = make_owner(
        credit_quest(QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 0, 1),
        QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0, 0],
    );
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    for _ in 0..2 {
        credit_quest_flag_for_test(&mut session, 
            &generator, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 42, 1,
        ).await;
        for _ in 0..2 {
            assert_eq!(packets.try_recv().unwrap(), QuestUpdateAddCreditSimple {
                quest_id: 7104, object_id: 42, objective_type: QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP,
            }.to_bytes());
        }
        assert!(packets.try_recv().is_err());
        assert_eq!(player_quest_status_requests_for_test(&persistence).len(), 1);
    }
    assert_eq!(player_quest_gameplay_snapshot_for_test(&session).unwrap()
        .statuses_like_cpp()[&7104].objective_counts, vec![1, 0]);
    credit_quest_flag_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 42, 0,
    ).await;
    assert_eq!(player_quest_gameplay_snapshot_for_test(&session).unwrap()
        .statuses_like_cpp()[&7104].objective_counts, vec![0, 0]);
    assert_eq!(player_quest_status_requests_for_test(&persistence).len(), 2);
    assert!(packets.try_recv().is_err());
}

#[tokio::test]
async fn money_threshold_loss_reopens_complete_status_once_without_counter_write_or_packet() {
    let mut quest = basic_quest_template_for_loot_test(7104);
    let mut money = objective(quest.id, QUEST_OBJECTIVE_MONEY_LIKE_CPP, -1, 100);
    money.object_id = 0;
    quest.objectives = vec![money.clone(), money];
    let (mut session, packets, persistence, _) = make_owner(
        quest, QUEST_STATUS_COMPLETE_LIKE_CPP, vec![7, 8],
    );
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    update_quest_money_objectives_for_test(&mut session, &generator, 100, 50).await;
    update_quest_money_objectives_for_test(&mut session, &generator, 100, 50).await;
    let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
    assert_eq!(state.statuses_like_cpp()[&7104].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(state.statuses_like_cpp()[&7104].objective_counts, vec![7, 8]);
    assert_eq!(state.statuses_like_cpp()[&7104].accept_time_secs, 12);
    assert_eq!(state.statuses_like_cpp()[&7104].end_time_secs, 34);
    assert_eq!(player_quest_status_requests_for_test(&persistence).len(), 1);
    assert!(packets.try_recv().is_err());
}

#[tokio::test]
async fn currency_and_reputation_threshold_losses_use_canonical_inputs_and_reopen_once() {
    for objective_type in [QUEST_OBJECTIVE_CURRENCY_LIKE_CPP, QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP] {
        let mut quest = basic_quest_template_for_loot_test(7104);
        quest.objectives = vec![objective(quest.id, objective_type, -1, 100)];
        let (mut session, packets, persistence, _) = make_owner(
            quest, QUEST_STATUS_COMPLETE_LIKE_CPP, vec![7, 8],
        );
        if objective_type == QUEST_OBJECTIVE_CURRENCY_LIKE_CPP {
            assert!(install_player_currencies_for_test(&mut session, std::collections::HashMap::from([
                (42, wow_entities::PlayerCurrency {
                    state: wow_entities::PlayerCurrencyState::Unchanged,
                    quantity: 100, weekly_quantity: 0, tracked_quantity: 0,
                    increased_cap_quantity: 0, earned_quantity: 0, flags: 0,
                }),
            ])));
        } else {
            session.set_faction_store(Arc::new(wow_data::progression_rewards::FactionStore::from_entries([
                wow_data_model::reputation::FactionEntry::for_test_like_cpp(42, 5),
            ])));
            mutate_quest_reputation_for_test(&mut session, |mgr| {
                mgr.get_state_mut(5).expect("canonical faction state").standing = 100;
            }).expect("canonical reputation owner");
        }
        let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
        for _ in 0..2 {
            if objective_type == QUEST_OBJECTIVE_CURRENCY_LIKE_CPP {
                update_quest_currency_objectives_for_test(&mut session, 
                    &generator, 42, -60,
                ).await;
            } else {
                update_quest_reputation_objectives_for_test(&mut session, 
                    &generator, objective_type, 42, -60,
                ).await;
            }
        }
        let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
        assert_eq!(state.statuses_like_cpp()[&7104].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert_eq!(state.statuses_like_cpp()[&7104].objective_counts, vec![7, 8]);
        assert_eq!(player_quest_status_requests_for_test(&persistence).len(), 1);
        assert!(packets.try_recv().is_err());
    }
}

#[tokio::test]
async fn stale_canonical_owner_cannot_credit_fixture_or_replacement_player() {
    let (mut session, packets, persistence, canonical) = make_owner(
        credit_quest(QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 0, 2),
        QUEST_STATUS_INCOMPLETE_LIKE_CPP, vec![0, 0],
    );
    let before = player_quest_gameplay_snapshot_for_test(&session).unwrap();
    let fixture_before = quest_compatibility_statuses_for_test(&session);
    let old_handle = quest_player_handle_for_test(&session).unwrap();
    let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(ObjectGuid::create_player(1, 7104));
    replacement.gameplay_state_mut().quests = before.clone();
    let new_handle = canonical.lock().unwrap()
        .install_detached_player_like_cpp(replacement).expect("replacement owner");
    assert_ne!(new_handle, old_handle);
    assert!(player_quest_gameplay_snapshot_for_test(&session).is_none());
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    credit_quest_value_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_MONSTER_LIKE_CPP, 42, 1, ObjectGuid::EMPTY,
    ).await;
    credit_quest_flag_for_test(&mut session, 
        &generator, QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, 42, 1,
    ).await;
    update_quest_money_objectives_for_test(&mut session, &generator, 100, 50).await;
    assert_eq!(quest_compatibility_statuses_for_test(&session), fixture_before);
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(new_handle,
        |player| player.gameplay_state().quests.statuses_snapshot_like_cpp()),
        Some(before.statuses_snapshot_like_cpp()));
    assert!(player_quest_status_requests_for_test(&persistence).is_empty());
    assert!(packets.try_recv().is_err());
}
