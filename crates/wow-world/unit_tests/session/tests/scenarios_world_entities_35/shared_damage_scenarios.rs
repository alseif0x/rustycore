use super::*;

fn damage_aura_spell_like_cpp(
    spell_id: i32,
    aura_type: i32,
    amount: i32,
    misc_value: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: amount,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_base_points: amount,
            ..Default::default()
        }],
    }
}

fn attach_share_test_player_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    map_store: Arc<wow_data::MapStore>,
    guid: ObjectGuid,
    name: &str,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.set_map_store(map_store);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        name.to_string(),
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
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
}

fn set_player_aura_caster_like_cpp(session: &mut WorldSession, spell_id: i32, caster: ObjectGuid) {
    session
        .mutate_player_aura_subsystem_like_cpp(|auras| {
            let slot = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(slot, aura)| (aura.spell_id == spell_id).then_some(*slot))
                .expect("aura slot");
            auras
                .runtime_application_mut_like_cpp(slot)
                .expect("aura application")
                .caster_guid = caster;
        })
        .unwrap();
}

/// Target 3.4.3 `Unit::DealDamage` computes every share from the same
/// post-split `damageDone`. Share neither subtracts from the primary hit nor
/// recursively triggers a share aura on the secondary target (`NODAMAGE`).
#[test]
fn legacy_creature_melee_tick_once_shares_post_split_player_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
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
        .mutate_world_creature(attacker_guid, |creature| {
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

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
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
        event.recipients == crate::map_manager::RecipientRule::ExplicitPlayer(secondary_guid)
            && wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::HealthUpdate)
    }));
}

/// Creature targets retain the C++ packet/mutation order: the primary
/// `AttackerStateUpdate` is serialized before `DealDamage` shares, and the
/// primary values update follows those secondary mutations. Multiple share
/// auras each use the same base, including a self-targeting aura.
#[test]
fn legacy_creature_melee_tick_once_shares_creature_damage_in_cpp_order() {
    use crate::map_manager::RuntimeTickOwner;
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
        .mutate_world_creature(attacker_guid, |creature| {
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
        .mutate_world_creature(attacker_guid, |creature| {
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

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
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
        .mutate_world_creature(attacker_guid, |creature| {
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
        .mutate_world_creature(attacker_guid, |creature| {
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
        .mutate_world_creature(secondary_guid, |creature| {
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
        .mutate_world_creature(attacker_guid, |creature| {
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

/// `SPELL_ATTR2_NO_INITIAL_THREAT` does not prevent the recursive share
/// damage, but `ThreatManager::AddThreat` skips a target that is not engaged.
/// Once that same target already has combat, a later share creates threat.
#[test]
fn legacy_creature_melee_share_honors_no_initial_threat_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_401);
    let victim_guid = test_creature_guid(91_402);
    let secondary_guid = test_creature_guid(91_403);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for guid in [attacker_guid, victim_guid, secondary_guid] {
        register_test_creature(&mut session, manager.clone(), guid, 100);
    }
    session
        .mutate_world_creature(attacker_guid, |creature| {
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

    let hit_spell_id = 91_404;
    let share_spell_id = 91_405;
    let mut spells = wow_data::SpellStore::new();
    spells.insert(
        hit_spell_id,
        damage_aura_spell_like_cpp(
            hit_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
            5,
            0,
        ),
    );
    spells.insert(
        share_spell_id,
        damage_aura_spell_like_cpp(
            share_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
    );
    let mut attributes = [0; 15];
    attributes[2] = wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT;
    spells.insert_spell_misc_attributes_like_cpp(share_spell_id, attributes);
    let spells = Arc::new(spells);
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    hit_spell_id as u32,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(victim_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(wow_entities::AppliedAuraRef::new(
            share_spell_id as u32,
            secondary_guid,
            0,
            1,
        ));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spells),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let first = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(first.legacy_creature_victim_syncs, 2);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        let (health, threat, ai_state) = map
            .with_creature_like_cpp(secondary_guid, |secondary| {
                (
                    secondary.unit().data().health,
                    secondary
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(attacker_guid),
                    secondary.ai_ownership().state,
                )
            })
            .unwrap();
        assert_eq!(health, 95);
        assert_eq!(threat, None);
        assert_ne!(ai_state, wow_entities::CreatureAiState::InCombat);
    }

    {
        let mut canonical = canonical.lock().unwrap();
        canonical
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(secondary_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, false, false);
    }
    session
        .mutate_world_creature(secondary_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_in_combat_with(attacker_guid, false, false);
        })
        .unwrap();
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let second = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(second.legacy_creature_victim_syncs, 2);
    for threat in [
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(secondary_guid, |secondary| {
                secondary
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(attacker_guid)
            })
            .flatten(),
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, secondary_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(attacker_guid),
    ] {
        assert_eq!(threat, Some(5.0));
    }
}
