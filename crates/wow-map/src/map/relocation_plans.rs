// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Visibility and AI notification plans for object relocation.

use super::NearbyCellGuids;
use crate::coords::CellCoord;
use std::collections::HashSet;
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerRelocationVisibilityPlan {
    pub visible_guids: HashSet<ObjectGuid>,
    pub out_of_range_guids: HashSet<ObjectGuid>,
    pub reciprocal_player_updates: HashSet<ObjectGuid>,
    pub ai_relocation_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl PlayerRelocationVisibilityPlan {
    pub fn from_nearby_like_cpp(
        player_guid: ObjectGuid,
        previous_client_guids: impl IntoIterator<Item = ObjectGuid>,
        nearby: &NearbyCellGuids,
        relocated_for_ai: bool,
        player_seers_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let player_seers_needing_notify: HashSet<_> =
            player_seers_needing_notify.into_iter().collect();
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let visible_guids = nearby.all_guids();
        let mut out_of_range_guids: HashSet<_> = previous_client_guids.into_iter().collect();
        out_of_range_guids.remove(&player_guid);

        for guid in &visible_guids {
            out_of_range_guids.remove(guid);
        }

        let mut reciprocal_player_updates = HashSet::new();
        for guid in &nearby.world.players {
            if *guid != player_guid && !player_seers_needing_notify.contains(guid) {
                reciprocal_player_updates.insert(*guid);
            }
        }

        for guid in &out_of_range_guids {
            if guid.is_player() && !player_seers_needing_notify.contains(guid) {
                reciprocal_player_updates.insert(*guid);
            }
        }

        let ai_relocation_checks = if relocated_for_ai {
            nearby_creature_guids_excluding(nearby, player_guid)
                .into_iter()
                .filter(|guid| !creatures_needing_notify.contains(guid))
                .map(|guid| (guid, player_guid))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            visible_guids,
            out_of_range_guids,
            reciprocal_player_updates,
            ai_relocation_checks,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureRelocationVisibilityPlan {
    pub player_visibility_updates: HashSet<ObjectGuid>,
    pub ai_relocation_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl CreatureRelocationVisibilityPlan {
    pub fn from_nearby_like_cpp(
        creature_guid: ObjectGuid,
        source_creature_alive: bool,
        nearby: &NearbyCellGuids,
        player_seers_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let player_seers_needing_notify: HashSet<_> =
            player_seers_needing_notify.into_iter().collect();
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let mut player_visibility_updates = HashSet::new();
        let mut ai_relocation_checks = Vec::new();

        for player in &nearby.world.players {
            if !player_seers_needing_notify.contains(player) {
                player_visibility_updates.insert(*player);
            }
            ai_relocation_checks.push((creature_guid, *player));
        }

        if source_creature_alive {
            for creature in nearby_creature_guids_excluding(nearby, creature_guid) {
                ai_relocation_checks.push((creature_guid, creature));
                if !creatures_needing_notify.contains(&creature) {
                    ai_relocation_checks.push((creature, creature_guid));
                }
            }
        }

        Self {
            player_visibility_updates,
            ai_relocation_checks,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationPlan {
    pub creature_relocations: Vec<ObjectGuid>,
    pub player_relocations: Vec<ObjectGuid>,
    pub skipped_invalid_viewpoints: Vec<ObjectGuid>,
}

impl DelayedUnitRelocationPlan {
    pub fn from_nearby_like_cpp(
        nearby: &NearbyCellGuids,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        player_viewpoints_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let player_viewpoints_needing_notify: HashSet<_> =
            player_viewpoints_needing_notify.into_iter().collect();
        let invalid_non_self_viewpoints: HashSet<_> =
            invalid_non_self_viewpoints.into_iter().collect();

        let mut creature_relocations: Vec<_> = nearby
            .world
            .creatures
            .iter()
            .chain(nearby.grid.creatures.iter())
            .copied()
            .filter(|guid| creatures_needing_notify.contains(guid))
            .collect();
        creature_relocations.sort();
        creature_relocations.dedup();

        let mut player_relocations = Vec::new();
        let mut skipped_invalid_viewpoints = Vec::new();
        let mut players: Vec<_> = nearby.world.players.iter().copied().collect();
        players.sort();
        for player in players {
            if !player_viewpoints_needing_notify.contains(&player) {
                continue;
            }

            if invalid_non_self_viewpoints.contains(&player) {
                skipped_invalid_viewpoints.push(player);
            } else {
                player_relocations.push(player);
            }
        }

        Self {
            creature_relocations,
            player_relocations,
            skipped_invalid_viewpoints,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationForCellsPlan {
    pub cell_plans: Vec<DelayedUnitRelocationCellPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedUnitRelocationCellPlan {
    pub cell_coord: CellCoord,
    pub plan: DelayedUnitRelocationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedPlayerRelocationContext {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: ObjectGuid,
    pub previous_client_guids: Vec<ObjectGuid>,
    pub relocated_for_ai: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedCreatureRelocationContext {
    pub creature_guid: ObjectGuid,
    pub source_creature_alive: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationVisibilityPlans {
    pub creature_plans: Vec<CreatureDelayedRelocationVisibilityPlan>,
    pub player_plans: Vec<PlayerDelayedRelocationVisibilityPlan>,
    pub skipped_missing_sources: Vec<ObjectGuid>,
    pub skipped_invalid_source_positions: Vec<ObjectGuid>,
    pub missing_player_contexts: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureDelayedRelocationVisibilityPlan {
    pub creature_guid: ObjectGuid,
    pub cell_coord: CellCoord,
    pub nearby: NearbyCellGuids,
    pub visibility_plan: CreatureRelocationVisibilityPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDelayedRelocationVisibilityPlan {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: ObjectGuid,
    pub cell_coord: CellCoord,
    pub nearby: NearbyCellGuids,
    pub visibility_plan: PlayerRelocationVisibilityPlan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AIRelocationPlan {
    pub creature_unit_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl AIRelocationPlan {
    pub fn from_nearby_like_cpp(
        unit_guid: ObjectGuid,
        unit_is_creature: bool,
        nearby: &NearbyCellGuids,
    ) -> Self {
        let nearby_creatures = nearby_creature_guids_excluding(nearby, unit_guid);
        let mut creature_unit_checks = Vec::with_capacity(if unit_is_creature {
            nearby_creatures.len() * 2
        } else {
            nearby_creatures.len()
        });

        for creature in nearby_creatures {
            creature_unit_checks.push((creature, unit_guid));
            if unit_is_creature {
                creature_unit_checks.push((unit_guid, creature));
            }
        }

        Self {
            creature_unit_checks,
        }
    }
}

fn nearby_creature_guids_excluding(
    nearby: &NearbyCellGuids,
    excluded: ObjectGuid,
) -> Vec<ObjectGuid> {
    let mut nearby_creatures: Vec<_> = nearby
        .world
        .creatures
        .iter()
        .chain(nearby.grid.creatures.iter())
        .copied()
        .filter(|guid| *guid != excluded)
        .collect();
    nearby_creatures.sort();
    nearby_creatures.dedup();
    nearby_creatures
}
