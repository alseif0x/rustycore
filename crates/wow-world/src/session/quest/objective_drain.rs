// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the App quest-objective drain (#1263 F4 LootMoney
//! dependency 8) and the reward fixture split it shares with the App reward.

use super::*;

/// Test-build inputs of the App reward operation, split from the session
/// fixtures once so the reward and the objective drain lend the same fields.
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) struct QuestRewardFixtureRefsLikeCpp<'a> {
    pub(crate) xp: wow_world_quest::QuestXpGainFixtureRefsLikeCpp<'a>,
    pub(crate) planning: wow_world_quest::QuestRewardItemPlanningFixtureRefsLikeCpp<'a>,
    pub(crate) reputation: wow_world_quest::QuestRewardReputationFixtureRefsLikeCpp<'a>,
    pub(crate) teleport: &'a mut wow_world_core::session::state::TeleportState,
    pub(crate) transport:
        &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    pub(crate) vehicle_and_pet: (
        &'a Option<wow_entities::Vehicle>,
        &'a Option<i32>,
        &'a Option<u32>,
        &'a Option<ObjectGuid>,
    ),
    pub(crate) race: &'a u8,
    pub(crate) class: &'a u8,
    pub(crate) level: &'a mut u8,
    pub(crate) game_master: &'a bool,
    pub(crate) faction_template: &'a Option<u32>,
    pub(crate) position: &'a Option<wow_core::Position>,
    pub(crate) spell_state: &'a wow_world_spell::SessionSpellState,
}

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) fn quest_reward_fixture_refs_like_cpp<'a>(
    fixtures: &'a mut crate::session::state::SessionFixtures,
    spell_state: &'a wow_world_spell::SessionSpellState,
    instances: &'a wow_world_instances::InstanceState,
) -> QuestRewardFixtureRefsLikeCpp<'a> {
    let xp = wow_world_quest::QuestXpGainFixtureRefsLikeCpp::new_like_cpp(
        wow_world_core::session::CoreXPGainFixtureRefsLikeCpp::new_like_cpp(
            &mut fixtures.progression.player_xp,
            &mut fixtures.progression.player_next_level_xp,
            &fixtures.movement.player_position,
            &fixtures.battleground.player_battleground_type_id_like_cpp,
            &fixtures.battleground.player_battleground_map_id_like_cpp,
            &fixtures
                .battleground
                .represented_battleground_status_like_cpp,
            &fixtures
                .battleground
                .represented_battleground_queue_slots_like_cpp,
            &fixtures
                .battleground
                .represented_arena_team_id_invited_like_cpp,
            &mut fixtures.progression.rest_mgr_test_fixture_like_cpp,
            &fixtures.auras.player_aura_authority_complete_like_cpp,
            &fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp,
            &fixtures.auras.visible_auras,
            &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
        ),
        &mut fixtures.progression.player_character_points_like_cpp,
        &fixtures
            .progression
            .represented_gray_level_script_overrides_like_cpp,
        &fixtures.progression.represented_talents_like_cpp,
        &fixtures
            .progression
            .represented_active_talent_group_like_cpp,
        wow_world_core::session::StatsFixtureRefs::new_like_cpp(
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut fixtures.combat.player_health_like_cpp,
                &mut fixtures.combat.player_max_health_like_cpp,
                &mut fixtures.combat.player_alive_like_cpp,
                &mut fixtures.combat.represented_player_powers_like_cpp[0],
                &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                &fixtures.auras.represented_shapeshift_form_like_cpp,
                &fixtures.auras.player_aura_authority_complete_like_cpp,
                &fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &fixtures.auras.visible_auras,
                &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            ),
        ),
    );
    let planning = wow_world_quest::QuestRewardItemPlanningFixtureRefsLikeCpp::new_like_cpp(
        &fixtures.identity.player_race,
        &fixtures.identity.player_class,
        &fixtures.identity.player_gender,
        &fixtures
            .progression
            .represented_primary_specialization_id_like_cpp,
        &fixtures.movement.player_position,
        &fixtures.identity.player_zone_id_like_cpp,
        &fixtures.identity.player_area_id_like_cpp,
        &fixtures
            .identity
            .player_zone_area_authority_complete_like_cpp,
        &fixtures.combat.player_pvp_hostile_like_cpp,
        &fixtures.combat.player_pvp_end_timer_like_cpp,
        &fixtures.combat.player_contested_pvp_timer_like_cpp,
        &fixtures.identity.represented_is_outdoors_like_cpp,
        &fixtures.vehicles.taxi_destinations_like_cpp,
        &fixtures.vehicles.taxi_flight_state_like_cpp,
        &fixtures.vehicles.taxi_unit_flags_like_cpp,
        &fixtures.vehicles.taxi_mounted_like_cpp,
        &fixtures.auras.visible_auras,
        &fixtures.auras.player_aura_authority_complete_like_cpp,
        &fixtures
            .auras
            .player_spell_hit_aura_authority_tombstoned_like_cpp,
        &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_like_cpp,
        spell_state,
        instances,
        &fixtures.battleground,
        &fixtures
            .battleground
            .represented_battleground_status_like_cpp,
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_complete_like_cpp,
        &fixtures.combat.in_combat,
    );
    let reputation = wow_world_quest::QuestRewardReputationFixtureRefsLikeCpp::new_like_cpp(
        &mut fixtures.progression.reputation_state_like_cpp,
        &fixtures
            .progression
            .represented_gray_level_script_overrides_like_cpp,
        &fixtures.movement.player_position,
        &fixtures.auras.player_aura_authority_complete_like_cpp,
        &fixtures
            .auras
            .player_spell_hit_aura_authority_tombstoned_like_cpp,
        &fixtures.auras.visible_auras,
        &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
    );
    QuestRewardFixtureRefsLikeCpp {
        xp,
        planning,
        reputation,
        teleport: &mut fixtures.teleport,
        transport: &fixtures.vehicles.player_transport_login_state_like_cpp,
        vehicle_and_pet: (
            &fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &fixtures.pets.represented_pet_guid_like_cpp,
        ),
        race: &fixtures.identity.player_race,
        class: &fixtures.identity.player_class,
        level: &mut fixtures.identity.player_level,
        game_master: &fixtures.combat.player_game_master_like_cpp,
        faction_template: &fixtures.identity.player_faction_template_like_cpp,
        position: &fixtures.movement.player_position,
        spell_state,
    }
}

/// One operation of the App quest-objective owner
/// (`wow-world-quest/src/quest/objectives.rs`): C++
/// `Player::UpdateQuestObjectiveProgress` and `Player::CompleteQuest`. The
/// World shell only lends the selected owners; it keeps no copy of either body.
pub(crate) enum QuestObjectiveOperationLikeCpp<'q> {
    /// Recorded money/currency/reputation events, in order.
    Drain,
    /// Storing-value objectives (kill, talk-to, gameobject, player kill).
    StoringValue {
        objective_type: u8,
        object_id: i32,
        add_count: i32,
        credit_guid: ObjectGuid,
    },
    /// Storing-flag objectives (criteria tree).
    #[cfg(test)]
    StoringFlag {
        objective_type: u8,
        object_id: i32,
        add_count: i32,
    },
    /// Completion of a quest whose objectives are met, ignoring one objective.
    Complete {
        quest: &'q wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    },
}

impl WorldSession {
    /// C++ `Player::MoneyChanged` / `CurrencyChanged` / `ReputationChanged` →
    /// `UpdateQuestObjectiveProgress` (`Player.cpp:16162`–`:16176`), drained in
    /// recorded order by the App owner
    /// `QuestObjectiveProgressCx::drain_represented_quest_objective_progress_like_cpp`,
    /// whose completions run the `SendQuestUpdate` visibility refresh themselves.
    pub(crate) async fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            QuestObjectiveOperationLikeCpp::Drain,
        )
        .await;
    }

    /// Run one App quest-objective operation; only `Complete` reports a result.
    pub(crate) async fn quest_objective_operation_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        operation: QuestObjectiveOperationLikeCpp<'_>,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut fixtures = quest_reward_fixture_refs_like_cpp(
            &mut self.fixtures,
            &self.spell_state,
            &self.instances,
        );
        let player = self.core.quest_reward_player_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.race,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.class,
        );
        let mut reward = wow_world_quest::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.level,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
            &self.module_hooks,
        );
        let mut progress = wow_world_quest::QuestObjectiveProgressCx::new(
            &mut reward,
            &self.loot,
            &self.world_entities,
            item_guid_generator,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut fixtures.reputation,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut fixtures.xp,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.teleport,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.planning,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.game_master,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_quest::QuestObjectiveRegistryFixtureRefsLikeCpp::new(
                fixtures.spell_state,
                fixtures.position,
                fixtures.faction_template,
                fixtures.transport,
                fixtures.vehicle_and_pet.0,
                fixtures.vehicle_and_pet.1,
                fixtures.vehicle_and_pet.2,
                fixtures.vehicle_and_pet.3,
            ),
        );
        match operation {
            QuestObjectiveOperationLikeCpp::Drain => {
                progress
                    .drain_represented_quest_objective_progress_like_cpp()
                    .await;
                true
            }
            QuestObjectiveOperationLikeCpp::StoringValue {
                objective_type,
                object_id,
                add_count,
                credit_guid,
            } => {
                progress
                    .update_storing_value_quest_objective_progress_like_cpp(
                        objective_type,
                        object_id,
                        add_count,
                        credit_guid,
                    )
                    .await;
                true
            }
            #[cfg(test)]
            QuestObjectiveOperationLikeCpp::StoringFlag {
                objective_type,
                object_id,
                add_count,
            } => {
                progress
                    .update_storing_flag_quest_objective_progress_like_cpp(
                        objective_type,
                        object_id,
                        add_count,
                    )
                    .await;
                true
            }
            QuestObjectiveOperationLikeCpp::Complete {
                quest,
                ignored_objective_id,
            } => {
                progress
                    .complete_represented_quest_after_objective_like_cpp(
                        quest,
                        ignored_objective_id,
                    )
                    .await
            }
        }
    }
}
