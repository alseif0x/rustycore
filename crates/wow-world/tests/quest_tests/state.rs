//! Quest log state and slots at the handler owner.

use super::*;

#[tokio::test]
async fn quest_log_remove_duplicate_slot_fails_closed_and_removes_none_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    run_remove_quest_slot(&mut session, 2).await;

    assert!(contains_player_quest_status_for_test(&session, 5915));
    assert!(contains_player_quest_status_for_test(&session, 5916));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 2), None);
    assert_eq!(first_free_quest_slot_for_test(&session), Some(0));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_short_packet_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5911, 0);

    session
        .handle_quest_log_remove_quest(WorldPacket::from_bytes(&[]))
        .await;

    assert!(contains_player_quest_status_for_test(&session, 5911));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 0), Some(5911));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_slot_outside_max_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5912, 0);

    run_remove_quest_slot(&mut session, 25).await;

    assert!(contains_player_quest_status_for_test(&session, 5912));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 0), Some(5912));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_valid_slot_removes_only_that_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 880_001, 7);
    add_active_quest_in_slot(&mut session, 17, 3);

    run_remove_quest_slot(&mut session, 7).await;

    assert!(!contains_player_quest_status_for_test(&session, 880_001));
    assert!(contains_player_quest_status_for_test(&session, 17));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 7), None);
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 3), Some(17));

    let update = send_rx
        .try_recv()
        .expect("C++ SetQuestSlot(slot, 0) must become an immediate player UpdateObject");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    assert!(
        !update
            .windows(std::mem::size_of::<u32>())
            .any(|window| window == 880_001_u32.to_le_bytes()),
        "quest-log abandon UpdateObject should clear the removed QuestID"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_empty_valid_slot_does_not_remove_other_quest_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5914, 4);

    run_remove_quest_slot(&mut session, 3).await;

    assert!(contains_player_quest_status_for_test(&session, 5914));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 4), Some(5914));
    assert_eq!(get_quest_slot_quest_id_for_test(&session, 3), None);
    assert!(send_rx.try_recv().is_err());
}
