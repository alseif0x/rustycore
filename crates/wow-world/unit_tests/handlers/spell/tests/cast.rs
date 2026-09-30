//! Spell handler cast scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[tokio::test]
async fn cancel_cast_clears_matching_active_cast_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
    install_active_spell_cast(&mut session, 12_345, cast_id);

    session
        .handle_cancel_cast(cancel_cast_packet(cast_id, 12_345))
        .await;

    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
}
#[tokio::test]
async fn cancel_cast_also_cancels_pending_spell_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
    let pending_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
    install_active_spell_cast(&mut session, 12_345, active_cast_id);
    install_pending_spell_cast_request(&mut session, 67_890, pending_cast_id);

    session
        .handle_cancel_cast(cancel_cast_packet(active_cast_id, 12_345))
        .await;

    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_fields_like_cpp(&packets[0]),
        (pending_cast_id, 67_890, 32),
        "C++ HandleCancelCastOpcode calls Player::CancelPendingCastRequest after interrupting"
    );
}
#[tokio::test]
async fn cancel_cast_mismatched_spell_preserves_active_cast_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
    install_active_spell_cast(&mut session, 12_345, cast_id);

    session
        .handle_cancel_cast(cancel_cast_packet(cast_id, 67_890))
        .await;

    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
}
#[tokio::test]
async fn cancel_cast_mismatch_cancels_pending_but_preserves_active_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
    let pending_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
    install_active_spell_cast(&mut session, 12_345, active_cast_id);
    install_pending_spell_cast_request(&mut session, 67_890, pending_cast_id);

    session
        .handle_cancel_cast(cancel_cast_packet(active_cast_id, 54_321))
        .await;

    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_fields_like_cpp(&packets[0]),
        (pending_cast_id, 67_890, 32),
        "Classic SpellHandler.cpp:263 cancels the pending request whenever a non-melee cast exists"
    );
}
#[tokio::test]
async fn self_res_listed_spell_casts_and_removes_self_res_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let spell_id = 20_001;

    session.set_player_guid(Some(ObjectGuid::create_player(1, 20_001)));
    session.set_spell_store(self_res_spell_store(spell_id));
    session.set_player_health_like_cpp(0, 100);
    session.add_represented_self_res_spell_like_cpp(spell_id);

    session.handle_self_res(int32_spell_packet(spell_id)).await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 35);
    assert!(!session.has_represented_self_res_spell_like_cpp(spell_id));
    assert!(!send_rx.is_empty());
}
#[tokio::test]
async fn cast_spell_applies_embedded_move_update_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_337;
    let moved_position = Position::new(33.0, 44.0, 55.0, 1.25);
    let move_update = MovementInfo {
        guid: player_guid,
        time: 12_345,
        position: moved_position,
        ..MovementInfo::default()
    };

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(10.0, 20.0, 30.0, 0.0));
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(basic_spell_store([spell_id]));

    session
        .handle_cast_spell(cast_spell_packet_with_move_update(
            spell_id,
            player_guid,
            Some(move_update),
        ))
        .await;

    assert_eq!(session.player_position_like_cpp(), Some(moved_position));
}
#[tokio::test]
async fn cast_spell_uses_represented_override_spell_info_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let original_spell_id = 13_347;
    let override_spell_id = 13_348;

    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![original_spell_id]);
    session.set_spell_store(basic_spell_store([original_spell_id, override_spell_id]));
    session.add_represented_override_spell_like_cpp(original_spell_id, override_spell_id);

    session
        .handle_cast_spell(cast_spell_packet(original_spell_id, player_guid))
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 4);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        &packets[1][..2],
        &(ServerOpcodes::SpellStart as u16).to_le_bytes()
    );
    assert_eq!(
        spell_go_spell_id_like_cpp(&packets[2]),
        override_spell_id,
        "C++ Player::GetCastSpellInfo resolves m_overrideSpells after the original spell known check"
    );
}
#[tokio::test]
async fn cast_spell_falls_back_when_represented_override_missing_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let original_spell_id = 13_349;
    let missing_override_spell_id = 13_350;

    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![original_spell_id]);
    session.set_spell_store(basic_spell_store([original_spell_id]));
    session.add_represented_override_spell_like_cpp(original_spell_id, missing_override_spell_id);

    session
        .handle_cast_spell(cast_spell_packet(original_spell_id, player_guid))
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 4);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        &packets[1][..2],
        &(ServerOpcodes::SpellStart as u16).to_le_bytes()
    );
    assert_eq!(
        spell_go_spell_id_like_cpp(&packets[2]),
        original_spell_id,
        "C++ Player::GetCastSpellInfo ignores override entries whose SpellInfo cannot be resolved"
    );
}
#[tokio::test]
async fn cast_spell_with_mana_cost_deducts_power_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_351;
    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_mana_like_cpp(&mut session, 500, 1000);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_power_cost_like_cpp(
        spell_id, 50, 10.0,
    ));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert_eq!(
        canonical_player_mana_like_cpp(&mut session),
        350,
        "C++ CalcPowerCost charges flat ManaCost plus PowerCostPct of create mana"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::CastFailed),
        "accepted casts must not send SPELL_FAILED_NO_POWER"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::UpdateObject),
        "accepted casts with represented power cost must send a player values update"
    );
}
#[tokio::test]
async fn cast_spell_power_spend_survives_canonical_resync_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_356;
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_loaded_player_powers_like_cpp([500, 222, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        session.represented_player_power_values_like_cpp().unwrap()[1],
        222
    );
    assert!(session.sync_canonical_player_primary_power_like_cpp(
        PowerType::Mana,
        500,
        1_000,
        1_000,
    ));
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_power_cost_like_cpp(
        spell_id, 50, 10.0,
    ));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert_eq!(canonical_player_mana_like_cpp(&mut session), 350);
    assert_eq!(
        session.represented_player_power_values_like_cpp().unwrap()[0],
        350,
        "represented session power must mirror Spell::TakePower before later AddToMap-style resync"
    );
    assert_eq!(
        session.represented_player_power_values_like_cpp().unwrap()[1],
        222
    );
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        canonical_player_mana_like_cpp(&mut session),
        350,
        "resync from the session snapshot must not resurrect pre-cast mana"
    );
    assert_eq!(
        session.represented_player_power_values_like_cpp().unwrap()[1],
        222
    );
    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .unwrap();
    assert_eq!(snapshot.powers[0], Some(350));
    assert_eq!(
        snapshot.powers[1],
        Some(222),
        "saving power1 after a cast must preserve other loaded character power columns"
    );
    let _ = drain_server_opcodes(&send_rx);
}
#[tokio::test]
async fn cast_spell_with_insufficient_mana_fails_no_power_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_352;
    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_mana_like_cpp(&mut session, 149, 1000);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_power_cost_like_cpp(
        spell_id, 50, 10.0,
    ));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert_eq!(
        canonical_player_mana_like_cpp(&mut session),
        149,
        "failed casts must not deduct power"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::NoPower as i32
    );
}
#[tokio::test]
async fn cast_spell_late_check_failure_does_not_deduct_power_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_353;
    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_mana_like_cpp(&mut session, 500, 1000);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_cost_and_missing_focus_like_cpp(
        spell_id, 150,
    ));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert_eq!(
        canonical_player_mana_like_cpp(&mut session),
        500,
        "C++ Spell::TakePower happens after represented CheckCast failures such as missing spell focus"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::RequiresSpellFocus as i32
    );
}
#[tokio::test]
async fn retained_direct_cast_late_power_failure_restores_legacy_timestamp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 22);
    let spell_id = 13_357;
    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_mana_like_cpp(&mut session, 149, 1000);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_power_cost_like_cpp(
        spell_id, 50, 10.0,
    ));
    let previous_last_spell_cast_time =
        Some(std::time::Instant::now() - std::time::Duration::from_millis(5_000));
    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time = previous_last_spell_cast_time
    });
    install_active_spell_cast(&mut session, spell_id, cast_id);
    session.mutate_cast_execution_like_cpp(|state| {
        if let Some(active) = state.active.as_mut() {
            active.cast_start_time =
                std::time::Instant::now() - std::time::Duration::from_millis(30_000);
            active.metadata = SpellCastMetadata {
                from_client: true,
                original_cast_id: cast_id,
                ..SpellCastMetadata::default()
            };
        }
    });

    session.tick_active_spell_cast().await;

    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert_eq!(canonical_player_mana_like_cpp(&mut session), 149);
    assert_eq!(
        session.last_spell_cast_time_like_cpp().flatten(),
        previous_last_spell_cast_time,
        "Shared-executor consumers retain their legacy timestamp rollback contract; normal client preparation has separate GCD coverage"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[0]),
        SpellCastResult::NoPower as i32
    );
}
#[tokio::test]
async fn cast_spell_rejects_gcd_outside_spell_queue_window_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_338;
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_global_cooldown(spell_id, 1_500));
    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time = Some(std::time::Instant::now())
    });

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[0]),
        SpellCastResult::SpellInProgress as i32,
        "C++ HandleCastSpellOpcode sends SPELL_FAILED_SPELL_IN_PROGRESS when CanRequestSpellCast rejects the request"
    );
}
#[tokio::test]
async fn cast_spell_queues_within_spell_queue_window_and_executes_after_gcd_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_339;
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_global_cooldown(spell_id, 1_500));
    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_200))
    });

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(
        session
            .pending_spell_cast_for_test_like_cpp()
            .as_ref()
            .is_some_and(|pending| pending.spell_id == spell_id)
    );
    assert!(
        send_rx.is_empty(),
        "C++ RequestSpellCast only queues while GCD is still active"
    );

    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_500))
    });
    session.tick_pending_spell_cast_request_like_cpp().await;

    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    assert!(opcodes.contains(&ServerOpcodes::CooldownEvent));
}
#[tokio::test]
async fn cast_spell_rejects_active_cast_outside_spell_queue_window_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 20);
    let queued_spell_id = 13_340;
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![12_345, queued_spell_id]);
    session.set_spell_store(basic_spell_store([12_345, queued_spell_id]));
    install_active_spell_cast(&mut session, 12_345, active_cast_id);

    session
        .handle_cast_spell(cast_spell_packet(queued_spell_id, player_guid))
        .await;

    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[0]),
        SpellCastResult::SpellInProgress as i32
    );
}
#[tokio::test]
async fn cast_spell_queues_near_active_cast_finish_and_executes_after_cast_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 21);
    let queued_spell_id = 13_341;
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![12_345, queued_spell_id]);
    session.set_spell_store(basic_spell_store([12_345, queued_spell_id]));
    install_active_spell_cast(&mut session, 12_345, active_cast_id);
    session.mutate_cast_execution_like_cpp(|state| {
        if let Some(active) = state.active.as_mut() {
            active.cast_start_time =
                std::time::Instant::now() - std::time::Duration::from_millis(29_700);
        }
    });

    session
        .handle_cast_spell(cast_spell_packet(queued_spell_id, player_guid))
        .await;

    assert!(
        session
            .pending_spell_cast_for_test_like_cpp()
            .as_ref()
            .is_some_and(|pending| pending.spell_id == queued_spell_id)
    );
    assert!(send_rx.is_empty());

    session.mutate_cast_execution_like_cpp(|state| {
        if let Some(active) = state.active.as_mut() {
            active.cast_start_time =
                std::time::Instant::now() - std::time::Duration::from_millis(30_000);
        }
    });
    session.tick_active_spell_cast().await;
    session.tick_pending_spell_cast_request_like_cpp().await;

    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    assert!(opcodes.contains(&ServerOpcodes::CooldownEvent));
}
