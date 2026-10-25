// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned result and narrow inputs for one pure Player stats calculation.

use crate::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_entities::PlayerEffectiveCombatStatsLikeCpp;

/// Optional compatibility inputs for the existing projection API.
#[derive(Clone, Copy)]
pub struct PlayerStatComputationRequest<'a> {
    projection_identity: Option<(u8, u8, u8)>,
    gear_override: Option<&'a RepresentedPlayerGearStatsLikeCpp>,
}

impl<'a> PlayerStatComputationRequest<'a> {
    pub fn new(
        projection_identity: Option<(u8, u8, u8)>,
        gear_override: Option<&'a RepresentedPlayerGearStatsLikeCpp>,
    ) -> Self {
        Self {
            projection_identity,
            gear_override,
        }
    }

    pub fn projection_identity(&self) -> Option<(u8, u8, u8)> {
        self.projection_identity
    }

    pub fn gear_override(&self) -> Option<&'a RepresentedPlayerGearStatsLikeCpp> {
        self.gear_override
    }
}

/// Owned calculation output consumed by Session adapters.
pub struct PlayerStatComputation {
    pub gear: RepresentedPlayerGearStatsLikeCpp,
    pub projection: PlayerStatSystemProjectionLikeCpp,
    pub effective: PlayerEffectiveCombatStatsLikeCpp,
}

/// Pure Inventory calculation called by Core while the canonical Player is borrowed.
/// Implementations must not perform I/O, packet delivery or await inside the owner callback.
pub type PlayerStatCalculator =
    for<'facts, 'catalogs, 'request> fn(
        &super::PlayerStatFacts<'facts>,
        &super::PlayerStatCatalogs<'catalogs>,
        &PlayerStatComputationRequest<'request>,
    ) -> Option<PlayerStatComputation>;
