// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the bodies moved out of the World
//! `session/social`, `session/chat`, `session/visibility` and
//! `session/deferred_visibility` in #1363 slices 12b5–12b6: one-call forwards
//! to the moved functions the scenarios call, and the field-split forwards
//! whose production callers now name the split, so no scenario call site
//! changes.
#![cfg(test)]

use crate::session::WorldSession;
use wow_core::ObjectGuid;

impl WorldSession {
    pub(crate) fn current_group_member_guids_for_tap_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        wow_world_instances::current_group_member_guids_for_tap_like_cpp(self, player_guid)
    }
    pub(crate) fn reset_group_update_sequence_if_needed_like_cpp(&mut self) -> bool {
        wow_world_instances::reset_group_update_sequence_if_needed_like_cpp(self)
    }
    pub(crate) fn set_owned_player_group_like_cpp(
        &mut self,
        membership: Option<(u64, u8)>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_owned_player_group_like_cpp(&mut hub, membership)
    }
    pub(crate) fn set_represented_duel_arbiter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let (state, mut hub) = crate::session::split_social_mut(self);
        wow_world_social::set_represented_duel_arbiter_guid_like_cpp(&mut hub, state, guid);
    }
    pub(crate) fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool {
        wow_world_social::set_represented_guild_id_like_cpp(self, guild_id)
    }
    pub(crate) fn replace_player_gossip_options_like_cpp(
        &mut self,
        options: Vec<crate::session::GossipOptionInfo>,
    ) -> bool {
        wow_world_character::replace_player_gossip_options_like_cpp(self, options)
    }
    pub(crate) fn player_gossip_option_like_cpp(
        &self,
        gossip_option_id: i32,
    ) -> Option<crate::session::GossipOptionInfo> {
        wow_world_character::player_gossip_option_like_cpp(self, gossip_option_id)
    }
    pub(crate) fn represented_creature_gossip_text_like_cpp(
        &self,
        creature_entry: u32,
    ) -> Vec<wow_packet::packets::gossip::ClientGossipText> {
        wow_world_quest::represented_creature_gossip_text_like_cpp(self, creature_entry)
    }
}
