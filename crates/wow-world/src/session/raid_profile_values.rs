// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Raid profile values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(in crate::session) fn player_cuf_profile_from_packet_like_cpp(
    profile: wow_packet::packets::misc::CufProfile,
) -> wow_entities::PlayerCufProfile {
    wow_entities::PlayerCufProfile {
        profile_name: profile.profile_name,
        frame_height: profile.frame_height,
        frame_width: profile.frame_width,
        sort_by: profile.sort_by,
        health_text: profile.health_text,
        top_point: profile.top_point,
        bottom_point: profile.bottom_point,
        left_point: profile.left_point,
        top_offset: profile.top_offset,
        bottom_offset: profile.bottom_offset,
        left_offset: profile.left_offset,
        bool_options: profile.bool_options,
    }
}

pub(in crate::session) fn player_cuf_profile_to_packet_like_cpp(
    profile: &wow_entities::PlayerCufProfile,
) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: profile.profile_name.clone(),
        frame_height: profile.frame_height,
        frame_width: profile.frame_width,
        sort_by: profile.sort_by,
        health_text: profile.health_text,
        top_point: profile.top_point,
        bottom_point: profile.bottom_point,
        left_point: profile.left_point,
        top_offset: profile.top_offset,
        bottom_offset: profile.bottom_offset,
        left_offset: profile.left_offset,
        bool_options: profile.bool_options,
    }
}

impl WorldSession {
    pub(crate) fn clear_represented_cuf_profiles_like_cpp(&mut self) {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.reset_cuf_profiles_like_cpp();
        });
        if canonical.is_some() {
            return;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.cuf_profiles_like_cpp =
                vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            self.cuf_profiles_loaded_like_cpp = false;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_cuf_profiles_like_cpp(
        &self,
    ) -> &[Option<wow_packet::packets::misc::CufProfile>] {
        &self.cuf_profiles_like_cpp
    }
}
