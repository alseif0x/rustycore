// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player reputation adapters shared with World.

use crate::session::state::hub_support::player_team_for_race_cpp;
use crate::session::{
    AttackReputationFactionSnapshotLikeCpp, PlayerBootstrapCatalogsLikeCpp,
    ReputationGainSourceLikeCpp,
};
use crate::session_policy::ReputationRatesLikeCpp;
use std::sync::Arc;
use wow_constants::Team;
use wow_data::progression_rewards::ParagonReputationStore;
use wow_data::reputation::{RepSpilloverTemplateStoreLikeCpp, ReputationRewardRateStoreLikeCpp};
use wow_progression::{ReputationMgrLikeCpp, ReputationMgrMutLikeCpp, ReputationMgrRefLikeCpp};

const FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP: i32 = 42_999;
pub const FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP: &[u32] = &[
    942, 935, 936, 1011, 970, 967, 989, 932, 934, 1038, 1077, 1106, 1104, 1090, 1098, 1156, 1073,
    1105, 1119, 1091,
];
pub const FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP: &[u32] = &[
    72, 47, 69, 930, 730, 978, 54, 946, 1037, 1068, 1126, 1094, 1050,
];
pub const FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP: &[u32] = &[
    76, 68, 81, 911, 729, 941, 530, 947, 1052, 1067, 1124, 1064, 1085,
];

impl crate::session::HubMut<'_> {
    pub fn apply_represented_first_login_reputation_with_catalogs_like_cpp(
        &mut self,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        if !player_bootstrap.start_all_reputation {
            return 0;
        }

        let Some(faction_store) = self.catalogs.faction_store().map(Arc::clone) else {
            return 0;
        };
        let friendship_rep_reaction_store = self
            .catalogs
            .friendship_rep_reaction_store()
            .map(Arc::clone);
        let paragon_reputation_store = self.catalogs.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.catalogs.currency_types_store().map(Arc::clone);
        let player_race = self.shared().player_race_like_cpp();
        let player_class = self.shared().player_class_like_cpp();
        let team_factions = match player_team_for_race_cpp(player_race) {
            Team::Horde => FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP,
            _ => FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP,
        };

        let Some((applied, packet)) = self.mutate_reputation_mgr_like_cpp(|mgr| {
            let mut applied = 0usize;
            for faction_id in FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP
                .iter()
                .chain(team_factions.iter())
            {
                let Some(faction_entry) = faction_store.get(*faction_id).cloned() else {
                    continue;
                };
                let outcome = mgr.set_one_faction_reputation_like_cpp(
                    &faction_entry,
                    FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP,
                    false,
                    1.0,
                    friendship_rep_reaction_store.as_deref(),
                    paragon_reputation_store.as_deref(),
                    true,
                    currency_types_store.as_deref(),
                    0,
                    0,
                    player_race,
                    player_class,
                );
                if outcome.applied {
                    applied += 1;
                }
            }
            let packet = (applied > 0).then(|| mgr.set_faction_standing_packet_like_cpp(None));
            (applied, packet)
        }) else {
            return 0;
        };
        if let Some(packet) = packet {
            self.core.send_packet(&packet);
        }

        applied
    }
}

impl crate::session::HubRef<'_> {
    pub fn reputation_price_discount_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> f32 {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template_store) = self.catalogs.factions.template_store.as_ref() else {
            return 1.0;
        };
        let Some(faction_template) = faction_template_store.get(faction_template_id) else {
            return 1.0;
        };
        if faction_template.faction == 0 {
            return 1.0;
        }
        let Some(faction_store) = self.catalogs.factions.store.as_ref() else {
            return 1.0;
        };
        let Some(faction_entry) = faction_store.get(u32::from(faction_template.faction)) else {
            return 1.0;
        };

        // Resolve identity and the friendship store before the canonical
        // manager lock; the session accessors re-enter it.
        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let friendship_rep_reaction_store = self.catalogs.friendship_rep_reaction_store.as_deref();
        let Some(rank) = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.rank_for_faction_entry_like_cpp(
                faction_entry,
                friendship_rep_reaction_store,
                player_race,
                player_class,
            )
        }) else {
            return 1.0;
        };
        if rank <= ReputationRankLikeCpp::Neutral {
            return 1.0;
        }

        1.0 - 0.05 * f32::from(rank.as_u8() - ReputationRankLikeCpp::Neutral.as_u8())
    }

    pub fn canonical_player_reputation_standing_like_cpp(&self, faction_id: u32) -> Option<i32> {
        self.core
            .canonical_player_reputation_standing_like_cpp(faction_id)
    }

    pub fn reputation_reward_rate_for_source_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        faction_id: u32,
    ) -> Option<f32> {
        self.catalogs
            .reputation_reward_rate_for_source_like_cpp(source, faction_id)
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn reputation_reward_rate_for_source_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        faction_id: u32,
    ) -> Option<f32> {
        let rates = self
            .reputation_reward_rate_store()
            .and_then(|store| store.get(faction_id))?;
        let rate = match source {
            ReputationGainSourceLikeCpp::Kill => rates.creature_rate,
            ReputationGainSourceLikeCpp::Quest => rates.quest_rate,
            ReputationGainSourceLikeCpp::DailyQuest => rates.quest_daily_rate,
            ReputationGainSourceLikeCpp::WeeklyQuest => rates.quest_weekly_rate,
            ReputationGainSourceLikeCpp::MonthlyQuest => rates.quest_monthly_rate,
            ReputationGainSourceLikeCpp::RepeatableQuest => rates.quest_repeatable_rate,
            ReputationGainSourceLikeCpp::Spell => rates.spell_rate,
        };
        Some(rate)
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn attack_reputation_faction_snapshot_like_cpp(
        &self,
        creature: &wow_entities::Creature,
    ) -> Option<AttackReputationFactionSnapshotLikeCpp> {
        let faction_template_id = u32::try_from(creature.unit().data().faction_template).ok()?;
        self.factions
            .template_store
            .as_ref()
            .and_then(|store| store.get(faction_template_id))
            .map(|entry| {
                let faction_id = u32::from(entry.faction);
                AttackReputationFactionSnapshotLikeCpp {
                    faction_id,
                    contested_guard: entry.is_contested_guard_faction_like_cpp(),
                    can_have_reputation: self.factions.store.as_ref().and_then(|store| {
                        store
                            .get(faction_id)
                            .map(|faction| faction.can_have_reputation_like_cpp())
                    }),
                }
            })
            .or_else(|| {
                creature
                    .attack_reputation_faction_id_like_cpp()
                    .map(|faction_id| AttackReputationFactionSnapshotLikeCpp {
                        faction_id,
                        contested_guard: creature.is_contested_guard_like_cpp(),
                        can_have_reputation: None,
                    })
            })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_start_all_reputation_like_cpp(&mut self, enabled: bool) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_reputation_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn start_all_reputation_like_cpp(&self) -> bool {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_reputation_like_cpp
    }

    pub fn reputation_reward_rate_store(&self) -> Option<&Arc<ReputationRewardRateStoreLikeCpp>> {
        self.reputation_reward_rate_store.as_ref()
    }
}

impl crate::session::HubRef<'_> {
    /// Reputation rank used by deterministic trainer pricing. Missing/zero
    /// faction references retain C++'s full-price fallback (`Neutral`).
    pub fn trainer_price_reputation_rank_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        self.trainer_npc_interaction_access_like_cpp()
            .trainer_price_reputation_rank_like_cpp(faction_template_id)
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn paragon_reputation_store(&self) -> Option<&Arc<ParagonReputationStore>> {
        self.paragon_reputation_store.as_ref()
    }

    pub fn reputation_spillover_template_store(
        &self,
    ) -> Option<&Arc<RepSpilloverTemplateStoreLikeCpp>> {
        self.reputation_spillover_template_store.as_ref()
    }
}

impl crate::session::HubMut<'_> {
    /// Run one C++ `ReputationMgr` transition against the Player's own state.
    ///
    /// The transition writes through the Player's named reputation owner; no
    /// aggregate is reconstructed and nothing is written back through the
    /// Player's whole gameplay state (#735).
    pub fn mutate_reputation_mgr_like_cpp<R>(
        &mut self,
        operation: impl FnOnce(&mut ReputationMgrMutLikeCpp<'_>) -> R,
    ) -> Option<R> {
        self.core.mutate_reputation_mgr_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.progression.reputation_state_like_cpp,
            operation,
        )
    }
}

impl crate::session::SessionCore {
    pub(in crate::session) fn canonical_player_reputation_standing_like_cpp(
        &self,
        faction_id: u32,
    ) -> Option<i32> {
        self.canonical_player_snapshot_like_cpp(|player| {
            player
                .reputation_like_cpp()
                .factions_like_cpp()
                .find_map(|state| (state.faction_id == faction_id).then_some(state.standing))
                .unwrap_or(0)
        })
    }

    pub(in crate::session) fn mutate_reputation_mgr_with_fixture_like_cpp<R>(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &mut wow_entities::PlayerReputationStateLikeCpp,
        operation: impl FnOnce(&mut ReputationMgrMutLikeCpp<'_>) -> R,
    ) -> Option<R> {
        let mut operation = Some(operation);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let mut manager =
                ReputationMgrLikeCpp::borrowing_mut_like_cpp(player.reputation_mut_like_cpp());
            operation.take().expect("reputation mutation runs once")(&mut manager)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            let mut manager = ReputationMgrLikeCpp::borrowing_mut_like_cpp(fixture);
            return Some(operation.take().expect("reputation mutation is available")(
                &mut manager,
            ));
        }
        None
    }
}

impl crate::session::HubMut<'_> {
    pub fn initialize_reputation_mgr_like_cpp(&mut self) {
        let Some(faction_store) = self.catalogs.factions.store.clone() else {
            return;
        };
        let paragon_reputation_store = self.catalogs.paragon_reputation_store.clone();
        let race = self.shared().player_race_like_cpp();
        let class = self.shared().player_class_like_cpp();
        let _ = self.mutate_reputation_mgr_like_cpp(|mgr| {
            mgr.initialize_like_cpp(
                faction_store.as_ref(),
                paragon_reputation_store.as_deref(),
                race,
                class,
            );
        });
    }
}

impl crate::session::HubRef<'_> {
    /// Run one C++ `ReputationMgr` read against the Player's own state.
    ///
    /// C++ `Player::GetReputationMgr()` hands out a reference to the manager
    /// the Player owns (`Player.h:3116`). This borrows the equivalent canonical
    /// state instead of rebuilding a manager from it (#735).
    pub fn with_reputation_mgr_like_cpp<R>(
        &self,
        operation: impl FnOnce(&ReputationMgrRefLikeCpp<'_>) -> R,
    ) -> Option<R> {
        let mut operation = Some(operation);
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(player.reputation_like_cpp());
            operation.take().expect("reputation operation runs once")(&manager)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(
                &self.fixtures.progression.reputation_state_like_cpp,
            );
            return Some(operation.take().expect("reputation operation is available")(&manager));
        }
        None
    }

    /// Clone the Player's reputation state for a read that outlives the
    /// canonical borrow. This is a read snapshot, never a writable mirror.
    pub(in crate::session) fn cloned_reputation_state_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerReputationStateLikeCpp> {
        self.with_reputation_mgr_like_cpp(|manager| manager.cloned_state_like_cpp())
    }
}

impl crate::session::state::SessionWorldConfig {
    pub fn reputation_rates_like_cpp(&self) -> ReputationRatesLikeCpp {
        self.reputation_rates
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::ProgressionState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn reputation_mgr_like_cpp(&self) -> ReputationMgrRefLikeCpp<'_> {
        ReputationMgrLikeCpp::borrowing_like_cpp(&self.reputation_state_like_cpp)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn reputation_mgr_like_cpp_mut(&mut self) -> ReputationMgrMutLikeCpp<'_> {
        ReputationMgrLikeCpp::borrowing_mut_like_cpp(&mut self.reputation_state_like_cpp)
    }
}
