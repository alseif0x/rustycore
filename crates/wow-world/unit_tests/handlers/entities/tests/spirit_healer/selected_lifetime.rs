use super::*;

#[tokio::test]
async fn selected_lifetime_sickness_expiry_restores_stats_through_map_values() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_429).await;
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, None);
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("race-selected sickness SpellInfo");
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::unit::DeathState::Alive)
        })
        .expect("canonical Player owner");
    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            player_guid,
            &spell_info,
            600_000,
            0,
            0x88,
            0,
            0,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
        .expect("apply sickness through the canonical owner");

    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical map manager");
    let applied = manager
        .lock()
        .expect("canonical map manager")
        .find_map_mut(571, 0)
        .expect("Player map")
        .map_mut()
        .send_object_updates_like_cpp();
    assert_eq!(applied.player_values_updates.len(), 1);
    assert_eq!(
        applied.player_values_updates[0]
            .values_update
            .derived_stats
            .as_ref()
            .expect("Map snapshot carries effective stats")
            .effective_combat_stats
            .stats[wow_constants::Stats::Strength as usize],
        2
    );
    let slot = visible_sickness_aura(&session, SICKNESS_SPELL)[0].slot;
    prepare_loaded_sickness_for_recast(&mut session, slot, player_guid);
    let healer = insert_healer(
        &session,
        43_430,
        session.player_position_like_cpp().expect("Player position"),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;
    let refreshed = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(refreshed.len(), 1);
    assert_eq!(refreshed[0].duration_total, 900_000);
    assert_eq!(refreshed[0].duration_remaining, 60_000);
    let _ = drain_packets(&send_rx);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let aura = player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(slot)
                .expect("active sickness application");
            aura.applied_at = std::time::Instant::now() - std::time::Duration::from_secs(3_600);
        })
        .expect("canonical Player owner");

    let countdown = wow_world_spell::tick_selected_aura_lifetimes(&mut session, 59_999)
        .expect("selected lifetime uses the refreshed remaining duration, not wall time");
    assert_eq!(countdown.decremented, 1);
    assert_eq!(countdown.expired, 0);
    assert_eq!(
        visible_sickness_aura(&session, SICKNESS_SPELL)[0].duration_remaining,
        1
    );

    let result = wow_world_spell::tick_selected_aura_lifetimes(&mut session, 1)
        .expect("selected sickness expires through Aura removal participants");
    assert_eq!(result.decremented, 1);
    assert_eq!(result.expired, 1);
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    let removal_packets = drain_packets(&send_rx);
    assert!(
        removal_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
    );
    assert!(
        removal_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );

    let removed = manager
        .lock()
        .expect("canonical map manager")
        .find_map_mut(571, 0)
        .expect("Player map")
        .map_mut()
        .send_object_updates_like_cpp();
    assert_eq!(removed.player_values_updates.len(), 1);
    assert_eq!(
        removed.player_values_updates[0]
            .values_update
            .derived_stats
            .as_ref()
            .expect("expiry dirties the derived stat snapshot")
            .effective_combat_stats
            .stats[wow_constants::Stats::Strength as usize],
        10
    );
}
