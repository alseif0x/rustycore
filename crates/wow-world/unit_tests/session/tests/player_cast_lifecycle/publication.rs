//! Recipient and combat-log format checks for Player spell publication.

use super::*;

#[test]
fn player_damage_log_reaches_only_visible_observers_in_their_combat_log_format() {
    verify_player_spell_publication_with_logging(true, false, true);
}

#[test]
fn player_damage_log_uses_each_caster_and_observer_combat_log_format() {
    verify_player_spell_publication_with_logging(true, true, false);
}

pub(super) fn verify_player_spell_publication(damage_log: bool) {
    verify_player_spell_publication_with_logging(damage_log, false, true);
}

fn verify_player_spell_publication_with_logging(
    damage_log: bool,
    source_advanced_combat_logging: bool,
    observer_advanced_combat_logging: bool,
) {
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

    attach_publication_test_player(&canonical, visible_guid, source_position, 571, 0);
    attach_publication_test_player(&canonical, hidden_guid, source_position, 571, 0);
    attach_publication_test_player(&canonical, other_map_guid, source_position, 571, 1);

    let registry = Arc::new(PlayerRegistry::new());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    source.set_player_registry(Arc::clone(&registry));

    let visible_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let hidden_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let other_map_durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    let visible_set = SharedClientVisibleGuidsLikeCpp::default();
    visible_set.insert(source_guid);
    let visible_info = player_registration_info(
        visible_guid,
        flume::unbounded().0,
        571,
        0,
        source_position,
        visible_set.clone(),
        Arc::clone(&visible_durable),
    );
    visible_info.advanced_combat_logging_enabled_like_cpp.store(
        observer_advanced_combat_logging,
        std::sync::atomic::Ordering::Relaxed,
    );
    let visible_registration =
        registry.register_or_replace(visible_guid, visible_info, Default::default());
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
        .player_active_residence_revision_like_cpp(source.core.player_handle_like_cpp.unwrap())
        .unwrap()
        .1;
    source
        .core
        .flags
        .advanced_combat_logging_enabled_like_cpp
        .store(
            source_advanced_combat_logging,
            std::sync::atomic::Ordering::Relaxed,
        );
    let (packet, observer_packet) = if damage_log {
        use wow_world_spell::SpellbookHostLikeCpp;
        let log = wow_packet::packets::combat::SpellNonMeleeDamageLog {
            target: ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 191),
            caster: source_guid,
            cast_id: ObjectGuid::new(6, 0xA1),
            spell_id: TEST_SPELL_ID,
            visual_id: 0,
            damage: 13,
            original_damage: 13,
            overkill: -1,
            school_mask: 4,
            absorbed: 0,
            resisted: 0,
            shield_block: 0,
            periodic: false,
            flags: 0,
        };
        let basic_packet = log.to_bytes();
        let full_packet = log.to_full_log_bytes_like_cpp();
        let source_packet = if source_advanced_combat_logging {
            full_packet.clone()
        } else {
            basic_packet.clone()
        };
        let observer_packet = if observer_advanced_combat_logging {
            full_packet
        } else {
            basic_packet
        };
        let (spell, mut hub) = source.split_spell_state_mut();
        spell.publish_player_damage_log_like_cpp(&mut hub, &log);
        (source_packet, observer_packet)
    } else {
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
        (packet.clone(), packet)
    };
    assert_eq!(source_rx.try_recv().unwrap(), packet);

    let command = match visible_durable.lock().unwrap().drain_like_cpp().as_slice() {
        [SessionCommand::SendPlayerSpellIfVisibleLikeCpp(command)] => command.clone(),
        commands => panic!("expected one visible player spell command, got {commands:?}"),
    };
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, observer_packet);
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
    receiver.core.client_visible_guids_like_cpp = receiver_visibility.clone();
    receiver.handle_player_cast_publication_like_cpp(SendPlayerSpellIfVisibleLikeCppCommand {
        map_id: 571,
        instance_id: 0,
        packet_bytes: observer_packet.clone(),
        committed_visibility_like_cpp: receiver_visibility.clone(),
    });
    assert_eq!(receiver_rx.try_recv().unwrap(), observer_packet);

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

fn attach_publication_test_player(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = Box::new(wow_entities::Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);

    let mut canonical = canonical.lock().unwrap();
    canonical.create_map_entry(map_id, instance_id, 0, wow_map::ManagedMapKind::World);
    let handle = canonical
        .install_detached_player_like_cpp(player)
        .expect("publication observer gets a canonical detached Player");
    canonical
        .attach_player_like_cpp(handle, wow_map::MapKey::new(map_id, instance_id), position)
        .expect("publication observer attaches through the canonical map owner");
}
