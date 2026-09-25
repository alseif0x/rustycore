// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Faction reactions: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ReputationMgrLikeCpp, WorldSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedFactionReactionInputLikeCpp {
    pub source_faction_template_id: u32,
    pub target_faction_template_id: u32,
    pub target_has_player_owner: bool,
    pub target_player_owner_is_current_session: bool,
    pub target_player_contested_pvp: bool,
    pub target_is_unit: bool,
    pub target_ignores_reputation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGetReactionInputLikeCpp {
    pub self_faction_template_id: u32,
    pub target_faction_template_id: u32,
    pub same_object: bool,
    pub attackable_by_summoner: bool,
    pub same_charmer_or_owner_or_self: bool,
    pub self_has_player_owner: bool,
    pub target_has_player_owner: bool,
    pub target_player_owner_is_current_session: bool,
    pub target_owner_forced_rank_for_self: Option<wow_data::reputation::ReputationRankLikeCpp>,
    pub same_player_owner: bool,
    pub duel_in_progress: bool,
    pub same_raid: bool,
    pub self_unit_player_controlled: bool,
    pub target_unit_player_controlled: bool,
    pub self_ffa_pvp: bool,
    pub target_ffa_pvp: bool,
    pub self_ignores_reputation: bool,
    pub target_ignores_reputation: bool,
    pub target_is_unit: bool,
    pub target_player_contested_pvp: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct AttackReputationFactionSnapshotLikeCpp {
    pub(in crate::session) faction_id: u32,
    pub(in crate::session) contested_guard: bool,
    pub(in crate::session) can_have_reputation: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReputationGainSourceLikeCpp {
    Kill,
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
    Spell,
}

impl WorldSession {
    pub(crate) const fn reset_schedule_like_cpp(&self) -> wow_instances::ResetSchedule {
        self.reset_schedule_like_cpp
    }

    pub(crate) fn resolved_watched_faction_index_like_cpp(&self) -> Option<i32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.watched_faction_index_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.watched_faction_index_like_cpp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn watched_faction_index_like_cpp(&self) -> i32 {
        self.resolved_watched_faction_index_like_cpp()
            .expect("Player watched-faction owner must resolve")
    }

    pub(crate) fn set_watched_faction_index_like_cpp(&mut self, index: i32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_watched_faction_index_like_cpp(index)
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.watched_faction_index_like_cpp = index;
        }
    }

    pub(crate) fn represented_faction_reaction_to_like_cpp(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template_store) = self.factions.template_store.as_ref() else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(source_faction_template) =
            faction_template_store.get(input.source_faction_template_id)
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(target_faction_template) =
            faction_template_store.get(input.target_faction_template_id)
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(reputation_state) = self.cloned_reputation_state_like_cpp() else {
            return ReputationRankLikeCpp::Neutral;
        };
        let reputation_mgr = ReputationMgrLikeCpp::borrowing_like_cpp(&reputation_state);

        if input.target_has_player_owner && input.target_player_owner_is_current_session {
            if source_faction_template.is_contested_guard_faction_like_cpp()
                && input.target_player_contested_pvp
            {
                return ReputationRankLikeCpp::Hostile;
            }
            if let Some(forced_rank) = reputation_mgr
                .forced_rank_by_faction_id_like_cpp(u32::from(source_faction_template.faction))
            {
                return forced_rank;
            }
            if input.target_is_unit
                && !input.target_ignores_reputation
                && let Some(faction_store) = self.factions.store.as_ref()
                && let Some(source_faction_entry) =
                    faction_store.get(u32::from(source_faction_template.faction))
                && source_faction_entry.can_have_reputation_like_cpp()
            {
                let mut rank = reputation_mgr.rank_for_faction_entry_like_cpp(
                    source_faction_entry,
                    self.friendship_rep_reaction_store.as_deref(),
                    self.player_race_like_cpp(),
                    self.player_class_like_cpp(),
                );
                if reputation_mgr.is_at_war_with_faction_like_cpp(source_faction_entry)
                    && rank > ReputationRankLikeCpp::Neutral
                {
                    rank = ReputationRankLikeCpp::Neutral;
                }
                return rank;
            }
        }

        if source_faction_template.is_hostile_to_like_cpp(target_faction_template) {
            return ReputationRankLikeCpp::Hostile;
        }
        if source_faction_template.is_friendly_to_like_cpp(target_faction_template) {
            return ReputationRankLikeCpp::Friendly;
        }
        if target_faction_template.is_friendly_to_like_cpp(source_faction_template) {
            return ReputationRankLikeCpp::Friendly;
        }
        if source_faction_template.is_hostile_by_default_like_cpp() {
            return ReputationRankLikeCpp::Hostile;
        }
        ReputationRankLikeCpp::Neutral
    }

    pub(crate) fn represented_get_reaction_to_like_cpp(
        &self,
        input: RepresentedGetReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        if input.same_object {
            return ReputationRankLikeCpp::Friendly;
        }
        if input.attackable_by_summoner {
            return ReputationRankLikeCpp::Neutral;
        }
        if input.same_charmer_or_owner_or_self {
            return ReputationRankLikeCpp::Friendly;
        }
        let Some(reputation_state) = self.cloned_reputation_state_like_cpp() else {
            return ReputationRankLikeCpp::Neutral;
        };
        let reputation_mgr = ReputationMgrLikeCpp::borrowing_like_cpp(&reputation_state);

        if input.self_has_player_owner {
            if let Some(faction_template_store) = self.factions.template_store.as_ref()
                && let Some(target_faction_template) =
                    faction_template_store.get(input.target_faction_template_id)
                && let Some(forced_rank) = reputation_mgr
                    .forced_rank_by_faction_id_like_cpp(u32::from(target_faction_template.faction))
            {
                return forced_rank;
            }
        } else if input.target_has_player_owner
            && let Some(faction_template_store) = self.factions.template_store.as_ref()
            && faction_template_store
                .get(input.self_faction_template_id)
                .is_some()
            && let Some(forced_rank) = input.target_owner_forced_rank_for_self
        {
            return forced_rank;
        }

        if input.self_unit_player_controlled && input.target_unit_player_controlled {
            if input.self_has_player_owner && input.target_has_player_owner {
                if input.same_player_owner {
                    return ReputationRankLikeCpp::Friendly;
                }
                if input.duel_in_progress {
                    return ReputationRankLikeCpp::Hostile;
                }
                if input.same_raid {
                    return ReputationRankLikeCpp::Friendly;
                }
            }

            if input.self_ffa_pvp && input.target_ffa_pvp {
                return ReputationRankLikeCpp::Hostile;
            }

            if input.self_has_player_owner {
                let Some(faction_template_store) = self.factions.template_store.as_ref() else {
                    return self.represented_faction_reaction_to_like_cpp(
                        RepresentedFactionReactionInputLikeCpp {
                            source_faction_template_id: input.self_faction_template_id,
                            target_faction_template_id: input.target_faction_template_id,
                            target_has_player_owner: input.target_has_player_owner,
                            target_player_owner_is_current_session: input
                                .target_player_owner_is_current_session,
                            target_player_contested_pvp: input.target_player_contested_pvp,
                            target_is_unit: input.target_is_unit,
                            target_ignores_reputation: input.target_ignores_reputation,
                        },
                    );
                };
                if let Some(target_faction_template) =
                    faction_template_store.get(input.target_faction_template_id)
                {
                    if let Some(forced_rank) = reputation_mgr.forced_rank_by_faction_id_like_cpp(
                        u32::from(target_faction_template.faction),
                    ) {
                        return forced_rank;
                    }
                    if !input.self_ignores_reputation
                        && let Some(faction_store) = self.factions.store.as_ref()
                        && let Some(target_faction_entry) =
                            faction_store.get(u32::from(target_faction_template.faction))
                        && target_faction_entry.can_have_reputation_like_cpp()
                    {
                        if target_faction_template.is_contested_guard_faction_like_cpp()
                            && input.target_player_contested_pvp
                        {
                            return ReputationRankLikeCpp::Hostile;
                        }
                        if reputation_mgr.is_at_war_with_faction_like_cpp(target_faction_entry) {
                            return ReputationRankLikeCpp::Hostile;
                        }
                        return ReputationRankLikeCpp::Friendly;
                    }
                }
            }
        }

        self.represented_faction_reaction_to_like_cpp(RepresentedFactionReactionInputLikeCpp {
            source_faction_template_id: input.self_faction_template_id,
            target_faction_template_id: input.target_faction_template_id,
            target_has_player_owner: input.target_has_player_owner,
            target_player_owner_is_current_session: input.target_player_owner_is_current_session,
            target_player_contested_pvp: input.target_player_contested_pvp,
            target_is_unit: input.target_is_unit,
            target_ignores_reputation: input.target_ignores_reputation,
        })
    }
}
