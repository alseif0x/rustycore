// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the chat handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only splits
//! its social state from the hub and lends the process chat policy, so no
//! session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends `CMSG_SEND_TEXT_EMOTE`,
//! whose body stays in the World shell and needs the session catalog view.
//!
//! `#1363 slice 4a`: the text-emote and LFG lock-info bodies moved to
//! `wow_world_social`; `SocialSessionHostLikeCpp` lends them the session
//! operations they still call.

use wow_entities::{PlayerQuestGameplayState, RepresentedAuraEffectLikeCpp};
use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_social::{ChatHandlerCxLikeCpp, ChatHandlerHostLikeCpp, SocialSessionHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl ChatHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn chat_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> ChatHandlerCxLikeCpp<'a> {
        let (social, hub) = crate::session::split_social_mut(self);
        ChatHandlerCxLikeCpp::new(hub, social, catalogs.chat_policy.as_ref())
    }

    fn handle_text_emote_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_social::handle_text_emote_with_catalogs_like_cpp(
                self,
                catalogs.emotes_text.as_ref(),
                catalogs.emotes.as_ref(),
                catalogs.chat_policy.as_ref(),
                pkt,
            )
            .await
        })
    }
}

impl SocialSessionHostLikeCpp for WorldSession {
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize {
        wow_world_spell::remove_auras_with_interrupt_flags_like_cpp(self, flags, flags2)
    }

    fn access_requirement_leader_has_achievement_like_cpp(&self, achievement_id: u32) -> bool {
        WorldSession::access_requirement_leader_has_achievement_like_cpp(self, achievement_id)
    }

    fn lfg_has_active_instance_lock_like_cpp(&self, map_id: u32, difficulty_id: u8) -> bool {
        WorldSession::lfg_has_active_instance_lock_like_cpp(self, map_id, difficulty_id)
    }

    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState> {
        WorldSession::player_quest_gameplay_snapshot_like_cpp(self)
    }

    fn quest_money_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32 {
        WorldSession::quest_money_reward_like_cpp(self, quest)
    }

    fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32 {
        WorldSession::quest_xp_reward_like_cpp(self, quest)
    }

    fn represented_average_item_level_like_cpp(&self) -> Option<f32> {
        WorldSession::represented_average_item_level_like_cpp(self)
    }

    fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool {
        WorldSession::represented_has_item_count_like_cpp(self, item_entry, count)
    }

    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_group_guid_like_cpp(self)
    }

    fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32> {
        WorldSession::resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
            self, effect, misc_value,
        )
    }

    fn gets_recruit_a_friend_bonus_like_cpp(&self, for_xp: bool) -> bool {
        WorldSession::gets_recruit_a_friend_bonus_like_cpp(self, for_xp)
    }
}
