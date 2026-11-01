// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved text-emote, LFG lock-info (#1363 slice 4a),
//! arena-skirmish and wargame-invite (slice 4b) and reputation-gain (slice 7b)
//! bodies still call. Each method is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move;
//! auras, instance locks, quest rewards and the represented inventory stay
//! with the host.

use wow_entities::PlayerQuestGameplayState;
use wow_world_core::session::{HubMut, SessionHubHostLikeCpp};

pub trait SocialSessionHostLikeCpp: SessionHubHostLikeCpp {
    /// The World `split_social_mut` split (the duel arbiter install, #1363
    /// slice 12b5).
    fn split_social_mut(&mut self) -> (&mut crate::SessionSocialLimits, HubMut<'_>);
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize;
    fn access_requirement_leader_has_achievement_like_cpp(&self, achievement_id: u32) -> bool;
    fn lfg_has_active_instance_lock_like_cpp(&self, map_id: u32, difficulty_id: u8) -> bool;
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState>;
    fn quest_money_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32;
    fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32;
    fn represented_average_item_level_like_cpp(&self) -> Option<f32>;
    fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool;
    fn resolved_group_guid_like_cpp(&self) -> Option<u64>;
    fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        effect: wow_entities::RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32>;
    fn gets_recruit_a_friend_bonus_like_cpp(&self, for_xp: bool) -> bool;
    /// The World `social` field inside the World crate's own `cfg(test)`
    /// build, `None` everywhere else: the force-deselect fixture record runs
    /// exactly where its former `#[cfg(test)]` block ran (#1363 slice 12b3).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn social_limits_for_test_like_cpp(&mut self) -> Option<&mut crate::SessionSocialLimits> {
        None
    }
}
