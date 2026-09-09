//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn chr_classes_store_drives_creature_display_power_else_fallback_like_cpp() {
    use wow_data::character_progression::{ChrClassesEntry, ChrClassesStore};
    let (mut session, _, _) = make_session();

    // [M0.1/#14] Without the ChrClasses store, class display power uses the
    // hardcoded fallback (class 8 = Mage → Mana).
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(8),
        PowerType::Mana as u8
    );

    // With the store loaded, the DB2 `display_power` wins over the fallback.
    // Give class 8 a non-default power (Rage) to prove the store is consulted.
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([ChrClassesEntry {
        id: 8,
        display_power: PowerType::Rage as u8,
        ..Default::default()
    }])));
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(8),
        PowerType::Rage as u8
    );
    // A class with no DB2 row still falls back (class 4 = Rogue → Energy).
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(4),
        PowerType::Energy as u8
    );
}
#[test]
fn spell_threat_entry_prefers_exact_spell_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = HashMap::new();
    entries.insert(11, test_spell_threat_entry_like_cpp(11));
    entries.insert(42, test_spell_threat_entry_like_cpp(42));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: entries,
    }));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 42,
                supercedes_spell_id: 11,
            }],
            |_| true,
        ),
    ));

    let entry = session
        .spell_threat_entry_like_cpp(42)
        .expect("exact threat entry");

    assert_eq!(entry.flat_mod, 42);
}
#[test]
fn spell_threat_entry_falls_back_to_first_rank_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = HashMap::new();
    entries.insert(11, test_spell_threat_entry_like_cpp(11));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: entries,
    }));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 42,
                supercedes_spell_id: 11,
            }],
            |_| true,
        ),
    ));

    let entry = session
        .spell_threat_entry_like_cpp(42)
        .expect("first-rank threat entry");

    assert_eq!(entry.flat_mod, 11);
}
#[test]
fn creature_realm_fanout_uses_validated_position_without_canonical_mirror() {
    let (mut source, _, _) = make_session();
    let source_player = ObjectGuid::create_player(1, 42);
    let observer = ObjectGuid::create_player(1, 43);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1014);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let packet_bytes = vec![0x44, 0x55, 0x68];
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (observer_send_tx, _observer_send_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(1);
    let mut observer_info =
        broadcast_info_with_command(observer, observer_send_tx, observer_command_tx);
    observer_info.placement.map_id = 571;
    observer_info.placement.instance_id = 0;
    observer_info.placement.position = source_position;
    registry.register_or_replace(observer, observer_info, Default::default());

    source.set_player_guid(Some(source_player));
    source.set_player_map_position_like_cpp(571, Position::ZERO);
    source.set_player_registry(registry);
    source.broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
        creature,
        source_position,
        packet_bytes.clone(),
    );

    let command = observer_command_rx.try_recv().expect("observer fanout");
    let SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(command) = command else {
        panic!("creature visual must use the Realm-visible command");
    };
    assert_eq!(command.source_guid, creature);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, packet_bytes);
}
#[tokio::test]
async fn send_if_visible_creature_command_rechecks_current_phase_and_range_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1011);
    let packet_bytes = vec![0xD4, 0x2D, 0xAA];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::new(5000.0, 5000.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    session.state = SessionState::LoggedIn;
    session.set_map_manager(Arc::clone(&manager));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects stale HaveAtClient entries outside source range"
    );

    session.mutate_world_creature(source_guid, |creature| {
        creature
            .creature
            .set_ai_position(Position::new(10.0, 0.0, 0.0, 0.0));
        *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
    });
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects stale HaveAtClient entries outside source phase"
    );

    session.mutate_world_creature(source_guid, |creature| {
        *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    });
    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        send_rx.try_recv().expect("same phase and range delivers"),
        packet_bytes
    );
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
/// Future global creature aggro must mirror one map-owned AttackStart into
/// the victim session. C++ anchor: `CreatureAI::MoveInLineOfSight` ->
/// `Creature::CanStartAttack` -> `Unit::SendMeleeAttackStart`.
#[tokio::test]
async fn creature_attack_start_command_sets_combat_and_sends_packet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1006);
    let victim_guid = ObjectGuid::create_player(1, 7000);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(attacker_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.combat_target, None,
        "incoming attacks must not select the attacker as the player's target"
    );
    assert!(session.in_combat);
    let packet = send_rx.try_recv().expect("attack start packet");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackStart as u16);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
#[tokio::test]
async fn creature_attack_start_command_syncs_combat_when_attacker_is_no_longer_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1009);
    let victim_guid = ObjectGuid::create_player(1, 7003);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.combat_target, None,
        "incoming attacks must not select the attacker as the player's target"
    );
    assert!(session.in_combat);
    assert!(
        send_rx.try_recv().is_err(),
        "an attacker no longer visible to the client must not emit AttackStart"
    );
}
#[tokio::test]
async fn creature_attack_start_command_rejects_dead_victim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1010);
    let victim_guid = ObjectGuid::create_player(1, 7004);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(attacker_guid);
    session.set_player_alive_like_cpp(false);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
    assert!(
        send_rx.try_recv().is_err(),
        "dead victim must not receive attack-start"
    );
}
/// Future global creature melee must deliver exactly one already-resolved
/// map-owned swing to the victim session. C++ anchor:
/// `Creature::Update` -> `DoMeleeAttackIfReady` -> `AttackerStateUpdate`.
#[tokio::test]
async fn apply_creature_melee_damage_command_updates_victim_and_sends_hit_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1007);
    let victim_guid = ObjectGuid::create_player(1, 7001);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(100, 100);
    session.client_visible_guids_like_cpp.insert(attacker_guid);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        83,
        wow_constants::DeathState::Alive,
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 17,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 83,
                victim_health_state_revision_after: committed_revision,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.player_health_like_cpp(), 83);
    let packet = send_rx.try_recv().expect("attacker state update");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
    let packet = send_rx.try_recv().expect("health update");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
#[tokio::test]
async fn apply_creature_melee_damage_command_syncs_health_without_visible_attacker_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1008);
    let victim_guid = ObjectGuid::create_player(1, 7002);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(100, 100);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        83,
        wow_constants::DeathState::Alive,
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 17,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 83,
                victim_health_state_revision_after: committed_revision,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.player_health_like_cpp(), 83);
    let packet = send_rx
        .try_recv()
        .expect("authoritative health update is independent of attacker visibility");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    assert!(send_rx.try_recv().is_err(), "no attacker-state packet");
}
#[tokio::test]
async fn apply_creature_melee_damage_command_delayed_after_heal_presents_current_canonical_state_like_cpp()
 {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1014);
    let victim_guid = ObjectGuid::create_player(1, 7008);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(83, 100);
    session.client_visible_guids_like_cpp.insert(attacker_guid);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        83,
        wow_constants::DeathState::Alive,
    );
    let command = ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid,
        victim_guid,
        map_id: 571,
        instance_id: 0,
        damage: 17,
        over_damage: -1,
        target_level: 80,
        victim_health_after: 83,
        victim_health_state_revision_after: committed_revision,
    };

    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_health(95))
        .expect("canonical player remains present for delayed delivery");
    session.set_player_health_like_cpp(95, 100);
    let canonical_before = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
        .expect("delayed command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let canonical_after = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(canonical_after, canonical_before);
    assert_eq!(session.player_health_like_cpp(), 95);
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        session.last_presented_creature_melee_health_state_revision_like_cpp,
        committed_revision
    );

    let attacker_state = send_rx.try_recv().expect("committed hit is presented once");
    assert_eq!(
        u16::from_le_bytes([attacker_state[0], attacker_state[1]]),
        ServerOpcodes::AttackerStateUpdate as u16
    );
    let health_update = send_rx.try_recv().expect("current health is presented");
    let mut health_update = wow_packet::world_packet::WorldPacket::from_bytes(&health_update);
    assert_eq!(
        health_update.read_uint16().unwrap(),
        ServerOpcodes::HealthUpdate as u16
    );
    assert_eq!(health_update.read_packed_guid().unwrap(), victim_guid);
    assert_eq!(health_update.read_int64().unwrap(), 95);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn apply_creature_melee_damage_command_replay_after_resurrection_is_suppressed_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1015);
    let victim_guid = ObjectGuid::create_player(1, 7009);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(100, 100);
    session.client_visible_guids_like_cpp.insert(attacker_guid);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        0,
        wow_constants::DeathState::JustDied,
    );
    let command = ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid,
        victim_guid,
        map_id: 571,
        instance_id: 0,
        damage: 100,
        over_damage: 0,
        target_level: 80,
        victim_health_after: 0,
        victim_health_state_revision_after: committed_revision,
    };
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            command.clone(),
        ))
        .expect("lethal command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(send_rx.drain().count(), 2);

    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(40);
        })
        .expect("canonical player resurrected");
    session.set_player_health_like_cpp(40, 100);
    let canonical_before = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    let presented_before = session.last_presented_creature_melee_health_state_revision_like_cpp;
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
        .expect("replayed command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let canonical_after = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(canonical_after, canonical_before);
    assert_eq!(session.player_health_like_cpp(), 40);
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        session.last_presented_creature_melee_health_state_revision_like_cpp,
        presented_before
    );
    assert!(send_rx.try_recv().is_err(), "replay emits no packets");
}
#[tokio::test]
async fn durable_creature_runtime_rail_is_drained_by_session_update_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1011);
    let victim_guid = ObjectGuid::create_player(1, 7005);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(100, 100);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        0,
        wow_constants::DeathState::JustDied,
    );
    assert!(
        session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap()
            .publish_melee_damage_like_cpp(ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 100,
                over_damage: 0,
                target_level: 80,
                victim_health_after: 0,
                victim_health_state_revision_after: committed_revision,
            })
    );

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(!session.player_is_alive_like_cpp());
    let packet = send_rx.recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::HealthUpdate as u16
    );
}
#[tokio::test]
async fn durable_creature_runtime_overflow_disconnects_desynchronized_session() {
    let (mut session, _, _) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1012);
    let victim_guid = ObjectGuid::create_player(1, 7006);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    let command = CreatureAttackStartLikeCppCommand {
        attacker_guid,
        victim_guid,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    {
        let mut pending = session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap();
        for _ in 0..crate::session::mailbox::MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            assert!(pending.publish_attack_start_like_cpp(command.clone()));
        }
        assert!(!pending.publish_attack_start_like_cpp(command));
    }

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(session.is_disconnecting());
    assert!(
        session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap()
            .drain_like_cpp()
            .is_empty()
    );
}
/// Future global creature CREATE/DESTROY work must not use
/// `SendIfVisibleLikeCpp`: a not-yet-visible creature needs the session's
/// visibility pass to build CREATE bytes and update HaveAtClient.
#[tokio::test]
async fn refresh_visible_world_creatures_command_forces_creature_visibility_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_009);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(90_010);
    let creature_position = Position::new(12.0, 10.0, 0.0, 0.0);
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            901,
            creature_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    session.state = SessionState::LoggedIn;
    session.set_map_manager(manager);
    session.set_canonical_map_manager(canonical);
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RefreshVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    // Prove the command bypasses the 50-yard visibility throttle.
    session.last_visibility_pos = Some(player_position);

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 571,
                instance_id: 0,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&creature_guid),
        "forced creature visibility must create the unseen creature"
    );
    let packet = send_rx
        .try_recv()
        .expect("creature CREATE visibility packet");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
#[tokio::test]
async fn refresh_visible_world_creatures_command_rejects_wrong_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.last_visibility_pos = Some(Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 530,
                instance_id: 0,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.last_visibility_pos,
        Some(Position::ZERO),
        "wrong-map command must not force visibility"
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn refresh_visible_gameobjects_or_spellclicks_command_sends_gameobject_delta_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_126;
    let gameobject_guid = test_gameobject_guid(go_entry, 135);
    let quest_id = 12_543;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 2,
        order: 0,
        storage_index: 0,
        object_id: go_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_gameobject(
        &canonical,
        gameobject_guid,
        go_entry,
        Position::new(12.0, 0.0, 0.0, 0.0),
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_gameobject_use_states.insert(
        gameobject_guid,
        RepresentedGameObjectUseState {
            go_type: Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: Some(wow_entities::LootState::Ready),
            ..Default::default()
        },
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        send_rx.try_recv().expect("remote GO refresh update"),
        expected_gameobject_dynamic_flags_update_like_cpp(
            gameobject_guid,
            571,
            wow_entities::GO_DYNFLAG_LO_ACTIVATE
                | wow_entities::GO_DYNFLAG_LO_SPARKLE
                | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT
        )
    );
    assert!(send_rx.try_recv().is_err());
}
