//! Behaviour tests for [`super`].
//!
//! Extracted from `map.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::*;
use crate::grid_unload::{
    GridObjectKind, GridUnloadAction, GridUnloadApplyOutcome, GuidGridUnloadLifecycle,
    apply_grid_unload_action, apply_grid_unload_actions,
};
use crate::pool::{PoolGroupLikeCpp, PoolTemplateDataLikeCpp};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wow_constants::{DeathState, TypeId, TypeMask, UnitStandStateType};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    ACTIVE_PLAYER_DATA_COINAGE_BIT, AccessorObjectRef, AppliedAuraRef, Corpse, CorpseType,
    Creature, CreatureAddToWorldVehicleResetContextLikeCpp, CreatureFormationInfoLikeCpp,
    GameObject, GameObjectLootSource, GameObjectOwnedLoot, GooberUseSource, ObjectAccessor,
    ObjectNotifyFlags, OwnedAuraRef, PLAYER_DATA_INEBRIATION_BIT, Player,
    SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, Transport, UNIT_DATA_STAND_STATE_BIT,
    VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo, VehicleSpellImmunity,
    VehicleSpellImmunityKind,
};
use wow_loot::{CreatureLoot, LootClaimCommitError, OwnedLootAuthorityLifecycle};

const GO_FLAG_MAP_OBJECT: u32 = 0x0010_0000;

#[derive(Debug, Default)]
struct RecordingTerrain {
    loads: Vec<(u32, u32)>,
    unloads: Vec<(u32, u32)>,
}

impl TerrainGridLoader for RecordingTerrain {
    fn load_map_and_vmap(&mut self, grid_x: u32, grid_y: u32) {
        self.loads.push((grid_x, grid_y));
    }

    fn unload_map(&mut self, grid_x: u32, grid_y: u32) {
        self.unloads.push((grid_x, grid_y));
    }
}

impl MapWorldObjectEnvironment for RecordingTerrain {
    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool {
        true
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        INVALID_HEIGHT
    }

    fn floor_z(&self, _object: &WorldObject, _position: Position, _max_search_dist: f32) -> f32 {
        INVALID_HEIGHT
    }
}

fn script_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn dynamic_model_key(counter: i64) -> RepresentedGameObjectModelKeyLikeCpp {
    RepresentedGameObjectModelKeyLikeCpp {
        owner_guid: guid(HighGuid::GameObject, counter),
    }
}

#[test]
fn dynamic_tree_update_empty_tree_returns_before_timer_or_balance_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(3);

    let summary = map.update_dynamic_tree_like_cpp(250);

    assert_eq!(summary.diff_ms, 250);
    assert!(summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 200);
    assert!(!summary.timer_passed);
    assert_eq!(summary.timer_reset_to_ms, None);
    assert_eq!(summary.unbalanced_before, 3);
    assert!(!summary.balanced);
    assert_eq!(summary.unbalanced_after, 3);

    let next = map.update_dynamic_tree_like_cpp(50);
    assert_eq!(next.timer_before_ms, 200);
    assert_eq!(next.unbalanced_before, 3);
}

#[test]
fn dynamic_tree_update_non_empty_clean_tree_consumes_timer_and_resets_without_balance_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.insert_gameobject_model_like_cpp(dynamic_model_key(1));
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(0);

    let first = map.update_dynamic_tree_like_cpp(50);
    assert!(!first.empty);
    assert_eq!(first.timer_before_ms, 200);
    assert_eq!(first.timer_after_ms, 150);
    assert!(!first.timer_passed);
    assert_eq!(first.timer_reset_to_ms, None);
    assert_eq!(first.unbalanced_before, 0);
    assert!(!first.balanced);
    assert_eq!(first.unbalanced_after, 0);

    let second = map.update_dynamic_tree_like_cpp(150);
    assert_eq!(second.timer_before_ms, 150);
    assert_eq!(second.timer_after_ms, 200);
    assert!(second.timer_passed);
    assert_eq!(second.timer_reset_to_ms, Some(200));
    assert_eq!(second.unbalanced_before, 0);
    assert!(!second.balanced);
    assert_eq!(second.unbalanced_after, 0);
}

#[test]
fn dynamic_tree_update_non_empty_unbalanced_tree_balances_when_timer_passes_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.insert_gameobject_model_like_cpp(dynamic_model_key(1));
    map.insert_gameobject_model_like_cpp(dynamic_model_key(2));

    let first = map.update_dynamic_tree_like_cpp(199);
    assert_eq!(first.timer_before_ms, 200);
    assert_eq!(first.timer_after_ms, 1);
    assert!(!first.timer_passed);
    assert_eq!(first.unbalanced_before, 2);
    assert!(!first.balanced);
    assert_eq!(first.unbalanced_after, 2);

    let second = map.update_dynamic_tree_like_cpp(1);
    assert_eq!(second.timer_before_ms, 1);
    assert_eq!(second.timer_after_ms, 200);
    assert!(second.timer_passed);
    assert_eq!(second.timer_reset_to_ms, Some(200));
    assert_eq!(second.unbalanced_before, 2);
    assert!(second.balanced);
    assert_eq!(second.unbalanced_after, 0);

    let third = map.update_dynamic_tree_like_cpp(200);
    assert_eq!(third.unbalanced_before, 0);
    assert!(!third.balanced);
    assert_eq!(third.unbalanced_after, 0);
}

#[test]
fn dynamic_tree_insert_first_model_makes_tree_non_empty_and_update_consumes_timer_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45001);

    let inserted = map.insert_gameobject_model_like_cpp(key);
    assert_eq!(
        inserted.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(inserted.model_count_before, 0);
    assert_eq!(inserted.model_count_after, 1);
    assert_eq!(inserted.unbalanced_before, 0);
    assert_eq!(inserted.unbalanced_after, 1);
    assert!(map.contains_gameobject_model_like_cpp(key));

    let summary = map.update_dynamic_tree_like_cpp(50);
    assert!(!summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 150);
    assert!(!summary.timer_passed);
    assert_eq!(summary.unbalanced_before, 1);
    assert_eq!(summary.unbalanced_after, 1);
}

#[test]
fn dynamic_tree_duplicate_insert_does_not_double_count_or_increment_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45002);

    let first = map.insert_gameobject_model_like_cpp(key);
    let duplicate = map.insert_gameobject_model_like_cpp(key);

    assert_eq!(
        first.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(
        duplicate.status,
        DynamicMapTreeModelMutationStatusLikeCpp::AlreadyPresent
    );
    assert_eq!(duplicate.model_count_before, 1);
    assert_eq!(duplicate.model_count_after, 1);
    assert_eq!(duplicate.unbalanced_before, 1);
    assert_eq!(duplicate.unbalanced_after, 1);
}

#[test]
fn dynamic_tree_remove_contained_model_empties_tree_and_next_update_early_returns_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45003);
    map.insert_gameobject_model_like_cpp(key);

    let removed = map.remove_gameobject_model_like_cpp(key);
    assert_eq!(
        removed.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(removed.model_count_before, 1);
    assert_eq!(removed.model_count_after, 0);
    assert_eq!(removed.unbalanced_before, 1);
    assert_eq!(removed.unbalanced_after, 2);
    assert!(!map.contains_gameobject_model_like_cpp(key));

    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 200);
    assert!(!summary.timer_passed);
    assert_eq!(summary.unbalanced_before, 2);
    assert_eq!(summary.unbalanced_after, 2);
}

#[test]
fn dynamic_tree_missing_remove_is_noop_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45004);

    let missing = map.remove_gameobject_model_like_cpp(key);

    assert_eq!(
        missing.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Missing
    );
    assert_eq!(missing.model_count_before, 0);
    assert_eq!(missing.model_count_after, 0);
    assert_eq!(missing.unbalanced_before, 0);
    assert_eq!(missing.unbalanced_after, 0);
}

#[test]
fn dynamic_tree_contains_reflects_insert_and_remove_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45005);

    assert!(!map.contains_gameobject_model_like_cpp(key));
    map.insert_gameobject_model_like_cpp(key);
    assert!(map.contains_gameobject_model_like_cpp(key));
    map.remove_gameobject_model_like_cpp(key);
    assert!(!map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn dynamic_tree_gameobject_add_consumes_explicit_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45101, 4510101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let insert = outcome
        .gameobject_model_insert
        .expect("explicit represented model should insert dynamic-tree key");
    assert_eq!(
        insert.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(insert.model_count_before, 0);
    assert_eq!(insert.model_count_after, 1);
    assert_eq!(insert.unbalanced_before, 0);
    assert_eq!(insert.unbalanced_after, 1);
    let collision = outcome
        .gameobject_collision_enable
        .expect("represented model should record EnableCollision evidence");
    assert!(collision.represented_model_present);
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.previous_collision_enabled, None);
    assert_eq!(collision.new_collision_enabled, Some(false));
    assert!(map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn add_to_map_exact_gameobject_preinserts_canonical_store_and_spawn_index_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(47901, 4790101);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(
        outcome.gameobject_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.gameobject_spawn_indexed_before_add_to_world,
        Some(true)
    );
    assert!(outcome.gameobject_model_insert.is_some());
    assert!(outcome.gameobject_collision_enable.is_some());
    assert!(outcome.add_to_map_tail.is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(47901)
            .contains(&guid)
    );
}

#[test]
fn add_to_map_exact_gameobject_model_collision_and_world_state_mutate_canonical_record_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(47902, 4790201);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Ready);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let canonical = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(canonical.world().object().is_in_world());
    assert!(!canonical.world().object().is_new_object());
    assert_eq!(
        canonical.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.set_is_new_object_true);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
}

#[test]
fn active_non_player_add_remove_gameobject_updates_set_and_unload_lock_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48501, 4850101);
    let guid = gameobject.world().guid();
    let respawn_position = gameobject.stationary_position();
    let respawn_cell = Cell::from_world(respawn_position.x, respawn_position.y);
    let respawn_grid = GridCoord::new(respawn_cell.grid_x(), respawn_cell.grid_y());
    map.ensure_grid_loaded(&cell_from_grid_center(respawn_grid));
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let add_active = add
        .add_to_map_tail
        .unwrap()
        .add_to_active
        .expect("active exact typed GameObject should consume AddToActive seam");

    assert_eq!(
        add_active.status,
        ActiveNonPlayerMutationStatusLikeCpp::Mutated
    );
    assert!(add_active.inserted_in_active_set);
    assert!(map.is_active_non_player_like_cpp(guid));
    assert_eq!(map.active_non_players_count_like_cpp(), 1);
    let add_lock = add_active.unload_lock.unwrap();
    assert_eq!(add_lock.spawn_id, 48501);
    assert_eq!(add_lock.respawn_grid, Some(respawn_grid));
    assert!(add_lock.lock_incremented);
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        1
    );

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove
        .remove_from_active
        .expect("active exact typed GameObject should consume RemoveFromActive seam");
    assert!(remove_active.removed_from_active_set);
    assert!(!map.is_active_non_player_like_cpp(guid));
    assert_eq!(map.active_non_players_count_like_cpp(), 0);
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        0
    );
}

#[test]
fn active_non_player_add_remove_creature_updates_set_and_unload_lock_like_cpp() {
    let mut map = test_map();
    let respawn_position = Position::xyz(1.0, 2.0, 3.0);
    let respawn_cell = Cell::from_world(respawn_position.x, respawn_position.y);
    let respawn_grid = GridCoord::new(respawn_cell.grid_x(), respawn_cell.grid_y());
    map.ensure_grid_loaded(&cell_from_grid_center(respawn_grid));
    let mut creature = test_creature_for_spawn(48502, 4850201, true);
    let guid = creature.guid();
    creature.set_ai_home_position(respawn_position);
    creature.unit_mut().world_mut().set_active(true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let add_active = add
        .add_to_map_tail
        .unwrap()
        .add_to_active
        .expect("active exact typed Creature should consume AddToActive seam");

    assert_eq!(
        add_active.status,
        ActiveNonPlayerMutationStatusLikeCpp::Mutated
    );
    assert!(add_active.inserted_in_active_set);
    assert_eq!(
        add_active.unload_lock.unwrap().respawn_grid,
        Some(respawn_grid)
    );
    assert!(map.is_active_non_player_like_cpp(guid));
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        1
    );

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove.remove_from_active.unwrap();
    assert!(remove_active.removed_from_active_set);
    assert_eq!(
        remove_active.unload_lock.unwrap().respawn_grid,
        Some(respawn_grid)
    );
    assert!(!map.is_active_non_player_like_cpp(guid));
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        0
    );
}

#[test]
fn active_non_player_zero_spawn_mutates_set_without_unload_lock_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(0, 4850301);
    let guid = gameobject.world().guid();
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let add_active = add.add_to_map_tail.unwrap().add_to_active.unwrap();
    assert!(add_active.inserted_in_active_set);
    assert!(add_active.spawn_id_zero_or_unsupported);
    assert!(add_active.unload_lock.is_none());
    assert!(map.is_active_non_player_like_cpp(guid));

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove.remove_from_active.unwrap();
    assert!(remove_active.removed_from_active_set);
    assert!(remove_active.spawn_id_zero_or_unsupported);
    assert!(remove_active.unload_lock.is_none());
    assert!(!map.is_active_non_player_like_cpp(guid));
}

#[test]
fn active_non_player_active_objects_near_grid_uses_real_active_set_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48504, 4850401);
    let guid = gameobject.world().guid();
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();
    let object_cell = Cell::from_world(
        gameobject.world().position().x,
        gameobject.world().position().y,
    );
    let object_grid = GridCoord::new(object_cell.grid_x(), object_cell.grid_y());
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.unmark_active_cell(object_cell.cell_coord());
    let grid = NGrid::from_coords(
        object_grid.x_coord as i32,
        object_grid.y_coord as i32,
        1000,
        true,
    );

    assert!(map.active_objects_near_grid(&grid));
    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    assert!(remove.remove_from_active.unwrap().removed_from_active_set);
    assert!(!map.active_objects_near_grid(&grid));

    let mut stale_map = test_map();
    let mut stale_gameobject = test_gameobject_for_spawn(48504, 4850402);
    stale_gameobject.world_mut().set_active(true);
    stale_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    let stale_guid = stale_gameobject.world().guid();
    stale_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(stale_gameobject).unwrap(),
        )
        .unwrap();
    stale_map.unmark_active_cell(object_cell.cell_coord());
    stale_map.active_non_players_like_cpp.remove(&stale_guid);
    assert!(!stale_map.active_objects_near_grid(&grid));
}

#[test]
fn active_non_player_visit_sources_use_real_set_and_filter_stale_like_cpp() {
    let mut map = test_map();
    let mut active = test_gameobject_for_spawn(48505, 4850501);
    let active_guid = active.world().guid();
    active.world_mut().set_active(true);
    active.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(active).unwrap())
        .unwrap();

    let mut stale = test_gameobject_for_spawn(48505, 4850502);
    let stale_guid = stale.world().guid();
    stale.world_mut().set_active(true);
    stale.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(stale).unwrap())
        .unwrap();
    map.active_non_players_like_cpp.remove(&stale_guid);
    map.active_non_players_like_cpp
        .insert(guid(HighGuid::GameObject, 4850503));

    let active_sources = map.represented_active_non_player_sources_like_cpp();
    assert_eq!(active_sources, vec![active_guid]);
    let plan = map.map_update_visit_plan_like_cpp(
        std::iter::empty::<MapUpdatePlayerSources>(),
        active_sources,
        std::iter::empty::<ObjectGuid>(),
        1,
    );
    assert!(plan.process_relocation_notifies);
    assert_eq!(plan.nearby_visit_centers, vec![active_guid]);
}

#[test]
fn map_grid_state_delayed_helper_active_expired_moves_to_idle_and_stops_like_cpp() {
    let mut map = test_map();
    let position = Position::xyz(3_000.0, 3_000.0, 0.0);
    assert!(map.load_grid(position.x, position.y));
    let cell = Cell::from_world(position.x, position.y);
    let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
    let grid = map.get_ngrid_mut(coord).unwrap();
    grid.set_state(GridStateKind::Active);
    grid.info_mut().reset_time_tracker(1);

    let summary = map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(summary.diff_ms, 1);
    assert_eq!(summary.visited, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.active_to_idle, 1);
    assert_eq!(summary.unloaded, 0);
    assert_eq!(summary.missing_after_snapshot, 0);
    assert_eq!(map.get_ngrid(coord).unwrap().state(), GridStateKind::Idle);
    assert_eq!(map.lifecycle().stops, 1);
}

#[test]
fn map_grid_state_delayed_helper_removal_lock_and_active_near_defer_unload_like_cpp() {
    let mut locked_map = test_map();
    let locked_position = Position::xyz(3_100.0, 3_100.0, 0.0);
    assert!(locked_map.load_grid(locked_position.x, locked_position.y));
    let locked_cell = Cell::from_world(locked_position.x, locked_position.y);
    let locked_coord = GridCoord::new(locked_cell.grid_x(), locked_cell.grid_y());
    let locked_grid = locked_map.get_ngrid_mut(locked_coord).unwrap();
    locked_grid.set_state(GridStateKind::Removal);
    locked_grid.info_mut().reset_time_tracker(1);
    locked_grid.info_mut().set_unload_explicit_lock(true);

    let locked_summary = locked_map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(locked_summary.visited, 1);
    assert_eq!(locked_summary.updated, 1);
    assert_eq!(locked_summary.unloaded, 0);
    assert_eq!(locked_summary.removal_deferred_or_reset, 1);
    assert_eq!(
        locked_map.get_ngrid(locked_coord).unwrap().state(),
        GridStateKind::Removal
    );

    let mut active_near_map = test_map();
    let active_position = Position::xyz(3_200.0, 3_200.0, 0.0);
    assert!(active_near_map.load_grid(active_position.x, active_position.y));
    let active_cell = Cell::from_world(active_position.x, active_position.y);
    let active_coord = GridCoord::new(active_cell.grid_x(), active_cell.grid_y());
    let active_grid = active_near_map.get_ngrid_mut(active_coord).unwrap();
    active_grid.set_state(GridStateKind::Removal);
    active_grid.info_mut().reset_time_tracker(1);
    active_near_map.mark_active_cell(active_cell.cell_coord());

    let active_summary = active_near_map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(active_summary.visited, 1);
    assert_eq!(active_summary.updated, 1);
    assert_eq!(active_summary.unloaded, 0);
    assert_eq!(active_summary.removal_deferred_or_reset, 1);
    assert_eq!(
        active_near_map.get_ngrid(active_coord).unwrap().state(),
        GridStateKind::Removal
    );
}

#[test]
fn add_to_map_gameobject_non_exact_paths_do_not_emit_typed_preinsert_evidence_like_cpp() {
    let mut already_map = test_map();
    let mut already_gameobject = test_gameobject_for_spawn(47903, 4790301);
    already_gameobject.set_represented_gameobject_model_like_cpp(true);
    let already_outcome = already_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(already_gameobject).unwrap(),
        )
        .unwrap();
    assert!(already_outcome.already_in_world);
    assert_eq!(
        already_outcome.gameobject_store_inserted_before_add_to_world,
        None
    );
    assert_eq!(
        already_outcome.gameobject_spawn_indexed_before_add_to_world,
        None
    );

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4790302, 571, 7, false);
    let generic_outcome = generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    assert!(!generic_outcome.already_in_world);
    assert_eq!(
        generic_outcome.gameobject_store_inserted_before_add_to_world,
        None
    );
    assert_eq!(
        generic_outcome.gameobject_spawn_indexed_before_add_to_world,
        None
    );
}

#[test]
fn gameobject_zone_script_create_precedes_store_insert_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48001, 4800101);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let zone_script = outcome
        .gameobject_zone_script_create
        .expect("exact typed GameObject should expose represented ZoneScript create boundary");
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback_boundary);
    assert!(!zone_script.script_dispatch_represented);
    assert!(!zone_script.object_store_present_before_callback);
    assert!(!zone_script.spawn_index_present_before_callback);
    assert_eq!(
        outcome.gameobject_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.gameobject_spawn_indexed_before_add_to_world,
        Some(true)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(48001)
            .contains(&guid)
    );
}

#[test]
fn gameobject_zone_script_create_skips_already_in_world_and_generic_paths_like_cpp() {
    let mut already_map = test_map();
    let already_gameobject = test_gameobject_for_spawn(48002, 4800201);
    let already_outcome = already_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(already_gameobject).unwrap(),
        )
        .unwrap();
    assert!(already_outcome.already_in_world);
    assert!(already_outcome.gameobject_zone_script_create.is_none());

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4800202, 571, 7, false);
    let generic_outcome = generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    assert!(!generic_outcome.already_in_world);
    assert!(generic_outcome.gameobject_zone_script_create.is_none());

    let mut non_gameobject_map = test_map();
    let mut creature = test_creature_for_spawn(48003, 4800301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let non_gameobject = non_gameobject_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(non_gameobject.gameobject_zone_script_create.is_none());
}

#[test]
fn gameobject_zone_script_remove_snapshots_before_model_spawn_unindex_like_cpp() {
    let mut map = test_map();
    let spawn_id = 48101;
    let mut gameobject = test_gameobject_for_spawn(spawn_id, 4810101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(map.map_object_record(guid).is_some());
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            .contains(&guid)
    );
    assert!(map.contains_gameobject_model_like_cpp(key));

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    let zone_script = outcome.gameobject_zone_script_remove.expect(
        "exact typed in-world GameObject should expose represented ZoneScript remove boundary",
    );
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback_boundary);
    assert!(!zone_script.script_dispatch_represented);
    assert!(zone_script.model_remove_pending_before_callback);
    assert!(zone_script.spawn_index_present_before_callback);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(map.map_object_record(guid).is_none());
    assert!(
        map.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            .is_empty()
    );
    assert!(!map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn gameobject_zone_script_remove_skips_generic_and_not_in_world_like_cpp() {
    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4810201, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();

    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();

    assert!(generic_removed.gameobject_zone_script_remove.is_none());

    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48102, 4810202);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(
            MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
        )
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();

    assert!(not_in_world_removed.gameobject_zone_script_remove.is_none());
}

#[test]
fn gameobject_add_to_owner_registers_owner_list_and_guid_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4820601);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48206, 4820602);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_owner = map.gameobject_add_to_owner_like_cpp(owner_guid, guid);

    assert_eq!(add_owner.guid, guid);
    assert_eq!(add_owner.owner_guid, owner_guid);
    assert!(add_owner.owner_found_as_unit_like);
    assert!(add_owner.gameobject_found);
    assert_eq!(add_owner.owner_guid_before, ObjectGuid::EMPTY);
    assert_eq!(add_owner.owner_guid_after, owner_guid);
    assert!(add_owner.gameobject_owner_empty_before);
    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.owner_guid_set);
    assert!(!add_owner.cooldown_start_represented);
    assert!(!add_owner.creature_ai_callback_represented);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![guid]
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), owner_guid);
}

#[test]
fn gameobject_add_to_owner_dispatches_creature_ai_summon_boundary_like_cpp() {
    let mut map = test_map();
    let mut owner = test_creature_for_spawn(48207, 4820701, true);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .ai
        .set_active(Some("NullCreatureAI"));
    let gameobject = test_gameobject_for_spawn(48207, 4820702);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_owner = map.gameobject_add_to_owner_like_cpp(owner_guid, guid);

    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.creature_ai_callback_represented);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().ai.just_summoned_gameobject_count,
        1
    );

    let mut disabled_map = test_map();
    let disabled_owner = test_creature_for_spawn(48208, 4820801, true);
    let disabled_owner_guid = disabled_owner.guid();
    let disabled_gameobject = test_gameobject_for_spawn(48208, 4820802);
    let disabled_guid = disabled_gameobject.world().guid();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_creature(disabled_owner).unwrap())
        .unwrap();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_game_object(disabled_gameobject).unwrap())
        .unwrap();

    let disabled_add_owner =
        disabled_map.gameobject_add_to_owner_like_cpp(disabled_owner_guid, disabled_guid);
    assert!(disabled_add_owner.registered_owned_gameobject);
    assert!(!disabled_add_owner.creature_ai_callback_represented);
}

#[test]
fn gameobject_add_to_owner_noops_for_missing_owner_or_preowned_gameobject_like_cpp() {
    let mut preowned_map = test_map();
    let owner = test_player_for_viewpoint(4820901);
    let owner_guid = owner.guid();
    let existing_owner_guid = ObjectGuid::create_player(1, 4820903);
    let mut gameobject = test_gameobject_for_spawn(48209, 4820902);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(existing_owner_guid);

    preowned_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let preowned = preowned_map.gameobject_add_to_owner_like_cpp(owner_guid, guid);
    assert!(preowned.owner_found_as_unit_like);
    assert!(preowned.gameobject_found);
    assert_eq!(preowned.owner_guid_before, existing_owner_guid);
    assert_eq!(preowned.owner_guid_after, existing_owner_guid);
    assert!(!preowned.gameobject_owner_empty_before);
    assert!(!preowned.registered_owned_gameobject);
    assert!(!preowned.owner_guid_set);

    let mut missing_owner_map = test_map();
    let gameobject = test_gameobject_for_spawn(48210, 4821002);
    let guid = gameobject.world().guid();
    missing_owner_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let missing = missing_owner_map
        .gameobject_add_to_owner_like_cpp(ObjectGuid::create_player(1, 4821001), guid);
    assert!(!missing.owner_found_as_unit_like);
    assert!(missing.gameobject_found);
    assert!(!missing.registered_owned_gameobject);
    assert_eq!(missing.owner_guid_after, ObjectGuid::EMPTY);
}

#[test]
fn gameobject_add_to_owner_slot_sets_effect_summon_slot_tail_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821101);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48211, 4821102);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_slot = map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 2);

    assert!(add_slot.add_owner.registered_owned_gameobject);
    assert_eq!(add_slot.slot, 2);
    assert_eq!(add_slot.slot_previous_guid, ObjectGuid::EMPTY);
    assert!(add_slot.slot_set);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(owner.unit().subsystems().control.gameobject_slots[2], guid);
}

#[test]
fn gameobject_add_to_owner_slot_keeps_cpp_guards_visible() {
    let mut invalid_slot_map = test_map();
    let owner = test_player_for_viewpoint(4821201);
    let owner_guid = owner.guid();
    let gameobject = test_gameobject_for_spawn(48212, 4821202);
    let guid = gameobject.world().guid();
    invalid_slot_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    invalid_slot_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let invalid_slot = invalid_slot_map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 99);
    assert!(invalid_slot.add_owner.registered_owned_gameobject);
    assert!(!invalid_slot.slot_set);
    let owner = invalid_slot_map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .gameobject_slots
            .iter()
            .all(ObjectGuid::is_empty)
    );

    let mut preowned_map = test_map();
    let owner = test_player_for_viewpoint(4821301);
    let owner_guid = owner.guid();
    let existing_owner_guid = ObjectGuid::create_player(1, 4821303);
    let mut gameobject = test_gameobject_for_spawn(48213, 4821302);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(existing_owner_guid);
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    preowned_map
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let preowned_slot = preowned_map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 1);
    assert!(!preowned_slot.add_owner.registered_owned_gameobject);
    assert!(!preowned_slot.slot_set);
    let owner = preowned_map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        ObjectGuid::EMPTY
    );
}

#[test]
fn gameobject_prepare_owner_slot_for_summon_recast_preserves_owner_auras_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821601);
    let owner_guid = owner.guid();
    let spell_id = 4821610;
    let recast_aura = AppliedAuraRef::new(spell_id, owner_guid, 0, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(recast_aura);
    let mut gameobject = test_gameobject_for_spawn(48216, 4821602);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(spell_id);
    gameobject.set_respawn_time(60);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 0)
            .slot_set
    );

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 0, spell_id);

    assert_eq!(cleanup.owner_guid, owner_guid);
    assert_eq!(cleanup.slot, 0);
    assert_eq!(cleanup.slot_guid_before, guid);
    assert!(cleanup.slot_had_guid);
    assert!(cleanup.gameobject_found);
    assert!(cleanup.recast_spell_id_cleared);
    assert!(cleanup.unit_pointer_owner_match);
    assert!(cleanup.respawn_time_cleared);
    assert!(cleanup.slot_cleared);
    assert!(!cleanup.cooldown_event_represented);
    let remove_owner = cleanup.remove_from_owner.unwrap();
    assert_eq!(remove_owner.spell_id, 0);
    assert!(remove_owner.unit_owned_gameobject_list_removed);
    assert!(remove_owner.unit_object_slot_cleared);
    assert!(!remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 0);
    assert!(cleanup.delete_outcome.is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[0],
        ObjectGuid::EMPTY
    );
    assert!(owner.unit().subsystems().auras.has_applied(recast_aura));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.spell_id(), 0);
    assert_eq!(gameobject.respawn_time(), 0);
}

#[test]
fn gameobject_prepare_owner_slot_for_summon_different_spell_removes_old_aura_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821701);
    let owner_guid = owner.guid();
    let old_spell_id = 4821710;
    let new_spell_id = 4821720;
    let old_aura = AppliedAuraRef::new(old_spell_id, owner_guid, 0, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(old_aura);
    let mut gameobject = test_gameobject_for_spawn(48217, 4821702);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(old_spell_id);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, guid, 1)
            .slot_set
    );

    let cleanup =
        map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 1, new_spell_id);

    assert!(cleanup.gameobject_found);
    assert!(!cleanup.recast_spell_id_cleared);
    assert!(cleanup.unit_pointer_owner_match);
    let remove_owner = cleanup.remove_from_owner.unwrap();
    assert_eq!(remove_owner.spell_id, old_spell_id);
    assert!(remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 1);
    assert!(cleanup.delete_outcome.is_some());

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(!owner.unit().subsystems().auras.has_applied(old_aura));
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        ObjectGuid::EMPTY
    );
}

#[test]
fn gameobject_prepare_owner_slot_for_summon_clears_missing_guid_without_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821801);
    let owner_guid = owner.guid();
    let missing_guid = guid(HighGuid::GameObject, 4821802);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(3, missing_guid)
    );
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 3, 4821810);

    assert!(cleanup.owner_found_as_unit_like);
    assert_eq!(cleanup.slot_guid_before, missing_guid);
    assert!(cleanup.slot_had_guid);
    assert!(!cleanup.gameobject_found);
    assert!(!cleanup.recast_spell_id_cleared);
    assert!(!cleanup.unit_pointer_owner_match);
    assert!(cleanup.remove_from_owner.is_none());
    assert!(!cleanup.respawn_time_cleared);
    assert!(cleanup.delete_outcome.is_none());
    assert!(cleanup.slot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[3],
        ObjectGuid::EMPTY
    );

    let invalid_slot =
        map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 99, 4821810);
    assert!(invalid_slot.owner_found_as_unit_like);
    assert_eq!(invalid_slot.slot_guid_before, ObjectGuid::EMPTY);
    assert!(!invalid_slot.slot_had_guid);
    assert!(!invalid_slot.slot_cleared);
}

#[test]
fn gameobject_prepare_owner_slot_for_summon_owner_mismatch_keeps_object_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4821901);
    let owner_guid = owner.guid();
    let other_owner_guid = ObjectGuid::create_player(1, 4821903);
    let spell_id = 4821910;
    let guid = guid(HighGuid::GameObject, 4821902);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(2, guid)
    );
    let mut gameobject = test_gameobject_for_spawn(48219, 4821902);
    gameobject.set_owner_guid_like_cpp(other_owner_guid);
    gameobject.set_spell_id(spell_id);
    gameobject.set_respawn_time(90);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let cleanup = map.gameobject_prepare_owner_slot_for_summon_like_cpp(owner_guid, 2, spell_id);

    assert!(cleanup.gameobject_found);
    assert!(cleanup.recast_spell_id_cleared);
    assert!(!cleanup.unit_pointer_owner_match);
    assert!(cleanup.remove_from_owner.is_none());
    assert!(!cleanup.respawn_time_cleared);
    assert!(cleanup.delete_outcome.is_none());
    assert!(cleanup.slot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);

    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), other_owner_guid);
    assert_eq!(gameobject.spell_id(), 0);
    assert_eq!(gameobject.respawn_time(), 90);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[2],
        ObjectGuid::EMPTY
    );
}

#[test]
fn gameobject_summon_object_for_owner_slot_creates_adds_and_slots_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4822001);
    let owner_guid = owner.guid();
    owner.unit_mut().set_faction(1735);
    owner.unit_mut().set_level(47);
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let position = Position::new(10.0, 11.0, 12.0, 1.25);
    let template = summon_gameobject_template_like_cpp(4822002, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.gameobject_summon_object_for_owner_slot_like_cpp(
        owner_guid, 1, 4822010, template, position, 12_345,
    );

    assert_eq!(
        outcome.status,
        GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
    );
    assert_eq!(outcome.low_guid, Some(1));
    let guid = outcome
        .guid
        .expect("summon should allocate a GameObject guid");
    assert_eq!(guid.entry(), 4822002);
    assert_eq!(outcome.respawn_time_secs, Some(12));
    assert_eq!(outcome.caster_faction, Some(1735));
    assert_eq!(outcome.caster_level, Some(47));
    assert!(!outcome.phase_inherit_represented);
    assert!(outcome.execute_log_represented);
    assert!(!outcome.cooldown_event_represented);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| {
        add.gameobject_store_inserted_before_add_to_world == Some(true)
            && add
                .add_to_map_tail
                .as_ref()
                .is_some_and(|tail| tail.update_object_visibility_on_create_represented)
    }));
    let add_owner_slot = outcome.add_owner_slot.unwrap();
    assert!(add_owner_slot.add_owner.registered_owned_gameobject);
    assert!(add_owner_slot.slot_set);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(owner.unit().subsystems().control.gameobject_slots[1], guid);
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![guid]
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), owner_guid);
    assert_eq!(gameobject.spell_id(), 4822010);
    assert_eq!(gameobject.respawn_time(), 12);
    assert_eq!(gameobject.data().faction_template, 1735);
    assert_eq!(gameobject.data().level, 47);
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(gameobject.world().position(), position);
    assert_eq!(
        gameobject.local_rotation_like_cpp(),
        gameobject_local_rotation_from_orientation_like_cpp(position.orientation)
    );
    assert_eq!(gameobject.spawn_id(), 0);
    assert!(!gameobject.respawn_compatibility_mode());
}

#[test]
fn gameobject_summon_object_for_owner_slot_missing_owner_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let template = summon_gameobject_template_like_cpp(4822102, 5);

    let outcome = map.gameobject_summon_object_for_owner_slot_like_cpp(
        ObjectGuid::create_player(1, 4822101),
        0,
        4822110,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        -1,
    );

    assert_eq!(
        outcome.status,
        GameObjectSummonObjectForOwnerSlotStatusLikeCpp::MissingOwner
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert!(outcome.add_owner_slot.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}

#[test]
fn world_object_summon_gameobject_position_keeps_explicit_coords_like_cpp() {
    let source = Position::new(10.0, 20.0, 30.0, 1.5);

    let outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 2.0, 4.0, 5.0, 6.0, 0.75,
    );

    assert_eq!(outcome.position, Position::new(4.0, 5.0, 6.0, 0.75));
    assert!(!outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(!outcome.collision_los_adjustment_represented);
}

#[test]
fn world_object_summon_gameobject_position_zero_coords_use_close_point_like_cpp() {
    let source = Position::new(10.0, 20.0, 30.0, 0.0);

    let outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 1.25, 0.0, 0.0, 0.0, 0.75,
    );

    assert_eq!(outcome.position, Position::new(12.5, 20.0, 30.0, 0.0));
    assert!(outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(!outcome.collision_los_adjustment_represented);

    let source = Position::new(10.0, 20.0, 30.0, std::f32::consts::FRAC_PI_2);
    let y_outcome = world_object_summon_gameobject_position_from_coords_like_cpp(
        source, 2.0, 0.0, 0.0, 0.0, 0.0,
    );
    assert!((y_outcome.position.x - 10.0).abs() < 0.00001);
    assert!((y_outcome.position.y - 24.0).abs() < 0.00001);
    assert_eq!(y_outcome.position.z, 30.0);
    assert_eq!(y_outcome.position.orientation, std::f32::consts::FRAC_PI_2);
}

#[test]
fn summon_object_wild_position_keeps_explicit_destination_like_cpp() {
    let caster = Position::new(10.0, 20.0, 30.0, 1.5);
    let destination = Position::new(4.0, 5.0, 6.0, 0.75);

    let outcome =
        spell_effect_summon_object_wild_position_like_cpp(caster, 2.0, 2.25, Some(destination));

    assert_eq!(outcome.position, destination);
    assert!(outcome.explicit_destination_used);
    assert!(!outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(outcome.focus_object_orientation_represented);
    assert!(!outcome.collision_los_adjustment_represented);
}

#[test]
fn summon_object_wild_position_missing_dst_uses_default_player_radius_like_cpp() {
    let caster = Position::new(10.0, 20.0, 30.0, 0.0);

    let outcome = spell_effect_summon_object_wild_position_like_cpp(caster, 1.25, 0.75, None);

    assert_eq!(
        outcome.position,
        Position::new(
            10.0 + 1.25 + DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP,
            20.0,
            30.0,
            0.75
        )
    );
    assert!(!outcome.explicit_destination_used);
    assert!(outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(outcome.focus_object_orientation_represented);
    assert!(!outcome.collision_los_adjustment_represented);
}

#[test]
fn world_object_summon_gameobject_player_owner_branch_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4822201);
    let owner_guid = owner.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let position = Position::new(4.0, 5.0, 6.0, 0.75);
    let template = summon_gameobject_template_like_cpp(4822202, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        position,
        45,
        GameObjectSummonTypeLikeCpp::TimedDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
    );
    assert_eq!(outcome.low_guid, Some(1));
    assert!(!outcome.phase_inherit_represented);
    assert!(!outcome.spawned_by_default_forced_false);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));
    let add_owner = outcome
        .add_owner
        .expect("player summoner always calls Unit::AddGameObject");
    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.owner_guid_set);
    let guid = outcome.guid.unwrap();

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![guid]
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), owner_guid);
    assert_eq!(gameobject.respawn_time(), 45);
    assert_eq!(gameobject.world().position(), position);
    assert!(gameobject.world().object().is_in_world());
    assert!(!gameobject.spawned_by_default());
    assert_eq!(gameobject.spell_id(), 0);
}

#[test]
fn world_object_summon_gameobject_unit_timed_despawn_forces_non_default_like_cpp() {
    let mut map = test_map();
    let owner = test_creature_for_spawn(48223, 4822301, true);
    let owner_guid = owner.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822302, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        Position::xyz(7.0, 8.0, 9.0),
        12,
        GameObjectSummonTypeLikeCpp::TimedDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
    );
    assert!(outcome.add_owner.is_none());
    assert!(outcome.spawned_by_default_forced_false);
    let guid = outcome.guid.unwrap();
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert!(!gameobject.spawned_by_default());
    assert_eq!(gameobject.respawn_time(), 12);
}

#[test]
fn world_object_summon_gameobject_not_in_world_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4822401);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822402, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.world_object_summon_gameobject_like_cpp(
        owner_guid,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        30,
        GameObjectSummonTypeLikeCpp::TimedOrCorpseDespawn,
    );

    assert_eq!(
        outcome.status,
        WorldObjectSummonGameObjectStatusLikeCpp::SummonerNotInWorld
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert!(outcome.add_owner.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}

#[test]
fn spell_effect_summon_object_wild_creates_spell_go_without_owner_like_cpp() {
    let mut map = test_map();
    let caster = test_player_for_viewpoint(4822501);
    let caster_guid = caster.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(caster).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822502, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);
    let position = Position::new(14.0, 15.0, 16.0, 1.5);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        caster_guid,
        4822510,
        template,
        position,
        23_456,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
    );
    assert_eq!(outcome.low_guid, Some(1));
    assert_eq!(outcome.respawn_time_secs, Some(23));
    assert!(!outcome.phase_inherit_represented);
    assert!(outcome.execute_log_represented);
    assert!(!outcome.owner_linked);
    assert!(!outcome.flagdrop_type);
    assert!(!outcome.flagdrop_player_branch_reached);
    assert!(!outcome.flagdrop_battleground_update_represented);
    assert!(outcome.linked_trap_guid.is_none());
    assert!(!outcome.linked_trap_side_effect_represented);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));

    let guid = outcome.guid.unwrap();
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.spell_id(), 4822510);
    assert_eq!(gameobject.respawn_time(), 23);
    assert_eq!(gameobject.world().position(), position);
    assert!(gameobject.world().object().is_in_world());
    assert!(!gameobject.spawned_by_default());
    let caster = map
        .map_object_record(caster_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        caster
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
}

#[test]
fn spell_effect_summon_object_wild_flagdrop_records_unrepresented_bg_branch_like_cpp() {
    let mut map = test_map();
    let caster = test_player_for_viewpoint(4822601);
    let caster_guid = caster.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(caster).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822602, GAMEOBJECT_TYPE_FLAGDROP);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        caster_guid,
        4822610,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        0,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
    );
    assert!(outcome.flagdrop_type);
    assert!(outcome.flagdrop_player_branch_reached);
    assert!(!outcome.flagdrop_battleground_update_represented);
    assert_eq!(outcome.respawn_time_secs, Some(0));
    let gameobject = map
        .map_object_record(outcome.guid.unwrap())
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().type_id, GAMEOBJECT_TYPE_FLAGDROP as i8);
    assert_eq!(gameobject.spell_id(), 4822610);
    assert_eq!(gameobject.respawn_time(), 0);
}

#[test]
fn spell_effect_summon_object_wild_missing_caster_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let template = summon_gameobject_template_like_cpp(4822702, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        ObjectGuid::create_player(1, 4822701),
        4822710,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        -1,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::MissingCaster
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}

#[test]
fn unit_remove_gameobjects_by_spell_filters_owner_list_without_slot_cleanup_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821401);
    let owner_guid = owner.guid();
    let mut matched_gameobject = test_gameobject_for_spawn(48214, 4821402);
    let matched_guid = matched_gameobject.world().guid();
    matched_gameobject.set_spell_id(4821410);
    let mut kept_gameobject = test_gameobject_for_spawn(48214, 4821403);
    let kept_guid = kept_gameobject.world().guid();
    kept_gameobject.set_spell_id(4821420);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(matched_gameobject).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(kept_gameobject).unwrap())
        .unwrap();

    assert!(
        map.gameobject_add_to_owner_slot_like_cpp(owner_guid, matched_guid, 1)
            .slot_set
    );
    assert!(
        map.gameobject_add_to_owner_like_cpp(owner_guid, kept_guid)
            .registered_owned_gameobject
    );

    let remove_by_spell = map.unit_remove_gameobjects_by_spell_like_cpp(owner_guid, 4821410, false);

    assert_eq!(remove_by_spell.owner_guid, owner_guid);
    assert_eq!(remove_by_spell.spell_id, 4821410);
    assert!(!remove_by_spell.delete_requested);
    assert!(remove_by_spell.owner_found_as_unit_like);
    assert_eq!(remove_by_spell.owned_entries_before, 2);
    assert_eq!(remove_by_spell.matched_entries, 1);
    assert_eq!(remove_by_spell.owner_guid_cleared, 1);
    assert_eq!(remove_by_spell.respawn_time_cleared, 0);
    assert_eq!(remove_by_spell.owner_list_entries_removed, 1);
    assert_eq!(remove_by_spell.delete_outcomes, 0);
    assert!(!remove_by_spell.object_slot_cleanup_represented);
    assert!(!remove_by_spell.aura_cleanup_represented);
    assert!(!remove_by_spell.cooldown_event_represented);
    assert!(!remove_by_spell.creature_ai_callback_represented);

    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().control.owned_gameobjects,
        vec![kept_guid]
    );
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[1],
        matched_guid
    );
    let matched = map
        .map_object_record(matched_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(matched.owner_guid(), ObjectGuid::EMPTY);
    let kept = map
        .map_object_record(kept_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(kept.owner_guid(), owner_guid);
}

#[test]
fn unit_remove_gameobjects_by_spell_delete_path_sets_respawn_zero_and_delete_like_cpp() {
    let mut map = test_map();
    let owner = test_player_for_viewpoint(4821501);
    let owner_guid = owner.guid();
    let mut gameobject = test_gameobject_for_spawn(48215, 4821502);
    let guid = gameobject.world().guid();
    gameobject.set_spell_id(4821510);
    gameobject.set_respawn_time(60);

    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    assert!(
        map.gameobject_add_to_owner_like_cpp(owner_guid, guid)
            .registered_owned_gameobject
    );

    let remove_by_spell = map.unit_remove_gameobjects_by_spell_like_cpp(owner_guid, 0, true);

    assert!(remove_by_spell.delete_requested);
    assert_eq!(remove_by_spell.owned_entries_before, 1);
    assert_eq!(remove_by_spell.matched_entries, 1);
    assert_eq!(remove_by_spell.owner_guid_cleared, 1);
    assert_eq!(remove_by_spell.respawn_time_cleared, 1);
    assert_eq!(remove_by_spell.owner_list_entries_removed, 1);
    assert_eq!(remove_by_spell.delete_outcomes, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.respawn_time(), 0);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
}

#[test]
fn gameobject_remove_from_owner_clears_owner_before_model_remove_like_cpp() {
    let mut map = test_map();
    let mut owner = test_player_for_viewpoint(4820101);
    let owner_guid = owner.guid();

    let mut gameobject = test_gameobject_for_spawn(48201, 4820102);
    let guid = gameobject.world().guid();
    let removed_aura = AppliedAuraRef::new(482001, owner_guid, 0, 0x1);
    let removed_owned_aura = OwnedAuraRef::new(482001, owner_guid, None);
    let kept_aura = AppliedAuraRef::new(482002, owner_guid, 1, 0x1);
    owner
        .unit_mut()
        .subsystems_mut()
        .control
        .register_owned_gameobject_like_cpp(guid);
    assert!(
        owner
            .unit_mut()
            .subsystems_mut()
            .control
            .set_gameobject_slot(1, guid)
    );
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(removed_aura);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_owned(removed_owned_aura);
    owner
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(kept_aura);
    map.insert_map_object_record(MapObjectRecord::new_player(owner).unwrap())
        .unwrap();

    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_owner_guid_like_cpp(owner_guid);
    gameobject.set_spell_id(482001);
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.insert_gameobject_model_like_cpp(key);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_owner = outcome
        .gameobject_remove_from_owner
        .expect("exact typed in-world GameObject should expose RemoveFromOwner boundary");

    assert_eq!(remove_owner.guid, guid);
    assert_eq!(remove_owner.owner_guid_before, owner_guid);
    assert_eq!(remove_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.cleared_owner);
    assert_eq!(remove_owner.spell_id, 482001);
    assert!(remove_owner.unit_side_effects_represented);
    assert!(remove_owner.unit_owned_gameobject_list_removed);
    assert!(remove_owner.unit_object_slot_cleared);
    assert!(remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 1);
    assert!(!remove_owner.cooldown_event_represented);
    assert!(!remove_owner.creature_ai_callback_represented);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
    assert!(
        owner
            .unit()
            .subsystems()
            .control
            .gameobject_slots
            .iter()
            .all(ObjectGuid::is_empty)
    );
    assert!(!owner.unit().subsystems().auras.has_applied(removed_aura));
    assert!(owner.unit().subsystems().auras.has_applied(kept_aura));
    assert_eq!(
        owner.unit().subsystems().auras.removed_auras,
        vec![removed_aura.aura_ref()]
    );
    assert!(
        !owner
            .unit()
            .subsystems()
            .auras
            .has_owned(removed_owned_aura)
    );
}

#[test]
fn gameobject_remove_from_owner_clears_lost_owner_fallback_like_cpp() {
    let mut map = test_map();
    let missing_owner_guid = ObjectGuid::create_player(1, 4820201);
    let mut gameobject = test_gameobject_for_spawn(48202, 4820202);
    let guid = gameobject.world().guid();
    gameobject.set_owner_guid_like_cpp(missing_owner_guid);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_owner = outcome.gameobject_remove_from_owner.unwrap();

    assert_eq!(remove_owner.owner_guid_before, missing_owner_guid);
    assert_eq!(remove_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(!remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.cleared_owner);
    assert!(!remove_owner.unit_side_effects_represented);
    assert!(!remove_owner.aura_cleanup_represented);
    assert_eq!(remove_owner.aura_cleanup_removed_count, 0);
}

#[test]
fn gameobject_remove_from_owner_dispatches_creature_ai_despawn_boundary_like_cpp() {
    let mut map = test_map();
    let mut owner = test_creature_for_spawn(48204, 4820401, true);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .ai
        .set_active(Some("NullCreatureAI"));

    let mut gameobject = test_gameobject_for_spawn(48204, 4820402);
    let guid = gameobject.world().guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .control
        .register_owned_gameobject_like_cpp(guid);
    gameobject.set_owner_guid_like_cpp(owner_guid);

    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let remove_owner = map
        .remove_from_map_like_cpp(guid, true)
        .unwrap()
        .gameobject_remove_from_owner
        .unwrap();

    assert!(remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.creature_ai_callback_represented);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(
        owner
            .unit()
            .subsystems()
            .ai
            .summoned_gameobject_despawn_count,
        1
    );

    let mut disabled_map = test_map();
    let disabled_owner = test_creature_for_spawn(48205, 4820501, true);
    let disabled_owner_guid = disabled_owner.guid();
    let mut disabled_gameobject = test_gameobject_for_spawn(48205, 4820502);
    let disabled_guid = disabled_gameobject.world().guid();
    disabled_gameobject.set_owner_guid_like_cpp(disabled_owner_guid);
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_creature(disabled_owner).unwrap())
        .unwrap();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_game_object(disabled_gameobject).unwrap())
        .unwrap();

    let disabled_remove_owner = disabled_map
        .remove_from_map_like_cpp(disabled_guid, true)
        .unwrap()
        .gameobject_remove_from_owner
        .unwrap();
    assert!(disabled_remove_owner.owner_found_as_unit_like);
    assert!(!disabled_remove_owner.creature_ai_callback_represented);
}

#[test]
fn gameobject_remove_from_owner_noops_empty_owner_generic_and_not_in_world_like_cpp() {
    let mut empty_owner_map = test_map();
    let empty_owner_gameobject = test_gameobject_for_spawn(48203, 4820301);
    let empty_owner_guid = empty_owner_gameobject.world().guid();
    empty_owner_map
        .insert_map_object_record(MapObjectRecord::new_game_object(empty_owner_gameobject).unwrap())
        .unwrap();

    let empty_owner_removed = empty_owner_map
        .remove_from_map_like_cpp(empty_owner_guid, true)
        .unwrap();
    let empty_owner = empty_owner_removed.gameobject_remove_from_owner.unwrap();
    assert_eq!(empty_owner.owner_guid_before, ObjectGuid::EMPTY);
    assert_eq!(empty_owner.owner_guid_after, ObjectGuid::EMPTY);
    assert!(!empty_owner.owner_found_as_unit_like);
    assert!(!empty_owner.cleared_owner);
    assert!(!empty_owner.aura_cleanup_represented);
    assert_eq!(empty_owner.aura_cleanup_removed_count, 0);

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4820302, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();
    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();
    assert!(generic_removed.gameobject_remove_from_owner.is_none());

    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48203, 4820303);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject.set_owner_guid_like_cpp(ObjectGuid::create_player(1, 4820304));
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(
            MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
        )
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(not_in_world_removed.gameobject_remove_from_owner.is_none());
}

#[test]
fn gameobject_linked_trap_remove_runs_before_owner_store_extraction_like_cpp() {
    let mut map = test_map();
    let mut trap = test_gameobject_for_spawn(48301, 4830101);
    let trap_guid = trap.world().guid();
    trap.set_represented_gameobject_model_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();

    let mut owner = test_gameobject_for_spawn(48302, 4830102);
    let owner_guid = owner.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_represented_gameobject_model_like_cpp(true);
    let owner_model_key = RepresentedGameObjectModelKeyLikeCpp { owner_guid };
    map.insert_gameobject_model_like_cpp(owner_model_key);
    map.insert_map_object_record(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    let linked_trap = outcome.gameobject_linked_trap_remove.expect(
        "exact typed in-world GameObject should expose linked-trap RemoveFromWorld evidence",
    );

    assert_eq!(linked_trap.guid, owner_guid);
    assert_eq!(linked_trap.linked_trap_guid, Some(trap_guid));
    assert!(linked_trap.owner_present_before_linked_trap_remove);
    assert!(!linked_trap.linked_trap_removed);
    assert!(linked_trap.linked_trap_remove_queued);
    assert!(!linked_trap.linked_trap_missing_or_self);
    assert!(!linked_trap.linked_trap_cycle_guarded);
    assert!(linked_trap.despawn_or_unsummon_scheduler_represented);
    assert!(!linked_trap.object_accessor_fanout_represented);
    assert!(outcome.gameobject_model_remove.is_some());
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(!map.contains_gameobject_model_like_cpp(owner_model_key));
}

#[test]
fn gameobject_linked_trap_remove_cycle_guard_allows_single_nested_remove_like_cpp() {
    let mut map = test_map();
    let mut owner = test_gameobject_for_spawn(48307, 4830401);
    let owner_guid = owner.world().guid();
    let mut trap = test_gameobject_for_spawn(48308, 4830402);
    let trap_guid = trap.world().guid();

    owner.set_linked_trap_like_cpp(trap_guid);
    trap.set_linked_trap_like_cpp(owner_guid);
    map.insert_map_object_record(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();

    let outcome = map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    let linked_trap = outcome.gameobject_linked_trap_remove.expect(
        "exact typed in-world GameObject should expose linked-trap RemoveFromWorld evidence",
    );

    assert_eq!(linked_trap.guid, owner_guid);
    assert_eq!(linked_trap.linked_trap_guid, Some(trap_guid));
    assert!(linked_trap.owner_present_before_linked_trap_remove);
    assert!(!linked_trap.linked_trap_removed);
    assert!(linked_trap.linked_trap_remove_queued);
    assert!(!linked_trap.linked_trap_missing_or_self);
    assert!(!linked_trap.linked_trap_cycle_guarded);
    assert!(linked_trap.despawn_or_unsummon_scheduler_represented);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_linked_trap_remove_noops_missing_self_and_empty_like_cpp() {
    let mut missing_map = test_map();
    let missing_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 0, 4830201);
    let mut missing_owner = test_gameobject_for_spawn(48303, 4830202);
    let missing_owner_guid = missing_owner.world().guid();
    missing_owner.set_linked_trap_like_cpp(missing_trap_guid);
    missing_map
        .insert_map_object_record(MapObjectRecord::new_game_object(missing_owner).unwrap())
        .unwrap();

    let missing_outcome = missing_map
        .remove_from_map_like_cpp(missing_owner_guid, true)
        .unwrap();
    let missing = missing_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(missing.linked_trap_guid, Some(missing_trap_guid));
    assert!(!missing.linked_trap_removed);
    assert!(missing.linked_trap_missing_or_self);
    assert!(!missing.linked_trap_cycle_guarded);
    assert!(missing_map.map_object_record(missing_owner_guid).is_none());

    let mut self_map = test_map();
    let mut self_owner = test_gameobject_for_spawn(48304, 4830203);
    let self_guid = self_owner.world().guid();
    self_owner.set_linked_trap_like_cpp(self_guid);
    self_map
        .insert_map_object_record(MapObjectRecord::new_game_object(self_owner).unwrap())
        .unwrap();

    let self_outcome = self_map.remove_from_map_like_cpp(self_guid, true).unwrap();
    let self_linked = self_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(self_linked.linked_trap_guid, Some(self_guid));
    assert!(!self_linked.linked_trap_removed);
    assert!(self_linked.linked_trap_missing_or_self);
    assert!(!self_linked.linked_trap_cycle_guarded);
    assert!(self_map.map_object_record(self_guid).is_none());

    let mut empty_map = test_map();
    let empty_owner = test_gameobject_for_spawn(48305, 4830204);
    let empty_guid = empty_owner.world().guid();
    empty_map
        .insert_map_object_record(MapObjectRecord::new_game_object(empty_owner).unwrap())
        .unwrap();

    let empty_outcome = empty_map
        .remove_from_map_like_cpp(empty_guid, true)
        .unwrap();
    let empty = empty_outcome.gameobject_linked_trap_remove.unwrap();
    assert_eq!(empty.linked_trap_guid, None);
    assert!(!empty.linked_trap_removed);
    assert!(empty.linked_trap_missing_or_self);
    assert!(empty_map.map_object_record(empty_guid).is_none());
}

#[test]
fn gameobject_linked_trap_remove_skips_not_in_world_and_generic_paths_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_gameobject = test_gameobject_for_spawn(48306, 4830301);
    let not_in_world_guid = not_in_world_gameobject.world().guid();
    not_in_world_gameobject.set_linked_trap_like_cpp(ObjectGuid::create_world_object(
        HighGuid::GameObject,
        0,
        1,
        571,
        7,
        0,
        4830302,
    ));
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(
            MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
        )
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(not_in_world_removed.gameobject_linked_trap_remove.is_none());

    let mut generic_map = test_map();
    let generic_object = world_object_with_counter(HighGuid::GameObject, 4830303, 571, 7, false);
    let generic_guid = generic_object.guid();
    generic_map
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, generic_object)
        .unwrap();

    let generic_removed = generic_map
        .remove_from_map_like_cpp(generic_guid, true)
        .unwrap();
    assert!(generic_removed.gameobject_linked_trap_remove.is_none());
}

#[test]
fn dynamic_tree_gameobject_add_without_model_evidence_leaves_tree_empty_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45102, 4510201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.unbalanced_after, 0);
}

#[test]
fn dynamic_tree_gameobject_already_in_world_add_does_not_insert_model_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45103, 4510301);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn dynamic_tree_gameobject_add_chest_ready_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45201, 4520101);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Ready, None);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
}

#[test]
fn dynamic_tree_gameobject_add_chest_non_ready_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45202, 4520201);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, None);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.new_collision_enabled, Some(false));
}

#[test]
fn dynamic_tree_gameobject_add_non_chest_ready_state_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45203, 4520301);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Ready);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
}

#[test]
fn dynamic_tree_gameobject_add_non_chest_active_state_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45204, 4520401);
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    gameobject.set_go_state(GoState::Active);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let collision = outcome.gameobject_collision_enable.unwrap();
    assert!(outcome.gameobject_model_insert.is_some());
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.new_collision_enabled, Some(false));
}

#[test]
fn dynamic_tree_gameobject_remove_consumes_contained_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45104, 4510401);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.world_mut().object_mut().remove_from_world();
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(map.contains_gameobject_model_like_cpp(key));

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    let remove = outcome
        .gameobject_model_remove
        .expect("contained represented model should be removed before final map removal");
    assert_eq!(
        remove.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(remove.model_count_before, 1);
    assert_eq!(remove.model_count_after, 0);
    assert_eq!(remove.unbalanced_before, 1);
    assert_eq!(remove.unbalanced_after, 2);
    assert!(!map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn dynamic_tree_gameobject_remove_missing_key_is_guarded_noop_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45105, 4510501);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_represented_gameobject_model_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(5);

    let outcome = map.remove_from_map_like_cpp(guid, true).unwrap();

    assert!(outcome.gameobject_model_remove.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.unbalanced_before, 5);
    assert_eq!(summary.unbalanced_after, 5);
}

#[test]
fn dynamic_tree_transport_add_excludes_immediate_gameobject_model_insert_like_cpp() {
    let mut map = test_map();
    let mut transport = test_transport_for_update(4510601, false);
    let guid = transport.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    transport
        .game_object_mut()
        .set_represented_gameobject_model_like_cpp(true);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.gameobject_model_insert.is_none());
    assert!(outcome.gameobject_collision_enable.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
}

#[test]
fn dynamic_tree_gameobject_update_model_removes_old_and_inserts_new_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45301, 4530101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, true, false);

    assert_eq!(outcome.status, GameObjectUpdateModelStatusLikeCpp::Updated);
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    let remove = outcome.old_model_remove.unwrap();
    assert_eq!(
        remove.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(remove.model_count_before, 1);
    assert_eq!(remove.model_count_after, 0);
    assert_eq!(remove.unbalanced_before, 1);
    assert_eq!(remove.unbalanced_after, 2);
    let insert = outcome.new_model_insert.unwrap();
    assert_eq!(
        insert.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(insert.model_count_before, 0);
    assert_eq!(insert.model_count_after, 1);
    assert_eq!(insert.unbalanced_before, 2);
    assert_eq!(insert.unbalanced_after, 3);
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}

#[test]
fn dynamic_tree_gameobject_update_model_to_no_model_removes_old_without_insert_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45302, 4530201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, false, true);

    assert_eq!(outcome.status, GameObjectUpdateModelStatusLikeCpp::Updated);
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    assert_eq!(
        outcome.old_model_remove.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert!(outcome.new_model_insert.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(!gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}

#[test]
fn dynamic_tree_gameobject_update_model_not_in_world_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45303, 4530301);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.update_gameobject_model_like_cpp(guid, false, false);

    assert_eq!(
        outcome.status,
        GameObjectUpdateModelStatusLikeCpp::NotInWorld
    );
    assert!(outcome.old_model_present);
    assert!(outcome.old_model_registered);
    assert!(outcome.old_model_remove.is_none());
    assert!(outcome.new_model_insert.is_none());
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        gameobject.data().flags & GO_FLAG_MAP_OBJECT,
        GO_FLAG_MAP_OBJECT
    );
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}

#[test]
fn dynamic_tree_gameobject_update_model_missing_and_wrong_kind_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4530401);
    let creature = test_creature_for_spawn(45304, 4530402, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4530403, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.update_gameobject_model_like_cpp(missing_guid, true, true);
    let wrong_kind = map.update_gameobject_model_like_cpp(creature_guid, true, true);
    let untyped = map.update_gameobject_model_like_cpp(untyped_guid, true, true);

    assert_eq!(
        missing.status,
        GameObjectUpdateModelStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectUpdateModelStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectUpdateModelStatusLikeCpp::WrongKind
    );
    assert!(missing.new_model_insert.is_none());
    assert!(wrong_kind.new_model_insert.is_none());
    assert!(untyped.new_model_insert.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}

#[test]
fn gameobject_display_set_in_world_writes_field_then_updates_model_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45401, 4540101);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(111);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 777, true, false);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(111));
    assert_eq!(outcome.new_display_id, Some(777));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::Updated
    );
    assert_eq!(
        update_model.old_model_remove.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(
        update_model.new_model_insert.unwrap().status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 777);
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(!gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}

#[test]
fn gameobject_display_set_not_in_world_preserves_old_model_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45402, 4540201);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(222);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_gameobject_model_like_cpp(key);

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 888, false, false);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(222));
    assert_eq!(outcome.new_display_id, Some(888));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::NotInWorld
    );
    assert!(update_model.old_model_present);
    assert!(update_model.old_model_registered);
    assert!(update_model.old_model_remove.is_none());
    assert!(update_model.new_model_insert.is_none());
    assert!(map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 888);
    assert!(gameobject.has_represented_gameobject_model_like_cpp());
    assert!(gameobject.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}

#[test]
fn gameobject_display_set_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4540301);
    let creature = test_creature_for_spawn(45403, 4540302, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4540303, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_display_id_like_cpp(missing_guid, 777, true, true);
    let wrong_kind = map.set_gameobject_display_id_like_cpp(creature_guid, 777, true, true);
    let untyped = map.set_gameobject_display_id_like_cpp(untyped_guid, 777, true, true);

    assert_eq!(
        missing.status,
        GameObjectSetDisplayIdStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetDisplayIdStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectSetDisplayIdStatusLikeCpp::WrongKind
    );
    assert!(missing.update_model.is_none());
    assert!(wrong_kind.update_model.is_none());
    assert!(untyped.update_model.is_none());
    assert_eq!(missing.previous_display_id, None);
    assert_eq!(wrong_kind.previous_display_id, None);
    assert_eq!(untyped.previous_display_id, None);
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}

#[test]
fn gameobject_display_set_does_not_infer_model_from_nonzero_display_id_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45404, 4540401);
    let guid = gameobject.world().guid();
    let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
    gameobject.set_display_id(0);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_display_id_like_cpp(guid, 999, false, true);

    assert_eq!(outcome.status, GameObjectSetDisplayIdStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_display_id, Some(0));
    assert_eq!(outcome.new_display_id, Some(999));
    let update_model = outcome.update_model.unwrap();
    assert_eq!(
        update_model.status,
        GameObjectUpdateModelStatusLikeCpp::Updated
    );
    assert!(!update_model.old_model_present);
    assert!(update_model.old_model_remove.is_none());
    assert!(update_model.new_model_insert.is_none());
    assert!(!map.contains_gameobject_model_like_cpp(key));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().display_id, 999);
    assert!(!gameobject.has_represented_gameobject_model_like_cpp());
    assert_eq!(gameobject.data().flags & GO_FLAG_MAP_OBJECT, 0);
}

#[test]
fn gameobject_set_go_state_ready_enables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45501, 4550101);
    let guid = gameobject.world().guid();
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(!outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, Some(true));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.previous_collision_enabled, None);
    assert_eq!(collision.new_collision_enabled, Some(true));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}

#[test]
fn gameobject_set_go_state_active_disables_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45502, 4550201);
    let guid = gameobject.world().guid();
    gameobject.set_go_state(GoState::Ready);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Active);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Ready as i8));
    assert_eq!(outcome.new_state, Some(GoState::Active as i8));
    assert_eq!(outcome.in_world_for_collision_branch, Some(true));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, false);
    assert_eq!(collision.previous_collision_enabled, Some(true));
    assert_eq!(collision.new_collision_enabled, Some(false));
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Active as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}

#[test]
fn gameobject_set_go_state_not_in_world_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45503, 4550301);
    let guid = gameobject.world().guid();
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    gameobject.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(!outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, Some(false));
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}

#[test]
fn gameobject_set_go_state_transport_type_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45504, 4550401);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_TRANSPORT as u8);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(false);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, None);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}

#[test]
fn gameobject_set_go_state_map_obj_transport_writes_state_without_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45506, 4550601);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_go_state_like_cpp(guid, GoState::Ready);

    assert_eq!(outcome.status, GameObjectSetGoStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_state, Some(GoState::Active as i8));
    assert_eq!(outcome.new_state, Some(GoState::Ready as i8));
    assert!(outcome.represented_model_present);
    assert!(outcome.transport_type);
    assert_eq!(outcome.in_world_for_collision_branch, None);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().state, GoState::Ready as i8);
    assert_eq!(
        gameobject.data().type_id,
        GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT as i8
    );
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );
}

#[test]
fn gameobject_set_go_state_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4550501);
    let creature = test_creature_for_spawn(45505, 4550502, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4550503, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_go_state_like_cpp(missing_guid, GoState::Ready);
    let wrong_kind = map.set_gameobject_go_state_like_cpp(creature_guid, GoState::Ready);
    let untyped = map.set_gameobject_go_state_like_cpp(untyped_guid, GoState::Ready);

    assert_eq!(
        missing.status,
        GameObjectSetGoStateStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetGoStateStatusLikeCpp::WrongKind
    );
    assert_eq!(untyped.status, GameObjectSetGoStateStatusLikeCpp::WrongKind);
    assert_eq!(missing.previous_state, None);
    assert_eq!(wrong_kind.previous_state, None);
    assert_eq!(untyped.previous_state, None);
    assert!(missing.collision_enable.is_none());
    assert!(wrong_kind.collision_enable.is_none());
    assert!(untyped.collision_enable.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}

#[test]
fn gameobject_set_loot_state_chest_activated_arms_restock_and_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45601, 4560101);
    let go_guid = gameobject.world().guid();
    let unit = guid(HighGuid::Player, 4560199);
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_gameobject_loot_state_like_cpp(
        go_guid,
        LootState::Activated,
        Some(unit),
        1_000,
        30,
        true,
    );

    assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
    assert_eq!(outcome.previous_loot_state, Some(LootState::NotReady));
    assert_eq!(outcome.new_loot_state, Some(LootState::Activated));
    assert_eq!(outcome.new_loot_state_unit_guid, Some(unit));
    assert!(outcome.ai_on_loot_state_changed_not_represented);
    assert!(outcome.restock_armed);
    assert_eq!(outcome.previous_restock_time, Some(0));
    assert_eq!(outcome.new_restock_time, Some(1_030));
    let collision = outcome.collision_enable.unwrap();
    assert_eq!(collision.requested_enable, true);
    assert_eq!(collision.new_collision_enabled, Some(true));
    let gameobject = map
        .map_object_record(go_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.restock_time(), 1_030);
    assert_eq!(gameobject.loot_state_unit_guid(), unit);
}

#[test]
fn gameobject_set_loot_state_restock_requires_changed_and_zero_previous_like_cpp() {
    let mut map = test_map();
    let mut unchanged = test_gameobject_for_spawn(45602, 4560201);
    let unchanged_guid = unchanged.world().guid();
    unchanged.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    map.insert_map_object_record(MapObjectRecord::new_game_object(unchanged).unwrap())
        .unwrap();

    let unchanged_outcome = map.set_gameobject_loot_state_like_cpp(
        unchanged_guid,
        LootState::Activated,
        None,
        2_000,
        60,
        false,
    );
    assert!(!unchanged_outcome.restock_armed);
    assert_eq!(unchanged_outcome.new_restock_time, Some(0));

    let mut already_restocking = test_gameobject_for_spawn(45603, 4560301);
    let already_guid = already_restocking.world().guid();
    already_restocking.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    already_restocking.set_restock_time_like_cpp(77);
    map.insert_map_object_record(MapObjectRecord::new_game_object(already_restocking).unwrap())
        .unwrap();

    let already_outcome = map.set_gameobject_loot_state_like_cpp(
        already_guid,
        LootState::Activated,
        None,
        2_000,
        60,
        true,
    );
    assert!(!already_outcome.restock_armed);
    assert_eq!(already_outcome.previous_restock_time, Some(77));
    assert_eq!(already_outcome.new_restock_time, Some(77));
}

#[test]
fn gameobject_set_loot_state_door_writes_loot_but_preserves_collision_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(45604, 4560401);
    let guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_DOOR as u8);
    gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
    gameobject.enable_represented_gameobject_collision_like_cpp(false);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome =
        map.set_gameobject_loot_state_like_cpp(guid, LootState::Ready, None, 3_000, 60, true);

    assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
    assert_eq!(outcome.new_loot_state, Some(LootState::Ready));
    assert!(outcome.door_type_early_return);
    assert!(outcome.collision_enable.is_none());
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.loot_state(), LootState::Ready);
    assert_eq!(
        gameobject.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}

#[test]
fn gameobject_set_loot_state_model_collision_condition_matches_cpp() {
    let mut map = test_map();
    let cases = [
        (4560501, GoState::Active, LootState::Ready, true),
        (4560502, GoState::Active, LootState::Activated, true),
        (4560503, GoState::Active, LootState::JustDeactivated, true),
        (4560504, GoState::Ready, LootState::Activated, false),
    ];

    for (counter, go_state, loot_state, expected_collision) in cases {
        let mut gameobject = test_gameobject_for_spawn(counter as SpawnId, counter);
        let guid = gameobject.world().guid();
        gameobject.set_go_state(go_state);
        gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
        gameobject.apply_represented_gameobject_model_creation_like_cpp(true, true);
        map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
            .unwrap();

        let outcome =
            map.set_gameobject_loot_state_like_cpp(guid, loot_state, None, 4_000, 0, true);

        assert_eq!(outcome.status, GameObjectSetLootStateStatusLikeCpp::Updated);
        let collision = outcome.collision_enable.unwrap();
        assert_eq!(collision.requested_enable, expected_collision);
        assert_eq!(collision.new_collision_enabled, Some(expected_collision));
    }
}

#[test]
fn gameobject_set_loot_state_missing_wrong_kind_and_untyped_are_no_mutation_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::GameObject, 4560601);
    let creature = test_creature_for_spawn(45606, 4560602, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped = world_object_with_counter(HighGuid::GameObject, 4560603, 571, 7, true);
    let untyped_guid = untyped.guid();
    map.insert_map_object(AccessorObjectKind::GameObject, untyped)
        .unwrap();

    let missing = map.set_gameobject_loot_state_like_cpp(
        missing_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );
    let wrong_kind = map.set_gameobject_loot_state_like_cpp(
        creature_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );
    let untyped = map.set_gameobject_loot_state_like_cpp(
        untyped_guid,
        LootState::Ready,
        None,
        5_000,
        10,
        true,
    );

    assert_eq!(
        missing.status,
        GameObjectSetLootStateStatusLikeCpp::MissingGameObject
    );
    assert_eq!(
        wrong_kind.status,
        GameObjectSetLootStateStatusLikeCpp::WrongKind
    );
    assert_eq!(
        untyped.status,
        GameObjectSetLootStateStatusLikeCpp::WrongKind
    );
    assert_eq!(missing.previous_loot_state, None);
    assert_eq!(wrong_kind.previous_loot_state, None);
    assert_eq!(untyped.previous_loot_state, None);
    assert!(missing.collision_enable.is_none());
    assert!(wrong_kind.collision_enable.is_none());
    assert!(untyped.collision_enable.is_none());
    assert_eq!(map.update_dynamic_tree_like_cpp(250).unbalanced_after, 0);
}

#[test]
fn script_schedule_due_prefix_drains_sorted_and_keeps_future_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let delayed = map.schedule_represented_script_action_like_cpp(
        100,
        10,
        script_guid(1),
        script_guid(2),
        script_guid(3),
        1001,
    );
    let due_a = map.schedule_represented_script_action_like_cpp(
        100,
        5,
        script_guid(4),
        script_guid(5),
        script_guid(6),
        1002,
    );
    let due_b = map.schedule_represented_script_action_like_cpp(
        100,
        5,
        script_guid(7),
        script_guid(8),
        script_guid(9),
        1003,
    );

    assert_eq!(delayed.represented_increase_count, 1);
    assert_eq!(due_a.represented_increase_count, 1);
    assert_eq!(due_b.represented_increase_count, 1);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 3);

    let summary = map.process_due_script_schedule_like_cpp(105);

    assert_eq!(summary.queued_before, 3);
    assert_eq!(summary.processed, 2);
    assert_eq!(summary.represented_decrease_count, 2);
    assert_eq!(summary.remaining, 1);
    assert!(!summary.empty_noop);
    assert_eq!(
        summary
            .processed_actions
            .iter()
            .map(|action| action.command_id)
            .collect::<Vec<_>>(),
        vec![1002, 1003]
    );
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
    assert_eq!(
        map.represented_executed_script_actions_like_cpp()
            .iter()
            .map(|action| action.command_id)
            .collect::<Vec<_>>(),
        vec![1002, 1003]
    );

    let delayed_summary = map.process_script_schedule_update_order_like_cpp(110);
    assert_eq!(delayed_summary.processed_actions, vec![delayed.scheduled]);
    assert_eq!(delayed_summary.remaining, 0);
    assert!(delayed_summary.lock_entered);
}

#[test]
fn script_schedule_empty_update_order_is_noop_without_lock_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);

    let summary = map.process_script_schedule_update_order_like_cpp(100);

    assert!(summary.empty_noop);
    assert_eq!(summary.queued_before, 0);
    assert_eq!(summary.processed, 0);
    assert_eq!(summary.remaining, 0);
    assert!(!summary.lock_entered);
    assert!(!map.is_script_schedule_locked_like_cpp());
}

#[test]
fn script_schedule_zero_delay_processes_immediately_when_unlocked_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);

    let outcome = map.schedule_represented_script_action_like_cpp(
        200,
        0,
        script_guid(11),
        script_guid(12),
        script_guid(13),
        2001,
    );

    let immediate = outcome
        .immediate_process
        .expect("zero-delay represented schedule should process while unlocked");
    assert_eq!(immediate.queued_before, 1);
    assert_eq!(immediate.processed, 1);
    assert_eq!(immediate.remaining, 0);
    assert!(immediate.lock_entered);
    assert_eq!(immediate.processed_actions, vec![outcome.scheduled]);
    assert_eq!(outcome.remaining_after_schedule, 0);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 0);
}

#[test]
fn script_schedule_zero_delay_remains_queued_when_locked_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.set_script_schedule_lock_for_test(true);

    let outcome = map.schedule_represented_script_action_like_cpp(
        300,
        0,
        script_guid(21),
        script_guid(22),
        script_guid(23),
        3001,
    );

    assert!(outcome.immediate_process.is_none());
    assert_eq!(outcome.remaining_after_schedule, 1);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
    assert!(map.is_script_schedule_locked_like_cpp());
    map.set_script_schedule_lock_for_test(false);

    let summary = map.process_script_schedule_update_order_like_cpp(300);
    assert_eq!(summary.processed_actions, vec![outcome.scheduled]);
    assert_eq!(summary.remaining, 0);
}

#[test]
fn weather_timer_not_passed_does_not_call_default_weather_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44701);

    let summary = map.update_weather_like_cpp(999);

    assert_eq!(summary.interval_ms, 1_000);
    assert_eq!(summary.timer_current_before, 0);
    assert_eq!(summary.timer_current_after_update, 999);
    assert_eq!(summary.timer_current_after_reset, 999);
    assert!(!summary.timer_passed);
    assert_eq!(summary.zones_seen, 0);
    assert_eq!(summary.default_weather_updated, 0);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 999);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44701),
        Some([].as_slice())
    );
}

#[test]
fn weather_timer_passed_exact_interval_calls_default_weather_with_interval_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44702);

    let summary = map.update_weather_like_cpp(1_000);

    assert!(summary.timer_passed);
    assert_eq!(summary.timer_current_before, 0);
    assert_eq!(summary.timer_current_after_update, 1_000);
    assert_eq!(summary.timer_current_after_reset, 0);
    assert_eq!(summary.zones_seen, 1);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(summary.default_weather_removed, 0);
    assert_eq!(summary.weather_update_call_diff_ms, Some(1_000));
    assert!(summary.script_update_regeneration_fanout_not_represented);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 0);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44702),
        Some([1_000].as_slice())
    );
}

#[test]
fn weather_timer_overshoot_reset_preserves_modulo_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44703);

    let summary = map.update_weather_like_cpp(2_500);

    assert!(summary.timer_passed);
    assert_eq!(summary.timer_current_after_update, 2_500);
    assert_eq!(summary.timer_current_after_reset, 500);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 500);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44703),
        Some([1_000].as_slice())
    );
}

#[test]
fn weather_update_false_return_resets_default_weather_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44704);
    assert!(map.set_represented_zone_default_weather_next_update_alive_for_test(44704, false));

    let summary = map.update_weather_like_cpp(1_000);

    assert!(summary.timer_passed);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(summary.default_weather_removed, 1);
    assert!(
        map.represented_zone_dynamic_info_like_cpp(44704)
            .and_then(|zone| zone.default_weather.as_ref())
            .is_none()
    );
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LosCall {
    source_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    from: Position,
    to: Position,
    check_dynamic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HeightCall {
    object_guid: ObjectGuid,
    x: f32,
    y: f32,
    z: f32,
    query: WorldObjectHeightQuery,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloorCall {
    object_guid: ObjectGuid,
    position: Position,
    max_search_dist: f32,
}

#[derive(Debug)]
struct RecordingWorldObjectTerrain {
    los_result: bool,
    height_result: f32,
    floor_result: f32,
    los_calls: RefCell<Vec<LosCall>>,
    height_calls: RefCell<Vec<HeightCall>>,
    floor_calls: RefCell<Vec<FloorCall>>,
}

impl RecordingWorldObjectTerrain {
    fn new(los_result: bool, height_result: f32, floor_result: f32) -> Self {
        Self {
            los_result,
            height_result,
            floor_result,
            los_calls: RefCell::new(Vec::new()),
            height_calls: RefCell::new(Vec::new()),
            floor_calls: RefCell::new(Vec::new()),
        }
    }
}

impl TerrainGridLoader for RecordingWorldObjectTerrain {
    fn load_map_and_vmap(&mut self, _grid_x: u32, _grid_y: u32) {}
    fn unload_map(&mut self, _grid_x: u32, _grid_y: u32) {}
}

impl MapWorldObjectEnvironment for RecordingWorldObjectTerrain {
    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.los_calls.borrow_mut().push(LosCall {
            source_guid: query.source.guid(),
            target_guid: query.target.map(WorldObject::guid),
            from: query.from.position,
            to: query.to.position,
            check_dynamic: query.options.check_dynamic,
        });
        self.los_result
    }

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        self.height_calls.borrow_mut().push(HeightCall {
            object_guid: object.guid(),
            x,
            y,
            z,
            query,
        });
        self.height_result
    }

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32 {
        self.floor_calls.borrow_mut().push(FloorCall {
            object_guid: object.guid(),
            position,
            max_search_dist,
        });
        self.floor_result
    }
}

#[derive(Debug, Default)]
struct RecordingLifecycle {
    loads: usize,
    stops: usize,
    evacuates: usize,
    cleans: usize,
    unloads: usize,
}

impl GridLifecycle for RecordingLifecycle {
    fn load_grid_objects(&mut self, _grid: &mut NGrid, _cell: &Cell) {
        self.loads += 1;
    }

    fn stop_grid_objects(&mut self, _grid: &NGrid) {
        self.stops += 1;
    }

    fn evacuate_grid(&mut self, _grid: &mut NGrid) {
        self.evacuates += 1;
    }

    fn clean_grid(&mut self, _grid: &mut NGrid) {
        self.cleans += 1;
    }

    fn unload_grid_objects(&mut self, _grid: &mut NGrid) {
        self.unloads += 1;
    }
}

fn test_map() -> Map<RecordingTerrain, RecordingLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        100.0,
        RecordingTerrain::default(),
        RecordingLifecycle::default(),
    )
}

fn guid_unload_test_map() -> Map<RecordingTerrain, GuidGridUnloadLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        100.0,
        RecordingTerrain::default(),
        GuidGridUnloadLifecycle::new(),
    )
}

fn world_object_environment_test_map(
    terrain: RecordingWorldObjectTerrain,
    visible_distance: f32,
) -> Map<RecordingWorldObjectTerrain, RecordingLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        visible_distance,
        terrain,
        RecordingLifecycle::default(),
    )
}

#[test]
fn guid_sequence_creature_starts_at_one_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Creature), Ok(3));
}

#[test]
fn guid_sequence_creature_and_gameobject_are_independent_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(2));
}

#[test]
fn guid_sequence_accepts_non_creature_gameobject_map_sources_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(1));
    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(1)
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(2));
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(2)
    );
}

#[test]
fn guid_sequence_is_map_instance_local_like_cpp() {
    let mut first_map = test_map();
    let mut second_map = test_map();

    assert_eq!(
        first_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(1)
    );
    assert_eq!(
        first_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
    assert_eq!(
        second_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(1)
    );
    assert_eq!(
        second_map.get_max_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
}

#[test]
fn guid_sequence_transport_can_be_set_for_future_global_sync_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.set_guid_sequence_like_cpp(HighGuid::Transport, 77),
        Ok(())
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Transport), Ok(77));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Transport), Ok(78));
}

#[test]
fn guid_sequence_rejects_non_map_local_high_guid_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::Player),
        Err(MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource {
            high: HighGuid::Player,
        })
    );
}

#[test]
fn urand_inclusive_like_cpp_stays_within_inclusive_bounds() {
    let mut map = test_map();
    map.seed_creature_level_rng_for_tests_like_cpp(0x407);

    let mut saw_min = false;
    let mut saw_max = false;
    for _ in 0..512 {
        let value = map.urand_inclusive_like_cpp(18, 20);
        assert!((18..=20).contains(&value));
        saw_min |= value == 18;
        saw_max |= value == 20;
    }

    assert!(saw_min, "inclusive C++ urand should be able to return min");
    assert!(saw_max, "inclusive C++ urand should be able to return max");
}

#[test]
#[should_panic(expected = "C++ urand requires max >= min")]
fn urand_inclusive_like_cpp_asserts_max_at_least_min_like_cpp() {
    let mut map = test_map();
    let _ = map.urand_inclusive_like_cpp(20, 18);
}

#[test]
fn select_creature_level_fixed_path_does_not_consume_rng_like_cpp() {
    let mut fixed_then_variable = test_map();
    fixed_then_variable.seed_creature_level_rng_for_tests_like_cpp(0x407);
    assert_eq!(
        fixed_then_variable.select_creature_level_like_cpp(19, 19),
        19
    );
    let after_fixed = fixed_then_variable.select_creature_level_like_cpp(18, 20);

    let mut variable_only = test_map();
    variable_only.seed_creature_level_rng_for_tests_like_cpp(0x407);
    let without_fixed = variable_only.select_creature_level_like_cpp(18, 20);

    assert_eq!(after_fixed, without_fixed);
    assert!((18..=20).contains(&after_fixed));
}

#[test]
fn map_init_pools_for_map_mutates_map_owned_pool_data_like_cpp() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, group)
        .expect("test pool group");
    pool_mgr.add_auto_spawn_pool_like_cpp(571, 10);
    let mut map = test_map();

    let plan = map.init_pools_for_map_like_cpp(
        &pool_mgr,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(plan.map_id, 571);
    assert_eq!(plan.planned(), 1);
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(101));
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(10), 1);
}

#[test]
fn world_object_visibility_range_reads_map_visible_distance_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(true, INVALID_HEIGHT, INVALID_HEIGHT),
        123.5,
    );
    let object = world_object(HighGuid::DynamicObject, 571, 7, true);

    assert_eq!(object.get_visibility_range(&map), 123.5);
}

#[test]
fn world_object_los_delegates_to_map_environment_hook_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(false, INVALID_HEIGHT, INVALID_HEIGHT),
        100.0,
    );
    let mut source = world_object(HighGuid::DynamicObject, 571, 7, true);
    source.relocate(Position::new(1.0, 2.0, 3.0, 0.25));
    let mut target = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, true);
    target.relocate(Position::new(4.0, 5.0, 6.0, 0.75));

    let result = source.is_within_los_in_map(
        &target,
        &map,
        wow_entities::LineOfSightOptions {
            check_dynamic: true,
        },
    );

    assert!(!result);
    assert_eq!(
        map.terrain().los_calls.borrow().as_slice(),
        &[LosCall {
            source_guid: source.guid(),
            target_guid: Some(target.guid()),
            from: Position::new(1.0, 2.0, 3.0, 0.0),
            to: Position::new(4.0, 5.0, 6.0, 0.0),
            check_dynamic: true,
        }]
    );
}

#[test]
fn world_object_map_height_and_floor_delegate_to_map_environment_hook_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(true, 88.0, 25.0),
        100.0,
    );
    let mut object = world_object(HighGuid::DynamicObject, 571, 7, true);
    object.relocate(Position::new(1.0, 2.0, 3.0, 0.25));
    object.set_static_floor_z(20.0);
    let height_query = WorldObjectHeightQuery {
        vmap: false,
        distance_to_search: 9.0,
    };

    let height = object.get_map_height(&map, 4.0, 5.0, 6.0, height_query);
    let floor = object.get_floor_z(&map);

    assert_eq!(height, 88.0);
    assert_eq!(floor, 25.0);
    assert_eq!(
        map.terrain().height_calls.borrow().as_slice(),
        &[HeightCall {
            object_guid: object.guid(),
            x: 4.0,
            y: 5.0,
            z: 6.5,
            query: height_query,
        }]
    );
    assert_eq!(
        map.terrain().floor_calls.borrow().as_slice(),
        &[FloorCall {
            object_guid: object.guid(),
            position: Position::new(1.0, 2.0, 3.5, 0.25),
            max_search_dist: 50.0,
        }]
    );
}

fn spawn_group(group_id: u32, flags: SpawnGroupFlags) -> SpawnGroupTemplateData {
    SpawnGroupTemplateData {
        group_id,
        name: format!("group-{group_id}"),
        map_id: 571,
        flags,
    }
}

const fn spawn_group_flags(left: SpawnGroupFlags, right: SpawnGroupFlags) -> SpawnGroupFlags {
    SpawnGroupFlags(left.0 | right.0)
}

fn spawn_data(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    spawn_group: SpawnGroupTemplateData,
) -> crate::spawn::SpawnData {
    crate::spawn::SpawnData {
        object_type,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group,
        id: 99,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    }
}

fn spawn_group_store(
    group: SpawnGroupTemplateData,
    mut spawns: Vec<crate::spawn::SpawnData>,
) -> (SpawnGroupTemplateData, SpawnStore) {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let rows = spawns
        .iter()
        .map(|spawn| crate::spawn::SpawnGroupMemberRow {
            group_id: group.group_id,
            spawn_type: spawn.object_type as u8,
            spawn_id: spawn.spawn_id,
        })
        .collect::<Vec<_>>();
    for spawn in &spawns {
        match spawn.object_type {
            SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                store.add_object_spawn(spawn, |_| false);
            }
            SpawnObjectType::AreaTrigger => store.add_area_trigger_spawn(spawn),
        }
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    for spawn in &mut spawns {
        spawn.spawn_group = templates
            .get(&group.group_id)
            .expect("group resolved")
            .clone();
    }
    (
        templates
            .get(&group.group_id)
            .expect("group resolved")
            .clone(),
        store,
    )
}

fn respawn_info(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    respawn_time: i64,
) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type,
        spawn_id,
        entry: 42,
        respawn_time,
        grid_id: 7,
    }
}

fn test_creature_for_spawn(spawn_id: SpawnId, counter: i64, alive: bool) -> Creature {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Creature, counter));
    creature.unit_mut().world_mut().object_mut().set_entry(42);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_death_state(DeathState::Alive);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(spawn_id);
    if !alive {
        creature.mark_ai_dead(1);
    }
    creature
}

fn test_gameobject_for_spawn(spawn_id: SpawnId, counter: i64) -> GameObject {
    let mut gameobject = GameObject::new();
    gameobject
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::GameObject, counter));
    gameobject.world_mut().object_mut().set_entry(42);
    gameobject.world_mut().set_map(571, 7).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    gameobject.world_mut().object_mut().add_to_world();
    gameobject.set_spawn_id(spawn_id);
    gameobject
}

fn money_loot_for_player_like_cpp(
    loot_guid: ObjectGuid,
    coins: u32,
    player: ObjectGuid,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid,
        coins,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn poll_immediately_ready<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;

    impl std::task::Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("expected the uncontended claim to be ready"),
    }
}

fn summon_gameobject_template_like_cpp(
    entry: u32,
    go_type: u32,
) -> GameObjectTemplateLifecycleRecord {
    GameObjectTemplateLifecycleRecord {
        entry,
        name: "spell summoned gameobject".to_string(),
        go_type,
        display_id: 400,
        scale: 1.0,
        faction: 35,
        flags: 0,
        data: [0; wow_entities::MAX_GAMEOBJECT_DATA],
        world_effect_id: 0,
        anim_kit_id: 0,
        level: 1,
        percent_health: 100,
        custom_param: 0,
    }
}

fn test_transport(counter: i64, in_world: bool) -> wow_entities::Transport {
    let mut transport = wow_entities::Transport::new();
    transport
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Transport, counter));
    transport.world_mut().set_map(571, 7).unwrap();
    transport.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        transport.world_mut().object_mut().add_to_world();
    }
    transport
}

fn test_pet(counter: i64, in_world: bool) -> wow_entities::Pet {
    let owner = ObjectGuid::create_player(1, 484_000);
    let mut pet = wow_entities::Pet::new(owner, wow_entities::PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Pet, counter));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 7)
        .unwrap();
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
    }
    pet
}

fn test_area_trigger_for_spawn(spawn_id: SpawnId, counter: i64) -> AreaTrigger {
    let mut area_trigger = AreaTrigger::new();
    area_trigger
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::AreaTrigger, counter));
    area_trigger.world_mut().object_mut().set_entry(42);
    area_trigger.world_mut().set_map(571, 7).unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    area_trigger.world_mut().object_mut().add_to_world();
    area_trigger.set_spawn_id(spawn_id);
    area_trigger
}

#[test]
fn game_event_smart_ai_candidates_count_exact_in_world_creature_gameobject_only_like_cpp() {
    let mut map = test_map();

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(55301, 5530101, true)).unwrap(),
    )
    .unwrap();

    let mut not_in_world_creature = test_creature_for_spawn(55302, 5530102, true);
    not_in_world_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_creature(not_in_world_creature).unwrap())
        .unwrap();

    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(55303, 5530103)).unwrap(),
    )
    .unwrap();

    let mut not_in_world_gameobject = test_gameobject_for_spawn(55304, 5530104);
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
    )
    .unwrap();

    let generic = world_object_with_counter(HighGuid::GameObject, 5530105, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::GameObject, generic)
        .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_transport(test_transport(553_106, true)).unwrap(),
    )
    .unwrap();

    let summary = map.game_event_smart_ai_script_candidates_like_cpp();

    assert_eq!(summary.maps_visited, 1);
    assert_eq!(summary.in_world_creature_candidates, 1);
    assert_eq!(summary.in_world_gameobject_candidates, 1);
    assert_eq!(summary.creature_ai_enabled_unrepresented, 1);
    assert_eq!(summary.script_dispatch_unrepresented, 2);
}

#[test]
fn grid_unload_actions_apply_to_map_owned_creature_record() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 3711);
    let mut creature = test_creature_for_spawn(371, 3711, true);
    creature.unit_mut().world_mut().set_current_cell(3, 4);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcomes = apply_grid_unload_actions(
        &mut map,
        [
            GridUnloadAction::CreatureRespawnRelocation(creature_guid),
            GridUnloadAction::CleanupsBeforeDelete(GridObjectKind::Creature, creature_guid),
            GridUnloadAction::DeleteObject(GridObjectKind::Creature, creature_guid),
        ],
    );

    assert_eq!(outcomes, vec![GridUnloadApplyOutcome::Applied; 3]);
    assert_eq!(map.map_object_count(), 1);
    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(creature.grid_unload_respawn_relocation_requested());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
    assert!(creature.grid_unload_delete_requested());
    assert_eq!(creature.unit().world().current_cell(), None);
}

#[test]
fn grid_unload_actions_apply_to_map_owned_gameobject_record() {
    let mut map = test_map();
    let go_guid = guid(HighGuid::GameObject, 3712);
    let mut gameobject = test_gameobject_for_spawn(372, 3712);
    gameobject.world_mut().set_current_cell(5, 6);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcomes = apply_grid_unload_actions(
        &mut map,
        [
            GridUnloadAction::GameObjectRespawnRelocation(go_guid),
            GridUnloadAction::CleanupsBeforeDelete(GridObjectKind::GameObject, go_guid),
            GridUnloadAction::DeleteObject(GridObjectKind::GameObject, go_guid),
        ],
    );

    assert_eq!(outcomes, vec![GridUnloadApplyOutcome::Applied; 3]);
    assert_eq!(map.map_object_count(), 1);
    let gameobject = map
        .map_object_record(go_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(gameobject.grid_unload_respawn_relocation_requested());
    assert_eq!(gameobject.cleanup_before_delete_count(), 1);
    assert!(gameobject.grid_unload_delete_requested());
    assert_eq!(gameobject.world().current_cell(), None);
}

#[test]
fn grid_unload_map_store_missing_and_kind_mismatch_are_best_effort() {
    let mut map = test_map();
    let go_guid = guid(HighGuid::GameObject, 3713);
    let gameobject = test_gameobject_for_spawn(373, 3713);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    assert_eq!(
        apply_grid_unload_action(
            &mut map,
            GridUnloadAction::CreatureRespawnRelocation(go_guid),
        ),
        GridUnloadApplyOutcome::MissingEntity
    );
    assert_eq!(
        apply_grid_unload_action(
            &mut map,
            GridUnloadAction::CreatureRespawnRelocation(guid(HighGuid::Creature, 3714)),
        ),
        GridUnloadApplyOutcome::MissingEntity
    );

    let gameobject = map
        .map_object_record(go_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(!gameobject.grid_unload_respawn_relocation_requested());
    assert_eq!(gameobject.cleanup_before_delete_count(), 0);
    assert!(!gameobject.grid_unload_delete_requested());
}

#[test]
fn map_owned_respawn_get_time_zero_area_trigger_and_inserted_timers_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::AreaTrigger, 10),
        0
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::AreaTrigger, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 200)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );

    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        100
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::AreaTrigger, 10),
        0
    );
}

#[test]
fn loaded_grid_area_trigger_records_callback_adds_map_owned_record_like_cpp() {
    let mut map = test_map();
    let group = SpawnGroupTemplateData::legacy_group();
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8801, group);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));

    let mut calls = Vec::new();
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, object_type, spawn_id| {
            calls.push((object_type, spawn_id));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(spawn_id, 880101))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(calls, vec![(SpawnObjectType::AreaTrigger, 8801)]);
    assert!(!summary.grid_not_loaded);
    assert_eq!(summary.metadata_entries, 1);
    assert_eq!(summary.loaded_grid_primary_records.len(), 1);
    assert_eq!(summary.add_to_map_errors, 0);
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(8801).is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(8801), 1);
}

#[test]
fn loaded_grid_area_trigger_records_respect_spawn_grid_load_state_like_cpp() {
    let mut map = test_map();
    let manual = spawn_group(90, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8802, manual);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));

    let mut callback_calls = 0;
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, _, _| {
            callback_calls += 1;
            None
        },
    );

    assert_eq!(callback_calls, 0);
    assert_eq!(summary.metadata_entries, 0);
    assert_eq!(summary.skipped_should_not_spawn, 1);
    assert!(summary.loaded_grid_primary_records.is_empty());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(8802).is_none());
}

#[test]
fn loaded_grid_area_trigger_records_skip_already_loaded_spawn_like_cpp() {
    let mut map = test_map();
    let group = SpawnGroupTemplateData::legacy_group();
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8803, group);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(8803, 880301)).unwrap(),
    )
    .unwrap();

    let mut callback_calls = 0;
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, _, _| {
            callback_calls += 1;
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(8803, 880302))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(callback_calls, 0);
    assert_eq!(summary.skipped_already_loaded, 1);
    assert_eq!(summary.metadata_entries, 0);
    assert!(summary.loaded_grid_primary_records.is_empty());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(8803), 1);
}

#[test]
fn map_owned_respawn_add_replace_remove_unload_and_timer_keys_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 150)),
        AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        100
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 90)),
        AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        90
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 80)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );

    let timer_keys = map.respawn_timer_keys_like_cpp().collect::<Vec<_>>();
    assert_eq!(
        timer_keys,
        vec![
            (SpawnObjectType::GameObject, 20),
            (SpawnObjectType::Creature, 10)
        ]
    );

    let removed = map.remove_respawn_time_like_cpp(SpawnObjectType::Creature, 10);
    assert_eq!(removed.map(|info| info.respawn_time), Some(90));
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.respawn_timer_keys_like_cpp().collect::<Vec<_>>(),
        vec![(SpawnObjectType::GameObject, 20)]
    );

    map.unload_all_respawn_infos_like_cpp();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        0
    );
    assert!(map.respawn_timer_keys_like_cpp().next().is_none());
}

#[test]
fn map_owned_respawn_grid_load_state_uses_map_timer_and_group_sources_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = spawn_data(SpawnObjectType::Creature, 42, manual.clone());
    store.add_object_spawn(&spawn, |_| false);

    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 42, 100));
    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.remove_respawn_time_like_cpp(SpawnObjectType::Creature, 42);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );
}

#[test]
fn spawned_pool_data_creature_gameobject_and_dispatcher_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::Creature, 101),
        Ok(false)
    );
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::GameObject, 202),
        Ok(false)
    );
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::AreaTrigger, 303),
        Err(SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(
            SpawnObjectType::AreaTrigger
        ))
    );

    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::GameObject, 202, 7),
        Ok(())
    );
    assert!(pool_data.is_spawned_creature_like_cpp(101));
    assert!(pool_data.is_spawned_gameobject_like_cpp(202));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 2);
    assert_eq!(
        pool_data.spawned_objects_like_cpp(),
        vec![
            (SpawnObjectType::Creature, 101),
            (SpawnObjectType::GameObject, 202),
        ]
    );
}

#[test]
fn spawned_pool_data_duplicate_add_and_remove_counter_semantics_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert!(pool_data.is_spawned_creature_like_cpp(101));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 2);

    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert!(!pool_data.is_spawned_creature_like_cpp(101));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 1);

    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::GameObject, 202, 99),
        Ok(())
    );
    assert_eq!(pool_data.get_spawned_objects_like_cpp(99), 0);
}

#[test]
fn spawned_pool_data_pool_subpool_membership_and_counts_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    pool_data.add_pool_spawn_like_cpp(70, 7);
    assert!(pool_data.is_spawned_pool_like_cpp(70));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(70), 0);
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 1);

    pool_data.remove_pool_spawn_like_cpp(70, 7);
    assert!(!pool_data.is_spawned_pool_like_cpp(70));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);

    pool_data.remove_pool_spawn_like_cpp(70, 7);
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
}

#[test]
fn grid_load_state_uses_map_pool_data_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    let mut creature_spawn = spawn_data(SpawnObjectType::Creature, 501, active.clone());
    creature_spawn.pool_id = 7;
    let mut gameobject_spawn = spawn_data(SpawnObjectType::GameObject, 502, active);
    gameobject_spawn.pool_id = 7;
    store.add_object_spawn(&creature_spawn, |_| false);
    store.add_object_spawn(&gameobject_spawn, |_| false);

    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, 501, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 502, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .remove_spawn_like_cpp(SpawnObjectType::Creature, 501, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));
}

#[test]
fn map_owned_respawn_process_due_respawns_delegates_to_owned_store_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 200));

    let actions = map.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::DoRespawn {
            object_type: SpawnObjectType::Creature,
            spawn_id: 10,
            grid_id: 7,
        }]
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );

    let future_actions = map.process_due_respawns_like_cpp(
        150,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );
    assert!(future_actions.is_empty());
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );
}

#[test]
fn process_respawns_delete_only_inactive_spawn_group_removes_map_owned_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 42, manual), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 42, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert_eq!(summary.blocked_missing_spawn_data, 0);
    assert_eq!(summary.blocked_pool_runtime, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 42),
        0
    );
}

#[test]
fn process_respawns_delete_only_active_due_timer_loaded_grid_blocks_do_respawn_and_preserves_timer_like_cpp()
 {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(13, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 43, active), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 43, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 43),
        100
    );
}

#[test]
fn process_respawns_allowed_unloaded_grid_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(16, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 47, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 48, active), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 47, 90));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 48,
        entry: 42,
        respawn_time: 100,
        grid_id: 8,
    });

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.processed_unloaded_grid_respawns, 2);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 47),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 48),
        0
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 47)
            .is_none()
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 48)
            .is_none()
    );
}

#[test]
fn process_respawns_delete_only_missing_metadata_preserves_timer_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 44, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.blocked_missing_spawn_data, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 44),
        100
    );
}

#[test]
fn process_respawns_loaded_grid_creature_loader_adds_record_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(397, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39701, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39701, 100));
    let expected_guid = guid(HighGuid::Creature, 3970101);
    let mut loader_calls = 0;

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |map, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 39701);
            let low = map
                .generate_low_guid_like_cpp(HighGuid::Creature)
                .expect("map-owned Creature low-guid allocator");
            assert_eq!(low, 1);
            let mut creature = test_creature_for_spawn(39701, 3970101, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(loader_calls, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39701),
        0
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(39701), 1);
    let record = map.map_object_record(expected_guid).unwrap();
    assert!(record.object().object().is_in_world());
    assert!(record.creature().is_some());
    assert!(map.get_creature_by_spawn_id_like_cpp(39701).is_some());
    let cell = Cell::from_world(record.object().position().x, record.object().position().y);
    let grid = map
        .get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap();
    let local_cell = grid
        .get_grid_type(cell.cell_x(), cell.cell_y())
        .expect("record inserted into target cell");
    assert!(local_cell.grid_objects.creatures.contains(&expected_guid));
}

#[test]
fn process_respawns_loaded_grid_gameobject_loader_adds_record_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(398, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 39801, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 39801, 100));
    let expected_guid = guid(HighGuid::GameObject, 3980101);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 39801);
            let mut gameobject = test_gameobject_for_spawn(39801, 3980101);
            gameobject.world_mut().object_mut().remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(gameobject).unwrap(),
            ))
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 39801),
        0
    );
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(39801), 1);
    let record = map.map_object_record(expected_guid).unwrap();
    assert!(record.object().object().is_in_world());
    assert!(record.game_object().is_some());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(39801).is_some());
    let cell = Cell::from_world(record.object().position().x, record.object().position().y);
    let grid = map
        .get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap();
    let local_cell = grid
        .get_grid_type(cell.cell_x(), cell.cell_y())
        .expect("record inserted into target cell");
    assert!(local_cell.grid_objects.gameobjects.contains(&expected_guid));
}

#[test]
fn process_respawns_loaded_grid_pre_add_records_are_best_effort_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(409, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 40901, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 40901, 100));
    let owner_guid = guid(HighGuid::GameObject, 4090101);
    let trap_guid = guid(HighGuid::GameObject, 4090102);
    let missing_trap_guid = guid(HighGuid::GameObject, 4090103);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 40901);
            let mut trap = test_gameobject_for_spawn(0, 4090102);
            trap.world_mut().object_mut().remove_from_world();
            let mut missing_trap = test_gameobject_for_spawn(0, 4090103);
            missing_trap.world_mut().object_mut().remove_from_world();
            missing_trap
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            let mut owner = test_gameobject_for_spawn(40901, 4090101);
            owner.world_mut().object_mut().remove_from_world();
            owner.set_linked_trap_like_cpp(trap_guid);
            Some(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: vec![
                    MapObjectRecord::new_game_object(trap).unwrap(),
                    MapObjectRecord::new_game_object(missing_trap).unwrap(),
                ],
                primary_record: MapObjectRecord::new_game_object(owner).unwrap(),
            })
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
    assert!(map.map_object_record(missing_trap_guid).is_none());

    map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let remove_list = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(remove_list.processed, 1);
    assert_eq!(remove_list.removed, 1);
    assert!(map.map_object_record(trap_guid).is_none());
}

#[test]
fn process_respawns_loaded_grid_loader_none_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(399, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39901, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39902, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39901, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39902, 100));

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            if spawn_id == 39901 {
                None
            } else {
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_creature(test_creature_for_spawn(spawn_id, 3990201, true))
                        .unwrap(),
                ))
            }
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(map.map_object_count(), 1);
    assert!(map.get_gameobject_by_spawn_id_like_cpp(39902).is_none());
    assert!(map.get_creature_by_spawn_id_like_cpp(39902).is_some());
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39901),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39902),
        0
    );
}

#[test]
fn process_respawns_unloaded_grid_allowed_branch_does_not_call_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(400, SpawnGroupFlags::NONE);
    let mut far_spawn = spawn_data(SpawnObjectType::Creature, 40001, active);
    far_spawn.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&far_spawn, |_| false);
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40001, 100));
    let mut loader_calls = 0;

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, _object_type, _spawn_id| {
            loader_calls += 1;
            None
        },
    );

    assert_eq!(loader_calls, 0);
    assert_eq!(summary.processed_unloaded_grid_respawns, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40001),
        0
    );
}

#[test]
fn process_respawns_loaded_grid_add_to_map_failure_counts_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(401, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 40101, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40101, 100));
    let expected_guid = guid(HighGuid::Creature, 4010101);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, _object_type, _spawn_id| {
            let mut creature = test_creature_for_spawn(40101, 4010101, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40101),
        0
    );
    assert!(map.map_object_record(expected_guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(40101), 0);
}

#[test]
fn process_respawns_pool_loaded_grid_spawn_one_loader_adds_record_and_removes_trigger_timer_like_cpp()
 {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(526, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52601, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52602, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 52601, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(526, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 526);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52601, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52602, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 526, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 52601, 526)
        .expect("test spawn pool relation");
    let expected_guid = guid(HighGuid::Creature, 5260201);
    let mut loader_calls = 0;

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
        true,
        |_map, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 52602);
            let mut creature = test_creature_for_spawn(52602, 5260201, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(loader_calls, 1);
    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(summary.pool_spawn_action_load_plans, Vec::new());
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 52601),
        0
    );
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(52602));
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(526),
        1
    );
    assert!(map.map_object_record(expected_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(52602), 1);
}

#[test]
fn process_respawns_pool_loaded_grid_spawn_one_loader_none_keeps_load_plan_evidence_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(527, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 52701, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 52702, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 52701, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(527, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 527);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52701, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52702, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 527, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 52701, 527)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 52702,
            respawn: false,
        }]
    );
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 52701),
        0
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(52702)
    );
}

#[test]
fn process_respawns_pool_loaded_grid_add_to_map_failure_counts_and_removes_trigger_timer_like_cpp()
{
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(528, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52801, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52802, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 52801, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(528, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 528);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52801, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52802, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 528, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 52801, 528)
        .expect("test spawn pool relation");
    let expected_guid = guid(HighGuid::Creature, 5280201);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 52802);
            let mut creature = test_creature_for_spawn(52802, 5280201, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 52801),
        0
    );
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(52802));
    assert!(map.map_object_record(expected_guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(52802), 0);
}

#[test]
fn process_respawns_pool_timer_updates_pool_plan_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    let inactive = spawn_group(15, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 45, active), |_| {
        false
    });
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 46, inactive), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 45, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 46, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(55, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(45, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(145, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 45, 55)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.pool_update_plans.len(), 1);
    assert_eq!(summary.blocked_pool_plan_errors, Vec::new());
    assert_eq!(summary.blocked_pool_runtime, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert!(map.pool_data_like_cpp().is_spawned_gameobject_like_cpp(145));
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(55), 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 45),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 46),
        0
    );
}

#[test]
fn process_respawns_pool_plan_despawn_one_removes_live_creature_and_gameobject_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(31, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 71, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 72, active), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(71, 7101, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(72, 7201)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 71, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 72, 100));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, 71, 171),
        Ok(())
    );
    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 72, 172),
        Ok(())
    );
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(171, PoolTemplateDataLikeCpp::new(0, 571));
    pool_mgr.insert_template_like_cpp(172, PoolTemplateDataLikeCpp::new(0, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 171);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(71, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 171, creature_group)
        .expect("test creature pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 71, 171)
        .expect("test creature pool relation");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 172);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(72, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 172, gameobject_group)
        .expect("test gameobject pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 72, 172)
        .expect("test gameobject pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(summary.processed_pool_timers, 2);
    assert_eq!(summary.pool_objects_removed, 2);
    assert_eq!(summary.pool_stale_index_entries, 0);
    assert_eq!(summary.pool_remove_errors, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(71), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(72), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 71),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 72),
        0
    );
}

#[test]
fn process_respawns_pool_respawn_one_despawns_without_removing_unrelated_respawn_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(32, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 73, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 74, active), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(73, 7301, true)).unwrap(),
    )
    .unwrap();
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 74, 150));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Creature,
        pool_id: 173,
        trigger_from: 73,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![PoolSpawnObjectActionLikeCpp::RespawnOne {
                kind: PoolMemberKindLikeCpp::Creature,
                guid: 73,
            }],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: true,
            ..PoolSpawnObjectPlanLikeCpp::default()
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(&plan, &store, &mut summary);

    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 73,
            respawn: true,
        }]
    );
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 74),
        150
    );
}

#[test]
fn process_respawns_pool_remove_respawn_time_action_removes_member_timer_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 75, 200));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::GameObject,
        pool_id: 175,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![
                PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 75,
                },
                PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 76,
                },
            ],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: false,
            ..PoolSpawnObjectPlanLikeCpp::default()
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(&plan, &store, &mut summary);

    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 75),
        0
    );
}

#[test]
fn process_respawns_pool_spawn_action_reports_unloaded_loaded_and_missing_spawn_data_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(33, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 76, active.clone()),
        |_| false,
    );
    let mut unloaded_spawn = spawn_data(SpawnObjectType::GameObject, 77, active);
    unloaded_spawn.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&unloaded_spawn, |_| false);
    let loaded_cell = cell_from_world(0.0, 0.0);
    map.ensure_grid_loaded(&loaded_cell);
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Creature,
        pool_id: 176,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Creature,
                    guid: 76,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 77,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Creature,
                    guid: 78,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    guid: 179,
                },
            ],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: false,
            ..PoolSpawnObjectPlanLikeCpp::default()
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(&plan, &store, &mut summary);

    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_missing_spawn_data, 1);
    assert_eq!(summary.pool_unsupported_action_kind, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 76,
            respawn: false,
        }]
    );
}

#[test]
fn process_respawns_pool_spawn_one_pool_applies_real_child_spawn_plan_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(34, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 80, active), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Pool,
        pool_id: 180,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
                kind: PoolMemberKindLikeCpp::Pool,
                guid: 181,
            }],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: false,
            child_pool_spawn_plans: vec![PoolSpawnPoolPlanLikeCpp {
                pool_id: 181,
                subplans: vec![PoolTypedSpawnPlanLikeCpp {
                    kind: PoolMemberKindLikeCpp::Creature,
                    pool_id: 181,
                    trigger_from: 0,
                    max_limit: Some(1),
                    object_plan: Some(PoolSpawnObjectPlanLikeCpp {
                        actions: vec![PoolSpawnObjectActionLikeCpp::SpawnOne {
                            kind: PoolMemberKindLikeCpp::Creature,
                            guid: 80,
                        }],
                        selected: vec![],
                        despawned_trigger: None,
                        respawned_trigger: false,
                        ..PoolSpawnObjectPlanLikeCpp::default()
                    }),
                    skip_reason: None,
                }],
            }],
            child_pool_despawn_plans: vec![],
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
        &plan,
        &store,
        &mut summary,
        Some(&mut |_, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 80);
            let mut creature = test_creature_for_spawn(spawn_id, 8001, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        }),
    );

    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_action_load_plans, vec![]);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(80), 1);
}

#[test]
fn spawn_pool_facade_mutates_pool_data_and_executes_loaded_grid_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(530, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 530101, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5301, crate::pool::PoolTemplateDataLikeCpp::new(1, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5301);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(530101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5301, creature_group)
        .expect("test creature pool group");
    let mut loader_calls = 0usize;

    let summary = map
        .spawn_pool_loaded_grid_records_like_cpp(
            &pool_mgr,
            5301,
            &store,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, object_type, spawn_id| {
                loader_calls += 1;
                assert_eq!(object_type, SpawnObjectType::Creature);
                assert_eq!(spawn_id, 530101);
                let mut creature = test_creature_for_spawn(spawn_id, 53010101, true);
                creature
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .remove_from_world();
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_creature(creature).unwrap(),
                ))
            },
        )
        .expect("spawn pool facade plan");

    assert_eq!(loader_calls, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530101)
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(530101), 1);
}

#[test]
fn spawn_pool_facade_filters_unloaded_grid_without_calling_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(531, SpawnGroupFlags::NONE);
    let mut unloaded = spawn_data(SpawnObjectType::Creature, 530201, active);
    unloaded.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&unloaded, |_| false);
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5302, crate::pool::PoolTemplateDataLikeCpp::new(1, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5302);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(530201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5302, creature_group)
        .expect("test creature pool group");
    let mut loader_calls = 0usize;

    let summary = map
        .spawn_pool_loaded_grid_records_like_cpp(
            &pool_mgr,
            5302,
            &store,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, _, _| {
                loader_calls += 1;
                None
            },
        )
        .expect("spawn pool facade plan");

    assert_eq!(loader_calls, 0);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530201)
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(530201), 0);
}

#[test]
fn despawn_pool_facade_removes_live_creature_and_gameobject_from_map_owned_state_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(528101, 52810101, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(528102, 52810201)).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, 528101, 5281)
        .expect("test creature pool state");
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 528102, 5281)
        .expect("test gameobject pool state");
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5281);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5281, creature_group)
        .expect("test creature pool group");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5281);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528102, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 5281, gameobject_group)
        .expect("test gameobject pool group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5281, false)
        .expect("despawn pool plan");

    assert_eq!(summary.pool_objects_removed, 2);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(528101), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(528102), 0);
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(528101)
    );
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(528102)
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5281),
        0
    );
}

#[test]
fn despawn_pool_facade_always_delete_removes_non_spawned_creature_gameobject_timers_not_pool_like_cpp()
 {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 528201, 200));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 528202, 200));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5282);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5282, creature_group)
        .expect("test creature pool group");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5282);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528202, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 5282, gameobject_group)
        .expect("test gameobject pool group");
    let mut pool_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 5282);
    pool_group.add_entry_like_cpp(PoolObjectLikeCpp::new(5283, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 5282, pool_group)
        .expect("test child-pool relation group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5282, true)
        .expect("despawn pool plan");

    assert_eq!(summary.pool_respawn_timers_removed, 2);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(summary.pool_objects_removed, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 528201),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 528202),
        0
    );
}

#[test]
fn despawn_pool_facade_consumes_child_pool_recursion_without_pool_timer_removal_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(528401, 52840101, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 528402, 250));
    map.pool_data_mut_like_cpp()
        .add_pool_spawn_like_cpp(5284, 5280);
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, 528401, 5284)
        .expect("test child creature pool state");
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut parent_pool_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 5280);
    parent_pool_group.add_entry_like_cpp(PoolObjectLikeCpp::new(5284, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 5280, parent_pool_group)
        .expect("test parent pool group");
    let mut child_creature_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5284);
    child_creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528401, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Creature,
            5284,
            child_creature_group,
        )
        .expect("test child creature group");
    let mut child_gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5284);
    child_gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528402, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::GameObject,
            5284,
            child_gameobject_group,
        )
        .expect("test child gameobject group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5280, true)
        .expect("despawn parent pool plan");

    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(528401), 0);
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(528401)
    );
    assert!(!map.pool_data_like_cpp().is_spawned_pool_like_cpp(5284));
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5280),
        0
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5284),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 528402),
        0
    );
}

#[test]
fn process_respawns_pool_plan_error_preserves_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 47, active), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 47, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(47, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 47, 55)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(summary.processed_pool_timers, 0);
    assert_eq!(
        summary.blocked_pool_plan_errors,
        vec![PoolMgrPlanErrorLikeCpp::MissingTemplate { pool_id: 55 }]
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 47),
        100
    );
    assert!(!map.pool_data_like_cpp().is_spawned_creature_like_cpp(47));
}

#[test]
fn process_respawns_delete_only_preserves_cpp_order_when_first_due_blocks_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(15, SpawnGroupFlags::NONE);
    let manual = spawn_group(16, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 50, active), |_| {
        false
    });
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 40, manual), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 50, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 50),
        90
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40),
        100
    );
}

#[test]
fn check_respawn_spawn_group_guard_inactive_manual_group_clears_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 42, manual), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::Creature, 42, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(
        outcome,
        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
    );
    assert_eq!(info.respawn_time, 0);
}

#[test]
fn check_respawn_spawn_group_guard_active_manual_group_preserves_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 42, manual.clone()),
        |_| false,
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    let mut info = respawn_info(SpawnObjectType::Creature, 42, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(outcome, CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn check_respawn_spawn_group_guard_system_group_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 43, system), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::GameObject, 43, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(outcome, CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn check_respawn_spawn_group_guard_missing_metadata_preserves_timer_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 44, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(
        outcome,
        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn check_respawn_live_object_guard_alive_creature_same_spawn_clears_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(21, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 51, group), |_| false);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(51, 51, true)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::Creature, 51, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}

#[test]
fn check_respawn_live_object_guard_dead_creature_same_spawn_allows_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(22, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 52, group), |_| false);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(52, 52, false)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::Creature, 52, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn check_respawn_live_object_guard_dynamic_escort_closure_allows_only_when_config_enabled_like_cpp()
{
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(23, SpawnGroupFlags::ESCORTQUESTNPC);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 53, group.clone()),
        |_| false,
    );
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(53, 53, true)).unwrap(),
    )
    .unwrap();

    let mut info_config_enabled = respawn_info(SpawnObjectType::Creature, 53, 100);
    let enabled_outcome = map.check_respawn_live_object_guard_like_cpp(
        &mut info_config_enabled,
        &store,
        true,
        |_, _| true,
    );
    assert_eq!(
        enabled_outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
    );
    assert_eq!(info_config_enabled.respawn_time, 100);

    let mut info_config_disabled = respawn_info(SpawnObjectType::Creature, 53, 100);
    let disabled_outcome = map.check_respawn_live_object_guard_like_cpp(
        &mut info_config_disabled,
        &store,
        false,
        |_, _| true,
    );
    assert_eq!(
        disabled_outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info_config_disabled.respawn_time, 0);
}

#[test]
fn check_respawn_live_object_guard_gameobject_same_spawn_clears_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(24, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 54, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(54, 54)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::GameObject, 54, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}

#[test]
fn check_respawn_live_object_guard_missing_spawn_data_preserves_timer_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 55, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn check_respawn_live_object_guard_area_trigger_unsupported_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(25, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::AreaTrigger, 56, group), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 56, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn spawn_id_store_two_live_creatures_same_spawn_blocks_respawn_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(26, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 57, group), |_| false);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(57, 5701, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(57, 5702, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(57), 2);
    let mut info = respawn_info(SpawnObjectType::Creature, 57, 100);
    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}

#[test]
fn game_event_change_equip_or_model_two_live_creatures_same_spawn_mutates_equipment_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(157, 15701, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(157, 15702, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(157, 9, 0, false);

    assert_eq!(outcome.indexed_guids, 2);
    assert_eq!(outcome.live_creatures_mutated, 2);
    assert_eq!(outcome.equipment_changed, 2);
    for guid in map.creature_spawn_id_store_guids_like_cpp(157) {
        let creature = map
            .map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap();
        assert_eq!(creature.equipment_id(), 9);
    }
}

#[test]
fn game_event_change_equip_or_model_missing_spawn_id_does_not_panic_like_cpp() {
    let mut map = test_map();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(158, 9, 123, false);

    assert_eq!(outcome.indexed_guids, 0);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.model_validation_unavailable, 0);
}

#[test]
fn game_event_change_equip_or_model_model_gate_reports_unavailable_without_display_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(159, 15901, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(159, 4, 123, false);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 1);
    assert_eq!(outcome.equipment_changed, 1);
    assert_eq!(outcome.display_changed, 0);
    assert_eq!(outcome.model_validation_unavailable, 1);
}

#[test]
fn game_event_change_equip_or_model_wrong_kind_index_entry_does_not_mutate_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(160, 16001, true)).unwrap(),
    )
    .unwrap();
    let guid = guid(HighGuid::Creature, 16001);
    if let Some(creature_index) = map.creatures_by_spawn_id.get_mut(&160) {
        creature_index.retain(|indexed_guid| *indexed_guid != guid);
    }
    map.creatures_by_spawn_id
        .entry(161)
        .or_default()
        .insert(guid);

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(161, 9, 0, false);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.stale_index_or_wrong_kind, 1);
}

#[test]
fn game_event_npc_flag_live_consumer_mutates_exact_spawn_low_bits_like_cpp() {
    let mut map = test_map();
    let mut first = test_creature_for_spawn(547, 54701, true);
    first.ai_ownership_mut().npc_flags = 0x1;
    let mut second = test_creature_for_spawn(547, 54702, true);
    second.ai_ownership_mut().npc_flags = 0x2;
    map.insert_map_object_record(MapObjectRecord::new_creature(first).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(second).unwrap())
        .unwrap();

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(547, 0x1_0000_00A5);

    assert_eq!(outcome.indexed_guids, 2);
    assert_eq!(outcome.live_creatures_mutated, 2);
    assert_eq!(outcome.npc_flags_low_applied, 2);
    assert_eq!(outcome.npc_flags2_applied, 2);
    for guid in map.creature_spawn_id_store_guids_like_cpp(547) {
        let creature = map
            .map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap();
        assert_eq!(creature.ai_ownership().npc_flags, 0xA5);
        assert_eq!(creature.ai_ownership().npc_flags2, 0x1);
        assert_eq!(creature.unit().data().npc_flags, [0xA5, 0x1]);
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_PARENT_BIT)
        );
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT)
        );
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1)
        );
    }
}

#[test]
fn game_event_npc_flag_live_consumer_wrong_kind_or_mismatched_spawn_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(548, 54801, true);
    creature.ai_ownership_mut().npc_flags = 0x11;
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let guid = guid(HighGuid::Creature, 54801);
    if let Some(creature_index) = map.creatures_by_spawn_id.get_mut(&548) {
        creature_index.retain(|indexed_guid| *indexed_guid != guid);
    }
    map.creatures_by_spawn_id
        .entry(549)
        .or_default()
        .insert(guid);

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(549, 0x22);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.stale_index_or_wrong_kind, 1);
    let creature = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(creature.ai_ownership().npc_flags, 0x11);
    assert_eq!(creature.ai_ownership().npc_flags2, 0);
}

#[test]
fn game_event_npc_flag_live_consumer_applies_upper_bits_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(550, 55001, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(550, 0xFFFF_FFFF_0000_0040);

    assert_eq!(outcome.live_creatures_mutated, 1);
    assert_eq!(outcome.npc_flags_low_applied, 1);
    assert_eq!(outcome.npc_flags2_applied, 1);
    let guid = map.creature_spawn_id_store_guids_like_cpp(550)[0];
    let creature = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(creature.ai_ownership().npc_flags, 0x40);
    assert_eq!(creature.ai_ownership().npc_flags2, 0xFFFF_FFFF);
    assert_eq!(creature.unit().data().npc_flags, [0x40, 0xFFFF_FFFF]);
}

#[test]
fn spawn_id_store_removing_creatures_prunes_index_and_guard_allows_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(27, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 58, group), |_| false);
    let first_guid = guid(HighGuid::Creature, 5801);
    let second_guid = guid(HighGuid::Creature, 5802);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(58, 5801, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(58, 5802, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 2);
    assert!(map.remove_map_object(first_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 1);

    let mut blocked_info = respawn_info(SpawnObjectType::Creature, 58, 100);
    let blocked =
        map.check_respawn_live_object_guard_like_cpp(&mut blocked_info, &store, false, |_, _| {
            false
        });
    assert_eq!(
        blocked,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(blocked_info.respawn_time, 0);

    assert!(map.remove_map_object(second_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 0);

    let mut allowed_info = respawn_info(SpawnObjectType::Creature, 58, 100);
    let allowed =
        map.check_respawn_live_object_guard_like_cpp(&mut allowed_info, &store, false, |_, _| {
            false
        });
    assert_eq!(allowed, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(allowed_info.respawn_time, 100);
}

#[test]
fn spawn_id_store_replacing_same_guid_moves_creature_spawn_id_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 5901);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(59, 5901, true)).unwrap(),
    )
    .unwrap();
    let previous = map
        .insert_map_object_record(
            MapObjectRecord::new_creature(test_creature_for_spawn(60, 5901, true)).unwrap(),
        )
        .unwrap();

    assert!(previous.is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(59), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(60), 1);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(60), vec![guid]);
}

#[test]
fn world_object_by_spawn_id_typed_getters_return_indexed_objects_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(67, 6701, true);
    let creature_guid = creature.unit().world().guid();
    let gameobject = test_gameobject_for_spawn(68, 6801);
    let gameobject_guid = gameobject.world().guid();
    let area_trigger = test_area_trigger_for_spawn(69, 6901);
    let area_trigger_guid = area_trigger.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let creature = map.get_creature_by_spawn_id_like_cpp(67).unwrap();
    assert_eq!(creature.unit().world().guid(), creature_guid);
    assert_eq!(
        creature.unit().world().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
    let gameobject = map.get_gameobject_by_spawn_id_like_cpp(68).unwrap();
    assert_eq!(gameobject.world().guid(), gameobject_guid);
    assert_eq!(gameobject.world().position(), Position::xyz(1.0, 2.0, 3.0));
    let area_trigger = map.get_area_trigger_by_spawn_id_like_cpp(69).unwrap();
    assert_eq!(area_trigger.world().guid(), area_trigger_guid);
    assert_eq!(
        area_trigger.world().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );

    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 67)
            .unwrap()
            .guid(),
        creature_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 68)
            .unwrap()
            .guid(),
        gameobject_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 69)
            .unwrap()
            .guid(),
        area_trigger_guid
    );
}

#[test]
fn world_object_by_spawn_id_absent_and_zero_spawn_return_none_like_cpp() {
    let mut map = test_map();

    assert!(map.get_creature_by_spawn_id_like_cpp(75).is_none());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(75).is_none());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(75).is_none());
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 75)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 75)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 75)
            .is_none()
    );

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(0, 6001, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(0, 6002)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(0, 6003)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.map_object_count(), 3);
    assert!(map.get_creature_by_spawn_id_like_cpp(0).is_none());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(0).is_none());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(0).is_none());
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 0)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 0)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 0)
            .is_none()
    );
}

#[test]
fn world_object_by_spawn_id_creature_prefers_alive_then_fallback_like_cpp() {
    let mut map = test_map();
    let dead_guid = guid(HighGuid::Creature, 7601);
    let alive_guid = guid(HighGuid::Creature, 7602);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(76, 7601, false)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(76, 7602, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(
        map.creature_spawn_id_store_guids_like_cpp(76),
        vec![dead_guid, alive_guid]
    );
    assert_eq!(
        map.get_creature_by_spawn_id_like_cpp(76)
            .unwrap()
            .unit()
            .world()
            .guid(),
        alive_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 76)
            .unwrap()
            .guid(),
        alive_guid
    );

    assert!(map.remove_map_object(alive_guid).is_some());
    assert_eq!(
        map.get_creature_by_spawn_id_like_cpp(76)
            .unwrap()
            .unit()
            .world()
            .guid(),
        dead_guid
    );
}

#[test]
fn world_object_by_spawn_id_gameobject_prefers_spawned_then_fallback_like_cpp() {
    let mut map = test_map();
    let despawned_guid = guid(HighGuid::GameObject, 7801);
    let spawned_guid = guid(HighGuid::GameObject, 7802);
    let mut despawned = test_gameobject_for_spawn(78, 7801);
    despawned.set_respawn_delay_time(30);
    despawned.set_respawn_time(100);
    despawned.set_spawned_by_default(true);
    let mut spawned = test_gameobject_for_spawn(78, 7802);
    spawned.set_respawn_delay_time(30);
    spawned.set_respawn_time(100);
    spawned.set_spawned_by_default(false);

    map.insert_map_object_record(MapObjectRecord::new_game_object(despawned).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(spawned).unwrap())
        .unwrap();

    assert_eq!(
        map.gameobject_spawn_id_store_guids_like_cpp(78),
        vec![despawned_guid, spawned_guid]
    );
    assert_eq!(
        map.get_gameobject_by_spawn_id_like_cpp(78)
            .unwrap()
            .world()
            .guid(),
        spawned_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 78)
            .unwrap()
            .guid(),
        spawned_guid
    );

    assert!(map.remove_map_object(spawned_guid).is_some());
    assert_eq!(
        map.get_gameobject_by_spawn_id_like_cpp(78)
            .unwrap()
            .world()
            .guid(),
        despawned_guid
    );
}

#[test]
fn area_trigger_spawn_id_store_indexes_and_gets_typed_object_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_spawn(70, 7001);
    let guid = area_trigger.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(70), 1);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(70),
        vec![guid]
    );
    let stored = map.get_area_trigger_by_spawn_id_like_cpp(70).unwrap();
    assert_eq!(stored.world().guid(), guid);
    assert_eq!(stored.spawn_id(), 70);
}

#[test]
fn area_trigger_spawn_id_store_remove_desindexes_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::AreaTrigger, 7101);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(71, 7101)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(71), 1);

    assert!(map.remove_map_object(guid).is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(71), 0);
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(71).is_none());
}

#[test]
fn area_trigger_spawn_id_store_replacing_same_guid_moves_spawn_id_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::AreaTrigger, 7201);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(72, 7201)).unwrap(),
    )
    .unwrap();
    let previous = map
        .insert_map_object_record(
            MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(73, 7201)).unwrap(),
        )
        .unwrap();

    assert!(previous.is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(72), 0);
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(73), 1);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(73),
        vec![guid]
    );
}

#[test]
fn area_trigger_spawn_id_store_multiple_same_spawn_keeps_multimap_cardinality_like_cpp() {
    let mut map = test_map();
    let first_guid = guid(HighGuid::AreaTrigger, 7401);
    let second_guid = guid(HighGuid::AreaTrigger, 7402);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(74, 7402)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(74, 7401)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(74), 2);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(74),
        vec![first_guid, second_guid]
    );
    assert_eq!(
        map.get_area_trigger_by_spawn_id_like_cpp(74)
            .unwrap()
            .world()
            .guid(),
        first_guid
    );
}

#[test]
fn area_trigger_spawn_id_store_absent_query_returns_none_like_cpp() {
    let map = test_map();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(75), 0);
    assert!(
        map.area_trigger_spawn_id_store_guids_like_cpp(75)
            .is_empty()
    );
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(75).is_none());
}

#[test]
fn spawn_id_store_gameobject_same_spawn_blocks_until_removed_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(28, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 61, group), |_| {
        false
    });
    let gameobject_guid = guid(HighGuid::GameObject, 6101);

    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(61, 6101)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(61), 1);

    let mut blocked_info = respawn_info(SpawnObjectType::GameObject, 61, 100);
    let blocked =
        map.check_respawn_live_object_guard_like_cpp(&mut blocked_info, &store, false, |_, _| {
            false
        });
    assert_eq!(
        blocked,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn
    );
    assert_eq!(blocked_info.respawn_time, 0);

    assert!(map.remove_map_object(gameobject_guid).is_some());
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(61), 0);

    let mut allowed_info = respawn_info(SpawnObjectType::GameObject, 61, 100);
    let allowed =
        map.check_respawn_live_object_guard_like_cpp(&mut allowed_info, &store, false, |_, _| {
            false
        });
    assert_eq!(allowed, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(allowed_info.respawn_time, 100);
}

#[test]
fn spawn_id_store_dead_creature_indexed_but_does_not_block_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(29, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 62, group), |_| false);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(62, 6201, false)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(62), 1);
    let mut info = respawn_info(SpawnObjectType::Creature, 62, 100);
    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}

#[test]
fn map_spawn_group_initial_state_system_active_and_not_toggleable() {
    let mut map = test_map();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);

    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
    assert!(map.is_spawn_group_active_like_cpp(Some(&system)));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&system), false),
        SpawnGroupActiveChange::SystemGroup
    );
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
    assert!(map.is_spawn_group_active_like_cpp(Some(&system)));
}

#[test]
fn map_spawn_group_manual_default_inactive_activate_toggles_deactivate_clears() {
    let mut map = test_map();
    let manual = spawn_group(10, SpawnGroupFlags::MANUAL_SPAWN);

    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&manual), true),
        SpawnGroupActiveChange::Toggled
    );
    assert!(map.spawn_group_state().is_toggled(manual.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));

    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&manual)),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!map.spawn_group_state().is_toggled(manual.group_id));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
}

#[test]
fn map_spawn_group_non_manual_default_active_deactivate_toggles_activate_clears() {
    let mut map = test_map();
    let automatic = spawn_group(11, SpawnGroupFlags::NONE);

    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&automatic)),
        SpawnGroupActiveChange::Toggled
    );
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));

    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&automatic), true),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
}

#[test]
fn map_spawn_group_missing_group_returns_false_and_does_not_mutate_toggles() {
    let mut map = test_map();

    assert!(!map.is_spawn_group_active_like_cpp(None));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(None, true),
        SpawnGroupActiveChange::MissingGroup
    );
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(None),
        SpawnGroupActiveChange::MissingGroup
    );
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}

#[test]
fn map_spawn_group_grid_load_bridge_uses_map_owned_toggle_state() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = crate::spawn::SpawnData {
        object_type: SpawnObjectType::Creature,
        spawn_id: 42,
        map_id: 571,
        db_data: true,
        spawn_group: manual.clone(),
        id: 99,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |_| false);

    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );
}

#[test]
fn map_spawn_group_init_bridge_skips_system_and_applies_condition_semantics() {
    let mut map = test_map();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);
    let manual = spawn_group(20, SpawnGroupFlags::MANUAL_SPAWN);
    let automatic = spawn_group(21, SpawnGroupFlags::NONE);
    let groups = [&system, &manual, &automatic];

    let changes = map.init_spawn_group_state_like_cpp(groups, |group| group.group_id == 20);

    assert_eq!(
        changes,
        vec![
            (20, SpawnGroupActiveChange::Toggled),
            (21, SpawnGroupActiveChange::Toggled)
        ]
    );
    assert!(!map.spawn_group_state().is_toggled(system.group_id));
    assert!(map.spawn_group_state().is_toggled(manual.group_id));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
}

#[test]
fn update_spawn_group_conditions_manual_active_condition_false_with_despawn_flag_plans_despawn() {
    let mut map = test_map();
    let manual = spawn_group(
        30,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual], |_| false);

    assert_eq!(
        actions,
        vec![(
            30,
            SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            }
        )]
    );
}

#[test]
fn update_spawn_group_conditions_manual_active_condition_true_is_noop() {
    let mut map = test_map();
    let manual = spawn_group(
        31,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual], |_| true);

    assert_eq!(actions, vec![(31, SpawnGroupConditionActionLikeCpp::Noop)]);
}

#[test]
fn update_spawn_group_conditions_automatic_inactive_condition_true_plans_spawn() {
    let mut map = test_map();
    let automatic = spawn_group(32, SpawnGroupFlags::NONE);
    map.set_spawn_group_inactive_like_cpp(Some(&automatic));

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| true);

    assert_eq!(
        actions,
        vec![(
            32,
            SpawnGroupConditionActionLikeCpp::Spawn {
                ignore_respawn: false,
                force: false
            }
        )]
    );
}

#[test]
fn update_spawn_group_conditions_automatic_active_condition_false_with_despawn_flag_plans_despawn()
{
    let map = test_map();
    let automatic = spawn_group(33, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| false);

    assert_eq!(
        actions,
        vec![(
            33,
            SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            }
        )]
    );
}

#[test]
fn update_spawn_group_conditions_automatic_active_condition_false_without_despawn_flag_sets_inactive()
 {
    let map = test_map();
    let automatic = spawn_group(34, SpawnGroupFlags::NONE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| false);

    assert_eq!(
        actions,
        vec![(34, SpawnGroupConditionActionLikeCpp::SetInactive)]
    );
}

#[test]
fn update_spawn_group_conditions_automatic_active_condition_true_is_noop() {
    let map = test_map();
    let automatic = spawn_group(35, SpawnGroupFlags::NONE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| true);

    assert_eq!(actions, vec![(35, SpawnGroupConditionActionLikeCpp::Noop)]);
}

#[test]
fn update_spawn_group_conditions_planner_is_pure_and_preserves_spawn_group_state() {
    let mut map = test_map();
    let manual = spawn_group(
        36,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    let automatic = spawn_group(37, SpawnGroupFlags::NONE);
    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    let before = map
        .spawn_group_state()
        .toggled_spawn_group_ids()
        .iter()
        .copied()
        .collect::<Vec<_>>();

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual, &automatic], |_| false);
    let after = map
        .spawn_group_state()
        .toggled_spawn_group_ids()
        .iter()
        .copied()
        .collect::<Vec<_>>();

    assert_eq!(
        actions,
        vec![
            (
                36,
                SpawnGroupConditionActionLikeCpp::Despawn {
                    delete_respawn_times: true
                }
            ),
            (37, SpawnGroupConditionActionLikeCpp::SetInactive),
        ]
    );
    assert_eq!(after, before);
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
}

#[test]
fn update_spawn_group_conditions_apply_automatic_condition_failure_without_despawn_sets_inactive() {
    let mut map = test_map();
    let automatic = spawn_group(38, SpawnGroupFlags::NONE);

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| false);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 38,
            action: SpawnGroupConditionActionLikeCpp::SetInactive,
            applied_change: Some(SpawnGroupActiveChange::Toggled),
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
}

#[test]
fn update_spawn_group_conditions_apply_automatic_condition_failure_with_despawn_only_plans_despawn()
{
    let mut map = test_map();
    let automatic = spawn_group(39, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| false);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 39,
            action: SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            },
            applied_change: None,
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}

#[test]
fn update_spawn_group_conditions_apply_automatic_inactive_condition_true_only_plans_spawn() {
    let mut map = test_map();
    let automatic = spawn_group(40, SpawnGroupFlags::NONE);
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&automatic)),
        SpawnGroupActiveChange::Toggled
    );

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| true);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 40,
            action: SpawnGroupConditionActionLikeCpp::Spawn {
                ignore_respawn: false,
                force: false,
            },
            applied_change: None,
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
}

#[test]
fn update_spawn_group_conditions_condition_failure_despawns_live_objects_and_timers_like_cpp() {
    let group = spawn_group(391, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let creature_spawn = spawn_data(
        SpawnObjectType::Creature,
        10,
        SpawnGroupTemplateData::default_group(),
    );
    let gameobject_spawn = spawn_data(
        SpawnObjectType::GameObject,
        20,
        SpawnGroupTemplateData::default_group(),
    );
    store.add_object_spawn(&creature_spawn, |_| false);
    store.add_object_spawn(&gameobject_spawn, |_| false);
    store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            crate::spawn::SpawnGroupMemberRow {
                group_id: group.group_id,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 10,
            },
            crate::spawn::SpawnGroupMemberRow {
                group_id: group.group_id,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 20,
            },
        ],
    );
    let group = templates.get(&391).expect("group resolved").clone();
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(10, 10, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(20, 20)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(10), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(20), 1);
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 100));

    let outcomes =
        map.apply_update_spawn_group_conditions_represented_like_cpp([&group], &store, |_| false);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
    );
    let despawn = outcomes[0].despawn_outcome.expect("despawn executed");
    assert_eq!(despawn.objects_removed, 2);
    assert_eq!(despawn.respawn_timers_removed, 2);
    assert_eq!(despawn.blocked_missing_group, 0);
    assert_eq!(despawn.blocked_system_group, 0);
    assert_eq!(despawn.unsupported_live_despawn_types, 0);
    assert_eq!(
        despawn.applied_inactive_change,
        Some(SpawnGroupActiveChange::Toggled)
    );
    map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(10), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(20), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        0
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn spawn_group_spawn_missing_or_system_group_blocks_without_activation_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    let system = spawn_group(3940, SpawnGroupFlags::SYSTEM);

    let missing = map.spawn_group_spawn_like_cpp(None, false, false, &store);
    let system_outcome = map.spawn_group_spawn_like_cpp(Some(&system), false, false, &store);

    assert_eq!(missing.blocked_missing_group, 1);
    assert_eq!(missing.applied_active_change, None);
    assert_eq!(system_outcome.blocked_system_group, 1);
    assert_eq!(system_outcome.applied_active_change, None);
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}

#[test]
fn spawn_group_spawn_loaded_grid_creature_and_gameobject_are_planned_but_not_created_like_cpp() {
    let group = spawn_group(3941, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                101,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                201,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.metadata_entries, 2);
    assert_eq!(
        outcome.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 2);
    assert_eq!(outcome.executed_loaded_grid_spawns, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(
        outcome.load_plans,
        vec![
            SpawnGroupSpawnLoadPlanLikeCpp {
                object_type: SpawnObjectType::Creature,
                spawn_id: 101,
                force: false,
            },
            SpawnGroupSpawnLoadPlanLikeCpp {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 201,
                force: false,
            },
        ]
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn spawn_group_spawn_respawn_timer_skips_unless_ignore_or_force_removes_like_cpp() {
    let group = spawn_group(3942, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            102,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut blocked_map = test_map();
    blocked_map.load_grid(0.0, 0.0);
    blocked_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));

    let blocked = blocked_map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(blocked.skipped_respawn_timer_active, 1);
    assert_eq!(blocked.load_plans.len(), 0);
    assert_eq!(
        blocked_map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 102),
        100
    );

    let mut ignore_map = test_map();
    ignore_map.load_grid(0.0, 0.0);
    ignore_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));
    let ignored = ignore_map.spawn_group_spawn_like_cpp(Some(&group), true, false, &store);

    assert_eq!(ignored.respawn_timers_removed, 1);
    assert_eq!(ignored.skipped_respawn_timer_active, 0);
    assert_eq!(ignored.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(
        ignore_map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 102),
        0
    );

    let mut force_map = test_map();
    force_map.load_grid(0.0, 0.0);
    force_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));
    let forced = force_map.spawn_group_spawn_like_cpp(Some(&group), false, true, &store);
    assert_eq!(forced.respawn_timers_removed, 1);
    assert_eq!(forced.load_plans[0].force, true);
}

#[test]
fn spawn_group_spawn_live_object_skip_is_bypassed_by_force_like_cpp() {
    let group = spawn_group(3943, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                103,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                203,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(103, 103, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(203, 203)).unwrap(),
    )
    .unwrap();

    let skipped = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(skipped.skipped_live_object_active, 2);
    assert!(skipped.load_plans.is_empty());
    assert_eq!(map.map_object_count(), 2);

    let forced = map.spawn_group_spawn_like_cpp(Some(&group), false, true, &store);
    assert_eq!(forced.skipped_live_object_active, 0);
    assert_eq!(forced.load_plans.len(), 2);
    assert_eq!(forced.respawn_timers_missing, 2);
    assert_eq!(map.map_object_count(), 2);
}

#[test]
fn spawn_group_spawn_difficulty_mismatch_precedes_unloaded_grid_like_cpp() {
    let group = spawn_group(3944, SpawnGroupFlags::NONE);
    let mut spawn = spawn_data(
        SpawnObjectType::Creature,
        104,
        SpawnGroupTemplateData::default_group(),
    );
    spawn.spawn_difficulties = vec![2];
    let (group, store) = spawn_group_store(group, vec![spawn]);
    let mut map = test_map();

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.skipped_difficulty_mismatch, 1);
    assert_eq!(outcome.skipped_unloaded_grid, 0);
    assert!(outcome.load_plans.is_empty());
}

#[test]
fn spawn_group_spawn_unloaded_grid_skips_before_plan_like_cpp() {
    let group = spawn_group(3945, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            205,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.skipped_unloaded_grid, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert!(outcome.load_plans.is_empty());
}

#[test]
fn spawn_group_spawn_area_trigger_skips_no_respawn_map_before_loader_like_cpp() {
    let group = spawn_group(3946, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::AreaTrigger,
            305,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    let mut loader_calls = 0;

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, _spawn_id, _force| {
            loader_calls += 1;
            None
        },
    );

    assert_eq!(outcome.metadata_entries, 1);
    assert_eq!(outcome.skipped_no_respawn_map, 1);
    assert_eq!(outcome.unsupported_spawn_types, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(loader_calls, 0);
    assert!(outcome.load_plans.is_empty());
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn spawn_group_spawn_loaded_grid_loader_some_inserts_gameobject_like_cpp() {
    let group = spawn_group(3947, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            207,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, object_type, spawn_id, force| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 207);
            assert!(!force);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 207)).unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(207), 1);
}

#[test]
fn spawn_group_spawn_loader_none_blocks_and_continues_to_later_member_like_cpp() {
    let group = spawn_group(3948, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                108,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                208,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    let mut calls = Vec::new();

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, object_type, spawn_id, force| {
            calls.push((object_type, spawn_id, force));
            if object_type == SpawnObjectType::Creature {
                None
            } else {
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 208))
                        .unwrap(),
                ))
            }
        },
    );

    assert_eq!(calls.len(), 2);
    assert_eq!(outcome.load_plans.len(), 2);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(208), 1);
}

#[test]
fn spawn_group_spawn_primary_add_to_map_failure_blocks_without_executed_like_cpp() {
    let group = spawn_group(3949, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            209,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, spawn_id, _force| {
            let mut gameobject = GameObject::new();
            gameobject
                .world_mut()
                .object_mut()
                .create(guid(HighGuid::GameObject, 209));
            gameobject.world_mut().object_mut().set_entry(42);
            gameobject.world_mut().set_map(999, 7).unwrap();
            gameobject
                .world_mut()
                .relocate(Position::xyz(1.0, 2.0, 3.0));
            gameobject.set_spawn_id(spawn_id);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(gameobject).unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 1);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(209), 0);
}

#[test]
fn spawn_group_spawn_passes_force_to_loader_after_force_bypasses_live_skip_like_cpp() {
    let group = spawn_group(3950, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            110,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(110, 110, true)).unwrap(),
    )
    .unwrap();
    let mut captured_force = Vec::new();

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        true,
        &store,
        |_map, _object_type, _spawn_id, force| {
            captured_force.push(force);
            None
        },
    );

    assert_eq!(captured_force, vec![true]);
    assert_eq!(outcome.skipped_live_object_active, 0);
    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
}

#[test]
fn spawn_group_spawn_loaded_grid_loader_some_inserts_creature_like_cpp() {
    let group = spawn_group(3951, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            111,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, spawn_id, _force| {
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(test_creature_for_spawn(spawn_id, 111, true))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(111), 1);
}

#[test]
fn update_spawn_group_conditions_spawn_branch_returns_spawn_outcome_and_activates_like_cpp() {
    let group = spawn_group(392, SpawnGroupFlags::NONE);
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let creature_spawn = spawn_data(
        SpawnObjectType::Creature,
        30,
        SpawnGroupTemplateData::default_group(),
    );
    store.add_object_spawn(&creature_spawn, |_| false);
    store.apply_spawn_groups_like_cpp(
        &mut templates,
        [crate::spawn::SpawnGroupMemberRow {
            group_id: group.group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id: 30,
        }],
    );
    let group = templates.get(&392).expect("group resolved").clone();
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);

    let outcomes =
        map.apply_update_spawn_group_conditions_represented_like_cpp([&group], &store, |_| true);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(outcomes[0].applied_change, None);
    assert_eq!(outcomes[0].despawn_outcome, None);
    let spawn = outcomes[0].spawn_outcome.as_ref().expect("spawn executed");
    assert_eq!(
        spawn.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(spawn.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(
        spawn.load_plans,
        vec![SpawnGroupSpawnLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 30,
            force: false,
        }]
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn update_spawn_group_conditions_spawn_group_spawn_loaded_grid_loader_some_inserts_primary_record_like_cpp()
 {
    let group = spawn_group(393, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            31,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);
    let mut calls = Vec::new();

    let outcomes = map.apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
        [&group],
        &store,
        |_| true,
        |_map, object_type, spawn_id, force| {
            calls.push((object_type, spawn_id, force));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 31)).unwrap(),
            ))
        },
    );

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(outcomes[0].applied_change, None);
    assert_eq!(outcomes[0].despawn_outcome, None);
    let spawn = outcomes[0].spawn_outcome.as_ref().expect("spawn executed");
    assert_eq!(calls, vec![(SpawnObjectType::GameObject, 31, false)]);
    assert_eq!(
        spawn.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(spawn.load_plans.len(), 1);
    assert_eq!(spawn.executed_loaded_grid_spawns, 1);
    assert_eq!(spawn.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(spawn.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(31), 1);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn update_spawn_group_conditions_apply_manual_condition_failure_never_sets_inactive() {
    let mut map = test_map();
    let manual_with_despawn = spawn_group(
        41,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    let manual_without_despawn = spawn_group(42, SpawnGroupFlags::MANUAL_SPAWN);
    map.set_spawn_group_active_like_cpp(Some(&manual_with_despawn), true);
    map.set_spawn_group_active_like_cpp(Some(&manual_without_despawn), true);

    let outcomes = map.apply_update_spawn_group_conditions_set_inactive_like_cpp(
        [&manual_with_despawn, &manual_without_despawn],
        |_| false,
    );

    assert_eq!(
        outcomes,
        vec![
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 41,
                action: SpawnGroupConditionActionLikeCpp::Despawn {
                    delete_respawn_times: true
                },
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 42,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
        ]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual_with_despawn)));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual_without_despawn)));
    assert!(
        map.spawn_group_state()
            .is_toggled(manual_with_despawn.group_id)
    );
    assert!(
        map.spawn_group_state()
            .is_toggled(manual_without_despawn.group_id)
    );
}

#[test]
fn update_spawn_group_conditions_apply_active_equals_should_is_noop_without_change() {
    let mut map = test_map();
    let automatic = spawn_group(43, SpawnGroupFlags::NONE);
    let manual = spawn_group(44, SpawnGroupFlags::MANUAL_SPAWN);

    let outcomes = map.apply_update_spawn_group_conditions_set_inactive_like_cpp(
        [&automatic, &manual],
        |group| group.group_id == automatic.group_id,
    );

    assert_eq!(
        outcomes,
        vec![
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 43,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 44,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
        ]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}

fn dynamic_respawn_context(spawn_type: Option<SpawnObjectType>) -> DynamicRespawnScalingContext {
    DynamicRespawnScalingContext {
        mode: 1,
        spawn_type,
        spawn_metadata_present: true,
        spawn_group_flags: Some(SpawnGroupFlags::DYNAMIC_SPAWN_RATE),
        is_battleground_or_arena: false,
        zone_player_count: Some(4),
        config: DynamicRespawnScalingConfig {
            creature_rate: 1.0,
            creature_minimum_secs: 30,
            gameobject_rate: 1.5,
            gameobject_minimum_secs: 60,
        },
    }
}

fn assert_dynamic_respawn_noop(
    context: DynamicRespawnScalingContext,
    reason: DynamicRespawnScalingNoopReason,
) {
    let outcome = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);
    assert_eq!(outcome.delay_secs, 120);
    assert_eq!(outcome.noop_reason, Some(reason));
    assert!(!outcome.was_scaled());
}

#[test]
fn dynamic_respawn_bg_or_arena_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.is_battleground_or_arena = true;

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::BattlegroundOrArena,
    );
}

fn linked_respawn_guid(high: HighGuid, entry: u32, spawn_id: SpawnId) -> ObjectGuid {
    ObjectGuid::create_world_object(high, 0, 0, 571, 0, entry, spawn_id as i64)
}

#[test]
fn linked_respawn_time_missing_link_returns_zero_like_cpp() {
    let map = test_map();
    let store = LinkedRespawnStoreLikeCpp::new();

    assert_eq!(
        map.get_linked_respawn_time_like_cpp(
            linked_respawn_guid(HighGuid::Creature, 42, 100),
            &store,
        ),
        0
    );
}

#[test]
fn linked_respawn_time_reads_creature_and_gameobject_timers_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1234,
        grid_id: 7,
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 300,
        entry: 88,
        respawn_time: 5678,
        grid_id: 7,
    });
    let slave_creature = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master_creature = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let slave_go = linked_respawn_guid(HighGuid::GameObject, 43, 101);
    let master_go = linked_respawn_guid(HighGuid::GameObject, 88, 300);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(slave_creature, master_creature);
    linked.insert_like_cpp(slave_go, master_go);

    assert_eq!(
        map.get_linked_respawn_time_like_cpp(slave_creature, &linked),
        1234
    );
    assert_eq!(
        map.get_linked_respawn_time_like_cpp(slave_go, &linked),
        5678
    );
}

#[test]
fn check_respawn_linked_respawn_guard_no_linked_time_leaves_info_unchanged_like_cpp() {
    let map = test_map();
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let original = info.clone();

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 5);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed
    );
    assert_eq!(info, original);
}

#[test]
fn check_respawn_linked_respawn_guard_self_link_sets_week_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 100,
        entry: 42,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, this);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 5);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn
    );
    assert_eq!(info.respawn_time, 1000 + WEEK_SECS_LIKE_CPP);
}

#[test]
fn check_respawn_linked_respawn_guard_infinite_time_sets_i64_max_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 200,
        entry: 77,
        respawn_time: i64::MAX,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::GameObject, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 15);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite
    );
    assert_eq!(info.respawn_time, i64::MAX);
}

#[test]
fn check_respawn_linked_respawn_guard_delays_by_max_now_or_linked_plus_jitter_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 900,
        grid_id: 7,
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 300,
        entry: 88,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this_past = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let this_future = linked_respawn_guid(HighGuid::GameObject, 43, 101);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this_past, linked_respawn_guid(HighGuid::Creature, 77, 200));
    linked.insert_like_cpp(
        this_future,
        linked_respawn_guid(HighGuid::GameObject, 88, 300),
    );

    let mut past = respawn_info(SpawnObjectType::Creature, 100, 55);
    let past_outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut past, &linked, 1000, 5);
    assert_eq!(
        past_outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    );
    assert_eq!(past.respawn_time, 1005);

    let mut future = respawn_info(SpawnObjectType::GameObject, 101, 55);
    future.entry = 43;
    let future_outcome =
        map.check_respawn_linked_respawn_guard_like_cpp(&mut future, &linked, 1000, 15);
    assert_eq!(
        future_outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    );
    assert_eq!(future.respawn_time, 1215);
}

#[test]
fn check_respawn_like_cpp_inactive_spawn_group_stops_before_live_and_linked_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(61, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, manual), |_| {
        false
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
    );
    assert_eq!(info.respawn_time, 0);
    assert!(!escort_checked);
}

#[test]
fn check_respawn_like_cpp_live_blocker_stops_before_linked_reschedule_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(62, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(100, 100, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}

#[test]
fn check_respawn_like_cpp_linked_delayed_runs_after_allowed_guards_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(63, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 11, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed);
    assert_eq!(info.respawn_time, 1211);
}

#[test]
fn check_respawn_like_cpp_allowed_path_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(64, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 101, group), |_| {
        false
    });
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::GameObject, 101, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnCompositeOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 55);
}

#[test]
fn check_respawn_like_cpp_missing_metadata_preserves_timer_and_stops_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 55);
    assert!(!escort_checked);
}

#[test]
fn check_respawn_like_cpp_unsupported_areatrigger_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(65, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::AreaTrigger, 102, group),
        |_| false,
    );
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 102, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 55);
}

#[test]
fn check_respawn_like_cpp_areatrigger_manual_inactive_group_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(66, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::AreaTrigger, 103, manual),
        |_| false,
    );
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 103, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 55);
    assert!(!escort_checked);
}

#[test]
fn process_respawns_composite_live_creature_blocker_deletes_due_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(67, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(100, 100, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 10));

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.deleted_live_object_blocker, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .is_none()
    );
}

#[test]
fn process_respawns_composite_live_gameobject_blocker_deletes_due_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(68, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 101, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(101, 101)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 101, 10));

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.deleted_live_object_blocker, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 101)
            .is_none()
    );
}

#[test]
fn process_respawns_composite_linked_respawn_reschedules_future_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(69, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 10));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &linked,
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.rescheduled_linked_respawns.len(), 1);
    assert_eq!(summary.blocked_linked_respawn_non_future, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.deleted_live_object_blocker, 0);
    let rescheduled = &summary.rescheduled_linked_respawns[0];
    assert_eq!(rescheduled.spawn_id, 100);
    assert_eq!(rescheduled.respawn_time, 1205);
    assert_eq!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .unwrap()
            .respawn_time,
        1205
    );
}

#[test]
fn process_respawns_composite_linked_reschedule_allows_later_due_delete_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(70, SpawnGroupFlags::NONE);
    let inactive = spawn_group(71, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, active), |_| {
        false
    });
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 101, inactive),
        |_| false,
    );
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 9));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 101, 10));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &linked,
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.rescheduled_linked_respawns.len(), 1);
    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert_eq!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .unwrap()
            .respawn_time,
        1205
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 101)
            .is_none()
    );
}

#[test]
fn dynamic_respawn_unsupported_type_and_missing_metadata_do_not_scale() {
    assert_dynamic_respawn_noop(
        dynamic_respawn_context(Some(SpawnObjectType::AreaTrigger)),
        DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
    );

    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.spawn_metadata_present = false;
    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
    );
}

#[test]
fn dynamic_respawn_without_dynamic_spawn_rate_flag_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.spawn_group_flags = Some(SpawnGroupFlags::NONE);

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::MissingDynamicSpawnRateFlag,
    );
}

#[test]
fn dynamic_respawn_missing_or_zero_players_do_not_scale() {
    let mut missing = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    missing.zone_player_count = None;
    assert_dynamic_respawn_noop(
        missing,
        DynamicRespawnScalingNoopReason::MissingZonePlayerCount,
    );

    let mut zero = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    zero.zone_player_count = Some(0);
    assert_dynamic_respawn_noop(zero, DynamicRespawnScalingNoopReason::ZeroZonePlayers);
}

#[test]
fn dynamic_respawn_adjust_factor_at_least_one_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.zone_player_count = Some(1);
    context.config.gameobject_rate = 1.0;

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::AdjustFactorAtLeastOne,
    );
}

#[test]
fn dynamic_respawn_delay_at_or_below_minimum_does_not_scale() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    let outcome = apply_dynamic_mode_respawn_scaling_like_cpp(60, context);

    assert_eq!(outcome.delay_secs, 60);
    assert_eq!(
        outcome.noop_reason,
        Some(DynamicRespawnScalingNoopReason::DelayAtOrBelowMinimum)
    );
}

#[test]
fn dynamic_respawn_gameobject_ceil_scales_and_clamps_to_minimum() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    let scaled = apply_dynamic_mode_respawn_scaling_like_cpp(241, context);

    assert_eq!(scaled.delay_secs, 91);
    assert!(scaled.was_scaled());

    let clamped = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);
    assert_eq!(clamped.delay_secs, 60);
    assert!(clamped.was_scaled());
}

#[test]
fn dynamic_respawn_creature_uses_creature_rate_and_minimum() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::Creature));
    let scaled = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);

    assert_eq!(scaled.delay_secs, 30);
    assert!(scaled.was_scaled());
}

#[test]
fn dynamic_respawn_unsupported_mode_is_safe_noop() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.mode = 2;

    assert_dynamic_respawn_noop(context, DynamicRespawnScalingNoopReason::UnsupportedMode);
}

fn guid(high: HighGuid, counter: i64) -> ObjectGuid {
    if high == HighGuid::Player {
        ObjectGuid::create_global(high, 0, counter)
    } else if high == HighGuid::Transport {
        ObjectGuid::create_transport(high, counter)
    } else {
        ObjectGuid::create_world_object(high, 0, 1, 571, 7, 100, counter)
    }
}

fn world_object(high: HighGuid, map_id: u32, instance_id: u32, in_world: bool) -> WorldObject {
    let type_id = guid(high, 1).type_id();
    let type_mask = match type_id {
        wow_core::guid::TypeId::Player => TypeMask::PLAYER,
        wow_core::guid::TypeId::Unit => TypeMask::UNIT,
        wow_core::guid::TypeId::GameObject => TypeMask::GAME_OBJECT,
        wow_core::guid::TypeId::DynamicObject => TypeMask::DYNAMIC_OBJECT,
        wow_core::guid::TypeId::Corpse => TypeMask::CORPSE,
        wow_core::guid::TypeId::AreaTrigger => TypeMask::AREA_TRIGGER,
        wow_core::guid::TypeId::SceneObject => TypeMask::SCENE_OBJECT,
        wow_core::guid::TypeId::Conversation => TypeMask::CONVERSATION,
        _ => TypeMask::OBJECT,
    };
    let mut object = WorldObject::new(false, convert_type_id(type_id), type_mask);
    object.object_mut().create(guid(high, 1));
    object.set_map(map_id, instance_id).unwrap();
    object.relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        object.object_mut().add_to_world();
    }
    object
}

fn world_object_with_counter(
    high: HighGuid,
    counter: i64,
    map_id: u32,
    instance_id: u32,
    in_world: bool,
) -> WorldObject {
    let object_guid = guid(high, counter);
    let type_id = object_guid.type_id();
    let type_mask = match type_id {
        wow_core::guid::TypeId::Player => TypeMask::PLAYER,
        wow_core::guid::TypeId::Unit => TypeMask::UNIT,
        wow_core::guid::TypeId::GameObject => TypeMask::GAME_OBJECT,
        wow_core::guid::TypeId::DynamicObject => TypeMask::DYNAMIC_OBJECT,
        wow_core::guid::TypeId::Corpse => TypeMask::CORPSE,
        wow_core::guid::TypeId::AreaTrigger => TypeMask::AREA_TRIGGER,
        wow_core::guid::TypeId::SceneObject => TypeMask::SCENE_OBJECT,
        wow_core::guid::TypeId::Conversation => TypeMask::CONVERSATION,
        _ => TypeMask::OBJECT,
    };
    let mut object = WorldObject::new(false, convert_type_id(type_id), type_mask);
    object.object_mut().create(object_guid);
    object.set_map(map_id, instance_id).unwrap();
    object.relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        object.object_mut().add_to_world();
    }
    object
}

fn game_object_with_counter(
    counter: i64,
    map_id: u32,
    instance_id: u32,
    in_world: bool,
) -> GameObject {
    let mut game_object = GameObject::new();
    game_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::GameObject, counter));
    game_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        game_object.world_mut().object_mut().add_to_world();
    }
    game_object
}

fn convert_type_id(type_id: wow_core::guid::TypeId) -> TypeId {
    match type_id {
        wow_core::guid::TypeId::Object => TypeId::Object,
        wow_core::guid::TypeId::Item => TypeId::Item,
        wow_core::guid::TypeId::Container => TypeId::Container,
        wow_core::guid::TypeId::AzeriteEmpoweredItem => TypeId::AzeriteEmpoweredItem,
        wow_core::guid::TypeId::AzeriteItem => TypeId::AzeriteItem,
        wow_core::guid::TypeId::Unit => TypeId::Unit,
        wow_core::guid::TypeId::Player => TypeId::Player,
        wow_core::guid::TypeId::ActivePlayer => TypeId::ActivePlayer,
        wow_core::guid::TypeId::GameObject => TypeId::GameObject,
        wow_core::guid::TypeId::DynamicObject => TypeId::DynamicObject,
        wow_core::guid::TypeId::Corpse => TypeId::Corpse,
        wow_core::guid::TypeId::AreaTrigger => TypeId::AreaTrigger,
        wow_core::guid::TypeId::SceneObject => TypeId::SceneObject,
        wow_core::guid::TypeId::Conversation => TypeId::Conversation,
    }
}

#[test]
fn map_constructor_starts_with_empty_grid_slots_like_cpp_pointer_array() {
    let map = test_map();

    assert_eq!(map.map_id(), 571);
    assert_eq!(map.instance_id(), 7);
    assert_eq!(map.spawn_mode(), 1);
    assert_eq!(map.grid_expiry_ms(), 1000);
    assert!(map.grid_unload());
    assert_eq!(map.visibility_range(), 100.0);
    assert_eq!(map.grids.len(), GRID_SLOT_COUNT);
    assert!(map.grids.iter().all(Option::is_none));
}

#[test]
fn map_object_store_inserts_finds_typed_objects_and_removes_by_guid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let gameobject = world_object(HighGuid::GameObject, 571, 7, true);
    let creature_guid = creature.guid();
    let gameobject_guid = gameobject.guid();

    assert!(
        map.insert_map_object(AccessorObjectKind::Creature, creature)
            .unwrap()
            .is_none()
    );
    assert!(
        map.insert_map_object(AccessorObjectKind::GameObject, gameobject)
            .unwrap()
            .is_none()
    );

    assert_eq!(map.map_object_count(), 2);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().guid(),
        creature_guid
    );
    assert_eq!(
        map.get_game_object(gameobject_guid).unwrap().guid(),
        gameobject_guid
    );
    assert!(map.get_game_object(creature_guid).is_none());

    assert_eq!(
        map.remove_map_object(creature_guid)
            .unwrap()
            .object()
            .guid(),
        creature_guid
    );
    assert!(map.get_creature(creature_guid).is_none());
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn map_object_store_can_hold_typed_gameobject_entity_like_cpp() {
    let mut map = test_map();
    let mut gameobject = GameObject::new();
    let guid = guid(HighGuid::GameObject, 77);
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(123);
    gameobject.world_mut().set_map(571, 7).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    gameobject.set_created_by(ObjectGuid::create_player(1, 42));

    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    assert_eq!(map.get_game_object(guid).unwrap().guid(), guid);
    assert_eq!(
        map.get_typed_game_object(guid).unwrap().owner_guid(),
        ObjectGuid::create_player(1, 42)
    );
}

#[test]
fn map_object_store_can_hold_typed_creature_entity_like_cpp() {
    let mut map = test_map();
    let mut creature = Creature::new(false);
    let guid = guid(HighGuid::Creature, 78);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(321);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_level(42);

    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(map.get_creature(guid).unwrap().guid(), guid);
    assert_eq!(
        map.get_typed_creature(guid).unwrap().unit().data().level,
        42
    );
    map.get_typed_creature_mut(guid)
        .unwrap()
        .unit_mut()
        .set_level(43);
    assert_eq!(
        map.get_typed_creature(guid).unwrap().unit().data().level,
        43
    );
}

#[test]
fn map_object_store_can_hold_typed_player_entity_like_cpp() {
    let mut map = test_map();
    let mut player = Player::new(Some(7), false);
    let player_guid = guid(HighGuid::Player, 42);
    let victim_guid = guid(HighGuid::Creature, 77);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_attacking(Some(victim_guid));

    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    assert_eq!(map.map_object(player_guid).unwrap().guid(), player_guid);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .attacking(),
        Some(victim_guid)
    );
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .set_attacking(None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .attacking(),
        None
    );
}

#[test]
fn typed_player_counts_exclude_game_masters_like_cpp() {
    let mut map = test_map();
    let normal_guid = guid(HighGuid::Player, 42);
    let gm_guid = guid(HighGuid::Player, 43);

    let mut normal = Player::new(Some(7), false);
    normal
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(normal_guid);
    normal.unit_mut().world_mut().set_map(571, 7).unwrap();
    normal.unit_mut().world_mut().object_mut().add_to_world();
    map.insert_map_object_record(MapObjectRecord::new_player(normal).unwrap())
        .unwrap();

    let mut gm = Player::new(Some(8), false);
    gm.unit_mut().world_mut().object_mut().create(gm_guid);
    gm.unit_mut().world_mut().set_map(571, 7).unwrap();
    gm.unit_mut().world_mut().object_mut().add_to_world();
    gm.set_game_master_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_player(gm).unwrap())
        .unwrap();

    assert_eq!(map.typed_player_counts_like_cpp(), (2, 1));
}

#[test]
fn map_revalidates_all_typed_combat_refs_like_cpp_multi_owner_sweep() {
    let mut map = test_map();
    let alive_player_guid = guid(HighGuid::Player, 501);
    let dead_player_guid = guid(HighGuid::Player, 502);
    let creature_guid = guid(HighGuid::Creature, 503);

    let mut alive_player = Player::new(Some(7), false);
    alive_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(alive_player_guid);
    alive_player.unit_mut().world_mut().set_map(571, 7).unwrap();
    alive_player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    alive_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();

    let mut dead_player = Player::new(Some(7), false);
    dead_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(dead_player_guid);
    dead_player.unit_mut().world_mut().set_map(571, 7).unwrap();
    dead_player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(11.0, 20.0, 30.0));
    dead_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    dead_player.unit_mut().set_death_state(DeathState::Dead);

    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(12.0, 20.0, 30.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();

    map.insert_map_object_record(MapObjectRecord::new_player(alive_player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_player(dead_player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    map.get_typed_player_mut(alive_player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(creature_guid, false, false);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(alive_player_guid, false, false);
    map.get_typed_player_mut(dead_player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(creature_guid, false, false);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(dead_player_guid, false, false);

    let invalid = map.revalidate_all_combat_refs_like_cpp();

    assert!(invalid.contains(&(dead_player_guid, creature_guid)));
    assert!(invalid.contains(&(creature_guid, dead_player_guid)));
    assert!(
        map.get_typed_player(alive_player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(creature_guid)
    );
    assert!(
        map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(alive_player_guid)
    );
    assert!(
        !map.get_typed_player(dead_player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(creature_guid)
    );
    assert!(
        !map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(dead_player_guid)
    );
}

#[test]
fn map_ticks_pvp_combat_refs_and_purges_reciprocal_like_cpp() {
    let mut map = test_map();
    let first_guid = guid(HighGuid::Creature, 504);
    let second_guid = guid(HighGuid::Creature, 505);

    for guid in [first_guid, second_guid] {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().set_map(571, 7).unwrap();
        creature.unit_mut().world_mut().object_mut().add_to_world();
        map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
            .unwrap();
    }

    map.get_typed_creature_mut(first_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(second_guid, true, false);
    map.get_typed_creature_mut(second_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(first_guid, true, false);

    assert!(
        map.update_all_pvp_combat_refs_like_cpp(wow_entities::PVP_COMBAT_TIMEOUT_MS - 1)
            .is_empty()
    );
    let expired = map.update_all_pvp_combat_refs_like_cpp(1);
    assert!(!expired.is_empty());
    assert!(
        !map.get_typed_creature(first_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(second_guid)
    );
    assert!(
        !map.get_typed_creature(second_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(first_guid)
    );
}

#[test]
fn map_object_store_rejects_records_from_other_map_or_instance() {
    let mut map = test_map();
    let other_map_creature = world_object(HighGuid::Creature, 530, 7, true);
    let other_instance_creature = world_object(HighGuid::Creature, 571, 8, true);

    assert!(matches!(
        map.insert_map_object(AccessorObjectKind::Creature, other_map_creature),
        Err(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            expected_instance_id: 7,
            actual_map_id: 530,
            actual_instance_id: 7,
            ..
        })
    ));
    assert!(matches!(
        map.insert_map_object(AccessorObjectKind::Creature, other_instance_creature),
        Err(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            expected_instance_id: 7,
            actual_map_id: 571,
            actual_instance_id: 8,
            ..
        })
    ));
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn object_accessor_can_consult_map_owned_object_store() {
    let accessor = ObjectAccessor::default();
    let mut map = test_map();
    let context = world_object(HighGuid::Player, 571, 7, true);
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let creature_guid = creature.guid();

    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();

    assert_eq!(
        accessor
            .get_world_object_from_map_source(&context, &map, creature_guid)
            .unwrap()
            .guid(),
        creature_guid
    );
    assert!(matches!(
        accessor.get_object_ref_by_type_mask_from_map_source(
            &context,
            &map,
            creature_guid,
            TypeMask::UNIT
        ),
        Some(AccessorObjectRef::WorldObject(object)) if object.guid() == creature_guid
    ));
}

#[test]
fn add_to_map_like_cpp_creates_grid_marks_world_and_stores_grid_object() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.grid_created);
    assert!(!outcome.grid_loaded);
    assert!(outcome.inserted_into_cell);

    let stored = map.get_creature(guid).unwrap();
    assert!(stored.object().is_in_world());
    assert!(stored.object().is_in_grid());
    assert!(!stored.object().is_new_object());
    assert_eq!(
        stored.current_cell(),
        Some((
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS
        ))
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.creatures.contains(&guid));
    assert!(!cell.world_objects.creatures.contains(&guid));
}

#[test]
fn add_map_object_record_to_map_like_cpp_preserves_typed_creature_spawn_index() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(396, 39601, false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.inserted_into_cell);
    assert!(map.get_creature_by_spawn_id_like_cpp(396).is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .is_some()
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.creatures.contains(&guid));
    assert!(!cell.world_objects.creatures.contains(&guid));
}

fn creature_formation_info_like_cpp(leader_spawn_id: SpawnId) -> CreatureFormationInfoLikeCpp {
    CreatureFormationInfoLikeCpp {
        leader_spawn_id,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }
}

#[test]
fn creature_search_formation_add_to_map_inserts_group_holder_and_coexists_with_vehicle_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(470, 47001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900470)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9470);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let search = outcome.creature_search_formation.unwrap();
    assert_eq!(search.spawn_id, 470);
    assert_eq!(search.leader_spawn_id, Some(900470));
    assert!(search.add_to_group_requested);
    assert!(map.creature_group_holder_contains_like_cpp(900470, guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900470), 1);
    assert!(outcome.creature_vehicle_reset.is_some());
    assert!(outcome.creature_vehicle_install.is_some());
}

#[test]
fn creature_search_formation_add_to_map_removes_stale_same_spawn_member_like_cpp() {
    let mut map = test_map();
    let mut old_creature = test_creature_for_spawn(471, 47101, true);
    old_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    old_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900471)));
    let old_guid = old_creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(old_creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900471, old_guid));

    let mut new_creature = test_creature_for_spawn(471, 47102, true);
    new_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    new_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900471)));
    let new_guid = new_creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(new_creature).unwrap())
        .unwrap();

    assert!(
        outcome
            .creature_search_formation
            .as_ref()
            .is_some_and(|search| search.add_to_group_requested)
    );
    assert!(!map.creature_group_holder_contains_like_cpp(900471, old_guid));
    assert!(map.creature_group_holder_contains_like_cpp(900471, new_guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900471), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(471), 2);
}

#[test]
fn creature_search_formation_add_to_map_already_in_world_is_not_consumed_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(472, 47201, true);
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900472)));
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_search_formation.is_none());
    assert!(!map.creature_group_holder_contains_like_cpp(900472, guid));
}

#[test]
fn creature_search_formation_add_to_map_non_creature_path_is_unchanged_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(473, 47301);
    gameobject.world_mut().object_mut().remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_search_formation.is_none());
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900473), 0);
}

#[test]
fn creature_search_formation_remove_from_map_removes_last_member_and_holder_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(474, 47401, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900474)));
    let guid = creature.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900474, guid));

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert_eq!(formation.guid, guid);
    assert_eq!(formation.spawn_id, 474);
    assert_eq!(formation.leader_spawn_id, Some(900474));
    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(formation.removed_group);
    assert_eq!(formation.remaining_members, 0);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900474), 0);
}

#[test]
fn creature_search_formation_remove_from_map_keeps_group_with_other_member_like_cpp() {
    let mut map = test_map();
    let mut first = test_creature_for_spawn(475, 47501, true);
    first
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    first.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900475)));
    let first_guid = first.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(first).unwrap())
        .unwrap();

    let mut second = test_creature_for_spawn(476, 47601, true);
    second
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    second.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900475)));
    let second_guid = second.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(second).unwrap())
        .unwrap();
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900475), 2);

    let removed = map.remove_from_map_like_cpp(first_guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, 1);
    assert!(!map.creature_group_holder_contains_like_cpp(900475, first_guid));
    assert!(map.creature_group_holder_contains_like_cpp(900475, second_guid));
}

#[test]
fn creature_search_formation_remove_from_map_existing_holder_non_member_keeps_holder_like_cpp() {
    let mut map = test_map();
    let mut member = test_creature_for_spawn(483, 48301, true);
    member
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    member.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900483)));
    let member_guid = member.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(member).unwrap())
        .unwrap();
    let member_count_before = map.creature_group_holder_member_count_like_cpp(900483);
    assert_eq!(member_count_before, 1);
    assert!(map.creature_group_holder_contains_like_cpp(900483, member_guid));

    let mut non_member = test_creature_for_spawn(484, 48401, true);
    non_member.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900483)));
    let non_member_guid = non_member.guid();
    let add_outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(non_member).unwrap())
        .unwrap();
    assert!(add_outcome.already_in_world);
    assert!(!map.creature_group_holder_contains_like_cpp(900483, non_member_guid));

    let removed = map.remove_from_map_like_cpp(non_member_guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert!(formation.had_group);
    assert!(!formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, member_count_before);
    assert!(map.creature_group_holder_contains_like_cpp(900483, member_guid));
    assert!(!map.creature_group_holder_contains_like_cpp(900483, non_member_guid));
}

#[test]
fn creature_search_formation_remove_from_map_no_formation_or_not_in_world_noops_like_cpp() {
    let mut map = test_map();
    let mut holder_creature = test_creature_for_spawn(477, 47701, true);
    holder_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    holder_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900477)));
    let holder_guid = holder_creature.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(holder_creature).unwrap(),
    )
    .unwrap();

    let mut no_formation = test_creature_for_spawn(478, 47801, true);
    no_formation
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let no_formation_guid = no_formation.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(no_formation).unwrap())
        .unwrap();
    let removed = map
        .remove_from_map_like_cpp(no_formation_guid, true)
        .unwrap();
    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));

    let mut missing_holder = test_creature_for_spawn(479, 47901, true);
    missing_holder.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900479)));
    let missing_holder_guid = missing_holder.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(missing_holder).unwrap(),
    )
    .unwrap();
    let removed = map
        .remove_from_map_like_cpp(missing_holder_guid, true)
        .unwrap();
    let formation = removed.creature_remove_formation.unwrap();
    assert_eq!(formation.leader_spawn_id, Some(900479));
    assert!(!formation.had_group);
    assert!(!formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, 0);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900479), 0);
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));

    let mut not_in_world = test_creature_for_spawn(482, 48201, true);
    not_in_world.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900477)));
    let not_in_world_guid = not_in_world.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&not_in_world_guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let removed = map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));
}

#[test]
fn creature_search_formation_remove_from_map_non_creature_path_is_unchanged_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(480, 48001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900480)));
    let creature_guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let mut gameobject = test_gameobject_for_spawn(481, 48101);
    gameobject.world_mut().object_mut().remove_from_world();
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let removed = map.remove_from_map_like_cpp(gameobject_guid, true).unwrap();

    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900480, creature_guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900480), 1);
}

#[test]
fn creature_add_to_world_unit_seam_only_for_exact_typed_creature_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(475, 47501, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let caster = ObjectGuid::new(0, 47599);
    let enter_world_aura = AppliedAuraRef::new(47_510, caster, 1, 0x1);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .register_applied_aura(
            enter_world_aura,
            None,
            SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
            0,
        );
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        outcome.creature_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.creature_spawn_indexed_before_add_to_world,
        Some(true)
    );
    let unit_add = outcome.creature_unit_add_to_world.unwrap();
    assert_eq!(unit_add.guid, guid);
    assert!(unit_add.world_object_added);
    assert!(unit_add.is_in_world_after);
    assert_eq!(unit_add.removed_enter_world_auras, vec![enter_world_aura]);
    assert!(
        unit_add
            .motion_master_add_to_world
            .had_initialization_pending
    );
    assert!(
        unit_add
            .motion_master_add_to_world
            .direct_initialize_represented
    );
    assert!(outcome.creature_zone_script_create.is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .is_some_and(|creature| !creature
                .unit()
                .subsystems()
                .auras
                .has_applied(enter_world_aura))
    );

    let generic_creature = world_object_with_counter(HighGuid::Creature, 47502, 571, 7, false);
    let generic = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
        )
        .unwrap();
    assert!(
        generic
            .creature_store_inserted_before_add_to_world
            .is_none()
    );
    assert!(generic.creature_spawn_indexed_before_add_to_world.is_none());
    assert!(generic.creature_unit_add_to_world.is_none());
    assert!(generic.creature_search_formation.is_none());
    assert!(generic.creature_aim_initialize.is_none());
    assert!(generic.creature_zone_script_create.is_none());

    let gameobject = test_gameobject_for_spawn(47503, 47504);
    let non_creature = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    assert!(
        non_creature
            .creature_store_inserted_before_add_to_world
            .is_none()
    );
    assert!(
        non_creature
            .creature_spawn_indexed_before_add_to_world
            .is_none()
    );
    assert!(non_creature.creature_unit_add_to_world.is_none());
    assert!(non_creature.creature_aim_initialize.is_none());
}

#[test]
fn creature_aim_initialize_add_to_map_emits_for_normal_creature_without_vehicle_reset_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(476, 47601, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let aim = outcome.creature_aim_initialize.unwrap();
    assert_eq!(aim.guid, guid);
    assert_eq!(aim.spawn_id, 476);
    assert!(aim.aim_create_represented);
    assert!(aim.motion_initialize_represented);
    assert!(!aim.formation_present);
    assert!(!aim.formation_leader);
    assert!(!aim.formation_move_idle_represented);
    assert!(!aim.motion_initialize_requires_formed_state);
    assert!(aim.motion_master_initialize_represented);
    assert!(aim.ai_selected_represented);
    assert!(aim.ai_initialize_represented);
    assert!(!aim.vehicle_reset_expected);
    assert!(aim.succeeded);
    assert!(outcome.creature_vehicle_reset.is_none());
}

#[test]
fn creature_zone_script_add_to_map_create_evidence_only_on_normal_creature_path_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(490, 49001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let zone_script = outcome.creature_zone_script_create.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
    assert!(!outcome.already_in_world);

    let already_in_world = test_creature_for_spawn(491, 49101, true);
    let already = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(already_in_world).unwrap(),
        )
        .unwrap();

    assert!(already.already_in_world);
    assert!(already.creature_zone_script_create.is_none());

    let generic_creature = world_object_with_counter(HighGuid::Creature, 49002, 571, 7, false);
    let generic = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
        )
        .unwrap();

    assert!(!generic.already_in_world);
    assert!(generic.creature_zone_script_create.is_none());
}

#[test]
fn creature_zone_script_add_to_map_create_follows_vehicle_install_tail_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(492, 49201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9492, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let install = outcome.creature_vehicle_install.unwrap();
    assert_eq!(install.kit_id, Some(9492));
    let zone_script = outcome.creature_zone_script_create.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
}

#[test]
fn creature_zone_script_remove_from_map_remove_evidence_precedes_formation_then_unit_vehicle_like_cpp()
 {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(493, 49301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900493)));
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9493, true);
    let guid = creature.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900493, guid));

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    let zone_script = removed.creature_zone_script_remove.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
    let formation = removed.creature_remove_formation.unwrap();
    assert_eq!(formation.guid, guid);
    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(formation.removed_group);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900493), 0);
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.guid, guid);
    assert!(unit_remove.was_in_world);
    assert!(unit_remove.during_remove_entered);
    assert!(!unit_remove.ai_on_despawn_represented);
    assert!(!unit_remove.leave_world_cleanup_represented);
    assert!(unit_remove.world_object_removed);
    assert!(unit_remove.during_remove_cleared);
    let unit_vehicle_remove = unit_remove.vehicle_remove.unwrap();
    assert_eq!(unit_vehicle_remove.kit_id, Some(9493));
    assert_eq!(removed.creature_vehicle_remove, Some(unit_vehicle_remove));
}

#[test]
fn creature_zone_script_remove_from_map_missing_not_in_world_and_non_creature_noop_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 49401);
    assert!(matches!(
        map.remove_from_map_like_cpp(missing_guid, true),
        Err(RemoveFromMapError::ObjectNotFound { guid }) if guid == missing_guid
    ));

    let mut not_in_world = test_creature_for_spawn(494, 49402, true);
    not_in_world
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let not_in_world_guid = not_in_world.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&not_in_world_guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let removed = map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(!removed.was_in_world);
    assert!(removed.creature_zone_script_remove.is_none());
    assert!(removed.creature_remove_formation.is_none());
    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());

    let mut gameobject = test_gameobject_for_spawn(495, 49501);
    gameobject.world_mut().object_mut().remove_from_world();
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    let removed = map.remove_from_map_like_cpp(gameobject_guid, true).unwrap();
    assert!(removed.creature_zone_script_remove.is_none());
}

fn creature_add_to_world_vehicle_reset_context(
    is_mechanical_creature: bool,
    is_world_boss: bool,
) -> CreatureAddToWorldVehicleResetContextLikeCpp {
    CreatureAddToWorldVehicleResetContextLikeCpp {
        is_mechanical_creature,
        is_world_boss,
        accessories: vec![VehicleAccessory {
            accessory_entry: 7001,
            is_minion: false,
            summon_time_ms: 3_000,
            seat_id: 1,
            summoned_type: 6,
        }],
    }
}

fn create_loaded_creature_vehicle_kit_like_cpp(creature: &mut Creature, vehicle_id: u32) {
    let guid = creature.guid();
    let position = creature.unit().world().position();
    let entry = creature.unit().world().object().entry();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .create_vehicle_kit_like_cpp(
            guid,
            position,
            Some(vehicle_id),
            entry,
            true,
            Some(vec![(
                0,
                VehicleSeatInfo {
                    id: 100,
                    attachment_offset: Position::default(),
                    can_enter_or_exit: true,
                    usable_by_override: false,
                    can_control: false,
                    can_switch_from_seat: false,
                    ejectable: false,
                    disables_gravity: false,
                    passenger_not_selectable: false,
                    keep_pet: false,
                },
                VehicleSeatAddon::default(),
            )]),
        );
}

#[test]
fn creature_vehicle_add_to_map_resets_then_installs_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(400, 40001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9003);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let reset = outcome.creature_vehicle_reset.unwrap();
    assert_eq!(reset.kit_id, 9003);
    let aim = outcome.creature_aim_initialize.unwrap();
    assert_eq!(aim.guid, guid);
    assert_eq!(aim.spawn_id, 400);
    assert!(!aim.motion_initialize_requires_formed_state);
    assert!(aim.vehicle_reset_expected);
    assert!(aim.aim_create_represented);
    assert!(aim.ai_initialize_represented);
    assert!(reset.aim_create_represented);
    assert!(reset.ai_initialize_represented);
    assert!(!reset.reset_evading);
    assert!(reset.reset_plan.call_on_reset_script);
    assert!(
        reset
            .reset_plan
            .immunity_plan
            .immunities
            .contains(&VehicleSpellImmunity {
                kind: VehicleSpellImmunityKind::Effect,
                spell_or_mechanic: 98,
                apply: true,
            })
    );
    let accessory_plan = reset.reset_plan.accessory_install_plan.unwrap();
    assert_eq!(accessory_plan.accessories.len(), 1);
    assert_eq!(accessory_plan.accessories[0].accessory_entry, 7001);
    assert!(outcome.creature_vehicle_install.is_some());
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id(), 9003);
    assert!(kit.installed());
}

#[test]
fn creature_vehicle_add_to_map_mechanical_world_boss_skips_mechanical_immunities_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(401, 40101, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(true, true),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9004);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let immunities = &outcome
        .creature_vehicle_reset
        .unwrap()
        .reset_plan
        .immunity_plan
        .immunities;
    assert!(immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::Effect,
        spell_or_mechanic: 98,
        apply: true,
    }));
    assert!(!immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::Effect,
        spell_or_mechanic: 6,
        apply: true,
    }));
}

#[test]
fn creature_vehicle_add_to_map_without_kit_has_no_reset_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(402, 40201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_vehicle_reset.is_none());
}

#[test]
fn creature_vehicle_add_to_map_already_in_world_has_no_reset_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(403, 40301, true);
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9005);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_vehicle_reset.is_none());
}

#[test]
fn creature_vehicle_add_to_map_installs_local_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(397, 39701, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9001, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    let install = outcome.creature_vehicle_install.unwrap();
    assert_eq!(install.kit_id, Some(9001));
    assert!(install.had_kit);
    assert_eq!(install.previous_installed, Some(false));
    assert!(install.installed);
    assert!(install.script_on_install_represented);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id, 9001);
    assert!(kit.active);
    assert!(kit.installed);
}

#[test]
fn creature_vehicle_add_to_map_without_kit_has_no_install_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(398, 39801, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_vehicle_install.is_none());
}

#[test]
fn creature_vehicle_add_to_map_already_in_world_does_not_install_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(399, 39901, true);
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9002, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_vehicle_install.is_none());
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id, 9002);
    assert!(!kit.installed);
}

#[test]
fn creature_add_to_world_add_to_map_tail_initializes_clears_move_and_visibility_flags_like_cpp() {
    let mut map = test_map();
    let mut stale_creature = test_creature_for_spawn(478, 47801, true);
    stale_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = stale_creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(stale_creature).unwrap())
        .unwrap();
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, Position::xyz(40.0, 41.0, 42.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
            .is_some()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );

    let mut creature = test_creature_for_spawn(478, 47801, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900478)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9478);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.creature_unit_add_to_world.is_some());
    assert!(outcome.creature_search_formation.is_some());
    assert!(outcome.creature_aim_initialize.is_some());
    assert!(outcome.creature_vehicle_reset.is_some());
    assert!(outcome.creature_vehicle_install.is_some());
    assert!(outcome.creature_zone_script_create.is_some());
    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.initialize_object_represented);
    assert!(tail.pending_move_state_cleared);
    assert!(!tail.no_pending_move_state);
    assert!(!tail.add_to_active_represented);
    assert!(!tail.add_to_active_skipped_runtime_gap);
    assert!(tail.set_is_new_object_true);
    assert!(tail.update_object_visibility_on_create_represented);
    assert!(tail.update_object_visibility_on_create_runtime_gap);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
            .is_none()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    let drain = map.move_all_creatures_in_move_list_like_cpp();
    assert_eq!(drain.processed, 0);
    assert_eq!(drain.relocated, 0);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    assert!(!stored.unit().world().object().is_new_object());
}

#[test]
fn add_to_map_typed_gameobject_tail_initializes_and_clears_move_like_cpp() {
    let mut map = test_map();
    let mut stale_gameobject = test_gameobject_for_spawn(478, 47802);
    stale_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = stale_gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(stale_gameobject).unwrap())
        .unwrap();
    assert_eq!(
        map.add_game_object_to_move_list_like_cpp(guid, Position::xyz(50.0, 51.0, 52.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let mut gameobject = test_gameobject_for_spawn(478, 47802);
    gameobject.world_mut().object_mut().remove_from_world();
    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let tail = outcome.add_to_map_tail.unwrap();
    assert!(tail.initialize_object_represented);
    assert!(tail.pending_move_state_cleared);
    assert!(tail.set_is_new_object_true);
    assert!(tail.update_object_visibility_on_create_represented);
    assert!(tail.update_object_visibility_on_create_runtime_gap);
    assert!(tail.set_is_new_object_false);
    assert!(!tail.final_is_new_object);
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid)
            .is_none()
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    let drain = map.move_all_game_objects_in_move_list_like_cpp();
    assert_eq!(drain.processed, 0);
    assert_eq!(drain.relocated, 0);
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert!(stored.world().object().is_in_world());
    assert!(!stored.world().object().is_new_object());
}

#[test]
fn add_to_map_generic_creature_and_already_in_world_do_not_overclaim_tail_like_cpp() {
    let mut map = test_map();
    let generic = world_object_with_counter(HighGuid::Creature, 47803, 571, 7, false);
    let generic_guid = generic.guid();
    map.insert_map_object(AccessorObjectKind::Creature, generic)
        .unwrap();
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(generic_guid, Position::xyz(60.0, 61.0, 62.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let generic_again = world_object_with_counter(HighGuid::Creature, 47803, 571, 7, false);

    let generic_outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, generic_again)
        .unwrap();

    assert!(generic_outcome.creature_unit_add_to_world.is_none());
    assert!(generic_outcome.creature_search_formation.is_none());
    assert!(generic_outcome.creature_aim_initialize.is_none());
    assert!(generic_outcome.creature_vehicle_reset.is_none());
    assert!(generic_outcome.creature_vehicle_install.is_none());
    assert!(generic_outcome.creature_zone_script_create.is_none());
    assert!(generic_outcome.add_to_map_tail.is_none());
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, generic_guid)
            .is_some()
    );

    let mut creature = test_creature_for_spawn(479, 47901, true);
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900479)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9479);
    let already_guid = creature.guid();
    let already = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(already.already_in_world);
    assert!(already.creature_unit_add_to_world.is_none());
    assert!(already.creature_search_formation.is_none());
    assert!(already.creature_aim_initialize.is_none());
    assert!(already.creature_vehicle_reset.is_none());
    assert!(already.creature_vehicle_install.is_none());
    assert!(already.creature_zone_script_create.is_none());
    assert!(already.add_to_map_tail.is_none());
    let stored = map
        .map_object_record(already_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert!(stored.unit().world().object().is_in_world());
    assert!(!stored.unit().world().object().is_new_object());
}

#[test]
fn add_map_object_record_to_map_like_cpp_preserves_typed_gameobject_spawn_index() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(396, 39602);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.inserted_into_cell);
    assert!(map.get_gameobject_by_spawn_id_like_cpp(396).is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .is_some()
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.gameobjects.contains(&guid));
}

#[test]
fn add_to_map_like_cpp_active_world_object_loads_grid_and_world_container() {
    let mut map = test_map();
    let mut object = WorldObject::new(true, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
    object.object_mut().create(guid(HighGuid::DynamicObject, 2));
    object.set_map(571, 7).unwrap();
    object.relocate(Position::xyz(20.0, 20.0, 3.0));
    object.set_active(true);
    let guid = object.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::DynamicObject, object)
        .unwrap();

    assert!(outcome.grid_loaded);
    assert!(!outcome.grid_created);
    assert!(map.is_grid_loaded(outcome.grid));
    assert_eq!(map.lifecycle().loads, 1);
    let grid = map.get_ngrid(outcome.grid).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.world_objects.dynamic_objects.contains(&guid));
    assert!(!cell.grid_objects.dynamic_objects.contains(&guid));
}

#[test]
fn add_to_map_like_cpp_player_is_active_even_without_runtime_active_flag() {
    let mut map = test_map();
    let player = world_object(HighGuid::Player, 571, 7, false);
    let guid = player.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.grid_loaded);
    assert!(!outcome.grid_created);
    assert!(map.is_grid_loaded(outcome.grid));
    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.world_objects.players.contains(&guid));
}

#[test]
fn add_to_map_like_cpp_rejects_invalid_coordinates_before_grid_mutation() {
    let mut map = test_map();
    let mut creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    creature.relocate(Position::xyz(f32::NAN, 0.0, 0.0));

    assert!(matches!(
        map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature),
        Err(AddToMapError::InvalidCoordinates { guid: actual, .. }) if actual == guid
    ));
    assert_eq!(map.map_object_count(), 0);
    assert!(map.terrain().loads.is_empty());
}

#[test]
fn add_to_map_like_cpp_rejects_wrong_map_before_grid_mutation() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 530, 7, false);

    assert!(matches!(
        map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature),
        Err(AddToMapError::Store(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            actual_map_id: 530,
            ..
        }))
    ));
    assert_eq!(map.map_object_count(), 0);
    assert!(map.terrain().loads.is_empty());
}

#[test]
fn creature_vehicle_remove_from_map_unit_remove_from_world_uninstalls_local_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46701, 4670101, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9101, true);
    let guid = creature.guid();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(added.creature_vehicle_install.is_some());

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.was_in_world);
    let remove = removed.creature_vehicle_remove.unwrap();
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.guid, guid);
    assert!(unit_remove.was_in_world);
    assert_eq!(unit_remove.vehicle_remove, Some(remove));
    assert!(unit_remove.world_object_removed);
    assert_eq!(remove.kit_id, Some(9101));
    assert!(remove.had_kit);
    assert_eq!(remove.previous_installed, Some(true));
    assert!(remove.on_remove_from_world);
    assert!(!remove.send_set_vehicle_rec_id_zero_represented);
    assert!(remove.uninstall_represented);
    assert!(remove.remove_all_passengers_represented);
    assert!(remove.script_on_uninstall_represented);
    assert!(remove.kit_cleared);
    assert!(removed.object.is_some());
}

#[test]
fn creature_vehicle_remove_from_map_delete_keeps_uninstall_evidence_without_object_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46702, 4670201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9102, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    let remove = removed.creature_vehicle_remove.unwrap();
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.vehicle_remove, Some(remove));
    assert!(unit_remove.world_object_removed);
    assert_eq!(remove.kit_id, Some(9102));
    assert!(remove.kit_cleared);
    assert!(removed.object.is_none());
}

#[test]
fn creature_vehicle_remove_from_map_not_in_world_does_not_consume_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(46703, 4670301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9103, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let stored = map
        .entity_world
        .get_mut(&guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap();
    stored
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());
    assert!(removed.object.is_some());
}

#[test]
fn creature_vehicle_remove_from_map_non_creature_has_no_evidence_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(46704, 4670401);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());
}

#[test]
fn remove_from_map_like_cpp_removes_store_cell_and_resets_object_binding() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    assert!(map.get_creature(guid).is_some());

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert_eq!(removed.guid, guid);
    assert_eq!(removed.cell, added.cell);
    assert!(removed.was_in_world);
    assert!(removed.cxx_in_world);
    assert!(!removed.was_active);
    assert!(removed.removed_from_cell);
    assert!(!removed.delete_from_world);
    assert!(map.get_creature(guid).is_none());

    let grid = map.get_ngrid(removed.grid).unwrap();
    let cell = grid
        .get_grid_type(
            removed.cell.x_coord % MAX_NUMBER_OF_CELLS,
            removed.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(!cell.grid_objects.creatures.contains(&guid));

    let object = removed.object.unwrap();
    assert!(!object.object().is_in_world());
    assert!(!object.object().is_in_grid());
    assert!(!object.has_current_map());
    assert_eq!(object.current_cell(), None);
}

#[test]
fn remove_from_map_delete_detaches_creature_loot_authority_before_typed_drop() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_101);
    let mut creature = test_creature_for_spawn(484_101, 4_841_010, true);
    let guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 17, player,
            ))
            .installed()
    );
    let retained_authority = creature.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live Creature authority must grant the uncontended lease");
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    assert!(removed.object.is_none());
    assert_eq!(
        retained_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

#[test]
fn remove_from_map_nondelete_detaches_orphaned_typed_loot_authority() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_109);
    let mut creature = test_creature_for_spawn(484_109, 4_841_090, true);
    let guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 17, player,
            ))
            .installed()
    );
    let retained_authority = creature.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live typed Creature owns the reservation");
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(
        removed.object.is_some(),
        "the erased WorldObject remains available to the non-delete caller"
    );
    assert_eq!(
        retained_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached,
        "the returned erased object cannot preserve the destroyed typed loot owner"
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

#[test]
fn remove_from_map_delete_detaches_gameobject_loot_authority_before_typed_drop() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_102);
    let mut gameobject = test_gameobject_for_spawn(484_102, 4_841_020);
    let guid = gameobject.world().guid();
    assert!(
        gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 19, player,
            ))
            .installed()
    );
    let retained_authority = gameobject.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live GameObject authority must grant the uncontended lease");
    gameobject.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    assert!(removed.object.is_none());
    assert_eq!(
        retained_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

#[test]
fn insert_map_object_record_detaches_displaced_creature_authority_for_same_guid() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_103);
    let mut displaced_creature = test_creature_for_spawn(484_103, 4_841_030, true);
    let guid = displaced_creature.guid();
    assert!(
        displaced_creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 23, player,
            ))
            .installed()
    );
    let displaced_authority = displaced_creature.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_creature(displaced_creature).unwrap())
        .unwrap();
    let lease = poll_immediately_ready(displaced_authority.reserve_money_like_cpp(player))
        .expect("the displaced Creature authority must grant the uncontended lease");

    let mut replacement = test_creature_for_spawn(484_104, 4_841_030, true);
    assert!(
        replacement
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 29, player,
            ))
            .installed()
    );
    let replacement_authority = replacement.loot_authority_like_cpp().clone();
    let displaced = map
        .insert_map_object_record(MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap()
        .expect("same-GUID insert must return the displaced Creature record");

    assert_eq!(
        displaced_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(
        displaced
            .creature()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&displaced_authority)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement_authority)
    );
    assert_eq!(
        replacement_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

#[test]
fn insert_map_object_record_detaches_displaced_gameobject_authority_for_same_guid() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_104);
    let mut displaced_gameobject = test_gameobject_for_spawn(484_105, 4_841_040);
    let guid = displaced_gameobject.world().guid();
    assert!(
        displaced_gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 31, player,
            ))
            .installed()
    );
    let displaced_authority = displaced_gameobject.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_game_object(displaced_gameobject).unwrap())
        .unwrap();
    let lease = poll_immediately_ready(displaced_authority.reserve_money_like_cpp(player))
        .expect("the displaced GameObject authority must grant the uncontended lease");

    let mut replacement = test_gameobject_for_spawn(484_106, 4_841_040);
    assert!(
        replacement
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 37, player,
            ))
            .installed()
    );
    let replacement_authority = replacement.loot_authority_like_cpp().clone();
    let displaced = map
        .insert_map_object_record(MapObjectRecord::new_game_object(replacement).unwrap())
        .unwrap()
        .expect("same-GUID insert must return the displaced GameObject record");

    assert_eq!(
        displaced_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(
        displaced
            .game_object()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&displaced_authority)
    );
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::game_object)
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement_authority)
    );
    assert_eq!(
        replacement_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

#[test]
fn insert_map_object_record_preserves_shared_typed_authority_for_same_guid_refresh() {
    let player = ObjectGuid::create_player(1, 484_105);

    let mut creature_map = test_map();
    let mut creature = test_creature_for_spawn(484_107, 4_841_050, true);
    let creature_guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                creature_guid,
                41,
                player,
            ))
            .installed()
    );
    let creature_record = MapObjectRecord::new_creature(creature).unwrap();
    let creature_refresh = creature_record.clone();
    let creature_authority = creature_record
        .creature()
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    creature_map
        .insert_map_object_record(creature_record)
        .unwrap();
    let creature_lease = poll_immediately_ready(creature_authority.reserve_money_like_cpp(player))
        .expect("the shared Creature authority must grant the uncontended lease");

    let displaced_creature = creature_map
        .insert_map_object_record(creature_refresh)
        .unwrap()
        .expect("same-GUID refresh must return the prior Creature record");

    assert!(
        displaced_creature
            .creature()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&creature_authority)
    );
    assert_eq!(
        creature_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(creature_lease.commit_like_cpp(), Ok(true));
    assert_eq!(
        creature_map
            .map_object_record(creature_guid)
            .and_then(MapObjectRecord::creature)
            .unwrap()
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        0
    );

    let mut gameobject_map = test_map();
    let mut gameobject = test_gameobject_for_spawn(484_108, 4_841_060);
    let gameobject_guid = gameobject.world().guid();
    assert!(
        gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                gameobject_guid,
                43,
                player,
            ))
            .installed()
    );
    let gameobject_record = MapObjectRecord::new_game_object(gameobject).unwrap();
    let gameobject_refresh = gameobject_record.clone();
    let gameobject_authority = gameobject_record
        .game_object()
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    gameobject_map
        .insert_map_object_record(gameobject_record)
        .unwrap();
    let gameobject_lease =
        poll_immediately_ready(gameobject_authority.reserve_money_like_cpp(player))
            .expect("the shared GameObject authority must grant the uncontended lease");

    let displaced_gameobject = gameobject_map
        .insert_map_object_record(gameobject_refresh)
        .unwrap()
        .expect("same-GUID refresh must return the prior GameObject record");

    assert!(
        displaced_gameobject
            .game_object()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&gameobject_authority)
    );
    assert_eq!(
        gameobject_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(gameobject_lease.commit_like_cpp(), Ok(true));
    assert_eq!(
        gameobject_map
            .map_object_record(gameobject_guid)
            .and_then(MapObjectRecord::game_object)
            .unwrap()
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[test]
fn remove_from_map_like_cpp_unregisters_personal_phase_tracker_from_object_owner_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 48401);
    let mut gameobject = test_gameobject_for_spawn(48401, 4840101);
    gameobject
        .world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(owner);
    let guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.register_personal_phase_object_for_test(84, owner, guid);
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);

    let outcome = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert_eq!(outcome.personal_phase_unregister.phase_owner, owner);
    assert!(outcome.personal_phase_unregister.attempted);
    assert!(outcome.personal_phase_unregister.tracker_found);
    assert!(outcome.personal_phase_unregister.removed);
    assert!(outcome.personal_phase_unregister.removed_owner_tracker);
    assert_eq!(map.personal_phase_tracker().tracker_count(), 0);
}

#[test]
fn remove_from_map_like_cpp_personal_phase_empty_or_missing_owner_noops_like_cpp() {
    let mut empty_owner_map = test_map();
    let empty_owner_gameobject = test_gameobject_for_spawn(48402, 4840201);
    let empty_owner_guid = empty_owner_gameobject.world().guid();
    empty_owner_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(empty_owner_gameobject).unwrap(),
        )
        .unwrap();

    let empty_owner = empty_owner_map
        .remove_from_map_like_cpp(empty_owner_guid, false)
        .unwrap();
    assert_eq!(
        empty_owner.personal_phase_unregister.phase_owner,
        ObjectGuid::EMPTY
    );
    assert!(!empty_owner.personal_phase_unregister.attempted);
    assert!(!empty_owner.personal_phase_unregister.tracker_found);
    assert!(!empty_owner.personal_phase_unregister.removed);

    let mut missing_tracker_map = test_map();
    let missing_owner = ObjectGuid::create_player(1, 48403);
    let mut missing_tracker_gameobject = test_gameobject_for_spawn(48403, 4840301);
    missing_tracker_gameobject
        .world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(missing_owner);
    let missing_tracker_guid = missing_tracker_gameobject.world().guid();
    missing_tracker_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(missing_tracker_gameobject).unwrap(),
        )
        .unwrap();

    let missing_tracker = missing_tracker_map
        .remove_from_map_like_cpp(missing_tracker_guid, false)
        .unwrap();
    assert_eq!(
        missing_tracker.personal_phase_unregister.phase_owner,
        missing_owner
    );
    assert!(missing_tracker.personal_phase_unregister.attempted);
    assert!(!missing_tracker.personal_phase_unregister.tracker_found);
    assert!(!missing_tracker.personal_phase_unregister.removed);
}

#[test]
fn remove_from_map_like_cpp_visibility_on_destroy_follows_cpp_in_world_type_range() {
    let mut dynamic_map = test_map();
    let dynamic = test_dynamic_object_for_viewpoint(4840401);
    let dynamic_guid = dynamic.world().guid();
    dynamic_map
        .insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic).unwrap())
        .unwrap();
    let dynamic_removed = dynamic_map
        .remove_from_map_like_cpp(dynamic_guid, false)
        .unwrap();
    assert!(dynamic_removed.was_in_world);
    assert!(!dynamic_removed.cxx_in_world);
    assert!(
        dynamic_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut area_map = test_map();
    let area_trigger = test_area_trigger_for_spawn(48405, 4840501);
    let area_guid = area_trigger.world().guid();
    area_map
        .insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();
    let area_removed = area_map.remove_from_map_like_cpp(area_guid, false).unwrap();
    assert!(area_removed.was_in_world);
    assert!(!area_removed.cxx_in_world);
    assert!(
        area_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}

#[test]
fn remove_from_map_like_cpp_visibility_on_destroy_skips_in_world_eligible_records() {
    let mut gameobject_map = test_map();
    let gameobject = test_gameobject_for_spawn(48406, 4840601);
    let gameobject_guid = gameobject.world().guid();
    gameobject_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let gameobject_removed = gameobject_map
        .remove_from_map_like_cpp(gameobject_guid, false)
        .unwrap();
    assert!(gameobject_removed.cxx_in_world);
    assert!(
        !gameobject_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut creature_map = test_map();
    let creature = test_creature_for_spawn(48407, 4840701, true);
    let creature_guid = creature.guid();
    creature_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let creature_removed = creature_map
        .remove_from_map_like_cpp(creature_guid, false)
        .unwrap();
    assert!(creature_removed.cxx_in_world);
    assert!(
        !creature_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut player_map = test_map();
    let player = test_player_for_viewpoint(4840801);
    let player_guid = player.guid();
    player_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let player_removed = player_map
        .remove_from_map_like_cpp(player_guid, false)
        .unwrap();
    assert!(player_removed.cxx_in_world);
    assert!(
        !player_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut pet_map = test_map();
    let pet = test_pet(4840901, true);
    let pet_guid = pet.creature().guid();
    pet_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    let pet_removed = pet_map.remove_from_map_like_cpp(pet_guid, false).unwrap();
    assert!(pet_removed.cxx_in_world);
    assert!(
        !pet_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut transport_map = test_map();
    let transport = test_transport(4841001, true);
    let transport_guid = transport.world().guid();
    transport_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();
    let transport_removed = transport_map
        .remove_from_map_like_cpp(transport_guid, false)
        .unwrap();
    assert!(transport_removed.cxx_in_world);
    assert!(
        !transport_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}

#[test]
fn remove_from_map_like_cpp_visibility_on_destroy_runs_for_not_in_world_gameobject_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48411, 4841101);
    gameobject.world_mut().object_mut().remove_from_world();
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(!removed.was_in_world);
    assert!(!removed.cxx_in_world);
    assert!(
        removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}

#[test]
fn remove_list_enqueue_creature_marks_destroyed_cleans_and_keeps_record_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41901;
    let mut creature = test_creature_for_spawn(spawn_id, 4190101, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.add_object_to_remove_list_like_cpp(guid);

    assert_eq!(outcome.guid, guid);
    assert!(outcome.queued);
    assert!(!outcome.duplicate);
    assert_eq!(outcome.cleanup_before_delete_count, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    assert!(
        map.exact_cell_guids_like_cpp(added.cell)
            .grid
            .creatures
            .contains(&guid)
    );
    let creature = map.get_typed_creature(guid).unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
}

#[test]
fn personal_phase_tracker_update_enqueues_expired_canonical_object_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 44001);
    let phase_id = 44;
    let creature = test_creature_for_spawn(44001, 4400101, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.register_personal_phase_object_for_test(phase_id, owner, guid);
    map.mark_personal_phases_for_deletion_for_test(owner);

    let early = map.update_personal_phase_tracker_like_cpp(59_999);

    assert_eq!(early, PersonalPhaseTrackerUpdateSummaryLikeCpp::default());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_some());

    let expired = map.update_personal_phase_tracker_like_cpp(1);

    assert_eq!(
        expired,
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 1,
            missing_or_stale: 0,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(guid).is_some());
    let creature = map.get_typed_creature(guid).unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
}

#[test]
fn personal_phase_tracker_update_counts_missing_expired_guid_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 44002);
    let missing_guid = guid(HighGuid::Creature, 4400201);
    map.register_personal_phase_object_for_test(44, owner, missing_guid);
    map.mark_personal_phases_for_deletion_for_test(owner);

    let summary = map.update_personal_phase_tracker_like_cpp(60_000);

    assert_eq!(
        summary,
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 0,
            missing_or_stale: 1,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn remove_list_drain_physically_removes_creature_and_second_cleanup_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41902;
    let mut creature = test_creature_for_spawn(spawn_id, 4190201, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.add_object_to_remove_list_like_cpp(guid).queued);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.creature_second_cleanup_count, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
    assert!(
        !map.exact_cell_guids_like_cpp(added.cell)
            .grid
            .creatures
            .contains(&guid)
    );
}

#[test]
fn remove_list_duplicate_enqueue_follows_cpp_cleanup_before_set_insert_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(41903, 4190301, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let first = map.add_object_to_remove_list_like_cpp(guid);
    let second = map.add_object_to_remove_list_like_cpp(guid);

    assert!(first.queued);
    assert!(second.duplicate);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(
        map.get_typed_creature(guid)
            .unwrap()
            .cleanup_before_delete_count(),
        2
    );
}

#[test]
fn remove_list_drain_missing_stale_guid_does_not_create_object_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4190401);
    map.enqueue_object_to_remove_for_test(guid);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 1);
    assert_eq!(outcome.missing_or_stale, 1);
    assert_eq!(outcome.removed, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}

fn add_loaded_grid_creature_for_switch(
    map: &mut Map<RecordingTerrain, RecordingLifecycle>,
    spawn_id: SpawnId,
    counter: i64,
) -> (ObjectGuid, CellCoord, GridCoord) {
    let cell = Cell::from_world(1.0, 2.0);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    map.ensure_grid_loaded(&cell);
    let mut creature = test_creature_for_spawn(spawn_id, counter, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(outcome.inserted_into_cell);
    (guid, outcome.cell, grid)
}

fn local_cell_for_switch<'a>(
    map: &'a Map<RecordingTerrain, RecordingLifecycle>,
    grid: GridCoord,
    cell: CellCoord,
) -> &'a Cell {
    map.get_ngrid(grid)
        .unwrap()
        .get_grid_type(
            cell.x_coord % MAX_NUMBER_OF_CELLS,
            cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap()
}

fn test_player_for_viewpoint(counter: i64) -> Player {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Player, counter));
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player
}

fn test_dynamic_object_for_viewpoint(counter: i64) -> DynamicObject {
    let mut dynamic_object = DynamicObject::new(true);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::DynamicObject, counter));
    dynamic_object.world_mut().set_map(571, 7).unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(11.0, 21.0, 31.0));
    dynamic_object.world_mut().object_mut().add_to_world();
    dynamic_object
}

#[test]
fn send_object_updates_processes_dynamic_object_data_update_like_cpp() {
    use wow_entities::{DYNAMIC_OBJECT_DATA_PARENT_BIT, DYNAMIC_OBJECT_DATA_RADIUS_BIT};

    let mut map = test_map();
    let dynamic_object = test_dynamic_object_for_viewpoint(501001);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let record = map.entity_world.get_mut(&dynamic_object_guid).unwrap();
    assert!(!record.object().object().is_object_updated());
    record.dynamic_object_mut().unwrap().set_radius(12.5);
    assert!(record.object().object().is_object_updated());
    assert!(
        record
            .dynamic_object()
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary.queued_before, 1);
    assert_eq!(summary.processed, 1);
    assert_eq!(summary.cleared_update_masks, 1);
    assert_eq!(summary.skipped_not_in_world, 0);
    assert_eq!(summary.missing_or_stale, 0);
    assert_eq!(summary.fanout_not_represented, 1);
    assert_eq!(summary.dynamic_object_values_updates.len(), 1);
    let represented_values = &summary.dynamic_object_values_updates[0];
    assert_eq!(represented_values.guid, dynamic_object_guid);
    let dynamic_object_data = represented_values
        .values_update
        .dynamic_object_data
        .as_ref()
        .unwrap();
    assert!(
        dynamic_object_data
            .mask
            .is_set(DYNAMIC_OBJECT_DATA_PARENT_BIT)
    );
    assert!(
        dynamic_object_data
            .mask
            .is_set(DYNAMIC_OBJECT_DATA_RADIUS_BIT)
    );
    assert_eq!(dynamic_object_data.values.radius, 12.5);
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .object()
            .object()
            .is_object_updated()
    );
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .dynamic_object()
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .dynamic_object()
            .unwrap()
            .values_update()
            .has_data()
    );
}

#[test]
fn far_spell_callbacks_drain_fifo_record_execution_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 10,
        action: RepresentedFarSpellCallbackActionLikeCpp::RecordExecution,
    });
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 20,
        action: RepresentedFarSpellCallbackActionLikeCpp::RecordExecution,
    });

    let summary = map.drain_far_spell_callbacks_like_cpp();

    assert_eq!(summary.queued_before, 2);
    assert_eq!(summary.processed, 2);
    assert_eq!(summary.record_only, 2);
    assert_eq!(summary.queued_after, 0);
    assert_eq!(map.far_spell_callbacks_count_like_cpp(), 0);
    assert_eq!(
        map.represented_far_spell_callback_execution_log_like_cpp(),
        &[10, 20]
    );
}

#[test]
fn far_spell_callback_queue_object_remove_missing_records_stale_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let missing_guid = guid(HighGuid::DynamicObject, 487_001);
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 1,
        action: RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove { guid: missing_guid },
    });

    let summary = map.drain_far_spell_callbacks_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.remove_queue_attempted, 1);
    assert_eq!(summary.remove_queued, 0);
    assert_eq!(summary.remove_missing_or_stale, 1);
    assert_eq!(summary.remove_duplicates, 0);
    assert_eq!(summary.queued_after, 0);
}

fn create_farsight_focus_for_tests<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    caster_player_guid: ObjectGuid,
) -> FarsightDynamicObjectCreateOutcomeLikeCpp
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    map.create_farsight_dynamic_object_like_cpp(
        caster_player_guid,
        12_345,
        678,
        Position::new(100.0, 200.0, 30.0, 1.5),
        42.5,
        30_000,
        987_654,
        1,
        7,
    )
}

#[test]
fn farsight_dynamic_object_create_inserts_focus_and_sets_viewpoint_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4280101);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    assert_eq!(outcome.caster_player_guid, player_guid);
    assert_eq!(outcome.low_guid, Some(1));
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    assert_eq!(dynamic_guid.high_type(), HighGuid::DynamicObject);
    assert_ne!(dynamic_guid.counter(), 12_345);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        2
    );
    let add_to_map = outcome.add_to_map.unwrap();
    assert!(add_to_map.inserted);
    assert!(add_to_map.inserted_into_cell);
    assert!(!add_to_map.already_in_world);

    let dynamic_object = map.get_typed_dynamic_object(dynamic_guid).unwrap();
    assert_eq!(dynamic_object.world().guid(), dynamic_guid);
    assert_eq!(dynamic_object.world().map_id(), 571);
    assert_eq!(dynamic_object.world().instance_id(), 7);
    assert_eq!(
        dynamic_object.world().position(),
        Position::new(100.0, 200.0, 30.0, 1.5)
    );
    assert!(dynamic_object.world().object().is_in_world());
    assert!(dynamic_object.world().is_active());
    assert_eq!(dynamic_object.world().object().entry(), 12_345);
    assert_eq!(dynamic_object.world().object().scale(), 1.0);
    assert_eq!(dynamic_object.caster_guid(), player_guid);
    assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
    assert_eq!(
        dynamic_object.data().dynamic_object_type,
        DynamicObjectType::FarsightFocus as u8
    );
    assert_eq!(dynamic_object.data().spell_visual_id, 678);
    assert_eq!(dynamic_object.spell_id(), 12_345);
    assert_eq!(dynamic_object.radius(), 42.5);
    assert_eq!(dynamic_object.data().cast_time_ms, 987_654);
    assert_eq!(dynamic_object.duration_ms(), 30_000);
    assert!(dynamic_object.is_caster_viewpoint());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_guid
    );
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(viewpoint.dynamic_object_guid, dynamic_guid);
    assert_eq!(
        viewpoint.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Applied
    );
    assert!(viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(viewpoint.player_set_viewpoint.set_seer_requested);
}

#[test]
fn farsight_dynamic_object_create_missing_caster_does_not_mutate_or_consume_low_guid_like_cpp() {
    let mut map = test_map();
    let missing_player_guid = guid(HighGuid::Player, 4280201);

    let outcome = create_farsight_focus_for_tests(&mut map, missing_player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer
    );
    assert_eq!(outcome.dynamic_object_guid, None);
    assert_eq!(map.entity_world.len(), 0);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}

#[test]
fn farsight_dynamic_object_create_untyped_caster_record_does_not_mutate_like_cpp() {
    let mut map = test_map();
    let mut player_object = world_object_with_counter(HighGuid::Player, 4280301, 571, 7, true);
    let player_guid = player_object.guid();
    player_object.object_mut().add_to_world();
    map.insert_map_object(AccessorObjectKind::Player, player_object)
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer
    );
    assert_eq!(map.entity_world.len(), 1);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}

#[test]
fn farsight_dynamic_object_create_caster_not_in_world_or_wrong_map_do_not_mutate_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_player = test_player_for_viewpoint(4280401);
    let not_in_world_guid = not_in_world_player.guid();
    not_in_world_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(MapObjectRecord::new_player(not_in_world_player).unwrap())
        .unwrap();

    let not_in_world = create_farsight_focus_for_tests(&mut not_in_world_map, not_in_world_guid);

    assert_eq!(
        not_in_world.status,
        FarsightDynamicObjectCreateStatusLikeCpp::CasterNotInWorld
    );
    assert_eq!(not_in_world_map.entity_world.len(), 1);
    assert_eq!(
        not_in_world_map
            .get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );

    let mut wrong_map = test_map();
    let wrong_map_player = test_player_for_viewpoint(4280402);
    let wrong_map_guid = wrong_map_player.guid();
    wrong_map
        .insert_map_object_record(MapObjectRecord::new_player(wrong_map_player).unwrap())
        .unwrap();
    wrong_map.map_id = 530;

    let wrong_map_outcome = create_farsight_focus_for_tests(&mut wrong_map, wrong_map_guid);

    assert_eq!(
        wrong_map_outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::CasterWrongMap
    );
    assert_eq!(wrong_map.entity_world.len(), 1);
    assert_eq!(
        wrong_map
            .get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}

#[test]
fn farsight_dynamic_object_create_invalid_destination_preserves_no_mutation_like_cpp() {
    let invalid_destinations = [
        Position::new(f32::NAN, 200.0, 30.0, 1.5),
        Position::new(100.0, 200.0, f32::NAN, 1.5),
        Position::new(100.0, 200.0, Position::MAP_HALFSIZE_LIKE_CPP, 1.5),
        Position::new(100.0, 200.0, 30.0, f32::NAN),
        Position::new(100.0, 200.0, 30.0, f32::INFINITY),
    ];

    for (index, dest) in invalid_destinations.into_iter().enumerate() {
        let mut map = test_map();
        let player = test_player_for_viewpoint(4280501 + index as i64);
        let player_guid = player.guid();
        map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();

        let outcome = map.create_farsight_dynamic_object_like_cpp(
            player_guid,
            12_345,
            678,
            dest,
            42.5,
            30_000,
            987_654,
            1,
            7,
        );

        assert_eq!(
            outcome.status,
            FarsightDynamicObjectCreateStatusLikeCpp::InvalidDestination
        );
        assert_eq!(map.entity_world.len(), 1);
        assert_eq!(
            map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
                .unwrap(),
            1
        );
        assert_eq!(
            map.get_typed_player(player_guid)
                .unwrap()
                .active_data()
                .farsight_object,
            ObjectGuid::EMPTY
        );
    }
}

#[test]
fn farsight_dynamic_object_create_reports_viewpoint_no_mutation_without_panicking_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4280601);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4280609);
    player.set_farsight_object_like_cpp(existing_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(!viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}

#[test]
fn dynamic_object_caster_viewpoint_apply_sets_player_and_toggles_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260102);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(outcome.player_guid, player_guid);
    assert_eq!(outcome.dynamic_object_guid, dynamic_object_guid);
    assert!(outcome.apply);
    assert_eq!(
        outcome.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Applied
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(outcome.player_set_viewpoint.update_visibility_requested);
    assert!(outcome.player_set_viewpoint.set_seer_requested);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}

#[test]
fn dynamic_object_caster_viewpoint_apply_existing_viewpoint_only_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4260209);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260202);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(!outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}

#[test]
fn dynamic_object_caster_viewpoint_remove_match_clears_player_and_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260301);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260302);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, false);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}

#[test]
fn dynamic_object_caster_viewpoint_remove_mismatch_only_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260401);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4260409);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260402);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, false);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(!outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}

#[test]
fn dynamic_object_caster_viewpoint_missing_records_do_not_create_or_mutate_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260501);
    let player_guid = player.guid();
    let missing_dynamic_object_guid = guid(HighGuid::DynamicObject, 4260502);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let missing_dynamic_object =
        map.apply_dynamic_object_caster_viewpoint_like_cpp(missing_dynamic_object_guid, true);

    assert_eq!(
        missing_dynamic_object.status,
        DynamicObjectCasterViewpointStatusLikeCpp::MissingDynamicObject
    );
    assert_eq!(
        missing_dynamic_object.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingTarget
    );
    assert!(!missing_dynamic_object.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert_eq!(map.map_object_count(), 1);

    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260503);
    let dynamic_object_guid = dynamic_object.world().guid();
    let missing_player_guid = guid(HighGuid::Player, 4260504);
    dynamic_object.set_caster_guid(missing_player_guid);
    dynamic_object.bind_to_caster(missing_player_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let missing_player =
        map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        missing_player.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterNotPlayer
    );
    assert_eq!(
        missing_player.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingPlayer
    );
    assert!(!missing_player.dynamic_object_viewpoint_toggled);
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.map_object_count(), 2);
}

#[test]
fn dynamic_object_caster_viewpoint_absent_bound_caster_no_mutation_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260601);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260602);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        outcome.status,
        DynamicObjectCasterViewpointStatusLikeCpp::MissingCaster
    );
    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingPlayer
    );
    assert!(!outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_caster_viewpoint_match_cleans_player_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270102);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(viewpoint.player_guid, player_guid);
    assert_eq!(viewpoint.dynamic_object_guid, dynamic_object_guid);
    assert!(!viewpoint.apply);
    assert_eq!(
        viewpoint.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_caster_viewpoint_mismatch_keeps_player_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4270209);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270202);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(!viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_not_viewpoint_skips_cleanup_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270301);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270302);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_not_in_world_skips_viewpoint_cleanup_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270401);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270402);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert!(!removed.was_in_world);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_aura_and_caster_cleanup_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4300101);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300102);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(caster_guid);
    dynamic_object.set_aura_bound();
    dynamic_object.bind_to_caster(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: true,
            removed_aura_pending_delete: true,
            unbound_caster: Some(caster_guid),
        })
    );
    assert!(!removed.object.unwrap().object().is_in_world());
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_without_aura_or_caster_reports_no_cleanup_like_cpp() {
    let mut map = test_map();
    let dynamic_object = test_dynamic_object_for_viewpoint(4300201);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: false,
            removed_aura_pending_delete: false,
            unbound_caster: None,
        })
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_not_in_world_skips_aura_and_caster_cleanup_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4300301);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300302);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(caster_guid);
    dynamic_object.set_aura_bound();
    dynamic_object.bind_to_caster(caster_guid);
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(removed.dynamic_object_remove_cleanup, None);
    assert!(!removed.was_in_world);
    assert!(!removed.object.unwrap().object().is_in_world());
}

#[test]
fn remove_from_map_like_cpp_dynamic_object_viewpoint_aura_and_caster_order_evidence_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4300401);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300402);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    dynamic_object.set_aura_bound();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: true,
            removed_aura_pending_delete: true,
            unbound_caster: Some(player_guid),
        })
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}

#[test]
fn move_list_add_same_guid_updates_position_without_duplicate_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44101, 4410101, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let first = Position::xyz(2.0, 3.0, 4.0);
    let second = Position::xyz(3.0, 4.0, 5.0);

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, first),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, second),
        AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
    );

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );
    let pending = map
        .pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
        .unwrap();
    assert_eq!(pending.state, MapObjectCellMoveStateLikeCpp::Active);
    assert_eq!(pending.new_position, second);
}

#[test]
fn move_list_remove_marks_inactive_and_drain_resets_without_relocation_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44102, 4410201, true);
    let guid = creature.guid();
    let original_position = creature.unit().world().position();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, Position::xyz(50.0, 50.0, 6.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.remove_creature_from_move_list_like_cpp(guid),
        RemoveObjectFromMoveListOutcomeLikeCpp::MarkedInactive
    );
    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.inactive_reset, 1);
    assert_eq!(summary.relocated, 0);
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid),
        None
    );
    assert_eq!(map.map_object(guid).unwrap().position(), original_position);
}

#[test]
fn move_list_drain_active_in_world_relocates_cell_membership_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44103, 4410301, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let new_position = Position::xyz(120.0, 120.0, 7.0);
    map.load_grid(new_position.x, new_position.y);
    let new_cell = Cell::from_world(new_position.x, new_position.y);

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, new_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.relocated, 1);
    let stored = map.map_object(guid).unwrap();
    assert_eq!(stored.position(), new_position);
    assert_eq!(
        stored.current_cell(),
        Some((new_cell.cell_x(), new_cell.cell_y()))
    );
    let nearby = map.exact_cell_guids_like_cpp(new_cell.cell_coord());
    assert!(nearby.grid.creatures.contains(&guid) || nearby.world.creatures.contains(&guid));
}

#[test]
fn move_list_drain_tolerates_missing_wrong_kind_and_not_in_world_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 4410401);
    let gameobject = test_gameobject_for_spawn(44104, 4410402);
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    let mut not_in_world = test_creature_for_spawn(44105, 4410403, true);
    not_in_world
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let not_in_world_guid = not_in_world.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(missing_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::MissingOrStale
    );
    assert!(matches!(
        map.add_creature_to_move_list_like_cpp(gameobject_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::WrongKind {
            actual: AccessorObjectKind::GameObject
        }
    ));
    map.creatures_to_move.push(missing_guid);
    map.creature_move_states.insert(
        missing_guid,
        PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: Position::xyz(2.0, 2.0, 2.0),
        },
    );
    map.creatures_to_move.push(gameobject_guid);
    map.creature_move_states.insert(
        gameobject_guid,
        PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: Position::xyz(2.0, 2.0, 2.0),
        },
    );
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(not_in_world_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 3);
    assert_eq!(summary.missing_or_stale, 1);
    assert_eq!(summary.wrong_kind, 1);
    assert_eq!(summary.not_in_world, 1);
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, missing_guid),
        None
    );
}

#[test]
fn move_list_dynamic_and_area_trigger_blocked_unloaded_grid_do_not_queue_remove_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4410501);
    dynamic_object.world_mut().set_active(false);
    let dynamic_guid = dynamic_object.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
    )
    .unwrap();
    let area_trigger = test_area_trigger_for_update(4410502, 10_000, true);
    let area_guid = area_trigger.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let unloaded_grid_position = Position::xyz(5_000.0, 5_000.0, 1.0);

    assert_eq!(
        map.add_dynamic_object_to_move_list_like_cpp(dynamic_guid, unloaded_grid_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_area_trigger_to_move_list_like_cpp(area_guid, unloaded_grid_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let dyn_summary = map.move_all_dynamic_objects_in_move_list_like_cpp();
    let area_summary = map.move_all_area_triggers_in_move_list_like_cpp();

    assert_eq!(dyn_summary.blocked_by_unloaded_grid, 1);
    assert_eq!(area_summary.blocked_by_unloaded_grid, 1);
    assert_eq!(dyn_summary.remove_list_queued, 0);
    assert_eq!(area_summary.remove_list_queued, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(dynamic_guid).is_some());
    assert!(map.map_object_record(area_guid).is_some());
}

#[test]
fn unload_grid_at_false_consumes_creature_gameobject_area_trigger_move_lists_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44106, 4410601, true);
    let creature_guid = creature.guid();
    let gameobject = test_gameobject_for_spawn(44106, 4410602);
    let gameobject_guid = gameobject.world().guid();
    let area_trigger = test_area_trigger_for_update(4410603, 10_000, true);
    let area_guid = area_trigger.world().guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let same_cell_new_position = Position::xyz(2.0, 3.0, 4.0);
    map.add_creature_to_move_list_like_cpp(creature_guid, same_cell_new_position);
    map.add_game_object_to_move_list_like_cpp(gameobject_guid, same_cell_new_position);
    map.add_area_trigger_to_move_list_like_cpp(area_guid, same_cell_new_position);

    let unload_position = Position::xyz(3_000.0, 3_000.0, 0.0);
    map.load_grid(unload_position.x, unload_position.y);
    let unload_cell = Cell::from_world(unload_position.x, unload_position.y);
    let unload_grid = GridCoord::new(unload_cell.grid_x(), unload_cell.grid_y());

    assert!(map.unload_grid_at(unload_grid, false));

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        0
    );
    assert_eq!(
        map.map_object(creature_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(
        map.map_object(gameobject_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(
        map.map_object(area_guid).unwrap().position(),
        same_cell_new_position
    );
    assert_eq!(map.lifecycle().evacuates, 1);
}

#[test]
fn unload_grid_at_true_does_not_drain_move_lists_and_unload_all_helper_clears_cpp_subset_like_cpp()
{
    let mut map = test_map();
    let creature = test_creature_for_spawn(44107, 4410701, true);
    let creature_guid = creature.guid();
    let creature_original_position = creature.unit().world().position();
    let gameobject = test_gameobject_for_spawn(44107, 4410702);
    let gameobject_guid = gameobject.world().guid();
    let gameobject_original_position = gameobject.world().position();
    let area_trigger = test_area_trigger_for_update(4410703, 10_000, true);
    let area_guid = area_trigger.world().guid();
    let area_original_position = area_trigger.world().position();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let pending_position = Position::xyz(2.0, 3.0, 4.0);
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(creature_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_game_object_to_move_list_like_cpp(gameobject_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_area_trigger_to_move_list_like_cpp(area_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let unload_position = Position::xyz(3_500.0, 3_500.0, 0.0);
    map.load_grid(unload_position.x, unload_position.y);
    let unload_cell = Cell::from_world(unload_position.x, unload_position.y);
    let unload_grid = GridCoord::new(unload_cell.grid_x(), unload_cell.grid_y());

    assert!(map.unload_grid_at(unload_grid, true));

    assert_eq!(
        map.map_object(creature_guid).unwrap().position(),
        creature_original_position
    );
    assert_eq!(
        map.map_object(gameobject_guid).unwrap().position(),
        gameobject_original_position
    );
    assert_eq!(
        map.map_object(area_guid).unwrap().position(),
        area_original_position
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        1
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        1
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, creature_guid)
            .unwrap()
            .new_position,
        pending_position
    );
    assert_eq!(map.lifecycle().evacuates, 0);

    map.clear_unload_all_delayed_moves_like_cpp();

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        1
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, creature_guid),
        None
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, gameobject_guid),
        None
    );
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, area_guid)
            .is_some()
    );
}

fn test_area_trigger_for_update(counter: i64, duration_ms: i32, in_world: bool) -> AreaTrigger {
    let mut area_trigger = AreaTrigger::new();
    area_trigger
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::AreaTrigger, counter));
    area_trigger.world_mut().object_mut().set_entry(42);
    area_trigger.world_mut().set_map(571, 7).unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        area_trigger.world_mut().object_mut().add_to_world();
    }
    area_trigger.set_duration(duration_ms);
    area_trigger
}

#[test]
fn area_trigger_update_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340101, 1_000, true);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(250));
    assert!(outcome.non_static_movement_would_run);
    assert!(outcome.ai_update_would_run);
    assert!(outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 750);
    assert_eq!(area_trigger.time_since_created_ms(), 250);
    assert!(!area_trigger.is_removed());
}

#[test]
fn area_trigger_update_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340201, 250, true);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(
        outcome.status,
        AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(250));
    assert!(outcome.non_static_movement_would_run);
    assert!(!outcome.ai_update_would_run);
    assert!(!outcome.target_list_update_would_run);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert!(area_trigger.is_removed());
}

#[test]
fn area_trigger_update_permanent_duration_increments_time_without_queue_like_cpp() {
    let mut map = test_map();
    let mut area_trigger = test_area_trigger_for_update(4340301, -1, true);
    area_trigger.set_spawn_id(4340301);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 1_000);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(-1));
    assert_eq!(outcome.duration_after_ms, Some(-1));
    assert_eq!(outcome.time_since_created_after_ms, Some(1_000));
    assert!(!outcome.non_static_movement_would_run);
    assert!(outcome.ai_update_would_run);
    assert!(outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn area_trigger_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340401, 500, false);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(500));
    assert_eq!(outcome.duration_after_ms, Some(500));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(0));
    assert!(!outcome.non_static_movement_would_run);
    assert!(!outcome.ai_update_would_run);
    assert!(!outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 500);
    assert_eq!(area_trigger.time_since_created_ms(), 0);
    assert!(!area_trigger.is_removed());
}

#[test]
fn area_trigger_update_missing_or_non_area_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::AreaTrigger, 4340501);
    let creature_guid = guid(HighGuid::Creature, 4340502);
    let creature = test_creature_for_spawn(43405, 4340502, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let missing = map.update_area_trigger_like_cpp(missing_guid, 250);
    let non_area = map.update_area_trigger_like_cpp(creature_guid, 250);

    assert_eq!(
        missing.status,
        AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger
    );
    assert_eq!(
        non_area.status,
        AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
}

#[test]
fn remove_all_area_triggers_for_caster_removes_only_matching_caster_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4340601);
    let other_caster_guid = guid(HighGuid::Player, 4340602);

    let mut matching_area_trigger = test_area_trigger_for_update(4340603, 1_000, true);
    let matching_guid = matching_area_trigger.world().guid();
    matching_area_trigger.set_caster_guid(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(matching_area_trigger).unwrap())
        .unwrap();

    let mut other_area_trigger = test_area_trigger_for_update(4340604, 1_000, true);
    let other_guid = other_area_trigger.world().guid();
    other_area_trigger.set_caster_guid(other_caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(other_area_trigger).unwrap())
        .unwrap();

    let outcome = map.remove_all_area_triggers_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.caster_guid, caster_guid);
    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert!(map.map_object_record(matching_guid).is_none());
    assert!(map.map_object_record(other_guid).is_some());
}

#[test]
fn remove_all_area_triggers_for_caster_ignores_other_object_kinds_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4340611);
    let creature = test_creature_for_spawn(4340612, 4340612, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.remove_all_area_triggers_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.candidates, 0);
    assert_eq!(outcome.removed, 0);
    assert!(map.map_object_record(creature_guid).is_some());
}

#[test]
fn send_object_updates_clears_in_world_changed_object_like_cpp() {
    let mut map = test_map();
    let game_object = test_gameobject_for_spawn(4450101, 4450101);
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&game_object_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .set_scale(2.0);

    let before = map.map_object(game_object_guid).unwrap().object();
    assert!(before.is_object_updated());
    assert!(!before.changed_fields().is_empty());

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(
        summary,
        SendObjectUpdatesSummaryLikeCpp {
            queued_before: 1,
            processed: 1,
            cleared_update_masks: 1,
            skipped_not_in_world: 0,
            missing_or_stale: 0,
            fanout_not_represented: 1,
            dynamic_object_values_updates: Vec::new(),
            player_values_updates: Vec::new(),
            unit_values_updates: Vec::new(),
        }
    );
    let after = map.map_object(game_object_guid).unwrap().object();
    assert!(!after.is_object_updated());
    assert!(after.changed_fields().is_empty());
}

#[test]
fn send_object_updates_consumes_queued_player_stand_state_like_cpp() {
    let mut map = test_map();
    let player_guid = ObjectGuid::create_player(1, 4_450_151);
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();

    player.set_inebriation_like_cpp(7);
    player.set_money(42);
    player
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);
    assert!(player.unit().world().object().is_object_updated());
    assert!(
        player
            .unit()
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_BIT)
    );
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary.queued_before, 1);
    assert_eq!(summary.processed, 1);
    assert_eq!(summary.cleared_update_masks, 1);
    assert_eq!(summary.player_values_updates.len(), 1);
    let captured = &summary.player_values_updates[0];
    assert_eq!(captured.guid, player_guid);
    assert!(
        captured
            .values_update
            .unit_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(UNIT_DATA_STAND_STATE_BIT))
    );
    assert!(
        captured
            .values_update
            .player_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(PLAYER_DATA_INEBRIATION_BIT)),
        "an unrelated PlayerData delta is captured before masks are cleared"
    );
    assert!(
        captured
            .values_update
            .active_player_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(ACTIVE_PLAYER_DATA_COINAGE_BIT)),
        "an unrelated ActivePlayerData delta is captured before masks are cleared"
    );
    let player = map
        .map_object_record(player_guid)
        .and_then(MapObjectRecord::player)
        .expect("typed Player remains on map");
    assert_eq!(
        player.unit().stand_state_like_cpp(),
        UnitStandStateType::Sit
    );
    assert!(!player.unit().world().object().is_object_updated());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
    assert!(
        !player
            .unit()
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_BIT),
        "canonical SendObjectUpdates consumes the Player UnitData delta"
    );
}

#[test]
fn send_object_updates_skips_unchanged_objects_like_cpp() {
    let mut map = test_map();
    let game_object = test_gameobject_for_spawn(4450201, 4450201);
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    let before = map.map_object(game_object_guid).unwrap().object();
    assert!(!before.is_object_updated());
    assert!(before.changed_fields().is_empty());

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary, SendObjectUpdatesSummaryLikeCpp::default());
    let after = map.map_object(game_object_guid).unwrap().object();
    assert!(!after.is_object_updated());
    assert!(after.changed_fields().is_empty());
}

#[test]
fn send_object_updates_not_in_world_updated_state_is_not_publicly_constructible() {
    // C++ `_updateObjects` should never contain not-in-world objects:
    // `Object::AddToObjectUpdateIfNeeded` only sets `m_objectUpdated` when
    // `m_inWorld`, and `Object::remove_from_world`/`ClearUpdateMask(true)`
    // clears the flag. Rust mirrors that public invariant in
    // `EntityObject`, whose `object_updated`/`in_world` fields are private to
    // `wow-entities`, so wow-map tests cannot construct the defensive
    // `skipped_not_in_world` branch without unsafe/private-field hacks.
    let mut map = test_map();
    let mut game_object = test_gameobject_for_spawn(4450301, 4450301);
    game_object.world_mut().object_mut().set_scale(2.0);
    game_object.world_mut().object_mut().remove_from_world();
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    assert!(
        !map.map_object(game_object_guid)
            .unwrap()
            .object()
            .is_in_world()
    );
    assert!(
        !map.map_object(game_object_guid)
            .unwrap()
            .object()
            .is_object_updated()
    );
    assert_eq!(
        map.send_object_updates_like_cpp(),
        SendObjectUpdatesSummaryLikeCpp::default()
    );
}

fn test_conversation_for_update(counter: i64, duration_ms: i32, in_world: bool) -> Conversation {
    let mut conversation = Conversation::new();
    conversation
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Conversation, counter));
    conversation.world_mut().object_mut().set_entry(42);
    conversation.world_mut().set_map(571, 7).unwrap();
    conversation
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        conversation.world_mut().object_mut().add_to_world();
    }
    conversation.set_duration_ms(duration_ms);
    conversation
}

fn test_transport_for_update(counter: i64, in_world: bool) -> Transport {
    let template = wow_entities::TransportTemplate {
        total_path_time_ms: 1_000,
        path_legs: vec![wow_entities::TransportPathLeg {
            map_id: 571,
            start_timestamp_ms: 0,
            duration_ms: 1_000,
            segments: vec![],
        }],
        ..wow_entities::TransportTemplate::default()
    };
    let mut transport = Transport::with_template(template);
    transport
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Transport, counter));
    transport.world_mut().set_map(571, 7).unwrap();
    transport.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    transport.set_path_progress_ms(100);
    if in_world {
        transport.world_mut().object_mut().add_to_world();
    }
    transport
}

#[test]
fn transport_update_mutates_typed_canonical_record_like_cpp() {
    let mut map = test_map();
    let transport = test_transport_for_update(4390101, true);
    let transport_guid = transport.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    let outcome = map.update_transport_like_cpp(transport_guid, 50, 10_000);

    assert_eq!(outcome.status, TransportUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.path_progress_before_ms, Some(100));
    assert_eq!(outcome.path_progress_after_ms, Some(150));
    assert_eq!(outcome.timer_ms, Some(150));
    assert!(outcome.position_update_represented);
    let transport = map
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 150);
}

#[test]
fn transport_update_wrong_kind_missing_untyped_skip_but_not_in_world_updates_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Transport, 4390201);
    let creature = test_creature_for_spawn(43902, 4390202, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let not_in_world = test_transport_for_update(4390203, false);
    let not_in_world_guid = not_in_world.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(not_in_world).unwrap())
        .unwrap();
    let untyped_transport = world_object_with_counter(HighGuid::Transport, 4390204, 571, 7, true);
    let untyped_guid = untyped_transport.guid();
    map.insert_map_object(AccessorObjectKind::Transport, untyped_transport)
        .unwrap();

    let missing = map.update_transport_like_cpp(missing_guid, 50, 10_000);
    let wrong_kind = map.update_transport_like_cpp(creature_guid, 50, 10_000);
    let not_in_world = map.update_transport_like_cpp(not_in_world_guid, 50, 10_000);
    let untyped = map.update_transport_like_cpp(untyped_guid, 50, 10_000);

    assert_eq!(
        missing.status,
        TransportUpdateStatusLikeCpp::MissingTransport
    );
    assert_eq!(
        wrong_kind.status,
        TransportUpdateStatusLikeCpp::NotTransport
    );
    assert_eq!(not_in_world.status, TransportUpdateStatusLikeCpp::Updated);
    assert_eq!(not_in_world.path_progress_before_ms, Some(100));
    assert_eq!(not_in_world.path_progress_after_ms, Some(150));
    assert_eq!(untyped.status, TransportUpdateStatusLikeCpp::NotTransport);
    assert_eq!(map.map_object_count(), 3);
    let transport = map
        .map_object_record(not_in_world_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 150);
}

#[test]
fn transports_update_summary_snapshots_only_typed_transports_like_cpp() {
    let mut map = test_map();
    let typed_transport = test_transport_for_update(4390301, true);
    let typed_guid = typed_transport.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(typed_transport).unwrap())
        .unwrap();
    let untyped_transport = world_object_with_counter(HighGuid::Transport, 4390302, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::Transport, untyped_transport)
        .unwrap();
    let creature = test_creature_for_spawn(43903, 4390303, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let summary = map.update_transports_like_cpp(250, 10_000);

    assert_eq!(
        summary,
        TransportsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            unsupported_no_period: 0,
            missing_or_stale: 0,
            not_transport: 0,
            not_in_world: 0,
            position_updates_represented: 1,
            just_stopped: 0,
        }
    );
    let transport = map
        .map_object_record(typed_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 350);
}

#[test]
fn transport_update_period_zero_reports_unsupported_without_mutation_like_cpp() {
    let mut map = test_map();
    let mut transport = test_transport_for_update(4390401, true);
    let transport_guid = transport.world().guid();
    transport.set_period(0);
    transport.set_path_progress_ms(333);
    map.insert_map_object_record(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    let outcome = map.update_transport_like_cpp(transport_guid, 50, 10_000);

    assert_eq!(
        outcome.status,
        TransportUpdateStatusLikeCpp::UnsupportedNoPeriod
    );
    assert_eq!(outcome.path_progress_before_ms, Some(333));
    assert_eq!(outcome.path_progress_after_ms, Some(333));
    assert_eq!(outcome.timer_ms, None);
    let transport = map
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 333);
}

fn test_scene_object_for_update(
    counter: i64,
    in_world: bool,
    created_by_spell_cast: ObjectGuid,
) -> SceneObject {
    let mut scene_object = SceneObject::new();
    scene_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::SceneObject, counter));
    scene_object.world_mut().object_mut().set_entry(42);
    scene_object.world_mut().set_map(571, 7).unwrap();
    scene_object
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    scene_object.set_created_by(guid(HighGuid::Player, counter + 1_000));
    scene_object.set_created_by_spell_cast(created_by_spell_cast);
    if in_world {
        scene_object.world_mut().object_mut().add_to_world();
    }
    scene_object
}

#[test]
fn scene_object_update_with_creator_and_aura_updates_without_queue_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370101, true, guid(HighGuid::Cast, 4370102));
    let scene_object_guid = scene_object.world().guid();
    let owner_guid = scene_object.owner_guid();
    let cast_guid = scene_object.created_by_spell_cast();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: true,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.owner_guid, Some(owner_guid));
    assert_eq!(outcome.created_by_spell_cast, Some(cast_guid));
    assert!(outcome.creator_exists);
    assert!(outcome.linked_aura_exists);
    assert!(outcome.world_update_would_run);
    assert!(!outcome.should_be_removed);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(scene_object_guid).is_some());
}

#[test]
fn scene_object_update_missing_creator_queues_remove_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370201, true, ObjectGuid::EMPTY);
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: false,
            linked_aura_exists: true,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::RemoveQueued);
    assert!(outcome.world_update_would_run);
    assert!(outcome.should_be_removed);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(scene_object_guid).is_some());
}

#[test]
fn scene_object_update_missing_linked_aura_queues_remove_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370301, true, guid(HighGuid::Cast, 4370302));
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: false,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::RemoveQueued);
    assert!(outcome.world_update_would_run);
    assert!(outcome.should_be_removed);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn scene_object_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370401, false, guid(HighGuid::Cast, 4370402));
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: false,
            linked_aura_exists: false,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::NotInWorld);
    assert!(!outcome.world_update_would_run);
    assert!(!outcome.should_be_removed);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(scene_object_guid).is_some());
}

#[test]
fn scene_object_update_missing_non_scene_or_untyped_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::SceneObject, 4370501);
    let creature = test_creature_for_spawn(43705, 4370502, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped_scene = world_object_with_counter(HighGuid::SceneObject, 4370503, 571, 7, true);
    let untyped_scene_guid = untyped_scene.guid();
    map.insert_map_object(AccessorObjectKind::SceneObject, untyped_scene)
        .unwrap();

    let context = SceneObjectUpdateContextLikeCpp {
        creator_exists: false,
        linked_aura_exists: false,
    };
    let missing = map.update_scene_object_like_cpp(missing_guid, 250, context);
    let non_scene = map.update_scene_object_like_cpp(creature_guid, 250, context);
    let untyped = map.update_scene_object_like_cpp(untyped_scene_guid, 250, context);

    assert_eq!(
        missing.status,
        SceneObjectUpdateStatusLikeCpp::MissingSceneObject
    );
    assert_eq!(
        non_scene.status,
        SceneObjectUpdateStatusLikeCpp::NotSceneObject
    );
    assert_eq!(
        untyped.status,
        SceneObjectUpdateStatusLikeCpp::NotSceneObject
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_scene.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_scene_guid).is_some());
}

#[test]
fn scene_objects_update_summary_snapshots_only_typed_scene_objects_like_cpp() {
    let mut map = test_map();
    let typed_scene = test_scene_object_for_update(4370601, true, ObjectGuid::EMPTY);
    let typed_scene_guid = typed_scene.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(typed_scene).unwrap())
        .unwrap();
    let untyped_scene = world_object_with_counter(HighGuid::SceneObject, 4370602, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::SceneObject, untyped_scene)
        .unwrap();
    let creature = test_creature_for_spawn(43706, 4370603, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let summary = map.update_scene_objects_like_cpp(250, |_guid, scene_object| {
        assert_eq!(scene_object.world().guid(), typed_scene_guid);
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: true,
        }
    });

    assert_eq!(
        summary,
        SceneObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            remove_queued: 0,
            missing_or_stale: 0,
            not_scene_object: 0,
            not_in_world: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn conversation_update_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360101, 1_000, true);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(outcome.status, ConversationUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert!(outcome.script_update_would_run);
    assert!(outcome.world_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert_eq!(conversation.duration_ms(), 750);
    assert!(!conversation.is_removed());
}

#[test]
fn conversation_update_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360201, 250, true);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(
        outcome.status,
        ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert!(outcome.script_update_would_run);
    assert!(!outcome.world_update_would_run);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert!(conversation.is_removed());
}

#[test]
fn conversation_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360301, 500, false);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(outcome.status, ConversationUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(500));
    assert_eq!(outcome.duration_after_ms, Some(500));
    assert!(!outcome.script_update_would_run);
    assert!(!outcome.world_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert_eq!(conversation.duration_ms(), 500);
    assert!(!conversation.is_removed());
}

#[test]
fn conversation_update_missing_non_conversation_or_untyped_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Conversation, 4360401);
    let creature = test_creature_for_spawn(43604, 4360402, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped_conversation =
        world_object_with_counter(HighGuid::Conversation, 4360403, 571, 7, true);
    let untyped_conversation_guid = untyped_conversation.guid();
    map.insert_map_object(AccessorObjectKind::Conversation, untyped_conversation)
        .unwrap();

    let missing = map.update_conversation_like_cpp(missing_guid, 250);
    let non_conversation = map.update_conversation_like_cpp(creature_guid, 250);
    let untyped = map.update_conversation_like_cpp(untyped_conversation_guid, 250);

    assert_eq!(
        missing.status,
        ConversationUpdateStatusLikeCpp::MissingConversation
    );
    assert_eq!(
        non_conversation.status,
        ConversationUpdateStatusLikeCpp::NotConversation
    );
    assert_eq!(
        untyped.status,
        ConversationUpdateStatusLikeCpp::NotConversation
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_conversation.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_conversation_guid).is_some());
}

#[test]
fn creature_update_in_world_consumes_runtime_plan_once_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350101);
    let creature = test_creature_for_spawn(43501, 4350101, true);
    assert!(creature.trigger_just_appeared());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let first = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_000,
        CreatureRuntimeUpdateContext::default(),
    );
    let second = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_001,
        CreatureRuntimeUpdateContext::default(),
    );

    assert_eq!(first.status, CreatureUpdateStatusLikeCpp::Updated);
    assert!(
        first
            .plan
            .as_ref()
            .unwrap()
            .contains(wow_entities::CreatureRuntimeAction::NotifyJustAppeared)
    );
    assert!(
        !map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
    assert_eq!(second.status, CreatureUpdateStatusLikeCpp::Updated);
    assert!(
        !second
            .plan
            .as_ref()
            .unwrap()
            .contains(wow_entities::CreatureRuntimeAction::NotifyJustAppeared)
    );
}

#[test]
fn creature_update_not_in_world_skips_without_mutation_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350201);
    let mut creature = test_creature_for_spawn(43502, 4350201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    assert!(creature.trigger_just_appeared());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.update_creature_like_cpp(
        creature_guid,
        1,
        1_000,
        CreatureRuntimeUpdateContext::default(),
    );

    assert_eq!(outcome.status, CreatureUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.actions_recorded, 0);
    assert!(
        map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
}

#[test]
fn creature_update_snapshot_ignores_gameobject_areatrigger_dynamicobject_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350301);
    let mut creature = test_creature_for_spawn(43503, 4350301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(43504, 4350302)).unwrap(),
    )
    .unwrap();
    let mut area_trigger = test_area_trigger_for_update(4350303, 10, true);
    area_trigger.set_duration(10);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4350304);
    dynamic_object.set_duration(10);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let summary = map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
        CreatureRuntimeUpdateContext::default()
    });

    assert_eq!(summary.visited, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.skipped_non_creature, 0);
    assert!(summary.actions_recorded > 0);
    assert!(
        !map.map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .trigger_just_appeared()
    );
    assert_eq!(
        map.map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap()
            .duration_ms(),
        10
    );
    assert_eq!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        10
    );
}

#[test]
fn creature_update_snapshot_skips_unloaded_grid_records_like_cpp() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 4350311);
    let mut creature = test_creature_for_spawn(43503, 4350311, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let added = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(map.unload_grid_at(added.grid, true));
    assert!(map.get_ngrid(added.grid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());

    let summary = map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
        CreatureRuntimeUpdateContext::default()
    });

    assert_eq!(summary.visited, 0);
    assert_eq!(summary.updated, 0);
    assert_eq!(summary.actions_recorded, 0);
}

#[test]
fn creature_update_context_resolver_affects_plan_like_cpp() {
    let mut default_map = test_map();
    let mut default_creature = test_creature_for_spawn(43504, 4350401, true);
    default_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    default_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(default_creature).unwrap(),
        )
        .unwrap();
    let default_summary = default_map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
        CreatureRuntimeUpdateContext::default()
    });

    let mut disabled_ai_map = test_map();
    let mut disabled_ai_creature = test_creature_for_spawn(43504, 4350402, true);
    disabled_ai_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    disabled_ai_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(disabled_ai_creature).unwrap(),
        )
        .unwrap();
    let disabled_ai_summary =
        disabled_ai_map.update_creatures_like_cpp(1, 1_000, |_guid, _creature| {
            CreatureRuntimeUpdateContext {
                ai_enabled: false,
                ..CreatureRuntimeUpdateContext::default()
            }
        });

    assert_eq!(default_summary.visited, 1);
    assert_eq!(disabled_ai_summary.visited, 1);
    assert!(default_summary.actions_recorded > disabled_ai_summary.actions_recorded);
}

#[test]
fn dynamic_object_update_non_aura_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290101);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.dynamic_object_guid, dynamic_object_guid);
    assert_eq!(outcome.elapsed_ms, 250);
    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert!(outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        750
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .world()
            .object()
            .is_destroyed_object()
    );
}

#[test]
fn dynamic_object_update_non_aura_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290201);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(250);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(
        outcome.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert!(!outcome.script_update_would_run);
    let remove_list = outcome.remove_list.unwrap();
    assert_eq!(remove_list.guid, dynamic_object_guid);
    assert!(remove_list.queued);
    assert!(!remove_list.duplicate);
    assert!(!remove_list.missing_or_stale);
    assert_eq!(remove_list.unsupported_kind, None);
    assert_eq!(remove_list.cleanup_before_delete_count, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 250);
    assert!(dynamic_object.world().object().is_destroyed_object());
    assert_eq!(dynamic_object.cleanup_before_delete_count(), 1);
}

#[test]
fn dynamic_object_update_expired_then_remove_list_drain_clears_farsight_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4290301);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_object_guid = create.dynamic_object_guid.unwrap();
    map.get_typed_dynamic_object_mut(dynamic_object_guid)
        .unwrap()
        .set_duration(1);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );

    let update = map.update_dynamic_object_like_cpp(dynamic_object_guid, 1);

    assert_eq!(
        update.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
}

#[test]
fn remove_all_dynamic_objects_for_caster_removes_only_matching_caster_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4290701);
    let other_caster_guid = guid(HighGuid::Player, 4290702);

    let mut matching_dynamic = test_dynamic_object_for_viewpoint(4290703);
    let matching_guid = matching_dynamic.world().guid();
    matching_dynamic.set_caster_guid(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(matching_dynamic).unwrap())
        .unwrap();

    let mut other_dynamic = test_dynamic_object_for_viewpoint(4290704);
    let other_guid = other_dynamic.world().guid();
    other_dynamic.set_caster_guid(other_caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(other_dynamic).unwrap())
        .unwrap();

    let outcome = map.remove_all_dynamic_objects_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.caster_guid, caster_guid);
    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert!(map.map_object_record(matching_guid).is_none());
    assert!(map.map_object_record(other_guid).is_some());
}

#[test]
fn remove_all_dynamic_objects_for_caster_uses_dynamic_object_cleanup_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4290711);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = create.dynamic_object_guid.unwrap();
    {
        let dynamic_object = map.get_typed_dynamic_object_mut(dynamic_guid).unwrap();
        dynamic_object.set_aura_bound();
        assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
        assert!(dynamic_object.has_aura());
    }

    let outcome = map.remove_all_dynamic_objects_for_caster_like_cpp(player_guid);

    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.dynamic_object_remove_aura_cleanup_count, 1);
    assert_eq!(outcome.dynamic_object_unbound_caster_count, 1);
    assert!(map.map_object_record(dynamic_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
}

#[test]
fn dynamic_object_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290401);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert!(!outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 1_000);
    assert!(!dynamic_object.world().object().is_destroyed_object());
}

#[test]
fn dynamic_object_update_aura_bound_not_expired_runs_represented_update_owner_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290501);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    dynamic_object.set_aura_bound();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert_eq!(outcome.aura_update_owner_calls_before, Some(0));
    assert_eq!(outcome.aura_update_owner_calls_after, Some(1));
    assert!(outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 1_000);
    assert!(dynamic_object.has_aura());
    assert_eq!(dynamic_object.represented_aura_update_owner_count(), 1);
    assert!(!dynamic_object.world().object().is_destroyed_object());
}

#[test]
fn dynamic_object_update_aura_bound_expired_queues_remove_and_drain_cleans_aura_caster_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4310201);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_object_guid = create.dynamic_object_guid.unwrap();
    {
        let dynamic_object = map
            .get_typed_dynamic_object_mut(dynamic_object_guid)
            .unwrap();
        dynamic_object.set_duration(1_000);
        dynamic_object.set_aura_bound();
        dynamic_object.set_aura_removed_like_cpp(true);
    }

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(
        outcome.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert_eq!(outcome.aura_update_owner_calls_before, Some(0));
    assert_eq!(outcome.aura_update_owner_calls_after, Some(0));
    assert!(!outcome.script_update_would_run);
    assert!(outcome.remove_list.unwrap().queued);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert!(dynamic_object.has_aura());
    assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
    assert!(dynamic_object.world().object().is_destroyed_object());

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(drain.dynamic_object_remove_aura_cleanup_count, 1);
    assert_eq!(drain.dynamic_object_unbound_caster_count, 1);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}

#[test]
fn dynamic_object_update_missing_or_non_dynamic_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::DynamicObject, 4290601);
    let creature = world_object_with_counter(HighGuid::Creature, 4290602, 571, 7, true);
    let creature_guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    let untyped_dynamic = world_object_with_counter(HighGuid::DynamicObject, 4290603, 571, 7, true);
    let untyped_dynamic_guid = untyped_dynamic.guid();
    map.insert_map_object(AccessorObjectKind::DynamicObject, untyped_dynamic)
        .unwrap();

    let missing = map.update_dynamic_object_like_cpp(missing_guid, 250);
    let non_dynamic = map.update_dynamic_object_like_cpp(creature_guid, 250);
    let untyped = map.update_dynamic_object_like_cpp(untyped_dynamic_guid, 250);

    assert_eq!(
        missing.status,
        DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject
    );
    assert_eq!(
        non_dynamic.status,
        DynamicObjectUpdateStatusLikeCpp::NotDynamicObject
    );
    assert_eq!(
        untyped.status,
        DynamicObjectUpdateStatusLikeCpp::NotDynamicObject
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_dynamic.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_dynamic_guid).is_some());
}

#[test]
fn player_set_viewpoint_apply_unit_target_consumes_set_world_object_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4240101);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424010, 4240102);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, true, None);

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Applied);
    assert!(outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(
        outcome.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: true,
            status: SetWorldObjectStatusLikeCpp::Delegated(
                AddObjectToSwitchListStatusLikeCpp::Queued
            ),
        })
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        target_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .contains(&player_guid)
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(true));
}

#[test]
fn player_set_viewpoint_apply_existing_viewpoint_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4240209);
    player.set_farsight_object_like_cpp(existing_guid);
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424020, 4240202);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, true, None);

    assert_eq!(
        outcome.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert_eq!(outcome.set_world_object, None);
    assert!(!outcome.update_visibility_requested);
    assert!(!outcome.set_seer_requested);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);
}

#[test]
fn player_set_viewpoint_remove_last_viewer_consumes_set_world_object_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240301);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424030, 4240302);
    player.set_farsight_object_like_cpp(target_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.get_typed_creature_mut(target_guid)
        .unwrap()
        .unit_mut()
        .add_player_to_vision_like_cpp(player_guid);
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, false, None);

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Removed);
    assert!(!outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(
        outcome.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: false,
            status: SetWorldObjectStatusLikeCpp::Delegated(
                AddObjectToSwitchListStatusLikeCpp::Queued
            ),
        })
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(false));
}

#[test]
fn player_set_viewpoint_remove_mismatch_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240401);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424040, 4240402);
    let existing_guid = guid(HighGuid::Creature, 4240409);
    player.set_farsight_object_like_cpp(existing_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, false, None);

    assert_eq!(
        outcome.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert_eq!(outcome.set_world_object, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);
}

#[test]
fn player_set_viewpoint_vehicle_base_skips_unit_shared_vision_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4240501);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424050, 4240502);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = map.apply_player_set_viewpoint_unit_like_cpp(
        player_guid,
        target_guid,
        true,
        Some(target_guid),
    );

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Applied);
    assert!(outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(outcome.set_world_object, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        target_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);
}

#[test]
fn player_remove_from_world_viewpoint_dynamic_object_cleans_before_extract_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4930101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4930102);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(player_guid, false).unwrap();

    let cleanup = removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(cleanup.player_guid, player_guid);
    assert_eq!(cleanup.viewpoint_guid, dynamic_object_guid);
    assert_eq!(
        cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert!(!cleanup.update_visibility_requested);
    assert!(cleanup.set_seer_requested);
    assert!(!cleanup.object_accessor_fanout_represented);
    assert_eq!(cleanup.dynamic_object_caster_viewpoint, None);
    let player_set_viewpoint = cleanup.player_set_viewpoint.unwrap();
    assert_eq!(player_set_viewpoint.player_guid, player_guid);
    assert_eq!(player_set_viewpoint.target_guid, dynamic_object_guid);
    assert!(!player_set_viewpoint.apply);
    assert_eq!(
        player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!player_set_viewpoint.update_visibility_requested);
    assert!(player_set_viewpoint.set_seer_requested);
    assert!(map.map_object_record(player_guid).is_none());
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}

#[test]
fn player_remove_from_world_viewpoint_dynamic_object_ignores_bound_caster_like_cpp() {
    let mut missing_caster_map = test_map();
    let mut missing_caster_player = test_player_for_viewpoint(4930111);
    let missing_caster_player_guid = missing_caster_player.guid();
    let mut missing_caster_dynamic_object = test_dynamic_object_for_viewpoint(4930112);
    let missing_caster_dynamic_object_guid = missing_caster_dynamic_object.world().guid();
    missing_caster_player.set_farsight_object_like_cpp(missing_caster_dynamic_object_guid);
    missing_caster_dynamic_object.set_caster_viewpoint();
    missing_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(missing_caster_player).unwrap())
        .unwrap();
    missing_caster_map
        .insert_map_object_record(
            MapObjectRecord::new_dynamic_object(missing_caster_dynamic_object).unwrap(),
        )
        .unwrap();

    let missing_caster_removed = missing_caster_map
        .remove_from_map_like_cpp(missing_caster_player_guid, false)
        .unwrap();

    let missing_caster_cleanup = missing_caster_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        missing_caster_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert_eq!(missing_caster_cleanup.dynamic_object_caster_viewpoint, None);
    let missing_caster_set_viewpoint = missing_caster_cleanup.player_set_viewpoint.unwrap();
    assert_eq!(
        missing_caster_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(missing_caster_set_viewpoint.set_seer_requested);
    assert!(
        missing_caster_map
            .get_typed_dynamic_object(missing_caster_dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );

    let mut other_caster_map = test_map();
    let mut removed_player = test_player_for_viewpoint(4930121);
    let removed_player_guid = removed_player.guid();
    let mut other_player = test_player_for_viewpoint(4930122);
    let other_player_guid = other_player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4930123);
    let dynamic_object_guid = dynamic_object.world().guid();
    removed_player.set_farsight_object_like_cpp(dynamic_object_guid);
    other_player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(other_player_guid);
    dynamic_object.bind_to_caster(other_player_guid);
    dynamic_object.set_caster_viewpoint();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(removed_player).unwrap())
        .unwrap();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(other_player).unwrap())
        .unwrap();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let other_caster_removed = other_caster_map
        .remove_from_map_like_cpp(removed_player_guid, false)
        .unwrap();

    let other_caster_cleanup = other_caster_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        other_caster_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert_eq!(other_caster_cleanup.dynamic_object_caster_viewpoint, None);
    let other_caster_set_viewpoint = other_caster_cleanup.player_set_viewpoint.unwrap();
    assert_eq!(other_caster_set_viewpoint.player_guid, removed_player_guid);
    assert_eq!(
        other_caster_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(other_caster_set_viewpoint.set_seer_requested);
    assert_eq!(
        other_caster_map
            .get_typed_player(other_player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        other_caster_map
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}

#[test]
fn player_remove_from_world_viewpoint_creature_consumes_shared_vision_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4930201);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 493020, 4930202);
    player.set_farsight_object_like_cpp(target_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.get_typed_creature_mut(target_guid)
        .unwrap()
        .unit_mut()
        .add_player_to_vision_like_cpp(player_guid);

    let removed = map.remove_from_map_like_cpp(player_guid, false).unwrap();

    let cleanup = removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedUnitViewpoint
    );
    let player_set_viewpoint = cleanup.player_set_viewpoint.unwrap();
    assert_eq!(
        player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!player_set_viewpoint.update_visibility_requested);
    assert!(player_set_viewpoint.set_seer_requested);
    assert_eq!(
        player_set_viewpoint.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: false,
            status: SetWorldObjectStatusLikeCpp::Delegated(
                AddObjectToSwitchListStatusLikeCpp::Queued
            ),
        })
    );
    assert!(map.map_object_record(player_guid).is_none());
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(false));
    assert!(removed.object.unwrap().guid() == player_guid);
}

#[test]
fn player_remove_from_world_viewpoint_missing_or_unsupported_target_no_cleanup_success_like_cpp() {
    let mut missing_map = test_map();
    let mut missing_player = test_player_for_viewpoint(4930301);
    let missing_player_guid = missing_player.guid();
    let missing_viewpoint_guid = guid(HighGuid::Creature, 4930302);
    missing_player.set_farsight_object_like_cpp(missing_viewpoint_guid);
    missing_map
        .insert_map_object_record(MapObjectRecord::new_player(missing_player).unwrap())
        .unwrap();

    let missing_removed = missing_map
        .remove_from_map_like_cpp(missing_player_guid, false)
        .unwrap();

    let missing_cleanup = missing_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        missing_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::MissingTarget
    );
    assert_eq!(missing_cleanup.player_set_viewpoint, None);
    assert_eq!(missing_cleanup.dynamic_object_caster_viewpoint, None);
    assert!(missing_map.map_object_record(missing_player_guid).is_none());

    let mut unsupported_map = test_map();
    let mut unsupported_player = test_player_for_viewpoint(4930401);
    let unsupported_player_guid = unsupported_player.guid();
    let game_object = game_object_with_counter(4930402, 571, 7, true);
    let game_object_guid = game_object.world().guid();
    unsupported_player.set_farsight_object_like_cpp(game_object_guid);
    unsupported_map
        .insert_map_object_record(MapObjectRecord::new_player(unsupported_player).unwrap())
        .unwrap();
    unsupported_map
        .insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    let unsupported_removed = unsupported_map
        .remove_from_map_like_cpp(unsupported_player_guid, false)
        .unwrap();

    let unsupported_cleanup = unsupported_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        unsupported_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotSeer
    );
    assert_eq!(unsupported_cleanup.player_set_viewpoint, None);
    assert_eq!(unsupported_cleanup.dynamic_object_caster_viewpoint, None);
    assert!(
        unsupported_map
            .map_object_record(unsupported_player_guid)
            .is_none()
    );
    assert!(
        unsupported_map
            .map_object_record(game_object_guid)
            .is_some()
    );
}

#[test]
fn player_remove_from_world_not_in_world_or_empty_farsight_emits_no_cleanup_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_player = test_player_for_viewpoint(4930501);
    let not_in_world_player_guid = not_in_world_player.guid();
    not_in_world_player.set_farsight_object_like_cpp(guid(HighGuid::Creature, 4930502));
    not_in_world_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(MapObjectRecord::new_player(not_in_world_player).unwrap())
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_player_guid, false)
        .unwrap();

    assert_eq!(not_in_world_removed.player_viewpoint_cleanup, None);

    let mut empty_map = test_map();
    let empty_player = test_player_for_viewpoint(4930601);
    let empty_player_guid = empty_player.guid();
    empty_map
        .insert_map_object_record(MapObjectRecord::new_player(empty_player).unwrap())
        .unwrap();

    let empty_removed = empty_map
        .remove_from_map_like_cpp(empty_player_guid, false)
        .unwrap();

    assert_eq!(empty_removed.player_viewpoint_cleanup, None);
}

#[test]
fn unit_shared_vision_set_world_object_request_enqueues_like_cpp() {
    let mut map = test_map();
    let spawn_id = 423010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4230101);

    let outcome = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: guid,
            on: true,
        },
    );

    assert_eq!(outcome.guid, guid);
    assert_eq!(outcome.on, true);
    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(!local_cell.grid_objects.creatures.contains(&guid));
    assert!(local_cell.world_objects.creatures.contains(&guid));
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn unit_shared_vision_set_world_object_request_opposite_toggle_cancels_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 423020, 4230201);

    assert_eq!(
        map.apply_unit_shared_vision_set_world_object_request_like_cpp(
            UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid: guid,
                on: true,
            },
        )
        .status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(
        map.apply_unit_shared_vision_set_world_object_request_like_cpp(
            UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid: guid,
                on: false,
            },
        )
        .status,
        SetWorldObjectStatusLikeCpp::Delegated(
            AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
        )
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    assert_eq!(drain.switch_executed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn unit_shared_vision_set_world_object_request_uses_existing_fallbacks_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 4230301);

    let missing = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: missing_guid,
            on: true,
        },
    );

    assert_eq!(missing.status, SetWorldObjectStatusLikeCpp::MissingOrStale);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);

    let gameobject = test_gameobject_for_spawn(423030, 4230302);
    let gameobject_guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let non_unit = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: gameobject_guid,
            on: true,
        },
    );

    assert_eq!(
        non_unit.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn set_world_object_like_cpp_creature_in_world_enqueues_and_drain_executes() {
    let mut map = test_map();
    let spawn_id = 421010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4210101);

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.guid, guid);
    assert_eq!(outcome.on, true);
    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    assert!(
        local_cell_for_switch(&map, grid, cell)
            .grid_objects
            .creatures
            .contains(&guid)
    );

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(!local_cell.grid_objects.creatures.contains(&guid));
    assert!(local_cell.world_objects.creatures.contains(&guid));
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn set_world_object_like_cpp_creature_not_in_world_does_not_enqueue_or_mutate() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(421020, 4210201, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.status, SetWorldObjectStatusLikeCpp::NotInWorld);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn set_world_object_like_cpp_non_unit_in_world_uses_ignored_outcome_without_queue() {
    let mut map = test_map();
    let gameobject = test_gameobject_for_spawn(421030, 4210301);
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn set_world_object_like_cpp_missing_stale_does_not_create_records() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4210401);

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.status, SetWorldObjectStatusLikeCpp::MissingOrStale);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn set_world_object_like_cpp_opposite_toggle_cancels_before_drain() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 421050, 4210501);

    assert_eq!(
        map.set_world_object_like_cpp(guid, true).status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    assert_eq!(
        map.set_world_object_like_cpp(guid, false).status,
        SetWorldObjectStatusLikeCpp::Delegated(
            AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
        )
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn switch_list_on_moves_creature_from_grid_to_world_container_like_cpp() {
    let mut map = test_map();
    let spawn_id = 420010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4200101);
    assert!(
        local_cell_for_switch(&map, grid, cell)
            .grid_objects
            .creatures
            .contains(&guid)
    );

    let queued = map.add_object_to_switch_list_like_cpp(guid, true);
    assert_eq!(queued.status, AddObjectToSwitchListStatusLikeCpp::Queued);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(!local_cell.grid_objects.creatures.contains(&guid));
    assert!(local_cell.world_objects.creatures.contains(&guid));
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn switch_list_off_moves_temp_creature_from_world_to_grid_container_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 420020, 4200201);
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.remove_all_objects_in_remove_list_like_cpp()
            .switch_executed,
        1
    );
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, false).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_executed, 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn switch_list_opposite_toggle_before_drain_cancels_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 420030, 4200301);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, false).status,
        AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn switch_list_duplicate_same_direction_reports_abort_outcome_like_cpp() {
    let mut map = test_map();
    let (guid, _, _) = add_loaded_grid_creature_for_switch(&mut map, 420040, 4200401);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::DuplicateSameDirectionAbort
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
}

#[test]
fn switch_list_non_unit_gameobject_enqueue_is_ignored_like_cpp() {
    let mut map = test_map();
    let gameobject = test_gameobject_for_spawn(420050, 4200501);
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.add_object_to_switch_list_like_cpp(guid, true);

    assert_eq!(
        outcome.status,
        AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}

#[test]
fn switch_list_stale_guid_drain_does_not_create_dummy_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4200601);
    map.enqueue_object_to_switch_for_test(guid, true);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_missing_or_stale, 1);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn switch_list_unloaded_grid_does_not_create_grid_and_drains_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(420070, 4200701, true);
    let guid = creature.guid();
    let cell = Cell::from_world(1.0, 2.0);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.get_ngrid(grid).is_none());

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_invalid_or_unloaded_grid, 1);
    assert!(map.get_ngrid(grid).is_none());
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}

#[test]
fn remove_list_drain_runs_switch_list_before_physical_remove_like_cpp() {
    let mut map = test_map();
    let spawn_id = 420080;
    let (guid, _, _) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4200801);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert!(map.add_object_to_remove_list_like_cpp(guid).queued);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
}

#[test]
fn despawn_all_by_spawn_id_queues_and_defers_physical_removal_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41905;
    let mut creature = test_creature_for_spawn(spawn_id, 4190501, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.despawn_all_by_spawn_id_like_cpp(SpawnObjectType::Creature, spawn_id);

    assert_eq!(outcome.queued, 1);
    assert_eq!(outcome.removed, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.removed, 1);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
}

#[test]
fn gameobject_update_just_deactivated_queues_linked_trap_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580101, 571, 7, false);
    let mut trap = game_object_with_counter(4580102, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_respawn_delay_time(0);
    owner.set_linked_trap_like_cpp(trap_guid);
    trap.set_loot_state(LootState::Ready, None);
    trap.set_go_state(GoState::Active);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.linked_trap_guid, Some(trap_guid));
    assert!(!outcome.linked_trap_removed);
    assert!(outcome.linked_trap_remove_queued);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(outcome.loot_cleared);
    let trap_after_update = map
        .map_object_record(trap_guid)
        .and_then(MapObjectRecord::game_object)
        .expect("linked trap should stay in map until remove-list drain");
    assert_eq!(trap_after_update.loot_state(), LootState::NotReady);
    assert_eq!(trap_after_update.data().state, GoState::Ready as i8);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.removed, 1);
    assert!(map.map_object_record(trap_guid).is_none());
}

#[test]
fn gameobject_update_just_deactivated_clears_owned_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4590101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let personal_guid = guid(HighGuid::Player, 4590191);
    let unique_guid = guid(HighGuid::Player, 4590192);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(7, 2));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(11, 3));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));
    gameobject.add_use_like_cpp();
    assert!(gameobject.shared_loot_like_cpp().is_some());
    assert_eq!(gameobject.personal_loot_count_like_cpp(), 1);
    assert_eq!(gameobject.unique_user_count_like_cpp(), 1);
    assert_eq!(gameobject.use_times(), 2);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.generic_not_ready);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}

#[test]
fn gameobject_update_just_deactivated_goober_spell_represents_casts_and_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let personal_guid = guid(HighGuid::Player, 4600191);
    let first_unique_guid = guid(HighGuid::Player, 4600192);
    let second_unique_guid = guid(HighGuid::Player, 4600193);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        spell_id: 12345,
        ..GooberUseSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(17, 4));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(19, 5));
    assert!(gameobject.add_unique_use_like_cpp(first_unique_guid));
    assert!(gameobject.add_unique_use_like_cpp(second_unique_guid));
    gameobject.add_use_like_cpp();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, Some(12345));
    assert_eq!(outcome.goober_spell_casts_represented, 2);
    assert!(outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(!outcome.goober_nodespawn_return);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_set_ready);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}

#[test]
fn gameobject_update_just_deactivated_goober_lock_resets_state_and_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        lock_id: 77,
        ..GooberUseSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(23, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.goober_state_reset);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert!(outcome.loot_cleared);
    assert!(canonical.shared_loot_like_cpp().is_none());
}

#[test]
fn gameobject_update_just_deactivated_goober_nodespawn_returns_before_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600391);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_flags(gameobject.data().flags | GO_FLAG_NODESPAWN);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        spell_id: 23456,
        auto_close_ms: 5000,
        ..GooberUseSource::default()
    }));
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(29, 2));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, Some(23456));
    assert_eq!(outcome.goober_spell_casts_represented, 1);
    assert!(outcome.goober_users_cleared);
    assert!(outcome.goober_state_reset);
    assert!(outcome.goober_nodespawn_return);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}

#[test]
fn gameobject_update_just_deactivated_goober_nodespawn_without_source_returns_before_clearloot_like_cpp()
 {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600351, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600352);
    let personal_guid = guid(HighGuid::Player, 4600353);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_flags(gameobject.data().flags | GO_FLAG_NODESPAWN);
    gameobject.set_represented_goober_use_source_like_cpp(None);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(30, 2));
    gameobject.set_personal_loot_like_cpp(personal_guid, GameObjectOwnedLoot::new(31, 1));
    gameobject.add_use_like_cpp();
    gameobject.add_use_like_cpp();
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));
    let use_times_before = gameobject.use_times();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, None);
    assert_eq!(outcome.goober_spell_casts_represented, 0);
    assert!(!outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(outcome.goober_nodespawn_return);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(canonical.data().state, GoState::Active as i8);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 1);
    assert_eq!(canonical.unique_user_count_like_cpp(), 1);
    assert_eq!(canonical.use_times(), use_times_before);
}

#[test]
fn gameobject_update_just_deactivated_goober_without_spell_clears_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4600401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    let unique_guid = guid(HighGuid::Player, 4600491);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource::default()));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(31, 3));
    assert!(gameobject.add_unique_use_like_cpp(unique_guid));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.goober_spell_cast_spell_id, None);
    assert_eq!(outcome.goober_spell_casts_represented, 0);
    assert!(!outcome.goober_users_cleared);
    assert!(!outcome.goober_state_reset);
    assert!(!outcome.goober_nodespawn_return);
    assert!(outcome.loot_cleared);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(canonical.unique_user_count_like_cpp(), 0);
    assert_eq!(canonical.use_times(), 0);
}

#[test]
fn gameobject_update_non_consumed_chest_restock_returns_after_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 45,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(41, 2));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_restock_armed);
    assert!(!outcome.non_consumed_set_ready);
    assert!(outcome.non_consumed_update_visibility_represented);
    assert!(outcome.non_consumed_update_dynamic_flags_represented);
    assert!(!outcome.non_consumed_source_missing);
    assert_eq!(canonical.restock_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_non_consumed_chest_without_restock_sets_ready_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 0,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(43, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(outcome.non_consumed_set_ready);
    assert!(outcome.non_consumed_update_visibility_represented);
    assert!(!outcome.non_consumed_update_dynamic_flags_represented);
    assert_eq!(canonical.restock_time(), 0);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_non_consumed_goober_sets_ready_after_prebranch_and_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    gameobject.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        consumable: false,
        lock_id: 88,
        ..GooberUseSource::default()
    }));
    gameobject.set_go_state(GoState::Active);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(47, 1));

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.goober_state_reset);
    assert!(outcome.loot_cleared);
    assert!(outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.non_consumed_set_ready);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_consumable_chest_or_goober_does_not_take_non_consumed_return_like_cpp() {
    let mut map = test_map();
    let mut chest = game_object_with_counter(4610401, 571, 7, false);
    let mut goober = game_object_with_counter(4610402, 571, 7, false);
    let chest_guid = chest.world().guid();
    let goober_guid = goober.world().guid();
    chest.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    chest.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 90,
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    chest.set_loot_state(LootState::JustDeactivated, None);
    goober.set_go_type(GAMEOBJECT_TYPE_GOOBER as u8);
    goober.set_represented_goober_use_source_like_cpp(Some(GooberUseSource {
        consumable: true,
        ..GooberUseSource::default()
    }));
    goober.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(chest).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(goober).unwrap())
        .unwrap();

    let chest_outcome = map.update_game_object_like_cpp(chest_guid, 1, 1_000);
    let goober_outcome = map.update_game_object_like_cpp(goober_guid, 1, 1_000);

    assert!(!chest_outcome.non_consumed_chest_or_goober_return);
    assert!(!chest_outcome.non_consumed_restock_armed);
    assert!(!chest_outcome.non_consumed_set_ready);
    assert!(!goober_outcome.non_consumed_chest_or_goober_return);
    assert!(!goober_outcome.non_consumed_set_ready);
}

#[test]
fn gameobject_update_spell_created_expired_deletes_after_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610501, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_go_state(GoState::Active);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 30,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_spell_id(123);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.loot_cleared);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.summoned_expired_despawn_represented);
    assert!(outcome.summoned_expired_go_state_ready);
    assert!(outcome.remove_list.as_ref().is_some_and(|list| list.queued));
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(canonical.data().state, GoState::Ready as i8);
    assert_eq!(canonical.respawn_time(), 0);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(!outcome.non_consumed_set_ready);
    assert!(!outcome.non_consumed_update_visibility_represented);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_owner_created_expired_deletes_after_clearloot_like_cpp() {
    let mut map = test_map();
    let owner_guid = guid(HighGuid::Player, 4620191);
    let mut gameobject = game_object_with_counter(4620101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 30,
        chest_consumable: false,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_created_by(owner_guid);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.loot_cleared);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.remove_list.as_ref().is_some_and(|list| list.queued));
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_new_flag_drop_owner_new_flag_command_is_represented_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4620201, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG as u8);
    let mut drop = game_object_with_counter(4620202, 571, 7, false);
    let drop_guid = drop.world().guid();
    drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    drop.set_created_by(owner_guid);
    drop.set_respawn_time(0);
    drop.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(drop).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(drop_guid, 1, 1_000);

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.new_flag_drop_owner_in_base_command_represented);
    assert!(!outcome.new_flag_drop_owner_missing_or_empty);
    assert!(!outcome.new_flag_drop_owner_wrong_kind);
    assert!(!outcome.new_flag_drop_owner_not_new_flag);
    assert!(map.map_object_record(owner_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_new_flag_drop_missing_and_wrong_owner_are_explicit_noops_like_cpp() {
    let mut missing_map = test_map();
    let mut missing_drop = game_object_with_counter(4620301, 571, 7, false);
    let missing_drop_guid = missing_drop.world().guid();
    missing_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    missing_drop.set_created_by(guid(HighGuid::GameObject, 4620399));
    missing_drop.set_respawn_time(0);
    missing_drop.set_loot_state(LootState::JustDeactivated, None);
    missing_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(missing_drop).unwrap(),
        )
        .unwrap();

    let missing_outcome = missing_map.update_game_object_like_cpp(missing_drop_guid, 1, 1_000);

    assert!(missing_outcome.summoned_expired_delete);
    assert!(missing_outcome.new_flag_drop_owner_missing_or_empty);
    assert!(!missing_outcome.new_flag_drop_owner_in_base_command_represented);

    let mut wrong_kind_map = test_map();
    let creature = test_creature_for_spawn(4620401, 4620401, true);
    let creature_guid = creature.guid();
    let mut wrong_kind_drop = game_object_with_counter(4620402, 571, 7, false);
    let wrong_kind_drop_guid = wrong_kind_drop.world().guid();
    wrong_kind_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    wrong_kind_drop.set_created_by(creature_guid);
    wrong_kind_drop.set_respawn_time(0);
    wrong_kind_drop.set_loot_state(LootState::JustDeactivated, None);
    wrong_kind_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    wrong_kind_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(wrong_kind_drop).unwrap(),
        )
        .unwrap();

    let wrong_kind_outcome =
        wrong_kind_map.update_game_object_like_cpp(wrong_kind_drop_guid, 1, 1_000);

    assert!(wrong_kind_outcome.summoned_expired_delete);
    assert!(wrong_kind_outcome.new_flag_drop_owner_wrong_kind);
    assert!(!wrong_kind_outcome.new_flag_drop_owner_in_base_command_represented);

    let mut not_new_flag_map = test_map();
    let mut owner = game_object_with_counter(4620501, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    let mut not_new_flag_drop = game_object_with_counter(4620502, 571, 7, false);
    let not_new_flag_drop_guid = not_new_flag_drop.world().guid();
    not_new_flag_drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    not_new_flag_drop.set_created_by(owner_guid);
    not_new_flag_drop.set_respawn_time(0);
    not_new_flag_drop.set_loot_state(LootState::JustDeactivated, None);
    not_new_flag_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    not_new_flag_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(not_new_flag_drop).unwrap(),
        )
        .unwrap();

    let not_new_flag_outcome =
        not_new_flag_map.update_game_object_like_cpp(not_new_flag_drop_guid, 1, 1_000);

    assert!(not_new_flag_outcome.summoned_expired_delete);
    assert!(not_new_flag_outcome.new_flag_drop_owner_not_new_flag);
    assert!(!not_new_flag_outcome.new_flag_drop_owner_in_base_command_represented);
}

#[test]
fn gameobject_update_owner_or_spell_with_future_respawn_does_not_delete_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4620601, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_created_by(guid(HighGuid::Player, 4620691));
    gameobject.set_spell_id(456);
    gameobject.set_respawn_time(60);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(!outcome.summoned_expired_delete);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert_eq!(canonical.respawn_time(), 60);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_generic_spawned_default_noncompat_schedules_respawn_and_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190001);
    gameobject.set_spawn_id(4640101);
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(45);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    let respawn_info = map
        .get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640101)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(1_045));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(
        outcome.generic_respawn_timer_add,
        Some(AddRespawnInfoOutcomeLikeCpp::Inserted)
    );
    assert!(!outcome.generic_respawn_compatibility_db_only_represented);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(respawn_info.object_type, SpawnObjectType::GameObject);
    assert_eq!(respawn_info.spawn_id, 4640101);
    assert_eq!(respawn_info.entry, 190001);
    assert_eq!(respawn_info.respawn_time, 1_045);
    assert_eq!(respawn_info.grid_id, compute_grid_coord(1.0, 2.0).get_id());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_with_pool_metadata_uses_delete_pool_branch_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640110, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640111, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640112, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640112, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640111, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640111, 464)
        .expect("test pool relation");

    let mut gameobject = game_object_with_counter(4640111, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_011);
    gameobject.set_spawn_id(4640111);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_created_by(guid(HighGuid::Player, 4640199));
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640111, 464)
        .expect("trigger spawned in pool");

    let outcome = map.update_game_object_with_pool_update_like_cpp(
        gameobject_guid,
        1,
        1_000,
        &store,
        &pool_mgr,
    );

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated
    );
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.summoned_expired_despawn_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(gameobject_guid).is_none());
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640112)
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(464),
        1
    );
}

#[test]
fn gameobject_delete_pool_update_loaded_grid_loader_adds_replacement_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640120, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640121, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640122, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640122, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640121, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640121, 464)
        .expect("test pool relation");

    map.ensure_grid_loaded(&cell_from_world(1.0, 2.0));
    let mut gameobject = game_object_with_counter(4640121, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_121);
    gameobject.set_spawn_id(4640121);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640121, 464)
        .expect("trigger spawned in pool");

    let replacement_guid = guid(HighGuid::GameObject, 4640122);
    let mut loader_calls = 0usize;
    let outcome = map
        .gameobject_delete_with_pool_update_loaded_grid_records_like_cpp(
            gameobject_guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, object_type, spawn_id| {
                loader_calls += 1;
                assert_eq!(object_type, SpawnObjectType::GameObject);
                assert_eq!(spawn_id, 4640122);
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 4640122))
                        .unwrap(),
                ))
            },
        )
        .expect("pool-aware delete outcome");

    assert_eq!(loader_calls, 1);
    assert!(outcome.pool_update_represented);
    let summary = outcome.pool_update_summary.expect("pool update summary");
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_action_load_plans, vec![]);
    assert!(map.map_object_record(replacement_guid).is_some());
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(4640122), 1);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640122)
    );
}

#[test]
fn gameobject_update_pool_update_loaded_grid_loader_adds_replacement_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640130, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640131, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640132, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640132, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640131, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640131, 464)
        .expect("test pool relation");

    map.ensure_grid_loaded(&cell_from_world(1.0, 2.0));
    let mut gameobject = game_object_with_counter(4640131, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_131);
    gameobject.set_spawn_id(4640131);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_created_by(guid(HighGuid::Player, 4640199));
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640131, 464)
        .expect("trigger spawned in pool");

    let replacement_guid = guid(HighGuid::GameObject, 4640132);
    let mut loader_calls = 0usize;
    let outcome = map.update_game_object_with_pool_update_loaded_grid_records_like_cpp(
        gameobject_guid,
        1,
        1_000,
        &store,
        &pool_mgr,
        |_, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 4640132);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 4640132))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated
    );
    assert_eq!(loader_calls, 1);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(gameobject_guid).is_none());
    assert!(map.map_object_record(replacement_guid).is_some());
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640132)
    );
}

#[test]
fn gameobject_update_generic_spawned_default_compat_saves_db_only_and_visibility_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190002);
    gameobject.set_spawn_id(4640201);
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_delay_time(30);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 2_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(2_030));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_respawn_compatibility_db_only_represented);
    assert!(outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(canonical.respawn_time(), 2_030);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640201)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_visibility_on_destroy_update_summary_carries_guids_without_truncation_like_cpp() {
    let mut map = test_map();
    let mut expected_guids = Vec::new();

    for offset in 0..300 {
        let counter = 5_050_101_i64 + i64::from(offset);
        let mut gameobject = game_object_with_counter(counter, 571, 7, false);
        let gameobject_guid = gameobject.world().guid();
        gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
        gameobject.world_mut().object_mut().set_entry(190_505);
        gameobject.set_spawn_id(u64::try_from(counter).unwrap());
        gameobject.set_spawned_by_default(true);
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_loot_state(LootState::JustDeactivated, None);

        expected_guids.push(gameobject_guid);
        map.add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    }

    let summary = map.update_game_objects_like_cpp(1, 2_000);

    assert_eq!(summary.generic_visibility_on_destroy_represented, 300);
    assert_eq!(
        summary.generic_respawn_compatibility_db_only_represented,
        300
    );
    assert_eq!(summary.respawn_db_saves.len(), 300);
    assert_eq!(
        summary.respawn_db_saves[256].object_type,
        SpawnObjectType::GameObject
    );
    assert_eq!(summary.respawn_db_saves[256].respawn_time, 2_030);
    let carried_guids = summary.generic_visibility_on_destroy_guids.as_slice();
    assert_eq!(carried_guids.len(), 300);
    assert!(carried_guids.contains(&expected_guids[256]));
    assert!(carried_guids.contains(&expected_guids[299]));
    let carried_set = carried_guids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let expected_set = expected_guids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(carried_set, expected_set);
    assert_eq!(
        summary.generic_respawn_compatibility_db_only_represented,
        300
    );
    assert_eq!(summary.despawn_remove_queued, 0);
    for guid in expected_guids {
        assert!(map.map_object_record(guid).is_some());
    }
}

#[test]
fn gameobject_update_generic_spawned_default_noncompat_missing_godata_does_not_insert_respawn_like_cpp()
 {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640251, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190003);
    gameobject.set_spawn_id(4640251);
    gameobject.set_spawned_by_default(true);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(25);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 2_500);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(2_525));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_respawn_save_missing_gameobject_data);
    assert!(!outcome.generic_respawn_save_missing_spawn_id);
    assert!(!outcome.generic_respawn_compatibility_db_only_represented);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 2_525);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640251)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_generic_temporary_noncompat_spawn_id_visibility_no_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_spawn_id(4640301);
    gameobject.set_spawned_by_default(false);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(60);
    gameobject.set_respawn_time(999);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 3_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, None);
    assert!(!outcome.generic_spawned_by_default_branch);
    assert!(outcome.generic_temporary_respawn_zeroed);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(canonical.respawn_time(), 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640301)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_generic_temporary_zero_spawn_id_deletes_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640351, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_spawn_id(0);
    gameobject.set_spawned_by_default(false);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(60);
    gameobject.set_respawn_time(999);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 3_100);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_temporary_respawn_zeroed);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn gameobject_update_generic_zero_respawn_sets_not_ready_without_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_zero_respawn_delay_return);
    assert!(!outcome.generic_visual_despawn_represented);
    assert!(!outcome.generic_flags_restored_represented);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_generic_chest_consumable_visual_despawn_restores_flags_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x10));
    gameobject.set_flags(0x90);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert!(outcome.generic_flags_restored_represented);
    assert!(!outcome.generic_despawn_at_action_source_missing);
    assert_eq!(canonical.data().flags, 0x10);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}

#[test]
fn gameobject_update_generic_anim_progress_visual_despawn_without_despawn_source_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_go_anim_progress_like_cpp(1);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x04));
    gameobject.set_flags(0x84);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert!(outcome.generic_flags_restored_represented);
    assert!(!outcome.generic_despawn_at_action_source_missing);
    assert_eq!(canonical.data().flags, 0x04);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
}

#[test]
fn gameobject_update_generic_chest_missing_source_does_not_assume_despawn_at_action_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(None);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.non_consumed_source_missing);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_despawn_at_action_source_missing);
    assert!(!outcome.generic_visual_despawn_represented);
    assert!(!outcome.generic_flags_restored_represented);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
}

#[test]
fn gameobject_update_summary_counts_generic_branch_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630501, 571, 7, false);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_go_anim_progress_like_cpp(3);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
    gameobject.set_flags(0x88);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    let gameobject_guid = gameobject.world().guid();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.generic_not_ready, 1);
    assert_eq!(summary.generic_visual_despawn_represented, 1);
    assert_eq!(
        summary.generic_visual_despawn_guids.as_slice(),
        &[gameobject_guid]
    );
    assert_eq!(summary.generic_flags_restored_represented, 1);
    assert_eq!(summary.generic_zero_respawn_delay_returns, 1);
    assert_eq!(summary.despawn_remove_queued, 0);
}

#[test]
fn gameobject_visual_despawn_summary_guids_are_not_truncated_like_cpp() {
    let mut map = test_map();
    let mut expected_guids = Vec::new();
    for index in 0..300 {
        let mut gameobject = game_object_with_counter(4630601 + index, 571, 7, false);
        gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
        gameobject.set_go_anim_progress_like_cpp(1);
        gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
        gameobject.set_flags(0x88);
        gameobject.set_respawn_delay_time(0);
        gameobject.set_loot_state(LootState::JustDeactivated, None);
        expected_guids.push(gameobject.world().guid());
        map.add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    }

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.generic_visual_despawn_represented, 300);
    assert_eq!(summary.generic_visual_despawn_guids.as_slice().len(), 300);
    let mut actual_guids = summary.generic_visual_despawn_guids.as_slice().to_vec();
    actual_guids.sort_by_key(|guid| guid.counter());
    expected_guids.sort_by_key(|guid| guid.counter());
    assert_eq!(actual_guids, expected_guids);
}

#[test]
fn gameobject_update_summary_counts_summoned_expired_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4620701, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG as u8);
    let mut drop = game_object_with_counter(4620702, 571, 7, false);
    drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    drop.set_created_by(owner_guid);
    drop.set_respawn_time(0);
    drop.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(drop).unwrap())
        .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.summoned_expired_deletes, 1);
    assert_eq!(summary.summoned_expired_respawn_time_zeroed, 1);
    assert_eq!(summary.summoned_expired_despawn_represented, 1);
    assert_eq!(summary.new_flag_drop_owner_in_base_commands_represented, 1);
    assert_eq!(summary.despawn_remove_queued, 1);
}

#[test]
fn gameobject_update_missing_template_source_does_not_assume_non_consumed_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610601, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(None);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);

    assert!(outcome.loot_cleared);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_set_ready);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(outcome.non_consumed_source_missing);
}

#[test]
fn gameobject_update_despawn_requested_does_not_consume_just_deactivated_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580501, 571, 7, false);
    let trap = game_object_with_counter(4580502, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(5, 1));
    owner.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590291),
        GameObjectOwnedLoot::new(6, 1),
    );
    let loot_authority = owner.loot_authority_like_cpp().clone();
    assert!(owner.add_unique_use_like_cpp(guid(HighGuid::Player, 4590292)));
    assert!(owner.schedule_despawn_or_unsummon_like_cpp(1, 0));

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert_eq!(outcome.linked_trap_guid, None);
    assert!(!outcome.linked_trap_removed);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let owner_after = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(owner_after.loot_state(), LootState::NotReady);
    assert!(owner_after.shared_loot_like_cpp().is_some());
    assert_eq!(owner_after.personal_loot_count_like_cpp(), 1);
    assert_eq!(owner_after.unique_user_count_like_cpp(), 1);
    assert_eq!(owner_after.use_times(), 1);
    assert!(loot_authority.is_retired_like_cpp());
    assert!(map.map_object_record(trap_guid).is_some());
}

#[test]
fn gameobject_update_non_just_deactivated_keeps_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580201, 571, 7, false);
    let trap = game_object_with_counter(4580202, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::Ready, None);
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(9, 1));
    owner.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590391),
        GameObjectOwnedLoot::new(10, 1),
    );
    assert!(owner.add_unique_use_like_cpp(guid(HighGuid::Player, 4590392)));

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.linked_trap_guid, None);
    assert!(!outcome.linked_trap_removed);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(!outcome.loot_cleared);
    let owner_after = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(owner_after.loot_state(), LootState::Ready);
    assert!(owner_after.shared_loot_like_cpp().is_some());
    assert_eq!(owner_after.personal_loot_count_like_cpp(), 1);
    assert_eq!(owner_after.unique_user_count_like_cpp(), 1);
    assert_eq!(owner_after.use_times(), 1);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
}

#[test]
fn gameobject_update_not_in_world_does_not_clear_owned_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4590401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(12, 1));
    gameobject.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590491),
        GameObjectOwnedLoot::new(13, 1),
    );
    assert!(gameobject.add_unique_use_like_cpp(guid(HighGuid::Player, 4590492)));

    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::NotInWorld);
    assert!(!outcome.loot_cleared);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 1);
    assert_eq!(canonical.unique_user_count_like_cpp(), 1);
    assert_eq!(canonical.use_times(), 1);
}

#[test]
fn gameobject_update_just_deactivated_empty_self_missing_trap_is_noop_like_cpp() {
    let mut map = test_map();
    let mut empty = game_object_with_counter(4580301, 571, 7, false);
    let mut self_linked = game_object_with_counter(4580302, 571, 7, false);
    let mut missing = game_object_with_counter(4580303, 571, 7, false);
    let unrelated = game_object_with_counter(4580304, 571, 7, false);
    let empty_guid = empty.world().guid();
    let self_guid = self_linked.world().guid();
    let missing_guid = missing.world().guid();
    let missing_trap_guid = guid(HighGuid::GameObject, 4580399);
    let unrelated_guid = unrelated.world().guid();
    empty.set_loot_state(LootState::JustDeactivated, None);
    self_linked.set_loot_state(LootState::JustDeactivated, None);
    self_linked.set_linked_trap_like_cpp(self_guid);
    missing.set_loot_state(LootState::JustDeactivated, None);
    missing.set_linked_trap_like_cpp(missing_trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(empty).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(self_linked).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(missing).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(unrelated).unwrap())
        .unwrap();

    let empty_outcome = map.update_game_object_like_cpp(empty_guid, 1, 1_000);
    let self_outcome = map.update_game_object_like_cpp(self_guid, 1, 1_000);
    let missing_outcome = map.update_game_object_like_cpp(missing_guid, 1, 1_000);

    assert_eq!(empty_outcome.linked_trap_guid, None);
    assert!(!empty_outcome.linked_trap_removed);
    assert!(empty_outcome.linked_trap_missing_or_self);
    assert_eq!(self_outcome.linked_trap_guid, Some(self_guid));
    assert!(!self_outcome.linked_trap_removed);
    assert!(self_outcome.linked_trap_missing_or_self);
    assert_eq!(missing_outcome.linked_trap_guid, Some(missing_trap_guid));
    assert!(!missing_outcome.linked_trap_removed);
    assert!(missing_outcome.linked_trap_missing_or_self);
    assert!(map.map_object_record(empty_guid).is_some());
    assert!(map.map_object_record(self_guid).is_some());
    assert!(map.map_object_record(missing_guid).is_some());
    assert!(map.map_object_record(unrelated_guid).is_some());
}

#[test]
fn gameobject_update_summary_counts_linked_trap_remove_queue_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580401, 571, 7, false);
    let trap = game_object_with_counter(4580402, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_respawn_delay_time(0);
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.linked_traps_removed, 0);
    assert_eq!(summary.linked_traps_remove_queued, 1);
    assert_eq!(summary.loot_cleared, 1);
    assert!(summary.visited >= 1);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}

#[test]
fn remove_list_drain_gameobject_owner_removes_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4190601, 571, 7, false);
    let trap = game_object_with_counter(4190602, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    assert!(map.add_object_to_remove_list_like_cpp(owner_guid).queued);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 2);
    assert_eq!(outcome.removed, 2);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_none());
}

#[test]
fn linked_trap_remove_owner_removes_trap_map_local_and_leaves_unrelated_objects() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(10, 571, 7, false);
    let trap = game_object_with_counter(11, 571, 7, false);
    let unrelated = game_object_with_counter(12, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    let unrelated_guid = unrelated.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(unrelated).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(owner_guid, true).unwrap();

    assert_eq!(removed.guid, owner_guid);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(unrelated_guid).is_some());
}

#[test]
fn gameobject_delete_compatibility_pool_updates_pool_without_remove_list_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(531, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 53101, active.clone());
    trigger_spawn.pool_id = 55;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 53102, active);
    replacement_spawn.pool_id = 55;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(55, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53101, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53102, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 53101, 55)
        .expect("test pool relation");

    let mut gameobject = test_gameobject_for_spawn(53101, 5310101);
    let guid = gameobject.world().guid();
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_go_state(GoState::Active);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 53101, 55)
            .is_ok()
    );

    let outcome = map
        .gameobject_delete_with_pool_update_like_cpp(
            guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, _count| vec![1],
        )
        .expect("delete outcome");

    assert!(outcome.pool_update_represented);
    assert!(outcome.pool_update_error.is_none());
    assert!(outcome.pool_update_plan.is_some());
    assert!(outcome.pool_update_summary.is_some());
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let summary = outcome.pool_update_summary.as_ref().unwrap();
    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(53102)
    );
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(55), 1);
    assert!(map.map_object_record(guid).is_none());
}

#[test]
fn gameobject_delete_without_compatibility_pool_keeps_remove_list_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(532, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 53201, active);
    trigger_spawn.pool_id = 56;
    store.add_object_spawn(&trigger_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(56, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 56);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 56, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 53201, 56)
        .expect("test pool relation");

    let mut gameobject = test_gameobject_for_spawn(53201, 5320101);
    let guid = gameobject.world().guid();
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.replace_loot_authority_like_cpp(None, HashMap::new());
    let loot_authority = gameobject.loot_authority_like_cpp().clone();
    assert!(!loot_authority.is_retired_like_cpp());
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map
        .gameobject_delete_with_pool_update_like_cpp(
            guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
        .expect("delete outcome");

    assert!(!outcome.pool_update_represented);
    assert!(outcome.pool_update_plan.is_none());
    assert!(outcome.pool_update_summary.is_none());
    assert!(
        outcome
            .remove_list
            .as_ref()
            .is_some_and(|remove| remove.queued)
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(56), 0);
    assert!(
        loot_authority.is_retired_like_cpp(),
        "queued GameObject deletion must invalidate Arc-held loot claims"
    );
}

#[test]
fn remove_from_map_like_cpp_can_delete_object_and_reports_missing_guid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();
    assert!(removed.delete_from_world);
    assert!(removed.object.is_none());
    assert_eq!(map.map_object_count(), 0);

    assert_eq!(
        map.remove_from_map_like_cpp(guid, false),
        Err(RemoveFromMapError::ObjectNotFound { guid })
    );
}

#[test]
fn relocate_map_object_like_cpp_same_cell_only_updates_position() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(2.0, 3.0, 4.0))
        .unwrap();

    assert!(outcome.relocated);
    assert!(!outcome.moved_between_cells);
    assert_eq!(outcome.old_cell, added.cell);
    assert_eq!(outcome.new_cell, added.cell);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(2.0, 3.0, 4.0)
    );
}

#[test]
fn relocate_map_object_like_cpp_moves_between_cells_in_same_grid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    let new_position = Position::xyz(90.0, 20.0, 5.0);

    let outcome = map
        .relocate_map_object_like_cpp(guid, new_position)
        .unwrap();

    assert!(outcome.relocated);
    assert!(outcome.moved_between_cells);
    assert_eq!(outcome.old_grid, outcome.new_grid);
    assert_eq!(map.get_creature(guid).unwrap().position(), new_position);
    assert_eq!(
        map.get_creature(guid).unwrap().current_cell(),
        Some((
            outcome.new_cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.new_cell.y_coord % MAX_NUMBER_OF_CELLS
        ))
    );

    let old_grid = map.get_ngrid(added.grid).unwrap();
    let old_cell = old_grid
        .get_grid_type(
            added.cell.x_coord % MAX_NUMBER_OF_CELLS,
            added.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(!old_cell.grid_objects.creatures.contains(&guid));

    let new_cell = old_grid
        .get_grid_type(
            outcome.new_cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.new_cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(new_cell.grid_objects.creatures.contains(&guid));
}

#[test]
fn relocate_map_object_like_cpp_blocks_normal_object_to_unloaded_grid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0))
        .unwrap();

    assert!(!outcome.relocated);
    assert!(outcome.blocked_by_unloaded_grid);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
    let old_grid = map.get_ngrid(added.grid).unwrap();
    let old_cell = old_grid
        .get_grid_type(
            added.cell.x_coord % MAX_NUMBER_OF_CELLS,
            added.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(old_cell.grid_objects.creatures.contains(&guid));
}

#[test]
fn relocate_map_object_like_cpp_blocks_missing_target_grid_without_panicking() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(90.0, 20.0, 5.0))
        .unwrap();

    assert!(!outcome.relocated);
    assert!(outcome.blocked_by_unloaded_grid);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
}

#[test]
fn relocate_map_object_like_cpp_active_object_loads_new_grid_and_moves() {
    let mut map = test_map();
    let mut object = WorldObject::new(true, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
    object.object_mut().create(guid(HighGuid::DynamicObject, 3));
    object.set_map(571, 7).unwrap();
    object.relocate(Position::xyz(20.0, 20.0, 3.0));
    object.set_active(true);
    let guid = object.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::DynamicObject, object)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0))
        .unwrap();

    assert!(outcome.relocated);
    assert!(outcome.moved_between_cells);
    assert_ne!(outcome.old_grid, outcome.new_grid);
    assert!(outcome.loaded_grid);
    assert!(map.is_grid_loaded(outcome.new_grid));
    assert_eq!(
        map.get_dynamic_object(guid).unwrap().position(),
        Position::xyz(700.0, 20.0, 5.0)
    );
}

#[test]
fn nearby_cell_guids_like_cpp_visits_existing_cells_without_loading_grids() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object(HighGuid::GameObject, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();

    let nearby = map.nearby_cell_guids_like_cpp(0.0, 0.0, 70.0);

    assert_eq!(nearby.visited_cells, 16);
    assert_eq!(nearby.len(), 2);
    assert!(nearby.grid.creatures.contains(&creature_guid));
    assert!(nearby.grid.gameobjects.contains(&gameobject_guid));
    assert_eq!(map.terrain().loads.len(), 1);

    let far = map.nearby_cell_guids_like_cpp(700.0, 700.0, 0.0);
    assert_eq!(far.visited_cells, 1);
    assert!(far.is_empty());
    assert_eq!(map.terrain().loads.len(), 1);
}

#[test]
fn nearby_cell_guids_like_cpp_rejects_invalid_center_without_visits() {
    let map = test_map();
    let nearby = map.nearby_cell_guids_like_cpp(f32::NAN, 0.0, 100.0);

    assert_eq!(nearby.visited_cells, 0);
    assert!(nearby.is_empty());
}

#[test]
fn visit_nearby_cells_of_like_cpp_marks_cells_once_and_collects_objects() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let viewpoint_guid = viewpoint.guid();
    let creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let creature_guid = creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let plan = map.visit_nearby_cells_of_like_cpp([
        NearbyCellVisitCenter {
            guid: player_guid,
            activation_radius: 0.0,
        },
        NearbyCellVisitCenter {
            guid: viewpoint_guid,
            activation_radius: 0.0,
        },
    ]);

    assert_eq!(plan.marked_cells.len(), 1);
    assert_eq!(plan.nearby.visited_cells, 1);
    assert!(plan.nearby.world.players.contains(&player_guid));
    assert!(plan.nearby.grid.creatures.contains(&viewpoint_guid));
    assert!(plan.nearby.grid.creatures.contains(&creature_guid));
}

#[test]
fn visit_nearby_cells_of_like_cpp_skips_missing_and_invalid_centers() {
    let mut map = test_map();
    let mut invalid_center = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let invalid_guid = invalid_center.guid();
    invalid_center.relocate(Position::xyz(f32::NAN, 0.0, 0.0));
    map.insert_map_object(AccessorObjectKind::Player, invalid_center)
        .unwrap();
    let missing = guid(HighGuid::Player, 9);

    let plan = map.visit_nearby_cells_of_like_cpp([
        NearbyCellVisitCenter {
            guid: invalid_guid,
            activation_radius: 100.0,
        },
        NearbyCellVisitCenter {
            guid: missing,
            activation_radius: 100.0,
        },
    ]);

    assert!(plan.marked_cells.is_empty());
    assert!(plan.nearby.is_empty());
    assert_eq!(plan.skipped_invalid_position_centers, vec![invalid_guid]);
    assert_eq!(plan.skipped_missing_centers, vec![missing]);
}

#[test]
fn player_relocation_visibility_plan_matches_cpp_visible_and_out_of_range_shape() {
    let player = guid(HighGuid::Player, 1);
    let other_player = guid(HighGuid::Player, 2);
    let old_player = guid(HighGuid::Player, 3);
    let creature = guid(HighGuid::Creature, 4);
    let old_creature = guid(HighGuid::Creature, 5);
    let gameobject = guid(HighGuid::GameObject, 6);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.world.players.insert(other_player);
    nearby.grid.creatures.insert(creature);
    nearby.grid.gameobjects.insert(gameobject);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [other_player, old_player, old_creature],
        &nearby,
        true,
        [],
        [],
    );

    assert!(plan.visible_guids.contains(&player));
    assert!(plan.visible_guids.contains(&other_player));
    assert!(plan.visible_guids.contains(&creature));
    assert!(plan.visible_guids.contains(&gameobject));
    assert_eq!(
        plan.out_of_range_guids,
        HashSet::from([old_player, old_creature])
    );
    assert_eq!(
        plan.reciprocal_player_updates,
        HashSet::from([other_player, old_player])
    );
    assert_eq!(plan.ai_relocation_checks, vec![(creature, player)]);
}

#[test]
fn player_relocation_visibility_plan_skips_ai_when_not_relocated_for_ai() {
    let player = guid(HighGuid::Player, 1);
    let creature = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [creature],
        &nearby,
        false,
        [],
        [],
    );

    assert!(plan.out_of_range_guids.is_empty());
    assert!(plan.ai_relocation_checks.is_empty());
}

#[test]
fn player_relocation_visibility_plan_filters_targets_needing_cpp_notify() {
    let player = guid(HighGuid::Player, 4440201);
    let player_target_needs_notify = guid(HighGuid::Player, 4440202);
    let player_target_clear = guid(HighGuid::Player, 4440203);
    let old_player_needs_notify = guid(HighGuid::Player, 4440204);
    let old_player_clear = guid(HighGuid::Player, 4440205);
    let creature_needs_notify = guid(HighGuid::Creature, 4440206);
    let creature_clear = guid(HighGuid::Creature, 4440207);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.world.players.insert(player_target_needs_notify);
    nearby.world.players.insert(player_target_clear);
    nearby.grid.creatures.insert(creature_needs_notify);
    nearby.grid.creatures.insert(creature_clear);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [old_player_needs_notify, old_player_clear],
        &nearby,
        true,
        [player_target_needs_notify, old_player_needs_notify],
        [creature_needs_notify],
    );

    assert!(
        !plan
            .reciprocal_player_updates
            .contains(&player_target_needs_notify)
    );
    assert!(
        plan.reciprocal_player_updates
            .contains(&player_target_clear)
    );
    assert!(
        !plan
            .reciprocal_player_updates
            .contains(&old_player_needs_notify)
    );
    assert!(plan.reciprocal_player_updates.contains(&old_player_clear));
    assert_eq!(plan.ai_relocation_checks, vec![(creature_clear, player)]);
}

#[test]
fn creature_relocation_visibility_plan_matches_cpp_player_and_creature_visits() {
    let source = guid(HighGuid::Creature, 1);
    let player_visible = guid(HighGuid::Player, 2);
    let player_needs_notify = guid(HighGuid::Player, 3);
    let creature_normal = guid(HighGuid::Creature, 4);
    let creature_needs_notify = guid(HighGuid::Creature, 5);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player_visible);
    nearby.world.players.insert(player_needs_notify);
    nearby.grid.creatures.insert(source);
    nearby.grid.creatures.insert(creature_normal);
    nearby.grid.creatures.insert(creature_needs_notify);

    let plan = CreatureRelocationVisibilityPlan::from_nearby_like_cpp(
        source,
        true,
        &nearby,
        [player_needs_notify],
        [creature_needs_notify],
    );

    assert_eq!(
        plan.player_visibility_updates,
        HashSet::from([player_visible])
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, player_visible))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, player_needs_notify))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, creature_normal))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(creature_normal, source))
    );
    assert!(
        plan.ai_relocation_checks
            .contains(&(source, creature_needs_notify))
    );
    assert!(
        !plan
            .ai_relocation_checks
            .contains(&(creature_needs_notify, source))
    );
}

#[test]
fn creature_relocation_visibility_plan_skips_creature_visits_when_source_dead() {
    let source = guid(HighGuid::Creature, 1);
    let player = guid(HighGuid::Player, 2);
    let creature = guid(HighGuid::Creature, 3);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.grid.creatures.insert(creature);

    let plan =
        CreatureRelocationVisibilityPlan::from_nearby_like_cpp(source, false, &nearby, [], []);

    assert_eq!(plan.player_visibility_updates, HashSet::from([player]));
    assert_eq!(plan.ai_relocation_checks, vec![(source, player)]);
}

#[test]
fn delayed_unit_relocation_plan_selects_only_units_needing_notify_like_cpp() {
    let creature_notify = guid(HighGuid::Creature, 1);
    let creature_normal = guid(HighGuid::Creature, 2);
    let world_creature_notify = guid(HighGuid::Creature, 3);
    let player_notify = guid(HighGuid::Player, 4);
    let player_normal = guid(HighGuid::Player, 5);
    let player_invalid_viewpoint = guid(HighGuid::Player, 6);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature_notify);
    nearby.grid.creatures.insert(creature_normal);
    nearby.world.creatures.insert(world_creature_notify);
    nearby.world.players.insert(player_notify);
    nearby.world.players.insert(player_normal);
    nearby.world.players.insert(player_invalid_viewpoint);

    let plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(
        &nearby,
        [creature_notify, world_creature_notify],
        [player_notify, player_invalid_viewpoint],
        [player_invalid_viewpoint],
    );

    assert_eq!(
        plan.creature_relocations,
        vec![creature_notify, world_creature_notify]
    );
    assert_eq!(plan.player_relocations, vec![player_notify]);
    assert_eq!(
        plan.skipped_invalid_viewpoints,
        vec![player_invalid_viewpoint]
    );
}

#[test]
fn delayed_unit_relocation_plan_deduplicates_creatures_from_world_and_grid_sets() {
    let creature = guid(HighGuid::Creature, 1);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature);
    nearby.world.creatures.insert(creature);

    let plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(&nearby, [creature], [], []);

    assert_eq!(plan.creature_relocations, vec![creature]);
    assert!(plan.player_relocations.is_empty());
}

#[test]
fn delayed_unit_relocation_for_cells_like_cpp_reads_notify_flags_from_map_store() {
    let mut map = test_map();
    let creature_notify = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_notify_guid = creature_notify.guid();
    let creature_normal = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let player_notify = world_object_with_counter(HighGuid::Player, 3, 571, 7, false);
    let player_notify_guid = player_notify.guid();
    let player_invalid = world_object_with_counter(HighGuid::Player, 4, 571, 7, false);
    let player_invalid_guid = player_invalid.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature_notify)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature_normal)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_invalid)
        .unwrap();
    for guid in [
        creature_notify_guid,
        player_notify_guid,
        player_invalid_guid,
    ] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], [player_invalid_guid]);

    assert_eq!(plan.cell_plans.len(), 1);
    assert_eq!(plan.cell_plans[0].cell_coord, cell);
    assert_eq!(
        plan.cell_plans[0].plan.creature_relocations,
        vec![creature_notify_guid]
    );
    assert_eq!(
        plan.cell_plans[0].plan.player_relocations,
        vec![player_notify_guid, player_invalid_guid]
    );
    assert!(
        plan.cell_plans[0]
            .plan
            .skipped_invalid_viewpoints
            .is_empty()
    );
}

#[test]
fn delayed_unit_relocation_for_cells_uses_player_seer_notify_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4440101);
    let player_guid = player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 4440102, 571, 7, false);
    let viewpoint_guid = viewpoint.guid();
    player.set_farsight_object_like_cpp(viewpoint_guid);

    let cell = map
        .add_to_map_like_cpp(
            AccessorObjectKind::Player,
            world_object_with_counter(HighGuid::Player, 4440101, 571, 7, false),
        )
        .unwrap()
        .cell;
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    assert_eq!(plan.cell_plans.len(), 1);
    assert_eq!(
        plan.cell_plans[0].plan.player_relocations,
        vec![player_guid]
    );
    assert!(
        plan.cell_plans[0]
            .plan
            .skipped_invalid_viewpoints
            .is_empty()
    );

    let visibility_plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &plan,
        map.delayed_player_relocation_contexts_from_plan_like_cpp(&plan),
        [DelayedCreatureRelocationContext {
            creature_guid: viewpoint_guid,
            source_creature_alive: true,
        }],
    );
    assert_eq!(visibility_plans.player_plans.len(), 1);
    assert_eq!(visibility_plans.player_plans[0].player_guid, player_guid);
    assert_eq!(
        visibility_plans.player_plans[0].viewpoint_guid,
        viewpoint_guid
    );
    assert!(
        visibility_plans.player_plans[0]
            .visibility_plan
            .ai_relocation_checks
            .is_empty()
    );
    let creature_plan = visibility_plans
        .creature_plans
        .iter()
        .find(|plan| plan.creature_guid == viewpoint_guid)
        .unwrap();
    assert!(
        !creature_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_guid),
        "CreatureRelocationNotifier must test player->m_seer notify, not the Player notify flag"
    );

    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .relocate(Position::xyz(1.0e9, 1.0e9, 0.0));
    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let skipped = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    assert_eq!(skipped.cell_plans.len(), 1);
    assert!(skipped.cell_plans[0].plan.player_relocations.is_empty());
    assert_eq!(
        skipped.cell_plans[0].plan.skipped_invalid_viewpoints,
        vec![player_guid]
    );
}

#[test]
fn delayed_unit_relocation_visibility_plans_filter_player_seers_like_cpp() {
    let mut map = test_map();
    let source_player = world_object_with_counter(HighGuid::Player, 4440301, 571, 7, false);
    let source_player_guid = source_player.guid();
    let target_needs_notify = world_object_with_counter(HighGuid::Player, 4440302, 571, 7, false);
    let target_needs_notify_guid = target_needs_notify.guid();
    let target_clear = world_object_with_counter(HighGuid::Player, 4440303, 571, 7, false);
    let target_clear_guid = target_clear.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, source_player)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Player, target_needs_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, target_clear)
        .unwrap();
    for guid in [source_player_guid, target_needs_notify_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let visibility_plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        map.delayed_player_relocation_contexts_from_plan_like_cpp(&delayed_plan),
        std::iter::empty::<DelayedCreatureRelocationContext>(),
    );
    let source_plan = visibility_plans
        .player_plans
        .iter()
        .find(|plan| plan.player_guid == source_player_guid)
        .unwrap();

    assert!(
        !source_plan
            .visibility_plan
            .reciprocal_player_updates
            .contains(&target_needs_notify_guid)
    );
    assert!(
        source_plan
            .visibility_plan
            .reciprocal_player_updates
            .contains(&target_clear_guid)
    );
}

#[test]
fn process_relocation_notifies_like_cpp_selects_delayed_before_resetting_flags() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let player = world_object_with_counter(HighGuid::Player, 2, 571, 7, false);
    let player_guid = player.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap()
        .cell;
    let active_cell = Cell::from_cell_coord(cell);
    let active_grid = GridCoord::new(active_cell.grid_x(), active_cell.grid_y());
    map.get_ngrid_mut(active_grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();
    for guid in [creature_guid, player_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let outcome = map.process_relocation_notifies_like_cpp(
        [cell],
        1000,
        1000,
        std::iter::empty::<ObjectGuid>(),
    );

    assert_eq!(outcome.process_plan.delayed_relocation_cells, vec![cell]);
    assert_eq!(outcome.process_plan.reset_notify_cells, vec![cell]);
    assert_eq!(outcome.process_plan.reset_timer_grids, vec![active_grid]);
    assert_eq!(outcome.delayed_plan.cell_plans.len(), 1);
    assert_eq!(
        outcome.delayed_plan.cell_plans[0].plan.creature_relocations,
        vec![creature_guid]
    );
    assert_eq!(
        outcome.delayed_plan.cell_plans[0].plan.player_relocations,
        vec![player_guid]
    );
    assert_eq!(outcome.reset_outcome.reset_player_guids, vec![player_guid]);
    assert_eq!(
        outcome.reset_outcome.reset_creature_guids,
        vec![creature_guid]
    );
    assert!(
        !map.map_object(creature_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        !map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
}

#[test]
fn delayed_unit_relocation_visibility_plans_use_cpp_max_visibility_visits() {
    let mut map = test_map();
    let source_creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let source_creature_guid = source_creature.guid();
    let other_creature = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let other_creature_guid = other_creature.guid();
    let notified_creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let notified_creature_guid = notified_creature.guid();
    let player_notify = world_object_with_counter(HighGuid::Player, 4, 571, 7, false);
    let player_notify_guid = player_notify.guid();
    let player_normal = world_object_with_counter(HighGuid::Player, 5, 571, 7, false);
    let player_normal_guid = player_normal.guid();
    let old_player = guid(HighGuid::Player, 6);
    let old_creature = guid(HighGuid::Creature, 7);

    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, source_creature)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, other_creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, notified_creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_normal)
        .unwrap();
    for guid in [
        source_creature_guid,
        notified_creature_guid,
        player_notify_guid,
    ] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        [DelayedPlayerRelocationContext {
            player_guid: player_notify_guid,
            viewpoint_guid: player_notify_guid,
            previous_client_guids: vec![old_player, old_creature],
            relocated_for_ai: true,
        }],
        [
            DelayedCreatureRelocationContext {
                creature_guid: source_creature_guid,
                source_creature_alive: true,
            },
            DelayedCreatureRelocationContext {
                creature_guid: notified_creature_guid,
                source_creature_alive: true,
            },
        ],
    );

    assert_eq!(plans.creature_plans.len(), 2);
    let source_plan = plans
        .creature_plans
        .iter()
        .find(|plan| plan.creature_guid == source_creature_guid)
        .unwrap();
    assert_eq!(source_plan.cell_coord, cell);
    assert!(
        source_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_normal_guid)
    );
    assert!(
        !source_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_notify_guid)
    );
    assert!(
        source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(source_creature_guid, other_creature_guid))
    );
    assert!(
        source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(other_creature_guid, source_creature_guid))
    );
    assert!(
        !source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(notified_creature_guid, source_creature_guid))
    );

    assert_eq!(plans.player_plans.len(), 1);
    let player_plan = &plans.player_plans[0];
    assert_eq!(player_plan.player_guid, player_notify_guid);
    assert_eq!(player_plan.viewpoint_guid, player_notify_guid);
    assert!(
        player_plan
            .visibility_plan
            .out_of_range_guids
            .contains(&old_player)
    );
    assert!(
        player_plan
            .visibility_plan
            .out_of_range_guids
            .contains(&old_creature)
    );
    assert!(
        !player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(source_creature_guid, player_notify_guid))
    );
    assert!(
        player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(other_creature_guid, player_notify_guid))
    );
    assert!(
        !player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(notified_creature_guid, player_notify_guid))
    );
}

#[test]
fn delayed_unit_relocation_visibility_plans_report_missing_player_contexts_like_cpp_gap() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap()
        .cell;
    map.entity_world
        .get_mut(&player_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        std::iter::empty::<DelayedPlayerRelocationContext>(),
        std::iter::empty::<DelayedCreatureRelocationContext>(),
    );

    assert!(plans.player_plans.is_empty());
    assert_eq!(plans.missing_player_contexts, vec![player_guid]);
}

#[test]
fn ai_relocation_plan_for_player_checks_nearby_creatures_against_source_unit() {
    let player = guid(HighGuid::Player, 1);
    let world_creature = guid(HighGuid::Creature, 2);
    let grid_creature = guid(HighGuid::Creature, 3);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(world_creature);
    nearby.grid.creatures.insert(grid_creature);

    let plan = AIRelocationPlan::from_nearby_like_cpp(player, false, &nearby);

    assert_eq!(
        plan.creature_unit_checks,
        vec![(world_creature, player), (grid_creature, player)]
    );
}

#[test]
fn ai_relocation_plan_for_creature_checks_both_cpp_directions() {
    let source = guid(HighGuid::Creature, 1);
    let other = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(source);
    nearby.grid.creatures.insert(other);

    let plan = AIRelocationPlan::from_nearby_like_cpp(source, true, &nearby);

    assert_eq!(
        plan.creature_unit_checks,
        vec![(other, source), (source, other)]
    );
}

#[test]
fn ai_relocation_plan_deduplicates_world_grid_creatures_and_skips_self_worker_noop() {
    let source = guid(HighGuid::Creature, 1);
    let other = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(source);
    nearby.grid.creatures.insert(source);
    nearby.world.creatures.insert(other);
    nearby.grid.creatures.insert(other);

    let plan = AIRelocationPlan::from_nearby_like_cpp(source, false, &nearby);

    assert_eq!(plan.creature_unit_checks, vec![(other, source)]);
}

#[test]
fn object_update_plan_for_nearby_like_cpp_selects_in_world_updateable_objects_only() {
    let mut map = test_map();
    let player = world_object(HighGuid::Player, 571, 7, true);
    let player_guid = player.guid();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let creature_guid = creature.guid();
    let gameobject = world_object(HighGuid::GameObject, 571, 7, true);
    let gameobject_guid = gameobject.guid();
    let dynamic_not_in_world = world_object(HighGuid::DynamicObject, 571, 7, false);
    let dynamic_guid = dynamic_not_in_world.guid();
    let missing_conversation = guid(HighGuid::Conversation, 9);
    map.insert_map_object(AccessorObjectKind::Player, player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::DynamicObject, dynamic_not_in_world)
        .unwrap();

    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player_guid);
    nearby.grid.creatures.insert(creature_guid);
    nearby.grid.gameobjects.insert(gameobject_guid);
    nearby.grid.dynamic_objects.insert(dynamic_guid);
    nearby.grid.conversations.insert(missing_conversation);

    let plan = map.object_update_plan_for_nearby_like_cpp(&nearby, 42);

    assert_eq!(plan.diff_ms, 42);
    assert_eq!(plan.update_guids, vec![creature_guid, gameobject_guid]);
}

#[test]
fn object_update_plan_for_nearby_like_cpp_deduplicates_world_and_grid_objects() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let creature_guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    let mut nearby = NearbyCellGuids::default();
    nearby.world.creatures.insert(creature_guid);
    nearby.grid.creatures.insert(creature_guid);

    let plan = map.object_update_plan_for_nearby_like_cpp(&nearby, 1);

    assert_eq!(plan.update_guids, vec![creature_guid]);
}

#[test]
fn map_update_visit_plan_like_cpp_filters_sources_by_cpp_in_world_guards() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, true);
    let player_guid = player.guid();
    let offline_player = world_object_with_counter(HighGuid::Player, 2, 571, 7, false);
    let offline_player_guid = offline_player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 3, 571, 7, true);
    let viewpoint_guid = viewpoint.guid();
    let far_combat = world_object_with_counter(HighGuid::Creature, 4, 571, 7, true);
    let far_combat_guid = far_combat.guid();
    let offline_aura = world_object_with_counter(HighGuid::Creature, 5, 571, 7, false);
    let offline_aura_guid = offline_aura.guid();
    let active_non_player = world_object_with_counter(HighGuid::DynamicObject, 6, 571, 7, true);
    let active_non_player_guid = active_non_player.guid();
    let transport = world_object_with_counter(HighGuid::Transport, 7, 571, 7, false);
    let transport_guid = transport.guid();

    map.insert_map_object(AccessorObjectKind::Player, player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Player, offline_player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, far_combat)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, offline_aura)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::DynamicObject, active_non_player)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Transport, transport)
        .unwrap();

    let plan = map.map_update_visit_plan_like_cpp(
        [
            MapUpdatePlayerSources {
                player_guid,
                viewpoint_guid: Some(viewpoint_guid),
                far_combat_unit_guids: vec![far_combat_guid],
                far_aura_caster_guids: vec![offline_aura_guid],
                far_summon_guids: vec![],
            },
            MapUpdatePlayerSources {
                player_guid: offline_player_guid,
                viewpoint_guid: Some(far_combat_guid),
                far_combat_unit_guids: vec![viewpoint_guid],
                far_aura_caster_guids: vec![],
                far_summon_guids: vec![],
            },
        ],
        [active_non_player_guid, offline_aura_guid],
        [transport_guid],
        50,
    );

    assert_eq!(plan.diff_ms, 50);
    assert_eq!(plan.session_update_players, vec![player_guid]);
    assert_eq!(plan.player_update_guids, vec![player_guid]);
    assert_eq!(plan.transport_update_guids, vec![transport_guid]);
    assert_eq!(
        plan.nearby_visit_centers
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([
            player_guid,
            viewpoint_guid,
            far_combat_guid,
            active_non_player_guid
        ])
    );
    assert!(plan.process_relocation_notifies);
}

#[test]
fn map_update_visit_plan_like_cpp_processes_relocation_notifies_only_for_players_or_active_non_players()
 {
    let mut map = test_map();
    let transport = world_object_with_counter(HighGuid::Transport, 7, 571, 7, false);
    let transport_guid = transport.guid();
    map.insert_map_object(AccessorObjectKind::Transport, transport)
        .unwrap();

    let plan = map.map_update_visit_plan_like_cpp(
        std::iter::empty::<MapUpdatePlayerSources>(),
        std::iter::empty::<ObjectGuid>(),
        [transport_guid],
        1,
    );

    assert_eq!(plan.transport_update_guids, vec![transport_guid]);
    assert!(!plan.process_relocation_notifies);
}

#[test]
fn process_relocation_notifies_plan_like_cpp_waits_for_active_grid_timer() {
    let mut map = test_map();
    let grid = GridCoord::new(2, 3);
    map.ensure_grid_created(grid);
    map.get_ngrid_mut(grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    let marked = CellCoord::new(2 * MAX_NUMBER_OF_CELLS, 3 * MAX_NUMBER_OF_CELLS);

    let plan = map.process_relocation_notifies_plan_like_cpp([marked], 999, 1000);

    assert!(plan.delayed_relocation_cells.is_empty());
    assert!(plan.reset_notify_cells.is_empty());
    assert!(plan.reset_timer_grids.is_empty());
}

#[test]
fn process_relocation_notifies_plan_like_cpp_visits_marked_cells_and_resets_timer() {
    let mut map = test_map();
    let active_grid = GridCoord::new(2, 3);
    let idle_grid = GridCoord::new(4, 5);
    map.ensure_grid_created(active_grid);
    map.ensure_grid_created(idle_grid);
    map.get_ngrid_mut(active_grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    map.get_ngrid_mut(idle_grid)
        .unwrap()
        .set_state(GridStateKind::Idle);
    let marked_a = CellCoord::new(2 * MAX_NUMBER_OF_CELLS, 3 * MAX_NUMBER_OF_CELLS);
    let marked_b = CellCoord::new(2 * MAX_NUMBER_OF_CELLS + 1, 3 * MAX_NUMBER_OF_CELLS);
    let marked_idle = CellCoord::new(4 * MAX_NUMBER_OF_CELLS, 5 * MAX_NUMBER_OF_CELLS);

    let plan = map.process_relocation_notifies_plan_like_cpp(
        [marked_b, marked_idle, marked_a],
        1000,
        1000,
    );

    assert_eq!(plan.diff_ms, 1000);
    assert_eq!(plan.delayed_relocation_cells, vec![marked_a, marked_b]);
    assert_eq!(plan.reset_notify_cells, vec![marked_a, marked_b]);
    assert_eq!(plan.reset_timer_grids, vec![active_grid]);
    assert_eq!(
        map.get_ngrid(active_grid)
            .unwrap()
            .info()
            .relocation_timer()
            .expire_time_ms(),
        1000
    );
}

#[test]
fn reset_notify_flags_for_cells_like_cpp_resets_only_players_and_creatures() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let creature = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 3, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    let player_cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    for guid in [player_guid, creature_guid, gameobject_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let outcome = map.reset_notify_flags_for_cells_like_cpp([player_cell]);

    assert_eq!(outcome.reset_player_guids, vec![player_guid]);
    assert_eq!(outcome.reset_creature_guids, vec![creature_guid]);
    assert!(outcome.missing_guids.is_empty());
    assert!(
        !map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        !map.map_object(creature_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        map.map_object(gameobject_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
}

#[test]
fn process_map_object_move_list_like_cpp_relocates_active_entries_and_resets_inactive() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(5.0, 5.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: gameobject_guid,
            kind: AccessorObjectKind::GameObject,
            move_state: MapObjectCellMoveState::Inactive,
            new_position: Position::xyz(6.0, 6.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
    ]);

    assert_eq!(plan.relocated, vec![creature_guid]);
    assert_eq!(plan.reset_inactive_or_none, vec![gameobject_guid]);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().position(),
        Position::xyz(5.0, 5.0, 3.0)
    );
}

#[test]
fn process_map_object_move_list_like_cpp_uses_respawn_or_removal_fallbacks() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    let pet = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let pet_guid = pet.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, pet)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: Some(Position::xyz(2.0, 2.0, 3.0)),
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: gameobject_guid,
            kind: AccessorObjectKind::GameObject,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: pet_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: true,
        },
    ]);

    assert_eq!(plan.respawn_relocated, vec![creature_guid]);
    assert_eq!(plan.remove_from_world, vec![gameobject_guid]);
    assert_eq!(plan.pet_removed, vec![pet_guid]);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().position(),
        Position::xyz(2.0, 2.0, 3.0)
    );
}

#[test]
fn process_map_object_move_list_like_cpp_blocks_dynamic_and_skips_not_in_world() {
    let mut map = test_map();
    let dynamic = world_object_with_counter(HighGuid::DynamicObject, 1, 571, 7, false);
    let dynamic_guid = dynamic.guid();
    let area_trigger = world_object_with_counter(HighGuid::AreaTrigger, 2, 571, 7, false);
    let area_trigger_guid = area_trigger.guid();
    let offline_creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let offline_creature_guid = offline_creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::DynamicObject, dynamic)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::AreaTrigger, area_trigger)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, offline_creature)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: dynamic_guid,
            kind: AccessorObjectKind::DynamicObject,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: area_trigger_guid,
            kind: AccessorObjectKind::AreaTrigger,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: offline_creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(2.0, 2.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
    ]);

    assert_eq!(
        plan.blocked_unloaded_grid,
        vec![dynamic_guid, area_trigger_guid]
    );
    assert_eq!(plan.skipped_not_in_world, vec![offline_creature_guid]);
}

#[test]
fn ensure_grid_created_sets_idle_grid_and_loads_reversed_terrain_coords() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);

    assert!(map.ensure_grid_created(coord));
    assert!(!map.ensure_grid_created(coord));

    let grid = map.get_ngrid(coord).unwrap();
    assert_eq!(grid.grid_id(), 2 * MAX_NUMBER_OF_GRIDS + 3);
    assert_eq!(grid.state(), GridStateKind::Idle);
    assert!(!grid.grid_object_data_loaded());
    assert_eq!(map.terrain().loads, vec![(61, 60)]);
}

#[test]
fn ensure_grid_loaded_marks_loaded_before_object_loader_hook() {
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(2, 3));

    assert!(map.ensure_grid_loaded(&cell));
    assert!(!map.ensure_grid_loaded(&cell));

    assert!(map.is_grid_loaded(GridCoord::new(2, 3)));
    assert_eq!(map.lifecycle().loads, 1);
}

#[test]
fn registered_loaded_corpse_tracks_grid_load_and_unload_like_cpp() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let position = Position::new(10.0, 20.0, 30.0, 1.5);
    let cell = Cell::from_world(position.x, position.y);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    let guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 1);
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse.world_mut().set_map(571, 0).unwrap();
    corpse.world_mut().relocate(position);

    assert!(!map.register_loaded_corpse_like_cpp(corpse).unwrap());
    assert!(
        !map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(
        map.nearby_cell_guids_like_cpp(position.x, position.y, 1.0)
            .world
            .corpses
            .is_empty()
    );

    assert!(map.ensure_grid_loaded(&cell));
    assert!(
        map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(
        map.nearby_cell_guids_like_cpp(position.x, position.y, 1.0)
            .world
            .corpses
            .contains(&guid)
    );

    assert!(map.unload_grid_at(grid, true));
    assert!(
        !map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(map.ensure_grid_loaded(&cell));
    assert!(
        map.get_typed_corpse(guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
}

#[test]
fn loaded_grid_coords_only_reports_object_data_loaded_grids_like_cpp() {
    let mut map = test_map();
    let created_only = GridCoord::new(2, 3);
    let loaded_a = GridCoord::new(4, 5);
    let loaded_b = GridCoord::new(4, 6);

    assert!(map.ensure_grid_created(created_only));
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(loaded_b)));
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(loaded_a)));

    assert_eq!(map.loaded_grid_coords_like_cpp(), vec![loaded_a, loaded_b]);
}

#[test]
fn active_object_loading_sets_grid_active_and_short_expiry() {
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(2, 3));

    assert!(map.ensure_grid_loaded_for_active_object(&cell, ActiveObjectKind::NonPlayer));

    let grid = map.get_ngrid(GridCoord::new(2, 3)).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    assert_eq!(grid.info().time_tracker().remaining_ms(), 100);
    assert!(map.active_objects_near_grid(grid));
}

#[test]
fn player_phase_loading_invokes_personal_phase_tracker_before_activation() {
    let mut store = crate::spawn::SpawnStore::new();
    let spawn = crate::spawn::SpawnData {
        object_type: crate::spawn::SpawnObjectType::Creature,
        spawn_id: 100,
        map_id: 571,
        db_data: true,
        spawn_group: crate::spawn::SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 1.0, 2.0),
        phase_use_flags: 0,
        phase_id: 9,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |phase_id| phase_id == 9);
    let corpses = crate::object_grid_loader::CorpseCellStore::new();
    let mut loader =
        crate::object_grid_loader::ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);
    let owner = ObjectGuid::create_player(1, 100);
    let phase_shift = crate::personal_phase::PhaseShift::new(
        Some(owner),
        vec![crate::personal_phase::PhaseRef::new(9, true)],
    );
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(32, 32));

    assert!(map.ensure_grid_loaded_for_player_phase(&cell, &phase_shift, &mut loader));

    let grid = map.get_ngrid(GridCoord::new(32, 32)).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    assert_eq!(
        grid.get_grid_type(0, 0)
            .unwrap()
            .grid_objects
            .creatures
            .len(),
        1
    );
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);
}

#[test]
fn unload_grid_applies_guid_lifecycle_actions_to_canonical_map_objects_like_cpp() {
    let mut map = guid_unload_test_map();
    let coord = GridCoord::new(2, 3);
    let cell = cell_from_grid_center(coord);
    assert!(map.ensure_grid_loaded(&cell));

    let creature = test_creature_for_spawn(4181, 4181, true);
    let creature_guid = creature.unit().world().guid();
    let gameobject = test_gameobject_for_spawn(4182, 4182);
    let gameobject_guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let grid_cell = map
        .get_ngrid_mut(coord)
        .unwrap()
        .get_grid_type_mut(0, 0)
        .unwrap();
    grid_cell.grid_objects.creatures.insert(creature_guid);
    grid_cell.grid_objects.gameobjects.insert(gameobject_guid);

    assert!(map.unload_grid_at(coord, true));

    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.terrain().unloads, vec![(61, 60)]);
    assert_eq!(map.map_object_count(), 2);

    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 2);
    assert!(creature.grid_unload_delete_requested());
    assert!(!creature.grid_unload_respawn_relocation_requested());

    let gameobject = map
        .map_object_record(gameobject_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(gameobject.world().object().is_destroyed_object());
    assert_eq!(gameobject.cleanup_before_delete_count(), 2);
    assert!(gameobject.grid_unload_delete_requested());
    assert!(!gameobject.grid_unload_respawn_relocation_requested());
}

#[test]
fn unload_grid_purges_personal_phase_tracker_before_unloader_like_cpp() {
    let mut store = crate::spawn::SpawnStore::new();
    let spawn = crate::spawn::SpawnData {
        object_type: crate::spawn::SpawnObjectType::Creature,
        spawn_id: 4183,
        map_id: 571,
        db_data: true,
        spawn_group: crate::spawn::SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 1.0, 2.0),
        phase_use_flags: 0,
        phase_id: 9,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |phase_id| phase_id == 9);
    let corpses = crate::object_grid_loader::CorpseCellStore::new();
    let mut loader =
        crate::object_grid_loader::ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);
    let owner = ObjectGuid::create_player(1, 4183);
    let phase_shift = crate::personal_phase::PhaseShift::new(
        Some(owner),
        vec![crate::personal_phase::PhaseRef::new(9, true)],
    );
    let mut map = test_map();
    let coord = GridCoord::new(32, 32);
    let cell = cell_from_grid_center(coord);

    assert!(map.ensure_grid_loaded_for_player_phase(&cell, &phase_shift, &mut loader));
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);

    assert!(map.unload_grid_at(coord, true));

    assert_eq!(map.personal_phase_tracker().tracker_count(), 0);
    assert!(map.get_ngrid(coord).is_none());
}

#[test]
fn active_to_idle_stop_drains_guid_lifecycle_stoper_actions_into_creature_like_cpp() {
    let mut map = guid_unload_test_map();
    let coord = GridCoord::new(2, 3);
    assert!(map.ensure_grid_loaded(&cell_from_grid_center(coord)));

    let dynamic_object_guid = guid(HighGuid::DynamicObject, 4184);
    let area_trigger_guid = guid(HighGuid::AreaTrigger, 4185);
    let victim_guid = guid(HighGuid::Creature, 4186);
    let mut creature = test_creature_for_spawn(4184, 4184, true);
    let creature_guid = creature.unit().world().guid();
    creature.register_dynamic_object(dynamic_object_guid);
    creature.register_area_trigger(area_trigger_guid);
    creature.unit_mut().set_attacking(Some(victim_guid));
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let grid = map.get_ngrid_mut(coord).unwrap();
    grid.get_grid_type_mut(0, 0)
        .unwrap()
        .grid_objects
        .creatures
        .insert(creature_guid);
    grid.set_state(GridStateKind::Active);

    assert!(!map.update_grid_state_at(coord, 1001));

    let grid = map.get_ngrid(coord).unwrap();
    assert_eq!(grid.state(), GridStateKind::Idle);
    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(!creature.is_in_combat());
    assert!(creature.dynamic_objects().is_empty());
    assert_eq!(
        creature.removed_dynamic_objects_from_grid_unload(),
        &[dynamic_object_guid]
    );
    assert!(creature.area_triggers().is_empty());
    assert_eq!(
        creature.removed_area_triggers_from_grid_unload(),
        &[area_trigger_guid]
    );
}

#[test]
fn unload_grid_refuses_world_creatures_and_active_neighbors_unless_forced() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    let cell = cell_from_grid_center(coord);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(coord)
        .unwrap()
        .get_grid_type_mut(0, 0)
        .unwrap()
        .world_objects
        .creatures
        .insert(ObjectGuid::new(1, 1));

    assert!(!map.unload_grid_at(coord, false));
    assert!(map.is_grid_loaded(coord));

    assert!(map.unload_grid_at(coord, true));
    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.lifecycle().evacuates, 0);
    assert_eq!(map.lifecycle().cleans, 1);
    assert_eq!(map.lifecycle().unloads, 1);
    assert_eq!(map.terrain().unloads, vec![(61, 60)]);
}

#[test]
fn update_grid_state_at_removes_grid_when_removal_unloads_successfully() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    map.ensure_grid_loaded(&cell_from_grid_center(coord));
    map.get_ngrid_mut(coord)
        .unwrap()
        .set_state(GridStateKind::Removal);

    assert!(map.update_grid_state_at(coord, 1001));

    assert!(map.get_ngrid(coord).is_none());
    assert_eq!(map.lifecycle().evacuates, 1);
    assert_eq!(map.lifecycle().cleans, 1);
    assert_eq!(map.lifecycle().unloads, 1);
}

#[test]
fn active_objects_near_grid_matches_cpp_cell_range_expansion() {
    let mut map = test_map();
    let coord = GridCoord::new(10, 10);
    map.ensure_grid_created(coord);
    let grid = map.get_ngrid(coord).unwrap();
    assert!(!map.active_objects_near_grid(grid));

    map.mark_active_cell(CellCoord::new(79, 80));
    let grid = map.get_ngrid(coord).unwrap();
    assert!(map.active_objects_near_grid(grid));

    map.unmark_active_cell(CellCoord::new(79, 80));
    map.mark_active_cell(CellCoord::new(1, 1));
    let grid = map.get_ngrid(coord).unwrap();
    assert!(!map.active_objects_near_grid(grid));
}

#[test]
fn grid_id_loaded_uses_cpp_public_grid_id_decomposition() {
    let mut map = test_map();
    let coord = GridCoord::new(2, 3);
    map.ensure_grid_loaded(&cell_from_grid_center(coord));

    assert!(is_grid_id_loaded(&map, 3 * MAX_NUMBER_OF_GRIDS + 2));
}
