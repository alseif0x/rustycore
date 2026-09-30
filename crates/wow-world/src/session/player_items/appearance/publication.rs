//! publication for the existing appearance owner.

use super::*;

impl WorldSession {
    /// C++ `CollectionMgr::SendFavoriteAppearances`.
    pub fn send_favorite_appearances_like_cpp(&self) {
        if !account_transmog_update_opcode_resolved_like_cpp() {
            warn!(
                "Skipping AccountTransmogUpdate full update: legacy C++ opcode is unresolved 0xBADD for 54261"
            );
            return;
        }

        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return;
        };
        let favorite_appearances = collections.favorite_appearance_ids();

        self.send_packet(&wow_packet::packets::collection::AccountTransmogUpdate {
            is_full_update: true,
            is_set_favorite: false,
            favorite_appearances,
            new_appearances: Vec::new(),
        });
    }
}
