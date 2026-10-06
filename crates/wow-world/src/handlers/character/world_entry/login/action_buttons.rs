// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load and packet-project the selected character action-button map.

use super::*;

impl WorldSession {
    pub(super) async fn load_action_buttons_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> Option<[i64; 180]> {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .load_action_buttons_for_login_like_cpp(&mut hub, player_lifecycle_port, guid)
            .await
    }
}
