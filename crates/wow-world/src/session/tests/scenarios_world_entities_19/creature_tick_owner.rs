//! Session and legacy owner coverage for creature ticks.

use super::*;

#[test]
fn default_session_owner_preserves_creatures_tick_packets() {
    // With the default Session owner, run_creatures_tick must produce the
    // same bytes that tick_creatures_sync previously sent directly.
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    assert_eq!(
        manager.read().unwrap().tick_owner(),
        RuntimeTickOwner::Session
    );

    // Session A: call run_creatures_tick and collect output.
    let (mut session_a, _, recv_a) = make_session();
    session_a.set_mmap_runtime_config_like_cpp(MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    });
    let guid = test_creature_guid(90_001);
    register_test_creature(&mut session_a, manager.clone(), guid, 25);
    session_a.client_visible_guids_like_cpp.insert(guid);
    session_a
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x9001);
        })
        .unwrap();

    let output = (0..32)
        .find_map(|_| {
            let output = session_a.run_creatures_tick();
            // Channel must be empty — no direct send happened.
            assert!(
                recv_a.try_recv().is_err(),
                "run_creatures_tick must not send directly to the channel"
            );
            (!output.packets.is_empty()).then_some(output)
        })
        .expect("session-owned random movement should eventually emit MonsterMove");
    // Flush and verify at least one packet arrived (MonsterMove).
    session_a.flush_runtime_output(output);
    let pkt = recv_a
        .try_recv()
        .expect("flush must deliver the MonsterMove packet");
    let opcode = u16::from_le_bytes([pkt[0], pkt[1]]);
    assert_eq!(opcode, ServerOpcodes::OnMonsterMove as u16);
}
#[test]
fn session_creature_tick_suppresses_monster_move_for_non_visible_creature_like_cpp() {
    // C++ MoveSplineInit::Launch uses Unit::SendMessageToSet, which only
    // reaches players that can see the moving unit. The legacy per-session
    // tick must not send OnMonsterMove for map creatures absent from this
    // session's HaveAtClient set.
    let manager = shared_map_manager();
    let (mut session, _, recv) = make_session();
    let guid = test_creature_guid(90_011);
    register_test_creature(&mut session, manager.clone(), guid, 25);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x9011);
        })
        .unwrap();

    let output = session.run_creatures_tick();
    assert!(
        output.packets.is_empty(),
        "non-visible creature movement must not be delivered to this session"
    );
    session.flush_runtime_output(output);
    assert!(
        recv.try_recv().is_err(),
        "non-visible creature movement must not reach the send channel"
    );
}
#[test]
fn global_legacy_owner_skips_creature_tick_but_keeps_player_combat_tick_like_cpp() {
    // With GlobalLegacy, only the session creature tick is suppressed.
    // Player auto-attack is a Player::Update responsibility in C++
    // (DoMeleeAttackIfReady) and must keep running until Slice 6 moves
    // combat ownership explicitly.
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let (mut session, _, recv) = make_session();
    let guid = test_creature_guid(90_003);
    let player = ObjectGuid::create_player(1, 90_003);
    session.player_guid = Some(player);
    session.combat_target = Some(guid);
    session.in_combat = true;
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    // Drive creature_tick to a value where both %4 and %2 would fire.
    session.creature_tick = 3; // next wrapping_add → 4, divisible by both 4 and 2.
    session.state = crate::session::SessionState::LoggedIn;

    // Simulate the guard logic in update() for the tick path only.
    let owner = session.runtime_tick_owner_like_cpp();
    session.creature_tick = session.creature_tick.wrapping_add(1);
    if session.creature_tick % 4 == 0 && owner == RuntimeTickOwner::Session {
        session.tick_creatures_sync();
    }
    if session.creature_tick % 2 == 0 {
        session.tick_combat_sync();
    }

    let attacker_state = recv
        .try_recv()
        .expect("GlobalLegacy must not suppress player combat packets");
    let opcode = u16::from_le_bytes([attacker_state[0], attacker_state[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .expect("creature must exist")
        .current_hp();
    assert!(
        hp < 40,
        "player combat must still damage the target under GlobalLegacy"
    );
    // creature_tick was still incremented (guard only wraps the tick calls).
    assert_eq!(session.creature_tick, 4);
}
#[tokio::test]
async fn update_global_legacy_owner_skips_real_session_creature_tick_path() {
    // This drives `WorldSession::update` itself, not a hand-copied subset of
    // the guard.  With `GlobalLegacy`, the session must not move the shared
    // creature even when the tick cadence would normally fire.
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let (mut session, _pkt_tx, recv) = make_session();
    let guid = test_creature_guid(90_006);
    register_test_creature(&mut session, manager.clone(), guid, 25);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x5757);
        })
        .unwrap();

    let before = {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, guid).expect("creature exists");
        assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
        creature.position()
    };

    session.state = crate::session::SessionState::LoggedIn;
    session.creature_tick = 3; // update() increments to 4, so creature tick would fire.
    session.time_synchronization.timer_ms = 0;

    assert_eq!(session.update(50).await, 0);

    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, guid).expect("creature exists");
    assert_eq!(
        creature.position(),
        before,
        "GlobalLegacy owner must suppress movement from the session update path"
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::Idle,
        "session update must not launch a wander spline under GlobalLegacy"
    );
    assert!(
        recv.try_recv().is_err(),
        "session update must not send creature tick packets under GlobalLegacy"
    );
}
