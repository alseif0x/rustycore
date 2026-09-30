use super::*;

/// `Unit::DealDamage` applies the Creature static-flag clamp after the melee
/// result has already been serialized. The client therefore sees the raw hit,
/// while canonical health, death and the compatibility mirror retain 1 HP.
#[test]
fn legacy_creature_melee_tick_once_preserves_unkillable_creature_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_398);
    let victim_guid = test_creature_guid(91_399);
    let zero_damage_victim_guid = test_creature_guid(91_400);
    let lethal_victim_guid = test_creature_guid(91_406);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 4);
    register_test_creature(&mut session, manager.clone(), zero_damage_victim_guid, 1);
    register_test_creature(&mut session, manager.clone(), lethal_victim_guid, 4);
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    {
        let mut static_flags = [0; 8];
        static_flags[0] = wow_constants::creature::CreatureStaticFlags::UNKILLABLE.bits();
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        for guid in [victim_guid, zero_damage_victim_guid] {
            map.get_typed_creature_mut(guid)
                .unwrap()
                .set_static_flags_runtime_like_cpp(static_flags);
        }
    }
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    let canonical_guard = canonical.lock().unwrap();
    let (health, alive, ai_state, threat, reciprocal) = canonical_guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim_guid, |victim| {
            (
                victim.unit().data().health,
                victim.is_alive(),
                victim.ai_ownership().state,
                victim
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(attacker_guid),
                canonical_guard
                    .find_map(0, 0)
                    .unwrap()
                    .map()
                    .with_creature_like_cpp(attacker_guid, |attacker| {
                        attacker
                            .unit()
                            .subsystems()
                            .combat
                            .threatened_by_me_owner_guids()
                            .contains(&victim_guid)
                    })
                    .unwrap(),
            )
        })
        .unwrap();
    assert_eq!(health, 1);
    assert!(alive);
    assert_ne!(ai_state, wow_entities::CreatureAiState::Dead);
    assert_eq!(threat, Some(3.0), "threat uses post-UNKILLABLE damage");
    assert!(reciprocal);
    drop(canonical_guard);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    {
        let manager = manager.read().unwrap();
        assert_eq!(
            manager
                .find_creature(0, 0, victim_guid)
                .unwrap()
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid),
            Some(3.0)
        );
        assert!(
            manager
                .find_creature(0, 0, attacker_guid)
                .unwrap()
                .creature
                .unit()
                .subsystems()
                .combat
                .threatened_by_me_owner_guids()
                .contains(&victim_guid)
        );
    }

    let attacker_state = outcome
        .plan
        .events
        .iter()
        .find(|event| {
            wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::AttackerStateUpdate)
        })
        .expect("primary attacker state");
    let mut packet = wow_packet::WorldPacket::from_bytes(&attacker_state.packet_bytes);
    packet.read_uint16().expect("opcode");
    assert!(!packet.read_bit().expect("has log data"));
    let info_len = packet.read_uint32().expect("attack round size") as usize;
    let info_bytes = packet.read_bytes(info_len).expect("attack round bytes");
    let mut info = wow_packet::WorldPacket::from_bytes(&info_bytes);
    info.read_uint32().expect("hit info");
    info.read_packed_guid().expect("attacker");
    info.read_packed_guid().expect("victim");
    assert_eq!(
        info.read_int32().expect("wire damage"),
        10,
        "the pre-DealDamage attacker-state packet retains raw damage"
    );

    // `damageDone` is still ten, but UNKILLABLE clamps `damageTaken` to zero
    // at one HP. C++ still calls AddThreat(0), entering combat and creating
    // reciprocal references without increasing the numeric value.
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.enter_combat(zero_damage_victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let zero_damage = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(zero_damage.legacy_creature_victim_syncs, 1);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        let (health, ai_state, threat) = map
            .with_creature_like_cpp(zero_damage_victim_guid, |victim| {
                (
                    victim.unit().data().health,
                    victim.ai_ownership().state,
                    victim
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(attacker_guid),
                )
            })
            .unwrap();
        assert_eq!(health, 1);
        assert_eq!(ai_state, wow_entities::CreatureAiState::InCombat);
        assert_eq!(threat, Some(0.0));
        assert!(
            map.with_creature_like_cpp(attacker_guid, |attacker| {
                attacker
                    .unit()
                    .subsystems()
                    .combat
                    .threatened_by_me_owner_guids()
                    .contains(&zero_damage_victim_guid)
            })
            .unwrap()
        );
    }
    {
        let manager = manager.read().unwrap();
        let victim = manager
            .find_creature(0, 0, zero_damage_victim_guid)
            .unwrap();
        assert_eq!(victim.creature.unit().data().health, 1);
        assert_eq!(victim.state(), wow_entities::CreatureAiState::InCombat);
        assert_eq!(
            victim
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid),
            Some(0.0)
        );
        assert!(
            manager
                .find_creature(0, 0, attacker_guid)
                .unwrap()
                .creature
                .unit()
                .subsystems()
                .combat
                .threatened_by_me_owner_guids()
                .contains(&zero_damage_victim_guid)
        );
    }

    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.enter_combat(lethal_victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let lethal = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(lethal.legacy_creature_victim_syncs, 1);
    {
        let canonical = canonical.lock().unwrap();
        let (health, threat) = canonical
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(lethal_victim_guid, |victim| {
                (
                    victim.unit().data().health,
                    victim
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(attacker_guid),
                )
            })
            .unwrap();
        assert_eq!(health, 0);
        assert_eq!(
            threat, None,
            "the lethal branch does not execute nonlethal threat settlement"
        );
    }
    {
        let manager = manager.read().unwrap();
        let victim = manager.find_creature(0, 0, lethal_victim_guid).unwrap();
        assert_eq!(victim.creature.unit().data().health, 0);
        assert_eq!(
            victim
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid),
            None
        );
    }
}
