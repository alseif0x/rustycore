//! Reputation manager regression scenarios.
//!
//! Separated from the mgr.rs root under #662.

use super::*;
use wow_constants::{CurrencyTypesFlags, CurrencyTypesFlagsB};
use wow_data::CurrencyTypesEntry;
use wow_data::progression_rewards::{FriendshipRepReactionEntry, ParagonReputationEntry};

fn currency_entry_for_test_like_cpp(id: u32, max_qty: u32) -> CurrencyTypesEntry {
    CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: CurrencyTypesFlags::empty(),
        flags_b: CurrencyTypesFlagsB::empty(),
    }
}

fn set_reputation_options_for_test_like_cpp(incremental: bool) -> SetReputationOptionsLikeCpp {
    SetReputationOptionsLikeCpp {
        incremental,
        spillover_only: false,
        no_spillover: false,
        reputation_gain_rate: 1.0,
        paragon_reward_quest_status_none_like_cpp: true,
        renown_current_level_like_cpp: 0,
        renown_currency_increased_cap_quantity_like_cpp: 0,
        player_race: 1,
        player_class: 1,
    }
}

mod scenarios_1;
mod scenarios_2;
