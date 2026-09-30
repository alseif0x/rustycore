use super::*;
use wow_core::guid::HighGuid;
use wow_entities::{AuraCastProvenanceLikeCpp, AuraRef, AppliedAuraRef, MapObjectRecord, Pet, PetType, Unit};

mod auras;
mod capture;
mod selection;

fn creature(counter: i64, in_world: bool) -> Creature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(42);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.set_ai_position(Position::xyz(10.0, 10.0, 0.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.unit_mut().set_level(12);
    creature.unit_mut().set_combat_reach(1.5);
    if in_world {
        creature.unit_mut().world_mut().object_mut().add_to_world();
    }
    creature
}

fn legacy(counter: i64) -> WorldCreature {
    let creature = creature(counter, false);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

fn pet(counter: i64) -> Pet {
    let mut pet = Pet::new(ObjectGuid::create_player(1, 99), PetType::Hunter);
    let guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 7, 42, counter);
    pet.creature_mut().unit_mut().world_mut().object_mut().create(guid);
    pet.creature_mut().unit_mut().world_mut().set_map(571, 7).unwrap();
    pet.creature_mut().set_ai_position(Position::xyz(11.0, 10.0, 0.0));
    pet.creature_mut().unit_mut().set_max_health(100);
    pet.creature_mut().unit_mut().set_health(80);
    pet
}

fn collect(map: &Map) -> Vec<CreatureVisibilityCandidate> {
    map.capture_compatible_creature_visibility(
        &Position::xyz(10.0, 10.0, 0.0), 100.0, 1.5, &PhaseShift::default(),
    )
}
