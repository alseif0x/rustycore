//! Original packet contracts and actual dormant manager/APP consumer.
use super::*;
use crate::map_manager::{RecipientRule, WorldCreature};
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Player, MapObjectRecord, UnitVisibilityDetectionStateLikeCpp};
use wow_packet::ServerPacket;
use wow_map::{GridCoord, MapManager, MapObjectUpdateSelectionLikeCpp, MIN_GRID_DELAY_MS};

mod packets;
mod driver;

fn guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 42, counter)
}

fn actor(counter: i64, position: Position) -> WorldCreature {
    let mut actor = WorldCreature::new(guid(counter), 42, position,
        100, 25, 3, 5, 20.0, 100, 14, 0, 0);
    actor.creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    actor.creature.unit_mut().world_mut().object_mut().add_to_world();
    actor.creature.set_react_state(wow_entities::ReactState::Aggressive);
    actor.creature.ai_ownership_mut().aggro_radius = 20.0;
    actor.seed_runtime_rng_like_cpp(0x5757);
    actor
}

fn candidate(guid: ObjectGuid) -> LegacyCreatureAggroCandidateLikeCpp {
    LegacyCreatureAggroCandidateLikeCpp {
        player_guid: guid, map_id: 1, instance_id: 0, map_difficulty_id: 0,
        position: Position::xyz(11.0, 20.0, 30.0), player_visibility_represented: true,
        player_phase_shift: PhaseShift::default(), player_visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
        player_combat_reach: 1.5, player_detected_range_aura_mod: 0.0, player_liquid_status_like_cpp: 0,
        player_level: 25, player_gray_level: 0, player_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        player_unit_flags2: 0, player_unit_state: 0, player_is_game_master: false, player_is_contested_pvp: false,
        player_faction_template_id: 1, player_reputation_standings: Vec::new(), player_reputation_state_flags: Vec::new(),
        player_forced_reputation_ranks: Vec::new(), player_forced_reputation_faction_ids: Vec::new(),
        player_school_immunity_mask: 0, player_damage_immunity_mask: 0,
        player_has_confuse_aura: false, player_has_breakable_stun_aura: false,
    }
}

fn config() -> LegacyCreatureAggroConfigLikeCpp {
    use wow_data::progression_rewards::{FactionTemplateEntry, FactionTemplateStore};
    let entry = |id, faction, enemy| {
        let mut enemies = [0; 8]; enemies[0] = enemy;
        FactionTemplateEntry { id, faction, flags: 0, faction_group: 0,
            friend_group: 0, enemy_group: 0, enemies, friend: [0; 8] }
    };
    LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(FactionTemplateStore::from_entries([
            entry(14, 72, 930), entry(1, 930, 0)]))),
        faction_store: Some(Arc::new(FactionStore::from_entries([
            wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1)]))),
        family_assistance_radius: 1_000.0, family_assistance_delay_ms: 100,
        ..Default::default()
    }
}

fn insert(manager: &mut MapManager, mut incoming: WorldCreature, active: bool) {
    let guid = incoming.guid(); let position = incoming.position();
    incoming.creature.unit_mut().world_mut().set_active(active);
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.test_fixture_admit_creature_actor(incoming);
    let cell = wow_map::cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y())).unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y()).unwrap().grid_objects.creatures.insert(guid);
    if active { assert!(map.add_to_active_like_cpp(guid).inserted_in_active_set); }
}

fn add_player(manager: &mut MapManager, guid: ObjectGuid) {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player.unit_mut().world_mut().relocate(candidate(guid).position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_max_health(100); player.unit_mut().set_health(100);
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(MapObjectRecord::new_player(player).unwrap()).unwrap();
    manager.adopt_active_player_like_cpp(guid).unwrap();
}

fn begin(manager: &mut MapManager) -> (MapObjectTickContinuation, ObjectMapUpdateToken) {
    let plan = manager.begin_tick_like_cpp(77).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells).unwrap().unwrap();
    (tick, token)
}
