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
    install_native_quest_log_player_like_cpp(&mut session);
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
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
async fn native_quest_log_reward_reaches_the_database_once_with_its_status_row_like_cpp() {
    let quest_id = 7101;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));

    choose_reward_like_cpp(&mut session, quest_id).await;

    let quests = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest state");
    assert!(
        !quests.statuses_like_cpp().contains_key(&quest_id),
        "the operation must have run to completion before it commits"
    );
    assert!(
        session
            .quest_log_create_entries_like_cpp()
            .iter()
            .all(|entry| entry.0 == 0),
        "RewardQuest's explicit SetQuestSlot(0) clears the native row after commit"
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
async fn native_quest_log_rolled_back_reward_retains_its_row_like_cpp() {
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
    assert_eq!(
        session.quest_log_create_entries_like_cpp()[0].0,
        quest_id,
        "a rolled-back RewardQuest retains the native QuestLog row"
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

/// Records each `player.quest_rewarded` with the commits already requested
/// and whether the canonical map lock was free.
struct RewardProbe {
    requests: Arc<std::sync::Mutex<Vec<wow_persistence::PlayerQuestRewardDurableRequestLikeCpp>>>,
    map: Option<crate::session::SharedCanonicalMapManager>,
    calls: std::sync::Mutex<Vec<(u32, u32, usize, bool)>>,
}

impl wow_module_api::PlayerModule for RewardProbe {
    fn on_quest_rewarded(
        &self,
        snapshot: &wow_module_api::QuestRewardedSnapshot,
        effects: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        let map_free = self.map.as_ref().is_none_or(|map| map.try_lock().is_ok());
        self.calls.lock().unwrap().push((
            snapshot.quest_id,
            snapshot.choice_item_id,
            self.requests.lock().unwrap().len(),
            map_free,
        ));
        effects.send_system_message_self("Quest done!");
    }
}

fn install_reward_probe(
    session: &mut WorldSession,
    requests: &Arc<std::sync::Mutex<Vec<wow_persistence::PlayerQuestRewardDurableRequestLikeCpp>>>,
) -> Arc<RewardProbe> {
    let probe = Arc::new(RewardProbe {
        requests: Arc::clone(requests),
        map: session.core.canonical_map_manager.clone(),
        calls: std::sync::Mutex::new(Vec::new()),
    });
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.reward",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Reward",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

/// `player.quest_rewarded` (#1444 H1): the last step of AzerothCore
/// `RewardQuest`, `OnPlayerCompleteQuest` (`PlayerQuest.cpp:903`). Once, after
/// the single reward transaction was committed.
#[tokio::test]
async fn quest_rewarded_hook_runs_once_after_the_reward_commit_like_cpp() {
    let quest_id = 7105;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));
    let probe = install_reward_probe(&mut session, &requests);

    choose_reward_like_cpp(&mut session, quest_id).await;

    let calls = probe.calls.lock().unwrap().clone();
    assert_eq!(calls, vec![(quest_id, 0, 1, true)]);
}

#[tokio::test]
async fn a_rolled_back_reward_raises_no_quest_rewarded_hook_like_cpp() {
    let quest_id = 7106;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::with_outcome(
        PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "deadlock".to_string(),
        },
    );
    let requests = Arc::clone(&fixture.requests);
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));
    let probe = install_reward_probe(&mut session, &requests);

    choose_reward_like_cpp(&mut session, quest_id).await;

    assert!(probe.calls.lock().unwrap().is_empty());
}

/// Records the order of the reward's `player.*` hooks.
struct RewardOrderProbe {
    seen: std::sync::Mutex<Vec<String>>,
}

impl wow_module_api::PlayerModule for RewardOrderProbe {
    fn on_money_changed(
        &self,
        s: &wow_module_api::PlayerMoneyChangedSnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("money {} {}", s.old_money, s.new_money));
    }
    fn on_quest_rewarded(
        &self,
        s: &wow_module_api::QuestRewardedSnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "rewarded {} {} {}",
            s.quest_id, s.money_before, s.money_after
        ));
    }
}

/// AzerothCore `RewardQuest`: `ModifyMoney` (`OnPlayerMoneyChanged`) before
/// `OnPlayerCompleteQuest` (`PlayerQuest.cpp:903`). The reward reports its
/// committed money change, then itself with the committed balances, and the
/// later objective drain does not report the money again (#1444 review F3/F4).
#[tokio::test]
async fn a_money_reward_reports_money_then_the_reward_with_committed_balances_like_cpp() {
    let quest_id = 7107;
    let (mut session, _send_rx) = complete_quest_session_like_cpp(quest_id, 0);
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 250;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_player_gold_like_cpp(100);
    let fixture = PlayerQuestRewardPersistencePortFixtureLikeCpp::default();
    session.set_player_quest_reward_persistence_port_like_cpp(Arc::new(fixture));
    let probe = Arc::new(RewardOrderProbe {
        seen: std::sync::Mutex::new(Vec::new()),
    });
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.order",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Order",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));

    choose_reward_like_cpp(&mut session, quest_id).await;

    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![
            "money 100 350".to_owned(),
            format!("rewarded {quest_id} 100 350")
        ]
    );
}
