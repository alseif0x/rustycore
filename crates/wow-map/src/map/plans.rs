// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map visit and relocation planning types.

use super::*;

#[derive(Debug, Clone, Default)]
pub struct NearbyCellGuids {
    pub world: WorldObjectGuids,
    pub grid: GridObjectGuids,
    pub visited_cells: usize,
}

impl PartialEq for NearbyCellGuids {
    fn eq(&self, other: &Self) -> bool {
        self.visited_cells == other.visited_cells
            && self.world.players == other.world.players
            && self.world.creatures == other.world.creatures
            && self.world.corpses == other.world.corpses
            && self.world.dynamic_objects == other.world.dynamic_objects
            && self.grid.gameobjects == other.grid.gameobjects
            && self.grid.creatures == other.grid.creatures
            && self.grid.dynamic_objects == other.grid.dynamic_objects
            && self.grid.corpses == other.grid.corpses
            && self.grid.area_triggers == other.grid.area_triggers
            && self.grid.scene_objects == other.grid.scene_objects
            && self.grid.conversations == other.grid.conversations
    }
}

impl Eq for NearbyCellGuids {}

impl NearbyCellGuids {
    pub fn is_empty(&self) -> bool {
        self.world.is_empty() && self.grid.is_empty()
    }

    pub fn len(&self) -> usize {
        self.world.len() + self.grid.len()
    }

    pub fn all_guids(&self) -> HashSet<ObjectGuid> {
        let mut guids = HashSet::with_capacity(self.len());
        guids.extend(self.world.players.iter().copied());
        guids.extend(self.world.creatures.iter().copied());
        guids.extend(self.world.corpses.iter().copied());
        guids.extend(self.world.dynamic_objects.iter().copied());
        guids.extend(self.grid.gameobjects.iter().copied());
        guids.extend(self.grid.creatures.iter().copied());
        guids.extend(self.grid.dynamic_objects.iter().copied());
        guids.extend(self.grid.corpses.iter().copied());
        guids.extend(self.grid.area_triggers.iter().copied());
        guids.extend(self.grid.scene_objects.iter().copied());
        guids.extend(self.grid.conversations.iter().copied());
        guids
    }

    pub(in crate::map) fn merge_world(&mut self, other: &WorldObjectGuids) {
        self.world.players.extend(other.players.iter().copied());
        self.world.creatures.extend(other.creatures.iter().copied());
        self.world.corpses.extend(other.corpses.iter().copied());
        self.world
            .dynamic_objects
            .extend(other.dynamic_objects.iter().copied());
    }

    pub(in crate::map) fn merge_grid(&mut self, other: &GridObjectGuids) {
        self.grid
            .gameobjects
            .extend(other.gameobjects.iter().copied());
        self.grid.creatures.extend(other.creatures.iter().copied());
        self.grid
            .dynamic_objects
            .extend(other.dynamic_objects.iter().copied());
        self.grid.corpses.extend(other.corpses.iter().copied());
        self.grid
            .area_triggers
            .extend(other.area_triggers.iter().copied());
        self.grid
            .scene_objects
            .extend(other.scene_objects.iter().copied());
        self.grid
            .conversations
            .extend(other.conversations.iter().copied());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearbyCellVisitCenter {
    pub guid: ObjectGuid,
    pub activation_radius: f32,
}

#[derive(Debug, Clone, Default)]
pub struct NearbyCellVisitPlan {
    pub marked_cells: Vec<CellCoord>,
    pub nearby: NearbyCellGuids,
    pub skipped_missing_centers: Vec<ObjectGuid>,
    pub skipped_invalid_position_centers: Vec<ObjectGuid>,
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectUpdatePlan {
    pub diff_ms: u32,
    pub update_guids: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapUpdatePlayerSources {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: Option<ObjectGuid>,
    pub far_combat_unit_guids: Vec<ObjectGuid>,
    pub far_aura_caster_guids: Vec<ObjectGuid>,
    pub far_summon_guids: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapUpdateVisitPlan {
    pub diff_ms: u32,
    pub session_update_players: Vec<ObjectGuid>,
    pub player_update_guids: Vec<ObjectGuid>,
    pub nearby_visit_centers: Vec<ObjectGuid>,
    pub transport_update_guids: Vec<ObjectGuid>,
    pub process_relocation_notifies: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelocationNotifyProcessPlan {
    pub diff_ms: u32,
    pub delayed_relocation_cells: Vec<CellCoord>,
    pub reset_notify_cells: Vec<CellCoord>,
    pub reset_timer_grids: Vec<GridCoord>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessRelocationNotifiesOutcome {
    pub process_plan: RelocationNotifyProcessPlan,
    pub delayed_plan: DelayedUnitRelocationForCellsPlan,
    pub visibility_plans: DelayedUnitRelocationVisibilityPlans,
    pub reset_outcome: ResetNotifyFlagsOutcome,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetNotifyFlagsOutcome {
    pub reset_player_guids: Vec<ObjectGuid>,
    pub reset_creature_guids: Vec<ObjectGuid>,
    pub missing_guids: Vec<ObjectGuid>,
}

/// C++ `MapObjectCellMoveState` (`MapObject.h:28-33`) represented for
/// map-owned delayed cell/grid move-list state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectCellMoveStateLikeCpp {
    None,
    Active,
    Inactive,
}

pub type MapObjectCellMoveState = MapObjectCellMoveStateLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectMoveListFamilyLikeCpp {
    Creature,
    GameObject,
    DynamicObject,
    AreaTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingCellMoveLikeCpp {
    pub state: MapObjectCellMoveStateLikeCpp,
    pub new_position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddObjectToMoveListOutcomeLikeCpp {
    Queued,
    UpdatedExisting,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveObjectFromMoveListOutcomeLikeCpp {
    MarkedInactive,
    AlreadyInactive,
    NotQueued,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetRemoveRequestLikeCpp {
    pub pet_guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
}

/// Drain summary for C++ `Map::MoveAll*InMoveList` (`Map.cpp:1239-1416`).
/// This is a map-owned seam only: it does not claim UpdatePositionData,
/// visibility fanout, AfterRelocation, dynamic tree, scripts/AI, ObjectAccessor,
/// or session packet runtime. Pet removal requests are delivered by the session
/// runtime after the map guard is released.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveListDrainSummaryLikeCpp {
    pub family: Option<MapObjectMoveListFamilyLikeCpp>,
    pub processed: usize,
    pub relocated: usize,
    pub inactive_reset: usize,
    pub not_in_world: usize,
    pub missing_or_stale: usize,
    pub wrong_kind: usize,
    pub blocked_by_unloaded_grid: usize,
    pub respawn_relocated: usize,
    pub remove_list_queued: usize,
    pub pet_remove_requested: usize,
    pub pet_remove_requests: Vec<PetRemoveRequestLikeCpp>,
    pub respawn_relocation_unsupported: usize,
    pub failed_invalid_position: usize,
    pub failed_store: usize,
    pub locked_ignored: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapObjectMoveListEntry {
    pub guid: ObjectGuid,
    pub kind: AccessorObjectKind,
    pub move_state: MapObjectCellMoveState,
    pub new_position: Position,
    pub respawn_position: Option<Position>,
    pub is_pet: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapObjectMoveListPlan {
    pub relocated: Vec<ObjectGuid>,
    pub respawn_relocated: Vec<ObjectGuid>,
    pub remove_from_world: Vec<ObjectGuid>,
    pub pet_removed: Vec<ObjectGuid>,
    pub blocked_unloaded_grid: Vec<ObjectGuid>,
    pub reset_inactive_or_none: Vec<ObjectGuid>,
    pub skipped_not_in_world: Vec<ObjectGuid>,
    pub skipped_other_map_or_missing: Vec<ObjectGuid>,
    pub skipped_kind_mismatch: Vec<ObjectGuid>,
    pub failed_invalid_position: Vec<ObjectGuid>,
    pub failed_store: Vec<ObjectGuid>,
    pub unsupported_kind: Vec<ObjectGuid>,
}
