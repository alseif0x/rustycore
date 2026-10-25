use super::*;

#[tokio::test]
async fn high_level_sickness_recast_preserves_duration_without_a_shortening_request() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_419).await;
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 20, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("current race-selected sickness SpellInfo");
    let player_guid = session.player_guid().expect("canonical Player GUID");
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
        .expect("install source sickness Aura before high-level recast");
    let aura = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("installed sickness Aura");
    prepare_loaded_sickness_for_recast(&mut session, aura.slot, player_guid);
    let applied_at_before = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(aura.slot)
                .expect("prepared canonical sickness Aura")
                .applied_at
        })
        .expect("canonical Player owner");
    let _ = drain_packets(&send_rx);

    let healer = insert_healer(
        &session,
        43_420,
        session.player_position_like_cpp().expect("Player position"),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    let refreshed = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(refreshed.len(), 1);
    assert_eq!(refreshed[0].slot, aura.slot);
    assert_eq!(refreshed[0].stack_count, 1);
    assert_eq!(refreshed[0].duration_total, 900_000);
    assert_eq!(refreshed[0].duration_remaining, 10_000);
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_application_like_cpp(aura.slot)
                    .expect("refreshed canonical sickness Aura")
                    .applied_at
            })
            .expect("canonical Player owner"),
        applied_at_before,
        "a high-level cast with no SetDuration request keeps the old timer origin"
    );
    let (_, metadata, _) = refreshed_sickness_state(&session, aura.slot);
    assert_eq!(metadata.charges, 5);

    let updates = drain_packets(&send_rx)
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|update| update.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert_eq!(updates.len(), 1, "the unchanged Aura duration is published");
    assert_eq!(updates[0].duration_ms, Some(900_000));
    assert_eq!(updates[0].remaining_ms, Some(10_000));
}
