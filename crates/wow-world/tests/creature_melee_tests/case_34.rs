use super::*;

/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop for a player victim
/// (`Unit.cpp:1886-1930`).
///
/// The map-owned swing drains the victim's mana in the same committed phase as
/// the health write, publishes the resulting `SMSG_POWER_UPDATE` through the
/// victim session on delivery, and leaves the shield's amount spent so the mana
/// it can drain is finite.
#[test]
fn legacy_creature_melee_tick_once_drains_player_mana_shield_like_cpp() {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_600);
    let creature_guid = test_creature_guid(91_601);

    let (mut session, _, send_rx) = make_session();
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
    session.fixture_melee_attach_player_controller(SessionPlayerController::new(
        player,
        "Victim".to_string(),
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
            player.set_power_index(wow_constants::PowerType::Mana, Some(0));
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Mana, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Mana, 100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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
    for (spell_id, aura_type, amount, amplitude, misc_value) in [
        (
            91_610_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0.0_f32,
            0_i32,
        ),
        (
            91_611,
            wow_data::spell::aura_types::SPELL_AURA_MANA_SHIELD,
            30,
            // One point of mana per point of damage absorbed.
            1.0,
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
                    effect_base_points: amount,
                    effect_amplitude: amplitude,
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: 0,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_610, player, 30_000, 1)
        .expect("apply hit-chance aura");
    session
        .apply_aura(91_611, player, 30_000, 1)
        .expect("apply mana shield aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .fixture_melee_mutate_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let victim_health = || {
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
            .health
    };
    let victim_mana = |session: &WorldSession| {
        session
            .fixture_melee_player_snapshot(|player| {
                player.unit().get_power(wow_constants::PowerType::Mana)
            })
            .expect("canonical victim")
    };
    let shield_amount = |session: &WorldSession| {
        session
            .fixture_melee_player_snapshot(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == 91_611)
                    .map(|aura| {
                        (
                            aura.slot,
                            aura.represented_effect_amounts
                                .iter()
                                .find(|represented| represented.effect_index == 0)
                                .map(|represented| represented.amount),
                        )
                    })
            })
            .flatten()
    };

    // Plenty of mana: the whole hit is absorbed and the drain is published.
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(command.mana_spent, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM | 0x20);
    assert_eq!(victim_health(), 100);
    assert_eq!(victim_mana(&session), 90);
    assert_eq!(shield_amount(&session).expect("shield").1, Some(20));
    let _ = drain_server_opcodes(&send_rx);
    session.fixture_melee_set_state(wow_world::session::SessionState::LoggedIn);
    session.fixture_melee_apply_damage_command(command);
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::PowerUpdate),
        "C++ ModifyPower publishes the drained mana"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::SpellAbsorbLog),
        "the mana shield publishes its absorb log"
    );

    // Only three mana left: the shield absorbs what the victim can pay and the
    // remainder lands.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player)
        .unwrap()
        .unit_mut()
        .set_power(wow_constants::PowerType::Mana, 3);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.mana_spent, 3);
    assert_eq!(command.absorbed, 3);
    assert_eq!(command.damage, 7);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(victim_health(), 93);
    assert_eq!(victim_mana(&session), 0);
    // Stage 1 left 20 points, and this one spent the three the victim could pay.
    assert_eq!(shield_amount(&session).expect("shield").1, Some(17));

    // No mana: the shield absorbs nothing and the full hit lands.
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.mana_spent, 0);
    assert_eq!(command.absorbed, 0);
    assert_eq!(command.damage, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(), 83);
}
