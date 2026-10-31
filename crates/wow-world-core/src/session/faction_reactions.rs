// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical faction-reaction inputs and Hub adapters shared with World.

use crate::session::{HubRef, NpcInteractionAccessLikeCpp};
use wow_data::progression_rewards::{
    FactionStore, FactionTemplateStore, FriendshipRepReactionStore,
};
use wow_entities::PlayerReputationStateLikeCpp;
use wow_progression::ReputationMgrLikeCpp;

mod rules;
pub use rules::{resolve_faction_reaction, resolve_faction_reaction_with_raid};

#[derive(Clone, Copy)]
pub struct FactionReactionCatalogs<'a> {
    faction_store: Option<&'a FactionStore>,
    faction_template_store: Option<&'a FactionTemplateStore>,
    friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
}

impl<'a> FactionReactionCatalogs<'a> {
    #[must_use]
    pub const fn new(
        faction_store: Option<&'a FactionStore>,
        faction_template_store: Option<&'a FactionTemplateStore>,
        friendship_rep_reaction_store: Option<&'a FriendshipRepReactionStore>,
    ) -> Self {
        Self {
            faction_store,
            faction_template_store,
            friendship_rep_reaction_store,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FactionReactionOwner<'a> {
    reputation: &'a PlayerReputationStateLikeCpp,
    race: u8,
    class: u8,
}

impl<'a> FactionReactionOwner<'a> {
    #[must_use]
    pub const fn new(reputation: &'a PlayerReputationStateLikeCpp, race: u8, class: u8) -> Self {
        Self {
            reputation,
            race,
            class,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactionReactionError {
    UnknownFactionStore,
    UnknownFactionTemplateStore,
    UnknownFriendshipReactionStore,
    UnknownSourcePlayerOwner,
    UnknownTargetPlayerOwner,
    UnknownRaidRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedFactionReactionInputLikeCpp {
    pub source_faction_template_id: u32,
    pub target_faction_template_id: u32,
    pub target_has_player_owner: bool,
    pub target_player_owner_is_current_session: bool,
    pub target_player_contested_pvp: bool,
    pub target_is_unit: bool,
    pub target_ignores_reputation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedGetReactionInputLikeCpp {
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
    pub self_player_contested_pvp: bool,
    pub target_player_contested_pvp: bool,
}

impl crate::session::HubMut<'_> {
    pub fn set_watched_faction_index_like_cpp(&mut self, index: i32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_watched_faction_index_like_cpp(index)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.fixtures.progression.watched_faction_index_like_cpp = index;
        }
    }
}

impl crate::session::HubRef<'_> {
    pub fn resolved_watched_faction_index_like_cpp(&self) -> Option<i32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.watched_faction_index_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.progression.watched_faction_index_like_cpp);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn watched_faction_index_like_cpp(&self) -> i32 {
        self.resolved_watched_faction_index_like_cpp()
            .expect("Player watched-faction owner must resolve")
    }
}

impl crate::session::state::SessionWorldConfig {
    pub const fn reset_schedule_like_cpp(&self) -> wow_instances::ResetSchedule {
        self.reset_schedule_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn represented_faction_reaction_to_like_cpp(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        self.trainer_npc_interaction_access_like_cpp()
            .represented_faction_reaction_to_like_cpp(input)
    }

    pub fn represented_get_reaction_to_like_cpp(
        &self,
        input: RepresentedGetReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        self.trainer_npc_interaction_access_like_cpp()
            .represented_get_reaction_to_like_cpp(input)
    }
}

impl NpcInteractionAccessLikeCpp<'_> {
    /// Resolve the trainer price rank from this selected NPC capability's
    /// faction catalogs and the canonical reputation owner. The manager read
    /// remains at the caller's original price-evaluation point.
    pub fn trainer_price_reputation_rank_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template) = self
            .faction_template_store
            .and_then(|store| store.get(faction_template_id))
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        if faction_template.faction == 0 {
            return ReputationRankLikeCpp::Neutral;
        }
        let Some(faction_entry) = self
            .faction_store
            .and_then(|store| store.get(u32::from(faction_template.faction)))
        else {
            return ReputationRankLikeCpp::Neutral;
        };

        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store;
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(player.reputation_like_cpp());
            manager.rank_for_faction_entry_like_cpp(
                faction_entry,
                friendship_rep_reaction_store,
                player_race,
                player_class,
            )
        });
        if let Some(rank) = canonical {
            return rank;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(self.fixtures.reputation_state);
            return manager.rank_for_faction_entry_like_cpp(
                faction_entry,
                friendship_rep_reaction_store,
                player_race,
                player_class,
            );
        }
        ReputationRankLikeCpp::Neutral
    }

    pub fn represented_faction_reaction_to_like_cpp(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        self.resolve_lower_reaction(input)
    }

    pub fn represented_get_reaction_to_like_cpp(
        &self,
        input: RepresentedGetReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        self.resolve_reaction(input)
    }

    fn resolve_lower_reaction(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let catalogs = FactionReactionCatalogs::new(
            self.faction_store,
            self.faction_template_store,
            self.friendship_rep_reaction_store,
        );
        match rules::resolve_faction_template_reaction(input, None, catalogs) {
            Ok(rank) => rank,
            Err(FactionReactionError::UnknownTargetPlayerOwner)
                if input.target_has_player_owner
                    && input.target_player_owner_is_current_session =>
            {
                self.with_session_reaction_owner(|owner| {
                    rules::resolve_faction_template_reaction(input, Some(owner), catalogs)
                        .unwrap_or(ReputationRankLikeCpp::Neutral)
                })
                .unwrap_or(ReputationRankLikeCpp::Neutral)
            }
            Err(_) => ReputationRankLikeCpp::Neutral,
        }
    }

    fn resolve_reaction(
        &self,
        input: RepresentedGetReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let catalogs = FactionReactionCatalogs::new(
            self.faction_store,
            self.faction_template_store,
            self.friendship_rep_reaction_store,
        );
        let needs_session_owner =
            input.self_has_player_owner || input.target_player_owner_is_current_session;

        match resolve_faction_reaction(input, None, None, catalogs) {
            Ok(rank) => return rank,
            Err(
                FactionReactionError::UnknownSourcePlayerOwner
                | FactionReactionError::UnknownTargetPlayerOwner,
            ) if needs_session_owner => {}
            Err(_) => return ReputationRankLikeCpp::Neutral,
        }

        self.with_session_reaction_owner(|owner| {
            resolve_faction_reaction(
                input,
                input.self_has_player_owner.then_some(owner),
                input
                    .target_player_owner_is_current_session
                    .then_some(owner),
                catalogs,
            )
            .unwrap_or(ReputationRankLikeCpp::Neutral)
        })
        .unwrap_or(ReputationRankLikeCpp::Neutral)
    }

    fn with_session_reaction_owner<R>(
        &self,
        f: impl FnOnce(FactionReactionOwner<'_>) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let owner = FactionReactionOwner::new(
                self.fixtures.reputation_state,
                *self.fixtures.player_race,
                *self.fixtures.player_class,
            );
            return Some(f(owner));
        }

        self.core.with_owned_player_like_cpp(|player| {
            f(FactionReactionOwner::new(
                player.reputation_like_cpp(),
                player.race_like_cpp(),
                player.class_like_cpp(),
            ))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackReputationFactionSnapshotLikeCpp {
    pub faction_id: u32,
    pub contested_guard: bool,
    pub can_have_reputation: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationGainSourceLikeCpp {
    Kill,
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
    Spell,
}
