//! Scenarios for [`super`], part 12.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

/// 4A.3c dormant rail: map/instance scoped creature visibility refresh.
///
/// C++ anchor: `Player::UpdateVisibilityOf` (Player.cpp:23138+) is the
/// seam that mutates `m_clientGUIDs` and emits CREATE/DESTROY. The global
/// runtime must wake matching sessions to run that seam rather than trying
/// to send raw CREATE bytes through HaveAtClient.
#[test]
fn refresh_visible_world_creatures_routes_by_map_instance_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();

    let in_a = ObjectGuid::create_player(1, 50);
    let (in_a_info, in_a_rx) = make_registry_player_like_cpp(571, 7, Position::ZERO, true);
    registry.register_or_replace(in_a, in_a_info, Default::default());

    let in_b = ObjectGuid::create_player(1, 51);
    let (in_b_info, in_b_rx) =
        make_registry_player_like_cpp(571, 7, Position::new(9000.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(in_b, in_b_info, Default::default());

    let wrong_map = ObjectGuid::create_player(1, 52);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 7, Position::ZERO, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    let wrong_instance = ObjectGuid::create_player(1, 53);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 8, Position::ZERO, true);
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());

    let not_in_world = ObjectGuid::create_player(1, 54);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 7, Position::ZERO, false);
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());

    let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

    assert_eq!(summary.candidates_seen, 5);
    assert_eq!(summary.candidates_queued, 2);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);

    for command in [
        in_a_rx.try_recv().expect("same-map player A refresh"),
        in_b_rx.try_recv().expect("same-map player B refresh"),
    ] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(command.map_id, 571);
        assert_eq!(command.instance_id, 7);
    }
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
}
/// Backpressure on the refresh rail must not block the runtime task.
#[test]
fn refresh_visible_world_creatures_full_channel_counts_send_failed_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 55);

    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    drop(command_rx);

    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "RefreshFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 7;
    info.placement.is_in_world = true;
    registry.register_or_replace(guid, info, Default::default());

    let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.send_failed, 1);
}
#[test]
fn collect_legacy_creature_aggro_candidates_uses_living_in_world_players_like_cpp() {
    let registry = PlayerRegistry::default();
    let in_world = ObjectGuid::create_player(1, 64);
    let not_in_world = ObjectGuid::create_player(1, 65);
    let dead_in_world = ObjectGuid::create_player(1, 66);
    let (mut in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(1.0, 2.0, 3.0, 0.0), true);
    let (not_in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(9.0, 9.0, 9.0, 0.0), false);
    let (mut dead_in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(4.0, 4.0, 4.0, 0.0), true);
    dead_in_world_info.placement.is_alive = false;
    registry.register_or_replace(in_world, in_world_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());
    registry.register_or_replace(dead_in_world, dead_in_world_info, Default::default());
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 0, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        in_world,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        2,
        100,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(in_world)
            .unwrap();
        player.unit_mut().set_combat_reach(1.5);
        player.unit_mut().set_faction(1);
        player.gameplay_state_mut().liquid_status =
            wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP;
        player.gameplay_state_mut().forced_reputation_ranks = vec![(87, 1)];
    }
    assert!(registry.bind_canonical_map_manager(canonical));

    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].player_guid, in_world);
    assert_eq!(candidates[0].map_id, 571);
    assert_eq!(candidates[0].instance_id, 2);
    assert_eq!(candidates[0].position, Position::new(1.0, 2.0, 3.0, 0.0));
    assert!(!candidates[0].player_visibility_represented);
    assert_eq!(candidates[0].player_combat_reach, 1.5);
    assert_eq!(
        candidates[0].player_liquid_status_like_cpp,
        wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP
    );
    assert_eq!(candidates[0].player_level, 1);
    assert_eq!(candidates[0].player_gray_level, 0);
    assert_eq!(candidates[0].player_faction_template_id, 1);
    assert_eq!(
        candidates[0].player_forced_reputation_ranks,
        vec![(87, wow_data::reputation::ReputationRankLikeCpp::Hostile)]
    );
}
#[test]
fn collect_legacy_creature_aggro_candidates_hydrates_canonical_visibility_like_cpp() {
    let registry = PlayerRegistry::default();
    let player_guid = ObjectGuid::create_player(1, 68);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 2, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(&canonical, player_guid, position, 571, 2, 100);
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        *player.unit_mut().world_mut().phase_shift_mut() =
            wow_entities::PhaseShift::from_phases([77]);
        player.unit_mut().set_invisibility_like_cpp(0, 100);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_modifier_like_cpp(
                wow_entities::AppliedAuraRef::new(91_136, player_guid, 0, 0x1),
                wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE,
                6,
            );
        let school_immunity = wow_entities::AppliedAuraRef::new(91_137, player_guid, 1, 0x1);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_effect_like_cpp(
                school_immunity,
                wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                99,
                0x1,
            );
        let confuse = wow_entities::AppliedAuraRef::new(91_138, player_guid, 2, 0x1);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_type_like_cpp(
                confuse,
                wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE,
            );
        let breakable_stun = wow_entities::AppliedAuraRef::new(91_139, player_guid, 3, 0x1);
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        auras.register_applied_aura_type_like_cpp(
            breakable_stun,
            wow_data::spell::aura_types::SPELL_AURA_MOD_STUN,
        );
        auras.register_applied_aura(
            breakable_stun,
            None,
            wow_constants::SpellAuraInterruptFlags::DAMAGE.bits(),
            0,
        );
    }
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));

    let candidates = collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
        &registry,
        Some(&canonical),
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].map_difficulty_id, 2);
    assert!(candidates[0].player_visibility_represented);
    assert!(candidates[0].player_phase_shift.has_phase_like_cpp(77));
    assert_ne!(
        candidates[0].player_visibility_detection,
        wow_entities::UnitVisibilityDetectionStateLikeCpp::default()
    );
    assert_eq!(candidates[0].player_detected_range_aura_mod, 6.0);
    assert_eq!(candidates[0].player_school_immunity_mask, 0x1);
    assert!(candidates[0].player_has_confuse_aura);
    assert!(candidates[0].player_has_breakable_stun_aura);
}
/// The aggro scan's reputation/flag inputs come off the canonical player (#252).
///
/// These four values used to be copied into `PlayerBroadcastInfo` at registration
/// and refreshed on every registry sync. They are read in the collector's existing
/// canonical pass now, which takes the map lock once for the whole batch, so the
/// redirect adds no lock and no nesting.
///
/// C++ anchor: `Creature::CanCreatureAttack`/`Unit::IsValidAttackTarget` consult the
/// inspected `Player`'s own reputation state and unit flags, not a per-session copy.
#[test]
fn collect_legacy_creature_aggro_candidates_reads_reputation_and_flags_from_canonical_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let player_guid = ObjectGuid::create_player(1, 69);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 2, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(&canonical, player_guid, position, 571, 2, 100);
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        player
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::UnitFlags2::IGNORE_REPUTATION);
        player.set_player_flag(
            wow_world::canonical_player_access::PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
        );
        player
            .gameplay_state_mut()
            .reputations
            .push(wow_entities::PlayerReputationRecord {
                faction_id: 72,
                standing: -6000,
                flags: wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP,
                ..Default::default()
            });
        player.set_forced_reputation_rank_like_cpp(87, true);
    }

    let candidates = collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
        &registry,
        Some(&canonical),
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].player_unit_flags2,
        wow_constants::UnitFlags2::IGNORE_REPUTATION.bits()
    );
    assert!(candidates[0].player_is_contested_pvp);
    assert_eq!(
        candidates[0].player_reputation_standings,
        vec![(72, -6_000)]
    );
    assert_eq!(
        candidates[0].player_reputation_state_flags,
        vec![(72, wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP)]
    );
    assert_eq!(candidates[0].player_forced_reputation_faction_ids, vec![87]);
}
/// Negative branch: a directory entry without its canonical owner is unknown and
/// cannot become an aggro candidate. The far-teleport window must not manufacture
/// zero/default gameplay values for a player that is temporarily on no map.
#[test]
fn collect_legacy_creature_aggro_candidates_skips_unknown_canonical_owner_like_cpp() {
    let registry = PlayerRegistry::default();
    let player_guid = ObjectGuid::create_player(1, 70);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);

    assert!(candidates.is_empty());
}
#[test]
fn creature_attack_start_delivery_routes_only_to_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 66);
    let other = ObjectGuid::create_player(1, 67);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_060);
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    registry.register_or_replace(other, other_info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: false,
        },
    ];
    let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 1);
    let SessionCommand::CreatureAttackStartLikeCpp(command) =
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .expect("victim receives attack-start")
    else {
        panic!("expected CreatureAttackStartLikeCpp command");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert!(
        other_rx.try_recv().is_err(),
        "non-victim session is untouched"
    );
}
#[test]
fn creature_attack_start_commits_player_victim_without_a_session_recipient_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_068);
    let victim = ObjectGuid::create_player(1, 90_069);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, Position::ZERO, 571, 4, 100);
    add_canonical_test_player_on_map_like_cpp(&canonical, victim, Position::ZERO, 571, 4, 100);
    let commands = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: false,
        },
    ];

    let outcomes = apply_canonical_creature_attack_starts_like_cpp(&commands, Some(&canonical));
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].is_applied());

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 4).unwrap().map();
    let attacker_has_combat = map
        .with_creature_like_cpp(attacker, |attacker| {
            attacker
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(victim)
        })
        .unwrap();
    let victim = map.get_typed_player(victim).unwrap();
    assert!(attacker_has_combat);
    assert!(victim.unit().subsystems().combat.has_combat());
    assert!(victim.unit().has_attacker_like_cpp(attacker));
}
#[test]
fn rejected_map_attack_filters_both_session_and_visual_delivery_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_070);
    let victim = ObjectGuid::create_player(1, 90_071);
    let commands = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: true,
        },
    ];
    let outcomes = apply_canonical_creature_attack_starts_like_cpp(&commands, Some(&canonical));
    assert_eq!(
        outcomes[0].status,
        wow_map::MapCommandStatusLikeCpp::MissingMap
    );

    let attack_bytes = wow_packet::packets::combat::AttackStart { attacker, victim }.to_bytes();
    let recipients = wow_world::map_manager::RecipientRule::NearbyVisibleDurable {
        source_guid: attacker,
        map_id: 571,
        instance_id: 4,
        source_position: Position::ZERO,
        range: 100.0,
        required_3d: false,
    };
    let mut plan = wow_world::map_manager::RuntimePlan {
        events: vec![
            wow_world::map_manager::RuntimeEvent {
                source_guid: attacker,
                recipients: recipients.clone(),
                packet_bytes: attack_bytes,
            },
            wow_world::map_manager::RuntimeEvent {
                source_guid: attacker,
                recipients,
                packet_bytes: vec![0xAA, 0x55],
            },
        ],
    };

    retain_committed_creature_combat_events_like_cpp(&mut plan, &commands, &outcomes, &[], &[]);

    assert_eq!(plan.events.len(), 1);
    assert_eq!(plan.events[0].packet_bytes, vec![0xAA, 0x55]);
}
#[test]
fn creature_assistance_start_establishes_canonical_combat_for_both_creatures_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_070);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9002, 90_071);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, Position::ZERO, 571, 4, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, Position::ZERO, 571, 4, 100);
    let commands = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: true,
        },
    ];

    let outcomes = apply_canonical_creature_attack_starts_like_cpp(&commands, Some(&canonical));
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].is_applied());
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 4).unwrap().map();
    let attacker_creature = map.with_creature_like_cpp(attacker, Clone::clone).unwrap();
    let victim_creature = map.with_creature_like_cpp(victim, Clone::clone).unwrap();
    let attacker_unit = attacker_creature.unit();
    let victim_unit = victim_creature.unit();
    assert!(attacker_unit.subsystems().combat.is_in_combat_with(victim));
    assert!(victim_unit.subsystems().combat.is_in_combat_with(attacker));
    assert_eq!(
        attacker_unit.subsystems().combat.threat_value(victim),
        Some(0.0),
        "C++ EngageWithTarget creates the assistant's zero-threat forward reference"
    );
    assert!(
        victim_unit
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&attacker),
        "the victim must carry the reciprocal reference used by helpful-threat fanout"
    );
}
#[test]
fn creature_assistance_stop_purges_canonical_combat_for_both_creatures_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_072);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9002, 90_073);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, Position::ZERO, 571, 4, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, Position::ZERO, 571, 4, 100);
    let starts = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: true,
        },
    ];
    let stops = [
        wow_world::session::mailbox::CreatureAttackStopLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 4,
        },
    ];

    let start_outcomes = apply_canonical_creature_attack_starts_like_cpp(&starts, Some(&canonical));
    let stop_outcomes = apply_canonical_creature_attack_stops_like_cpp(&stops, Some(&canonical));
    assert_eq!(start_outcomes.len(), 1);
    assert!(start_outcomes[0].is_applied());
    assert_eq!(stop_outcomes.len(), 1);
    assert!(stop_outcomes[0].is_applied());
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 4).unwrap().map();
    let attacker_creature = map.with_creature_like_cpp(attacker, Clone::clone).unwrap();
    let victim_creature = map.with_creature_like_cpp(victim, Clone::clone).unwrap();
    let attacker_unit = attacker_creature.unit();
    let victim_unit = victim_creature.unit();
    assert!(!attacker_unit.subsystems().combat.is_in_combat_with(victim));
    assert!(!victim_unit.subsystems().combat.is_in_combat_with(attacker));
    assert!(
        !victim_unit
            .subsystems()
            .combat
            .attackers
            .contains(&attacker)
    );
}
#[test]
fn creature_attack_start_delivery_filters_registry_state_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_061);
    let wrong_map = ObjectGuid::create_player(1, 68);
    let wrong_instance = ObjectGuid::create_player(1, 69);
    let not_in_world = ObjectGuid::create_player(1, 70);
    let missing = ObjectGuid::create_player(1, 71);
    let dead = ObjectGuid::create_player(1, 72);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 9, Position::ZERO, true);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    let (mut dead_info, dead_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    dead_info.placement.is_alive = false;
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());
    registry.register_or_replace(dead, dead_info, Default::default());

    let make_command =
        |victim_guid| wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 0,
            packet_already_broadcast: false,
        };
    let commands = vec![
        make_command(wrong_map),
        make_command(wrong_instance),
        make_command(not_in_world),
        make_command(missing),
        make_command(dead),
    ];
    let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 5);
    assert_eq!(summary.candidates_seen, 4);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert_eq!(summary.candidates_skipped_dead, 1);
    assert_eq!(summary.candidates_skipped_missing_victim, 1);
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
    assert!(dead_rx.try_recv().is_err());
}
#[test]
fn creature_attack_start_delivery_uses_durable_rail_when_general_queue_is_full_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 73);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_062);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx.clone(), "AggroFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    info.placement.is_alive = true;
    registry.register_or_replace(victim, info, Default::default());
    let command = wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    command_tx
        .send(SessionCommand::CreatureAttackStartLikeCpp(command.clone()))
        .unwrap();

    let summary = deliver_creature_attack_start_commands_like_cpp(&[command], &registry);
    assert_eq!(summary.candidates_queued, 1);
    assert_eq!(summary.send_failed, 0);
    assert_eq!(command_rx.len(), 1, "bounded general queue remains full");
    assert!(matches!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .unwrap(),
        SessionCommand::CreatureAttackStartLikeCpp(_)
    ));
}
/// 4C.4 bridge coverage: the combined global runtime body can perform the
/// C++ `CreatureAI::MoveInLineOfSight`-style aggro transition once from the
/// map owner and deliver both the authoritative session transition and its
/// visible attack-start packet to the victim session in FIFO order.
#[test]
fn legacy_creature_runtime_bridge_delivers_aggro_start_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_player(1, 93_001);
    let victim_position = Position::new(10.5, 10.5, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, victim, victim_position, 0, 0, 100);

    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 93_002);
    let attacker_position = Position::new(10.0, 10.0, 0.0, 0.0);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, attacker_position, 0, 0, 100);
    let mut creature = wow_world::map_manager::WorldCreature::new(
        attacker,
        9001,
        attacker_position,
        25,
        2,
        3,
        5,
        5.0,
        100,
        14,
        0,
        0,
    );
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = u64::MAX;
        ai.swing_timer_ms = u64::MAX;
    }

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(attacker_position.x),
            wow_world::map_manager::world_to_grid_y(attacker_position.y),
            creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::default();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let (mut victim_info, victim_rx) = make_registry_player_like_cpp(0, 0, victim_position, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 93_003);
    let (wrong_map_info, wrong_map_rx) = make_registry_player_like_cpp(1, 0, victim_position, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    add_canonical_test_player_on_map_like_cpp(&canonical, wrong_map, victim_position, 1, 0, 100);
    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let aggro_config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 14,
                    faction: 72,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [930, 0, 0, 0, 0, 0, 0, 0],
                    friend: [0; 8],
                },
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 1,
                    faction: 930,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [0; 8],
                    friend: [0; 8],
                },
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    };
    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);
    assert_eq!(candidates.len(), 2, "{candidates:?}");
    let outcome = run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
        &legacy,
        Some(&canonical),
        &legacy_runtime_world_map_store_like_cpp(),
        &mmap_config,
        None,
        aggro_config,
        10,
        std::time::Instant::now(),
        &registry,
        None,
        None,
        None,
        &Arc::new(Mutex::new(Default::default())),
    );

    assert!(!outcome.aggro.skipped_owner_not_global);
    assert_eq!(outcome.aggro.maps_seen, 1);
    assert_eq!(outcome.aggro.creatures_seen, 1);
    assert_eq!(outcome.aggro.candidates_seen, 1);
    assert_eq!(outcome.aggro.aggro_starts, 1);
    assert_eq!(outcome.aggro.commands.len(), 1);
    assert_eq!(outcome.aggro_delivery.commands_seen, 1);
    assert_eq!(outcome.aggro_delivery.candidates_seen, 1);
    assert_eq!(outcome.aggro_delivery.candidates_queued, 1);
    assert_eq!(outcome.aggro_delivery.candidates_skipped_wrong_map, 0);
    assert_eq!(outcome.aggro_plan_delivery.events_seen, 1);
    assert_eq!(outcome.aggro_plan_delivery.candidates_seen, 2);
    assert_eq!(outcome.aggro_plan_delivery.candidates_queued, 1);
    assert_eq!(outcome.aggro_plan_delivery.candidates_skipped_distance, 1);
    assert_eq!(outcome.movement.movement_packets, 0);
    assert_eq!(outcome.melee.swings_ready, 0);

    let commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    let [
        SessionCommand::CreatureAttackStartLikeCpp(command),
        SessionCommand::SendIfVisibleLikeCpp(visual),
    ] = commands.as_slice()
    else {
        panic!("expected authoritative then visual attack-start commands: {commands:?}");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    assert_eq!(visual.source_guid, attacker);
    assert_eq!(visual.map_id, 0);
    assert_eq!(visual.instance_id, 0);
    assert_eq!(
        u16::from_le_bytes([visual.packet_bytes[0], visual.packet_bytes[1]]),
        wow_constants::ServerOpcodes::AttackStart as u16
    );
    assert!(victim_rx.try_recv().is_err());
    assert!(wrong_map_rx.try_recv().is_err());

    let combat_target = {
        let guard = legacy.read().unwrap();
        guard
            .find_creature(0, 0, attacker)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, Some(victim));
}
/// 4C.3 dormant rail: map-owned creature melee results route to exactly
/// the victim session. C++ anchor: `Unit::AttackerStateUpdate` resolves a
/// single melee hit for one victim, then `Unit::DealDamage` mutates health.
#[test]
fn creature_melee_damage_delivery_routes_only_to_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 56);
    let other = ObjectGuid::create_player(1, 57);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_056);
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    registry.register_or_replace(other, other_info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 3,
            damage: 17,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 83,
            victim_health_state_revision_after: 7,
        },
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 1);
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) =
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .expect("victim receives melee command")
    else {
        panic!("expected ApplyCreatureMeleeDamageLikeCpp command");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert_eq!(command.victim_health_after, 83);
    assert_eq!(command.victim_health_state_revision_after, 7);
    assert!(
        other_rx.try_recv().is_err(),
        "non-victim session is untouched"
    );
}
