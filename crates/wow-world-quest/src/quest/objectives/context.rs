// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed inputs for the selected objective-drain operation.

use super::super::QuestRewardCx;
#[cfg(any(test, feature = "test-fixtures"))]
use super::super::QuestRewardReputationFixtureRefsLikeCpp;
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_world_entities::WorldEntitiesState;
use wow_world_loot::LootState;

/// Test-only inputs consumed by the existing registry publication phase.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct QuestObjectiveRegistryFixtureRefsLikeCpp<'a> {
    pub(super) spell_state: &'a wow_world_spell::SessionSpellState,
    pub(super) position: &'a Option<wow_core::Position>,
    pub(super) faction_template: &'a Option<u32>,
    pub(super) transport:
        &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    pub(super) mount_vehicle: &'a Option<wow_entities::Vehicle>,
    pub(super) vehicle_seat_flags: &'a Option<i32>,
    pub(super) vehicle_seat_id: &'a Option<u32>,
    pub(super) pet_guid: &'a Option<ObjectGuid>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> QuestObjectiveRegistryFixtureRefsLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spell_state: &'a wow_world_spell::SessionSpellState,
        position: &'a Option<wow_core::Position>,
        faction_template: &'a Option<u32>,
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        mount_vehicle: &'a Option<wow_entities::Vehicle>,
        vehicle_seat_flags: &'a Option<i32>,
        vehicle_seat_id: &'a Option<u32>,
        pet_guid: &'a Option<ObjectGuid>,
    ) -> Self {
        Self {
            spell_state,
            position,
            faction_template,
            transport,
            mount_vehicle,
            vehicle_seat_flags,
            vehicle_seat_id,
            pet_guid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestObjectiveCompletionSpellLikeCpp {
    pub quest_id: u32,
    pub objective_id: u32,
    pub spell_id: u32,
    pub target_guid: ObjectGuid,
    pub completion_state_changed: bool,
}

pub fn completion_spell_for_objective_like_cpp(
    store: &wow_data::quest::QuestStore,
    quest_id: u32,
    objective_id: u32,
    target_guid: Option<ObjectGuid>,
    completion_state_changed: bool,
) -> Option<QuestObjectiveCompletionSpellLikeCpp> {
    let spell_id = store
        .get(quest_id)
        .and_then(|quest| {
            quest
                .objectives
                .iter()
                .find(|objective| objective.id == objective_id)
        })
        .and_then(|objective| objective.completion_effect.as_ref())
        .and_then(|effect| effect.spell_id)
        .filter(|spell_id| *spell_id != 0)?;
    Some(QuestObjectiveCompletionSpellLikeCpp {
        quest_id,
        objective_id,
        spell_id,
        target_guid: target_guid?,
        completion_state_changed,
    })
}

pub fn objective_updates_phase_shift_like_cpp(
    store: &wow_data::quest::QuestStore,
    quest_id: u32,
    objective_id: u32,
) -> bool {
    store
        .get(quest_id)
        .and_then(|quest| {
            quest
                .objectives
                .iter()
                .find(|objective| objective.id == objective_id)
        })
        .and_then(|objective| objective.completion_effect.as_ref())
        .is_some_and(|effect| effect.update_phase_shift)
}

pub fn objective_updates_zone_auras_like_cpp(
    store: &wow_data::quest::QuestStore,
    quest_id: u32,
    objective_id: u32,
) -> bool {
    store
        .get(quest_id)
        .and_then(|quest| {
            quest
                .objectives
                .iter()
                .find(|objective| objective.id == objective_id)
        })
        .and_then(|objective| objective.completion_effect.as_ref())
        .is_some_and(|effect| effect.update_zone_auras)
}

/// A borrowed view of the reward operation, the existing loot publication state
/// and the represented world entities the completion visibility refresh scans.
///
/// Objective completions reborrow the same selected Core owner for their nested reward;
/// this context never stores a World session or a callback into one.
pub struct QuestObjectiveProgressCx<'cx, 'session> {
    pub(in crate::quest) reward: &'cx mut QuestRewardCx<'session>,
    pub(super) loot: &'cx LootState,
    pub(in crate::quest) world_entities: &'cx WorldEntitiesState,
    pub(super) item_guid_generator: &'cx ObjectGuidGenerator,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) reputation_fixture: &'cx mut QuestRewardReputationFixtureRefsLikeCpp<'session>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) xp_fixtures: &'cx mut super::super::reward::QuestXpGainFixtureRefsLikeCpp<'session>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) teleport_fixture: &'cx mut wow_world_core::session::state::TeleportState,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) item_planning_fixtures:
        &'cx super::super::reward::QuestRewardItemPlanningFixtureRefsLikeCpp<'session>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::quest) player_game_master_fixture: &'cx bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) registry_fixtures: QuestObjectiveRegistryFixtureRefsLikeCpp<'cx>,
    pub(super) deferred_completion_spell: Option<QuestObjectiveCompletionSpellLikeCpp>,
    pub(super) completion_state_refreshed: bool,
    pub(super) update_zone_auras: bool,
    pub(super) update_phase_shift: bool,
}
