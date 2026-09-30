use super::*;

#[test]
fn legacy_creature_melee_tick_once_preserves_fake_damage_wire_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_FAKE_DAMAGE};
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_032);
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
        typed.unit_mut().set_health(50);
        typed.set_sparring_health_pct_like_cpp(50.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_033);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .fixture_melee_mutate_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
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
    assert!(!outcome.plan.events.is_empty());
    let mut packet =
        wow_packet::world_packet::WorldPacket::from_bytes(&outcome.plan.events[0].packet_bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::AttackerStateUpdate as u16
    );
    assert!(!packet.read_bit().expect("has_log_data"));
    let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
    let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
    let mut attack_round_info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
    assert_eq!(
        attack_round_info.read_uint32().expect("hitInfo"),
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FAKE_DAMAGE
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
}
