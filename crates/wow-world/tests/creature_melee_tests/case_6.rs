use super::*;

#[tokio::test]
async fn apply_creature_melee_damage_command_battleground_skips_durability_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1017);
    let victim_guid = ObjectGuid::create_player(1, 7011);
    session.fixture_melee_set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(victim_guid));
    session.fixture_melee_set_map_position(571, Position::ZERO);
    session.fixture_melee_set_health(100, 100);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        0,
        wow_constants::DeathState::JustDied,
    );
    assert!(
        session.fixture_melee_adopt_registered_player(),
        "battleground fixture adopts the map-owned Player handle"
    );
    session
        .fixture_melee_mutate_player(|player| {
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

    session.fixture_melee_process_commands().await;

    assert!(
        !drain_server_opcodes(&send_rx).contains(&ServerOpcodes::DurabilityDamageDeath),
        "C++ Unit::Kill skips the creature-killer durability branch inside a battleground"
    );
}
