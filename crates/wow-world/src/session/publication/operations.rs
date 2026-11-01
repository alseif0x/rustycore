//! Packets and values updates published from the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// The loot host's entry to the entity-bridge values publication, moved
    /// to `wow_world_inventory` (#1363 slice 12b7).
    pub(crate) fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, wow_core::ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        wow_world_inventory::send_player_values_update_from_entity_bridge(
            self,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    pub(crate) fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.core.send_tx()
    }
    pub fn send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
        self.core.send_packet(pkt)
    }
    pub(crate) fn send_raw_packet(&self, data: &[u8]) {
        self.core.send_raw_packet(data)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/publication/operations/test_only.rs"]
mod test_only;
