use std::sync::Arc;

use crate::session::state::SessionCore;
use wow_core::Position;

impl SessionCore {
    pub fn sync_canonical_player_position_if_same_or_detached_like_cpp(
        &mut self,
        map_id: u16,
        position: Position,
    ) {
        let (Some(manager), Some(handle)) = (
            self.canonical_map_manager.as_ref().map(Arc::clone),
            self.player_handle_like_cpp,
        ) else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => {
                let _ = manager.with_player_mut_like_cpp(handle, |player| {
                    player.unit_mut().world_mut().relocate(position);
                });
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(key))
                if key.map_id == u32::from(map_id) =>
            {
                let _ = manager.relocate_player_like_cpp(handle, position);
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) | None => {}
        }
    }
}
