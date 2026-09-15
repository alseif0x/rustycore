use super::super::*;
use super::*;

pub(crate) fn consume_game_event_live_update_side_effects_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    active_event_ids: &[u16],
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let actions = game_event_live_update_actions_like_cpp(
        canonical_spawn_metadata,
        outcome,
        config_event_announce,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp {
        actions,
        ..GameEventLiveUpdateSideEffectSummaryLikeCpp::default()
    };
    for action in summary.actions.clone() {
        match action {
            GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                event_id: _,
                description,
                description_len,
                announce: _,
                config_event_announce: _,
            } => {
                summary.announce_event_actions += 1;
                summary.announce_event_description_len_total += description_len;
                fanout_game_event_announcement_to_player_sessions_like_cpp(
                    player_registry,
                    &description,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::Spawn(event_id) => {
                let _ = game_event_spawn_for_event_like_cpp(
                    manager,
                    legacy_manager,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches,
                    event_id,
                );
                summary.spawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::Unspawn(event_id) => {
                let _ = game_event_unspawn_for_event_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    active_event_ids,
                    event_id,
                );
                summary.unspawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel { event_id, activate } => {
                let change_summary = game_event_change_equip_or_model_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.change_equip_or_model_actions += 1;
                summary.change_equip_or_model_records_seen +=
                    change_summary.change_equip_or_model_records_seen;
                summary.change_equip_or_model_records_applied +=
                    change_summary.change_equip_or_model_records_applied;
                summary.change_equip_or_model_missing_event_buckets +=
                    change_summary.change_equip_or_model_missing_event_buckets;
                summary.change_equip_or_model_missing_spawn_metadata +=
                    change_summary.change_equip_or_model_missing_spawn_metadata;
                summary.change_equip_or_model_missing_runtime_rows +=
                    change_summary.change_equip_or_model_missing_runtime_rows;
                summary.change_equip_or_model_maps_matched +=
                    change_summary.change_equip_or_model_maps_matched;
                summary.change_equip_or_model_live_creatures_mutated +=
                    change_summary.change_equip_or_model_live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    change_summary.change_equip_or_model_stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    change_summary.change_equip_or_model_model_validation_unavailable;
            }
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts { event_id, activate } => {
                let smart_ai_summary =
                    game_event_run_smart_ai_scripts_like_cpp(manager, event_id, activate);
                summary.run_smart_ai_actions += 1;
                summary.run_smart_ai_maps_visited += smart_ai_summary.run_smart_ai_maps_visited;
                summary.run_smart_ai_creature_candidates +=
                    smart_ai_summary.run_smart_ai_creature_candidates;
                summary.run_smart_ai_gameobject_candidates +=
                    smart_ai_summary.run_smart_ai_gameobject_candidates;
                summary.run_smart_ai_creature_ai_enabled_unrepresented +=
                    smart_ai_summary.run_smart_ai_creature_ai_enabled_unrepresented;
                summary.run_smart_ai_script_dispatch_unrepresented +=
                    smart_ai_summary.run_smart_ai_script_dispatch_unrepresented;
            }
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => {
                summary.reset_event_seasonal_quests_actions += 1;
                if event_start_time == 0 {
                    summary.reset_event_seasonal_quests_event_start_time_zero += 1;
                } else {
                    summary.reset_event_seasonal_quests_event_start_time_nonzero += 1;
                }
                game_event_seasonal_quest_db_delete_like_cpp(
                    event_id,
                    event_start_time,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests { event_id, activate } => {
                let quest_summary =
                    game_event_update_quests_like_cpp(canonical_spawn_metadata, event_id, activate);
                summary.update_event_quests_actions += 1;
                summary.update_event_quests_creature_records_seen +=
                    quest_summary.update_event_quests_creature_records_seen;
                summary.update_event_quests_gameobject_records_seen +=
                    quest_summary.update_event_quests_gameobject_records_seen;
                summary.update_event_quests_creature_inserted +=
                    quest_summary.update_event_quests_creature_inserted;
                summary.update_event_quests_gameobject_inserted +=
                    quest_summary.update_event_quests_gameobject_inserted;
                summary.update_event_quests_creature_removed +=
                    quest_summary.update_event_quests_creature_removed;
                summary.update_event_quests_gameobject_removed +=
                    quest_summary.update_event_quests_gameobject_removed;
                summary.update_event_quests_creature_remove_misses +=
                    quest_summary.update_event_quests_creature_remove_misses;
                summary.update_event_quests_gameobject_remove_misses +=
                    quest_summary.update_event_quests_gameobject_remove_misses;
                summary.update_event_quests_creature_no_match +=
                    quest_summary.update_event_quests_creature_no_match;
                summary.update_event_quests_gameobject_no_match +=
                    quest_summary.update_event_quests_gameobject_no_match;
                summary.update_event_quests_creature_missing_event_buckets +=
                    quest_summary.update_event_quests_creature_missing_event_buckets;
                summary.update_event_quests_gameobject_missing_event_buckets +=
                    quest_summary.update_event_quests_gameobject_missing_event_buckets;
                summary.update_event_quests_creature_skipped_active_other_event +=
                    quest_summary.update_event_quests_creature_skipped_active_other_event;
                summary.update_event_quests_gameobject_skipped_active_other_event +=
                    quest_summary.update_event_quests_gameobject_skipped_active_other_event;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates { event_id, activate } => {
                let world_state_summary = game_event_update_world_states_like_cpp(
                    canonical_spawn_metadata,
                    battlemaster_list_store,
                    world_state_mgr.as_deref_mut(),
                    player_registry,
                    event_id,
                    activate,
                );
                summary.update_world_states_actions += 1;
                summary.update_world_states_no_holiday +=
                    world_state_summary.update_world_states_no_holiday;
                summary.update_world_states_missing_event +=
                    world_state_summary.update_world_states_missing_event;
                summary.update_world_states_store_missing +=
                    world_state_summary.update_world_states_store_missing;
                summary.update_world_states_holiday_not_weekend_battleground +=
                    world_state_summary.update_world_states_holiday_not_weekend_battleground;
                summary.update_world_states_battlemaster_list_missing +=
                    world_state_summary.update_world_states_battlemaster_list_missing;
                summary.update_world_states_holiday_world_state_zero +=
                    world_state_summary.update_world_states_holiday_world_state_zero;
                summary.update_world_states_holiday_lookup_unrepresented +=
                    world_state_summary.update_world_states_holiday_lookup_unrepresented;
                summary.update_world_states_set_value_represented +=
                    world_state_summary.update_world_states_set_value_represented;
                summary.update_world_states_set_value_attempts +=
                    world_state_summary.update_world_states_set_value_attempts;
                summary.update_world_states_realm_changed_or_inserted +=
                    world_state_summary.update_world_states_realm_changed_or_inserted;
                summary.update_world_states_realm_unchanged_noop +=
                    world_state_summary.update_world_states_realm_unchanged_noop;
                summary.update_world_states_map_specific_no_map_unsupported +=
                    world_state_summary.update_world_states_map_specific_no_map_unsupported;
                summary.update_world_states_global_message_represented +=
                    world_state_summary.update_world_states_global_message_represented;
                summary.update_world_states_global_message_registry_missing +=
                    world_state_summary.update_world_states_global_message_registry_missing;
                summary.update_world_states_global_message_send_attempted +=
                    world_state_summary.update_world_states_global_message_send_attempted;
                summary.update_world_states_global_message_send_queued +=
                    world_state_summary.update_world_states_global_message_send_queued;
                summary.update_world_states_global_message_send_failed +=
                    world_state_summary.update_world_states_global_message_send_failed;
                summary.update_world_states_global_message_not_in_world_skipped +=
                    world_state_summary.update_world_states_global_message_not_in_world_skipped;
                summary.update_world_states_last_world_state_id =
                    world_state_summary.update_world_states_last_world_state_id;
                summary.update_world_states_last_world_state_value =
                    world_state_summary.update_world_states_last_world_state_value;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id } => {
                let npc_flag_summary = game_event_update_npc_flags_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches.template_store.as_ref(),
                    player_registry,
                    event_id,
                    active_event_ids,
                );
                summary.update_npc_flags_actions += 1;
                summary.update_npc_flags_records_seen +=
                    npc_flag_summary.update_npc_flags_records_seen;
                summary.update_npc_flags_missing_event_buckets +=
                    npc_flag_summary.update_npc_flags_missing_event_buckets;
                summary.update_npc_flags_missing_spawn_metadata +=
                    npc_flag_summary.update_npc_flags_missing_spawn_metadata;
                summary.update_npc_flags_template_npcflag_missing +=
                    npc_flag_summary.update_npc_flags_template_npcflag_missing;
                summary.update_npc_flags_maps_matched +=
                    npc_flag_summary.update_npc_flags_maps_matched;
                summary.update_npc_flags_indexed_guids +=
                    npc_flag_summary.update_npc_flags_indexed_guids;
                summary.update_npc_flags_live_creatures_mutated +=
                    npc_flag_summary.update_npc_flags_live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    npc_flag_summary.update_npc_flags_stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied +=
                    npc_flag_summary.update_npc_flags_low_applied;
                summary.update_npc_flags2_applied += npc_flag_summary.update_npc_flags2_applied;
                summary.update_npc_flags_values_updates_built +=
                    npc_flag_summary.update_npc_flags_values_updates_built;
                summary.update_npc_flags_values_update_empty +=
                    npc_flag_summary.update_npc_flags_values_update_empty;
                summary.update_npc_flags_values_update_map_id_out_of_range +=
                    npc_flag_summary.update_npc_flags_values_update_map_id_out_of_range;
                summary.update_npc_flags_values_update_registry_missing +=
                    npc_flag_summary.update_npc_flags_values_update_registry_missing;
                summary.update_npc_flags_values_update_not_in_world_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_not_in_world_skipped;
                summary.update_npc_flags_values_update_wrong_map_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_wrong_map_skipped;
                summary.update_npc_flags_values_update_send_attempted +=
                    npc_flag_summary.update_npc_flags_values_update_send_attempted;
                summary.update_npc_flags_values_update_send_queued +=
                    npc_flag_summary.update_npc_flags_values_update_send_queued;
                summary.update_npc_flags_values_update_send_failed +=
                    npc_flag_summary.update_npc_flags_values_update_send_failed;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor { event_id, activate } => {
                let npc_vendor_summary = game_event_update_npc_vendor_like_cpp(
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.update_npc_vendor_actions += 1;
                summary.update_npc_vendor_records_seen +=
                    npc_vendor_summary.update_npc_vendor_records_seen;
                summary.update_npc_vendor_items_added +=
                    npc_vendor_summary.update_npc_vendor_items_added;
                summary.update_npc_vendor_items_removed +=
                    npc_vendor_summary.update_npc_vendor_items_removed;
                summary.update_npc_vendor_missing_event_buckets +=
                    npc_vendor_summary.update_npc_vendor_missing_event_buckets;
                summary.update_npc_vendor_remove_misses +=
                    npc_vendor_summary.update_npc_vendor_remove_misses;
                summary.update_npc_vendor_no_match += npc_vendor_summary.update_npc_vendor_no_match;
            }
        }
    }
    summary
}
