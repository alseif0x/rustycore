//! Reputation runtime state and pure quest reward valuation.

pub mod mgr;
mod quest_dialog;
pub use quest_dialog::QuestDialogClassification;
mod quest_reputation;
mod quest_rewards;
mod reputation_gain;

pub use quest_reputation::{
    QuestReputationReward, QuestReputationSlotRules, QuestReputationSource,
    calculate_quest_reputation_reward,
};

pub use quest_rewards::{
    calculate_quest_xp, effective_quest_level, fallback_quest_xp, player_level_difficulty_xp,
    quest_money_value, quest_xp_is_blocked,
};

pub use mgr::{
    FactionStateLikeCpp, ForcedReactionsLikeCpp, RepListIdLikeCpp, ReputationMgrLikeCpp,
    ReputationMgrMutLikeCpp, ReputationMgrRefLikeCpp, ReputationRankCounterLikeCpp,
    ReputationRankCountersLikeCpp, reputation_to_rank_like_cpp,
};

pub use reputation_gain::{
    apply_recruit_a_friend_reputation_bonus, calculate_reputation_gain,
    reputation_gain_percent_before_reward_rate,
};
