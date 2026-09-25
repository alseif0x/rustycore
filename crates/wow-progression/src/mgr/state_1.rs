//! Reputation manager state definitions, part 1 of 3.
//!
//! Separated from the mgr.rs root under #662. Behaviour is preserved.

use super::*;

pub type RepListIdLikeCpp = u32;

pub type ForcedReactionsLikeCpp = BTreeMap<u32, ReputationRankLikeCpp>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetReputationOptionsLikeCpp {
    pub incremental: bool,
    pub spillover_only: bool,
    pub no_spillover: bool,
    pub reputation_gain_rate: f32,
    pub paragon_reward_quest_status_none_like_cpp: bool,
    pub renown_current_level_like_cpp: i32,
    pub renown_currency_increased_cap_quantity_like_cpp: u32,
    pub player_race: u8,
    pub player_class: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetReputationOutcomeLikeCpp {
    pub applied: bool,
    pub script_reputation_change_event: Option<(u32, i32, bool)>,
    pub spillover_mutations: Vec<(u32, ReputationMutationOutcomeLikeCpp)>,
    pub primary_mutation: Option<(u32, ReputationMutationOutcomeLikeCpp)>,
    pub send_state_rep_list_id: Option<RepListIdLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReputationMutationOutcomeLikeCpp {
    pub applied: bool,
    pub reputation_change: i32,
    pub old_rank: Option<ReputationRankLikeCpp>,
    pub new_rank: Option<ReputationRankLikeCpp>,
    pub set_at_war_for_hostile: bool,
    pub became_visible: bool,
    pub paragon_reward_quest_id_to_add_if_template_exists_like_cpp: Option<i32>,
    pub renown_currency_delta_like_cpp: Option<(u32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationCriteriaProgressKindLikeCpp {
    ReputationGained { faction_id: u32 },
    TotalExaltedFactions,
    TotalReveredFactions,
    TotalHonoredFactions,
    TotalFactionsEncountered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterReputationRowLikeCpp {
    pub faction_id: u16,
    pub standing: i32,
    pub flags: u16,
}

// C++ `FactionState` and the cached rank counters are Player state
// (`ReputationMgr.h:63`, `Player.h:3116`), so they live in `wow-entities` with
// the canonical Player and are named here by their C++ names (#735).
pub use wow_entities::{
    PlayerFactionStateLikeCpp as FactionStateLikeCpp, ReputationRankCounterLikeCpp,
    ReputationRankCountersLikeCpp,
};
