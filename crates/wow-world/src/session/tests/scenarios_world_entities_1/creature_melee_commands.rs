use super::*;

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
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 17,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: vec![91],
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
    let packet = send_rx.try_recv().expect("self-share health update");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    let packet = send_rx.try_recv().expect("primary health update");
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
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 17,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
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
        hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
        victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
        original_damage: 17,
        absorbed: 0,
        mana_spent: 0,
        absorb_consumptions: Vec::new(),
        split_combat_log_packets: Vec::new(),
        self_share_health_updates: Vec::new(),
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
        hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
        victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
        original_damage: 100,
        absorbed: 0,
        mana_spent: 0,
        absorb_consumptions: Vec::new(),
        split_combat_log_packets: Vec::new(),
        self_share_health_updates: Vec::new(),
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
    assert_eq!(
        send_rx.drain().count(),
        3,
        "AttackerStateUpdate + HealthUpdate + the C++ Unit::Kill durability loss message"
    );

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
async fn apply_creature_melee_damage_command_lethal_publishes_durability_loss_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1016);
    let victim_guid = ObjectGuid::create_player(1, 7010);
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
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 100,
                over_damage: 0,
                target_level: 80,
                victim_health_after: 0,
                victim_health_state_revision_after: committed_revision,
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 100,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
            },
        ))
        .expect("lethal command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::DurabilityDamageDeath),
        "C++ Unit::Kill applies the creature-killer PvE durability loss on a lethal hit"
    );
}

#[tokio::test]
async fn apply_creature_melee_damage_command_battleground_skips_durability_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1017);
    let victim_guid = ObjectGuid::create_player(1, 7011);
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
        session.adopt_registered_canonical_player_fixture_like_cpp(),
        "battleground fixture adopts the map-owned Player handle"
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_battleground_type_id_like_cpp(1);
        })
        .expect("canonical player carries the battleground state");
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 100,
                over_damage: 0,
                target_level: 80,
                victim_health_after: 0,
                victim_health_state_revision_after: committed_revision,
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 100,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
            },
        ))
        .expect("lethal battleground command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        !drain_server_opcodes(&send_rx).contains(&ServerOpcodes::DurabilityDamageDeath),
        "C++ Unit::Kill skips the creature-killer durability branch inside a battleground"
    );
}
