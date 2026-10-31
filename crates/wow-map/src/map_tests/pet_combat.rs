// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Native pet combat-stop scenarios for the canonical Map unit store.

use super::*;
use wow_constants::UnitFlags;
use wow_entities::{CurrentSpellRef, CurrentSpellSlot};

fn combat_test_player(counter: i64) -> Player {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Player, counter));
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player
}

fn combat_test_creature(counter: i64) -> Creature {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Creature, counter));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(11.0, 20.0, 30.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature
}

fn set_in_combat_flag(unit: &mut wow_entities::Unit, in_combat: bool) {
    let mut flags = unit.unit_flags_like_cpp();
    flags.set(UnitFlags::IN_COMBAT, in_combat);
    unit.set_unit_flags_like_cpp(flags);
}

#[test]
fn pet_combat_stop_unlinks_typed_attackers_and_reciprocal_refs_like_cpp() {
    let mut map = test_map();
    let pet_guid = guid(HighGuid::Pet, 701);
    let other_pet_guid = guid(HighGuid::Pet, 702);
    let player_guid = guid(HighGuid::Player, 703);
    let creature_guid = guid(HighGuid::Creature, 704);
    let unrelated_creature_guid = guid(HighGuid::Creature, 705);

    map.insert_map_object_record(MapObjectRecord::new_pet(test_pet(701, true)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_pet(test_pet(702, true)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_player(combat_test_player(703)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(combat_test_creature(704)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(combat_test_creature(705)).unwrap())
        .unwrap();

    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .set_attacking(Some(player_guid));
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_attacker(pet_guid);

    for attacker_guid in [player_guid, creature_guid, other_pet_guid] {
        if let Some(player) = map.get_typed_player_mut(attacker_guid) {
            player.unit_mut().set_attacking(Some(pet_guid));
        } else if let Some(creature) = map.get_typed_creature_mut(attacker_guid) {
            creature.unit_mut().set_attacking(Some(pet_guid));
        } else {
            map.get_typed_pet_mut(attacker_guid)
                .unwrap()
                .creature_mut()
                .unit_mut()
                .set_attacking(Some(pet_guid));
        }
        map.get_typed_pet_mut(pet_guid)
            .unwrap()
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_attacker(attacker_guid);
    }

    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(creature_guid, false, false);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(pet_guid, false, false);
    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(other_pet_guid, false, false);
    map.get_typed_pet_mut(other_pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(pet_guid, false, false);
    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(player_guid, true, false);
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(pet_guid, true, false);

    let pet_threat = {
        let unit = map
            .get_typed_pet_mut(pet_guid)
            .unwrap()
            .creature_mut()
            .unit_mut();
        unit.subsystems_mut().combat.set_threat(creature_guid, 9.0);
        unit.subsystems()
            .combat
            .threat_ref(creature_guid)
            .copied()
            .unwrap()
    };
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(pet_guid, pet_threat);

    let creature_threat = {
        let unit = map
            .get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut();
        unit.subsystems_mut().combat.set_threat(pet_guid, 5.0);
        unit.subsystems()
            .combat
            .threat_ref(pet_guid)
            .copied()
            .unwrap()
    };
    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(creature_guid, creature_threat);

    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(unrelated_creature_guid, false, false);
    map.get_typed_creature_mut(unrelated_creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(player_guid, false, false);

    let non_melee = CurrentSpellRef::new(900, Some(pet_guid), None).with_cast_time_ms(1_000);
    let melee = CurrentSpellRef::new(901, Some(pet_guid), None).with_cast_time_ms(1_000);
    let pet_unit = map
        .get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut();
    pet_unit
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Generic, non_melee);
    pet_unit
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee);
    set_in_combat_flag(pet_unit, true);
    set_in_combat_flag(
        map.get_typed_pet_mut(other_pet_guid)
            .unwrap()
            .creature_mut()
            .unit_mut(),
        true,
    );
    set_in_combat_flag(
        map.get_typed_player_mut(player_guid).unwrap().unit_mut(),
        true,
    );
    set_in_combat_flag(
        map.get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut(),
        true,
    );
    set_in_combat_flag(
        map.get_typed_creature_mut(unrelated_creature_guid)
            .unwrap()
            .unit_mut(),
        true,
    );

    let outcome = map.stop_pet_combat_like_cpp(pet_guid).unwrap();

    assert_eq!(outcome.attacks_stopped.len(), 4);
    for stopped in [
        (pet_guid, player_guid),
        (other_pet_guid, pet_guid),
        (player_guid, pet_guid),
        (creature_guid, pet_guid),
    ] {
        assert!(outcome.attacks_stopped.contains(&stopped));
    }
    let exited_combat: HashSet<_> = outcome.exited_combat.iter().copied().collect();
    assert_eq!(
        exited_combat,
        HashSet::from([pet_guid, other_pet_guid, creature_guid])
    );
    assert_eq!(
        map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .current_spell(CurrentSpellSlot::Generic),
        Some(non_melee)
    );
    assert_eq!(
        map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .current_spell(CurrentSpellSlot::Melee),
        None
    );
    assert_eq!(
        map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .attacking(),
        None
    );
    assert!(
        map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .subsystems()
            .combat
            .attackers
            .is_empty()
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .attacking(),
        None
    );
    assert!(
        !map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .attackers
            .contains(&pet_guid)
    );
    assert!(
        map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .subsystems()
            .combat
            .threat_refs
            .is_empty()
    );
    assert!(
        map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_refs
            .is_empty()
    );
    assert!(
        map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me
            .is_empty()
    );
    assert!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(unrelated_creature_guid)
    );
    assert!(
        map.get_typed_creature(unrelated_creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(player_guid)
    );
    assert!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IN_COMBAT)
    );
    assert!(
        map.get_typed_creature(unrelated_creature_guid)
            .unwrap()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IN_COMBAT)
    );
    assert!(
        !map.get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IN_COMBAT)
    );
}

#[test]
fn pet_combat_stop_rejects_missing_references_without_mutating_state() {
    let mut map = test_map();
    let pet_guid = guid(HighGuid::Pet, 711);
    let missing_guid = guid(HighGuid::Creature, 712);
    map.insert_map_object_record(MapObjectRecord::new_pet(test_pet(711, true)).unwrap())
        .unwrap();
    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(missing_guid, false, false);

    let before = map
        .get_typed_pet(pet_guid)
        .unwrap()
        .creature()
        .unit()
        .clone();
    assert!(map.stop_pet_combat_like_cpp(pet_guid).is_err());
    assert_eq!(
        map.get_typed_pet(pet_guid).unwrap().creature().unit(),
        &before
    );
}

#[test]
fn pet_combat_stop_rejects_missing_or_wrong_typed_pet_without_mutation() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 721);
    map.insert_map_object_record(MapObjectRecord::new_creature(combat_test_creature(721)).unwrap())
        .unwrap();
    let before = map.get_typed_creature(creature_guid).unwrap().clone();

    assert!(
        map.stop_pet_combat_like_cpp(guid(HighGuid::Pet, 722))
            .is_err()
    );
    assert!(map.stop_pet_combat_like_cpp(creature_guid).is_err());
    assert_eq!(map.get_typed_creature(creature_guid).unwrap(), &before);
}

#[test]
fn pet_combat_stop_rejects_attacker_indexed_for_another_victim_without_mutation() {
    let mut map = test_map();
    let pet_guid = guid(HighGuid::Pet, 731);
    let attacker_guid = guid(HighGuid::Player, 732);
    let other_victim_guid = guid(HighGuid::Creature, 733);

    map.insert_map_object_record(MapObjectRecord::new_pet(test_pet(731, true)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_player(combat_test_player(732)).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(combat_test_creature(733)).unwrap())
        .unwrap();

    map.get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_attacker(attacker_guid);
    map.get_typed_player_mut(attacker_guid)
        .unwrap()
        .unit_mut()
        .set_attacking(Some(other_victim_guid));
    map.get_typed_creature_mut(other_victim_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_attacker(attacker_guid);

    let pet_before = map
        .get_typed_pet(pet_guid)
        .unwrap()
        .creature()
        .unit()
        .clone();
    let attacker_before = map.get_typed_player(attacker_guid).unwrap().unit().clone();
    let victim_before = map
        .get_typed_creature(other_victim_guid)
        .unwrap()
        .unit()
        .clone();

    assert_eq!(
        map.stop_pet_combat_like_cpp(pet_guid),
        Err("pet attacker index contains a unit targeting another victim")
    );
    assert_eq!(
        map.get_typed_pet(pet_guid).unwrap().creature().unit(),
        &pet_before
    );
    assert_eq!(
        map.get_typed_player(attacker_guid).unwrap().unit(),
        &attacker_before
    );
    assert_eq!(
        map.get_typed_creature(other_victim_guid).unwrap().unit(),
        &victim_before
    );
}
