use super::*;

#[test]
fn legacy_creature_melee_tick_once_preserves_sparring_damage_clamp_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_030);
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(52);
        typed.set_sparring_health_pct_like_cpp(50.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_031);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, Arc::clone(&manager), victim, 52);
    session
        .fixture_melee_mutate_creature(victim, |creature| {
            creature.creature.set_sparring_health_pct_like_cpp(50.0);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .fixture_melee_mutate_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.ai_ownership_mut().min_damage = 52;
            creature.creature.ai_ownership_mut().max_damage = 52;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert_eq!(outcome.legacy_creature_victim_sync_cas_rejections, 0);
    assert_eq!(outcome.plan.events.len(), 2);
    let mut packet =
        wow_packet::world_packet::WorldPacket::from_bytes(&outcome.plan.events[0].packet_bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AttackerStateUpdate as u16
    );
    assert!(!packet.read_bit().unwrap());
    let info_len = packet.read_uint32().unwrap() as usize;
    let info_bytes = packet.read_bytes(info_len).unwrap();
    let mut info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
    assert_eq!(info.read_uint32().unwrap(), 0x0000_0002);
    assert_eq!(info.read_packed_guid().unwrap(), attacker);
    assert_eq!(info.read_packed_guid().unwrap(), victim);
    assert_eq!(info.read_int32().unwrap(), 52);
    assert_eq!(info.read_int32().unwrap(), 52);
    assert_eq!(
        info.read_int32().unwrap(),
        0,
        "C++ wire overdamage uses raw damage before the sparring clamp"
    );
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 50);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, victim)
            .unwrap()
            .creature
            .unit()
            .data()
            .health,
        50
    );
}
