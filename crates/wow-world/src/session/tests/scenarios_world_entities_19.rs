//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_goober_just_deactivated_consumable_stays_not_ready_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 16);
    let linked_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .loot_state = Some(wow_entities::LootState::JustDeactivated);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .linked_trap_guid = Some(linked_trap_guid);
    let linked_state = session
        .represented_gameobject_use_states
        .entry(linked_trap_guid)
        .or_default();
    linked_state.map_id = Some(571);
    linked_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
    linked_state.loot_state = Some(wow_entities::LootState::Ready);
    session
        .client_visible_guids_like_cpp
        .insert(linked_trap_guid);

    assert!(
        session.apply_represented_gameobject_goober_just_deactivated_like_cpp(
            gameobject_guid,
            wow_entities::GooberUseSource {
                consumable: true,
                linked_trap_entry: 999,
                ..Default::default()
            },
        )
    );

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::NotReady));
    assert_eq!(state.go_state, None);
    let linked_state = session
        .represented_gameobject_use_states
        .get(&linked_trap_guid)
        .unwrap();
    assert_eq!(
        linked_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&linked_trap_guid)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GooberLinkedTrapDespawn {
                gameobject_guid,
                trap_entry: 999,
            },
            RepresentedGameObjectUseEffect::GooberCleared {
                gameobject_guid,
                loot_state: wow_entities::LootState::NotReady,
                go_state: None,
            },
        ]
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::GameObjectDespawn)
            .count(),
        2
    );
    assert_eq!(opcodes.len(), 3);
    let linked_command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => {
            panic!("expected represented linked trap despawn fanout command, got {other:?}")
        }
    };
    assert_eq!(linked_command.source_guid, linked_trap_guid);
    assert_eq!(linked_command.map_id, 571);
    assert_eq!(linked_command.instance_id, 0);
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => panic!("expected represented goober despawn fanout command, got {other:?}"),
    };
    let mut expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(command.source_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, expected);
}
#[tokio::test]
async fn gameobject_goober_just_deactivated_non_consumable_anim_progress_sends_no_despawn_like_cpp()
{
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_GOOBER as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.go_anim_progress = 1;
    state.goober_use_source = Some(wow_entities::GooberUseSource {
        consumable: false,
        spell_id: 7777,
        ..Default::default()
    });
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
// Anchor: legacy motor A (WorldSession) shares a single Arc<RwLock<MapManager>>
// across sessions — mutations from one session are immediately visible to the
// other without any message-passing. This characterizes the shared-state
// contract that tick_creatures_sync / tick_combat_sync depend on.
#[tokio::test]
async fn two_sessions_sharing_legacy_map_manager_see_same_creature_state() {
    let manager = shared_map_manager();

    let (mut session1, _pkt_tx1, _send_rx1) = make_session();
    let (mut session2, _pkt_tx2, _send_rx2) = make_session();

    session1.set_map_manager(Arc::clone(&manager));
    session2.set_map_manager(Arc::clone(&manager));

    let creature_guid = test_creature_guid(99901);
    let pos = Position::new(10.0, 10.0, 0.0, 0.0);
    let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(pos.x, pos.y);
    manager.write().unwrap().add_creature(
        0,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            900,
            pos,
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

    let hp_before = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .expect("creature must exist before mutation")
        .current_hp();
    assert_eq!(hp_before, 100, "initial HP must be 100");

    session1.mutate_world_creature(creature_guid, |c| {
        c.take_damage(40);
    });

    let hp_after = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .expect("creature must exist after mutation")
        .current_hp();
    assert_eq!(
        hp_after, 60,
        "session2 must observe HP=60 after session1 applied 40 damage"
    );

    let guids2 = session2.world_creature_guids();
    assert!(
        guids2.contains(&creature_guid),
        "session2 must see creature inserted via shared manager"
    );
}
/// Two players attacking one creature resolve it once, not twice.
///
/// This is #28's acceptance criterion, and nothing in the tree asserted it: the
/// existing "two session" tests either hand-copy the owner guard or drive a
/// single session. Under `GlobalLegacy` the map owns the transition, so one
/// pass of the phase applies each attacker's swing exactly once against shared
/// creature state — and running the phase twice with no time elapsed must not
/// apply anything again, because the swing timers were consumed.
#[tokio::test]
async fn two_players_attacking_one_creature_resolve_once_under_the_map_owner_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_930);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    // Two attackers, each a real canonical player swinging at the same creature.
    let registry = PlayerRegistry::new();
    let mut attackers = Vec::new();
    for (index, counter) in [(0usize, 5_101i64), (1usize, 5_102i64)] {
        let player_guid = ObjectGuid::create_player(1, counter);
        let (mut session, _pkt_tx, _send_rx) = make_session();
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::clone(&map_store));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            format!("Attacker{index}"),
            Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            1,
            1,
            80,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                unit.set_attacking(Some(creature_guid));
                unit.set_target(creature_guid);
                unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
                unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
                unit.set_weapon_damage(WeaponAttackType::BaseAttack, 5.0, 5.0);
                unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
            })
            .unwrap();
        // Only the first session registers the creature; both share the manager.
        if index == 0 {
            session.set_map_manager(Arc::clone(&manager));
            register_test_creature(&mut session, manager.clone(), creature_guid, 100);
        }
        // `PlayerRegistration` is opaque by design (#150): the only way to get one
        // is to register, which is also what production does.
        let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
        let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
        let registration = registry.register_or_replace(
            player_guid,
            broadcast_info_with_command(player_guid, send_tx, command_tx),
            Default::default(),
        );
        attackers.push(crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
            registration,
            player_guid,
            map_id: 0,
            instance_id: 0,
            in_combat_mirror: true,
            tap_group_guids: Vec::new(),
        });
    }

    let health_before = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .current_hp();

    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let first = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
    );

    assert!(!first.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(first.attackers_seen, 2);
    assert_eq!(
        first.victims_resolved, 2,
        "both attackers resolve the victim"
    );
    assert_eq!(
        first.creature_hits, 2,
        "each attacker lands its own swing, once"
    );

    let health_after_first = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .current_hp();
    assert!(
        health_after_first < health_before,
        "the shared creature took damage"
    );

    // A second pass with no time elapsed must apply nothing: the swing timers
    // were consumed by the first. This is the double-tick guard.
    let second = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        0,
        &mut phase_state,
    );
    assert_eq!(
        second.creature_hits, 0,
        "a second pass with no elapsed time must not resolve the same swing again"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp(),
        health_after_first,
        "and the creature's health must not move"
    );
}
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
    session.time_sync_timer_ms = 0;

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
#[test]
fn legacy_creature_movement_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_007);
    register_test_creature(&mut session, manager.clone(), guid, 25);

    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        None,
        &MMapRuntimeConfigLikeCpp::default(),
        None,
        &HashMap::new(),
        10,
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
    assert_eq!(outcome.movement_packets, 0);
    assert!(outcome.plan.events.is_empty());
    let guard = manager.read().unwrap();
    assert_eq!(
        guard
            .find_creature(0, 0, guid)
            .expect("creature remains present")
            .state(),
        wow_entities::CreatureAiState::Idle
    );
}
#[test]
fn legacy_creature_movement_tick_once_moves_once_syncs_canonical_and_plans_fanout_like_cpp() {
    use crate::map_manager::{RecipientRule, RuntimeTickOwner, VISIBILITY_RADIUS};
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_008);
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
            creature.seed_runtime_rng_like_cpp(0x9008);
            creature.backdate_runtime_clock_for_test(Duration::from_millis(10));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.canonical_syncs, 1);
    assert_eq!(outcome.plan.events.len(), 1);
    let event = &outcome.plan.events[0];
    assert_eq!(event.source_guid, guid);
    match &event.recipients {
        RecipientRule::NearbyVisible {
            source_guid,
            map_id,
            instance_id,
            range,
            required_3d,
            ..
        } => {
            assert_eq!(*source_guid, guid);
            assert_eq!(*map_id, 0);
            assert_eq!(*instance_id, 0);
            assert_eq!(*range, VISIBILITY_RADIUS);
            assert!(!required_3d);
        }
        other => panic!("expected NearbyVisible, got {other:?}"),
    }

    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, guid).expect("legacy creature");
        assert_eq!(
            creature.state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
        assert_eq!(
            creature.runtime_motion_master_ticks_like_cpp(),
            1,
            "one global map frame must call MotionMaster::Update once per creature, independent of fanout recipients"
        );
    }
    let legacy_authority = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .creature
        .loot_authority_like_cpp()
        .clone();
    {
        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(guid, Clone::clone)
            .expect("canonical creature sync must keep typed record fresh");
        assert_eq!(
            typed.ai_state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
        assert!(legacy_authority.shares_storage_like_cpp(typed.loot_authority_like_cpp()));
    }
}
#[test]
fn legacy_creature_movement_tick_once_uses_creature_visibility_override_like_cpp() {
    use crate::map_manager::{RecipientRule, RuntimeTickOwner};
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_009);
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
            creature
                .creature
                .unit_mut()
                .world_mut()
                .set_visibility_distance_override_like_cpp(
                    wow_entities::VisibilityDistanceTypeLikeCpp::Gigantic,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let outcome = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        None,
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );

    assert_eq!(outcome.movement_packets, 1);
    let event = &outcome.plan.events[0];
    match &event.recipients {
        RecipientRule::NearbyVisible { range, .. } => assert_eq!(
            *range,
            wow_entities::VisibilityDistanceTypeLikeCpp::Gigantic.distance_like_cpp(),
            "C++ SendMessageToSet uses source GetVisibilityRange(), including creature addon visibility overrides"
        ),
        other => panic!("expected NearbyVisible, got {other:?}"),
    }
}
#[test]
fn legacy_creature_melee_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_001);
    add_canonical_test_player_on_map(&canonical, player, Position::ZERO, 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_002);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
    assert!(outcome.commands.is_empty());
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 100);
}
#[test]
fn legacy_creature_aggro_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_005);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    let player = ObjectGuid::create_player(1, 91_006);
    let candidates = vec![legacy_aggro_candidate_like_cpp(player, Position::ZERO)];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
