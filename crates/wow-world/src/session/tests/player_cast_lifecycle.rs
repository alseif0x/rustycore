//! Production-backed Player cast identity, admission and recipient fences.
//!
//! These fixtures deliberately install the same canonical `Player` owner that
//! the Session adapters resolve at runtime.  A map-local cast sequence is the
//! only creature-side stand-in here: the creature generator call below is a
//! direct map-generator check, not a second cast authority.

use super::*;
use crate::session::mailbox::{
    DurableCreatureRuntimeCommandsLikeCpp, SendPlayerSpellIfVisibleLikeCppCommand,
    SharedClientVisibleGuidsLikeCpp,
};
use std::sync::{Arc, Mutex};
use wow_packet::ServerPacket;

const TEST_SPELL_ID: i32 = 133;

fn install_canonical_player(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    position: Position,
) {
    session.set_player_guid(Some(guid));
    session.set_canonical_map_manager(Arc::clone(canonical));
    add_canonical_test_player_on_map(canonical, guid, position, map_id, instance_id);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

fn request(guid: ObjectGuid, id: i64) -> RepresentedPendingSpellCastRequestLikeCpp {
    RepresentedPendingSpellCastRequestLikeCpp {
        cast_id: ObjectGuid::new(6, id),
        spell_id: TEST_SPELL_ID,
        casting_unit_guid: guid,
        target_guid: guid,
        target_data: Default::default(),
        spell_visual: Default::default(),
        metadata: Default::default(),
    }
}

fn minimal_spell() -> SpellInfo {
    SpellInfo {
        spell_id: TEST_SPELL_ID,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

fn prepared_cast(guid: ObjectGuid, revision: u64) -> SpellCastState {
    SpellCastState {
        spell_id: TEST_SPELL_ID,
        target_guid: guid,
        target_data: Default::default(),
        cast_id: ObjectGuid::new(6, 0xCA57),
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 0,
        spell_visual: Default::default(),
        metadata: SpellCastMetadata {
            client_cast_id: Some(ObjectGuid::new(6, 0xC1)),
            prepared_residence_revision: Some(revision),
            from_client: true,
            ..Default::default()
        },
    }
}

fn spell_start_bytes(caster: ObjectGuid) -> Vec<u8> {
    wow_packet::packets::spell::SpellStartPkt {
        cast_data: Default::default(),
        caster,
        cast_id: ObjectGuid::new(6, 0xBEEF),
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: TEST_SPELL_ID,
        visual: Default::default(),
        cast_flags: 0x2,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: Default::default(),
    }
    .to_bytes()
}

fn player_registration_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    map_id: u16,
    instance_id: u32,
    position: Position,
    committed_visibility: SharedClientVisibleGuidsLikeCpp,
    durable: Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>,
) -> PlayerSessionRegistrationLikeCpp {
    let mut info = broadcast_info(guid, send_tx);
    info.placement.map_id = map_id;
    info.placement.instance_id = instance_id;
    info.placement.position = position;
    info.client_visible_guids_like_cpp = committed_visibility;
    info.durable_creature_runtime_commands_like_cpp = durable;
    info
}

#[test]
fn player_and_map_cast_generators_share_one_map_sequence_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_901);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);

    let first = session
        .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
        .expect("active canonical Player admits a cast identity");
    assert_eq!(first.high_type(), HighGuid::Cast);
    assert_eq!(first.sub_type(), 3, "SPELL_CAST_SOURCE_NORMAL");
    assert_eq!(first.map_id(), 571);
    assert_eq!(first.entry(), TEST_SPELL_ID as u32);
    assert_eq!(first.counter(), 1);

    let creature_stand_in = canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .generate_low_guid_like_cpp(HighGuid::Cast)
        .expect("the same map owns the cast sequence");
    assert_eq!(creature_stand_in, 2);

    let second = session
        .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
        .expect("the Player remains active after the direct map check");
    assert_eq!(second.counter(), 3);
}

#[test]
fn map_cast_generators_are_independent_per_instance_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let guid_a = ObjectGuid::create_player(1, 58_902);
    let guid_b = ObjectGuid::create_player(1, 58_903);
    let (mut session_a, _, _) = make_session();
    let (mut session_b, _, _) = make_session();
    install_canonical_player(&mut session_a, &canonical, guid_a, 571, 0, Position::ZERO);
    install_canonical_player(&mut session_b, &canonical, guid_b, 571, 1, Position::ZERO);

    let cast_a = session_a
        .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
        .unwrap();
    let cast_b = session_b
        .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
        .unwrap();
    assert_eq!(cast_a.counter(), 1);
    assert_eq!(
        cast_b.counter(),
        1,
        "Map instances own independent sequences"
    );
    assert_eq!(cast_a.map_id(), cast_b.map_id());
}

#[test]
fn detached_or_stale_player_handles_consume_no_cast_ids_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_904);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert!(
        session
            .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
            .is_none(),
        "detached admission fails before touching Map::GenerateLowGuid"
    );

    let replacement = {
        let mut player = Box::new(Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        let mut manager = canonical.lock().unwrap();
        let replacement = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(replacement, wow_map::MapKey::new(571, 0), Position::ZERO)
            .unwrap();
        replacement
    };
    assert_ne!(replacement, session.player_handle_like_cpp.unwrap());
    assert!(
        session
            .next_represented_spell_cast_guid_like_cpp(TEST_SPELL_ID)
            .is_none(),
        "a stale generation fails closed before allocation"
    );

    let next = canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_max_low_guid_like_cpp(HighGuid::Cast)
        .unwrap();
    assert_eq!(next, 1, "rejected detached/stale attempts consumed no ID");
}

#[test]
fn reentry_changes_residence_revision_and_denies_prepared_cast_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_905);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);
    let handle = session.player_handle_like_cpp.unwrap();
    let first_revision = canonical
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle)
        .unwrap()
        .1;
    assert!(session.set_active_spell_cast_like_cpp(Some(prepared_cast(guid, first_revision))));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), Position::ZERO)
            .unwrap();
    }
    let second_revision = canonical
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle)
        .unwrap()
        .1;
    assert_ne!(first_revision, second_revision);
    session.publish_player_cast_frame_like_cpp(
        prepared_cast(guid, first_revision).metadata,
        spell_start_bytes(guid),
    );
    assert!(
        send_rx.try_recv().is_err(),
        "old residence cannot publish after reentry"
    );
    assert!(
        session.take_ready_player_cast_like_cpp().is_none(),
        "a prepared cast cannot cross an away-and-back residence"
    );
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
}

#[test]
fn canonical_admission_replaces_pending_without_allocating_a_server_cast_id_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_906);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);
    let mut spells = SpellStore::new();
    spells.insert(TEST_SPELL_ID, minimal_spell());
    session.set_spell_store(Arc::new(spells));

    let first = request(guid, 1);
    let second = request(guid, 2);
    assert!(crate::player_cast::request(&mut session, first));
    assert!(crate::player_cast::request(&mut session, second.clone()));
    assert_eq!(
        session.pending_spell_cast_snapshot_like_cpp(),
        Some(Some(second))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CastFailed]
    );

    assert!(session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id: TEST_SPELL_ID,
        target_guid: guid,
        target_data: Default::default(),
        cast_id: ObjectGuid::new(6, 3),
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: Default::default(),
        metadata: Default::default(),
    })));
    assert!(!crate::player_cast::request(&mut session, request(guid, 4)));
    assert_eq!(
        session.pending_spell_cast_snapshot_like_cpp(),
        Some(Some(request(guid, 2))),
        "SpellInProgress leaves the queued request untouched"
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_max_low_guid_like_cpp(HighGuid::Cast)
            .unwrap(),
        1,
        "admission does not allocate the server Cast GUID"
    );
}

#[tokio::test]
async fn canonical_prepare_allocates_map_cast_and_stamps_residence_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_912);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);

    let mut spell = minimal_spell();
    spell.cast_time_ms = 1_500;
    let mut spells = SpellStore::new();
    spells.insert(TEST_SPELL_ID, spell);
    session.set_spell_store(Arc::new(spells));
    session.set_known_spells_like_cpp(vec![TEST_SPELL_ID]);
    session.set_legacy_creature_aggro_config_like_cpp(LegacyCreatureAggroConfigLikeCpp {
        // An empty effective visual catalog still takes the production
        // resolver path and returns the C++ default visual.
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [],
        ))),
        ..Default::default()
    });

    let pending = request(guid, 55);
    assert!(crate::player_cast::request(&mut session, pending.clone()));
    session.tick_pending_spell_cast_request_like_cpp().await;

    let active = session
        .active_spell_cast_snapshot_like_cpp()
        .expect("a timed canonical request remains prepared");
    assert_eq!(active.cast_id.high_type(), HighGuid::Cast);
    assert_eq!(active.cast_id.counter(), 1);
    assert_eq!(active.metadata.client_cast_id, Some(pending.cast_id));
    assert_eq!(active.metadata.prepared_residence_revision, Some(1));
    assert!(active.metadata.from_client);
    assert_eq!(session.pending_spell_cast_snapshot_like_cpp(), Some(None));
    let prepare = send_rx.try_recv().expect("mapping precedes Start");
    assert_eq!(
        &prepare[..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    let mut mapping = wow_packet::WorldPacket::from_bytes(&prepare[2..]);
    assert_eq!(mapping.read_packed_guid().unwrap(), pending.cast_id);
    assert_eq!(mapping.read_packed_guid().unwrap(), active.cast_id);
    assert!(mapping.is_empty());
    let start = send_rx.try_recv().expect("timed Start follows mapping");
    assert_eq!(
        &start[..2],
        &(ServerOpcodes::SpellStart as u16).to_le_bytes()
    );
    let mut payload = wow_packet::WorldPacket::from_bytes(&start[2..]);
    assert_eq!(payload.read_packed_guid().unwrap(), guid);
    assert_eq!(payload.read_packed_guid().unwrap(), guid);
    assert_eq!(payload.read_packed_guid().unwrap(), active.cast_id);
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(
        send_rx.try_recv().is_err(),
        "timed cast cannot publish Go during preparation"
    );
}

#[test]
fn player_cast_publication_fences_visibility_generation_and_map_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_player(1, 58_907);
    let visible_guid = ObjectGuid::create_player(1, 58_908);
    let hidden_guid = ObjectGuid::create_player(1, 58_909);
    let other_map_guid = ObjectGuid::create_player(1, 58_910);
    let receiver_guid = ObjectGuid::create_player(1, 58_911);
    let source_position = Position::ZERO;
    let (mut source, _, source_rx) = make_session();
    install_canonical_player(
        &mut source,
        &canonical,
        source_guid,
        571,
        0,
        source_position,
    );

    add_canonical_test_player_on_map(&canonical, visible_guid, source_position, 571, 0);
    add_canonical_test_player_on_map(&canonical, hidden_guid, source_position, 571, 0);
    add_canonical_test_player_on_map(&canonical, other_map_guid, source_position, 571, 1);

    let registry = Arc::new(PlayerRegistry::new());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    source.set_player_registry(Arc::clone(&registry));

    let visible_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let hidden_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let other_map_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let mut visible_set = SharedClientVisibleGuidsLikeCpp::default();
    visible_set.insert(source_guid);
    let visible_registration = registry.register_or_replace(
        visible_guid,
        player_registration_info(
            visible_guid,
            flume::unbounded().0,
            571,
            0,
            source_position,
            visible_set.clone(),
            Arc::clone(&visible_durable),
        ),
        Default::default(),
    );
    registry.register_or_replace(
        hidden_guid,
        player_registration_info(
            hidden_guid,
            flume::unbounded().0,
            571,
            0,
            source_position,
            SharedClientVisibleGuidsLikeCpp::default(),
            Arc::clone(&hidden_durable),
        ),
        Default::default(),
    );
    registry.register_or_replace(
        other_map_guid,
        player_registration_info(
            other_map_guid,
            flume::unbounded().0,
            571,
            1,
            source_position,
            visible_set.clone(),
            Arc::clone(&other_map_durable),
        ),
        Default::default(),
    );

    let source_revision = canonical
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(source.player_handle_like_cpp.unwrap())
        .unwrap()
        .1;
    let packet = spell_start_bytes(source_guid);
    source.publish_player_cast_frame_like_cpp(
        SpellCastMetadata {
            client_cast_id: Some(ObjectGuid::new(6, 0xA1)),
            prepared_residence_revision: Some(source_revision),
            from_client: true,
            ..Default::default()
        },
        packet.clone(),
    );
    assert_eq!(source_rx.try_recv().unwrap(), packet);

    let command = match visible_durable.lock().unwrap().drain_like_cpp().as_slice() {
        [SessionCommand::SendPlayerSpellIfVisibleLikeCpp(command)] => command.clone(),
        commands => panic!("expected one visible player spell command, got {commands:?}"),
    };
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, packet);
    assert!(command.committed_visibility_like_cpp.contains(&source_guid));
    assert!(hidden_durable.lock().unwrap().drain_like_cpp().is_empty());
    assert!(
        other_map_durable
            .lock()
            .unwrap()
            .drain_like_cpp()
            .is_empty()
    );

    let replacement_durable =
        Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let replacement_registration = registry.register_or_replace(
        visible_guid,
        player_registration_info(
            visible_guid,
            flume::unbounded().0,
            571,
            0,
            source_position,
            visible_set.clone(),
            Arc::clone(&replacement_durable),
        ),
        Default::default(),
    );
    assert_ne!(visible_registration, replacement_registration);
    assert!(
        !registry.publish_current_player_spell_if_visible(visible_registration, command.clone()),
        "a reconnect cannot receive the prior incarnation's publication"
    );
    assert!(
        replacement_durable
            .lock()
            .unwrap()
            .drain_like_cpp()
            .is_empty()
    );

    let (mut receiver, _, receiver_rx) = make_session();
    install_canonical_player(
        &mut receiver,
        &canonical,
        receiver_guid,
        571,
        0,
        source_position,
    );
    receiver.set_state(SessionState::LoggedIn);
    let receiver_visibility = SharedClientVisibleGuidsLikeCpp::default();
    receiver_visibility.insert(source_guid);
    receiver.client_visible_guids_like_cpp = receiver_visibility.clone();
    receiver.handle_player_cast_publication_like_cpp(SendPlayerSpellIfVisibleLikeCppCommand {
        map_id: 571,
        instance_id: 0,
        packet_bytes: packet.clone(),
        committed_visibility_like_cpp: receiver_visibility.clone(),
    });
    assert_eq!(receiver_rx.try_recv().unwrap(), packet);

    receiver.handle_player_cast_publication_like_cpp(SendPlayerSpellIfVisibleLikeCppCommand {
        map_id: 571,
        instance_id: 1,
        packet_bytes: spell_start_bytes(source_guid),
        committed_visibility_like_cpp: receiver_visibility.clone(),
    });
    let unrelated_visibility = SharedClientVisibleGuidsLikeCpp::default();
    unrelated_visibility.insert(source_guid);
    receiver.handle_player_cast_publication_like_cpp(SendPlayerSpellIfVisibleLikeCppCommand {
        map_id: 571,
        instance_id: 0,
        packet_bytes: spell_start_bytes(source_guid),
        committed_visibility_like_cpp: unrelated_visibility,
    });
    assert!(receiver_rx.try_recv().is_err());
}

#[tokio::test]
async fn prepared_late_power_failure_keeps_gcd_and_orders_interruption_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_913);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);
    let handle = session.player_handle_like_cpp.unwrap();
    let revision = canonical
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle)
        .unwrap()
        .1;
    let mut spell = minimal_spell();
    spell.power_costs.push(wow_data::SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: PowerType::Mana as i8,
        mana_cost: 50,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });
    let mut spells = SpellStore::new();
    spells.insert(TEST_SPELL_ID, spell);
    session.set_spell_store(Arc::new(spells));
    let mut cast = prepared_cast(guid, revision);
    session.mutate_canonical_player_like_cpp(|player| {
        player.set_power_index(PowerType::Mana, Some(0));
        player.unit_mut().set_max_power(PowerType::Mana, 100);
        player.unit_mut().set_power(PowerType::Mana, 49);
    });
    cast.metadata.client_started_global_cooldown = true;
    let gcd_started = cast.cast_start_time;
    assert!(session.set_active_spell_cast_like_cpp(Some(cast)));
    session.mutate_cast_execution_like_cpp(|state| state.last_cast_time = Some(gcd_started));
    session.tick_active_spell_cast().await;
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert_eq!(
        session.last_spell_cast_time_like_cpp().flatten(),
        Some(gcd_started)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CastFailed,
            ServerOpcodes::SpellFailure,
            ServerOpcodes::SpellFailedOther,
        ]
    );
    session.tick_active_spell_cast().await;
    assert!(
        send_rx.try_recv().is_err(),
        "failed cast is consumed exactly once"
    );
}

#[test]
fn prepared_cancel_orders_interruption_before_result_and_clears_gcd_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 58_914);
    install_canonical_player(&mut session, &canonical, guid, 571, 0, Position::ZERO);
    let handle = session.player_handle_like_cpp.unwrap();
    let revision = canonical
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle)
        .unwrap()
        .1;
    let mut cast = prepared_cast(guid, revision);
    cast.cast_time_ms = 1_500;
    cast.metadata.client_started_global_cooldown = true;
    let gcd_started = cast.cast_start_time;
    assert!(session.set_active_spell_cast_like_cpp(Some(cast)));
    session.mutate_cast_execution_like_cpp(|state| state.last_cast_time = Some(gcd_started));
    session.cancel_client_cast_request_like_cpp(Some(TEST_SPELL_ID));
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert_eq!(session.last_spell_cast_time_like_cpp().flatten(), None);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellFailure,
            ServerOpcodes::SpellFailedOther,
            ServerOpcodes::CastFailed,
        ]
    );
    session.cancel_client_cast_request_like_cpp(Some(TEST_SPELL_ID));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn normal_visual_uses_server_relation_id_and_cpp_equal_condition_order() {
    use crate::player_cast::Runtime;
    let (mut session, _, _) = make_session();
    let row = |id| wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id: 0,
        spell_visual_id: 900_000 + id,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id: TEST_SPELL_ID as u32,
    };
    session.set_legacy_creature_aggro_config_like_cpp(LegacyCreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [row(10), row(20)],
        ))),
        ..Default::default()
    });
    let visual = Runtime::visual(&session, &minimal_spell()).unwrap();
    assert_eq!(
        visual.spell_visual_id, 20,
        "SpellMgr lower_bound reverses equal-condition relation IDs"
    );
    assert_eq!(visual.script_visual_id, 0);
}
