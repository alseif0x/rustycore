//! Nearby-cell `ObjectUpdater` selection and typed consumers.
//!
//! C++ `Map::Update` marks cells from active sources, then visits only the
//! object families held by those cells. Keeping the GUID consumers in this
//! module prevents the family lifecycle modules from growing past their
//! reviewed physical budgets while preserving one canonical `Map` owner.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Update only the canonical DynamicObjects selected by one
    /// `ObjectUpdater` nearby-cell plan.
    pub fn update_dynamic_objects_for_guids_like_cpp(
        &mut self,
        dynamic_object_guids: impl IntoIterator<Item = ObjectGuid>,
        elapsed_ms: u32,
    ) -> DynamicObjectsUpdateSummaryLikeCpp {
        let mut summary = DynamicObjectsUpdateSummaryLikeCpp::default();
        for guid in dynamic_object_guids {
            summary.visited += 1;
            let outcome = self.update_dynamic_object_like_cpp(guid, elapsed_ms);
            match outcome.status {
                DynamicObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject => {
                    summary.missing_or_stale += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotDynamicObject => {
                    summary.not_dynamic_object += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }
        summary
    }

    /// Update only the canonical Creature/Pet GUIDs selected by one
    /// `ObjectUpdater` nearby-cell plan. Pet records retain the same outcome
    /// classification as the existing map-wide visitor.
    pub fn update_creatures_for_guids_like_cpp<F>(
        &mut self,
        creature_guids: impl IntoIterator<Item = ObjectGuid>,
        diff_ms: u32,
        now_secs: i64,
        mut context_resolver: F,
    ) -> CreatureUpdateSummaryLikeCpp
    where
        F: FnMut(
            ObjectGuid,
            CreatureTransformVitalsSnapshotLikeCpp,
        ) -> CreatureRuntimeUpdateContext,
    {
        let creature_guids = creature_guids.into_iter().collect::<Vec<_>>();
        let creature_lookups = self
            .entity_world
            .creature_transform_vitals_lookups(creature_guids);
        let mut summary = CreatureUpdateSummaryLikeCpp::default();

        for (guid, snapshot) in creature_lookups {
            summary.visited += 1;
            let context = snapshot.map_or_else(CreatureRuntimeUpdateContext::default, |snapshot| {
                context_resolver(guid, snapshot)
            });
            let outcome = self.update_creature_like_cpp(guid, diff_ms, now_secs, context);
            match outcome.status {
                CreatureUpdateStatusLikeCpp::Updated => {
                    summary.updated += 1;
                    summary.actions_recorded += outcome.actions_recorded;
                }
                CreatureUpdateStatusLikeCpp::MissingCreature => summary.skipped_missing += 1,
                CreatureUpdateStatusLikeCpp::NotCreature => summary.skipped_non_creature += 1,
                CreatureUpdateStatusLikeCpp::NotInWorld => summary.skipped_not_in_world += 1,
            }
        }
        summary
    }

    /// Update selected GameObjects while preserving the optional pool and
    /// loaded-grid respawn lifecycle of the map-wide helper.
    pub fn update_game_objects_for_guids_with_optional_pool_update_like_cpp<L>(
        &mut self,
        game_object_guids: impl IntoIterator<Item = ObjectGuid>,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectsUpdateSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut summary = GameObjectsUpdateSummaryLikeCpp::default();
        for guid in game_object_guids {
            summary.visited += 1;
            let outcome = self.update_game_object_with_optional_pool_update_like_cpp(
                guid,
                diff_ms,
                game_time_secs,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            if outcome.linked_trap_removed {
                summary.linked_traps_removed += 1;
            }
            if outcome.linked_trap_remove_queued {
                summary.linked_traps_remove_queued += 1;
            }
            if outcome.loot_cleared {
                summary.loot_cleared += 1;
            }
            summary.goober_spell_casts_represented += outcome.goober_spell_casts_represented;
            if outcome.goober_users_cleared {
                summary.goober_users_cleared += 1;
            }
            if outcome.goober_state_reset {
                summary.goober_state_reset += 1;
            }
            if outcome.goober_nodespawn_return {
                summary.goober_nodespawn_returns += 1;
            }
            if outcome.non_consumed_chest_or_goober_return {
                summary.non_consumed_chest_or_goober_returns += 1;
            }
            if outcome.non_consumed_restock_armed {
                summary.non_consumed_restock_armed += 1;
            }
            if outcome.non_consumed_set_ready {
                summary.non_consumed_set_ready += 1;
            }
            if outcome.non_consumed_update_visibility_represented {
                summary.non_consumed_update_visibility_represented += 1;
            }
            if outcome.non_consumed_update_dynamic_flags_represented {
                summary.non_consumed_update_dynamic_flags_represented += 1;
            }
            if outcome.non_consumed_source_missing {
                summary.non_consumed_source_missing += 1;
            }
            if outcome.summoned_expired_delete {
                summary.summoned_expired_deletes += 1;
            }
            if outcome.summoned_expired_respawn_time_zeroed {
                summary.summoned_expired_respawn_time_zeroed += 1;
            }
            if outcome.summoned_expired_despawn_represented {
                summary.summoned_expired_despawn_represented += 1;
            }
            if outcome.summoned_expired_go_state_ready {
                summary.summoned_expired_go_state_ready += 1;
            }
            if outcome.new_flag_drop_owner_in_base_command_represented {
                summary.new_flag_drop_owner_in_base_commands_represented += 1;
            }
            if outcome.new_flag_drop_owner_missing_or_empty {
                summary.new_flag_drop_owner_missing_or_empty += 1;
            }
            if outcome.new_flag_drop_owner_wrong_kind {
                summary.new_flag_drop_owner_wrong_kind += 1;
            }
            if outcome.new_flag_drop_owner_not_new_flag {
                summary.new_flag_drop_owner_not_new_flag += 1;
            }
            if outcome.generic_not_ready {
                summary.generic_not_ready += 1;
            }
            if outcome.generic_capture_point_removed_represented {
                summary.generic_capture_point_removed_represented += 1;
                summary
                    .generic_capture_point_removed_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_visual_despawn_represented {
                summary.generic_visual_despawn_represented += 1;
                summary
                    .generic_visual_despawn_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_flags_restored_represented {
                summary.generic_flags_restored_represented += 1;
            }
            if outcome.generic_zero_respawn_delay_return {
                summary.generic_zero_respawn_delay_returns += 1;
            }
            if outcome.generic_despawn_at_action_source_missing {
                summary.generic_despawn_at_action_source_missing += 1;
            }
            if outcome.generic_respawn_scheduled_time.is_some() {
                summary.generic_respawn_scheduled += 1;
            }
            if outcome.generic_spawned_by_default_branch {
                summary.generic_spawned_by_default_branches += 1;
            }
            if outcome.generic_temporary_respawn_zeroed {
                summary.generic_temporary_respawn_zeroed += 1;
            }
            let map_timer_added = matches!(
                outcome.generic_respawn_timer_add,
                Some(
                    AddRespawnInfoOutcomeLikeCpp::Inserted
                        | AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
                )
            );
            if map_timer_added {
                summary.generic_respawn_timer_added += 1;
            }
            if outcome.generic_respawn_save_missing_spawn_id {
                summary.generic_respawn_save_missing_spawn_id += 1;
            }
            if outcome.generic_respawn_save_missing_gameobject_data {
                summary.generic_respawn_save_missing_gameobject_data += 1;
            }
            if outcome.generic_respawn_compatibility_db_only_represented {
                summary.generic_respawn_compatibility_db_only_represented += 1;
            }
            if (map_timer_added || outcome.generic_respawn_compatibility_db_only_represented)
                && let (Some(respawn_time), Some(game_object)) = (
                    outcome.generic_respawn_scheduled_time,
                    self.map_object_record(outcome.game_object_guid)
                        .and_then(MapObjectRecord::game_object),
                )
            {
                let position = game_object.world().position();
                summary.respawn_db_saves.push(RespawnInfoLikeCpp {
                    object_type: SpawnObjectType::GameObject,
                    spawn_id: game_object.spawn_id(),
                    entry: game_object.world().object().entry(),
                    respawn_time,
                    grid_id: compute_grid_coord(position.x, position.y).get_id(),
                });
            }
            if outcome.generic_visibility_on_destroy_represented {
                summary.generic_visibility_on_destroy_represented += 1;
                summary
                    .generic_visibility_on_destroy_guids
                    .push(outcome.game_object_guid);
            }
            match outcome.status {
                GameObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued => {
                    summary.despawn_remove_queued += 1;
                }
                GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated => {
                    summary.despawn_pool_updated += 1;
                }
                GameObjectUpdateStatusLikeCpp::MissingGameObject => summary.missing_or_stale += 1,
                GameObjectUpdateStatusLikeCpp::NotGameObject => summary.not_game_object += 1,
                GameObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }
        summary
    }

    /// Update only the canonical AreaTriggers selected by one nearby-cell plan.
    pub fn update_area_triggers_for_guids_like_cpp(
        &mut self,
        area_trigger_guids: impl IntoIterator<Item = ObjectGuid>,
        elapsed_ms: u32,
    ) -> AreaTriggersUpdateSummaryLikeCpp {
        let mut summary = AreaTriggersUpdateSummaryLikeCpp::default();
        for guid in area_trigger_guids {
            summary.visited += 1;
            let outcome = self.update_area_trigger_like_cpp(guid, elapsed_ms);
            match outcome.status {
                AreaTriggerUpdateStatusLikeCpp::Updated => summary.updated += 1,
                AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger => {
                    summary.missing_or_stale += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger => summary.not_area_trigger += 1,
                AreaTriggerUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }
        summary
    }

    /// Update only the canonical Conversations selected by one nearby-cell plan.
    pub fn update_conversations_for_guids_like_cpp(
        &mut self,
        conversation_guids: impl IntoIterator<Item = ObjectGuid>,
        elapsed_ms: u32,
    ) -> ConversationsUpdateSummaryLikeCpp {
        let mut summary = ConversationsUpdateSummaryLikeCpp::default();
        for guid in conversation_guids {
            summary.visited += 1;
            let outcome = self.update_conversation_like_cpp(guid, elapsed_ms);
            match outcome.status {
                ConversationUpdateStatusLikeCpp::Updated => summary.updated += 1,
                ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                ConversationUpdateStatusLikeCpp::MissingConversation => {
                    summary.missing_or_stale += 1;
                }
                ConversationUpdateStatusLikeCpp::NotConversation => summary.not_conversation += 1,
                ConversationUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }
        summary
    }

    /// Update only the canonical SceneObjects selected by one nearby-cell plan.
    pub fn update_scene_objects_for_guids_like_cpp<F>(
        &mut self,
        scene_object_guids: impl IntoIterator<Item = ObjectGuid>,
        elapsed_ms: u32,
        mut context_resolver: F,
    ) -> SceneObjectsUpdateSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &SceneObject) -> SceneObjectUpdateContextLikeCpp,
    {
        let scene_object_guids = scene_object_guids.into_iter().collect::<Vec<_>>();
        let mut summary = SceneObjectsUpdateSummaryLikeCpp::default();
        for guid in scene_object_guids {
            summary.visited += 1;
            let Some(context) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::scene_object)
                .map(|scene_object| context_resolver(guid, scene_object))
            else {
                let outcome = self.update_scene_object_like_cpp(
                    guid,
                    elapsed_ms,
                    SceneObjectUpdateContextLikeCpp::default(),
                );
                match outcome.status {
                    SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                        summary.missing_or_stale += 1;
                    }
                    SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                    SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
                    SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                    SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                }
                continue;
            };
            let outcome = self.update_scene_object_like_cpp(guid, elapsed_ms, context);
            match outcome.status {
                SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                    summary.missing_or_stale += 1;
                }
                SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }
        summary
    }
}
