//! #1363 slice 2 / #1359: a focal drain through the App quest-objective owner,
//! the only implementation of C++ `Player::UpdateQuestObjectiveProgress`
//! (`Player.cpp:16162`–`:16176`): recorded events apply in order.

use super::*;

#[tokio::test]
async fn app_owner_drain_applies_reputation_then_currency_in_recorded_order_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    let mut quests = Vec::new();
    for (id, obj_type, object_id) in [
        (13_631, QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP, 395),
        (13_632, QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP, 7),
    ] {
        let mut quest = test_quest_template(id);
        quest.objectives.push(wow_data::quest::QuestObjective {
            id: id * 10,
            quest_id: id,
            obj_type,
            order: 0,
            storage_index: 0,
            object_id,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quests.push(quest);
        let status = crate::handlers::quest::PlayerQuestStatus {
            quest_id: id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: (id % 2) as u8,
        };
        let quest_state = &mut session.quest_state;
        quest_state.fixture_insert_player_quest_status_like_cpp(id, status);
    }
    let store = wow_data::quest::QuestStore::from_quests_like_cpp(quests);
    session.set_quest_store(Arc::new(store));
    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    let _ = drain_server_opcodes(&send_rx);

    session.reputation_changed_like_cpp(7, 30).await;
    session.currency_changed_like_cpp(395, 40).await;

    let state = session.player_quest_gameplay_snapshot_like_cpp().unwrap();
    let progress = |id| {
        state
            .statuses_like_cpp()
            .get(&id)
            .map(|s| (s.status, s.objective_counts[0]))
    };
    let incomplete = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
    assert_eq!(progress(13_632), Some((incomplete, 30)));
    assert_eq!(progress(13_631), Some((incomplete, 40)));
    let credited = drain_server_packet_bytes(&send_rx).iter().map(|bytes| {
        let mut packet = WorldPacket::from_bytes(bytes);
        let opcode = packet.read_uint16().unwrap();
        assert_eq!(opcode, ServerOpcodes::QuestUpdateAddCredit as u16);
        let _victim = packet.read_packed_guid().unwrap();
        packet.read_uint32().unwrap()
    });
    assert_eq!(credited.collect::<Vec<_>>(), vec![13_632, 13_631]);
    let updates = session
        .quest_state
        .represented_quest_complete_status_updates_like_cpp();
    assert!(updates.is_empty());
}
