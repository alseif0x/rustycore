use super::*;

/// Target 3.4.3 `Unit::DealDamage` computes every share from the same
/// post-split `damageDone`. Share neither subtracts from the primary hit nor
/// recursively triggers a share aura on the secondary target (`NODAMAGE`).
#[test]
fn legacy_creature_melee_tick_once_shares_post_split_player_damage_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let victim_guid = ObjectGuid::create_player(1, 91_380);
    let secondary_guid = ObjectGuid::create_player(1, 91_381);
    let recursive_guid = ObjectGuid::create_player(1, 91_382);
    let attacker_guid = test_creature_guid(91_383);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _, _) = make_session();
    let (mut secondary_session, _, _) = make_session();
    let (mut recursive_session, _, _) = make_session();
    attach_share_test_player_like_cpp(
        &mut session,
        &canonical,
        Arc::clone(&map_store),
        victim_guid,
        "ShareVictim",
    );
    attach_share_test_player_like_cpp(
        &mut secondary_session,
        &canonical,
        Arc::clone(&map_store),
        secondary_guid,
        "ShareCaster",
    );
    attach_share_test_player_like_cpp(
        &mut recursive_session,
        &canonical,
        map_store,
        recursive_guid,
        "RecursiveCaster",
    );
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spells = wow_data::SpellStore::new();
    for spell in [
        damage_aura_spell_like_cpp(
            91_384,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
            0,
        ),
        damage_aura_spell_like_cpp(
            91_385,
            wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_386,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_387,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_388,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
    ] {
        spells.insert(spell.spell_id, spell);
    }
    let spells = Arc::new(spells);
    for target in [&mut session, &mut secondary_session, &mut recursive_session] {
        target.set_spell_store(Arc::clone(&spells));
    }
    session.apply_aura(91_384, victim_guid, 30_000, 1).unwrap();
    session.apply_aura(91_385, victim_guid, 30_000, 1).unwrap();
    set_player_aura_caster_like_cpp(&mut session, 91_385, secondary_guid);
    session.apply_aura(91_386, victim_guid, 30_000, 1).unwrap();
    set_player_aura_caster_like_cpp(&mut session, 91_386, secondary_guid);
    session.apply_aura(91_388, victim_guid, 30_000, 1).unwrap();
    set_player_aura_caster_like_cpp(&mut session, 91_388, victim_guid);
    secondary_session
        .apply_aura(91_387, secondary_guid, 30_000, 1)
        .unwrap();
    set_player_aura_caster_like_cpp(&mut secondary_session, 91_387, recursive_guid);

    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spells),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.first().expect("primary player command");
    assert_eq!((command.damage, command.absorbed), (5, 5));
    assert_eq!(command.split_combat_log_packets.len(), 1);
    let canonical = canonical.lock().unwrap();
    let map = canonical.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(victim_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        93
    );
    // Five split damage plus 50% of the post-split five-damage primary.
    assert_eq!(
        map.get_typed_player(secondary_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        93
    );
    assert_eq!(
        map.get_typed_player(recursive_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        100
    );
    drop(canonical);
    assert_eq!(command.self_share_health_updates, vec![98]);
    assert_eq!(outcome.legacy_creature_victim_syncs, 0);
    assert!(outcome.plan.events.iter().any(|event| {
        event.recipients == wow_world::map_manager::RecipientRule::ExplicitPlayer(secondary_guid)
            && wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::HealthUpdate)
    }));
}
