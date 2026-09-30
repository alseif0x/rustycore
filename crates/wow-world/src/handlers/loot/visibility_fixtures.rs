//! Delivery fixtures keep real commands, connection queues and visibility identity.
use super::*;
use crate::session::mailbox::{SharedClientVisibleGuidsLikeCpp, SendVisibleObjectValuesUpdateCommand};

pub fn loot_committed_visibility_for_test(session: &WorldSession) -> SharedClientVisibleGuidsLikeCpp {
    session.client_visible_guids_like_cpp.clone()
}
pub fn remove_loot_visibility_for_test(session: &mut WorldSession, guid: ObjectGuid) -> bool {
    session.client_visible_guids_like_cpp.remove(&guid)
}
pub fn set_loot_combat_logging_for_test(session: &mut WorldSession, enable: bool) {
    session.represented_set_advanced_combat_logging_like_cpp(enable);
}
pub async fn drain_loot_delivery_commands_for_test(session: &mut WorldSession) {
    session.process_represented_session_commands_like_cpp().await;
}
pub fn deliver_loot_visible_values_for_test(session: &mut WorldSession, command: SendVisibleObjectValuesUpdateCommand) {
    session.handle_send_visible_object_values_update_command_like_cpp(command);
}
pub fn insert_loot_transport_visibility_for_test(session: &mut WorldSession, guid: ObjectGuid) {
    session.client_visible_transports_like_cpp.insert(guid);
}
pub fn clear_loot_transport_visibility_for_test(session: &mut WorldSession) {
    session.client_visible_transports_like_cpp.clear();
}

pub fn loot_visibility_matches_for_test(session: &WorldSession, committed: &SharedClientVisibleGuidsLikeCpp) -> bool {
    session.client_visible_guids_like_cpp.shares_storage_like_cpp(committed)
}
