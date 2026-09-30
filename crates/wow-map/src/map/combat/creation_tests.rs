use super::{Creature, Map, ObjectGuid, Player, Unit};
use wow_constants::{DeathState, UnitState};
use wow_core::{Position, guid::HighGuid};
use wow_entities::{MapObjectRecord, PVP_COMBAT_TIMEOUT_MS, PhaseShift};

fn bind_unit(unit: &mut Unit, guid: ObjectGuid) {
    unit.world_mut().object_mut().create(guid);
    unit.world_mut().set_map(571, 7).unwrap();
    unit.world_mut().relocate(Position::xyz(10.0, 20.0, 30.0));
    unit.world_mut().object_mut().add_to_world();
    unit.set_death_state(DeathState::Alive);
    unit.set_max_health(100);
    unit.set_health(100);
}

fn pair(player_victim: bool) -> (Map, ObjectGuid, ObjectGuid) {
    let mut map = Map::new(571, 7, 1, 60_000);
    let attacker_guid = ObjectGuid::create_player(1, 1);
    let mut attacker = Player::new(Some(7), false);
    bind_unit(attacker.unit_mut(), attacker_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(attacker).unwrap()).unwrap();

    let victim_guid = if player_victim {
        let guid = ObjectGuid::create_player(1, 2);
        let mut victim = Player::new(Some(7), false);
        bind_unit(victim.unit_mut(), guid);
        map.insert_map_object_record(MapObjectRecord::new_player(victim).unwrap()).unwrap();
        guid
    } else {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 2);
        let mut victim = Creature::new(false);
        bind_unit(victim.unit_mut(), guid);
        map.insert_map_object_record(MapObjectRecord::new_creature(victim).unwrap()).unwrap();
        guid
    };
    (map, attacker_guid, victim_guid)
}

fn unit(map: &Map, guid: ObjectGuid) -> &Unit {
    if let Some(player) = map.get_typed_player(guid) {
        player.unit()
    } else {
        map.get_typed_creature(guid).expect("fixture creature").unit()
    }
}

fn unit_mut(map: &mut Map, guid: ObjectGuid) -> &mut Unit {
    if map.get_typed_player(guid).is_some() {
        map.get_typed_player_mut(guid).unwrap().unit_mut()
    } else {
        map.get_typed_creature_mut(guid).expect("fixture creature").unit_mut()
    }
}

fn assert_rejected_unchanged(
    map: &mut Map,
    attacker: ObjectGuid,
    victim: ObjectGuid,
    relation_represented: bool,
    attacker_friendly: bool,
    victim_friendly: bool,
) {
    let attacker_before = unit(map, attacker).subsystems().combat.clone();
    let victim_before = unit(map, victim).subsystems().combat.clone();
    assert!(!map.begin_player_combat_ref(
        attacker, victim, relation_represented, attacker_friendly, victim_friendly,
    ));
    assert_eq!(unit(map, attacker).subsystems().combat, attacker_before);
    assert_eq!(unit(map, victim).subsystems().combat, victim_before);
}

#[test]
fn player_and_creature_victims_receive_reciprocal_refs_with_original_classification() {
    for player_victim in [true, false] {
        let (mut map, attacker, victim) = pair(player_victim);
        assert!(map.begin_player_combat_ref(attacker, victim, true, false, false));
        for (owner, target) in [(attacker, victim), (victim, attacker)] {
            let combat = &unit(&map, owner).subsystems().combat;
            assert!(combat.has_combat());
            assert!(combat.is_in_combat_with(target));
            let reference = if player_victim {
                assert!(combat.pve_refs.is_empty());
                assert_eq!(combat.pvp_refs.len(), 1);
                combat.pvp_refs.get(&target).unwrap()
            } else {
                assert!(combat.pvp_refs.is_empty());
                assert_eq!(combat.pve_refs.len(), 1);
                combat.pve_refs.get(&target).unwrap()
            };
            assert_eq!(reference.pvp, player_victim);
            assert!(!reference.suppressed_for_owner);
            assert_eq!(reference.timeout_ms, player_victim.then_some(PVP_COMBAT_TIMEOUT_MS));
        }
    }
}

#[test]
fn missing_units_self_target_and_creature_attackers_fail_before_writes() {
    let (mut map, attacker, victim) = pair(false);
    let missing = ObjectGuid::create_player(1, 99);
    assert!(!map.begin_player_combat_ref(missing, victim, false, false, false));
    assert!(!map.begin_player_combat_ref(attacker, missing, false, false, false));
    assert_rejected_unchanged(&mut map, attacker, attacker, false, false, false);
    assert_rejected_unchanged(&mut map, victim, attacker, false, false, false);
    assert!(!unit(&map, attacker).subsystems().combat.has_combat());
    assert!(!unit(&map, victim).subsystems().combat.has_combat());
}

#[test]
fn unworld_dead_evading_flying_disallowed_and_gm_units_reject_without_mutation() {
    for player_victim in [true, false] {
        for attacker_side in [true, false] {
            for gate in ["unworld", "dead", "evade", "flight", "disallowed", "gm"] {
                if gate == "gm" && !attacker_side && !player_victim {
                    continue;
                }
                let (mut map, attacker, victim) = pair(player_victim);
                let selected = if attacker_side { attacker } else { victim };
                match gate {
                    "unworld" => unit_mut(&mut map, selected).world_mut().object_mut().remove_from_world(),
                    "dead" => unit_mut(&mut map, selected).set_death_state(DeathState::Corpse),
                    "evade" => unit_mut(&mut map, selected).add_unit_state(UnitState::EVADE.bits()),
                    "flight" => unit_mut(&mut map, selected).add_unit_state(UnitState::IN_FLIGHT.bits()),
                    "disallowed" => unit_mut(&mut map, selected).subsystems_mut().combat.combat_disallowed = true,
                    "gm" => map.get_typed_player_mut(selected).unwrap().set_game_master_like_cpp(true),
                    _ => unreachable!(),
                }
                assert_rejected_unchanged(&mut map, attacker, victim, false, false, false);
            }
        }
    }
}

#[test]
fn different_map_instance_or_phase_rejects_without_writing_either_owner() {
    for player_victim in [true, false] {
        for attacker_side in [true, false] {
            for gate in ["map", "instance", "phase"] {
                let (mut map, attacker, victim) = pair(player_victim);
                let selected = if attacker_side { attacker } else { victim };
                if gate == "phase" {
                    *unit_mut(&mut map, attacker).world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
                    *unit_mut(&mut map, victim).world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
                    *unit_mut(&mut map, selected).world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
                } else {
                    let world = unit_mut(&mut map, selected).world_mut();
                    world.object_mut().remove_from_world();
                    world.reset_map().unwrap();
                    let (map_id, instance_id) = if gate == "map" { (572, 7) } else { (571, 8) };
                    world.set_map(map_id, instance_id).unwrap();
                    world.object_mut().add_to_world();
                }
                assert_rejected_unchanged(&mut map, attacker, victim, false, false, false);
            }
        }
    }
}

#[test]
fn overlapping_phases_allow_creation_for_both_victim_kinds() {
    for player_victim in [true, false] {
        let (mut map, attacker, victim) = pair(player_victim);
        *unit_mut(&mut map, attacker).world_mut().phase_shift_mut() = PhaseShift::from_phases([10, 20]);
        *unit_mut(&mut map, victim).world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
        assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
        assert!(unit(&map, attacker).subsystems().combat.is_in_combat_with(victim));
        assert!(unit(&map, victim).subsystems().combat.is_in_combat_with(attacker));
    }
}

#[test]
fn either_friendly_direction_rejects_only_when_relation_is_represented() {
    for player_victim in [true, false] {
        for (attacker_friendly, victim_friendly) in [(true, false), (false, true), (true, true)] {
            let (mut map, attacker, victim) = pair(player_victim);
            assert_rejected_unchanged(&mut map, attacker, victim, true, attacker_friendly, victim_friendly);
            assert!(map.begin_player_combat_ref(attacker, victim, false, attacker_friendly, victim_friendly));
            assert!(unit(&map, attacker).subsystems().combat.is_in_combat_with(victim));
            assert!(unit(&map, victim).subsystems().combat.is_in_combat_with(attacker));
        }
    }
}

#[test]
fn player_controlled_creature_keeps_pve_and_unresolved_charmer_gm_boundary() {
    let (mut map, attacker, victim) = pair(false);
    let gm_guid = ObjectGuid::create_player(1, 3);
    let mut gm = Player::new(Some(7), false);
    bind_unit(gm.unit_mut(), gm_guid);
    gm.set_game_master_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_player(gm).unwrap()).unwrap();
    unit_mut(&mut map, victim).subsystems_mut().control.set_charmer(gm_guid, true);

    assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
    for (owner, target) in [(attacker, victim), (victim, attacker)] {
        let combat = &unit(&map, owner).subsystems().combat;
        assert!(combat.pvp_refs.is_empty());
        assert!(combat.pve_refs.contains_key(&target));
    }
    assert!(!unit(&map, gm_guid).subsystems().combat.has_combat());
}

#[test]
fn successful_refresh_unsuppresses_both_refs_and_renews_pvp_timeout() {
    for player_victim in [true, false] {
        let (mut map, attacker, victim) = pair(player_victim);
        assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
        for (owner, target) in [(attacker, victim), (victim, attacker)] {
            let combat = &mut unit_mut(&mut map, owner).subsystems_mut().combat;
            let reference = if player_victim {
                combat.pvp_refs.get_mut(&target).unwrap()
            } else {
                combat.pve_refs.get_mut(&target).unwrap()
            };
            reference.suppress_for_owner();
            if player_victim {
                reference.timeout_ms = Some(1);
            }
        }
        assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
        for (owner, target) in [(attacker, victim), (victim, attacker)] {
            let combat = &unit(&map, owner).subsystems().combat;
            let reference = if player_victim {
                assert_eq!(combat.pvp_refs.len(), 1);
                combat.pvp_refs.get(&target).unwrap()
            } else {
                assert_eq!(combat.pve_refs.len(), 1);
                combat.pve_refs.get(&target).unwrap()
            };
            assert!(!reference.suppressed_for_owner);
            assert_eq!(reference.timeout_ms, player_victim.then_some(PVP_COMBAT_TIMEOUT_MS));
        }
    }
}

#[test]
fn failed_admission_does_not_refresh_existing_references() {
    for player_victim in [true, false] {
        let (mut map, attacker, victim) = pair(player_victim);
        assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
        for (owner, target) in [(attacker, victim), (victim, attacker)] {
            let combat = &mut unit_mut(&mut map, owner).subsystems_mut().combat;
            let reference = if player_victim {
                combat.pvp_refs.get_mut(&target).unwrap()
            } else {
                combat.pve_refs.get_mut(&target).unwrap()
            };
            reference.suppress_for_owner();
            if player_victim {
                reference.timeout_ms = Some(1);
            }
        }
        unit_mut(&mut map, victim).set_death_state(DeathState::Corpse);
        assert_rejected_unchanged(&mut map, attacker, victim, false, false, false);
    }
}

#[test]
fn existing_reference_classification_is_preserved_during_reciprocal_creation() {
    let (mut map, attacker, victim) = pair(true);
    unit_mut(&mut map, attacker).subsystems_mut().combat.set_in_combat_with(victim, false, false);

    assert!(map.begin_player_combat_ref(attacker, victim, false, false, false));
    let attacker_combat = &unit(&map, attacker).subsystems().combat;
    assert!(attacker_combat.pve_refs.contains_key(&victim));
    assert!(attacker_combat.pvp_refs.is_empty());
    let victim_combat = &unit(&map, victim).subsystems().combat;
    assert!(victim_combat.pvp_refs.contains_key(&attacker));
    assert!(victim_combat.pve_refs.is_empty());
}
