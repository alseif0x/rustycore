//! Ordered startup composition for the canonical C++ game-event system.

use anyhow::Result;
use tracing::debug;

use crate::{
    consume_game_event_live_update_side_effects_like_cpp,
    current_unix_time_secs_like_cpp,
    execute_game_event_seasonal_quest_db_deletes_like_cpp,
    execute_game_event_world_event_state_db_bridge_like_cpp,
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp,
    materialize_game_event_world_event_state_db_bridge_like_cpp,
    represented_game_event_world_conditions_met_like_cpp,
    CanonicalGameEventSchedulerLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
    SharedCanonicalMapManager, SharedCanonicalSpawnMetadataLikeCpp, SharedMapManager,
    SharedWorldStateMgrLikeCpp,
};

pub(super) async fn start_system(
    canonical_spawn_metadata: &SharedCanonicalSpawnMetadataLikeCpp,
    canonical_map_manager: &SharedCanonicalMapManager,
    shared_map: &SharedMapManager,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    battlemaster_list_typed_store: &wow_data::BattlemasterListStore,
    world_state_mgr: &SharedWorldStateMgrLikeCpp,
    player_registry: &crate::PlayerRegistry,
    game_event_persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
) -> Result<CanonicalGameEventSchedulerLikeCpp> {
    let game_event_scheduler = {
        let current_time_secs = current_unix_time_secs_like_cpp();
        let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!(
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem"
                )
            })?;
            canonical_spawn_metadata.clear_active_game_events_like_cpp();
            let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                current_time_secs,
                false,
                represented_game_event_world_conditions_met_like_cpp,
            );
            let db_bridge_summary = materialize_game_event_world_event_state_db_bridge_like_cpp(
                &outcome,
                &canonical_spawn_metadata,
            );
            let active_event_ids = canonical_spawn_metadata
                .game_event_active_set_like_cpp()
                .active_event_ids_like_cpp()
                .collect::<Vec<_>>();
            (outcome, active_event_ids, db_bridge_summary)
        };
        execute_game_event_world_event_state_db_bridge_like_cpp(
            game_event_persistence,
            &mut db_bridge_summary,
        )
        .await;
        let mut side_effect_summary = {
            let mut manager = canonical_map_manager.lock().map_err(|_| {
                anyhow::anyhow!("Canonical MapManager mutex poisoned during GameEvent StartSystem")
            })?;
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!("CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem side effects")
            })?;
            let mut world_state_mgr = world_state_mgr.lock().map_err(|_| {
                anyhow::anyhow!(
                    "WorldStateMgrLikeCpp mutex poisoned during GameEvent StartSystem side effects"
                )
            })?;
            consume_game_event_live_update_side_effects_like_cpp(
                &mut manager,
                Some(shared_map),
                &mut canonical_spawn_metadata,
                loaded_grid_creature_respawn_caches,
                Some(battlemaster_list_typed_store),
                Some(&mut world_state_mgr),
                Some(player_registry),
                &active_event_ids,
                &game_event_outcome,
                false,
            )
        };
        execute_game_event_seasonal_quest_db_deletes_like_cpp(
            game_event_persistence,
            &mut side_effect_summary,
        )
        .await;
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            Some(player_registry),
            &mut side_effect_summary,
        );
        debug!(
            scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
            queued_activation_event_ids = game_event_outcome.queued_activation_event_ids.len(),
            queued_deactivation_event_ids = game_event_outcome.queued_deactivation_event_ids.len(),
            start_outcomes = game_event_outcome.start_outcomes.len(),
            stop_outcomes = game_event_outcome.stop_outcomes.len(),
            negative_spawn_event_ids = game_event_outcome.negative_spawn_event_ids.len(),
            world_nextphase_finished = game_event_outcome.world_nextphase_finished.len(),
            world_conditions_save_requested =
                game_event_outcome.world_conditions_save_requested.len(),
            game_event_db_saves_queued = db_bridge_summary.saves_queued,
            game_event_db_saves_executed = db_bridge_summary.saves_executed,
            game_event_db_saves_failed = db_bridge_summary.saves_failed,
            game_event_db_saves_skipped_event_id_out_of_range =
                db_bridge_summary.saves_skipped_event_id_out_of_range,
            game_event_db_saves_skipped_missing_event =
                db_bridge_summary.saves_skipped_missing_event,
            game_event_db_deletes_queued = db_bridge_summary.deletes_queued,
            game_event_db_deletes_executed = db_bridge_summary.deletes_executed,
            game_event_db_deletes_failed = db_bridge_summary.deletes_failed,
            game_event_db_deletes_skipped_event_id_out_of_range =
                db_bridge_summary.deletes_skipped_event_id_out_of_range,
            game_event_db_condition_delete_rows_queued =
                db_bridge_summary.condition_delete_rows_queued,
            game_event_db_condition_delete_rows_executed =
                db_bridge_summary.condition_delete_rows_executed,
            game_event_db_condition_delete_rows_failed =
                db_bridge_summary.condition_delete_rows_failed,
            invalid_check_outcomes = game_event_outcome.invalid_check_outcomes.len(),
            invalid_next_check_outcomes = game_event_outcome.invalid_next_check_outcomes.len(),
            next_update_delay_millis = game_event_outcome.next_update_delay_millis,
            side_effect_actions = side_effect_summary.actions.len(),
            spawn_actions = side_effect_summary.spawn_actions,
            unspawn_actions = side_effect_summary.unspawn_actions,
            announce_event_actions = side_effect_summary.announce_event_actions,
            announce_event_description_len_total =
                side_effect_summary.announce_event_description_len_total,
            announce_event_world_text_represented =
                side_effect_summary.announce_event_world_text_represented,
            announce_event_lines = side_effect_summary.announce_event_lines,
            announce_event_registry_missing = side_effect_summary.announce_event_registry_missing,
            announce_event_send_attempted = side_effect_summary.announce_event_send_attempted,
            announce_event_send_queued = side_effect_summary.announce_event_send_queued,
            announce_event_send_failed = side_effect_summary.announce_event_send_failed,
            announce_event_localization_unrepresented =
                side_effect_summary.announce_event_localization_unrepresented,
            announce_event_in_world_filter_unrepresented =
                side_effect_summary.announce_event_in_world_filter_unrepresented,
            announce_event_not_in_world_skipped =
                side_effect_summary.announce_event_not_in_world_skipped,
            announce_event_world_text_unimplemented =
                side_effect_summary.announce_event_world_text_unimplemented,
            announce_event_session_fanout_unimplemented =
                side_effect_summary.announce_event_session_fanout_unimplemented,
            change_equip_or_model_actions = side_effect_summary.change_equip_or_model_actions,
            change_equip_or_model_records_seen =
                side_effect_summary.change_equip_or_model_records_seen,
            change_equip_or_model_records_applied =
                side_effect_summary.change_equip_or_model_records_applied,
            change_equip_or_model_maps_matched =
                side_effect_summary.change_equip_or_model_maps_matched,
            change_equip_or_model_live_creatures_mutated =
                side_effect_summary.change_equip_or_model_live_creatures_mutated,
            change_equip_or_model_model_validation_unavailable =
                side_effect_summary.change_equip_or_model_model_validation_unavailable,
            update_event_quests_actions = side_effect_summary.update_event_quests_actions,
            update_event_quests_creature_records_seen =
                side_effect_summary.update_event_quests_creature_records_seen,
            update_event_quests_gameobject_records_seen =
                side_effect_summary.update_event_quests_gameobject_records_seen,
            update_event_quests_creature_inserted =
                side_effect_summary.update_event_quests_creature_inserted,
            update_event_quests_gameobject_inserted =
                side_effect_summary.update_event_quests_gameobject_inserted,
            update_event_quests_creature_removed =
                side_effect_summary.update_event_quests_creature_removed,
            update_event_quests_gameobject_removed =
                side_effect_summary.update_event_quests_gameobject_removed,
            update_event_quests_creature_skipped_active_other_event =
                side_effect_summary.update_event_quests_creature_skipped_active_other_event,
            update_event_quests_gameobject_skipped_active_other_event =
                side_effect_summary.update_event_quests_gameobject_skipped_active_other_event,
            update_world_states_actions = side_effect_summary.update_world_states_actions,
            update_world_states_no_holiday = side_effect_summary.update_world_states_no_holiday,
            update_world_states_missing_event =
                side_effect_summary.update_world_states_missing_event,
            update_world_states_store_missing = side_effect_summary.update_world_states_store_missing,
            update_world_states_holiday_not_weekend_battleground =
                side_effect_summary.update_world_states_holiday_not_weekend_battleground,
            update_world_states_battlemaster_list_missing =
                side_effect_summary.update_world_states_battlemaster_list_missing,
            update_world_states_holiday_world_state_zero =
                side_effect_summary.update_world_states_holiday_world_state_zero,
            update_world_states_holiday_lookup_unrepresented =
                side_effect_summary.update_world_states_holiday_lookup_unrepresented,
            update_world_states_set_value_represented =
                side_effect_summary.update_world_states_set_value_represented,
            update_world_states_last_world_state_id =
                side_effect_summary.update_world_states_last_world_state_id,
            update_world_states_last_world_state_value =
                side_effect_summary.update_world_states_last_world_state_value,
            update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
            update_npc_flags_records_seen = side_effect_summary.update_npc_flags_records_seen,
            update_npc_flags_maps_matched = side_effect_summary.update_npc_flags_maps_matched,
            update_npc_flags_live_creatures_mutated =
                side_effect_summary.update_npc_flags_live_creatures_mutated,
            update_npc_flags2_applied =
                side_effect_summary.update_npc_flags2_applied,
            update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
            update_npc_vendor_records_seen = side_effect_summary.update_npc_vendor_records_seen,
            update_npc_vendor_items_added = side_effect_summary.update_npc_vendor_items_added,
            update_npc_vendor_items_removed = side_effect_summary.update_npc_vendor_items_removed,
            update_npc_vendor_missing_event_buckets =
                side_effect_summary.update_npc_vendor_missing_event_buckets,
            update_npc_vendor_remove_misses = side_effect_summary.update_npc_vendor_remove_misses,
            update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
            reset_event_seasonal_quests_actions =
                side_effect_summary.reset_event_seasonal_quests_actions,
            reset_event_seasonal_quests_event_start_time_zero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
            reset_event_seasonal_quests_event_start_time_nonzero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
            reset_event_seasonal_quests_player_session_runtime_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_player_session_runtime_unimplemented,
            reset_event_seasonal_quests_character_db_statement_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_character_db_statement_unimplemented,
            reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_queued,
            reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_executed,
            reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_failed,
            reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
            "Represented C++ GameEventMgr::StartSystem: cleared active events, ran first Update with isSystemInit=false, installed WUPDATE_EVENTS delay, and consumed safe represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; real SendWorldText/session fanout, full ConditionMgr world-event runtime, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
        );
        CanonicalGameEventSchedulerLikeCpp::start_system(
            game_event_outcome.next_update_delay_millis,
        )
    };
    Ok(game_event_scheduler)
}
