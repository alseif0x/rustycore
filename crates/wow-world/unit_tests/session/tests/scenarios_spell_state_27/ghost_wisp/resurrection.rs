use super::*;

fn selected_slots(fixture: &GhostFixture) -> (Option<u8>, Option<u8>) {
    let manager = fixture.canonical.lock().expect("canonical manager");
    let auras = &manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras;
    let slot_for = |spell_id| {
        auras
            .runtime_applications_like_cpp()
            .values()
            .find_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
    };
    (slot_for(GHOST), slot_for(WISP))
}

fn continuation_phase(fixture: &GhostFixture) -> Option<wow_entities::PlayerResurrectionPhase> {
    fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .flatten()
}

fn canonical_death_state(fixture: &GhostFixture) -> Option<wow_constants::DeathState> {
    fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().death_state())
}

fn set_resurrection_teleport_completed(fixture: &mut GhostFixture) {
    let resurrecter = ObjectGuid::create_player(1, 96_111);
    let request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter,
        map_id: 571,
        position: Position::ZERO,
        health: 450,
        mana: 120,
        aura: 0,
    };
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(0);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
            player.set_resurrection_request_like_cpp(request);
            let continuation = player
                .accept_resurrection_request(resurrecter)
                .expect("accepted resurrection request");
            assert_eq!(
                continuation.phase,
                wow_entities::PlayerResurrectionPhase::TeleportInFlight
            );
        })
        .expect("canonical Player");
    assert!(wow_world_entities::mark_player_resurrection_teleport_completed(&mut fixture.session));
}

#[tokio::test]
async fn resurrection_native_failure_is_nonreplayable_and_keeps_player_dead() {
    let mut fixture = fixture();
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    let (ghost_slot, wisp_slot) = selected_slots(&fixture);
    let ghost_slot = ghost_slot.expect("active Ghost application");
    let wisp_slot = wisp_slot.expect("active Wisp application");
    outgoing(&fixture);

    set_resurrection_teleport_completed(&mut fixture);

    // The runtime applications remain installed, but removing Wisp cannot
    // prepare its selected effects without SpellInfo. Its preflight fails
    // before the Ghost removal or Alive transition.
    fixture
        .session
        .set_spell_store(Arc::new(wow_data::SpellStore::new()));
    let (realm_tx, realm_rx) = flume::bounded(8);
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    let generators = fixture.session.id_generators_for_test_like_cpp();
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
    let catalogs = wow_world_entities::ResurrectionOperationCatalogs {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
    };

    assert!(
        !wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    assert_eq!(
        continuation_phase(&fixture),
        Some(wow_entities::PlayerResurrectionPhase::ResurrectPlayerInFlight)
    );
    assert!(!fixture.session.player_is_alive_like_cpp());
    assert_eq!(
        canonical_death_state(&fixture),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 0);
    assert_eq!(
        selected_slots(&fixture),
        (Some(ghost_slot), Some(wisp_slot))
    );
    assert!(
        outgoing(&fixture).is_empty(),
        "no values, aura, speed, or visibility packet"
    );
    let death_release = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(
        death_release.len(),
        1,
        "the earlier native packet is not rolled back"
    );
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&death_release[0])
            .read_uint16()
            .expect("DeathReleaseLoc opcode"),
        ServerOpcodes::DeathReleaseLoc as u16
    );

    // The retained InFlight marker cannot rerun native effects on retry.
    assert!(
        !wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    assert_eq!(
        continuation_phase(&fixture),
        Some(wow_entities::PlayerResurrectionPhase::ResurrectPlayerInFlight)
    );
    assert!(!fixture.session.player_is_alive_like_cpp());
    assert_eq!(
        canonical_death_state(&fixture),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 0);
    assert!(
        realm_rx.try_iter().next().is_none(),
        "no second DeathReleaseLoc"
    );
    assert!(outgoing(&fixture).is_empty());
}

#[tokio::test]
async fn resurrection_native_failure_holds_when_applied_order_reference_is_missing() {
    let mut fixture = fixture();
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    let (ghost_slot, wisp_slot) = selected_slots(&fixture);
    let ghost_slot = ghost_slot.expect("active Ghost application");
    let wisp_slot = wisp_slot.expect("active Wisp application");
    outgoing(&fixture);
    set_resurrection_teleport_completed(&mut fixture);

    // Runtime state proves a selected Wisp remains, but the canonical Vec no
    // longer establishes its C++ application order. Hold the transition.
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .applied_auras
                .retain(|applied| applied.slot != wisp_slot);
        })
        .expect("canonical Player");

    let (realm_tx, realm_rx) = flume::bounded(8);
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    let generators = fixture.session.id_generators_for_test_like_cpp();
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
    let catalogs = wow_world_entities::ResurrectionOperationCatalogs {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
    };

    assert!(
        !wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    assert_eq!(
        continuation_phase(&fixture),
        Some(wow_entities::PlayerResurrectionPhase::ResurrectPlayerInFlight)
    );
    assert!(!fixture.session.player_is_alive_like_cpp());
    assert_eq!(
        canonical_death_state(&fixture),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 0);
    assert_eq!(
        selected_slots(&fixture),
        (Some(ghost_slot), Some(wisp_slot))
    );
    assert!(outgoing(&fixture).is_empty());
    let death_release = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(death_release.len(), 1);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&death_release[0])
            .read_uint16()
            .expect("DeathReleaseLoc opcode"),
        ServerOpcodes::DeathReleaseLoc as u16
    );

    assert!(
        !wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    assert_eq!(
        continuation_phase(&fixture),
        Some(wow_entities::PlayerResurrectionPhase::ResurrectPlayerInFlight)
    );
    assert!(!fixture.session.player_is_alive_like_cpp());
    assert_eq!(
        canonical_death_state(&fixture),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 0);
    assert!(
        realm_rx.try_iter().next().is_none(),
        "no second DeathReleaseLoc"
    );
    assert!(outgoing(&fixture).is_empty());
}

#[tokio::test]
async fn resurrection_request_clears_ghost_and_wisp_before_completing() {
    let mut fixture = fixture();
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    outgoing(&fixture);
    set_resurrection_teleport_completed(&mut fixture);

    let (realm_tx, realm_rx) = flume::bounded(8);
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    let generators = fixture.session.id_generators_for_test_like_cpp();
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
    let catalogs = wow_world_entities::ResurrectionOperationCatalogs {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
    };

    assert!(
        wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    let canonical_vitals = fixture.session.core.with_owned_player_like_cpp(|player| {
        (player.unit().is_alive(), player.unit().data().health)
    });
    assert!(
        fixture.session.player_is_alive_like_cpp(),
        "canonical vitals after reported completion: {canonical_vitals:?}"
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 450);
    assert_eq!(selected_slots(&fixture), (None, None));
    assert_eq!(continuation_phase(&fixture), None);
    let death_release = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(death_release.len(), 1);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&death_release[0])
            .read_uint16()
            .expect("DeathReleaseLoc opcode"),
        ServerOpcodes::DeathReleaseLoc as u16
    );
}

#[tokio::test]
async fn ghost_wisp_removal_publishes_aura_update_before_speed_packets() {
    let mut fixture = fixture();
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    let (_, wisp_slot) = selected_slots(&fixture);
    let wisp_slot = wisp_slot.expect("active Wisp application");
    outgoing(&fixture);

    fixture
        .session
        .remove_aura(wisp_slot)
        .expect("selected Wisp aura removal");
    let packets = outgoing(&fixture);
    let opcodes = packets
        .iter()
        .map(|packet| opcode(packet))
        .collect::<Vec<_>>();
    let aura_update_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AuraUpdate as u16)
        .expect("AuraUpdate removal packet");
    let run_speed_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveSetRunSpeed as u16)
        .expect("self run-speed update");
    let swim_speed_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveSetSwimSpeed as u16)
        .expect("self swim-speed update");
    assert!(aura_update_index < run_speed_index);
    assert!(aura_update_index < swim_speed_index);

    let mut aura_update = wow_packet::WorldPacket::from_bytes(&packets[aura_update_index]);
    assert_eq!(
        aura_update.read_uint16().expect("AuraUpdate opcode"),
        ServerOpcodes::AuraUpdate as u16
    );
    let _update_all = aura_update.read_bit().expect("AuraUpdate update-all bit");
    assert_eq!(aura_update.read_bits(9).expect("AuraUpdate slot count"), 1);
    assert_eq!(
        aura_update.read_uint8().expect("removed Wisp slot"),
        wisp_slot
    );
    assert!(!aura_update.read_bit().expect("AuraUpdate removal bit"));
}
