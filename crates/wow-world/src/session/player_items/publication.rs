//! Values updates and packets published for represented item state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

pub(crate) fn item_push_result_from_send_new_item_plan(
    plan: &SendNewItemPlan,
) -> wow_packet::packets::item::ItemPushResult {
    wow_world_inventory::item_push_result_from_send_new_item_plan(plan)
}

impl WorldSession {
    pub(in crate::session) fn broadcast_item_push_result_to_group(&self, bytes: Vec<u8>) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.resolved_group_guid_like_cpp(),
            &self.core.directory.group_registry,
            &self.core.player_registry,
        ) else {
            return false;
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };

        let mut delivered = false;
        for member_guid in &group.members {
            if let Some(member) = player_registry.loot_presence(*member_guid) {
                delivered |= player_registry
                    .send_current_realm_packet(member.registration, bytes.clone())
                    .is_ok();
            }
        }

        delivered
    }
}
