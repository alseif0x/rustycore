use super::*;

/// Creature targets retain the C++ packet/mutation order: the primary
/// `AttackerStateUpdate` is serialized before `DealDamage` shares, and the
/// primary values update follows those secondary mutations. Multiple share
/// auras each use the same base, including a self-targeting aura.
#[test]
fn legacy_creature_melee_tick_once_shares_creature_damage_in_cpp_order() {
    use wow_world::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_390);
    let victim_guid = test_creature_guid(91_391);
    let secondary_guid = test_creature_guid(91_392);
    let recursive_guid = test_creature_guid(91_393);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for guid in [attacker_guid, victim_guid, secondary_guid, recursive_guid] {
        register_test_creature(&mut session, manager.clone(), guid, 100);
    }
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
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
            91_394,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
            5,
            0,
        ),
        damage_aura_spell_like_cpp(
            91_395,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_396,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_397,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
        damage_aura_spell_like_cpp(
            91_398,
            wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT,
            50,
            0x01,
        ),
    ] {
        spells.insert(spell.spell_id, spell);
    }
    let mut no_threat_attributes = [0; 15];
    no_threat_attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT;
    spells.insert_spell_misc_attributes_like_cpp(91_396, no_threat_attributes);
    let spells = Arc::new(spells);
    session.set_spell_store(Arc::clone(&spells));
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_394,
                    attacker_guid,
                    0,
                    1,
                ));
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_398,
                    attacker_guid,
                    1,
                    1,
                ));
        })
        .unwrap();
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        let victim = map.get_typed_creature_mut(victim_guid).unwrap();
        victim
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(wow_entities::AppliedAuraRef::new(
                91_395,
                secondary_guid,
                0,
                1,
            ));
        victim
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(wow_entities::AppliedAuraRef::new(91_396, victim_guid, 1, 1));
        map.get_typed_creature_mut(secondary_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(wow_entities::AppliedAuraRef::new(
                91_397,
                recursive_guid,
                0,
                1,
            ));
        map.get_typed_creature_mut(attacker_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(wow_entities::AppliedAuraRef::new(
                91_398,
                attacker_guid,
                1,
                1,
            ));
    }

    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spells),
        spell_threat_store: Some(Arc::new(wow_data::SpellThreatStoreLikeCpp {
            entries_by_spell_id: std::collections::HashMap::from([(
                91_395,
                wow_data::SpellThreatEntryLikeCpp {
                    flat_mod: 0,
                    pct_mod: 2.0,
                    ap_pct_mod: 0.0,
                },
            )]),
        })),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        // Self share (5) plus the unchanged primary hit (10).
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            85
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(secondary_guid)
                .unwrap()
                .health,
            95
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(recursive_guid)
                .unwrap()
                .health,
            100
        );
        assert_eq!(
            map.with_creature_like_cpp(victim_guid, |victim| victim
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid))
                .flatten(),
            Some(15.0),
            "NO_THREAT suppresses self-share while physical threat uses MOD_THREAT"
        );
        assert_eq!(
            map.with_creature_like_cpp(secondary_guid, |victim| victim
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid))
                .flatten(),
            Some(15.0),
            "spell pctMod and the attacker's school modifier both apply"
        );
        let threatened_by = map
            .with_creature_like_cpp(attacker_guid, |attacker| {
                attacker
                    .unit()
                    .subsystems()
                    .combat
                    .threatened_by_me_owner_guids()
            })
            .unwrap();
        assert!(threatened_by.contains(&victim_guid));
        assert!(threatened_by.contains(&secondary_guid));
    }
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
            Some(15.0)
        );
        assert_eq!(
            manager
                .find_creature(0, 0, secondary_guid)
                .unwrap()
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid),
            Some(15.0)
        );
        let threatened_by = manager
            .find_creature(0, 0, attacker_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids();
        assert!(threatened_by.contains(&victim_guid));
        assert!(threatened_by.contains(&secondary_guid));
    }
    assert_eq!(outcome.legacy_creature_victim_syncs, 3);
    assert!(!outcome.plan.events.iter().any(|event| {
        wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
            == Some(ServerOpcodes::SpellNonMeleeDamageLog)
    }));
    let attacker_state = outcome
        .plan
        .events
        .iter()
        .position(|event| {
            wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::AttackerStateUpdate)
        })
        .expect("primary attacker state");
    let secondary_update = outcome
        .plan
        .events
        .iter()
        .position(|event| event.source_guid == secondary_guid)
        .expect("secondary values update");
    let primary_update = outcome
        .plan
        .events
        .iter()
        .rposition(|event| event.source_guid == victim_guid)
        .expect("primary values update");
    assert!(attacker_state < secondary_update && secondary_update < primary_update);

    // `DealDamageMods` zeroes an evading share target without reducing the
    // primary hit. The self share remains an independent five-damage copy.
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(secondary_guid)
            .unwrap()
            .set_in_evade_mode_like_cpp(true);
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .set_sparring_health_pct_like_cpp(0.0);
    }
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let evading = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            70
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(secondary_guid)
                .unwrap()
                .health,
            95
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(recursive_guid)
                .unwrap()
                .health,
            100
        );
    }
    assert_eq!(evading.legacy_creature_victim_syncs, 2);

    // Outer `DealDamage` sparring modifies `damageDone` before the share loop.
    // At the threshold, neither share target nor primary loses health even
    // though the already-serialized attacker state retains the raw hit.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(victim_guid)
        .unwrap()
        .set_sparring_health_pct_like_cpp(100.0);
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let sparring =
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            70
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(secondary_guid)
                .unwrap()
                .health,
            95
        );
    }
    assert_eq!(sparring.legacy_creature_victim_syncs, 0);

    // A recursive `NODAMAGE` share still traverses the Creature unkillable
    // branch in `DealDamage`. It does not reduce the primary hit or add a
    // combat-log frame, but its own health transition stops at one.
    session
        .fixture_melee_mutate_creature(secondary_guid, |creature| {
            creature.creature.unit_mut().set_health(4);
        })
        .unwrap();
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .set_sparring_health_pct_like_cpp(0.0);
        let secondary = map.get_typed_creature_mut(secondary_guid).unwrap();
        secondary.set_in_evade_mode_like_cpp(false);
        secondary.unit_mut().set_health(4);
        let mut static_flags = [0; 8];
        static_flags[0] = wow_constants::creature::CreatureStaticFlags::UNKILLABLE.bits();
        secondary.set_static_flags_runtime_like_cpp(static_flags);
    }
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let unkillable =
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        let (health, alive, ai_state) = map
            .with_creature_like_cpp(secondary_guid, |secondary| {
                (
                    secondary.unit().data().health,
                    secondary.is_alive(),
                    secondary.ai_ownership().state,
                )
            })
            .unwrap();
        assert_eq!(health, 1);
        assert!(alive);
        assert_ne!(ai_state, wow_entities::CreatureAiState::Dead);
    }
    assert_eq!(unkillable.legacy_creature_victim_syncs, 3);
}
