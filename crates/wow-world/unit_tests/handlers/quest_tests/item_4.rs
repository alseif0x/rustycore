//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_log_remove_inventory_registration_and_dispatcher_contract_like_cpp() {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == ClientOpcodes::QuestLogRemoveQuest)
        .expect("QuestLogRemoveQuest handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_quest_log_remove_quest");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains(".handle_quest_log_remove_quest(pkt)"),
        "the QuestLogRemoveQuest registration must carry the call itself"
    );
}
