//! Creature melee absorb and mana-shield scenarios.

use super::*;

/// C++ `Unit::CalculateMeleeDamage`'s school-absorb stage for a player victim
/// (`Unit.cpp:1449-1466`, `Unit::CalcAbsorbResist` `Unit.cpp:1789-1880`).
///
/// The map-owned swing spends the victim's `SPELL_AURA_SCHOOL_ABSORB` amount in
/// the same committed phase as the health write, publishes the absorbed amount
/// with the `HITINFO_FULL_ABSORB`/`HITINFO_PARTIAL_ABSORB` bit, and hands the
/// exhausted slot to the victim session, which owns the aura removal
/// (`Unit.cpp:1856-1860`) and its publication. A spent shield therefore stops
/// absorbing and the next swing lands at full damage.
#[test]
fn legacy_creature_melee_tick_once_absorbs_player_victim_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_FULL_ABSORB, HIT_INFO_PARTIAL_ABSORB,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_500);
    let creature_guid = test_creature_guid(91_501);

    let (mut session, _, send_rx) = make_session();
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let observer = ObjectGuid::create_player(1, 91_513);
    let (observer_send_tx, _observer_send_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(1);
    let mut observer_info =
        broadcast_info_with_command(observer, observer_send_tx, observer_command_tx);
    observer_info.placement.map_id = 0;
    observer_info.placement.instance_id = 0;
    observer_info.placement.position = Position::new(10.0, 10.0, 0.0, 0.0);
    registry.register_or_replace(observer, observer_info, Default::default());
    session.set_player_registry(Arc::clone(&registry));
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
        "Victim".to_string(),
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
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .expect("canonical victim");

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_510_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_511,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            // `MiscValue` is the school mask: the normal school.
            0x01,
        ),
        (
            91_512,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            4,
            0x01,
        ),
        (
            91_514,
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE,
            100_000,
            0,
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
                    effect_misc_value_2: 0,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_510, player, 30_000, 1)
        .expect("apply hit-chance aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
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
    let shield_amount = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == 91_511)
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

    session
        .apply_aura(91_511, player, 30_000, 1)
        .expect("apply absorb shield");
    let (shield_slot, _) = shield_amount(&session).expect("shield applied");

    // First swing: the 30-point shield absorbs the whole 10-point hit. C++
    // publishes the zero dealt damage with the full-absorb bit and no health
    // transition, and the shield keeps 20 points.
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FULL_ABSORB
    );
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].slot, shield_slot);
    assert_eq!(command.absorb_consumptions[0].consumed, 10);
    assert!(
        !command.absorb_consumptions[0].removed,
        "a partially spent shield is not removed"
    );
    assert_eq!(victim_health(), 100, "a fully absorbed hit deals no damage");
    assert_eq!(shield_amount(&session).expect("shield").1, Some(20));

    // Second swing spends ten more and still publishes a full absorb.
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.commands.last().expect("command").damage, 0);
    assert_eq!(shield_amount(&session).expect("shield").1, Some(10));

    // Third swing spends the last ten points and reports the exhausted shield.
    // Delivering the command publishes C++'s absorb log first
    // (`Unit.cpp:1876-1889`) and then removes the shield through the session's
    // aura transition (`Unit::CalcAbsorbResist`'s
    // `Remove(AURA_REMOVE_BY_ENEMY_SPELL)`).
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].slot, shield_slot);
    assert_eq!(command.absorb_consumptions[0].consumed, 10);
    assert!(command.absorb_consumptions[0].removed);
    assert_eq!(victim_health(), 100);
    let _ = drain_server_opcodes(&send_rx);
    session.state = crate::session::SessionState::LoggedIn;
    session.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
    assert_eq!(
        shield_amount(&session),
        None,
        "the delivered command removes the spent shield"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    let absorb_log = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::SpellAbsorbLog)
        .expect("C++ publishes one absorb log per consuming shield");
    let removal = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AuraUpdate)
        .expect("the session-owned transition publishes the shield removal");
    assert!(
        absorb_log < removal,
        "C++ logs the absorb before removing the spent shield (`Unit.cpp:1876-1889`)"
    );
    let SessionCommand::SendRealmIfVisibleLikeCpp(observer_command) = observer_command_rx
        .try_recv()
        .expect("C++ fans the absorb log to nearby visible players")
    else {
        panic!("the absorb log must use the realm-visible observer rail");
    };
    assert_eq!(observer_command.source_guid, player);
    assert_eq!(observer_command.map_id, 0);
    assert_eq!(observer_command.instance_id, 0);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&observer_command.packet_bytes).server_opcode(),
        Some(ServerOpcodes::SpellAbsorbLog),
        "the observer receives the same C++ SpellAbsorbLog frame"
    );

    // Fourth swing: with the shield gone the full hit lands.
    session
        .apply_aura(91_514, player, 30_000, 1)
        .expect("apply a large elemental resistance aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 0);
    assert_eq!(command.damage, 10);
    // C++ `CalcSpellResistedDamage` returns zero before reading resistances for
    // a non-magic school (`Unit.cpp:1688-1693`); a physical white swing must
    // remain a ten-point hit despite the enormous aura above.
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(), 90);

    // A shield smaller than the hit leaves a partial absorb: the 4-point shield
    // spends everything and the remaining 6 points land.
    session
        .apply_aura(91_512, player, 30_000, 1)
        .expect("apply small absorb shield");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 4);
    assert_eq!(command.damage, 6);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].consumed, 4);
    assert!(command.absorb_consumptions[0].removed);
    assert_eq!(victim_health(), 84);
}

/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop for a player victim
/// (`Unit.cpp:1886-1930`).
///
/// The map-owned swing drains the victim's mana in the same committed phase as
/// the health write, publishes the resulting `SMSG_POWER_UPDATE` through the
/// victim session on delivery, and leaves the shield's amount spent so the mana
/// it can drain is finite.
#[test]
fn legacy_creature_melee_tick_once_drains_player_mana_shield_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};

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
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Victim".to_string(),
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
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
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
            .mutate_world_creature(creature_guid, |creature| {
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
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().get_power(wow_constants::PowerType::Mana)
            })
            .expect("canonical victim")
    };
    let shield_amount = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
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
    session.state = crate::session::SessionState::LoggedIn;
    session.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
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

/// C++ `Unit::CalcAbsorbResist`'s ignore-absorb term (`Unit.cpp:1803-1832`).
///
/// A creature attacker carrying `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` may only
/// push the non-ignored portion of its hit into the victim's school-absorb
/// shield, unless that shield's spell carries
/// `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
#[test]
fn legacy_creature_melee_tick_once_honors_ignore_absorb_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_800);
    let creature_guid = test_creature_guid(91_801);

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
        "Victim".to_string(),
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
            // C++ `GetMaxPositiveAuraModifierByMiscMask` reads the attacker's
            // own `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` effects.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_812_u32, player, 0, 1));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_810_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_811,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            0x01,
        ),
        (
            91_812,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL,
            50,
            0x01,
        ),
        (
            91_813,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
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
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: 0,
                    ..Default::default()
                }],
            },
        );
    }
    // The protected shield carries `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
    let mut attributes = [0_u32; 15];
    attributes[6] = wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE;
    spell_store.insert_spell_misc_attributes_like_cpp(91_813, attributes);
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_810, player, 30_000, 1)
        .expect("apply hit-chance aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
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

    // A 30-point shield without the attribute may only take the half the
    // attacker's 50% modifier leaves: 5 of the 10-point hit.
    session
        .apply_aura(91_811, player, 30_000, 1)
        .expect("apply absorb shield");
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 5);
    assert_eq!(command.damage, 5);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(victim_health(), 95);

    // The shield whose spell carries
    // `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE` ignores the modifier and absorbs the
    // whole hit.
    session
        .apply_aura(91_813, player, 30_000, 1)
        .expect("apply protected absorb shield");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(victim_health(), 95);
}
