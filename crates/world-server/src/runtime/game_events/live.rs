use super::super::*;
use wow_persistence::{
    GameEventPersistenceMutationLikeCpp, GameEventPersistenceMutationOutcomeLikeCpp,
    GameEventPersistencePortLikeCpp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameEventLiveUpdateActionLikeCpp {
    Spawn(i16),
    Unspawn(i16),
    AnnounceEvent {
        event_id: u16,
        description: String,
        description_len: usize,
        announce: u8,
        config_event_announce: bool,
    },
    ChangeEquipOrModel {
        event_id: u16,
        activate: bool,
    },
    RunSmartAIScripts {
        event_id: u16,
        activate: bool,
    },
    ResetEventSeasonalQuests {
        event_id: u16,
        event_start_time: u64,
    },
    UpdateEventQuests {
        event_id: u16,
        activate: bool,
    },
    UpdateWorldStates {
        event_id: u16,
        activate: bool,
    },
    UpdateNpcFlags {
        event_id: u16,
    },
    UpdateNpcVendor {
        event_id: u16,
        activate: bool,
    },
}
#[derive(Debug, Clone)]
pub(crate) struct GameEventSeasonalQuestDbDeleteLikeCpp {
    pub(crate) event_id: u16,
    pub(crate) event_start_time: i64,
    pub(crate) mutation: GameEventPersistenceMutationLikeCpp,
}
#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventLiveUpdateSideEffectSummaryLikeCpp {
    pub(crate) actions: Vec<GameEventLiveUpdateActionLikeCpp>,
    pub(crate) spawn_actions: usize,
    pub(crate) unspawn_actions: usize,
    pub(crate) announce_event_actions: usize,
    pub(crate) announce_event_description_len_total: usize,
    pub(crate) announce_event_world_text_represented: usize,
    pub(crate) announce_event_lines: usize,
    pub(crate) announce_event_registry_missing: usize,
    pub(crate) announce_event_send_attempted: usize,
    pub(crate) announce_event_send_queued: usize,
    pub(crate) announce_event_send_failed: usize,
    pub(crate) announce_event_localization_unrepresented: usize,
    pub(crate) announce_event_in_world_filter_unrepresented: usize,
    pub(crate) announce_event_not_in_world_skipped: usize,
    pub(crate) announce_event_world_text_unimplemented: usize,
    pub(crate) announce_event_session_fanout_unimplemented: usize,
    pub(crate) change_equip_or_model_actions: usize,
    pub(crate) change_equip_or_model_records_seen: usize,
    pub(crate) change_equip_or_model_records_applied: usize,
    pub(crate) change_equip_or_model_missing_event_buckets: usize,
    pub(crate) change_equip_or_model_missing_spawn_metadata: usize,
    pub(crate) change_equip_or_model_missing_runtime_rows: usize,
    pub(crate) change_equip_or_model_maps_matched: usize,
    pub(crate) change_equip_or_model_live_creatures_mutated: usize,
    pub(crate) change_equip_or_model_stale_index_or_wrong_kind: usize,
    pub(crate) change_equip_or_model_model_validation_unavailable: usize,
    pub(crate) run_smart_ai_actions: usize,
    pub(crate) run_smart_ai_maps_visited: usize,
    pub(crate) run_smart_ai_creature_candidates: usize,
    pub(crate) run_smart_ai_gameobject_candidates: usize,
    pub(crate) run_smart_ai_creature_ai_enabled_unrepresented: usize,
    pub(crate) run_smart_ai_script_dispatch_unrepresented: usize,
    pub(crate) reset_event_seasonal_quests_actions: usize,
    pub(crate) reset_event_seasonal_quests_event_start_time_zero: usize,
    pub(crate) reset_event_seasonal_quests_event_start_time_nonzero: usize,
    pub(crate) reset_event_seasonal_quests_player_session_runtime_unimplemented: usize,
    pub(crate) reset_event_seasonal_quests_player_session_registry_missing: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_attempted: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_queued: usize,
    pub(crate) reset_event_seasonal_quests_player_session_send_failed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_statement_unimplemented: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_queued: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_executed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_failed: usize,
    pub(crate) reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range:
        usize,
    pub(crate) reset_event_seasonal_quest_db_deletes: Vec<GameEventSeasonalQuestDbDeleteLikeCpp>,
    pub(crate) update_event_quests_actions: usize,
    pub(crate) update_event_quests_creature_records_seen: usize,
    pub(crate) update_event_quests_gameobject_records_seen: usize,
    pub(crate) update_event_quests_creature_inserted: usize,
    pub(crate) update_event_quests_gameobject_inserted: usize,
    pub(crate) update_event_quests_creature_removed: usize,
    pub(crate) update_event_quests_gameobject_removed: usize,
    pub(crate) update_event_quests_creature_remove_misses: usize,
    pub(crate) update_event_quests_gameobject_remove_misses: usize,
    pub(crate) update_event_quests_creature_no_match: usize,
    pub(crate) update_event_quests_gameobject_no_match: usize,
    pub(crate) update_event_quests_creature_missing_event_buckets: usize,
    pub(crate) update_event_quests_gameobject_missing_event_buckets: usize,
    pub(crate) update_event_quests_creature_skipped_active_other_event: usize,
    pub(crate) update_event_quests_gameobject_skipped_active_other_event: usize,
    pub(crate) update_world_states_actions: usize,
    pub(crate) update_world_states_no_holiday: usize,
    pub(crate) update_world_states_missing_event: usize,
    pub(crate) update_world_states_store_missing: usize,
    pub(crate) update_world_states_holiday_not_weekend_battleground: usize,
    pub(crate) update_world_states_battlemaster_list_missing: usize,
    pub(crate) update_world_states_holiday_world_state_zero: usize,
    pub(crate) update_world_states_holiday_lookup_unrepresented: usize,
    pub(crate) update_world_states_set_value_represented: usize,
    pub(crate) update_world_states_set_value_attempts: usize,
    pub(crate) update_world_states_realm_changed_or_inserted: usize,
    pub(crate) update_world_states_realm_unchanged_noop: usize,
    pub(crate) update_world_states_map_specific_no_map_unsupported: usize,
    pub(crate) update_world_states_global_message_represented: usize,
    pub(crate) update_world_states_global_message_registry_missing: usize,
    pub(crate) update_world_states_global_message_send_attempted: usize,
    pub(crate) update_world_states_global_message_send_queued: usize,
    pub(crate) update_world_states_global_message_send_failed: usize,
    pub(crate) update_world_states_global_message_not_in_world_skipped: usize,
    pub(crate) update_world_states_last_world_state_id: Option<i16>,
    pub(crate) update_world_states_last_world_state_value: Option<i32>,
    pub(crate) update_npc_flags_actions: usize,
    pub(crate) update_npc_flags_records_seen: usize,
    pub(crate) update_npc_flags_missing_event_buckets: usize,
    pub(crate) update_npc_flags_missing_spawn_metadata: usize,
    pub(crate) update_npc_flags_template_npcflag_missing: usize,
    pub(crate) update_npc_flags_maps_matched: usize,
    pub(crate) update_npc_flags_indexed_guids: usize,
    pub(crate) update_npc_flags_live_creatures_mutated: usize,
    pub(crate) update_npc_flags_stale_index_or_wrong_kind: usize,
    pub(crate) update_npc_flags_low_applied: usize,
    pub(crate) update_npc_flags2_applied: usize,
    pub(crate) update_npc_flags_values_updates_built: usize,
    pub(crate) update_npc_flags_values_update_empty: usize,
    pub(crate) update_npc_flags_values_update_map_id_out_of_range: usize,
    pub(crate) update_npc_flags_values_update_registry_missing: usize,
    pub(crate) update_npc_flags_values_update_not_in_world_skipped: usize,
    pub(crate) update_npc_flags_values_update_wrong_map_skipped: usize,
    pub(crate) update_npc_flags_values_update_send_attempted: usize,
    pub(crate) update_npc_flags_values_update_send_queued: usize,
    pub(crate) update_npc_flags_values_update_send_failed: usize,
    pub(crate) update_npc_vendor_actions: usize,
    pub(crate) update_npc_vendor_records_seen: usize,
    pub(crate) update_npc_vendor_items_added: usize,
    pub(crate) update_npc_vendor_items_removed: usize,
    pub(crate) update_npc_vendor_missing_event_buckets: usize,
    pub(crate) update_npc_vendor_remove_misses: usize,
    pub(crate) update_npc_vendor_no_match: usize,
}
pub(crate) fn game_event_signed_id_like_cpp(event_id: u16) -> i16 {
    i16::try_from(event_id).unwrap_or(i16::MAX)
}
pub(crate) fn should_announce_game_event_like_cpp(
    announce: u8,
    config_event_announce: bool,
) -> bool {
    announce == 1 || (announce == 2 && config_event_announce)
}
pub(crate) fn game_event_announcement_lines_like_cpp(description: &str) -> Vec<String> {
    // C++ WorldWorldTextBuilder formats LANG_EVENTMESSAGE first and then
    // ChatHandler::LineFromMessage tokenizes the resulting buffer with strtok("\n"),
    // so empty newline runs are skipped. Rust does not have ObjectMgr TrinityString
    // locale storage yet; represent the known enUS fallback format explicitly.
    let formatted = format!("|cffff0000[Event Message]: {description}|r");
    formatted
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}
pub(crate) fn fanout_game_event_announcement_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    description: &str,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    summary.announce_event_world_text_represented += 1;
    summary.announce_event_localization_unrepresented += 1;

    let lines = game_event_announcement_lines_like_cpp(description);
    summary.announce_event_lines += lines.len();
    if lines.is_empty() {
        return;
    }

    let Some(player_registry) = player_registry else {
        summary.announce_event_registry_missing += 1;
        return;
    };

    let packet_bytes: Vec<Vec<u8>> = lines
        .into_iter()
        .map(|text| {
            ChatPkt {
                msg_type: ChatMsg::System,
                language: 0,
                sender_guid: ObjectGuid::EMPTY,
                sender_name: String::new(),
                target_guid: ObjectGuid::EMPTY,
                target_name: String::new(),
                prefix: String::new(),
                channel: String::new(),
                text,
                virtual_realm: 0,
            }
            .to_bytes()
        })
        .collect();

    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.announce_event_not_in_world_skipped += 1;
            continue;
        }

        for bytes in &packet_bytes {
            summary.announce_event_send_attempted += 1;
            match player_registry.try_send_current_packet(recipient.registration, bytes.clone()) {
                Ok(()) => summary.announce_event_send_queued += 1,
                Err(_) => summary.announce_event_send_failed += 1,
            }
        }
    }
}
pub(crate) fn game_event_seasonal_quest_db_delete_like_cpp(
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(event_start_time_i64) = i64::try_from(event_start_time) else {
        summary.reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range += 1;
        return;
    };

    summary.reset_event_seasonal_quests_character_db_delete_queued += 1;
    summary
        .reset_event_seasonal_quest_db_deletes
        .push(GameEventSeasonalQuestDbDeleteLikeCpp {
            event_id,
            event_start_time: event_start_time_i64,
            mutation: GameEventPersistenceMutationLikeCpp::ResetSeasonalQuests {
                event_id,
                event_start_time: event_start_time_i64,
            },
        });
}
pub(crate) fn fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.reset_event_seasonal_quests_player_session_registry_missing += 1;
        return;
    };

    for recipient in player_registry.runtime_recipients() {
        summary.reset_event_seasonal_quests_player_session_send_attempted += 1;
        let command = SessionCommand::ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand {
            event_id,
            event_start_time,
        });
        match player_registry.try_send_current_command(recipient.registration, command) {
            Ok(()) => summary.reset_event_seasonal_quests_player_session_send_queued += 1,
            Err(_) => summary.reset_event_seasonal_quests_player_session_send_failed += 1,
        }
    }
}
pub(crate) fn fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let reset_actions: Vec<(u16, u64)> = summary
        .actions
        .iter()
        .filter_map(|action| match action {
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => Some((*event_id, *event_start_time)),
            _ => None,
        })
        .collect();

    for (event_id, event_start_time) in reset_actions {
        fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
            player_registry,
            event_id,
            event_start_time,
            summary,
        );
    }
}
pub(crate) async fn execute_game_event_seasonal_quest_db_deletes_like_cpp(
    persistence: &dyn GameEventPersistencePortLikeCpp,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let db_delete_total = summary.reset_event_seasonal_quest_db_deletes.len();
    for (db_delete_index, db_delete) in summary
        .reset_event_seasonal_quest_db_deletes
        .drain(..)
        .enumerate()
    {
        match persistence
            .execute_mutation_like_cpp(db_delete.mutation)
            .await
        {
            GameEventPersistenceMutationOutcomeLikeCpp::Applied => {
                summary.reset_event_seasonal_quests_character_db_delete_executed += 1;
            }
            GameEventPersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                summary.reset_event_seasonal_quests_character_db_delete_failed += 1;
                tracing::error!(
                    error = %reason,
                    db_delete_index = db_delete_index + 1,
                    db_delete_total,
                    event_id = db_delete.event_id,
                    event_start_time = db_delete.event_start_time,
                    "Failed to execute C++ World::ResetEventSeasonalQuests character DB delete; continuing live update loop"
                );
            }
        }
    }
}
pub(crate) fn game_event_live_update_actions_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> Vec<GameEventLiveUpdateActionLikeCpp> {
    let mut actions = Vec::new();
    for &event_id in &outcome.negative_spawn_event_ids {
        actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(summary) = start_outcome {
            if summary.apply_new_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                if let Some(event) = canonical_spawn_metadata.game_event_like_cpp(summary.event_id)
                {
                    if should_announce_game_event_like_cpp(event.announce, config_event_announce) {
                        actions.push(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                            event_id: summary.event_id,
                            description: event.description.clone(),
                            description_len: event.description.len(),
                            announce: event.announce,
                            config_event_announce,
                        });
                    }
                }
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                    event_id: summary.event_id,
                    event_start_time: canonical_spawn_metadata.game_event_last_start_time_like_cpp(
                        summary.event_id,
                        outcome.current_time_secs,
                    ),
                });
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(summary) = stop_outcome {
            if summary.unapply_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: false,
                });
            }
        }
    }
    actions
}
pub(crate) fn game_event_change_equip_or_model_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let records = canonical_spawn_metadata
        .game_event_model_equip_like_cpp(event_id)
        .map_or_else(Vec::new, <[_]>::to_vec);

    for record in &records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.change_equip_or_model_missing_spawn_metadata += 1;
            continue;
        };

        let (equipment_id, model_id) = if activate {
            (record.equipment_id, record.model_id)
        } else {
            (record.equipment_id_prev, record.model_id_prev)
        };
        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .change_game_event_equip_or_model_by_spawn_id_like_cpp(
                        record.spawn_id,
                        equipment_id,
                        model_id,
                        false,
                    );
                summary.change_equip_or_model_live_creatures_mutated +=
                    outcome.live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    outcome.model_validation_unavailable;
            }
        });
        summary.change_equip_or_model_maps_matched += maps_matched_for_record;
    }

    let baseline_summary = canonical_spawn_metadata
        .change_game_event_model_equip_baseline_like_cpp(event_id, activate);
    summary.change_equip_or_model_records_seen += baseline_summary.records_seen;
    summary.change_equip_or_model_records_applied += baseline_summary.records_applied;
    if baseline_summary.missing_event_bucket {
        summary.change_equip_or_model_missing_event_buckets += 1;
    }
    summary.change_equip_or_model_missing_spawn_metadata += baseline_summary.missing_spawn_metadata;
    summary.change_equip_or_model_missing_runtime_rows +=
        baseline_summary.missing_creature_runtime_rows;
    summary
}
pub(crate) fn fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    values_update: &wow_map::GameEventNpcFlagValuesUpdateLikeCpp,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(map_id) = u16::try_from(values_update.map_id) else {
        summary.update_npc_flags_values_update_map_id_out_of_range += 1;
        return;
    };
    let Some(packet_update) = unit_values_update_to_packet(&values_update.values_update) else {
        summary.update_npc_flags_values_update_empty += 1;
        return;
    };
    let update = wow_packet::packets::update::UpdateObject::unit_values_update(
        values_update.guid,
        map_id,
        packet_update.clone(),
    );
    summary.update_npc_flags_values_updates_built += 1;

    let Some(player_registry) = player_registry else {
        summary.update_npc_flags_values_update_registry_missing += 1;
        return;
    };

    let packet_bytes = update.to_bytes();
    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.update_npc_flags_values_update_not_in_world_skipped += 1;
            continue;
        }
        if recipient.map_id != map_id {
            summary.update_npc_flags_values_update_wrong_map_skipped += 1;
            continue;
        }

        summary.update_npc_flags_values_update_send_attempted += 1;
        let command =
            SessionCommand::SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand {
                object_guid: values_update.guid,
                map_id,
                packet_bytes: packet_bytes.clone(),
                unit_values_update: Some(packet_update.clone()),
            });
        match player_registry.try_send_current_command(recipient.registration, command) {
            Ok(()) => summary.update_npc_flags_values_update_send_queued += 1,
            Err(_) => summary.update_npc_flags_values_update_send_failed += 1,
        }
    }
}
pub(crate) fn game_event_update_npc_flags_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    creature_template_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    active_event_ids: &[u16],
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(records) = canonical_spawn_metadata.game_event_npc_flags_like_cpp(event_id) else {
        summary.update_npc_flags_missing_event_buckets += 1;
        return summary;
    };
    summary.update_npc_flags_records_seen = records.len();

    for record in records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.update_npc_flags_missing_spawn_metadata += 1;
            continue;
        };
        let template_npc_flags = creature_template_store
            .get(spawn_data.id)
            .map(|template| template.npc_flags)
            .unwrap_or_else(|| {
                summary.update_npc_flags_template_npcflag_missing += 1;
                0
            });
        let overlay = canonical_spawn_metadata
            .game_event_npc_flag_mask_like_cpp(record.spawn_id, active_event_ids);
        let npcflag_mask_with_template = overlay | template_npc_flags;

        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .update_game_event_npc_flags_by_spawn_id_like_cpp(
                        record.spawn_id,
                        npcflag_mask_with_template,
                    );
                summary.update_npc_flags_indexed_guids += outcome.indexed_guids;
                summary.update_npc_flags_live_creatures_mutated += outcome.live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied += outcome.npc_flags_low_applied;
                summary.update_npc_flags2_applied += outcome.npc_flags2_applied;
                for values_update in &outcome.values_updates {
                    fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
                        player_registry,
                        values_update,
                        &mut summary,
                    );
                }
            }
        });
        summary.update_npc_flags_maps_matched += maps_matched_for_record;
    }

    summary
}

pub(crate) fn fanout_realm_update_world_state_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    world_state_id: i32,
    value: i32,
    hidden: bool,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.update_world_states_global_message_registry_missing += 1;
        return;
    };

    // C++ assigns signed `int32 worldStateId` into packet `uint32 VariableID`;
    // Rust's `as u32` preserves the same two's-complement wrapping semantics.
    let packet = wow_packet::packets::misc::UpdateWorldState {
        variable_id: world_state_id as u32,
        value,
        hidden,
    };
    let bytes = packet.to_bytes();

    for recipient in player_registry.runtime_recipients() {
        if !recipient.is_in_world {
            summary.update_world_states_global_message_not_in_world_skipped += 1;
            continue;
        }

        summary.update_world_states_global_message_send_attempted += 1;
        match player_registry.try_send_current_packet(recipient.registration, bytes.clone()) {
            Ok(()) => summary.update_world_states_global_message_send_queued += 1,
            Err(_) => summary.update_world_states_global_message_send_failed += 1,
        }
    }
}

pub(crate) fn game_event_update_npc_vendor_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let vendor_summary =
        canonical_spawn_metadata.update_game_event_npc_vendor_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_npc_vendor_records_seen = vendor_summary.records_seen;
    summary.update_npc_vendor_items_added = vendor_summary.items_added;
    summary.update_npc_vendor_items_removed = vendor_summary.items_removed;
    summary.update_npc_vendor_remove_misses = vendor_summary.remove_misses;
    summary.update_npc_vendor_no_match = vendor_summary.no_match;
    if vendor_summary.missing_event_bucket {
        summary.update_npc_vendor_missing_event_buckets = 1;
    }
    summary
}

pub(crate) fn game_event_update_world_states_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(event) = canonical_spawn_metadata.game_event_like_cpp(event_id) else {
        summary.update_world_states_missing_event = 1;
        return summary;
    };

    if event.holiday_id == 0 {
        summary.update_world_states_no_holiday = 1;
        return summary;
    }

    let Some(battlemaster_list_store) = battlemaster_list_store else {
        summary.update_world_states_store_missing = 1;
        summary.update_world_states_holiday_lookup_unrepresented = 1;
        return summary;
    };

    match battlemaster_list_store.holiday_world_state_for_weekend_holiday_like_cpp(event.holiday_id)
    {
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNone => {
            summary.update_world_states_no_holiday = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNotWeekendBattleground { .. } => {
            summary.update_world_states_holiday_not_weekend_battleground = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::BattlemasterListMissing { .. } => {
            summary.update_world_states_battlemaster_list_missing = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayWorldStateZero { .. } => {
            summary.update_world_states_holiday_world_state_zero = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::SetValueRepresented {
            world_state_id, ..
        } => {
            let value = if activate { 1 } else { 0 };
            summary.update_world_states_set_value_attempts = 1;
            summary.update_world_states_last_world_state_id = Some(world_state_id);
            summary.update_world_states_last_world_state_value = Some(value);
            let Some(world_state_mgr) = world_state_mgr.as_deref_mut() else {
                summary.update_world_states_set_value_represented = 1;
                return summary;
            };
            match world_state_mgr.set_value_realm_or_map_null_like_cpp(
                i32::from(world_state_id),
                value,
                false,
            ) {
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmInsertedOrChanged {
                    world_state_id,
                    new_value,
                    hidden,
                    global_message_represented,
                    ..
                } => {
                    summary.update_world_states_realm_changed_or_inserted = 1;
                    if global_message_represented {
                        summary.update_world_states_global_message_represented = 1;
                        fanout_realm_update_world_state_to_player_sessions_like_cpp(
                            player_registry,
                            world_state_id,
                            new_value,
                            hidden,
                            &mut summary,
                        );
                    }
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmUnchanged { .. } => {
                    summary.update_world_states_realm_unchanged_noop = 1;
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::MapSpecificNoMapUnsupported { .. } => {
                    summary.update_world_states_map_specific_no_map_unsupported = 1;
                }
            }
        }
    }

    summary
}

pub(crate) fn game_event_update_quests_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let quest_summary = canonical_spawn_metadata
        .update_game_event_quest_relation_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_event_quests_creature_records_seen = quest_summary.creature_records_seen;
    summary.update_event_quests_gameobject_records_seen = quest_summary.gameobject_records_seen;
    summary.update_event_quests_creature_inserted = quest_summary.creature_inserted;
    summary.update_event_quests_gameobject_inserted = quest_summary.gameobject_inserted;
    summary.update_event_quests_creature_removed = quest_summary.creature_removed;
    summary.update_event_quests_gameobject_removed = quest_summary.gameobject_removed;
    summary.update_event_quests_creature_remove_misses = quest_summary.creature_remove_misses;
    summary.update_event_quests_gameobject_remove_misses = quest_summary.gameobject_remove_misses;
    summary.update_event_quests_creature_no_match = quest_summary.creature_no_match;
    summary.update_event_quests_gameobject_no_match = quest_summary.gameobject_no_match;
    summary.update_event_quests_creature_skipped_active_other_event =
        quest_summary.creature_skipped_active_other_event;
    summary.update_event_quests_gameobject_skipped_active_other_event =
        quest_summary.gameobject_skipped_active_other_event;
    if quest_summary.creature_missing_event_bucket {
        summary.update_event_quests_creature_missing_event_buckets = 1;
    }
    if quest_summary.gameobject_missing_event_bucket {
        summary.update_event_quests_gameobject_missing_event_buckets = 1;
    }
    summary
}

pub(crate) fn game_event_run_smart_ai_scripts_like_cpp(
    manager: &wow_map::MapManager,
    _event_id: u16,
    _activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    manager.do_for_all_maps(|managed_map| {
        let candidates = managed_map
            .map()
            .game_event_smart_ai_script_candidates_like_cpp();
        summary.run_smart_ai_maps_visited += candidates.maps_visited;
        summary.run_smart_ai_creature_candidates += candidates.in_world_creature_candidates;
        summary.run_smart_ai_gameobject_candidates += candidates.in_world_gameobject_candidates;
        summary.run_smart_ai_creature_ai_enabled_unrepresented +=
            candidates.creature_ai_enabled_unrepresented;
        summary.run_smart_ai_script_dispatch_unrepresented +=
            candidates.script_dispatch_unrepresented;
    });
    summary
}
