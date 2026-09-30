// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-GameObject update coordinator with optional pool actions.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(in crate::map) fn update_game_object_with_optional_pool_update_like_cpp<L>(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectUpdateOutcomeLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(record) = self.map_object_record(game_object_guid) else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let Some(game_object) = record.game_object() else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        let despawn_delay_before_ms = game_object.despawn_delay();
        let despawn_respawn_time_secs = game_object.despawn_respawn_time();
        if !game_object.world().object().is_in_world() {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotInWorld,
                despawn_delay_before_ms: Some(despawn_delay_before_ms),
                despawn_delay_after_ms: Some(despawn_delay_before_ms),
                despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let entity_update = {
            let Some(record) = self.entity_world.get_mut(&game_object_guid) else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            let Some(game_object) = record.game_object_mut() else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            game_object.update_like_cpp(diff_ms)
        };

        let (
            linked_trap_guid,
            linked_trap_removed,
            linked_trap_remove_queued,
            linked_trap_missing_or_self,
        ) = if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            (None, false, false, false)
        } else {
            self.map_object_record(game_object_guid)
                .and_then(|record| record.game_object())
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| game_object.linked_trap_guid_like_cpp())
                .map_or((None, false, false, false), |linked_guid| {
                    if linked_guid.is_empty() || linked_guid == game_object_guid {
                        return (
                            (!linked_guid.is_empty()).then_some(linked_guid),
                            false,
                            false,
                            true,
                        );
                    }

                    let linked_trap_exists = self
                        .map_object_record(linked_guid)
                        .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                        .and_then(|record| record.game_object())
                        .is_some();
                    if !linked_trap_exists {
                        return (Some(linked_guid), false, false, true);
                    }

                    match self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                        linked_guid,
                        pool_update,
                        load_record.as_mut().map(|loader| &mut **loader),
                    ) {
                        Some(delete) => (
                            Some(linked_guid),
                            false,
                            delete
                                .remove_list
                                .as_ref()
                                .is_some_and(|remove| remove.queued || remove.duplicate),
                            false,
                        ),
                        None => (Some(linked_guid), false, false, true),
                    }
                })
        };

        let mut goober_spell_cast_spell_id = None;
        let mut goober_spell_casts_represented = 0;
        let mut goober_users_cleared = false;
        let mut goober_state_reset = false;
        let mut goober_nodespawn_return = false;
        let mut non_consumed_chest_or_goober_return = false;
        let mut non_consumed_restock_armed = false;
        let mut non_consumed_set_ready = false;
        let mut non_consumed_update_visibility_represented = false;
        let mut non_consumed_update_dynamic_flags_represented = false;
        let mut non_consumed_source_missing = false;
        let mut summoned_expired_delete = false;
        let mut summoned_expired_respawn_time_zeroed = false;
        let mut summoned_expired_despawn_represented = false;
        let mut summoned_expired_go_state_ready = false;
        let mut new_flag_drop_owner_in_base_command_represented = false;
        let mut new_flag_drop_owner_missing_or_empty = false;
        let mut new_flag_drop_owner_wrong_kind = false;
        let mut new_flag_drop_owner_not_new_flag = false;
        let mut generic_not_ready = false;
        let mut generic_visual_despawn_represented = false;
        let mut generic_flags_restored_represented = false;
        let mut generic_zero_respawn_delay_return = false;
        let mut generic_despawn_at_action_source_missing = false;
        let mut generic_respawn_scheduled_time = None;
        let mut generic_spawned_by_default_branch = false;
        let mut generic_temporary_respawn_zeroed = false;
        let mut generic_respawn_timer_add = None;
        let mut generic_respawn_save_missing_spawn_id = false;
        let mut generic_respawn_save_missing_gameobject_data = false;
        let mut generic_respawn_compatibility_db_only_represented = false;
        let mut generic_visibility_on_destroy_represented = false;

        if entity_update.status != EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(ObjectMut::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .filter(|game_object| game_object.data().type_id == GAMEOBJECT_TYPE_GOOBER as i8)
            {
                if let Some(goober_source) = game_object.represented_goober_use_source_like_cpp() {
                    if goober_source.spell_id != 0 {
                        goober_spell_cast_spell_id = Some(goober_source.spell_id);
                        goober_spell_casts_represented =
                            game_object.unique_users_snapshot_like_cpp().len();
                        game_object.clear_unique_users_and_reset_use_times_like_cpp();
                        goober_users_cleared = true;
                    }

                    if goober_source.lock_id != 0 || goober_source.auto_close_ms != 0 {
                        game_object.set_go_state(GoState::Ready);
                        goober_state_reset = true;
                    }
                }

                goober_nodespawn_return = game_object.data().flags & GO_FLAG_NODESPAWN != 0;
            }
        }

        let loot_cleared = if entity_update.status
            == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested
            || goober_nodespawn_return
        {
            false
        } else if let Some(game_object) = self
            .entity_world
            .get_mut(&game_object_guid)
            .and_then(ObjectMut::game_object_mut)
            .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
        {
            game_object.clear_loot_like_cpp();
            true
        } else {
            false
        };

        if loot_cleared {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(ObjectMut::game_object_mut)
            {
                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => None,
                };

                if matches!(go_type, GAMEOBJECT_TYPE_CHEST | GAMEOBJECT_TYPE_GOOBER) {
                    // C++ anchor: GameObject.cpp:1609-1623. This represented seam
                    // deliberately does not call the broader SetLootState facade from
                    // GameObject.cpp:3683-3709 because line 1617 only writes
                    // GO_NOT_READY after arming the fully-looted chest restock timer;
                    // Activated-specific restock/collision semantics are not part of
                    // this branch. Owner/spell-created expiration is consumed below
                    // through the represented `Delete()` seam.
                    if let Some(despawn_at_action) = despawn_at_action {
                        let is_summoned_and_expired = (game_object.owner_guid()
                            != ObjectGuid::EMPTY
                            || game_object.spell_id() != 0)
                            && game_object.respawn_time() == 0;
                        if !despawn_at_action && !is_summoned_and_expired {
                            if go_type == GAMEOBJECT_TYPE_CHEST {
                                if let Some(source) =
                                    game_object.represented_chest_loot_source_like_cpp()
                                {
                                    if source.chest_restock_time_secs > 0 {
                                        let restock_time = game_time_secs.saturating_add(
                                            i64::from(source.chest_restock_time_secs),
                                        );
                                        game_object.set_restock_time_like_cpp(restock_time);
                                        game_object.set_loot_state(LootState::NotReady, None);
                                        non_consumed_restock_armed = true;
                                        non_consumed_update_dynamic_flags_represented = true;
                                    } else {
                                        game_object.set_loot_state(LootState::Ready, None);
                                        non_consumed_set_ready = true;
                                    }
                                }
                            } else {
                                game_object.set_loot_state(LootState::Ready, None);
                                non_consumed_set_ready = true;
                            }
                            non_consumed_chest_or_goober_return = true;
                            non_consumed_update_visibility_represented = true;
                        }
                    } else {
                        non_consumed_source_missing = true;
                    }
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return {
            let summoned_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(|record| record.game_object())
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| {
                    (
                        game_object.data().type_id as u32,
                        game_object.owner_guid(),
                        game_object.spell_id(),
                        game_object.respawn_time(),
                    )
                });

            if let Some((go_type, owner_guid, spell_id, respawn_time)) = summoned_snapshot {
                let is_summoned_and_expired =
                    (owner_guid != ObjectGuid::EMPTY || spell_id != 0) && respawn_time == 0;
                if is_summoned_and_expired {
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(ObjectMut::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                        game_object.set_loot_state(LootState::NotReady, None);
                        summoned_expired_respawn_time_zeroed = true;
                        summoned_expired_despawn_represented = true;
                        if go_type != GAMEOBJECT_TYPE_TRANSPORT {
                            game_object.set_go_state(GoState::Ready);
                            summoned_expired_go_state_ready = true;
                        }
                    }

                    if go_type == GAMEOBJECT_TYPE_NEW_FLAG_DROP {
                        if owner_guid == ObjectGuid::EMPTY {
                            new_flag_drop_owner_missing_or_empty = true;
                        } else {
                            match self.map_object_record(owner_guid) {
                                Some(owner_record)
                                    if owner_record.kind() == AccessorObjectKind::GameObject =>
                                {
                                    match owner_record.game_object() {
                                        Some(owner_go)
                                            if owner_go.data().type_id as u32
                                                == GAMEOBJECT_TYPE_NEW_FLAG =>
                                        {
                                            // C++ NewFlag::SetState(InBase, nullptr) has
                                            // no full Rust go-type state object yet; record
                                            // the exact typed owner command as represented
                                            // evidence only, without faking ZoneScript or
                                            // fanout.
                                            new_flag_drop_owner_in_base_command_represented = true;
                                        }
                                        Some(_) => {
                                            new_flag_drop_owner_not_new_flag = true;
                                        }
                                        None => {
                                            new_flag_drop_owner_wrong_kind = true;
                                        }
                                    }
                                }
                                Some(_) => {
                                    new_flag_drop_owner_wrong_kind = true;
                                }
                                None => {
                                    new_flag_drop_owner_missing_or_empty = true;
                                }
                            }
                        }
                    }

                    summoned_expired_delete = true;
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return && !summoned_expired_delete {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(ObjectMut::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
            {
                // C++ anchor: GameObject.cpp:1639-1651. This represented seam
                // preserves the `if (!m_respawnDelayTime) return;` early return;
                // the positive-delay scheduling/SaveRespawnTime tail is consumed
                // immediately below after releasing the typed GameObject borrow.
                game_object.set_loot_state(LootState::NotReady, None);
                generic_not_ready = true;

                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => Some(false),
                };
                generic_despawn_at_action_source_missing = despawn_at_action.is_none();
                let visual_despawn = despawn_at_action.unwrap_or(false)
                    || game_object.go_anim_progress_like_cpp() > 0;
                if visual_despawn {
                    generic_visual_despawn_represented = true;
                    generic_flags_restored_represented =
                        game_object.restore_represented_baseline_flags_like_cpp();
                }
                generic_zero_respawn_delay_return = game_object.respawn_delay_time() == 0;
            }
        }

        if generic_not_ready && !generic_zero_respawn_delay_return {
            let generic_respawn_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(|record| record.game_object())
                .map(|game_object| {
                    (
                        game_object.spawned_by_default(),
                        game_object.respawn_compatibility_mode(),
                        game_object.respawn_delay_time(),
                        game_object.spawn_id(),
                        game_object.has_represented_gameobject_data_like_cpp(),
                        game_object.world().object().entry(),
                        game_object.world().position(),
                    )
                });

            if let Some((
                spawned_by_default,
                respawn_compatibility_mode,
                respawn_delay_time,
                spawn_id,
                represented_gameobject_data_present,
                entry,
                position,
            )) = generic_respawn_snapshot
            {
                if spawned_by_default {
                    let scheduled_respawn_time =
                        game_time_secs.saturating_add(i64::from(respawn_delay_time));
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(ObjectMut::game_object_mut)
                    {
                        game_object.set_respawn_time(scheduled_respawn_time);
                    }
                    generic_respawn_scheduled_time = Some(scheduled_respawn_time);
                    generic_spawned_by_default_branch = true;

                    if !represented_gameobject_data_present {
                        // C++ `GameObject::SaveRespawnTime` is guarded by `m_goData`.
                        // A nonzero spawn id is not enough evidence for map-owned
                        // respawn persistence in this represented seam.
                        generic_respawn_save_missing_gameobject_data = true;
                    } else if spawn_id == 0 {
                        generic_respawn_save_missing_spawn_id = true;
                    } else if scheduled_respawn_time > game_time_secs {
                        if respawn_compatibility_mode {
                            // C++ `SaveRespawnTime` compatibility mode calls
                            // `SaveRespawnInfoDB` only. `wow-map` owns no async DB
                            // writes, so record DB-only evidence without mutating the
                            // map-owned respawn store.
                            generic_respawn_compatibility_db_only_represented = true;
                        } else {
                            let grid = compute_grid_coord(position.x, position.y);
                            let add_outcome = self.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
                                object_type: SpawnObjectType::GameObject,
                                spawn_id,
                                entry,
                                respawn_time: scheduled_respawn_time,
                                grid_id: grid.get_id(),
                            });
                            generic_respawn_timer_add = Some(add_outcome);
                        }
                    }

                    if respawn_compatibility_mode {
                        generic_visibility_on_destroy_represented = true;
                    }
                } else {
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(ObjectMut::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                    }
                    generic_temporary_respawn_zeroed = true;
                    generic_visibility_on_destroy_represented = spawn_id != 0;
                }
            }
        }

        if summoned_expired_delete
            || (generic_not_ready
                && !generic_zero_respawn_delay_return
                && !generic_visibility_on_destroy_represented)
        {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: generic_visual_despawn_represented
                    || delete_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        } else if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: delete_visual_despawn_represented,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            }
        } else {
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::Updated,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list: None,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return,
                non_consumed_restock_armed,
                non_consumed_set_ready,
                non_consumed_update_visibility_represented,
                non_consumed_update_dynamic_flags_represented,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        }
    }

}
