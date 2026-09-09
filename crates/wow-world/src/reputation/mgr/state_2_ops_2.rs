//! Reputation manager state definitions, part 2 of 3. operations, part 2 of 2.
//!
//! The inherent impl is divided by responsibility under #662; every
//! method keeps its original body.

use super::*;

impl ReputationMgrLikeCpp {
    pub fn set_one_faction_reputation_like_cpp(
        &mut self,
        faction_entry: &FactionEntry,
        standing: i32,
        incremental: bool,
        reputation_gain_rate: f32,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        paragon_reward_quest_status_none_like_cpp: bool,
        currency_types_store: Option<&CurrencyTypesStore>,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        player_race: u8,
        player_class: u8,
    ) -> ReputationMutationOutcomeLikeCpp {
        let rep_list_id = faction_entry.reputation_index as RepListIdLikeCpp;
        let Some(state) = self.factions.get(&rep_list_id) else {
            return ReputationMutationOutcomeLikeCpp {
                applied: false,
                reputation_change: 0,
                old_rank: None,
                new_rank: None,
                set_at_war_for_hostile: false,
                became_visible: false,
                paragon_reward_quest_id_to_add_if_template_exists_like_cpp: None,
                renown_currency_delta_like_cpp: None,
            };
        };

        let base_reputation = base_reputation_like_cpp(faction_entry, player_race, player_class);
        let old_standing = state.standing + base_reputation;
        let is_renown = faction_entry.renown_currency_id > 0;
        if is_renown
            && standing > 0
            && renown_current_level_like_cpp
                >= renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            if let Some(state) = self.factions.get_mut(&rep_list_id) {
                state.need_send = false;
                state.need_save = false;
            }
            return ReputationMutationOutcomeLikeCpp {
                applied: false,
                reputation_change: 0,
                old_rank: None,
                new_rank: None,
                set_at_war_for_hostile: false,
                became_visible: false,
                paragon_reward_quest_id_to_add_if_template_exists_like_cpp: None,
                renown_currency_delta_like_cpp: None,
            };
        }

        let mut target_standing = standing;
        if incremental || is_renown {
            target_standing =
                (target_standing as f32 * reputation_gain_rate + 0.5_f32).floor() as i32;
            target_standing += old_standing;
        }

        let paragon_reputation = paragon_reputation_store
            .and_then(|store| store.get_by_faction_id_like_cpp(faction_entry.id));
        let is_paragon = paragon_reputation.is_some();
        let is_renown = faction_entry.renown_currency_id > 0;
        let min_reputation = min_reputation_like_cpp(faction_entry, friendship_rep_reaction_store);
        let max_reputation = max_reputation_like_cpp(
            faction_entry,
            friendship_rep_reaction_store,
            paragon_reputation_store,
            old_standing,
            paragon_reward_quest_status_none_like_cpp,
            currency_types_store,
            renown_currency_increased_cap_quantity_like_cpp,
            player_race,
            player_class,
        );
        target_standing = target_standing.clamp(min_reputation, max_reputation);

        let mut old_rank = None;
        let mut new_rank = None;
        let mut set_at_war_for_hostile = false;
        if !is_paragon && !is_renown {
            let old = reputation_to_rank_like_cpp(
                faction_entry,
                old_standing,
                friendship_rep_reaction_store,
            );
            let new = reputation_to_rank_like_cpp(
                faction_entry,
                target_standing,
                friendship_rep_reaction_store,
            );
            old_rank = Some(old);
            new_rank = Some(new);

            if new <= ReputationRankLikeCpp::Hostile {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    true,
                    faction_entry,
                    friendship_rep_reaction_store,
                    player_race,
                    player_class,
                );
                set_at_war_for_hostile = true;
            }
            if new > old {
                self.send_faction_increased = true;
            }
            if faction_entry.friendship_rep_id == 0 {
                self.update_rank_counters_like_cpp(old, new);
            }
        } else {
            self.send_faction_increased = true;
        }

        let mut new_standing = target_standing - base_reputation;
        let mut reputation_change = target_standing - old_standing;
        let mut renown_currency_delta_like_cpp = None;
        if is_renown {
            if let Some(currency) = currency_types_store
                .and_then(|store| store.get(faction_entry.renown_currency_id as u32))
            {
                let renown_level_threshold =
                    renown_level_threshold_like_cpp(faction_entry, player_race, player_class);
                let renown_max_level = renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                );
                if renown_level_threshold > 0 {
                    let total_reputation = (renown_current_level_like_cpp * renown_level_threshold)
                        + (target_standing - base_reputation);
                    let new_renown_level = total_reputation / renown_level_threshold;
                    new_standing = total_reputation % renown_level_threshold;

                    if new_renown_level >= renown_max_level {
                        new_standing = 0;
                        reputation_change +=
                            (renown_max_level * renown_level_threshold) - total_reputation;
                    }

                    if let Some(state) = self.factions.get_mut(&rep_list_id) {
                        state.visual_standing_increase = reputation_change;
                    }
                    if renown_current_level_like_cpp != new_renown_level {
                        renown_currency_delta_like_cpp = Some((
                            currency.id,
                            new_renown_level - renown_current_level_like_cpp,
                        ));
                    }
                }
            }
        }

        if let Some(state) = self.factions.get_mut(&rep_list_id) {
            state.standing = new_standing;
            state.need_send = true;
            state.need_save = true;
        }
        let was_visible = self
            .get_state(rep_list_id)
            .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        self.set_visible_like_cpp(rep_list_id, paragon_reputation_store);
        let became_visible = !was_visible
            && self
                .get_state(rep_list_id)
                .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        let paragon_reward_quest_id_to_add_if_template_exists_like_cpp = paragon_reputation
            .filter(|entry| entry.level_threshold > 0)
            .filter(|entry| {
                old_standing / entry.level_threshold != target_standing / entry.level_threshold
            })
            .map(|entry| entry.quest_id);

        ReputationMutationOutcomeLikeCpp {
            applied: true,
            reputation_change,
            old_rank,
            new_rank,
            set_at_war_for_hostile,
            became_visible,
            paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
            renown_currency_delta_like_cpp,
        }
    }
    pub(super) fn set_visible_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        paragon_reputation_store: Option<&ParagonReputationStore>,
    ) {
        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction.flags.contains(ReputationFlagsLikeCpp::HIDDEN) {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::HEADER)
            && !faction
                .flags
                .contains(ReputationFlagsLikeCpp::HEADER_SHOWS_BAR)
        {
            return;
        }
        if paragon_reputation_store
            .is_some_and(|store| store.get_by_faction_id_like_cpp(faction.id).is_some())
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE) {
            return;
        }
        faction.flags |= ReputationFlagsLikeCpp::VISIBLE;
        faction.need_send = true;
        faction.need_save = true;
        self.rank_counters.visible = self.rank_counters.visible.saturating_add(1);
    }
    pub(super) fn set_inactive_like_cpp(&mut self, rep_list_id: RepListIdLikeCpp, inactive: bool) {
        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction
            .flags
            .intersects(ReputationFlagsLikeCpp::HIDDEN | ReputationFlagsLikeCpp::HEADER)
            || !faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE)
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::INACTIVE) == inactive {
            return;
        }

        if inactive {
            faction.flags |= ReputationFlagsLikeCpp::INACTIVE;
        } else {
            faction.flags &= !ReputationFlagsLikeCpp::INACTIVE;
        }
        faction.need_send = true;
        faction.need_save = true;
    }
    pub(super) fn set_at_war_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        at_war: bool,
        faction_entry: &FactionEntry,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) {
        let rank = self
            .factions
            .get(&rep_list_id)
            .map(|faction| {
                reputation_to_rank_like_cpp(
                    faction_entry,
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + faction.standing,
                    friendship_rep_reaction_store,
                )
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral);

        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction
            .flags
            .intersects(ReputationFlagsLikeCpp::HIDDEN | ReputationFlagsLikeCpp::HEADER)
        {
            return;
        }
        if at_war
            && faction.flags.contains(ReputationFlagsLikeCpp::PEACEFUL)
            && rank > ReputationRankLikeCpp::Hated
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::AT_WAR) == at_war {
            return;
        }

        if at_war {
            faction.flags |= ReputationFlagsLikeCpp::AT_WAR;
        } else {
            faction.flags &= !ReputationFlagsLikeCpp::AT_WAR;
        }
        faction.need_send = true;
        faction.need_save = true;
    }
    pub(super) fn update_rank_counters_like_cpp(
        &mut self,
        old_rank: ReputationRankLikeCpp,
        new_rank: ReputationRankLikeCpp,
    ) {
        if old_rank >= ReputationRankLikeCpp::Exalted {
            self.rank_counters.exalted = self.rank_counters.exalted.saturating_sub(1);
        }
        if old_rank >= ReputationRankLikeCpp::Revered {
            self.rank_counters.revered = self.rank_counters.revered.saturating_sub(1);
        }
        if old_rank >= ReputationRankLikeCpp::Honored {
            self.rank_counters.honored = self.rank_counters.honored.saturating_sub(1);
        }

        if new_rank >= ReputationRankLikeCpp::Exalted {
            self.rank_counters.exalted = self.rank_counters.exalted.saturating_add(1);
        }
        if new_rank >= ReputationRankLikeCpp::Revered {
            self.rank_counters.revered = self.rank_counters.revered.saturating_add(1);
        }
        if new_rank >= ReputationRankLikeCpp::Honored {
            self.rank_counters.honored = self.rank_counters.honored.saturating_add(1);
        }
    }
    pub(super) fn get_reputation_rank_by_faction_id_like_cpp(
        &self,
        faction_id: u32,
        faction_store: &FactionStore,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) -> ReputationRankLikeCpp {
        faction_store
            .get(faction_id)
            .map(|faction| {
                let standing = self
                    .get_state(faction.reputation_index as RepListIdLikeCpp)
                    .map(|state| state.standing)
                    .unwrap_or(0)
                    + base_reputation_like_cpp(faction, player_race, player_class);
                reputation_to_rank_like_cpp(faction, standing, friendship_rep_reaction_store)
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral)
    }
    pub(super) fn can_gain_paragon_reputation_for_faction_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        faction_store: &FactionStore,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        currency_types_store: Option<&CurrencyTypesStore>,
        player_race: u8,
        player_class: u8,
    ) -> bool {
        if faction_store
            .get(u32::from(faction_entry.paragon_faction_id))
            .is_none()
        {
            return false;
        }

        let rank = self
            .get_state(faction_entry.reputation_index as RepListIdLikeCpp)
            .map(|state| {
                reputation_to_rank_like_cpp(
                    faction_entry,
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + state.standing,
                    None,
                )
            });
        if rank != Some(ReputationRankLikeCpp::Exalted)
            && renown_current_level_like_cpp
                < renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            return false;
        }

        paragon_reputation_store
            .and_then(|store| {
                store.get_by_faction_id_like_cpp(u32::from(faction_entry.paragon_faction_id))
            })
            .is_some()
    }
}
