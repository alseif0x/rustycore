// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical Player roles used by the quest-reward application.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use wow_entities::PlayerCurrency;

use super::owned_inventory::OwnedInventoryAccessLikeCpp;
use crate::session::MAX_SPECIALIZATIONS_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::StatsFixtureRefs;
use crate::session::{
    PacketPublicationAccessLikeCpp, PlayerGroupOwnerAccessLikeCpp,
    PlayerMoneyTransactionSessionAccessLikeCpp, PlayerStatsAccessLikeCpp, SessionCatalogs,
    SessionCore, SessionState, SessionWorldConfig,
};
use wow_progression::ReputationMgrLikeCpp;

/// Borrowed, operation-specific access to the canonical quest-reward owner.
pub struct QuestRewardPlayerAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_class: &'a u8,
}

/// Selected inputs for one Player level transition and talent-point refresh.
/// The test-fixture form requires every World-owned input at construction.
pub struct PlayerLevelTransitionAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a mut u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    gray_level_overrides: &'a HashMap<u8, u8>,
    #[cfg(any(test, feature = "test-fixtures"))]
    talent_groups: &'a [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
    #[cfg(any(test, feature = "test-fixtures"))]
    active_talent_group: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_character_points: &'a mut i32,
}

/// Shared-borrow canonical currency capability; unlike the larger mutable
/// reward owner, the existing Inventory getter can construct it from HubRef.
pub struct OwnedPlayerCurrencyAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build the selected Player capability without resolving or copying the Player.
    pub fn quest_reward_player_access_like_cpp<'a>(
        &'a mut self,
        #[cfg(any(test, feature = "test-fixtures"))] player_race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] player_class: &'a u8,
    ) -> QuestRewardPlayerAccessLikeCpp<'a> {
        QuestRewardPlayerAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_class,
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn player_level_transition_access_like_cpp<'a>(
        &'a mut self,
        player_class: &'a u8,
        player_level: &'a mut u8,
        gray_level_overrides: &'a HashMap<u8, u8>,
        talent_groups: &'a [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
        active_talent_group: &'a u8,
        player_character_points: &'a mut i32,
    ) -> PlayerLevelTransitionAccessLikeCpp<'a> {
        PlayerLevelTransitionAccessLikeCpp {
            core: self,
            player_class,
            player_level,
            gray_level_overrides,
            talent_groups,
            active_talent_group,
            player_character_points,
        }
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    fn player_level_transition_access_like_cpp(
        &mut self,
    ) -> PlayerLevelTransitionAccessLikeCpp<'_> {
        PlayerLevelTransitionAccessLikeCpp { core: self }
    }

    pub fn owned_player_currency_access_like_cpp(&self) -> OwnedPlayerCurrencyAccessLikeCpp<'_> {
        OwnedPlayerCurrencyAccessLikeCpp { core: self }
    }
}

impl OwnedPlayerCurrencyAccessLikeCpp<'_> {
    pub fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().currencies.clone())
    }

    pub fn set_player_currencies_like_cpp(
        &self,
        currencies: &HashMap<u32, PlayerCurrency>,
    ) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_currencies_like_cpp(currencies.clone());
            })
            .is_some()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}

impl QuestRewardPlayerAccessLikeCpp<'_> {
    /// Resolve the selected quest skill through the canonical skill records,
    /// retaining the World fixture fallback only for an absent owner.
    pub fn resolved_quest_skill_value_like_cpp(
        &self,
        skill_id: u16,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_records: &HashMap<
            u16,
            crate::session::RepresentedPlayerSkillLikeCpp,
        >,
    ) -> Option<u16> {
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_records,
            )?;
        Some(
            crate::session::represented_skill_values_from_records_like_cpp(&records)
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }

    /// Resolve one faction's standing through the selected manager without
    /// exposing Core or retaining a manager reference beyond this read.
    pub fn quest_reputation_for_faction_like_cpp(
        &self,
        faction: &wow_data::progression_rewards::FactionEntry,
        race: u8,
        class: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_state: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> Option<i32> {
        quest_reputation_for_faction_like_cpp(
            self.core,
            faction,
            race,
            class,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_state,
        )
    }

    pub fn reward_reputation_gray_level_like_cpp(
        &self,
        level: u8,
        #[cfg(any(test, feature = "test-fixtures"))] overrides: &HashMap<u8, u8>,
    ) -> u8 {
        crate::session::progression_adapters::gray_level_for_script_like_cpp(
            level,
            #[cfg(any(test, feature = "test-fixtures"))]
            Some(overrides),
        )
    }

    pub fn canonical_reward_reputation_standing_like_cpp(&self, faction_id: u32) -> Option<i32> {
        self.core
            .canonical_player_reputation_standing_like_cpp(faction_id)
    }

    pub fn mutate_reward_reputation_mgr_like_cpp<R>(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &mut wow_entities::PlayerReputationStateLikeCpp,
        operation: impl FnOnce(&mut wow_progression::ReputationMgrMutLikeCpp<'_>) -> R,
    ) -> Option<R> {
        self.core.mutate_reputation_mgr_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture,
            operation,
        )
    }

    pub fn item_planning_condition_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: crate::session::PlayerConditionFixtureRefsLikeCpp<'a>,
    ) -> crate::session::PlayerConditionAccessLikeCpp<'a> {
        self.core
            .player_condition_access_with_selected_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            )
    }

    pub fn item_planning_realm_id_like_cpp(&self) -> u16 {
        self.core
            .inventory_valuation_access_like_cpp()
            .realm_id_like_cpp()
    }

    pub fn send_equip_error_like_cpp(
        &self,
        result: wow_constants::InventoryResult,
        item1: Option<wow_core::ObjectGuid>,
        item2: Option<wow_core::ObjectGuid>,
        required_level: u32,
        limit_category: u32,
    ) {
        self.core
            .send_equip_error(result, item1, item2, required_level, limit_category);
    }

    pub fn reborrow_like_cpp<'a>(&'a mut self) -> QuestRewardPlayerAccessLikeCpp<'a> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let player_race = self.player_race;
        #[cfg(any(test, feature = "test-fixtures"))]
        let player_class = self.player_class;
        let core = &mut *self.core;

        QuestRewardPlayerAccessLikeCpp {
            core,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_class,
        }
    }

    /// Reborrow the selected owner with all level-transition fixture inputs
    /// required at compile time.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_level_transition_access_like_cpp<'a>(
        &'a mut self,
        player_level: &'a mut u8,
        gray_level_overrides: &'a HashMap<u8, u8>,
        talent_groups: &'a [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
        active_talent_group: &'a u8,
        player_character_points: &'a mut i32,
    ) -> PlayerLevelTransitionAccessLikeCpp<'a> {
        self.core.player_level_transition_access_like_cpp(
            self.player_class,
            player_level,
            gray_level_overrides,
            talent_groups,
            active_talent_group,
            player_character_points,
        )
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn player_level_transition_access_like_cpp(
        &mut self,
    ) -> PlayerLevelTransitionAccessLikeCpp<'_> {
        self.core.player_level_transition_access_like_cpp()
    }

    /// Borrow the selected stats inputs at the level-up phase without
    /// resolving them when the enclosing reward context is constructed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn stats_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        player_level: &'a u8,
        fixtures: StatsFixtureRefs<'a>,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        self.core.player_stats_access_with_fixture_refs_like_cpp(
            catalogs,
            config,
            &*self.player_race,
            &*self.player_class,
            player_level,
            fixtures,
        )
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn stats_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        self.core.player_stats_access_like_cpp(catalogs, config)
    }

    /// Borrow the session's finite packet-publication capability for this
    /// reward operation without exposing its Core owner.
    pub fn packet_publication_access_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn inventory_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn currency_like_cpp(&self) -> OwnedPlayerCurrencyAccessLikeCpp<'_> {
        self.core.owned_player_currency_access_like_cpp()
    }

    pub fn xp_gain_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
    ) -> crate::session::CoreXPGainAccessLikeCpp<'a> {
        self.core.xp_gain_access_like_cpp(catalogs, config)
    }

    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    pub fn session_is_logged_in_like_cpp(&self) -> bool {
        self.core.state == SessionState::LoggedIn
    }

    pub fn recruiter_id_like_cpp(&self) -> u32 {
        self.core.recruiter_id_like_cpp()
    }

    pub fn player_group_owner_access_like_cpp(&self) -> PlayerGroupOwnerAccessLikeCpp<'_> {
        self.core.player_group_owner_access_like_cpp()
    }

    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_level_like_cpp(&self, player_level: &u8) -> u8 {
        self.core.player_level_with_fixture_like_cpp(player_level)
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn player_level_like_cpp(&self) -> u8 {
        self.core.player_level_with_fixture_like_cpp()
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_race_with_fixture_like_cpp(self.player_race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_class_with_fixture_like_cpp(self.player_class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    /// C++ `Player::HasPlayerFlag` for this reward owner's canonical incarnation.
    pub fn canonical_player_has_player_flag_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
        flag: u32,
    ) -> Option<bool> {
        self.core
            .canonical_player_has_player_flag_like_cpp(guid, flag)
    }

    /// Resolve one faction's standing through the selected manager, keeping the
    /// fixture State fallback at the caller's phase.
    pub fn reputation_for_faction_like_cpp(
        &self,
        faction: &wow_data::progression_rewards::FactionEntry,
        race: u8,
        class: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_state: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> Option<i32> {
        quest_reputation_for_faction_like_cpp(
            self.core,
            faction,
            race,
            class,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_state,
        )
    }
}

pub(in crate::session::canonical_access) fn quest_reputation_for_faction_like_cpp(
    core: &SessionCore,
    faction: &wow_data::progression_rewards::FactionEntry,
    race: u8,
    class: u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_state: &wow_entities::PlayerReputationStateLikeCpp,
) -> Option<i32> {
    let canonical = core.with_owned_player_like_cpp(|player| {
        let manager = ReputationMgrLikeCpp::borrowing_like_cpp(player.reputation_like_cpp());
        manager.reputation_for_faction_like_cpp(faction, race, class)
    });
    if canonical.is_some() {
        return canonical;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if core.player_handle_like_cpp.is_none() {
        let manager = ReputationMgrLikeCpp::borrowing_like_cpp(fixture_state);
        return Some(manager.reputation_for_faction_like_cpp(faction, race, class));
    }
    None
}

impl PlayerLevelTransitionAccessLikeCpp<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    fn player_level_like_cpp(&self) -> u8 {
        self.core
            .player_level_with_fixture_like_cpp(self.player_level)
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    fn player_level_like_cpp(&self) -> u8 {
        self.core.player_level_with_fixture_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn player_class_like_cpp(&self) -> u8 {
        self.core
            .player_class_with_fixture_like_cpp(self.player_class)
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    fn player_class_like_cpp(&self) -> u8 {
        self.core.player_class_with_fixture_like_cpp()
    }

    /// Apply the Player level/gray-level transition and refresh available talent points.
    pub fn set_player_level_and_refresh_talent_points_like_cpp(
        &mut self,
        level: u8,
        catalogs: &SessionCatalogs,
        world_test_consumer: bool,
        fixture_quest_talent_points: impl Iterator<Item = u32>,
    ) {
        let gray_level = crate::session::progression_adapters::gray_level_for_script_like_cpp(
            level,
            #[cfg(any(test, feature = "test-fixtures"))]
            Some(self.gray_level_overrides),
        );
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_level_and_gray_level_like_cpp(level, gray_level);
            })
            .is_some();
        if !canonical {
            if !world_test_consumer && self.core.player_handle_like_cpp.is_some() {
                return;
            }
            self.core
                .player_identity_bootstrap_like_cpp
                .get_or_insert_default()
                .level = level;
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                *self.player_level = level;
            }
        }

        self.refresh_represented_talent_points_like_cpp(
            catalogs,
            world_test_consumer,
            fixture_quest_talent_points,
        );
    }

    /// Recalculate canonical CharacterPoints, using the World test fallback only when requested.
    pub fn refresh_represented_talent_points_like_cpp(
        &mut self,
        catalogs: &SessionCatalogs,
        world_test_consumer: bool,
        fixture_quest_talent_points: impl Iterator<Item = u32>,
    ) {
        if world_test_consumer && self.core.player_handle_like_cpp.is_none() {
            let Some(spent) = self.represented_spent_talent_points_count_like_cpp(catalogs) else {
                return;
            };
            let base_points = catalogs
                .num_talents_at_level_store()
                .map(|store| {
                    store.num_talents_at_level_like_cpp(
                        u32::from(self.player_level_like_cpp()),
                        self.player_class_like_cpp(),
                    )
                })
                .unwrap_or(0);
            let canonical_quest_points = self.core.with_owned_player_like_cpp(|player| {
                player.gameplay_state().quest_rewarded_talent_points
            });
            let quest_points = match canonical_quest_points {
                Some(points) => points,
                None => fixture_quest_talent_points.sum(),
            };
            let available = (base_points + quest_points)
                .saturating_sub(spent)
                .min(i32::MAX as u32) as i32;
            self.set_player_character_points_after_level_change_like_cpp(available);
            return;
        }

        let base_points = catalogs
            .num_talents_at_level_store()
            .map(|store| {
                store.num_talents_at_level_like_cpp(
                    u32::from(self.player_level_like_cpp()),
                    self.player_class_like_cpp(),
                )
            })
            .unwrap_or(0);
        let points = self.core.with_owned_player_mut_like_cpp(|player| {
            player.refresh_represented_talent_points_like_cpp(base_points, |talent_id, rank| {
                catalogs
                    .represented_talent_info_like_cpp(talent_id, rank)
                    .is_some()
            })
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if world_test_consumer {
            if let Some(points) = points {
                *self.player_character_points = points;
            }
        }
    }

    fn represented_spent_talent_points_count_like_cpp(
        &self,
        catalogs: &SessionCatalogs,
    ) -> Option<u32> {
        // The former fixture snapshot cloned every talent/glyph field before
        // this tally. The count uses only the selected talent map and active
        // group; the snapshot's remaining assignments are independent fields.
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let runtime = player.talent_runtime_like_cpp();
            let group = runtime
                .active_group_like_cpp()
                .min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
            Self::spent_talent_points_in_group_like_cpp(
                runtime.talent_group_like_cpp(group),
                catalogs,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let group = (*self.active_talent_group).min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
            return Some(Self::spent_talent_points_in_group_like_cpp(
                self.talent_groups.get(usize::from(group)),
                catalogs,
            ));
        }
        canonical
    }

    fn spent_talent_points_in_group_like_cpp(
        talents: Option<&BTreeMap<u32, u8>>,
        catalogs: &SessionCatalogs,
    ) -> u32 {
        talents
            .into_iter()
            .flat_map(|talents| talents.iter())
            .filter(|(talent_id, rank)| {
                catalogs
                    .represented_talent_info_like_cpp(**talent_id, **rank)
                    .is_some()
            })
            .map(|(_, rank)| u32::from(*rank) + 1)
            .sum()
    }

    fn set_player_character_points_after_level_change_like_cpp(&mut self, points: i32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_character_points_like_cpp(points);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            *self.player_character_points = points;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }
}

impl QuestRewardPlayerAccessLikeCpp<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_can_delay_teleport_like_cpp(
        &self,
        teleport_fixture: &crate::session::state::TeleportState,
    ) -> bool {
        self.core
            .represented_can_delay_teleport_with_fixture_like_cpp(teleport_fixture)
    }

    pub fn allocate_item_instance_guids_with_generator_like_cpp(
        &self,
        generator: &wow_core::ObjectGuidGenerator,
        count: usize,
    ) -> Option<Vec<(u64, wow_core::ObjectGuid)>> {
        self.core
            .allocate_item_instance_guids_with_generator_like_cpp(generator, count)
    }

    pub fn set_can_delay_teleport_like_cpp(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        teleport_fixture: &mut crate::session::state::TeleportState,
        can_delay: bool,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .set_can_delay_teleport_with_fixture_like_cpp(teleport_fixture, can_delay)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .set_can_delay_teleport_with_fixture_like_cpp(can_delay)
        }
    }

    pub fn quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        self.core
            .player_registry_hydration_access_like_cpp()
            .owned_player_quest_gameplay_snapshot_like_cpp()
    }

    pub fn quest_objective_access_like_cpp(
        &self,
    ) -> crate::session::QuestObjectiveAccessLikeCpp<'_> {
        self.core.quest_objective_access_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    pub fn settle_rewarded_quest_like_cpp(&mut self, quest_id: u32, repeatable: bool) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                let quests = &mut player.gameplay_state_mut().quests;
                quests.remove_status_like_cpp(quest_id);
                if !repeatable {
                    quests.set_rewarded_like_cpp(quest_id, true);
                }
            })
            .is_some()
    }

    pub fn record_daily_quest_reward_recurrence_like_cpp(
        &mut self,
        quest_id: u32,
        now_secs: i64,
        is_df_quest: bool,
    ) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                let quests = &mut player.gameplay_state_mut().quests;
                quests.set_last_daily_quest_time_secs_like_cpp(now_secs);
                if is_df_quest {
                    quests.set_df_quest_like_cpp(quest_id, true);
                } else {
                    quests.set_daily_like_cpp(quest_id, true);
                }
            })
            .is_some()
    }

    pub fn record_weekly_quest_reward_recurrence_like_cpp(&mut self, quest_id: u32) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .gameplay_state_mut()
                    .quests
                    .set_weekly_like_cpp(quest_id, true);
            })
            .is_some()
    }

    pub fn record_monthly_quest_reward_recurrence_like_cpp(&mut self, quest_id: u32) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .gameplay_state_mut()
                    .quests
                    .set_monthly_like_cpp(quest_id, true);
            })
            .is_some()
    }

    pub fn record_seasonal_quest_reward_recurrence_like_cpp(
        &mut self,
        event_id: u16,
        quest_id: u32,
        completed_at: u64,
    ) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                player.gameplay_state_mut().quests.set_seasonal_like_cpp(
                    event_id,
                    quest_id,
                    completed_at,
                );
            })
            .is_some()
    }

    pub fn add_quest_rewarded_talent_points_like_cpp(&self, points: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.add_quest_rewarded_talent_points_like_cpp(points);
            })
            .is_some()
    }

    pub fn clear_quest_reward_end_time_like_cpp(&mut self, quest_id: u32) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                let Some(status) = player
                    .gameplay_state_mut()
                    .quests
                    .status_mut_like_cpp(quest_id)
                else {
                    return false;
                };
                if status.end_time_secs <= 0 {
                    return false;
                }
                status.end_time_secs = 0;
                true
            })
            .unwrap_or(false)
    }

    pub async fn notify_game_event_quest_complete_like_cpp(
        &self,
        quest_id: u32,
    ) -> crate::session::mailbox::GameEventQuestCompleteClientOutcomeLikeCpp {
        use crate::session::mailbox::{
            GameEventQuestCompleteClientOutcomeLikeCpp, GameEventQuestCompleteCommandLikeCpp,
        };

        let Some(sender) = self.core.directory.game_event_quest_complete_tx.as_ref() else {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SenderMissing { quest_id };
        };
        let (response_tx, response_rx) = flume::bounded(1);
        if sender
            .try_send(GameEventQuestCompleteCommandLikeCpp {
                quest_id,
                response_tx,
            })
            .is_err()
        {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SendFailed { quest_id };
        }
        match tokio::time::timeout(Duration::from_millis(250), response_rx.recv_async()).await {
            Ok(Ok(response)) => GameEventQuestCompleteClientOutcomeLikeCpp::Ok(response),
            Ok(Err(_)) => {
                GameEventQuestCompleteClientOutcomeLikeCpp::ResponseChannelClosed { quest_id }
            }
            Err(_) => GameEventQuestCompleteClientOutcomeLikeCpp::ResponseTimeout { quest_id },
        }
    }

    pub fn money_transaction_access_like_cpp(
        &mut self,
    ) -> PlayerMoneyTransactionSessionAccessLikeCpp<'_> {
        self.core.player_money_transaction_access_like_cpp()
    }

    pub fn quarantine_like_cpp(&mut self, reason: &str) {
        self.core.kick(reason);
    }
}
