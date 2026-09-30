// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed projections must preserve the distinction between kind and body.

use super::*;
use wow_core::guid::HighGuid;
use wow_entities::{CorpseType, PetType};

fn bind(world: &mut WorldObject, high: HighGuid) {
    let guid = match high {
        HighGuid::Player => ObjectGuid::create_global(high, 0, 42),
        HighGuid::Transport => ObjectGuid::create_transport(high, 42),
        _ => ObjectGuid::create_world_object(high, 0, 1, 571, 7, 100, 42),
    };
    world.object_mut().create(guid);
    world.set_map(571, 7).unwrap();
}

fn typed_records() -> Vec<MapObjectRecord> {
    let mut records = Vec::new();
    let mut area_trigger = AreaTrigger::new();
    bind(area_trigger.world_mut(), HighGuid::AreaTrigger);
    records.push(MapObjectRecord::new_area_trigger(area_trigger).unwrap());
    let mut conversation = Conversation::new();
    bind(conversation.world_mut(), HighGuid::Conversation);
    records.push(MapObjectRecord::new_conversation(conversation).unwrap());
    let mut corpse = Corpse::new_at(CorpseType::Bones, 0);
    bind(corpse.world_mut(), HighGuid::Corpse);
    records.push(MapObjectRecord::new_corpse(corpse).unwrap());
    let mut creature = Creature::new(false);
    bind(creature.unit_mut().world_mut(), HighGuid::Creature);
    records.push(MapObjectRecord::new_creature(creature).unwrap());
    let mut dynamic_object = DynamicObject::new(false);
    bind(dynamic_object.world_mut(), HighGuid::DynamicObject);
    records.push(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap());
    let mut game_object = GameObject::new();
    bind(game_object.world_mut(), HighGuid::GameObject);
    records.push(MapObjectRecord::new_game_object(game_object).unwrap());
    let mut pet = Pet::new(ObjectGuid::EMPTY, PetType::Hunter);
    bind(pet.creature_mut().unit_mut().world_mut(), HighGuid::Pet);
    records.push(MapObjectRecord::new_pet(pet).unwrap());
    let mut player = Player::new(Some(7), false);
    bind(player.unit_mut().world_mut(), HighGuid::Player);
    records.push(MapObjectRecord::new_player(player).unwrap());
    let mut scene_object = SceneObject::new();
    bind(scene_object.world_mut(), HighGuid::SceneObject);
    records.push(MapObjectRecord::new_scene_object(scene_object).unwrap());
    let mut transport = Transport::new();
    bind(transport.world_mut(), HighGuid::Transport);
    records.push(MapObjectRecord::new_transport(transport).unwrap());
    records
}

fn typed_presence(view: &ObjectRef<'_>) -> [bool; 10] {
    [
        view.area_trigger().is_some(),
        view.conversation().is_some(),
        view.corpse().is_some(),
        view.creature().is_some(),
        view.dynamic_object().is_some(),
        view.game_object().is_some(),
        view.pet().is_some(),
        view.player().is_some(),
        view.scene_object().is_some(),
        view.transport().is_some(),
    ]
}

fn mutable_presence(view: &mut ObjectMut<'_>) -> [bool; 10] {
    [
        view.reborrow().area_trigger_mut().is_some(),
        view.reborrow().conversation_mut().is_some(),
        view.reborrow().corpse_mut().is_some(),
        view.reborrow().creature_mut().is_some(),
        view.reborrow().dynamic_object_mut().is_some(),
        view.reborrow().game_object_mut().is_some(),
        view.reborrow().pet_mut().is_some(),
        view.reborrow().player_mut().is_some(),
        view.reborrow().scene_object_mut().is_some(),
        view.reborrow().transport_mut().is_some(),
    ]
}

#[test]
fn typed_and_generic_bodies_keep_exact_kind_gates_and_transport_alias() {
    let kinds = [
        AccessorObjectKind::AreaTrigger,
        AccessorObjectKind::Conversation,
        AccessorObjectKind::Corpse,
        AccessorObjectKind::Creature,
        AccessorObjectKind::DynamicObject,
        AccessorObjectKind::GameObject,
        AccessorObjectKind::Pet,
        AccessorObjectKind::Player,
        AccessorObjectKind::SceneObject,
        AccessorObjectKind::Transport,
    ];
    for (index, mut record) in typed_records().into_iter().enumerate() {
        let view = ObjectRef::new(&record);
        let mut expected = [false; 10];
        expected[index] = true;
        if index == 9 {
            expected[5] = true;
            assert!(std::ptr::eq(
                view.game_object().unwrap(),
                view.transport().unwrap().game_object(),
            ));
        }
        assert_eq!(view.kind(), kinds[index]);
        assert_eq!(typed_presence(&view), expected);
        assert!(std::ptr::eq(view.object(), record.object()));
        let unit_body = matches!(index, 3 | 6 | 7);
        assert_eq!(view.unit().is_some(), unit_body);
        assert_eq!(view.is_unit_owner(), unit_body);
        assert_eq!(view.charmer_guid(), None);

        // A declared typed kind does not manufacture a typed body.
        let mut generic = MapObjectRecord::new(view.kind(), view.object().clone()).unwrap();
        let generic_view = ObjectRef::new(&generic);
        assert_eq!(generic_view.kind(), kinds[index]);
        assert_eq!(typed_presence(&generic_view), [false; 10]);
        assert!(generic_view.unit().is_none());
        assert!(!generic_view.is_unit_owner());
        assert_eq!(generic_view.charmer_guid(), None);
        let mut generic_mut = ObjectMut::new(&mut generic);
        assert_eq!(mutable_presence(&mut generic_mut), [false; 10]);
        assert!(generic_mut.unit_mut().is_none());

        let mut mutable = ObjectMut::new(&mut record);
        assert_eq!(mutable_presence(&mut mutable), expected);
        assert_eq!(mutable.unit_mut().is_some(), unit_body);
    }
}

#[test]
fn reborrow_preserves_sequential_typed_and_common_writes_on_one_body() {
    let mut record = typed_records().remove(3);
    let mut view = ObjectMut::new(&mut record);
    view.reborrow().object_mut().object_mut().set_is_new_object(true);
    assert!(view.as_ref().object().object().is_new_object());
    {
        let creature = view.reborrow().creature_mut().unwrap();
        assert!(creature.unit().world().object().is_new_object());
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(75);
        creature.unit_mut().world_mut().object_mut().set_is_new_object(false);
    }
    assert!(!view.as_ref().object().object().is_new_object());
    assert_eq!(view.as_ref().creature().unwrap().current_health(), 75);
    view.reborrow().unit_mut().unwrap().set_health(25);
    assert_eq!(view.as_ref().creature().unwrap().current_health(), 25);
    // The terminal projection consumes the view and retains the owner borrow.
    let creature = view.creature_mut().unwrap();
    creature.unit_mut().set_health(5);
    assert_eq!(record.creature().unwrap().current_health(), 5);
}

#[test]
fn immutable_projection_outlives_the_temporary_view() {
    fn creature(record: &MapObjectRecord) -> &Creature {
        ObjectRef::new(record).creature().unwrap()
    }
    let record = typed_records().remove(3);
    let projected = creature(&record);
    assert!(std::ptr::eq(projected, record.creature().unwrap()));
}

#[test]
fn charmer_projection_includes_creature_and_pet_but_excludes_player() {
    let charmer = ObjectGuid::create_global(HighGuid::Player, 0, 99);
    for mut record in typed_records() {
        let kind = record.kind();
        let view = ObjectMut::new(&mut record);
        if let Some(unit) = view.unit_mut() {
            unit.subsystems_mut().control.set_charmer(charmer, true);
        }
        let expected = matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
            .then_some(charmer);
        assert_eq!(ObjectRef::new(&record).charmer_guid(), expected);
    }
}

#[test]
fn transport_gameobject_writes_share_the_same_body() {
    let mut record = typed_records().remove(9);
    let mut view = ObjectMut::new(&mut record);
    view.reborrow()
        .game_object_mut()
        .unwrap()
        .world_mut()
        .object_mut()
        .set_is_new_object(true);
    let transport = view.reborrow().transport_mut().unwrap();
    assert!(transport.world().object().is_new_object());
    transport.world_mut().object_mut().set_is_new_object(false);
    assert!(!view.as_ref().game_object().unwrap().world().object().is_new_object());
}
