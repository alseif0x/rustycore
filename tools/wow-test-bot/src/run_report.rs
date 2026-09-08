//! Human and JSON reports for the integrated bot run.

use super::*;

pub(super) fn log_bot_summary(
    result: &BotRunResult,
    require_proposal: bool,
    require_group: bool,
    login_only: bool,
) {
    if result.success(require_proposal, require_group, login_only) {
        if result.stand_state_smoke {
            info!(
                "✅ Bot {}: SUCCESS stand_state_smoke requested={:?} confirmed={:?} failure={:?}",
                result.account,
                result.stand_states_requested,
                result.stand_states_confirmed,
                result.stand_state_failure
            );
            return;
        }
        if result.quest_smoke {
            info!(
                "✅ Bot {}: SUCCESS quest_smoke target={:?}/{:?} ids={:?} details={} request_items={} accept_sent={} db_verified={} db_status={:?} obj_verified={} obj_before={:?} obj_after={:?} failure={:?}",
                result.account,
                result.quest_target_entry,
                result.quest_target_spawn_guid,
                result.quest_ids_seen,
                result.quest_details_seen,
                result.quest_request_items_seen,
                result.quest_accept_sent,
                result.quest_db_verified,
                result.quest_db_status,
                result.quest_objective_db_verified,
                result.quest_objective_db_before,
                result.quest_objective_db_after,
                result.quest_failure
            );
            return;
        }
        if result.bank_smoke {
            info!(
                "✅ Bot {}: SUCCESS bank_smoke banker={:?}/{:?} item={:?}/entry={:?} slots={:?}->{:?} open={} deposit={} relog={} withdraw={} failure={:?}",
                result.account,
                result.bank_banker_entry,
                result.bank_banker_spawn_guid,
                result.bank_item_guid,
                result.bank_item_entry,
                result.bank_inventory_slot,
                result.bank_bank_slot,
                result.bank_open_confirmed,
                result.bank_deposit_persisted,
                result.bank_relogin_after_deposit,
                result.bank_withdraw_persisted,
                result.bank_failure
            );
            return;
        }
        if result.void_storage_smoke {
            info!(
                "✅ Bot {}: SUCCESS void_storage item_id={:?} unlock={} deposit={} deposit_relog={} swap={} swap_relog={} withdraw={} withdraw_relog={} failure={:?}",
                result.account,
                result.void_storage_item_id,
                result.void_storage_unlock_persisted,
                result.void_storage_deposit_persisted,
                result.void_storage_deposit_relogin_verified,
                result.void_storage_swap_persisted,
                result.void_storage_swap_relogin_verified,
                result.void_storage_withdraw_persisted,
                result.void_storage_withdraw_relogin_verified,
                result.void_storage_failure
            );
            return;
        }
        if result.void_storage_query_capture {
            info!(
                "✅ Bot {}: SUCCESS void_storage_query_capture item_id={:?} failure={:?}",
                result.account, result.void_storage_item_id, result.void_storage_failure
            );
            return;
        }
        if result.homebind_smoke {
            info!(
                "✅ Bot {}: SUCCESS homebind_smoke innkeeper={:?}/{:?} spell_go={} bind_update={} player_bound={} gossip_complete={} db_persisted={} relog={} failure={:?}",
                result.account,
                result.homebind_innkeeper_entry,
                result.homebind_innkeeper_spawn_guid,
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen,
                result.homebind_db_persisted,
                result.homebind_relogin_verified,
                result.homebind_failure
            );
            return;
        }
        if result.inventory_swap_smoke {
            info!(
                "✅ Bot {}: SUCCESS inventory_swap_smoke items={:?}/{:?} entries={:?}/{:?} slots={:?}<->{:?} validation_gate={} forward={} relog_forward={} reverse={} relog_reverse={} item_create={:?} item_create_relog={} metadata_persisted={} failure={:?}",
                result.account,
                result.inventory_swap_item_guid_a,
                result.inventory_swap_item_guid_b,
                result.inventory_swap_item_entry_a,
                result.inventory_swap_item_entry_b,
                result.inventory_swap_slot_a,
                result.inventory_swap_slot_b,
                result.inventory_swap_validation_gate_seen,
                result.inventory_swap_forward_persisted,
                result.inventory_swap_relogin_after_forward,
                result.inventory_swap_reverse_persisted,
                result.inventory_swap_relogin_after_reverse,
                result.inventory_swap_item_create_sha256,
                result.inventory_swap_item_create_relogin_verified,
                result.inventory_swap_item_metadata_persisted,
                result.inventory_swap_failure
            );
            return;
        }
        if result.vendor_smoke {
            info!(
                "✅ Bot {}: SUCCESS vendor_smoke vendor={:?}/{:?}/counter={:?} item={:?}/cost={:?} currency={:?} {:?}->{:?} item_total={:?} list={} buy={} set_currency={} item_push={} relog={} failure={:?}",
                result.account,
                result.vendor_entry,
                result.vendor_spawn_guid,
                result.vendor_runtime_counter,
                result.vendor_item_entry,
                result.vendor_extended_cost,
                result.vendor_currency_id,
                result.vendor_currency_before,
                result.vendor_currency_after,
                result.vendor_item_total_after,
                result.vendor_inventory_seen,
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                result.vendor_relogin_verified,
                result.vendor_failure,
            );
            return;
        }
        if result.equipment_set_smoke {
            info!(
                "✅ Bot {}: SUCCESS equipment_set_smoke type={:?} set_id={:?} guid={:?} login_count={:?} load={} db={} relog={} failure={:?}",
                result.account,
                result.equipment_set_type,
                result.equipment_set_id,
                result.equipment_set_generated_guid,
                result.equipment_set_login_count,
                result.equipment_set_load_seen,
                result.equipment_set_db_persisted,
                result.equipment_set_relogin_verified,
                result.equipment_set_failure,
            );
            return;
        }
        if result.rested_xp_smoke {
            info!(
                "✅ Bot {}: SUCCESS rested_xp_smoke offline={:?}/{:?} target={:?}/{:?}/counter={:?} xp={:?}+{:?} rest={:?}->{:?} relog={} failure={:?}",
                result.account,
                result.rested_xp_offline_wilderness_bonus,
                result.rested_xp_offline_resting_bonus,
                result.rested_xp_target_entry,
                result.rested_xp_target_spawn_guid,
                result.rested_xp_target_guid_counter,
                result.rested_xp_packet_amount,
                result.rested_xp_packet_original,
                result.rested_xp_db_rest_before,
                result.rested_xp_db_rest_after,
                result.rested_xp_relog_verified,
                result.rested_xp_failure,
            );
            return;
        }
        if result.detour_chase_capture {
            info!(
                "✅ Bot {}: SUCCESS detour_chase target={:?}/{:?}/counter={:?} attack_start={} first_swing={} prewindow_moves={} heartbeat={} window_moves={} move_sha256={:?} ping={:?}/pong={} time_sync={}/{}/{} logout={} failure={:?}",
                result.account,
                result.detour_chase_target_entry,
                result.detour_chase_target_spawn_guid,
                result.detour_chase_target_runtime_counter,
                result.detour_chase_attack_start_confirmed,
                result.detour_chase_first_swing_confirmed,
                result.detour_chase_prewindow_target_moves,
                result.detour_chase_heartbeat_sent,
                result.detour_chase_window_target_moves,
                result.detour_chase_monster_move_sha256,
                result.detour_chase_ping_serial,
                result.detour_chase_pong_confirmed,
                result.detour_chase_time_sync_before_window,
                result.detour_chase_time_sync_during_window,
                result.detour_chase_time_sync_after_fence,
                result.detour_chase_logout_confirmed,
                result.detour_chase_failure,
            );
            return;
        }
        if result.creature_spell_capture {
            info!(
                "✅ Bot {}: SUCCESS creature_spell target={:?}/{:?}/counter={:?} heartbeat={} start={:?}/{:?} go={:?}/{:?} cast={:?}:{:?} hit={:?} miss={:?} full_log={:?} disconnect={} logout={} failure={:?}",
                result.account,
                result.creature_spell_target_entry,
                result.creature_spell_target_spawn_guid,
                result.creature_spell_target_runtime_counter,
                result.creature_spell_heartbeat_sent,
                result.creature_spell_start_opcode,
                result.creature_spell_start_body_sha256,
                result.creature_spell_go_opcode,
                result.creature_spell_go_body_sha256,
                result.creature_spell_cast_id_high,
                result.creature_spell_cast_id_low,
                result.creature_spell_go_hit_target_count,
                result.creature_spell_go_miss_target_count,
                result.creature_spell_full_combat_log,
                result.creature_spell_disconnect_confirmed,
                result.creature_spell_logout_confirmed,
                result.creature_spell_failure,
            );
            return;
        }
        if let Some(evidence) = result.cast_lifecycle.as_ref() {
            info!(
                "✅ Bot {}: SUCCESS cast_lifecycle passed={} actions={} expectations={}/{} events={} logout={} failure={:?}",
                result.account,
                evidence.passed,
                evidence.actions.len(),
                evidence.expected_rules.iter().filter(|rule| rule.matched).count(),
                evidence.expected_rules.len(),
                evidence.events.len(),
                evidence.logout_confirmed,
                evidence.failure,
            );
            return;
        }
        if result.loot_race_smoke {
            info!(
                "✅ Bot {}: SUCCESS loot_race target={:?}/{:?}/counter={:?} party={} discovered={} opened={} list={:?} coins={:?} item_push={} removed={} money_notify={:?} coin_removed={} db_item={:?} db_money_delta={:?} relog={} failure={:?}",
                result.account,
                result.loot_race_target_entry,
                result.loot_race_target_spawn_guid,
                result.loot_race_target_runtime_counter,
                result.loot_race_party_confirmed,
                result.loot_race_target_discovered,
                result.loot_race_loot_opened,
                result.loot_race_loot_list_id,
                result.loot_race_loot_coins,
                result.loot_race_item_push_seen,
                result.loot_race_loot_removed_seen,
                result.loot_race_money_notify_amount,
                result.loot_race_coin_removed_seen,
                result.loot_race_db_item_total,
                result.loot_race_db_money_delta,
                result.loot_race_relog_verified,
                result.loot_race_failure,
            );
            return;
        }
        if result.group_capacity_race_smoke {
            info!(
                "✅ Bot {}: SUCCESS group_capacity_race group={:?} outcome={:?} final_members={:?} failure={:?}",
                result.account,
                result.group_capacity_group_id,
                result.group_capacity_outcome,
                result.group_capacity_final_member_count,
                result.group_capacity_failure,
            );
            return;
        }
        info!(
            "✅ Bot {}: SUCCESS login={{auth:{}, enum:{}, player:{}}} join={:?}/{:?} proposal={} group={} teleport_denied={:?}",
            result.account,
            result.world_auth,
            result.enum_characters,
            result.player_login_verified,
            result.join_result,
            result.join_detail,
            result.got_proposal,
            result.group_formed,
            result.teleport_denied_reason
        );
    } else {
        if result.stand_state_smoke {
            error!(
                "❌ Bot {}: FAILED stand_state_smoke requested={:?} confirmed={:?} failure={:?}",
                result.account,
                result.stand_states_requested,
                result.stand_states_confirmed,
                result.stand_state_failure
            );
            return;
        }
        if result.quest_smoke {
            error!(
                "❌ Bot {}: FAILED quest_smoke target={:?}/{:?} ids={:?} details={} request_items={} accept_sent={} db_verified={} db_status={:?} obj_verified={} obj_before={:?} obj_after={:?} failure={:?}",
                result.account,
                result.quest_target_entry,
                result.quest_target_spawn_guid,
                result.quest_ids_seen,
                result.quest_details_seen,
                result.quest_request_items_seen,
                result.quest_accept_sent,
                result.quest_db_verified,
                result.quest_db_status,
                result.quest_objective_db_verified,
                result.quest_objective_db_before,
                result.quest_objective_db_after,
                result.quest_failure
            );
            return;
        }
        if result.bank_smoke {
            error!(
                "❌ Bot {}: FAILED bank_smoke banker={:?}/{:?} item={:?}/entry={:?} slots={:?}->{:?} open={} deposit={} relog={} withdraw={} failure={:?}",
                result.account,
                result.bank_banker_entry,
                result.bank_banker_spawn_guid,
                result.bank_item_guid,
                result.bank_item_entry,
                result.bank_inventory_slot,
                result.bank_bank_slot,
                result.bank_open_confirmed,
                result.bank_deposit_persisted,
                result.bank_relogin_after_deposit,
                result.bank_withdraw_persisted,
                result.bank_failure
            );
            return;
        }
        if result.void_storage_smoke {
            error!(
                "❌ Bot {}: FAILED void_storage item_id={:?} unlock={} deposit={} deposit_relog={} swap={} swap_relog={} withdraw={} withdraw_relog={} failure={:?}",
                result.account,
                result.void_storage_item_id,
                result.void_storage_unlock_persisted,
                result.void_storage_deposit_persisted,
                result.void_storage_deposit_relogin_verified,
                result.void_storage_swap_persisted,
                result.void_storage_swap_relogin_verified,
                result.void_storage_withdraw_persisted,
                result.void_storage_withdraw_relogin_verified,
                result.void_storage_failure
            );
            return;
        }
        if result.void_storage_query_capture {
            error!(
                "❌ Bot {}: FAILED void_storage_query_capture item_id={:?} failure={:?}",
                result.account, result.void_storage_item_id, result.void_storage_failure
            );
            return;
        }
        if result.homebind_smoke {
            error!(
                "❌ Bot {}: FAILED homebind_smoke innkeeper={:?}/{:?} spell_go={} bind_update={} player_bound={} gossip_complete={} db_persisted={} relog={} failure={:?}",
                result.account,
                result.homebind_innkeeper_entry,
                result.homebind_innkeeper_spawn_guid,
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen,
                result.homebind_db_persisted,
                result.homebind_relogin_verified,
                result.homebind_failure
            );
            return;
        }
        if result.inventory_swap_smoke {
            error!(
                "❌ Bot {}: FAILED inventory_swap_smoke items={:?}/{:?} entries={:?}/{:?} slots={:?}<->{:?} validation_gate={} forward={} relog_forward={} reverse={} relog_reverse={} item_create={:?} item_create_relog={} metadata_persisted={} failure={:?}",
                result.account,
                result.inventory_swap_item_guid_a,
                result.inventory_swap_item_guid_b,
                result.inventory_swap_item_entry_a,
                result.inventory_swap_item_entry_b,
                result.inventory_swap_slot_a,
                result.inventory_swap_slot_b,
                result.inventory_swap_validation_gate_seen,
                result.inventory_swap_forward_persisted,
                result.inventory_swap_relogin_after_forward,
                result.inventory_swap_reverse_persisted,
                result.inventory_swap_relogin_after_reverse,
                result.inventory_swap_item_create_sha256,
                result.inventory_swap_item_create_relogin_verified,
                result.inventory_swap_item_metadata_persisted,
                result.inventory_swap_failure
            );
            return;
        }
        if result.vendor_smoke {
            error!(
                "❌ Bot {}: FAILED vendor_smoke vendor={:?}/{:?}/counter={:?} item={:?}/cost={:?} currency={:?} {:?}->{:?} item_total={:?} list={} buy={} set_currency={} item_push={} relog={} failure={:?}",
                result.account,
                result.vendor_entry,
                result.vendor_spawn_guid,
                result.vendor_runtime_counter,
                result.vendor_item_entry,
                result.vendor_extended_cost,
                result.vendor_currency_id,
                result.vendor_currency_before,
                result.vendor_currency_after,
                result.vendor_item_total_after,
                result.vendor_inventory_seen,
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                result.vendor_relogin_verified,
                result.vendor_failure,
            );
            return;
        }
        if result.equipment_set_smoke {
            error!(
                "❌ Bot {}: FAILED equipment_set_smoke type={:?} set_id={:?} guid={:?} login_count={:?} load={} db={} relog={} failure={:?}",
                result.account,
                result.equipment_set_type,
                result.equipment_set_id,
                result.equipment_set_generated_guid,
                result.equipment_set_login_count,
                result.equipment_set_load_seen,
                result.equipment_set_db_persisted,
                result.equipment_set_relogin_verified,
                result.equipment_set_failure,
            );
            return;
        }
        if result.rested_xp_smoke {
            error!(
                "❌ Bot {}: FAILED rested_xp_smoke offline={:?}/{:?} target={:?}/{:?}/counter={:?} packet={:?}/{:?} db_xp={:?}->{:?} db_rest={:?}->{:?} relog={} failure={:?}",
                result.account,
                result.rested_xp_offline_wilderness_bonus,
                result.rested_xp_offline_resting_bonus,
                result.rested_xp_target_entry,
                result.rested_xp_target_spawn_guid,
                result.rested_xp_target_guid_counter,
                result.rested_xp_packet_amount,
                result.rested_xp_packet_original,
                result.rested_xp_db_xp_before,
                result.rested_xp_db_xp_after,
                result.rested_xp_db_rest_before,
                result.rested_xp_db_rest_after,
                result.rested_xp_relog_verified,
                result.rested_xp_failure,
            );
            return;
        }
        if result.detour_chase_capture {
            error!(
                "❌ Bot {}: FAILED detour_chase target={:?}/{:?}/counter={:?} discovered={} active_mover={} attack_start={} first_swing={} prewindow_moves={} heartbeat={} window_moves={} move_sha256={:?} ping={:?}/pong={} time_sync={}/{}/{} logout={} failure={:?}",
                result.account,
                result.detour_chase_target_entry,
                result.detour_chase_target_spawn_guid,
                result.detour_chase_target_runtime_counter,
                result.detour_chase_target_discovered,
                result.detour_chase_active_mover_ack_sent,
                result.detour_chase_attack_start_confirmed,
                result.detour_chase_first_swing_confirmed,
                result.detour_chase_prewindow_target_moves,
                result.detour_chase_heartbeat_sent,
                result.detour_chase_window_target_moves,
                result.detour_chase_monster_move_sha256,
                result.detour_chase_ping_serial,
                result.detour_chase_pong_confirmed,
                result.detour_chase_time_sync_before_window,
                result.detour_chase_time_sync_during_window,
                result.detour_chase_time_sync_after_fence,
                result.detour_chase_logout_confirmed,
                result.detour_chase_failure,
            );
            return;
        }
        if result.creature_spell_capture {
            error!(
                "❌ Bot {}: FAILED creature_spell target={:?}/{:?}/counter={:?} discovered={} heartbeat={} start={:?}/{:?} go={:?}/{:?} hit={:?} miss={:?} adjacent={} disconnect={} logout={} failure={:?}",
                result.account,
                result.creature_spell_target_entry,
                result.creature_spell_target_spawn_guid,
                result.creature_spell_target_runtime_counter,
                result.creature_spell_target_discovered,
                result.creature_spell_heartbeat_sent,
                result.creature_spell_start_opcode,
                result.creature_spell_start_body_sha256,
                result.creature_spell_go_opcode,
                result.creature_spell_go_body_sha256,
                result.creature_spell_go_hit_target_count,
                result.creature_spell_go_miss_target_count,
                result.creature_spell_adjacent_start_go,
                result.creature_spell_disconnect_confirmed,
                result.creature_spell_logout_confirmed,
                result.creature_spell_failure,
            );
            return;
        }
        if let Some(evidence) = result.cast_lifecycle.as_ref() {
            error!(
                "❌ Bot {}: FAILED cast_lifecycle passed={} actions={} expectations={}/{} events={} logout={} failure={:?}",
                result.account,
                evidence.passed,
                evidence.actions.len(),
                evidence.expected_rules.iter().filter(|rule| rule.matched).count(),
                evidence.expected_rules.len(),
                evidence.events.len(),
                evidence.logout_confirmed,
                evidence.failure,
            );
            return;
        }
        if result.loot_race_smoke {
            error!(
                "❌ Bot {}: FAILED loot_race target={:?}/{:?}/counter={:?} party={} discovered={} opened={} list={:?} coins={:?} item_push={} removed={} money_notify={:?} coin_removed={} db_item={:?} db_money_delta={:?} relog={} failure={:?}",
                result.account,
                result.loot_race_target_entry,
                result.loot_race_target_spawn_guid,
                result.loot_race_target_runtime_counter,
                result.loot_race_party_confirmed,
                result.loot_race_target_discovered,
                result.loot_race_loot_opened,
                result.loot_race_loot_list_id,
                result.loot_race_loot_coins,
                result.loot_race_item_push_seen,
                result.loot_race_loot_removed_seen,
                result.loot_race_money_notify_amount,
                result.loot_race_coin_removed_seen,
                result.loot_race_db_item_total,
                result.loot_race_db_money_delta,
                result.loot_race_relog_verified,
                result.loot_race_failure,
            );
            return;
        }
        if result.group_capacity_race_smoke {
            error!(
                "❌ Bot {}: FAILED group_capacity_race group={:?} outcome={:?} final_members={:?} failure={:?}",
                result.account,
                result.group_capacity_group_id,
                result.group_capacity_outcome,
                result.group_capacity_final_member_count,
                result.group_capacity_failure,
            );
            return;
        }
        error!(
            "❌ Bot {}: FAILED login={{auth:{}, enum:{}, player:{}}} join={:?}/{:?} proposal={} group={} teleport_denied={:?}",
            result.account,
            result.world_auth,
            result.enum_characters,
            result.player_login_verified,
            result.join_result,
            result.join_detail,
            result.got_proposal,
            result.group_formed,
            result.teleport_denied_reason
        );
    }
}

pub(super) fn write_report_if_requested(
    cli: &CliOptions,
    dungeon_id: u32,
    timeout_secs: u64,
    require_proposal: bool,
    require_group: bool,
    auto_teleport: bool,
    login_only: bool,
    stand_state_smoke: bool,
    bank_smoke: bool,
    void_storage_smoke: bool,
    void_storage_query_capture: bool,
    homebind_smoke: bool,
    inventory_swap_smoke: bool,
    vendor_smoke: bool,
    equipment_set_race_smoke: bool,
    rested_xp_smoke: bool,
    detour_chase_capture: bool,
    creature_spell_capture: bool,
    cast_lifecycle: bool,
    loot_race_smoke: bool,
    loot_item_capture: bool,
    group_capacity_race_smoke: bool,
    quest_smoke: bool,
    results: &[BotRunResult],
) -> Result<()> {
    let path = cli.report_path.clone().unwrap_or_else(|| {
        format!(
            "/tmp/wow-bot-run-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        )
    });
    let report = RunReport {
        dungeon_id,
        timeout_secs,
        require_proposal,
        require_group,
        auto_teleport,
        login_only,
        stand_state_smoke,
        bank_smoke,
        void_storage_smoke,
        void_storage_query_capture,
        homebind_smoke,
        inventory_swap_smoke,
        vendor_smoke,
        equipment_set_race_smoke,
        rested_xp_smoke,
        detour_chase_capture,
        creature_spell_capture,
        cast_lifecycle,
        loot_race_smoke,
        loot_item_capture,
        group_capacity_race_smoke,
        quest_smoke,
        results: results.to_vec(),
    };
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&path, json)?;
    info!("Report written: {}", path);
    Ok(())
}
