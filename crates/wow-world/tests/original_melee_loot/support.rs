//! Real canonical owners and immutable catalog fixtures; no copied loot rules.
use super::catalogs::Catalogs;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{Creature, OwnedLootAuthority};
use wow_map::{MapManager, GridCoord};
use wow_map::manager::{MapObjectTickContinuation, MapObjectUpdateSelectionLikeCpp, PreparedMeleeKill,
    PendingMeleeKills, MeleeLootError};
use wow_map::map_manager::WorldCreature;
use wow_world::{session::WorldSession, test_fixtures};
use wow_world::test_fixtures::loot::*;
use wow_loot::{LootStore, LootStoreKind, LootStores, LootTemplateRow, LootStoreItem};

pub(super) struct Fixture {
    pub session: WorldSession,
    pub manager: Arc<Mutex<MapManager>>,
    pub tick: MapObjectTickContinuation,
    pub prepared: PreparedMeleeKill,
    pub root: ObjectGuid,
    pub victim: ObjectGuid,
    pub player: ObjectGuid,
    pub authority: OwnedLootAuthority,
}
pub(super) fn guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 42, counter)
}
fn actor(counter: i64, health: u64, position: Position) -> WorldCreature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid(counter));
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(health);
    creature.unit_mut().set_level(80);
    creature.set_ai_identity_runtime(1, 35, 0, 0);
    creature.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
        dodge_pct: 0.0, parry_pct: 0.0, block_pct: 0.0,
    });
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}
fn admit(manager: &mut MapManager, actor: WorldCreature) {
    let guid = actor.guid();
    let position = actor.position();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.test_fixture_admit_creature_actor(actor);
    let cell = wow_map::map::cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y())).unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y()).unwrap()
        .grid_objects.creatures.insert(guid);
}
pub(super) fn fixture(counter: i64, loot_id: u32, tapped: bool) -> Fixture {
    fixture_with(counter, loot_id, tapped, 0, false)
}
pub(super) fn fixture_with(counter: i64, loot_id: u32, tapped: bool,
    encounter: u32, companion: bool) -> Fixture {
    let mut session = make_session();
    let manager = Arc::new(Mutex::new(MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200)));
    manager.lock().unwrap().create_world_map(1, 0);
    session.set_canonical_map_manager(manager.clone());
    let player = test_fixtures::install_canonical_player_owner_for_test(&mut session, 1, 0);
    let root = guid(counter);
    let victim = guid(counter + 1);
    let mut attacker = actor(counter, 75, Position::xyz(10.0, 20.0, 30.0));
    attacker.creature.unit_mut().world_mut().set_active(true);
    attacker.enter_combat(victim);
    attacker.creature.ai_ownership_mut().min_damage = 10;
    attacker.creature.ai_ownership_mut().max_damage = 10;
    attacker.creature.ai_ownership_mut().swing_timer_ms = 0;
    attacker.seed_runtime_rng_like_cpp(17);
    let mut target = actor(counter + 1, 5, Position::xyz(11.0, 20.0, 30.0));
    target.creature.ai_ownership_mut().loot_id = loot_id;
    target.creature.ai_ownership_mut().gold_min = 7;
    target.creature.ai_ownership_mut().gold_max = 7;
    target.creature.ai_ownership_mut().dungeon_encounter_id = encounter;
    let other = ObjectGuid::create_player(player.realm_id(), player.counter() + 1);
    if tapped {
        let others = if companion { vec![other] } else { Vec::new() };
        target.creature.set_tapped_by_player(player, &others);
    }
    let authority = target.creature.loot_authority_like_cpp().clone();
    let (tick, prepared) = {
        let mut maps = manager.lock().unwrap();
        admit(&mut maps, attacker);
        admit(&mut maps, target);
        maps.find_map_mut(1, 0).unwrap().map_mut().add_to_active_like_cpp(root);
        let plan = maps.begin_tick_like_cpp(200).into_started().unwrap();
        let mut tick = maps.begin_object_tick(plan).unwrap();
        let token = maps.prepare_next_object_map(&mut tick,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores).unwrap().unwrap();
        let execution = maps.apply_selected_creature_melee_with_kills(
            &tick, token, root, &Catalogs::default()).ok().unwrap();
        assert_eq!(execution.outcome().canonical_creature_hits, 1);
        let prepared = maps.prepare_next_melee_kill(&tick, execution.into_pending()).ok().unwrap();
        (tick, prepared)
    };
    Fixture { session, manager, tick, prepared, root, victim, player, authority }
}
pub(super) fn map_kind(session: &mut WorldSession, dungeon: bool) {
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1, instance_type: if dungeon { wow_data::map::MAP_INSTANCE }
            else { wow_data::map::MAP_COMMON },
        expansion_id: 0, parent_map_id: -1, cosmetic_parent_map_id: -1,
        flags1: 0, flags2: 0,
    }])));
}
pub(super) fn store(session: &mut WorldSession, loot_id: u32) {
    install_basic_item_template_for_loot_test(session, 80101, 0);
    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature.load_rows_like_cpp([LootTemplateRow { entry: loot_id, item: LootStoreItem {
        item_id: 80101, reference: 0, chance: 100.0, needs_quest: false,
        loot_mode: wow_loot::LOOT_MODE_DEFAULT_LIKE_CPP, group_id: 0, min_count: 1, max_count: 1,
    } }], |_| true).unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);
    session.set_loot_stores(Arc::new(stores));
}
pub(super) fn pending(result: Result<PendingMeleeKills, (MeleeLootError, PendingMeleeKills)>)
    -> PendingMeleeKills {
    match result { Ok(pending) => pending, Err((error, _)) => panic!("loot failed: {error:?}") }
}
pub(super) fn rejected(result: Result<PendingMeleeKills, (MeleeLootError, PendingMeleeKills)>)
    -> (MeleeLootError, PendingMeleeKills) {
    match result { Err(error) => error, Ok(_) => panic!("expected original-operation rejection") }
}
pub(super) fn replace_actor(manager: &mut MapManager, id: ObjectGuid) {
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    assert!(map.remove_map_object(id).is_some());
    let replacement = actor(id.counter(), 75, Position::xyz(11.0, 20.0, 30.0));
    assert_eq!(replacement.guid(), id);
    map.test_fixture_admit_creature_actor(replacement);
}
pub(super) fn retained(pending: &PendingMeleeKills, root: ObjectGuid) {
    assert_eq!(pending.root_guid(), root);
    assert_eq!(pending.cursor(), 0);
    assert_eq!(pending.occurrence_count(), 1);
    assert_eq!(pending.outcome().canonical_creature_hits, 1);
    assert_eq!(pending.outcome().events.len(), 2);
    assert_eq!(pending.outcome().syncs.len(), 1);
}
