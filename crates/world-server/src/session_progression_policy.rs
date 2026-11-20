//! Session quest, reputation and progression policy construction.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_session_progression_policy(
    quest_xp_store: &Arc<wow_data::quest_xp::QuestXpStore>,
    quest_money_reward_store: &Arc<wow_data::progression_rewards::QuestMoneyRewardStore>,
    quest_store: &Arc<wow_data::quest::QuestStore>,
    quest_v2_store: &Arc<wow_data::progression_rewards::QuestV2Store>,
    quest_info_store: &Arc<wow_data::progression_rewards::QuestInfoStore>,
    quest_package_item_store: &Arc<wow_data::progression_rewards::QuestPackageItemStore>,
    quest_faction_reward_store: &Arc<wow_data::progression_rewards::QuestFactionRewardStore>,
    progression_faction_store: &Arc<wow_data::progression_rewards::FactionStore>,
    faction_template_store: &Arc<wow_data::progression_rewards::FactionTemplateStore>,
    friendship_rep_reaction_store: &Arc<wow_data::progression_rewards::FriendshipRepReactionStore>,
    paragon_reputation_store: &Arc<wow_data::progression_rewards::ParagonReputationStore>,
    reputation_reward_rate_store: &Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>,
    creature_onkill_reputation_store: &Arc<
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp,
    >,
    reputation_spillover_template_store: &Arc<
        wow_data::reputation::RepSpilloverTemplateStoreLikeCpp,
    >,
    world_configs: &wow_config::WorldConfigSet,
) -> SessionProgressionCapabilitiesLikeCpp {
    SessionProgressionCapabilitiesLikeCpp {
        quest_xp_store: Arc::clone(quest_xp_store),
        quest_money_reward_store: Arc::clone(quest_money_reward_store),
        quest_store: Arc::clone(quest_store),
        quest_v2_store: Arc::clone(quest_v2_store),
        quest_info_store: Arc::clone(quest_info_store),
        quest_package_item_store: Arc::clone(quest_package_item_store),
        quest_faction_reward_store: Arc::clone(quest_faction_reward_store),
        progression_faction_store: Arc::clone(progression_faction_store),
        faction_template_store: Arc::clone(faction_template_store),
        friendship_rep_reaction_store: Arc::clone(friendship_rep_reaction_store),
        paragon_reputation_store: Arc::clone(paragon_reputation_store),
        reputation_reward_rate_store: Arc::clone(reputation_reward_rate_store),
        creature_onkill_reputation_store: Arc::clone(creature_onkill_reputation_store),
        reputation_spillover_template_store: Arc::clone(reputation_spillover_template_store),
        min_quest_scaled_xp_ratio: world_config_u32(
            world_configs,
            "CONFIG_MIN_QUEST_SCALED_XP_RATIO",
            0,
        ),
        max_player_level_config: world_config_u32(world_configs, "CONFIG_MAX_PLAYER_LEVEL", 80),
        max_primary_trade_skills: max_primary_trade_skills_like_cpp(world_configs),
        is_pvp_realm: is_pvp_realm_type_like_cpp(world_config_u32(
            world_configs,
            "CONFIG_GAME_TYPE",
            u32::from(REALM_TYPE_NORMAL_LIKE_CPP),
        )),
        is_ffa_pvp_realm: is_ffa_pvp_realm_type_like_cpp(world_config_u32(
            world_configs,
            "CONFIG_GAME_TYPE",
            u32::from(REALM_TYPE_NORMAL_LIKE_CPP),
        )),
        max_recruit_a_friend_bonus_player_level: world_config_u32(
            world_configs,
            "CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL",
            85,
        ),
        max_recruit_a_friend_bonus_player_level_difference: world_config_u32(
            world_configs,
            "CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL_DIFFERENCE",
            4,
        ),
    }
}
