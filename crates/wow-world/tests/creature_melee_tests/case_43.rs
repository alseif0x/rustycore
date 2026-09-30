use super::*;

/// C++ `CalcAbsorbResist` applies `SPLIT_DAMAGE_PCT` after school/mana absorbs,
/// subtracts it from the primary hit and deals the secondary direct damage to
/// the live aura caster before publishing the primary attacker-state result.
#[test]
fn legacy_creature_melee_tick_once_splits_player_victim_damage_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
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
    session.fixture_melee_attach_player_controller(SessionPlayerController::new(
        player,
        "SplitVictim".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.fixture_melee_ensure_world_map();
    session
        .fixture_melee_mutate_player(|player| {
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
    split_session.fixture_melee_attach_player_controller(SessionPlayerController::new(
        split_target_guid,
        "SplitCaster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = split_session.fixture_melee_ensure_world_map();
    split_session
        .fixture_melee_mutate_player(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
        })
        .unwrap();
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
        .fixture_melee_mutate_auras(|auras| {
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

    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
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
        event.recipients == wow_world::map_manager::RecipientRule::ExplicitPlayer(split_target_guid)
            && wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                == Some(ServerOpcodes::HealthUpdate)
    }));

    // A missing caster fails the C++ liveness lookup before any percentage is
    // removed from the primary hit.
    session
        .fixture_melee_mutate_auras(|auras| {
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
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
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
        .fixture_melee_mutate_auras(|auras| {
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
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
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
