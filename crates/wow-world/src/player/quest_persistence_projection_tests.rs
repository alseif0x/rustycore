//! Player quest-status persistence projection owner regression.

use super::*;
use std::sync::Arc;
use crate::test_fixtures::*;
use wow_constants::quest::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_data::quest::{
    QuestObjective, QuestStore, QuestTemplate, QUEST_ITEM_DROP_COUNT,
    QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT,
    QUEST_REWARD_REPUTATIONS_COUNT,
};

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let mut session = WorldSession::new(
        1,
        "QuestStatusTest".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    set_player_position_for_test(&mut session, Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    // Reward tests model successful persistence. Production composition
    // installs the typed ports; these narrow unit fixtures retain the
    // explicit no-I/O success seam for unrelated reward assertions.
    set_loot_money_persistence_test_result_for_test(&mut session, true);
    (session, send_rx)
}

fn quest_template(id: u32) -> QuestTemplate {
    crate::quest_template_fixture::quest_template_row_for_test(id, 2, format!("Quest {id}"))
}

#[test]
fn quest_status_projection_persists_only_nonzero_storage_objectives_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5925;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 2,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL,
        order: 2,
        storage_index: -1,
        object_id: 0,
        amount: 20,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: vec![3, 0, 9],
            slot: 0,
        },
    );

    let projected = session.represented_quest_status_persistence_like_cpp(
        &player_quest_status_for_test(&session, quest_id).unwrap(),
    );
    assert_eq!(projected.quest_id, quest_id);
    assert_eq!(projected.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(projected.explored);
    assert_eq!(projected.accept_time_secs, 12);
    assert_eq!(projected.end_time_secs, 34);
    assert_eq!(
        projected.objectives,
        vec![wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
            objective_index: 0,
            count: 3,
        }]
    );
}
