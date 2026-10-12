//! #1363 slice 2 / #1359: focal currency, reputation and tracking-event drains
//! through the App quest-objective owner
//! (`wow-world-application/src/quest/objectives.rs`), which is now the only
//! implementation of C++ `Player::UpdateQuestObjectiveProgress` /
//! `Player::CompleteQuest` (`Player.cpp:16162`–`:16176`, `:14496`).

use super::*;

const FACTION_ID: u32 = 7;
const REP_LIST_ID: u32 = 5;
const CURRENCY_ID: u32 = 395;

fn objective_like_cpp(
    quest_id: u32,
    obj_type: u8,
    object_id: u32,
    amount: i32,
) -> wow_data::quest::QuestObjective {
    wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type,
        order: 0,
        storage_index: 0,
        object_id: object_id as i32,
        amount,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }
}

/// A session with the given quests accepted (INCOMPLETE, one zeroed counter),
/// a currency and a visible reputation faction.
fn drain_session_like_cpp(
    quests: Vec<wow_data::quest::QuestTemplate>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let mut faction = FactionEntry::for_test_like_cpp(FACTION_ID, REP_LIST_ID as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(CURRENCY_ID),
    ])));
    let ids = quests.iter().map(|quest| quest.id).collect::<Vec<_>>();
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        quests,
    )));
    for (slot, quest_id) in ids.into_iter().enumerate() {
        session
            .quest_state
            .fixture_insert_player_quest_status_like_cpp(
                quest_id,
                crate::handlers::quest::PlayerQuestStatus {
                    quest_id,
                    status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: vec![0],
                    slot: slot as u8,
                },
            );
    }
    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    let _ = drain_server_opcodes(&send_rx);
    (session, send_rx)
}

fn objective_count_like_cpp(session: &WorldSession, quest_id: u32) -> Option<i32> {
    session
        .player_quest_gameplay_snapshot_like_cpp()?
        .statuses_like_cpp()
        .get(&quest_id)?
        .objective_counts
        .first()
        .copied()
}

async fn drain_events_like_cpp(
    session: &mut WorldSession,
    events: &[RepresentedQuestObjectiveProgressEventLikeCpp],
) {
    for event in events {
        session
            .quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(*event);
    }
    session
        .drain_represented_quest_objective_progress_like_cpp()
        .await;
}

#[tokio::test]
async fn app_owner_drain_applies_currency_and_reputation_events_in_recorded_order_like_cpp() {
    let currency_quest = 13_631;
    let reputation_quest = 13_632;
    let mut obtain = test_quest_template(currency_quest);
    obtain.objectives.push(objective_like_cpp(
        currency_quest,
        QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
        CURRENCY_ID,
        100,
    ));
    let mut increase = test_quest_template(reputation_quest);
    increase.objectives.push(objective_like_cpp(
        reputation_quest,
        QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
        FACTION_ID,
        100,
    ));
    let (mut session, send_rx) = drain_session_like_cpp(vec![obtain, increase]);

    drain_events_like_cpp(
        &mut session,
        &[
            RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                faction_id: FACTION_ID,
                change: 30,
            },
            RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                currency_id: CURRENCY_ID,
                change: 40,
            },
        ],
    )
    .await;

    // Both partial credits are stored and published, reputation first because
    // it was recorded first; neither quest completes below its amount.
    assert_eq!(
        objective_count_like_cpp(&session, reputation_quest),
        Some(30)
    );
    assert_eq!(objective_count_like_cpp(&session, currency_quest), Some(40));
    let credits = drain_server_packet_bytes(&send_rx)
        .iter()
        .map(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            assert_eq!(
                packet.read_uint16().unwrap(),
                ServerOpcodes::QuestUpdateAddCredit as u16
            );
            let _victim = packet.read_packed_guid().unwrap();
            packet.read_uint32().unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(credits, vec![reputation_quest, currency_quest]);
    assert_canonical_quest_status_like_cpp(
        &session,
        currency_quest,
        Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        false,
    );
    assert_canonical_quest_status_like_cpp(
        &session,
        reputation_quest,
        Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        false,
    );
    assert!(
        session
            .quest_state
            .represented_quest_complete_status_updates_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn app_owner_drain_completes_a_non_tracking_currency_quest_without_reward_like_cpp() {
    let quest_id = 13_633;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(objective_like_cpp(
        quest_id,
        QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
        CURRENCY_ID,
        100,
    ));
    let (mut session, send_rx) = drain_session_like_cpp(vec![quest]);

    drain_events_like_cpp(
        &mut session,
        &[
            RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                currency_id: CURRENCY_ID,
                change: 100,
            },
        ],
    )
    .await;

    assert_canonical_quest_status_like_cpp(
        &session,
        quest_id,
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP),
        false,
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
    let updates = session
        .quest_state
        .represented_quest_complete_status_updates_like_cpp();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].quest_id, quest_id);
    assert_eq!(
        updates[0].old_status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert!(!updates[0].tracking_event_auto_reward_unrepresented);
}

#[tokio::test]
async fn app_owner_drain_auto_rewards_a_tracking_event_reputation_quest_once_like_cpp() {
    let quest_id = 13_634;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(objective_like_cpp(
        quest_id,
        QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
        FACTION_ID,
        100,
    ));
    let (mut session, send_rx) = drain_session_like_cpp(vec![quest]);

    drain_events_like_cpp(
        &mut session,
        &[
            RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                faction_id: FACTION_ID,
                change: 100,
            },
        ],
    )
    .await;

    // `CompleteQuest` (`Player.cpp:14496`) rewards a tracking event at once:
    // the quest leaves the log as rewarded and the App owner marks the
    // completion evidence as auto-rewarded.
    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    let updates = session
        .quest_state
        .represented_quest_complete_status_updates_like_cpp();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].quest_id, quest_id);
    assert!(!updates[0].tracking_event_auto_reward_unrepresented);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::QuestGiverQuestComplete)
            .count(),
        1,
        "{opcodes:?}"
    );

    // The drain is finished: a second one with no recorded event is a no-op.
    session
        .drain_represented_quest_objective_progress_like_cpp()
        .await;
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert_eq!(
        session
            .quest_state
            .represented_quest_complete_status_updates_like_cpp()
            .len(),
        1
    );
}
