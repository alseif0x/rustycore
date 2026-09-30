use super::*;
use rand::rngs::StdRng;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{AppliedAuraRef, Creature, Player,
    SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP};

mod record;
mod actor;

fn map() -> Map {
    Map::new(571, 7, 1, 1000)
}

fn creature(counter: i64) -> Creature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(counter as u64 * 10);
    creature
}

fn record(counter: i64) -> MapObjectRecord {
    MapObjectRecord::new_creature(creature(counter)).unwrap()
}

fn player_record(counter: i64, map_id: u32) -> MapObjectRecord {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(ObjectGuid::create_player(1, counter));
    player.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    MapObjectRecord::new_player(player).unwrap()
}

fn actor(counter: i64, point: bool) -> (WorldCreature, StdRng) {
    let creature = creature(counter);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x1234;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

fn aura() -> AppliedAuraRef {
    AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 0x1)
}

fn records_with_facets(counter: i64, primary_wrong_map: bool) -> LoadedGridRespawnRecordsLikeCpp {
    let mut primary = creature(counter);
    primary.unit_mut().subsystems_mut().auras.register_applied_aura(
        aura(), None, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, 0,
    );
    if primary_wrong_map {
        primary.unit_mut().world_mut().reset_map().unwrap();
        primary.unit_mut().world_mut().set_map(530, 7).unwrap();
    }
    LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player_record(981, 571), player_record(982, 530), player_record(983, 571)],
        primary_record: MapObjectRecord::new_creature(primary).unwrap(),
    }
}
