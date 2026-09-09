//! Object accessor records and lookups regression scenarios, part 1 of 2.
//!
//! Moved out of the object_accessor.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn player_name_normalization_matches_cpp_shape() {
    assert_eq!(normalize_player_name("thrall"), Some("Thrall".to_string()));
    assert_eq!(normalize_player_name("THRALL"), Some("Thrall".to_string()));
    assert_eq!(normalize_player_name(""), None);
}

#[test]
fn global_player_lookup_distinguishes_connected_and_in_world() {
    let mut accessor = ObjectAccessor::default();
    let player = world_object(HighGuid::Player, 1, 0, false);
    let player_guid = player.guid();

    accessor.add_player("jaina", player).unwrap();
    assert!(accessor.find_connected_player(player_guid).is_some());
    assert!(accessor.find_connected_player_by_name("JAINA").is_some());
    assert!(accessor.find_player(player_guid).is_none());
    assert!(accessor.find_player_by_name("jaina").is_none());

    let mut in_world = world_object(HighGuid::Player, 1, 0, true);
    in_world.object_mut().create(player_guid);
    accessor.add_player("jaina", in_world).unwrap();
    assert!(accessor.find_player(player_guid).is_some());
    assert_eq!(accessor.save_all_players_count(), 1);
}

#[test]
fn map_object_record_can_store_typed_gameobject_like_cpp() {
    let mut game_object = GameObject::new();
    let guid = guid(HighGuid::GameObject, 77);
    game_object.world_mut().object_mut().create(guid);
    game_object.world_mut().object_mut().set_entry(123);
    game_object.world_mut().set_map(571, 0).unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    game_object.set_created_by(ObjectGuid::create_player(1, 42));

    let mut record = MapObjectRecord::new_game_object(game_object).unwrap();

    assert_eq!(record.kind(), AccessorObjectKind::GameObject);
    assert_eq!(record.object().guid(), guid);
    assert_eq!(
        record.game_object().unwrap().owner_guid(),
        ObjectGuid::create_player(1, 42)
    );
    record.object_mut().relocate(Position::xyz(4.0, 5.0, 6.0));
    assert_eq!(record.game_object().unwrap().world().position().x, 4.0);
}

#[test]
fn map_object_record_can_store_typed_creature_like_cpp() {
    let mut creature = Creature::new(false);
    let guid = guid(HighGuid::Creature, 78);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(321);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_level(42);

    let mut record = MapObjectRecord::new_creature(creature).unwrap();

    assert_eq!(record.kind(), AccessorObjectKind::Creature);
    assert_eq!(record.object().guid(), guid);
    assert_eq!(record.creature().unwrap().unit().data().level, 42);
    record
        .creature_mut()
        .unwrap()
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(4.0, 5.0, 6.0));
    assert_eq!(record.object().position().x, 4.0);
}

#[test]
fn map_object_record_can_store_typed_player_like_cpp() {
    let mut player = Player::new(Some(7), false);
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = guid(HighGuid::Creature, 77);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    player.unit_mut().set_attacking(Some(victim_guid));

    let mut record = MapObjectRecord::new_player(player).unwrap();

    assert_eq!(record.kind(), AccessorObjectKind::Player);
    assert_eq!(record.object().guid(), player_guid);
    assert_eq!(
        record.player().unwrap().unit().attacking(),
        Some(victim_guid)
    );
    record.player_mut().unwrap().unit_mut().set_attacking(None);
    assert_eq!(record.player().unwrap().unit().attacking(), None);
}

#[test]
fn typed_global_player_lookup_preserves_player_body_like_cpp_hashmap_holder() {
    run_player_stack_test(|| {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let target = guid(HighGuid::Creature, 4_201);
        let player = player_entity(4_200, 530, 1, true, Some(target));
        let player_guid = player.unit().world().guid();
        let source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };

        accessor.add_player_entity("anduin", player).unwrap();

        assert_eq!(
            accessor
                .find_connected_player_entity(player_guid)
                .unwrap()
                .unit()
                .attacking(),
            Some(target)
        );
        assert_eq!(
            accessor
                .find_player_entity(player_guid)
                .unwrap()
                .unit()
                .attacking(),
            Some(target)
        );
        assert_eq!(
            accessor
                .get_player_entity(&context, player_guid)
                .unwrap()
                .unit()
                .attacking(),
            Some(target)
        );
        assert_eq!(
            accessor
                .get_typed_player_from_map_source(&context, &source, player_guid)
                .unwrap()
                .unit()
                .attacking(),
            Some(target)
        );
    });
}

#[test]
fn typed_global_player_lookup_rejects_legacy_and_cpp_early_returns() {
    run_player_stack_test(|| {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let legacy = world_object(HighGuid::Player, 530, 1, true);
        let legacy_guid = legacy.guid();
        let not_in_world = player_entity(4_210, 530, 1, false, None);
        let not_in_world_guid = not_in_world.unit().world().guid();
        let other_map = player_entity(4_211, 571, 1, true, None);
        let other_map_guid = other_map.unit().world().guid();
        let other_instance = player_entity(4_212, 530, 2, true, None);
        let other_instance_guid = other_instance.unit().world().guid();

        accessor.add_player("legacy", legacy).unwrap();
        accessor.add_player_entity("ghost", not_in_world).unwrap();
        accessor.add_player_entity("map", other_map).unwrap();
        accessor
            .add_player_entity("instance", other_instance)
            .unwrap();

        assert!(accessor.get_player(&context, legacy_guid).is_some());
        assert!(accessor.get_player_entity(&context, legacy_guid).is_none());
        assert!(
            accessor
                .find_connected_player_entity(not_in_world_guid)
                .is_some()
        );
        assert!(accessor.find_player_entity(not_in_world_guid).is_none());
        assert!(
            accessor
                .get_player_entity(&context, not_in_world_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_player_entity(&context, other_map_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_player_entity(&context, other_instance_guid)
                .is_none()
        );
        assert!(
            accessor
                .get_player_entity(&context, guid(HighGuid::Creature, 4_213))
                .is_none()
        );
    });
}

#[test]
fn typed_player_map_source_ignores_map_object_record_player_like_cpp_hashmap_holder() {
    run_player_stack_test(|| {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let source_only_player = player_entity(4_220, 530, 1, true, None);
        let source_only_guid = source_only_player.unit().world().guid();
        let global_player = player_entity(4_221, 530, 1, true, None);
        let global_guid = global_player.unit().world().guid();
        let mut source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };
        source.records.insert(
            source_only_guid,
            MapObjectRecord::new_player(source_only_player).unwrap(),
        );

        assert!(
            accessor
                .get_typed_player_from_map_source(&context, &source, source_only_guid)
                .is_none()
        );

        accessor
            .add_player_entity("tyrande", global_player)
            .unwrap();
        assert!(
            accessor
                .get_typed_player_from_map_source(&context, &source, global_guid)
                .is_some()
        );
        source.map_id = 571;
        assert!(
            accessor
                .get_typed_player_from_map_source(&context, &source, global_guid)
                .is_none()
        );
        source.map_id = 530;
        source.instance_id = 2;
        assert!(
            accessor
                .get_typed_player_from_map_source(&context, &source, global_guid)
                .is_none()
        );
    });
}

#[test]
fn player_worldobject_dispatch_and_type_mask_stay_on_global_registry_like_cpp() {
    run_player_stack_test(|| {
        let mut accessor = ObjectAccessor::default();
        let context = world_object(HighGuid::Player, 530, 1, true);
        let player = player_entity(4_230, 530, 1, true, None);
        let player_guid = player.unit().world().guid();
        let source = TestMapSource {
            map_id: 530,
            instance_id: 1,
            records: std::collections::HashMap::new(),
        };

        accessor.add_player_entity("uther", player).unwrap();

        assert_eq!(
            accessor
                .get_world_object_from_map_source(&context, &source, player_guid)
                .unwrap()
                .guid(),
            player_guid
        );
        assert!(matches!(
            accessor.get_object_ref_by_type_mask_from_map_source(
                &context,
                &source,
                player_guid,
                TypeMask::PLAYER
            ),
            Some(AccessorObjectRef::WorldObject(object)) if object.guid() == player_guid
        ));
        assert_eq!(
            accessor
                .get_unit_from_map_source(&context, &source, player_guid)
                .unwrap()
                .guid(),
            player_guid
        );
    });
}

#[test]
fn typed_player_registration_validates_player_high_guid() {
    run_player_stack_test(|| {
        let mut player = player_entity(4_240, 530, 1, true, None);
        let creature_guid = guid(HighGuid::Creature, 4_241);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);

        let error = AccessorPlayer::new_player("bad", player).unwrap_err();
        assert_eq!(
            error,
            ObjectAccessorError::WrongGuidKind {
                guid: creature_guid,
                expected: AccessorObjectKind::Player,
            }
        );
    });
}

#[test]
fn save_all_players_with_invokes_sink_once_per_registered_player() {
    let mut accessor = ObjectAccessor::default();
    let player_a = world_object(HighGuid::Player, 1, 0, true);
    let guid_a = player_a.guid();
    let mut player_b = world_object(HighGuid::Player, 1, 0, true);
    player_b
        .object_mut()
        .create(ObjectGuid::create_global(HighGuid::Player, 0, 2));
    let guid_b = player_b.guid();

    accessor.add_player("jaina", player_a).unwrap();
    accessor.add_player("thrall", player_b).unwrap();

    let mut saved = Vec::new();
    let count = accessor
        .save_all_players_with(|player| {
            saved.push(player.object().guid());
            Ok::<(), ()>(())
        })
        .unwrap();

    assert_eq!(count, 2);
    assert!(saved.contains(&guid_a));
    assert!(saved.contains(&guid_b));
    assert_eq!(saved.len(), 2);
}

#[test]
fn save_all_players_with_propagates_error_with_player_guid() {
    let mut accessor = ObjectAccessor::default();
    let player = world_object(HighGuid::Player, 1, 0, true);
    let guid = player.guid();
    accessor.add_player("jaina", player).unwrap();

    let error = accessor
        .save_all_players_with(|_| Err::<(), _>("db unavailable"))
        .unwrap_err();

    assert_eq!(error.guid, guid);
    assert_eq!(error.source, "db unavailable");
}

#[test]
fn save_all_players_with_does_not_break_name_or_canonical_map_source_lookup() {
    let mut accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let player_guid = context.guid();
    let creature = world_object(HighGuid::Creature, 530, 1, true);
    let creature_guid = creature.guid();
    let record = MapObjectRecord::new(AccessorObjectKind::Creature, creature).unwrap();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, record);

    accessor.add_player("valeera", context.clone()).unwrap();

    let saved = accessor
        .save_all_players_with(|_| Ok::<(), ()>(()))
        .unwrap();

    assert_eq!(saved, 1);
    assert_eq!(
        accessor
            .find_connected_player_by_name("VALEERA")
            .unwrap()
            .guid(),
        player_guid
    );
    assert_eq!(
        accessor
            .get_creature_from_map_source(&context, &source, creature_guid)
            .unwrap()
            .guid(),
        creature_guid
    );
}

#[test]
fn typed_map_source_lookup_preserves_creature_and_gameobject_bodies_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let linked_trap = guid(HighGuid::GameObject, 4101);
    let (creature_guid, creature_record) = typed_creature_record(530, 1, 4102, 61);
    let (game_object_guid, game_object_record) =
        typed_game_object_record(530, 1, 4103, linked_trap);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, creature_record);
    source.records.insert(game_object_guid, game_object_record);

    let creature = accessor
        .get_typed_creature_from_map_source(&context, &source, creature_guid)
        .unwrap();
    assert_eq!(creature.level(), 61);
    assert_eq!(creature.unit().world().guid(), creature_guid);

    let game_object = accessor
        .get_typed_game_object_from_map_source(&context, &source, game_object_guid)
        .unwrap();
    assert_eq!(game_object.linked_trap_guid_like_cpp(), linked_trap);
    assert_eq!(game_object.world().guid(), game_object_guid);
}

#[test]
fn typed_map_source_lookup_rejects_generic_worldobject_fallback_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let generic_creature = world_object(HighGuid::Creature, 530, 1, true);
    let generic_creature_guid = generic_creature.guid();
    let generic_game_object = world_object(HighGuid::GameObject, 530, 1, true);
    let generic_game_object_guid = generic_game_object.guid();
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(
        generic_creature_guid,
        MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
    );
    source.records.insert(
        generic_game_object_guid,
        MapObjectRecord::new(AccessorObjectKind::GameObject, generic_game_object).unwrap(),
    );

    assert_eq!(
        accessor
            .get_creature_from_map_source(&context, &source, generic_creature_guid)
            .unwrap()
            .guid(),
        generic_creature_guid
    );
    assert_eq!(
        accessor
            .get_game_object_from_map_source(&context, &source, generic_game_object_guid)
            .unwrap()
            .guid(),
        generic_game_object_guid
    );
    assert!(
        accessor
            .get_typed_creature_from_map_source(&context, &source, generic_creature_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_game_object_from_map_source(&context, &source, generic_game_object_guid)
            .is_none()
    );
}

#[test]
fn typed_map_source_lookup_requires_source_and_context_same_map_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let (creature_guid, creature_record) = typed_creature_record(530, 1, 4104, 62);
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, creature_record);

    source.map_id = 571;
    assert!(
        accessor
            .get_typed_creature_from_map_source(&context, &source, creature_guid)
            .is_none()
    );
    source.map_id = 530;
    source.instance_id = 2;
    assert!(
        accessor
            .get_typed_creature_from_map_source(&context, &source, creature_guid)
            .is_none()
    );
    source.instance_id = 1;

    let wrong_context = world_object(HighGuid::Player, 571, 1, true);
    assert!(
        accessor
            .get_typed_creature_from_map_source(&wrong_context, &source, creature_guid)
            .is_none()
    );
}

#[test]
fn typed_map_source_lookup_does_not_cross_creature_and_gameobject_kinds_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let (creature_guid, creature_record) = typed_creature_record(530, 1, 4105, 63);
    let (game_object_guid, game_object_record) =
        typed_game_object_record(530, 1, 4106, guid(HighGuid::GameObject, 4107));
    let mut source = TestMapSource {
        map_id: 530,
        instance_id: 1,
        records: std::collections::HashMap::new(),
    };
    source.records.insert(creature_guid, creature_record);
    source.records.insert(game_object_guid, game_object_record);

    assert!(
        accessor
            .get_typed_creature_from_map_source(&context, &source, game_object_guid)
            .is_none()
    );
    assert!(
        accessor
            .get_typed_game_object_from_map_source(&context, &source, creature_guid)
            .is_none()
    );
}

#[test]
fn typed_map_source_lookup_preserves_non_creature_gameobject_bodies_like_cpp() {
    let accessor = ObjectAccessor::default();
    let context = world_object(HighGuid::Player, 530, 1, true);
    let caster = guid(HighGuid::Creature, 4110);
    let creator = guid(HighGuid::Player, 4111);
    let (dynamic_guid, dynamic_record) = typed_dynamic_object_record(530, 1, 4112, caster, 12345);
    let (area_trigger_guid, area_trigger_record) =
        typed_area_trigger_record(530, 1, 4113, caster, 12346);
    let (corpse_guid, corpse_record) = typed_corpse_record(530, 1, 4114, creator, 98765);
    let (scene_guid, scene_record) = typed_scene_object_record(530, 1, 4115, creator, 4567);
    let (conversation_guid, conversation_record) =
        typed_conversation_record(530, 1, 4116, creator, 8901);
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

    let dynamic_object = accessor
        .get_typed_dynamic_object_from_map_source(&context, &source, dynamic_guid)
        .unwrap();
    assert_eq!(dynamic_object.caster_guid(), caster);
    assert_eq!(dynamic_object.spell_id(), 12345);
    assert_eq!(dynamic_object.radius(), 12.5);
    assert_eq!(dynamic_object.world().guid(), dynamic_guid);

    let area_trigger = accessor
        .get_typed_area_trigger_from_map_source(&context, &source, area_trigger_guid)
        .unwrap();
    assert_eq!(area_trigger.caster_guid(), caster);
    assert_eq!(area_trigger.spell_id(), 12346);
    assert_eq!(area_trigger.duration_ms(), 4_500);
    assert_eq!(area_trigger.world().guid(), area_trigger_guid);

    let corpse = accessor
        .get_typed_corpse_from_map_source(&context, &source, corpse_guid)
        .unwrap();
    assert_eq!(corpse.corpse_type(), crate::CorpseType::ResurrectablePve);
    assert_eq!(corpse.ghost_time(), 98765);
    assert_eq!(corpse.data().owner, creator);
    assert_eq!(corpse.data().display_id, 11_111);
    assert_eq!(corpse.world().guid(), corpse_guid);

    let scene_object = accessor
        .get_typed_scene_object_from_map_source(&context, &source, scene_guid)
        .unwrap();
    assert_eq!(scene_object.creator_guid(), creator);
    assert_eq!(scene_object.data().script_package_id, 4567);
    assert_eq!(
        scene_object.data().scene_type,
        crate::SceneType::PetBattle as u32
    );
    assert_eq!(scene_object.world().guid(), scene_guid);

    let conversation = accessor
        .get_typed_conversation_from_map_source(&context, &source, conversation_guid)
        .unwrap();
    assert_eq!(conversation.creator_guid(), creator);
    assert_eq!(conversation.duration_ms(), 8901);
    assert_eq!(conversation.texture_kit_id(), 77);
    assert_eq!(conversation.data().lines[0].conversation_line_id, 9901);
    assert_eq!(conversation.world().guid(), conversation_guid);
}
