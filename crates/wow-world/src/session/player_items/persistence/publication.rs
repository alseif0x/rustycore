//! publication for the existing persistence owner.

use super::*;

impl WorldSession {
    pub fn send_new_item_plan(&self, plan: &SendNewItemPlan) {
        let packet = crate::session::item_push_result_from_send_new_item_plan(plan);
        if plan.delivery == SendNewItemDelivery::GroupBroadcast {
            use wow_packet::ServerPacket;

            if self.broadcast_item_push_result_to_group(packet.to_bytes()) {
                return;
            }
        }

        // C++ `Player::SendNewItem` uses `SendDirectMessage`; opcode routing
        // places `SMSG_ITEM_PUSH_RESULT` on CONNECTION_TYPE_REALM even while
        // the inventory object updates remain on the instance connection.
        self.send_packet_realm(&packet);
    }
    pub fn send_item_time_update_plan(&self, update: &PlayerItemTimeUpdate) {
        self.send_packet(&ItemTimeUpdate {
            item_guid: update.item_guid,
            duration_left: update.expiration,
        });
    }
    pub fn send_item_time_update_plans(&self, updates: &[PlayerItemTimeUpdate]) {
        for update in updates {
            self.send_item_time_update_plan(update);
        }
    }
}
