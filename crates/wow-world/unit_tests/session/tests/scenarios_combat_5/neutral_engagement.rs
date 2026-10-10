//! Session scenarios for a neutral creature attacked by a player (#1344).
//!
//! C++ anchors: `Creature::_IsTargetAcceptable` (`Creature.cpp:2624-2649`)
//! accepts `IsEngagedBy(target) || IsHostileTo(target)` and backs
//! `ThreatReference::ShouldBeOffline` (`ThreatManager.cpp:99-108`), while the
//! detection scan (`Creature::CanStartAttack`) only reaches new, hostile
//! targets. `Unit::AttackerStateUpdate` → `AtTargetAttacked` →
//! `EngageWithTarget` engages without a victim; `CreatureAI::UpdateVictim` →
//! `AttackStart` → `Unit::Attack` then sends `SMSG_AI_REACTION` (hostile) and
//! `SMSG_ATTACK_START` (`Unit.cpp:5645-5743`).

use super::*;

/// Creature faction template 14 shaped like 3.4.3 `FactionTemplate` 7, the
/// template of the paired-run creature entry 15271: no groups, no enemies, no
/// friends, no flags, on a faction without reputation (neutral to all).
fn neutral_creature_config_like_cpp() -> LegacyCreatureAggroConfigLikeCpp {
    legacy_aggro_relation_config_like_cpp(
        faction_template_entry(14, 7, 0, 0, 0),
        faction_template_entry(1, 930, 0, 0, 0),
        FactionEntry::for_test_like_cpp(7, -1),
    )
}

fn server_opcode_like_cpp(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn creature_threat_online_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
) -> (Option<ObjectGuid>, bool) {
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    (
        creature.creature.ai_ownership().combat_target,
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(victim_guid)
            .is_some_and(wow_entities::ThreatReferenceState::is_online),
    )
}

#[test]
fn neutral_creature_engaged_by_player_stays_engaged_and_attacks_back_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_344);
    let victim_guid = ObjectGuid::create_player(1, 91_345);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            // The player's first swing (damage 4): engage, then damage threat.
            creature.creature.engage_with_target_like_cpp(victim_guid);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(victim_guid, 4.0);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        victim_guid,
        Position::new(11.0, 10.0, 0.0, 0.0),
    )];
    assert_eq!(
        creature_threat_online_like_cpp(&manager, creature_guid, victim_guid),
        (None, true),
        "the swing engages the creature without choosing its victim"
    );

    for tick in 0..3 {
        let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
            &manager,
            &candidates,
            neutral_creature_config_like_cpp(),
        );
        assert_eq!(outcome.evades_started, 0, "tick {tick}: {outcome:?}");
        assert!(outcome.stop_commands.is_empty(), "tick {tick}");
        assert!(
            outcome
                .plan
                .events
                .iter()
                .all(|event| server_opcode_like_cpp(&event.packet_bytes)
                    != ServerOpcodes::AttackStop as u16),
            "tick {tick}: no SMSG_ATTACK_STOP while the attacker stays engaged"
        );
        assert_eq!(
            creature_threat_online_like_cpp(&manager, creature_guid, victim_guid),
            (Some(victim_guid), true),
            "tick {tick}: the engaged neutral target keeps an online threat ref"
        );
        assert_eq!(outcome.aggro_starts, 0, "tick {tick}");
        assert_eq!(
            outcome.victim_switches,
            usize::from(tick == 0),
            "tick {tick}"
        );
    }

    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &neutral_creature_config_like_cpp(),
    );
    assert_eq!(
        melee.swings_ready, 1,
        "the creature attacks back: {melee:?}"
    );
    assert_eq!(melee.canonical_hits, 1);
    assert_eq!(melee.commands.len(), 1);
    assert_eq!(melee.commands[0].attacker_guid, creature_guid);
    assert_eq!(melee.commands[0].victim_guid, victim_guid);
}

#[test]
fn neutral_creature_does_not_start_aggro_on_passer_by_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_346);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let passer_by = ObjectGuid::create_player(1, 91_347);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        passer_by,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    // A neutral-to-all creature selects `ReactorAI`, whose `MoveInLineOfSight`
    // never starts aggro.
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        neutral_creature_config_like_cpp(),
    );
    assert_eq!(outcome.ai_los_suppressed, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    assert!(outcome.plan.events.is_empty());

    // Even with the base `MoveInLineOfSight` (`AggressorAI`), the start scan
    // stays hostile-only: the engaged clause never admits a new target.
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("AggressorAI", String::new());
        })
        .unwrap();
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        neutral_creature_config_like_cpp(),
    );
    assert_eq!(outcome.hostility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    assert!(outcome.plan.events.is_empty());
    assert_eq!(
        creature_threat_online_like_cpp(&manager, creature_guid, passer_by),
        (None, false)
    );
}

#[tokio::test]
async fn attack_swing_engages_and_creature_reacts_with_ai_reaction_then_attack_start_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::ServerPacket;

    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_348);
    let creature_guid = test_creature_guid(91_349);
    canonical.lock().unwrap().create_world_map(0, 0);
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
        "Attacker".to_string(),
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical attacking Player map");
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&creature_guid);
    session.handle_attack_swing(pkt).await;
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AttackStart]
    );
    assert_eq!(
        creature_threat_online_like_cpp(&manager, creature_guid, player),
        (None, true),
        "Player::Attack publishes nothing for the creature and presets no victim"
    );

    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(11.0, 10.0, 0.0, 0.0),
    )];
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        neutral_creature_config_like_cpp(),
    );

    let packets: Vec<_> = outcome
        .plan
        .events
        .iter()
        .map(|event| event.packet_bytes.clone())
        .collect();
    assert_eq!(
        packets,
        vec![
            wow_packet::packets::combat::AIReaction {
                unit_guid: creature_guid,
                reaction: wow_constants::creature::AiReaction::Hostile,
            }
            .to_bytes(),
            wow_packet::packets::combat::AttackStart {
                attacker: creature_guid,
                victim: player,
            }
            .to_bytes(),
        ],
        "C++ Unit::Attack: SendAIReaction(HOSTILE) before SendMeleeAttackStart"
    );
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].attacker_guid, creature_guid);
    assert_eq!(outcome.commands[0].victim_guid, player);
    assert_eq!(outcome.commands[0].previous_victim_guid, None);
    assert_eq!(outcome.evades_started, 0);
}
