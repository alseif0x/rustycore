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
            for (power, index, max) in [
                (wow_constants::PowerType::Mana, 0, 2_000),
                (wow_constants::PowerType::Rage, 1, 100),
                (wow_constants::PowerType::Focus, 2, 80),
                (wow_constants::PowerType::Energy, 3, 100),
            ] {
                player.unit_mut().set_power_index(power, Some(index));
                player.unit_mut().set_max_power(power, max);
            }
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
        graveyards: None,
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
        graveyards: None,
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
    assert!(
        !fixture
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player),
        "C++ HaveAtClient(self) does not seed the discovery ledger"
    );
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
        graveyards: None,
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
    assert_eq!(
        fixture.session.visibility.last_visibility_pos_like_cpp(),
        Some(Position::ZERO),
        "forced visibility completes while native health and power VALUES remain dirty"
    );
    let direct_packets = outgoing(&fixture);
    assert!(
        direct_packets
            .iter()
            .all(|packet| opcode(packet) != ServerOpcodes::UpdateObject as u16),
        "resurrection request values stay pending for the native Map owner"
    );
    {
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        let power_indexes = {
            let player = manager
                .find_map_mut(571, 0)
                .expect("current World map")
                .map_mut()
                .get_typed_player_mut(fixture.player)
                .expect("request Player belongs to the canonical map");
            let object = player.unit().world().object();
            assert!(object.is_in_world());
            assert!(object.is_object_updated());
            assert!(object.is_need_notify(wow_entities::ObjectNotifyFlags::VISIBILITY_CHANGED));
            [
                wow_constants::PowerType::Mana,
                wow_constants::PowerType::Rage,
                wow_constants::PowerType::Energy,
                wow_constants::PowerType::Focus,
            ]
            .map(|power| {
                player
                    .unit()
                    .get_power_index(power)
                    .expect("native Player power index")
            })
        };
        let _ = manager.update(1);
        let map = manager.find_map(571, 0).expect("current World map");
        let summary = map.last_send_object_updates_summary_like_cpp();
        let updates = &summary.player_values_updates;
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].guid, fixture.player);
        let values_update = updates[0].values_update.clone();
        let unit = values_update
            .unit_data
            .as_ref()
            .expect("Map snapshot contains request Unit fields");
        assert_eq!(unit.values.health, 450);
        for (index, value) in power_indexes.into_iter().zip([120, 0, 100, 80]) {
            assert_eq!(unit.values.power[index], value);
        }
        assert!(
            !map.map()
                .get_typed_player(fixture.player)
                .unwrap()
                .unit()
                .world()
                .object()
                .is_object_updated()
        );
    }
    assert_eq!(
        fixture
            .session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1
    );
    let map_packets = outgoing(&fixture);
    assert_eq!(map_packets.len(), 1);
    assert_eq!(opcode(&map_packets[0]), ServerOpcodes::UpdateObject as u16);
    assert_eq!(
        fixture
            .session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0
    );
    let death_release = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(death_release.len(), 5);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&death_release[0])
            .read_uint16()
            .expect("DeathReleaseLoc opcode"),
        ServerOpcodes::DeathReleaseLoc as u16
    );
    for (packet, (power, value)) in death_release[1..].iter().zip([
        (wow_constants::PowerType::Mana, 120),
        (wow_constants::PowerType::Rage, 0),
        (wow_constants::PowerType::Energy, 100),
        (wow_constants::PowerType::Focus, 80),
    ]) {
        assert_eq!(
            *packet,
            wow_packet::packets::combat::PowerUpdate {
                guid: fixture.player,
                powers: vec![(value, power as u8)],
            }
            .to_bytes()
        );
    }
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

#[tokio::test]
async fn kill_player_forces_stationary_self_and_recipient_visibility_like_cpp() {
    let mut fixture = fixture();
    let registry = super::observers::setup_registry(&mut fixture);
    let observer_guid = ObjectGuid::create_player(1, 4_630_012);
    let mut observer = super::observers::add_observer(
        &fixture,
        &registry,
        4_630_012,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        false,
    );
    wow_world_entities::apply_move_init_active_mover_complete_like_cpp(&mut observer.session, 0);
    let source_position = fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("canonical Player remains in the Map");
    let observer_position = observer
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("observer remains in the Map");
    fixture
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(source_position);
    observer
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(observer_position);
    assert_eq!(
        fixture.session.visibility.last_visibility_pos_like_cpp(),
        Some(source_position)
    );
    assert_eq!(
        observer.session.visibility.last_visibility_pos_like_cpp(),
        Some(observer_position)
    );
    assert!(
        !fixture
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&observer_guid)
    );
    assert!(
        !observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );

    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_health(0);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
        })
        .expect("canonical Player owns the native death state");
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();

    // No relocation or Map::update runs between the before/after GUID snapshots.
    assert!(
        wow_world_entities::kill_player_like_cpp(&mut fixture.session, &creature_spawns).await,
        "the same Player residence survives the forced visibility await"
    );
    assert_eq!(
        canonical_death_state(&fixture),
        Some(wow_constants::DeathState::Corpse)
    );
    assert!(
        fixture
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&observer_guid)
    );
    assert!(
        !observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );
    assert_eq!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().position()),
        Some(source_position),
        "forced visibility does not move the source Player"
    );
    assert_eq!(
        observer
            .session
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().position()),
        Some(observer_position),
        "forced visibility does not move the recipient"
    );

    assert!(
        observer
            .session
            .core
            .flags
            .visibility_refresh_pending_like_cpp
            .load(std::sync::atomic::Ordering::Acquire),
        "the stationary recipient retains the forced visibility obligation before its pump"
    );
    super::observers::flush_observer(&mut observer).await;
    assert!(
        observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );
    assert!(
        observer
            .output
            .try_iter()
            .any(|packet| opcode(&packet) == ServerOpcodes::UpdateObject as u16)
    );
}

#[tokio::test]
async fn resurrection_forces_stationary_self_and_recipient_visibility_like_cpp() {
    let mut fixture = fixture();
    let registry = super::observers::setup_registry(&mut fixture);
    let observer_guid = ObjectGuid::create_player(1, 4_630_013);
    let mut observer = super::observers::add_observer(
        &fixture,
        &registry,
        4_630_013,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        false,
    );
    wow_world_entities::apply_move_init_active_mover_complete_like_cpp(&mut observer.session, 0);
    cast_selected(&mut fixture, GHOST, None).await;
    assert!(selected_slots(&fixture).0.is_some(), "Ghost Aura is active");
    let _ = outgoing(&fixture);
    set_resurrection_teleport_completed(&mut fixture);
    assert!(
        !fixture
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&observer_guid)
    );
    assert!(
        !observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );
    let source_position = fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("canonical Player remains in the Map");
    let observer_position = observer
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("observer remains in the Map");
    fixture
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(source_position);
    observer
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(observer_position);
    assert_eq!(
        fixture.session.visibility.last_visibility_pos_like_cpp(),
        Some(source_position)
    );
    assert_eq!(
        observer.session.visibility.last_visibility_pos_like_cpp(),
        Some(observer_position)
    );

    let generators = fixture.session.id_generators_for_test_like_cpp();
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
    let catalogs = wow_world_entities::ResurrectionOperationCatalogs {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: None,
    };
    // No relocation or Map::update runs between the before/after GUID snapshots.
    assert!(
        wow_world_entities::continue_player_resurrection_after_teleport(
            &mut fixture.session,
            catalogs,
        )
        .await
    );
    assert_eq!(fixture.session.player_health_like_cpp(), 450);
    assert_eq!(
        selected_slots(&fixture).0,
        None,
        "resurrection removes Ghost Aura"
    );
    assert!(
        fixture
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&observer_guid)
    );
    assert_eq!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().position()),
        Some(source_position),
        "forced visibility does not move the source Player"
    );
    assert_eq!(
        observer
            .session
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().position()),
        Some(observer_position),
        "forced visibility does not move the recipient"
    );

    assert!(
        observer
            .session
            .core
            .flags
            .visibility_refresh_pending_like_cpp
            .load(std::sync::atomic::Ordering::Acquire),
        "the stationary recipient retains the forced visibility obligation before its pump"
    );
    super::observers::flush_observer(&mut observer).await;
    assert!(
        observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );
    assert!(
        observer
            .output
            .try_iter()
            .any(|packet| opcode(&packet) == ServerOpcodes::UpdateObject as u16)
    );
}

#[tokio::test]
async fn out_of_world_forced_visibility_preserves_stationary_cache_and_skips_recipients() {
    let mut fixture = fixture();
    let registry = super::observers::setup_registry(&mut fixture);
    let mut observer = super::observers::add_observer(
        &fixture,
        &registry,
        4_630_014,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        false,
    );
    let source_position = fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("canonical Player remains owned after detaching from world");
    let observer_position = observer
        .session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().position())
        .expect("observer remains in the Map");
    fixture
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(source_position);
    observer
        .session
        .visibility
        .set_last_visibility_pos_like_cpp(observer_position);
    assert_eq!(
        fixture.session.visibility.last_visibility_pos_like_cpp(),
        Some(source_position)
    );
    assert_eq!(
        observer.session.visibility.last_visibility_pos_like_cpp(),
        Some(observer_position)
    );
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let _ = player.unit_mut().remove_from_world_like_cpp();
        })
        .expect("out-of-world Player remains the canonical owner");
    let _ = outgoing(&fixture);
    let _ = observer.output.try_iter().collect::<Vec<_>>();
    let creature_spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();

    assert!(
        !wow_world_application::force_player_visibility_with_catalogs_like_cpp(
            &mut fixture.session,
            &creature_spawns,
        )
        .await
    );
    assert_eq!(
        fixture.session.visibility.last_visibility_pos_like_cpp(),
        Some(source_position),
        "out-of-world force leaves the source visibility cache untouched"
    );
    assert!(outgoing(&fixture).is_empty(), "no self packets are emitted");

    super::observers::flush_observer(&mut observer).await;
    assert_eq!(
        observer.session.visibility.last_visibility_pos_like_cpp(),
        Some(observer_position),
        "no recipient refresh command was queued"
    );
    assert!(
        !observer
            .session
            .core
            .client_visible_guids_like_cpp
            .contains(&fixture.player)
    );
    assert!(observer.output.try_iter().next().is_none());
}
