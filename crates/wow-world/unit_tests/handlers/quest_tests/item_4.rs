//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_log_remove_inventory_registration_and_dispatcher_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestLogRemoveQuest)
        .expect("QuestLogRemoveQuest handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_quest_log_remove_quest");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_quest_log_remove_quest(pkt).await"),
        "the QuestLogRemoveQuest registration must carry the call itself"
    );
}
