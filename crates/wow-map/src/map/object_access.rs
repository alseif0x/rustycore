// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map object access, combat validation and grid host delegation.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(in crate::map) fn viewpoint_has_invalid_position_like_cpp(
        &self,
        viewpoint_guid: ObjectGuid,
    ) -> bool {
        self.map_object(viewpoint_guid).is_none_or(|viewpoint| {
            let position = viewpoint.position();
            !is_valid_map_coord_2d(position.x, position.y)
        })
    }

    pub(crate) fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
        self.entity_world.get(&guid)
    }

    /// Represented tail metrics from C++ `Map::Update` after
    /// `sScriptMgr->OnMapUpdate(this, t_diff)` (`Map.cpp:804-815`).
    ///
    /// C++ emits `TC_METRIC_VALUE("map_creatures", GetObjectsStore().Size<Creature>())`
    /// and `TC_METRIC_VALUE("map_gameobjects", GetObjectsStore().Size<GameObject>())`.
    /// Rust reads only canonical typed `MapObjectRecord`s from `entity_world`: a
    /// record must have both the exact canonical kind and the corresponding typed
    /// body. Generic `WorldObject` records, Pet, Transport, DynamicObject,
    /// AreaTrigger, Player, etc. are intentionally excluded; no telemetry backend
    /// is invoked here.
    pub fn map_update_metrics_like_cpp(&self) -> MapUpdateMetricsSummaryLikeCpp {
        let mut summary = MapUpdateMetricsSummaryLikeCpp {
            map_id: self.map_id,
            instance_id: self.instance_id,
            ..MapUpdateMetricsSummaryLikeCpp::default()
        };

        for record in self.entity_world.values() {
            match record.kind() {
                AccessorObjectKind::Creature if record.creature().is_some() => {
                    summary.creature_count += 1;
                }
                AccessorObjectKind::GameObject if record.game_object().is_some() => {
                    summary.gameobject_count += 1;
                }
                _ => {}
            }
        }

        summary
    }

    pub fn map_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_record(guid).map(MapObjectRecord::object)
    }

    pub(in crate::map) fn object_is_in_world(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid)
            .is_some_and(|object| object.object().is_in_world())
    }

    pub fn map_object_by_kind(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<&WorldObject> {
        let record = self.map_object_record(guid)?;
        allowed.contains(&record.kind()).then_some(record.object())
    }

    pub fn typed_player_counts_like_cpp(&self) -> (u32, u32) {
        let mut total = 0u32;
        let mut non_game_masters = 0u32;
        for record in self.entity_world.values() {
            if record.kind() != AccessorObjectKind::Player {
                continue;
            }
            let Some(player) = record.player() else {
                continue;
            };
            total = total.saturating_add(1);
            if !player.is_game_master_like_cpp() {
                non_game_masters = non_game_masters.saturating_add(1);
            }
        }
        (total, non_game_masters)
    }

    pub(in crate::map) fn combat_unit_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<CombatUnitSnapshotLikeCpp<'_>> {
        if let Some(player) = self.get_typed_player(guid) {
            return Some(CombatUnitSnapshotLikeCpp {
                guid,
                unit: player.unit(),
                game_master_player: player.is_game_master_like_cpp(),
            });
        }
        self.get_typed_creature(guid)
            .map(|creature| CombatUnitSnapshotLikeCpp {
                guid,
                unit: creature.unit(),
                game_master_player: false,
            })
    }

    pub(in crate::map) fn combat_begin_context_like_cpp(
        &self,
        owner: CombatUnitSnapshotLikeCpp<'_>,
        target: CombatUnitSnapshotLikeCpp<'_>,
    ) -> CombatBeginContextLikeCpp {
        let owner_world = owner.unit.world();
        let target_world = target.unit.world();
        CombatBeginContextLikeCpp {
            same_unit: owner.guid == target.guid,
            attacker_in_world: owner_world.object().is_in_world(),
            victim_in_world: target_world.object().is_in_world(),
            attacker_alive: owner.unit.is_alive(),
            victim_alive: target.unit.is_alive(),
            same_map: owner_world.is_in_map(target_world),
            same_phase: owner_world.in_same_phase(target_world),
            attacker_unit_state: owner.unit.unit_state(),
            victim_unit_state: target.unit.unit_state(),
            attacker_combat_disallowed: owner.unit.subsystems().combat.combat_disallowed,
            victim_combat_disallowed: target.unit.subsystems().combat.combat_disallowed,
            relation_represented: false,
            attacker_is_friendly_to_victim: false,
            victim_is_friendly_to_attacker: false,
            attacker_or_owner_player_is_game_master: owner.game_master_player,
            victim_or_owner_player_is_game_master: target.game_master_player,
        }
    }

    pub fn revalidate_all_combat_refs_like_cpp(&mut self) -> Vec<(ObjectGuid, ObjectGuid)> {
        let owner_guids = self.typed_combat_unit_guids_like_cpp();
        let mut invalid = Vec::new();

        for owner_guid in owner_guids {
            let Some(owner) = self.combat_unit_snapshot_like_cpp(owner_guid) else {
                continue;
            };
            let refs: Vec<_> = owner
                .unit
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .chain(owner.unit.subsystems().combat.pvp_refs.keys())
                .copied()
                .collect();

            for target_guid in refs {
                let Some(target) = self.combat_unit_snapshot_like_cpp(target_guid) else {
                    invalid.push((owner_guid, target_guid));
                    continue;
                };
                if !CombatSubsystem::can_begin_combat_like_cpp(
                    self.combat_begin_context_like_cpp(owner, target),
                ) {
                    invalid.push((owner_guid, target_guid));
                }
            }
        }

        for (owner_guid, target_guid) in &invalid {
            if let Some(owner) = self.get_typed_player_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            } else if let Some(owner) = self.get_typed_creature_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            }

            if let Some(target) = self.get_typed_player_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            } else if let Some(target) = self.get_typed_creature_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            }
        }

        invalid
    }

    pub(in crate::map) fn validate_map_object(
        &self,
        object: &WorldObject,
    ) -> Result<(), MapObjectStoreError> {
        if object.map_id() == self.map_id && object.instance_id() == self.instance_id {
            return Ok(());
        }

        Err(MapObjectStoreError::WrongMap {
            guid: object.guid(),
            expected_map_id: self.map_id,
            expected_instance_id: self.instance_id,
            actual_map_id: object.map_id(),
            actual_instance_id: object.instance_id(),
        })
    }
}

pub(in crate::map) fn is_active_object_like_cpp(
    kind: AccessorObjectKind,
    object: &WorldObject,
) -> bool {
    kind == AccessorObjectKind::Player || object.is_active()
}

pub(in crate::map) fn remove_from_map_in_world_eligible_type_like_cpp(
    kind: AccessorObjectKind,
) -> bool {
    matches!(
        kind,
        AccessorObjectKind::Player
            | AccessorObjectKind::Creature
            | AccessorObjectKind::Pet
            | AccessorObjectKind::Corpse
            | AccessorObjectKind::GameObject
            | AccessorObjectKind::Transport
    )
}

pub(in crate::map) fn move_list_family_accepts_kind_like_cpp(
    family: MapObjectMoveListFamilyLikeCpp,
    kind: AccessorObjectKind,
) -> bool {
    match family {
        MapObjectMoveListFamilyLikeCpp::Creature => {
            matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
        }
        MapObjectMoveListFamilyLikeCpp::GameObject => {
            matches!(
                kind,
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport
            )
        }
        MapObjectMoveListFamilyLikeCpp::DynamicObject => kind == AccessorObjectKind::DynamicObject,
        MapObjectMoveListFamilyLikeCpp::AreaTrigger => kind == AccessorObjectKind::AreaTrigger,
    }
}

pub(in crate::map) fn push_in_world_guids<Terrain, Lifecycle>(
    map: &Map<Terrain, Lifecycle>,
    target: &mut Vec<ObjectGuid>,
    guids: impl IntoIterator<Item = ObjectGuid>,
) where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    target.extend(
        guids
            .into_iter()
            .filter(|guid| map.object_is_in_world(*guid)),
    );
}

pub(in crate::map) fn sort_dedup(guids: &mut Vec<ObjectGuid>) {
    guids.sort();
    guids.dedup();
}

pub(in crate::map) fn marked_cells_in_grid_like_cpp(
    grid: GridCoord,
    marked_cells: &HashSet<CellCoord>,
) -> Vec<CellCoord> {
    let cell_min_x = grid.x_coord * MAX_NUMBER_OF_CELLS;
    let cell_min_y = grid.y_coord * MAX_NUMBER_OF_CELLS;
    let cell_max_x = cell_min_x + MAX_NUMBER_OF_CELLS;
    let cell_max_y = cell_min_y + MAX_NUMBER_OF_CELLS;
    let mut cells = Vec::new();

    for x in cell_min_x..cell_max_x {
        for y in cell_min_y..cell_max_y {
            let cell = CellCoord::new(x, y);
            if marked_cells.contains(&cell) {
                cells.push(cell);
            }
        }
    }

    cells
}

pub(in crate::map) fn nearby_creature_guids_excluding(
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

pub(in crate::map) fn remove_list_grid_kind_like_cpp(
    kind: AccessorObjectKind,
) -> Option<GridObjectKind> {
    match kind {
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => Some(GridObjectKind::Creature),
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            Some(GridObjectKind::GameObject)
        }
        AccessorObjectKind::DynamicObject => Some(GridObjectKind::DynamicObject),
        AccessorObjectKind::AreaTrigger => Some(GridObjectKind::AreaTrigger),
        AccessorObjectKind::Corpse => Some(GridObjectKind::Corpse),
        AccessorObjectKind::SceneObject => Some(GridObjectKind::SceneObject),
        AccessorObjectKind::Conversation => Some(GridObjectKind::Conversation),
        AccessorObjectKind::Player => None,
    }
}

pub(in crate::map) fn switch_list_unit_kind_like_cpp(kind: AccessorObjectKind) -> bool {
    matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
}

pub(in crate::map) fn set_record_temp_world_object_like_cpp(
    record: &mut MapObjectRecord,
    on: bool,
) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.set_temp_world_object_like_cpp(on);
            }
        }
        AccessorObjectKind::Pet => {
            if let Some(pet) = record.pet_mut() {
                pet.creature_mut().set_temp_world_object_like_cpp(on);
            }
        }
        _ => {}
    }
}

pub(in crate::map) fn map_record_is_world_object_like_cpp(record: &MapObjectRecord) -> bool {
    if record.object().is_world_object() {
        return true;
    }
    if let Some(creature) = record.creature() {
        return creature.is_temp_world_object();
    }
    if let Some(pet) = record.pet() {
        return pet.creature().is_temp_world_object();
    }
    false
}

impl SwitchGridContainersOutcomeLikeCpp {
    pub(in crate::map) const fn executed() -> Self {
        Self {
            executed: true,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    pub(in crate::map) const fn missing_or_stale() -> Self {
        Self {
            executed: false,
            missing_or_stale: true,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    pub(in crate::map) const fn unsupported_kind() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: true,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    pub(in crate::map) const fn permanent_world_object() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: true,
            invalid_or_unloaded_grid: false,
        }
    }

    pub(in crate::map) const fn invalid_or_unloaded_grid() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: true,
        }
    }
}

pub(in crate::map) fn cleanup_map_object_record_before_delete_like_cpp(
    record: &mut MapObjectRecord,
    kind: AccessorObjectKind,
    creature_second_cleanup: bool,
) -> usize {
    match kind {
        AccessorObjectKind::Creature => record.creature_mut().map_or(0, |creature| {
            if !creature_second_cleanup {
                creature.set_destroyed_object(true);
            }
            creature.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Pet => record.pet_mut().map_or(0, |pet| {
            if !creature_second_cleanup {
                pet.creature_mut().set_destroyed_object(true);
            }
            pet.creature_mut().cleanup_before_delete();
            1
        }),
        AccessorObjectKind::GameObject => record.game_object_mut().map_or(0, |game_object| {
            game_object.set_destroyed_object(true);
            game_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Transport => record.transport_mut().map_or(0, |transport| {
            transport.game_object_mut().set_destroyed_object(true);
            let _removed_static_passengers = transport.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::DynamicObject => {
            record.dynamic_object_mut().map_or(0, |dynamic_object| {
                dynamic_object.set_destroyed_object(true);
                dynamic_object.cleanup_before_delete();
                1
            })
        }
        AccessorObjectKind::AreaTrigger => record.area_trigger_mut().map_or(0, |area_trigger| {
            area_trigger.set_destroyed_object(true);
            area_trigger.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Corpse => record.corpse_mut().map_or(0, |corpse| {
            corpse.set_destroyed_object(true);
            corpse.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::SceneObject => record.scene_object_mut().map_or(0, |scene_object| {
            scene_object.set_destroyed_object(true);
            scene_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Conversation => record.conversation_mut().map_or(0, |conversation| {
            conversation.set_destroyed_object(true);
            conversation.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Player => {
            // No typed represented `CleanupsBeforeDelete` exists for Player in this
            // bounded map remove-list seam. Preserve at least the base
            // `WorldObject::SetDestroyedObject(true)` mutation and report no
            // represented cleanup.
            record.object_mut().object_mut().set_destroyed_object(true);
            0
        }
    }
}

pub(in crate::map) fn insert_object_guid_in_cell_like_cpp(
    cell: &mut Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) {
    match kind {
        AccessorObjectKind::Player => {
            cell.world_objects.players.insert(guid);
        }
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                cell.world_objects.creatures.insert(guid);
            } else {
                cell.grid_objects.creatures.insert(guid);
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            cell.grid_objects.gameobjects.insert(guid);
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                cell.world_objects.dynamic_objects.insert(guid);
            } else {
                cell.grid_objects.dynamic_objects.insert(guid);
            }
        }
        AccessorObjectKind::AreaTrigger => {
            cell.grid_objects.area_triggers.insert(guid);
        }
        AccessorObjectKind::Corpse => {
            if is_world_object {
                cell.world_objects.corpses.insert(guid);
            } else {
                cell.grid_objects.corpses.insert(guid);
            }
        }
        AccessorObjectKind::SceneObject => {
            cell.grid_objects.scene_objects.insert(guid);
        }
        AccessorObjectKind::Conversation => {
            cell.grid_objects.conversations.insert(guid);
        }
    }
}

pub(in crate::map) fn remove_object_guid_from_cell_like_cpp<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    grid: GridCoord,
    cell: &Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) -> bool
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    let Some(ngrid) = map.get_ngrid_mut(grid) else {
        return false;
    };
    let Some(local_cell) = ngrid.get_grid_type_mut(cell.cell_x(), cell.cell_y()) else {
        return false;
    };

    match kind {
        AccessorObjectKind::Player => local_cell.world_objects.players.remove(&guid),
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                local_cell.world_objects.creatures.remove(&guid)
            } else {
                local_cell.grid_objects.creatures.remove(&guid)
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            local_cell.grid_objects.gameobjects.remove(&guid)
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                local_cell.world_objects.dynamic_objects.remove(&guid)
            } else {
                local_cell.grid_objects.dynamic_objects.remove(&guid)
            }
        }
        AccessorObjectKind::AreaTrigger => local_cell.grid_objects.area_triggers.remove(&guid),
        AccessorObjectKind::Corpse => {
            if is_world_object {
                local_cell.world_objects.corpses.remove(&guid)
            } else {
                local_cell.grid_objects.corpses.remove(&guid)
            }
        }
        AccessorObjectKind::SceneObject => local_cell.grid_objects.scene_objects.remove(&guid),
        AccessorObjectKind::Conversation => local_cell.grid_objects.conversations.remove(&guid),
    }
}

/// Invalidates async claims before a terminal typed object lifetime is dropped.
///
/// C++ destroys `Creature::loot` / `GameObject::loot` together with the typed
/// object. Rust leases can retain the shared authority after that drop, so the
/// backing allocation must become permanently detached first.
pub(in crate::map) fn detach_typed_loot_authority_like_cpp(record: &mut MapObjectRecord) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        AccessorObjectKind::GameObject => {
            if let Some(game_object) = record.game_object_mut() {
                game_object.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        _ => {}
    }
}

/// A same-GUID whole-entity refresh may replace the record while deliberately
/// retaining the exact object lifetime. Do not detach the shared authority in
/// that case; only a distinct backing allocation is displaced terminally.
pub(in crate::map) fn typed_loot_authorities_share_storage_like_cpp(
    previous: &MapObjectRecord,
    replacement: &MapObjectRecord,
) -> bool {
    match (previous.kind(), replacement.kind()) {
        (AccessorObjectKind::Creature, AccessorObjectKind::Creature) => previous
            .creature()
            .zip(replacement.creature())
            .is_some_and(|(left, right)| left.is_same_incarnation_like_cpp(right)),
        (AccessorObjectKind::GameObject, AccessorObjectKind::GameObject) => previous
            .game_object()
            .zip(replacement.game_object())
            .is_some_and(|(previous, replacement)| {
                previous
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(replacement.loot_authority_like_cpp())
            }),
        _ => false,
    }
}

impl<Terrain, Lifecycle> GridUnloadEntityStore for Map<Terrain, Lifecycle> {
    fn creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::creature_mut)
    }

    fn game_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut GameObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
    }

    fn dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::dynamic_object_mut)
    }

    fn corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::corpse_mut)
    }

    fn area_trigger_mut(&mut self, guid: ObjectGuid) -> Option<&mut AreaTrigger> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::area_trigger_mut)
    }

    fn scene_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut SceneObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::scene_object_mut)
    }

    fn conversation_mut(&mut self, guid: ObjectGuid) -> Option<&mut Conversation> {
        self.entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::conversation_mut)
    }
}

impl<Terrain, Lifecycle> WorldObjectEnvironment for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader + MapWorldObjectEnvironment,
    Lifecycle: GridLifecycle,
{
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.terrain.line_of_sight(query)
    }

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        self.terrain.map_height(object, x, y, z, query)
    }

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32 {
        self.terrain.floor_z(object, position, max_search_dist)
    }
}
