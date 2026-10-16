//! Packets and values updates published from the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let packet_update = unit_values_update_to_packet(values_update)?;
        Some(
            self.represented_unit_packet_update_to_update_object_like_cpp(
                unit_guid,
                map_id,
                packet_update,
            ),
        )
    }
    pub(crate) fn represented_unit_packet_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        mut packet_update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UpdateObject {
        if unit_guid.is_any_type_creature()
            && (packet_update.npc_flags[0] & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) != 0
        {
            let npc_flags = u64::from(packet_update.npc_flags[0])
                | (u64::from(packet_update.npc_flags[1]) << 32);
            let filtered =
                self.represented_viewer_dependent_creature_npc_flags_like_cpp(unit_guid, npc_flags);
            packet_update.npc_flags[0] = filtered as u32;
            packet_update.npc_flags[1] = (filtered >> 32) as u32;
        }

        wow_packet::packets::update::UpdateObject::unit_values_update(
            unit_guid,
            map_id,
            packet_update,
        )
    }
    pub(crate) fn player_values_update_snapshot(
        &self,
    ) -> Option<wow_world_core::session::InventoryPlayerProjectionLikeCpp> {
        let access = self.core.owned_inventory_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        self.inventory
            .player_values_update_snapshot_with_access_like_cpp(
                &access,
                &publication,
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
            )
    }
    pub(crate) fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        let access = self.core.owned_inventory_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        self.inventory
            .send_player_values_update_from_entity_bridge_with_access_like_cpp(
                &access,
                &publication,
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                inv_slot_changes,
                visible_item_changes,
                virtual_item_changes,
                buyback_changes,
                coinage,
            )
    }
    pub(crate) fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32) {
        wow_world_character::send_represented_cinematic_start_like_cpp(
            &mut crate::session::hub_mut(self),
            cinematic_id,
        );
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_represented_resting_player_flag_update_like_cpp(hub)
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
