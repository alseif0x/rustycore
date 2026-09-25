// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load persisted compact unit-frame profiles during character login.

use super::*;

impl WorldSession {
    pub(super) async fn load_cuf_profiles_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) {
        // C++ `Player::_LoadCUFProfiles` fills `_CUFProfiles[id]`, then
        // `WorldSession::SendLoadCUFProfiles` sends only occupied slots. The
        // legacy fork checks `id > MAX_CUF_PROFILES`, but the backing array has
        // length MAX_CUF_PROFILES; Rust rejects `id >= MAX` to avoid the OOB
        // bug while preserving valid row semantics.
        self.clear_represented_cuf_profiles_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CufProfiles(rows),
            ) => {
                for row in rows {
                    let id = row.id;
                    let profile = wow_packet::packets::misc::CufProfile {
                        profile_name: row.name,
                        frame_height: row.frame_height,
                        frame_width: row.frame_width,
                        sort_by: row.sort_by,
                        health_text: row.health_text,
                        bool_options: row.bool_options,
                        top_point: row.top_point,
                        bottom_point: row.bottom_point,
                        left_point: row.left_point,
                        top_offset: row.top_offset,
                        bottom_offset: row.bottom_offset,
                        left_offset: row.left_offset,
                    };
                    if !self.load_represented_cuf_profile_like_cpp(id, profile) {
                        warn!(
                            player_guid = guid.counter(),
                            id,
                            max_profiles = wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                            "Skipping invalid CUF profile id"
                        );
                    }
                }
                self.mark_represented_cuf_profiles_loaded_like_cpp();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load CUF profiles for {:?}: {}", guid, reason);
            }
            _ => unreachable!("CUF-profile request returned a different row family"),
        }
    }
}
