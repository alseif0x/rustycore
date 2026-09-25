//! Visibility, farsight, and dynamic-object fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn test_dynamic_object_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::DynamicObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

pub(in crate::session::tests) fn test_area_trigger_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::AreaTrigger,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

pub(in crate::session::tests) fn prepare_dynamic_object_values_snapshot_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    dynamic_object_guid: ObjectGuid,
    radius: f32,
) {
    {
        let mut guard = canonical.lock().unwrap();
        let managed = guard.create_world_map(map_id, instance_id);
        let record = managed
            .map_mut()
            .get_typed_dynamic_object_mut(dynamic_object_guid)
            .unwrap();
        record.set_radius(radius);
    }
    let mut guard = canonical.lock().unwrap();
    let _ = guard.update(1);
    assert_eq!(
        guard
            .find_map(map_id, instance_id)
            .unwrap()
            .last_send_object_updates_summary_like_cpp()
            .dynamic_object_values_updates
            .len(),
        1
    );
    assert!(
        !guard
            .find_map(map_id, instance_id)
            .unwrap()
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );
}

pub(in crate::session::tests) fn configure_dynamic_object_values_snapshot_session_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DynamicObjectViewer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        map_id as u16,
        1,
        1,
        80,
        0,
    ));
    session.set_state(SessionState::LoggedIn);
    add_canonical_test_player_on_map(
        canonical,
        player_guid,
        Position::new(10.0, 20.0, 30.0, 0.0),
        map_id,
        instance_id,
    );
}

pub(in crate::session::tests) fn add_shared_vision_viewer_to_canonical_target_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    target_guid: ObjectGuid,
    viewer_guid: ObjectGuid,
) {
    let mut guard = canonical.lock().unwrap();
    let map = guard.find_map_mut(map_id, instance_id).unwrap().map_mut();
    if let Some(target) = map.get_typed_player_mut(target_guid) {
        target
            .unit_mut()
            .subsystems_mut()
            .control
            .add_shared_vision(viewer_guid);
    } else {
        map.get_typed_creature_mut(target_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .add_shared_vision(viewer_guid);
    }
}

pub(in crate::session::tests) fn add_canonical_test_dynamic_object_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    caster: ObjectGuid,
    spell_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut dynamic_object = wow_entities::DynamicObject::new(true);
    dynamic_object.world_mut().object_mut().create(guid);
    dynamic_object.world_mut().object_mut().set_entry(spell_id);
    dynamic_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    dynamic_object.world_mut().relocate(position);
    dynamic_object.set_caster_guid(caster);
    dynamic_object.set_dynamic_object_type(wow_entities::DynamicObjectType::FarsightFocus);
    dynamic_object.set_spell_visual_id(700);
    dynamic_object.set_spell_id(spell_id as i32);
    dynamic_object.set_radius(25.0);
    dynamic_object.set_cast_time_ms(1500);
    dynamic_object.set_duration(5000);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map.map_mut().add_to_map_like_cpp(
        AccessorObjectKind::DynamicObject,
        dynamic_object.world().clone(),
    );
    dynamic_object.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_visibility_misc_objects_on_map(
    canonical: &SharedCanonicalMapManager,
    corpse_guid: ObjectGuid,
    scene_object_guid: ObjectGuid,
    conversation_guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().object_mut().set_entry(501);
    corpse.world_mut().set_map(map_id, instance_id).unwrap();
    corpse.world_mut().relocate(position);
    corpse.set_display_id(7001);
    corpse.set_race(1);
    corpse.set_class(1);
    corpse.set_owner_guid(ObjectGuid::create_player(1, 501));

    let mut scene_object = wow_entities::SceneObject::new();
    scene_object
        .world_mut()
        .object_mut()
        .create(scene_object_guid);
    scene_object.world_mut().object_mut().set_entry(502);
    scene_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    scene_object.world_mut().relocate(position);
    scene_object.relocate_stationary_position(position);
    scene_object.set_script_package_id(502);
    scene_object.set_rnd_seed_val(1234);

    let mut conversation = wow_entities::Conversation::new();
    conversation
        .world_mut()
        .object_mut()
        .create(conversation_guid);
    conversation.world_mut().object_mut().set_entry(503);
    conversation
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    conversation.world_mut().relocate(position);
    conversation.relocate_stationary_position(position);
    conversation.set_duration_ms(10_000);
    conversation.set_texture_kit_id(77);
    conversation.add_line(wow_entities::ConversationLine {
        conversation_line_id: 9,
        start_time: 10,
        ui_camera_id: 0,
        actor_index: 0,
        flags: 0,
    });

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    for (kind, world) in [
        (AccessorObjectKind::Corpse, corpse.world().clone()),
        (
            AccessorObjectKind::SceneObject,
            scene_object.world().clone(),
        ),
        (
            AccessorObjectKind::Conversation,
            conversation.world().clone(),
        ),
    ] {
        map.map_mut().add_to_map_like_cpp(kind, world).unwrap();
    }
    corpse.world_mut().object_mut().add_to_world();
    scene_object.world_mut().object_mut().add_to_world();
    conversation.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_corpse(corpse).unwrap())
        .unwrap();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_scene_object(scene_object).unwrap(),
        )
        .unwrap();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_conversation(conversation).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_area_trigger_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    caster: ObjectGuid,
    spell_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut area_trigger = wow_entities::AreaTrigger::new();
    area_trigger.world_mut().object_mut().create(guid);
    area_trigger.world_mut().object_mut().set_entry(spell_id);
    area_trigger
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    area_trigger.world_mut().relocate(position);
    area_trigger.set_caster_guid(caster);
    area_trigger.set_spell_id(spell_id as i32);
    area_trigger.set_duration(5000);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map.map_mut().add_to_map_like_cpp(
        AccessorObjectKind::AreaTrigger,
        area_trigger.world().clone(),
    );
    area_trigger.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn configure_test_dynamic_object_for_visual_despawn_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    duration_ms: i32,
    bound_caster: Option<ObjectGuid>,
    phase_shift: Option<PhaseShift>,
) {
    let mut guard = canonical.lock().unwrap();
    let dynamic_object = guard
        .find_map_mut(map_id, instance_id)
        .unwrap()
        .map_mut()
        .get_typed_dynamic_object_mut(guid)
        .unwrap();
    dynamic_object.set_duration(duration_ms);
    if let Some(bound_caster) = bound_caster {
        dynamic_object.bind_to_caster(bound_caster);
    }
    if let Some(phase_shift) = phase_shift {
        *dynamic_object.world_mut().phase_shift_mut() = phase_shift;
    }
}

pub(in crate::session::tests) fn configure_add_farsight_live_session_like_cpp(
    session: &mut WorldSession,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    player_guid: ObjectGuid,
    spell_id: i32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Farseer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 1500,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT,
                effect_radius_index_1: 11,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: 1,
            attributes: [0; 15],
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 7,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: spell_id as u32,
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: -5000,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));
    session.set_spell_radius_store(Arc::new(wow_data::SpellRadiusStore::from_entries([
        wow_data::SpellRadiusEntry {
            id: 11,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: 25.0,
        },
    ])));
}

pub(in crate::session::tests) fn add_farsight_target_data(
    destination: Option<Position>,
) -> SpellTargetData {
    SpellTargetData {
        flags: if destination.is_some() { 0x40 } else { 0 },
        dst_location: destination.map(|position| wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position,
        }),
        ..Default::default()
    }
}
