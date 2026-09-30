//! stat publication for the existing modifiers owner.

use super::*;

impl WorldSession {
    fn represented_item_bonus_player_stat_update_object_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let player_guid = self.player_guid()?;
        let bonuses = self.resolved_item_bonus_state_like_cpp()?;
        Some(
            wow_packet::packets::update::UpdateObject::player_stat_update(
                player_guid,
                self.player_map_id_like_cpp(),
                represented_player_stat_changes_like_cpp(&bonuses),
            ),
        )
    }
    pub(crate) fn send_represented_item_bonus_player_stat_update_like_cpp(&mut self) -> bool {
        // Item changes alter derived stats, vital maxima and weapon ranges
        // together. Publish the same complete projection consumed by combat;
        // sending only the raw bonus accumulator would leave the canonical
        // snapshot stale and could make the client and server disagree after
        // repair or an equipment-set swap. A raw packet is retained only as a
        // fixture/early-login fallback when the complete projection cannot yet
        // be formed; no derived combat snapshot exists in that state.
        if self.send_stat_update() {
            return true;
        }
        let Some(update) = self.represented_item_bonus_player_stat_update_object_like_cpp() else {
            return false;
        };
        self.send_packet(&update);
        true
    }
}
