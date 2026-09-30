use super::*;

#[tokio::test]
async fn apply_creature_melee_damage_command_replay_after_resurrection_is_suppressed_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1015);
    let victim_guid = ObjectGuid::create_player(1, 7009);
    session.fixture_melee_set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(victim_guid));
    session.fixture_melee_set_map_position(571, Position::ZERO);
    session.fixture_melee_set_health(100, 100);
    session.fixture_melee_make_visible(attacker_guid);
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
        .fixture_melee_process_commands()
        .await;
    assert_eq!(
        send_rx.drain().count(),
        3,
        "AttackerStateUpdate + HealthUpdate + the C++ Unit::Kill durability loss message"
    );

    session
        .fixture_melee_mutate_player(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(40);
        })
        .expect("canonical player resurrected");
    session.fixture_melee_set_health(40, 100);
    let canonical_before = session
        .fixture_melee_mutate_player(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    let presented_before = session
        .view
        .last_presented_creature_melee_health_state_revision_like_cpp;
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
        .expect("replayed command queued");

    session
        .fixture_melee_process_commands()
        .await;

    let canonical_after = session
        .fixture_melee_mutate_player(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(canonical_after, canonical_before);
    assert_eq!(session.fixture_melee_health(), 40);
    assert!(session.fixture_melee_is_alive());
    assert_eq!(
        session
            .view
            .last_presented_creature_melee_health_state_revision_like_cpp,
        presented_before
    );
    assert!(send_rx.try_recv().is_err(), "replay emits no packets");
}
