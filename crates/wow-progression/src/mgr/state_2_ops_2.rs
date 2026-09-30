//! Reputation manager state definitions, part 2 of 3. operations, part 2 of 2.
//!
//! The inherent impl is divided by responsibility under #662; every
//! method keeps its original body.

use super::*;

impl<S: std::borrow::Borrow<PlayerReputationStateLikeCpp>> ReputationMgrLikeCpp<S> {
    pub(super) fn get_reputation_rank_by_faction_id_like_cpp(
        &self,
        faction_id: u32,
        catalogs: &impl ReputationCatalogReadLikeCpp,
        player_race: u8,
        player_class: u8,
    ) -> ReputationRankLikeCpp {
        catalogs
            .faction_like_cpp(faction_id)
            .map(|faction| {
                let standing = self
                    .get_state(faction.reputation_index as RepListIdLikeCpp)
                    .map(|state| state.standing)
                    .unwrap_or(0)
                    + base_reputation_like_cpp(faction, player_race, player_class);
                reputation_to_rank_like_cpp(faction, standing, catalogs)
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral)
    }
    pub(super) fn can_gain_paragon_reputation_for_faction_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        catalogs: &impl ReputationCatalogReadLikeCpp,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        player_race: u8,
        player_class: u8,
    ) -> bool {
        if catalogs
            .faction_like_cpp(u32::from(faction_entry.paragon_faction_id))
            .is_none()
        {
            return false;
        }

        let rank = self
            .get_state(faction_entry.reputation_index as RepListIdLikeCpp)
            .map(|state| {
                // Preserve the existing paragon-gate rank lookup, which
                // deliberately ignored the optional friendship catalog.
                reputation_rank_from_standing_like_cpp(
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + state.standing,
                )
            });
        if rank != Some(ReputationRankLikeCpp::Exalted)
            && renown_current_level_like_cpp
                < renown_max_level_like_cpp(
                    faction_entry,
                    catalogs,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            return false;
        }

        catalogs
            .paragon_for_faction_like_cpp(u32::from(faction_entry.paragon_faction_id))
            .is_some()
    }
}

impl<S: std::borrow::BorrowMut<PlayerReputationStateLikeCpp>> ReputationMgrLikeCpp<S> {
    /// C++ `ReputationMgr::_sendFactionIncreased`, set by a gain and cleared
    /// once the visual has been published.
    pub fn set_send_faction_increased_like_cpp(&mut self, value: bool) {
        self.state_mut().set_send_faction_increased_like_cpp(value);
    }

    pub fn set_one_faction_reputation_like_cpp(
        &mut self,
        faction_entry: &FactionEntry,
        standing: i32,
        incremental: bool,
        reputation_gain_rate: f32,
        catalogs: &impl ReputationCatalogReadLikeCpp,
        paragon_reward_quest_status_none_like_cpp: bool,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        player_race: u8,
        player_class: u8,
    ) -> ReputationMutationOutcomeLikeCpp {
        let rep_list_id = faction_entry.reputation_index as RepListIdLikeCpp;
        let Some(state) = self.state().faction_like_cpp(rep_list_id) else {
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
                    catalogs,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            if let Some(state) = self.state_mut().faction_mut_like_cpp(rep_list_id) {
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

        let paragon_reputation = catalogs.paragon_for_faction_like_cpp(faction_entry.id);
        let is_paragon = paragon_reputation.is_some();
        let is_renown = faction_entry.renown_currency_id > 0;
        let min_reputation = min_reputation_like_cpp(faction_entry, catalogs);
        let max_reputation = max_reputation_like_cpp(
            faction_entry,
            catalogs,
            old_standing,
            paragon_reward_quest_status_none_like_cpp,
            renown_currency_increased_cap_quantity_like_cpp,
            player_race,
            player_class,
        );
        target_standing = target_standing.clamp(min_reputation, max_reputation);

        let mut old_rank = None;
        let mut new_rank = None;
        let mut set_at_war_for_hostile = false;
        if !is_paragon && !is_renown {
            let old = reputation_to_rank_like_cpp(faction_entry, old_standing, catalogs);
            let new = reputation_to_rank_like_cpp(faction_entry, target_standing, catalogs);
            old_rank = Some(old);
            new_rank = Some(new);

            if new <= ReputationRankLikeCpp::Hostile {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    true,
                    faction_entry,
                    catalogs,
                    player_race,
                    player_class,
                );
                set_at_war_for_hostile = true;
            }
            if new > old {
                self.state_mut().set_send_faction_increased_like_cpp(true);
            }
            if faction_entry.friendship_rep_id == 0 {
                self.update_rank_counters_like_cpp(old, new);
            }
        } else {
            self.state_mut().set_send_faction_increased_like_cpp(true);
        }

        let mut new_standing = target_standing - base_reputation;
        let mut reputation_change = target_standing - old_standing;
        let mut renown_currency_delta_like_cpp = None;
        if is_renown {
            if let Some(currency) =
                catalogs.currency_types_like_cpp(faction_entry.renown_currency_id as u32)
            {
                let renown_level_threshold =
                    renown_level_threshold_like_cpp(faction_entry, player_race, player_class);
                let renown_max_level = renown_max_level_like_cpp(
                    faction_entry,
                    catalogs,
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

                    if let Some(state) = self.state_mut().faction_mut_like_cpp(rep_list_id) {
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

        if let Some(state) = self.state_mut().faction_mut_like_cpp(rep_list_id) {
            state.standing = new_standing;
            state.need_send = true;
            state.need_save = true;
        }
        let was_visible = self
            .get_state(rep_list_id)
            .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        self.set_visible_like_cpp(rep_list_id, catalogs);
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
        catalogs: &impl ReputationCatalogReadLikeCpp,
    ) {
        let Some(faction) = self.state_mut().faction_mut_like_cpp(rep_list_id) else {
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
        if catalogs
            .paragon_for_faction_like_cpp(faction.faction_id)
            .is_some()
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE) {
            return;
        }
        faction.flags |= ReputationFlagsLikeCpp::VISIBLE;
        faction.need_send = true;
        faction.need_save = true;
        self.state_mut()
            .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Visible, 1);
    }
    pub(super) fn set_inactive_like_cpp(&mut self, rep_list_id: RepListIdLikeCpp, inactive: bool) {
        let Some(faction) = self.state_mut().faction_mut_like_cpp(rep_list_id) else {
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
        catalogs: &impl ReputationCatalogReadLikeCpp,
        player_race: u8,
        player_class: u8,
    ) {
        let rank = self
            .state()
            .faction_like_cpp(rep_list_id)
            .map(|faction| {
                reputation_to_rank_like_cpp(
                    faction_entry,
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + faction.standing,
                    catalogs,
                )
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral);

        let Some(faction) = self.state_mut().faction_mut_like_cpp(rep_list_id) else {
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
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Exalted, -1);
        }
        if old_rank >= ReputationRankLikeCpp::Revered {
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Revered, -1);
        }
        if old_rank >= ReputationRankLikeCpp::Honored {
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Honored, -1);
        }

        if new_rank >= ReputationRankLikeCpp::Exalted {
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Exalted, 1);
        }
        if new_rank >= ReputationRankLikeCpp::Revered {
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Revered, 1);
        }
        if new_rank >= ReputationRankLikeCpp::Honored {
            self.state_mut()
                .adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Honored, 1);
        }
    }
}
