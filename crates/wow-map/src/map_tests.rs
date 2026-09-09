//! Behaviour tests for [`super`].
//!
//! Extracted from `map.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::*;
use crate::grid_unload::{
    GridObjectKind, GridUnloadAction, GridUnloadApplyOutcome, GuidGridUnloadLifecycle,
    apply_grid_unload_action, apply_grid_unload_actions,
};
use crate::pool::{PoolGroupLikeCpp, PoolTemplateDataLikeCpp};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wow_constants::{DeathState, TypeId, TypeMask, UnitStandStateType};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    ACTIVE_PLAYER_DATA_COINAGE_BIT, AppliedAuraRef, Corpse, CorpseType, Creature,
    CreatureAddToWorldVehicleResetContextLikeCpp, CreatureFormationInfoLikeCpp, GameObject,
    GameObjectLootSource, GameObjectOwnedLoot, GooberUseSource, ObjectNotifyFlags, OwnedAuraRef,
    PLAYER_DATA_INEBRIATION_BIT, Player, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, Transport,
    UNIT_DATA_STAND_STATE_BIT, VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo,
    VehicleSpellImmunity, VehicleSpellImmunityKind,
};
use wow_loot::{CreatureLoot, LootClaimCommitError, OwnedLootAuthorityLifecycle};

const GO_FLAG_MAP_OBJECT: u32 = 0x0010_0000;

#[derive(Debug, Default)]
struct RecordingTerrain {
    loads: Vec<(u32, u32)>,
    unloads: Vec<(u32, u32)>,
}

impl TerrainGridLoader for RecordingTerrain {
    fn load_map_and_vmap(&mut self, grid_x: u32, grid_y: u32) {
        self.loads.push((grid_x, grid_y));
    }

    fn unload_map(&mut self, grid_x: u32, grid_y: u32) {
        self.unloads.push((grid_x, grid_y));
    }
}

impl MapWorldObjectEnvironment for RecordingTerrain {
    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool {
        true
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        INVALID_HEIGHT
    }

    fn floor_z(&self, _object: &WorldObject, _position: Position, _max_search_dist: f32) -> f32 {
        INVALID_HEIGHT
    }
}

fn script_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn dynamic_model_key(counter: i64) -> RepresentedGameObjectModelKeyLikeCpp {
    RepresentedGameObjectModelKeyLikeCpp {
        owner_guid: guid(HighGuid::GameObject, counter),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LosCall {
    source_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    from: Position,
    to: Position,
    check_dynamic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HeightCall {
    object_guid: ObjectGuid,
    x: f32,
    y: f32,
    z: f32,
    query: WorldObjectHeightQuery,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloorCall {
    object_guid: ObjectGuid,
    position: Position,
    max_search_dist: f32,
}

#[derive(Debug)]
struct RecordingWorldObjectTerrain {
    los_result: bool,
    height_result: f32,
    floor_result: f32,
    los_calls: RefCell<Vec<LosCall>>,
    height_calls: RefCell<Vec<HeightCall>>,
    floor_calls: RefCell<Vec<FloorCall>>,
}

impl RecordingWorldObjectTerrain {
    fn new(los_result: bool, height_result: f32, floor_result: f32) -> Self {
        Self {
            los_result,
            height_result,
            floor_result,
            los_calls: RefCell::new(Vec::new()),
            height_calls: RefCell::new(Vec::new()),
            floor_calls: RefCell::new(Vec::new()),
        }
    }
}

impl TerrainGridLoader for RecordingWorldObjectTerrain {
    fn load_map_and_vmap(&mut self, _grid_x: u32, _grid_y: u32) {}
    fn unload_map(&mut self, _grid_x: u32, _grid_y: u32) {}
}

impl MapWorldObjectEnvironment for RecordingWorldObjectTerrain {
    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.los_calls.borrow_mut().push(LosCall {
            source_guid: query.source.guid(),
            target_guid: query.target.map(WorldObject::guid),
            from: query.from.position,
            to: query.to.position,
            check_dynamic: query.options.check_dynamic,
        });
        self.los_result
    }

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        self.height_calls.borrow_mut().push(HeightCall {
            object_guid: object.guid(),
            x,
            y,
            z,
            query,
        });
        self.height_result
    }

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32 {
        self.floor_calls.borrow_mut().push(FloorCall {
            object_guid: object.guid(),
            position,
            max_search_dist,
        });
        self.floor_result
    }
}

#[derive(Debug, Default)]
struct RecordingLifecycle {
    loads: usize,
    stops: usize,
    evacuates: usize,
    cleans: usize,
    unloads: usize,
}

impl GridLifecycle for RecordingLifecycle {
    fn load_grid_objects(&mut self, _grid: &mut NGrid, _cell: &Cell) {
        self.loads += 1;
    }

    fn stop_grid_objects(&mut self, _grid: &NGrid) {
        self.stops += 1;
    }

    fn evacuate_grid(&mut self, _grid: &mut NGrid) {
        self.evacuates += 1;
    }

    fn clean_grid(&mut self, _grid: &mut NGrid) {
        self.cleans += 1;
    }

    fn unload_grid_objects(&mut self, _grid: &mut NGrid) {
        self.unloads += 1;
    }
}

fn test_map() -> Map<RecordingTerrain, RecordingLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        100.0,
        RecordingTerrain::default(),
        RecordingLifecycle::default(),
    )
}

fn guid_unload_test_map() -> Map<RecordingTerrain, GuidGridUnloadLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        100.0,
        RecordingTerrain::default(),
        GuidGridUnloadLifecycle::new(),
    )
}

fn world_object_environment_test_map(
    terrain: RecordingWorldObjectTerrain,
    visible_distance: f32,
) -> Map<RecordingWorldObjectTerrain, RecordingLifecycle> {
    Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        visible_distance,
        terrain,
        RecordingLifecycle::default(),
    )
}

fn spawn_group(group_id: u32, flags: SpawnGroupFlags) -> SpawnGroupTemplateData {
    SpawnGroupTemplateData {
        group_id,
        name: format!("group-{group_id}"),
        map_id: 571,
        flags,
    }
}

const fn spawn_group_flags(left: SpawnGroupFlags, right: SpawnGroupFlags) -> SpawnGroupFlags {
    SpawnGroupFlags(left.0 | right.0)
}

fn spawn_data(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    spawn_group: SpawnGroupTemplateData,
) -> crate::spawn::SpawnData {
    crate::spawn::SpawnData {
        object_type,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group,
        id: 99,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    }
}

fn spawn_group_store(
    group: SpawnGroupTemplateData,
    mut spawns: Vec<crate::spawn::SpawnData>,
) -> (SpawnGroupTemplateData, SpawnStore) {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let rows = spawns
        .iter()
        .map(|spawn| crate::spawn::SpawnGroupMemberRow {
            group_id: group.group_id,
            spawn_type: spawn.object_type as u8,
            spawn_id: spawn.spawn_id,
        })
        .collect::<Vec<_>>();
    for spawn in &spawns {
        match spawn.object_type {
            SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                store.add_object_spawn(spawn, |_| false);
            }
            SpawnObjectType::AreaTrigger => store.add_area_trigger_spawn(spawn),
        }
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    for spawn in &mut spawns {
        spawn.spawn_group = templates
            .get(&group.group_id)
            .expect("group resolved")
            .clone();
    }
    (
        templates
            .get(&group.group_id)
            .expect("group resolved")
            .clone(),
        store,
    )
}

fn respawn_info(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    respawn_time: i64,
) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type,
        spawn_id,
        entry: 42,
        respawn_time,
        grid_id: 7,
    }
}

fn test_creature_for_spawn(spawn_id: SpawnId, counter: i64, alive: bool) -> Creature {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Creature, counter));
    creature.unit_mut().world_mut().object_mut().set_entry(42);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_death_state(DeathState::Alive);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(spawn_id);
    if !alive {
        creature.mark_ai_dead(1);
    }
    creature
}

fn test_gameobject_for_spawn(spawn_id: SpawnId, counter: i64) -> GameObject {
    let mut gameobject = GameObject::new();
    gameobject
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::GameObject, counter));
    gameobject.world_mut().object_mut().set_entry(42);
    gameobject.world_mut().set_map(571, 7).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    gameobject.world_mut().object_mut().add_to_world();
    gameobject.set_spawn_id(spawn_id);
    gameobject
}

fn money_loot_for_player_like_cpp(
    loot_guid: ObjectGuid,
    coins: u32,
    player: ObjectGuid,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid,
        coins,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn poll_immediately_ready<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;

    impl std::task::Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("expected the uncontended claim to be ready"),
    }
}

fn summon_gameobject_template_like_cpp(
    entry: u32,
    go_type: u32,
) -> GameObjectTemplateLifecycleRecord {
    GameObjectTemplateLifecycleRecord {
        entry,
        name: "spell summoned gameobject".to_string(),
        go_type,
        display_id: 400,
        scale: 1.0,
        faction: 35,
        flags: 0,
        data: [0; wow_entities::MAX_GAMEOBJECT_DATA],
        world_effect_id: 0,
        anim_kit_id: 0,
        level: 1,
        percent_health: 100,
        custom_param: 0,
    }
}

fn test_transport(counter: i64, in_world: bool) -> wow_entities::Transport {
    let mut transport = wow_entities::Transport::new();
    transport
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Transport, counter));
    transport.world_mut().set_map(571, 7).unwrap();
    transport.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        transport.world_mut().object_mut().add_to_world();
    }
    transport
}

fn test_pet(counter: i64, in_world: bool) -> wow_entities::Pet {
    let owner = ObjectGuid::create_player(1, 484_000);
    let mut pet = wow_entities::Pet::new(owner, wow_entities::PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Pet, counter));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 7)
        .unwrap();
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
    }
    pet
}

fn test_area_trigger_for_spawn(spawn_id: SpawnId, counter: i64) -> AreaTrigger {
    let mut area_trigger = AreaTrigger::new();
    area_trigger
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::AreaTrigger, counter));
    area_trigger.world_mut().object_mut().set_entry(42);
    area_trigger.world_mut().set_map(571, 7).unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    area_trigger.world_mut().object_mut().add_to_world();
    area_trigger.set_spawn_id(spawn_id);
    area_trigger
}

fn dynamic_respawn_context(spawn_type: Option<SpawnObjectType>) -> DynamicRespawnScalingContext {
    DynamicRespawnScalingContext {
        mode: 1,
        spawn_type,
        spawn_metadata_present: true,
        spawn_group_flags: Some(SpawnGroupFlags::DYNAMIC_SPAWN_RATE),
        is_battleground_or_arena: false,
        zone_player_count: Some(4),
        config: DynamicRespawnScalingConfig {
            creature_rate: 1.0,
            creature_minimum_secs: 30,
            gameobject_rate: 1.5,
            gameobject_minimum_secs: 60,
        },
    }
}

fn assert_dynamic_respawn_noop(
    context: DynamicRespawnScalingContext,
    reason: DynamicRespawnScalingNoopReason,
) {
    let outcome = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);
    assert_eq!(outcome.delay_secs, 120);
    assert_eq!(outcome.noop_reason, Some(reason));
    assert!(!outcome.was_scaled());
}

fn linked_respawn_guid(high: HighGuid, entry: u32, spawn_id: SpawnId) -> ObjectGuid {
    ObjectGuid::create_world_object(high, 0, 0, 571, 0, entry, spawn_id as i64)
}

fn guid(high: HighGuid, counter: i64) -> ObjectGuid {
    if high == HighGuid::Player {
        ObjectGuid::create_global(high, 0, counter)
    } else if high == HighGuid::Transport {
        ObjectGuid::create_transport(high, counter)
    } else {
        ObjectGuid::create_world_object(high, 0, 1, 571, 7, 100, counter)
    }
}

fn world_object(high: HighGuid, map_id: u32, instance_id: u32, in_world: bool) -> WorldObject {
    let type_id = guid(high, 1).type_id();
    let type_mask = match type_id {
        wow_core::guid::TypeId::Player => TypeMask::PLAYER,
        wow_core::guid::TypeId::Unit => TypeMask::UNIT,
        wow_core::guid::TypeId::GameObject => TypeMask::GAME_OBJECT,
        wow_core::guid::TypeId::DynamicObject => TypeMask::DYNAMIC_OBJECT,
        wow_core::guid::TypeId::Corpse => TypeMask::CORPSE,
        wow_core::guid::TypeId::AreaTrigger => TypeMask::AREA_TRIGGER,
        wow_core::guid::TypeId::SceneObject => TypeMask::SCENE_OBJECT,
        wow_core::guid::TypeId::Conversation => TypeMask::CONVERSATION,
        _ => TypeMask::OBJECT,
    };
    let mut object = WorldObject::new(false, convert_type_id(type_id), type_mask);
    object.object_mut().create(guid(high, 1));
    object.set_map(map_id, instance_id).unwrap();
    object.relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        object.object_mut().add_to_world();
    }
    object
}

fn world_object_with_counter(
    high: HighGuid,
    counter: i64,
    map_id: u32,
    instance_id: u32,
    in_world: bool,
) -> WorldObject {
    let object_guid = guid(high, counter);
    let type_id = object_guid.type_id();
    let type_mask = match type_id {
        wow_core::guid::TypeId::Player => TypeMask::PLAYER,
        wow_core::guid::TypeId::Unit => TypeMask::UNIT,
        wow_core::guid::TypeId::GameObject => TypeMask::GAME_OBJECT,
        wow_core::guid::TypeId::DynamicObject => TypeMask::DYNAMIC_OBJECT,
        wow_core::guid::TypeId::Corpse => TypeMask::CORPSE,
        wow_core::guid::TypeId::AreaTrigger => TypeMask::AREA_TRIGGER,
        wow_core::guid::TypeId::SceneObject => TypeMask::SCENE_OBJECT,
        wow_core::guid::TypeId::Conversation => TypeMask::CONVERSATION,
        _ => TypeMask::OBJECT,
    };
    let mut object = WorldObject::new(false, convert_type_id(type_id), type_mask);
    object.object_mut().create(object_guid);
    object.set_map(map_id, instance_id).unwrap();
    object.relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        object.object_mut().add_to_world();
    }
    object
}

fn game_object_with_counter(
    counter: i64,
    map_id: u32,
    instance_id: u32,
    in_world: bool,
) -> GameObject {
    let mut game_object = GameObject::new();
    game_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::GameObject, counter));
    game_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        game_object.world_mut().object_mut().add_to_world();
    }
    game_object
}

fn convert_type_id(type_id: wow_core::guid::TypeId) -> TypeId {
    match type_id {
        wow_core::guid::TypeId::Object => TypeId::Object,
        wow_core::guid::TypeId::Item => TypeId::Item,
        wow_core::guid::TypeId::Container => TypeId::Container,
        wow_core::guid::TypeId::AzeriteEmpoweredItem => TypeId::AzeriteEmpoweredItem,
        wow_core::guid::TypeId::AzeriteItem => TypeId::AzeriteItem,
        wow_core::guid::TypeId::Unit => TypeId::Unit,
        wow_core::guid::TypeId::Player => TypeId::Player,
        wow_core::guid::TypeId::ActivePlayer => TypeId::ActivePlayer,
        wow_core::guid::TypeId::GameObject => TypeId::GameObject,
        wow_core::guid::TypeId::DynamicObject => TypeId::DynamicObject,
        wow_core::guid::TypeId::Corpse => TypeId::Corpse,
        wow_core::guid::TypeId::AreaTrigger => TypeId::AreaTrigger,
        wow_core::guid::TypeId::SceneObject => TypeId::SceneObject,
        wow_core::guid::TypeId::Conversation => TypeId::Conversation,
    }
}

fn creature_formation_info_like_cpp(leader_spawn_id: SpawnId) -> CreatureFormationInfoLikeCpp {
    CreatureFormationInfoLikeCpp {
        leader_spawn_id,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }
}

fn creature_add_to_world_vehicle_reset_context(
    is_mechanical_creature: bool,
    is_world_boss: bool,
) -> CreatureAddToWorldVehicleResetContextLikeCpp {
    CreatureAddToWorldVehicleResetContextLikeCpp {
        is_mechanical_creature,
        is_world_boss,
        accessories: vec![VehicleAccessory {
            accessory_entry: 7001,
            is_minion: false,
            summon_time_ms: 3_000,
            seat_id: 1,
            summoned_type: 6,
        }],
    }
}

fn create_loaded_creature_vehicle_kit_like_cpp(creature: &mut Creature, vehicle_id: u32) {
    let guid = creature.guid();
    let position = creature.unit().world().position();
    let entry = creature.unit().world().object().entry();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .create_vehicle_kit_like_cpp(
            guid,
            position,
            Some(vehicle_id),
            entry,
            true,
            Some(vec![(
                0,
                VehicleSeatInfo {
                    id: 100,
                    attachment_offset: Position::default(),
                    can_enter_or_exit: true,
                    usable_by_override: false,
                    can_control: false,
                    can_switch_from_seat: false,
                    ejectable: false,
                    disables_gravity: false,
                    passenger_not_selectable: false,
                    keep_pet: false,
                },
                VehicleSeatAddon::default(),
            )]),
        );
}

fn add_loaded_grid_creature_for_switch(
    map: &mut Map<RecordingTerrain, RecordingLifecycle>,
    spawn_id: SpawnId,
    counter: i64,
) -> (ObjectGuid, CellCoord, GridCoord) {
    let cell = Cell::from_world(1.0, 2.0);
    let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
    map.ensure_grid_loaded(&cell);
    let mut creature = test_creature_for_spawn(spawn_id, counter, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(outcome.inserted_into_cell);
    (guid, outcome.cell, grid)
}

fn local_cell_for_switch<'a>(
    map: &'a Map<RecordingTerrain, RecordingLifecycle>,
    grid: GridCoord,
    cell: CellCoord,
) -> &'a Cell {
    map.get_ngrid(grid)
        .unwrap()
        .get_grid_type(
            cell.x_coord % MAX_NUMBER_OF_CELLS,
            cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap()
}

fn test_player_for_viewpoint(counter: i64) -> Player {
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

fn test_dynamic_object_for_viewpoint(counter: i64) -> DynamicObject {
    let mut dynamic_object = DynamicObject::new(true);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::DynamicObject, counter));
    dynamic_object.world_mut().set_map(571, 7).unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(11.0, 21.0, 31.0));
    dynamic_object.world_mut().object_mut().add_to_world();
    dynamic_object
}

fn create_farsight_focus_for_tests<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    caster_player_guid: ObjectGuid,
) -> FarsightDynamicObjectCreateOutcomeLikeCpp
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    map.create_farsight_dynamic_object_like_cpp(
        caster_player_guid,
        12_345,
        678,
        Position::new(100.0, 200.0, 30.0, 1.5),
        42.5,
        30_000,
        987_654,
        1,
        7,
    )
}

fn test_area_trigger_for_update(counter: i64, duration_ms: i32, in_world: bool) -> AreaTrigger {
    let mut area_trigger = AreaTrigger::new();
    area_trigger
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::AreaTrigger, counter));
    area_trigger.world_mut().object_mut().set_entry(42);
    area_trigger.world_mut().set_map(571, 7).unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        area_trigger.world_mut().object_mut().add_to_world();
    }
    area_trigger.set_duration(duration_ms);
    area_trigger
}

fn test_conversation_for_update(counter: i64, duration_ms: i32, in_world: bool) -> Conversation {
    let mut conversation = Conversation::new();
    conversation
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Conversation, counter));
    conversation.world_mut().object_mut().set_entry(42);
    conversation.world_mut().set_map(571, 7).unwrap();
    conversation
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        conversation.world_mut().object_mut().add_to_world();
    }
    conversation.set_duration_ms(duration_ms);
    conversation
}

fn test_transport_for_update(counter: i64, in_world: bool) -> Transport {
    let template = wow_entities::TransportTemplate {
        total_path_time_ms: 1_000,
        path_legs: vec![wow_entities::TransportPathLeg {
            map_id: 571,
            start_timestamp_ms: 0,
            duration_ms: 1_000,
            segments: vec![],
        }],
        ..wow_entities::TransportTemplate::default()
    };
    let mut transport = Transport::with_template(template);
    transport
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Transport, counter));
    transport.world_mut().set_map(571, 7).unwrap();
    transport.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    transport.set_path_progress_ms(100);
    if in_world {
        transport.world_mut().object_mut().add_to_world();
    }
    transport
}

fn test_scene_object_for_update(
    counter: i64,
    in_world: bool,
    created_by_spell_cast: ObjectGuid,
) -> SceneObject {
    let mut scene_object = SceneObject::new();
    scene_object
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::SceneObject, counter));
    scene_object.world_mut().object_mut().set_entry(42);
    scene_object.world_mut().set_map(571, 7).unwrap();
    scene_object
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    scene_object.set_created_by(guid(HighGuid::Player, counter + 1_000));
    scene_object.set_created_by_spell_cast(created_by_spell_cast);
    if in_world {
        scene_object.world_mut().object_mut().add_to_world();
    }
    scene_object
}

#[path = "map_tests/combat.rs"]
mod combat;
#[path = "map_tests/creature_1.rs"]
mod creature_1;
#[path = "map_tests/creature_2.rs"]
mod creature_2;
#[path = "map_tests/creature_3.rs"]
mod creature_3;
#[path = "map_tests/creature_4.rs"]
mod creature_4;
#[path = "map_tests/gameobject_1.rs"]
mod gameobject_1;
#[path = "map_tests/gameobject_2.rs"]
mod gameobject_2;
#[path = "map_tests/gameobject_3.rs"]
mod gameobject_3;
#[path = "map_tests/gameobject_4.rs"]
mod gameobject_4;
#[path = "map_tests/gameobject_5.rs"]
mod gameobject_5;
#[path = "map_tests/gameobject_6.rs"]
mod gameobject_6;
#[path = "map_tests/group_1.rs"]
mod group_1;
#[path = "map_tests/group_2.rs"]
mod group_2;
#[path = "map_tests/instance.rs"]
mod instance;
#[path = "map_tests/item.rs"]
mod item;
#[path = "map_tests/loot.rs"]
mod loot;
#[path = "map_tests/misc_1.rs"]
mod misc_1;
#[path = "map_tests/misc_2.rs"]
mod misc_2;
#[path = "map_tests/movement_1.rs"]
mod movement_1;
#[path = "map_tests/movement_2.rs"]
mod movement_2;
#[path = "map_tests/persistence_1.rs"]
mod persistence_1;
#[path = "map_tests/persistence_2.rs"]
mod persistence_2;
#[path = "map_tests/quest.rs"]
mod quest;
#[path = "map_tests/skill.rs"]
mod skill;
#[path = "map_tests/spawn_1.rs"]
mod spawn_1;
#[path = "map_tests/spawn_2.rs"]
mod spawn_2;
#[path = "map_tests/spell_1.rs"]
mod spell_1;
#[path = "map_tests/spell_2.rs"]
mod spell_2;
#[path = "map_tests/visibility.rs"]
mod visibility;
