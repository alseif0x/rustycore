use super::Map;
use wow_constants::{HighGuid, UnitState};
use wow_core::ObjectGuid;
use wow_entities::{
    CombatReferenceState, Creature, CurrentSpellRef, CurrentSpellSlot, MapObjectRecord, Player,
    Unit,
};

fn bind_unit(unit: &mut Unit, guid: ObjectGuid) {
    unit.world_mut().object_mut().create(guid);
    unit.world_mut().set_map(571, 7).unwrap();
    unit.world_mut().object_mut().add_to_world();
}

fn fixture() -> (Map, ObjectGuid, ObjectGuid, ObjectGuid, ObjectGuid) {
    let mut map = Map::new(571, 7, 1, 60_000);
    let owner = ObjectGuid::create_player(1, 1);
    let peer = ObjectGuid::create_player(1, 2);
    let foreign = ObjectGuid::create_player(1, 3);
    for guid in [owner, peer, foreign] {
        let mut player = Player::new(Some(7), false);
        bind_unit(player.unit_mut(), guid);
        map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 100, 4);
    let mut entity = Creature::new(false);
    bind_unit(entity.unit_mut(), creature);
    map.insert_map_object_record(MapObjectRecord::new_creature(entity).unwrap())
        .unwrap();
    (map, owner, peer, creature, foreign)
}

fn unit(map: &Map, guid: ObjectGuid) -> &Unit {
    if let Some(player) = map.get_typed_player(guid) {
        player.unit()
    } else {
        map.get_typed_creature(guid)
            .expect("fixture creature")
            .unit()
    }
}

fn unit_mut(map: &mut Map, guid: ObjectGuid) -> &mut Unit {
    if map.get_typed_player(guid).is_some() {
        map.get_typed_player_mut(guid).unwrap().unit_mut()
    } else {
        map.get_typed_creature_mut(guid)
            .expect("fixture creature")
            .unit_mut()
    }
}

fn start_melee(unit: &mut Unit, target: ObjectGuid) {
    let guid = unit.world().object().guid();
    unit.set_attacking(Some(target));
    unit.set_target(target);
    unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
    unit.subsystems_mut().spells.set_current_spell(
        CurrentSpellSlot::Melee,
        CurrentSpellRef::new(701, Some(guid), None).with_cast_time_ms(1_000),
    );
}

#[test]
fn cleanup_returns_sorted_unique_original_union_including_missing_owners() {
    let (mut map, owner, peer, creature, _) = fixture();
    let missing = ObjectGuid::create_player(1, 99);
    let combat = &mut unit_mut(&mut map, owner).subsystems_mut().combat;
    combat
        .pve_refs
        .insert(creature, CombatReferenceState::pve());
    combat.pve_refs.insert(peer, CombatReferenceState::pve());
    combat.pvp_refs.insert(peer, CombatReferenceState::pvp());
    combat.add_attacker(creature);
    combat.add_attacker(missing);

    let result = map.clear_player_combat(owner).unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&peer));
    assert!(result.contains(&creature));
    assert!(result.contains(&missing));
    assert!(result.windows(2).all(|pair| pair[0] < pair[1]));
    let combat = &unit(&map, owner).subsystems().combat;
    assert!(combat.pve_refs.is_empty());
    assert!(combat.pvp_refs.is_empty());
    assert!(combat.attackers.is_empty());
}

#[test]
fn cleanup_stops_player_and_creature_peers_and_preserves_unrelated_relationships() {
    let (mut map, owner, peer, creature, foreign) = fixture();
    let combat = &mut unit_mut(&mut map, owner).subsystems_mut().combat;
    combat.pvp_refs.insert(peer, CombatReferenceState::pvp());
    combat
        .pve_refs
        .insert(creature, CombatReferenceState::pve());
    for guid in [peer, creature] {
        let unit = unit_mut(&mut map, guid);
        start_melee(unit, owner);
        let combat = &mut unit.subsystems_mut().combat;
        if guid == peer {
            combat.pvp_refs.insert(owner, CombatReferenceState::pvp());
        } else {
            combat.pve_refs.insert(owner, CombatReferenceState::pve());
        }
        combat.pve_refs.insert(foreign, CombatReferenceState::pve());
        combat.add_attacker(owner);
        combat.add_attacker(foreign);
    }

    let owners = map.clear_player_combat(owner).unwrap();
    assert_eq!(owners.len(), 2);
    assert!(owners.contains(&peer));
    assert!(owners.contains(&creature));
    for guid in [peer, creature] {
        let unit = unit(&map, guid);
        assert_eq!(unit.attacking(), None);
        assert_eq!(unit.data().target, ObjectGuid::EMPTY);
        assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
        assert!(unit.current_spell(CurrentSpellSlot::Melee).is_none());
        let combat = &unit.subsystems().combat;
        assert!(!combat.pve_refs.contains_key(&owner));
        assert!(!combat.pvp_refs.contains_key(&owner));
        assert!(!combat.attackers.contains(&owner));
        assert_eq!(
            combat.pve_refs.get(&foreign),
            Some(&CombatReferenceState::pve())
        );
        assert!(combat.attackers.contains(&foreign));
    }
}

#[test]
fn foreign_victim_keeps_peer_attack_target_state_and_melee_spell() {
    let (mut map, owner, peer, creature, foreign) = fixture();
    for guid in [peer, creature] {
        unit_mut(&mut map, owner).add_attacker_like_cpp(guid);
        let unit = unit_mut(&mut map, guid);
        start_melee(unit, foreign);
        unit.subsystems_mut()
            .combat
            .pve_refs
            .insert(owner, CombatReferenceState::pve());
        unit.add_attacker_like_cpp(owner);
    }

    assert_eq!(map.clear_player_combat(owner).unwrap().len(), 2);
    for guid in [peer, creature] {
        let unit = unit(&map, guid);
        assert_eq!(unit.attacking(), Some(foreign));
        assert_eq!(unit.data().target, foreign);
        assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
        assert_eq!(
            unit.current_spell(CurrentSpellSlot::Melee)
                .unwrap()
                .spell_id,
            701
        );
        assert!(!unit.subsystems().combat.pve_refs.contains_key(&owner));
        assert!(!unit.has_attacker_like_cpp(owner));
    }
}

#[test]
fn absent_or_creature_owner_returns_none_without_peer_writes() {
    let (mut map, owner, peer, creature, _) = fixture();
    let missing = ObjectGuid::create_player(1, 99);
    for guid in [owner, peer, creature] {
        let unit = unit_mut(&mut map, guid);
        start_melee(unit, missing);
        unit.subsystems_mut()
            .combat
            .pve_refs
            .insert(missing, CombatReferenceState::pve());
        unit.add_attacker_like_cpp(missing);
    }
    for absent in [missing, creature] {
        let before: Vec<_> = [owner, peer, creature]
            .into_iter()
            .map(|guid| (guid, unit(&map, guid).subsystems().combat.clone()))
            .collect();
        assert_eq!(map.clear_player_combat(absent), None);
        for (guid, combat) in before {
            let unit = unit(&map, guid);
            assert_eq!(unit.subsystems().combat, combat);
            assert_eq!(unit.data().target, missing);
            assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
            assert!(unit.current_spell(CurrentSpellSlot::Melee).is_some());
        }
    }
}

#[test]
fn existing_player_with_no_relationships_returns_some_empty_set() {
    let (mut map, owner, peer, creature, _) = fixture();
    let peer_before = unit(&map, peer).subsystems().combat.clone();
    let creature_before = unit(&map, creature).subsystems().combat.clone();
    unit_mut(&mut map, owner)
        .subsystems_mut()
        .combat
        .combat_disallowed = true;

    assert_eq!(map.clear_player_combat(owner), Some(Vec::new()));
    assert!(unit(&map, owner).subsystems().combat.combat_disallowed);
    assert_eq!(unit(&map, peer).subsystems().combat, peer_before);
    assert_eq!(unit(&map, creature).subsystems().combat, creature_before);
}

#[test]
fn peers_outside_the_original_union_are_not_scanned_or_changed() {
    let (mut map, owner, peer, creature, _) = fixture();
    start_melee(unit_mut(&mut map, peer), owner);
    unit_mut(&mut map, peer)
        .subsystems_mut()
        .combat
        .pvp_refs
        .insert(owner, CombatReferenceState::pvp());
    unit_mut(&mut map, owner).add_attacker_like_cpp(creature);
    let peer_before = unit(&map, peer).subsystems().combat.clone();

    assert_eq!(map.clear_player_combat(owner), Some(vec![creature]));
    assert_eq!(unit(&map, peer).subsystems().combat, peer_before);
    assert_eq!(unit(&map, peer).data().target, owner);
    assert!(
        unit(&map, peer)
            .current_spell(CurrentSpellSlot::Melee)
            .is_some()
    );
}

#[test]
fn cleanup_retains_the_existing_purge_short_circuit_for_duplicate_peer_buckets() {
    let (mut map, owner, peer, _, _) = fixture();
    unit_mut(&mut map, owner).add_attacker_like_cpp(peer);
    let combat = &mut unit_mut(&mut map, peer).subsystems_mut().combat;
    combat.pve_refs.insert(owner, CombatReferenceState::pve());
    combat.pvp_refs.insert(owner, CombatReferenceState::pvp());
    combat.add_attacker(owner);

    assert_eq!(map.clear_player_combat(owner), Some(vec![peer]));
    let combat = &unit(&map, peer).subsystems().combat;
    assert!(!combat.pve_refs.contains_key(&owner));
    // The original purge helper uses short-circuit OR; moving the loop does
    // not add a second purge to repair an already duplicated reference.
    assert_eq!(
        combat.pvp_refs.get(&owner),
        Some(&CombatReferenceState::pvp())
    );
    assert!(!combat.attackers.contains(&owner));
}

#[test]
fn owner_clear_attackers_keeps_its_existing_guid_side_effect_without_extra_attack_stop() {
    let (mut map, owner, _, _, foreign) = fixture();
    start_melee(unit_mut(&mut map, owner), foreign);

    assert_eq!(map.clear_player_combat(owner), Some(Vec::new()));
    let unit = unit(&map, owner);
    assert_eq!(unit.attacking(), None);
    assert_eq!(unit.data().target, foreign);
    assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
    assert!(unit.current_spell(CurrentSpellSlot::Melee).is_some());
}
