// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Visibility notification and nearby-object resolution.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub const fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    /// Resolve the activation radius used by TrinityCore's
    /// `WorldObject::GetGridActivationRange` for a map-owned source.
    ///
    /// C++ (`Entities/Object/Object.cpp:1433-1450`) gives active objects and
    /// Players the map visibility range (with the `CinematicMgr::IsOnCinematic`
    /// instance-distance override), while an ordinary Creature uses its
    /// canonical `m_SightDistance`. Pets follow the Creature branch through
    /// their embedded Creature. Unsupported or missing records are fail-closed
    /// with a zero radius; callers still validate the source position before
    /// visiting cells. The represented cinematic state has the same active
    /// camera cursor as C++; resolving a FlyByCamera row is still outside this
    /// map-owned selector.
    pub(crate) fn grid_activation_range_for_guid_like_cpp(&self, guid: ObjectGuid) -> f32 {
        let Some(record) = self.map_object_record(guid) else {
            return 0.0;
        };

        if record.kind() == AccessorObjectKind::Player {
            let cinematic_active = record.player().is_some_and(|player| {
                player.gameplay_state().cinematic.camera_index_like_cpp() >= 0
            });
            if cinematic_active {
                return self
                    .visible_distance
                    .max(wow_entities::DEFAULT_VISIBILITY_INSTANCE);
            }
            return self.visible_distance;
        }

        if record.object().is_active() {
            return self.visible_distance;
        }

        match record.kind() {
            AccessorObjectKind::Creature => record
                .creature()
                .map(|creature| creature.sight_distance())
                .unwrap_or(0.0),
            AccessorObjectKind::Pet => record
                .pet()
                .map(|pet| pet.creature().sight_distance())
                .unwrap_or(0.0),
            _ => 0.0,
        }
    }

    pub fn nearby_cell_guids_like_cpp(&self, x: f32, y: f32, radius: f32) -> NearbyCellGuids {
        if !is_valid_map_coord_2d(x, y) {
            return NearbyCellGuids::default();
        }

        let area = calculate_cell_area_like_cpp(x, y, radius);
        let mut result = NearbyCellGuids::default();
        for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
            for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                result.visited_cells += 1;
                let cell = Cell::from_cell_coord(CellCoord::new(cell_x, cell_y));
                let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                else {
                    continue;
                };
                let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                    continue;
                };
                result.merge_world(&local_cell.world_objects);
                result.merge_grid(&local_cell.grid_objects);
            }
        }

        result
    }

    fn nearby_player_guids_for_visibility_like_cpp(
        &self,
        source_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some((position, combat_reach, is_in_world)) =
            self.map_object_record(source_guid).map(|record| {
                let object = record.object();
                (
                    object.position(),
                    object.combat_reach(),
                    object.object().is_in_world(),
                )
            })
        else {
            return Vec::new();
        };
        if !is_in_world || !is_valid_map_coord_2d(position.x, position.y) {
            return Vec::new();
        }

        let mut players: Vec<_> = self
            .nearby_cell_guids_like_cpp(
                position.x,
                position.y,
                self.visibility_range() + combat_reach,
            )
            .world
            .players
            .iter()
            .copied()
            .filter(|player_guid| *player_guid != source_guid)
            .filter(|player_guid| {
                self.map_object_record(*player_guid)
                    .is_some_and(|record| record.object().object().is_in_world())
            })
            .collect();
        players.sort();
        players.dedup();
        players
    }

    fn mark_player_visibility_guids_like_cpp(&mut self, players: &[ObjectGuid]) -> usize {
        let mut marked = 0;
        for player_guid in players {
            if let Some(player) = self.get_typed_player_mut(*player_guid) {
                player
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
                marked += 1;
            }
        }
        marked
    }

    /// Snapshot canonical typed map transports without exposing the map's
    /// storage guard to a packet/session consumer. `Map::SendInitTransports`
    /// walks every same-map transport, so this intentionally is not a nearby
    /// cell query.
    pub fn typed_transport_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut guids = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Transport && record.transport().is_some())
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();
        guids.sort();
        guids
    }

    /// Resolve the Players that share a phase with a map-owned transport.
    ///
    /// C++ `Map::AddToMap(Transport)` and `Map::RemoveFromMap` use the map
    /// reference walk (`Map.cpp:574-610, 1853-1915`), rather than the
    /// distance-based cell walk used by ordinary world objects.  Capture only
    /// owned GUIDs while the map is exclusive; the Session later resolves its
    /// current registration and publishes the transport block outside this
    /// guard.
    pub fn transport_visibility_recipients_like_cpp(
        &self,
        transport_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(transport) = self.map_object_record(transport_guid).filter(|record| {
            record.kind() == AccessorObjectKind::Transport && record.object().object().is_in_world()
        }) else {
            return Vec::new();
        };
        let transport_world = transport.object();
        let mut players = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Player
                    && record.object().object().is_in_world()
                    && record.object().in_same_phase(transport_world))
                .then_some(*guid)
            })
            .collect::<Vec<_>>();
        players.sort();
        players
    }

    /// Mark every same-phase Player selected by the transport map-reference
    /// walk. Packet delivery remains on the existing deferred visibility rail.
    pub(super) fn mark_transport_players_for_visibility_like_cpp(
        &mut self,
        transport_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let players = self.transport_visibility_recipients_like_cpp(transport_guid);
        self.mark_player_visibility_guids_like_cpp(&players);
        players
    }

    /// Capture the nearby in-world Players that may observe a map-owned source
    /// and mark them for the existing deferred visibility rail.
    ///
    /// C++ `Map::AddToMap`/`Map::RemoveFromMap` eventually call
    /// `UpdateObjectVisibilityOnCreate/Destroy` (`Map.cpp:530-610,933-951`,
    /// `Object.h:703-704`). Those routines may walk nearby players, but Rust
    /// must not deliver packets while the map owner is mutating storage.
    pub(super) fn nearby_and_mark_player_visibility_guids_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let players = self.nearby_player_guids_for_visibility_like_cpp(source_guid);
        self.mark_player_visibility_guids_like_cpp(&players);
        players
    }

    pub(super) fn capture_creature_visibility_destroy_recipients_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(record) = self.map_object_record(source_guid) else {
            return Vec::new();
        };
        if !source_guid.is_creature_or_pet() || !record.object().object().is_in_world() {
            return Vec::new();
        }
        let charmer_guid = record.charmer_guid_like_cpp();
        self.nearby_and_mark_player_visibility_guids_like_cpp(source_guid)
            .into_iter()
            .filter(|player_guid| Some(*player_guid) != charmer_guid)
            .collect()
    }

    /// Mark the canonical Players that may observe a map-owned source so the
    /// existing delayed relocation/session rail recomputes their exact
    /// visibility on the next map phase.
    ///
    /// C++ `Map::AddToMap`/`Map::RemoveFromMap` eventually call
    /// `UpdateObjectVisibilityOnCreate/Destroy` (`Map.cpp:530-610,933-951`,
    /// `Object.h:703-704`).  Those routines may walk nearby players, but Rust
    /// must not deliver packets while the map owner is mutating storage.  The
    /// canonical equivalent is to set `NOTIFY_VISIBILITY_CHANGED` on each
    /// nearby in-world Player; `process_live_relocation_notifies_like_cpp`
    /// consumes that flag and hands an owned refresh intent to the session
    /// outside the map borrow.
    pub(super) fn mark_nearby_players_for_visibility_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> usize {
        self.nearby_and_mark_player_visibility_guids_like_cpp(source_guid)
            .len()
    }

    /// Drain Creature destroy recipients captured during map-owned removals.
    pub fn take_creature_visibility_destroy_recipients_like_cpp(
        &mut self,
    ) -> Vec<CreatureVisibilityDestroyRecipientsLikeCpp> {
        let mut pending =
            std::mem::take(&mut self.pending_creature_visibility_destroy_recipients_like_cpp);
        pending.sort_by_key(|intent| intent.creature_guid);
        for intent in &mut pending {
            intent.recipient_guids.sort();
            intent.recipient_guids.dedup();
        }
        pending.retain(|intent| !intent.recipient_guids.is_empty());
        pending
    }

    pub fn visit_nearby_cells_of_like_cpp(
        &self,
        centers: impl IntoIterator<Item = NearbyCellVisitCenter>,
    ) -> NearbyCellVisitPlan {
        let mut marked_cells = HashSet::new();
        let mut marked_cells_in_visit_order = Vec::new();
        let mut nearby = NearbyCellGuids::default();
        let mut skipped_missing_centers = Vec::new();
        let mut skipped_invalid_position_centers = Vec::new();

        for center in centers {
            let Some(object) = self.map_object(center.guid) else {
                skipped_missing_centers.push(center.guid);
                continue;
            };
            let position = object.position();
            if !is_valid_map_coord_2d(position.x, position.y) {
                skipped_invalid_position_centers.push(center.guid);
                continue;
            }

            let area =
                calculate_cell_area_like_cpp(position.x, position.y, center.activation_radius);
            for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
                for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                    let cell_coord = CellCoord::new(cell_x, cell_y);
                    if !marked_cells.insert(cell_coord) {
                        continue;
                    }

                    marked_cells_in_visit_order.push(cell_coord);
                    nearby.visited_cells += 1;
                    let cell = Cell::from_cell_coord(cell_coord);
                    let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                    else {
                        continue;
                    };
                    let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                        continue;
                    };
                    nearby.merge_world(&local_cell.world_objects);
                    nearby.merge_grid(&local_cell.grid_objects);
                }
            }
        }

        NearbyCellVisitPlan {
            marked_cells: marked_cells_in_visit_order,
            nearby,
            skipped_missing_centers,
            skipped_invalid_position_centers,
        }
    }

    pub fn object_update_plan_for_nearby_like_cpp(
        &self,
        nearby: &NearbyCellGuids,
        diff_ms: u32,
    ) -> ObjectUpdatePlan {
        let mut update_guids = Vec::new();
        for guid in nearby
            .world
            .creatures
            .iter()
            .chain(nearby.world.dynamic_objects.iter())
            .chain(nearby.grid.creatures.iter())
            .chain(nearby.grid.gameobjects.iter())
            .chain(nearby.grid.dynamic_objects.iter())
            .chain(nearby.grid.area_triggers.iter())
            .chain(nearby.grid.scene_objects.iter())
            .chain(nearby.grid.conversations.iter())
        {
            if self
                .map_object(*guid)
                .is_some_and(|object| object.object().is_in_world())
            {
                update_guids.push(*guid);
            }
        }

        update_guids.sort();
        update_guids.dedup();
        ObjectUpdatePlan {
            diff_ms,
            update_guids,
        }
    }

    /// Build the production `ObjectUpdater` player source selection for this
    /// map incarnation. C++ source categories are taken from `Map.cpp:701-754`:
    /// in-world players, viewpoints, far PvE combat creatures, out-of-range
    /// aura casters and summons. Unsupported runtime references remain absent
    /// rather than being fabricated.
    pub(super) fn map_update_player_sources_for_current_tick_like_cpp(
        &self,
    ) -> Vec<MapUpdatePlayerSources> {
        let mut sources = Vec::new();
        for (guid, record) in self.entity_world.iter() {
            if record.kind() != AccessorObjectKind::Player
                || !record.object().object().is_in_world()
            {
                continue;
            }

            let player_object = record.object();
            let Some(player) = record.player() else {
                continue;
            };
            let is_far_in_world_unit = |target_guid: ObjectGuid| {
                let Some(target) = self.map_object(target_guid) else {
                    return false;
                };
                target.object().is_in_world()
                    && !player_object.is_within_dist_in_map(target, self.visible_distance, false)
            };

            let far_combat_unit_guids = player
                .unit()
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .copied()
                .filter(|target_guid| {
                    is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();
            let far_aura_caster_guids = player
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .map(|aura| aura.caster_guid)
                .filter(|target_guid| {
                    is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();
            let far_summon_guids = player
                .unit()
                .subsystems()
                .control
                .summon_slots
                .iter()
                .copied()
                .filter(|target_guid| {
                    !target_guid.is_empty()
                        && is_far_in_world_unit(*target_guid)
                        && self.map_object_record(*target_guid).is_some_and(|target| {
                            matches!(
                                target.kind(),
                                AccessorObjectKind::Creature | AccessorObjectKind::Pet
                            )
                        })
                })
                .collect();

            sources.push(MapUpdatePlayerSources {
                player_guid: *guid,
                viewpoint_guid: (!player.active_data().farsight_object.is_empty())
                    .then_some(player.active_data().farsight_object),
                far_combat_unit_guids,
                far_aura_caster_guids,
                far_summon_guids,
            });
        }

        sources
    }

    /// Build the production `ObjectUpdater` selection for the current map
    /// incarnation. This keeps source discovery and nearby-cell marking on the
    /// canonical map owner; the caller may then consume the owned GUID plan
    /// without retaining any map borrow across delivery or I/O.
    ///
    /// C++ source categories are taken from `Map.cpp:701-754`: in-world
    /// players, viewpoints, far PvE combat creatures, out-of-range aura
    /// casters, summons and active non-Players. Players and Corpses are not
    /// included in the resulting ObjectUpdater GUID set.
    pub fn object_update_plan_for_current_tick_like_cpp(&self, diff_ms: u32) -> ObjectUpdatePlan {
        let sources = self.map_update_player_sources_for_current_tick_like_cpp();

        let visit_plan = self.map_update_visit_plan_like_cpp(
            sources,
            self.represented_active_non_player_sources_like_cpp(),
            std::iter::empty(),
            diff_ms,
        );
        let centers =
            visit_plan
                .nearby_visit_centers
                .into_iter()
                .map(|guid| NearbyCellVisitCenter {
                    guid,
                    activation_radius: self.grid_activation_range_for_guid_like_cpp(guid),
                });
        let nearby = self.visit_nearby_cells_of_like_cpp(centers);
        self.object_update_plan_for_nearby_like_cpp(&nearby.nearby, diff_ms)
    }

    pub fn reset_notify_flags_for_cells_like_cpp(
        &mut self,
        cells: impl IntoIterator<Item = CellCoord>,
    ) -> ResetNotifyFlagsOutcome {
        let mut reset_player_guids = Vec::new();
        let mut reset_creature_guids = Vec::new();
        let mut missing_guids = Vec::new();

        for cell_coord in cells {
            let cell = Cell::from_cell_coord(cell_coord);
            let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
                continue;
            };
            let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                continue;
            };

            reset_player_guids.extend(local_cell.world_objects.players.iter().copied());
            reset_creature_guids.extend(local_cell.grid_objects.creatures.iter().copied());
            reset_creature_guids.extend(local_cell.world_objects.creatures.iter().copied());
        }

        sort_dedup(&mut reset_player_guids);
        sort_dedup(&mut reset_creature_guids);

        for guid in reset_player_guids
            .iter()
            .chain(reset_creature_guids.iter())
            .copied()
        {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                missing_guids.push(guid);
                continue;
            };
            record.object_mut().object_mut().reset_all_notifies();
        }

        ResetNotifyFlagsOutcome {
            reset_player_guids,
            reset_creature_guids,
            missing_guids,
        }
    }

    pub(super) fn player_seer_needs_notify_visibility_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> bool {
        self.player_viewpoint_guid_like_cpp(player_guid)
            .is_some_and(|viewpoint_guid| self.object_needs_notify_visibility(viewpoint_guid))
    }

    pub(super) fn object_needs_notify_visibility(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid).is_some_and(|object| {
            object
                .object()
                .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
        })
    }
}
