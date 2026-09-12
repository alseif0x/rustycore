//! Canonical World/Map phase producer and its shutdown boundary.
use std::collections::BTreeSet;
use std::sync::{Arc, atomic::Ordering};
use std::time::{Duration, Instant};

use tracing::debug;
use wow_persistence::GameEventPersistencePortLikeCpp;

use super::super::{deferred_visibility, map_session_pass, world_session_pass};
use super::{
    CanonicalGameEventSchedulerLikeCpp, CanonicalRespawnConditionSchedulerLikeCpp,
    LoadedGridCreatureRespawnCachesLikeCpp, PlayerRegistry, RespawnDbWriterSenderLikeCpp,
    SharedCanonicalMapManager, SharedCanonicalSpawnMetadataLikeCpp, SharedMapManager,
    SharedRespawnDbMutationOrderLikeCpp, SharedRespawnDbProducerStopLikeCpp,
    SharedWorldStateMgrLikeCpp, canonical_map_coordinator_id_like_cpp,
    canonical_map_tick_begin_like_cpp, canonical_map_tick_resume_like_cpp,
    consume_game_event_live_update_side_effects_like_cpp, current_unix_time_secs_like_cpp,
    execute_game_event_seasonal_quest_db_deletes_like_cpp,
    execute_game_event_world_event_state_db_bridge_like_cpp,
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp,
    load_loaded_grid_area_triggers_like_cpp,
    materialize_game_event_world_event_state_db_bridge_like_cpp,
    represented_game_event_world_conditions_met_like_cpp, warn_about_sync_queries_scope_like_cpp,
};

pub(crate) fn spawn_canonical_map_update_loop(
    map_manager: SharedCanonicalMapManager,
    legacy_map_manager: SharedMapManager,
    tick_interval_ms: u32,
    respawn_condition_interval_ms: u32,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    map_store: Arc<wow_data::MapStore>,
    game_event_persistence: Arc<dyn GameEventPersistencePortLikeCpp>,
    respawn_db_writer_tx: RespawnDbWriterSenderLikeCpp,
    respawn_db_mutation_order: SharedRespawnDbMutationOrderLikeCpp,
    respawn_db_producer_stop: SharedRespawnDbProducerStopLikeCpp,
    loaded_grid_creature_respawn_caches: LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: Arc<wow_data::AreaTriggerTemplateStore>,
    mut game_event_scheduler: CanonicalGameEventSchedulerLikeCpp,
    player_registry: Arc<PlayerRegistry>,
    active_session_registry: Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    battlemaster_list_store: Arc<wow_data::BattlemasterListStore>,
    world_state_mgr: SharedWorldStateMgrLikeCpp,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut last_tick = Instant::now();
        // Identifies one split tick, so a completion that arrives after its
        // tick resumed cannot be counted as this tick's (#787). The coordinator
        // id distinguishes this loop from any other that could admit a pass, so
        // a matching epoch from a different coordinator is still refused.
        let mut tick_epoch: u64 = 0;
        let coordinator_id = canonical_map_coordinator_id_like_cpp();
        // Passes whose effects this producer could not account for. While any of
        // them is unresolved, no phase is issued and no tick is admitted: an
        // operation of unknown extent is not made safe by starting the next
        // step over it, and neither respawns nor DelayedUpdate may run on that
        // uncertainty.
        let mut unresolved_phase_permits: Vec<
            std::sync::Arc<wow_world::session::mailbox::SessionPhasePermitLikeCpp>,
        > = Vec::new();
        let mut respawn_condition_scheduler =
            CanonicalRespawnConditionSchedulerLikeCpp::new(respawn_condition_interval_ms);
        loop {
            interval.tick().await;
            let stop_after_tick = respawn_db_producer_stop.load(Ordering::Acquire);

            // Shutdown has handed session control to their task owners. Do not
            // admit simulation over that finalization. The existing final
            // respawn tick is requested only after the session drain.
            if active_session_registry.is_shutting_down_like_cpp() {
                if !stop_after_tick {
                    continue;
                }
                if active_session_registry.len_like_cpp() != 0 {
                    // Shutdown already reports the failed session drain. A
                    // retained finalizer cannot authorize a final simulation tick.
                    break;
                }
            }

            let now = Instant::now();
            let mut diff_ms = now
                .duration_since(last_tick)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            last_tick = now;

            if diff_ms == 0 {
                if !stop_after_tick {
                    continue;
                }
                diff_ms = 1;
            }

            // The barrier is re-read before anything else: a session that
            // finally resolved its pass releases it, and nothing else does.
            world_session_pass::retain_unresolved_phase_permits_like_cpp(
                &mut unresolved_phase_permits,
            );
            if !unresolved_phase_permits.is_empty() {
                tracing::error!(
                    tick_epoch,
                    unresolved = unresolved_phase_permits.len(),
                    "Canonical producer barrier held: admitted passes have not accounted for their effects"
                );
                continue;
            }

            tick_epoch = tick_epoch.wrapping_add(1);
            let phase_ack_timeout = Duration::from_millis(u64::from(tick_interval_ms.max(1)));

            // C++ `World::Update` runs `UpdateSessions(diff)` (`World.cpp:2704`)
            // over every session — character screen included — and only then
            // `MapManager::Update(diff)` (`World.cpp:2748`). The world pass
            // therefore completes before this step admits a map tick, and no
            // session drives itself in between.
            let world_pass_summary = world_session_pass::run_world_phase_session_passes_like_cpp(
                &active_session_registry.world_phase_participants_like_cpp(),
                coordinator_id,
                tick_epoch,
                diff_ms,
                phase_ack_timeout,
            )
            .await;
            if !world_pass_summary.quiescent_like_cpp() {
                // A claimed world pass whose end is unknown may still be
                // mutating. Admitting a map tick over it would overlap exactly
                // what C++ serializes, and so would the next step, so the
                // barrier is retained until those permits resolve.
                tracing::error!(
                    tick_epoch,
                    unresolved_after_start = world_pass_summary.unresolved_after_start,
                    "Skipping the canonical map tick: a world-phase pass left live effects"
                );
                unresolved_phase_permits.extend(world_pass_summary.unresolved_permits);
                continue;
            }
            if world_pass_summary.stalled_ms > 0 || world_pass_summary.send_failed > 0 {
                debug!(
                    tick_epoch,
                    participants = world_pass_summary.participants,
                    completed = world_pass_summary.completed,
                    revoked_before_start = world_pass_summary.revoked_before_start,
                    send_failed = world_pass_summary.send_failed,
                    stalled_ms = world_pass_summary.stalled_ms,
                    "The world phase of this step did not run for every session"
                );
            }
            if active_session_registry.is_shutting_down_like_cpp() && !stop_after_tick {
                continue;
            }

            let (area_trigger_sweep_summary, session_plan) = {
                // Canonical and legacy respawn mutations share this ordering
                // gate. Statements are coalesced before releasing it, so
                // mailbox replacement order is the same as mutation order even
                // when this loop later awaits unrelated game-event DB work.
                let _respawn_db_mutation_order = respawn_db_mutation_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Ok(mut manager) = map_manager.lock() else {
                    tracing::error!(
                        "Canonical MapManager mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                    tracing::error!(
                        "CanonicalSpawnMetadataLikeCpp mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let area_trigger_sweep_summary = load_loaded_grid_area_triggers_like_cpp(
                    &mut manager,
                    &canonical_spawn_metadata,
                    area_trigger_template_store.as_ref(),
                );
                let session_plan = canonical_map_tick_begin_like_cpp(&mut manager, diff_ms);
                drop(canonical_spawn_metadata);
                drop(manager);
                (area_trigger_sweep_summary, session_plan)
            };

            // #787: every synchronous guard of the split is released above,
            // including the respawn mutation-order gate, before any session
            // request is delivered or any completion awaited. C++ drives these
            // sessions inside `Map::Update` (`Maps/Map.cpp:669-680`); RustyCore
            // asks each admitted session to run that pass in its own task and
            // waits for its completion boundary here.
            let session_pass_summary = if let Some(plan) = session_plan.as_ref() {
                map_session_pass::run_map_phase_session_passes_like_cpp(
                    &plan.participants,
                    player_registry.as_ref(),
                    coordinator_id,
                    tick_epoch,
                    plan.plan.effective_diff_ms(),
                    phase_ack_timeout,
                )
                .await
            } else {
                map_session_pass::MapSessionPassSummaryLikeCpp::default()
            };
            if session_pass_summary.stalled_ms > 0 || session_pass_summary.send_failed > 0 {
                debug!(
                    tick_epoch,
                    participants = session_pass_summary.participants,
                    completed = session_pass_summary.completed,
                    revoked_before_start = session_pass_summary.revoked_before_start,
                    refused_before_start = session_pass_summary.refused_before_start,
                    send_failed = session_pass_summary.send_failed,
                    unaddressable = session_pass_summary.unaddressable,
                    stalled_ms = session_pass_summary.stalled_ms,
                    "Canonical map tick waited past its deadline for an admitted session pass"
                );
            }

            let tick_summary = {
                let Some(plan) = session_plan else {
                    continue;
                };
                if !session_pass_summary.quiescent_like_cpp() {
                    unresolved_phase_permits
                        .extend(session_pass_summary.unresolved_permits.clone());
                    // An admitted pass was claimed and this tick could not
                    // observe its end. Its mutations may still be running, so
                    // the remaining phases of the tick must not run over them:
                    // the tick is abandoned, with the unresolved permit retained
                    // across steps until completion is actually established.
                    tracing::error!(
                        tick_epoch,
                        unresolved_after_start = session_pass_summary.unresolved_after_start,
                        "Abandoning the canonical map tick: an admitted session pass left live effects"
                    );
                    let Ok(mut manager) = map_manager.lock() else {
                        tracing::error!(
                            "Canonical MapManager mutex poisoned; stopping map update loop"
                        );
                        break;
                    };
                    manager.abandon_tick_like_cpp(plan.plan);
                    drop(manager);
                    continue;
                }
                if active_session_registry.is_shutting_down_like_cpp() && !stop_after_tick {
                    if let Ok(mut manager) = map_manager.lock() {
                        manager.abandon_tick_like_cpp(plan.plan);
                    }
                    continue;
                }
                // The persistence fence is re-taken for the half that produces
                // respawn mutations, and its statements are still coalesced
                // before it is released, so mailbox replacement order still
                // matches mutation order.
                let _respawn_db_mutation_order = respawn_db_mutation_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Ok(mut manager) = map_manager.lock() else {
                    tracing::error!(
                        "Canonical MapManager mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                    tracing::error!(
                        "CanonicalSpawnMetadataLikeCpp mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let mut tick_summary = canonical_map_tick_resume_like_cpp(
                    &mut manager,
                    Some(&legacy_map_manager),
                    plan.plan,
                    &mut respawn_condition_scheduler,
                    &canonical_spawn_metadata,
                    condition_store.as_ref(),
                    map_store.as_ref(),
                    &loaded_grid_creature_respawn_caches,
                );
                drop(canonical_spawn_metadata);
                drop(manager);

                if let Some(summary) = tick_summary.as_mut() {
                    let mut reconciled_players = BTreeSet::new();
                    for (map_id, instance_id, owner_guid, target_guid) in
                        summary.expired_pvp_combat_refs.drain(..)
                    {
                        let Ok(map_id) = u16::try_from(map_id) else {
                            continue;
                        };
                        for player_guid in [owner_guid, target_guid] {
                            if !reconciled_players.insert((map_id, instance_id, player_guid)) {
                                continue;
                            }
                            let Some(recipient) = player_registry.runtime_recipient(player_guid)
                            else {
                                continue;
                            };
                            let command =
                                wow_world::session::mailbox::ReconcilePvpCombatExpiryLikeCppCommand {
                                    player_guid,
                                    map_id,
                                    instance_id,
                                };
                            player_registry
                                .publish_current_pvp_combat_expiry(recipient.registration, command);
                        }
                    }
                    for save in summary.respawn_db_saves.drain(..) {
                        if respawn_db_writer_tx.send(save.mutation).is_err() {
                            summary.respawn_db_save_failed += 1;
                            tracing::error!(
                                "Shared respawn DB writer stopped before canonical REP_RESPAWN submission"
                            );
                        }
                    }
                    // A timer can be created and consumed in one canonical
                    // update. Submit deletes last so the final state wins.
                    for delete in summary.respawn_db_deletes.drain(..) {
                        if respawn_db_writer_tx.send(delete.mutation).is_err() {
                            summary.respawn_db_delete_failed += 1;
                            tracing::error!(
                                "Shared respawn DB writer stopped before canonical DEL_RESPAWN submission"
                            );
                        }
                    }
                }

                tick_summary
            };

            if let Some(summary) = tick_summary.as_ref() {
                deferred_visibility::deliver_deferred_player_visibility_like_cpp(
                    &summary.player_visibility_refresh_intents,
                    &player_registry,
                );
            }

            if area_trigger_sweep_summary.loaded_grid_primary_records > 0
                || area_trigger_sweep_summary.load_record_missing > 0
                || area_trigger_sweep_summary.add_to_map_errors > 0
            {
                debug!(
                    maps_evaluated = area_trigger_sweep_summary.maps_evaluated,
                    loaded_grids_evaluated = area_trigger_sweep_summary.loaded_grids_evaluated,
                    metadata_entries = area_trigger_sweep_summary.metadata_entries,
                    skipped_already_loaded = area_trigger_sweep_summary.skipped_already_loaded,
                    skipped_should_not_spawn = area_trigger_sweep_summary.skipped_should_not_spawn,
                    stale_index_entries = area_trigger_sweep_summary.stale_index_entries,
                    skipped_difficulty_mismatch =
                        area_trigger_sweep_summary.skipped_difficulty_mismatch,
                    load_record_missing = area_trigger_sweep_summary.load_record_missing,
                    loaded_grid_primary_records =
                        area_trigger_sweep_summary.loaded_grid_primary_records,
                    add_to_map_errors = area_trigger_sweep_summary.add_to_map_errors,
                    "C++ ObjectGridLoader AreaTrigger loaded-grid sweep materialized canonical AreaTrigger records for already-loaded grids; ObjectAccessor/fanout/scripts/actions/dynamic-tree runtime remain pending"
                );
            }

            if !stop_after_tick && game_event_scheduler.update(diff_ms) {
                let current_time_secs = current_unix_time_secs_like_cpp();
                let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent update; stopping map update loop"
                        );
                        break;
                    };
                    let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                        current_time_secs,
                        true,
                        represented_game_event_world_conditions_met_like_cpp,
                    );
                    game_event_scheduler.set_interval_and_reset(outcome.next_update_delay_millis);
                    let db_bridge_summary =
                        materialize_game_event_world_event_state_db_bridge_like_cpp(
                            &outcome,
                            &canonical_spawn_metadata,
                        );
                    let active_event_ids = canonical_spawn_metadata
                        .game_event_active_set_like_cpp()
                        .active_event_ids_like_cpp()
                        .collect::<Vec<_>>();
                    (outcome, active_event_ids, db_bridge_summary)
                };
                warn_about_sync_queries_scope_like_cpp(
                    execute_game_event_world_event_state_db_bridge_like_cpp(
                        game_event_persistence.as_ref(),
                        &mut db_bridge_summary,
                    ),
                )
                .await;
                let mut side_effect_summary = {
                    let Ok(mut manager) = map_manager.lock() else {
                        tracing::error!(
                            "Canonical MapManager mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut world_state_mgr) = world_state_mgr.lock() else {
                        tracing::error!(
                            "WorldStateMgrLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    consume_game_event_live_update_side_effects_like_cpp(
                        &mut manager,
                        Some(&legacy_map_manager),
                        &mut canonical_spawn_metadata,
                        &loaded_grid_creature_respawn_caches,
                        Some(battlemaster_list_store.as_ref()),
                        Some(&mut world_state_mgr),
                        Some(player_registry.as_ref()),
                        &active_event_ids,
                        &game_event_outcome,
                        false,
                    )
                };
                warn_about_sync_queries_scope_like_cpp(
                    execute_game_event_seasonal_quest_db_deletes_like_cpp(
                        game_event_persistence.as_ref(),
                        &mut side_effect_summary,
                    ),
                )
                .await;
                fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
                    Some(player_registry.as_ref()),
                    &mut side_effect_summary,
                );
                debug!(
                    scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
                    queued_activation_event_ids =
                        game_event_outcome.queued_activation_event_ids.len(),
                    queued_deactivation_event_ids =
                        game_event_outcome.queued_deactivation_event_ids.len(),
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
                    invalid_next_check_outcomes =
                        game_event_outcome.invalid_next_check_outcomes.len(),
                    next_update_delay_millis = game_event_outcome.next_update_delay_millis,
                    side_effect_actions = side_effect_summary.actions.len(),
                    spawn_actions = side_effect_summary.spawn_actions,
                    unspawn_actions = side_effect_summary.unspawn_actions,
                    announce_event_actions = side_effect_summary.announce_event_actions,
                    announce_event_description_len_total =
                        side_effect_summary.announce_event_description_len_total,
                    announce_event_world_text_unimplemented =
                        side_effect_summary.announce_event_world_text_unimplemented,
                    announce_event_session_fanout_unimplemented =
                        side_effect_summary.announce_event_session_fanout_unimplemented,
                    change_equip_or_model_actions =
                        side_effect_summary.change_equip_or_model_actions,
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
                    update_event_quests_creature_remove_misses =
                        side_effect_summary.update_event_quests_creature_remove_misses,
                    update_event_quests_gameobject_remove_misses =
                        side_effect_summary.update_event_quests_gameobject_remove_misses,
                    update_event_quests_creature_skipped_active_other_event =
                        side_effect_summary.update_event_quests_creature_skipped_active_other_event,
                    update_event_quests_gameobject_skipped_active_other_event = side_effect_summary
                        .update_event_quests_gameobject_skipped_active_other_event,
                    update_world_states_actions = side_effect_summary.update_world_states_actions,
                    update_world_states_no_holiday =
                        side_effect_summary.update_world_states_no_holiday,
                    update_world_states_missing_event =
                        side_effect_summary.update_world_states_missing_event,
                    update_world_states_holiday_lookup_unrepresented =
                        side_effect_summary.update_world_states_holiday_lookup_unrepresented,
                    update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
                    update_npc_flags_records_seen =
                        side_effect_summary.update_npc_flags_records_seen,
                    update_npc_flags_maps_matched =
                        side_effect_summary.update_npc_flags_maps_matched,
                    update_npc_flags_live_creatures_mutated =
                        side_effect_summary.update_npc_flags_live_creatures_mutated,
                    update_npc_flags2_applied =
                        side_effect_summary.update_npc_flags2_applied,
                    update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
                    update_npc_vendor_records_seen =
                        side_effect_summary.update_npc_vendor_records_seen,
                    update_npc_vendor_items_added =
                        side_effect_summary.update_npc_vendor_items_added,
                    update_npc_vendor_items_removed =
                        side_effect_summary.update_npc_vendor_items_removed,
                    update_npc_vendor_missing_event_buckets =
                        side_effect_summary.update_npc_vendor_missing_event_buckets,
                    update_npc_vendor_remove_misses =
                        side_effect_summary.update_npc_vendor_remove_misses,
                    update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
                    reset_event_seasonal_quests_actions =
                        side_effect_summary.reset_event_seasonal_quests_actions,
                    reset_event_seasonal_quests_event_start_time_zero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
                    reset_event_seasonal_quests_event_start_time_nonzero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
                    reset_event_seasonal_quests_player_session_runtime_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_player_session_runtime_unimplemented,
                    reset_event_seasonal_quests_character_db_statement_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_character_db_statement_unimplemented,
                    reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_queued,
                    reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_executed,
                    reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_failed,
                    reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
                    "C++ WUPDATE_EVENTS represented timer fired; updated canonical GameEvent metadata and consumed represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; ConditionMgr world-event rows, real SendWorldText/session fanout, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
                );
            }

            if let Some(summary) = tick_summary {
                debug!(
                    maps_evaluated = summary.maps_evaluated,
                    outcomes = summary.outcomes,
                    applied_set_inactive = summary.applied_set_inactive,
                    planned_spawn = summary.planned_spawn,
                    condition_spawn_executed_loaded_grid_spawns =
                        summary.condition_spawn_executed_loaded_grid_spawns,
                    condition_spawn_blocked_loaded_grid_spawn_loads =
                        summary.condition_spawn_blocked_loaded_grid_spawn_loads,
                    condition_spawn_blocked_loaded_grid_creature_loads =
                        summary.condition_spawn_blocked_loaded_grid_creature_loads,
                    condition_spawn_blocked_loaded_grid_gameobject_loads =
                        summary.condition_spawn_blocked_loaded_grid_gameobject_loads,
                    condition_spawn_blocked_loaded_grid_spawn_add_to_map =
                        summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map,
                    condition_spawn_load_plan_count = summary.condition_spawn_load_plan_count,
                    condition_spawn_unsupported_spawn_types =
                        summary.condition_spawn_unsupported_spawn_types,
                    condition_spawn_skipped_respawn_timer_active =
                        summary.condition_spawn_skipped_respawn_timer_active,
                    condition_spawn_skipped_live_object_active =
                        summary.condition_spawn_skipped_live_object_active,
                    condition_spawn_skipped_unloaded_grid =
                        summary.condition_spawn_skipped_unloaded_grid,
                    condition_spawn_skipped_difficulty_mismatch =
                        summary.condition_spawn_skipped_difficulty_mismatch,
                    planned_despawn = summary.planned_despawn,
                    despawn_executed = summary.despawn_executed,
                    despawn_objects_removed = summary.despawn_objects_removed,
                    despawn_respawn_timers_removed = summary.despawn_respawn_timers_removed,
                    despawn_blocked_missing_group = summary.despawn_blocked_missing_group,
                    despawn_blocked_system_group = summary.despawn_blocked_system_group,
                    despawn_unsupported_live_types = summary.despawn_unsupported_live_types,
                    despawn_respawn_timer_unsupported_types =
                        summary.despawn_respawn_timer_unsupported_types,
                    despawn_stale_index_entries = summary.despawn_stale_index_entries,
                    despawn_remove_errors = summary.despawn_remove_errors,
                    respawn_deleted_inactive_spawn_group =
                        summary.respawn_deleted_inactive_spawn_group,
                    respawn_deleted_live_object_blocker =
                        summary.respawn_deleted_live_object_blocker,
                    respawn_processed_pool_timers = summary.respawn_processed_pool_timers,
                    respawn_processed_unloaded_grid_respawns =
                        summary.respawn_processed_unloaded_grid_respawns,
                    respawn_executed_loaded_grid_respawns =
                        summary.respawn_executed_loaded_grid_respawns,
                    respawn_blocked_loaded_grid_respawn_loads =
                        summary.respawn_blocked_loaded_grid_respawn_loads,
                    respawn_blocked_loaded_grid_respawn_add_to_map =
                        summary.respawn_blocked_loaded_grid_respawn_add_to_map,
                    respawn_pool_update_plans = summary.respawn_pool_update_plans,
                    respawn_blocked_pool_plan_errors = summary.respawn_blocked_pool_plan_errors,
                    respawn_blocked_missing_spawn_data = summary.respawn_blocked_missing_spawn_data,
                    respawn_blocked_pool_runtime = summary.respawn_blocked_pool_runtime,
                    respawn_blocked_do_respawn_runtime = summary.respawn_blocked_do_respawn_runtime,
                    respawn_blocked_linked_respawn_non_future =
                        summary.respawn_blocked_linked_respawn_non_future,
                    respawn_blocked_unsupported_spawn_type =
                        summary.respawn_blocked_unsupported_spawn_type,
                    respawn_db_delete_queued = summary.respawn_db_delete_queued,
                    respawn_db_delete_executed = summary.respawn_db_delete_executed,
                    respawn_db_delete_failed = summary.respawn_db_delete_failed,
                    respawn_db_delete_skipped_non_world_map =
                        summary.respawn_db_delete_skipped_non_world_map,
                    respawn_db_delete_skipped_instanceable_map =
                        summary.respawn_db_delete_skipped_instanceable_map,
                    respawn_db_delete_skipped_invalid_map_id =
                        summary.respawn_db_delete_skipped_invalid_map_id,
                    respawn_db_save_queued = summary.respawn_db_save_queued,
                    respawn_db_save_executed = summary.respawn_db_save_executed,
                    respawn_db_save_failed = summary.respawn_db_save_failed,
                    respawn_db_save_skipped_non_world_map =
                        summary.respawn_db_save_skipped_non_world_map,
                    respawn_db_save_skipped_instanceable_map =
                        summary.respawn_db_save_skipped_instanceable_map,
                    respawn_db_save_skipped_invalid_map_id =
                        summary.respawn_db_save_skipped_invalid_map_id,
                    "C++ respawn-check timer fired; executed safe ProcessRespawns composite zero-delete branches plus linked future reschedules, represented pooled timer UpdatePool plans, safe DoRespawn unloaded-grid early-return timer removals, map-local SpawnGroupDespawn condition-failure side effects, and bounded loaded-grid SpawnGroupSpawn condition loads; submitted DEL_RESPAWN/REP_RESPAWN side effects to the shared async DB writer outside the MapManager lock; full SpawnGroupSpawn AreaTrigger/ObjectAccessor/fanout/scripts/AI and Spawn1Object/ReSpawn1Object runtime remain pending"
                );
            }

            if stop_after_tick {
                break;
            }
        }
    })
}
