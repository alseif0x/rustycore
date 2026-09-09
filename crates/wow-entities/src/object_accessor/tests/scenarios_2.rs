//! Object accessor records and lookups regression scenarios, part 2 of 2.
//!
//! Moved out of the object_accessor.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn typed_map_source_lookup_rejects_generic_non_creature_gameobject_fallbacks_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let generic_dynamic = world_object(HighGuid::DynamicObject, 530, 1, true);
    let generic_dynamic_guid = generic_dynamic.guid();
    let generic_area_trigger = world_object(HighGuid::AreaTrigger, 530, 1, true);
    let generic_area_trigger_guid = generic_area_trigger.guid();
    let generic_corpse = world_object(HighGuid::Corpse, 530, 1, true);
    let generic_corpse_guid = generic_corpse.guid();
    let generic_scene = world_object(HighGuid::SceneObject, 530, 1, true);
    let generic_scene_guid = generic_scene.guid();
    let generic_conversation = world_object(HighGuid::Conversation, 530, 1, true);
    let generic_conversation_guid = generic_conversation.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(
        generic_dynamic_guid,
        MapObjectRecord::new(AccessorObjectKind::DynamicObject, generic_dynamic).unwrap(),
    );
    source.records.insert(
        generic_area_trigger_guid,
        MapObjectRecord::new(AccessorObjectKind::AreaTrigger, generic_area_trigger).unwrap(),
    );
    source.records.insert(
        generic_corpse_guid,
        MapObjectRecord::new(AccessorObjectKind::Corpse, generic_corpse).unwrap(),
    );
    source.records.insert(
        generic_scene_guid,
        MapObjectRecord::new(AccessorObjectKind::SceneObject, generic_scene).unwrap(),
    );
    source.records.insert(
        generic_conversation_guid,
        MapObjectRecord::new(AccessorObjectKind::Conversation, generic_conversation).unwrap(),
    );

    assert_eq!(
        accessor
            .get_dynamic_object_from_map_source(&context, &source, generic_dynamic_guid)
            .unwrap()
            .guid(),
        generic_dynamic_guid
    );
    assert_eq!(
        accessor
            .get_area_trigger_from_map_source(&context, &source, generic_area_trigger_guid)
            .unwrap()
            .guid(),
        generic_area_trigger_guid
    );
    assert_eq!(
        accessor
            .get_corpse_from_map_source(&context, &source, generic_corpse_guid)
            .unwrap()
            .guid(),
        generic_corpse_guid
    );
    assert_eq!(
        accessor
            .get_scene_object_from_map_source(&context, &source, generic_scene_guid)
            .unwrap()
            .guid(),
        generic_scene_guid
    );
    assert_eq!(
        accessor
            .get_conversation_from_map_source(&context, &source, generic_conversation_guid)
            .unwrap()
            .guid(),
        generic_conversation_guid
    );

    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, generic_dynamic_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_area_trigger_from_map_source(&context, &source, generic_area_trigger_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_corpse_from_map_source(&context, &source, generic_corpse_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_scene_object_from_map_source(&context, &source, generic_scene_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_conversation_from_map_source(&context, &source, generic_conversation_guid)
            .is_none()
    );
}

#[test]
fn typed_map_source_lookup_does_not_cross_non_creature_gameobject_kinds_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let caster = guid(HighGuid::Creature, 4120);
    let creator = guid(HighGuid::Player, 4121);
    let (dynamic_guid, dynamic_record) = typed_dynamic_object_record(530, 1, 4122, caster, 1);
    let (area_trigger_guid, area_trigger_record) =
        typed_area_trigger_record(530, 1, 4123, caster, 2);
    let (corpse_guid, corpse_record) = typed_corpse_record(530, 1, 4124, creator, 3);
    let (scene_guid, scene_record) = typed_scene_object_record(530, 1, 4125, creator, 4);
    let (conversation_guid, conversation_record) =
        typed_conversation_record(530, 1, 4126, creator, 5);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(dynamic_guid, dynamic_record);
    source
        .records
        .insert(area_trigger_guid, area_trigger_record);
    source.records.insert(corpse_guid, corpse_record);
    source.records.insert(scene_guid, scene_record);
    source
        .records
        .insert(conversation_guid, conversation_record);

    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, area_trigger_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_area_trigger_from_map_source(&context, &source, dynamic_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_corpse_from_map_source(&context, &source, scene_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_scene_object_from_map_source(&context, &source, corpse_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_conversation_from_map_source(&context, &source, dynamic_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, conversation_guid)
            .is_none()
    );
}

#[test]
fn typed_map_source_lookup_requires_source_and_context_same_map_for_non_creature_gameobject_like_cpp()
 {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let (dynamic_guid, dynamic_record) =
        typed_dynamic_object_record(530, 1, 4130, guid(HighGuid::Creature, 4131), 7);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(dynamic_guid, dynamic_record);

    source.map_id = 571;
    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
            .is_none()
    );
    source.map_id = 530;
    source.instance_id = 2;
    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
            .is_none()
    );
    source.instance_id = 1;

    let wrong_context = world_object(HighGuid::Player, 571, 1, true);
    assert!(
        accessor
            .get_typed_dynamic_object_from_map_source(&wrong_context, &source, dynamic_guid)
            .is_none()
    );
}

#[test]
fn map_source_lookup_reads_canonical_source_without_bridge_storage() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let creature = world_object(HighGuid::Creature, 530, 1, true);
    let creature_guid = creature.guid();
    let record = MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, record);

    assert_eq!(
        accessor
            .get_world_object_from_map_source(&context, &source, creature_guid)
            .unwrap()
            .guid(),
        creature_guid
    );
    assert!(matches!(
        accessor.get_object_ref_by_type_mask_from_map_source(
            &context,
            &source,
            creature_guid,
            TypeMask::UNIT
        ),
        Some(AccessorObjectRef::WorldObject(object)) if object.guid() == creature_guid
    ));

    source.instance_id = 2;
    assert!(
        accessor
            .get_world_object_from_map_source(&context, &source, creature_guid)
            .is_none()
    );
}

#[test]
fn get_player_requires_same_map_like_cpp_get_player_map() {
    let mut accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Creature, 1, 0, true);
    let same_map_player = world_object(HighGuid::Player, 1, 0, true);
    let same_guid = same_map_player.guid();
    let mut other_map_player = world_object(HighGuid::Player, 2, 0, true);
    other_map_player
        .object_mut()
        .create(ObjectGuid::create_global(HighGuid::Player, 0, 2));
    let other_guid = other_map_player.guid();

    accessor.add_player("anduin", same_map_player).unwrap();
    accessor.add_player("baine", other_map_player).unwrap();

    assert!(accessor.get_player(&context, same_guid).is_some());
    assert!(accessor.get_player(&context, other_guid).is_none());
}

#[test]
fn world_object_dispatches_by_high_guid_to_map_source() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let creature = world_object(HighGuid::Creature, 530, 1, true);
    let gameobject = world_object(HighGuid::GameObject, 530, 1, true);
    let creature_guid = creature.guid();
    let gameobject_guid = gameobject.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };

    source.records.insert(
        creature_guid,
        MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap(),
    );
    source.records.insert(
        gameobject_guid,
        MapObjectRecord::new(AccessorObjectKind::GameObject, gameobject).unwrap(),
    );

    assert_eq!(
        accessor
            .get_world_object_from_map_source(&context, &source, creature_guid)
            .unwrap()
            .guid(),
        creature_guid
    );
    assert_eq!(
        accessor
            .get_world_object_from_map_source(&context, &source, gameobject_guid)
            .unwrap()
            .guid(),
        gameobject_guid
    );
}

#[test]
fn object_by_type_mask_matches_cpp_dispatch_rules_with_map_source() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let creature = world_object(HighGuid::Creature, 530, 1, true);
    let creature_guid = creature.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(
        creature_guid,
        MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap(),
    );

    assert!(
        accessor
            .get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                creature_guid,
                TypeMask::UNIT
            )
            .is_some()
    );
    assert!(
        accessor
            .get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                creature_guid,
                TypeMask::GAME_OBJECT
            )
            .is_none()
    );
    assert!(
        accessor
            .get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                creature_guid,
                TypeMask::PLAYER
            )
            .is_none()
    );
}

#[test]
fn type_mask_item_uses_player_inventory_like_cpp_branch() {
    let mut accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let player_guid = context.guid();
    let item_guid = ObjectGuid::create_item(1, 77);
    let mut inventory = PlayerInventoryStorage::default();
    inventory.items[0] = Some(item_guid);
    let item = item(item_guid, 6948);

    accessor
        .add_player_with_inventory_and_items("valeera", context.clone(), inventory, [item])
        .unwrap();

    let found = accessor.get_object_ref_by_type_mask(&context, item_guid, TypeMask::ITEM);
    match found {
        Some(AccessorObjectRef::Item(item)) => {
            assert_eq!(item.object().guid(), item_guid);
            assert_eq!(item.object().entry(), 6948);
        }
        other => panic!("expected item ref, got {other:?}"),
    }
    assert!(
        accessor
            .get_object_by_type_mask(&context, item_guid, TypeMask::ITEM)
            .is_none()
    );
    assert!(
        accessor
            .get_object_ref_by_type_mask(&context, item_guid, TypeMask::UNIT)
            .is_none()
    );

    let non_player_context = world_object(HighGuid::Creature, 530, 1, true);
    assert!(
        accessor
            .get_object_ref_by_type_mask(&non_player_context, item_guid, TypeMask::ITEM)
            .is_none()
    );
    assert!(accessor.player_inventory_mut(player_guid).is_some());
}

#[test]
fn type_mask_item_requires_registered_item_object_like_cpp_item_pointer() {
    let mut accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let item_guid = ObjectGuid::create_item(1, 77);
    let mut inventory = PlayerInventoryStorage::default();
    inventory.items[0] = Some(item_guid);

    accessor
        .add_player_with_inventory("valeera", context.clone(), inventory)
        .unwrap();

    assert!(
        accessor
            .get_object_ref_by_type_mask(&context, item_guid, TypeMask::ITEM)
            .is_none()
    );
}

#[test]
fn corpse_is_directly_accessible_but_not_returned_by_type_mask_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let corpse = world_object(HighGuid::Corpse, 530, 1, true);
    let corpse_guid = corpse.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };

    source.records.insert(
        corpse_guid,
        MapObjectRecord::new(AccessorObjectKind::Corpse, corpse).unwrap(),
    );

    assert_eq!(
        accessor
            .get_corpse_from_map_source(&context, &source, corpse_guid)
            .unwrap()
            .guid(),
        corpse_guid
    );
    assert!(
        accessor
            .get_world_object_from_map_source(&context, &source, corpse_guid)
            .is_some()
    );
    assert!(
        accessor
            .get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                corpse_guid,
                TypeMask::CORPSE
            )
            .is_none()
    );
}

#[test]
fn typed_pet_map_source_preserves_body_and_generic_pet_lookup_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let owner = guid(HighGuid::Player, 4_120);
    let (pet_guid, pet_record) = typed_pet_record(530, 1, 4_121, owner, 77);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(pet_guid, pet_record);

    let generic = accessor
        .get_pet_from_map_source(&context, &source, pet_guid)
        .unwrap();
    assert_eq!(generic.guid(), pet_guid);

    let pet = accessor
        .get_typed_pet_from_map_source(&context, &source, pet_guid)
        .unwrap();
    assert_eq!(pet.owner_guid(), owner);
    assert_eq!(pet.pet_type(), crate::PetType::Hunter);
    assert_eq!(pet.specialization(), 77);
    assert_eq!(pet.duration_ms(), 12_345);
    assert_eq!(pet.creature().unit().world().guid(), pet_guid);
}

#[test]
fn typed_transport_map_source_preserves_body_and_embedded_gameobject_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let passenger = guid(HighGuid::Player, 4_130);
    let (transport_guid, transport_record) = typed_transport_record(530, 1, 4_131, passenger);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(transport_guid, transport_record);

    let game_object = accessor
        .get_typed_game_object_from_map_source(&context, &source, transport_guid)
        .unwrap();
    assert_eq!(game_object.world().guid(), transport_guid);
    assert_eq!(
        game_object.linked_trap_guid_like_cpp(),
        guid(HighGuid::GameObject, 9_901)
    );

    let transport = accessor
        .get_typed_transport_from_map_source(&context, &source, transport_guid)
        .unwrap();
    assert_eq!(
        transport.movement_state(),
        crate::TransportMovementState::WaitingOnPauseWaypoint
    );
    assert_eq!(transport.path_progress_ms(), 4_321);
    assert_eq!(transport.position_change_timer_ms(), 222);
    assert!(transport.passengers().contains(&passenger));
    assert_eq!(transport.world().guid(), transport_guid);
}

#[test]
fn typed_pet_transport_helpers_reject_generic_worldobject_fallbacks_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let generic_pet = world_object(HighGuid::Pet, 530, 1, true);
    let generic_pet_guid = generic_pet.guid();
    let generic_transport = world_object(HighGuid::Transport, 530, 1, true);
    let generic_transport_guid = generic_transport.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(
        generic_pet_guid,
        MapObjectRecord::new(AccessorObjectKind::Pet, generic_pet).unwrap(),
    );
    source.records.insert(
        generic_transport_guid,
        MapObjectRecord::new(AccessorObjectKind::Transport, generic_transport).unwrap(),
    );

    assert_eq!(
        accessor
            .get_pet_from_map_source(&context, &source, generic_pet_guid)
            .unwrap()
            .guid(),
        generic_pet_guid
    );
    assert_eq!(
        accessor
            .get_world_object_from_map_source(&context, &source, generic_transport_guid)
            .unwrap()
            .guid(),
        generic_transport_guid
    );
    assert_eq!(
        accessor
            .get_game_object_from_map_source(&context, &source, generic_transport_guid)
            .unwrap()
            .guid(),
        generic_transport_guid
    );
    assert!(
        accessor
            .get_typed_pet_from_map_source(&context, &source, generic_pet_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, generic_transport_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_game_object_from_map_source(&context, &source, generic_transport_guid)
            .is_none()
    );
}

#[test]
fn typed_pet_transport_lookup_does_not_cross_kinds_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let owner = guid(HighGuid::Player, 4_140);
    let (creature_guid, creature_record) = typed_creature_record(530, 1, 4_141, 60);
    let (pet_guid, pet_record) = typed_pet_record(530, 1, 4_142, owner, 11);
    let (game_object_guid, game_object_record) =
        typed_game_object_record(530, 1, 4_143, guid(HighGuid::GameObject, 4_144));
    let (transport_guid, transport_record) =
        typed_transport_record(530, 1, 4_145, guid(HighGuid::Player, 4_146));
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, creature_record);
    source.records.insert(pet_guid, pet_record);
    source.records.insert(game_object_guid, game_object_record);
    source.records.insert(transport_guid, transport_record);

    assert!(
        accessor
            .get_typed_pet_from_map_source(&context, &source, creature_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_creature_from_map_source(&context, &source, pet_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, game_object_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, transport_guid)
            .is_some()
    );
    assert!(
        accessor
            .get_typed_game_object_from_map_source(&context, &source, transport_guid)
            .is_some()
    );
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, game_object_guid)
            .is_none()
    );
}

#[test]
fn typed_transport_lookup_requires_source_and_context_same_map_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let (transport_guid, transport_record) =
        typed_transport_record(530, 1, 4_150, guid(HighGuid::Player, 4_151));
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(transport_guid, transport_record);

    source.map_id = 571;
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, transport_guid)
            .is_none()
    );
    source.map_id = 530;
    source.instance_id = 2;
    assert!(
        accessor
            .get_typed_transport_from_map_source(&context, &source, transport_guid)
            .is_none()
    );
    source.instance_id = 1;

    let wrong_context = world_object(HighGuid::Player, 571, 1, true);
    assert!(
        accessor
            .get_typed_transport_from_map_source(&wrong_context, &source, transport_guid)
            .is_none()
    );
}

#[test]
fn unit_and_creature_or_pet_or_vehicle_helpers_match_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let pet = world_object(HighGuid::Pet, 530, 1, true);
    let pet_guid = pet.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(
        pet_guid,
        MapObjectRecord::new(AccessorObjectKind::Pet, pet).unwrap(),
    );

    assert_eq!(
        accessor
            .get_unit_from_map_source(&context, &source, pet_guid)
            .unwrap()
            .guid(),
        pet_guid
    );
    assert_eq!(
        accessor
            .get_creature_or_pet_or_vehicle_from_map_source(&context, &source, pet_guid)
            .unwrap()
            .guid(),
        pet_guid
    );
}
