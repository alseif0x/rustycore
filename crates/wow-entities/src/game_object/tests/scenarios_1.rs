//! GameObject template, loot and runtime state regression scenarios, part 1 of 3.
//!
//! Moved out of the game_object.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn gameobject_constructor_matches_cpp_base_state() {
    let go = GameObject::new();

    assert_eq!(go.world().object().type_id(), TypeId::GameObject);
    assert_eq!(
        go.world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::GAME_OBJECT
    );
    assert!(!go.world().is_world_object());
    assert!(
        go.world()
            .object()
            .create_flags()
            .contains(CreateObjectFlags::STATIONARY)
    );
    assert!(
        go.world()
            .object()
            .create_flags()
            .contains(CreateObjectFlags::ROTATION)
    );
    assert_eq!(go.respawn_time(), 0);
    assert_eq!(
        go.respawn_delay_time(),
        DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS
    );
    assert_eq!(go.despawn_delay(), 0);
    assert_eq!(go.despawn_respawn_time(), 0);
    assert_eq!(go.restock_time(), 0);
    assert_eq!(go.loot_state(), LootState::NotReady);
    assert_eq!(go.loot_state_unit_guid(), ObjectGuid::EMPTY);
    assert!(go.spawned_by_default());
    assert_eq!(go.use_times(), 0);
    assert_eq!(go.spell_id(), 0);
    assert_eq!(go.cooldown_time(), 0);
    assert_eq!(go.prev_go_state(), GoState::Active);
    assert_eq!(go.packed_rotation(), 0);
    assert_eq!(go.spawn_id(), 0);
    assert_eq!(go.loot_mode(), GAMEOBJECT_LOOT_MODE_DEFAULT);
    assert!(!go.respawn_compatibility_mode());
    assert_eq!(go.anim_kit_id(), 0);
    assert_eq!(go.world_effect_id(), 0);
    assert_eq!(go.stationary_position(), Position::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(go.cleanup_before_delete_count(), 0);
    assert!(!go.grid_unload_delete_requested());
    assert!(!go.grid_unload_respawn_relocation_requested());
    assert!(!go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
    assert!(!go.has_represented_gameobject_data_like_cpp());
    assert!(!go.game_object_data_changes_mask().is_any_set());
}

#[test]
fn gameobject_data_presence_evidence_defaults_false_and_setter_round_trips_like_cpp() {
    let mut go = GameObject::new();

    assert!(!go.has_represented_gameobject_data_like_cpp());
    go.set_represented_gameobject_data_present_like_cpp(true);
    assert!(go.has_represented_gameobject_data_like_cpp());
    go.set_represented_gameobject_data_present_like_cpp(false);
    assert!(!go.has_represented_gameobject_data_like_cpp());
}

#[test]
fn gameobject_model_existence_evidence_defaults_false_and_setter_round_trips_like_cpp() {
    let mut go = GameObject::new();

    assert!(!go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
    go.set_represented_gameobject_model_like_cpp(true);
    assert!(go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
    go.set_represented_gameobject_model_like_cpp(false);
    assert!(!go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}

#[test]
fn gameobject_model_map_object_flag_requires_explicit_model_evidence_like_cpp() {
    let mut go = GameObject::new();

    go.apply_represented_gameobject_model_creation_like_cpp(false, true);
    assert!(!go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, 0);

    go.apply_represented_gameobject_model_creation_like_cpp(true, true);
    assert!(go.has_represented_gameobject_model_like_cpp());
    assert!(go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, GO_FLAG_MAP_OBJECT);

    go.apply_represented_gameobject_model_creation_like_cpp(true, false);
    assert!(go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, 0);
}

#[test]
fn gameobject_model_disable_clears_map_object_flag_and_collision_evidence_like_cpp() {
    let mut go = GameObject::new();
    go.apply_represented_gameobject_model_creation_like_cpp(true, true);
    let enabled = go.enable_represented_gameobject_collision_like_cpp(true);
    assert_eq!(enabled.new_collision_enabled, Some(true));
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, GO_FLAG_MAP_OBJECT);

    go.set_represented_gameobject_model_like_cpp(false);

    assert!(!go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, 0);
}

#[test]
fn gameobject_model_recreation_clears_previous_collision_evidence_like_cpp() {
    let mut go = GameObject::new();
    go.apply_represented_gameobject_model_creation_like_cpp(true, true);
    let enabled = go.enable_represented_gameobject_collision_like_cpp(true);
    assert_eq!(enabled.new_collision_enabled, Some(true));
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, GO_FLAG_MAP_OBJECT);

    go.apply_represented_gameobject_model_creation_like_cpp(true, false);

    assert!(go.has_represented_gameobject_model_like_cpp());
    assert!(!go.has_represented_gameobject_model_map_object_like_cpp());
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
    assert_eq!(go.data().flags & GO_FLAG_MAP_OBJECT, 0);
}

#[test]
fn gameobject_model_collision_no_model_is_noop_like_cpp() {
    let mut go = GameObject::new();

    let outcome = go.enable_represented_gameobject_collision_like_cpp(true);

    assert_eq!(
        outcome,
        GameObjectCollisionOutcomeLikeCpp {
            requested_enable: true,
            represented_model_present: false,
            previous_collision_enabled: None,
            new_collision_enabled: None,
        }
    );
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        None
    );
}

#[test]
fn gameobject_model_collision_with_model_stores_true_and_false_like_cpp() {
    let mut go = GameObject::new();
    go.set_represented_gameobject_model_like_cpp(true);

    let enabled = go.enable_represented_gameobject_collision_like_cpp(true);
    assert_eq!(enabled.previous_collision_enabled, None);
    assert_eq!(enabled.new_collision_enabled, Some(true));
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(true)
    );

    let disabled = go.enable_represented_gameobject_collision_like_cpp(false);
    assert_eq!(disabled.previous_collision_enabled, Some(true));
    assert_eq!(disabled.new_collision_enabled, Some(false));
    assert_eq!(
        go.represented_gameobject_model_collision_enabled_like_cpp(),
        Some(false)
    );
}

fn lifecycle_template() -> GameObjectTemplateLifecycleRecord {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 9001;
    GameObjectTemplateLifecycleRecord {
        entry: 17_000,
        name: "C++ anchored chest".to_string(),
        go_type: GAMEOBJECT_TYPE_CHEST,
        display_id: 400,
        scale: 1.75,
        faction: 35,
        flags: GO_FLAG_IN_USE | GO_FLAG_IN_MULTI_USE,
        data,
        world_effect_id: 77,
        anim_kit_id: 12,
        level: 80,
        percent_health: 100,
        custom_param: 44,
    }
}

fn lifecycle_create(dynamic: bool) -> GameObjectCreateLifecycleRecord {
    GameObjectCreateLifecycleRecord {
        guid: ObjectGuid::new(8, 17_000),
        map_id: 571,
        instance_id: 3,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        rotation: [0.125, 0.25, 0.375, 0.875],
        anim_progress: 33,
        go_state: GoState::Ready,
        art_kit: 6,
        dynamic,
        spawn_id: 98_765,
        template: lifecycle_template(),
    }
}

#[test]
fn gameobject_create_from_lifecycle_applies_cpp_create_state() {
    let record = lifecycle_create(false);
    let position = record.position;
    let guid = record.guid;
    let go = GameObject::try_create_from_lifecycle(record).expect("valid lifecycle record");

    assert_eq!(go.world().guid(), guid);
    assert_eq!(go.world().map_id(), 571);
    assert_eq!(go.world().instance_id(), 3);
    assert_eq!(go.world().position(), position);
    assert_eq!(go.stationary_position(), position);
    assert_eq!(go.local_rotation_like_cpp(), [0.125, 0.25, 0.375, 0.875]);
    assert_eq!(go.world().object().entry(), 17_000);
    assert_eq!(go.world().object().scale(), 1.75);
    assert_eq!(go.world().name(), "C++ anchored chest");
    assert_eq!(go.data().display_id, 400);
    assert_eq!(go.data().faction_template, 35);
    assert_eq!(go.data().flags, GO_FLAG_IN_USE | GO_FLAG_IN_MULTI_USE);
    assert_eq!(go.data().type_id, GAMEOBJECT_TYPE_CHEST as i8);
    assert_eq!(go.data().state, GoState::Ready as i8);
    assert_eq!(go.prev_go_state(), GoState::Ready);
    assert_eq!(go.data().art_kit, 6);
    assert_eq!(go.data().level, 80);
    assert_eq!(go.data().percent_health, 100);
    assert_eq!(go.data().custom_param, 44);
    assert_eq!(go.anim_kit_id(), 12);
    assert_eq!(go.world_effect_id(), 77);
    assert_eq!(go.spawn_id(), 98_765);
    assert_ne!(go.packed_rotation(), 0);
    assert!(go.respawn_compatibility_mode());
}

#[test]
fn gameobject_create_from_lifecycle_carries_spell_focus_source_like_cpp() {
    let mut record = lifecycle_create(true);
    record.template.go_type = GAMEOBJECT_TYPE_SPELL_FOCUS;
    record.template.data = [0; MAX_GAMEOBJECT_DATA];
    record.template.data[GAMEOBJECT_DATA_SPELL_FOCUS_TYPE] = 181;
    record.template.data[GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS] = 10;
    record.template.data[GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP] = 987;

    let go = GameObject::try_create_from_lifecycle(record).expect("valid focus lifecycle");

    assert_eq!(
        go.represented_spell_focus_use_source_like_cpp(),
        Some(SpellFocusUseSource {
            focus_type: 181,
            radius: 10,
            linked_trap_entry: 987,
        })
    );
}

#[test]
fn gameobject_try_create_from_lifecycle_rejects_invalid_go_type_like_cpp() {
    let mut record = lifecycle_create(true);
    record.template.go_type = MAX_GAMEOBJECT_TYPE;

    assert_eq!(
        GameObject::try_create_from_lifecycle(record),
        Err(GameObjectLifecycleError::InvalidGameObjectType {
            entry: 17_000,
            go_type: MAX_GAMEOBJECT_TYPE,
        })
    );
}

#[test]
fn gameobject_try_create_from_lifecycle_rejects_map_obj_transport_like_cpp() {
    let mut record = lifecycle_create(true);
    record.template.go_type = GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT;

    assert_eq!(
        GameObject::try_create_from_lifecycle(record),
        Err(GameObjectLifecycleError::InvalidMapObjectTransportType { entry: 17_000 })
    );
}

#[test]
fn gameobject_try_create_from_lifecycle_rejects_invalid_position_without_partial_state() {
    let mut record = lifecycle_create(true);
    record.position = Position::new(f32::INFINITY, 2.0, 3.0, 4.0);
    let mut go = GameObject::new();

    assert_eq!(
        go.apply_create_lifecycle(record.clone()),
        Err(GameObjectLifecycleError::InvalidPosition {
            entry: 17_000,
            position: record.position,
        })
    );
    assert_eq!(go.world().guid(), ObjectGuid::EMPTY);
    assert!(!go.world().has_current_map());
    assert_eq!(go.world().position(), Position::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn gameobject_apply_create_lifecycle_propagates_map_binding_error() {
    let mut record = lifecycle_create(true);
    record.map_id = 571;
    record.instance_id = 3;
    let mut go = GameObject::new();
    go.world_mut().set_map(1, 2).expect("initial binding");

    assert_eq!(
        go.apply_create_lifecycle(record),
        Err(GameObjectLifecycleError::MapBinding {
            entry: 17_000,
            source: MapBindingError::AlreadyBound {
                old_map_id: 1,
                old_instance_id: 2,
                new_map_id: 571,
                new_instance_id: 3,
            },
        })
    );
}

#[test]
fn gameobject_load_from_db_lifecycle_applies_respawn_state_like_cpp() {
    let go = GameObject::try_load_from_db_lifecycle(GameObjectLoadFromDbLifecycleRecord {
        create: lifecycle_create(true),
        spawntimesecs: 300,
        effective_map_respawn_time: 123_456,
        despawn_possible: true,
        despawn_at_action: false,
        respawn_compatibility_mode: true,
        string_id: "db-string-id".to_string(),
    })
    .expect("valid lifecycle record");

    assert!(go.spawned_by_default());
    assert_eq!(go.respawn_delay_time(), 300);
    assert_eq!(go.respawn_time(), 123_456);
    assert!(go.respawn_compatibility_mode());
    assert_eq!(go.lifecycle_string_id(), "db-string-id");
}

#[test]
fn gameobject_load_from_db_lifecycle_handles_negative_spawntime_state() {
    let go = GameObject::try_load_from_db_lifecycle(GameObjectLoadFromDbLifecycleRecord {
        create: lifecycle_create(true),
        spawntimesecs: -45,
        effective_map_respawn_time: 123_456,
        despawn_possible: true,
        despawn_at_action: false,
        respawn_compatibility_mode: false,
        string_id: String::new(),
    })
    .expect("valid lifecycle record");

    assert!(!go.spawned_by_default());
    assert_eq!(go.respawn_delay_time(), 45);
    assert_eq!(go.respawn_time(), 0);
    assert!(go.respawn_compatibility_mode());
}

#[test]
fn gameobject_load_from_db_lifecycle_zeroes_respawn_for_nodespawn_like_cpp() {
    let go = GameObject::try_load_from_db_lifecycle(GameObjectLoadFromDbLifecycleRecord {
        create: lifecycle_create(true),
        spawntimesecs: 300,
        effective_map_respawn_time: 123_456,
        despawn_possible: false,
        despawn_at_action: false,
        respawn_compatibility_mode: false,
        string_id: String::new(),
    })
    .expect("valid lifecycle record");

    assert!(go.spawned_by_default());
    assert_eq!(go.respawn_delay_time(), 0);
    assert_eq!(go.respawn_time(), 0);
    assert_eq!(
        go.data().flags,
        GO_FLAG_IN_USE | GO_FLAG_IN_MULTI_USE | GO_FLAG_NODESPAWN
    );
    assert!(!go.respawn_compatibility_mode());
}

#[test]
fn gameobject_load_from_db_lifecycle_preserves_prenormalized_zero_respawn_time() {
    let go = GameObject::try_load_from_db_lifecycle(GameObjectLoadFromDbLifecycleRecord {
        create: lifecycle_create(true),
        spawntimesecs: 300,
        effective_map_respawn_time: 0,
        despawn_possible: true,
        despawn_at_action: false,
        respawn_compatibility_mode: false,
        string_id: String::new(),
    })
    .expect("valid lifecycle record");

    assert!(go.spawned_by_default());
    assert_eq!(go.respawn_delay_time(), 300);
    assert_eq!(go.respawn_time(), 0);
    assert!(!go.respawn_compatibility_mode());
}

#[test]
fn gameobject_template_get_loot_id_matches_cpp_switch() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 44;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FISHING_HOLE, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(2, data).get_loot_id_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_is_despawn_at_action_like_cpp_matches_switch() {
    let mut chest_data = [0; MAX_GAMEOBJECT_DATA];
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest_data)
            .is_despawn_at_action_like_cpp()
    );
    chest_data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest_data)
            .is_despawn_at_action_like_cpp()
    );

    let mut goober_data = [0; MAX_GAMEOBJECT_DATA];
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober_data)
            .is_despawn_at_action_like_cpp()
    );
    goober_data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] = 1;
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober_data)
            .is_despawn_at_action_like_cpp()
    );

    let mut generic_data = [0; MAX_GAMEOBJECT_DATA];
    generic_data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    generic_data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] = 1;
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_GENERIC, generic_data)
            .is_despawn_at_action_like_cpp()
    );
}

#[test]
fn gameobject_template_condition_id1_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 7),
        (GAMEOBJECT_TYPE_BUTTON, 9),
        (GAMEOBJECT_TYPE_QUESTGIVER, 10),
        (GAMEOBJECT_TYPE_CHEST, 17),
        (GAMEOBJECT_TYPE_GENERIC, 6),
        (GAMEOBJECT_TYPE_TRAP, 15),
        (GAMEOBJECT_TYPE_CHAIR, 4),
        (GAMEOBJECT_TYPE_SPELL_FOCUS, 8),
        (GAMEOBJECT_TYPE_TEXT, 4),
        (GAMEOBJECT_TYPE_GOOBER, 22),
        (GAMEOBJECT_TYPE_CAMERA, 4),
        (GAMEOBJECT_TYPE_RITUAL, 8),
        (GAMEOBJECT_TYPE_MAILBOX, 0),
        (GAMEOBJECT_TYPE_SPELLCASTER, 5),
        (GAMEOBJECT_TYPE_FLAGSTAND, 8),
        (GAMEOBJECT_TYPE_AURA_GENERATOR, 3),
        (GAMEOBJECT_TYPE_GUILD_BANK, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG, 4),
        (GAMEOBJECT_TYPE_ITEM_FORGE, 0),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 11),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 7_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_condition_id1_like_cpp(),
            7_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(25, data).get_condition_id1_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_interact_radius_override_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 12),
        (GAMEOBJECT_TYPE_BUTTON, 10),
        (GAMEOBJECT_TYPE_QUESTGIVER, 12),
        (GAMEOBJECT_TYPE_CHEST, 9),
        (GAMEOBJECT_TYPE_BINDER, 0),
        (GAMEOBJECT_TYPE_GENERIC, 9),
        (GAMEOBJECT_TYPE_TRAP, 21),
        (GAMEOBJECT_TYPE_CHAIR, 5),
        (GAMEOBJECT_TYPE_SPELL_FOCUS, 9),
        (GAMEOBJECT_TYPE_TEXT, 6),
        (GAMEOBJECT_TYPE_GOOBER, 33),
        (GAMEOBJECT_TYPE_AREADAMAGE, 8),
        (GAMEOBJECT_TYPE_CAMERA, 5),
        (GAMEOBJECT_TYPE_FISHING_NODE, 0),
        (GAMEOBJECT_TYPE_RITUAL, 9),
        (GAMEOBJECT_TYPE_MAILBOX, 1),
        (GAMEOBJECT_TYPE_SPELLCASTER, 8),
        (GAMEOBJECT_TYPE_MEETINGSTONE, 3),
        (GAMEOBJECT_TYPE_FLAGSTAND, 13),
        (GAMEOBJECT_TYPE_FISHING_HOLE, 5),
        (GAMEOBJECT_TYPE_FLAGDROP, 10),
        (GAMEOBJECT_TYPE_AURA_GENERATOR, 7),
        (GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, 11),
        (GAMEOBJECT_TYPE_BARBER_CHAIR, 3),
        (GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING, 27),
        (GAMEOBJECT_TYPE_GUILD_BANK, 1),
        (GAMEOBJECT_TYPE_NEW_FLAG, 14),
        (GAMEOBJECT_TYPE_NEW_FLAG_DROP, 2),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 24),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 10_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_interact_radius_override_like_cpp(),
            10_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAP_OBJECT, data)
            .get_interact_radius_override_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_lock_id_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 1),
        (GAMEOBJECT_TYPE_BUTTON, 1),
        (GAMEOBJECT_TYPE_QUESTGIVER, 0),
        (GAMEOBJECT_TYPE_CHEST, 0),
        (GAMEOBJECT_TYPE_TRAP, 0),
        (GAMEOBJECT_TYPE_GOOBER, 0),
        (GAMEOBJECT_TYPE_AREADAMAGE, 0),
        (GAMEOBJECT_TYPE_CAMERA, 0),
        (GAMEOBJECT_TYPE_FLAGSTAND, 0),
        (GAMEOBJECT_TYPE_FISHING_HOLE, 4),
        (GAMEOBJECT_TYPE_FLAGDROP, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG_DROP, 0),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 3),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 20_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_lock_id_like_cpp(),
            20_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAP_OBJECT, data).get_lock_id_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_usable_mounted_matches_cpp_switch() {
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAILBOX, [0; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_BARBER_CHAIR, [1; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );

    let cases = [
        (GAMEOBJECT_TYPE_QUESTGIVER, 8),
        (GAMEOBJECT_TYPE_TEXT, 3),
        (GAMEOBJECT_TYPE_GOOBER, 17),
        (GAMEOBJECT_TYPE_SPELLCASTER, 3),
        (GAMEOBJECT_TYPE_UI_LINK, 1),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        assert!(
            !GameObjectTemplateData::new(go_type, data).is_usable_mounted_like_cpp(),
            "type {go_type} should default to not usable mounted"
        );
        data[index] = 1;
        assert!(
            GameObjectTemplateData::new(go_type, data).is_usable_mounted_like_cpp(),
            "type {go_type} should read allowMounted from data[{index}]"
        );
    }

    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [1; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );
}

#[test]
fn gameobject_template_no_damage_immune_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 3),
        (GAMEOBJECT_TYPE_BUTTON, 4),
        (GAMEOBJECT_TYPE_QUESTGIVER, 5),
        (GAMEOBJECT_TYPE_GOOBER, 11),
        (GAMEOBJECT_TYPE_FLAGSTAND, 5),
        (GAMEOBJECT_TYPE_FLAGDROP, 3),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_no_damage_immune_like_cpp(),
            0
        );
        data[index] = 7;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_no_damage_immune_like_cpp(),
            7
        );
    }

    let mut chest = [0; MAX_GAMEOBJECT_DATA];
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest).get_no_damage_immune_like_cpp(),
        1
    );
    chest[22] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest).get_no_damage_immune_like_cpp(),
        0
    );

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TEXT, [1; MAX_GAMEOBJECT_DATA])
            .get_no_damage_immune_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_cooldown_matches_cpp_switch() {
    let mut trap = [0; MAX_GAMEOBJECT_DATA];
    trap[5] = 12;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, trap).get_cooldown_like_cpp(),
        12
    );

    let mut goober = [0; MAX_GAMEOBJECT_DATA];
    goober[6] = 34;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober).get_cooldown_like_cpp(),
        34
    );

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [99; MAX_GAMEOBJECT_DATA])
            .get_cooldown_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_auto_close_time_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 2, 11),
        (GAMEOBJECT_TYPE_BUTTON, 2, 22),
        (GAMEOBJECT_TYPE_TRAP, 6, 33),
        (GAMEOBJECT_TYPE_GOOBER, 3, 44),
    ];

    for (go_type, index, value) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = value;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_auto_close_time_like_cpp(),
            value
        );
    }

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [99; MAX_GAMEOBJECT_DATA])
            .get_auto_close_time_like_cpp(),
        0
    );
}

#[test]
fn trap_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 20;
    data[3] = 123;
    data[4] = 1;
    data[5] = 9;
    data[7] = 3;
    data[14] = 1;
    data[20] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, data).trap_use_source_like_cpp(),
        Some(TrapUseSource {
            radius: 20,
            spell_id: 123,
            charges: 1,
            cooldown_secs: 9,
            start_delay_secs: 3,
            ignore_totems: true,
            check_all_units: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).trap_use_source_like_cpp(),
        None
    );
}

#[test]
fn chair_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 3;
    data[1] = 2;
    data[3] = 77;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHAIR, data).chair_use_source_like_cpp(),
        Some(ChairUseSource {
            chair_slots: 3,
            chair_height: 2,
            triggered_event_id: 77,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).chair_use_source_like_cpp(),
        None
    );
}

#[test]
fn barber_chair_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 2;
    data[2] = 345;
    data[4] = 9;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_BARBER_CHAIR, data)
            .barber_chair_use_source_like_cpp(),
        Some(BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 345,
            customization_scope: 9,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHAIR, data).barber_chair_use_source_like_cpp(),
        None
    );
}

#[test]
fn ui_link_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 3;
    data[6] = 99;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_UI_LINK, data).ui_link_use_source_like_cpp(),
        Some(UiLinkUseSource { ui_link_type: 3 })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).ui_link_use_source_like_cpp(),
        None
    );
}

#[test]
fn item_forge_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 77;
    data[5] = 4;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_ITEM_FORGE, data)
            .item_forge_use_source_like_cpp(),
        Some(ItemForgeUseSource {
            condition_id: 77,
            forge_type: 4,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).item_forge_use_source_like_cpp(),
        None
    );
}
