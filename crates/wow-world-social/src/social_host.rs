// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved text-emote and LFG lock-info bodies still
//! call (#1363 slice 4a). Each method is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move;
//! auras, instance locks, quest rewards and the represented inventory stay
//! with the host.

use wow_entities::PlayerQuestGameplayState;
use wow_world_core::session::SessionHubHostLikeCpp;

pub trait SocialSessionHostLikeCpp: SessionHubHostLikeCpp {
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize;
    fn access_requirement_leader_has_achievement_like_cpp(&self, achievement_id: u32) -> bool;
    fn lfg_has_active_instance_lock_like_cpp(&self, map_id: u32, difficulty_id: u8) -> bool;
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState>;
    fn quest_money_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32;
    fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32;
    fn represented_average_item_level_like_cpp(&self) -> Option<f32>;
    fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool;
}
