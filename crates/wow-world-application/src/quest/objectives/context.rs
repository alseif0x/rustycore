// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed inputs for the selected objective-drain operation.

use super::super::QuestRewardCx;
#[cfg(any(test, feature = "test-fixtures"))]
use super::super::QuestRewardReputationFixtureRefsLikeCpp;
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_world_loot::LootState;

/// Test-only inputs consumed by the existing registry publication phase.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct QuestObjectiveRegistryFixtureRefsLikeCpp<'a> {
    pub(super) spell_state: &'a wow_world_spell::SessionSpellState,
    pub(super) position: &'a Option<wow_core::Position>,
    pub(super) health: &'a u32,
    pub(super) max_health: &'a u32,
    pub(super) alive: &'a bool,
    pub(super) transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
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
        health: &'a u32,
        max_health: &'a u32,
        alive: &'a bool,
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        mount_vehicle: &'a Option<wow_entities::Vehicle>,
        vehicle_seat_flags: &'a Option<i32>,
        vehicle_seat_id: &'a Option<u32>,
        pet_guid: &'a Option<ObjectGuid>,
    ) -> Self {
        Self {
            spell_state,
            position,
            health,
            max_health,
            alive,
            transport,
            mount_vehicle,
            vehicle_seat_flags,
            vehicle_seat_id,
            pet_guid,
        }
    }
}

/// A borrowed view of the reward operation and the existing loot publication state.
///
/// Objective completions reborrow the same selected Core owner for their nested reward;
/// this context never stores a World session or a callback into one.
pub struct QuestObjectiveProgressCx<'cx, 'session> {
    pub(in crate::quest) reward: &'cx mut QuestRewardCx<'session>,
    pub(super) loot: &'cx LootState,
    pub(super) item_guid_generator: &'cx ObjectGuidGenerator,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) reputation_fixture:
        &'cx mut QuestRewardReputationFixtureRefsLikeCpp<'session>,
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
}
