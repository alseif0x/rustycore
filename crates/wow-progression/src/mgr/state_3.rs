//! Reputation manager state definitions, part 3 of 3.
//!
//! Separated from the mgr.rs root under #662. Behaviour is preserved.

use super::*;

pub(super) fn default_state_flags_like_cpp(
    faction_entry: &FactionEntry,
    paragon_reputation_store: Option<&ParagonReputationStore>,
    player_race: u8,
    player_class: u8,
) -> ReputationFlagsLikeCpp {
    let mut flags =
        faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
            .map(|index| {
                ReputationFlagsLikeCpp::from_bits_truncate(faction_entry.reputation_flags[index])
            })
            .unwrap_or(ReputationFlagsLikeCpp::NONE);

    if paragon_reputation_store
        .is_some_and(|store| store.get_by_faction_id_like_cpp(faction_entry.id).is_some())
    {
        flags |= ReputationFlagsLikeCpp::SHOW_PROPAGATED;
    }

    flags
}

pub(super) fn base_reputation_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> i32 {
    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_base[index])
        .unwrap_or(0)
}

pub(super) fn base_rank_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> ReputationRankLikeCpp {
    reputation_to_rank_like_cpp(
        faction_entry,
        base_reputation_like_cpp(faction_entry, player_race, player_class),
        None,
    )
}

pub(super) fn min_reputation_like_cpp(
    faction_entry: &FactionEntry,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
) -> i32 {
    friendship_rep_reaction_store
        .filter(|_| faction_entry.friendship_rep_id != 0)
        .and_then(|store| {
            store
                .reactions_for_friendship_rep_like_cpp(faction_entry.friendship_rep_id)
                .first()
                .map(|entry| i32::from(entry.reaction_threshold))
        })
        .unwrap_or(wow_data::reputation::REPUTATION_BOTTOM_LIKE_CPP)
}

pub(super) fn max_reputation_like_cpp(
    faction_entry: &FactionEntry,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
    paragon_reputation_store: Option<&ParagonReputationStore>,
    current_reputation: i32,
    paragon_reward_quest_status_none_like_cpp: bool,
    currency_types_store: Option<&CurrencyTypesStore>,
    renown_currency_increased_cap_quantity_like_cpp: u32,
    player_race: u8,
    player_class: u8,
) -> i32 {
    if let Some(paragon_reputation) = paragon_reputation_store
        .and_then(|store| store.get_by_faction_id_like_cpp(faction_entry.id))
        .filter(|entry| entry.level_threshold > 0)
    {
        let threshold = paragon_reputation.level_threshold;
        let mut cap = current_reputation + threshold - current_reputation % threshold - 1;
        if paragon_reward_quest_status_none_like_cpp {
            cap += threshold;
        }
        return cap;
    }

    if faction_entry.renown_currency_id > 0 {
        return renown_max_level_like_cpp(
            faction_entry,
            currency_types_store,
            renown_currency_increased_cap_quantity_like_cpp,
        ) * renown_level_threshold_like_cpp(faction_entry, player_race, player_class);
    }

    if let Some(max) = friendship_rep_reaction_store
        .filter(|_| faction_entry.friendship_rep_id != 0)
        .and_then(|store| {
            store
                .reactions_for_friendship_rep_like_cpp(faction_entry.friendship_rep_id)
                .last()
                .map(|entry| i32::from(entry.reaction_threshold))
        })
    {
        return max;
    }

    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_max[index])
        .unwrap_or(wow_data::reputation::REPUTATION_CAP_LIKE_CPP)
}

pub(super) fn renown_level_threshold_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> i32 {
    if faction_entry.renown_currency_id <= 0 {
        return 0;
    }

    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_max[index])
        .unwrap_or(0)
}

pub(super) fn renown_max_level_like_cpp(
    faction_entry: &FactionEntry,
    currency_types_store: Option<&CurrencyTypesStore>,
    renown_currency_increased_cap_quantity_like_cpp: u32,
) -> i32 {
    if faction_entry.renown_currency_id <= 0 {
        return 0;
    }

    currency_types_store
        .and_then(|store| store.get(faction_entry.renown_currency_id as u32))
        .filter(|currency| currency.has_max_quantity(false, false))
        .map(|currency| {
            currency
                .max_qty
                .saturating_add(renown_currency_increased_cap_quantity_like_cpp) as i32
        })
        .unwrap_or(0)
}

pub fn reputation_to_rank_like_cpp(
    faction_entry: &FactionEntry,
    standing: i32,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
) -> ReputationRankLikeCpp {
    if let Some(friendship_rep_id) =
        (faction_entry.friendship_rep_id != 0).then_some(faction_entry.friendship_rep_id)
    {
        if let Some(store) = friendship_rep_reaction_store {
            let rank = rank_from_thresholds_like_cpp(
                store
                    .reactions_for_friendship_rep_like_cpp(friendship_rep_id)
                    .into_iter()
                    .map(|entry| i32::from(entry.reaction_threshold)),
                standing,
            );
            if let Some(rank) = ReputationRankLikeCpp::from_u8_like_cpp(rank) {
                return rank;
            }
        }
    }

    wow_data::reputation::reputation_rank_from_standing_like_cpp(standing)
}

pub(super) fn rank_from_thresholds_like_cpp(
    thresholds: impl IntoIterator<Item = i32>,
    standing: i32,
) -> u8 {
    let mut rank: i32 = -1;
    for threshold in thresholds {
        if standing < threshold {
            break;
        }
        rank += 1;
    }
    rank.clamp(
        ReputationRankLikeCpp::Hated.as_u8() as i32,
        ReputationRankLikeCpp::Exalted.as_u8() as i32,
    ) as u8
}

pub(super) fn faction_data_index_for_race_and_class_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> Option<usize> {
    let class_mask = player_class_mask_like_cpp(player_class)?;

    for index in 0..4 {
        let race_mask = faction_entry.reputation_race_mask[index] as u64;
        let class_slot_mask = if faction_entry.reputation_class_mask[index] < 0 {
            0
        } else {
            faction_entry.reputation_class_mask[index] as u32
        };
        let race_matches = race_mask_has_race_like_cpp(race_mask, player_race)
            || (race_mask == 0 && class_slot_mask != 0);
        let class_matches = (class_slot_mask & class_mask) != 0 || class_slot_mask == 0;

        if race_matches && class_matches {
            return Some(index);
        }
    }

    None
}

pub(super) fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    (1..=13)
        .contains(&class_id)
        .then(|| 1_u32 << (class_id - 1))
}

pub(super) fn race_mask_has_race_like_cpp(mask: u64, race_id: u8) -> bool {
    player_race_mask_like_cpp(race_id).is_some_and(|race_mask| (mask & race_mask) != 0)
}

pub(super) fn player_race_mask_like_cpp(race_id: u8) -> Option<u64> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u64 << bit)
}

pub(super) fn standing_for_packet_like_cpp(state: &FactionStateLikeCpp) -> i32 {
    if state.visual_standing_increase != 0 {
        state.visual_standing_increase
    } else {
        state.standing
    }
}
