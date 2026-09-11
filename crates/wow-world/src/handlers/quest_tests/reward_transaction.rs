//! The quest-reward operation's single durable transaction, for [`super`].
//!
//! C++ closes `Player::RewardQuest` with one `SaveToDB(false)`
//! (Player.cpp:14867). These scenarios prove the operation reaches the
//! database exactly once and that a reward is never durable in parts.

use super::*;

use crate::player::quest_persistence_test_fixture::PlayerQuestRewardPersistencePortFixtureLikeCpp;
use wow_persistence::{
    PlayerQuestRewardCommitOutcomeLikeCpp, PlayerQuestRewardCommitWitnessLikeCpp,
    PlayerQuestStatusPersistenceRequestLikeCpp,
};

/// C++ `QUEST_SPECIAL_FLAGS_REPEATABLE`, the flag `Quest::IsRepeatable` reads.
const REPEATABLE_SPECIAL_FLAG_LIKE_CPP: u32 = 0x0000_0001;

/// A complete quest whose reward grants no money, so the operation's commit
/// witness is the quest-status row rather than the money column.
fn complete_quest_session_like_cpp(
    quest_id: u32,
    special_flags: u32,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.special_flags |= special_flags;
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
    (session, send_rx)
}

async fn choose_reward_like_cpp(session: &mut WorldSession, quest_id: u32) {
    let player_guid = session.player_guid().unwrap();
    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;
}

#[tokio::test]
async fn quest_reward_reaches_the_database_once_with_its_status_row_like_cpp() {
    let quest_id = 7101;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));

    choose_reward_like_cpp(&mut session, quest_id).await;

    assert!(
        !session.player_quests.contains_key(&quest_id),
        "the operation must have run to completion before it commits"
    );
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "the operation must commit exactly one character transaction"
    );
    let request = &requests[0];
    assert_eq!(request.quest_id, quest_id);
    match &request.quest_status {
        PlayerQuestStatusPersistenceRequestLikeCpp::Save { status, .. } => {
            assert_eq!(status.quest_id, quest_id);
            assert_eq!(status.status, QUEST_STATUS_REWARDED_LIKE_CPP);
        }
        other => panic!("a non-repeatable reward saves a rewarded row, got {other:?}"),
    }
}

#[tokio::test]
async fn a_rolled_back_quest_reward_transaction_is_not_reported_as_rewarded_like_cpp() {
    let quest_id = 7102;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::with_outcome(
        PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "deadlock".to_string(),
        },
    );
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));

    choose_reward_like_cpp(&mut session, quest_id).await;

    assert_eq!(
        requests.lock().unwrap().len(),
        1,
        "the operation still attempts exactly one transaction"
    );
    assert!(
        !session.represented_can_delay_teleport_like_cpp(),
        "the aborted reward must clear the delayed-teleport guard it took"
    );
}

#[tokio::test]
async fn a_lost_commit_reply_is_settled_by_the_durable_quest_status_like_cpp() {
    for (observed, expect_session_open) in [(Some(true), true), (Some(false), true), (None, false)]
    {
        let quest_id = 7103;
        let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
        let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::with_outcome(
            PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
                reason: "connection reset".to_string(),
                witness: PlayerQuestRewardCommitWitnessLikeCpp::QuestStatus {
                    observed_matches_request: observed,
                },
            },
        );
        session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));

        choose_reward_like_cpp(&mut session, quest_id).await;

        assert_eq!(
            session.state() != crate::session::SessionState::Disconnecting,
            expect_session_open,
            "an unobservable reward COMMIT must quarantine the session, observed = {observed:?}"
        );
    }
}

#[tokio::test]
async fn a_repeatable_quest_reward_deletes_its_status_row_in_the_same_transaction_like_cpp() {
    let quest_id = 7104;
    let (mut session, _send_rx) =
        complete_quest_session_like_cpp(quest_id, REPEATABLE_SPECIAL_FLAG_LIKE_CPP);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));

    choose_reward_like_cpp(&mut session, quest_id).await;

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].quest_status,
        PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
            owner_guid: 42,
            quest_id,
        }
    );
}
