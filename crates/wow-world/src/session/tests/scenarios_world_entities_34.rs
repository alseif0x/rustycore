//! Damage-immunity melee regression under #29.

#[path = "scenarios_world_entities_35.rs"]
mod share_damage;

use super::*;

/// C++ `Unit::IsImmunedToDamage` (`Unit.cpp:7318-7336`) checks both the
/// school-immunity and `IMMUNITY_DAMAGE` registries before the melee hit table.
/// This production-shaped player-victim path proves the latter independently.
#[test]
fn legacy_creature_melee_tick_once_honors_player_damage_immunity_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let player = ObjectGuid::create_player(1, 91_340);
    let creature_guid = test_creature_guid(91_341);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "DamageImmune".to_string(),
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
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_350_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
            0,
        ),
        (
            91_351,
            wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
            0,
            0x01,
        ),
    ] {
        spell_store.insert(
            spell_id,
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
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .apply_aura(91_350, player, 30_000, 1)
        .expect("apply hit-chance aura");
    session
        .apply_aura(91_351, player, 30_000, 1)
        .expect("apply damage-immunity aura");

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 0);
    assert_eq!(outcome.commands[0].victim_state, VICTIM_STATE_IS_IMMUNE);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap()
            .unit()
            .data()
            .health,
        100
    );
}

/// C++ `CalcAbsorbResist` applies `SPLIT_DAMAGE_PCT` after school/mana absorbs,
/// subtracts it from the primary hit and deals the secondary direct damage to
/// the live aura caster before publishing the primary attacker-state result.
#[test]
fn legacy_creature_melee_tick_once_splits_player_victim_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let player = ObjectGuid::create_player(1, 91_360);
    let attacker_guid = test_creature_guid(91_361);
    let split_target_guid = ObjectGuid::create_player(1, 91_362);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "SplitVictim".to_string(),
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
    let (mut split_session, _, _) = make_session();
    split_session.set_canonical_map_manager(Arc::clone(&canonical));
    split_session.set_map_store(map_store);
    split_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        split_target_guid,
        "SplitCaster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = split_session.ensure_canonical_world_map_for_current_player_like_cpp();
    split_session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
        })
        .unwrap();
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
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_363_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
            0,
        ),
        (
            91_364,
            wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
            50,
            0x01,
        ),
        (
            91_365,
            wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
            0,
            0x01,
        ),
    ] {
        spell_store.insert(
            spell_id,
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
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session.apply_aura(91_363, player, 30_000, 1).unwrap();
    session.apply_aura(91_364, player, 30_000, 1).unwrap();
    let split_cast_id = ObjectGuid::new(6, 91_364);
    session
        .mutate_player_aura_subsystem_like_cpp(|auras| {
            let split = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(slot, aura)| (aura.spell_id == 91_364).then_some(*slot))
                .expect("split aura slot");
            auras
                .runtime_application_mut_like_cpp(split)
                .expect("split aura")
                .caster_guid = split_target_guid;
            auras.set_aura_cast_provenance_like_cpp(
                split,
                wow_entities::AuraCastProvenanceLikeCpp {
                    cast_id: split_cast_id,
                    spell_visual_id: 7_364,
                },
            );
        })
        .unwrap();

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("primary victim command");
    assert_eq!((command.damage, command.absorbed), (5, 5));
    assert_eq!(command.split_combat_log_packets.len(), 1);
    assert_eq!(
        command.split_combat_log_packets[0],
        wow_packet::packets::combat::SpellNonMeleeDamageLog {
            target: split_target_guid,
            caster: attacker_guid,
            cast_id: split_cast_id,
            spell_id: 91_364,
            visual_id: 7_364,
            damage: 5,
            original_damage: 5,
            overkill: -1,
            school_mask: 1,
            absorbed: 0,
            resisted: 0,
            shield_block: 0,
            periodic: false,
            flags: 0,
        }
        .to_bytes(),
        "C++ builds the split log from the aura base cast and visual provenance"
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap()
            .unit()
            .data()
            .health,
        95
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(split_target_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        95
    );
    assert_eq!(outcome.legacy_creature_victim_syncs, 0);
    assert!(outcome.plan.events.iter().any(|event| {
        event.recipients == crate::map_manager::RecipientRule::ExplicitPlayer(split_target_guid)
            && wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::HealthUpdate)
    }));

    // A missing caster fails the C++ liveness lookup before any percentage is
    // removed from the primary hit.
    session
        .mutate_player_aura_subsystem_like_cpp(|auras| {
            let split = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(slot, aura)| (aura.spell_id == 91_364).then_some(*slot))
                .unwrap();
            auras
                .runtime_application_mut_like_cpp(split)
                .unwrap()
                .caster_guid = test_creature_guid(999_999);
        })
        .unwrap();
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let missing = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(
        (missing.commands[0].damage, missing.commands[0].absorbed),
        (10, 0)
    );
    assert!(missing.commands[0].split_combat_log_packets.is_empty());

    // C++ absorbs the split from the primary hit before testing immunity, then
    // leaves the secondary target unchanged and publishes SPELL_MISS_IMMUNE.
    session
        .mutate_player_aura_subsystem_like_cpp(|auras| {
            let split = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(slot, aura)| (aura.spell_id == 91_364).then_some(*slot))
                .unwrap();
            auras
                .runtime_application_mut_like_cpp(split)
                .unwrap()
                .caster_guid = split_target_guid;
        })
        .unwrap();
    split_session.set_spell_store(config.spell_store.as_ref().expect("spell store").clone());
    split_session
        .apply_aura(91_365, split_target_guid, 30_000, 1)
        .unwrap();
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let immune = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(
        (immune.commands[0].damage, immune.commands[0].absorbed),
        (5, 5)
    );
    assert_eq!(immune.commands[0].split_combat_log_packets.len(), 1);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&immune.commands[0].split_combat_log_packets[0])
            .server_opcode(),
        Some(ServerOpcodes::SpellMissLog)
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(split_target_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        95
    );
}

/// Creature victims use the same canonical split stage. Its secondary log is
/// ordered after any absorb publications and before the primary
/// `AttackerStateUpdate` in the map runtime plan.
#[test]
fn legacy_creature_melee_tick_once_splits_creature_victim_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_370);
    let victim_guid = test_creature_guid(91_371);
    let split_target_guid = test_creature_guid(91_372);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for guid in [attacker_guid, victim_guid, split_target_guid] {
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

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_373_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            100,
            0,
        ),
        (
            91_374,
            wow_data::spell::aura_types::SPELL_AURA_SPLIT_DAMAGE_PCT,
            50,
            0x01,
        ),
    ] {
        spell_store.insert(
            spell_id,
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
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_373,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();
    let split_cast_id = ObjectGuid::new(6, 91_374);
    {
        let mut manager = canonical.lock().unwrap();
        let auras = &mut manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .auras;
        auras.add_applied(wow_entities::AppliedAuraRef::new(
            91_374,
            split_target_guid,
            0,
            1,
        ));
        auras.set_aura_cast_provenance_like_cpp(
            0,
            wow_entities::AuraCastProvenanceLikeCpp {
                cast_id: split_cast_id,
                spell_visual_id: 7_374,
            },
        );
    }

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            95
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(split_target_guid)
                .unwrap()
                .health,
            95
        );
        assert_eq!(
            map.with_creature_like_cpp(victim_guid, |victim| victim
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid))
                .flatten(),
            Some(5.0),
            "the primary call settles threat from its post-split damage"
        );
        assert_eq!(
            map.with_creature_like_cpp(split_target_guid, |victim| victim
                .unit()
                .subsystems()
                .combat
                .threat_value(attacker_guid))
                .flatten(),
            Some(5.0),
            "the recursive split call settles its own threat"
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
        assert!(threatened_by.contains(&split_target_guid));
    }
    {
        let manager = manager.read().unwrap();
        for target_guid in [victim_guid, split_target_guid] {
            assert_eq!(
                manager
                    .find_creature(0, 0, target_guid)
                    .unwrap()
                    .creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(attacker_guid),
                Some(5.0),
                "the compatibility mirror replays each damage call's threat"
            );
        }
        let threatened_by = manager
            .find_creature(0, 0, attacker_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids();
        assert!(threatened_by.contains(&victim_guid));
        assert!(threatened_by.contains(&split_target_guid));
    }
    let opcodes = outcome
        .plan
        .events
        .iter()
        .filter_map(|event| {
            (event.packet_bytes.len() >= 2)
                .then(|| u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]]))
        })
        .collect::<Vec<_>>();
    let split_log = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::SpellNonMeleeDamageLog as u16)
        .expect("secondary split log");
    let split_log_bytes = &outcome.plan.events[split_log].packet_bytes;
    assert_eq!(
        split_log_bytes,
        &wow_packet::packets::combat::SpellNonMeleeDamageLog {
            target: split_target_guid,
            caster: attacker_guid,
            cast_id: split_cast_id,
            spell_id: 91_374,
            visual_id: 7_374,
            damage: 5,
            original_damage: 5,
            overkill: -1,
            school_mask: 1,
            absorbed: 0,
            resisted: 0,
            shield_block: 0,
            periodic: false,
            flags: 0,
        }
        .to_bytes()
    );
    let primary = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AttackerStateUpdate as u16)
        .expect("primary attacker state");
    assert!(split_log < primary);
    assert_eq!(outcome.legacy_creature_victim_syncs, 2);

    // `DealDamageMods` runs after the split has been absorbed from the
    // primary victim. An evading secondary creature therefore takes no health
    // damage, while the primary still loses only the unsplit remainder.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(split_target_guid)
        .unwrap()
        .set_in_evade_mode_like_cpp(true);
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let evading = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            90
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(split_target_guid)
                .unwrap()
                .health,
            95
        );
    }
    assert_eq!(evading.legacy_creature_victim_syncs, 1);
    assert!(evading.plan.events.iter().any(|event| {
        wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
            == Some(ServerOpcodes::SpellNonMeleeDamageLog)
    }));

    // The unusual C++ split tail also zeroes the primary Creature's remaining
    // wire damage when it is already at its sparring threshold, after a
    // non-immune secondary reaches `DealDamageMods` (`Unit.cpp:2000-2003`).
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(split_target_guid)
            .unwrap()
            .set_in_evade_mode_like_cpp(false);
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .set_sparring_health_pct_like_cpp(100.0);
    }
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let sparring =
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            90
        );
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(split_target_guid)
                .unwrap()
                .health,
            90
        );
    }
    let attacker_state = sparring
        .plan
        .events
        .iter()
        .find(|event| {
            wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::AttackerStateUpdate)
        })
        .expect("primary attacker state after split sparring");
    let mut packet = wow_packet::WorldPacket::from_bytes(&attacker_state.packet_bytes);
    packet.read_uint16().expect("opcode");
    assert!(!packet.read_bit().expect("has log data"));
    let info_len = packet.read_uint32().expect("attack round size") as usize;
    let info_bytes = packet.read_bytes(info_len).expect("attack round bytes");
    let mut info = wow_packet::WorldPacket::from_bytes(&info_bytes);
    info.read_uint32().expect("hit info");
    info.read_packed_guid().expect("attacker");
    info.read_packed_guid().expect("victim");
    assert_eq!(info.read_int32().expect("primary wire damage"), 0);

    // The secondary `DealDamage` still applies Creature unkillable semantics.
    // Its non-melee log retains the five-point split calculated before the
    // health clamp, while the canonical target remains alive at one HP.
    session
        .mutate_world_creature(split_target_guid, |creature| {
            creature.creature.unit_mut().set_health(4);
        })
        .unwrap();
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .set_sparring_health_pct_like_cpp(0.0);
        let split_target = map.get_typed_creature_mut(split_target_guid).unwrap();
        split_target.set_sparring_health_pct_like_cpp(0.0);
        split_target.unit_mut().set_health(4);
        let mut static_flags = [0; 8];
        static_flags[0] = wow_constants::creature::CreatureStaticFlags::UNKILLABLE.bits();
        split_target.set_static_flags_runtime_like_cpp(static_flags);
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
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        assert_eq!(
            map.creature_transform_vitals_snapshot_like_cpp(victim_guid)
                .unwrap()
                .health,
            85
        );
        let (health, alive, ai_state) = map
            .with_creature_like_cpp(split_target_guid, |split_target| {
                (
                    split_target.unit().data().health,
                    split_target.is_alive(),
                    split_target.ai_ownership().state,
                )
            })
            .unwrap();
        assert_eq!(health, 1);
        assert!(alive);
        assert_ne!(ai_state, wow_entities::CreatureAiState::Dead);
    }
    assert_eq!(unkillable.legacy_creature_victim_syncs, 2);
    assert!(unkillable.plan.events.iter().any(|event| {
        wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
            == Some(ServerOpcodes::SpellNonMeleeDamageLog)
    }));
}
