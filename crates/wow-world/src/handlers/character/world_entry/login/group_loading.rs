// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restore the selected character's represented group membership during login.

use super::*;

impl WorldSession {
    pub(super) async fn load_group_membership_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) {
        let _ = self.set_owned_player_group_like_cpp(None);
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(rows),
            ) => {
                if let Some(db_store_id) = rows.into_iter().next() {
                    let _ = self.load_represented_group_by_db_store_id_like_cpp(db_store_id);
                    let _ = self.reset_group_update_sequence_if_needed_like_cpp();
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => warn!(
                player_guid = guid.counter(),
                error = %reason,
                "failed to load represented group membership"
            ),
            _ => unreachable!("group-membership request returned a different row family"),
        }
    }
}
