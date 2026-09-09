//! Object accessor records and lookups regression scenarios.
//!
//! Separated from the object_accessor.rs root under #648.

use super::*;
use wow_constants::{TypeId, TypeMask};
use wow_core::Position;

fn guid(high: HighGuid, counter: i64) -> ObjectGuid {
    if high == HighGuid::Player {
        ObjectGuid::create_global(high, 0, counter)
    } else if high == HighGuid::Transport {
        ObjectGuid::create_transport(high, counter)
    } else {
        ObjectGuid::create_world_object(high, 0, 1, 530, 1, 100, counter)
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

fn item(guid: ObjectGuid, entry: u32) -> Item {
    let mut item = Item::default();
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry);
    item
}

fn player_entity(
    counter: i64,
    map_id: u32,
    instance_id: u32,
    in_world: bool,
    attacking: Option<ObjectGuid>,
) -> Player {
    let mut player = Player::new(Some(counter as u64), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid(HighGuid::Player, counter));
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    if in_world {
        player.unit_mut().world_mut().object_mut().add_to_world();
    }
    player.unit_mut().set_attacking(attacking);
    player
}

fn run_player_stack_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
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

#[derive(Default)]
struct TestMapSource {
    map_id: u32,
    instance_id: u32,
    records: std::collections::HashMap<ObjectGuid, MapObjectRecord>,
}

impl ObjectAccessorMapSource for TestMapSource {
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
        self.records.get(&guid)
    }
}

fn typed_creature_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    level: u8,
) -> (ObjectGuid, MapObjectRecord) {
    let mut creature = Creature::new(false);
    let creature_guid = guid(HighGuid::Creature, counter);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    creature.unit_mut().set_level(level);

    (
        creature_guid,
        MapObjectRecord::new_creature(creature).unwrap(),
    )
}

fn typed_game_object_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    linked_trap: ObjectGuid,
) -> (ObjectGuid, MapObjectRecord) {
    let mut game_object = GameObject::new();
    let game_object_guid = guid(HighGuid::GameObject, counter);
    game_object
        .world_mut()
        .object_mut()
        .create(game_object_guid);
    game_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    game_object.set_linked_trap_like_cpp(linked_trap);

    (
        game_object_guid,
        MapObjectRecord::new_game_object(game_object).unwrap(),
    )
}

fn typed_dynamic_object_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    caster: ObjectGuid,
    spell_id: i32,
) -> (ObjectGuid, MapObjectRecord) {
    let mut dynamic_object = DynamicObject::new(false);
    let dynamic_object_guid = guid(HighGuid::DynamicObject, counter);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(dynamic_object_guid);
    dynamic_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    dynamic_object.set_caster_guid(caster);
    dynamic_object.set_spell_id(spell_id);
    dynamic_object.set_radius(12.5);

    (
        dynamic_object_guid,
        MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
    )
}

fn typed_area_trigger_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    caster: ObjectGuid,
    spell_id: i32,
) -> (ObjectGuid, MapObjectRecord) {
    let mut area_trigger = AreaTrigger::new();
    let area_trigger_guid = guid(HighGuid::AreaTrigger, counter);
    area_trigger
        .world_mut()
        .object_mut()
        .create(area_trigger_guid);
    area_trigger
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    area_trigger.set_caster_guid(caster);
    area_trigger.set_spell_id(spell_id);
    area_trigger.set_duration(4_500);

    (
        area_trigger_guid,
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
}

fn typed_corpse_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    owner: ObjectGuid,
    ghost_time: i64,
) -> (ObjectGuid, MapObjectRecord) {
    let mut corpse = Corpse::new_at(crate::CorpseType::ResurrectablePve, ghost_time);
    let corpse_guid = guid(HighGuid::Corpse, counter);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().set_map(map_id, instance_id).unwrap();
    corpse.world_mut().relocate(Position::xyz(10.0, 20.0, 30.0));
    corpse.set_owner_guid(owner);
    corpse.set_display_id(11_111);

    (corpse_guid, MapObjectRecord::new_corpse(corpse).unwrap())
}

fn typed_scene_object_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    creator: ObjectGuid,
    script_package_id: i32,
) -> (ObjectGuid, MapObjectRecord) {
    let mut scene_object = SceneObject::new();
    let scene_object_guid = guid(HighGuid::SceneObject, counter);
    scene_object
        .world_mut()
        .object_mut()
        .create(scene_object_guid);
    scene_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    scene_object
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    scene_object.set_created_by(creator);
    scene_object.set_script_package_id(script_package_id);
    scene_object.set_scene_type(crate::SceneType::PetBattle);

    (
        scene_object_guid,
        MapObjectRecord::new_scene_object(scene_object).unwrap(),
    )
}

fn typed_conversation_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    creator: ObjectGuid,
    duration_ms: i32,
) -> (ObjectGuid, MapObjectRecord) {
    let mut conversation = Conversation::new();
    let conversation_guid = guid(HighGuid::Conversation, counter);
    conversation
        .world_mut()
        .object_mut()
        .create(conversation_guid);
    conversation
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    conversation
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    conversation.set_creator_guid(creator);
    conversation.set_duration_ms(duration_ms);
    conversation.set_texture_kit_id(77);
    conversation.add_line(crate::ConversationLine {
        conversation_line_id: 9901,
        start_time: 12,
        ui_camera_id: 34,
        actor_index: 1,
        flags: 2,
    });

    (
        conversation_guid,
        MapObjectRecord::new_conversation(conversation).unwrap(),
    )
}

fn typed_pet_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    owner: ObjectGuid,
    specialization: u16,
) -> (ObjectGuid, MapObjectRecord) {
    let mut pet = Pet::new(owner, crate::PetType::Hunter);
    let pet_guid = guid(HighGuid::Pet, counter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    pet.set_duration(12_345);
    pet.set_specialization(specialization);

    (pet_guid, MapObjectRecord::new_pet(pet).unwrap())
}

fn typed_transport_record(
    map_id: u32,
    instance_id: u32,
    counter: i64,
    passenger: ObjectGuid,
) -> (ObjectGuid, MapObjectRecord) {
    let mut transport = Transport::new();
    let transport_guid = guid(HighGuid::Transport, counter);
    transport.world_mut().object_mut().create(transport_guid);
    transport.world_mut().set_map(map_id, instance_id).unwrap();
    transport
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    transport.world_mut().object_mut().add_to_world();
    transport
        .game_object_mut()
        .set_linked_trap_like_cpp(guid(HighGuid::GameObject, 9_901));
    transport.set_movement_state(crate::TransportMovementState::WaitingOnPauseWaypoint);
    transport.set_path_progress_ms(4_321);
    transport.set_position_change_timer_ms(222);
    assert!(transport.add_passenger(passenger));

    (
        transport_guid,
        MapObjectRecord::new_transport(transport).unwrap(),
    )
}

mod scenarios_1;
mod scenarios_2;
